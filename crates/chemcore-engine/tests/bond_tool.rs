use chemcore_engine::{
    angle_between, direction_from_angle, line_object_points, parse_document_json, ArrowCurve,
    ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, BondLinePattern, BondLineWeight,
    BondVariant, BracketKind, DoubleBondPlacement, Engine, Point, PointerEvent, RenderPrimitive,
    RenderRole, ShapeKind, ShapeStyle, TextEditSession, TextEditTarget, Tool, ToolState,
    DEFAULT_BOND_LENGTH, DEFAULT_BOND_STROKE, ENDPOINT_FOCUS_RADIUS,
};
use serde_json::json;
use std::collections::BTreeMap;

mod support;
use support::read_optional_cdxml_fixture;

const fn px(value: f64) -> f64 {
    chemcore_engine::px_to_pt(value)
}

fn px_point(x: f64, y: f64) -> chemcore_engine::Point {
    chemcore_engine::Point::new(px(x), px(y))
}

fn round_to_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn polygon_area(points: &[Point]) -> f64 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    (area * 0.5).abs()
}

fn rect_path_coordinate_bounds(d: &str) -> [f64; 4] {
    let normalized = d.replace(',', " ");
    let nums: Vec<f64> = normalized
        .split_whitespace()
        .filter_map(|part| part.parse().ok())
        .collect();
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for pair in nums.chunks_exact(2) {
        min_x = min_x.min(pair[0]);
        min_y = min_y.min(pair[1]);
        max_x = max_x.max(pair[0]);
        max_y = max_y.max(pair[1]);
    }
    [min_x, min_y, max_x, max_y]
}

fn label_glyph_bounds(label: &chemcore_engine::NodeLabel) -> [f64; 4] {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for polygon in &label.glyph_polygons {
        for [x, y] in polygon {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
    }
    [min_x, min_y, max_x, max_y]
}

fn assert_point_close(left: Point, right: Point) {
    assert!(
        left.distance(right) < 1e-9,
        "expected {left:?} to be close to {right:?}"
    );
}

fn endpoint_from_anchor(anchor: Point, angle: f64) -> Point {
    anchor.translated(direction_from_angle(angle).scaled(DEFAULT_BOND_LENGTH))
}

fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

const FIRST_START_X: f64 = px(300.0);
const FIRST_START_Y: f64 = px(260.0);
const FIRST_END_X: f64 = 250.98;
const FIRST_END_Y: f64 = 180.0;
const FIRST_END_HOVER_X: f64 = FIRST_END_X + px(1.0);
const FIRST_END_HOVER_Y: f64 = FIRST_END_Y + px(2.0);
const FIRST_CENTER_X: f64 = (FIRST_START_X + FIRST_END_X) * 0.5;
const FIRST_CENTER_Y: f64 = (FIRST_START_Y + FIRST_END_Y) * 0.5;
const FIRST_END_SINGLE_EXTEND_X: f64 = 276.96;
const FIRST_END_SINGLE_EXTEND_Y: f64 = 195.0;
const FIRST_END_TRIPLE_EXTEND_X: f64 = 276.96;
const FIRST_END_TRIPLE_EXTEND_Y: f64 = 165.0;
const ROOT_SINGLE_BRANCH_X: f64 = px(300.0);
const ROOT_SINGLE_BRANCH_Y: f64 = 225.0;
const ROOT_OPPOSITE_BRANCH_X: f64 = 250.98;
const ROOT_OPPOSITE_BRANCH_Y: f64 = 210.0;

fn bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    }
}

fn triple_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Triple,
        ..ToolState::default()
    }
}

fn double_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Double,
        ..ToolState::default()
    }
}

fn delete_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Delete,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    }
}

fn dashed_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Dashed,
        ..ToolState::default()
    }
}

fn dashed_double_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::DashedDouble,
        ..ToolState::default()
    }
}

fn bold_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Bold,
        ..ToolState::default()
    }
}

fn bold_dashed_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::BoldDashed,
        ..ToolState::default()
    }
}

fn wedge_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::Wedge,
        ..ToolState::default()
    }
}

fn hashed_wedge_bond_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Bond,
        bond_variant: BondVariant::HashedWedge,
        ..ToolState::default()
    }
}

fn select_tool() -> ToolState {
    ToolState {
        active_tool: Tool::Select,
        bond_variant: BondVariant::Single,
        ..ToolState::default()
    }
}

fn templates_tool(template: &str) -> ToolState {
    ToolState {
        active_tool: Tool::Templates,
        bond_variant: BondVariant::Single,
        template: template.to_string(),
        ..ToolState::default()
    }
}

fn shape_tool(shape_kind: ShapeKind, shape_style: ShapeStyle) -> ToolState {
    ToolState {
        active_tool: Tool::Shape,
        shape_kind,
        shape_style,
        shape_color: "#000000".to_string(),
        ..ToolState::default()
    }
}

fn fragment_counts(engine: &Engine) -> (usize, usize) {
    let entry = engine.state().document.editable_fragment().unwrap();
    (entry.fragment.nodes.len(), entry.fragment.bonds.len())
}

fn node_world_point(engine: &Engine, node_id: &str) -> Point {
    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .expect("node should exist");
    entry.world_point_for_node(node)
}

fn node_id_at(engine: &Engine, point: Point) -> Option<String> {
    let entry = engine.state().document.editable_fragment().unwrap();
    entry
        .fragment
        .nodes
        .iter()
        .find(|node| entry.world_point_for_node(node).distance(point) < 0.02)
        .map(|node| node.id.clone())
}

fn attached_node_points(engine: &Engine, node_id: &str) -> Vec<Point> {
    let entry = engine.state().document.editable_fragment().unwrap();
    entry
        .fragment
        .bonds
        .iter()
        .filter_map(|bond| {
            if bond.begin == node_id {
                Some(bond.end.as_str())
            } else if bond.end == node_id {
                Some(bond.begin.as_str())
            } else {
                None
            }
        })
        .filter_map(|other_id| {
            entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == other_id)
                .map(|node| entry.world_point_for_node(node))
        })
        .collect()
}

fn ring_bond_lengths(engine: &Engine) -> Vec<f64> {
    let entry = engine.state().document.editable_fragment().unwrap();
    entry
        .fragment
        .bonds
        .iter()
        .map(|bond| {
            let begin = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.begin)
                .map(|node| entry.world_point_for_node(node))
                .unwrap();
            let end = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.end)
                .map(|node| entry.world_point_for_node(node))
                .unwrap();
            begin.distance(end)
        })
        .collect()
}

fn assert_no_duplicate_node_positions(engine: &Engine) {
    let entry = engine.state().document.editable_fragment().unwrap();
    for (index, left) in entry.fragment.nodes.iter().enumerate() {
        let left_point = entry.world_point_for_node(left);
        for right in entry.fragment.nodes.iter().skip(index + 1) {
            let right_point = entry.world_point_for_node(right);
            assert!(
                left_point.distance(right_point) > 0.01,
                "duplicate nodes {} and {} at {:?}",
                left.id,
                right.id,
                left_point
            );
        }
    }
}

fn fused_ring_points_for_bond(
    begin: Point,
    end: Point,
    ring_size: usize,
    side_sign: f64,
) -> Vec<Point> {
    let side = begin.distance(end).max(DEFAULT_BOND_LENGTH);
    let apothem = side / (2.0 * (std::f64::consts::PI / ring_size as f64).tan());
    let unit = chemcore_engine::Vector::new((end.x - begin.x) / side, (end.y - begin.y) / side);
    let normal = chemcore_engine::Vector::new(-unit.y, unit.x).scaled(side_sign);
    let center = Point::new(
        (begin.x + end.x) * 0.5 + normal.x * apothem,
        (begin.y + end.y) * 0.5 + normal.y * apothem,
    );
    let first_vector = chemcore_engine::Vector::new(begin.x - center.x, begin.y - center.y);
    let positive = regular_points_from_vector(ring_size, center, first_vector, 1.0);
    let negative = regular_points_from_vector(ring_size, center, first_vector, -1.0);
    if positive
        .get(1)
        .is_some_and(|point| point.distance(end) <= 0.05)
    {
        positive
    } else {
        negative
    }
}

fn regular_points_from_vector(
    ring_size: usize,
    center: Point,
    first_vector: chemcore_engine::Vector,
    direction: f64,
) -> Vec<Point> {
    let step = direction * 2.0 * std::f64::consts::PI / ring_size as f64;
    (0..ring_size)
        .map(|index| {
            let angle = step * index as f64;
            let cos = angle.cos();
            let sin = angle.sin();
            Point::new(
                center.x + first_vector.x * cos - first_vector.y * sin,
                center.y + first_vector.x * sin + first_vector.y * cos,
            )
        })
        .collect()
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

fn selection_box_rect(engine: &Engine) -> (f64, f64, f64, f64) {
    engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBox,
                x,
                y,
                width,
                height,
                ..
            } => Some((x, y, width, height)),
            _ => None,
        })
        .expect("selection box rect should exist")
}

fn selection_box_bounds(engine: &Engine) -> [f64; 4] {
    let (x, y, width, height) = selection_box_rect(engine);
    [x, y, x + width, y + height]
}

fn primitive_object_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { object_id, .. }
        | RenderPrimitive::Circle { object_id, .. }
        | RenderPrimitive::Polygon { object_id, .. }
        | RenderPrimitive::Rect { object_id, .. }
        | RenderPrimitive::Ellipse { object_id, .. }
        | RenderPrimitive::Polyline { object_id, .. }
        | RenderPrimitive::Path { object_id, .. }
        | RenderPrimitive::FilledPath { object_id, .. }
        | RenderPrimitive::Text { object_id, .. } => object_id.as_deref(),
    }
}

fn rendered_object_bounds(engine: &Engine, object_id: &str) -> [f64; 4] {
    let render_list = engine.render_list();
    chemcore_engine::render_primitives_bounds(
        render_list
            .iter()
            .filter(|primitive| primitive_object_id(primitive) == Some(object_id)),
    )
    .expect("object should have rendered bounds")
}

fn assert_bounds_contains(outer: [f64; 4], inner: [f64; 4]) {
    let epsilon = 1.0e-6;
    assert!(
        outer[0] <= inner[0] + epsilon
            && outer[1] <= inner[1] + epsilon
            && outer[2] >= inner[2] - epsilon
            && outer[3] >= inner[3] - epsilon,
        "outer bounds {outer:?} should contain inner bounds {inner:?}"
    );
}

fn assert_rect_close(actual: (f64, f64, f64, f64), expected: (f64, f64, f64, f64)) {
    assert!(
        (actual.0 - expected.0).abs() < 1.0e-9
            && (actual.1 - expected.1).abs() < 1.0e-9
            && (actual.2 - expected.2).abs() < 1.0e-9
            && (actual.3 - expected.3).abs() < 1.0e-9,
        "expected rect {actual:?} to be close to {expected:?}"
    );
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
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
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
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
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
        assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
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

fn hover(engine: &mut Engine, x: f64, y: f64) {
    engine.pointer_move(PointerEvent {
        x,
        y,
        button: None,
        alt_key: false,
    });
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

fn drag(engine: &mut Engine, from: Point, to: Point) {
    engine.pointer_down(PointerEvent {
        x: from.x,
        y: from.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: to.x,
        y: to.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: to.x,
        y: to.y,
        button: Some(0),
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
    assert!(object.payload.extra.get("arrowGeometry").is_some());

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
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
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

    engine.select_at_point(Point::new(49.0, 55.0), false);
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
fn curved_arrow_path_uses_circular_arc_control_points() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        arrow_variant: ArrowVariant::Curved,
        arrow_curve: ArrowCurve::Arc270,
        ..ToolState::default()
    });

    drag(&mut engine, Point::new(10.0, 20.0), Point::new(110.0, 20.0));

    let path_d = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                d,
                ..
            } => Some(d),
            _ => None,
        })
        .expect("curved arrow should render as a path");
    let numbers: Vec<f64> = path_d
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',' || ch == 'M' || ch == 'C')
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();
    let start = Point::new(numbers[0], numbers[1]);
    let first_control = Point::new(numbers[2], numbers[3]);
    assert!(
        start.distance(first_control) < 50.0,
        "arc control point should stay near the circular tangent, got path {path_d}"
    );
}

#[test]
fn half_arrow_heads_keep_visual_left_and_right_sides_on_curves() {
    fn rendered_half_head(
        variant: ArrowVariant,
        style: ArrowEndpointStyle,
    ) -> (Vec<Point>, Vec<Point>) {
        let mut engine = Engine::new();
        engine.set_tool_state(ToolState {
            active_tool: Tool::Arrow,
            arrow_variant: variant,
            arrow_curve: ArrowCurve::Arc120,
            arrow_head_style: style,
            ..ToolState::default()
        });
        drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

        let mut arc = Vec::new();
        let mut head = Vec::new();
        for primitive in engine.render_list() {
            match primitive {
                RenderPrimitive::Path { points, .. } | RenderPrimitive::Polyline { points, .. } => {
                    arc = points
                }
                RenderPrimitive::Line { from, to, .. } => arc = vec![from, to],
                RenderPrimitive::FilledPath { points, .. } if points.len() >= 4 => head = points,
                _ => {}
            }
        }
        (arc, head)
    }

    let (straight_arc, straight_left) =
        rendered_half_head(ArrowVariant::Solid, ArrowEndpointStyle::Left);
    assert_eq!(straight_arc.len(), 2);
    assert_point_close(
        straight_arc[1],
        Point::new(90.0 - (8.75 - 2.5 * 2.0 / 3.0), 20.0),
    );
    assert_point_close(straight_left[0], Point::new(90.0, 20.5));
    assert_point_close(straight_left[1], Point::new(80.0, 17.5));
    assert_point_close(straight_left[3], Point::new(81.25, 20.5));
    assert!(straight_left[1].y < straight_left[2].y);
    assert!(straight_left[2].y < straight_left[3].y);
    let (straight_right_shaft, straight_right) =
        rendered_half_head(ArrowVariant::Solid, ArrowEndpointStyle::Right);
    assert_eq!(straight_right_shaft.len(), 2);
    assert_point_close(
        straight_right_shaft[1],
        Point::new(90.0 - (8.75 - 2.5 * 2.0 / 3.0), 20.0),
    );
    assert_point_close(straight_right[0], Point::new(90.0, 19.5));
    assert_point_close(straight_right[1], Point::new(80.0, 22.5));
    assert!(straight_right[1].y > straight_right[2].y);
    assert!(straight_right[2].y > straight_right[3].y);
    assert_point_close(straight_right[3], Point::new(81.25, 19.5));

    let (curved_arc, curved_left) =
        rendered_half_head(ArrowVariant::Curved, ArrowEndpointStyle::Left);
    assert!(curved_arc[curved_arc.len() / 2].y < curved_arc[0].y);
    assert!((*curved_arc.last().unwrap()).distance(Point::new(90.0, 20.0)) > 1.0);
    assert!(curved_left[1].distance(curved_left[0]) > curved_left[3].distance(curved_left[0]));
    let (_, curved_right) = rendered_half_head(ArrowVariant::Curved, ArrowEndpointStyle::Right);
    assert!(curved_right[1].distance(curved_right[0]) > curved_right[3].distance(curved_right[0]));

    let (mirror_arc, mirror_left) =
        rendered_half_head(ArrowVariant::CurvedMirror, ArrowEndpointStyle::Left);
    assert!(mirror_arc[mirror_arc.len() / 2].y > mirror_arc[0].y);
    assert!((*mirror_arc.last().unwrap()).distance(Point::new(90.0, 20.0)) > 1.0);
    assert!(mirror_left[1].distance(mirror_left[0]) > mirror_left[3].distance(mirror_left[0]));
    let (_, mirror_right) =
        rendered_half_head(ArrowVariant::CurvedMirror, ArrowEndpointStyle::Right);
    assert!(mirror_right[1].distance(mirror_right[0]) > mirror_right[3].distance(mirror_right[0]));
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
                "fontSize": chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
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

#[test]
fn component_selection_from_label_selects_whole_fragment() {
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

    assert!(engine.select_component_at_point(px_point(318.0, 260.0), false));

    let selection = &engine.state().selection;
    assert_eq!(selection.nodes.len(), 2);
    assert!(selection.nodes.contains(&"n0".to_string()));
    assert!(selection.nodes.contains(&"n1".to_string()));
    assert_eq!(selection.bonds, vec!["b1".to_string()]);
    assert_eq!(selection.label_nodes, vec!["n1".to_string()]);
}

#[test]
fn clipboard_document_json_contains_selected_molecule_fragment() {
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
    assert!(engine.select_component_at_point(px_point(318.0, 260.0), false));

    let document_json = engine
        .clipboard_document_json()
        .expect("clipboard document should serialize")
        .expect("selection should produce a clipboard document");
    let document = parse_document_json(&document_json).expect("clipboard document should parse");
    let entry = document
        .editable_fragment()
        .expect("clipboard document should keep the selected molecule");

    assert_eq!(document.objects.len(), 1);
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert_eq!(entry.fragment.bonds.len(), 1);
    assert_eq!(
        entry
            .fragment
            .nodes
            .iter()
            .filter(|node| node.label.is_some())
            .count(),
        1
    );
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

fn load_arrange_text_document(engine: &mut Engine) {
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_arrange",
            "title": "arrange test",
            "page": { "width": px(400.0), "height": px(320.0), "background": "#ffffff" }
        },
        "styles": {},
        "objects": [{
            "id": "obj_text_a",
            "type": "text",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": { "text": "A", "bbox": [0.0, 0.0, 10.0, 10.0], "runs": [] }
        }, {
            "id": "obj_text_b",
            "type": "text",
            "visible": true,
            "zIndex": 21,
            "transform": { "translate": [30.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": { "text": "B", "bbox": [0.0, 0.0, 10.0, 10.0], "runs": [] }
        }, {
            "id": "obj_text_c",
            "type": "text",
            "visible": true,
            "zIndex": 22,
            "transform": { "translate": [100.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": { "text": "C", "bbox": [0.0, 0.0, 30.0, 10.0], "runs": [] }
        }]
    });
    engine
        .load_document_json(&document.to_string())
        .expect("arrange document should load");
}

fn text_translate(engine: &Engine, object_id: &str) -> [f64; 2] {
    engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("text object should exist")
        .transform
        .translate
}

fn text_bbox(engine: &Engine, object_id: &str) -> [f64; 4] {
    engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("text object should exist")
        .payload
        .bbox
        .expect("text object should have a bbox")
}

fn select_all_arrange_text_objects(engine: &mut Engine) {
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(5.0, 5.0), false);
    engine.select_at_point(Point::new(35.0, 25.0), true);
    engine.select_at_point(Point::new(105.0, 45.0), true);
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
    assert_eq!(entry.fragment.bonds[0].bond_spacing, Some(12.0));
    assert_eq!(entry.fragment.bonds[0].margin_width, Some(2.0));
}

#[test]
fn acs_document_1996_preset_sets_new_bond_metrics() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let begin = entry.world_point_for_node(&entry.fragment.nodes[0]);
    let end = entry.world_point_for_node(&entry.fragment.nodes[1]);
    let bond = &entry.fragment.bonds[0];
    assert!((begin.distance(end) - 14.4).abs() < 0.001);
    assert!((bond.stroke_width - 0.6).abs() < 0.001);
    assert_eq!(bond.bold_width, Some(2.0));
    assert_eq!(bond.wedge_width, Some(3.0));
    assert_eq!(bond.label_clip_margin, Some(0.8));
    assert_eq!(bond.hash_spacing, Some(2.5));
    assert_eq!(bond.bond_spacing, Some(18.0));
    assert_eq!(bond.margin_width, Some(1.6));
}

#[test]
fn acs_document_1996_preset_reflows_existing_endpoint_label_geometry() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.replace_hovered_endpoint_label("N"));

    engine.set_document_style_preset("acs-document-1996");

    let entry = engine.state().document.editable_fragment().unwrap();
    let labeled = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.element == "N")
        .expect("endpoint label should remain an N node");
    let label = labeled.label.as_ref().expect("N node should have a label");
    let bounds = label_glyph_bounds(label);
    let glyph_width = bounds[2] - bounds[0];
    let box_width = label.bbox().map(|bbox| bbox[2] - bbox[0]).unwrap_or(0.0);

    assert_eq!(
        label.font_size,
        Some(chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
    );
    assert!(
        glyph_width > 8.0,
        "ACS style switch should reflow glyph geometry for the 10pt label, not keep a scaled 0.48x box: {bounds:?}"
    );
    assert!(
        box_width > 7.0,
        "label bbox should also be reflowed at the current font size: {:?}",
        label.bbox()
    );
}

#[test]
fn acs_document_1996_preset_sets_bold_render_width() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(bold_bond_tool());

    click(&mut engine, px(300.0), px(260.0));

    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert_eq!(bond.line_weights.main, BondLineWeight::Bold);
    assert_eq!(bond.bold_width, Some(2.0));
    let bold_area = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                points,
                ..
            } => Some(polygon_area(&points)),
            _ => None,
        })
        .expect("bold bond should render as a filled polygon");
    assert!((bold_area - 28.8).abs() < 0.01, "{bold_area}");
}

#[test]
fn acs_document_1996_preset_sets_new_graphic_strokes() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(&mut engine, Point::new(10.0, 20.0), Point::new(90.0, 20.0));

    let arrow = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow object should exist");
    let arrow_style = arrow.style_ref.as_ref().expect("arrow should have style");
    assert_eq!(arrow_style, "style_arrow_0_60");
    assert_eq!(
        engine.state().document.styles[arrow_style]
            .get("strokeWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    drag(&mut engine, Point::new(20.0, 30.0), Point::new(60.0, 80.0));

    let shape = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    let shape_style = shape.style_ref.as_ref().expect("shape should have style");
    assert_eq!(
        engine.state().document.styles[shape_style]
            .get("strokeWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );
}

#[test]
fn acs_document_1996_preset_sets_template_ring_bond_lengths() {
    let mut blank = Engine::new();
    blank.set_document_style_preset("acs-document-1996");
    blank.set_tool_state(templates_tool("ring-6"));
    click(&mut blank, px(300.0), px(260.0));
    assert!(ring_bond_lengths(&blank)
        .iter()
        .all(|length| (length - 14.4).abs() < 0.001));

    let mut endpoint = Engine::new();
    endpoint.set_document_style_preset("acs-document-1996");
    endpoint.set_tool_state(bond_tool());
    click(&mut endpoint, px(300.0), px(260.0));
    let anchor = node_world_point(&endpoint, "n_2");
    endpoint.set_tool_state(templates_tool("ring-6"));
    click(&mut endpoint, anchor.x, anchor.y);
    assert!(ring_bond_lengths(&endpoint)
        .iter()
        .all(|length| (length - 14.4).abs() < 0.001));

    let mut fused = Engine::new();
    fused.set_document_style_preset("acs-document-1996");
    fused.set_tool_state(bond_tool());
    click(&mut fused, px(300.0), px(260.0));
    let center = {
        let entry = fused.state().document.editable_fragment().unwrap();
        let bond = &entry.fragment.bonds[0];
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5)
    };
    fused.set_tool_state(templates_tool("ring-6"));
    click(&mut fused, center.x, center.y);
    assert!(ring_bond_lengths(&fused)
        .iter()
        .all(|length| (length - 14.4).abs() < 0.001));
}

#[test]
fn acs_document_1996_preset_scales_existing_document_as_one_group() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    drag(
        &mut engine,
        Point::new(600.0, 100.0),
        Point::new(660.0, 100.0),
    );

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    drag(
        &mut engine,
        Point::new(700.0, 200.0),
        Point::new(760.0, 260.0),
    );

    let before_page = engine.state().document.document.page.clone();
    let entry = engine.state().document.editable_fragment().unwrap();
    let before_bond_start = entry.world_point_for_node(&entry.fragment.nodes[0]);
    let before_arrow_start = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| line_object_points(object).first().copied())
        .expect("arrow start should exist");
    let before_shape_translate = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape should exist")
        .transform
        .translate;

    engine.set_document_style_preset("acs-document-1996");

    assert_eq!(
        engine.state().document.document.page.width,
        before_page.width
    );
    assert_eq!(
        engine.state().document.document.page.height,
        before_page.height
    );
    let entry = engine.state().document.editable_fragment().unwrap();
    let after_bond_start = entry.world_point_for_node(&entry.fragment.nodes[0]);
    let after_bond_end = entry.world_point_for_node(&entry.fragment.nodes[1]);
    let after_arrow_start = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| line_object_points(object).first().copied())
        .expect("arrow start should exist");
    let after_shape_translate = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape should exist")
        .transform
        .translate;

    let scale = 14.4 / DEFAULT_BOND_LENGTH;
    assert!((after_bond_start.distance(after_bond_end) - 14.4).abs() < 0.001);
    assert!(
        ((after_arrow_start.x - after_bond_start.x)
            - (before_arrow_start.x - before_bond_start.x) * scale)
            .abs()
            < 0.001
    );
    assert!(
        ((after_arrow_start.y - after_bond_start.y)
            - (before_arrow_start.y - before_bond_start.y) * scale)
            .abs()
            < 0.001
    );
    assert!(
        ((after_shape_translate[0] - after_bond_start.x)
            - (before_shape_translate[0] - before_bond_start.x) * scale)
            .abs()
            < 0.001
    );
    assert!(
        ((after_shape_translate[1] - after_bond_start.y)
            - (before_shape_translate[1] - before_bond_start.y) * scale)
            .abs()
            < 0.001
    );

    let after_once = after_arrow_start;
    engine.set_document_style_preset("acs-document-1996");
    let after_twice = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .and_then(|object| line_object_points(object).first().copied())
        .expect("arrow start should exist");
    assert_point_close(after_once, after_twice);

    let bond = &engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds[0];
    assert!((bond.stroke_width - 0.6).abs() < 0.001);
    assert_eq!(bond.bold_width, Some(2.0));
    assert_eq!(bond.wedge_width, Some(3.0));
    assert_eq!(bond.label_clip_margin, Some(0.8));
    assert_eq!(bond.hash_spacing, Some(2.5));
    assert_eq!(bond.margin_width, Some(1.6));

    engine.set_document_style_preset("default");
    let entry = engine.state().document.editable_fragment().unwrap();
    let default_bond = &entry.fragment.bonds[0];
    let default_begin = entry.world_point_for_node(
        entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == default_bond.begin)
            .unwrap(),
    );
    let default_end = entry.world_point_for_node(
        entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == default_bond.end)
            .unwrap(),
    );
    assert_eq!(engine.document_style_preset(), "default");
    assert!((default_begin.distance(default_end) - DEFAULT_BOND_LENGTH).abs() < 0.001);
    assert!((default_bond.stroke_width - DEFAULT_BOND_STROKE).abs() < 0.001);
    assert_eq!(default_bond.wedge_width, Some(6.0));
    assert_eq!(default_bond.label_clip_margin, Some(1.2));
    assert_eq!(default_bond.margin_width, Some(2.0));
}

#[test]
fn object_settings_update_bond_and_graphic_metrics() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_object_settings",
            "title": "object settings",
            "page": { "width": 160.0, "height": 120.0, "background": "#ffffff" }
        },
        "styles": {
            "style_line": { "kind": "stroke", "stroke": "#111111", "strokeWidth": 1.0 },
            "style_shape": { "kind": "shape", "stroke": "#111111", "strokeWidth": 1.0, "fill": null }
        },
        "objects": [
            {
                "id": "obj_mol",
                "type": "molecule",
                "styleRef": "style_molecule_default",
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol" }
            },
            {
                "id": "obj_line",
                "type": "line",
                "styleRef": "style_line",
                "payload": {
                    "points": [[50.0, 20.0], [90.0, 20.0]],
                    "kind": "line"
                }
            },
            {
                "id": "obj_shape",
                "type": "shape",
                "styleRef": "style_shape",
                "transform": { "translate": [50.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 24.0, 12.0], "kind": "rect" }
            },
            {
                "id": "obj_bracket",
                "type": "bracket",
                "transform": { "translate": [90.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 14.0, 28.0], "kind": "round", "stroke": "#111111", "strokeWidth": 1.0 }
            }
        ],
        "resources": {
            "mol": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 20.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 10.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [40.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 1.0 }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("object settings fixture should load");

    engine.select_at_point(Point::new(25.0, 10.0), false);
    let dialog: serde_json::Value =
        serde_json::from_str(&engine.object_settings_dialog_json()).unwrap();
    let field_keys = dialog["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(field_keys, vec!["bondLength", "lineWidth", "marginWidth"]);

    let original_options = engine.options().clone();
    let changed = engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "bondLength": 15.0,
                    "lineWidth": 0.7,
                    "marginWidth": 1.8
                }
            }"#,
        )
        .expect("object settings should parse");
    assert!(changed);
    assert!((engine.options().bond_length - original_options.bond_length).abs() < 0.001);
    assert!(
        (engine.options().bond_stroke_width - original_options.bond_stroke_width).abs() < 0.001
    );
    assert!(
        (engine.options().graphic_stroke_width - original_options.graphic_stroke_width).abs()
            < 0.001
    );

    let fragment = engine.state().document.editable_fragment().unwrap();
    let bond = &fragment.fragment.bonds[0];
    let begin = fragment
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .unwrap();
    let end = fragment
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .unwrap();
    assert!((begin.point().distance(end.point()) - 15.0).abs() < 0.001);
    assert!((bond.stroke_width - 0.7).abs() < 0.001);
    assert_eq!(bond.margin_width, Some(1.8));
    assert_eq!(bond.bold_width, None);
    assert_eq!(bond.bond_spacing, None);
    assert_eq!(bond.hash_spacing, None);

    engine.select_at_point(Point::new(70.0, 20.0), false);
    let dialog: serde_json::Value =
        serde_json::from_str(&engine.object_settings_dialog_json()).unwrap();
    let field_keys = dialog["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(field_keys, vec!["lineWidth"]);
    assert!(engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "lineWidth": 0.7
                }
            }"#,
        )
        .expect("graphic settings should parse"));

    let line = engine
        .state()
        .document
        .find_scene_object("obj_line")
        .unwrap();
    let line_style = line.style_ref.as_deref().unwrap();
    assert_eq!(
        engine.state().document.styles[line_style]["strokeWidth"],
        json!(0.7)
    );
    assert_eq!(
        engine.state().document.styles["style_line"]["strokeWidth"],
        json!(1.0)
    );
    assert_eq!(
        engine.state().document.styles["style_shape"]["strokeWidth"],
        json!(1.0)
    );
    let bracket = engine
        .state()
        .document
        .find_scene_object("obj_bracket")
        .unwrap();
    assert_eq!(bracket.payload.extra["strokeWidth"], json!(1.0));
    let defaults = &engine.state().document.document.meta["import"]["cdxml"]["defaults"];
    assert!(defaults.is_null());
}

#[test]
fn object_settings_multi_selection_uses_union_and_blanks_mixed_values() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(double_bond_tool());
    click(&mut engine, px(420.0), px(260.0));

    let bond_centers = {
        let entry = engine.state().document.editable_fragment().unwrap();
        entry
            .fragment
            .bonds
            .iter()
            .map(|bond| {
                let begin = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.begin)
                    .unwrap()
                    .point();
                let end = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.end)
                    .unwrap()
                    .point();
                (
                    bond.order,
                    Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5),
                )
            })
            .collect::<Vec<_>>()
    };
    assert!(bond_centers.iter().any(|(order, _)| *order >= 2));
    assert!(engine.select_component_at_point(bond_centers[0].1, false));
    assert!(engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "bondLength": 12.0
                }
            }"#,
        )
        .expect("single selected setting should parse"));
    let bond_centers = {
        let entry = engine.state().document.editable_fragment().unwrap();
        entry
            .fragment
            .bonds
            .iter()
            .map(|bond| {
                let begin = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.begin)
                    .unwrap()
                    .point();
                let end = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == bond.end)
                    .unwrap()
                    .point();
                (
                    bond.order,
                    Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5),
                )
            })
            .collect::<Vec<_>>()
    };
    assert!(engine.select_component_at_point(bond_centers[0].1, false));
    assert!(engine.select_component_at_point(bond_centers[1].1, true));

    let dialog: serde_json::Value =
        serde_json::from_str(&engine.object_settings_dialog_json()).unwrap();
    let fields = dialog["fields"].as_array().unwrap();
    let field_keys = fields
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        field_keys,
        vec!["bondLength", "lineWidth", "bondSpacing", "marginWidth"]
    );
    let bond_length = fields
        .iter()
        .find(|field| field["key"] == "bondLength")
        .unwrap();
    assert_eq!(bond_length["mixed"], json!(true));
    assert!(bond_length["value"].is_null());

    assert!(engine
        .apply_object_settings_dialog_json(
            r#"{
                "unit": "pt",
                "values": {
                    "bondLength": 12.0,
                    "bondSpacing": 14.0
                }
            }"#,
        )
        .expect("mixed settings should parse"));

    let entry = engine.state().document.editable_fragment().unwrap();
    let selected = engine
        .state()
        .selection
        .bonds
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for bond in &entry.fragment.bonds {
        if !selected.contains(bond.id.as_str()) {
            continue;
        }
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .unwrap();
        assert!((begin.point().distance(end.point()) - 12.0).abs() < 0.02);
        if bond.order >= 2 {
            assert_eq!(bond.bond_spacing, Some(14.0));
        } else {
            assert_ne!(bond.bond_spacing, Some(14.0));
        }
    }
}

#[test]
fn engine_provides_context_menu_and_numeric_dialog_schemas() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let hit = engine.context_hit_test_json(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y));
    let menu: serde_json::Value =
        serde_json::from_str(&engine.context_menu_json(&hit, false)).unwrap();
    let labels = menu
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item.get("label").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    assert!(labels.contains(&"Bond Type"));
    assert!(labels.contains(&"Object Settings..."));

    let scale: serde_json::Value =
        serde_json::from_str(&engine.selection_numeric_dialog_json("scale")).unwrap();
    assert_eq!(scale["kind"], "scale");
    assert_eq!(scale["field"]["unit"], "%");
    assert!(engine.select_all());
    assert!(engine
        .apply_selection_numeric_dialog_json(r#"{"kind":"scale","value":110}"#)
        .unwrap());
}

#[test]
fn template_click_on_bond_uses_bond_as_ring_side() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry
        .fragment
        .bonds
        .iter()
        .any(|bond| (bond.begin == "n_1" && bond.end == "n_2")
            || (bond.begin == "n_2" && bond.end == "n_1")));
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_click_on_bond_supports_ring_sizes_three_through_eight() {
    for ring_size in 3..=8 {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        click(&mut engine, px(300.0), px(260.0));

        engine.set_tool_state(templates_tool(&format!("ring-{ring_size}")));
        click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.nodes.len(), ring_size);
        assert_eq!(entry.fragment.bonds.len(), ring_size);
        assert_no_duplicate_node_positions(&engine);
    }
}

#[test]
fn template_ring_bonds_inherit_existing_anchor_stroke_width() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let mut document: serde_json::Value =
        serde_json::from_str(&engine.state_json().expect("state json")).expect("json");
    document["document"]["resources"]["mol_editor"]["data"]["bonds"][0]["strokeWidth"] =
        json!(0.07);
    engine
        .load_document_json(
            &serde_json::to_string(&document["document"]).expect("document json should encode"),
        )
        .expect("document should reload");

    engine.set_tool_state(templates_tool("ring-3"));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert!(entry
        .fragment
        .bonds
        .iter()
        .all(|bond| (bond.stroke_width - 0.07).abs() < 0.001));
}

#[test]
fn template_endpoint_ring_connects_adjacent_intersections_through_center() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    engine.set_tool_state(templates_tool("ring-3"));
    click(&mut engine, endpoint.x, endpoint.y);

    let original_bond_points = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(bond_id),
                points,
                ..
            } if bond_id == "b_3" => Some(points),
            _ => None,
        })
        .expect("original bond should render as polygon");
    let center_index = original_bond_points
        .iter()
        .position(|point| point.distance(endpoint) < 0.001)
        .expect("polygon should include the shared center point");
    let previous = original_bond_points
        [(center_index + original_bond_points.len() - 1) % original_bond_points.len()];
    let next = original_bond_points[(center_index + 1) % original_bond_points.len()];

    const ENDPOINT_RING_JUNCTION_TOLERANCE_PT: f64 = 2.267_716_535_433_071;

    assert!(
        previous.distance(endpoint) < ENDPOINT_RING_JUNCTION_TOLERANCE_PT,
        "{previous:?}"
    );
    assert!(
        next.distance(endpoint) < ENDPOINT_RING_JUNCTION_TOLERANCE_PT,
        "{next:?}"
    );

    assert!(
        engine.render_list().into_iter().all(|primitive| {
            !matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond,
                    bond_id: None,
                    points,
                    ..
                } if points
                    .iter()
                    .any(|point| point.distance(endpoint) < ENDPOINT_RING_JUNCTION_TOLERANCE_PT)
            )
        }),
        "endpoint ring junction should be covered by bond polygons, not an extra center patch"
    );

    let incident_areas: Vec<f64> = engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role: RenderRole::DocumentBond,
                bond_id: Some(_),
                points,
                ..
            } if points
                .iter()
                .any(|point| point.distance(endpoint) < DEFAULT_BOND_STROKE) =>
            {
                Some(polygon_area(&points))
            }
            _ => None,
        })
        .collect();
    assert!(
        incident_areas.iter().all(|area| *area > 0.01),
        "{incident_areas:?}"
    );
}

#[test]
fn template_click_on_endpoint_attaches_ring_on_symmetry_axis() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let existing_begin = node_world_point(&engine, "n_1");
    let endpoint = node_world_point(&engine, "n_2");

    engine.set_tool_state(templates_tool("ring-5"));
    click(&mut engine, endpoint.x, endpoint.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == "n_2" || bond.end == "n_2")
            .count(),
        3
    );
    let ring_points = entry
        .fragment
        .nodes
        .iter()
        .filter(|node| node.id != "n_1")
        .map(|node| entry.world_point_for_node(node))
        .collect::<Vec<_>>();
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    let expected_axis = chemcore_engine::angle_between(existing_begin, endpoint);
    let actual_axis = chemcore_engine::angle_between(endpoint, center);
    assert!(
        chemcore_engine::angular_distance(expected_axis, actual_axis) < 0.2,
        "{expected_axis} {actual_axis}"
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_endpoint_snaps_ring_axis_to_15_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    let target = endpoint.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));
    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    let ring_points = {
        let entry = engine.state().document.editable_fragment().unwrap();
        entry
            .fragment
            .nodes
            .iter()
            .filter(|node| node.id != "n_1")
            .map(|node| entry.world_point_for_node(node))
            .collect::<Vec<_>>()
    };
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    assert!((chemcore_engine::angle_between(endpoint, center) - 15.0).abs() < 0.2);
    assert_eq!(attached_node_points(&engine, "n_2").len(), 3);
    assert!(
        attached_node_points(&engine, "n_2")
            .iter()
            .filter(|point| point.distance(node_world_point(&engine, "n_1")) > 0.03)
            .count()
            == 2
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_endpoint_keeps_live_focus_on_connection_anchor() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    let target = endpoint.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));
    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("template drag should keep live focus on the connection endpoint");
    assert_eq!(hover.node_id, "n_2");
    assert!((hover.point.x - endpoint.x).abs() < 0.001, "{hover:?}");
    assert!((hover.point.y - endpoint.y).abs() < 0.001, "{hover:?}");
    assert!(hover.point.distance(target) > DEFAULT_BOND_LENGTH);
    let preview = engine
        .state()
        .overlay
        .preview
        .as_ref()
        .expect("template drag should keep the ring preview active");
    assert!((preview.end.x - endpoint.x).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - endpoint.y).abs() < 0.001, "{preview:?}");
    assert!(preview.end.distance(target) > DEFAULT_BOND_LENGTH);
}

#[test]
fn template_chain_drag_on_blank_canvas_creates_open_zigzag() {
    let mut engine = Engine::new();
    let start = px_point(300.0, 260.0);
    let target = start.translated(direction_from_angle(0.0).scaled(DEFAULT_BOND_LENGTH * 4.1));

    engine.set_tool_state(templates_tool("chain"));
    drag(&mut engine, start, target);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 5);
    assert_eq!(entry.fragment.bonds.len(), 4);
    for bond in &entry.fragment.bonds {
        let begin = node_world_point(&engine, &bond.begin);
        let end = node_world_point(&engine, &bond.end);
        assert!((begin.distance(end) - DEFAULT_BOND_LENGTH).abs() < 0.001);
    }
    assert!(!entry.fragment.bonds.iter().any(|bond| {
        (bond.begin == "n_1" && bond.end == "n_5") || (bond.begin == "n_5" && bond.end == "n_1")
    }));
    let first_angle = angle_between(
        node_world_point(&engine, "n_1"),
        node_world_point(&engine, "n_2"),
    );
    let second_angle = angle_between(
        node_world_point(&engine, "n_2"),
        node_world_point(&engine, "n_3"),
    );
    assert!(chemcore_engine::angular_distance(first_angle, 30.0) < 0.2);
    assert!(chemcore_engine::angular_distance(second_angle, 330.0) < 0.2);
}

#[test]
fn template_chain_drag_on_endpoint_reuses_anchor_node() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let endpoint = node_world_point(&engine, "n_2");
    let target = endpoint.translated(direction_from_angle(45.0).scaled(DEFAULT_BOND_LENGTH * 3.1));
    engine.set_tool_state(templates_tool("chain"));
    drag(&mut engine, endpoint, target);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 5);
    assert_eq!(entry.fragment.bonds.len(), 4);
    assert_eq!(attached_node_points(&engine, "n_2").len(), 2);
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chain_click_without_drag_does_not_insert_chain() {
    let mut engine = Engine::new();
    engine.set_tool_state(templates_tool("chain"));

    click(&mut engine, px(300.0), px(260.0));

    assert_eq!(fragment_counts(&engine), (0, 0));
}

#[test]
fn template_chain_drag_preview_shows_terminal_count_label() {
    let mut engine = Engine::new();
    let start = px_point(300.0, 260.0);
    let target = start.translated(direction_from_angle(0.0).scaled(DEFAULT_BOND_LENGTH * 3.1));
    engine.set_tool_state(templates_tool("chain"));

    engine.pointer_down(PointerEvent {
        x: start.x,
        y: start.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });

    let preview_label = engine
        .render_list()
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                object_id: Some(object_id),
                text,
                font_size,
                ..
            } if object_id == "__preview_chain_count" => Some((text, font_size)),
            _ => None,
        })
        .expect("chain drag should show terminal count label");
    assert_eq!(preview_label.0, "4");
    assert_eq!(preview_label.1, 8.0);

    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });
    assert!(engine.render_list().into_iter().all(|primitive| {
        !matches!(
            primitive,
            RenderPrimitive::Text {
                object_id: Some(object_id),
                ..
            } if object_id == "__preview_chain_count"
        )
    }));
}

#[test]
fn template_click_on_blank_canvas_creates_regular_ring() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);

    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, anchor.x, anchor.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    let ring_points = entry
        .fragment
        .nodes
        .iter()
        .map(|node| entry.world_point_for_node(node))
        .collect::<Vec<_>>();
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    assert!(center.distance(anchor) < 0.002, "{center:?} {anchor:?}");
    assert!(ring_points
        .iter()
        .all(|point| point.distance(anchor) > 0.01));
    for bond in &entry.fragment.bonds {
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .map(|node| entry.world_point_for_node(node))
            .unwrap();
        assert!(
            (begin.distance(end) - DEFAULT_BOND_LENGTH).abs() < 0.01,
            "{begin:?} {end:?}"
        );
    }
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_click_creates_alternating_double_bonds() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);

    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, anchor.x, anchor.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2 && bond.double.is_some())
            .count(),
        3
    );
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 1 && bond.double.is_none())
            .count(),
        3
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_benzene_click_on_bond_keeps_fused_side_and_adds_three_double_bonds() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    engine.set_tool_state(templates_tool("benzene"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    let original_bond = entry
        .fragment
        .bonds
        .iter()
        .find(|bond| bond.id == "b_3")
        .expect("clicked bond should be reused");
    assert_eq!(original_bond.order, 1);
    assert_eq!(
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.order == 2 && bond.double.is_some())
            .count(),
        3
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chair_click_on_blank_canvas_centers_shape() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);

    engine.set_tool_state(templates_tool("chair-6-right"));
    click(&mut engine, center.x, center.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry
        .fragment
        .bonds
        .iter()
        .all(|bond| bond.order == 1 && bond.double.is_none()));
    let points = entry
        .fragment
        .nodes
        .iter()
        .map(|node| entry.world_point_for_node(node))
        .collect::<Vec<_>>();
    let actual_center = Point::new(
        points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64,
        points.iter().map(|point| point.y).sum::<f64>() / points.len() as f64,
    );
    assert!(
        actual_center.distance(center) < 0.005,
        "{actual_center:?} {center:?}"
    );
    let first_bond_angle = chemcore_engine::angle_between(points[0], points[1]);
    assert!(
        chemcore_engine::angular_distance(60.0, first_bond_angle) < 0.2,
        "{first_bond_angle}"
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chair_drag_on_blank_canvas_uses_initial_point_as_anchor() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);
    let target = anchor.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(templates_tool("chair-6-right"));
    drag(&mut engine, anchor, target);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 6);
    assert_eq!(entry.fragment.bonds.len(), 6);
    assert!(entry
        .fragment
        .nodes
        .iter()
        .any(|node| entry.world_point_for_node(node).distance(anchor) < 0.01));
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_chair_endpoint_click_aligns_anchor_bisector_with_existing_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    let existing_begin = node_world_point(&engine, "n_1");
    let endpoint = node_world_point(&engine, "n_2");

    engine.set_tool_state(templates_tool("chair-6-right"));
    click(&mut engine, endpoint.x, endpoint.y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 7);
    assert_eq!(entry.fragment.bonds.len(), 7);
    let new_neighbors = attached_node_points(&engine, "n_2")
        .into_iter()
        .filter(|point| point.distance(existing_begin) > 0.01)
        .collect::<Vec<_>>();
    assert_eq!(new_neighbors.len(), 2, "{new_neighbors:?}");
    let first = chemcore_engine::Vector::new(
        new_neighbors[0].x - endpoint.x,
        new_neighbors[0].y - endpoint.y,
    )
    .normalized();
    let second = chemcore_engine::Vector::new(
        new_neighbors[1].x - endpoint.x,
        new_neighbors[1].y - endpoint.y,
    )
    .normalized();
    let bisector = Point::new(
        endpoint.x + first.x + second.x,
        endpoint.y + first.y + second.y,
    );
    let expected_axis = chemcore_engine::angle_between(existing_begin, endpoint);
    let actual_axis = chemcore_engine::angle_between(endpoint, bisector);
    assert!(
        chemcore_engine::angular_distance(expected_axis, actual_axis) < 0.2,
        "{expected_axis} {actual_axis}"
    );
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_blank_canvas_snaps_ring_axis_to_15_degrees() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);
    let target = anchor.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: anchor.x,
        y: anchor.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    let ring_points = {
        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.nodes.len(), 6);
        assert_eq!(entry.fragment.bonds.len(), 6);
        entry
            .fragment
            .nodes
            .iter()
            .map(|node| entry.world_point_for_node(node))
            .collect::<Vec<_>>()
    };
    let center = Point::new(
        ring_points.iter().map(|point| point.x).sum::<f64>() / ring_points.len() as f64,
        ring_points.iter().map(|point| point.y).sum::<f64>() / ring_points.len() as f64,
    );
    assert!(ring_points
        .iter()
        .any(|point| point.distance(anchor) < 0.01));
    assert!((chemcore_engine::angle_between(anchor, center) - 15.0).abs() < 0.2);
    assert_no_duplicate_node_positions(&engine);
}

#[test]
fn template_drag_on_blank_canvas_keeps_live_focus_on_initial_connection() {
    let mut engine = Engine::new();
    let anchor = px_point(300.0, 260.0);
    let target = anchor.translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(templates_tool("ring-6"));
    engine.pointer_down(PointerEvent {
        x: anchor.x,
        y: anchor.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });

    let hover = engine
        .state()
        .overlay
        .hover_endpoint
        .as_ref()
        .expect("template drag should keep live focus on the initial generated connection");
    assert!((hover.point.x - anchor.x).abs() < 0.001, "{hover:?}");
    assert!((hover.point.y - anchor.y).abs() < 0.001, "{hover:?}");
    assert!(hover.point.distance(target) > DEFAULT_BOND_LENGTH);
    let preview = engine
        .state()
        .overlay
        .preview
        .as_ref()
        .expect("template drag should keep the ring preview active");
    assert!((preview.end.x - anchor.x).abs() < 0.001, "{preview:?}");
    assert!((preview.end.y - anchor.y).abs() < 0.001, "{preview:?}");
    assert!(preview.end.distance(target) > DEFAULT_BOND_LENGTH);
}

#[test]
fn template_click_reuses_existing_endpoint_at_generated_ring_vertex() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));

    let ring_points = fused_ring_points_for_bond(
        node_world_point(&engine, "n_1"),
        node_world_point(&engine, "n_2"),
        6,
        1.0,
    );
    let reusable_point = ring_points[2];
    engine.add_single_bond_between(
        chemcore_engine::BondAnchor {
            node_id: None,
            point: reusable_point,
            label_anchor: None,
        },
        chemcore_engine::BondAnchor {
            node_id: None,
            point: reusable_point
                .translated(direction_from_angle(37.0).scaled(DEFAULT_BOND_LENGTH * 1.7)),
            label_anchor: None,
        },
    );
    let reusable_id = node_id_at(&engine, reusable_point).expect("reusable node should exist");

    engine.set_tool_state(templates_tool("ring-6"));
    click(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 7);
    assert!(entry.fragment.bonds.iter().any(|bond| {
        (bond.begin == "n_2" && bond.end == reusable_id)
            || (bond.begin == reusable_id && bond.end == "n_2")
    }));
    assert_no_duplicate_node_positions(&engine);
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
    assert_eq!(node.num_hydrogens, 2);
    assert!(!node.is_placeholder);
    assert_eq!(
        node.label.as_ref().map(|label| label.text.as_str()),
        Some("NH2")
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.font_size),
        Some(chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.align.as_deref()),
        Some("left")
    );
}

#[test]
fn shortcut_generated_hydrogen_label_preserves_manual_endpoint_text_edit() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    hover(&mut engine, FIRST_END_HOVER_X, FIRST_END_HOVER_Y);
    assert!(engine.replace_hovered_endpoint_label("N"));

    let label_box = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
            .unwrap();
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.source_text.as_deref()),
            Some("NH2")
        );
        assert!(
            node.meta.get("labelRecognition").is_none(),
            "generated one-connection NH2 should be valid"
        );
        node.label.as_ref().and_then(|label| label.bbox()).unwrap()
    };
    let session = engine
        .begin_text_edit(Point::new(
            (label_box[0] + label_box[2]) * 0.5,
            (label_box[1] + label_box[3]) * 0.5,
        ))
        .expect("label edit session should start");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "NH".to_string(),
        source_runs: Vec::new(),
        ..session
    }));

    let label_box = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
            .unwrap();
        assert_eq!(node.element, "N");
        assert_eq!(node.num_hydrogens, 2);
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.source_text.as_deref()),
            Some("NH")
        );
        assert_eq!(
            node.meta
                .get("implicitHydrogenLabel")
                .and_then(|value| value.get("userEdited"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("implicitHydrogenLabel"))
                .and_then(|value| value.get("userEdited"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            node.meta
                .get("labelRecognition")
                .and_then(|value| value.get("status"))
                .and_then(serde_json::Value::as_str),
            Some("invalid")
        );
        node.label.as_ref().and_then(|label| label.bbox()).unwrap()
    };

    let session = engine
        .begin_text_edit(Point::new(
            (label_box[0] + label_box[2]) * 0.5,
            (label_box[1] + label_box[3]) * 0.5,
        ))
        .expect("label edit session should restart");
    assert!(engine.apply_text_edit(chemcore_engine::TextEditSession {
        text: "NX".to_string(),
        source_runs: Vec::new(),
        ..session
    }));
    let entry = engine.state().document.editable_fragment().unwrap();
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.position == [FIRST_END_X, FIRST_END_Y])
        .unwrap();
    assert_eq!(
        node.meta
            .get("labelRecognition")
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str),
        Some("invalid")
    );
}

#[test]
fn hovered_endpoint_can_be_replaced_with_abbreviation_label() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());

    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(delete_tool());
    engine.set_tool_state(bond_tool());
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
        Some(chemcore_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
    );
    assert_eq!(
        node.label.as_ref().and_then(|label| label.align.as_deref()),
        Some("left")
    );

    engine.set_tool_state(select_tool());
    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
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
fn single_group_label_right_anchor_uses_terminal_letter_but_not_digit() {
    for (label, expected_anchor_x) in [("Ph", px(305.0)), ("N3", px(297.0))] {
        let mut engine = Engine::new();
        engine.set_tool_state(bond_tool());
        load_label_document(
            &mut engine,
            label,
            vec![
                rect_polygon(294.0, 256.0, 300.0, 264.0),
                rect_polygon(302.0, 256.0, 308.0, 264.0),
            ],
            json!([]),
        );

        hover(&mut engine, px(305.0), px(260.0));

        let anchor = engine
            .state()
            .overlay
            .hover_endpoint
            .as_ref()
            .and_then(|hover| hover.label_anchor.as_ref())
            .expect("label anchor should exist");
        let right_group_point = anchor
            .right_group_point
            .expect("right group anchor should exist");
        assert!(
            (right_group_point.x - expected_anchor_x).abs() < 0.01,
            "{label}: {right_group_point:?}"
        );
        assert!(
            (right_group_point.y - px(260.0)).abs() < 0.01,
            "{label}: {right_group_point:?}"
        );
    }
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
        (last.position[0] - px(305.0)).abs() < 0.01,
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
        (last.position[0] - px(353.0)).abs() < 0.01,
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
fn click_draw_clears_hover_after_commit() {
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

    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.preview.is_none());
}

#[test]
fn dragged_bond_clears_hover_after_commit() {
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

    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.preview.is_none());
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
        (last.position[0] - px(389.0)).abs() < 0.001,
        "{:?}",
        last.position
    );
    assert!(
        (last.position[1] - px(301.0)).abs() < 0.001,
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
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right | DoubleBondPlacement::Center)
    ));
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
        assert_eq!(bond.double.as_ref().map(|double| double.frozen), Some(true));
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

    engine.set_tool_state(dashed_bond_tool());
    engine.pointer_down(PointerEvent {
        x: FIRST_CENTER_X,
        y: FIRST_CENTER_Y,
        button: Some(0),
        alt_key: false,
    });
    let entry = engine.state().document.editable_fragment().unwrap();
    let bond = &entry.fragment.bonds[0];
    assert!(matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right | DoubleBondPlacement::Center)
    ));
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
            Some(
                DoubleBondPlacement::Left
                    | DoubleBondPlacement::Right
                    | DoubleBondPlacement::Center
            )
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
            Some(
                DoubleBondPlacement::Left
                    | DoubleBondPlacement::Right
                    | DoubleBondPlacement::Center
            )
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
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right | DoubleBondPlacement::Center)
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
    assert!(matches!(
        closed_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    ));
    assert_ne!(
        closed_bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
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
fn select_delete_atom_removes_attached_bonds_but_keeps_neighbor_atoms() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert_eq!(fragment_counts(&engine), (3, 2));

    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.delete_selection());

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.bonds.len(), 0);
    assert_eq!(entry.fragment.nodes.len(), 2);
    assert!(entry.fragment.nodes.iter().all(|node| node.id != "n_2"));
}

#[test]
fn select_copy_and_paste_selected_bond_duplicates_atoms_and_bond() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());

    let entry = engine.state().document.editable_fragment().unwrap();
    assert_eq!(entry.fragment.nodes.len(), 4);
    assert_eq!(entry.fragment.bonds.len(), 2);
    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
}

#[test]
fn select_cut_stores_bond_then_deletes_and_allows_paste() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.cut_selection());
    assert_eq!(fragment_counts(&engine), (0, 0));
    assert!(engine.paste_clipboard());
    assert_eq!(fragment_counts(&engine), (2, 1));
}

#[test]
fn select_cut_undo_redo_is_one_command() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.cut_selection());
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
fn select_tool_does_not_hover_selected_text_object() {
    let mut engine = Engine::new();
    load_text_object_document(&mut engine);
    engine.set_tool_state(select_tool());

    let point = px_point(300.0, 250.0);
    engine.select_at_point(point, false);
    engine.pointer_move(PointerEvent {
        x: point.x,
        y: point.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::HoverTextBox,
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
fn select_tool_does_not_hover_selected_label_box() {
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

    let point = px_point(305.0, 260.0);
    engine.select_at_point(point, false);
    engine.pointer_move(PointerEvent {
        x: point.x,
        y: point.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.state().overlay.hover_text_box.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::HoverLabelGlyph | RenderRole::HoverTextBox,
            ..
        }
    )));
}

#[test]
fn select_tool_does_not_hover_selected_bond_or_atom() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);

    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polygon {
            role: RenderRole::HoverBondCenter,
            ..
        }
    )));
    let state_json: serde_json::Value =
        serde_json::from_str(&engine.state_json().expect("state json")).expect("json");
    assert!(state_json["overlay"]["hoverBondCenter"].is_null());
    assert!(state_json["overlay"].get("hoverBondTarget").is_none());

    engine.select_at_point(Point::new(FIRST_END_X, FIRST_END_Y), false);
    hover(&mut engine, FIRST_END_X, FIRST_END_Y);

    assert!(engine.state().overlay.hover_endpoint.is_none());
    let state_json: serde_json::Value =
        serde_json::from_str(&engine.state_json().expect("state json")).expect("json");
    assert!(state_json["overlay"]["hoverEndpoint"].is_null());
    assert!(!engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Circle {
            role: RenderRole::HoverEndpoint,
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

fn shape_payload_point(engine: &Engine, key: &str) -> Point {
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    let coords = object
        .payload
        .extra
        .get(key)
        .and_then(serde_json::Value::as_array)
        .expect("shape point should exist");
    Point::new(
        coords[0].as_f64().expect("x should be numeric"),
        coords[1].as_f64().expect("y should be numeric"),
    )
}

fn first_shape_object(engine: &Engine) -> &chemcore_engine::SceneObject {
    engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist")
}

fn shape_object_count(engine: &Engine) -> usize {
    engine
        .state()
        .document
        .objects
        .iter()
        .filter(|object| object.object_type == "shape")
        .count()
}

fn hover_shape_handle_count(engine: &Engine) -> usize {
    engine
        .render_list()
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::HoverShapeHandle,
                    ..
                }
            )
        })
        .count()
}

#[test]
fn switching_to_select_selects_latest_changed_graphic_or_molecule_component() {
    let mut shape_engine = Engine::new();
    shape_engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    shape_engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    shape_engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: None,
        alt_key: false,
    });
    shape_engine.pointer_up(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: Some(0),
        alt_key: false,
    });
    let shape_id = shape_engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist")
        .id
        .clone();
    shape_engine.set_tool_state(select_tool());
    assert_eq!(shape_engine.state().selection.arrow_objects, vec![shape_id]);

    let mut arrow_engine = Engine::new();
    arrow_engine.set_tool_state(ToolState {
        active_tool: Tool::Arrow,
        ..ToolState::default()
    });
    arrow_engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    arrow_engine.pointer_move(PointerEvent {
        x: 70.0,
        y: 20.0,
        button: None,
        alt_key: false,
    });
    arrow_engine.pointer_up(PointerEvent {
        x: 70.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    let arrow_id = arrow_engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "line")
        .expect("arrow object should exist")
        .id
        .clone();
    arrow_engine.set_tool_state(select_tool());
    assert_eq!(arrow_engine.state().selection.arrow_objects, vec![arrow_id]);

    let mut bond_engine = Engine::new();
    bond_engine.set_tool_state(bond_tool());
    click(&mut bond_engine, px(300.0), px(260.0));
    bond_engine.set_tool_state(select_tool());
    assert_eq!(bond_engine.state().selection.nodes.len(), 2);
    assert_eq!(bond_engine.state().selection.bonds.len(), 1);
}

#[test]
fn switching_to_select_without_tool_changes_does_not_restore_previous_latest_object() {
    let mut engine = Engine::new();
    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: Some(0),
        alt_key: false,
    });
    engine.set_tool_state(select_tool());
    assert!(!engine.state().selection.arrow_objects.is_empty());

    engine.select_at_point(Point::new(500.0, 500.0), false);
    assert!(engine.state().selection.is_empty());

    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.set_tool_state(select_tool());
    assert!(engine.state().selection.is_empty());
}

#[test]
fn shape_tool_circle_uses_click_as_center_and_cursor_as_radius() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);
    let cursor = px_point(360.0, 290.0);

    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: None,
        alt_key: false,
    });

    let preview = engine.render_list();
    assert!(preview.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: Some(id),
            stroke_width,
            ..
        } if id == "__preview_shape" && (*stroke_width - 1.0).abs() < 0.001
    )));

    engine.pointer_up(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: Some(0),
        alt_key: false,
    });

    assert_point_close(shape_payload_point(&engine, "center"), center);
    assert_point_close(shape_payload_point(&engine, "majorAxisEnd"), cursor);
    let minor = shape_payload_point(&engine, "minorAxisEnd");
    let radius = center.distance(cursor);
    assert!((center.distance(minor) - radius).abs() < 0.001);
}

#[test]
fn shape_tool_ellipse_uses_center_and_snaps_major_axis_to_15_degrees() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);
    let cursor = center.translated(direction_from_angle(29.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(shape_tool(ShapeKind::Ellipse, ShapeStyle::Dashed));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: Some(0),
        alt_key: false,
    });

    assert_point_close(shape_payload_point(&engine, "center"), center);
    let major = shape_payload_point(&engine, "majorAxisEnd");
    let minor = shape_payload_point(&engine, "minorAxisEnd");
    assert!((angle_between(center, major) - 30.0).abs() < 0.001);
    assert!((center.distance(minor) / center.distance(major) - 0.4).abs() < 0.001);
}

#[test]
fn shape_tool_rectangles_use_drag_corners() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Shadowed));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    assert_eq!(object.transform.translate, [top_left.x, top_left.y]);
    assert_eq!(
        object.payload.bbox,
        Some([
            0.0,
            0.0,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y
        ])
    );
    assert_eq!(
        object
            .payload
            .extra
            .get("kind")
            .and_then(serde_json::Value::as_str),
        Some("roundRect")
    );
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            stroke_width,
            dash_array,
            d,
            ..
        } if (*stroke_width - 1.0).abs() < 0.001 && dash_array.is_empty() && d.starts_with("M ")
    )));
}

#[test]
fn shape_tool_click_on_existing_atom_adds_fixed_rect_centered_on_atom() {
    let mut engine = Engine::new();
    let endpoint = px_point(300.0, 260.0);
    let target = px_point(330.0, 260.0);

    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });

    let object = first_shape_object(&engine);
    assert_point_close(
        Point::new(object.transform.translate[0], object.transform.translate[1]),
        Point::new(endpoint.x - 7.7, endpoint.y - 7.7),
    );
    let bbox = object.payload.bbox.expect("shape should have bbox");
    assert!((bbox[2] - 15.4).abs() < 1e-9, "{bbox:?}");
    assert!((bbox[3] - 15.4).abs() < 1e-9, "{bbox:?}");
}

#[test]
fn shape_tool_drag_from_atom_uses_atom_as_rect_corner() {
    let mut engine = Engine::new();
    let endpoint = px_point(300.0, 260.0);
    let bond_target = px_point(330.0, 260.0);
    let rect_target = px_point(340.0, 292.0);

    engine.set_tool_state(bond_tool());
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bond_target.x,
        y: bond_target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bond_target.x,
        y: bond_target.y,
        button: Some(0),
        alt_key: false,
    });

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: endpoint.x,
        y: endpoint.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: rect_target.x,
        y: rect_target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: rect_target.x,
        y: rect_target.y,
        button: Some(0),
        alt_key: false,
    });

    let object = first_shape_object(&engine);
    assert_point_close(
        Point::new(object.transform.translate[0], object.transform.translate[1]),
        endpoint,
    );
    assert_eq!(
        object.payload.bbox,
        Some([
            0.0,
            0.0,
            rect_target.x - endpoint.x,
            rect_target.y - endpoint.y
        ])
    );
}

#[test]
fn shape_tool_click_on_label_rect_uses_label_box() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(294.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(294.0), px(264.0)]
        ])],
        json!([]),
    );
    let label_center = px_point(309.0, 260.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: label_center.x,
        y: label_center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: label_center.x,
        y: label_center.y,
        button: Some(0),
        alt_key: false,
    });

    let object = first_shape_object(&engine);
    assert_point_close(
        Point::new(object.transform.translate[0], object.transform.translate[1]),
        px_point(294.0, 256.0),
    );
    assert_eq!(object.payload.bbox, Some([0.0, 0.0, px(30.0), px(8.0)]));
}

#[test]
fn shape_tool_drag_from_label_circle_uses_label_center() {
    let mut engine = Engine::new();
    load_label_document(
        &mut engine,
        "Ph",
        vec![json!([
            [px(294.0), px(256.0)],
            [px(324.0), px(256.0)],
            [px(324.0), px(264.0)],
            [px(294.0), px(264.0)]
        ])],
        json!([]),
    );
    let click_point = px_point(296.0, 258.0);
    let label_center = px_point(309.0, 260.0);
    let target = px_point(340.0, 260.0);

    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: click_point.x,
        y: click_point.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: target.x,
        y: target.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: target.x,
        y: target.y,
        button: Some(0),
        alt_key: false,
    });

    assert_point_close(shape_payload_point(&engine, "center"), label_center);
    assert_point_close(shape_payload_point(&engine, "majorAxisEnd"), target);
}

#[test]
fn shape_tool_ignores_plain_text_focus() {
    let mut engine = Engine::new();
    load_text_object_document(&mut engine);

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_move(PointerEvent {
        x: px(300.0),
        y: px(250.0),
        button: None,
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_text_box.is_none());

    engine.pointer_down(PointerEvent {
        x: px(300.0),
        y: px(250.0),
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: px(300.0),
        y: px(250.0),
        button: Some(0),
        alt_key: false,
    });
    assert!(
        engine
            .state()
            .document
            .objects
            .iter()
            .all(|object| object.object_type != "shape"),
        "plain text click should not create a shape"
    );
}

#[test]
fn select_tool_click_selects_loaded_shape_object() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_shape_select",
            "title": "shape select",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "fill": null
            }
        },
        "objects": [{
            "id": "obj_shape_loaded",
            "type": "shape",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [20.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape",
            "payload": {
                "kind": "rect",
                "bbox": [0.0, 0.0, 40.0, 24.0]
            }
        }],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("shape document should load");
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(40.0, 42.0), false);

    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_shape_loaded".to_string()]
    );
}

#[test]
fn select_tool_shape_hover_and_hit_testing_follow_shape_geometry() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_shape_select",
            "title": "shape select",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "fill": null
            }
        },
        "objects": [{
            "id": "obj_circle_loaded",
            "type": "shape",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_shape",
            "payload": {
                "kind": "circle",
                "bbox": [20.0, 20.0, 40.0, 40.0],
                "center": [40.0, 40.0],
                "majorAxisEnd": [60.0, 40.0],
                "minorAxisEnd": [40.0, 60.0]
            }
        }],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("shape document should load");
    engine.set_tool_state(select_tool());

    engine.pointer_move(PointerEvent {
        x: 40.0,
        y: 20.0,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine.hover_shape_action_at_point(Point::new(40.0, 20.0)),
        "circle-radius"
    );
    assert_eq!(hover_shape_handle_count(&engine), 1);

    engine.select_at_point(Point::new(40.0, 40.0), false);
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec!["obj_circle_loaded".to_string()]
    );

    engine.select_at_point(Point::new(63.5, 63.5), false);
    assert!(engine.state().selection.arrow_objects.is_empty());
}

#[test]
fn selected_shape_boxes_are_tight_axis_aligned_bounds() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1" },
        "document": {
            "id": "doc_shape_selection_boxes",
            "title": "shape selection boxes",
            "page": { "width": 260.0, "height": 180.0, "background": "#ffffff" }
        },
        "styles": {
            "style_shape": {
                "kind": "shape",
                "stroke": "#000000",
                "strokeWidth": 0.6,
                "fill": null
            }
        },
        "objects": [
            {
                "id": "shape_circle",
                "type": "shape",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "circle",
                    "bbox": [20.0, 20.0, 40.0, 40.0],
                    "center": [40.0, 40.0],
                    "majorAxisEnd": [60.0, 40.0],
                    "minorAxisEnd": [40.0, 60.0]
                }
            },
            {
                "id": "shape_ellipse",
                "type": "shape",
                "visible": true,
                "zIndex": 11,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "ellipse",
                    "bbox": [80.0, 60.0, 80.0, 80.0],
                    "center": [120.0, 100.0],
                    "majorAxisEnd": [160.0, 100.0],
                    "minorAxisEnd": [120.0, 112.0]
                }
            },
            {
                "id": "shape_rect",
                "type": "shape",
                "visible": true,
                "zIndex": 12,
                "transform": { "translate": [170.0, 40.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "rect",
                    "bbox": [0.0, 0.0, 48.0, 24.0]
                }
            },
            {
                "id": "shape_rotated_ellipse",
                "type": "shape",
                "visible": true,
                "zIndex": 13,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_shape",
                "payload": {
                    "kind": "ellipse",
                    "bbox": [10.0, 80.0, 100.0, 100.0],
                    "center": [50.0, 130.0],
                    "majorAxisEnd": [80.0, 170.0],
                    "minorAxisEnd": [42.0, 136.0]
                }
            }
        ],
        "resources": {}
    });
    engine
        .load_document_json(&document.to_string())
        .expect("shape document should load");
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(40.0, 40.0), false);
    assert_rect_close(selection_box_rect(&engine), (19.7, 19.7, 40.6, 40.6));

    engine.select_at_point(Point::new(120.0, 100.0), false);
    assert_rect_close(selection_box_rect(&engine), (79.7, 87.7, 80.6, 24.6));

    engine.select_at_point(Point::new(190.0, 50.0), false);
    assert_rect_close(selection_box_rect(&engine), (169.7, 39.7, 48.6, 24.6));

    engine.select_at_point(Point::new(50.0, 130.0), false);
    let rotated_extent_x = (30.0_f64 * 30.0 + (-8.0_f64) * (-8.0)).sqrt();
    let rotated_extent_y = (40.0_f64 * 40.0 + 6.0 * 6.0).sqrt();
    assert_rect_close(
        selection_box_rect(&engine),
        (
            50.0 - rotated_extent_x - 0.3,
            130.0 - rotated_extent_y - 0.3,
            rotated_extent_x * 2.0 + 0.6,
            rotated_extent_y * 2.0 + 0.6,
        ),
    );
}

#[test]
fn shape_tool_circle_edge_handle_resizes_existing_circle_without_drawing_new_shape() {
    let mut engine = Engine::new();
    let center = Point::new(40.0, 40.0);
    let edge = Point::new(60.0, 40.0);
    engine.set_tool_state(shape_tool(ShapeKind::Circle, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: Some(0),
        alt_key: false,
    });

    assert_eq!(shape_object_count(&engine), 1);
    engine.pointer_move(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: None,
        alt_key: false,
    });
    assert_eq!(engine.hover_shape_action_at_point(edge), "circle-radius");
    assert_eq!(hover_shape_handle_count(&engine), 1);

    engine.pointer_down(PointerEvent {
        x: edge.x,
        y: edge.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 80.0,
        y: 40.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 80.0,
        y: 40.0,
        button: Some(0),
        alt_key: false,
    });
    assert!(engine.state().overlay.hover_shape.is_none());
    assert_eq!(hover_shape_handle_count(&engine), 0);

    assert_eq!(shape_object_count(&engine), 1);
    assert_point_close(
        shape_payload_point(&engine, "majorAxisEnd"),
        Point::new(80.0, 40.0),
    );
}

#[test]
fn shape_tool_ellipse_handles_resize_axes_and_non_handle_drag_draws_new_shape() {
    let mut engine = Engine::new();
    let center = Point::new(40.0, 40.0);
    let major = Point::new(80.0, 40.0);
    engine.set_tool_state(shape_tool(ShapeKind::Ellipse, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: major.x,
        y: major.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: major.x,
        y: major.y,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_move(PointerEvent {
        x: major.x,
        y: major.y,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine.hover_shape_action_at_point(major),
        "ellipse-major-positive"
    );
    assert_eq!(hover_shape_handle_count(&engine), 4);

    engine.pointer_down(PointerEvent {
        x: major.x,
        y: major.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 100.0,
        y: 40.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 100.0,
        y: 40.0,
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(shape_object_count(&engine), 1);
    assert_point_close(
        shape_payload_point(&engine, "majorAxisEnd"),
        Point::new(100.0, 40.0),
    );

    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 65.0,
        y: 65.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 65.0,
        y: 65.0,
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(shape_object_count(&engine), 2);
}

#[test]
fn shape_tool_rect_handles_resize_but_edge_non_handles_continue_drawing() {
    let mut engine = Engine::new();
    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Solid));
    engine.pointer_down(PointerEvent {
        x: 20.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 60.0,
        y: 44.0,
        button: Some(0),
        alt_key: false,
    });

    engine.pointer_move(PointerEvent {
        x: 60.0,
        y: 32.0,
        button: None,
        alt_key: false,
    });
    assert_eq!(
        engine.hover_shape_action_at_point(Point::new(60.0, 32.0)),
        "e"
    );
    assert_eq!(hover_shape_handle_count(&engine), 8);
    engine.pointer_down(PointerEvent {
        x: 60.0,
        y: 32.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 80.0,
        y: 32.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 80.0,
        y: 32.0,
        button: Some(0),
        alt_key: false,
    });

    assert_eq!(shape_object_count(&engine), 1);
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "shape")
        .expect("shape object should exist");
    assert_eq!(object.payload.bbox, Some([0.0, 0.0, 60.0, 24.0]));

    engine.pointer_down(PointerEvent {
        x: 35.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 90.0,
        y: 55.0,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 90.0,
        y: 55.0,
        button: Some(0),
        alt_key: false,
    });
    assert_eq!(shape_object_count(&engine), 2);
}

#[test]
fn shape_tool_dashed_round_rect_uses_chemdraw_path_dash_spacing() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Dashed));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            stroke_width,
            dash_array,
            d,
            ..
        } if (*stroke_width - 1.0).abs() < 0.001
            && dash_array == &vec![2.7]
            && d.starts_with(&format!("M {},{}", top_left.x, bottom_right.y - 6.0))
    )));
}

#[test]
fn shape_tool_shaded_style_renders_chemdraw_gray_layers() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::Rect, ShapeStyle::Shaded));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    let render_list = engine.render_list();
    let shaded_fills = render_list
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::FilledPath {
                    role: RenderRole::DocumentGraphic,
                    fill_rule: None,
                    ..
                }
            )
        })
        .count();
    assert!(
        shaded_fills >= 32,
        "expected ChemDraw-style shaded fill stack"
    );
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            stroke_width,
            dash_array,
            ..
        } if (*stroke_width - 1.0).abs() < 0.001 && dash_array.is_empty()
    )));
}

#[test]
fn shape_tool_shadowed_style_masks_shadow_inside_original_shape() {
    let mut engine = Engine::new();
    let top_left = px_point(300.0, 260.0);
    let bottom_right = px_point(380.0, 330.0);

    engine.set_tool_state(shape_tool(ShapeKind::RoundRect, ShapeStyle::Shadowed));
    engine.pointer_down(PointerEvent {
        x: top_left.x,
        y: top_left.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: bottom_right.x,
        y: bottom_right.y,
        button: Some(0),
        alt_key: false,
    });

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            clip_rule: Some(rule),
            clip_path_d: Some(_),
            fill,
            ..
        } if rule == "evenodd" && fill == "rgba(0,0,0,0.247059)"
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            stroke: None,
            stroke_width,
            ..
        } if *stroke_width == 0.0
    )));
}

#[test]
fn shape_tool_shadowed_ellipse_matches_reference_clipped_shadow() {
    let mut engine = Engine::new();
    let center = px_point(300.0, 260.0);
    let cursor = center.translated(direction_from_angle(30.0).scaled(DEFAULT_BOND_LENGTH * 2.0));

    engine.set_tool_state(shape_tool(ShapeKind::Ellipse, ShapeStyle::Shadowed));
    engine.pointer_down(PointerEvent {
        x: center.x,
        y: center.y,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: None,
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: cursor.x,
        y: cursor.y,
        button: Some(0),
        alt_key: false,
    });

    let render_list = engine.render_list();
    assert!(render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            clip_rule: Some(rule),
            clip_path_d: Some(_),
            fill,
            ..
        } if rule == "evenodd" && fill == "rgba(0,0,0,0.247059)"
    )));
    assert!(!render_list.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            fill_rule: Some(rule),
            ..
        } if rule == "evenodd"
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
fn select_tool_whole_fragment_box_ignores_hidden_atom_handles() {
    let mut complete = Engine::new();
    complete.set_tool_state(bond_tool());
    click(&mut complete, px(300.0), px(260.0));
    complete.set_tool_state(select_tool());
    complete.select_in_rect(px_point(290.0, 234.0), px_point(346.0, 286.0), false);
    let complete_rect = selection_box_rect(&complete);

    let mut bond = Engine::new();
    bond.set_tool_state(bond_tool());
    click(&mut bond, px(300.0), px(260.0));
    bond.set_tool_state(select_tool());
    bond.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    let bond_rect = selection_bond_rect(&bond);

    assert_rect_close(complete_rect, bond_rect);
    assert!(!complete.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionNode,
            ..
        }
    )));
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
fn switching_from_select_tool_to_drawing_tool_clears_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());

    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);
    assert!(!engine.state().selection.is_empty());
    assert!(engine.render_list().iter().any(|primitive| {
        matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBond,
                ..
            } | RenderPrimitive::Circle {
                role: RenderRole::SelectionBondDot,
                ..
            }
        )
    }));

    engine.set_tool_state(bond_tool());

    assert!(engine.state().selection.is_empty());
    assert!(engine.render_list().iter().all(|primitive| {
        !matches!(
            primitive,
            RenderPrimitive::Rect {
                role: RenderRole::SelectionBox
                    | RenderRole::SelectionBond
                    | RenderRole::SelectionNode
                    | RenderRole::SelectionTextBox,
                ..
            } | RenderPrimitive::Circle {
                role: RenderRole::SelectionBondDot,
                ..
            }
        )
    }));
}

#[test]
fn selection_color_applies_to_all_selected_object_kinds() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_selection_color",
            "title": "selection color",
            "page": { "width": 160.0, "height": 100.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#111111",
                "fill": "#111111",
                "strokeWidth": 0.85,
                "fontSize": 10.0
            },
            "style_text": { "kind": "text", "fill": "#111111", "fontSize": 10.0 },
            "style_line": {
                "kind": "line",
                "stroke": "#111111",
                "strokeWidth": 1.0,
                "lineCap": "round",
                "lineJoin": "round"
            },
            "style_shape": {
                "kind": "shape",
                "fill": null,
                "stroke": "#111111",
                "strokeWidth": 1.0
            }
        },
        "objects": [
            {
                "id": "obj_mol",
                "type": "molecule",
                "styleRef": "style_molecule_default",
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol" }
            },
            {
                "id": "obj_text",
                "type": "text",
                "styleRef": "style_text",
                "transform": { "translate": [40.0, 4.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 26.0, 12.0],
                    "text": "note",
                    "fontSize": 10.0,
                    "lineHeight": 12.0,
                    "align": "left",
                    "preserveLines": true
                }
            },
            {
                "id": "obj_line",
                "type": "line",
                "styleRef": "style_line",
                "payload": { "points": [[8.0, 40.0], [30.0, 40.0]], "kind": "line" }
            },
            {
                "id": "obj_shape",
                "type": "shape",
                "styleRef": "style_shape",
                "transform": { "translate": [40.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 22.0, 14.0], "kind": "rect" }
            },
            {
                "id": "obj_bracket",
                "type": "bracket",
                "transform": { "translate": [70.0, 26.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 16.0, 24.0],
                    "kind": "round",
                    "stroke": "#111111",
                    "strokeWidth": 1.0
                }
            },
            {
                "id": "obj_symbol",
                "type": "symbol",
                "transform": { "translate": [96.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "bbox": [0.0, 0.0, 10.0, 10.0],
                    "kind": "plus",
                    "fill": "#111111",
                    "strokeWidth": 1.0
                }
            }
        ],
        "resources": {
            "mol": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 20.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 10.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("selection color fixture should load");

    engine.select_in_rect(Point::new(0.0, 0.0), Point::new(120.0, 70.0), false);
    assert_eq!(engine.state().selection.text_objects, vec!["obj_text"]);
    assert!(engine
        .state()
        .selection
        .arrow_objects
        .iter()
        .any(|object_id| object_id == "obj_shape"));
    assert!(engine.apply_color_to_selection("#2288cc"));

    let document = &engine.state().document;
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment should remain editable");
    assert_eq!(
        fragment.bonds[0].stroke.as_deref(),
        Some("#2288cc"),
        "selected bond should receive its own stroke"
    );

    let style_value = |style_id: &str, key: &str| {
        document
            .styles
            .get(style_id)
            .and_then(|style| style.get(key))
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    assert_eq!(
        style_value("style_obj_text_color", "fill").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        style_value("style_obj_line_color", "stroke").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        style_value("style_obj_shape_color", "stroke").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        style_value("style_obj_mol_color", "stroke").as_deref(),
        Some("#2288cc")
    );
    let payload_value = |object_id: &str, key: &str| {
        document
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .and_then(|object| object.payload.extra.get(key))
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    assert_eq!(
        payload_value("obj_bracket", "stroke").as_deref(),
        Some("#2288cc")
    );
    assert_eq!(
        payload_value("obj_symbol", "fill").as_deref(),
        Some("#2288cc")
    );
}

#[test]
fn context_style_commands_apply_to_graphic_text_and_bond_selections() {
    let mut engine = Engine::new();
    let document = json!({
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_context_styles",
            "title": "context styles",
            "page": { "width": 160.0, "height": 100.0, "background": "#ffffff" }
        },
        "styles": {
            "style_line": { "kind": "line", "stroke": "#111111", "strokeWidth": 1.0 },
            "style_shape": { "kind": "shape", "stroke": "#111111", "strokeWidth": 1.0, "fill": null },
            "style_text": { "kind": "text", "fill": "#111111", "fontSize": 10.0 }
        },
        "objects": [
            {
                "id": "obj_mol",
                "type": "molecule",
                "styleRef": "style_molecule_default",
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "resourceRef": "mol" }
            },
            {
                "id": "obj_line",
                "type": "line",
                "styleRef": "style_line",
                "payload": {
                    "points": [[12.0, 42.0], [34.0, 42.0]],
                    "kind": "line",
                    "arrowHead": { "kind": "solid", "head": "full", "tail": "none", "length": 10.0, "centerLength": 8.0, "width": 2.0, "bold": false }
                }
            },
            {
                "id": "obj_shape",
                "type": "shape",
                "styleRef": "style_shape",
                "transform": { "translate": [48.0, 30.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 22.0, 14.0], "kind": "rect" }
            },
            {
                "id": "obj_bracket",
                "type": "bracket",
                "transform": { "translate": [78.0, 26.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 16.0, 24.0], "kind": "round", "stroke": "#111111", "strokeWidth": 1.0 }
            },
            {
                "id": "obj_text",
                "type": "text",
                "styleRef": "style_text",
                "transform": { "translate": [105.0, 28.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": { "bbox": [0.0, 0.0, 32.0, 14.0], "text": "note", "fontSize": 10.0, "lineHeight": 12.0, "align": "left", "preserveLines": true }
            }
        ],
        "resources": {
            "mol": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 20.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 10.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n2", "element": "C", "atomicNumber": 6, "position": [30.0, 10.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("context style fixture should load");

    engine.select_at_point(Point::new(55.0, 36.0), false);
    assert!(engine.apply_shape_style_to_selection("filled"));
    let shape = engine
        .state()
        .document
        .find_scene_object("obj_shape")
        .expect("shape should exist");
    let shape_style = engine
        .state()
        .document
        .styles
        .get(shape.style_ref.as_deref().unwrap())
        .expect("shape style should exist");
    assert!(shape_style
        .get("stroke")
        .is_some_and(|value| value.is_null()));
    assert_eq!(shape_style["fill"], "#111111");

    engine.select_at_point(Point::new(80.0, 30.0), false);
    assert!(engine.apply_bracket_kind_to_selection("curly"));
    let bracket = engine
        .state()
        .document
        .find_scene_object("obj_bracket")
        .expect("bracket should exist");
    assert_eq!(bracket.payload.extra["kind"], "curly");

    engine.select_at_point(Point::new(23.0, 42.0), false);
    assert!(engine.apply_line_style_to_selection("bold"));
    let line = engine
        .state()
        .document
        .find_scene_object("obj_line")
        .expect("line should exist");
    assert_eq!(line.payload.extra["arrowHead"]["bold"], true);
    let line_style = engine
        .state()
        .document
        .styles
        .get(line.style_ref.as_deref().unwrap())
        .expect("line style should exist");
    assert_eq!(line_style["strokeWidth"], 2.0);

    engine.select_at_point(Point::new(112.0, 34.0), false);
    assert!(engine.apply_text_style_to_selection("bold", "on"));
    assert!(engine.apply_text_style_to_selection("align", "center"));
    let text = engine
        .state()
        .document
        .find_scene_object("obj_text")
        .expect("text should exist");
    assert_eq!(text.payload.extra["align"], "center");
    assert_eq!(text.payload.extra["runs"][0]["fontWeight"], 700.0);

    engine.select_at_point(Point::new(20.0, 10.0), false);
    assert!(engine.apply_bond_style_to_selection("double-double-dashed"));
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = fragment.bonds.iter().find(|bond| bond.id == "b1").unwrap();
    assert_eq!(bond.order, 2);
    assert_eq!(bond.line_styles.left, BondLinePattern::Dashed);
    assert_eq!(bond.line_styles.right, BondLinePattern::Dashed);

    assert!(engine.undo());
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = fragment.bonds.iter().find(|bond| bond.id == "b1").unwrap();
    assert_eq!(bond.order, 1);
}

#[test]
fn hovered_bond_style_shortcut_updates_bond_without_changing_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    let first_bond_id = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .bonds[0]
        .id
        .clone();
    assert!(engine.select_all());
    let before_selection = engine.state().selection.clone();

    engine.set_tool_state(select_tool());
    let center = bond_center_point(&engine, &first_bond_id);
    engine.pointer_move(PointerEvent {
        x: center.x,
        y: center.y,
        button: None,
        alt_key: false,
    });

    assert!(engine.apply_hovered_bond_style("double-center"));
    assert_eq!(engine.state().selection, before_selection);

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let bond = fragment
        .bonds
        .iter()
        .find(|bond| bond.id == first_bond_id)
        .unwrap();
    assert_eq!(bond.order, 2);
    assert_eq!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Center)
    );
}

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
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_chemical_check_toggle",
            "title": "chemical check toggle",
            "page": { "width": 80.0, "height": 60.0, "background": "#ffffff" }
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
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
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

    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(end, false));
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
        "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
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

#[test]
fn select_tool_move_undo_redo_returns_to_final_drag_position() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let start = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let mid = Point::new(FIRST_CENTER_X + px(10.0), FIRST_CENTER_Y);
    let end = Point::new(FIRST_CENTER_X + px(24.0), FIRST_CENTER_Y + px(18.0));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.update_selection_move(mid, false));
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));

    let final_n1 = node_world_point(&engine, "n_1");
    assert!(engine.undo());
    assert_eq!(
        node_world_point(&engine, "n_1"),
        Point::new(FIRST_START_X, FIRST_START_Y)
    );
    assert!(engine.redo());
    assert_eq!(node_world_point(&engine, "n_1"), final_n1);
}

#[test]
fn select_tool_resizing_selected_bond_from_east_scales_selected_nodes() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let center = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);

    engine.select_at_point(center, false);
    let (x, y, width, height) = selection_bond_rect(&engine);
    let pivot_x = x;
    let target = Point::new(x + width * 2.0, y + height * 0.5);

    assert!(engine.begin_selection_resize("east", Point::new(x + width, y + height * 0.5)));
    assert!(engine.update_selection_resize(target));
    assert!(engine.finish_selection_resize(target));

    let n1 = node_world_point(&engine, "n_1");
    let n2 = node_world_point(&engine, "n_2");
    assert!((n1.x - round_to_2(pivot_x + (FIRST_START_X - pivot_x) * 2.0)).abs() < 0.001);
    assert!((n1.y - FIRST_START_Y).abs() < 0.001);
    assert!((n2.x - round_to_2(pivot_x + (FIRST_END_X - pivot_x) * 2.0)).abs() < 0.001);
    assert!((n2.y - FIRST_END_Y).abs() < 0.001);

    let final_n2 = n2;
    assert!(engine.undo());
    assert_eq!(
        node_world_point(&engine, "n_2"),
        Point::new(FIRST_END_X, FIRST_END_Y)
    );
    assert!(engine.redo());
    assert_eq!(node_world_point(&engine, "n_2"), final_n2);
}

#[test]
fn select_tool_resizing_one_text_selection_box_scales_all_selected_text_objects() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.begin_selection_resize("east", Point::new(40.0, 25.0)));
    assert!(engine.update_selection_resize(Point::new(260.0, 25.0)));
    assert!(engine.finish_selection_resize(Point::new(260.0, 25.0)));

    assert_eq!(text_translate(&engine, "obj_text_a"), [0.0, 0.0]);
    assert_eq!(text_translate(&engine, "obj_text_b"), [60.0, 20.0]);
    assert_eq!(text_translate(&engine, "obj_text_c"), [200.0, 40.0]);
    assert_eq!(text_bbox(&engine, "obj_text_a")[2], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_b")[2], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_c")[2], 60.0);
}

#[test]
fn select_tool_corner_resize_is_proportional() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.begin_selection_resize("ne", Point::new(130.0, 0.0)));
    assert!(engine.update_selection_resize(Point::new(260.0, -50.0)));
    assert!(engine.finish_selection_resize(Point::new(260.0, -50.0)));

    assert_eq!(text_translate(&engine, "obj_text_b"), [60.0, -10.0]);
    assert_eq!(text_translate(&engine, "obj_text_c"), [200.0, 30.0]);
    assert_eq!(text_bbox(&engine, "obj_text_a")[2], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_a")[3], 20.0);
    assert_eq!(text_bbox(&engine, "obj_text_c")[2], 60.0);
    assert_eq!(text_bbox(&engine, "obj_text_c")[3], 20.0);
}

#[test]
fn select_tool_dragging_unselected_bond_focus_starts_move() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(10000.0, 10000.0), false);
    let start = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let end = Point::new(FIRST_CENTER_X + px(16.0), FIRST_CENTER_Y);

    assert!(engine.state().selection.is_empty());
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(engine.state().selection.bonds, vec!["b_3"]);
    assert!(engine.update_selection_move(end, false));
    assert!(engine.finish_selection_move(end, false));
    assert!(engine.state().selection.is_empty());

    let entry = engine.state().document.editable_fragment().unwrap();
    let n1 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_1")
        .unwrap();
    assert!((n1.position[0] - round_to_2(FIRST_START_X + px(16.0))).abs() < 0.001);
}

#[test]
fn select_tool_rotating_selected_bond_snaps_to_15_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let center = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let start = center.translated(direction_from_angle(0.0).scaled(1.0));
    let target = center.translated(direction_from_angle(22.0).scaled(1.0));

    engine.select_at_point(center, false);
    assert!(engine.begin_selection_rotate(start));
    assert!(engine.update_selection_rotate(target, false));
    assert!(engine.render_list().iter().all(|primitive| !matches!(
        primitive,
        RenderPrimitive::Rect {
            role: RenderRole::SelectionBox
                | RenderRole::SelectionBond
                | RenderRole::SelectionNode
                | RenderRole::SelectionTextBox,
            ..
        } | RenderPrimitive::Circle {
            role: RenderRole::SelectionBondDot,
            ..
        }
    )));
    assert!(engine.finish_selection_rotate(target, false));

    let expected_n1 = rotate_point_around(Point::new(FIRST_START_X, FIRST_START_Y), center, 15.0);
    let expected_n2 = rotate_point_around(Point::new(FIRST_END_X, FIRST_END_Y), center, 15.0);
    let n1 = node_world_point(&engine, "n_1");
    let n2 = node_world_point(&engine, "n_2");
    assert!((n1.x - round_to_2(expected_n1.x)).abs() < 0.001, "{n1:?}");
    assert!((n1.y - round_to_2(expected_n1.y)).abs() < 0.001, "{n1:?}");
    assert!((n2.x - round_to_2(expected_n2.x)).abs() < 0.001, "{n2:?}");
    assert!((n2.y - round_to_2(expected_n2.y)).abs() < 0.001, "{n2:?}");
}

#[test]
fn select_tool_alt_rotating_selected_bond_uses_free_angle() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let center = Point::new(FIRST_CENTER_X, FIRST_CENTER_Y);
    let start = center.translated(direction_from_angle(0.0).scaled(1.0));
    let target = center.translated(direction_from_angle(22.0).scaled(1.0));

    engine.select_at_point(center, false);
    assert!(engine.begin_selection_rotate(start));
    assert!(engine.update_selection_rotate(target, true));
    assert!(engine.finish_selection_rotate(target, true));

    let expected_n2 = rotate_point_around(Point::new(FIRST_END_X, FIRST_END_Y), center, 22.0);
    let n2 = node_world_point(&engine, "n_2");
    assert!((n2.x - round_to_2(expected_n2.x)).abs() < 0.001, "{n2:?}");
    assert!((n2.y - round_to_2(expected_n2.y)).abs() < 0.001, "{n2:?}");
}

#[test]
fn select_tool_dragging_single_terminal_endpoint_snaps_to_15_degrees() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(10000.0, 10000.0), false);
    let start = Point::new(FIRST_END_X, FIRST_END_Y);
    let target = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 1.4));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.update_selection_move(target, false));
    assert!(engine.state().overlay.hover_endpoint.is_none());
    assert!(engine.finish_selection_move(target, false));

    let expected = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(15.0).scaled(DEFAULT_BOND_LENGTH));
    let entry = engine.state().document.editable_fragment().unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n2.position[0] - round_to_2(expected.x)).abs() < 0.001);
    assert!((n2.position[1] - round_to_2(expected.y)).abs() < 0.001);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
}

#[test]
fn select_tool_dragging_unselected_single_terminal_endpoint_clears_temporary_selection() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(10000.0, 10000.0), false);
    let start = Point::new(FIRST_END_X, FIRST_END_Y);
    let target = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(22.0).scaled(DEFAULT_BOND_LENGTH * 1.4));

    assert!(engine.state().selection.is_empty());
    assert!(engine.begin_selection_move_at_point(start, false, false));
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
    assert!(engine.update_selection_move(target, false));
    assert!(engine.finish_selection_move(target, false));

    let expected = Point::new(FIRST_START_X, FIRST_START_Y)
        .translated(direction_from_angle(15.0).scaled(DEFAULT_BOND_LENGTH));
    let entry = engine.state().document.editable_fragment().unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n2.position[0] - round_to_2(expected.x)).abs() < 0.001);
    assert!((n2.position[1] - round_to_2(expected.y)).abs() < 0.001);
    assert!(engine.state().selection.is_empty());
}

#[test]
fn select_tool_alt_dragging_single_terminal_endpoint_uses_pointer_position() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    let start = Point::new(FIRST_END_X, FIRST_END_Y);
    let target = Point::new(FIRST_END_X + px(17.0), FIRST_END_Y + px(23.0));

    engine.select_at_point(start, false);
    assert!(engine.begin_selection_move_at_point(start, false, true));
    assert!(engine.update_selection_move(target, true));
    assert!(engine.finish_selection_move(target, true));

    let entry = engine.state().document.editable_fragment().unwrap();
    let n2 = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n_2")
        .unwrap();
    assert!((n2.position[0] - round_to_2(target.x)).abs() < 0.001);
    assert!((n2.position[1] - round_to_2(target.y)).abs() < 0.001);
    assert_eq!(engine.state().selection.nodes, vec!["n_2"]);
}

#[test]
fn select_toolbar_align_left_uses_outer_left_edge() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("align-left"));

    assert_eq!(text_translate(&engine, "obj_text_a")[0], 0.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[0], 0.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[0], 0.0);
}

#[test]
fn select_toolbar_vertical_center_aligns_box_centers_on_y_axis() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("align-v-center"));

    assert_eq!(text_translate(&engine, "obj_text_a")[1], 20.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[1], 20.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[1], 20.0);
}

#[test]
fn select_toolbar_horizontal_distribution_equalizes_edge_gaps_not_centers() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("distribute-h"));

    assert_eq!(text_translate(&engine, "obj_text_a")[0], 0.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[0], 50.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[0], 100.0);
}

#[test]
fn select_toolbar_flip_horizontal_keeps_selection_center_fixed() {
    let mut engine = Engine::new();
    load_arrange_text_document(&mut engine);
    select_all_arrange_text_objects(&mut engine);

    assert!(engine.apply_selection_arrange_command("flip-h"));

    assert_eq!(text_translate(&engine, "obj_text_a")[0], 120.0);
    assert_eq!(text_translate(&engine, "obj_text_b")[0], 90.0);
    assert_eq!(text_translate(&engine, "obj_text_c")[0], 0.0);
}

#[test]
fn select_toolbar_flip_horizontal_mirrors_selected_molecule_geometry_in_place() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, px(300.0), px(260.0));
    engine.set_tool_state(select_tool());
    engine.select_at_point(Point::new(FIRST_CENTER_X, FIRST_CENTER_Y), false);

    assert!(engine.apply_selection_arrange_command("flip-h"));

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
    assert!((n1.position[0] - FIRST_END_X).abs() < 0.01);
    assert!((n1.position[1] - FIRST_START_Y).abs() < 0.01);
    assert!((n2.position[0] - FIRST_START_X).abs() < 0.01);
    assert!((n2.position[1] - FIRST_END_Y).abs() < 0.01);
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
    let focus_length = (0..4)
        .map(|index| center_rect[index].distance(center_rect[(index + 1) % 4]))
        .fold(0.0, f64::max);
    assert!(
        (focus_length - DEFAULT_BOND_LENGTH * 0.5).abs() < 0.001,
        "bond center focus length should be half the bond length: {focus_length}"
    );

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
        Some(DoubleBondPlacement::Left),
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
        Some(DoubleBondPlacement::Right),
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
fn double_tool_does_not_default_to_center_when_each_endpoint_has_one_same_side_substituent() {
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
            point: px_point(300.0, 242.0),
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
        Some(DoubleBondPlacement::Left)
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
            Some(DoubleBondPlacement::Right)
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
        Some(DoubleBondPlacement::Left)
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

#[test]
fn bracket_tool_drag_creates_bracket_object() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Square,
        ..ToolState::default()
    });

    drag(
        &mut engine,
        Point::new(120.0, 130.0),
        Point::new(180.0, 220.0),
    );

    let bracket = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "bracket")
        .expect("dragging bracket tool should create bracket object");
    assert_eq!(
        bracket
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("square")
    );
    assert_eq!(bracket.payload.bbox, Some([0.0, 0.0, 60.0, 90.0]));
}

#[test]
fn bracket_tool_focuses_bonds_but_not_endpoints() {
    let mut engine = Engine::new();
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Round,
        ..ToolState::default()
    });

    hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);
    assert!(engine.state().overlay.hover_bond_center.is_some());
    assert!(engine.state().overlay.hover_endpoint.is_none());

    hover(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert!(engine.state().overlay.hover_endpoint.is_none());
}

#[test]
fn bracket_symbol_click_creates_selectable_symbol_object() {
    let mut engine = Engine::new();
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::DoubleDagger,
        ..ToolState::default()
    });

    click(&mut engine, 150.0, 160.0);
    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("clicking double dagger should create symbol object");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("double-dagger")
    );
    let symbol_id = symbol.id.clone();

    engine.set_tool_state(ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    });
    engine.select_at_point(Point::new(150.0, 160.0), false);
    assert_eq!(engine.state().selection.arrow_objects, vec![symbol_id]);
}

#[test]
fn symbol_tool_focuses_endpoints_but_not_bonds() {
    let mut engine = Engine::new();
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    hover(&mut engine, FIRST_CENTER_X, FIRST_CENTER_Y);
    assert!(engine.state().overlay.hover_bond_center.is_none());
    assert!(engine.state().overlay.hover_endpoint.is_none());

    hover(&mut engine, FIRST_END_X, FIRST_END_Y);
    assert!(engine.state().overlay.hover_endpoint.is_some());
    assert!(engine.state().overlay.hover_bond_center.is_none());
}

#[test]
fn symbol_tool_drag_from_endpoint_orbits_around_endpoint() {
    let mut engine = Engine::new();
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    drag(
        &mut engine,
        Point::new(FIRST_END_X, FIRST_END_Y),
        Point::new(FIRST_END_X, FIRST_END_Y - 13.0),
    );

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("dragging from an endpoint should create a symbol");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("plus")
    );
    assert_eq!(
        round_to_2(symbol.transform.translate[0]),
        round_to_2(FIRST_END_X - 2.16675)
    );
    assert_eq!(
        round_to_2(symbol.transform.translate[1]),
        round_to_2(FIRST_END_Y - 13.0 - 2.16675)
    );
}

fn load_symbol_direction_document(
    engine: &mut Engine,
    nodes: serde_json::Value,
    bonds: serde_json::Value,
) {
    let document = json!({
        "format": {"name": "chemcore", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_symbol_direction",
            "title": "symbol direction",
            "page": {"width": 200.0, "height": 160.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 200.0, 160.0]}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 200.0, 160.0],
                    "nodes": nodes,
                    "bonds": bonds
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("symbol direction fixture should load");
}

#[test]
fn symbol_tool_click_on_single_bond_endpoint_uses_extension_angle() {
    let mut engine = Engine::new();
    load_symbol_direction_document(
        &mut engine,
        json!([
            {"id": "n1", "element": "C", "atomicNumber": 6, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "n2", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0}
        ]),
        json!([
            {"id": "b1", "begin": "n1", "end": "n2", "order": 1}
        ]),
    );
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 130.0, 100.0);

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("clicking endpoint should create a symbol");
    assert_eq!(round_to_2(symbol.transform.translate[0]), 132.50);
    assert_eq!(round_to_2(symbol.transform.translate[1]), 96.25);
}

#[test]
fn symbol_tool_click_on_two_bond_junction_uses_convex_side_center() {
    let mut engine = Engine::new();
    load_symbol_direction_document(
        &mut engine,
        json!([
            {"id": "n1", "element": "C", "atomicNumber": 6, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "n2", "element": "C", "atomicNumber": 6, "position": [70.0, 130.0], "charge": 0, "numHydrogens": 0},
            {"id": "n3", "element": "C", "atomicNumber": 6, "position": [130.0, 130.0], "charge": 0, "numHydrogens": 0}
        ]),
        json!([
            {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
            {"id": "b2", "begin": "n1", "end": "n3", "order": 1}
        ]),
    );
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 100.0, 100.0);

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("clicking two-bond junction should create a symbol");
    assert_eq!(round_to_2(symbol.transform.translate[0]), 96.25);
    assert_eq!(round_to_2(symbol.transform.translate[1]), 90.00);
}

#[test]
fn charge_symbol_attaches_to_terminal_carbon_and_reduces_hidden_hydrogen() {
    let mut engine = Engine::new();
    engine.set_tool_state(bond_tool());
    click(&mut engine, FIRST_START_X, FIRST_START_Y);
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    click(&mut engine, FIRST_END_X, FIRST_END_Y);

    let entry = engine.state().document.editable_fragment().unwrap();
    let terminal = entry
        .fragment
        .nodes
        .iter()
        .find(|node| (node.position[0] - FIRST_END_X).abs() < 0.01)
        .expect("terminal carbon should exist");
    assert_eq!(terminal.charge, 1);
    assert_eq!(
        terminal
            .meta
            .get("effectiveNumHydrogens")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        terminal
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        None
    );
    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should exist");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("attachedAtomId")
            .and_then(|value| value.as_str()),
        Some(terminal.id.as_str())
    );
}

#[test]
fn ordinary_charge_or_radical_on_four_connected_carbon_is_invalid_but_radical_ion_is_allowed() {
    fn four_connected_engine(symbol_kind: BracketKind) -> Engine {
        let mut engine = Engine::new();
        load_symbol_direction_document(
            &mut engine,
            json!([
                {"id": "n0", "element": "C", "atomicNumber": 6, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "n1", "element": "C", "atomicNumber": 6, "position": [70.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "n2", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "n3", "element": "C", "atomicNumber": 6, "position": [100.0, 70.0], "charge": 0, "numHydrogens": 0},
                {"id": "n4", "element": "C", "atomicNumber": 6, "position": [100.0, 130.0], "charge": 0, "numHydrogens": 0}
            ]),
            json!([
                {"id": "b1", "begin": "n0", "end": "n1", "order": 1},
                {"id": "b2", "begin": "n0", "end": "n2", "order": 1},
                {"id": "b3", "begin": "n0", "end": "n3", "order": 1},
                {"id": "b4", "begin": "n0", "end": "n4", "order": 1}
            ]),
        );
        engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });
        click(&mut engine, 100.0, 100.0);
        engine
    }

    for symbol_kind in [BracketKind::Plus, BracketKind::Minus, BracketKind::Electron] {
        let engine = four_connected_engine(symbol_kind);
        let entry = engine.state().document.editable_fragment().unwrap();
        let center = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n0")
            .unwrap();
        assert_eq!(
            center
                .meta
                .get("chargeSymbolInvalid")
                .and_then(|value| value.as_bool()),
            Some(true),
            "{symbol_kind:?} should be invalid on four-connected carbon"
        );
        assert!(
            engine.render_list().into_iter().any(|primitive| matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::DocumentDiagnostic,
                    node_id: Some(ref node_id),
                    stroke,
                    ..
                } if node_id == "n0" && stroke == "#d32f2f"
            )),
            "{symbol_kind:?} should render an invalid red circle"
        );
    }

    for symbol_kind in [BracketKind::RadicalCation, BracketKind::RadicalAnion] {
        let engine = four_connected_engine(symbol_kind);
        let entry = engine.state().document.editable_fragment().unwrap();
        let center = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n0")
            .unwrap();
        assert_eq!(
            center
                .meta
                .get("chargeSymbolInvalid")
                .and_then(|value| value.as_bool()),
            None,
            "{symbol_kind:?} should be allowed on four-connected carbon"
        );
    }
}

#[test]
fn hetero_atom_charge_symbols_update_hydrogens_and_invalid_state() {
    fn terminal_nitrogen_engine(symbol_kind: BracketKind) -> Engine {
        let mut engine = Engine::new();
        load_symbol_direction_document(
            &mut engine,
            json!([
                {"id": "n1", "element": "N", "atomicNumber": 7, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "c1", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0}
            ]),
            json!([
                {"id": "b1", "begin": "n1", "end": "c1", "order": 1}
            ]),
        );
        engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });
        click(&mut engine, 100.0, 100.0);
        engine
    }

    let engine = terminal_nitrogen_engine(BracketKind::Plus);
    let nitrogen = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    assert_eq!(nitrogen.charge, 1);
    assert_eq!(nitrogen.num_hydrogens, 3);
    assert_eq!(
        nitrogen
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        None
    );

    let engine = terminal_nitrogen_engine(BracketKind::Minus);
    let nitrogen = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    assert_eq!(nitrogen.charge, -1);
    assert_eq!(nitrogen.num_hydrogens, 1);
    assert_eq!(
        nitrogen
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        None
    );

    let mut engine = Engine::new();
    load_symbol_direction_document(
        &mut engine,
        json!([
            {"id": "n1", "element": "N", "atomicNumber": 7, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "c1", "element": "C", "atomicNumber": 6, "position": [70.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "c2", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0},
            {"id": "c3", "element": "C", "atomicNumber": 6, "position": [100.0, 130.0], "charge": 0, "numHydrogens": 0}
        ]),
        json!([
            {"id": "b1", "begin": "n1", "end": "c1", "order": 1},
            {"id": "b2", "begin": "n1", "end": "c2", "order": 1},
            {"id": "b3", "begin": "n1", "end": "c3", "order": 1}
        ]),
    );
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Minus,
        ..ToolState::default()
    });
    click(&mut engine, 100.0, 100.0);
    let nitrogen = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n1")
        .unwrap();
    assert_eq!(nitrogen.charge, -1);
    assert_eq!(
        nitrogen
            .meta
            .get("chargeSymbolInvalid")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[test]
fn third_period_hetero_charge_symbols_use_main_group_hydrogen_rule() {
    fn terminal_hetero_engine(
        element: &str,
        atomic_number: u8,
        symbol_kind: BracketKind,
    ) -> Engine {
        let mut engine = Engine::new();
        load_symbol_direction_document(
            &mut engine,
            json!([
                {"id": "n1", "element": element, "atomicNumber": atomic_number, "position": [100.0, 100.0], "charge": 0, "numHydrogens": 0},
                {"id": "c1", "element": "C", "atomicNumber": 6, "position": [130.0, 100.0], "charge": 0, "numHydrogens": 0}
            ]),
            json!([
                {"id": "b1", "begin": "n1", "end": "c1", "order": 1}
            ]),
        );
        engine.set_tool_state(ToolState {
            active_tool: Tool::Symbol,
            symbol_kind,
            ..ToolState::default()
        });
        click(&mut engine, 100.0, 100.0);
        engine
    }

    for (element, atomic_number, symbol_kind, expected_charge, expected_hydrogens) in [
        ("P", 15, BracketKind::Plus, 1, 1),
        ("P", 15, BracketKind::Minus, -1, 1),
        ("S", 16, BracketKind::Plus, 1, 0),
        ("S", 16, BracketKind::Minus, -1, 0),
    ] {
        let engine = terminal_hetero_engine(element, atomic_number, symbol_kind);
        let node = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == "n1")
            .unwrap();
        assert_eq!(
            node.charge, expected_charge,
            "{element} {symbol_kind:?} charge"
        );
        assert_eq!(
            node.num_hydrogens, expected_hydrogens,
            "{element} {symbol_kind:?} hydrogens"
        );
        assert_eq!(
            node.meta
                .get("chargeSymbolInvalid")
                .and_then(|value| value.as_bool()),
            None,
            "{element} {symbol_kind:?} should remain valid"
        );
    }
}

#[test]
fn symbol_tool_uses_current_document_symbol_line_width() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 120.0, 140.0);

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should be created");
    assert_eq!(symbol.payload.bbox, Some([0.0, 0.0, 7.18, 7.18]));
    assert_eq!(
        symbol
            .payload
            .extra
            .get("symbolStyle")
            .and_then(|value| value.as_str()),
        Some("acs")
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("symbolLineWidth")
            .and_then(|value| value.as_f64()),
        Some(0.6)
    );
    assert_eq!(
        symbol
            .payload
            .extra
            .get("symbolAnchorHeight")
            .and_then(|value| value.as_f64()),
        Some(7.5)
    );
}

#[test]
fn acs_circle_charge_symbol_uses_full_size_internal_sign() {
    let mut engine = Engine::new();
    engine.set_document_style_preset("acs-document-1996");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::CirclePlus,
        ..ToolState::default()
    });

    click(&mut engine, 120.0, 140.0);

    let symbol_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("symbol should be created")
        .id
        .clone();
    let sign_bounds = engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                object_id: Some(object_id),
                d,
                ..
            } if object_id == symbol_id => Some(rect_path_coordinate_bounds(&d)),
            _ => None,
        })
        .reduce(|left, right| {
            [
                left[0].min(right[0]),
                left[1].min(right[1]),
                left[2].max(right[2]),
                left[3].max(right[3]),
            ]
        })
        .expect("circle-plus sign should render as a filled path");

    assert_eq!(round_to_2(sign_bounds[2] - sign_bounds[0]), 3.93);
    assert_eq!(round_to_2(sign_bounds[3] - sign_bounds[1]), 3.93);
}

#[test]
fn bracket_tool_imports_chemdraw_charge_symbol_kinds() {
    let Some(cdxml) = read_optional_cdxml_fixture("kuohao-acs.cdxml") else {
        return;
    };
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("fixture should load");
    let kinds: Vec<_> = engine
        .state()
        .document
        .objects
        .iter()
        .filter(|object| object.object_type == "symbol")
        .filter_map(|object| {
            object
                .payload
                .extra
                .get("kind")
                .and_then(|value| value.as_str())
        })
        .collect();
    for kind in [
        "circle-plus",
        "plus",
        "radical-cation",
        "lone-pair",
        "circle-minus",
        "minus",
        "radical-anion",
        "electron",
    ] {
        assert!(kinds.contains(&kind), "missing imported symbol kind {kind}");
    }
    let plus = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some("plus")
        })
        .expect("ACS plus symbol should import");
    assert_eq!(plus.payload.bbox, Some([0.0, 0.0, 10.49, 10.49]));
    assert_eq!(
        plus.payload
            .extra
            .get("symbolStyle")
            .and_then(|value| value.as_str()),
        Some("acs")
    );
    let radical = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some("radical-cation")
        })
        .expect("ACS radical cation should import");
    assert_eq!(radical.payload.bbox, Some([0.0, 0.0, 8.8, 5.87]));
}

#[test]
fn bracket_symbol_drag_from_label_glyph_orbits_around_clicked_glyph() {
    let mut engine = Engine::new();
    let document = json!({
        "format": {"name": "chemcore", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_symbol_orbit",
            "title": "symbol orbit",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 40.0]}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        {
                            "id": "n1",
                            "element": "N",
                            "atomicNumber": 7,
                            "position": [30.0, 30.0],
                            "charge": 0,
                            "numHydrogens": 0,
                            "label": {
                                "text": "N",
                                "sourceText": "N",
                                "position": [30.0, 30.0],
                                "box": [26.0, 22.0, 34.0, 32.0],
                                "glyphPolygons": [[[26.0, 22.0], [34.0, 22.0], [34.0, 32.0], [26.0, 32.0]]]
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
        .expect("symbol orbit fixture should load");
    engine.set_tool_state(ToolState {
        active_tool: Tool::Symbol,
        symbol_kind: BracketKind::Plus,
        ..ToolState::default()
    });

    drag(&mut engine, Point::new(30.0, 27.0), Point::new(30.0, 14.0));

    let symbol = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "symbol")
        .expect("dragging from label glyph should create symbol");
    assert_eq!(
        symbol
            .payload
            .extra
            .get("kind")
            .and_then(|value| value.as_str()),
        Some("plus")
    );
    assert_eq!(round_to_2(symbol.transform.translate[0]), 27.83);
    assert_eq!(round_to_2(symbol.transform.translate[1]), 16.83);
}

#[test]
fn double_click_component_selection_includes_enclosing_bracket() {
    let mut engine = Engine::new();
    click(&mut engine, 300.0, 300.0);
    let (min_x, min_y, max_x, max_y, hit_point) = {
        let entry = engine.state().document.editable_fragment().unwrap();
        let points: Vec<Point> = entry
            .fragment
            .nodes
            .iter()
            .map(|node| entry.world_point_for_node(node))
            .collect();
        let min_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::NEG_INFINITY, f64::max);
        (min_x, min_y, max_x, max_y, points[0])
    };
    engine.set_tool_state(ToolState {
        active_tool: Tool::Bracket,
        bracket_kind: BracketKind::Round,
        ..ToolState::default()
    });
    drag(
        &mut engine,
        Point::new(min_x - 10.0, min_y - 10.0),
        Point::new(max_x + 10.0, max_y + 10.0),
    );
    let bracket_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "bracket")
        .map(|object| object.id.clone())
        .unwrap();

    assert!(engine.select_component_at_point(hit_point, false));
    assert!(engine.state().selection.arrow_objects.contains(&bracket_id));
    assert_eq!(engine.state().selection.nodes.len(), 2);
    assert_eq!(engine.state().selection.bonds.len(), 1);
}

#[test]
fn bracketed_multiple_group_stores_repeating_unit_expansion_when_legal() {
    let mut engine = Engine::new();
    let document = json!({
        "format": {"name": "chemcore", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_repeat",
            "title": "repeat",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 20.0]}
            },
            {
                "id": "obj_bracket_1",
                "type": "bracket",
                "visible": true,
                "locked": false,
                "zIndex": 20,
                "transform": {"translate": [15.0, -10.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "meta": {"linkedTextObjectId": "obj_text_1"},
                "payload": {"bbox": [0.0, 0.0, 30.0, 20.0], "kind": "square"}
            },
            {
                "id": "obj_text_1",
                "type": "text",
                "visible": true,
                "locked": false,
                "zIndex": 30,
                "transform": {"translate": [45.0, 5.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "meta": {"linkKind": "bracket-label", "linkedBracketObjectId": "obj_bracket_1"},
                "payload": {"bbox": [0.0, 0.0, 8.0, 10.0], "text": "3"}
            },
            {
                "id": "obj_symbol_1",
                "type": "symbol",
                "visible": true,
                "locked": false,
                "zIndex": 40,
                "transform": {"translate": [17.83325, -2.16675], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 4.3335, 4.3335], "kind": "plus", "fill": "#000000"}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, -10.0, 70.0, 20.0],
                    "nodes": [
                        {"id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n3", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 0.0], "charge": 0, "numHydrogens": 0}
                    ],
                    "bonds": [
                        {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
                        {"id": "b2", "begin": "n2", "end": "n3", "order": 1},
                        {"id": "b3", "begin": "n3", "end": "n4", "order": 1}
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("repeat fixture should load");

    assert!(engine.select_all());
    let summary: serde_json::Value =
        serde_json::from_str(&engine.selection_chemistry_summary_json()).unwrap();
    assert_eq!(summary["formula"], "C8H15");
    assert_eq!(summary["atomCount"], 23);
    assert!((summary["formulaWeight"].as_f64().unwrap() - 111.208).abs() < 1.0e-9);
    assert!((summary["exactMass"].as_f64().unwrap() - 111.117_375_483_45).abs() < 1.0e-9);

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("legal bracketed group should store repeatingUnits");
    assert_eq!(units.len(), 1);
    let unit = &units[0];
    assert_eq!(unit["bracketObjectId"], "obj_bracket_1");
    assert_eq!(unit["countTextObjectId"], "obj_text_1");
    assert_eq!(unit["repeatCount"]["value"], 3);
    assert_eq!(unit["atomIds"], json!(["n2", "n3"]));
    assert_eq!(unit["internalBondIds"], json!(["b2"]));

    let expansion = unit
        .get("expansion")
        .expect("legal bracketed group should include expansion");
    assert_eq!(expansion["schema"], "chemcore.repeatingUnitExpansion.v1");
    assert_eq!(expansion["complete"], true);
    assert_eq!(expansion["count"], 3);
    assert_eq!(expansion["atoms"].as_array().unwrap().len(), 6);
    assert_eq!(expansion["bonds"].as_array().unwrap().len(), 5);
    assert_eq!(expansion["attachments"][0]["atomId"], "n2_r1");
    assert_eq!(expansion["attachments"][1]["atomId"], "n3_r3");
    let first_atom = expansion["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|atom| atom["id"] == "n2_r1")
        .expect("expanded charged atom should exist");
    assert_eq!(first_atom["charge"], 1);
    assert_eq!(first_atom["numHydrogens"], 1);
    assert_eq!(first_atom["electronSymbols"][0]["kind"], "plus");
    assert_eq!(
        first_atom["electronSymbols"][0]["sourceSymbolObjectId"],
        "obj_symbol_1"
    );
}

#[test]
fn bracket_label_count_links_with_bracket_and_selects_with_component() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&repeat_unit_chain_document().to_string())
        .expect("repeat fixture should load");

    assert!(engine.apply_bracket_label_text("obj_bracket_1", bracket_label_session("3")));
    assert!(
        engine.state().selection.is_empty(),
        "committing a bracket label should not select it until the Select tool consumes the pending target"
    );
    assert!(engine
        .state()
        .document
        .objects
        .iter()
        .all(|object| object.object_type != "group"));
    let count_text = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "text")
        .expect("bracket label commit should create text");
    assert_eq!(
        count_text
            .meta
            .get("linkKind")
            .and_then(|value| value.as_str()),
        Some("bracket-label")
    );
    assert_eq!(
        count_text
            .meta
            .get("linkedBracketObjectId")
            .and_then(|value| value.as_str()),
        Some("obj_bracket_1")
    );
    let text_id = count_text.id.clone();
    let bracket = engine
        .state()
        .document
        .find_scene_object("obj_bracket_1")
        .expect("bracket should remain a top-level object");
    assert_eq!(
        bracket
            .meta
            .get("linkedTextObjectId")
            .and_then(|value| value.as_str()),
        Some(text_id.as_str())
    );

    assert!(engine.select_component_at_point(Point::new(20.0, 0.0), false));
    assert!(engine
        .state()
        .selection
        .arrow_objects
        .contains(&"obj_bracket_1".to_string()));
    assert!(engine.state().selection.text_objects.contains(&text_id));
    assert_eq!(engine.state().selection.nodes.len(), 4);
    assert_eq!(engine.state().selection.bonds.len(), 3);

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("numeric bracket label should create repeatingUnits");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["bracketObjectId"], "obj_bracket_1");
    assert!(units[0]["countTextObjectId"]
        .as_str()
        .is_some_and(|object_id| object_id.starts_with("obj_text")));
}

#[test]
fn unlinking_repeat_unit_link_detaches_count_label() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&repeat_unit_chain_document().to_string())
        .expect("repeat fixture should load");
    assert!(engine.apply_bracket_label_text("obj_bracket_1", bracket_label_session("3")));
    assert!(engine.select_component_at_point(Point::new(20.0, 0.0), false));
    assert!(engine.selection_can_unlink_bracket_text());

    assert!(engine.unlink_selection());
    assert!(!engine.selection_can_unlink_bracket_text());
    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    assert!(fragment.meta.get("repeatingUnits").is_none());
    let count_text = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object.object_type == "text")
        .expect("unlink should keep the count text object");
    assert!(count_text.meta.get("linkKind").is_none());
    assert!(count_text.meta.get("linkedBracketObjectId").is_none());
    let bracket = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object.id == "obj_bracket_1")
        .expect("unlink should keep the bracket object");
    assert!(bracket.meta.get("repeatUnitId").is_none());
    assert!(bracket.meta.get("linkedTextObjectId").is_none());
}

#[test]
fn editing_linked_repeat_count_refreshes_repeat_unit_semantics() {
    let mut engine = Engine::new();
    engine
        .load_document_json(&repeat_unit_chain_document().to_string())
        .expect("repeat fixture should load");
    assert!(engine.apply_bracket_label_text("obj_bracket_1", bracket_label_session("3")));
    let text_id = engine
        .state()
        .document
        .scene_objects()
        .into_iter()
        .find(|object| object.object_type == "text")
        .expect("linked count text should exist")
        .id
        .clone();

    let session = engine
        .begin_text_edit(Point::new(46.0, 6.0))
        .expect("linked count text should stay independently editable");
    match &session.target {
        TextEditTarget::TextObject { object_id, .. } => {
            assert_eq!(object_id.as_deref(), Some(text_id.as_str()));
        }
        _ => panic!("linked repeat count should edit as a text object"),
    }
    assert!(engine.apply_text_edit(TextEditSession {
        text: "4".to_string(),
        ..session
    }));

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .expect("fragment should still be editable")
        .fragment;
    let units = fragment
        .meta
        .get("repeatingUnits")
        .and_then(|value| value.as_array())
        .expect("linked count edit should refresh repeatingUnits");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["bracketObjectId"], "obj_bracket_1");
    assert_eq!(units[0]["countTextObjectId"], text_id);
    assert_eq!(units[0]["repeatCount"]["value"], 4);
}

fn bracket_label_session(text: &str) -> TextEditSession {
    TextEditSession {
        target: TextEditTarget::TextObject {
            object_id: None,
            x: 45.0,
            y: 5.0,
        },
        text: text.to_string(),
        source_runs: Vec::new(),
        font_family: Some("Arial".to_string()),
        font_size: Some(7.5),
        fill: Some("#000000".to_string()),
        align: Some("left".to_string()),
        line_height: Some(9.0),
        box_value: None,
        anchor_offset: None,
        preserve_lines: true,
        default_chemical: false,
    }
}

fn repeat_unit_chain_document() -> serde_json::Value {
    json!({
        "format": {"name": "chemcore", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_repeat_label",
            "title": "repeat label",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 20.0]}
            },
            {
                "id": "obj_bracket_1",
                "type": "bracket",
                "visible": true,
                "locked": false,
                "zIndex": 20,
                "transform": {"translate": [15.0, -10.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 30.0, 20.0], "kind": "square"}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, -10.0, 70.0, 20.0],
                    "nodes": [
                        {"id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n3", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 0.0], "charge": 0, "numHydrogens": 0}
                    ],
                    "bonds": [
                        {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
                        {"id": "b2", "begin": "n2", "end": "n3", "order": 1},
                        {"id": "b3", "begin": "n3", "end": "n4", "order": 1}
                    ]
                }
            }
        }
    })
}

#[test]
fn bracketed_group_without_numeric_count_does_not_store_expansion() {
    let mut engine = Engine::new();
    let document = json!({
        "format": {"name": "chemcore", "version": "0.1", "unit": "pt"},
        "document": {
            "id": "doc_repeat_invalid",
            "title": "repeat invalid",
            "page": {"width": 200.0, "height": 120.0, "background": "#ffffff"}
        },
        "styles": {},
        "objects": [
            {
                "id": "obj_mol_1",
                "type": "molecule",
                "visible": true,
                "locked": false,
                "zIndex": 10,
                "transform": {"translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"resourceRef": "mol_1", "bbox": [0.0, 0.0, 80.0, 20.0]}
            },
            {
                "id": "obj_bracket_1",
                "type": "bracket",
                "visible": true,
                "locked": false,
                "zIndex": 20,
                "transform": {"translate": [15.0, -10.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 30.0, 20.0], "kind": "square"}
            },
            {
                "id": "obj_text_1",
                "type": "text",
                "visible": true,
                "locked": false,
                "zIndex": 30,
                "transform": {"translate": [45.0, 5.0], "rotate": 0.0, "scale": [1.0, 1.0]},
                "payload": {"bbox": [0.0, 0.0, 8.0, 10.0], "text": "n"}
            }
        ],
        "resources": {
            "mol_1": {
                "type": "molecule_fragment2d",
                "encoding": "chemcore.molecule.fragment2d",
                "data": {
                    "schema": "chemcore.molecule.fragment2d",
                    "bbox": [0.0, -10.0, 70.0, 20.0],
                    "nodes": [
                        {"id": "n1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n3", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0},
                        {"id": "n4", "element": "C", "atomicNumber": 6, "position": [60.0, 0.0], "charge": 0, "numHydrogens": 0}
                    ],
                    "bonds": [
                        {"id": "b1", "begin": "n1", "end": "n2", "order": 1},
                        {"id": "b2", "begin": "n2", "end": "n3", "order": 1},
                        {"id": "b3", "begin": "n3", "end": "n4", "order": 1}
                    ]
                }
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("invalid repeat fixture should load");

    let fragment = engine
        .state()
        .document
        .editable_fragment()
        .unwrap()
        .fragment;
    assert!(fragment.meta.get("repeatingUnits").is_none());
}

#[test]
fn bond_tool_icons_are_rendered_with_kernel_bond_styles() {
    let dashed = Engine::bond_tool_icon_svg(BondVariant::Dashed, 1.32, 3.68);
    assert!(
        dashed.contains(r#"class="chemcore-icon cc-bond-icon""#),
        "{dashed}"
    );
    assert!(dashed.contains(r#"viewBox="0 0 24 24""#), "{dashed}");
    assert!(dashed.contains(r#"fill="currentColor""#), "{dashed}");
    assert_eq!(dashed.matches(r##"fill="#ffffff""##).count(), 2, "{dashed}");
    assert_eq!(dashed.matches("<line ").count(), 0, "{dashed}");

    let single = Engine::bond_tool_icon_svg(BondVariant::Single, 1.32, 3.68);
    assert!(single.contains(r#"fill="currentColor""#), "{single}");

    let bold = Engine::bond_tool_icon_svg(BondVariant::Bold, 1.32, 3.68);
    assert!(bold.contains(r#"fill="currentColor""#), "{bold}");

    let wavy = Engine::bond_tool_icon_svg(BondVariant::Wavy, 1.32, 3.68);
    assert_eq!(wavy.matches(" A ").count(), 8, "{wavy}");
    assert!(wavy.contains(r#"A 1.8750,1.8750"#), "{wavy}");
    assert!(wavy.contains(r#"stroke-width="1.32""#), "{wavy}");
}

#[test]
fn text_format_icons_are_rendered_with_kernel_text_runs() {
    let tool = Engine::text_format_icon_svg("tool");
    assert!(
        tool.contains(r#"class="chemcore-icon cc-tool-icon cc-text-tool-icon""#),
        "{tool}"
    );
    assert!(tool.contains(r#"font-family="Times New Roman""#), "{tool}");
    assert!(tool.contains(">A</tspan>"), "{tool}");

    let bold = Engine::text_format_icon_svg("bold");
    assert!(
        bold.contains(r#"class="chemcore-icon cc-text-format-icon""#),
        "{bold}"
    );
    assert!(bold.contains("<text "), "{bold}");
    assert!(bold.contains("<tspan"), "{bold}");
    assert!(bold.contains(r#"font-family="Times New Roman""#), "{bold}");
    assert!(bold.contains(r#"font-size="16""#), "{bold}");
    assert!(bold.contains(r#"font-weight="700""#), "{bold}");
    assert!(bold.contains(">B</tspan>"), "{bold}");

    let italic = Engine::text_format_icon_svg("italic");
    assert!(italic.contains(r#"font-style="italic""#), "{italic}");
    assert!(italic.contains(">I</tspan>"), "{italic}");

    let underline = Engine::text_format_icon_svg("underline");
    assert!(
        underline.contains(r#"text-decoration="underline""#),
        "{underline}"
    );
    assert!(underline.contains(">U</tspan>"), "{underline}");

    let chemical = Engine::text_format_icon_svg("chemical");
    assert!(
        chemical.contains(r#"class="chemcore-icon cc-text-format-icon cc-script-icon""#),
        "{chemical}"
    );
    assert!(chemical.contains(r#"font-size="14""#), "{chemical}");
    assert!(chemical.contains(">CH</tspan>"), "{chemical}");
    assert!(chemical.contains(">2</tspan>"), "{chemical}");
    assert!(chemical.contains("baseline-shift"), "{chemical}");

    let subscript = Engine::text_format_icon_svg("subscript");
    assert!(subscript.contains(">X</tspan>"), "{subscript}");
    assert!(subscript.contains(">2</tspan>"), "{subscript}");
    assert!(subscript.contains(r#"font-size="12""#), "{subscript}");
    assert!(subscript.contains("baseline-shift"), "{subscript}");

    let superscript = Engine::text_format_icon_svg("superscript");
    assert!(superscript.contains(">X</tspan>"), "{superscript}");
    assert!(superscript.contains(">2</tspan>"), "{superscript}");
    assert!(superscript.contains(r#"font-size="12""#), "{superscript}");
    assert!(superscript.contains("baseline-shift"), "{superscript}");
}

#[test]
fn shape_tool_icons_are_rendered_in_double_size_viewbox() {
    let styled_kinds = [
        ShapeKind::Circle,
        ShapeKind::Ellipse,
        ShapeKind::RoundRect,
        ShapeKind::Rect,
    ];
    let styles = [
        ShapeStyle::Solid,
        ShapeStyle::Dashed,
        ShapeStyle::Shaded,
        ShapeStyle::Filled,
        ShapeStyle::Shadowed,
    ];
    for kind in styled_kinds {
        for style in styles {
            let svg = Engine::shape_tool_icon_svg(kind, style);
            assert!(
                svg.contains(r#"viewBox="0 0 48 48""#),
                "{kind:?} {style:?}: {svg}"
            );
        }
    }

    let cross_table = Engine::shape_tool_icon_svg(ShapeKind::CrossTable, ShapeStyle::Solid);
    assert!(
        cross_table.contains(r#"viewBox="0 0 48 48""#),
        "{cross_table}"
    );

    let round_rect = Engine::shape_tool_icon_svg(ShapeKind::RoundRect, ShapeStyle::Solid);
    assert!(round_rect.contains("M 8.4,31.68"), "{round_rect}");
    assert!(round_rect.contains("39.6,16.08"), "{round_rect}");
    assert!(round_rect.contains(r#"stroke-width="2""#), "{round_rect}");

    assert!(
        cross_table.matches(r#"stroke-width="2""#).count() >= 3,
        "{cross_table}"
    );
}
