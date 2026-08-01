use super::*;

#[test]
fn parse_cdxml_hash_bond_keeps_an_explicit_hash_pattern_and_round_trips_it() {
    let cdxml = r#"<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page><fragment>
    <n id="n1" p="0 0"/><n id="n2" p="30 0"/>
    <b id="b1" B="n1" E="n2" Display="Hash"/>
  </fragment></page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("explicit hash bond")).expect("parses");
    let bond = &document
        .editable_fragment()
        .expect("fragment")
        .fragment
        .bonds[0];
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Hash
    );
    assert_eq!(
        bond.line_weights.main,
        chemsema_engine::BondLineWeight::Normal,
        "Hash is a line pattern, not a disguised bold dashed line"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Display=\"Hash\""), "{exported}");
    assert!(!exported.contains("Display=\"Dash\""), "{exported}");
}

#[test]
fn render_cdxml_hash_bond_uses_line_width_bold_width_and_hash_spacing() {
    let cdxml = r#"<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page><fragment>
    <n id="n1" p="0 0"/><n id="n2" p="30 0"/>
    <b id="b1" B="n1" E="n2" Display="Hash"/>
  </fragment></page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("hash geometry")).expect("parses");
    let stripes = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                bond_id: Some(bond_id),
                points,
                ..
            } if bond_id == "b1" => Some(points),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(stripes.len(), 11, "{stripes:?}");
    for stripe in stripes {
        let min_x = stripe
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = stripe
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = stripe
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = stripe
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        assert_close(max_x - min_x, 1.0);
        assert_close(max_y - min_y, 4.0);
    }
}

#[test]
fn parse_cdxml_hash_display_is_ignored_by_chemdraw_for_multiple_bonds() {
    let cdxml = r#"<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page><fragment>
    <n id="n1" p="0 0"/><n id="n2" p="30 0"/>
    <b id="b1" B="n1" E="n2" Order="2" Display="Hash" Display2="Hash" DoublePosition="Center"/>
  </fragment></page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("ignored multiple hash displays")).expect("parses");
    let bond = &document
        .editable_fragment()
        .expect("fragment")
        .fragment
        .bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Hash
    );
    assert_eq!(
        bond.line_styles.left,
        chemsema_engine::BondLinePattern::Hash
    );
    assert_eq!(
        bond.line_styles.right,
        chemsema_engine::BondLinePattern::Hash
    );
    let polygons = render_document(&document)
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    bond_id: Some(bond_id),
                    ..
                } if bond_id == "b1"
            )
        })
        .count();
    assert_eq!(
        polygons, 2,
        "ChemDraw renders an ordinary solid double bond"
    );
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Display=\"Hash\""), "{exported}");
    assert!(exported.contains("Display2=\"Hash\""), "{exported}");
}

#[test]
fn parse_ccjs_migrates_only_the_two_legacy_single_hash_encodings() {
    let legacy = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 0.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [60.0, 0.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [90.0, 0.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "legacy_toolbar", "begin": "n1", "end": "n2", "order": 1,
                "lineStyles": { "main": "dashed" },
                "lineWeights": { "main": "bold" }
            },
            {
                "id": "legacy_menu", "begin": "n2", "end": "n3", "order": 1,
                "lineStyles": { "main": "dashed" },
                "meta": { "contextMenuBondStyle": "single-hashed" }
            },
            {
                "id": "bold_dashed_double", "begin": "n3", "end": "n4", "order": 2,
                "double": { "placement": "center" },
                "lineStyles": { "main": "dashed" },
                "lineWeights": { "main": "bold" }
            }
        ]),
    );
    let reopened = parse_document_json(&serde_json::to_string(&legacy).expect("serializes"))
        .expect("legacy document reopens");
    let fragment = reopened.editable_fragment().expect("fragment").fragment;
    for id in ["legacy_toolbar", "legacy_menu"] {
        let bond = fragment.bonds.iter().find(|bond| bond.id == id).expect(id);
        assert_eq!(
            bond.line_styles.main,
            chemsema_engine::BondLinePattern::Hash
        );
        assert_eq!(
            bond.line_weights.main,
            chemsema_engine::BondLineWeight::Normal
        );
        assert!(bond.meta.get("contextMenuBondStyle").is_none());
    }
    let untouched = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "bold_dashed_double")
        .expect("bold dashed double");
    assert_eq!(
        untouched.line_styles.main,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        untouched.line_weights.main,
        chemsema_engine::BondLineWeight::Bold
    );
}

#[test]
fn parse_cdxml_automatically_positions_query_tags_relative_to_their_bonds() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page id="1"><fragment id="2">
    <n id="3" p="0 0"/><n id="4" p="30 0"/>
    <n id="5" p="50 0"/><n id="6" p="50 30"/>
    <n id="7" p="80 0"/><n id="8" p="80 30"/>
    <n id="9" p="110 0"/><n id="10" p="110 30"/>
    <b id="11" B="3" E="4"><objecttag Name="query">
      <t p="0 -4" BoundingBox="0 -10 12 -4"><s font="3" size="7.5">Rxn</s></t>
    </objecttag></b>
    <b id="12" B="5" E="6"><objecttag Name="query">
      <t p="32 21" BoundingBox="32 15 44 21"><s font="3" size="7.5">Rxn</s></t>
    </objecttag></b>
    <b id="13" B="7" E="8"><objecttag Name="query">
      <t p="86 16" BoundingBox="86 10 98 16"><s font="3" size="7.5">Rxn</s></t>
    </objecttag></b>
    <b id="14" B="9" E="10"><objecttag Name="query" PositioningType="offset" PositioningOffset="2 3">
      <t p="120 18" BoundingBox="120 12 132 18"><s font="3" size="7.5">Rxn</s></t>
    </objecttag></b>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("automatic bond query tags"))
        .expect("bond query tags should parse");
    let labels: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.meta.get("role").and_then(|value| value.as_str()) == Some("query"))
        .collect();
    assert_eq!(labels.len(), 4);
    assert_eq!(labels[0].transform.translate, [1.65, -6.53]);
    assert_eq!(labels[1].transform.translate, [34.48, 15.49]);
    assert_eq!(labels[2].transform.translate, [82.38, 12.3]);
    assert_eq!(labels[3].transform.translate, [120.0, 12.0]);
    for label in &labels[..3] {
        assert_eq!(
            label
                .payload
                .extra
                .get("baselineOffset")
                .and_then(|value| value.as_f64()),
            Some(6.0)
        );
        assert!(label
            .payload
            .extra
            .get("automaticPositioningVector")
            .and_then(|value| value.as_array())
            .is_some_and(|vector| vector.len() == 2));
    }
    let exported = document_to_cdxml(&document);
    let reimported = parse_cdxml_document(&exported, Some("automatic bond query round trip"))
        .expect("exported bond query tags should reimport");
    let reimported_positions = reimported
        .objects
        .iter()
        .filter(|object| object.meta.get("role").and_then(|value| value.as_str()) == Some("query"))
        .map(|object| object.transform.translate)
        .collect::<Vec<_>>();
    assert_eq!(
        reimported_positions,
        labels
            .iter()
            .map(|object| object.transform.translate)
            .collect::<Vec<_>>()
    );
}

#[test]
fn parse_cdxml_keeps_combined_bond_order_query_native_and_renders_its_mnemonic() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page id="1"><fragment id="2">
    <n id="3" p="0 0"/><n id="4" p="30 0"/>
    <b id="5" B="3" E="4" Order="1 2"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("combined bond-order query"))
        .expect("combined bond-order query should parse");
    let bond = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.bonds.first())
        .expect("query bond should import");
    assert_eq!(
        bond.properties.query_orders,
        vec![
            chemsema_engine::BondQueryOrder::Single,
            chemsema_engine::BondQueryOrder::Double
        ]
    );
    assert!(
        document.objects.iter().all(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) != Some("query")
        }),
        "native query fields must not create a second editable text object"
    );
    assert!(render_document(&document)
        .iter()
        .any(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => {
                text == "S/D"
                    || runs.iter().map(|run| run.text.as_str()).collect::<String>() == "S/D"
            }
            _ => false,
        }));
}

#[test]
fn generated_bond_query_uses_chemdraw_text_baseline_not_its_box_top() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="10"><fragment id="11">
    <n id="2" p="157.21 133.17"/>
    <n id="4" p="183.19 148.17"/>
    <b id="5" B="2" E="4" Order="1 2"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("ChemDraw 6 bond query baseline"))
        .expect("bond query should parse");
    let (x, y, baseline_offset) = render_document(&document)
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                x,
                y,
                baseline_offset,
                text,
                runs,
                ..
            } => {
                let rendered = if text.is_empty() {
                    runs.iter().map(|run| run.text.as_str()).collect::<String>()
                } else {
                    text.clone()
                };
                (rendered == "S/D").then_some((*x, *y, *baseline_offset))
            }
            _ => None,
        })
        .expect("generated S/D annotation");

    // ChemDraw saves this generated tag at p=168.23 138.87 with
    // BoundingBox=168.23 132.51 180.73 138.97. Our primitive is centered
    // horizontally, but its y coordinate is the same alphabetic baseline.
    assert!((x - 174.48).abs() < 0.08, "x={x}");
    assert!((y - 138.87).abs() < 0.03, "y={y}");
    assert!(
        baseline_offset.is_some_and(|value| (value - 6.36).abs() < 1.0e-9),
        "baseline_offset={baseline_offset:?}"
    );
}

#[test]
fn generated_bond_query_uses_the_label_clipped_visible_bond_midpoint() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" MarginWidth="2" LineWidth="1">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="5"><fragment id="6">
    <n id="1" p="147 176" NodeType="GenericNickname">
      <t id="4" p="143.64 179.9" BoundingBox="145 173 152 180">
        <s font="3" size="10" face="96">M</s>
      </t>
    </n>
    <n id="2" p="177 176"/>
    <b id="3" B="1" E="2" Order="1 2"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("labeled query-bond endpoint"))
        .expect("labeled query bond should parse");
    let (x, y) = render_document(&document)
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                x, y, text, runs, ..
            } => {
                let rendered = if text.is_empty() {
                    runs.iter().map(|run| run.text.as_str()).collect::<String>()
                } else {
                    text.clone()
                };
                (rendered == "S/D").then_some((*x, *y))
            }
            _ => None,
        })
        .expect("generated S/D annotation");

    // ChemDraw centers the query on the visible line from x=152.385 to
    // x=177, rather than on the raw node segment from x=147 to x=177.
    // Its saved ObjectTag has BoundingBox=158.42 167.02 170.92 173.48
    // and p=158.42 173.38.
    assert!((x - 164.67).abs() < 0.08, "x={x}");
    assert!((y - 173.38).abs() < 0.03, "y={y}");
}

#[test]
fn explicit_bond_query_display_uses_its_authored_object_tag_geometry_once() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" ShowBondRxn="yes">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1"><fragment id="2">
    <n id="3" p="0 0"/><n id="4" p="30 20"/>
    <b id="5" B="3" E="4" RxnParticipation="MakeOrBreak">
      <objecttag Name="query">
        <t p="7 3" BoundingBox="7 -3 19 3"><s font="3" size="7.5">Rxn</s></t>
      </objecttag>
    </b>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("authored bond query display"))
        .expect("bond query display should parse");
    let query_objects = document
        .objects
        .iter()
        .filter(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) == Some("query")
                && object
                    .meta
                    .get("attachedBondId")
                    .and_then(|value| value.as_str())
                    == Some("5")
        })
        .collect::<Vec<_>>();
    assert_eq!(query_objects.len(), 1);
    assert_eq!(query_objects[0].transform.translate, [7.0, -3.0]);

    let rendered = render_document(&document);
    let query_texts = rendered
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                x, y, text, runs, ..
            } => {
                let rendered = if text.is_empty() {
                    runs.iter().map(|run| run.text.as_str()).collect::<String>()
                } else {
                    text.clone()
                };
                (rendered == "Rxn").then_some((*x, *y))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(query_texts.len(), 1, "{query_texts:?}");
    assert_eq!(query_texts[0], (7.0, 3.0));
}

#[test]
fn bond_annotation_visibility_inherits_document_and_keeps_query_order_visible() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelSize="10" ShowBondQuery="no" ShowBondRxn="no" ShowBondStereo="no">
  <page id="1"><fragment id="2">
    <n id="3" p="0 0"/><n id="4" p="30 0"/>
    <b id="5" B="3" E="4" Order="1 2" Topology="Ring"
       RxnParticipation="MakeAndChange" BS="E"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("bond annotation inheritance"))
        .expect("bond annotations should parse");
    let texts = render_document(&document)
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => Some(if text.is_empty() {
                runs.iter().map(|run| run.text.as_str()).collect::<String>()
            } else {
                text.clone()
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text| text == "S/D"), "{texts:?}");
    assert!(!texts
        .iter()
        .any(|text| text.contains("Rng") || text.contains("Rxn")));
    assert!(!texts.iter().any(|text| text == "(E)"));
}

#[test]
fn bond_annotation_side_is_independent_of_bond_endpoint_order() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelSize="10" ShowBondStereo="yes">
  <page id="1"><fragment id="2">
    <n id="3" p="0 0"/><n id="4" p="30 20"/>
    <b id="5" B="3" E="4" Order="1 2" Topology="Ring"
       RxnParticipation="MakeAndChange" BS="E"/>
    <b id="6" B="4" E="3" Order="1 2" Topology="Ring"
       RxnParticipation="MakeAndChange" BS="E"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("reversed bond annotations"))
        .expect("bond annotations should parse");
    let mut positions = render_document(&document)
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                x, y, text, runs, ..
            } => {
                let rendered = if text.is_empty() {
                    runs.iter().map(|run| run.text.as_str()).collect::<String>()
                } else {
                    text.clone()
                };
                matches!(rendered.as_str(), "RngRxnS/D" | "(E)").then_some((rendered, *x, *y))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    positions.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(positions.len(), 4, "{positions:?}");
    for pair in positions.chunks_exact(2) {
        assert_eq!(pair[0].0, pair[1].0);
        assert!((pair[0].1 - pair[1].1).abs() < 1.0e-9, "{pair:?}");
        assert!((pair[0].2 - pair[1].2).abs() < 1.0e-9, "{pair:?}");
    }
}

#[test]
fn parse_cdxml_prefers_absolute_multiple_bond_spacing() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" BondSpacing="12">
  <page id="1"><fragment id="2">
    <n id="3" p="0 0"/><n id="4" p="20 0"/>
    <b id="5" B="3" E="4" Order="2" BondSpacing="40" BondSpacingAbs="2"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("absolute bond spacing"))
        .expect("absolute bond spacing should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("molecule fragment should import");
    assert_eq!(fragment.bonds[0].bond_spacing, Some(40.0));
    assert_eq!(fragment.bonds[0].bond_spacing_absolute, Some(2.0));
    assert!((imported_double_bond_center_spacing(&document, "obj_mol_001") - 2.0).abs() < 0.001);
    let native_json = serde_json::to_string(&document).expect("native JSON should serialize");
    assert!(
        native_json.contains("\"bondSpacingAbsolute\":2.0"),
        "{native_json}"
    );
    let reopened: ChemSemaDocument =
        serde_json::from_str(&native_json).expect("native JSON should deserialize");
    assert!((imported_double_bond_center_spacing(&reopened, "obj_mol_001") - 2.0).abs() < 0.001);
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("BondSpacing=\"40\""), "{exported}");
    assert!(exported.contains("BondSpacingAbs=\"2\""), "{exported}");
}

#[test]
fn parse_cdxml_imports_visible_number_and_query_object_tags_inside_bonded_nodes() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page id="1">
    <fragment id="2">
      <n id="3" p="20 20" ShowAtomNumber="yes" AtomNumber="1">
        <objecttag Name="number">
          <t p="16 14" BoundingBox="16 8 20 14"><s font="3" size="7.5">1</s></t>
        </objecttag>
        <objecttag Name="query">
          <t p="23 28" BoundingBox="23 22 36 28"><s font="3" size="7.5">Any</s></t>
        </objecttag>
      </n>
      <n id="4" p="50 20" ShowAtomNumber="yes" AtomNumber="2">
        <objecttag Name="number" Visible="no">
          <t p="46 14" BoundingBox="46 8 50 14"><s font="3" size="7.5">2</s></t>
        </objecttag>
      </n>
      <b id="5" B="3" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("number and query tags"))
        .expect("number and query object tags should parse");
    let tagged_text: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .map(|object| {
            (
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default(),
                object
                    .meta
                    .get("role")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default(),
                object.visible,
            )
        })
        .collect();

    assert_eq!(
        tagged_text,
        vec![
            ("1", "atom_number", true),
            ("Any", "query", true),
            ("2", "atom_number", false),
        ]
    );
}

#[test]
fn parse_cdxml_only_paints_unknown_object_tags_when_explicitly_visible() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML>
  <page id="1">
    <objecttag Name="producer-note">
      <t p="10 10" BoundingBox="10 4 40 10"><s font="3" size="7.5">hidden</s></t>
    </objecttag>
    <objecttag Name="producer-note" Visible="yes">
      <t p="10 20" BoundingBox="10 14 40 20"><s font="3" size="7.5">shown</s></t>
    </objecttag>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("unknown object-tag visibility"))
        .expect("unknown object tags should parse");
    let visibility = document
        .objects
        .iter()
        .filter(|object| object.object_type == "text")
        .map(|object| {
            (
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(|value| value.as_str()),
                object.visible,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visibility,
        vec![(Some("hidden"), false), (Some("shown"), true)]
    );
}

#[test]
fn parse_cdxml_synthesizes_missing_enhanced_stereo_object_tag_opposite_wedge() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="2">
      <n id="3" p="20 20" EnhancedStereoType="Or" EnhancedStereoGroupNum="2"/>
      <n id="4" p="10 30"/>
      <b id="5" B="3" E="4" Display="WedgeBegin"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("automatic enhanced stereo"))
        .expect("enhanced stereo fields should parse");
    let label = document
        .objects
        .iter()
        .find(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) == Some("enhanced_stereo")
        })
        .expect("missing objecttag should synthesize a visible enhanced-stereo label");

    assert_eq!(
        label
            .payload
            .extra
            .get("text")
            .and_then(|value| value.as_str()),
        Some("or2")
    );
    assert_eq!(
        label
            .meta
            .get("synthetic")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(
        label.transform.translate[0] > 20.0,
        "label should sit opposite the down-left wedge"
    );
    assert!(
        label.transform.translate[1] < 20.0,
        "label should sit above the stereocenter"
    );
}

#[test]
fn render_cdxml_node_display_markers_from_official_node_properties() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LineWidth="1" BoldWidth="4" HashSpacing="2.7">
  <page id="1">
    <fragment id="2">
      <n id="3" p="20 20" Geometry="Tetrahedral" HDot="yes"/>
      <n id="4" p="50 20" Geometry="Tetrahedral" HDash="yes"/>
      <n id="5" p="80 20" NodeType="MultiAttachment" Attachments="6 7 8"/>
      <b id="6" B="3" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("node markers"))
        .expect("node display markers should parse");
    let primitives = render_document(&document);
    let dot_count = primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    radius,
                    fill,
                    stroke_width,
                    ..
                } if (*radius - 2.0).abs() < 1.0e-6
                    && fill == "#000000"
                    && stroke_width.abs() < 1.0e-6
            )
        })
        .count();
    let marker_path_count = primitives
        .iter()
        .filter(|primitive| matches!(primitive, RenderPrimitive::Path { bond_id: None, .. }))
        .count();

    assert_eq!(dot_count, 1, "HDot should render one filled dot");
    assert_eq!(
        marker_path_count, 5,
        "HDash should render two bars and an unbonded MultiAttachment three rays"
    );
    let multi_attachment_vertical = primitives.iter().find_map(|primitive| match primitive {
        RenderPrimitive::Path {
            bond_id: None,
            points,
            stroke_width,
            ..
        } if points.len() == 2
            && (points[0].x - points[1].x).abs() < 1.0e-6
            && (points[0].y - points[1].y).abs() > 8.0 =>
        {
            Some((points[0], points[1], *stroke_width))
        }
        _ => None,
    });
    let (from, to, stroke_width) =
        multi_attachment_vertical.expect("MultiAttachment should include a vertical ray");
    assert!(((from.y - to.y).abs() - 9.0).abs() < 1.0e-6);
    assert!((stroke_width - 1.0).abs() < 1.0e-6);
}

#[test]
fn load_cdxml_dragging_unselected_bracket_side_does_not_move_other_side() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="20" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="g1" BoundingBox="40 90 40 20" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="g2" BoundingBox="150 20 150 90" GraphicType="Bracket" BracketType="Square"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("bracket pair cdxml should load");
    engine.set_tool_state(select_tool_state());

    let group = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object_is_bracket_group(object))
        .expect("paired cdxml brackets should import as a bracket group");
    let left_id = group
        .children
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("left")
        })
        .map(|object| object.id.clone())
        .expect("left side should import");
    let right_id = group
        .children
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("side")
                .and_then(|value| value.as_str())
                == Some("right")
        })
        .map(|object| object.id.clone())
        .expect("right side should import");

    let left_before = engine
        .state()
        .document
        .find_scene_object(&left_id)
        .expect("left side should remain")
        .clone();
    let right_before = engine
        .state()
        .document
        .find_scene_object(&right_id)
        .expect("right side should remain")
        .clone();
    let left_height = left_before.payload.bbox.expect("left side bbox")[3];
    let start = Point::new(
        left_before.transform.translate[0] + 0.5,
        left_before.transform.translate[1] + left_height * 0.5,
    );
    let end = Point::new(start.x + 12.0, start.y + 6.0);

    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec![left_id.clone()]
    );
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let left_after = engine
        .state()
        .document
        .find_scene_object(&left_id)
        .expect("left side should remain");
    let right_after = engine
        .state()
        .document
        .find_scene_object(&right_id)
        .expect("right side should remain");
    assert_eq!(
        left_after.transform.translate,
        [
            round_to_2(left_before.transform.translate[0] + 12.0),
            round_to_2(left_before.transform.translate[1] + 6.0)
        ]
    );
    assert_eq!(
        right_after.transform.translate,
        right_before.transform.translate
    );
    assert_eq!(engine.state().selection.arrow_objects, vec![left_id]);
}

#[test]
fn load_cdxml_bracketusage_repeat_count_feeds_selection_summary() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="20" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="f1" BoundingBox="0 -10 60 10">
      <n id="n1" p="0 0"/>
      <n id="n2" p="20 0"/>
      <n id="n3" p="40 0"/>
      <n id="n4" p="60 0"/>
      <b id="b1" B="n1" E="n2" Order="1"/>
      <b id="b2" B="n2" E="n3" Order="1"/>
      <b id="b3" B="n3" E="n4" Order="1"/>
    </fragment>
    <graphic id="g1" BoundingBox="15 10 15 -10" GraphicType="Bracket" BracketType="Square"/>
    <graphic id="g2" BoundingBox="45 -10 45 10" GraphicType="Bracket" BracketType="Square">
      <objecttag id="ot1" Name="bracketusage">
        <t p="0 0" BoundingBox="0 -6.30 4.17 0"><s font="3" size="7.5" color="0">3</s></t>
      </objecttag>
    </graphic>
    <bracketedgroup id="bg1" BracketUsage="MultipleGroup" RepeatCount="3">
      <bracketattachment id="ba1" GraphicID="g1"/>
      <bracketattachment id="ba2" GraphicID="g2"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("bracketed cdxml should load");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("repeat count should produce a repeating unit");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["repeatCount"]["value"], 3);
    assert!(units[0]["countTextObjectId"].is_null());

    assert!(engine.select_all());
    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "C8H18");
    assert_eq!(summary["atomCount"], 26);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 114.232).abs() < 1.0e-9);
    assert!((summary["exactMass"].as_f64().unwrap() - 114.140_850_580_14).abs() < 1.0e-9);
}

#[test]
fn parse_cdxml_bracket_label_fixtures_match_chemdraw_offsets() {
    for fixture in [
        "manual/desktop/kuohao.cdxml",
        "manual/desktop/kuohao-acs.cdxml",
    ] {
        let cdxml = read_cdxml_fixture(fixture);
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should parse");
        let bracket_groups: Vec<_> = document
            .scene_objects()
            .into_iter()
            .filter(|object| object_is_bracket_group(object))
            .collect();
        let labels: Vec<_> = document
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "text" && object.visible)
            .filter(|object| {
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(|value| value.as_str())
                    == Some("apple")
            })
            .collect();
        assert_eq!(
            bracket_groups.len(),
            3,
            "{fixture} should import three bracket pairs"
        );
        assert_eq!(
            labels.len(),
            3,
            "{fixture} should import three bracket labels"
        );

        for label in labels {
            let style = document
                .styles
                .get(
                    label
                        .style_ref
                        .as_ref()
                        .expect("label should have text style"),
                )
                .expect("label style should exist");
            assert_eq!(
                style.get("fontSize").and_then(|value| value.as_f64()),
                Some(7.5)
            );
            assert_eq!(
                label
                    .payload
                    .extra
                    .get("fontSize")
                    .and_then(|value| value.as_f64()),
                Some(7.5)
            );

            let label_anchor_y = label.transform.translate[1]
                + label
                    .payload
                    .extra
                    .get("baselineOffset")
                    .and_then(|value| value.as_f64())
                    .expect("label should keep CDXML baseline offset");
            let mut closest = None::<(f64, f64, f64)>;
            for bracket in &bracket_groups {
                let bbox = bracket.payload.bbox.expect("bracket should have bbox");
                let right = bracket.transform.translate[0] + bbox[0] + bbox[2];
                let bottom = bracket.transform.translate[1] + bbox[1] + bbox[3];
                let dx = label.transform.translate[0] - right;
                let dy = label_anchor_y - bottom;
                let score = (dx - 1.41).abs() + (dy - 2.4).abs();
                closest = match closest {
                    Some((best_score, _, _)) if best_score <= score => closest,
                    _ => Some((score, dx, dy)),
                };
            }
            let (_, dx, dy) = closest.expect("label should match a right bracket");
            assert!(
                (dx - 1.41).abs() < 0.02,
                "{fixture} automatic bracket label x offset should be 0.1875 em, got {dx}"
            );
            assert!(
                (2.30..=2.50).contains(&dy),
                "{fixture} bracket label baseline y offset should be about 2.4 pt, got {dy}"
            );
        }
    }
}

#[test]
fn parse_cdxml_preserves_small_text_object_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 18 28" Justification="Left" UTF8Text="x">
      <s font="3" size="6" face="0" color="0">x</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("small text")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let bbox: [f64; 4] = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("text object should preserve box"),
    )
    .expect("text box should deserialize");

    assert_eq!(bbox, [0.0, 0.0, 8.0, 8.0]);
}

#[test]
fn parse_cdxml_preserves_aligned_text_object_source_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="30 10 70 30" Justification="Center" UTF8Text="center">
      <s font="3" size="10" face="0" color="0">center</s>
    </t>
    <t id="3" p="120 20" BoundingBox="80 10 120 30" Justification="Right" UTF8Text="right">
      <s font="3" size="10" face="0" color="0">right</s>
    </t>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("aligned text")).expect("text cdxml should parse");
    let text_objects: Vec<_> = document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text")
        .collect();

    let center = text_objects
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(serde_json::Value::as_str)
                == Some("center")
        })
        .expect("center text should import");
    let center_box: [f64; 4] = serde_json::from_value(
        center
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("center text should preserve box"),
    )
    .expect("center box should deserialize");
    assert_eq!(center.transform.translate, [50.0, 10.0]);
    assert_eq!(center_box, [-20.0, 0.0, 40.0, 20.0]);

    let right = text_objects
        .iter()
        .find(|object| {
            object
                .payload
                .extra
                .get("text")
                .and_then(serde_json::Value::as_str)
                == Some("right")
        })
        .expect("right text should import");
    let right_box: [f64; 4] = serde_json::from_value(
        right
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("right text should preserve box"),
    )
    .expect("right box should deserialize");
    assert_eq!(right.transform.translate, [120.0, 10.0]);
    assert_eq!(right_box, [-40.0, 0.0, 40.0, 20.0]);
}

#[test]
fn load_cdxml_document_hit_tests_aligned_text_object_source_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="30 10 70 30" Justification="Center" UTF8Text="center">
      <s font="3" size="10" face="0" color="0">center</s>
    </t>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    engine.select_at_point(
        Point::new(31.0 * CDXML_EDIT_SCALE, 20.0 * CDXML_EDIT_SCALE),
        false,
    );
    assert_eq!(engine.state().selection.text_objects, vec!["obj_text_001"]);

    engine.select_at_point(
        Point::new(71.0 * CDXML_EDIT_SCALE, 20.0 * CDXML_EDIT_SCALE),
        false,
    );
    assert!(engine.state().selection.text_objects.is_empty());
}

#[test]
fn parse_document_json_migrates_legacy_aligned_text_object_box() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
            },
            "objects": [{
                "id": "obj_text_001",
                "type": "text",
                "transform": { "translate": [50.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "text": "center",
                    "align": "center",
                    "box": [0.0, 0.0, 40.0, 20.0]
                }
            }]
        })
        .to_string(),
    )
    .expect("document json should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let box_value: [f64; 4] = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("box")
            .cloned()
            .expect("text object should preserve migrated box"),
    )
    .expect("text box should deserialize");

    assert_eq!(box_value, [-20.0, 0.0, 40.0, 20.0]);
}

#[test]
fn parse_cdxml_formula_face_expands_digits_to_subscript() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 80 36" Justification="Left" UTF8Text="CF3">
      <s font="3" size="12" face="97" color="0">CF3</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("formula")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("formula text should import");
    let runs: Vec<chemsema_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("imported text should preserve runs"),
    )
    .expect("runs should deserialize");

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "CF");
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[0].script.as_deref(), Some("normal"));
    assert_eq!(runs[1].text, "3");
    assert_eq!(runs[1].font_weight, Some(700));
    assert_eq!(runs[1].script.as_deref(), Some("subscript"));
}

#[test]
fn parse_cdxml_chemical_face_subscripts_group_multipliers() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 36" Justification="Left" UTF8Text="ArB(OH)2">
      <s font="3" size="10" face="96" color="0">ArB(OH)2</s>
    </t>
    <t id="3" p="10 40" BoundingBox="10 40 160 56" Justification="Left" UTF8Text="Cu(CH3CN)4PF6">
      <s font="3" size="10" face="96" color="0">Cu(CH3CN)4PF6</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("formula")).expect("text cdxml should parse");
    let text_runs = |text: &str| -> Vec<chemsema_engine::LabelRun> {
        let object = document
            .objects
            .iter()
            .find(|object| {
                object
                    .payload
                    .extra
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    == Some(text)
            })
            .expect("formula text should import");
        serde_json::from_value(
            object
                .payload
                .extra
                .get("runs")
                .cloned()
                .expect("imported text should preserve runs"),
        )
        .expect("runs should deserialize")
    };

    let ar_boron = text_runs("ArB(OH)2");
    assert_eq!(ar_boron.last().map(|run| run.text.as_str()), Some("2"));
    assert_eq!(
        ar_boron.last().and_then(|run| run.script.as_deref()),
        Some("subscript")
    );

    let copper = text_runs("Cu(CH3CN)4PF6");
    let subscript_text: Vec<_> = copper
        .iter()
        .filter(|run| run.script.as_deref() == Some("subscript"))
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(subscript_text, vec!["3", "4", "6"]);
}

#[test]
fn parse_cdxml_decodes_face_bit_combinations() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 120 36" Justification="Left" UTF8Text="A2B+NO2">
      <s font="3" size="12" face="39" color="0">A2</s>
      <s font="3" size="12" face="70" color="0">B+</s>
      <s font="3" size="12" face="103" color="0">NO2</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("faces")).expect("text cdxml should parse");
    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let runs: Vec<chemsema_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("imported text should preserve runs"),
    )
    .expect("runs should deserialize");

    assert_eq!(runs[0].text, "A2");
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[0].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[0].underline, Some(true));
    assert_eq!(runs[0].script.as_deref(), Some("subscript"));

    assert_eq!(runs[1].text, "B+");
    assert_eq!(runs[1].font_weight, Some(400));
    assert_eq!(runs[1].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[1].underline, Some(true));
    assert_eq!(runs[1].script.as_deref(), Some("superscript"));

    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        vec!["A2", "B+", "NO", "2"]
    );
    assert_eq!(runs[2].font_weight, Some(700));
    assert_eq!(runs[2].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[2].underline, Some(true));
    assert_eq!(runs[2].script.as_deref(), Some("normal"));
    assert_eq!(runs[3].font_weight, Some(700));
    assert_eq!(runs[3].font_style.as_deref(), Some("italic"));
    assert_eq!(runs[3].underline, Some(true));
    assert_eq!(runs[3].script.as_deref(), Some("subscript"));
}

#[test]
fn cdxml_outline_shadow_and_custom_font_use_native_semantic_fields() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="12" LabelFace="24">
  <fonttable><font id="3" charset="iso-8859-1" name="Aptos Display"/></fonttable>
  <page id="1">
    <t id="2" p="10 20" BoundingBox="10 20 80 36" Justification="Left" UTF8Text="Effect">
      <s font="3" size="12" face="24" color="0">Effect</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("effects")).expect("text cdxml should parse");
    assert_eq!(document.style.label_style.font_family, "Aptos Display");
    assert!(document.style.label_style.outline);
    assert!(document.style.label_style.shadow);

    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text should import");
    let run = text_object.payload.extra["runs"][0]
        .as_object()
        .expect("run should be an object");
    assert_eq!(run.get("fontFamily"), Some(&json!("Aptos Display")));
    assert_eq!(run.get("outline"), Some(&json!(true)));
    assert_eq!(run.get("shadow"), Some(&json!(true)));
    assert!(
        !run.contains_key("face"),
        "CDXML face must not enter CCJS runs"
    );
    let native_json = serde_json::to_string(&document).expect("document should serialize");
    assert!(
        !native_json.contains("\"face\""),
        "CDXML face must not be stored anywhere in native JSON: {native_json}"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("LabelFace=\"24\""), "{exported}");
    assert!(exported.contains("face=\"24\""), "{exported}");
    assert!(exported.contains("name=\"Aptos Display\""), "{exported}");
}

#[test]
fn parse_cdxml_imports_table_lines_and_text_boxes() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <graphic id="2" GraphicType="Line" LineType="Dashed" Head3D="10 10 0" Tail3D="80 10 0"/>
    <t id="3" p="12 14" BoundingBox="12 14 60 30" Justification="Left">
      <s font="3" size="10" color="0">entry</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("table")).expect("table cdxml should parse");
    let line = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("plain table line should import");
    assert!(!line.payload.extra.contains_key("arrowHead"));
    let line_style = document
        .styles
        .get(line.style_ref.as_deref().expect("line style ref"))
        .expect("line style should exist");
    assert_eq!(line_style["dashArray"], json!([2.5]));
    assert!(document
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polyline {
            role: RenderRole::DocumentGraphic,
            dash_array,
            ..
        } if !dash_array.is_empty()
    )));
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            text,
            runs,
            ..
        } if text == "entry" || runs.iter().any(|run| run.text == "entry")
    )));
}

#[test]
fn parse_cdxml_imports_line_endpoints_from_bounding_box() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60">
  <page id="1">
    <graphic id="2" BoundingBox="80 10 20 10" GraphicType="Line"/>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("bbox line")).expect("parse cdxml");
    let line = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("BoundingBox-only line should import");
    assert_eq!(
        line.payload.extra["points"],
        json!([[20.0, 10.0], [80.0, 10.0]])
    );
    assert!(render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polyline {
            role: RenderRole::DocumentGraphic,
            line_cap: Some(line_cap),
            line_join: Some(line_join),
            ..
        } if line_cap == "butt" && line_join == "miter"
    )));
}

#[test]
fn parse_cdxml_centers_standalone_horizontal_curly_bracket_on_ordered_bbox() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.6">
  <page id="1">
    <graphic id="2" BoundingBox="140 60 20 60" GraphicType="Bracket"
      BracketType="Curly" LipSize="60"/>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("standalone bracket")).expect("parse cdxml");
    let bracket = document
        .objects
        .iter()
        .find(|object| object.object_type == "bracket")
        .expect("standalone bracket should import");
    assert_eq!(bracket.payload.extra["orientation"], json!("horizontal"));
    assert_eq!(bracket.transform.rotate, -90.0);
    let primitives = render_document(&document);
    let points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                rotate,
                points,
                ..
            } if (*rotate + 90.0).abs() <= f64::EPSILON => Some(points),
            _ => None,
        })
        .expect("horizontal curly path should render");
    let minimum_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let maximum_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((minimum_y + maximum_y - 120.0).abs() <= 1e-9);
    assert!(minimum_y < 60.0 && maximum_y > 60.0);
    let exported = document_to_cdxml(&document);
    assert!(
        exported.contains("BoundingBox=\"140 60 20 60\""),
        "{exported}"
    );
    let reopened =
        parse_cdxml_document(&exported, Some("reopened standalone bracket")).expect("reopens");
    let reopened_bracket = reopened
        .objects
        .iter()
        .find(|object| object.object_type == "bracket")
        .expect("standalone bracket survives export");
    assert_eq!(
        reopened_bracket.payload.extra["orientation"],
        json!("horizontal")
    );
    assert_eq!(reopened_bracket.transform, bracket.transform);
    assert_eq!(reopened_bracket.payload.bbox, bracket.payload.bbox);
}

#[test]
fn parse_cdxml_centers_vertical_and_diagonal_curly_brackets_on_ordered_bbox() {
    for (name, bbox, expected_orientation) in [
        ("vertical", [100.0, 160.0, 100.0, 40.0], "vertical"),
        ("diagonal", [180.0, 160.0, 60.0, 40.0], "diagonal"),
    ] {
        let cdxml = format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.6">
  <page id="1">
    <graphic id="2" BoundingBox="{} {} {} {}" GraphicType="Bracket"
      BracketType="Curly" LipSize="60"/>
  </page>
</CDXML>"##,
            bbox[0], bbox[1], bbox[2], bbox[3]
        );
        let document = parse_cdxml_document(&cdxml, Some(name)).expect("parse curly bracket");
        let bracket = document
            .objects
            .iter()
            .find(|object| object.object_type == "bracket")
            .expect("standalone curly bracket should import at every angle");
        assert_eq!(
            bracket.payload.extra["orientation"],
            json!(expected_orientation)
        );
        let points = render_document(&document)
            .into_iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic,
                    points,
                    ..
                } => Some(points),
                _ => None,
            })
            .expect("curly path should render");
        let dx = bbox[2] - bbox[0];
        let dy = bbox[3] - bbox[1];
        let length = f64::hypot(dx, dy);
        let normal = [-dy / length, dx / length];
        let center = [(bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5];
        let projections: Vec<_> = points
            .iter()
            .map(|point| (point.x - center[0]) * normal[0] + (point.y - center[1]) * normal[1])
            .collect();
        let minimum = projections.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = projections
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (minimum + maximum).abs() <= 1e-9,
            "{name}: {minimum}..{maximum}"
        );
        assert!(
            minimum < 0.0 && maximum > 0.0,
            "{name}: {minimum}..{maximum}"
        );
        let exported = document_to_cdxml(&document);
        assert!(
            exported.contains(&format!(
                "BoundingBox=\"{} {} {} {}\"",
                bbox[0] as i32, bbox[1] as i32, bbox[2] as i32, bbox[3] as i32
            )),
            "{name}: {exported}"
        );
    }
}

#[test]
fn parse_cdxml_preserves_arbitrary_angle_bracket_endpoints() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.6">
  <page id="1">
    <graphic id="20" BoundingBox="646.48 107.65 660.58 83.24"
      GraphicType="Bracket" BracketType="Square" LipSize="60"/>
    <graphic id="21" BoundingBox="692.99 101.96 678.89 126.37"
      GraphicType="Bracket" BracketType="Square" LipSize="60"/>
    <fragment id="39"><n id="40" p="670 105" Element="8"/></fragment>
    <bracketedgroup id="30" BracketedObjectIDs="40" BracketUsage="MultipleGroup">
      <bracketattachment id="31" GraphicID="20"/>
      <bracketattachment id="32" GraphicID="21"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("angled brackets")).expect("parse cdxml");
    let group = document
        .objects
        .iter()
        .find(|object| object_is_bracket_group(object))
        .expect("paired bracket group");
    let left = group
        .children
        .iter()
        .find(|object| object.payload.extra["side"] == json!("left"))
        .expect("left bracket");
    let right = group
        .children
        .iter()
        .find(|object| object.payload.extra["side"] == json!("right"))
        .expect("right bracket");
    assert!(
        (left.transform.rotate - 30.012_11).abs() < 1.0e-5,
        "{left:?}"
    );
    assert!(
        (right.transform.rotate - 30.012_11).abs() < 1.0e-5,
        "{right:?}"
    );
    assert!(
        left.payload.bbox.expect("left bbox")[2] < 3.0,
        "endpoint x displacement is rotation, not bracket depth: {left:?}"
    );
    assert!(
        render_document(&document)
            .iter()
            .filter(|primitive| matches!(
                primitive,
                RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic,
                    rotate,
                    ..
                        } if (*rotate - 30.012_11).abs() <= 1.0e-5
            ))
            .count()
            >= 2
    );

    let exported = document_to_cdxml(&document);
    assert!(
        exported.contains("BoundingBox=\"646.48 107.65 660.58 83.24\""),
        "{exported}"
    );
    assert!(
        exported.contains("BoundingBox=\"692.99 101.96 678.89 126.37\""),
        "{exported}"
    );
    assert!(exported.contains("LineWidth=\"0.6\""), "{exported}");

    let reopened =
        parse_cdxml_document(&exported, Some("reopened angled brackets")).expect("reopens");
    let reopened_group = reopened
        .objects
        .iter()
        .find(|object| object_is_bracket_group(object))
        .expect("reopened paired bracket group");
    for (before, after) in group.children.iter().zip(&reopened_group.children) {
        assert_eq!(after.payload.bbox, before.payload.bbox);
        assert_eq!(after.transform, before.transform);
    }
}

#[test]
fn parse_cdxml_round_bracket_geometry_does_not_cross_rounding_thresholds() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.5">
  <page id="1">
    <graphic id="20" BoundingBox="21.272 115.064 21.272 16.05"
      GraphicType="Bracket" BracketType="Round" LipSize="60"/>
    <graphic id="21" BoundingBox="99.424 16.05 99.424 115.064"
      GraphicType="Bracket" BracketType="Round" LipSize="60"/>
    <bracketedgroup id="30" BracketedObjectIDs="">
      <bracketattachment id="31" GraphicID="20"/>
      <bracketattachment id="32" GraphicID="21"/>
    </bracketedgroup>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("precise round brackets")).expect("parse");
    let group = document
        .objects
        .iter()
        .find(|object| object_is_bracket_group(object))
        .expect("paired bracket group");
    let exported = document_to_cdxml(&document);
    assert!(
        exported.contains("BoundingBox=\"21.272 115.064 21.272 16.05\""),
        "{exported}"
    );
    assert!(
        exported.contains("BoundingBox=\"99.424 16.05 99.424 115.064\""),
        "{exported}"
    );
    let reopened =
        parse_cdxml_document(&exported, Some("reopened precise round brackets")).expect("reopens");
    let reopened_group = reopened
        .objects
        .iter()
        .find(|object| object_is_bracket_group(object))
        .expect("reopened paired bracket group");
    for (before, after) in group.children.iter().zip(&reopened_group.children) {
        assert_eq!(after.payload.bbox, before.payload.bbox);
        assert_eq!(after.transform, before.transform);
    }
}

#[test]
fn parse_cdxml_displays_isolated_group_16_and_17_hydrides_hydrogen_first() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" LabelFace="96">
  <fonttable><font id="3" name="Arial" charset="iso-8859-1"/></fonttable>
  <page id="1">
    <fragment id="2">
      <n id="3" p="20 20" Element="17" NumHydrogens="1">
        <t p="20 20" LabelJustification="Left"><s font="3" size="10" face="96">ClH</s></t>
      </n>
      <n id="4" p="60 20" Element="8" NumHydrogens="2">
        <t p="60 20" LabelJustification="Left"><s font="3" size="10" face="96">OH2</s></t>
      </n>
      <n id="5" p="100 20" Element="7" NumHydrogens="3">
        <t p="100 20" LabelJustification="Left"><s font="3" size="10" face="96">NH3</s></t>
      </n>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("isolated hydrides")).expect("parse cdxml");
    let rendered_text = render_document(&document)
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => Some(if runs.is_empty() {
                text.clone()
            } else {
                runs.iter().map(|run| run.text.as_str()).collect::<String>()
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        rendered_text.iter().any(|text| text == "HCl"),
        "{rendered_text:?}"
    );
    assert!(
        rendered_text.iter().any(|text| text == "H2O"),
        "{rendered_text:?}"
    );
    assert!(
        rendered_text.iter().any(|text| text == "NH3"),
        "{rendered_text:?}"
    );
}

#[test]
fn parse_cdxml_renders_acs_dashed_bond_patterns_like_chemdraw() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="80 35 116 45">
      <n id="3" p="80 40"/>
      <n id="4" p="116 40"/>
      <b id="5" B="3" E="4" Display="Dash"/>
    </fragment>
    <fragment id="6" BoundingBox="80 65 116 80">
      <n id="7" p="80 70"/>
      <n id="8" p="116 70"/>
      <b id="9" B="7" E="8" Order="2" Display2="Dash"/>
    </fragment>
    <fragment id="10" BoundingBox="80 95 116 110">
      <n id="11" p="80 100"/>
      <n id="12" p="116 100"/>
      <b id="13" B="11" E="12" Order="2" Display="Dash" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("acs dash patterns")).expect("parse cdxml");
    let primitives = render_document(&document);

    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_001"),
        8
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_001"),
        0
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_002"),
        9
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_002"),
        0
    );
    assert_eq!(
        document_bond_polygon_count_for_object(&primitives, "obj_mol_003"),
        16
    );
    assert_eq!(
        document_knockout_count_for_object(&primitives, "obj_mol_003"),
        0
    );

    let single_segments = document_bond_axis_intervals_for_object(&primitives, "obj_mol_001");
    assert_eq!(single_segments.len(), 8, "{single_segments:?}");
    assert!(
        (single_segments[0].0 - 0.0).abs() < 0.01 && (single_segments[0].1 - 2.4).abs() < 0.01,
        "{single_segments:?}"
    );
    assert!(
        (single_segments[1].0 - 4.8).abs() < 0.01 && (single_segments[1].1 - 7.2).abs() < 0.01,
        "{single_segments:?}"
    );
    assert!(
        (single_segments[7].0 - 33.6).abs() < 0.01 && (single_segments[7].1 - 36.0).abs() < 0.01,
        "{single_segments:?}"
    );
    let solid_dash_lengths = document_bond_axis_lengths_for_object(&primitives, "obj_mol_002");
    assert!(
        solid_dash_lengths
            .iter()
            .filter(|length| (**length - 2.4).abs() < 0.01)
            .count()
            == 8
            && solid_dash_lengths.iter().any(|length| *length > 35.0),
        "{solid_dash_lengths:?}"
    );
    let double_dash_lengths = document_bond_axis_lengths_for_object(&primitives, "obj_mol_003");
    assert!(
        double_dash_lengths
            .iter()
            .all(|length| (*length - 2.4).abs() < 0.01),
        "{double_dash_lengths:?}"
    );
}

#[test]
fn parse_cdxml_imports_published_formula_face_node_labels_with_subscripts() {
    let Some(cdxml) = read_optional_cdxml_fixture("figure2.cdxml") else {
        return;
    };
    let document =
        parse_cdxml_document(&cdxml, Some("figure2")).expect("published cdxml should parse");
    let cf3_label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .filter_map(|node| node.label.as_ref())
        .find(|label| label.source_text.as_deref() == Some("CF3"))
        .expect("example should import CF3 node label");

    assert_eq!(cf3_label.text, "CF3");
    assert_eq!(
        cf3_label
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        vec!["CF", "3"]
    );
    assert_eq!(cf3_label.runs[1].script.as_deref(), Some("subscript"));
    assert_eq!(
        cf3_label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("CF3")
    );
}

#[test]
fn load_cdxml_document_uses_internal_single_character_below_label_position() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="p1" BoundingBox="238.76 122.79 310.06 156.07">
    <fragment id="f1" BoundingBox="238.76 122.79 310.06 156.07">
      <n id="n1" p="256.05 139.70"/>
      <n id="n2" p="270.45 139.70" NodeType="Fragment">
        <t id="t1" p="268.70 143.60" BoundingBox="268.70 138.06 272.20 143.60" LabelJustification="Left" LabelAlignment="Below" UTF8Text="•">
          <s font="3" size="10" color="0" face="96">•</s>
        </t>
      </n>
      <n id="n3" p="284.85 139.70"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("imported fragment should be editable");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| {
            node.label
                .as_ref()
                .is_some_and(|label| label.source_text.as_deref() == Some("•"))
        })
        .expect("bullet label node should import");
    let label = node.label.as_ref().expect("node should have a label");
    let position = label.position.expect("bullet label should keep position");
    let world_position = [
        entry.object.transform.translate[0] + position[0],
        entry.object.transform.translate[1] + position[1],
    ];

    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(
        !node.is_placeholder,
        "the CDXML bullet is a visible carbon atom, not an invalid text placeholder"
    );
    assert!(
        node.meta.get("labelRecognition").is_none(),
        "the CDXML bullet carbon should not be marked as an invalid functional label"
    );
    assert_eq!(label.text, "•");
    assert!(
        label.meta.get("labelRecognition").is_none(),
        "the CDXML bullet carbon label should not carry invalid-label metadata"
    );
    assert_eq!(
        label
            .meta
            .pointer("/import/cdxml/labelAlignment")
            .and_then(serde_json::Value::as_str),
        Some("Below")
    );
    assert!(
        (world_position[0] - 268.68).abs() < 0.01,
        "single-character CDXML labels should use internal below-label x, got {world_position:?}"
    );
    assert!(
        (world_position[1] - 143.60).abs() < 0.01,
        "single-character CDXML labels should use internal below-label y, got {world_position:?}"
    );
}

#[test]
fn parse_cdxml_keeps_numeric_suffix_node_label_anchored_on_letter() {
    fn labeled_nodes(
        document: &ChemSemaDocument,
    ) -> Vec<(&chemsema_engine::Node, &chemsema_engine::NodeLabel)> {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .filter_map(|node| node.label.as_ref().map(|label| (node, label)))
            .collect()
    }

    fn anchor_of(label: &chemsema_engine::NodeLabel, index: usize) -> Point {
        let polygon = label
            .glyph_polygons
            .get(index)
            .expect("glyph polygon should exist");
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for [x, y] in polygon {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
        Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
    }

    let invalid_cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 40 20">
      <n id="n1" p="10 10" NodeType="Nickname">
        <t id="t1" p="10 14" BoundingBox="4 4 16 16" UTF8Text="X3">
          <s font="3" size="10" face="96">X3</s>
        </t>
      </n>
      <n id="n2" p="24.4 10"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let invalid_imported =
        parse_cdxml_document(invalid_cdxml, Some("invalid")).expect("invalid label cdxml");
    let (invalid_node, invalid_label) = labeled_nodes(&invalid_imported)
        .into_iter()
        .find(|(_, label)| label.source_text.as_deref() == Some("X3"))
        .expect("invalid X3 label should import");
    assert_eq!(
        invalid_label
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
    let invalid_anchor = anchor_of(invalid_label, 0);
    let invalid_line_anchor_y = invalid_label.position.expect("invalid label baseline")[1]
        - invalid_label.font_size.unwrap_or(10.0) * 0.39;
    assert!(
        (invalid_anchor.x - invalid_node.position[0]).abs() < 0.01
            && (invalid_line_anchor_y - invalid_node.position[1]).abs() < 0.01,
        "invalid labels should prefer non-script glyph x anchors and label-line y anchors over subscript/superscript glyphs: node={invalid_node:?}, label={invalid_label:?}"
    );
}

#[test]
fn parse_cdxml_uses_chemdraw_color_table_offset() {
    let Some(cdxml) = read_optional_cdxml_fixture("color.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&cdxml, Some("color")).expect("color cdxml should parse");

    let shape_fills = cdxml_shape_fills_by_z(&document);
    assert_eq!(
        shape_fills,
        vec![
            "#000000", "#ff0000", "#ffff00", "#00ff00", "#ffffff", "#00ffff", "#0000ff", "#ff00ff",
            "#804040", "#008000", "#0000a0", "#808080",
        ]
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("color=\"4\""), "{exported}");
    assert!(
        exported.contains("<color r=\"1\" g=\"0\" b=\"0\"/>"),
        "{exported}"
    );

    let reimported =
        parse_cdxml_document(&exported, Some("color export")).expect("export should parse");
    assert_eq!(cdxml_shape_fills_by_z(&reimported), shape_fills);
}

#[test]
fn cdxml_electron_symbol_uses_chemdraw_top_anchor_and_color() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <graphic id="2" BoundingBox="285.19 130.29 285.19 141.94" Z="1" color="4" GraphicType="Symbol" SymbolType="Electron"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("electron symbol cdxml should load");
    let document = &engine.state().document;
    let symbol = document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("electron symbol should import");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("electron")
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("fill")
            .and_then(|value| value.as_str()),
        Some("#ff0000")
    );
    let [_, _, width, height] = symbol.payload.bbox.expect("symbol should have bbox");
    let center = [
        symbol.transform.translate[0] + width * 0.5,
        symbol.transform.translate[1] + height * 0.5,
    ];
    let expected_diameter = 11.65 * 2.0 / 9.0;
    assert!(
        (width - expected_diameter).abs() < 0.01,
        "electron diameter should follow ChemDraw's anchor height ratio, got width={width}"
    );
    assert!(
        (height - expected_diameter).abs() < 0.01,
        "electron diameter should follow ChemDraw's anchor height ratio, got height={height}"
    );
    assert!(
        (center[0] - 285.19).abs() < 0.01,
        "electron center x should use the CDXML anchor x, got {center:?}"
    );
    assert!(
        (center[1] - 130.29).abs() < 0.01,
        "electron center y should use the top of the CDXML anchor bbox, got {center:?}"
    );
    let rendered_diameter = render_document(document)
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                object_id, d, fill, ..
            } if object_id.as_deref() == Some(symbol.id.as_str()) && fill == "#ff0000" => {
                horizontal_path_span_at_y(d, center[1])
            }
            _ => None,
        })
        .expect("electron should render as a filled path");
    assert!(
        (rendered_diameter - expected_diameter).abs() < 0.01,
        "rendered electron diameter should match imported geometry, got {rendered_diameter}"
    );

    let exported = document_to_cdxml(document);
    assert!(exported.contains("SymbolType=\"Electron\""), "{exported}");
    assert!(exported.contains("color=\"4\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("electron export")).expect("export should parse");
    let reimported_symbol = reimported
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("exported electron should reimport");
    let [_, _, re_width, re_height] = reimported_symbol
        .payload
        .bbox
        .expect("reimported symbol should have bbox");
    assert!((re_width - expected_diameter).abs() < 0.01, "{re_width}");
    assert!((re_height - expected_diameter).abs() < 0.01, "{re_height}");
    let re_center = [
        reimported_symbol.transform.translate[0] + re_width * 0.5,
        reimported_symbol.transform.translate[1] + re_height * 0.5,
    ];
    assert!((re_center[0] - 285.19).abs() < 0.01, "{re_center:?}");
    assert!((re_center[1] - 130.29).abs() < 0.01, "{re_center:?}");
}

#[test]
fn cdxml_charge_symbols_use_first_bbox_point_as_center() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <graphic id="2" BoundingBox="94.25 102.47 86.75 102.47" Z="1" GraphicType="Symbol" SymbolType="Minus"/>
    <graphic id="3" BoundingBox="97.99 113.04 90.49 113.04" Z="2" GraphicType="Symbol" SymbolType="Plus"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("charge symbol cdxml should load");
    let document = &engine.state().document;
    assert_symbol_center(document, "minus", [94.25, 102.47]);
    assert_symbol_center(document, "plus", [97.99, 113.04]);

    let exported = document_to_cdxml(document);
    let reimported =
        parse_cdxml_document(&exported, Some("charge symbol export")).expect("export should parse");
    assert_symbol_center(&reimported, "minus", [94.25, 102.47]);
    assert_symbol_center(&reimported, "plus", [97.99, 113.04]);
}

#[test]
fn cdxml_represented_radical_symbol_does_not_double_count_node_radical() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10" Element="7" Radical="Doublet"/>
      <n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4"/>
      <graphic id="6" BoundingBox="10 13 2.5 13" GraphicType="Symbol" SymbolType="Electron">
        <represent attribute="Radical" object="3"/>
      </graphic>
    </fragment>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("radical cdxml should load");
    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "3")
        .expect("radical nitrogen should import");
    assert_eq!(
        node.meta
            .get("radicalCount")
            .and_then(|value| value.as_i64()),
        Some(1)
    );
    assert_eq!(node.num_hydrogens, 1);
    let attached = node
        .meta
        .get("attachedElectronSymbols")
        .and_then(|value| value.as_array())
        .expect("electron symbol should attach to the radical nitrogen");
    assert_eq!(
        attached
            .first()
            .and_then(|value| value.get("radicalDelta"))
            .and_then(|value| value.as_i64()),
        Some(0)
    );
}

#[test]
fn cdxml_represented_charge_symbol_roundtrips_without_accumulating_charge() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10" Element="6" Charge="-1" NumHydrogens="1">
        <t id="4" p="10 10" BoundingBox="5 5 15 15" InterpretChemically="yes" UTF8Text="CH"><s face="96">CH</s></t>
      </n>
      <n id="5" p="24.4 10"/>
      <b id="6" B="3" E="5"/>
    </fragment>
    <graphic id="7" BoundingBox="10 15 10 5" GraphicType="Symbol" SymbolType="CircleMinus">
      <represent attribute="Charge" object="3"/>
    </graphic>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("represented charge should import");
    let exported = document_to_cdxml(&engine.state().document);
    assert!(
        exported.contains("<represent attribute=\"Charge\" object="),
        "{exported}"
    );
    let mut reimported = Engine::new();
    reimported
        .load_cdxml_document(&exported)
        .expect("represented charge export should import");
    let fragment = reimported
        .state()
        .document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should survive");
    assert_eq!(
        fragment
            .nodes
            .iter()
            .find(|node| node.charge != 0)
            .map(|node| node.charge),
        Some(-1)
    );
}

#[test]
fn cdxml_represent_target_overrides_nearest_atom_across_fragments() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 30 20">
      <n id="3" p="10 10" Element="6"/>
      <n id="4" p="24.4 10" Element="6"/>
      <b id="5" B="3" E="4"/>
    </fragment>
    <fragment id="20" BoundingBox="50 0 80 20">
      <n id="21" p="60 10" Element="6" Charge="1"/>
      <n id="22" p="74.4 10" Element="6"/>
      <b id="23" B="21" E="22"/>
    </fragment>
    <graphic id="30" BoundingBox="31 10 23.5 10" GraphicType="Symbol" SymbolType="Plus">
      <represent attribute="Charge" object="21"/>
    </graphic>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("represented charge across fragments should import");
    let document = &engine.state().document;
    let relation = document
        .links
        .iter()
        .find(|relation| relation.kind == "atom-symbol")
        .expect("represented symbol should create an atom-symbol link");
    assert!(relation
        .endpoints
        .iter()
        .any(|endpoint| { endpoint.role == "atom" && endpoint.entity_id == "21" }));
    assert_eq!(
        relation
            .data
            .pointer("/source")
            .and_then(serde_json::Value::as_str),
        Some("cdxml-represent")
    );
    let symbol = document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should survive");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("attachedAtomId")
            .and_then(serde_json::Value::as_str),
        Some("21")
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("attachmentSource")
            .and_then(serde_json::Value::as_str),
        Some("source")
    );
    let nodes = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .collect::<Vec<_>>();
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == "4")
            .map(|node| node.charge),
        Some(0)
    );
    let represented = nodes
        .iter()
        .find(|node| node.id == "21")
        .expect("represented atom should survive");
    assert_eq!(represented.charge, 1);
    assert!(represented.meta.get("effectiveNumHydrogens").is_none());
    assert_eq!(
        represented
            .meta
            .get("attachedElectronSymbols")
            .and_then(serde_json::Value::as_array)
            .and_then(|symbols| symbols.first())
            .and_then(|symbol| symbol.get("chargeDelta"))
            .and_then(serde_json::Value::as_i64),
        Some(0)
    );

    let exported = document_to_cdxml(document);
    assert!(
        exported.contains("<represent attribute=\"Charge\" object=\"21\""),
        "{exported}"
    );
    let reimported =
        parse_cdxml_document(&exported, Some("represented cross-fragment charge export"))
            .expect("represented charge export should import");
    let charges = reimported
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter().map(|node| node.charge))
        .filter(|charge| *charge != 0)
        .collect::<Vec<_>>();
    assert_eq!(charges, vec![1]);
}

#[test]
fn imported_unrepresented_symbol_does_not_infer_chemistry_from_proximity() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2">
      <n id="3" p="10 10"/>
      <n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4"/>
    </fragment>
    <graphic id="6" BoundingBox="31.9 10 24.4 10"
      GraphicType="Symbol" SymbolType="Plus"/>
  </page>
</CDXML>"##;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("unrepresented symbol should import");
    let document = &engine.state().document;
    assert!(
        document
            .links
            .iter()
            .all(|relation| relation.kind != "atom-symbol"),
        "source proximity alone must not create an atom-symbol relation"
    );
    let symbol = document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should survive");
    assert!(symbol.payload.extra.get("attachedAtomId").is_none());
    assert!(document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .all(|node| node.charge == 0));
}

#[test]
fn cdxml_element_list_query_roundtrips_without_becoming_a_nickname() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10" NodeType="ElementList" ElementList="6 7 15">
        <t id="4" p="10 10" BoundingBox="5 5 25 15" UTF8Text="[C,N,P]"><s face="96">[C,N,P]</s></t>
      </n>
      <n id="5" p="24.4 10"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("element list")).expect("query should import");
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("NodeType=\"ElementList\""), "{exported}");
    assert!(exported.contains("ElementList=\"6 7 15\""), "{exported}");
    let reimported = parse_cdxml_document(&exported, Some("element list export"))
        .expect("query should reimport");
    let query_node = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.label.is_some()))
        .expect("query node should survive");
    assert!(!query_node.is_placeholder);
    assert_eq!(
        query_node
            .meta
            .pointer("/import/cdxml/elementList")
            .and_then(|value| value.as_str()),
        Some("6 7 15")
    );
}

#[test]
fn cdxml_left_dashed_double_bond_preserves_which_line_is_dashed() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" color="0" bgcolor="1">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 40 20">
      <n id="3" p="10 10"/><n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4" Order="2" DoublePosition="Left" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("left dashed double")).expect("bond imports");
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Display2=\"Dash\""), "{exported}");
    assert!(!exported.contains(" Display=\"Dash\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("left dashed double export")).expect("bond reimports");
    let bond = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.bonds.first())
        .expect("bond survives");
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Solid
    );
    assert_eq!(
        bond.line_styles.left,
        chemsema_engine::BondLinePattern::Dashed
    );
}

#[test]
fn parse_cdxml_color_table_keeps_duplicate_slots() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <graphic id="2" BoundingBox="20 20 40 40" Z="1" color="5" GraphicType="Oval" OvalType="Circle Filled" Center3D="30 30 0" MajorAxisEnd3D="40 30 0" MinorAxisEnd3D="30 40 0"/>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("duplicate colors")).expect("cdxml should parse");

    assert_eq!(cdxml_shape_fills_by_z(&document), vec!["#ff0000"]);
}

#[test]
fn parse_cdxml_old_circle_uses_ordered_graphic_bounding_box_as_radius_and_center() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.5">
  <page id="1">
    <graphic id="2" BoundingBox="98.5875 87.6 95.2875 87.6"
      GraphicType="Oval" OvalType="Circle"/>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("legacy circle")).expect("CDXML");
    let shape = document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("legacy circle should import");

    assert_eq!(shape.payload.extra.get("kind"), Some(&json!("circle")));
    assert_eq!(
        shape.payload.extra.get("center"),
        Some(&json!([95.29, 87.6]))
    );
    assert_eq!(
        shape.payload.extra.get("majorAxisEnd"),
        Some(&json!([98.59, 87.6]))
    );
    assert_eq!(
        shape.payload.extra.get("minorAxisEnd"),
        Some(&json!([95.29, 90.9]))
    );
}

#[test]
fn parse_cdxml_infers_benzene_double_bond_sides_and_bond_colors() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="2" BondCircularOrdering="b2 0 0 b6"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4" Order="2" color="4" BondCircularOrdering="b4 0 0 b2"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6" Order="2" BondCircularOrdering="b6 0 0 b4"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("benzene")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");

    for bond_id in ["b1", "b3", "b5"] {
        let bond = fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .expect("benzene double bond should import");
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(chemsema_engine::DoubleBondPlacement::Left),
            "{bond_id} should infer an inward side double placement"
        );
    }
    assert_eq!(
        fragment
            .bonds
            .iter()
            .find(|bond| bond.id == "b3")
            .and_then(|bond| bond.stroke.as_deref()),
        Some("#ff0000")
    );

    let primitives = render_document(&document);
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Line {
                role: RenderRole::DocumentBond,
                bond_id: Some(id),
                stroke,
                ..
            } | RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(id),
                stroke,
                ..
            } if id == "b3" && stroke == "#ff0000"
        )),
        "colored CDXML bond should render with its imported stroke"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("color=\"4\""), "{exported}");
    assert!(exported.contains("DoublePosition=\"Left\""), "{exported}");
    let reimported =
        parse_cdxml_document(&exported, Some("benzene export")).expect("export should parse");
    let reimported_fragment = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reimported fragment should exist");
    assert!(reimported_fragment
        .bonds
        .iter()
        .any(|bond| bond.stroke.as_deref() == Some("#ff0000")));
}

#[test]
fn parse_cdxml_auto_dashed_double_bond_uses_ring_inside_side() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="1.5" Display2="Dash"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4" Order="2"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6" Order="2"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("auto dashed ring double")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");
    let double = bond
        .double
        .as_ref()
        .expect("auto dashed double should import as a double bond");

    assert_eq!(double.placement, chemsema_engine::DoubleBondPlacement::Left);
    assert!(
        !double.frozen,
        "no explicit DoublePosition means placement should remain auto"
    );
    assert_eq!(
        bond.line_styles.main,
        chemsema_engine::BondLinePattern::Solid
    );
    assert_eq!(
        bond.line_styles.left,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        bond.line_styles.right,
        chemsema_engine::BondLinePattern::Solid
    );
}

#[test]
fn parse_cdxml_auto_ring_double_prioritizes_ring_side_over_neighbor_double() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60">
  <page id="1">
    <fragment id="2" BoundingBox="7 10 33 39">
      <n id="n1" p="20.00 10.00"/>
      <n id="n2" p="32.47 17.20"/>
      <n id="n3" p="32.47 31.60"/>
      <n id="n4" p="20.00 38.80"/>
      <n id="n5" p="7.53 31.60"/>
      <n id="n6" p="7.53 17.20"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
      <b id="b3" B="n3" E="n4"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n6"/>
      <b id="b6" B="n6" E="n1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("ring priority")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Left),
        "ring membership should choose the inward side before non-ring attachment coverage"
    );
}

#[test]
fn parse_cdxml_auto_side_double_ignores_coordinate_rounding_in_a_full_endpoint_tie() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" BondSpacing="12" LineWidth="1" BoldWidth="4">
  <page id="1"><fragment id="2">
    <n id="1" p="254.63 397.50" Element="9" NumHydrogens="0"/>
    <n id="2" p="254.63 367.50"/>
    <n id="3" p="228.65 352.50" Element="53" NumHydrogens="0"/>
    <n id="4" p="280.61 352.50"/>
    <n id="5" p="306.60 367.50" Element="17" NumHydrogens="0"/>
    <n id="6" p="280.61 322.50" Element="35" NumHydrogens="0"/>
    <b id="8" B="1" E="2"/>
    <b id="9" B="2" E="3"/>
    <b id="10" B="2" E="4" Order="2" BS="Z" BondCircularOrdering="9 8 11 12"/>
    <b id="11" B="4" E="5"/>
    <b id="12" B="4" E="6"/>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("rounded full endpoint tie"))
        .expect("rounded full endpoint tie should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "10");
    let double = bond.double.as_ref().expect("double bond should import");

    assert_eq!(
        double.placement,
        chemsema_engine::DoubleBondPlacement::Right,
        "ChemDraw counts effective endpoint coverage; a 0.01 pt coordinate rounding delta must not choose the other side"
    );
    assert!(!double.frozen);
    assert_eq!(
        bond.properties.absolute_stereo,
        chemsema_engine::BondAbsoluteStereo::Z
    );
}

#[test]
fn parse_cdxml_auto_side_double_uses_effective_endpoint_coverage_and_fixed_collinear_cutoff() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" BondSpacing="12" LineWidth="1" BoldWidth="4">
  <page id="1">
    <fragment id="left_coverage">
      <n id="lc_b" p="100 100"/><n id="lc_e" p="160 100"/>
      <n id="lc_bl" p="74.0192 115"/><n id="lc_el" p="185.9808 115"/>
      <b id="left_coverage_double" B="lc_b" E="lc_e" Order="2"/>
      <b id="lc_1" B="lc_b" E="lc_bl"/><b id="lc_2" B="lc_e" E="lc_el"/>
    </fragment>
    <fragment id="trans_tie">
      <n id="tt_b" p="100 160"/><n id="tt_e" p="160 160"/>
      <n id="tt_bu" p="74.0192 145"/><n id="tt_el" p="185.9808 175"/>
      <b id="trans_tie_double" B="tt_b" E="tt_e" Order="2"/>
      <b id="tt_1" B="tt_b" E="tt_bu"/><b id="tt_2" B="tt_e" E="tt_el"/>
    </fragment>
    <fragment id="acute_default_side">
      <n id="ad_b" p="100 220"/><n id="ad_e" p="160 220"/>
      <n id="ad_bu" p="70.6556 213.7627"/><n id="ad_eu" p="189.3444 213.7627"/>
      <n id="ad_bl" p="74.0192 235"/><n id="ad_el" p="185.9808 235"/>
      <b id="acute_default_double" B="ad_b" E="ad_e" Order="2"/>
      <b id="ad_1" B="ad_b" E="ad_bu"/><b id="ad_2" B="ad_e" E="ad_eu"/>
      <b id="ad_3" B="ad_b" E="ad_bl"/><b id="ad_4" B="ad_e" E="ad_el"/>
    </fragment>
    <fragment id="valid_default_side">
      <n id="vd_b" p="100 280"/><n id="vd_e" p="160 280"/>
      <n id="vd_bu" p="70.888 272.751"/><n id="vd_eu" p="189.112 272.751"/>
      <n id="vd_bl" p="74.0192 295"/><n id="vd_el" p="185.9808 295"/>
      <b id="valid_default_double" B="vd_b" E="vd_e" Order="2"/>
      <b id="vd_1" B="vd_b" E="vd_bu"/><b id="vd_2" B="vd_e" E="vd_eu"/>
      <b id="vd_3" B="vd_b" E="vd_bl"/><b id="vd_4" B="vd_e" E="vd_el"/>
    </fragment>
    <fragment id="only_collinear">
      <n id="oc_b" p="100 340"/><n id="oc_e" p="160 340"/>
      <n id="oc_bu" p="70.6556 333.7627"/>
      <b id="only_collinear_double" B="oc_b" E="oc_e" Order="2"/>
      <b id="oc_1" B="oc_b" E="oc_bu"/>
    </fragment>
    <fragment id="bent_conjugated">
      <n id="bc_b" p="100 400"/><n id="bc_e" p="160 400"/>
      <n id="bc_bl" p="74.0192 415"/><n id="bc_el" p="185.9808 415"/>
      <b id="bent_conjugated_double" B="bc_b" E="bc_e" Order="2"/>
      <b id="bc_1" B="bc_b" E="bc_bl" Order="2"/>
      <b id="bc_2" B="bc_e" E="bc_el" Order="2"/>
    </fragment>
    <fragment id="linear_cumulene">
      <n id="cm_b" p="100 460"/><n id="cm_e" p="160 460"/>
      <n id="cm_bl" p="40 460"/><n id="cm_er" p="220 460"/>
      <b id="linear_cumulene_double" B="cm_b" E="cm_e" Order="2"/>
      <b id="cm_1" B="cm_b" E="cm_bl" Order="2"/>
      <b id="cm_2" B="cm_e" E="cm_er" Order="2"/>
    </fragment>
    <fragment id="substituted_linear_cumulene">
      <n id="sl_b" p="100 520"/><n id="sl_e" p="160 520"/>
      <n id="sl_neighbor" p="190 520"/><n id="sl_branch" p="74.0192 535"/>
      <b id="substituted_linear_double" B="sl_b" E="sl_e" Order="2"/>
      <b id="sl_1" B="sl_e" E="sl_neighbor" Order="2"/>
      <b id="sl_2" B="sl_b" E="sl_branch"/>
    </fragment>
    <fragment id="substituted_ten_degree_cumulene">
      <n id="st_b" p="100 580"/><n id="st_e" p="160 580"/>
      <n id="st_neighbor" p="189.5442 585.2094"/><n id="st_branch" p="74.0192 595"/>
      <b id="substituted_ten_degree_double" B="st_b" E="st_e" Order="2"/>
      <b id="st_1" B="st_e" E="st_neighbor" Order="2"/>
      <b id="st_2" B="st_b" E="st_branch"/>
    </fragment>
    <fragment id="substituted_fourteen_degree_conjugated">
      <n id="sf_b" p="100 640"/><n id="sf_e" p="160 640"/>
      <n id="sf_neighbor" p="189.1089 647.2577"/><n id="sf_branch" p="74.0192 655"/>
      <b id="substituted_fourteen_degree_double" B="sf_b" E="sf_e" Order="2"/>
      <b id="sf_1" B="sf_e" E="sf_neighbor" Order="2"/>
      <b id="sf_2" B="sf_b" E="sf_branch"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("automatic side coverage matrix"))
        .expect("automatic side coverage matrix should parse");
    let placement = |bond_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.bonds.iter())
            .find(|bond| bond.id == bond_id)
            .and_then(|bond| bond.double.as_ref())
            .map(|double| double.placement)
            .expect("probe double bond should import")
    };

    assert_eq!(
        placement("left_coverage_double"),
        chemsema_engine::DoubleBondPlacement::Left
    );
    assert_eq!(
        placement("trans_tie_double"),
        chemsema_engine::DoubleBondPlacement::Right
    );
    assert_eq!(
        placement("acute_default_double"),
        chemsema_engine::DoubleBondPlacement::Left,
        "12 degree attachments are effectively collinear and do not cover the default side"
    );
    assert_eq!(
        placement("valid_default_double"),
        chemsema_engine::DoubleBondPlacement::Right,
        "14 degree attachments cover the default side"
    );
    assert_eq!(
        placement("only_collinear_double"),
        chemsema_engine::DoubleBondPlacement::Center,
        "a segment with no effective side attachment is centered"
    );
    assert_eq!(
        placement("bent_conjugated_double"),
        chemsema_engine::DoubleBondPlacement::Left,
        "bent adjacent double bonds participate in the same endpoint-coverage rule"
    );
    assert_eq!(
        placement("linear_cumulene_double"),
        chemsema_engine::DoubleBondPlacement::Center,
        "a linear cumulene centers because both adjacent bonds are effectively collinear"
    );
    assert_eq!(
        placement("substituted_linear_double"),
        chemsema_engine::DoubleBondPlacement::Center,
        "a collinear neighboring double centers the cumulene even with an angled terminal single bond"
    );
    assert_eq!(
        placement("substituted_ten_degree_double"),
        chemsema_engine::DoubleBondPlacement::Center,
        "ChemDraw keeps the dedicated cumulene branch through ten degrees"
    );
    assert_eq!(
        placement("substituted_fourteen_degree_double"),
        chemsema_engine::DoubleBondPlacement::Left,
        "a clearly bent neighboring double resumes ordinary endpoint-side coverage"
    );
}

#[test]
fn cdxml_export_import_preserves_non_white_page_background() {
    let document = parse_document_json(
        &json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "red background",
                "page": { "width": 120.0, "height": 80.0, "background": "#ff0000" }
            },
            "styles": {},
            "objects": [],
            "resources": {}
        })
        .to_string(),
    )
    .expect("document json should parse");

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("bgcolor=\"4\""), "{exported}");

    let reimported =
        parse_cdxml_document(&exported, Some("red background")).expect("export should parse");
    assert_eq!(reimported.document.page.background, "#ff0000");
}

#[test]
fn parse_cdxml_right_side_double_bonds_render_on_begin_to_end_right_side() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 40 50">
  <page id="p1" BoundingBox="0 0 40 50">
    <fragment id="f1" BoundingBox="0 0 40 50">
      <n id="n1" p="10 10"/>
      <n id="n2" p="10 30"/>
      <b id="b1" B="n1" E="n2" Order="2" DoublePosition="Right"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("right side")).expect("cdxml should parse");
    let object = document
        .objects
        .iter()
        .find(|object| object.id == "obj_mol_001")
        .expect("fragment should import as obj_mol_001");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should have resourceRef");
    let fragment = document
        .resources
        .get(resource_ref)
        .expect("molecule resource should exist")
        .data
        .as_fragment()
        .expect("molecule resource should have fragment data");

    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b1")
        .expect("double bond should import");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Right)
    );

    let primitives = render_document(&document);
    let centerlines = object_bond_centerlines_with_ids(&primitives, "obj_mol_001");
    let begin = chemsema_engine::Point::new(10.0, 10.0);
    let end = chemsema_engine::Point::new(10.0, 30.0);
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    let right_normal = chemsema_engine::Point::new(dy / length, -dx / length);
    let raw_mid = chemsema_engine::Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
    let max_rendered_projection = centerlines
        .iter()
        .filter(|(id, _, _)| id == "b1")
        .map(|(_, from, to)| {
            let mid = chemsema_engine::Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5);
            (mid.x - raw_mid.x) * right_normal.x + (mid.y - raw_mid.y) * right_normal.y
        })
        .max_by(|a, b| a.total_cmp(b))
        .expect("double bond should render centerlines");
    assert!(
        max_rendered_projection > 0.0,
        "outer line should render on B->E right side, got {max_rendered_projection}"
    );
}
