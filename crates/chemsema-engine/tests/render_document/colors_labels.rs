use super::*;

#[test]
fn engine_reports_document_colors_from_document_model() {
    let mut engine = Engine::new();
    engine
        .load_document_json(
            &json!({
                "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
                "document": {
                    "id": "doc_colors",
                    "title": "colors",
                    "page": { "width": 200.0, "height": 160.0, "background": "#fff" }
                },
                "styles": {
                    "style_molecule_default": {
                        "kind": "molecule",
                        "stroke": "rgb(1, 2, 3)",
                        "fill": "#abc",
                        "strokeWidth": 0.85,
                        "fontSize": 11.0
                    }
                },
                "objects": [{
                    "id": "obj_molecule",
                    "type": "molecule",
                    "styleRef": "style_molecule_default",
                    "payload": { "resourceRef": "mol" }
                }],
                "resources": {
                    "mol": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemsema.molecule.fragment2d",
                        "data": {
                            "schema": "chemsema.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 200.0, 160.0],
                            "nodes": [{
                                "id": "n1",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [20.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": { "text": "Me", "fill": "#00ff00" }
                            }, {
                                "id": "n2",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [50.0, 20.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "b1",
                                "begin": "n1",
                                "end": "n2",
                                "order": 1,
                                "stroke": "#ff00ff",
                                "strokeWidth": 0.85
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

    let colors = engine.document_colors();
    assert!(colors.contains(&"#ffffff".to_string()));
    assert!(colors.contains(&"#010203".to_string()));
    assert!(colors.contains(&"#aabbcc".to_string()));
    assert!(colors.contains(&"#00ff00".to_string()));
    assert!(colors.contains(&"#ff00ff".to_string()));
    assert_eq!(colors.iter().filter(|color| *color == "#ffffff").count(), 1);
}

#[test]
fn export_cdxml_emits_chemdraw_document_with_native_fragment() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 40.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [70.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "sourceText": "O",
                    "position": [70.0, 40.0],
                    "box": [66.0, 28.0, 78.0, 44.0],
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "attachment": "node",
                    "anchor": "start"
                }
            },
            {
                "id": "n3",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 80.0],
                "charge": 0,
                "numHydrogens": 0,
                "isPlaceholder": true,
                "label": {
                    "text": "CF3",
                    "sourceText": "CF3",
                    "position": [30.0, 80.0],
                    "box": [30.0, 70.0, 47.4, 82.5],
                    "fontSize": 10.0,
                    "fill": "#d61f1f",
                    "attachment": "node",
                    "anchor": "start"
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "center", "frozen": false },
                "strokeWidth": 0.6,
                "bondSpacing": 18.0,
                "marginWidth": 1.6
            },
            {
                "id": "b2",
                "begin": "n1",
                "end": "n3",
                "order": 1,
                "stereo": { "kind": "solid-wedge", "wideEnd": "end" },
                "strokeWidth": 0.6
            }
        ]),
    );

    let cdxml = document_to_cdxml(&document);

    assert!(cdxml.contains("<!DOCTYPE CDXML"));
    assert!(cdxml.contains("<CDXML"));
    assert!(cdxml.contains("CreationProgram=\"ChemSema\""));
    assert!(cdxml.contains("LabelFace=\"96\""));
    assert!(cdxml.contains("CaptionFace=\"0\""));
    assert!(
        cdxml.contains("color=\"3\" bgcolor=\"1\""),
        "known black should reuse its color-table id: {cdxml}"
    );
    assert!(cdxml.contains("<page"));
    assert!(cdxml.contains("HeaderPosition=\"36\""));
    assert!(cdxml.contains("<fragment"));
    assert!(cdxml.contains("Order=\"2\""));
    assert!(
        !cdxml.contains(" BS="),
        "unspecified bond stereo must remain absent instead of becoming explicit BS=N"
    );
    assert!(cdxml.contains("BondSpacing=\"18\""));
    assert!(cdxml.contains("MarginWidth=\"1.6\""));
    assert!(cdxml.contains("NodeType=\"Nickname\""));
    assert!(cdxml.contains("UTF8Text=\"CF3\""));
    assert!(!cdxml.contains("<t font="));
    assert!(!cdxml.contains("<t size="));
    assert!(!cdxml.contains("<t color="));
    assert!(cdxml.contains("<s font=\"3\" size=\"10\" color=\"3\""));

    let roundtripped =
        parse_cdxml_document(&cdxml, Some("roundtrip")).expect("export should parse");
    let fragment = roundtripped
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("roundtrip should create a molecule fragment");
    assert_eq!(fragment.nodes.len(), 3);
    assert_eq!(fragment.bonds.len(), 2);
    assert!(fragment.bonds.iter().any(|bond| bond.order == 2));
    assert!(fragment
        .nodes
        .iter()
        .any(|node| node.label.as_ref().is_some_and(|label| label.text == "CF3")));
    let cf3_label = fragment
        .nodes
        .iter()
        .find_map(|node| node.label.as_ref().filter(|label| label.text == "CF3"))
        .expect("CF3 label should roundtrip");
    assert_eq!(cf3_label.fill.as_deref(), Some("#d61f1f"));
}

#[test]
fn export_cdxml_preserves_text_run_style_across_reimport() {
    let document: ChemSemaDocument = serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "text style",
            "page": { "width": 200.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_text_default": {
                "kind": "text",
                "fontFamily": "Times New Roman",
                "fontSize": 16.0,
                "fill": "#d61f1f",
                "stroke": null
            }
        },
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [24.0, 32.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_text_default",
            "payload": {
                "text": "RedBlue",
                "box": [0.0, 0.0, 80.0, 22.0],
                "align": "left",
                "fontSize": 16.0,
                "runs": [
                    {
                        "text": "Red",
                        "fontFamily": "Times New Roman",
                        "fontSize": 16.0,
                        "fill": "#d61f1f",
                        "fontWeight": 700,
                        "script": "normal"
                    },
                    {
                        "text": "Blue",
                        "fontFamily": "Arial",
                        "fontSize": 12.0,
                        "fill": "#1b32d8",
                        "fontStyle": "italic",
                        "script": "normal"
                    }
                ]
            }
        }],
        "resources": {}
    }))
    .expect("document should deserialize");

    let cdxml = document_to_cdxml(&document);
    let roundtripped =
        parse_cdxml_document(&cdxml, Some("text style")).expect("export should parse");
    let text_object = roundtripped
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should roundtrip");
    let runs: Vec<chemsema_engine::LabelRun> = serde_json::from_value(
        text_object
            .payload
            .extra
            .get("runs")
            .cloned()
            .expect("text runs should roundtrip"),
    )
    .expect("text runs should deserialize");

    assert_eq!(runs[0].text, "Red");
    assert_eq!(runs[0].fill.as_deref(), Some("#d61f1f"));
    assert_eq!(runs[0].font_family.as_deref(), Some("Times New Roman"));
    assert_eq!(runs[0].font_weight, Some(700));
    assert_eq!(runs[1].text, "Blue");
    assert_eq!(runs[1].fill.as_deref(), Some("#1b32d8"));
    assert_eq!(runs[1].font_family.as_deref(), Some("Arial"));
    assert_eq!(runs[1].font_style.as_deref(), Some("italic"));
}

#[test]
fn cdxml_single_atom_fragments_roundtrip_as_chemical_objects() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 80" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 160 80">
    <fragment id="2" BoundingBox="10 10 30 30"><n id="3" p="20 20" Element="1"/></fragment>
    <fragment id="4" BoundingBox="50 10 70 30"><n id="5" p="60 20" Element="19" Charge="1"/></fragment>
    <fragment id="6" BoundingBox="90 10 110 30"><n id="7" p="100 20" Element="55" Charge="1"/></fragment>
  </page>
</CDXML>"#;

    let imported = parse_cdxml_document(source, Some("single atoms")).expect("source imports");
    let exported = document_to_cdxml(&imported);
    let reopened =
        parse_cdxml_document(&exported, Some("single atoms reopened")).expect("export imports");
    let mut atoms: Vec<_> = reopened
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .filter_map(|fragment| fragment.nodes.first())
        .map(|node| (node.atomic_number, node.charge))
        .collect();
    atoms.sort_unstable();

    assert_eq!(atoms, vec![(1, 0), (19, 1), (55, 1)]);
    assert_eq!(
        reopened
            .objects
            .iter()
            .filter(|object| object.object_type == "molecule")
            .count(),
        3
    );
    assert!(!reopened
        .objects
        .iter()
        .any(|object| object.object_type == "text"));
}

#[test]
fn cdxml_custom_element_labels_preserve_atomic_identity() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 80" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 160 80">
    <fragment id="2" BoundingBox="10 10 130 50">
      <n id="3" p="20 30" Element="8" Charge="-1"><t p="16 34" BoundingBox="16 22 26 36" UTF8Text="O"><s font="3" size="10">O</s></t></n>
      <n id="4" p="70 30" Element="7"><t p="62 34" BoundingBox="62 18 78 38" UTF8Text="N&#10;H"><s font="3" size="10">N&#10;H</s></t></n>
      <n id="5" p="120 30" Element="62"><t p="104 34" BoundingBox="104 22 136 36" UTF8Text="SmIII"><s font="3" size="10">SmIII</s></t></n>
      <b id="6" B="3" E="4"/><b id="7" B="4" E="5"/>
    </fragment>
  </page>
</CDXML>"#;

    let imported = parse_cdxml_document(source, Some("custom elements")).expect("source imports");
    let exported = document_to_cdxml(&imported);
    assert!(exported.contains("Element=\"8\""));
    assert!(exported.contains("Element=\"7\""));
    assert!(exported.contains("Element=\"62\""));
    let reopened =
        parse_cdxml_document(&exported, Some("custom elements reopened")).expect("export imports");
    let fragment = reopened
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("molecule survives");
    let atomic_numbers: Vec<_> = fragment
        .nodes
        .iter()
        .map(|node| node.atomic_number)
        .collect();

    assert_eq!(atomic_numbers, vec![8, 7, 62]);
    assert_eq!(fragment.nodes[0].charge, -1);
    assert!(fragment.nodes.iter().all(|node| !node.is_placeholder));
}

#[test]
fn cdxml_headless_arrow_remains_an_arrow() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 80">
  <page id="1" BoundingBox="0 0 160 80">
    <arrow id="2" Tail3D="20 40 0" Head3D="140 40 0" ArrowheadHead="None" ArrowheadTail="None"/>
  </page>
</CDXML>"#;

    let imported = parse_cdxml_document(source, Some("headless arrow")).expect("source imports");
    let exported = document_to_cdxml(&imported);
    assert!(
        exported.contains("<arrow"),
        "arrow identity must survive: {exported}"
    );
    assert!(!exported.contains("GraphicType=\"Line\""));
    let reopened =
        parse_cdxml_document(&exported, Some("headless arrow reopened")).expect("export imports");
    let arrow = reopened
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow survives");
    let arrow_head = arrow
        .payload
        .extra
        .get("arrowHead")
        .expect("headless arrow keeps arrow payload");
    assert_eq!(
        arrow_head.get("head").and_then(|value| value.as_str()),
        Some("none")
    );
    assert_eq!(
        arrow_head.get("tail").and_then(|value| value.as_str()),
        Some("none")
    );
}

#[test]
fn cdxml_round_brackets_do_not_gain_groups_or_expand() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 160 100">
  <page id="1" BoundingBox="0 0 160 100">
    <graphic id="2" GraphicType="Bracket" BracketType="Round" BoundingBox="30 80 30 20"/>
    <graphic id="3" GraphicType="Bracket" BracketType="Round" BoundingBox="130 20 130 80"/>
  </page>
</CDXML>"#;

    let first = parse_cdxml_document(source, Some("round brackets")).expect("source imports");
    let first_group = first
        .objects
        .iter()
        .find(|object| object_is_bracket_group(object))
        .expect("bracket group exists");
    let first_positions: Vec<_> = first_group
        .children
        .iter()
        .map(|child| child.transform.translate)
        .collect();
    let exported = document_to_cdxml(&first);
    assert!(
        !exported.contains("<group"),
        "synthetic group must not be serialized"
    );
    let second = parse_cdxml_document(&exported, Some("round brackets second"))
        .expect("first export imports");
    let second_export = document_to_cdxml(&second);
    let third = parse_cdxml_document(&second_export, Some("round brackets third"))
        .expect("second export imports");

    for document in [&second, &third] {
        let groups: Vec<_> = document
            .objects
            .iter()
            .filter(|object| object_is_bracket_group(object))
            .collect();
        assert_eq!(groups.len(), 1);
        let positions: Vec<_> = groups[0]
            .children
            .iter()
            .map(|child| child.transform.translate)
            .collect();
        assert_eq!(positions, first_positions);
    }
}

#[test]
fn cdxml_drops_bonds_whose_normalized_endpoint_is_missing() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 100 60">
  <page id="1" BoundingBox="0 0 100 60">
    <fragment id="2" BoundingBox="10 10 90 50">
      <n id="3" p="20 30"/><n id="4" p="60 30"/><n id="5" p="not-a-point"/>
      <b id="6" B="3" E="4"/><b id="7" B="4" E="5"/>
    </fragment>
  </page>
</CDXML>"#;

    let document = parse_cdxml_document(source, Some("missing endpoint")).expect("source imports");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("valid component survives");
    assert_eq!(fragment.nodes.len(), 2);
    assert_eq!(fragment.bonds.len(), 1);
    assert_eq!(fragment.bonds[0].begin, "3");
    assert_eq!(fragment.bonds[0].end, "4");
}

#[test]
fn cdxml_import_export_import_is_render_stable_for_tmp_fixtures() {
    let fixtures = [
        "molecule.cdxml",
        "shape.cdxml",
        "kuohao.cdxml",
        "duibi.cdxml",
        "color.cdxml",
        "assets-acs.cdxml",
        "arrows-acs.cdxml",
    ];
    if fixtures
        .iter()
        .any(|fixture| !cdxml_fixture_exists(fixture))
    {
        eprintln!("skipping external CDXML roundtrip render suite; fixture set is incomplete");
        return;
    }
    for fixture in fixtures {
        let cdxml = read_cdxml_fixture(fixture);
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let exported = document_to_cdxml(&imported);
        let reimported =
            parse_cdxml_document(&exported, Some(fixture)).expect("export should reimport");

        assert_eq!(
            render_roundtrip_signature(&reimported),
            render_roundtrip_signature(&imported),
            "{fixture} should be stable across import/export/import",
        );
    }
}

#[test]
fn cdxml_import_export_import_is_svg_stable_for_tmp_fixtures() {
    let fixtures = [
        "molecule.cdxml",
        "shape.cdxml",
        "kuohao.cdxml",
        "duibi.cdxml",
        "color.cdxml",
        "assets-acs.cdxml",
        "arrows-acs.cdxml",
    ];
    if fixtures
        .iter()
        .any(|fixture| !cdxml_fixture_exists(fixture))
    {
        eprintln!("skipping external CDXML roundtrip SVG suite; fixture set is incomplete");
        return;
    }
    for fixture in fixtures {
        let cdxml = read_cdxml_fixture(fixture);
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let exported = document_to_cdxml(&imported);
        let reimported =
            parse_cdxml_document(&exported, Some(fixture)).expect("export should reimport");

        assert_eq!(
            document_to_svg(&reimported),
            document_to_svg(&imported),
            "{fixture} should keep the same SVG across import/export/import",
        );
    }
}

#[test]
fn public_cdxml_fixture_svg_golden_snapshots_match() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    let fixture_dir = repo_root.join("fixtures").join("cdxml");
    let expected_dir = repo_root.join("fixtures").join("expected").join("svg");
    let mut fixtures = std::fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| panic!("{}: {error}", fixture_dir.display()))
        .map(|entry| entry.expect("fixture entry should be readable").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("cdxml"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "public CDXML fixture directory should contain regression cases"
    );

    for fixture_path in fixtures {
        let stem = fixture_path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("fixture file should have a UTF-8 stem");
        let expected_path = expected_dir.join(format!("{stem}.svg"));
        let cdxml = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture_path.display()));
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(&cdxml)
            .unwrap_or_else(|error| panic!("{stem}: {error}"));
        let actual = normalize_svg_snapshot(&engine.document_svg());
        let expected = normalize_svg_snapshot(
            &std::fs::read_to_string(&expected_path)
                .unwrap_or_else(|error| panic!("{}: {error}", expected_path.display())),
        );

        assert_eq!(actual, expected, "{stem} SVG golden snapshot changed");
    }
}

#[test]
fn cdxml_exported_arrow_fixtures_are_stable_after_first_save() {
    for fixture in ["assets-acs.cdxml", "arrows-acs.cdxml"] {
        let Some(cdxml) = read_optional_cdxml_fixture(fixture) else {
            continue;
        };
        let imported = parse_cdxml_document(&cdxml, Some(fixture)).expect("fixture should import");
        let first_export = document_to_cdxml(&imported);
        let first_reimport =
            parse_cdxml_document(&first_export, Some(fixture)).expect("first export should import");
        let second_export = document_to_cdxml(&first_reimport);
        let second_reimport = parse_cdxml_document(&second_export, Some(fixture))
            .expect("second export should import");

        assert_eq!(
            render_roundtrip_signature(&second_reimport),
            render_roundtrip_signature(&first_reimport),
            "{fixture} should not drift after the first save",
        );
    }
}

#[test]
fn export_svg_emits_rendered_document_primitives() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [30.0, 40.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [70.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "sourceText": "O",
                    "position": [70.0, 40.0],
                    "box": [66.0, 28.0, 78.0, 44.0],
                    "fontSize": 10.0,
                    "fill": "#000000",
                    "attachment": "node",
                    "anchor": "start"
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "double": { "placement": "center", "frozen": false },
                "strokeWidth": 0.6,
                "bondSpacing": 18.0
            }
        ]),
    );

    let svg = document_to_svg(&document);

    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("viewBox=\""));
    assert!(svg.contains("<polygon"));
    assert!(svg.contains("<text"));
    assert!(svg.contains(">O</"));
    assert!(!svg.contains("document-knockout"));
}

#[test]
fn load_cdxml_document_preserves_display_fragments_for_editing_hit_tests() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="10" BoundingBox="10 10 40 20">
      <n id="11" p="10 15"/>
      <n id="12" p="40 15"/>
      <b id="13" B="11" E="12" Order="1"/>
    </fragment>
    <fragment id="20" BoundingBox="70 10 100 20">
      <n id="21" p="70 15"/>
      <n id="22" p="100 15"/>
      <b id="23" B="21" E="22" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 2);
    let fragments = document.editable_fragments();
    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].fragment.bonds.len(), 1);
    assert_eq!(fragments[1].fragment.bonds.len(), 1);
    assert!(!document
        .objects
        .iter()
        .any(|object| object.id == "obj_cdxml_merged_molecule"));
    assert!(hit_test_bond_center(
        document,
        Point::new(85.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
}

#[test]
fn load_cdxml_document_splits_disconnected_components_inside_one_fragment() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 140 80">
    <fragment id="10" BoundingBox="10 10 112 20">
      <n id="11" p="10 15"/>
      <n id="12" p="40 15"/>
      <b id="13" B="11" E="12" Order="1"/>
      <n id="21" p="82 15"/>
      <n id="22" p="112 15"/>
      <b id="23" B="21" E="22" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 2);
    let fragments = document.editable_fragments();
    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[0].fragment.nodes.len(), 2);
    assert_eq!(fragments[0].fragment.bonds.len(), 1);
    assert_eq!(fragments[1].fragment.nodes.len(), 2);
    assert_eq!(fragments[1].fragment.bonds.len(), 1);
    assert_eq!(document.resources.len(), 2);
    assert!(document.resources.contains_key("mol_001"));
    assert!(document.resources.contains_key("mol_002"));
    assert!(hit_test_bond_center(
        document,
        Point::new(25.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
    assert!(hit_test_bond_center(
        document,
        Point::new(97.0 * CDXML_EDIT_SCALE, 15.0 * CDXML_EDIT_SCALE),
        30.0 * CDXML_EDIT_SCALE
    )
    .is_some());
}

#[test]
fn load_cdxml_document_preserves_figure2_display_fragments() {
    let Some(cdxml) = read_optional_cdxml_fixture("figure2.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("figure2 should load into editing engine");
    let document = &engine.state().document;
    let molecule_objects = document
        .objects
        .iter()
        .filter(|object| object.object_type == "molecule")
        .count();
    assert_eq!(molecule_objects, 7);
    assert_eq!(document.editable_fragments().len(), 7);
    assert!(!document
        .objects
        .iter()
        .any(|object| object.id == "obj_cdxml_merged_molecule"));
    assert!(!document.resources.contains_key("mol_cdxml_merged"));
}

#[test]
fn render_cdxml_fragment_node_labels_interleave_with_external_graphics_by_source_z() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="10" BoundingBox="0 0 30 20" Z="5">
      <n id="11" p="5 10" Z="5"/>
      <n id="12" p="25 10" Z="5"/>
      <b id="13" B="11" E="12" Order="1" Z="30"/>
    </fragment>
    <fragment id="20" BoundingBox="40 40 85 65" Z="5">
      <n id="21" p="52 50" Z="30" Element="18">
        <t p="54 54" BoundingBox="44 44 54 54" LabelJustification="Right">
          <s font="3" size="10" color="0">Ar</s>
        </t>
      </n>
      <n id="22" p="74 50" Z="10"/>
      <b id="23" B="21" E="22" Order="1" Z="10"/>
    </fragment>
    <graphic id="30"
      BoundingBox="58.64 50 50 50"
      Z="20"
      GraphicType="Orbital"
      OvalType="Circle Shaded"
      OrbitalType="sShaded"
      Center3D="50 50 0"
      MajorAxisEnd3D="58.64 50 0"
      MinorAxisEnd3D="50 58.64 0"/>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("layered cdxml should load");
    let document = &engine.state().document;
    let primitives = render_document(document);
    let orbital_last = primitives
        .iter()
        .enumerate()
        .filter(|(_, primitive)| {
            render_primitive_object_id(primitive) == Some("obj_shape_orbital_001")
        })
        .map(|(index, _)| index)
        .max()
        .expect("orbital should render");
    let ar_text = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_text_content(primitive).as_deref() == Some("Ar")).then_some(index)
        })
        .expect("Ar node label should render");
    let high_bond = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_bond_id(primitive) == Some("13")).then_some(index)
        })
        .unwrap_or_else(|| {
            panic!(
                "high-Z bond should render; bond ids: {:?}",
                primitives
                    .iter()
                    .filter_map(render_primitive_bond_id)
                    .collect::<Vec<_>>()
            )
        });

    assert!(
        orbital_last < ar_text,
        "source node Z should draw the Ar label above the external orbital"
    );
    assert!(
        orbital_last < high_bond,
        "source bond Z should draw the imported bond above the external orbital"
    );
}

#[test]
fn render_cdxml_group_children_keep_source_z_against_external_symbols() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BondSpacing="18">
  <page id="1" BoundingBox="0 0 80 80">
    <group id="10" Z="30">
      <graphic id="11"
        BoundingBox="40 20 40 34"
        Z="1"
        GraphicType="Orbital"
        OrbitalType="lobe"/>
    </group>
    <graphic id="12"
      BoundingBox="40 26 40 38"
      Z="2"
      GraphicType="Symbol"
      SymbolType="Electron"/>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("group z")).expect("grouped orbital cdxml should parse");
    let primitives = render_document(&document);
    let lobe_last = primitives
        .iter()
        .enumerate()
        .filter(|(_, primitive)| {
            render_primitive_object_id(primitive) == Some("obj_shape_orbital_001")
        })
        .map(|(index, _)| index)
        .max()
        .expect("grouped lobe should render");
    let electron_first = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| {
            (render_primitive_object_id(primitive) == Some("obj_symbol_001")).then_some(index)
        })
        .expect("external electron should render");

    assert!(
        lobe_last < electron_first,
        "CDXML group Z must not lift the white lobe fill above the higher-Z electron"
    );
}

#[test]
fn cdxml_generic_r_prime_labels_do_not_render_invalid_markers() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" BondSpacing="18">
  <page id="1" BoundingBox="0 0 120 50">
    <fragment id="10" BoundingBox="0 0 100 40">
      <n id="11" p="10 20" NodeType="GenericNickname" Z="1">
        <t p="10 20" BoundingBox="4 12 14 22"><s font="3" size="10">R&apos;</s></t>
      </n>
      <n id="12" p="40 20" Z="2"/>
      <n id="13" p="70 20" NodeType="GenericNickname" Z="3">
        <t p="70 20" BoundingBox="64 12 76 22"><s font="3" size="10">R&apos;&apos;</s></t>
      </n>
      <b id="14" B="11" E="12" Order="1" Z="4"/>
      <b id="15" B="12" E="13" Order="1" Z="5"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("r groups")).expect("generic R labels should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| match &resource.data {
            ResourceData::Fragment(fragment) => Some(fragment),
            _ => None,
        })
        .expect("fragment should be imported");
    for text in ["R'", "R''"] {
        let node = fragment
            .nodes
            .iter()
            .find(|node| {
                node.label
                    .as_ref()
                    .is_some_and(|label| label.source_text.as_deref() == Some(text))
            })
            .unwrap_or_else(|| panic!("{text} node should exist"));
        assert!(
            node.is_placeholder,
            "{text} GenericNickname should import as a placeholder"
        );
        assert_ne!(
            node.meta
                .get("labelRecognition")
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid"),
            "{text} should not be an invalid chemical label"
        );
        assert_ne!(
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("labelRecognition"))
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid"),
            "{text} label should not be an invalid chemical label"
        );
    }
    assert!(!render_document(&document).iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            stroke: Some(stroke),
            ..
        } if stroke == "#d32f2f"
    )));
}

#[test]
fn parse_cdxml_skips_cached_fragments_inside_placeholder_nodes() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="18" LineWidth="0.6" LabelSize="10">
  <page id="1" BoundingBox="0 0 140 60">
    <fragment id="visible" BoundingBox="0 0 120 50">
      <n id="n1" p="20 25" NodeType="Nickname">
        <fragment id="cached_bn" BoundingBox="-10 10 40 55">
          <n id="c1" p="0 20"/>
          <n id="c2" p="20 20"/>
          <b id="cb1" B="c1" E="c2"/>
        </fragment>
        <t p="20 29" BoundingBox="8 18 20 30" LabelJustification="Right" Justification="Right">
          <s font="3" size="10" face="97">Bn</s>
        </t>
      </n>
      <n id="n2" p="50 25"/>
      <n id="n3" p="80 25" NodeType="Fragment">
        <fragment id="cached_frag" BoundingBox="70 10 110 45">
          <n id="f1" p="80 20"/>
          <n id="f2" p="100 20"/>
          <b id="fb1" B="f1" E="f2"/>
        </fragment>
        <t p="80 29" BoundingBox="80 18 100 30" LabelJustification="Left">
          <s font="3" size="10" face="97">OMe</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
      <b id="b2" B="n2" E="n3"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("cached fragments")).expect("cdxml should parse");
    let fragments: Vec<_> = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .collect();
    assert_eq!(
        fragments.len(),
        1,
        "cached child fragments under placeholder nodes should not import as visible molecules"
    );
    let fragment = fragments[0];
    assert_eq!(fragment.nodes.len(), 3);
    assert_eq!(fragment.bonds.len(), 2);
    assert!(fragment.nodes.iter().any(|node| {
        node.label
            .as_ref()
            .is_some_and(|label| label.source_text.as_deref() == Some("Bn"))
    }));
    assert!(fragment.nodes.iter().any(|node| {
        node.label
            .as_ref()
            .is_some_and(|label| label.source_text.as_deref() == Some("OMe"))
    }));
}

#[test]
fn parse_cdxml_skips_embedded_fragments_for_every_node_type() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="18" LineWidth="0.6" LabelSize="10">
  <page id="1" BoundingBox="0 0 140 60">
    <fragment id="visible" BoundingBox="0 0 120 50">
      <n id="n1" p="20 25" NodeType="AnonymousAlternativeGroup">
        <t p="20 29" BoundingBox="18 18 24 30"><s font="3" size="10">F,</s></t>
        <fragment id="alternative-definition">
          <n id="a1" p="20 45" Element="9"/>
          <n id="a2" NodeType="ExternalConnectionPoint"/>
          <b id="ab1" B="a2" E="a1"/>
        </fragment>
      </n>
      <n id="n2" p="55 25" NodeType="ElementListNickname">
        <t p="55 29" BoundingBox="53 18 65 30"><s font="3" size="10">Hal</s></t>
        <fragment id="element-list-definition">
          <n id="e1" p="55 45" Element="17"/>
          <n id="e2" NodeType="ExternalConnectionPoint"/>
          <b id="eb1" B="e2" E="e1"/>
        </fragment>
      </n>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#;
    let document =
        parse_cdxml_document(cdxml, Some("embedded node fragments")).expect("cdxml should parse");
    let fragments: Vec<_> = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .collect();
    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].nodes.len(), 2);
    assert_eq!(fragments[0].bonds.len(), 1);
    assert!(!fragments[0]
        .nodes
        .iter()
        .any(|node| matches!(node.atomic_number, 9 | 17)));
}

#[test]
fn parse_cdxml_preserves_authored_geometry_for_multiline_character_attachments() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="18" LineWidth="0.5" LabelSize="7">
  <page id="1" BoundingBox="100 50 200 110">
    <fragment id="f1" BoundingBox="100 50 200 110">
      <n id="label" p="150 80" NodeType="Unspecified" LabelDisplay="Right">
        <t p="162 70" BoundingBox="140 60 162 84" LineStarts="3 8 10"
           LabelJustification="Right" LabelLineHeight="8">
          <s font="3" size="7">R5&#10;COOC&#10;R7</s>
        </t>
      </n>
      <n id="left" p="120 80"/><n id="right" p="180 80"/>
      <b id="b1" B="label" E="left" BeginAttach="3"/>
      <b id="b2" B="label" E="right" BeginAttach="6"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("multiline character attachment"))
        .expect("cdxml should parse");
    let label = document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.nodes.iter())
        .find(|node| node.id == "label")
        .and_then(|node| node.label.as_ref())
        .expect("attached label");
    let molecule = document
        .objects
        .iter()
        .find(|object| object.payload.resource_ref.is_some())
        .expect("molecule object");
    assert_eq!(molecule.transform.translate, [100.0, 50.0]);
    assert_eq!(label.position, Some([62.0, 20.0]));
    assert_eq!(label.box_field, Some([40.0, 10.0, 62.0, 34.0]));
    assert_eq!(
        [
            molecule.transform.translate[0] + label.position.unwrap()[0],
            molecule.transform.translate[1] + label.position.unwrap()[1],
        ],
        [162.0, 70.0]
    );
    assert_eq!(label.lines, ["R5", "COOC", "R7"]);
}

#[test]
fn load_cdxml_document_preserves_imported_acs_drawing_options() {
    let Some(cdxml) = read_optional_cdxml_fixture("db-acs.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");

    assert!((engine.options().bond_length - 14.4).abs() < 0.05);
    assert!((engine.options().bond_stroke_width - 0.6).abs() < 0.01);
    assert!((engine.options().bold_bond_width - 2.0).abs() < 0.05);
    assert!((engine.options().wedge_width - 3.0).abs() < 0.05);
    assert!((engine.options().hash_spacing - 2.5).abs() < 0.05);
    assert!((engine.options().bond_spacing - 18.0).abs() < 0.05);
    assert!(engine.options().label_clip_margin.abs() < 0.01);
    assert!((engine.options().margin_width - 2.0).abs() < 0.05);
}

#[test]
fn load_cdxml_document_preserves_imported_label_font_size() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" LabelSize="10">
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
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment")
        .fragment;
    let label = fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .and_then(|node| node.label.as_ref())
        .expect("imported N label");
    assert_eq!(label.font_family.as_deref(), Some("Arial"));
    assert_eq!(label.font_size, Some(10.0));
    assert_eq!(label.runs.first().and_then(|run| run.font_size), Some(10.0));

    let session = engine
        .begin_text_edit(Point::new(10.0, 12.0))
        .expect("clicking label should open a text edit session");
    assert_eq!(session.font_family.as_deref(), Some("Arial"));
    assert_eq!(session.font_size, Some(10.0));
    assert_eq!(
        session.source_runs.first().and_then(|run| run.font_size),
        Some(10.0)
    );
}

#[test]
fn load_cdxml_document_derives_wedge_width_from_imported_bold_width_multiplier() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.99" BoldWidth="2.01" HashSpacing="2.49" BondSpacing="18" MarginWidth="1.7" LabelSize="10">
  <page id="p1" BoundingBox="0 0 100 100">
    <fragment id="f1" BoundingBox="10 10 40 20">
      <n id="n1" p="10 15"/>
      <n id="n2" p="24.4 15"/>
      <b id="b1" B="n1" E="n2" Display="WedgeBegin"/>
    </fragment>
  </page>
</CDXML>"#;
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(cdxml)
        .expect("cdxml should load into engine");

    assert!((engine.options().bond_length - 14.4).abs() < 0.05);
    assert!((engine.options().bond_stroke_width - 0.99).abs() < 0.01);
    assert!((engine.options().bold_bond_width - 2.01).abs() < 0.01);
    assert!((engine.options().wedge_width - 3.015).abs() < 0.01);
    assert!(engine.options().label_clip_margin.abs() < 0.01);
    assert!((engine.options().margin_width - 1.7).abs() < 0.01);

    let bond = &engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .bonds[0];
    assert!((bond.wedge_width.unwrap_or_default() - 3.015).abs() < 0.01);
    assert_eq!(bond.label_clip_margin, None);
    assert_eq!(bond.margin_width, None);
}

#[test]
fn load_cdxml_document_does_not_import_margin_width_as_label_retreat() {
    fn imported_label_clip_profile(
        line_width: f64,
        margin_width: f64,
    ) -> (f64, Option<(f64, f64)>) {
        let cdxml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="{line_width:.2}" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="{margin_width:.2}" LabelSize="10">
  <page id="p1" BoundingBox="0 0 50 30">
    <fragment id="f1" BoundingBox="0 0 50 30">
      <n id="n1" p="10 15"/>
      <n id="n2" p="24.4 15" Element="7">
        <t p="20.8 18.9" BoundingBox="20.8 10.56 28.02 18.9" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2"/>
    </fragment>
  </page>
</CDXML>"#
        );
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(&cdxml)
            .expect("cdxml should load");
        let profile = engine
            .state()
            .document
            .editable_fragment()
            .and_then(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .find_map(|node| node.label.as_ref())
                    .and_then(|label| {
                        let meta = label.meta.pointer("/import/cdxml")?;
                        let natural = meta.get("naturalOutsetPt")?.as_f64()?;
                        let radius = meta.get("circleRadiusPt")?.as_f64()?;
                        Some((natural, radius))
                    })
            });
        (engine.options().label_clip_margin, profile)
    }

    let (normal, normal_profile) = imported_label_clip_profile(0.60, 1.60);
    let (wide_line, wide_line_profile) = imported_label_clip_profile(1.80, 1.60);
    let (wide_margin, wide_margin_profile) = imported_label_clip_profile(0.60, 5.00);

    assert!(normal.abs() < 0.01, "{normal}");
    assert_eq!(normal_profile, Some((1.6, 3.2)));
    assert_eq!(wide_line_profile, Some((1.6, 3.2)));
    assert_eq!(wide_margin_profile, Some((5.0, 10.0)));
    assert!(
        (wide_line - normal).abs() < 0.01,
        "CDXML MarginWidth should not mutate the legacy global label clip option: {normal} {wide_line}"
    );
    assert!((wide_margin - normal).abs() < 0.01, "{wide_margin}");
}

#[test]
fn cdxml_imported_bonds_use_engine_glyph_retreat() {
    let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML BondLength="14.40" LineWidth="0.60" BoldWidth="2.00" HashSpacing="2.50" BondSpacing="18" MarginWidth="1.60" LabelSize="10">
  <page id="p1" BoundingBox="0 0 70 30">
    <fragment id="f1" BoundingBox="0 0 70 30">
      <n id="n1" p="10 15"/>
      <n id="n2" p="34.4 15" Element="7">
        <t p="30.8 18.9" BoundingBox="30.8 10.56 38.02 18.9" LabelJustification="Left">
          <s font="3" size="10" color="0" face="96">N</s>
        </t>
      </n>
      <b id="b1" B="n1" E="n2" Display="Bold"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(cdxml, Some("glyph retreat")).expect("cdxml should parse");
    let bond = imported_fragment_bond(&document, "obj_mol_001", "b1");
    assert_eq!(bond.label_clip_margin, None);
    assert_eq!(bond.margin_width, None);

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
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
        .expect("bold bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };

    let retreat_from_text_origin = 30.8 - label_endpoint.x;
    assert!(
        (0.75..=1.05).contains(&retreat_from_text_origin),
        "imported bond should clip at the source-margin glyph polygon without adding a second MarginWidth retreat: {polygon:?}"
    );
}
