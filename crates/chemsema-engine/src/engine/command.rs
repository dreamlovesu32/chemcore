use crate::{
    ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, BondAnchor,
    BondLineWeights, BondVariant, BracketKind, ChemSemaDocument, DoubleBondPlacement, LabelRun,
    ObjectSettings, OrbitalPhase, OrbitalStyle, OrbitalTemplate, Point, SceneObject, ShapeKind,
    ShapeStyle,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAnchor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandTargetSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bonds: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub label_nodes: Vec<String>,
}

impl CommandTargetSet {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
            && self.bonds.is_empty()
            && self.objects.is_empty()
            && self.label_nodes.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDelta {
    pub dx: f64,
    pub dy: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextCommandDisplayMode {
    ConnectionAuto,
    LeftAuto,
    RightAuto,
    PreserveLeft,
    PreserveRight,
    PreserveCenter,
    StackAbove,
    StackBelow,
}

impl TextCommandDisplayMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionAuto => "connection-auto",
            Self::LeftAuto => "left-auto",
            Self::RightAuto => "right-auto",
            Self::PreserveLeft => "preserve-left",
            Self::PreserveRight => "preserve-right",
            Self::PreserveCenter => "preserve-center",
            Self::StackAbove => "stack-above",
            Self::StackBelow => "stack-below",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextCommandContent {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(default, alias = "runs", skip_serializing_if = "Vec::is_empty")]
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
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub box_value: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_offset: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_position: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glyph_polygons: Vec<Vec<[f64; 2]>>,
    #[serde(default)]
    pub preserve_measured_box: bool,
    #[serde(default)]
    pub preserve_implicit_hydrogen_label: bool,
    #[serde(default)]
    pub default_chemical: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<TextCommandDisplayMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentCommandFormat {
    Json,
    Ccjs,
    Cdxml,
    Cdx,
    Sdf,
    Svg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChemicalAnalysisFormat {
    ChemicalGraphV2,
    Smiles,
    Inchi,
    InchiKey,
}

impl From<&BondAnchor> for CommandAnchor {
    fn from(anchor: &BondAnchor) -> Self {
        Self {
            node_id: anchor.node_id.clone(),
            object_id: anchor.object_id.clone(),
            x: anchor.point.x,
            y: anchor.point.y,
        }
    }
}

impl From<Point> for CommandAnchor {
    fn from(point: Point) -> Self {
        Self {
            node_id: None,
            object_id: None,
            x: point.x,
            y: point.y,
        }
    }
}

fn default_bond_order() -> u8 {
    1
}

fn default_bond_variant() -> BondVariant {
    BondVariant::Single
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDoubleBond {
    pub placement: DoubleBondPlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_exit_side: Option<DoubleBondPlacement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum EditorCommand {
    Undo,
    Redo,
    LoadDocument {
        format: DocumentCommandFormat,
        #[serde(default, skip_serializing)]
        content: String,
        #[serde(default, alias = "contentBytes", skip_serializing)]
        bytes: Vec<u8>,
    },
    ExportDocument {
        format: DocumentCommandFormat,
    },
    ConvertDocument {
        from: DocumentCommandFormat,
        to: DocumentCommandFormat,
        #[serde(default, skip_serializing)]
        content: String,
        #[serde(default, alias = "contentBytes", skip_serializing)]
        bytes: Vec<u8>,
    },
    InspectDocument {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        include: Vec<String>,
    },
    InsertSmiles {
        smiles: String,
        x: f64,
        y: f64,
    },
    ChemicalAnalysis {
        format: ChemicalAnalysisFormat,
        #[serde(default)]
        targets: CommandTargetSet,
    },
    SelectTargets {
        targets: CommandTargetSet,
    },
    SelectAll,
    ClearSelection,
    PlanBond {
        begin: CommandAnchor,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<Point>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        angle: Option<f64>,
        #[serde(
            default,
            rename = "bondLength",
            alias = "length",
            skip_serializing_if = "Option::is_none"
        )]
        bond_length: Option<f64>,
        #[serde(default = "default_bond_order")]
        order: u8,
        #[serde(default = "default_bond_variant")]
        variant: BondVariant,
    },
    PlanTemplate {
        template: String,
        x: f64,
        y: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<CommandAnchor>,
        #[serde(
            default,
            rename = "bondId",
            alias = "bond_id",
            skip_serializing_if = "Option::is_none"
        )]
        bond_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<Point>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        angle: Option<f64>,
        #[serde(
            default,
            rename = "bondLength",
            alias = "length",
            skip_serializing_if = "Option::is_none"
        )]
        bond_length: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        side: Option<f64>,
    },
    AddBond {
        begin: CommandAnchor,
        end: CommandAnchor,
        order: u8,
        variant: BondVariant,
        #[serde(
            default,
            rename = "wideEnd",
            alias = "wide_end",
            skip_serializing_if = "Option::is_none"
        )]
        wide_end: Option<String>,
        #[serde(
            default,
            rename = "doublePlacement",
            alias = "double_placement",
            skip_serializing_if = "Option::is_none"
        )]
        double_placement: Option<DoubleBondPlacement>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        double: Option<CommandDoubleBond>,
        #[serde(
            default,
            rename = "lineWeights",
            alias = "line_weights",
            skip_serializing_if = "Option::is_none"
        )]
        line_weights: Option<BondLineWeights>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(
            default,
            rename = "endpointAttachments",
            alias = "endpoint_attachments",
            skip_serializing_if = "Option::is_none"
        )]
        endpoint_attachments: Option<Value>,
    },
    AddArrow {
        begin: CommandAnchor,
        end: CommandAnchor,
        variant: ArrowVariant,
        #[serde(alias = "headSize")]
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        #[serde(alias = "headStyle")]
        head_style: ArrowEndpointStyle,
        #[serde(alias = "tailStyle")]
        tail_style: ArrowEndpointStyle,
        head: bool,
        tail: bool,
        bold: bool,
        #[serde(alias = "noGo")]
        no_go: ArrowNoGo,
    },
    AddShape {
        kind: ShapeKind,
        style: ShapeStyle,
        color: String,
        begin: CommandAnchor,
        end: CommandAnchor,
    },
    AddBracket {
        kind: BracketKind,
        begin: CommandAnchor,
        end: CommandAnchor,
    },
    AddSymbol {
        kind: BracketKind,
        center: CommandAnchor,
    },
    AddElement {
        symbol: String,
        #[serde(alias = "atomicNumber")]
        atomic_number: u8,
        center: CommandAnchor,
    },
    AddText {
        position: Point,
        #[serde(flatten)]
        content: TextCommandContent,
    },
    AddImage {
        #[serde(alias = "mimeType")]
        mime_type: String,
        #[serde(alias = "dataBase64")]
        data_base64: String,
        #[serde(alias = "pixelWidth")]
        pixel_width: u32,
        #[serde(alias = "pixelHeight")]
        pixel_height: u32,
        position: Point,
        width: f64,
        height: f64,
        #[serde(default, alias = "sourceName", skip_serializing_if = "Option::is_none")]
        source_name: Option<String>,
    },
    SetTextRuns {
        #[serde(alias = "objectId")]
        object_id: String,
        #[serde(flatten)]
        content: TextCommandContent,
    },
    SetNodeLabelRuns {
        #[serde(alias = "nodeId")]
        node_id: String,
        #[serde(flatten)]
        content: TextCommandContent,
    },
    SetNodeCharge {
        #[serde(alias = "nodeId")]
        node_id: String,
        charge: i32,
    },
    ReplaceNodeLabel {
        node_id: String,
        label: String,
    },
    MoveTlcSpot {
        #[serde(alias = "objectId")]
        object_id: String,
        #[serde(alias = "laneIndex")]
        lane_index: usize,
        #[serde(alias = "spotIndex")]
        spot_index: usize,
        #[serde(alias = "beforeRf")]
        before_rf: f64,
    },
    ApplyArrowStyle {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        #[serde(alias = "headSize")]
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        #[serde(alias = "headStyle")]
        head_style: ArrowEndpointStyle,
        #[serde(alias = "tailStyle")]
        tail_style: ArrowEndpointStyle,
        head: bool,
        tail: bool,
        bold: bool,
        #[serde(alias = "noGo")]
        no_go: ArrowNoGo,
        variant: ArrowVariant,
    },
    CycleBondStyle {
        #[serde(alias = "bondId")]
        bond_id: String,
        variant: BondVariant,
    },
    DeleteSelection,
    DeleteTargets {
        targets: CommandTargetSet,
    },
    DeleteFocusedAtPoint {
        x: f64,
        y: f64,
        source: FocusedDeleteSource,
    },
    PasteClipboard,
    CutSelection,
    InsertTemplate {
        template: String,
        x: f64,
        y: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<CommandAnchor>,
        #[serde(
            default,
            rename = "bondId",
            alias = "bond_id",
            skip_serializing_if = "Option::is_none"
        )]
        bond_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<Point>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        angle: Option<f64>,
        #[serde(
            default,
            rename = "bondLength",
            alias = "length",
            skip_serializing_if = "Option::is_none"
        )]
        bond_length: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        side: Option<f64>,
    },
    ApplySelectionArrange {
        command: String,
    },
    ApplySelectionOrder {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        command: String,
    },
    ApplySelectionColor {
        color: String,
    },
    ApplyShapeStyle {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        style: String,
    },
    ApplyBracketKind {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        kind: String,
    },
    ApplyOrbitalTemplate {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        template: String,
    },
    ApplyOrbitalStyle {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        style: String,
    },
    ApplyOrbitalPhase {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        phase: String,
    },
    ApplyLineStyle {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        style: String,
    },
    ApplyBondStyle {
        #[serde(default, alias = "bondIds")]
        bond_ids: Vec<String>,
        style: String,
    },
    ApplyTextStyle {
        #[serde(default, alias = "textObjectIds")]
        text_object_ids: Vec<String>,
        #[serde(default, alias = "labelNodeIds")]
        label_node_ids: Vec<String>,
        #[serde(default, alias = "nodeIds")]
        node_ids: Vec<String>,
        command: String,
        value: String,
    },
    SetInterpretChemicallyForSelection {
        enabled: bool,
    },
    SetImplicitHydrogenCountForSelection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<u8>,
    },
    SetAtomPropertyForSelection {
        property: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
    SetBondPropertyForSelection {
        property: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
    SetChemicalCheckForSelection {
        enabled: bool,
    },
    ExpandLabelsInSelection,
    CenterSelectionOnPage,
    GroupSelection {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
    },
    UngroupSelection {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
    },
    LinkSelection {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
    },
    UnlinkSelection {
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
    },
    JoinSelection,
    MoveTargets {
        targets: CommandTargetSet,
        delta: CommandDelta,
    },
    RotateTargets {
        targets: CommandTargetSet,
        center: Point,
        degrees: f64,
    },
    ScaleTargets {
        targets: CommandTargetSet,
        #[serde(rename = "scaleX", alias = "scale_x")]
        scale_x: f64,
        #[serde(rename = "scaleY", alias = "scale_y")]
        scale_y: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pivot: Option<Point>,
    },
    MoveSelection,
    RotateSelection,
    ResizeSelection,
    ScaleSelection {
        percent: f64,
    },
    EditArrowGeometry {
        object_id: Option<String>,
        action: String,
    },
    EditShapeGeometry {
        object_id: Option<String>,
        action: String,
    },
    ApplyTextEdit {
        target: TextEditCommandTarget,
    },
    ApplyObjectSettings {
        settings: ObjectSettings,
    },
    ApplyObjectSettingsToSelection {
        #[serde(default, alias = "bondIds")]
        bond_ids: Vec<String>,
        #[serde(default, alias = "objectIds")]
        object_ids: Vec<String>,
        settings: ObjectSettingsPatch,
    },
    ApplyDocumentStyle {
        preset: String,
    },
    SetArrowGeometry {
        #[serde(alias = "objectId")]
        object_id: String,
        begin: CommandAnchor,
        end: CommandAnchor,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        curve: Option<f64>,
        #[serde(default, alias = "headStyle", skip_serializing_if = "Option::is_none")]
        head_style: Option<ArrowEndpointStyle>,
        #[serde(default, alias = "tailStyle", skip_serializing_if = "Option::is_none")]
        tail_style: Option<ArrowEndpointStyle>,
    },
    SetShapeGeometry {
        #[serde(alias = "objectId")]
        object_id: String,
        begin: CommandAnchor,
        end: CommandAnchor,
    },
    SetSpectrumData {
        #[serde(alias = "objectId")]
        object_id: String,
        spectrum: crate::SpectrumData,
    },
    ReplaceHoveredEndpointLabel {
        label: String,
    },
    AddOrbital {
        template: OrbitalTemplate,
        style: OrbitalStyle,
        phase: OrbitalPhase,
        color: String,
        center: CommandAnchor,
        end: CommandAnchor,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bond_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bond_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_spacing: Option<f64>,
}

impl ObjectSettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.bond_length.is_none()
            && self.line_width.is_none()
            && self.bold_width.is_none()
            && self.bond_spacing.is_none()
            && self.margin_width.is_none()
            && self.hash_spacing.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FocusedDeleteSource {
    DeleteTool,
    CommandKey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TextEditCommandTarget {
    TextObject {
        #[serde(skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
    },
    EndpointLabel {
        node_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "snapshotKind", rename_all = "kebab-case")]
pub enum HistorySnapshot {
    Document {
        before: Box<ChemSemaDocument>,
        #[serde(skip_serializing_if = "Option::is_none")]
        after: Option<Box<ChemSemaDocument>>,
    },
    SceneObjects {
        before_objects: Vec<SceneObject>,
        #[serde(skip_serializing_if = "Option::is_none")]
        after_objects: Option<Vec<SceneObject>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub command: EditorCommand,
    #[serde(flatten)]
    pub snapshot: HistorySnapshot,
}

impl HistoryEntry {
    pub fn new(command: EditorCommand, before: ChemSemaDocument) -> Self {
        Self {
            command,
            snapshot: HistorySnapshot::Document {
                before: Box::new(before),
                after: None,
            },
        }
    }

    pub fn new_scene_objects(command: EditorCommand, before_objects: Vec<SceneObject>) -> Self {
        Self {
            command,
            snapshot: HistorySnapshot::SceneObjects {
                before_objects,
                after_objects: None,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandTargets {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bonds: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<String>,
}

impl CommandTargets {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
            && self.bonds.is_empty()
            && self.objects.is_empty()
            && self.styles.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandTargetDelta {
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub created: CommandTargets,
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub updated: CommandTargets,
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub deleted: CommandTargets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub changed: bool,
    pub revision: u64,
    pub before_revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<EditorCommand>,
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub targets: CommandTargets,
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub created: CommandTargets,
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub updated: CommandTargets,
    #[serde(default, skip_serializing_if = "CommandTargets::is_empty")]
    pub deleted: CommandTargets,
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_depth: usize,
    pub redo_depth: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub diagnostics: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

impl CommandResult {
    pub fn unchanged(
        revision: u64,
        can_undo: bool,
        can_redo: bool,
        undo_depth: usize,
        redo_depth: usize,
    ) -> Self {
        Self {
            changed: false,
            revision,
            before_revision: revision,
            command: None,
            targets: CommandTargets::default(),
            created: CommandTargets::default(),
            updated: CommandTargets::default(),
            deleted: CommandTargets::default(),
            can_undo,
            can_redo,
            undo_depth,
            redo_depth,
            diagnostics: BTreeMap::new(),
            output: None,
        }
    }
}
