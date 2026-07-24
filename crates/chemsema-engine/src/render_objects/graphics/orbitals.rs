use super::*;

pub(super) fn render_orbital_shape_object(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    style: ShapeStyleSpec,
) {
    let template =
        payload_string(&object.payload, "orbitalTemplate").unwrap_or_else(|| "s".to_string());
    let phase =
        payload_string(&object.payload, "orbitalPhase").unwrap_or_else(|| "plus".to_string());
    let stroke = style.base_color().to_string();
    let stroke_width = if style.stroke_width > crate::EPSILON {
        style.stroke_width
    } else {
        px_to_pt(1.0)
    };

    if matches!(template.as_str(), "s" | "oval") {
        let Some(center) = payload_point(&object.payload, "center") else {
            return;
        };
        let Some(major) = payload_point(&object.payload, "majorAxisEnd") else {
            return;
        };
        let Some(minor) = payload_point(&object.payload, "minorAxisEnd") else {
            return;
        };
        let rx = center.distance(major);
        let ry = center.distance(minor);
        let rotate = angle_between(center, major);
        render_orbital_ellipse_style(
            out,
            &object.id,
            center,
            rx,
            ry,
            rotate,
            true,
            &stroke,
            stroke_width,
            &style,
            true,
        );
        return;
    }

    let Some(start) = payload_point(&object.payload, "axisStart") else {
        return;
    };
    let Some(end) = payload_point(&object.payload, "axisEnd") else {
        return;
    };
    let axis = crate::Vector::new(end.x - start.x, end.y - start.y);
    let axis_len = axis.length();
    if axis_len <= crate::EPSILON {
        return;
    }
    let unit = axis.normalized();
    let normal = crate::Vector::new(-unit.y, unit.x);
    let center = start;
    let rotate = angle_between(start, end);
    let phase_positive = phase != "minus";
    match template.as_str() {
        "p" => {
            let phase_positive = true;
            let primary = orbital_lobe_geometry(center, end, P_ORBITAL_PROFILE);
            let secondary = orbital_lobe_geometry(
                center,
                center.translated(unit.scaled(-axis_len)),
                P_ORBITAL_PROFILE,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &primary,
                &stroke,
                stroke_width,
                &style,
                phase_positive,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &secondary,
                &stroke,
                stroke_width,
                &style,
                !phase_positive,
            );
        }
        "dxy" => {
            let phase_positive = true;
            let vertical = orbital_lobe_geometry(center, end, DXY_ORBITAL_PROFILE);
            let vertical_opposite = orbital_lobe_geometry(
                center,
                center.translated(unit.scaled(-axis_len)),
                DXY_ORBITAL_PROFILE,
            );
            let horizontal_tip = center.translated(normal.scaled(axis_len));
            let horizontal_opposite_tip = center.translated(normal.scaled(-axis_len));
            let horizontal = orbital_lobe_geometry(center, horizontal_tip, DXY_ORBITAL_PROFILE);
            let horizontal_opposite =
                orbital_lobe_geometry(center, horizontal_opposite_tip, DXY_ORBITAL_PROFILE);
            render_orbital_lobe_style(
                out,
                &object.id,
                &vertical,
                &stroke,
                stroke_width,
                &style,
                phase_positive,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &vertical_opposite,
                &stroke,
                stroke_width,
                &style,
                phase_positive,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &horizontal,
                &stroke,
                stroke_width,
                &style,
                !phase_positive,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &horizontal_opposite,
                &stroke,
                stroke_width,
                &style,
                !phase_positive,
            );
        }
        "hybrid" => {
            let primary = orbital_lobe_geometry(center, end, P_ORBITAL_PROFILE);
            let secondary = orbital_lobe_geometry(
                center,
                center.translated(unit.scaled(-(axis_len * 0.4))),
                P_ORBITAL_PROFILE,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &primary,
                &stroke,
                stroke_width,
                &style,
                !phase_positive,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &secondary,
                &stroke,
                stroke_width,
                &style,
                phase_positive,
            );
        }
        "dz2" => {
            let top = orbital_lobe_geometry(
                center,
                center.translated(unit.scaled(-axis_len)),
                P_ORBITAL_PROFILE,
            );
            let bottom = orbital_lobe_geometry(center, end, P_ORBITAL_PROFILE);
            render_orbital_lobe_style(
                out,
                &object.id,
                &top,
                &stroke,
                stroke_width,
                &style,
                !phase_positive,
            );
            render_orbital_lobe_style(
                out,
                &object.id,
                &bottom,
                &stroke,
                stroke_width,
                &style,
                !phase_positive,
            );
            render_orbital_ring(
                out,
                object,
                center,
                axis_len * 0.48,
                axis_len * 0.155,
                rotate - 90.0,
                &stroke,
                stroke_width,
                &style,
                phase_positive,
                true,
            );
        }
        "lobe" => {
            let lobe = orbital_lobe_geometry(center, end, P_ORBITAL_PROFILE);
            render_orbital_lobe_style(out, &object.id, &lobe, &stroke, stroke_width, &style, true);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_orbital_ellipse_style(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    _primary: bool,
    stroke: &str,
    stroke_width: f64,
    style: &ShapeStyleSpec,
    active_fill: bool,
) {
    let ellipse_fill = if active_fill {
        Some(stroke.to_string())
    } else {
        Some("#ffffff".to_string())
    };
    match style.render_style {
        ShapeRenderStyle::Filled => {
            push_shape_ellipse_fill(
                out,
                object_id,
                center,
                rx,
                ry,
                rotate,
                true,
                ellipse_fill.unwrap_or_else(|| stroke.to_string()),
            );
            push_orbital_outline(out, object_id, center, rx, ry, rotate, stroke, stroke_width);
        }
        ShapeRenderStyle::Shaded if active_fill => {
            push_shaded_ellipse_layers(out, object_id, true, center, rx, ry, rotate, stroke);
            push_orbital_outline(out, object_id, center, rx, ry, rotate, stroke, stroke_width);
        }
        ShapeRenderStyle::Shaded => {
            push_shape_ellipse_fill(
                out,
                object_id,
                center,
                rx,
                ry,
                rotate,
                true,
                "#ffffff".to_string(),
            );
            push_orbital_outline(out, object_id, center, rx, ry, rotate, stroke, stroke_width);
        }
        _ => {
            if active_fill && style.fill.is_some() {
                push_shape_ellipse_fill(
                    out,
                    object_id,
                    center,
                    rx,
                    ry,
                    rotate,
                    true,
                    stroke.to_string(),
                );
            } else if !active_fill {
                push_shape_ellipse_fill(
                    out,
                    object_id,
                    center,
                    rx,
                    ry,
                    rotate,
                    true,
                    "#ffffff".to_string(),
                );
            }
            push_orbital_outline(out, object_id, center, rx, ry, rotate, stroke, stroke_width);
        }
    }
}

pub(super) fn orbital_lobe_geometry(
    apex: Point,
    tip: Point,
    profile: OrbitalLobeProfile,
) -> OrbitalLobeGeometry {
    let axis = crate::Vector::new(tip.x - apex.x, tip.y - apex.y);
    let length = axis.length();
    let unit = axis.normalized();
    let normal = crate::Vector::new(-unit.y, unit.x);
    let local = |x_ratio: f64, y_ratio: f64| {
        apex.translated(normal.scaled(length * x_ratio))
            .translated(unit.scaled(length * y_ratio))
    };
    OrbitalLobeGeometry {
        apex,
        c1: local(profile.start_ctrl, 0.0),
        c2: local(profile.side_ctrl, profile.belly_ctrl),
        p1: local(profile.side_ctrl, profile.shoulder),
        c3: local(profile.side_ctrl, profile.tip_ctrl),
        c4: local(profile.tip_half, 1.0),
        tip,
        c5: local(-profile.tip_half, 1.0),
        c6: local(-profile.side_ctrl, profile.tip_ctrl),
        p2: local(-profile.side_ctrl, profile.shoulder),
        c7: local(-profile.side_ctrl, profile.belly_ctrl),
        c8: local(-profile.start_ctrl, 0.0),
    }
}

pub(super) fn orbital_lobe_path_d(geometry: &OrbitalLobeGeometry) -> String {
    format!(
        "M {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} C {} {} {} {} {} {} Z",
        geometry.apex.x,
        geometry.apex.y,
        geometry.c1.x,
        geometry.c1.y,
        geometry.c2.x,
        geometry.c2.y,
        geometry.p1.x,
        geometry.p1.y,
        geometry.c3.x,
        geometry.c3.y,
        geometry.c4.x,
        geometry.c4.y,
        geometry.tip.x,
        geometry.tip.y,
        geometry.c5.x,
        geometry.c5.y,
        geometry.c6.x,
        geometry.c6.y,
        geometry.p2.x,
        geometry.p2.y,
        geometry.c7.x,
        geometry.c7.y,
        geometry.c8.x,
        geometry.c8.y,
        geometry.apex.x,
        geometry.apex.y,
    )
}

pub(super) fn orbital_lobe_outline_points(geometry: &OrbitalLobeGeometry) -> Vec<Point> {
    vec![geometry.apex, geometry.p1, geometry.tip, geometry.p2]
}

pub(super) fn orbital_lobe_bounds(geometry: &OrbitalLobeGeometry) -> [f64; 4] {
    let points = [
        geometry.apex,
        geometry.c1,
        geometry.c2,
        geometry.p1,
        geometry.c3,
        geometry.c4,
        geometry.tip,
        geometry.c5,
        geometry.c6,
        geometry.p2,
        geometry.c7,
        geometry.c8,
    ];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    [min_x, min_y, max_x, max_y]
}

pub(super) fn scale_orbital_lobe_point(point: Point, focus: Point, scale: f64) -> Point {
    Point::new(
        focus.x + (point.x - focus.x) * scale,
        focus.y + (point.y - focus.y) * scale,
    )
}

pub(super) fn scaled_orbital_lobe_geometry(
    geometry: &OrbitalLobeGeometry,
    focus: Point,
    scale: f64,
) -> OrbitalLobeGeometry {
    OrbitalLobeGeometry {
        apex: scale_orbital_lobe_point(geometry.apex, focus, scale),
        c1: scale_orbital_lobe_point(geometry.c1, focus, scale),
        c2: scale_orbital_lobe_point(geometry.c2, focus, scale),
        p1: scale_orbital_lobe_point(geometry.p1, focus, scale),
        c3: scale_orbital_lobe_point(geometry.c3, focus, scale),
        c4: scale_orbital_lobe_point(geometry.c4, focus, scale),
        tip: scale_orbital_lobe_point(geometry.tip, focus, scale),
        c5: scale_orbital_lobe_point(geometry.c5, focus, scale),
        c6: scale_orbital_lobe_point(geometry.c6, focus, scale),
        p2: scale_orbital_lobe_point(geometry.p2, focus, scale),
        c7: scale_orbital_lobe_point(geometry.c7, focus, scale),
        c8: scale_orbital_lobe_point(geometry.c8, focus, scale),
    }
}

pub(super) fn push_shaded_orbital_lobe_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &OrbitalLobeGeometry,
    base_color: &str,
) {
    let [min_x, min_y, max_x, max_y] = orbital_lobe_bounds(geometry);
    let width = (max_x - min_x).max(crate::EPSILON);
    let height = (max_y - min_y).max(crate::EPSILON);
    let focus = Point::new(min_x + width * 0.33, min_y + height * 0.25);
    let max_index = (SHADED_LEVELS.len() - 1) as f64;
    for (index, level) in SHADED_LEVELS.iter().enumerate() {
        let t = index as f64 / max_index;
        let scale = 1.0 - (1.0 - ELLIPSE_SHADED_REMAIN_RATIO) * t;
        let layer = scaled_orbital_lobe_geometry(geometry, focus, scale);
        out.push(RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id.to_string()),
            node_id: None,
            bond_id: None,
            d: orbital_lobe_path_d(&layer),
            points: orbital_lobe_outline_points(&layer),
            fill: shaded_level_color(base_color, level, t),
            fill_rule: Some("evenodd".to_string()),
            clip_path_d: None,
            clip_rule: None,
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

pub(super) fn render_orbital_lobe_style(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    geometry: &OrbitalLobeGeometry,
    stroke: &str,
    stroke_width: f64,
    style: &ShapeStyleSpec,
    active_fill: bool,
) {
    let suppress_outline = style.render_style == ShapeRenderStyle::Filled && active_fill;
    match style.render_style {
        ShapeRenderStyle::Filled if active_fill => {
            out.push(RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object_id.to_string()),
                node_id: None,
                bond_id: None,
                d: orbital_lobe_path_d(geometry),
                points: orbital_lobe_outline_points(geometry),
                fill: stroke.to_string(),
                fill_rule: Some("evenodd".to_string()),
                clip_path_d: None,
                clip_rule: None,
                rotate: 0.0,
                rotate_center: None,
            });
        }
        ShapeRenderStyle::Shaded if active_fill => {
            push_shaded_orbital_lobe_layers(out, object_id, geometry, stroke);
        }
        _ => {
            out.push(RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object_id.to_string()),
                node_id: None,
                bond_id: None,
                d: orbital_lobe_path_d(geometry),
                points: orbital_lobe_outline_points(geometry),
                fill: "#ffffff".to_string(),
                fill_rule: Some("evenodd".to_string()),
                clip_path_d: None,
                clip_rule: None,
                rotate: 0.0,
                rotate_center: None,
            });
        }
    }
    if !suppress_outline {
        out.push(RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id.to_string()),
            bond_id: None,
            d: orbital_lobe_path_d(geometry),
            points: orbital_lobe_outline_points(geometry),
            stroke: stroke.to_string(),
            stroke_width,
            dash_array: Vec::new(),
            line_cap: None,
            line_join: Some("bevel".to_string()),
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_orbital_ring(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    stroke: &str,
    stroke_width: f64,
    style: &ShapeStyleSpec,
    active_fill: bool,
    flip_y: bool,
) {
    let ring_path = dz2_ring_path_d(center, rx, ry, rotate, flip_y);
    let ring_bounds = ellipse_bounds_points(center, rx, ry, rotate);
    let suppress_outline = style.render_style == ShapeRenderStyle::Filled && active_fill;
    match style.render_style {
        ShapeRenderStyle::Filled if active_fill => {
            out.push(RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object.id.clone()),
                node_id: None,
                bond_id: None,
                d: ring_path.clone(),
                points: ring_bounds.clone(),
                fill: stroke.to_string(),
                fill_rule: Some("evenodd".to_string()),
                clip_path_d: None,
                clip_rule: None,
                rotate: 0.0,
                rotate_center: None,
            });
        }
        ShapeRenderStyle::Shaded if active_fill => {
            push_shaded_dz2_ring_layers(out, &object.id, center, rx, ry, rotate, flip_y);
        }
        ShapeRenderStyle::Filled | ShapeRenderStyle::Shaded => {
            out.push(RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object.id.clone()),
                node_id: None,
                bond_id: None,
                d: ring_path.clone(),
                points: ring_bounds.clone(),
                fill: "#ffffff".to_string(),
                fill_rule: Some("evenodd".to_string()),
                clip_path_d: None,
                clip_rule: None,
                rotate: 0.0,
                rotate_center: None,
            });
        }
        _ => {}
    }
    if !suppress_outline {
        out.push(RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            bond_id: None,
            d: ring_path,
            points: ring_bounds,
            stroke: stroke.to_string(),
            stroke_width,
            dash_array: Vec::new(),
            line_cap: None,
            line_join: Some("bevel".to_string()),
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

pub(super) fn dz2_ring_path_d(
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    flip_y: bool,
) -> String {
    let outer_cx = 0.499;
    let inner_cx = 0.315;
    let center_cx = 0.239;
    let lower_ry = 0.923;
    let center_ry = 0.579;
    let radians = rotate.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let rotate_point = |point: Point| {
        let point = if flip_y {
            Point::new(point.x, (center.y * 2.0) - point.y)
        } else {
            point
        };
        if rotate.abs() <= crate::EPSILON {
            return point;
        }
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        Point::new(
            center.x + (dx * cos) - (dy * sin),
            center.y + (dx * sin) + (dy * cos),
        )
    };
    let x0 = center.x - rx;
    let x1 = center.x + rx;
    let cx = center.x;
    let cy = center.y;
    let y_top = center.y - ry;
    let y_top_ctrl = center.y - (ry * 0.725);
    let y_bottom_ctrl = center.y + (ry * center_ry);
    let y_lower = center.y + (ry * lower_ry);
    let y_center_approach = center.y + (ry * center_ry);
    let x_outer = center.x + (rx * outer_cx);
    let x_outer_left = center.x - (rx * outer_cx);
    let x_outer_right = center.x + (rx * outer_cx);
    let x_inner_left = center.x - (rx * inner_cx);
    let x_inner_right = center.x + (rx * inner_cx);
    let x_approach_left = center.x - (rx * inner_cx * 0.76);
    let x_approach_right = center.x + (rx * inner_cx * 0.76);
    let x_center_left = center.x - (rx * center_cx);
    let x_center_right = center.x + (rx * center_cx);
    let p0 = rotate_point(Point::new(x1, cy));
    let c1 = rotate_point(Point::new(x1, y_top_ctrl));
    let c2 = rotate_point(Point::new(x_outer, y_top));
    let p1 = rotate_point(Point::new(cx, y_top));
    let c3 = rotate_point(Point::new(x_inner_left, y_top));
    let c4 = rotate_point(Point::new(x0, y_top_ctrl));
    let p2 = rotate_point(Point::new(x0, cy));
    let c5 = rotate_point(Point::new(x0, y_bottom_ctrl));
    let c6 = rotate_point(Point::new(x_outer_left, y_lower));
    let p3 = rotate_point(Point::new(x_inner_left, y_lower));
    let c7 = rotate_point(Point::new(x_approach_left, y_center_approach));
    let c8 = rotate_point(Point::new(x_center_left, cy));
    let p4 = rotate_point(Point::new(cx, cy));
    let c9 = rotate_point(Point::new(x_center_right, cy));
    let c10 = rotate_point(Point::new(x_approach_right, y_center_approach));
    let p5 = rotate_point(Point::new(x_inner_right, y_lower));
    let c11 = rotate_point(Point::new(x_outer_right, y_lower));
    let c12 = rotate_point(Point::new(x1, y_bottom_ctrl));
    format!(
        "M {:.4} {:.4} C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} Z",
        p0.x, p0.y,
        c1.x, c1.y, c2.x, c2.y, p1.x, p1.y,
        c3.x, c3.y, c4.x, c4.y, p2.x, p2.y,
        c5.x, c5.y, c6.x, c6.y, p3.x, p3.y,
        c7.x, c7.y, c8.x, c8.y, p4.x, p4.y,
        c9.x, c9.y, c10.x, c10.y, p5.x, p5.y,
        c11.x, c11.y, c12.x, c12.y, p0.x, p0.y,
    )
}

pub(super) fn push_shaded_dz2_ring_layers(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    flip_y: bool,
) {
    let clip_path = dz2_ring_path_d(center, rx, ry, rotate, flip_y);
    let max_index = (SHADED_LEVELS.len() - 1) as f64;
    for (index, level) in SHADED_LEVELS.iter().enumerate() {
        let t = index as f64 / max_index;
        let layer_rx = rx * (1.0 - (1.0 - ELLIPSE_SHADED_REMAIN_RATIO) * t);
        let layer_ry = ry * (1.0 - (1.0 - ELLIPSE_SHADED_REMAIN_RATIO) * t);
        let y_shift_sign = if flip_y { 1.0 } else { -1.0 };
        let layer_center = center.translated(crate::Vector::new(
            -ELLIPSE_SHADED_CENTER_SHIFT_RATIO * rx * t,
            y_shift_sign * ELLIPSE_SHADED_CENTER_SHIFT_RATIO * ry * t,
        ));
        out.push(RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object_id.to_string()),
            node_id: None,
            bond_id: None,
            d: oval_path_d(layer_center, layer_rx, layer_ry, rotate, true),
            points: ellipse_bounds_points(layer_center, layer_rx, layer_ry, rotate),
            fill: (*level).to_string(),
            fill_rule: None,
            clip_path_d: Some(clip_path.clone()),
            clip_rule: Some("evenodd".to_string()),
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_orbital_outline(
    out: &mut Vec<RenderPrimitive>,
    object_id: &str,
    center: Point,
    rx: f64,
    ry: f64,
    rotate: f64,
    stroke: &str,
    stroke_width: f64,
) {
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object_id.to_string()),
        bond_id: None,
        d: oval_path_d(center, rx, ry, rotate, true),
        points: rotated_rect_points(center.x - rx, center.y - ry, rx * 2.0, ry * 2.0, rotate),
        stroke: stroke.to_string(),
        stroke_width,
        dash_array: Vec::new(),
        line_cap: Some("round".to_string()),
        line_join: Some("round".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
}
