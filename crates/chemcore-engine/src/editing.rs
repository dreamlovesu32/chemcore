use crate::{
    angle_between, angle_in_clockwise_arc, angular_distance, css_px, direction_from_angle,
    fragment_bond_visual_bounds, largest_angular_gap, normalize_angle, split_label_groups,
    world_pt, Bond, ChemcoreDocument, EditableFragment, Node, Point, Vector, WorldPoint, WorldPt,
    DEFAULT_BOND_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

#[path = "editing/anchors.rs"]
mod anchors;
#[path = "editing/arrows.rs"]
mod arrows;
#[path = "editing/geometry.rs"]
mod geometry;
#[path = "editing/hit_testing.rs"]
mod hit_testing;

pub use self::anchors::{
    adjacent_directions, anchor_from_point, default_angle_for_anchor,
    default_angle_for_anchor_for_variant, endpoint_from_angle, endpoint_from_angle_for_document,
    nearest_angle, node_by_id, snapped_angle_for_anchor,
};
pub use self::arrows::{
    arrow_endpoint_style_handle_point, arrow_object_focus_points, arrow_object_handle_points,
    arrow_object_has_curve_handle, line_object_arrow_dimension, line_object_endpoint_style,
    line_object_points, point_at_distance_from_start, polyline_length,
};
pub use self::geometry::bond_center_focus_length;
use self::geometry::*;
pub use self::hit_testing::{
    hit_test_arrow_center, hit_test_bond, hit_test_bond_center, hit_test_endpoint,
    hit_test_endpoint_excluding, select_at,
};

pub const ENDPOINT_FOCUS_RADIUS_PT: WorldPt = world_pt(2.834_645_669_291_339);
pub const ENDPOINT_HIT_RADIUS_PT: WorldPt = css_px(9.0).to_world_pt();
pub const BOND_HIT_RADIUS_PT: WorldPt = css_px(6.0).to_world_pt();
pub const BOND_CENTER_FOCUS_LENGTH_PT: WorldPt = world_pt(22.677_165_354_330_71);
pub const BOND_CENTER_FOCUS_WIDTH_PT: WorldPt = world_pt(5.669_291_338_582_678);
pub const BOND_CENTER_HIT_RADIUS_PT: WorldPt = BOND_CENTER_FOCUS_LENGTH_PT;
pub const DRAG_START_THRESHOLD_PT: WorldPt = css_px(4.0).to_world_pt();
pub const ENDPOINT_FOCUS_RADIUS: f64 = ENDPOINT_FOCUS_RADIUS_PT.value();
pub const ENDPOINT_HIT_RADIUS: f64 = ENDPOINT_HIT_RADIUS_PT.value();
pub const BOND_HIT_RADIUS: f64 = BOND_HIT_RADIUS_PT.value();
pub const BOND_CENTER_FOCUS_LENGTH: f64 = BOND_CENTER_FOCUS_LENGTH_PT.value();
pub const BOND_CENTER_FOCUS_WIDTH: f64 = BOND_CENTER_FOCUS_WIDTH_PT.value();
pub const BOND_CENTER_HIT_RADIUS: f64 = BOND_CENTER_HIT_RADIUS_PT.value();
pub const DRAG_START_THRESHOLD: f64 = DRAG_START_THRESHOLD_PT.value();
pub const BLANK_CANVAS_DEFAULT_ANGLE: f64 = 330.0;
pub const GLOBAL_SNAP_ANGLES: &[f64] = &[
    0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0, 150.0, 165.0, 180.0, 195.0,
    210.0, 225.0, 240.0, 255.0, 270.0, 285.0, 300.0, 315.0, 330.0, 345.0,
];
pub const RELATIVE_BOND_ANGLES: &[f64] = &[
    15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0, 150.0, 165.0, 180.0,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    Select,
    Bond,
    Arrow,
    Bracket,
    Symbol,
    Element,
    Delete,
    Text,
    Shape,
    TlcPlate,
    Orbital,
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
    Wavy,
    Wedge,
    HashedWedge,
    HollowWedge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrowVariant {
    Solid,
    Curved,
    CurvedMirror,
    Hollow,
    Open,
    Equilibrium,
    UnequalEquilibrium,
}

impl Default for ArrowVariant {
    fn default() -> Self {
        Self::Solid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrowHeadSize {
    Large,
    Medium,
    Small,
}

impl Default for ArrowHeadSize {
    fn default() -> Self {
        Self::Small
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrowEndpointStyle {
    None,
    Full,
    Left,
    Right,
}

impl Default for ArrowEndpointStyle {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrowNoGo {
    None,
    Cross,
    Hash,
}

impl Default for ArrowNoGo {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrowCurve {
    Arc270,
    Arc180,
    Arc120,
    Arc90,
}

impl Default for ArrowCurve {
    fn default() -> Self {
        Self::Arc270
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeKind {
    Circle,
    Ellipse,
    RoundRect,
    Rect,
    CrossTable,
    TlcPlate,
}

impl Default for ShapeKind {
    fn default() -> Self {
        Self::Circle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeStyle {
    Solid,
    Dashed,
    Shaded,
    Filled,
    Shadowed,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self::Solid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrbitalTemplate {
    S,
    P,
    Dxy,
    Oval,
    Hybrid,
    Dz2,
    Lobe,
}

impl Default for OrbitalTemplate {
    fn default() -> Self {
        Self::S
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrbitalStyle {
    Hollow,
    Shaded,
    Filled,
}

impl Default for OrbitalStyle {
    fn default() -> Self {
        Self::Hollow
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrbitalPhase {
    Plus,
    Minus,
}

impl Default for OrbitalPhase {
    fn default() -> Self {
        Self::Plus
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BracketKind {
    Round,
    Square,
    Curly,
    DoubleDagger,
    Dagger,
    CirclePlus,
    Plus,
    RadicalCation,
    LonePair,
    CircleMinus,
    Minus,
    RadicalAnion,
    Electron,
}

impl Default for BracketKind {
    fn default() -> Self {
        Self::Round
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorOptions {
    pub bond_length: f64,
    pub bond_stroke_width: f64,
    pub bold_bond_width: f64,
    pub wedge_width: f64,
    pub label_clip_margin: f64,
    pub hash_spacing: f64,
    pub bond_spacing: f64,
    pub margin_width: f64,
    pub graphic_stroke_width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSettings {
    pub bond_length: f64,
    pub line_width: f64,
    pub bold_width: f64,
    pub bond_spacing: f64,
    pub margin_width: f64,
    pub hash_spacing: f64,
}

impl From<&EditorOptions> for ObjectSettings {
    fn from(options: &EditorOptions) -> Self {
        Self {
            bond_length: options.bond_length,
            line_width: options.bond_stroke_width,
            bold_width: options.bold_bond_width,
            bond_spacing: options.bond_spacing,
            margin_width: options.margin_width,
            hash_spacing: options.hash_spacing,
        }
    }
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            bond_length: DEFAULT_BOND_LENGTH,
            bond_stroke_width: crate::DEFAULT_BOND_STROKE,
            bold_bond_width: crate::BOLD_BOND_WIDTH_PT.value(),
            wedge_width: crate::SOLID_WEDGE_WIDTH_PT.value(),
            label_clip_margin: crate::LABEL_GEOMETRY_CLIP_MARGIN_PT.value(),
            hash_spacing: crate::DEFAULT_HASH_SPACING_PT.value(),
            bond_spacing: crate::DEFAULT_BOND_SPACING_PERCENT,
            margin_width: crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value(),
            graphic_stroke_width: crate::DEFAULT_BOND_STROKE,
        }
    }
}

impl EditorOptions {
    pub const fn bond_length_world_pt(&self) -> WorldPt {
        WorldPt(self.bond_length)
    }

    pub const fn bond_stroke_world_pt(&self) -> WorldPt {
        WorldPt(self.bond_stroke_width)
    }

    pub const fn bold_bond_width_world_pt(&self) -> WorldPt {
        WorldPt(self.bold_bond_width)
    }

    pub const fn wedge_width_world_pt(&self) -> WorldPt {
        WorldPt(self.wedge_width)
    }

    pub const fn label_clip_margin_world_pt(&self) -> WorldPt {
        WorldPt(self.label_clip_margin)
    }

    pub const fn hash_spacing_world_pt(&self) -> WorldPt {
        WorldPt(self.hash_spacing)
    }

    pub const fn bond_spacing_percent(&self) -> f64 {
        self.bond_spacing
    }

    pub const fn margin_width_world_pt(&self) -> WorldPt {
        WorldPt(self.margin_width)
    }

    pub const fn graphic_stroke_world_pt(&self) -> WorldPt {
        WorldPt(self.graphic_stroke_width)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    pub active_tool: Tool,
    pub bond_variant: BondVariant,
    #[serde(default)]
    pub arrow_variant: ArrowVariant,
    #[serde(default)]
    pub arrow_head_size: ArrowHeadSize,
    #[serde(default)]
    pub arrow_curve: ArrowCurve,
    #[serde(default = "default_arrow_head_style")]
    pub arrow_head_style: ArrowEndpointStyle,
    #[serde(default)]
    pub arrow_tail_style: ArrowEndpointStyle,
    #[serde(default = "default_arrow_head")]
    pub arrow_head: bool,
    #[serde(default)]
    pub arrow_tail: bool,
    #[serde(default)]
    pub arrow_bold: bool,
    #[serde(default)]
    pub arrow_no_go: ArrowNoGo,
    #[serde(default)]
    pub shape_kind: ShapeKind,
    #[serde(default)]
    pub shape_style: ShapeStyle,
    #[serde(default = "default_shape_color")]
    pub shape_color: String,
    #[serde(default)]
    pub orbital_template: OrbitalTemplate,
    #[serde(default)]
    pub orbital_style: OrbitalStyle,
    #[serde(default)]
    pub orbital_phase: OrbitalPhase,
    #[serde(default = "default_shape_color")]
    pub orbital_color: String,
    #[serde(default)]
    pub bracket_kind: BracketKind,
    #[serde(default = "default_symbol_kind")]
    pub symbol_kind: BracketKind,
    #[serde(default = "default_element_symbol")]
    pub element_symbol: String,
    #[serde(default = "default_element_atomic_number")]
    pub element_atomic_number: u8,
    #[serde(default = "default_template")]
    pub template: String,
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            active_tool: Tool::Bond,
            bond_variant: BondVariant::Single,
            arrow_variant: ArrowVariant::Solid,
            arrow_head_size: ArrowHeadSize::Small,
            arrow_curve: ArrowCurve::Arc270,
            arrow_head_style: ArrowEndpointStyle::Full,
            arrow_tail_style: ArrowEndpointStyle::None,
            arrow_head: true,
            arrow_tail: false,
            arrow_bold: false,
            arrow_no_go: ArrowNoGo::None,
            shape_kind: ShapeKind::Circle,
            shape_style: ShapeStyle::Solid,
            shape_color: default_shape_color(),
            orbital_template: OrbitalTemplate::S,
            orbital_style: OrbitalStyle::Hollow,
            orbital_phase: OrbitalPhase::Plus,
            orbital_color: default_shape_color(),
            bracket_kind: BracketKind::Round,
            symbol_kind: default_symbol_kind(),
            element_symbol: default_element_symbol(),
            element_atomic_number: default_element_atomic_number(),
            template: default_template(),
        }
    }
}

fn default_arrow_head() -> bool {
    true
}

fn default_arrow_head_style() -> ArrowEndpointStyle {
    ArrowEndpointStyle::Full
}

fn default_template() -> String {
    "ring-6".to_string()
}

fn default_symbol_kind() -> BracketKind {
    BracketKind::CirclePlus
}

fn default_element_symbol() -> String {
    "P".to_string()
}

fn default_element_atomic_number() -> u8 {
    15
}

fn default_shape_color() -> String {
    "#000000".to_string()
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
    pub const fn from_world_point(point: WorldPoint, button: Option<u8>, alt_key: bool) -> Self {
        Self {
            x: point.x.value(),
            y: point.y.value(),
            button,
            alt_key,
        }
    }

    pub fn point(&self) -> Point {
        Point::from_world(self.world_point())
    }

    pub const fn world_point(&self) -> WorldPoint {
        WorldPoint::new(WorldPt(self.x), WorldPt(self.y))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointHit {
    pub node_id: String,
    pub object_id: String,
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
    #[serde(default)]
    pub text_objects: Vec<String>,
    #[serde(default)]
    pub arrow_objects: Vec<String>,
    #[serde(default)]
    pub label_nodes: Vec<String>,
    #[serde(default)]
    pub region: bool,
    pub nodes: Vec<String>,
    pub bonds: Vec<String>,
}

impl SelectionState {
    pub fn is_empty(&self) -> bool {
        self.text_objects.is_empty()
            && self.arrow_objects.is_empty()
            && self.label_nodes.is_empty()
            && self.nodes.is_empty()
            && self.bonds.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondAnchor {
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    pub point: Point,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_anchor: Option<LabelAnchorGeometry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelAnchorGeometry {
    pub glyph_index: usize,
    pub glyph_point: Point,
    pub glyph_box: [f64; 4],
    pub first_glyph_point: Point,
    pub left_point: Point,
    pub right_point: Point,
    pub rightmost_glyph_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_group_point: Option<Point>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverTextBox {
    pub bounds: [f64; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverArrow {
    pub object_id: String,
    pub center: Point,
    #[serde(default)]
    pub handles: Vec<Point>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverShape {
    pub object_id: String,
    #[serde(default)]
    pub handles: Vec<Point>,
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
    pub hover_text_box: Option<HoverTextBox>,
    pub hover_arrow: Option<HoverArrow>,
    pub hover_shape: Option<HoverShape>,
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
    matches!(tool_state.active_tool, Tool::Bond | Tool::Delete)
}

pub fn can_focus_endpoint(tool_state: &ToolState) -> bool {
    matches!(
        tool_state.active_tool,
        Tool::Bond | Tool::Delete | Tool::Text
    )
}
