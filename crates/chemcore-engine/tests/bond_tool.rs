use chemcore_engine::{
    BondLinePattern, BondLineWeight, BondVariant, DoubleBondPlacement, Engine, PointerEvent,
    RenderPrimitive, RenderRole, Tool, ToolState, BOND_CENTER_FOCUS_LENGTH, BOND_CENTER_FOCUS_WIDTH,
    DEFAULT_BOND_LENGTH, DEFAULT_BOND_STROKE, ENDPOINT_FOCUS_RADIUS,
};
use serde_json::json;
use std::collections::BTreeMap;

fn bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Single,
    }
}

fn triple_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Triple,
    }
}

fn double_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Double,
    }
}

fn dashed_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Dashed,
    }
}

fn dashed_double_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::DashedDouble,
    }
}

fn bold_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Bold,
    }
}

fn bold_dashed_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::BoldDashed,
    }
}

fn wedge_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Wedge,
    }
}

fn hashed_wedge_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::HashedWedge,
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

fn rect_polygon(x1: f64, y1: f64, x2: f64, y2: f64) -> serde_json::Value {
    json!([[x1, y1], [x2, y1], [x2, y2], [x1, y2]])
}

fn load_label_document(
    engine: &mut Engine,
    label_text: &str,
    glyph_polygons: Vec<serde_json::Value>,
    bonds: serde_json::Value,
) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "label test",
            "page": { "width": 400.0, "height": 320.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": 11.0
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
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 400.0, 320.0],
                    "nodes": [{
                        "id": "n1",
                        "element": "N",
                        "atomicNumber": 7,
                        "position": [300.0, 260.0],
                        "charge": 0,
                        "numHydrogens": 0,
                        "label": {
                            "text": label_text,
                            "sourceText": label_text,
                            "position": [297.0, 260.0],
                            "box": [294.0, 256.0, 324.0, 264.0],
                            "glyphPolygons": glyph_polygons
                        }
                    }, {
                        "id": "n0",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [264.0, 260.0],
                        "charge": 0,
                        "numHydrogens": 0
                    }],
                    "bonds": bonds
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");
}

#[test]
fn click_on_blank_canvas_creates_horizontal_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
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
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_move(PointerEvent {
        x: 337.0,
        y: 263.0,
        button: None,
        alt_key: false,
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
fn hover_focuses_label_glyph_anchor() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    engine.pointer_move(PointerEvent {
        x: 305.0,
        y: 260.0,
        button: None,
        alt_key: false,
    });

    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.node_id, "n1");
    assert!((hover.point.x - 305.0).abs() < 0.001, "{hover:?}");
    assert!((hover.point.y - 260.0).abs() < 0.001, "{hover:?}");
    assert!(hover.label_anchor.is_some(), "{hover:?}");
}

#[test]
fn click_on_label_glyph_uses_rightmost_label_anchor_for_horizontal_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    click(&mut engine, 305.0, 260.0);

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!((last.position[0] - 357.0).abs() < 0.01, "{:?}", last.position);
    assert!((last.position[1] - 260.0).abs() < 0.01, "{:?}", last.position);
}

#[test]
fn drag_from_label_glyph_uses_focused_glyph_for_vertical_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([]),
    );

    engine.pointer_down(PointerEvent {
        x: 305.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 305.0,
        y: 220.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 305.0,
        y: 220.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!((last.position[0] - 305.0).abs() < 0.01, "{:?}", last.position);
    assert!((last.position[1] - 224.0).abs() < 0.01, "{:?}", last.position);
}

#[test]
fn drag_from_connected_label_uses_rightmost_group_uppercase_anchor() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    load_label_document(
        &mut engine,
        "CuF3",
        vec![
            rect_polygon(294.0, 256.0, 300.0, 264.0),
            rect_polygon(302.0, 256.0, 308.0, 264.0),
            rect_polygon(310.0, 256.0, 316.0, 264.0),
            rect_polygon(318.0, 256.0, 324.0, 264.0),
        ],
        json!([{
            "id": "b0",
            "begin": "n0",
            "end": "n1",
            "order": 1,
            "strokeWidth": DEFAULT_BOND_STROKE
        }]),
    );

    engine.pointer_down(PointerEvent {
        x: 305.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 360.0,
        y: 260.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 360.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!((last.position[0] - 349.0).abs() < 0.01, "{:?}", last.position);
    assert!((last.position[1] - 260.0).abs() < 0.01, "{:?}", last.position);
}

#[test]
fn click_on_single_bond_endpoint_extends_at_120_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
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
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 370.0,
        y: 292.0,
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.preview.is_some());
    engine.pointer_up(PointerEvent {
        x: 370.0,
        y: 292.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let length = ((last.position[0] - 336.0).powi(2) + (last.position[1] - 260.0).powi(2)).sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() < 0.01, "{length}");
    assert_eq!(fragment_counts(&engine), (3, 2));
}

#[test]
fn drag_preview_renders_document_geometry_instead_of_overlay_line() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 370.0,
        y: 292.0,
        button: None,
        alt_key: false,
    });

    let render_list = engine.render_list();
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Line { role, .. } if *role == RenderRole::PreviewBond
    )));
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle { role, .. } if *role == RenderRole::PreviewEnd
    )));
    assert!(
        render_list
            .iter()
            .filter(|primitive| matches!(
                primitive,
                RenderPrimitive::Polygon { role, .. } if *role == RenderRole::DocumentBond
            ))
            .count()
            >= 2,
        "{render_list:?}"
    );
}

#[test]
fn alt_drag_from_endpoint_uses_mouse_distance_without_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: true,
    });
    engine.pointer_move(PointerEvent {
        x: 389.0,
        y: 301.0,
        button: None,
        alt_key: true,
    });
    let preview = engine.state().overlay.preview.as_ref().unwrap();
    assert!((preview.end.x - 389.0).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - 301.0).abs() < 0.001, "{preview:?}");
    engine.pointer_up(PointerEvent {
        x: 389.0,
        y: 301.0,
        button: Some(0),
        alt_key: true,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!((last.position[0] - 389.0).abs() < 0.001, "{:?}", last.position);
    assert!((last.position[1] - 301.0).abs() < 0.001, "{:?}", last.position);
    let length = ((last.position[0] - 336.0).powi(2) + (last.position[1] - 260.0).powi(2)).sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() > 5.0, "{length}");
}

#[test]
fn click_on_blank_canvas_creates_horizontal_triple_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 3);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 2);
}

#[test]
fn click_on_triple_bond_endpoint_extends_at_180_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!((point[0] - 372.0).abs() < 0.01, "{point:?}");
    assert!((point[1] - 260.0).abs() < 0.01, "{point:?}");
    assert_eq!(entry.fragment.bonds[1].order, 3);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_dashed_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(dashed_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(entry.fragment.bonds[0].line_styles.main, BondLinePattern::Dashed);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_dashed_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(dashed_double_bond_tool());

    click(&mut engine, 300.0, 260.0);

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Right)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);
}

#[test]
fn dashed_double_tool_cycles_side_center_and_opposite_side() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.set_tool_state(dashed_double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let active_side = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        let side = bond
            .double
            .as_ref()
            .map(|double| double.placement)
            .expect("side dashed-double placement");
        assert!(matches!(
            side,
            DoubleBondPlacement::Left | DoubleBondPlacement::Right
        ));
        assert_eq!(bond.order, 2);
        assert_eq!(
            match side {
                DoubleBondPlacement::Left => bond.line_styles.left,
                DoubleBondPlacement::Right => bond.line_styles.right,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Dashed
        );
        assert_eq!(
            match side {
                DoubleBondPlacement::Left => bond.line_styles.right,
                DoubleBondPlacement::Right => bond.line_styles.left,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Solid
        );
        side
    };

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Center)
        );
        assert_eq!(
            match active_side {
                DoubleBondPlacement::Left => bond.line_styles.left,
                DoubleBondPlacement::Right => bond.line_styles.right,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Dashed
        );
        assert_eq!(
            match active_side {
                DoubleBondPlacement::Left => bond.line_styles.right,
                DoubleBondPlacement::Right => bond.line_styles.left,
                DoubleBondPlacement::Center => unreachable!(),
            },
            BondLinePattern::Solid
        );
    }

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let opposite_side = match active_side {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!(),
    };
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(opposite_side)
    );
    assert_eq!(
        match opposite_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match opposite_side {
            DoubleBondPlacement::Left => bond.line_styles.right,
            DoubleBondPlacement::Right => bond.line_styles.left,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );
}

#[test]
fn dashed_tool_click_on_single_bond_makes_it_dashed() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds[0].line_styles.main, BondLinePattern::Dashed);
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn dashed_tool_resets_non_double_styles_to_plain_dashed_single() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
    assert!(bond.stereo.is_none());
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.left, BondLinePattern::Solid);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
}

#[test]
fn dashed_tool_cycles_side_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let active_side = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .expect("side double placement");
    let opposite_side = match active_side {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!("side double should not be centered"),
    };
    assert_eq!(bond.double.as_ref().map(|double| double.placement), Some(active_side));
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.double.as_ref().map(|double| double.placement), Some(active_side));
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let double = bond.double.as_ref().unwrap();
    assert_eq!(double.placement, DoubleBondPlacement::Center);
    assert_eq!(double.center_exit_side, Some(opposite_side));
    assert_eq!(bond.line_styles.left, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.double.as_ref().map(|double| double.placement), Some(opposite_side));
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(
        match opposite_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );
}

#[test]
fn dashed_tool_cycles_center_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.double.as_ref().map(|double| double.placement), Some(DoubleBondPlacement::Center));
    let first_dashed = if bond.line_styles.left == BondLinePattern::Dashed {
        DoubleBondPlacement::Left
    } else {
        DoubleBondPlacement::Right
    };
    let second_dashed = match first_dashed {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!(),
    };
    assert_eq!(
        match first_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match second_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let double = bond.double.as_ref().unwrap();
    assert_eq!(double.placement, DoubleBondPlacement::Center);
    assert_eq!(double.center_exit_side, Some(second_dashed));
    assert_eq!(bond.line_styles.left, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.double.as_ref().map(|double| double.placement), Some(second_dashed));
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(
        match second_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Dashed
    );
    assert_eq!(
        match first_dashed {
            DoubleBondPlacement::Left => bond.line_styles.left,
            DoubleBondPlacement::Right => bond.line_styles.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLinePattern::Solid
    );
}

#[test]
fn click_on_blank_canvas_creates_horizontal_bold_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(entry.fragment.bonds[0].line_weights.main, BondLineWeight::Bold);
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn bold_tool_click_on_single_bond_makes_it_bold() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(entry.fragment.bonds[0].line_weights.main, BondLineWeight::Bold);
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn bold_tool_cycles_single_and_side_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let active_side = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .expect("side double placement");
    assert_eq!(bond.order, 2);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
    assert_eq!(bond.line_weights.right, BondLineWeight::Normal);

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
    assert_eq!(
        bond.double
            .as_ref()
            .and_then(|double| double.center_exit_side),
        Some(match active_side {
            DoubleBondPlacement::Left => DoubleBondPlacement::Right,
            DoubleBondPlacement::Right => DoubleBondPlacement::Left,
            DoubleBondPlacement::Center => unreachable!(),
        })
    );
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
    assert_eq!(
        match active_side {
            DoubleBondPlacement::Left => bond.line_weights.left,
            DoubleBondPlacement::Right => bond.line_weights.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLineWeight::Bold
    );
}

#[test]
fn bold_tool_cycles_plain_center_double_into_bold_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let first_bold_side = if bond.line_weights.left == BondLineWeight::Bold {
        DoubleBondPlacement::Left
    } else {
        DoubleBondPlacement::Right
    };
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
    assert_eq!(
        match first_bold_side {
            DoubleBondPlacement::Left => bond.line_weights.left,
            DoubleBondPlacement::Right => bond.line_weights.right,
            DoubleBondPlacement::Center => unreachable!(),
        },
        BondLineWeight::Bold
    );

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let exit_side = match first_bold_side {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => unreachable!(),
    };
    assert_eq!(bond.double.as_ref().map(|double| double.placement), Some(exit_side));
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
    assert_eq!(bond.line_weights.right, BondLineWeight::Normal);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_bold_dashed_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_dashed_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
}

#[test]
fn bold_dashed_tool_click_on_endpoint_creates_bold_dashed_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[1];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
}

#[test]
fn bold_dashed_tool_replaces_existing_bond_regardless_of_order() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.bonds[0].order, 3);
    }

    engine.set_tool_state(bold_dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
    assert!(bond.stereo.is_none());
    assert_eq!(bond.line_styles.main, BondLinePattern::Dashed);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_styles.left, BondLinePattern::Solid);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_wedge_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(wedge_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "end");
}

#[test]
fn click_on_blank_canvas_creates_horizontal_hashed_wedge_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(hashed_wedge_bond_tool());

    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "hashed-wedge");
    assert_eq!(stereo.wide_end, "end");
}

#[test]
fn wedge_tool_replaces_bond_and_toggles_direction() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert!(bond.double.is_none());
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "end");

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "begin");
}

#[test]
fn hashed_wedge_tool_replaces_bond_and_toggles_direction() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(hashed_wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "hashed-wedge");
    assert_eq!(stereo.wide_end, "end");

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
    assert_eq!(stereo.kind, "hashed-wedge");
    assert_eq!(stereo.wide_end, "begin");
}

#[test]
fn all_bond_tools_can_extend_from_existing_endpoint() {
    let tools = [
        bond_tool(),
        double_bond_tool(),
        triple_bond_tool(),
        dashed_bond_tool(),
        dashed_double_bond_tool(),
        bold_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for tool in tools {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        click(&mut engine, 300.0, 260.0);

        engine.set_tool_state(tool.clone());
        click(&mut engine, 336.0, 260.0);

        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.bonds.len(), 2, "{tool:?}");
    }
}

#[test]
fn all_bond_tools_can_focus_existing_triple_bond_center() {
    let tools = [
        bond_tool(),
        double_bond_tool(),
        triple_bond_tool(),
        dashed_bond_tool(),
        dashed_double_bond_tool(),
        bold_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for tool in tools {
        let mut engine = Engine::new();
        engine.set_tool_state(triple_bond_tool());
        click(&mut engine, 300.0, 260.0);

        engine.set_tool_state(tool.clone());
        engine.pointer_move(PointerEvent {
            x: 318.0,
            y: 260.0,
            button: None,
        alt_key: false,
        });

        let hover = engine
            .state()
            .overlay
            .hover_bond_center
            .as_ref()
            .unwrap_or_else(|| panic!("{tool:?} should focus triple-bond center"));
        assert_eq!(hover.order, 3, "{tool:?}");
    }
}

#[test]
fn single_tool_resets_styled_bonds_before_entering_double_cycle() {
    let source_tools = [
        triple_bond_tool(),
        dashed_bond_tool(),
        bold_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for source in source_tools {
        let mut engine = Engine::new();
        engine.set_tool_state(source.clone());
        click(&mut engine, 300.0, 260.0);

        engine.set_tool_state(bond_tool());
        engine.pointer_down(PointerEvent {
            x: 318.0,
            y: 260.0,
            button: Some(0),
        alt_key: false,
        });

        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(bond.order, 1, "{source:?}");
        assert!(bond.double.is_none(), "{source:?}");
        assert!(bond.stereo.is_none(), "{source:?}");
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");

        engine.pointer_down(PointerEvent {
            x: 318.0,
            y: 260.0,
            button: Some(0),
        alt_key: false,
        });

        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(bond.order, 2, "{source:?}");
        assert!(matches!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
        ));
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");
    }
}

#[test]
fn double_tool_converts_other_styles_into_expected_double_states() {
    let plain_sources = [
        triple_bond_tool(),
        dashed_bond_tool(),
        bold_dashed_bond_tool(),
        wedge_bond_tool(),
        hashed_wedge_bond_tool(),
    ];

    for source in plain_sources {
        let mut engine = Engine::new();
        engine.set_tool_state(source.clone());
        click(&mut engine, 300.0, 260.0);

        engine.set_tool_state(double_bond_tool());
        engine.pointer_down(PointerEvent {
            x: 318.0,
            y: 260.0,
            button: Some(0),
        alt_key: false,
        });

        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(bond.order, 2, "{source:?}");
        assert!(matches!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
        ));
        assert!(bond.stereo.is_none(), "{source:?}");
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");
    }

    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
}

#[test]
fn triple_tool_replaces_existing_style_with_plain_triple() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_dashed_bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 3);
    assert!(bond.double.is_none());
    assert!(bond.stereo.is_none());
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(bond.line_weights.main, BondLineWeight::Normal);
}

#[test]
fn wedge_tools_preserve_orientation_when_switching_kinds() {
    let mut engine = Engine::new();
    engine.set_tool_state(wedge_bond_tool());
    click(&mut engine, 300.0, 260.0);

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
        assert_eq!(stereo.kind, "solid-wedge");
        assert_eq!(stereo.wide_end, "begin");
    }

    engine.set_tool_state(hashed_wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
        assert_eq!(stereo.kind, "hashed-wedge");
        assert_eq!(stereo.wide_end, "begin");
    }

    engine.set_tool_state(wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let stereo = entry.fragment.bonds[0].stereo.as_ref().unwrap();
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "begin");
}

#[test]
fn dragged_bond_endpoint_reuses_focused_existing_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 318.0,
        y: 228.82,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 318.0,
        y: 228.82,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: 318.0,
        y: 228.82,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 304.0,
        y: 263.0,
        button: None,
        alt_key: false,
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
        alt_key: false,
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
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 309.0,
        y: 244.41,
        button: Some(0),
        alt_key: false,
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
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });

    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(200.0, 200.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(354.0, 228.82),
    );

    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
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
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
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
        alt_key: false,
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
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 300.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 336.0,
        y: 260.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 318.0,
        y: 260.0,
        button: None,
        alt_key: false,
    });

    let center = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert_eq!(center.point.x, 318.0);
    assert_eq!(center.point.y, 260.0);
    assert_eq!(center.order, 1);
    let center_rect = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon { role, points, .. }
                if role == RenderRole::HoverBondCenter && points.len() == 4 =>
            {
                Some(points)
            }
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
        alt_key: false,
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
        alt_key: false,
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
        alt_key: false,
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
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left),
    );
}
