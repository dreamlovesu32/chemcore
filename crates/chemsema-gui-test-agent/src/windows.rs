use crate::{AgentAttestation, ForegroundProcess, InputGuard, AGENT_PROTOCOL};
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FALSE, POINT, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::Security::Authentication::Identity::{
    LsaClose, LsaOpenPolicy, LsaStorePrivateData, LSA_HANDLE, LSA_OBJECT_ATTRIBUTES,
    LSA_UNICODE_STRING, POLICY_CREATE_SECRET,
};
use windows_sys::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_READOBJECTS,
    DESKTOP_SWITCHDESKTOP, UOI_NAME,
};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess,
    QueryFullProcessImageNameW, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE,
    MAPVK_VK_TO_VSC_EX, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VK_BACK,
    VK_CONTROL, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_MENU, VK_RETURN, VK_RIGHT, VK_SHIFT,
    VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow,
    GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    PostMessageW, SetCursorPos, SetForegroundWindow, SetProcessDPIAware, SetWindowPos, ShowWindowAsync, HWND_NOTOPMOST,
    HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, WM_CLOSE,
};

pub fn initialize_process() {
    // UI Automation and SendInput use physical screen coordinates. Mark every
    // short-lived and persistent agent process DPI-aware before it observes a
    // foreground rectangle so high-DPI physical workers retain one coordinate
    // system. A false return only means Windows fixed awareness earlier.
    unsafe { SetProcessDPIAware() };
}

fn last_error(context: &str) -> String {
    format!("{context} failed with Windows error {}", unsafe {
        GetLastError()
    })
}

fn enable_physical_pixel_coordinates() -> Result<(), String> {
    static DPI_AWARENESS: OnceLock<Result<(), String>> = OnceLock::new();
    DPI_AWARENESS
        .get_or_init(|| {
            if unsafe {
                SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            } == 0
            {
                Err(last_error("SetProcessDpiAwarenessContext"))
            } else {
                Ok(())
            }
        })
        .clone()
}

fn wide_string(buffer: &[u16], length: usize) -> String {
    String::from_utf16_lossy(&buffer[..length])
}

fn input_desktop_name() -> Result<Option<String>, String> {
    let desktop =
        unsafe { OpenInputDesktop(0, FALSE, DESKTOP_READOBJECTS | DESKTOP_SWITCHDESKTOP) };
    if desktop.is_null() {
        return Ok(None);
    }
    let result = (|| {
        let mut bytes_needed = 0u32;
        unsafe {
            GetUserObjectInformationW(desktop, UOI_NAME, null_mut(), 0, &mut bytes_needed);
        }
        if bytes_needed == 0 {
            return Err(last_error("GetUserObjectInformationW(size)"));
        }
        let mut buffer = vec![0u16; bytes_needed as usize / 2];
        let ok = unsafe {
            GetUserObjectInformationW(
                desktop,
                UOI_NAME,
                buffer.as_mut_ptr() as *mut c_void,
                bytes_needed,
                &mut bytes_needed,
            )
        };
        if ok == 0 {
            return Err(last_error("GetUserObjectInformationW"));
        }
        let length = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        Ok(Some(wide_string(&buffer, length)))
    })();
    unsafe { CloseDesktop(desktop) };
    result
}

fn window_text(window: *mut c_void) -> String {
    let mut buffer = vec![0u16; 4096];
    let length = unsafe { GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32) };
    if length <= 0 {
        String::new()
    } else {
        wide_string(&buffer, length as usize)
    }
}

fn window_class(window: *mut c_void) -> String {
    let mut buffer = vec![0u16; 512];
    let length = unsafe { GetClassNameW(window, buffer.as_mut_ptr(), buffer.len() as i32) };
    if length <= 0 {
        String::new()
    } else {
        wide_string(&buffer, length as usize)
    }
}

fn process_executable(process_id: u32) -> Result<PathBuf, String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id) };
    if process.is_null() {
        return Err(last_error("OpenProcess"));
    }
    let result = (|| {
        let mut buffer = vec![0u16; 32768];
        let mut length = buffer.len() as u32;
        let ok =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
        if ok == 0 {
            return Err(last_error("QueryFullProcessImageNameW"));
        }
        Ok(PathBuf::from(wide_string(&buffer, length as usize)))
    })();
    unsafe { CloseHandle(process) };
    result
}

fn application_user_model_id(process_id: u32) -> Result<String, String> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id) };
    if process.is_null() {
        return Err(last_error("OpenProcess(AppUserModelId)"));
    }
    let result = (|| {
        let mut length = 0u32;
        unsafe { GetApplicationUserModelId(process, &mut length, null_mut()) };
        if length == 0 {
            return Err("foreground process has no application user model id".to_string());
        }
        let mut buffer = vec![0u16; length as usize];
        let status =
            unsafe { GetApplicationUserModelId(process, &mut length, buffer.as_mut_ptr()) };
        if status != 0 {
            return Err(format!(
                "GetApplicationUserModelId failed with Windows error {status}"
            ));
        }
        let used = buffer
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(buffer.len());
        Ok(wide_string(&buffer, used))
    })();
    unsafe { CloseHandle(process) };
    result
}

fn foreground_process_once() -> Result<Option<ForegroundProcess>, String> {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return Ok(None);
    }
    let mut process_id = 0u32;
    unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    if process_id == 0 {
        return Err(last_error("GetWindowThreadProcessId"));
    }
    let mut session_id = 0u32;
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
        return Err(last_error("ProcessIdToSessionId(foreground)"));
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(window, &mut rect) } == 0 {
        return Err(last_error("GetWindowRect"));
    }
    let mut client = RECT::default();
    if unsafe { GetClientRect(window, &mut client) } == 0 {
        return Err(last_error("GetClientRect"));
    }
    let mut client_top_left = POINT {
        x: client.left,
        y: client.top,
    };
    let mut client_bottom_right = POINT {
        x: client.right,
        y: client.bottom,
    };
    if unsafe { ClientToScreen(window, &mut client_top_left) } == 0
        || unsafe { ClientToScreen(window, &mut client_bottom_right) } == 0
    {
        return Err(last_error("ClientToScreen"));
    }
    Ok(Some(ForegroundProcess {
        window_handle: window as usize as u64,
        process_id,
        session_id,
        executable: process_executable(process_id)?,
        title: window_text(window),
        class_name: window_class(window),
        rect: [rect.left, rect.top, rect.right, rect.bottom],
        client_rect: [
            client_top_left.x,
            client_top_left.y,
            client_bottom_right.x,
            client_bottom_right.y,
        ],
    }))
}

fn retryable_window_snapshot_error(message: &str) -> bool {
    message.contains("Windows error 1400")
}

fn foreground_process() -> Result<Option<ForegroundProcess>, String> {
    for attempt in 0..10 {
        match foreground_process_once() {
            Ok(snapshot) => return Ok(snapshot),
            Err(message) if retryable_window_snapshot_error(&message) => {
                if attempt < 9 {
                    thread::sleep(Duration::from_millis(25));
                }
            }
            Err(message) => return Err(message),
        }
    }
    Ok(None)
}

pub fn attest() -> Result<AgentAttestation, String> {
    enable_physical_pixel_coordinates()?;
    let process_id = unsafe { GetCurrentProcessId() };
    let mut session_id = 0u32;
    if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
        return Err(last_error("ProcessIdToSessionId(agent)"));
    }
    let input_desktop = input_desktop_name()?;
    let foreground = foreground_process()?;
    let interactive_ready = session_id != 0
        && input_desktop.as_deref() == Some("Default")
        && foreground
            .as_ref()
            .is_some_and(|value| value.session_id == session_id);
    let account = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(domain), Ok(user)) => format!("{domain}\\{user}"),
        (_, Ok(user)) => user,
        _ => String::new(),
    };
    Ok(AgentAttestation {
        schema: AGENT_PROTOCOL.to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        process_id,
        session_id,
        account,
        input_desktop,
        interactive_ready,
        foreground,
    })
}

fn mouse_input(flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_mouse(flags: u32) -> Result<(), String> {
    let input = mouse_input(flags);
    let sent = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
    if sent != 1 {
        return Err(last_error("SendInput"));
    }
    Ok(())
}

const CLICK_CURSOR_SETTLE: Duration = Duration::from_millis(25);
const CLICK_BUTTON_DWELL: Duration = Duration::from_millis(25);

fn deliver_click_with_timing(
    down: u32,
    up: u32,
    mut send: impl FnMut(u32) -> Result<(), String>,
    mut wait: impl FnMut(Duration),
) -> Result<(), String> {
    // SetCursorPos followed immediately by a zero-dwell down/up pair can be
    // accepted by SendInput without WebView2 dispatching a DOM click. Preserve
    // the real OS input path while giving the compositor one bounded interval
    // to settle the cursor and the control one interval to observe button-down.
    wait(CLICK_CURSOR_SETTLE);
    send(down)?;
    wait(CLICK_BUTTON_DWELL);
    send(up)
}

const TEXT_INPUT_EVENT_SETTLE: Duration = Duration::from_millis(100);

fn settle_after_text_input(mut wait: impl FnMut(Duration)) {
    // SendInput can return after accepting a Unicode batch while WebView2 is
    // still draining its key/input event queue. A following click or Escape
    // can otherwise be swallowed even though CDP already observes input.value.
    wait(TEXT_INPUT_EVENT_SETTLE);
}

fn uses_virtual_key_input(virtual_key: u16) -> bool {
    matches!(virtual_key, VK_DELETE | VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN)
}

fn send_key_event(virtual_key: u16, flags: u32) -> Result<(), String> {
    // Physical scan-code injection is stable across keyboard layouts and matches
    // the hardware path used by an actual keyboard more closely than a bare VK.
    let mapped = unsafe { MapVirtualKeyW(virtual_key as u32, MAPVK_VK_TO_VSC_EX) };
    if mapped == 0 {
        return Err(format!(
            "cannot map virtual key {virtual_key} to a scan code"
        ));
    }
    let extended = if mapped & 0xff00 != 0 {
        KEYEVENTF_EXTENDEDKEY
    } else {
        0
    };
    // WebView2 on a physical Windows desktop does not consistently surface an
    // injected extended Delete scan code as a DOM `Delete` key. Preserve the
    // OS SendInput path, but retain VK_DELETE in wVk so the window receives the
    // same logical key used by native menu accelerators and browser key events.
    let use_virtual_key = uses_virtual_key_input(virtual_key);
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: if use_virtual_key { virtual_key } else { 0 },
                wScan: if use_virtual_key { 0 } else { (mapped & 0xff) as u16 },
                dwFlags: flags | extended | if use_virtual_key { 0 } else { KEYEVENTF_SCANCODE },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    if unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) } != 1 {
        return Err(last_error("SendInput(keyboard)"));
    }
    Ok(())
}

fn parse_shortcut(shortcut: &str) -> Result<(Vec<u16>, u16), String> {
    let parts = shortcut
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let (key, modifiers) = parts
        .split_last()
        .ok_or_else(|| "keyboard shortcut is empty".to_string())?;
    let mut modifier_keys = Vec::new();
    for modifier in modifiers {
        let code = match modifier.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => VK_CONTROL,
            "shift" => VK_SHIFT,
            "alt" => VK_MENU,
            _ => return Err(format!("unsupported keyboard modifier {modifier}")),
        };
        if !modifier_keys.contains(&code) {
            modifier_keys.push(code);
        }
    }
    let normalized = key.to_ascii_lowercase();
    let key_code = match normalized.as_str() {
        "delete" => VK_DELETE,
        "backspace" => VK_BACK,
        "escape" => VK_ESCAPE,
        "enter" => VK_RETURN,
        "tab" => VK_TAB,
        "arrowleft" => VK_LEFT,
        "arrowright" => VK_RIGHT,
        "arrowup" => VK_UP,
        "arrowdown" => VK_DOWN,
        value if value.len() == 1 && value.as_bytes()[0].is_ascii_alphanumeric() => {
            value.as_bytes()[0].to_ascii_uppercase() as u16
        }
        _ => return Err(format!("unsupported keyboard key {key}")),
    };
    if key_code == VK_DELETE
        && modifier_keys.contains(&VK_CONTROL)
        && modifier_keys.contains(&VK_MENU)
    {
        return Err("Control+Alt+Delete is forbidden".to_string());
    }
    Ok((modifier_keys, key_code))
}

pub fn key(guard: &InputGuard, shortcut: &str) -> Result<AgentAttestation, String> {
    let before = attest()?;
    crate::validate_input_guard(&before, guard)?;
    let (modifiers, key_code) = parse_shortcut(shortcut)?;
    for modifier in &modifiers {
        send_key_event(*modifier, 0)?;
    }
    send_key_event(key_code, 0)?;
    send_key_event(key_code, KEYEVENTF_KEYUP)?;
    for modifier in modifiers.iter().rev() {
        send_key_event(*modifier, KEYEVENTF_KEYUP)?;
    }
    let after = attest()?;
    crate::validate_input_guard(&after, guard)?;
    Ok(after)
}

fn validate_text_input(text: &str) -> Result<Vec<u16>, String> {
    if text.is_empty() {
        return Err("text input is empty".to_string());
    }
    if text
        .chars()
        .any(|character| character == '\0' || character.is_control())
    {
        return Err("text input contains a control character".to_string());
    }
    let encoded = text.encode_utf16().collect::<Vec<_>>();
    if encoded.len() > 4096 {
        return Err("text input exceeds 4096 UTF-16 code units".to_string());
    }
    Ok(encoded)
}

pub fn text(guard: &InputGuard, value: &str) -> Result<AgentAttestation, String> {
    let before = attest()?;
    crate::validate_input_guard(&before, guard)?;
    let encoded = validate_text_input(value)?;
    let mut inputs = Vec::with_capacity(encoded.len() * 2);
    for code_unit in encoded {
        for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: code_unit,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            });
        }
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent as usize != inputs.len() {
        return Err(last_error("SendInput(text)"));
    }
    settle_after_text_input(thread::sleep);
    let after = attest()?;
    crate::validate_input_guard(&after, guard)?;
    Ok(after)
}

fn send_alt_key() -> Result<(), String> {
    let mut inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(last_error("SendInput(Alt)"));
    }
    Ok(())
}

fn button_flags(button: &str) -> Result<(u32, u32), String> {
    match button {
        "left" => Ok((MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)),
        "right" => Ok((MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)),
        "middle" => Ok((MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP)),
        _ => Err(format!("unsupported mouse button '{button}'")),
    }
}

fn parse_pointer_modifiers(value: &str) -> Result<Vec<u16>, String> {
    let mut keys = Vec::new();
    for modifier in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let key = match modifier.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => VK_CONTROL,
            "shift" => VK_SHIFT,
            "alt" => VK_MENU,
            _ => return Err(format!("unsupported pointer modifier {modifier}")),
        };
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    if keys.len() > 3 {
        return Err("pointer input accepts at most three modifiers".to_string());
    }
    Ok(keys)
}

fn with_modifier_keys<T>(
    modifiers: &[u16],
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let mut pressed = Vec::new();
    for modifier in modifiers {
        if let Err(error) = send_key_event(*modifier, 0) {
            for pressed_modifier in pressed.iter().rev() {
                let _ = send_key_event(*pressed_modifier, KEYEVENTF_KEYUP);
            }
            return Err(error);
        }
        pressed.push(*modifier);
    }
    let operation_result = operation();
    let mut release_error = None;
    for modifier in pressed.iter().rev() {
        if let Err(error) = send_key_event(*modifier, KEYEVENTF_KEYUP) {
            release_error.get_or_insert(error);
        }
    }
    match (operation_result, release_error) {
        (Err(error), Some(release)) => {
            Err(format!("{error}; modifier release also failed: {release}"))
        }
        (Err(error), None) => Err(error),
        (Ok(_), Some(release)) => Err(format!("modifier release failed: {release}")),
        (Ok(value), None) => Ok(value),
    }
}

fn validate_point(attestation: &AgentAttestation, x: i32, y: i32) -> Result<(), String> {
    let rect = attestation
        .foreground
        .as_ref()
        .ok_or_else(|| "no foreground window is available".to_string())?
        .rect;
    if x < rect[0] || y < rect[1] || x >= rect[2] || y >= rect[3] {
        return Err(format!(
            "input point ({x},{y}) is outside the authorized foreground window [{},{},{},{}]",
            rect[0], rect[1], rect[2], rect[3]
        ));
    }
    Ok(())
}

pub fn click(
    guard: &InputGuard,
    x: i32,
    y: i32,
    button: &str,
    modifiers: &str,
) -> Result<AgentAttestation, String> {
    let before = attest()?;
    crate::validate_input_guard(&before, guard)?;
    validate_point(&before, x, y)?;
    if unsafe { SetCursorPos(x, y) } == 0 {
        return Err(last_error("SetCursorPos"));
    }
    let (down, up) = button_flags(button)?;
    let modifier_keys = parse_pointer_modifiers(modifiers)?;
    with_modifier_keys(&modifier_keys, || {
        deliver_click_with_timing(down, up, send_mouse, thread::sleep)
    })?;
    let after = attest()?;
    crate::validate_input_guard(&after, guard)?;
    Ok(after)
}

struct WindowSearch {
    process_id: u32,
    window: *mut c_void,
}

unsafe extern "system" fn find_process_window(window: *mut c_void, parameter: isize) -> i32 {
    let search = &mut *(parameter as *mut WindowSearch);
    let mut process_id = 0u32;
    GetWindowThreadProcessId(window, &mut process_id);
    if process_id == search.process_id && IsWindowVisible(window) != 0 {
        search.window = window;
        return FALSE;
    }
    TRUE
}

fn process_window(process_id: u32) -> Result<*mut c_void, String> {
    let mut search = WindowSearch {
        process_id,
        window: null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(find_process_window),
            &mut search as *mut WindowSearch as isize,
        )
    };
    if search.window.is_null() {
        Err("authorized target process has no visible top-level window on this desktop".to_string())
    } else {
        Ok(search.window)
    }
}

pub fn activate(guard: &InputGuard) -> Result<AgentAttestation, String> {
    let before = attest()?;
    crate::validate_target_guard(&before, guard)?;
    let window = process_window(guard.expected_process_id)?;
    if window.is_null()
        || unsafe { IsWindow(window) } == 0
        || unsafe { IsWindowVisible(window) } == 0
    {
        return Err("authorized target window is absent or not visible".to_string());
    }
    let mut process_id = 0u32;
    let target_thread = unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    if target_thread == 0 || process_id != guard.expected_process_id {
        return Err("target window process id does not match the authorized target".to_string());
    }
    if !process_executable(process_id)?
        .to_string_lossy()
        .eq_ignore_ascii_case(&guard.expected_executable.to_string_lossy())
    {
        return Err("target window executable does not match the authorized target".to_string());
    }
    let foreground = unsafe { GetForegroundWindow() };
    let mut foreground_process_id = 0u32;
    let foreground_thread = if foreground.is_null() {
        0
    } else {
        unsafe { GetWindowThreadProcessId(foreground, &mut foreground_process_id) }
    };
    let current_thread = unsafe { GetCurrentThreadId() };
    if foreground_thread != 0 && foreground_thread != current_thread {
        unsafe { AttachThreadInput(current_thread, foreground_thread, 1) };
    }
    if target_thread != current_thread {
        unsafe { AttachThreadInput(current_thread, target_thread, 1) };
    }
    send_alt_key()?;
    unsafe {
        ShowWindowAsync(window, SW_RESTORE);
        SetWindowPos(
            window,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        BringWindowToTop(window);
        SetForegroundWindow(window);
        SetWindowPos(window, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    }
    if foreground_thread != 0 && foreground_thread != current_thread {
        unsafe { AttachThreadInput(current_thread, foreground_thread, 0) };
    }
    if target_thread != current_thread {
        unsafe { AttachThreadInput(current_thread, target_thread, 0) };
    }
    thread::sleep(Duration::from_millis(250));
    let after = attest()?;
    crate::validate_input_guard(&after, guard)?;
    Ok(after)
}

pub fn dismiss_known_blocker() -> Result<AgentAttestation, String> {
    const DEDICATED_GUEST_ACCOUNT_SUFFIX: &str = "\\chemsema-test";
    let before = attest()?;
    if !before
        .account
        .to_ascii_lowercase()
        .ends_with(DEDICATED_GUEST_ACCOUNT_SUFFIX)
        || before.session_id == 0
        || before.input_desktop.as_deref() != Some("Default")
    {
        return Err("agent is not on the dedicated interactive Default desktop".to_string());
    }
    let foreground = before
        .foreground
        .as_ref()
        .ok_or_else(|| "no foreground blocker is available".to_string())?;
    let executable = foreground.executable.to_string_lossy().to_ascii_lowercase();
    let title = foreground.title.to_ascii_lowercase();
    let known_title = title == "microsoft 账户" || title == "microsoft account";
    if !executable.ends_with("\\windows\\system32\\wwahost.exe")
        || foreground.class_name != "Windows.UI.Core.CoreWindow"
        || !known_title
    {
        return Err("foreground window is not an allowlisted test-environment blocker".to_string());
    }
    let app_id = application_user_model_id(foreground.process_id)?;
    if app_id != "Microsoft.Windows.CloudExperienceHost_cw5n1h2txyewy!App" {
        return Err(
            "foreground WWA host is not the allowlisted CloudExperienceHost app".to_string(),
        );
    }
    let window = foreground.window_handle as usize as *mut c_void;
    if unsafe { PostMessageW(window, WM_CLOSE, 0, 0) } == 0 {
        return Err(last_error("PostMessageW(WM_CLOSE)"));
    }
    thread::sleep(Duration::from_millis(500));
    let process = unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            FALSE,
            foreground.process_id,
        )
    };
    if !process.is_null() {
        let terminated = unsafe { TerminateProcess(process, 0) };
        unsafe { CloseHandle(process) };
        if terminated == 0 {
            return Err(last_error("TerminateProcess(CloudExperienceHost)"));
        }
    }
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(250));
        if let Ok(after) = attest() {
            if after.foreground.as_ref().is_none_or(|value| {
                value.window_handle != foreground.window_handle
                    || value.process_id != foreground.process_id
            }) {
                return Ok(after);
            }
        }
    }
    Err("allowlisted blocker did not close within five seconds".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        deliver_click_with_timing, parse_pointer_modifiers, parse_shortcut,
        retryable_window_snapshot_error, settle_after_text_input, uses_virtual_key_input,
        validate_text_input, CLICK_BUTTON_DWELL, CLICK_CURSOR_SETTLE, TEXT_INPUT_EVENT_SETTLE,
        VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_LEFT, VK_MENU, VK_RIGHT, VK_SHIFT, VK_UP,
    };
    use std::cell::RefCell;

    #[test]
    fn only_invalid_window_handles_are_retried_during_snapshot_capture() {
        assert!(retryable_window_snapshot_error(
            "GetWindowThreadProcessId failed with Windows error 1400"
        ));
        assert!(!retryable_window_snapshot_error(
            "OpenProcess failed with Windows error 5"
        ));
    }

    #[test]
    fn keyboard_shortcuts_are_allowlisted_and_secure_attention_is_forbidden() {
        assert!(parse_shortcut("Control+Z").is_ok());
        assert!(parse_shortcut("Shift+ArrowLeft").is_ok());
        assert_eq!(parse_shortcut("Backspace").unwrap().1, VK_BACK);
        assert!(parse_shortcut("Alt+F4").is_err());
        assert!(parse_shortcut("Control+Alt+Delete").is_err());
        assert!(parse_shortcut("Meta+R").is_err());
        for key in [VK_DELETE, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN] {
            assert!(uses_virtual_key_input(key));
        }
        assert!(!uses_virtual_key_input(b'A' as u16));
    }

    #[test]
    fn text_input_is_bounded_and_rejects_control_characters() {
        assert_eq!(
            validate_text_input(r"C:\ChemSemaGuiTest\documents\roundtrip.ccjs")
                .unwrap()
                .len(),
            r"C:\ChemSemaGuiTest\documents\roundtrip.ccjs"
                .encode_utf16()
                .count()
        );
        assert!(validate_text_input("").is_err());
        assert!(validate_text_input("line\nfeed").is_err());
        assert!(validate_text_input(&"a".repeat(4097)).is_err());
    }

    #[test]
    fn pointer_modifiers_are_bounded_allowlisted_and_deduplicated() {
        assert_eq!(parse_pointer_modifiers("").unwrap(), Vec::<u16>::new());
        assert_eq!(
            parse_pointer_modifiers("Shift,shift").unwrap(),
            vec![VK_SHIFT]
        );
        assert_eq!(
            parse_pointer_modifiers("Control,Alt").unwrap(),
            vec![VK_CONTROL, VK_MENU]
        );
        assert!(parse_pointer_modifiers("Windows").is_err());
    }

    #[test]
    fn physical_click_settles_before_down_and_dwells_before_up() {
        let events = RefCell::new(Vec::new());
        deliver_click_with_timing(
            10,
            20,
            |flag| {
                events.borrow_mut().push(format!("send:{flag}"));
                Ok(())
            },
            |duration| events.borrow_mut().push(format!("wait:{}", duration.as_millis())),
        )
        .unwrap();
        assert!(CLICK_CURSOR_SETTLE >= std::time::Duration::from_millis(20));
        assert!(CLICK_BUTTON_DWELL >= std::time::Duration::from_millis(20));
        assert_eq!(
            events.into_inner(),
            vec!["wait:25", "send:10", "wait:25", "send:20"]
        );
    }

    #[test]
    fn unicode_text_input_settles_before_the_next_physical_action() {
        let waits = RefCell::new(Vec::new());
        settle_after_text_input(|duration| waits.borrow_mut().push(duration));
        assert!(TEXT_INPUT_EVENT_SETTLE >= std::time::Duration::from_millis(100));
        assert_eq!(waits.into_inner(), vec![std::time::Duration::from_millis(100)]);
        assert!(include_str!("windows.rs").contains("settle_after_text_input(thread::sleep);"));
    }
}

pub fn drag(
    guard: &InputGuard,
    from: [i32; 2],
    to: [i32; 2],
    steps: u32,
    button: &str,
    modifiers: &str,
) -> Result<AgentAttestation, String> {
    if steps == 0 || steps > 1000 {
        return Err("drag steps must be in 1..=1000".to_string());
    }
    let before = attest()?;
    crate::validate_input_guard(&before, guard)?;
    validate_point(&before, from[0], from[1])?;
    validate_point(&before, to[0], to[1])?;
    if unsafe { SetCursorPos(from[0], from[1]) } == 0 {
        return Err(last_error("SetCursorPos(start)"));
    }
    let (down, up) = button_flags(button)?;
    let modifier_keys = parse_pointer_modifiers(modifiers)?;
    with_modifier_keys(&modifier_keys, || {
        send_mouse(down)?;
        for step in 1..=steps {
            let ratio = step as f64 / steps as f64;
            let x = from[0] as f64 + (to[0] - from[0]) as f64 * ratio;
            let y = from[1] as f64 + (to[1] - from[1]) as f64 * ratio;
            if unsafe { SetCursorPos(x.round() as i32, y.round() as i32) } == 0 {
                let _ = send_mouse(up);
                return Err(last_error("SetCursorPos(drag)"));
            }
            thread::sleep(Duration::from_millis(8));
        }
        send_mouse(up)
    })?;
    let after = attest()?;
    crate::validate_input_guard(&after, guard)?;
    Ok(after)
}

fn lsa_string(buffer: &mut [u16]) -> Result<LSA_UNICODE_STRING, String> {
    let bytes = buffer
        .len()
        .checked_mul(2)
        .ok_or_else(|| "LSA string length overflow".to_string())?;
    if bytes > u16::MAX as usize {
        return Err("LSA string exceeds the Windows length limit".to_string());
    }
    Ok(LSA_UNICODE_STRING {
        Length: bytes as u16,
        MaximumLength: bytes as u16,
        Buffer: buffer.as_mut_ptr(),
    })
}

pub fn store_autologon_secret(password: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("autologon password cannot be empty".to_string());
    }
    let mut key_buffer: Vec<u16> = "DefaultPassword".encode_utf16().collect();
    let mut password_buffer: Vec<u16> = password.encode_utf16().collect();
    let key = lsa_string(&mut key_buffer)?;
    let secret = lsa_string(&mut password_buffer)?;
    let mut attributes = LSA_OBJECT_ATTRIBUTES::default();
    attributes.Length = size_of::<LSA_OBJECT_ATTRIBUTES>() as u32;
    let mut policy: LSA_HANDLE = 0;
    let open_status = unsafe {
        LsaOpenPolicy(
            null(),
            &attributes,
            POLICY_CREATE_SECRET as u32,
            &mut policy,
        )
    };
    if open_status != 0 {
        password_buffer.fill(0);
        return Err(format!("LsaOpenPolicy failed with NTSTATUS {open_status}"));
    }
    let store_status = unsafe { LsaStorePrivateData(policy, &key, &secret) };
    unsafe { LsaClose(policy) };
    password_buffer.fill(0);
    if store_status != 0 {
        return Err(format!(
            "LsaStorePrivateData failed with NTSTATUS {store_status}"
        ));
    }
    Ok(())
}
