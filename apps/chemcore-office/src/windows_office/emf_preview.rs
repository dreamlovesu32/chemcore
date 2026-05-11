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
use windows_sys::Win32::Foundation::{GlobalFree, COLORREF, HGLOBAL, POINT, RECT, SIZE};
use windows_sys::Win32::Globalization::WideCharToMultiByte;
use windows_sys::Win32::Graphics::Gdi::{
    BeginPath, CloseEnhMetaFile, CloseFigure, CloseMetaFile, CreateEnhMetaFileW, CreateFontW,
    CreateMetaFileW, CreatePen, CreateSolidBrush, DeleteEnhMetaFile, DeleteMetaFile, DeleteObject,
    Ellipse, EndPath, ExtCreatePen, FillPath, GetEnhMetaFileBits, GetMetaFileBitsEx,
    GetStockObject, GetTextExtentPoint32W, LineTo, MoveToEx, PolyBezier, PolyBezierTo, Polygon,
    Polyline, Rectangle, RestoreDC, SaveDC, SelectClipPath, SelectObject, SetBkMode, SetMapMode,
    SetMiterLimit, SetPolyFillMode, SetTextAlign, SetTextColor, SetViewportExtEx, SetWindowExtEx,
    StretchDIBits, StrokePath, TextOutA, TextOutW, ALTERNATE, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BS_SOLID, DIB_RGB_COLORS, HDC, HGDIOBJ, LOGBRUSH, MM_ANISOTROPIC, NULL_BRUSH, NULL_PEN,
    PS_DASH, PS_ENDCAP_FLAT, PS_ENDCAP_ROUND, PS_ENDCAP_SQUARE, PS_GEOMETRIC, PS_JOIN_BEVEL,
    PS_JOIN_MITER, PS_JOIN_ROUND, PS_SOLID, PS_USERSTYLE, RGN_AND, SRCCOPY, TA_BASELINE, TA_LEFT,
    TRANSPARENT,
};
use windows_sys::Win32::System::Com::DVASPECT_CONTENT;
use windows_sys::Win32::System::DataExchange::METAFILEPICT;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::Ole::{CF_ENHMETAFILE, CF_METAFILEPICT};

use super::{
    wide_null, OleObjectPayload, DOCUMENT_DISPLAY_NAME, DV_E_FORMATETC,
    EMF_LOGICAL_UNITS_PER_CSS_PX, E_FAIL, E_OUTOFMEMORY, GMEM_MOVEABLE_FLAG, HIMETRIC_PER_CM,
    HIMETRIC_PER_CSS_PX, MIN_OBJECT_EXTENT_HIMETRIC, WMF_PREVIEW_MAX_EXTENT, WORD_A4_BODY_WIDTH_CM,
};

mod renderer;

use renderer::{
    draw_payload_vector_preview, draw_payload_vector_preview_with_source_bounds,
    office_preview_primitive_visible,
};

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
    let width_cm = (bounds[2] - bounds[0]).max(0.0) / PT_PER_CM;
    let height_cm = (bounds[3] - bounds[1]).max(0.0) / PT_PER_CM;
    if !width_cm.is_finite() || !height_cm.is_finite() || width_cm <= 0.0 || height_cm <= 0.0 {
        return None;
    }

    let scale = if width_cm > WORD_A4_BODY_WIDTH_CM {
        WORD_A4_BODY_WIDTH_CM / width_cm
    } else {
        1.0
    };
    let cx = (width_cm * scale * HIMETRIC_PER_CM)
        .round()
        .clamp(MIN_OBJECT_EXTENT_HIMETRIC as f64, i32::MAX as f64) as i32;
    let cy = (height_cm * scale * HIMETRIC_PER_CM)
        .round()
        .clamp(MIN_OBJECT_EXTENT_HIMETRIC as f64, i32::MAX as f64) as i32;
    Some(SIZE { cx, cy })
}

fn visible_payload_bounds(payload: &OleObjectPayload) -> Option<[f64; 4]> {
    if let Some(bounds) = svg_viewbox_bounds(&payload.svg) {
        return Some(bounds);
    }
    let document = parse_document_json(&payload.chemcore_document_json).ok()?;
    let primitives = render_document(&document);
    render_primitives_bounds(
        primitives
            .iter()
            .filter(|primitive| office_preview_primitive_visible(primitive)),
    )
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
        let (frame_bounds, draw_bounds, source_bounds, use_logical_preview_coords) =
            if let Some(primitive_bounds) = visible_payload_bounds(payload) {
                (
                    office_preview_frame_bounds(primitive_bounds),
                    office_preview_logical_bounds(primitive_bounds),
                    Some(primitive_bounds),
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
        let dc = CreateEnhMetaFileW(0 as HDC, null(), &frame_bounds, null());
        if dc.is_null() {
            return Err(E_FAIL);
        }
        if !use_logical_preview_coords {
            SetMapMode(dc, MM_ANISOTROPIC);
            SetWindowExtEx(dc, extent.cx.max(1), extent.cy.max(1), null_mut());
            SetViewportExtEx(dc, extent.cx.max(1), extent.cy.max(1), null_mut());
        }
        if !draw_payload_vector_preview_with_source_bounds(dc, &draw_bounds, payload, source_bounds)
        {
            draw_placeholder_preview(dc, &draw_bounds);
        }
        let metafile = CloseEnhMetaFile(dc);
        if metafile.is_null() {
            return Err(E_FAIL);
        }
        Ok(metafile)
    }
}

fn office_preview_frame_bounds(bounds: [f64; 4]) -> RECT {
    RECT {
        left: css_px_to_himetric(bounds[0]),
        top: css_px_to_himetric(bounds[1]),
        right: css_px_to_himetric(bounds[2]).max(css_px_to_himetric(bounds[0]) + 1),
        bottom: css_px_to_himetric(bounds[3]).max(css_px_to_himetric(bounds[1]) + 1),
    }
}

fn office_preview_logical_bounds(bounds: [f64; 4]) -> RECT {
    RECT {
        left: css_px_to_emf_logical(bounds[0]),
        top: css_px_to_emf_logical(bounds[1]),
        right: css_px_to_emf_logical(bounds[2]).max(css_px_to_emf_logical(bounds[0]) + 1),
        bottom: css_px_to_emf_logical(bounds[3]).max(css_px_to_emf_logical(bounds[1]) + 1),
    }
}

fn css_px_to_himetric(value: f64) -> i32 {
    (value * HIMETRIC_PER_CSS_PX).round() as i32
}

fn css_px_to_emf_logical(value: f64) -> i32 {
    (value * EMF_LOGICAL_UNITS_PER_CSS_PX).round() as i32
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
