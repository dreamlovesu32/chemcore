use super::*;

pub(super) unsafe fn draw_gdiplus_primitive(
    graphics: *mut GpGraphics,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    bond_context: Option<&PreviewBondContext>,
    label_context: Option<&PreviewLabelContext>,
) -> bool {
    let save_restore = transform.emf_recording
        && matches!(
            primitive,
            RenderPrimitive::Line { .. } | RenderPrimitive::Polyline { .. }
        );
    let mut state = 0u32;
    if save_restore && GdipSaveGraphics(graphics, &mut state) != GDI_PLUS_OK {
        return false;
    }
    let ok = match primitive {
        RenderPrimitive::Line {
            from,
            to,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => draw_gdiplus_polyline(
            graphics,
            &[*from, *to],
            stroke,
            *stroke_width,
            Some("butt"),
            Some("miter"),
            transform,
            dash_array,
        ),
        RenderPrimitive::Polyline {
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => draw_gdiplus_polyline(
            graphics,
            points,
            stroke,
            *stroke_width,
            line_cap.as_deref(),
            line_join.as_deref(),
            transform,
            dash_array,
        ),
        RenderPrimitive::Polygon {
            role,
            bond_id,
            points,
            fill,
            stroke,
            stroke_width,
            ..
        } => draw_gdiplus_polygon(
            graphics,
            *role,
            bond_id.as_deref(),
            points,
            fill,
            stroke,
            *stroke_width,
            transform,
            bond_context,
        ),
        RenderPrimitive::FilledPath {
            d,
            points,
            fill,
            clip_path_d,
            role,
            bond_id,
            ..
        } => {
            if clip_path_d.is_some() {
                return false;
            }
            if *role == RenderRole::DocumentBond {
                let hashed_wedge =
                    preview_hashed_wedge_stroke_line(points, bond_id.as_deref(), bond_context);
                let stroke_line = if transform.office_presentation {
                    preview_office_hashed_wedge_stroke_line(
                        points,
                        bond_id.as_deref(),
                        bond_context,
                    )
                } else {
                    hashed_wedge
                };
                if let Some(stroke_line) = stroke_line {
                    return draw_gdiplus_polyline(
                        graphics,
                        &[stroke_line.start, stroke_line.end],
                        fill,
                        stroke_line.width,
                        Some("round"),
                        None,
                        transform,
                        &[],
                    );
                }
                if transform.office_presentation && hashed_wedge.is_some() {
                    return fill_gdiplus_polygon(graphics, points, fill, transform);
                }
            }
            draw_gdiplus_path(
                graphics,
                d,
                Some(fill),
                None,
                0.0,
                None,
                None,
                transform,
                &[],
            )
        }
        RenderPrimitive::Path {
            d,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => draw_gdiplus_path(
            graphics,
            d,
            None,
            Some(stroke),
            *stroke_width,
            line_cap.as_deref(),
            line_join.as_deref(),
            transform,
            dash_array,
        ),
        RenderPrimitive::Rect {
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => draw_gdiplus_rect(
            graphics,
            *x,
            *y,
            *width,
            *height,
            fill.as_deref(),
            stroke.as_deref(),
            *stroke_width,
            dash_array,
            transform,
        ),
        RenderPrimitive::Ellipse {
            center,
            rx,
            ry,
            fill,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => draw_gdiplus_ellipse(
            graphics,
            center.x - rx,
            center.y - ry,
            rx * 2.0,
            ry * 2.0,
            fill.as_deref(),
            stroke.as_deref(),
            *stroke_width,
            dash_array,
            transform,
        ),
        RenderPrimitive::Circle {
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => draw_gdiplus_ellipse(
            graphics,
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
            Some(fill),
            Some(stroke),
            *stroke_width,
            &[],
            transform,
        ),
        RenderPrimitive::Image { .. } => false,
        RenderPrimitive::Text {
            x,
            y,
            baseline_offset,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            line_height,
            runs,
            node_id,
            ..
        } => draw_gdiplus_text(
            graphics,
            *x,
            *y,
            *baseline_offset,
            text,
            *font_size,
            font_family.as_deref(),
            fill.as_deref(),
            text_anchor.as_deref(),
            *line_height,
            runs,
            transform,
            node_id.as_deref(),
            label_context,
        ),
    };
    if save_restore {
        let _ = GdipRestoreGraphics(graphics, state);
    }
    ok
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_polyline(
    graphics: *mut GpGraphics,
    points: &[CorePoint],
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) -> bool {
    if points.len() < 2 {
        return true;
    }
    if transform.emf_recording && !dash_array.is_empty() {
        return draw_gdiplus_dashed_polyline(
            graphics,
            points,
            color,
            stroke_width,
            line_cap,
            line_join,
            transform,
            dash_array,
        );
    }
    let Some(pen) = create_gdiplus_pen(
        color,
        transform.gdip_length(stroke_width),
        line_cap,
        line_join,
        dash_array,
        transform,
    ) else {
        return false;
    };
    let mapped: Vec<PointF> = points
        .iter()
        .map(|point| transform.gdip_point(*point))
        .collect();
    let ok = GdipDrawLines(graphics, pen, mapped.as_ptr(), mapped.len() as i32) == GDI_PLUS_OK;
    GdipDeletePen(pen);
    ok
}

pub(super) fn normalized_dash_pattern(dash_array: &[f64]) -> Vec<f64> {
    let mut pattern: Vec<f64> = dash_array
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect();
    if pattern.len() % 2 == 1 {
        pattern.extend_from_within(..);
    }
    pattern
}

pub(super) fn dashed_polyline_segments(
    points: &[CorePoint],
    dash_array: &[f64],
) -> Vec<[CorePoint; 2]> {
    let pattern = normalized_dash_pattern(dash_array);
    if pattern.is_empty() || points.len() < 2 {
        return Vec::new();
    }
    let mut segments = Vec::new();
    let mut pattern_index = 0usize;
    let mut remaining = pattern[0];
    let mut draw_segment = true;

    for pair in points.windows(2) {
        let from = pair[0];
        let to = pair[1];
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f64::EPSILON {
            continue;
        }
        let unit_x = dx / length;
        let unit_y = dy / length;
        let mut offset = 0.0;
        while offset < length - f64::EPSILON {
            let step = remaining.min(length - offset);
            if draw_segment && step > f64::EPSILON {
                let start = CorePoint {
                    x: from.x + unit_x * offset,
                    y: from.y + unit_y * offset,
                };
                let end = CorePoint {
                    x: from.x + unit_x * (offset + step),
                    y: from.y + unit_y * (offset + step),
                };
                segments.push([start, end]);
            }
            offset += step;
            remaining -= step;
            if remaining <= f64::EPSILON {
                pattern_index = (pattern_index + 1) % pattern.len();
                remaining = pattern[pattern_index];
                draw_segment = pattern_index.is_multiple_of(2);
            }
        }
    }

    segments
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_dashed_polyline(
    graphics: *mut GpGraphics,
    points: &[CorePoint],
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) -> bool {
    let segments = dashed_polyline_segments(points, dash_array);
    if segments.is_empty() {
        return true;
    }
    let Some(pen) = create_gdiplus_pen(
        color,
        transform.gdip_length(stroke_width),
        line_cap,
        line_join,
        &[],
        transform,
    ) else {
        return false;
    };
    let mut ok = true;
    for [from, to] in segments {
        let p1 = transform.gdip_point(from);
        let p2 = transform.gdip_point(to);
        ok &= GdipDrawLine(graphics, pen, p1.X, p1.Y, p2.X, p2.Y) == GDI_PLUS_OK;
    }
    GdipDeletePen(pen);
    ok
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_polygon(
    graphics: *mut GpGraphics,
    role: RenderRole,
    bond_id: Option<&str>,
    points: &[CorePoint],
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    transform: &PreviewTransform,
    bond_context: Option<&PreviewBondContext>,
) -> bool {
    if points.len() < 3 {
        return true;
    }
    if role == RenderRole::DocumentBond {
        let hashed_wedge = preview_hashed_wedge_stroke_line(points, bond_id, bond_context);
        let stroke_line = if hashed_wedge.is_some() {
            if transform.office_presentation {
                preview_office_hashed_wedge_stroke_line(points, bond_id, bond_context)
            } else {
                hashed_wedge
            }
        } else {
            preview_bond_stroke_line(points, bond_id, bond_context)
        };
        if let Some(stroke_line) = stroke_line {
            let line_points = [stroke_line.start, stroke_line.end];
            return draw_gdiplus_polyline(
                graphics,
                &line_points,
                fill,
                stroke_line.width,
                Some("round"),
                None,
                transform,
                &[],
            );
        }
        if transform.office_presentation && hashed_wedge.is_some() {
            return fill_gdiplus_polygon(graphics, points, fill, transform);
        }
    }
    let mapped: Vec<PointF> = points
        .iter()
        .map(|point| transform.gdip_point(*point))
        .collect();
    let mut ok = true;
    if let Some(brush) = create_gdiplus_solid_brush(fill) {
        ok &= GdipFillPolygon(
            graphics,
            brush,
            mapped.as_ptr(),
            mapped.len() as i32,
            FillModeAlternate,
        ) == GDI_PLUS_OK;
        GdipDeleteBrush(brush);
    }
    if stroke_width > 0.0 {
        if let Some(pen) = create_gdiplus_pen(
            stroke,
            transform.gdip_length(stroke_width),
            Some("butt"),
            Some("miter"),
            &[],
            transform,
        ) {
            ok &=
                GdipDrawPolygon(graphics, pen, mapped.as_ptr(), mapped.len() as i32) == GDI_PLUS_OK;
            GdipDeletePen(pen);
        }
    }
    ok
}

pub(super) unsafe fn fill_gdiplus_polygon(
    graphics: *mut GpGraphics,
    points: &[CorePoint],
    fill: &str,
    transform: &PreviewTransform,
) -> bool {
    if points.len() < 3 {
        return true;
    }
    let mapped: Vec<PointF> = points
        .iter()
        .map(|point| transform.gdip_point(*point))
        .collect();
    let Some(brush) = create_gdiplus_solid_brush(fill) else {
        return false;
    };
    let ok = GdipFillPolygon(
        graphics,
        brush,
        mapped.as_ptr(),
        mapped.len() as i32,
        FillModeAlternate,
    ) == GDI_PLUS_OK;
    GdipDeleteBrush(brush);
    ok
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_rect(
    graphics: *mut GpGraphics,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: Option<&str>,
    stroke: Option<&str>,
    stroke_width: f64,
    dash_array: &[f64],
    transform: &PreviewTransform,
) -> bool {
    let p1 = transform.gdip_point(CorePoint { x, y });
    let p2 = transform.gdip_point(CorePoint {
        x: x + width,
        y: y + height,
    });
    let left = p1.X.min(p2.X);
    let top = p1.Y.min(p2.Y);
    let w = (p1.X - p2.X).abs();
    let h = (p1.Y - p2.Y).abs();
    let mut ok = true;
    if let Some(fill) = fill {
        if let Some(brush) = create_gdiplus_solid_brush(fill) {
            ok &= GdipFillRectangle(graphics, brush, left, top, w, h) == GDI_PLUS_OK;
            GdipDeleteBrush(brush);
        }
    }
    if let Some(stroke) = stroke {
        if let Some(pen) = create_gdiplus_pen(
            stroke,
            transform.gdip_length(stroke_width),
            Some("butt"),
            Some("miter"),
            dash_array,
            transform,
        ) {
            ok &= GdipDrawRectangle(graphics, pen, left, top, w, h) == GDI_PLUS_OK;
            GdipDeletePen(pen);
        }
    }
    ok
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_ellipse(
    graphics: *mut GpGraphics,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: Option<&str>,
    stroke: Option<&str>,
    stroke_width: f64,
    dash_array: &[f64],
    transform: &PreviewTransform,
) -> bool {
    let p1 = transform.gdip_point(CorePoint { x, y });
    let p2 = transform.gdip_point(CorePoint {
        x: x + width,
        y: y + height,
    });
    let left = p1.X.min(p2.X);
    let top = p1.Y.min(p2.Y);
    let w = (p1.X - p2.X).abs();
    let h = (p1.Y - p2.Y).abs();
    let mut ok = true;
    if let Some(fill) = fill {
        if let Some(brush) = create_gdiplus_solid_brush(fill) {
            ok &= GdipFillEllipse(graphics, brush, left, top, w, h) == GDI_PLUS_OK;
            GdipDeleteBrush(brush);
        }
    }
    if let Some(stroke) = stroke {
        if let Some(pen) = create_gdiplus_pen(
            stroke,
            transform.gdip_length(stroke_width),
            Some("round"),
            Some("round"),
            dash_array,
            transform,
        ) {
            ok &= GdipDrawEllipse(graphics, pen, left, top, w, h) == GDI_PLUS_OK;
            GdipDeletePen(pen);
        }
    }
    ok
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_path(
    graphics: *mut GpGraphics,
    d: &str,
    fill: Option<&str>,
    stroke: Option<&str>,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) -> bool {
    let Some(commands) = parse_preview_path(d) else {
        return false;
    };
    let Some(path) = create_gdiplus_path(&commands, transform) else {
        return false;
    };
    let mut ok = true;
    if let Some(fill) = fill {
        if let Some(brush) = create_gdiplus_solid_brush(fill) {
            ok &= GdipFillPath(graphics, brush, path) == GDI_PLUS_OK;
            GdipDeleteBrush(brush);
        }
    }
    if let Some(stroke) = stroke {
        if let Some(pen) = create_gdiplus_pen(
            stroke,
            transform.gdip_length(stroke_width),
            line_cap,
            line_join,
            dash_array,
            transform,
        ) {
            ok &= GdipDrawPath(graphics, pen, path) == GDI_PLUS_OK;
            GdipDeletePen(pen);
        }
    }
    GdipDeletePath(path);
    ok
}

pub(super) unsafe fn create_gdiplus_path(
    commands: &[PreviewPathCommand],
    transform: &PreviewTransform,
) -> Option<*mut GpPath> {
    let mut path = null_mut();
    if GdipCreatePath(FillModeAlternate, &mut path) != GDI_PLUS_OK || path.is_null() {
        return None;
    }
    let mut current = None;
    let mut ok = true;
    for command in commands {
        match *command {
            PreviewPathCommand::Move(point) => {
                if current.is_some() {
                    ok &= GdipStartPathFigure(path) == GDI_PLUS_OK;
                }
                current = Some(point);
            }
            PreviewPathCommand::Line(point) => {
                if let Some(from) = current {
                    let p1 = transform.gdip_point(from);
                    let p2 = transform.gdip_point(point);
                    ok &= GdipAddPathLine(path, p1.X, p1.Y, p2.X, p2.Y) == GDI_PLUS_OK;
                }
                current = Some(point);
            }
            PreviewPathCommand::Cubic(c1, c2, to) => {
                if let Some(from) = current {
                    let p1 = transform.gdip_point(from);
                    let p2 = transform.gdip_point(c1);
                    let p3 = transform.gdip_point(c2);
                    let p4 = transform.gdip_point(to);
                    ok &= GdipAddPathBezier(path, p1.X, p1.Y, p2.X, p2.Y, p3.X, p3.Y, p4.X, p4.Y)
                        == GDI_PLUS_OK;
                }
                current = Some(to);
            }
            PreviewPathCommand::Close => {
                ok &= GdipClosePathFigure(path) == GDI_PLUS_OK;
                current = None;
            }
        }
    }
    if ok {
        Some(path)
    } else {
        GdipDeletePath(path);
        None
    }
}

pub(super) unsafe fn create_gdiplus_pen(
    color: &str,
    width: f32,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    dash_array: &[f64],
    transform: &PreviewTransform,
) -> Option<*mut GpPen> {
    let mut pen = null_mut();
    let unit = if transform.emf_recording {
        UnitWorld
    } else {
        UnitPixel
    };
    if GdipCreatePen1(css_argb(color)?, width.max(0.01), unit, &mut pen) != GDI_PLUS_OK
        || pen.is_null()
    {
        return None;
    }
    let cap = gdiplus_line_cap(line_cap);
    GdipSetPenStartCap(pen, cap);
    GdipSetPenEndCap(pen, cap);
    if line_join.is_some() {
        GdipSetPenLineJoin(pen, gdiplus_line_join(line_join));
    }
    GdipSetPenMiterLimit(pen, PREVIEW_MITER_LIMIT);
    if line_cap == Some("round") {
        GdipSetPenDashCap197819(pen, DashCapRound);
    }
    if !dash_array.is_empty() {
        let mut dash: Vec<f32> = dash_array
            .iter()
            .copied()
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| (transform.gdip_length(value) / width.max(0.01)).max(0.1))
            .collect();
        if dash.len() == 1 {
            GdipSetPenDashStyle(pen, DashStyleDash);
        } else if !dash.is_empty() {
            if dash.len() % 2 == 1 {
                dash.extend_from_within(..);
            }
            GdipSetPenDashArray(pen, dash.as_ptr(), dash.len() as i32);
        }
    }
    Some(pen)
}

pub(super) unsafe fn create_gdiplus_solid_brush(color: &str) -> Option<*mut GpBrush> {
    let mut brush = null_mut();
    if GdipCreateSolidFill(css_argb(color)?, &mut brush) == GDI_PLUS_OK && !brush.is_null() {
        Some(brush as *mut GpBrush)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_text(
    graphics: *mut GpGraphics,
    x: f64,
    y: f64,
    baseline_offset: Option<f64>,
    text: &str,
    font_size: f64,
    font_family: Option<&str>,
    fill: Option<&str>,
    text_anchor: Option<&str>,
    line_height: Option<f64>,
    runs: &[chemsema_engine::LabelRun],
    transform: &PreviewTransform,
    node_id: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
) -> bool {
    let x_nudge_px = preview_attached_label_replay_nudge_px(
        node_id,
        runs,
        fill,
        text_anchor,
        label_context,
        x,
        y,
        baseline_offset,
        font_size,
        transform,
    );
    let effective_font_scale =
        preview_attached_label_replay_font_scale(node_id, runs, fill, text_anchor, label_context);
    let default_text_hint = preview_default_gdiplus_text_rendering_hint(transform);
    let effective_text_hint =
        preview_attached_label_replay_text_hint(node_id, runs, fill, text_anchor, label_context)
            .unwrap_or(default_text_hint);
    if effective_text_hint != default_text_hint {
        GdipSetTextRenderingHint(graphics, effective_text_hint);
    }
    let effective_font_size = font_size * effective_font_scale;
    let x = x + x_nudge_px / (transform.scale * transform.record_scale.max(1.0));
    let y_nudge_px = preview_attached_label_replay_y_nudge_px(
        node_id,
        runs,
        fill,
        text_anchor,
        label_context,
        x,
        y,
        baseline_offset,
        effective_font_size,
        transform,
    );
    let top_nudge_px = preview_attached_label_replay_top_nudge_px(
        node_id,
        runs,
        fill,
        text_anchor,
        label_context,
        x,
        y,
        baseline_offset,
        effective_font_size,
        transform,
    );
    let line_step_world = line_height.unwrap_or(effective_font_size * 1.2).max(0.01);
    let mut lines = preview_text_lines(text, runs);
    preview_scale_text_run_font_sizes(&mut lines, effective_font_scale);
    let layouts = gdiplus_text_layout(
        graphics,
        &lines,
        effective_font_size,
        font_family,
        transform,
        transform.emf_recording && matches!(text_anchor, Some("middle")),
    );
    let mut ok = true;
    for (index, line_runs) in lines.iter().enumerate() {
        if line_runs.is_empty() {
            continue;
        }
        let origin = transform.gdip_point(CorePoint {
            x,
            y: y + y_nudge_px / (transform.scale * transform.record_scale.max(1.0))
                + index as f64 * line_step_world,
        });
        let Some(line_layout) = layouts.get(index) else {
            continue;
        };
        let trailing_trim = if transform.emf_recording
            && !preview_env_enabled(ENV_DISABLE_PACKAGED_TRAILING_TRIM)
            && matches!(text_anchor, Some("middle" | "end"))
        {
            gdiplus_line_trailing_space_trim(
                graphics,
                line_runs,
                effective_font_size,
                font_family,
                transform,
            )
            .unwrap_or(0.0)
        } else {
            0.0
        };
        let width = (line_layout.width - trailing_trim).max(0.0);
        let mut cursor_x = match text_anchor {
            Some("middle") => origin.X - width / 2.0,
            Some("end") => origin.X - width,
            _ => origin.X,
        };
        let line_top = preview_gdiplus_text_top(
            origin.Y,
            baseline_offset,
            text_anchor,
            top_nudge_px,
            line_runs.first(),
            effective_font_size,
            transform,
        );
        let line_rect_override = preview_packaged_node_label_line_rect(
            node_id,
            text_anchor,
            label_context,
            transform,
            index,
            lines.len(),
            cursor_x,
            line_top,
        );
        let line_scale_x = line_rect_override
            .map(|rect| rect.Width / width.max(0.01))
            .unwrap_or(1.0);
        let mut logical_cursor = 0.0f32;
        for (run, run_layout) in line_runs.iter().zip(&line_layout.runs) {
            let run_rect_override = line_rect_override.map(|line_rect| RectF {
                X: line_rect.X + (logical_cursor + run_layout.dx) * line_scale_x,
                Y: line_rect.Y,
                Width: (run_layout.advance * line_scale_x).max(0.0),
                Height: line_rect.Height,
            });
            ok &= draw_gdiplus_text_run(
                graphics,
                cursor_x + run_layout.dx,
                origin.Y,
                baseline_offset,
                text_anchor,
                node_id,
                label_context,
                run_layout.advance,
                top_nudge_px,
                run_rect_override,
                run,
                effective_font_size,
                font_family,
                fill,
                transform,
            );
            logical_cursor += run_layout.dx + run_layout.advance;
            cursor_x += run_layout.dx + run_layout.advance;
        }
    }
    if effective_text_hint != default_text_hint {
        GdipSetTextRenderingHint(graphics, default_text_hint);
    }
    ok
}

pub(super) unsafe fn gdiplus_line_trailing_space_trim(
    graphics: *mut GpGraphics,
    line_runs: &[PreviewTextRun],
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<f32> {
    let mut remaining = 0usize;
    for run in line_runs.iter().rev() {
        for ch in run.text.chars().rev() {
            if ch == ' ' || ch == '\t' {
                remaining += 1;
            } else {
                break;
            }
        }
        if remaining > 0 && !run.text.chars().all(|ch| ch == ' ' || ch == '\t') {
            break;
        }
    }
    if remaining == 0 {
        return Some(0.0);
    }
    let mut trim = 0.0f32;
    for run in line_runs.iter().rev() {
        if remaining == 0 {
            break;
        }
        let trailing = run
            .text
            .chars()
            .rev()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .count();
        if trailing == 0 {
            break;
        }
        let take = trailing.min(remaining);
        let original =
            gdiplus_text_run_advance(graphics, run, default_font_size, default_family, transform)?;
        let keep_chars = run.text.chars().count().saturating_sub(take);
        let trimmed_text: String = run.text.chars().take(keep_chars).collect();
        let mut trimmed_run = run.clone();
        trimmed_run.text = trimmed_text;
        let trimmed = gdiplus_text_run_advance(
            graphics,
            &trimmed_run,
            default_font_size,
            default_family,
            transform,
        )
        .unwrap_or(0.0);
        trim += (original - trimmed).max(0.0);
        remaining -= take;
        if !run.text.chars().all(|ch| ch == ' ' || ch == '\t') {
            break;
        }
    }
    Some(trim)
}

pub(super) unsafe fn gdiplus_text_layout(
    graphics: *mut GpGraphics,
    lines: &[Vec<PreviewTextRun>],
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
    packaged_centered: bool,
) -> Vec<GdiplusTextLineLayout> {
    let dc = CreateCompatibleDC(null_mut());
    if dc.is_null() {
        return lines
            .iter()
            .map(|runs| GdiplusTextLineLayout {
                width: preview_line_width_f32(runs, default_font_size, transform),
                runs: runs
                    .iter()
                    .map(|run| GdiplusTextRunLayout {
                        dx: preview_script_dx_f32(run, default_font_size, transform),
                        advance: preview_text_run_advance_estimate_f32(
                            run,
                            default_font_size,
                            transform,
                        ),
                    })
                    .collect(),
            })
            .collect();
    }
    let mut cache = PreviewGdiCache::default();
    let layouts = lines
        .iter()
        .map(|runs| {
            let use_plain_gdi_widths = packaged_centered
                && preview_env_enabled(ENV_PACKAGED_CENTERED_PLAIN_GDI_WIDTH)
                && runs.iter().all(|run| run.script.is_none());
            let mut width = 0.0f32;
            let run_layouts = runs
                .iter()
                .map(|run| {
                    let dx = preview_script_dx_f32(run, default_font_size, transform);
                    let advance = if use_plain_gdi_widths {
                        preview_text_run_extent(
                            dc,
                            run,
                            default_font_size,
                            default_family,
                            transform,
                            &mut cache,
                        ) as f32
                    } else {
                        gdiplus_text_run_advance(
                            graphics,
                            run,
                            default_font_size,
                            default_family,
                            transform,
                        )
                        .unwrap_or_else(|| {
                            preview_text_run_extent(
                                dc,
                                run,
                                default_font_size,
                                default_family,
                                transform,
                                &mut cache,
                            ) as f32
                        })
                    };
                    width += dx + advance;
                    GdiplusTextRunLayout { dx, advance }
                })
                .collect();
            GdiplusTextLineLayout {
                width,
                runs: run_layouts,
            }
        })
        .collect();
    cache.delete_objects();
    DeleteDC(dc);
    layouts
}

pub(super) unsafe fn gdiplus_text_run_advance(
    graphics: *mut GpGraphics,
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<f32> {
    if run.text.is_empty() {
        return Some(0.0);
    }
    let font = create_gdiplus_font(run, default_font_size, default_family, transform)?;
    let Some(format) = create_gdiplus_string_format() else {
        GdipDeleteFont(font);
        return None;
    };
    let wide: Vec<u16> = run.text.encode_utf16().collect();
    let width = gdiplus_measure_text_width(
        graphics,
        font,
        format,
        &wide,
        run,
        default_font_size,
        transform,
    );
    GdipDeleteStringFormat(format);
    GdipDeleteFont(font);
    width
}

pub(super) unsafe fn gdiplus_measure_text_width(
    graphics: *mut GpGraphics,
    font: *mut GpFont,
    format: *mut GpStringFormat,
    wide: &[u16],
    run: &PreviewTextRun,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> Option<f32> {
    if wide.is_empty() {
        return Some(0.0);
    }
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_px =
        (run.font_size.unwrap_or(default_font_size) * script_scale * gdiplus_text_scale(transform))
            .max(1.0) as f32;
    let layout = RectF {
        X: 0.0,
        Y: 0.0,
        Width: font_px * wide.len().max(1) as f32 * 4.0,
        Height: font_px * 2.0,
    };
    let mut bounds = RectF {
        X: 0.0,
        Y: 0.0,
        Width: 0.0,
        Height: 0.0,
    };
    let ok = GdipMeasureString(
        graphics,
        wide.as_ptr(),
        wide.len() as i32,
        font,
        &layout,
        format,
        &mut bounds,
        null_mut(),
        null_mut(),
    ) == GDI_PLUS_OK;
    ok.then_some(bounds.Width.max(0.0))
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_gdiplus_text_run(
    graphics: *mut GpGraphics,
    x: f32,
    baseline_y: f32,
    baseline_offset: Option<f64>,
    text_anchor: Option<&str>,
    node_id: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
    advance: f32,
    top_nudge_px: f64,
    layout_rect_override: Option<RectF>,
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    default_fill: Option<&str>,
    transform: &PreviewTransform,
) -> bool {
    if run.text.is_empty() {
        return true;
    }
    let Some(font) = create_gdiplus_font(run, default_font_size, default_family, transform) else {
        return false;
    };
    let fill = run.fill.as_deref().or(default_fill).unwrap_or("#000000");
    let Some(brush) = create_gdiplus_solid_brush(fill) else {
        GdipDeleteFont(font);
        return false;
    };
    let Some(format) = create_gdiplus_string_format() else {
        GdipDeleteBrush(brush);
        GdipDeleteFont(font);
        return false;
    };
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_px =
        (run.font_size.unwrap_or(default_font_size) * script_scale * gdiplus_text_scale(transform))
            .max(1.0) as f32;
    let top = preview_gdiplus_text_top(
        baseline_y,
        baseline_offset,
        text_anchor,
        top_nudge_px,
        Some(run),
        default_font_size,
        transform,
    );
    let attached_start_layout = preview_packaged_attached_start_layout_mode(
        node_id,
        fill,
        text_anchor,
        label_context,
        transform,
    );
    let zero_layout = attached_start_layout == PreviewAttachedStartLayoutMode::Zero
        || (transform.emf_recording
            && matches!(text_anchor, Some("middle"))
            && run.script.is_none()
            && preview_env_enabled(ENV_PACKAGED_CENTERED_PLAIN_ZERO_LAYOUT));
    let rect = if let Some(mut rect) = layout_rect_override {
        rect.Width = rect.Width.max(font_px * 0.5);
        rect.Height = rect.Height.max(1.0);
        rect
    } else if zero_layout {
        RectF {
            X: x,
            Y: top,
            Width: 0.0,
            Height: 0.0,
        }
    } else {
        let (width_scale, height_scale) =
            if attached_start_layout == PreviewAttachedStartLayoutMode::Tight {
                (1.0, 1.1)
            } else {
                (1.8, 1.45)
            };
        RectF {
            X: x,
            Y: top,
            Width: (advance * width_scale).max(font_px * 0.5),
            Height: (font_px * height_scale).max(1.0),
        }
    };
    let wide: Vec<u16> = run.text.encode_utf16().collect();
    let ok = GdipDrawString(
        graphics,
        wide.as_ptr(),
        wide.len() as i32,
        font,
        &rect,
        format,
        brush,
    ) == GDI_PLUS_OK;
    GdipDeleteStringFormat(format);
    GdipDeleteBrush(brush);
    GdipDeleteFont(font);
    ok
}

pub(super) fn preview_gdiplus_text_top(
    baseline_y: f32,
    baseline_offset: Option<f64>,
    text_anchor: Option<&str>,
    top_nudge_px: f64,
    run: Option<&PreviewTextRun>,
    default_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    let script = run.and_then(|run| run.script.as_deref());
    let font_px = (run
        .and_then(|run| run.font_size)
        .unwrap_or(default_font_size)
        * preview_script_scale(script)
        * gdiplus_text_scale(transform))
    .max(1.0) as f32;
    let baseline_top = if transform.emf_recording {
        match script {
            Some("subscript" | "superscript") => font_px * 0.905_273_44,
            _ => baseline_offset
                .map(|value| (value * gdiplus_text_scale(transform)) as f32)
                .unwrap_or(font_px * 0.905_273_44),
        }
    } else {
        font_px * 0.86
    };
    let packaged_centered_bias = if transform.emf_recording && matches!(text_anchor, Some("middle"))
    {
        let mut bias = font_px
            * preview_env_f64_or(
                ENV_PACKAGED_CENTERED_TEXT_TOP_BIAS_EM,
                CHEMDRAW_PACKAGED_CENTERED_TEXT_TOP_BIAS_EM as f64,
            ) as f32;
        if matches!(script, Some("subscript" | "superscript")) {
            bias += font_px
                * preview_env_f64_or(
                    ENV_PACKAGED_CENTERED_SCRIPT_EXTRA_TOP_BIAS_EM,
                    CHEMDRAW_PACKAGED_CENTERED_SCRIPT_EXTRA_TOP_BIAS_EM as f64,
                ) as f32;
        }
        bias
    } else {
        0.0
    };
    baseline_y - baseline_top
        + run
            .map(|run| preview_script_baseline_shift_f32(run, default_font_size, transform))
            .unwrap_or(0.0)
        - packaged_centered_bias
        + (top_nudge_px / (transform.scale * transform.record_scale.max(1.0))) as f32
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_packaged_node_label_line_rect(
    node_id: Option<&str>,
    _text_anchor: Option<&str>,
    label_context: Option<&PreviewLabelContext>,
    transform: &PreviewTransform,
    line_index: usize,
    line_count: usize,
    line_start_x: f32,
    line_top: f32,
) -> Option<RectF> {
    if !transform.emf_recording {
        return None;
    }
    let mode = preview_packaged_node_label_layout_mode()?;
    let info = label_context.and_then(|context| node_id.and_then(|id| context.infos.get(id)))?;
    let attached_only = matches!(
        mode,
        PreviewPackagedNodeLabelLayoutMode::AttachedPayloadSize
            | PreviewPackagedNodeLabelLayoutMode::AttachedPayloadBox
    );
    let simple_only = matches!(
        mode,
        PreviewPackagedNodeLabelLayoutMode::SimplePayloadSize
            | PreviewPackagedNodeLabelLayoutMode::SimplePayloadBox
    );
    if attached_only && (!info.is_attached_group_layout() || info.line_count != 1) {
        return None;
    }
    if simple_only && !info.simple_single_run {
        return None;
    }
    let world_box = info.world_box?;
    let segments = line_count.max(1) as f64;
    let world_top =
        world_box.top + (world_box.bottom - world_box.top) * line_index as f64 / segments;
    let world_bottom =
        world_box.top + (world_box.bottom - world_box.top) * (line_index + 1) as f64 / segments;
    let top_left = transform.gdip_point(CorePoint {
        x: world_box.left,
        y: world_top,
    });
    let bottom_right = transform.gdip_point(CorePoint {
        x: world_box.right,
        y: world_bottom,
    });
    let width = (bottom_right.X - top_left.X).abs().max(0.0);
    let height = (bottom_right.Y - top_left.Y).abs().max(1.0);
    Some(match mode {
        PreviewPackagedNodeLabelLayoutMode::PayloadSize
        | PreviewPackagedNodeLabelLayoutMode::AttachedPayloadSize
        | PreviewPackagedNodeLabelLayoutMode::SimplePayloadSize => RectF {
            X: line_start_x,
            Y: line_top,
            Width: width,
            Height: height,
        },
        PreviewPackagedNodeLabelLayoutMode::PayloadBox
        | PreviewPackagedNodeLabelLayoutMode::AttachedPayloadBox
        | PreviewPackagedNodeLabelLayoutMode::SimplePayloadBox => RectF {
            X: top_left.X,
            Y: top_left.Y,
            Width: width,
            Height: height,
        },
    })
}

pub(super) unsafe fn create_gdiplus_font(
    run: &PreviewTextRun,
    default_font_size: f64,
    default_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<*mut GpFont> {
    let family_name = run
        .font_family
        .as_deref()
        .or(default_family)
        .unwrap_or("Arial");
    let wide_family = wide_null(family_name);
    let mut family: *mut GpFontFamily = null_mut();
    if GdipCreateFontFamilyFromName(wide_family.as_ptr(), null_mut(), &mut family) != GDI_PLUS_OK
        || family.is_null()
    {
        return None;
    }
    let style = gdiplus_font_style(run);
    let script_scale = preview_script_scale(run.script.as_deref());
    let em_size =
        (run.font_size.unwrap_or(default_font_size) * script_scale * gdiplus_text_scale(transform))
            .max(0.1) as f32;
    let mut font: *mut GpFont = null_mut();
    let ok = GdipCreateFont(family, em_size, style, UnitPixel, &mut font) == GDI_PLUS_OK
        && !font.is_null();
    GdipDeleteFontFamily(family);
    ok.then_some(font)
}

pub(super) fn gdiplus_font_style(run: &PreviewTextRun) -> i32 {
    let mut style = FontStyleRegular;
    if run.font_weight.unwrap_or(400) >= 600 {
        style |= FontStyleBold;
    }
    if run.font_style.as_deref() == Some("italic") {
        style |= FontStyleItalic;
    }
    if run.underline.unwrap_or(false) {
        style |= FontStyleUnderline;
    }
    style
}

pub(super) unsafe fn create_gdiplus_string_format() -> Option<*mut GpStringFormat> {
    let mut format: *mut GpStringFormat = null_mut();
    let mut base: *mut GpStringFormat = null_mut();
    if GdipStringFormatGetGenericTypographic(&mut base) == GDI_PLUS_OK
        && !base.is_null()
        && GdipCloneStringFormat(base, &mut format) == GDI_PLUS_OK
        && !format.is_null()
    {
        // GenericTypographic avoids the extra layout padding in DrawString, which keeps
        // EMF text anchors aligned with the SVG renderer's alphabetic-baseline model.
    } else if GdipCreateStringFormat(0, 0, &mut format) != GDI_PLUS_OK || format.is_null() {
        return None;
    }
    let mut flags = 0x2000 | StringFormatFlagsNoClip | StringFormatFlagsMeasureTrailingSpaces;
    if !preview_env_enabled(ENV_DISABLE_PACKAGED_NOFITBLACKBOX) {
        flags |= StringFormatFlagsNoFitBlackBox;
    }
    GdipSetStringFormatFlags(format, flags);
    GdipSetStringFormatAlign(format, StringAlignmentNear);
    GdipSetStringFormatLineAlign(format, StringAlignmentNear);
    Some(format)
}

pub(super) fn gdiplus_line_cap(line_cap: Option<&str>) -> i32 {
    match line_cap {
        Some("round") => LineCapRound,
        Some("square") => LineCapSquare,
        _ => LineCapFlat,
    }
}

pub(super) fn gdiplus_line_join(line_join: Option<&str>) -> i32 {
    match line_join {
        Some("round") => LineJoinRound,
        Some("bevel") => LineJoinBevel,
        _ => LineJoinMiter,
    }
}

pub(super) fn css_argb(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return None;
    }
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        let rgb = u32::from_str_radix(hex, 16).ok()?;
        return Some(0xff000000 | rgb);
    }
    if let Some((r, g, b, alpha)) = parse_css_rgba(value) {
        let a = (alpha * 255.0).round().clamp(0.0, 255.0) as u32;
        return Some((a << 24) | (r << 16) | (g << 8) | b);
    }
    None
}
