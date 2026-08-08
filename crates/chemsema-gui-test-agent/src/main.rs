#![cfg_attr(windows, windows_subsystem = "windows")]

use chemsema_gui_test_agent::{InputGuard, AGENT_PROTOCOL};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorReport {
    schema: &'static str,
    status: &'static str,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRequest {
    schema: String,
    id: String,
    args: Vec<String>,
}

fn validate_server_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("click" | "drag") => Ok(()),
        _ => Err("persistent agent accepts only click or drag requests".to_string()),
    }
}

fn value(args: &[String], name: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|argument| argument == name)
        .ok_or_else(|| format!("missing required argument {name}"))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value for {name}"))
}

fn number<T: std::str::FromStr>(args: &[String], name: &str) -> Result<T, String> {
    value(args, name)?
        .parse()
        .map_err(|_| format!("invalid numeric value for {name}"))
}

fn guard(args: &[String]) -> Result<InputGuard, String> {
    let path = value(args, "--guard")?;
    let source = fs::read_to_string(Path::new(&path))
        .map_err(|error| format!("cannot read guard file: {error}"))?;
    serde_json::from_str(&source).map_err(|error| format!("invalid guard JSON: {error}"))
}

#[cfg(windows)]
fn serve(args: &[String]) -> Result<serde_json::Value, String> {
    let allowed_root = std::path::PathBuf::from(value(args, "--allowed-root")?);
    let channel_root = std::path::PathBuf::from(value(args, "--channel-root")?);
    fs::create_dir_all(channel_root.join("inbox")).map_err(|error| error.to_string())?;
    fs::create_dir_all(channel_root.join("outbox")).map_err(|error| error.to_string())?;
    if !chemsema_gui_test_agent::is_bounded_child(&allowed_root, &channel_root)? {
        return Err("persistent agent channel is outside the authorized test root".to_string());
    }
    let ready_path = channel_root.join("ready.json");
    fs::write(
        &ready_path,
        format!("{{\"schema\":\"chemsema.gui.guest-agent-server.v1\",\"status\":\"ready\",\"processId\":{}}}\n", std::process::id()),
    ).map_err(|error| error.to_string())?;
    loop {
        if channel_root.join("shutdown").exists() {
            break;
        }
        let mut requests = fs::read_dir(channel_root.join("inbox"))
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        requests.sort();
        for request_path in requests {
            let claim_path = request_path.with_extension("claim");
            if fs::rename(&request_path, &claim_path).is_err() {
                continue;
            }
            let response = (|| {
                let source = fs::read_to_string(&claim_path).map_err(|error| error.to_string())?;
                let request: ServerRequest =
                    serde_json::from_str(&source).map_err(|error| error.to_string())?;
                if request.schema != "chemsema.gui.guest-agent-request.v1" {
                    return Err("unsupported persistent agent request schema".to_string());
                }
                let file_id = claim_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if request.id != file_id || request.id.is_empty() {
                    return Err(
                        "persistent agent request id does not match its file name".to_string()
                    );
                }
                validate_server_command(&request.args)?;
                run(&request.args)
            })();
            let id = claim_path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("invalid");
            let envelope = match response {
                Ok(result) => {
                    serde_json::json!({"schema":"chemsema.gui.guest-agent-response.v1","id":id,"status":"passed","result":result})
                }
                Err(message) => {
                    serde_json::json!({"schema":"chemsema.gui.guest-agent-response.v1","id":id,"status":"failed","message":message})
                }
            };
            let output = channel_root.join("outbox").join(format!("{id}.json"));
            let temporary = output.with_extension("tmp");
            fs::write(
                &temporary,
                format!("{}\n", serde_json::to_string(&envelope).unwrap()),
            )
            .map_err(|error| error.to_string())?;
            fs::rename(&temporary, &output).map_err(|error| error.to_string())?;
            let _ = fs::remove_file(&claim_path);
        }
        thread::sleep(Duration::from_millis(20));
    }
    Ok(serde_json::json!({"schema":"chemsema.gui.guest-agent-server.v1","status":"stopped"}))
}

#[cfg(windows)]
fn run(args: &[String]) -> Result<serde_json::Value, String> {
    use chemsema_gui_test_agent::windows;
    match args.first().map(String::as_str) {
        Some("serve") => serve(args),
        Some("attest") => {
            serde_json::to_value(windows::attest()?).map_err(|error| error.to_string())
        }
        Some("click") => {
            let attestation = windows::click(
                &guard(args)?,
                number(args, "--x")?,
                number(args, "--y")?,
                &value(args, "--button")?,
            )?;
            serde_json::to_value(attestation).map_err(|error| error.to_string())
        }
        Some("drag") => {
            let attestation = windows::drag(
                &guard(args)?,
                [number(args, "--from-x")?, number(args, "--from-y")?],
                [number(args, "--to-x")?, number(args, "--to-y")?],
                number(args, "--steps")?,
                &value(args, "--button")?,
            )?;
            serde_json::to_value(attestation).map_err(|error| error.to_string())
        }
        Some("activate") => {
            let attestation = windows::activate(&guard(args)?)?;
            serde_json::to_value(attestation).map_err(|error| error.to_string())
        }
        Some("dismiss-known-blocker") => serde_json::to_value(windows::dismiss_known_blocker()?)
            .map_err(|error| error.to_string()),
        Some("store-autologon-secret") => {
            let mut password = String::new();
            std::io::stdin()
                .read_to_string(&mut password)
                .map_err(|error| format!("cannot read autologon secret from stdin: {error}"))?;
            if password.ends_with("\n") {
                password.pop();
                if password.ends_with("\r") {
                    password.pop();
                }
            }
            windows::store_autologon_secret(&password)?;
            password.replace_range(.., "");
            Ok(serde_json::json!({
                "schema": AGENT_PROTOCOL,
                "status": "stored"
            }))
        }
        _ => Err(
            "usage: chemsema-gui-test-agent <attest|activate|click|drag|dismiss-known-blocker> ..."
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_server_command;

    #[test]
    fn persistent_server_rejects_non_input_commands() {
        assert!(validate_server_command(&["click".to_string()]).is_ok());
        assert!(validate_server_command(&["drag".to_string()]).is_ok());
        assert!(validate_server_command(&["store-autologon-secret".to_string()]).is_err());
        assert!(validate_server_command(&["attest".to_string()]).is_err());
    }
}

#[cfg(not(windows))]
fn run(_args: &[String]) -> Result<serde_json::Value, String> {
    Err("chemsema-gui-test-agent is supported only on Windows".to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(result_value) => {
            let json = serde_json::to_string(&result_value).unwrap();
            if let Ok(output) = value(&args, "--output") {
                let output = std::path::PathBuf::from(output);
                let parent = output.parent().unwrap_or_else(|| Path::new("."));
                if let Err(error) = fs::create_dir_all(parent) {
                    eprintln!("cannot create output directory: {error}");
                    std::process::exit(1);
                }
                let temporary = output.with_extension("tmp");
                if let Err(error) = fs::write(&temporary, format!("{json}\n"))
                    .and_then(|_| fs::rename(&temporary, &output))
                {
                    eprintln!("cannot write attestation output: {error}");
                    std::process::exit(1);
                }
            } else {
                println!("{json}");
            }
        }
        Err(message) => {
            let json = serde_json::to_string(&ErrorReport {
                schema: AGENT_PROTOCOL,
                status: "failed",
                message,
            })
            .unwrap();
            if let Ok(output) = value(&args, "--output") {
                let output = std::path::PathBuf::from(output);
                let parent = output.parent().unwrap_or_else(|| Path::new("."));
                let _ = fs::create_dir_all(parent);
                let _ = fs::write(output, format!("{json}\n"));
            } else {
                eprintln!("{json}");
            }
            std::process::exit(1);
        }
    }
}
