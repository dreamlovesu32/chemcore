use super::*;

pub(super) unsafe fn launch_desktop_for_object(
    object: *mut ChemSemaOleObject,
) -> Result<(), String> {
    if object.is_null() {
        return Err("Cannot launch OLE editor for a null object.".to_string());
    }
    if let Some(payload_path) = ole_edit_session_path_for_object(object) {
        log_ole_event(&format!(
            "Reusing OLE edit session payload at {}",
            payload_path.display()
        ));
        write_ole_edit_session_notify_file(&payload_path)?;
        let desktop_exe = resolve_desktop_exe()?;
        ensure_desktop_dev_server_for_debug_exe(&desktop_exe)?;
        log_ole_event(&format!(
            "Activating ChemSema desktop for existing OLE edit session from {}",
            desktop_exe.display()
        ));
        unsafe {
            AllowSetForegroundWindow(ASFW_ANY_PROCESS);
        }
        launch_desktop_process(&desktop_exe, &payload_path)?;
        return Ok(());
    }
    let session_id = format!(
        "{}-{}-{}",
        OLE_EDIT_SESSION_PREFIX,
        std::process::id(),
        monotonic_millis()
    );
    let payload_path = env::temp_dir().join(format!("{session_id}.ccjs"));
    let payload_json = ole_edit_session_payload_json(&(*object).payload)?;
    std::fs::write(&payload_path, payload_json)
        .map_err(|error| format!("Failed to write temporary OLE edit payload: {error}"))?;
    write_ole_edit_session_notify_file(&payload_path)?;
    log_ole_event(&format!(
        "Wrote OLE edit session {session_id} payload to {}",
        payload_path.display()
    ));
    register_ole_edit_session(session_id, payload_path.clone(), object)?;

    let desktop_exe = resolve_desktop_exe()?;
    ensure_desktop_dev_server_for_debug_exe(&desktop_exe)?;

    log_ole_event(&format!(
        "Launching ChemSema desktop from {}",
        desktop_exe.display()
    ));
    unsafe {
        AllowSetForegroundWindow(ASFW_ANY_PROCESS);
    }
    launch_desktop_process(&desktop_exe, &payload_path)?;
    Ok(())
}

pub(super) fn resolve_desktop_exe() -> Result<PathBuf, String> {
    if let Ok(override_path) = env::var(CHEMSEMA_DESKTOP_ENV) {
        let override_path = PathBuf::from(override_path);
        if override_path.exists() {
            log_ole_event(&format!(
                "Using {CHEMSEMA_DESKTOP_ENV} desktop executable at {}",
                override_path.display()
            ));
            return Ok(override_path);
        }
        log_ole_event(&format!(
            "{CHEMSEMA_DESKTOP_ENV} points to missing desktop executable at {}",
            override_path.display()
        ));
    }

    let server_path = current_server_path()?;
    let candidates = desktop_exe_candidates_for_server_path(&server_path);
    if let Some(candidate) = candidates.iter().find(|path| path.exists()) {
        return Ok(candidate.clone());
    }

    let searched = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "ChemSema desktop executable was not found. Searched: {searched}"
    ))
}

pub(super) fn desktop_exe_candidates_for_server_path(server_path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(server_dir) = server_path.parent() else {
        return candidates;
    };

    push_desktop_exe_candidates(&mut candidates, server_dir);
    if server_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("resources"))
    {
        if let Some(app_dir) = server_dir.parent() {
            push_desktop_exe_candidates(&mut candidates, app_dir);
        }
    }
    candidates
}

pub(super) fn push_desktop_exe_candidates(candidates: &mut Vec<PathBuf>, dir: &Path) {
    for name in [CHEMSEMA_DESKTOP_EXE_NAME, "ChemSema.exe", "chemsema.exe"] {
        let candidate = dir.join(name);
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
}

pub(super) fn launch_desktop_process(
    desktop_exe: &Path,
    payload_path: &Path,
) -> Result<(), String> {
    let mut command = Command::new(desktop_exe);
    command.arg(payload_path);
    if let Some(parent) = desktop_exe.parent() {
        command.current_dir(parent);
    }

    match command.spawn() {
        Ok(_) => Ok(()),
        Err(error) => {
            log_ole_event(&format!(
                "CreateProcess launch failed for {}: {error}",
                desktop_exe.display()
            ));
            shell_execute_desktop(desktop_exe, payload_path).map_err(|shell_error| {
                format!(
                    "Failed to launch ChemSema desktop with CreateProcess ({error}) and ShellExecuteW ({shell_error})"
                )
            })
        }
    }
}

pub(super) fn shell_execute_desktop(desktop_exe: &Path, payload_path: &Path) -> Result<(), String> {
    let operation = wide_null("open");
    let file = wide_path_null(desktop_exe);
    let parameters = wide_null(&quote_path(payload_path));
    let directory = desktop_exe.parent().map(wide_path_null);
    let directory_ptr = directory
        .as_ref()
        .map(|value| value.as_ptr())
        .unwrap_or(null());
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            directory_ptr,
            SW_SHOWNORMAL,
        )
    } as isize;
    if result > 32 {
        log_ole_event(&format!(
            "ShellExecuteW launch succeeded for {}",
            desktop_exe.display()
        ));
        return Ok(());
    }
    Err(format!("ShellExecuteW returned {result}"))
}

pub(super) fn ensure_desktop_dev_server_for_debug_exe(desktop_exe: &Path) -> Result<(), String> {
    let Some(debug_dir) = desktop_exe.parent() else {
        return Ok(());
    };
    if debug_dir.file_name().and_then(|name| name.to_str()) != Some("debug") {
        return Ok(());
    }
    let Some(target_dir) = debug_dir.parent() else {
        return Ok(());
    };
    if target_dir.file_name().and_then(|name| name.to_str()) != Some("target") {
        return Ok(());
    }
    let Some(repo_root) = target_dir.parent() else {
        return Ok(());
    };
    let server_script = repo_root.join("scripts").join("desktop-dev-server.mjs");
    if !server_script.exists() {
        return Ok(());
    }
    if desktop_dev_server_is_ready() {
        return Ok(());
    }

    log_ole_event(&format!(
        "Starting desktop dev server from {}",
        server_script.display()
    ));
    Command::new("node")
        .arg(server_script)
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW_FLAG)
        .spawn()
        .map_err(|error| format!("Failed to start ChemSema desktop dev server: {error}"))?;

    for _ in 0..30 {
        if desktop_dev_server_is_ready() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "ChemSema desktop dev server did not start at http://{DESKTOP_DEV_SERVER_ADDR}/"
    ))
}

pub(super) fn desktop_dev_server_is_ready() -> bool {
    let Ok(addr) = DESKTOP_DEV_SERVER_ADDR.parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
}

pub(super) fn log_ole_event(message: &str) {
    let line = format!(
        "[{} pid={}] {message}\n",
        monotonic_millis(),
        std::process::id()
    );
    let temp_path = env::temp_dir().join("chemsema-office.log");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(temp_path)
        .and_then(|mut file| file.write_all(line.as_bytes()));

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(parent.join("chemsema-office.log"))
                .and_then(|mut file| file.write_all(line.as_bytes()));
        }
    }
}

pub(super) fn monotonic_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(super) unsafe fn allocate_com_string(value: &str, out: *mut *mut u16) -> i32 {
    if out.is_null() {
        return E_POINTER;
    }
    let wide = wide_null(value);
    let bytes = wide.len() * std::mem::size_of::<u16>();
    let ptr = CoTaskMemAlloc(bytes).cast::<u16>();
    if ptr.is_null() {
        *out = null_mut();
        return E_OUTOFMEMORY;
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
    *out = ptr;
    S_OK
}

pub(super) fn guid_eq(left: &GUID, right: &GUID) -> bool {
    left.data1 == right.data1
        && left.data2 == right.data2
        && left.data3 == right.data3
        && left.data4 == right.data4
}

pub(super) fn hresult_succeeded(hr: i32) -> bool {
    hr >= 0
}

pub(super) fn print_help() {
    println!("{APP_NAME} Office/OLE integration server");
    println!();
    println!("Usage:");
    println!("  chemsema-office.exe --register-user");
    println!("  chemsema-office.exe --unregister-user");
    println!("  chemsema-office.exe --register-machine");
    println!("  chemsema-office.exe --unregister-machine");
    println!("  chemsema-office.exe --print-registration");
    println!("  chemsema-office.exe --self-test");
    println!("  chemsema-office.exe --copy-clipboard-payload <payload.json>");
    println!("  chemsema-office.exe --read-clipboard-payload <output.json>");
    println!("  chemsema-office.exe --write-word-docx-payload <payload.json> <output.docx>");
    println!("  chemsema-office.exe --write-emf-payload <payload.json> <output.emf>");
    println!("  chemsema-office.exe --write-preview-bounds-payload <payload.json> <output.json>");
    println!("  chemsema-office.exe --serve");
    println!();
    println!("COM may launch this executable with -Embedding or /Embedding.");
}
