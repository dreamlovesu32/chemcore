use super::*;

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
