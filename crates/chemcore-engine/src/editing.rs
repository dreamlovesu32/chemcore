use crate::{
    angle_between, angle_in_clockwise_arc, angular_distance, direction_from_angle,
    largest_angular_gap, normalize_angle, ChemcoreDocument, EditableFragment, Node, Point,
    DEFAULT_BOND_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const ENDPOINT_HIT_RADIUS: f64 = 16.0;
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
            active_tool: Tool::Select,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondAnchor {
    pub node_id: Option<String>,
    pub point: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DragState {
    pub anchor: BondAnchor,
    pub start: Point,
    pub has_dragged: bool,
    pub preview_end: Option<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub hover_endpoint: Option<EndpointHit>,
    pub preview: Option<BondPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondPreview {
    pub start: Point,
    pub end: Point,
}

pub fn can_draw_single_bond(tool_state: &ToolState) -> bool {
    tool_state.active_tool == Tool::Bond && tool_state.bond_variant == BondVariant::Single
}

pub fn can_focus_endpoint(tool_state: &ToolState) -> bool {
    tool_state.active_tool == Tool::Bond
}

pub fn hit_test_endpoint(
    document: &ChemcoreDocument,
    point: Point,
    radius: f64,
) -> Option<EndpointHit> {
    let entry = document.editable_fragment()?;
    let mut best: Option<EndpointHit> = None;
    for node in &entry.fragment.nodes {
        let node_point = entry.world_point_for_node(node);
        let distance = point.distance(node_point);
        if distance <= radius && best.as_ref().map_or(true, |hit| distance < hit.distance) {
            best = Some(EndpointHit {
                node_id: node.id.clone(),
                point: node_point,
                distance,
            });
        }
    }
    best
}

pub fn anchor_from_point(document: &ChemcoreDocument, point: Point) -> Option<BondAnchor> {
    if let Some(hit) = hit_test_endpoint(document, point, ENDPOINT_HIT_RADIUS) {
        return Some(BondAnchor {
            node_id: Some(hit.node_id),
            point: hit.point,
        });
    }
    document.editable_fragment()?;
    Some(BondAnchor {
        node_id: None,
        point,
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

pub fn default_angle_for_anchor(document: &ChemcoreDocument, anchor: &BondAnchor) -> f64 {
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
            let a = normalize_angle(directions[0] + 120.0);
            let b = normalize_angle(directions[0] - 120.0);
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
