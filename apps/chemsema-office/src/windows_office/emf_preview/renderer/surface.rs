use super::*;

pub(super) unsafe fn draw_payload_vector_preview_internal(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
    high_resolution_vectors: bool,
) -> bool {
    let primitives = if let Some(primitives) = payload_render_primitives(payload) {
        primitives
    } else {
        let Ok(document) = parse_document_json(&payload.chemsema_document_json) else {
            return false;
        };
        render_document(&document)
    };
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
    let bond_context = preview_bond_context(payload);
    let label_context = preview_label_context(payload);
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
                draw_preview_primitive(
                    dc,
                    primitive,
                    &vector_transform,
                    &mut cache,
                    bond_context.as_ref(),
                    label_context.as_ref(),
                );
                continue;
            }
        } else if vector_scope != 0 {
            RestoreDC(dc, vector_scope);
            vector_scope = 0;
        }
        draw_preview_primitive(
            dc,
            primitive,
            &transform,
            &mut cache,
            bond_context.as_ref(),
            label_context.as_ref(),
        );
    }
    if vector_scope != 0 {
        RestoreDC(dc, vector_scope);
    }
    cache.delete_objects();
    true
}

pub(super) unsafe fn begin_high_resolution_vector_scope(dc: HDC, record_scale: f64) -> i32 {
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

pub(super) fn preview_primitive_record_scale(primitive: &RenderPrimitive) -> f64 {
    match primitive {
        RenderPrimitive::Text { .. } | RenderPrimitive::Image { .. } => 1.0,
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

pub(super) fn render_svg_preview_bitmap(svg: &str) -> Option<SvgPreviewBitmap> {
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

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw_preview_image(
    dc: HDC,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    href: &str,
    opacity: f64,
    preserve_aspect_ratio: bool,
    rotate: f64,
    rotate_center: Option<CorePoint>,
    transform: &PreviewTransform,
) -> bool {
    if !href.starts_with("data:image/")
        || width.abs() <= f64::EPSILON
        || height.abs() <= f64::EPSILON
    {
        return false;
    }
    let center = rotate_center.unwrap_or(CorePoint {
        x: x + width * 0.5,
        y: y + height * 0.5,
    });
    let radians = rotate.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let rotate_point = |point: CorePoint| {
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        CorePoint {
            x: center.x + dx * cos - dy * sin,
            y: center.y + dx * sin + dy * cos,
        }
    };
    let corners = [
        rotate_point(CorePoint { x, y }),
        rotate_point(CorePoint { x: x + width, y }),
        rotate_point(CorePoint {
            x: x + width,
            y: y + height,
        }),
        rotate_point(CorePoint { x, y: y + height }),
    ];
    let min_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let bounds_width = (max_x - min_x).max(0.01);
    let bounds_height = (max_y - min_y).max(0.01);
    let preserve = if preserve_aspect_ratio {
        "xMidYMid meet"
    } else {
        "none"
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{bounds_width}" height="{bounds_height}" viewBox="{min_x} {min_y} {bounds_width} {bounds_height}"><image x="{x}" y="{y}" width="{width}" height="{height}" href="{href}" opacity="{}" preserveAspectRatio="{preserve}" transform="rotate({rotate} {} {})"/></svg>"#,
        opacity.clamp(0.0, 1.0),
        center.x,
        center.y,
    );
    let Some(bitmap) = render_svg_preview_bitmap(&svg) else {
        return false;
    };
    let top_left = transform.xy(min_x, min_y);
    let bottom_right = transform.xy(max_x, max_y);
    let info = BITMAPINFO {
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
        bmiColors: zeroed(),
    };
    StretchDIBits(
        dc,
        top_left.x,
        top_left.y,
        (bottom_right.x - top_left.x).max(1),
        (bottom_right.y - top_left.y).max(1),
        0,
        0,
        bitmap.width,
        bitmap.height,
        bitmap.bgra.as_ptr().cast::<c_void>(),
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    ) != 0
}

pub(super) unsafe fn draw_svg_preview(dc: HDC, bounds: &RECT, payload: &OleObjectPayload) -> bool {
    let Some(bitmap) = render_svg_preview_bitmap(&payload.svg) else {
        return false;
    };
    let target_width = (bounds.right - bounds.left).max(1);
    let target_height = (bounds.bottom - bounds.top).max(1);
    let info = BITMAPINFO {
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
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    lines != 0
}

pub(super) fn office_preview_primitive_visible_impl(primitive: &RenderPrimitive) -> bool {
    if preview_is_invalid_marker_primitive(primitive)
        && !preview_env_enabled(ENV_SHOW_INVALID_MARKERS)
    {
        return false;
    }
    if let Some(allow_ids) = preview_env_object_id_filter() {
        let Some(object_id) = primitive.object_id() else {
            return false;
        };
        if !allow_ids.contains(object_id) {
            return false;
        }
    }
    if let Some(allow_node_ids) = preview_env_node_id_filter() {
        let Some(node_id) = preview_primitive_node_id(primitive) else {
            return false;
        };
        if !allow_node_ids.contains(node_id) {
            return false;
        }
    }
    let role = match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Image { role, .. }
        | RenderPrimitive::Text { role, .. } => role,
    };
    match role {
        RenderRole::DocumentDiagnostic => return preview_env_enabled(ENV_SHOW_INVALID_MARKERS),
        RenderRole::DocumentKnockout => {
            if preview_primitive_node_id(primitive).is_some() {
                return false;
            }
            if preview_env_enabled(ENV_HIDE_DOCUMENT_KNOCKOUT) {
                return false;
            }
        }
        RenderRole::DocumentText if preview_env_enabled(ENV_HIDE_DOCUMENT_TEXT) => return false,
        RenderRole::DocumentBond if preview_env_enabled(ENV_HIDE_DOCUMENT_BOND) => return false,
        RenderRole::DocumentGraphic if preview_env_enabled(ENV_HIDE_DOCUMENT_GRAPHIC) => {
            return false;
        }
        _ => {}
    }
    matches!(
        role,
        RenderRole::DocumentBond
            | RenderRole::DocumentDiagnostic
            | RenderRole::DocumentGraphic
            | RenderRole::DocumentKnockout
            | RenderRole::DocumentText
    )
}

unsafe fn draw_preview_filled_path(
    dc: HDC,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    bond_context: Option<&PreviewBondContext>,
) {
    let RenderPrimitive::FilledPath {
        d,
        points,
        fill,
        clip_path_d,
        clip_rule,
        role,
        bond_id,
        ..
    } = primitive
    else {
        return;
    };
    if *role == RenderRole::DocumentBond {
        let stroke_line = if transform.office_presentation {
            preview_office_hashed_wedge_stroke_line(points, bond_id.as_deref(), bond_context)
        } else {
            preview_hashed_wedge_stroke_line(points, bond_id.as_deref(), bond_context)
        };
        if let Some(stroke_line) = stroke_line {
            draw_preview_polyline(
                dc,
                &[stroke_line.start, stroke_line.end],
                fill,
                stroke_line.width,
                Some("round"),
                None,
                transform,
                &[],
            );
            return;
        }
    }
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
            None,
            points,
            fill,
            fill,
            0.0,
            transform,
            cache,
            None,
        );
    }
    end_preview_clip(dc, saved_clip);
}

unsafe fn draw_preview_basic_shape(
    dc: HDC,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) {
    let (bounds, fill, stroke, stroke_width, dash_array, line_cap, line_join) = match primitive {
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
        } => (
            (
                transform.xy(*x, *y),
                transform.xy(*x + *width, *y + *height),
            ),
            fill.as_deref(),
            stroke.as_deref(),
            *stroke_width,
            dash_array.as_slice(),
            "butt",
            "miter",
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
        } => {
            let center = transform.point(*center);
            let rx = transform.length(*rx);
            let ry = transform.length(*ry);
            (
                (
                    POINT {
                        x: center.x - rx,
                        y: center.y - ry,
                    },
                    POINT {
                        x: center.x + rx,
                        y: center.y + ry,
                    },
                ),
                fill.as_deref(),
                stroke.as_deref(),
                *stroke_width,
                dash_array.as_slice(),
                "round",
                "round",
            )
        }
        RenderPrimitive::Circle {
            center,
            radius,
            fill,
            stroke,
            stroke_width,
            ..
        } => {
            let center = transform.point(*center);
            let radius = transform.length(*radius);
            (
                (
                    POINT {
                        x: center.x - radius,
                        y: center.y - radius,
                    },
                    POINT {
                        x: center.x + radius,
                        y: center.y + radius,
                    },
                ),
                Some(fill.as_str()),
                Some(stroke.as_str()),
                *stroke_width,
                &[] as &[f64],
                "round",
                "round",
            )
        }
        _ => return,
    };
    let brush = fill
        .and_then(colorref_from_css)
        .map(|color| cache.solid_brush(color))
        .unwrap_or_else(|| GetStockObject(NULL_BRUSH));
    let pen = stroke
        .and_then(colorref_from_css)
        .map(|color| {
            create_preview_pen(
                color,
                transform.pen_width(stroke_width),
                Some(line_cap),
                Some(line_join),
                dash_array,
                transform,
            )
        })
        .unwrap_or_else(|| GetStockObject(NULL_PEN));
    let old_brush = SelectObject(dc, brush as HGDIOBJ);
    let old_pen = SelectObject(dc, pen);
    set_preview_miter_limit(dc);
    match primitive {
        RenderPrimitive::Rect { .. } => {
            Rectangle(dc, bounds.0.x, bounds.0.y, bounds.1.x, bounds.1.y)
        }
        _ => Ellipse(dc, bounds.0.x, bounds.0.y, bounds.1.x, bounds.1.y),
    };
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    delete_preview_pen(pen);
}

pub(super) unsafe fn draw_preview_primitive(
    dc: HDC,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    bond_context: Option<&PreviewBondContext>,
    label_context: Option<&PreviewLabelContext>,
) {
    match primitive {
        RenderPrimitive::Line {
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
            Some("butt"),
            Some("miter"),
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
        } => draw_preview_polygon(
            dc,
            *role,
            bond_id.as_deref(),
            points,
            fill,
            stroke,
            *stroke_width,
            transform,
            cache,
            bond_context,
        ),
        RenderPrimitive::FilledPath { .. } => {
            draw_preview_filled_path(dc, primitive, transform, cache, bond_context)
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
        RenderPrimitive::Rect { .. }
        | RenderPrimitive::Ellipse { .. }
        | RenderPrimitive::Circle { .. } => {
            draw_preview_basic_shape(dc, primitive, transform, cache)
        }
        RenderPrimitive::Image {
            x,
            y,
            width,
            height,
            href,
            opacity,
            preserve_aspect_ratio,
            rotate,
            rotate_center,
            ..
        } => {
            let _ = draw_preview_image(
                dc,
                *x,
                *y,
                *width,
                *height,
                href,
                *opacity,
                *preserve_aspect_ratio,
                *rotate,
                *rotate_center,
                transform,
            );
        }
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
        } => {
            draw_preview_text(
                dc,
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
                cache,
                node_id.as_deref(),
                label_context,
            );
        }
    }
}
