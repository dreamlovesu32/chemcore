// GDI replay for Chemcore document preview primitives.
//
// Keep Office/OLE container decisions out of this file. Code here should be
// about geometry, pens, brushes, text metrics, path replay, clipping, and the
// ChemDraw-style EMF record strategy.

use super::*;

const EMF_VECTOR_RECORD_SCALE: f64 = 16.0;
const EMF_ARROW_RECORD_SCALE: f64 = 3.0;

#[derive(Clone, Copy)]
struct PreviewTransform {
    min_x: f64,
    min_y: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    record_scale: f64,
}

impl PreviewTransform {
    fn from_bounds(bounds: &RECT, primitive_bounds: [f64; 4]) -> Option<Self> {
        let [min_x, min_y, max_x, max_y] = primitive_bounds;
        let source_width = (max_x - min_x).max(1.0);
        let source_height = (max_y - min_y).max(1.0);
        let target_width = (bounds.right - bounds.left).max(1) as f64;
        let target_height = (bounds.bottom - bounds.top).max(1) as f64;
        let scale = (target_width / source_width).min(target_height / source_height);
        if !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let drawn_width = source_width * scale;
        let drawn_height = source_height * scale;
        Some(Self {
            min_x,
            min_y,
            scale,
            offset_x: bounds.left as f64 + (target_width - drawn_width) / 2.0,
            offset_y: bounds.top as f64 + (target_height - drawn_height) / 2.0,
            record_scale: 1.0,
        })
    }

    fn with_record_scale(self, record_scale: f64) -> Self {
        Self {
            record_scale: record_scale.max(1.0),
            ..self
        }
    }

    fn point(&self, point: CorePoint) -> POINT {
        POINT {
            x: ((self.offset_x + (point.x - self.min_x) * self.scale) * self.record_scale).round()
                as i32,
            y: ((self.offset_y + (point.y - self.min_y) * self.scale) * self.record_scale).round()
                as i32,
        }
    }

    fn xy(&self, x: f64, y: f64) -> POINT {
        self.point(CorePoint { x, y })
    }

    fn length(&self, value: f64) -> i32 {
        (value.abs() * self.scale * self.record_scale)
            .round()
            .max(1.0) as i32
    }
}

pub(super) unsafe fn draw_payload_preview(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
) -> bool {
    if draw_payload_vector_preview(dc, bounds, payload) {
        return true;
    }

    draw_svg_preview(dc, bounds, payload)
}

pub(super) unsafe fn draw_payload_vector_preview(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
) -> bool {
    draw_payload_vector_preview_with_source_bounds(dc, bounds, payload, None)
}

pub(super) unsafe fn draw_payload_vector_preview_with_source_bounds(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
) -> bool {
    draw_payload_vector_preview_internal(dc, bounds, payload, source_bounds, false)
}

pub(super) unsafe fn draw_payload_emf_vector_preview_with_source_bounds(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
) -> bool {
    draw_payload_vector_preview_internal(dc, bounds, payload, source_bounds, true)
}

unsafe fn draw_payload_vector_preview_internal(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
    high_resolution_vectors: bool,
) -> bool {
    let Ok(document) = parse_document_json(&payload.chemcore_document_json) else {
        return false;
    };
    let primitives = render_document(&document);
    let visible: Vec<_> = primitives
        .iter()
        .filter(|primitive| office_preview_primitive_visible(primitive))
        .collect();
    let Some(primitive_bounds) = render_primitives_bounds(visible.iter().copied()) else {
        return false;
    };
    let Some(transform) =
        PreviewTransform::from_bounds(bounds, source_bounds.unwrap_or(primitive_bounds))
    else {
        return false;
    };

    let mut cache = PreviewGdiCache::default();
    let mut vector_scope = 0;
    let mut active_record_scale = 1.0;
    let mut high_resolution_available = high_resolution_vectors;
    for primitive in visible {
        let record_scale = if high_resolution_available {
            preview_primitive_record_scale(primitive)
        } else {
            1.0
        };
        if record_scale > 1.0 {
            if vector_scope != 0 && (active_record_scale - record_scale).abs() > f64::EPSILON {
                RestoreDC(dc, vector_scope);
                vector_scope = 0;
                active_record_scale = 1.0;
            }
            if vector_scope == 0 {
                vector_scope = begin_high_resolution_vector_scope(dc, record_scale);
                if vector_scope == 0 {
                    high_resolution_available = false;
                }
                active_record_scale = record_scale;
            }
            if high_resolution_available {
                let vector_transform = transform.with_record_scale(record_scale);
                draw_preview_primitive(dc, primitive, &vector_transform, &mut cache);
                continue;
            }
        } else if vector_scope != 0 {
            RestoreDC(dc, vector_scope);
            vector_scope = 0;
        }
        draw_preview_primitive(dc, primitive, &transform, &mut cache);
    }
    if vector_scope != 0 {
        RestoreDC(dc, vector_scope);
    }
    cache.delete_objects();
    true
}

unsafe fn begin_high_resolution_vector_scope(dc: HDC, record_scale: f64) -> i32 {
    if !record_scale.is_finite() || record_scale <= 1.0 {
        return 0;
    }
    let saved = SaveDC(dc);
    if saved == 0 {
        return 0;
    }
    if SetGraphicsMode(dc, GM_ADVANCED) == 0 {
        RestoreDC(dc, saved);
        return 0;
    }
    let inverse = (1.0 / record_scale) as f32;
    let transform = XFORM {
        eM11: inverse,
        eM12: 0.0,
        eM21: 0.0,
        eM22: inverse,
        eDx: 0.0,
        eDy: 0.0,
    };
    if SetWorldTransform(dc, &transform) == 0 {
        RestoreDC(dc, saved);
        return 0;
    }
    saved
}

fn preview_primitive_record_scale(primitive: &RenderPrimitive) -> f64 {
    match primitive {
        RenderPrimitive::Text { .. } => 1.0,
        RenderPrimitive::Line {
            role, object_id, ..
        }
        | RenderPrimitive::Circle {
            role, object_id, ..
        }
        | RenderPrimitive::Polygon {
            role, object_id, ..
        }
        | RenderPrimitive::Rect {
            role, object_id, ..
        }
        | RenderPrimitive::Ellipse {
            role, object_id, ..
        }
        | RenderPrimitive::Polyline {
            role, object_id, ..
        }
        | RenderPrimitive::Path {
            role, object_id, ..
        }
        | RenderPrimitive::FilledPath {
            role, object_id, ..
        } => {
            if *role == RenderRole::DocumentBond {
                return EMF_VECTOR_RECORD_SCALE;
            }
            if *role != RenderRole::DocumentGraphic {
                return 1.0;
            }
            if object_id
                .as_deref()
                .is_some_and(|id| id.starts_with("obj_line_"))
            {
                EMF_ARROW_RECORD_SCALE
            } else {
                EMF_VECTOR_RECORD_SCALE
            }
        }
    }
}

struct SvgPreviewBitmap {
    width: i32,
    height: i32,
    bgra: Vec<u8>,
}

fn render_svg_preview_bitmap(svg: &str) -> Option<SvgPreviewBitmap> {
    if svg.trim().is_empty() {
        return None;
    }
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options).ok()?;
    let size = tree.size().to_int_size();
    let source_width = size.width().max(1);
    let source_height = size.height().max(1);
    let max_side = 2400.0_f32;
    let scale = (max_side / source_width.max(source_height) as f32).min(1.0);
    let width = ((source_width as f32) * scale).round().max(1.0) as u32;
    let height = ((source_height as f32) * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap_mut,
    );

    let mut bgra = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for pixel in pixmap.data().chunks_exact(4) {
        bgra.push(pixel[2]);
        bgra.push(pixel[1]);
        bgra.push(pixel[0]);
        bgra.push(0xFF);
    }

    Some(SvgPreviewBitmap {
        width: width as i32,
        height: height as i32,
        bgra,
    })
}

unsafe fn draw_svg_preview(dc: HDC, bounds: &RECT, payload: &OleObjectPayload) -> bool {
    let Some(bitmap) = render_svg_preview_bitmap(&payload.svg) else {
        return false;
    };
    let target_width = (bounds.right - bounds.left).max(1);
    let target_height = (bounds.bottom - bounds.top).max(1);
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bitmap.width,
            biHeight: -bitmap.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: unsafe { zeroed() },
    };
    let lines = StretchDIBits(
        dc,
        bounds.left,
        bounds.top,
        target_width,
        target_height,
        0,
        0,
        bitmap.width,
        bitmap.height,
        bitmap.bgra.as_ptr().cast::<c_void>(),
        &mut info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    lines != 0
}

pub(super) fn office_preview_primitive_visible(primitive: &RenderPrimitive) -> bool {
    let role = match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. } => role,
    };
    matches!(
        role,
        RenderRole::DocumentBond | RenderRole::DocumentGraphic | RenderRole::DocumentText
    )
}

unsafe fn draw_preview_primitive(
    dc: HDC,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) {
    match primitive {
        RenderPrimitive::Line {
            role,
            from,
            to,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => draw_preview_line(
            dc,
            transform.point(*from),
            transform.point(*to),
            stroke,
            *stroke_width,
            if *role == RenderRole::DocumentBond {
                Some("round")
            } else {
                Some("butt")
            },
            Some("miter"),
            transform,
            dash_array,
        ),
        RenderPrimitive::Polygon {
            role,
            points,
            fill,
            stroke,
            stroke_width,
            ..
        } => draw_preview_polygon(
            dc,
            *role,
            points,
            fill,
            stroke,
            *stroke_width,
            transform,
            cache,
        ),
        RenderPrimitive::FilledPath {
            d,
            points,
            fill,
            clip_path_d,
            clip_rule,
            ..
        } => {
            let saved_clip =
                begin_preview_clip(dc, clip_path_d.as_deref(), clip_rule.as_deref(), transform);
            if draw_preview_svg_path(
                dc,
                d,
                Some(fill.as_str()),
                None,
                0.0,
                None,
                None,
                transform,
                &[],
                cache,
            ) {
                end_preview_clip(dc, saved_clip);
                return;
            }
            if is_oval_bounds_path(d, points) {
                draw_preview_oval_bounds(
                    dc,
                    points,
                    Some(fill.as_str()),
                    Some(fill.as_str()),
                    0.0,
                    transform,
                    &[],
                    cache,
                );
            } else {
                draw_preview_polygon(
                    dc,
                    RenderRole::DocumentGraphic,
                    points,
                    fill,
                    fill,
                    0.0,
                    transform,
                    cache,
                );
            }
            end_preview_clip(dc, saved_clip);
        }
        RenderPrimitive::Polyline {
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => {
            draw_preview_polyline(
                dc,
                points,
                stroke,
                *stroke_width,
                line_cap.as_deref(),
                line_join.as_deref(),
                transform,
                dash_array,
            );
        }
        RenderPrimitive::Path {
            d,
            points,
            stroke,
            stroke_width,
            dash_array,
            line_cap,
            line_join,
            ..
        } => {
            if draw_preview_svg_path(
                dc,
                d,
                None,
                Some(stroke.as_str()),
                *stroke_width,
                line_cap.as_deref(),
                line_join.as_deref(),
                transform,
                dash_array,
                cache,
            ) {
                return;
            }
            if is_oval_bounds_path(d, points) {
                draw_preview_oval_bounds(
                    dc,
                    points,
                    None,
                    Some(stroke.as_str()),
                    *stroke_width,
                    transform,
                    dash_array,
                    cache,
                );
            } else {
                draw_preview_polyline(
                    dc,
                    points,
                    stroke,
                    *stroke_width,
                    line_cap.as_deref(),
                    line_join.as_deref(),
                    transform,
                    dash_array,
                );
            }
        }
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
        } => {
            let p1 = transform.xy(*x, *y);
            let p2 = transform.xy(*x + *width, *y + *height);
            let fill_color = fill.as_deref().and_then(colorref_from_css);
            let brush = fill_color
                .map(|color| cache.solid_brush(color))
                .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
            let pen = stroke
                .as_deref()
                .and_then(colorref_from_css)
                .map(|color| {
                    create_preview_pen(
                        color,
                        transform.length(*stroke_width),
                        Some("butt"),
                        Some("miter"),
                        dash_array,
                        transform,
                    )
                })
                .unwrap_or_else(|| GetStockObject(NULL_PEN));
            let old_brush = SelectObject(dc, brush as HGDIOBJ);
            let old_pen = SelectObject(dc, pen);
            set_preview_miter_limit(dc);
            Rectangle(dc, p1.x, p1.y, p2.x, p2.y);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            delete_preview_pen(pen);
        }
        RenderPrimitive::Ellipse {
            center,
            rx,
            ry,
            fill,
            stroke,
            stroke_width,
            dash_array,
            ..
        } => {
            let c = transform.point(*center);
            let rx = transform.length(*rx);
            let ry = transform.length(*ry);
            let fill_color = fill.as_deref().and_then(colorref_from_css);
            let brush = fill_color
                .map(|color| cache.solid_brush(color))
                .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
            let pen = stroke
                .as_deref()
                .and_then(colorref_from_css)
                .map(|color| {
                    create_preview_pen(
                        color,
                        transform.length(*stroke_width),
                        Some("round"),
                        Some("round"),
                        dash_array,
                        transform,
                    )
                })
                .unwrap_or_else(|| GetStockObject(NULL_PEN));
            let old_brush = SelectObject(dc, brush as HGDIOBJ);
            let old_pen = SelectObject(dc, pen);
            set_preview_miter_limit(dc);
            Ellipse(dc, c.x - rx, c.y - ry, c.x + rx, c.y + ry);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            delete_preview_pen(pen);
        }
        RenderPrimitive::Circle {
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => {
            let c = transform.point(*center);
            let r = transform.length(*radius);
            let fill_color = colorref_from_css(fill);
            let brush = fill_color
                .map(|color| cache.solid_brush(color))
                .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
            let pen = colorref_from_css(stroke)
                .map(|color| {
                    create_preview_pen(
                        color,
                        transform.length(*stroke_width),
                        Some("round"),
                        Some("round"),
                        &[],
                        transform,
                    )
                })
                .unwrap_or_else(|| GetStockObject(NULL_PEN));
            let old_brush = SelectObject(dc, brush as HGDIOBJ);
            let old_pen = SelectObject(dc, pen);
            set_preview_miter_limit(dc);
            Ellipse(dc, c.x - r, c.y - r, c.x + r, c.y + r);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            delete_preview_pen(pen);
        }
        RenderPrimitive::Text {
            x,
            y,
            text,
            font_size,
            font_family,
            fill,
            text_anchor,
            line_height,
            runs,
            ..
        } => {
            draw_preview_text(
                dc,
                *x,
                *y,
                text,
                *font_size,
                font_family.as_deref(),
                fill.as_deref(),
                text_anchor.as_deref(),
                *line_height,
                runs,
                transform,
                cache,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_preview_text(
    dc: HDC,
    x: f64,
    y: f64,
    text: &str,
    font_size: f64,
    font_family: Option<&str>,
    fill: Option<&str>,
    text_anchor: Option<&str>,
    line_height: Option<f64>,
    runs: &[chemcore_engine::LabelRun],
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) {
    let old_align = SetTextAlign(dc, TA_LEFT | TA_BASELINE);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, fill.and_then(colorref_from_css).unwrap_or(0x000000));

    let line_step_world = line_height.unwrap_or(font_size * 1.2).max(0.01);
    let lines = preview_text_lines(text, runs);
    for (index, line_runs) in lines.iter().enumerate() {
        if line_runs.is_empty() {
            continue;
        }
        let origin = transform.xy(x, y + index as f64 * line_step_world);
        let width = preview_line_width(line_runs, font_size, transform);
        let mut cursor_x = match text_anchor {
            Some("middle") => origin.x - width / 2,
            Some("end") => origin.x - width,
            _ => origin.x,
        };
        for run in line_runs {
            cursor_x += draw_preview_text_run(
                dc,
                cursor_x,
                origin.y,
                run,
                font_size,
                font_family,
                transform,
                cache,
            );
        }
    }

    SetTextAlign(dc, old_align);
}

#[derive(Clone)]
struct PreviewTextRun {
    text: String,
    font_family: Option<String>,
    font_size: Option<f64>,
    fill: Option<String>,
    font_weight: Option<u32>,
    font_style: Option<String>,
    underline: Option<bool>,
    script: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
struct PreviewFontKey {
    height: i32,
    family: String,
    weight: i32,
    italic: bool,
    underline: bool,
}

#[derive(Default)]
struct PreviewGdiCache {
    fonts: Vec<(PreviewFontKey, HGDIOBJ)>,
    brushes: Vec<(COLORREF, HGDIOBJ)>,
}

impl PreviewGdiCache {
    unsafe fn solid_brush(&mut self, color: COLORREF) -> HGDIOBJ {
        if let Some((_, brush)) = self.brushes.iter().find(|(cached, _)| *cached == color) {
            return *brush;
        }
        let brush = CreateSolidBrush(color) as HGDIOBJ;
        if !brush.is_null() {
            self.brushes.push((color, brush));
        }
        brush
    }

    unsafe fn font_for_run(
        &mut self,
        run: &PreviewTextRun,
        fallback_font_size: f64,
        fallback_family: Option<&str>,
        transform: &PreviewTransform,
    ) -> HGDIOBJ {
        let key = preview_font_key(run, fallback_font_size, fallback_family, transform);
        if let Some((_, font)) = self.fonts.iter().find(|(cached, _)| cached == &key) {
            return *font;
        }
        let font = create_preview_font(&key);
        if !font.is_null() {
            self.fonts.push((key, font));
        }
        font
    }

    unsafe fn delete_objects(&mut self) {
        for (_, font) in self.fonts.drain(..) {
            DeleteObject(font);
        }
        for (_, brush) in self.brushes.drain(..) {
            DeleteObject(brush);
        }
    }
}

fn preview_text_lines(text: &str, runs: &[chemcore_engine::LabelRun]) -> Vec<Vec<PreviewTextRun>> {
    if runs.is_empty() {
        return text
            .lines()
            .map(|line| {
                vec![PreviewTextRun {
                    text: line.to_string(),
                    font_family: None,
                    font_size: None,
                    fill: None,
                    font_weight: None,
                    font_style: None,
                    underline: None,
                    script: None,
                }]
            })
            .collect();
    }

    let mut lines = vec![Vec::new()];
    for run in runs {
        let segments: Vec<&str> = run.text.split('\n').collect();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                lines.last_mut().expect("line exists").push(PreviewTextRun {
                    text: (*segment).to_string(),
                    font_family: run.font_family.clone(),
                    font_size: run.font_size,
                    fill: run.fill.clone(),
                    font_weight: run.font_weight,
                    font_style: run.font_style.clone(),
                    underline: run.underline,
                    script: run.script.clone(),
                });
            }
            if index + 1 < segments.len() {
                lines.push(Vec::new());
            }
        }
    }
    lines
}

fn preview_line_width(
    runs: &[PreviewTextRun],
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    runs.iter()
        .map(|run| preview_text_run_advance_estimate(run, fallback_font_size, transform))
        .sum()
}

unsafe fn draw_preview_text_run(
    dc: HDC,
    x: i32,
    baseline_y: i32,
    run: &PreviewTextRun,
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) -> i32 {
    let label: Vec<u16> = run.text.encode_utf16().collect();
    if label.is_empty() {
        return 0;
    }
    let font = cache.font_for_run(run, fallback_font_size, fallback_family, transform);
    let old_font = if font.is_null() {
        null_mut()
    } else {
        SelectObject(dc, font as HGDIOBJ)
    };
    let text_color = run
        .fill
        .as_deref()
        .and_then(colorref_from_css)
        .unwrap_or(0x000000);
    SetTextColor(dc, text_color);
    let script_shift = preview_script_baseline_shift(run, fallback_font_size, transform);
    TextOutW(
        dc,
        x,
        baseline_y + script_shift,
        label.as_ptr(),
        label.len() as i32,
    );
    let advance = preview_text_extent(dc, &label)
        .unwrap_or_else(|| preview_text_run_advance_estimate(run, fallback_font_size, transform));
    if !font.is_null() {
        if !old_font.is_null() {
            SelectObject(dc, old_font);
        }
    }
    advance
}

unsafe fn preview_text_extent(dc: HDC, label: &[u16]) -> Option<i32> {
    let mut size = SIZE { cx: 0, cy: 0 };
    if GetTextExtentPoint32W(dc, label.as_ptr(), label.len() as i32, &mut size) == 0 {
        None
    } else {
        Some(size.cx.max(0))
    }
}

fn preview_text_run_advance_estimate(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_size = run.font_size.unwrap_or(fallback_font_size) * script_scale;
    let world_width: f64 = run
        .text
        .chars()
        .map(|character| preview_char_advance_em(character) * font_size)
        .sum();
    (world_width * transform.scale).round().max(0.0) as i32
}

fn preview_font_key(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    transform: &PreviewTransform,
) -> PreviewFontKey {
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_size = run.font_size.unwrap_or(fallback_font_size) * script_scale;
    PreviewFontKey {
        height: transform.length(font_size).max(1),
        family: run
            .font_family
            .as_deref()
            .or(fallback_family)
            .unwrap_or("Arial")
            .to_string(),
        weight: run.font_weight.unwrap_or(400).clamp(100, 900) as i32,
        italic: run.font_style.as_deref() == Some("italic"),
        underline: run.underline.unwrap_or(false),
    }
}

unsafe fn create_preview_font(key: &PreviewFontKey) -> HGDIOBJ {
    let family = wide_null(&key.family);
    CreateFontW(
        -key.height,
        0,
        0,
        0,
        key.weight,
        key.italic as u32,
        key.underline as u32,
        0,
        0,
        0,
        0,
        ANTIALIASED_QUALITY as u32,
        0,
        family.as_ptr(),
    ) as HGDIOBJ
}

fn preview_script_baseline_shift(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    let base_height = transform.length(run.font_size.unwrap_or(fallback_font_size));
    match run.script.as_deref() {
        Some("subscript") => (base_height as f64 * 0.22).round() as i32,
        Some("superscript") => -(base_height as f64 * 0.38).round() as i32,
        _ => 0,
    }
}

fn preview_script_scale(script: Option<&str>) -> f64 {
    match script {
        Some("subscript" | "superscript") => 0.7,
        _ => 1.0,
    }
}

fn preview_char_advance_em(character: char) -> f64 {
    match character {
        ' ' | '\t' => 0.32,
        'i' | 'l' | 'I' | '!' | '|' => 0.28,
        'f' | 'j' | 'r' | 't' | ',' | '.' | ':' | ';' => 0.34,
        '(' | ')' | '[' | ']' | '{' | '}' => 0.36,
        'M' | 'W' => 0.86,
        'm' | 'w' => 0.78,
        '0'..='9' => 0.56,
        'A'..='Z' => 0.68,
        '+' | '-' | '=' | '/' | '\\' => 0.55,
        _ if character.is_ascii() => 0.52,
        _ => 0.9,
    }
}

fn ansi_metafile_text_bytes(text: &str) -> Vec<u8> {
    const CP_ACP: u32 = 0;
    let wide: Vec<u16> = text.encode_utf16().collect();
    if wide.is_empty() {
        return Vec::new();
    }
    unsafe {
        let needed = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            null_mut(),
            0,
            null(),
            null_mut(),
        );
        if needed <= 0 {
            return text
                .chars()
                .map(|ch| if ch.is_ascii() { ch as u8 } else { b'?' })
                .collect();
        }
        let mut out = vec![0u8; needed as usize];
        let written = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            out.as_mut_ptr(),
            out.len() as i32,
            null(),
            null_mut(),
        );
        if written <= 0 {
            Vec::new()
        } else {
            out.truncate(written as usize);
            out
        }
    }
}

const PREVIEW_MITER_LIMIT: f32 = 10.0;

fn preview_pen_style(line_cap: Option<&str>, line_join: Option<&str>, style: i32) -> u32 {
    let cap = match line_cap {
        Some("round") => PS_ENDCAP_ROUND,
        Some("square") => PS_ENDCAP_SQUARE,
        _ => PS_ENDCAP_FLAT,
    };
    let join = match line_join {
        Some("round") => PS_JOIN_ROUND,
        Some("bevel") => PS_JOIN_BEVEL,
        _ => PS_JOIN_MITER,
    };
    (PS_GEOMETRIC | style | cap | join) as u32
}

fn preview_dash_style(dash_array: &[f64], transform: &PreviewTransform) -> Vec<u32> {
    dash_array
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| transform.length(value).max(1) as u32)
        .collect()
}

fn preview_builtin_dash_pattern(style: &[u32]) -> bool {
    match style {
        [_] => true,
        [dash, gap] => dash.abs_diff(*gap) <= 1,
        _ => false,
    }
}

unsafe fn create_preview_pen(
    color: COLORREF,
    width: i32,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    dash_array: &[f64],
    transform: &PreviewTransform,
) -> HGDIOBJ {
    if width <= 0 {
        return GetStockObject(NULL_PEN);
    }
    let mut dash_style = preview_dash_style(dash_array, transform);
    let pen_style = if dash_style.is_empty() {
        PS_SOLID
    } else if preview_builtin_dash_pattern(&dash_style) {
        dash_style.clear();
        PS_DASH
    } else {
        if dash_style.len() % 2 == 1 {
            dash_style.extend_from_within(..);
        }
        dash_style.truncate(16);
        PS_USERSTYLE
    };
    let brush = LOGBRUSH {
        lbStyle: BS_SOLID,
        lbColor: color,
        lbHatch: 0,
    };
    let pen = ExtCreatePen(
        preview_pen_style(line_cap, line_join, pen_style),
        width.max(1) as u32,
        &brush,
        dash_style.len() as u32,
        if dash_style.is_empty() {
            null()
        } else {
            dash_style.as_ptr()
        },
    );
    if pen.is_null() {
        CreatePen(PS_SOLID, width.max(1), color) as HGDIOBJ
    } else {
        pen as HGDIOBJ
    }
}

unsafe fn set_preview_miter_limit(dc: HDC) {
    SetMiterLimit(dc, PREVIEW_MITER_LIMIT, null_mut());
}

unsafe fn delete_preview_pen(pen: HGDIOBJ) {
    if pen != GetStockObject(NULL_PEN) {
        DeleteObject(pen);
    }
}

unsafe fn draw_preview_line(
    dc: HDC,
    from: POINT,
    to: POINT,
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) {
    let points = [from, to];
    draw_preview_polyline_points(
        dc,
        &points,
        color,
        stroke_width,
        line_cap,
        line_join,
        transform,
        dash_array,
    );
}

unsafe fn draw_preview_polyline(
    dc: HDC,
    points: &[CorePoint],
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) {
    if points.len() < 2 {
        return;
    }
    let mapped: Vec<POINT> = points.iter().map(|point| transform.point(*point)).collect();
    draw_preview_polyline_points(
        dc,
        &mapped,
        color,
        stroke_width,
        line_cap,
        line_join,
        transform,
        dash_array,
    );
}

unsafe fn draw_preview_polyline_points(
    dc: HDC,
    points: &[POINT],
    color: &str,
    stroke_width: f64,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    transform: &PreviewTransform,
    dash_array: &[f64],
) {
    if points.len() < 2 {
        return;
    }
    let pen = create_preview_pen(
        colorref_from_css(color).unwrap_or(0x000000),
        transform.length(stroke_width),
        line_cap,
        line_join,
        dash_array,
        transform,
    );
    let old_pen = SelectObject(dc, pen as HGDIOBJ);
    set_preview_miter_limit(dc);
    Polyline(dc, points.as_ptr(), points.len() as i32);
    SelectObject(dc, old_pen);
    delete_preview_pen(pen);
}

#[derive(Debug, Clone, Copy)]
enum PreviewPathCommand {
    Move(CorePoint),
    Line(CorePoint),
    Cubic(CorePoint, CorePoint, CorePoint),
    Close,
}

unsafe fn draw_preview_svg_path(
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
                transform.length(stroke_width),
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

unsafe fn begin_preview_clip(
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

unsafe fn end_preview_clip(dc: HDC, saved: i32) {
    if saved != 0 {
        RestoreDC(dc, saved);
    }
}

unsafe fn apply_preview_clip_path(dc: HDC, d: &str, transform: &PreviewTransform) -> bool {
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

unsafe fn replay_preview_path(
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

fn preview_closed_linear_path_points(commands: &[PreviewPathCommand]) -> Option<Vec<CorePoint>> {
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

unsafe fn draw_preview_svg_polygon_path(
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
                transform.length(stroke_width),
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

unsafe fn draw_preview_svg_polyline_path(
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
        transform.length(stroke_width),
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

fn preview_cubic_is_line(start: CorePoint, c1: CorePoint, c2: CorePoint, end: CorePoint) -> bool {
    let length = start.distance(end);
    if length <= 0.01 {
        return c1.distance(start) <= 0.01 && c2.distance(end) <= 0.01;
    }
    point_line_distance(c1, start, end) <= 0.01 && point_line_distance(c2, start, end) <= 0.01
}

fn point_line_distance(point: CorePoint, start: CorePoint, end: CorePoint) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= 0.0 {
        return point.distance(start);
    }
    ((point.x - start.x) * dy - (point.y - start.y) * dx).abs() / length
}

fn parse_preview_path(d: &str) -> Option<Vec<PreviewPathCommand>> {
    let mut parser = PreviewPathParser::new(d);
    parser.parse()
}

struct PreviewPathParser<'a> {
    input: &'a [u8],
    index: usize,
    command: Option<char>,
    current: CorePoint,
    start: CorePoint,
}

impl<'a> PreviewPathParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            index: 0,
            command: None,
            current: CorePoint { x: 0.0, y: 0.0 },
            start: CorePoint { x: 0.0, y: 0.0 },
        }
    }

    fn parse(&mut self) -> Option<Vec<PreviewPathCommand>> {
        let mut out = Vec::new();
        while self.skip_separators() {
            if let Some(command) = self.peek_command() {
                self.index += 1;
                self.command = Some(command);
            }
            let command = self.command?;
            match command {
                'M' | 'm' => {
                    let relative = command == 'm';
                    let mut first = true;
                    while let Some(point) = self.read_point(relative) {
                        if first {
                            out.push(PreviewPathCommand::Move(point));
                            self.start = point;
                            self.command = Some(if relative { 'l' } else { 'L' });
                            first = false;
                        } else {
                            out.push(PreviewPathCommand::Line(point));
                        }
                        self.current = point;
                        if self.next_is_command_or_end() {
                            break;
                        }
                    }
                    if first {
                        return None;
                    }
                }
                'L' | 'l' => {
                    let relative = command == 'l';
                    let mut read_any = false;
                    while let Some(point) = self.read_point(relative) {
                        out.push(PreviewPathCommand::Line(point));
                        self.current = point;
                        read_any = true;
                        if self.next_is_command_or_end() {
                            break;
                        }
                    }
                    if !read_any {
                        return None;
                    }
                }
                'C' | 'c' => {
                    let relative = command == 'c';
                    let mut read_any = false;
                    loop {
                        let c1 = self.read_point(relative);
                        let c2 = self.read_point(relative);
                        let end = self.read_point(relative);
                        let (Some(c1), Some(c2), Some(end)) = (c1, c2, end) else {
                            break;
                        };
                        out.push(PreviewPathCommand::Cubic(c1, c2, end));
                        self.current = end;
                        read_any = true;
                        if self.next_is_command_or_end() {
                            break;
                        }
                    }
                    if !read_any {
                        return None;
                    }
                }
                'A' | 'a' => {
                    let relative = command == 'a';
                    loop {
                        let rx = self.read_number()?;
                        let ry = self.read_number()?;
                        let x_axis_rotation = self.read_number()?;
                        let large_arc = self.read_flag()?;
                        let sweep = self.read_flag()?;
                        let end = self.read_point(relative)?;
                        append_preview_arc_cubics(
                            &mut out,
                            self.current,
                            end,
                            rx,
                            ry,
                            x_axis_rotation,
                            large_arc,
                            sweep,
                        )?;
                        self.current = end;
                        if self.next_is_command_or_end() {
                            break;
                        }
                    }
                }
                'Z' | 'z' => {
                    out.push(PreviewPathCommand::Close);
                    self.current = self.start;
                    self.command = None;
                }
                _ => return None,
            }
        }
        Some(out)
    }

    fn read_point(&mut self, relative: bool) -> Option<CorePoint> {
        let x = self.read_number()?;
        let y = self.read_number()?;
        let point = if relative {
            CorePoint {
                x: self.current.x + x,
                y: self.current.y + y,
            }
        } else {
            CorePoint { x, y }
        };
        Some(point)
    }

    fn read_number(&mut self) -> Option<f64> {
        self.skip_separators();
        let start = self.index;
        if self.index < self.input.len() && matches!(self.input[self.index] as char, '+' | '-') {
            self.index += 1;
        }
        let mut saw_digit = false;
        while self.index < self.input.len() && (self.input[self.index] as char).is_ascii_digit() {
            saw_digit = true;
            self.index += 1;
        }
        if self.index < self.input.len() && self.input[self.index] == b'.' {
            self.index += 1;
            while self.index < self.input.len() && (self.input[self.index] as char).is_ascii_digit()
            {
                saw_digit = true;
                self.index += 1;
            }
        }
        if !saw_digit {
            self.index = start;
            return None;
        }
        if self.index < self.input.len() && matches!(self.input[self.index] as char, 'e' | 'E') {
            let exp = self.index;
            self.index += 1;
            if self.index < self.input.len() && matches!(self.input[self.index] as char, '+' | '-')
            {
                self.index += 1;
            }
            let exp_digits = self.index;
            while self.index < self.input.len() && (self.input[self.index] as char).is_ascii_digit()
            {
                self.index += 1;
            }
            if self.index == exp_digits {
                self.index = exp;
            }
        }
        std::str::from_utf8(&self.input[start..self.index])
            .ok()?
            .parse()
            .ok()
    }

    fn read_flag(&mut self) -> Option<bool> {
        self.skip_separators();
        if self.index >= self.input.len() {
            return None;
        }
        match self.input[self.index] {
            b'0' => {
                self.index += 1;
                Some(false)
            }
            b'1' => {
                self.index += 1;
                Some(true)
            }
            _ => None,
        }
    }

    fn skip_separators(&mut self) -> bool {
        while self.index < self.input.len() {
            let ch = self.input[self.index] as char;
            if ch.is_ascii_whitespace() || ch == ',' {
                self.index += 1;
            } else {
                break;
            }
        }
        self.index < self.input.len()
    }

    fn next_is_command_or_end(&mut self) -> bool {
        !self.skip_separators() || self.peek_command().is_some()
    }

    fn peek_command(&self) -> Option<char> {
        if self.index >= self.input.len() {
            return None;
        }
        let ch = self.input[self.index] as char;
        if ch.is_ascii_alphabetic() {
            Some(ch)
        } else {
            None
        }
    }
}

fn append_preview_arc_cubics(
    out: &mut Vec<PreviewPathCommand>,
    start: CorePoint,
    end: CorePoint,
    rx: f64,
    ry: f64,
    x_axis_rotation: f64,
    large_arc: bool,
    sweep: bool,
) -> Option<()> {
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

fn is_oval_bounds_path(d: &str, points: &[CorePoint]) -> bool {
    points.len() == 2 && (d.contains(" A ") || d.contains(" C ")) && !d.contains(" L ")
}

unsafe fn draw_preview_oval_bounds(
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
        transform.length(stroke_width),
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

unsafe fn draw_preview_polygon(
    dc: HDC,
    role: RenderRole,
    points: &[CorePoint],
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) {
    if points.len() < 2 {
        return;
    }
    if role == RenderRole::DocumentBond {
        if let Some((start, end, width)) = preview_thin_polygon_centerline(points) {
            draw_preview_line(
                dc,
                transform.point(start),
                transform.point(end),
                fill,
                width.max(0.5),
                Some("round"),
                Some("miter"),
                transform,
                &[],
            );
            return;
        }
    }
    if role == RenderRole::DocumentBond && points.len() == 4 {
        draw_preview_polygon_centerline(dc, points, fill, transform);
        return;
    }
    let mapped: Vec<POINT> = points.iter().map(|point| transform.point(*point)).collect();
    let fill_color = colorref_from_css(fill);
    let brush = fill_color
        .map(|color| cache.solid_brush(color))
        .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
    let pen = create_preview_pen(
        colorref_from_css(stroke).unwrap_or_else(|| colorref_from_css(fill).unwrap_or(0x000000)),
        transform.length(stroke_width),
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
    if role == RenderRole::DocumentBond {
        draw_preview_polygon_centerline(dc, points, fill, transform);
    }
}

fn preview_thin_polygon_centerline(points: &[CorePoint]) -> Option<(CorePoint, CorePoint, f64)> {
    if points.len() < 4 {
        return None;
    }
    let mut best = (0usize, 1usize, 0.0);
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let distance = points[i].distance(points[j]);
            if distance > best.2 {
                best = (i, j, distance);
            }
        }
    }
    let length = best.2;
    if length <= 1.0 {
        return None;
    }
    let area = polygon_area(points).abs();
    let width = area / length;
    if !width.is_finite() || width <= 0.0 || width / length > 0.12 {
        return None;
    }

    let axis = CorePoint {
        x: (points[best.1].x - points[best.0].x) / length,
        y: (points[best.1].y - points[best.0].y) / length,
    };
    let projections: Vec<f64> = points
        .iter()
        .map(|point| point.x * axis.x + point.y * axis.y)
        .collect();
    let min_projection = projections.iter().copied().fold(f64::INFINITY, f64::min);
    let max_projection = projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let tolerance = width.max(length * 0.02);
    let start = average_projected_points(points, &projections, min_projection, tolerance)?;
    let end = average_projected_points(points, &projections, max_projection, tolerance)?;
    if start.distance(end) < 1.0 {
        None
    } else {
        Some((start, end, width))
    }
}

fn polygon_area(points: &[CorePoint]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

fn average_projected_points(
    points: &[CorePoint],
    projections: &[f64],
    target: f64,
    tolerance: f64,
) -> Option<CorePoint> {
    let mut sum = CorePoint { x: 0.0, y: 0.0 };
    let mut count = 0.0;
    for (point, projection) in points.iter().zip(projections) {
        if (*projection - target).abs() <= tolerance {
            sum.x += point.x;
            sum.y += point.y;
            count += 1.0;
        }
    }
    if count == 0.0 {
        None
    } else {
        Some(CorePoint {
            x: sum.x / count,
            y: sum.y / count,
        })
    }
}

unsafe fn draw_preview_polygon_centerline(
    dc: HDC,
    points: &[CorePoint],
    color: &str,
    transform: &PreviewTransform,
) {
    if points.len() != 4 {
        return;
    }
    let middle = points.len() / 2;
    if middle == 0 || middle >= points.len() {
        return;
    }
    let start = CorePoint {
        x: (points[0].x + points[points.len() - 1].x) * 0.5,
        y: (points[0].y + points[points.len() - 1].y) * 0.5,
    };
    let end = CorePoint {
        x: (points[middle - 1].x + points[middle].x) * 0.5,
        y: (points[middle - 1].y + points[middle].y) * 0.5,
    };
    let width = points[0].distance(points[points.len() - 1]);
    draw_preview_line(
        dc,
        transform.point(start),
        transform.point(end),
        color,
        width.max(0.5),
        Some("round"),
        Some("miter"),
        transform,
        &[],
    );
}

fn colorref_from_css(value: &str) -> Option<COLORREF> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        let rgb = u32::from_str_radix(hex, 16).ok()?;
        let r = (rgb >> 16) & 0xff;
        let g = (rgb >> 8) & 0xff;
        let b = rgb & 0xff;
        return Some((b << 16) | (g << 8) | r);
    }
    if let Some((r, g, b, alpha)) = parse_css_rgba(value) {
        if alpha <= 0.0 {
            return None;
        }
        let r = composite_css_channel_on_white(r, alpha);
        let g = composite_css_channel_on_white(g, alpha);
        let b = composite_css_channel_on_white(b, alpha);
        return Some((b << 16) | (g << 8) | r);
    }
    None
}

fn parse_css_rgba(value: &str) -> Option<(u32, u32, u32, f64)> {
    let inner = value
        .strip_prefix("rgba(")
        .and_then(|rest| rest.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix("rgb(")
                .and_then(|rest| rest.strip_suffix(')'))
        })?;
    let parts: Vec<&str> = inner
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let r = parse_css_channel(parts[0])?;
    let g = parse_css_channel(parts[1])?;
    let b = parse_css_channel(parts[2])?;
    let alpha = if parts.len() == 4 {
        parts[3].parse::<f64>().ok()?.clamp(0.0, 1.0)
    } else {
        1.0
    };
    Some((r, g, b, alpha))
}

fn parse_css_channel(value: &str) -> Option<u32> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.parse::<f64>().ok()?.clamp(0.0, 100.0);
        Some((percent * 255.0 / 100.0).round() as u32)
    } else {
        let channel = value.parse::<f64>().ok()?.clamp(0.0, 255.0);
        Some(channel.round() as u32)
    }
}

fn composite_css_channel_on_white(channel: u32, alpha: f64) -> u32 {
    ((channel as f64 * alpha) + 255.0 * (1.0 - alpha))
        .round()
        .clamp(0.0, 255.0) as u32
}

pub(super) unsafe fn draw_placeholder_preview(dc: HDC, bounds: &RECT) {
    let width = (bounds.right - bounds.left).max(1);
    let height = (bounds.bottom - bounds.top).max(1);
    let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let pen = CreatePen(PS_SOLID, (width.min(height) / 120).clamp(1, 16), 0x000000);
    let old_pen = SelectObject(dc, pen as HGDIOBJ);

    let mid_y = bounds.top + height * 58 / 100;
    let left_x = bounds.left + width * 24 / 100;
    let right_x = bounds.left + width * 76 / 100;
    MoveToEx(dc, left_x, mid_y, null_mut());
    LineTo(dc, right_x, mid_y);
    let radius = (width.min(height) / 20).max(3);
    Ellipse(
        dc,
        left_x - radius,
        mid_y - radius,
        left_x + radius,
        mid_y + radius,
    );
    Ellipse(
        dc,
        right_x - radius,
        mid_y - radius,
        right_x + radius,
        mid_y + radius,
    );

    SetBkMode(dc, TRANSPARENT as i32);
    let label = ansi_metafile_text_bytes(DOCUMENT_DISPLAY_NAME);
    TextOutA(
        dc,
        bounds.left + width * 30 / 100,
        bounds.top + height * 18 / 100,
        label.as_ptr(),
        label.len() as i32,
    );

    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen as HGDIOBJ);
}
