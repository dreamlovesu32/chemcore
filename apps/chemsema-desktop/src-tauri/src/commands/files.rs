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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopBinaryFile {
    file_name: String,
    mime_type: String,
    data_base64: String,
}

#[tauri::command]
pub(crate) fn desktop_file_read_binary_path(path: String) -> Result<DesktopBinaryFile, String> {
    const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
    let path = PathBuf::from(path);
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_IMAGE_BYTES {
        return Err("Image must be a non-empty file no larger than 64 MiB.".to_string());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime_type = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => return Err(format!("Unsupported image extension: .{extension}")),
    };
    let bytes =
        fs::read(&path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    Ok(DesktopBinaryFile {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("image")
            .to_string(),
        mime_type: mime_type.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
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
pub(crate) fn desktop_file_read_recovery_journal(
    state: tauri::State<'_, DesktopState>,
    path: String,
) -> Result<Option<String>, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.read_recovery_journal(path)
}

#[tauri::command]
pub(crate) fn desktop_file_write_recovery_journal(
    state: tauri::State<'_, DesktopState>,
    path: String,
    content: String,
) -> Result<(), String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.write_recovery_journal(path, &content)
}

#[tauri::command]
pub(crate) fn desktop_file_delete_recovery_journal(
    state: tauri::State<'_, DesktopState>,
    path: String,
) -> Result<(), String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.delete_recovery_journal(path)
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
        .unwrap_or("chemsema-ole-edit.ccjs");
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
    payload: NativeClipboardWritePayload,
) -> Result<DesktopSavedDocument, String> {
    let path = normalize_output_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create directory {}: {error}", parent.display()))?;
    }
    let payload_json = serde_json::to_string(&payload)
        .map_err(|error| format!("Failed to serialize EMF payload: {error}"))?;
    chemsema_office::write_emf_payload_json(&path, &payload_json)?;
    Ok(DesktopSavedDocument {
        file_name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("document.emf")
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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn desktop_emf_export_writes_an_enhanced_metafile() {
        let document_json = serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank())
            .expect("blank document should serialize");
        let path =
            std::env::temp_dir().join(format!("chemsema-desktop-emf-{}.emf", std::process::id()));
        let payload = NativeClipboardWritePayload {
            chemsema_fragment_json: None,
            chemsema_document_json: Some(document_json),
            render_list_json: None,
            cdxml: None,
            svg: None,
            text: None,
        };
        let saved = desktop_file_export_emf(path.to_string_lossy().to_string(), payload)
            .expect("desktop EMF export should succeed");
        let bytes = std::fs::read(&path).expect("desktop EMF should be readable");
        let _ = std::fs::remove_file(path);
        assert_eq!(saved.format, "emf");
        assert!(bytes.len() >= 88, "EMF header should be present");
        assert_eq!(&bytes[40..44], b" EMF", "EMF signature should match");
    }
}
