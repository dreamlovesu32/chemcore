use crate::{
    legacy_mol::{parse_molblock, LegacyAtom, LegacyBond as LegacyMolBond, LegacyMol},
    px_to_cm, Bond, BondLinePattern, BondLineWeight, ChemcoreDocument, DoubleBondPlacement,
    LabelRun, MoleculeFragment, Node, ObjectPayload, Point, ResourceData, SceneObject, Vector,
    DEFAULT_BOND_STROKE, EPSILON,
};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};

#[path = "render_bonds.rs"]
mod bond_render;
#[path = "render_contact.rs"]
mod contact;
#[path = "render_legacy.rs"]
mod legacy_render;
#[path = "render_objects.rs"]
mod object_render;
#[path = "render_primitives.rs"]
mod primitives;

use bond_render::{compute_solid_wedge_points, render_fragment_bond};
use contact::{
    bond_ray_is_acute, build_main_bond_contact_kernel, center_double_skips_extension,
    main_bond_endpoint_geometry, main_contact_is_straight_through, main_contact_side,
    render_main_bond_contact_patches, MainBondContactKernel,
};
use legacy_render::render_legacy_molecule_object;
use object_render::{
    render_line_object, render_molecule_object, render_shape_object, render_text_object,
};
use primitives::{
    push_bond_line, push_bond_polygon, push_knockout_polygon, push_line, push_polygon,
    push_polyline, push_text,
};
pub use primitives::{RenderPrimitive, RenderRole};

const VIEWER_BOND_STROKE: f64 = crate::VIEWER_BOND_STROKE_CM.value();
const MULTI_BOND_OFFSET_RATIO: f64 = 0.12;
const DOUBLE_BOND_SIDE_INSET: f64 = crate::DOUBLE_BOND_SIDE_INSET_CM.value();
const DOUBLE_BOND_SIDE_INSET_RATIO: f64 = 0.14;
const HASH_WEDGE_SPACING: f64 = crate::HASH_WEDGE_SPACING_CM.value();
const HASH_WEDGE_START_OFFSET: f64 = crate::HASH_WEDGE_START_OFFSET_CM.value();
const HASH_WEDGE_END_INSET: f64 = crate::HASH_WEDGE_END_INSET_CM.value();
const HASH_BLACK_SEGMENT_LENGTH: f64 = crate::HASH_BLACK_SEGMENT_LENGTH_CM.value();
const HASH_TARGET_GAP_LENGTH: f64 = crate::HASH_TARGET_GAP_LENGTH_CM.value();
const HASH_WEDGE_EDGE_OVERDRAW: f64 = crate::HASH_WEDGE_EDGE_OVERDRAW_CM.value();
const HASH_MULTI_BOND_RETREAT_GAP: f64 = crate::HASH_MULTI_BOND_RETREAT_GAP_CM.value();
const SOLID_WEDGE_END_INSET: f64 = crate::SOLID_WEDGE_END_INSET_CM.value();
const SOLID_WEDGE_HALF_WIDTH: f64 = crate::SOLID_WEDGE_HALF_WIDTH_CM.value();
const CENTER_DOUBLE_NO_EXTENSION_ANGLE_DEGREES: f64 = 162.0;
const CHEMCORE_INK: &str = "#000000";
const KNOCKOUT_FILL: &str = "#ffffff";
const DASHED_BOND_PATTERN: [f64; 2] = [
    crate::DASHED_BOND_PATTERN_CM[0].value(),
    crate::DASHED_BOND_PATTERN_CM[1].value(),
];
const BOLD_BOND_WIDTH: f64 = crate::BOLD_BOND_WIDTH_CM.value();
const MAIN_CONTACT_MITER_LIMIT: f64 = 4.0;

#[derive(Debug, Clone, Copy)]
struct RectBox {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl RectBox {
    fn expanded(self, margin: f64) -> Self {
        Self {
            x1: self.x1 - margin,
            y1: self.y1 - margin,
            x2: self.x2 + margin,
            y2: self.y2 + margin,
        }
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.x1 && point.x <= self.x2 && point.y >= self.y1 && point.y <= self.y2
    }
}

#[derive(Debug, Clone, Copy)]
struct LineGeometry {
    point: Point,
    direction: Vector,
    shared: Point,
    length: f64,
    offset_distance: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct ArrowHeadGeometry {
    length: f64,
    width: f64,
    head_full: bool,
}

pub fn render_document(document: &ChemcoreDocument) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    let mut objects: Vec<&SceneObject> = document
        .objects
        .iter()
        .filter(|object| object.visible)
        .collect();
    objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));

    for object in objects {
        match object.object_type.as_str() {
            "molecule" => render_molecule_object(&mut out, document, object),
            "line" => render_line_object(&mut out, document, object),
            "text" => render_text_object(&mut out, document, object),
            "shape" => render_shape_object(&mut out, document, object),
            _ => {}
        }
    }

    out
}

pub(crate) fn fragment_bond_visual_bounds(
    document: &ChemcoreDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    bond: &Bond,
) -> Option<[f64; 4]> {
    let node_map: BTreeMap<&str, &Node> = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let contact_kernel =
        build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);
    let mut out = Vec::new();
    render_fragment_bond(
        &mut out,
        document,
        object,
        &contact_kernel,
        &fragment.bonds,
        &node_map,
        bond,
        &molecule_stroke(document, object),
        None,
    );

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    for primitive in out {
        if !primitive_matches_bond(&primitive, &bond.id) {
            continue;
        }
        let Some([x1, y1, x2, y2]) = render_primitive_bounds(&primitive) else {
            continue;
        };
        min_x = min_x.min(x1);
        min_y = min_y.min(y1);
        max_x = max_x.max(x2);
        max_y = max_y.max(y2);
        found = true;
    }

    found.then_some([min_x, min_y, max_x, max_y])
}

fn primitive_matches_bond(primitive: &RenderPrimitive, bond_id: &str) -> bool {
    match primitive {
        RenderPrimitive::Line {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Polygon {
            bond_id: Some(current),
            ..
        }
        | RenderPrimitive::Polyline {
            bond_id: Some(current),
            ..
        } => current == bond_id,
        _ => false,
    }
}

fn render_primitive_bounds(primitive: &RenderPrimitive) -> Option<[f64; 4]> {
    match primitive {
        RenderPrimitive::Line {
            from,
            to,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                from.x.min(to.x) - half_width,
                from.y.min(to.y) - half_width,
                from.x.max(to.x) + half_width,
                from.y.max(to.y) + half_width,
            ])
        }
        RenderPrimitive::Polygon {
            points,
            stroke_width,
            ..
        }
        | RenderPrimitive::Polyline {
            points,
            stroke_width,
            ..
        } => point_list_bounds(points, *stroke_width * 0.5),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            stroke_width,
            ..
        } => {
            let half_width = stroke_width * 0.5;
            Some([
                *x - half_width,
                *y - half_width,
                *x + *width + half_width,
                *y + *height + half_width,
            ])
        }
        RenderPrimitive::Circle { center, radius, .. } => Some([
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        ]),
        RenderPrimitive::Text { .. } => None,
    }
}

fn point_list_bounds(points: &[Point], margin: f64) -> Option<[f64; 4]> {
    let mut iter = points.iter().copied();
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in iter {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([
        min_x - margin,
        min_y - margin,
        max_x + margin,
        max_y + margin,
    ])
}

fn endpoint_profile_global(
    profile: Option<Vec<Point>>,
    reverse: bool,
    default_profile: Vec<Point>,
) -> Vec<Point> {
    let points = if let Some(mut profile) = profile {
        if reverse {
            profile.reverse();
        }
        profile
    } else {
        default_profile
    };
    compact_polygon_points(points)
}

fn bond_polygon_from_endpoint_profiles(
    start_profile: Vec<Point>,
    end_profile: Vec<Point>,
) -> Vec<Point> {
    let mut points = Vec::with_capacity(start_profile.len() + end_profile.len());
    if let Some(first) = start_profile.first().copied() {
        points.push(first);
    }
    for point in end_profile {
        if points
            .last()
            .is_some_and(|last| last.distance(point) <= 1.0e-6)
        {
            continue;
        }
        points.push(point);
    }
    let mut start_tail: Vec<Point> = start_profile.into_iter().skip(1).collect();
    start_tail.reverse();
    for point in start_tail {
        if points
            .last()
            .is_some_and(|last| last.distance(point) <= 1.0e-6)
        {
            continue;
        }
        points.push(point);
    }
    compact_polygon_points(points)
}

fn midpoint(first: Point, second: Point) -> Point {
    Point::new((first.x + second.x) * 0.5, (first.y + second.y) * 0.5)
}

fn compact_polygon_points(points: Vec<Point>) -> Vec<Point> {
    let mut out = Vec::new();
    for point in points {
        if out
            .last()
            .is_some_and(|last: &Point| last.distance(point) <= 1.0e-6)
        {
            continue;
        }
        out.push(point);
    }
    if out.len() >= 2
        && out
            .first()
            .zip(out.last())
            .is_some_and(|(first, last)| first.distance(*last) <= 1.0e-6)
    {
        out.pop();
    }
    out
}

fn polygon_area_signed(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area * 0.5
}

fn axis_angle(axis: Vector) -> f64 {
    axis.y.atan2(axis.x)
}

fn vector_dot(first: Vector, second: Vector) -> f64 {
    first.x * second.x + first.y * second.y
}

fn vector_cross(first: Vector, second: Vector) -> f64 {
    first.x * second.y - first.y * second.x
}

fn compute_bold_bond_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    allow_start_contacts: bool,
    allow_end_contacts: bool,
    start_endpoint_profile: Option<Vec<Point>>,
    end_endpoint_profile: Option<Vec<Point>>,
) -> Vec<Point> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width = line_weight_stroke_width(stroke_width, BondLineWeight::Bold) / 2.0;
    let start_profile = if let Some(profile) = start_endpoint_profile {
        endpoint_profile_global(Some(profile), false, Vec::new())
    } else if allow_start_contacts {
        let (start_plus, start_minus) = bold_band_cap_points(
            object,
            bonds,
            node_map,
            bond,
            &bond.begin,
            start,
            unit,
            normal,
            half_width,
            end,
            stroke_width,
        );
        vec![start_plus, start_minus]
    } else {
        vec![
            Point::new(
                start.x + normal.x * half_width,
                start.y + normal.y * half_width,
            ),
            Point::new(
                start.x - normal.x * half_width,
                start.y - normal.y * half_width,
            ),
        ]
    };
    let end_profile = if let Some(profile) = end_endpoint_profile {
        endpoint_profile_global(Some(profile), true, Vec::new())
    } else if allow_end_contacts {
        let (end_plus, end_minus) = bold_band_cap_points(
            object,
            bonds,
            node_map,
            bond,
            &bond.end,
            end,
            Vector::new(-unit.x, -unit.y),
            normal,
            half_width,
            start,
            stroke_width,
        );
        vec![end_plus, end_minus]
    } else {
        vec![
            Point::new(end.x + normal.x * half_width, end.y + normal.y * half_width),
            Point::new(end.x - normal.x * half_width, end.y - normal.y * half_width),
        ]
    };
    if length <= 1.0e-6 {
        return bond_polygon_from_endpoint_profiles(start_profile, end_profile);
    }
    bond_polygon_from_endpoint_profiles(start_profile, end_profile)
}

fn is_hash_bond(bond: &Bond) -> bool {
    bond.order == 1
        && bond.line_styles.main == BondLinePattern::Dashed
        && bond.line_weights.main == BondLineWeight::Bold
}

fn is_hashed_wedge_bond(bond: &Bond) -> bool {
    matches!(
        bond_stereo_kind(bond),
        Some(BondStereoKind::HashedWedgeBegin | BondStereoKind::HashedWedgeEnd)
    )
}

fn is_hash_contact_obstacle(bond: &Bond) -> bool {
    is_hash_bond(bond) || is_hashed_wedge_bond(bond)
}

fn apply_segment_endpoint_retreats(
    start: Point,
    end: Point,
    start_retreat: f64,
    end_retreat: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return (start, end);
    }
    let unit = direction.normalized();
    let max_total = length * 0.8;
    let desired_total = start_retreat.max(0.0) + end_retreat.max(0.0);
    let scale = if desired_total > max_total && desired_total > EPSILON {
        max_total / desired_total
    } else {
        1.0
    };
    let start_shift = start_retreat.max(0.0) * scale;
    let end_shift = end_retreat.max(0.0) * scale;
    (
        Point::new(
            start.x + unit.x * start_shift,
            start.y + unit.y * start_shift,
        ),
        Point::new(end.x - unit.x * end_shift, end.y - unit.y * end_shift),
    )
}

fn bold_band_cap_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    normal: Vector,
    half_width: f64,
    interior_point: Point,
    stroke_width: f64,
) -> (Point, Point) {
    let base_plus = Point::new(
        endpoint.x + normal.x * half_width,
        endpoint.y + normal.y * half_width,
    );
    let base_minus = Point::new(
        endpoint.x - normal.x * half_width,
        endpoint.y - normal.y * half_width,
    );
    if is_hash_bond(bond) {
        return (base_plus, base_minus);
    }
    if let Some(join_plus) = bold_edge_join_point(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        endpoint,
        forward,
        normal,
        half_width,
        1.0,
        stroke_width,
    ) {
        if let Some(join_minus) = bold_edge_join_point(
            object,
            bonds,
            node_map,
            bond,
            shared_node_id,
            endpoint,
            forward,
            normal,
            half_width,
            -1.0,
            stroke_width,
        ) {
            return (join_plus, join_minus);
        }
    }

    let mut contact_directions = Vec::new();
    if let Some(shared_node) = node_map.get(shared_node_id).copied() {
        let shared_point = world_point(object, shared_node);
        for other_bond in bonds {
            if other_bond.id == bond.id || !is_wide_contact_candidate(other_bond) {
                continue;
            }
            if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
                continue;
            }
            if has_joinable_main_line(other_bond) {
                continue;
            }
            let other_node_id = if other_bond.begin == shared_node_id {
                other_bond.end.as_str()
            } else {
                other_bond.begin.as_str()
            };
            let Some(other_node) = node_map.get(other_node_id).copied() else {
                continue;
            };
            let other_point = world_point(object, other_node);
            let vector = Vector::new(
                other_point.x - shared_point.x,
                other_point.y - shared_point.y,
            );
            if vector.length() > 1.0e-6 {
                contact_directions.push(vector);
            }
        }
    }
    let contacts = contact_entries(&contact_directions, normal);
    let has_plus = contacts.iter().any(|entry| entry.side > 0.0);
    let has_minus = contacts.iter().any(|entry| entry.side < 0.0);

    if has_plus && has_minus {
        let plus = contacts
            .iter()
            .filter(|entry| entry.side > 0.0)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        let minus = contacts
            .iter()
            .filter(|entry| entry.side < 0.0)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        if let (Some(plus), Some(minus)) = (plus, minus) {
            let plus_intersection = line_intersection(
                base_plus,
                forward,
                far_side_contact_line_point(endpoint, plus.direction, interior_point, stroke_width),
                plus.direction,
            )
            .unwrap_or(base_plus);
            let minus_intersection = line_intersection(
                base_minus,
                forward,
                far_side_contact_line_point(
                    endpoint,
                    minus.direction,
                    interior_point,
                    stroke_width,
                ),
                minus.direction,
            )
            .unwrap_or(base_minus);
            return (plus_intersection, minus_intersection);
        }
    }

    if has_plus || has_minus {
        let side = if has_plus { 1.0 } else { -1.0 };
        let contact = contacts
            .iter()
            .filter(|entry| entry.side == side)
            .max_by(|a, b| a.side_value.abs().total_cmp(&b.side_value.abs()))
            .copied();
        if let Some(contact) = contact {
            let plus_intersection = line_intersection(
                base_plus,
                forward,
                far_side_contact_line_point(
                    endpoint,
                    contact.direction,
                    interior_point,
                    stroke_width,
                ),
                contact.direction,
            )
            .unwrap_or(base_plus);
            let minus_intersection = line_intersection(
                base_minus,
                forward,
                far_side_contact_line_point(
                    endpoint,
                    contact.direction,
                    interior_point,
                    stroke_width,
                ),
                contact.direction,
            )
            .unwrap_or(base_minus);
            return (plus_intersection, minus_intersection);
        }
    }

    (base_plus, base_minus)
}

#[allow(clippy::too_many_arguments)]
fn solid_wedge_cap_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    tip_plus: Point,
    tip_minus: Point,
    endpoint: Point,
    cap_plus: Point,
    cap_minus: Point,
    stroke_width: f64,
) -> Option<(Point, Point)> {
    if let Some(join_points) = wide_endpoint_join_points_against_main_lines(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        stroke_width,
    ) {
        return Some(join_points);
    }
    let join_plus = solid_wedge_edge_join_point(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        tip_plus,
        endpoint,
        cap_plus,
        stroke_width,
    )?;
    let join_minus = solid_wedge_edge_join_point(
        object,
        bonds,
        node_map,
        bond,
        shared_node_id,
        tip_minus,
        endpoint,
        cap_minus,
        stroke_width,
    )?;
    Some((join_plus, join_minus))
}

#[allow(clippy::too_many_arguments)]
fn solid_wedge_edge_join_point(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    tip: Point,
    endpoint: Point,
    cap_point: Point,
    stroke_width: f64,
) -> Option<Point> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let edge_direction = Vector::new(cap_point.x - tip.x, cap_point.y - tip.y);
    if edge_direction.length() <= EPSILON {
        return None;
    }
    let mut best: Option<(Point, f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if has_joinable_main_line(other_bond) {
            if let Some(other_line) =
                main_bond_cap_line_for_endpoint(object, node_map, other_bond, shared_node_id)
            {
                let Some((intersection, t, _u)) = line_intersection_with_parameters(
                    tip,
                    edge_direction,
                    other_line.point,
                    other_line.direction,
                ) else {
                    continue;
                };
                if t < 0.65 {
                    continue;
                }
                let endpoint_distance = intersection.distance(endpoint);
                let max_join_distance = other_line.length.min(edge_direction.length()) * 0.45;
                if endpoint_distance
                    > (stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE)) * 4.5)
                        .max(max_join_distance)
                {
                    continue;
                }
                if best
                    .as_ref()
                    .is_none_or(|(_, best_distance)| endpoint_distance < *best_distance)
                {
                    best = Some((intersection, endpoint_distance));
                }
                continue;
            }
        }
        for other_side in main_bond_candidate_sides(other_bond) {
            let Some(other_line) = main_bond_boundary_line_for_endpoint(
                object,
                node_map,
                other_bond,
                shared_node_id,
                other_side,
                stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE)),
            ) else {
                continue;
            };
            let Some((intersection, t, u)) = line_intersection_with_parameters(
                tip,
                edge_direction,
                other_line.point,
                other_line.direction,
            ) else {
                continue;
            };
            if t < 0.65 || u < -0.2 {
                continue;
            }
            let endpoint_distance = intersection.distance(endpoint);
            let max_join_distance = other_line.length.min(edge_direction.length()) * 0.45;
            if endpoint_distance
                > (stroke_width.max(other_line.offset_distance) * 4.5).max(max_join_distance)
            {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|(_, best_distance)| endpoint_distance < *best_distance)
            {
                best = Some((intersection, endpoint_distance));
            }
        }
    }
    best.map(|(point, _)| point)
}

#[allow(clippy::too_many_arguments)]
fn bold_edge_join_point(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    normal: Vector,
    half_width: f64,
    side: f64,
    stroke_width: f64,
) -> Option<Point> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current_point = Point::new(
        endpoint.x + normal.x * half_width * side,
        endpoint.y + normal.y * half_width * side,
    );
    let mut best: Option<(Point, f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if solid_joinable_main_line(other_bond) {
            continue;
        }
        if has_joinable_main_line(other_bond) {
            if let Some(other_line) =
                main_bond_cap_line_for_endpoint(object, node_map, other_bond, shared_node_id)
            {
                let Some((intersection, t, _u)) = line_intersection_with_parameters(
                    current_point,
                    forward,
                    other_line.point,
                    other_line.direction,
                ) else {
                    continue;
                };
                let min_backtrack = -(half_width * 2.5).max(stroke_width);
                if t < min_backtrack {
                    continue;
                }
                let distance = intersection.distance(endpoint);
                if distance > other_line.length.max(forward.length()) * 0.45 {
                    continue;
                }
                if best
                    .as_ref()
                    .is_none_or(|(_, best_distance)| distance < *best_distance)
                {
                    best = Some((intersection, distance));
                }
                continue;
            }
        }
        for other_side in main_bond_candidate_sides(other_bond) {
            let Some(other_line) = main_bond_boundary_line_for_endpoint(
                object,
                node_map,
                other_bond,
                shared_node_id,
                other_side,
                stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE)),
            ) else {
                continue;
            };
            let Some((intersection, t, u)) = line_intersection_with_parameters(
                current_point,
                forward,
                other_line.point,
                other_line.direction,
            ) else {
                continue;
            };
            if t < -0.2 || u < -0.2 {
                continue;
            }
            let distance = intersection.distance(endpoint);
            if distance > other_line.length.max(forward.length()) * 0.45 {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|(_, best_distance)| distance < *best_distance)
            {
                best = Some((intersection, distance));
            }
        }
    }
    best.map(|(point, _)| point)
}

fn compute_hashed_wedge_segments(
    start: Point,
    end: Point,
    stroke_width: f64,
) -> Vec<(Point, Point, f64)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let start_gap = HASH_WEDGE_START_OFFSET.min(length * 0.3);
    let end_gap = HASH_WEDGE_END_INSET.min(length * 0.08);
    let usable = (length - start_gap - end_gap).max(0.01);
    let steps = ((usable / HASH_WEDGE_SPACING).round() as usize + 1).max(2);
    let spacing = if steps > 1 {
        usable / (steps - 1) as f64
    } else {
        usable
    };
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let mut segments = Vec::new();
    for index in 0..steps {
        let dist = start_gap + spacing * index as f64;
        if dist > length - end_gap + 1.0e-6 {
            break;
        }
        let progress = if steps > 1 {
            index as f64 / (steps - 1) as f64
        } else {
            1.0
        };
        let half_width = if index == 0 {
            crate::HASH_WEDGE_INITIAL_HALF_WIDTH_CM.value()
        } else {
            crate::HASH_WEDGE_PROGRESS_BASE_HALF_WIDTH_CM.value()
                + progress * crate::HASH_WEDGE_PROGRESS_HALF_WIDTH_RANGE_CM.value()
        } * scale;
        let center = Point::new(start.x + unit.x * dist, start.y + unit.y * dist);
        let segment_width = if index == 0 {
            crate::HASH_WEDGE_INITIAL_SEGMENT_WIDTH_CM.value()
        } else {
            crate::HASH_WEDGE_SEGMENT_WIDTH_CM.value()
        } * scale;
        if index == 0 {
            segments.push((
                Point::new(center.x, center.y - half_width),
                Point::new(center.x, center.y + half_width),
                segment_width,
            ));
        } else {
            segments.push((
                Point::new(
                    center.x - normal.x * half_width,
                    center.y - normal.y * half_width,
                ),
                Point::new(
                    center.x + normal.x * half_width,
                    center.y + normal.y * half_width,
                ),
                segment_width,
            ));
        }
    }
    segments
}

fn lerp_point(from: Point, to: Point, t: f64) -> Point {
    Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
}

fn has_joinable_main_line(bond: &Bond) -> bool {
    if bond.stereo.is_some() || bond.line_weights.main != BondLineWeight::Normal {
        return false;
    }
    if bond.order == 1 || bond.order >= 3 {
        return true;
    }
    bond.order == 2 && side_double_placement(bond).is_some()
}

fn is_joinable_main_line_render(
    bond: &Bond,
    allow_bold_contacts: bool,
    line_weight: BondLineWeight,
) -> bool {
    allow_bold_contacts && line_weight == BondLineWeight::Normal && has_joinable_main_line(bond)
}

fn boundary_lines_from_endpoint(
    endpoint: Point,
    forward: Vector,
    half_width: f64,
) -> Option<[LineGeometry; 2]> {
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    Some([
        LineGeometry {
            point: Point::new(
                endpoint.x + normal.x * half_width,
                endpoint.y + normal.y * half_width,
            ),
            direction: unit,
            shared: endpoint,
            length,
            offset_distance: half_width,
        },
        LineGeometry {
            point: Point::new(
                endpoint.x - normal.x * half_width,
                endpoint.y - normal.y * half_width,
            ),
            direction: unit,
            shared: endpoint,
            length,
            offset_distance: half_width,
        },
    ])
}

fn main_line_boundary_lines_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<[LineGeometry; 2]> {
    let center_line = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    boundary_lines_from_endpoint(
        center_line.shared,
        center_line.direction,
        stroke_width * 0.5,
    )
}

fn wide_boundary_line_pair_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<[LineGeometry; 2]> {
    let lines =
        wide_boundary_lines_for_endpoint(object, node_map, bond, shared_node_id, stroke_width);
    if lines.len() != 2 {
        return None;
    }
    Some([lines[0], lines[1]])
}

#[derive(Debug, Clone, Copy)]
struct BoundaryJoinCandidate {
    point: Point,
    score: f64,
    t: f64,
    u: f64,
}

fn boundary_line_join_candidate(
    current: &LineGeometry,
    other: &LineGeometry,
) -> Option<BoundaryJoinCandidate> {
    let (intersection, t, u) = line_intersection_with_parameters(
        current.point,
        current.direction,
        other.point,
        other.direction,
    )?;
    let min_current =
        -(current.offset_distance * 4.0).max(crate::BOUNDARY_JOIN_MIN_BACKTRACK_CM.value());
    let min_other =
        -(other.offset_distance * 4.0).max(crate::BOUNDARY_JOIN_MIN_BACKTRACK_CM.value());
    if t < min_current || u < min_other {
        return None;
    }
    let distance = intersection.distance(current.shared);
    let max_join_distance = current.length.min(other.length) * 0.55
        + current.offset_distance.max(other.offset_distance) * 4.0;
    if distance > max_join_distance {
        return None;
    }
    Some(BoundaryJoinCandidate {
        point: intersection,
        score: distance,
        t,
        u,
    })
}

fn is_trivial_boundary_assignment(candidates: [BoundaryJoinCandidate; 2]) -> bool {
    candidates
        .iter()
        .all(|candidate| candidate.t.abs() <= 1.0e-4 && candidate.u.abs() <= 1.0e-4)
}

fn paired_boundary_line_join_points(
    current: [LineGeometry; 2],
    other: [LineGeometry; 2],
) -> Option<([Point; 2], f64)> {
    let direct = boundary_line_join_candidate(&current[0], &other[0])
        .zip(boundary_line_join_candidate(&current[1], &other[1]))
        .map(|(plus, minus)| {
            (
                [plus.point, minus.point],
                plus.score + minus.score,
                [plus, minus],
            )
        });
    let swapped = boundary_line_join_candidate(&current[0], &other[1])
        .zip(boundary_line_join_candidate(&current[1], &other[0]))
        .map(|(plus, minus)| {
            (
                [plus.point, minus.point],
                plus.score + minus.score,
                [plus, minus],
            )
        });
    match (direct, swapped) {
        (Some(a), Some(b)) => {
            if is_trivial_boundary_assignment(a.2) && !is_trivial_boundary_assignment(b.2) {
                Some((b.0, b.1))
            } else if is_trivial_boundary_assignment(b.2) && !is_trivial_boundary_assignment(a.2) {
                Some((a.0, a.1))
            } else if a.1 <= b.1 {
                Some((a.0, a.1))
            } else {
                Some((b.0, b.1))
            }
        }
        (Some(a), None) => Some((a.0, a.1)),
        (None, Some(b)) => Some((b.0, b.1)),
        (None, None) => None,
    }
}

fn extended_boundary_line_join_points(
    current: [LineGeometry; 2],
    other: [LineGeometry; 2],
) -> Option<([Point; 2], f64)> {
    let direct = line_intersection(
        current[0].point,
        current[0].direction,
        other[0].point,
        other[0].direction,
    )
    .zip(line_intersection(
        current[1].point,
        current[1].direction,
        other[1].point,
        other[1].direction,
    ))
    .map(|(plus, minus)| {
        let score = plus.distance(current[0].shared) + minus.distance(current[1].shared);
        ([plus, minus], score)
    });
    let swapped = line_intersection(
        current[0].point,
        current[0].direction,
        other[1].point,
        other[1].direction,
    )
    .zip(line_intersection(
        current[1].point,
        current[1].direction,
        other[0].point,
        other[0].direction,
    ))
    .map(|(plus, minus)| {
        let score = plus.distance(current[0].shared) + minus.distance(current[1].shared);
        ([plus, minus], score)
    });

    match (direct, swapped) {
        (Some(a), Some(b)) => {
            if a.1 <= b.1 {
                Some(a)
            } else {
                Some(b)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn wide_endpoint_join_points_against_main_lines(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<(Point, Point)> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current =
        wide_boundary_line_pair_for_endpoint(object, node_map, bond, shared_node_id, stroke_width)?;
    let mut best: Option<([Point; 2], f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if !has_joinable_main_line(other_bond) {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let Some(other) = main_line_boundary_lines_for_endpoint(
            object,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ) else {
            continue;
        };
        let Some(candidate) = extended_boundary_line_join_points(current, other) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, best_score)| candidate.1 < *best_score)
        {
            best = Some(candidate);
        }
    }
    best.map(|(points, _)| (points[0], points[1]))
}

fn main_line_join_points_against_wide_bonds(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    current: [LineGeometry; 2],
    stroke_width: f64,
) -> Option<(Point, Point)> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let mut best: Option<([Point; 2], f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if bond_stereo_kind(other_bond).is_none() {
            continue;
        }
        if is_hashed_wedge_bond(other_bond) {
            continue;
        }
        let other_stroke_width = stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE));
        let Some(other) = wide_boundary_line_pair_for_endpoint(
            object,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ) else {
            continue;
        };
        let Some(candidate) = paired_boundary_line_join_points(current, other) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, best_score)| candidate.1 < *best_score)
        {
            best = Some(candidate);
        }
    }
    best.map(|(points, _)| (points[0], points[1]))
}

fn solid_joinable_main_line(bond: &Bond) -> bool {
    has_joinable_main_line(bond)
        && bond.line_weights.main == BondLineWeight::Normal
        && bond.line_styles.main == BondLinePattern::Solid
}

fn main_line_far_boundary_for_wide_bond(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    contact_sample: Point,
    stroke_width: f64,
) -> Option<LineGeometry> {
    let center = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    let unit = center.direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let to_contact = Vector::new(
        contact_sample.x - center.shared.x,
        contact_sample.y - center.shared.y,
    );
    let contact_side = (to_contact.x * normal.x + to_contact.y * normal.y).signum();
    let far_side = if contact_side.abs() <= EPSILON {
        -1.0
    } else {
        -contact_side
    };
    let half_width = stroke_width * 0.5;
    Some(LineGeometry {
        point: Point::new(
            center.shared.x + normal.x * half_width * far_side,
            center.shared.y + normal.y * half_width * far_side,
        ),
        direction: center.direction,
        shared: center.shared,
        length: center.length,
        offset_distance: half_width,
    })
}

fn bold_main_line_join_polygon(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    stroke_width: f64,
) -> Option<Vec<Point>> {
    let half_width = line_weight_stroke_width(stroke_width, BondLineWeight::Bold) * 0.5;
    let current = boundary_lines_from_endpoint(endpoint, forward, half_width)?;
    let base_plus = current[0].point;
    let base_minus = current[1].point;
    let mut best: Option<(Vec<Point>, f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if !solid_joinable_main_line(other_bond) {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let Some(far_boundary) = main_line_far_boundary_for_wide_bond(
            object,
            node_map,
            other_bond,
            shared_node_id,
            Point::new(endpoint.x + forward.x, endpoint.y + forward.y),
            other_stroke_width,
        ) else {
            continue;
        };
        let Some(plus_intersection) = line_intersection(
            base_plus,
            current[0].direction,
            far_boundary.point,
            far_boundary.direction,
        ) else {
            continue;
        };
        let Some(minus_intersection) = line_intersection(
            base_minus,
            current[1].direction,
            far_boundary.point,
            far_boundary.direction,
        ) else {
            continue;
        };
        let score = plus_intersection.distance(endpoint) + minus_intersection.distance(endpoint);
        let polygon = vec![base_plus, plus_intersection, minus_intersection, base_minus];
        if best
            .as_ref()
            .is_none_or(|(_, best_score)| score < *best_score)
        {
            best = Some((polygon, score));
        }
    }
    best.map(|(polygon, _)| polygon)
}

#[allow(clippy::too_many_arguments)]
fn main_line_polygon_points(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    allow_start_join: bool,
    allow_end_join: bool,
    start_endpoint_profile: Option<Vec<Point>>,
    end_endpoint_profile: Option<Vec<Point>>,
) -> Option<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let half_width = stroke_width * 0.5;
    let mut start_lines = boundary_lines_from_endpoint(start, unit, half_width)?;
    let mut end_lines =
        boundary_lines_from_endpoint(end, Vector::new(-unit.x, -unit.y), half_width)?;

    if start_endpoint_profile.is_none() && allow_start_join {
        if let Some((join_plus, join_minus)) = main_line_join_points_against_wide_bonds(
            object,
            bonds,
            node_map,
            bond,
            &bond.begin,
            start_lines,
            stroke_width,
        ) {
            start_lines[0].point = join_plus;
            start_lines[1].point = join_minus;
        }
    }
    if end_endpoint_profile.is_none() && allow_end_join {
        if let Some((join_plus, join_minus)) = main_line_join_points_against_wide_bonds(
            object,
            bonds,
            node_map,
            bond,
            &bond.end,
            end_lines,
            stroke_width,
        ) {
            end_lines[0].point = join_plus;
            end_lines[1].point = join_minus;
        }
    }

    let start_profile = endpoint_profile_global(
        start_endpoint_profile,
        false,
        vec![start_lines[0].point, start_lines[1].point],
    );
    let end_profile = endpoint_profile_global(
        end_endpoint_profile,
        true,
        vec![end_lines[1].point, end_lines[0].point],
    );

    Some(bond_polygon_from_endpoint_profiles(
        start_profile,
        end_profile,
    ))
}

fn is_centered_double_bond(bond: &Bond) -> bool {
    bond.order == 2 && side_double_placement(bond).is_none()
}

fn centered_double_line_weight_for_side(bond: &Bond, line_side: f64) -> BondLineWeight {
    if line_side > 0.0 {
        bond.line_weights.left
    } else {
        bond.line_weights.right
    }
}

fn centered_double_outer_line_boundary_pair_for_direction(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    reference_direction: Vector,
    stroke_width: f64,
) -> Option<[LineGeometry; 2]> {
    if !is_centered_double_bond(bond) {
        return None;
    }
    let center = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    // Choose the centered-double child line by the bond's global axis rather than the
    // endpoint-local axis. At `bond.end`, the local axis points back into the bond, which
    // would flip the side test and make hash bonds / hashed wedges retreat to the wrong line.
    let axis = if shared_node_id == bond.begin {
        center.direction
    } else if shared_node_id == bond.end {
        Vector::new(-center.direction.x, -center.direction.y)
    } else {
        return None;
    };
    let line_side = main_contact_side(axis, reference_direction)?;
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    centered_double_line_boundary_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        line_side,
        double_bond_center_distance_for_weights(
            begin,
            end,
            stroke_width,
            bond.line_weights.left,
            bond.line_weights.right,
        ) * 0.5,
        stroke_width,
        centered_double_line_weight_for_side(bond, line_side),
    )
    .map(|(lines, _)| lines)
}

fn endpoint_retreat_against_center_double_outer_line(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    half_width: f64,
    stroke_width: f64,
) -> Option<f64> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current = boundary_lines_from_endpoint(endpoint, forward, half_width)?;
    let mut best: Option<f64> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        let other_stroke_width = stroke_width.max(other_bond.stroke_width.max(VIEWER_BOND_STROKE));
        let Some(other) = centered_double_outer_line_boundary_pair_for_direction(
            object,
            node_map,
            other_bond,
            shared_node_id,
            forward,
            other_stroke_width,
        ) else {
            continue;
        };
        let Some((points, _)) = extended_boundary_line_join_points(current, other) else {
            continue;
        };
        let retreat = points
            .into_iter()
            .zip(current.into_iter())
            .map(|(point, line)| {
                let delta = Vector::new(point.x - line.point.x, point.y - line.point.y);
                vector_dot(delta, line.direction).max(0.0)
            })
            .fold(0.0, f64::max);
        if retreat <= EPSILON {
            continue;
        }
        if best.is_none_or(|current_best| retreat < current_best) {
            best = Some(retreat);
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn main_bond_center_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else {
        (end, Vector::new(-unit.x, -unit.y))
    };
    Some(LineGeometry {
        point: shared,
        direction,
        shared,
        length,
        offset_distance: 0.0,
    })
}

fn main_bond_cap_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
) -> Option<LineGeometry> {
    let center_line = main_bond_center_line_for_endpoint(object, node_map, bond, shared_node_id)?;
    Some(LineGeometry {
        point: center_line.shared,
        direction: Vector::new(-center_line.direction.y, center_line.direction.x),
        shared: center_line.shared,
        length: center_line.length,
        offset_distance: 0.0,
    })
}

fn wide_boundary_lines_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Vec<LineGeometry> {
    let begin = match node_map.get(bond.begin.as_str()).copied() {
        Some(node) => world_point(object, node),
        None => return Vec::new(),
    };
    let end = match node_map.get(bond.end.as_str()).copied() {
        Some(node) => world_point(object, node),
        None => return Vec::new(),
    };
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let mut out = Vec::new();

    if bond.line_weights.main == BondLineWeight::Bold
        && bond.line_styles.main == BondLinePattern::Solid
        && bond.stereo.is_none()
    {
        let half_width = line_weight_stroke_width(stroke_width, BondLineWeight::Bold) * 0.5;
        let (shared, direction) = if shared_node_id == bond.begin {
            (begin, unit)
        } else if shared_node_id == bond.end {
            (end, Vector::new(-unit.x, -unit.y))
        } else {
            return Vec::new();
        };
        for side in [1.0, -1.0] {
            out.push(LineGeometry {
                point: Point::new(
                    shared.x + normal.x * half_width * side,
                    shared.y + normal.y * half_width * side,
                ),
                direction,
                shared,
                length,
                offset_distance: half_width,
            });
        }
        return out;
    }

    let Some(stereo_kind) = bond_stereo_kind(bond) else {
        return Vec::new();
    };
    let Some((tip_center, cap_center)) = (match stereo_kind {
        BondStereoKind::SolidWedgeEnd | BondStereoKind::HashedWedgeEnd
            if shared_node_id == bond.end =>
        {
            Some((begin, end))
        }
        BondStereoKind::SolidWedgeBegin | BondStereoKind::HashedWedgeBegin
            if shared_node_id == bond.begin =>
        {
            Some((end, begin))
        }
        _ => None,
    }) else {
        return Vec::new();
    };
    let cap_half_width = solid_wedge_half_width(stroke_width);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    for (cap_point, tip_point) in [
        (
            Point::new(
                cap_center.x + normal.x * cap_half_width,
                cap_center.y + normal.y * cap_half_width,
            ),
            Point::new(
                tip_center.x + normal.x * tip_half_width,
                tip_center.y + normal.y * tip_half_width,
            ),
        ),
        (
            Point::new(
                cap_center.x - normal.x * cap_half_width,
                cap_center.y - normal.y * cap_half_width,
            ),
            Point::new(
                tip_center.x - normal.x * tip_half_width,
                tip_center.y - normal.y * tip_half_width,
            ),
        ),
    ] {
        let direction = Vector::new(tip_point.x - cap_point.x, tip_point.y - cap_point.y);
        out.push(LineGeometry {
            point: cap_point,
            direction,
            shared: cap_center,
            length: direction.length(),
            offset_distance: cap_half_width,
        });
    }
    out
}

fn molecule_stroke(document: &ChemcoreDocument, object: &SceneObject) -> String {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_string(style, "stroke"))
        .unwrap_or_else(|| CHEMCORE_INK.to_string())
}

fn bond_stroke_width(document: &ChemcoreDocument, object: &SceneObject, bond: &Bond) -> f64 {
    if bond.stroke_width > 0.0 {
        return bond.stroke_width;
    }
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| {
            style_number(style, "strokeWidth").or_else(|| style_number(style, "stroke_width"))
        })
        .unwrap_or(DEFAULT_BOND_STROKE)
}

fn style_string(style: &JsonValue, key: &str) -> Option<String> {
    style.get(key)?.as_str().map(ToString::to_string)
}

fn style_nullable_string(style: &JsonValue, key: &str) -> Option<String> {
    let value = style.get(key)?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(ToString::to_string)
}

fn style_number(style: &JsonValue, key: &str) -> Option<f64> {
    style.get(key)?.as_f64()
}

fn style_number_array(style: &JsonValue, key: &str) -> Option<Vec<f64>> {
    Some(
        style
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(JsonValue::as_f64)
            .collect(),
    )
}

fn payload_string(payload: &ObjectPayload, key: &str) -> Option<String> {
    payload.extra.get(key)?.as_str().map(ToString::to_string)
}

fn payload_number(payload: &ObjectPayload, key: &str) -> Option<f64> {
    payload.extra.get(key)?.as_f64()
}

fn payload_bool(payload: &ObjectPayload, key: &str) -> Option<bool> {
    payload.extra.get(key)?.as_bool()
}

fn payload_points(payload: &ObjectPayload, key: &str) -> Vec<Point> {
    payload
        .extra
        .get(key)
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let coords = value.as_array()?;
            Some(Point::new(
                coords.first()?.as_f64()?,
                coords.get(1)?.as_f64()?,
            ))
        })
        .collect()
}

fn payload_box_width(payload: &ObjectPayload, key: &str) -> Option<f64> {
    let coords = payload.extra.get(key)?.as_array()?;
    coords.get(2)?.as_f64()
}

fn payload_runs(payload: &ObjectPayload, key: &str) -> Vec<LabelRun> {
    payload
        .extra
        .get(key)
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .unwrap_or_default()
}

fn payload_arrow_head(payload: &ObjectPayload, key: &str) -> Option<ArrowHeadGeometry> {
    let value = payload.extra.get(key)?;
    Some(ArrowHeadGeometry {
        length: value
            .get("length")
            .and_then(JsonValue::as_f64)
            .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_CM.value()),
        width: value
            .get("width")
            .and_then(JsonValue::as_f64)
            .unwrap_or_else(|| {
                value
                    .get("length")
                    .and_then(JsonValue::as_f64)
                    .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_CM.value())
                    * 0.55
            }),
        head_full: value
            .get("head")
            .and_then(JsonValue::as_str)
            .is_some_and(|head| head.eq_ignore_ascii_case("full")),
    })
}

fn world_point(object: &SceneObject, node: &Node) -> Point {
    Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    )
}

fn label_box_world(node: &Node, object: &SceneObject) -> Option<RectBox> {
    let label = node.label.as_ref()?;
    let bbox = label.bbox()?;
    Some(RectBox {
        x1: bbox[0] + object.transform.translate[0],
        y1: bbox[1] + object.transform.translate[1],
        x2: bbox[2] + object.transform.translate[0],
        y2: bbox[3] + object.transform.translate[1],
    })
}

fn label_polygons_world(node: &Node, object: &SceneObject) -> Vec<Vec<Point>> {
    node.label
        .as_ref()
        .map(|label| {
            label
                .glyph_polygons()
                .into_iter()
                .map(|polygon| {
                    compact_polygon_points(
                        polygon
                            .into_iter()
                            .map(|point| {
                                Point::new(
                                    point.x + object.transform.translate[0],
                                    point.y + object.transform.translate[1],
                                )
                            })
                            .collect(),
                    )
                })
                .filter(|polygon| polygon.len() >= 3)
                .collect()
        })
        .unwrap_or_default()
}

fn segment_intersection_fraction(
    start: Point,
    end: Point,
    first: Point,
    second: Point,
) -> Option<f64> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let edge = Vector::new(second.x - first.x, second.y - first.y);
    let denom = vector_cross(direction, edge);
    if denom.abs() <= EPSILON {
        return None;
    }
    let offset = Vector::new(first.x - start.x, first.y - start.y);
    let t = vector_cross(offset, edge) / denom;
    let u = vector_cross(offset, direction) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

fn clip_point_out_of_polygons(start: Point, end: Point, polygons: &[Vec<Point>]) -> Point {
    let mut best_t: Option<f64> = None;
    for polygon in polygons {
        if polygon.len() < 3 {
            continue;
        }
        let mut polygon_t: Option<f64> = None;
        for index in 0..polygon.len() {
            let next = (index + 1) % polygon.len();
            let Some(t) = segment_intersection_fraction(start, end, polygon[index], polygon[next])
            else {
                continue;
            };
            if t <= EPSILON {
                continue;
            }
            if polygon_t.is_none_or(|current| t > current) {
                polygon_t = Some(t);
            }
        }
        if let Some(t) = polygon_t {
            if best_t.is_none_or(|current| t > current) {
                best_t = Some(t);
            }
        }
    }
    best_t
        .map(|t| {
            Point::new(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            )
        })
        .unwrap_or(start)
}

fn clip_point_out_of_box(start: Point, end: Point, rect: Option<RectBox>, margin: f64) -> Point {
    let Some(expanded) = rect.map(|box_value| box_value.expanded(margin)) else {
        return start;
    };
    if !expanded.contains(start) {
        return start;
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut candidates = Vec::new();
    if dx.abs() > EPSILON {
        for x in [expanded.x1, expanded.x2] {
            let t = (x - start.x) / dx;
            let y = start.y + dy * t;
            if (0.0..=1.0).contains(&t) && y >= expanded.y1 && y <= expanded.y2 {
                candidates.push((t, Point::new(x, y)));
            }
        }
    }
    if dy.abs() > EPSILON {
        for y in [expanded.y1, expanded.y2] {
            let t = (y - start.y) / dy;
            let x = start.x + dx * t;
            if (0.0..=1.0).contains(&t) && x >= expanded.x1 && x <= expanded.x2 {
                candidates.push((t, Point::new(x, y)));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, point)| point)
        .unwrap_or(start)
}

fn clip_point_out_of_label_geometry(
    start: Point,
    end: Point,
    rect: Option<RectBox>,
    polygons: &[Vec<Point>],
    margin: f64,
) -> Point {
    if polygons.is_empty() {
        return clip_point_out_of_box(start, end, rect, margin);
    }
    clip_point_out_of_polygons(start, end, polygons)
}

#[allow(clippy::too_many_arguments)]
fn render_fragment_line(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
) {
    render_fragment_line_with_profiles(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        start_box,
        end_box,
        allow_bold_contacts,
        stroke,
        stroke_width,
        dash_array,
        line_weight,
        object_id,
        true,
        true,
        true,
        true,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_fragment_line_with_profiles(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    start_box: Option<RectBox>,
    end_box: Option<RectBox>,
    allow_bold_contacts: bool,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_weight: BondLineWeight,
    object_id: Option<String>,
    clip_against_label_geometry: bool,
    allow_start_join: bool,
    allow_end_join: bool,
    inherit_kernel_profiles: bool,
    start_endpoint_profile_override: Option<Vec<Point>>,
    end_endpoint_profile_override: Option<Vec<Point>>,
) {
    let start_polygons = node_map
        .get(bond.begin.as_str())
        .map(|node| label_polygons_world(node, object))
        .unwrap_or_default();
    let end_polygons = node_map
        .get(bond.end.as_str())
        .map(|node| label_polygons_world(node, object))
        .unwrap_or_default();
    let (clipped_start, clipped_end) = if clip_against_label_geometry {
        let clipped_start =
            clip_point_out_of_label_geometry(start, end, start_box, &start_polygons, 0.8);
        let clipped_end =
            clip_point_out_of_label_geometry(end, clipped_start, end_box, &end_polygons, 0.8);
        (clipped_start, clipped_end)
    } else {
        (start, end)
    };
    let mut start_retreat = contact_kernel.endpoint_retreat(&bond.id, &bond.begin);
    let mut end_retreat = contact_kernel.endpoint_retreat(&bond.id, &bond.end);
    if is_hash_bond(bond) && line_weight == BondLineWeight::Bold && !dash_array.is_empty() {
        let direction = Vector::new(
            clipped_end.x - clipped_start.x,
            clipped_end.y - clipped_start.y,
        );
        if direction.length() > EPSILON {
            let unit = direction.normalized();
            let half_width = line_weight_stroke_width(stroke_width, line_weight) * 0.5;
            start_retreat = start_retreat.max(
                endpoint_retreat_against_center_double_outer_line(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.begin,
                    clipped_start,
                    unit,
                    half_width,
                    stroke_width,
                )
                .unwrap_or(0.0),
            );
            end_retreat = end_retreat.max(
                endpoint_retreat_against_center_double_outer_line(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.end,
                    clipped_end,
                    Vector::new(-unit.x, -unit.y),
                    half_width,
                    stroke_width,
                )
                .unwrap_or(0.0),
            );
        }
    }
    let (clipped_start, clipped_end) =
        apply_segment_endpoint_retreats(clipped_start, clipped_end, start_retreat, end_retreat);
    let mut start_endpoint_profile = start_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.begin)
        } else {
            None
        }
    });
    let mut end_endpoint_profile = end_endpoint_profile_override.or_else(|| {
        if inherit_kernel_profiles {
            contact_kernel.endpoint_profile(&bond.id, &bond.end)
        } else {
            None
        }
    });
    if start_retreat > EPSILON {
        start_endpoint_profile = None;
    }
    if end_retreat > EPSILON {
        end_endpoint_profile = None;
    }
    let use_start_contact_kernel =
        contact_kernel.uses_endpoint(&bond.id, &bond.begin) || start_endpoint_profile.is_some();
    let use_end_contact_kernel =
        contact_kernel.uses_endpoint(&bond.id, &bond.end) || end_endpoint_profile.is_some();
    if line_weight == BondLineWeight::Normal && dash_array.is_empty() {
        let allow_main_line_join =
            is_joinable_main_line_render(bond, allow_bold_contacts, line_weight);
        if let Some(points) = main_line_polygon_points(
            object,
            bonds,
            node_map,
            bond,
            clipped_start,
            clipped_end,
            stroke_width,
            allow_main_line_join && allow_start_join && !use_start_contact_kernel,
            allow_main_line_join && allow_end_join && !use_end_contact_kernel,
            start_endpoint_profile.clone(),
            end_endpoint_profile.clone(),
        ) {
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
            return;
        }
    }
    if !dash_array.is_empty() {
        let polygon_points = if line_weight == BondLineWeight::Bold {
            Some(compute_bold_bond_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                stroke_width,
                allow_bold_contacts && !use_start_contact_kernel,
                allow_bold_contacts && !use_end_contact_kernel,
                start_endpoint_profile.clone(),
                end_endpoint_profile.clone(),
            ))
        } else {
            main_line_polygon_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                line_weight_stroke_width(stroke_width, line_weight),
                is_joinable_main_line_render(bond, allow_bold_contacts, line_weight)
                    && allow_start_join
                    && !use_start_contact_kernel,
                is_joinable_main_line_render(bond, allow_bold_contacts, line_weight)
                    && allow_end_join
                    && !use_end_contact_kernel,
                start_endpoint_profile.clone(),
                end_endpoint_profile.clone(),
            )
        };
        if let Some(points) = polygon_points {
            push_bond_polygon(
                out,
                &bond.id,
                points,
                stroke,
                stroke,
                0.0,
                object_id.clone(),
            );
            let knockouts = if line_weight == BondLineWeight::Bold {
                hash_bond_knockout_polygons(
                    clipped_start,
                    clipped_end,
                    line_weight_stroke_width(stroke_width, line_weight),
                )
            } else {
                dashed_bond_knockout_polygons(
                    clipped_start,
                    clipped_end,
                    line_weight_stroke_width(stroke_width, line_weight),
                    &dash_array,
                )
            };
            for knockout in knockouts {
                push_knockout_polygon(out, knockout, object_id.clone());
            }
            return;
        }
    }
    if line_weight == BondLineWeight::Bold && dash_array.is_empty() {
        let direction = Vector::new(
            clipped_end.x - clipped_start.x,
            clipped_end.y - clipped_start.y,
        );
        if direction.length() > EPSILON {
            if !use_start_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.begin,
                    clipped_start,
                    direction,
                    stroke_width,
                ) {
                    push_bond_polygon(
                        out,
                        &bond.id,
                        points,
                        stroke,
                        stroke,
                        0.0,
                        object_id.clone(),
                    );
                }
            }
            if !use_end_contact_kernel {
                if let Some(points) = bold_main_line_join_polygon(
                    object,
                    bonds,
                    node_map,
                    bond,
                    &bond.end,
                    clipped_end,
                    Vector::new(-direction.x, -direction.y),
                    stroke_width,
                ) {
                    push_bond_polygon(
                        out,
                        &bond.id,
                        points,
                        stroke,
                        stroke,
                        0.0,
                        object_id.clone(),
                    );
                }
            }
        }
        push_bond_polygon(
            out,
            &bond.id,
            compute_bold_bond_points(
                object,
                bonds,
                node_map,
                bond,
                clipped_start,
                clipped_end,
                stroke_width,
                allow_bold_contacts && !use_start_contact_kernel,
                allow_bold_contacts && !use_end_contact_kernel,
                start_endpoint_profile,
                end_endpoint_profile,
            ),
            stroke,
            stroke,
            0.0,
            object_id,
        );
        return;
    }
    push_bond_line(
        out,
        &bond.id,
        clipped_start,
        clipped_end,
        stroke,
        line_weight_stroke_width(stroke_width, line_weight),
        dash_array,
        object_id,
    );
}

fn arrow_shaft_end(from: Point, to: Point, arrow_head: ArrowHeadGeometry) -> Point {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let head_length = crate::ARROW_SHAPE_MIN_HEAD_LENGTH_CM
        .value()
        .max(arrow_head.length * 0.6);
    let notch_length = crate::ARROW_SHAPE_MIN_NOTCH_LENGTH_CM
        .value()
        .max(head_length * 0.66);
    let center_length = notch_length.min(length * 0.8).max(0.0);
    Point::new(to.x - unit.x * center_length, to.y - unit.y * center_length)
}

fn arrow_head_points(from: Point, to: Point, arrow_head: ArrowHeadGeometry) -> Vec<Point> {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let head_length = crate::ARROW_SHAPE_MIN_HEAD_LENGTH_CM
        .value()
        .max(arrow_head.length * 0.6);
    let head_width = crate::ARROW_SHAPE_MIN_HEAD_WIDTH_CM
        .value()
        .max(arrow_head.width * 1.16);
    let notch_length = crate::ARROW_SHAPE_MIN_NOTCH_LENGTH_CM.value().max(
        (head_length * 0.66).min(head_length - crate::ARROW_SHAPE_MIN_HEAD_TO_NOTCH_GAP_CM.value()),
    );
    let tip = to;
    let left = Point::new(
        to.x - unit.x * head_length + normal.x * (head_width / 2.0),
        to.y - unit.y * head_length + normal.y * (head_width / 2.0),
    );
    let right = Point::new(
        to.x - unit.x * head_length - normal.x * (head_width / 2.0),
        to.y - unit.y * head_length - normal.y * (head_width / 2.0),
    );
    if arrow_head.head_full && notch_length < head_length - 0.2 {
        let notch = Point::new(to.x - unit.x * notch_length, to.y - unit.y * notch_length);
        vec![tip, left, notch, right]
    } else {
        vec![tip, left, right]
    }
}

fn split_runs_by_line(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut out = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                let mut next_run = run.clone();
                next_run.text = (*segment).to_string();
                out.last_mut()
                    .expect("line vector always exists")
                    .push(next_run);
            }
            if index + 1 < segments.len() {
                out.push(Vec::new());
            }
        }
    }
    out
}

fn split_preserved_text_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn wrap_text_lines(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let raw_lines: Vec<&str> = text.split('\n').collect();
    let max_chars = (max_width
        / crate::TEXT_WRAP_ESTIMATED_CHAR_WIDTH_CM
            .value()
            .max(font_size * 0.6))
    .floor()
    .max(8.0) as usize;
    let mut out = Vec::new();

    for raw_line in raw_lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() <= max_chars || !line.contains(' ') {
            out.push(line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in line.split_whitespace() {
            let next = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if next.len() > max_chars && !current.is_empty() {
                out.push(current);
                current = word.to_string();
            } else {
                current = next;
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }

    out
}

fn unit_normal(start: Point, end: Point) -> (f64, f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy).max(1.0);
    (-dy / length, dx / length)
}

fn inset_bond_segment(
    start: Point,
    end: Point,
    inset_start: f64,
    inset_end: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length().max(1.0);
    let unit = direction.normalized();
    let clamped_start = inset_start.max(0.0).min(length * 0.45);
    let clamped_end = inset_end.max(0.0).min(length * 0.45);
    (
        Point::new(
            start.x + unit.x * clamped_start,
            start.y + unit.y * clamped_start,
        ),
        Point::new(end.x - unit.x * clamped_end, end.y - unit.y * clamped_end),
    )
}

fn line_weight_stroke_width(stroke_width: f64, line_weight: BondLineWeight) -> f64 {
    if line_weight == BondLineWeight::Bold {
        (BOLD_BOND_WIDTH * (stroke_width / VIEWER_BOND_STROKE)).max(stroke_width)
    } else {
        stroke_width
    }
}

fn multi_bond_inner_gap(start: Point, end: Point) -> f64 {
    start.distance(end) * MULTI_BOND_OFFSET_RATIO
}

fn double_bond_center_distance_for_weights(
    start: Point,
    end: Point,
    stroke_width: f64,
    first_weight: BondLineWeight,
    second_weight: BondLineWeight,
) -> f64 {
    let first_width = line_weight_stroke_width(stroke_width, first_weight);
    let second_width = line_weight_stroke_width(stroke_width, second_weight);
    multi_bond_inner_gap(start, end) + 0.5 * (first_width + second_width)
}

fn double_bond_offset_distance(start: Point, end: Point, stroke_width: f64) -> f64 {
    double_bond_center_distance_for_weights(
        start,
        end,
        stroke_width,
        BondLineWeight::Normal,
        BondLineWeight::Normal,
    )
}

fn triple_bond_offset_distance(start: Point, end: Point, _stroke_width: f64) -> f64 {
    start.distance(end) * MULTI_BOND_OFFSET_RATIO
}

fn solid_wedge_half_width(stroke_width: f64) -> f64 {
    let _ = stroke_width;
    SOLID_WEDGE_HALF_WIDTH
}

fn solid_wedge_tip_half_width(stroke_width: f64) -> f64 {
    let _ = stroke_width;
    crate::SOLID_WEDGE_TIP_HALF_WIDTH_CM.value()
}

fn dash_gap_intervals(length: f64, dash_array: &[f64]) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let segments: Vec<f64> = dash_array
        .iter()
        .copied()
        .filter(|value| *value > EPSILON)
        .collect();
    if segments.is_empty() {
        return Vec::new();
    }
    let mut gap_intervals = Vec::new();
    let mut offset = 0.0;
    let mut gap_segment = false;
    let mut index = 0usize;
    while offset < length - EPSILON {
        let segment_length = segments[index % segments.len()];
        let next = (offset + segment_length).min(length);
        if gap_segment && next > offset + EPSILON {
            gap_intervals.push((offset, next));
        }
        offset += segment_length;
        gap_segment = !gap_segment;
        index += 1;
    }
    gap_intervals
}

fn equal_black_segment_gap_intervals(
    length: f64,
    start_offset: f64,
    end_inset: f64,
    stripe_length: f64,
    target_gap_length: f64,
) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let usable_start = start_offset.max(0.0);
    let usable_end = (length - end_inset).max(usable_start);
    let usable_length = usable_end - usable_start;
    let stripe_length = stripe_length.max(EPSILON);
    let target_gap_length = target_gap_length.max(EPSILON);
    if usable_length <= stripe_length + EPSILON {
        return Vec::new();
    }

    let mut stripe_count = ((usable_length + target_gap_length)
        / (stripe_length + target_gap_length))
        .round() as usize;
    stripe_count = stripe_count.max(2);
    while stripe_count > 1 && stripe_length * stripe_count as f64 > usable_length + EPSILON {
        stripe_count -= 1;
    }
    if stripe_count < 2 {
        return Vec::new();
    }

    let gap_count = stripe_count - 1;
    let total_gap_length = (usable_length - stripe_length * stripe_count as f64).max(0.0);
    let gap_length = total_gap_length / gap_count as f64;
    let mut intervals = Vec::with_capacity(gap_count);
    let mut cursor = usable_start + stripe_length;
    for index in 0..gap_count {
        let gap_start = cursor;
        let gap_end = if index + 1 == gap_count {
            usable_end - stripe_length
        } else {
            gap_start + gap_length
        };
        if gap_end > gap_start + EPSILON {
            intervals.push((gap_start, gap_end));
        }
        cursor = gap_end + stripe_length;
    }
    intervals
}

fn dashed_bond_knockout_polygons(
    start: Point,
    end: Point,
    stroke_width: f64,
    dash_array: &[f64],
) -> Vec<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width =
        stroke_width * 0.5 + stroke_width.max(crate::DASH_GAP_STROKE_EXTRA_CM.value()) * 0.45;
    dash_gap_intervals(length, dash_array)
        .into_iter()
        .map(|(gap_start, gap_end)| {
            let segment_start =
                Point::new(start.x + unit.x * gap_start, start.y + unit.y * gap_start);
            let segment_end = Point::new(start.x + unit.x * gap_end, start.y + unit.y * gap_end);
            compact_polygon_points(vec![
                Point::new(
                    segment_start.x + normal.x * half_width,
                    segment_start.y + normal.y * half_width,
                ),
                Point::new(
                    segment_end.x + normal.x * half_width,
                    segment_end.y + normal.y * half_width,
                ),
                Point::new(
                    segment_end.x - normal.x * half_width,
                    segment_end.y - normal.y * half_width,
                ),
                Point::new(
                    segment_start.x - normal.x * half_width,
                    segment_start.y - normal.y * half_width,
                ),
            ])
        })
        .collect()
}

fn hash_bond_knockout_polygons(start: Point, end: Point, stroke_width: f64) -> Vec<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width = stroke_width * 0.5 + stroke_width * 0.12;
    let scale = stroke_width / VIEWER_BOND_STROKE;
    equal_black_segment_gap_intervals(
        length,
        0.0,
        0.0,
        HASH_BLACK_SEGMENT_LENGTH * scale,
        HASH_TARGET_GAP_LENGTH * scale,
    )
    .into_iter()
    .map(|(gap_start, gap_end)| {
        let segment_start = Point::new(start.x + unit.x * gap_start, start.y + unit.y * gap_start);
        let segment_end = Point::new(start.x + unit.x * gap_end, start.y + unit.y * gap_end);
        compact_polygon_points(vec![
            Point::new(
                segment_start.x + normal.x * half_width,
                segment_start.y + normal.y * half_width,
            ),
            Point::new(
                segment_end.x + normal.x * half_width,
                segment_end.y + normal.y * half_width,
            ),
            Point::new(
                segment_end.x - normal.x * half_width,
                segment_end.y - normal.y * half_width,
            ),
            Point::new(
                segment_start.x - normal.x * half_width,
                segment_start.y - normal.y * half_width,
            ),
        ])
    })
    .collect()
}

fn hashed_wedge_gap_intervals(length: f64, stroke_width: f64) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let start_offset = (crate::HASH_WEDGE_GAP_START_OFFSET_CM.value() * scale).min(length * 0.06);
    let end_inset = (crate::HASH_WEDGE_GAP_END_INSET_CM.value() * scale).min(length * 0.03);
    equal_black_segment_gap_intervals(
        length,
        start_offset,
        end_inset,
        (HASH_BLACK_SEGMENT_LENGTH * scale).max(length * 0.014),
        (HASH_TARGET_GAP_LENGTH * scale).max(length * 0.018),
    )
}

fn side_double_placement(bond: &Bond) -> Option<DoubleBondPlacement> {
    match bond.double.as_ref().map(|double| double.placement) {
        Some(DoubleBondPlacement::Left) => Some(DoubleBondPlacement::Left),
        Some(DoubleBondPlacement::Right) => Some(DoubleBondPlacement::Right),
        _ => None,
    }
}

fn line_pattern_dash_array(pattern: BondLinePattern) -> Vec<f64> {
    if pattern == BondLinePattern::Dashed {
        DASHED_BOND_PATTERN.to_vec()
    } else {
        Vec::new()
    }
}

fn outer_line_pattern(bond: &Bond, side: f64) -> BondLinePattern {
    if side > 0.0 {
        bond.line_styles.left
    } else {
        bond.line_styles.right
    }
}

fn outer_line_weight(bond: &Bond, side: f64) -> BondLineWeight {
    if side > 0.0 {
        bond.line_weights.left
    } else {
        bond.line_weights.right
    }
}

fn fragment_node_degree(bonds: &[Bond], node_id: &str) -> usize {
    bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .count()
}

fn fragment_outer_bond_offset_for_side(
    bond: &Bond,
    side: f64,
    stroke_width: f64,
    start: Point,
    end: Point,
) -> Option<f64> {
    if bond.order >= 3 {
        return Some(triple_bond_offset_distance(start, end, stroke_width));
    }
    let placement = side_double_placement(bond)?;
    if (placement == DoubleBondPlacement::Left && side < 0.0)
        || (placement == DoubleBondPlacement::Right && side > 0.0)
    {
        return Some(double_bond_center_distance_for_weights(
            start,
            end,
            stroke_width,
            bond.line_weights.main,
            outer_line_weight(bond, side),
        ));
    }
    None
}

fn outer_bond_candidate_sides(bond: &Bond) -> Vec<f64> {
    if bond.order >= 3 {
        return vec![1.0, -1.0];
    }
    match side_double_placement(bond) {
        Some(DoubleBondPlacement::Left) => vec![-1.0],
        Some(DoubleBondPlacement::Right) => vec![1.0],
        _ => Vec::new(),
    }
}

fn outer_bond_half_width_for_side(bond: &Bond, side: f64, stroke_width: f64) -> f64 {
    line_weight_stroke_width(stroke_width, outer_line_weight(bond, side)) * 0.5
}

fn outer_bond_offset_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let offset_distance =
        fragment_outer_bond_offset_for_side(bond, side, stroke_width, begin, end)?;
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else {
        (end, Vector::new(-unit.x, -unit.y))
    };
    Some(LineGeometry {
        point: Point::new(
            shared.x + normal.x * offset_distance * side,
            shared.y + normal.y * offset_distance * side,
        ),
        direction,
        shared,
        length,
        offset_distance,
    })
}

fn outer_bond_boundary_line_pair_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<([LineGeometry; 2], LineGeometry)> {
    let center = outer_bond_offset_line_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        side,
        stroke_width,
    )?;
    let half_width = outer_bond_half_width_for_side(bond, side, stroke_width);
    let boundaries = boundary_lines_from_endpoint(center.point, center.direction, half_width)?;
    Some((boundaries, center))
}

fn centered_double_line_boundary_pair_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    line_side: f64,
    offset_distance: f64,
    stroke_width: f64,
    line_weight: BondLineWeight,
) -> Option<([LineGeometry; 2], LineGeometry)> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else if shared_node_id == bond.end {
        (end, Vector::new(-unit.x, -unit.y))
    } else {
        return None;
    };
    let center = LineGeometry {
        point: Point::new(
            shared.x + normal.x * offset_distance * line_side,
            shared.y + normal.y * offset_distance * line_side,
        ),
        direction,
        shared,
        length,
        offset_distance,
    };
    let half_width = line_weight_stroke_width(stroke_width, line_weight) * 0.5;
    let boundaries = boundary_lines_from_endpoint(center.point, center.direction, half_width)?;
    Some((boundaries, center))
}

fn boundary_lines_with_profile_terminals(
    mut lines: [LineGeometry; 2],
    profile: &[Point],
) -> [LineGeometry; 2] {
    if profile.len() < 2 {
        return lines;
    }
    let terminals = [profile[0], *profile.last().unwrap_or(&profile[0])];
    let direct = terminals[0].distance(lines[0].point) + terminals[1].distance(lines[1].point);
    let swapped = terminals[0].distance(lines[1].point) + terminals[1].distance(lines[0].point);
    if direct <= swapped {
        lines[0].point = terminals[0];
        lines[1].point = terminals[1];
    } else {
        lines[0].point = terminals[1];
        lines[1].point = terminals[0];
    }
    lines
}

fn main_bond_drawn_boundary_pair_for_endpoint(
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Option<([LineGeometry; 2], LineGeometry)> {
    if is_hash_contact_obstacle(bond) {
        return None;
    }
    let geometry =
        main_bond_endpoint_geometry(object, node_map, bond, shared_node_id, stroke_width)?;
    let mut lines = [
        LineGeometry {
            point: geometry.base_plus,
            direction: geometry.contour_plus.direction,
            shared: geometry.center,
            length: geometry.contour_plus.extent,
            offset_distance: geometry.contour_plus.half_width,
        },
        LineGeometry {
            point: geometry.base_minus,
            direction: geometry.contour_minus.direction,
            shared: geometry.center,
            length: geometry.contour_minus.extent,
            offset_distance: geometry.contour_minus.half_width,
        },
    ];
    if let Some(profile) = contact_kernel.endpoint_profile(&bond.id, shared_node_id) {
        lines = boundary_lines_with_profile_terminals(lines, &profile);
    }
    Some((
        lines,
        LineGeometry {
            point: geometry.center,
            direction: geometry.axis,
            shared: geometry.center,
            length: geometry
                .contour_plus
                .extent
                .max(geometry.contour_minus.extent)
                .max(1.0),
            offset_distance: 0.0,
        },
    ))
}

fn outer_bond_drawn_boundary_pairs_for_endpoint(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    stroke_width: f64,
) -> Vec<([LineGeometry; 2], LineGeometry)> {
    let mut out = Vec::new();
    for side in outer_bond_candidate_sides(bond) {
        let Some((mut lines, center)) = outer_bond_boundary_line_pair_for_endpoint(
            object,
            node_map,
            bond,
            shared_node_id,
            side,
            stroke_width,
        ) else {
            continue;
        };
        if let Some(profile) = outer_bond_endpoint_profile_for_side(
            object,
            bonds,
            node_map,
            bond,
            shared_node_id,
            side,
            stroke_width,
        ) {
            lines = boundary_lines_with_profile_terminals(lines, &profile);
        }
        out.push((lines, center));
    }
    out
}

fn main_bond_candidate_sides(bond: &Bond) -> Vec<f64> {
    if bond.line_weights.main == BondLineWeight::Bold
        && bond.line_styles.main == BondLinePattern::Solid
    {
        vec![1.0, -1.0]
    } else {
        vec![0.0]
    }
}

fn main_bond_boundary_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else {
        (end, Vector::new(-unit.x, -unit.y))
    };
    let offset_distance = if side == 0.0 {
        stroke_width * 0.5
    } else {
        line_weight_stroke_width(stroke_width, BondLineWeight::Bold) * 0.5
    };
    let point = if side == 0.0 {
        far_side_contact_line_point(
            shared,
            direction,
            if shared_node_id == bond.begin {
                end
            } else {
                begin
            },
            stroke_width,
        )
    } else {
        Point::new(
            shared.x + normal.x * offset_distance * side,
            shared.y + normal.y * offset_distance * side,
        )
    };
    Some(LineGeometry {
        point,
        direction,
        shared,
        length,
        offset_distance,
    })
}

fn outer_bond_endpoint_profile_for_side(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> Option<Vec<Point>> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    if outer_line_pattern(bond, side) != BondLinePattern::Solid {
        return None;
    }

    let current_stroke_width = stroke_width.max(bond.stroke_width.max(VIEWER_BOND_STROKE));
    let (current, current_center) = outer_bond_boundary_line_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        side,
        current_stroke_width,
    )?;
    let current_local_side = line_geometry_local_side(current_center)?;

    let mut best: Option<([Point; 2], f64)> = None;
    let mut straight_through_profile = false;
    let mut acute_retreat = false;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let mut same_side_outer_candidate = false;
        for other_side in outer_bond_candidate_sides(other_bond) {
            if outer_line_pattern(other_bond, other_side) != BondLinePattern::Solid {
                continue;
            }
            let Some((other, other_center)) = outer_bond_boundary_line_pair_for_endpoint(
                object,
                node_map,
                other_bond,
                shared_node_id,
                other_side,
                other_stroke_width,
            ) else {
                continue;
            };
            if main_contact_is_straight_through(current_center.direction, other_center.direction) {
                if side_double_placement(bond).is_some() {
                    straight_through_profile = true;
                }
                continue;
            }
            if side_double_placement(bond).is_some()
                && !line_geometries_share_side(current_center, other_center)
            {
                continue;
            }
            same_side_outer_candidate = true;
            let Some(candidate) = extended_boundary_line_join_points(current, other) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, best_score)| candidate.1 < *best_score)
            {
                best = Some(candidate);
            }
        }
        if side_double_placement(bond).is_some()
            && !same_side_outer_candidate
            && side_double_outer_line_requires_acute_retreat(
                object,
                node_map,
                other_bond,
                shared_node_id,
                current_center,
                current_local_side,
            )
        {
            acute_retreat = true;
        }
    }

    if acute_retreat && best.is_none() {
        return None;
    }
    if let Some((points, _)) = best {
        return Some(compact_polygon_points(vec![points[0], points[1]]));
    }
    if straight_through_profile {
        return Some(compact_polygon_points(vec![
            current[0].point,
            current[1].point,
        ]));
    }
    None
}

fn line_geometry_local_side(line: LineGeometry) -> Option<f64> {
    let normal = Vector::new(-line.direction.y, line.direction.x);
    let offset = Vector::new(line.point.x - line.shared.x, line.point.y - line.shared.y);
    let local_side = vector_dot(offset, normal).signum();
    if local_side.abs() <= EPSILON {
        None
    } else {
        Some(local_side)
    }
}

fn line_geometries_share_side(first: LineGeometry, second: LineGeometry) -> bool {
    let first_offset = Vector::new(
        first.point.x - first.shared.x,
        first.point.y - first.shared.y,
    );
    let second_offset = Vector::new(
        second.point.x - second.shared.x,
        second.point.y - second.shared.y,
    );
    vector_dot(first_offset, second_offset) > EPSILON
}

fn side_double_outer_line_requires_acute_retreat(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    other_bond: &Bond,
    shared_node_id: &str,
    current_center: LineGeometry,
    current_local_side: f64,
) -> bool {
    let Some(other_axis) =
        bond_axis_line_for_endpoint(object, node_map, other_bond, shared_node_id)
    else {
        return false;
    };
    let Some(contact_side) = main_contact_side(current_center.direction, other_axis.direction)
    else {
        return false;
    };
    (contact_side - current_local_side).abs() <= 1.0e-6
        && bond_ray_is_acute(current_center.direction, other_axis.direction)
}

fn bond_axis_line_for_endpoint(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
) -> Option<LineGeometry> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    let forward = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = forward.length();
    if length <= EPSILON {
        return None;
    }
    let unit = forward.normalized();
    let (shared, direction) = if shared_node_id == bond.begin {
        (begin, unit)
    } else if shared_node_id == bond.end {
        (end, Vector::new(-unit.x, -unit.y))
    } else {
        return None;
    };
    Some(LineGeometry {
        point: shared,
        direction,
        shared,
        length,
        offset_distance: 0.0,
    })
}

fn center_double_endpoint_profile_for_line_side(
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    line_side: f64,
    offset_distance: f64,
    stroke_width: f64,
    line_weight: BondLineWeight,
) -> Option<Vec<Point>> {
    let shared_node = node_map.get(shared_node_id).copied()?;
    if shared_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return None;
    }
    let current_stroke_width = stroke_width.max(bond.stroke_width.max(VIEWER_BOND_STROKE));
    let (current, current_center) = centered_double_line_boundary_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        line_side,
        offset_distance,
        current_stroke_width,
        line_weight,
    )?;
    let current_local_side = line_geometry_local_side(current_center)?;

    let mut best: Option<([Point; 2], f64)> = None;
    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        let other_stroke_width = other_bond.stroke_width.max(VIEWER_BOND_STROKE);
        let mut candidates = Vec::new();
        if let Some(candidate) = main_bond_drawn_boundary_pair_for_endpoint(
            object,
            contact_kernel,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ) {
            candidates.push(candidate);
        }
        candidates.extend(outer_bond_drawn_boundary_pairs_for_endpoint(
            object,
            bonds,
            node_map,
            other_bond,
            shared_node_id,
            other_stroke_width,
        ));

        for (other, other_center) in candidates {
            if center_double_skips_extension(current_center.direction, other_center.direction) {
                continue;
            }
            let Some(contact_side) =
                main_contact_side(current_center.direction, other_center.direction)
            else {
                continue;
            };
            if (contact_side - current_local_side).abs() > 1.0e-6 {
                continue;
            }
            let Some(candidate) = extended_boundary_line_join_points(current, other) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, best_score)| candidate.1 < *best_score)
            {
                best = Some(candidate);
            }
        }
    }

    best.map(|(points, _)| compact_polygon_points(vec![points[0], points[1]]))
}

fn line_intersection(
    point: Point,
    direction: Vector,
    other_point: Point,
    other_direction: Vector,
) -> Option<Point> {
    line_intersection_with_parameters(point, direction, other_point, other_direction)
        .map(|value| value.0)
}

fn line_intersection_with_parameters(
    point: Point,
    direction: Vector,
    other_point: Point,
    other_direction: Vector,
) -> Option<(Point, f64, f64)> {
    let cross = direction.x * other_direction.y - direction.y * other_direction.x;
    if cross.abs() < 1.0e-6 {
        return None;
    }
    let dx = other_point.x - point.x;
    let dy = other_point.y - point.y;
    let t = (dx * other_direction.y - dy * other_direction.x) / cross;
    let u = (dx * direction.y - dy * direction.x) / cross;
    Some((
        Point::new(point.x + direction.x * t, point.y + direction.y * t),
        t,
        u,
    ))
}

#[derive(Debug, Clone, Copy)]
struct ContactEntry {
    direction: Vector,
    side: f64,
    side_value: f64,
}

fn contact_entries(directions: &[Vector], normal: Vector) -> Vec<ContactEntry> {
    directions
        .iter()
        .filter_map(|direction| {
            let unit = direction.normalized();
            let side_value = normal.x * unit.x + normal.y * unit.y;
            let side = side_value.signum();
            if side.abs() <= EPSILON {
                None
            } else {
                Some(ContactEntry {
                    direction: unit,
                    side,
                    side_value,
                })
            }
        })
        .collect()
}

fn far_side_contact_line_point(
    contact_point: Point,
    contact_direction: Vector,
    interior_point: Point,
    stroke_width: f64,
) -> Point {
    let normal = Vector::new(-contact_direction.y, contact_direction.x);
    let to_interior = Vector::new(
        interior_point.x - contact_point.x,
        interior_point.y - contact_point.y,
    );
    let interior_side = (to_interior.x * normal.x + to_interior.y * normal.y).signum();
    let offset = stroke_width * 0.55;
    Point::new(
        contact_point.x
            - normal.x
                * if interior_side == 0.0 {
                    1.0
                } else {
                    interior_side
                }
                * offset,
        contact_point.y
            - normal.y
                * if interior_side == 0.0 {
                    1.0
                } else {
                    interior_side
                }
                * offset,
    )
}

fn wide_contact_directions(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    wide_node_id: &str,
) -> Vec<Vector> {
    let Some(wide_node) = node_map.get(wide_node_id).copied() else {
        return Vec::new();
    };
    if wide_node
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text())
    {
        return Vec::new();
    }
    let wide_point = world_point(object, wide_node);
    let mut out = Vec::new();
    for other_bond in bonds {
        if other_bond.id == bond.id || !is_wide_contact_candidate(other_bond) {
            continue;
        }
        if other_bond.begin != wide_node_id && other_bond.end != wide_node_id {
            continue;
        }
        let other_node_id = if other_bond.begin == wide_node_id {
            other_bond.end.as_str()
        } else {
            other_bond.begin.as_str()
        };
        let Some(other_node) = node_map.get(other_node_id).copied() else {
            continue;
        };
        let other_point = world_point(object, other_node);
        let vector = Vector::new(other_point.x - wide_point.x, other_point.y - wide_point.y);
        if vector.length() > 1.0e-6 {
            out.push(vector);
        }
    }
    out
}

fn is_wide_contact_candidate(bond: &Bond) -> bool {
    if bond_stereo_kind(bond).is_some() {
        return false;
    }
    if bond.order == 1 {
        return true;
    }
    matches!(
        side_double_placement(bond),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    )
}

#[derive(Debug, Clone, Copy)]
enum BondStereoKind {
    SolidWedgeBegin,
    SolidWedgeEnd,
    HashedWedgeBegin,
    HashedWedgeEnd,
}

fn bond_stereo_kind(bond: &Bond) -> Option<BondStereoKind> {
    if let Some(stereo) = bond.stereo.as_ref() {
        return match (stereo.kind.as_str(), stereo.wide_end.as_str()) {
            ("solid-wedge", "begin") => Some(BondStereoKind::SolidWedgeBegin),
            ("solid-wedge", "end") => Some(BondStereoKind::SolidWedgeEnd),
            ("hashed-wedge", "begin") => Some(BondStereoKind::HashedWedgeBegin),
            ("hashed-wedge", "end") => Some(BondStereoKind::HashedWedgeEnd),
            _ => None,
        };
    }
    let display = bond
        .meta
        .pointer("/import/cdxml/display")
        .and_then(JsonValue::as_str)?;
    match display {
        "WedgeBegin" => Some(BondStereoKind::SolidWedgeEnd),
        "WedgeEnd" => Some(BondStereoKind::SolidWedgeBegin),
        "WedgedHashBegin" => Some(BondStereoKind::HashedWedgeEnd),
        "WedgedHashEnd" => Some(BondStereoKind::HashedWedgeBegin),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f64, right: f64) {
        assert!((left - right).abs() <= 1.0e-6, "{left} != {right}");
    }

    fn line_geometry(point: Point, direction: Vector, shared: Point) -> LineGeometry {
        LineGeometry {
            point,
            direction,
            shared,
            length: 20.0,
            offset_distance: 1.0,
        }
    }

    fn black_segment_lengths(
        length: f64,
        start_offset: f64,
        end_inset: f64,
        gaps: &[(f64, f64)],
    ) -> Vec<f64> {
        let usable_start = start_offset.max(0.0);
        let usable_end = (length - end_inset).max(usable_start);
        let mut cursor = usable_start;
        let mut segments = Vec::new();
        for (gap_start, gap_end) in gaps {
            segments.push(gap_start - cursor);
            cursor = *gap_end;
        }
        if usable_end > cursor {
            segments.push(usable_end - cursor);
        }
        segments
    }

    #[test]
    fn equal_black_segment_gap_intervals_keep_black_segments_equal() {
        let gaps = equal_black_segment_gap_intervals(10.0, 0.4, 0.6, 1.0, 1.5);
        assert!(!gaps.is_empty());

        let black_lengths = black_segment_lengths(10.0, 0.4, 0.6, &gaps);
        assert!(black_lengths.len() >= 2);
        for length in &black_lengths {
            approx_eq(*length, 1.0);
        }
    }

    #[test]
    fn equal_black_segment_gap_intervals_return_empty_when_too_short() {
        let gaps = equal_black_segment_gap_intervals(0.8, 0.0, 0.0, 1.0, 1.5);
        assert!(gaps.is_empty());
    }

    #[test]
    fn hashed_wedge_gap_intervals_respect_start_offset_and_end_inset() {
        let gaps = hashed_wedge_gap_intervals(18.0, VIEWER_BOND_STROKE * 2.0);
        assert!(!gaps.is_empty());
        assert!(gaps[0].0 > 0.0);
        assert!(gaps.last().unwrap().1 < 18.0);

        let start_offset = crate::HASH_WEDGE_GAP_START_OFFSET_CM.value() * 2.0;
        let end_inset = crate::HASH_WEDGE_GAP_END_INSET_CM.value() * 2.0;
        let black_lengths = black_segment_lengths(18.0, start_offset, end_inset, &gaps);
        for length in &black_lengths {
            approx_eq(*length, black_lengths[0]);
        }
    }

    #[test]
    fn boundary_line_join_candidate_rejects_far_backward_intersection() {
        let current = line_geometry(
            Point::new(0.0, 0.0),
            Vector::new(1.0, 0.0),
            Point::new(0.0, 0.0),
        );
        let other = line_geometry(
            Point::new(-5.0, -1.0),
            Vector::new(0.0, 1.0),
            Point::new(-5.0, 0.0),
        );

        assert!(boundary_line_join_candidate(&current, &other).is_none());
    }

    #[test]
    fn paired_boundary_line_join_points_avoids_trivial_assignment() {
        let current = [
            line_geometry(
                Point::new(0.0, 1.0),
                Vector::new(1.0, 0.0),
                Point::new(0.0, 0.0),
            ),
            line_geometry(
                Point::new(0.0, -1.0),
                Vector::new(1.0, 0.0),
                Point::new(0.0, 0.0),
            ),
        ];
        let other = [
            line_geometry(
                Point::new(0.0, -1.0),
                Vector::new(1.0, 1.0),
                Point::new(0.0, 0.0),
            ),
            line_geometry(
                Point::new(0.0, 1.0),
                Vector::new(1.0, -1.0),
                Point::new(0.0, 0.0),
            ),
        ];

        let (points, _) = paired_boundary_line_join_points(current, other).expect("join points");
        approx_eq(points[0].x, 2.0);
        approx_eq(points[0].y, 1.0);
        approx_eq(points[1].x, 2.0);
        approx_eq(points[1].y, -1.0);
    }
}
