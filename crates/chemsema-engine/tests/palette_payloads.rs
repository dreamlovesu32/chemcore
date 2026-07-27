use chemsema_engine::Engine;

#[test]
fn engine_exposes_ui_palette_payloads() {
    let engine = Engine::new();

    let toolbar_colors: serde_json::Value =
        serde_json::from_str(&engine.toolbar_color_palette_json(r##"["#336699"]"##)).unwrap();
    assert_eq!(toolbar_colors["type"], "toolbar-color-palette");
    assert!(toolbar_colors["colors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["value"] == "#336699"));

    let color_dialog: serde_json::Value =
        serde_json::from_str(&engine.color_dialog_palette_json("#ff0000", r##"["#336699"]"##))
            .unwrap();
    assert_eq!(color_dialog["type"], "color-dialog");
    assert_eq!(color_dialog["selected"], "#ff0000");
    assert!(color_dialog["basicColors"].as_array().unwrap().len() >= 48);

    let text_symbols: serde_json::Value =
        serde_json::from_str(&engine.text_symbol_palette_json()).unwrap();
    assert_eq!(text_symbols["type"], "text-symbol-palette");
    assert!(text_symbols["groups"].as_array().unwrap().len() >= 3);

    let elements: serde_json::Value = serde_json::from_str(&engine.element_palette_json()).unwrap();
    assert_eq!(elements["type"], "periodic-table");
    assert_eq!(elements["current"]["symbol"], "P");
    assert!(elements["elements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["symbol"] == "C" && entry["color"]["background"] == "#000000"));
}

#[test]
fn engine_applies_element_palette_selection() {
    let mut engine = Engine::new();

    assert!(engine
        .apply_element_palette_json(r#"{"symbol":"O"}"#)
        .unwrap());

    let elements: serde_json::Value = serde_json::from_str(&engine.element_palette_json()).unwrap();
    assert_eq!(elements["current"]["symbol"], "O");
    assert_eq!(elements["current"]["atomicNumber"], 8);
}

#[test]
fn engine_builds_template_palette_documents_and_icons_from_cdxml_pages() {
    let engine = Engine::new();
    let source = r#"<CDXML BondLength="14.4">
      <page id="1"><fragment id="2"><n id="3" p="0 0"/><n id="4" p="14.4 0"/><b id="5" B="3" E="4"/></fragment><annotation Keyword="Name" Content="ethane"/></page>
      <page id="6"><graphic id="7" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="10 10 40 30"/></page>
    </CDXML>"#;
    let palette: serde_json::Value = serde_json::from_str(
        &engine
            .template_library_palette_json("test-library", "Test Library", source)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(palette["type"], "template-library-palette");
    assert_eq!(palette["library"]["templateCount"], 2);
    assert_eq!(palette["templates"][0]["id"], "test-library:001");
    assert_eq!(palette["templates"][0]["label"], "ethane");
    assert!(palette["templates"][0]["iconSvg"]
        .as_str()
        .unwrap()
        .contains("cc-kernel-template-icon"));
    assert!(palette["templates"][0]["documentJson"]
        .as_str()
        .unwrap()
        .contains("\"resources\""));
}

#[test]
fn document_template_insertion_centers_content_and_preserves_styles() {
    let source = r#"<CDXML><page id="1"><graphic id="2" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="10 10 40 30"/></page></CDXML>"#;
    let engine = Engine::new();
    let palette: serde_json::Value = serde_json::from_str(
        &engine
            .template_library_palette_json("shapes", "Shapes", source)
            .unwrap(),
    )
    .unwrap();
    let mut template_document: serde_json::Value =
        serde_json::from_str(palette["templates"][0]["documentJson"].as_str().unwrap()).unwrap();
    template_document["styles"]["style_template_custom"] = serde_json::json!({
        "kind": "shape",
        "stroke": "#d12f2f",
        "strokeWidth": 2.25,
        "fill": "#f8dddd"
    });
    template_document["objects"][0]["styleRef"] = serde_json::json!("style_template_custom");

    let mut target = Engine::new();
    assert!(target
        .insert_document_template_json_at(
            "shapes:001",
            &serde_json::to_string(&template_document).unwrap(),
            100.0,
            200.0,
        )
        .unwrap());
    let bounds = target
        .render_bounds(chemsema_engine::RenderBoundsScope::Selection)
        .unwrap();
    assert!(
        ((bounds[0] + bounds[2]) * 0.5 - 100.0).abs() < 0.02,
        "{bounds:?}"
    );
    assert!(
        ((bounds[1] + bounds[3]) * 0.5 - 200.0).abs() < 0.02,
        "{bounds:?}"
    );
    assert!(target.state().document.styles.values().any(|style| {
        style.get("stroke").and_then(serde_json::Value::as_str) == Some("#d12f2f")
    }));
}

#[test]
fn molecular_template_click_on_atom_merges_primary_node_and_uses_open_direction() {
    let template_cdxml = r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2">
      <n id="3" p="0 0"/><n id="4" p="12.47 -7.2"/><n id="5" p="24.94 0"/>
      <b id="6" B="3" E="4"/><b id="7" B="4" E="5" Order="2"/>
    </fragment></page></CDXML>"#;
    let palette_engine = Engine::new();
    let palette: serde_json::Value = serde_json::from_str(
        &palette_engine
            .template_library_palette_json("functional", "Functional", template_cdxml)
            .unwrap(),
    )
    .unwrap();
    let template_json = palette["templates"][0]["documentJson"].as_str().unwrap();

    let mut target = Engine::new();
    target
        .load_cdxml_document(
            r#"<CDXML BondLength="14.4"><page id="10"><fragment id="11">
              <n id="12" p="100 100"/><n id="13" p="112.47 92.8"/><n id="14" p="124.94 100"/>
              <b id="15" B="12" E="13"/><b id="16" B="13" E="14" Order="2"/>
            </fragment></page></CDXML>"#,
        )
        .unwrap();
    let changed = target
        .insert_document_template_json_at("functional:001", template_json, 100.0, 100.0)
        .unwrap();
    assert!(
        changed,
        "selection={:?} fragments={:?}",
        target.state().selection,
        target
            .state()
            .document
            .editable_fragments()
            .into_iter()
            .map(|entry| (
                entry.object.id.clone(),
                entry
                    .fragment
                    .nodes
                    .iter()
                    .map(|node| (node.id.clone(), entry.world_point_for_node(node)))
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );

    let fragments = target.state().document.editable_fragments();
    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].fragment.nodes.len(), 5);
    assert_eq!(fragments[0].fragment.bonds.len(), 4);
    let anchor = fragments[0]
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "12")
        .unwrap();
    let anchor_point = fragments[0].world_point_for_node(anchor);
    let attached = fragments[0]
        .fragment
        .bonds
        .iter()
        .find(|bond| (bond.begin == "12" || bond.end == "12") && bond.id != "15")
        .unwrap();
    let other_id = if attached.begin == "12" {
        &attached.end
    } else {
        &attached.begin
    };
    let other = fragments[0]
        .fragment
        .nodes
        .iter()
        .find(|node| &node.id == other_id)
        .unwrap();
    let other_point = fragments[0].world_point_for_node(other);
    assert!(
        other_point.x < anchor_point.x,
        "attached template should turn into the open half-plane: {anchor_point:?} -> {other_point:?}"
    );
}

#[test]
fn molecular_template_click_on_bond_fuses_primary_bond_and_keeps_target_bond() {
    let template_cdxml = r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2">
      <n id="3" p="0 0"/><n id="4" p="12.47 -7.2"/><n id="5" p="24.94 0"/>
      <b id="6" B="3" E="4"/><b id="7" B="4" E="5" Order="2"/>
    </fragment></page></CDXML>"#;
    let palette_engine = Engine::new();
    let palette: serde_json::Value = serde_json::from_str(
        &palette_engine
            .template_library_palette_json("functional", "Functional", template_cdxml)
            .unwrap(),
    )
    .unwrap();
    let template_json = palette["templates"][0]["documentJson"].as_str().unwrap();

    let mut target = Engine::new();
    target
        .load_cdxml_document(
            r#"<CDXML BondLength="14.4"><page id="10"><fragment id="11">
              <n id="12" p="100 100"/><n id="13" p="112.47 92.8"/><n id="14" p="124.94 100"/>
              <b id="15" B="12" E="13"/><b id="16" B="13" E="14" Order="2"/>
            </fragment></page></CDXML>"#,
        )
        .unwrap();
    assert!(target
        .insert_document_template_json_at(
            "functional:001",
            template_json,
            (112.47 + 124.94) * 0.5,
            (92.8 + 100.0) * 0.5,
        )
        .unwrap());

    let fragments = target.state().document.editable_fragments();
    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].fragment.nodes.len(), 4);
    assert_eq!(fragments[0].fragment.bonds.len(), 3);
    let retained = fragments[0]
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "16")
        .unwrap();
    assert_eq!(retained.order, 2);
    assert_eq!(
        fragments[0]
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                (bond.begin == "13" && bond.end == "14") || (bond.begin == "14" && bond.end == "13")
            })
            .count(),
        1,
        "the source fusion bond must be discarded instead of duplicated",
    );
}

#[test]
fn molecular_template_fusion_remaps_semantic_references_to_retained_entities() {
    let template_cdxml = r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2">
      <n id="3" p="0 0"/><n id="4" p="14.4 0"/><n id="5" p="7.2 -12.47"/>
      <b id="6" B="3" E="4" Order="2"/><b id="7" B="4" E="5"/><b id="8" B="5" E="3"/>
    </fragment></page></CDXML>"#;
    let palette_engine = Engine::new();
    let palette: serde_json::Value = serde_json::from_str(
        &palette_engine
            .template_library_palette_json("semantic", "Semantic", template_cdxml)
            .unwrap(),
    )
    .unwrap();
    let mut template: serde_json::Value =
        serde_json::from_str(palette["templates"][0]["documentJson"].as_str().unwrap()).unwrap();
    let resource_id = template["objects"][0]["payload"]["resourceRef"]
        .as_str()
        .unwrap()
        .to_string();
    let fragment = &mut template["resources"][&resource_id]["data"];
    fragment["stereo"] = serde_json::json!([{
        "kind": "doubleBond",
        "id": "stereo-template",
        "bond": "6",
        "leftReference": "3",
        "rightReference": "5",
        "relation": "opposite"
    }]);
    fragment["interactions"] = serde_json::json!([{
        "id": "interaction-template",
        "kind": "coordination",
        "centers": [
            {"role": "donor", "atoms": ["3"]},
            {"role": "acceptor", "atoms": ["5"]}
        ]
    }]);
    fragment["coloredAreas"] = serde_json::json!([{
        "id": "area-template",
        "color": "#00ff00",
        "basisBonds": ["6", "7", "8"]
    }]);

    let mut target = Engine::new();
    target
        .load_cdxml_document(
            r#"<CDXML BondLength="14.4"><page id="10"><fragment id="11">
              <n id="12" p="100 100"/><n id="13" p="114.4 100"/>
              <b id="15" B="12" E="13" Order="2"/>
            </fragment></page></CDXML>"#,
        )
        .unwrap();
    assert!(target
        .insert_document_template_json_at(
            "semantic:001",
            &serde_json::to_string(&template).unwrap(),
            107.2,
            100.0,
        )
        .unwrap());

    let fragments = target.state().document.editable_fragments();
    assert_eq!(fragments.len(), 1);
    let fragment = fragments[0].fragment;
    let stereo = serde_json::to_value(&fragment.stereo).unwrap();
    assert_eq!(stereo[0]["bond"], "15");
    assert_eq!(stereo[0]["leftReference"], "12");
    let interactions = serde_json::to_value(&fragment.interactions).unwrap();
    assert_eq!(interactions[0]["centers"][0]["atoms"][0], "12");
    assert!(fragment.colored_areas[0]
        .basis_bonds
        .iter()
        .any(|bond| bond == "15"));
}

#[test]
fn document_template_scales_from_library_bond_length_to_target_bond_length() {
    let template_cdxml = r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2">
      <n id="3" p="0 0"/><n id="4" p="14.4 0"/><b id="5" B="3" E="4"/>
    </fragment></page></CDXML>"#;
    let palette_engine = Engine::new();
    let palette: serde_json::Value = serde_json::from_str(
        &palette_engine
            .template_library_palette_json("scale", "Scale", template_cdxml)
            .unwrap(),
    )
    .unwrap();
    let mut target = Engine::new();
    target
        .load_cdxml_document(
            r#"<CDXML BondLength="28.8"><page id="10"><fragment id="11">
              <n id="12" p="0 0"/><n id="13" p="28.8 0"/><b id="14" B="12" E="13"/>
            </fragment></page></CDXML>"#,
        )
        .unwrap();
    assert!(target
        .insert_document_template_json_at(
            "scale:001",
            palette["templates"][0]["documentJson"].as_str().unwrap(),
            200.0,
            200.0,
        )
        .unwrap());
    let inserted = target
        .state()
        .document
        .editable_fragments()
        .into_iter()
        .find(|entry| entry.object.id != "11")
        .unwrap();
    let begin = inserted.world_point_for_node(&inserted.fragment.nodes[0]);
    let end = inserted.world_point_for_node(&inserted.fragment.nodes[1]);
    assert!((begin.distance(end) - 28.8).abs() < 0.02);
}

#[test]
fn nonmolecular_template_over_atom_uses_explicit_center_placement() {
    let source = r#"<CDXML><page id="1"><graphic id="2" GraphicType="Rectangle" RectangleType="Plain" BoundingBox="10 10 40 30"/></page></CDXML>"#;
    let palette_engine = Engine::new();
    let palette: serde_json::Value = serde_json::from_str(
        &palette_engine
            .template_library_palette_json("shape", "Shape", source)
            .unwrap(),
    )
    .unwrap();
    let mut target = Engine::new();
    target
        .load_cdxml_document(
            r#"<CDXML><page id="10"><fragment id="11"><n id="12" p="100 100"/></fragment></page></CDXML>"#,
        )
        .unwrap();
    assert!(target
        .insert_document_template_json_at(
            "shape:001",
            palette["templates"][0]["documentJson"].as_str().unwrap(),
            100.0,
            100.0,
        )
        .unwrap());
    let bounds = target
        .render_bounds(chemsema_engine::RenderBoundsScope::Selection)
        .unwrap();
    assert!(((bounds[0] + bounds[2]) * 0.5 - 100.0).abs() < 0.02);
    assert!(((bounds[1] + bounds[3]) * 0.5 - 100.0).abs() < 0.02);
}
