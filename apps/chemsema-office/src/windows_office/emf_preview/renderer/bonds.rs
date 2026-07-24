use super::*;

pub(super) fn preview_bond_context(payload: &OleObjectPayload) -> Option<PreviewBondContext> {
    let document = parse_document_json(&payload.chemsema_document_json).ok()?;
    Some(preview_bond_context_from_document(&document))
}

pub(super) fn preview_label_context(payload: &OleObjectPayload) -> Option<PreviewLabelContext> {
    let document = parse_document_json(&payload.chemsema_document_json).ok()?;
    Some(preview_label_context_from_document(&document))
}

pub(super) fn preview_label_context_from_document(
    document: &ChemSemaDocument,
) -> PreviewLabelContext {
    let mut infos = BTreeMap::new();
    for object in document
        .scene_objects()
        .into_iter()
        .filter(|object| object.visible && object.object_type == "molecule")
    {
        let Some(fragment) = preview_molecule_fragment(document, object) else {
            continue;
        };
        let node_map: BTreeMap<&str, &chemsema_engine::Node> = fragment
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for bond in &fragment.bonds {
            adjacency
                .entry(bond.begin.as_str())
                .or_default()
                .push(bond.end.as_str());
            adjacency
                .entry(bond.end.as_str())
                .or_default()
                .push(bond.begin.as_str());
        }
        let components = preview_fragment_components(&node_map, &adjacency);
        for component in components {
            for node_id in &component {
                let Some(node) = node_map.get(node_id.as_str()).copied() else {
                    continue;
                };
                let Some(label) = node.label.as_ref().filter(|label| label.has_visible_text())
                else {
                    continue;
                };
                let Some(label_box) = preview_label_world_box(object, label) else {
                    continue;
                };
                infos.insert(
                    (*node_id).to_string(),
                    PreviewLabelInfo {
                        layout: label.layout.clone(),
                        world_box: Some(label_box),
                        simple_single_run: preview_label_is_simple_single_run(label),
                        line_count: if !label.line_runs.is_empty() {
                            label.line_runs.len()
                        } else {
                            label.text.lines().count().max(1)
                        },
                    },
                );
            }
        }
    }
    PreviewLabelContext { infos }
}

pub(super) fn preview_bond_context_from_document(
    document: &ChemSemaDocument,
) -> PreviewBondContext {
    let mut infos = BTreeMap::new();
    for object in document
        .scene_objects()
        .into_iter()
        .filter(|object| object.visible && object.object_type == "molecule")
    {
        let Some(fragment) = preview_molecule_fragment(document, object) else {
            continue;
        };
        let node_map: BTreeMap<&str, &chemsema_engine::Node> = fragment
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let mut incident: BTreeMap<&str, Vec<&Bond>> = BTreeMap::new();
        for bond in &fragment.bonds {
            incident.entry(bond.begin.as_str()).or_default().push(bond);
            incident.entry(bond.end.as_str()).or_default().push(bond);
        }
        for bond in &fragment.bonds {
            let Some(begin) = node_map.get(bond.begin.as_str()).copied() else {
                continue;
            };
            let Some(end) = node_map.get(bond.end.as_str()).copied() else {
                continue;
            };
            let axis = preview_bond_axis_from_nodes(object, begin.point(), end.point());
            let Some(axis) = axis else {
                continue;
            };
            let begin_world =
                preview_transform_scene_point(object, begin.position[0], begin.position[1]);
            let end_world = preview_transform_scene_point(object, end.position[0], end.position[1]);
            let allow_pen = preview_bond_is_pen_family(bond);
            let start_has_label = begin
                .label
                .as_ref()
                .is_some_and(|label| label.has_visible_text());
            let end_has_label = end
                .label
                .as_ref()
                .is_some_and(|label| label.has_visible_text());
            infos.insert(
                bond.id.clone(),
                PreviewBondInfo {
                    axis,
                    line_width: if bond.stroke_width > 0.0 {
                        bond.stroke_width
                    } else {
                        document
                            .style
                            .defaults
                            .get("lineWidth")
                            .copied()
                            .unwrap_or(DEFAULT_BOND_STROKE)
                    },
                    hashed_wedge_wide_projection: bond.stereo.as_ref().and_then(|stereo| {
                        if stereo.kind != "hashed-wedge" {
                            return None;
                        }
                        Some(if stereo.wide_end == bond.begin {
                            begin_world.x * axis.x + begin_world.y * axis.y
                        } else {
                            end_world.x * axis.x + end_world.y * axis.y
                        })
                    }),
                    allow_pen,
                    order: bond.order,
                    start_projection: begin_world.x * axis.x + begin_world.y * axis.y,
                    end_projection: end_world.x * axis.x + end_world.y * axis.y,
                    axis_normal_projection: begin_world.x * -axis.y + begin_world.y * axis.x,
                    both_junction: incident
                        .get(bond.begin.as_str())
                        .is_some_and(|bonds| bonds.len() > 1)
                        && incident
                            .get(bond.end.as_str())
                            .is_some_and(|bonds| bonds.len() > 1),
                    side_double: preview_bond_is_side_double(bond),
                    center_double: bond.order == 2 && !preview_bond_is_side_double(bond),
                    hashed_wedge: bond
                        .stereo
                        .as_ref()
                        .is_some_and(|stereo| stereo.kind == "hashed-wedge"),
                    start_has_label,
                    end_has_label,
                },
            );
        }
    }
    PreviewBondContext { infos }
}

pub(super) fn preview_fragment_components(
    node_map: &BTreeMap<&str, &chemsema_engine::Node>,
    adjacency: &BTreeMap<&str, Vec<&str>>,
) -> Vec<Vec<String>> {
    let mut visited = std::collections::BTreeSet::new();
    let mut components = Vec::new();
    for node_id in node_map.keys().copied() {
        if visited.contains(node_id) {
            continue;
        }
        let mut queue = std::collections::VecDeque::from([node_id]);
        visited.insert(node_id);
        let mut component = Vec::new();
        while let Some(current) = queue.pop_front() {
            component.push(current.to_string());
            for neighbor in adjacency.get(current).into_iter().flatten().copied() {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
        components.push(component);
    }
    components
}

pub(super) fn preview_label_world_box(
    object: &SceneObject,
    label: &chemsema_engine::NodeLabel,
) -> Option<PreviewLabelBBox> {
    let mut candidates = Vec::new();
    if let Some(bbox) = label.bbox() {
        candidates.push(preview_transform_scene_bbox(
            object,
            PreviewLabelBBox {
                left: bbox[0],
                top: bbox[1],
                right: bbox[2],
                bottom: bbox[3],
            },
        ));
    }
    let glyph_points: Vec<CorePoint> = label
        .glyph_polygons()
        .into_iter()
        .flat_map(|polygon| polygon.into_iter())
        .map(|point| preview_transform_scene_point(object, point.x, point.y))
        .collect();
    if let Some(glyph_box) = preview_bbox_from_points(&glyph_points) {
        candidates.push(glyph_box);
    }
    preview_union_boxes(&candidates)
}

pub(super) fn preview_label_is_simple_single_run(label: &chemsema_engine::NodeLabel) -> bool {
    if label.text.contains('\n') || !label.has_visible_text() {
        return false;
    }
    let runs = if !label.line_runs.is_empty() {
        if label.line_runs.len() != 1 {
            return false;
        }
        label.line_runs.first().map(Vec::as_slice).unwrap_or(&[])
    } else {
        label.runs.as_slice()
    };
    if runs.len() != 1 {
        return false;
    }
    let run = &runs[0];
    !matches!(run.script.as_deref(), Some("subscript" | "superscript"))
}

pub(super) fn preview_transform_scene_point(object: &SceneObject, x: f64, y: f64) -> CorePoint {
    let mut x = x * object.transform.scale[0];
    let mut y = y * object.transform.scale[1];
    if object.transform.rotate.abs() > f64::EPSILON {
        let theta = object.transform.rotate.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let rotated_x = x * cos_t - y * sin_t;
        let rotated_y = x * sin_t + y * cos_t;
        x = rotated_x;
        y = rotated_y;
    }
    CorePoint {
        x: x + object.transform.translate[0],
        y: y + object.transform.translate[1],
    }
}

pub(super) fn preview_transform_scene_bbox(
    object: &SceneObject,
    bbox: PreviewLabelBBox,
) -> PreviewLabelBBox {
    let corners = [
        preview_transform_scene_point(object, bbox.left, bbox.top),
        preview_transform_scene_point(object, bbox.right, bbox.top),
        preview_transform_scene_point(object, bbox.right, bbox.bottom),
        preview_transform_scene_point(object, bbox.left, bbox.bottom),
    ];
    preview_bbox_from_points(&corners).unwrap_or(bbox)
}

pub(super) fn preview_bbox_from_points(points: &[CorePoint]) -> Option<PreviewLabelBBox> {
    let mut iter = points.iter();
    let first = iter.next()?;
    let mut bbox = PreviewLabelBBox {
        left: first.x,
        top: first.y,
        right: first.x,
        bottom: first.y,
    };
    for point in iter {
        bbox.left = bbox.left.min(point.x);
        bbox.top = bbox.top.min(point.y);
        bbox.right = bbox.right.max(point.x);
        bbox.bottom = bbox.bottom.max(point.y);
    }
    Some(bbox)
}

pub(super) fn preview_union_boxes(boxes: &[PreviewLabelBBox]) -> Option<PreviewLabelBBox> {
    let mut iter = boxes.iter().copied();
    let mut bbox = iter.next()?;
    for next in iter {
        bbox = bbox.expand_to_include(next);
    }
    Some(bbox)
}

pub(super) fn preview_molecule_fragment<'a>(
    document: &'a ChemSemaDocument,
    object: &SceneObject,
) -> Option<&'a MoleculeFragment> {
    let resource_ref = object.payload.resource_ref.as_ref()?;
    document.resources.get(resource_ref)?.data.as_fragment()
}

pub(super) fn preview_bond_axis_from_nodes(
    object: &SceneObject,
    begin: CorePoint,
    end: CorePoint,
) -> Option<CorePoint> {
    let start = preview_transform_scene_point(object, begin.x, begin.y);
    let finish = preview_transform_scene_point(object, end.x, end.y);
    preview_normalize_axis(CorePoint {
        x: finish.x - start.x,
        y: finish.y - start.y,
    })
}

pub(super) fn preview_bond_is_pen_family(bond: &Bond) -> bool {
    if bond.stereo.is_some() || bond.line_styles.main != BondLinePattern::Solid {
        return false;
    }
    match bond.order {
        0 => false,
        1 => bond.line_weights.main == BondLineWeight::Normal,
        2 => {
            bond.line_weights.main == BondLineWeight::Normal
                && bond.line_styles.left == BondLinePattern::Solid
                && bond.line_styles.right == BondLinePattern::Solid
                && bond.line_weights.left == BondLineWeight::Normal
                && bond.line_weights.right == BondLineWeight::Normal
        }
        _ => {
            bond.line_weights.main == BondLineWeight::Normal
                && bond.line_styles.left == BondLinePattern::Solid
                && bond.line_styles.right == BondLinePattern::Solid
                && bond.line_weights.left == BondLineWeight::Normal
                && bond.line_weights.right == BondLineWeight::Normal
        }
    }
}

pub(super) fn preview_bond_is_side_double(bond: &Bond) -> bool {
    matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    )
}

pub(super) fn preview_normalize_axis(axis: CorePoint) -> Option<CorePoint> {
    let length = axis.distance(CorePoint { x: 0.0, y: 0.0 });
    if length <= 1.0e-9 {
        return None;
    }
    Some(CorePoint {
        x: axis.x / length,
        y: axis.y / length,
    })
}

pub(super) fn preview_hashed_wedge_stroke_line(
    points: &[CorePoint],
    bond_id: Option<&str>,
    bond_context: Option<&PreviewBondContext>,
) -> Option<PreviewBondStrokeLine> {
    if points.len() != 4 {
        return None;
    }
    let info = bond_id.and_then(|id| bond_context?.infos.get(id))?;
    if !info.hashed_wedge {
        return None;
    }
    let axis = info.axis;
    let axis_projection = |point: CorePoint| point.x * axis.x + point.y * axis.y;
    let projections: Vec<f64> = points.iter().copied().map(axis_projection).collect();
    let min_projection = projections.iter().copied().fold(f64::INFINITY, f64::min);
    let max_projection = projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let axial_width = max_projection - min_projection;
    if !axial_width.is_finite() || axial_width <= 0.0 || axial_width > PREVIEW_BOND_STROKE_MAX_WIDTH
    {
        return None;
    }
    // With two or more hashes, each primitive is already one LineWidth long
    // along the axis, so its midpoint is the sampled stripe center. When a
    // short wedge has only one hash, the scene primitive is the entire
    // trapezoid. ChemDraw's direct EMF replay samples that trapezoid half a
    // LineWidth inward from the wide end instead of sampling its midpoint.
    let center_projection = if axial_width > info.line_width + 1.0e-9 {
        let wide_projection = info.hashed_wedge_wide_projection?;
        let inward = info.line_width.min(axial_width) * 0.5;
        if (wide_projection - max_projection).abs() <= (wide_projection - min_projection).abs() {
            max_projection - inward
        } else {
            min_projection + inward
        }
    } else {
        (min_projection + max_projection) * 0.5
    };
    let mut intersections = Vec::new();
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        let from_delta = projections[index] - center_projection;
        let to_delta = projections[next] - center_projection;
        if from_delta.abs() <= 1.0e-9 {
            intersections.push(points[index]);
        }
        if from_delta * to_delta < -1.0e-12 {
            let t = from_delta / (from_delta - to_delta);
            intersections.push(CorePoint {
                x: points[index].x + (points[next].x - points[index].x) * t,
                y: points[index].y + (points[next].y - points[index].y) * t,
            });
        }
    }
    intersections.sort_by(|left, right| {
        let left_normal = left.x * -axis.y + left.y * axis.x;
        let right_normal = right.x * -axis.y + right.y * axis.x;
        left_normal.total_cmp(&right_normal)
    });
    intersections.dedup_by(|left, right| left.distance(*right) <= 1.0e-8);
    if intersections.len() != 2 || intersections[0].distance(intersections[1]) <= 1.0e-9 {
        return None;
    }
    Some(PreviewBondStrokeLine {
        start: intersections[0],
        end: intersections[1],
        width: info.line_width,
    })
}

pub(super) fn preview_office_hashed_wedge_stroke_line(
    points: &[CorePoint],
    bond_id: Option<&str>,
    bond_context: Option<&PreviewBondContext>,
) -> Option<PreviewBondStrokeLine> {
    let transverse = preview_hashed_wedge_stroke_line(points, bond_id, bond_context)?;
    let info = bond_id.and_then(|id| bond_context?.infos.get(id))?;
    let transverse_length = transverse.start.distance(transverse.end);
    // ChemDraw's Office/OLE presentation path treats the near-square narrow
    // stripe as a generic filled shaft and replays it along its axial major
    // direction. Wider stripes remain filled transverse quadrilaterals. Direct
    // SaveAs(EMF) instead replays every stripe transversely.
    if transverse_length > info.line_width * 1.25 {
        return None;
    }
    let center = CorePoint {
        x: (transverse.start.x + transverse.end.x) * 0.5,
        y: (transverse.start.y + transverse.end.y) * 0.5,
    };
    let half_length = info.line_width * 0.5;
    Some(PreviewBondStrokeLine {
        start: CorePoint {
            x: center.x - info.axis.x * half_length,
            y: center.y - info.axis.y * half_length,
        },
        end: CorePoint {
            x: center.x + info.axis.x * half_length,
            y: center.y + info.axis.y * half_length,
        },
        width: info.line_width,
    })
}

pub(super) fn preview_bond_stroke_line(
    points: &[CorePoint],
    bond_id: Option<&str>,
    bond_context: Option<&PreviewBondContext>,
) -> Option<PreviewBondStrokeLine> {
    let pen_mode = preview_bond_pen_conversion_mode();
    if pen_mode == PreviewBondPenConversionMode::Off {
        return None;
    }
    if points.len() < 4 || points.len() > 6 {
        return None;
    }
    let bond_info = bond_id.and_then(|id| bond_context.and_then(|context| context.infos.get(id)));
    if bond_info.is_some_and(|info| !info.allow_pen) {
        return None;
    }
    if !preview_bond_pen_conversion_allowed(pen_mode, bond_info) {
        return None;
    }
    let preferred_axis = bond_info.map(|info| info.axis);
    let axis = preferred_axis.or_else(|| preview_polygon_principal_axis(points))?;
    let normal = CorePoint {
        x: -axis.y,
        y: axis.x,
    };
    let projections: Vec<f64> = points
        .iter()
        .map(|point| point.x * axis.x + point.y * axis.y)
        .collect();
    let normal_projections: Vec<f64> = points
        .iter()
        .map(|point| point.x * normal.x + point.y * normal.y)
        .collect();
    let min_projection = projections.iter().copied().fold(f64::INFINITY, f64::min);
    let max_projection = projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let length = max_projection - min_projection;
    let width = normal_projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        - normal_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    if !length.is_finite()
        || !width.is_finite()
        || length <= 0.0
        || width <= 0.0
        || width > PREVIEW_BOND_STROKE_MAX_WIDTH
    {
        return None;
    }
    let simplified = preview_simplify_bond_polygon(
        points,
        axis,
        width * PREVIEW_BOND_STROKE_COLLINEAR_TOLERANCE_WIDTH_FACTOR,
    )?;
    if simplified.len() < 4 || simplified.len() > 6 {
        return None;
    }
    let axis = preferred_axis.or_else(|| preview_polygon_principal_axis(&simplified))?;
    let normal = CorePoint {
        x: -axis.y,
        y: axis.x,
    };
    let projections: Vec<f64> = simplified
        .iter()
        .map(|point| point.x * axis.x + point.y * axis.y)
        .collect();
    let normal_projections: Vec<f64> = simplified
        .iter()
        .map(|point| point.x * normal.x + point.y * normal.y)
        .collect();
    let min_projection = projections.iter().copied().fold(f64::INFINITY, f64::min);
    let max_projection = projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let length = max_projection - min_projection;
    let width = normal_projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        - normal_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    if !length.is_finite()
        || !width.is_finite()
        || length <= 0.0
        || width <= 0.0
        || width > PREVIEW_BOND_STROKE_MAX_WIDTH
    {
        return None;
    }
    if preferred_axis.is_some() {
        let stroke_width = width * PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE;
        if !stroke_width.is_finite() || stroke_width <= 0.0 {
            return None;
        }
        let min_normal = normal_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_normal = normal_projections
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let normal_mid = (min_normal + max_normal) * 0.5;
        let tolerance = (width * PREVIEW_BOND_STROKE_TOLERANCE_WIDTH_FACTOR)
            .max(length * 0.01)
            .max(0.05);
        let start_edge = preview_bond_terminal_edge(
            &simplified,
            &projections,
            axis,
            min_projection,
            tolerance,
            width,
        );
        let end_edge = preview_bond_terminal_edge(
            &simplified,
            &projections,
            axis,
            max_projection,
            tolerance,
            width,
        );
        let mut start_axis_projection = bond_info
            .map(|info| info.start_projection)
            .unwrap_or(min_projection);
        let mut end_axis_projection = bond_info
            .map(|info| info.end_projection)
            .unwrap_or(max_projection);
        if let Some(info) = bond_info {
            if info.center_double {
                let cap_radius = stroke_width * 0.5;
                if let Some(edge) = start_edge {
                    let edge_projection = edge.center.x * axis.x + edge.center.y * axis.y;
                    start_axis_projection = if edge_projection < info.start_projection - 1.0e-6 {
                        edge_projection + cap_radius
                    } else {
                        info.start_projection
                    };
                }
                if let Some(edge) = end_edge {
                    let edge_projection = edge.center.x * axis.x + edge.center.y * axis.y;
                    end_axis_projection = if edge_projection > info.end_projection + 1.0e-6 {
                        edge_projection - cap_radius
                    } else {
                        info.end_projection
                    };
                }
            }
            if info.start_has_label {
                if let Some(edge) = start_edge {
                    start_axis_projection = edge.center.x * axis.x + edge.center.y * axis.y;
                }
            }
            if info.end_has_label {
                if let Some(edge) = end_edge {
                    end_axis_projection = edge.center.x * axis.x + edge.center.y * axis.y;
                }
            }
            let cap_radius = stroke_width * 0.5;
            if info.side_double && (normal_mid - info.axis_normal_projection).abs() > width * 0.35 {
                start_axis_projection = min_projection + cap_radius;
                end_axis_projection = max_projection - cap_radius;
            }
        }
        return Some(PreviewBondStrokeLine {
            start: preview_point_from_axis_coordinates(
                axis,
                normal,
                start_axis_projection,
                normal_mid,
            ),
            end: preview_point_from_axis_coordinates(axis, normal, end_axis_projection, normal_mid),
            width: stroke_width,
        });
    }
    let tolerance = (width * PREVIEW_BOND_STROKE_TOLERANCE_WIDTH_FACTOR)
        .max(length * 0.01)
        .max(0.05);
    let start_edge = preview_bond_terminal_edge(
        &simplified,
        &projections,
        axis,
        min_projection,
        tolerance,
        width,
    )?;
    let end_edge = preview_bond_terminal_edge(
        &simplified,
        &projections,
        axis,
        max_projection,
        tolerance,
        width,
    )?;
    let stroke_width =
        (start_edge.length + end_edge.length) * 0.5 * PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE;
    if !stroke_width.is_finite() || stroke_width <= 0.0 {
        return None;
    }
    Some(PreviewBondStrokeLine {
        start: start_edge.center,
        end: end_edge.center,
        width: stroke_width,
    })
}

pub(super) fn preview_point_from_axis_coordinates(
    axis: CorePoint,
    normal: CorePoint,
    axis_projection: f64,
    normal_projection: f64,
) -> CorePoint {
    CorePoint {
        x: axis.x * axis_projection + normal.x * normal_projection,
        y: axis.y * axis_projection + normal.y * normal_projection,
    }
}

pub(super) fn preview_simplify_bond_polygon(
    points: &[CorePoint],
    axis: CorePoint,
    tolerance: f64,
) -> Option<Vec<CorePoint>> {
    if points.len() < 4 {
        return None;
    }
    let mut simplified = points.to_vec();
    loop {
        if simplified.len() <= 4 {
            break;
        }
        let mut removed = false;
        let len = simplified.len();
        for index in 0..len {
            let prev = simplified[(index + len - 1) % len];
            let point = simplified[index];
            let next = simplified[(index + 1) % len];
            if preview_point_is_collinear(prev, point, next, axis, tolerance) {
                simplified.remove(index);
                removed = true;
                break;
            }
        }
        if !removed {
            break;
        }
    }
    Some(simplified)
}

pub(super) fn preview_point_is_collinear(
    prev: CorePoint,
    point: CorePoint,
    next: CorePoint,
    axis: CorePoint,
    tolerance: f64,
) -> bool {
    let segment = CorePoint {
        x: next.x - prev.x,
        y: next.y - prev.y,
    };
    let segment_length = segment.distance(CorePoint { x: 0.0, y: 0.0 });
    if segment_length <= 1.0e-9 {
        return point.distance(prev) <= tolerance;
    }
    let prev_to_point = CorePoint {
        x: point.x - prev.x,
        y: point.y - prev.y,
    };
    let distance =
        ((prev_to_point.x * segment.y) - (prev_to_point.y * segment.x)).abs() / segment_length;
    if distance > tolerance.max(0.05) {
        return false;
    }
    let dot = prev_to_point.x * segment.x + prev_to_point.y * segment.y;
    let projection = dot / (segment_length * segment_length);
    if !(0.0..=1.0).contains(&projection) {
        return false;
    }
    let segment_axis_ratio = ((segment.x * axis.x + segment.y * axis.y).abs()) / segment_length;
    segment_axis_ratio >= 1.0 - PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO
}

pub(super) fn preview_polygon_principal_axis(points: &[CorePoint]) -> Option<CorePoint> {
    if points.len() < 2 {
        return None;
    }
    let mut mean = CorePoint { x: 0.0, y: 0.0 };
    for point in points {
        mean.x += point.x;
        mean.y += point.y;
    }
    let point_count = points.len() as f64;
    mean.x /= point_count;
    mean.y /= point_count;

    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for point in points {
        let dx = point.x - mean.x;
        let dy = point.y - mean.y;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    let trace = sxx + syy;
    let root = (sxx - syy).hypot(2.0 * sxy);
    let lambda = (trace + root) * 0.5;
    let mut axis = CorePoint {
        x: sxy,
        y: lambda - sxx,
    };
    if axis.distance(CorePoint { x: 0.0, y: 0.0 }) <= 1.0e-9 {
        axis = CorePoint {
            x: lambda - syy,
            y: sxy,
        };
    }
    let length = axis.distance(CorePoint { x: 0.0, y: 0.0 });
    if length <= 1.0e-9 {
        return None;
    }
    Some(CorePoint {
        x: axis.x / length,
        y: axis.y / length,
    })
}

pub(super) fn preview_bond_terminal_edge(
    points: &[CorePoint],
    projections: &[f64],
    axis: CorePoint,
    target: f64,
    tolerance: f64,
    width: f64,
) -> Option<PreviewBondTerminalEdge> {
    let indices: Vec<usize> = projections
        .iter()
        .enumerate()
        .filter_map(|(index, projection)| {
            if (*projection - target).abs() <= tolerance {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if indices.is_empty() || indices.len() > 3 {
        return None;
    }
    let ordered = preview_polygon_terminal_chain(points.len(), &indices)?;
    let normal_projection = |index: usize| points[index].x * -axis.y + points[index].y * axis.x;
    let center = match ordered.len() {
        1 => points[ordered[0]],
        2 => {
            let first = ordered[0];
            let last = ordered[1];
            let edge = CorePoint {
                x: points[last].x - points[first].x,
                y: points[last].y - points[first].y,
            };
            let edge_length = points[first].distance(points[last]).max(1.0e-9);
            let along_axis = (edge.x * axis.x + edge.y * axis.y).abs() / edge_length;
            if along_axis <= PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO {
                CorePoint {
                    x: (points[first].x + points[last].x) * 0.5,
                    y: (points[first].y + points[last].y) * 0.5,
                }
            } else {
                let apex = ordered
                    .iter()
                    .copied()
                    .min_by(|left, right| {
                        (projections[*left] - target)
                            .abs()
                            .total_cmp(&(projections[*right] - target).abs())
                    })
                    .unwrap_or(first);
                points[apex]
            }
        }
        3 => {
            let apex = ordered
                .iter()
                .copied()
                .min_by(|left, right| {
                    (projections[*left] - target)
                        .abs()
                        .total_cmp(&(projections[*right] - target).abs())
                })
                .unwrap_or(ordered[1]);
            points[apex]
        }
        _ => return None,
    };
    let edge_length = if ordered.len() == 1 {
        width
    } else {
        let first = ordered[0];
        let last = *ordered.last().unwrap_or(&ordered[0]);
        let span = (normal_projection(first) - normal_projection(last)).abs();
        let edge = CorePoint {
            x: points[last].x - points[first].x,
            y: points[last].y - points[first].y,
        };
        let along_axis = edge.x * axis.x + edge.y * axis.y;
        let edge_length = span.max(points[first].distance(points[last]));
        if edge_length <= 0.0 {
            return None;
        }
        if ordered.len() == 2
            && along_axis.abs() / edge_length > PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO
        {
            width
        } else {
            if along_axis.abs() / edge_length > PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO {
                return None;
            }
            edge_length
        }
    };
    if edge_length < width * PREVIEW_BOND_STROKE_EDGE_WIDTH_MIN_RATIO
        || edge_length > width * PREVIEW_BOND_STROKE_EDGE_WIDTH_MAX_RATIO
    {
        return None;
    }
    Some(PreviewBondTerminalEdge {
        center,
        length: edge_length,
    })
}

pub(super) fn preview_polygon_terminal_chain(len: usize, indices: &[usize]) -> Option<Vec<usize>> {
    if indices.is_empty() || indices.len() > 3 {
        return None;
    }
    if indices.len() == 1 {
        return Some(indices.to_vec());
    }
    let mut ordered = indices.to_vec();
    ordered.sort_unstable();
    for &start in &ordered {
        let mut chain = vec![start];
        let mut current = start;
        while chain.len() < ordered.len() {
            let next = (current + 1) % len;
            if ordered.contains(&next) {
                chain.push(next);
                current = next;
            } else {
                break;
            }
        }
        if chain.len() == ordered.len() {
            return Some(chain);
        }
    }
    None
}

pub(super) fn polygon_area(points: &[CorePoint]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

pub(super) unsafe fn draw_placeholder_preview_impl(dc: HDC, bounds: &RECT) {
    let width = (bounds.right - bounds.left).max(1);
    let height = (bounds.bottom - bounds.top).max(1);
    let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let pen = CreatePen(PS_SOLID, (width.min(height) / 120).clamp(1, 16), 0x000000);
    let old_pen = SelectObject(dc, pen as HGDIOBJ);

    let mid_y = bounds.top + height * 58 / 100;
    let left_x = bounds.left + width * 24 / 100;
    let right_x = bounds.left + width * 76 / 100;
    MoveToEx(dc, left_x, mid_y, null_mut());
    LineTo(dc, right_x, mid_y);
    let radius = (width.min(height) / 20).max(3);
    Ellipse(
        dc,
        left_x - radius,
        mid_y - radius,
        left_x + radius,
        mid_y + radius,
    );
    Ellipse(
        dc,
        right_x - radius,
        mid_y - radius,
        right_x + radius,
        mid_y + radius,
    );

    SetBkMode(dc, TRANSPARENT as i32);
    let label = ansi_metafile_text_bytes(DOCUMENT_DISPLAY_NAME);
    TextOutA(
        dc,
        bounds.left + width * 30 / 100,
        bounds.top + height * 18 / 100,
        label.as_ptr(),
        label.len() as i32,
    );

    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen as HGDIOBJ);
}
