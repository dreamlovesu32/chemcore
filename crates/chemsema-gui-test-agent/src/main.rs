#![cfg_attr(windows, windows_subsystem = "windows")]

use chemsema_gui_test_agent::{InputGuard, AGENT_PROTOCOL};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorReport {
    schema: &'static str,
    status: &'static str,
    message: String,
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
fn run(args: &[String]) -> Result<serde_json::Value, String> {
    use chemsema_gui_test_agent::windows;
    match args.first().map(String::as_str) {
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
