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

pub(crate) fn render_stoichiometry_grid_object(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
) {
    let Some(grid) = object.payload.stoichiometry_grid.as_ref() else {
        return;
    };
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let label_width = grid
        .components
        .iter()
        .find(|component| component.is_header)
        .map(|component| component.width)
        .unwrap_or(86.0);
    let visible_components = grid
        .components
        .iter()
        .filter(|component| component.visible && !component.is_header)
        .collect::<Vec<_>>();
    let visible_rows = grid
        .rows
        .iter()
        .filter(|row| row.visible)
        .collect::<Vec<_>>();
    let header_height = 28.0;
    let total_width = label_width
        + visible_components
            .iter()
            .map(|component| component.width)
            .sum::<f64>();
    let total_height = header_height + visible_rows.iter().map(|row| row.height).sum::<f64>();
    if total_width <= crate::EPSILON || total_height <= crate::EPSILON {
        return;
    }
    out.push(RenderPrimitive::Rect {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        node_id: None,
        x: tx,
        y: ty,
        width: total_width,
        height: total_height,
        fill: Some("#ffffff".to_string()),
        stroke: Some(grid.style.color.clone()),
        stroke_width: grid.style.line_width,
        rx: None,
        ry: None,
        dash_array: Vec::new(),
        fill_gradient: None,
    });
    let mut x = tx + label_width;
    push_stoichiometry_line(
        out,
        object,
        Point::new(x, ty),
        Point::new(x, ty + total_height),
        grid,
    );
    for component in &visible_components {
        let center_x = x + component.width * 0.5;
        let title = component
            .reference_entity_id
            .as_deref()
            .or(component.unresolved_reference_id.as_deref())
            .unwrap_or_else(|| match component.role {
                crate::StoichiometryComponentRole::Product => "Product",
                crate::StoichiometryComponentRole::Reagent => "Reagent",
                crate::StoichiometryComponentRole::Condition => "Condition",
                _ => "Reactant",
            });
        push_stoichiometry_text(
            out,
            object,
            center_x,
            ty + header_height * 0.55,
            title,
            false,
            "middle",
            grid,
        );
        x += component.width;
        push_stoichiometry_line(
            out,
            object,
            Point::new(x, ty),
            Point::new(x, ty + total_height),
            grid,
        );
    }
    let mut y = ty + header_height;
    push_stoichiometry_line(
        out,
        object,
        Point::new(tx, y),
        Point::new(tx + total_width, y),
        grid,
    );
    for row in visible_rows {
        push_stoichiometry_text(
            out,
            object,
            tx + grid.style.margin_width,
            y + row.height * 0.55,
            &row.label,
            false,
            "start",
            grid,
        );
        let mut cell_x = tx + label_width;
        for component in &visible_components {
            if let Some(datum) = grid.data.iter().find(|datum| {
                datum.component_id == component.id
                    && datum.row_id == row.id
                    && datum.visible
                    && !datum.is_hidden
            }) {
                let text = if datum.value.display.is_empty() {
                    datum.value.canonical.as_str()
                } else {
                    datum.value.display.as_str()
                };
                let rendered = match datum.value.unit.as_deref() {
                    Some(unit) if !unit.is_empty() && !text.is_empty() => {
                        format!("{text} {unit}")
                    }
                    _ => text.to_string(),
                };
                push_stoichiometry_text(
                    out,
                    object,
                    cell_x + component.width * 0.5,
                    y + row.height * 0.55,
                    &rendered,
                    datum.is_edited || datum.origin == crate::StoichiometryValueOrigin::Authored,
                    "middle",
                    grid,
                );
            }
            cell_x += component.width;
        }
        y += row.height;
        push_stoichiometry_line(
            out,
            object,
            Point::new(tx, y),
            Point::new(tx + total_width, y),
            grid,
        );
    }
}

fn push_stoichiometry_line(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    from: Point,
    to: Point,
    grid: &crate::StoichiometryGridData,
) {
    out.push(RenderPrimitive::Line {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        bond_id: None,
        from,
        to,
        stroke: grid.style.color.clone(),
        stroke_width: grid.style.line_width,
        dash_array: Vec::new(),
    });
}

#[allow(clippy::too_many_arguments)]
fn push_stoichiometry_text(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    x: f64,
    y: f64,
    text: &str,
    bold: bool,
    anchor: &str,
    grid: &crate::StoichiometryGridData,
) {
    if text.is_empty() {
        return;
    }
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id: Some(object.id.clone()),
        node_id: None,
        x,
        y,
        baseline_offset: None,
        dominant_baseline: Some("central".to_string()),
        text: text.to_string(),
        font_size: grid.style.label_size,
        font_family: Some(grid.style.label_font.clone()),
        fill: Some(grid.style.color.clone()),
        text_anchor: Some(anchor.to_string()),
        line_height: None,
        preserve_lines: false,
        box_width: None,
        runs: vec![crate::LabelRun {
            text: text.to_string(),
            font_family: Some(grid.style.label_font.clone()),
            font_size: Some(grid.style.label_size),
            fill: Some(grid.style.color.clone()),
            font_weight: Some(if bold { 700 } else { 400 }),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            outline: Some(false),
            shadow: Some(false),
            script: Some("normal".to_string()),
        }],
        rotate: 0.0,
        rotate_center: None,
    });
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
        let center = Point::new(tx + width * 0.5, ty + height * 0.5);
        let points = [
            Point::new(tx, ty),
            Point::new(tx + width, ty),
            Point::new(tx + width, ty + height),
            Point::new(tx, ty + height),
        ]
        .into_iter()
        .map(|point| rotate_gel_point(point, center, rotate))
        .collect();
        let transparent = payload_bool(&object.payload, "transparent").unwrap_or(false);
        let alpha = payload_number(&object.payload, "alpha").unwrap_or(1.0);
        out.push(RenderPrimitive::Polygon {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            node_id: None,
            bond_id: None,
            points,
            fill: if transparent {
                "none".to_string()
            } else {
                color_with_alpha(style.fill.as_deref().unwrap_or("#ffffff"), alpha)
            },
            stroke: stroke.clone(),
            stroke_width,
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
        if !lane
            .get("visible")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
        {
            continue;
        }
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
            if !spot
                .get("visible")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
            {
                continue;
            }
            let rf = spot
                .get("rf")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.15);
            let spot_y = origin_y - (origin_y - solvent_y) * rf;
            let default_diameter = (width.min(height) * 0.03).clamp(4.0, 10.0);
            let spot_width = spot
                .get("width")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(default_diameter)
                .max(0.1);
            let spot_height = spot
                .get("height")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(default_diameter)
                .max(0.1);
            let spot_color = spot
                .get("color")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&stroke);
            let spot_alpha = spot
                .get("alpha")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.0);
            let plate_center = Point::new(tx + width * 0.5, ty + height * 0.5);
            out.push(RenderPrimitive::Ellipse {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object.id.clone()),
                center: rotate_gel_point(Point::new(lane_x, spot_y), plate_center, rotate),
                rx: spot_width * 0.5,
                ry: spot_height * 0.5,
                rotate,
                fill: Some(color_with_alpha(spot_color, spot_alpha)),
                stroke: None,
                stroke_width: 0.0,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
            if spot
                .get("showRf")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                let label_point = rotate_gel_point(
                    Point::new(lane_x + spot_width * 0.5 + 2.0, spot_y),
                    plate_center,
                    rotate,
                );
                out.push(RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    object_id: Some(object.id.clone()),
                    node_id: None,
                    x: label_point.x,
                    y: label_point.y,
                    baseline_offset: None,
                    dominant_baseline: Some("central".to_string()),
                    text: crate::round2(rf).to_string(),
                    font_size: payload_number(&object.payload, "labelSize").unwrap_or(10.0),
                    font_family: Some("Arial".to_string()),
                    fill: Some(stroke.clone()),
                    text_anchor: Some("start".to_string()),
                    line_height: None,
                    preserve_lines: false,
                    box_width: None,
                    runs: Vec::new(),
                    rotate,
                    rotate_center: Some(label_point),
                });
            }
        }
    }
}

pub(super) fn render_gel_electrophoresis_object(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    style: ShapeStyleSpec,
) {
    let (Some([x, y, width, height]), Some(gel)) = (
        object.payload.bbox,
        object.payload.gel_electrophoresis.as_ref(),
    ) else {
        return;
    };
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return;
    }
    let tx = object.transform.translate[0] + x;
    let ty = object.transform.translate[1] + y;
    let center = Point::new(tx + width * 0.5, ty + height * 0.5);
    let rotate = object.transform.rotate;
    let stroke = if gel.color.is_empty() {
        style.base_color().to_string()
    } else {
        gel.color.clone()
    };
    let stroke_width = gel.line_width.max(0.05);
    let corners = gel
        .corners
        .unwrap_or([[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]]);
    let plate_points = corners
        .into_iter()
        .map(|point| rotate_gel_point(Point::new(tx + point[0], ty + point[1]), center, rotate))
        .collect::<Vec<_>>();
    if gel.show_borders {
        out.push(RenderPrimitive::Polygon {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            node_id: None,
            bond_id: None,
            points: plate_points,
            fill: if gel.transparent {
                "none".to_string()
            } else {
                color_with_alpha(style.fill.as_deref().unwrap_or("#ffffff"), gel.alpha)
            },
            stroke: stroke.clone(),
            stroke_width,
        });
    }
    let lanes = &gel.lanes;
    let range = gel.end_range - gel.start_range;
    for (lane_index, lane) in lanes.iter().enumerate() {
        if !lane.visible {
            continue;
        }
        let lane_x = tx + width * (lane_index as f64 + 1.0) / (lanes.len() as f64 + 1.0);
        if !lane.label_text.is_empty() {
            let label_point = rotate_gel_point(
                Point::new(lane_x, ty - gel.margin_width.max(2.0)),
                center,
                rotate,
            );
            out.push(RenderPrimitive::Text {
                role: RenderRole::DocumentText,
                object_id: Some(object.id.clone()),
                node_id: None,
                x: label_point.x,
                y: label_point.y,
                baseline_offset: None,
                dominant_baseline: Some("auto".to_string()),
                text: lane.label_text.clone(),
                font_size: gel.label_size,
                font_family: Some("Arial".to_string()),
                fill: Some(stroke.clone()),
                text_anchor: Some("middle".to_string()),
                line_height: None,
                preserve_lines: false,
                box_width: None,
                runs: Vec::new(),
                rotate: rotate + gel.labels_angle,
                rotate_center: Some(label_point),
            });
        }
        for band in lane.bands.iter().filter(|band| band.visible) {
            let fraction = ((band.value - gel.start_range) / range).clamp(0.0, 1.0);
            let band_y = match gel.unit_id {
                0..=2 => ty + height * (1.0 - fraction),
                3 => ty + height * fraction,
                _ => continue,
            };
            let half_width = band.width.min(width / (lanes.len() as f64 + 1.0)) * 0.5;
            let half_height = band.height.min(height) * 0.5;
            let radius = match band.curve_type {
                0 => 0.0,
                128 => half_height.min(half_width),
                _ => continue,
            };
            let left = lane_x - half_width;
            let right = lane_x + half_width;
            let top = band_y - half_height;
            let bottom = band_y + half_height;
            let d = format!(
                "M {:.4} {:.4} Q {:.4} {:.4} {:.4} {:.4} L {:.4} {:.4} Q {:.4} {:.4} {:.4} {:.4} L {:.4} {:.4} Q {:.4} {:.4} {:.4} {:.4} L {:.4} {:.4} Q {:.4} {:.4} {:.4} {:.4} Z",
                left + radius, top,
                left, top, left, top + radius,
                left, bottom - radius,
                left, bottom, left + radius, bottom,
                right - radius, bottom,
                right, bottom, right, bottom - radius,
                right, top + radius,
                right, top, right - radius, top,
            );
            out.push(RenderPrimitive::FilledPath {
                role: RenderRole::DocumentGraphic,
                object_id: Some(object.id.clone()),
                node_id: None,
                bond_id: None,
                d,
                points: vec![
                    Point::new(left, top),
                    Point::new(right, top),
                    Point::new(right, bottom),
                    Point::new(left, bottom),
                ],
                fill: color_with_alpha(&band.color, band.alpha),
                fill_rule: None,
                clip_path_d: None,
                clip_rule: None,
                rotate,
                rotate_center: Some(center),
            });
            if band.show_value {
                let value_point =
                    rotate_gel_point(Point::new(right + gel.margin_width, band_y), center, rotate);
                out.push(RenderPrimitive::Text {
                    role: RenderRole::DocumentText,
                    object_id: Some(object.id.clone()),
                    node_id: None,
                    x: value_point.x,
                    y: value_point.y,
                    baseline_offset: None,
                    dominant_baseline: Some("central".to_string()),
                    text: crate::round2(band.value).to_string(),
                    font_size: gel.label_size,
                    font_family: Some("Arial".to_string()),
                    fill: Some(stroke.clone()),
                    text_anchor: Some("start".to_string()),
                    line_height: None,
                    preserve_lines: false,
                    box_width: None,
                    runs: Vec::new(),
                    rotate,
                    rotate_center: Some(value_point),
                });
            }
        }
    }
    if gel.show_scale {
        let axis_x = tx + width + gel.margin_width.max(4.0);
        push_tlc_graphic_line(
            out,
            object,
            Point::new(axis_x, ty),
            Point::new(axis_x, ty + height),
            &stroke,
            gel.axis_width.max(0.05),
            Vec::new(),
            rotate,
            Some(center),
        );
        for index in 0..=4 {
            let fraction = index as f64 / 4.0;
            let axis_y = ty + height * fraction;
            push_tlc_graphic_line(
                out,
                object,
                Point::new(axis_x, axis_y),
                Point::new(axis_x + gel.margin_width.max(3.0), axis_y),
                &stroke,
                gel.axis_width.max(0.05),
                Vec::new(),
                rotate,
                Some(center),
            );
        }
    }
}

fn color_with_alpha(color: &str, alpha: f64) -> String {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha >= 0.999 {
        return color.to_string();
    }
    let hex = color.strip_prefix('#').unwrap_or(color);
    if hex.len() == 6 {
        let parsed = u32::from_str_radix(hex, 16).ok();
        if let Some(value) = parsed {
            return format!(
                "rgba({},{},{},{:.4})",
                (value >> 16) & 0xff,
                (value >> 8) & 0xff,
                value & 0xff,
                alpha
            );
        }
    }
    color.to_string()
}

fn rotate_gel_point(point: Point, center: Point, degrees: f64) -> Point {
    if degrees.abs() <= crate::EPSILON {
        return point;
    }
    let radians = degrees.to_radians();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * radians.cos() - dy * radians.sin(),
        center.y + dx * radians.sin() + dy * radians.cos(),
    )
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
