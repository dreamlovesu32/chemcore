use chemcore_engine::{
    BondAnchor, BondVariant, Engine, Point, PointerEvent, TextEditLayoutRequest, TextEditSelection,
    TextEditTarget, Tool, ToolState,
};

fn px(value: f64) -> f64 {
    chemcore_engine::px_to_cm(value)
}

fn px_point(x: f64, y: f64) -> Point {
    Point::new(px(x), px(y))
}

fn click(engine: &mut Engine, x: f64, y: f64) {
    engine.pointer_down(PointerEvent {
        x: px(x),
        y: px(y),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(x),
        y: px(y),
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

fn delete_tool_state() -> ToolState {
    ToolState {
        active_tool: Tool::Delete,
        bond_variant: BondVariant::Single,
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

fn approx_eq(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() <= tolerance,
        "left={left:?} right={right:?} tolerance={tolerance:?}"
    );
}

#[test]
fn begin_and_apply_text_object_edit_creates_text_scene_object() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    match &session.target {
        TextEditTarget::TextObject { object_id, x, y } => {
            assert!(object_id.is_none());
            assert_eq!((*x, *y), (px(120.0), px(88.0)));
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
fn preview_text_edit_layout_returns_kernel_caret_and_selection_geometry() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session: chemcore_engine::TextEditSession {
            text: "Hello".to_string(),
            ..session
        },
        selection: Some(TextEditSelection {
            anchor: 2,
            focus: 5,
        }),
    });

    assert_eq!(layout.text, "Hello");
    assert_eq!(layout.anchor_offset, [0.0, 0.0]);
    assert_eq!(layout.lines.len(), 1);
    assert_eq!(layout.lines[0].start_offset, 0);
    assert_eq!(layout.lines[0].end_offset, 5);
    assert_eq!(layout.caret_positions.len(), 6);
    assert_eq!(
        layout.caret_positions.last().map(|caret| caret.offset),
        Some(5)
    );
    assert_eq!(
        layout.selection.as_ref().map(|selection| selection.start),
        Some(2)
    );
    assert_eq!(
        layout.selection.as_ref().map(|selection| selection.end),
        Some(5)
    );
    assert_eq!(layout.selection_rects.len(), 1);
    assert!(layout.width >= px(8.0));
    assert!(layout.height >= layout.line_height);
}

#[test]
fn reopening_text_object_preserves_default_font_size_precision() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");

    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "reaction note".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");

    let stored_font_size = text_object
        .payload
        .extra
        .get("fontSize")
        .and_then(serde_json::Value::as_f64)
        .expect("text object should persist font size");
    assert!((stored_font_size - chemcore_engine::DEFAULT_TEXT_FONT_SIZE_CM).abs() < 1.0e-6);

    let reopened = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text object session should reopen");
    let reopened_font_size = reopened.font_size.expect("reopened session font size");
    assert!((reopened_font_size - chemcore_engine::DEFAULT_TEXT_FONT_SIZE_CM).abs() < 1.0e-6);
}

#[test]
fn delete_tool_click_on_text_box_removes_text_object() {
    let mut engine = Engine::new();
    let session = engine
        .begin_text_edit(px_point(120.0, 88.0))
        .expect("text session should be created");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "reaction note".to_string(),
        ..session
    }));

    let text_object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("text object should exist");
    let translate = text_object.transform.translate;
    let bounds = text_object.payload.bbox.expect("text bbox");

    engine.set_tool_state(delete_tool_state());
    click(
        &mut engine,
        chemcore_engine::cm_to_px(translate[0] + bounds[2] * 0.5),
        chemcore_engine::cm_to_px(translate[1] + bounds[3] * 0.5),
    );
    assert!(engine
        .state()
        .document
        .objects
        .iter()
        .all(|object| object.object_type != "text"));
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

    let click_x = node.position[0] + px(6.0);
    let click_y = node.position[1] + px(4.0);
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
fn preview_text_edit_layout_matches_committed_endpoint_label_geometry() {
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
    let edited_session = chemcore_engine::TextEditSession {
        text: "H2SO4".to_string(),
        source_runs: Vec::new(),
        ..session.clone()
    };

    let layout = engine.preview_text_edit_layout(&TextEditLayoutRequest {
        session: edited_session.clone(),
        selection: Some(TextEditSelection {
            anchor: 5,
            focus: 5,
        }),
    });

    assert!(engine.apply_text_edit(edited_session));
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
    let box_value = label.box_field.expect("label box should exist");
    approx_eq(layout.width, box_value[2] - box_value[0], 0.02);
    approx_eq(layout.height, box_value[3] - box_value[1], 0.02);
    approx_eq(
        layout.anchor_offset[0],
        session.target.world_point().x.value() - box_value[0],
        0.02,
    );
    approx_eq(
        layout.anchor_offset[1],
        session.target.world_point().y.value() - box_value[1],
        0.02,
    );
    assert_eq!(
        layout.lines.first().map(|line| {
            line.runs
                .iter()
                .map(|run| (run.text.clone(), run.script.clone().unwrap_or_default()))
                .collect::<Vec<_>>()
        }),
        Some(
            label
                .runs
                .iter()
                .map(|run| (run.text.clone(), run.script.clone().unwrap_or_default()))
                .collect::<Vec<_>>()
        )
    );
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
fn reopening_endpoint_label_session_preserves_bbox_and_anchor_precision() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
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
        .cloned()
        .expect("node should exist");
    let label_box = node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("label box");

    let reopened = engine
        .begin_text_edit(Point::new(node.position[0], node.position[1]))
        .expect("endpoint session should reopen");
    let box_value = reopened.box_value.expect("session box");
    let anchor_offset = reopened.anchor_offset.expect("session anchor offset");

    assert!((box_value[0] - label_box[0]).abs() < 1.0e-6);
    assert!((box_value[1] - label_box[1]).abs() < 1.0e-6);
    assert!((box_value[2] - label_box[2]).abs() < 1.0e-6);
    assert!((box_value[3] - label_box[3]).abs() < 1.0e-6);
    assert!(anchor_offset[0].abs() < 1.0e-6);
    assert!(anchor_offset[1].abs() < 1.0e-6);
}

#[test]
fn endpoint_text_edit_ignores_implausible_dom_label_measurements() {
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
        text: "N".to_string(),
        source_runs: Vec::new(),
        box_value: Some([0.0, 0.0, px(3000.0), px(3000.0)]),
        anchor_offset: Some([px(2000.0), px(2000.0)]),
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
    let label = node.label.as_ref().expect("label should be generated");
    let box_value = label.bbox().expect("label should have bounds");
    let glyph_box = label.glyph_polygons.iter().flatten().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |bounds, point| {
            [
                bounds[0].min(point[0]),
                bounds[1].min(point[1]),
                bounds[2].max(point[0]),
                bounds[3].max(point[1]),
            ]
        },
    );
    assert!((box_value[2] - box_value[0]) < px(24.0), "{box_value:?}");
    assert!((box_value[3] - box_value[1]) < px(24.0), "{box_value:?}");
    assert!((glyph_box[2] - glyph_box[0]) < px(24.0), "{glyph_box:?}");
    assert!((glyph_box[3] - glyph_box[1]) < px(24.0), "{glyph_box:?}");
    assert!(
        ((box_value[0] + box_value[2]) * 0.5 - node.position[0]).abs() < px(12.0),
        "{box_value:?} vs {:?}",
        node.position
    );
    assert!(
        ((box_value[1] + box_value[3]) * 0.5 - node.position[1]).abs() < px(12.0),
        "{box_value:?} vs {:?}",
        node.position
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
    let box_value = label.bbox().expect("label should have bbox");

    let reopened = engine
        .begin_text_edit(Point::new(
            node.position[0] + px(9.0),
            node.position[1] + px(7.0),
        ))
        .expect("existing label session should be created");
    match reopened.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - box_value[0]).abs() < 0.001, "{x}");
            assert!((y - box_value[1]).abs() < 0.001, "{y}");
        }
        other => panic!("unexpected target: {other:?}"),
    }
    assert_eq!(reopened.anchor_offset, Some([0.0, 0.0]));
}

#[test]
fn endpoint_label_anchor_tracks_terminal_double_status() {
    let mut engine = Engine::new();
    engine.set_tool_state(tool_state(BondVariant::Double));
    assert!(engine.add_bond_between(
        free_anchor(px_point(100.0, 100.0)),
        free_anchor(px_point(140.0, 100.0)),
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
    let terminal_node = entry
        .fragment
        .nodes
        .iter()
        .max_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("terminal node should exist")
        .clone();
    let terminal_session = engine
        .begin_text_edit(Point::new(
            terminal_node.position[0],
            terminal_node.position[1],
        ))
        .expect("endpoint session should be created");
    let terminal_anchor = match terminal_session.target.clone() {
        TextEditTarget::EndpointLabel { x, y, .. } => Point::new(x, y),
        other => panic!("unexpected target: {other:?}"),
    };
    assert!(
        (terminal_anchor.x - terminal_node.position[0]).abs() > 0.001
            || (terminal_anchor.y - terminal_node.position[1]).abs() > 0.001,
        "{terminal_anchor:?} vs {:?}",
        terminal_node.position
    );
    assert!(
        (terminal_anchor.x - terminal_node.position[0]).abs() < 0.001,
        "{terminal_anchor:?} vs {:?}",
        terminal_node.position
    );
    assert!(
        terminal_anchor.y > terminal_node.position[1],
        "{terminal_anchor:?} vs {:?}",
        terminal_node.position
    );
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "Ph".to_string(),
        source_runs: Vec::new(),
        ..terminal_session
    }));

    let reopened_terminal = engine
        .begin_text_edit(Point::new(
            terminal_node.position[0],
            terminal_node.position[1],
        ))
        .expect("existing endpoint label session should be created");
    let reopened_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should still exist");
    let reopened_label_box = reopened_node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("reopened label box");
    match reopened_terminal.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!(
                (x - reopened_label_box[0]).abs() < 0.01,
                "{x} vs {}",
                reopened_label_box[0]
            );
            assert!(
                (y - reopened_label_box[1]).abs() < 0.01,
                "{y} vs {}",
                reopened_label_box[1]
            );
        }
        other => panic!("unexpected target: {other:?}"),
    }

    engine.set_tool_state(tool_state(BondVariant::Single));
    assert!(engine.add_single_bond_between(
        node_anchor(
            &terminal_node.id,
            Point::new(terminal_node.position[0], terminal_node.position[1]),
        ),
        free_anchor(px_point(172.0, 128.0)),
    ));

    let node_after_attachment = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should still exist")
        .position;
    let attached_session = engine
        .begin_text_edit(Point::new(
            node_after_attachment[0],
            node_after_attachment[1],
        ))
        .expect("attached endpoint label session should be created");
    let attached_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should still exist");
    let attached_label_box = attached_node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("attached label box");
    match attached_session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - attached_label_box[0]).abs() < 0.01, "{x}");
            assert!((y - attached_label_box[1]).abs() < 0.01, "{y}");
        }
        other => panic!("unexpected target: {other:?}"),
    }
}

#[test]
fn endpoint_label_reanchors_when_double_bond_style_changes() {
    let mut engine = Engine::new();
    engine.set_tool_state(tool_state(BondVariant::Double));
    assert!(engine.add_bond_between(
        free_anchor(px_point(100.0, 100.0)),
        free_anchor(px_point(140.0, 100.0)),
        2,
    ));

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist");
    let terminal_node = entry
        .fragment
        .nodes
        .iter()
        .max_by(|left, right| left.position[0].total_cmp(&right.position[0]))
        .expect("terminal node should exist")
        .clone();
    let bond_id = entry
        .fragment
        .bonds
        .first()
        .expect("bond should exist")
        .id
        .clone();
    let session = engine
        .begin_text_edit(Point::new(
            terminal_node.position[0],
            terminal_node.position[1],
        ))
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
    let terminal_node_position = entry
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should exist")
        .position;
    let centered_session = engine
        .begin_text_edit(Point::new(
            terminal_node_position[0],
            terminal_node_position[1],
        ))
        .expect("centered endpoint label session should be created");
    let centered_node = engine
        .state()
        .document
        .editable_fragment()
        .expect("editable fragment should exist")
        .fragment
        .nodes
        .iter()
        .find(|candidate| candidate.id == terminal_node.id)
        .expect("terminal node should exist");
    let centered_label_box = centered_node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("centered label box");
    match centered_session.target {
        TextEditTarget::EndpointLabel { x, y, .. } => {
            assert!((x - centered_label_box[0]).abs() < 0.01, "{x}");
            assert!((y - centered_label_box[1]).abs() < 0.01, "{y}");
        }
        other => panic!("unexpected target: {other:?}"),
    }
}

#[test]
fn text_mode_hover_prefers_label_box_over_endpoint_focus() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 260.0);
    let session = engine
        .begin_text_edit(px_point(300.0, 260.0))
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
        .begin_text_edit(px_point(120.0, 88.0))
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
