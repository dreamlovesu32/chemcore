use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_svg_path(
    dc: HDC,
    d: &str,
    fill: Option<&str>,
    stroke: Option<&str>,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
    cache: &mut PreviewGdiCache,
) -> bool {
    let Some(commands) = parse_preview_path(d) else {
        return false;
    };
    if commands.is_empty() {
        return false;
    }

    let fill_color = fill.and_then(colorref_from_css);
    let stroke_color = stroke.and_then(colorref_from_css);
    if fill_color.is_none() {
        if let Some(color) = stroke_color {
            if draw_preview_svg_polyline_path(
                dc,
                &commands,
                color,
                stroke_width,
                line_cap,
                line_join,
                transform,
                dash_array,
            ) {
                return true;
            }
        }
    } else if let Some(points) = preview_closed_linear_path_points(&commands) {
        if stroke_color.is_none() || dash_array.is_empty() {
            draw_preview_svg_polygon_path(
                dc,
                &points,
                fill_color,
                stroke_color,
                stroke_width,
                line_cap,
                line_join,
                transform,
                dash_array,
                cache,
            );
            return true;
        }
    }

    let brush = fill_color
        .map(|color| cache.solid_brush(color))
        .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
    let pen = stroke_color
        .map(|color| {
            create_preview_pen(
                color,
                transform.pen_width(stroke_width),
                line_cap,
                line_join,
                dash_array,
                transform,
            )
        })
        .unwrap_or_else(|| GetStockObject(NULL_PEN));
    let old_brush = SelectObject(dc, brush as HGDIOBJ);
    let old_pen = SelectObject(dc, pen);
    set_preview_miter_limit(dc);
    SetPolyFillMode(dc, ALTERNATE);
    BeginPath(dc);
    replay_preview_path(dc, &commands, transform);
    EndPath(dc);
    let ok = if fill_color.is_some() {
        FillPath(dc) != 0
    } else {
        StrokePath(dc) != 0
    };
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen);
    ok
}

pub(super) unsafe fn begin_preview_clip(
    dc: HDC,
    clip_path_d: Option<&str>,
    _clip_rule: Option<&str>,
    transform: &PreviewTransform,
) -> i32 {
    let Some(clip_path_d) = clip_path_d else {
        return 0;
    };
    let saved = SaveDC(dc);
    if saved == 0 {
        return 0;
    }
    if apply_preview_clip_path(dc, clip_path_d, transform) {
        saved
    } else {
        RestoreDC(dc, saved);
        0
    }
}

pub(super) unsafe fn end_preview_clip(dc: HDC, saved: i32) {
    if saved != 0 {
        RestoreDC(dc, saved);
    }
}

pub(super) unsafe fn apply_preview_clip_path(
    dc: HDC,
    d: &str,
    transform: &PreviewTransform,
) -> bool {
    let Some(commands) = parse_preview_path(d) else {
        return false;
    };
    if commands.is_empty() {
        return false;
    }
    SetPolyFillMode(dc, ALTERNATE);
    BeginPath(dc);
    replay_preview_path(dc, &commands, transform);
    EndPath(dc);
    SelectClipPath(dc, RGN_AND) != 0
}

pub(super) unsafe fn replay_preview_path(
    dc: HDC,
    commands: &[PreviewPathCommand],
    transform: &PreviewTransform,
) {
    let mut index = 0;
    let mut current = None;
    while index < commands.len() {
        match commands[index] {
            PreviewPathCommand::Move(point) => {
                current = Some(point);
                if !matches!(
                    commands.get(index + 1),
                    Some(PreviewPathCommand::Cubic(_, _, _))
                ) {
                    let p = transform.point(point);
                    MoveToEx(dc, p.x, p.y, null_mut());
                }
                index += 1;
            }
            PreviewPathCommand::Line(point) => {
                let p = transform.point(point);
                LineTo(dc, p.x, p.y);
                current = Some(point);
                index += 1;
            }
            PreviewPathCommand::Cubic(c1, c2, end) => {
                let Some(start) = current else {
                    let mapped = [
                        transform.point(c1),
                        transform.point(c2),
                        transform.point(end),
                    ];
                    PolyBezierTo(dc, mapped.as_ptr(), mapped.len() as u32);
                    current = Some(end);
                    index += 1;
                    continue;
                };
                let mut mapped = vec![transform.point(start)];
                while index < commands.len() {
                    let PreviewPathCommand::Cubic(c1, c2, end) = commands[index] else {
                        break;
                    };
                    mapped.push(transform.point(c1));
                    mapped.push(transform.point(c2));
                    mapped.push(transform.point(end));
                    current = Some(end);
                    index += 1;
                }
                PolyBezier(dc, mapped.as_ptr(), mapped.len() as u32);
            }
            PreviewPathCommand::Close => {
                CloseFigure(dc);
                index += 1;
            }
        }
    }
}

pub(super) fn preview_closed_linear_path_points(
    commands: &[PreviewPathCommand],
) -> Option<Vec<CorePoint>> {
    let mut points = Vec::new();
    let mut current = None;
    let mut started = false;
    let mut closed = false;
    for command in commands {
        if closed {
            return None;
        }
        match *command {
            PreviewPathCommand::Move(point) => {
                if started {
                    return None;
                }
                points.push(point);
                current = Some(point);
                started = true;
            }
            PreviewPathCommand::Line(point) => {
                if !started {
                    return None;
                }
                points.push(point);
                current = Some(point);
            }
            PreviewPathCommand::Cubic(c1, c2, end) => {
                let start = current?;
                if !preview_cubic_is_line(start, c1, c2, end) {
                    return None;
                }
                points.push(end);
                current = Some(end);
            }
            PreviewPathCommand::Close => {
                closed = true;
            }
        }
    }
    if !closed
        && points
            .last()
            .is_some_and(|last| last.distance(points[0]) <= 0.01)
    {
        closed = true;
    }
    if !closed || points.len() < 3 {
        return None;
    }
    if points
        .last()
        .is_some_and(|last| last.distance(points[0]) <= 0.01)
    {
        points.pop();
    }
    if points.len() < 3 || polygon_area(&points).abs() <= 0.01 {
        None
    } else {
        Some(points)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_svg_polygon_path(
    dc: HDC,
    points: &[CorePoint],
    fill_color: Option<COLORREF>,
    stroke_color: Option<COLORREF>,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
    cache: &mut PreviewGdiCache,
) {
    if points.len() < 3 {
        return;
    }
    let mapped: Vec<POINT> = points.iter().map(|point| transform.point(*point)).collect();
    let brush = fill_color
        .map(|color| cache.solid_brush(color))
        .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
    let pen = stroke_color
        .map(|color| {
            create_preview_pen(
                color,
                transform.pen_width(stroke_width),
                line_cap,
                line_join,
                dash_array,
                transform,
            )
        })
        .unwrap_or_else(|| GetStockObject(NULL_PEN));
    let old_brush = SelectObject(dc, brush as HGDIOBJ);
    let old_pen = SelectObject(dc, pen);
    set_preview_miter_limit(dc);
    Polygon(dc, mapped.as_ptr(), mapped.len() as i32);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen);
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_svg_polyline_path(
    dc: HDC,
    commands: &[PreviewPathCommand],
    color: COLORREF,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) -> bool {
    let mut subpaths = Vec::<Vec<POINT>>::new();
    let mut current = Vec::<POINT>::new();
    let mut current_core = None;
    let mut start = None;
    for command in commands {
        match *command {
            PreviewPathCommand::Move(point) => {
                if current.len() >= 2 {
                    subpaths.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                let mapped = transform.point(point);
                current.push(mapped);
                start = Some(mapped);
                current_core = Some(point);
            }
            PreviewPathCommand::Line(point) => {
                if current.is_empty() {
                    return false;
                }
                current.push(transform.point(point));
                current_core = Some(point);
            }
            PreviewPathCommand::Close => {
                let Some(start) = start else {
                    return false;
                };
                if current.is_empty() {
                    return false;
                }
                current.push(start);
            }
            PreviewPathCommand::Cubic(c1, c2, end) => {
                let Some(start) = current_core else {
                    return false;
                };
                if !preview_cubic_is_line(start, c1, c2, end) {
                    return false;
                }
                current.push(transform.point(end));
                current_core = Some(end);
            }
        }
    }
    if current.len() >= 2 {
        subpaths.push(current);
    }
    if subpaths.is_empty() {
        return false;
    }

    let pen = create_preview_pen(
        color,
        transform.pen_width(stroke_width),
        line_cap,
        line_join,
        dash_array,
        transform,
    );
    let old_pen = SelectObject(dc, pen);
    set_preview_miter_limit(dc);
    for subpath in &subpaths {
        Polyline(dc, subpath.as_ptr(), subpath.len() as i32);
    }
    SelectObject(dc, old_pen);
    delete_preview_pen(pen);
    true
}

pub(super) fn preview_cubic_is_line(
    start: CorePoint,
    c1: CorePoint,
    c2: CorePoint,
    end: CorePoint,
) -> bool {
    let length = start.distance(end);
    if length <= 0.01 {
        return c1.distance(start) <= 0.01 && c2.distance(end) <= 0.01;
    }
    point_line_distance(c1, start, end) <= 0.01 && point_line_distance(c2, start, end) <= 0.01
}

pub(super) fn point_line_distance(point: CorePoint, start: CorePoint, end: CorePoint) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= 0.0 {
        return point.distance(start);
    }
    ((point.x - start.x) * dy - (point.y - start.y) * dx).abs() / length
}

pub(super) fn parse_preview_path(d: &str) -> Option<Vec<PreviewPathCommand>> {
    let mut parser = PreviewPathParser::new(d);
    parser.parse()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_preview_arc_cubics(
    out: &mut Vec<PreviewPathCommand>,
    start: CorePoint,
    end: CorePoint,
    rx: f64,
    ry: f64,
    x_axis_rotation: f64,
    large_arc: bool,
    sweep: bool,
) -> Option<()> {
    let x_axis_rotation = if (rx - ry).abs() <= 1.0e-6 {
        0.0
    } else {
        x_axis_rotation
    };
    if x_axis_rotation.abs() > 1.0e-6 {
        return None;
    }
    let mut rx = rx.abs();
    let mut ry = ry.abs();
    if rx <= 0.0 || ry <= 0.0 {
        out.push(PreviewPathCommand::Line(end));
        return Some(());
    }
    if (start.x - end.x).abs() < 1.0e-9 && (start.y - end.y).abs() < 1.0e-9 {
        return Some(());
    }

    let x1p = (start.x - end.x) * 0.5;
    let y1p = (start.y - end.y) * 0.5;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
    }

    let numerator = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let denominator = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let coefficient = if denominator.abs() < 1.0e-12 {
        0.0
    } else {
        let sign = if large_arc == sweep { -1.0 } else { 1.0 };
        sign * (numerator / denominator).max(0.0).sqrt()
    };
    let cxp = coefficient * rx * y1p / ry;
    let cyp = -coefficient * ry * x1p / rx;
    let center = CorePoint {
        x: cxp + (start.x + end.x) * 0.5,
        y: cyp + (start.y + end.y) * 0.5,
    };

    let theta1 = ((y1p - cyp) / ry).atan2((x1p - cxp) / rx);
    let theta2 = ((-y1p - cyp) / ry).atan2((-x1p - cxp) / rx);
    let mut delta = theta2 - theta1;
    while delta > PI {
        delta -= 2.0 * PI;
    }
    while delta < -PI {
        delta += 2.0 * PI;
    }
    if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    } else if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    }

    let segments = (delta.abs() / (PI * 0.5)).ceil().max(1.0) as usize;
    let step = delta / segments as f64;
    for index in 0..segments {
        let a0 = theta1 + step * index as f64;
        let a1 = a0 + step;
        let alpha = (4.0 / 3.0) * ((a1 - a0) * 0.25).tan();
        let p1 = CorePoint {
            x: center.x + rx * (a0.cos() - alpha * a0.sin()),
            y: center.y + ry * (a0.sin() + alpha * a0.cos()),
        };
        let p2 = CorePoint {
            x: center.x + rx * (a1.cos() + alpha * a1.sin()),
            y: center.y + ry * (a1.sin() - alpha * a1.cos()),
        };
        let p3 = CorePoint {
            x: center.x + rx * a1.cos(),
            y: center.y + ry * a1.sin(),
        };
        out.push(PreviewPathCommand::Cubic(p1, p2, p3));
    }
    Some(())
}

pub(super) fn is_oval_bounds_path(d: &str, points: &[CorePoint]) -> bool {
    points.len() == 2 && (d.contains(" A ") || d.contains(" C ")) && !d.contains(" L ")
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_oval_bounds(
    dc: HDC,
    points: &[CorePoint],
    fill: Option<&str>,
    stroke: Option<&str>,
    stroke_width: f64,
    transform: &PreviewTransform,
    dash_array: &[f64],
    cache: &mut PreviewGdiCache,
) {
    if points.len() != 2 {
        return;
    }
    let p1 = transform.point(points[0]);
    let p2 = transform.point(points[1]);
    let left = p1.x.min(p2.x);
    let top = p1.y.min(p2.y);
    let right = p1.x.max(p2.x);
    let bottom = p1.y.max(p2.y);
    let fill_color = fill.and_then(colorref_from_css);
    let stroke_color = stroke
        .and_then(colorref_from_css)
        .or(fill_color)
        .unwrap_or(0x000000);
    let brush = fill_color
        .map(|color| cache.solid_brush(color))
        .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
    let pen = create_preview_pen(
        stroke_color,
        transform.pen_width(stroke_width),
        Some("round"),
        Some("round"),
        dash_array,
        transform,
    );
    let old_brush = SelectObject(dc, brush as HGDIOBJ);
    let old_pen = SelectObject(dc, pen);
    set_preview_miter_limit(dc);
    Ellipse(dc, left, top, right, bottom);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen);
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_polygon(
    dc: HDC,
    role: RenderRole,
    bond_id: Option<&str>,
    points: &[CorePoint],
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    bond_context: Option<&PreviewBondContext>,
) {
    if points.len() < 2 {
        return;
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
            draw_preview_polyline(
                dc,
                &line_points,
                fill,
                stroke_line.width,
                Some("round"),
                None,
                transform,
                &[],
            );
            return;
        }
        if transform.office_presentation && hashed_wedge.is_some() {
            let mapped: Vec<POINT> = points.iter().map(|point| transform.point(*point)).collect();
            let brush = colorref_from_css(fill)
                .map(|color| cache.solid_brush(color))
                .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
            let old_brush = SelectObject(dc, brush as HGDIOBJ);
            let old_pen = SelectObject(dc, GetStockObject(NULL_PEN));
            Polygon(dc, mapped.as_ptr(), mapped.len() as i32);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            return;
        }
    }
    let mapped: Vec<POINT> = points.iter().map(|point| transform.point(*point)).collect();
    let fill_color = colorref_from_css(fill);
    let brush = fill_color
        .map(|color| cache.solid_brush(color))
        .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
    let pen = create_preview_pen(
        colorref_from_css(stroke).unwrap_or_else(|| colorref_from_css(fill).unwrap_or(0x000000)),
        transform.pen_width(stroke_width),
        Some("butt"),
        Some("miter"),
        &[],
        transform,
    );
    let old_brush = SelectObject(dc, brush as HGDIOBJ);
    let old_pen = SelectObject(dc, pen);
    set_preview_miter_limit(dc);
    Polygon(dc, mapped.as_ptr(), mapped.len() as i32);
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen);
}
