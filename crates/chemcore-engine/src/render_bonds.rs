use super::*;

pub(super) fn render_fragment_bond(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    stroke: &str,
    object_id: Option<String>,
) {
    let Some(begin) = node_map.get(bond.begin.as_str()).copied() else {
        return;
    };
    let Some(end) = node_map.get(bond.end.as_str()).copied() else {
        return;
    };
    let stroke_width = bond_stroke_width(document, object, bond);
    let actual_start = world_point(object, begin);
    let actual_finish = world_point(object, end);
    let mut start = actual_start;
    let mut finish = actual_finish;
    let begin_box = label_box_world(begin, object);
    let end_box = label_box_world(end, object);
    let begin_polygons = label_polygons_world(begin, object);
    let end_polygons = label_polygons_world(end, object);
    let begin_has_label = begin
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text());
    let end_has_label = end
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text());

    let label_clip_margin = label_clip_margin_for_bond(bond, stroke_width);
    start = clip_point_out_of_label_geometry(
        start,
        finish,
        begin_box,
        &begin_polygons,
        label_clip_margin,
    );
    finish =
        clip_point_out_of_label_geometry(finish, start, end_box, &end_polygons, label_clip_margin);

    if let Some(stereo) = bond_stereo_kind(bond) {
        render_stereo_bond(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            stereo,
            start,
            finish,
            begin_has_label,
            end_has_label,
            stroke,
            stroke_width,
            object_id,
        );
        return;
    }

    if bond.order == 2 {
        render_double_bond(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            start,
            finish,
            actual_start,
            actual_finish,
            begin_box,
            end_box,
            begin_has_label,
            end_has_label,
            stroke,
            stroke_width,
            object_id,
        );
        return;
    }

    if bond.order >= 3 {
        render_triple_bond(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            start,
            finish,
            actual_start,
            actual_finish,
            begin_box,
            end_box,
            begin_has_label,
            end_has_label,
            stroke,
            stroke_width,
            object_id,
        );
        return;
    }

    render_fragment_line(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        finish,
        begin_box,
        end_box,
        true,
        stroke,
        stroke_width,
        line_pattern_dash_array(bond.line_styles.main),
        bond.line_weights.main,
        object_id,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_double_bond(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    actual_start: Point,
    actual_end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let side_mode = bond.double.as_ref().map(|double| double.placement);

    match side_mode {
        Some(DoubleBondPlacement::Left) | Some(DoubleBondPlacement::Right) => {
            let side = if side_mode == Some(DoubleBondPlacement::Left) {
                1.0
            } else {
                -1.0
            };
            let double_offset = double_bond_center_distance_for_bond_weights(
                bond,
                actual_start,
                actual_end,
                stroke_width,
                bond.line_weights.main,
                outer_line_weight(bond, side),
            );
            render_fragment_line(
                out,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                start,
                end,
                begin_box,
                end_box,
                true,
                stroke,
                stroke_width,
                line_pattern_dash_array(bond.line_styles.main),
                bond.line_weights.main,
                object_id.clone(),
            );
            render_outer_bond_lines(
                out,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                start,
                end,
                begin_box,
                end_box,
                actual_start,
                actual_end,
                begin_has_label,
                end_has_label,
                stroke,
                stroke_width,
                object_id,
                &[side],
                double_offset,
            );
        }
        _ => {
            let double_offset = double_bond_center_distance_for_bond_weights(
                bond,
                actual_start,
                actual_end,
                stroke_width,
                bond.line_weights.left,
                bond.line_weights.right,
            );
            render_center_double_bond_lines(
                out,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                start,
                end,
                begin_box,
                end_box,
                begin_has_label,
                end_has_label,
                stroke,
                stroke_width,
                object_id,
                double_offset,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_center_double_bond_lines(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    double_offset: f64,
) {
    let (normal_x, normal_y) = unit_normal(start, end);
    for (line_side, offset, pattern, weight) in [
        (
            -1.0,
            -double_offset / 2.0,
            bond.line_styles.left,
            bond.line_weights.left,
        ),
        (
            1.0,
            double_offset / 2.0,
            bond.line_styles.right,
            bond.line_weights.right,
        ),
    ] {
        let line_start = Point::new(start.x + normal_x * offset, start.y + normal_y * offset);
        let line_end = Point::new(end.x + normal_x * offset, end.y + normal_y * offset);
        let start_endpoint_profile = center_double_endpoint_profile_for_line_side(
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            &bond.begin,
            line_side,
            double_offset * 0.5,
            stroke_width,
            weight,
        );
        let end_endpoint_profile = center_double_endpoint_profile_for_line_side(
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            &bond.end,
            line_side,
            double_offset * 0.5,
            stroke_width,
            weight,
        );
        render_fragment_line_with_profiles(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            line_start,
            line_end,
            begin_box,
            end_box,
            false,
            stroke,
            stroke_width,
            line_pattern_dash_array(pattern),
            weight,
            object_id.clone(),
            false,
            !begin_has_label,
            !end_has_label,
            false,
            start_endpoint_profile,
            end_endpoint_profile,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_triple_bond(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    actual_start: Point,
    actual_end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let triple_offset = triple_bond_offset_distance(actual_start, actual_end, stroke_width);

    render_fragment_line(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        begin_box,
        end_box,
        true,
        stroke,
        stroke_width,
        line_pattern_dash_array(bond.line_styles.main),
        bond.line_weights.main,
        object_id.clone(),
    );

    render_outer_bond_lines(
        out,
        object,
        contact_kernel,
        bonds,
        node_map,
        bond,
        start,
        end,
        begin_box,
        end_box,
        actual_start,
        actual_end,
        begin_has_label,
        end_has_label,
        stroke,
        stroke_width,
        object_id,
        &[1.0, -1.0],
        triple_offset,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_outer_bond_lines(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    start: Point,
    end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    actual_start: Point,
    actual_end: Point,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    sides: &[f64],
    offset_distance: f64,
) {
    let length = actual_start.distance(actual_end);
    let is_side_double = side_double_placement(bond).is_some();
    let side_inset = if is_side_double {
        offset_distance * (3.0f64).sqrt() / 3.0
    } else {
        (DOUBLE_BOND_SIDE_INSET * (stroke_width / VIEWER_BOND_STROKE))
            .max(length * DOUBLE_BOND_SIDE_INSET_RATIO)
    };
    let begin_terminal = fragment_node_degree(bonds, &bond.begin) <= 1;
    let end_terminal = fragment_node_degree(bonds, &bond.end) <= 1;
    let (normal_x, normal_y) = unit_normal(start, end);

    for side in sides {
        let line_pattern = outer_line_pattern(bond, *side);
        let line_weight = outer_line_weight(bond, *side);
        let offset_start = Point::new(
            start.x + normal_x * offset_distance * *side,
            start.y + normal_y * offset_distance * *side,
        );
        let offset_end = Point::new(
            end.x + normal_x * offset_distance * *side,
            end.y + normal_y * offset_distance * *side,
        );
        let start_endpoint_profile = outer_bond_endpoint_profile_for_side(
            object,
            bonds,
            node_map,
            bond,
            &bond.begin,
            *side,
            stroke_width,
        );
        let end_endpoint_profile = outer_bond_endpoint_profile_for_side(
            object,
            bonds,
            node_map,
            bond,
            &bond.end,
            *side,
            stroke_width,
        );
        let (short_start, short_end) = inset_bond_segment(
            offset_start,
            offset_end,
            if start_endpoint_profile.is_some() || begin_has_label || begin_terminal {
                0.0
            } else {
                side_inset
            },
            if end_endpoint_profile.is_some() || end_has_label || end_terminal {
                0.0
            } else {
                side_inset
            },
        );
        render_fragment_line_with_profiles(
            out,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            short_start,
            short_end,
            begin_box,
            end_box,
            false,
            stroke,
            stroke_width,
            line_pattern_dash_array(line_pattern),
            line_weight,
            object_id.clone(),
            true,
            true,
            true,
            false,
            start_endpoint_profile,
            end_endpoint_profile,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_stereo_bond(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    stereo: BondStereoKind,
    start: Point,
    end: Point,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    match stereo {
        BondStereoKind::SolidWedgeEnd => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.end,
                start,
                end,
                if end_has_label {
                    SOLID_WEDGE_END_INSET
                } else {
                    0.0
                },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.end),
            );
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
        }
        BondStereoKind::SolidWedgeBegin => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.begin,
                end,
                start,
                if begin_has_label {
                    SOLID_WEDGE_END_INSET
                } else {
                    0.0
                },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.begin),
            );
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
        }
        BondStereoKind::HashedWedgeEnd => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.end,
                start,
                end,
                if end_has_label {
                    SOLID_WEDGE_END_INSET
                } else {
                    0.0
                },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.end),
            );
            push_bond_polygon(
                out,
                &bond.id,
                points.clone(),
                stroke,
                stroke,
                0.0,
                object_id.clone(),
            );
            for knockout in
                compute_fragment_hashed_wedge_knockout_polygons(&points, stroke_width, bond)
            {
                push_knockout_polygon(out, knockout, object_id.clone());
            }
        }
        BondStereoKind::HashedWedgeBegin => {
            let points = compute_fragment_solid_wedge_points(
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                &bond.begin,
                end,
                start,
                if begin_has_label {
                    SOLID_WEDGE_END_INSET
                } else {
                    0.0
                },
                stroke_width,
                !contact_kernel.uses_endpoint(&bond.id, &bond.begin),
            );
            push_bond_polygon(
                out,
                &bond.id,
                points.clone(),
                stroke,
                stroke,
                0.0,
                object_id.clone(),
            );
            for knockout in
                compute_fragment_hashed_wedge_knockout_polygons(&points, stroke_width, bond)
            {
                push_knockout_polygon(out, knockout, object_id.clone());
            }
        }
    }
}

pub(super) fn compute_solid_wedge_points(
    start: Point,
    end: Point,
    end_inset: f64,
    wide_contact_directions: Vec<Vector>,
    stroke_width: f64,
    wide_half_width: f64,
) -> Vec<Point> {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    let width = wide_half_width;
    let tip_plus = Point::new(
        start.x + normal.x * tip_half_width,
        start.y + normal.y * tip_half_width,
    );
    let tip_minus = Point::new(
        start.x - normal.x * tip_half_width,
        start.y - normal.y * tip_half_width,
    );
    let cap_inset = end_inset.min(length * 0.22);
    let cap_center = Point::new(end.x - unit.x * cap_inset, end.y - unit.y * cap_inset);
    let cap_plus = Point::new(
        cap_center.x + normal.x * width,
        cap_center.y + normal.y * width,
    );
    let cap_minus = Point::new(
        cap_center.x - normal.x * width,
        cap_center.y - normal.y * width,
    );

    let contacts = contact_entries(&wide_contact_directions, normal);
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
                tip_plus,
                Vector::new(cap_plus.x - tip_plus.x, cap_plus.y - tip_plus.y),
                far_side_contact_line_point(end, plus.direction, start, stroke_width),
                plus.direction,
            )
            .unwrap_or(cap_plus);
            let minus_intersection = line_intersection(
                tip_minus,
                Vector::new(cap_minus.x - tip_minus.x, cap_minus.y - tip_minus.y),
                far_side_contact_line_point(end, minus.direction, start, stroke_width),
                minus.direction,
            )
            .unwrap_or(cap_minus);
            return vec![tip_plus, plus_intersection, minus_intersection, tip_minus];
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
                tip_plus,
                Vector::new(cap_plus.x - tip_plus.x, cap_plus.y - tip_plus.y),
                far_side_contact_line_point(end, contact.direction, start, stroke_width),
                contact.direction,
            )
            .unwrap_or(cap_plus);
            let minus_intersection = line_intersection(
                tip_minus,
                Vector::new(cap_minus.x - tip_minus.x, cap_minus.y - tip_minus.y),
                far_side_contact_line_point(end, contact.direction, start, stroke_width),
                contact.direction,
            )
            .unwrap_or(cap_minus);
            return vec![tip_plus, plus_intersection, minus_intersection, tip_minus];
        }
    }

    vec![tip_plus, cap_plus, cap_minus, tip_minus]
}

#[allow(clippy::too_many_arguments)]
fn compute_fragment_solid_wedge_points(
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    wide_node_id: &str,
    start: Point,
    end: Point,
    end_inset: f64,
    stroke_width: f64,
    allow_endpoint_contacts: bool,
) -> Vec<Point> {
    let narrow_node_id = if wide_node_id == bond.begin {
        bond.end.as_str()
    } else {
        bond.begin.as_str()
    };
    let start_retreat = contact_kernel.endpoint_retreat(&bond.id, narrow_node_id);
    let mut end_retreat = if is_hashed_wedge_bond(bond) {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, wide_node_id)
    };
    if is_hashed_wedge_bond(bond) && allow_endpoint_contacts {
        end_retreat = end_retreat.max(
            endpoint_retreat_against_center_double_outer_line(
                object,
                bonds,
                node_map,
                bond,
                wide_node_id,
                end,
                Vector::new(start.x - end.x, start.y - end.y).normalized(),
                solid_wedge_half_width_for_bond(bond, stroke_width),
                stroke_width,
            )
            .unwrap_or(0.0),
        );
    }
    let (start, end) = apply_segment_endpoint_retreats(start, end, start_retreat, end_retreat);
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    let width = solid_wedge_half_width_for_bond(bond, stroke_width);
    let tip_plus = Point::new(
        start.x + normal.x * tip_half_width,
        start.y + normal.y * tip_half_width,
    );
    let tip_minus = Point::new(
        start.x - normal.x * tip_half_width,
        start.y - normal.y * tip_half_width,
    );
    let cap_inset = end_inset.min(length * 0.22);
    let cap_center = Point::new(end.x - unit.x * cap_inset, end.y - unit.y * cap_inset);
    let cap_plus = Point::new(
        cap_center.x + normal.x * width,
        cap_center.y + normal.y * width,
    );
    let cap_minus = Point::new(
        cap_center.x - normal.x * width,
        cap_center.y - normal.y * width,
    );
    let start_profile = endpoint_profile_global(
        if start_retreat > EPSILON {
            None
        } else {
            contact_kernel.endpoint_profile(&bond.id, narrow_node_id)
        },
        false,
        vec![tip_plus, tip_minus],
    );

    if end_retreat <= EPSILON {
        if let Some(profile) = contact_kernel.endpoint_profile(&bond.id, wide_node_id) {
            let end_profile =
                endpoint_profile_global(Some(profile), true, vec![cap_plus, cap_minus]);
            return bond_polygon_from_endpoint_profiles(start_profile, end_profile);
        }
    }

    if is_hashed_wedge_bond(bond) {
        return bond_polygon_from_endpoint_profiles(start_profile, vec![cap_plus, cap_minus]);
    }

    if allow_endpoint_contacts {
        if let Some((join_plus, join_minus)) = solid_wedge_cap_points(
            object,
            bonds,
            node_map,
            bond,
            wide_node_id,
            tip_plus,
            tip_minus,
            end,
            cap_plus,
            cap_minus,
            stroke_width,
        ) {
            return bond_polygon_from_endpoint_profiles(start_profile, vec![join_plus, join_minus]);
        }
    }

    let points = compute_solid_wedge_points(
        start,
        end,
        end_inset,
        if allow_endpoint_contacts {
            wide_contact_directions(object, bonds, node_map, bond, wide_node_id)
        } else {
            Vec::new()
        },
        stroke_width,
        solid_wedge_half_width_for_bond(bond, stroke_width),
    );
    if points.len() == 4 {
        return bond_polygon_from_endpoint_profiles(start_profile, vec![points[1], points[2]]);
    }
    points
}

fn compute_fragment_hashed_wedge_knockout_polygons(
    polygon: &[Point],
    stroke_width: f64,
    bond: &Bond,
) -> Vec<Vec<Point>> {
    if polygon.len() != 4 {
        return Vec::new();
    }
    let tip_plus = polygon[0];
    let cap_plus = polygon[1];
    let cap_minus = polygon[2];
    let tip_minus = polygon[3];
    let tip_center = midpoint(tip_plus, tip_minus);
    let cap_center = midpoint(cap_plus, cap_minus);
    let direction = Vector::new(cap_center.x - tip_center.x, cap_center.y - tip_center.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }

    let mut knockouts = Vec::new();
    for (gap_start, gap_end) in hashed_wedge_gap_intervals(length, stroke_width, bond) {
        let t0 = (gap_start / length).clamp(0.0, 1.0);
        let t1 = (gap_end / length).clamp(0.0, 1.0);
        let mut top_start = lerp_point(tip_plus, cap_plus, t0);
        let mut top_end = lerp_point(tip_plus, cap_plus, t1);
        let mut bottom_end = lerp_point(tip_minus, cap_minus, t1);
        let mut bottom_start = lerp_point(tip_minus, cap_minus, t0);
        let overdraw = HASH_WEDGE_EDGE_OVERDRAW * (stroke_width / VIEWER_BOND_STROKE);
        for (upper, lower) in [
            (&mut top_start, &mut bottom_start),
            (&mut top_end, &mut bottom_end),
        ] {
            let mid = Point::new((upper.x + lower.x) * 0.5, (upper.y + lower.y) * 0.5);
            let upper_out = Vector::new(upper.x - mid.x, upper.y - mid.y).normalized();
            let lower_out = Vector::new(lower.x - mid.x, lower.y - mid.y).normalized();
            upper.x += upper_out.x * overdraw;
            upper.y += upper_out.y * overdraw;
            lower.x += lower_out.x * overdraw;
            lower.y += lower_out.y * overdraw;
        }
        knockouts.push(compact_polygon_points(vec![
            top_start,
            top_end,
            bottom_end,
            bottom_start,
        ]));
    }
    knockouts
}
