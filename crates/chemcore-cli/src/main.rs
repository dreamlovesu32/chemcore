mod agent;
mod protocol;

use chemcore_desktop_service::DesktopDocumentService;
use chemcore_engine::Engine;
use protocol::{
    about_command, capabilities_command, doctor_command, examples_command, guide_command,
    schema_command, schema_or_capabilities_for_help, version_command, version_text, CliError,
    CliResult,
};
use serde_json::Map;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const DOCUMENT_HASH_ALGORITHM: &str = "sha256";
const DOCUMENT_HASH_INPUT: &str = "chemcore-document-json-v1";
const IMPORT_CACHE_VERSION: &str = "chemcore-cli-import-cache-v1";

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(error) => {
            if let Err(write_error) = write_json_value(error.to_json(), None, false) {
                eprintln!("failed to write cli error json: {write_error}");
            }
            1
        }
    };
    std::process::exit(exit_code);
}

fn run() -> CliResult<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().map(String::as_str) else {
        capabilities_command(&[]).map_err(CliError::message)?;
        return Ok(());
    };
    if matches!(command, "-V" | "--version") {
        println!("{}", version_text());
        return Ok(());
    }
    if matches!(command, "-h" | "--help" | "help") {
        let help_args = if command == "help" { &args[1..] } else { &[] };
        schema_or_capabilities_for_help(help_args).map_err(CliError::message)?;
        return Ok(());
    }
    if args[1..]
        .iter()
        .any(|argument| matches!(argument.as_str(), "-h" | "--help"))
    {
        schema_or_capabilities_for_help(std::slice::from_ref(&args[0]))
            .map_err(CliError::message)?;
        return Ok(());
    }

    match command {
        "version" => version_command(&args[1..]).map_err(CliError::message),
        "capabilities" => capabilities_command(&args[1..]).map_err(CliError::message),
        "schema" => schema_command(&args[1..]).map_err(CliError::message),
        "doctor" => doctor_command(&args[1..]).map_err(CliError::message),
        "about" => about_command(&args[1..]).map_err(CliError::message),
        "examples" => examples_command(&args[1..]).map_err(CliError::message),
        "guide" => guide_command(&args[1..]).map_err(CliError::message),
        "targets" => agent::targets_command(&args[1..])
            .map_err(|error| CliError::for_command("targets", error)),
        "capture" => agent::capture_command(&args[1..])
            .map_err(|error| CliError::for_command("capture", error)),
        "context" => agent::context_command(&args[1..])
            .map_err(|error| CliError::for_command("context", error)),
        "detail" | "details" | "describe" | "show" => agent::detail_command(&args[1..])
            .map_err(|error| CliError::for_command("detail", error)),
        "copy" => {
            agent::copy_command(&args[1..]).map_err(|error| CliError::for_command("copy", error))
        }
        "session" => agent::session_command(&args[1..])
            .map_err(|error| CliError::for_command("session", error)),
        "inspect" => {
            inspect_command(&args[1..]).map_err(|error| CliError::for_command("inspect", error))
        }
        "new" => new_command(&args[1..]).map_err(|error| CliError::for_command("new", error)),
        "convert" => {
            convert_command(&args[1..]).map_err(|error| CliError::for_command("convert", error))
        }
        "export" => {
            convert_command(&args[1..]).map_err(|error| CliError::for_command("export", error))
        }
        "run" => {
            run_command_script(&args[1..]).map_err(|error| CliError::for_command("run", error))
        }
        other => Err(CliError::unknown_command(other)),
    }
}

fn new_command(args: &[String]) -> Result<(), String> {
    let mut script = None;
    let mut output = None;
    let mut save_format = None;
    let mut results = None;
    let mut document_json_output = None;
    let mut inspect_after = default_inspect_after();
    let mut continue_on_error = false;
    let mut pretty = false;
    let mut quiet = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--save-format" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--save-format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--results" => {
                index += 1;
                results = Some(
                    args.get(index)
                        .ok_or_else(|| "--results requires a path.".to_string())?
                        .clone(),
                );
            }
            "--document-json" => {
                index += 1;
                document_json_output = Some(
                    args.get(index)
                        .ok_or_else(|| "--document-json requires a path.".to_string())?
                        .clone(),
                );
            }
            "--inspect-after" => {
                index += 1;
                inspect_after = parse_inspect_after_value(
                    args.get(index)
                        .ok_or_else(|| "--inspect-after requires a value.".to_string())?,
                );
            }
            "--no-inspect-after" => inspect_after = None,
            "--continue-on-error" => continue_on_error = true,
            "--pretty" => pretty = true,
            "--quiet" => quiet = true,
            value if script.is_none() => script = Some(value.to_string()),
            value => return Err(format!("Unexpected new argument '{value}'.")),
        }
        index += 1;
    }

    let output = output.ok_or_else(|| {
        "new requires --out <path>; primary document output has no default path.".to_string()
    })?;
    if document_json_output.as_deref() == Some("-") && !quiet && results.is_none() {
        return Err("Use --results or --quiet when --document-json is '-'.".to_string());
    }
    let mut engine = Engine::new();
    let mut execution = if let Some(script) = script.as_deref() {
        execute_command_file(
            &mut engine,
            script,
            inspect_after.as_deref(),
            continue_on_error,
        )
    } else {
        empty_script_execution(&mut engine, inspect_after.as_deref())
    };
    set_report_field(
        &mut execution.report,
        "io",
        json!({
            "operation": "new",
            "input": null,
            "script": script.as_deref(),
            "output": {
                "path": output.as_str(),
                "format": save_format
                    .as_deref()
                    .map(str::to_string)
                    .or_else(|| infer_format_from_path(&output)),
            },
        }),
    );
    write_optional_document_json(
        &mut execution,
        &engine,
        document_json_output.as_deref(),
        "documentJson",
    );
    if execution.ok {
        match write_engine_output(&engine, &output, save_format.as_deref()) {
            Ok(()) => set_report_field(
                &mut execution.report,
                "save",
                json!({
                    "ok": true,
                    "path": output,
                    "format": save_format
                        .as_deref()
                        .map(str::to_string)
                        .or_else(|| infer_format_from_path(&output)),
                }),
            ),
            Err(error) => {
                execution.ok = false;
                execution.error_message = Some(error.clone());
                set_report_field(
                    &mut execution.report,
                    "save",
                    json!({
                        "ok": false,
                        "path": output,
                        "error": {
                            "stage": "save-output",
                            "message": error,
                        }
                    }),
                );
            }
        }
    } else {
        let reason = execution
            .error_message
            .clone()
            .unwrap_or_else(|| "command script failed".to_string());
        set_report_field(
            &mut execution.report,
            "save",
            json!({
                "ok": false,
                "path": output,
                "skipped": true,
                "reason": reason,
            }),
        );
    }
    set_report_field(&mut execution.report, "ok", json!(execution.ok));
    if !quiet {
        write_json_value(execution.report.clone(), results.as_deref(), pretty)?;
    }
    if !execution.ok {
        return Err(execution
            .error_message
            .unwrap_or_else(|| "Command script failed.".to_string()));
    }
    Ok(())
}

fn inspect_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut include = Vec::new();
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--include" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--include requires a value.".to_string())?;
                include = split_csv(value);
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected inspect argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "inspect requires an input file.".to_string())?;
    let mut engine = load_engine_from_file(&input)?;
    let mut command = json!({ "type": "inspect-document" });
    if !include.is_empty() {
        command["include"] = json!(include);
    }
    let result = execute_json_command(&mut engine, command)?;
    write_json_value(
        result
            .get("output")
            .cloned()
            .ok_or_else(|| "inspect command did not return output.".to_string())?,
        output.as_deref(),
        pretty,
    )
}

fn convert_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--format" | "-f" => {
                index += 1;
                format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            value if input.is_none() => input = Some(value.to_string()),
            value if output.is_none() => output = Some(value.to_string()),
            value => return Err(format!("Unexpected convert/export argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "convert/export requires an input file.".to_string())?;
    let output = output.ok_or_else(|| {
        "convert/export requires an output path; primary document output has no default path."
            .to_string()
    })?;
    let engine = load_engine_from_file(&input)?;
    write_engine_output(&engine, &output, format.as_deref())
}

fn run_command_script(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut script = None;
    let mut output = None;
    let mut save_format = None;
    let mut results = None;
    let mut document_json_output = None;
    let mut inspect_after = default_inspect_after();
    let mut continue_on_error = false;
    let mut pretty = false;
    let mut quiet = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--save-format" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--save-format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--results" => {
                index += 1;
                results = Some(
                    args.get(index)
                        .ok_or_else(|| "--results requires a path.".to_string())?
                        .clone(),
                );
            }
            "--document-json" => {
                index += 1;
                document_json_output = Some(
                    args.get(index)
                        .ok_or_else(|| "--document-json requires a path.".to_string())?
                        .clone(),
                );
            }
            "--inspect-after" => {
                index += 1;
                inspect_after = parse_inspect_after_value(
                    args.get(index)
                        .ok_or_else(|| "--inspect-after requires a value.".to_string())?,
                );
            }
            "--no-inspect-after" => inspect_after = None,
            "--continue-on-error" => continue_on_error = true,
            "--pretty" => pretty = true,
            "--quiet" => quiet = true,
            value if input.is_none() => input = Some(value.to_string()),
            value if script.is_none() => script = Some(value.to_string()),
            value => return Err(format!("Unexpected run argument '{value}'.")),
        }
        index += 1;
    }

    let input = input.ok_or_else(|| "run requires an input file.".to_string())?;
    let script = script.ok_or_else(|| "run requires a command JSON file.".to_string())?;
    if output.as_deref() == Some("-") && !quiet && results.is_none() {
        return Err("Use --results or --quiet when --out is '-'.".to_string());
    }
    if document_json_output.as_deref() == Some("-") && !quiet && results.is_none() {
        return Err("Use --results or --quiet when --document-json is '-'.".to_string());
    }

    let mut engine = load_engine_from_file(&input)?;
    let mut execution = execute_command_file(
        &mut engine,
        &script,
        inspect_after.as_deref(),
        continue_on_error,
    );
    let output_io = output
        .as_deref()
        .map(|path| {
            json!({
                "path": path,
                "format": save_format
                    .as_deref()
                    .map(str::to_string)
                    .or_else(|| infer_format_from_path(path)),
            })
        })
        .unwrap_or(Value::Null);
    set_report_field(
        &mut execution.report,
        "io",
        json!({
            "operation": "run",
            "input": {
                "path": input.as_str(),
            },
            "script": script.as_str(),
            "output": output_io,
        }),
    );
    write_optional_document_json(
        &mut execution,
        &engine,
        document_json_output.as_deref(),
        "documentJson",
    );

    if execution.ok {
        if let Some(output) = output.as_deref() {
            match write_engine_output(&engine, output, save_format.as_deref()) {
                Ok(()) => set_report_field(
                    &mut execution.report,
                    "save",
                    json!({
                        "ok": true,
                        "path": output,
                        "format": save_format
                            .as_deref()
                            .map(str::to_string)
                            .or_else(|| infer_format_from_path(output)),
                    }),
                ),
                Err(error) => {
                    execution.ok = false;
                    execution.error_message = Some(error.clone());
                    set_report_field(
                        &mut execution.report,
                        "save",
                        json!({
                            "ok": false,
                            "path": output,
                            "error": {
                                "stage": "save-output",
                                "message": error,
                            }
                        }),
                    );
                }
            }
        } else {
            set_report_field(
                &mut execution.report,
                "save",
                json!({
                    "ok": true,
                    "skipped": true,
                    "reason": "--out was not provided",
                    "warning": "No output document was saved. Pass --out <path> to save the edited document.",
                }),
            );
        }
    } else {
        let reason = execution
            .error_message
            .clone()
            .unwrap_or_else(|| "command script failed".to_string());
        set_report_field(
            &mut execution.report,
            "save",
            json!({
                "ok": false,
                "skipped": true,
                "reason": reason,
            }),
        );
    }
    set_report_field(&mut execution.report, "ok", json!(execution.ok));
    if !quiet {
        write_json_value(execution.report.clone(), results.as_deref(), pretty)?;
    }
    if !execution.ok {
        return Err(execution
            .error_message
            .unwrap_or_else(|| "Command script failed.".to_string()));
    }
    Ok(())
}

#[derive(Debug)]
struct ScriptExecution {
    ok: bool,
    report: Value,
    error_message: Option<String>,
}

fn execute_command_file(
    engine: &mut Engine,
    script: &str,
    inspect_after: Option<&[String]>,
    continue_on_error: bool,
) -> ScriptExecution {
    let commands = match read_command_values(script) {
        Ok(commands) => commands,
        Err(error) => {
            let script_before_hash = document_hash(engine);
            let script_before_revision = engine.revision();
            let mut report = json!({
                "ok": false,
                "commandCount": 0,
                "executedCount": 0,
                "failedCount": 0,
                "failedIndex": null,
                "failedIndices": [],
                "continueOnError": continue_on_error,
                "commands": [],
                "error": {
                    "stage": "read-script",
                    "message": error,
                },
            });
            append_script_document_summary(
                &mut report,
                script_before_hash,
                script_before_revision,
                engine,
            );
            return ScriptExecution {
                ok: false,
                error_message: Some(error.clone()),
                report,
            };
        }
    };
    execute_command_values(engine, commands, inspect_after, continue_on_error)
}

fn execute_command_values(
    engine: &mut Engine,
    commands: Vec<Value>,
    inspect_after: Option<&[String]>,
    continue_on_error: bool,
) -> ScriptExecution {
    let command_count = commands.len();
    let mut entries = Vec::new();
    let mut executed_count = 0usize;
    let mut failed_indices = Vec::new();
    let mut first_error_message = None;
    let script_before_hash = document_hash(engine);
    let script_before_revision = engine.revision();
    for (index, command) in commands.into_iter().enumerate() {
        let command_type = command_type_name(&command);
        let before_hash = document_hash(engine);
        let before_revision = engine.revision();
        match execute_json_command(engine, command.clone()) {
            Ok(engine_result) => {
                executed_count += 1;
                let changed = engine_result
                    .get("changed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let after_hash = document_hash(engine);
                let after_revision = engine.revision();
                let created = engine_result
                    .get("created")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let updated = engine_result
                    .get("updated")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let deleted = engine_result
                    .get("deleted")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let change_summary = change_summary_json(&engine_result);
                let mut entry = json!({
                    "index": index,
                    "ok": true,
                    "executed": true,
                    "changed": changed,
                    "commandType": command_type,
                    "command": command,
                    "revision": engine_result.get("revision").cloned().unwrap_or(Value::Null),
                    "beforeRevision": engine_result
                        .get("beforeRevision")
                        .cloned()
                        .unwrap_or(Value::Null),
                    "document": document_transition_json(
                        &before_hash,
                        before_revision,
                        &after_hash,
                        after_revision,
                    ),
                    "changeSummary": change_summary,
                    "targets": engine_result.get("targets").cloned().unwrap_or_else(|| json!({})),
                    "created": created,
                    "updated": updated,
                    "deleted": deleted,
                    "diagnostics": engine_result
                        .get("diagnostics")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                    "engineResult": engine_result,
                });
                append_after_snapshot(engine, &mut entry, inspect_after);
                entries.push(entry);
            }
            Err(error) => {
                failed_indices.push(index);
                if first_error_message.is_none() {
                    first_error_message = Some(error.clone());
                }
                let after_hash = document_hash(engine);
                let after_revision = engine.revision();
                entries.push(json!({
                    "index": index,
                    "ok": false,
                    "executed": false,
                    "changed": false,
                    "commandType": command_type,
                    "command": command,
                    "document": document_transition_json(
                        &before_hash,
                        before_revision,
                        &after_hash,
                        after_revision,
                    ),
                    "changeSummary": empty_change_summary_json(),
                    "error": {
                        "stage": "execute-command",
                        "message": error,
                    },
                }));
                if continue_on_error {
                    continue;
                }
                let error_message = entries
                    .last()
                    .and_then(|entry| entry.pointer("/error/message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Command failed.")
                    .to_string();
                let mut report = json!({
                    "ok": false,
                    "commandCount": command_count,
                    "executedCount": executed_count,
                    "failedCount": 1,
                    "failedIndex": index,
                    "failedIndices": [index],
                    "continueOnError": continue_on_error,
                    "commands": entries,
                    "error": {
                        "stage": "execute-command",
                        "message": error_message,
                    },
                });
                append_script_document_summary(
                    &mut report,
                    script_before_hash,
                    script_before_revision,
                    engine,
                );
                append_final_snapshot(engine, &mut report, inspect_after);
                return ScriptExecution {
                    ok: false,
                    report,
                    error_message: Some(error_message),
                };
            }
        }
    }
    if !failed_indices.is_empty() {
        let error_message = if failed_indices.len() == 1 {
            first_error_message.unwrap_or_else(|| "Command failed.".to_string())
        } else {
            format!("{} commands failed.", failed_indices.len())
        };
        let mut report = json!({
            "ok": false,
            "commandCount": command_count,
            "executedCount": executed_count,
            "failedCount": failed_indices.len(),
            "failedIndex": failed_indices.first().copied(),
            "failedIndices": failed_indices,
            "continueOnError": continue_on_error,
            "commands": entries,
            "error": {
                "stage": "execute-command",
                "message": error_message,
            },
        });
        append_script_document_summary(
            &mut report,
            script_before_hash,
            script_before_revision,
            engine,
        );
        append_final_snapshot(engine, &mut report, inspect_after);
        return ScriptExecution {
            ok: false,
            report,
            error_message: Some(error_message),
        };
    }
    let mut report = json!({
        "ok": true,
        "commandCount": command_count,
        "executedCount": executed_count,
        "failedCount": 0,
        "failedIndex": null,
        "failedIndices": [],
        "continueOnError": continue_on_error,
        "commands": entries,
    });
    append_script_document_summary(
        &mut report,
        script_before_hash,
        script_before_revision,
        engine,
    );
    append_final_snapshot(engine, &mut report, inspect_after);
    ScriptExecution {
        ok: true,
        report,
        error_message: None,
    }
}

fn empty_script_execution(
    engine: &mut Engine,
    inspect_after: Option<&[String]>,
) -> ScriptExecution {
    let mut report = json!({
        "ok": true,
        "commandCount": 0,
        "executedCount": 0,
        "failedCount": 0,
        "failedIndex": null,
        "failedIndices": [],
        "continueOnError": false,
        "commands": [],
    });
    append_script_document_summary(
        &mut report,
        document_hash(engine),
        engine.revision(),
        engine,
    );
    append_final_snapshot(engine, &mut report, inspect_after);
    ScriptExecution {
        ok: true,
        report,
        error_message: None,
    }
}

fn append_after_snapshot(engine: &mut Engine, entry: &mut Value, inspect_after: Option<&[String]>) {
    let Some(include) = inspect_after else {
        return;
    };
    match inspect_engine(engine, include) {
        Ok(snapshot) => set_report_field(entry, "after", snapshot),
        Err(error) => set_report_field(
            entry,
            "afterError",
            json!({
                "stage": "inspect-after",
                "message": error,
            }),
        ),
    }
}

fn append_final_snapshot(
    engine: &mut Engine,
    report: &mut Value,
    inspect_after: Option<&[String]>,
) {
    let Some(include) = inspect_after else {
        return;
    };
    match inspect_engine(engine, include) {
        Ok(snapshot) => set_report_field(report, "final", snapshot),
        Err(error) => set_report_field(
            report,
            "finalError",
            json!({
                "stage": "inspect-final",
                "message": error,
            }),
        ),
    }
}

fn append_script_document_summary(
    report: &mut Value,
    before_hash: Option<String>,
    before_revision: u64,
    engine: &Engine,
) {
    let after_hash = document_hash(engine);
    set_report_field(
        report,
        "document",
        document_transition_json(
            &before_hash,
            before_revision,
            &after_hash,
            engine.revision(),
        ),
    );
}

fn document_transition_json(
    before_hash: &Option<String>,
    before_revision: u64,
    after_hash: &Option<String>,
    after_revision: u64,
) -> Value {
    json!({
        "hashAlgorithm": DOCUMENT_HASH_ALGORITHM,
        "hashInput": DOCUMENT_HASH_INPUT,
        "beforeHash": before_hash,
        "afterHash": after_hash,
        "hashChanged": hash_changed_value(before_hash, after_hash),
        "beforeRevision": before_revision,
        "afterRevision": after_revision,
    })
}

fn hash_changed_value(before_hash: &Option<String>, after_hash: &Option<String>) -> Value {
    match (before_hash, after_hash) {
        (Some(before_hash), Some(after_hash)) => json!(before_hash != after_hash),
        _ => Value::Null,
    }
}

fn document_hash(engine: &Engine) -> Option<String> {
    document_json(engine).ok().map(|text| {
        let digest = Sha256::digest(text.as_bytes());
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    })
}

fn change_summary_json(engine_result: &Value) -> Value {
    change_summary_from_targets(
        engine_result.get("created"),
        engine_result.get("updated"),
        engine_result.get("deleted"),
    )
}

fn empty_change_summary_json() -> Value {
    change_summary_from_targets(None, None, None)
}

fn change_summary_from_targets(
    created: Option<&Value>,
    updated: Option<&Value>,
    deleted: Option<&Value>,
) -> Value {
    let created_selectors = target_selector_group(created);
    let updated_selectors = target_selector_group(updated);
    let deleted_selectors = target_selector_group(deleted);
    let touched_selectors =
        combined_touched_selectors(&[&created_selectors, &updated_selectors, &deleted_selectors]);
    json!({
        "createdCount": selector_group_count(&created_selectors),
        "updatedCount": selector_group_count(&updated_selectors),
        "deletedCount": selector_group_count(&deleted_selectors),
        "createdSelectors": created_selectors,
        "updatedSelectors": updated_selectors,
        "deletedSelectors": deleted_selectors,
        "touchedSelectors": touched_selectors,
    })
}

fn target_selector_group(targets: Option<&Value>) -> Value {
    json!({
        "objects": target_selector_values(targets, "objects", "object:"),
        "nodes": target_selector_values(targets, "nodes", "node:"),
        "bonds": target_selector_values(targets, "bonds", "bond:"),
        "styles": target_selector_values(targets, "styles", "style:"),
    })
}

fn target_selector_values(targets: Option<&Value>, key: &str, prefix: &str) -> Vec<String> {
    targets
        .and_then(|targets| targets.get(key))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|id| format!("{prefix}{id}"))
                .collect()
        })
        .unwrap_or_default()
}

fn selector_group_count(group: &Value) -> usize {
    selector_group_values(group).len()
}

fn selector_group_values(group: &Value) -> Vec<String> {
    let mut values = Vec::new();
    for key in ["objects", "nodes", "bonds", "styles"] {
        let Some(items) = group.get(key).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            if let Some(selector) = item.as_str() {
                values.push(selector.to_string());
            }
        }
    }
    values
}

fn combined_touched_selectors(groups: &[&Value]) -> Vec<String> {
    let mut selectors = Vec::new();
    for group in groups {
        for selector in selector_group_values(group) {
            if !selectors.contains(&selector) {
                selectors.push(selector);
            }
        }
    }
    selectors
}

fn inspect_engine(engine: &mut Engine, include: &[String]) -> Result<Value, String> {
    let result = execute_json_command(
        engine,
        json!({
            "type": "inspect-document",
            "include": include,
        }),
    )?;
    result
        .get("output")
        .cloned()
        .ok_or_else(|| "inspect command did not return output.".to_string())
}

fn write_optional_document_json(
    execution: &mut ScriptExecution,
    engine: &Engine,
    path: Option<&str>,
    report_key: &str,
) {
    let Some(path) = path else {
        return;
    };
    match document_json(engine)
        .and_then(|text| write_text_output(Some(path), &format!("{text}\n")).map(|_| ()))
    {
        Ok(()) => set_report_field(
            &mut execution.report,
            report_key,
            json!({
                "ok": true,
                "path": path,
                "format": "json",
            }),
        ),
        Err(error) => {
            execution.ok = false;
            execution.error_message = Some(error.clone());
            set_report_field(
                &mut execution.report,
                report_key,
                json!({
                    "ok": false,
                    "path": path,
                    "error": {
                        "stage": "write-document-json",
                        "message": error,
                    },
                }),
            );
        }
    }
}

fn command_type_name(command: &Value) -> Value {
    command
        .get("type")
        .and_then(Value::as_str)
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn set_report_field(report: &mut Value, key: &str, value: Value) {
    if !report.is_object() {
        *report = Value::Object(Map::new());
    }
    if let Some(object) = report.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

fn load_engine_from_file(path: &str) -> Result<Engine, String> {
    let mut service = DesktopDocumentService::default();
    let opened = service.read_document_file(path)?;
    let mut engine = Engine::new();
    match opened.format.as_str() {
        "ccjs" | "ccjz" => engine.load_document_json(&opened.text)?,
        "cdxml" | "cdx" => {
            load_cdxml_document_with_cache(&mut engine, &opened.text, &opened.format)?
        }
        "sdf" => engine.load_sdf_document(&opened.text)?,
        "svg" => {
            return Err(
                "SVG is an export format and cannot be opened as an editable document.".to_string(),
            );
        }
        format => return Err(format!("Unsupported input format '{format}'.")),
    }
    Ok(engine)
}

fn load_cdxml_document_with_cache(
    engine: &mut Engine,
    source_text: &str,
    format: &str,
) -> Result<(), String> {
    if cli_import_cache_enabled() {
        let cache_path = import_cache_path(source_text, format);
        if cache_path.is_file() {
            let cache_result = fs::read_to_string(&cache_path)
                .map_err(|error| {
                    format!(
                        "Failed to read import cache {}: {error}",
                        cache_path.display()
                    )
                })
                .and_then(|cached_json| {
                    let mut cached_engine = Engine::new();
                    cached_engine.load_document_json(&cached_json)?;
                    Ok(cached_engine)
                });
            match cache_result {
                Ok(cached_engine) => {
                    *engine = cached_engine;
                    return Ok(());
                }
                Err(_) => {
                    let _ = fs::remove_file(&cache_path);
                }
            }
        }

        engine.load_cdxml_document(source_text)?;
        if let Ok(document_json) = document_json(engine) {
            let _ = write_import_cache(&cache_path, &document_json);
        }
        return Ok(());
    }

    engine.load_cdxml_document(source_text)
}

pub(crate) fn cli_import_cache_enabled() -> bool {
    !std::env::var("CHEMCORE_CLI_DISABLE_CACHE")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub(crate) fn cli_cache_dir() -> PathBuf {
    if let Ok(path) = std::env::var("CHEMCORE_CLI_CACHE_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(path) = std::env::var("LOCALAPPDATA") {
            if !path.trim().is_empty() {
                return PathBuf::from(path).join("ChemCore").join("cli-cache");
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(path) = std::env::var("XDG_CACHE_HOME") {
            if !path.trim().is_empty() {
                return PathBuf::from(path).join("chemcore").join("cli-cache");
            }
        }
        if let Ok(path) = std::env::var("HOME") {
            if !path.trim().is_empty() {
                return PathBuf::from(path)
                    .join(".cache")
                    .join("chemcore")
                    .join("cli-cache");
            }
        }
    }
    std::env::temp_dir().join("chemcore-cli").join("cache")
}

fn import_cache_path(source_text: &str, format: &str) -> PathBuf {
    cli_cache_dir()
        .join("imports")
        .join(IMPORT_CACHE_VERSION)
        .join(format!("{}.ccjs", import_cache_key(source_text, format)))
}

fn import_cache_key(source_text: &str, format: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(IMPORT_CACHE_VERSION.as_bytes());
    digest.update(b"\0");
    digest.update(env!("CARGO_PKG_VERSION").as_bytes());
    digest.update(b"\0");
    digest.update(current_exe_cache_stamp().as_bytes());
    digest.update(b"\0");
    digest.update(format.as_bytes());
    digest.update(b"\0");
    digest.update(source_text.as_bytes());
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn current_exe_cache_stamp() -> String {
    let Some(metadata) = std::env::current_exe()
        .ok()
        .and_then(|path| fs::metadata(path).ok())
    else {
        return "unknown-exe".to_string();
    };
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{}:{modified_ms}", metadata.len())
}

fn write_import_cache(path: &Path, document_json: &str) -> Result<u64, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create import cache directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let temp_path = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temp_path, document_json.as_bytes()).map_err(|error| {
        format!(
            "Failed to write import cache {}: {error}",
            temp_path.display()
        )
    })?;
    verify_file_written_exact(&temp_path, document_json.len() as u64, "import cache")?;
    fs::rename(&temp_path, path).map_err(|error| {
        format!(
            "Failed to move import cache into place {}: {error}",
            path.display()
        )
    })?;
    verify_file_written_exact(path, document_json.len() as u64, "import cache")
}

fn write_engine_output(engine: &Engine, path: &str, format: Option<&str>) -> Result<(), String> {
    let format = format
        .map(normalize_format)
        .transpose()?
        .or_else(|| infer_format_from_path(path))
        .ok_or_else(|| "Output format is ambiguous; pass --format.".to_string())?;

    if path == "-" {
        return write_engine_output_to_stdout(engine, &format);
    }

    ensure_output_parent(path)?;
    let mut service = DesktopDocumentService::default();
    match format.as_str() {
        "json" | "ccjs" => service.write_document_file(path, &document_json(engine)?, Some("ccjs")),
        "ccjz" => service.write_document_file(path, &document_json(engine)?, Some("ccjz")),
        "cdxml" => service.write_document_file(path, &engine.document_cdxml(), Some("cdxml")),
        "cdx" => service.write_document_file(path, &engine.document_cdxml(), Some("cdx")),
        "sdf" => service.write_document_file(path, &engine.document_sdf()?, Some("sdf")),
        "svg" => service.write_document_file(path, &engine.document_svg(), Some("svg")),
        _ => Err(format!("Unsupported output format '{format}'.")),
    }?;
    verify_file_written(Path::new(path), 1, "document output")?;
    Ok(())
}

fn write_engine_output_to_stdout(engine: &Engine, format: &str) -> Result<(), String> {
    match format {
        "json" | "ccjs" => write_stdout_text(&document_json(engine)?),
        "ccjz" => Err("Writing compressed ccjz to stdout is not supported.".to_string()),
        "cdxml" => write_stdout_text(&engine.document_cdxml()),
        "cdx" => io::stdout()
            .write_all(&engine.document_cdx()?)
            .map_err(|error| error.to_string()),
        "sdf" => write_stdout_text(&engine.document_sdf()?),
        "svg" => write_stdout_text(&engine.document_svg()),
        _ => Err(format!("Unsupported output format '{format}'.")),
    }
}

fn read_command_values(path: &str) -> Result<Vec<Value>, String> {
    let text = if path == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| error.to_string())?;
        input
    } else {
        fs::read_to_string(path).map_err(|error| format!("Failed to read {path}: {error}"))?
    };
    let text = text.trim_start_matches('\u{feff}');
    let value: Value = serde_json::from_str(text)
        .map_err(|error| format!("Invalid command JSON in {path}: {error}"))?;
    match value {
        Value::Array(commands) => Ok(commands),
        Value::Object(_) => Ok(vec![value]),
        _ => Err("Command JSON must be an object or an array of objects.".to_string()),
    }
}

fn execute_json_command(engine: &mut Engine, command: Value) -> Result<Value, String> {
    let result = engine.execute_command_json(&command.to_string())?;
    serde_json::from_str(&result).map_err(|error| error.to_string())
}

fn write_json_value(value: Value, path: Option<&str>, pretty: bool) -> Result<(), String> {
    let text = if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    }
    .map_err(|error| error.to_string())?;
    write_text_output(path, &format!("{text}\n")).map(|_| ())
}

fn write_text_output(path: Option<&str>, text: &str) -> Result<u64, String> {
    match path {
        Some("-") | None => write_stdout_text(text).map(|_| text.len() as u64),
        Some(path) => {
            ensure_output_parent(path)?;
            fs::write(path, text).map_err(|error| format!("Failed to write {path}: {error}"))?;
            verify_file_written_exact(Path::new(path), text.len() as u64, "text output")
        }
    }
}

pub(crate) fn ensure_output_parent(path: &str) -> Result<(), String> {
    if path == "-" {
        return Ok(());
    }
    ensure_output_parent_path(Path::new(path))
}

pub(crate) fn ensure_output_parent_path(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create output directory {}: {error}",
            parent.display()
        )
    })?;
    if !parent.is_dir() {
        return Err(format!(
            "Failed to verify output directory {} after creating it.",
            parent.display()
        ));
    }
    Ok(())
}

pub(crate) fn verify_file_written(path: &Path, min_bytes: u64, label: &str) -> Result<u64, String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "Failed to verify {label} at {} after writing: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "Failed to verify {label} at {} after writing: path is not a regular file.",
            path.display()
        ));
    }
    let bytes = metadata.len();
    if bytes < min_bytes {
        return Err(format!(
            "Failed to verify {label} at {} after writing: file has {bytes} bytes, expected at least {min_bytes}.",
            path.display()
        ));
    }
    Ok(bytes)
}

pub(crate) fn verify_file_written_exact(
    path: &Path,
    expected_bytes: u64,
    label: &str,
) -> Result<u64, String> {
    let bytes = verify_file_written(path, expected_bytes, label)?;
    if bytes != expected_bytes {
        return Err(format!(
            "Failed to verify {label} at {} after writing: file has {bytes} bytes, expected {expected_bytes}.",
            path.display()
        ));
    }
    Ok(bytes)
}

fn write_stdout_text(text: &str) -> Result<(), String> {
    io::stdout()
        .write_all(text.as_bytes())
        .map_err(|error| error.to_string())
}

fn document_json(engine: &Engine) -> Result<String, String> {
    engine.document_json().map_err(|error| error.to_string())
}

fn normalize_format(value: &str) -> Result<String, String> {
    let normalized = value.trim().trim_start_matches('.').to_ascii_lowercase();
    let normalized = match normalized.as_str() {
        "json" => "json",
        "ccjs" => "ccjs",
        "ccjz" => "ccjz",
        "cdxml" => "cdxml",
        "cdx" => "cdx",
        "sdf" | "sd" => "sdf",
        "svg" => "svg",
        _ => return Err(format!("Unsupported format '{value}'.")),
    };
    Ok(normalized.to_string())
}

fn infer_format_from_path(path: &str) -> Option<String> {
    if path == "-" {
        return None;
    }
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| normalize_format(extension).ok())
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn default_inspect_after() -> Option<Vec<String>> {
    None
}

fn parse_inspect_after_value(value: &str) -> Option<Vec<String>> {
    let normalized = value.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "none" | "off" | "false" | "0") {
        return None;
    }
    let include = split_csv(value);
    if include.is_empty() {
        None
    } else {
        Some(include)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_include() -> Vec<String> {
        vec![
            "summary".to_string(),
            "objects".to_string(),
            "molecules".to_string(),
        ]
    }

    #[test]
    fn normalizes_common_formats() {
        assert_eq!(normalize_format("json").unwrap(), "json");
        assert_eq!(normalize_format(".cdxml").unwrap(), "cdxml");
        assert_eq!(normalize_format("sd").unwrap(), "sdf");
        assert!(normalize_format("png").is_err());
    }

    #[test]
    fn infers_format_from_output_path() {
        assert_eq!(infer_format_from_path("out.svg").as_deref(), Some("svg"));
        assert_eq!(infer_format_from_path("out.json").as_deref(), Some("json"));
        assert_eq!(infer_format_from_path("-"), None);
    }

    #[test]
    fn text_file_output_is_verified_after_write() {
        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-write-verify-{}.json",
            std::process::id()
        ));
        let bytes = write_text_output(Some(path.to_str().unwrap()), "{\"ok\":true}\n").unwrap();
        assert_eq!(bytes, 12);
        assert_eq!(fs::metadata(&path).unwrap().len(), 12);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_agent_target_selectors() {
        assert_eq!(
            agent::parse_target_selector("all").unwrap(),
            agent::TargetSelector::All
        );
        assert_eq!(
            agent::parse_target_selector("object:obj_1").unwrap(),
            agent::TargetSelector::Object("obj_1".to_string())
        );
        assert_eq!(
            agent::parse_target_selector("mol:2").unwrap(),
            agent::TargetSelector::Molecule(2)
        );
        assert_eq!(
            agent::parse_target_selector("atom:n_1").unwrap(),
            agent::TargetSelector::Node("n_1".to_string())
        );
        assert_eq!(
            agent::parse_target_selector("bond:b_1").unwrap(),
            agent::TargetSelector::Bond("b_1".to_string())
        );
        assert_eq!(
            agent::parse_target_selector("object:obj_1;bond:b_1").unwrap(),
            agent::TargetSelector::Selection(vec![
                agent::TargetSelector::Object("obj_1".to_string()),
                agent::TargetSelector::Bond("b_1".to_string()),
            ])
        );
        assert_eq!(
            agent::parse_target_selector("selection:node:n_1;bond:b_1").unwrap(),
            agent::TargetSelector::Selection(vec![
                agent::TargetSelector::Node("n_1".to_string()),
                agent::TargetSelector::Bond("b_1".to_string()),
            ])
        );
        assert!(agent::parse_target_selector("molecule:not-a-number").is_err());
    }

    #[test]
    fn schema_topics_accept_agent_friendly_aliases() {
        assert_eq!(protocol::schema_topic_key("protocol"), Some("protocol"));
        assert_eq!(protocol::schema_topic_key("version"), Some("protocol"));
        assert_eq!(protocol::schema_topic_key("target"), Some("target"));
        assert_eq!(protocol::schema_topic_key("targets"), Some("target"));
        assert_eq!(protocol::schema_topic_key("context"), Some("context"));
        assert_eq!(protocol::schema_topic_key("nearby"), Some("context"));
        assert_eq!(protocol::schema_topic_key("neighbors"), Some("context"));
        assert_eq!(protocol::schema_topic_key("detail"), Some("detail"));
        assert_eq!(protocol::schema_topic_key("object-detail"), Some("detail"));
        assert_eq!(protocol::schema_topic_key("guide"), Some("guide"));
        assert_eq!(protocol::schema_topic_key("agent-guide"), Some("guide"));
        assert_eq!(protocol::schema_topic_key("clipboard"), Some("copy"));
        assert_eq!(
            protocol::schema_topic_key("json-output"),
            Some("jsonOutput")
        );
        assert_eq!(protocol::schema_topic_key("pretty"), Some("jsonOutput"));
        assert_eq!(
            protocol::schema_topic_key("command-script"),
            Some("commandScript")
        );
    }

    #[test]
    fn version_metadata_is_machine_readable() {
        assert_eq!(
            protocol::version_text(),
            format!("chemcore-cli {}", env!("CARGO_PKG_VERSION"))
        );
        let value = protocol::version_value();
        assert_eq!(value["ok"], true);
        assert_eq!(value["cli"], "chemcore-cli");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["protocol"], protocol::CLI_PROTOCOL_VERSION);
        assert_eq!(
            value["protocols"]["session"],
            protocol::SESSION_PROTOCOL_VERSION
        );
    }

    #[test]
    fn missing_argument_errors_include_machine_readable_fix() {
        let error = protocol::CliError::for_command(
            "capture",
            "capture requires --target <object:id|molecule:index|node:id|bond:id|all> or --bounds."
                .to_string(),
        )
        .to_json();

        assert_eq!(error["error"]["kind"], "missing_argument");
        assert_eq!(error["error"]["argument"], "--target");
        assert_eq!(error["error"]["fix"]["action"], "provide_required_argument");
        assert_eq!(error["error"]["fix"]["missing"], "--target");
        assert!(error["error"]["fix"]["usage"]
            .as_str()
            .unwrap()
            .contains("chemcore-cli capture"));
        assert_eq!(
            error["error"]["suggestions"][0]["action"],
            "provide_required_argument"
        );
    }

    #[test]
    fn missing_flag_value_errors_include_expected_value() {
        let error = protocol::CliError::for_command(
            "capture",
            "--scale requires a positive number.".to_string(),
        )
        .to_json();

        assert_eq!(error["error"]["kind"], "missing_argument");
        assert_eq!(error["error"]["argument"], "--scale");
        assert_eq!(error["error"]["fix"]["missing"], "--scale");
        assert_eq!(error["error"]["fix"]["expected"], "a positive number");
        assert!(error["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("chemcore-cli help capture"));
    }

    #[test]
    fn command_json_accepts_single_object_or_array() {
        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-command-test-{}-single.json",
            std::process::id()
        ));
        fs::write(&path, r#"{ "type": "inspect-document" }"#).unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            1
        );
        let _ = fs::remove_file(path);

        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-command-test-{}-array.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"[{ "type": "inspect-document" }, { "type": "export-document", "format": "svg" }]"#,
        )
        .unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            2
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn command_json_accepts_utf8_bom() {
        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-command-test-{}-bom.json",
            std::process::id()
        ));
        fs::write(&path, "\u{feff}{ \"type\": \"inspect-document\" }").unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            1
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn import_cache_key_changes_with_source_or_format() {
        let base_key = import_cache_key("<CDXML><page /></CDXML>", "cdxml");

        assert_ne!(
            base_key,
            import_cache_key("<CDXML><page id=\"2\" /></CDXML>", "cdxml")
        );
        assert_ne!(base_key, import_cache_key("<CDXML><page /></CDXML>", "cdx"));
    }

    #[test]
    fn import_cache_write_verifies_written_bytes() {
        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-import-cache-test-{}.ccjs",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let bytes = write_import_cache(&path, "{\"nodes\":[],\"bonds\":[]}\n").unwrap();

        assert_eq!(bytes, 24);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\"nodes\":[],\"bonds\":[]}\n"
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn execution_report_includes_after_snapshot_for_success() {
        let mut engine = Engine::new();
        let include = snapshot_include();
        let execution = execute_command_values(
            &mut engine,
            vec![json!({
                "type": "add-bond",
                "begin": { "x": 100.0, "y": 120.0 },
                "end": { "x": 140.0, "y": 120.0 },
                "order": 1,
                "variant": "single",
            })],
            Some(&include),
            false,
        );

        assert!(execution.ok);
        assert_eq!(execution.report["ok"], true);
        assert_eq!(execution.report["commandCount"], 1);
        assert_eq!(execution.report["executedCount"], 1);
        assert_eq!(execution.report["commands"][0]["ok"], true);
        assert_eq!(execution.report["commands"][0]["executed"], true);
        assert_eq!(execution.report["commands"][0]["changed"], true);
        assert_eq!(
            execution.report["commands"][0]["document"]["hashAlgorithm"],
            DOCUMENT_HASH_ALGORITHM
        );
        assert_eq!(
            execution.report["commands"][0]["document"]["hashInput"],
            DOCUMENT_HASH_INPUT
        );
        assert_eq!(
            execution.report["commands"][0]["document"]["hashChanged"],
            true
        );
        assert_eq!(execution.report["document"]["hashChanged"], true);
        assert_eq!(
            execution.report["commands"][0]["changeSummary"]["createdCount"],
            3
        );
        assert!(
            execution.report["commands"][0]["changeSummary"]["touchedSelectors"]
                .as_array()
                .unwrap()
                .contains(&json!("node:n_1"))
        );
        assert!(execution.report["commands"][0]["after"]["molecules"].is_array());
        assert!(execution.report["final"]["molecules"].is_array());
    }

    #[test]
    fn execution_report_is_lightweight_by_default() {
        let mut engine = Engine::new();
        let execution = execute_command_values(
            &mut engine,
            vec![json!({
                "type": "add-bond",
                "begin": { "x": 100.0, "y": 120.0 },
                "end": { "x": 140.0, "y": 120.0 },
                "order": 1,
                "variant": "single",
            })],
            default_inspect_after().as_deref(),
            false,
        );

        assert!(execution.ok);
        assert!(execution.report["commands"][0]["after"].is_null());
        assert!(execution.report["final"].is_null());
        assert_eq!(execution.report["document"]["hashChanged"], true);
        assert_eq!(
            execution.report["commands"][0]["document"]["hashChanged"],
            true
        );
        assert_eq!(
            execution.report["commands"][0]["changeSummary"]["createdSelectors"]["bonds"][0],
            "bond:b_3"
        );
    }

    #[test]
    fn execution_report_records_failed_command_without_saving() {
        let mut engine = Engine::new();
        let include = snapshot_include();
        let execution = execute_command_values(
            &mut engine,
            vec![
                json!({
                    "type": "add-bond",
                    "begin": { "x": 100.0, "y": 120.0 },
                    "end": { "x": 140.0, "y": 120.0 },
                    "order": 1,
                    "variant": "single",
                }),
                json!({
                    "type": "add-bond",
                    "begin": { "x": 100.0, "y": 120.0 },
                    "end": { "x": 180.0, "y": 120.0 },
                    "order": 1,
                    "variant": "not-a-bond-style",
                }),
            ],
            Some(&include),
            false,
        );

        assert!(!execution.ok);
        assert_eq!(execution.report["ok"], false);
        assert_eq!(execution.report["commandCount"], 2);
        assert_eq!(execution.report["executedCount"], 1);
        assert_eq!(execution.report["failedCount"], 1);
        assert_eq!(execution.report["failedIndex"], 1);
        assert_eq!(execution.report["failedIndices"], json!([1]));
        assert_eq!(execution.report["commands"][0]["executed"], true);
        assert_eq!(execution.report["commands"][1]["executed"], false);
        assert_eq!(
            execution.report["commands"][1]["document"]["hashChanged"],
            false
        );
        assert_eq!(
            execution.report["commands"][1]["changeSummary"]["touchedSelectors"],
            json!([])
        );
        assert_eq!(
            execution.report["commands"][1]["error"]["stage"],
            "execute-command"
        );
        assert!(execution.report["final"]["molecules"].is_array());
    }

    #[test]
    fn execution_report_can_continue_after_command_failures() {
        let mut engine = Engine::new();
        let include = snapshot_include();
        let execution = execute_command_values(
            &mut engine,
            vec![
                json!({
                    "type": "add-bond",
                    "begin": { "x": 100.0, "y": 120.0 },
                    "end": { "x": 140.0, "y": 120.0 },
                    "order": 1,
                    "variant": "single",
                }),
                json!({
                    "type": "add-bond",
                    "begin": { "x": 100.0, "y": 120.0 },
                    "end": { "x": 180.0, "y": 120.0 },
                    "order": 1,
                    "variant": "not-a-bond-style",
                }),
                json!({
                    "type": "add-text",
                    "position": { "x": 120.0, "y": 80.0 },
                    "text": "still executes"
                }),
            ],
            Some(&include),
            true,
        );

        assert!(!execution.ok);
        assert_eq!(execution.report["ok"], false);
        assert_eq!(execution.report["commandCount"], 3);
        assert_eq!(execution.report["executedCount"], 2);
        assert_eq!(execution.report["failedCount"], 1);
        assert_eq!(execution.report["failedIndex"], 1);
        assert_eq!(execution.report["failedIndices"], json!([1]));
        assert_eq!(execution.report["commands"][2]["executed"], true);
        assert_eq!(
            execution.report["commands"][2]["commandType"],
            json!("add-text")
        );
        assert_eq!(
            execution.report["final"]["summary"]["counts"]["objectTypes"]["text"],
            1
        );
    }
}
