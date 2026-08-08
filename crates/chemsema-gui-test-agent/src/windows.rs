use crate::{
    AgentAttestation, ForegroundProcess, InputGuard, AGENT_PROTOCOL, AUTHORIZED_ACCOUNT_SUFFIX,
};
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FALSE, RECT, TRUE};
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
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VK_MENU,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible, PostMessageW,
    SetCursorPos, SetForegroundWindow, SetWindowPos, ShowWindowAsync, HWND_NOTOPMOST, HWND_TOPMOST,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, WM_CLOSE,
};

fn last_error(context: &str) -> String {
    format!("{context} failed with Windows error {}", unsafe {
        GetLastError()
    })
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

fn foreground_process() -> Result<Option<ForegroundProcess>, String> {
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
    Ok(Some(ForegroundProcess {
        window_handle: window as usize as u64,
        process_id,
        session_id,
        executable: process_executable(process_id)?,
        title: window_text(window),
        class_name: window_class(window),
        rect: [rect.left, rect.top, rect.right, rect.bottom],
    }))
}

pub fn attest() -> Result<AgentAttestation, String> {
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

fn validate_point(attestation: &AgentAttestation, x: i32, y: i32) -> Result<(), String> {
    let rect = attestation
        .foreground
        .as_ref()
        .ok_or_else(|| "no foreground window is available".to_string())?
        .rect;
    if x < rect[0] || y < rect[1] || x >= rect[2] || y >= rect[3] {
        return Err("input point is outside the authorized foreground window".to_string());
    }
    Ok(())
}

pub fn click(guard: &InputGuard, x: i32, y: i32, button: &str) -> Result<AgentAttestation, String> {
    let before = attest()?;
    crate::validate_input_guard(&before, guard)?;
    validate_point(&before, x, y)?;
    if unsafe { SetCursorPos(x, y) } == 0 {
        return Err(last_error("SetCursorPos"));
    }
    let (down, up) = button_flags(button)?;
    send_mouse(down)?;
    send_mouse(up)?;
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
    let before = attest()?;
    if !before
        .account
        .to_ascii_lowercase()
        .ends_with(AUTHORIZED_ACCOUNT_SUFFIX)
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

pub fn drag(
    guard: &InputGuard,
    from: [i32; 2],
    to: [i32; 2],
    steps: u32,
    button: &str,
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
    send_mouse(up)?;
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
