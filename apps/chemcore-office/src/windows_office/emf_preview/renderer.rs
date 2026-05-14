// GDI replay for Chemcore document preview primitives.
//
// Keep Office/OLE container decisions out of this file. Code here should be
// about geometry, pens, brushes, text metrics, path replay, clipping, and the
// ChemDraw-style EMF record strategy.

use super::*;
use chemcore_engine::{
    Bond, BondLinePattern, BondLineWeight, ChemcoreDocument, DoubleBondPlacement, MoleculeFragment,
    SceneObject,
};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use windows_sys::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, HENHMETAFILE};
use windows_sys::Win32::Graphics::GdiPlus::{
    DashStyleDash, EmfTypeEmfPlusDual, FillModeAlternate, FontStyleBold, FontStyleItalic,
    FontStyleRegular, FontStyleUnderline, GdipAddPathBezier, GdipAddPathLine,
    GdipCloneStringFormat, GdipClosePathFigure, GdipCreateFont, GdipCreateFontFamilyFromName,
    GdipCreatePath, GdipCreatePen1, GdipCreateSolidFill, GdipCreateStringFormat, GdipDeleteBrush,
    GdipDeleteFont, GdipDeleteFontFamily, GdipDeleteGraphics, GdipDeletePath, GdipDeletePen,
    GdipDeleteStringFormat, GdipDisposeImage, GdipDrawEllipse, GdipDrawLine, GdipDrawLines,
    GdipDrawPath, GdipDrawPolygon, GdipDrawRectangle, GdipDrawString, GdipFillEllipse,
    GdipFillPath, GdipFillPolygon, GdipFillRectangle, GdipGetDC, GdipGetHemfFromMetafile,
    GdipGetImageGraphicsContext, GdipMeasureString, GdipRecordMetafile, GdipReleaseDC,
    GdipSetPenDashArray, GdipSetPenDashStyle, GdipSetPenEndCap, GdipSetPenLineJoin,
    GdipSetPenMiterLimit, GdipSetPenStartCap, GdipSetSmoothingMode, GdipSetStringFormatAlign,
    GdipSetStringFormatFlags, GdipSetStringFormatLineAlign, GdipSetTextRenderingHint,
    GdipStartPathFigure, GdipStringFormatGetGenericTypographic, GdipSaveGraphics,
    GdipRestoreGraphics, GdiplusStartup,
    GdiplusStartupInput, GpBrush, GpFont, GpFontFamily, GpGraphics, GpImage, GpMetafile, GpPath,
    GpPen, GpStringFormat, LineCapFlat, LineCapRound, LineCapSquare, LineJoinBevel, LineJoinMiter,
    LineJoinRound, MetafileFrameUnitGdi, Ok as GDI_PLUS_OK, PointF, RectF, SmoothingModeAntiAlias,
    StringAlignmentNear, StringFormatFlagsMeasureTrailingSpaces, StringFormatFlagsNoClip,
    StringFormatFlagsNoFitBlackBox, StringFormatFlagsNoWrap, TextRenderingHintAntiAliasGridFit,
    UnitPixel,
};

const EMF_VECTOR_RECORD_SCALE: f64 = 16.0;
const EMF_ARROW_RECORD_SCALE: f64 = EMF_VECTOR_RECORD_SCALE;
const USE_GDIPLUS_TEXT_PREVIEW: bool = true;
const CHEMDRAW_SCRIPT_SCALE: f64 = 0.75;
const CHEMDRAW_SUBSCRIPT_SHIFT_DOWN_EM: f64 = 0.22;
const CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM: f64 = 0.215;
const CHEMDRAW_SUPERSCRIPT_SHIFT_UP_EM: f64 = 0.392;

#[derive(Clone, Copy)]
struct PreviewTransform {
    min_x: f64,
    min_y: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    record_scale: f64,
    emf_recording: bool,
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
            emf_recording: false,
        })
    }

    fn with_record_scale(self, record_scale: f64) -> Self {
        Self {
            record_scale: record_scale.max(1.0),
            ..self
        }
    }

    fn for_emf_recording(self) -> Self {
        Self {
            emf_recording: true,
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

    fn gdip_point(&self, point: CorePoint) -> PointF {
        PointF {
            X: (self.offset_x + (point.x - self.min_x) * self.scale) as f32,
            Y: (self.offset_y + (point.y - self.min_y) * self.scale) as f32,
        }
    }

    fn length(&self, value: f64) -> i32 {
        (value.abs() * self.scale * self.record_scale)
            .round()
            .max(1.0) as i32
    }

    fn gdip_length(&self, value: f64) -> f32 {
        if self.emf_recording {
            value.abs().max(0.01) as f32
        } else {
            (value.abs() * self.scale).max(0.01) as f32
        }
    }

    fn pen_width(&self, value: f64) -> i32 {
        self.length(value)
    }
}

pub(super) unsafe fn draw_payload_preview(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
) -> bool {
    if draw_payload_vector_preview_internal(dc, bounds, payload, None, true) {
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

pub(super) unsafe fn enhanced_metafile_gdiplus_dual_preview(
    frame_bounds: &RECT,
    draw_bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
) -> Option<HENHMETAFILE> {
    if !ensure_gdiplus_started() {
        return None;
    }
    let primitives = if let Some(primitives) = payload_render_primitives(payload) {
        primitives
    } else {
        let Ok(document) = parse_document_json(&payload.chemcore_document_json) else {
            return None;
        };
        render_document(&document)
    };
    let visible: Vec<_> = primitives
        .iter()
        .filter(|primitive| office_preview_primitive_visible(primitive))
        .collect();
    let Some(primitive_bounds) = render_primitives_bounds(visible.iter().copied()) else {
        return None;
    };
    let Some(transform) =
        PreviewTransform::from_bounds(draw_bounds, source_bounds.unwrap_or(primitive_bounds))
    else {
        return None;
    };
    let transform = transform.for_emf_recording();
    let ref_dc = CreateCompatibleDC(null_mut());
    if ref_dc.is_null() {
        return None;
    }
    let frame = RectF {
        X: frame_bounds.left as f32,
        Y: frame_bounds.top as f32,
        Width: (frame_bounds.right - frame_bounds.left).max(1) as f32,
        Height: (frame_bounds.bottom - frame_bounds.top).max(1) as f32,
    };
    let mut metafile: *mut GpMetafile = null_mut();
    let record_status = GdipRecordMetafile(
        ref_dc,
        EmfTypeEmfPlusDual,
        &frame,
        MetafileFrameUnitGdi,
        null(),
        &mut metafile,
    );
    DeleteDC(ref_dc);
    if record_status != GDI_PLUS_OK || metafile.is_null() {
        return None;
    }
    let mut graphics = null_mut();
    if GdipGetImageGraphicsContext(metafile as *mut GpImage, &mut graphics) != GDI_PLUS_OK
        || graphics.is_null()
    {
        GdipDisposeImage(metafile as *mut GpImage);
        return None;
    }
    GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
    GdipSetTextRenderingHint(graphics, TextRenderingHintAntiAliasGridFit);
    let use_gdiplus_text = gdiplus_text_preview_enabled() && !transform.emf_recording;
    let bond_context = preview_bond_context(payload);
    let mut gdi_cache = PreviewGdiCache::default();
    let mut ok = true;
    for primitive in visible {
        if matches!(primitive, RenderPrimitive::Text { .. }) {
            let drawn = use_gdiplus_text
                && draw_gdiplus_primitive(graphics, primitive, &transform, bond_context.as_ref());
            if !drawn
                && !draw_gdi_primitive_in_gdiplus(
                    graphics,
                    primitive,
                    &transform,
                    &mut gdi_cache,
                    bond_context.as_ref(),
                )
            {
                ok = false;
                break;
            }
        } else if !draw_gdiplus_primitive(graphics, primitive, &transform, bond_context.as_ref()) {
            if !draw_gdi_primitive_in_gdiplus(
                graphics,
                primitive,
                &transform,
                &mut gdi_cache,
                bond_context.as_ref(),
            ) {
                ok = false;
                break;
            }
        }
    }
    gdi_cache.delete_objects();
    GdipDeleteGraphics(graphics);
    if !ok {
        GdipDisposeImage(metafile as *mut GpImage);
        return None;
    }
    let mut hemf = null_mut();
    if GdipGetHemfFromMetafile(metafile, &mut hemf) != GDI_PLUS_OK || hemf.is_null() {
        GdipDisposeImage(metafile as *mut GpImage);
        return None;
    }
    GdipDisposeImage(metafile as *mut GpImage);
    Some(hemf)
}

fn gdiplus_text_preview_enabled() -> bool {
    (USE_GDIPLUS_TEXT_PREVIEW || std::env::var_os("CHEMCORE_OFFICE_GDIPLUS_TEXT").is_some())
        && std::env::var_os("CHEMCORE_OFFICE_DISABLE_GDIPLUS_TEXT").is_none()
}

unsafe fn draw_gdi_primitive_in_gdiplus(
    graphics: *mut GpGraphics,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    bond_context: Option<&PreviewBondContext>,
) -> bool {
    let mut dc: HDC = null_mut();
    if GdipGetDC(graphics, &mut dc) != GDI_PLUS_OK || dc.is_null() {
        return false;
    }
    draw_preview_primitive(dc, primitive, transform, cache, bond_context);
    GdipReleaseDC(graphics, dc) == GDI_PLUS_OK
}

fn ensure_gdiplus_started() -> bool {
    static GDIPLUS_TOKEN: OnceLock<Option<usize>> = OnceLock::new();
    GDIPLUS_TOKEN
        .get_or_init(|| unsafe {
            let mut token = 0usize;
            let input = GdiplusStartupInput {
                GdiplusVersion: 1,
                DebugEventCallback: 0,
                SuppressBackgroundThread: 0,
                SuppressExternalCodecs: 0,
            };
            if GdiplusStartup(&mut token, &input, null_mut()) == GDI_PLUS_OK {
                Some(token)
            } else {
                None
            }
        })
        .is_some()
}

unsafe fn draw_payload_vector_preview_internal(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
    high_resolution_vectors: bool,
) -> bool {
    let primitives = if let Some(primitives) = payload_render_primitives(payload) {
        primitives
    } else {
        let Ok(document) = parse_document_json(&payload.chemcore_document_json) else {
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
                );
                continue;
            }
        } else if vector_scope != 0 {
            RestoreDC(dc, vector_scope);
            vector_scope = 0;
        }
        draw_preview_primitive(dc, primitive, &transform, &mut cache, bond_context.as_ref());
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
        RenderRole::DocumentBond
            | RenderRole::DocumentGraphic
            | RenderRole::DocumentKnockout
            | RenderRole::DocumentText
    )
}

unsafe fn draw_preview_primitive(
    dc: HDC,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    bond_context: Option<&PreviewBondContext>,
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
                        transform.pen_width(*stroke_width),
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
                        transform.pen_width(*stroke_width),
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
                        transform.pen_width(*stroke_width),
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

unsafe fn draw_gdiplus_primitive(
    graphics: *mut GpGraphics,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    bond_context: Option<&PreviewBondContext>,
) -> bool {
    let save_restore = transform.emf_recording
        && matches!(
            primitive,
            RenderPrimitive::Line {
                ..
            } | RenderPrimitive::Polyline {
                ..
            }
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
        } => {
            let Some(pen) = create_gdiplus_pen(
                stroke,
                transform.gdip_length(*stroke_width),
                Some("butt"),
                Some("miter"),
                dash_array,
                transform,
            ) else {
                return false;
            };
            let p1 = transform.gdip_point(*from);
            let p2 = transform.gdip_point(*to);
            let ok = GdipDrawLine(graphics, pen, p1.X, p1.Y, p2.X, p2.Y) == GDI_PLUS_OK;
            GdipDeletePen(pen);
            ok
        }
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
            fill,
            clip_path_d,
            ..
        } => {
            if clip_path_d.is_some() {
                return false;
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
        } => draw_gdiplus_text(
            graphics,
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
        ),
    };
    if save_restore {
        let _ = GdipRestoreGraphics(graphics, state);
    }
    ok
}

unsafe fn draw_gdiplus_polyline(
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

unsafe fn draw_gdiplus_polygon(
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
        if let Some(stroke_line) = preview_bond_stroke_line(points, bond_id, bond_context) {
            let line_points = [stroke_line.start, stroke_line.end];
            return draw_gdiplus_polyline(
                graphics,
                &line_points,
                fill,
                stroke_line.width,
                Some("round"),
                Some("round"),
                transform,
                &[],
            );
        }
    }
    let mapped: Vec<PointF> = points.iter().map(|point| transform.gdip_point(*point)).collect();
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

unsafe fn draw_gdiplus_rect(
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

unsafe fn draw_gdiplus_ellipse(
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

unsafe fn draw_gdiplus_path(
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

unsafe fn create_gdiplus_path(
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

unsafe fn create_gdiplus_pen(
    color: &str,
    width: f32,
    line_cap: Option<&str>,
    line_join: Option<&str>,
    dash_array: &[f64],
    transform: &PreviewTransform,
) -> Option<*mut GpPen> {
    let mut pen = null_mut();
    if GdipCreatePen1(css_argb(color)?, width.max(0.01), UnitPixel, &mut pen) != GDI_PLUS_OK
        || pen.is_null()
    {
        return None;
    }
    let cap = gdiplus_line_cap(line_cap);
    GdipSetPenStartCap(pen, cap);
    GdipSetPenEndCap(pen, cap);
    GdipSetPenLineJoin(pen, gdiplus_line_join(line_join));
    GdipSetPenMiterLimit(pen, PREVIEW_MITER_LIMIT);
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

unsafe fn create_gdiplus_solid_brush(color: &str) -> Option<*mut GpBrush> {
    let mut brush = null_mut();
    if GdipCreateSolidFill(css_argb(color)?, &mut brush) == GDI_PLUS_OK && !brush.is_null() {
        Some(brush as *mut GpBrush)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_gdiplus_text(
    graphics: *mut GpGraphics,
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
) -> bool {
    let line_step_world = line_height.unwrap_or(font_size * 1.2).max(0.01);
    let lines = preview_text_lines(text, runs);
    let layouts = gdiplus_text_layout(graphics, &lines, font_size, font_family, transform);
    let mut ok = true;
    for (index, line_runs) in lines.iter().enumerate() {
        if line_runs.is_empty() {
            continue;
        }
        let origin = transform.gdip_point(CorePoint {
            x,
            y: y + index as f64 * line_step_world,
        });
        let Some(line_layout) = layouts.get(index) else {
            continue;
        };
        let width = line_layout.width;
        let mut cursor_x = match text_anchor {
            Some("middle") => origin.X - width / 2.0,
            Some("end") => origin.X - width,
            _ => origin.X,
        };
        for (run, run_layout) in line_runs.iter().zip(&line_layout.runs) {
            ok &= draw_gdiplus_text_run(
                graphics,
                cursor_x + run_layout.dx,
                origin.Y,
                run_layout.advance,
                run,
                font_size,
                font_family,
                fill,
                transform,
            );
            cursor_x += run_layout.dx + run_layout.advance;
        }
    }
    ok
}

struct GdiplusTextLineLayout {
    width: f32,
    runs: Vec<GdiplusTextRunLayout>,
}

struct GdiplusTextRunLayout {
    dx: f32,
    advance: f32,
}

unsafe fn gdiplus_text_layout(
    graphics: *mut GpGraphics,
    lines: &[Vec<PreviewTextRun>],
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    transform: &PreviewTransform,
) -> Vec<GdiplusTextLineLayout> {
    let dc = CreateCompatibleDC(null_mut());
    if dc.is_null() {
        return lines
            .iter()
            .map(|runs| GdiplusTextLineLayout {
                width: preview_line_width_f32(runs, fallback_font_size, transform),
                runs: runs
                    .iter()
                    .map(|run| GdiplusTextRunLayout {
                        dx: preview_script_dx_f32(run, fallback_font_size, transform),
                        advance: preview_text_run_advance_estimate_f32(
                            run,
                            fallback_font_size,
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
            let mut width = 0.0f32;
            let run_layouts = runs
                .iter()
                .map(|run| {
                    let dx = preview_script_dx_f32(run, fallback_font_size, transform);
                    let advance = gdiplus_text_run_advance(
                        graphics,
                        run,
                        fallback_font_size,
                        fallback_family,
                        transform,
                    )
                    .unwrap_or_else(|| {
                        preview_text_run_extent(
                            dc,
                            run,
                            fallback_font_size,
                            fallback_family,
                            transform,
                            &mut cache,
                        ) as f32
                    });
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

unsafe fn gdiplus_text_run_advance(
    graphics: *mut GpGraphics,
    run: &PreviewTextRun,
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<f32> {
    if run.text.is_empty() {
        return Some(0.0);
    }
    let font = create_gdiplus_font(run, fallback_font_size, fallback_family, transform)?;
    let Some(format) = create_gdiplus_string_format() else {
        GdipDeleteFont(font);
        return None;
    };
    let wide: Vec<u16> = run.text.encode_utf16().collect();
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_px = (run.font_size.unwrap_or(fallback_font_size) * script_scale * transform.scale)
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
    GdipDeleteStringFormat(format);
    GdipDeleteFont(font);
    ok.then_some(bounds.Width.max(0.0))
}

unsafe fn draw_gdiplus_text_run(
    graphics: *mut GpGraphics,
    x: f32,
    baseline_y: f32,
    advance: f32,
    run: &PreviewTextRun,
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    fallback_fill: Option<&str>,
    transform: &PreviewTransform,
) -> bool {
    if run.text.is_empty() {
        return true;
    }
    let Some(font) = create_gdiplus_font(run, fallback_font_size, fallback_family, transform)
    else {
        return false;
    };
    let fill = run.fill.as_deref().or(fallback_fill).unwrap_or("#000000");
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
    let font_px = (run.font_size.unwrap_or(fallback_font_size) * script_scale * transform.scale)
        .max(1.0) as f32;
    let top = baseline_y - (font_px * 0.86)
        + preview_script_baseline_shift_f32(run, fallback_font_size, transform);
    let rect = RectF {
        X: x,
        Y: top,
        Width: (advance * 1.8).max(font_px * 0.5),
        Height: (font_px * 1.45).max(1.0),
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

unsafe fn create_gdiplus_font(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    transform: &PreviewTransform,
) -> Option<*mut GpFont> {
    let family_name = run
        .font_family
        .as_deref()
        .or(fallback_family)
        .unwrap_or("Arial");
    let wide_family = wide_null(family_name);
    let mut family: *mut GpFontFamily = null_mut();
    if GdipCreateFontFamilyFromName(wide_family.as_ptr(), null_mut(), &mut family) != GDI_PLUS_OK
        || family.is_null()
    {
        return None;
    }
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
    let script_scale = preview_script_scale(run.script.as_deref());
    let em_size = (run.font_size.unwrap_or(fallback_font_size) * script_scale * transform.scale)
        .max(0.1) as f32;
    let mut font: *mut GpFont = null_mut();
    let ok = GdipCreateFont(family, em_size, style, UnitPixel, &mut font) == GDI_PLUS_OK
        && !font.is_null();
    GdipDeleteFontFamily(family);
    ok.then_some(font)
}

unsafe fn create_gdiplus_string_format() -> Option<*mut GpStringFormat> {
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
    GdipSetStringFormatFlags(
        format,
        StringFormatFlagsNoWrap
            | StringFormatFlagsNoClip
            | StringFormatFlagsNoFitBlackBox
            | StringFormatFlagsMeasureTrailingSpaces,
    );
    GdipSetStringFormatAlign(format, StringAlignmentNear);
    GdipSetStringFormatLineAlign(format, StringAlignmentNear);
    Some(format)
}

fn gdiplus_line_cap(line_cap: Option<&str>) -> i32 {
    match line_cap {
        Some("round") => LineCapRound,
        Some("square") => LineCapSquare,
        _ => LineCapFlat,
    }
}

fn gdiplus_line_join(line_join: Option<&str>) -> i32 {
    match line_join {
        Some("round") => LineJoinRound,
        Some("bevel") => LineJoinBevel,
        _ => LineJoinMiter,
    }
}

fn css_argb(value: &str) -> Option<u32> {
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
        let width =
            preview_line_width_measured(dc, line_runs, font_size, font_family, transform, cache);
        let mut cursor_x = match text_anchor {
            Some("middle") => origin.x - width / 2,
            Some("end") => origin.x - width,
            _ => origin.x,
        };
        for run in line_runs {
            let dx = preview_script_dx(run, font_size, transform);
            let advance = draw_preview_text_run(
                dc,
                cursor_x + dx,
                origin.y,
                run,
                font_size,
                font_family,
                transform,
                cache,
            );
            cursor_x += dx + advance;
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

unsafe fn preview_line_width_measured(
    dc: HDC,
    runs: &[PreviewTextRun],
    fallback_font_size: f64,
    fallback_family: Option<&str>,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
) -> i32 {
    runs.iter()
        .map(|run| {
            preview_text_run_extent(
                dc,
                run,
                fallback_font_size,
                fallback_family,
                transform,
                cache,
            )
        })
        .sum()
}

fn preview_line_width_f32(
    runs: &[PreviewTextRun],
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    runs.iter()
        .map(|run| preview_text_run_advance_estimate_f32(run, fallback_font_size, transform))
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
    let old_font = select_preview_font(dc, font);
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
    restore_preview_font(dc, old_font);
    advance
}

unsafe fn preview_text_run_extent(
    dc: HDC,
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
    let old_font = select_preview_font(dc, font);
    let advance = preview_text_extent(dc, &label)
        .unwrap_or_else(|| preview_text_run_advance_estimate(run, fallback_font_size, transform));
    restore_preview_font(dc, old_font);
    advance
}

unsafe fn select_preview_font(dc: HDC, font: HGDIOBJ) -> HGDIOBJ {
    if font.is_null() {
        null_mut()
    } else {
        SelectObject(dc, font as HGDIOBJ)
    }
}

unsafe fn restore_preview_font(dc: HDC, old_font: HGDIOBJ) {
    if !old_font.is_null() {
        SelectObject(dc, old_font);
    }
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

fn preview_text_run_advance_estimate_f32(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    let script_scale = preview_script_scale(run.script.as_deref());
    let font_size = run.font_size.unwrap_or(fallback_font_size) * script_scale;
    let world_width: f64 = run
        .text
        .chars()
        .map(|character| preview_char_advance_em(character) * font_size)
        .sum();
    (world_width * transform.scale).max(0.0) as f32
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
    (base_height as f64 * preview_script_baseline_shift_em(run)).round() as i32
}

fn preview_script_baseline_shift_f32(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    let base_height = run.font_size.unwrap_or(fallback_font_size) * transform.scale;
    (base_height * preview_script_baseline_shift_em(run)) as f32
}

fn preview_script_baseline_shift_em(run: &PreviewTextRun) -> f64 {
    match run.script.as_deref() {
        Some("subscript") if run.font_weight.unwrap_or(400) >= 600 => {
            CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM
        }
        Some("subscript") => CHEMDRAW_SUBSCRIPT_SHIFT_DOWN_EM,
        Some("superscript") => -CHEMDRAW_SUPERSCRIPT_SHIFT_UP_EM,
        _ => 0.0,
    }
}

fn preview_script_dx(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> i32 {
    preview_script_dx_f64(run, fallback_font_size, transform)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

fn preview_script_dx_f32(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> f32 {
    preview_script_dx_f64(run, fallback_font_size, transform) as f32
}

fn preview_script_dx_f64(
    run: &PreviewTextRun,
    fallback_font_size: f64,
    transform: &PreviewTransform,
) -> f64 {
    if run.script.as_deref() != Some("superscript") {
        return 0.0;
    }
    let font_size =
        run.font_size.unwrap_or(fallback_font_size) * preview_script_scale(run.script.as_deref());
    -0.02 * font_size * transform.scale * transform.record_scale
}

fn preview_script_scale(script: Option<&str>) -> f64 {
    match script {
        Some("subscript" | "superscript") => CHEMDRAW_SCRIPT_SCALE,
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
const PREVIEW_BOND_STROKE_MAX_ASPECT_RATIO: f64 = 0.24;
const PREVIEW_BOND_STROKE_MAX_WIDTH: f64 = 4.25;
const PREVIEW_BOND_STROKE_TOLERANCE_WIDTH_FACTOR: f64 = 0.45;
const PREVIEW_BOND_STROKE_COLLINEAR_TOLERANCE_WIDTH_FACTOR: f64 = 0.18;
const PREVIEW_BOND_STROKE_EDGE_WIDTH_MIN_RATIO: f64 = 0.55;
const PREVIEW_BOND_STROKE_EDGE_WIDTH_MAX_RATIO: f64 = 1.45;
const PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO: f64 = 0.25;
const PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE: f64 = 1.83;

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
        transform.pen_width(stroke_width),
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

unsafe fn draw_preview_polygon(
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
        if let Some(stroke_line) = preview_bond_stroke_line(points, bond_id, bond_context) {
            let line_points = [stroke_line.start, stroke_line.end];
            draw_preview_polyline(
                dc,
                &line_points,
                fill,
                stroke_line.width,
                Some("round"),
                Some("round"),
                transform,
                &[],
            );
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

#[derive(Debug, Clone, Copy)]
struct PreviewBondInfo {
    axis: CorePoint,
    allow_pen: bool,
    start_projection: f64,
    end_projection: f64,
    axis_normal_projection: f64,
    side_double: bool,
    start_has_label: bool,
    end_has_label: bool,
}

#[derive(Debug, Default)]
struct PreviewBondContext {
    infos: BTreeMap<String, PreviewBondInfo>,
}

fn preview_bond_context(payload: &OleObjectPayload) -> Option<PreviewBondContext> {
    let document = parse_document_json(&payload.chemcore_document_json).ok()?;
    Some(preview_bond_context_from_document(&document))
}

fn preview_bond_context_from_document(document: &ChemcoreDocument) -> PreviewBondContext {
    let mut infos = BTreeMap::new();
    for object in document
        .scene_objects()
        .into_iter()
        .filter(|object| object.visible && object.object_type == "molecule")
    {
        let Some(fragment) = preview_molecule_fragment(document, object) else {
            continue;
        };
        let node_map: BTreeMap<&str, &chemcore_engine::Node> = fragment
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let mut incident: BTreeMap<&str, Vec<&Bond>> = BTreeMap::new();
        for bond in &fragment.bonds {
            incident.entry(bond.begin.as_str()).or_default().push(bond);
            incident.entry(bond.end.as_str()).or_default().push(bond);
        }
        for bond in &fragment.bonds {
            let Some(begin) = node_map.get(bond.begin.as_str()).copied() else {
                continue;
            };
            let Some(end) = node_map.get(bond.end.as_str()).copied() else {
                continue;
            };
            let axis = preview_bond_axis_from_nodes(object, begin.point(), end.point());
            let Some(axis) = axis else {
                continue;
            };
            let begin_world = CorePoint {
                x: begin.position[0] + object.transform.translate[0],
                y: begin.position[1] + object.transform.translate[1],
            };
            let end_world = CorePoint {
                x: end.position[0] + object.transform.translate[0],
                y: end.position[1] + object.transform.translate[1],
            };
            let allow_pen = preview_bond_is_pen_family(bond);
            let start_has_label = begin
                .label
                .as_ref()
                .is_some_and(|label| label.has_visible_text());
            let end_has_label = end
                .label
                .as_ref()
                .is_some_and(|label| label.has_visible_text());
            infos.insert(
                bond.id.clone(),
                PreviewBondInfo {
                    axis,
                    allow_pen,
                    start_projection: begin_world.x * axis.x + begin_world.y * axis.y,
                    end_projection: end_world.x * axis.x + end_world.y * axis.y,
                    axis_normal_projection: begin_world.x * -axis.y + begin_world.y * axis.x,
                    side_double: preview_bond_is_side_double(bond),
                    start_has_label,
                    end_has_label,
                },
            );
        }
    }
    PreviewBondContext { infos }
}

fn preview_molecule_fragment<'a>(
    document: &'a ChemcoreDocument,
    object: &SceneObject,
) -> Option<&'a MoleculeFragment> {
    let resource_ref = object.payload.resource_ref.as_ref()?;
    document.resources.get(resource_ref)?.data.as_fragment()
}

fn preview_bond_axis_from_nodes(
    object: &SceneObject,
    begin: CorePoint,
    end: CorePoint,
) -> Option<CorePoint> {
    let start = CorePoint {
        x: begin.x + object.transform.translate[0],
        y: begin.y + object.transform.translate[1],
    };
    let finish = CorePoint {
        x: end.x + object.transform.translate[0],
        y: end.y + object.transform.translate[1],
    };
    preview_normalize_axis(CorePoint {
        x: finish.x - start.x,
        y: finish.y - start.y,
    })
}

fn preview_bond_is_pen_family(bond: &Bond) -> bool {
    if bond.stereo.is_some() || bond.line_styles.main != BondLinePattern::Solid {
        return false;
    }
    match bond.order {
        0 => false,
        1 => bond.line_weights.main == BondLineWeight::Normal,
        2 => {
            bond.line_weights.main == BondLineWeight::Normal
                && bond.line_styles.left == BondLinePattern::Solid
                && bond.line_styles.right == BondLinePattern::Solid
                && bond.line_weights.left == BondLineWeight::Normal
                && bond.line_weights.right == BondLineWeight::Normal
        }
        _ => {
            bond.line_weights.main == BondLineWeight::Normal
                && bond.line_styles.left == BondLinePattern::Solid
                && bond.line_styles.right == BondLinePattern::Solid
                && bond.line_weights.left == BondLineWeight::Normal
                && bond.line_weights.right == BondLineWeight::Normal
        }
    }
}

fn preview_bond_is_side_double(bond: &Bond) -> bool {
    matches!(
        bond.double.as_ref().map(|double| double.placement),
        Some(DoubleBondPlacement::Left | DoubleBondPlacement::Right)
    )
}

fn preview_normalize_axis(axis: CorePoint) -> Option<CorePoint> {
    let length = axis.distance(CorePoint { x: 0.0, y: 0.0 });
    if length <= 1.0e-9 {
        return None;
    }
    Some(CorePoint {
        x: axis.x / length,
        y: axis.y / length,
    })
}

#[derive(Debug, Clone, Copy)]
struct PreviewBondStrokeLine {
    start: CorePoint,
    end: CorePoint,
    width: f64,
}

#[derive(Debug, Clone, Copy)]
struct PreviewBondTerminalEdge {
    center: CorePoint,
    length: f64,
}

fn preview_bond_stroke_line(
    points: &[CorePoint],
    bond_id: Option<&str>,
    bond_context: Option<&PreviewBondContext>,
) -> Option<PreviewBondStrokeLine> {
    if points.len() < 4 || points.len() > 6 {
        return None;
    }
    let bond_info = bond_id.and_then(|id| bond_context.and_then(|context| context.infos.get(id)));
    if bond_info.is_some_and(|info| !info.allow_pen) {
        return None;
    }
    let preferred_axis = bond_info.map(|info| info.axis);
    let axis = preferred_axis.or_else(|| preview_polygon_principal_axis(points))?;
    let normal = CorePoint {
        x: -axis.y,
        y: axis.x,
    };
    let projections: Vec<f64> = points
        .iter()
        .map(|point| point.x * axis.x + point.y * axis.y)
        .collect();
    let normal_projections: Vec<f64> = points
        .iter()
        .map(|point| point.x * normal.x + point.y * normal.y)
        .collect();
    let min_projection = projections.iter().copied().fold(f64::INFINITY, f64::min);
    let max_projection = projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let length = max_projection - min_projection;
    let width = normal_projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        - normal_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    if !length.is_finite()
        || !width.is_finite()
        || length <= 0.0
        || width <= 0.0
        || width > PREVIEW_BOND_STROKE_MAX_WIDTH
        || width / length > PREVIEW_BOND_STROKE_MAX_ASPECT_RATIO
    {
        return None;
    }
    let simplified =
        preview_simplify_bond_polygon(points, axis, width * PREVIEW_BOND_STROKE_COLLINEAR_TOLERANCE_WIDTH_FACTOR)?;
    if simplified.len() < 4 || simplified.len() > 6 {
        return None;
    }
    let axis = preferred_axis.or_else(|| preview_polygon_principal_axis(&simplified))?;
    let normal = CorePoint {
        x: -axis.y,
        y: axis.x,
    };
    let projections: Vec<f64> = simplified
        .iter()
        .map(|point| point.x * axis.x + point.y * axis.y)
        .collect();
    let normal_projections: Vec<f64> = simplified
        .iter()
        .map(|point| point.x * normal.x + point.y * normal.y)
        .collect();
    let min_projection = projections.iter().copied().fold(f64::INFINITY, f64::min);
    let max_projection = projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let length = max_projection - min_projection;
    let width = normal_projections
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        - normal_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
    if !length.is_finite()
        || !width.is_finite()
        || length <= 0.0
        || width <= 0.0
        || width > PREVIEW_BOND_STROKE_MAX_WIDTH
        || width / length > PREVIEW_BOND_STROKE_MAX_ASPECT_RATIO
    {
        return None;
    }
    if preferred_axis.is_some() {
        let stroke_width = width * PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE;
        if !stroke_width.is_finite() || stroke_width <= 0.0 {
            return None;
        }
        let min_normal = normal_projections
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_normal = normal_projections
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let normal_mid = (min_normal + max_normal) * 0.5;
        let tolerance = (width * PREVIEW_BOND_STROKE_TOLERANCE_WIDTH_FACTOR)
            .max(length * 0.01)
            .max(0.05);
        let start_edge =
            preview_bond_terminal_edge(&simplified, &projections, axis, min_projection, tolerance, width);
        let end_edge =
            preview_bond_terminal_edge(&simplified, &projections, axis, max_projection, tolerance, width);
        let mut start_axis_projection = bond_info
            .map(|info| info.start_projection)
            .unwrap_or(min_projection);
        let mut end_axis_projection = bond_info
            .map(|info| info.end_projection)
            .unwrap_or(max_projection);
        if let Some(info) = bond_info {
            if info.start_has_label {
                if let Some(edge) = start_edge {
                    start_axis_projection = edge.center.x * axis.x + edge.center.y * axis.y;
                }
            }
            if info.end_has_label {
                if let Some(edge) = end_edge {
                    end_axis_projection = edge.center.x * axis.x + edge.center.y * axis.y;
                }
            }
            let cap_radius = stroke_width * 0.5;
            if info.side_double
                && (normal_mid - info.axis_normal_projection).abs() > width * 0.35
            {
                start_axis_projection = min_projection + cap_radius;
                end_axis_projection = max_projection - cap_radius;
            }
        }
        return Some(PreviewBondStrokeLine {
            start: preview_point_from_axis_coordinates(
                axis,
                normal,
                start_axis_projection,
                normal_mid,
            ),
            end: preview_point_from_axis_coordinates(axis, normal, end_axis_projection, normal_mid),
            width: stroke_width,
        });
    }
    let tolerance = (width * PREVIEW_BOND_STROKE_TOLERANCE_WIDTH_FACTOR)
        .max(length * 0.01)
        .max(0.05);
    let start_edge =
        preview_bond_terminal_edge(&simplified, &projections, axis, min_projection, tolerance, width)?;
    let end_edge =
        preview_bond_terminal_edge(&simplified, &projections, axis, max_projection, tolerance, width)?;
    let stroke_width =
        (start_edge.length + end_edge.length) * 0.5 * PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE;
    if !stroke_width.is_finite() || stroke_width <= 0.0 {
        return None;
    }
    Some(PreviewBondStrokeLine {
        start: start_edge.center,
        end: end_edge.center,
        width: stroke_width,
    })
}

fn preview_point_from_axis_coordinates(
    axis: CorePoint,
    normal: CorePoint,
    axis_projection: f64,
    normal_projection: f64,
) -> CorePoint {
    CorePoint {
        x: axis.x * axis_projection + normal.x * normal_projection,
        y: axis.y * axis_projection + normal.y * normal_projection,
    }
}

fn preview_simplify_bond_polygon(
    points: &[CorePoint],
    axis: CorePoint,
    tolerance: f64,
) -> Option<Vec<CorePoint>> {
    if points.len() < 4 {
        return None;
    }
    let mut simplified = points.to_vec();
    loop {
        if simplified.len() <= 4 {
            break;
        }
        let mut removed = false;
        let len = simplified.len();
        for index in 0..len {
            let prev = simplified[(index + len - 1) % len];
            let point = simplified[index];
            let next = simplified[(index + 1) % len];
            if preview_point_is_collinear(prev, point, next, axis, tolerance) {
                simplified.remove(index);
                removed = true;
                break;
            }
        }
        if !removed {
            break;
        }
    }
    Some(simplified)
}

fn preview_point_is_collinear(
    prev: CorePoint,
    point: CorePoint,
    next: CorePoint,
    axis: CorePoint,
    tolerance: f64,
) -> bool {
    let segment = CorePoint {
        x: next.x - prev.x,
        y: next.y - prev.y,
    };
    let segment_length = segment.distance(CorePoint { x: 0.0, y: 0.0 });
    if segment_length <= 1.0e-9 {
        return point.distance(prev) <= tolerance;
    }
    let prev_to_point = CorePoint {
        x: point.x - prev.x,
        y: point.y - prev.y,
    };
    let distance = ((prev_to_point.x * segment.y) - (prev_to_point.y * segment.x)).abs() / segment_length;
    if distance > tolerance.max(0.05) {
        return false;
    }
    let dot = prev_to_point.x * segment.x + prev_to_point.y * segment.y;
    let projection = dot / (segment_length * segment_length);
    if !(0.0..=1.0).contains(&projection) {
        return false;
    }
    let segment_axis_ratio = ((segment.x * axis.x + segment.y * axis.y).abs()) / segment_length;
    segment_axis_ratio >= 1.0 - PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO
}

fn preview_polygon_principal_axis(points: &[CorePoint]) -> Option<CorePoint> {
    if points.len() < 2 {
        return None;
    }
    let mut mean = CorePoint { x: 0.0, y: 0.0 };
    for point in points {
        mean.x += point.x;
        mean.y += point.y;
    }
    let point_count = points.len() as f64;
    mean.x /= point_count;
    mean.y /= point_count;

    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for point in points {
        let dx = point.x - mean.x;
        let dy = point.y - mean.y;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    let trace = sxx + syy;
    let root = (sxx - syy).hypot(2.0 * sxy);
    let lambda = (trace + root) * 0.5;
    let mut axis = CorePoint {
        x: sxy,
        y: lambda - sxx,
    };
    if axis.distance(CorePoint { x: 0.0, y: 0.0 }) <= 1.0e-9 {
        axis = CorePoint {
            x: lambda - syy,
            y: sxy,
        };
    }
    let length = axis.distance(CorePoint { x: 0.0, y: 0.0 });
    if length <= 1.0e-9 {
        return None;
    }
    Some(CorePoint {
        x: axis.x / length,
        y: axis.y / length,
    })
}

fn preview_bond_terminal_edge(
    points: &[CorePoint],
    projections: &[f64],
    axis: CorePoint,
    target: f64,
    tolerance: f64,
    width: f64,
) -> Option<PreviewBondTerminalEdge> {
    let indices: Vec<usize> = projections
        .iter()
        .enumerate()
        .filter_map(|(index, projection)| {
            if (*projection - target).abs() <= tolerance {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if indices.is_empty() || indices.len() > 3 {
        return None;
    }
    let ordered = preview_polygon_terminal_chain(points.len(), &indices)?;
    let normal_projection = |index: usize| points[index].x * -axis.y + points[index].y * axis.x;
    let center = match ordered.len() {
        1 => points[ordered[0]],
        2 => {
            let first = ordered[0];
            let last = ordered[1];
            let edge = CorePoint {
                x: points[last].x - points[first].x,
                y: points[last].y - points[first].y,
            };
            let edge_length = points[first].distance(points[last]).max(1.0e-9);
            let along_axis = (edge.x * axis.x + edge.y * axis.y).abs() / edge_length;
            if along_axis <= PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO {
                CorePoint {
                    x: (points[first].x + points[last].x) * 0.5,
                    y: (points[first].y + points[last].y) * 0.5,
                }
            } else {
                let apex = ordered
                    .iter()
                    .copied()
                    .min_by(|left, right| {
                        (projections[*left] - target)
                            .abs()
                            .total_cmp(&(projections[*right] - target).abs())
                    })
                    .unwrap_or(first);
                points[apex]
            }
        }
        3 => {
            let apex = ordered
                .iter()
                .copied()
                .min_by(|left, right| {
                    (projections[*left] - target)
                        .abs()
                        .total_cmp(&(projections[*right] - target).abs())
                })
                .unwrap_or(ordered[1]);
            points[apex]
        }
        _ => return None,
    };
    let edge_length = if ordered.len() == 1 {
        width
    } else {
        let first = ordered[0];
        let last = *ordered.last().unwrap_or(&ordered[0]);
        let span = (normal_projection(first) - normal_projection(last)).abs();
        let edge = CorePoint {
            x: points[last].x - points[first].x,
            y: points[last].y - points[first].y,
        };
        let along_axis = edge.x * axis.x + edge.y * axis.y;
        let edge_length = span.max(points[first].distance(points[last]));
        if edge_length <= 0.0 {
            return None;
        }
        if ordered.len() == 2 && along_axis.abs() / edge_length > PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO {
            width
        } else {
            if along_axis.abs() / edge_length > PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO {
                return None;
            }
            edge_length
        }
    };
    if edge_length < width * PREVIEW_BOND_STROKE_EDGE_WIDTH_MIN_RATIO
        || edge_length > width * PREVIEW_BOND_STROKE_EDGE_WIDTH_MAX_RATIO
    {
        return None;
    }
    Some(PreviewBondTerminalEdge {
        center,
        length: edge_length,
    })
}

fn preview_polygon_terminal_chain(len: usize, indices: &[usize]) -> Option<Vec<usize>> {
    if indices.is_empty() || indices.len() > 3 {
        return None;
    }
    if indices.len() == 1 {
        return Some(indices.to_vec());
    }
    let mut ordered = indices.to_vec();
    ordered.sort_unstable();
    for &start in &ordered {
        let mut chain = vec![start];
        let mut current = start;
        while chain.len() < ordered.len() {
            let next = (current + 1) % len;
            if ordered.contains(&next) {
                chain.push(next);
                current = next;
            } else {
                break;
            }
        }
        if chain.len() == ordered.len() {
            return Some(chain);
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64) -> CorePoint {
        CorePoint { x, y }
    }

    fn test_bond(id: &str, begin: &str, end: &str) -> Bond {
        Bond {
            id: id.to_string(),
            begin: begin.to_string(),
            end: end.to_string(),
            order: 1,
            double: None,
            stereo: None,
            stroke_width: 0.85,
            stroke: None,
            bold_width: None,
            wedge_width: None,
            label_clip_margin: None,
            hash_spacing: None,
            bond_spacing: None,
            margin_width: None,
            line_styles: Default::default(),
            line_weights: Default::default(),
            meta: serde_json::Value::Null,
        }
    }

    fn context_with_bond(
        bond_id: &str,
        axis: CorePoint,
        allow_pen: bool,
        start_projection: f64,
        end_projection: f64,
    ) -> PreviewBondContext {
        let mut infos = BTreeMap::new();
        infos.insert(
            bond_id.to_string(),
            PreviewBondInfo {
                axis,
                allow_pen,
                start_projection,
                end_projection,
                axis_normal_projection: 0.0,
                side_double: false,
                start_has_label: false,
                end_has_label: false,
            },
        );
        PreviewBondContext { infos }
    }

    #[test]
    fn preview_bond_stroke_line_converts_simple_rectangle() {
        let stroke_line = preview_bond_stroke_line(&[
            point(0.0, 0.0),
            point(20.0, 0.0),
            point(20.0, 2.64),
            point(0.0, 2.64),
        ], None, None)
        .expect("simple bond shaft should convert to a centerline stroke");
        assert!((stroke_line.start.x - 0.0).abs() < 1.0e-6);
        assert!((stroke_line.start.y - 1.32).abs() < 1.0e-6);
        assert!((stroke_line.end.x - 20.0).abs() < 1.0e-6);
        assert!((stroke_line.end.y - 1.32).abs() < 1.0e-6);
        assert!(
            (stroke_line.width - 2.64 * PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE).abs() < 1.0e-6
        );
    }

    #[test]
    fn preview_bond_stroke_line_converts_pentagon_join() {
        let stroke_line = preview_bond_stroke_line(&[
            point(0.0, 0.0),
            point(10.0, 0.0),
            point(20.0, 0.0),
            point(20.0, 2.64),
            point(0.0, 2.64),
        ], None, None)
        .expect("pentagon with a collinear shoulder should still convert to a pen stroke");
        assert!(stroke_line.width > 0.0);
        assert!((stroke_line.start.x - 0.0).abs() < 1.0e-6);
        assert!((stroke_line.end.x - 20.0).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_converts_complex_junction_hexagon() {
        let axis = preview_normalize_axis(point(392.077488 - 370.689708, 662.127888 - 696.293739))
            .expect("axis");
        let start_projection = 370.689708 * axis.x + 696.293739 * axis.y;
        let end_projection = 392.077488 * axis.x + 662.127888 * axis.y;
        let context = context_with_bond("b1", axis, true, start_projection, end_projection);
        let stroke_line = preview_bond_stroke_line(&[
            point(370.689708, 696.293739),
            point(392.077488, 662.127888),
            point(390.45, 662.24),
            point(388.822512, 662.352112),
            point(369.470292, 693.266261),
            point(370.08, 694.78),
        ], Some("b1"), Some(&context))
        .expect("plain same-width junction hexagon should convert when bond axis is known");
        assert!(stroke_line.width > 0.0);
        let direction = preview_normalize_axis(point(
            stroke_line.end.x - stroke_line.start.x,
            stroke_line.end.y - stroke_line.start.y,
        ))
        .expect("stroke direction");
        let dot = (direction.x * axis.x + direction.y * axis.y).abs();
        assert!(dot > 0.995, "stroke should stay parallel to the bond axis");
    }

    #[test]
    fn preview_bond_stroke_line_accepts_short_thin_horizontal_bond() {
        let stroke_line = preview_bond_stroke_line(&[
            point(714.35, 707.54),
            point(727.33, 707.54),
            point(727.33, 704.9),
            point(714.35, 704.9),
        ], None, None)
        .expect("short thin N-O bond should still convert to a stroke");
        assert!(
            (stroke_line.width - 2.64 * PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE).abs() < 1.0e-6
        );
    }

    #[test]
    fn preview_bond_stroke_line_accepts_apex_terminal_with_axis_hint() {
        let axis = preview_normalize_axis(point(0.0, -1.0)).expect("axis");
        let context = context_with_bond("b1", axis, true, -421.402155, -382.24);
        let stroke_line = preview_bond_stroke_line(&[
            point(285.27, 421.402155),
            point(285.27, 383.002041),
            point(283.95, 382.24),
            point(282.63, 383.002155),
            point(282.63, 419.877845),
        ], Some("b1"), Some(&context));
        assert!(stroke_line.is_some(), "axis hint should keep plain apex terminals on pen");
    }

    #[test]
    fn preview_bond_stroke_line_uses_shared_cap_center_projection_on_joined_end() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut context = context_with_bond("b1", axis, true, -3.0, 20.0);
        context.infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                allow_pen: true,
                start_projection: -3.0,
                end_projection: 20.0,
                axis_normal_projection: 0.0,
                side_double: false,
                start_has_label: false,
                end_has_label: false,
            },
        );
        let stroke_line = preview_bond_stroke_line(&[
            point(0.0, 0.0),
            point(20.0, 0.0),
            point(20.0, 2.64),
            point(0.0, 2.64),
        ], Some("b1"), Some(&context))
        .expect("joined end should still convert");
        assert!((stroke_line.start.x + 3.0).abs() < 1.0e-6);
        assert!((stroke_line.end.x - 20.0).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_rejects_wedge_like_junction_hexagon() {
        let axis = preview_normalize_axis(point(392.077488 - 370.689708, 662.127888 - 696.293739))
            .expect("axis");
        let start_projection = 370.689708 * axis.x + 696.293739 * axis.y;
        let end_projection = 392.077488 * axis.x + 662.127888 * axis.y;
        let context = context_with_bond("b1", axis, false, start_projection, end_projection);
        let stroke_line = preview_bond_stroke_line(&[
            point(370.689708, 696.293739),
            point(392.077488, 662.127888),
            point(390.45, 662.24),
            point(388.822512, 662.352112),
            point(369.470292, 693.266261),
            point(370.08, 694.78),
        ], Some("b1"), Some(&context));
        assert!(stroke_line.is_none(), "junction hexagon should stay as polygon");
    }

    #[test]
    fn preview_bond_stroke_line_keeps_side_double_outer_line_inset_length() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut infos = BTreeMap::new();
        infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                allow_pen: true,
                start_projection: 0.0,
                end_projection: 20.0,
                axis_normal_projection: 0.0,
                side_double: true,
                start_has_label: false,
                end_has_label: false,
            },
        );
        let context = PreviewBondContext { infos };
        let stroke_line = preview_bond_stroke_line(
            &[
                point(2.0, 8.68),
                point(18.0, 8.68),
                point(18.0, 11.32),
                point(2.0, 11.32),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("side-double outer line should still convert to a stroke");
        let radius = stroke_line.width * 0.5;
        assert!(((stroke_line.start.x - radius) - 2.0).abs() < 1.0e-6);
        assert!(((stroke_line.end.x + radius) - 18.0).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_uses_visible_endpoint_center_for_labeled_end() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut infos = BTreeMap::new();
        infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                allow_pen: true,
                start_projection: 0.0,
                end_projection: 12.0,
                axis_normal_projection: 0.0,
                side_double: false,
                start_has_label: false,
                end_has_label: true,
            },
        );
        let context = PreviewBondContext { infos };
        let stroke_line = preview_bond_stroke_line(
            &[
                point(0.0, 1.0),
                point(10.0, 1.0),
                point(10.0, 0.0),
                point(0.5, 0.0),
                point(0.0, 0.5),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("labeled end shaft should still convert");
        assert!((stroke_line.end.x - 10.0).abs() < 1.0e-6);
    }

    #[test]
    fn preview_pen_family_bonds_ignore_neighbor_shape_for_allow_pen() {
        let bond = test_bond("b1", "n1", "n2");
        assert!(preview_bond_is_pen_family(&bond));
    }
}
