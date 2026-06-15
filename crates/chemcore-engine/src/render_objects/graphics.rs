use super::*;
use crate::angle_between;

#[derive(Clone, Copy)]
struct OrbitalLobeProfile {
    start_ctrl: f64,
    side_ctrl: f64,
    belly_ctrl: f64,
    shoulder: f64,
    tip_ctrl: f64,
    tip_half: f64,
}

#[derive(Clone, Copy)]
struct OrbitalLobeGeometry {
    apex: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    c3: Point,
    c4: Point,
    tip: Point,
    c5: Point,
    c6: Point,
    p2: Point,
    c7: Point,
    c8: Point,
}

// These lobe profiles are calibrated against ChemDraw's exported orbital templates.
// Keep them centralized so geometry tweaks stay explicit instead of drifting as inline literals.
const P_ORBITAL_PROFILE: OrbitalLobeProfile = OrbitalLobeProfile {
    start_ctrl: 0.156,
    side_ctrl: 0.291,
    belly_ctrl: 0.51,
    shoulder: 0.667,
    tip_ctrl: 0.86,
    tip_half: 0.25,
};

const DXY_ORBITAL_PROFILE: OrbitalLobeProfile = OrbitalLobeProfile {
    start_ctrl: 0.0,
    side_ctrl: 0.352,
    belly_ctrl: 0.357,
    shoulder: 0.668,
    tip_ctrl: 0.86,
    tip_half: 0.25,
};

pub(crate) fn render_shape_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
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
    if payload_string(&object.payload, "kind").as_deref() == Some("crossTable") {
        render_cross_table_shape_object(out, object, style);
        return;
    }
    let Some(geometry) = ShapeGeometry::from_object(object) else {
        return;
    };
    render_shape_geometry(out, &object.id, &geometry, style);
}

fn render_orbital_shape_object(
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

fn render_cross_table_shape_object(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    style: ShapeStyleSpec,
) {
    let Some([x, y, width, height]) = object.payload.bbox else {
        return;
    };
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return;
    }
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let outer = ShapeGeometry::Rect {
        x: tx,
        y: ty,
        width,
        height,
        corner_radius: None,
        rounded: false,
        rotate: object.transform.rotate,
    };
    render_shape_geometry(out, &object.id, &outer, style.clone());

    let stroke = style
        .stroke
        .clone()
        .unwrap_or_else(|| style.base_color().to_string());
    let stroke_width = if style.stroke_width > crate::EPSILON {
        style.stroke_width
    } else {
        px_to_pt(1.0)
    };
    let dash_array = style.dash_array;
    let mid_x = tx + width * 0.5;
    let mid_y = ty + height * 0.5;
    let vertical = vec![Point::new(mid_x, ty), Point::new(mid_x, ty + height)];
    let horizontal = vec![Point::new(tx, mid_y), Point::new(tx + width, mid_y)];
    for points in [vertical, horizontal] {
        let d = format!(
            "M {:.4} {:.4} L {:.4} {:.4}",
            points[0].x, points[0].y, points[1].x, points[1].y
        );
        out.push(RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            bond_id: None,
            d,
            points,
            stroke: stroke.clone(),
            stroke_width,
            dash_array: dash_array.clone(),
            line_cap: Some("square".to_string()),
            line_join: Some("miter".to_string()),
            rotate: object.transform.rotate,
            rotate_center: (object.transform.rotate.abs() > crate::EPSILON)
                .then_some(Point::new(tx + width * 0.5, ty + height * 0.5)),
        });
    }
}

fn render_tlc_plate_shape_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    style: ShapeStyleSpec,
) {
    let Some([x, y, width, height]) = object.payload.bbox else {
        return;
    };
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return;
    }
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let rotate = object.transform.rotate;
    let rotate_center =
        (rotate.abs() > crate::EPSILON).then_some(Point::new(tx + width * 0.5, ty + height * 0.5));
    let stroke = style
        .stroke
        .clone()
        .unwrap_or_else(|| style.base_color().to_string());
    let stroke_width = if style.stroke_width > crate::EPSILON {
        style.stroke_width
    } else {
        px_to_pt(1.0)
    };
    let dash_spacing = payload_number(&object.payload, "dashSpacing")
        .unwrap_or(crate::DEFAULT_HASH_SPACING_PT.value());
    let editing_scale = (object.meta.get("source").and_then(JsonValue::as_str) == Some("cdxml"))
        .then(|| cdxml_editing_scale(document))
        .flatten()
        .unwrap_or(1.0);
    if payload_bool(&object.payload, "showBorders").unwrap_or(true) {
        out.push(RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            node_id: None,
            x: tx,
            y: ty,
            width,
            height,
            fill: Some(style.fill.clone().unwrap_or_else(|| "#ffffff".to_string())),
            stroke: Some(stroke.clone()),
            stroke_width,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        });
    }
    let origin_fraction = payload_number(&object.payload, "originFraction").unwrap_or(0.1);
    let solvent_fraction = payload_number(&object.payload, "solventFrontFraction").unwrap_or(0.1);
    let origin_y = ty + height * (1.0 - origin_fraction);
    let solvent_y = ty + height * solvent_fraction;
    if payload_bool(&object.payload, "showOrigin").unwrap_or(true) {
        push_tlc_graphic_line(
            out,
            object,
            Point::new(tx, origin_y),
            Point::new(tx + width, origin_y),
            &stroke,
            stroke_width,
            vec![dash_spacing],
            rotate,
            rotate_center,
        );
    }
    if payload_bool(&object.payload, "showSolventFront").unwrap_or(true) {
        push_tlc_graphic_line(
            out,
            object,
            Point::new(tx, solvent_y),
            Point::new(tx + width, solvent_y),
            &stroke,
            stroke_width,
            vec![dash_spacing],
            rotate,
            rotate_center,
        );
    }
    let show_side_ticks = payload_bool(&object.payload, "showSideTicks").unwrap_or(true);
    let tick_half = 3.0 * editing_scale;
    let lanes = object
        .payload
        .extra
        .get("lanes")
        .and_then(serde_json::Value::as_array)
        .map(|value| value.clone())
        .unwrap_or_default();
    for lane in lanes {
        let offset = lane
            .get("offset")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.5);
        let lane_x = tx + width * offset;
        if show_side_ticks {
            push_tlc_graphic_line(
                out,
                object,
                Point::new(lane_x, origin_y - tick_half),
                Point::new(lane_x, origin_y + tick_half),
                &stroke,
                stroke_width,
                Vec::new(),
                rotate,
                rotate_center,
            );
        }
        for spot in lane
            .get("spots")
            .and_then(serde_json::Value::as_array)
            .map(|value| value.clone())
            .unwrap_or_default()
        {
            let rf = spot
                .get("rf")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.15);
            let spot_y = origin_y - (origin_y - solvent_y) * rf;
            let spot_radius = spot
                .get("width")
                .and_then(serde_json::Value::as_f64)
                .or_else(|| spot.get("height").and_then(serde_json::Value::as_f64))
                .map(|diameter| (diameter * 0.5).clamp(2.0, 10.0))
                .unwrap_or_else(|| (width.min(height) * 0.015).clamp(2.0, 5.0));
            out.push(RenderPrimitive::Circle {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object.id.clone()),
                node_id: None,
                center: Point::new(lane_x, spot_y),
                radius: spot_radius,
                fill: stroke.clone(),
                stroke: stroke.clone(),
                stroke_width: 0.0,
            });
        }
    }
}

fn push_tlc_graphic_line(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    from: Point,
    to: Point,
    stroke: &str,
    stroke_width: f64,
    dash_array: Vec<f64>,
    rotate: f64,
    rotate_center: Option<Point>,
) {
    let points = vec![from, to];
    let d = format!("M {:.4} {:.4} L {:.4} {:.4}", from.x, from.y, to.x, to.y);
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        bond_id: None,
        d,
        points,
        stroke: stroke.to_string(),
        stroke_width,
        dash_array,
        line_cap: Some("butt".to_string()),
        line_join: Some("miter".to_string()),
        rotate,
        rotate_center,
    });
}

pub(crate) fn render_bracket_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let Some([x, y, width, height]) = object.payload.bbox else {
        return;
    };
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return;
    }
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let rotate = object.transform.rotate;
    let rotate_center = Point::new(tx + width * 0.5, ty + height * 0.5);
    let kind = payload_string(&object.payload, "kind").unwrap_or_else(|| "round".to_string());
    let bounds = bracket_path_bounds(tx, ty, width, height, &kind, rotate_center, rotate);
    let transform_center = (rotate.abs() > crate::EPSILON).then_some(rotate_center);
    if object.object_type == "symbol" {
        let symbol_layout_scale = cdxml_editing_scale(document).unwrap_or(1.0);
        render_symbol_object_geometry(
            out,
            object,
            tx,
            ty,
            width,
            height,
            &kind,
            bounds,
            rotate,
            transform_center,
            symbol_layout_scale,
        );
        return;
    }

    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        bond_id: None,
        d: bracket_pair_path_d(tx, ty, width, height, &kind),
        points: bounds,
        stroke: payload_string(&object.payload, "stroke").unwrap_or_else(|| "#000000".to_string()),
        stroke_width: payload_number(&object.payload, "strokeWidth").unwrap_or(px_to_pt(1.0)),
        dash_array: Vec::new(),
        line_cap: Some("butt".to_string()),
        line_join: Some(if kind == "curly" { "round" } else { "miter" }.to_string()),
        rotate,
        rotate_center: transform_center,
    });
}

fn bracket_pair_path_d(x: f64, y: f64, width: f64, height: f64, kind: &str) -> String {
    let right = x + width;
    let bottom = y + height;
    match kind {
        "square" => {
            let lip = square_bracket_lip(width, height);
            format!(
                "M {},{} L {},{} L {},{} L {},{} M {},{} L {},{} L {},{} L {},{}",
                x + lip,
                y,
                x,
                y,
                x,
                bottom,
                x + lip,
                bottom,
                right - lip,
                y,
                right,
                y,
                right,
                bottom,
                right - lip,
                bottom
            )
        }
        "curly" => {
            let depth = curly_bracket_depth(width, height);
            let half_depth = depth * 0.5;
            let middle = y + height * 0.5;
            let c_large = height * 0.039805;
            let c_small = height * 0.032308;
            let left_end = x + depth;
            let left_mid = x + half_depth;
            let right_end = right - depth;
            let right_mid = right - half_depth;
            let top_inner = y + half_depth;
            let bottom_inner = bottom - half_depth;
            format!(
                concat!(
                    "M {le},{y} ",
                    "C {le_c},{y} {lm},{y_cs} {lm},{ti} ",
                    "C {lm},{ti} {lm},{mti} {lm},{mti} ",
                    "C {lm},{mti_c} {lm_c},{middle} {x},{middle} ",
                    "C {lm_c},{middle} {lm},{mbi_c} {lm},{mbi} ",
                    "C {lm},{mbi} {lm},{b_cs} {le_c},{bottom} ",
                    "C {le},{bottom} {le},{bottom} {le},{bottom} ",
                    "M {re},{bottom} ",
                    "C {re_c},{bottom} {rm},{b_cs} {rm},{bi} ",
                    "C {rm},{bi} {rm},{mbi} {rm},{mbi} ",
                    "C {rm},{mbi_c} {rm_c},{middle} {right},{middle} ",
                    "C {rm_c},{middle} {rm},{mti_c} {rm},{mti} ",
                    "C {rm},{mti} {rm},{y_cs} {re_c},{y} ",
                    "C {re},{y} {re},{y} {re},{y}"
                ),
                le = left_end,
                le_c = left_end - c_large,
                lm = left_mid,
                lm_c = left_mid - c_small,
                re = right_end,
                re_c = right_end + c_large,
                rm = right_mid,
                rm_c = right_mid + c_small,
                x = x,
                right = right,
                y = y,
                bottom = bottom,
                middle = middle,
                y_cs = y + c_small,
                b_cs = bottom - c_small,
                ti = top_inner,
                bi = bottom_inner,
                mti = middle - half_depth,
                mbi = middle + half_depth,
                mti_c = middle - half_depth + c_large,
                mbi_c = middle + half_depth - c_large,
            )
        }
        _ => {
            format!(
                "M {},{} A {height},{height} 0 0 0 {},{} M {},{} A {height},{height} 0 0 0 {},{}",
                x, y, x, bottom, right, bottom, right, y
            )
        }
    }
}

fn bracket_path_bounds(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    kind: &str,
    rotate_center: Point,
    rotate: f64,
) -> Vec<Point> {
    if kind == "round" {
        let depth = round_bracket_depth(width, height);
        return rotated_rect_points_around(
            x - depth,
            y,
            width + depth * 2.0,
            height,
            rotate_center,
            rotate,
        );
    }
    rotated_rect_points_around(x, y, width, height, rotate_center, rotate)
}

fn square_bracket_lip(width: f64, height: f64) -> f64 {
    (height * 0.07248).min(width * 0.22).max(0.0)
}

fn round_bracket_depth(width: f64, height: f64) -> f64 {
    (height * (1.0 - 3.0_f64.sqrt() * 0.5))
        .min(width * 0.22)
        .max(0.0)
}

fn curly_bracket_depth(width: f64, height: f64) -> f64 {
    (height * 0.14423).min(width * 0.24).max(0.0)
}

fn cdxml_editing_scale(document: &ChemcoreDocument) -> Option<f64> {
    document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn bracket_symbol_path_d(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    kind: &str,
    thick: f64,
) -> String {
    let thick = thick.min(width * 0.35).min(height * 0.18);
    let cx = x + width * 0.5;
    let vertical = rect_path_d(cx - thick * 0.5, y, thick, height);
    let top_bar = rect_path_d(x, y + height * 0.28 - thick * 0.5, width, thick);
    if kind == "double-dagger" {
        let bottom_bar = rect_path_d(x, y + height * 0.72 - thick * 0.5, width, thick);
        format!("{vertical} {top_bar} {bottom_bar}")
    } else {
        format!("{vertical} {top_bar}")
    }
}

fn render_symbol_object_geometry(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    kind: &str,
    bounds: Vec<Point>,
    rotate: f64,
    rotate_center: Option<Point>,
    layout_scale: f64,
) {
    let fill = payload_string(&object.payload, "fill").unwrap_or_else(|| "#000000".to_string());
    let stroke_width = payload_number(&object.payload, "strokeWidth").unwrap_or(px_to_pt(1.0));
    let object_id = Some(object.id.clone());
    let symbol_style = payload_string(&object.payload, "symbolStyle")
        .map(|style| crate::cdxml_symbol_style_from_name(&style))
        .unwrap_or(crate::CdxmlSymbolStyle::Default);
    let layout = charge_symbol_layout(symbol_style);
    let layout = layout.scaled(layout_scale);
    match kind {
        "circle-plus" | "circle-minus" => {
            let center = Point::new(x + width * 0.5, y + height * 0.5);
            out.push(RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                object_id: object_id.clone(),
                bond_id: None,
                d: ellipse_path_d(center, width * 0.5, height * 0.5, 0.0),
                points: bounds.clone(),
                stroke: fill.clone(),
                stroke_width,
                dash_array: Vec::new(),
                line_cap: None,
                line_join: None,
                rotate,
                rotate_center,
            });
            let sign_x = center.x - layout.circle_sign_size * 0.5 + layout.circle_sign_offset;
            let sign_y = center.y - layout.circle_sign_size * 0.5 + layout.circle_sign_offset;
            if kind == "circle-plus" {
                push_symbol_filled_paths(
                    out,
                    object_id,
                    plus_symbol_path_ds_with_thick(
                        sign_x,
                        sign_y,
                        layout.circle_sign_size,
                        layout.circle_sign_size,
                        layout.sign_thickness,
                    ),
                    bounds,
                    rotate,
                    rotate_center,
                    &fill,
                );
            } else {
                push_symbol_filled_path(
                    out,
                    object_id,
                    minus_symbol_path_d_with_thick(
                        sign_x,
                        sign_y,
                        layout.circle_sign_size,
                        layout.circle_sign_size,
                        layout.sign_thickness,
                    ),
                    bounds,
                    rotate,
                    rotate_center,
                    &fill,
                );
            }
        }
        "plus" => push_symbol_filled_paths(
            out,
            object_id,
            plus_symbol_path_ds_with_thick(x, y, width, height, layout.sign_thickness),
            bounds,
            rotate,
            rotate_center,
            &fill,
        ),
        "minus" => push_symbol_filled_path(
            out,
            object_id,
            minus_symbol_path_d_with_thick(x, y, width, height, layout.sign_thickness),
            bounds,
            rotate,
            rotate_center,
            &fill,
        ),
        "radical-cation" => {
            let mut paths = vec![dot_symbol_path_d(
                x + layout.dot_diameter * 0.5,
                y + height * 0.5,
                layout.dot_diameter,
            )];
            paths.extend(plus_symbol_path_ds_with_thick(
                x + layout.dot_diameter + layout.radical_gap,
                y + (height - layout.radical_sign_size) * 0.5,
                layout.radical_sign_size,
                layout.radical_sign_size,
                layout.sign_thickness,
            ));
            push_symbol_filled_paths(out, object_id, paths, bounds, rotate, rotate_center, &fill);
        }
        "radical-anion" => push_symbol_filled_paths(
            out,
            object_id,
            vec![
                dot_symbol_path_d(
                    x + layout.dot_diameter * 0.5,
                    y + height * 0.5,
                    layout.dot_diameter,
                ),
                minus_symbol_path_d_with_thick(
                    x + layout.dot_diameter + layout.radical_gap,
                    y + (height - layout.radical_sign_size) * 0.5,
                    layout.radical_sign_size,
                    layout.radical_sign_size,
                    layout.sign_thickness,
                ),
            ],
            bounds,
            rotate,
            rotate_center,
            &fill,
        ),
        "lone-pair" => push_symbol_filled_paths(
            out,
            object_id,
            vec![
                dot_symbol_path_d(
                    x + layout.dot_diameter * 0.5,
                    y + height * 0.5,
                    layout.dot_diameter,
                ),
                dot_symbol_path_d(
                    x + layout.dot_diameter + layout.lone_pair_gap + layout.dot_diameter * 0.5,
                    y + height * 0.5,
                    layout.dot_diameter,
                ),
            ],
            bounds,
            rotate,
            rotate_center,
            &fill,
        ),
        "electron" => {
            let diameter = width.min(height);
            push_symbol_filled_path(
                out,
                object_id,
                dot_symbol_path_d(x + width * 0.5, y + height * 0.5, diameter),
                bounds,
                rotate,
                rotate_center,
                &fill,
            );
        }
        _ => push_symbol_filled_path(
            out,
            object_id,
            bracket_symbol_path_d(x, y, width, height, kind, layout.sign_thickness),
            bounds,
            rotate,
            rotate_center,
            &fill,
        ),
    }
}

#[derive(Debug, Clone, Copy)]
struct ChargeSymbolLayout {
    circle_sign_size: f64,
    circle_sign_offset: f64,
    radical_sign_size: f64,
    sign_thickness: f64,
    dot_diameter: f64,
    radical_gap: f64,
    lone_pair_gap: f64,
}

fn charge_symbol_layout(style: crate::CdxmlSymbolStyle) -> ChargeSymbolLayout {
    match style {
        crate::CdxmlSymbolStyle::Default => ChargeSymbolLayout {
            circle_sign_size: 4.3335,
            circle_sign_offset: -0.01675,
            radical_sign_size: 4.3335,
            sign_thickness: 0.8,
            dot_diameter: 1.667,
            radical_gap: 0.7495,
            lone_pair_gap: 2.083,
        },
        crate::CdxmlSymbolStyle::Acs => ChargeSymbolLayout {
            circle_sign_size: 3.9335,
            circle_sign_offset: -0.01675,
            radical_sign_size: 2.2,
            sign_thickness: 0.5,
            dot_diameter: 0.8,
            radical_gap: 0.3,
            lone_pair_gap: 1.0,
        },
    }
}

impl ChargeSymbolLayout {
    fn scaled(self, factor: f64) -> Self {
        Self {
            circle_sign_size: self.circle_sign_size * factor,
            circle_sign_offset: self.circle_sign_offset * factor,
            radical_sign_size: self.radical_sign_size * factor,
            sign_thickness: self.sign_thickness * factor,
            dot_diameter: self.dot_diameter * factor,
            radical_gap: self.radical_gap * factor,
            lone_pair_gap: self.lone_pair_gap * factor,
        }
    }
}

fn push_symbol_filled_path(
    out: &mut Vec<RenderPrimitive>,
    object_id: Option<String>,
    d: String,
    bounds: Vec<Point>,
    rotate: f64,
    rotate_center: Option<Point>,
    fill: &str,
) {
    out.push(RenderPrimitive::FilledPath {
        role: RenderRole::DocumentGraphic,
        object_id,
        node_id: None,
        bond_id: None,
        d,
        points: bounds,
        fill: fill.to_string(),
        fill_rule: None,
        clip_path_d: None,
        clip_rule: None,
        rotate,
        rotate_center,
    });
}

fn push_symbol_filled_paths(
    out: &mut Vec<RenderPrimitive>,
    object_id: Option<String>,
    paths: Vec<String>,
    bounds: Vec<Point>,
    rotate: f64,
    rotate_center: Option<Point>,
    fill: &str,
) {
    for d in paths {
        push_symbol_filled_path(
            out,
            object_id.clone(),
            d,
            bounds.clone(),
            rotate,
            rotate_center,
            fill,
        );
    }
}

fn plus_symbol_path_ds_with_thick(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    thick: f64,
) -> Vec<String> {
    let cx = x + width * 0.5;
    let cy = y + height * 0.5;
    vec![
        symbol_rect_path_d(x, cy - thick * 0.5, width, thick),
        symbol_rect_path_d(cx - thick * 0.5, y, thick, height),
    ]
}

fn minus_symbol_path_d_with_thick(x: f64, y: f64, width: f64, height: f64, thick: f64) -> String {
    let cy = y + height * 0.5;
    symbol_rect_path_d(x, cy - thick * 0.5, width, thick)
}

fn dot_symbol_path_d(cx: f64, cy: f64, diameter: f64) -> String {
    let radius = diameter * 0.5;
    format!(
        "M {},{} A {r},{r} 0 1 0 {},{} A {r},{r} 0 1 0 {},{} Z",
        cx - radius,
        cy,
        cx + radius,
        cy,
        cx - radius,
        cy,
        r = radius
    )
}

fn symbol_rect_path_d(x: f64, y: f64, width: f64, height: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!("M {x},{y} L {right},{y} L {right},{bottom} L {x},{bottom} Z")
}

#[derive(Clone)]
struct ShapeStyleSpec {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: f64,
    dash_array: Vec<f64>,
    fill_gradient: Option<JsonValue>,
    render_style: ShapeRenderStyle,
    shadow_size: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeRenderStyle {
    Solid,
    Dashed,
    Filled,
    Shaded,
    Shadowed,
    Custom,
}

impl ShapeStyleSpec {
    fn from_style(style: Option<&JsonValue>) -> Self {
        let fill = style.and_then(|value| style_nullable_string(value, "fill"));
        let stroke = style.and_then(|value| style_nullable_string(value, "stroke"));
        let stroke_width = style
            .and_then(|value| {
                style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
            })
            .unwrap_or(px_to_pt(1.0));
        let dash_array = style
            .and_then(|value| style_number_array(value, "dashArray"))
            .unwrap_or_default();
        let fill_gradient = style
            .and_then(|value| value.get("fillGradient").cloned())
            .filter(|value| !value.is_null());
        let shaded = style
            .and_then(|value| value.get("shaded"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let shadowed = style
            .and_then(|value| value.get("shadow"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let shadow_size = style
            .and_then(|value| style_number(value, "shadowSize"))
            .unwrap_or(4.0);
        let render_style = if shaded {
            ShapeRenderStyle::Shaded
        } else if shadowed {
            ShapeRenderStyle::Shadowed
        } else if fill.is_some() && stroke.is_none() && fill_gradient.is_none() {
            ShapeRenderStyle::Filled
        } else if fill.is_none() && stroke.is_some() && !dash_array.is_empty() {
            ShapeRenderStyle::Dashed
        } else if fill.is_none() && stroke.is_some() && dash_array.is_empty() {
            ShapeRenderStyle::Solid
        } else {
            ShapeRenderStyle::Custom
        };
        Self {
            fill,
            stroke,
            stroke_width,
            dash_array,
            fill_gradient,
            render_style,
            shadow_size,
        }
    }

    fn base_color(&self) -> &str {
        self.stroke
            .as_deref()
            .or(self.fill.as_deref())
            .unwrap_or("#000000")
    }
}

enum ShapeGeometry {
    Oval {
        center: Point,
        rx: f64,
        ry: f64,
        rotate: f64,
        ellipse: bool,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        corner_radius: Option<f64>,
        rounded: bool,
        rotate: f64,
    },
}

impl ShapeGeometry {
    fn from_object(object: &SceneObject) -> Option<Self> {
        let [tx, ty] = object.transform.translate;
        let kind = payload_string(&object.payload, "kind").unwrap_or_else(|| "rect".to_string());
        if matches!(kind.as_str(), "circle" | "ellipse") {
            let center = payload_point(&object.payload, "center")?;
            let major_axis_end = payload_point(&object.payload, "majorAxisEnd")?;
            let minor_axis_end = payload_point(&object.payload, "minorAxisEnd")?;
            let rx = center.distance(major_axis_end);
            let ry = center.distance(minor_axis_end);
            if rx <= crate::EPSILON || ry <= crate::EPSILON {
                return None;
            }
            return Some(Self::Oval {
                center,
                rx,
                ry,
                rotate: crate::angle_between(center, major_axis_end),
                ellipse: kind == "ellipse",
            });
        }

        let [_, _, width, height] = object.payload.bbox?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let corner_radius =
            payload_number(&object.payload, "cornerRadius").filter(|value| *value > 0.0);
        Some(Self::Rect {
            x: tx,
            y: ty,
            width,
            height,
            corner_radius,
            rounded: kind == "roundRect",
            rotate: object.transform.rotate,
        })
    }

    fn fill_path_d(&self) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(center, rx, ry, rotate, ellipse),
            Self::Rect {
                x,
                y,
                width,
                height,
                corner_radius,
                rounded,
                rotate,
            } => {
                if rotate.abs() > crate::EPSILON {
                    rotated_rect_path_d(x, y, width, height, rotate)
                } else if rounded {
                    rounded_rect_path_d(x, y, width, height, corner_radius.unwrap_or(0.0))
                } else {
                    rect_path_d(x, y, width, height)
                }
            }
        }
    }

    fn outline_path_d(&self, dash_array: &[f64]) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(center, rx, ry, rotate, ellipse || !dash_array.is_empty()),
            _ => self.fill_path_d(),
        }
    }

    fn shifted_fill_path_d(&self, dx: f64, dy: f64) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(
                center.translated(crate::Vector::new(dx, dy)),
                rx,
                ry,
                rotate,
                ellipse,
            ),
            Self::Rect {
                x,
                y,
                width,
                height,
                corner_radius,
                rounded,
                rotate,
            } => {
                if rotate.abs() > crate::EPSILON {
                    rotated_rect_path_d(x + dx, y + dy, width, height, rotate)
                } else if rounded {
                    rounded_rect_path_d(x + dx, y + dy, width, height, corner_radius.unwrap_or(0.0))
                } else {
                    rect_path_d(x + dx, y + dy, width, height)
                }
            }
        }
    }

    fn bounds_points(&self) -> Vec<Point> {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ..
            } => ellipse_bounds_points(center, rx, ry, rotate),
            Self::Rect {
                x,
                y,
                width,
                height,
                rotate,
                ..
            } => rotated_rect_points(x, y, width, height, rotate),
        }
    }

    fn shadow_bounds_points(&self, offset: f64) -> Vec<Point> {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ..
            } => {
                let mut points = ellipse_bounds_points(center, rx, ry, rotate);
                points.push(Point::new(points[1].x + offset, points[1].y + offset));
                points
            }
            Self::Rect {
                x,
                y,
                width,
                height,
                rotate,
                ..
            } => {
                let mut points = rotated_rect_points(x, y, width, height, rotate);
                let shifted = points
                    .iter()
                    .map(|point| Point::new(point.x + offset, point.y + offset))
                    .collect::<Vec<_>>();
                points.extend(shifted);
                points
            }
        }
    }
}

fn render_shape_geometry(
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
            push_shape_shadow_path(
                out,
                object_id,
                geometry.shifted_fill_path_d(style.shadow_size, style.shadow_size),
                geometry.fill_path_d(),
                shape_shadow_fill(style.stroke.as_deref(), style.fill.as_deref()),
                geometry.shadow_bounds_points(style.shadow_size),
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

fn push_shape_fill(
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

fn push_shape_outline(
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

fn push_shape_shaded_layers(
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

fn push_shape_custom(
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

fn push_shape_shadow_path(
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

fn push_shape_ellipse_fill(
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

fn ellipse_bounds_points(center: Point, rx: f64, ry: f64, rotate: f64) -> Vec<Point> {
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

fn shape_shadow_clip_path(points: &[Point], original_shape_path: &str) -> String {
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
fn push_shape_rect_fill(
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

const SHADED_LEVELS: &[&str] = &[
    "#000000", "#0f0f0f", "#1e1e1e", "#2d2d2d", "#3b3b3b", "#494949", "#565656", "#636363",
    "#6f6f6f", "#7b7b7b", "#868686", "#919191", "#9b9b9b", "#a5a5a5", "#aeaeae", "#b7b7b7",
    "#bfbfbf", "#c7c7c7", "#cecece", "#d5d5d5", "#dbdbdb", "#e1e1e1", "#e6e6e6", "#ebebeb",
    "#efefef", "#f3f3f3", "#f6f6f6", "#f9f9f9", "#fbfbfb", "#fdfdfd", "#fefefe", "#ffffff",
];

const CIRCLE_SHADED_LEVELS: &[&str] = &[
    "#000000", "#0f0f0f", "#1e1e1e", "#2d2d2d", "#3b3b3b", "#494949", "#565656", "#636363",
    "#6f6f6f", "#7b7b7b", "#868686", "#919191", "#9b9b9b", "#a5a5a5", "#aeaeae", "#b7b7b7",
    "#bfbfbf", "#c6c6c6", "#cecece", "#d4d4d4", "#dbdbdb", "#e0e0e0", "#e6e6e6", "#eaeaea",
    "#efefef", "#f2f2f2", "#f6f6f6", "#f8f8f8", "#fbfbfb", "#fcfcfc", "#fefefe", "#fefefe",
];

const CIRCLE_SHADED_REMAIN_RATIO: f64 = 0.152_470_445_589_572_57;
const CIRCLE_SHADED_CENTER_SHIFT_RATIO: f64 = 0.484_377_144_287_654_77;
const ELLIPSE_SHADED_REMAIN_RATIO: f64 = 0.111_974_358_974_358_58;
const ELLIPSE_SHADED_CENTER_SHIFT_RATIO: f64 = 0.484_730_769_230_768_24;
const RECT_SHADED_INSET_RATIO: f64 = 0.058_648_052_902_278_19;
const ROUND_RECT_SHADED_INSET_RATIO: f64 = 0.127_129_977_460_556;
const RECT_SHADED_REMAIN_RATIO: f64 = 0.111_976_487_876_561_09;

#[allow(clippy::too_many_arguments)]
fn push_shaded_ellipse_layers(
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
fn push_shaded_rect_layers(
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

fn shaded_level_color(base_color: &str, gray: &str, t: f64) -> String {
    let Some((r, g, b)) = parse_hex_color(base_color) else {
        return gray.to_string();
    };
    if r == 0 && g == 0 && b == 0 {
        return gray.to_string();
    }
    let mix = |channel: u8| -> u8 { (channel as f64 + (255.0 - channel as f64) * t).round() as u8 };
    format!("#{:02x}{:02x}{:02x}", mix(r), mix(g), mix(b))
}

fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
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

fn rounded_rect_path_d(x: f64, y: f64, width: f64, height: f64, radius: f64) -> String {
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

fn rect_path_d(x: f64, y: f64, width: f64, height: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!(
        "M {right},{bottom} C {right},{bottom} {right},{y} {right},{y} C {right},{y} {x},{y} {x},{y} C {x},{y} {x},{bottom} {x},{bottom} C {x},{bottom} {right},{bottom} {right},{bottom}"
    )
}

fn rotated_rect_points(x: f64, y: f64, width: f64, height: f64, rotate: f64) -> Vec<Point> {
    let center = Point::new(x + width * 0.5, y + height * 0.5);
    rotated_rect_points_around(x, y, width, height, center, rotate)
}

fn rotated_rect_points_around(
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

fn rotated_rect_path_d(x: f64, y: f64, width: f64, height: f64, rotate: f64) -> String {
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

fn oval_path_d(center: Point, rx: f64, ry: f64, rotate: f64, use_cubic: bool) -> String {
    if use_cubic {
        return ellipse_cubic_path_d(center, rx, ry, rotate);
    }
    ellipse_path_d(center, rx, ry, rotate)
}

fn ellipse_cubic_path_d(center: Point, rx: f64, ry: f64, rotate: f64) -> String {
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

fn ellipse_path_d(center: Point, rx: f64, ry: f64, rotate: f64) -> String {
    let unit = crate::direction_from_angle(rotate);
    let start = center.translated(unit.scaled(-rx));
    let end = center.translated(unit.scaled(rx));
    format!(
        "M {},{} A {rx},{ry} {rotate} 1 0 {},{} A {rx},{ry} {rotate} 1 0 {},{} Z",
        start.x, start.y, end.x, end.y, start.x, start.y
    )
}

#[allow(clippy::too_many_arguments)]
fn render_orbital_ellipse_style(
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

fn orbital_lobe_geometry(
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

fn orbital_lobe_path_d(geometry: &OrbitalLobeGeometry) -> String {
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

fn orbital_lobe_outline_points(geometry: &OrbitalLobeGeometry) -> Vec<Point> {
    vec![geometry.apex, geometry.p1, geometry.tip, geometry.p2]
}

fn orbital_lobe_bounds(geometry: &OrbitalLobeGeometry) -> [f64; 4] {
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

fn scale_orbital_lobe_point(point: Point, focus: Point, scale: f64) -> Point {
    Point::new(
        focus.x + (point.x - focus.x) * scale,
        focus.y + (point.y - focus.y) * scale,
    )
}

fn scaled_orbital_lobe_geometry(
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

fn push_shaded_orbital_lobe_layers(
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

fn render_orbital_lobe_style(
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

fn render_orbital_ring(
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

fn dz2_ring_path_d(center: Point, rx: f64, ry: f64, rotate: f64, flip_y: bool) -> String {
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

fn push_shaded_dz2_ring_layers(
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

fn push_orbital_outline(
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

fn payload_point(payload: &ObjectPayload, key: &str) -> Option<Point> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

fn shape_shadow_fill(stroke: Option<&str>, fill: Option<&str>) -> String {
    let color = fill.or(stroke).unwrap_or("#000000");
    if color.eq_ignore_ascii_case("#000000") {
        return "rgba(0,0,0,0.247059)".to_string();
    }
    let Some((r, g, b)) = parse_hex_color(color) else {
        return color.to_string();
    };
    format!("rgba({r},{g},{b},0.247059)")
}
