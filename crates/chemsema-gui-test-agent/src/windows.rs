use crate::{AgentAttestation, ForegroundProcess, InputGuard, AGENT_PROTOCOL};
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FALSE, RECT};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_READOBJECTS,
    DESKTOP_SWITCHDESKTOP, UOI_NAME,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEINPUT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    SetCursorPos,
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
