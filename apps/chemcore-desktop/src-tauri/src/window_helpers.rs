use crate::*;

pub(crate) fn emit_open_paths(app: &tauri::AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    trace_desktop_event(format!("emit_open_paths paths={paths:?}"));
    let target = app
        .webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
        .or_else(|| app.get_webview_window("main"))
        .or_else(|| app.webview_windows().into_values().next());
    if let Some(window) = target {
        trace_desktop_event(format!("emit_open_paths target={}", window.label()));
        focus_webview_window(&window);
        let _ = window.emit(EVENT_DESKTOP_OPEN_PATHS, DesktopOpenPathsPayload { paths });
        return;
    }
    trace_desktop_event("emit_open_paths queued=no_window");
    if let Some(state) = app.try_state::<DesktopState>() {
        if let Ok(mut pending) = state.pending_open_paths.lock() {
            pending.extend(paths);
        }
    }
}

pub(crate) fn editor_window_url(app: &tauri::AppHandle) -> Result<WebviewUrl, String> {
    let query = "chemcoreWindow=1";
    if tauri::is_dev() {
        let mut url = app
            .config()
            .build
            .dev_url
            .clone()
            .ok_or_else(|| "Desktop dev URL is unavailable.".to_string())?;
        url.set_query(Some(query));
        return Ok(WebviewUrl::External(url));
    }
    Ok(WebviewUrl::App(PathBuf::from(format!(
        "index.html?{query}"
    ))))
}

pub(crate) fn next_document_window_label(app: &tauri::AppHandle) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    for index in 0..1000 {
        let label = format!("document-{stamp}-{index}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
    }
    format!("document-{stamp}")
}

pub(crate) fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        focus_webview_window(&window);
    }
}

pub(crate) fn focus_webview_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}
