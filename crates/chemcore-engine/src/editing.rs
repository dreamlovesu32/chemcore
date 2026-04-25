use crate::{
    angle_between, angle_in_clockwise_arc, angular_distance, direction_from_angle,
    largest_angular_gap, normalize_angle, split_label_groups, ChemcoreDocument, EditableFragment,
    Node, Point, DEFAULT_BOND_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const ENDPOINT_FOCUS_RADIUS: f64 = 4.5;
pub const ENDPOINT_HIT_RADIUS: f64 = 9.0;
pub const BOND_HIT_RADIUS: f64 = 6.0;
pub const BOND_CENTER_FOCUS_LENGTH: f64 = 18.0;
pub const BOND_CENTER_FOCUS_WIDTH: f64 = 9.0;
pub const BOND_CENTER_HIT_RADIUS: f64 = BOND_CENTER_FOCUS_LENGTH;
pub const DRAG_START_THRESHOLD: f64 = 4.0;
pub const GLOBAL_SNAP_ANGLES: &[f64] = &[
    0.0, 30.0, 45.0, 60.0, 90.0, 120.0, 135.0, 150.0, 180.0, 210.0, 225.0, 240.0, 270.0, 300.0,
    315.0, 330.0,
];
pub const RELATIVE_BOND_ANGLES: &[f64] = &[30.0, 60.0, 90.0, 120.0, 150.0, 180.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    Select,
    Bond,
    Text,
    Shape,
    Templates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BondVariant {
    Single,
    Double,
    Triple,
    Dashed,
    DashedDouble,
    Bold,
    BoldDashed,
    Wedge,
    HashedWedge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorOptions {
    pub bond_length: f64,
    pub bond_stroke_width: f64,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            bond_length: DEFAULT_BOND_LENGTH,
            bond_stroke_width: crate::DEFAULT_BOND_STROKE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    pub active_tool: Tool,
    pub bond_variant: BondVariant,
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            active_tool: Tool::Bond,
            bond_variant: BondVariant::Single,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerEvent {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub button: Option<u8>,
    #[serde(default)]
    pub alt_key: bool,
}

impl PointerEvent {
    pub fn point(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointHit {
    pub node_id: String,
    pub point: Point,
    pub distance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_anchor: Option<LabelAnchorGeometry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondHit {
    pub bond_id: String,
    pub begin: Point,
    pub end: Point,
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondCenterHit {
    pub bond_id: String,
    pub point: Point,
    pub begin: Point,
    pub end: Point,
    pub order: u8,
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SelectionState {
    pub nodes: Vec<String>,
    pub bonds: Vec<String>,
}

impl SelectionState {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.bonds.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondAnchor {
    pub node_id: Option<String>,
    pub point: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_anchor: Option<LabelAnchorGeometry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelAnchorGeometry {
    pub glyph_index: usize,
    pub glyph_point: Point,
    pub first_glyph_point: Point,
    pub left_point: Point,
    pub right_point: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_group_point: Option<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DragState {
    pub anchor: BondAnchor,
    pub start: Point,
    pub has_dragged: bool,
    pub free_length: bool,
    pub preview_end: Option<Point>,
    pub target: Option<BondAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub hover_endpoint: Option<EndpointHit>,
    pub hover_bond_center: Option<BondCenterHit>,
    pub preview: Option<BondPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondPreview {
    pub start: Point,
    pub end: Point,
}

pub fn can_draw_bond(tool_state: &ToolState) -> bool {
    tool_state.active_tool == Tool::Bond
}

pub fn can_focus_bond_center(tool_state: &ToolState) -> bool {
    tool_state.active_tool == Tool::Bond
}

pub fn can_focus_endpoint(tool_state: &ToolState) -> bool {
    tool_state.active_tool == Tool::Bond
}

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
    let entry = document.editable_fragment()?;
    let mut best: Option<EndpointHit> = None;
    for node in &entry.fragment.nodes {
        if excluded_node_id == Some(node.id.as_str()) {
            continue;
        }
        let node_point = entry.world_point_for_node(node);
        let distance = point.distance(node_point);
        if distance <= radius && best.as_ref().map_or(true, |hit| distance < hit.distance) {
            best = Some(EndpointHit {
                node_id: node.id.clone(),
                point: node_point,
                distance,
                label_anchor: None,
            });
        }
        for label_anchor in label_anchor_geometries(&entry, node) {
            let distance = point.distance(label_anchor.glyph_point);
            if distance <= radius && best.as_ref().map_or(true, |hit| distance < hit.distance) {
                best = Some(EndpointHit {
                    node_id: node.id.clone(),
                    point: label_anchor.glyph_point,
                    distance,
                    label_anchor: Some(label_anchor),
                });
            }
        }
    }
    best
}

pub fn hit_test_bond(document: &ChemcoreDocument, point: Point, radius: f64) -> Option<BondHit> {
    let entry = document.editable_fragment()?;
    let mut best: Option<BondHit> = None;
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
    best
}

pub fn hit_test_bond_center(
    document: &ChemcoreDocument,
    point: Point,
    radius: f64,
) -> Option<BondCenterHit> {
    let entry = document.editable_fragment()?;
    let mut best: Option<BondCenterHit> = None;
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
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
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
    best
}

pub fn select_at(document: &ChemcoreDocument, point: Point) -> SelectionState {
    if let Some(endpoint) = hit_test_endpoint(document, point, ENDPOINT_HIT_RADIUS) {
        return SelectionState {
            nodes: vec![endpoint.node_id],
            bonds: Vec::new(),
        };
    }
    if let Some(bond) = hit_test_bond(document, point, BOND_HIT_RADIUS) {
        return SelectionState {
            nodes: Vec::new(),
            bonds: vec![bond.bond_id],
        };
    }
    SelectionState::default()
}

pub fn anchor_from_point(document: &ChemcoreDocument, point: Point) -> Option<BondAnchor> {
    if let Some(hit) = hit_test_endpoint(document, point, ENDPOINT_HIT_RADIUS) {
        return Some(BondAnchor {
            node_id: Some(hit.node_id),
            point: hit.point,
            label_anchor: hit.label_anchor,
        });
    }
    document.editable_fragment()?;
    Some(BondAnchor {
        node_id: None,
        point,
        label_anchor: None,
    })
}

pub fn adjacent_directions(entry: &EditableFragment<'_>, node_id: &str) -> Vec<f64> {
    let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Vec::new();
    };
    let point = entry.world_point_for_node(node);
    let mut out = Vec::new();
    for bond in &entry.fragment.bonds {
        if bond.begin != node_id && bond.end != node_id {
            continue;
        }
        let other_id = if bond.begin == node_id {
            &bond.end
        } else {
            &bond.begin
        };
        let Some(other) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| &node.id == other_id)
        else {
            continue;
        };
        out.push(angle_between(point, entry.world_point_for_node(other)));
    }
    out
}

fn default_angle_for_anchor_with_single_neighbor_delta(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    single_neighbor_delta: f64,
) -> f64 {
    let Some(node_id) = &anchor.node_id else {
        return 0.0;
    };
    let Some(entry) = document.editable_fragment() else {
        return 0.0;
    };
    let directions = adjacent_directions(&entry, node_id);
    match directions.len() {
        0 => 0.0,
        1 => {
            let a = normalize_angle(directions[0] + single_neighbor_delta);
            let b = normalize_angle(directions[0] - single_neighbor_delta);
            let da = direction_from_angle(a);
            let db = direction_from_angle(b);
            if (da.y - db.y).abs() > 1.0e-9 {
                if da.y < db.y {
                    a
                } else {
                    b
                }
            } else if da.x > db.x {
                a
            } else {
                b
            }
        }
        _ => largest_angular_gap(&directions).center,
    }
}

pub fn default_angle_for_anchor(document: &ChemcoreDocument, anchor: &BondAnchor) -> f64 {
    default_angle_for_anchor_with_single_neighbor_delta(document, anchor, 120.0)
}

pub fn default_angle_for_anchor_for_variant(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    bond_variant: BondVariant,
) -> f64 {
    if bond_variant == BondVariant::Triple {
        if let Some(node_id) = &anchor.node_id {
            if let Some(entry) = document.editable_fragment() {
                let directions = adjacent_directions(&entry, node_id);
                if directions.len() == 1 {
                    return normalize_angle(directions[0] + 180.0);
                }
            }
        }
    }
    let single_neighbor_delta = if bond_variant == BondVariant::Triple {
        180.0
    } else {
        120.0
    };
    default_angle_for_anchor_with_single_neighbor_delta(document, anchor, single_neighbor_delta)
}

pub fn snapped_angle_for_anchor(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    mouse: Point,
) -> f64 {
    let mouse_angle = angle_between(anchor.point, mouse);
    let directions = anchor
        .node_id
        .as_ref()
        .and_then(|node_id| {
            document
                .editable_fragment()
                .map(|entry| adjacent_directions(&entry, node_id))
        })
        .unwrap_or_default();

    if directions.is_empty() {
        return nearest_angle(mouse_angle, GLOBAL_SNAP_ANGLES);
    }

    let mut candidates = HashSet::new();
    for angle in GLOBAL_SNAP_ANGLES {
        candidates.insert((*angle * 1000.0).round() as i32);
    }
    for base in &directions {
        for relative in RELATIVE_BOND_ANGLES {
            candidates.insert((normalize_angle(base + relative) * 1000.0).round() as i32);
            candidates.insert((normalize_angle(base - relative) * 1000.0).round() as i32);
        }
    }

    let gap = largest_angular_gap(&directions);
    let mut best = 0.0;
    let mut best_score = f64::INFINITY;
    for candidate_key in candidates {
        let candidate = candidate_key as f64 / 1000.0;
        let mut score = angular_distance(candidate, mouse_angle);
        if directions.len() >= 2 && !angle_in_clockwise_arc(candidate, gap.start, gap.end) {
            score += 25.0;
        }
        if directions.len() >= 2 {
            let satisfied = directions
                .iter()
                .filter(|direction| {
                    RELATIVE_BOND_ANGLES.iter().any(|allowed| {
                        (angular_distance(candidate, **direction) - allowed).abs() < 0.001
                    })
                })
                .count();
            score += (directions.len() - satisfied) as f64 * 8.0;
        }
        if score < best_score {
            best_score = score;
            best = candidate;
        }
    }
    normalize_angle(best)
}

pub fn endpoint_from_angle(anchor: &BondAnchor, angle: f64, length: f64) -> Point {
    anchor
        .point
        .translated(direction_from_angle(angle).scaled(length))
}

pub fn endpoint_from_angle_for_document(
    document: &ChemcoreDocument,
    anchor: &BondAnchor,
    angle: f64,
    length: f64,
) -> Point {
    resolved_anchor_point_for_angle(document, anchor, angle)
        .translated(direction_from_angle(angle).scaled(length))
}

pub fn nearest_angle(target: f64, candidates: &[f64]) -> f64 {
    candidates
        .iter()
        .copied()
        .min_by(|a, b| angular_distance(*a, target).total_cmp(&angular_distance(*b, target)))
        .unwrap_or(0.0)
}

pub fn node_by_id<'a>(nodes: &'a [Node], node_id: &str) -> Option<&'a Node> {
    nodes.iter().find(|node| node.id == node_id)
}

fn point_to_segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= crate::EPSILON {
        return point.distance(start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0);
    point.distance(Point::new(start.x + dx * t, start.y + dy * t))
}

fn label_anchor_geometries(entry: &EditableFragment<'_>, node: &Node) -> Vec<LabelAnchorGeometry> {
    let Some(label) = node.label.as_ref() else {
        return Vec::new();
    };
    if label.glyph_polygons.is_empty() {
        return Vec::new();
    }

    let glyph_points: Vec<Point> = label
        .glyph_polygons()
        .into_iter()
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
    let right_point = glyph_points
        .iter()
        .copied()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap_or(first_glyph_point);
    let right_group_index = rightmost_group_anchor_index(label, glyph_points.len());
    let right_group_point = right_group_index.and_then(|index| glyph_points.get(index).copied());

    glyph_points
        .iter()
        .enumerate()
        .map(|(glyph_index, glyph_point)| LabelAnchorGeometry {
            glyph_index,
            glyph_point: *glyph_point,
            first_glyph_point,
            left_point,
            right_point,
            right_group_point,
        })
        .collect()
}

fn polygon_anchor_point(polygon: &[Point]) -> Option<Point> {
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

fn label_visible_chars(node_label: &crate::NodeLabel) -> Vec<char> {
    node_label
        .source_text
        .as_deref()
        .unwrap_or(node_label.text.as_str())
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn rightmost_group_anchor_index(
    node_label: &crate::NodeLabel,
    glyph_count: usize,
) -> Option<usize> {
    let chars = label_visible_chars(node_label);
    if chars.len() != glyph_count {
        return None;
    }
    let grouped_text = chars.iter().collect::<String>();
    let groups = split_label_groups(&grouped_text);
    let rightmost_group = groups.last()?;
    let anchor_char = chars.len().checked_sub(rightmost_group.chars().count())?;
    Some(anchor_char)
}

fn anchor_has_existing_connections(document: &ChemcoreDocument, anchor: &BondAnchor) -> bool {
    let Some(node_id) = anchor.node_id.as_deref() else {
        return false;
    };
    let Some(entry) = document.editable_fragment() else {
        return false;
    };
    !adjacent_directions(&entry, node_id).is_empty()
}

fn angle_uses_vertical_label_anchor(angle: f64) -> bool {
    angular_distance(angle, 90.0) <= 7.5 || angular_distance(angle, 270.0) <= 7.5
}

fn resolved_anchor_point_for_angle(
    document: &ChemcoreDocument,
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
        if anchor_has_existing_connections(document, anchor) {
            return label_anchor
                .right_group_point
                .unwrap_or(label_anchor.right_point);
        }
        return label_anchor.right_point;
    }
    label_anchor.glyph_point
}

fn point_in_bond_center_focus(point: Point, start: Point, end: Point) -> bool {
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
    (length - ENDPOINT_FOCUS_RADIUS * 2.0)
        .max(0.0)
        .min(BOND_CENTER_FOCUS_LENGTH)
}

fn bond_center_focus_radius(start: Point, end: Point) -> f64 {
    let half_length = bond_center_focus_length(start, end) / 2.0;
    let half_width = BOND_CENTER_FOCUS_WIDTH / 2.0;
    half_length.hypot(half_width)
}
