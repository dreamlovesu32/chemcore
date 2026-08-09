mod analysis_captions;
mod arrows;
mod bio_shapes;
mod bond_edit;
mod bond_properties;
mod bond_styles;
mod bond_tools;
mod brackets;
mod chemical_graph;
mod chemical_properties;
mod chemistry;
mod clipboard;
mod command;
mod command_runtime;
mod context_menu;
mod context_styles;
mod delete;
mod document_commands;
mod document_layout;
mod geometry_constraints;
mod groups;
mod history;
mod images;
mod io;
mod links;
mod locks;
mod logical_relations;
mod molecular_coloring;
mod nmr_results;
mod orbitals;
mod palettes;
mod patch;
mod plasmids;
mod pointer;
mod presets;
mod render_state;
mod select;
mod selection_summary;
mod shapes;
mod spatial_index;
mod spectra;
mod stoichiometry_grids;
mod tables;
mod templates;
mod text_edit;
mod tlc;

pub(crate) use self::context_styles::expand_complete_labels_in_fragment;

pub use self::command::{
    AnnotationPropertiesPatch, ChemicalAnalysisFormat, CommandAnchor, CommandDelta,
    CommandDoubleBond, CommandResult, CommandTargetDelta, CommandTargetSet, CommandTargets,
    DocumentCommandFormat, EditorCommand, FocusedDeleteSource, HistoryEntry, HistorySnapshot,
    ObjectSettingsPatch, TextCommandContent, TextCommandDisplayMode, TextEditCommandTarget,
};
pub use self::patch::{DocumentPatch, SceneEntityPatch};
pub use self::spatial_index::SpatialQueryResult;
pub(crate) use self::text_edit::{
    carbon_valence_hydrogen_count_for_node, formula_hydrogen_count_for_node,
    implicit_hydrogen_label_text_for_count, make_periodic_element_node_label,
    refresh_attached_node_label_geometry_for_all_nodes,
    refresh_attached_node_label_geometry_for_all_nodes_with_profile,
    refresh_attached_node_label_geometry_for_node,
    refresh_attached_node_label_geometry_for_node_without_implicit_hydrogen_refresh,
    refresh_implicit_hydrogens, refresh_label_recognition_for_node,
};
use self::text_edit::{
    element_symbol_info, endpoint_label_world_bounds, mark_shortcut_implicit_hydrogen_label,
    refresh_element_valence_recognition_for_all_nodes, standalone_element_hydrogen_count,
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
pub(crate) use self::presets::editor_options_from_document;
use self::presets::{
    document_style_preset_from_document, editor_options_from_imported_cdxml_document,
    sync_document_style_info_from_options, SelectedObjectSettings,
};
use crate::{
    adjacent_directions, anchor_from_point, angle_between, bond_center_focus_length, can_draw_bond,
    can_focus_bond_center, can_focus_endpoint, default_angle_for_anchor_for_variant,
    direction_from_angle, endpoint_from_angle_for_document, endpoint_hover_radius_for_node,
    hit_test_arrow_center, hit_test_bond_center, hit_test_endpoint, hit_test_endpoint_excluding,
    largest_angular_gap, nearest_angle, normalize_angle, point_in_polygon, px_to_pt,
    refresh_repeating_units, render_document, render_document_targets, render_primitives_bounds,
    rotate_point_around, round2, snapped_angle_for_anchor, ArrowCurve, ArrowEndpointStyle,
    ArrowHeadSize, ArrowNoGo, ArrowVariant, Bond, BondAnchor, BondLinePattern, BondLineStyles,
    BondLineWeight, BondLineWeights, BondPreview, BondStereo, BondVariant, ChemSemaDocument,
    DoubleBond, DoubleBondPlacement, DragState, EditableFragment, EditableFragmentMut,
    EditorOptions, EndpointHit, HoverShape, HoverTextBox, Node, OrbitalPhase, OrbitalStyle,
    OrbitalTemplate, OverlayState, Point, PointerEvent, RenderPrimitive, RenderRole, ResourceData,
    SceneObject, SelectionState, ShapeKind, ShapeStyle, Tool, ToolState, Vector, ARROW_HIT_RADIUS,
    BOND_CENTER_HIT_RADIUS, DRAG_START_THRESHOLD, ENDPOINT_FOCUS_RADIUS, ENDPOINT_HIT_RADIUS,
    GLOBAL_SNAP_ANGLES, GRAPHIC_EDGE_HIT_RADIUS,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};

const HOVER_STROKE_WIDTH: f64 = crate::px_to_pt(1.1);
const HOVER_LABEL_STROKE_WIDTH: f64 = crate::px_to_pt(1.1);
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
            let role = primitive.role();
            role != RenderRole::DocumentKnockout
                && role != RenderRole::DocumentDiagnostic
                && !render_role_is_selection(role)
                && !render_role_is_hover(role)
                && !render_role_is_preview(role)
        }
        RenderBoundsScope::Selection => render_role_is_selection_bounds(primitive.role()),
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
        RenderPrimitive::Ellipse { object_id, .. } | RenderPrimitive::Image { object_id, .. } => {
            (object_id.as_deref(), None, None)
        }
    }
}

fn is_preview_id(id: Option<&str>) -> bool {
    id.is_some_and(|id| id.starts_with("__preview_"))
}

fn mark_preview_primitives(primitives: &mut [RenderPrimitive]) {
    for primitive in primitives {
        let role = primitive.role();
        if render_role_is_preview(role) {
            continue;
        }
        let (object_id, node_id, bond_id) = preview_primitive_ids(primitive);
        if is_preview_id(bond_id) || is_preview_id(object_id) || is_preview_id(node_id) {
            *primitive.role_mut() = RenderRole::PreviewBond;
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
    pub document: ChemSemaDocument,
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

#[derive(Clone)]
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
    bracket_edit_drag: Option<BracketEditDragState>,
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
    command_before_snapshot: Option<ChemSemaDocument>,
    pending_dialog: Option<serde_json::Value>,
    spatial_index: std::cell::RefCell<Option<spatial_index::SceneSpatialIndex>>,
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
    pub value_kind: String,
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
    PlasmidMarkerLabel(usize),
    PlasmidRegionStart(usize),
    PlasmidRegionEnd(usize),
    PlasmidRegionOffset(usize),
    BioReceptorWidth,
    BioGProteinGammaShape,
    BioDnaHeight,
    BioDnaSpacing,
    BioDnaStrandWidth,
    BioDnaOffset,
    BioHelixHeight,
    BioHelixStrandWidth,
    BioHelixCylinderWidth,
    BioHelixSpacing,
    BioMembraneUnitSize,
    BioMembraneArcStart,
    BioMembraneArcEnd,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BracketEditHandle {
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
struct BracketEditDragState {
    object_id: String,
    handle: BracketEditHandle,
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
    MoleculeSelection {
        nodes: Vec<String>,
        bonds: Vec<String>,
    },
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub(super) fn endpoint_hit_radius(&self) -> f64 {
        let scale = (self.options.bond_length_world_pt().value() / crate::DEFAULT_BOND_LENGTH)
            .sqrt()
            .clamp(0.6, 1.0);
        ENDPOINT_HIT_RADIUS * scale
    }

    pub(super) fn endpoint_focus_radius(&self) -> f64 {
        let scale = (self.options.bond_length_world_pt().value() / crate::DEFAULT_BOND_LENGTH)
            .clamp(0.35, 1.0);
        ENDPOINT_FOCUS_RADIUS * scale
    }
}

fn history_entry_is_open_for_command(entry: &HistoryEntry, command: &EditorCommand) -> bool {
    if entry.command != *command {
        return false;
    }
    match &entry.snapshot {
        HistorySnapshot::Document { after, .. } => after.is_none(),
        HistorySnapshot::SceneObjects { after_objects, .. } => after_objects.is_none(),
    }
}

fn history_entry_before_document(entry: &HistoryEntry) -> Option<&ChemSemaDocument> {
    match &entry.snapshot {
        HistorySnapshot::Document { before, .. } => Some(before.as_ref()),
        HistorySnapshot::SceneObjects { .. } => None,
    }
}

fn history_entry_before_scene_objects(entry: &HistoryEntry) -> Option<&[SceneObject]> {
    match &entry.snapshot {
        HistorySnapshot::SceneObjects { before_objects, .. } => Some(before_objects),
        HistorySnapshot::Document { .. } => None,
    }
}

fn capture_history_after_snapshot_for_document(
    entry: &mut HistoryEntry,
    document: &ChemSemaDocument,
) {
    match &mut entry.snapshot {
        HistorySnapshot::Document { after, .. } => {
            *after = Some(Box::new(document.clone()));
        }
        HistorySnapshot::SceneObjects {
            before_objects,
            after_objects,
        } => {
            let ids = before_objects
                .iter()
                .map(|object| object.id.as_str())
                .collect::<BTreeSet<_>>();
            *after_objects = Some(
                ids.iter()
                    .filter_map(|object_id| document.find_scene_object(object_id).cloned())
                    .collect(),
            );
        }
    }
}

fn replace_scene_object_snapshot(objects: &mut [SceneObject], snapshot: &SceneObject) -> bool {
    for object in objects {
        if object.id == snapshot.id {
            *object = snapshot.clone();
            return true;
        }
        if replace_scene_object_snapshot(&mut object.children, snapshot) {
            return true;
        }
    }
    false
}

fn scene_object_target_delta(
    before_objects: &[SceneObject],
    after_document: &ChemSemaDocument,
) -> CommandTargetDelta {
    let mut delta = CommandTargetDelta::default();
    for before in before_objects {
        match after_document.find_scene_object(&before.id) {
            Some(after) if before != after => delta.updated.objects.push(before.id.clone()),
            None => delta.deleted.objects.push(before.id.clone()),
            _ => {}
        }
    }
    delta
}

#[derive(Default)]
struct DocumentTargetMaps<'a> {
    nodes: BTreeMap<&'a str, &'a crate::Node>,
    bonds: BTreeMap<&'a str, &'a Bond>,
    objects: BTreeMap<&'a str, &'a SceneObject>,
    styles: BTreeMap<&'a str, &'a JsonValue>,
}

#[derive(Clone, Copy)]
struct CommandDeltaScope {
    molecule_components: bool,
    objects: bool,
    styles: bool,
}

impl CommandDeltaScope {
    const fn all() -> Self {
        Self {
            molecule_components: true,
            objects: true,
            styles: true,
        }
    }

    const fn objects_and_styles() -> Self {
        Self {
            molecule_components: false,
            objects: true,
            styles: true,
        }
    }
}

fn document_from_command_content(
    format: DocumentCommandFormat,
    content: &str,
    bytes: &[u8],
) -> Result<ChemSemaDocument, String> {
    let mut document = match format {
        DocumentCommandFormat::Json | DocumentCommandFormat::Ccjs => {
            crate::parse_document_json(content)?
        }
        DocumentCommandFormat::Cdxml => {
            let mut document = crate::parse_cdxml_document(content, None)?;
            crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
            document
        }
        DocumentCommandFormat::Cdx => {
            let mut document = crate::parse_cdx_document(bytes, None)?;
            crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
            document
        }
        DocumentCommandFormat::Sdf => crate::parse_sdf_document(content, None)?,
        DocumentCommandFormat::Svg => {
            return Err(
                "SVG is an export format and cannot be converted into an editable document."
                    .to_string(),
            );
        }
    };
    refresh_repeating_units(&mut document);
    Ok(document)
}

fn refresh_element_valence_recognition_for_all_editable_fragments(document: &mut ChemSemaDocument) {
    let object_ids = document
        .editable_fragments()
        .into_iter()
        .map(|entry| entry.object.id.clone())
        .collect::<Vec<_>>();
    for object_id in object_ids {
        if let Some(entry) = document.editable_fragment_mut_for_object(&object_id) {
            refresh_element_valence_recognition_for_all_nodes(entry.fragment);
        }
    }
}

fn document_command_format_name(format: DocumentCommandFormat) -> &'static str {
    match format {
        DocumentCommandFormat::Json => "json",
        DocumentCommandFormat::Ccjs => "ccjs",
        DocumentCommandFormat::Cdxml => "cdxml",
        DocumentCommandFormat::Cdx => "cdx",
        DocumentCommandFormat::Sdf => "sdf",
        DocumentCommandFormat::Svg => "svg",
    }
}

fn json_value_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

fn document_target_delta(
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
) -> CommandTargetDelta {
    document_target_delta_with_scope(before, after, CommandDeltaScope::all())
}

fn document_target_delta_with_scope(
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
    scope: CommandDeltaScope,
) -> CommandTargetDelta {
    let before_maps = document_target_maps(before);
    let after_maps = document_target_maps(after);
    let (created_nodes, mut updated_nodes, deleted_nodes) = if scope.molecule_components {
        diff_target_map(&before_maps.nodes, &after_maps.nodes)
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    let (created_bonds, mut updated_bonds, deleted_bonds) = if scope.molecule_components {
        diff_target_map(&before_maps.bonds, &after_maps.bonds)
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    if scope.molecule_components {
        expand_updated_bonds_with_colored_area_changes(
            &mut updated_bonds,
            before,
            after,
            &after_maps.bonds,
        );
        expand_updated_nodes_with_changed_bond_endpoints(
            &mut updated_nodes,
            &created_bonds,
            &updated_bonds,
            &deleted_bonds,
            &before_maps.bonds,
            &after_maps.bonds,
            &after_maps.nodes,
        );
        expand_updated_bonds_with_visual_dependencies(
            &mut updated_bonds,
            &created_nodes,
            &updated_nodes,
            &deleted_nodes,
            &created_bonds,
            &deleted_bonds,
            before,
            after,
        );
    }
    let (mut created_objects, mut updated_objects, mut deleted_objects) = if scope.objects {
        diff_target_map_by(
            &before_maps.objects,
            &after_maps.objects,
            scene_object_shallow_eq,
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    if scope.objects {
        let before_properties = before
            .chemical_properties
            .iter()
            .map(|property| (property.id.as_str(), property))
            .collect::<BTreeMap<_, _>>();
        let after_properties = after
            .chemical_properties
            .iter()
            .map(|property| (property.id.as_str(), property))
            .collect::<BTreeMap<_, _>>();
        let (created, updated, deleted) = diff_target_map(&before_properties, &after_properties);
        created_objects.extend(created);
        updated_objects.extend(updated);
        deleted_objects.extend(deleted);
        let before_logical_values = logical_target_values(before);
        let after_logical_values = logical_target_values(after);
        let before_logical = before_logical_values
            .iter()
            .map(|(id, value)| (*id, value))
            .collect::<BTreeMap<_, _>>();
        let after_logical = after_logical_values
            .iter()
            .map(|(id, value)| (*id, value))
            .collect::<BTreeMap<_, _>>();
        let (created, updated, deleted) = diff_target_map(&before_logical, &after_logical);
        created_objects.extend(created);
        updated_objects.extend(updated);
        deleted_objects.extend(deleted);
        created_objects.sort();
        created_objects.dedup();
        updated_objects.sort();
        updated_objects.dedup();
        deleted_objects.sort();
        deleted_objects.dedup();
    }
    let (created_styles, updated_styles, deleted_styles) = if scope.styles {
        diff_target_map(&before_maps.styles, &after_maps.styles)
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };

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

fn logical_target_values(document: &ChemSemaDocument) -> Vec<(&str, JsonValue)> {
    let mut values = Vec::new();
    macro_rules! append {
        ($items:expr) => {
            values.extend($items.iter().map(|item| {
                (
                    item.id.as_str(),
                    serde_json::to_value(item).expect("logical object serializes"),
                )
            }));
        };
    }
    append!(document.logical_objects.alternative_groups);
    append!(document.logical_objects.bracketed_groups);
    append!(document.logical_objects.sequences);
    append!(document.logical_objects.cross_references);
    append!(document.logical_objects.object_tags);
    append!(document.logical_objects.annotations);
    append!(document.logical_objects.registry_numbers);
    append!(document.logical_objects.representations);
    append!(document.links);
    values.extend([
        (
            "logical-order:alternative-groups",
            serde_json::json!(document
                .logical_objects
                .alternative_groups
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:bracketed-groups",
            serde_json::json!(document
                .logical_objects
                .bracketed_groups
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:sequences",
            serde_json::json!(document
                .logical_objects
                .sequences
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:cross-references",
            serde_json::json!(document
                .logical_objects
                .cross_references
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:object-tags",
            serde_json::json!(document
                .logical_objects
                .object_tags
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:annotations",
            serde_json::json!(document
                .logical_objects
                .annotations
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:registry-numbers",
            serde_json::json!(document
                .logical_objects
                .registry_numbers
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:representations",
            serde_json::json!(document
                .logical_objects
                .representations
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
        (
            "logical-order:reaction-schemes",
            serde_json::json!(document
                .reaction_schemes
                .iter()
                .map(|item| &item.id)
                .collect::<Vec<_>>()),
        ),
    ]);
    for scheme in &document.reaction_schemes {
        values.push((
            scheme.id.as_str(),
            serde_json::to_value(scheme).expect("reaction scheme serializes"),
        ));
        append!(scheme.steps);
    }
    values
}

fn document_target_maps(document: &ChemSemaDocument) -> DocumentTargetMaps<'_> {
    let mut maps = DocumentTargetMaps::default();
    for object in document.scene_objects() {
        maps.objects.insert(object.id.as_str(), object);
    }
    for (style_id, style) in &document.styles {
        maps.styles.insert(style_id.as_str(), style);
    }
    for entry in document.editable_fragments() {
        for node in &entry.fragment.nodes {
            maps.nodes.insert(node.id.as_str(), node);
        }
        for bond in &entry.fragment.bonds {
            maps.bonds.insert(bond.id.as_str(), bond);
        }
    }
    maps
}

fn expand_updated_bonds_with_colored_area_changes(
    updated_bonds: &mut Vec<String>,
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
    after_bonds: &BTreeMap<&str, &Bond>,
) {
    let area_map = |document: &ChemSemaDocument| {
        document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| {
                entry
                    .fragment
                    .colored_areas
                    .iter()
                    .map(move |area| ((entry.object.id.clone(), area.id.clone()), area.clone()))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before_areas = area_map(before);
    let after_areas = area_map(after);
    let mut changed_bonds = updated_bonds.iter().cloned().collect::<BTreeSet<_>>();
    for key in before_areas.keys().chain(after_areas.keys()) {
        if before_areas.get(key) == after_areas.get(key) {
            continue;
        }
        for bond_id in before_areas
            .get(key)
            .into_iter()
            .chain(after_areas.get(key))
            .flat_map(|area| area.basis_bonds.iter())
        {
            if after_bonds.contains_key(bond_id.as_str()) {
                changed_bonds.insert(bond_id.clone());
            }
        }
    }
    *updated_bonds = changed_bonds.into_iter().collect();
}

fn expand_updated_nodes_with_changed_bond_endpoints(
    updated_nodes: &mut Vec<String>,
    created_bonds: &[String],
    updated_bonds: &[String],
    deleted_bonds: &[String],
    before_bonds: &BTreeMap<&str, &Bond>,
    after_bonds: &BTreeMap<&str, &Bond>,
    after_nodes: &BTreeMap<&str, &Node>,
) {
    let mut nodes: BTreeSet<String> = updated_nodes.iter().cloned().collect();
    let mut add_existing_endpoint = |node_id: &str| {
        if after_nodes.contains_key(node_id) {
            nodes.insert(node_id.to_string());
        }
    };
    for bond_id in created_bonds.iter().chain(updated_bonds) {
        if let Some(bond) = after_bonds.get(bond_id.as_str()) {
            add_existing_endpoint(&bond.begin);
            add_existing_endpoint(&bond.end);
        }
    }
    for bond_id in updated_bonds.iter().chain(deleted_bonds) {
        if let Some(bond) = before_bonds.get(bond_id.as_str()) {
            add_existing_endpoint(&bond.begin);
            add_existing_endpoint(&bond.end);
        }
    }
    *updated_nodes = nodes.into_iter().collect();
}

#[derive(Debug, Clone)]
struct TargetBondSegment {
    id: String,
    begin: String,
    end: String,
    start: Point,
    end_point: Point,
}

#[allow(clippy::too_many_arguments)]
fn expand_updated_bonds_with_visual_dependencies(
    updated_bonds: &mut Vec<String>,
    created_nodes: &[String],
    updated_nodes: &[String],
    deleted_nodes: &[String],
    created_bonds: &[String],
    deleted_bonds: &[String],
    before: &ChemSemaDocument,
    after: &ChemSemaDocument,
) {
    let created_bonds: BTreeSet<String> = created_bonds.iter().cloned().collect();
    let changed_nodes: BTreeSet<String> = created_nodes
        .iter()
        .chain(updated_nodes)
        .chain(deleted_nodes)
        .cloned()
        .collect();
    if updated_bonds.is_empty()
        && created_bonds.is_empty()
        && deleted_bonds.is_empty()
        && changed_nodes.is_empty()
    {
        return;
    }
    let after_bonds: BTreeSet<String> = after
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.clone()))
        .collect();
    let mut affected_bonds: BTreeSet<String> = updated_bonds
        .iter()
        .chain(&created_bonds)
        .chain(deleted_bonds)
        .cloned()
        .collect();
    let mut visual_bonds: BTreeSet<String> = updated_bonds.iter().cloned().collect();

    for segments in collect_target_bond_segments(before)
        .into_iter()
        .chain(collect_target_bond_segments(after))
    {
        for segment in &segments {
            if changed_nodes.contains(&segment.begin) || changed_nodes.contains(&segment.end) {
                affected_bonds.insert(segment.id.clone());
                if after_bonds.contains(&segment.id) && !created_bonds.contains(&segment.id) {
                    visual_bonds.insert(segment.id.clone());
                }
            }
        }

        let affected_indices: Vec<usize> = segments
            .iter()
            .enumerate()
            .filter_map(|(index, segment)| affected_bonds.contains(&segment.id).then_some(index))
            .collect();
        for affected_index in affected_indices {
            for other_index in 0..segments.len() {
                if affected_index == other_index {
                    continue;
                }
                let (under, over) = if affected_index < other_index {
                    (&segments[affected_index], &segments[other_index])
                } else {
                    (&segments[other_index], &segments[affected_index])
                };
                if target_bond_segments_cross(under, over)
                    && after_bonds.contains(&over.id)
                    && !created_bonds.contains(&over.id)
                {
                    visual_bonds.insert(over.id.clone());
                }
            }
        }
    }

    *updated_bonds = visual_bonds.into_iter().collect();
}

fn collect_target_bond_segments(document: &ChemSemaDocument) -> Vec<Vec<TargetBondSegment>> {
    let mut segments = Vec::new();
    for entry in document.editable_fragments() {
        let node_map: BTreeMap<&str, &Node> = entry
            .fragment
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        for bond in &entry.fragment.bonds {
            let (Some(begin), Some(end)) = (
                node_map.get(bond.begin.as_str()),
                node_map.get(bond.end.as_str()),
            ) else {
                continue;
            };
            let start = entry.world_point_for_node(begin);
            let end_point = entry.world_point_for_node(end);
            if start.distance(end_point) <= crate::EPSILON {
                continue;
            }
            segments.push(TargetBondSegment {
                id: bond.id.clone(),
                begin: bond.begin.clone(),
                end: bond.end.clone(),
                start,
                end_point,
            });
        }
    }
    vec![segments]
}

fn target_bond_segments_cross(first: &TargetBondSegment, second: &TargetBondSegment) -> bool {
    if first.begin == second.begin
        || first.begin == second.end
        || first.end == second.begin
        || first.end == second.end
    {
        return false;
    }
    let first_vector = Vector::new(
        first.end_point.x - first.start.x,
        first.end_point.y - first.start.y,
    );
    let second_vector = Vector::new(
        second.end_point.x - second.start.x,
        second.end_point.y - second.start.y,
    );
    if first_vector.length() <= crate::EPSILON || second_vector.length() <= crate::EPSILON {
        return false;
    }
    let crossing_sin =
        target_vector_cross(first_vector.normalized(), second_vector.normalized()).abs();
    if crossing_sin <= 0.1 {
        return false;
    }
    target_segment_intersection(first.start, first.end_point, second.start, second.end_point)
        .is_some()
}

fn target_segment_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<Point> {
    let a = Vector::new(a2.x - a1.x, a2.y - a1.y);
    let b = Vector::new(b2.x - b1.x, b2.y - b1.y);
    let denom = target_vector_cross(a, b);
    if denom.abs() <= crate::EPSILON {
        return None;
    }
    let offset = Vector::new(b1.x - a1.x, b1.y - a1.y);
    let t = target_vector_cross(offset, b) / denom;
    let u = target_vector_cross(offset, a) / denom;
    if t <= 1.0e-6 || t >= 1.0 - 1.0e-6 || u <= 1.0e-6 || u >= 1.0 - 1.0e-6 {
        return None;
    }
    Some(Point::new(a1.x + a.x * t, a1.y + a.y * t))
}

fn target_vector_cross(first: Vector, second: Vector) -> f64 {
    first.x * second.y - first.y * second.x
}

fn diff_target_map<T: PartialEq>(
    before: &BTreeMap<&str, &T>,
    after: &BTreeMap<&str, &T>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    diff_target_map_by(before, after, |before, after| before == after)
}

fn diff_target_map_by<T>(
    before: &BTreeMap<&str, &T>,
    after: &BTreeMap<&str, &T>,
    equivalent: impl Fn(&T, &T) -> bool,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut created = Vec::new();
    let mut updated = Vec::new();
    let mut deleted = Vec::new();
    for (id, value) in after {
        match before.get(id) {
            Some(before_value) if equivalent(*before_value, *value) => {}
            Some(_) => updated.push((*id).to_string()),
            None => created.push((*id).to_string()),
        }
    }
    for id in before.keys() {
        if !after.contains_key(id) {
            deleted.push((*id).to_string());
        }
    }
    (created, updated, deleted)
}

fn scene_object_shallow_eq(before: &SceneObject, after: &SceneObject) -> bool {
    before.id == after.id
        && before.object_type == after.object_type
        && before.name == after.name
        && before.visible == after.visible
        && before.locked == after.locked
        && before.z_index == after.z_index
        && before.transform == after.transform
        && before.style_ref == after.style_ref
        && before.link_policy == after.link_policy
        && before.meta == after.meta
        && before.payload == after.payload
}

fn scene_object_needs_repeating_unit_refresh(object: &SceneObject) -> bool {
    matches!(
        object.object_type.as_str(),
        "bracket" | "group" | "molecule"
    ) || object
        .children
        .iter()
        .any(scene_object_needs_repeating_unit_refresh)
}

fn command_target_delta_is_empty(delta: &CommandTargetDelta) -> bool {
    delta.created.is_empty() && delta.updated.is_empty() && delta.deleted.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DocumentInfo, DocumentLayout, DocumentStyleInfo, FormatInfo, MoleculeFragment, Node,
        ObjectPayload, Page, Resource, ResourceData, Transform,
    };

    fn molecule_object(id: &str, resource_ref: &str) -> SceneObject {
        SceneObject {
            id: id.to_string(),
            object_type: "molecule".to_string(),
            name: id.to_string(),
            visible: true,
            locked: false,
            z_index: 10,
            transform: Transform::identity(),
            style_ref: None,
            link_policy: Default::default(),
            meta: JsonValue::Null,
            payload: ObjectPayload {
                resource_ref: Some(resource_ref.to_string()),
                bbox: Some([0.0, 0.0, 80.0, 80.0]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis: None,
                plasmid_map: None,
                bio_shape: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        }
    }

    fn molecule_resource(node: Node) -> Resource {
        Resource {
            resource_type: "molecule_fragment2d".to_string(),
            encoding: "chemsema.molecule.fragment2d".to_string(),
            data: ResourceData::Fragment(MoleculeFragment {
                schema: "chemsema.molecule.fragment2d".to_string(),
                bbox: [0.0, 0.0, 80.0, 80.0],
                nodes: vec![node],
                bonds: Vec::new(),
                colored_areas: Vec::new(),
                stereo: Vec::new(),
                interactions: Vec::new(),
                meta: JsonValue::Null,
            }),
            meta: JsonValue::Null,
        }
    }

    fn two_molecule_document() -> ChemSemaDocument {
        let mut resources = BTreeMap::new();
        resources.insert(
            "mol_a".to_string(),
            molecule_resource(Node::carbon("node_a".to_string(), Point::new(10.0, 10.0))),
        );
        resources.insert(
            "mol_b".to_string(),
            molecule_resource(Node::carbon("node_b".to_string(), Point::new(40.0, 40.0))),
        );
        ChemSemaDocument {
            format: FormatInfo {
                name: "chemsema".to_string(),
                version: "0.2".to_string(),
                unit: "pt".to_string(),
            },
            document: DocumentInfo {
                id: "doc_multi_molecule".to_string(),
                title: "multi molecule".to_string(),
                page: Page {
                    width: 100.0,
                    height: 100.0,
                    background: "#ffffff".to_string(),
                },
                layout: DocumentLayout::default(),
                meta: JsonValue::Null,
            },
            style: DocumentStyleInfo::default(),
            styles: BTreeMap::new(),
            objects: vec![
                molecule_object("obj_mol_a", "mol_a"),
                molecule_object("obj_mol_b", "mol_b"),
            ],
            links: Vec::new(),
            orders: Default::default(),
            logical_objects: Default::default(),
            reaction_schemes: Vec::new(),
            chemical_properties: Vec::new(),
            resources,
            interchange: BTreeMap::new(),
        }
    }

    #[test]
    fn command_target_delta_tracks_nodes_in_later_molecule_objects() {
        let before = two_molecule_document();
        let mut after = before.clone();
        let entry = after
            .editable_fragment_mut_for_object("obj_mol_b")
            .expect("second molecule should be editable");
        entry.fragment.nodes[0].position = [52.0, 44.0];

        let delta = document_target_delta(&before, &after);

        assert!(
            delta.updated.nodes.contains(&"node_b".to_string()),
            "updated node in second molecule should be reported for incremental rendering: {delta:?}"
        );
        assert!(
            !delta.updated.nodes.contains(&"node_a".to_string()),
            "unchanged node in first molecule should not be reported: {delta:?}"
        );
    }

    #[test]
    fn locking_selected_molecule_is_one_undoable_command() {
        let mut engine = Engine::new();
        engine.state.document = two_molecule_document();
        engine.state.selection = SelectionState {
            molecule_objects: vec!["obj_mol_a".to_string()],
            nodes: vec!["node_a".to_string()],
            ..SelectionState::default()
        };

        assert!(engine.set_selection_locked(true));
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some_and(|object| object.locked));
        assert!(engine.undo());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some_and(|object| !object.locked));
        assert!(engine.redo());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some_and(|object| object.locked));
    }

    #[test]
    fn mixed_delete_preserves_locked_object_and_remains_one_history_step() {
        let mut engine = Engine::new();
        engine.state.document = two_molecule_document();
        engine
            .state
            .document
            .find_scene_object_mut("obj_mol_a")
            .expect("first molecule exists")
            .locked = true;

        assert!(engine.select_all());
        assert!(engine.delete_selection());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_b")
            .is_none());

        assert!(engine.undo());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_b")
            .is_some());
        assert!(engine.redo());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some());
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_b")
            .is_none());

        assert!(engine.select_all());
        assert!(
            !engine.delete_selection(),
            "an all-locked selection must be a no-op"
        );
        assert!(engine
            .state
            .document
            .find_scene_object("obj_mol_a")
            .is_some());
    }

    #[test]
    fn mixed_target_move_changes_only_the_editable_molecule_and_is_undoable() {
        let mut engine = Engine::new();
        engine.state.document = two_molecule_document();
        engine
            .state
            .document
            .find_scene_object_mut("obj_mol_a")
            .expect("first molecule exists")
            .locked = true;
        let targets = CommandTargetSet {
            nodes: vec!["node_a".to_string(), "node_b".to_string()],
            ..CommandTargetSet::default()
        };

        assert!(
            engine
                .execute_command(EditorCommand::MoveTargets {
                    targets,
                    delta: CommandDelta { dx: 5.0, dy: 3.0 },
                })
                .expect("move command executes")
                .changed
        );
        let positions = engine
            .state
            .document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .map(|node| (node.id.clone(), node.position))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(positions["node_a"], [10.0, 10.0]);
        assert_eq!(positions["node_b"], [45.0, 43.0]);

        assert!(engine.undo());
        let positions = engine
            .state
            .document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .map(|node| (node.id.clone(), node.position))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(positions["node_a"], [10.0, 10.0]);
        assert_eq!(positions["node_b"], [40.0, 40.0]);
    }

    #[test]
    fn all_locked_target_move_is_a_no_op_without_history() {
        let mut engine = Engine::new();
        engine.state.document = two_molecule_document();
        for object_id in ["obj_mol_a", "obj_mol_b"] {
            engine
                .state
                .document
                .find_scene_object_mut(object_id)
                .expect("molecule exists")
                .locked = true;
        }
        let targets = CommandTargetSet {
            nodes: vec!["node_a".to_string(), "node_b".to_string()],
            ..CommandTargetSet::default()
        };

        assert!(
            !engine
                .execute_command(EditorCommand::MoveTargets {
                    targets,
                    delta: CommandDelta { dx: 5.0, dy: 3.0 },
                })
                .expect("move command executes")
                .changed
        );
        assert!(!engine.can_undo());
    }

    #[test]
    fn mixed_arrow_pointer_drag_moves_only_the_editable_object() {
        let mut engine = Engine::new();
        let first_id = engine
            .add_arrow_between(Point::new(10.0, 20.0), Point::new(40.0, 20.0))
            .expect("first arrow is created");
        let second_id = engine
            .add_arrow_between(Point::new(60.0, 50.0), Point::new(90.0, 50.0))
            .expect("second arrow is created");
        engine
            .state
            .document
            .find_scene_object_mut(&first_id)
            .expect("first arrow exists")
            .locked = true;
        engine.state.selection = SelectionState {
            arrow_objects: vec![first_id.clone(), second_id.clone()],
            ..SelectionState::default()
        };
        let points = |engine: &Engine, object_id: &str| {
            engine
                .state
                .document
                .find_scene_object(object_id)
                .and_then(|object| object.payload.extra.get("points"))
                .cloned()
                .expect("arrow points exist")
        };
        let first_before = points(&engine, &first_id);
        let second_before = points(&engine, &second_id);

        assert!(engine.begin_selection_move_at_point(Point::new(75.0, 50.0), false, false));
        assert!(engine.finish_selection_move(Point::new(87.0, 50.0), true));

        assert_eq!(points(&engine, &first_id), first_before);
        assert_ne!(points(&engine, &second_id), second_before);
    }

    #[test]
    fn locked_group_ancestor_blocks_descendant_motion_in_mixed_pointer_drag() {
        let mut engine = Engine::new();
        let first_id = engine
            .add_arrow_between(Point::new(10.0, 20.0), Point::new(40.0, 20.0))
            .expect("first grouped arrow is created");
        let second_id = engine
            .add_arrow_between(Point::new(10.0, 40.0), Point::new(40.0, 40.0))
            .expect("second grouped arrow is created");
        engine.state.selection = SelectionState {
            arrow_objects: vec![first_id.clone(), second_id.clone()],
            ..SelectionState::default()
        };
        assert!(engine.group_selection());
        let group_id = engine.state.selection.arrow_objects[0].clone();
        assert!(engine.set_selection_locked(true));

        let editable_id = engine
            .add_arrow_between(Point::new(70.0, 60.0), Point::new(100.0, 60.0))
            .expect("editable root arrow is created");
        let points = |engine: &Engine, object_id: &str| {
            engine
                .state
                .document
                .find_scene_object(object_id)
                .and_then(|object| object.payload.extra.get("points"))
                .cloned()
                .expect("arrow points exist")
        };
        let first_before = points(&engine, &first_id);
        let second_before = points(&engine, &second_id);
        let editable_before = points(&engine, &editable_id);
        let group_before = engine
            .state
            .document
            .find_scene_object(&group_id)
            .expect("group exists")
            .transform
            .clone();

        assert!(engine.select_all());
        assert!(engine.begin_selection_move_at_point(Point::new(85.0, 60.0), false, false));
        assert!(engine.finish_selection_move(Point::new(97.0, 60.0), true));

        assert_eq!(points(&engine, &first_id), first_before);
        assert_eq!(points(&engine, &second_id), second_before);
        assert_eq!(
            engine
                .state
                .document
                .find_scene_object(&group_id)
                .expect("group remains")
                .transform,
            group_before
        );
        assert_ne!(points(&engine, &editable_id), editable_before);

        assert!(engine.undo());
        engine.state.selection = SelectionState {
            arrow_objects: vec![group_id.clone()],
            ..SelectionState::default()
        };
        assert!(engine.set_selection_locked(false));
        assert!(engine.select_all());
        assert!(engine.begin_selection_move_at_point(Point::new(85.0, 60.0), false, false));
        assert!(engine.finish_selection_move(Point::new(97.0, 60.0), true));

        assert_ne!(
            engine
                .state
                .document
                .find_scene_object(&group_id)
                .expect("unlocked group remains")
                .transform,
            group_before
        );
        assert_ne!(points(&engine, &editable_id), editable_before);
    }

    fn distance_annotation_engine() -> Engine {
        let source = r#"<CDXML BondLength="14.4"><page id="1">
          <fragment id="10"><n id="101" p="40 40"/><n id="102" p="60 40"/><b id="103" B="101" E="102"/></fragment>
          <constraint id="201" ConstraintType="Distance" ConstraintMin="0" ConstraintMax="0" BasisObjects="101 102">
            <objecttag TagType="Unknown" Name="distance"><t p="50 35"><s>0 Å</s></t></objecttag>
          </constraint>
        </page></CDXML>"#;
        let mut engine = Engine::new();
        engine
            .load_cdxml_document(source)
            .expect("distance constraint loads");
        engine
    }

    #[test]
    fn annotation_drag_is_transient_and_does_not_create_undo_history() {
        let mut engine = distance_annotation_engine();
        let annotation_id = engine
            .state
            .document
            .scene_objects()
            .into_iter()
            .find(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .expect("constraint exists")
            .id
            .clone();
        let bounds = super::select::object_selection_bounds_for_render(
            &engine.state.document,
            engine
                .state
                .document
                .find_scene_object(&annotation_id)
                .expect("constraint exists"),
        )
        .expect("constraint has visual bounds");
        let start = Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5);
        engine.state.selection = SelectionState {
            arrow_objects: vec![annotation_id],
            ..SelectionState::default()
        };
        let before = engine.document_json().expect("document serializes");
        let before_render =
            serde_json::to_string(&engine.render_list()).expect("render list serializes");
        assert!(engine.begin_selection_move_at_point(start, false, false));
        let end = Point::new(start.x + 30.0, start.y + 30.0);
        assert!(engine.update_selection_move(end, false));
        assert_eq!(engine.document_json().expect("preview serializes"), before);
        assert_ne!(
            serde_json::to_string(&engine.render_list()).expect("render list serializes"),
            before_render
        );
        assert!(!engine.finish_selection_move(end, false));
        assert_eq!(engine.document_json().expect("document serializes"), before);
        assert_eq!(
            serde_json::to_string(&engine.render_list()).expect("render list serializes"),
            before_render
        );
        assert!(!engine.can_undo());
    }

    #[test]
    fn labeled_atoms_are_valid_ordered_annotation_points() {
        let mut engine = distance_annotation_engine();
        let node_ids = engine
            .state
            .document
            .editable_fragment()
            .expect("fragment exists")
            .fragment
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        engine.state.selection = SelectionState {
            label_nodes: node_ids,
            ..SelectionState::default()
        };
        assert!(engine
            .annotation_menu_values()
            .iter()
            .any(|(_, value)| *value == "distance"));
    }

    #[test]
    fn deleting_a_basis_entity_cascades_annotations_and_their_links() {
        let mut engine = distance_annotation_engine();
        let node_id = engine
            .state
            .document
            .editable_fragment()
            .expect("fragment exists")
            .fragment
            .nodes[0]
            .id
            .clone();
        engine.state.selection = SelectionState {
            nodes: vec![node_id],
            ..SelectionState::default()
        };
        assert!(engine.delete_selection());
        assert!(!engine.state.document.scene_objects().iter().any(|object| {
            matches!(
                object.kind(),
                crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
            )
        }));
        assert!(engine
            .state
            .document
            .links
            .iter()
            .all(|relation| relation.kind != "annotation-basis"));
    }

    #[test]
    fn copying_annotation_with_all_basis_entities_remaps_strong_references() {
        let mut engine = distance_annotation_engine();
        let node_ids = engine
            .state
            .document
            .editable_fragment()
            .expect("fragment exists")
            .fragment
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let annotation_id = engine
            .state
            .document
            .scene_objects()
            .into_iter()
            .find(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .expect("constraint exists")
            .id
            .clone();
        engine.state.selection = SelectionState {
            nodes: node_ids,
            arrow_objects: vec![annotation_id],
            ..SelectionState::default()
        };
        assert!(engine.copy_selection());
        assert!(engine.paste_clipboard());
        let constraints = engine
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .collect::<Vec<_>>();
        assert_eq!(constraints.len(), 2);
        let pasted = constraints
            .into_iter()
            .find(|object| engine.state.selection.arrow_objects.contains(&object.id))
            .expect("pasted constraint is selected");
        let basis = &pasted
            .payload
            .constraint
            .as_ref()
            .expect("constraint payload")
            .basis_entity_ids;
        assert_eq!(basis.len(), 2);
        assert!(basis
            .iter()
            .all(|id| engine.state.selection.nodes.contains(id)));
        let serialized = engine.document_json().expect("document serializes");
        crate::parse_document_json(&serialized).expect("pasted document validates");
    }

    #[test]
    fn moving_selected_basis_and_annotation_moves_live_value_exactly_once() {
        let mut engine = distance_annotation_engine();
        let node_ids = engine
            .state
            .document
            .editable_fragment()
            .expect("fragment exists")
            .fragment
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let annotation_id = engine
            .state
            .document
            .scene_objects()
            .into_iter()
            .find(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .expect("constraint exists")
            .id
            .clone();
        let annotation_before = engine
            .state
            .document
            .find_scene_object(&annotation_id)
            .expect("constraint exists")
            .clone();
        let bounds_before = super::select::object_selection_bounds_for_render(
            &engine.state.document,
            &annotation_before,
        )
        .expect("constraint has visual bounds");
        let start = Point::new(
            (bounds_before[0] + bounds_before[2]) * 0.5,
            (bounds_before[1] + bounds_before[3]) * 0.5,
        );
        engine.state.selection = SelectionState {
            nodes: node_ids,
            arrow_objects: vec![annotation_id.clone()],
            ..SelectionState::default()
        };
        assert!(engine.begin_selection_move_at_point(start, false, false));
        let end = Point::new(start.x + 30.0, start.y + 20.0);
        assert!(engine.update_selection_move(end, false));
        assert!(engine.finish_selection_move(end, false));

        let annotation_after = engine
            .state
            .document
            .find_scene_object(&annotation_id)
            .expect("constraint remains");
        assert_eq!(annotation_after.transform, annotation_before.transform);
        let bounds_after = super::select::object_selection_bounds_for_render(
            &engine.state.document,
            annotation_after,
        )
        .expect("constraint has moved visual bounds");
        assert!((bounds_after[0] - bounds_before[0] - 30.0).abs() < 0.05);
        assert!((bounds_after[1] - bounds_before[1] - 20.0).abs() < 0.05);
    }

    #[test]
    fn cross_tab_document_copy_omits_annotation_without_complete_basis() {
        let mut engine = distance_annotation_engine();
        let node_id = engine
            .state
            .document
            .editable_fragment()
            .expect("fragment exists")
            .fragment
            .nodes[0]
            .id
            .clone();
        let annotation_id = engine
            .state
            .document
            .scene_objects()
            .into_iter()
            .find(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .expect("constraint exists")
            .id
            .clone();
        engine.state.selection = SelectionState {
            nodes: vec![node_id],
            arrow_objects: vec![annotation_id],
            ..SelectionState::default()
        };
        let json = engine
            .clipboard_document_json()
            .expect("clipboard document serializes")
            .expect("selection creates clipboard document");
        let document = crate::parse_document_json(&json).expect("clipboard document validates");
        assert!(!document.scene_objects().iter().any(|object| {
            matches!(
                object.kind(),
                crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
            )
        }));
        assert!(document
            .links
            .iter()
            .all(|relation| relation.kind != "annotation-basis"));
    }

    #[test]
    fn annotation_property_dialog_and_command_edit_explicit_display_state() {
        let mut engine = distance_annotation_engine();
        let annotation_id = engine
            .state
            .document
            .scene_objects()
            .into_iter()
            .find(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .expect("constraint exists")
            .id
            .clone();
        engine.state.selection = SelectionState {
            arrow_objects: vec![annotation_id.clone()],
            ..SelectionState::default()
        };
        let dialog: JsonValue =
            serde_json::from_str(&engine.annotation_dialog_json("selected")).expect("dialog JSON");
        let field_keys = dialog["fields"]
            .as_array()
            .expect("dialog fields")
            .iter()
            .filter_map(|field| field["key"].as_str())
            .collect::<BTreeSet<_>>();
        for key in [
            "minimum",
            "maximum",
            "autoValue",
            "textOverride",
            "positioningType",
            "indicatorVisible",
            "fontFamily",
            "fontSize",
            "fill",
        ] {
            assert!(field_keys.contains(key), "missing dialog field {key}");
        }
        engine
            .execute_command_json(
                &serde_json::json!({
                    "type": "update-annotation",
                    "objectId": annotation_id,
                    "properties": {
                        "autoValue": false,
                        "textOverride": "measured",
                        "positioningType": "absolute",
                        "positionX": 75.0,
                        "positionY": 55.0,
                        "indicatorVisible": false,
                        "fontFamily": "Times New Roman",
                        "fontSize": 9.0,
                        "fill": "#ff0000",
                        "bold": true,
                        "italic": true,
                        "underline": true
                    }
                })
                .to_string(),
            )
            .expect("annotation update succeeds");
        let constraint = engine
            .state
            .document
            .find_scene_object(
                engine
                    .state
                    .selection
                    .arrow_objects
                    .first()
                    .expect("annotation remains selected"),
            )
            .and_then(|object| object.payload.constraint.as_ref())
            .expect("constraint payload");
        assert!(!constraint.display.auto_value);
        assert_eq!(
            constraint.display.text_override.as_deref(),
            Some("measured")
        );
        assert_eq!(
            constraint.display.positioning_type,
            crate::AnnotationPositioningType::Absolute
        );
        assert_eq!(constraint.display.position, Some([75.0, 55.0]));
        assert!(!constraint.display.indicator_visible);
        assert_eq!(constraint.display.font_weight, 700);
        crate::parse_document_json(&engine.document_json().expect("document serializes"))
            .expect("edited annotation validates");
    }
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

fn command_double_bond_override(
    double_placement: Option<DoubleBondPlacement>,
    double: Option<CommandDoubleBond>,
) -> Option<DoubleBond> {
    double_placement
        .map(|placement| DoubleBond {
            placement,
            center_exit_side: double.and_then(|double| double.center_exit_side),
            frozen: true,
        })
        .or_else(|| {
            double.map(|double| DoubleBond {
                placement: double.placement,
                center_exit_side: double.center_exit_side,
                frozen: true,
            })
        })
}

fn bond_plan_keypad_slots(
    document: &ChemSemaDocument,
    anchor: &BondAnchor,
    default_angle: f64,
    length: f64,
) -> Vec<JsonValue> {
    [
        ("6", 0.0),
        ("3", 45.0),
        ("2", 90.0),
        ("1", 135.0),
        ("4", 180.0),
        ("7", 225.0),
        ("8", 270.0),
        ("9", 315.0),
        ("5", default_angle),
    ]
    .into_iter()
    .map(|(key, angle)| {
        let angle = normalize_angle(angle);
        let endpoint = endpoint_from_angle_for_document(document, anchor, angle, length);
        json!({
            "key": key,
            "angleDeg": angle,
            "end": CommandAnchor {
                node_id: None,
                object_id: anchor.object_id.clone(),
                x: endpoint.x,
                y: endpoint.y,
            },
        })
    })
    .collect()
}

fn scene_object_selection_from_ids(
    document: &ChemSemaDocument,
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
        } else if object.object_type == "molecule" {
            selection.molecule_objects.push(object.id.clone());
        } else {
            selection.arrow_objects.push(object.id.clone());
        }
    }
    for entry in document.editable_fragments() {
        for node in &entry.fragment.nodes {
            if selected.contains(node.id.as_str()) {
                selection.nodes.push(node.id.clone());
                if node.label.is_some() {
                    selection.label_nodes.push(node.id.clone());
                }
            }
        }
    }
    selection
}

fn editor_command_is_creation(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::InsertSmiles { .. }
            | EditorCommand::AddBond { .. }
            | EditorCommand::AddArrow { .. }
            | EditorCommand::AddShape { .. }
            | EditorCommand::AddBioShape { .. }
            | EditorCommand::AddTable { .. }
            | EditorCommand::AddBracket { .. }
            | EditorCommand::AddSymbol { .. }
            | EditorCommand::AddElement { .. }
            | EditorCommand::AddText { .. }
            | EditorCommand::AddImage { .. }
            | EditorCommand::AddOrbital { .. }
    )
}

fn editor_command_is_immediate(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::LoadDocument { .. }
            | EditorCommand::ExportDocument { .. }
            | EditorCommand::ConvertDocument { .. }
            | EditorCommand::InspectDocument { .. }
            | EditorCommand::ChemicalAnalysis { .. }
            | EditorCommand::SelectTargets { .. }
            | EditorCommand::SelectAll
            | EditorCommand::ClearSelection
            | EditorCommand::PlanBond { .. }
            | EditorCommand::PlanTemplate { .. }
    )
}

fn editor_command_is_selection_semantic(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::SetInterpretChemicallyForSelection { .. }
            | EditorCommand::SetImplicitHydrogenCountForSelection { .. }
            | EditorCommand::SetAtomPropertyForSelection { .. }
            | EditorCommand::SetBondPropertyForSelection { .. }
            | EditorCommand::SetChemicalCheckForSelection { .. }
    )
}

fn editor_command_is_relationship(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::LinkSelection { .. }
            | EditorCommand::UnlinkSelection { .. }
            | EditorCommand::SetLinkPolicy { .. }
            | EditorCommand::PasteAnalysisCaption { .. }
            | EditorCommand::ApplyChemicalProperty { .. }
            | EditorCommand::ApplyChemicalPropertyResult { .. }
            | EditorCommand::DeleteChemicalProperty { .. }
            | EditorCommand::CreateAnnotation { .. }
            | EditorCommand::UpdateAnnotation { .. }
            | EditorCommand::AnalyzeStoichiometry { .. }
            | EditorCommand::SetStoichiometryDatum { .. }
            | EditorCommand::EditStoichiometryGrid { .. }
            | EditorCommand::BindStoichiometryGrid { .. }
    )
}

fn editor_command_is_logical_object(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::SetLogicalObject { .. }
            | EditorCommand::DeleteLogicalObject { .. }
            | EditorCommand::ReorderLogicalObject { .. }
    )
}

fn editor_command_is_style(command: &EditorCommand) -> bool {
    matches!(
        command,
        EditorCommand::ApplyArrowStyle { .. }
            | EditorCommand::ApplyShapeStyle { .. }
            | EditorCommand::ApplyBracketKind { .. }
            | EditorCommand::ApplyOrbitalTemplate { .. }
            | EditorCommand::ApplyOrbitalStyle { .. }
            | EditorCommand::ApplyOrbitalPhase { .. }
            | EditorCommand::ApplyLineStyle { .. }
            | EditorCommand::ApplyBondStyle { .. }
            | EditorCommand::ApplyMolecularHighlight { .. }
            | EditorCommand::ApplyRingFill { .. }
            | EditorCommand::ApplyTextStyle { .. }
    )
}

fn editor_command_type_name(command: &EditorCommand) -> &'static str {
    match command {
        EditorCommand::Undo => "undo",
        EditorCommand::Redo => "redo",
        EditorCommand::LoadDocument { .. } => "load-document",
        EditorCommand::ExportDocument { .. } => "export-document",
        EditorCommand::ConvertDocument { .. } => "convert-document",
        EditorCommand::InspectDocument { .. } => "inspect-document",
        EditorCommand::InsertSmiles { .. } => "insert-smiles",
        EditorCommand::ChemicalAnalysis { .. } => "chemical-analysis",
        EditorCommand::SelectTargets { .. } => "select-targets",
        EditorCommand::SelectAll => "select-all",
        EditorCommand::ClearSelection => "clear-selection",
        EditorCommand::PlanBond { .. } => "plan-bond",
        EditorCommand::PlanTemplate { .. } => "plan-template",
        EditorCommand::AddBond { .. } => "add-bond",
        EditorCommand::AddArrow { .. } => "add-arrow",
        EditorCommand::AddShape { .. } => "add-shape",
        EditorCommand::AddBioShape { .. } => "add-bio-shape",
        EditorCommand::AddTable { .. } => "add-table",
        EditorCommand::EditTable { .. } => "edit-table",
        EditorCommand::SetTableBorders { .. } => "set-table-borders",
        EditorCommand::SetPlasmidMap { .. } => "set-plasmid-map",
        EditorCommand::SetBioShape { .. } => "set-bio-shape",
        EditorCommand::AddBracket { .. } => "add-bracket",
        EditorCommand::AddSymbol { .. } => "add-symbol",
        EditorCommand::AddElement { .. } => "add-element",
        EditorCommand::AddText { .. } => "add-text",
        EditorCommand::AddImage { .. } => "add-image",
        EditorCommand::SetImageCrop { .. } => "set-image-crop",
        EditorCommand::SetTextRuns { .. } => "set-text-runs",
        EditorCommand::SetNodeLabelRuns { .. } => "set-node-label-runs",
        EditorCommand::SetNodeCharge { .. } => "set-node-charge",
        EditorCommand::ReplaceNodeLabel { .. } => "replace-node-label",
        EditorCommand::MoveChromatographyMark { .. } => "move-chromatography-mark",
        EditorCommand::ApplyArrowStyle { .. } => "apply-arrow-style",
        EditorCommand::CycleBondStyle { .. } => "cycle-bond-style",
        EditorCommand::DeleteSelection => "delete-selection",
        EditorCommand::SetObjectsLocked { .. } => "set-objects-locked",
        EditorCommand::DeleteTargets { .. } => "delete-targets",
        EditorCommand::DeleteFocusedAtPoint { .. } => "delete-focused-at-point",
        EditorCommand::PasteClipboard => "paste-clipboard",
        EditorCommand::CutSelection => "cut-selection",
        EditorCommand::InsertTemplate { .. } => "insert-template",
        EditorCommand::ApplySelectionArrange { .. } => "apply-selection-arrange",
        EditorCommand::ApplySelectionOrder { .. } => "apply-selection-order",
        EditorCommand::ApplySelectionColor { .. } => "apply-selection-color",
        EditorCommand::ApplyMolecularHighlight { .. } => "apply-molecular-highlight",
        EditorCommand::ApplyRingFill { .. } => "apply-ring-fill",
        EditorCommand::ApplyShapeStyle { .. } => "apply-shape-style",
        EditorCommand::ApplyBracketKind { .. } => "apply-bracket-kind",
        EditorCommand::ApplyOrbitalTemplate { .. } => "apply-orbital-template",
        EditorCommand::ApplyOrbitalStyle { .. } => "apply-orbital-style",
        EditorCommand::ApplyOrbitalPhase { .. } => "apply-orbital-phase",
        EditorCommand::ApplyLineStyle { .. } => "apply-line-style",
        EditorCommand::ApplyBondStyle { .. } => "apply-bond-style",
        EditorCommand::ApplyTextStyle { .. } => "apply-text-style",
        EditorCommand::SetInterpretChemicallyForSelection { .. } => {
            "set-interpret-chemically-for-selection"
        }
        EditorCommand::SetImplicitHydrogenCountForSelection { .. } => {
            "set-implicit-hydrogen-count-for-selection"
        }
        EditorCommand::SetAtomPropertyForSelection { .. } => "set-atom-property-for-selection",
        EditorCommand::SetBondPropertyForSelection { .. } => "set-bond-property-for-selection",
        EditorCommand::SetChemicalCheckForSelection { .. } => "set-chemical-check-for-selection",
        EditorCommand::ExpandLabelsInSelection => "expand-labels-in-selection",
        EditorCommand::CenterSelectionOnPage => "center-selection-on-page",
        EditorCommand::GroupSelection { .. } => "group-selection",
        EditorCommand::UngroupSelection { .. } => "ungroup-selection",
        EditorCommand::LinkSelection { .. } => "link-selection",
        EditorCommand::PasteAnalysisCaption { .. } => "paste-analysis-caption",
        EditorCommand::ApplyChemicalProperty { .. } => "apply-chemical-property",
        EditorCommand::ApplyChemicalPropertyResult { .. } => "apply-chemical-property-result",
        EditorCommand::DeleteChemicalProperty { .. } => "delete-chemical-property",
        EditorCommand::CreateAnnotation { .. } => "create-annotation",
        EditorCommand::UpdateAnnotation { .. } => "update-annotation",
        EditorCommand::AnalyzeStoichiometry { .. } => "analyze-stoichiometry",
        EditorCommand::SetStoichiometryDatum { .. } => "set-stoichiometry-datum",
        EditorCommand::EditStoichiometryGrid { .. } => "edit-stoichiometry-grid",
        EditorCommand::BindStoichiometryGrid { .. } => "bind-stoichiometry-grid",
        EditorCommand::SetLogicalObject { .. } => "set-logical-object",
        EditorCommand::DeleteLogicalObject { .. } => "delete-logical-object",
        EditorCommand::ReorderLogicalObject { .. } => "reorder-logical-object",
        EditorCommand::SetLinkPolicy { .. } => "set-link-policy",
        EditorCommand::UnlinkSelection { .. } => "unlink-selection",
        EditorCommand::JoinSelection => "join-selection",
        EditorCommand::MoveTargets { .. } => "move-targets",
        EditorCommand::RotateTargets { .. } => "rotate-targets",
        EditorCommand::ScaleTargets { .. } => "scale-targets",
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
        EditorCommand::InitializeDocumentLayout => "initialize-document-layout",
        EditorCommand::SetDocumentLayout { .. } => "set-document-layout",
        EditorCommand::SetArrowGeometry { .. } => "set-arrow-geometry",
        EditorCommand::SetShapeGeometry { .. } => "set-shape-geometry",
        EditorCommand::SetSpectrumData { .. } => "set-spectrum-data",
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
    .map(|point| crate::rotate_point_around(point, geometry.center, geometry.rotate))
    .collect()
}

fn collect_document_colors(document: &ChemSemaDocument) -> Vec<String> {
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
