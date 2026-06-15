use crate::*;

#[tauri::command]
pub(crate) fn desktop_engine_create(
    state: tauri::State<'_, DesktopState>,
) -> Result<SessionId, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    Ok(service.create_session())
}

#[tauri::command]
pub(crate) fn desktop_engine_free(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    Ok(service.free_session(session_id))
}

#[tauri::command]
pub(crate) fn desktop_engine_load_document_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    json: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.load_document_json(session_id, &json)
}

#[tauri::command]
pub(crate) fn desktop_engine_load_document_cdxml(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    cdxml: String,
) -> Result<(), String> {
    trace_desktop_event(format!(
        "desktop_engine_load_document_cdxml session_id={session_id} cdxml_len={}",
        cdxml.len()
    ));
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    let result = service.load_document_cdxml(session_id, &cdxml);
    trace_desktop_event(format!(
        "desktop_engine_load_document_cdxml result={}",
        if result.is_ok() { "ok" } else { "err" }
    ));
    result
}

#[tauri::command]
pub(crate) fn desktop_engine_load_document_sdf(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    sdf: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.load_document_sdf(session_id, &sdf)
}

#[tauri::command]
pub(crate) fn desktop_engine_document_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_execute_command_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    command_json: String,
) -> Result<String, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.execute_command_json(session_id, &command_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_state_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.state_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_render_list_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.render_list_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_render_bounds_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    scope: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.render_bounds_json(session_id, &scope)
}

#[tauri::command]
pub(crate) fn desktop_engine_snapshot_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    mode: DesktopEngineSnapshotMode,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    let snapshot = service.snapshot_json(session_id, mode)?;
    trace_desktop_event(format!(
        "desktop_engine_snapshot_json session_id={session_id} mode={mode:?} len={}",
        snapshot.len()
    ));
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn desktop_engine_document_cdxml(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_cdxml(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_document_sdf(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_sdf(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_document_svg(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_svg(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_document_colors_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_colors_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_tool(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    active_tool: String,
    bond_variant: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_tool(session_id, &active_tool, &bond_variant)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_shape_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    kind: String,
    style: String,
    color: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_shape_options(session_id, &kind, &style, &color)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_orbital_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    template: String,
    style: String,
    phase: String,
    color: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_orbital_options(session_id, &template, &style, &phase, &color)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_template(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    template: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_template(session_id, &template)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_bracket_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    kind: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_bracket_options(session_id, &kind)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_symbol_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    kind: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_symbol_options(session_id, &kind)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_element_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    symbol: String,
    atomic_number: u8,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_element_options(session_id, &symbol, atomic_number)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_document_style_preset(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    preset: String,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_document_style_preset(session_id, &preset)
}

#[tauri::command]
pub(crate) fn desktop_engine_document_style_preset(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.document_style_preset(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_object_settings_dialog_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.object_settings_dialog_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_toolbar_color_palette_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    custom_colors_json: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.toolbar_color_palette_json(session_id, &custom_colors_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_color_dialog_palette_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    current_color: String,
    custom_colors_json: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.color_dialog_palette_json(session_id, &current_color, &custom_colors_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_text_symbol_palette_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.text_symbol_palette_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_element_palette_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.element_palette_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_element_palette_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    selection_json: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_element_palette_json(session_id, &selection_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_object_settings_dialog_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    settings_json: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_object_settings_dialog_json(session_id, &settings_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_arrow_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    variant: String,
    head_size: String,
    head: bool,
    tail: bool,
    bold: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_arrow_options(session_id, &variant, &head_size, head, tail, bold)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_arrow_endpoint_options(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    variant: String,
    head_size: String,
    curve: String,
    head_style: String,
    tail_style: String,
    no_go: String,
    bold: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_arrow_endpoint_options(
        session_id,
        &variant,
        &head_size,
        &curve,
        &head_style,
        &tail_style,
        &no_go,
        bold,
    )
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_arrow_options_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    variant: String,
    head_size: String,
    head: bool,
    tail: bool,
    bold: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_arrow_options_to_selection(session_id, &variant, &head_size, head, tail, bold)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_arrow_endpoint_options_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    variant: String,
    head_size: String,
    curve: String,
    head_style: String,
    tail_style: String,
    no_go: String,
    bold: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_arrow_endpoint_options_to_selection(
        session_id,
        &variant,
        &head_size,
        &curve,
        &head_style,
        &tail_style,
        &no_go,
        bold,
    )
}

#[tauri::command]
pub(crate) fn desktop_engine_pointer_move(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.pointer_move(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_pointer_down(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.pointer_down(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_pointer_up(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.pointer_up(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_select_at_point(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    additive: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.select_at_point(session_id, x, y, additive)
}

#[tauri::command]
pub(crate) fn desktop_engine_select_component_at_point(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    additive: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.select_component_at_point(session_id, x, y, additive)
}

#[tauri::command]
pub(crate) fn desktop_engine_select_in_rect(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    additive: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.select_in_rect(session_id, x1, y1, x2, y2, additive)
}

#[tauri::command]
pub(crate) fn desktop_engine_select_in_polygon(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    points_json: String,
    additive: bool,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.select_in_polygon_json(session_id, &points_json, additive)
}

#[tauri::command]
pub(crate) fn desktop_engine_select_all(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.select_all(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_clear_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.clear_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_context_hit_test_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.context_hit_test_json(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_context_menu_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    hit_json: String,
    has_paste: bool,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.context_menu_json(session_id, &hit_json, has_paste)
}

#[tauri::command]
pub(crate) fn desktop_engine_selection_contains_point(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<bool, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.selection_contains_point(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_hover_arrow_action(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.hover_arrow_action(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_begin_hover_arrow_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<String, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.begin_hover_arrow_edit(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_update_hover_arrow_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.update_hover_arrow_edit(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_finish_hover_arrow_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.finish_hover_arrow_edit(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_hover_shape_action(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.hover_shape_action(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_begin_hover_shape_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<String, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.begin_hover_shape_edit(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_update_hover_shape_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.update_hover_shape_edit(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_finish_hover_shape_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.finish_hover_shape_edit(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_active_arrow_edit_degrees(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<f64, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.active_arrow_edit_degrees(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_begin_selection_move(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    additive: bool,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.begin_selection_move(session_id, x, y, additive, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_update_selection_move(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.update_selection_move(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_finish_selection_move(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.finish_selection_move(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_begin_selection_rotate(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.begin_selection_rotate(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_update_selection_rotate(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.update_selection_rotate(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_finish_selection_rotate(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
    alt_key: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.finish_selection_rotate(session_id, x, y, alt_key)
}

#[tauri::command]
pub(crate) fn desktop_engine_begin_selection_resize(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    handle: String,
    x: f64,
    y: f64,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.begin_selection_resize(session_id, &handle, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_update_selection_resize(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.update_selection_resize(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_finish_selection_resize(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.finish_selection_resize(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_selection_arrange_command(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    command: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_selection_arrange_command(session_id, &command)
}

#[tauri::command]
pub(crate) fn desktop_engine_scale_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    percent: f64,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.scale_selection(session_id, percent)
}

#[tauri::command]
pub(crate) fn desktop_engine_rotate_selection_degrees(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    degrees: f64,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.rotate_selection_degrees(session_id, degrees)
}

#[tauri::command]
pub(crate) fn desktop_engine_selection_numeric_dialog_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    kind: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.selection_numeric_dialog_json(session_id, &kind)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_selection_numeric_dialog_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    payload_json: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_selection_numeric_dialog_json(session_id, &payload_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_selection_order_command(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    command: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_selection_order_command(session_id, &command)
}

#[tauri::command]
pub(crate) fn desktop_engine_group_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.group_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_ungroup_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.ungroup_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_join_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.join_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_color_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    color: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_color_to_selection(session_id, &color)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_shape_style_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    style: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_shape_style_to_selection(session_id, &style)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_orbital_template_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    template: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_orbital_template_to_selection(session_id, &template)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_orbital_style_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    style: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_orbital_style_to_selection(session_id, &style)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_orbital_phase_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    phase: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_orbital_phase_to_selection(session_id, &phase)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_bracket_kind_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    kind: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_bracket_kind_to_selection(session_id, &kind)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_line_style_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    style: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_line_style_to_selection(session_id, &style)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_bond_style_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    style: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_bond_style_to_selection(session_id, &style)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_hovered_bond_style(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    style: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_hovered_bond_style(session_id, &style)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_text_style_to_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    command: String,
    value: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_text_style_to_selection(session_id, &command, &value)
}

#[tauri::command]
pub(crate) fn desktop_engine_set_chemical_check_for_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    enabled: bool,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.set_chemical_check_for_selection(session_id, enabled)
}

#[tauri::command]
pub(crate) fn desktop_engine_expand_labels_in_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.expand_labels_in_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_center_selection_on_page(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.center_selection_on_page(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_clear_interaction(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<(), String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.clear_interaction(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_undo(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.undo(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_redo(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.redo(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_can_undo(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.can_undo(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_can_redo(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.can_redo(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_delete_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.delete_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_copy_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.copy_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_has_clipboard(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.has_clipboard(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_clipboard_selection_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<Option<String>, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.clipboard_selection_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_clipboard_document_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<Option<String>, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.clipboard_document_json(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_cut_selection(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.cut_selection(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_paste_clipboard(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.paste_clipboard(session_id)
}

#[tauri::command]
pub(crate) fn desktop_engine_paste_clipboard_json(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    json: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.paste_clipboard_json(session_id, &json)
}

#[tauri::command]
pub(crate) fn desktop_engine_replace_hovered_endpoint_label(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    label: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.replace_hovered_endpoint_label(session_id, &label)
}

#[tauri::command]
pub(crate) fn desktop_engine_begin_text_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    x: f64,
    y: f64,
) -> Result<Option<String>, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.begin_text_edit(session_id, x, y)
}

#[tauri::command]
pub(crate) fn desktop_engine_apply_text_edit(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    session_json: String,
) -> Result<bool, String> {
    let mut service = state.service.lock().map_err(|error| error.to_string())?;
    service.apply_text_edit(session_id, &session_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_preview_text_runs(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    session_json: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.preview_text_runs(session_id, &session_json)
}

#[tauri::command]
pub(crate) fn desktop_engine_preview_text_edit_layout(
    state: tauri::State<'_, DesktopState>,
    session_id: SessionId,
    request_json: String,
) -> Result<String, String> {
    let service = state.service.lock().map_err(|error| error.to_string())?;
    service.preview_text_edit_layout(session_id, &request_json)
}
