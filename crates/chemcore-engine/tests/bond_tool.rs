use chemcore_engine::{
    BondVariant, Engine, PointerEvent, Tool, ToolState, DEFAULT_BOND_LENGTH, DEFAULT_BOND_STROKE,
};

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
