use std::f64::consts::PI;
// Windows metafile preview generation for OLE and Office hosts.
//
// This module owns the EMF/WMF/OlePres containers and delegates actual GDI
// drawing to `renderer`, so future ChemDraw-matching work can evolve there
// without touching the COM and storage plumbing in `windows_office.rs`.

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::{null, null_mut};

use chemcore_engine::{
    parse_document_json, render_document, render_primitives_bounds, Point as CorePoint,
    RenderPrimitive, RenderRole, PT_PER_CM,
};
use serde_json::json;
use windows_sys::Win32::Foundation::{GlobalFree, COLORREF, HGLOBAL, POINT, RECT, SIZE};
use windows_sys::Win32::Globalization::WideCharToMultiByte;
use windows_sys::Win32::Graphics::Gdi::{
    BeginPath, CloseEnhMetaFile, CloseFigure, CloseMetaFile, CreateEnhMetaFileW, CreateFontW,
    CreateMetaFileW, CreatePen, CreateSolidBrush, DeleteEnhMetaFile, DeleteMetaFile, DeleteObject,
    Ellipse, EndPath, ExtCreatePen, ExtTextOutW, FillPath, GetEnhMetaFileBits, GetMetaFileBitsEx,
    GetStockObject, GetTextExtentExPointW, GetTextExtentPoint32W, LineTo, MoveToEx, PolyBezier,
    PolyBezierTo, Polygon, Polyline, Rectangle, RestoreDC, SaveDC, SelectClipPath, SelectObject,
    SetBkMode, SetGraphicsMode, SetMapMode, SetMiterLimit, SetPolyFillMode, SetTextAlign,
    SetTextColor, SetViewportExtEx, SetWindowExtEx, SetWorldTransform, StretchDIBits, StrokePath,
    TextOutA, TextOutW, ALTERNATE, ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BS_SOLID, DIB_RGB_COLORS, GM_ADVANCED, HDC, HGDIOBJ, LOGBRUSH, MM_ANISOTROPIC, NULL_BRUSH,
    NULL_PEN, PS_ENDCAP_FLAT, PS_ENDCAP_ROUND, PS_ENDCAP_SQUARE, PS_GEOMETRIC, PS_JOIN_BEVEL,
    PS_JOIN_MITER, PS_JOIN_ROUND, PS_SOLID, PS_USERSTYLE, RGN_AND, SRCCOPY, TA_BASELINE, TA_LEFT,
    TRANSPARENT, XFORM,
};
use windows_sys::Win32::System::Com::DVASPECT_CONTENT;
use windows_sys::Win32::System::DataExchange::METAFILEPICT;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::Ole::{CF_ENHMETAFILE, CF_METAFILEPICT};

use super::{
    wide_null, OleObjectPayload, DOCUMENT_DISPLAY_NAME, DV_E_FORMATETC,
    EMF_LOGICAL_UNITS_PER_CSS_PX, E_FAIL, E_OUTOFMEMORY, GMEM_MOVEABLE_FLAG, HIMETRIC_PER_CM,
    HIMETRIC_PER_CSS_PX, MIN_OBJECT_EXTENT_HIMETRIC, WMF_PREVIEW_MAX_EXTENT,
};

mod renderer;

use renderer::{
    draw_payload_emf_vector_preview_with_source_bounds, draw_payload_vector_preview,
    office_preview_primitive_visible,
};

const CHEMDRAW_HIMETRIC_PER_SVG_PX: f64 = 2540.0 / 240.0;
const CHEMDRAW_EMF_LOGICAL_UNITS_PER_SVG_PX: f64 = 1.0;
const USE_GDIPLUS_DUAL_PREVIEW: bool = true;
const PREVIEW_SOURCE_RIGHT_PADDING_PT: f64 = 0.0;
const ENV_PREVIEW_SOURCE_RIGHT_PADDING_PT: &str = "CHEMCORE_PREVIEW_SOURCE_RIGHT_PADDING_PT";
const ENV_PREVIEW_SOURCE_BOUNDS_SIDES: &str = "CHEMCORE_PREVIEW_SOURCE_BOUNDS_SIDES";
const ENV_PREVIEW_SOURCE_BOUNDS_MODE: &str = "CHEMCORE_PREVIEW_SOURCE_BOUNDS_MODE";
const ENV_PREVIEW_FRAME_BOUNDS_MODE: &str = "CHEMCORE_PREVIEW_FRAME_BOUNDS_MODE";
const ENV_PREVIEW_FRAME_OFFSETS_SVG_PX: &str = "CHEMCORE_PREVIEW_FRAME_OFFSETS_SVG_PX";
const DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX: PreviewFrameOffsetsSvgPx = PreviewFrameOffsetsSvgPx {
    left: 1.0,
    top: 1.0,
    right: 1.0,
    bottom: -1.0,
};

#[derive(Clone, Copy, Debug)]
enum PreviewSourceBoundsMode {
    Current,
    Visible,
    Svg,
    SvgPadRight,
    Union,
    UnionPadRight,
}

#[derive(Clone, Copy, Debug)]
enum PreviewFrameBoundsMode {
    Source,
    Visible,
    MixedSourceBottom,
    MixedVisibleBottom,
}

#[derive(Clone, Copy, Debug)]
struct PreviewSourceBoundsSides {
    left_from_svg: bool,
    top_from_svg: bool,
    right_from_svg: bool,
    bottom_from_svg: bool,
}

#[derive(Clone, Copy, Debug)]
struct PreviewFrameOffsetsSvgPx {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

pub(super) unsafe fn draw_payload_preview(
    dc: HDC,
    bounds: &RECT,
    payload: &OleObjectPayload,
) -> bool {
    renderer::draw_payload_preview(dc, bounds, payload)
}

pub(super) unsafe fn draw_placeholder_preview(dc: HDC, bounds: &RECT) {
    renderer::draw_placeholder_preview(dc, bounds)
}

pub(super) fn extent_himetric_for_payload(payload: &OleObjectPayload) -> Option<SIZE> {
    let bounds = visible_payload_bounds(payload)?;
    let (width_cm, height_cm) = if payload_uses_cdxml_editing_scale(payload) {
        if let Some(cdxml_bounds) = cdxml_root_bounding_box_points(payload) {
            (
                (cdxml_bounds[2] - cdxml_bounds[0]).max(0.0) / PT_PER_CM,
                (cdxml_bounds[3] - cdxml_bounds[1]).max(0.0) / PT_PER_CM,
            )
        } else {
            (
                (bounds[2] - bounds[0]).max(0.0) * CHEMDRAW_HIMETRIC_PER_SVG_PX / HIMETRIC_PER_CM,
                (bounds[3] - bounds[1]).max(0.0) * CHEMDRAW_HIMETRIC_PER_SVG_PX / HIMETRIC_PER_CM,
            )
        }
    } else {
        (
            (bounds[2] - bounds[0]).max(0.0) / PT_PER_CM,
            (bounds[3] - bounds[1]).max(0.0) / PT_PER_CM,
        )
    };
    if !width_cm.is_finite() || !height_cm.is_finite() || width_cm <= 0.0 || height_cm <= 0.0 {
        return None;
    }

    let cx = (width_cm * HIMETRIC_PER_CM)
        .round()
        .clamp(MIN_OBJECT_EXTENT_HIMETRIC as f64, i32::MAX as f64) as i32;
    let cy = (height_cm * HIMETRIC_PER_CM)
        .round()
        .clamp(MIN_OBJECT_EXTENT_HIMETRIC as f64, i32::MAX as f64) as i32;
    Some(SIZE { cx, cy })
}

fn cdxml_root_bounding_box_points(payload: &OleObjectPayload) -> Option<[f64; 4]> {
    let cdxml = payload.cdxml.as_deref()?;
    let start = cdxml.find("<CDXML")?;
    let head_end = cdxml[start..].find('>')? + start;
    let head = &cdxml[start..head_end];
    let marker = "BoundingBox=\"";
    let bbox_start = head.find(marker)? + marker.len();
    let bbox_end = head[bbox_start..].find('"')? + bbox_start;
    let values: Vec<f64> = head[bbox_start..bbox_end]
        .split_ascii_whitespace()
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();
    let [x1, y1, x2, y2] = values.as_slice() else {
        return None;
    };
    valid_bounds([*x1, *y1, *x2, *y2])
}

fn payload_uses_cdxml_editing_scale(payload: &OleObjectPayload) -> bool {
    parse_document_json(&payload.chemcore_document_json)
        .ok()
        .and_then(|document| {
            document
                .document
                .meta
                .pointer("/import/cdxml/editingScale")
                .and_then(serde_json::Value::as_f64)
                .filter(|scale| (*scale - 1.0).abs() > f64::EPSILON)
        })
        .is_some()
}

fn visible_payload_bounds(payload: &OleObjectPayload) -> Option<[f64; 4]> {
    if let Some(primitives) = payload_render_primitives(payload) {
        let primitive_bounds = render_primitives_bounds(
            primitives
                .iter()
                .filter(|primitive| office_preview_primitive_visible(primitive)),
        );
        if let Ok(document) = parse_document_json(&payload.chemcore_document_json) {
            let clipboard_bounds = clipboard_selection_bounds(&document.document.meta);
            match (clipboard_bounds, primitive_bounds) {
                (Some(selection), Some(primitives)) => {
                    return Some(union_bounds(selection, primitives));
                }
                (Some(selection), None) => return Some(selection),
                (None, Some(primitives)) => return Some(primitives),
                (None, None) => {}
            }
        } else if let Some(bounds) = primitive_bounds {
            return Some(bounds);
        }
    } else if let Ok(document) = parse_document_json(&payload.chemcore_document_json) {
        let clipboard_bounds = clipboard_selection_bounds(&document.document.meta);
        let primitives = render_document(&document);
        let primitive_bounds = render_primitives_bounds(
            primitives
                .iter()
                .filter(|primitive| office_preview_primitive_visible(primitive)),
        );
        match (clipboard_bounds, primitive_bounds) {
            (Some(selection), Some(primitives)) => {
                return Some(union_bounds(selection, primitives))
            }
            (Some(selection), None) => return Some(selection),
            (None, Some(primitives)) => return Some(primitives),
            (None, None) => {}
        }
    }
    svg_viewbox_bounds(&payload.svg)
}

fn document_clipboard_bounds(payload: &OleObjectPayload) -> Option<[f64; 4]> {
    let document = parse_document_json(&payload.chemcore_document_json).ok()?;
    clipboard_selection_bounds(&document.document.meta)
}

pub(super) fn preview_source_bounds(payload: &OleObjectPayload) -> Option<[f64; 4]> {
    let right_padding = std::env::var(ENV_PREVIEW_SOURCE_RIGHT_PADDING_PT)
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(PREVIEW_SOURCE_RIGHT_PADDING_PT);
    let visible = visible_payload_bounds(payload).or_else(|| {
        (!payload.svg_was_supplied)
            .then(|| document_clipboard_bounds(payload))
            .flatten()
    });
    let svg = payload
        .svg_was_supplied
        .then(|| svg_viewbox_bounds(&payload.svg))
        .flatten();
    if let Some(sides) = preview_source_bounds_sides_override() {
        return match (visible, svg) {
            (Some(visible), Some(svg)) => Some([
                if sides.left_from_svg {
                    svg[0]
                } else {
                    visible[0]
                },
                if sides.top_from_svg {
                    svg[1]
                } else {
                    visible[1]
                },
                if sides.right_from_svg {
                    svg[2] + right_padding
                } else {
                    visible[2] + right_padding
                },
                if sides.bottom_from_svg {
                    svg[3]
                } else {
                    visible[3]
                },
            ]),
            (Some(visible), None) => Some([
                visible[0],
                visible[1],
                visible[2] + right_padding,
                visible[3],
            ]),
            (None, Some(svg)) => Some([svg[0], svg[1], svg[2] + right_padding, svg[3]]),
            (None, None) => None,
        };
    }
    match preview_source_bounds_mode() {
        PreviewSourceBoundsMode::Current => match (visible, svg) {
            (Some(visible), Some(svg)) => Some([
                visible[0],
                visible[1],
                visible[2].max(svg[2]) + right_padding,
                visible[3],
            ]),
            (Some(visible), None) => Some([
                visible[0],
                visible[1],
                visible[2] + right_padding,
                visible[3],
            ]),
            (None, Some(svg)) => Some([svg[0], svg[1], svg[2] + right_padding, svg[3]]),
            (None, None) => None,
        },
        PreviewSourceBoundsMode::Visible => visible,
        PreviewSourceBoundsMode::Svg => svg,
        PreviewSourceBoundsMode::SvgPadRight => {
            svg.map(|svg| [svg[0], svg[1], svg[2] + right_padding, svg[3]])
        }
        PreviewSourceBoundsMode::Union => match (visible, svg) {
            (Some(visible), Some(svg)) => Some(union_bounds(visible, svg)),
            (Some(visible), None) => Some(visible),
            (None, Some(svg)) => Some(svg),
            (None, None) => None,
        },
        PreviewSourceBoundsMode::UnionPadRight => match (visible, svg) {
            (Some(visible), Some(svg)) => {
                let union = union_bounds(visible, svg);
                Some([union[0], union[1], union[2] + right_padding, union[3]])
            }
            (Some(visible), None) => Some([
                visible[0],
                visible[1],
                visible[2] + right_padding,
                visible[3],
            ]),
            (None, Some(svg)) => Some([svg[0], svg[1], svg[2] + right_padding, svg[3]]),
            (None, None) => None,
        },
    }
}

fn preview_frame_source_bounds(payload: &OleObjectPayload) -> Option<[f64; 4]> {
    let visible = visible_payload_bounds(payload);
    let source = preview_source_bounds(payload);
    match preview_frame_bounds_mode() {
        PreviewFrameBoundsMode::Source => source.or(visible),
        PreviewFrameBoundsMode::Visible => visible.or(source),
        PreviewFrameBoundsMode::MixedSourceBottom => match (visible, source) {
            (Some(visible), Some(source)) => Some([visible[0], visible[1], source[2], source[3]]),
            (Some(visible), None) => Some(visible),
            (None, Some(source)) => Some(source),
            (None, None) => None,
        },
        PreviewFrameBoundsMode::MixedVisibleBottom => match (visible, source) {
            (Some(visible), Some(source)) => Some([visible[0], visible[1], source[2], visible[3]]),
            (Some(visible), None) => Some(visible),
            (None, Some(source)) => Some(source),
            (None, None) => None,
        },
    }
}

pub(super) fn preview_bounds_debug_report(
    payload: &OleObjectPayload,
    extent: SIZE,
) -> serde_json::Value {
    let use_chemdraw_units = payload_uses_cdxml_editing_scale(payload);
    let visible_bounds = visible_payload_bounds(payload);
    let svg_bounds = payload
        .svg_was_supplied
        .then(|| svg_viewbox_bounds(&payload.svg))
        .flatten();
    let source_bounds = preview_source_bounds(payload);
    let frame_source_bounds = preview_frame_source_bounds(payload);
    let source_bounds_mode = preview_source_bounds_mode();
    let frame_bounds_mode = preview_frame_bounds_mode();
    let source_bounds_sides_override = std::env::var(ENV_PREVIEW_SOURCE_BOUNDS_SIDES).ok();
    let right_padding = std::env::var(ENV_PREVIEW_SOURCE_RIGHT_PADDING_PT)
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(PREVIEW_SOURCE_RIGHT_PADDING_PT);
    let (frame_bounds, draw_bounds, use_logical_preview_coords) =
        if let Some(visible) = visible_bounds {
            let draw_source_bounds = source_bounds.unwrap_or(visible);
            let frame_source_bounds = frame_source_bounds.unwrap_or(draw_source_bounds);
            (
                office_preview_frame_bounds(frame_source_bounds, use_chemdraw_units),
                office_preview_logical_bounds(draw_source_bounds, use_chemdraw_units),
                true,
            )
        } else {
            let bounds = RECT {
                left: 0,
                top: 0,
                right: extent.cx.max(1),
                bottom: extent.cy.max(1),
            };
            (bounds, bounds, false)
        };
    json!({
        "useCdxmlEditingScale": use_chemdraw_units,
        "sourceBoundsMode": format!("{source_bounds_mode:?}"),
        "frameBoundsMode": format!("{frame_bounds_mode:?}"),
        "sourceBoundsSidesOverride": source_bounds_sides_override,
        "frameOffsetsSvgPx": preview_frame_offsets_svg_px().map(|offsets| json!({
            "left": offsets.left,
            "top": offsets.top,
            "right": offsets.right,
            "bottom": offsets.bottom,
        })),
        "extentHimetric": {
            "width": extent.cx,
            "height": extent.cy,
        },
        "rightPaddingPt": right_padding,
        "visibleBoundsSvgPx": visible_bounds,
        "svgViewBoxBoundsSvgPx": svg_bounds,
        "sourceBoundsSvgPx": source_bounds,
        "frameSourceBoundsSvgPx": frame_source_bounds,
        "useLogicalPreviewCoords": use_logical_preview_coords,
        "frameBoundsHimetric": rect_debug_json(frame_bounds),
        "drawBoundsLogical": rect_debug_json(draw_bounds),
    })
}

fn payload_render_primitives(payload: &OleObjectPayload) -> Option<Vec<RenderPrimitive>> {
    payload
        .render_list_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Vec<RenderPrimitive>>(json).ok())
        .filter(|primitives| !primitives.is_empty())
}

fn clipboard_selection_bounds(meta: &serde_json::Value) -> Option<[f64; 4]> {
    let value = meta.pointer("/clipboard/selectionBounds")?;
    if let Some(array) = value.as_array() {
        if array.len() == 4 {
            return valid_bounds([
                array[0].as_f64()?,
                array[1].as_f64()?,
                array[2].as_f64()?,
                array[3].as_f64()?,
            ]);
        }
    }
    valid_bounds([
        value.get("minX")?.as_f64()?,
        value.get("minY")?.as_f64()?,
        value.get("maxX")?.as_f64()?,
        value.get("maxY")?.as_f64()?,
    ])
}

fn valid_bounds(bounds: [f64; 4]) -> Option<[f64; 4]> {
    let [min_x, min_y, max_x, max_y] = bounds;
    if min_x.is_finite()
        && min_y.is_finite()
        && max_x.is_finite()
        && max_y.is_finite()
        && max_x > min_x
        && max_y > min_y
    {
        Some(bounds)
    } else {
        None
    }
}

fn union_bounds(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

fn svg_viewbox_bounds(svg: &str) -> Option<[f64; 4]> {
    let marker = "viewBox=\"";
    let start = svg.find(marker)? + marker.len();
    let end = svg[start..].find('"')? + start;
    let values: Vec<f64> = svg[start..end]
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();
    let [x, y, width, height] = values.as_slice() else {
        return None;
    };
    if !x.is_finite()
        || !y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || *width <= 0.0
        || *height <= 0.0
    {
        return None;
    }
    Some([*x, *y, *x + *width, *y + *height])
}

fn wmf_preview_canvas_size(extent: SIZE) -> SIZE {
    let source_width = extent.cx.max(1) as f64;
    let source_height = extent.cy.max(1) as f64;
    let scale = (WMF_PREVIEW_MAX_EXTENT as f64 / source_width.max(source_height)).min(1.0);
    let width = (source_width * scale)
        .round()
        .clamp(1.0, WMF_PREVIEW_MAX_EXTENT as f64) as i32;
    let height = (source_height * scale)
        .round()
        .clamp(1.0, WMF_PREVIEW_MAX_EXTENT as f64) as i32;
    SIZE {
        cx: width,
        cy: height,
    }
}

pub(super) fn hglobal_for_metafile_pict(
    payload: &OleObjectPayload,
    extent: SIZE,
) -> Result<HGLOBAL, i32> {
    unsafe {
        let metafile = windows_metafile_for_payload(payload, extent)?;

        let handle = GlobalAlloc(GMEM_MOVEABLE_FLAG, std::mem::size_of::<METAFILEPICT>());
        if handle.is_null() {
            DeleteMetaFile(metafile);
            return Err(E_OUTOFMEMORY);
        }
        let target = GlobalLock(handle).cast::<METAFILEPICT>();
        if target.is_null() {
            GlobalFree(handle);
            DeleteMetaFile(metafile);
            return Err(E_FAIL);
        }
        (*target).mm = MM_ANISOTROPIC;
        (*target).xExt = extent.cx;
        (*target).yExt = extent.cy;
        (*target).hMF = metafile;
        GlobalUnlock(handle);
        Ok(handle)
    }
}

unsafe fn windows_metafile_for_payload(
    payload: &OleObjectPayload,
    extent: SIZE,
) -> Result<*mut c_void, i32> {
    let canvas = wmf_preview_canvas_size(extent);
    let metafile_dc = CreateMetaFileW(null());
    if metafile_dc.is_null() {
        return Err(E_FAIL);
    }
    SetMapMode(metafile_dc, MM_ANISOTROPIC);
    SetWindowExtEx(metafile_dc, canvas.cx, canvas.cy, null_mut());
    SetViewportExtEx(metafile_dc, canvas.cx, canvas.cy, null_mut());
    let bounds = RECT {
        left: 0,
        top: 0,
        right: canvas.cx,
        bottom: canvas.cy,
    };
    if !draw_payload_vector_preview(metafile_dc, &bounds, payload) {
        draw_placeholder_preview(metafile_dc, &bounds);
    }
    let metafile = CloseMetaFile(metafile_dc);
    if metafile.is_null() {
        return Err(E_FAIL);
    }
    Ok(metafile)
}

pub(super) fn enhanced_metafile_for_payload(
    payload: &OleObjectPayload,
    extent: SIZE,
) -> Result<*mut c_void, i32> {
    unsafe {
        let use_chemdraw_units = payload_uses_cdxml_editing_scale(payload);
        let (frame_bounds, draw_bounds, source_bounds, use_logical_preview_coords) =
            if let Some(visible_bounds) = visible_payload_bounds(payload) {
                let draw_source_bounds = preview_source_bounds(payload).unwrap_or(visible_bounds);
                let frame_source_bounds =
                    preview_frame_source_bounds(payload).unwrap_or(draw_source_bounds);
                (
                    office_preview_frame_bounds(frame_source_bounds, use_chemdraw_units),
                    office_preview_logical_bounds(draw_source_bounds, use_chemdraw_units),
                    Some(draw_source_bounds),
                    true,
                )
            } else {
                let bounds = RECT {
                    left: 0,
                    top: 0,
                    right: extent.cx.max(1),
                    bottom: extent.cy.max(1),
                };
                (bounds, bounds, None, false)
            };
        // Default to EMF+ dual recording for smooth bond geometry. We still keep
        // a shared EMF playback path in IViewObject so Word's live redraw stays
        // closer to the packaged preview, while pure GDI remains available via env
        // for targeted investigation.
        let force_gdiplus_dual = std::env::var_os("CHEMCORE_OFFICE_GDIPLUS_DUAL").is_some();
        let disable_gdiplus_dual =
            std::env::var_os("CHEMCORE_OFFICE_DISABLE_GDIPLUS_DUAL").is_some();
        if (USE_GDIPLUS_DUAL_PREVIEW || force_gdiplus_dual)
            && !disable_gdiplus_dual
            && use_logical_preview_coords
        {
            if let Some(metafile) = renderer::enhanced_metafile_gdiplus_dual_preview(
                &frame_bounds,
                &draw_bounds,
                payload,
                source_bounds,
            ) {
                return Ok(metafile);
            }
        }
        let dc = CreateEnhMetaFileW(0 as HDC, null(), &frame_bounds, null());
        if dc.is_null() {
            return Err(E_FAIL);
        }
        if !use_logical_preview_coords {
            SetMapMode(dc, MM_ANISOTROPIC);
            SetWindowExtEx(dc, extent.cx.max(1), extent.cy.max(1), null_mut());
            SetViewportExtEx(dc, extent.cx.max(1), extent.cy.max(1), null_mut());
        }
        if !draw_payload_emf_vector_preview_with_source_bounds(
            dc,
            &draw_bounds,
            payload,
            source_bounds,
        ) {
            draw_placeholder_preview(dc, &draw_bounds);
        }
        let metafile = CloseEnhMetaFile(dc);
        if metafile.is_null() {
            return Err(E_FAIL);
        }
        Ok(metafile)
    }
}

fn office_preview_frame_bounds(bounds: [f64; 4], use_chemdraw_units: bool) -> RECT {
    let bounds = if let Some(offsets) = preview_frame_offsets_svg_px() {
        [
            bounds[0] + offsets.left,
            bounds[1] + offsets.top,
            bounds[2] + offsets.right,
            bounds[3] + offsets.bottom,
        ]
    } else {
        bounds
    };
    let scale = if use_chemdraw_units {
        CHEMDRAW_HIMETRIC_PER_SVG_PX
    } else {
        HIMETRIC_PER_CSS_PX
    };
    RECT {
        left: scaled_to_i32(bounds[0], scale),
        top: scaled_to_i32(bounds[1], scale),
        right: scaled_to_i32(bounds[2], scale).max(scaled_to_i32(bounds[0], scale) + 1),
        bottom: scaled_to_i32(bounds[3], scale).max(scaled_to_i32(bounds[1], scale) + 1),
    }
}

fn office_preview_logical_bounds(bounds: [f64; 4], use_chemdraw_units: bool) -> RECT {
    let scale = if use_chemdraw_units {
        CHEMDRAW_EMF_LOGICAL_UNITS_PER_SVG_PX
    } else {
        EMF_LOGICAL_UNITS_PER_CSS_PX
    };
    RECT {
        left: scaled_to_i32(bounds[0], scale),
        top: scaled_to_i32(bounds[1], scale),
        right: scaled_to_i32(bounds[2], scale).max(scaled_to_i32(bounds[0], scale) + 1),
        bottom: scaled_to_i32(bounds[3], scale).max(scaled_to_i32(bounds[1], scale) + 1),
    }
}

fn scaled_to_i32(value: f64, scale: f64) -> i32 {
    (value * scale).round() as i32
}

fn rect_debug_json(rect: RECT) -> serde_json::Value {
    json!({
        "left": rect.left,
        "top": rect.top,
        "right": rect.right,
        "bottom": rect.bottom,
        "width": rect.right - rect.left,
        "height": rect.bottom - rect.top,
    })
}

fn preview_source_bounds_mode() -> PreviewSourceBoundsMode {
    match std::env::var(ENV_PREVIEW_SOURCE_BOUNDS_MODE)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("current") => PreviewSourceBoundsMode::Current,
        Some("visible") => PreviewSourceBoundsMode::Visible,
        Some("svg") => PreviewSourceBoundsMode::Svg,
        Some("svgpad") => PreviewSourceBoundsMode::SvgPadRight,
        Some("union") => PreviewSourceBoundsMode::Union,
        Some("unionpad") => PreviewSourceBoundsMode::UnionPadRight,
        _ => PreviewSourceBoundsMode::SvgPadRight,
    }
}

fn preview_frame_bounds_mode() -> PreviewFrameBoundsMode {
    match std::env::var(ENV_PREVIEW_FRAME_BOUNDS_MODE)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("visible") => PreviewFrameBoundsMode::Visible,
        Some("mixed") | Some("mixedsourcebottom") | Some("visible-source-bottom") => {
            PreviewFrameBoundsMode::MixedSourceBottom
        }
        Some("mixed-tight-bottom") | Some("mixedvisiblebottom") | Some("visible-source-right") => {
            PreviewFrameBoundsMode::MixedVisibleBottom
        }
        _ => PreviewFrameBoundsMode::Source,
    }
}

fn preview_source_bounds_sides_override() -> Option<PreviewSourceBoundsSides> {
    let raw = std::env::var(ENV_PREVIEW_SOURCE_BOUNDS_SIDES).ok()?;
    let parts: Vec<&str> = raw
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();
    let [left, top, right, bottom] = parts.as_slice() else {
        return None;
    };
    Some(PreviewSourceBoundsSides {
        left_from_svg: matches!((*left).to_ascii_lowercase().as_str(), "svg" | "s"),
        top_from_svg: matches!((*top).to_ascii_lowercase().as_str(), "svg" | "s"),
        right_from_svg: matches!((*right).to_ascii_lowercase().as_str(), "svg" | "s"),
        bottom_from_svg: matches!((*bottom).to_ascii_lowercase().as_str(), "svg" | "s"),
    })
}

fn preview_frame_offsets_svg_px() -> Option<PreviewFrameOffsetsSvgPx> {
    let Some(raw) = std::env::var(ENV_PREVIEW_FRAME_OFFSETS_SVG_PX).ok() else {
        return Some(DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX);
    };
    if raw.trim().eq_ignore_ascii_case("none") {
        return None;
    }
    let parts: Vec<&str> = raw
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();
    let [left, top, right, bottom] = parts.as_slice() else {
        return Some(DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX);
    };
    Some(PreviewFrameOffsetsSvgPx {
        left: left
            .parse::<f64>()
            .ok()
            .unwrap_or(DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX.left),
        top: top
            .parse::<f64>()
            .ok()
            .unwrap_or(DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX.top),
        right: right
            .parse::<f64>()
            .ok()
            .unwrap_or(DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX.right),
        bottom: bottom
            .parse::<f64>()
            .ok()
            .unwrap_or(DEFAULT_PREVIEW_FRAME_OFFSETS_SVG_PX.bottom),
    })
}

pub(super) fn ole_presentation_stream_for_payload(
    payload: &OleObjectPayload,
    extent: SIZE,
    format: u16,
) -> Result<Vec<u8>, i32> {
    let data = match format {
        CF_METAFILEPICT => windows_metafile_bits_for_payload(payload, extent)?,
        CF_ENHMETAFILE => enhanced_metafile_bits_for_payload(payload, extent)?,
        _ => return Err(DV_E_FORMATETC),
    };
    Ok(ole_presentation_stream_bytes(format, extent, &data))
}

fn ole_presentation_stream_bytes(format: u16, extent: SIZE, data: &[u8]) -> Vec<u8> {
    let mut out =
        Vec::with_capacity(40 + data.len() + if format == CF_METAFILEPICT { 18 } else { 0 });
    write_u32_le(&mut out, 0xFFFF_FFFF);
    write_u32_le(&mut out, format as u32);
    write_u32_le(&mut out, 4);
    write_u32_le(&mut out, DVASPECT_CONTENT);
    write_u32_le(&mut out, 0xFFFF_FFFF);
    write_u32_le(&mut out, 2);
    write_u32_le(&mut out, 0);
    write_u32_le(&mut out, extent.cx.max(1) as u32);
    write_u32_le(&mut out, extent.cy.max(1) as u32);
    write_u32_le(&mut out, data.len().min(u32::MAX as usize) as u32);
    out.extend_from_slice(data);
    if format == CF_METAFILEPICT {
        out.extend_from_slice(&[0u8; 18]);
    }
    out
}

fn write_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn windows_metafile_bits_for_payload(
    payload: &OleObjectPayload,
    extent: SIZE,
) -> Result<Vec<u8>, i32> {
    unsafe {
        let metafile = windows_metafile_for_payload(payload, extent)?;
        let size = GetMetaFileBitsEx(metafile, 0, null_mut());
        if size == 0 {
            DeleteMetaFile(metafile);
            return Err(E_FAIL);
        }
        let mut bytes = vec![0u8; size as usize];
        let written = GetMetaFileBitsEx(metafile, size, bytes.as_mut_ptr().cast::<c_void>());
        DeleteMetaFile(metafile);
        if written == 0 {
            return Err(E_FAIL);
        }
        bytes.truncate(written as usize);
        Ok(bytes)
    }
}

pub(super) fn enhanced_metafile_bits_for_payload(
    payload: &OleObjectPayload,
    extent: SIZE,
) -> Result<Vec<u8>, i32> {
    unsafe {
        let metafile = enhanced_metafile_for_payload(payload, extent)?;
        let size = GetEnhMetaFileBits(metafile, 0, null_mut());
        if size == 0 {
            DeleteEnhMetaFile(metafile);
            return Err(E_FAIL);
        }
        let mut bytes = vec![0u8; size as usize];
        let written = GetEnhMetaFileBits(metafile, size, bytes.as_mut_ptr());
        DeleteEnhMetaFile(metafile);
        if written == 0 {
            return Err(E_FAIL);
        }
        bytes.truncate(written as usize);
        Ok(bytes)
    }
}
