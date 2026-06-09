use std::fmt::Write;

use crate::{render_document, ChemcoreDocument, LabelRun, Point, RenderPrimitive};

const EXPORT_MARGIN: f64 = 8.0;
const DEFAULT_TEXT_LINE_HEIGHT: f64 = 12.0;
const TEXT_INK_HORIZONTAL_PAD_EM: f64 = 0.16;
const TEXT_GDI_DESCENT_EM: f64 = 0.59;
const TEXT_GDI_LINE_BOX_EM: f64 = 1.45;

pub fn document_to_svg(document: &ChemcoreDocument) -> String {
    let primitives = render_document(document);
    primitives_to_svg(
        &primitives,
        Some((document.document.page.width, document.document.page.height)),
    )
}

pub fn primitives_to_svg(
    primitives: &[RenderPrimitive],
    fallback_size: Option<(f64, f64)>,
) -> String {
    let bounds = primitives
        .iter()
        .filter(|primitive| visible_in_document_svg(primitive))
        .fold(None, extend_bounds_for_primitive)
        .unwrap_or_else(|| {
            let (width, height) = fallback_size.unwrap_or((100.0, 100.0));
            [0.0, 0.0, width.max(1.0), height.max(1.0)]
        });
    let min_x = bounds[0] - EXPORT_MARGIN;
    let min_y = bounds[1] - EXPORT_MARGIN;
    let width = (bounds[2] - bounds[0] + EXPORT_MARGIN * 2.0).max(1.0);
    let height = (bounds[3] - bounds[1] + EXPORT_MARGIN * 2.0).max(1.0);

    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">"#,
        fmt_num(width),
        fmt_num(height),
        fmt_num(min_x),
        fmt_num(min_y),
        fmt_num(width),
        fmt_num(height)
    )
    .expect("write svg root");

    let mut defs = SvgDefs::default();
    let mut body = String::new();
    for primitive in primitives {
        if visible_in_document_svg(primitive) {
            write_primitive_svg(&mut body, &mut defs, primitive);
        }
    }
    if !defs.body.is_empty() {
        svg.push_str("  <defs>\n");
        svg.push_str(&defs.body);
        svg.push_str("  </defs>\n");
    }
    svg.push_str(&body);
    svg.push_str("</svg>\n");
    svg
}

pub fn primitives_to_svg_viewbox(
    primitives: &[RenderPrimitive],
    view_box: [f64; 4],
    class_name: Option<&str>,
) -> String {
    let mut svg = String::new();
    let class_attr = class_name
        .map(|class_name| format!(r#" class="{}""#, escape_attr(class_name)))
        .unwrap_or_default();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg"{} viewBox="{} {} {} {}" aria-hidden="true">"#,
        class_attr,
        fmt_num(view_box[0]),
        fmt_num(view_box[1]),
        fmt_num(view_box[2]),
        fmt_num(view_box[3])
    )
    .expect("write svg root");

    let mut defs = SvgDefs::default();
    let mut body = String::new();
    for primitive in primitives {
        if visible_in_document_svg(primitive) {
            write_primitive_svg(&mut body, &mut defs, primitive);
        }
    }
    if !defs.body.is_empty() {
        svg.push_str("  <defs>\n");
        svg.push_str(&defs.body);
        svg.push_str("  </defs>\n");
    }
    svg.push_str(&body);
    svg.push_str("</svg>\n");
    svg
}

#[derive(Default)]
struct SvgDefs {
    next_id: usize,
    body: String,
}

fn visible_in_document_svg(primitive: &RenderPrimitive) -> bool {
    !matches!(
        primitive,
        RenderPrimitive::Rect {
            role: crate::RenderRole::DocumentKnockout,
            ..
        } | RenderPrimitive::Polygon {
            role: crate::RenderRole::DocumentKnockout,
            node_id: Some(_),
            ..
        }
    )
}

fn extend_bounds_for_primitive(
    bounds: Option<[f64; 4]>,
    primitive: &RenderPrimitive,
) -> Option<[f64; 4]> {
    let mut bounds = bounds;
    match primitive {
        RenderPrimitive::Line {
            from,
            to,
            stroke_width,
            ..
        } => {
            extend_bounds_for_point(&mut bounds, *from, *stroke_width * 0.5);
            extend_bounds_for_point(&mut bounds, *to, *stroke_width * 0.5);
        }
        RenderPrimitive::Circle {
            center,
            radius,
            stroke_width,
            ..
        } => {
            extend_bounds_for_point(&mut bounds, *center, radius + stroke_width * 0.5);
        }
        RenderPrimitive::Polygon {
            points,
            stroke_width,
            ..
        }
        | RenderPrimitive::Polyline {
            points,
            stroke_width,
            ..
        } => {
            for point in points {
                extend_bounds_for_point(&mut bounds, *point, *stroke_width * 0.5);
            }
        }
        RenderPrimitive::Path {
            points,
            stroke_width,
            ..
        } => {
            for point in points {
                extend_bounds_for_point(&mut bounds, *point, *stroke_width * 0.5);
            }
        }
        RenderPrimitive::FilledPath { points, .. } => {
            for point in points {
                extend_bounds_for_point(&mut bounds, *point, 0.0);
            }
        }
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            stroke_width,
            ..
        } => {
            extend_bounds_for_point(&mut bounds, Point::new(*x, *y), *stroke_width * 0.5);
            extend_bounds_for_point(
                &mut bounds,
                Point::new(x + width, y + height),
                *stroke_width * 0.5,
            );
        }
        RenderPrimitive::Ellipse {
            center,
            rx,
            ry,
            stroke_width,
            ..
        } => {
            extend_bounds_for_point(
                &mut bounds,
                Point::new(center.x - rx, center.y - ry),
                *stroke_width * 0.5,
            );
            extend_bounds_for_point(
                &mut bounds,
                Point::new(center.x + rx, center.y + ry),
                *stroke_width * 0.5,
            );
        }
        RenderPrimitive::Text {
            x,
            y,
            font_size,
            line_height,
            box_width,
            text,
            runs,
            text_anchor,
            dominant_baseline,
            rotate,
            rotate_center,
            ..
        } => {
            let measured_width = estimate_text_width(text, runs, *font_size);
            let width = box_width.unwrap_or(0.0).max(measured_width);
            let max_font_size = estimate_text_max_font_size(*font_size, runs);
            let line_count = estimate_text_line_count(text, runs) as f64;
            let line_height = line_height
                .unwrap_or(max_font_size * TEXT_GDI_LINE_BOX_EM)
                .max(DEFAULT_TEXT_LINE_HEIGHT)
                .max(max_font_size)
                .max(0.01);
            let right_pad = max_font_size * TEXT_INK_HORIZONTAL_PAD_EM;
            let left_pad = right_pad;
            let min_x = match text_anchor.as_deref() {
                Some("middle") => x - width * 0.5,
                Some("end") => x - width,
                _ => *x,
            };
            let (min_y, max_y) =
                if matches!(dominant_baseline.as_deref(), Some("central" | "middle")) {
                    let block_height = line_height * line_count.max(1.0);
                    (y - block_height * 0.5, y + block_height * 0.5)
                } else {
                    (
                        y - max_font_size,
                        y + (line_count - 1.0).max(0.0) * line_height
                            + max_font_size * TEXT_GDI_DESCENT_EM,
                    )
                };
            let top_left = Point::new(min_x - left_pad, min_y);
            let bottom_right = Point::new(min_x + width + right_pad, max_y);
            if rotate.abs() > crate::EPSILON {
                let center = rotate_center.unwrap_or(Point::new(*x, *y));
                for point in rotated_box_points(top_left, bottom_right, center, *rotate) {
                    extend_bounds_for_point(&mut bounds, point, 0.0);
                }
            } else {
                extend_bounds_for_point(&mut bounds, top_left, 0.0);
                extend_bounds_for_point(&mut bounds, bottom_right, 0.0);
            }
        }
    }
    bounds
}

fn extend_bounds_for_point(bounds: &mut Option<[f64; 4]>, point: Point, pad: f64) {
    let next = [point.x - pad, point.y - pad, point.x + pad, point.y + pad];
    *bounds = Some(match *bounds {
        Some([min_x, min_y, max_x, max_y]) => [
            min_x.min(next[0]),
            min_y.min(next[1]),
            max_x.max(next[2]),
            max_y.max(next[3]),
        ],
        None => next,
    });
}

fn rotated_box_points(
    top_left: Point,
    bottom_right: Point,
    center: Point,
    rotate: f64,
) -> [Point; 4] {
    [
        rotate_point_around(top_left, center, rotate),
        rotate_point_around(Point::new(bottom_right.x, top_left.y), center, rotate),
        rotate_point_around(bottom_right, center, rotate),
        rotate_point_around(Point::new(top_left.x, bottom_right.y), center, rotate),
    ]
}

fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
    if degrees.abs() <= crate::EPSILON {
        return point;
    }
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

fn rotate_transform_attr(rotate: f64, center: Option<&Point>) -> String {
    let Some(center) = center else {
        return String::new();
    };
    if rotate.abs() <= crate::EPSILON {
        return String::new();
    }
    format!(
        r#" transform="rotate({} {} {})""#,
        fmt_num(rotate),
        fmt_num(center.x),
        fmt_num(center.y)
    )
}

fn write_primitive_svg(out: &mut String, defs: &mut SvgDefs, primitive: &RenderPrimitive) {
    match primitive {
        RenderPrimitive::Line {
            from,
            to,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => {
            writeln!(
                out,
                r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"{} />"#,
                fmt_num(from.x),
                fmt_num(from.y),
                fmt_num(to.x),
                fmt_num(to.y),
                escape_attr(stroke),
                fmt_num(*stroke_width),
                dash_attr(dash_array)
            )
            .expect("write line");
        }
        RenderPrimitive::Circle {
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => {
            writeln!(
                out,
                r#"  <circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
                fmt_num(center.x),
                fmt_num(center.y),
                fmt_num(*radius),
                escape_attr(fill),
                escape_attr(stroke),
                fmt_num(*stroke_width)
            )
            .expect("write circle");
        }
        RenderPrimitive::Polygon {
            points,
            fill,
            stroke,
            stroke_width,
            ..
        } => {
            writeln!(
                out,
                r#"  <polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
                points_attr(points),
                escape_attr(fill),
                if *stroke_width > 0.0 {
                    escape_attr(stroke)
                } else {
                    "none".to_string()
                },
                fmt_num(*stroke_width)
            )
            .expect("write polygon");
        }
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            rx,
            ry,
            dash_array,
            fill_gradient,
            ..
        } => {
            let fill = gradient_fill(defs, fill_gradient)
                .unwrap_or_else(|| fill.clone().unwrap_or_else(|| "none".to_string()));
            writeln!(
                out,
                r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"{}{}{} />"#,
                fmt_num(*x),
                fmt_num(*y),
                fmt_num(*width),
                fmt_num(*height),
                escape_attr(&fill),
                escape_attr(stroke.as_deref().unwrap_or("none")),
                fmt_num(*stroke_width),
                optional_num_attr("rx", *rx),
                optional_num_attr("ry", *ry),
                dash_attr(dash_array)
            )
            .expect("write rect");
        }
        RenderPrimitive::Ellipse {
            center,
            rx,
            ry,
            rotate,
            fill,
            stroke,
            stroke_width,
            dash_array,
            fill_gradient,
            ..
        } => {
            let fill = gradient_fill(defs, fill_gradient)
                .unwrap_or_else(|| fill.clone().unwrap_or_else(|| "none".to_string()));
            let transform = if rotate.abs() > crate::EPSILON {
                format!(
                    r#" transform="rotate({} {} {})""#,
                    fmt_num(*rotate),
                    fmt_num(center.x),
                    fmt_num(center.y)
                )
            } else {
                String::new()
            };
            writeln!(
                out,
                r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}"{}{} />"#,
                fmt_num(center.x),
                fmt_num(center.y),
                fmt_num(*rx),
                fmt_num(*ry),
                escape_attr(&fill),
                escape_attr(stroke.as_deref().unwrap_or("none")),
                fmt_num(*stroke_width),
                dash_attr(dash_array),
                transform
            )
            .expect("write ellipse");
        }
        RenderPrimitive::Polyline {
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => {
            writeln!(
                out,
                r#"  <polyline points="{}" fill="none" stroke="{}" stroke-width="{}"{}{}{} />"#,
                points_attr(points),
                escape_attr(stroke),
                fmt_num(*stroke_width),
                dash_attr(dash_array),
                optional_str_attr("stroke-linecap", line_cap.as_deref()),
                optional_str_attr("stroke-linejoin", line_join.as_deref())
            )
            .expect("write polyline");
        }
        RenderPrimitive::Path {
            d,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            rotate,
            rotate_center,
            ..
        } => {
            let transform = rotate_transform_attr(*rotate, rotate_center.as_ref());
            writeln!(
                out,
                r#"  <path d="{}" fill="none" stroke="{}" stroke-width="{}"{}{}{}{} />"#,
                escape_attr(d),
                escape_attr(stroke),
                fmt_num(*stroke_width),
                dash_attr(dash_array),
                optional_str_attr("stroke-linecap", line_cap.as_deref()),
                optional_str_attr("stroke-linejoin", line_join.as_deref()),
                transform
            )
            .expect("write path");
        }
        RenderPrimitive::FilledPath {
            d,
            fill,
            fill_rule,
            clip_path_d,
            clip_rule,
            rotate,
            rotate_center,
            ..
        } => {
            let clip_attr = clip_path_attr(defs, clip_path_d.as_deref(), clip_rule.as_deref());
            let transform = rotate_transform_attr(*rotate, rotate_center.as_ref());
            writeln!(
                out,
                r#"  <path d="{}" fill="{}" stroke="none"{}{}{} />"#,
                escape_attr(d),
                escape_attr(fill),
                optional_str_attr("fill-rule", fill_rule.as_deref()),
                clip_attr,
                transform
            )
            .expect("write filled path");
        }
        RenderPrimitive::Text {
            x,
            y,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            dominant_baseline,
            runs,
            rotate,
            rotate_center,
            ..
        } => {
            let center = rotate_center.unwrap_or(Point::new(*x, *y));
            let transform = rotate_transform_attr(*rotate, Some(&center));
            write!(
                out,
                r#"  <text x="{}" y="{}" font-size="{}" dominant-baseline="{}" text-anchor="{}" fill="{}"{}{}>"#,
                fmt_num(*x),
                fmt_num(*y),
                fmt_num(*font_size),
                escape_attr(dominant_baseline.as_deref().unwrap_or("alphabetic")),
                escape_attr(text_anchor.as_deref().unwrap_or("start")),
                escape_attr(fill.as_deref().unwrap_or("#000000")),
                optional_str_attr("font-family", font_family.as_deref()),
                transform
            )
            .expect("write text start");
            if runs.is_empty() {
                out.push_str(&escape_text(text));
            } else {
                for run in runs {
                    write_text_run(out, run, *font_size);
                }
            }
            out.push_str("</text>\n");
        }
    }
}

fn write_text_run(out: &mut String, run: &LabelRun, fallback_font_size: f64) {
    let is_sub = run.script.as_deref() == Some("subscript");
    let is_super = run.script.as_deref() == Some("superscript");
    let font_size = run.font_size.unwrap_or(fallback_font_size)
        * if is_sub || is_super {
            crate::shared_script_scale_factor(run.script.as_deref())
        } else {
            1.0
        };
    let baseline_shift = if is_sub || is_super {
        let base_font_size = run.font_size.unwrap_or(fallback_font_size);
        let shift =
            crate::shared_svg_script_baseline_shift_em(run.script.as_deref(), run.font_weight);
        Some(base_font_size * shift)
    } else {
        None
    };
    write!(
        out,
        r#"<tspan{}{}{}{}{}{}{}>{}</tspan>"#,
        optional_num_attr("font-size", Some(font_size)),
        optional_str_attr("font-family", run.font_family.as_deref()),
        optional_str_attr("fill", run.fill.as_deref()),
        optional_u32_attr("font-weight", run.font_weight),
        optional_str_attr("font-style", run.font_style.as_deref()),
        optional_str_attr(
            "text-decoration",
            run.underline.filter(|value| *value).map(|_| "underline")
        ),
        optional_num_attr("baseline-shift", baseline_shift),
        escape_text(&run.text)
    )
    .expect("write text run");
}

fn clip_path_attr(
    defs: &mut SvgDefs,
    clip_path_d: Option<&str>,
    clip_rule: Option<&str>,
) -> String {
    let Some(clip_path_d) = clip_path_d else {
        return String::new();
    };
    let id = next_def_id(defs, "clip");
    writeln!(
        defs.body,
        r#"    <clipPath id="{}"><path d="{}" clip-rule="{}" /></clipPath>"#,
        id,
        escape_attr(clip_path_d),
        escape_attr(clip_rule.unwrap_or("nonzero"))
    )
    .expect("write clip path");
    format!(r#" clip-path="url(#{id})""#)
}

fn gradient_fill(defs: &mut SvgDefs, gradient: &Option<serde_json::Value>) -> Option<String> {
    let gradient = gradient.as_ref()?;
    let stops = gradient.get("stops")?.as_array()?;
    if stops.is_empty() {
        return None;
    }
    let id = next_def_id(defs, "gradient");
    writeln!(
        defs.body,
        r#"    <linearGradient id="{}" x1="{}" y1="{}" x2="{}" y2="{}">"#,
        id,
        escape_attr(
            gradient
                .get("x1")
                .and_then(|value| value.as_str())
                .unwrap_or("0%")
        ),
        escape_attr(
            gradient
                .get("y1")
                .and_then(|value| value.as_str())
                .unwrap_or("0%")
        ),
        escape_attr(
            gradient
                .get("x2")
                .and_then(|value| value.as_str())
                .unwrap_or("100%")
        ),
        escape_attr(
            gradient
                .get("y2")
                .and_then(|value| value.as_str())
                .unwrap_or("100%")
        ),
    )
    .expect("write gradient");
    for stop in stops {
        writeln!(
            defs.body,
            r#"      <stop offset="{}" stop-color="{}" />"#,
            escape_attr(
                stop.get("offset")
                    .and_then(|value| value.as_str())
                    .unwrap_or("0%")
            ),
            escape_attr(
                stop.get("color")
                    .and_then(|value| value.as_str())
                    .unwrap_or("#000000")
            ),
        )
        .expect("write gradient stop");
    }
    defs.body.push_str("    </linearGradient>\n");
    Some(format!("url(#{id})"))
}

fn next_def_id(defs: &mut SvgDefs, prefix: &str) -> String {
    defs.next_id += 1;
    format!("chemcore-{prefix}-{}", defs.next_id)
}

fn points_attr(points: &[Point]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", fmt_num(point.x), fmt_num(point.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn dash_attr(values: &[f64]) -> String {
    if values.is_empty() {
        String::new()
    } else {
        format!(
            r#" stroke-dasharray="{}""#,
            values
                .iter()
                .map(|value| fmt_num(*value))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

fn optional_str_attr(name: &str, value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(|value| format!(r#" {name}="{}""#, escape_attr(value)))
        .unwrap_or_default()
}

fn optional_num_attr(name: &str, value: Option<f64>) -> String {
    value
        .map(|value| format!(r#" {name}="{}""#, fmt_num(value)))
        .unwrap_or_default()
}

fn optional_u32_attr(name: &str, value: Option<u32>) -> String {
    value
        .map(|value| format!(r#" {name}="{value}""#))
        .unwrap_or_default()
}

fn estimate_text_width(text: &str, runs: &[LabelRun], fallback_font_size: f64) -> f64 {
    if !runs.is_empty() {
        let mut max_width = 0.0;
        let mut line_width = 0.0;
        for run in runs {
            let font_size = run.font_size.unwrap_or(fallback_font_size)
                * crate::shared_script_scale_factor(run.script.as_deref());
            for character in run.text.chars() {
                match character {
                    '\n' => {
                        max_width = f64::max(max_width, line_width);
                        line_width = 0.0;
                    }
                    '\r' => {}
                    _ => line_width += crate::shared_estimated_char_width(character, font_size),
                }
            }
        }
        return f64::max(max_width, line_width);
    }
    text.lines()
        .map(|line| estimate_text_line_width(line, fallback_font_size))
        .fold(0.0, f64::max)
}

fn estimate_text_line_width(text: &str, font_size: f64) -> f64 {
    text.chars()
        .filter(|character| *character != '\r')
        .map(|character| crate::shared_estimated_char_width(character, font_size))
        .sum()
}

fn estimate_text_line_count(text: &str, runs: &[LabelRun]) -> usize {
    if !runs.is_empty() {
        return runs
            .iter()
            .map(|run| {
                run.text
                    .chars()
                    .filter(|character| *character == '\n')
                    .count()
            })
            .sum::<usize>()
            + 1;
    }
    text.lines().count().max(1)
}

fn estimate_text_max_font_size(fallback_font_size: f64, runs: &[LabelRun]) -> f64 {
    runs.iter()
        .map(|run| {
            run.font_size.unwrap_or(fallback_font_size)
                * crate::shared_script_scale_factor(run.script.as_deref())
        })
        .fold(fallback_font_size, f64::max)
}

fn fmt_num(value: f64) -> String {
    let value = crate::round6(value);
    if value.fract().abs() <= crate::EPSILON {
        format!("{value:.0}")
    } else {
        let mut text = format!("{value:.6}");
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    }
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
