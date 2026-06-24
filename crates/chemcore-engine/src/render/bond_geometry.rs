use super::*;

pub(super) fn endpoint_profile_global(
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

pub(super) fn bond_polygon_from_endpoint_profiles(
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

pub(super) fn midpoint(first: Point, second: Point) -> Point {
    Point::new((first.x + second.x) * 0.5, (first.y + second.y) * 0.5)
}

pub(super) fn compact_polygon_points(points: Vec<Point>) -> Vec<Point> {
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

pub(super) fn polygon_area_signed(points: &[Point]) -> f64 {
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

pub(super) fn axis_angle(axis: Vector) -> f64 {
    axis.y.atan2(axis.x)
}

pub(super) fn vector_dot(first: Vector, second: Vector) -> f64 {
    first.x * second.x + first.y * second.y
}

pub(super) fn vector_cross(first: Vector, second: Vector) -> f64 {
    first.x * second.y - first.y * second.x
}

pub(super) fn compute_bold_bond_points(
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
    let length = direction.length();
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width =
        line_weight_stroke_width_for_bond(bond, stroke_width, BondLineWeight::Bold) / 2.0;
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

pub(super) fn is_hash_bond(bond: &Bond) -> bool {
    bond.order == 1
        && bond.line_styles.main == BondLinePattern::Dashed
        && bond.line_weights.main == BondLineWeight::Bold
}

pub(super) fn is_hashed_wedge_bond(bond: &Bond) -> bool {
    matches!(
        bond_stereo_kind(bond),
        Some(BondStereoKind::HashedWedgeBegin | BondStereoKind::HashedWedgeEnd)
    )
}

pub(super) fn is_hash_contact_obstacle(bond: &Bond) -> bool {
    is_hash_bond(bond) || is_hashed_wedge_bond(bond)
}

pub(super) fn endpoint_has_other_bond(bonds: &[Bond], bond: &Bond, node_id: &str) -> bool {
    bonds
        .iter()
        .any(|other| other.id != bond.id && (other.begin == node_id || other.end == node_id))
}

pub(super) fn apply_segment_endpoint_retreats(
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
    let desired_total = start_retreat.max(0.0) + end_retreat.max(0.0);
    let scale = if desired_total > length && desired_total > EPSILON {
        length / desired_total
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

pub(super) fn bold_band_cap_points(
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
pub(super) fn solid_wedge_cap_points(
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
pub(super) fn solid_wedge_edge_join_point(
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
                let other_stroke_width = neighbor_bond_stroke_width(other_bond, stroke_width);
                if endpoint_distance
                    > (stroke_width.max(other_stroke_width) * 4.5).max(max_join_distance)
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
                neighbor_bond_stroke_width(other_bond, stroke_width),
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
pub(super) fn bold_edge_join_point(
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
                neighbor_bond_stroke_width(other_bond, stroke_width),
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

pub(super) fn compute_hashed_wedge_segments(
    start: Point,
    end: Point,
    stroke_width: f64,
) -> Vec<(Point, Point, f64)> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
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
            crate::HASH_WEDGE_INITIAL_HALF_WIDTH_PT.value()
        } else {
            crate::HASH_WEDGE_PROGRESS_BASE_HALF_WIDTH_PT.value()
                + progress * crate::HASH_WEDGE_PROGRESS_HALF_WIDTH_RANGE_PT.value()
        } * scale;
        let center = Point::new(start.x + unit.x * dist, start.y + unit.y * dist);
        let segment_width = if index == 0 {
            crate::HASH_WEDGE_INITIAL_SEGMENT_WIDTH_PT.value()
        } else {
            crate::HASH_WEDGE_SEGMENT_WIDTH_PT.value()
        } * scale;
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
    segments
}

pub(super) fn lerp_point(from: Point, to: Point, t: f64) -> Point {
    Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
}

pub(super) fn has_joinable_main_line(bond: &Bond) -> bool {
    if bond.stereo.is_some() || bond.line_weights.main != BondLineWeight::Normal {
        return false;
    }
    if bond.order == 1 || bond.order >= 3 {
        return true;
    }
    bond.order == 2 && side_double_placement(bond).is_some()
}

pub(super) fn is_joinable_main_line_render(
    bond: &Bond,
    allow_bold_contacts: bool,
    line_weight: BondLineWeight,
) -> bool {
    allow_bold_contacts && line_weight == BondLineWeight::Normal && has_joinable_main_line(bond)
}

pub(super) fn neighbor_bond_stroke_width(bond: &Bond, fallback: f64) -> f64 {
    if bond.stroke_width > 0.0 {
        bond.stroke_width
    } else {
        fallback
    }
}

pub(super) fn boundary_lines_from_endpoint(
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

pub(super) fn main_line_boundary_lines_for_endpoint(
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

pub(super) fn wide_boundary_line_pair_for_endpoint(
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
pub(super) struct BoundaryJoinCandidate {
    point: Point,
    score: f64,
    t: f64,
    u: f64,
}

pub(super) fn boundary_line_join_candidate(
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
        -(current.offset_distance * 4.0).max(crate::BOUNDARY_JOIN_MIN_BACKTRACK_PT.value());
    let min_other =
        -(other.offset_distance * 4.0).max(crate::BOUNDARY_JOIN_MIN_BACKTRACK_PT.value());
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

pub(super) fn is_trivial_boundary_assignment(candidates: [BoundaryJoinCandidate; 2]) -> bool {
    candidates
        .iter()
        .all(|candidate| candidate.t.abs() <= 1.0e-4 && candidate.u.abs() <= 1.0e-4)
}

pub(super) fn paired_boundary_line_join_points(
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

pub(super) fn extended_boundary_line_join_points(
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

pub(super) fn wide_endpoint_join_points_against_main_lines(
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
        let other_stroke_width = neighbor_bond_stroke_width(other_bond, stroke_width);
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

pub(super) fn main_line_join_points_against_wide_bonds(
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
        let other_stroke_width = neighbor_bond_stroke_width(other_bond, stroke_width);
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

pub(super) fn solid_joinable_main_line(bond: &Bond) -> bool {
    has_joinable_main_line(bond)
        && bond.line_weights.main == BondLineWeight::Normal
        && bond.line_styles.main == BondLinePattern::Solid
}

pub(super) fn main_line_far_boundary_for_wide_bond(
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

pub(super) fn bold_main_line_join_polygon(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    endpoint: Point,
    forward: Vector,
    stroke_width: f64,
) -> Option<Vec<Point>> {
    let half_width =
        line_weight_stroke_width_for_bond(bond, stroke_width, BondLineWeight::Bold) * 0.5;
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
        let other_stroke_width = neighbor_bond_stroke_width(other_bond, stroke_width);
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
pub(super) fn main_line_polygon_points(
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

pub(super) fn simple_main_line_polygon_points(
    start: Point,
    end: Point,
    stroke_width: f64,
) -> Option<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let half_width = stroke_width * 0.5;
    Some(bond_polygon_from_endpoint_profiles(
        vec![
            Point::new(
                start.x + normal.x * half_width,
                start.y + normal.y * half_width,
            ),
            Point::new(
                start.x - normal.x * half_width,
                start.y - normal.y * half_width,
            ),
        ],
        vec![
            Point::new(end.x - normal.x * half_width, end.y - normal.y * half_width),
            Point::new(end.x + normal.x * half_width, end.y + normal.y * half_width),
        ],
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn main_bond_center_line_for_endpoint(
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

pub(super) fn main_bond_cap_line_for_endpoint(
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

pub(super) fn wide_boundary_lines_for_endpoint(
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
        let half_width =
            line_weight_stroke_width_for_bond(bond, stroke_width, BondLineWeight::Bold) * 0.5;
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
    let cap_half_width = solid_wedge_half_width_for_bond(bond, stroke_width);
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
