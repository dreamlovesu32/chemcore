use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const AGENT_PROTOCOL: &str = "chemsema.gui.guest-agent.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForegroundProcess {
    pub window_handle: u64,
    pub process_id: u32,
    pub session_id: u32,
    pub executable: PathBuf,
    pub title: String,
    pub class_name: String,
    pub rect: [i32; 4],
    pub client_rect: [i32; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAttestation {
    pub schema: String,
    pub agent_version: String,
    pub process_id: u32,
    pub session_id: u32,
    pub account: String,
    pub input_desktop: Option<String>,
    pub interactive_ready: bool,
    pub foreground: Option<ForegroundProcess>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputGuard {
    pub expected_agent_account: String,
    pub expected_agent_session_id: u32,
    pub expected_process_id: u32,
    pub expected_executable: PathBuf,
    pub allowed_run_root: PathBuf,
    pub run_directory: PathBuf,
}

pub fn validate_input_guard(
    attestation: &AgentAttestation,
    guard: &InputGuard,
) -> Result<(), String> {
    if !attestation.account.eq_ignore_ascii_case(&guard.expected_agent_account) {
        return Err("input agent account does not match the authorized worker account".to_string());
    }
    if !attestation.interactive_ready {
        return Err("guest input desktop is not interactive and unlocked".to_string());
    }
    if attestation.session_id == 0 || attestation.session_id != guard.expected_agent_session_id {
        return Err("agent session does not match the authorized interactive session".to_string());
    }
    let foreground = attestation
        .foreground
        .as_ref()
        .ok_or_else(|| "no foreground window is available".to_string())?;
    if foreground.session_id != attestation.session_id {
        return Err("foreground window belongs to another session".to_string());
    }
    if foreground.process_id != guard.expected_process_id {
        return Err(format!(
            "foreground process id {} does not match authorized target {}; executable {}",
            foreground.process_id,
            guard.expected_process_id,
            foreground.executable.display()
        ));
    }
    if !same_path(&foreground.executable, &guard.expected_executable) {
        return Err("foreground executable does not match the authorized target".to_string());
    }
    if !is_bounded_child(&guard.allowed_run_root, &guard.run_directory)? {
        return Err("run directory is outside the authorized guest test root".to_string());
    }
    Ok(())
}

pub fn validate_target_guard(
    attestation: &AgentAttestation,
    guard: &InputGuard,
) -> Result<(), String> {
    if !attestation.account.eq_ignore_ascii_case(&guard.expected_agent_account) {
        return Err("input agent account does not match the authorized worker account".to_string());
    }
    if attestation.session_id == 0
        || attestation.session_id != guard.expected_agent_session_id
        || attestation.input_desktop.as_deref() != Some("Default")
    {
        return Err(
            "agent is not attached to the authorized interactive Default desktop".to_string(),
        );
    }
    if !is_bounded_child(&guard.allowed_run_root, &guard.run_directory)? {
        return Err("run directory is outside the authorized guest test root".to_string());
    }
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

pub fn is_bounded_child(root: &Path, child: &Path) -> Result<bool, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve allowed run root: {error}"))?;
    let child = child
        .canonicalize()
        .map_err(|error| format!("cannot resolve run directory: {error}"))?;
    Ok(child != root && child.starts_with(root))
}

#[cfg(windows)]
pub mod windows;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(root: &Path, run: &Path) -> (AgentAttestation, InputGuard) {
        let executable = PathBuf::from(r"C:\Program Files\ChemSema\ChemSema.exe");
        (
            AgentAttestation {
                schema: AGENT_PROTOCOL.to_string(),
                agent_version: env!("CARGO_PKG_VERSION").to_string(),
                process_id: 10,
                session_id: 2,
                account: "guest\\chemsema-test".to_string(),
                input_desktop: Some("Default".to_string()),
                interactive_ready: true,
                foreground: Some(ForegroundProcess {
                    window_handle: 100,
                    process_id: 50,
                    session_id: 2,
                    executable: executable.clone(),
                    title: "ChemSema".to_string(),
                    class_name: "WebView2".to_string(),
                    rect: [0, 0, 1600, 1000],
                    client_rect: [8, 1, 1592, 992],
                }),
            },
            InputGuard {
                expected_agent_account: "guest\\chemsema-test".to_string(),
                expected_agent_session_id: 2,
                expected_process_id: 50,
                expected_executable: executable,
                allowed_run_root: root.to_path_buf(),
                run_directory: run.to_path_buf(),
            },
        )
    }

    #[test]
    fn input_guard_requires_session_process_executable_and_bounded_run_root() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("chemsema-gui-agent-{unique}"));
        let run = root.join("run-1");
        fs::create_dir_all(&run).unwrap();
        let (attestation, guard) = fixture(&root, &run);
        assert_eq!(validate_input_guard(&attestation, &guard), Ok(()));

        let mut wrong = guard.clone();
        wrong.expected_process_id += 1;
        assert!(validate_input_guard(&attestation, &wrong)
            .unwrap_err()
            .contains("process id"));

        let outside = std::env::temp_dir().join(format!("chemsema-gui-agent-outside-{unique}"));
        fs::create_dir_all(&outside).unwrap();
        let mut wrong = guard.clone();
        wrong.run_directory = outside.clone();
        assert!(validate_input_guard(&attestation, &wrong)
            .unwrap_err()
            .contains("outside"));
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn input_guard_rejects_locked_or_session_zero_agents() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("chemsema-gui-agent-locked-{unique}"));
        let run = root.join("run-1");
        fs::create_dir_all(&run).unwrap();
        let (mut attestation, guard) = fixture(&root, &run);
        attestation.interactive_ready = false;
        assert!(validate_input_guard(&attestation, &guard).is_err());
        attestation.interactive_ready = true;
        attestation.session_id = 0;
        assert!(validate_input_guard(&attestation, &guard).is_err());
        attestation.session_id = 2;
        attestation.account = "host\\developer".to_string();
        assert!(validate_input_guard(&attestation, &guard)
            .unwrap_err()
            .contains("worker account"));
        fs::remove_dir_all(&root).unwrap();
    }
}
