use chemcore_engine::{
    BondVariant, DoubleBondPlacement, Engine, PointerEvent, RenderPrimitive, Tool, ToolState,
    BOND_CENTER_FOCUS_LENGTH, BOND_CENTER_FOCUS_WIDTH, DEFAULT_BOND_LENGTH, DEFAULT_BOND_STROKE,
    ENDPOINT_FOCUS_RADIUS,
};
use std::collections::BTreeMap;

fn bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Single,
    }
}

fn fragment_counts(engine: &Engine) -> (usize, usize) {
    let entry = engine.state().document.editable_fragment().unwrap();
    (entry.fragment.nodes.len(), entry.fragment.bonds.len())
}

fn node_degrees(engine: &Engine) -> BTreeMap<String, usize> {
    let entry = engine.state().document.editable_fragment().unwrap();
    let mut degrees = BTreeMap::new();
    for node in &entry.fragment.nodes {
        degrees.insert(node.id.clone(), 0);
    }
    for bond in &entry.fragment.bonds {
        *degrees.entry(bond.begin.clone()).or_insert(0) += 1;
        *degrees.entry(bond.end.clone()).or_insert(0) += 1;
    }
    degrees
}

#[test]
fn click_on_blank_canvas_creates_horizontal_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.nodes[0].position, [300.0, 260.0]);
    assert_eq!(entry.fragment.nodes[1].position, [336.0, 260.0]);
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn hover_focuses_existing_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });

    engine.pointer_move(PointerEvent {
        x: 337.0,
        y: 263.0,
        button: None,
    });

    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.point.x, 336.0);
    assert_eq!(hover.point.y, 260.0);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle { radius, .. } if (*radius - ENDPOINT_FOCUS_RADIUS).abs() < 0.001
    )));
}

#[test]
fn click_on_single_bond_endpoint_extends_at_120_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!((point[0] - 354.0).abs() < 0.01, "{point:?}");
    assert!((point[1] - 228.82).abs() < 0.01, "{point:?}");
}

#[test]
fn drag_from_endpoint_uses_fixed_length_and_angle_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_move(PointerEvent {
        x: 370.0,
        y: 292.0,
        button: None,
    });
    assert!(engine.state().overlay.preview.is_some());
    engine.pointer_up(PointerEvent {
        x: 370.0,
        y: 292.0,
        button: Some(0),
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let length = ((last.position[0] - 336.0).powi(2) + (last.position[1] - 260.0).powi(2)).sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() < 0.01, "{length}");
    assert_eq!(fragment_counts(&engine), (3, 2));
}

#[test]
fn dragged_bond_endpoint_reuses_focused_existing_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_move(PointerEvent {
        x: 318.0,
        y: 228.82,
        button: None,
    });
    engine.pointer_up(PointerEvent {
        x: 318.0,
        y: 228.82,
        button: Some(0),
    });

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 228.82,
        button: Some(0),
    });
    engine.pointer_move(PointerEvent {
        x: 304.0,
        y: 263.0,
        button: None,
    });
    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.point.x, 300.0);
    assert_eq!(hover.point.y, 260.0);
    let preview = engine.state().overlay.preview.as_ref().unwrap();
    assert_eq!(preview.end.x, 300.0);
    assert_eq!(preview.end.y, 260.0);
    engine.pointer_up(PointerEvent {
        x: 304.0,
        y: 263.0,
        button: Some(0),
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    assert_eq!(entry.fragment.bonds.len(), 3);
    assert_eq!(
        node_degrees(&engine).values().copied().collect::<Vec<_>>(),
        vec![2, 2, 2]
    );

    engine.pointer_move(PointerEvent {
        x: 309.0,
        y: 244.41,
        button: None,
    });
    engine.pointer_down(PointerEvent {
        x: 309.0,
        y: 244.41,
        button: Some(0),
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let closed_bond = entry.fragment.bonds.last().unwrap();
    assert_eq!(closed_bond.order, 2);
    assert_ne!(
        closed_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center),
    );
}

#[test]
fn click_extension_reuses_endpoint_at_default_angle() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });

    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(200.0, 200.0),
        },
        chemcore_engine::Point::new(354.0, 228.82),
    );

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 4);
    assert_eq!(entry.fragment.bonds.len(), 3);
    let closing = entry.fragment.bonds.last().unwrap();
    assert!(matches!(
        (closing.begin.as_str(), closing.end.as_str()),
        ("n_2", "n_5") | ("n_5", "n_2")
    ));
    assert_eq!(node_degrees(&engine).get("n_2"), Some(&2));
    assert_eq!(node_degrees(&engine).get("n_5"), Some(&2));
}

#[test]
fn select_delete_and_undo_redo_round_trip() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    assert_eq!(fragment_counts(&engine), (2, 1));
    assert!(engine.can_undo());

    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        bond_variant: BondVariant::Single,
    });
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
    });
    assert_eq!(engine.state().selection.bonds.len(), 1);

    assert!(engine.delete_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));

    assert!(engine.undo());
    assert_eq!(fragment_counts(&engine), (2, 1));

    assert!(engine.redo());
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn bond_tool_focuses_bond_center_and_cycles_double_styles() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
    });
    engine.pointer_move(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: None,
    });

    let center = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert_eq!(center.point.x, 318.0);
    assert_eq!(center.point.y, 260.0);
    assert_eq!(center.order, 1);
    let center_rect = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon { points, .. } if points.len() == 4 => Some(points),
            _ => None,
        })
        .expect("single-bond center focus should render as a 4-point rectangle");
    let min_x = center_rect
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = center_rect
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = center_rect
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = center_rect
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((max_x - min_x - BOND_CENTER_FOCUS_LENGTH).abs() < 0.001);
    assert!((max_y - min_y - BOND_CENTER_FOCUS_WIDTH).abs() < 0.001);
    assert!((min_x - 309.0).abs() < 0.001);
    assert!((max_x - 327.0).abs() < 0.001);

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Right),
    );
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center),
    );
    assert!(engine.can_undo());

    engine.pointer_move(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: None,
    });
    let double_center = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert_eq!(double_center.order, 2);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon { points, .. } if points.len() == 4
    )));

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center),
    );

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left),
    );
}
