use crate::{
    anchor_from_point, bond_center_focus_length, can_draw_single_bond, can_focus_bond_center,
    can_focus_endpoint, default_angle_for_anchor, endpoint_from_angle, hit_test_bond_center,
    hit_test_endpoint, hit_test_endpoint_excluding, render_document, select_at,
    snapped_angle_for_anchor, Bond, BondAnchor, BondPreview, ChemcoreDocument, DoubleBond,
    DoubleBondPlacement, DragState, EditorOptions, EndpointHit, OverlayState, Point, PointerEvent,
    RenderPrimitive, RenderRole, SelectionState, Tool, ToolState, BOND_CENTER_FOCUS_WIDTH,
    BOND_CENTER_HIT_RADIUS, DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS,
    ENDPOINT_HIT_RADIUS,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub document: ChemcoreDocument,
    pub tool: ToolState,
    pub selection: SelectionState,
    pub overlay: OverlayState,
}

pub struct Engine {
    state: EngineState,
    drag: Option<DragState>,
    options: EditorOptions,
    next_id: u64,
    undo_stack: Vec<ChemcoreDocument>,
    redo_stack: Vec<ChemcoreDocument>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState {
                document: ChemcoreDocument::blank(),
                tool: ToolState::default(),
                selection: SelectionState::default(),
                overlay: OverlayState::default(),
            },
            drag: None,
            options: EditorOptions::default(),
            next_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn state(&self) -> &EngineState {
        &self.state
    }

    pub fn state_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.state)
    }

    pub fn document_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.state.document)
    }

    pub fn render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = render_document(&self.state.document);
        out.extend(self.selection_render_list());
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            out.push(RenderPrimitive::Circle {
                role: RenderRole::HoverEndpoint,
                center: hover.point,
                radius: ENDPOINT_FOCUS_RADIUS,
                fill: "rgba(47,111,237,0.24)".to_string(),
                stroke: "rgba(47,111,237,0.78)".to_string(),
                stroke_width: 1.4,
            });
        }
        if let Some(hover) = &self.state.overlay.hover_bond_center {
            let focus_length = bond_center_focus_length(hover.begin, hover.end);
            if focus_length > crate::EPSILON {
                out.push(RenderPrimitive::Polygon {
                    role: RenderRole::HoverBondCenter,
                    points: centered_oriented_rect_points(
                        hover.begin,
                        hover.end,
                        focus_length,
                        BOND_CENTER_FOCUS_WIDTH,
                    ),
                    fill: "rgba(47,111,237,0.11)".to_string(),
                    stroke: "rgba(47,111,237,0.72)".to_string(),
                    stroke_width: 1.2,
                });
            }
        }
        if let Some(preview) = &self.state.overlay.preview {
            out.push(RenderPrimitive::Line {
                role: RenderRole::PreviewBond,
                from: preview.start,
                to: preview.end,
                stroke: "rgba(0,0,0,0.72)".to_string(),
                stroke_width: self.options.bond_stroke_width,
            });
            out.push(RenderPrimitive::Circle {
                role: RenderRole::PreviewEnd,
                center: preview.end,
                radius: 5.0,
                fill: "#ffffff".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: 1.2,
            });
        }
        out
    }

    pub fn set_tool_state(&mut self, tool: ToolState) {
        self.state.tool = tool;
        self.clear_interaction();
    }

    pub fn pointer_move(&mut self, event: PointerEvent) {
        let point = event.point();
        if self.state.tool.active_tool == Tool::Select {
            self.clear_interaction();
            return;
        }
        if !can_focus_endpoint(&self.state.tool) {
            self.clear_interaction();
            return;
        }

        if let Some(mut drag) = self.drag.take() {
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                self.state.overlay.hover_endpoint = None;
                let target = self.drag_target_endpoint(&drag.anchor, point);
                let end = if let Some(target) = target {
                    self.state.overlay.hover_endpoint = Some(target.clone());
                    drag.target = Some(BondAnchor {
                        node_id: Some(target.node_id.clone()),
                        point: target.point,
                    });
                    target.point
                } else {
                    drag.target = None;
                    let angle = snapped_angle_for_anchor(&self.state.document, &drag.anchor, point);
                    endpoint_from_angle(&drag.anchor, angle, self.options.bond_length)
                };
                drag.preview_end = Some(end);
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.anchor.point,
                    end,
                });
            }
            self.drag = Some(drag);
            return;
        }

        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        if self.state.tool.bond_variant == crate::BondVariant::Single {
            if let Some(endpoint) =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            {
                self.state.overlay.hover_endpoint = Some(endpoint);
                return;
            }
        }
        if can_focus_bond_center(&self.state.tool) {
            if let Some(center) =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
            {
                self.state.overlay.hover_bond_center = Some(center);
                return;
            }
        }
        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
    }

    pub fn pointer_down(&mut self, event: PointerEvent) {
        if self.state.tool.active_tool == Tool::Select {
            self.state.selection = select_at(&self.state.document, event.point());
            self.clear_interaction();
            return;
        }
        if !can_draw_single_bond(&self.state.tool) {
            if can_focus_bond_center(&self.state.tool) {
                if let Some(hit) = hit_test_bond_center(
                    &self.state.document,
                    event.point(),
                    BOND_CENTER_HIT_RADIUS,
                ) {
                    self.cycle_bond_center_style(&hit.bond_id);
                }
            }
            return;
        }
        let point = event.point();
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.drag = Some(DragState {
                anchor: BondAnchor {
                    node_id: Some(endpoint.node_id),
                    point: endpoint.point,
                },
                start: point,
                has_dragged: false,
                preview_end: None,
                target: None,
            });
            return;
        }
        if let Some(hit) = hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            self.cycle_bond_center_style(&hit.bond_id);
            return;
        }
        let Some(anchor) = anchor_from_point(&self.state.document, point) else {
            return;
        };
        self.drag = Some(DragState {
            anchor,
            start: point,
            has_dragged: false,
            preview_end: None,
            target: None,
        });
    }

    pub fn pointer_up(&mut self, event: PointerEvent) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let end_anchor = if drag.has_dragged {
            if let Some(target) = drag.target {
                target
            } else {
                let end = drag.preview_end.unwrap_or_else(|| {
                    let angle =
                        snapped_angle_for_anchor(&self.state.document, &drag.anchor, event.point());
                    endpoint_from_angle(&drag.anchor, angle, self.options.bond_length)
                });
                BondAnchor {
                    node_id: None,
                    point: end,
                }
            }
        } else {
            let angle = default_angle_for_anchor(&self.state.document, &drag.anchor);
            let end = endpoint_from_angle(&drag.anchor, angle, self.options.bond_length);
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    point: end,
                })
        };
        self.state.overlay.preview = None;
        self.add_single_bond_between(drag.anchor, end_anchor);
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.state.overlay = OverlayState::default();
    }

    pub fn add_single_bond(&mut self, anchor: BondAnchor, end: Point) {
        self.add_single_bond_between(
            anchor,
            BondAnchor {
                node_id: None,
                point: end,
            },
        );
    }

    pub fn add_single_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor) -> bool {
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists(begin_id, end_id) {
                return false;
            }
        }
        self.push_undo_snapshot();
        self.state.selection = SelectionState::default();
        let begin_id = match anchor.node_id {
            Some(node_id) => node_id,
            None => self.insert_carbon(anchor.point),
        };
        let end_id = match end.node_id {
            Some(node_id) => node_id,
            None => self.insert_carbon(end.point),
        };
        if begin_id == end_id || self.bond_exists(&begin_id, &end_id) {
            self.undo_stack.pop();
            return false;
        }
        let bond_id = self.next_id("b");
        let mut entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        entry.fragment.bonds.push(Bond {
            id: bond_id,
            begin: begin_id,
            end: end_id.clone(),
            order: 1,
            double: None,
            stroke_width: self.options.bond_stroke_width,
        });
        entry.update_bounds();

        let endpoint = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == end_id)
            .map(|node| EndpointHit {
                node_id: node.id.clone(),
                point: entry.world_point_for_node(node),
                distance: 0.0,
            });
        self.state.overlay.hover_endpoint = endpoint;
        true
    }

    pub fn cycle_bond_center_style(&mut self, bond_id: &str) -> bool {
        let Some(default_placement) = self.default_double_bond_placement(bond_id) else {
            return false;
        };
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.id == bond_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        if bond.order != 1 && bond.order != 2 {
            self.undo_stack.pop();
            return false;
        }
        let opposite_placement = opposite_double_bond_placement(default_placement);
        let next_placement = if bond.order == 1 {
            default_placement
        } else {
            match bond.double.as_ref().map(|double| double.placement) {
                Some(current) if current == default_placement => DoubleBondPlacement::Center,
                Some(DoubleBondPlacement::Center) => opposite_placement,
                Some(current) if current == opposite_placement => default_placement,
                _ => default_placement,
            }
        };
        bond.order = 2;
        bond.double = Some(DoubleBond {
            placement: next_placement,
        });
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    pub fn delete_selection(&mut self) -> bool {
        if self.state.selection.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let selection = self.state.selection.clone();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };

        let selected_nodes: BTreeSet<String> = selection.nodes.into_iter().collect();
        let selected_bonds: BTreeSet<String> = selection.bonds.into_iter().collect();
        entry.fragment.bonds.retain(|bond| {
            !selected_bonds.contains(&bond.id)
                && !selected_nodes.contains(&bond.begin)
                && !selected_nodes.contains(&bond.end)
        });

        let connected_nodes: BTreeSet<String> = entry
            .fragment
            .bonds
            .iter()
            .flat_map(|bond| [bond.begin.clone(), bond.end.clone()])
            .collect();
        entry.fragment.nodes.retain(|node| {
            !selected_nodes.contains(&node.id) && connected_nodes.contains(&node.id)
        });
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.state.document.clone());
        self.restore_document(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.state.document.clone());
        self.restore_document(next);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn drag_target_endpoint(&self, anchor: &BondAnchor, point: Point) -> Option<EndpointHit> {
        hit_test_endpoint_excluding(
            &self.state.document,
            point,
            ENDPOINT_HIT_RADIUS,
            anchor.node_id.as_deref(),
        )
    }

    fn endpoint_anchor_near(&self, anchor: &BondAnchor, point: Point) -> Option<BondAnchor> {
        let target = self.drag_target_endpoint(anchor, point)?;
        Some(BondAnchor {
            node_id: Some(target.node_id),
            point: target.point,
        })
    }

    fn bond_exists(&self, begin_id: &str, end_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        entry.fragment.bonds.iter().any(|bond| {
            (bond.begin == begin_id && bond.end == end_id)
                || (bond.begin == end_id && bond.end == begin_id)
        })
    }

    fn insert_carbon(&mut self, point: Point) -> String {
        let node_id = self.next_id("n");
        let entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        let local = entry.local_point(point);
        entry
            .fragment
            .nodes
            .push(crate::Node::carbon(node_id.clone(), local));
        node_id
    }

    fn next_id(&mut self, prefix: &str) -> String {
        let value = self.next_id;
        self.next_id += 1;
        format!("{prefix}_{value}")
    }

    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(self.state.document.clone());
        self.redo_stack.clear();
    }

    fn restore_document(&mut self, document: ChemcoreDocument) {
        self.state.document = document;
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.next_id = self.infer_next_id();
    }

    fn infer_next_id(&self) -> u64 {
        let mut max_id = 0;
        if let Some(entry) = self.state.document.editable_fragment() {
            for id in entry
                .fragment
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .chain(entry.fragment.bonds.iter().map(|bond| bond.id.as_str()))
            {
                if let Some((_, suffix)) = id.rsplit_once('_') {
                    if let Ok(value) = suffix.parse::<u64>() {
                        max_id = max_id.max(value);
                    }
                }
            }
        }
        max_id + 1
    }

    fn default_double_bond_placement(&self, bond_id: &str) -> Option<DoubleBondPlacement> {
        let entry = self.state.document.editable_fragment()?;
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id && (bond.order == 1 || bond.order == 2))?;
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)?;
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)?;
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        let dx = end_point.x - begin_point.x;
        let dy = end_point.y - begin_point.y;
        let length = dx.hypot(dy);
        if length <= crate::EPSILON {
            return Some(DoubleBondPlacement::Left);
        }
        let normal_x = -dy / length;
        let normal_y = dx / length;
        let mut score = 0.0;
        for other in &entry.fragment.bonds {
            if other.id == bond.id {
                continue;
            }
            if other.begin == bond.begin || other.end == bond.begin {
                let other_id = if other.begin == bond.begin {
                    &other.end
                } else {
                    &other.begin
                };
                if let Some(neighbor) = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| &node.id == other_id)
                {
                    let point = entry.world_point_for_node(neighbor);
                    score +=
                        (point.x - begin_point.x) * normal_x + (point.y - begin_point.y) * normal_y;
                }
            } else if other.begin == bond.end || other.end == bond.end {
                let other_id = if other.begin == bond.end {
                    &other.end
                } else {
                    &other.begin
                };
                if let Some(neighbor) = entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| &node.id == other_id)
                {
                    let point = entry.world_point_for_node(neighbor);
                    score +=
                        (point.x - end_point.x) * normal_x + (point.y - end_point.y) * normal_y;
                }
            }
        }
        if score >= 0.0 {
            Some(DoubleBondPlacement::Left)
        } else {
            Some(DoubleBondPlacement::Right)
        }
    }

    fn selection_render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = Vec::new();
        let Some(entry) = self.state.document.editable_fragment() else {
            return out;
        };
        let selected_bonds: BTreeSet<&str> = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect();
        let selected_nodes: BTreeSet<&str> = self
            .state
            .selection
            .nodes
            .iter()
            .map(String::as_str)
            .collect();

        for bond in &entry.fragment.bonds {
            if !selected_bonds.contains(bond.id.as_str()) {
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
            out.push(RenderPrimitive::Line {
                role: RenderRole::SelectionBond,
                from: entry.world_point_for_node(begin),
                to: entry.world_point_for_node(end),
                stroke: "rgba(47,111,237,0.72)".to_string(),
                stroke_width: self.options.bond_stroke_width + 5.0,
            });
        }

        for node in &entry.fragment.nodes {
            if !selected_nodes.contains(node.id.as_str()) {
                continue;
            }
            out.push(RenderPrimitive::Circle {
                role: RenderRole::SelectionNode,
                center: entry.world_point_for_node(node),
                radius: ENDPOINT_FOCUS_RADIUS,
                fill: "rgba(47,111,237,0.16)".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: 1.6,
            });
        }
        out
    }
}

fn opposite_double_bond_placement(placement: DoubleBondPlacement) -> DoubleBondPlacement {
    match placement {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => DoubleBondPlacement::Right,
    }
}

fn centered_oriented_rect_points(
    start: Point,
    end: Point,
    length_along_bond: f64,
    width_across_bond: f64,
) -> Vec<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let bond_length = dx.hypot(dy);
    let center = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    if bond_length <= crate::EPSILON {
        let half = width_across_bond / 2.0;
        return vec![
            Point::new(center.x - half, center.y - half),
            Point::new(center.x + half, center.y - half),
            Point::new(center.x + half, center.y + half),
            Point::new(center.x - half, center.y + half),
        ];
    }
    let ux = dx / bond_length;
    let uy = dy / bond_length;
    let tx = ux * length_along_bond / 2.0;
    let ty = uy * length_along_bond / 2.0;
    let nx = -uy * width_across_bond / 2.0;
    let ny = ux * width_across_bond / 2.0;
    vec![
        Point::new(center.x - tx + nx, center.y - ty + ny),
        Point::new(center.x + tx + nx, center.y + ty + ny),
        Point::new(center.x + tx - nx, center.y + ty - ny),
        Point::new(center.x - tx - nx, center.y - ty - ny),
    ]
}

impl Engine {
    pub fn options(&self) -> &EditorOptions {
        &self.options
    }

    pub fn set_bond_length(&mut self, length: f64) {
        self.options.bond_length = if length > 0.0 {
            length
        } else {
            DEFAULT_BOND_LENGTH
        };
    }
}
