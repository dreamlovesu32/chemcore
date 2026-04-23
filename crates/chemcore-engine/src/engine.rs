use crate::{
    anchor_from_point, can_draw_single_bond, can_focus_endpoint, default_angle_for_anchor,
    endpoint_from_angle, hit_test_endpoint, render_document, snapped_angle_for_anchor, Bond,
    BondAnchor, BondPreview, ChemcoreDocument, DragState, EditorOptions, EndpointHit, OverlayState,
    Point, PointerEvent, RenderPrimitive, ToolState, DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD,
    ENDPOINT_HIT_RADIUS,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub document: ChemcoreDocument,
    pub tool: ToolState,
    pub overlay: OverlayState,
}

pub struct Engine {
    state: EngineState,
    drag: Option<DragState>,
    options: EditorOptions,
    next_id: u64,
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
                overlay: OverlayState::default(),
            },
            drag: None,
            options: EditorOptions::default(),
            next_id: 1,
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
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            out.push(RenderPrimitive::Circle {
                center: hover.point,
                radius: ENDPOINT_HIT_RADIUS,
                fill: "rgba(47,111,237,0.24)".to_string(),
                stroke: "rgba(47,111,237,0.78)".to_string(),
                stroke_width: 1.4,
            });
        }
        if let Some(preview) = &self.state.overlay.preview {
            out.push(RenderPrimitive::Line {
                from: preview.start,
                to: preview.end,
                stroke: "rgba(0,0,0,0.72)".to_string(),
                stroke_width: self.options.bond_stroke_width,
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
        if !can_focus_endpoint(&self.state.tool) {
            self.clear_interaction();
            return;
        }

        if let Some(mut drag) = self.drag.take() {
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                let angle = snapped_angle_for_anchor(&self.state.document, &drag.anchor, point);
                let end = endpoint_from_angle(&drag.anchor, angle, self.options.bond_length);
                drag.preview_end = Some(end);
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.anchor.point,
                    end,
                });
            }
            self.drag = Some(drag);
            return;
        }

        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
    }

    pub fn pointer_down(&mut self, event: PointerEvent) {
        if !can_draw_single_bond(&self.state.tool) {
            return;
        }
        let point = event.point();
        let Some(anchor) = anchor_from_point(&self.state.document, point) else {
            return;
        };
        self.drag = Some(DragState {
            anchor,
            start: point,
            has_dragged: false,
            preview_end: None,
        });
    }

    pub fn pointer_up(&mut self, event: PointerEvent) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let end = if drag.has_dragged {
            drag.preview_end.unwrap_or_else(|| {
                let angle =
                    snapped_angle_for_anchor(&self.state.document, &drag.anchor, event.point());
                endpoint_from_angle(&drag.anchor, angle, self.options.bond_length)
            })
        } else {
            let angle = default_angle_for_anchor(&self.state.document, &drag.anchor);
            endpoint_from_angle(&drag.anchor, angle, self.options.bond_length)
        };
        self.state.overlay.preview = None;
        self.add_single_bond(drag.anchor, end);
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.state.overlay = OverlayState::default();
    }

    pub fn add_single_bond(&mut self, anchor: BondAnchor, end: Point) {
        let begin_id = match anchor.node_id {
            Some(node_id) => node_id,
            None => self.insert_carbon(anchor.point),
        };
        let end_id = self.insert_carbon(end);
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
