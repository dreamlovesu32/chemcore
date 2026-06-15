use crate::Point;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::{compact_polygon_points, polygon_area_signed, KNOCKOUT_FILL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderRole {
    DocumentBond,
    DocumentGraphic,
    DocumentKnockout,
    DocumentText,
    HoverEndpoint,
    HoverLabelGlyph,
    HoverBondCenter,
    HoverArrowCenter,
    HoverArrowHandle,
    HoverShapeHandle,
    HoverTextBox,
    PreviewBond,
    PreviewEnd,
    SelectionBox,
    SelectionBond,
    SelectionBondDot,
    SelectionCenterCross,
    SelectionNode,
    SelectionResizeHandle,
    SelectionRotateGlyph,
    SelectionRotateHandle,
    SelectionRotateStem,
    SelectionTextBox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RenderPrimitive {
    Line {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        from: Point,
        to: Point,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
    },
    Circle {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "nodeId", default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        center: Point,
        radius: f64,
        fill: String,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
    Polygon {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "nodeId", default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        points: Vec<Point>,
        fill: String,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
    },
    Rect {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "nodeId", default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rx: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ry: Option<f64>,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
        #[serde(
            rename = "fillGradient",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        fill_gradient: Option<JsonValue>,
    },
    Ellipse {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        center: Point,
        rx: f64,
        ry: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotate: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
        #[serde(
            rename = "fillGradient",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        fill_gradient: Option<JsonValue>,
    },
    Polyline {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        points: Vec<Point>,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
        #[serde(rename = "lineCap", default, skip_serializing_if = "Option::is_none")]
        line_cap: Option<String>,
        #[serde(rename = "lineJoin", default, skip_serializing_if = "Option::is_none")]
        line_join: Option<String>,
    },
    Path {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        d: String,
        #[serde(default)]
        points: Vec<Point>,
        stroke: String,
        #[serde(rename = "strokeWidth")]
        stroke_width: f64,
        #[serde(rename = "dashArray", default, skip_serializing_if = "Vec::is_empty")]
        dash_array: Vec<f64>,
        #[serde(rename = "lineCap", default, skip_serializing_if = "Option::is_none")]
        line_cap: Option<String>,
        #[serde(rename = "lineJoin", default, skip_serializing_if = "Option::is_none")]
        line_join: Option<String>,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotate: f64,
        #[serde(
            rename = "rotateCenter",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        rotate_center: Option<Point>,
    },
    FilledPath {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "nodeId", default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
        d: String,
        #[serde(default)]
        points: Vec<Point>,
        fill: String,
        #[serde(rename = "fillRule", default, skip_serializing_if = "Option::is_none")]
        fill_rule: Option<String>,
        #[serde(rename = "clipPathD", default, skip_serializing_if = "Option::is_none")]
        clip_path_d: Option<String>,
        #[serde(rename = "clipRule", default, skip_serializing_if = "Option::is_none")]
        clip_rule: Option<String>,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotate: f64,
        #[serde(
            rename = "rotateCenter",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        rotate_center: Option<Point>,
    },
    Text {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        #[serde(rename = "nodeId", default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        x: f64,
        y: f64,
        #[serde(
            rename = "baselineOffset",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        baseline_offset: Option<f64>,
        #[serde(
            rename = "dominantBaseline",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        dominant_baseline: Option<String>,
        text: String,
        #[serde(rename = "fontSize")]
        font_size: f64,
        #[serde(
            rename = "fontFamily",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        font_family: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        #[serde(
            rename = "textAnchor",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        text_anchor: Option<String>,
        #[serde(
            rename = "lineHeight",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        line_height: Option<f64>,
        #[serde(rename = "preserveLines", default)]
        preserve_lines: bool,
        #[serde(rename = "boxWidth", default, skip_serializing_if = "Option::is_none")]
        box_width: Option<f64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        runs: Vec<crate::LabelRun>,
        #[serde(default, skip_serializing_if = "is_zero")]
        rotate: f64,
        #[serde(
            rename = "rotateCenter",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        rotate_center: Option<Point>,
    },
}

fn is_zero(value: &f64) -> bool {
    value.abs() <= crate::EPSILON
}

pub(super) fn push_line(
    out: &mut Vec<RenderPrimitive>,
    from: Point,
    to: Point,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Line {
        role,
        object_id,
        bond_id: None,
        from,
        to,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_polygon(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polygon {
        role,
        object_id,
        node_id: None,
        bond_id: None,
        points,
        fill: fill.to_string(),
        stroke: stroke.to_string(),
        stroke_width,
    });
}

pub(super) fn push_bond_polygon(
    out: &mut Vec<RenderPrimitive>,
    bond_id: &str,
    points: Vec<Point>,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polygon {
        role: RenderRole::DocumentBond,
        object_id,
        node_id: None,
        bond_id: Some(bond_id.to_string()),
        points,
        fill: fill.to_string(),
        stroke: stroke.to_string(),
        stroke_width,
    });
}

pub(super) fn push_knockout_polygon(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    object_id: Option<String>,
) {
    push_knockout_polygon_with_ids(out, points, object_id, None, None);
}

pub(super) fn push_bond_knockout_polygon(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    object_id: Option<String>,
    bond_id: String,
) {
    push_knockout_polygon_with_ids(out, points, object_id, None, Some(bond_id));
}

pub(super) fn push_label_knockout_polygon(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    object_id: Option<String>,
    node_id: String,
) {
    push_knockout_polygon_with_ids(out, points, object_id, Some(node_id), None);
}

fn push_knockout_polygon_with_ids(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    object_id: Option<String>,
    node_id: Option<String>,
    bond_id: Option<String>,
) {
    let points = compact_polygon_points(points);
    if points.len() < 3 || polygon_area_signed(&points).abs() <= 1.0e-4 {
        return;
    }
    out.push(RenderPrimitive::Polygon {
        role: RenderRole::DocumentKnockout,
        object_id,
        node_id,
        bond_id,
        points,
        fill: KNOCKOUT_FILL.to_string(),
        stroke: "none".to_string(),
        stroke_width: 0.0,
    });
}

pub(super) fn push_polyline(
    out: &mut Vec<RenderPrimitive>,
    points: Vec<Point>,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_cap: Option<String>,
    line_join: Option<String>,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Polyline {
        role,
        object_id,
        bond_id: None,
        points,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
        line_cap,
        line_join,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_path(
    out: &mut Vec<RenderPrimitive>,
    d: String,
    points: Vec<Point>,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    line_cap: Option<String>,
    line_join: Option<String>,
    role: RenderRole,
    object_id: Option<String>,
) {
    out.push(RenderPrimitive::Path {
        role,
        object_id,
        bond_id: None,
        d,
        points,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
        line_cap,
        line_join,
        rotate: 0.0,
        rotate_center: None,
    });
}

pub(super) fn push_text(
    out: &mut Vec<RenderPrimitive>,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    text: String,
    font_size: f64,
    font_family: Option<String>,
    fill: Option<String>,
    text_anchor: Option<String>,
    runs: Vec<crate::LabelRun>,
    object_id: Option<String>,
) {
    push_text_rotated(
        out,
        x,
        y,
        baseline_offset,
        text,
        font_size,
        font_family,
        fill,
        text_anchor,
        runs,
        object_id,
        0.0,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_text_rotated(
    out: &mut Vec<RenderPrimitive>,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    text: String,
    font_size: f64,
    font_family: Option<String>,
    fill: Option<String>,
    text_anchor: Option<String>,
    runs: Vec<crate::LabelRun>,
    object_id: Option<String>,
    rotate: f64,
    rotate_center: Option<Point>,
) {
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id,
        node_id: None,
        x,
        y,
        baseline_offset,
        dominant_baseline: None,
        text,
        font_size,
        font_family,
        fill,
        text_anchor,
        line_height: None,
        preserve_lines: false,
        box_width: None,
        runs,
        rotate,
        rotate_center,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_text_for_node(
    out: &mut Vec<RenderPrimitive>,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    text: String,
    font_size: f64,
    font_family: Option<String>,
    fill: Option<String>,
    text_anchor: Option<String>,
    runs: Vec<crate::LabelRun>,
    object_id: Option<String>,
    node_id: Option<String>,
) {
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id,
        node_id,
        x,
        y,
        baseline_offset,
        dominant_baseline: None,
        text,
        font_size,
        font_family,
        fill,
        text_anchor,
        line_height: None,
        preserve_lines: false,
        box_width: None,
        runs,
        rotate: 0.0,
        rotate_center: None,
    });
}
