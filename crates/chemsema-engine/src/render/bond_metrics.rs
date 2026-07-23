use super::*;

pub(super) fn unit_normal(start: Point, end: Point) -> (f64, f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length <= EPSILON {
        return (0.0, 0.0);
    }
    (-dy / length, dx / length)
}

pub(super) fn inset_bond_segment(
    start: Point,
    end: Point,
    inset_start: f64,
    inset_end: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return (start, end);
    }
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

pub(super) fn line_weight_stroke_width(stroke_width: f64, line_weight: BondLineWeight) -> f64 {
    if line_weight == BondLineWeight::Bold {
        (BOLD_BOND_WIDTH * (stroke_width / VIEWER_BOND_STROKE)).max(stroke_width)
    } else {
        stroke_width
    }
}

pub(super) fn line_weight_stroke_width_for_bond(
    bond: &Bond,
    stroke_width: f64,
    line_weight: BondLineWeight,
) -> f64 {
    if line_weight == BondLineWeight::Bold {
        bond.bold_width
            .unwrap_or_else(|| line_weight_stroke_width(stroke_width, line_weight))
            .max(stroke_width)
    } else {
        stroke_width
    }
}

pub(super) fn wavy_bond_amplitude_for_bond(bond: &Bond, stroke_width: f64) -> f64 {
    bond.bold_width
        .unwrap_or_else(|| line_weight_stroke_width(stroke_width, BondLineWeight::Bold))
        .max(stroke_width)
        * 0.5
}

pub(super) fn hash_contact_retreat_distance_for_bond(bond: &Bond, stroke_width: f64) -> f64 {
    let scale = stroke_width / VIEWER_BOND_STROKE;
    bond.hash_spacing
        .filter(|spacing| *spacing > EPSILON)
        .unwrap_or(HASH_BLACK_SEGMENT_LENGTH * scale + HASH_TARGET_GAP_LENGTH * scale)
}

pub(super) fn multi_bond_inner_gap(
    bond: Option<&Bond>,
    start: Point,
    end: Point,
    stroke_width: f64,
) -> f64 {
    let spacing_ratio = bond
        .and_then(|bond| bond.bond_spacing)
        .map(|spacing| spacing / 100.0)
        .unwrap_or(DEFAULT_MULTI_BOND_CENTER_SPACING_RATIO);
    (start.distance(end) * spacing_ratio - stroke_width).max(stroke_width * 1.5)
}

pub(super) fn double_bond_center_distance_for_weights(
    start: Point,
    end: Point,
    stroke_width: f64,
    first_weight: BondLineWeight,
    second_weight: BondLineWeight,
) -> f64 {
    let first_width = line_weight_stroke_width(stroke_width, first_weight);
    let second_width = line_weight_stroke_width(stroke_width, second_weight);
    multi_bond_inner_gap(None, start, end, stroke_width) + 0.5 * (first_width + second_width)
}

pub(super) fn double_bond_center_distance_for_bond_weights(
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    first_weight: BondLineWeight,
    second_weight: BondLineWeight,
) -> f64 {
    let first_width = line_weight_stroke_width_for_bond(bond, stroke_width, first_weight);
    let second_width = line_weight_stroke_width_for_bond(bond, stroke_width, second_weight);
    multi_bond_inner_gap(Some(bond), start, end, stroke_width) + 0.5 * (first_width + second_width)
}

pub(super) fn double_bond_offset_distance(start: Point, end: Point, stroke_width: f64) -> f64 {
    double_bond_center_distance_for_weights(
        start,
        end,
        stroke_width,
        BondLineWeight::Normal,
        BondLineWeight::Normal,
    )
}

pub(super) fn triple_bond_offset_distance(start: Point, end: Point, stroke_width: f64) -> f64 {
    let spacing_ratio = DEFAULT_MULTI_BOND_CENTER_SPACING_RATIO;
    (start.distance(end) * spacing_ratio).max(stroke_width * 1.5)
}

pub(super) fn solid_wedge_half_width_for_bond(bond: &Bond, stroke_width: f64) -> f64 {
    bond.wedge_width
        .unwrap_or_else(|| solid_wedge_width_for_legacy_bond_template(bond, stroke_width))
        .max(stroke_width)
        * 0.5
}

pub(super) fn solid_wedge_half_width(stroke_width: f64) -> f64 {
    crate::SOLID_WEDGE_WIDTH_PT.value().max(stroke_width) * 0.5
}

pub(super) fn solid_wedge_tip_half_width(stroke_width: f64) -> f64 {
    stroke_width * 0.5
}

pub(super) fn document_margin_width_for_bond(
    document: &ChemSemaDocument,
    bond: &Bond,
    stroke_width: f64,
) -> f64 {
    bond.margin_width
        .or_else(|| {
            document
                .document
                .meta
                .pointer("/import/cdxml/defaults/marginWidth")
                .and_then(serde_json::Value::as_f64)
        })
        .or_else(|| document.style.defaults.get("marginWidth").copied())
        .unwrap_or_else(|| margin_width_for_legacy_bond_template(bond, stroke_width))
}

pub(super) fn solid_wedge_width_for_legacy_bond_template(bond: &Bond, stroke_width: f64) -> f64 {
    if let Some(bold_width) = bond.bold_width {
        bold_width * crate::WEDGE_BOLD_WIDTH_MULTIPLIER
    } else if is_acs_document_1996_bond_template(bond, stroke_width) {
        2.0 * crate::WEDGE_BOLD_WIDTH_MULTIPLIER
    } else {
        BOLD_BOND_WIDTH * crate::WEDGE_BOLD_WIDTH_MULTIPLIER
    }
}

pub(super) fn margin_width_for_legacy_bond_template(bond: &Bond, stroke_width: f64) -> f64 {
    if is_acs_document_1996_bond_template(bond, stroke_width) {
        crate::ACS_BOND_MARGIN_WIDTH_PT.value()
    } else {
        crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()
    }
}

pub(super) fn is_acs_document_1996_bond_template(bond: &Bond, stroke_width: f64) -> bool {
    let bold_width = bond.bold_width.unwrap_or(BOLD_BOND_WIDTH);
    (stroke_width - 0.6).abs() <= 0.01
        && (bold_width - 2.0).abs() <= 0.05
        && bond
            .hash_spacing
            .is_none_or(|spacing| (spacing - 2.5).abs() <= 0.05)
        && bond
            .bond_spacing
            .is_none_or(|spacing| (spacing - 18.0).abs() <= 0.05)
}

pub(super) fn equal_black_segment_gap_intervals(
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

fn chemdraw_dashed_bond_stripe_count(
    length: f64,
    stripe_length: f64,
    target_gap_length: f64,
) -> usize {
    let stripe_length = stripe_length.max(EPSILON);
    let target_gap_length = target_gap_length.max(EPSILON);
    let period = stripe_length + target_gap_length;
    // ChemDraw derives the count from the clipped visible length, then divides
    // that length into equal black/gap intervals which start and end in black.
    (((length + EPSILON) / period).floor() as usize + 1).max(1)
}

pub(super) fn chemdraw_dashed_bond_gap_intervals(
    length: f64,
    stripe_length: f64,
    target_gap_length: f64,
) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let stripe_count = chemdraw_dashed_bond_stripe_count(length, stripe_length, target_gap_length);
    if stripe_count <= 1 {
        return Vec::new();
    }

    let gap_count = stripe_count - 1;
    let interval_length = length / (stripe_count * 2 - 1) as f64;
    let mut intervals = Vec::with_capacity(gap_count);
    for index in 0..gap_count {
        let gap_start = (index * 2 + 1) as f64 * interval_length;
        let gap_end = (index * 2 + 2) as f64 * interval_length;
        intervals.push((gap_start, gap_end));
    }
    intervals
}

pub(super) fn dashed_bond_segment_polygons_with_profiles(
    start: Point,
    end: Point,
    stroke_width: f64,
    dash_array: &[f64],
    start_profile: Option<&[Point]>,
    end_profile: Option<&[Point]>,
) -> Vec<Vec<Point>> {
    let stripe_length = dash_array
        .first()
        .copied()
        .filter(|value| *value > EPSILON)
        .unwrap_or(crate::DEFAULT_HASH_SPACING_PT.value());
    let target_gap_length = dash_array
        .get(1)
        .copied()
        .filter(|value| *value > EPSILON)
        .unwrap_or(stripe_length);
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let start_inset = endpoint_profile_inward_extent(start_profile, start, unit);
    let end_inset = endpoint_profile_inward_extent(end_profile, end, Vector::new(-unit.x, -unit.y));
    dashed_segment_polygons_for_gap_intervals_with_profiles(
        start,
        end,
        stroke_width * 0.5,
        &chemdraw_dashed_bond_gap_intervals_with_endpoint_insets(
            length,
            stripe_length,
            target_gap_length,
            start_inset,
            end_inset,
        ),
        start_profile,
        end_profile,
    )
}

fn endpoint_profile_inward_extent(
    profile: Option<&[Point]>,
    endpoint: Point,
    inward: Vector,
) -> f64 {
    profile
        .into_iter()
        .flatten()
        .map(|point| {
            vector_dot(
                Vector::new(point.x - endpoint.x, point.y - endpoint.y),
                inward,
            )
        })
        .fold(0.0, f64::max)
}

fn chemdraw_dashed_bond_gap_intervals_with_endpoint_insets(
    length: f64,
    stripe_length: f64,
    target_gap_length: f64,
    start_inset: f64,
    end_inset: f64,
) -> Vec<(f64, f64)> {
    if start_inset <= EPSILON && end_inset <= EPSILON {
        return chemdraw_dashed_bond_gap_intervals(length, stripe_length, target_gap_length);
    }
    let usable_length = (length - start_inset - end_inset).max(0.0);
    let stripe_count =
        chemdraw_dashed_bond_stripe_count(usable_length, stripe_length, target_gap_length);
    if stripe_count <= 1 {
        return Vec::new();
    }
    let interval_length = usable_length / (stripe_count * 2 - 1) as f64;
    (0..stripe_count - 1)
        .map(|index| {
            (
                start_inset + (index * 2 + 1) as f64 * interval_length,
                start_inset + (index * 2 + 2) as f64 * interval_length,
            )
        })
        .collect()
}

pub(super) fn hash_bond_segment_polygons(
    start: Point,
    end: Point,
    visual_width: f64,
    pattern_width: f64,
) -> Vec<Vec<Point>> {
    let length = start.distance(end);
    if length <= EPSILON {
        return Vec::new();
    }
    let scale = pattern_width / VIEWER_BOND_STROKE;
    let gaps = equal_black_segment_gap_intervals(
        length,
        0.0,
        0.0,
        HASH_BLACK_SEGMENT_LENGTH * scale,
        HASH_TARGET_GAP_LENGTH * scale,
    );
    dashed_segment_polygons_for_gap_intervals(start, end, visual_width * 0.5, &gaps)
}

fn dashed_segment_polygons_for_gap_intervals(
    start: Point,
    end: Point,
    half_width: f64,
    gaps: &[(f64, f64)],
) -> Vec<Vec<Point>> {
    dashed_segment_polygons_for_gap_intervals_with_profiles(
        start, end, half_width, gaps, None, None,
    )
}

fn dashed_segment_polygons_for_gap_intervals_with_profiles(
    start: Point,
    end: Point,
    half_width: f64,
    gaps: &[(f64, f64)],
    start_profile: Option<&[Point]>,
    end_profile: Option<&[Point]>,
) -> Vec<Vec<Point>> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let mut segments = Vec::new();
    let mut cursor = 0.0;
    for (gap_start, gap_end) in gaps {
        if *gap_start > cursor + EPSILON {
            segments.push((cursor, *gap_start));
        }
        cursor = (*gap_end).max(cursor);
    }
    if length > cursor + EPSILON {
        segments.push((cursor, length));
    }

    let segment_count = segments.len();
    segments
        .into_iter()
        .enumerate()
        .filter(|(_, (segment_start, segment_end))| segment_end > segment_start)
        .map(|(index, (segment_start, segment_end))| {
            let from = Point::new(
                start.x + unit.x * segment_start,
                start.y + unit.y * segment_start,
            );
            let to = Point::new(
                start.x + unit.x * segment_end,
                start.y + unit.y * segment_end,
            );
            let default_start_profile = vec![
                Point::new(
                    from.x + normal.x * half_width,
                    from.y + normal.y * half_width,
                ),
                Point::new(
                    from.x - normal.x * half_width,
                    from.y - normal.y * half_width,
                ),
            ];
            let default_end_profile = vec![
                Point::new(to.x + normal.x * half_width, to.y + normal.y * half_width),
                Point::new(to.x - normal.x * half_width, to.y - normal.y * half_width),
            ];
            let start_profile = if index == 0 {
                endpoint_profile_global(
                    start_profile.map(ToOwned::to_owned),
                    false,
                    default_start_profile,
                )
            } else {
                default_start_profile
            };
            let end_profile = if index + 1 == segment_count {
                endpoint_profile_global(
                    end_profile.map(ToOwned::to_owned),
                    true,
                    default_end_profile,
                )
            } else {
                default_end_profile
            };
            bond_polygon_from_endpoint_profiles(start_profile, end_profile)
        })
        .collect()
}

pub(super) fn hashed_wedge_gap_intervals(
    length: f64,
    stroke_width: f64,
    bond: &Bond,
) -> Vec<(f64, f64)> {
    if length <= EPSILON {
        return Vec::new();
    }
    let stripe_length = stroke_width.min(length).max(EPSILON);
    let minimum_pitch = bond
        .hash_spacing
        .filter(|spacing| *spacing > EPSILON)
        .unwrap_or(crate::DEFAULT_HASH_SPACING_PT.value());

    // ChemDraw keeps every wedged-hash stripe exactly one LineWidth thick
    // along the bond axis.  The first and last stripes touch the two ends.  It
    // chooses the greatest stripe count whose center pitch is no smaller than
    // HashSpacing, then distributes that pitch evenly over the final body.
    let stripe_count = (((length - stripe_length + EPSILON) / minimum_pitch)
        .floor()
        .max(0.0) as usize)
        + 1;
    if stripe_count <= 1 {
        return Vec::new();
    }

    let pitch = (length - stripe_length) / (stripe_count - 1) as f64;
    (0..stripe_count - 1)
        .map(|index| {
            let stripe_start = index as f64 * pitch;
            (stripe_start + stripe_length, stripe_start + pitch)
        })
        .filter(|(gap_start, gap_end)| *gap_end > *gap_start + EPSILON)
        .collect()
}

pub(super) fn side_double_placement(bond: &Bond) -> Option<DoubleBondPlacement> {
    match bond.double.as_ref().map(|double| double.placement) {
        Some(DoubleBondPlacement::Left) => Some(DoubleBondPlacement::Left),
        Some(DoubleBondPlacement::Right) => Some(DoubleBondPlacement::Right),
        _ => None,
    }
}

pub(super) fn line_pattern_dash_array_for_bond(
    bond: &Bond,
    stroke_width: f64,
    pattern: BondLinePattern,
) -> Vec<f64> {
    if pattern == BondLinePattern::Dashed {
        let spacing = bond
            .hash_spacing
            .filter(|spacing| *spacing > crate::EPSILON)
            .unwrap_or(crate::DEFAULT_HASH_SPACING_PT.value());
        let segment = spacing.max(stroke_width * 0.75);
        vec![segment, segment]
    } else {
        Vec::new()
    }
}

pub(super) fn outer_line_pattern(bond: &Bond, side: f64) -> BondLinePattern {
    if side > 0.0 {
        bond.line_styles.left
    } else {
        bond.line_styles.right
    }
}

pub(super) fn outer_line_weight(bond: &Bond, side: f64) -> BondLineWeight {
    if side > 0.0 {
        bond.line_weights.left
    } else {
        bond.line_weights.right
    }
}

pub(super) fn fragment_node_degree(bonds: &[Bond], node_id: &str) -> usize {
    bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .count()
}

pub(super) fn fragment_outer_bond_offset_for_side(
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
    if (placement == DoubleBondPlacement::Left && side > 0.0)
        || (placement == DoubleBondPlacement::Right && side < 0.0)
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

pub(super) fn outer_bond_candidate_sides(bond: &Bond) -> Vec<f64> {
    if bond.order >= 3 {
        return vec![1.0, -1.0];
    }
    match side_double_placement(bond) {
        Some(DoubleBondPlacement::Left) => vec![1.0],
        Some(DoubleBondPlacement::Right) => vec![-1.0],
        _ => Vec::new(),
    }
}

pub(super) fn outer_bond_half_width_for_side(bond: &Bond, side: f64, stroke_width: f64) -> f64 {
    line_weight_stroke_width_for_bond(bond, stroke_width, outer_line_weight(bond, side)) * 0.5
}

pub(super) fn outer_bond_offset_line_for_endpoint(
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

pub(super) fn outer_bond_boundary_line_pair_for_endpoint(
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

pub(super) fn centered_double_line_boundary_pair_for_endpoint(
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
    let half_width = line_weight_stroke_width_for_bond(bond, stroke_width, line_weight) * 0.5;
    let boundaries = boundary_lines_from_endpoint(center.point, center.direction, half_width)?;
    Some((boundaries, center))
}

pub(super) fn boundary_lines_with_profile_terminals(
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

pub(super) fn main_bond_drawn_boundary_pair_for_endpoint(
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

pub(super) fn outer_bond_drawn_boundary_pairs_for_endpoint(
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

pub(super) fn main_bond_candidate_sides(bond: &Bond) -> Vec<f64> {
    if bond.line_weights.main == BondLineWeight::Bold
        && bond.line_styles.main == BondLinePattern::Solid
    {
        vec![1.0, -1.0]
    } else {
        vec![0.0]
    }
}

pub(super) fn main_bond_boundary_line_for_endpoint(
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
        line_weight_stroke_width_for_bond(bond, stroke_width, BondLineWeight::Bold) * 0.5
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

pub(super) fn outer_bond_endpoint_profile_for_side(
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

    let current_stroke_width = stroke_width.max(bond.stroke_width);
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
        let other_stroke_width = neighbor_bond_stroke_width(other_bond, stroke_width);
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

pub(super) fn side_double_outer_endpoint_can_match_main_length(
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    shared_node_id: &str,
    side: f64,
    stroke_width: f64,
) -> bool {
    if side_double_placement(bond).is_none()
        || outer_line_pattern(bond, side) != BondLinePattern::Solid
    {
        return false;
    }
    let current_stroke_width = stroke_width.max(bond.stroke_width);
    let Some((_, current_center)) = outer_bond_boundary_line_pair_for_endpoint(
        object,
        node_map,
        bond,
        shared_node_id,
        side,
        current_stroke_width,
    ) else {
        return false;
    };
    let Some(current_local_side) = line_geometry_local_side(current_center) else {
        return false;
    };

    for other_bond in bonds {
        if other_bond.id == bond.id {
            continue;
        }
        if other_bond.begin != shared_node_id && other_bond.end != shared_node_id {
            continue;
        }
        if other_bond.order >= 2 {
            return false;
        }
        let Some(other_axis) =
            bond_axis_line_for_endpoint(object, node_map, other_bond, shared_node_id)
        else {
            return false;
        };
        let Some(contact_side) = main_contact_side(current_center.direction, other_axis.direction)
        else {
            return false;
        };
        if (contact_side - current_local_side).abs() <= 1.0e-6 {
            return false;
        }
        if side_double_outer_line_requires_acute_retreat(
            object,
            node_map,
            other_bond,
            shared_node_id,
            current_center,
            current_local_side,
        ) {
            return false;
        }
    }
    true
}

pub(super) fn line_geometry_local_side(line: LineGeometry) -> Option<f64> {
    let normal = Vector::new(-line.direction.y, line.direction.x);
    let offset = Vector::new(line.point.x - line.shared.x, line.point.y - line.shared.y);
    let local_side = vector_dot(offset, normal).signum();
    if local_side.abs() <= EPSILON {
        None
    } else {
        Some(local_side)
    }
}

pub(super) fn line_geometries_share_side(first: LineGeometry, second: LineGeometry) -> bool {
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

pub(super) fn side_double_outer_line_requires_acute_retreat(
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

pub(super) fn bond_axis_line_for_endpoint(
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

pub(super) fn center_double_endpoint_profile_for_line_side(
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
    let current_stroke_width = stroke_width.max(bond.stroke_width);
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
        let other_stroke_width = neighbor_bond_stroke_width(other_bond, stroke_width);
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

pub(super) fn line_intersection(
    point: Point,
    direction: Vector,
    other_point: Point,
    other_direction: Vector,
) -> Option<Point> {
    line_intersection_with_parameters(point, direction, other_point, other_direction)
        .map(|value| value.0)
}

pub(super) fn line_intersection_with_parameters(
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
pub(super) struct ContactEntry {
    pub(super) direction: Vector,
    pub(super) side: f64,
    pub(super) side_value: f64,
}

pub(super) fn contact_entries(directions: &[Vector], normal: Vector) -> Vec<ContactEntry> {
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

pub(super) fn far_side_contact_line_point(
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

pub(super) fn wide_contact_directions(
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

pub(super) fn is_wide_contact_candidate(bond: &Bond) -> bool {
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
pub(super) enum BondStereoKind {
    SolidWedgeBegin,
    SolidWedgeEnd,
    HashedWedgeBegin,
    HashedWedgeEnd,
    HollowWedgeBegin,
    HollowWedgeEnd,
}

pub(super) fn bond_stereo_kind(bond: &Bond) -> Option<BondStereoKind> {
    if let Some(stereo) = bond.stereo.as_ref() {
        return match (stereo.kind.as_str(), stereo.wide_end.as_str()) {
            ("solid-wedge", "begin") => Some(BondStereoKind::SolidWedgeBegin),
            ("solid-wedge", "end") => Some(BondStereoKind::SolidWedgeEnd),
            ("hashed-wedge", "begin") => Some(BondStereoKind::HashedWedgeBegin),
            ("hashed-wedge", "end") => Some(BondStereoKind::HashedWedgeEnd),
            ("hollow-wedge", "begin") => Some(BondStereoKind::HollowWedgeBegin),
            ("hollow-wedge", "end") => Some(BondStereoKind::HollowWedgeEnd),
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
        "HollowWedgeBegin" => Some(BondStereoKind::HollowWedgeEnd),
        "HollowWedgeEnd" => Some(BondStereoKind::HollowWedgeBegin),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        chemdraw_dashed_bond_gap_intervals_with_endpoint_insets,
        hash_contact_retreat_distance_for_bond, hashed_wedge_gap_intervals,
        line_pattern_dash_array_for_bond,
    };
    use crate::{
        Bond, BondLinePattern, BondLineStyles, BondLineWeight, BondLineWeights, DEFAULT_BOND_STROKE,
    };
    use serde_json::Value;

    fn test_bond(hash_spacing: Option<f64>) -> Bond {
        Bond {
            id: "b1".to_string(),
            begin: "n1".to_string(),
            end: "n2".to_string(),
            order: 1,
            properties: Default::default(),
            double: None,
            stereo: None,
            stroke_width: DEFAULT_BOND_STROKE,
            stroke: None,
            bold_width: None,
            wedge_width: None,
            label_clip_margin: None,
            hash_spacing,
            bond_spacing: None,
            margin_width: None,
            line_styles: BondLineStyles::default(),
            line_weights: BondLineWeights {
                main: BondLineWeight::Normal,
                left: BondLineWeight::Normal,
                right: BondLineWeight::Normal,
            },
            meta: Value::Null,
        }
    }

    #[test]
    fn dashed_bond_dash_array_uses_explicit_hash_spacing() {
        let dash_array =
            line_pattern_dash_array_for_bond(&test_bond(Some(2.7)), 1.0, BondLinePattern::Dashed);
        assert_eq!(dash_array, vec![2.7, 2.7]);
    }

    #[test]
    fn dashed_bond_dash_array_defaults_to_chem_draw_hash_spacing() {
        let dash_array =
            line_pattern_dash_array_for_bond(&test_bond(None), 1.0, BondLinePattern::Dashed);
        assert_eq!(
            dash_array,
            vec![
                crate::DEFAULT_HASH_SPACING_PT.value(),
                crate::DEFAULT_HASH_SPACING_PT.value()
            ]
        );
    }

    #[test]
    fn hashed_wedge_count_switches_at_line_width_plus_whole_hash_pitches() {
        let default = test_bond(Some(2.7));
        assert_eq!(
            hashed_wedge_gap_intervals(14.49, 1.0, &default).len() + 1,
            5
        );
        assert_eq!(
            hashed_wedge_gap_intervals(14.50, 1.0, &default).len() + 1,
            6
        );

        let acs = test_bond(Some(2.5));
        assert_eq!(hashed_wedge_gap_intervals(13.09, 0.6, &acs).len() + 1, 5);
        assert_eq!(hashed_wedge_gap_intervals(13.10, 0.6, &acs).len() + 1, 6);
    }

    #[test]
    fn hashed_wedge_contact_retreat_keeps_the_recorded_hash_spacing() {
        let retreat = hash_contact_retreat_distance_for_bond(&test_bond(Some(2.7)), 1.0);
        assert!((retreat - 2.7).abs() < 1.0e-9, "{retreat}");
    }

    #[test]
    fn hashed_wedge_keeps_line_width_stripes_and_distributes_the_remaining_gap() {
        let bond = test_bond(Some(2.7));
        let gaps = hashed_wedge_gap_intervals(14.0, 1.0, &bond);
        assert_eq!(gaps.len() + 1, 5);
        let pitch = 13.0 / 4.0;
        for (index, (gap_start, gap_end)) in gaps.iter().enumerate() {
            assert!((*gap_start - (index as f64 * pitch + 1.0)).abs() < 1.0e-9);
            assert!((*gap_end - ((index + 1) as f64 * pitch)).abs() < 1.0e-9);
        }
    }

    #[test]
    fn dashed_bond_endpoint_miters_are_absorbed_by_terminal_stripes() {
        let gaps =
            chemdraw_dashed_bond_gap_intervals_with_endpoint_insets(14.4, 2.5, 2.5, 1.2, 1.2);
        assert_eq!(gaps.len(), 2, "{gaps:?}");
        assert!((gaps[0].0 - 3.6).abs() < 1.0e-9, "{gaps:?}");
        assert!((gaps[0].1 - 6.0).abs() < 1.0e-9, "{gaps:?}");
        assert!((gaps[1].0 - 8.4).abs() < 1.0e-9, "{gaps:?}");
        assert!((gaps[1].1 - 10.8).abs() < 1.0e-9, "{gaps:?}");
    }
}
