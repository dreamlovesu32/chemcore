use crate::{ChemcoreDocument, Point, DEFAULT_BOND_STROKE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RenderPrimitive {
    Line {
        from: Point,
        to: Point,
        stroke: String,
        stroke_width: f64,
    },
    Circle {
        center: Point,
        radius: f64,
        fill: String,
        stroke: String,
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
