use crate::{
    anchor_from_point, bond_center_focus_length, build_label_glyph_polygons, can_draw_bond,
    can_focus_bond_center, can_focus_endpoint, decide_label_layout,
    default_angle_for_anchor_for_variant, endpoint_from_angle_for_document, hit_test_bond_center,
    hit_test_endpoint, hit_test_endpoint_excluding, layout_label_text, render_document, round2,
    select_at, snapped_angle_for_anchor, Bond, BondAnchor, BondLinePattern, BondLineStyles,
    BondLineWeight, BondLineWeights, BondPreview, BondStereo, BondVariant, ChemcoreDocument,
    DoubleBond, DoubleBondPlacement, DragState, EditorOptions, EndpointHit, HoverTextBox,
    LabelFlow, LabelRun, OverlayState, Point, PointerEvent, RenderPrimitive, RenderRole,
    SelectionState, Tool, ToolState, BOND_CENTER_FOCUS_WIDTH, BOND_CENTER_HIT_RADIUS,
    DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS, ENDPOINT_HIT_RADIUS,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const DEFAULT_TEXT_FONT_FAMILY: &str = "Arial";
const DEFAULT_TEXT_FONT_SIZE: f64 = 12.0;
const DEFAULT_TEXT_FILL: &str = "#000000";
const DEFAULT_TEXT_LINE_HEIGHT: f64 = 12.6;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub document: ChemcoreDocument,
    pub tool: ToolState,
    pub selection: SelectionState,
    pub overlay: OverlayState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TextEditTarget {
    TextObject {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        x: f64,
        y: f64,
    },
    EndpointLabel {
        node_id: String,
        x: f64,
        y: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditSession {
    pub target: TextEditTarget,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "box")]
    pub box_value: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_offset: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured_size: Option<[f64; 2]>,
    #[serde(default)]
    pub preserve_lines: bool,
    #[serde(default)]
    pub default_chemical: bool,
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

    pub fn load_document_json(&mut self, json: &str) -> Result<(), String> {
        let document: ChemcoreDocument =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        self.state.document = document;
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.next_id = self.infer_next_id();
        Ok(())
    }

    pub fn render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = if let Some(preview_document) = self.preview_document() {
            render_document(&preview_document)
        } else {
            render_document(&self.state.document)
        };
        out.extend(self.selection_render_list());
        if let Some(hover) = &self.state.overlay.hover_text_box {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::HoverTextBox,
                object_id: hover.object_id.clone(),
                x: hover.bounds[0],
                y: hover.bounds[1],
                width: (hover.bounds[2] - hover.bounds[0]).max(0.0),
                height: (hover.bounds[3] - hover.bounds[1]).max(0.0),
                fill: Some("rgba(47,111,237,0.12)".to_string()),
                stroke: Some("rgba(47,111,237,0.76)".to_string()),
                stroke_width: 1.1,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            if let Some(label_anchor) = &hover.label_anchor {
                out.push(RenderPrimitive::Rect {
                    role: RenderRole::HoverLabelGlyph,
                    object_id: None,
                    x: label_anchor.glyph_box[0],
                    y: label_anchor.glyph_box[1],
                    width: (label_anchor.glyph_box[2] - label_anchor.glyph_box[0]).max(0.0),
                    height: (label_anchor.glyph_box[3] - label_anchor.glyph_box[1]).max(0.0),
                    fill: Some("rgba(47,111,237,0.12)".to_string()),
                    stroke: Some("rgba(47,111,237,0.82)".to_string()),
                    stroke_width: 1.1,
                    rx: None,
                    ry: None,
                    dash_array: Vec::new(),
                    fill_gradient: None,
                });
            } else {
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::HoverEndpoint,
                    object_id: None,
                    center: hover.point,
                    radius: ENDPOINT_FOCUS_RADIUS,
                    fill: "rgba(47,111,237,0.24)".to_string(),
                    stroke: "rgba(47,111,237,0.78)".to_string(),
                    stroke_width: 1.4,
                });
            }
        }
        if let Some(hover) = &self.state.overlay.hover_bond_center {
            let focus_length = bond_center_focus_length(hover.begin, hover.end);
            if focus_length > crate::EPSILON {
                out.push(RenderPrimitive::Polygon {
                    role: RenderRole::HoverBondCenter,
                    object_id: None,
                    bond_id: None,
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
            out.push(RenderPrimitive::Circle {
                role: RenderRole::PreviewEnd,
                object_id: None,
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

    pub fn begin_text_edit(&mut self, point: Point) -> Option<TextEditSession> {
        self.clear_interaction();
        if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
            self.state.overlay.hover_text_box = Some(HoverTextBox {
                bounds,
                object_id: None,
                node_id: Some(node_id.clone()),
            });
            return self.endpoint_text_session(&node_id, point);
        }
        if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
            self.state.overlay.hover_text_box = Some(HoverTextBox {
                bounds,
                object_id: Some(object_id.clone()),
                node_id: None,
            });
            return self.text_object_session(&object_id);
        }
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint.clone());
            return self.endpoint_text_session(&endpoint.node_id, endpoint.point);
        }
        Some(TextEditSession {
            target: TextEditTarget::TextObject {
                object_id: None,
                x: point.x,
                y: point.y,
            },
            text: String::new(),
            source_runs: Vec::new(),
            font_family: Some(DEFAULT_TEXT_FONT_FAMILY.to_string()),
            font_size: Some(DEFAULT_TEXT_FONT_SIZE),
            fill: Some(DEFAULT_TEXT_FILL.to_string()),
            align: Some("left".to_string()),
            line_height: Some(DEFAULT_TEXT_LINE_HEIGHT),
            box_value: Some([0.0, 0.0, 8.0, DEFAULT_TEXT_LINE_HEIGHT]),
            anchor_offset: None,
            measured_size: None,
            preserve_lines: true,
            default_chemical: false,
        })
    }

    pub fn apply_text_edit(&mut self, session: TextEditSession) -> bool {
        match &session.target {
            TextEditTarget::TextObject { object_id, .. } => {
                self.apply_text_object_edit(object_id.as_deref(), &session)
            }
            TextEditTarget::EndpointLabel { node_id, .. } => {
                self.apply_endpoint_text_edit(node_id, &session)
            }
        }
    }

    pub fn preview_text_runs(&self, session: &TextEditSession) -> (Vec<LabelRun>, Vec<LabelRun>) {
        let text = if !session.source_runs.is_empty() {
            runs_text(&session.source_runs)
        } else {
            session.text.clone()
        };
        let fallback_font_family = session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY);
        let fallback_font_size = session.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE);
        let fallback_fill = session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL);
        let source_runs = normalize_source_runs(session, &text);
        let display_runs = display_runs_from_source_runs(
            &source_runs,
            fallback_font_family,
            fallback_font_size,
            fallback_fill,
        );
        (source_runs, display_runs)
    }

    pub fn pointer_move(&mut self, event: PointerEvent) {
        let point = event.point();
        if self.state.tool.active_tool == Tool::Select {
            self.clear_interaction();
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.preview = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.hover_endpoint = None;
            if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: None,
                    node_id: Some(node_id),
                });
                return;
            }
            if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: Some(object_id),
                    node_id: None,
                });
                return;
            }
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
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
                        label_anchor: target.label_anchor.clone(),
                    });
                    target.point
                } else if drag.free_length {
                    drag.target = None;
                    point
                } else {
                    drag.target = None;
                    let angle = snapped_angle_for_anchor(&self.state.document, &drag.anchor, point);
                    endpoint_from_angle_for_document(
                        &self.state.document,
                        &drag.anchor,
                        angle,
                        self.options.bond_length,
                    )
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
        self.state.overlay.hover_text_box = None;
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint);
            return;
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
        if self.state.tool.active_tool == Tool::Text {
            self.state.selection = SelectionState::default();
            self.clear_interaction();
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, event.point(), ENDPOINT_HIT_RADIUS);
            return;
        }
        if !can_draw_bond(&self.state.tool) {
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
                    label_anchor: endpoint.label_anchor,
                },
                start: point,
                has_dragged: false,
                free_length: event.alt_key,
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
            free_length: false,
            preview_end: None,
            target: None,
        });
    }

    pub fn pointer_up(&mut self, event: PointerEvent) {
        if self.state.tool.active_tool == Tool::Text {
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, event.point(), ENDPOINT_HIT_RADIUS);
            return;
        }
        let Some(drag) = self.drag.take() else {
            return;
        };
        let end_anchor = if drag.has_dragged {
            if let Some(target) = drag.target {
                target
            } else if drag.free_length {
                BondAnchor {
                    node_id: None,
                    point: drag.preview_end.unwrap_or_else(|| event.point()),
                    label_anchor: None,
                }
            } else {
                let end = drag.preview_end.unwrap_or_else(|| {
                    let angle =
                        snapped_angle_for_anchor(&self.state.document, &drag.anchor, event.point());
                    endpoint_from_angle_for_document(
                        &self.state.document,
                        &drag.anchor,
                        angle,
                        self.options.bond_length,
                    )
                });
                BondAnchor {
                    node_id: None,
                    point: end,
                    label_anchor: None,
                }
            }
        } else {
            let angle = default_angle_for_anchor_for_variant(
                &self.state.document,
                &drag.anchor,
                self.state.tool.bond_variant,
            );
            let end = endpoint_from_angle_for_document(
                &self.state.document,
                &drag.anchor,
                angle,
                self.options.bond_length,
            );
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    point: end,
                    label_anchor: None,
                })
        };
        self.state.overlay.preview = None;
        self.add_bond_between(drag.anchor, end_anchor, self.pending_bond_order());
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.state.overlay = OverlayState::default();
    }

    pub fn add_single_bond(&mut self, anchor: BondAnchor, end: Point) {
        self.add_bond_between(
            anchor,
            BondAnchor {
                node_id: None,
                point: end,
                label_anchor: None,
            },
            1,
        );
    }

    pub fn add_single_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor) -> bool {
        self.add_bond_between(anchor, end, 1)
    }

    pub fn add_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor, order: u8) -> bool {
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
        let pending_double =
            self.pending_double_state_for_new_bond(&begin_id, &end_id, order.max(1));
        let pending_line_styles = self.pending_line_styles();
        let pending_line_weights = self.pending_line_weights();
        let pending_stereo = self.pending_bond_stereo();
        let mut entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        entry.fragment.bonds.push(Bond {
            id: bond_id.clone(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: pending_double,
            stereo: pending_stereo,
            stroke_width: self.options.bond_stroke_width,
            line_styles: pending_line_styles,
            line_weights: pending_line_weights,
            meta: serde_json::Value::Null,
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            &bond_id,
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            &bond_id,
        );
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_width,
        );
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
                label_anchor: None,
            });
        self.state.overlay.hover_endpoint = endpoint;
        true
    }

    fn preview_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let end_anchor = if let Some(target) = drag.target.clone() {
            target
        } else {
            BondAnchor {
                node_id: None,
                point: drag.preview_end?,
                label_anchor: None,
            }
        };
        self.document_with_preview_bond(&drag.anchor, &end_anchor, self.pending_bond_order())
    }

    fn document_with_preview_bond(
        &self,
        anchor: &BondAnchor,
        end: &BondAnchor,
        order: u8,
    ) -> Option<ChemcoreDocument> {
        let mut document = self.state.document.clone();
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists_in_document(&document, begin_id, end_id) {
                return None;
            }
        }
        let mut entry = document.editable_fragment_mut()?;
        let begin_id = match &anchor.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(anchor.point);
                let node_id = "__preview_node_begin".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        let end_id = match &end.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(end.point);
                let node_id = "__preview_node_end".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        if begin_id == end_id || self.bond_exists_in_fragment(entry.fragment, &begin_id, &end_id) {
            return None;
        }
        entry.fragment.bonds.push(Bond {
            id: "__preview_bond".to_string(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: self.pending_double_state_for_new_bond(&begin_id, &end_id, order.max(1)),
            stereo: self.pending_bond_stereo(),
            stroke_width: self.options.bond_stroke_width,
            line_styles: self.pending_line_styles(),
            line_weights: self.pending_line_weights(),
            meta: serde_json::Value::Null,
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            "__preview_bond",
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            "__preview_bond",
        );
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_width,
        );
        entry.update_bounds();
        Some(document)
    }

    pub fn cycle_bond_center_style(&mut self, bond_id: &str) -> bool {
        let (current_order, was_double_before) = self
            .state
            .document
            .editable_fragment()
            .and_then(|entry| entry.fragment.bonds.iter().find(|bond| bond.id == bond_id))
            .map(|bond| (bond.order, bond.order == 2 && bond.double.is_some()))
            .unwrap_or((1, false));
        let default_side = self
            .preferred_double_bond_side(bond_id)
            .unwrap_or(DoubleBondPlacement::Right);
        let default_placement =
            if current_order == 1 && self.should_default_center_double_bond(bond_id) {
                DoubleBondPlacement::Center
            } else {
                default_side
            };
        let should_freeze_after_change = was_double_before;
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
        let changed = match self.state.tool.bond_variant {
            BondVariant::Single => apply_single_tool_center_style(bond, default_placement),
            BondVariant::Double => apply_double_tool_center_style(bond, default_placement),
            BondVariant::Triple => replace_with_plain_triple_bond_style(bond),
            BondVariant::Dashed => cycle_dashed_bond_center_style(bond, default_placement),
            BondVariant::DashedDouble => {
                cycle_dashed_double_bond_tool_center_style(bond, default_placement)
            }
            BondVariant::Bold => cycle_bold_bond_center_style(bond, default_placement),
            BondVariant::BoldDashed => replace_with_bold_dashed_bond_style(bond),
            BondVariant::Wedge | BondVariant::HashedWedge => {
                replace_with_stereo_bond_style(bond, self.state.tool.bond_variant)
            }
        };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        if let Some(double) = bond.double.as_mut() {
            double.frozen = should_freeze_after_change;
        }
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_width,
        );
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
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_width,
        );
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    pub fn replace_hovered_endpoint_label(&mut self, label: &str) -> bool {
        let Some(hovered_node_id) = self
            .state
            .overlay
            .hover_endpoint
            .as_ref()
            .map(|hit| hit.node_id.clone())
        else {
            return false;
        };

        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(node_index) = entry
            .fragment
            .nodes
            .iter()
            .position(|node| node.id == hovered_node_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, &hovered_node_id);
        let node = &mut entry.fragment.nodes[node_index];

        if !apply_node_label_replacement(node, label, &connection_angles) {
            self.undo_stack.pop();
            return false;
        }

        let node_position = node.position;
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            &hovered_node_id,
            self.options.bond_stroke_width,
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: hovered_node_id,
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
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

    fn endpoint_text_session(&self, node_id: &str, _point: Point) -> Option<TextEditSession> {
        let entry = self.state.document.editable_fragment()?;
        let node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == node_id)?;
        let label = node.label.as_ref();
        let box_value = label.and_then(|label| label.bbox()).map(|bbox| {
            [
                round2(bbox[0] + entry.object.transform.translate[0]),
                round2(bbox[1] + entry.object.transform.translate[1]),
                round2(bbox[2] + entry.object.transform.translate[0]),
                round2(bbox[3] + entry.object.transform.translate[1]),
            ]
        });
        let anchor_point = endpoint_label_editor_anchor_world(node, entry.object.transform.translate)
            .unwrap_or_else(|| {
                attached_node_label_anchor_world(
                    entry.fragment,
                    node_id,
                    entry.object.transform.translate,
                    self.options.bond_stroke_width,
                )
            });
        let source_runs = label
            .and_then(|label| label.meta.get("sourceRuns"))
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_else(|| label.map(|label| label.runs.clone()).unwrap_or_default());
        let text = if !source_runs.is_empty() {
            runs_text(&source_runs)
        } else {
            label.map(|label| label.text.clone()).unwrap_or_default()
        };
        let font_size = label
            .and_then(|label| label.font_size)
            .or(Some(DEFAULT_TEXT_FONT_SIZE));
        let line_height = Some(
            (font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE) * 1.05)
                .max(font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE)),
        );
        Some(TextEditSession {
            target: TextEditTarget::EndpointLabel {
                node_id: node_id.to_string(),
                x: anchor_point.x,
                y: anchor_point.y,
            },
            text,
            source_runs,
            font_family: label
                .and_then(|label| label.font_family.clone())
                .or(Some(DEFAULT_TEXT_FONT_FAMILY.to_string())),
            font_size,
            fill: label
                .and_then(|label| label.fill.clone())
                .or(Some(DEFAULT_TEXT_FILL.to_string())),
            align: Some("left".to_string()),
            line_height,
            box_value,
            anchor_offset: box_value.map(|bbox| {
                [
                    round2(anchor_point.x - bbox[0]),
                    round2(anchor_point.y - bbox[1]),
                ]
            }),
            measured_size: None,
            preserve_lines: true,
            default_chemical: true,
        })
    }

    fn text_object_session(&self, object_id: &str) -> Option<TextEditSession> {
        let object = self
            .state
            .document
            .objects
            .iter()
            .find(|object| object.id == object_id && object.object_type == "text")?;
        let payload = &object.payload;
        let source_runs = payload
            .extra
            .get("sourceRuns")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_else(|| payload_runs_or_text(payload));
        let text = if !source_runs.is_empty() {
            runs_text(&source_runs)
        } else {
            payload_text(payload)
        };
        Some(TextEditSession {
            target: TextEditTarget::TextObject {
                object_id: Some(object_id.to_string()),
                x: object.transform.translate[0],
                y: object.transform.translate[1],
            },
            text,
            source_runs,
            font_family: payload
                .extra
                .get("fontFamily")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or(Some(DEFAULT_TEXT_FONT_FAMILY.to_string())),
            font_size: payload
                .extra
                .get("fontSize")
                .and_then(Value::as_f64)
                .or(Some(12.0)),
            fill: payload
                .extra
                .get("fill")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or(Some(DEFAULT_TEXT_FILL.to_string())),
            align: payload
                .extra
                .get("align")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or(Some("left".to_string())),
            line_height: payload
                .extra
                .get("lineHeight")
                .and_then(Value::as_f64)
                .or(Some(DEFAULT_TEXT_LINE_HEIGHT)),
            box_value: payload_box(payload),
            anchor_offset: None,
            measured_size: None,
            preserve_lines: payload
                .extra
                .get("preserveLines")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            default_chemical: false,
        })
    }

    fn apply_text_object_edit(
        &mut self,
        object_id: Option<&str>,
        session: &TextEditSession,
    ) -> bool {
        let text = session.text.replace("\r\n", "\n").replace('\r', "\n");
        if text.trim().is_empty() {
            return self.remove_text_object(object_id);
        }
        let source_runs = normalize_source_runs(session, &text);
        let display_runs = display_runs_from_source_runs(
            &source_runs,
            session.font_family.as_deref().unwrap_or("Arial"),
            session.font_size.unwrap_or(12.0),
            session.fill.as_deref().unwrap_or("#000000"),
        );
        let (width, height) = estimate_text_block_size(
            &display_runs,
            session.font_size.unwrap_or(12.0),
            session.line_height.unwrap_or(15.0),
        );
        let (x, y, existing_object_id) = match &session.target {
            TextEditTarget::TextObject { object_id, x, y } => (*x, *y, object_id.clone()),
            _ => return false,
        };
        let target_object_id = existing_object_id.or_else(|| object_id.map(ToString::to_string));

        self.push_undo_snapshot();
        let changed =
            if let Some(target_object_id) = target_object_id {
                let Some(object) =
                    self.state.document.objects.iter_mut().find(|object| {
                        object.id == target_object_id && object.object_type == "text"
                    })
                else {
                    self.undo_stack.pop();
                    return false;
                };
                update_text_object_fields(
                    object,
                    x,
                    y,
                    &text,
                    source_runs,
                    display_runs,
                    session,
                    width,
                    height,
                )
            } else {
                let next_id = self.next_id("obj_text");
                let object = make_text_object(
                    &next_id,
                    x,
                    y,
                    &text,
                    source_runs,
                    display_runs,
                    session,
                    width,
                    height,
                    self.next_text_z_index(),
                );
                self.state.document.objects.push(object);
                true
            };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    fn apply_endpoint_text_edit(&mut self, node_id: &str, session: &TextEditSession) -> bool {
        let text = session
            .text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', " ");
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(node_index) = entry
            .fragment
            .nodes
            .iter()
            .position(|node| node.id == node_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let local_anchor_position = match &session.target {
            TextEditTarget::EndpointLabel { x, y, .. } => [
                round2(*x - object_translate[0]),
                round2(*y - object_translate[1]),
            ],
            _ => entry.fragment.nodes[node_index].position,
        };
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, node_id);
        let node = &mut entry.fragment.nodes[node_index];
        let changed = apply_node_label_text_edit(
            node,
            &text,
            session,
            &connection_angles,
            local_anchor_position,
        );
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        let node_position = node.position;
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            node_id,
            self.options.bond_stroke_width,
        );
        entry.update_bounds();
        let hover_point = crate::Point::new(
            object_translate[0] + node_position[0],
            object_translate[1] + node_position[1],
        );
        self.drag = None;
        self.state.selection = SelectionState::default();
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(EndpointHit {
            node_id: node_id.to_string(),
            point: hover_point,
            distance: 0.0,
            label_anchor: None,
        });
        true
    }

    fn remove_text_object(&mut self, object_id: Option<&str>) -> bool {
        let Some(object_id) = object_id else {
            return false;
        };
        let Some(index) = self
            .state
            .document
            .objects
            .iter()
            .position(|object| object.id == object_id && object.object_type == "text")
        else {
            return false;
        };
        self.push_undo_snapshot();
        self.state.document.objects.remove(index);
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        true
    }

    fn hit_test_text_object(&self, point: Point) -> Option<(String, [f64; 4])> {
        let mut best: Option<(i32, usize, String)> = None;
        let mut best_bounds: Option<[f64; 4]> = None;
        for (index, object) in self.state.document.objects.iter().enumerate() {
            if object.object_type != "text" || !object.visible {
                continue;
            }
            let Some(bounds) = text_object_world_bounds(object) else {
                continue;
            };
            if point.x < bounds[0]
                || point.x > bounds[2]
                || point.y < bounds[1]
                || point.y > bounds[3]
            {
                continue;
            }
            let candidate = (object.z_index, index, object.id.clone());
            if best.as_ref().map_or(true, |current| {
                candidate.0 > current.0 || (candidate.0 == current.0 && candidate.1 > current.1)
            }) {
                best = Some(candidate);
                best_bounds = Some(bounds);
            }
        }
        best.and_then(|(_, _, object_id)| best_bounds.map(|bounds| (object_id, bounds)))
    }

    fn hit_test_endpoint_label_box(&self, point: Point) -> Option<(String, [f64; 4])> {
        let entry = self.state.document.editable_fragment()?;
        let mut best: Option<(f64, String, [f64; 4])> = None;
        for node in &entry.fragment.nodes {
            let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
            else {
                continue;
            };
            if point.x < bounds[0]
                || point.x > bounds[2]
                || point.y < bounds[1]
                || point.y > bounds[3]
            {
                continue;
            }
            let area = (bounds[2] - bounds[0]).abs() * (bounds[3] - bounds[1]).abs();
            if best.as_ref().map_or(true, |current| area < current.0) {
                best = Some((area, node.id.clone(), bounds));
            }
        }
        best.map(|(_, node_id, bounds)| (node_id, bounds))
    }

    fn next_text_z_index(&self) -> i32 {
        self.state
            .document
            .objects
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(10)
            + 10
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
            label_anchor: target.label_anchor,
        })
    }

    fn bond_exists(&self, begin_id: &str, end_id: &str) -> bool {
        self.bond_exists_in_document(&self.state.document, begin_id, end_id)
    }

    fn bond_exists_in_document(
        &self,
        document: &ChemcoreDocument,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        let Some(entry) = document.editable_fragment() else {
            return false;
        };
        self.bond_exists_in_fragment(entry.fragment, begin_id, end_id)
    }

    fn bond_exists_in_fragment(
        &self,
        fragment: &crate::MoleculeFragment,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        fragment.bonds.iter().any(|bond| {
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
        for id in self
            .state
            .document
            .objects
            .iter()
            .map(|object| object.id.as_str())
        {
            if let Some((_, suffix)) = id.rsplit_once('_') {
                if let Ok(value) = suffix.parse::<u64>() {
                    max_id = max_id.max(value);
                }
            }
        }
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

    fn preferred_double_bond_side(&self, bond_id: &str) -> Option<DoubleBondPlacement> {
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
        if score <= 0.0 {
            Some(DoubleBondPlacement::Left)
        } else {
            Some(DoubleBondPlacement::Right)
        }
    }

    fn should_default_center_double_bond(&self, bond_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id && (bond.order == 1 || bond.order == 2))
        else {
            return false;
        };
        let begin_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|other| {
                other.id != bond.id && (other.begin == bond.begin || other.end == bond.begin)
            })
            .count();
        let end_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|other| {
                other.id != bond.id && (other.begin == bond.end || other.end == bond.end)
            })
            .count();
        begin_connections + end_connections >= 2
    }

    fn pending_bond_order(&self) -> u8 {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble => 2,
            BondVariant::Triple => 3,
            _ => 1,
        }
    }

    fn pending_double_state_for_new_bond(
        &self,
        begin_id: &str,
        end_id: &str,
        order: u8,
    ) -> Option<DoubleBond> {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble if order >= 2 => {
                let placement = if self.should_default_center_for_new_bond(begin_id, end_id) {
                    DoubleBondPlacement::Center
                } else {
                    DoubleBondPlacement::Right
                };
                Some(DoubleBond {
                    placement,
                    center_exit_side: None,
                    frozen: false,
                })
            }
            _ => None,
        }
    }

    fn should_default_center_for_new_bond(&self, begin_id: &str, end_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        let begin_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == begin_id || bond.end == begin_id)
            .count();
        let end_connections = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == end_id || bond.end == end_id)
            .count();
        begin_connections + end_connections >= 2
    }

    fn pending_line_styles(&self) -> BondLineStyles {
        match self.state.tool.bond_variant {
            BondVariant::Dashed | BondVariant::BoldDashed => {
                return BondLineStyles {
                    main: BondLinePattern::Dashed,
                    ..BondLineStyles::default()
                };
            }
            BondVariant::DashedDouble => {
                return BondLineStyles {
                    right: BondLinePattern::Dashed,
                    ..BondLineStyles::default()
                };
            }
            _ => {}
        }
        BondLineStyles::default()
    }

    fn pending_bond_stereo(&self) -> Option<BondStereo> {
        match self.state.tool.bond_variant {
            BondVariant::Wedge => Some(BondStereo {
                kind: "solid-wedge".to_string(),
                wide_end: "end".to_string(),
            }),
            BondVariant::HashedWedge => Some(BondStereo {
                kind: "hashed-wedge".to_string(),
                wide_end: "end".to_string(),
            }),
            _ => None,
        }
    }

    fn pending_line_weights(&self) -> BondLineWeights {
        match self.state.tool.bond_variant {
            BondVariant::Bold | BondVariant::BoldDashed => {
                return BondLineWeights {
                    main: BondLineWeight::Bold,
                    ..BondLineWeights::default()
                };
            }
            _ => {}
        }
        BondLineWeights::default()
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
                object_id: None,
                bond_id: None,
                from: entry.world_point_for_node(begin),
                to: entry.world_point_for_node(end),
                stroke: "rgba(47,111,237,0.72)".to_string(),
                stroke_width: self.options.bond_stroke_width + 5.0,
                dash_array: Vec::new(),
            });
        }

        for node in &entry.fragment.nodes {
            if !selected_nodes.contains(node.id.as_str()) {
                continue;
            }
            out.push(RenderPrimitive::Circle {
                role: RenderRole::SelectionNode,
                object_id: None,
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

fn apply_node_label_replacement(
    node: &mut crate::Node,
    label: &str,
    connection_angles: &[f64],
) -> bool {
    if classify_node_label_replacement(label).is_none() {
        return false;
    }
    let session = TextEditSession {
        target: TextEditTarget::EndpointLabel {
            node_id: node.id.clone(),
            x: node.position[0],
            y: node.position[1],
        },
        text: label.to_string(),
        source_runs: Vec::new(),
        font_family: Some(DEFAULT_TEXT_FONT_FAMILY.to_string()),
        font_size: Some(DEFAULT_TEXT_FONT_SIZE),
        fill: Some(DEFAULT_TEXT_FILL.to_string()),
        align: Some("left".to_string()),
        line_height: Some(DEFAULT_TEXT_LINE_HEIGHT),
        box_value: None,
        anchor_offset: None,
        measured_size: None,
        preserve_lines: true,
        default_chemical: true,
    };
    apply_node_label_text_edit(node, label, &session, connection_angles, node.position)
}

fn apply_node_label_text_edit(
    node: &mut crate::Node,
    text: &str,
    session: &TextEditSession,
    connection_angles: &[f64],
    anchor_position: [f64; 2],
) -> bool {
    let previous_element = node.element.clone();
    let previous_atomic_number = node.atomic_number;
    let previous_is_placeholder = node.is_placeholder;
    let previous_label = node.label.clone();
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed == "C" {
        let changed = previous_element != "C"
            || previous_atomic_number != 6
            || previous_is_placeholder
            || previous_label.is_some();
        if !changed {
            return false;
        }
        node.element = "C".to_string();
        node.atomic_number = 6;
        node.is_placeholder = false;
        node.label = None;
        return true;
    }

    if let Some(replacement) = classify_node_label_replacement(trimmed) {
        match replacement {
            NodeLabelReplacement::Carbon => {}
            NodeLabelReplacement::Element {
                element,
                atomic_number,
            } => {
                node.element = element.to_string();
                node.atomic_number = atomic_number;
                node.is_placeholder = false;
            }
            NodeLabelReplacement::Abbreviation => {
                node.element = "C".to_string();
                node.atomic_number = 6;
                node.is_placeholder = true;
            }
        }
    } else {
        node.element = "C".to_string();
        node.atomic_number = 6;
        node.is_placeholder = true;
    }

    let source_runs = normalize_source_runs(session, text);
    let display_runs = display_runs_from_source_runs(
        &source_runs,
        session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY),
        session.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE),
        session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL),
    );
    let next_label = make_centered_node_label_from_runs(
        text,
        anchor_position,
        source_runs,
        display_runs,
        session
            .font_family
            .as_deref()
            .unwrap_or(DEFAULT_TEXT_FONT_FAMILY),
        session.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE),
        session.fill.as_deref().unwrap_or(DEFAULT_TEXT_FILL),
        connection_angles,
        session,
    );
    let changed = previous_element != node.element
        || previous_atomic_number != node.atomic_number
        || previous_is_placeholder != node.is_placeholder
        || !same_node_label(previous_label.as_ref(), Some(&next_label));
    if !changed {
        return false;
    }
    node.label = Some(next_label);
    true
}

fn endpoint_label_editor_anchor_world(
    node: &crate::Node,
    object_translate: [f64; 2],
) -> Option<Point> {
    let label = node.label.as_ref()?;
    if let Some(polygon) = label
        .glyph_polygons()
        .into_iter()
        .find(|polygon| !polygon.is_empty())
    {
        if let Some(anchor) = polygon_anchor_point(&polygon) {
            return Some(Point::new(
                anchor.x + object_translate[0],
                anchor.y + object_translate[1],
            ));
        }
    }

    let bbox = label.bbox()?;
    let font_size = label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE);
    let anchor_x = match label.anchor.as_deref() {
        Some("middle") => label
            .position
            .map(|position| position[0])
            .unwrap_or((bbox[0] + bbox[2]) * 0.5),
        Some("end") => bbox[2],
        _ => {
            let source_text = label.source_text.as_deref().unwrap_or(label.text.as_str());
            let first_char = source_text
                .chars()
                .find(|character| !character.is_whitespace())
                .unwrap_or('C');
            bbox[0] + estimated_char_width(first_char, font_size) * 0.5
        }
    };
    let anchor_y = bbox[1] + font_size * 0.44;
    Some(Point::new(
        anchor_x + object_translate[0],
        anchor_y + object_translate[1],
    ))
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

fn payload_text(payload: &crate::ObjectPayload) -> String {
    payload
        .extra
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn payload_box(payload: &crate::ObjectPayload) -> Option<[f64; 4]> {
    payload
        .extra
        .get("box")
        .cloned()
        .and_then(|value| serde_json::from_value::<[f64; 4]>(value).ok())
}

fn payload_runs_or_text(payload: &crate::ObjectPayload) -> Vec<LabelRun> {
    if let Some(value) = payload.extra.get("runs").cloned() {
        if let Ok(runs) = serde_json::from_value::<Vec<LabelRun>>(value) {
            if !runs.is_empty() {
                return runs;
            }
        }
    }
    let text = payload_text(payload);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text,
            font_family: payload
                .extra
                .get("fontFamily")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_size: payload.extra.get("fontSize").and_then(Value::as_f64),
            fill: payload
                .extra
                .get("fill")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
            face: None,
        }]
    }
}

fn runs_text(runs: &[LabelRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

fn normalize_source_runs(session: &TextEditSession, text: &str) -> Vec<LabelRun> {
    let source_runs = if !session.source_runs.is_empty() {
        session.source_runs.clone()
    } else if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text: text.to_string(),
            font_family: session.font_family.clone(),
            font_size: session.font_size,
            fill: session.fill.clone(),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some(if session.default_chemical {
                "chemical".to_string()
            } else {
                "normal".to_string()
            }),
            face: None,
        }]
    };
    source_runs
        .into_iter()
        .filter(|run| !run.text.is_empty())
        .map(|mut run| {
            if run.font_family.is_none() {
                run.font_family = session.font_family.clone();
            }
            if run.font_size.is_none() {
                run.font_size = session.font_size;
            }
            if run.fill.is_none() {
                run.fill = session.fill.clone();
            }
            if run.font_weight.is_none() {
                run.font_weight = Some(400);
            }
            if run.font_style.is_none() {
                run.font_style = Some("normal".to_string());
            }
            if run.underline.is_none() {
                run.underline = Some(false);
            }
            if run.script.is_none() {
                run.script = Some(if session.default_chemical {
                    "chemical".to_string()
                } else {
                    "normal".to_string()
                });
            }
            run
        })
        .collect()
}

fn display_runs_from_source_runs(
    source_runs: &[LabelRun],
    fallback_font_family: &str,
    fallback_font_size: f64,
    fallback_fill: &str,
) -> Vec<LabelRun> {
    let mut out = Vec::new();
    for run in source_runs {
        if run.text.is_empty() {
            continue;
        }
        let base = LabelRun {
            text: String::new(),
            font_family: Some(
                run.font_family
                    .clone()
                    .unwrap_or_else(|| fallback_font_family.to_string()),
            ),
            font_size: Some(run.font_size.unwrap_or(fallback_font_size)),
            fill: Some(
                run.fill
                    .clone()
                    .unwrap_or_else(|| fallback_fill.to_string()),
            ),
            font_weight: Some(run.font_weight.unwrap_or(400)),
            font_style: Some(
                run.font_style
                    .clone()
                    .unwrap_or_else(|| "normal".to_string()),
            ),
            underline: Some(run.underline.unwrap_or(false)),
            script: Some("normal".to_string()),
            face: None,
        };
        match run.script.as_deref().unwrap_or("normal") {
            "chemical" => out.extend(expand_chemical_run(&base, &run.text)),
            "subscript" | "superscript" => {
                let mut next = base.clone();
                next.text = run.text.clone();
                next.script = run.script.clone();
                out.push(next);
            }
            _ => {
                let mut next = base.clone();
                next.text = run.text.clone();
                out.push(next);
            }
        }
    }
    merge_adjacent_runs(out)
}

fn merge_adjacent_runs(runs: Vec<LabelRun>) -> Vec<LabelRun> {
    let mut merged: Vec<LabelRun> = Vec::new();
    for run in runs {
        if let Some(previous) = merged.last_mut() {
            if previous.font_family == run.font_family
                && previous.font_size == run.font_size
                && previous.fill == run.fill
                && previous.font_weight == run.font_weight
                && previous.font_style == run.font_style
                && previous.script == run.script
            {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        merged.push(run);
    }
    merged
}

fn expand_chemical_run(base: &LabelRun, text: &str) -> Vec<LabelRun> {
    let chars: Vec<char> = text.chars().collect();
    let mut scripts = vec!["normal"; chars.len()];

    for index in 0..chars.len() {
        let character = chars[index];
        if character.is_ascii_digit() && index > 0 && chars[index - 1].is_ascii_alphabetic() {
            scripts[index] = "subscript";
        }
        if matches!(character, '+' | '-') {
            scripts[index] = "superscript";
            if index > 0 && chars[index - 1].is_ascii_digit() {
                let previous_index = index - 1;
                if previous_index > 0 && !chars[previous_index - 1].is_whitespace() {
                    scripts[previous_index] = "superscript";
                }
            }
        }
    }

    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut active_script = "normal";
    for (index, character) in chars.into_iter().enumerate() {
        let script = scripts[index];
        if !buffer.is_empty() && script != active_script {
            let mut run = base.clone();
            run.text = std::mem::take(&mut buffer);
            run.script = Some(active_script.to_string());
            out.push(run);
        }
        active_script = script;
        buffer.push(character);
    }
    if !buffer.is_empty() {
        let mut run = base.clone();
        run.text = buffer;
        run.script = Some(active_script.to_string());
        out.push(run);
    }
    out
}

fn estimate_text_block_size(runs: &[LabelRun], font_size: f64, line_height: f64) -> (f64, f64) {
    let mut max_width = font_size * 0.6;
    let mut line_width = 0.0;
    let mut line_count = 1usize;

    for run in runs {
        let script_scale = crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        let run_font_size = run.font_size.unwrap_or(font_size) * script_scale;
        for character in run.text.chars() {
            if character == '\n' {
                max_width = max_width.max(line_width);
                line_width = 0.0;
                line_count += 1;
                continue;
            }
            line_width += estimated_char_width(character, run_font_size);
        }
    }
    max_width = max_width.max(line_width);
    let width = round2((max_width + font_size * 0.24).max(8.0));
    let height = round2((line_height * line_count as f64).max(line_height));
    (width, height)
}

fn estimated_char_width(character: char, font_size: f64) -> f64 {
    crate::glyph_kernel::shared_estimated_char_width(character, font_size)
}

#[allow(clippy::too_many_arguments)]
fn update_text_object_fields(
    object: &mut crate::SceneObject,
    x: f64,
    y: f64,
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
) -> bool {
    let next_payload = make_text_payload(text, source_runs, display_runs, session, width, height);
    let changed =
        object.transform.translate != [x, y] || object.payload.extra != next_payload.extra;
    if !changed {
        return false;
    }
    object.transform.translate = [round2(x), round2(y)];
    object.payload = next_payload;
    object.style_ref = None;
    object.visible = true;
    object.locked = false;
    true
}

#[allow(clippy::too_many_arguments)]
fn make_text_object(
    object_id: &str,
    x: f64,
    y: f64,
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
    z_index: i32,
) -> crate::SceneObject {
    crate::SceneObject {
        id: object_id.to_string(),
        object_type: "text".to_string(),
        name: "text".to_string(),
        visible: true,
        locked: false,
        z_index,
        transform: crate::Transform {
            translate: [round2(x), round2(y)],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: Value::Null,
        payload: make_text_payload(text, source_runs, display_runs, session, width, height),
    }
}

fn make_text_payload(
    text: &str,
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    session: &TextEditSession,
    width: f64,
    height: f64,
) -> crate::ObjectPayload {
    let mut extra = std::collections::BTreeMap::new();
    extra.insert("text".to_string(), Value::String(text.to_string()));
    extra.insert(
        "align".to_string(),
        Value::String(session.align.clone().unwrap_or_else(|| "left".to_string())),
    );
    extra.insert("valign".to_string(), Value::String("top".to_string()));
    extra.insert("preserveLines".to_string(), Value::Bool(true));
    extra.insert(
        "fontFamily".to_string(),
        Value::String(
            session
                .font_family
                .clone()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
    );
    extra.insert(
        "fontSize".to_string(),
        json!(round2(session.font_size.unwrap_or(12.0))),
    );
    extra.insert(
        "fill".to_string(),
        Value::String(
            session
                .fill
                .clone()
                .unwrap_or_else(|| "#000000".to_string()),
        ),
    );
    extra.insert(
        "lineHeight".to_string(),
        json!(round2(session.line_height.unwrap_or(15.0))),
    );
    extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
    extra.insert(
        "runs".to_string(),
        serde_json::to_value(display_runs).unwrap_or(Value::Array(Vec::new())),
    );
    extra.insert(
        "sourceRuns".to_string(),
        serde_json::to_value(source_runs).unwrap_or(Value::Array(Vec::new())),
    );
    crate::ObjectPayload {
        resource_ref: None,
        bbox: Some([0.0, 0.0, width, height]),
        extra,
    }
}

fn text_object_world_bounds(object: &crate::SceneObject) -> Option<[f64; 4]> {
    let local_box = payload_box(&object.payload).or(object
        .payload
        .bbox
        .map(|bbox| [bbox[0], bbox[1], bbox[2], bbox[3]]))?;
    let x = object.transform.translate[0] + local_box[0];
    let y = object.transform.translate[1] + local_box[1];
    Some([x, y, x + local_box[2], y + local_box[3]])
}

fn endpoint_label_world_bounds(node: &crate::Node, object_translate: [f64; 2]) -> Option<[f64; 4]> {
    let bbox = node.label.as_ref()?.bbox()?;
    Some([
        round2(bbox[0] + object_translate[0]),
        round2(bbox[1] + object_translate[1]),
        round2(bbox[2] + object_translate[0]),
        round2(bbox[3] + object_translate[1]),
    ])
}

#[derive(Clone, Copy)]
enum NodeLabelReplacement<'a> {
    Carbon,
    Element { element: &'a str, atomic_number: u8 },
    Abbreviation,
}

fn classify_node_label_replacement(label: &str) -> Option<NodeLabelReplacement<'_>> {
    match label {
        "C" => Some(NodeLabelReplacement::Carbon),
        "H" => Some(NodeLabelReplacement::Element {
            element: "H",
            atomic_number: 1,
        }),
        "N" => Some(NodeLabelReplacement::Element {
            element: "N",
            atomic_number: 7,
        }),
        "O" => Some(NodeLabelReplacement::Element {
            element: "O",
            atomic_number: 8,
        }),
        "S" => Some(NodeLabelReplacement::Element {
            element: "S",
            atomic_number: 16,
        }),
        "P" => Some(NodeLabelReplacement::Element {
            element: "P",
            atomic_number: 15,
        }),
        "F" => Some(NodeLabelReplacement::Element {
            element: "F",
            atomic_number: 9,
        }),
        "Cl" => Some(NodeLabelReplacement::Element {
            element: "Cl",
            atomic_number: 17,
        }),
        "Br" => Some(NodeLabelReplacement::Element {
            element: "Br",
            atomic_number: 35,
        }),
        "I" => Some(NodeLabelReplacement::Element {
            element: "I",
            atomic_number: 53,
        }),
        "Si" => Some(NodeLabelReplacement::Element {
            element: "Si",
            atomic_number: 14,
        }),
        "Na" => Some(NodeLabelReplacement::Element {
            element: "Na",
            atomic_number: 11,
        }),
        "B" => Some(NodeLabelReplacement::Element {
            element: "B",
            atomic_number: 5,
        }),
        "D" => Some(NodeLabelReplacement::Element {
            element: "D",
            atomic_number: 1,
        }),
        "Me" => Some(NodeLabelReplacement::Abbreviation),
        "Ph" => Some(NodeLabelReplacement::Abbreviation),
        _ => None,
    }
}

fn make_centered_node_label(text: &str, position: [f64; 2]) -> crate::NodeLabel {
    let font_size = 15.0;
    let (label_position, label_box) = estimated_centered_label_geometry(text, position, font_size);
    crate::NodeLabel {
        text: text.to_string(),
        source_text: Some(text.to_string()),
        position: Some(label_position),
        box_field: Some(label_box),
        runs: vec![crate::LabelRun {
            text: text.to_string(),
            font_family: Some("Arial".to_string()),
            font_size: Some(font_size),
            fill: Some("#000000".to_string()),
            font_weight: Some(700),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
            face: None,
        }],
        line_runs: Vec::new(),
        lines: Vec::new(),
        align: Some("center".to_string()),
        layout: None,
        attachment: None,
        anchor: Some("middle".to_string()),
        font_family: Some("Arial".to_string()),
        fill: Some("#000000".to_string()),
        font_size: Some(font_size),
        glyph_polygons: Vec::new(),
        box_value: Some(label_box),
        meta: serde_json::Value::Null,
    }
}

fn make_centered_node_label_from_runs(
    text: &str,
    position: [f64; 2],
    source_runs: Vec<LabelRun>,
    display_runs: Vec<LabelRun>,
    font_family: &str,
    font_size: f64,
    fill: &str,
    connection_angles: &[f64],
    session: &TextEditSession,
) -> crate::NodeLabel {
    let decision = decide_label_layout(connection_angles, false, false);
    let layout = layout_label_text(text, &decision);
    let (lines, line_runs) = layout_display_runs(&display_runs, &decision);
    let line_height = (font_size * 1.05).max(font_size);
    let estimated_width = lines
        .iter()
        .zip(line_runs.iter())
        .map(|(_, runs)| estimate_line_runs_width(runs, font_size))
        .fold(font_size * 0.6, f64::max);
    let estimated_height = round2((line_height * lines.len().max(1) as f64).max(line_height));
    let anchor_prefix_width = line_runs
        .get(layout.anchor_line)
        .map(|runs| estimate_prefix_width(runs, layout.anchor_char, font_size))
        .unwrap_or(0.0);
    let anchor_char_width = line_runs
        .get(layout.anchor_line)
        .and_then(|runs| estimate_anchor_char_width(runs, layout.anchor_char, font_size))
        .unwrap_or(font_size * 0.62);
    let anchor_center_x = anchor_prefix_width + anchor_char_width * 0.5;
    let can_use_measured_geometry =
        matches!(decision.flow, LabelFlow::Forward) && lines.len() == 1 && layout.anchor_line == 0;
    let measured_anchor = session.anchor_offset.map(|value| (value[0], value[1]));
    let measured_size = session.measured_size.map(|value| (value[0], value[1]));
    let (width, height, mut x1, mut y1, mut baseline_y) = if can_use_measured_geometry {
        if let (Some((anchor_offset_x, anchor_offset_y)), Some((measured_width, measured_height))) =
            (measured_anchor, measured_size)
        {
            let x1 = round2(position[0] - anchor_offset_x);
            let y1 = round2(position[1] - anchor_offset_y);
            let width = round2(measured_width.max(estimated_width));
            let height = round2(measured_height.max(estimated_height));
            let baseline_y = round2(y1 + font_size * 0.82);
            (width, height, x1, y1, baseline_y)
        } else {
            let x1 = round2(position[0] - anchor_center_x);
            let y1 =
                round2(position[1] - font_size * 0.42 - layout.anchor_line as f64 * line_height);
            let baseline_y =
                round2(y1 + layout.anchor_line as f64 * line_height + font_size * 0.82);
            (estimated_width, estimated_height, x1, y1, baseline_y)
        }
    } else {
        let x1 = round2(position[0] - anchor_center_x);
        let y1 = round2(position[1] - font_size * 0.42 - layout.anchor_line as f64 * line_height);
        let baseline_y = round2(y1 + layout.anchor_line as f64 * line_height + font_size * 0.82);
        (estimated_width, estimated_height, x1, y1, baseline_y)
    };
    let mut x2 = round2(x1 + width);
    let mut y2 = round2(y1 + height);
    let mut meta = serde_json::Map::new();
    meta.insert(
        "sourceRuns".to_string(),
        serde_json::to_value(source_runs).unwrap_or(Value::Array(Vec::new())),
    );
    let mut glyph_polygons = build_label_glyph_polygons(
        if line_runs.len() == 1 {
            line_runs.first().map(Vec::as_slice).unwrap_or(&[])
        } else {
            &[]
        },
        if line_runs.len() > 1 { &line_runs } else { &[] },
        [x1, baseline_y],
        Some([x1, y1, x2, y2]),
        font_size,
    );
    if lines.len() == 1 {
        if let Some(current_anchor) = glyph_polygons
            .get(layout.anchor_char)
            .and_then(|polygon| {
                let points: Vec<_> = polygon
                    .iter()
                    .map(|point| Point::new(point[0], point[1]))
                    .collect();
                polygon_anchor_point(&points)
            })
        {
            let dx = round2(position[0] - current_anchor.x);
            let dy = round2(position[1] - current_anchor.y);
            if dx.abs() > crate::EPSILON || dy.abs() > crate::EPSILON {
                x1 = round2(x1 + dx);
                y1 = round2(y1 + dy);
                x2 = round2(x2 + dx);
                y2 = round2(y2 + dy);
                baseline_y = round2(baseline_y + dy);
                for polygon in &mut glyph_polygons {
                    for point in polygon {
                        point[0] = round2(point[0] + dx);
                        point[1] = round2(point[1] + dy);
                    }
                }
            }
        }
    }
    crate::NodeLabel {
        text: layout.rendered_text,
        source_text: Some(text.to_string()),
        position: Some([x1, baseline_y]),
        box_field: Some([x1, y1, x2, y2]),
        runs: if line_runs.len() == 1 {
            line_runs.first().cloned().unwrap_or_default()
        } else {
            Vec::new()
        },
        line_runs: if line_runs.len() > 1 {
            line_runs
        } else {
            Vec::new()
        },
        lines: if lines.len() > 1 {
            lines.clone()
        } else {
            Vec::new()
        },
        align: Some("left".to_string()),
        layout: Some(match decision.flow {
            LabelFlow::StackAbove => "attached-group-above".to_string(),
            _ => "attached-group".to_string(),
        }),
        attachment: Some("node".to_string()),
        anchor: Some("start".to_string()),
        font_family: Some(font_family.to_string()),
        fill: Some(fill.to_string()),
        font_size: Some(font_size),
        glyph_polygons,
        box_value: Some([x1, y1, x2, y2]),
        meta: Value::Object(meta),
    }
}

#[derive(Clone)]
struct StyledGlyph {
    ch: char,
    run: LabelRun,
}

fn layout_display_runs(
    display_runs: &[LabelRun],
    decision: &crate::LabelLayoutDecision,
) -> (Vec<String>, Vec<Vec<LabelRun>>) {
    let groups = split_styled_groups(display_runs);
    if groups.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let lines = match decision.flow {
        LabelFlow::Forward => vec![groups.concat()],
        LabelFlow::Reverse => vec![groups.into_iter().rev().flatten().collect()],
        LabelFlow::StackAbove => {
            if groups.len() > 1 {
                vec![groups[1..].concat(), groups[0].clone()]
            } else {
                vec![groups[0].clone()]
            }
        }
        LabelFlow::StackBelow => {
            if groups.len() > 1 {
                vec![groups[0].clone(), groups[1..].concat()]
            } else {
                vec![groups[0].clone()]
            }
        }
    };
    let line_texts = lines
        .iter()
        .map(|line| line.iter().map(|glyph| glyph.ch).collect::<String>())
        .collect::<Vec<_>>();
    let line_runs = lines
        .iter()
        .map(|line| merge_styled_glyph_runs(line))
        .collect();
    (line_texts, line_runs)
}

fn split_styled_groups(display_runs: &[LabelRun]) -> Vec<Vec<StyledGlyph>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for run in display_runs {
        for ch in run.text.chars() {
            if ch.is_whitespace() {
                continue;
            }
            if ch.is_ascii_uppercase() && !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            current.push(StyledGlyph {
                ch,
                run: LabelRun {
                    text: ch.to_string(),
                    font_family: run.font_family.clone(),
                    font_size: run.font_size,
                    fill: run.fill.clone(),
                    font_weight: run.font_weight,
                    font_style: run.font_style.clone(),
                    underline: run.underline,
                    script: run.script.clone(),
                    face: run.face,
                },
            });
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

fn merge_styled_glyph_runs(line: &[StyledGlyph]) -> Vec<LabelRun> {
    let mut runs: Vec<LabelRun> = Vec::new();
    for glyph in line {
        if let Some(previous) = runs.last_mut() {
            if previous.font_family == glyph.run.font_family
                && previous.font_size == glyph.run.font_size
                && previous.fill == glyph.run.fill
                && previous.font_weight == glyph.run.font_weight
                && previous.font_style == glyph.run.font_style
                && previous.underline == glyph.run.underline
                && previous.script == glyph.run.script
            {
                previous.text.push(glyph.ch);
                continue;
            }
        }
        let mut next = glyph.run.clone();
        next.text = glyph.ch.to_string();
        runs.push(next);
    }
    runs
}

fn estimate_line_runs_width(runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    runs.iter().fold(0.0, |width, run| {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        width
            + run
                .text
                .chars()
                .map(|ch| estimated_char_width(ch, run_font_size))
                .sum::<f64>()
    })
}

fn estimate_prefix_width(runs: &[LabelRun], char_count: usize, fallback_font_size: f64) -> f64 {
    let mut remaining = char_count;
    let mut width = 0.0;
    for run in runs {
        if remaining == 0 {
            break;
        }
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        for ch in run.text.chars() {
            if remaining == 0 {
                break;
            }
            width += estimated_char_width(ch, run_font_size);
            remaining -= 1;
        }
    }
    width
}

fn estimate_anchor_char_width(
    runs: &[LabelRun],
    char_index: usize,
    fallback_font_size: f64,
) -> Option<f64> {
    let mut current_index = 0usize;
    for run in runs {
        let run_font_size = run.font_size.unwrap_or(fallback_font_size)
            * crate::glyph_kernel::shared_script_scale_factor(run.script.as_deref());
        for ch in run.text.chars() {
            if current_index == char_index {
                return Some(estimated_char_width(ch, run_font_size));
            }
            current_index += 1;
        }
    }
    None
}

fn adjacent_angles_for_fragment_node(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
) -> Vec<f64> {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Vec::new();
    };
    let point = Point::new(node.position[0], node.position[1]);
    let mut out = Vec::new();
    for bond in &fragment.bonds {
        if bond.begin != node_id && bond.end != node_id {
            continue;
        }
        let other_id = if bond.begin == node_id {
            &bond.end
        } else {
            &bond.begin
        };
        let Some(other) = fragment.nodes.iter().find(|node| &node.id == other_id) else {
            continue;
        };
        out.push(crate::angle_between(
            point,
            Point::new(other.position[0], other.position[1]),
        ));
    }
    out
}

fn same_node_label(current: Option<&crate::NodeLabel>, next: Option<&crate::NodeLabel>) -> bool {
    match (current, next) {
        (None, None) => true,
        (Some(current), Some(next)) => {
            current.text == next.text
                && current.align == next.align
                && current.runs == next.runs
                && current.font_family == next.font_family
                && current.font_size == next.font_size
                && current.fill == next.fill
        }
        _ => false,
    }
}

fn estimated_centered_label_geometry(
    text: &str,
    center: [f64; 2],
    font_size: f64,
) -> ([f64; 2], [f64; 4]) {
    let width = text
        .chars()
        .map(|ch| estimated_char_width(ch, font_size))
        .sum::<f64>()
        .max(crate::glyph_kernel::shared_estimated_char_width('C', font_size));
    let height = (font_size * 0.84).max(8.0);
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let x1 = center[0] - half_width;
    let y1 = center[1] - half_height;
    let x2 = center[0] + half_width;
    let y2 = center[1] + half_height;
    ([center[0], y2], [x1, y1, x2, y2])
}

fn refresh_attached_node_label_geometry_for_all_nodes(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    stroke_width: f64,
) {
    let node_ids: Vec<_> = fragment.nodes.iter().map(|node| node.id.clone()).collect();
    for node_id in node_ids {
        refresh_attached_node_label_geometry_for_node(
            fragment,
            object_translate,
            &node_id,
            stroke_width,
        );
    }
}

fn refresh_attached_node_label_geometry_for_node(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    node_id: &str,
    stroke_width: f64,
) {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return;
    };
    let Some(next_label) =
        refreshed_attached_node_label(fragment, node_id, object_translate, stroke_width)
    else {
        return;
    };
    fragment.nodes[node_index].label = Some(next_label);
}

fn is_generated_centered_label(label: &crate::NodeLabel) -> bool {
    label.align.as_deref() == Some("center")
        && label.anchor.as_deref() == Some("middle")
        && label.glyph_polygons.is_empty()
        && label.runs.len() == 1
}

fn is_attached_node_label(label: &crate::NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.align.as_deref() == Some("left")
        && label.anchor.as_deref() == Some("start")
}

fn refreshed_attached_node_label(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
    object_translate: [f64; 2],
    stroke_width: f64,
) -> Option<crate::NodeLabel> {
    let node = fragment.nodes.iter().find(|node| node.id == node_id)?;
    let label = node.label.as_ref()?;
    let world_anchor = attached_node_label_anchor_world(fragment, node_id, object_translate, stroke_width);
    let local_anchor = [
        round2(world_anchor.x - object_translate[0]),
        round2(world_anchor.y - object_translate[1]),
    ];
    if is_generated_centered_label(label) {
        return Some(make_centered_node_label(&label.text, local_anchor));
    }
    if !is_attached_node_label(label) {
        return None;
    }

    let source_runs = source_runs_from_node_label(label);
    let text = if !source_runs.is_empty() {
        runs_text(&source_runs)
    } else {
        label
            .source_text
            .clone()
            .unwrap_or_else(|| label.text.clone())
    };
    let font_family = label
        .font_family
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FONT_FAMILY.to_string());
    let font_size = label.font_size.unwrap_or(DEFAULT_TEXT_FONT_SIZE);
    let fill = label
        .fill
        .clone()
        .unwrap_or_else(|| DEFAULT_TEXT_FILL.to_string());
    let display_runs = display_runs_from_source_runs(
        &source_runs,
        &font_family,
        font_size,
        &fill,
    );
    let connection_angles = adjacent_angles_for_fragment_node(fragment, node_id);
    let (anchor_offset, measured_size) =
        current_node_label_measurements(node, object_translate);
    let session = TextEditSession {
        target: TextEditTarget::EndpointLabel {
            node_id: node_id.to_string(),
            x: world_anchor.x,
            y: world_anchor.y,
        },
        text: text.clone(),
        source_runs: source_runs.clone(),
        font_family: Some(font_family.clone()),
        font_size: Some(font_size),
        fill: Some(fill.clone()),
        align: Some("left".to_string()),
        line_height: Some((font_size * 1.05).max(font_size)),
        box_value: None,
        anchor_offset,
        measured_size,
        preserve_lines: true,
        default_chemical: source_runs
            .iter()
            .any(|run| run.script.as_deref() == Some("chemical")),
    };
    Some(make_centered_node_label_from_runs(
        &text,
        local_anchor,
        source_runs,
        display_runs,
        &font_family,
        font_size,
        &fill,
        &connection_angles,
        &session,
    ))
}

fn source_runs_from_node_label(label: &crate::NodeLabel) -> Vec<LabelRun> {
    label
        .meta
        .get("sourceRuns")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .or_else(|| (!label.runs.is_empty()).then(|| label.runs.clone()))
        .unwrap_or_else(|| {
            let text = label
                .source_text
                .clone()
                .unwrap_or_else(|| label.text.clone());
            if text.is_empty() {
                Vec::new()
            } else {
                vec![LabelRun {
                    text,
                    font_family: label.font_family.clone(),
                    font_size: label.font_size,
                    fill: label.fill.clone(),
                    font_weight: Some(400),
                    font_style: Some("normal".to_string()),
                    underline: Some(false),
                    script: Some("normal".to_string()),
                    face: None,
                }]
            }
        })
}

fn current_node_label_measurements(
    node: &crate::Node,
    object_translate: [f64; 2],
) -> (Option<[f64; 2]>, Option<[f64; 2]>) {
    let Some(bounds) = endpoint_label_world_bounds(node, object_translate) else {
        return (None, None);
    };
    let measured_size = Some([
        round2((bounds[2] - bounds[0]).max(0.0)),
        round2((bounds[3] - bounds[1]).max(0.0)),
    ]);
    let anchor_offset = endpoint_label_editor_anchor_world(node, object_translate).map(|anchor| {
        [
            round2(anchor.x - bounds[0]),
            round2(anchor.y - bounds[1]),
        ]
    });
    (anchor_offset, measured_size)
}

fn attached_node_label_anchor_world(
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
        -1.0
    } else {
        1.0
    };
    let normal_x = -dy / length;
    let normal_y = dx / length;
    let offset =
        0.5 * double_bond_offset_distance_for_points(begin_world, end_world, stroke_width) * side;
    Point::new(
        node_world.x + normal_x * offset,
        node_world.y + normal_y * offset,
    )
}

fn double_bond_offset_distance_for_points(start: Point, end: Point, stroke_width: f64) -> f64 {
    const DOUBLE_BOND_OFFSET: f64 = 2.85;
    const VIEWER_BOND_STROKE: f64 = 0.85;
    DOUBLE_BOND_OFFSET
        * (stroke_width / VIEWER_BOND_STROKE)
        * ((start.distance(end) / DEFAULT_BOND_LENGTH.max(crate::EPSILON)).max(0.01))
}

fn update_terminal_double_bond_placement_after_new_attachment(
    fragment: &mut crate::MoleculeFragment,
    attached_node_id: &str,
    new_bond_id: &str,
) {
    let connected_bond_ids: Vec<_> = fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == attached_node_id || bond.end == attached_node_id)
        .map(|bond| bond.id.clone())
        .collect();
    for bond_id in connected_bond_ids {
        if bond_id != new_bond_id {
            update_unfrozen_double_bond_auto_placement(fragment, &bond_id, new_bond_id);
        }
    }
}

fn update_unfrozen_double_bond_auto_placement(
    fragment: &mut crate::MoleculeFragment,
    double_bond_id: &str,
    new_bond_id: &str,
) {
    let Some(double_index) = fragment
        .bonds
        .iter()
        .position(|bond| bond.id == double_bond_id && bond.order == 2)
    else {
        return;
    };
    let Some(double) = fragment.bonds[double_index].double.as_ref() else {
        return;
    };
    if double.frozen {
        return;
    }

    let bond = fragment.bonds[double_index].clone();
    let Some(begin) = fragment.nodes.iter().find(|node| node.id == bond.begin) else {
        return;
    };
    let Some(end) = fragment.nodes.iter().find(|node| node.id == bond.end) else {
        return;
    };
    let begin_point = begin.point();
    let end_point = end.point();
    let axis_x = end_point.x - begin_point.x;
    let axis_y = end_point.y - begin_point.y;
    let axis_length = axis_x.hypot(axis_y);
    if axis_length <= crate::EPSILON {
        return;
    }
    let normal_x = -axis_y / axis_length;
    let normal_y = axis_x / axis_length;

    let mut left_count = 0usize;
    let mut right_count = 0usize;
    let mut new_bond_side: Option<DoubleBondPlacement> = None;
    for other in &fragment.bonds {
        if other.id == bond.id {
            continue;
        }
        let shared_id = if other.begin == bond.begin || other.end == bond.begin {
            Some(bond.begin.as_str())
        } else if other.begin == bond.end || other.end == bond.end {
            Some(bond.end.as_str())
        } else {
            None
        };
        let Some(shared_id) = shared_id else {
            continue;
        };
        let other_id = if other.begin == shared_id {
            other.end.as_str()
        } else {
            other.begin.as_str()
        };
        let Some(shared_node) = fragment.nodes.iter().find(|node| node.id == shared_id) else {
            continue;
        };
        let Some(other_node) = fragment.nodes.iter().find(|node| node.id == other_id) else {
            continue;
        };
        let side_score = (other_node.position[0] - shared_node.position[0]) * normal_x
            + (other_node.position[1] - shared_node.position[1]) * normal_y;
        let side = if side_score < -crate::EPSILON {
            Some(DoubleBondPlacement::Left)
        } else if side_score > crate::EPSILON {
            Some(DoubleBondPlacement::Right)
        } else {
            None
        };
        match side {
            Some(DoubleBondPlacement::Left) => left_count += 1,
            Some(DoubleBondPlacement::Right) => right_count += 1,
            _ => {}
        }
        if other.id == new_bond_id {
            new_bond_side = side;
        }
    }

    let placement = if left_count > right_count {
        Some(DoubleBondPlacement::Left)
    } else if right_count > left_count {
        Some(DoubleBondPlacement::Right)
    } else {
        new_bond_side
    };
    let Some(placement) = placement else {
        return;
    };
    fragment.bonds[double_index].double = Some(crate::DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
}

fn opposite_double_bond_placement(placement: DoubleBondPlacement) -> DoubleBondPlacement {
    match placement {
        DoubleBondPlacement::Left => DoubleBondPlacement::Right,
        DoubleBondPlacement::Right => DoubleBondPlacement::Left,
        DoubleBondPlacement::Center => DoubleBondPlacement::Right,
    }
}

fn apply_single_tool_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if is_plain_single_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    if is_plain_double_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    replace_with_plain_single_bond_style(bond)
}

fn apply_double_tool_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if is_plain_single_bond(bond) || is_plain_triple_bond(bond) {
        return replace_with_plain_double_bond_style(bond, default_placement);
    }
    if is_plain_double_bond(bond) {
        return advance_plain_double_cycle(bond, default_placement);
    }
    if is_bold_family_bond(bond) {
        return if bond.order == 2 {
            cycle_bold_double_bond_style(bond, Some(default_placement))
        } else {
            cycle_bold_single_bond_style(bond, Some(default_placement))
        };
    }
    replace_with_plain_double_bond_style(bond, default_placement)
}

fn cycle_dashed_bond_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        return cycle_dashed_double_bond_style(bond, Some(default_placement));
    }
    replace_with_plain_dashed_bond_style(bond)
}

fn cycle_dashed_double_bond_tool_center_style(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        return advance_plain_dashed_double_cycle(bond, default_placement);
    }
    replace_with_plain_dashed_double_bond_style(bond, default_placement)
}

fn cycle_bold_bond_center_style(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
    if bond.order == 2 && !has_stereo_style(bond) {
        if is_bold_family_bond(bond) {
            return cycle_bold_double_bond_style(bond, Some(default_placement));
        }
        let placement = bond
            .double
            .as_ref()
            .map(|double| double.placement)
            .unwrap_or(default_placement);
        return init_bold_double_bond_style(bond, placement, default_placement);
    }
    if bond.order == 1 && !has_stereo_style(bond) && all_line_patterns_solid(bond) {
        return cycle_bold_single_bond_style(bond, Some(default_placement));
    }
    if is_bold_family_bond(bond) && bond.order == 2 {
        return cycle_bold_double_bond_style(bond, Some(default_placement));
    }
    replace_with_plain_bold_bond_style(bond)
}

fn cycle_dashed_double_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    let default_side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(default_side);
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            let side_pattern = outer_line_pattern_mut(&mut bond.line_styles, placement);
            if *side_pattern != BondLinePattern::Dashed {
                *side_pattern = BondLinePattern::Dashed;
            } else if bond.line_styles.main != BondLinePattern::Dashed {
                bond.line_styles.main = BondLinePattern::Dashed;
            } else {
                let exit_side = opposite_double_bond_placement(placement);
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(exit_side),
                    frozen: false,
                });
                bond.line_styles.main = BondLinePattern::Solid;
                bond.line_styles.left = BondLinePattern::Dashed;
                bond.line_styles.right = BondLinePattern::Dashed;
            }
            true
        }
        DoubleBondPlacement::Center => {
            let dashed_sides = centered_dashed_sides(&bond.line_styles);
            if dashed_sides.is_empty() {
                *outer_line_pattern_mut(&mut bond.line_styles, default_side) =
                    BondLinePattern::Dashed;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: None,
                    frozen: false,
                });
                return true;
            }
            if dashed_sides.len() == 1 {
                let first_dashed = dashed_sides[0];
                let second_side = opposite_double_bond_placement(first_dashed);
                *outer_line_pattern_mut(&mut bond.line_styles, second_side) =
                    BondLinePattern::Dashed;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(opposite_double_bond_placement(first_dashed)),
                    frozen: false,
                });
                return true;
            }

            let exit_side = bond
                .double
                .as_ref()
                .and_then(|double| double.center_exit_side)
                .unwrap_or(default_side);
            bond.double = Some(DoubleBond {
                placement: exit_side,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_styles.main = BondLinePattern::Solid;
            bond.line_styles.left = BondLinePattern::Solid;
            bond.line_styles.right = BondLinePattern::Solid;
            *outer_line_pattern_mut(&mut bond.line_styles, exit_side) = BondLinePattern::Dashed;
            true
        }
    }
}

fn advance_plain_dashed_double_cycle(
    bond: &mut Bond,
    default_placement: DoubleBondPlacement,
) -> bool {
    let opposite_placement = opposite_double_bond_placement(default_placement);
    let dashed_side = current_dashed_double_side(bond);
    let next_placement = match bond.double.as_ref().map(|double| double.placement) {
        Some(current) if current == default_placement => DoubleBondPlacement::Center,
        Some(DoubleBondPlacement::Center) if dashed_side == Some(default_placement) => {
            opposite_placement
        }
        Some(current) if current == opposite_placement => DoubleBondPlacement::Center,
        Some(DoubleBondPlacement::Center) if dashed_side == Some(opposite_placement) => {
            default_placement
        }
        _ => default_placement,
    };

    bond.order = 2;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.double = Some(DoubleBond {
        placement: next_placement,
        center_exit_side: None,
        frozen: false,
    });

    let next_dashed_side = match next_placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => next_placement,
        DoubleBondPlacement::Center => dashed_side.unwrap_or(default_placement),
    };
    *outer_line_pattern_mut(&mut bond.line_styles, next_dashed_side) = BondLinePattern::Dashed;
    true
}

fn advance_plain_double_cycle(bond: &mut Bond, default_placement: DoubleBondPlacement) -> bool {
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
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_single_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_double_bond_style(bond: &mut Bond, placement: DoubleBondPlacement) -> bool {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_triple_bond_style(bond: &mut Bond) -> bool {
    bond.order = 3;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_dashed_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Dashed,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights::default();
    true
}

fn replace_with_plain_dashed_double_bond_style(
    bond: &mut Bond,
    placement: DoubleBondPlacement,
) -> bool {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    *outer_line_pattern_mut(&mut bond.line_styles, placement) = BondLinePattern::Dashed;
    true
}

fn current_dashed_double_side(bond: &Bond) -> Option<DoubleBondPlacement> {
    let left_dashed = bond.line_styles.left == BondLinePattern::Dashed;
    let right_dashed = bond.line_styles.right == BondLinePattern::Dashed;
    match (left_dashed, right_dashed) {
        (true, false) => Some(DoubleBondPlacement::Left),
        (false, true) => Some(DoubleBondPlacement::Right),
        _ => None,
    }
}

fn replace_with_plain_bold_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights {
        main: BondLineWeight::Bold,
        ..BondLineWeights::default()
    };
    true
}

fn replace_with_bold_dashed_bond_style(bond: &mut Bond) -> bool {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles {
        main: BondLinePattern::Dashed,
        ..BondLineStyles::default()
    };
    bond.line_weights = BondLineWeights {
        main: BondLineWeight::Bold,
        ..BondLineWeights::default()
    };
    true
}

fn replace_with_stereo_bond_style(bond: &mut Bond, variant: BondVariant) -> bool {
    let kind = match variant {
        BondVariant::Wedge => "solid-wedge",
        BondVariant::HashedWedge => "hashed-wedge",
        _ => return false,
    };
    let current_wide_end = bond
        .stereo
        .as_ref()
        .map(|stereo| stereo.wide_end.as_str())
        .unwrap_or("end");
    let next_wide_end = match bond.stereo.as_ref() {
        Some(stereo) if stereo.kind == kind && stereo.wide_end == "end" => "begin",
        Some(stereo) if stereo.kind == kind && stereo.wide_end == "begin" => "end",
        Some(_) => current_wide_end,
        None => "end",
    };
    bond.order = 1;
    bond.double = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.stereo = Some(BondStereo {
        kind: kind.to_string(),
        wide_end: next_wide_end.to_string(),
    });
    true
}

fn init_bold_double_bond_style(
    bond: &mut Bond,
    placement: DoubleBondPlacement,
    default_placement: DoubleBondPlacement,
) -> bool {
    bond.order = 2;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            bond.double = Some(DoubleBond {
                placement,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Bold;
        }
        DoubleBondPlacement::Center => {
            bond.double = Some(DoubleBond {
                placement: DoubleBondPlacement::Center,
                center_exit_side: Some(opposite_double_bond_placement(default_placement)),
                frozen: false,
            });
            *outer_line_weight_mut(&mut bond.line_weights, default_placement) =
                BondLineWeight::Bold;
        }
    }
    true
}

fn cycle_bold_single_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    if bond.line_weights.main != BondLineWeight::Bold {
        bond.line_weights.main = BondLineWeight::Bold;
        return true;
    }

    let side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement: side,
        center_exit_side: None,
        frozen: false,
    });
    bond.line_weights.main = BondLineWeight::Bold;
    bond.line_weights.left = BondLineWeight::Normal;
    bond.line_weights.right = BondLineWeight::Normal;
    true
}

fn cycle_bold_double_bond_style(
    bond: &mut Bond,
    default_placement: Option<DoubleBondPlacement>,
) -> bool {
    let default_side = default_placement.unwrap_or(DoubleBondPlacement::Right);
    let placement = bond
        .double
        .as_ref()
        .map(|double| double.placement)
        .unwrap_or(default_side);
    match placement {
        DoubleBondPlacement::Left | DoubleBondPlacement::Right => {
            if bond.line_weights.main != BondLineWeight::Bold {
                bond.line_weights.main = BondLineWeight::Bold;
                return true;
            }

            let exit_side = opposite_double_bond_placement(placement);
            bond.double = Some(DoubleBond {
                placement: DoubleBondPlacement::Center,
                center_exit_side: Some(exit_side),
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Normal;
            bond.line_weights.left = BondLineWeight::Normal;
            bond.line_weights.right = BondLineWeight::Normal;
            *outer_line_weight_mut(&mut bond.line_weights, placement) = BondLineWeight::Bold;
            true
        }
        DoubleBondPlacement::Center => {
            let bold_sides = centered_bold_sides(&bond.line_weights);
            if bold_sides.is_empty() {
                *outer_line_weight_mut(&mut bond.line_weights, default_side) = BondLineWeight::Bold;
                bond.double = Some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: Some(opposite_double_bond_placement(default_side)),
                    frozen: false,
                });
                return true;
            }

            let exit_side = bond
                .double
                .as_ref()
                .and_then(|double| double.center_exit_side)
                .unwrap_or_else(|| opposite_double_bond_placement(bold_sides[0]));
            bond.double = Some(DoubleBond {
                placement: exit_side,
                center_exit_side: None,
                frozen: false,
            });
            bond.line_weights.main = BondLineWeight::Bold;
            bond.line_weights.left = BondLineWeight::Normal;
            bond.line_weights.right = BondLineWeight::Normal;
            true
        }
    }
}

fn is_plain_single_bond(bond: &Bond) -> bool {
    bond.order == 1
        && bond.double.is_none()
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_plain_double_bond(bond: &Bond) -> bool {
    bond.order == 2
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_plain_triple_bond(bond: &Bond) -> bool {
    bond.order == 3
        && bond.double.is_none()
        && bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && all_line_weights_normal(bond)
}

fn is_bold_family_bond(bond: &Bond) -> bool {
    bond.stereo.is_none()
        && all_line_patterns_solid(bond)
        && (bond.line_weights.main == BondLineWeight::Bold
            || bond.line_weights.left == BondLineWeight::Bold
            || bond.line_weights.right == BondLineWeight::Bold)
}

fn has_stereo_style(bond: &Bond) -> bool {
    bond.stereo.is_some()
}

fn all_line_patterns_solid(bond: &Bond) -> bool {
    bond.line_styles.main == BondLinePattern::Solid
        && bond.line_styles.left == BondLinePattern::Solid
        && bond.line_styles.right == BondLinePattern::Solid
}

fn all_line_weights_normal(bond: &Bond) -> bool {
    bond.line_weights.main == BondLineWeight::Normal
        && bond.line_weights.left == BondLineWeight::Normal
        && bond.line_weights.right == BondLineWeight::Normal
}

fn centered_dashed_sides(line_styles: &BondLineStyles) -> Vec<DoubleBondPlacement> {
    let mut out = Vec::new();
    if line_styles.left == BondLinePattern::Dashed {
        out.push(DoubleBondPlacement::Left);
    }
    if line_styles.right == BondLinePattern::Dashed {
        out.push(DoubleBondPlacement::Right);
    }
    out
}

fn centered_bold_sides(line_weights: &BondLineWeights) -> Vec<DoubleBondPlacement> {
    let mut out = Vec::new();
    if line_weights.left == BondLineWeight::Bold {
        out.push(DoubleBondPlacement::Left);
    }
    if line_weights.right == BondLineWeight::Bold {
        out.push(DoubleBondPlacement::Right);
    }
    out
}

fn outer_line_pattern_mut(
    line_styles: &mut BondLineStyles,
    placement: DoubleBondPlacement,
) -> &mut BondLinePattern {
    match placement {
        DoubleBondPlacement::Left => &mut line_styles.left,
        DoubleBondPlacement::Right => &mut line_styles.right,
        DoubleBondPlacement::Center => &mut line_styles.right,
    }
}

fn outer_line_weight_mut(
    line_weights: &mut BondLineWeights,
    placement: DoubleBondPlacement,
) -> &mut BondLineWeight {
    match placement {
        DoubleBondPlacement::Left => &mut line_weights.left,
        DoubleBondPlacement::Right => &mut line_weights.right,
        DoubleBondPlacement::Center => &mut line_weights.right,
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
