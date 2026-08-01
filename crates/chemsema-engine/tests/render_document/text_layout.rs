use super::*;

#[test]
fn cdxml_styled_string_fonts_follow_chemdraw_run_state() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="12" CaptionFont="4" CaptionSize="12">
  <fonttable>
    <font id="2" charset="iso-8859-1" name="Arial"/>
    <font id="3" charset="iso-8859-1" name="Times New Roman"/>
  </fonttable>
  <page>
    <fragment>
      <n id="missing" p="20 30" NodeType="Nickname">
        <t p="20 34"><s>NiA</s></t>
      </n>
      <n id="preceding" p="80 30" NodeType="Nickname">
        <t p="80 34"><s font="3">A3</s><s>B0</s></t>
      </n>
      <n id="text-parent" p="140 30" NodeType="Nickname">
        <t p="140 34" font="3"><s>Ct</s></t>
      </n>
    </fragment>
    <t id="free-state" p="20 80"><s>G0</s><s font="3">H3</s><s>Ix</s></t>
    <t id="free-invalid" p="160 80"><s font="4">TxU</s></t>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(source, Some("styled string font state")).expect("CDXML parses");
    let label = |node_id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == node_id)
            .and_then(|node| node.label.as_ref())
            .expect("node label")
    };

    assert_eq!(label("missing").font_family.as_deref(), Some("Arial"));
    assert!(label("missing")
        .runs
        .iter()
        .all(|run| run.font_family.as_deref() == Some("Arial")));
    assert_eq!(
        label("preceding")
            .runs
            .iter()
            .map(|run| (run.text.as_str(), run.font_family.as_deref()))
            .collect::<Vec<_>>(),
        vec![("A3B0", Some("Times New Roman"))]
    );
    assert_eq!(
        label("text-parent").font_family.as_deref(),
        Some("Arial"),
        "ChemDraw ignores font on <t>; only preceding <s> run state is inherited"
    );

    let text_runs = |source_id: &str| {
        document
            .objects
            .iter()
            .find(|object| {
                object
                    .meta
                    .get("textId")
                    .and_then(serde_json::Value::as_str)
                    == Some(source_id)
            })
            .and_then(|object| object.payload.extra.get("runs"))
            .and_then(serde_json::Value::as_array)
            .expect("text runs")
    };
    assert_eq!(
        text_runs("free-state")
            .iter()
            .map(|run| {
                (
                    run.get("text").and_then(serde_json::Value::as_str),
                    run.get("fontFamily").and_then(serde_json::Value::as_str),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (Some("G0"), Some("Arial")),
            (Some("H3"), Some("Times New Roman")),
            (Some("Ix"), Some("Times New Roman")),
        ]
    );
    assert_eq!(
        text_runs("free-invalid")[0]
            .get("fontFamily")
            .and_then(serde_json::Value::as_str),
        Some("Arial"),
        "an undefined font-table ID resolves to ChemDraw's Arial fallback"
    );
}

#[test]
fn parse_cdxml_fixed_edge_labels_anchor_subscripts_but_not_superscripts() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.4">
  <fonttable><font id="4" charset="0" name="Times New Roman"/></fonttable>
  <page>
    <fragment>
      <n id="left" p="100 100" NodeType="Unspecified" LabelDisplay="Left">
        <t p="100 102.67" LabelAlignment="Left" LabelJustification="Left">
          <s font="4" size="7" face="64">+</s><s font="4" size="7" face="32">3</s><s font="4" size="7" face="96">(Aax)</s>
        </t>
      </n>
      <n id="left-neighbor" p="85.6 100"/>
      <b id="left-bond" B="left" E="left-neighbor"/>
      <n id="right" p="140 100" NodeType="Unspecified" LabelDisplay="Right">
        <t p="140 102.67" LabelAlignment="Right" LabelJustification="Right">
          <s font="4" size="7" face="96">(Aax)</s><s font="4" size="7" face="34">n</s><s font="4" size="7" face="66">+</s>
        </t>
      </n>
      <n id="right-neighbor" p="154.4 100"/>
      <b id="right-bond" B="right" E="right-neighbor"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(source, Some("fixed edge script anchors")).expect("CDXML parses");
    let find_node = |id: &str| {
        document
            .resources
            .values()
            .filter_map(|resource| resource.data.as_fragment())
            .flat_map(|fragment| fragment.nodes.iter())
            .find(|node| node.id == id)
            .expect("node")
    };
    let center_x = |polygon: &Vec<[f64; 2]>| {
        let min = polygon
            .iter()
            .map(|point| point[0])
            .fold(f64::INFINITY, f64::min);
        let max = polygon
            .iter()
            .map(|point| point[0])
            .fold(f64::NEG_INFINITY, f64::max);
        (min + max) * 0.5
    };

    let left = find_node("left");
    let left_label = left.label.as_ref().expect("left label");
    assert!(
        (center_x(&left_label.glyph_polygons[1]) - left.position[0]).abs() < 0.01,
        "the leading subscript is the fixed left attachment glyph"
    );

    let right = find_node("right");
    let right_label = right.label.as_ref().expect("right label");
    assert!(
        (center_x(&right_label.glyph_polygons[5]) - right.position[0]).abs() < 0.01,
        "the trailing subscript is the fixed right attachment glyph"
    );
    assert!(
        (center_x(&right_label.glyph_polygons[6]) - right.position[0]).abs() > 0.5,
        "the trailing superscript remains a decoration, not an attachment glyph"
    );
}

#[test]
fn render_single_line_attached_labels_uses_resolved_box_without_authored_overhang() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="4" LabelSize="7">
  <fonttable><font id="4" charset="0" name="Times New Roman"/></fonttable>
  <page>
    <fragment>
      <n id="right" p="100 100" NodeType="Unspecified" LabelDisplay="Right">
        <t p="102 102.73" BoundingBox="82 96 102 103"
           LabelAlignment="Right" LabelJustification="Right">
          <s font="4" size="7" face="96">(Aax)</s><s font="4" size="7" face="34">n</s>
        </t>
      </n>
      <n id="neighbor" p="116 100"/>
      <b id="bond" B="right" E="neighbor"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(source, Some("resolved label render origin")).expect("CDXML parses");
    let entry = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "right")
        .expect("right label node");
    let label = node.label.as_ref().expect("right label");
    let box_value = label.bbox().expect("resolved active label box");
    let last_glyph = label
        .glyph_polygons
        .last()
        .expect("trailing subscript glyph");
    let last_min_x = last_glyph
        .iter()
        .map(|point| point[0])
        .fold(f64::INFINITY, f64::min);
    let last_max_x = last_glyph
        .iter()
        .map(|point| point[0])
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        ((last_min_x + last_max_x) * 0.5 - node.position[0]).abs() < 0.01,
        "the semantic fixed-edge rule must still attach the trailing subscript"
    );

    let (render_x, render_anchor) = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id,
                x,
                text_anchor,
                ..
            } if node_id.as_deref() == Some("right") => Some((x, text_anchor)),
            _ => None,
        })
        .expect("rendered right label");
    let expected_x = entry.object.transform.translate[0] + box_value[0];
    assert_close(render_x, expected_x);
    assert_eq!(render_anchor.as_deref(), Some("start"));
}

#[test]
fn parse_cdxml_vertical_bond_retreat_ignores_distant_formula_subscript() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.6" MarginWidth="1.6" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="32">
    <fragment id="6">
      <n id="7" p="133.97 199.80" AS="N"/>
      <n id="11" p="133.97 185.40" NumHydrogens="3" AS="N">
        <t p="130.36 189.30" BoundingBox="130.36 180.84 148.97 191.60"
           LabelJustification="Left" LabelAlignment="Left">
          <s font="3" size="10" color="0" face="96">CH3</s>
        </t>
      </n>
      <b id="12" B="7" E="11"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(source, Some("vertical CH3 attachment")).expect("CDXML parses");
    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "12" => Some(points),
            _ => None,
        })
        .expect("bond polygon");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.y < to.y { from } else { to };

    assert!(
        (label_endpoint.y - 191.02).abs() < 0.08,
        "ChemDraw's attachment column is set by C, not the distant subscript 3: {polygon:?}"
    );
}

#[test]
fn parse_cdxml_vertical_bond_retreat_uses_the_glyph_owned_by_the_bond_column() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.5" MarginWidth="1.25" LabelFont="20" LabelSize="7">
  <fonttable><font id="20" charset="0" name="Times New Roman"/></fonttable>
  <page>
    <fragment>
      <n id="13" p="82.34 264.80" NodeType="Nickname" LabelDisplay="Left">
        <t p="79.82 267.45" BoundingBox="79.82 261.79 90.32 267.55"
           LabelJustification="Left" LabelAlignment="Left">
          <s font="20" size="7" face="96">Gln</s>
        </t>
      </n>
      <n id="17" p="85.72 280.80" NodeType="Nickname" LabelDisplay="Center">
        <t p="85.72 283.45" BoundingBox="80.47 278.02 90.97 284.95"
           LabelJustification="Center" Justification="Center" LabelAlignment="Left">
          <s font="20" size="7" face="96">Lys</s>
        </t>
      </n>
      <b id="18" B="13" E="17" BeginAttach="1"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(source, Some("vertical Lys attachment")).expect("CDXML parses");
    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "18" => Some(points),
            _ => None,
        })
        .expect("bond polygon");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.y > to.y { from } else { to };

    assert!(
        (label_endpoint.y - 279.07).abs() < 0.12,
        "the y glyph owns the vertical column; the expanded L must not shorten the bond: {polygon:?}"
    );
}

#[test]
fn parse_cdxml_downward_label_retreat_keeps_the_column_local_axis_contact() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.5" MarginWidth="1.25" LabelFont="4" LabelSize="7">
  <fonttable><font id="4" charset="0" name="Times New Roman"/></fonttable>
  <page>
    <fragment>
      <n id="11" p="28.56 18.65" NodeType="Unspecified" LabelDisplay="Left">
        <t p="26.40 21.38" BoundingBox="26.40 15.64 36.51 21.87"
           LabelJustification="Left" LabelAlignment="Left">
          <s font="4" size="7" face="96">Tyr</s>
        </t>
      </n>
      <n id="13" p="28.56 32.58"/>
      <b id="14" B="11" E="13"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(source, Some("downward Tyr attachment")).expect("CDXML parses");
    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "14" => Some(points),
            _ => None,
        })
        .expect("bond polygon");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.y < to.y { from } else { to };

    assert!(
        (label_endpoint.y - 24.105).abs() < 0.05,
        "ChemDraw keeps the column-local baseline contact below Tyr: {polygon:?}"
    );
}

#[test]
fn parse_cdxml_horizontal_label_retreat_uses_the_cardinal_run_contact() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.5" MarginWidth="1.25" LabelFont="20" LabelSize="7">
  <fonttable><font id="20" charset="iso-8859-1" name="Times New Roman"/></fonttable>
  <page>
    <fragment>
      <n id="1" p="100 100" NodeType="Nickname" LabelDisplay="Center">
        <t p="100 102.65" BoundingBox="94.95 97.22 105.05 104.15"
           LabelJustification="Center" Justification="Center" LabelAlignment="Center">
          <s font="20" size="7" face="96">Tyr</s>
        </t>
      </n>
      <n id="2" p="116 100"/>
      <b id="4" B="1" E="2"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(source, Some("horizontal Tyr attachment")).expect("CDXML parses");
    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "4" => Some(points),
            _ => None,
        })
        .expect("bond polygon");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x < to.x { from } else { to };

    assert!(
        (label_endpoint.x - 105.8465).abs() < 0.05,
        "ChemDraw uses the horizontal cardinal run contact for Tyr: {polygon:?}"
    );
}

#[test]
fn parse_cdxml_applies_authored_line_starts_to_unbroken_caption_runs() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Center">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="10 10 90 34"
       CaptionJustification="Center" WordWrapWidth="80" LineStarts="5">
      <s font="3" size="10">alphabeta</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("authored line starts")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(text.payload.extra.get("text"), Some(&json!("alpha\nbeta")));
    let rendered_runs = text
        .payload
        .extra
        .get("runs")
        .and_then(|value| value.as_array())
        .expect("styled runs");
    assert_eq!(rendered_runs[0].get("text"), Some(&json!("alpha\nbeta")));
}

#[test]
fn parse_cdxml_does_not_materialize_line_starts_without_word_wrap_width() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Left">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="10 10 90 34" LineStarts="2 3">
      <s font="3" size="10">ABC</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("non-wrapped line starts")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(text.payload.extra.get("text"), Some(&json!("ABC")));
    assert_eq!(
        text.meta.pointer("/import/cdxml/lineStarts"),
        Some(&json!("2 3"))
    );
}

#[test]
fn parse_cdxml_chemical_node_line_starts_remain_derived_layout_metadata() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.4" LabelFont="3" LabelSize="10">
  <page id="1">
    <fragment id="10">
      <n id="11" p="72 72" Element="7" NumHydrogens="1" Charge="1">
        <t id="20" p="72 72" InterpretChemically="yes"
           LabelAlignment="Above" LineStarts="2 4 6">
          <s font="3" size="10" face="96">NH+</s>
        </t>
      </n>
      <n id="12" p="54 82"/><n id="13" p="90 82"/>
      <b id="14" B="12" E="11"/><b id="15" B="11" E="13"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("chemical label line starts")).expect("CDXML");
    let label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "11")
        .and_then(|node| node.label.as_ref())
        .expect("N label");

    assert_eq!(label.source_text.as_deref(), Some("NH+"));
    assert_eq!(label.text, "H+\nN");
    assert_eq!(label.lines, ["H+", "N"]);
    assert_eq!(
        label.meta.pointer("/import/cdxml/lineStarts"),
        Some(&json!("2 4 6"))
    );
    assert_eq!(
        label
            .meta
            .get("sourceRuns")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|run| run.get("text").and_then(|value| value.as_str()))
            .collect::<String>(),
        "NH+"
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("LineStarts=\"2 4 6\""), "{exported}");
    assert!(exported.contains(">NH+</s>"), "{exported}");
    assert!(!exported.contains("NH&#10;+"), "{exported}");
}

#[test]
fn parse_cdxml_line_starts_count_existing_end_of_line_characters() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Center">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="10 10 90 46"
       CaptionJustification="Center" WordWrapWidth="80" LineStarts="4 7 10">
      <s font="3" size="10">abc&#10;de&#10;fgh</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("line starts with EOLs")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(text.payload.extra.get("text"), Some(&json!("abc\nde\nfgh")));
    let rendered_runs = text
        .payload
        .extra
        .get("runs")
        .and_then(|value| value.as_array())
        .expect("styled runs");
    assert_eq!(rendered_runs[0].get("text"), Some(&json!("abc\nde\nfgh")));
}

#[test]
fn parse_cdxml_line_starts_are_utf8_byte_offsets() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Center">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="10 10 90 34"
       CaptionJustification="Center" WordWrapWidth="80" LineStarts="8 17">
      <s font="3" size="10">alpha′betagamma</s>
    </t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("UTF-8 line starts")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(
        text.payload.extra.get("text"),
        Some(&json!("alpha′\nbetagamma"))
    );
}

#[test]
fn materialized_utf8_line_starts_are_idempotent_across_cdxml_saves() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionJustification="Center">
  <page id="1">
    <t id="2" p="50 20" BoundingBox="10 10 90 46"
       CaptionJustification="Center" WordWrapWidth="80" LineStarts="8 12 17">
      <s font="3" size="10">alpha′betagamma</s>
    </t>
  </page>
</CDXML>"##;
    let first = parse_cdxml_document(source, Some("UTF-8 authored wraps")).expect("CDXML");
    let first_text = first
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(
        first_text.payload.extra.get("text"),
        Some(&json!("alpha′\nbeta\ngamma"))
    );

    let exported = document_to_cdxml(&first);
    assert!(exported.contains("LineStarts=\"9 14 19\""), "{exported}");
    let second =
        parse_cdxml_document(&exported, Some("UTF-8 authored wraps reopened")).expect("CDXML");
    let second_text = second
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("reopened text object");
    assert_eq!(
        second_text.payload.extra.get("text"),
        first_text.payload.extra.get("text")
    );
    assert_eq!(
        second_text.payload.extra.get("runs"),
        first_text.payload.extra.get("runs")
    );
}

#[test]
fn parse_cdxml_line_starts_preserve_authored_leading_blank_lines() {
    let source = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML>
  <page id="1">
    <t id="2" p="50 20" BoundingBox="10 10 90 46"
       LineStarts="2 3 9"><s font="3" size="10">&#9;&#10;&#10;serial</s></t>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(source, Some("leading authored lines")).expect("CDXML");
    let text = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object");
    assert_eq!(text.payload.extra.get("text"), Some(&json!("\t\n\nserial")));
}

#[test]
fn parse_cdxml_preserves_explicit_zero_hydrogens_on_imported_nitrogen() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="1" p="20 20"/>
      <n id="2" p="40 20" Element="7" NumHydrogens="0">
        <t id="20" p="36 24" BoundingBox="36 16 44 25" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <n id="3" p="60 20"/>
      <b id="4" B="1" E="2"/>
      <b id="5" B="2" E="3"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("explicit h0")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen node should import");

    assert_eq!(nitrogen.num_hydrogens, 0);
    assert_eq!(
        nitrogen
            .meta
            .pointer("/import/cdxml/explicitNumHydrogens")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
    assert_eq!(
        nitrogen
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        None
    );
    assert_eq!(
        nitrogen.label.as_ref().map(|label| label.text.as_str()),
        Some("N")
    );
}

#[test]
fn neutral_second_period_nitrogen_does_not_use_five_valence_to_add_hydrogen() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 90 60">
      <n id="1" p="20 20"/>
      <n id="2" p="40 20" Element="7">
        <t id="20" p="36 24" BoundingBox="36 16 44 25" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <n id="3" p="60 20"/>
      <n id="4" p="40 40"/>
      <b id="5" B="1" E="2" Order="2"/>
      <b id="6" B="2" E="3"/>
      <b id="7" B="2" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("neutral tetravalent n")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen node should import");

    assert_eq!(nitrogen.num_hydrogens, 0);
    assert_eq!(
        nitrogen
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        Some("invalid")
    );
}

#[test]
fn neutral_second_period_boron_four_connection_label_is_invalid() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 100 80">
      <n id="1" p="20 40"/>
      <n id="2" p="40 40" Element="5">
        <t id="20" p="36 44" BoundingBox="36 34 44 45" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">B</s>
        </t>
      </n>
      <n id="3" p="60 40"/>
      <n id="4" p="40 20"/>
      <n id="5" p="40 60"/>
      <b id="6" B="1" E="2"/>
      <b id="7" B="2" E="3"/>
      <b id="8" B="2" E="4"/>
      <b id="9" B="2" E="5"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("neutral tetravalent b")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let boron = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("boron node should import");

    assert_eq!(
        boron
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        Some("invalid")
    );
}

#[test]
fn second_period_carbon_label_five_connection_is_invalid() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 120 80">
      <n id="1" p="20 40"/>
      <n id="2" p="50 40" Element="6">
        <t id="20" p="46 44" BoundingBox="46 34 54 45" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">C</s>
        </t>
      </n>
      <n id="3" p="80 40"/>
      <n id="4" p="50 15"/>
      <n id="5" p="50 65"/>
      <n id="6" p="70 60"/>
      <b id="7" B="1" E="2"/>
      <b id="8" B="2" E="3"/>
      <b id="9" B="2" E="4"/>
      <b id="10" B="2" E="5"/>
      <b id="11" B="2" E="6"/>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("pentavalent c")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let carbon = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("carbon node should import");

    assert_eq!(
        carbon
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        Some("invalid")
    );
}

#[test]
fn metal_coordination_does_not_create_implicit_hydrogen_on_pyridine_nitrogen() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2" HashSpacing="2.50" LabelSize="10">
  <page id="1">
    <fragment id="2" BoundingBox="0 0 110 60">
      <n id="1" p="20 20"/>
      <n id="2" p="40 20" Element="7">
        <t id="20" p="36 24" BoundingBox="36 16 44 25" LabelAlignment="Left" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <n id="3" p="60 20"/>
      <n id="4" p="40 40" Element="29">
        <t id="21" p="38 44" BoundingBox="38 34 50 45" LabelAlignment="Center" LabelJustification="Center">
          <s font="3" size="10" face="96" color="0">Cu</s>
        </t>
      </n>
      <b id="5" B="1" E="2" Order="2"/>
      <b id="6" B="2" E="3"/>
      <b id="7" B="2" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("coordinated pyridine n")).expect("cdxml should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("import should create molecule fragment resource");
    let nitrogen = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("nitrogen node should import");

    assert_eq!(nitrogen.num_hydrogens, 0);
    assert_eq!(
        nitrogen
            .meta
            .get("labelRecognition")
            .and_then(|meta| meta.get("status"))
            .and_then(|status| status.as_str()),
        None
    );
}

#[test]
fn parse_cdxml_matches_default_and_acs_double_bond_spacing_samples() {
    for (fixture, expected_normal, expected_bold, expected_widths) in [
        ("db.cdxml", 3.6, 5.1, [1.0, 4.0]),
        ("db-acs.cdxml", 2.592, 3.292, [0.6, 2.0]),
    ] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");
        let primitives = render_document(&document);

        let normal = imported_vertical_line_metrics(&primitives, "obj_mol_001");
        assert_line_spacing(&normal, expected_normal, fixture);
        assert_line_widths(&normal, expected_widths[0], expected_widths[0], fixture);

        let dashed_solid = imported_vertical_line_metrics(&primitives, "obj_mol_002");
        assert_line_spacing(&dashed_solid, expected_normal, fixture);
        let dashed_solid_bond = imported_fragment_bond(&document, "obj_mol_002", "9");
        assert_eq!(dashed_solid_bond.order, 2);
        assert_eq!(
            dashed_solid_bond.line_styles.right,
            chemsema_engine::BondLinePattern::Dashed
        );

        let bold = imported_vertical_line_metrics(&primitives, "obj_mol_003");
        assert_line_spacing(&bold, expected_bold, fixture);
        assert_line_widths(&bold, expected_widths[0], expected_widths[1], fixture);

        let dashed = imported_vertical_line_metrics(&primitives, "obj_mol_004");
        assert_line_spacing(&dashed, expected_normal, fixture);
        let dashed_bond = imported_fragment_bond(&document, "obj_mol_004", "17");
        assert_eq!(dashed_bond.order, 2);
        assert_eq!(
            dashed_bond.line_styles.left,
            chemsema_engine::BondLinePattern::Dashed
        );
        assert_eq!(
            dashed_bond.line_styles.right,
            chemsema_engine::BondLinePattern::Dashed
        );
    }
}

#[test]
fn parse_cdxml_double_bond_spacing_uses_chemdraw_line_width_floor() {
    for (name, line_width, bond_length, bond_spacing, expected_center_distance) in [
        ("acs", 0.60, 14.40, 18.0, 2.592),
        ("default", 1.00, 30.00, 12.0, 3.600),
        ("thick-short", 1.98, 22.68, 12.0, 4.950),
    ] {
        let end_x = 100.0 + bond_length;
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemDraw 22.2.0.3300" FractionalWidths="yes" LineWidth="{line_width:.2}" BoldWidth="4.00" BondLength="{bond_length:.2}" BondSpacing="{bond_spacing:.0}" HashSpacing="2.70" MarginWidth="2.00" LabelSize="10">
  <page id="1" BoundingBox="0 0 200 100">
    <fragment id="2" BoundingBox="90 90 140 110">
      <n id="3" p="100.00 100.00"/>
      <n id="4" p="{end_x:.2} 100.00"/>
      <b id="5" B="3" E="4" Order="2"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let document = parse_cdxml_document(&cdxml, Some(name)).expect("cdxml should parse");
        let rendered = imported_double_bond_center_spacing(&document, "obj_mol_001");
        let formula = imported_double_bond_formula_spacing(&document, "obj_mol_001");

        assert!(
            (rendered - expected_center_distance).abs() < 0.01,
            "{name}: expected {expected_center_distance}, rendered {rendered}"
        );
        assert!(
            (formula - expected_center_distance).abs() < 0.01,
            "{name}: expected {expected_center_distance}, formula {formula}"
        );
    }
}

#[test]
fn parse_cdxml_triple_bond_spacing_matches_chemdraw_percentage_and_absolute_rules() {
    for (name, line_width, bond_length, bond_spacing, bond_spacing_abs, expected_center_distance) in [
        ("acs", 0.60, 14.40, 18.0, None, 2.592),
        ("length-scaled", 0.60, 30.00, 18.0, None, 5.400),
        ("line-width-floor", 2.00, 14.40, 8.0, None, 5.000),
        ("absolute-no-floor", 2.00, 14.40, 30.0, Some(0.5), 0.500),
    ] {
        let spacing_abs = bond_spacing_abs
            .map(|value| format!(r#" BondSpacingAbs="{value}""#))
            .unwrap_or_default();
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="{line_width}" BondLength="14.4" BondSpacing="{bond_spacing}">
  <page id="1"><fragment id="2">
    <n id="3" p="20 20"/><n id="4" p="20 {}"/>
    <b id="5" B="3" E="4" Order="3"{spacing_abs}/>
  </fragment></page>
</CDXML>"#,
            20.0 + bond_length,
        );
        let document = parse_cdxml_document(&cdxml, Some(name)).expect("cdxml should parse");
        let primitives = render_document(&document);
        let metrics = imported_vertical_line_metrics(&primitives, "obj_mol_001");
        assert_adjacent_line_spacing(&metrics, expected_center_distance, name);
    }
}

#[test]
fn parse_cdxml_recognizes_fractional_dashed_double_bond() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" LabelSize="10">
  <page id="p1" BoundingBox="0 0 50 50">
    <fragment id="f1" BoundingBox="0 0 50 50">
      <n id="n1" p="24 10"/>
      <n id="n2" p="24 34"/>
      <b id="b1" B="n1" E="n2" Order="1.5" Display2="Dash"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("fractional dashed double")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");

    assert_eq!(bond.order, 2);
    let double = bond
        .double
        .as_ref()
        .expect("fractional bond should render as a double bond");
    assert_eq!(
        double.placement,
        chemsema_engine::DoubleBondPlacement::Center
    );
    assert!(
        !double.frozen,
        "Display2 without DoublePosition should keep automatic placement"
    );
    assert_eq!(
        bond.line_styles.right,
        chemsema_engine::BondLinePattern::Dashed
    );
    assert_eq!(
        bond.meta
            .pointer("/import/cdxml/display2")
            .and_then(serde_json::Value::as_str),
        Some("Dash")
    );

    let primitives = render_document(&document);
    let bond_polygons: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                object_id,
                bond_id,
                points,
                ..
            } if object_id.as_deref() == Some("obj_mol_001")
                && bond_id.as_deref() == Some("b1") =>
            {
                Some(points)
            }
            _ => None,
        })
        .collect();
    assert!(
        bond_polygons.len() > 2,
        "virtual/solid double bond should render one solid line plus black dash segments: {bond_polygons:?}"
    );
    let lengths: Vec<_> = bond_polygons
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    assert!(
        lengths.iter().any(|length| *length > 18.0)
            && lengths.iter().any(|length| *length > 2.0 && *length < 3.0),
        "Display2=\"Dash\" should use the same evenly distributed black segments as dashed bonds: {lengths:?}"
    );
    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentKnockout,
                object_id,
                node_id: None,
                ..
            } if object_id.as_deref() == Some("obj_mol_001")
        )),
        "dashed double bonds should draw black dash segments directly, not a solid line with knockout gaps: {primitives:?}"
    );
    let exported = document_to_cdxml(&document);
    assert!(exported.contains("Display2=\"Dash\""), "{exported}");
}

#[test]
fn parse_cdxml_double_bond_spacing_scales_with_actual_bond_length() {
    for (fixture, expected_spacings) in [
        (
            "db-chang.cdxml",
            [
                ("obj_mol_001", 9.0002),
                ("obj_mol_002", 12.8413),
                ("obj_mol_003", 14.5250),
                ("obj_mol_004", 9.5205),
            ],
        ),
        (
            "db-acs-chang.cdxml",
            [
                ("obj_mol_001", 4.7411),
                ("obj_mol_002", 5.7277),
                ("obj_mol_003", 5.9441),
                ("obj_mol_004", 5.2895),
            ],
        ),
    ] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let document = parse_cdxml_document(&cdxml, Some(fixture)).expect("cdxml should parse");

        for (object_id, expected) in expected_spacings {
            let rendered = imported_double_bond_center_spacing(&document, object_id);
            let formula = imported_double_bond_formula_spacing(&document, object_id);
            assert!(
                (rendered - expected).abs() < 0.01,
                "{fixture} {object_id}: expected {expected}, rendered {rendered}"
            );
            assert!(
                (formula - expected).abs() < 0.01,
                "{fixture} {object_id}: expected {expected}, formula {formula}"
            );
        }
    }
}

#[test]
fn render_document_emits_arrow_line_primitives() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[10.0, 20.0], [110.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "length": 22.5,
                    "centerLength": 19.69,
                    "width": 5.63,
                    "curve": 0.0,
                    "head": "full",
                    "tail": "full",
                    "bold": false,
                    "noGo": "none"
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let shaft = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("line shaft primitive");
    assert_eq!(shaft.len(), 2);
    assert!(shaft[1].x < 110.0);

    let arrow_head_paths: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                d,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() == 6 =>
            {
                Some((points.clone(), d.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(arrow_head_paths.len(), 2);
    assert!(arrow_head_paths[0].1.contains(" C "));
    let head_width = arrow_head_paths[0]
        .0
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        - arrow_head_paths[0]
            .0
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
    assert!((head_width - 8.2072).abs() <= 0.001);
}

#[test]
fn render_document_rounds_inner_curved_half_arrow_heads() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 100.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": 1.0,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[40.0, 20.0], [60.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "solid",
                    "length": 10.0,
                    "centerLength": 8.75,
                    "width": 2.5,
                    "curve": -120.0,
                    "head": "half-right",
                    "tail": "none",
                    "bold": false,
                    "noGo": "none"
                },
                "arrowGeometry": {
                    "center": [50.0, 25.77],
                    "majorAxisEnd": [61.55, 25.77],
                    "minorAxisEnd": [50.0, 37.32]
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let shaft_end = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001") =>
            {
                points.last().copied()
            }
            _ => None,
        })
        .expect("inner curved half arrow shaft path");
    let half_head_points = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() == 4 =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("inner curved half arrow head path");

    let cut_edge = half_head_points[3];
    assert!(
        shaft_end.distance(cut_edge) <= 0.65,
        "inner curved half-arrow shaft should stop at the head cut edge, shaft={shaft_end:?}, head={half_head_points:?}"
    );
}

#[test]
fn render_document_uses_open_arrow_width_as_extra_head_width() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [{
            "id": "obj_line_001",
            "type": "line",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_arrow_default",
            "payload": {
                "points": [[10.0, 20.0], [110.0, 20.0]],
                "head": "end",
                "tail": "none",
                "arrowHead": {
                    "kind": "hollow",
                    "length": 12.0,
                    "centerLength": 12.0,
                    "width": 3.0,
                    "curve": 0.0,
                    "head": "full",
                    "tail": "none",
                    "bold": false,
                    "noGo": "none"
                }
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let outline = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_line_001")
                && points.len() > 4 =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .expect("hollow arrow outline polygon");
    let outline_width = outline
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        - outline
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
    assert!((outline_width - 17.28).abs() <= 0.001);
}

#[test]
fn render_document_respects_thin_open_and_hollow_arrow_stroke_width() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_thin": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.6,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [
            {
                "id": "obj_hollow",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_thin",
                "payload": {
                    "points": [[10.0, 20.0], [110.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "hollow",
                        "length": 12.0,
                        "centerLength": 12.0,
                        "width": 3.0,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "none"
                    }
                }
            },
            {
                "id": "obj_open",
                "type": "line",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_thin",
                "payload": {
                    "points": [[10.0, 80.0], [110.0, 80.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "open",
                        "length": 12.0,
                        "centerLength": 12.0,
                        "width": 3.0,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "none"
                    }
                }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let hollow_width = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_hollow") =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .expect("hollow arrow outline");
    assert!((hollow_width - 0.6).abs() <= 1.0e-6, "{hollow_width}");

    let open_widths: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                role,
                object_id,
                stroke_width,
                ..
            } if *role == RenderRole::DocumentGraphic
                && object_id.as_deref() == Some("obj_open") =>
            {
                Some(*stroke_width)
            }
            _ => None,
        })
        .collect();
    assert!(!open_widths.is_empty());
    assert!(
        open_widths
            .iter()
            .all(|width| (*width - 0.6).abs() <= 1.0e-6),
        "{open_widths:?}"
    );
}

#[test]
fn cdxml_acs_hollow_and_open_arrows_keep_chemdraw_head_width() {
    let Some(arrows) = read_optional_cdxml_fixture("arrows-acs.cdxml") else {
        return;
    };
    let document = parse_cdxml_document(&arrows, Some("arrows")).expect("arrows should parse");
    let primitives = render_document(&document);

    for (object_id, expected_height) in [
        ("obj_line_004", 14.4),
        ("obj_line_005", 7.2),
        ("obj_line_006", 14.4),
        ("obj_line_007", 7.2),
    ] {
        let height = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Polygon {
                    role,
                    object_id: Some(id),
                    points,
                    ..
                }
                | RenderPrimitive::Polyline {
                    role,
                    object_id: Some(id),
                    points,
                    ..
                } if *role == RenderRole::DocumentGraphic && id == object_id => Some(
                    points
                        .iter()
                        .map(|point| point.y)
                        .fold(f64::NEG_INFINITY, f64::max)
                        - points
                            .iter()
                            .map(|point| point.y)
                            .fold(f64::INFINITY, f64::min),
                ),
                _ => None,
            })
            .fold(0.0, f64::max);

        assert!(
            (height - expected_height).abs() <= 0.001,
            "{object_id} height {height}"
        );
    }
}

#[test]
fn cdxml_import_preserves_hollow_and_open_arrow_dimensions() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="0.60">
  <page id="1">
    <arrow id="1" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="1200" ArrowheadCenterSize="1200" ArrowheadWidth="300" ArrowShaftSpacing="1200" Head3D="110 20 0" Tail3D="10 20 0"/>
    <arrow id="2" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="600" ArrowheadCenterSize="600" ArrowheadWidth="150" ArrowShaftSpacing="600" Head3D="110 50 0" Tail3D="10 50 0"/>
    <arrow id="3" ArrowheadHead="Full" ArrowheadType="Hollow" HeadSize="900" ArrowheadCenterSize="875" ArrowheadWidth="225" ArrowShaftSpacing="875" Head3D="110 80 0" Tail3D="10 80 0"/>
    <arrow id="4" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="1200" ArrowheadCenterSize="1200" ArrowheadWidth="300" ArrowShaftSpacing="1200" Head3D="110 110 0" Tail3D="10 110 0"/>
    <arrow id="5" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="600" ArrowheadCenterSize="600" ArrowheadWidth="150" ArrowShaftSpacing="600" Head3D="110 140 0" Tail3D="10 140 0"/>
    <arrow id="6" ArrowheadHead="Full" ArrowheadType="Angle" HeadSize="900" ArrowheadCenterSize="875" ArrowheadWidth="225" ArrowShaftSpacing="875" Head3D="110 170 0" Tail3D="10 170 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("hollow-open-sizes"))
        .expect("CDXML hollow/open arrows should parse");
    let arrow_head_for = |object_id: &str| {
        document
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .and_then(|object| object.payload.extra.get("arrowHead"))
            .cloned()
            .expect("arrowHead payload")
    };
    for (object_id, expected_kind, expected_length, expected_center_length, expected_width) in [
        ("obj_line_001", "hollow", 12.0, 12.0, 3.0),
        ("obj_line_002", "hollow", 6.0, 6.0, 1.5),
        ("obj_line_003", "hollow", 9.0, 8.75, 2.25),
        ("obj_line_004", "open", 12.0, 12.0, 3.0),
        ("obj_line_005", "open", 6.0, 6.0, 1.5),
        ("obj_line_006", "open", 9.0, 8.75, 2.25),
    ] {
        let arrow_head = arrow_head_for(object_id);
        assert_eq!(
            arrow_head.get("kind").and_then(serde_json::Value::as_str),
            Some(expected_kind),
            "{object_id}"
        );
        assert_eq!(
            arrow_head.get("length").and_then(serde_json::Value::as_f64),
            Some(expected_length),
            "{object_id}"
        );
        assert_eq!(
            arrow_head
                .get("centerLength")
                .and_then(serde_json::Value::as_f64),
            Some(expected_center_length),
            "{object_id}"
        );
        assert_eq!(
            arrow_head.get("width").and_then(serde_json::Value::as_f64),
            Some(expected_width),
            "{object_id}"
        );
    }
}

#[test]
fn cdxml_imports_exports_and_renders_equilibrium_arrows() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 140 60">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="1500" ArrowheadCenterSize="1313" ArrowheadWidth="375" ArrowShaftSpacing="300"
      Head3D="110 30 0" Tail3D="10 30 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("equilibrium arrow"))
        .expect("CDXML equilibrium arrow should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("equilibrium arrow should carry arrowHead payload");
    assert_eq!(
        arrow_head.get("kind").and_then(serde_json::Value::as_str),
        Some("equilibrium")
    );
    assert_eq!(
        arrow_head.get("head").and_then(serde_json::Value::as_str),
        Some("half-left")
    );
    assert_eq!(
        arrow_head.get("tail").and_then(serde_json::Value::as_str),
        Some("half-left")
    );
    assert_eq!(
        arrow_head.get("length").and_then(serde_json::Value::as_f64),
        Some(15.0)
    );
    assert_eq!(
        arrow_head
            .get("centerLength")
            .and_then(serde_json::Value::as_f64),
        Some(13.13)
    );
    assert_eq!(
        arrow_head.get("width").and_then(serde_json::Value::as_f64),
        Some(3.75)
    );
    assert_eq!(
        arrow_head
            .get("shaftSpacing")
            .and_then(serde_json::Value::as_f64),
        Some(3.0)
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ArrowheadType=\"Solid\""));
    assert!(exported.contains("ArrowShaftSpacing=\"300\""));
    assert!(!exported.contains("ArrowheadType=\"Equilibrium\""));

    let primitives: Vec<_> = render_document(&document)
        .into_iter()
        .filter(|primitive| match primitive {
            RenderPrimitive::Polyline { object_id, .. }
            | RenderPrimitive::FilledPath { object_id, .. } => {
                object_id.as_deref() == Some(&arrow.id)
            }
            _ => false,
        })
        .collect();
    assert_eq!(primitives.len(), 4);
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::Polyline { .. }))
            .count(),
        2
    );
    assert_eq!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, RenderPrimitive::FilledPath { .. }))
            .count(),
        2
    );
}

#[test]
fn cdxml_equilibrium_arrow_heads_scale_with_axis_length_like_chemdraw() {
    let regular_short = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 240 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      Head3D="194.66 94.13 0" Tail3D="183.79 94.13 0"/>
  </page>
</CDXML>"#;
    let regular_full = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 300 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      Head3D="234.50 161.63 0" Tail3D="183.79 161.63 0"/>
  </page>
</CDXML>"#;
    let unequal_short = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 260 80">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="208.54 370.50 0" Tail3D="195.79 370.50 0"/>
  </page>
</CDXML>"#;

    assert_eq!(right_arrow_head_width_from_cdxml(regular_short), 9.25);
    assert_eq!(right_arrow_head_width_from_cdxml(regular_full), 22.5);
    assert_eq!(right_arrow_head_width_from_cdxml(unequal_short), 8.5);
}

#[test]
fn cdxml_unequal_equilibrium_arrow_layout_matches_chemdraw() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="180 450 340 490">
    <arrow id="47" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="2250" ArrowheadCenterSize="1969" ArrowheadWidth="563" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="314.80 468.75 0" Tail3D="198.79 468.75 0"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("unequal equilibrium arrow")).expect("CDXML arrow parses");
    let primitives = render_document(&document);
    let mut polylines: Vec<([f64; 2], [f64; 2])> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline { points, .. } => Some(rounded_pair(points)),
            _ => None,
        })
        .collect();
    polylines.sort_by(|left, right| {
        left.0[1]
            .partial_cmp(&right.0[1])
            .unwrap()
            .then(left.0[0].partial_cmp(&right.0[0]).unwrap())
    });
    assert_eq!(
        polylines,
        vec![
            ([198.79, 467.25], [296.11, 467.25]),
            ([282.36, 470.25], [249.92, 470.25]),
        ]
    );

    let mut head_bounds: Vec<[f64; 4]> = primitives
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => {
                let bounds = primitive_polygon_bounds(&points);
                Some([
                    (bounds[0] * 100.0).round() / 100.0,
                    (bounds[1] * 100.0).round() / 100.0,
                    (bounds[2] * 100.0).round() / 100.0,
                    (bounds[3] * 100.0).round() / 100.0,
                ])
            }
            _ => None,
        })
        .collect();
    head_bounds.sort_by(|left, right| left[0].partial_cmp(&right[0]).unwrap());
    assert_eq!(
        head_bounds,
        vec![
            [231.23, 469.75, 253.73, 475.88],
            [292.3, 461.62, 314.8, 467.75],
        ]
    );
}

#[test]
fn cdxml_imports_exports_and_renders_unequal_equilibrium_arrows() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML LineWidth="1" BoldWidth="4" BondLength="30" LabelSize="10" CaptionSize="12">
  <page id="1" BoundingBox="0 0 140 60">
    <arrow id="1" ArrowheadHead="HalfLeft" ArrowheadTail="HalfLeft" ArrowheadType="Solid"
      HeadSize="1500" ArrowheadCenterSize="1313" ArrowheadWidth="375" ArrowShaftSpacing="300"
      ArrowEquilibriumRatio="300" Head3D="110 30 0" Tail3D="10 30 0"/>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("unequal equilibrium arrow"))
        .expect("CDXML unequal equilibrium arrow should parse");
    let arrow = document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow should import as line");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("unequal equilibrium arrow should carry arrowHead payload");
    assert_eq!(
        arrow_head.get("kind").and_then(serde_json::Value::as_str),
        Some("unequal-equilibrium")
    );
    assert_eq!(
        arrow_head
            .get("equilibriumRatio")
            .and_then(serde_json::Value::as_f64),
        Some(3.0)
    );

    let exported = document_to_cdxml(&document);
    assert!(exported.contains("ArrowheadType=\"Solid\""));
    assert!(exported.contains("ArrowShaftSpacing=\"300\""));
    assert!(exported.contains("ArrowEquilibriumRatio=\"300\""));

    let mut branch_lengths: Vec<f64> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polyline {
                object_id, points, ..
            } if object_id.as_deref() == Some(&arrow.id) => Some(
                points
                    .windows(2)
                    .map(|pair| pair[0].distance(pair[1]))
                    .sum::<f64>(),
            ),
            _ => None,
        })
        .collect();
    branch_lengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(branch_lengths.len(), 2);
    assert!(
        branch_lengths[0] < branch_lengths[1] * 0.45,
        "unequal equilibrium reverse branch should be much shorter: {branch_lengths:?}"
    );
}

#[test]
fn render_document_emits_arrow_no_go_marks_at_current_head_size() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_arrow_default": {
                "kind": "stroke",
                "stroke": "#222222",
                "strokeWidth": 0.72,
                "lineCap": "butt",
                "lineJoin": "miter"
            }
        },
        "objects": [
            {
                "id": "obj_line_001",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 20.0], [110.0, 20.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "hash"
                    }
                }
            },
            {
                "id": "obj_line_002",
                "type": "line",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_arrow_default",
                "payload": {
                    "points": [[10.0, 60.0], [110.0, 60.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "length": 10.0,
                        "centerLength": 8.75,
                        "width": 2.5,
                        "curve": 0.0,
                        "head": "full",
                        "tail": "none",
                        "bold": false,
                        "noGo": "cross"
                    }
                }
            }
        ],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let mark_lines_for = |object_id: &str| -> Vec<(Point, Point, f64)> {
        primitives
            .iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Line {
                    role,
                    object_id: primitive_object_id,
                    from,
                    to,
                    stroke_width,
                    ..
                } if *role == RenderRole::DocumentGraphic
                    && primitive_object_id.as_deref() == Some(object_id) =>
                {
                    Some((*from, *to, *stroke_width))
                }
                _ => None,
            })
            .collect()
    };

    let hash_marks = mark_lines_for("obj_line_001");
    assert_eq!(hash_marks.len(), 2);
    for (from, to, stroke_width) in &hash_marks {
        assert_close(*stroke_width, 0.72);
        assert_close(from.distance(*to), 10.0 * 0.72 * 5.0_f64.sqrt() * 0.5);
    }
    let mut hash_centers: Vec<Point> = hash_marks
        .iter()
        .map(|(from, to, _)| Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5))
        .collect();
    hash_centers.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    assert_point_close(hash_centers[0], Point::new(60.0 - 10.0 * 0.72 * 0.25, 20.0));
    assert_point_close(hash_centers[1], Point::new(60.0 + 10.0 * 0.72 * 0.25, 20.0));
    assert_close(hash_centers[0].distance(hash_centers[1]), 10.0 * 0.72 * 0.5);

    let cross_marks = mark_lines_for("obj_line_002");
    assert_eq!(cross_marks.len(), 2);
    for (from, to, stroke_width) in &cross_marks {
        assert_close(*stroke_width, 0.72);
        assert_close(from.distance(*to), 10.0 * 0.72 * std::f64::consts::SQRT_2);
        assert_point_close(
            Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5),
            Point::new(60.0, 60.0),
        );
    }
}

#[test]
fn render_document_emits_text_lines_from_runs() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 400.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text_001": {
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": 10.0,
                "fill": "#000000"
            }
        },
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [30.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_text_001",
            "payload": {
                "text": "Na\nCl",
                "align": "center",
                "fontSize": 10.0,
                "lineHeight": 14.0,
                "preserveLines": true,
                "runs": [{
                    "text": "Na\nCl",
                    "fontFamily": "Arial",
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "fontWeight": 400,
                    "fontStyle": "normal",
                    "script": "normal"
                }]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let primitives = render_document(&document);
    let text_lines: Vec<_> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text {
                role,
                object_id,
                x,
                y,
                runs,
                text_anchor,
                ..
            } if *role == RenderRole::DocumentText
                && object_id.as_deref() == Some("obj_text_001") =>
            {
                Some((*x, *y, runs.clone(), text_anchor.clone()))
            }
            _ => None,
        })
        .collect();

    assert_eq!(text_lines.len(), 2);
    assert!(text_lines
        .iter()
        .all(|(x, _, _, _)| (*x - 30.0).abs() < 0.001));
    assert_eq!(text_lines[0].2[0].text, "Na");
    assert_eq!(text_lines[1].2[0].text, "Cl");
    assert!(text_lines[1].1 > text_lines[0].1);
    assert_eq!(text_lines[0].3.as_deref(), Some("middle"));
}

#[test]
fn preserved_free_text_shapes_words_at_whitespace_boundaries() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "preserved text shaping",
            "page": { "width": 240.0, "height": 100.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text": {
                "kind": "text",
                "fontFamily": "Times New Roman",
                "fontSize": 7.0,
                "fill": "#000000"
            }
        },
        "objects": [{
            "id": "obj_text",
            "type": "text",
            "visible": true,
            "zIndex": 1,
            "transform": {
                "translate": [20.0, 30.0],
                "rotate": 0.0,
                "scale": [1.0, 1.0]
            },
            "styleRef": "style_text",
            "payload": {
                "text": "R is a hydrogen atom",
                "box": [0.0, 0.0, 120.0, 14.0],
                "align": "left",
                "fontSize": 7.0,
                "lineHeight": 8.16,
                "baselineOffset": 5.3,
                "preserveLines": true,
                "runs": [{
                    "text": "R is a hydrogen atom",
                    "fontFamily": "Times New Roman",
                    "fontSize": 7.0,
                    "fontWeight": 400,
                    "fontStyle": "normal",
                    "script": "normal",
                    "fill": "#000000"
                }]
            }
        }],
        "resources": {}
    }))
    .expect("preserved text document");

    let primitives = render_document(&document);
    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text {
            object_id,
            preserve_lines: true,
            ..
        } if object_id.as_deref() == Some("obj_text")
    )));
    let svg = document_to_svg(&document);
    assert!(svg.contains(">R</tspan>"), "{svg}");
    assert!(svg.contains("> </tspan>"), "{svg}");
    assert!(svg.contains(">hydrogen</tspan>"), "{svg}");
}

#[test]
fn free_text_without_authored_box_uses_face_aware_ink_bounds() {
    let text = "3-methoxy-4-((4-(trifluoromethyl)benzyl)oxy)benzaldehyde";
    let source = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionFont="4" CaptionSize="9.33333">
  <fonttable><font id="4" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <t id="2" p="537.279968 128.929993" Justification="Left" InterpretChemically="no">
      <s font="4" size="9.33333" face="1">{text}</s>
    </t>
  </page>
</CDXML>"#
    );
    let document =
        parse_cdxml_document(&source, Some("face-aware free text")).expect("CDXML parses");
    let object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("free text imports");
    assert_eq!(
        object
            .meta
            .pointer("/import/cdxml/authoredBoundingBox")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    let local_box = object
        .payload
        .extra
        .get("box")
        .and_then(serde_json::Value::as_array)
        .expect("derived box");
    assert_eq!(local_box[0].as_f64(), Some(0.35));
    assert_eq!(local_box[2].as_f64(), Some(256.94));

    let primitive = render_document(&document)
        .into_iter()
        .find(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Text {
                    object_id: Some(object_id),
                    ..
                } if object_id == &object.id
            )
        })
        .expect("text primitive");
    let bounds = render_primitives_bounds(std::iter::once(&primitive)).expect("text visual bounds");
    assert!((bounds[2] - 794.574794).abs() < 1.0e-5, "{bounds:?}");

    let svg = document_to_svg(&document);
    let view_box = svg
        .split_once("viewBox=\"")
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(value, _)| {
            value
                .split_whitespace()
                .map(|part| part.parse::<f64>().expect("viewBox number"))
                .collect::<Vec<_>>()
        })
        .expect("root viewBox");
    let view_box_right = view_box[0] + view_box[2];
    assert!(
        view_box_right >= bounds[2] + 7.999,
        "{view_box:?} versus {bounds:?}"
    );
    let exported = document_to_cdxml(&document);
    let text_tag = exported
        .split_once("<t ")
        .and_then(|(_, tail)| tail.split_once('>'))
        .map(|(tag, _)| tag)
        .expect("exported text tag");
    assert!(!text_tag.contains("BoundingBox="), "{text_tag}");
    let reopened =
        parse_cdxml_document(&exported, Some("reopened unboxed text")).expect("reopen export");
    let reopened_object = reopened
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("reopened text");
    assert_eq!(
        reopened_object.payload.extra.get("box"),
        object.payload.extra.get("box")
    );
}

#[test]
fn authored_text_box_is_unioned_with_real_ink_instead_of_clipping_it() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionFont="4" CaptionSize="10">
  <fonttable><font id="4" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <t id="2" p="100 50" BoundingBox="100 40 110 55"
       Justification="Left" InterpretChemically="no">
      <s font="4" size="10" face="1">Arial bold text extends beyond its authored box</s>
    </t>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(source, Some("narrow authored text box")).expect("CDXML parses");
    let object = document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("free text imports");
    assert_eq!(
        object
            .meta
            .pointer("/import/cdxml/authoredBoundingBox")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        object.payload.extra.get("box"),
        Some(&json!([0.0, 0.0, 10.0, 15.0]))
    );
    let primitive = render_document(&document)
        .into_iter()
        .find(|primitive| matches!(primitive, RenderPrimitive::Text { .. }))
        .expect("text primitive");
    let visual = render_primitives_bounds(std::iter::once(&primitive)).expect("text visual bounds");
    assert!(
        visual[2] > 290.0,
        "real glyph ink must outrun the 10 pt authored box: {visual:?}"
    );
    let svg = document_to_svg(&document);
    let view_box = svg
        .split_once("viewBox=\"")
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(value, _)| {
            value
                .split_whitespace()
                .map(|part| part.parse::<f64>().expect("viewBox number"))
                .collect::<Vec<_>>()
        })
        .expect("root viewBox");
    assert!(view_box[0] + view_box[2] >= visual[2] + 7.999);
    let exported = document_to_cdxml(&document);
    let text_tag = exported
        .split_once("<t ")
        .and_then(|(_, tail)| tail.split_once('>'))
        .map(|(tag, _)| tag)
        .expect("exported text tag");
    assert!(text_tag.contains("BoundingBox="), "{text_tag}");
}

#[test]
fn cdxml_free_text_renders_from_authored_p_anchor_for_every_justification() {
    for (justification, bbox, point, expected_anchor) in [
        ("Left", "100 40 150 70", "90 55", "start"),
        ("Center", "100 40 150 70", "120 55", "middle"),
        ("Right", "100 40 150 70", "160 55", "end"),
    ] {
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CaptionSize="10" CaptionFace="0" CaptionFont="3">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <t id="2" p="{point}" BoundingBox="{bbox}" CaptionJustification="{justification}"
       LineHeight="12" LineStarts="4 8">
      <s font="3" size="10" color="0" face="0">one&#10;two</s>
    </t>
  </page>
</CDXML>"#
        );
        let document = parse_cdxml_document(&cdxml, Some(justification))
            .expect("free text anchor fixture should import");
        let object = document
            .objects
            .iter()
            .find(|object| object.object_type == "text")
            .expect("text object");
        let point_x = point
            .split_whitespace()
            .next()
            .expect("point x")
            .parse::<f64>()
            .expect("numeric point x");
        let imported_offset = object
            .payload
            .extra
            .get("anchorOffsetX")
            .and_then(serde_json::Value::as_f64)
            .expect("CDXML p-to-box anchor offset");
        assert_close(object.transform.translate[0] + imported_offset, point_x);

        let rendered = render_document(&document)
            .into_iter()
            .filter_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    object_id,
                    x,
                    y,
                    text_anchor,
                    ..
                } if object_id.as_deref() == Some(object.id.as_str()) => Some((x, y, text_anchor)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rendered.len(), 2, "{justification}");
        assert!(
            rendered
                .iter()
                .all(|(x, _, _)| (*x - point_x).abs() < 0.001),
            "{justification}: all lines must use CDXML p.x={point_x}, got {rendered:?}"
        );
        assert_close(rendered[0].1, 55.0);
        assert_close(rendered[1].1, 67.0);
        assert!(
            rendered
                .iter()
                .all(|(_, _, anchor)| anchor.as_deref() == Some(expected_anchor)),
            "{justification}: {rendered:?}"
        );
    }
}

#[test]
fn text_anchor_offset_is_independent_of_line_preservation() {
    for preserve_lines in [true, false] {
        let document: ChemSemaDocument = serde_json::from_value(json!({
            "format": { "name": "chemsema", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "text anchor offset",
                "page": { "width": 200.0, "height": 100.0, "background": "#ffffff" }
            },
            "styles": {
                "style_text": {
                    "kind": "text",
                    "fontFamily": "Arial",
                    "fontSize": 10.0,
                    "fill": "#000000"
                }
            },
            "objects": [{
                "id": "obj_text",
                "type": "text",
                "visible": true,
                "zIndex": 1,
                "transform": {
                    "translate": [50.0, 20.0],
                    "rotate": 23.0,
                    "scale": [1.0, 1.0]
                },
                "styleRef": "style_text",
                "payload": {
                    "text": "anchor",
                    "box": [0.0, 0.0, 80.0, 20.0],
                    "align": "left",
                    "fontSize": 10.0,
                    "lineHeight": 12.0,
                    "anchorOffsetX": -7.5,
                    "baselineOffset": 8.0,
                    "preserveLines": preserve_lines
                }
            }],
            "resources": {}
        }))
        .expect("text document");
        let primitive = render_document(&document)
            .into_iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    object_id,
                    x,
                    rotate,
                    rotate_center,
                    ..
                } if object_id.as_deref() == Some("obj_text") => Some((x, rotate, rotate_center)),
                _ => None,
            })
            .expect("rendered text");
        assert_close(primitive.0, 42.5);
        assert_close(primitive.1, 23.0);
        assert_eq!(primitive.2, Some(Point::new(50.0, 20.0)));
    }
}

#[test]
fn bracket_object_tag_text_renders_from_owning_layout_anchor() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "bracket label anchor",
            "page": { "width": 200.0, "height": 100.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text": {
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": 7.5,
                "fill": "#000000"
            }
        },
        "objects": [{
            "id": "obj_text",
            "type": "text",
            "visible": true,
            "zIndex": 1,
            "transform": {
                "translate": [81.41, 67.37],
                "rotate": 0.0,
                "scale": [1.0, 1.0]
            },
            "styleRef": "style_text",
            "meta": { "role": "bracket_usage" },
            "payload": {
                "text": "2",
                "box": [2.34, 0.0, 3.62, 5.27],
                "align": "left",
                "fontSize": 7.5,
                "lineHeight": 6.68,
                "anchorOffsetX": 2.1,
                "baselineOffset": 5.27,
                "preserveLines": true
            }
        }],
        "resources": {}
    }))
    .expect("bracket label document");

    let x = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text { object_id, x, .. }
                if object_id.as_deref() == Some("obj_text") =>
            {
                Some(x)
            }
            _ => None,
        })
        .expect("rendered bracket label");
    assert_close(x, 81.41);
}

#[test]
fn attached_text_p_is_authoritative_only_without_explicit_label_alignment() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="7">
  <fonttable><font id="3" charset="iso-8859-1" name="Times New Roman"/></fonttable>
  <page id="1">
    <fragment id="2" BoundingBox="20 20 170 80">
      <n id="1" p="50 50" NodeType="Nickname" AS="N">
        <t p="38 54" BoundingBox="37.7 48 48 54.2"
           LabelJustification="Right" Justification="Right">
          <s font="3" size="7" face="96">tBu</s>
        </t>
      </n>
      <n id="2" p="100 50" NodeType="Nickname" AS="N">
        <t p="70 54" BoundingBox="59 48 70 54.2"
           LabelJustification="Right" Justification="Right" LabelAlignment="Right">
          <s font="3" size="7" face="96">HCO</s>
        </t>
      </n>
      <n id="4" p="150 50" Element="8" AS="N">
        <t p="120 54" BoundingBox="112 48 120 54.2"
           LabelJustification="Right" Justification="Right">
          <s font="3" size="7" face="96">O</s>
        </t>
      </n>
      <b id="3" B="1" E="2"/>
      <b id="5" B="2" E="4"/>
    </fragment>
  </page>
</CDXML>"##;
    let document =
        parse_cdxml_document(cdxml, Some("attached anchor authority")).expect("CDXML parses");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("molecule fragment");
    let authored_node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "1")
        .expect("authored attached node");
    let authored = authored_node
        .label
        .as_ref()
        .expect("authored attached label");
    let authored_offset: [f64; 2] = serde_json::from_value(
        authored
            .meta
            .pointer("/import/cdxml/textOffsetFromNode")
            .cloned()
            .expect("authored text offset"),
    )
    .expect("numeric authored text offset");
    let authored_position = authored.position.expect("authored position");
    assert_close(
        authored_position[0],
        authored_node.position[0] + authored_offset[0],
    );
    assert_close(
        authored_position[1],
        authored_node.position[1] + authored_offset[1],
    );

    let automatic_node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "2")
        .expect("automatic attached node");
    let automatic = automatic_node
        .label
        .as_ref()
        .expect("automatic attached label");
    let ignored_offset: [f64; 2] = serde_json::from_value(
        automatic
            .meta
            .pointer("/import/cdxml/textOffsetFromNode")
            .cloned()
            .expect("automatic source text offset"),
    )
    .expect("numeric automatic source text offset");
    let automatic_position = automatic.position.expect("automatic position");
    let ignored_position = [
        automatic_node.position[0] + ignored_offset[0],
        automatic_node.position[1] + ignored_offset[1],
    ];
    assert!(
        (automatic_position[0] - ignored_position[0]).abs() > 0.01
            || (automatic_position[1] - ignored_position[1]).abs() > 0.01,
        "LabelAlignment must make ChemDraw's node-relative layout authoritative: \
         automatic={automatic_position:?}, authored={ignored_position:?}"
    );

    let element_node = fragment
        .nodes
        .iter()
        .find(|node| node.id == "4")
        .expect("element node");
    let element = element_node.label.as_ref().expect("element label");
    let element_offset: [f64; 2] = serde_json::from_value(
        element
            .meta
            .pointer("/import/cdxml/textOffsetFromNode")
            .cloned()
            .expect("element source text offset"),
    )
    .expect("numeric element source text offset");
    let element_position = element.position.expect("element position");
    let ignored_element_position = [
        element_node.position[0] + element_offset[0],
        element_node.position[1] + element_offset[1],
    ];
    assert!(
        (element_position[0] - ignored_element_position[0]).abs() > 0.01
            || (element_position[1] - ignored_element_position[1]).abs() > 0.01,
        "ordinary elements stay atom-laid-out without LabelAlignment"
    );

    let exported = document_to_cdxml(&document);
    assert_eq!(
        exported.matches("LabelAlignment=").count(),
        2,
        "export must preserve the absent-vs-explicit alignment branch: {exported}"
    );
}
