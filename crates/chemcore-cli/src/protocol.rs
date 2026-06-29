use crate::write_json_value;
use serde_json::{json, Value};

#[derive(Clone, Copy)]
struct CommandSpec {
    name: &'static str,
    summary: &'static str,
    usage: &'static str,
    example: &'static str,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "capabilities",
        summary: "Return the machine-readable CLI protocol, commands, formats, and examples.",
        usage: "chemcore-cli capabilities [--pretty] [--out <path>]",
        example: "chemcore-cli capabilities --pretty",
    },
    CommandSpec {
        name: "schema",
        summary: "Return machine-readable command, target, and capture schemas.",
        usage: "chemcore-cli schema [commands|targets|capture|all] [--pretty] [--out <path>]",
        example: "chemcore-cli schema capture --pretty",
    },
    CommandSpec {
        name: "doctor",
        summary: "Report CLI installation paths, environment, and runtime capabilities.",
        usage: "chemcore-cli doctor [--pretty] [--out <path>]",
        example: "chemcore-cli doctor --pretty",
    },
    CommandSpec {
        name: "inspect",
        summary: "Inspect a document and write JSON summary/object/molecule/resource data.",
        usage: "chemcore-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]",
        example: "chemcore-cli inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty",
    },
    CommandSpec {
        name: "targets",
        summary: "List stable capture targets, object ids, molecule indices, node ids, bond ids, and bounds.",
        usage: "chemcore-cli targets <input> [--out <path>] [--pretty]",
        example: "chemcore-cli targets input.cdxml --out targets.json --pretty",
    },
    CommandSpec {
        name: "capture",
        summary: "Render a deterministic cropped SVG for an object, molecule, node, bond, all content, or explicit bounds.",
        usage: "chemcore-cli capture <input> --target <object:id|molecule:index|node:id|bond:id|all> --out <path.svg> [--padding <pt>] [--pretty]",
        example: "chemcore-cli capture input.cdxml --target molecule:0 --out mol-0.svg --padding 8",
    },
    CommandSpec {
        name: "copy",
        summary: "Copy all content or a target object/molecule/node/bond as a ChemCore Office/OLE clipboard payload.",
        usage: "chemcore-cli copy <input> [--target <object:id|molecule:index|node:id|bond:id|all>] [--office-helper <chemcore-office.exe>] [--payload <payload.json>] [--no-copy] [--pretty]",
        example: "chemcore-cli copy input.cdxml --target object:obj_arrow_1 --pretty",
    },
    CommandSpec {
        name: "new",
        summary: "Create a new document, optionally by applying a JSON command script.",
        usage: "chemcore-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--continue-on-error] [--pretty] [--quiet]",
        example: "chemcore-cli new commands.json --out generated.cdxml --results results.json --pretty",
    },
    CommandSpec {
        name: "run",
        summary: "Load a document, execute a JSON command script, and optionally save the edited document.",
        usage: "chemcore-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--continue-on-error] [--pretty] [--quiet]",
        example: "chemcore-cli run input.cdxml commands.json --out edited.cdxml --results results.json --pretty",
    },
    CommandSpec {
        name: "convert",
        summary: "Convert an editable document between ChemCore, CDXML/CDX, SDF, and SVG export formats.",
        usage: "chemcore-cli convert <input> <output> [--format <format>]",
        example: "chemcore-cli convert input.cdxml output.svg",
    },
    CommandSpec {
        name: "export",
        summary: "Alias of convert for export-oriented workflows.",
        usage: "chemcore-cli export <input> <output> [--format <format>]",
        example: "chemcore-cli export input.cdxml output.svg",
    },
];

#[derive(Debug)]
pub(crate) struct CliError {
    kind: String,
    message: String,
    command: Option<String>,
    argument: Option<String>,
    usage: Option<String>,
    examples: Vec<String>,
    suggestions: Vec<Value>,
}

pub(crate) type CliResult<T> = Result<T, CliError>;

impl CliError {
    pub(crate) fn message(message: String) -> Self {
        Self {
            kind: "command_failed".to_string(),
            message,
            command: None,
            argument: None,
            usage: None,
            examples: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub(crate) fn for_command(command: &str, message: String) -> Self {
        let spec = command_spec(command);
        Self {
            kind: classify_cli_error(&message).to_string(),
            message,
            command: Some(command.to_string()),
            argument: None,
            usage: spec.map(|spec| spec.usage.to_string()),
            examples: spec
                .map(|spec| vec![spec.example.to_string()])
                .unwrap_or_default(),
            suggestions: Vec::new(),
        }
    }

    pub(crate) fn unknown_command(command: &str) -> Self {
        Self {
            kind: "unknown_command".to_string(),
            message: format!("Unknown command '{command}'."),
            command: None,
            argument: Some(command.to_string()),
            usage: Some("chemcore-cli <command> [args]".to_string()),
            examples: vec![
                "chemcore-cli capabilities".to_string(),
                "chemcore-cli targets input.cdxml --out targets.json".to_string(),
                "chemcore-cli capture input.cdxml --target molecule:0 --out mol.svg".to_string(),
            ],
            suggestions: command_suggestions(command),
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "ok": false,
            "error": {
                "kind": self.kind,
                "message": self.message,
                "command": self.command,
                "argument": self.argument,
                "usage": self.usage,
                "examples": self.examples,
                "suggestions": self.suggestions,
            }
        })
    }
}

fn classify_cli_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("unexpected") {
        "unexpected_argument"
    } else if lower.contains("requires") || lower.contains("missing") {
        "missing_argument"
    } else if lower.contains("unsupported format") || lower.contains("ambiguous") {
        "invalid_format"
    } else if lower.contains("invalid command json") {
        "invalid_command_json"
    } else if lower.contains("not found") || lower.contains("no target") {
        "target_not_found"
    } else {
        "command_failed"
    }
}

fn command_spec(name: &str) -> Option<CommandSpec> {
    COMMAND_SPECS.iter().copied().find(|spec| spec.name == name)
}

fn command_suggestions(input: &str) -> Vec<Value> {
    let mut scored = COMMAND_SPECS
        .iter()
        .map(|spec| {
            let distance = edit_distance(input, spec.name);
            let max_len = input.len().max(spec.name.len()).max(1);
            let score = 1.0 - (distance as f64 / max_len as f64);
            (score, distance, spec)
        })
        .filter(|(score, distance, _)| *score >= 0.35 || *distance <= 3)
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    scored
        .into_iter()
        .take(4)
        .map(|(score, distance, spec)| {
            json!({
                "command": spec.name,
                "score": (score * 1000.0).round() / 1000.0,
                "distance": distance,
                "summary": spec.summary,
                "usage": spec.usage,
                "example": spec.example,
            })
        })
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.chars().collect::<Vec<_>>();
    let b = b.chars().collect::<Vec<_>>();
    let mut previous = (0..=b.len()).collect::<Vec<_>>();
    let mut current = vec![0; b.len() + 1];
    for (i, left) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, right) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(left != right);
            let insertion = current[j] + 1;
            let deletion = previous[j + 1] + 1;
            current[j + 1] = substitution.min(insertion).min(deletion);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

fn command_specs_json() -> Vec<Value> {
    COMMAND_SPECS
        .iter()
        .map(|spec| {
            json!({
                "name": spec.name,
                "summary": spec.summary,
                "usage": spec.usage,
                "example": spec.example,
            })
        })
        .collect()
}

fn protocol_schemas_json() -> Value {
    json!({
        "target": {
            "description": "Capture target selector.",
            "accepted": [
                "all",
                "object:<scene-object-id>",
                "molecule:<zero-based-molecule-index>",
                "node:<node-id>",
                "bond:<bond-id>"
            ],
            "examples": ["object:obj_round_bracket", "molecule:0", "node:n_4", "bond:b_5"]
        },
        "bounds": {
            "description": "World-space crop bounds in points.",
            "format": "minX,minY,maxX,maxY",
            "example": "-20,-10,140,80"
        },
        "capture": {
            "formats": ["svg"],
            "stdout": "JSON manifest only; rendered image data is written to --out.",
            "usage": command_spec("capture").map(|spec| spec.usage).unwrap_or("")
        },
        "copy": {
            "targets": ["all", "object", "molecule", "node", "bond"],
            "clipboard": "Windows Office/OLE via chemcore-office.exe --copy-clipboard-payload.",
            "stdout": "JSON manifest only; large clipboard payloads are written to a payload file.",
            "usage": command_spec("copy").map(|spec| spec.usage).unwrap_or("")
        },
        "commandScript": {
            "input": "A JSON object command or an array of command objects.",
            "stdin": "Use '-' for commands.json to read JSON from stdin.",
            "errorPointers": "Execution reports include command index, commandType, and engine error message."
        }
    })
}

fn capabilities_value() -> Value {
    json!({
        "ok": true,
        "name": "chemcore-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "chemcore-cli-agent",
        "stdout": {
            "default": "json",
            "largeOutputPolicy": "Prefer --out for large payloads. capture always writes image data to --out and returns a manifest."
        },
        "commands": command_specs_json(),
        "formats": {
            "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
            "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg"],
            "captureOutput": ["svg"],
            "clipboardOutput": ["windows-office-ole", "chemcore-payload-json"]
        },
        "schemas": protocol_schemas_json()
    })
}

fn parse_common_json_output_args(args: &[String]) -> Result<(Option<String>, bool), String> {
    let mut output = None;
    let mut pretty = false;
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
            "--pretty" => pretty = true,
            value => return Err(format!("Unexpected argument '{value}'.")),
        }
        index += 1;
    }
    Ok((output, pretty))
}

pub(crate) fn capabilities_command(args: &[String]) -> Result<(), String> {
    let (output, pretty) = parse_common_json_output_args(args)?;
    write_json_value(capabilities_value(), output.as_deref(), pretty)
}

pub(crate) fn schema_or_capabilities_for_help(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return write_json_value(capabilities_value(), None, false);
    }
    let command = args[0].as_str();
    if let Some(spec) = command_spec(command) {
        return write_json_value(
            json!({
                "ok": true,
                "command": spec.name,
                "summary": spec.summary,
                "usage": spec.usage,
                "example": spec.example,
                "schemas": protocol_schemas_json(),
            }),
            None,
            false,
        );
    }
    Err(format!("Unknown help topic '{command}'."))
}

pub(crate) fn schema_command(args: &[String]) -> Result<(), String> {
    let mut topic = "all".to_string();
    let mut output = None;
    let mut pretty = false;
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
            "--pretty" => pretty = true,
            value if !value.starts_with('-') && topic == "all" => topic = value.to_string(),
            value => return Err(format!("Unexpected schema argument '{value}'.")),
        }
        index += 1;
    }
    let schemas = protocol_schemas_json();
    let value = if topic == "all" {
        json!({ "ok": true, "schemas": schemas })
    } else if topic == "commands" {
        json!({ "ok": true, "commands": command_specs_json() })
    } else if let Some(schema_topic) = schema_topic_key(&topic) {
        let schema = schemas
            .get(schema_topic)
            .cloned()
            .ok_or_else(|| format!("Internal schema topic is missing: {schema_topic}."))?;
        json!({ "ok": true, "topic": topic, "schema": schema })
    } else {
        return Err(format!(
            "Unknown schema topic '{topic}'. Expected commands, targets, capture, copy, command-script, or all."
        ));
    };
    write_json_value(value, output.as_deref(), pretty)
}

pub(crate) fn schema_topic_key(topic: &str) -> Option<&'static str> {
    match topic {
        "target" | "targets" => Some("target"),
        "bounds" => Some("bounds"),
        "capture" => Some("capture"),
        "copy" | "clipboard" => Some("copy"),
        "command-script" | "commandScript" | "commands-json" => Some("commandScript"),
        _ => None,
    }
}

pub(crate) fn doctor_command(args: &[String]) -> Result<(), String> {
    let (output, pretty) = parse_common_json_output_args(args)?;
    let exe = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string());
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.display().to_string()));
    let path_env = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let path_contains_exe_dir = exe_dir
        .as_deref()
        .map(|dir| {
            std::env::split_paths(&path_env)
                .any(|entry| entry.to_string_lossy().eq_ignore_ascii_case(dir))
        })
        .unwrap_or(false);
    write_json_value(
        json!({
            "ok": true,
            "version": env!("CARGO_PKG_VERSION"),
            "exe": exe,
            "exeDir": exe_dir,
            "cwd": std::env::current_dir().ok().map(|path| path.display().to_string()),
            "tempDir": std::env::temp_dir().display().to_string(),
            "pathContainsExeDir": path_contains_exe_dir,
            "commands": COMMAND_SPECS.iter().map(|spec| spec.name).collect::<Vec<_>>(),
            "formats": {
                "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
                "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg"],
                "captureOutput": ["svg"],
                "clipboardOutput": ["windows-office-ole", "chemcore-payload-json"]
            }
        }),
        output.as_deref(),
        pretty,
    )
}
