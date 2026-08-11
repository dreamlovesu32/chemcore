use super::*;

#[test]
fn hovered_bond_style_shortcut_accepts_wavy_alias() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(select_tool());
    engine.pointer_move(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: None,
        alt_key: false,
    });

    assert!(engine.apply_hovered_bond_style("wavy"));
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = &fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_styles.main, BondLinePattern::Wavy);
}

#[test]
fn very_short_wavy_bond_renders_without_invalid_amplitude() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_short_wavy",
            "title": "short wavy bond",
            "page": { "width": 120.0, "height": 80.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemsema_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
            }
        },
        "objects": [{
            "id": "obj_molecule_001",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_001" }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [10.0, 10.0, 12.0, 4.0],
                    "nodes": [{
                        "id": "n1",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [10.0, 10.0],
                        "charge": 0,
                        "numHydrogens": 0
                    }, {
                        "id": "n2",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [12.0, 10.0],
                        "charge": 0,
                        "numHydrogens": 0
                    }],
                    "bonds": [{
                        "id": "b1",
                        "begin": "n1",
                        "end": "n2",
                        "order": 1,
                        "lineStyles": { "main": "wavy" }
                    }]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");

    let primitives = engine.render_list();

    assert!(primitives.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentBond,
            bond_id,
            ..
        } if bond_id.as_deref() == Some("b1")
    )));
}

#[test]
fn hovered_bond_style_shortcut_noops_without_hover() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    assert!(!engine.apply_hovered_bond_style("double-center"));
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = &fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
}

#[test]
fn join_selection_is_not_group_selection_fallback() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert!(engine.select_all());
    let before_document = engine.document_json().unwrap();
    let before_selection = engine.state().selection.clone();

    assert!(!engine.join_selection());
    assert_eq!(engine.document_json().unwrap(), before_document);
    assert_eq!(engine.state().selection, before_selection);
}

#[test]
fn chemical_check_context_command_suppresses_invalid_label_marker() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_chemical_check_toggle",
            "title": "chemical check toggle",
            "page": { "width": 80.0, "height": 60.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": { "kind": "molecule" }
        },
        "objects": [
            {
                "id": "obj_mol",
                "type": "molecule",
                "styleRef": "style_molecule_default",
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol" }
            }
        ],
        "resources": {
            "mol": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 30.0],
                    "nodes": [
                        {
                            "id": "n1",
                            "element": "C",
                            "atomicNumber": 6,
                            "position": [20.0, 20.0],
                            "charge": 0,
                            "numHydrogens": 0,
                            "label": {
                                "text": "BadLabel",
                                "position": [20.0, 20.0],
                                "box": [15.0, 14.0, 50.0, 26.0],
                                "meta": { "labelRecognition": { "status": "invalid" } }
                            }
                        }
                    ],
                    "bonds": []
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("chemical check fixture should load");
    engine.select_at_point(Point::new(20.0, 20.0), false);
    assert!(engine.set_chemical_check_for_selection(false));

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let node = fragment.nodes.iter().find(|node| node.id == "n1").unwrap();
    assert_eq!(node.meta["chemicalCheck"], false);
    assert_eq!(node.label.as_ref().unwrap().meta["chemicalCheck"], false);

    let render = engine.render_list();
    let has_invalid_box = render.iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::DocumentGraphic,
                node_id: Some(node_id),
                ..
            } if node_id == "n1"
        )
    });
    assert!(
        !has_invalid_box,
        "chemical check off should hide invalid label marker"
    );
}

#[test]
fn select_tool_dragging_selected_bond_moves_its_endpoints() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let start = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let end = Point::new(FIRST_CENTER_X + px(24.0), FIRST_CENTER_Y + px(18.0));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let entry = engine.state().document.editable_fragment().unwrap();
    let n1 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_1")
        .unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    let expected_n1_x = round_to_2(FIRST_START_X + px(24.0));
    let expected_n1_y = round_to_2(FIRST_START_Y + px(18.0));
    let expected_n2_x = round_to_2(FIRST_END_X + px(24.0));
    let expected_n2_y = round_to_2(FIRST_END_Y + px(18.0));
    assert!(
        (n1.position[0] - expected_n1_x).abs() < 0.01,
        "n1 x {:?} expected {expected_n1_x}",
        n1.position
    );
    assert!(
        (n1.position[1] - expected_n1_y).abs() < 0.01,
        "n1 y {:?} expected {expected_n1_y}",
        n1.position
    );
    assert!(
        (n2.position[0] - expected_n2_x).abs() < 0.01,
        "n2 x {:?} expected {expected_n2_x}",
        n2.position
    );
    assert!(
        (n2.position[1] - expected_n2_y).abs() < 0.01,
        "n2 y {:?} expected {expected_n2_y}",
        n2.position
    );
}

#[test]
fn select_tool_dragging_selected_label_translates_label_geometry() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(314.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(314.0), px(264.0)]
        ])],
        json!([{ "id": "b1", "begin": "n0", "end": "n1", "order": 1 }]),
    );
    engine.set_tool_state(select_tool());
    let start = px_point(318.0, 260.0);
    let delta = px_point(20.0, 12.0);
    let end = Point::new(start.x + delta.x, start.y + delta.y);

    assert!(engine.select_component_at_point(start, false));
    let entry = engine.state().document.editable_fragment().unwrap();
    let before_node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    let before_position = before_node.position;
    let before_label = before_node.label.as_ref().unwrap().clone();
    let before_label_position = before_label.position.unwrap();
    let before_label_box = before_label.bbox().unwrap();
    let before_glyph_bounds = label_glyph_bounds(&before_label);
    let before_clip_polygons = before_label.glyph_clip_polygons.clone();
    assert!(!before_clip_polygons.is_empty());

    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(end, false));
    let live_entry = engine.state().document.editable_fragment().unwrap();
    let live_label = live_entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .and_then(|node| node.label.as_ref())
        .unwrap();
    assert_eq!(
        live_label.glyph_clip_polygons.len(),
        before_clip_polygons.len()
    );
    for (before_polygon, live_polygon) in before_clip_polygons
        .iter()
        .zip(&live_label.glyph_clip_polygons)
    {
        for (before, live) in before_polygon.iter().zip(live_polygon) {
            assert!((live[0] - round_to_2(before[0] + delta.x)).abs() < 0.001);
            assert!((live[1] - round_to_2(before[1] + delta.y)).abs() < 0.001);
        }
    }
    assert!(engine.finish_selection_move(end, false));

    let entry = engine.state().document.editable_fragment().unwrap();
    let after_node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    let after_label = after_node.label.as_ref().unwrap();
    let after_label_position = after_label.position.unwrap();
    let after_label_box = after_label.bbox().unwrap();
    let after_glyph_bounds = label_glyph_bounds(after_label);

    assert!((after_node.position[0] - round_to_2(before_position[0] + delta.x)).abs() < 0.001);
    assert!((after_node.position[1] - round_to_2(before_position[1] + delta.y)).abs() < 0.001);
    assert!(
        (after_label_position[0] - round_to_2(before_label_position[0] + delta.x)).abs() < 0.001,
        "label x moved from {:?} to {:?}, expected delta {:?}",
        before_label_position,
        after_label_position,
        delta
    );
    assert!(
        (after_label_position[1] - round_to_2(before_label_position[1] + delta.y)).abs() < 0.001,
        "label y moved from {:?} to {:?}, expected delta {:?}",
        before_label_position,
        after_label_position,
        delta
    );
    for index in 0..4 {
        let expected_delta = if index % 2 == 0 { delta.x } else { delta.y };
        assert!(
            (after_label_box[index] - round_to_2(before_label_box[index] + expected_delta)).abs()
                < 0.001
        );
        assert!(
            (after_glyph_bounds[index] - round_to_2(before_glyph_bounds[index] + expected_delta))
                .abs()
                < 0.001
        );
    }
}

#[test]
fn select_tool_dragging_selected_line_translates_payload_points_on_finish() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_line_move",
            "title": "line move",
            "page": { "width": 120.0, "height": 80.0, "background": "#ffffff" }
        },
        "styles": {
            "style_line": {
                "kind": "line",
                "stroke": "#111111",
                "strokeWidth": 1.0,
                "lineCap": "round",
                "lineJoin": "round"
            }
        },
        "objects": [{
            "id": "obj_line",
            "type": "line",
            "styleRef": "style_line",
            "payload": {
                "points": [[10.0, 20.0], [40.0, 20.0]],
                "kind": "line",
                "arrowGeometry": {
                    "boundingBox": [10.0, 18.0, 40.0, 24.0],
                    "center": [25.0, 24.0],
                    "majorAxisEnd": [33.0, 24.0],
                    "minorAxisEnd": [25.0, 32.0]
                }
            }
        }],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("line move fixture should load");
    engine.set_tool_state(select_tool());
    assert!(engine.select_all());

    let start = Point::new(20.0, 20.0);
    let end = Point::new(26.0, 23.0);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.finish_selection_move(end, false));

    let line = engine
        .state()
        .document
        .find_scene_object("obj_line")
        .unwrap();
    assert_eq!(
        line.payload.extra["points"],
        json!([[16.0, 23.0], [46.0, 23.0]])
    );
    assert_eq!(
        line.payload.extra["arrowGeometry"]["center"],
        json!([31.0, 27.0])
    );
    assert_eq!(
        line.payload.extra["arrowGeometry"]["majorAxisEnd"],
        json!([39.0, 27.0])
    );
    assert_eq!(
        line.payload.extra["arrowGeometry"]["minorAxisEnd"],
        json!([31.0, 35.0])
    );
    assert_eq!(
        line.payload.extra["arrowGeometry"]["boundingBox"],
        json!([16.0, 21.0, 46.0, 27.0])
    );
}
