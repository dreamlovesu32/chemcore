use chemsema_engine::{
    angle_between, direction_from_angle, line_object_points, parse_document_json, ArrowCurve,
    ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowStylePatch, ArrowVariant, BondLinePattern,
    BondLineWeight, BondVariant, BracketKind, DoubleBondPlacement, Engine, Point, PointerEvent,
    RenderPrimitive, RenderRole, ShapeKind, ShapeStyle, TextEditSession, TextEditTarget, Tool,
    ToolState, ARROW_HIT_RADIUS, DEFAULT_BOND_LENGTH, DEFAULT_BOND_STROKE, GRAPHIC_EDGE_HIT_RADIUS,
};
use serde_json::json;
use std::collections::BTreeMap;

mod support;
mod bond_tool {
    use super::*;

    mod atom_semantics_icons;
    mod bond_drag_commands;
    mod bond_style_selection;
    mod brackets_templates;
    mod element_selection;
    mod endpoint_drag;
    mod history_properties;
    mod selection_objects;
    mod template_bonds;
}
use support::read_optional_cdxml_fixture;

const fn px(value: f64) -> f64 {
    chemsema_engine::px_to_pt(value)
}

fn px_point(x: f64, y: f64) -> chemsema_engine::Point {
    chemsema_engine::Point::new(px(x), px(y))
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

fn polygon_edge_lengths(points: &[Point]) -> Vec<f64> {
    let mut lengths = Vec::new();
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        lengths.push(points[index].distance(points[next]));
    }
    lengths
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

fn label_glyph_bounds(label: &chemsema_engine::NodeLabel) -> [f64; 4] {
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

fn label_clip_bounds(label: &chemsema_engine::NodeLabel) -> [f64; 4] {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for polygon in &label.glyph_clip_polygons {
        for [x, y] in polygon {
            bounds[0] = bounds[0].min(*x);
            bounds[1] = bounds[1].min(*y);
            bounds[2] = bounds[2].max(*x);
            bounds[3] = bounds[3].max(*y);
        }
    }
    bounds
}

fn label_glyph_box(label: &chemsema_engine::NodeLabel, index: usize) -> [f64; 4] {
    let polygon = label
        .glyph_polygons
        .get(index)
        .expect("label should contain the requested glyph");
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for [x, y] in polygon {
        bounds[0] = bounds[0].min(*x);
        bounds[1] = bounds[1].min(*y);
        bounds[2] = bounds[2].max(*x);
        bounds[3] = bounds[3].max(*y);
    }
    bounds
}

fn label_glyph_anchor(label: &chemsema_engine::NodeLabel, index: usize) -> Point {
    let bounds = label_glyph_box(label, index);
    Point::new(
        (bounds[0] + bounds[2]) * 0.5,
        fixture_label_anchor_y(label.position.expect("label position")[1]),
    )
}

fn label_glyph_hit_point(label: &chemsema_engine::NodeLabel, index: usize) -> Point {
    let bounds = label_glyph_box(label, index);
    Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
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

fn endpoint_from_anchor_toward(anchor: Point, target: Point) -> Point {
    endpoint_from_anchor(anchor, angle_between(anchor, target))
}

fn fixture_label_anchor_y(label_position_y: f64) -> f64 {
    label_position_y - chemsema_engine::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT * 0.39
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
    let unit = chemsema_engine::Vector::new((end.x - begin.x) / side, (end.y - begin.y) / side);
    let normal = chemsema_engine::Vector::new(-unit.y, unit.x).scaled(side_sign);
    let center = Point::new(
        (begin.x + end.x) * 0.5 + normal.x * apothem,
        (begin.y + end.y) * 0.5 + normal.y * apothem,
    );
    let first_vector = chemsema_engine::Vector::new(begin.x - center.x, begin.y - center.y);
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
    first_vector: chemsema_engine::Vector,
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

fn bond_center_point(engine: &Engine, bond_id: &str) -> chemsema_engine::Point {
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
    chemsema_engine::Point::new(
        (begin.position[0] + end.position[0]) * 0.5,
        (begin.position[1] + end.position[1]) * 0.5,
    )
}

fn bond_world_center_point(engine: &Engine, bond_id: &str) -> chemsema_engine::Point {
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
        .map(|node| entry.world_point_for_node(node))
        .expect("begin node should exist");
    let end = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .map(|node| entry.world_point_for_node(node))
        .expect("end node should exist");
    chemsema_engine::Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5)
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

fn selection_bond_dots(engine: &Engine) -> Vec<RenderPrimitive> {
    engine
        .render_list()
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Circle {
                    role: RenderRole::SelectionBondDot,
                    ..
                }
            )
        })
        .collect()
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
        | RenderPrimitive::Image { object_id, .. }
        | RenderPrimitive::Text { object_id, .. } => object_id.as_deref(),
    }
}

fn rendered_object_bounds(engine: &Engine, object_id: &str) -> [f64; 4] {
    let render_list = engine.render_list();
    chemsema_engine::render_primitives_bounds(
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

fn hover(engine: &mut Engine, x: f64, y: f64) {
    engine.pointer_move(PointerEvent {
        x,
        y,
        button: None,
        alt_key: false,
    });
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

fn load_label_document(
    engine: &mut Engine,
    label_text: &str,
    glyph_polygons: Vec<serde_json::Value>,
    bonds: serde_json::Value,
) {
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
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

fn load_two_molecule_document_with_duplicate_local_ids(engine: &mut Engine) {
    let fragment = |x1: f64, x2: f64| {
        json!({
            "schema": "chemsema.molecule.fragment2d",
            "bbox": [x1 - 5.0, 95.0, x2 + 5.0, 105.0],
            "nodes": [{
                "id": "n0",
                "element": "C",
                "atomicNumber": 6,
                "position": [x1, 100.0],
                "charge": 0,
                "numHydrogens": 0
            }, {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [x2, 100.0],
                "charge": 0,
                "numHydrogens": 0
            }],
            "bonds": [{ "id": "b1", "begin": "n0", "end": "n1", "order": 1 }]
        })
    };
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_multi",
            "title": "multi molecule clipboard test",
            "page": { "width": 360.0, "height": 180.0, "background": "#ffffff" }
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
            "id": "obj_molecule_a",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_a" }
        }, {
            "id": "obj_molecule_b",
            "type": "molecule",
            "visible": true,
            "zIndex": 11,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_b" }
        }],
        "resources": {
            "mol_a": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": fragment(80.0, 110.0)
            },
            "mol_b": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": fragment(220.0, 250.0)
            }
        }
    });
    engine
        .load_document_json(&document.to_string())
        .expect("document should load");
}

fn load_text_object_document(engine: &mut Engine) {
    let document = json!({
        "format": { "name": "chemsema", "version": "0.1" },
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
        "format": { "name": "chemsema", "version": "0.1" },
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

fn first_shape_object(engine: &Engine) -> &chemsema_engine::SceneObject {
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

fn load_symbol_direction_document(
    engine: &mut Engine,
    nodes: serde_json::Value,
    bonds: serde_json::Value,
) {
    let document = json!({
        "format": {"name": "chemsema", "version": "0.1", "unit": "pt"},
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
        text_position: None,
        glyph_polygons: Vec::new(),
        preserve_lines: true,
        default_chemical: false,
        display_mode: None,
    }
}

fn repeat_unit_chain_document() -> serde_json::Value {
    json!({
        "format": {"name": "chemsema", "version": "0.1", "unit": "pt"},
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
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
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
