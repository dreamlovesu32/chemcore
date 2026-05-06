use chemcore_desktop_service::{DesktopDocumentService, SessionId};
use std::sync::Mutex;

struct DesktopServiceState(Mutex<DesktopDocumentService>);

#[tauri::command]
fn desktop_engine_create(
    state: tauri::State<'_, DesktopServiceState>,
) -> Result<SessionId, String> {
    let mut service = state.0.lock().map_err(|error| error.to_string())?;
    Ok(service.create_session())
}

#[tauri::command]
fn desktop_engine_free(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.0.lock().map_err(|error| error.to_string())?;
    Ok(service.free_session(session_id))
}

#[tauri::command]
fn desktop_engine_load_document_json(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
    json: String,
) -> Result<(), String> {
    let mut service = state.0.lock().map_err(|error| error.to_string())?;
    service.load_document_json(session_id, &json)
}

#[tauri::command]
fn desktop_engine_load_document_cdxml(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
    cdxml: String,
) -> Result<(), String> {
    let mut service = state.0.lock().map_err(|error| error.to_string())?;
    service.load_document_cdxml(session_id, &cdxml)
}

#[tauri::command]
fn desktop_engine_document_json(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.0.lock().map_err(|error| error.to_string())?;
    service.document_json(session_id)
}

#[tauri::command]
fn desktop_engine_state_json(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.0.lock().map_err(|error| error.to_string())?;
    service.state_json(session_id)
}

#[tauri::command]
fn desktop_engine_render_list_json(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.0.lock().map_err(|error| error.to_string())?;
    service.render_list_json(session_id)
}

#[tauri::command]
fn desktop_engine_render_bounds_json(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
    scope: String,
) -> Result<String, String> {
    let service = state.0.lock().map_err(|error| error.to_string())?;
    service.render_bounds_json(session_id, &scope)
}

#[tauri::command]
fn desktop_engine_document_cdxml(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.0.lock().map_err(|error| error.to_string())?;
    service.document_cdxml(session_id)
}

#[tauri::command]
fn desktop_engine_document_svg(
    state: tauri::State<'_, DesktopServiceState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.0.lock().map_err(|error| error.to_string())?;
    service.document_svg(session_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DesktopServiceState(Mutex::new(
            DesktopDocumentService::new(),
        )))
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
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
