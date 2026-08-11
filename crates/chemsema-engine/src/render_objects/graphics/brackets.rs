use super::*;

pub(crate) fn render_bracket_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
    let side = payload_string(&object.payload, "side");
    let bounds = bracket_path_bounds(
        tx,
        ty,
        width,
        height,
        &kind,
        side.as_deref(),
        rotate_center,
        rotate,
    );
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
        d: bracket_path_d(tx, ty, width, height, &kind, side.as_deref()),
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

pub(super) fn bracket_path_d(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    kind: &str,
    side: Option<&str>,
) -> String {
    if let Some(side) = side {
        return bracket_side_path_d(x, y, width, height, kind, side);
    }
    bracket_pair_path_d(x, y, width, height, kind)
}

pub(super) fn bracket_pair_path_d(x: f64, y: f64, width: f64, height: f64, kind: &str) -> String {
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

pub(super) fn bracket_side_path_d(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    kind: &str,
    side: &str,
) -> String {
    let right = x + width;
    let bottom = y + height;
    let side = if side == "right" { "right" } else { "left" };
    match kind {
        "square" => {
            if side == "right" {
                format!(
                    "M {},{} L {},{} L {},{} L {},{}",
                    x, y, right, y, right, bottom, x, bottom
                )
            } else {
                format!(
                    "M {},{} L {},{} L {},{} L {},{}",
                    right, y, x, y, x, bottom, right, bottom
                )
            }
        }
        "curly" => {
            let depth = width.max(0.0);
            let half_depth = depth * 0.5;
            let middle = y + height * 0.5;
            let c_large = height * 0.039805;
            let c_small = height * 0.032308;
            let top_inner = y + half_depth;
            let bottom_inner = bottom - half_depth;
            if side == "right" {
                let re = x;
                let rm = x + half_depth;
                format!(
                    concat!(
                        "M {re},{bottom} ",
                        "C {re_c},{bottom} {rm},{b_cs} {rm},{bi} ",
                        "C {rm},{bi} {rm},{mbi} {rm},{mbi} ",
                        "C {rm},{mbi_c} {rm_c},{middle} {right},{middle} ",
                        "C {rm_c},{middle} {rm},{mti_c} {rm},{mti} ",
                        "C {rm},{mti} {rm},{y_cs} {re_c},{y} ",
                        "C {re},{y} {re},{y} {re},{y}"
                    ),
                    re = re,
                    re_c = re + c_large,
                    rm = rm,
                    rm_c = rm + c_small,
                    right = right,
                    y = y,
                    bottom = bottom,
                    middle = middle,
                    y_cs = y + c_small,
                    b_cs = bottom - c_small,
                    bi = bottom_inner,
                    mti = middle - half_depth,
                    mbi = middle + half_depth,
                    mti_c = middle - half_depth + c_large,
                    mbi_c = middle + half_depth - c_large,
                )
            } else {
                let le = right;
                let lm = x + half_depth;
                format!(
                    concat!(
                        "M {le},{y} ",
                        "C {le_c},{y} {lm},{y_cs} {lm},{ti} ",
                        "C {lm},{ti} {lm},{mti} {lm},{mti} ",
                        "C {lm},{mti_c} {lm_c},{middle} {x},{middle} ",
                        "C {lm_c},{middle} {lm},{mbi_c} {lm},{mbi} ",
                        "C {lm},{mbi} {lm},{b_cs} {le_c},{bottom} ",
                        "C {le},{bottom} {le},{bottom} {le},{bottom}"
                    ),
                    le = le,
                    le_c = le - c_large,
                    lm = lm,
                    lm_c = lm - c_small,
                    x = x,
                    y = y,
                    bottom = bottom,
                    middle = middle,
                    y_cs = y + c_small,
                    b_cs = bottom - c_small,
                    ti = top_inner,
                    mti = middle - half_depth,
                    mbi = middle + half_depth,
                    mti_c = middle - half_depth + c_large,
                    mbi_c = middle + half_depth - c_large,
                )
            }
        }
        _ => {
            if side == "right" {
                format!("M {},{} A {height},{height} 0 0 0 {},{}", x, bottom, x, y)
            } else {
                format!(
                    "M {},{} A {height},{height} 0 0 0 {},{}",
                    right, y, right, bottom
                )
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn bracket_path_bounds(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    kind: &str,
    side: Option<&str>,
    rotate_center: Point,
    rotate: f64,
) -> Vec<Point> {
    if side.is_some() {
        return rotated_rect_points_around(x, y, width, height, rotate_center, rotate);
    }
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

pub(super) fn square_bracket_lip(width: f64, height: f64) -> f64 {
    (height * 0.07248).min(width * 0.22).max(0.0)
}

pub(super) fn round_bracket_depth(width: f64, height: f64) -> f64 {
    (height * (1.0 - 3.0_f64.sqrt() * 0.5))
        .min(width * 0.22)
        .max(0.0)
}

pub(super) fn curly_bracket_depth(width: f64, height: f64) -> f64 {
    (height * 0.14423).min(width * 0.24).max(0.0)
}

pub(super) fn cdxml_editing_scale(document: &ChemSemaDocument) -> Option<f64> {
    document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
}

pub(super) fn bracket_symbol_path_d(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn render_symbol_object_geometry(
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
        "stereo-absolute" | "stereo-relative" | "stereo-racemic" => {
            let corner_radius =
                payload_number(&object.payload, "cornerRadius").unwrap_or(stroke_width * 4.0);
            out.push(RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                object_id: object_id.clone(),
                bond_id: None,
                d: rounded_rect_path_d(x, y, width, height, corner_radius),
                points: bounds.clone(),
                stroke: fill.clone(),
                stroke_width,
                dash_array: Vec::new(),
                line_cap: Some("round".to_string()),
                line_join: Some("round".to_string()),
                rotate,
                rotate_center,
            });
            let label =
                payload_string(&object.payload, "label").unwrap_or_else(|| "Abs".to_string());
            let font_size = payload_number(&object.payload, "fontSize").unwrap_or(12.0);
            let font_family = payload_string(&object.payload, "fontFamily")
                .unwrap_or_else(|| "Arial".to_string());
            let runs = object
                .payload
                .extra
                .get("runs")
                .cloned()
                .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
                .unwrap_or_else(|| {
                    vec![LabelRun {
                        text: label.clone(),
                        font_family: Some(font_family.clone()),
                        font_size: Some(font_size),
                        fill: Some(fill.clone()),
                        font_weight: Some(400),
                        font_style: Some("normal".to_string()),
                        underline: Some(false),
                        outline: Some(false),
                        shadow: Some(false),
                        script: Some("normal".to_string()),
                    }]
                });
            out.push(RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                object_id,
                node_id: None,
                x: x + 1.875,
                y: y + payload_number(&object.payload, "baselineOffset").unwrap_or(height * 0.75),
                baseline_offset: None,
                dominant_baseline: None,
                text: label,
                font_size,
                font_family: Some(font_family),
                fill: Some(fill),
                text_anchor: Some("start".to_string()),
                line_height: None,
                preserve_lines: false,
                box_width: None,
                runs,
                rotate,
                rotate_center,
            });
        }
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

pub(super) fn charge_symbol_layout(style: crate::CdxmlSymbolStyle) -> ChargeSymbolLayout {
    match style {
        crate::CdxmlSymbolStyle::Default => ChargeSymbolLayout {
            // ChemDraw's default circled charge uses a 5 4/9 pt internal
            // sign at editing scale 1 (108.88 units in its 20x SVG export).
            circle_sign_size: 5.444,
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

pub(super) fn push_symbol_filled_path(
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

pub(super) fn push_symbol_filled_paths(
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

pub(super) fn plus_symbol_path_ds_with_thick(
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

pub(super) fn minus_symbol_path_d_with_thick(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    thick: f64,
) -> String {
    let cy = y + height * 0.5;
    symbol_rect_path_d(x, cy - thick * 0.5, width, thick)
}

pub(super) fn dot_symbol_path_d(cx: f64, cy: f64, diameter: f64) -> String {
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

pub(super) fn symbol_rect_path_d(x: f64, y: f64, width: f64, height: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!("M {x},{y} L {right},{y} L {right},{bottom} L {x},{bottom} Z")
}
