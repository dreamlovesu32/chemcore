use super::*;

pub(super) struct BondBodyGeometry {
    pub actual_start: Point,
    pub actual_finish: Point,
    pub start: Point,
    pub finish: Point,
    pub begin_box: Option<RectBox>,
    pub end_box: Option<RectBox>,
    pub begin_has_label: bool,
    pub end_has_label: bool,
}

pub(super) fn bond_label_clipped_body_geometry(
    document: &ChemSemaDocument,
    object: &SceneObject,
    begin: &Node,
    end: &Node,
    bond: &Bond,
) -> Option<BondBodyGeometry> {
    let stroke_width = bond_stroke_width(document, object, bond);
    let actual_start = bond_endpoint_world(object, begin, bond, "begin");
    let actual_finish = bond_endpoint_world(object, end, bond, "end");
    let start = retreat_segment_endpoint(
        actual_start,
        actual_finish,
        external_connection_endpoint_retreat(document, begin, actual_start, actual_finish),
    );
    let finish = retreat_segment_endpoint(
        actual_finish,
        actual_start,
        external_connection_endpoint_retreat(document, end, actual_finish, actual_start),
    );
    let begin_box = label_box_world(begin, object);
    let end_box = label_box_world(end, object);
    let begin_polygons = label_clip_polygons_world(begin, object);
    let end_polygons = label_clip_polygons_world(end, object);
    let begin_has_label = begin
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text());
    let end_has_label = end
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text());
    let (begin_half_width, end_half_width) = bond_stereo_kind(bond)
        .map_or((stroke_width * 0.5, stroke_width * 0.5), |stereo| {
            wedge_endpoint_half_widths(bond, stereo, stroke_width)
        });
    let (start, finish) = clip_body_segment_out_of_label_geometry(
        start,
        finish,
        begin_box,
        &begin_polygons,
        begin_half_width,
        end_box,
        &end_polygons,
        end_half_width,
    )?;
    Some(BondBodyGeometry {
        actual_start,
        actual_finish,
        start,
        finish,
        begin_box,
        end_box,
        begin_has_label,
        end_has_label,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_bond(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
    let Some(geometry) = bond_label_clipped_body_geometry(document, object, begin, end, bond)
    else {
        return;
    };
    let BondBodyGeometry {
        actual_start,
        actual_finish,
        start,
        finish,
        begin_box,
        end_box,
        begin_has_label,
        end_has_label,
    } = geometry;

    let stereo = bond_stereo_kind(bond);
    if let Some(stereo) = stereo {
        let direction = Vector::new(finish.x - start.x, finish.y - start.y);
        if direction.x * direction.x + direction.y * direction.y <= EPSILON {
            return;
        }

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

    if bond.order == 1 && bond_main_line_pattern(bond) == crate::BondLinePattern::Wavy {
        render_wavy_bond(out, bond, start, finish, stroke, stroke_width, object_id);
        return;
    }

    if imported_cdxml_dative_bond(bond) {
        let Some(geometry) = dative_bond_geometry(start, finish, stroke_width) else {
            return;
        };
        render_fragment_line_with_profiles(
            out,
            document,
            object,
            contact_kernel,
            bonds,
            node_map,
            bond,
            start,
            geometry.shaft_end,
            begin_box,
            None,
            true,
            stroke,
            stroke_width,
            Vec::new(),
            bond.line_weights.main,
            object_id.clone(),
            true,
            true,
            false,
            false,
            contact_kernel.endpoint_profile(&bond.id, &bond.begin),
            None,
        );
        render_dative_bond_head(out, bond, geometry, stroke, object_id);
        return;
    }

    if bond.order == 2 {
        render_double_bond(
            out,
            document,
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
            document,
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
        document,
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
        line_pattern_dash_array_for_bond(bond, stroke_width, bond_main_line_pattern(bond)),
        bond.line_weights.main,
        object_id,
    );
}

fn external_connection_endpoint_retreat(
    document: &ChemSemaDocument,
    node: &Node,
    from: Point,
    to: Point,
) -> f64 {
    let Some(connection) = node.external_connection.as_ref() else {
        return 0.0;
    };
    if connection.connection_type == crate::ExternalConnectionType::Wavy {
        return 0.0;
    }
    let length = from.distance(to);
    if length <= EPSILON {
        return 0.0;
    }
    let line_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/lineWidth")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_BOND_STROKE);
    let label_size = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/labelSize")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    if connection.connection_type == crate::ExternalConnectionType::PolymerBead {
        return label_size * 0.75 + line_width * 2.0;
    }
    let radius_x = label_size * 0.375 + line_width;
    let radius_y = if matches!(
        connection.connection_type,
        crate::ExternalConnectionType::Residue
            | crate::ExternalConnectionType::Peptide
            | crate::ExternalConnectionType::Dna
            | crate::ExternalConnectionType::Rna
            | crate::ExternalConnectionType::Terminus
            | crate::ExternalConnectionType::Sulfide
    ) {
        radius_x * (2.0 / 3.0)
    } else {
        radius_x
    };
    let unit_x = (to.x - from.x) / length;
    let unit_y = (to.y - from.y) / length;
    1.0 / (unit_x.abs() / radius_x + unit_y.abs() / radius_y)
}

fn retreat_segment_endpoint(from: Point, to: Point, distance: f64) -> Point {
    let length = from.distance(to);
    if length <= EPSILON || distance <= 0.0 {
        return from;
    }
    let scale = (distance / length).min(1.0);
    Point::new(
        from.x + (to.x - from.x) * scale,
        from.y + (to.y - from.y) * scale,
    )
}

pub(super) fn imported_cdxml_dative_bond(bond: &Bond) -> bool {
    bond.meta
        .pointer("/import/cdxml/order")
        .and_then(JsonValue::as_str)
        .is_some_and(|order| order.eq_ignore_ascii_case("dative"))
}

struct DativeBondGeometry {
    shaft_end: Point,
    head_points: Vec<Point>,
}

fn dative_bond_geometry(start: Point, end: Point, stroke_width: f64) -> Option<DativeBondGeometry> {
    let axis = Vector::new(end.x - start.x, end.y - start.y);
    let length = axis.length();
    if length <= EPSILON || stroke_width <= EPSILON {
        return None;
    }
    let unit = axis.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let head_length = (stroke_width * 10.0).min(length * (2.0 / 3.0));
    let half_width = head_length * 0.25;
    let base = end.translated(unit.scaled(-head_length));
    let shaft_end = end.translated(unit.scaled(-head_length * 0.875));
    Some(DativeBondGeometry {
        shaft_end,
        head_points: vec![
            end,
            base.translated(normal.scaled(half_width)),
            shaft_end,
            base.translated(normal.scaled(-half_width)),
        ],
    })
}

fn render_dative_bond_head(
    out: &mut Vec<RenderPrimitive>,
    bond: &Bond,
    geometry: DativeBondGeometry,
    fill: &str,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polygon {
        role: RenderRole::DocumentBond,
        object_id,
        node_id: None,
        bond_id: Some(bond.id.clone()),
        points: geometry.head_points,
        fill: fill.to_string(),
        stroke: fill.to_string(),
        stroke_width: 0.0,
    });
}

fn bond_endpoint_world(object: &SceneObject, node: &Node, bond: &Bond, endpoint: &str) -> Point {
    bond.meta
        .pointer(&format!("/endpointAttachments/{endpoint}"))
        .and_then(|attachment| attachment.get("characterIndex"))
        .and_then(JsonValue::as_u64)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| attached_label_glyph_anchor_world(object, node, index))
        .unwrap_or_else(|| world_point(object, node))
}

#[allow(clippy::too_many_arguments)]
fn render_double_bond(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
            let shared_label_retreats = shared_parallel_label_retreats(
                object,
                node_map,
                bond,
                actual_start,
                actual_end,
                stroke_width,
                &[
                    (0.0, bond.line_weights.main),
                    (side * double_offset, outer_line_weight(bond, side)),
                ],
            );
            let (main_start, main_end) = apply_segment_endpoint_retreats(
                actual_start,
                actual_end,
                shared_label_retreats.0,
                shared_label_retreats.1,
            );
            render_fragment_line_with_profiles(
                out,
                document,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                main_start,
                main_end,
                begin_box,
                end_box,
                true,
                stroke,
                stroke_width,
                line_pattern_dash_array_for_bond(bond, stroke_width, bond_main_line_pattern(bond)),
                bond.line_weights.main,
                object_id.clone(),
                false,
                true,
                true,
                true,
                None,
                None,
            );
            render_outer_bond_lines(
                out,
                document,
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
                Some(shared_label_retreats),
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
                document,
                object,
                contact_kernel,
                bonds,
                node_map,
                bond,
                actual_start,
                actual_end,
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
fn shared_parallel_label_retreats(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    actual_start: Point,
    actual_end: Point,
    stroke_width: f64,
    lines: &[(f64, BondLineWeight)],
) -> (f64, f64) {
    let (normal_x, normal_y) = unit_normal(actual_start, actual_end);
    let begin_polygons = node_map
        .get(bond.begin.as_str())
        .map(|node| label_clip_polygons_world(node, object))
        .unwrap_or_default();
    let end_polygons = node_map
        .get(bond.end.as_str())
        .map(|node| label_clip_polygons_world(node, object))
        .unwrap_or_default();
    let begin_box = node_map
        .get(bond.begin.as_str())
        .and_then(|node| label_box_world(node, object));
    let end_box = node_map
        .get(bond.end.as_str())
        .and_then(|node| label_box_world(node, object));
    let mut shared_start_retreat: f64 = 0.0;
    let mut shared_end_retreat: f64 = 0.0;
    for (offset, weight) in lines {
        let line_start = Point::new(
            actual_start.x + normal_x * offset,
            actual_start.y + normal_y * offset,
        );
        let line_end = Point::new(
            actual_end.x + normal_x * offset,
            actual_end.y + normal_y * offset,
        );
        let half_width = line_weight_stroke_width_for_bond(bond, stroke_width, *weight) * 0.5;
        if let Some((start_retreat, end_retreat)) = body_segment_label_retreats(
            line_start,
            line_end,
            begin_box,
            &begin_polygons,
            half_width,
            end_box,
            &end_polygons,
            half_width,
        ) {
            shared_start_retreat = shared_start_retreat.max(start_retreat);
            shared_end_retreat = shared_end_retreat.max(end_retreat);
        }
    }
    (shared_start_retreat, shared_end_retreat)
}

#[allow(clippy::too_many_arguments)]
fn render_center_double_bond_lines(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    actual_start: Point,
    actual_end: Point,
    begin_box: Option<RectBox>,
    end_box: Option<RectBox>,
    begin_has_label: bool,
    end_has_label: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    double_offset: f64,
) {
    let (normal_x, normal_y) = unit_normal(actual_start, actual_end);
    let begin_polygons = label_clip_polygons_world(
        node_map
            .get(bond.begin.as_str())
            .copied()
            .expect("center double begin node must exist"),
        object,
    );
    let end_polygons = label_clip_polygons_world(
        node_map
            .get(bond.end.as_str())
            .copied()
            .expect("center double end node must exist"),
        object,
    );
    let mut shared_start_retreat: f64 = 0.0;
    let mut shared_end_retreat: f64 = 0.0;
    for (offset, weight) in [
        (-double_offset / 2.0, bond.line_weights.left),
        (double_offset / 2.0, bond.line_weights.right),
    ] {
        let line_start = Point::new(
            actual_start.x + normal_x * offset,
            actual_start.y + normal_y * offset,
        );
        let line_end = Point::new(
            actual_end.x + normal_x * offset,
            actual_end.y + normal_y * offset,
        );
        let half_width = line_weight_stroke_width_for_bond(bond, stroke_width, weight) * 0.5;
        if let Some((start_retreat, end_retreat)) = body_segment_label_retreats(
            line_start,
            line_end,
            begin_box,
            &begin_polygons,
            half_width,
            end_box,
            &end_polygons,
            half_width,
        ) {
            shared_start_retreat = shared_start_retreat.max(start_retreat);
            shared_end_retreat = shared_end_retreat.max(end_retreat);
        }
    }
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
        let raw_line_start = Point::new(
            actual_start.x + normal_x * offset,
            actual_start.y + normal_y * offset,
        );
        let raw_line_end = Point::new(
            actual_end.x + normal_x * offset,
            actual_end.y + normal_y * offset,
        );
        let (line_start, line_end) = apply_segment_endpoint_retreats(
            raw_line_start,
            raw_line_end,
            shared_start_retreat,
            shared_end_retreat,
        );
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
            document,
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
    document: &ChemSemaDocument,
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
    let triple_offset =
        triple_bond_offset_distance_for_bond(bond, actual_start, actual_end, stroke_width);

    render_fragment_line(
        out,
        document,
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
        line_pattern_dash_array_for_bond(bond, stroke_width, bond_main_line_pattern(bond)),
        bond.line_weights.main,
        object_id.clone(),
    );

    render_outer_bond_lines(
        out,
        document,
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
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_outer_bond_lines(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    contact_kernel: &MainBondContactKernel,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    _start: Point,
    _end: Point,
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
    shared_label_retreats: Option<(f64, f64)>,
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
    let (normal_x, normal_y) = unit_normal(_actual_start, _actual_end);

    for side in sides {
        let line_pattern = outer_line_pattern(bond, *side);
        let line_weight = outer_line_weight(bond, *side);
        let offset_start = Point::new(
            _actual_start.x + normal_x * offset_distance * *side,
            _actual_start.y + normal_y * offset_distance * *side,
        );
        let offset_end = Point::new(
            _actual_end.x + normal_x * offset_distance * *side,
            _actual_end.y + normal_y * offset_distance * *side,
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
        let (short_start, short_end) = shared_label_retreats.map_or(
            (short_start, short_end),
            |(start_retreat, end_retreat)| {
                apply_segment_endpoint_retreats(short_start, short_end, start_retreat, end_retreat)
            },
        );
        render_fragment_line_with_profiles(
            out,
            document,
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
            shared_label_retreats.is_none(),
            true,
            true,
            false,
            start_endpoint_profile,
            end_endpoint_profile,
        );
    }
}

fn wedge_endpoint_half_widths(
    bond: &Bond,
    stereo: BondStereoKind,
    stroke_width: f64,
) -> (f64, f64) {
    let tip_half_width = solid_wedge_tip_half_width(stroke_width);
    let wide_half_width = solid_wedge_half_width_for_bond(bond, stroke_width);
    match stereo {
        BondStereoKind::SolidWedgeBegin
        | BondStereoKind::HashedWedgeBegin
        | BondStereoKind::HollowWedgeBegin => (wide_half_width, tip_half_width),
        BondStereoKind::SolidWedgeEnd
        | BondStereoKind::HashedWedgeEnd
        | BondStereoKind::HollowWedgeEnd => (tip_half_width, wide_half_width),
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
    let amplitude = wavy_bond_amplitude_for_bond(bond, stroke_width)
        .min(length * 0.18)
        .max(EPSILON);
    let half_wave_count = ((length / amplitude).ceil() as usize).max(4);
    let drawn_length = ((half_wave_count.saturating_sub(1)) as f64 * amplitude).min(length);
    if drawn_length <= EPSILON {
        return;
    }
    let half_wave_step = drawn_length / half_wave_count as f64;
    let control = (amplitude * 0.552_284_749_830_793_6).min(half_wave_step * 0.5);
    let mut d = format!("M {:.4} {:.4}", start.x, start.y);
    let mut points = vec![start];
    for index in 0..half_wave_count {
        let segment_start = wavy_bond_point(start, unit, normal, half_wave_step, amplitude, index);
        let segment_end =
            wavy_bond_point(start, unit, normal, half_wave_step, amplitude, index + 1);
        let start_tangent = wavy_bond_tangent(unit, normal, index).scaled(control);
        let end_tangent = wavy_bond_tangent(unit, normal, index + 1).scaled(control);
        let control_1 = segment_start.translated(start_tangent);
        let control_2 = segment_end.translated(end_tangent.scaled(-1.0));
        d.push_str(&format!(
            " C {:.4},{:.4} {:.4},{:.4} {:.4},{:.4}",
            control_1.x, control_1.y, control_2.x, control_2.y, segment_end.x, segment_end.y
        ));
        points.push(segment_end);
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

fn wavy_bond_point(
    start: Point,
    unit: Vector,
    normal: Vector,
    step: f64,
    amplitude: f64,
    index: usize,
) -> Point {
    let side = match index % 4 {
        1 => 1.0,
        3 => -1.0,
        _ => 0.0,
    };
    start
        .translated(unit.scaled(step * index as f64))
        .translated(normal.scaled(amplitude * side))
}

fn wavy_bond_tangent(unit: Vector, normal: Vector, index: usize) -> Vector {
    match index % 4 {
        0 => normal,
        1 | 3 => unit,
        2 => normal.scaled(-1.0),
        _ => unreachable!(),
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

#[allow(clippy::too_many_arguments)]
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

#[cfg(test)]
mod dative_bond_tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn dative_head_uses_chemdraw_line_width_geometry() {
        let geometry = dative_bond_geometry(Point::new(20.0, 30.0), Point::new(50.0, 30.0), 1.0)
            .expect("dative geometry");
        assert_close(geometry.head_points[0].x, 50.0);
        assert_close(geometry.head_points[1].x, 40.0);
        assert_close(geometry.head_points[1].y, 32.5);
        assert_close(geometry.shaft_end.x, 41.25);
        assert_close(geometry.shaft_end.y, 30.0);
        assert_close(geometry.head_points[3].y, 27.5);
    }

    #[test]
    fn dative_head_caps_at_two_thirds_of_short_bond_length() {
        let geometry = dative_bond_geometry(Point::new(20.0, 30.0), Point::new(25.0, 30.0), 1.0)
            .expect("short dative geometry");
        assert_close(geometry.head_points[1].x, 25.0 - 10.0 / 3.0);
        assert_close(geometry.head_points[1].y, 30.0 + 5.0 / 6.0);
        assert_close(geometry.shaft_end.x, 25.0 - 35.0 / 12.0);
    }
}
