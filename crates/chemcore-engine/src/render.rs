use crate::{ChemcoreDocument, Point, DEFAULT_BOND_STROKE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderRole {
    DocumentBond,
    HoverEndpoint,
    HoverBondCenter,
    PreviewBond,
    PreviewEnd,
    SelectionBond,
    SelectionNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RenderPrimitive {
    Line {
        role: RenderRole,
        from: Point,
        to: Point,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
    Circle {
        role: RenderRole,
        center: Point,
        radius: f64,
        fill: String,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
    Polygon {
        role: RenderRole,
        points: Vec<Point>,
        fill: String,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
}

pub fn render_document(document: &ChemcoreDocument) -> Vec<RenderPrimitive> {
    let mut out = Vec::new();
    let Some(entry) = document.editable_fragment() else {
        return out;
    };
    for bond in &entry.fragment.bonds {
        if bond.order != 1 {
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
            role: RenderRole::DocumentBond,
            from: entry.world_point_for_node(begin),
            to: entry.world_point_for_node(end),
            stroke: "#000000".to_string(),
            stroke_width: if bond.stroke_width > 0.0 {
                bond.stroke_width
            } else {
                DEFAULT_BOND_STROKE
            },
        });
    }
    out
}
