use super::*;

pub(super) fn point_to_segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= crate::EPSILON {
        return point.distance(start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0);
    point.distance(Point::new(start.x + dx * t, start.y + dy * t))
}

pub(super) fn point_to_polyline_distance(point: Point, points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| point_to_segment_distance(point, pair[0], pair[1]))
        .min_by(f64::total_cmp)
        .unwrap_or(f64::INFINITY)
}

pub(super) fn point_in_box(point: Point, bounds: [f64; 4]) -> bool {
    point.x >= bounds[0] && point.x <= bounds[2] && point.y >= bounds[1] && point.y <= bounds[3]
}

pub(super) fn label_anchor_geometries(
    entry: &EditableFragment<'_>,
    node: &Node,
) -> Vec<LabelAnchorGeometry> {
    let Some(label) = node.label.as_ref() else {
        return Vec::new();
    };
    if label.glyph_polygons.is_empty() {
        return Vec::new();
    }

    let glyph_polygons = label.glyph_polygons();
    if let Some(center_anchor) = centered_label_anchor_geometry(label, &glyph_polygons, entry) {
        return vec![center_anchor];
    }
    let glyph_points: Vec<Point> = glyph_polygons
        .iter()
        .filter_map(|polygon| polygon_anchor_point(&polygon))
        .map(|point| {
            Point::new(
                point.x + entry.object.transform.translate[0],
                point.y + entry.object.transform.translate[1],
            )
        })
        .collect();
    if glyph_points.is_empty() {
        return Vec::new();
    }

    let first_glyph_point = glyph_points[0];
    let left_point = glyph_points
        .iter()
        .copied()
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap_or(first_glyph_point);
    let (rightmost_glyph_index, right_point) = glyph_points
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.x.total_cmp(&right.1.x))
        .map(|(index, point)| (index, point))
        .unwrap_or((0, first_glyph_point));
    let right_group_index = rightmost_group_anchor_index(label, glyph_points.len());
    let right_group_point = right_group_index.and_then(|index| glyph_points.get(index).copied());

    glyph_points
        .iter()
        .enumerate()
        .filter_map(|(glyph_index, glyph_point)| {
            let glyph_box = glyph_polygons.get(glyph_index).and_then(|polygon| {
                polygon_bounds_world(polygon, entry.object.transform.translate)
            })?;
            Some(LabelAnchorGeometry {
                glyph_index,
                glyph_point: *glyph_point,
                glyph_box,
                first_glyph_point,
                left_point,
                right_point,
                rightmost_glyph_index,
                right_group_point,
            })
        })
        .collect()
}

fn centered_label_anchor_geometry(
    label: &crate::NodeLabel,
    glyph_polygons: &[Vec<Point>],
    entry: &EditableFragment<'_>,
) -> Option<LabelAnchorGeometry> {
    if !label_is_centered(label) {
        return None;
    }
    let bbox = label.bbox()?;
    let center_x = (bbox[0] + bbox[2]) * 0.5;
    let (glyph_index, glyph_box_local) = centered_label_glyph_box(glyph_polygons, center_x)?;
    let translate = entry.object.transform.translate;
    let glyph_box = [
        glyph_box_local[0] + translate[0],
        glyph_box_local[1] + translate[1],
        glyph_box_local[2] + translate[0],
        glyph_box_local[3] + translate[1],
    ];
    let glyph_point = Point::new(
        center_x + translate[0],
        (glyph_box_local[1] + glyph_box_local[3]) * 0.5 + translate[1],
    );
    Some(LabelAnchorGeometry {
        glyph_index,
        glyph_point,
        glyph_box,
        first_glyph_point: glyph_point,
        left_point: glyph_point,
        right_point: glyph_point,
        rightmost_glyph_index: glyph_index,
        right_group_point: Some(glyph_point),
    })
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

pub(super) fn polygon_bounds_world(polygon: &[Point], translate: [f64; 2]) -> Option<[f64; 4]> {
    if polygon.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x + translate[0]);
        min_y = min_y.min(point.y + translate[1]);
        max_x = max_x.max(point.x + translate[0]);
        max_y = max_y.max(point.y + translate[1]);
    }
    Some([min_x, min_y, max_x, max_y])
}

pub(super) fn label_visible_chars(node_label: &crate::NodeLabel) -> Vec<char> {
    node_label
        .source_text
        .as_deref()
        .unwrap_or(node_label.text.as_str())
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

pub(super) fn rightmost_group_anchor_index(
    node_label: &crate::NodeLabel,
    glyph_count: usize,
) -> Option<usize> {
    if label_uses_rightmost_whole_group_anchor(node_label) {
        return glyph_count.checked_sub(1);
    }
    let chars = label_visible_chars(node_label);
    if chars.len() != glyph_count {
        return None;
    }
    let grouped_text = chars.iter().collect::<String>();
    let groups = split_label_groups(&grouped_text);
    let rightmost_group = groups.last()?;
    let group_start = chars.len().checked_sub(rightmost_group.chars().count())?;
    let anchor_char = group_start + crate::terminal_letter_anchor_offset(rightmost_group);
    Some(anchor_char)
}

pub(super) fn label_uses_rightmost_whole_group_anchor(node_label: &crate::NodeLabel) -> bool {
    let recognition = node_label.meta.get("labelRecognition");
    recognition
        .and_then(|meta| meta.get("canonicalLabel"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(crate::canonical_abbreviation_uses_whole_label_layout)
        || recognition
            .and_then(|meta| meta.get("status"))
            .and_then(serde_json::Value::as_str)
            == Some("invalid")
}

pub(super) fn angle_uses_vertical_label_anchor(angle: f64) -> bool {
    angular_distance(angle, 90.0) <= 7.5 || angular_distance(angle, 270.0) <= 7.5
}

pub(super) fn resolved_anchor_point_for_angle(
    _document: &ChemcoreDocument,
    anchor: &BondAnchor,
    angle: f64,
) -> Point {
    let Some(label_anchor) = anchor.label_anchor.as_ref() else {
        return anchor.point;
    };
    if angle_uses_vertical_label_anchor(angle) {
        return label_anchor.glyph_point;
    }
    let direction = direction_from_angle(angle);
    if direction.x < -1.0e-6 {
        return label_anchor.left_point;
    }
    if direction.x > 1.0e-6 {
        if label_anchor.glyph_index == label_anchor.rightmost_glyph_index {
            return label_anchor.glyph_point;
        }
        return label_anchor
            .right_group_point
            .unwrap_or(label_anchor.right_point);
    }
    label_anchor.glyph_point
}

pub(super) fn point_in_bond_center_focus(point: Point, start: Point, end: Point) -> bool {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return false;
    }
    let focus_length = bond_center_focus_length(start, end);
    if focus_length <= crate::EPSILON {
        return false;
    }
    let center = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    let ux = dx / length;
    let uy = dy / length;
    let local_x = (point.x - center.x) * ux + (point.y - center.y) * uy;
    let local_y = -(point.x - center.x) * uy + (point.y - center.y) * ux;
    local_x.abs() <= focus_length / 2.0 && local_y.abs() <= BOND_CENTER_FOCUS_WIDTH / 2.0
}

pub fn bond_center_focus_length(start: Point, end: Point) -> f64 {
    let length = start.distance(end);
    (length * 0.5).max(0.0)
}

pub(super) fn bond_center_focus_radius(start: Point, end: Point) -> f64 {
    let half_length = bond_center_focus_length(start, end) / 2.0;
    let half_width = BOND_CENTER_FOCUS_WIDTH / 2.0;
    half_length.hypot(half_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_event_world_point_round_trip() {
        let event = PointerEvent::from_world_point(
            WorldPoint::new(WorldPt(7.94), WorldPt(6.88)),
            Some(0),
            true,
        );
        assert_eq!(
            event.world_point(),
            WorldPoint::new(WorldPt(7.94), WorldPt(6.88))
        );
        assert_eq!(event.point(), Point::new(7.94, 6.88));
    }

    #[test]
    fn editor_options_accessors_expose_world_pt() {
        let options = EditorOptions {
            bond_length: 30.0,
            bond_stroke_width: 1.0,
            bold_bond_width: 4.0,
            wedge_width: 6.0,
            label_clip_margin: 1.2,
            hash_spacing: 2.7,
            bond_spacing: 12.0,
            margin_width: 2.0,
            graphic_stroke_width: 1.0,
        };
        assert_eq!(options.bond_length_world_pt(), WorldPt(30.0));
        assert_eq!(options.bond_stroke_world_pt(), WorldPt(1.0));
        assert_eq!(options.bold_bond_width_world_pt(), WorldPt(4.0));
        assert_eq!(options.wedge_width_world_pt(), WorldPt(6.0));
        assert_eq!(options.label_clip_margin_world_pt(), WorldPt(1.2));
        assert_eq!(options.hash_spacing_world_pt(), WorldPt(2.7));
        assert_eq!(options.bond_spacing_percent(), 12.0);
        assert_eq!(options.graphic_stroke_world_pt(), WorldPt(1.0));
    }

    #[test]
    fn arrow_center_hover_shows_only_endpoints_and_center_without_heads() {
        let document = arrow_hover_test_document("none", "none", "none", "none");
        let hover = hit_test_arrow_center(&document, Point::new(50.0, 0.0), 5.0)
            .expect("arrow center should focus");
        assert_eq!(
            hover.handles,
            vec![
                Point::new(0.0, 0.0),
                Point::new(50.0, 0.0),
                Point::new(100.0, 0.0),
            ]
        );
    }

    #[test]
    fn arrow_center_hover_uses_one_style_point_per_arrowhead() {
        let document = arrow_hover_test_document("none", "none", "full", "full");
        let hover = hit_test_arrow_center(&document, Point::new(50.0, 0.0), 5.0)
            .expect("arrow center should focus");
        assert_eq!(hover.handles.len(), 5);
        assert!(hover.handles.contains(&Point::new(85.0, 3.75)));
        assert!(hover.handles.contains(&Point::new(15.0, -3.75)));
    }

    #[test]
    fn curved_arrow_center_hover_uses_arc_midpoint_and_half_endpoint_side() {
        let document =
            arrow_hover_test_document_with_curve("none", "none", "half-left", "none", -120.0);
        assert!(
            hit_test_arrow_center(&document, Point::new(50.0, 0.0), 5.0).is_none(),
            "curved arrow hover should not use the chord center"
        );
        let hover = hit_test_arrow_center(&document, Point::new(50.0, -28.0), 5.0)
            .expect("curved arrow center should focus near the arc");
        assert_eq!(hover.handles.len(), 4);
        assert!(hover.center.y < -25.0);
        assert!(hover
            .handles
            .iter()
            .any(|point| point.distance(Point::new(100.0, 0.0)) > 1.0));
    }

    #[test]
    fn curved_double_arrow_hover_respects_each_endpoint_style() {
        let document =
            arrow_hover_test_document_with_curve("none", "none", "half-left", "full", -120.0);
        let hover = hit_test_arrow_center(&document, Point::new(50.0, -28.0), 5.0)
            .expect("curved arrow center should focus near the arc");
        assert_eq!(
            hover.handles.len(),
            5,
            "base handles plus one style handle per enabled endpoint"
        );
    }

    fn arrow_hover_test_document(
        head: &str,
        tail: &str,
        arrow_head: &str,
        arrow_tail: &str,
    ) -> ChemcoreDocument {
        arrow_hover_test_document_with_curve(head, tail, arrow_head, arrow_tail, 0.0)
    }

    fn arrow_hover_test_document_with_curve(
        head: &str,
        tail: &str,
        arrow_head: &str,
        arrow_tail: &str,
        curve: f64,
    ) -> ChemcoreDocument {
        let mut document: ChemcoreDocument = serde_json::from_value(serde_json::json!({
            "format": { "name": "chemcore", "version": "0.1" },
            "document": {
                "id": "doc_test",
                "title": "test",
                "page": { "width": 200.0, "height": 100.0, "background": "#ffffff" }
            },
            "styles": {},
            "objects": [{
                "id": "obj_line_001",
                "type": "line",
                "visible": true,
                "zIndex": 10,
                "transform": { "translate": [0.0, 0.0], "rotate": 0.0, "scale": [1.0, 1.0] },
                "payload": {
                    "points": [[0.0, 0.0], [100.0, 0.0]],
                    "head": head,
                    "tail": tail,
                    "arrowHead": {
                        "length": 15.0,
                        "width": 3.75,
                        "head": arrow_head,
                        "tail": arrow_tail,
                        "curve": curve
                    }
                }
            }],
            "resources": {}
        }))
        .expect("document should deserialize");
        crate::normalize_arrow_object_payloads(&mut document);
        document
    }
}
