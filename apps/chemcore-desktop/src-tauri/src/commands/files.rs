use crate::*;

#[tauri::command]
pub(crate) fn desktop_file_choose_open() -> Result<Option<String>, String> {
    Ok(document_file_dialog()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
pub(crate) fn desktop_dialog_confirm_style_preset(
    preset_name: String,
    message: String,
) -> Result<bool, String> {
    let description = if message.trim().is_empty() {
        format!("Apply {preset_name} to this document?")
    } else {
        message
    };
    let result = rfd::MessageDialog::new()
        .set_title("Apply Style Preset")
        .set_description(&description)
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::OkCancel)
        .show();
    Ok(result == rfd::MessageDialogResult::Ok)
}

#[tauri::command]
pub(crate) fn desktop_file_choose_save(suggested_name: String) -> Result<Option<String>, String> {
    Ok(document_file_dialog()
        .set_file_name(suggested_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
pub(crate) fn desktop_file_choose_export_save(
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
pub(crate) fn desktop_file_read_path(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    path: String,
) -> Result<DesktopOpenedDocument, String> {
    trace_desktop_event(format!("desktop_file_read_path path={path:?}"));
    let opened = {
        let mut service = state.service.lock().map_err(|error| error.to_string())?;
        service.read_document_file(path)?
    };
    trace_desktop_event(format!(
        "desktop_file_read_path result path={:?} format={} text_len={}",
        opened.path,
        opened.format,
        opened.text.len()
    ));
    refresh_native_menu(&app);
    Ok(opened)
}

#[tauri::command]
pub(crate) fn desktop_file_write_path(
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
pub(crate) fn desktop_file_write_transient_path(
    path: String,
    content: String,
) -> Result<DesktopSavedDocument, String> {
    Ok(write_transient_content(path, content)?.0)
}

fn write_transient_content(
    path: String,
    content: String,
) -> Result<(DesktopSavedDocument, PathBuf), String> {
    let path = normalize_output_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create directory {}: {error}", parent.display()))?;
    }
    fs::write(&path, content)
        .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
    Ok((
        DesktopSavedDocument {
            file_name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("transient.ccjs")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            format: "ccjs".to_string(),
        },
        path,
    ))
}

#[tauri::command]
pub(crate) fn desktop_file_write_ole_edit_payload(
    path: String,
    payload: NativeClipboardWritePayload,
) -> Result<DesktopSavedDocument, String> {
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Failed to serialize OLE edit payload: {error}"))?;
    let (saved, normalized_path) = write_transient_content(path, format!("{content}\n"))?;
    notify_ole_edit_session_payload_changed(&normalized_path);
    Ok(saved)
}

fn ole_edit_notify_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("chemcore-ole-edit.ccjs");
    path.with_file_name(format!("{file_name}.notify.json"))
}

#[cfg(target_os = "windows")]
fn notify_ole_edit_session_payload_changed(path: &Path) {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW;

    let notify_path = ole_edit_notify_path(path);
    let Ok(text) = fs::read_to_string(&notify_path) else {
        return;
    };
    let Ok(payload) = serde_json::from_str::<OleEditNotifyPayload>(&text) else {
        return;
    };
    if payload.thread_id == 0 {
        return;
    }
    unsafe {
        let posted = PostThreadMessageW(payload.thread_id, WM_OLE_EDIT_SESSION_CHANGED, 0, 0);
        if posted == 0 {
            eprintln!(
                "Failed to notify OLE edit session thread {} after writing {}: {}",
                payload.thread_id,
                path.display(),
                GetLastError()
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn notify_ole_edit_session_payload_changed(_path: &Path) {}

#[tauri::command]
pub(crate) fn desktop_file_write_base64(
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
    fs::write(&path, bytes)
        .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
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
pub(crate) fn desktop_file_export_emf(
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
pub(crate) fn desktop_recent_files(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<DesktopRecentFile>, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    Ok(service.recent_files())
}

#[tauri::command]
pub(crate) fn desktop_clear_recent_files(
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
pub(crate) fn desktop_take_startup_open_paths(
    state: tauri::State<'_, DesktopState>,
) -> Result<Vec<String>, String> {
    let mut paths = state
        .pending_open_paths
        .lock()
        .map_err(|error| error.to_string())?;
    Ok(std::mem::take(&mut *paths))
}
