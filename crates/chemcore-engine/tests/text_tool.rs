use chemcore_engine::{
    BondAnchor, BondVariant, Engine, Point, PointerEvent, TextEditTarget, Tool, ToolState,
};

fn click(engine: &mut Engine, x: f64, y: f64) {
    engine.pointer_down(PointerEvent {
        x,
        y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x,
        y,
        button: Some(0),
        alt_key: false,
    });
}

fn tool_state(bond_variant: BondVariant) -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant,
    }
}

fn free_anchor(point: Point) -> BondAnchor {
    BondAnchor {
        node_id: None,
        point,
        label_anchor: None,
    }
}

fn node_anchor(node_id: &str, point: Point) -> BondAnchor {
    BondAnchor {
        node_id: Some(node_id.to_string()),
        point,
        label_anchor: None,
    }
}

#[test]
fn begin_and_apply_text_object_edit_creates_text_scene_object() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(Point::new(120.0, 88.0))
        .expect("text session should be created");

    match &session.target {
        TextEditTarget::TextObject { object_id, x, y } => {
            assert!(object_id.is_none());
            assert_eq!((*x, *y), (120.0, 88.0));
        }
        other => panic!("unexpected target: {other:?}"),
    }

    let changed = engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "reaction note".to_string(),
        ..session
    });
    assert!(changed);

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    assert_eq!(
        text_object
            .payload
            .extra
            .get("text")
            .and_then(serde_json::Value::as_str),
        Some("reaction note")
    );
}

#[test]
fn endpoint_text_edit_defaults_to_chemical_and_formats_charge() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let click_x = node.position[0] + 6.0;
    let click_y = node.position[1] + 4.0;
    let session = engine
        .begin_text_edit(Point::new(click_x, click_y))
        .expect("endpoint session should be created");
    assert!(session.default_chemical);
    match &session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert_eq!((*x, *y), (node.position[0], node.position[1]));
        }
        other => panic!("unexpected target: {other:?}"),
    }

    let changed = engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Fe2+".to_string(),
        source_runs: Vec::new(),
        ..session
    });
    assert!(changed);

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should be generated");
    assert_eq!(label.text, "Fe2+");
    assert_eq!(label.runs.len(), 2);
    assert_eq!(label.align.as_deref(), Some("left"));
    assert_eq!(label.runs[0].text, "Fe");
    assert_eq!(label.runs[0].script.as_deref(), Some("normal"));
    assert_eq!(label.runs[1].text, "2+");
    assert_eq!(label.runs[1].script.as_deref(), Some("superscript"));
}

#[test]
fn endpoint_text_edit_populates_kernel_glyph_polygons_for_abbreviation_labels() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    let changed = engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    });
    assert!(changed);

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should be generated");
    assert_eq!(label.text, "Ph");
    assert_eq!(label.glyph_polygons.len(), 2, "{:?}", label.glyph_polygons);
    assert_eq!(
        label.glyph_polygons[0].len(),
        8,
        "{:?}",
        label.glyph_polygons[0]
    );
    assert_eq!(
        label.glyph_polygons[1].len(),
        8,
        "{:?}",
        label.glyph_polygons[1]
    );
}

#[test]
fn preview_text_runs_expands_chemical_source_runs_in_kernel() {
    let engine = Engine::new();
    let session = chemcore_engine::TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 0.0,
            y: 0.0,
        },
        text: "Fe2+".to_string(),
        source_runs: vec![chemcore_engine::LabelRun {
            text: "Fe2+".to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(12.0),
            fill: Some("#000000".to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("chemical".to_string()),
            face: None,
        }],
        font_family: Some("Arial".to_string()),
        font_size: Some(12.0),
        fill: Some("#000000".to_string()),
        align: Some("left".to_string()),
        line_height: Some(12.6),
        box_value: None,
        anchor_offset: None,
        measured_size: None,
        preserve_lines: true,
        default_chemical: true,
    };

    let (source_runs, display_runs) = engine.preview_text_runs(&session);
    assert_eq!(source_runs.len(), 1);
    assert_eq!(source_runs[0].script.as_deref(), Some("chemical"));
    assert_eq!(display_runs.len(), 2);
    assert_eq!(display_runs[0].text, "Fe");
    assert_eq!(display_runs[0].script.as_deref(), Some("normal"));
    assert_eq!(display_runs[1].text, "2+");
    assert_eq!(display_runs[1].script.as_deref(), Some("superscript"));
}

#[test]
fn reopening_existing_endpoint_label_uses_stable_label_anchor() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .cloned()
        .expect("node should exist");

    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label = node.label.as_ref().expect("label should exist");
    let polygon = &label.glyph_polygons[0];
    let min_x = polygon
        .iter()
        .map(|point| point[0])
        .fold(f64::INFINITY, f64::min);
    let max_x = polygon
        .iter()
        .map(|point| point[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = polygon
        .iter()
        .map(|point| point[1])
        .fold(f64::INFINITY, f64::min);
    let max_y = polygon
        .iter()
        .map(|point| point[1])
        .fold(f64::NEG_INFINITY, f64::max);

    let reopened = engine
        .begin_text_edit(Point::new(node.position[0] + 9.0, node.position[1] + 7.0))
        .expect("existing label session should be created");
    match reopened.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - ((min_x + max_x) * 0.5)).abs() < 0.001, "{x}");
            assert!((y - ((min_y + max_y) * 0.5)).abs() < 0.001, "{y}");
        }
        other => panic!("unexpected target: {other:?}"),
    }
    assert!(reopened.anchor_offset.is_some());
}

#[test]
fn endpoint_label_anchor_tracks_terminal_double_status() {
    let mut engine = Engine::new();
    engine.set_tool_state(tool_state(BondVariant::Double));
    assert!(engine.add_bond_between(
        free_anchor(Point::new(100.0, 100.0)),
        free_anchor(Point::new(140.0, 100.0)),
        2,
    ));

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let bond = entry.fragment.bonds.first().expect("bond should exist");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Right)
    );
    let node = entry
        .fragment
        .nodes
        .iter()
        .max_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("terminal node should exist")
        .clone();
    let terminal_session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    let terminal_anchor = match terminal_session.target.clone() {
        TextEditTarget::EndpointLabel { x, y, .. } => Point::new(x, y),
        other => panic!("unexpected target: {other:?}"),
    };
    assert!(
        (terminal_anchor.x - node.position[0]).abs() > 0.001
            || (terminal_anchor.y - node.position[1]).abs() > 0.001,
        "{terminal_anchor:?} vs {:?}",
        node.position
    );
    assert!(
        (terminal_anchor.x - node.position[0]).abs() < 0.001,
        "{terminal_anchor:?} vs {:?}",
        node.position
    );
    assert!(
        terminal_anchor.y > node.position[1],
        "{terminal_anchor:?} vs {:?}",
        node.position
    );
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..terminal_session
    }));

    let reopened_terminal = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("existing endpoint label session should be created");
    match reopened_terminal.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - terminal_anchor.x).abs() < 0.01, "{x} vs {}", terminal_anchor.x);
            assert!((y - terminal_anchor.y).abs() < 0.01, "{y} vs {}", terminal_anchor.y);
        }
        other => panic!("unexpected target: {other:?}"),
    }

    engine.set_tool_state(tool_state(BondVariant::Single));
    assert!(engine.add_single_bond_between(
        node_anchor(&node.id, Point::new(node.position[0], node.position[1])),
        free_anchor(Point::new(172.0, 128.0)),
    ));

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let node_after_attachment = entry
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == node.id)
        .expect("terminal node should still exist")
        .position;
    let attached_session = engine
        .begin_text_edit(Point::new(node_after_attachment[0], node_after_attachment[1]))
        .expect("attached endpoint label session should be created");
    match attached_session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - node_after_attachment[0]).abs() < 0.001, "{x}");
            assert!((y - node_after_attachment[1]).abs() < 0.001, "{y}");
        }
        other => panic!("unexpected target: {other:?}"),
    }
}

#[test]
fn endpoint_label_reanchors_when_double_bond_style_changes() {
    let mut engine = Engine::new();
    engine.set_tool_state(tool_state(BondVariant::Double));
    assert!(engine.add_bond_between(
        free_anchor(Point::new(100.0, 100.0)),
        free_anchor(Point::new(140.0, 100.0)),
        2,
    ));

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let node = entry
        .fragment
        .nodes
        .iter()
        .max_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("terminal node should exist")
        .clone();
    let bond_id = entry.fragment.bonds.first().expect("bond should exist").id.clone();
    let session = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    for _ in 0..3 {
        assert!(engine.cycle_bond_center_style(&bond_id));
        let entry = engine
            .state()
            .document
            .editable_fragment()
            .expect("editable fragment should exist");
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .expect("bond should exist");
        if bond.double.as_ref().map(|double| double.placement)
            == Some(chemcore_engine::DoubleBondPlacement::Center)
        {
            break;
        }
    }

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should exist");
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(chemcore_engine::DoubleBondPlacement::Center)
    );
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == node.id)
        .expect("terminal node should exist")
        .position;
    let centered_session = engine
        .begin_text_edit(Point::new(node[0], node[1]))
        .expect("centered endpoint label session should be created");
    match centered_session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - node[0]).abs() < 0.001, "{x}");
            assert!((y - node[1]).abs() < 0.001, "{y}");
        }
        other => panic!("unexpected target: {other:?}"),
    }
}

#[test]
fn text_mode_hover_prefers_label_box_over_endpoint_focus() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(Point::new(300.0, 260.0))
        .expect("endpoint session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .first()
        .expect("node should exist");
    let label_box = node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("label box");

    engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
    });
    engine.pointer_move(PointerEvent {
        x: (label_box[0] + label_box[2]) * 0.5,
        y: (label_box[1] + label_box[3]) * 0.5,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_some());
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn text_mode_hover_focuses_plain_text_box_bounds() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(Point::new(120.0, 88.0))
        .expect("text session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "note".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let object_id = text_object.id.clone();
    let translate = text_object.transform.translate;
    let bounds = text_object.payload.bbox.expect("text bbox");

    engine.set_tool_state(ToolState {
        active_tool: Tool::Text,
        bond_variant: BondVariant::Single,
    });
    engine.pointer_move(PointerEvent {
        x: translate[0] + bounds[2] * 0.5,
        y: translate[1] + bounds[3] * 0.5,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_text_box
        .as_ref()
        .expect("text hover box should exist");
    assert_eq!(hover.object_id.as_deref(), Some(object_id.as_str()));
    assert!(engine.state().overlay.hover_endpoint.is_none());
}
