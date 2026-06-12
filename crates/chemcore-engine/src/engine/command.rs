use crate::{
    ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, BondAnchor,
    BondVariant, BracketKind, ChemcoreDocument, ObjectSettings, OrbitalPhase, OrbitalStyle,
    OrbitalTemplate, Point, ShapeKind, ShapeStyle,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandAnchor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    pub x: f64,
    pub y: f64,
}

impl From<&BondAnchor> for CommandAnchor {
    fn from(anchor: &BondAnchor) -> Self {
        Self {
            node_id: anchor.node_id.clone(),
            x: anchor.point.x,
            y: anchor.point.y,
        }
    }
}

impl From<Point> for CommandAnchor {
    fn from(point: Point) -> Self {
        Self {
            node_id: None,
            x: point.x,
            y: point.y,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum EditorCommand {
    Undo,
    Redo,
    AddBond {
        begin: CommandAnchor,
        end: CommandAnchor,
        order: u8,
        variant: BondVariant,
    },
    AddArrow {
        begin: CommandAnchor,
        end: CommandAnchor,
        variant: ArrowVariant,
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        head_style: ArrowEndpointStyle,
        tail_style: ArrowEndpointStyle,
        head: bool,
        tail: bool,
        bold: bool,
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
        atomic_number: u8,
        center: CommandAnchor,
    },
    ReplaceNodeLabel {
        node_id: String,
        label: String,
    },
    MoveTlcSpot {
        object_id: String,
        lane_index: usize,
        spot_index: usize,
        before_rf: f64,
    },
    ApplyArrowStyle {
        object_ids: Vec<String>,
        variant: ArrowVariant,
        head_size: ArrowHeadSize,
        curve: ArrowCurve,
        head_style: ArrowEndpointStyle,
        tail_style: ArrowEndpointStyle,
        head: bool,
        tail: bool,
        bold: bool,
        no_go: ArrowNoGo,
    },
    CycleBondStyle {
        bond_id: String,
        variant: BondVariant,
    },
    DeleteSelection,
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
    },
    ApplySelectionArrange {
        command: String,
    },
    ApplySelectionOrder {
        object_ids: Vec<String>,
        command: String,
    },
    ApplySelectionColor {
        color: String,
    },
    ApplyShapeStyle {
        object_ids: Vec<String>,
        style: String,
    },
    ApplyBracketKind {
        object_ids: Vec<String>,
        kind: String,
    },
    ApplyOrbitalTemplate {
        object_ids: Vec<String>,
        template: String,
    },
    ApplyOrbitalStyle {
        object_ids: Vec<String>,
        style: String,
    },
    ApplyOrbitalPhase {
        object_ids: Vec<String>,
        phase: String,
    },
    ApplyLineStyle {
        object_ids: Vec<String>,
        style: String,
    },
    ApplyBondStyle {
        bond_ids: Vec<String>,
        style: String,
    },
    ApplyTextStyle {
        text_object_ids: Vec<String>,
        label_node_ids: Vec<String>,
        node_ids: Vec<String>,
        command: String,
        value: String,
    },
    SetChemicalCheckForSelection {
        enabled: bool,
    },
    ExpandLabelsInSelection,
    CenterSelectionOnPage,
    GroupSelection {
        object_ids: Vec<String>,
    },
    UngroupSelection {
        object_ids: Vec<String>,
    },
    JoinSelection,
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
        bond_ids: Vec<String>,
        object_ids: Vec<String>,
        settings: ObjectSettingsPatch,
    },
    ApplyDocumentStyle {
        preset: String,
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
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub command: EditorCommand,
    pub before: ChemcoreDocument,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<ChemcoreDocument>,
}

impl HistoryEntry {
    pub fn new(command: EditorCommand, before: ChemcoreDocument) -> Self {
        Self {
            command,
            before,
            after: None,
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
        }
    }
}
