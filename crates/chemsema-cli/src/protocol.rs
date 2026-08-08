use crate::{cli_cache_dir, cli_import_cache_enabled, write_json_value};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_GUIDE_FILE: &str = "chemsema-agent-guide.md";
const DETAILED_CLI_GUIDE_FILE: &str = "chemsema-cli-guide.md";
pub(crate) const CLI_PROTOCOL_VERSION: &str = "chemsema-cli-protocol.v1";
pub(crate) const SELECTOR_PROTOCOL_VERSION: &str = "chemsema-selector.v1";
pub(crate) const SESSION_PROTOCOL_VERSION: &str = "chemsema-cli-session-jsonl.v1";
pub(crate) const CAPTURE_MANIFEST_VERSION: &str = "chemsema-cli-capture-manifest.v1";
pub(crate) const AGENT_BUNDLE_SCHEMA_VERSION: &str = "chemsema.agent.bundle.v1";
pub(crate) const DOCUMENT_DIFF_SCHEMA_VERSION: &str = "chemsema.document.diff.v1";
pub(crate) const COMMAND_TRANSACTION_SCHEMA_VERSION: &str = "chemsema.command-transaction.v1";
pub(crate) const ERROR_MODEL_VERSION: &str = "chemsema-cli-error.v1";
pub(crate) const ENTRYPOINTS_SCHEMA_VERSION: &str = "chemsema.entrypoints.v1";
pub(crate) const CCJZ_CONTAINER_VERSION: &str = "chemsema.container.v1";
pub(crate) const JOURNAL_VERSION: &str = "chemsema.journal.v1";

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
        name: "version",
        summary: "Return ChemSema CLI product and protocol versions as JSON.",
        usage: "chemsema-cli version [--pretty] [--out <path>]",
        example: "chemsema-cli version --pretty",
    },
    CommandSpec {
        name: "capabilities",
        summary: "Return the machine-readable CLI protocol, commands, formats, and examples.",
        usage: "chemsema-cli capabilities [--pretty] [--out <path>]",
        example: "chemsema-cli capabilities --pretty",
    },
    CommandSpec {
        name: "schema",
        summary: "Return machine-readable command, target, and capture schemas.",
        usage: "chemsema-cli schema [ccjs-v0.2|ccjz-container|journal|protocol|commands|targets|capture|context|bundle|detail|diff|guide|copy|json-output|command-script|command-transaction|all] [--pretty] [--out <path>]",
        example: "chemsema-cli schema ccjz-container --pretty",
    },
    CommandSpec {
        name: "doctor",
        summary: "Report CLI installation paths, environment, and runtime capabilities.",
        usage: "chemsema-cli doctor [--pretty] [--out <path>]",
        example: "chemsema-cli doctor --pretty",
    },
    CommandSpec {
        name: "about",
        summary: "Return product metadata, installed entrypoints, packaging notes, and agent guidance.",
        usage: "chemsema-cli about [--pretty] [--out <path>]",
        example: "chemsema-cli about --pretty",
    },
    CommandSpec {
        name: "examples",
        summary: "Return ready-to-run JSON command scripts and CLI workflows.",
        usage: "chemsema-cli examples [basic|capture-copy|all] [--pretty] [--out <path>]",
        example: "chemsema-cli examples basic --pretty",
    },
    CommandSpec {
        name: "guide",
        summary: "Locate the installed agent and detailed CLI guides and optionally include their Markdown content.",
        usage: "chemsema-cli guide [--kind agent|detailed|all] [--include-content] [--pretty] [--out <path>]",
        example: "chemsema-cli guide --pretty",
    },
    CommandSpec {
        name: "validate",
        summary: "Validate container integrity, CCJS structure, engine semantics, and optional canonical round trips.",
        usage: "chemsema-cli validate <input> [--level structural|chemical|roundtrip] [--out <report.json>] [--pretty]",
        example: "chemsema-cli validate document.ccjz --level roundtrip --pretty",
    },
    CommandSpec {
        name: "canonicalize",
        summary: "Rewrite a document through the authoritative engine into canonical CCJS 0.2 or CCJZ form.",
        usage: "chemsema-cli canonicalize <input> --out <document.ccjs|document.ccjz> [--format ccjs|ccjz] [--pretty]",
        example: "chemsema-cli canonicalize input.ccjs --out canonical.ccjz --pretty",
    },
    CommandSpec {
        name: "migrate",
        summary: "Migrate a supported editable input to canonical CCJS 0.2 without overwriting the source.",
        usage: "chemsema-cli migrate <input> --out <document.ccjs|document.ccjz> [--format ccjs|ccjz] [--pretty]",
        example: "chemsema-cli migrate legacy.ccjs --out migrated.ccjz --pretty",
    },
    CommandSpec {
        name: "conformance",
        summary: "Run the built-in CCJS, deterministic CCJZ, and recovery-journal conformance probes.",
        usage: "chemsema-cli conformance [--out <report.json>] [--pretty]",
        example: "chemsema-cli conformance --pretty",
    },
    CommandSpec {
        name: "label-query",
        summary: "Ask the ChemSema text engine how a node label is recognized and displayed for a connection geometry.",
        usage: "chemsema-cli label-query (--text <source-label>|--visible-text <visible-label>) [--connection-angle <deg> ...] [--connection-count <n>] [--display-mode connection-auto|right-auto|left-auto|preserve-right|preserve-left|preserve-center] [--no-default-chemical] [--pretty] [--out <path>]",
        example: "chemsema-cli label-query --visible-text F3C --connection-angle 0 --pretty",
    },
    CommandSpec {
        name: "insert-smiles",
        summary: "Parse SMILES with the shared Rust chemistry kernel and insert the generated 2D structure into a new or existing document.",
        usage: "chemsema-cli insert-smiles <smiles> [--input <document>] --out <document> [--x <pt>] [--y <pt>] [--pretty]",
        example: "chemsema-cli insert-smiles 'CC(=O)O' --out acetate.ccjs --pretty",
    },
    CommandSpec {
        name: "chemistry",
        summary: "Generate a chemical identifier for a complete molecule using the shared Rust chemistry kernel.",
        usage: "chemsema-cli chemistry <input> [--format chemical-graph-v2|smiles|inchi|inchi-key] [--node <id> ...] [--out <json>] [--pretty]",
        example: "chemsema-cli chemistry molecule.ccjs --format smiles --pretty",
    },
    CommandSpec {
        name: "inspect",
        summary: "Inspect a document and write JSON summary/object/molecule/resource data.",
        usage: "chemsema-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]",
        example: "chemsema-cli inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty",
    },
    CommandSpec {
        name: "targets",
        summary: "List stable capture targets, object ids, molecule indices, node ids, bond ids, and bounds.",
        usage: "chemsema-cli targets <input> [--out <path>] [--pretty]",
        example: "chemsema-cli targets input.cdxml --out targets.json --pretty",
    },
    CommandSpec {
        name: "capture",
        summary: "Render a deterministic cropped SVG or high-resolution PNG for an object, molecule, node, bond, all content, or explicit bounds.",
        usage: "chemsema-cli capture <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] [--selection-only] [--crop-bounds <minX,minY,maxX,maxY>] [--out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--expand <pt>] [--expand-rel <fraction>] [--expand-left <pt>] [--pretty]",
        example: "chemsema-cli capture input.cdxml --target molecule:0 --target object:obj_label --selection-only --crop-bounds 0,0,800,600 --out selection.png --scale 6",
    },
    CommandSpec {
        name: "context",
        summary: "Report nearby objects/components around a target, including bounds, ids, spatial relation, group/link metadata, and optional screenshot.",
        usage: "chemsema-cli context <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] [--radius <pt>] [--expand-left <pt>] [--expand-rel <fraction>] [--out <context.json>] [--capture-out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--pretty]",
        example: "chemsema-cli context input.cdxml --target molecule:1 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty",
    },
    CommandSpec {
        name: "bundle",
        summary: "Write an object-grounded agent bundle with target detail, context, capture, editable subset, identity map, and manifest.",
        usage: "chemsema-cli bundle <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] --out-dir <directory> [--context-radius <pt>] [--capture-format png|svg] [--capture-width <px>] [--capture-height <px>] [--capture-scale <n>] [--subset-format ccjs|ccjz|cdxml|cdx|sdf] [--pretty]",
        example: "chemsema-cli bundle input.cdxml --target object:obj_mol_001 --out-dir output/bundle --context-radius 40 --capture-format png --subset-format ccjs --pretty",
    },
    CommandSpec {
        name: "detail",
        summary: "Return one target's detail JSON after targets/context discovery.",
        usage: "chemsema-cli detail <input> --target <object:id|molecule:index|node:id|bond:id> [--summary-only] [--include-resource] [--out <detail.json>] [--pretty]",
        example: "chemsema-cli detail input.cdxml --target object:obj_round_bracket --out object-detail.json --pretty",
    },
    CommandSpec {
        name: "copy",
        summary: "Copy all content or a target object/molecule/node/bond as a ChemSema Office/OLE clipboard payload.",
        usage: "chemsema-cli copy <input> [--target <object:id|molecule:index|node:id|bond:id|all>] [--office-helper <chemsema-office.exe>] [--payload <payload.json>] [--no-copy] [--pretty]",
        example: "chemsema-cli copy input.cdxml --target object:obj_arrow_1 --pretty",
    },
    CommandSpec {
        name: "session",
        summary: "Start a long-lived JSONL agent session that keeps one document open for repeated targets, detail, context, capture, execute, and save operations.",
        usage: "chemsema-cli session [input]",
        example: "chemsema-cli session input.cdxml",
    },
    CommandSpec {
        name: "new",
        summary: "Create a new document, optionally by applying a JSON command script.",
        usage: "chemsema-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--continue-on-error] [--pretty] [--quiet]",
        example: "chemsema-cli new commands.json --out generated.cdxml --results results.json --pretty",
    },
    CommandSpec {
        name: "run",
        summary: "Load a document, execute a JSON command script, and optionally save the edited document.",
        usage: "chemsema-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--continue-on-error] [--pretty] [--quiet]",
        example: "chemsema-cli run input.cdxml commands.json --out edited.cdxml --results results.json --pretty",
    },
    CommandSpec {
        name: "convert",
        summary: "Convert a whole editable document, or an editable target subset, between ChemSema, CDXML/CDX, SDF, SVG, PNG, and Windows EMF export formats.",
        usage: "chemsema-cli convert <input> <output> [--target <selector> ...|--targets <selector;selector>] [--selection-only] [--format <format>] [--scale <n>|--width <px>|--height <px>]",
        example: "chemsema-cli convert input.cdxml molecule-1.cdxml --target molecule:1",
    },
    CommandSpec {
        name: "export",
        summary: "Alias of convert for export-oriented workflows, including target-only document export.",
        usage: "chemsema-cli export <input> <output> [--target <selector> ...|--targets <selector;selector>] [--selection-only] [--format <format>] [--scale <n>|--width <px>|--height <px>]",
        example: "chemsema-cli export input.cdxml selection.svg --targets object:obj_a;object:obj_b",
    },
    CommandSpec {
        name: "diff",
        summary: "Compare two editable documents by document ids and structure instead of JSON text.",
        usage: "chemsema-cli diff <before> <after> --out <diff.json> [--pretty]",
        example: "chemsema-cli diff before.ccjs after.ccjs --out diff.json --pretty",
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
    details: Option<Value>,
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
            details: None,
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
            details: None,
        }
    }

    pub(crate) fn validation(report: Value) -> Self {
        let issue_count = report
            .get("issues")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        Self {
            kind: "validation_failed".to_string(),
            message: format!("Document validation failed with {issue_count} issue(s)."),
            command: Some("validate".to_string()),
            argument: None,
            usage: command_spec("validate").map(|spec| spec.usage.to_string()),
            examples: command_spec("validate")
                .map(|spec| vec![spec.example.to_string()])
                .unwrap_or_default(),
            hint: Some(
                "Inspect error.details.issues and correct every error-severity issue.".to_string(),
            ),
            fix: Some(json!({
                "action": "fix_validation_issues",
                "issueCount": issue_count,
            })),
            suggestions: Vec::new(),
            details: Some(report),
        }
    }

    pub(crate) fn unknown_command(command: &str) -> Self {
        Self {
            kind: "unknown_command".to_string(),
            message: format!("Unknown command '{command}'."),
            command: None,
            argument: Some(command.to_string()),
            usage: Some("chemsema-cli <command> [args]".to_string()),
            examples: vec![
                "chemsema-cli capabilities".to_string(),
                "chemsema-cli targets input.cdxml --out targets.json".to_string(),
                "chemsema-cli capture input.cdxml --target molecule:0 --out mol.png --scale 6"
                    .to_string(),
            ],
            hint: Some(
                "Choose one of the suggested commands or run chemsema-cli capabilities."
                    .to_string(),
            ),
            fix: Some(json!({
                "action": "choose_command",
                "helpCommand": "chemsema-cli capabilities",
            })),
            suggestions: command_suggestions(command),
            details: None,
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
                "details": self.details,
            }
        })
    }
}

fn command_error_hint(kind: &str, command: &str) -> Option<String> {
    match kind {
        "missing_argument" => Some(format!(
            "Add the required argument or value, then rerun. Use `chemsema-cli help {command}` for the exact format."
        )),
        "unexpected_argument" => Some(format!(
            "Remove the unexpected argument or move it to the correct position. Use `chemsema-cli help {command}` for accepted arguments."
        )),
        "invalid_format" => Some(format!(
            "Use a supported format or add the required --format value. Use `chemsema-cli schema {command}` when available."
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
            "helpCommand": format!("chemsema-cli help {command}"),
        })),
        "unexpected_argument" => Some(json!({
            "action": "use_supported_arguments",
            "usage": spec.map(|spec| spec.usage),
            "example": spec.map(|spec| spec.example),
            "helpCommand": format!("chemsema-cli help {command}"),
        })),
        "invalid_format" => Some(json!({
            "action": "use_supported_format_or_pass_format",
            "usage": spec.map(|spec| spec.usage),
            "example": spec.map(|spec| spec.example),
            "helpCommand": format!("chemsema-cli help {command}"),
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
            "usage": "chemsema-cli targets <input> --out targets.json --pretty",
            "example": "chemsema-cli targets input.cdxml --out targets.json --pretty",
        })],
        "invalid_command_json" => vec![json!({
            "action": "fix_command_json",
            "expected": "A JSON object command or an array of command objects.",
            "schemaCommand": "chemsema-cli schema command-script --pretty",
        })],
        _ if message.contains("Unknown schema topic") => vec![json!({
            "action": "choose_schema_topic",
            "accepted": ["protocol", "commands", "targets", "bounds", "capture", "context", "bundle", "detail", "diff", "guide", "copy", "session", "label-query", "json-output", "command-script", "command-transaction", "all"],
            "example": "chemsema-cli schema capture --pretty",
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
    let ccjs_v02: Value =
        serde_json::from_str(include_str!("../../../schemas/ccjs-v0.2.schema.json"))
            .expect("bundled CCJS v0.2 schema must be valid JSON");
    let ccjz_container: Value = serde_json::from_str(include_str!(
        "../../../schemas/ccjz-container-v1.schema.json"
    ))
    .expect("bundled CCJZ container schema must be valid JSON");
    let journal: Value =
        serde_json::from_str(include_str!("../../../schemas/journal-v1.schema.json"))
            .expect("bundled journal schema must be valid JSON");
    json!({
        "protocol": {
            "cli": CLI_PROTOCOL_VERSION,
            "selector": SELECTOR_PROTOCOL_VERSION,
            "session": SESSION_PROTOCOL_VERSION,
            "captureManifest": CAPTURE_MANIFEST_VERSION,
            "agentBundle": AGENT_BUNDLE_SCHEMA_VERSION,
            "documentDiff": DOCUMENT_DIFF_SCHEMA_VERSION,
            "commandTransaction": COMMAND_TRANSACTION_SCHEMA_VERSION,
            "errorModel": ERROR_MODEL_VERSION,
            "entrypoints": ENTRYPOINTS_SCHEMA_VERSION,
            "ccjzContainer": CCJZ_CONTAINER_VERSION,
            "journal": JOURNAL_VERSION,
            "compatibility": "v1 fields are intended to remain backward compatible throughout the 1.0 beta line unless explicitly marked experimental."
        },
        "ccjsV02": ccjs_v02,
        "ccjzContainer": ccjz_container,
        "journal": journal,
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
                "bond:<bond-id>",
                "bounds:<minX>,<minY>,<maxX>,<maxY>",
                "selection:<selector;selector>"
            ],
            "multiSelect": "For capture/context, repeat --target, pass --targets <selector;selector>, use selection:<selector;selector>, or in JSONL session pass target/targets as an array. The crop box is the minimum bounds union, matching the GUI selection box.",
            "examples": ["object:obj_round_bracket", "molecule:0", "node:n_4", "bond:b_5", "selection:object:obj_a;object:obj_b"]
        },
        "bounds": {
            "description": "World-space crop bounds in points.",
            "format": "minX,minY,maxX,maxY",
            "example": "-20,-10,140,80"
        },
        "capture": {
            "formats": ["svg", "png"],
            "resolution": "PNG defaults to --scale 10. Use --scale, --width, or --height for sharper or bounded raster output.",
            "expansion": "Use --expand/--padding for absolute pt expansion, --expand-left/right/top/bottom for per-side absolute expansion, and --expand-rel or --expand-rel-x/y for relative expansion based on target size.",
            "defaultOutput": "If --out is omitted, capture writes a PNG into the OS temp chemsema-cli directory and returns output.defaulted=true plus the exact path and a default_output_path warning in the JSON manifest.",
            "stdout": "JSON manifest only; rendered image data is written to --out or the default temp capture path.",
            "verification": "Capture manifests include output.verified=true and output.bytes after the rendered file is verified on disk.",
            "render": "Capture manifests include render.mode, render.primitiveCount, and render.targets. These describe how many render primitives and nearby node/bond/object targets were used to produce the crop.",
            "multiSelect": "Multiple targets are cropped by their minimum union bounds. The rendered image includes everything visible inside that box plus requested expansion.",
            "selectionOnly": "Pass --selection-only to render only the requested target primitives instead of every visible object inside the crop region.",
            "cropBounds": "Pass --crop-bounds minX,minY,maxX,maxY to force an exact world-space output canvas while the target still controls which primitives are rendered. This is useful for aligned object masks and training layers.",
            "usage": command_spec("capture").map(|spec| spec.usage).unwrap_or("")
        },
        "context": {
            "description": "Returns objects, molecules, nodes, and bonds near a target or multi-target selection. Entries include selector ids, bounds, center/edge distance, direction, overlap flags, selectionBoxRelation, group ancestry, child ids, and link metadata.",
            "selectionBox": "context.selectionBox.contents lists objects/molecules/nodes/bonds inside the target box. Each item reports selectionBoxRelation=inside or partial and isTarget=true only for explicitly selected targets.",
            "screenshot": "Pass --capture-out <path.svg|path.png> to render the same context bounds. The capture object includes render.mode, render.primitiveCount, and render.targets when a screenshot is written.",
            "usage": command_spec("context").map(|spec| spec.usage).unwrap_or("")
        },
        "bundle": {
            "schema": AGENT_BUNDLE_SCHEMA_VERSION,
            "description": "Writes an object-grounded bundle directory for one target or multi-target selection.",
            "artifacts": ["manifest.json", "target.json", "context.json", "editable-subset.<format>", "capture.png|capture.svg", "identity-map.json", "provenance.json"],
            "scope": "editableScope lists the target-only editable subset. visualScope describes the capture/context region and may include visible non-target objects. Seeing an object in context does not grant edit scope.",
            "verification": "manifest artifactVerification entries include path, format, verified, bytes, and sha256. integrity reports allResourcesResolved, allStylesResolved, captureVerified, and editableSubsetValid.",
            "sessionOperation": "JSONL session supports op=bundle with target, outDir, contextRadius, captureFormat, scale/width/height, subsetFormat, and pretty.",
            "usage": command_spec("bundle").map(|spec| spec.usage).unwrap_or("")
        },
        "detail": {
            "description": "Returns a single object's, molecule's, node's, or bond's detail JSON. Use targets/context first to discover selectors, then detail to expand one selector.",
            "rawPolicy": "By default, detail includes raw JSON for the selected entity. Use --summary-only for ids/bounds/relationship metadata only. Use --include-resource to embed the referenced molecule/text/json resource when inspecting an object.",
            "aliases": ["details", "describe", "show"],
            "usage": command_spec("detail").map(|spec| spec.usage).unwrap_or("")
        },
        "session": {
            "description": "Starts a JSON Lines protocol over stdin/stdout. The process keeps one Engine and parsed ChemSema document in memory until close or exit.",
            "protocol": SESSION_PROTOCOL_VERSION,
            "operations": ["open", "targets", "detail", "context", "bundle", "capture", "execute", "save", "status", "close", "exit"],
            "ready": "The first stdout line is a ready event. Send one compact JSON request per line and read one compact JSON response per line.",
            "historyPolicy": "The session does not persist undo history. execute responses report before/after revision and per-command results; callers should maintain history with git, files, or their own log.",
            "usage": command_spec("session").map(|spec| spec.usage).unwrap_or("")
        },
        "diff": {
            "schema": DOCUMENT_DIFF_SCHEMA_VERSION,
            "description": "Compares two editable documents by IDs and structure rather than raw JSON text.",
            "sections": ["document", "page", "objects", "resources", "styles", "nodes", "bonds", "changes", "counts", "unexpectedChanges"],
            "changes": "Field changes include selector, path, before, and after. Created/updated/deleted selectors are sorted deterministically.",
            "futureUse": "The same internal diff is intended for dry-run transaction reports, allowed scope validation, and no-unexpected-changes postconditions.",
            "usage": command_spec("diff").map(|spec| spec.usage).unwrap_or("")
        },
        "guide": {
            "description": "Returns installed guide metadata. Use --kind agent for the quick agent guide, --kind detailed for the detailed English CLI guide, or --kind all for both.",
            "files": [AGENT_GUIDE_FILE, DETAILED_CLI_GUIDE_FILE],
            "content": "Use --include-content with --out <path> when the caller needs Markdown text inside JSON.",
            "usage": command_spec("guide").map(|spec| spec.usage).unwrap_or("")
        },
        "copy": {
            "targets": ["all", "object", "molecule", "node", "bond"],
            "clipboard": "Windows Office/OLE via chemsema-office.exe --copy-clipboard-payload.",
            "stdout": "JSON manifest only; large clipboard payloads are written to a payload file.",
            "defaultPayload": "If --payload is omitted, copy writes the payload JSON into the OS temp chemsema-cli directory and reports payload.defaulted=true, payload.verified=true, payload.bytes, and a default_payload_path warning.",
            "usage": command_spec("copy").map(|spec| spec.usage).unwrap_or("")
        },
        "labelQuery": {
            "description": "Readonly label-engine query. It simulates attaching text to a node with the requested connection angles, then reports sourceText, displayText, sourceRuns, labelRecognition, semantics.anchorAtom, semantics.implicitHydrogenCount, whether the default display differs from the source text, and commandFields that can be copied into set-node-label-runs.",
            "displayMode": "Use --display-mode right-auto for imported CDXML/CDX LabelAlignment=Right without LabelDisplay: ChemSema applies chemical group reversal and right/end anchoring. Use preserve-right/left/center for forced visible display; if forced display and auto reversal produce the same visible text and anchor, pixels cannot recover the hidden source choice.",
            "reverseUse": "Use visible text queries to decide whether a visible string can be emitted as a default chemical label, should use displayMode preserve-left/right/center to keep visible ordering, or must preserve visible ordering with defaultChemical=false. Pass --display-mode right-auto when OCR evidence says the label is right/end anchored. Reverse reports include observableEquivalence groups so OCR can treat hidden source/display-mode choices as equivalent only when rendered glyphs, style runs, anchor/retreat, and topology are otherwise indistinguishable. Generated implicit-hydrogen glyphs are not bond anchors; semantics.generatedHydrogensMayBeBondAnchors is only true for standalone H.",
            "usage": command_spec("label-query").map(|spec| spec.usage).unwrap_or("")
        },
        "commandScript": {
            "input": "A JSON object command or an array of command objects.",
            "transactionEnvelope": {
                "schema": COMMAND_TRANSACTION_SCHEMA_VERSION,
                "description": "Optional envelope accepted by new/run and JSONL session execute. It declares preconditions, editable scope, atomic/dryRun options, commands, and postconditions.",
                "preconditions": ["expectedDocumentHash", "expectedRevision", "requiredSelectors"],
                "scope": ["editableTargets", "includeDescendants", "includeReferencedResources", "allowCreate", "allowDelete", "forbidChangesOutsideScope"],
                "options": ["atomic", "dryRun", "continueOnError"],
                "postconditions": ["document-valid", "no-unexpected-changes", "selector-exists"],
                "report": "Transaction reports include preconditions, execution, structured document diff, scope allowedSelectors/unexpectedChanges, postconditions, and transaction.applied."
            },
            "stdin": "Use '-' for commands.json to read JSON from stdin.",
            "audit": "new/run results are lightweight by default. They include top-level and per-command document hash/revision transitions plus selector-form created/updated/deleted summaries.",
            "historyPolicy": "The CLI does not maintain an undo stack or per-step snapshot history. Agents should maintain history with git, temp files, or their own logs.",
            "snapshots": "Per-command after snapshots and the top-level final snapshot are opt-in with --inspect-after <include>. Use --inspect-after none or --no-inspect-after to force no snapshots.",
            "errorPointers": "Execution reports include command index, commandType, document transition, changeSummary, and engine error message.",
            "editCommands": {
                "selection-state": {
                    "description": "Use select-targets, select-all, and clear-selection to drive GUI-style selection commands in new/run scripts and JSONL session execute requests. These commands change the in-memory selection but do not change the document revision."
                },
                "select-targets": {
                    "description": "Sets the current engine selection from explicit nodes, bonds, objects, and labelNodes. Passing one object is single-select; passing multiple ids is multi-select."
                },
                "select-all": {
                    "description": "Selects visible text and graphic objects plus editable molecule nodes, bonds, label nodes, and molecule objects, matching the GUI Select All action."
                },
                "clear-selection": {
                    "description": "Clears the current in-memory selection."
                },
                "add-bond": {
                    "description": "Creates a ChemSema bond from begin to end with order and variant. Optional wideEnd begin/end sets the wide endpoint for wedge, hashed-wedge, and hollow-wedge variants without reversing the bond connection direction. Optional stroke preserves explicit bond color; optional doublePlacement left/right/center, or double.placement, freezes an explicit double-bond placement while preserving the default automatic behavior when omitted. Optional endpointAttachments stores begin/end internal label character attachments, matching imported CDX/CDXML BeginAttach/EndAttach semantics."
                },
                "move-targets": {
                    "description": "Moves explicit nodes, bonds, objects, or labelNodes by delta dx/dy without relying on current selection."
                },
                "rotate-targets": {
                    "description": "Rotates explicit nodes, bonds, objects, or labelNodes around center by degrees."
                },
                "scale-targets": {
                    "description": "Scales explicit nodes, bonds, objects, or labelNodes by scaleX/scaleY around optional pivot. Use unequal factors for scripted stretch."
                },
                "selection-layout": {
                    "description": "After select-targets or select-all, GUI selection commands such as apply-selection-arrange, scale-selection, center-selection-on-page, apply-selection-color, delete-selection, group-selection, ungroup-selection, link-selection, set-link-policy, unlink-selection, paste-analysis-caption, and apply-chemical-property operate on the current selection."
                },
                "apply-object-settings-to-selection": {
                    "description": "Applies bondLength, lineWidth, boldWidth, bondSpacing, marginWidth, or hashSpacing to explicit bond_ids/object_ids. Use this for bond width and other object setting changes."
                }
            },
            "planningCommands": {
                "plan-bond": {
                    "description": "Readonly engine query for the final add-bond landing geometry. Accepts begin, optional cursor or angle, optional bondLength, order, and variant. Returns output.command as an executable add-bond command plus globalSnapAngles and keypadSlots.",
                    "agentBoundary": "This is for GUI-like drawing agents that want ChemSema placement behavior instead of duplicating GUI geometry."
                },
                "plan-template": {
                    "description": "Readonly engine query for template vertices and edges. Accepts template, x, y, and optional anchor, bondId, cursor, angle, bondLength, and side. Returns vertices, edges, and insertCommand."
                },
                "insert-template": {
                    "description": "Template edit command. The legacy centered form uses template/x/y. The extended form also accepts anchor, bondId, cursor, angle, bondLength, and side so callers can reuse engine placement instead of calculating ring geometry externally."
                }
            }
        }
    })
}

fn capabilities_value() -> Value {
    json!({
        "ok": true,
        "name": "chemsema-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": CLI_PROTOCOL_VERSION,
        "protocols": protocol_versions_value(),
        "stdout": {
            "default": "json",
            "pretty": "--pretty only changes JSON whitespace: compact JSON becomes line-broken and indented. It does not change fields, values, output files, exit code, schema, ordering, or command behavior.",
            "largeOutputPolicy": "For payloads that may exceed console buffers, pass --out <path> and read that file. capture writes image data to --out or a default temp PNG path and returns a JSON manifest.",
            "writeVerification": "File-writing commands verify the written file before reporting success."
        },
        "documentation": documentation_metadata(),
        "nextSteps": [
            "chemsema-cli guide --pretty",
            "chemsema-cli guide --kind detailed --pretty",
            "chemsema-cli about --pretty",
            "chemsema-cli examples basic --pretty",
            "chemsema-cli schema command-script --pretty",
            "chemsema-cli schema context --pretty",
            "chemsema-cli schema detail --pretty"
        ],
        "commands": command_specs_json(),
        "formats": {
            "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
            "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg", "png", "emf"],
            "chemicalAnalysis": ["chemical-graph-v2", "smiles", "inchi", "inchi-key"],
            "captureOutput": ["svg", "png"],
            "clipboardOutput": ["windows-office-ole", "chemsema-payload-json"]
        },
        "schemas": protocol_schemas_json()
    })
}

pub(crate) fn about_value() -> Value {
    json!({
        "ok": true,
        "schema": ENTRYPOINTS_SCHEMA_VERSION,
        "protocols": protocol_versions_value(),
        "product": {
            "name": "ChemSema",
            "version": env!("CARGO_PKG_VERSION"),
            "identifier": "com.chemsema.desktop",
            "description": "ChemSema is a desktop, browser, and CLI chemical drawing toolkit with editable ChemSema JSON, CDXML/CDX, SDF, SVG, and Office/OLE clipboard workflows."
        },
        "entrypoints": {
            "gui": {
                "name": "ChemSema desktop",
                "executable": "chemsema-desktop.exe",
                "installedPathHint": "<install-dir>\\chemsema-desktop.exe",
                "fileAssociations": ["ccjz", "ccjs", "cdxml", "cdx", "sdf", "sd"]
            },
            "cli": {
                "name": "chemsema-cli",
                "executable": "chemsema-cli.exe",
                "installedPathHints": [
                    "<install-dir>\\chemsema-cli.exe",
                    "<install-dir>\\resources\\chemsema-cli.exe"
                ],
                "discovery": [
                    "chemsema-cli guide --pretty",
                    "chemsema-cli guide --kind detailed --pretty",
                    "chemsema-cli about --pretty",
                    "chemsema-cli doctor --pretty",
                    "chemsema-cli capabilities --pretty",
                    "chemsema-cli examples basic --pretty",
                    "chemsema-cli schema context --pretty",
                    "chemsema-cli schema detail --pretty"
                ]
            },
            "officeOleHelper": {
                "executable": "chemsema-office.exe",
                "installedPathHints": [
                    "<install-dir>\\chemsema-office.exe",
                    "<install-dir>\\resources\\chemsema-office.exe"
                ],
                "purpose": "Registers the editable ChemSema OLE server and accepts CLI clipboard payloads for Office paste."
            }
        },
        "packaging": {
            "selfDescriptionFile": "chemsema-entrypoints.json",
            "selfDescriptionInstalledPathHint": "<install-dir>\\resources\\chemsema-entrypoints.json",
            "agentGuideFile": AGENT_GUIDE_FILE,
            "agentGuideInstalledPathHint": "<install-dir>\\resources\\chemsema-agent-guide.md",
            "detailedGuideFile": DETAILED_CLI_GUIDE_FILE,
            "detailedGuideInstalledPathHint": "<install-dir>\\resources\\chemsema-cli-guide.md",
            "installer": "NSIS x64",
            "windowsAppPaths": ["chemsema-cli.exe"],
            "pathRegistration": "The installer adds the ChemSema CLI directory to PATH. Open a new terminal after installing, then run `chemsema-cli guide --pretty`.",
            "pathRegistrationFallback": "If machine PATH registration fails, the installer writes the current-user PATH. App Paths are also registered for ShellExecute-style launchers.",
            "consoleNote": "Console agents can call `chemsema-cli` from a new terminal after install, or read installedPathHints from this file and `chemsema-cli doctor`."
        },
        "documentation": documentation_metadata(),
        "formats": {
            "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
            "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg", "png", "emf"],
            "chemicalAnalysis": ["chemical-graph-v2", "smiles", "inchi", "inchi-key"],
            "captureOutput": ["svg", "png"],
            "clipboardOutput": ["windows-office-ole", "chemsema-payload-json"]
        },
        "agentWorkflow": [
            "Run `chemsema-cli guide --pretty` first when using ChemSema without source-code context.",
            "Run `chemsema-cli guide --kind detailed --pretty` to locate the detailed English CLI guide.",
            "Run `chemsema-cli doctor --pretty` to identify the executable directory and install state.",
            "Run `chemsema-cli examples basic --pretty` for a minimal command script that creates an editable document.",
            "Run `chemsema-cli targets <document> --out targets.json --pretty` before precise capture or copy.",
            "Run `chemsema-cli context <document> --target <selector> --out context.json --capture-out context.png --scale 5` to inspect nearby objects and relationships.",
            "Run `chemsema-cli detail <document> --target <selector> --out detail.json --pretty` to expand one id into exact object/molecule/node/bond JSON.",
            "Run `chemsema-cli capture <document> --target <selector> --out crop.png --scale 6` for deterministic high-resolution cropped inspection.",
            "Use command JSON with `select-targets`, `select-all`, and `clear-selection` before GUI-style selection edits; the same commands work in `chemsema-cli run` and JSONL session `execute`.",
            "Run `chemsema-cli copy <document> --target <selector>` to place an editable Office/OLE payload on the Windows clipboard."
        ]
    })
}

pub(crate) fn version_text() -> String {
    format!("chemsema-cli {}", env!("CARGO_PKG_VERSION"))
}

fn protocol_versions_value() -> Value {
    json!({
        "cli": CLI_PROTOCOL_VERSION,
        "selector": SELECTOR_PROTOCOL_VERSION,
        "session": SESSION_PROTOCOL_VERSION,
        "captureManifest": CAPTURE_MANIFEST_VERSION,
            "agentBundle": AGENT_BUNDLE_SCHEMA_VERSION,
            "documentDiff": DOCUMENT_DIFF_SCHEMA_VERSION,
            "commandTransaction": COMMAND_TRANSACTION_SCHEMA_VERSION,
        "errorModel": ERROR_MODEL_VERSION,
        "entrypoints": ENTRYPOINTS_SCHEMA_VERSION,
    })
}

pub(crate) fn version_value() -> Value {
    json!({
        "ok": true,
        "product": "ChemSema",
        "cli": "chemsema-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "buildIdentity": build_identity(),
        "protocol": CLI_PROTOCOL_VERSION,
        "protocols": protocol_versions_value(),
    })
}

pub(crate) const fn build_identity() -> Option<&'static str> {
    option_env!("CHEMSEMA_BUILD_IDENTITY")
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
            "chemsema-cli guide --pretty",
            "$script = '[{\"type\":\"add-bond\",\"begin\":{\"x\":100,\"y\":120},\"end\":{\"x\":145,\"y\":120},\"order\":1,\"variant\":\"single\"},{\"type\":\"add-text\",\"position\":{\"x\":120,\"y\":82},\"text\":\"agent example\"}]'",
            "$script | chemsema-cli new - --out example.ccjs --results example-results.json --pretty",
            "chemsema-cli inspect example.ccjs --include summary,objects,molecules --out example-inspect.json --pretty",
            "chemsema-cli targets example.ccjs --out example-targets.json --pretty",
            "chemsema-cli convert example.ccjs example.svg"
        ]
    });
    let capture_copy = json!({
        "name": "capture-copy",
        "summary": "Use target discovery to crop a high-resolution PNG, inspect surrounding context, and copy the same target to Office.",
        "requires": ["An existing editable input document such as example.ccjs"],
        "powershell": [
            "chemsema-cli targets example.ccjs --out example-targets.json --pretty",
            "chemsema-cli context example.ccjs --target molecule:0 --radius 80 --out molecule-0-context.json --capture-out molecule-0-context.png --scale 5 --pretty",
            "chemsema-cli detail example.ccjs --target molecule:0 --out molecule-0-detail.json --pretty",
            "chemsema-cli capture example.ccjs --target molecule:0 --out molecule-0.png --scale 6 --expand-rel 0.15 --pretty",
            "chemsema-cli copy example.ccjs --target molecule:0 --pretty"
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
        title: "ChemSema Agent Guide",
        summary: "Quick machine-oriented guide for ChemSema CLI discovery, precise capture, context lookup, detail lookup, editing, and Office clipboard workflows.",
    }
}

fn detailed_cli_guide_spec() -> GuideSpec {
    GuideSpec {
        key: "detailed",
        file: DETAILED_CLI_GUIDE_FILE,
        language: "en",
        title: "ChemSema CLI Command Guide",
        summary:
            "Detailed command and JSON workflow guide for callers that use chemsema-cli directly.",
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
            "chemsema-cli guide --pretty",
            "chemsema-cli guide --kind detailed --pretty",
            "chemsema-cli doctor --pretty",
            "chemsema-cli targets input.cdxml --out targets.json --pretty",
            "chemsema-cli context input.cdxml --target molecule:0 --out context.json --capture-out context.png --scale 5 --pretty",
            "chemsema-cli detail input.cdxml --target object:<id> --out detail.json --pretty",
            "chemsema-cli capture input.cdxml --target object:<id> --out crop.png --scale 6 --expand-rel 0.15 --pretty"
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
        "discoveryCommand": format!("chemsema-cli guide --kind {} --pretty", spec.key),
        "contentCommand": format!("chemsema-cli guide --kind {} --include-content --out {} --pretty", spec.key, json_file),
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
                .join("chemsema-desktop")
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

pub(crate) fn version_command(args: &[String]) -> Result<(), String> {
    let (output, pretty) = parse_common_json_output_args(args)?;
    write_json_value(version_value(), output.as_deref(), pretty)
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
            "Unknown schema topic '{topic}'. Expected ccjs-v0.2, ccjz-container, journal, protocol, commands, targets, bounds, capture, context, bundle, detail, diff, guide, copy, session, label-query, json-output, command-script, command-transaction, or all."
        ));
    };
    write_json_value(value, output.as_deref(), pretty)
}

pub(crate) fn schema_topic_key(topic: &str) -> Option<&'static str> {
    match topic {
        "ccjs-v0.2" | "ccjs" | "document" | "document-format" => Some("ccjsV02"),
        "ccjz-container" | "ccjz" | "container" => Some("ccjzContainer"),
        "journal" | "recovery-journal" => Some("journal"),
        "protocol" | "protocols" | "version" | "versions" => Some("protocol"),
        "target" | "targets" => Some("target"),
        "bounds" => Some("bounds"),
        "capture" => Some("capture"),
        "context" | "nearby" | "neighbors" => Some("context"),
        "bundle" | "agent-bundle" | "object-bundle" => Some("bundle"),
        "detail" | "details" | "describe" | "show" | "object-detail" => Some("detail"),
        "diff" | "document-diff" | "compare" => Some("diff"),
        "guide" | "agent-guide" | "docs" | "documentation" => Some("guide"),
        "copy" | "clipboard" => Some("copy"),
        "session" | "jsonl" | "daemon" => Some("session"),
        "label-query" | "labelQuery" | "label" | "text-label" => Some("labelQuery"),
        "json-output" | "jsonOutput" | "stdout" | "output" | "pretty" => Some("jsonOutput"),
        "examples" => Some("commandScript"),
        "command-script" | "commandScript" | "commands-json" => Some("commandScript"),
        "command-transaction" | "transaction" | "transaction-script" => Some("commandScript"),
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
            "cache": {
                "enabled": cli_import_cache_enabled(),
                "dir": cli_cache_dir().display().to_string(),
                "disableEnv": "CHEMSEMA_CLI_DISABLE_CACHE=1",
                "dirEnv": "CHEMSEMA_CLI_CACHE_DIR"
            },
            "pathContainsExeDir": path_contains_exe_dir,
            "commands": COMMAND_SPECS.iter().map(|spec| spec.name).collect::<Vec<_>>(),
            "documentation": documentation_metadata(),
            "formats": {
                "editableInput": ["ccjs", "ccjz", "cdxml", "cdx", "sdf"],
                "documentOutput": ["json", "ccjs", "ccjz", "cdxml", "cdx", "sdf", "svg", "png", "emf"],
                "chemicalAnalysis": ["chemical-graph-v2", "smiles", "inchi", "inchi-key"],
                "captureOutput": ["svg", "png"],
                "clipboardOutput": ["windows-office-ole", "chemsema-payload-json"]
            }
        }),
        output.as_deref(),
        pretty,
    )
}
