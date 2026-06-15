use super::*;

pub fn line_object_points(object: &crate::SceneObject) -> Vec<Point> {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    object
        .payload
        .extra
        .get("points")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let coords = value.as_array()?;
            Some(Point::new(
                tx + coords.first()?.as_f64()?,
                ty + coords.get(1)?.as_f64()?,
            ))
        })
        .collect()
}

pub(super) fn line_object_endpoint_style(
    object: &crate::SceneObject,
    key: &str,
    expected: &str,
) -> ArrowEndpointStyle {
    if let Some(value) = object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
    {
        return match value.to_ascii_lowercase().as_str() {
            "full" => ArrowEndpointStyle::Full,
            "half-left" | "halfleft" | "left" | "top" => ArrowEndpointStyle::Left,
            "half-right" | "halfright" | "right" | "bottom" => ArrowEndpointStyle::Right,
            _ => ArrowEndpointStyle::None,
        };
    }
    object
        .payload
        .extra
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.eq_ignore_ascii_case(expected) || value.eq_ignore_ascii_case("both"))
        .map(|_| ArrowEndpointStyle::Full)
        .unwrap_or(ArrowEndpointStyle::None)
}

pub(super) fn line_object_arrow_dimension(
    object: &crate::SceneObject,
    key: &str,
    fallback: f64,
) -> f64 {
    object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(fallback)
}

pub(super) fn line_object_arrow_curve(object: &crate::SceneObject) -> f64 {
    object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get("curve"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

fn line_object_arrow_kind(object: &crate::SceneObject) -> String {
    object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get("kind"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase()
}

pub fn arrow_object_has_curve_handle(object: &crate::SceneObject) -> bool {
    line_object_arrow_kind(object) != "open"
}

pub fn arrow_object_handle_points(object: &crate::SceneObject, points: &[Point]) -> Vec<Point> {
    if points.len() < 2 {
        return Vec::new();
    }
    let focus_points = arrow_object_focus_points(object, points);
    let start = focus_points[0];
    let end = *focus_points.last().unwrap_or(&focus_points[0]);
    let center = point_at_distance_from_start(&focus_points, polyline_length(&focus_points) * 0.5)
        .unwrap_or_else(|| Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5));
    let mut handles = vec![start];
    if arrow_object_has_curve_handle(object) {
        handles.push(center);
    }
    handles.push(end);
    let head_length = line_object_arrow_dimension(object, "length", 15.0);
    let head_width = line_object_arrow_dimension(object, "width", 3.75);
    let head_style = line_object_endpoint_style(object, "head", "end");
    let tail_style = line_object_endpoint_style(object, "tail", "start");
    push_arrow_endpoint_handles(
        &mut handles,
        &focus_points,
        false,
        head_style,
        head_length,
        head_width,
    );
    push_arrow_endpoint_handles(
        &mut handles,
        &focus_points,
        true,
        tail_style,
        head_length,
        head_width,
    );
    handles
}

pub(super) fn push_arrow_endpoint_handles(
    handles: &mut Vec<Point>,
    points: &[Point],
    tail: bool,
    style: ArrowEndpointStyle,
    length: f64,
    half_width: f64,
) {
    if style == ArrowEndpointStyle::None || points.len() < 2 {
        return;
    }
    let tangent_from = if tail {
        point_at_distance_from_start(points, length).unwrap_or(points[1])
    } else {
        point_at_distance_from_end(points, length)
            .unwrap_or_else(|| points[points.len().saturating_sub(2)])
    };
    let tip = if tail {
        points[0]
    } else {
        *points.last().unwrap_or(&points[0])
    };
    let side_points = arrow_tip_side_points(tangent_from, tip, length, half_width);
    match style {
        ArrowEndpointStyle::Full => handles.extend(side_points),
        ArrowEndpointStyle::Left => handles.push(side_points[1]),
        ArrowEndpointStyle::Right => handles.push(side_points[0]),
        ArrowEndpointStyle::None => {}
    }
}

pub fn arrow_object_focus_points(object: &crate::SceneObject, points: &[Point]) -> Vec<Point> {
    if points.len() < 2 {
        return Vec::new();
    }
    let start = points[0];
    let end = *points.last().unwrap_or(&points[0]);
    let curve = line_object_arrow_curve(object);
    if curve.abs() <= crate::EPSILON {
        return vec![start, end];
    }
    curved_arrow_points(start, curve, object).unwrap_or_default()
}

pub(super) fn curved_arrow_points(
    start: Point,
    sweep_degrees: f64,
    object: &crate::SceneObject,
) -> Option<Vec<Point>> {
    let (center, major_axis_end, minor_axis_end) = line_object_arrow_arc_geometry(object)?;
    let major = Vector::new(major_axis_end.x - center.x, major_axis_end.y - center.y);
    let minor = Vector::new(minor_axis_end.x - center.x, minor_axis_end.y - center.y);
    let det = major.x * minor.y - major.y * minor.x;
    if det.abs() <= crate::EPSILON
        || major.length() <= crate::EPSILON
        || minor.length() <= crate::EPSILON
        || sweep_degrees.abs() <= crate::EPSILON
    {
        return None;
    }
    let relative = Vector::new(start.x - center.x, start.y - center.y);
    let cos = (relative.x * minor.y - relative.y * minor.x) / det;
    let sin = (major.x * relative.y - major.y * relative.x) / det;
    let start_angle = sin.atan2(cos);
    let sweep = -sweep_degrees.to_radians();
    let steps = ((sweep_degrees.abs() / 12.0).ceil() as usize).clamp(8, 32);
    Some(
        (0..=steps)
            .map(|index| {
                let t = index as f64 / steps as f64;
                let angle = start_angle + sweep * t;
                center
                    .translated(major.scaled(angle.cos()))
                    .translated(minor.scaled(angle.sin()))
            })
            .collect(),
    )
}

fn line_object_arrow_arc_geometry(object: &crate::SceneObject) -> Option<(Point, Point, Point)> {
    let geometry = object.payload.extra.get("arrowGeometry")?;
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let point = |key: &str| -> Option<Point> {
        let coords = geometry.get(key)?.as_array()?;
        Some(Point::new(
            tx + coords.first()?.as_f64()?,
            ty + coords.get(1)?.as_f64()?,
        ))
    };
    Some((
        point("center")?,
        point("majorAxisEnd")?,
        point("minorAxisEnd")?,
    ))
}

pub fn polyline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum()
}

pub fn point_at_distance_from_start(points: &[Point], distance: f64) -> Option<Point> {
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

pub(super) fn point_at_distance_from_end(points: &[Point], distance: f64) -> Option<Point> {
    if points.len() < 2 {
        return None;
    }
    let total = polyline_length(points);
    point_at_distance_from_start(points, (total - distance).max(0.0))
}

pub(super) fn arrow_tip_side_points(
    from: Point,
    to: Point,
    length: f64,
    half_width: f64,
) -> [Point; 2] {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance <= 1.0e-9 {
        return [to, to];
    }
    let ux = dx / distance;
    let uy = dy / distance;
    let nx = -uy;
    let ny = ux;
    [
        Point::new(
            to.x - ux * length + nx * half_width,
            to.y - uy * length + ny * half_width,
        ),
        Point::new(
            to.x - ux * length - nx * half_width,
            to.y - uy * length - ny * half_width,
        ),
    ]
}
