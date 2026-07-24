use super::*;

pub(super) fn run_com_server() -> Result<(), String> {
    let main_thread_id = unsafe { GetCurrentThreadId() };
    let _ = OLE_EDIT_MAIN_THREAD_ID.set(main_thread_id);
    log_ole_event(&format!("COM server main thread id {main_thread_id}"));
    log_ole_event("COM server initializing OLE");
    let hr = unsafe { OleInitialize(null()) };
    if !hresult_succeeded(hr) {
        log_ole_event(&format!("OleInitialize failed: 0x{:08X}", hr as u32));
        return Err(format!("OleInitialize failed: 0x{:08X}", hr as u32));
    }

    let mut registration_cookie = 0;
    log_ole_event("COM server registering class object");
    let hr = unsafe {
        CoRegisterClassObject(
            &CLSID_CHEMSEMA_DOCUMENT,
            (&CLASS_FACTORY as *const ClassFactory)
                .cast_mut()
                .cast::<c_void>(),
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE as u32,
            &mut registration_cookie,
        )
    };
    if !hresult_succeeded(hr) {
        unsafe {
            OleUninitialize();
        }
        log_ole_event(&format!(
            "CoRegisterClassObject failed for {CLSID_STRING}: 0x{:08X}",
            hr as u32
        ));
        return Err(format!(
            "CoRegisterClassObject failed for {CLSID_STRING}: 0x{:08X}",
            hr as u32
        ));
    }

    log_ole_event("COM server class object registered");
    unsafe {
        SetTimer(null_mut(), OLE_EDIT_TIMER_ID, OLE_EDIT_POLL_MS, None);
    }
    run_message_loop();

    log_ole_event("COM server message loop exited");
    unsafe {
        KillTimer(null_mut(), OLE_EDIT_TIMER_ID);
        CoRevokeClassObject(registration_cookie);
        OleUninitialize();
    }
    Ok(())
}

pub(super) fn run_message_loop() {
    let mut message: MSG = unsafe { zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result <= 0 {
            break;
        }
        if message.message == WM_TIMER && message.wParam == OLE_EDIT_TIMER_ID {
            poll_ole_edit_sessions();
            continue;
        }
        if message.message == WM_OLE_EDIT_SESSION_CHANGED {
            log_ole_event("COM server received OLE edit session change message");
            apply_pending_ole_edit_session_updates();
            poll_ole_edit_sessions();
            continue;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

pub(super) fn register_ole_edit_session(
    session_id: String,
    path: PathBuf,
    object: *mut ChemSemaOleObject,
) -> Result<(), String> {
    if object.is_null() {
        return Err("OLE edit session cannot register a null object.".to_string());
    }
    let last_modified = file_modified_time(&path);
    chemsema_object_add_ref(object);
    let mut sessions = ole_edit_sessions()
        .lock()
        .map_err(|error| error.to_string())?;
    sessions.insert(
        session_id.clone(),
        OleEditSession {
            path: path.clone(),
            object: object as usize,
            last_modified,
        },
    );
    unsafe {
        SetTimer(null_mut(), OLE_EDIT_TIMER_ID, OLE_EDIT_POLL_MS, None);
    }
    start_ole_edit_file_watcher(session_id, path, last_modified);
    Ok(())
}

pub(super) fn ole_edit_session_path_for_object(object: *mut ChemSemaOleObject) -> Option<PathBuf> {
    if object.is_null() {
        return None;
    }
    let object = object as usize;
    let sessions = ole_edit_sessions().lock().ok()?;
    sessions
        .values()
        .find(|session| session.object == object && session.path.exists())
        .map(|session| session.path.clone())
}

pub(super) fn file_modified_time(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

pub(super) fn ole_edit_session_notify_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("chemsema-ole-edit.ccjs");
    path.with_file_name(format!("{file_name}.notify.json"))
}

pub(super) fn write_ole_edit_session_notify_file(path: &Path) -> Result<(), String> {
    let Some(thread_id) = OLE_EDIT_MAIN_THREAD_ID.get().copied() else {
        return Ok(());
    };
    let notify_path = ole_edit_session_notify_path(path);
    let payload = serde_json::to_string_pretty(&serde_json::json!({
        "threadId": thread_id,
    }))
    .map_err(|error| format!("Failed to serialize OLE edit notification payload: {error}"))?;
    std::fs::write(&notify_path, format!("{payload}\n")).map_err(|error| {
        format!(
            "Failed to write OLE edit notification file {}: {error}",
            notify_path.display()
        )
    })
}

pub(super) fn start_ole_edit_file_watcher(
    session_id: String,
    path: PathBuf,
    initial_modified: Option<SystemTime>,
) {
    let Some(thread_id) = OLE_EDIT_MAIN_THREAD_ID.get().copied() else {
        return;
    };
    thread::spawn(move || {
        let mut last_modified = initial_modified;
        loop {
            thread::sleep(Duration::from_millis(u64::from(OLE_EDIT_POLL_MS)));
            let Some(modified) = file_modified_time(&path) else {
                break;
            };
            if last_modified.is_some_and(|last| modified <= last) {
                continue;
            }
            last_modified = Some(modified);
            if let Ok(mut pending) = ole_edit_pending_updates().lock() {
                if !pending.iter().any(|value| value == &session_id) {
                    pending.push(session_id.clone());
                }
            }
            log_ole_event(&format!(
                "OLE edit watcher noticed change for {session_id} at {}",
                path.display()
            ));
            unsafe {
                let posted = PostThreadMessageW(thread_id, WM_OLE_EDIT_SESSION_CHANGED, 0, 0);
                if posted == 0 {
                    log_ole_event(&format!(
                        "PostThreadMessageW({thread_id}, WM_OLE_EDIT_SESSION_CHANGED) failed: {}",
                        GetLastError()
                    ));
                }
            }
        }
    });
}

pub(super) fn poll_ole_edit_sessions() {
    let candidates = {
        let Ok(sessions) = ole_edit_sessions().lock() else {
            return;
        };
        sessions
            .iter()
            .filter_map(|(session_id, session)| {
                let modified = file_modified_time(&session.path)?;
                if session.last_modified.is_some_and(|last| modified <= last) {
                    return None;
                }
                Some((
                    session_id.clone(),
                    session.path.clone(),
                    session.object,
                    modified,
                ))
            })
            .collect::<Vec<_>>()
    };

    for (session_id, path, object, modified) in candidates {
        let document_json = match std::fs::read_to_string(&path) {
            Ok(value) => value,
            Err(error) => {
                log_ole_event(&format!(
                    "Failed to read OLE edit session {} from {}: {error}",
                    session_id,
                    path.display()
                ));
                continue;
            }
        };
        let hr = unsafe {
            apply_ole_edit_session_update(object as *mut ChemSemaOleObject, &document_json)
        };
        if hresult_succeeded(hr) {
            if let Ok(mut sessions) = ole_edit_sessions().lock() {
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.last_modified = Some(modified);
                }
            }
        }
        log_ole_event(&format!(
            "OLE edit session {session_id} update -> 0x{:08X}",
            hr as u32
        ));
    }
}

pub(super) fn apply_pending_ole_edit_session_updates() {
    let session_ids = {
        let Ok(mut pending) = ole_edit_pending_updates().lock() else {
            return;
        };
        std::mem::take(&mut *pending)
    };
    if session_ids.is_empty() {
        log_ole_event("OLE edit pending update queue was empty");
    } else {
        log_ole_event(&format!(
            "Applying {} pending OLE edit session update(s)",
            session_ids.len()
        ));
    }
    for session_id in session_ids {
        apply_ole_edit_session_update_by_id(&session_id);
    }
}

pub(super) fn apply_ole_edit_session_update_by_id(session_id: &str) {
    let Some((path, object, modified)) = ({
        let Ok(sessions) = ole_edit_sessions().lock() else {
            return;
        };
        sessions.get(session_id).and_then(|session| {
            let modified = file_modified_time(&session.path)?;
            Some((session.path.clone(), session.object, modified))
        })
    }) else {
        log_ole_event(&format!(
            "OLE edit session {session_id} was not found for pending update"
        ));
        return;
    };
    let document_json = match std::fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            log_ole_event(&format!(
                "Failed to read OLE edit session {} from {}: {error}",
                session_id,
                path.display()
            ));
            return;
        }
    };
    log_ole_event(&format!(
        "Read OLE edit session {session_id} from {} ({} bytes)",
        path.display(),
        document_json.len()
    ));
    let hr =
        unsafe { apply_ole_edit_session_update(object as *mut ChemSemaOleObject, &document_json) };
    if hresult_succeeded(hr) {
        if let Ok(mut sessions) = ole_edit_sessions().lock() {
            if let Some(session) = sessions.get_mut(session_id) {
                session.last_modified = Some(modified);
            }
        }
    }
    log_ole_event(&format!(
        "OLE edit session {session_id} update -> 0x{:08X}",
        hr as u32
    ));
}

pub(super) unsafe fn apply_ole_edit_session_update(
    object: *mut ChemSemaOleObject,
    document_json: &str,
) -> i32 {
    log_ole_event("Applying OLE edit session payload to object");
    if object.is_null() {
        return E_POINTER;
    }
    if document_json.trim().is_empty() {
        return E_FAIL;
    }
    let payload = match ole_object_payload_from_edit_session_text(document_json) {
        Ok(payload) => payload,
        Err(error) => {
            log_ole_event(&format!("Invalid OLE edit session payload: {error}"));
            return E_FAIL;
        }
    };
    log_ole_event("Built OLE edit session payload");
    (*object).payload = payload;
    (*object).extent_himetric = (*object).payload.extent_himetric();
    (*object).dirty = true;

    if !(*object).storage.is_null() {
        log_ole_event("Writing OLE edit session payload to storage");
        let hr = write_ole_storage_payload(
            (*object).storage,
            &(*object).payload,
            (*object).extent_himetric,
        );
        log_ole_event(&format!(
            "Write OLE edit session payload to storage -> 0x{:08X}",
            hr as u32
        ));
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    log_ole_event("Notifying OLE container about edit session payload");
    notify_ole_object_changed(object);
    log_ole_event("Finished notifying OLE container about edit session payload");
    S_OK
}

pub(super) unsafe fn notify_ole_object_changed(object: *mut ChemSemaOleObject) {
    if object.is_null() {
        return;
    }
    if !(*object).ole_advise_holder.is_null() {
        let holder_vtbl = *((*object)
            .ole_advise_holder
            .cast::<*const OleAdviseHolderVtbl>());
        ((*holder_vtbl).send_on_save)((*object).ole_advise_holder);
    }
    if !(*object).data_advise_holder.is_null() {
        let holder_vtbl = *((*object)
            .data_advise_holder
            .cast::<*const DataAdviseHolderVtbl>());
        let data_object =
            (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();
        let hr =
            ((*holder_vtbl).send_on_data_change)((*object).data_advise_holder, data_object, 0, 0);
        log_ole_event(&format!(
            "IDataAdviseHolder::SendOnDataChange -> 0x{:08X}",
            hr as u32
        ));
    }
    if !(*object).view_advise_sink.is_null()
        && ((*object).view_advise_aspects == 0
            || ((*object).view_advise_aspects & DVASPECT_CONTENT) != 0)
    {
        let sink_vtbl = *((*object).view_advise_sink.cast::<*const AdviseSinkVtbl>());
        ((*sink_vtbl).on_view_change)((*object).view_advise_sink, DVASPECT_CONTENT, -1);
        log_ole_event("IAdviseSink::OnViewChange(DVASPECT_CONTENT)");
    }
    if !(*object).client_site.is_null() {
        let site_vtbl = *((*object).client_site.cast::<*const OleClientSiteVtbl>());
        let hr = ((*site_vtbl).save_object)((*object).client_site);
        log_ole_event(&format!(
            "IOleClientSite::SaveObject -> 0x{:08X}",
            hr as u32
        ));
        let hr = ((*site_vtbl).show_object)((*object).client_site);
        log_ole_event(&format!(
            "IOleClientSite::ShowObject -> 0x{:08X}",
            hr as u32
        ));
        let hr = ((*site_vtbl).request_new_object_layout)((*object).client_site);
        log_ole_event(&format!(
            "IOleClientSite::RequestNewObjectLayout -> 0x{:08X}",
            hr as u32
        ));
    }
}

pub(super) unsafe extern "system" fn class_factory_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    if object.is_null() {
        return E_POINTER;
    }
    *object = null_mut();
    if riid.is_null() {
        return E_NOINTERFACE;
    }
    if guid_eq(&*riid, &IID_IUNKNOWN) || guid_eq(&*riid, &IID_ICLASS_FACTORY) {
        *object = this;
        class_factory_add_ref(this);
        return S_OK;
    }
    E_NOINTERFACE
}

pub(super) unsafe extern "system" fn class_factory_add_ref(_this: *mut c_void) -> u32 {
    2
}

pub(super) unsafe extern "system" fn class_factory_release(_this: *mut c_void) -> u32 {
    1
}

pub(super) unsafe extern "system" fn class_factory_create_instance(
    _this: *mut c_void,
    outer: *mut c_void,
    riid: *const GUID,
    object: *mut *mut c_void,
) -> i32 {
    log_ole_event("IClassFactory::CreateInstance");
    if !object.is_null() {
        *object = null_mut();
    }
    if object.is_null() {
        return E_POINTER;
    }
    if !outer.is_null() {
        return CLASS_E_NOAGGREGATION;
    }
    if riid.is_null() {
        return E_NOINTERFACE;
    }
    let mut instance = Box::new(ChemSemaOleObject::new());
    instance.init_self_references();
    let instance = Box::into_raw(instance);
    let hr = chemsema_object_query_interface(instance, riid, object);
    chemsema_object_release(instance);
    log_ole_event(&format!(
        "IClassFactory::CreateInstance -> 0x{:08X}",
        hr as u32
    ));
    hr
}

pub(super) unsafe extern "system" fn class_factory_lock_server(
    _this: *mut c_void,
    _lock: i32,
) -> i32 {
    S_OK
}
