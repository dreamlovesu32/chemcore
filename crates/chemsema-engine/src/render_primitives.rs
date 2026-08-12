use crate::{ImageCropRect, Point};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::{compact_polygon_points, polygon_area_signed, KNOCKOUT_FILL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderRole {
    DocumentBond,
    DocumentDiagnostic,
    DocumentGraphic,
    DocumentKnockout,
    DocumentMolecularColor,
    DocumentText,
    HoverEndpoint,
    HoverLabelGlyph,
    HoverObjectBox,
    HoverBondCenter,
    HoverArrowCenter,
    HoverArrowHandle,
    HoverShapeHandle,
    HoverTextBox,
    LinkFocusBox,
    LinkFocusConnector,
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
    Image {
        role: RenderRole,
        #[serde(rename = "objectId", default, skip_serializing_if = "Option::is_none")]
        object_id: Option<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        href: String,
        #[serde(
            rename = "sourceCrop",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        source_crop: Option<ImageCropRect>,
        #[serde(rename = "sourceWidth")]
        source_width: u32,
        #[serde(rename = "sourceHeight")]
        source_height: u32,
        #[serde(default = "default_opacity", skip_serializing_if = "is_one")]
        opacity: f64,
        #[serde(rename = "preserveAspectRatio", default)]
        preserve_aspect_ratio: bool,
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
        #[serde(rename = "bondId", default, skip_serializing_if = "Option::is_none")]
        bond_id: Option<String>,
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

impl RenderPrimitive {
    pub fn object_id(&self) -> Option<&str> {
        match self {
            Self::Line { object_id, .. }
            | Self::Circle { object_id, .. }
            | Self::Polygon { object_id, .. }
            | Self::Rect { object_id, .. }
            | Self::Ellipse { object_id, .. }
            | Self::Polyline { object_id, .. }
            | Self::Path { object_id, .. }
            | Self::FilledPath { object_id, .. }
            | Self::Image { object_id, .. }
            | Self::Text { object_id, .. } => object_id.as_deref(),
        }
    }

    pub fn role(&self) -> RenderRole {
        *self.role_ref()
    }

    pub fn role_mut(&mut self) -> &mut RenderRole {
        match self {
            Self::Line { role, .. }
            | Self::Circle { role, .. }
            | Self::Polygon { role, .. }
            | Self::Rect { role, .. }
            | Self::Ellipse { role, .. }
            | Self::Polyline { role, .. }
            | Self::Path { role, .. }
            | Self::FilledPath { role, .. }
            | Self::Image { role, .. }
            | Self::Text { role, .. } => role,
        }
    }

    fn role_ref(&self) -> &RenderRole {
        match self {
            Self::Line { role, .. }
            | Self::Circle { role, .. }
            | Self::Polygon { role, .. }
            | Self::Rect { role, .. }
            | Self::Ellipse { role, .. }
            | Self::Polyline { role, .. }
            | Self::Path { role, .. }
            | Self::FilledPath { role, .. }
            | Self::Image { role, .. }
            | Self::Text { role, .. } => role,
        }
    }
}

fn is_zero(value: &f64) -> bool {
    value.abs() <= crate::EPSILON
}

fn default_opacity() -> f64 {
    1.0
}

fn is_one(value: &f64) -> bool {
    (*value - 1.0).abs() <= crate::EPSILON
}

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
pub(super) fn push_node_polygon(
    out: &mut Vec<RenderPrimitive>,
    node_id: &str,
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
        node_id: Some(node_id.to_string()),
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

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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
        false,
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
    mut runs: Vec<crate::LabelRun>,
    object_id: Option<String>,
    rotate: f64,
    rotate_center: Option<Point>,
    preserve_lines: bool,
) {
    let underline_segments = chemdraw_text_underline_segments(
        x,
        y,
        font_size,
        text_anchor.as_deref(),
        fill.as_deref(),
        &runs,
        rotate,
        rotate_center,
    );
    for run in &mut runs {
        if run.underline == Some(true) {
            run.underline = Some(false);
        }
    }
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id: object_id.clone(),
        node_id: None,
        bond_id: None,
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
        preserve_lines,
        box_width: None,
        runs,
        rotate,
        rotate_center,
    });
    for (from, to, stroke) in underline_segments {
        out.push(RenderPrimitive::Line {
            role: RenderRole::DocumentText,
            object_id: object_id.clone(),
            bond_id: None,
            from,
            to,
            stroke,
            stroke_width: 0.4,
            dash_array: Vec::new(),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn chemdraw_text_underline_segments(
    x: f64,
    y: f64,
    default_font_size: f64,
    text_anchor: Option<&str>,
    default_fill: Option<&str>,
    runs: &[crate::LabelRun],
    rotate: f64,
    rotate_center: Option<Point>,
) -> Vec<(Point, Point, String)> {
    if !runs.iter().any(|run| run.underline == Some(true)) {
        return Vec::new();
    }
    let widths = runs
        .iter()
        .map(|run| {
            let font_size = run.font_size.unwrap_or(default_font_size)
                * crate::shared_script_scale_factor(run.script.as_deref());
            run.text
                .chars()
                .filter(|character| !matches!(character, '\r' | '\n'))
                .map(|character| crate::shared_estimated_char_width(character, font_size))
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let total_width = widths.iter().sum::<f64>();
    let mut cursor = match text_anchor {
        Some("middle") => x - total_width * 0.5,
        Some("end") => x - total_width,
        _ => x,
    };
    let center = rotate_center.unwrap_or(Point::new(x, y));
    let rotate_point = |point: Point| {
        if rotate.abs() <= crate::EPSILON {
            return point;
        }
        let radians = rotate.to_radians();
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        Point::new(
            center.x + dx * radians.cos() - dy * radians.sin(),
            center.y + dx * radians.sin() + dy * radians.cos(),
        )
    };
    let mut segments = Vec::new();
    for (run, width) in runs.iter().zip(widths) {
        if run.underline == Some(true) && width > crate::EPSILON {
            let base_font_size = run.font_size.unwrap_or(default_font_size);
            let baseline_shift = crate::shared_script_baseline_shift_em_for_face(
                run.script.as_deref(),
                run.font_weight,
                run.font_family.as_deref(),
                base_font_size,
            ) * base_font_size;
            let underline_y = y + baseline_shift + 0.8;
            segments.push((
                rotate_point(Point::new(cursor, underline_y)),
                rotate_point(Point::new(cursor + width, underline_y)),
                run.fill
                    .as_deref()
                    .or(default_fill)
                    .unwrap_or("#000000")
                    .to_string(),
            ));
        }
        cursor += width;
    }
    segments
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
    bond_id: Option<String>,
) {
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id,
        node_id,
        bond_id,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn chemdraw_underlines_use_fixed_geometry_and_run_advance() {
        let runs = vec![
            crate::LabelRun {
                text: "A".to_string(),
                font_size: Some(10.0),
                ..Default::default()
            },
            crate::LabelRun {
                text: "NH".to_string(),
                font_size: Some(10.0),
                fill: Some("#123456".to_string()),
                underline: Some(true),
                ..Default::default()
            },
        ];
        let prefix_width = crate::shared_estimated_char_width('A', 10.0);
        let underlined_width = crate::shared_estimated_char_width('N', 10.0)
            + crate::shared_estimated_char_width('H', 10.0);
        let total_width = prefix_width + underlined_width;

        let segments = chemdraw_text_underline_segments(
            50.0,
            20.0,
            12.0,
            Some("middle"),
            Some("#000000"),
            &runs,
            0.0,
            None,
        );

        assert_eq!(segments.len(), 1);
        let (from, to, stroke) = &segments[0];
        assert_close(from.x, 50.0 - total_width * 0.5 + prefix_width);
        assert_close(to.x, from.x + underlined_width);
        assert_close(from.y, 20.8);
        assert_close(to.y, 20.8);
        assert_eq!(stroke, "#123456");
    }

    #[test]
    fn push_text_emits_underlines_as_lines_and_clears_text_decoration() {
        let mut primitives = Vec::new();
        push_text(
            &mut primitives,
            5.0,
            10.0,
            None,
            "N".to_string(),
            10.0,
            None,
            Some("#000000".to_string()),
            None,
            vec![crate::LabelRun {
                text: "N".to_string(),
                underline: Some(true),
                ..Default::default()
            }],
            Some("text-1".to_string()),
        );

        assert_eq!(primitives.len(), 2);
        match &primitives[0] {
            RenderPrimitive::Text { runs, .. } => assert_eq!(runs[0].underline, Some(false)),
            other => panic!("expected text primitive, got {other:?}"),
        }
        match &primitives[1] {
            RenderPrimitive::Line {
                stroke_width,
                from,
                to,
                ..
            } => {
                assert_close(*stroke_width, 0.4);
                assert_close(from.y, 10.8);
                assert_close(to.y, 10.8);
            }
            other => panic!("expected underline line primitive, got {other:?}"),
        }
    }
}
