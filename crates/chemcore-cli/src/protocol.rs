use crate::write_json_value;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_GUIDE_FILE: &str = "chemcore-agent-guide.md";
const DETAILED_CLI_GUIDE_FILE: &str = "chemcore-cli-guide.md";

#[derive(Clone, Copy)]
struct GuideSpec {
    key: &'static str,
    file: &'static str,
    language: &'static str,
    title: &'static str,
    summary: &'static str,
}

#[derive(Clone, Copy)]
enum GuideKind {
    Agent,
    Detailed,
    All,
}

impl GuideKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "agent" | "quick" | "quickstart" => Ok(Self::Agent),
            "detailed" | "full" | "cli" | "en" | "english" => Ok(Self::Detailed),
            "all" => Ok(Self::All),
            _ => Err(format!(
                "Unknown guide kind '{value}'. Expected agent, detailed, or all."
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Detailed => "detailed",
            Self::All => "all",
        }
    }
}

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
        usage: "chemcore-cli schema [commands|targets|capture|context|detail|guide|copy|json-output|command-script|all] [--pretty] [--out <path>]",
        example: "chemcore-cli schema capture --pretty",
    },
    CommandSpec {
        name: "doctor",
        summary: "Report CLI installation paths, environment, and runtime capabilities.",
        usage: "chemcore-cli doctor [--pretty] [--out <path>]",
        example: "chemcore-cli doctor --pretty",
    },
    CommandSpec {
        name: "about",
        summary: "Return product metadata, installed entrypoints, packaging notes, and agent guidance.",
        usage: "chemcore-cli about [--pretty] [--out <path>]",
        example: "chemcore-cli about --pretty",
    },
    CommandSpec {
        name: "examples",
        summary: "Return ready-to-run JSON command scripts and CLI workflows.",
        usage: "chemcore-cli examples [basic|capture-copy|all] [--pretty] [--out <path>]",
        example: "chemcore-cli examples basic --pretty",
    },
    CommandSpec {
        name: "guide",
        summary: "Locate the installed agent and detailed CLI guides and optionally include their Markdown content.",
        usage: "chemcore-cli guide [--kind agent|detailed|all] [--include-content] [--pretty] [--out <path>]",
        example: "chemcore-cli guide --pretty",
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
        summary: "Render a deterministic cropped SVG or high-resolution PNG for an object, molecule, node, bond, all content, or explicit bounds.",
        usage: "chemcore-cli capture <input> --target <object:id|molecule:index|node:id|bond:id|all> [--out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--expand <pt>] [--expand-rel <fraction>] [--expand-left <pt>] [--pretty]",
        example: "chemcore-cli capture input.cdxml --target molecule:0 --out mol-0.png --scale 6 --expand-rel 0.15",
    },
    CommandSpec {
        name: "context",
        summary: "Report nearby objects/components around a target, including bounds, ids, spatial relation, group/link metadata, and optional screenshot.",
        usage: "chemcore-cli context <input> --target <selector> [--radius <pt>] [--expand-left <pt>] [--expand-rel <fraction>] [--out <context.json>] [--capture-out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--pretty]",
        example: "chemcore-cli context input.cdxml --target molecule:1 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty",
    },
    CommandSpec {
        name: "detail",
        summary: "Return one target's detail JSON after targets/context discovery.",
        usage: "chemcore-cli detail <input> --target <object:id|molecule:index|node:id|bond:id> [--summary-only] [--include-resource] [--out <detail.json>] [--pretty]",
        example: "chemcore-cli detail input.cdxml --target object:obj_round_bracket --out object-detail.json --pretty",
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
    hint: Option<String>,
    fix: Option<Value>,
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
            hint: Some("Read error.message, then rerun with corrected input.".to_string()),
            fix: None,
            suggestions: Vec::new(),
        }
    }

    pub(crate) fn for_command(command: &str, message: String) -> Self {
        let spec = command_spec(command);
        let kind = classify_cli_error(&message);
        let fix = command_error_fix(kind, command, &message, spec);
        let suggestions = command_error_suggestions(kind, command, &message, spec, fix.clone());
        let argument = command_error_argument(kind, &message);
        Self {
            kind: kind.to_string(),
            message,
            command: Some(command.to_string()),
            argument,
            usage: spec.map(|spec| spec.usage.to_string()),
            examples: spec
                .map(|spec| vec![spec.example.to_string()])
                .unwrap_or_default(),
            hint: command_error_hint(kind, command),
            fix,
            suggestions,
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
                "chemcore-cli capture input.cdxml --target molecule:0 --out mol.png --scale 6"
                    .to_string(),
            ],
            hint: Some(
                "Choose one of the suggested commands or run chemcore-cli capabilities."
                    .to_string(),
            ),
            fix: Some(json!({
                "action": "choose_command",
                "helpCommand": "chemcore-cli capabilities",
            })),
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
                "hint": self.hint,
                "fix": self.fix,
                "suggestions": self.suggestions,
            }
        })
    }
}

fn command_error_hint(kind: &str, command: &str) -> Option<String> {
    match kind {
        "missing_argument" => Some(format!(
            "Add the required argument or value, then rerun. Use `chemcore-cli help {command}` for the exact format."
        )),
        "unexpected_argument" => Some(format!(
            "Remove the unexpected argument or move it to the correct position. Use `chemcore-cli help {command}` for accepted arguments."
        )),
        "invalid_format" => Some(format!(
            "Use a supported format or add the required --format value. Use `chemcore-cli schema {command}` when available."
        )),
        _ => None,
    }
}

fn command_error_argument(kind: &str, message: &str) -> Option<String> {
    match kind {
        "missing_argument" => missing_argument_name(message)
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        "unexpected_argument" => quoted_argument(message),
        _ => None,
    }
}

fn command_error_fix(
    kind: &str,
    command: &str,
    message: &str,
    spec: Option<CommandSpec>,
) -> Option<Value> {
    match kind {
        "missing_argument" => Some(json!({
            "action": "provide_required_argument",
            "missing": missing_argument_name(message),
            "expected": missing_argument_expected(message),
            "usage": spec.map(|spec| spec.usage),
            "example": spec.map(|spec| spec.example),
            "helpCommand": format!("chemcore-cli help {command}"),
        })),
        "unexpected_argument" => Some(json!({
            "action": "use_supported_arguments",
            "usage": spec.map(|spec| spec.usage),
            "example": spec.map(|spec| spec.example),
            "helpCommand": format!("chemcore-cli help {command}"),
        })),
        "invalid_format" => Some(json!({
            "action": "use_supported_format_or_pass_format",
            "usage": spec.map(|spec| spec.usage),
            "example": spec.map(|spec| spec.example),
            "helpCommand": format!("chemcore-cli help {command}"),
        })),
        _ => None,
    }
}

fn quoted_argument(message: &str) -> Option<String> {
    let start = message.find('\'')?;
    let rest = &message[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

fn command_error_suggestions(
    kind: &str,
    command: &str,
    message: &str,
    spec: Option<CommandSpec>,
    fix: Option<Value>,
) -> Vec<Value> {
    if let Some(fix) = fix {
        return vec![fix];
    }
    match kind {
        "target_not_found" => vec![json!({
            "action": "discover_targets",
            "usage": "chemcore-cli targets <input> --out targets.json --pretty",
            "example": "chemcore-cli targets input.cdxml --out targets.json --pretty",
        })],
        "invalid_command_json" => vec![json!({
            "action": "fix_command_json",
            "expected": "A JSON object command or an array of command objects.",
            "schemaCommand": "chemcore-cli schema command-script --pretty",
        })],
        _ if message.contains("Unknown schema topic") => vec![json!({
            "action": "choose_schema_topic",
            "accepted": ["commands", "targets", "bounds", "capture", "context", "detail", "guide", "copy", "json-output", "command-script", "all"],
            "example": "chemcore-cli schema capture --pretty",
        })],
        _ => spec
            .map(|spec| {
                vec![json!({
                    "action": "retry_command",
                    "command": command,
                    "usage": spec.usage,
                    "example": spec.example,
                })]
            })
            .unwrap_or_default(),
    }
}

fn missing_argument_name(message: &str) -> Value {
    let trimmed = message.trim();
    if let Some(first) = trimmed.split_whitespace().next() {
        if first.starts_with("--") || first.starts_with('-') {
            return json!(first);
        }
    }
    let Some((_, rest)) = trimmed.split_once(" requires ") else {
        return Value::Null;
    };
    let expected = rest.trim().trim_end_matches('.');
    if let Some(first) = expected.split_whitespace().next() {
        if first.starts_with("--") || first.starts_with('<') {
            return json!(first.trim_end_matches(','));
        }
    }
    json!(expected.trim_end_matches(';'))
}

fn missing_argument_expected(message: &str) -> Value {
    let trimmed = message.trim().trim_end_matches('.');
    if let Some((_, rest)) = trimmed.split_once(" requires ") {
        return json!(rest.trim());
    }
    if let Some((_, rest)) = trimmed.split_once("missing") {
        let rest = rest.trim();
        if !rest.is_empty() {
            return json!(rest);
        }
    }
    Value::Null
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
    let name = canonical_command_name(name);
    COMMAND_SPECS.iter().copied().find(|spec| spec.name == name)
}

fn canonical_command_name(name: &str) -> &str {
    match name {
        "details" | "describe" | "show" => "detail",
        _ => name,
    }
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
        "jsonOutput": {
            "default": "Commands that print JSON emit compact single-line JSON unless --pretty is present.",
            "pretty": "--pretty only changes JSON whitespace: compact JSON becomes line-broken and indented. It does not change fields, values, output files, exit code, schema, ordering, or command behavior.",
            "out": "When complete output matters, pass --out <path> and read that file instead of relying on a console buffer.",
            "errors": "Error JSON includes error.kind, message, hint, fix, usage, examples, and suggestions. Missing argument errors include fix.action=provide_required_argument and machine-readable missing/expected fields.",
            "writeVerification": "When the CLI writes a file, it verifies after the write that the target exists, is a regular file, and has the expected or minimum byte size. Verification failures are command errors."
        },
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
            "formats": ["svg", "png"],
            "resolution": "PNG defaults to --scale 4. Use --scale, --width, or --height for sharper or bounded raster output.",
            "expansion": "Use --expand/--padding for absolute pt expansion, --expand-left/right/top/bottom for per-side absolute expansion, and --expand-rel or --expand-rel-x/y for relative expansion based on target size.",
            "defaultOutput": "If --out is omitted, capture writes a PNG into the OS temp chemcore-cli directory and returns output.defaulted=true plus the exact path and a default_output_path warning in the JSON manifest.",
            "stdout": "JSON manifest only; rendered image data is written to --out or the default temp capture path.",
            "verification": "Capture manifests include output.verified=true and output.bytes after the rendered file is verified on disk.",
            "usage": command_spec("capture").map(|spec| spec.usage).unwrap_or("")
        },
        "context": {
            "description": "Returns objects, molecules, nodes, and bonds near a target. Entries include selector ids, bounds, center/edge distance, direction, overlap flags, group ancestry, child ids, and link metadata.",
            "screenshot": "Pass --capture-out <path.svg|path.png> to render the same context bounds.",
            "usage": command_spec("context").map(|spec| spec.usage).unwrap_or("")
        },
        "detail": {
            "description": "Returns a single object's, molecule's, node's, or bond's detail JSON. Use targets/context first to discover selectors, then detail to expand one selector.",
            "rawPolicy": "By default, detail includes raw JSON for the selected entity. Use --summary-only for ids/bounds/relationship metadata only. Use --include-resource to embed the referenced molecule/text/json resource when inspecting an object.",
            "aliases": ["details", "describe", "show"],
            "usage": command_spec("detail").map(|spec| spec.usage).unwrap_or("")
        },
        "guide": {
            "description": "Returns installed guide metadata. Use --kind agent for the quick agent guide, --kind detailed for the detailed English CLI guide, or --kind all for both.",
            "files": [AGENT_GUIDE_FILE, DETAILED_CLI_GUIDE_FILE],
            "content": "Use --include-content with --out <path> when the caller needs Markdown text inside JSON.",
            "usage": command_spec("guide").map(|spec| spec.usage).unwrap_or("")
        },
        "copy": {
            "targets": ["all", "object", "molecule", "node", "bond"],
            "clipboard": "Windows Office/OLE via chemcore-office.exe --copy-clipboard-payload.",
            "stdout": "JSON manifest only; large clipboard payloads are written to a payload file.",
            "defaultPayload": "If --payload is omitted, copy writes the payload JSON into the OS temp chemcore-cli directory and reports payload.defaulted=true, payload.verified=true, payload.bytes, and a default_payload_path warning.",
            "usage": command_spec("copy").map(|spec| spec.usage).unwrap_or("")
        },
        "commandScript": {
            "input": "A JSON object command or an array of command objects.",
            "stdin": "Use '-' for commands.json to read JSON from stdin.",
            "audit": "new/run results are lightweight by default. They include top-level and per-command document hash/revision transitions plus selector-form created/updated/deleted summaries.",
            "historyPolicy": "The CLI does not maintain an undo stack or per-step snapshot history. Agents should maintain history with git, temp files, or their own logs.",
            "snapshots": "Per-command after snapshots and the top-level final snapshot are opt-in with --inspect-after <include>. Use --inspect-after none or --no-inspect-after to force no snapshots.",
            "errorPointers": "Execution reports include command index, commandType, document transition, changeSummary, and engine error message."
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
            "pretty": "--pretty only changes JSON whitespace: compact JSON becomes line-broken and indented. It does not change fields, values, output files, exit code, schema, ordering, or command behavior.",
            "largeOutputPolicy": "For payloads that may exceed console buffers, pass --out <path> and read that file. capture writes image data to --out or a default temp PNG path and returns a JSON manifest.",
            "writeVerification": "File-writing commands verify the written file before reporting success."
        },
        "documentation": documentation_metadata(),
        "nextSteps": [
            "chemcore-cli guide --pretty",
            "chemcore-cli guide --kind detailed --pretty",
            "chemcore-cli about --pretty",
            "chemcore-cli examples basic --pretty",
            "chemcore-cli schema command-script --pretty",
            "chemcore-cli schema context --pretty",
            "chemcore-cli schema detail --pretty"
        ],
        "commands": command_specs_json(),
        "formats": {
            "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
            "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg"],
            "captureOutput": ["svg", "png"],
            "clipboardOutput": ["windows-office-ole", "chemcore-payload-json"]
        },
        "schemas": protocol_schemas_json()
    })
}

pub(crate) fn about_value() -> Value {
    json!({
        "ok": true,
        "schema": "chemcore.entrypoints.v1",
        "product": {
            "name": "Chemcore",
            "version": env!("CARGO_PKG_VERSION"),
            "identifier": "com.chemcore.desktop",
            "description": "Chemcore is a desktop, browser, and CLI chemical drawing toolkit with editable ChemCore JSON, CDXML/CDX, SDF, SVG, and Office/OLE clipboard workflows."
        },
        "entrypoints": {
            "gui": {
                "name": "Chemcore desktop",
                "executable": "chemcore-desktop.exe",
                "installedPathHint": "<install-dir>\\chemcore-desktop.exe",
                "fileAssociations": ["ccjz", "ccjs", "cdxml", "cdx", "sdf", "sd"]
            },
            "cli": {
                "name": "chemcore-cli",
                "executable": "chemcore-cli.exe",
                "installedPathHints": [
                    "<install-dir>\\chemcore-cli.exe",
                    "<install-dir>\\resources\\chemcore-cli.exe"
                ],
                "discovery": [
                    "chemcore-cli guide --pretty",
                    "chemcore-cli guide --kind detailed --pretty",
                    "chemcore-cli about --pretty",
                    "chemcore-cli doctor --pretty",
                    "chemcore-cli capabilities --pretty",
                    "chemcore-cli examples basic --pretty",
                    "chemcore-cli schema context --pretty",
                    "chemcore-cli schema detail --pretty"
                ]
            },
            "officeOleHelper": {
                "executable": "chemcore-office.exe",
                "installedPathHints": [
                    "<install-dir>\\chemcore-office.exe",
                    "<install-dir>\\resources\\chemcore-office.exe"
                ],
                "purpose": "Registers the editable Chemcore OLE server and accepts CLI clipboard payloads for Office paste."
            }
        },
        "packaging": {
            "selfDescriptionFile": "chemcore-entrypoints.json",
            "selfDescriptionInstalledPathHint": "<install-dir>\\resources\\chemcore-entrypoints.json",
            "agentGuideFile": AGENT_GUIDE_FILE,
            "agentGuideInstalledPathHint": "<install-dir>\\resources\\chemcore-agent-guide.md",
            "detailedGuideFile": DETAILED_CLI_GUIDE_FILE,
            "detailedGuideInstalledPathHint": "<install-dir>\\resources\\chemcore-cli-guide.md",
            "installer": "NSIS x64",
            "windowsAppPaths": ["chemcore-cli.exe"],
            "pathRegistration": "The installer adds the Chemcore CLI directory to PATH. Open a new terminal after installing, then run `chemcore-cli guide --pretty`.",
            "pathRegistrationFallback": "If machine PATH registration fails, the installer writes the current-user PATH. App Paths are also registered for ShellExecute-style launchers.",
            "consoleNote": "Console agents can call `chemcore-cli` from a new terminal after install, or read installedPathHints from this file and `chemcore-cli doctor`."
        },
        "documentation": documentation_metadata(),
        "formats": {
            "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
            "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg"],
            "captureOutput": ["svg", "png"],
            "clipboardOutput": ["windows-office-ole", "chemcore-payload-json"]
        },
        "agentWorkflow": [
            "Run `chemcore-cli guide --pretty` first when using Chemcore without source-code context.",
            "Run `chemcore-cli guide --kind detailed --pretty` to locate the detailed English CLI guide.",
            "Run `chemcore-cli doctor --pretty` to identify the executable directory and install state.",
            "Run `chemcore-cli examples basic --pretty` for a minimal command script that creates an editable document.",
            "Run `chemcore-cli targets <document> --out targets.json --pretty` before precise capture or copy.",
            "Run `chemcore-cli context <document> --target <selector> --out context.json --capture-out context.png --scale 5` to inspect nearby objects and relationships.",
            "Run `chemcore-cli detail <document> --target <selector> --out detail.json --pretty` to expand one id into exact object/molecule/node/bond JSON.",
            "Run `chemcore-cli capture <document> --target <selector> --out crop.png --scale 6` for deterministic high-resolution cropped inspection.",
            "Run `chemcore-cli copy <document> --target <selector>` to place an editable Office/OLE payload on the Windows clipboard."
        ]
    })
}

fn examples_value(topic: &str) -> Result<Value, String> {
    let basic_script = json!([
        {
            "type": "add-bond",
            "begin": { "x": 100.0, "y": 120.0 },
            "end": { "x": 145.0, "y": 120.0 },
            "order": 1,
            "variant": "single"
        },
        {
            "type": "add-text",
            "position": { "x": 120.0, "y": 82.0 },
            "text": "agent example"
        }
    ]);
    let basic = json!({
        "name": "basic",
        "summary": "Create a small editable document from stdin, inspect it, list targets, and export SVG.",
        "commandScript": basic_script,
        "powershell": [
            "chemcore-cli guide --pretty",
            "$script = '[{\"type\":\"add-bond\",\"begin\":{\"x\":100,\"y\":120},\"end\":{\"x\":145,\"y\":120},\"order\":1,\"variant\":\"single\"},{\"type\":\"add-text\",\"position\":{\"x\":120,\"y\":82},\"text\":\"agent example\"}]'",
            "$script | chemcore-cli new - --out example.ccjs --results example-results.json --pretty",
            "chemcore-cli inspect example.ccjs --include summary,objects,molecules --out example-inspect.json --pretty",
            "chemcore-cli targets example.ccjs --out example-targets.json --pretty",
            "chemcore-cli convert example.ccjs example.svg"
        ]
    });
    let capture_copy = json!({
        "name": "capture-copy",
        "summary": "Use target discovery to crop a high-resolution PNG, inspect surrounding context, and copy the same target to Office.",
        "requires": ["An existing editable input document such as example.ccjs"],
        "powershell": [
            "chemcore-cli targets example.ccjs --out example-targets.json --pretty",
            "chemcore-cli context example.ccjs --target molecule:0 --radius 80 --out molecule-0-context.json --capture-out molecule-0-context.png --scale 5 --pretty",
            "chemcore-cli detail example.ccjs --target molecule:0 --out molecule-0-detail.json --pretty",
            "chemcore-cli capture example.ccjs --target molecule:0 --out molecule-0.png --scale 6 --expand-rel 0.15 --pretty",
            "chemcore-cli copy example.ccjs --target molecule:0 --pretty"
        ],
        "notes": [
            "Use object:<id>, molecule:<index>, node:<id>, bond:<id>, or all as target selectors.",
            "Use --expand-left/right/top/bottom for directional absolute expansion, or --expand-rel for proportional context.",
            "Use --width or --height when a model needs a fixed pixel budget.",
            "Use --payload payload.json with copy when debugging Office/OLE clipboard data.",
            "capture writes deterministic SVG or PNG; stdout remains a JSON manifest."
        ]
    });
    match topic {
        "basic" => Ok(json!({ "ok": true, "examples": [basic] })),
        "capture-copy" | "copy" | "capture" => {
            Ok(json!({ "ok": true, "examples": [capture_copy] }))
        }
        "all" => Ok(json!({ "ok": true, "examples": [basic, capture_copy] })),
        other => Err(format!(
            "Unknown examples topic '{other}'. Expected basic, capture-copy, or all."
        )),
    }
}

fn agent_guide_spec() -> GuideSpec {
    GuideSpec {
        key: "agent",
        file: AGENT_GUIDE_FILE,
        language: "en",
        title: "Chemcore Agent Guide",
        summary: "Quick machine-oriented guide for Chemcore CLI discovery, precise capture, context lookup, detail lookup, editing, and Office clipboard workflows.",
    }
}

fn detailed_cli_guide_spec() -> GuideSpec {
    GuideSpec {
        key: "detailed",
        file: DETAILED_CLI_GUIDE_FILE,
        language: "en",
        title: "ChemCore CLI Command Guide",
        summary:
            "Detailed command and JSON workflow guide for callers that use chemcore-cli directly.",
    }
}

fn guide_specs_for_kind(kind: GuideKind) -> Vec<GuideSpec> {
    match kind {
        GuideKind::Agent => vec![agent_guide_spec()],
        GuideKind::Detailed => vec![detailed_cli_guide_spec()],
        GuideKind::All => vec![agent_guide_spec(), detailed_cli_guide_spec()],
    }
}

fn guide_value(include_content: bool, kind: GuideKind) -> Value {
    let agent_metadata = guide_metadata(agent_guide_spec());
    let detailed_metadata = guide_metadata(detailed_cli_guide_spec());
    let mut value = json!({
        "ok": true,
        "selectedKind": kind.as_str(),
        "guide": agent_metadata.clone(),
        "detailedGuide": detailed_metadata.clone(),
        "guides": [agent_metadata, detailed_metadata],
        "quickStart": [
            "chemcore-cli guide --pretty",
            "chemcore-cli guide --kind detailed --pretty",
            "chemcore-cli doctor --pretty",
            "chemcore-cli targets input.cdxml --out targets.json --pretty",
            "chemcore-cli context input.cdxml --target molecule:0 --out context.json --capture-out context.png --scale 5 --pretty",
            "chemcore-cli detail input.cdxml --target object:<id> --out detail.json --pretty",
            "chemcore-cli capture input.cdxml --target object:<id> --out crop.png --scale 6 --expand-rel 0.15 --pretty"
        ],
        "outputPolicy": {
            "default": "metadata only",
            "pretty": "--pretty only changes JSON whitespace: compact JSON becomes line-broken and indented. It does not change fields, values, output files, exit code, schema, ordering, or command behavior.",
            "largeOutput": "When complete output matters or --include-content is used, pass --out <path> and read that file instead of relying on a console buffer."
        }
    });
    if include_content {
        let contents = guide_specs_for_kind(kind)
            .into_iter()
            .map(guide_content_value)
            .collect::<Vec<_>>();
        if contents.len() == 1 {
            set_json_field(&mut value, "content", contents[0].clone());
        }
        set_json_field(&mut value, "contents", json!(contents));
    }
    value
}

fn agent_guide_metadata() -> Value {
    guide_metadata(agent_guide_spec())
}

fn detailed_guide_metadata() -> Value {
    guide_metadata(detailed_cli_guide_spec())
}

fn documentation_metadata() -> Value {
    json!({
        "agentGuide": agent_guide_metadata(),
        "detailedGuide": detailed_guide_metadata()
    })
}

fn guide_metadata(spec: GuideSpec) -> Value {
    let candidates = guide_path_candidates(spec.file);
    let found = candidates.iter().find(|path| path.is_file()).cloned();
    let json_file = spec.file.trim_end_matches(".md").to_string() + ".json";
    json!({
        "key": spec.key,
        "file": spec.file,
        "language": spec.language,
        "title": spec.title,
        "found": found.is_some(),
        "path": found.as_ref().map(|path| path.display().to_string()),
        "pathCandidates": candidates.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "installedPathHints": [
            format!("<install-dir>\\resources\\{}", spec.file),
            format!("<install-dir>\\{}", spec.file)
        ],
        "discoveryCommand": format!("chemcore-cli guide --kind {} --pretty", spec.key),
        "contentCommand": format!("chemcore-cli guide --kind {} --include-content --out {} --pretty", spec.key, json_file),
        "summary": spec.summary
    })
}

fn guide_content_value(spec: GuideSpec) -> Value {
    if let Some(path) = found_guide_path(spec.file) {
        match fs::read_to_string(&path) {
            Ok(content) => json!({
                "ok": true,
                "key": spec.key,
                "file": spec.file,
                "language": spec.language,
                "path": path.display().to_string(),
                "format": "markdown",
                "text": content,
            }),
            Err(error) => json!({
                "ok": false,
                "key": spec.key,
                "file": spec.file,
                "path": path.display().to_string(),
                "message": error.to_string(),
            }),
        }
    } else {
        json!({
            "ok": false,
            "key": spec.key,
            "file": spec.file,
            "message": format!("{} was not found in known install or development locations.", spec.file),
        })
    }
}

fn found_guide_path(file: &str) -> Option<PathBuf> {
    guide_path_candidates(file)
        .into_iter()
        .find(|path| path.is_file())
}

fn guide_path_candidates(file: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        push_guide_candidate(
            &mut candidates,
            cwd.join("apps")
                .join("chemcore-desktop")
                .join("src-tauri")
                .join("resources")
                .join(file),
        );
        push_guide_candidate(&mut candidates, cwd.join("docs").join(file));
        push_guide_candidate(
            &mut candidates,
            cwd.join("target").join("release").join(file),
        );
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            push_guide_candidate(&mut candidates, dir.join(file));
            push_guide_candidate(&mut candidates, dir.join("resources").join(file));
            if let Some(parent) = dir.parent() {
                push_guide_candidate(&mut candidates, parent.join("resources").join(file));
            }
        }
    }
    candidates
}

fn push_guide_candidate(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates
        .iter()
        .any(|candidate| same_path(candidate, &path))
    {
        candidates.push(path);
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn set_json_field(value: &mut Value, key: &str, field: Value) {
    if !value.is_object() {
        *value = json!({});
    }
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_string(), field);
    }
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

pub(crate) fn about_command(args: &[String]) -> Result<(), String> {
    let (output, pretty) = parse_common_json_output_args(args)?;
    write_json_value(about_value(), output.as_deref(), pretty)
}

pub(crate) fn examples_command(args: &[String]) -> Result<(), String> {
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
            value => return Err(format!("Unexpected examples argument '{value}'.")),
        }
        index += 1;
    }
    write_json_value(examples_value(&topic)?, output.as_deref(), pretty)
}

pub(crate) fn guide_command(args: &[String]) -> Result<(), String> {
    let mut output = None;
    let mut pretty = false;
    let mut include_content = false;
    let mut kind = GuideKind::Agent;
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
            "--include-content" | "--content" => include_content = true,
            "--kind" | "--guide" => {
                index += 1;
                kind = GuideKind::parse(
                    args.get(index)
                        .ok_or_else(|| "--kind requires agent, detailed, or all.".to_string())?,
                )?;
            }
            "--detailed" | "--full" => kind = GuideKind::Detailed,
            "--all" => kind = GuideKind::All,
            "--pretty" => pretty = true,
            value if !value.starts_with('-') => kind = GuideKind::parse(value)?,
            value => return Err(format!("Unexpected guide argument '{value}'.")),
        }
        index += 1;
    }
    write_json_value(
        guide_value(include_content, kind),
        output.as_deref(),
        pretty,
    )
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
            "Unknown schema topic '{topic}'. Expected commands, targets, bounds, capture, context, detail, guide, copy, json-output, command-script, or all."
        ));
    };
    write_json_value(value, output.as_deref(), pretty)
}

pub(crate) fn schema_topic_key(topic: &str) -> Option<&'static str> {
    match topic {
        "target" | "targets" => Some("target"),
        "bounds" => Some("bounds"),
        "capture" => Some("capture"),
        "context" | "nearby" | "neighbors" => Some("context"),
        "detail" | "details" | "describe" | "show" | "object-detail" => Some("detail"),
        "guide" | "agent-guide" | "docs" | "documentation" => Some("guide"),
        "copy" | "clipboard" => Some("copy"),
        "json-output" | "jsonOutput" | "stdout" | "output" | "pretty" => Some("jsonOutput"),
        "examples" => Some("commandScript"),
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
            "documentation": documentation_metadata(),
            "formats": {
                "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
                "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg"],
                "captureOutput": ["svg", "png"],
                "clipboardOutput": ["windows-office-ole", "chemcore-payload-json"]
            }
        }),
        output.as_deref(),
        pretty,
    )
}
