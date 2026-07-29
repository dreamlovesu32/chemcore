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
        if let Some(anchor) = glyph_polygons.get(anchor_index).and_then(|polygon| {
            crate::node_label_glyph_anchor_point_with_anchor_y(
                label,
                anchor_index,
                polygon,
                label_line_anchor_y(label),
            )
        }) {
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
        .filter_map(|(index, polygon)| crate::polygon_bounds(polygon).map(|bbox| (index, bbox)))
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

fn label_line_anchor_y(label: &crate::NodeLabel) -> Option<f64> {
    let position = label.position?;
    let font_size = label
        .font_size
        .unwrap_or(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    Some(position[1] - font_size * crate::MOLECULE_LABEL_ANCHOR_BASELINE_RATIO)
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
    let asymmetric_double_offset =
        terminal_asymmetric_double_bond_label_offset(fragment, node_id, stroke_width);
    Point::new(
        object_translate[0] + node.position[0] + asymmetric_double_offset.x,
        object_translate[1] + node.position[1] + asymmetric_double_offset.y,
    )
}

fn terminal_asymmetric_double_bond_label_offset(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    stroke_width: f64,
) -> crate::Vector {
    let mut connected = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id);
    let Some(bond) = connected.next() else {
        return crate::Vector::new(0.0, 0.0);
    };
    if connected.next().is_some() || bond.order != 2 {
        return crate::Vector::new(0.0, 0.0);
    }
    let Some(double) = bond.double.as_ref() else {
        return crate::Vector::new(0.0, 0.0);
    };
    let side_sign = match double.placement {
        crate::DoubleBondPlacement::Left => 1.0,
        crate::DoubleBondPlacement::Right => -1.0,
        crate::DoubleBondPlacement::Center => return crate::Vector::new(0.0, 0.0),
    };
    let Some(begin) = fragment.nodes.iter().find(|node| node.id == bond.begin) else {
        return crate::Vector::new(0.0, 0.0);
    };
    let Some(end) = fragment.nodes.iter().find(|node| node.id == bond.end) else {
        return crate::Vector::new(0.0, 0.0);
    };
    let start = begin.point();
    let finish = end.point();
    let axis = crate::Vector::new(finish.x - start.x, finish.y - start.y);
    let length = axis.length();
    if length <= crate::EPSILON {
        return crate::Vector::new(0.0, 0.0);
    }
    let side_weight = match double.placement {
        crate::DoubleBondPlacement::Left => bond.line_weights.left,
        crate::DoubleBondPlacement::Right => bond.line_weights.right,
        crate::DoubleBondPlacement::Center => unreachable!(),
    };
    let effective_stroke_width =
        if bond.stroke_width.is_finite() && bond.stroke_width > crate::EPSILON {
            bond.stroke_width
        } else {
            stroke_width
        };
    let center_distance = crate::render::double_bond_center_distance_for_bond_weights(
        bond,
        start,
        finish,
        effective_stroke_width,
        bond.line_weights.main,
        side_weight,
    );
    crate::Vector::new(
        side_sign * (-axis.y / length) * center_distance * 0.5,
        side_sign * (axis.x / length) * center_distance * 0.5,
    )
}
