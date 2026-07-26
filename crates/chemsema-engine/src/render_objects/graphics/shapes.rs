use super::*;

pub(crate) fn render_shape_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let style = ShapeStyleSpec::from_style(style);
    if payload_string(&object.payload, "kind").as_deref() == Some("orbital") {
        render_orbital_shape_object(out, object, style);
        return;
    }
    if payload_string(&object.payload, "kind").as_deref() == Some("tlcPlate") {
        render_tlc_plate_shape_object(out, document, object, style);
        return;
    }
    let Some(geometry) = ShapeGeometry::from_object(object) else {
        return;
    };
    render_shape_geometry(out, &object.id, &geometry, style);
}

pub(super) fn render_shape_geometry(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    style: ShapeStyleSpec,
) {
    match style.render_style {
        ShapeRenderStyle::Solid | ShapeRenderStyle::Dashed => {
            if let Some(stroke) = style.stroke {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    style.stroke_width,
                    style.dash_array,
                );
            }
        }
        ShapeRenderStyle::Filled => {
            push_shape_fill(
                out,
                object_id,
                geometry,
                style.fill.unwrap_or_else(|| "#000000".to_string()),
            );
            if matches!(
                geometry,
                ShapeGeometry::Rect { .. } | ShapeGeometry::Oval { ellipse: true, .. }
            ) {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    "#000000".to_string(),
                    0.05,
                    Vec::new(),
                );
            }
        }
        ShapeRenderStyle::Shaded => {
            push_shape_shaded_layers(out, object_id, geometry, style.base_color());
            if let Some(stroke) = style.stroke {
                if matches!(geometry, ShapeGeometry::Rect { .. }) {
                    push_shape_outline(out, object_id, geometry, stroke.clone(), 0.05, Vec::new());
                }
                let stroke_width = match geometry {
                    ShapeGeometry::Oval { ellipse: true, .. } => 0.05,
                    _ => style.stroke_width,
                };
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    stroke_width,
                    style.dash_array,
                );
            }
        }
        ShapeRenderStyle::Shadowed => {
            // CDXML ShadowSize is a multiplier in hundredths of the outline
            // width, not an absolute point distance.  ChemDraw's SVG output
            // uses 12, 20 and 40 internal units for ShadowSize="100" when
            // LineWidth is respectively 0.6, 1 and 2 pt; the same linear rule
            // holds across the tested 100..800 range.
            let shadow_offset = style.shadow_size * style.stroke_width;
            push_shape_shadow_path(
                out,
                object_id,
                geometry.shifted_fill_path_d(shadow_offset, shadow_offset),
                geometry.fill_path_d(),
                shape_shadow_fill(style.stroke.as_deref(), style.fill.as_deref()),
                geometry.shadow_bounds_points(shadow_offset),
            );
            if let Some(fill) = style.fill {
                push_shape_fill(out, object_id, geometry, fill);
            }
            if let Some(stroke) = style.stroke {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    style.stroke_width,
                    style.dash_array,
                );
            }
        }
        ShapeRenderStyle::Custom => push_shape_custom(out, object_id, geometry, style),
    }
}

pub(super) fn push_shape_fill(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    fill: String,
) {
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        node_id: None,
        bond_id: None,
        d: geometry.fill_path_d(),
        points: geometry.bounds_points(),
        fill,
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
        rotate: 0.0,
        rotate_center: None,
    });
}

pub(super) fn push_shape_outline(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    stroke: String,
    stroke_width: f64,
    dash_array: Vec<f64>,
) {
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        bond_id: None,
        d: geometry.outline_path_d(&dash_array),
        points: geometry.bounds_points(),
        stroke,
        stroke_width,
        dash_array,
        line_cap: match geometry {
            ShapeGeometry::Rect { .. } => Some("butt".to_string()),
            ShapeGeometry::Oval { .. } => None,
        },
        line_join: match geometry {
            ShapeGeometry::Rect { .. } => Some("miter".to_string()),
            ShapeGeometry::Oval { .. } => None,
        },
        rotate: 0.0,
        rotate_center: None,
    });
}

pub(super) fn push_shape_shaded_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    base_color: &str,
) {
    match *geometry {
        ShapeGeometry::Oval {
            center,
            rx,
            ry,
            rotate,
            ellipse,
        } => {
            push_shaded_ellipse_layers(out, object_id, ellipse, center, rx, ry, rotate, base_color)
        }
        ShapeGeometry::Rect {
            x,
            y,
            width,
            height,
            corner_radius,
            rounded,
            ..
        } => push_shaded_rect_layers(
            out,
            object_id,
            x,
            y,
            width,
            height,
            corner_radius,
            rounded,
            base_color,
        ),
    }
}

pub(super) fn push_shape_custom(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &ShapeGeometry,
    style: ShapeStyleSpec,
) {
    match geometry {
        ShapeGeometry::Rect {
            x,
            y,
            width,
            height,
            corner_radius,
            rotate,
            ..
        } => {
            if rotate.abs() > crate::EPSILON {
                if let Some(fill) = style.fill {
                    push_shape_fill(out, object_id, geometry, fill);
                }
                if let Some(stroke) = style.stroke {
                    push_shape_outline(
                        out,
                        object_id,
                        geometry,
                        stroke,
                        style.stroke_width,
                        style.dash_array,
                    );
                }
            } else {
                out.push(RenderPrimitive::Rect {
                    role: RenderRole::DocumentGraphic,
                    object_id: Some(object_id.to_string()),
                    node_id: None,
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    fill: style.fill,
                    stroke: style.stroke,
                    stroke_width: style.stroke_width,
                    rx: *corner_radius,
                    ry: *corner_radius,
                    dash_array: style.dash_array,
                    fill_gradient: style.fill_gradient,
                });
            }
        }
        ShapeGeometry::Oval { .. } => {
            if let Some(fill) = style.fill {
                push_shape_fill(out, object_id, geometry, fill);
            }
            if let Some(stroke) = style.stroke {
                push_shape_outline(
                    out,
                    object_id,
                    geometry,
                    stroke,
                    style.stroke_width,
                    style.dash_array,
                );
            }
        }
    }
}

pub(super) fn push_shape_shadow_path(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    shifted_shape_path: String,
    original_shape_path: String,
    fill: String,
    points: Vec<Point>,
) {
    let clip_path = shape_shadow_clip_path(&points, &original_shape_path);
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        node_id: None,
        bond_id: None,
        d: shifted_shape_path,
        points,
        fill,
        fill_rule: None,
        clip_path_d: Some(clip_path),
        clip_rule: Some("evenodd".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_shape_ellipse_fill(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    use_cubic: bool,
    fill: String,
) {
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        node_id: None,
        bond_id: None,
        d: oval_path_d(center, rx, ry, rotate, use_cubic),
        points: ellipse_bounds_points(center, rx, ry, rotate),
        fill,
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
        rotate: 0.0,
        rotate_center: None,
    });
}

pub(super) fn ellipse_bounds_points(center: Point, rx: f64, ry: f64, rotate: f64) -> Vec<Point> {
    let radians = rotate.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let extent_x = ((rx * cos) * (rx * cos) + (ry * sin) * (ry * sin)).sqrt();
    let extent_y = ((rx * sin) * (rx * sin) + (ry * cos) * (ry * cos)).sqrt();
    vec![
        Point::new(center.x - extent_x, center.y - extent_y),
        Point::new(center.x + extent_x, center.y + extent_y),
    ]
}

pub(super) fn shape_shadow_clip_path(points: &[Point], original_shape_path: &str) -> String {
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let padding = 5.0;
    let left = min_x - padding;
    let top = min_y - padding;
    let right = max_x + padding;
    let bottom = max_y + padding;
    format!(
        "M {left},{top} L {right},{top} L {right},{bottom} L {left},{bottom} L {left},{top} {original_shape_path}"
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_shape_rect_fill(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: Option<f64>,
    fill: String,
) {
    let d = if corner_radius.is_some_and(|radius| radius > crate::EPSILON) {
        rounded_rect_path_d(x, y, width, height, corner_radius.unwrap_or(0.0))
    } else {
        rect_path_d(x, y, width, height)
    };
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        node_id: None,
        bond_id: None,
        d,
        points: vec![Point::new(x, y), Point::new(x + width, y + height)],
        fill,
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
        rotate: 0.0,
        rotate_center: None,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_shaded_ellipse_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    use_cubic: bool,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    base_color: &str,
) {
    let is_circle = (rx - ry).abs() <= crate::EPSILON;
    let levels = if is_circle {
        CIRCLE_SHADED_LEVELS
    } else {
        SHADED_LEVELS
    };
    let remain_ratio = if is_circle {
        CIRCLE_SHADED_REMAIN_RATIO
    } else {
        ELLIPSE_SHADED_REMAIN_RATIO
    };
    let shift_ratio = if is_circle {
        CIRCLE_SHADED_CENTER_SHIFT_RATIO
    } else {
        ELLIPSE_SHADED_CENTER_SHIFT_RATIO
    };
    let max_index = (levels.len() - 1) as f64;
    for (index, level) in levels.iter().enumerate() {
        let t = index as f64 / max_index;
        let layer_rx = rx * (1.0 - (1.0 - remain_ratio) * t);
        let layer_ry = ry * (1.0 - (1.0 - remain_ratio) * t);
        let layer_center = center.translated(crate::Vector::new(
            -shift_ratio * rx * t,
            -shift_ratio * ry * t,
        ));
        push_shape_ellipse_fill(
            out,
            object_id,
            layer_center,
            layer_rx,
            layer_ry,
            rotate,
            use_cubic,
            shaded_level_color(base_color, level, t),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_shaded_rect_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: Option<f64>,
    rounded: bool,
    base_color: &str,
) {
    let inset_ratio = if rounded {
        ROUND_RECT_SHADED_INSET_RATIO
    } else {
        RECT_SHADED_INSET_RATIO
    };
    let max_index = (SHADED_LEVELS.len() - 1) as f64;
    for (index, level) in SHADED_LEVELS.iter().enumerate() {
        let t = index as f64 / max_index;
        let layer_x = x + width * inset_ratio * t;
        let layer_y = y + height * inset_ratio * t;
        let layer_width = width * (1.0 - (1.0 - RECT_SHADED_REMAIN_RATIO) * t);
        let layer_height = height * (1.0 - (1.0 - RECT_SHADED_REMAIN_RATIO) * t);
        let layer_radius = corner_radius.map(|radius| {
            radius
                .min(layer_width * 0.5)
                .min(layer_height * 0.5)
                .max(0.0)
        });
        push_shape_rect_fill(
            out,
            object_id,
            layer_x,
            layer_y,
            layer_width,
            layer_height,
            layer_radius,
            shaded_level_color(base_color, level, t),
        );
    }
}

pub(super) fn shaded_level_color(base_color: &str, gray: &str, t: f64) -> String {
    let Some((r, g, b)) = parse_hex_color(base_color) else {
        return gray.to_string();
    };
    if r == 0 && g == 0 && b == 0 {
        return gray.to_string();
    }
    let mix = |channel: u8| -> u8 { (channel as f64 + (255.0 - channel as f64) * t).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", mix(r), mix(g), mix(b))
}

pub(super) fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    Some((
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

pub(super) fn rounded_rect_path_d(x: f64, y: f64, width: f64, height: f64, radius: f64) -> String {
    let r = radius.min(width * 0.5).min(height * 0.5).max(0.0);
    if r <= crate::EPSILON {
        return rect_path_d(x, y, width, height);
    }
    let right = x + width;
    let bottom = y + height;
    let k = r * 0.552_284_749_830_793_6;
    format!(
        "M {x},{bottom_start} C {x},{bottom_start} {x},{top_left_c1} {x},{top_left_start} C {x},{top_left_c2} {top_left_c3},{y} {top_left_end},{y} C {top_left_end},{y} {top_right_start},{y} {top_right_start},{y} C {top_right_c1},{y} {right},{top_left_c2} {right},{top_left_start} C {right},{top_left_start} {right},{bottom_start} {right},{bottom_start} C {right},{bottom_c1} {top_right_c1},{bottom} {top_right_start},{bottom} C {top_right_start},{bottom} {top_left_end},{bottom} {top_left_end},{bottom} C {top_left_c3},{bottom} {x},{bottom_c1} {x},{bottom_start}",
        top_left_start = y + r,
        top_left_c1 = y + r,
        top_left_c2 = y + r - k,
        top_left_c3 = x + r - k,
        top_left_end = x + r,
        top_right_start = right - r,
        top_right_c1 = right - r + k,
        bottom_start = bottom - r,
        bottom_c1 = bottom - r + k,
    )
}

pub(super) fn rect_path_d(x: f64, y: f64, width: f64, height: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!(
        "M {right},{bottom} C {right},{bottom} {right},{y} {right},{y} C {right},{y} {x},{y} {x},{y} C {x},{y} {x},{bottom} {x},{bottom} C {x},{bottom} {right},{bottom} {right},{bottom}"
    )
}

pub(super) fn rotated_rect_points(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    rotate: f64,
) -> Vec<Point> {
    let center = Point::new(x + width * 0.5, y + height * 0.5);
    rotated_rect_points_around(x, y, width, height, center, rotate)
}

pub(super) fn rotated_rect_points_around(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    center: Point,
    rotate: f64,
) -> Vec<Point> {
    [
        Point::new(x, y),
        Point::new(x + width, y),
        Point::new(x + width, y + height),
        Point::new(x, y + height),
    ]
    .into_iter()
    .map(|point| rotate_point_around(point, center, rotate))
    .collect()
}

pub(super) fn rotated_rect_path_d(x: f64, y: f64, width: f64, height: f64, rotate: f64) -> String {
    let points = rotated_rect_points(x, y, width, height, rotate);
    format!(
        "M {},{} L {},{} L {},{} L {},{} Z",
        points[0].x,
        points[0].y,
        points[1].x,
        points[1].y,
        points[2].x,
        points[2].y,
        points[3].x,
        points[3].y
    )
}

pub(super) fn oval_path_d(center: Point, rx: f64, ry: f64, rotate: f64, use_cubic: bool) -> String {
    if use_cubic {
        return ellipse_cubic_path_d(center, rx, ry, rotate);
    }
    ellipse_path_d(center, rx, ry, rotate)
}

pub(super) fn ellipse_cubic_path_d(center: Point, rx: f64, ry: f64, rotate: f64) -> String {
    let k = 0.552_284_749_830_793_6;
    let major = crate::direction_from_angle(rotate);
    let minor = crate::direction_from_angle(rotate + 90.0);
    let left = center.translated(major.scaled(-rx));
    let right = center.translated(major.scaled(rx));
    let bottom = center.translated(minor.scaled(ry));
    let top = center.translated(minor.scaled(-ry));
    let c1 = left.translated(minor.scaled(k * ry));
    let c2 = bottom.translated(major.scaled(-k * rx));
    let c3 = bottom.translated(major.scaled(k * rx));
    let c4 = right.translated(minor.scaled(k * ry));
    let c5 = right.translated(minor.scaled(-k * ry));
    let c6 = top.translated(major.scaled(k * rx));
    let c7 = top.translated(major.scaled(-k * rx));
    let c8 = left.translated(minor.scaled(-k * ry));
    format!(
        "M {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{}",
        left.x,
        left.y,
        c1.x,
        c1.y,
        c2.x,
        c2.y,
        bottom.x,
        bottom.y,
        c3.x,
        c3.y,
        c4.x,
        c4.y,
        right.x,
        right.y,
        c5.x,
        c5.y,
        c6.x,
        c6.y,
        top.x,
        top.y,
        c7.x,
        c7.y,
        c8.x,
        c8.y,
        left.x,
        left.y
    )
}

pub(super) fn ellipse_path_d(center: Point, rx: f64, ry: f64, rotate: f64) -> String {
    let unit = crate::direction_from_angle(rotate);
    let start = center.translated(unit.scaled(-rx));
    let end = center.translated(unit.scaled(rx));
    format!(
        "M {},{} A {rx},{ry} {rotate} 1 0 {},{} A {rx},{ry} {rotate} 1 0 {},{} Z",
        start.x, start.y, end.x, end.y, start.x, start.y
    )
}

pub(super) fn payload_point(payload: &ObjectPayload, key: &str) -> Option<Point> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

pub(super) fn shape_shadow_fill(stroke: Option<&str>, fill: Option<&str>) -> String {
    let color = fill.or(stroke).unwrap_or("#000000");
    if color.eq_ignore_ascii_case("#000000") {
        return "rgba(0,0,0,0.247059)".to_string();
    }
    let Some((r, g, b)) = parse_hex_color(color) else {
        return color.to_string();
    };
    format!("rgba({r},{g},{b},0.247059)")
}
