use chemsema_gui_test_agent::{InputGuard, AGENT_PROTOCOL};
use serde::Serialize;
use std::fs;
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
        _ => Err("usage: chemsema-gui-test-agent <attest|click|drag> ...".to_string()),
    }
}

#[cfg(not(windows))]
fn run(_args: &[String]) -> Result<serde_json::Value, String> {
    Err("chemsema-gui-test-agent is supported only on Windows".to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(value) => println!("{}", serde_json::to_string(&value).unwrap()),
        Err(message) => {
            eprintln!(
                "{}",
                serde_json::to_string(&ErrorReport {
                    schema: AGENT_PROTOCOL,
                    status: "failed",
                    message,
                })
                .unwrap()
            );
            std::process::exit(1);
        }
    }
}
