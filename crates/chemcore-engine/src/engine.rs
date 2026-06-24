mod arrows;
mod bond_styles;
mod bond_tools;
mod brackets;
mod clipboard;
mod command;
mod context_menu;
mod context_styles;
mod delete;
mod groups;
mod links;
mod orbitals;
mod palettes;
mod presets;
mod select;
mod selection_summary;
mod shapes;
mod templates;
mod text_edit;

pub use self::command::{
    CommandAnchor, CommandResult, CommandTargetDelta, CommandTargets, EditorCommand,
    FocusedDeleteSource, HistoryEntry, ObjectSettingsPatch, TextEditCommandTarget,
};
use self::text_edit::{
    element_symbol_info, endpoint_label_world_bounds, implicit_hydrogen_label_text_for_count,
    make_periodic_element_node_label, mark_shortcut_implicit_hydrogen_label,
    refresh_element_valence_recognition_for_all_nodes, standalone_element_hydrogen_count,
};
pub(crate) use self::text_edit::{
    formula_hydrogen_count_for_node, refresh_attached_node_label_geometry_for_all_nodes,
    refresh_attached_node_label_geometry_for_node,
};
pub use self::text_edit::{
    TextEditLayout, TextEditLayoutCaret, TextEditLayoutCaretOffset, TextEditLayoutLine,
    TextEditLayoutRect, TextEditSelection, TextEditSelectionState, TextEditSession, TextEditTarget,
};

use self::arrows::ensure_arrow_style;
pub(crate) use self::bond_styles::automatic_double_bond_placement_for_segment;
use self::bond_styles::{
    apply_double_tool_center_style, apply_single_tool_center_style, centered_oriented_rect_points,
    cycle_bold_bond_center_style, cycle_dashed_bond_center_style,
    cycle_dashed_double_bond_tool_center_style, preferred_double_bond_side_for_segment,
    replace_with_bold_dashed_bond_style, replace_with_plain_triple_bond_style,
    replace_with_plain_wavy_bond_style, replace_with_stereo_bond_style,
    should_default_center_double_bond_for_segment,
    update_terminal_double_bond_placement_after_new_attachment,
};
use self::delete::FocusedDeleteMode;
use self::presets::{
    editor_options_from_document, editor_options_from_imported_cdxml_document,
    SelectedObjectSettings,
};
use crate::{
    adjacent_directions, anchor_from_point, angle_between, bond_center_focus_length, can_draw_bond,
    can_focus_bond_center, can_focus_endpoint, default_angle_for_anchor_for_variant,
    direction_from_angle, endpoint_from_angle_for_document, hit_test_arrow_center,
    hit_test_bond_center, hit_test_endpoint, hit_test_endpoint_excluding, largest_angular_gap,
    nearest_angle, normalize_angle, px_to_pt, refresh_repeating_units, render_document,
    render_document_targets, render_primitives_bounds, round2, snapped_angle_for_anchor,
    ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, Bond, BondAnchor,
    BondLinePattern, BondLineStyles, BondLineWeight, BondLineWeights, BondPreview, BondStereo,
    BondVariant, ChemcoreDocument, DoubleBond, DoubleBondPlacement, DragState, EditableFragment,
    EditableFragmentMut, EditorOptions, EndpointHit, HoverShape, HoverTextBox, OrbitalPhase,
    OrbitalStyle, OrbitalTemplate, OverlayState, Point, PointerEvent, RenderPrimitive,
    RenderRole, SceneObject, SelectionState, ShapeKind, ShapeStyle, Tool, ToolState,
    BOND_CENTER_FOCUS_WIDTH,
    BOND_CENTER_HIT_RADIUS, DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS, ENDPOINT_HIT_RADIUS,
    GLOBAL_SNAP_ANGLES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};

const HOVER_STROKE_WIDTH: f64 = crate::px_to_pt(1.1);
const HOVER_LABEL_STROKE_WIDTH: f64 = crate::px_to_pt(1.1);
const HOVER_ENDPOINT_STROKE_WIDTH: f64 = crate::px_to_pt(1.4);
const HOVER_BOND_CENTER_STROKE_WIDTH: f64 = crate::px_to_pt(1.2);
const PREVIEW_END_RADIUS: f64 = crate::px_to_pt(5.0);
const PREVIEW_END_STROKE_WIDTH: f64 = crate::px_to_pt(1.2);
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
                && role != RenderRole::DocumentDiagnostic
                && !render_role_is_selection(role)
                && !render_role_is_hover(role)
                && !render_role_is_preview(role)
        }
        RenderBoundsScope::Selection => {
            render_role_is_selection_bounds(render_primitive_role(primitive))
        }
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
    render_role_is_selection_bounds(role)
        || matches!(
            role,
            RenderRole::SelectionCenterCross
                | RenderRole::SelectionResizeHandle
                | RenderRole::SelectionRotateGlyph
                | RenderRole::SelectionRotateHandle
                | RenderRole::SelectionRotateStem
        )
}

fn render_role_is_selection_bounds(role: RenderRole) -> bool {
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
            | RenderRole::HoverShapeHandle
            | RenderRole::HoverTextBox
    )
}

fn render_role_is_preview(role: RenderRole) -> bool {
    matches!(role, RenderRole::PreviewBond | RenderRole::PreviewEnd)
}

fn render_primitive_role_mut(primitive: &mut RenderPrimitive) -> &mut RenderRole {
    match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. } => role,
    }
}

fn preview_primitive_ids(
    primitive: &RenderPrimitive,
) -> (Option<&str>, Option<&str>, Option<&str>) {
    match primitive {
        RenderPrimitive::Line {
            object_id, bond_id, ..
        }
        | RenderPrimitive::Polyline {
            object_id, bond_id, ..
        }
        | RenderPrimitive::Path {
            object_id, bond_id, ..
        } => (object_id.as_deref(), None, bond_id.as_deref()),
        RenderPrimitive::Circle {
            object_id, node_id, ..
        }
        | RenderPrimitive::Rect {
            object_id, node_id, ..
        }
        | RenderPrimitive::Text {
            object_id, node_id, ..
        } => (object_id.as_deref(), node_id.as_deref(), None),
        RenderPrimitive::Polygon {
            object_id,
            node_id,
            bond_id,
            ..
        }
        | RenderPrimitive::FilledPath {
            object_id,
            node_id,
            bond_id,
            ..
        } => (object_id.as_deref(), node_id.as_deref(), bond_id.as_deref()),
        RenderPrimitive::Ellipse { object_id, .. } => (object_id.as_deref(), None, None),
    }
}

fn is_preview_id(id: Option<&str>) -> bool {
    id.is_some_and(|id| id.starts_with("__preview_"))
}

fn mark_preview_primitives(primitives: &mut [RenderPrimitive]) {
    for primitive in primitives {
        let role = render_primitive_role(primitive);
        if render_role_is_preview(role) {
            continue;
        }
        let (object_id, node_id, bond_id) = preview_primitive_ids(primitive);
        if is_preview_id(bond_id) {
            *render_primitive_role_mut(primitive) = RenderRole::PreviewBond;
        } else if is_preview_id(object_id) || is_preview_id(node_id) {
            *render_primitive_role_mut(primitive) = RenderRole::PreviewBond;
        }
    }
}

fn connected_component_node_ids_for_fragment(
    fragment: &crate::MoleculeFragment,
    start_node_id: &str,
) -> Vec<String> {
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue = std::collections::VecDeque::new();
    visited.insert(start_node_id.to_string());
    queue.push_back(start_node_id.to_string());
    while let Some(current) = queue.pop_front() {
        for bond in &fragment.bonds {
            let neighbor = if bond.begin == current {
                Some(bond.end.as_str())
            } else if bond.end == current {
                Some(bond.begin.as_str())
            } else {
                None
            };
            let Some(neighbor) = neighbor else {
                continue;
            };
            if visited.insert(neighbor.to_string()) {
                queue.push_back(neighbor.to_string());
            }
        }
    }
    visited.into_iter().collect()
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
    tlc_spot_drag: Option<TlcSpotDragState>,
    orbital_drag: Option<OrbitalDragState>,
    selection_drag: Option<select::SelectionMoveDrag>,
    selection_rotate_drag: Option<select::SelectionRotateDrag>,
    selection_resize_drag: Option<select::SelectionResizeDrag>,
    template_drag: Option<templates::TemplateDrag>,
    shape_drag: Option<ShapeDragState>,
    shape_edit_drag: Option<ShapeEditDragState>,
    bracket_drag: Option<BracketDragState>,
    pending_select_target: Option<PendingSelectTarget>,
    pointer_bond_target: Option<String>,
    clipboard: Option<clipboard::ClipboardContent>,
    options: EditorOptions,
    document_style_preset: String,
    next_id: u64,
    revision: u64,
    last_command_result: Option<CommandResult>,
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
    HeadStyle,
    TailStyle,
}

#[derive(Debug, Clone)]
struct ArrowEditDragState {
    object_id: String,
    mode: ArrowEditMode,
    original_points: Vec<Point>,
    start_pointer: Point,
    has_dragged: bool,
    changed: bool,
    current_degrees: f64,
    undo_pushed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlcSpotHit {
    pub object_id: String,
    pub lane_index: usize,
    pub spot_index: usize,
    pub rf: f64,
    pub center: Point,
    pub guide_points: Vec<Point>,
}

#[derive(Debug, Clone)]
struct TlcSpotDragState {
    hit: TlcSpotHit,
    initial_rf: f64,
    changed: bool,
    undo_pushed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShapeDragState {
    pointer_start: Point,
    start: Point,
    current: Point,
    anchor: ShapeDrawAnchor,
    has_dragged: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShapeDrawAnchor {
    kind: ShapeDrawAnchorKind,
    point: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ShapeDrawAnchorKind {
    Free,
    Endpoint,
    Label,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrbitalDragState {
    anchor: Point,
    current: Point,
    has_dragged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ShapeEditHandle {
    CircleRadius,
    EllipseMajorPositive,
    EllipseMajorNegative,
    EllipseMinorPositive,
    EllipseMinorNegative,
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Debug, Clone)]
struct ShapeEditDragState {
    object_id: String,
    handle: ShapeEditHandle,
    original_object: SceneObject,
    start_pointer: Point,
    has_dragged: bool,
    undo_pushed: bool,
    changed: bool,
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

#[derive(Debug, Clone)]
enum PendingSelectTarget {
    GraphicObject(String),
    SceneObjects {
        arrow_objects: Vec<String>,
        text_objects: Vec<String>,
    },
    TextObject(String),
    MoleculeNode(String),
    MoleculeBond(String),
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
            tlc_spot_drag: None,
            orbital_drag: None,
            selection_drag: None,
            selection_rotate_drag: None,
            selection_resize_drag: None,
            template_drag: None,
            shape_drag: None,
            shape_edit_drag: None,
            bracket_drag: None,
            pending_select_target: None,
            pointer_bond_target: None,
            clipboard: None,
            options: EditorOptions::default(),
            document_style_preset: DEFAULT_DOCUMENT_STYLE_PRESET.to_string(),
            next_id: 1,
            revision: 0,
            last_command_result: None,
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

    pub fn document_cdx(&self) -> Result<Vec<u8>, String> {
        crate::document_to_cdx(&self.state.document)
    }

    pub fn document_sdf(&self) -> Result<String, String> {
        crate::document_to_sdf(&self.state.document)
    }

    pub fn document_svg(&self) -> String {
        crate::document_to_svg(&self.state.document)
    }

    pub fn document_colors(&self) -> Vec<String> {
        collect_document_colors(&self.state.document)
    }

    pub fn render_bounds(&self, scope: RenderBoundsScope) -> Option<[f64; 4]> {
        if scope == RenderBoundsScope::Selection {
            return self.selection_bounds();
        }
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
        let options = editor_options_from_document(&document);
        self.state.document = document;
        self.options = options;
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
        self.revision = 0;
        self.last_command_result = None;
        self.next_id = self.infer_next_id();
        Ok(())
    }

    pub fn load_cdxml_document(&mut self, cdxml: &str) -> Result<(), String> {
        let mut document = crate::parse_cdxml_document(cdxml, None)?;
        crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
        self.load_imported_document(document)
    }

    pub fn load_cdx_document(&mut self, cdx: &[u8]) -> Result<(), String> {
        let mut document = crate::parse_cdx_document(cdx, None)?;
        crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
        self.load_imported_document(document)
    }

    pub fn load_sdf_document(&mut self, sdf: &str) -> Result<(), String> {
        let document = crate::parse_sdf_document(sdf, None)?;
        self.load_imported_document(document)
    }

    fn load_imported_document(&mut self, mut document: ChemcoreDocument) -> Result<(), String> {
        refresh_repeating_units(&mut document);
        self.state.document = document;
        self.next_id = self.infer_next_id();
        self.link_imported_repeat_unit_labels_untracked();
        refresh_repeating_units(&mut self.state.document);
        let options = editor_options_from_imported_cdxml_document(&self.state.document);
        self.options = options;
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
        self.revision = 0;
        self.last_command_result = None;
        self.next_id = self.infer_next_id();
        Ok(())
    }

    pub fn tlc_spot_hit_test(&self, point: Point) -> Option<TlcSpotHit> {
        let mut best: Option<(f64, TlcSpotHit)> = None;
        for object in self.state.document.scene_objects() {
            let Some(geometry) = tlc_plate_geometry(object) else {
                continue;
            };
            for (lane_index, lane_x) in geometry.lane_centers.iter().enumerate() {
                let Some(spots) = geometry.spots.get(lane_index) else {
                    continue;
                };
                for (spot_index, rf) in spots.iter().enumerate() {
                    let local_center = Point::new(
                        *lane_x,
                        geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * *rf,
                    );
                    let center = rotate_point(local_center, geometry.center, geometry.rotate);
                    let distance = center.distance(point);
                    if distance > geometry.spot_radius + px_to_pt(6.0) {
                        continue;
                    }
                    let hit = TlcSpotHit {
                        object_id: object.id.clone(),
                        lane_index,
                        spot_index,
                        rf: round2(*rf),
                        center,
                        guide_points: tlc_lane_guide_points(&geometry, lane_index),
                    };
                    match &best {
                        Some((best_distance, _)) if *best_distance <= distance => {}
                        _ => best = Some((distance, hit)),
                    }
                }
            }
        }
        best.map(|(_, hit)| hit)
    }

    pub fn begin_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let hit = self.tlc_spot_hit_test(point)?;
        self.tlc_spot_drag = Some(TlcSpotDragState {
            initial_rf: hit.rf,
            hit: hit.clone(),
            changed: false,
            undo_pushed: false,
        });
        Some(hit)
    }

    pub fn update_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let command = self.tlc_spot_drag_command()?;
        let mut next = None;
        self.with_transient_command(command, |engine| {
            next = engine.update_tlc_spot_drag_untracked(point);
            next.is_some()
        });
        next
    }

    fn update_tlc_spot_drag_untracked(&mut self, point: Point) -> Option<TlcSpotHit> {
        let drag = self.tlc_spot_drag.clone()?;
        let next_rf = self.tlc_spot_rf_at_point(&drag.hit.object_id, drag.hit.lane_index, point)?;
        let changed = (drag.hit.rf - next_rf).abs() > 0.0001;
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
        }
        let next = self.update_tlc_spot_to_point(
            &drag.hit.object_id,
            drag.hit.lane_index,
            drag.hit.spot_index,
            point,
        )?;
        if let Some(active_drag) = &mut self.tlc_spot_drag {
            active_drag.changed |= changed;
            active_drag.undo_pushed |= changed;
            active_drag.hit = next.clone();
        }
        Some(next)
    }

    pub fn finish_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let had_drag = self.tlc_spot_drag.is_some();
        let next = if had_drag {
            self.update_tlc_spot_drag(point)
        } else {
            None
        };
        let changed = self.tlc_spot_drag.as_ref().is_some_and(|drag| drag.changed);
        let undo_pushed = self
            .tlc_spot_drag
            .as_ref()
            .is_some_and(|drag| drag.undo_pushed);
        self.tlc_spot_drag = None;
        if had_drag && undo_pushed && !changed {
            self.undo_stack.pop();
        }
        next
    }

    fn tlc_spot_drag_command(&self) -> Option<EditorCommand> {
        let drag = self.tlc_spot_drag.as_ref()?;
        Some(EditorCommand::MoveTlcSpot {
            object_id: drag.hit.object_id.clone(),
            lane_index: drag.hit.lane_index,
            spot_index: drag.hit.spot_index,
            before_rf: drag.initial_rf,
        })
    }

    pub fn tlc_lane_guide_hit_test(&self, point: Point) -> Option<TlcSpotHit> {
        if self.tlc_spot_hit_test(point).is_some() {
            return None;
        }
        for object in self.state.document.scene_objects() {
            let Some(geometry) = tlc_plate_geometry(object) else {
                continue;
            };
            for (lane_index, spots) in geometry.spots.iter().enumerate() {
                let guide_points = tlc_lane_guide_points(&geometry, lane_index);
                if !point_in_polygon(point, &guide_points) {
                    continue;
                }
                let rf = spots.first().copied().unwrap_or(0.15);
                let lane_x = *geometry.lane_centers.get(lane_index)?;
                let local_center = Point::new(
                    lane_x,
                    geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * rf,
                );
                return Some(TlcSpotHit {
                    object_id: object.id.clone(),
                    lane_index,
                    spot_index: 0,
                    rf: round2(rf),
                    center: rotate_point(local_center, geometry.center, geometry.rotate),
                    guide_points,
                });
            }
        }
        None
    }

    fn push_interaction_render_primitives(&self, out: &mut Vec<RenderPrimitive>) {
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
                        radius: crate::px_to_pt(1.5),
                        fill: "#ffffff".to_string(),
                        stroke: "rgba(47,111,237,0.82)".to_string(),
                        stroke_width: HOVER_STROKE_WIDTH,
                    });
                }
            }
        }
        if let Some(hover) = &self.state.overlay.hover_shape {
            for handle in &hover.handles {
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::HoverShapeHandle,
                    object_id: Some(hover.object_id.clone()),
                    node_id: None,
                    center: *handle,
                    radius: crate::px_to_pt(2.0),
                    fill: "#ffffff".to_string(),
                    stroke: "rgba(47,111,237,0.82)".to_string(),
                    stroke_width: HOVER_STROKE_WIDTH,
                });
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
                fill: "rgba(47,111,237,0.16)".to_string(),
                stroke: "rgba(47,111,237,0.86)".to_string(),
                stroke_width: PREVIEW_END_STROKE_WIDTH,
            });
        }
        if let Some((point, count)) = self.template_chain_count_label() {
            out.push(RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                object_id: Some("__preview_chain_count".to_string()),
                node_id: None,
                x: point.x,
                y: point.y,
                baseline_offset: None,
                dominant_baseline: Some("central".to_string()),
                text: count.to_string(),
                font_size: 8.0,
                font_family: Some("Arial".to_string()),
                fill: Some("#000000".to_string()),
                text_anchor: Some("middle".to_string()),
                line_height: None,
                preserve_lines: false,
                box_width: None,
                runs: Vec::new(),
                rotate: 0.0,
                rotate_center: None,
            });
        }
    }

    pub fn interaction_render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = if let Some(preview_document) = self.preview_overlay_document() {
            let mut primitives = render_document(&preview_document);
            mark_preview_primitives(&mut primitives);
            primitives.retain(|primitive| render_role_is_preview(render_primitive_role(primitive)));
            primitives
        } else if self.arrow_edit_drag.is_some() || self.shape_edit_drag.is_some() {
            render_document(&self.state.document)
        } else {
            Vec::new()
        };
        out.extend(self.selection_render_list());
        self.push_interaction_render_primitives(&mut out);
        out
    }

    pub fn render_list(&self) -> Vec<RenderPrimitive> {
        let mut out = if let Some(preview_document) = self.preview_document() {
            render_document(&preview_document)
        } else {
            render_document(&self.state.document)
        };
        out.extend(self.selection_render_list());
        self.push_interaction_render_primitives(&mut out);
        out
    }

    pub fn render_targets(
        &self,
        node_ids: &BTreeSet<String>,
        bond_ids: &BTreeSet<String>,
        object_ids: &BTreeSet<String>,
    ) -> Vec<RenderPrimitive> {
        let document = self
            .preview_document()
            .unwrap_or_else(|| self.state.document.clone());
        render_document_targets(&document, node_ids, bond_ids, object_ids)
    }

    pub fn set_tool_state(&mut self, tool: ToolState) {
        let previous_tool = self.state.tool.active_tool;
        let next_tool = tool.active_tool;
        let changed_tool = previous_tool != next_tool;
        self.state.tool = tool;
        if changed_tool && previous_tool == Tool::Select && next_tool != Tool::Select {
            self.state.selection = SelectionState::default();
        }
        self.clear_interaction();
        if changed_tool && next_tool == Tool::Select {
            self.select_pending_target_for_select_tool();
        } else if changed_tool {
            self.pending_select_target = None;
        }
    }

    fn clear_overlay(&mut self) {
        // Overlay state is transient UI feedback. Clearing it without touching
        // selection or history prevents hover/focus from sticking after commits.
        self.state.overlay = OverlayState::default();
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
        if self.state.tool.active_tool == Tool::Orbital {
            self.pointer_move_orbital(event);
            return;
        }
        if matches!(self.state.tool.active_tool, Tool::Shape | Tool::TlcPlate) {
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
        if self.state.tool.active_tool == Tool::Element {
            self.pointer_move_element(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
            self.state.overlay.hover_shape = None;
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
            self.state.overlay.hover_shape = None;
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
                        object_id: Some(target.object_id.clone()),
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
                        self.options.bond_length_world_pt().value(),
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
        self.state.overlay.hover_shape = None;
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

    fn pointer_move_element(&mut self, event: PointerEvent) {
        let point = event.point();
        self.drag = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
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
        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS);
    }

    fn element_replacement_node_at_point(&self, point: Point) -> Option<String> {
        self.hit_test_endpoint_label_box(point)
            .map(|(node_id, _)| node_id)
            .or_else(|| {
                hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
                    .map(|hit| hit.node_id)
            })
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
        if self.state.tool.active_tool == Tool::Orbital {
            self.pointer_down_orbital(event);
            return;
        }
        if matches!(self.state.tool.active_tool, Tool::Shape | Tool::TlcPlate) {
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
        if self.state.tool.active_tool == Tool::Element {
            let point = event.point();
            self.state.selection = SelectionState::default();
            if let Some(node_id) = self.element_replacement_node_at_point(point) {
                let label = self.state.tool.element_symbol.clone();
                let target_node_id = node_id.clone();
                let target_label = label.clone();
                self.with_command(
                    EditorCommand::ReplaceNodeLabel { node_id, label },
                    |engine| engine.replace_node_label_untracked(&target_node_id, &target_label),
                );
                return;
            }
            self.clear_interaction();
            let symbol = self.state.tool.element_symbol.clone();
            let atomic_number = self.state.tool.element_atomic_number;
            self.with_command(
                EditorCommand::AddElement {
                    symbol,
                    atomic_number,
                    center: CommandAnchor::from(point),
                },
                |engine| engine.insert_periodic_element(point),
            );
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
            self.clear_overlay();
            self.drag = Some(DragState {
                anchor: BondAnchor {
                    node_id: Some(endpoint.node_id),
                    object_id: Some(endpoint.object_id),
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
        self.clear_overlay();
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
        if self.state.tool.active_tool == Tool::Orbital {
            self.pointer_up_orbital(event);
            return;
        }
        if matches!(self.state.tool.active_tool, Tool::Shape | Tool::TlcPlate) {
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
                    object_id: drag.anchor.object_id.clone(),
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
                        self.options.bond_length_world_pt().value(),
                    )
                });
                BondAnchor {
                    node_id: None,
                    object_id: drag.anchor.object_id.clone(),
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
                self.options.bond_length_world_pt().value(),
            );
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    object_id: drag.anchor.object_id.clone(),
                    point: end,
                    label_anchor: None,
                })
        };
        self.state.overlay.preview = None;
        let _ = self.add_bond_between(drag.anchor, end_anchor, self.pending_bond_order());
        // Do not synthesize a hover target at the committed endpoint; the next
        // pointer move should be the source of truth for focus feedback.
        self.clear_overlay();
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.arrow_drag = None;
        self.arrow_edit_drag = None;
        self.tlc_spot_drag = None;
        self.orbital_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        self.selection_resize_drag = None;
        self.template_drag = None;
        self.shape_drag = None;
        self.shape_edit_drag = None;
        self.bracket_drag = None;
        self.pointer_bond_target = None;
        self.state.overlay = OverlayState::default();
    }

    fn note_pending_select_target(&mut self, target: PendingSelectTarget) {
        self.pending_select_target = Some(target);
    }

    pub fn pending_graphic_object_id(&self) -> Option<&str> {
        match self.pending_select_target.as_ref() {
            Some(PendingSelectTarget::GraphicObject(object_id)) => Some(object_id.as_str()),
            _ => None,
        }
    }

    fn select_pending_target_for_select_tool(&mut self) {
        let Some(target) = self.pending_select_target.take() else {
            return;
        };
        let Some(selection) = self.selection_for_pending_target(&target) else {
            return;
        };
        self.state.selection = selection;
    }

    fn selection_for_pending_target(&self, target: &PendingSelectTarget) -> Option<SelectionState> {
        match target {
            PendingSelectTarget::GraphicObject(object_id) => self
                .state
                .document
                .objects
                .iter()
                .any(|object| object.id == *object_id && object.object_type != "text")
                .then(|| SelectionState {
                    arrow_objects: vec![object_id.clone()],
                    ..SelectionState::default()
                }),
            PendingSelectTarget::SceneObjects {
                arrow_objects,
                text_objects,
            } => {
                let mut selection = SelectionState::default();
                for object_id in arrow_objects {
                    if self
                        .state
                        .document
                        .find_scene_object(object_id)
                        .is_some_and(|object| object.object_type != "text")
                    {
                        selection.arrow_objects.push(object_id.clone());
                    }
                }
                for object_id in text_objects {
                    if self
                        .state
                        .document
                        .find_scene_object(object_id)
                        .is_some_and(|object| object.object_type == "text")
                    {
                        selection.text_objects.push(object_id.clone());
                    }
                }
                (!selection.is_empty()).then_some(selection)
            }
            PendingSelectTarget::TextObject(object_id) => self
                .state
                .document
                .objects
                .iter()
                .any(|object| object.id == *object_id && object.object_type == "text")
                .then(|| SelectionState {
                    text_objects: vec![object_id.clone()],
                    ..SelectionState::default()
                }),
            PendingSelectTarget::MoleculeNode(node_id) => {
                self.selection_for_molecule_component_containing_node(node_id)
            }
            PendingSelectTarget::MoleculeBond(bond_id) => {
                self.selection_for_molecule_component_containing_bond(bond_id)
            }
        }
    }

    fn selection_for_molecule_component_containing_bond(
        &self,
        bond_id: &str,
    ) -> Option<SelectionState> {
        let entry = self.state.document.editable_fragment()?;
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)?;
        self.selection_for_molecule_component_containing_node(&bond.begin)
    }

    fn selection_for_molecule_component_containing_node(
        &self,
        node_id: &str,
    ) -> Option<SelectionState> {
        let entry = self.state.document.editable_fragment()?;
        if !entry.fragment.nodes.iter().any(|node| node.id == node_id) {
            return None;
        }
        let nodes = connected_component_node_ids_for_fragment(entry.fragment, node_id);
        let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
        let bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                node_set.contains(bond.begin.as_str()) && node_set.contains(bond.end.as_str())
            })
            .map(|bond| bond.id.clone())
            .collect();
        Some(SelectionState {
            nodes,
            bonds,
            ..SelectionState::default()
        })
    }

    pub fn add_single_bond(&mut self, anchor: BondAnchor, end: Point) {
        self.add_bond_between(
            anchor.clone(),
            BondAnchor {
                node_id: None,
                object_id: anchor.object_id.clone(),
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
        if anchor
            .object_id
            .as_ref()
            .zip(end.object_id.as_ref())
            .is_some_and(|(left, right)| left != right)
        {
            return false;
        }
        let target_anchor = if anchor.node_id.is_some() || anchor.object_id.is_some() {
            &anchor
        } else {
            &end
        };
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists_for_anchor(target_anchor, begin_id, end_id) {
                return false;
            }
        }
        self.push_undo_snapshot();
        self.state.selection = SelectionState::default();
        let begin_id = match &anchor.node_id {
            Some(node_id) => node_id.clone(),
            None => self.insert_carbon_for_anchor(target_anchor, anchor.point),
        };
        let end_id = match &end.node_id {
            Some(node_id) => node_id.clone(),
            None => self.insert_carbon_for_anchor(target_anchor, end.point),
        };
        if begin_id == end_id || self.bond_exists_for_anchor(target_anchor, &begin_id, &end_id) {
            self.undo_stack.pop();
            return false;
        }
        let bond_id = self.next_id("b");
        let pending_line_styles = self.pending_line_styles();
        let pending_line_weights = self.pending_line_weights();
        let pending_stereo = self.pending_bond_stereo();
        let pending_double = self.pending_double_state_for_new_bond_in_anchor_fragment(
            target_anchor,
            &begin_id,
            &end_id,
            order.max(1),
        );
        let stroke_width = self.options.bond_stroke_world_pt().value();
        let bold_width = self.options.bold_bond_width_world_pt().value();
        let wedge_width = self.options.wedge_width_world_pt().value();
        let label_clip_margin = self.options.label_clip_margin_world_pt().value();
        let hash_spacing = self.options.hash_spacing_world_pt().value();
        let bond_spacing = self.options.bond_spacing_percent();
        let margin_width = self.options.margin_width_world_pt().value();
        let mut entry = self
            .editable_fragment_mut_for_anchor(target_anchor)
            .expect("blank document always has an editable fragment");
        entry.fragment.bonds.push(Bond {
            id: bond_id.clone(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: pending_double,
            stereo: pending_stereo,
            stroke_width,
            stroke: None,
            bold_width: Some(bold_width),
            wedge_width: Some(wedge_width),
            label_clip_margin: Some(label_clip_margin),
            hash_spacing: Some(hash_spacing),
            bond_spacing: Some(bond_spacing),
            margin_width: Some(margin_width),
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
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            entry.object.transform.translate,
            &begin_id,
            stroke_width,
        );
        if end_id != begin_id {
            refresh_attached_node_label_geometry_for_node(
                entry.fragment,
                entry.object.transform.translate,
                &end_id,
                stroke_width,
            );
        }
        entry.update_bounds();
        self.note_pending_select_target(PendingSelectTarget::MoleculeBond(bond_id));
        true
    }

    fn preview_document(&self) -> Option<ChemcoreDocument> {
        if let Some(preview_document) = self.template_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.shape_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.orbital_preview_document() {
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
                object_id: drag.anchor.object_id.clone(),
                point: drag.preview_end?,
                label_anchor: None,
            }
        };
        self.document_with_preview_bond(&drag.anchor, &end_anchor, self.pending_bond_order())
    }

    fn preview_document_shell(&self) -> ChemcoreDocument {
        let mut document = self.state.document.clone();
        document.objects.clear();
        document.resources.clear();
        document
    }

    fn preview_overlay_document(&self) -> Option<ChemcoreDocument> {
        if let Some(preview_document) = self.template_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.shape_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.orbital_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.bracket_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(drag) = self.arrow_drag.as_ref().filter(|drag| drag.has_dragged) {
            let end = drag.end?;
            let mut document = self.preview_document_shell();
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
        self.preview_bond_overlay_document()
    }

    pub fn undo(&mut self) -> bool {
        let Some(mut entry) = self.undo_stack.pop() else {
            return false;
        };
        let before_revision = self.revision;
        let before_document = self.state.document.clone();
        let after = entry
            .after
            .clone()
            .unwrap_or_else(|| self.state.document.clone());
        self.restore_document(entry.before.clone());
        entry.after = Some(after);
        self.redo_stack.push(entry);
        self.commit_command_result(EditorCommand::Undo, before_revision, before_document);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_stack.pop() else {
            return false;
        };
        let Some(after) = entry.after.clone() else {
            return false;
        };
        let before_revision = self.revision;
        let before_document = self.state.document.clone();
        self.restore_document(after);
        self.undo_stack.push(entry);
        self.commit_command_result(EditorCommand::Redo, before_revision, before_document);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn last_command_result(&self) -> Option<&CommandResult> {
        self.last_command_result.as_ref()
    }

    pub fn last_command_result_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.last_command_result)
    }

    pub fn history_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.undo_stack)
    }

    pub fn execute_command_json(&mut self, command_json: &str) -> Result<String, String> {
        let command: EditorCommand =
            serde_json::from_str(command_json).map_err(|error| error.to_string())?;
        let result = self.execute_command(command)?;
        serde_json::to_string(&result).map_err(|error| error.to_string())
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> Result<CommandResult, String> {
        self.last_command_result = None;
        let changed = match command.clone() {
            EditorCommand::Undo => self.undo(),
            EditorCommand::Redo => self.redo(),
            EditorCommand::AddBond {
                begin,
                end,
                order,
                variant,
            } => {
                let previous_tool = self.state.tool.clone();
                self.state.tool.bond_variant = variant;
                let changed = self.add_bond_between(
                    bond_anchor_from_command(begin),
                    bond_anchor_from_command(end),
                    order,
                );
                self.state.tool = previous_tool;
                changed
            }
            EditorCommand::AddArrow {
                begin,
                end,
                variant,
                head_size,
                curve,
                head_style,
                tail_style,
                head,
                tail,
                bold,
                no_go,
            } => {
                let previous_tool = self.state.tool.clone();
                self.state.tool.arrow_variant = variant;
                self.state.tool.arrow_head_size = head_size;
                self.state.tool.arrow_curve = curve;
                self.state.tool.arrow_head_style = head_style;
                self.state.tool.arrow_tail_style = tail_style;
                self.state.tool.arrow_head = head;
                self.state.tool.arrow_tail = tail;
                self.state.tool.arrow_bold = bold;
                self.state.tool.arrow_no_go = no_go;
                let changed = self
                    .add_arrow_between(point_from_command(&begin), point_from_command(&end))
                    .is_some();
                self.state.tool = previous_tool;
                changed
            }
            EditorCommand::AddShape {
                kind,
                style,
                color,
                begin,
                end,
            } => self.with_command(command.clone(), |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.shape_kind = kind;
                engine.state.tool.shape_style = style;
                engine.state.tool.shape_color = color;
                let start = point_from_command(&begin);
                let current = point_from_command(&end);
                let drag = ShapeDragState {
                    pointer_start: start,
                    start,
                    current,
                    anchor: ShapeDrawAnchor {
                        kind: ShapeDrawAnchorKind::Free,
                        point: start,
                        bounds: None,
                    },
                    has_dragged: start.distance(current) > crate::EPSILON,
                };
                let changed = engine.insert_shape_from_drag(&drag);
                engine.state.tool = previous_tool;
                changed
            }),
            EditorCommand::AddBracket { kind, begin, end } => {
                self.with_command(command.clone(), |engine| {
                    let previous_tool = engine.state.tool.clone();
                    engine.state.tool.bracket_kind = kind;
                    let drag = BracketDragState {
                        start: point_from_command(&begin),
                        current: point_from_command(&end),
                        symbol_anchor: None,
                        has_dragged: true,
                    };
                    let changed = engine.insert_bracket_from_drag(&drag);
                    engine.state.tool = previous_tool;
                    changed
                })
            }
            EditorCommand::AddSymbol { kind, center } => {
                self.with_command(command.clone(), |engine| {
                    let previous_tool = engine.state.tool.clone();
                    engine.state.tool.symbol_kind = kind;
                    let changed = engine.insert_bracket_symbol(point_from_command(&center));
                    engine.state.tool = previous_tool;
                    changed
                })
            }
            EditorCommand::AddElement {
                symbol,
                atomic_number,
                center,
            } => self.with_command(command.clone(), |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.element_symbol = symbol;
                engine.state.tool.element_atomic_number = atomic_number;
                let changed = engine.insert_periodic_element(point_from_command(&center));
                engine.state.tool = previous_tool;
                changed
            }),
            EditorCommand::ReplaceNodeLabel { node_id, label } => self
                .with_command(command.clone(), |engine| {
                    engine.replace_node_label_untracked(&node_id, &label)
                }),
            EditorCommand::MoveTlcSpot { .. }
            | EditorCommand::MoveSelection
            | EditorCommand::RotateSelection
            | EditorCommand::ResizeSelection
            | EditorCommand::EditArrowGeometry { .. }
            | EditorCommand::EditShapeGeometry { .. }
            | EditorCommand::ApplyTextEdit { .. } => {
                return Err(format!(
                    "Command '{}' requires an active interaction context.",
                    editor_command_type_name(&command)
                ));
            }
            EditorCommand::AddOrbital {
                template,
                style,
                phase,
                color,
                center,
                end,
            } => self.with_command(command.clone(), |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.orbital_template = template;
                engine.state.tool.orbital_style = style;
                engine.state.tool.orbital_phase = phase;
                engine.state.tool.orbital_color = color;
                let drag = OrbitalDragState {
                    anchor: point_from_command(&center),
                    current: point_from_command(&end),
                    has_dragged: true,
                };
                let changed = engine.insert_orbital_from_drag(&drag);
                engine.state.tool = previous_tool;
                changed
            }),
            EditorCommand::ApplyArrowStyle {
                object_ids,
                variant,
                head_size,
                curve,
                head_style,
                tail_style,
                head,
                tail,
                bold,
                no_go,
            } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_arrow_options_to_selection(
                    variant, head_size, curve, head_style, tail_style, head, tail, bold, no_go,
                )
            }
            EditorCommand::CycleBondStyle { bond_id, variant } => {
                let previous_tool = self.state.tool.clone();
                self.state.tool.bond_variant = variant;
                let changed = self.cycle_bond_center_style(&bond_id);
                self.state.tool = previous_tool;
                changed
            }
            EditorCommand::DeleteSelection => self.delete_selection(),
            EditorCommand::DeleteFocusedAtPoint { x, y, source } => self.delete_focused_at_point(
                Point::new(x, y),
                match source {
                    FocusedDeleteSource::DeleteTool => FocusedDeleteMode::DeleteToolClick,
                    FocusedDeleteSource::CommandKey => FocusedDeleteMode::CommandKey,
                },
            ),
            EditorCommand::PasteClipboard => self.paste_clipboard(),
            EditorCommand::CutSelection => self.cut_selection(),
            EditorCommand::InsertTemplate { template, x, y } => {
                let previous_tool = self.state.tool.clone();
                let before_revision = self.revision;
                self.state.tool.template = template;
                let event = PointerEvent {
                    x,
                    y,
                    button: Some(0),
                    alt_key: false,
                };
                self.pointer_down_template(event.clone());
                self.pointer_up_template(event);
                self.state.tool = previous_tool;
                self.revision != before_revision
            }
            EditorCommand::ApplySelectionArrange { command } => {
                self.apply_selection_arrange_command(&command)
            }
            EditorCommand::ApplySelectionOrder {
                object_ids,
                command,
            } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_selection_order_command(&command)
            }
            EditorCommand::ApplySelectionColor { color } => self.apply_color_to_selection(&color),
            EditorCommand::ApplyShapeStyle { object_ids, style } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_shape_style_to_selection(&style)
            }
            EditorCommand::ApplyBracketKind { object_ids, kind } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_bracket_kind_to_selection(&kind)
            }
            EditorCommand::ApplyOrbitalTemplate {
                object_ids,
                template,
            } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_orbital_template_to_selection(&template)
            }
            EditorCommand::ApplyOrbitalStyle { object_ids, style } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_orbital_style_to_selection(&style)
            }
            EditorCommand::ApplyOrbitalPhase { object_ids, phase } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_orbital_phase_to_selection(&phase)
            }
            EditorCommand::ApplyLineStyle { object_ids, style } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_line_style_to_selection(&style)
            }
            EditorCommand::ApplyBondStyle { bond_ids, style } => self
                .with_command(command.clone(), |engine| {
                    engine.apply_bond_style_to_bond_ids_untracked(&bond_ids, &style)
                }),
            EditorCommand::ApplyTextStyle {
                text_object_ids,
                label_node_ids,
                node_ids,
                command,
                value,
            } => {
                self.state.selection = SelectionState {
                    text_objects: text_object_ids,
                    label_nodes: label_node_ids,
                    nodes: node_ids,
                    ..SelectionState::default()
                };
                self.apply_text_style_to_selection(&command, &value)
            }
            EditorCommand::SetChemicalCheckForSelection { enabled } => {
                self.set_chemical_check_for_selection(enabled)
            }
            EditorCommand::ExpandLabelsInSelection => self.expand_labels_in_selection(),
            EditorCommand::CenterSelectionOnPage => self.center_selection_on_page(),
            EditorCommand::GroupSelection { object_ids } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.group_selection()
            }
            EditorCommand::UngroupSelection { object_ids } => {
                self.state.selection = SelectionState {
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.ungroup_selection()
            }
            EditorCommand::LinkSelection { object_ids } => {
                self.state.selection =
                    scene_object_selection_from_ids(&self.state.document, &object_ids);
                self.link_selection()
            }
            EditorCommand::UnlinkSelection { object_ids } => {
                self.state.selection =
                    scene_object_selection_from_ids(&self.state.document, &object_ids);
                self.unlink_selection()
            }
            EditorCommand::JoinSelection => self.join_selection(),
            EditorCommand::ScaleSelection { percent } => self.scale_selection(percent),
            EditorCommand::ApplyObjectSettings { settings } => self.apply_object_settings(settings),
            EditorCommand::ApplyObjectSettingsToSelection {
                bond_ids,
                object_ids,
                settings,
            } => {
                self.state.selection = SelectionState {
                    bonds: bond_ids,
                    arrow_objects: object_ids,
                    ..SelectionState::default()
                };
                self.apply_object_settings_to_selection(SelectedObjectSettings {
                    bond_length: settings.bond_length,
                    line_width: settings.line_width,
                    bold_width: settings.bold_width,
                    bond_spacing: settings.bond_spacing,
                    margin_width: settings.margin_width,
                    hash_spacing: settings.hash_spacing,
                })
            }
            EditorCommand::ApplyDocumentStyle { preset } => self.set_document_style_preset(&preset),
            EditorCommand::ReplaceHoveredEndpointLabel { label } => {
                self.replace_hovered_endpoint_label(&label)
            }
        };
        if !changed && self.last_command_result.is_none() {
            self.last_command_result = Some(self.unchanged_command_result());
        }
        Ok(self
            .last_command_result
            .clone()
            .unwrap_or_else(|| self.unchanged_command_result()))
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
            object_id: Some(target.object_id),
            point: target.point,
            label_anchor: target.label_anchor,
        })
    }

    fn editable_fragment_for_anchor(&self, anchor: &BondAnchor) -> Option<EditableFragment<'_>> {
        if let Some(object_id) = anchor.object_id.as_deref() {
            if let Some(entry) = self
                .state
                .document
                .editable_fragments()
                .into_iter()
                .find(|entry| entry.object.id == object_id)
            {
                return Some(entry);
            }
        }
        if let Some(node_id) = anchor.node_id.as_deref() {
            if let Some(entry) = self
                .state
                .document
                .editable_fragments()
                .into_iter()
                .find(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
            {
                return Some(entry);
            }
        }
        self.state.document.editable_fragment()
    }

    fn editable_fragment_object_id_for_anchor(&self, anchor: &BondAnchor) -> Option<String> {
        self.editable_fragment_for_anchor(anchor)
            .map(|entry| entry.object.id.clone())
    }

    fn editable_fragment_mut_for_anchor(
        &mut self,
        anchor: &BondAnchor,
    ) -> Option<EditableFragmentMut<'_>> {
        let object_id = self.editable_fragment_object_id_for_anchor(anchor)?;
        if self.state.document.find_scene_object(&object_id).is_some() {
            self.state
                .document
                .editable_fragment_mut_for_object(&object_id)
        } else {
            self.state.document.editable_fragment_mut()
        }
    }

    fn bond_exists_for_anchor(&self, anchor: &BondAnchor, begin_id: &str, end_id: &str) -> bool {
        self.editable_fragment_for_anchor(anchor)
            .is_some_and(|entry| self.bond_exists_in_fragment(entry.fragment, begin_id, end_id))
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

    fn insert_carbon_for_anchor(&mut self, anchor: &BondAnchor, point: Point) -> String {
        let node_id = self.next_id("n");
        let entry = self
            .editable_fragment_mut_for_anchor(anchor)
            .expect("blank document always has an editable fragment");
        let local = entry.local_point(point);
        entry
            .fragment
            .nodes
            .push(crate::Node::carbon(node_id.clone(), local));
        node_id
    }

    fn insert_periodic_element(&mut self, point: Point) -> bool {
        let Some((element, atomic_number)) = element_symbol_info(&self.state.tool.element_symbol)
        else {
            return false;
        };
        self.push_undo_snapshot();
        let node_id = self.next_id("n");
        let entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        let local = entry.local_point(point);
        let num_hydrogens = standalone_element_hydrogen_count(atomic_number);
        let label_text = implicit_hydrogen_label_text_for_count(element, num_hydrogens);
        let label = if element == "C" && num_hydrogens == 0 {
            None
        } else {
            Some(make_periodic_element_node_label(
                &label_text,
                [local.x, local.y],
            ))
        };
        let mut node = crate::Node {
            id: node_id.clone(),
            element: element.to_string(),
            atomic_number,
            position: [round2(local.x), round2(local.y)],
            charge: 0,
            num_hydrogens,
            is_external_connection_point: false,
            is_placeholder: false,
            label,
            meta: serde_json::Value::Null,
        };
        mark_shortcut_implicit_hydrogen_label(&mut node, &label_text);
        entry.fragment.nodes.push(node);
        self.state.selection = SelectionState::default();
        true
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
        let before_revision = self.revision;
        let before_document = self.state.document.clone();
        let before_redo_stack = self.redo_stack.clone();
        let undo_len = self.undo_stack.len();
        self.command_context.push(command.clone());
        let applied = apply(self);
        self.command_context.pop();
        let command_before_document = self
            .history_before_document_for_command(undo_len, &command)
            .unwrap_or_else(|| before_document.clone());
        let document_changed =
            !documents_equivalent(&command_before_document, &self.state.document);
        if applied && document_changed {
            refresh_repeating_units(&mut self.state.document);
            self.finalize_command_history(undo_len, command.clone());
            self.commit_command_result(command, before_revision, command_before_document);
            true
        } else {
            self.cleanup_unchanged_command_history(undo_len, &command, before_redo_stack);
            self.last_command_result = Some(self.unchanged_command_result());
            false
        }
    }

    fn with_transient_command<F>(&mut self, command: EditorCommand, apply: F) -> bool
    where
        F: FnOnce(&mut Self) -> bool,
    {
        if !self.command_context.is_empty() {
            return apply(self);
        }
        self.command_context.push(command);
        let changed = apply(self);
        self.command_context.pop();
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

    fn history_before_document_for_command(
        &self,
        undo_len: usize,
        command: &EditorCommand,
    ) -> Option<ChemcoreDocument> {
        if self.undo_stack.len() > undo_len {
            return self
                .undo_stack
                .get(undo_len)
                .map(|entry| entry.before.clone());
        }
        self.undo_stack
            .iter()
            .rev()
            .find(|entry| entry.command == *command && entry.after.is_none())
            .map(|entry| entry.before.clone())
    }

    fn cleanup_unchanged_command_history(
        &mut self,
        undo_len: usize,
        command: &EditorCommand,
        before_redo_stack: Vec<HistoryEntry>,
    ) {
        if self.undo_stack.len() > undo_len {
            self.undo_stack.truncate(undo_len);
            self.redo_stack = before_redo_stack;
            return;
        }
        if self
            .undo_stack
            .last()
            .is_some_and(|entry| entry.command == *command && entry.after.is_none())
        {
            self.undo_stack.pop();
        }
    }

    fn commit_command_result(
        &mut self,
        command: EditorCommand,
        before_revision: u64,
        before_document: ChemcoreDocument,
    ) {
        self.revision = self.revision.saturating_add(1);
        self.last_command_result = Some(self.command_result_from_diff(
            Some(command),
            before_revision,
            &before_document,
            &self.state.document,
        ));
    }

    fn command_result_from_diff(
        &self,
        command: Option<EditorCommand>,
        before_revision: u64,
        before_document: &ChemcoreDocument,
        after_document: &ChemcoreDocument,
    ) -> CommandResult {
        let delta = document_target_delta(before_document, after_document);
        CommandResult {
            changed: !delta.created.is_empty()
                || !delta.updated.is_empty()
                || !delta.deleted.is_empty(),
            revision: self.revision,
            before_revision,
            command,
            targets: command_targets_union(&delta),
            created: delta.created,
            updated: delta.updated,
            deleted: delta.deleted,
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            undo_depth: self.undo_stack.len(),
            redo_depth: self.redo_stack.len(),
            diagnostics: BTreeMap::new(),
        }
    }

    fn unchanged_command_result(&self) -> CommandResult {
        CommandResult::unchanged(
            self.revision,
            self.can_undo(),
            self.can_redo(),
            self.undo_stack.len(),
            self.redo_stack.len(),
        )
    }

    fn current_history_command(&self) -> EditorCommand {
        self.command_context
            .last()
            .cloned()
            .expect("document mutation must run inside Engine::with_command")
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
        self.pending_select_target = None;
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
        for entry in self.state.document.editable_fragments() {
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
                    automatic_double_bond_placement_for_segment(
                        entry.fragment,
                        begin_id,
                        end_id,
                        None,
                    )
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

    fn pending_double_state_for_new_bond_in_anchor_fragment(
        &self,
        anchor: &BondAnchor,
        begin_id: &str,
        end_id: &str,
        order: u8,
    ) -> Option<DoubleBond> {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble if order >= 2 => {
                let entry = self.editable_fragment_for_anchor(anchor)?;
                let placement = if should_default_center_double_bond_for_segment(
                    entry.fragment,
                    begin_id,
                    end_id,
                    None,
                ) {
                    DoubleBondPlacement::Center
                } else {
                    automatic_double_bond_placement_for_segment(
                        entry.fragment,
                        begin_id,
                        end_id,
                        None,
                    )
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
            BondVariant::Wavy => {
                return BondLineStyles {
                    main: BondLinePattern::Wavy,
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
            BondVariant::HollowWedge => Some(BondStereo {
                kind: "hollow-wedge".to_string(),
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

    fn update_tlc_spot_to_point(
        &mut self,
        object_id: &str,
        lane_index: usize,
        spot_index: usize,
        point: Point,
    ) -> Option<TlcSpotHit> {
        let object = self.state.document.find_scene_object_mut(object_id)?;
        let geometry = tlc_plate_geometry(object)?;
        let local_point = rotate_point(point, geometry.center, -geometry.rotate);
        let denominator = (geometry.origin_y - geometry.solvent_y).abs();
        if denominator <= crate::EPSILON {
            return None;
        }
        let rf = ((geometry.origin_y - local_point.y) / (geometry.origin_y - geometry.solvent_y))
            .clamp(0.0, 1.0);
        let lanes = object.payload.extra.get_mut("lanes")?.as_array_mut()?;
        let lane = lanes.get_mut(lane_index)?.as_object_mut()?;
        let spots = lane.get_mut("spots")?.as_array_mut()?;
        let spot = spots.get_mut(spot_index)?.as_object_mut()?;
        spot.insert("rf".to_string(), json!(round2(rf)));
        let lane_x = *geometry.lane_centers.get(lane_index)?;
        let local_center = Point::new(
            lane_x,
            geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * rf,
        );
        Some(TlcSpotHit {
            object_id: object_id.to_string(),
            lane_index,
            spot_index,
            rf: round2(rf),
            center: rotate_point(local_center, geometry.center, geometry.rotate),
            guide_points: tlc_lane_guide_points(&geometry, lane_index),
        })
    }

    fn tlc_spot_rf_at_point(
        &self,
        object_id: &str,
        lane_index: usize,
        point: Point,
    ) -> Option<f64> {
        let object = self.state.document.find_scene_object(object_id)?;
        let geometry = tlc_plate_geometry(object)?;
        let local_point = rotate_point(point, geometry.center, -geometry.rotate);
        let denominator = (geometry.origin_y - geometry.solvent_y).abs();
        if denominator <= crate::EPSILON {
            return None;
        }
        geometry.lane_centers.get(lane_index)?;
        Some(round2(
            ((geometry.origin_y - local_point.y) / (geometry.origin_y - geometry.solvent_y))
                .clamp(0.0, 1.0),
        ))
    }
}

#[derive(Default)]
struct DocumentTargetMaps {
    nodes: BTreeMap<String, JsonValue>,
    bonds: BTreeMap<String, JsonValue>,
    objects: BTreeMap<String, JsonValue>,
    styles: BTreeMap<String, JsonValue>,
}

fn documents_equivalent(before: &ChemcoreDocument, after: &ChemcoreDocument) -> bool {
    serde_json::to_value(before).ok() == serde_json::to_value(after).ok()
}

fn document_target_delta(
    before: &ChemcoreDocument,
    after: &ChemcoreDocument,
) -> CommandTargetDelta {
    let before = document_target_maps(before);
    let after = document_target_maps(after);
    let (created_nodes, updated_nodes, deleted_nodes) =
        diff_target_map(&before.nodes, &after.nodes);
    let (created_bonds, updated_bonds, deleted_bonds) =
        diff_target_map(&before.bonds, &after.bonds);
    let (created_objects, updated_objects, deleted_objects) =
        diff_target_map(&before.objects, &after.objects);
    let (created_styles, updated_styles, deleted_styles) =
        diff_target_map(&before.styles, &after.styles);

    CommandTargetDelta {
        created: CommandTargets {
            nodes: created_nodes,
            bonds: created_bonds,
            objects: created_objects,
            styles: created_styles,
        },
        updated: CommandTargets {
            nodes: updated_nodes,
            bonds: updated_bonds,
            objects: updated_objects,
            styles: updated_styles,
        },
        deleted: CommandTargets {
            nodes: deleted_nodes,
            bonds: deleted_bonds,
            objects: deleted_objects,
            styles: deleted_styles,
        },
    }
}

fn document_target_maps(document: &ChemcoreDocument) -> DocumentTargetMaps {
    let mut maps = DocumentTargetMaps::default();
    for object in document.scene_objects() {
        maps.objects.insert(
            object.id.clone(),
            serde_json::to_value(object).unwrap_or(JsonValue::Null),
        );
    }
    for (style_id, style) in &document.styles {
        maps.styles.insert(style_id.clone(), style.clone());
    }
    if let Some(entry) = document.editable_fragment() {
        for node in &entry.fragment.nodes {
            maps.nodes.insert(
                node.id.clone(),
                serde_json::to_value(node).unwrap_or(JsonValue::Null),
            );
        }
        for bond in &entry.fragment.bonds {
            maps.bonds.insert(
                bond.id.clone(),
                serde_json::to_value(bond).unwrap_or(JsonValue::Null),
            );
        }
    }
    maps
}

fn diff_target_map(
    before: &BTreeMap<String, JsonValue>,
    after: &BTreeMap<String, JsonValue>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut created = Vec::new();
    let mut updated = Vec::new();
    let mut deleted = Vec::new();
    for (id, value) in after {
        match before.get(id) {
            Some(before_value) if before_value == value => {}
            Some(_) => updated.push(id.clone()),
            None => created.push(id.clone()),
        }
    }
    for id in before.keys() {
        if !after.contains_key(id) {
            deleted.push(id.clone());
        }
    }
    (created, updated, deleted)
}

fn command_targets_union(delta: &CommandTargetDelta) -> CommandTargets {
    CommandTargets {
        nodes: union_target_ids([
            delta.created.nodes.as_slice(),
            delta.updated.nodes.as_slice(),
            delta.deleted.nodes.as_slice(),
        ]),
        bonds: union_target_ids([
            delta.created.bonds.as_slice(),
            delta.updated.bonds.as_slice(),
            delta.deleted.bonds.as_slice(),
        ]),
        objects: union_target_ids([
            delta.created.objects.as_slice(),
            delta.updated.objects.as_slice(),
            delta.deleted.objects.as_slice(),
        ]),
        styles: union_target_ids([
            delta.created.styles.as_slice(),
            delta.updated.styles.as_slice(),
            delta.deleted.styles.as_slice(),
        ]),
    }
}

fn union_target_ids<const N: usize>(groups: [&[String]; N]) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for group in groups {
        ids.extend(group.iter().cloned());
    }
    ids.into_iter().collect()
}

fn point_from_command(anchor: &CommandAnchor) -> Point {
    Point::new(anchor.x, anchor.y)
}

fn bond_anchor_from_command(anchor: CommandAnchor) -> BondAnchor {
    BondAnchor {
        node_id: anchor.node_id,
        object_id: anchor.object_id,
        point: Point::new(anchor.x, anchor.y),
        label_anchor: None,
    }
}

fn scene_object_selection_from_ids(
    document: &ChemcoreDocument,
    object_ids: &[String],
) -> SelectionState {
    let selected: BTreeSet<&str> = object_ids.iter().map(String::as_str).collect();
    let mut selection = SelectionState::default();
    for object in document.scene_objects() {
        if !selected.contains(object.id.as_str()) {
            continue;
        }
        if object.object_type == "text" {
            selection.text_objects.push(object.id.clone());
        } else {
            selection.arrow_objects.push(object.id.clone());
        }
    }
    selection
}

fn editor_command_type_name(command: &EditorCommand) -> &'static str {
    match command {
        EditorCommand::Undo => "undo",
        EditorCommand::Redo => "redo",
        EditorCommand::AddBond { .. } => "add-bond",
        EditorCommand::AddArrow { .. } => "add-arrow",
        EditorCommand::AddShape { .. } => "add-shape",
        EditorCommand::AddBracket { .. } => "add-bracket",
        EditorCommand::AddSymbol { .. } => "add-symbol",
        EditorCommand::AddElement { .. } => "add-element",
        EditorCommand::ReplaceNodeLabel { .. } => "replace-node-label",
        EditorCommand::MoveTlcSpot { .. } => "move-tlc-spot",
        EditorCommand::ApplyArrowStyle { .. } => "apply-arrow-style",
        EditorCommand::CycleBondStyle { .. } => "cycle-bond-style",
        EditorCommand::DeleteSelection => "delete-selection",
        EditorCommand::DeleteFocusedAtPoint { .. } => "delete-focused-at-point",
        EditorCommand::PasteClipboard => "paste-clipboard",
        EditorCommand::CutSelection => "cut-selection",
        EditorCommand::InsertTemplate { .. } => "insert-template",
        EditorCommand::ApplySelectionArrange { .. } => "apply-selection-arrange",
        EditorCommand::ApplySelectionOrder { .. } => "apply-selection-order",
        EditorCommand::ApplySelectionColor { .. } => "apply-selection-color",
        EditorCommand::ApplyShapeStyle { .. } => "apply-shape-style",
        EditorCommand::ApplyBracketKind { .. } => "apply-bracket-kind",
        EditorCommand::ApplyOrbitalTemplate { .. } => "apply-orbital-template",
        EditorCommand::ApplyOrbitalStyle { .. } => "apply-orbital-style",
        EditorCommand::ApplyOrbitalPhase { .. } => "apply-orbital-phase",
        EditorCommand::ApplyLineStyle { .. } => "apply-line-style",
        EditorCommand::ApplyBondStyle { .. } => "apply-bond-style",
        EditorCommand::ApplyTextStyle { .. } => "apply-text-style",
        EditorCommand::SetChemicalCheckForSelection { .. } => "set-chemical-check-for-selection",
        EditorCommand::ExpandLabelsInSelection => "expand-labels-in-selection",
        EditorCommand::CenterSelectionOnPage => "center-selection-on-page",
        EditorCommand::GroupSelection { .. } => "group-selection",
        EditorCommand::UngroupSelection { .. } => "ungroup-selection",
        EditorCommand::LinkSelection { .. } => "link-selection",
        EditorCommand::UnlinkSelection { .. } => "unlink-selection",
        EditorCommand::JoinSelection => "join-selection",
        EditorCommand::MoveSelection => "move-selection",
        EditorCommand::RotateSelection => "rotate-selection",
        EditorCommand::ResizeSelection => "resize-selection",
        EditorCommand::ScaleSelection { .. } => "scale-selection",
        EditorCommand::EditArrowGeometry { .. } => "edit-arrow-geometry",
        EditorCommand::EditShapeGeometry { .. } => "edit-shape-geometry",
        EditorCommand::ApplyTextEdit { .. } => "apply-text-edit",
        EditorCommand::ApplyObjectSettings { .. } => "apply-object-settings",
        EditorCommand::ApplyObjectSettingsToSelection { .. } => {
            "apply-object-settings-to-selection"
        }
        EditorCommand::ApplyDocumentStyle { .. } => "apply-document-style",
        EditorCommand::ReplaceHoveredEndpointLabel { .. } => "replace-hovered-endpoint-label",
        EditorCommand::AddOrbital { .. } => "add-orbital",
    }
}

#[derive(Debug, Clone)]
struct TlcPlateGeometry {
    center: Point,
    rotate: f64,
    left: f64,
    right: f64,
    origin_y: f64,
    solvent_y: f64,
    spot_radius: f64,
    lane_centers: Vec<f64>,
    spots: Vec<Vec<f64>>,
}

fn tlc_plate_geometry(object: &SceneObject) -> Option<TlcPlateGeometry> {
    if object.object_type != "shape" {
        return None;
    }
    if object.payload.extra.get("kind").and_then(JsonValue::as_str) != Some("tlcPlate") {
        return None;
    }
    let [x, y, width, height] = object.payload.bbox?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let origin_fraction = object
        .payload
        .extra
        .get("originFraction")
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.1);
    let solvent_fraction = object
        .payload
        .extra
        .get("solventFrontFraction")
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.1);
    let origin_y = ty + height * (1.0 - origin_fraction);
    let solvent_y = ty + height * solvent_fraction;
    let lane_values = object
        .payload
        .extra
        .get("lanes")
        .and_then(JsonValue::as_array)?
        .iter()
        .map(|lane| {
            let offset = lane
                .get("offset")
                .and_then(JsonValue::as_f64)
                .unwrap_or(0.5);
            let spots = lane
                .get("spots")
                .and_then(JsonValue::as_array)
                .map(|spots| {
                    spots
                        .iter()
                        .map(|spot| spot.get("rf").and_then(JsonValue::as_f64).unwrap_or(0.15))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (tx + width * offset, spots)
        })
        .collect::<Vec<_>>();
    Some(TlcPlateGeometry {
        center: Point::new(tx + width * 0.5, ty + height * 0.5),
        rotate: object.transform.rotate,
        left: tx,
        right: tx + width,
        origin_y,
        solvent_y,
        spot_radius: (width.min(height) * 0.015).clamp(2.0, 5.0),
        lane_centers: lane_values.iter().map(|(x, _)| *x).collect(),
        spots: lane_values.into_iter().map(|(_, spots)| spots).collect(),
    })
}

fn tlc_lane_guide_points(geometry: &TlcPlateGeometry, lane_index: usize) -> Vec<Point> {
    let Some(&lane_x) = geometry.lane_centers.get(lane_index) else {
        return Vec::new();
    };
    let left = if lane_index == 0 {
        (geometry.left + lane_x) * 0.5
    } else {
        (geometry.lane_centers[lane_index - 1] + lane_x) * 0.5
    };
    let right = if lane_index + 1 >= geometry.lane_centers.len() {
        (geometry.right + lane_x) * 0.5
    } else {
        (lane_x + geometry.lane_centers[lane_index + 1]) * 0.5
    };
    let top = geometry.solvent_y.min(geometry.origin_y);
    let bottom = geometry.solvent_y.max(geometry.origin_y);
    [
        Point::new(left, top),
        Point::new(right, top),
        Point::new(right, bottom),
        Point::new(left, bottom),
    ]
    .into_iter()
    .map(|point| rotate_point(point, geometry.center, geometry.rotate))
    .collect()
}

fn rotate_point(point: Point, center: Point, degrees: f64) -> Point {
    if degrees.abs() <= crate::EPSILON {
        return point;
    }
    let radians = degrees.to_radians();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * radians.cos() - dy * radians.sin(),
        center.y + dx * radians.sin() + dy * radians.cos(),
    )
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    let mut previous = *polygon.last().unwrap_or(&point);
    for current in polygon {
        let intersects = ((current.y > point.y) != (previous.y > point.y))
            && (point.x
                < (previous.x - current.x) * (point.y - current.y)
                    / (previous.y - current.y + 1.0e-12)
                    + current.x);
        if intersects {
            inside = !inside;
        }
        previous = *current;
    }
    inside
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
