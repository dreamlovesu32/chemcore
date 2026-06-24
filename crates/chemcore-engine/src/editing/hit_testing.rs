use super::*;

pub fn hit_test_endpoint(
    document: &ChemcoreDocument,
    point: Point,
    radius: f64,
) -> Option<EndpointHit> {
    hit_test_endpoint_excluding(document, point, radius, None)
}

pub fn hit_test_endpoint_excluding(
    document: &ChemcoreDocument,
    point: Point,
    radius: f64,
    excluded_node_id: Option<&str>,
) -> Option<EndpointHit> {
    let mut best: Option<(EndpointHit, bool)> = None;
    for entry in document.editable_fragments() {
        for node in &entry.fragment.nodes {
            if excluded_node_id == Some(node.id.as_str()) {
                continue;
            }
            let label_anchors = label_anchor_geometries(&entry, node);
            if !label_anchors.is_empty() {
                for label_anchor in label_anchors {
                    let distance = point.distance(label_anchor.glyph_point);
                    let inside_box = point_in_box(point, label_anchor.glyph_box);
                    if !inside_box && distance > radius {
                        continue;
                    }
                    let mut anchor = label_anchor;
                    if label_has_implicit_hydrogens(node) {
                        let node_point = entry.world_point_for_node(node);
                        anchor.glyph_point = node_point;
                        anchor.first_glyph_point = node_point;
                        anchor.left_point = node_point;
                        anchor.right_point = node_point;
                        anchor.right_group_point = Some(node_point);
                    }
                    let candidate = EndpointHit {
                        node_id: node.id.clone(),
                        object_id: entry.object.id.clone(),
                        point: anchor.glyph_point,
                        distance,
                        label_anchor: Some(anchor),
                    };
                    if endpoint_candidate_is_better(&best, inside_box, distance) {
                        best = Some((candidate, inside_box));
                    }
                }
                continue;
            }
            let node_point = entry.world_point_for_node(node);
            let distance = point.distance(node_point);
            if distance <= radius && endpoint_candidate_is_better(&best, false, distance) {
                best = Some((
                    EndpointHit {
                        node_id: node.id.clone(),
                        object_id: entry.object.id.clone(),
                        point: node_point,
                        distance,
                        label_anchor: None,
                    },
                    false,
                ));
            }
        }
    }
    best.map(|(hit, _)| hit)
}

pub(super) fn label_has_implicit_hydrogens(node: &Node) -> bool {
    node.num_hydrogens > 0
        && node.atomic_number != 1
        && !node.element.is_empty()
        && node.label.is_some()
}

pub(super) fn endpoint_candidate_is_better(
    best: &Option<(EndpointHit, bool)>,
    inside_box: bool,
    distance: f64,
) -> bool {
    let Some((current, current_inside_box)) = best.as_ref() else {
        return true;
    };
    if inside_box != *current_inside_box {
        return inside_box;
    }
    distance < current.distance
}

pub fn hit_test_bond(document: &ChemcoreDocument, point: Point, radius: f64) -> Option<BondHit> {
    let mut best: Option<BondHit> = None;
    for entry in document.editable_fragments() {
        for bond in &entry.fragment.bonds {
            let Some(begin) = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.begin)
            else {
                continue;
            };
            let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
                continue;
            };
            let begin_point = entry.world_point_for_node(begin);
            let end_point = entry.world_point_for_node(end);
            let distance = point_to_segment_distance(point, begin_point, end_point);
            if distance <= radius && best.as_ref().map_or(true, |hit| distance < hit.distance) {
                best = Some(BondHit {
                    bond_id: bond.id.clone(),
                    begin: begin_point,
                    end: end_point,
                    distance,
                });
            }
        }
    }
    best
}

pub fn hit_test_bond_center(
    document: &ChemcoreDocument,
    point: Point,
    radius: f64,
) -> Option<BondCenterHit> {
    let mut best: Option<BondCenterHit> = None;
    for entry in document.editable_fragments() {
        for bond in &entry.fragment.bonds {
            if bond.order < 1 {
                continue;
            }
            let Some(begin) = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.begin)
            else {
                continue;
            };
            let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
                continue;
            };
            let raw_begin = entry.world_point_for_node(begin);
            let raw_end = entry.world_point_for_node(end);
            let (begin_point, end_point) = visual_bond_center_focus_segment(
                document, &entry, bond, begin, end, raw_begin, raw_end,
            );
            let center = Point::new(
                (begin_point.x + end_point.x) / 2.0,
                (begin_point.y + end_point.y) / 2.0,
            );
            let distance = point.distance(center);
            let focus_radius = bond_center_focus_radius(begin_point, end_point);
            if point_in_bond_center_focus(point, begin_point, end_point)
                && distance <= radius.max(focus_radius)
                && best.as_ref().map_or(true, |hit| distance < hit.distance)
            {
                best = Some(BondCenterHit {
                    bond_id: bond.id.clone(),
                    point: center,
                    begin: begin_point,
                    end: end_point,
                    order: bond.order,
                    distance,
                });
            }
        }
    }
    best
}

fn visual_bond_center_focus_segment(
    document: &ChemcoreDocument,
    entry: &EditableFragment<'_>,
    bond: &Bond,
    begin: &Node,
    end: &Node,
    raw_begin: Point,
    raw_end: Point,
) -> (Point, Point) {
    let begin_has_label = begin
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text());
    let end_has_label = end
        .label
        .as_ref()
        .is_some_and(|label| label.has_visible_text());
    if !begin_has_label && !end_has_label {
        return (raw_begin, raw_end);
    }
    let Some(bounds) = fragment_bond_visual_bounds(document, entry.object, entry.fragment, bond)
    else {
        return (raw_begin, raw_end);
    };
    let raw_center = Point::new(
        (raw_begin.x + raw_end.x) * 0.5,
        (raw_begin.y + raw_end.y) * 0.5,
    );
    let visual_center = Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5);
    let offset = Vector::new(
        visual_center.x - raw_center.x,
        visual_center.y - raw_center.y,
    );
    (
        Point::new(raw_begin.x + offset.x, raw_begin.y + offset.y),
        Point::new(raw_end.x + offset.x, raw_end.y + offset.y),
    )
}

pub fn hit_test_arrow_center(
    document: &ChemcoreDocument,
    point: Point,
    radius: f64,
) -> Option<HoverArrow> {
    let mut best: Option<(f64, HoverArrow)> = None;
    for object in document.scene_objects() {
        if object.object_type != "line" || !object.visible {
            continue;
        }
        let points = line_object_points(object);
        if points.len() < 2 {
            continue;
        }
        let focus_points = arrow_object_focus_points(object, &points);
        if focus_points.len() < 2 {
            continue;
        }
        let center =
            point_at_distance_from_start(&focus_points, polyline_length(&focus_points) * 0.5)
                .unwrap_or_else(|| {
                    let start = focus_points[0];
                    let end = *focus_points.last().unwrap_or(&focus_points[0]);
                    Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5)
                });
        let handles = arrow_object_handle_points(object, &points);
        let handle_distance = handles
            .iter()
            .map(|handle| handle.distance(point))
            .min_by(|left, right| left.total_cmp(right))
            .unwrap_or(f64::INFINITY);
        let distance = point_to_polyline_distance(point, &focus_points).min(handle_distance);
        if distance > radius
            || best
                .as_ref()
                .is_some_and(|(current, _)| distance >= *current)
        {
            continue;
        }
        best = Some((
            distance,
            HoverArrow {
                object_id: object.id.clone(),
                center,
                handles,
            },
        ));
    }
    best.map(|(_, hit)| hit)
}

pub fn select_at(document: &ChemcoreDocument, point: Point) -> SelectionState {
    if let Some(endpoint) = hit_test_endpoint(document, point, ENDPOINT_HIT_RADIUS) {
        return SelectionState {
            text_objects: Vec::new(),
            arrow_objects: Vec::new(),
            label_nodes: Vec::new(),
            region: false,
            nodes: vec![endpoint.node_id],
            bonds: Vec::new(),
        };
    }
    if let Some(bond) = hit_test_bond(document, point, BOND_HIT_RADIUS) {
        return SelectionState {
            text_objects: Vec::new(),
            arrow_objects: Vec::new(),
            label_nodes: Vec::new(),
            region: false,
            nodes: Vec::new(),
            bonds: vec![bond.bond_id],
        };
    }
    SelectionState::default()
}
