use super::*;

#[test]
fn element_tool_places_selected_element_with_chemdraw_hydrogens() {
    let mut engine = Engine::new();
    let mut tool = ToolState {
        active_tool: Tool::Element,
        element_symbol: "Se".to_string(),
        element_atomic_number: 34,
        ..ToolState::default()
    };
    engine.set_tool_state(tool.clone());
    click(&mut engine, 40.0, 50.0);
    assert!(engine.state().selection.is_empty());

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("blank document has an editable fragment")
        .fragment;
    assert_eq!(fragment.nodes.len(), 1);
    let node = &fragment.nodes[0];
    assert_eq!(node.element, "Se");
    assert_eq!(node.atomic_number, 34);
    assert_eq!(node.num_hydrogens, 2);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("SeH2")
    );

    tool.element_symbol = "Au".to_string();
    tool.element_atomic_number = 79;
    engine.set_tool_state(tool);
    click(&mut engine, 70.0, 80.0);
    assert!(engine.state().selection.is_empty());
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("blank document has an editable fragment")
        .fragment;
    assert_eq!(fragment.nodes.len(), 2);
    let node = &fragment.nodes[1];
    assert_eq!(node.element, "Au");
    assert_eq!(node.num_hydrogens, 0);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("Au")
    );
}

#[test]
fn element_tool_replaces_focused_endpoint_without_adding_node() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    let target_point = Point::new(FIRST_END_X, FIRST_END_Y);
    let node_id = node_id_at(&engine, target_point).expect("terminal node should exist");

    engine.set_tool_state(ToolState {
        active_tool: Tool::Element,
        element_symbol: "Se".to_string(),
        element_atomic_number: 34,
        ..ToolState::default()
    });
    engine.pointer_move(PointerEvent {
        x: FIRST_END_HOVER_X,
        y: FIRST_END_HOVER_Y,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine
            .state()
            .overlay
            .hover_endpoint
            .as_ref()
            .map(|hit| hit.node_id.as_str()),
        Some(node_id.as_str())
    );

    click(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    assert_eq!(entry.fragment.nodes.len(), 2);
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("replaced node should still exist");
    assert_eq!(node.element, "Se");
    assert_eq!(node.atomic_number, 34);
    assert_eq!(node.num_hydrogens, 1);
    assert_eq!(
        node.label
            .as_ref()
            .and_then(|label| label.source_text.as_deref()),
        Some("SeH")
    );
}

#[test]
fn element_tool_replaces_structure_label_but_ignores_free_text() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    let target_point = Point::new(FIRST_END_X, FIRST_END_Y);
    let node_id = node_id_at(&engine, target_point).expect("terminal node should exist");

    engine.set_tool_state(ToolState {
        active_tool: Tool::Element,
        element_symbol: "N".to_string(),
        element_atomic_number: 7,
        ..ToolState::default()
    });
    click(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    let label_center = {
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .expect("nitrogen node should exist");
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.source_text.as_deref()),
            Some("NH2")
        );
        let bounds = node
            .label
            .as_ref()
            .and_then(|label| label.bbox())
            .expect("structure label should have bounds");
        Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
    };

    engine.set_tool_state(ToolState {
        active_tool: Tool::Element,
        element_symbol: "P".to_string(),
        element_atomic_number: 15,
        ..ToolState::default()
    });
    engine.pointer_move(PointerEvent {
        x: label_center.x,
        y: label_center.y,
        button: None,
        alt_key: false,
    });
    let hover_label = engine
        .state()
        .overlay
        .hover_text_box
        .as_ref()
        .expect("structure label should focus");
    assert_eq!(hover_label.node_id.as_deref(), Some(node_id.as_str()));
    assert!(hover_label.object_id.is_none());

    click(&mut engine, label_center.x, label_center.y);

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    assert_eq!(entry.fragment.nodes.len(), 2);
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("phosphorus node should exist");
    assert_eq!(node.element, "P");
    assert_eq!(node.atomic_number, 15);
    assert_eq!(node.num_hydrogens, 2);
    assert_eq!(
        node.label
            .as_ref()
            .and_then(|label| label.source_text.as_deref()),
        Some("PH2")
    );

    let text_session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text object session should be created");
    assert!(engine.apply_text_edit(chemsema_engine::TextEditSession {
        text: "free text".to_string(),
        ..text_session
    }));
    engine.set_tool_state(ToolState {
        active_tool: Tool::Element,
        element_symbol: "S".to_string(),
        element_atomic_number: 16,
        ..ToolState::default()
    });
    engine.pointer_move(PointerEvent {
        x: px(120.0),
        y: px(88.0),
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn selection_chemistry_summary_counts_selected_atoms_only() {
    let mut engine = Engine::new();
    let mut tool = ToolState {
        active_tool: Tool::Element,
        element_symbol: "Se".to_string(),
        element_atomic_number: 34,
        ..ToolState::default()
    };
    engine.set_tool_state(tool.clone());
    click(&mut engine, 40.0, 50.0);

    let empty: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert!(empty.is_null());

    engine.select_at_point(Point::new(40.0, 50.0), false);
    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "H2Se");
    assert_eq!(summary["atomCount"], 3);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 80.987).abs() < 1.0e-9);

    tool.element_symbol = "Au".to_string();
    tool.element_atomic_number = 79;
    engine.set_tool_state(tool);
    click(&mut engine, 70.0, 80.0);
    engine.select_at_point(Point::new(40.0, 50.0), false);
    engine.select_at_point(Point::new(70.0, 80.0), true);
    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "AuH2Se");
    assert_eq!(summary["atomCount"], 4);
}

#[test]
fn selection_chemistry_summary_ignores_selected_bonds() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    assert_eq!(engine.state().selection.bonds.len(), 1);
    assert!(engine.state().selection.nodes.is_empty());

    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert!(summary.is_null());
}

#[test]
fn selection_chemistry_summary_counts_implicit_carbon_hydrogens() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.select_component_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "C2H6");
    assert_eq!(summary["atomCount"], 8);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 30.07).abs() < 1.0e-9);
    assert!((summary["exactMass"].as_f64().unwrap() - 30.046_950_193_38).abs() < 1.0e-9);
}

#[test]
fn selection_chemistry_summary_counts_complete_label_expansions() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let left_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .min_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("left node should exist")
        .clone();
    let session = engine
        .begin_text_edit(Point::new(left_node.position[0], left_node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemsema_engine::TextEditSession {
        text: "OTMS".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let label_center = {
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == left_node.id)
            .expect("left node should still exist");
        assert!(node.is_placeholder);
        let label = node.label.as_ref().expect("label should exist");
        assert_eq!(label.source_text.as_deref(), Some("OTMS"));
        assert_eq!(label.text, "TMSO");
        let bbox = label.bbox().expect("label should have a bbox");
        Point::new((bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5)
    };
    engine.select_at_point(label_center, false);

    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "C3H9OSi");
    assert_eq!(summary["atomCount"], 14);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 89.189).abs() < 1.0e-9);
    assert!((summary["exactMass"].as_f64().unwrap() - 89.042_266_444_29).abs() < 1.0e-9);
}

#[test]
fn selection_chemistry_summary_hides_indeterminate_generic_labels() {
    for generic_label in ["R", "R'", "R''", "Ar"] {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        click(&mut engine, px(300.0), px(260.0));
        let left_node = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist")
            .fragment
            .nodes
            .iter()
            .min_by(|left, right| left.position[0].total_cmp(&right.position[0]))
            .expect("left node should exist")
            .clone();
        let session = engine
            .begin_text_edit(Point::new(left_node.position[0], left_node.position[1]))
            .expect("endpoint session should be created");
        assert!(engine.apply_text_edit(chemsema_engine::TextEditSession {
            text: generic_label.to_string(),
            source_runs: Vec::new(),
            ..session
        }));

        let label_center = {
            let entry = engine
                .state()
                .document
                .editable_fragment()
                .expect("editable fragment should exist");
            let node = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == left_node.id)
                .expect("left node should still exist");
            assert!(
                node.is_placeholder,
                "{generic_label} should remain a generic placeholder"
            );
            let expansion = node
                .meta
                .get("labelRecognition")
                .and_then(|value| value.get("expansion"))
                .expect("generic label should keep recognition metadata");
            assert_eq!(expansion["complete"], false);
            let label = node.label.as_ref().expect("label should exist");
            let bbox = label.bbox().expect("label should have a bbox");
            Point::new((bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5)
        };
        engine.select_component_at_point(label_center, false);

        let summary: serde_json::Value =
            serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
        assert!(
            summary.is_null(),
            "{generic_label} makes the selected molecule composition indeterminate"
        );
    }
}

#[test]
fn bond_pointer_down_clears_previous_hover_overlay() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);

    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.state().overlay.hover_endpoint.is_some());

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.hover_arrow.is_none());
    assert!(engine.state().overlay.hover_shape.is_none());
    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(engine.state().overlay.preview.is_none());
}

#[test]
fn template_pointer_down_clears_previous_hover_overlay() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);

    engine.set_tool_state(templates_tool("chain"));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.state().overlay.hover_endpoint.is_some());

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.hover_arrow.is_none());
    assert!(engine.state().overlay.hover_shape.is_none());
    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(engine.state().overlay.preview.is_none());
}

#[test]
fn arrow_tool_defaults_to_small_head_without_selecting_created_arrow() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });

    assert_eq!(engine.state().tool.arrow_head_size, ArrowHeadSize::Small);
    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

    assert!(engine.state().selection.arrow_objects.is_empty());
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("created arrow object should exist");
    let object_id = object.id.clone();
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("created arrow object should exist");
    let arrow_head = object.payload.extra.get("arrowHead").unwrap();
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("solid")
    );
    assert_eq!(
        arrow_head.get("length").and_then(|value| value.as_f64()),
        Some(10.0)
    );

    let render_list = engine.render_list();
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox,
            ..
        }
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverArrowHandle,
            ..
        }
    )));

    hover(&mut engine, 50.0, 20.0);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverArrowHandle,
            radius,
            ..
        } if (*radius - px(1.5)).abs() < 1.0e-9
    )));

    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    });
    engine.select_at_point(Point::new(50.0, 20.0), false);
    assert_eq!(engine.state().selection.arrow_objects, vec![object_id]);
    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox,
            ..
        }
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverArrowHandle,
            ..
        }
    )));

    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    });
    engine.select_at_point(Point::new(10000.0, 10000.0), false);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    hover(&mut engine, 50.0, 20.0);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverArrowHandle,
            radius,
            ..
        } if (*radius - px(1.5)).abs() < 1.0e-9
    )));
}

#[test]
fn hollow_and_open_arrow_sizes_use_their_own_two_step_template() {
    for (variant, size, expected) in [
        (ArrowVariant::Hollow, ArrowHeadSize::Large, 12.0),
        (ArrowVariant::Hollow, ArrowHeadSize::Medium, 6.0),
        (ArrowVariant::Hollow, ArrowHeadSize::Small, 6.0),
        (ArrowVariant::Open, ArrowHeadSize::Large, 12.0),
        (ArrowVariant::Open, ArrowHeadSize::Medium, 6.0),
        (ArrowVariant::Open, ArrowHeadSize::Small, 6.0),
    ] {
        let mut engine = Engine::new();
        engine.set_tool_state(ToolState {
            active_tool: Tool::Arrow,
            arrow_variant: variant,
            arrow_head_size: size,
            ..ToolState::default()
        });
        drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

        let arrow_head = engine
            .state()
            .document
            .objects
            .iter()
            .find(|object| object.object_type == "line")
            .and_then(|object| object.payload.extra.get("arrowHead"))
            .expect("created arrow should carry arrowHead payload");
        assert_eq!(
            arrow_head.get("length").and_then(|value| value.as_f64()),
            Some(expected)
        );
        assert_eq!(
            arrow_head
                .get("centerLength")
                .and_then(|value| value.as_f64()),
            Some(expected)
        );
        assert_eq!(
            arrow_head.get("width").and_then(|value| value.as_f64()),
            Some(expected * 0.25)
        );
    }
}

#[test]
fn arrow_hover_endpoint_drag_updates_head_with_angle_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(100.0, 0.0)),
        "head"
    );
    assert!(engine.update_hover_arrow_edit(Point::new(100.0, 36.4), false));
    assert!(engine.finish_hover_arrow_edit(Point::new(100.0, 36.4), false));

    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow object should exist");
    let points = object
        .payload
        .extra
        .get("points")
        .and_then(|value| value.as_array())
        .expect("arrow should store points");
    let end = points[1].as_array().unwrap();
    let angle = angle_between(
        Point::new(0.0, 0.0),
        Point::new(end[0].as_f64().unwrap(), end[1].as_f64().unwrap()),
    );
    assert_eq!(angle.round(), 15.0);
}

#[test]
fn arrow_hover_curve_drag_updates_curve_with_snap_and_selected_arrows_do_not_hover() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(50.0, 0.0)),
        "curve"
    );
    assert!(engine.update_hover_arrow_edit(Point::new(50.0, -30.0), false));
    assert_eq!(engine.active_arrow_edit_degrees(), 120.0);
    assert!(engine.finish_hover_arrow_edit(Point::new(50.0, -30.0), false));
    assert!(engine.state().overlay.hover_arrow.is_none());
    assert!(engine.state().overlay.hover_shape.is_none());

    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow object should exist");
    let arrow_head = object.payload.extra.get("arrowHead").unwrap();
    assert_eq!(
        arrow_head.get("curve").and_then(|value| value.as_f64()),
        Some(-120.0)
    );
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("curved")
    );

    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(50.0, -28.0), false);
    assert_eq!(engine.state().selection.arrow_objects.len(), 1);
    hover(&mut engine, 50.0, -28.0);
    assert!(engine.state().overlay.hover_arrow.is_none());
}

#[test]
fn arrow_body_hover_and_selection_use_graphic_edge_radius() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    let near_body = Point::new(50.0, ARROW_HIT_RADIUS - px(0.25));
    let far_body = Point::new(50.0, ARROW_HIT_RADIUS + px(0.25));

    hover(&mut engine, far_body.x, far_body.y);
    assert!(engine.state().overlay.hover_arrow.is_none());

    hover(&mut engine, near_body.x, near_body.y);
    assert!(engine.state().overlay.hover_arrow.is_some());

    engine.set_tool_state(select_tool());
    engine.select_at_point(far_body, false);
    assert!(engine.state().selection.arrow_objects.is_empty());

    engine.select_at_point(near_body, false);
    assert_eq!(engine.state().selection.arrow_objects.len(), 1);
}

#[test]
fn arrow_curve_drag_interaction_preview_only_renders_edited_arrow() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_arrow_preview",
            "title": "arrow preview",
            "page": { "width": 200.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_arrow",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "points": [[0.0, 0.0], [100.0, 0.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "head": "full",
                        "tail": "none",
                        "length": 15.0,
                        "width": 3.75,
                        "curve": 0.0
                    }
                }
            },
            {
                "id": "obj_other",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "points": [[0.0, 60.0], [100.0, 60.0]],
                    "head": "end",
                    "tail": "none",
                    "arrowHead": {
                        "kind": "solid",
                        "head": "full",
                        "tail": "none",
                        "length": 15.0,
                        "width": 3.75,
                        "curve": 0.0
                    }
                }
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");

    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(50.0, 0.0)),
        "curve"
    );
    assert!(engine.update_hover_arrow_edit(Point::new(50.0, -30.0), false));

    let preview_object_ids: BTreeMap<String, usize> = engine
        .interaction_render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Line {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::FilledPath {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Polygon {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Rect {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Ellipse {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Polyline {
                    role: RenderRole::DocumentGraphic,
                    ..
                }
            )
        })
        .filter_map(|primitive| primitive_object_id(&primitive).map(str::to_string))
        .fold(BTreeMap::new(), |mut counts, object_id| {
            *counts.entry(object_id).or_default() += 1;
            counts
        });

    assert!(
        preview_object_ids.contains_key("obj_arrow"),
        "edited arrow should be rendered as the live preview: {preview_object_ids:?}"
    );
    assert_eq!(
        preview_object_ids.get("obj_other"),
        None,
        "unrelated objects must stay out of the edit preview so they are not hidden: {preview_object_ids:?}"
    );
}

#[test]
fn hollow_arrow_center_drag_curves_with_snap_and_smooth_rendering() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Hollow,
        arrow_head_size: ArrowHeadSize::Large,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(50.0, 0.0)),
        "curve"
    );
    assert!(engine.update_hover_arrow_edit(Point::new(50.0, -30.0), false));
    assert_eq!(engine.active_arrow_edit_degrees(), 120.0);
    assert!(engine.finish_hover_arrow_edit(Point::new(50.0, -30.0), false));

    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("hollow arrow object should exist");
    let object_id = object.id.clone();
    let arrow_head = object.payload.extra.get("arrowHead").unwrap();
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("hollow")
    );
    assert_eq!(
        arrow_head.get("curve").and_then(|value| value.as_f64()),
        Some(-120.0)
    );
    assert!(object.payload.extra.contains_key("arrowGeometry"));

    let (path_d, points) = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                object_id: primitive_object_id,
                role: RenderRole::DocumentGraphic,
                d,
                points,
                ..
            } if primitive_object_id.as_deref() == Some(object_id.as_str()) => Some((d, points)),
            _ => None,
        })
        .expect("curved hollow arrow should render as a smooth outline path");
    assert!(
        path_d.contains(" C "),
        "path should use cubic curves: {path_d}"
    );
    assert!(
        points.iter().any(|point| point.y.abs() > 1.0),
        "curved hollow outline should leave the straight chord: {points:?}"
    );
}

#[test]
fn hollow_arrow_center_drag_alt_disables_curve_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Hollow,
        arrow_head_size: ArrowHeadSize::Large,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(50.0, 0.0)),
        "curve"
    );
    assert!(engine.finish_hover_arrow_edit(Point::new(50.0, -30.0), true));

    let curve = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| object.payload.extra.get("arrowHead"))
        .and_then(|arrow_head| arrow_head.get("curve"))
        .and_then(|value| value.as_f64())
        .expect("hollow arrow should store an unsnapped curve");
    assert!(curve < -120.0 && curve > -125.0, "curve={curve}");
    assert!(
        (curve / 15.0).fract().abs() > 0.01,
        "alt drag should not snap to 15 degree increments: {curve}"
    );
}

#[test]
fn open_arrow_does_not_expose_center_curve_drag_handle() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Open,
        arrow_head_size: ArrowHeadSize::Large,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert_eq!(engine.begin_hover_arrow_edit(Point::new(50.0, 0.0)), "");
    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(100.0, 0.0)),
        "head"
    );
}

#[test]
fn selected_arrow_style_updates_from_arrow_toolbar_options() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));
    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    });
    engine.select_at_point(Point::new(50.0, 20.0), false);

    assert!(engine.apply_arrow_options_to_selection(
        ArrowVariant::Hollow,
        ArrowHeadSize::Small,
        ArrowCurve::Arc270,
        ArrowEndpointStyle::None,
        ArrowEndpointStyle::Full,
        false,
        true,
        true,
        ArrowNoGo::None,
    ));
    let object_id = &engine.state().selection.arrow_objects[0];
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| &object.id == object_id)
        .expect("selected arrow object should exist");

    assert_eq!(
        object
            .payload
            .extra
            .get("head")
            .and_then(|value| value.as_str()),
        Some("none")
    );
    assert_eq!(
        object
            .payload
            .extra
            .get("tail")
            .and_then(|value| value.as_str()),
        Some("start")
    );
    let arrow_head = object.payload.extra.get("arrowHead").unwrap();
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("hollow")
    );
    assert_eq!(
        arrow_head.get("tail").and_then(|value| value.as_str()),
        Some("full")
    );
    assert_eq!(
        arrow_head.get("bold").and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[test]
fn multi_arrow_line_and_endpoint_properties_roundtrip_through_history() {
    fn assert_arrow_properties(engine: &Engine, expected_bold: bool, expected_head: &str) {
        let arrows = engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.object_type == "line")
            .collect::<Vec<_>>();
        assert_eq!(arrows.len(), 2);
        for arrow in arrows {
            let arrow_head = arrow
                .payload
                .extra
                .get("arrowHead")
                .expect("arrow should retain its arrowHead payload");
            assert_eq!(
                arrow_head.get("bold").and_then(|value| value.as_bool()),
                Some(expected_bold),
                "{} bold state",
                arrow.id
            );
            assert_eq!(
                arrow_head.get("head").and_then(|value| value.as_str()),
                Some(expected_head),
                "{} endpoint state",
                arrow.id
            );
        }
    }

    fn assert_checked_menu_value(engine: &Engine, label: &str, command: &str, value: &str) {
        let menu: serde_json::Value = serde_json::from_str(
            &engine.context_menu_json(r#"{"kind":"object","objectId":"obj_line_1"}"#, false),
        )
        .expect("context menu should be valid JSON");
        let submenu = menu
            .as_array()
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("label").and_then(serde_json::Value::as_str) == Some(label)
                })
            })
            .and_then(|item| item.get("submenu"))
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| {
                panic!("{label} submenu should be available for two arrows: {menu}")
            });
        assert!(submenu.iter().any(|item| {
            item.get("command").and_then(serde_json::Value::as_str) == Some(command)
                && item.get("value").and_then(serde_json::Value::as_str) == Some(value)
                && item.get("checked").and_then(serde_json::Value::as_bool) == Some(true)
        }));
    }

    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));
    drag(&mut engine, Point::new(10.0, 60.0), Point::new(90.0, 60.0));
    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    });
    assert!(engine.select_all());
    assert!(
        engine.state().selection.molecule_objects.is_empty(),
        "select-all must not inject the default invisible empty editor molecule into a graphic-only selection"
    );
    assert_checked_menu_value(&engine, "Line Style", "line-style", "plain");
    assert_checked_menu_value(&engine, "Arrowheads", "arrow-endpoint", "head:full");

    assert!(engine.apply_line_style_to_selection("bold"));
    assert_arrow_properties(&engine, true, "full");
    assert_checked_menu_value(&engine, "Line Style", "line-style", "bold");
    assert!(engine.apply_arrow_options_to_selection(
        ArrowVariant::Solid,
        ArrowHeadSize::Small,
        ArrowCurve::Arc270,
        ArrowEndpointStyle::Left,
        ArrowEndpointStyle::None,
        true,
        false,
        true,
        ArrowNoGo::None,
    ));
    assert_arrow_properties(&engine, true, "half-left");
    assert_checked_menu_value(&engine, "Arrowheads", "arrow-endpoint", "head:left");

    assert!(engine.undo());
    assert_arrow_properties(&engine, true, "full");
    assert!(engine.undo());
    assert_arrow_properties(&engine, false, "full");
    assert!(engine.redo());
    assert_arrow_properties(&engine, true, "full");
    assert!(engine.redo());
    assert_arrow_properties(&engine, true, "half-left");
}

#[test]
fn locked_arrow_properties_are_immutable_in_a_mixed_selection() {
    fn arrow_state(engine: &Engine, object_id: &str) -> (bool, String, String) {
        let object = engine
            .state()
            .document
            .find_scene_object(object_id)
            .expect("arrow should remain in the document");
        let arrow_head = object
            .payload
            .extra
            .get("arrowHead")
            .expect("arrow should retain its arrowHead payload");
        (
            arrow_head
                .get("bold")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            arrow_head
                .get("head")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("none")
                .to_string(),
            arrow_head
                .get("tail")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("none")
                .to_string(),
        )
    }

    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));
    drag(&mut engine, Point::new(10.0, 60.0), Point::new(90.0, 60.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(50.0, 20.0), false);
    assert_eq!(engine.state().selection.arrow_objects, vec!["obj_line_1"]);
    assert!(engine.set_selection_locked(true));
    assert!(engine.select_all());

    assert!(engine.apply_line_style_to_selection("bold"));
    assert_eq!(
        arrow_state(&engine, "obj_line_1"),
        (false, "full".into(), "none".into())
    );
    assert_eq!(
        arrow_state(&engine, "obj_line_2"),
        (true, "full".into(), "none".into())
    );

    assert!(engine.apply_arrow_endpoints_to_selection(
        Some(ArrowEndpointStyle::Right),
        Some(ArrowEndpointStyle::Left),
    ));
    assert_eq!(
        arrow_state(&engine, "obj_line_1"),
        (false, "full".into(), "none".into())
    );
    assert_eq!(
        arrow_state(&engine, "obj_line_2"),
        (true, "half-right".into(), "half-left".into())
    );

    assert!(engine.undo());
    assert_eq!(
        arrow_state(&engine, "obj_line_1"),
        (false, "full".into(), "none".into())
    );
    assert_eq!(
        arrow_state(&engine, "obj_line_2"),
        (true, "full".into(), "none".into())
    );
    assert!(engine.undo());
    assert_eq!(
        arrow_state(&engine, "obj_line_1"),
        (false, "full".into(), "none".into())
    );
    assert_eq!(
        arrow_state(&engine, "obj_line_2"),
        (false, "full".into(), "none".into())
    );
    assert!(engine.redo());
    assert!(engine.redo());
    assert_eq!(
        arrow_state(&engine, "obj_line_1"),
        (false, "full".into(), "none".into())
    );
    assert_eq!(
        arrow_state(&engine, "obj_line_2"),
        (true, "half-right".into(), "half-left".into())
    );
}

#[test]
fn curved_arrow_tool_stores_curve_and_renders_arc_segments() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::CurvedMirror,
        arrow_curve: ArrowCurve::Arc120,
        ..ToolState::default()
    });

    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("created curved arrow object should exist");
    let arrow_head = object.payload.extra.get("arrowHead").unwrap();
    assert_eq!(
        arrow_head.get("kind").and_then(|value| value.as_str()),
        Some("curved-mirror")
    );
    assert_eq!(
        arrow_head.get("curve").and_then(|value| value.as_f64()),
        Some(120.0)
    );
    let arrow_geometry = object
        .payload
        .extra
        .get("arrowGeometry")
        .expect("created curved arrow should store arc geometry");
    assert!(arrow_geometry.get("center").is_some());
    assert!(arrow_geometry.get("majorAxisEnd").is_some());
    assert!(arrow_geometry.get("minorAxisEnd").is_some());
    let arc_points = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                points,
                ..
            } if points.len() > 2 => Some(points),
            _ => None,
        })
        .expect("curved arrow should render as a smooth path with sampled bounds points");
    assert!(arc_points[arc_points.len() / 2].y > arc_points[0].y);
}

#[test]
fn selected_curved_arrow_box_wraps_visual_arc_and_head() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Curved,
        arrow_curve: ArrowCurve::Arc270,
        ..ToolState::default()
    });

    drag(
        &mut engine,
        Point::new(100.0, 100.0),
        Point::new(140.0, 100.0),
    );

    let object_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("created curved arrow object should exist")
        .id
        .clone();
    let visual_bounds = rendered_object_bounds(&engine, &object_id);
    assert!(
        visual_bounds[0] < 100.0,
        "fixture should exercise an arc that extends left of the endpoint handles: {visual_bounds:?}"
    );

    engine.set_tool_state(select_tool());

    assert_eq!(engine.state().selection.arrow_objects, vec![object_id]);
    assert_bounds_contains(selection_box_bounds(&engine), visual_bounds);
}

#[test]
fn selected_curved_equilibrium_arrow_box_wraps_both_branches() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Equilibrium,
        arrow_head_style: ArrowEndpointStyle::Left,
        arrow_tail_style: ArrowEndpointStyle::Left,
        ..ToolState::default()
    });

    drag(&mut engine, Point::new(40.0, 80.0), Point::new(120.0, 80.0));
    assert_eq!(
        engine.begin_hover_arrow_edit(Point::new(80.0, 80.0)),
        "curve"
    );
    assert!(engine.finish_hover_arrow_edit(Point::new(80.0, 56.0), false));

    let object_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("created curved equilibrium arrow object should exist")
        .id
        .clone();
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("created curved equilibrium arrow object should exist");
    assert_eq!(
        object
            .payload
            .extra
            .get("arrowHead")
            .and_then(|arrow_head| arrow_head.get("curve"))
            .and_then(|value| value.as_f64()),
        Some(-120.0)
    );
    let smooth_branch_count = engine
        .render_list()
        .into_iter()
        .filter(|primitive| match primitive {
            RenderPrimitive::Path {
                object_id: primitive_object_id,
                role: RenderRole::DocumentGraphic,
                d,
                ..
            } => primitive_object_id.as_deref() == Some(object_id.as_str()) && d.contains(" C "),
            _ => false,
        })
        .count();
    assert!(
        smooth_branch_count >= 2,
        "curved equilibrium branches should render as smooth paths"
    );
    engine.clear_interaction();
    let visual_bounds = rendered_object_bounds(&engine, &object_id);

    engine.set_tool_state(select_tool());

    assert_eq!(engine.state().selection.arrow_objects, vec![object_id]);
    assert_bounds_contains(selection_box_bounds(&engine), visual_bounds);
}

#[test]
fn selected_bracket_and_symbol_boxes_wrap_visual_geometry() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_graphic_selection_visual_bounds",
            "title": "graphic selection visual bounds",
            "page": { "width": 220.0, "height": 120.0, "background": "#ffffff" }
        },
        "objects": [
            {
                "id": "obj_round_bracket",
                "type": "bracket",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [40.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 18.0, 70.0],
                    "kind": "round",
                    "stroke": "#000000",
                    "strokeWidth": 1.0
                }
            },
            {
                "id": "obj_circle_plus",
                "type": "symbol",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [130.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 20.0, 20.0],
                    "kind": "circle-plus",
                    "fill": "#000000",
                    "strokeWidth": 1.0
                }
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("graphic selection document should load");

    let bracket_bounds = rendered_object_bounds(&engine, "obj_round_bracket");
    assert!(
        bracket_bounds[0] < 40.0,
        "fixture should exercise round bracket geometry outside the stored bbox: {bracket_bounds:?}"
    );
    let symbol_bounds = rendered_object_bounds(&engine, "obj_circle_plus");
    assert!(
        symbol_bounds[0] < 130.0,
        "fixture should exercise symbol stroke outside the stored bbox: {symbol_bounds:?}"
    );

    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(36.5, 55.0), false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_round_bracket".to_string()]
    );
    assert_bounds_contains(selection_box_bounds(&engine), bracket_bounds);

    engine.select_at_point(Point::new(140.0, 50.0), false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_circle_plus".to_string()]
    );
    assert_bounds_contains(selection_box_bounds(&engine), symbol_bounds);
}

#[test]
fn dragging_one_selected_bracket_does_not_move_sibling_brackets() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_single_bracket_drag",
            "title": "single bracket drag",
            "page": { "width": 260.0, "height": 120.0, "background": "#ffffff" }
        },
        "objects": [
            {
                "id": "obj_bracket_a",
                "type": "bracket",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [40.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 18.0, 70.0],
                    "kind": "round",
                    "stroke": "#000000",
                    "strokeWidth": 1.0
                }
            },
            {
                "id": "obj_bracket_b",
                "type": "bracket",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [150.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 18.0, 70.0],
                    "kind": "round",
                    "stroke": "#000000",
                    "strokeWidth": 1.0
                }
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("bracket drag document should load");
    engine.set_tool_state(select_tool());

    let start = Point::new(36.5, 55.0);
    let end = Point::new(48.5, 61.0);
    engine.select_at_point(start, false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_bracket_a".to_string()]
    );
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let bracket_a = engine
        .state()
        .document
        .find_scene_object("obj_bracket_a")
        .expect("selected bracket should remain");
    let bracket_b = engine
        .state()
        .document
        .find_scene_object("obj_bracket_b")
        .expect("sibling bracket should remain");
    assert_eq!(bracket_a.transform.translate, [52.0, 26.0]);
    assert_eq!(bracket_b.transform.translate, [150.0, 20.0]);
}

#[test]
fn select_tool_bracket_side_hit_testing_ignores_interior_space() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_bracket_side_hit_testing",
            "title": "bracket side hit testing",
            "page": { "width": 240.0, "height": 120.0, "background": "#ffffff" }
        },
        "objects": [
            {
                "id": "obj_bracket_group",
                "type": "group",
                "name": "bracket-group",
                "visible": true,
                "locked": false,
                "zIndex": 9,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "meta": { "kind": "bracket-group" },
                "payload": { "bbox": [40.0, 20.0, 118.0, 70.0] },
                "children": [
                    {
                        "id": "obj_left_bracket",
                        "type": "bracket",
                        "visible": true,
                        "zIndex": 10,
                        "transform": { "translate": [40.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "payload": {
                            "bbox": [0.0, 0.0, 18.0, 70.0],
                            "kind": "square",
                            "side": "left",
                            "stroke": "#000000",
                            "strokeWidth": 1.0
                        }
                    },
                    {
                        "id": "obj_right_bracket",
                        "type": "bracket",
                        "visible": true,
                        "zIndex": 11,
                        "transform": { "translate": [140.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                        "payload": {
                            "bbox": [0.0, 0.0, 18.0, 70.0],
                            "kind": "square",
                            "side": "right",
                            "stroke": "#000000",
                            "strokeWidth": 1.0
                        }
                    }
                ]
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("bracket side document should load");
    engine.set_tool_state(select_tool());

    let left_interior = Point::new(49.0, 55.0);
    engine.pointer_move(PointerEvent {
        x: left_interior.x,
        y: left_interior.y,
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_shape.is_none());
    engine.select_at_point(left_interior, false);
    assert!(engine.state().selection.arrow_objects.is_empty());
    assert!(!engine.begin_selection_move_at_point(left_interior, false, false));

    let between_sides = Point::new(100.0, 55.0);
    engine.select_at_point(between_sides, false);
    assert!(engine.state().selection.arrow_objects.is_empty());
    assert!(!engine.begin_selection_move_at_point(between_sides, false, false));

    let left_stroke = Point::new(40.5, 55.0);
    engine.pointer_move(PointerEvent {
        x: left_stroke.x,
        y: left_stroke.y,
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_shape.is_some());
    assert_eq!(
        engine.hover_shape_action_at_point(left_stroke),
        "",
        "bracket side strokes should select/move the bracket, not start endpoint resize"
    );
    assert_eq!(engine.begin_hover_shape_edit(left_stroke), "");

    engine.select_in_rect(Point::new(49.0, 55.0), Point::new(55.0, 60.0), false);
    assert!(
        engine.state().selection.arrow_objects.is_empty(),
        "a region inside the bracket's empty side bbox should not select the bracket"
    );
    engine.select_in_rect(Point::new(39.0, 54.0), Point::new(42.0, 58.0), false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_left_bracket".to_string()]
    );

    engine.select_at_point(left_stroke, false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_left_bracket".to_string()]
    );
    let left_near_edge = Point::new(40.0 + GRAPHIC_EDGE_HIT_RADIUS + 0.25, 55.0);
    engine.select_at_point(left_near_edge, false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_left_bracket".to_string()]
    );
    let left_far_edge = Point::new(40.0 + GRAPHIC_EDGE_HIT_RADIUS + 0.75, 55.0);
    engine.select_at_point(left_far_edge, false);
    assert!(engine.state().selection.arrow_objects.is_empty());
    engine.select_at_point(Point::new(157.5, 55.0), false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_right_bracket".to_string()]
    );
}
