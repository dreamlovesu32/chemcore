mod arrows;
mod bond_styles;
mod bond_tools;
mod brackets;
mod clipboard;
mod command;
mod delete;
mod presets;
mod select;
mod shapes;
mod templates;
mod text_edit;

pub use self::command::{
    CommandAnchor, EditorCommand, FocusedDeleteSource, HistoryEntry, TextEditCommandTarget,
};
pub(crate) use self::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use self::text_edit::{
    endpoint_label_world_bounds, refresh_element_valence_recognition_for_all_nodes,
};
pub use self::text_edit::{
    TextEditLayout, TextEditLayoutCaret, TextEditLayoutCaretOffset, TextEditLayoutLine,
    TextEditLayoutRect, TextEditSelection, TextEditSelectionState, TextEditSession, TextEditTarget,
};

use self::arrows::ensure_arrow_style;
use self::bond_styles::{
    apply_double_tool_center_style, apply_single_tool_center_style, centered_oriented_rect_points,
    cycle_bold_bond_center_style, cycle_dashed_bond_center_style,
    cycle_dashed_double_bond_tool_center_style, preferred_double_bond_side_for_segment,
    replace_with_bold_dashed_bond_style, replace_with_plain_triple_bond_style,
    replace_with_stereo_bond_style, should_default_center_double_bond_for_segment,
    update_terminal_double_bond_placement_after_new_attachment,
};
use self::delete::FocusedDeleteMode;
use self::presets::{document_style_preset_for_options, editor_options_from_cdxml_document};
use crate::{
    adjacent_directions, anchor_from_point, angle_between, bond_center_focus_length, can_draw_bond,
    can_focus_bond_center, can_focus_endpoint, default_angle_for_anchor_for_variant,
    direction_from_angle, endpoint_from_angle_for_document, hit_test_arrow_center,
    hit_test_bond_center, hit_test_endpoint, hit_test_endpoint_excluding, largest_angular_gap,
    nearest_angle, normalize_angle, refresh_repeating_units, render_document,
    render_primitives_bounds, snapped_angle_for_anchor, ArrowCurve, ArrowEndpointStyle,
    ArrowHeadSize, ArrowNoGo, ArrowVariant, Bond, BondAnchor, BondLinePattern, BondLineStyles,
    BondLineWeight, BondLineWeights, BondPreview, BondStereo, BondVariant, ChemcoreDocument,
    DoubleBond, DoubleBondPlacement, DragState, EditorOptions, EndpointHit, HoverTextBox,
    OverlayState, Point, PointerEvent, RenderPrimitive, RenderRole, SceneObject, SelectionState,
    ShapeKind, ShapeStyle, Tool, ToolState, BOND_CENTER_FOCUS_WIDTH, BOND_CENTER_HIT_RADIUS,
    DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS, ENDPOINT_HIT_RADIUS, GLOBAL_SNAP_ANGLES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;

const HOVER_STROKE_WIDTH: f64 = crate::px_to_cm(1.1);
const HOVER_LABEL_STROKE_WIDTH: f64 = crate::px_to_cm(1.1);
const HOVER_ENDPOINT_STROKE_WIDTH: f64 = crate::px_to_cm(1.4);
const HOVER_BOND_CENTER_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const PREVIEW_END_RADIUS: f64 = crate::px_to_cm(5.0);
const PREVIEW_END_STROKE_WIDTH: f64 = crate::px_to_cm(1.2);
const SHAPE_DASH_LENGTH: f64 = 2.7;
const SYMBOL_CLICK_CLEARANCE: f64 = 2.5;
const ELLIPSE_MINOR_AXIS_RATIO: f64 = 0.4;
const ROUND_RECT_CORNER_RADIUS: f64 = 6.0;
const DEFAULT_DOCUMENT_STYLE_PRESET: &str = "default";
const ACS_DOCUMENT_1996_PRESET: &str = "acs-document-1996";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBoundsScope {
    All,
    Document,
    Selection,
}

fn render_bounds_scope_accepts(scope: RenderBoundsScope, primitive: &RenderPrimitive) -> bool {
    match scope {
        RenderBoundsScope::All => true,
        RenderBoundsScope::Document => {
            let role = render_primitive_role(primitive);
            role != RenderRole::DocumentKnockout
                && !render_role_is_selection(role)
                && !render_role_is_hover(role)
                && !render_role_is_preview(role)
        }
        RenderBoundsScope::Selection => render_role_is_selection(render_primitive_role(primitive)),
    }
}

fn render_primitive_role(primitive: &RenderPrimitive) -> RenderRole {
    match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. } => *role,
    }
}

fn render_role_is_selection(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::SelectionBox
            | RenderRole::SelectionBond
            | RenderRole::SelectionBondDot
            | RenderRole::SelectionNode
            | RenderRole::SelectionTextBox
    )
}

fn render_role_is_hover(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::HoverEndpoint
            | RenderRole::HoverLabelGlyph
            | RenderRole::HoverBondCenter
            | RenderRole::HoverArrowCenter
            | RenderRole::HoverArrowHandle
            | RenderRole::HoverTextBox
    )
}

fn render_role_is_preview(role: RenderRole) -> bool {
    matches!(role, RenderRole::PreviewBond | RenderRole::PreviewEnd)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    pub document: ChemcoreDocument,
    pub tool: ToolState,
    pub selection: SelectionState,
    pub overlay: OverlayState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEditLayoutRequest {
    pub session: TextEditSession,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<TextEditSelection>,
}

pub struct Engine {
    state: EngineState,
    drag: Option<DragState>,
    arrow_drag: Option<ArrowDragState>,
    arrow_edit_drag: Option<ArrowEditDragState>,
    selection_drag: Option<select::SelectionMoveDrag>,
    selection_rotate_drag: Option<select::SelectionRotateDrag>,
    template_drag: Option<templates::TemplateDrag>,
    shape_drag: Option<ShapeDragState>,
    bracket_drag: Option<BracketDragState>,
    clipboard: Option<clipboard::ClipboardContent>,
    options: EditorOptions,
    document_style_preset: String,
    next_id: u64,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    command_context: Vec<EditorCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArrowDragState {
    start: Point,
    end: Option<Point>,
    has_dragged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrowEditMode {
    Head,
    Tail,
    Curve,
}

#[derive(Debug, Clone)]
struct ArrowEditDragState {
    object_id: String,
    mode: ArrowEditMode,
    original_points: Vec<Point>,
    start_pointer: Point,
    has_dragged: bool,
    current_degrees: f64,
    undo_pushed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShapeDragState {
    start: Point,
    current: Point,
    has_dragged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BracketDragState {
    start: Point,
    current: Point,
    symbol_anchor: Option<SymbolOrbitAnchor>,
    has_dragged: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolOrbitAnchor {
    point: Point,
    mode: SymbolOrbitMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SymbolOrbitMode {
    Endpoint,
    Label,
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
            arrow_drag: None,
            arrow_edit_drag: None,
            selection_drag: None,
            selection_rotate_drag: None,
            template_drag: None,
            shape_drag: None,
            bracket_drag: None,
            clipboard: None,
            options: EditorOptions::default(),
            document_style_preset: DEFAULT_DOCUMENT_STYLE_PRESET.to_string(),
            next_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            command_context: Vec::new(),
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

    pub fn document_cdxml(&self) -> String {
        crate::document_to_cdxml(&self.state.document)
    }

    pub fn document_svg(&self) -> String {
        crate::document_to_svg(&self.state.document)
    }

    pub fn document_colors(&self) -> Vec<String> {
        collect_document_colors(&self.state.document)
    }

    pub fn render_bounds(&self, scope: RenderBoundsScope) -> Option<[f64; 4]> {
        let primitives = self.render_list();
        render_primitives_bounds(
            primitives
                .iter()
                .filter(|primitive| render_bounds_scope_accepts(scope, primitive)),
        )
    }

    pub fn load_document_json(&mut self, json: &str) -> Result<(), String> {
        let mut document = crate::parse_document_json(json)?;
        refresh_repeating_units(&mut document);
        self.state.document = document;
        self.options = EditorOptions::default();
        self.document_style_preset = DEFAULT_DOCUMENT_STYLE_PRESET.to_string();
        self.refresh_symbol_chemistry();
        if let Some(entry) = self.state.document.editable_fragment_mut() {
            refresh_element_valence_recognition_for_all_nodes(entry.fragment);
        }
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.command_context.clear();
        self.next_id = self.infer_next_id();
        Ok(())
    }

    pub fn load_cdxml_document(&mut self, cdxml: &str) -> Result<(), String> {
        let mut document = crate::parse_cdxml_document(cdxml, None)?;
        crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
        refresh_repeating_units(&mut document);
        let options = editor_options_from_cdxml_document(&document);
        let preset = document_style_preset_for_options(&options).to_string();
        self.state.document = document;
        self.options = options;
        self.document_style_preset = preset;
        self.refresh_symbol_chemistry();
        if let Some(entry) = self.state.document.editable_fragment_mut() {
            refresh_element_valence_recognition_for_all_nodes(entry.fragment);
        }
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.command_context.clear();
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
                node_id: hover.node_id.clone(),
                x: hover.bounds[0],
                y: hover.bounds[1],
                width: (hover.bounds[2] - hover.bounds[0]).max(0.0),
                height: (hover.bounds[3] - hover.bounds[1]).max(0.0),
                fill: Some("rgba(47,111,237,0.12)".to_string()),
                stroke: Some("rgba(47,111,237,0.76)".to_string()),
                stroke_width: HOVER_STROKE_WIDTH,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
        if let Some(hover) = &self.state.overlay.hover_arrow {
            if !self
                .state
                .selection
                .arrow_objects
                .contains(&hover.object_id)
            {
                for handle in &hover.handles {
                    out.push(RenderPrimitive::Circle {
                        role: RenderRole::HoverArrowHandle,
                        object_id: Some(hover.object_id.clone()),
                        node_id: None,
                        center: *handle,
                        radius: crate::px_to_cm(1.5),
                        fill: "#ffffff".to_string(),
                        stroke: "rgba(47,111,237,0.82)".to_string(),
                        stroke_width: HOVER_STROKE_WIDTH,
                    });
                }
            }
        }
        if let Some(hover) = &self.state.overlay.hover_endpoint {
            if let Some(label_anchor) = &hover.label_anchor {
                out.push(RenderPrimitive::Rect {
                    role: RenderRole::HoverLabelGlyph,
                    object_id: None,
                    node_id: Some(hover.node_id.clone()),
                    x: label_anchor.glyph_box[0],
                    y: label_anchor.glyph_box[1],
                    width: (label_anchor.glyph_box[2] - label_anchor.glyph_box[0]).max(0.0),
                    height: (label_anchor.glyph_box[3] - label_anchor.glyph_box[1]).max(0.0),
                    fill: Some("rgba(47,111,237,0.12)".to_string()),
                    stroke: Some("rgba(47,111,237,0.82)".to_string()),
                    stroke_width: HOVER_LABEL_STROKE_WIDTH,
                    rx: None,
                    ry: None,
                    dash_array: Vec::new(),
                    fill_gradient: None,
                });
            } else {
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::HoverEndpoint,
                    object_id: None,
                    node_id: Some(hover.node_id.clone()),
                    center: hover.point,
                    radius: ENDPOINT_FOCUS_RADIUS,
                    fill: "rgba(47,111,237,0.24)".to_string(),
                    stroke: "rgba(47,111,237,0.78)".to_string(),
                    stroke_width: HOVER_ENDPOINT_STROKE_WIDTH,
                });
            }
        }
        if let Some(hover) = &self.state.overlay.hover_bond_center {
            let focus_length = bond_center_focus_length(hover.begin, hover.end);
            if focus_length > crate::EPSILON {
                out.push(RenderPrimitive::Polygon {
                    role: RenderRole::HoverBondCenter,
                    object_id: None,
                    node_id: None,
                    bond_id: None,
                    points: centered_oriented_rect_points(
                        hover.begin,
                        hover.end,
                        focus_length,
                        BOND_CENTER_FOCUS_WIDTH,
                    ),
                    fill: "rgba(47,111,237,0.11)".to_string(),
                    stroke: "rgba(47,111,237,0.72)".to_string(),
                    stroke_width: HOVER_BOND_CENTER_STROKE_WIDTH,
                });
            }
        }
        if let Some(preview) = &self.state.overlay.preview {
            out.push(RenderPrimitive::Circle {
                role: RenderRole::PreviewEnd,
                object_id: None,
                node_id: None,
                center: preview.end,
                radius: PREVIEW_END_RADIUS,
                fill: "#ffffff".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: PREVIEW_END_STROKE_WIDTH,
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
            self.hover_select_target(point);
            return;
        }
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_move_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_move_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Shape {
            self.pointer_move_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Bracket {
            self.pointer_move_bracket(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Symbol {
            self.pointer_move_symbol(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
            self.state.overlay.preview = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.hover_endpoint = None;
            if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: Some(object_id),
                    node_id: None,
                });
                return;
            }
            if let Some(endpoint) =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            {
                self.state.overlay.hover_endpoint = Some(endpoint);
                return;
            }
            if let Some(center) =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
            {
                self.state.overlay.hover_bond_center = Some(center);
            }
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
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
            if let Some(endpoint) =
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
            {
                if endpoint.label_anchor.is_some() {
                    if let Some(entry) = self.state.document.editable_fragment() {
                        if let Some(node) = entry
                            .fragment
                            .nodes
                            .iter()
                            .find(|node| node.id == endpoint.node_id)
                        {
                            if let Some(bounds) =
                                endpoint_label_world_bounds(node, entry.object.transform.translate)
                            {
                                self.state.overlay.hover_text_box = Some(HoverTextBox {
                                    bounds,
                                    object_id: None,
                                    node_id: Some(endpoint.node_id),
                                });
                                return;
                            }
                        }
                    }
                }
                self.state.overlay.hover_endpoint = Some(endpoint);
            }
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
                        self.options.bond_length_world_cm().value(),
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
        self.state.overlay.hover_arrow = None;
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
            return;
        }
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_down_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_down_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Shape {
            self.pointer_down_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Bracket {
            self.pointer_down_bracket(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Symbol {
            self.pointer_down_symbol(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.state.selection = SelectionState::default();
            self.clear_interaction();
            let point = event.point();
            self.with_command(
                EditorCommand::DeleteFocusedAtPoint {
                    x: point.x,
                    y: point.y,
                    source: FocusedDeleteSource::DeleteTool,
                },
                |engine| engine.delete_focused_at_point(point, FocusedDeleteMode::DeleteToolClick),
            );
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
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_up_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.state.overlay.hover_endpoint =
                hit_test_endpoint(&self.state.document, event.point(), ENDPOINT_HIT_RADIUS);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_up_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Shape {
            self.pointer_up_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Bracket {
            self.pointer_up_bracket(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Symbol {
            self.pointer_up_symbol(event);
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
                        self.options.bond_length_world_cm().value(),
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
                self.options.bond_length_world_cm().value(),
            );
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    point: end,
                    label_anchor: None,
                })
        };
        self.state.overlay.preview = None;
        let pointer_point = event.point();
        let added = self.add_bond_between(drag.anchor, end_anchor, self.pending_bond_order());
        if added {
            self.refresh_bond_mode_hover(pointer_point);
        }
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.arrow_drag = None;
        self.arrow_edit_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        self.template_drag = None;
        self.shape_drag = None;
        self.bracket_drag = None;
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
        let command = EditorCommand::AddBond {
            begin: CommandAnchor::from(&anchor),
            end: CommandAnchor::from(&end),
            order,
            variant: self.state.tool.bond_variant,
        };
        self.with_command(command, |engine| {
            engine.add_bond_between_untracked(anchor, end, order)
        })
    }

    fn add_bond_between_untracked(
        &mut self,
        anchor: BondAnchor,
        end: BondAnchor,
        order: u8,
    ) -> bool {
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
            stroke_width: self.options.bond_stroke_world_cm().value(),
            bold_width: Some(self.options.bold_bond_width_world_cm().value()),
            wedge_width: Some(self.options.wedge_width_world_cm().value()),
            label_clip_margin: Some(self.options.label_clip_margin_world_cm().value()),
            hash_spacing: Some(self.options.hash_spacing_world_cm().value()),
            bond_spacing: Some(self.options.bond_spacing_percent()),
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
            self.options.bond_stroke_world_cm().value(),
        );
        entry.update_bounds();
        true
    }

    fn preview_document(&self) -> Option<ChemcoreDocument> {
        if let Some(preview_document) = self.template_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.shape_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.bracket_preview_document() {
            return Some(preview_document);
        }
        if let Some(drag) = self.arrow_drag.as_ref().filter(|drag| drag.has_dragged) {
            let end = drag.end?;
            let mut document = self.state.document.clone();
            let style_id = self.arrow_style_id();
            ensure_arrow_style(&mut document, &style_id, self.options.graphic_stroke_width);
            document.objects.push(self.arrow_scene_object(
                drag.start,
                end,
                "__preview_arrow".to_string(),
                style_id,
            ));
            return Some(document);
        }
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

    pub fn undo(&mut self) -> bool {
        let Some(mut entry) = self.undo_stack.pop() else {
            return false;
        };
        let after = entry
            .after
            .clone()
            .unwrap_or_else(|| self.state.document.clone());
        self.restore_document(entry.before.clone());
        entry.after = Some(after);
        self.redo_stack.push(entry);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_stack.pop() else {
            return false;
        };
        let Some(after) = entry.after.clone() else {
            return false;
        };
        self.restore_document(after);
        self.undo_stack.push(entry);
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
            label_anchor: target.label_anchor,
        })
    }

    fn refresh_bond_mode_hover(&mut self, point: Point) {
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
        if self.state.overlay.hover_endpoint.is_none() && can_focus_bond_center(&self.state.tool) {
            self.state.overlay.hover_bond_center =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS);
        }
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

    fn with_command<F>(&mut self, command: EditorCommand, apply: F) -> bool
    where
        F: FnOnce(&mut Self) -> bool,
    {
        if !self.command_context.is_empty() {
            return apply(self);
        }
        let undo_len = self.undo_stack.len();
        self.command_context.push(command.clone());
        let changed = apply(self);
        self.command_context.pop();
        if changed {
            refresh_repeating_units(&mut self.state.document);
            self.finalize_command_history(undo_len, command);
        }
        changed
    }

    fn finalize_command_history(&mut self, undo_len: usize, command: EditorCommand) {
        if self.undo_stack.len() <= undo_len {
            if let Some(entry) = self.undo_stack.last_mut() {
                if entry.command == command {
                    entry.after = Some(self.state.document.clone());
                }
            }
            return;
        }
        let before = self.undo_stack[undo_len].before.clone();
        self.undo_stack.truncate(undo_len);
        self.undo_stack.push(HistoryEntry {
            command,
            before,
            after: Some(self.state.document.clone()),
        });
    }

    fn current_history_command(&self) -> EditorCommand {
        self.command_context
            .last()
            .cloned()
            .unwrap_or_else(|| EditorCommand::LegacyMutation {
                label: "unclassified-mutation".to_string(),
            })
    }

    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(HistoryEntry::new(
            self.current_history_command(),
            self.state.document.clone(),
        ));
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
        preferred_double_bond_side_for_segment(
            entry.fragment,
            &bond.begin,
            &bond.end,
            Some(&bond.id),
        )
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
        should_default_center_double_bond_for_segment(
            entry.fragment,
            &bond.begin,
            &bond.end,
            Some(&bond.id),
        )
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
                    let entry = self.state.document.editable_fragment()?;
                    preferred_double_bond_side_for_segment(entry.fragment, begin_id, end_id, None)
                        .unwrap_or(DoubleBondPlacement::Right)
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
        should_default_center_double_bond_for_segment(entry.fragment, begin_id, end_id, None)
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
}

fn collect_document_colors(document: &ChemcoreDocument) -> Vec<String> {
    let mut colors = Vec::new();
    push_normalized_color(&document.document.page.background, &mut colors);
    let Ok(value) = serde_json::to_value(document) else {
        return colors;
    };
    visit_document_colors(&value, false, &mut colors);
    colors
}

fn visit_document_colors(value: &JsonValue, accepts_string: bool, colors: &mut Vec<String>) {
    match value {
        JsonValue::String(raw) if accepts_string => push_normalized_color(raw, colors),
        JsonValue::Array(items) => {
            for item in items {
                visit_document_colors(item, accepts_string, colors);
            }
        }
        JsonValue::Object(map) => {
            for (key, child) in map {
                let color_key = key_contains_color(key);
                visit_document_colors(child, accepts_string || color_key, colors);
            }
        }
        _ => {}
    }
}

fn key_contains_color(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("color")
        || key.contains("fill")
        || key.contains("stroke")
        || key.contains("background")
}

fn push_normalized_color(raw: &str, colors: &mut Vec<String>) {
    let Some(color) = normalize_document_color(raw) else {
        return;
    };
    if !colors.iter().any(|existing| existing == &color) {
        colors.push(color);
    }
}

fn normalize_document_color(raw: &str) -> Option<String> {
    let raw = raw.trim().to_ascii_lowercase();
    if raw == "none" || raw.is_empty() {
        return None;
    }
    if raw.len() == 7
        && raw.starts_with('#')
        && raw[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Some(raw);
    }
    if raw.len() == 4
        && raw.starts_with('#')
        && raw[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        let mut expanded = String::from("#");
        for character in raw[1..].chars() {
            expanded.push(character);
            expanded.push(character);
        }
        return Some(expanded);
    }
    let inner = raw.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut values = [0u8; 3];
    let mut count = 0usize;
    for part in inner.split(',') {
        if count >= values.len() {
            return None;
        }
        values[count] = part.trim().parse::<u8>().ok()?;
        count += 1;
    }
    if count != values.len() {
        return None;
    }
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        values[0], values[1], values[2]
    ))
}
