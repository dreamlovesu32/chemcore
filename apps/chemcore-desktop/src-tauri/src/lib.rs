use base64::Engine as _;
use chemcore_desktop_service::{
    DesktopDocumentService, DesktopOpenedDocument, DesktopRecentFile, DesktopSavedDocument,
    SessionId,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

const EVENT_DESKTOP_MENU: &str = "chemcore-desktop-menu";
const EVENT_DESKTOP_OPEN_PATHS: &str = "chemcore-desktop-open-paths";
const FORMAT_CHEMCORE_FRAGMENT: &str = "Chemcore Clipboard Fragment";
const FORMAT_CHEMCORE_DOCUMENT_JSON: &str = "Chemcore Document JSON";
const FORMAT_CHEMDRAW_INTERCHANGE: &str = "ChemDraw Interchange Format";
const FORMAT_CDXML_MIME: &str = "chemical/x-cdxml";
const FORMAT_SVG_MIME: &str = "image/svg+xml";
const FORMAT_SVG: &str = "SVG";
const GMEM_MOVEABLE_FLAG: u32 = 0x0002;

struct DesktopState {
    service: Mutex<DesktopDocumentService>,
    pending_open_paths: Mutex<Vec<String>>,
}

impl DesktopState {
    fn new() -> Self {
        Self {
            service: Mutex::new(DesktopDocumentService::new()),
            pending_open_paths: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopMenuPayload {
    command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopOpenPathsPayload {
    paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeClipboardWritePayload {
    chemcore_fragment_json: Option<String>,
    chemcore_document_json: Option<String>,
    cdxml: Option<String>,
    svg: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeClipboardReadPayload {
    chemcore_fragment_json: Option<String>,
    chemcore_document_json: Option<String>,
    cdxml: Option<String>,
    svg: Option<String>,
    text: Option<String>,
}

#[tauri::command]
fn desktop_engine_create(state: tauri::State<'_, DesktopState>) -> Result<SessionId, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    Ok(service.create_session())
}

#[tauri::command]
fn desktop_engine_free(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    Ok(service.free_session(session_id))
}

#[tauri::command]
fn desktop_engine_load_document_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    json: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.load_document_json(session_id, &json)
}

#[tauri::command]
fn desktop_engine_load_document_cdxml(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    cdxml: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.load_document_cdxml(session_id, &cdxml)
}

#[tauri::command]
fn desktop_engine_document_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_json(session_id)
}

#[tauri::command]
fn desktop_engine_state_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.state_json(session_id)
}

#[tauri::command]
fn desktop_engine_render_list_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.render_list_json(session_id)
}

#[tauri::command]
fn desktop_engine_render_bounds_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    scope: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.render_bounds_json(session_id, &scope)
}

#[tauri::command]
fn desktop_engine_document_cdxml(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_cdxml(session_id)
}

#[tauri::command]
fn desktop_engine_document_svg(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_svg(session_id)
}

#[tauri::command]
fn desktop_engine_document_colors_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_colors_json(session_id)
}

#[tauri::command]
fn desktop_color_choose(
    initial_color: String,
    custom_colors: Vec<String>,
) -> Result<Option<String>, String> {
    native_choose_color(&initial_color, &custom_colors)
}

#[tauri::command]
fn desktop_file_choose_open() -> Result<Option<String>, String> {
    Ok(document_file_dialog()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn desktop_file_choose_save(suggested_name: String) -> Result<Option<String>, String> {
    Ok(document_file_dialog()
        .set_file_name(suggested_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn desktop_file_choose_export_save(
    suggested_name: String,
    extension: String,
) -> Result<Option<String>, String> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    let dialog = match extension.as_str() {
        "pdf" => rfd::FileDialog::new().add_filter("PDF preview", &["pdf"]),
        "emf" => rfd::FileDialog::new().add_filter("Enhanced Metafile", &["emf"]),
        "svg" => rfd::FileDialog::new().add_filter("SVG", &["svg"]),
        _ => document_file_dialog(),
    };
    Ok(dialog
        .set_file_name(suggested_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn desktop_file_read_path(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    path: String,
) -> Result<DesktopOpenedDocument, String> {
    let opened = {
        let mut service = state.service.lock().map_err(|error| error.to_string())?;
        service.read_document_file(path)?
    };
    refresh_native_menu(&app);
    Ok(opened)
}

#[tauri::command]
fn desktop_file_write_path(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    path: String,
    content: String,
    format: Option<String>,
) -> Result<DesktopSavedDocument, String> {
    let saved = {
        let mut service = state.service.lock().map_err(|error| error.to_string())?;
        service.write_document_file(path, &content, format.as_deref())?
    };
    refresh_native_menu(&app);
    Ok(saved)
}

#[tauri::command]
fn desktop_file_write_base64(
    app: tauri::AppHandle,
    path: String,
    content_base64: String,
) -> Result<DesktopSavedDocument, String> {
    let path = normalize_output_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create directory {}: {error}", parent.display()))?;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content_base64)
        .map_err(|error| format!("Failed to decode export data: {error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
    refresh_native_menu(&app);
    Ok(DesktopSavedDocument {
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("export")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        format: path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("binary")
            .to_ascii_lowercase(),
    })
}

#[tauri::command]
fn desktop_file_export_emf(
    path: String,
    render_list_json: String,
    bounds_json: String,
) -> Result<DesktopSavedDocument, String> {
    let path = normalize_output_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create directory {}: {error}", parent.display()))?;
    }
    write_emf_preview(&path, &render_list_json, &bounds_json)?;
    Ok(DesktopSavedDocument {
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("preview.emf")
            .to_string(),
        path: path.to_string_lossy().to_string(),
        format: "emf".to_string(),
    })
}

#[tauri::command]
fn desktop_recent_files(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<DesktopRecentFile>, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    Ok(service.recent_files())
}

#[tauri::command]
fn desktop_clear_recent_files(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
) -> Result<(), String> {
    {
        let mut service = state.service.lock().map_err(|error| error.to_string())?;
        service.clear_recent_files()?;
    }
    refresh_native_menu(&app);
    Ok(())
}

#[tauri::command]
fn desktop_take_startup_open_paths(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<String>, String> {
    let mut paths = state
        .pending_open_paths
        .lock()
        .map_err(|error| error.to_string())?;
    Ok(std::mem::take(&mut *paths))
}

#[tauri::command]
fn desktop_clipboard_write(payload: NativeClipboardWritePayload) -> Result<(), String> {
    native_clipboard_write(payload)
}

#[tauri::command]
fn desktop_clipboard_read() -> Result<NativeClipboardReadPayload, String> {
    native_clipboard_read()
}

#[cfg(target_os = "windows")]
fn native_clipboard_write(payload: NativeClipboardWritePayload) -> Result<(), String> {
    use windows_sys::Win32::System::DataExchange::{
        EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open Windows clipboard.".to_string());
        }
    }
    let _guard = ClipboardCloseGuard;

    unsafe {
        if EmptyClipboard() == 0 {
            return Err("Failed to clear Windows clipboard.".to_string());
        }
    }

    let mut wrote = false;
    for (format_name, value) in [
        (FORMAT_CHEMCORE_FRAGMENT, payload.chemcore_fragment_json.as_deref()),
        (
            FORMAT_CHEMCORE_DOCUMENT_JSON,
            payload.chemcore_document_json.as_deref(),
        ),
        (FORMAT_CHEMDRAW_INTERCHANGE, payload.cdxml.as_deref()),
        (FORMAT_CDXML_MIME, payload.cdxml.as_deref()),
        (FORMAT_SVG_MIME, payload.svg.as_deref()),
        (FORMAT_SVG, payload.svg.as_deref()),
    ] {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            continue;
        };
        let format = unsafe { RegisterClipboardFormatW(wide_null(format_name).as_ptr()) };
        if format == 0 {
            return Err(format!("Failed to register clipboard format {format_name}."));
        }
        unsafe {
            write_clipboard_utf8(format, value, GlobalAlloc, GlobalLock, GlobalUnlock, SetClipboardData)?;
        }
        wrote = true;
    }

    let text = payload
        .text
        .as_deref()
        .or(payload.cdxml.as_deref())
        .filter(|value| !value.is_empty());
    if let Some(text) = text {
        unsafe {
            write_clipboard_utf16(13, text, GlobalAlloc, GlobalLock, GlobalUnlock, SetClipboardData)?;
        }
        wrote = true;
    }

    if wrote {
        Ok(())
    } else {
        Err("Clipboard payload is empty.".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn native_clipboard_write(_payload: NativeClipboardWritePayload) -> Result<(), String> {
    Err("Native clipboard is only implemented on Windows.".to_string())
}

#[cfg(target_os = "windows")]
fn native_clipboard_read() -> Result<NativeClipboardReadPayload, String> {
    use windows_sys::Win32::System::DataExchange::{
        GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open Windows clipboard.".to_string());
        }
    }
    let _guard = ClipboardCloseGuard;

    let read_registered = |format_name: &str| -> Result<Option<String>, String> {
        let format = unsafe { RegisterClipboardFormatW(wide_null(format_name).as_ptr()) };
        if format == 0 {
            return Ok(None);
        }
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return Ok(None);
            }
            let handle = GetClipboardData(format);
            read_clipboard_utf8(handle, GlobalLock, GlobalSize, GlobalUnlock)
        }
    };

    let text = unsafe {
        if IsClipboardFormatAvailable(13) == 0 {
            None
        } else {
            let handle = GetClipboardData(13);
            read_clipboard_utf16(handle, GlobalLock, GlobalSize, GlobalUnlock)?
        }
    };

    Ok(NativeClipboardReadPayload {
        chemcore_fragment_json: read_registered(FORMAT_CHEMCORE_FRAGMENT)?,
        chemcore_document_json: read_registered(FORMAT_CHEMCORE_DOCUMENT_JSON)?,
        cdxml: read_registered(FORMAT_CHEMDRAW_INTERCHANGE)?
            .or(read_registered(FORMAT_CDXML_MIME)?),
        svg: read_registered(FORMAT_SVG_MIME)?.or(read_registered(FORMAT_SVG)?),
        text,
    })
}

#[cfg(not(target_os = "windows"))]
fn native_clipboard_read() -> Result<NativeClipboardReadPayload, String> {
    Err("Native clipboard is only implemented on Windows.".to_string())
}

#[cfg(target_os = "windows")]
fn native_choose_color(
    initial_color: &str,
    custom_colors: &[String],
) -> Result<Option<String>, String> {
    use windows_sys::Win32::UI::Controls::Dialogs::{
        ChooseColorW, CommDlgExtendedError, CHOOSECOLORW, CC_FULLOPEN, CC_RGBINIT,
    };

    let mut custom_color_refs = [0u32; 16];
    for (index, color) in custom_colors.iter().take(custom_color_refs.len()).enumerate() {
        custom_color_refs[index] = colorref_from_hex(color).unwrap_or(0x00ff_ffff);
    }

    let mut choose = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: std::ptr::null_mut(),
        hInstance: std::ptr::null_mut(),
        rgbResult: colorref_from_hex(initial_color).unwrap_or(0),
        lpCustColors: custom_color_refs.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        lCustData: 0,
        lpfnHook: None,
        lpTemplateName: std::ptr::null(),
    };

    let picked = unsafe { ChooseColorW(&mut choose) };
    if picked != 0 {
        return Ok(Some(hex_from_colorref(choose.rgbResult)));
    }
    let error = unsafe { CommDlgExtendedError() };
    if error == 0 {
        Ok(None)
    } else {
        Err(format!("Windows color dialog failed with error code {error}."))
    }
}

#[cfg(not(target_os = "windows"))]
fn native_choose_color(
    _initial_color: &str,
    _custom_colors: &[String],
) -> Result<Option<String>, String> {
    Err("Native color dialog is only implemented on Windows.".to_string())
}

fn colorref_from_hex(color: &str) -> Option<u32> {
    let raw = color.trim().trim_start_matches('#');
    if raw.len() < 6 || !raw[..6].chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = u32::from_str_radix(&raw[..6], 16).ok()?;
    Some(rgb_to_colorref(rgb))
}

fn hex_from_colorref(colorref: u32) -> String {
    let red = colorref & 0xff;
    let green = (colorref >> 8) & 0xff;
    let blue = (colorref >> 16) & 0xff;
    format!("#{red:02x}{green:02x}{blue:02x}")
}

#[cfg(target_os = "windows")]
struct ClipboardCloseGuard;

#[cfg(target_os = "windows")]
impl Drop for ClipboardCloseGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::DataExchange::CloseClipboard();
        }
    }
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
unsafe fn write_clipboard_utf8(
    format: u32,
    value: &str,
    global_alloc: unsafe extern "system" fn(u32, usize) -> *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
    set_clipboard_data:
        unsafe extern "system" fn(u32, *mut std::ffi::c_void) -> *mut std::ffi::c_void,
) -> Result<(), String> {
    let bytes = value.as_bytes();
    let handle = global_alloc(GMEM_MOVEABLE_FLAG, bytes.len() + 1);
    if handle.is_null() {
        return Err("Failed to allocate clipboard memory.".to_string());
    }
    let target = global_lock(handle) as *mut u8;
    if target.is_null() {
        return Err("Failed to lock clipboard memory.".to_string());
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
    *target.add(bytes.len()) = 0;
    global_unlock(handle);
    if set_clipboard_data(format, handle).is_null() {
        return Err("Failed to set clipboard data.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn write_clipboard_utf16(
    format: u32,
    value: &str,
    global_alloc: unsafe extern "system" fn(u32, usize) -> *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
    set_clipboard_data:
        unsafe extern "system" fn(u32, *mut std::ffi::c_void) -> *mut std::ffi::c_void,
) -> Result<(), String> {
    let wide = wide_null(value);
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    let handle = global_alloc(GMEM_MOVEABLE_FLAG, byte_len);
    if handle.is_null() {
        return Err("Failed to allocate clipboard memory.".to_string());
    }
    let target = global_lock(handle) as *mut u16;
    if target.is_null() {
        return Err("Failed to lock clipboard memory.".to_string());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
    global_unlock(handle);
    if set_clipboard_data(format, handle).is_null() {
        return Err("Failed to set clipboard data.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn read_clipboard_utf8(
    handle: *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_size: unsafe extern "system" fn(*mut std::ffi::c_void) -> usize,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
) -> Result<Option<String>, String> {
    if handle.is_null() {
        return Ok(None);
    }
    let size = global_size(handle);
    if size == 0 {
        return Ok(None);
    }
    let source = global_lock(handle) as *const u8;
    if source.is_null() {
        return Ok(None);
    }
    let slice = std::slice::from_raw_parts(source, size);
    let len = slice.iter().position(|byte| *byte == 0).unwrap_or(slice.len());
    let text = String::from_utf8_lossy(&slice[..len]).to_string();
    global_unlock(handle);
    Ok(Some(text))
}

#[cfg(target_os = "windows")]
unsafe fn read_clipboard_utf16(
    handle: *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_size: unsafe extern "system" fn(*mut std::ffi::c_void) -> usize,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
) -> Result<Option<String>, String> {
    if handle.is_null() {
        return Ok(None);
    }
    let size = global_size(handle);
    if size < std::mem::size_of::<u16>() {
        return Ok(None);
    }
    let source = global_lock(handle) as *const u16;
    if source.is_null() {
        return Ok(None);
    }
    let slice = std::slice::from_raw_parts(source, size / std::mem::size_of::<u16>());
    let len = slice.iter().position(|unit| *unit == 0).unwrap_or(slice.len());
    let text = String::from_utf16_lossy(&slice[..len]);
    global_unlock(handle);
    Ok(Some(text))
}

#[cfg(target_os = "windows")]
fn write_emf_preview(path: &Path, render_list_json: &str, bounds_json: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CloseEnhMetaFile, CreateEnhMetaFileW, DeleteEnhMetaFile, Ellipse, LineTo, MoveToEx,
        Polygon, Polyline, Rectangle, SetBkMode, SetTextColor, TextOutW, TRANSPARENT,
    };

    let primitives: Vec<serde_json::Value> =
        serde_json::from_str(render_list_json).map_err(|error| error.to_string())?;
    let bounds_value: serde_json::Value =
        serde_json::from_str(bounds_json).map_err(|error| error.to_string())?;
    let bounds = EmfBounds::from_json(&bounds_value).unwrap_or_else(|| {
        bounds_from_primitives(&primitives).unwrap_or(EmfBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        })
    });
    let layout = EmfLayout::new(bounds);
    let frame = RECT {
        left: 0,
        top: 0,
        right: layout.page_width,
        bottom: layout.page_height,
    };
    let path_wide = wide_null(&path.to_string_lossy());
    let desc = wide_null("Chemcore\0EMF Preview");
    let hdc = unsafe {
        CreateEnhMetaFileW(
            std::ptr::null_mut(),
            path_wide.as_ptr(),
            &frame,
            desc.as_ptr(),
        )
    };
    if hdc.is_null() {
        return Err("Failed to create EMF preview.".to_string());
    }

    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
    }

    for primitive in primitives.iter().filter(|primitive| is_document_primitive(primitive)) {
        let kind = primitive.get("kind").and_then(|value| value.as_str()).unwrap_or("");
        match kind {
            "line" => {
                let Some(from) = point_from_json(primitive.get("from")) else {
                    continue;
                };
                let Some(to) = point_from_json(primitive.get("to")) else {
                    continue;
                };
                with_pen(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    || unsafe {
                        let from = layout.point(from);
                        let to = layout.point(to);
                        MoveToEx(hdc, from.x, from.y, std::ptr::null_mut());
                        LineTo(hdc, to.x, to.y);
                    },
                );
            }
            "polyline" | "path" => {
                let points = points_from_json(primitive.get("points"), &layout);
                if points.len() < 2 {
                    continue;
                }
                with_pen(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    || unsafe {
                        Polyline(hdc, points.as_ptr(), points.len() as i32);
                    },
                );
            }
            "polygon" | "filled-path" => {
                let points = points_from_json(primitive.get("points"), &layout);
                if points.len() < 3 {
                    continue;
                }
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").is_some(),
                    || unsafe {
                        Polygon(hdc, points.as_ptr(), points.len() as i32);
                    },
                );
            }
            "rect" => {
                let Some((left, top, right, bottom)) = rect_from_json(primitive, &layout) else {
                    continue;
                };
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").and_then(|value| value.as_str()) != Some("none"),
                    || unsafe {
                        Rectangle(hdc, left, top, right, bottom);
                    },
                );
            }
            "circle" => {
                let Some(center) = point_from_json(primitive.get("center")) else {
                    continue;
                };
                let radius = primitive.get("radius").and_then(|value| value.as_f64()).unwrap_or(0.0);
                let center = layout.point(center);
                let radius = (radius * layout.scale).round().max(1.0) as i32;
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").is_some(),
                    || unsafe {
                        Ellipse(
                            hdc,
                            center.x - radius,
                            center.y - radius,
                            center.x + radius,
                            center.y + radius,
                        );
                    },
                );
            }
            "ellipse" => {
                let Some(center) = point_from_json(primitive.get("center")) else {
                    continue;
                };
                let rx = primitive.get("rx").and_then(|value| value.as_f64()).unwrap_or(0.0);
                let ry = primitive.get("ry").and_then(|value| value.as_f64()).unwrap_or(0.0);
                let center = layout.point(center);
                let rx = (rx * layout.scale).round().max(1.0) as i32;
                let ry = (ry * layout.scale).round().max(1.0) as i32;
                with_pen_and_brush(
                    hdc,
                    color_from_json(primitive.get("stroke"), 0x000000),
                    pen_width_from_json(primitive, layout.scale),
                    color_from_json(primitive.get("fill"), 0xffffff),
                    primitive.get("fill").is_some(),
                    || unsafe {
                        Ellipse(hdc, center.x - rx, center.y - ry, center.x + rx, center.y + ry);
                    },
                );
            }
            "text" => {
                let Some(text) = primitive.get("text").and_then(|value| value.as_str()) else {
                    continue;
                };
                let x = primitive.get("x").and_then(|value| value.as_f64()).unwrap_or(0.0);
                let y = primitive.get("y").and_then(|value| value.as_f64()).unwrap_or(0.0);
                let point = layout.point((x, y));
                let wide = wide_null(text);
                unsafe {
                    SetTextColor(hdc, color_from_json(primitive.get("fill"), 0x000000));
                    TextOutW(hdc, point.x, point.y, wide.as_ptr(), text.encode_utf16().count() as i32);
                }
            }
            _ => {}
        }
    }

    let metafile = unsafe { CloseEnhMetaFile(hdc) };
    if metafile.is_null() {
        return Err("Failed to finalize EMF preview.".to_string());
    }
    unsafe {
        DeleteEnhMetaFile(metafile);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn write_emf_preview(_path: &Path, _render_list_json: &str, _bounds_json: &str) -> Result<(), String> {
    Err("EMF export is only implemented on Windows.".to_string())
}

#[derive(Debug, Clone, Copy)]
struct EmfBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl EmfBounds {
    fn from_json(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            min_x: value.get("minX")?.as_f64()?,
            min_y: value.get("minY")?.as_f64()?,
            max_x: value.get("maxX")?.as_f64()?,
            max_y: value.get("maxY")?.as_f64()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct EmfLayout {
    bounds: EmfBounds,
    scale: f64,
    page_width: i32,
    page_height: i32,
    margin: i32,
}

impl EmfLayout {
    fn new(bounds: EmfBounds) -> Self {
        let width = (bounds.max_x - bounds.min_x).abs().max(1.0);
        let height = (bounds.max_y - bounds.min_y).abs().max(1.0);
        let margin = 300;
        let max_side = 9000.0;
        let scale = (max_side / width.max(height)).max(1.0);
        Self {
            bounds,
            scale,
            page_width: (width * scale).round() as i32 + margin * 2,
            page_height: (height * scale).round() as i32 + margin * 2,
            margin,
        }
    }

    fn point(&self, point: (f64, f64)) -> windows_sys::Win32::Foundation::POINT {
        windows_sys::Win32::Foundation::POINT {
            x: self.margin + ((point.0 - self.bounds.min_x) * self.scale).round() as i32,
            y: self.margin + ((point.1 - self.bounds.min_y) * self.scale).round() as i32,
        }
    }
}

fn is_document_primitive(primitive: &serde_json::Value) -> bool {
    primitive
        .get("role")
        .and_then(|value| value.as_str())
        .map(|role| role.starts_with("document-"))
        .unwrap_or(false)
}

fn point_from_json(value: Option<&serde_json::Value>) -> Option<(f64, f64)> {
    let value = value?;
    Some((value.get("x")?.as_f64()?, value.get("y")?.as_f64()?))
}

fn points_from_json(
    value: Option<&serde_json::Value>,
    layout: &EmfLayout,
) -> Vec<windows_sys::Win32::Foundation::POINT> {
    value
        .and_then(|value| value.as_array())
        .map(|points| {
            points
                .iter()
                .filter_map(|point| point_from_json(Some(point)))
                .map(|point| layout.point(point))
                .collect()
        })
        .unwrap_or_default()
}

fn rect_from_json(primitive: &serde_json::Value, layout: &EmfLayout) -> Option<(i32, i32, i32, i32)> {
    let x = primitive.get("x")?.as_f64()?;
    let y = primitive.get("y")?.as_f64()?;
    let width = primitive.get("width")?.as_f64()?;
    let height = primitive.get("height")?.as_f64()?;
    let top_left = layout.point((x, y));
    let bottom_right = layout.point((x + width, y + height));
    Some((top_left.x, top_left.y, bottom_right.x, bottom_right.y))
}

fn bounds_from_primitives(primitives: &[serde_json::Value]) -> Option<EmfBounds> {
    let mut bounds: Option<EmfBounds> = None;
    for point in primitives
        .iter()
        .flat_map(|primitive| primitive_points(primitive).into_iter())
    {
        bounds = Some(match bounds {
            Some(bounds) => EmfBounds {
                min_x: bounds.min_x.min(point.0),
                min_y: bounds.min_y.min(point.1),
                max_x: bounds.max_x.max(point.0),
                max_y: bounds.max_y.max(point.1),
            },
            None => EmfBounds {
                min_x: point.0,
                min_y: point.1,
                max_x: point.0,
                max_y: point.1,
            },
        });
    }
    bounds
}

fn primitive_points(primitive: &serde_json::Value) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for key in ["from", "to", "center"] {
        if let Some(point) = point_from_json(primitive.get(key)) {
            points.push(point);
        }
    }
    if let Some(array) = primitive.get("points").and_then(|value| value.as_array()) {
        points.extend(array.iter().filter_map(|point| point_from_json(Some(point))));
    }
    if let (Some(x), Some(y), Some(width), Some(height)) = (
        primitive.get("x").and_then(|value| value.as_f64()),
        primitive.get("y").and_then(|value| value.as_f64()),
        primitive.get("width").and_then(|value| value.as_f64()),
        primitive.get("height").and_then(|value| value.as_f64()),
    ) {
        points.push((x, y));
        points.push((x + width, y + height));
    }
    points
}

fn pen_width_from_json(primitive: &serde_json::Value, scale: f64) -> i32 {
    let width = primitive
        .get("strokeWidth")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.02);
    (width * scale).round().clamp(1.0, 80.0) as i32
}

fn color_from_json(value: Option<&serde_json::Value>, fallback_rgb: u32) -> u32 {
    let Some(raw) = value.and_then(|value| value.as_str()) else {
        return rgb_to_colorref(fallback_rgb);
    };
    let raw = raw.trim();
    if raw == "none" {
        return rgb_to_colorref(fallback_rgb);
    }
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    if hex.len() < 6 {
        return rgb_to_colorref(fallback_rgb);
    }
    u32::from_str_radix(&hex[..6], 16)
        .map(rgb_to_colorref)
        .unwrap_or_else(|_| rgb_to_colorref(fallback_rgb))
}

fn rgb_to_colorref(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xff;
    let g = (rgb >> 8) & 0xff;
    let b = rgb & 0xff;
    r | (g << 8) | (b << 16)
}

#[cfg(target_os = "windows")]
fn with_pen<F: FnOnce()>(hdc: windows_sys::Win32::Graphics::Gdi::HDC, color: u32, width: i32, draw: F) {
    use windows_sys::Win32::Graphics::Gdi::{CreatePen, DeleteObject, SelectObject, PS_SOLID};
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        let previous = SelectObject(hdc, pen);
        draw();
        SelectObject(hdc, previous);
        DeleteObject(pen);
    }
}

#[cfg(target_os = "windows")]
fn with_pen_and_brush<F: FnOnce()>(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    stroke: u32,
    width: i32,
    fill: u32,
    fill_enabled: bool,
    draw: F,
) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, GetStockObject, SelectObject, NULL_BRUSH,
        PS_SOLID,
    };
    unsafe {
        let pen = CreatePen(PS_SOLID, width, stroke);
        let brush = if fill_enabled {
            CreateSolidBrush(fill)
        } else {
            GetStockObject(NULL_BRUSH)
        };
        let previous_pen = SelectObject(hdc, pen);
        let previous_brush = SelectObject(hdc, brush);
        draw();
        SelectObject(hdc, previous_brush);
        SelectObject(hdc, previous_pen);
        if fill_enabled {
            DeleteObject(brush);
        }
        DeleteObject(pen);
    }
}

fn document_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter("Chemcore and CDXML", &["ccjz", "ccjs", "cdxml"])
        .add_filter("Chemcore compressed", &["ccjz"])
        .add_filter("Chemcore JSON", &["ccjs"])
        .add_filter("ChemDraw CDXML", &["cdxml"])
        .add_filter("SVG", &["svg"])
}

fn normalize_output_path(path: String) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err("Path is empty.".to_string());
    }
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| error.to_string())
    }
}

fn build_native_menu(
    app: &tauri::AppHandle,
    recent_files: &[DesktopRecentFile],
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let new_item = MenuItemBuilder::with_id("desktop-file-new", "&New")
        .accelerator("Ctrl+N")
        .build(app)?;
    let open_item = MenuItemBuilder::with_id("desktop-file-open", "&Open...")
        .accelerator("Ctrl+O")
        .build(app)?;
    let save_item = MenuItemBuilder::with_id("desktop-file-save", "&Save")
        .accelerator("Ctrl+S")
        .build(app)?;
    let save_as_item = MenuItemBuilder::with_id("desktop-file-save-as", "Save &As...")
        .accelerator("Ctrl+Shift+S")
        .build(app)?;
    let export_cdxml_item =
        MenuItemBuilder::with_id("desktop-file-export-cdxml", "Export &CDXML...").build(app)?;
    let export_svg_item =
        MenuItemBuilder::with_id("desktop-file-export-svg", "Export &SVG...").build(app)?;
    let export_pdf_item =
        MenuItemBuilder::with_id("desktop-file-export-pdf", "Export &PDF Preview...").build(app)?;
    let export_emf_item =
        MenuItemBuilder::with_id("desktop-file-export-emf", "Export &EMF Preview...").build(app)?;
    let clear_recent_item =
        MenuItemBuilder::with_id("desktop-recent-clear", "&Clear Recent Files").build(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("E&xit"))?;

    let mut recent_builder = SubmenuBuilder::new(app, "&Recent Files");
    if recent_files.is_empty() {
        let empty_item = MenuItemBuilder::with_id("desktop-recent-empty", "No Recent Files")
            .enabled(false)
            .build(app)?;
        recent_builder = recent_builder.item(&empty_item);
    } else {
        for (index, file) in recent_files.iter().enumerate() {
            let label = format!("{} {}", index + 1, file.file_name);
            let item = MenuItemBuilder::with_id(format!("desktop-recent-open-{index}"), label)
                .build(app)?;
            recent_builder = recent_builder.item(&item);
        }
        recent_builder = recent_builder.separator().item(&clear_recent_item);
    }
    let recent_menu = recent_builder.build()?;

    let undo_item = MenuItemBuilder::with_id("desktop-edit-undo", "&Undo")
        .accelerator("Ctrl+Z")
        .build(app)?;
    let redo_item = MenuItemBuilder::with_id("desktop-edit-redo", "&Redo")
        .accelerator("Ctrl+Y")
        .build(app)?;
    let cut_item = MenuItemBuilder::with_id("desktop-edit-cut", "Cu&t")
        .accelerator("Ctrl+X")
        .build(app)?;
    let copy_item = MenuItemBuilder::with_id("desktop-edit-copy", "&Copy")
        .accelerator("Ctrl+C")
        .build(app)?;
    let paste_item = MenuItemBuilder::with_id("desktop-edit-paste", "&Paste")
        .accelerator("Ctrl+V")
        .build(app)?;
    let delete_item = MenuItemBuilder::with_id("desktop-edit-delete", "&Delete")
        .accelerator("Delete")
        .build(app)?;

    let zoom_in_item = MenuItemBuilder::with_id("desktop-view-zoom-in", "Zoom &In")
        .accelerator("Ctrl+=")
        .build(app)?;
    let zoom_out_item = MenuItemBuilder::with_id("desktop-view-zoom-out", "Zoom &Out")
        .accelerator("Ctrl+-")
        .build(app)?;
    let fit_item = MenuItemBuilder::with_id("desktop-view-fit", "&Fit All")
        .accelerator("Ctrl+0")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "&File")
        .item(&new_item)
        .item(&open_item)
        .separator()
        .item(&save_item)
        .item(&save_as_item)
        .separator()
        .item(&export_cdxml_item)
        .item(&export_svg_item)
        .item(&export_pdf_item)
        .item(&export_emf_item)
        .separator()
        .item(&recent_menu)
        .separator()
        .item(&quit_item)
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, "&Edit")
        .item(&undo_item)
        .item(&redo_item)
        .separator()
        .item(&cut_item)
        .item(&copy_item)
        .item(&paste_item)
        .item(&delete_item)
        .build()?;
    let view_menu = SubmenuBuilder::new(app, "&View")
        .item(&zoom_in_item)
        .item(&zoom_out_item)
        .item(&fit_item)
        .build()?;

    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()
}

fn refresh_native_menu(app: &tauri::AppHandle) {
    let recent_files = app
        .try_state::<DesktopState>()
        .and_then(|state| {
            state
                .service
                .lock()
                .ok()
                .map(|service| service.recent_files())
        })
        .unwrap_or_default();
    if let Ok(menu) = build_native_menu(app, &recent_files) {
        let _ = app.set_menu(menu);
    }
}

fn install_native_menu(app: &tauri::App) -> tauri::Result<()> {
    let recent_files = app
        .try_state::<DesktopState>()
        .and_then(|state| {
            state
                .service
                .lock()
                .ok()
                .map(|service| service.recent_files())
        })
        .unwrap_or_default();
    let menu = build_native_menu(app.handle(), &recent_files)?;
    app.set_menu(menu)?;
    Ok(())
}

fn handle_native_menu_event(app: &tauri::AppHandle, id: &str) {
    if let Some(index) = id.strip_prefix("desktop-recent-open-") {
        if let Ok(index) = index.parse::<usize>() {
            let path = app
                .try_state::<DesktopState>()
                .and_then(|state| {
                    state
                        .service
                        .lock()
                        .ok()
                        .and_then(|service| service.recent_files().get(index).cloned())
                })
                .map(|entry| entry.path);
            if let Some(path) = path {
                emit_open_paths(app, vec![path]);
            }
        }
        return;
    }

    if id == "desktop-recent-clear" {
        if let Some(state) = app.try_state::<DesktopState>() {
            if let Ok(mut service) = state.service.lock() {
                let _ = service.clear_recent_files();
            }
        }
        refresh_native_menu(app);
        return;
    }

    let command = match id {
        "desktop-file-new" => "new",
        "desktop-file-open" => "open",
        "desktop-file-save" => "save",
        "desktop-file-save-as" => "save-as",
        "desktop-file-export-cdxml" => "save-cdxml",
        "desktop-file-export-svg" => "save-svg",
        "desktop-file-export-pdf" => "save-pdf",
        "desktop-file-export-emf" => "save-emf",
        "desktop-edit-undo" => "undo",
        "desktop-edit-redo" => "redo",
        "desktop-edit-cut" => "cut",
        "desktop-edit-copy" => "copy",
        "desktop-edit-paste" => "paste",
        "desktop-edit-delete" => "delete",
        "desktop-view-zoom-in" => "zoom-in",
        "desktop-view-zoom-out" => "zoom-out",
        "desktop-view-fit" => "fit",
        _ => return,
    };
    let _ = app.emit(
        EVENT_DESKTOP_MENU,
        DesktopMenuPayload {
            command: command.to_string(),
        },
    );
}

fn emit_open_paths(app: &tauri::AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    if let Some(state) = app.try_state::<DesktopState>() {
        if let Ok(mut pending) = state.pending_open_paths.lock() {
            pending.extend(paths.iter().cloned());
        }
    }
    let _ = app.emit(EVENT_DESKTOP_OPEN_PATHS, DesktopOpenPathsPayload { paths });
}

fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn startup_file_args() -> Vec<String> {
    let cwd = std::env::current_dir().ok();
    openable_document_args(std::env::args().skip(1), cwd.as_deref())
}

fn openable_document_args<I, S>(args: I, cwd: Option<&Path>) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .filter_map(|arg| resolve_open_arg(arg.as_ref(), cwd))
        .collect()
}

fn resolve_open_arg(arg: &str, cwd: Option<&Path>) -> Option<String> {
    let trimmed = arg.trim_matches('"');
    if !is_openable_document_arg(trimmed) {
        return None;
    }
    let path = PathBuf::from(trimmed);
    let path = if path.is_absolute() {
        path
    } else {
        cwd.unwrap_or_else(|| Path::new(".")).join(path)
    };
    Some(path.to_string_lossy().to_string())
}

fn is_openable_document_arg(arg: &str) -> bool {
    let lower = arg.to_ascii_lowercase();
    lower.ends_with(".ccjz")
        || lower.ends_with(".ccjs")
        || lower.ends_with(".cdxml")
        || lower.ends_with(".svg")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup_paths = startup_file_args();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let paths = openable_document_args(args.into_iter().skip(1), Some(Path::new(&cwd)));
            if !paths.is_empty() {
                emit_open_paths(app, paths);
            }
            focus_main_window(app);
        }))
        .manage(DesktopState::new())
        .invoke_handler(tauri::generate_handler![
            desktop_engine_create,
            desktop_engine_free,
            desktop_engine_load_document_json,
            desktop_engine_load_document_cdxml,
            desktop_engine_document_json,
            desktop_engine_state_json,
            desktop_engine_render_list_json,
            desktop_engine_render_bounds_json,
            desktop_engine_document_cdxml,
            desktop_engine_document_svg,
            desktop_engine_document_colors_json,
            desktop_color_choose,
            desktop_file_choose_open,
            desktop_file_choose_save,
            desktop_file_choose_export_save,
            desktop_file_read_path,
            desktop_file_write_path,
            desktop_file_write_base64,
            desktop_file_export_emf,
            desktop_recent_files,
            desktop_clear_recent_files,
            desktop_take_startup_open_paths,
            desktop_clipboard_write,
            desktop_clipboard_read,
        ])
        .on_menu_event(|app, event| {
            handle_native_menu_event(app, event.id().as_ref());
        })
        .on_window_event(|window, event| {
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                let app = window.app_handle();
                emit_open_paths(
                    app,
                    paths
                        .iter()
                        .map(|path| path.to_string_lossy().to_string())
                        .collect(),
                );
            }
        })
        .setup(move |app| {
            install_native_menu(app)?;
            if let Some(state) = app.try_state::<DesktopState>() {
                if let Ok(mut pending) = state.pending_open_paths.lock() {
                    pending.extend(startup_paths.iter().cloned());
                }
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_, _| {});
}
