use super::*;

pub(super) fn endpoint_label_editor_anchor_world(
    node: &crate::Node,
    object_translate: [f64; 2],
    connection_angles: &[f64],
) -> Option<Point> {
    let label = node.label.as_ref()?;
    let glyph_polygons = label.glyph_polygons();
    if !glyph_polygons.is_empty() {
        if let Some(anchor) = centered_label_anchor_world(label, &glyph_polygons, object_translate)
        {
            return Some(anchor);
        }
        let source_runs = source_runs_from_node_label(label);
        let source_text = if !source_runs.is_empty() {
            runs_text(&source_runs)
        } else {
            label
                .source_text
                .clone()
                .unwrap_or_else(|| label.text.clone())
        };
        let decision = label_layout_decision_for_text_mode(
            &source_text,
            connection_angles,
            source_runs_are_chemical(&source_runs),
        );
        let layout = layout_label_text(&source_text, &decision);
        let font_family = label
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_TEXT_FONT_FAMILY.to_string());
        let font_size = WorldPt(label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE)).value();
        let fill = label
            .fill
            .clone()
            .unwrap_or_else(|| DEFAULT_TEXT_FILL.to_string());
        let display_runs =
            display_runs_from_source_runs(&source_runs, &font_family, font_size, &fill);
        let (_, line_runs) = layout_display_runs(&display_runs, &decision);
        let anchor_index = label_anchor_index_for_layout(&line_runs, &layout);
        if let Some(anchor) = glyph_polygons
            .get(anchor_index)
            .and_then(|polygon| polygon_anchor_point(polygon))
        {
            return Some(Point::new(
                anchor.x + object_translate[0],
                anchor.y + object_translate[1],
            ));
        }
    }
    let bbox = label.bbox()?;
    Some(Point::new(
        bbox[0] + object_translate[0],
        bbox[1] + object_translate[1],
    ))
}

fn centered_label_anchor_world(
    label: &crate::NodeLabel,
    glyph_polygons: &[Vec<Point>],
    object_translate: [f64; 2],
) -> Option<Point> {
    if !label_is_centered(label) {
        return None;
    }
    let bbox = label.bbox()?;
    let center_x = (bbox[0] + bbox[2]) * 0.5;
    let (_, glyph_box) = centered_label_glyph_box(glyph_polygons, center_x)?;
    Some(Point::new(
        center_x + object_translate[0],
        (glyph_box[1] + glyph_box[3]) * 0.5 + object_translate[1],
    ))
}

fn label_is_centered(label: &crate::NodeLabel) -> bool {
    label.layout.as_deref() == Some("attached-group-center")
        || (label.align.as_deref() == Some("center") && label.anchor.as_deref() == Some("middle"))
}

fn centered_label_glyph_box(
    glyph_polygons: &[Vec<Point>],
    center_x: f64,
) -> Option<(usize, [f64; 4])> {
    glyph_polygons
        .iter()
        .enumerate()
        .filter_map(|(index, polygon)| polygon_bounds(polygon).map(|bbox| (index, bbox)))
        .min_by(|(_, left), (_, right)| {
            glyph_center_distance_to_x(*left, center_x)
                .total_cmp(&glyph_center_distance_to_x(*right, center_x))
        })
}

fn glyph_center_distance_to_x(bbox: [f64; 4], x: f64) -> f64 {
    if x >= bbox[0] && x <= bbox[2] {
        0.0
    } else {
        (((bbox[0] + bbox[2]) * 0.5) - x).abs()
    }
}

fn polygon_bounds(polygon: &[Point]) -> Option<[f64; 4]> {
    if polygon.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some([min_x, min_y, max_x, max_y])
}

pub(super) fn polygon_anchor_point(polygon: &[Point]) -> Option<Point> {
    if polygon.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some(Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5))
}

pub(super) fn current_node_label_editor_geometry(
    node: &crate::Node,
    object_translate: [f64; 2],
    connection_angles: &[f64],
) -> (Option<[f64; 2]>, Option<[f64; 4]>) {
    let Some(bounds) = endpoint_label_world_bounds(node, object_translate) else {
        return (None, None);
    };
    let anchor_offset =
        endpoint_label_editor_anchor_world(node, object_translate, connection_angles)
            .map(|anchor| [round6(anchor.x - bounds[0]), round6(anchor.y - bounds[1])]);
    (anchor_offset, Some(bounds))
}

pub(super) fn attached_node_label_anchor_world(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    stroke_width: f64,
) -> Point {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Point::new(object_translate[0], object_translate[1]);
    };
    let node_world = Point::new(
        object_translate[0] + node.position[0],
        object_translate[1] + node.position[1],
    );
    let connected: Vec<_> = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .collect();
    if connected.len() != 1 || connected[0].order != 2 {
        return node_world;
    }
    let bond = connected[0];
    let Some(begin_node) = fragment.nodes.iter().find(|other| other.id == bond.begin) else {
        return node_world;
    };
    let Some(end_node) = fragment.nodes.iter().find(|other| other.id == bond.end) else {
        return node_world;
    };
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(DoubleBondPlacement::Center);
    if placement == DoubleBondPlacement::Center {
        return node_world;
    }
    let begin_world = Point::new(
        object_translate[0] + begin_node.position[0],
        object_translate[1] + begin_node.position[1],
    );
    let end_world = Point::new(
        object_translate[0] + end_node.position[0],
        object_translate[1] + end_node.position[1],
    );
    let dx = end_world.x - begin_world.x;
    let dy = end_world.y - begin_world.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return node_world;
    }
    let side = if placement == DoubleBondPlacement::Left {
        1.0
    } else {
        -1.0
    };
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let offset =
        0.5 * side_double_center_distance_for_bond_points(
            bond,
            begin_world,
            end_world,
            stroke_width,
            placement,
        ) * side;
    Point::new(
        node_world.x + normal_x * offset,
        node_world.y + normal_y * offset,
    )
}

pub(super) fn bond_line_weight_stroke_width_for_engine(
    bond: &Bond,
    stroke_width: f64,
    weight: BondLineWeight,
) -> f64 {
    if weight == BondLineWeight::Bold {
        bond.bold_width
            .unwrap_or_else(|| {
                crate::BOLD_BOND_WIDTH_PT.value() * (stroke_width / crate::DEFAULT_BOND_STROKE_PT)
            })
            .max(stroke_width)
    } else {
        stroke_width
    }
}

pub(super) fn side_double_center_distance_for_bond_points(
    bond: &Bond,
    start: Point,
    end: Point,
    stroke_width: f64,
    placement: DoubleBondPlacement,
) -> f64 {
    let outer_weight = match placement {
        DoubleBondPlacement::Left => bond.line_weights.left,
        DoubleBondPlacement::Right => bond.line_weights.right,
        DoubleBondPlacement::Center => BondLineWeight::Normal,
    };
    let main_width =
        bond_line_weight_stroke_width_for_engine(bond, stroke_width, bond.line_weights.main);
    let outer_width = bond_line_weight_stroke_width_for_engine(bond, stroke_width, outer_weight);
    start.distance(end) * 0.12 + 0.5 * (main_width + outer_width)
}
