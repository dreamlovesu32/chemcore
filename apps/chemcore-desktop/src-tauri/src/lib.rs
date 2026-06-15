use base64::Engine as _;
use chemcore_desktop_service::{
    DesktopDocumentService, DesktopEngineSnapshotMode, DesktopOpenedDocument, DesktopRecentFile,
    DesktopSavedDocument, SessionId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{
    DragDropEvent, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

const USE_NATIVE_MENU: bool = false;
const EVENT_DESKTOP_MENU: &str = "chemcore-desktop-menu";
const EVENT_DESKTOP_OPEN_PATHS: &str = "chemcore-desktop-open-paths";
const FORMAT_CHEMCORE_FRAGMENT: &str = "Chemcore Clipboard Fragment";
const FORMAT_CHEMCORE_DOCUMENT_JSON: &str = "Chemcore Document JSON";
const FORMAT_CHEMDRAW_INTERCHANGE: &str = "ChemDraw Interchange Format";
const FORMAT_CDXML_MIME: &str = "chemical/x-cdxml";
const FORMAT_SVG_MIME: &str = "image/svg+xml";
const FORMAT_SVG: &str = "SVG";
const GMEM_MOVEABLE_FLAG: u32 = 0x0002;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;
#[cfg(target_os = "windows")]
const WM_OLE_EDIT_SESSION_CHANGED: u32 = 0x80CC;

struct DesktopState {
    service: Mutex<DesktopDocumentService>,
    pending_open_paths: Mutex<Vec<String>>,
    pending_detached_documents: Mutex<BTreeMap<String, DesktopDetachedDocumentPayload>>,
}

fn current_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn desktop_trace_path() -> PathBuf {
    if let Ok(path) = std::env::var("CHEMCORE_DRAG_TRACE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("Chemcore").join("desktop-events.log")
}

fn trace_desktop_event(message: impl AsRef<str>) {
    let path = desktop_trace_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let timestamp = current_timestamp_ms();
    let line = format!("{timestamp} {}\n", message.as_ref());
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

#[tauri::command]
fn desktop_trace_event(message: String) {
    trace_desktop_event(format!("frontend {message}"));
}

impl DesktopState {
    fn new() -> Self {
        Self {
            service: Mutex::new(DesktopDocumentService::new()),
            pending_open_paths: Mutex::new(Vec::new()),
            pending_detached_documents: Mutex::new(BTreeMap::new()),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopDetachedDocumentPayload {
    title: String,
    file_name: Option<String>,
    file_path: Option<String>,
    document_json: String,
    saved_document_json: Option<String>,
    zoom_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeClipboardWritePayload {
    chemcore_fragment_json: Option<String>,
    chemcore_document_json: Option<String>,
    render_list_json: Option<String>,
    cdxml: Option<String>,
    svg: Option<String>,
    text: Option<String>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OleEditNotifyPayload {
    thread_id: u32,
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

mod commands;

use commands::clipboard::*;
use commands::engine::*;
use commands::files::*;
use commands::window::*;

#[cfg(target_os = "windows")]
mod desktop_emf;
mod menus;
mod paths;
mod window_helpers;

use desktop_emf::*;
use menus::*;
use paths::*;
use window_helpers::*;

pub fn run() {
    let startup_paths = startup_file_args();
    trace_desktop_event(format!(
        "app_start pid={} startup_paths={startup_paths:?}",
        std::process::id()
    ));
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let paths = openable_document_args(args.into_iter().skip(1), Some(Path::new(&cwd)));
            if !paths.is_empty() {
                emit_open_paths(app, paths);
            } else {
                focus_main_window(app);
            }
        }))
        .manage(DesktopState::new())
        .invoke_handler(tauri::generate_handler![
            desktop_engine_create,
            desktop_engine_free,
            desktop_engine_load_document_json,
            desktop_engine_load_document_cdxml,
            desktop_engine_load_document_sdf,
            desktop_engine_document_json,
            desktop_engine_execute_command_json,
            desktop_engine_state_json,
            desktop_engine_render_list_json,
            desktop_engine_render_bounds_json,
            desktop_engine_snapshot_json,
            desktop_engine_document_cdxml,
            desktop_engine_document_sdf,
            desktop_engine_document_svg,
            desktop_engine_document_colors_json,
            desktop_engine_set_tool,
            desktop_engine_set_shape_options,
            desktop_engine_set_orbital_options,
            desktop_engine_set_template,
            desktop_engine_set_bracket_options,
            desktop_engine_set_symbol_options,
            desktop_engine_set_element_options,
            desktop_engine_set_document_style_preset,
            desktop_engine_document_style_preset,
            desktop_engine_object_settings_dialog_json,
            desktop_engine_toolbar_color_palette_json,
            desktop_engine_color_dialog_palette_json,
            desktop_engine_text_symbol_palette_json,
            desktop_engine_element_palette_json,
            desktop_engine_apply_element_palette_json,
            desktop_engine_apply_object_settings_dialog_json,
            desktop_engine_set_arrow_options,
            desktop_engine_set_arrow_endpoint_options,
            desktop_engine_apply_arrow_options_to_selection,
            desktop_engine_apply_arrow_endpoint_options_to_selection,
            desktop_engine_pointer_move,
            desktop_engine_pointer_down,
            desktop_engine_pointer_up,
            desktop_engine_select_at_point,
            desktop_engine_select_component_at_point,
            desktop_engine_select_in_rect,
            desktop_engine_select_in_polygon,
            desktop_engine_select_all,
            desktop_engine_clear_selection,
            desktop_engine_context_hit_test_json,
            desktop_engine_context_menu_json,
            desktop_engine_selection_contains_point,
            desktop_engine_hover_arrow_action,
            desktop_engine_begin_hover_arrow_edit,
            desktop_engine_update_hover_arrow_edit,
            desktop_engine_finish_hover_arrow_edit,
            desktop_engine_hover_shape_action,
            desktop_engine_begin_hover_shape_edit,
            desktop_engine_update_hover_shape_edit,
            desktop_engine_finish_hover_shape_edit,
            desktop_engine_active_arrow_edit_degrees,
            desktop_engine_begin_selection_move,
            desktop_engine_update_selection_move,
            desktop_engine_finish_selection_move,
            desktop_engine_begin_selection_rotate,
            desktop_engine_update_selection_rotate,
            desktop_engine_finish_selection_rotate,
            desktop_engine_begin_selection_resize,
            desktop_engine_update_selection_resize,
            desktop_engine_finish_selection_resize,
            desktop_engine_apply_selection_arrange_command,
            desktop_engine_scale_selection,
            desktop_engine_rotate_selection_degrees,
            desktop_engine_selection_numeric_dialog_json,
            desktop_engine_apply_selection_numeric_dialog_json,
            desktop_engine_apply_selection_order_command,
            desktop_engine_group_selection,
            desktop_engine_ungroup_selection,
            desktop_engine_join_selection,
            desktop_engine_apply_color_to_selection,
            desktop_engine_apply_shape_style_to_selection,
            desktop_engine_apply_orbital_template_to_selection,
            desktop_engine_apply_orbital_style_to_selection,
            desktop_engine_apply_orbital_phase_to_selection,
            desktop_engine_apply_bracket_kind_to_selection,
            desktop_engine_apply_line_style_to_selection,
            desktop_engine_apply_bond_style_to_selection,
            desktop_engine_apply_hovered_bond_style,
            desktop_engine_apply_text_style_to_selection,
            desktop_engine_set_chemical_check_for_selection,
            desktop_engine_expand_labels_in_selection,
            desktop_engine_center_selection_on_page,
            desktop_engine_clear_interaction,
            desktop_engine_undo,
            desktop_engine_redo,
            desktop_engine_can_undo,
            desktop_engine_can_redo,
            desktop_engine_delete_selection,
            desktop_engine_copy_selection,
            desktop_engine_has_clipboard,
            desktop_engine_clipboard_selection_json,
            desktop_engine_clipboard_document_json,
            desktop_engine_cut_selection,
            desktop_engine_paste_clipboard,
            desktop_engine_paste_clipboard_json,
            desktop_engine_replace_hovered_endpoint_label,
            desktop_engine_begin_text_edit,
            desktop_engine_apply_text_edit,
            desktop_engine_preview_text_runs,
            desktop_engine_preview_text_edit_layout,
            desktop_file_choose_open,
            desktop_dialog_confirm_style_preset,
            desktop_file_choose_save,
            desktop_file_choose_export_save,
            desktop_file_read_path,
            desktop_file_write_path,
            desktop_file_write_transient_path,
            desktop_file_write_ole_edit_payload,
            desktop_file_write_base64,
            desktop_file_export_emf,
            desktop_recent_files,
            desktop_clear_recent_files,
            desktop_take_startup_open_paths,
            desktop_window_set_title,
            desktop_window_minimize,
            desktop_window_toggle_maximize,
            desktop_window_close,
            desktop_window_destroy,
            desktop_window_start_dragging,
            desktop_window_is_maximized,
            desktop_window_detach_document,
            desktop_window_take_detached_document,
            desktop_clipboard_write,
            desktop_clipboard_read,
            desktop_trace_event,
        ])
        .on_menu_event(|app, event| {
            handle_native_menu_event(app, event.id().as_ref());
        })
        .on_window_event(|window, event| {
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                trace_desktop_event(format!("window.drag_drop.drop paths={paths:?}"));
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
