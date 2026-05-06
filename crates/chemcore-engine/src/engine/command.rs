use crate::{
    ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, BondAnchor,
    BondVariant, ChemcoreDocument, Point, ShapeKind, ShapeStyle,
};
use serde::{Deserialize, Serialize};

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
    ApplySelectionColor {
        color: String,
    },
    MoveSelection,
    RotateSelection,
    ApplyTextEdit {
        target: TextEditCommandTarget,
    },
    ReplaceHoveredEndpointLabel {
        label: String,
    },
    LegacyMutation {
        label: String,
    },
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
