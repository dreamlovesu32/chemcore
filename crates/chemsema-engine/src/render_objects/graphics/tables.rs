use super::*;

pub(crate) fn render_table_object(out: &mut Vec<RenderPrimitive>, object: &SceneObject) {
    let Some(table) = object.payload.table.as_ref() else {
        return;
    };
    let (Some(&left), Some(&right), Some(&top), Some(&bottom)) = (
        table.column_guides.first(),
        table.column_guides.last(),
        table.row_guides.first(),
        table.row_guides.last(),
    ) else {
        return;
    };
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    for cell in &table.cells {
        if cell.row >= table.rows || cell.column >= table.columns {
            continue;
        }
        let x1 = tx + table.column_guides[cell.column];
        let x2 = tx + table.column_guides[cell.column + 1];
        let y1 = ty + table.row_guides[cell.row];
        let y2 = ty + table.row_guides[cell.row + 1];
        out.push(RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            node_id: None,
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: 0.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        });
        for (border, from, to) in [
            (
                cell.borders.top.as_ref().unwrap_or(&table.default_border),
                Point::new(x1, y1),
                Point::new(x2, y1),
            ),
            (
                cell.borders.left.as_ref().unwrap_or(&table.default_border),
                Point::new(x1, y1),
                Point::new(x1, y2),
            ),
            (
                cell.borders
                    .bottom
                    .as_ref()
                    .unwrap_or(&table.default_border),
                Point::new(x1, y2),
                Point::new(x2, y2),
            ),
            (
                cell.borders.right.as_ref().unwrap_or(&table.default_border),
                Point::new(x2, y1),
                Point::new(x2, y2),
            ),
        ] {
            if !border.visible || border.width <= crate::EPSILON {
                continue;
            }
            let points = if border.line_style == crate::TableLineStyle::Wavy {
                table_wavy_points(from, to, border.width)
            } else {
                vec![from, to]
            };
            let d = points
                .iter()
                .enumerate()
                .map(|(index, point)| {
                    format!(
                        "{} {:.4} {:.4}",
                        if index == 0 { "M" } else { "L" },
                        point.x,
                        point.y
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            out.push(RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object.id.clone()),
                bond_id: None,
                d,
                points,
                stroke: border.color.clone(),
                stroke_width: if border.line_style == crate::TableLineStyle::Bold {
                    border.width * 2.0
                } else {
                    border.width
                },
                dash_array: match border.line_style {
                    crate::TableLineStyle::Solid => Vec::new(),
                    crate::TableLineStyle::Dashed => vec![2.5, 2.5],
                    crate::TableLineStyle::Bold | crate::TableLineStyle::Wavy => Vec::new(),
                },
                line_cap: Some("butt".to_string()),
                line_join: Some("miter".to_string()),
                rotate: object.transform.rotate,
                rotate_center: (object.transform.rotate.abs() > crate::EPSILON).then_some(
                    Point::new(tx + (left + right) * 0.5, ty + (top + bottom) * 0.5),
                ),
            });
        }
    }
}

fn table_wavy_points(from: Point, to: Point, width: f64) -> Vec<Point> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return vec![from, to];
    }
    let wavelength = 4.0_f64.max(width * 4.0);
    let wave_count = (length / wavelength).max(1.0).round() as usize;
    let steps = wave_count * 8;
    let nx = -dy / length;
    let ny = dx / length;
    let amplitude = (width * 1.5).max(0.75);
    (0..=steps)
        .map(|index| {
            let fraction = index as f64 / steps as f64;
            let offset = (fraction * wave_count as f64 * std::f64::consts::TAU).sin() * amplitude;
            Point::new(
                from.x + dx * fraction + nx * offset,
                from.y + dy * fraction + ny * offset,
            )
        })
        .collect()
}

pub(super) fn render_tlc_plate_shape_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
        .cloned()
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
            .cloned()
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

#[allow(clippy::too_many_arguments)]
pub(super) fn push_tlc_graphic_line(
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
