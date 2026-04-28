use chemcore_engine::{
    direction_from_angle, BondLinePattern, BondLineWeight, BondVariant, DoubleBondPlacement,
    Engine, Point, PointerEvent, RenderPrimitive, RenderRole, Tool, ToolState, DEFAULT_BOND_LENGTH,
    DEFAULT_BOND_STROKE, ENDPOINT_FOCUS_RADIUS,
};
use serde_json::json;
use std::collections::BTreeMap;

const fn px(value: f64) -> f64 {
    chemcore_engine::px_to_cm(value)
}

fn px_point(x: f64, y: f64) -> chemcore_engine::Point {
    chemcore_engine::Point::new(px(x), px(y))
}

fn endpoint_from_anchor(anchor: Point, angle: f64) -> Point {
    anchor.translated(direction_from_angle(angle).scaled(DEFAULT_BOND_LENGTH))
}

const FIRST_START_X: f64 = 7.94;
const FIRST_START_Y: f64 = 6.88;
const FIRST_END_X: f64 = 8.85;
const FIRST_END_Y: f64 = 6.35;
const FIRST_END_HOVER_X: f64 = FIRST_END_X + px(1.0);
const FIRST_END_HOVER_Y: f64 = FIRST_END_Y + px(2.0);
const FIRST_CENTER_X: f64 = (FIRST_START_X + FIRST_END_X) * 0.5;
const FIRST_CENTER_Y: f64 = (FIRST_START_Y + FIRST_END_Y) * 0.5;
const FIRST_END_SINGLE_EXTEND_X: f64 = 9.77;
const FIRST_END_SINGLE_EXTEND_Y: f64 = 6.88;
const FIRST_END_TRIPLE_EXTEND_X: f64 = 9.76;
const FIRST_END_TRIPLE_EXTEND_Y: f64 = 5.82;
const ROOT_SINGLE_BRANCH_X: f64 = px(300.0);
const ROOT_SINGLE_BRANCH_Y: f64 = 7.94;
const ROOT_OPPOSITE_BRANCH_X: f64 = 8.85;
const ROOT_OPPOSITE_BRANCH_Y: f64 = 7.41;

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

fn delete_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Delete,
        bond_variant: BondVariant::Single,
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

fn select_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Select,
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

fn bond_order(engine: &Engine, bond_id: &str) -> Option<u8> {
    engine
        .state()
        .document
        .editable_fragment()
        .and_then(|entry| {
            entry
                .fragment
                .bonds
                .iter()
                .find(|bond| bond.id == bond_id)
                .map(|bond| bond.order)
        })
}

fn bond_center_point(engine: &Engine, bond_id: &str) -> chemcore_engine::Point {
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should exist");
    let begin = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    chemcore_engine::Point::new(
        (begin.position[0] + end.position[0]) * 0.5,
        (begin.position[1] + end.position[1]) * 0.5,
    )
}

fn selection_bond_rect(engine: &Engine) -> (f64, f64, f64, f64) {
    engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBond,
                x,
                y,
                width,
                height,
                ..
            } => Some((x, y, width, height)),
            _ => None,
        })
        .expect("selection bond rect should exist")
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

fn hover(engine: &mut Engine, x: f64, y: f64) {
    engine.pointer_move(PointerEvent {
        x,
        y,
        button: None,
        alt_key: false,
    });
}

fn rect_polygon(x1: f64, y1: f64, x2: f64, y2: f64) -> serde_json::Value {
    json!([
        [px(x1), px(y1)],
        [px(x2), px(y1)],
        [px(x2), px(y2)],
        [px(x1), px(y2)]
    ])
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
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM
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
                    "bbox": [0.0, 0.0, px(400.0), px(320.0)],
                    "nodes": [{
                        "id": "n1",
                        "element": "N",
                        "atomicNumber": 7,
                        "position": [px(300.0), px(260.0)],
                        "charge": 0,
                        "numHydrogens": 0,
                        "label": {
                            "text": label_text,
                            "sourceText": label_text,
                            "position": [px(297.0), px(260.0)],
                            "box": [px(294.0), px(256.0), px(324.0), px(264.0)],
                            "glyphPolygons": glyph_polygons
                        }
                    }, {
                        "id": "n0",
                        "element": "C",
                        "atomicNumber": 6,
                        "position": [px(264.0), px(260.0)],
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

fn load_text_object_document(engine: &mut Engine) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_text",
            "title": "text test",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {},
        "objects": [{
            "id": "obj_text_001",
            "type": "text",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": {
                "text": "Note",
                "bbox": [px(280.0), px(240.0), px(320.0), px(268.0)],
                "runs": []
            }
        }]
    });
    engine
        .load_document_json(&document.to_string())
        .expect("text document should load");
}

#[test]
fn click_on_blank_canvas_creates_up_right_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(
        entry.fragment.nodes[0].position,
        [FIRST_START_X, FIRST_START_Y]
    );
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn hover_focuses_existing_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.point.x, FIRST_END_X);
    assert_eq!(hover.point.y, FIRST_END_Y);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle { radius, .. } if (*radius - ENDPOINT_FOCUS_RADIUS).abs() < 0.001
    )));
}

#[test]
fn hovered_endpoint_can_be_replaced_with_element_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    assert!(engine.replace_hovered_endpoint_label("N"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(node.element, "N");
    assert_eq!(node.atomic_number, 7);
    assert!(!node.is_placeholder);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("N")
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.font_size),
        Some(chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM)
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.align.as_deref()),
        Some("left")
    );
}

#[test]
fn hovered_endpoint_can_be_replaced_with_abbreviation_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    assert!(engine.replace_hovered_endpoint_label("Ph"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(node.is_placeholder);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("Ph")
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.font_size),
        Some(chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM)
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.align.as_deref()),
        Some("left")
    );
}

#[test]
fn labeled_node_center_no_longer_focuses_plain_endpoint() {
    let mut engine = Engine::new();
    let glyph_polygons = vec![
        rect_polygon(304.0, 256.0, 310.0, 264.0),
        rect_polygon(312.0, 256.0, 318.0, 264.0),
        rect_polygon(320.0, 256.0, 326.0, 264.0),
        rect_polygon(328.0, 256.0, 334.0, 264.0),
    ];
    let bonds = json!([{
        "id": "b1",
        "begin": "n0",
        "end": "n1",
        "order": 1,
        "stereo": "none"
    }]);
    load_label_document(&mut engine, "CuF3", glyph_polygons, bonds);
    engine.set_tool_state(bond_tool());

    hover(&mut engine, px(300.0), px(260.0));
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn hovered_endpoint_can_be_cleared_back_to_carbon() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.replace_hovered_endpoint_label("Me"));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);

    assert!(engine.replace_hovered_endpoint_label("C"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(node.element, "C");
    assert_eq!(node.atomic_number, 6);
    assert!(!node.is_placeholder);
    assert!(node.label.is_none());
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
        x: px(305.0),
        y: px(260.0),
        button: None,
        alt_key: false,
    });

    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.node_id, "n1");
    assert!((hover.point.x - px(305.0)).abs() < 0.001, "{hover:?}");
    assert!((hover.point.y - px(260.0)).abs() < 0.001, "{hover:?}");
    let anchor = hover
        .label_anchor
        .as_ref()
        .expect("label anchor should exist");
    let expected_glyph_box = [px(302.0), px(256.0), px(308.0), px(264.0)];
    for (actual, expected) in anchor.glyph_box.iter().zip(expected_glyph_box) {
        assert!((*actual - expected).abs() < 0.001, "{anchor:?}");
    }

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::HoverLabelGlyph,
                x,
                y,
                width,
                height,
                ..
            } if (*x - px(302.0)).abs() < 0.001
                && (*y - px(256.0)).abs() < 0.001
                && (*width - px(6.0)).abs() < 0.001
                && (*height - px(8.0)).abs() < 0.001
        )
    }));
}

#[test]
fn click_on_label_glyph_uses_rightmost_group_anchor_for_default_bond() {
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

    click(&mut engine, px(305.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(px_point(313.0, 260.0), 0.0);
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
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
        x: px(305.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(305.0),
        y: px(220.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(305.0),
        y: px(220.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!(
        (last.position[0] - 8.07).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - FIRST_END_TRIPLE_EXTEND_Y).abs() < 0.01,
        "{:?}",
        last.position
    );
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
        x: px(305.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(360.0),
        y: px(260.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(360.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!(
        (last.position[0] - 9.34).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - FIRST_START_Y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn drag_from_middle_label_glyph_uses_leftmost_anchor_for_leftward_bond() {
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
        x: px(305.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(250.0),
        y: px(260.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(250.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(px_point(297.0, 260.0), 180.0);
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn drag_from_rightmost_label_glyph_keeps_clicked_glyph_for_rightward_bond() {
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
        x: px(321.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(360.0),
        y: px(260.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(360.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let expected = endpoint_from_anchor(px_point(321.0, 260.0), 0.0);
    assert!(
        (last.position[0] - expected.x).abs() < 0.01,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - expected.y).abs() < 0.01,
        "{:?}",
        last.position
    );
}

#[test]
fn click_on_single_bond_endpoint_extends_at_120_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!(
        (point[0] - FIRST_END_SINGLE_EXTEND_X).abs() < 0.01,
        "{point:?}"
    );
    assert!(
        (point[1] - FIRST_END_SINGLE_EXTEND_Y).abs() < 0.01,
        "{point:?}"
    );
}

#[test]
fn click_draw_keeps_hover_at_pointer_position_instead_of_new_endpoint() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("pointer-position hover should remain on clicked endpoint");
    assert_eq!(hover.point.x, FIRST_END_X);
    assert_eq!(hover.point.y, FIRST_END_Y);
}

#[test]
fn click_on_single_bond_endpoint_prefers_rightward_120_degree_branch_for_single_bond_component() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!((point[0] - ROOT_SINGLE_BRANCH_X).abs() < 0.01, "{point:?}");
    assert!((point[1] - ROOT_SINGLE_BRANCH_Y).abs() < 0.01, "{point:?}");
}

#[test]
fn click_on_endpoint_prefers_candidate_closer_to_same_component_bond_geometry() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 4);
    let point = entry.fragment.nodes[3].position;
    assert!((point[0] - ROOT_SINGLE_BRANCH_X).abs() < 0.01, "{point:?}");
    assert!((point[1] - ROOT_SINGLE_BRANCH_Y).abs() < 0.01, "{point:?}");
}

#[test]
fn click_on_endpoint_ignores_disconnected_bond_geometry_when_choosing_direction() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    click(&mut engine, ROOT_SINGLE_BRANCH_X, ROOT_SINGLE_BRANCH_Y);

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 5);
    let point = entry.fragment.nodes[4].position;
    assert!(
        (point[0] - ROOT_OPPOSITE_BRANCH_X).abs() < 0.01,
        "{point:?}"
    );
    assert!(
        (point[1] - ROOT_OPPOSITE_BRANCH_Y).abs() < 0.01,
        "{point:?}"
    );
}

#[test]
fn drag_from_endpoint_uses_fixed_length_and_angle_snap() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.preview.is_some());
    engine.pointer_up(PointerEvent {
        x: px(370.0),
        y: px(292.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let length = ((last.position[0] - FIRST_END_X).powi(2)
        + (last.position[1] - FIRST_END_Y).powi(2))
    .sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() < 0.01, "{length}");
    assert_eq!(fragment_counts(&engine), (3, 2));
}

#[test]
fn drag_from_endpoint_snaps_to_15_degree_increment() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(371.0),
        y: px(271.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(371.0),
        y: px(271.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    let dx = last.position[0] - FIRST_END_X;
    let dy = last.position[1] - FIRST_END_Y;
    let angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);

    assert!((angle - 45.0).abs() < 0.01, "{angle} {:?}", last.position);
    let length = (dx.powi(2) + dy.powi(2)).sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() < 0.01, "{length}");
}

#[test]
fn drag_preview_renders_document_geometry_instead_of_overlay_line() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(370.0),
        y: px(292.0),
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
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: true,
    });
    engine.pointer_move(PointerEvent {
        x: px(389.0),
        y: px(301.0),
        button: None,
        alt_key: true,
    });
    let preview = engine.state().overlay.preview.as_ref().unwrap();
    assert!((preview.end.x - px(389.0)).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - px(301.0)).abs() < 0.001, "{preview:?}");
    engine.pointer_up(PointerEvent {
        x: px(389.0),
        y: px(301.0),
        button: Some(0),
        alt_key: true,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let last = entry.fragment.nodes.last().unwrap();
    assert!(
        (last.position[0] - 10.29).abs() < 0.001,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - 7.96).abs() < 0.001,
        "{:?}",
        last.position
    );
    let length = ((last.position[0] - FIRST_END_X).powi(2)
        + (last.position[1] - FIRST_END_Y).powi(2))
    .sqrt();
    assert!((length - DEFAULT_BOND_LENGTH).abs() > px(5.0), "{length}");
}

#[test]
fn click_on_blank_canvas_creates_up_right_triple_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    assert_eq!(entry.fragment.bonds[0].order, 3);
}

#[test]
fn click_on_blank_canvas_creates_up_right_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn click_on_triple_bond_endpoint_extends_at_180_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 3);
    let point = entry.fragment.nodes[2].position;
    assert!(
        (point[0] - FIRST_END_TRIPLE_EXTEND_X).abs() < 0.01,
        "{point:?}"
    );
    assert!(
        (point[1] - FIRST_END_TRIPLE_EXTEND_Y).abs() < 0.01,
        "{point:?}"
    );
    assert_eq!(entry.fragment.bonds[1].order, 3);
}

#[test]
fn click_on_blank_canvas_creates_up_right_dashed_single_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(dashed_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(
        entry.fragment.bonds[0].line_styles.main,
        BondLinePattern::Dashed
    );
}

#[test]
fn click_on_blank_canvas_creates_up_right_dashed_double_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(dashed_double_bond_tool());

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Right)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
    assert_eq!(bond.line_styles.main, BondLinePattern::Solid);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);
}

#[test]
fn dashed_double_tool_cycles_side_center_and_opposite_side() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(dashed_double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
            bond.double.as_ref().map(|double| double.frozen),
            Some(false)
        );
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(
        entry.fragment.bonds[0].line_styles.main,
        BondLinePattern::Dashed
    );
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn dashed_tool_resets_non_double_styles_to_plain_dashed_single() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(active_side)
    );
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(active_side)
    );
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(opposite_side)
    );
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(second_dashed)
    );
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(
        entry.fragment.bonds[0].line_weights.main,
        BondLineWeight::Bold
    );
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn bold_tool_click_on_single_bond_makes_it_bold() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds[0].order, 1);
    assert_eq!(
        entry.fragment.bonds[0].line_weights.main,
        BondLineWeight::Bold
    );
    assert_eq!(entry.fragment.bonds[0].stroke_width, DEFAULT_BOND_STROKE);
}

#[test]
fn bold_tool_cycles_single_and_side_double_states() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.order, 1);
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(exit_side)
    );
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.line_weights.left, BondLineWeight::Normal);
    assert_eq!(bond.line_weights.right, BondLineWeight::Normal);
}

#[test]
fn click_on_blank_canvas_creates_horizontal_bold_dashed_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bold_dashed_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(bold_dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.bonds[0].order, 3);
    }

    engine.set_tool_state(bold_dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
fn click_on_blank_canvas_creates_up_right_wedge_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(wedge_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
    let bond = &entry.fragment.bonds[0];
    let stereo = bond.stereo.as_ref().unwrap();
    assert_eq!(bond.order, 1);
    assert_eq!(stereo.kind, "solid-wedge");
    assert_eq!(stereo.wide_end, "end");
}

#[test]
fn click_on_blank_canvas_creates_up_right_hashed_wedge_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(hashed_wedge_bond_tool());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes[1].position, [FIRST_END_X, FIRST_END_Y]);
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(hashed_wedge_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(tool.clone());
        click(&mut engine, FIRST_END_X, FIRST_END_Y);

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
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(tool.clone());
        engine.pointer_move(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
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
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(bond_tool());
        engine.pointer_down(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
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
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
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
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(double_bond_tool());
        engine.pointer_down(PointerEvent {
            x: FIRST_CENTER_X,
            y: FIRST_CENTER_Y,
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
        assert_eq!(
            bond.double.as_ref().map(|double| double.frozen),
            Some(false),
            "{source:?}"
        );
        assert!(bond.stereo.is_none(), "{source:?}");
        assert_eq!(bond.line_styles.main, BondLinePattern::Solid, "{source:?}");
        assert_eq!(bond.line_weights.main, BondLineWeight::Normal, "{source:?}");
    }

    let mut engine = Engine::new();
    engine.set_tool_state(bold_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(triple_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
    click(&mut engine, px(300.0), px(260.0));

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_down(PointerEvent {
        x: FIRST_END_SINGLE_EXTEND_X,
        y: FIRST_END_SINGLE_EXTEND_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: px(304.0),
        y: px(263.0),
        button: None,
        alt_key: false,
    });
    let hover = engine.state().overlay.hover_endpoint.as_ref().unwrap();
    assert_eq!(hover.point.x, FIRST_START_X);
    assert_eq!(hover.point.y, FIRST_START_Y);
    let preview = engine.state().overlay.preview.as_ref().unwrap();
    assert_eq!(preview.end.x, FIRST_START_X);
    assert_eq!(preview.end.y, FIRST_START_Y);
    engine.pointer_up(PointerEvent {
        x: px(304.0),
        y: px(263.0),
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
        x: FIRST_END_X,
        y: px(260.0),
        button: None,
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let closed_bond = entry.fragment.bonds.last().unwrap();
    assert_eq!(closed_bond.order, 2);
    assert_eq!(
        closed_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center),
    );
}

#[test]
fn click_extension_reuses_endpoint_at_default_angle() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });

    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(200.0, 200.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_SINGLE_EXTEND_X, FIRST_END_SINGLE_EXTEND_Y),
    );

    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
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
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(fragment_counts(&engine), (2, 1));
    assert!(engine.can_undo());

    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    assert_eq!(engine.state().selection.bonds.len(), 1);

    assert!(engine.delete_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));

    assert!(engine.undo());
    assert_eq!(fragment_counts(&engine), (2, 1));

    assert!(engine.redo());
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn select_tool_click_on_text_object_selects_text_box() {
    let mut engine = Engine::new();
    load_text_object_document(&mut engine);
    engine.set_tool_state(select_tool());

    engine.select_at_point(px_point(300.0, 250.0), false);

    assert_eq!(engine.state().selection.text_objects, vec!["obj_text_001"]);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_click_on_label_selects_label_box_not_atom() {
    let mut engine = Engine::new();
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
    engine.set_tool_state(select_tool());

    engine.select_at_point(px_point(305.0, 260.0), false);

    assert_eq!(engine.state().selection.label_nodes, vec!["n1"]);
    assert!(engine.state().selection.nodes.is_empty());
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_click_on_endpoint_selects_atom_box() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);

    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionNode,
            ..
        }
    )));
}

#[test]
fn select_tool_click_on_bond_does_not_render_outer_region_box() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBond,
            ..
        }
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox,
            ..
        }
    )));
}

#[test]
fn select_tool_click_on_side_double_bond_wraps_both_lines() {
    let mut single = Engine::new();
    single.set_tool_state(bond_tool());
    click(&mut single, px(300.0), px(260.0));
    single.set_tool_state(select_tool());
    single.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, single_width, single_height) = selection_bond_rect(&single);

    let mut double = Engine::new();
    double.set_tool_state(double_bond_tool());
    click(&mut double, px(300.0), px(260.0));
    double.set_tool_state(select_tool());
    double.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, double_width, double_height) = selection_bond_rect(&double);

    assert!(
        double_width > single_width + 0.04 || double_height > single_height + 0.04,
        "expected double bond rect to exceed single bond rect, single=({single_width}, {single_height}) double=({double_width}, {double_height})"
    );
}

#[test]
fn select_tool_click_on_triple_bond_wraps_all_three_lines() {
    let mut single = Engine::new();
    single.set_tool_state(bond_tool());
    click(&mut single, px(300.0), px(260.0));
    single.set_tool_state(select_tool());
    single.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, single_width, single_height) = selection_bond_rect(&single);

    let mut triple = Engine::new();
    triple.set_tool_state(triple_bond_tool());
    click(&mut triple, px(300.0), px(260.0));
    triple.set_tool_state(select_tool());
    triple.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let (_, _, triple_width, triple_height) = selection_bond_rect(&triple);

    assert!(
        triple_width > single_width + 0.08 || triple_height > single_height + 0.08,
        "expected triple bond rect to exceed single bond rect, single=({single_width}, {single_height}) triple=({triple_width}, {triple_height})"
    );
}

#[test]
fn select_tool_box_selecting_whole_fragment_renders_component_box() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_in_rect(px_point(290.0, 234.0), px_point(346.0, 286.0), false);

    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
    assert!(engine.render_list().iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBox,
                ..
            }
        )
    }));
}

#[test]
fn select_tool_shift_click_adds_to_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);
    engine.select_at_point(Point::new(FIRST_START_X, FIRST_START_Y), true);

    assert_eq!(engine.state().selection.nodes.len(), 2);
}

#[test]
fn delete_tool_click_degrades_double_before_removing_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds
        .first()
        .expect("bond should exist")
        .id
        .clone();
    let center = bond_center_point(&engine, &bond_id);

    engine.set_tool_state(delete_tool());
    click(&mut engine, center.x, center.y);
    assert_eq!(bond_order(&engine, &bond_id), Some(1));

    click(&mut engine, center.x, center.y);
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn delete_tool_click_degrades_triple_to_side_double() {
    let mut engine = Engine::new();
    engine.set_tool_state(triple_bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry.fragment.bonds.first().expect("bond should exist");
    let bond_id = bond.id.clone();
    let center = bond_center_point(&engine, &bond_id);
    drop(entry);

    engine.set_tool_state(delete_tool());
    click(&mut engine, center.x, center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should remain");
    assert_eq!(bond.order, 2);
    assert!(bond.double.is_some());
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn delete_tool_click_on_endpoint_removes_all_connected_bonds() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert_eq!(fragment_counts(&engine), (3, 2));

    let entry = engine.state().document.editable_fragment().unwrap();
    let branch_node_id = node_degrees(&engine)
        .into_iter()
        .find_map(|(node_id, degree)| (degree == 2).then_some(node_id))
        .expect("branch node should exist");
    let branch_node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == branch_node_id)
        .expect("branch node should exist");
    let branch_point = (branch_node.position[0], branch_node.position[1]);
    drop(entry);

    engine.set_tool_state(delete_tool());
    click(&mut engine, branch_point.0, branch_point.1);
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn delete_tool_click_on_label_removes_only_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, px(300.0), px(260.0));
    assert!(engine.replace_hovered_endpoint_label("Ph"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry.fragment.nodes.first().expect("node should exist");
    let label_box = node
        .label
        .as_ref()
        .and_then(|label| label.bbox())
        .expect("label box");
    drop(entry);

    engine.set_tool_state(delete_tool());
    click(
        &mut engine,
        (label_box[0] + label_box[2]) * 0.5,
        (label_box[1] + label_box[3]) * 0.5,
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry.fragment.nodes.first().expect("node should remain");
    assert!(node.label.is_none());
    assert_eq!(entry.fragment.bonds.len(), 1);
}

#[test]
fn delete_command_on_hovered_bond_center_removes_entire_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds
        .first()
        .expect("bond should exist")
        .id
        .clone();
    let center = bond_center_point(&engine, &bond_id);

    engine.set_tool_state(bond_tool());
    hover(&mut engine, center.x, center.y);
    assert!(engine.delete_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn bond_tool_focuses_bond_center_and_cycles_double_styles() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(260.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: FIRST_END_X,
        y: FIRST_END_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: None,
        alt_key: false,
    });

    let center = engine.state().overlay.hover_bond_center.as_ref().unwrap();
    assert!((center.point.x - FIRST_CENTER_X).abs() < 0.001);
    assert!((center.point.y - FIRST_CENTER_Y).abs() < 0.001);
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
    assert_eq!(center_rect.len(), 4);

    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
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

#[test]
fn double_tool_defaults_to_center_on_three_connected_node() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(331.18, 278.0),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.begin == "n_1" && bond.end == "n_2")
        .unwrap();
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn double_tool_defaults_to_side_when_substituents_span_both_sides() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.begin == "n_1" && bond.end == "n_2")
        .unwrap();
    assert_eq!(bond.order, 2);
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_ne!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

#[test]
fn collinear_attachment_does_not_trigger_centered_double_default() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(
                FIRST_END_TRIPLE_EXTEND_X,
                FIRST_END_TRIPLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert!(matches!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
        ));
        assert_ne!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Center)
        );
        assert_eq!(
            bond.double.as_ref().map(|double| double.frozen),
            Some(false)
        );
    }
}

#[test]
fn adding_fourth_bond_to_unfrozen_center_double_moves_to_last_drawn_side_on_tie() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(ROOT_SINGLE_BRANCH_X, ROOT_SINGLE_BRANCH_Y),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(331.18, 278.0),
            label_anchor: None,
        },
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Right)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn adding_cis_substituent_to_unfrozen_monosubstituted_double_moves_to_inner_side() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        assert_eq!(
            bond.double.as_ref().map(|double| double.placement),
            Some(DoubleBondPlacement::Left)
        );
        assert_eq!(
            bond.double.as_ref().map(|double| double.frozen),
            Some(false)
        );
    }

    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Right)
    );
    assert_eq!(
        bond.double.as_ref().map(|double| double.frozen),
        Some(false)
    );
}

#[test]
fn frozen_double_bond_keeps_manual_style_after_new_attachment() {
    let mut engine = Engine::new();
    engine.add_single_bond(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: px_point(268.82, 242.0),
            label_anchor: None,
        },
    );
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_2".to_string()),
            point: chemcore_engine::Point::new(FIRST_END_X, FIRST_END_Y),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(
                FIRST_END_SINGLE_EXTEND_X,
                FIRST_END_SINGLE_EXTEND_Y,
            ),
            label_anchor: None,
        },
    );

    engine.set_tool_state(double_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });

    let manual_placement = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        let placement = bond.double.as_ref().map(|double| double.placement);
        assert!(matches!(
            placement,
            Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
        ));
        assert_eq!(bond.double.as_ref().map(|double| double.frozen), Some(true));
        placement
    };

    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: Some("n_1".to_string()),
            point: px_point(300.0, 260.0),
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: chemcore_engine::Point::new(ROOT_SINGLE_BRANCH_X, ROOT_SINGLE_BRANCH_Y),
            label_anchor: None,
        },
    );

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        manual_placement
    );
    assert_eq!(bond.double.as_ref().map(|double| double.frozen), Some(true));
}
