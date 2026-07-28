mod agent;
mod protocol;

use chemsema_desktop_service::DesktopDocumentService;
use chemsema_engine::{
    compact_label_text, decide_label_layout, layout_label_text, reverse_label_groups,
    ChemicalAnalysisFormat, CommandTargetSet, EditorCommand, Engine, LabelAnchorPolicy, LabelFlow,
    LabelLayoutDecision,
};
use protocol::{
    about_command, capabilities_command, doctor_command, examples_command, guide_command,
    schema_command, schema_or_capabilities_for_help, version_command, version_text, CliError,
    CliResult,
};
use serde_json::Map;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const DOCUMENT_HASH_ALGORITHM: &str = "sha256";
const DOCUMENT_HASH_INPUT: &str = "chemsema-document-json-v1";
const IMPORT_CACHE_VERSION: &str = "chemsema-cli-import-cache-v1";

#[derive(Debug, Clone, Copy, Default)]
struct RasterOutputOptions {
    scale: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
}

impl RasterOutputOptions {
    fn has_explicit_options(self) -> bool {
        self.scale.is_some() || self.width.is_some() || self.height.is_some()
    }
}

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

#[allow(clippy::result_large_err)]
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
        "label-query" | "label" => label_query_command(&args[1..])
            .map_err(|error| CliError::for_command("label-query", error)),
        "insert-smiles" => insert_smiles_command(&args[1..])
            .map_err(|error| CliError::for_command("insert-smiles", error)),
        "chemistry" | "chemical-analysis" => {
            chemistry_command(&args[1..]).map_err(|error| CliError::for_command("chemistry", error))
        }
        "targets" => agent::targets_command(&args[1..])
            .map_err(|error| CliError::for_command("targets", error)),
        "capture" => agent::capture_command(&args[1..])
            .map_err(|error| CliError::for_command("capture", error)),
        "context" => agent::context_command(&args[1..])
            .map_err(|error| CliError::for_command("context", error)),
        "bundle" => agent::bundle_command(&args[1..])
            .map_err(|error| CliError::for_command("bundle", error)),
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
        "diff" => {
            agent::diff_command(&args[1..]).map_err(|error| CliError::for_command("diff", error))
        }
        "run" => {
            run_command_script(&args[1..]).map_err(|error| CliError::for_command("run", error))
        }
        other => Err(CliError::unknown_command(other)),
    }
}

fn insert_smiles_command(args: &[String]) -> Result<(), String> {
    let mut smiles = None;
    let mut input = None;
    let mut output = None;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--smiles" | "-s" => {
                index += 1;
                smiles = Some(
                    args.get(index)
                        .ok_or_else(|| "--smiles requires a value.".to_string())?
                        .clone(),
                );
            }
            "--input" | "-i" => {
                index += 1;
                input = Some(
                    args.get(index)
                        .ok_or_else(|| "--input requires a path.".to_string())?
                        .clone(),
                );
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--x" => {
                index += 1;
                x = parse_f64_arg(
                    args.get(index)
                        .ok_or_else(|| "--x requires a value.".to_string())?,
                    "--x",
                )?;
            }
            "--y" => {
                index += 1;
                y = parse_f64_arg(
                    args.get(index)
                        .ok_or_else(|| "--y requires a value.".to_string())?,
                    "--y",
                )?;
            }
            "--pretty" => pretty = true,
            value if smiles.is_none() => smiles = Some(value.to_string()),
            value => return Err(format!("Unexpected insert-smiles argument '{value}'.")),
        }
        index += 1;
    }
    let smiles = smiles.ok_or_else(|| "insert-smiles requires a SMILES string.".to_string())?;
    let output = output.ok_or_else(|| "insert-smiles requires --out <document>.".to_string())?;
    let mut engine = if let Some(input) = input {
        load_engine_from_file(&input)?
    } else {
        Engine::new()
    };
    let result = engine.execute_command(EditorCommand::InsertSmiles { smiles, x, y })?;
    write_engine_output(&engine, &output, None)?;
    write_json_value(
        json!({
            "ok": true,
            "command": "insert-smiles",
            "output": output,
            "result": result,
        }),
        None,
        pretty,
    )
}

fn chemistry_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut format = ChemicalAnalysisFormat::Smiles;
    let mut node_ids = Vec::new();
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--format" | "-f" => {
                index += 1;
                format = match args
                    .get(index)
                    .ok_or_else(|| "--format requires a value.".to_string())?
                    .to_ascii_lowercase()
                    .replace('_', "-")
                    .as_str()
                {
                    "chemical-graph-v2" | "chemicalgraphv2" | "cgv2" => {
                        ChemicalAnalysisFormat::ChemicalGraphV2
                    }
                    "smiles" => ChemicalAnalysisFormat::Smiles,
                    "inchi" => ChemicalAnalysisFormat::Inchi,
                    "inchikey" | "inchi-key" => ChemicalAnalysisFormat::InchiKey,
                    value => return Err(format!("Unsupported chemistry format '{value}'.")),
                };
            }
            "--node" => {
                index += 1;
                node_ids.push(
                    args.get(index)
                        .ok_or_else(|| "--node requires an id.".to_string())?
                        .clone(),
                );
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
            value => return Err(format!("Unexpected chemistry argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "chemistry requires an input document.".to_string())?;
    let mut engine = load_engine_from_file(&input)?;
    let targets = if node_ids.is_empty() {
        let object_id = engine
            .state()
            .document
            .editable_fragment()
            .ok_or_else(|| "document has no editable molecule.".to_string())?
            .object
            .id
            .clone();
        CommandTargetSet {
            objects: vec![object_id],
            ..CommandTargetSet::default()
        }
    } else {
        CommandTargetSet {
            nodes: node_ids,
            ..CommandTargetSet::default()
        }
    };
    let result = engine.execute_command(EditorCommand::ChemicalAnalysis { format, targets })?;
    write_json_value(
        json!({
            "ok": true,
            "command": "chemistry",
            "input": input,
            "analysis": result.output,
        }),
        output.as_deref(),
        pretty,
    )
}

fn label_query_command(args: &[String]) -> Result<(), String> {
    let mut text = None;
    let mut visible_text = None;
    let mut reverse_mode = false;
    let mut connection_angles = Vec::<f64>::new();
    let mut connection_count = None;
    let mut default_chemical = true;
    let mut display_mode = LabelQueryDisplayMode::ConnectionAuto;
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--text" | "-t" => {
                index += 1;
                text = Some(
                    args.get(index)
                        .ok_or_else(|| "--text requires a value.".to_string())?
                        .clone(),
                );
            }
            "--visible-text" => {
                index += 1;
                visible_text = Some(
                    args.get(index)
                        .ok_or_else(|| "--visible-text requires a value.".to_string())?
                        .clone(),
                );
                reverse_mode = true;
            }
            "--mode" => {
                index += 1;
                match args
                    .get(index)
                    .ok_or_else(|| "--mode requires source or reverse.".to_string())?
                    .as_str()
                {
                    "source" | "forward" => reverse_mode = false,
                    "reverse" | "visible" => reverse_mode = true,
                    value => {
                        return Err(format!(
                            "Unsupported label-query --mode '{value}'. Use source or reverse."
                        ));
                    }
                }
            }
            "--connection-angle" | "--angle" => {
                index += 1;
                connection_angles.push(parse_f64_arg(
                    args.get(index)
                        .ok_or_else(|| "--connection-angle requires a value.".to_string())?,
                    "--connection-angle",
                )?);
            }
            "--connection-count" | "--connections" => {
                index += 1;
                connection_count = Some(parse_usize_arg(
                    args.get(index)
                        .ok_or_else(|| "--connection-count requires a value.".to_string())?,
                    "--connection-count",
                )?);
            }
            "--default-chemical" => default_chemical = true,
            "--no-default-chemical" => default_chemical = false,
            "--display-mode" | "--label-display" | "--label-alignment" | "--anchor-mode" => {
                index += 1;
                display_mode = parse_label_query_display_mode(
                    args.get(index)
                        .ok_or_else(|| format!("{} requires a value.", args[index - 1]))?,
                )?;
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
            value if text.is_none() => text = Some(value.to_string()),
            value => return Err(format!("Unexpected label-query argument '{value}'.")),
        }
        index += 1;
    }
    let text = if reverse_mode {
        visible_text.or(text).ok_or_else(|| {
            "label-query reverse mode requires --visible-text <label>.".to_string()
        })?
    } else {
        text.ok_or_else(|| "label-query requires --text <label>.".to_string())?
    };
    let connection_count = connection_count.unwrap_or_else(|| connection_angles.len().max(1));
    if connection_angles.len() > connection_count {
        return Err(
            "More --connection-angle values were provided than --connection-count.".to_string(),
        );
    }
    while connection_angles.len() < connection_count {
        let default_angle = match connection_angles.len() {
            0 => 0.0,
            1 => 180.0,
            index => 360.0 * index as f64 / connection_count.max(1) as f64,
        };
        connection_angles.push(default_angle);
    }

    let report = if reverse_mode {
        label_query_reverse_report(&text, &connection_angles, display_mode)?
    } else {
        label_query_report(&text, &connection_angles, default_chemical, display_mode)?
    };
    write_json_value(report, output.as_deref(), pretty)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelQueryDisplayMode {
    ConnectionAuto,
    RightAuto,
    LeftAuto,
    PreserveRight,
    PreserveLeft,
    PreserveCenter,
}

impl LabelQueryDisplayMode {
    fn as_str(self) -> &'static str {
        match self {
            LabelQueryDisplayMode::ConnectionAuto => "connection-auto",
            LabelQueryDisplayMode::RightAuto => "right-auto",
            LabelQueryDisplayMode::LeftAuto => "left-auto",
            LabelQueryDisplayMode::PreserveRight => "preserve-right",
            LabelQueryDisplayMode::PreserveLeft => "preserve-left",
            LabelQueryDisplayMode::PreserveCenter => "preserve-center",
        }
    }

    fn default_chemical(self) -> bool {
        !matches!(
            self,
            LabelQueryDisplayMode::PreserveRight
                | LabelQueryDisplayMode::PreserveLeft
                | LabelQueryDisplayMode::PreserveCenter
        )
    }

    fn alignment(self) -> &'static str {
        match self {
            LabelQueryDisplayMode::RightAuto | LabelQueryDisplayMode::PreserveRight => "right",
            LabelQueryDisplayMode::PreserveCenter => "center",
            _ => "left",
        }
    }

    fn anchor(self) -> &'static str {
        match self {
            LabelQueryDisplayMode::RightAuto | LabelQueryDisplayMode::PreserveRight => "end",
            LabelQueryDisplayMode::PreserveCenter => "middle",
            _ => "start",
        }
    }
}

fn parse_label_query_display_mode(value: &str) -> Result<LabelQueryDisplayMode, String> {
    match value.to_ascii_lowercase().replace('_', "-").as_str() {
        "auto" | "connection-auto" | "connection" | "engine" | "default" => {
            Ok(LabelQueryDisplayMode::ConnectionAuto)
        }
        "right-auto" | "right-alignment" | "alignment-right" | "right" => {
            Ok(LabelQueryDisplayMode::RightAuto)
        }
        "left-auto" | "left-alignment" | "alignment-left" | "left" => {
            Ok(LabelQueryDisplayMode::LeftAuto)
        }
        "preserve-right" | "display-right" | "labeldisplay-right" => {
            Ok(LabelQueryDisplayMode::PreserveRight)
        }
        "preserve-left" | "display-left" | "labeldisplay-left" => {
            Ok(LabelQueryDisplayMode::PreserveLeft)
        }
        "preserve-center" | "display-center" | "labeldisplay-center" | "center" => {
            Ok(LabelQueryDisplayMode::PreserveCenter)
        }
        other => Err(format!(
            "Unsupported label-query display mode '{other}'. Use connection-auto, right-auto, left-auto, preserve-right, preserve-left, or preserve-center."
        )),
    }
}

fn label_query_report(
    text: &str,
    connection_angles: &[f64],
    default_chemical: bool,
    display_mode: LabelQueryDisplayMode,
) -> Result<Value, String> {
    let mut engine = Engine::new();
    let node_id = if connection_angles.is_empty() {
        None
    } else {
        Some(build_label_query_node(&mut engine, connection_angles)?)
    };
    if let Some(node_id) = node_id.as_deref() {
        execute_json_command(
            &mut engine,
            label_query_set_node_label_command(
                node_id,
                text,
                label_query_source_runs(text, default_chemical),
                default_chemical,
                display_mode,
            ),
        )?;
    }
    let document_text = document_json(&engine)?;
    let document: Value =
        serde_json::from_str(&document_text).map_err(|error| error.to_string())?;
    let node = node_id
        .as_deref()
        .and_then(|id| find_document_node(&document, id).cloned());
    let label = node.as_ref().and_then(|node| node.get("label")).cloned();
    let label_meta = label
        .as_ref()
        .and_then(|label| label.get("meta"))
        .cloned()
        .unwrap_or(Value::Null);
    let node_meta = node
        .as_ref()
        .and_then(|node| node.get("meta"))
        .cloned()
        .unwrap_or(Value::Null);
    let recognition = label_meta
        .get("labelRecognition")
        .cloned()
        .or_else(|| node_meta.get("labelRecognition").cloned());
    let display_text = label
        .as_ref()
        .and_then(|label| label.get("text"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let display_runs = label
        .as_ref()
        .and_then(|label| label.get("runs"))
        .cloned()
        .unwrap_or(Value::Null);
    let stored_source_runs = label_meta.get("sourceRuns").cloned().unwrap_or(Value::Null);
    let source_text = label
        .as_ref()
        .and_then(|label| label.get("sourceText"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| text.to_string());
    let node_element = node
        .as_ref()
        .and_then(|node| node.get("element"))
        .and_then(Value::as_str);
    let node_atomic_number = node
        .as_ref()
        .and_then(|node| node.get("atomicNumber"))
        .and_then(Value::as_u64);
    let source_anchor_element = label_query_oxidation_state_anchor_element(source_text.as_str());
    let semantic_anchor_element = source_anchor_element
        .map(|(element, _)| element)
        .or(node_element);
    let semantic_anchor_atomic_number = source_anchor_element
        .map(|(_, atomic_number)| u64::from(atomic_number))
        .or(node_atomic_number);
    let implicit_hydrogen_count = node
        .as_ref()
        .and_then(|node| node.get("numHydrogens"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let status = recognition
        .as_ref()
        .and_then(|recognition| recognition.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("accepted");
    let accepted = status != "invalid";
    let hydrogen_is_anchor =
        accepted && node_element == Some("H") && source_text == "H" && implicit_hydrogen_count == 0;

    let query_source_runs = label_query_source_runs(text, default_chemical);

    Ok(json!({
        "schema": "chemsema.labelQuery.v1",
        "input": {
            "text": text,
            "connectionCount": connection_angles.len(),
            "connectionAngles": connection_angles,
            "defaultChemical": default_chemical
        },
        "accepted": accepted,
        "status": status,
        "semantics": {
            "anchorAtom": if accepted { semantic_anchor_element } else { None },
            "anchorAtomicNumber": if accepted { semantic_anchor_atomic_number } else { None },
            "implicitHydrogenCount": if accepted { Some(implicit_hydrogen_count) } else { None },
            "generatedHydrogensMayBeBondAnchors": hydrogen_is_anchor,
            "hydrogenAnchorRule": "Generated implicit-hydrogen glyphs are display/edit text only; bond drawing anchors stay on the heavy atom. Standalone H is the only hydrogen label that may anchor a bond."
        },
        "node": node,
        "label": label,
        "sourceText": source_text,
        "displayText": display_text,
        "displayDiffersFromSource": display_text.as_deref().is_some_and(|display| display != source_text),
        "layoutPrediction": label_query_layout_predictions(text, connection_angles),
        "requestedDisplay": label_query_display_mode_prediction(text, connection_angles, display_mode),
        "recognition": recognition,
        "sourceRuns": query_source_runs.clone(),
        "storedSourceRuns": stored_source_runs,
        "displayRuns": display_runs,
        "commandFields": label_query_command_fields(
            text,
            default_chemical,
            display_mode,
            query_source_runs,
        ),
    }))
}

fn label_query_set_node_label_command(
    node_id: &str,
    text: &str,
    source_runs: Value,
    default_chemical: bool,
    display_mode: LabelQueryDisplayMode,
) -> Value {
    let mut command = label_query_command_fields(text, default_chemical, display_mode, source_runs);
    command["type"] = json!("set-node-label-runs");
    command["nodeId"] = json!(node_id);
    command
}

fn label_query_command_fields(
    text: &str,
    default_chemical: bool,
    display_mode: LabelQueryDisplayMode,
    source_runs: Value,
) -> Value {
    let mut fields = json!({
        "text": text,
        "runs": source_runs,
        "sourceText": text,
        "defaultChemical": default_chemical,
        "displayMode": display_mode.as_str()
    });
    if matches!(display_mode, LabelQueryDisplayMode::ConnectionAuto) {
        fields
            .as_object_mut()
            .expect("command fields are an object")
            .remove("displayMode");
    }
    fields
}

fn label_query_oxidation_state_anchor_element(text: &str) -> Option<(&'static str, u8)> {
    let compact = compact_label_text(text);
    let open = compact.find('(')?;
    if !compact.ends_with(')') || open == 0 {
        return None;
    }
    let symbol = &compact[..open];
    if !symbol
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return None;
    }
    element_symbol_info_for_label_query(symbol)
}

fn element_symbol_info_for_label_query(symbol: &str) -> Option<(&'static str, u8)> {
    const SYMBOLS: &[&str] = &[
        "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
        "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga",
        "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd",
        "Ag", "Cd", "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm",
        "Sm", "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os",
        "Ir", "Pt", "Au", "Hg", "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa",
        "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg",
        "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
    ];
    SYMBOLS
        .iter()
        .enumerate()
        .find(|(_, candidate)| **candidate == symbol)
        .map(|(atomic_number, candidate)| (*candidate, atomic_number as u8))
}

fn label_query_reverse_report(
    visible_text: &str,
    connection_angles: &[f64],
    display_mode: LabelQueryDisplayMode,
) -> Result<Value, String> {
    let visible_compact = compact_label_text(visible_text);
    let mut source_candidates = Vec::<LabelQuerySourceCandidate>::new();
    push_label_query_source_candidate(
        &mut source_candidates,
        visible_compact.clone(),
        true,
        "visible-text-as-chemical-source",
        None,
    );
    let reversed_source = reverse_label_groups(&visible_compact);
    if reversed_source != visible_compact {
        push_label_query_source_candidate(
            &mut source_candidates,
            reversed_source.clone(),
            true,
            "kernel-reverse-label-groups",
            None,
        );
        if matches!(display_mode, LabelQueryDisplayMode::ConnectionAuto) {
            push_label_query_source_candidate(
                &mut source_candidates,
                reversed_source,
                true,
                "right-auto-reverse-label-groups",
                Some(LabelQueryDisplayMode::RightAuto),
            );
        }
    }
    push_label_query_source_candidate(
        &mut source_candidates,
        visible_compact.clone(),
        false,
        "plain-visible-text-preserve-layout",
        None,
    );

    let mut candidates = Vec::<Value>::new();
    for source in source_candidates {
        let effective_display_mode = if let Some(display_mode_override) = source.display_mode {
            display_mode_override
        } else if source.default_chemical {
            display_mode
        } else if matches!(
            display_mode,
            LabelQueryDisplayMode::RightAuto | LabelQueryDisplayMode::PreserveRight
        ) {
            LabelQueryDisplayMode::PreserveRight
        } else if matches!(display_mode, LabelQueryDisplayMode::PreserveCenter) {
            LabelQueryDisplayMode::PreserveCenter
        } else {
            LabelQueryDisplayMode::PreserveLeft
        };
        let report = label_query_report(
            &source.text,
            connection_angles,
            source.default_chemical,
            effective_display_mode,
        )?;
        let requested_display = report.get("requestedDisplay").cloned().unwrap_or_else(|| {
            label_query_display_mode_prediction(
                &source.text,
                connection_angles,
                effective_display_mode,
            )
        });
        let display_text = requested_display
            .get("displayText")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                report
                    .get("displayText")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| source.text.clone())
            });
        let display_matches_visible = compact_label_text(&display_text) == visible_compact;
        let source_text = source.text.clone();
        let source_runs = label_query_source_runs(&source_text, source.default_chemical);
        candidates.push(json!({
            "sourceText": source_text,
            "defaultChemical": source.default_chemical,
            "reason": source.reason,
            "displayMode": effective_display_mode.as_str(),
            "accepted": report.get("accepted").cloned().unwrap_or(Value::Bool(false)),
            "status": report.get("status").cloned().unwrap_or(Value::Null),
            "displayText": display_text,
            "displayMatchesVisible": display_matches_visible,
            "displayDiffersFromSource": requested_display.get("displayDiffersFromSource").cloned().unwrap_or(Value::Null),
            "layout": requested_display,
            "semantics": report.get("semantics").cloned().unwrap_or(Value::Null),
            "recognition": report.get("recognition").cloned().unwrap_or(Value::Null),
            "sourceRuns": source_runs.clone(),
            "storedSourceRuns": report.get("storedSourceRuns").cloned().unwrap_or(Value::Null),
            "displayRuns": report.get("displayRuns").cloned().unwrap_or(Value::Null),
            "commandFields": label_query_command_fields(
                &source_text,
                source.default_chemical,
                effective_display_mode,
                source_runs,
            )
        }));
    }

    let recommended_index = recommended_label_query_reverse_candidate_index(&candidates);
    let recommendation = recommended_index.and_then(|index| candidates.get(index).cloned());
    let observable_equivalence = label_query_observable_equivalence(&candidates);
    Ok(json!({
        "schema": "chemsema.labelQueryReverse.v1",
        "input": {
            "visibleText": visible_text,
            "normalizedVisibleText": visible_compact,
            "connectionCount": connection_angles.len(),
            "connectionAngles": connection_angles,
            "displayMode": display_mode.as_str()
        },
        "recommendedIndex": recommended_index,
        "recommendation": recommendation,
        "candidates": candidates,
        "observableEquivalence": observable_equivalence,
        "contract": {
            "reverseRole": "The caller supplies visible text, connection geometry, and optional display mode; ChemSema validates source text, display reversal, generated-hydrogen semantics, and defaultChemical behavior.",
            "displayMode": "connection-auto uses the normal endpoint label flow; right-auto models imported CDXML/CDX LabelAlignment=Right without LabelDisplay and may reverse chemical groups while anchoring the original first group at the right end; preserve-right/left/center models forced visible display where source cannot be distinguished from pixels alone when the same visible text and anchor policy result.",
            "plainFallback": "When no chemical candidate both validates and renders back to the visible text, use the plain defaultChemical=false candidate to preserve the source drawing."
        }
    }))
}

fn label_query_layout_predictions(text: &str, connection_angles: &[f64]) -> Value {
    json!({
        "connectionAuto": label_query_display_mode_prediction(text, connection_angles, LabelQueryDisplayMode::ConnectionAuto),
        "rightAuto": label_query_display_mode_prediction(text, connection_angles, LabelQueryDisplayMode::RightAuto),
        "leftAuto": label_query_display_mode_prediction(text, connection_angles, LabelQueryDisplayMode::LeftAuto),
        "preserveRight": label_query_display_mode_prediction(text, connection_angles, LabelQueryDisplayMode::PreserveRight),
        "preserveLeft": label_query_display_mode_prediction(text, connection_angles, LabelQueryDisplayMode::PreserveLeft),
        "preserveCenter": label_query_display_mode_prediction(text, connection_angles, LabelQueryDisplayMode::PreserveCenter)
    })
}

fn label_query_display_mode_prediction(
    text: &str,
    connection_angles: &[f64],
    mode: LabelQueryDisplayMode,
) -> Value {
    let decision = match mode {
        LabelQueryDisplayMode::ConnectionAuto => {
            decide_label_layout(connection_angles, false, false)
        }
        LabelQueryDisplayMode::RightAuto => LabelLayoutDecision {
            flow: LabelFlow::Reverse,
            anchor: LabelAnchorPolicy::OriginalFirstGroup,
        },
        LabelQueryDisplayMode::LeftAuto => LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::FirstGlyph,
        },
        LabelQueryDisplayMode::PreserveRight => LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::LastGlyph,
        },
        LabelQueryDisplayMode::PreserveLeft => LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::FirstGlyph,
        },
        LabelQueryDisplayMode::PreserveCenter => LabelLayoutDecision {
            flow: LabelFlow::Forward,
            anchor: LabelAnchorPolicy::WholeLabel,
        },
    };
    let mut layout = layout_label_text(text, &decision);
    if !mode.default_chemical() {
        layout.rendered_text = compact_label_text(text);
        layout.lines = if layout.rendered_text.is_empty() {
            Vec::new()
        } else {
            vec![layout.rendered_text.clone()]
        };
        layout.anchor_line = 0;
        layout.anchor_char = match mode {
            LabelQueryDisplayMode::PreserveRight => {
                layout.rendered_text.chars().count().saturating_sub(1)
            }
            LabelQueryDisplayMode::PreserveCenter => {
                layout.rendered_text.chars().count().saturating_sub(1) / 2
            }
            _ => 0,
        };
    }

    json!({
        "mode": mode.as_str(),
        "defaultChemical": mode.default_chemical(),
        "displayText": layout.rendered_text,
        "displayDiffersFromSource": layout.rendered_text != compact_label_text(text),
        "flow": layout.flow,
        "anchorPolicy": layout.anchor,
        "anchorLine": layout.anchor_line,
        "anchorChar": layout.anchor_char,
        "align": mode.alignment(),
        "anchor": mode.anchor(),
        "sourceText": text
    })
}

#[derive(Debug)]
struct LabelQuerySourceCandidate {
    text: String,
    default_chemical: bool,
    reason: &'static str,
    display_mode: Option<LabelQueryDisplayMode>,
}

fn push_label_query_source_candidate(
    candidates: &mut Vec<LabelQuerySourceCandidate>,
    text: String,
    default_chemical: bool,
    reason: &'static str,
    display_mode: Option<LabelQueryDisplayMode>,
) {
    if text.is_empty()
        || candidates.iter().any(|candidate| {
            candidate.text == text
                && candidate.default_chemical == default_chemical
                && candidate.display_mode == display_mode
        })
    {
        return;
    }
    candidates.push(LabelQuerySourceCandidate {
        text,
        default_chemical,
        reason,
        display_mode,
    });
}

fn recommended_label_query_reverse_candidate_index(candidates: &[Value]) -> Option<usize> {
    candidates
        .iter()
        .position(|candidate| {
            candidate.get("accepted").and_then(Value::as_bool) == Some(true)
                && candidate
                    .get("displayMatchesVisible")
                    .and_then(Value::as_bool)
                    == Some(true)
                && candidate.get("defaultChemical").and_then(Value::as_bool) == Some(true)
        })
        .or_else(|| {
            candidates.iter().position(|candidate| {
                candidate.get("accepted").and_then(Value::as_bool) == Some(true)
                    && candidate
                        .get("displayMatchesVisible")
                        .and_then(Value::as_bool)
                        == Some(true)
            })
        })
        .or_else(|| {
            candidates.iter().position(|candidate| {
                candidate.get("defaultChemical").and_then(Value::as_bool) == Some(false)
            })
        })
}

fn label_query_observable_equivalence(candidates: &[Value]) -> Value {
    let mut groups_by_key = BTreeMap::<String, (Value, Vec<usize>)>::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.get("accepted").and_then(Value::as_bool) != Some(true)
            || candidate
                .get("displayMatchesVisible")
                .and_then(Value::as_bool)
                != Some(true)
        {
            continue;
        }
        let layout = candidate.get("layout").unwrap_or(&Value::Null);
        let key = json!({
            "displayText": candidate.get("displayText").cloned().unwrap_or(Value::Null),
            "align": layout.get("align").cloned().unwrap_or(Value::Null),
            "anchor": layout.get("anchor").cloned().unwrap_or(Value::Null),
            "anchorChar": layout.get("anchorChar").cloned().unwrap_or(Value::Null),
            "anchorLine": layout.get("anchorLine").cloned().unwrap_or(Value::Null)
        });
        groups_by_key
            .entry(key.to_string())
            .or_insert_with(|| (key, Vec::new()))
            .1
            .push(index);
    }

    let groups = groups_by_key
        .into_iter()
        .filter_map(|(_, (observable, indices))| {
            if indices.len() < 2 {
                return None;
            }
            let source_choices = indices
                .iter()
                .filter_map(|index| {
                    let candidate = candidates.get(*index)?;
                    Some(json!({
                        "candidateIndex": index,
                        "sourceText": candidate.get("sourceText").cloned().unwrap_or(Value::Null),
                        "defaultChemical": candidate.get("defaultChemical").cloned().unwrap_or(Value::Null),
                        "displayMode": candidate.get("displayMode").cloned().unwrap_or(Value::Null),
                        "layoutFlow": candidate.pointer("/layout/flow").cloned().unwrap_or(Value::Null),
                        "layoutAnchorPolicy": candidate.pointer("/layout/anchorPolicy").cloned().unwrap_or(Value::Null),
                        "reason": candidate.get("reason").cloned().unwrap_or(Value::Null),
                    }))
                })
                .collect::<Vec<_>>();
            Some(json!({
                "observable": observable,
                "candidateIndices": indices,
                "sourceChoices": source_choices,
                "gatePolicy": "Treat these source/display-mode choices as equivalent only when the rendered glyphs, style runs, anchor/retreat, and topology are otherwise indistinguishable in the input pixels."
            }))
        })
        .collect::<Vec<_>>();

    json!({
        "indistinguishable": !groups.is_empty(),
        "groups": groups,
        "policy": "Reverse label queries expose hidden source-state ambiguity so OCR and gates do not guess author input when multiple accepted candidates produce the same visible text and anchor."
    })
}

fn build_label_query_node(
    engine: &mut Engine,
    connection_angles: &[f64],
) -> Result<String, String> {
    let origin = (100.0, 100.0);
    let length = 48.0;
    let first_angle = *connection_angles
        .first()
        .ok_or_else(|| "label query requires at least one connection angle.".to_string())?;
    let first_end = point_from_angle(origin, length, first_angle);
    let first = execute_json_command(
        engine,
        json!({
            "type": "add-bond",
            "begin": { "x": origin.0, "y": origin.1 },
            "end": { "x": first_end.0, "y": first_end.1 },
            "order": 1,
            "variant": "single"
        }),
    )?;
    let node_id = first
        .get("created")
        .and_then(|created| created.get("nodes"))
        .and_then(Value::as_array)
        .and_then(|nodes| nodes.first())
        .and_then(Value::as_str)
        .ok_or_else(|| "add-bond did not return a created label node.".to_string())?
        .to_string();
    for angle in connection_angles.iter().copied().skip(1) {
        let end = point_from_angle(origin, length, angle);
        execute_json_command(
            engine,
            json!({
                "type": "add-bond",
                "begin": { "nodeId": node_id, "x": origin.0, "y": origin.1 },
                "end": { "x": end.0, "y": end.1 },
                "order": 1,
                "variant": "single"
            }),
        )?;
    }
    Ok(node_id)
}

fn point_from_angle(origin: (f64, f64), length: f64, angle_deg: f64) -> (f64, f64) {
    let radians = angle_deg.to_radians();
    (
        origin.0 + length * radians.cos(),
        origin.1 + length * radians.sin(),
    )
}

fn label_query_source_runs(text: &str, default_chemical: bool) -> Value {
    json!([{
        "text": text,
        "script": if default_chemical { "chemical" } else { "normal" }
    }])
}

fn find_document_node<'a>(document: &'a Value, node_id: &str) -> Option<&'a Value> {
    document
        .get("resources")?
        .as_object()?
        .values()
        .filter_map(|resource| resource.get("data"))
        .filter_map(|data| data.get("nodes"))
        .filter_map(Value::as_array)
        .flat_map(|nodes| nodes.iter())
        .find(|node| node.get("id").and_then(Value::as_str) == Some(node_id))
}

fn parse_f64_arg(value: &str, name: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|error| format!("{name} must be a number: {error}"))
}

fn parse_usize_arg(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{name} must be an integer: {error}"))
}

fn new_command(args: &[String]) -> Result<(), String> {
    let mut script = None;
    let mut output = None;
    let mut save_format = None;
    let mut results = None;
    let mut document_json_output = None;
    let mut style_preset = None;
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
            "--style-preset" | "--template" => {
                index += 1;
                style_preset = Some(
                    args.get(index)
                        .ok_or_else(|| "--style-preset requires a value.".to_string())?
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
    if let Some(preset) = style_preset.as_deref() {
        engine.set_document_style_preset(preset);
    }
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
            "stylePreset": style_preset.as_deref(),
            "output": {
                "path": output.as_str(),
                "format": save_format
                    .as_deref()
                    .map(str::to_string)
                    .or_else(|| infer_format_from_path(&output)),
            },
        }),
    );
    if script_execution_is_dry_run(&execution) {
        set_dry_run_document_json_skip(&mut execution, document_json_output.as_deref());
    } else {
        write_optional_document_json(
            &mut execution,
            &engine,
            document_json_output.as_deref(),
            "documentJson",
        );
    }
    if execution.ok && script_execution_is_dry_run(&execution) {
        set_report_field(
            &mut execution.report,
            "save",
            json!({
                "ok": true,
                "skipped": true,
                "reason": "transaction dryRun=true",
                "warning": "Dry-run transactions do not write document output.",
            }),
        );
    } else if execution.ok {
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
    if results.is_some() || !quiet {
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
    let mut target = None;
    let mut selection_only = false;
    let mut raster = RasterOutputOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" | "--targets" => {
                index += 1;
                agent::add_target_arg(
                    &mut target,
                    agent::parse_target_selector(
                        args.get(index)
                            .ok_or_else(|| "--target requires a selector.".to_string())?,
                    )?,
                )?;
            }
            "--object" => {
                index += 1;
                agent::add_target_arg(
                    &mut target,
                    agent::TargetSelector::Object(
                        args.get(index)
                            .ok_or_else(|| "--object requires an object id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--molecule" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--molecule requires a non-negative integer.".to_string())?;
                agent::add_target_arg(
                    &mut target,
                    agent::TargetSelector::Molecule(
                        value.parse::<usize>().map_err(|_| {
                            "--molecule requires a non-negative integer.".to_string()
                        })?,
                    ),
                )?;
            }
            "--node" => {
                index += 1;
                agent::add_target_arg(
                    &mut target,
                    agent::TargetSelector::Node(
                        args.get(index)
                            .ok_or_else(|| "--node requires a node id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bond" => {
                index += 1;
                agent::add_target_arg(
                    &mut target,
                    agent::TargetSelector::Bond(
                        args.get(index)
                            .ok_or_else(|| "--bond requires a bond id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--all" => agent::add_target_arg(&mut target, agent::TargetSelector::All)?,
            "--selection-only" => selection_only = true,
            "--format" | "-f" => {
                index += 1;
                format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--scale" => {
                index += 1;
                raster.scale = Some(parse_positive_f64_arg(
                    "--scale",
                    args.get(index)
                        .ok_or_else(|| "--scale requires a positive number.".to_string())?,
                )?);
            }
            "--width" => {
                index += 1;
                raster.width = Some(parse_positive_u32_arg(
                    "--width",
                    args.get(index)
                        .ok_or_else(|| "--width requires a positive integer.".to_string())?,
                )?);
            }
            "--height" => {
                index += 1;
                raster.height = Some(parse_positive_u32_arg(
                    "--height",
                    args.get(index)
                        .ok_or_else(|| "--height requires a positive integer.".to_string())?,
                )?);
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
    if selection_only && target.is_none() {
        return Err(
            "--selection-only requires --target, --targets, or a target shortcut.".to_string(),
        );
    }
    let mut engine = load_engine_from_file(&input)?;
    if let Some(target) = target {
        engine = engine_for_export_target(&engine, &target)?;
    }
    write_engine_output_with_raster(&engine, &output, format.as_deref(), raster)
}

fn engine_for_export_target(
    engine: &Engine,
    target: &agent::TargetSelector,
) -> Result<Engine, String> {
    let document = serde_json::from_str(&document_json(engine)?)
        .map_err(|error| format!("Failed to parse engine document JSON: {error}"))?;
    let export_document = agent::export_document_for_target(&document, target)?;
    let export_json = serde_json::to_string(&export_document).map_err(|error| error.to_string())?;
    let mut export_engine = Engine::new();
    export_engine.load_document_json(&export_json)?;
    Ok(export_engine)
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
    if script_execution_is_dry_run(&execution) {
        set_dry_run_document_json_skip(&mut execution, document_json_output.as_deref());
    } else {
        write_optional_document_json(
            &mut execution,
            &engine,
            document_json_output.as_deref(),
            "documentJson",
        );
    }

    if execution.ok && script_execution_is_dry_run(&execution) {
        set_report_field(
            &mut execution.report,
            "save",
            json!({
                "ok": true,
                "skipped": true,
                "reason": "transaction dryRun=true",
                "warning": "Dry-run transactions do not write document output.",
            }),
        );
    } else if execution.ok {
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
    if results.is_some() || !quiet {
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
    let value = match read_command_script_value(script) {
        Ok(value) => value,
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
    if agent::is_transaction_script(&value) {
        let transaction = agent::execute_transaction_script(engine, &value);
        return ScriptExecution {
            ok: transaction.ok,
            report: transaction.report,
            error_message: transaction.error_message,
        };
    }
    let commands = match command_values_from_script_value(value) {
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

fn script_execution_is_dry_run(execution: &ScriptExecution) -> bool {
    execution
        .report
        .pointer("/transaction/dryRun")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn set_dry_run_document_json_skip(execution: &mut ScriptExecution, path: Option<&str>) {
    let Some(path) = path else {
        return;
    };
    set_report_field(
        &mut execution.report,
        "documentJson",
        json!({
            "ok": true,
            "path": path,
            "skipped": true,
            "reason": "transaction dryRun=true",
            "warning": "Dry-run transactions do not write document JSON output.",
        }),
    );
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
    !std::env::var("CHEMSEMA_CLI_DISABLE_CACHE")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub(crate) fn cli_cache_dir() -> PathBuf {
    if let Ok(path) = std::env::var("CHEMSEMA_CLI_CACHE_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(path) = std::env::var("LOCALAPPDATA") {
            if !path.trim().is_empty() {
                return PathBuf::from(path).join("ChemSema").join("cli-cache");
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(path) = std::env::var("XDG_CACHE_HOME") {
            if !path.trim().is_empty() {
                return PathBuf::from(path).join("chemsema").join("cli-cache");
            }
        }
        if let Ok(path) = std::env::var("HOME") {
            if !path.trim().is_empty() {
                return PathBuf::from(path)
                    .join(".cache")
                    .join("chemsema")
                    .join("cli-cache");
            }
        }
    }
    std::env::temp_dir().join("chemsema-cli").join("cache")
}

fn import_cache_path(source_text: &str, format: &str) -> PathBuf {
    cli_cache_dir()
        .join("imports")
        .join(IMPORT_CACHE_VERSION)
        .join(format!("{}.ccjs", import_cache_key(source_text, format)))
}

fn import_cache_key(source_text: &str, format: &str) -> String {
    import_cache_key_with_identity(
        source_text,
        format,
        protocol::build_identity().unwrap_or("development-build"),
        &current_exe_cache_stamp(),
    )
}

fn import_cache_key_with_identity(
    source_text: &str,
    format: &str,
    build_identity: &str,
    executable_stamp: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(IMPORT_CACHE_VERSION.as_bytes());
    digest.update(b"\0");
    digest.update(env!("CARGO_PKG_VERSION").as_bytes());
    digest.update(b"\0");
    digest.update(build_identity.as_bytes());
    digest.update(b"\0");
    digest.update(executable_stamp.as_bytes());
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
    if let Some(parent) = output_parent_path(path) {
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
    write_engine_output_with_raster(engine, path, format, RasterOutputOptions::default())
}

fn write_engine_output_with_raster(
    engine: &Engine,
    path: &str,
    format: Option<&str>,
    raster: RasterOutputOptions,
) -> Result<(), String> {
    let format = format
        .map(normalize_format)
        .transpose()?
        .or_else(|| infer_format_from_path(path))
        .ok_or_else(|| "Output format is ambiguous; pass --format.".to_string())?;

    if raster.has_explicit_options() && format != "png" {
        return Err(
            "--scale, --width, and --height are only supported for PNG output.".to_string(),
        );
    }

    if path == "-" {
        return write_engine_output_to_stdout(engine, &format);
    }

    if format == "png" {
        agent::write_document_png_output(engine, path, raster.scale, raster.width, raster.height)?;
        return Ok(());
    }

    if format == "emf" {
        return write_engine_emf_output(engine, path);
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
        "png" => Err("Writing PNG to stdout is not supported.".to_string()),
        "emf" => Err("Writing EMF to stdout is not supported.".to_string()),
        _ => Err(format!("Unsupported output format '{format}'.")),
    }
}

fn write_engine_emf_output(engine: &Engine, path: &str) -> Result<(), String> {
    ensure_output_parent(path)?;
    let payload = json!({
        "chemsemaFragmentJson": null,
        "chemsemaDocumentJson": document_json(engine)?,
        "renderListJson": serde_json::to_string(&engine.render_list())
            .map_err(|error| format!("Failed to serialize EMF render list: {error}"))?,
        "cdxml": null,
        "svg": null,
        "text": null,
    });
    chemsema_office::write_emf_payload_json(path, &payload.to_string())?;
    verify_file_written(Path::new(path), 1, "EMF output")?;
    Ok(())
}

fn read_command_script_value(path: &str) -> Result<Value, String> {
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
    serde_json::from_str(text).map_err(|error| format!("Invalid command JSON in {path}: {error}"))
}

#[cfg(test)]
fn read_command_values(path: &str) -> Result<Vec<Value>, String> {
    command_values_from_script_value(read_command_script_value(path)?)
}

fn command_values_from_script_value(value: Value) -> Result<Vec<Value>, String> {
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
    let Some(parent) = output_parent_path(path) else {
        return Ok(());
    };
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

fn output_parent_path(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    if parent.as_os_str().is_empty() || parent.components().next().is_none() {
        None
    } else {
        Some(parent)
    }
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
        "png" => "png",
        "emf" => "emf",
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

fn parse_positive_f64_arg(name: &str, value: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a positive number."))?;
    if !number.is_finite() || number <= 0.0 {
        return Err(format!("{name} requires a finite positive number."));
    }
    Ok(number)
}

fn parse_positive_u32_arg(name: &str, value: &str) -> Result<u32, String> {
    let number = value
        .parse::<u32>()
        .map_err(|_| format!("{name} requires a positive integer."))?;
    if number == 0 {
        return Err(format!("{name} requires a positive integer."));
    }
    Ok(number)
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
        assert_eq!(normalize_format("png").unwrap(), "png");
        assert_eq!(normalize_format("emf").unwrap(), "emf");
    }

    #[test]
    fn infers_format_from_output_path() {
        assert_eq!(infer_format_from_path("out.svg").as_deref(), Some("svg"));
        assert_eq!(infer_format_from_path("out.png").as_deref(), Some("png"));
        assert_eq!(infer_format_from_path("out.emf").as_deref(), Some("emf"));
        assert_eq!(infer_format_from_path("out.json").as_deref(), Some("json"));
        assert_eq!(infer_format_from_path("-"), None);
    }

    #[test]
    fn text_file_output_is_verified_after_write() {
        let path = std::env::temp_dir().join(format!(
            "chemsema-cli-write-verify-{}.json",
            std::process::id()
        ));
        let bytes = write_text_output(Some(path.to_str().unwrap()), "{\"ok\":true}\n").unwrap();
        assert_eq!(bytes, 12);
        assert_eq!(fs::metadata(&path).unwrap().len(), 12);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn bare_relative_output_paths_do_not_require_parent_directories() {
        assert!(output_parent_path(Path::new("output.cdxml")).is_none());
        assert!(output_parent_path(Path::new("results.json")).is_none());
        ensure_output_parent_path(Path::new("output.cdxml")).unwrap();
        ensure_output_parent("results.json").unwrap();
    }

    #[test]
    fn new_quiet_still_writes_explicit_results_file() {
        let root = std::env::temp_dir().join(format!(
            "chemsema-cli-new-quiet-results-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("commands.json");
        let out = root.join("out.ccjs");
        let results = root.join("results.json");
        fs::write(
            &script,
            r#"[{"type":"add-bond","begin":{"x":10,"y":10},"end":{"x":40,"y":10},"order":1,"variant":"single"}]"#,
        )
        .unwrap();

        new_command(&[
            script.display().to_string(),
            "--out".to_string(),
            out.display().to_string(),
            "--results".to_string(),
            results.display().to_string(),
            "--quiet".to_string(),
        ])
        .unwrap();

        let report: Value = serde_json::from_str(&fs::read_to_string(&results).unwrap()).unwrap();
        assert_eq!(report["ok"], true);
        assert_eq!(report["commandCount"], 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_quiet_still_writes_explicit_results_file() {
        let root = std::env::temp_dir().join(format!(
            "chemsema-cli-run-quiet-results-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let input = root.join("input.ccjs");
        let script = root.join("commands.json");
        let results = root.join("results.json");
        new_command(&[
            "--out".to_string(),
            input.display().to_string(),
            "--quiet".to_string(),
        ])
        .unwrap();
        fs::write(
            &script,
            r#"[{"type":"add-bond","begin":{"x":20,"y":20},"end":{"x":50,"y":20},"order":1,"variant":"single"}]"#,
        )
        .unwrap();

        run_command_script(&[
            input.display().to_string(),
            script.display().to_string(),
            "--results".to_string(),
            results.display().to_string(),
            "--quiet".to_string(),
        ])
        .unwrap();

        let report: Value = serde_json::from_str(&fs::read_to_string(&results).unwrap()).unwrap();
        assert_eq!(report["ok"], true);
        assert_eq!(report["commandCount"], 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn label_query_reports_default_cf3_reversal() {
        let report =
            label_query_report("CF3", &[0.0], true, LabelQueryDisplayMode::ConnectionAuto).unwrap();

        assert_eq!(report["accepted"], json!(true));
        assert_eq!(report["sourceText"], json!("CF3"));
        assert_eq!(report["displayText"], json!("F3C"));
        assert_eq!(report["displayDiffersFromSource"], json!(true));
        assert_eq!(report["recognition"]["canonicalLabel"], json!("CF3"));
        assert_eq!(report["semantics"]["anchorAtom"], json!("C"));
        assert_eq!(
            report["semantics"]["generatedHydrogensMayBeBondAnchors"],
            json!(false)
        );
    }

    #[test]
    fn label_query_keeps_hyphenated_label_tokens_whole() {
        let locant =
            label_query_report("2-Np", &[0.0], true, LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();
        let unknown_locant =
            label_query_report("3-Xyz", &[0.0], true, LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();
        let tert_butyl =
            label_query_report("t-Bu", &[0.0], true, LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();

        assert_eq!(locant["accepted"], json!(true));
        assert_eq!(locant["sourceText"], json!("2-Np"));
        assert_eq!(locant["displayText"], json!("2-Np"));
        assert_eq!(locant["displayDiffersFromSource"], json!(false));
        assert_eq!(locant["recognition"]["canonicalLabel"], json!("2-Np"));
        assert_eq!(
            locant["recognition"]["components"][0]["name"],
            json!("2-naphthyl")
        );
        assert_eq!(unknown_locant["accepted"], json!(false));
        assert_eq!(unknown_locant["sourceText"], json!("3-Xyz"));
        assert_eq!(unknown_locant["displayText"], json!("3-Xyz"));
        assert_eq!(unknown_locant["displayDiffersFromSource"], json!(false));
        assert_eq!(tert_butyl["sourceText"], json!("t-Bu"));
        assert_eq!(tert_butyl["displayText"], json!("t-Bu"));
        assert_eq!(tert_butyl["displayDiffersFromSource"], json!(false));
    }

    #[test]
    fn label_query_exposes_display_runs_for_copper_i_digit_ambiguity() {
        let cu_digit =
            label_query_report("Cu1", &[180.0], true, LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();
        let cu_iodide =
            label_query_report("CuI", &[180.0], true, LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();
        let cu_iodide_right =
            label_query_report("CuI", &[0.0], true, LabelQueryDisplayMode::ConnectionAuto).unwrap();

        assert_eq!(cu_digit["sourceText"], json!("Cu1"));
        assert_eq!(cu_digit["displayText"], json!("Cu1"));
        assert_eq!(cu_digit["displayRuns"][0]["text"], json!("Cu"));
        assert_eq!(cu_digit["displayRuns"][0]["script"], json!("normal"));
        assert_eq!(cu_digit["displayRuns"][1]["text"], json!("1"));
        assert_eq!(cu_digit["displayRuns"][1]["script"], json!("subscript"));

        assert_eq!(cu_iodide["sourceText"], json!("CuI"));
        assert_eq!(cu_iodide["displayText"], json!("CuI"));
        assert_eq!(cu_iodide["displayRuns"][0]["text"], json!("CuI"));
        assert_eq!(cu_iodide["displayRuns"][0]["script"], json!("normal"));

        assert_eq!(cu_iodide_right["sourceText"], json!("CuI"));
        assert_eq!(cu_iodide_right["displayText"], json!("ICu"));
        assert_eq!(cu_iodide_right["displayDiffersFromSource"], json!(true));
        assert_eq!(cu_iodide_right["displayRuns"][0]["text"], json!("ICu"));
        assert_eq!(cu_iodide_right["displayRuns"][0]["script"], json!("normal"));
    }

    #[test]
    fn label_query_can_disable_default_chemical_layout() {
        let report =
            label_query_report("CF3", &[0.0], false, LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();

        assert_eq!(report["accepted"], json!(true));
        assert_eq!(report["sourceText"], json!("CF3"));
        assert_eq!(report["displayText"], json!("CF3"));
        assert_eq!(report["displayDiffersFromSource"], json!(false));
        assert_eq!(report["sourceRuns"][0]["script"], json!("normal"));
        assert!(report["recognition"].is_null());
    }

    #[test]
    fn label_query_uses_kernel_abbreviation_recognition() {
        let bn = label_query_report("Bn", &[180.0], true, LabelQueryDisplayMode::ConnectionAuto)
            .unwrap();
        let et = label_query_report("Et", &[180.0], true, LabelQueryDisplayMode::ConnectionAuto)
            .unwrap();

        assert_eq!(bn["accepted"], json!(true));
        assert_eq!(bn["recognition"]["canonicalLabel"], json!("Bn"));
        assert_eq!(bn["recognition"]["groupKind"], json!("terminal-fragment"));
        assert_eq!(et["accepted"], json!(true));
        assert_eq!(et["recognition"]["canonicalLabel"], json!("Et"));
        assert_eq!(et["recognition"]["groupKind"], json!("terminal-fragment"));
    }

    #[test]
    fn label_query_reports_implicit_hydrogen_anchor_semantics() {
        let nh = label_query_report(
            "NH",
            &[0.0, 180.0],
            true,
            LabelQueryDisplayMode::ConnectionAuto,
        )
        .unwrap();
        let hn = label_query_report(
            "HN",
            &[0.0, 180.0],
            true,
            LabelQueryDisplayMode::ConnectionAuto,
        )
        .unwrap();
        let standalone_h =
            label_query_report("H", &[0.0], true, LabelQueryDisplayMode::ConnectionAuto).unwrap();

        assert_eq!(nh["accepted"], json!(true));
        assert_eq!(nh["semantics"]["anchorAtom"], json!("N"));
        assert_eq!(nh["semantics"]["implicitHydrogenCount"], json!(1));
        assert_eq!(
            nh["semantics"]["generatedHydrogensMayBeBondAnchors"],
            json!(false)
        );

        assert_eq!(hn["accepted"], json!(false));
        assert!(hn["semantics"]["anchorAtom"].is_null());

        assert_eq!(standalone_h["accepted"], json!(true));
        assert_eq!(standalone_h["semantics"]["anchorAtom"], json!("H"));
        assert_eq!(
            standalone_h["semantics"]["generatedHydrogensMayBeBondAnchors"],
            json!(true)
        );
    }

    #[test]
    fn label_query_reverse_maps_visible_h2n_to_nh2_source() {
        let report =
            label_query_reverse_report("H2N", &[0.0], LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();

        assert_eq!(report["schema"], json!("chemsema.labelQueryReverse.v1"));
        assert_eq!(report["recommendation"]["sourceText"], json!("NH2"));
        assert_eq!(report["recommendation"]["defaultChemical"], json!(true));
        assert_eq!(report["recommendation"]["displayText"], json!("H2N"));
        assert_eq!(
            report["recommendation"]["displayMatchesVisible"],
            json!(true)
        );
        assert_eq!(
            report["recommendation"]["semantics"]["anchorAtom"],
            json!("N")
        );
        assert_eq!(
            report["recommendation"]["semantics"]["generatedHydrogensMayBeBondAnchors"],
            json!(false)
        );
    }

    #[test]
    fn label_query_reverse_recommends_right_auto_for_visible_ho() {
        let report =
            label_query_reverse_report("HO", &[180.0], LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();

        assert_eq!(report["recommendation"]["sourceText"], json!("OH"));
        assert_eq!(report["recommendation"]["defaultChemical"], json!(true));
        assert_eq!(report["recommendation"]["displayText"], json!("HO"));
        assert_eq!(report["recommendation"]["displayMode"], json!("right-auto"));
        assert_eq!(
            report["recommendation"]["commandFields"]["displayMode"],
            json!("right-auto")
        );
        assert_eq!(
            report["recommendation"]["semantics"]["anchorAtom"],
            json!("O")
        );
    }

    #[test]
    fn label_query_reverse_maps_visible_f3c_to_cf3_source() {
        let report =
            label_query_reverse_report("F3C", &[0.0], LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();

        assert_eq!(report["recommendation"]["sourceText"], json!("CF3"));
        assert_eq!(report["recommendation"]["defaultChemical"], json!(true));
        assert_eq!(report["recommendation"]["displayText"], json!("F3C"));
        assert_eq!(
            report["recommendation"]["displayMatchesVisible"],
            json!(true)
        );
        assert_eq!(
            report["recommendation"]["recognition"]["canonicalLabel"],
            json!("CF3")
        );
    }

    #[test]
    fn label_query_reverse_prefers_plain_when_default_chemical_display_conflicts() {
        let report =
            label_query_reverse_report("CF3", &[0.0], LabelQueryDisplayMode::ConnectionAuto)
                .unwrap();

        assert_eq!(report["recommendation"]["sourceText"], json!("CF3"));
        assert_eq!(report["recommendation"]["defaultChemical"], json!(false));
        assert_eq!(report["recommendation"]["displayText"], json!("CF3"));
        assert_eq!(
            report["recommendation"]["displayMatchesVisible"],
            json!(true)
        );
        assert!(report["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| {
                candidate["sourceText"] == json!("CF3")
                    && candidate["defaultChemical"] == json!(true)
                    && candidate["displayText"] == json!("F3C")
                    && candidate["displayMatchesVisible"] == json!(false)
            }));
    }

    #[test]
    fn label_query_reverse_preserve_left_reports_copy_ready_command_fields() {
        let report =
            label_query_reverse_report("CN", &[-90.0], LabelQueryDisplayMode::PreserveLeft)
                .unwrap();

        let command_fields = &report["recommendation"]["commandFields"];
        assert_eq!(report["recommendation"]["sourceText"], json!("CN"));
        assert_eq!(report["recommendation"]["defaultChemical"], json!(true));
        assert_eq!(report["recommendation"]["displayText"], json!("CN"));
        assert_eq!(
            report["recommendation"]["displayMode"],
            json!("preserve-left")
        );
        assert_eq!(command_fields["text"], json!("CN"));
        assert_eq!(command_fields["sourceText"], json!("CN"));
        assert_eq!(command_fields["defaultChemical"], json!(true));
        assert_eq!(command_fields["displayMode"], json!("preserve-left"));
        assert_eq!(command_fields["runs"][0]["script"], json!("chemical"));
    }

    #[test]
    fn label_query_right_auto_models_cdxml_alignment_reversal() {
        let report = label_query_report(
            "Cu(II)",
            &[93.0, 17.0, -30.0],
            true,
            LabelQueryDisplayMode::RightAuto,
        )
        .unwrap();

        assert_eq!(report["requestedDisplay"]["displayText"], json!("(II)Cu"));
        assert_eq!(report["requestedDisplay"]["mode"], json!("right-auto"));
        assert_eq!(report["requestedDisplay"]["align"], json!("right"));
        assert_eq!(report["requestedDisplay"]["anchor"], json!("end"));
        assert_eq!(report["semantics"]["anchorAtom"], json!("Cu"));
        assert_eq!(report["semantics"]["anchorAtomicNumber"], json!(29));
    }

    #[test]
    fn label_query_reverse_right_auto_recovers_cuii_source_from_visible_iicu() {
        let report = label_query_reverse_report(
            "(II)Cu",
            &[93.0, 17.0, -30.0],
            LabelQueryDisplayMode::RightAuto,
        )
        .unwrap();

        assert_eq!(report["recommendation"]["sourceText"], json!("Cu(II)"));
        assert_eq!(report["recommendation"]["defaultChemical"], json!(true));
        assert_eq!(report["recommendation"]["displayText"], json!("(II)Cu"));
        assert_eq!(report["recommendation"]["displayMode"], json!("right-auto"));
        assert_eq!(report["recommendation"]["layout"]["anchor"], json!("end"));
        assert_eq!(
            report["recommendation"]["semantics"]["anchorAtom"],
            json!("Cu")
        );
        assert_eq!(
            report["recommendation"]["semantics"]["anchorAtomicNumber"],
            json!(29)
        );
        assert_eq!(
            report["observableEquivalence"]["indistinguishable"],
            json!(true)
        );
        assert!(report["observableEquivalence"]["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| {
                group["candidateIndices"].as_array().is_some_and(|indices| {
                    indices.contains(&json!(1)) && indices.contains(&json!(2))
                })
            }));
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
        assert_eq!(protocol::schema_topic_key("bundle"), Some("bundle"));
        assert_eq!(protocol::schema_topic_key("agent-bundle"), Some("bundle"));
        assert_eq!(protocol::schema_topic_key("detail"), Some("detail"));
        assert_eq!(protocol::schema_topic_key("object-detail"), Some("detail"));
        assert_eq!(protocol::schema_topic_key("diff"), Some("diff"));
        assert_eq!(protocol::schema_topic_key("document-diff"), Some("diff"));
        assert_eq!(protocol::schema_topic_key("guide"), Some("guide"));
        assert_eq!(protocol::schema_topic_key("agent-guide"), Some("guide"));
        assert_eq!(protocol::schema_topic_key("clipboard"), Some("copy"));
        assert_eq!(
            protocol::schema_topic_key("label-query"),
            Some("labelQuery")
        );
        assert_eq!(
            protocol::schema_topic_key("json-output"),
            Some("jsonOutput")
        );
        assert_eq!(protocol::schema_topic_key("pretty"), Some("jsonOutput"));
        assert_eq!(
            protocol::schema_topic_key("command-script"),
            Some("commandScript")
        );
        assert_eq!(
            protocol::schema_topic_key("command-transaction"),
            Some("commandScript")
        );
        assert_eq!(
            protocol::schema_topic_key("transaction"),
            Some("commandScript")
        );
    }

    #[test]
    fn version_metadata_is_machine_readable() {
        assert_eq!(
            protocol::version_text(),
            format!("chemsema-cli {}", env!("CARGO_PKG_VERSION"))
        );
        let value = protocol::version_value();
        assert_eq!(value["ok"], true);
        assert_eq!(value["cli"], "chemsema-cli");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        match option_env!("CHEMSEMA_BUILD_IDENTITY") {
            Some(identity) => assert_eq!(value["buildIdentity"], identity),
            None => assert!(value["buildIdentity"].is_null()),
        }
        assert_eq!(value["protocol"], protocol::CLI_PROTOCOL_VERSION);
        assert_eq!(
            value["protocols"]["session"],
            protocol::SESSION_PROTOCOL_VERSION
        );
    }

    #[test]
    fn protocol_docs_include_runtime_protocol_ids() {
        let cli_doc = include_str!("../../../docs/protocol/chemsema-cli-protocol-v1.md");
        let selector_doc = include_str!("../../../docs/protocol/selector-v1.md");
        let session_doc = include_str!("../../../docs/protocol/session-jsonl-v1.md");
        let capture_doc = include_str!("../../../docs/protocol/capture-manifest-v1.md");
        let bundle_doc = include_str!("../../../docs/protocol/agent-bundle-v1.md");
        let diff_doc = include_str!("../../../docs/protocol/document-diff-v1.md");
        let transaction_doc = include_str!("../../../docs/protocol/command-transaction-v1.md");
        let error_doc = include_str!("../../../docs/protocol/error-model-v1.md");
        let entrypoints_doc = include_str!("../../../docs/protocol/entrypoints-v1.md");

        assert!(cli_doc.contains(protocol::CLI_PROTOCOL_VERSION));
        assert!(selector_doc.contains(protocol::SELECTOR_PROTOCOL_VERSION));
        assert!(session_doc.contains(protocol::SESSION_PROTOCOL_VERSION));
        assert!(capture_doc.contains(protocol::CAPTURE_MANIFEST_VERSION));
        assert!(bundle_doc.contains(protocol::AGENT_BUNDLE_SCHEMA_VERSION));
        assert!(diff_doc.contains(protocol::DOCUMENT_DIFF_SCHEMA_VERSION));
        assert!(transaction_doc.contains(protocol::COMMAND_TRANSACTION_SCHEMA_VERSION));
        assert!(error_doc.contains(protocol::ERROR_MODEL_VERSION));
        assert!(entrypoints_doc.contains(protocol::ENTRYPOINTS_SCHEMA_VERSION));
    }

    #[test]
    fn error_model_doc_lists_runtime_error_kinds() {
        let error_doc = include_str!("../../../docs/protocol/error-model-v1.md");
        for kind in [
            "unknown_command",
            "missing_argument",
            "unexpected_argument",
            "invalid_format",
            "invalid_command_json",
            "target_not_found",
            "command_failed",
            "invalid_json",
            "missing_operation",
            "operation_failed",
        ] {
            assert!(
                error_doc.contains(kind),
                "error model doc should list runtime error kind {kind}"
            );
        }
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
            .contains("chemsema-cli capture"));
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
            .contains("chemsema-cli help capture"));
    }

    #[test]
    fn command_json_accepts_single_object_or_array() {
        let path = std::env::temp_dir().join(format!(
            "chemsema-cli-command-test-{}-single.json",
            std::process::id()
        ));
        fs::write(&path, r#"{ "type": "inspect-document" }"#).unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            1
        );
        let _ = fs::remove_file(path);

        let path = std::env::temp_dir().join(format!(
            "chemsema-cli-command-test-{}-array.json",
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
            "chemsema-cli-command-test-{}-bom.json",
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
    fn import_cache_key_changes_with_source_build_identity() {
        let source = "<CDXML><page /></CDXML>";
        let first = import_cache_key_with_identity(source, "cdxml", "source-a", "same-exe");
        let second = import_cache_key_with_identity(source, "cdxml", "source-b", "same-exe");
        assert_ne!(first, second);
    }

    #[test]
    fn import_cache_write_verifies_written_bytes() {
        let path = std::env::temp_dir().join(format!(
            "chemsema-cli-import-cache-test-{}.ccjs",
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

    #[test]
    fn execution_report_selection_commands_drive_arrange() {
        let mut engine = Engine::new();
        let execution = execute_command_values(
            &mut engine,
            vec![
                json!({
                    "type": "add-text",
                    "position": { "x": 10.0, "y": 10.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                }),
                json!({
                    "type": "add-text",
                    "position": { "x": 40.0, "y": 30.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                }),
                json!({
                    "type": "select-targets",
                    "targets": { "objects": ["obj_text_1", "obj_text_2"] }
                }),
                json!({
                    "type": "apply-selection-arrange",
                    "command": "align-left"
                }),
            ],
            Some(&["objects".to_string()]),
            false,
        );

        assert!(execution.ok);
        assert_eq!(execution.report["commandCount"], 4);
        assert_eq!(
            execution.report["commands"][2]["commandType"],
            "select-targets"
        );
        assert_eq!(execution.report["commands"][2]["changed"], false);
        assert_eq!(
            execution.report["commands"][2]["engineResult"]["output"]["counts"]["textObjects"],
            2
        );
        assert_eq!(
            execution.report["commands"][3]["commandType"],
            "apply-selection-arrange"
        );
        assert_eq!(execution.report["commands"][3]["changed"], true);

        let document: Value = serde_json::from_str(&document_json(&engine).expect("document json"))
            .expect("document value");
        let text_x = document["objects"]
            .as_array()
            .expect("objects")
            .iter()
            .filter(|object| object["type"].as_str() == Some("text"))
            .map(|object| object["transform"]["translate"][0].as_f64().expect("x"))
            .collect::<Vec<_>>();
        assert_eq!(text_x, vec![10.0, 10.0]);
    }

    #[test]
    fn transaction_dry_run_reports_diff_without_mutating_engine() {
        let mut engine = Engine::new();
        engine
            .execute_command_json(
                &json!({
                    "type": "add-bond",
                    "begin": { "x": 20.0, "y": 20.0 },
                    "end": { "x": 60.0, "y": 20.0 },
                    "order": 1,
                    "variant": "single"
                })
                .to_string(),
            )
            .unwrap();
        let before_hash = document_hash(&engine).unwrap();
        let before_revision = engine.revision();

        let execution = agent::execute_transaction_script(
            &mut engine,
            &json!({
                "schema": "chemsema.command-transaction.v1",
                "preconditions": {
                    "expectedDocumentHash": before_hash,
                    "expectedRevision": before_revision,
                    "requiredSelectors": ["object:obj_editor_molecule", "node:n_1"]
                },
                "scope": {
                    "editableTargets": ["object:obj_editor_molecule"],
                    "includeDescendants": true,
                    "includeReferencedResources": true,
                    "allowCreate": false,
                    "allowDelete": false,
                    "forbidChangesOutsideScope": true
                },
                "options": { "atomic": true, "dryRun": true },
                "commands": [
                    { "type": "replace-node-label", "node_id": "n_1", "label": "OMe" }
                ],
                "postconditions": [
                    { "type": "document-valid" },
                    { "type": "no-unexpected-changes" },
                    { "type": "selector-exists", "selector": "object:obj_editor_molecule" }
                ]
            }),
        );

        assert!(execution.ok);
        assert_eq!(execution.report["transaction"]["dryRun"], true);
        assert_eq!(execution.report["transaction"]["applied"], false);
        assert_eq!(document_hash(&engine).unwrap(), before_hash);
        assert_eq!(engine.revision(), before_revision);
        assert!(execution.report["diff"]["nodes"]["updated"]
            .as_array()
            .unwrap()
            .contains(&json!("node:n_1")));
        assert_eq!(execution.report["scope"]["unexpectedChanges"], json!([]));
    }

    #[test]
    fn transaction_rejects_out_of_scope_changes_and_rolls_back() {
        let mut engine = Engine::new();
        engine
            .execute_command_json(
                &json!({
                    "type": "add-bond",
                    "begin": { "x": 20.0, "y": 20.0 },
                    "end": { "x": 60.0, "y": 20.0 },
                    "order": 1,
                    "variant": "single"
                })
                .to_string(),
            )
            .unwrap();
        let before_hash = document_hash(&engine).unwrap();

        let execution = agent::execute_transaction_script(
            &mut engine,
            &json!({
                "schema": "chemsema.command-transaction.v1",
                "scope": {
                    "editableTargets": ["object:obj_editor_molecule"],
                    "includeReferencedResources": true,
                    "allowCreate": false,
                    "allowDelete": false,
                    "forbidChangesOutsideScope": true
                },
                "commands": [
                    {
                        "type": "add-text",
                        "position": { "x": 100.0, "y": 100.0 },
                        "text": "outside"
                    }
                ],
                "postconditions": [{ "type": "no-unexpected-changes" }]
            }),
        );

        assert!(!execution.ok);
        assert_eq!(execution.report["error"]["stage"], "scope");
        assert_eq!(document_hash(&engine).unwrap(), before_hash);
        assert!(execution.report["scope"]["unexpectedChanges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| change["section"] == "objects" && change["action"] == "created"));
    }
}
