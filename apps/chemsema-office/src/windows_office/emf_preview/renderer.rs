// GDI replay for ChemSema document preview primitives.
//
// Keep Office/OLE container decisions out of this file. Code here should be
// about geometry, pens, brushes, text metrics, path replay, clipping, and the
// ChemDraw-style EMF record strategy.

use super::*;
use chemsema_engine::{
    Bond, BondLinePattern, BondLineWeight, ChemSemaDocument, DoubleBondPlacement, MoleculeFragment,
    SceneObject, DEFAULT_BOND_STROKE,
};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use windows_sys::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, HENHMETAFILE};
use windows_sys::Win32::Graphics::GdiPlus::{
    DashCapRound, DashStyleDash, EmfTypeEmfPlusDual, FillModeAlternate, FontStyleBold,
    FontStyleItalic, FontStyleRegular, FontStyleUnderline, GdipAddPathBezier, GdipAddPathLine,
    GdipCloneStringFormat, GdipClosePathFigure, GdipCreateFont, GdipCreateFontFamilyFromName,
    GdipCreateFromHDC, GdipCreatePath, GdipCreatePen1, GdipCreateSolidFill, GdipCreateStringFormat,
    GdipDeleteBrush, GdipDeleteFont, GdipDeleteFontFamily, GdipDeleteGraphics, GdipDeletePath,
    GdipDeletePen, GdipDeleteStringFormat, GdipDisposeImage, GdipDrawEllipse, GdipDrawLine,
    GdipDrawLines, GdipDrawPath, GdipDrawPolygon, GdipDrawRectangle, GdipDrawString,
    GdipFillEllipse, GdipFillPath, GdipFillPolygon, GdipFillRectangle, GdipGetDC,
    GdipGetHemfFromMetafile, GdipGetImageGraphicsContext, GdipMeasureString, GdipRecordMetafile,
    GdipReleaseDC, GdipRestoreGraphics, GdipSaveGraphics, GdipSetPageScale, GdipSetPageUnit,
    GdipSetPenDashArray, GdipSetPenDashCap197819, GdipSetPenDashStyle, GdipSetPenEndCap,
    GdipSetPenLineJoin, GdipSetPenMiterLimit, GdipSetPenStartCap, GdipSetPixelOffsetMode,
    GdipSetSmoothingMode, GdipSetStringFormatAlign, GdipSetStringFormatFlags,
    GdipSetStringFormatLineAlign, GdipSetTextRenderingHint, GdipStartPathFigure,
    GdipStringFormatGetGenericTypographic, GdiplusStartup, GdiplusStartupInput, GpBrush, GpFont,
    GpFontFamily, GpGraphics, GpImage, GpMetafile, GpPath, GpPen, GpStringFormat, LineCapFlat,
    LineCapRound, LineCapSquare, LineJoinBevel, LineJoinMiter, LineJoinRound, MetafileFrameUnitGdi,
    Ok as GDI_PLUS_OK, PixelOffsetModeHighQuality, PointF, RectF, SmoothingModeAntiAlias,
    StringAlignmentNear, StringFormatFlagsMeasureTrailingSpaces, StringFormatFlagsNoClip,
    StringFormatFlagsNoFitBlackBox, TextRenderingHintAntiAlias, TextRenderingHintAntiAliasGridFit,
    UnitPixel, UnitWorld,
};

mod bonds;
mod color;
mod gdiplus;
mod paths;
mod renderer_env;
mod surface;
mod text;

use bonds::*;
use color::*;
use gdiplus::*;
use paths::*;
use renderer_env::*;
use surface::*;
use text::*;

pub(super) fn office_preview_primitive_visible(primitive: &RenderPrimitive) -> bool {
    office_preview_primitive_visible_impl(primitive)
}

pub(super) unsafe fn draw_placeholder_preview(dc: HDC, bounds: &RECT) {
    draw_placeholder_preview_impl(dc, bounds)
}

const EMF_VECTOR_RECORD_SCALE: f64 = 16.0;
const EMF_ARROW_RECORD_SCALE: f64 = EMF_VECTOR_RECORD_SCALE;
const USE_GDIPLUS_TEXT_PREVIEW: bool = true;
const CHEMDRAW_EMF_PAGE_SCALE: f32 = 0.266_666_68;
const CHEMDRAW_SCRIPT_SCALE: f64 = 0.75;
const CHEMDRAW_SUBSCRIPT_SHIFT_DOWN_EM: f64 = 0.22;
const CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM: f64 = 0.215;
const CHEMDRAW_SUPERSCRIPT_SHIFT_UP_EM: f64 = 0.392;
const CHEMDRAW_PACKAGED_CENTERED_TEXT_TOP_BIAS_EM: f32 = 0.012;
const CHEMDRAW_PACKAGED_CENTERED_SCRIPT_EXTRA_TOP_BIAS_EM: f32 = 0.02;
const CHEMDRAW_DEFAULT_MULTILINE_BLACK_LABEL_Y_NUDGE_PX: f64 = -3.0;
const OUT_TT_ONLY_PRECIS_VALUE: u32 = 7;
const CHEMDRAW_GDI_TEXT_ADVANCE_TIGHTEN: f64 = 0.965;

#[derive(Clone, Copy)]
struct PreviewTransform {
    min_x: f64,
    min_y: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    record_scale: f64,
    emf_recording: bool,
    office_presentation: bool,
}

#[derive(Debug, Clone)]
struct PreviewLabelContext {
    infos: BTreeMap<String, PreviewLabelInfo>,
}

#[derive(Debug, Clone)]
struct PreviewLabelInfo {
    layout: Option<String>,
    world_box: Option<PreviewLabelBBox>,
    simple_single_run: bool,
    line_count: usize,
}

impl PreviewLabelInfo {
    fn is_attached_group_layout(&self) -> bool {
        matches!(
            self.layout.as_deref(),
            Some("attached-group" | "attached-group-above")
        )
    }
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
            office_presentation: false,
        })
    }

    fn with_record_scale(self, record_scale: f64) -> Self {
        Self {
            record_scale: record_scale.max(1.0),
            ..self
        }
    }

    fn for_emf_recording(self, office_presentation: bool) -> Self {
        Self {
            emf_recording: true,
            office_presentation,
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
        let page_scale = if self.emf_recording {
            CHEMDRAW_EMF_PAGE_SCALE
        } else {
            1.0
        };
        PointF {
            X: (self.offset_x + (point.x - self.min_x) * self.scale) as f32 / page_scale,
            Y: (self.offset_y + (point.y - self.min_y) * self.scale) as f32 / page_scale,
        }
    }

    fn length(&self, value: f64) -> i32 {
        (value.abs() * self.scale * self.record_scale)
            .round()
            .max(1.0) as i32
    }

    fn gdip_length(&self, value: f64) -> f32 {
        if self.emf_recording {
            ((value.abs() * self.scale) as f32 / CHEMDRAW_EMF_PAGE_SCALE).max(0.01)
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
    if draw_payload_vector_preview_internal(
        dc,
        bounds,
        payload,
        super::preview_source_bounds(payload),
        true,
    ) {
        return true;
    }

    draw_svg_preview(dc, bounds, payload)
}

pub(super) unsafe fn draw_payload_emf_vector_preview_with_source_bounds(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
) -> bool {
    draw_payload_vector_preview_internal(dc, bounds, payload, source_bounds, true)
}

pub(super) unsafe fn draw_payload_compatible_vector_preview_with_source_bounds(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
) -> bool {
    draw_payload_vector_preview_internal(dc, bounds, payload, source_bounds, false)
}

pub(super) unsafe fn enhanced_metafile_gdiplus_dual_preview(
    frame_bounds: &RECT,
    draw_bounds: &RECT,
    payload: &OleObjectPayload,
    source_bounds: Option<[f64; 4]>,
    office_presentation: bool,
) -> Option<HENHMETAFILE> {
    if !ensure_gdiplus_started() {
        return None;
    }
    let primitives = if let Some(primitives) = payload_render_primitives(payload) {
        primitives
    } else {
        let Ok(document) = parse_document_json(&payload.chemsema_document_json) else {
            return None;
        };
        render_document(&document)
    };
    let visible: Vec<_> = primitives
        .iter()
        .filter(|primitive| office_preview_primitive_visible(primitive))
        .collect();
    let primitive_bounds = render_primitives_bounds(visible.iter().copied())?;
    let transform =
        PreviewTransform::from_bounds(draw_bounds, source_bounds.unwrap_or(primitive_bounds))?;
    let transform = transform.for_emf_recording(office_presentation);
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
    if transform.emf_recording {
        GdipSetPageUnit(graphics, UnitPixel);
        GdipSetPageScale(graphics, 1.0);
        GdipSetPageScale(graphics, CHEMDRAW_EMF_PAGE_SCALE);
        if let Some(pixel_offset_mode) = preview_env_i32(ENV_PACKAGED_PIXEL_OFFSET_MODE_VALUE) {
            GdipSetPixelOffsetMode(graphics, pixel_offset_mode);
        } else if preview_env_enabled(ENV_PACKAGED_PIXEL_OFFSET_HIGHQUALITY) {
            GdipSetPixelOffsetMode(graphics, PixelOffsetModeHighQuality);
        }
    }
    let smoothing_mode = if transform.emf_recording {
        preview_env_i32(ENV_PACKAGED_SMOOTHING_MODE_VALUE).unwrap_or(SmoothingModeAntiAlias)
    } else {
        SmoothingModeAntiAlias
    };
    GdipSetSmoothingMode(graphics, smoothing_mode);
    GdipSetTextRenderingHint(
        graphics,
        preview_default_gdiplus_text_rendering_hint(&transform),
    );
    let use_gdiplus_text = gdiplus_text_preview_enabled();
    let bond_context = preview_bond_context(payload);
    let label_context = preview_label_context(payload);
    let mut gdi_cache = PreviewGdiCache::default();
    let mut ok = true;
    for primitive in visible {
        if matches!(primitive, RenderPrimitive::Text { .. }) {
            let drawn = use_gdiplus_text
                && draw_gdiplus_primitive(
                    graphics,
                    primitive,
                    &transform,
                    bond_context.as_ref(),
                    label_context.as_ref(),
                );
            if !drawn
                && !draw_gdi_primitive_in_gdiplus(
                    graphics,
                    primitive,
                    &transform,
                    &mut gdi_cache,
                    bond_context.as_ref(),
                    label_context.as_ref(),
                )
            {
                ok = false;
                break;
            }
        } else if !draw_gdiplus_primitive(
            graphics,
            primitive,
            &transform,
            bond_context.as_ref(),
            label_context.as_ref(),
        ) && !draw_gdi_primitive_in_gdiplus(
            graphics,
            primitive,
            &transform,
            &mut gdi_cache,
            bond_context.as_ref(),
            label_context.as_ref(),
        ) {
            ok = false;
            break;
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
    (USE_GDIPLUS_TEXT_PREVIEW || std::env::var_os("CHEMSEMA_OFFICE_GDIPLUS_TEXT").is_some())
        && std::env::var_os("CHEMSEMA_OFFICE_DISABLE_GDIPLUS_TEXT").is_none()
}

fn gdiplus_text_scale(transform: &PreviewTransform) -> f64 {
    if transform.emf_recording {
        transform.scale / CHEMDRAW_EMF_PAGE_SCALE as f64
    } else {
        transform.scale
    }
}

unsafe fn draw_gdi_primitive_in_gdiplus(
    graphics: *mut GpGraphics,
    primitive: &RenderPrimitive,
    transform: &PreviewTransform,
    cache: &mut PreviewGdiCache,
    bond_context: Option<&PreviewBondContext>,
    label_context: Option<&PreviewLabelContext>,
) -> bool {
    let mut dc: HDC = null_mut();
    if GdipGetDC(graphics, &mut dc) != GDI_PLUS_OK || dc.is_null() {
        return false;
    }
    draw_preview_primitive(dc, primitive, transform, cache, bond_context, label_context);
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

struct SvgPreviewBitmap {
    width: i32,
    height: i32,
    bgra: Vec<u8>,
}

struct GdiplusTextLineLayout {
    width: f32,
    runs: Vec<GdiplusTextRunLayout>,
}

struct GdiplusTextRunLayout {
    dx: f32,
    advance: f32,
}

#[allow(clippy::too_many_arguments)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewAttachedStartLayoutMode {
    Default,
    Zero,
    Tight,
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
    tighten_advance: bool,
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
        default_font_size: f64,
        default_family: Option<&str>,
        transform: &PreviewTransform,
    ) -> HGDIOBJ {
        let key = preview_font_key(run, default_font_size, default_family, transform);
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

const PREVIEW_MITER_LIMIT: f32 = 2.0;
const PREVIEW_BOND_STROKE_MAX_WIDTH: f64 = 4.25;
const PREVIEW_BOND_STROKE_TOLERANCE_WIDTH_FACTOR: f64 = 0.45;
const PREVIEW_BOND_STROKE_COLLINEAR_TOLERANCE_WIDTH_FACTOR: f64 = 0.18;
const PREVIEW_BOND_STROKE_EDGE_WIDTH_MIN_RATIO: f64 = 0.55;
const PREVIEW_BOND_STROKE_EDGE_WIDTH_MAX_RATIO: f64 = 1.45;
const PREVIEW_BOND_STROKE_EDGE_AXIS_MAX_RATIO: f64 = 0.25;
// Keep pen-converted bond polygons aligned with the kernel/document strokeWidth.
// The document payload already carries the intended bond width (for this file: 2.64),
// so the Office preview should not apply an extra optical inflation here.
const PREVIEW_BOND_STROKE_OPTICAL_WIDTH_SCALE: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
enum PreviewPathCommand {
    Move(CorePoint),
    Line(CorePoint),
    Cubic(CorePoint, CorePoint, CorePoint),
    Close,
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

#[derive(Debug, Clone, Copy)]
struct PreviewBondInfo {
    axis: CorePoint,
    line_width: f64,
    hashed_wedge_wide_projection: Option<f64>,
    allow_pen: bool,
    order: u8,
    start_projection: f64,
    end_projection: f64,
    axis_normal_projection: f64,
    both_junction: bool,
    side_double: bool,
    center_double: bool,
    hashed_wedge: bool,
    start_has_label: bool,
    end_has_label: bool,
}

#[derive(Debug, Default)]
struct PreviewBondContext {
    infos: BTreeMap<String, PreviewBondInfo>,
}

#[derive(Debug, Clone, Copy)]
struct PreviewLabelBBox {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl PreviewLabelBBox {
    fn expand_to_include(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64) -> CorePoint {
        CorePoint { x, y }
    }

    #[test]
    fn emf_gdiplus_length_scales_like_points() {
        let transform = PreviewTransform {
            min_x: 0.0,
            min_y: 0.0,
            scale: 2.0,
            offset_x: 0.0,
            offset_y: 0.0,
            record_scale: 1.0,
            emf_recording: true,
            office_presentation: false,
        };

        let expected = (0.85_f64 * 2.0) as f32 / CHEMDRAW_EMF_PAGE_SCALE;
        assert!((transform.gdip_length(0.85) - expected).abs() < 1.0e-6);
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
            highlight_color: None,
            bold_width: None,
            wedge_width: None,
            label_clip_margin: None,
            hash_spacing: None,
            bond_spacing: None,
            margin_width: None,
            line_styles: Default::default(),
            line_weights: Default::default(),
            properties: Default::default(),
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
                line_width: 1.0,
                hashed_wedge_wide_projection: None,
                allow_pen,
                order: 1,
                start_projection,
                end_projection,
                axis_normal_projection: 0.0,
                both_junction: false,
                side_double: false,
                center_double: false,
                hashed_wedge: false,
                start_has_label: false,
                end_has_label: false,
            },
        );
        PreviewBondContext { infos }
    }

    #[test]
    fn preview_bond_stroke_line_converts_simple_rectangle() {
        let stroke_line = preview_bond_stroke_line(
            &[
                point(0.0, 0.0),
                point(20.0, 0.0),
                point(20.0, 2.64),
                point(0.0, 2.64),
            ],
            None,
            None,
        )
        .expect("simple bond shaft should convert to a centerline stroke");
        assert!((stroke_line.start.x - 0.0).abs() < 1.0e-6);
        assert!((stroke_line.start.y - 1.32).abs() < 1.0e-6);
        assert!((stroke_line.end.x - 20.0).abs() < 1.0e-6);
        assert!((stroke_line.end.y - 1.32).abs() < 1.0e-6);
        assert!((stroke_line.width - 2.64).abs() < 1.0e-6);
    }

    #[test]
    fn preview_hashed_wedge_polygon_becomes_a_perpendicular_round_pen_centerline() {
        let axis = point(1.0, 0.0);
        let mut context = context_with_bond("b1", axis, false, 0.0, 30.0);
        context
            .infos
            .get_mut("b1")
            .expect("bond context")
            .hashed_wedge = true;
        let stroke_line = preview_hashed_wedge_stroke_line(
            &[
                point(0.0, 0.5),
                point(1.0, 0.583_333_333_3),
                point(1.0, -0.583_333_333_3),
                point(0.0, -0.5),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("hashed wedge stripe should become a perpendicular pen stroke");
        assert!((stroke_line.start.x - 0.5).abs() < 1.0e-9);
        assert!((stroke_line.end.x - 0.5).abs() < 1.0e-9);
        assert!((stroke_line.start.y + 0.541_666_666_65).abs() < 1.0e-9);
        assert!((stroke_line.end.y - 0.541_666_666_65).abs() < 1.0e-9);
        assert!((stroke_line.width - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn preview_single_hash_samples_half_a_line_width_inside_the_wide_end() {
        let axis = point(1.0, 0.0);
        let mut context = context_with_bond("b1", axis, false, 0.0, 2.0);
        let info = context.infos.get_mut("b1").expect("bond context");
        info.hashed_wedge = true;
        info.hashed_wedge_wide_projection = Some(2.0);
        let stroke_line = preview_hashed_wedge_stroke_line(
            &[
                point(0.0, 0.5),
                point(2.0, 3.0),
                point(2.0, -3.0),
                point(0.0, -0.5),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("single-hash trapezoid should be sampled at its wide-end stripe center");
        assert!((stroke_line.start.x - 1.5).abs() < 1.0e-9);
        assert!((stroke_line.end.x - 1.5).abs() < 1.0e-9);
        assert!((stroke_line.start.y + 2.375).abs() < 1.0e-9);
        assert!((stroke_line.end.y - 2.375).abs() < 1.0e-9);
        assert!((stroke_line.width - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn preview_office_hashed_wedge_replays_only_the_narrow_stripe_axially() {
        let axis = point(1.0, 0.0);
        let mut context = context_with_bond("b1", axis, false, 0.0, 30.0);
        context
            .infos
            .get_mut("b1")
            .expect("bond context")
            .hashed_wedge = true;
        let narrow = preview_office_hashed_wedge_stroke_line(
            &[
                point(0.0, 0.5),
                point(1.0, 0.583_333_333_3),
                point(1.0, -0.583_333_333_3),
                point(0.0, -0.5),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("the near-square narrow stripe should become an axial Office pen");
        assert!((narrow.start.x - 0.0).abs() < 1.0e-9);
        assert!((narrow.end.x - 1.0).abs() < 1.0e-9);
        assert!((narrow.start.y).abs() < 1.0e-9);
        assert!((narrow.end.y).abs() < 1.0e-9);

        assert!(preview_office_hashed_wedge_stroke_line(
            &[
                point(3.0, 1.5),
                point(4.0, 1.7),
                point(4.0, -1.7),
                point(3.0, -1.5),
            ],
            Some("b1"),
            Some(&context),
        )
        .is_none());
    }

    #[test]
    fn preview_bond_stroke_line_converts_pentagon_join() {
        let stroke_line = preview_bond_stroke_line(
            &[
                point(0.0, 0.0),
                point(10.0, 0.0),
                point(20.0, 0.0),
                point(20.0, 2.64),
                point(0.0, 2.64),
            ],
            None,
            None,
        )
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
        let stroke_line = preview_bond_stroke_line(
            &[
                point(370.689708, 696.293739),
                point(392.077488, 662.127888),
                point(390.45, 662.24),
                point(388.822512, 662.352112),
                point(369.470292, 693.266261),
                point(370.08, 694.78),
            ],
            Some("b1"),
            Some(&context),
        )
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
        let stroke_line = preview_bond_stroke_line(
            &[
                point(714.35, 707.54),
                point(727.33, 707.54),
                point(727.33, 704.9),
                point(714.35, 704.9),
            ],
            None,
            None,
        )
        .expect("short thin N-O bond should still convert to a stroke");
        assert!((stroke_line.width - 2.64).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_accepts_apex_terminal_with_axis_hint() {
        let axis = preview_normalize_axis(point(0.0, -1.0)).expect("axis");
        let context = context_with_bond("b1", axis, true, -421.402155, -382.24);
        let stroke_line = preview_bond_stroke_line(
            &[
                point(285.27, 421.402155),
                point(285.27, 383.002041),
                point(283.95, 382.24),
                point(282.63, 383.002155),
                point(282.63, 419.877845),
            ],
            Some("b1"),
            Some(&context),
        );
        assert!(
            stroke_line.is_some(),
            "axis hint should keep plain apex terminals on pen"
        );
    }

    #[test]
    fn preview_bond_stroke_line_uses_shared_cap_center_projection_on_joined_end() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut context = context_with_bond("b1", axis, true, -3.0, 20.0);
        context.infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                line_width: 1.0,
                hashed_wedge_wide_projection: None,
                allow_pen: true,
                order: 1,
                start_projection: -3.0,
                end_projection: 20.0,
                axis_normal_projection: 0.0,
                both_junction: false,
                side_double: false,
                center_double: false,
                hashed_wedge: false,
                start_has_label: false,
                end_has_label: false,
            },
        );
        let stroke_line = preview_bond_stroke_line(
            &[
                point(0.0, 0.0),
                point(20.0, 0.0),
                point(20.0, 2.64),
                point(0.0, 2.64),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("joined end should still convert");
        assert!((stroke_line.start.x + 3.0).abs() < 1.0e-6);
        assert!((stroke_line.end.x - 20.0).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_uses_terminal_edge_center_for_center_double_projection() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut context = context_with_bond("b1", axis, true, 0.0, 18.0);
        context.infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                line_width: 1.0,
                hashed_wedge_wide_projection: None,
                allow_pen: true,
                order: 2,
                start_projection: 0.0,
                end_projection: 18.0,
                axis_normal_projection: 0.0,
                both_junction: false,
                side_double: false,
                center_double: true,
                hashed_wedge: false,
                start_has_label: false,
                end_has_label: false,
            },
        );
        let stroke_line = preview_bond_stroke_line(
            &[
                point(0.0, 0.0),
                point(18.0, 0.0),
                point(20.0, 1.32),
                point(18.0, 2.64),
                point(0.0, 2.64),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("center-double joined end should convert");
        assert!((stroke_line.start.x - 0.0).abs() < 1.0e-6);
        assert!((stroke_line.end.x - 18.68).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_rejects_wedge_like_junction_hexagon() {
        let axis = preview_normalize_axis(point(392.077488 - 370.689708, 662.127888 - 696.293739))
            .expect("axis");
        let start_projection = 370.689708 * axis.x + 696.293739 * axis.y;
        let end_projection = 392.077488 * axis.x + 662.127888 * axis.y;
        let context = context_with_bond("b1", axis, false, start_projection, end_projection);
        let stroke_line = preview_bond_stroke_line(
            &[
                point(370.689708, 696.293739),
                point(392.077488, 662.127888),
                point(390.45, 662.24),
                point(388.822512, 662.352112),
                point(369.470292, 693.266261),
                point(370.08, 694.78),
            ],
            Some("b1"),
            Some(&context),
        );
        assert!(
            stroke_line.is_none(),
            "junction hexagon should stay as polygon"
        );
    }

    #[test]
    fn preview_bond_stroke_line_keeps_side_double_outer_line_inset_length() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut infos = BTreeMap::new();
        infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                line_width: 1.0,
                hashed_wedge_wide_projection: None,
                allow_pen: true,
                order: 2,
                start_projection: 0.0,
                end_projection: 20.0,
                axis_normal_projection: 0.0,
                both_junction: false,
                side_double: true,
                center_double: false,
                hashed_wedge: false,
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
    fn preview_bond_stroke_line_converts_short_label_clipped_so_double_segment() {
        let axis =
            preview_normalize_axis(point(-1.173625246904458, -2.0326537262359904)).expect("axis");
        let mut infos = BTreeMap::new();
        infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                line_width: 1.0,
                hashed_wedge_wide_projection: None,
                allow_pen: true,
                order: 2,
                start_projection: 0.0,
                end_projection: 0.0,
                axis_normal_projection: 0.0,
                both_junction: false,
                side_double: false,
                center_double: true,
                hashed_wedge: false,
                start_has_label: true,
                end_has_label: true,
            },
        );
        let context = PreviewBondContext { infos };
        let stroke_line = preview_bond_stroke_line(
            &[
                point(349.23256650268956, 243.45628947936865),
                point(348.0589412557851, 241.42363575313266),
                point(347.5393339912999, 241.72364957063814),
                point(348.71295923820435, 243.75630329687414),
            ],
            Some("b1"),
            Some(&context),
        )
        .expect("short clipped S=O double-bond segment should convert to a round-cap pen");
        assert!((stroke_line.width - 0.6).abs() < 1.0e-6);
        assert!((stroke_line.start.distance(stroke_line.end) - 2.347142388299574).abs() < 1.0e-6);
    }

    #[test]
    fn preview_bond_stroke_line_uses_visible_endpoint_center_for_labeled_end() {
        let axis = preview_normalize_axis(point(1.0, 0.0)).expect("axis");
        let mut infos = BTreeMap::new();
        infos.insert(
            "b1".to_string(),
            PreviewBondInfo {
                axis,
                line_width: 1.0,
                hashed_wedge_wide_projection: None,
                allow_pen: true,
                order: 1,
                start_projection: 0.0,
                end_projection: 12.0,
                axis_normal_projection: 0.0,
                both_junction: false,
                side_double: false,
                center_double: false,
                hashed_wedge: false,
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

    #[test]
    fn preview_text_chunks_match_chemdraw_word_spacing_pattern() {
        assert_eq!(
            preview_text_chunks("4DPAIPN (2 mol%)"),
            vec!["4DPAIPN ", "(2 ", "mol%)"]
        );
        assert_eq!(
            preview_text_chunks(" (5 mol%), L (7 mol%)"),
            vec![" ", "(5 ", "mol%), ", "L ", "(7 ", "mol%)"]
        );
        assert_eq!(
            preview_text_chunks("76% yield, 94% ee"),
            vec!["76% ", "yield, ", "94% ", "ee"]
        );
    }

    #[test]
    fn preview_text_lines_split_plain_runs_into_word_chunks() {
        let lines = preview_text_lines("4DPAIPN (2 mol%)", &[]);
        let chunks: Vec<_> = lines[0].iter().map(|run| run.text.as_str()).collect();
        assert_eq!(chunks, vec!["4DPAIPN ", "(2 ", "mol%)"]);
    }

    #[test]
    fn preview_invalid_marker_rect_is_hidden_by_default() {
        let primitive = RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            object_id: Some("o1".to_string()),
            node_id: Some("n1".to_string()),
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some("none".to_string()),
            stroke: Some("#d32f2f".to_string()),
            stroke_width: 1.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        };
        assert!(preview_is_invalid_marker_primitive(&primitive));
        assert!(!office_preview_primitive_visible(&primitive));
    }

    #[test]
    fn preview_invalid_label_marker_without_node_id_is_hidden_by_default() {
        let primitive = RenderPrimitive::Rect {
            role: RenderRole::DocumentDiagnostic,
            object_id: None,
            node_id: None,
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some("none".to_string()),
            stroke: Some("#d32f2f".to_string()),
            stroke_width: 1.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        };
        assert!(preview_is_invalid_marker_primitive(&primitive));
        assert!(!office_preview_primitive_visible(&primitive));
    }

    #[test]
    fn preview_non_invalid_document_graphic_stays_visible() {
        let primitive = RenderPrimitive::Rect {
            role: RenderRole::DocumentGraphic,
            object_id: Some("o1".to_string()),
            node_id: Some("n1".to_string()),
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some("none".to_string()),
            stroke: Some("#000000".to_string()),
            stroke_width: 1.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        };
        assert!(!preview_is_invalid_marker_primitive(&primitive));
        assert!(office_preview_primitive_visible(&primitive));
    }

    #[test]
    fn preview_document_knockout_is_visible_by_default() {
        let primitive = RenderPrimitive::Rect {
            role: RenderRole::DocumentKnockout,
            object_id: Some("o1".to_string()),
            node_id: None,
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: 0.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        };
        assert!(office_preview_primitive_visible(&primitive));
    }

    #[test]
    fn preview_label_knockout_stays_transparent() {
        let primitive = RenderPrimitive::Rect {
            role: RenderRole::DocumentKnockout,
            object_id: Some("o1".to_string()),
            node_id: Some("n1".to_string()),
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: 0.0,
            rx: None,
            ry: None,
            dash_array: Vec::new(),
            fill_gradient: None,
        };
        assert!(!office_preview_primitive_visible(&primitive));
    }

    #[test]
    fn preview_default_multiline_black_attached_labels_nudge_up() {
        let mut infos = BTreeMap::new();
        infos.insert(
            "n1".to_string(),
            PreviewLabelInfo {
                layout: Some("attached-group-above".to_string()),
                world_box: None,
                simple_single_run: false,
                line_count: 2,
            },
        );
        let context = PreviewLabelContext { infos };
        let runs = vec![chemsema_engine::LabelRun {
            text: "O2".to_string(),
            fill: Some("#000000".to_string()),
            ..Default::default()
        }];
        assert_eq!(
            preview_attached_label_replay_default_family_y_nudge_px(
                Some("n1"),
                &runs,
                Some("#000000"),
                Some("start"),
                Some(&context),
            ),
            Some(-3.0)
        );
    }
}
