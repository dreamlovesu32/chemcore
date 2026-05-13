use super::*;

pub(super) fn world_point(object: &SceneObject, node: &Node) -> Point {
    Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    )
}

pub(super) fn label_box_world(node: &Node, object: &SceneObject) -> Option<RectBox> {
    let label = node.label.as_ref()?;
    let bbox = label.bbox()?;
    Some(RectBox {
        x1: bbox[0] + object.transform.translate[0],
        y1: bbox[1] + object.transform.translate[1],
        x2: bbox[2] + object.transform.translate[0],
        y2: bbox[3] + object.transform.translate[1],
    })
}

pub(super) fn label_polygons_world(node: &Node, object: &SceneObject) -> Vec<Vec<Point>> {
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

pub(super) fn segment_intersection_fraction(
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

pub(super) fn clip_point_out_of_polygons(
    start: Point,
    end: Point,
    polygons: &[Vec<Point>],
) -> Option<Point> {
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
    best_t.map(|t| {
        Point::new(
            start.x + (end.x - start.x) * t,
            start.y + (end.y - start.y) * t,
        )
    })
}

pub(super) fn advance_point_toward(point: Point, target: Point, distance: f64) -> Point {
    if distance <= EPSILON {
        return point;
    }
    let direction = Vector::new(target.x - point.x, target.y - point.y);
    let length = direction.length();
    if length <= EPSILON {
        return point;
    }
    let step = distance.min(length);
    let unit = direction.normalized();
    Point::new(point.x + unit.x * step, point.y + unit.y * step)
}

pub(super) fn clip_point_out_of_box(
    start: Point,
    end: Point,
    rect: Option<RectBox>,
    margin: f64,
) -> Point {
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

pub(super) fn clip_point_out_of_label_geometry(
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
        .map(|point| advance_point_toward(point, end, margin))
        .unwrap_or(start)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_fragment_line(
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
pub(super) fn render_fragment_line_with_profiles(
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
    let start_has_label = node_map
        .get(bond.begin.as_str())
        .and_then(|node| node.label.as_ref())
        .is_some_and(|label| label.has_visible_text());
    let end_has_label = node_map
        .get(bond.end.as_str())
        .and_then(|node| node.label.as_ref())
        .is_some_and(|label| label.has_visible_text());
    let (clipped_start, clipped_end) = if clip_against_label_geometry {
        let label_clip_margin = label_clip_margin_for_bond(bond, stroke_width);
        let clipped_start = clip_point_out_of_label_geometry(
            start,
            end,
            start_box,
            &start_polygons,
            label_clip_margin,
        );
        let clipped_end = clip_point_out_of_label_geometry(
            end,
            clipped_start,
            end_box,
            &end_polygons,
            label_clip_margin,
        );
        (clipped_start, clipped_end)
    } else {
        (start, end)
    };
    let hash_pattern_start = clipped_start;
    let hash_pattern_end = clipped_end;
    let mut start_retreat = contact_kernel.endpoint_retreat(&bond.id, &bond.begin);
    let mut end_retreat = contact_kernel.endpoint_retreat(&bond.id, &bond.end);
    if is_hash_bond(bond) && line_weight == BondLineWeight::Bold && !dash_array.is_empty() {
        let retreat = hash_contact_retreat_distance_for_bond(bond, stroke_width);
        if !start_has_label && endpoint_has_other_bond(bonds, bond, &bond.begin) {
            start_retreat = start_retreat.max(retreat);
        }
        if !end_has_label && endpoint_has_other_bond(bonds, bond, &bond.end) {
            end_retreat = end_retreat.max(retreat);
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
                line_weight_stroke_width_for_bond(bond, stroke_width, line_weight),
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
                    if is_hash_bond(bond) {
                        hash_pattern_start
                    } else {
                        clipped_start
                    },
                    if is_hash_bond(bond) {
                        hash_pattern_end
                    } else {
                        clipped_end
                    },
                    line_weight_stroke_width_for_bond(bond, stroke_width, line_weight),
                    stroke_width,
                )
            } else {
                dashed_bond_knockout_polygons(
                    clipped_start,
                    clipped_end,
                    line_weight_stroke_width_for_bond(bond, stroke_width, line_weight),
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
        line_weight_stroke_width_for_bond(bond, stroke_width, line_weight),
        dash_array,
        object_id,
    );
}
