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
    let stroke = bond.stroke.as_deref().unwrap_or(stroke);
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
    let Some((clipped_start, clipped_finish)) = clip_segment_out_of_label_geometry(
        start,
        finish,
        begin_box,
        &begin_polygons,
        end_box,
        &end_polygons,
        label_clip_margin,
    ) else {
        return;
    };
    start = clipped_start;
    finish = clipped_finish;

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

    if bond.order == 1 && bond.line_styles.main == crate::BondLinePattern::Wavy {
        render_wavy_bond(out, bond, start, finish, stroke, stroke_width, object_id);
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
        line_pattern_dash_array_for_bond(bond, stroke_width, bond.line_styles.main),
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
                line_pattern_dash_array_for_bond(bond, stroke_width, bond.line_styles.main),
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
            line_pattern_dash_array_for_bond(bond, stroke_width, pattern),
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
        line_pattern_dash_array_for_bond(bond, stroke_width, bond.line_styles.main),
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
    _actual_start: Point,
    _actual_end: Point,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    sides: &[f64],
    offset_distance: f64,
) {
    let length = _actual_start.distance(_actual_end);
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
        let start_can_match_main_length = is_side_double
            && side_double_outer_endpoint_can_match_main_length(
                object,
                bonds,
                node_map,
                bond,
                &bond.begin,
                *side,
                stroke_width,
            );
        let end_can_match_main_length = is_side_double
            && side_double_outer_endpoint_can_match_main_length(
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
            if start_endpoint_profile.is_some()
                || begin_has_label
                || begin_terminal
                || start_can_match_main_length
            {
                0.0
            } else {
                side_inset
            },
            if end_endpoint_profile.is_some()
                || end_has_label
                || end_terminal
                || end_can_match_main_length
            {
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
            line_pattern_dash_array_for_bond(bond, stroke_width, line_pattern),
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
                begin_has_label,
                end_has_label,
                !end_has_label && !contact_kernel.uses_endpoint(&bond.id, &bond.end),
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
                end_has_label,
                begin_has_label,
                !begin_has_label && !contact_kernel.uses_endpoint(&bond.id, &bond.begin),
            );
            push_bond_polygon(out, &bond.id, points, stroke, stroke, 0.0, object_id);
        }
        BondStereoKind::HashedWedgeEnd => {
            let points = compute_fragment_hashed_wedge_points(
                bonds,
                bond,
                &bond.begin,
                &bond.end,
                begin_has_label,
                end_has_label,
                start,
                end,
                if end_has_label {
                    SOLID_WEDGE_END_INSET
                } else {
                    0.0
                },
                stroke_width,
            );
            for stripe in compute_fragment_hashed_wedge_stripe_polygons(&points, stroke_width, bond)
            {
                push_bond_filled_path(out, &bond.id, stripe, stroke, object_id.clone());
            }
        }
        BondStereoKind::HashedWedgeBegin => {
            let points = compute_fragment_hashed_wedge_points(
                bonds,
                bond,
                &bond.end,
                &bond.begin,
                end_has_label,
                begin_has_label,
                end,
                start,
                if begin_has_label {
                    SOLID_WEDGE_END_INSET
                } else {
                    0.0
                },
                stroke_width,
            );
            for stripe in compute_fragment_hashed_wedge_stripe_polygons(&points, stroke_width, bond)
            {
                push_bond_filled_path(out, &bond.id, stripe, stroke, object_id.clone());
            }
        }
        BondStereoKind::HollowWedgeEnd => {
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
                begin_has_label,
                end_has_label,
                !end_has_label && !contact_kernel.uses_endpoint(&bond.id, &bond.end),
            );
            push_hollow_wedge(out, &bond.id, points, stroke, stroke_width, object_id);
        }
        BondStereoKind::HollowWedgeBegin => {
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
                end_has_label,
                begin_has_label,
                !begin_has_label && !contact_kernel.uses_endpoint(&bond.id, &bond.begin),
            );
            push_hollow_wedge(out, &bond.id, points, stroke, stroke_width, object_id);
        }
    }
}

fn render_wavy_bond(
    out: &mut Vec<RenderPrimitive>,
    bond: &Bond,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let amplitude = (1.875 * (stroke_width / VIEWER_BOND_STROKE)).clamp(0.8, length * 0.18);
    let mut half_wave_count = ((length / amplitude).round() as usize).max(4);
    if half_wave_count % 2 != 0 {
        half_wave_count += 1;
    }
    let half_wave_step = length / half_wave_count as f64;
    let mut d = format!("M {:.4} {:.4}", start.x, start.y);
    let mut points = vec![start];
    for index in 0..half_wave_count {
        let t = half_wave_step * (index + 1) as f64;
        let endpoint_side = match (index + 1) % 4 {
            1 => 1.0,
            3 => -1.0,
            _ => 0.0,
        };
        let end_point = Point::new(
            start.x + unit.x * t + normal.x * amplitude * endpoint_side,
            start.y + unit.y * t + normal.y * amplitude * endpoint_side,
        );
        let sweep = match (index + 1) % 4 {
            1 | 2 => 0,
            _ => 1,
        };
        d.push_str(&format!(
            " A {:.4},{:.4} 90 0,{} {:.4} {:.4}",
            amplitude, amplitude, sweep, end_point.x, end_point.y
        ));
        points.push(end_point);
    }
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentBond,
        object_id,
        bond_id: Some(bond.id.clone()),
        d,
        points,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array: Vec::new(),
        line_cap: None,
        line_join: Some("bevel".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
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
    narrow_has_label: bool,
    wide_has_label: bool,
    allow_endpoint_contacts: bool,
) -> Vec<Point> {
    let narrow_node_id = if wide_node_id == bond.begin {
        bond.end.as_str()
    } else {
        bond.begin.as_str()
    };
    let start_retreat = if narrow_has_label {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, narrow_node_id)
    };
    let end_retreat = if wide_has_label {
        0.0
    } else {
        contact_kernel.endpoint_retreat(&bond.id, wide_node_id)
    };
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
        if narrow_has_label || start_retreat > EPSILON {
            None
        } else {
            contact_kernel.endpoint_profile(&bond.id, narrow_node_id)
        },
        false,
        vec![tip_plus, tip_minus],
    );

    if !wide_has_label && end_retreat <= EPSILON {
        if let Some(profile) = contact_kernel.endpoint_profile(&bond.id, wide_node_id) {
            let end_profile =
                endpoint_profile_global(Some(profile), true, vec![cap_plus, cap_minus]);
            return bond_polygon_from_endpoint_profiles(start_profile, end_profile);
        }
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

fn compute_fragment_hashed_wedge_points(
    bonds: &[Bond],
    bond: &Bond,
    start_node_id: &str,
    end_node_id: &str,
    start_has_label: bool,
    end_has_label: bool,
    start: Point,
    end: Point,
    end_inset: f64,
    stroke_width: f64,
) -> Vec<Point> {
    let wide_half_width = solid_wedge_half_width_for_bond(bond, stroke_width);
    let retreat = hash_contact_retreat_distance_for_bond(bond, stroke_width);
    let start_retreat = if !start_has_label && endpoint_has_other_bond(bonds, bond, start_node_id) {
        retreat
    } else {
        0.0
    };
    let end_retreat = if !end_has_label && endpoint_has_other_bond(bonds, bond, end_node_id) {
        retreat
    } else {
        0.0
    };
    compute_plain_wedge_trapezoid(
        start,
        end,
        start_retreat,
        end_retreat,
        end_inset,
        stroke_width,
        wide_half_width,
    )
}

fn compute_plain_wedge_trapezoid(
    start: Point,
    end: Point,
    start_retreat: f64,
    end_retreat: f64,
    end_inset: f64,
    stroke_width: f64,
    wide_half_width: f64,
) -> Vec<Point> {
    let (start, end) = apply_segment_endpoint_retreats(start, end, start_retreat, end_retreat);
    let direction = Vector::new(end.x - start.x, end.y - start.y);
    let length = direction.length();
    if length <= EPSILON {
        return Vec::new();
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
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
        cap_center.x + normal.x * wide_half_width,
        cap_center.y + normal.y * wide_half_width,
    );
    let cap_minus = Point::new(
        cap_center.x - normal.x * wide_half_width,
        cap_center.y - normal.y * wide_half_width,
    );
    vec![tip_plus, cap_plus, cap_minus, tip_minus]
}

fn compute_fragment_hashed_wedge_stripe_polygons(
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

    let mut stripes = Vec::new();
    let mut cursor = 0.0;
    for (gap_start, gap_end) in hashed_wedge_gap_intervals(length, stroke_width, bond) {
        push_hashed_wedge_stripe(&mut stripes, polygon, cursor, gap_start, length);
        cursor = gap_end;
    }
    push_hashed_wedge_stripe(&mut stripes, polygon, cursor, length, length);
    stripes
}

fn push_hashed_wedge_stripe(
    stripes: &mut Vec<Vec<Point>>,
    polygon: &[Point],
    start: f64,
    end: f64,
    length: f64,
) {
    if end <= start + EPSILON || length <= EPSILON {
        return;
    }
    let t0 = (start / length).clamp(0.0, 1.0);
    let t1 = (end / length).clamp(0.0, 1.0);
    if t1 <= t0 + EPSILON {
        return;
    }
    let tip_plus = polygon[0];
    let cap_plus = polygon[1];
    let cap_minus = polygon[2];
    let tip_minus = polygon[3];
    stripes.push(compact_polygon_points(vec![
        lerp_point(tip_plus, cap_plus, t0),
        lerp_point(tip_plus, cap_plus, t1),
        lerp_point(tip_minus, cap_minus, t1),
        lerp_point(tip_minus, cap_minus, t0),
    ]));
}

fn push_hollow_wedge(
    out: &mut Vec<RenderPrimitive>,
    bond_id: &str,
    points: Vec<Point>,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let points = compact_polygon_points(points);
    if points.len() != 4 || polygon_area_signed(&points).abs() <= 1.0e-4 {
        return;
    }
    let mut d = format!("M {:.4} {:.4}", points[0].x, points[0].y);
    for point in points.iter().skip(1) {
        d.push_str(&format!(" L {:.4} {:.4}", point.x, point.y));
    }
    d.push_str(" Z");
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentBond,
        object_id,
        bond_id: Some(bond_id.to_string()),
        d,
        points,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array: Vec::new(),
        line_cap: Some("butt".to_string()),
        line_join: Some("miter".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
}

fn push_bond_filled_path(
    out: &mut Vec<RenderPrimitive>,
    bond_id: &str,
    points: Vec<Point>,
    fill: &str,
    object_id: Option<String>,
) {
    let points = compact_polygon_points(points);
    if points.len() < 3 || polygon_area_signed(&points).abs() <= 1.0e-4 {
        return;
    }

    let mut d = format!("M {:.4} {:.4}", points[0].x, points[0].y);
    for point in points.iter().skip(1) {
        d.push_str(&format!(" L {:.4} {:.4}", point.x, point.y));
    }
    d.push_str(" Z");

    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentBond,
        object_id,
        node_id: None,
        bond_id: Some(bond_id.to_string()),
        d,
        points,
        fill: fill.to_string(),
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
        rotate: 0.0,
        rotate_center: None,
    });
}
