use super::*;

fn arrow_head_points(from: Point, to: Point, arrow_head: ArrowHeadGeometry) -> Vec<Point> {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let head_length = arrow_head.length;
    let head_half_width = arrow_head.width.max(0.0) + 0.05;
    let notch_length = arrow_head.center_length.max(0.0).min(head_length);
    let tip = to;
    let left = Point::new(
        to.x - unit.x * head_length + normal.x * head_half_width,
        to.y - unit.y * head_length + normal.y * head_half_width,
    );
    let right = Point::new(
        to.x - unit.x * head_length - normal.x * head_half_width,
        to.y - unit.y * head_length - normal.y * head_half_width,
    );
    if notch_length < head_length - 0.2 {
        let notch = Point::new(to.x - unit.x * notch_length, to.y - unit.y * notch_length);
        vec![tip, left, notch, right]
    } else {
        vec![tip, left, right]
    }
}

fn arrow_axis(from: Point, to: Point) -> Option<(Vector, Vector, f64)> {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    Some((unit, normal, length))
}

fn hollow_arrow_outline_points(
    start: Point,
    end: Point,
    arrow_head: ArrowHeadGeometry,
    has_head: bool,
    has_tail: bool,
) -> Option<Vec<Point>> {
    let (unit, normal, length) = arrow_axis(start, end)?;
    let shaft_half_width = arrow_head.center_length.max(arrow_head.length) * 0.5;
    let head_length = arrow_head.length.min(length * 0.45);
    let head_half_width = hollow_open_arrow_head_half_width(shaft_half_width, arrow_head);
    let start_neck = if has_tail {
        start.translated(unit.scaled(head_length))
    } else {
        start
    };
    let end_neck = if has_head {
        end.translated(unit.scaled(-head_length))
    } else {
        end
    };

    if !has_head && !has_tail {
        return Some(vec![
            start.translated(normal.scaled(shaft_half_width)),
            end.translated(normal.scaled(shaft_half_width)),
            end.translated(normal.scaled(-shaft_half_width)),
            start.translated(normal.scaled(-shaft_half_width)),
        ]);
    }

    let mut points = Vec::new();
    if has_tail {
        let tail_outer = start.translated(unit.scaled(head_length));
        points.push(start);
        points.push(tail_outer.translated(normal.scaled(-head_half_width)));
    } else {
        points.push(start.translated(normal.scaled(-shaft_half_width)));
    }
    points.push(start_neck.translated(normal.scaled(-shaft_half_width)));
    points.push(end_neck.translated(normal.scaled(-shaft_half_width)));
    if has_head {
        let head_outer = end.translated(unit.scaled(-head_length));
        points.push(head_outer.translated(normal.scaled(-head_half_width)));
        points.push(end);
        points.push(head_outer.translated(normal.scaled(head_half_width)));
    } else {
        points.push(end.translated(normal.scaled(-shaft_half_width)));
        points.push(end.translated(normal.scaled(shaft_half_width)));
    }
    points.push(end_neck.translated(normal.scaled(shaft_half_width)));
    points.push(start_neck.translated(normal.scaled(shaft_half_width)));
    if has_tail {
        let tail_outer = start.translated(unit.scaled(head_length));
        points.push(tail_outer.translated(normal.scaled(head_half_width)));
    } else {
        points.push(start.translated(normal.scaled(shaft_half_width)));
    }
    Some(compact_polygon_points(points))
}

pub(crate) fn render_line_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let points = payload_points(&object.payload, "points");
    if points.len() < 2 {
        return;
    }

    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_string(value, "stroke"))
        .unwrap_or_else(|| "#222222".to_string());
    let stroke_width = style
        .and_then(|value| {
            style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
        })
        .unwrap_or(px_to_pt(1.6));
    let line_cap = style
        .and_then(|value| style_string(value, "lineCap"))
        .unwrap_or_else(|| "round".to_string());
    let line_join = style
        .and_then(|value| style_string(value, "lineJoin"))
        .unwrap_or_else(|| "round".to_string());
    let dash_array = style
        .and_then(|value| style_number_array(value, "dashArray"))
        .unwrap_or_default();
    let object_id = Some(object.id.clone());
    let arrow_head = payload_arrow_head(&object.payload, "arrowHead", stroke_width);
    let arrow_arc = payload_arrow_arc_geometry(&object.payload, "arrowGeometry");
    if let Some(arrow_head) = arrow_head.filter(|arrow_head| arrow_head.length > 0.0) {
        let head_style = payload_arrow_endpoint_style(&object.payload, "head", "end");
        let tail_style = payload_arrow_endpoint_style(&object.payload, "tail", "start");
        render_arrow_line_object(
            out,
            &points,
            &stroke,
            stroke_width,
            &line_cap,
            &line_join,
            arrow_head,
            arrow_arc,
            head_style,
            tail_style,
            &dash_array,
            object_id,
        );
        return;
    }

    push_polyline(
        out,
        points,
        &stroke,
        stroke_width,
        dash_array,
        Some(line_cap),
        Some(line_join),
        RenderRole::DocumentGraphic,
        object_id,
    );
}

fn endpoint_flag_enabled(value: &str, expected: &str) -> bool {
    value.eq_ignore_ascii_case(expected) || value.eq_ignore_ascii_case("both")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderArrowEndpointStyle {
    None,
    Full,
    Left,
    Right,
}

impl RenderArrowEndpointStyle {
    fn enabled(self) -> bool {
        !matches!(self, Self::None)
    }
}

fn payload_arrow_endpoint_style(
    payload: &crate::ObjectPayload,
    key: &str,
    expected: &str,
) -> RenderArrowEndpointStyle {
    let explicit_enabled =
        payload_string(payload, key).is_some_and(|value| endpoint_flag_enabled(&value, expected));
    let arrow_style = payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .map(render_arrow_endpoint_style)
        .unwrap_or(RenderArrowEndpointStyle::None);
    if arrow_style.enabled() {
        return arrow_style;
    }
    if explicit_enabled {
        RenderArrowEndpointStyle::Full
    } else {
        RenderArrowEndpointStyle::None
    }
}

fn render_arrow_endpoint_style(value: &str) -> RenderArrowEndpointStyle {
    match value.to_ascii_lowercase().as_str() {
        "full" => RenderArrowEndpointStyle::Full,
        "half-left" | "halfleft" | "left" | "top" => RenderArrowEndpointStyle::Left,
        "half-right" | "halfright" | "right" | "bottom" => RenderArrowEndpointStyle::Right,
        _ => RenderArrowEndpointStyle::None,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_arrow_line_object(
    out: &mut Vec<RenderPrimitive>,
    points: &[Point],
    stroke: &str,
    stroke_width: f64,
    line_cap: &str,
    line_join: &str,
    arrow_head: ArrowHeadGeometry,
    arrow_arc: Option<ArrowArcGeometry>,
    head_style: RenderArrowEndpointStyle,
    tail_style: RenderArrowEndpointStyle,
    dash_array: &[f64],
    object_id: Option<String>,
) {
    if points.len() < 2 {
        return;
    }
    let start = points[0];
    let end = *points.last().unwrap_or(&start);
    if arrow_head.curve.abs() > crate::EPSILON && arrow_head.kind == ArrowHeadKind::Solid {
        let Some(arrow_arc) = arrow_arc else {
            return;
        };
        render_curved_solid_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            arrow_arc,
            head_style,
            tail_style,
            dash_array,
            object_id,
        );
        return;
    }
    match arrow_head.kind {
        ArrowHeadKind::Solid => render_solid_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            head_style,
            tail_style,
            dash_array,
            object_id,
        ),
        ArrowHeadKind::Hollow => render_hollow_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            arrow_head,
            head_style.enabled(),
            tail_style.enabled(),
            object_id,
        ),
        ArrowHeadKind::Open => render_open_arrow_line(
            out,
            start,
            end,
            stroke,
            stroke_width,
            line_cap,
            line_join,
            arrow_head,
            head_style.enabled(),
            tail_style.enabled(),
            dash_array,
            object_id,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_curved_solid_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    arrow_head: ArrowHeadGeometry,
    arrow_arc: ArrowArcGeometry,
    head_style: RenderArrowEndpointStyle,
    tail_style: RenderArrowEndpointStyle,
    dash_array: &[f64],
    object_id: Option<String>,
) {
    let points = curved_arrow_points(start, arrow_head.curve, arrow_arc);
    if points.len() < 2 {
        return;
    }
    let line_width = if arrow_head.bold {
        stroke_width.max(4.0)
    } else {
        stroke_width
    };
    let start_trim = curved_arrow_endpoint_shaft_trim(tail_style, arrow_head);
    let end_trim = curved_arrow_endpoint_shaft_trim(head_style, arrow_head);
    if let Some(path) = curved_arrow_path(start, arrow_head.curve, arrow_arc, start_trim, end_trim)
    {
        push_path(
            out,
            path.d,
            path.points,
            stroke,
            line_width,
            dash_array.to_vec(),
            Some("butt".to_string()),
            Some("round".to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    render_arrow_no_go_on_points(out, &points, stroke, arrow_head, object_id.clone());
    if head_style.enabled() {
        let tangent_from = point_at_distance_from_end(&points, arrow_head.center_length)
            .unwrap_or_else(|| points[points.len().saturating_sub(2)]);
        if head_style == RenderArrowEndpointStyle::Full {
            push_polygon(
                out,
                arrow_head_points(tangent_from, end, arrow_head),
                stroke,
                stroke,
                0.0,
                RenderRole::DocumentGraphic,
                object_id.clone(),
            );
        } else {
            render_solid_arrow_head(
                out,
                tangent_from,
                end,
                arrow_head,
                head_style,
                line_width,
                stroke,
                object_id.clone(),
            );
        }
    }
    if tail_style.enabled() {
        let tangent_from = point_at_distance_from_start(&points, arrow_head.center_length)
            .unwrap_or_else(|| *points.get(1).unwrap_or(&end));
        if tail_style == RenderArrowEndpointStyle::Full {
            push_polygon(
                out,
                arrow_head_points(tangent_from, start, arrow_head),
                stroke,
                stroke,
                0.0,
                RenderRole::DocumentGraphic,
                object_id,
            );
        } else {
            render_solid_arrow_head(
                out,
                tangent_from,
                start,
                arrow_head,
                tail_style,
                line_width,
                stroke,
                object_id,
            );
        }
    }
}

fn curved_arrow_points(
    start: Point,
    sweep_degrees: f64,
    arrow_arc: ArrowArcGeometry,
) -> Vec<Point> {
    let Some(arc) = curved_arrow_arc(start, sweep_degrees, arrow_arc) else {
        return Vec::new();
    };
    let steps = ((sweep_degrees.abs() / 12.0).ceil() as usize).clamp(8, 32);
    (0..=steps)
        .map(|index| {
            let t = index as f64 / steps as f64;
            arc.point_at(arc.start_angle() + arc.sweep() * t)
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct CurvedArrowArc {
    center: Point,
    major: Vector,
    minor: Vector,
    start_angle: f64,
    sweep: f64,
}

impl CurvedArrowArc {
    fn point_at(&self, angle: f64) -> Point {
        self.center
            .translated(self.major.scaled(angle.cos()))
            .translated(self.minor.scaled(angle.sin()))
    }

    fn derivative_at(&self, angle: f64) -> Vector {
        Vector::new(
            -self.major.x * angle.sin() + self.minor.x * angle.cos(),
            -self.major.y * angle.sin() + self.minor.y * angle.cos(),
        )
    }

    fn start_angle(&self) -> f64 {
        self.start_angle
    }

    fn sweep(&self) -> f64 {
        self.sweep
    }
}

struct CurvedArrowPath {
    d: String,
    points: Vec<Point>,
}

fn curved_arrow_arc(
    start: Point,
    sweep_degrees: f64,
    geometry: ArrowArcGeometry,
) -> Option<CurvedArrowArc> {
    let major = Vector::new(
        geometry.major_axis_end.x - geometry.center.x,
        geometry.major_axis_end.y - geometry.center.y,
    );
    let minor = Vector::new(
        geometry.minor_axis_end.x - geometry.center.x,
        geometry.minor_axis_end.y - geometry.center.y,
    );
    let det = major.x * minor.y - major.y * minor.x;
    if det.abs() <= crate::EPSILON
        || major.length() <= crate::EPSILON
        || minor.length() <= crate::EPSILON
        || sweep_degrees.abs() <= crate::EPSILON
    {
        return None;
    }
    let relative = Vector::new(start.x - geometry.center.x, start.y - geometry.center.y);
    let cos = (relative.x * minor.y - relative.y * minor.x) / det;
    let sin = (major.x * relative.y - major.y * relative.x) / det;
    Some(CurvedArrowArc {
        center: geometry.center,
        major,
        minor,
        start_angle: sin.atan2(cos),
        sweep: -sweep_degrees.to_radians(),
    })
}

fn curved_arrow_path(
    start: Point,
    sweep_degrees: f64,
    arrow_arc: ArrowArcGeometry,
    start_trim: f64,
    end_trim: f64,
) -> Option<CurvedArrowPath> {
    let arc = curved_arrow_arc(start, sweep_degrees, arrow_arc)?;
    let total = curved_arrow_arc_length(&arc, arc.start_angle(), arc.sweep());
    if start_trim + end_trim >= total - crate::EPSILON {
        return None;
    }
    let start_angle = curved_arrow_angle_at_distance(&arc, start_trim);
    let end_angle = curved_arrow_angle_at_distance(&arc, total - end_trim);
    let sweep = end_angle - start_angle;
    let start_point = arc.point_at(start_angle);
    let mut d = format!("M {} {}", start_point.x, start_point.y);
    let mut angle = start_angle;
    let segments = (sweep.abs() / (std::f64::consts::FRAC_PI_2)).ceil() as usize;
    let segments = segments.max(1);
    let delta = sweep / segments as f64;
    let mut points = Vec::with_capacity(33);
    let sample_steps = ((sweep.abs().to_degrees() / 6.0).ceil() as usize).clamp(8, 64);
    for index in 0..=sample_steps {
        let t = index as f64 / sample_steps as f64;
        points.push(arc.point_at(start_angle + sweep * t));
    }
    for _ in 0..segments {
        let next = angle + delta;
        let p0 = arc.point_at(angle);
        let p3 = arc.point_at(next);
        let k = 4.0 / 3.0 * (delta * 0.25).tan();
        let d0 = arc.derivative_at(angle);
        let d1 = arc.derivative_at(next);
        let c1 = p0.translated(d0.scaled(k));
        let c2 = p3.translated(d1.scaled(-k));
        d.push_str(&format!(
            " C {} {}, {} {}, {} {}",
            c1.x, c1.y, c2.x, c2.y, p3.x, p3.y
        ));
        angle = next;
    }
    Some(CurvedArrowPath { d, points })
}

fn curved_arrow_arc_length(arc: &CurvedArrowArc, start_angle: f64, sweep: f64) -> f64 {
    let steps = ((sweep.abs().to_degrees() / 2.0).ceil() as usize).clamp(16, 256);
    let mut length = 0.0;
    let mut previous = arc.point_at(start_angle);
    for index in 1..=steps {
        let t = index as f64 / steps as f64;
        let point = arc.point_at(start_angle + sweep * t);
        length += previous.distance(point);
        previous = point;
    }
    length
}

fn curved_arrow_angle_at_distance(arc: &CurvedArrowArc, distance: f64) -> f64 {
    if distance <= 0.0 {
        return arc.start_angle();
    }
    let total = curved_arrow_arc_length(arc, arc.start_angle(), arc.sweep());
    if distance >= total {
        return arc.start_angle() + arc.sweep();
    }
    let steps = ((arc.sweep().abs().to_degrees() / 2.0).ceil() as usize).clamp(16, 256);
    let mut walked = 0.0;
    let mut previous_angle = arc.start_angle();
    let mut previous = arc.point_at(previous_angle);
    for index in 1..=steps {
        let t = index as f64 / steps as f64;
        let angle = arc.start_angle() + arc.sweep() * t;
        let point = arc.point_at(angle);
        let segment = previous.distance(point);
        if walked + segment >= distance {
            let local = if segment <= crate::EPSILON {
                0.0
            } else {
                (distance - walked) / segment
            };
            return previous_angle + (angle - previous_angle) * local;
        }
        walked += segment;
        previous = point;
        previous_angle = angle;
    }
    arc.start_angle() + arc.sweep()
}

fn polyline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum()
}

fn point_at_distance_from_start(points: &[Point], distance: f64) -> Option<Point> {
    if points.len() < 2 {
        return None;
    }
    if distance <= 0.0 {
        return points.first().copied();
    }
    let mut remaining = distance;
    for pair in points.windows(2) {
        let segment = pair[0].distance(pair[1]);
        if remaining <= segment {
            let t = if segment <= crate::EPSILON {
                0.0
            } else {
                remaining / segment
            };
            return Some(Point::new(
                pair[0].x + (pair[1].x - pair[0].x) * t,
                pair[0].y + (pair[1].y - pair[0].y) * t,
            ));
        }
        remaining -= segment;
    }
    points.last().copied()
}

fn point_at_distance_from_end(points: &[Point], distance: f64) -> Option<Point> {
    if points.len() < 2 {
        return None;
    }
    let total = polyline_length(points);
    point_at_distance_from_start(points, (total - distance).max(0.0))
}

#[allow(clippy::too_many_arguments)]
fn render_solid_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    arrow_head: ArrowHeadGeometry,
    head_style: RenderArrowEndpointStyle,
    tail_style: RenderArrowEndpointStyle,
    dash_array: &[f64],
    object_id: Option<String>,
) {
    let Some((unit, _normal, length)) = arrow_axis(start, end) else {
        return;
    };
    let line_width = if arrow_head.bold {
        stroke_width.max(4.0)
    } else {
        stroke_width
    };
    let start_shaft = start.translated(
        unit.scaled(arrow_endpoint_shaft_trim(tail_style, arrow_head).min(length * 0.45)),
    );
    let end_shaft = end.translated(
        unit.scaled(-arrow_endpoint_shaft_trim(head_style, arrow_head).min(length * 0.45)),
    );
    if start_shaft.distance(end_shaft) > crate::EPSILON {
        push_polyline(
            out,
            vec![start_shaft, end_shaft],
            stroke,
            line_width,
            dash_array.to_vec(),
            Some("butt".to_string()),
            Some("miter".to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    render_arrow_no_go_on_axis(out, start, end, stroke, arrow_head, object_id.clone());
    if head_style.enabled() {
        render_solid_arrow_head(
            out,
            start,
            end,
            arrow_head,
            head_style,
            line_width,
            stroke,
            object_id.clone(),
        );
    }
    if tail_style.enabled() {
        render_solid_arrow_head(
            out, end, start, arrow_head, tail_style, line_width, stroke, object_id,
        );
    }
}

fn render_arrow_no_go_on_points(
    out: &mut Vec<RenderPrimitive>,
    points: &[Point],
    stroke: &str,
    arrow_head: ArrowHeadGeometry,
    object_id: Option<String>,
) {
    if points.len() < 2 {
        return;
    }
    let total = polyline_length(points);
    if total <= crate::EPSILON {
        return;
    }
    let center = point_at_distance_from_start(points, total * 0.5).unwrap_or(points[0]);
    let before =
        point_at_distance_from_start(points, (total * 0.5 - 0.1).max(0.0)).unwrap_or(points[0]);
    let after = point_at_distance_from_start(points, (total * 0.5 + 0.1).min(total))
        .unwrap_or_else(|| *points.last().unwrap_or(&points[0]));
    render_arrow_no_go_mark(out, center, before, after, stroke, arrow_head, object_id);
}

fn render_arrow_no_go_on_axis(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    arrow_head: ArrowHeadGeometry,
    object_id: Option<String>,
) {
    let center = Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
    render_arrow_no_go_mark(out, center, start, end, stroke, arrow_head, object_id);
}

fn render_arrow_no_go_mark(
    out: &mut Vec<RenderPrimitive>,
    center: Point,
    tangent_from: Point,
    tangent_to: Point,
    stroke: &str,
    arrow_head: ArrowHeadGeometry,
    object_id: Option<String>,
) {
    if arrow_head.no_go == ArrowNoGoGeometry::None {
        return;
    }
    let Some((unit, normal, _length)) = arrow_axis(tangent_from, tangent_to) else {
        return;
    };
    let bar_width =
        (arrow_head.length * 0.14).clamp(1.4, 2.6) * if arrow_head.bold { 1.25 } else { 1.0 };
    match arrow_head.no_go {
        ArrowNoGoGeometry::None => {}
        ArrowNoGoGeometry::Cross => {
            let bar_length = arrow_head.length * 2.05;
            for axis in [
                Vector::new(unit.x + normal.x, unit.y + normal.y).normalized(),
                Vector::new(unit.x - normal.x, unit.y - normal.y).normalized(),
            ] {
                push_polygon(
                    out,
                    no_go_bar_points(center, axis, bar_length, bar_width),
                    stroke,
                    stroke,
                    0.0,
                    RenderRole::DocumentGraphic,
                    object_id.clone(),
                );
            }
        }
        ArrowNoGoGeometry::Hash => {
            let bar_length = arrow_head.length * 2.25;
            let axis = Vector::new(unit.x - normal.x * 2.0, unit.y - normal.y * 2.0).normalized();
            let offset = unit.scaled(arrow_head.length * 0.36);
            for center in [
                center.translated(offset.scaled(-0.5)),
                center.translated(offset.scaled(0.5)),
            ] {
                push_polygon(
                    out,
                    no_go_bar_points(center, axis, bar_length, bar_width),
                    stroke,
                    stroke,
                    0.0,
                    RenderRole::DocumentGraphic,
                    object_id.clone(),
                );
            }
        }
    }
}

fn no_go_bar_points(center: Point, axis: Vector, length: f64, width: f64) -> Vec<Point> {
    let normal = Vector::new(-axis.y, axis.x);
    let half_length = length * 0.5;
    let half_width = width * 0.5;
    vec![
        center
            .translated(axis.scaled(-half_length))
            .translated(normal.scaled(-half_width)),
        center
            .translated(axis.scaled(half_length))
            .translated(normal.scaled(-half_width)),
        center
            .translated(axis.scaled(half_length))
            .translated(normal.scaled(half_width)),
        center
            .translated(axis.scaled(-half_length))
            .translated(normal.scaled(half_width)),
    ]
}

fn render_solid_arrow_head(
    out: &mut Vec<RenderPrimitive>,
    from: Point,
    to: Point,
    arrow_head: ArrowHeadGeometry,
    style: RenderArrowEndpointStyle,
    line_width: f64,
    fill: &str,
    object_id: Option<String>,
) {
    if style == RenderArrowEndpointStyle::Full {
        if let Some(path) = solid_full_arrow_head_path(from, to, arrow_head) {
            out.push(RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id,
                node_id: None,
                bond_id: None,
                d: path.d,
                points: path.points,
                fill: fill.to_string(),
                fill_rule: None,
                clip_path_d: None,
                clip_rule: None,
                rotate: 0.0,
                rotate_center: None,
            });
        }
        return;
    }
    if let Some(path) = solid_half_arrow_head_path(from, to, arrow_head, style, line_width) {
        out.push(RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id,
            node_id: None,
            bond_id: None,
            d: path.d,
            points: path.points,
            fill: fill.to_string(),
            fill_rule: None,
            clip_path_d: None,
            clip_rule: None,
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

struct SolidArrowHeadPath {
    d: String,
    points: Vec<Point>,
}

fn solid_full_arrow_head_path(
    from: Point,
    to: Point,
    arrow_head: ArrowHeadGeometry,
) -> Option<SolidArrowHeadPath> {
    let (unit, normal, _) = arrow_axis(from, to)?;
    let head_length = arrow_head.length;
    let head_half_width = solid_arrow_head_outer_half_width(arrow_head);
    let notch_length = arrow_head.center_length.max(0.0).min(head_length);
    let control_half_width = head_half_width * 7.0 / 16.0;

    let tip = to;
    let left = to
        .translated(unit.scaled(-head_length))
        .translated(normal.scaled(head_half_width));
    let left_control = to
        .translated(unit.scaled(-notch_length))
        .translated(normal.scaled(control_half_width));
    let notch = to.translated(unit.scaled(-notch_length));
    let right_control = to
        .translated(unit.scaled(-notch_length))
        .translated(normal.scaled(-control_half_width));
    let right = to
        .translated(unit.scaled(-head_length))
        .translated(normal.scaled(-head_half_width));

    Some(SolidArrowHeadPath {
        d: format!(
            "M {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{}",
            tip.x,
            tip.y,
            tip.x,
            tip.y,
            left.x,
            left.y,
            left.x,
            left.y,
            left.x,
            left.y,
            left_control.x,
            left_control.y,
            notch.x,
            notch.y,
            right_control.x,
            right_control.y,
            right.x,
            right.y,
            right.x,
            right.y,
            right.x,
            right.y,
            tip.x,
            tip.y,
            tip.x,
            tip.y
        ),
        points: vec![tip, left, left_control, notch, right_control, right],
    })
}

fn solid_half_arrow_head_path(
    from: Point,
    to: Point,
    arrow_head: ArrowHeadGeometry,
    style: RenderArrowEndpointStyle,
    line_width: f64,
) -> Option<SolidArrowHeadPath> {
    let (unit, normal, _) = arrow_axis(from, to)?;
    let head_length = arrow_head.length;
    let head_half_width = arrow_head.width.max(0.0);
    let notch_length = arrow_head.center_length.max(0.0).min(head_length);
    let line_half_width = (line_width * 0.5).max(0.0);
    let curve_run = (head_length - notch_length).max(0.0);
    let control_distance = (notch_length - curve_run * 0.59).max(0.0);
    let control_half_width = line_half_width + (head_half_width - line_half_width).max(0.0) * 0.16;

    let tip = to;
    let positive_tip_edge = tip.translated(normal.scaled(line_half_width));
    let negative_tip_edge = tip.translated(normal.scaled(-line_half_width));
    let positive_notch_edge = tip
        .translated(unit.scaled(-notch_length))
        .translated(normal.scaled(line_half_width));
    let negative_notch_edge = tip
        .translated(unit.scaled(-notch_length))
        .translated(normal.scaled(-line_half_width));
    let positive_outer = to
        .translated(unit.scaled(-head_length))
        .translated(normal.scaled(head_half_width));
    let positive_control = to
        .translated(unit.scaled(-control_distance))
        .translated(normal.scaled(control_half_width));
    let negative_control = to
        .translated(unit.scaled(-control_distance))
        .translated(normal.scaled(-control_half_width));
    let negative_outer = to
        .translated(unit.scaled(-head_length))
        .translated(normal.scaled(-head_half_width));

    match style {
        RenderArrowEndpointStyle::Left => Some(SolidArrowHeadPath {
            d: format!(
                "M {},{} L {},{} C {},{} {},{} {},{} Z",
                positive_tip_edge.x,
                positive_tip_edge.y,
                negative_outer.x,
                negative_outer.y,
                negative_control.x,
                negative_control.y,
                positive_notch_edge.x,
                positive_notch_edge.y,
                positive_notch_edge.x,
                positive_notch_edge.y,
            ),
            points: vec![
                positive_tip_edge,
                negative_outer,
                negative_control,
                positive_notch_edge,
            ],
        }),
        RenderArrowEndpointStyle::Right => Some(SolidArrowHeadPath {
            d: format!(
                "M {},{} L {},{} C {},{} {},{} {},{} Z",
                negative_tip_edge.x,
                negative_tip_edge.y,
                positive_outer.x,
                positive_outer.y,
                positive_control.x,
                positive_control.y,
                negative_notch_edge.x,
                negative_notch_edge.y,
                negative_notch_edge.x,
                negative_notch_edge.y,
            ),
            points: vec![
                negative_tip_edge,
                positive_outer,
                positive_control,
                negative_notch_edge,
            ],
        }),
        RenderArrowEndpointStyle::Full | RenderArrowEndpointStyle::None => None,
    }
}

fn solid_arrow_head_outer_half_width(arrow_head: ArrowHeadGeometry) -> f64 {
    arrow_head.width.max(0.0) + 0.05
}

fn arrow_endpoint_shaft_trim(
    style: RenderArrowEndpointStyle,
    arrow_head: ArrowHeadGeometry,
) -> f64 {
    match style {
        RenderArrowEndpointStyle::Full => arrow_head.center_length,
        RenderArrowEndpointStyle::Left | RenderArrowEndpointStyle::Right => {
            half_arrow_shaft_trim(arrow_head)
        }
        RenderArrowEndpointStyle::None => 0.0,
    }
}

fn curved_arrow_endpoint_shaft_trim(
    style: RenderArrowEndpointStyle,
    arrow_head: ArrowHeadGeometry,
) -> f64 {
    match style {
        RenderArrowEndpointStyle::Full => arrow_head.center_length,
        RenderArrowEndpointStyle::Left | RenderArrowEndpointStyle::Right => {
            half_arrow_shaft_trim(arrow_head)
        }
        RenderArrowEndpointStyle::None => 0.0,
    }
}

fn half_arrow_shaft_trim(arrow_head: ArrowHeadGeometry) -> f64 {
    (arrow_head.center_length.max(0.0).min(arrow_head.length)
        - arrow_head.width.max(0.0) * 2.0 / 3.0)
        .max(0.0)
}

#[allow(clippy::too_many_arguments)]
fn render_hollow_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    arrow_head: ArrowHeadGeometry,
    has_head: bool,
    has_tail: bool,
    object_id: Option<String>,
) {
    let Some(points) = hollow_arrow_outline_points(start, end, arrow_head, has_head, has_tail)
    else {
        return;
    };
    push_polygon(
        out,
        points,
        "none",
        stroke,
        if arrow_head.bold {
            stroke_width.max(2.0)
        } else {
            stroke_width
        },
        RenderRole::DocumentGraphic,
        object_id,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_open_arrow_line(
    out: &mut Vec<RenderPrimitive>,
    start: Point,
    end: Point,
    stroke: &str,
    stroke_width: f64,
    line_cap: &str,
    line_join: &str,
    arrow_head: ArrowHeadGeometry,
    has_head: bool,
    has_tail: bool,
    dash_array: &[f64],
    object_id: Option<String>,
) {
    let Some((unit, normal, length)) = arrow_axis(start, end) else {
        return;
    };
    let line_width = if arrow_head.bold {
        stroke_width.max(2.0)
    } else {
        stroke_width
    };
    let shaft_half_width = open_arrow_shaft_half_width(arrow_head);
    let head_length = arrow_head.length.min(length * 0.45);
    let neck_offset = (head_length * 0.5).min(length * 0.3);
    let head_half_width = open_arrow_head_half_width(arrow_head);
    let start_neck = if has_tail {
        start.translated(unit.scaled(neck_offset))
    } else {
        start
    };
    let end_neck = if has_head {
        end.translated(unit.scaled(-neck_offset))
    } else {
        end
    };

    if has_tail {
        push_polyline(
            out,
            open_arrow_head_outline_points(
                start,
                unit.scaled(-1.0),
                normal,
                head_length,
                neck_offset,
                shaft_half_width,
                head_half_width,
            ),
            stroke,
            line_width,
            dash_array.to_vec(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    if start_neck.distance(end_neck) > crate::EPSILON {
        push_polyline(
            out,
            vec![
                start_neck.translated(normal.scaled(shaft_half_width)),
                end_neck.translated(normal.scaled(shaft_half_width)),
            ],
            stroke,
            line_width,
            dash_array.to_vec(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
        push_polyline(
            out,
            vec![
                start_neck.translated(normal.scaled(-shaft_half_width)),
                end_neck.translated(normal.scaled(-shaft_half_width)),
            ],
            stroke,
            line_width,
            dash_array.to_vec(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
    if has_head {
        push_polyline(
            out,
            open_arrow_head_outline_points(
                end,
                unit,
                normal,
                head_length,
                neck_offset,
                shaft_half_width,
                head_half_width,
            ),
            stroke,
            line_width,
            dash_array.to_vec(),
            Some(line_cap.to_string()),
            Some(line_join.to_string()),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    }
}

fn open_arrow_shaft_half_width(arrow_head: ArrowHeadGeometry) -> f64 {
    arrow_head.center_length.max(arrow_head.length) * 0.5
}

fn open_arrow_head_half_width(arrow_head: ArrowHeadGeometry) -> f64 {
    hollow_open_arrow_head_half_width(open_arrow_shaft_half_width(arrow_head), arrow_head)
}

fn hollow_open_arrow_head_half_width(shaft_half_width: f64, arrow_head: ArrowHeadGeometry) -> f64 {
    shaft_half_width + arrow_head.width.max(0.0) * 2.0
}

fn open_arrow_head_outline_points(
    tip: Point,
    outward: crate::Vector,
    normal: crate::Vector,
    head_length: f64,
    neck_offset: f64,
    shaft_half_width: f64,
    head_half_width: f64,
) -> Vec<Point> {
    vec![
        tip.translated(outward.scaled(-neck_offset))
            .translated(normal.scaled(shaft_half_width)),
        tip.translated(outward.scaled(-head_length))
            .translated(normal.scaled(head_half_width)),
        tip,
        tip.translated(outward.scaled(-head_length))
            .translated(normal.scaled(-head_half_width)),
        tip.translated(outward.scaled(-neck_offset))
            .translated(normal.scaled(-shaft_half_width)),
    ]
}
