use chemsema_engine::{
    angular_distance, document_to_cdxml, document_to_svg, hit_test_bond_center,
    parse_cdxml_document, parse_document_json, render_document, render_primitives_bounds,
    ChemSemaDocument, Engine, Point, RenderPrimitive, RenderRole, ResourceData, Tool, ToolState,
};
use serde_json::json;
use serde_json::Map;
use std::collections::BTreeSet;

mod support;
mod render_document {
    use super::*;

    mod atom_queries;
    mod bond_endpoints;
    mod colors_labels;
    mod crossings_selection;
    mod double_bond_text_import;
    mod external_connections;
    mod fragment_label_origin;
    mod glyph_retreat_stereo;
    mod junctions;
    mod query_double_bonds;
    mod shapes_bonds;
    mod text_layout;
}
use support::{cdxml_fixture_exists, read_cdxml_fixture, read_optional_cdxml_fixture};

const fn cdxml_cm_to_pt(value: f64) -> f64 {
    value * chemsema_engine::PT_PER_CM
}

const CDXML_EDIT_SCALE: f64 = 1.0;

fn assert_close(left: f64, right: f64) {
    assert!(
        (left - right).abs() < 1e-6,
        "expected {left:.6} to equal {right:.6}"
    );
}

fn assert_point_close(left: Point, right: Point) {
    assert_close(left.x, right.x);
    assert_close(left.y, right.y);
}

fn round_to_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn select_tool_state() -> ToolState {
    ToolState {
        active_tool: Tool::Select,
        ..ToolState::default()
    }
}

fn object_is_bracket_group(object: &chemsema_engine::SceneObject) -> bool {
    object.object_type == "group"
        && object.meta.get("kind").and_then(|value| value.as_str()) == Some("bracket-group")
}

fn fragment_document(nodes: serde_json::Value, bonds: serde_json::Value) -> ChemSemaDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 200.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
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
            "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 120.0, 120.0],
                    "nodes": nodes,
                    "bonds": bonds
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn normalize_test_document(document: &ChemSemaDocument) -> ChemSemaDocument {
    parse_document_json(&serde_json::to_string(document).expect("document should serialize"))
        .expect("document should normalize derived geometry")
}

fn fragment_document_preserving_disconnected_components(
    nodes: serde_json::Value,
    bonds: serde_json::Value,
) -> ChemSemaDocument {
    let mut document = fragment_document(nodes, bonds);
    document.objects[0].meta = json!({ "preserveDisconnectedComponents": true });
    document
}

fn grouped_two_fragment_document() -> ChemSemaDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1" },
        "document": {
            "id": "doc_test",
            "title": "test",
            "page": { "width": 300.0, "height": 200.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "obj_molecule_first",
            "type": "molecule",
            "visible": true,
            "zIndex": 10,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "styleRef": "style_molecule_default",
            "payload": { "resourceRef": "mol_first", "bbox": [0.0, 0.0, 40.0, 0.0] }
        }, {
            "id": "obj_group",
            "type": "group",
            "visible": true,
            "zIndex": 20,
            "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
            "payload": { "bbox": [100.0, 20.0, 40.0, 0.0] },
            "children": [{
                "id": "obj_molecule_grouped",
                "type": "molecule",
                "visible": true,
                "zIndex": 30,
                "transform": { "translate": [100.0, 20.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_grouped", "bbox": [0.0, 0.0, 40.0, 0.0] }
            }]
        }],
        "resources": {
            "mol_first": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 0.0],
                    "nodes": [
                        { "id": "n_first_1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n_first_2", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [{ "id": "b_first", "begin": "n_first_1", "end": "n_first_2", "order": 1 }]
                }
            },
            "mol_grouped": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 40.0, 0.0],
                    "nodes": [
                        { "id": "n_grouped_1", "element": "C", "atomicNumber": 6, "position": [0.0, 0.0], "charge": 0, "numHydrogens": 0 },
                        { "id": "n_grouped_2", "element": "C", "atomicNumber": 6, "position": [40.0, 0.0], "charge": 0, "numHydrogens": 0 }
                    ],
                    "bonds": [{ "id": "b_grouped", "begin": "n_grouped_1", "end": "n_grouped_2", "order": 1 }]
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn grouped_labeled_molecule_document() -> ChemSemaDocument {
    serde_json::from_value(json!({
        "format": { "name": "chemsema", "version": "0.1", "unit": "pt" },
        "document": {
            "id": "doc_group_labeled_molecule",
            "title": "group labeled molecule",
            "page": { "width": 220.0, "height": 160.0, "background": "#ffffff" }
        },
        "styles": {
            "style_molecule_default": {
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": 0.85,
                "fontFamily": "Arial",
                "fontSize": 11.0
            }
        },
        "objects": [{
            "id": "grp_1",
            "type": "group",
            "zIndex": 30,
            "children": [{
                "id": "obj_molecule_001",
                "type": "molecule",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [10.0, 10.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "styleRef": "style_molecule_default",
                "payload": { "resourceRef": "mol_001", "bbox": [0.0, 0.0, 80.0, 40.0] }
            }]
        }],
        "resources": {
            "mol_001": {
                "type": "molecule_fragment2d",
                "encoding": "chemsema.molecule.fragment2d",
                "data": {
                    "schema": "chemsema.molecule.fragment2d",
                    "bbox": [0.0, 0.0, 80.0, 40.0],
                    "nodes": [
                        { "id": "n1", "element": "C", "atomicNumber": 6, "position": [10.0, 20.0], "charge": 0, "numHydrogens": 0 },
                        {
                            "id": "n2",
                            "element": "O",
                            "atomicNumber": 8,
                            "position": [60.0, 20.0],
                            "charge": 0,
                            "numHydrogens": 0,
                            "label": {
                                "text": "O",
                                "sourceText": "O",
                                "position": [60.0, 20.0],
                                "box": [56.0, 12.0, 66.0, 24.0],
                                "fontSize": 11.0
                            }
                        }
                    ],
                    "bonds": [
                        { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
                    ]
                }
            }
        }
    }))
    .expect("document should deserialize")
}

fn primitive_polygon_bounds(points: &[Point]) -> [f64; 4] {
    points.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |mut bounds, point| {
            bounds[0] = bounds[0].min(point.x);
            bounds[1] = bounds[1].min(point.y);
            bounds[2] = bounds[2].max(point.x);
            bounds[3] = bounds[3].max(point.y);
            bounds
        },
    )
}

fn comma_point_token(token: &str) -> Option<Point> {
    let token = token.trim_end_matches([',', ';']);
    let (x, y) = token.split_once(',')?;
    Some(Point::new(x.parse().ok()?, y.parse().ok()?))
}

fn horizontal_path_span_at_y(d: &str, y: f64) -> Option<f64> {
    let xs = d
        .split_whitespace()
        .filter_map(comma_point_token)
        .filter(|point| (point.y - y).abs() < 0.01)
        .map(|point| point.x)
        .collect::<Vec<_>>();
    let min = xs.iter().copied().reduce(f64::min)?;
    let max = xs.iter().copied().reduce(f64::max)?;
    Some(max - min)
}

fn render_primitive_object_id(primitive: &RenderPrimitive) -> Option<&str> {
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

fn render_primitive_bond_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { bond_id, .. }
        | RenderPrimitive::Polygon { bond_id, .. }
        | RenderPrimitive::Polyline { bond_id, .. }
        | RenderPrimitive::Path { bond_id, .. }
        | RenderPrimitive::FilledPath { bond_id, .. } => bond_id.as_deref(),
        _ => None,
    }
}

fn render_primitive_text_content(primitive: &RenderPrimitive) -> Option<String> {
    match primitive {
        RenderPrimitive::Text { text, runs, .. } => {
            if runs.is_empty() {
                Some(text.clone())
            } else {
                Some(runs.iter().map(|run| run.text.as_str()).collect())
            }
        }
        _ => None,
    }
}

fn rects_with_role(engine: &Engine, role_filter: RenderRole) -> Vec<[f64; 4]> {
    engine
        .render_list()
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Rect {
                role,
                x,
                y,
                width,
                height,
                ..
            } if role == role_filter => Some([x, y, x + width, y + height]),
            _ => None,
        })
        .collect()
}

fn polygon_area(points: &[chemsema_engine::Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area.abs() * 0.5
}

fn centered_bond_polygons(
    primitives: &[RenderPrimitive],
    center: chemsema_engine::Point,
) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001")
                && points.iter().any(|point| point.distance(center) <= 4.0) =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_bond_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn object_knockout_polygons(primitives: &[RenderPrimitive]) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentKnockout
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn document_bond_polygon_count_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> usize {
    document_bond_polygons_for_object(primitives, object_id).len()
}

fn document_bond_polygons_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<Vec<chemsema_engine::Point>> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                Some(points.clone())
            }
            _ => None,
        })
        .collect()
}

fn document_knockout_count_for_object(primitives: &[RenderPrimitive], object_id: &str) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Polygon {
                    role,
                    object_id: primitive_object_id,
                    ..
                } if *role == RenderRole::DocumentKnockout
                    && primitive_object_id.as_deref() == Some(object_id)
            )
        })
        .count()
}

fn document_bond_axis_lengths_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<f64> {
    let mut lengths: Vec<_> = document_bond_polygons_for_object(primitives, object_id)
        .iter()
        .filter_map(|points| bond_axis_length(points))
        .collect();
    lengths.sort_by(f64::total_cmp);
    lengths
}

fn document_bond_axis_intervals_for_object(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<(f64, f64)> {
    let polygons = document_bond_polygons_for_object(primitives, object_id);
    let (axis_from, axis_to) = polygons
        .iter()
        .filter_map(|points| {
            let (axis_from, axis_to) = bond_axis_from_points(points)?;
            let length = axis_from.distance(axis_to);
            Some((length, axis_from, axis_to))
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, axis_from, axis_to)| (axis_from, axis_to))
        .expect("object should have a bond axis");
    let mut intervals: Vec<_> = polygons
        .iter()
        .filter_map(|points| projection_range_on_axis(points, axis_from, axis_to))
        .collect();
    let origin = intervals
        .iter()
        .map(|(start, _)| *start)
        .fold(f64::INFINITY, f64::min);
    for (start, end) in &mut intervals {
        *start -= origin;
        *end -= origin;
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    intervals
}

fn render_roundtrip_signature(document: &ChemSemaDocument) -> Vec<String> {
    let primitives: Vec<_> = render_document(document)
        .into_iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Line {
                    role: RenderRole::DocumentBond | RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Circle {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentText,
                    ..
                } | RenderPrimitive::Polygon {
                    role: RenderRole::DocumentBond
                        | RenderRole::DocumentGraphic
                        | RenderRole::DocumentKnockout,
                    ..
                } | RenderPrimitive::Rect {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentText,
                    ..
                } | RenderPrimitive::Ellipse {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Polyline {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentBond,
                    ..
                } | RenderPrimitive::Path {
                    role: RenderRole::DocumentGraphic | RenderRole::DocumentBond,
                    ..
                } | RenderPrimitive::FilledPath {
                    role: RenderRole::DocumentGraphic,
                    ..
                } | RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    ..
                }
            )
        })
        .collect();
    let (offset_x, offset_y) = render_signature_origin(&primitives);
    let mut signature: Vec<String> = primitives
        .iter()
        .map(|primitive| primitive_signature(primitive, offset_x, offset_y))
        .collect();
    signature.sort();
    signature
}

fn render_signature_origin(primitives: &[RenderPrimitive]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for primitive in primitives {
        for point in primitive_points(primitive) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
        }
    }
    if min_x.is_finite() && min_y.is_finite() {
        (min_x, min_y)
    } else {
        (0.0, 0.0)
    }
}

fn primitive_points(primitive: &RenderPrimitive) -> Vec<Point> {
    match primitive {
        RenderPrimitive::Line { from, to, .. } => vec![*from, *to],
        RenderPrimitive::Circle { center, radius, .. } => vec![
            Point::new(center.x - radius, center.y - radius),
            Point::new(center.x + radius, center.y + radius),
        ],
        RenderPrimitive::Polygon { points, .. }
        | RenderPrimitive::Polyline { points, .. }
        | RenderPrimitive::Path { points, .. }
        | RenderPrimitive::FilledPath { points, .. } => points.clone(),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            ..
        } => vec![Point::new(*x, *y), Point::new(*x + *width, *y + *height)],
        RenderPrimitive::Ellipse { center, rx, ry, .. } => vec![
            Point::new(center.x - rx, center.y - ry),
            Point::new(center.x + rx, center.y + ry),
        ],
        RenderPrimitive::Image {
            x,
            y,
            width,
            height,
            ..
        } => vec![Point::new(*x, *y), Point::new(*x + *width, *y + *height)],
        RenderPrimitive::Text { x, y, .. } => vec![Point::new(*x, *y)],
    }
}

fn primitive_signature(primitive: &RenderPrimitive, offset_x: f64, offset_y: f64) -> String {
    match primitive {
        RenderPrimitive::Line {
            role,
            from,
            to,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => format!(
            "line:{role:?}:{}:{}:{stroke}:{}:{:?}",
            point_sig(*from, offset_x, offset_y),
            point_sig(*to, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Circle {
            role,
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => format!(
            "circle:{role:?}:{}:{}:{fill}:{stroke}:{}",
            point_sig(*center, offset_x, offset_y),
            num_sig(*radius),
            num_sig(*stroke_width),
        ),
        RenderPrimitive::Polygon {
            role,
            points,
            fill,
            stroke,
            stroke_width,
            ..
        } => format!(
            "polygon:{role:?}:{}:{fill}:{stroke}:{}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
        ),
        RenderPrimitive::Rect {
            role,
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            rx,
            ry,
            dash_array,
            ..
        } => format!(
            "rect:{role:?}:{}:{}:{}:{}:{fill:?}:{stroke:?}:{}:{rx:?}:{ry:?}:{:?}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*width),
            num_sig(*height),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Ellipse {
            role,
            center,
            rx,
            ry,
            rotate,
            fill,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => format!(
            "ellipse:{role:?}:{}:{}:{}:{}:{fill:?}:{stroke:?}:{}:{:?}",
            point_sig(*center, offset_x, offset_y),
            num_sig(*rx),
            num_sig(*ry),
            num_sig(*rotate),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Polyline {
            role,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => format!(
            "polyline:{role:?}:{}:{stroke}:{}:{:?}:{line_cap:?}:{line_join:?}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::Path {
            role,
            d,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => format!(
            "path:{role:?}:{}:{d}:{stroke}:{}:{:?}:{line_cap:?}:{line_join:?}",
            points_sig(points, offset_x, offset_y),
            num_sig(*stroke_width),
            nums_sig(dash_array),
        ),
        RenderPrimitive::FilledPath {
            role,
            d,
            points,
            fill,
            fill_rule,
            ..
        } => format!(
            "filled-path:{role:?}:{}:{d}:{fill}:{fill_rule:?}",
            points_sig(points, offset_x, offset_y),
        ),
        RenderPrimitive::Image {
            role,
            x,
            y,
            width,
            height,
            opacity,
            preserve_aspect_ratio,
            rotate,
            ..
        } => format!(
            "image:{role:?}:{}:{}:{}:{}:{}:{preserve_aspect_ratio}:{}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*width),
            num_sig(*height),
            num_sig(*opacity),
            num_sig(*rotate),
        ),
        RenderPrimitive::Text {
            role,
            x,
            y,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            line_height,
            box_width,
            runs,
            ..
        } => format!(
            "text:{role:?}:{}:{}:{text}:{}:{font_family:?}:{fill:?}:{text_anchor:?}:{line_height:?}:{box_width:?}:{runs:?}",
            num_sig(*x - offset_x),
            num_sig(*y - offset_y),
            num_sig(*font_size),
        ),
    }
}

fn point_sig(point: Point, offset_x: f64, offset_y: f64) -> String {
    format!(
        "{} {}",
        num_sig(point.x - offset_x),
        num_sig(point.y - offset_y)
    )
}

fn points_sig(points: &[Point], offset_x: f64, offset_y: f64) -> String {
    points
        .iter()
        .map(|point| point_sig(*point, offset_x, offset_y))
        .collect::<Vec<_>>()
        .join(";")
}

fn nums_sig(values: &[f64]) -> Vec<String> {
    values.iter().map(|value| num_sig(*value)).collect()
}

fn num_sig(value: f64) -> String {
    format!("{:.2}", value)
}

fn object_knockout_rect_count(primitives: &[RenderPrimitive]) -> usize {
    primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                RenderPrimitive::Rect { role, object_id, .. }
                    if *role == RenderRole::DocumentKnockout
                        && object_id.as_deref() == Some("obj_molecule_001")
            )
        })
        .count()
}

fn object_bond_polygons_with_ids(
    primitives: &[RenderPrimitive],
) -> Vec<(String, Vec<chemsema_engine::Point>)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                bond_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((bond_id.clone().unwrap_or_default(), points.clone()))
            }
            _ => None,
        })
        .collect()
}

fn object_bond_points_for_id(
    primitives: &[RenderPrimitive],
    target_bond_id: &str,
) -> Vec<chemsema_engine::Point> {
    object_bond_polygons_with_ids(primitives)
        .into_iter()
        .filter(|(bond_id, _)| bond_id == target_bond_id)
        .flat_map(|(_, points)| points)
        .collect()
}

fn object_bond_centerlines_with_ids(
    primitives: &[RenderPrimitive],
    target_object_id: &str,
) -> Vec<(String, chemsema_engine::Point, chemsema_engine::Point)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                bond_id,
                from,
                to,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some(target_object_id) =>
            {
                Some((bond_id.clone().unwrap_or_default(), *from, *to))
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                bond_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some(target_object_id) =>
            {
                let (from, to) = bond_axis_from_points(points)?;
                Some((bond_id.clone().unwrap_or_default(), from, to))
            }
            _ => None,
        })
        .collect()
}

fn bond_axis_from_points(
    points: &[chemsema_engine::Point],
) -> Option<(chemsema_engine::Point, chemsema_engine::Point)> {
    if points.len() != 4 {
        return None;
    }
    Some((
        chemsema_engine::Point::new(
            (points[0].x + points[3].x) * 0.5,
            (points[0].y + points[3].y) * 0.5,
        ),
        chemsema_engine::Point::new(
            (points[1].x + points[2].x) * 0.5,
            (points[1].y + points[2].y) * 0.5,
        ),
    ))
}

fn bond_polygon_normal_widths(points: &[chemsema_engine::Point]) -> Option<(f64, f64)> {
    let (from, to) = bond_axis_from_points(points)?;
    let axis = chemsema_engine::Point::new(to.x - from.x, to.y - from.y);
    let length = (axis.x.powi(2) + axis.y.powi(2)).sqrt();
    if length <= 1.0e-9 {
        return None;
    }
    let normal = chemsema_engine::Point::new(-axis.y / length, axis.x / length);
    let start_width =
        ((points[0].x - points[3].x) * normal.x + (points[0].y - points[3].y) * normal.y).abs();
    let end_width =
        ((points[1].x - points[2].x) * normal.x + (points[1].y - points[2].y) * normal.y).abs();
    Some((start_width, end_width))
}

fn bond_axis_length(points: &[chemsema_engine::Point]) -> Option<f64> {
    let (from, to) = bond_axis_from_points(points)?;
    Some(from.distance(to))
}

fn cdxml_shape_fills_by_z(document: &ChemSemaDocument) -> Vec<String> {
    let mut shapes: Vec<_> = document
        .objects
        .iter()
        .filter(|object| object.object_type == "shape")
        .collect();
    shapes.sort_by_key(|object| object.z_index);
    shapes
        .into_iter()
        .filter_map(|object| {
            let style_ref = object.style_ref.as_ref()?;
            document.styles.get(style_ref)?.get("fill")?.as_str()
        })
        .map(ToString::to_string)
        .collect()
}

fn normalize_svg_snapshot(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n");
    format!("{}\n", normalized.trim_end())
}

#[test]
fn parse_cdxml_preserves_chemdraw_js_cached_enhanced_stereo_position() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemDraw JS 2.0.0.9" BondLength="14.4" LineWidth="0.6">
  <page id="1"><fragment id="2">
    <n id="3" p="20 20" EnhancedStereoType="Absolute">
      <objecttag Name="enhancedstereo">
        <t p="23 18" BoundingBox="23 12 35 18"><s font="3" size="7.5">abs</s></t>
      </objecttag>
    </n>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("ChemDraw JS enhanced stereo"))
        .expect("ChemDraw JS enhanced-stereo tag should parse");
    let label = document
        .objects
        .iter()
        .find(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) == Some("enhanced_stereo")
        })
        .expect("enhanced-stereo label should import");
    assert_eq!(label.transform.translate, [23.0, 12.0]);
    assert_eq!(
        label
            .payload
            .extra
            .get("baselineOffset")
            .and_then(|value| value.as_f64()),
        Some(6.0)
    );
    let tag = document
        .logical_objects
        .object_tags
        .iter()
        .find(|tag| tag.name == "enhancedstereo")
        .expect("enhanced-stereo object tag should import");
    assert_eq!(
        tag.positioning_type,
        chemsema_engine::AnnotationPositioningType::Absolute
    );
    let exported = chemsema_engine::document_to_cdxml(&document);
    assert!(exported.contains("PositioningType=\"absolute\""));
    let reopened = parse_cdxml_document(&exported, Some("ChemDraw JS enhanced stereo roundtrip"))
        .expect("exported enhanced-stereo tag should parse");
    let reopened_label = reopened
        .objects
        .iter()
        .find(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) == Some("enhanced_stereo")
        })
        .expect("enhanced-stereo label should survive");
    assert_eq!(reopened_label.transform.translate, [23.0, 12.0]);
}

fn assert_symbol_center(document: &ChemSemaDocument, kind: &str, expected: [f64; 2]) {
    let symbol = document
        .objects
        .iter()
        .find(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(|value| value.as_str())
                    == Some(kind)
        })
        .unwrap_or_else(|| panic!("{kind} symbol should import"));
    let [_, _, width, height] = symbol.payload.bbox.expect("symbol should have bbox");
    let center = [
        symbol.transform.translate[0] + width * 0.5,
        symbol.transform.translate[1] + height * 0.5,
    ];
    assert!(
        (center[0] - expected[0]).abs() < 0.01,
        "{kind} center x should use first CDXML bbox point, got {center:?}"
    );
    assert!(
        (center[1] - expected[1]).abs() < 0.01,
        "{kind} center y should use first CDXML bbox point, got {center:?}"
    );
}

fn imported_fragment_bond<'a>(
    document: &'a ChemSemaDocument,
    object_id: &str,
    bond_id: &str,
) -> &'a chemsema_engine::Bond {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    fragment
        .bonds
        .iter()
        .find(|bond| bond.id == bond_id)
        .expect("bond should exist")
}

fn imported_double_bond_center_spacing(document: &ChemSemaDocument, object_id: &str) -> f64 {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    let bond = fragment
        .bonds
        .first()
        .expect("fixture fragment has one bond");
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    let begin = chemsema_engine::Point::new(
        object.transform.translate[0] + begin.position[0],
        object.transform.translate[1] + begin.position[1],
    );
    let end = chemsema_engine::Point::new(
        object.transform.translate[0] + end.position[0],
        object.transform.translate[1] + end.position[1],
    );
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy);
    let normal = chemsema_engine::Point::new(-dy / length, dx / length);
    let mut projections: Vec<f64> = render_document(document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let projection = points
                    .iter()
                    .map(|point| point.x * normal.x + point.y * normal.y)
                    .sum::<f64>()
                    / points.len() as f64;
                Some(projection)
            }
            _ => None,
        })
        .collect();
    projections.sort_by(f64::total_cmp);
    assert!(
        projections.len() >= 2,
        "{object_id}: expected at least two rendered bond line polygons"
    );
    let split = projections
        .windows(2)
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            (left[1] - left[0])
                .abs()
                .total_cmp(&(right[1] - right[0]).abs())
        })
        .map(|(index, _)| index + 1)
        .expect("projection split should exist");
    let first = projections[..split].iter().sum::<f64>() / split as f64;
    let second = projections[split..].iter().sum::<f64>() / (projections.len() - split) as f64;
    (second - first).abs()
}

fn imported_double_bond_formula_spacing(document: &ChemSemaDocument, object_id: &str) -> f64 {
    let object = document
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .expect("imported molecule object should exist");
    let resource_ref = object
        .payload
        .resource_ref
        .as_deref()
        .expect("molecule object should reference fragment resource");
    let fragment = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
        .expect("fragment resource should exist");
    let bond = fragment
        .bonds
        .first()
        .expect("fixture fragment has one bond");
    let begin = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .expect("begin node should exist");
    let end = fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .expect("end node should exist");
    let length = chemsema_engine::Point::new(begin.position[0], begin.position[1]).distance(
        chemsema_engine::Point::new(end.position[0], end.position[1]),
    );
    if let Some(spacing) = bond.bond_spacing_absolute {
        return spacing;
    }
    let ratio = bond
        .bond_spacing
        .expect("cdxml fixture should import bond spacing")
        / 100.0;
    let stroke_width = bond.stroke_width;
    let line_width = |weight| {
        if weight == chemsema_engine::BondLineWeight::Bold {
            bond.bold_width.unwrap_or(stroke_width).max(stroke_width)
        } else {
            stroke_width
        }
    };
    let first_width = line_width(bond.line_weights.left);
    let second_width = line_width(bond.line_weights.right);
    (length * ratio - stroke_width).max(stroke_width * 1.5) + 0.5 * (first_width + second_width)
}

fn imported_vertical_line_metrics(
    primitives: &[RenderPrimitive],
    object_id: &str,
) -> Vec<(f64, f64)> {
    let mut metrics: Vec<(f64, f64)> = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id: primitive_object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && primitive_object_id.as_deref() == Some(object_id) =>
            {
                let (from, to) = bond_axis_from_points(points)?;
                let center_x = (from.x + to.x) * 0.5;
                let min_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min);
                let max_x = points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max);
                Some((center_x, max_x - min_x))
            }
            _ => None,
        })
        .collect();
    metrics.sort_by(|a, b| a.0.total_cmp(&b.0));
    metrics
}

fn assert_line_spacing(metrics: &[(f64, f64)], expected: f64, context: &str) {
    assert_eq!(metrics.len(), 2, "{context}: {metrics:?}");
    let actual = metrics[1].0 - metrics[0].0;
    assert!(
        (actual - expected).abs() < 0.001,
        "{context}: expected {expected}, got {actual}, metrics={metrics:?}"
    );
}

fn assert_adjacent_line_spacing(metrics: &[(f64, f64)], expected: f64, context: &str) {
    assert_eq!(metrics.len(), 3, "{context}: {metrics:?}");
    for pair in metrics.windows(2) {
        let actual = pair[1].0 - pair[0].0;
        assert!(
            (actual - expected).abs() < 0.001,
            "{context}: expected {expected}, got {actual}, metrics={metrics:?}"
        );
    }
}

fn assert_line_widths(
    metrics: &[(f64, f64)],
    expected_left: f64,
    expected_right: f64,
    context: &str,
) {
    assert_eq!(metrics.len(), 2, "{context}: {metrics:?}");
    assert!(
        (metrics[0].1 - expected_left).abs() < 0.001,
        "{context}: expected left width {expected_left}, got {}, metrics={metrics:?}",
        metrics[0].1
    );
    assert!(
        (metrics[1].1 - expected_right).abs() < 0.001,
        "{context}: expected right width {expected_right}, got {}, metrics={metrics:?}",
        metrics[1].1
    );
}

fn projection_range_on_axis(
    points: &[chemsema_engine::Point],
    axis_from: chemsema_engine::Point,
    axis_to: chemsema_engine::Point,
) -> Option<(f64, f64)> {
    let axis = chemsema_engine::Point::new(axis_to.x - axis_from.x, axis_to.y - axis_from.y);
    let length = (axis.x * axis.x + axis.y * axis.y).sqrt();
    if length <= 1.0e-6 {
        return None;
    }
    let unit_x = axis.x / length;
    let unit_y = axis.y / length;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for point in points {
        let projection = (point.x - axis_from.x) * unit_x + (point.y - axis_from.y) * unit_y;
        min_value = min_value.min(projection);
        max_value = max_value.max(projection);
    }
    Some((min_value, max_value))
}

fn object_bond_centerlines(
    primitives: &[RenderPrimitive],
) -> Vec<(chemsema_engine::Point, chemsema_engine::Point)> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                role,
                object_id,
                from,
                to,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some((*from, *to))
            }
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            }
            | RenderPrimitive::FilledPath {
                role,
                object_id,
                points,
                ..
            } if *role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                bond_axis_from_points(points)
            }
            _ => None,
        })
        .collect()
}

fn shared_point_count(
    first: &[chemsema_engine::Point],
    second: &[chemsema_engine::Point],
    tolerance: f64,
) -> usize {
    first
        .iter()
        .filter(|point| {
            second
                .iter()
                .any(|other| point.distance(*other) <= tolerance)
        })
        .count()
}

fn point_lies_on_segment(
    point: chemsema_engine::Point,
    from: chemsema_engine::Point,
    to: chemsema_engine::Point,
    tolerance: f64,
) -> bool {
    let cross = (point.x - from.x) * (to.y - from.y) - (point.y - from.y) * (to.x - from.x);
    if cross.abs() > tolerance {
        return false;
    }
    let dot = (point.x - from.x) * (to.x - from.x) + (point.y - from.y) * (to.y - from.y);
    if dot < -tolerance {
        return false;
    }
    let length_squared = (to.x - from.x).powi(2) + (to.y - from.y).powi(2);
    dot <= length_squared + tolerance
}

fn polygons_have_same_vertices(
    first: &[chemsema_engine::Point],
    second: &[chemsema_engine::Point],
    tolerance: f64,
) -> bool {
    first.len() == second.len()
        && first.iter().all(|point| {
            second
                .iter()
                .any(|other| point.distance(*other) <= tolerance)
        })
}

fn point_lies_on_polygon_boundary(
    point: chemsema_engine::Point,
    polygon: &[chemsema_engine::Point],
    tolerance: f64,
) -> bool {
    if polygon.len() < 2 {
        return false;
    }
    (0..polygon.len()).any(|index| {
        let next = (index + 1) % polygon.len();
        point_lies_on_segment(point, polygon[index], polygon[next], tolerance)
    })
}

fn average_closest_distance_to_point(
    points: &[chemsema_engine::Point],
    target: chemsema_engine::Point,
    count: usize,
) -> f64 {
    let mut distances: Vec<_> = points.iter().map(|point| point.distance(target)).collect();
    distances.sort_by(|a, b| a.total_cmp(b));
    distances.into_iter().take(count).sum::<f64>() / count as f64
}

fn side_double_outer_polygon_for_bond(
    polygons: &[(String, Vec<chemsema_engine::Point>)],
    bond_id: &str,
    shared_node: chemsema_engine::Point,
) -> Vec<chemsema_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .max_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2)
                .total_cmp(&average_closest_distance_to_point(&b.1, shared_node, 2))
        })
        .map(|(_, points)| points.clone())
        .expect("side double outer polygon")
}

fn side_double_main_polygon_for_bond(
    polygons: &[(String, Vec<chemsema_engine::Point>)],
    bond_id: &str,
    shared_node: chemsema_engine::Point,
) -> Vec<chemsema_engine::Point> {
    polygons
        .iter()
        .filter(|(current_bond_id, _)| current_bond_id == bond_id)
        .min_by(|a, b| {
            average_closest_distance_to_point(&a.1, shared_node, 2)
                .total_cmp(&average_closest_distance_to_point(&b.1, shared_node, 2))
        })
        .map(|(_, points)| points.clone())
        .expect("side double main polygon")
}

fn closest_points_to_target(
    points: &[chemsema_engine::Point],
    target: chemsema_engine::Point,
    count: usize,
) -> Vec<chemsema_engine::Point> {
    let mut indexed: Vec<_> = points.to_vec();
    indexed.sort_by(|a, b| a.distance(target).total_cmp(&b.distance(target)));
    indexed.into_iter().take(count).collect()
}

#[test]
#[ignore = "backend debug output for bond vertices"]
fn debug_print_single_and_bold_bond_vertices() {
    let single_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let bold_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let single_polygons = object_bond_polygons(&render_document(&single_document));
    let bold_polygons = object_bond_polygons(&render_document(&bold_document));

    let single = single_polygons
        .iter()
        .find(|points| points.len() == 4)
        .expect("single bond should render as one 4-point polygon");
    let bold = bold_polygons
        .iter()
        .find(|points| points.len() == 4)
        .expect("bold bond should render as one 4-point polygon");

    println!("single bond vertices:");
    for (index, point) in single.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("bold bond vertices:");
    for (index, point) in bold.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }
}

#[test]
#[ignore = "backend debug output for joined bond vertices"]
fn debug_print_joined_single_and_bold_bond_vertices() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygons = object_bond_polygons(&render_document(&document));
    assert_eq!(
        polygons.len(),
        2,
        "joined single+bold should render as two 4-point polygons"
    );

    let first = polygons
        .iter()
        .find(|points| points.iter().any(|point| (point.x - 20.0).abs() < 0.001))
        .expect("first bond polygon");
    let second = polygons
        .iter()
        .find(|points| points.iter().any(|point| (point.x - 74.0).abs() < 1.0))
        .expect("second bond polygon");

    let mut shared = Vec::new();
    for first_point in first {
        for second_point in second {
            if first_point.distance(*second_point) <= 1.0e-6 {
                shared.push(*first_point);
            }
        }
    }

    println!("joined bold bond vertices:");
    for (index, point) in first.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("joined single bond vertices:");
    for (index, point) in second.iter().enumerate() {
        println!("  p{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }

    println!("shared vertices:");
    for (index, point) in shared.iter().enumerate() {
        println!("  s{} = ({:.6}, {:.6})", index + 1, point.x, point.y);
    }
}

fn right_arrow_head_width_from_cdxml(cdxml: &str) -> f64 {
    let document =
        parse_cdxml_document(cdxml, Some("equilibrium arrow")).expect("CDXML arrow should parse");
    render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::FilledPath { points, .. } => {
                let bounds = primitive_polygon_bounds(&points);
                Some((bounds[2], bounds[2] - bounds[0]))
            }
            _ => None,
        })
        .max_by(|left, right| left.0.partial_cmp(&right.0).unwrap())
        .map(|(_, width)| (width * 100.0).round() / 100.0)
        .expect("arrow should render a filled head")
}

fn rounded_pair(points: &[Point]) -> ([f64; 2], [f64; 2]) {
    fn round2(value: f64) -> f64 {
        (value * 100.0).round() / 100.0
    }
    (
        [round2(points[0].x), round2(points[0].y)],
        [round2(points[1].x), round2(points[1].y)],
    )
}
