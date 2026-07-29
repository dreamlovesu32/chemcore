use super::*;

#[test]
fn parse_cdxml_unspecified_alkene_double_bond_uses_automatic_side_placement() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 120 80">
    <fragment id="f1" BoundingBox="10 10 80 50">
      <n id="n1" p="10 40"/>
      <n id="n2" p="24.4 40"/>
      <n id="n3" p="38.8 40"/>
      <n id="n4" p="24.4 26"/>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n2" E="n3" Order="2"/>
      <b id="b3" B="n2" E="n4"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("alkene")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b2")
        .expect("alkene double bond should import");

    let double = bond
        .double
        .as_ref()
        .expect("double bond state should be inferred");
    assert_ne!(
        double.placement,
        chemsema_engine::DoubleBondPlacement::Center
    );
    assert!(!double.frozen);
}

#[test]
fn parse_cdxml_auto_double_bond_matches_chemdraw_center_and_tie_rules() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 140 70">
  <page id="p1" BoundingBox="0 0 140 70">
    <fragment id="f1" BoundingBox="0 0 60 60">
      <n id="n1" p="30 25"/>
      <n id="n2" p="30 45"/>
      <n id="n3" p="12.68 15"/>
      <n id="n4" p="47.32 15"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n1" E="n3"/>
      <b id="b3" B="n1" E="n4"/>
    </fragment>
    <fragment id="f2" BoundingBox="60 0 140 60">
      <n id="m1" p="80 30"/>
      <n id="m2" p="100 30"/>
      <n id="m3" p="62.68 20"/>
      <n id="m4" p="117.32 40"/>
      <b id="m5" B="m1" E="m2" Order="2"/>
      <b id="m6" B="m1" E="m3"/>
      <b id="m7" B="m2" E="m4"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("auto double")).expect("cdxml should parse");
    let mut placements = Vec::new();
    for fragment in document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
    {
        for bond in &fragment.bonds {
            if bond.order == 2 {
                placements.push((
                    bond.id.as_str(),
                    bond.double.as_ref().map(|double| double.placement),
                    bond.double.as_ref().map(|double| double.frozen),
                ));
            }
        }
    }
    assert!(
        placements.contains(&(
            "b1",
            Some(chemsema_engine::DoubleBondPlacement::Center),
            Some(false)
        )),
        "{placements:?}"
    );
    assert!(
        placements.contains(&(
            "m5",
            Some(chemsema_engine::DoubleBondPlacement::Right),
            Some(false)
        )),
        "{placements:?}"
    );
}

#[test]
fn parse_cdxml_auto_double_bond_places_five_member_ring_inside() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="20.00" BondSpacing="18" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" LabelSize="10" BoundingBox="0 0 50 60">
  <page id="p1" BoundingBox="0 0 50 60">
    <fragment id="f1" BoundingBox="0 0 50 60">
      <n id="n1" p="10 20"/>
      <n id="n2" p="24.4 20"/>
      <n id="n3" p="31.25 33.65"/>
      <n id="n4" p="17.2 44"/>
      <n id="n5" p="3.15 33.65"/>
      <b id="b1" B="n1" E="n2" Order="2"/>
      <b id="b2" B="n2" E="n3"/>
      <b id="b3" B="n3" E="n4"/>
      <b id="b4" B="n4" E="n5"/>
      <b id="b5" B="n5" E="n1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("cyclopentene")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b1")
        .expect("ring double bond should import");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Left)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn parse_cdxml_auto_double_bond_prefers_alternating_ring_over_short_fused_cycle() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="40 240 90 305">
    <fragment id="f1" BoundingBox="40 240 90 305">
      <n id="a" p="62.90 267.82"/>
      <n id="b" p="76.43 272.76"/>
      <n id="c" p="51.87 277.07"/>
      <n id="d" p="54.35 291.25"/>
      <n id="e" p="67.88 296.19"/>
      <n id="f" p="78.92 286.94"/>
      <n id="g" p="63.42 253.43"/>
      <n id="h" p="77.27 249.48"/>
      <n id="i" p="85.31 261.42"/>
      <b id="target" B="a" E="b" Order="2"/>
      <b id="outer1" B="a" E="c"/>
      <b id="outer2" B="c" E="d" Order="2"/>
      <b id="outer3" B="d" E="e"/>
      <b id="outer4" B="e" E="f" Order="2"/>
      <b id="outer5" B="f" E="b"/>
      <b id="short1" B="a" E="g"/>
      <b id="short2" B="g" E="h"/>
      <b id="short3" B="h" E="i"/>
      <b id="short4" B="i" E="b"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("fused alternating")).expect("cdxml should parse");
    let fragment = document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "target")
        .expect("target double bond should import");

    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemsema_engine::DoubleBondPlacement::Left)
    );
}

#[test]
fn parse_cdxml_terminal_label_tracks_asymmetric_double_bond_centerline() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" LabelSize="10"
 LabelFont="3" LabelFace="96">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="p1" BoundingBox="0 0 180 120">
    <fragment id="right">
      <n id="right_o" p="20 20" Element="8" NumHydrogens="0">
        <t p="16.75 23.52"><s font="3" size="10" face="96">O</s></t>
      </n>
      <n id="right_c" p="33.82 15.97"/>
      <b id="right_b" B="right_o" E="right_c" Order="2" DoublePosition="Right"/>
    </fragment>
    <fragment id="left">
      <n id="left_o" p="70 20" Element="8" NumHydrogens="0">
        <t p="66.75 23.52"><s font="3" size="10" face="96">O</s></t>
      </n>
      <n id="left_c" p="83.82 15.97"/>
      <b id="left_b" B="left_o" E="left_c" Order="2" DoublePosition="Left"/>
    </fragment>
    <fragment id="reversed">
      <n id="reverse_o" p="120 20" Element="8" NumHydrogens="0">
        <t p="116.75 23.52"><s font="3" size="10" face="96">O</s></t>
      </n>
      <n id="reverse_c" p="133.82 15.97"/>
      <b id="reverse_b" B="reverse_c" E="reverse_o" Order="2" DoublePosition="Right"/>
    </fragment>
    <fragment id="degree2">
      <n id="degree2_o" p="20 80" Element="8" NumHydrogens="0">
        <t p="16.75 83.52"><s font="3" size="10" face="96">O</s></t>
      </n>
      <n id="degree2_c" p="33.82 75.97"/>
      <n id="degree2_other" p="9.82 69.82"/>
      <b id="degree2_b" B="degree2_o" E="degree2_c" Order="2" DoublePosition="Right"/>
      <b id="degree2_single" B="degree2_o" E="degree2_other"/>
    </fragment>
    <fragment id="automatic">
      <n id="auto_o" p="70 80" Element="8" NumHydrogens="0">
        <t p="66.75 83.52"><s font="3" size="10" face="96">O</s></t>
      </n>
      <n id="auto_c" p="83.82 75.97"/>
      <n id="auto_next" p="87.55 62.06"/>
      <b id="auto_b" B="auto_o" E="auto_c" Order="2"/>
      <b id="auto_single" B="auto_c" E="auto_next"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("asymmetric terminal labels"))
        .expect("cdxml should parse");
    let label_offset = |node_id: &str| {
        let node = document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .unwrap_or_else(|| panic!("missing node {node_id}"));
        let position = node
            .label
            .as_ref()
            .and_then(|label| label.position)
            .unwrap_or_else(|| panic!("missing label position for {node_id}"));
        [
            position[0] - node.position[0],
            position[1] - node.position[1],
        ]
    };

    for (node_id, expected) in [
        ("right_o", [-4.25, 2.66]),
        ("left_o", [-3.53, 5.14]),
        ("reverse_o", [-3.53, 5.14]),
        ("degree2_o", [-3.89, 3.90]),
        ("auto_o", [-4.25, 2.66]),
    ] {
        let actual = label_offset(node_id);
        assert!(
            (actual[0] - expected[0]).abs() < 0.04 && (actual[1] - expected[1]).abs() < 0.04,
            "{node_id}: expected={expected:?}, actual={actual:?}"
        );
    }
}

#[test]
fn parse_cdxml_attached_atom_label_rebuilds_active_bbox_from_glyph_metrics() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("atom label bbox")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| {
            resource
                .data
                .as_fragment()
                .and_then(|fragment| fragment.nodes.iter().find(|node| node.atomic_number == 7))
        })
        .and_then(|node| node.label.as_ref())
        .expect("N label should import");
    let bbox = label.bbox().expect("N label should have an active bbox");
    let height = bbox[3] - bbox[1];
    assert!(
        (height - 8.9).abs() < 0.01,
        "attached CDXML labels should use the internal molecule-label line advance, got {bbox:?}"
    );
    assert_eq!(
        label.meta.pointer("/import/cdxml/boundingBox"),
        Some(&json!([6.4, 7.56, 13.62, 15.9])),
        "the original ChemDraw box should remain import evidence"
    );
    assert!(
        !label.glyph_polygons.is_empty(),
        "refresh should still populate glyph polygons for clipping"
    );

    let displaced_cdxml = cdxml
        .replace("p=\"6.40 15.90\"", "p=\"31.00 -17.00\"")
        .replace(
            "BoundingBox=\"6.40 7.56 13.62 15.90\"",
            "BoundingBox=\"-80 -60 140 190\"",
        );
    let displaced_document = parse_cdxml_document(&displaced_cdxml, Some("displaced atom label"))
        .expect("CDXML with displaced source geometry should parse");
    let displaced_label = displaced_document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("displaced N label should import");
    assert_eq!(
        displaced_label.position, label.position,
        "source text position must not affect active node-label layout"
    );
    assert_eq!(
        displaced_label.bbox(),
        label.bbox(),
        "source BoundingBox must not affect active node-label layout"
    );
    assert_eq!(
        displaced_label.glyph_polygons, label.glyph_polygons,
        "source text geometry must not affect active clipping geometry"
    );
}

#[test]
fn parse_cdxml_right_aligned_attached_labels_use_line_anchor_y() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="30.00" LineWidth="1.00" BoldWidth="4.00" HashSpacing="2.70" BondSpacing="12" MarginWidth="2.00" LabelSize="10">
  <page id="p1" BoundingBox="0 0 120 170">
    <fragment id="f1" BoundingBox="20 20 80 150">
      <n id="c1" p="70 40" AS="N"/>
      <n id="rprime" p="40 40" NodeType="GenericNickname" GenericNickname="R" NumHydrogens="0" AS="N">
        <t p="40.95 43.90" BoundingBox="31.82 35.56 40.95 43.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0">R&apos;</s>
        </t>
      </n>
      <b id="b1" B="c1" E="rprime"/>
      <n id="c2" p="70 90" AS="N"/>
      <n id="me" p="40 90" NodeType="GenericNickname" GenericNickname="Me" NumHydrogens="0" AS="N">
        <t p="42.78 93.90" BoundingBox="30.00 85.56 42.78 93.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0">Me</s>
        </t>
      </n>
      <b id="b2" B="c2" E="me"/>
      <n id="c3" p="70 140" AS="N"/>
      <n id="ar" p="40 140" NodeType="GenericNickname" GenericNickname="Ar" NumHydrogens="0" AS="N">
        <t p="41.67 143.90" BoundingBox="30.00 135.56 41.67 143.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0">Ar</s>
        </t>
      </n>
      <b id="b3" B="c3" E="ar"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("right attached labels")).expect("cdxml should parse");
    for node_id in ["rprime", "me", "ar"] {
        let node = document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .find_map(|fragment| fragment.nodes.iter().find(|node| node.id == node_id))
            .expect("node should import");
        let label = node.label.as_ref().expect("node label should import");
        let baseline = label.position.expect("label should have a baseline")[1];
        assert!(
            (baseline - node.position[1] - 3.9).abs() < 0.01,
            "{node_id} baseline should follow ChemDraw's line-anchor y, got node={:?} label={:?}",
            node.position,
            label.position
        );
    }
}

#[test]
fn render_cdxml_single_character_atom_label_uses_text_primitive() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.62 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("single atom label")).expect("cdxml should parse");
    let primitives = render_document(&document);
    let text = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                role,
                x,
                y,
                runs,
                text_anchor,
                ..
            } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText => {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .expect("N label should render as text");

    assert!((text.0 - 6.42).abs() < 0.001, "{text:?}");
    assert!((text.1 - 15.90).abs() < 0.001, "{text:?}");
    assert_eq!(
        text.2
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "N"
    );
    assert_eq!(text.3.as_deref(), Some("start"));
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            node_id,
            role,
            ..
        } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText
    )));
}

#[test]
fn parse_cdxml_node_label_keeps_face_style_independent_of_nonchemical_semantics() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="7">
        <t p="-20 40" BoundingBox="-80 -60 140 190" InterpretChemically="no">
          <s font="3" size="10" color="0" face="96">NH2</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("nonchemical label")).expect("CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.atomic_number == 7))
        .and_then(|node| node.label.as_ref())
        .expect("NH2 label should import");

    assert_eq!(label.text, "NH2");
    assert_eq!(label.meta.pointer("/defaultChemical"), Some(&json!(false)));
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/script"),
        Some(&json!("chemical"))
    );
    assert_eq!(label.runs[1].text, "2");
    assert_eq!(label.runs[1].script.as_deref(), Some("subscript"));
    let exported = document_to_cdxml(&document);
    assert!(
        exported.contains("InterpretChemically=\"no\""),
        "{exported}"
    );
    assert!(exported.contains("BoundingBox="), "{exported}");
}

#[test]
fn public_patent_ammonium_label_preserves_formula_count_below_explicit_charge() {
    // Reduced from the byte-identical public RDKit patent fixtures
    // US11707511-20230725-C00001.CDX and US20200222518A1-20200716-C00001.CDX.
    // Their CDX styled string authors NH3 with chemical face 96 and the
    // terminal + in a separate superscript face 64 run.
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LabelFont="3" LabelSize="10" LabelFace="96" InterpretChemically="yes">
  <fonttable><font id="3" charset="iso-8859-1" name="Times New Roman"/></fonttable>
  <page id="p1" BoundingBox="0 0 80 32">
    <fragment id="f1" BoundingBox="0 0 80 32">
      <n id="11" p="52 16" Element="7" Charge="1" NumHydrogens="3">
        <t p="52 20" BoundingBox="42 8 74 22" InterpretChemically="yes">
          <s font="3" size="10" face="96" color="0">NH3</s>
          <s font="3" size="10" face="64" color="0">+</s>
        </t>
      </n>
      <n id="12" p="20 16"/>
      <b id="14" B="11" E="12"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("public patent ammonium")).expect("CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "11"))
        .and_then(|node| node.label.as_ref())
        .expect("NH3+ label should import");

    let source_runs = label
        .meta
        .get("sourceRuns")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<chemsema_engine::LabelRun>>(value).ok())
        .expect("authored CDX/CDXML runs should survive import");
    assert_eq!(
        source_runs
            .iter()
            .map(|run| (run.text.as_str(), run.script.as_deref()))
            .collect::<Vec<_>>(),
        vec![("NH3", Some("chemical")), ("+", Some("superscript")),]
    );
    assert_eq!(
        label
            .runs
            .iter()
            .map(|run| (run.text.as_str(), run.script.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("NH", Some("normal")),
            ("3", Some("subscript")),
            ("+", Some("superscript")),
        ]
    );

    let svg = document_to_svg(&document);
    assert!(svg.contains(">3</tspan>"), "{svg}");
    assert!(svg.contains(">+</tspan>"), "{svg}");
    assert!(!svg.contains(">3+</tspan>"), "{svg}");
}

#[test]
fn parse_cdxml_node_label_preserves_explicit_regular_face_when_interpreted_chemically() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LabelFont="3" LabelSize="10" LabelFace="96" InterpretChemically="yes">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="p1" BoundingBox="0 0 80 32">
    <fragment id="f1" BoundingBox="0 0 80 32">
      <n id="n1" p="20 16" NodeType="Fragment">
        <t p="20 20" BoundingBox="10 8 32 22" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">NH2</s>
        </t>
      </n>
      <n id="n2" p="52 16"/>
      <b id="b1" B="n2" E="n1" EndAttach="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("regular chemical label")).expect("CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("NH2 label should import");

    assert_eq!(label.meta.pointer("/defaultChemical"), Some(&json!(true)));
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/script"),
        Some(&json!("normal"))
    );
    assert_eq!(label.runs.len(), 1);
    assert_eq!(label.runs[0].text, "NH2");
    assert_eq!(label.runs[0].script.as_deref(), Some("normal"));
}

#[test]
fn parse_normalized_cdxml_without_label_face_defaults_to_regular_face() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LabelFont="3" LabelSize="10" InterpretChemically="yes">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="p1" BoundingBox="0 0 80 32">
    <fragment id="f1" BoundingBox="0 0 80 32">
      <n id="n1" p="20 16" NodeType="Fragment">
        <t p="20 20" BoundingBox="10 8 32 22" InterpretChemically="yes">
          <s font="3" size="10" color="0">NH2</s>
        </t>
      </n>
      <n id="n2" p="52 16"/>
      <b id="b1" B="n2" E="n1" EndAttach="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("normalized regular label")).expect("CDXML parses");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("NH2 label should import");

    assert_eq!(label.meta.pointer("/defaultChemical"), Some(&json!(true)));
    assert_eq!(label.runs.len(), 1);
    assert_eq!(label.runs[0].text, "NH2");
    assert_eq!(label.runs[0].script.as_deref(), Some("normal"));
}

#[test]
fn parse_cdxml_node_label_subscripts_digits_across_style_run_boundaries() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" InterpretChemically="yes">
  <page id="p1" BoundingBox="0 0 80 32">
    <fragment id="f1" BoundingBox="0 0 80 32">
      <n id="pd" p="20 16">
        <t p="20 20" BoundingBox="20 8 72 22" InterpretChemically="yes">
          <s font="3" size="10" color="0" face="97">Pd</s>
          <s font="3" size="10" color="0" face="65">IV</s>
          <s font="3" size="10" color="0" face="97">(OCF</s>
          <s font="3" size="10" color="0" face="97">3</s>
          <s font="3" size="10" color="0" face="97">)n</s>
        </t>
      </n>
      <n id="c1" p="40 16"/>
      <b id="b1" B="pd" E="c1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("split formula run"))
        .expect("split formula CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
        .expect("Pd formula label should import");

    assert!(
        label
            .runs
            .iter()
            .any(|run| matches!(run.text.as_str(), "IV" | "VI")
                && run.script.as_deref() == Some("superscript")),
        "{:?}",
        label.runs
    );
    assert!(
        label
            .runs
            .iter()
            .any(|run| run.text == "3" && run.script.as_deref() == Some("subscript")),
        "{:?}",
        label.runs
    );
}

#[test]
fn parse_cdxml_preserves_document_drawing_defaults_without_using_cached_label_geometry() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML FractionalWidths="no" InterpretChemically="no" ShowTerminalCarbonLabels="yes" ShowNonTerminalCarbonLabels="yes" HideImplicitHydrogens="yes" LabelFont="4" LabelSize="11" LabelFace="98" CaptionFont="5" CaptionSize="9" CaptionFace="2" LineWidth="0.72" BoldWidth="3.20" BondLength="17.50" BondSpacing="21" HashSpacing="2.20" MarginWidth="1.60" ChainAngle="109.5" LabelJustification="Right" CaptionJustification="Center" PrintMargins="12 13 14 15" color="2">
  <fonttable>
    <font id="4" charset="iso-8859-1" name="Times New Roman"/>
    <font id="5" charset="iso-8859-1" name="Courier New"/>
  </fonttable>
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 50 24">
      <n id="n1" p="10 12" Element="7">
        <t p="-20 40" BoundingBox="-80 -60 140 190">
          <s color="0">NH2</s>
        </t>
      </n>
      <n id="n2" p="28 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
    <t id="txt1" p="60 14" BoundingBox="45 5 75 18">
      <s color="0">note</s>
    </t>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("defaults")).expect("CDXML should parse");
    let defaults = document
        .document
        .meta
        .pointer("/import/cdxml/defaults")
        .expect("CDXML defaults should be preserved");

    assert_eq!(defaults.get("chainAngle"), Some(&json!(109.5)));
    assert_eq!(
        defaults.pointer("/labelStyle/fontFamily"),
        Some(&json!("Times New Roman"))
    );
    assert_eq!(defaults.pointer("/labelStyle/fontSize"), Some(&json!(11.0)));
    assert_eq!(
        defaults.pointer("/labelStyle/fontStyle"),
        Some(&json!("italic"))
    );
    assert_eq!(
        defaults.pointer("/labelStyle/script"),
        Some(&json!("chemical"))
    );
    assert_eq!(
        defaults.pointer("/labelStyle/fill"),
        Some(&json!("#ffffff"))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/fontFamily"),
        Some(&json!("Courier New"))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/fontSize"),
        Some(&json!(9.0))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/fontStyle"),
        Some(&json!("italic"))
    );
    assert_eq!(
        defaults.pointer("/captionStyle/script"),
        Some(&json!("normal"))
    );
    assert_eq!(defaults.get("foregroundColor"), Some(&json!("#ffffff")));
    for opaque_key in [
        "labelFont",
        "labelFace",
        "captionFont",
        "captionFace",
        "color",
    ] {
        assert!(
            defaults.get(opaque_key).is_none(),
            "opaque {opaque_key} leaked into CCJS"
        );
    }
    assert_eq!(defaults.get("labelJustification"), Some(&json!("Right")));
    assert_eq!(defaults.get("captionJustification"), Some(&json!("Center")));
    assert_eq!(defaults.get("fractionalWidths"), Some(&json!(false)));
    assert_eq!(defaults.get("interpretChemically"), Some(&json!(false)));
    assert_eq!(defaults.get("showTerminalCarbonLabels"), Some(&json!(true)));
    assert_eq!(
        defaults.get("showNonTerminalCarbonLabels"),
        Some(&json!(true))
    );
    assert_eq!(defaults.get("hideImplicitHydrogens"), Some(&json!(true)));
    assert_eq!(
        defaults.get("printMargins"),
        Some(&json!([12.0, 13.0, 14.0, 15.0]))
    );
    assert_eq!(
        document.style.defaults.get("chainAngle").copied(),
        Some(109.5)
    );
    assert_eq!(document.style.label_style.font_family, "Times New Roman");
    assert_eq!(document.style.label_style.font_size, 11.0);
    assert_eq!(document.style.label_style.font_style, "italic");
    assert_eq!(document.style.label_style.script, "chemical");
    assert_eq!(document.style.caption_style.font_family, "Courier New");
    assert_eq!(document.style.caption_style.font_size, 9.0);

    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("node label should import");
    // ChemDraw keeps the document LabelFont as the editing default, but a
    // styled string whose first <s> omits font starts its own run state at
    // Arial instead of inheriting that document default.
    assert_eq!(label.font_family.as_deref(), Some("Arial"));
    assert_eq!(label.font_size, Some(11.0));
    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.meta.pointer("/defaultChemical"), Some(&json!(false)));
    for opaque_key in ["font", "face", "color"] {
        assert!(
            label
                .meta
                .pointer(&format!("/import/cdxml/{opaque_key}"))
                .is_none(),
            "opaque label {opaque_key} leaked into CCJS"
        );
    }
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/fontStyle"),
        Some(&json!("italic"))
    );
    assert_eq!(
        label.meta.pointer("/sourceRuns/0/script"),
        Some(&json!("chemical")),
        "InterpretChemically controls semantics without overriding inherited LabelFace"
    );
    assert_ne!(
        label.bbox(),
        Some([-80.0, -60.0, 140.0, 190.0]),
        "source BoundingBox must remain evidence, not active node-label geometry"
    );

    let text_object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("free text should import");
    let style = document
        .styles
        .get(text_object.style_ref.as_deref().expect("text style ref"))
        .expect("text style should exist");
    assert_eq!(style.get("fontFamily"), Some(&json!("Arial")));
    assert_eq!(style.get("fontSize"), Some(&json!(9.0)));
    assert_eq!(
        text_object.payload.extra.get("align"),
        Some(&json!("center"))
    );
    assert_eq!(
        text_object
            .payload
            .extra
            .get("runs")
            .and_then(|runs| runs.pointer("/0/fontStyle")),
        Some(&json!("italic"))
    );

    let exported = document_to_cdxml(&document);
    for expected in [
        "FractionalWidths=\"no\"",
        "InterpretChemically=\"no\"",
        "ShowTerminalCarbonLabels=\"yes\"",
        "ShowNonTerminalCarbonLabels=\"yes\"",
        "HideImplicitHydrogens=\"yes\"",
        "LabelFont=\"4\"",
        "LabelFace=\"98\"",
        "CaptionFont=\"5\"",
        "CaptionFace=\"2\"",
        "ChainAngle=\"109.5\"",
        "LabelJustification=\"Right\"",
        "CaptionJustification=\"Center\"",
        "PrintMargins=\"12 13 14 15\"",
        "color=\"2\"",
    ] {
        assert!(
            exported.contains(expected),
            "missing {expected} in {exported}"
        );
    }
}

#[test]
fn cdxml_centered_text_anchor_is_stable_after_first_save() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" CaptionFace="0">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 300 200" Width="300" Height="200">
    <t id="2" p="135.75 535.25" BoundingBox="113.59 526 157.92 655.25" Justification="Center" Z="1" UTF8Text="Acid"><s font="3" size="10" color="3" face="0">Acid</s></t>
  </page>
</CDXML>"#;
    let imported = parse_cdxml_document(cdxml, Some("centered-text")).expect("CDXML imports");
    let first = document_to_cdxml(&imported);
    let reimported = parse_cdxml_document(&first, Some("centered-text")).expect("export imports");
    let second = document_to_cdxml(&reimported);

    assert_eq!(
        second, first,
        "centered text must not drift after first save"
    );
}

#[test]
fn render_cdxml_imported_atom_label_uses_text_primitive() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 48 24">
    <fragment id="f1" BoundingBox="0 0 48 24">
      <n id="n1" p="10 12" Element="7">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 21.60 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="n2" p="30 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("multi atom label")).expect("cdxml");
    let primitives = render_document(&document);
    let text = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                role,
                runs,
                ..
            } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText => {
                Some(runs.clone())
            }
            _ => None,
        })
        .expect("HN label should render as text");

    assert_eq!(
        text.iter().map(|run| run.text.as_str()).collect::<String>(),
        "HN"
    );
    assert!(!primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            node_id,
            role,
            ..
        } if node_id.as_deref() == Some("n1") && *role == RenderRole::DocumentText
    )));
}

#[test]
fn parse_cdxml_right_aligned_chemical_node_label_reverses_visible_groups() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 44 24">
    <fragment id="f1" BoundingBox="0 0 44 24">
      <n id="n1" p="22 12" Element="6">
        <t p="22.00 15.90" BoundingBox="10.00 7.56 22.00 15.90" LabelJustification="Right">
          <s font="3" size="10" color="0" face="96">OCF3</s>
        </t>
      </n>
      <n id="n2" p="34 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("right aligned label")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("OCF3 label should import");

    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.source_text.as_deref(), Some("OCF3"));
    assert_eq!(label.text, "F3CO");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "F3CO");
    assert_eq!(
        label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("OCF3")
    );
}

#[test]
fn parse_cdxml_right_aligned_labels_reverse_groups_independent_of_validity() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 84">
    <fragment id="f1" BoundingBox="0 0 80 84">
      <n id="tfa" p="30 12" Element="6">
        <t p="30.00 15.90" BoundingBox="-2.00 7.56 30.00 15.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0" face="96">OTFA</s>
        </t>
      </n>
      <n id="tfa2" p="48 12"/>
      <b id="btfa" B="tfa" E="tfa2"/>
      <n id="xyz" p="30 40" Element="6">
        <t p="30.00 43.90" BoundingBox="-2.00 35.56 30.00 43.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0" face="96">OXYZ</s>
        </t>
      </n>
      <n id="xyz2" p="48 40"/>
      <b id="bxyz" B="xyz" E="xyz2"/>
      <n id="nme" p="30 68" Element="6">
        <t p="30.00 71.90" BoundingBox="-2.00 63.56 30.00 71.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="10" color="0" face="96">NMe4</s>
        </t>
      </n>
      <n id="nme2" p="48 68"/>
      <b id="bnme" B="nme" E="nme2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("right aligned display tokens"))
        .expect("cdxml should parse");
    let label_for = |node_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .unwrap_or_else(|| panic!("{node_id} label should import"))
    };

    let tfa = label_for("tfa");
    assert_eq!(tfa.source_text.as_deref(), Some("OTFA"));
    assert_eq!(tfa.text, "TFAO");
    assert_eq!(
        tfa.meta
            .pointer("/labelRecognition/status")
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );

    let xyz = label_for("xyz");
    assert_eq!(xyz.source_text.as_deref(), Some("OXYZ"));
    assert_eq!(xyz.text, "ZYXO");
    assert_eq!(
        xyz.meta
            .pointer("/labelRecognition/diagnostic")
            .and_then(serde_json::Value::as_str),
        Some("uninterpretable-label")
    );

    let nme = label_for("nme");
    assert_eq!(nme.source_text.as_deref(), Some("NMe4"));
    assert_eq!(nme.text, "Me4N");
    assert_eq!(
        nme.meta
            .pointer("/labelRecognition/diagnostic")
            .and_then(serde_json::Value::as_str),
        Some("invalid-valence")
    );
}

#[test]
fn parse_cdxml_attached_chemical_label_preserves_visible_spaces() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 100 36">
    <fragment id="f1" BoundingBox="0 0 100 36">
      <n id="n1" p="20 18"/>
      <n id="n2" p="34 18" NodeType="Nickname">
        <t p="34 22" BoundingBox="34 8 93 24" LabelJustification="Left" LabelAlignment="Left" InterpretChemically="yes" UTF8Text="MgBr CuI Bipy">
          <s font="3" size="10" color="0" face="96">MgBr CuI Bipy</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("spaced chemical label")).expect("cdxml");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n2"))
        .and_then(|node| node.label.as_ref())
        .expect("spaced label should import");

    assert_eq!(label.source_text.as_deref(), Some("MgBr CuI Bipy"));
    assert_eq!(label.text, "MgBr CuI Bipy");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "MgBr CuI Bipy");
    assert_eq!(
        label
            .meta
            .pointer("/sourceRuns/0/text")
            .and_then(serde_json::Value::as_str),
        Some("MgBr CuI Bipy")
    );

    let first_export = document_to_cdxml(&document);
    assert!(
        first_export.contains("UTF8Text=\"MgBr CuI Bipy\""),
        "export should keep visible/source spaces: {first_export}"
    );
    assert!(
        first_export.contains(">MgBr CuI Bipy</s>"),
        "exported text run should keep spaces: {first_export}"
    );
    let reimported = parse_cdxml_document(&first_export, Some("spaced chemical label"))
        .expect("reimport should parse");
    let second_export = document_to_cdxml(&reimported);
    assert_eq!(
        second_export, first_export,
        "CDXML spaced chemical label roundtrip should stabilize"
    );
}

#[test]
fn parse_cdxml_normal_face_attached_label_uses_connection_aware_group_layout() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 84 44">
    <fragment id="f1" BoundingBox="0 0 84 44">
      <n id="n1" p="20 12" NodeType="Fragment">
        <t p="16.40 15.90" BoundingBox="16.40 4.40 35.20 15.90" LabelJustification="Left" LabelAlignment="Above" UTF8Text="NTs">
          <s font="3" size="10" color="0" face="1">NTs</s>
        </t>
      </n>
      <n id="n2" p="8 24"/>
      <n id="n3" p="32 24"/>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n1" E="n3"/>
      <n id="n4" p="64 12" NodeType="Fragment">
        <t p="64.00 15.90" BoundingBox="45.20 7.56 64.00 15.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="NTs">
          <s font="3" size="10" color="0" face="1">NTs</s>
        </t>
      </n>
      <n id="n5" p="76 12"/>
      <n id="n6" p="60 24"/>
      <b id="b3" B="n4" E="n5"/>
      <b id="b4" B="n4" E="n6"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("normal face labels")).expect("cdxml should parse");
    let fragments: Vec<_> = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .collect();
    assert_eq!(fragments.len(), 2);
    let stacked = fragments
        .iter()
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "n1")
        .and_then(|node| node.label.as_ref())
        .expect("stacked NTs label should import");
    assert_eq!(stacked.text, "Ts\nN");
    assert_eq!(stacked.lines, vec!["Ts", "N"]);
    assert_eq!(
        stacked
            .meta
            .pointer("/sourceRuns/0/script")
            .and_then(serde_json::Value::as_str),
        Some("normal")
    );
    assert_eq!(
        stacked
            .meta
            .pointer("/labelRecognition/canonicalLabel")
            .and_then(serde_json::Value::as_str),
        Some("NTs")
    );

    let mixed_direction = fragments
        .iter()
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "n4")
        .and_then(|node| node.label.as_ref())
        .expect("mixed-direction NTs label should import");
    // ChemDraw applies the same two-connection bisector flow to a collapsed
    // Fragment label: the visible order reverses while the canonical label
    // below remains NTs.
    assert_eq!(mixed_direction.text, "TsN");
    assert_eq!(
        mixed_direction
            .meta
            .pointer("/sourceRuns/0/script")
            .and_then(serde_json::Value::as_str),
        Some("normal")
    );
    assert_eq!(
        mixed_direction
            .meta
            .pointer("/labelRecognition/components/1/label")
            .and_then(serde_json::Value::as_str),
        Some("Ts")
    );
}

#[test]
fn parse_cdxml_parenthesized_attached_label_reverses_inner_groups() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 92 44">
    <fragment id="f1" BoundingBox="0 0 92 44">
      <n id="n1" p="20 16" NodeType="Fragment">
        <t p="20.00 19.90" BoundingBox="-34.00 8.40 20.00 19.90" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="N(PhSO2)2">
          <s font="3" size="10" color="0" face="96">N(PhSO2)2</s>
        </t>
      </n>
      <n id="n2" p="36 16"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("parenthesized label")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("N(PhSO2)2 node should import");
    let label = node.label.as_ref().expect("N(PhSO2)2 label should import");

    assert_eq!(label.source_text.as_deref(), Some("N(PhSO2)2"));
    assert_eq!(label.text, "(O2SPh)2N");
    let display_text: String = label.runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(display_text, "(O2SPh)2N");
    assert_eq!(
        label
            .meta
            .pointer("/labelRecognition/canonicalLabel")
            .and_then(serde_json::Value::as_str),
        Some("N(PhSO2)2")
    );
    assert_eq!(
        label
            .meta
            .pointer("/labelRecognition/anchorAtom")
            .and_then(serde_json::Value::as_str),
        Some("N")
    );
    assert_eq!(label.align.as_deref(), Some("right"));
    assert_eq!(label.anchor.as_deref(), Some("end"));
}

#[test]
fn parse_cdxml_fixed_right_hydrocarbon_formula_preserves_order_and_subscripts() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 36">
    <fragment id="f1" BoundingBox="0 0 80 36">
      <n id="alkyl" p="32 18" NodeType="Nickname">
        <t p="32 22" BoundingBox="-8 8 32 24" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="C10H21">
          <s font="3" size="10" color="0" face="96">C10H21</s>
        </t>
      </n>
      <n id="c1" p="50 18"/>
      <b id="b1" B="alkyl" E="c1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("fixed right C10H21")).expect("cdxml");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "alkyl"))
        .and_then(|node| node.label.as_ref())
        .expect("C10H21 label should import");

    assert_eq!(label.text, "C10H21");
    assert_eq!(label.source_text.as_deref(), Some("C10H21"));
    let subscript_text: String = label
        .runs
        .iter()
        .filter(|run| run.script.as_deref() == Some("subscript"))
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(subscript_text, "1021");

    let first_export = document_to_cdxml(&document);
    for expected in [
        "LabelJustification=\"Right\"",
        "Justification=\"Right\"",
        "LabelAlignment=\"Right\"",
        ">C10H21</s>",
    ] {
        assert!(
            first_export.contains(expected),
            "missing {expected}: {first_export}"
        );
    }
    let reimported =
        parse_cdxml_document(&first_export, Some("fixed right C10H21")).expect("reimport");
    let second_export = document_to_cdxml(&reimported);
    assert_eq!(
        second_export, first_export,
        "CDXML open/save must stabilize after export"
    );
}

#[test]
fn parse_cdxml_auto_right_alignment_reverses_hydrocarbon_as_one_group() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="14.40" LineWidth="0.60" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 90 36">
    <fragment id="f1" BoundingBox="0 0 90 36">
      <n id="alkoxy" p="40 18" NodeType="Nickname">
        <t p="40 22" BoundingBox="-12 8 40 24" LabelJustification="Auto" Justification="Right" LabelAlignment="Right" UTF8Text="C10H21O3">
          <s font="3" size="10" color="0" face="96">C10H21O3</s>
        </t>
      </n>
      <n id="c1" p="58 18"/>
      <b id="b1" B="alkoxy" E="c1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("auto C10H21O3")).expect("cdxml");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "alkoxy"))
        .and_then(|node| node.label.as_ref())
        .expect("C10H21O3 label should import");

    assert_eq!(label.source_text.as_deref(), Some("C10H21O3"));
    assert_eq!(label.text, "O3C10H21");
    assert_ne!(label.text, "O3H21C10");
}

#[test]
fn parse_cdxml_centered_multichar_label_uses_internal_center_anchor() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" color="0" bgcolor="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="p1" BoundingBox="0 0 60 30">
    <fragment id="f1" BoundingBox="0 0 60 30">
      <n id="n1" p="10 12" NodeType="Fragment" LabelDisplay="Center">
        <t p="10 16" BoundingBox="-80 -60 140 190" LabelJustification="Center" Justification="Center" LabelAlignment="Right" UTF8Text="CF3">
          <s font="3" size="10" color="2" face="96">CF3</s>
        </t>
      </n>
      <n id="n2" p="26 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("centered CF3 label"))
        .expect("centered CF3 CDXML should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .expect("CF3 node should import");
    let label = node.label.as_ref().expect("CF3 label should import");
    let bbox = label.bbox().expect("CF3 label should have a rebuilt bbox");

    assert_eq!(label.text, "CF3");
    assert_eq!(label.source_text.as_deref(), Some("CF3"));
    assert_eq!(label.align.as_deref(), Some("center"));
    assert_eq!(label.anchor.as_deref(), Some("middle"));
    assert_eq!(label.layout.as_deref(), Some("attached-group-center"));
    assert!(
        ((bbox[0] + bbox[2]) * 0.5 - node.position[0]).abs() < 0.01,
        "CDXML LabelDisplay=Center must center the internally rebuilt label box on the node: bbox={bbox:?}, node={node:?}"
    );
    assert_ne!(
        bbox,
        [-80.0, -60.0, 140.0, 190.0],
        "source BoundingBox must remain provenance, not active label geometry"
    );
    assert_eq!(
        label.meta.pointer("/import/cdxml/labelDisplay"),
        Some(&json!("Center"))
    );
}

#[test]
fn inferred_centered_label_exports_center_alignment_without_becoming_right_aligned() {
    let cdxml = r#"<CDXML BondLength="14.4" LabelSize="10">
      <page id="1">
        <fragment id="2">
          <n id="3" p="30 20">
            <t p="30 20" Justification="Center"><s font="3" size="10">CH4</s></t>
          </n>
        </fragment>
      </page>
    </CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("inferred centered label")).expect("CDXML parses");
    let imported_label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.first())
        .and_then(|node| node.label.as_ref())
        .expect("label imports");
    assert_eq!(imported_label.align.as_deref(), Some("center"));
    assert_eq!(
        imported_label.layout.as_deref(),
        Some("attached-group-center")
    );
    assert_eq!(
        imported_label.meta.pointer("/import/cdxml/labelAlignment"),
        Some(&json!(null))
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("LabelAlignment=\"Center\""), "{exported}");
    assert!(!exported.contains("LabelAlignment=\"Right\""), "{exported}");
    let reopened =
        parse_cdxml_document(&exported, Some("reopened centered label")).expect("export reopens");
    let reopened_label = reopened
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.first())
        .and_then(|node| node.label.as_ref())
        .expect("label reopens");
    assert_eq!(reopened_label.align.as_deref(), Some("center"));
    assert_eq!(reopened_label.anchor.as_deref(), Some("middle"));
}

#[test]
fn parse_cdxml_inferred_centered_metal_label_uses_the_same_baseline_as_neighboring_atoms() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM="http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10" MarginWidth="1.6" color="0" bgcolor="1">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 80 40">
      <n id="left" p="16 20" Element="7" NumHydrogens="0">
        <t p="12.4 23.9" BoundingBox="12.4 15.7 19.6 24.6" LabelJustification="Left" UTF8Text="N">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <n id="metal" p="30.4 20" Element="46" NumHydrogens="0">
        <t p="30.4 23.9" BoundingBox="24.3 14.9 36.5 26.4" LabelJustification="Center" Justification="Center" LabelAlignment="Center" UTF8Text="Pd">
          <s font="3" size="10" color="0" face="96">Pd</s>
        </t>
      </n>
      <n id="right" p="44.8 20" Element="7" NumHydrogens="0">
        <t p="41.2 23.9" BoundingBox="41.2 15.7 48.4 24.6" LabelJustification="Left" UTF8Text="N">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="left" E="metal"/>
      <b id="b2" B="metal" E="right"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("inferred centered Pd label"))
        .expect("inferred centered Pd CDXML should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let label_for = |id: &str| {
        fragment
            .nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| node.label.as_ref())
            .expect("node label should import")
    };
    let left = label_for("left");
    let metal = label_for("metal");
    let right = label_for("right");
    let left_baseline = left.position.expect("left baseline")[1];
    let metal_baseline = metal.position.expect("metal baseline")[1];
    let right_baseline = right.position.expect("right baseline")[1];

    assert_eq!(metal.align.as_deref(), Some("center"));
    assert_eq!(metal.anchor.as_deref(), Some("middle"));
    assert_eq!(metal.layout.as_deref(), Some("attached-group-center"));
    assert_eq!(
        metal.meta.pointer("/import/cdxml/labelDisplay"),
        Some(&serde_json::Value::Null),
        "center alignment inferred from justification must not invent LabelDisplay"
    );
    assert_close(left_baseline, 23.9);
    assert_close(metal_baseline, 23.9);
    assert_close(right_baseline, 23.9);
}

#[test]
fn parse_cdxml_right_aligned_metal_oxidation_label_reverses_visible_order() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 90 60">
    <fragment id="f1" BoundingBox="0 0 90 60">
      <n id="cu" p="40 24" Element="29" NumHydrogens="0">
        <t p="42.00 27.90" BoundingBox="16.00 16.40 42.00 27.90" LabelAlignment="Right" LabelJustification="Unspecified">
          <s font="3" size="10" color="0" face="96">Cu(II)</s>
        </t>
      </n>
      <n id="n1" p="58 12" Element="7"/>
      <n id="n2" p="58 24" Element="7"/>
      <n id="n3" p="58 36" Element="7"/>
      <b id="b1" B="cu" E="n1"/>
      <b id="b2" B="cu" E="n2"/>
      <b id="b3" B="cu" E="n3"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("metal oxidation label")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let copper = fragment
        .nodes
        .iter()
        .find(|node| node.id == "cu")
        .expect("copper node should import");
    let label = copper.label.as_ref().expect("Cu(II) label should import");

    assert_eq!(label.source_text.as_deref(), Some("Cu(II)"));
    assert_eq!(label.text, "(II)Cu");
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/status")
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/source")
            .and_then(serde_json::Value::as_str),
        Some("element-oxidation-state-label")
    );
    assert_eq!(copper.num_hydrogens, 0);
}

#[test]
fn parse_cdxml_metal_containing_chemical_label_does_not_mark_invalid() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 80 40">
    <fragment id="f1" BoundingBox="0 0 80 40">
      <n id="cu" p="30 20" Element="29" NumHydrogens="0">
        <t p="30.00 23.90" BoundingBox="30.00 12.40 76.00 23.90" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">Cu(NO3)2</s>
        </t>
      </n>
      <n id="n1" p="54 20" Element="7"/>
      <b id="b1" B="cu" E="n1"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("metal salt label")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should import");
    let copper = fragment
        .nodes
        .iter()
        .find(|node| node.id == "cu")
        .expect("copper node should import");

    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/status")
            .and_then(serde_json::Value::as_str),
        Some("recognized")
    );
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/source")
            .and_then(serde_json::Value::as_str),
        Some("metal-containing-chemical-text")
    );
    assert_eq!(
        copper
            .meta
            .pointer("/labelRecognition/groupKind")
            .and_then(serde_json::Value::as_str),
        Some("chemical-text")
    );
    assert!(copper.meta.pointer("/labelRecognition/expansion").is_none());
}

#[test]
fn parse_cdxml_label_display_overrides_auto_reversal_without_losing_chemistry() {
    let cdxml = include_str!("../fixtures/label-display-modes.cdxml");
    let document = parse_cdxml_document(cdxml, Some("label display modes"))
        .expect("label display mode CDXML should parse");
    let mut labels: std::collections::BTreeMap<
        String,
        Vec<(&chemsema_engine::Node, &chemsema_engine::NodeLabel)>,
    > = std::collections::BTreeMap::new();
    for resource in document.resources.values() {
        if let Some(fragment) = resource.data.as_fragment() {
            for node in &fragment.nodes {
                if let Some(label) = &node.label {
                    if label.source_text.as_deref() == Some("CF3") {
                        let display = label
                            .meta
                            .pointer("/import/cdxml/labelDisplay")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Auto");
                        labels
                            .entry(display.to_string())
                            .or_default()
                            .push((node, label));
                    }
                }
            }
        }
    }

    let auto_label = labels["Auto"]
        .iter()
        .map(|(_, label)| *label)
        .find(|label| label.text == "F3C")
        .expect("a right-aligned CF3 without LabelDisplay should use chemical group reversal");
    assert_eq!(auto_label.source_text.as_deref(), Some("CF3"));

    for (_, right_label) in &labels["Right"] {
        assert_eq!(right_label.source_text.as_deref(), Some("CF3"));
        assert_eq!(right_label.text, "CF3");
        assert_eq!(right_label.align.as_deref(), Some("right"));
        assert_eq!(right_label.anchor.as_deref(), Some("end"));
    }

    let (center_node, center_label) = labels["Center"]
        .iter()
        .find(|(_, label)| label.text == "CF3")
        .copied()
        .expect("LabelDisplay=Center CF3 should import");
    let center_box = center_label.bbox().expect("center label box");
    assert_eq!(center_label.source_text.as_deref(), Some("CF3"));
    assert_eq!(center_label.text, "CF3");
    assert_eq!(center_label.align.as_deref(), Some("center"));
    assert_eq!(center_label.anchor.as_deref(), Some("middle"));
    assert!(
        ((center_box[0] + center_box[2]) * 0.5 - center_node.position[0]).abs() < 0.01,
        "LabelDisplay=Center should center the rebuilt label box on the node"
    );

    for (_, left_label) in &labels["Left"] {
        assert_eq!(left_label.source_text.as_deref(), Some("CF3"));
        assert_eq!(left_label.text, "CF3");
        assert_eq!(left_label.align.as_deref(), Some("left"));
        assert_eq!(left_label.anchor.as_deref(), Some("start"));
    }
}

#[test]
fn parse_cdxml_attached_sulfur_label_uses_elliptical_clip_geometry() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 40 24">
    <fragment id="f1" BoundingBox="0 0 40 24">
      <n id="n1" p="10 12" Element="16">
        <t p="6.40 15.90" BoundingBox="6.40 7.56 13.10 15.90" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">S</s>
        </t>
      </n>
      <n id="n2" p="24 12"/>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("sulfur ellipse clip")).expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "n1"))
        .and_then(|node| node.label.as_ref())
        .expect("S label should import");

    assert!(
        !label.glyph_polygons.is_empty(),
        "sulfur label should populate glyph polygons"
    );
    assert!(
        label
            .glyph_polygons
            .iter()
            .any(|polygon| polygon.len() >= 16),
        "sulfur clipping should include an ellipse-like polygon for S; text={:?}, polygons={:?}",
        label.text,
        label.glyph_polygons
    );
}

#[test]
fn parse_cdxml_double_bond_spacing_uses_bond_spacing_percent() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" BondSpacing="18" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="9 8 26 12">
      <n id="3" p="10 10"/>
      <n id="4" p="24.4 10"/>
      <b id="5" B="3" E="4" Order="2"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("spacing")).expect("cdxml should parse");
    let primitives = render_document(&document);
    let mut center_ys: Vec<f64> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_mol_001") =>
            {
                bond_axis_from_points(points).map(|(from, to)| (from.y + to.y) * 0.5)
            }
            _ => None,
        })
        .collect();
    center_ys.sort_by(f64::total_cmp);

    assert_eq!(center_ys.len(), 2, "{center_ys:?}");
    let center_distance = center_ys[1] - center_ys[0];
    let expected_center_distance = 14.4 * 0.18;
    assert!(
        (center_distance - expected_center_distance).abs() < 0.001,
        "{center_distance}"
    );
}

#[test]
fn parse_cdxml_node_labels_use_internal_attached_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 120 120">
      <n id="3" p="50 50" Element="7">
        <t id="30" p="0 0" BoundingBox="0 0 100 100" UTF8Text="NH">
          <s font="3" size="10" color="0">NH</s>
        </t>
      </n>
      <n id="4" p="42 65"/>
      <n id="5" p="58 65"/>
      <b id="6" B="3" E="4"/>
      <b id="7" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("labels")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "3")
        .expect("labeled node should import");
    let label = node.label.as_ref().expect("node label should import");

    assert_eq!(node.position, [50.0, 50.0]);
    assert_eq!(label.attachment.as_deref(), Some("node"));
    assert_eq!(label.anchor.as_deref(), Some("start"));
    assert_eq!(label.lines, vec!["H".to_string(), "N".to_string()]);
    assert_eq!(label.layout.as_deref(), Some("attached-group-above"));
    assert!(
        !label.glyph_polygons.is_empty(),
        "internal glyph geometry should be generated"
    );
    let glyph_center = |index: usize| {
        let polygon = label
            .glyph_polygons
            .get(index)
            .expect("expected glyph polygon");
        let bounds = polygon.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |mut bounds, point| {
                bounds[0] = bounds[0].min(point[0]);
                bounds[1] = bounds[1].min(point[1]);
                bounds[2] = bounds[2].max(point[0]);
                bounds[3] = bounds[3].max(point[1]);
                bounds
            },
        );
        Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
    };
    let hydrogen_center = glyph_center(0);
    let nitrogen_center = glyph_center(1);
    assert!(
        (nitrogen_center.x - node.position[0]).abs() < 0.01,
        "stacked NH labels should anchor the original first atom glyph horizontally to the node: H={hydrogen_center:?}, N={nitrogen_center:?}, node={:?}",
        node.position
    );
    assert!(
        hydrogen_center.y < nitrogen_center.y,
        "hydrogen should render above nitrogen for an above-stacked NH label"
    );
    let box_value = label.box_value.expect("internal label box should exist");
    assert!(
        box_value[2] - box_value[0] < 30.0 && box_value[3] - box_value[1] < 30.0,
        "{box_value:?}"
    );
    assert_ne!(box_value, [0.0, 0.0, 100.0, 100.0]);

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Element=\"7\""), "{exported}");
    assert!(exported.contains("NumHydrogens=\"1\""), "{exported}");
    assert!(exported.contains("LabelAlignment=\"Above\""), "{exported}");
    assert!(
        !exported.contains("LineStarts="),
        "directional NH layout is represented by LabelAlignment, not authored line breaks: {exported}"
    );
    assert!(exported.contains("BoundingBox="), "{exported}");
    assert!(
        exported.contains("InterpretChemically=\"yes\""),
        "{exported}"
    );
    assert!(exported.contains("LabelFace=\"0\""), "{exported}");
    assert!(!exported.contains("face=\"96\""), "{exported}");

    let reimported =
        parse_cdxml_document(&exported, Some("labels export")).expect("export should parse");
    let reimported_fragment = reimported
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("reimport should create molecule fragment resource");
    let reimported_node = reimported_fragment
        .nodes
        .iter()
        .find(|node| node.atomic_number == 7)
        .expect("nitrogen node should reimport as nitrogen");
    let reimported_label = reimported_node
        .label
        .as_ref()
        .expect("nitrogen label should reimport");
    assert_eq!(reimported_node.num_hydrogens, 1);
    assert_eq!(
        reimported_label.lines,
        vec!["H".to_string(), "N".to_string()]
    );
    assert_eq!(
        reimported_label.layout.as_deref(),
        Some("attached-group-above")
    );
}

#[test]
fn parse_cdxml_left_justification_does_not_override_connection_aware_label_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelJustification="Auto">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 180 80">
      <n id="right" p="20 40" Element="7" NumHydrogens="1">
        <t id="right_text" p="20 44" BoundingBox="14 32 26 46" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="right_up" p="34 32"/>
      <n id="right_down" p="34 48"/>
      <b id="right_bond_up" B="right" E="right_up"/>
      <b id="right_bond_down" B="right" E="right_down"/>

      <n id="below" p="80 24" Element="7" NumHydrogens="1">
        <t id="below_text" p="80 28" BoundingBox="74 16 86 30" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="below_left" p="72 40"/>
      <n id="below_right" p="88 40"/>
      <b id="below_bond_left" B="below" E="below_left"/>
      <b id="below_bond_right" B="below" E="below_right"/>

      <n id="above" p="140 48" Element="7" NumHydrogens="1">
        <t id="above_text" p="140 52" BoundingBox="134 40 146 54" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="above_left" p="132 32"/>
      <n id="above_right" p="148 32"/>
      <b id="above_bond_left" B="above" E="above_left"/>
      <b id="above_bond_right" B="above" E="above_right"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("left-justified automatic labels"))
        .expect("CDXML should parse");
    let label = |node_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .unwrap_or_else(|| panic!("missing label for {node_id}"))
    };

    let reversed = label("right");
    assert_eq!(reversed.source_text.as_deref(), Some("NH"));
    assert_eq!(reversed.text, "HN");
    assert_eq!(reversed.layout.as_deref(), Some("attached-group"));
    assert_eq!(
        reversed
            .meta
            .pointer("/import/cdxml/labelJustification")
            .and_then(serde_json::Value::as_str),
        Some("Left")
    );

    let stacked_above = label("below");
    assert_eq!(stacked_above.lines, ["H", "N"]);
    assert_eq!(
        stacked_above.layout.as_deref(),
        Some("attached-group-above")
    );

    let stacked_below = label("above");
    assert_eq!(stacked_below.lines, ["N", "H"]);
    assert_eq!(
        stacked_below.layout.as_deref(),
        Some("attached-group-below")
    );
}

#[test]
fn parse_cdxml_explicit_label_display_still_overrides_connection_aware_layout() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" LabelJustification="Auto">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 60 40">
      <n id="fixed" p="16 20" Element="7" NumHydrogens="1" LabelDisplay="Left">
        <t id="fixed_text" p="16 24" BoundingBox="10 12 22 26" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">NH</s>
        </t>
      </n>
      <n id="right_up" p="32 12"/>
      <n id="right_down" p="32 28"/>
      <b id="bond_up" B="fixed" E="right_up"/>
      <b id="bond_down" B="fixed" E="right_down"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("fixed left label display")).expect("CDXML should parse");
    let label = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "fixed"))
        .and_then(|node| node.label.as_ref())
        .expect("fixed label should import");

    assert_eq!(label.source_text.as_deref(), Some("NH"));
    assert_eq!(label.text, "NH");
    assert_eq!(label.align.as_deref(), Some("left"));
    assert_eq!(label.anchor.as_deref(), Some("start"));
}

#[test]
fn parse_cdxml_label_fields_keep_their_official_layout_roles() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" LabelJustification="Auto">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 190 110">
      <n id="alignment_right" p="42 20" Element="7" NumHydrogens="1">
        <t p="42 24" LabelAlignment="Right"><s font="3" size="10" face="96">NH</s></t>
      </n>
      <n id="ar_up" p="26 12"/><n id="ar_down" p="26 28"/>
      <b id="ar_b1" B="alignment_right" E="ar_up"/><b id="ar_b2" B="alignment_right" E="ar_down"/>

      <n id="alignment_above" p="76 20" Element="7" NumHydrogens="1">
        <t p="76 24" LabelAlignment="Above"><s font="3" size="10" face="96">NH</s></t>
      </n>
      <n id="aa_up" p="92 12"/><n id="aa_down" p="92 28"/>
      <b id="aa_b1" B="alignment_above" E="aa_up"/><b id="aa_b2" B="alignment_above" E="aa_down"/>

      <n id="display_above" p="112 20" Element="7" NumHydrogens="1" LabelDisplay="Above">
        <t p="112 12" LabelAlignment="Above"><s font="3" size="10" face="96">NH</s></t>
      </n>
      <n id="da_up" p="128 12"/><n id="da_down" p="128 28"/>
      <b id="da_b1" B="display_above" E="da_up"/><b id="da_b2" B="display_above" E="da_down"/>

      <n id="authored_lines" p="154 20" NodeType="Fragment">
        <t p="154 24" LineStarts="4 7"><s font="3" size="10" face="96">Cl2&#10;Zr</s></t>
      </n>
      <n id="ml_up" p="170 12"/><n id="ml_down" p="170 28"/>
      <b id="ml_b1" B="authored_lines" E="ml_up"/><b id="ml_b2" B="authored_lines" E="ml_down"/>

      <n id="authored_offsets" p="154 64" NodeType="Fragment">
        <t p="154 68" LineStarts="3 5"><s font="3" size="10" face="96">Cl2Zr</s></t>
      </n>
      <n id="mo_right" p="170 64"/>
      <b id="mo_b1" B="authored_offsets" E="mo_right"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("label field roles")).expect("CDXML");
    let label = |node_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .unwrap_or_else(|| panic!("missing {node_id}"))
    };

    assert_eq!(label("alignment_right").text, "NH");
    assert_eq!(label("alignment_above").text, "HN");
    assert!(label("alignment_above").lines.is_empty());
    assert_eq!(label("display_above").text, "NH");
    assert!(label("display_above").lines.is_empty());
    assert_eq!(label("authored_lines").text, "Cl2\nZr");
    assert_eq!(label("authored_lines").lines, ["Cl2", "Zr"]);
    assert!(label("authored_lines").runs.is_empty());
    assert_eq!(label("authored_lines").line_runs.len(), 2);
    assert_eq!(
        label("authored_lines").line_runs[0]
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Cl2"
    );
    assert_eq!(
        label("authored_lines").line_runs[1]
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "Zr"
    );
    assert_eq!(label("authored_offsets").text, "Cl2\nZr");
    assert_eq!(label("authored_offsets").lines, ["Cl2", "Zr"]);
    assert_eq!(label("authored_offsets").line_runs.len(), 2);
}

#[test]
fn parse_cdxml_infers_nested_fragment_external_connection_for_parent_bond() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="10" LabelSize="10">
  <page id="1">
    <fragment id="2">
      <n id="wrapper" NodeType="Fragment">
        <fragment id="3">
          <n id="inner" p="10 20"/>
          <n id="anchor" p="20 20"/>
          <n id="external" NodeType="ExternalConnectionPoint"/>
          <b id="i1" B="inner" E="anchor"/>
          <b id="i2" B="anchor" E="external"/>
        </fragment>
        <t><s font="3" size="10" face="96">DCM</s></t>
      </n>
      <n id="m" NodeType="GenericNickname" p="40 20">
        <t p="40 20"><s font="3" size="10" face="96">M</s></t>
      </n>
      <b id="outer" B="wrapper" E="m"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("nested fragment connection")).expect("CDXML");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("parent fragment");
    let wrapper = fragment
        .nodes
        .iter()
        .find(|node| node.id == "wrapper")
        .expect("wrapper node");

    assert!(fragment.bonds.iter().any(|bond| bond.id == "outer"));
    assert!(wrapper.label.is_some());
    let m = fragment
        .nodes
        .iter()
        .find(|node| node.id == "m")
        .expect("M");
    assert!((m.position[0] - wrapper.position[0] - 10.0).abs() < 0.01);
}

#[test]
fn cdxml_caption_fields_override_obsolete_text_fields_and_roundtrip() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Left">
  <page id="1" BoundingBox="0 0 120 60">
    <t id="2" p="80 30" BoundingBox="20 10 80 34"
       CaptionJustification="Right" Justification="Left" LabelJustification="Center"
       LineHeight="9" CaptionLineHeight="auto" WordWrapWidth="72" LineStarts="6 12">
      <s font="3" size="10">alpha&#10;beta</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("caption fields")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(text.payload.extra.get("align"), Some(&json!("right")));
    assert_eq!(
        text.meta.pointer("/import/cdxml/captionLineHeight"),
        Some(&json!("auto"))
    );

    let exported = document_to_cdxml(&document);
    for expected in [
        "CaptionJustification=\"Right\"",
        "Justification=\"Left\"",
        "CaptionLineHeight=\"auto\"",
        "LineHeight=\"9\"",
        "WordWrapWidth=\"72\"",
        "LineStarts=\"6 10\"",
    ] {
        assert!(
            exported.contains(expected),
            "missing {expected}: {exported}"
        );
    }
    assert!(!exported.contains("LabelJustification=\"Center\""));
    let reopened =
        parse_cdxml_document(&exported, Some("reopened caption fields")).expect("export reopens");
    let exported_again = document_to_cdxml(&reopened);
    assert!(exported_again.contains("LineStarts=\"6 10\""));
    assert!(!exported_again.contains("LineStarts=\"6 12\""));
}
