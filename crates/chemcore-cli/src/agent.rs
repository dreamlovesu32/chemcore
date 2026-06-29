use crate::{
    document_json, ensure_output_parent_path, infer_format_from_path, load_engine_from_file,
    verify_file_written, verify_file_written_exact, write_engine_output, write_json_value,
    write_text_output,
};
use chemcore_engine::{
    document_to_cdxml, document_to_svg, primitives_to_svg_viewbox, render_document,
    render_document_targets, render_primitives_bounds, Bond, ChemcoreDocument, Engine, Node,
    RenderPrimitive, ResourceData, SceneObject,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CAPTURE_SCALE: f64 = 4.0;
const DEFAULT_OUTPUT_DIR_NAME: &str = "chemcore-cli";
const MAX_CAPTURE_SIDE_PX: u32 = 32_000;
const MAX_CAPTURE_PIXELS: u64 = 120_000_000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TargetSelector {
    All,
    Object(String),
    Molecule(usize),
    Node(String),
    Bond(String),
    Bounds([f64; 4]),
    Selection(Vec<TargetSelector>),
}

impl TargetSelector {
    fn selector(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Object(id) => format!("object:{id}"),
            Self::Molecule(index) => format!("molecule:{index}"),
            Self::Node(id) => format!("node:{id}"),
            Self::Bond(id) => format!("bond:{id}"),
            Self::Bounds(bounds) => format!(
                "bounds:{},{},{},{}",
                bounds[0], bounds[1], bounds[2], bounds[3]
            ),
            Self::Selection(targets) => format!(
                "selection:{}",
                targets
                    .iter()
                    .map(TargetSelector::selector)
                    .collect::<Vec<_>>()
                    .join(";")
            ),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Object(_) => "object",
            Self::Molecule(_) => "molecule",
            Self::Node(_) => "node",
            Self::Bond(_) => "bond",
            Self::Bounds(_) => "bounds",
            Self::Selection(_) => "selection",
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::All => json!({ "kind": self.kind(), "selector": self.selector() }),
            Self::Object(id) => {
                json!({ "kind": self.kind(), "selector": self.selector(), "id": id })
            }
            Self::Molecule(index) => {
                json!({ "kind": self.kind(), "selector": self.selector(), "index": index })
            }
            Self::Node(id) | Self::Bond(id) => {
                json!({ "kind": self.kind(), "selector": self.selector(), "id": id })
            }
            Self::Bounds(bounds) => json!({
                "kind": self.kind(),
                "selector": self.selector(),
                "bounds": bounds_json(*bounds),
            }),
            Self::Selection(targets) => json!({
                "kind": self.kind(),
                "selector": self.selector(),
                "targetCount": targets.len(),
                "targets": targets.iter().map(TargetSelector::to_json).collect::<Vec<_>>(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureFormat {
    Svg,
    Png,
}

impl CaptureFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CropExpansion {
    abs_left: f64,
    abs_top: f64,
    abs_right: f64,
    abs_bottom: f64,
    rel_left: f64,
    rel_top: f64,
    rel_right: f64,
    rel_bottom: f64,
}

impl CropExpansion {
    fn uniform_abs(value: f64) -> Self {
        Self {
            abs_left: value,
            abs_top: value,
            abs_right: value,
            abs_bottom: value,
            rel_left: 0.0,
            rel_top: 0.0,
            rel_right: 0.0,
            rel_bottom: 0.0,
        }
    }

    fn left_for(self, width: f64) -> f64 {
        self.abs_left + width * self.rel_left
    }

    fn right_for(self, width: f64) -> f64 {
        self.abs_right + width * self.rel_right
    }

    fn top_for(self, height: f64) -> f64 {
        self.abs_top + height * self.rel_top
    }

    fn bottom_for(self, height: f64) -> f64 {
        self.abs_bottom + height * self.rel_bottom
    }

    fn to_json(self) -> Value {
        json!({
            "absolute": {
                "left": self.abs_left,
                "top": self.abs_top,
                "right": self.abs_right,
                "bottom": self.abs_bottom,
            },
            "relative": {
                "left": self.rel_left,
                "top": self.rel_top,
                "right": self.rel_right,
                "bottom": self.rel_bottom,
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RasterOptions {
    scale: f64,
    width: Option<u32>,
    height: Option<u32>,
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            scale: DEFAULT_CAPTURE_SCALE,
            width: None,
            height: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PixelSize {
    width: u32,
    height: u32,
    scale_x: f64,
    scale_y: f64,
}

impl PixelSize {
    fn to_json(self) -> Value {
        json!({
            "width": self.width,
            "height": self.height,
            "scaleX": self.scale_x,
            "scaleY": self.scale_y,
        })
    }
}

struct CaptureRender {
    primitives: Vec<RenderPrimitive>,
    mode: &'static str,
    targets: RegionRenderTargets,
}

#[derive(Default)]
struct RegionRenderTargets {
    nodes: BTreeSet<String>,
    bonds: BTreeSet<String>,
    objects: BTreeSet<String>,
}

impl RegionRenderTargets {
    fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.bonds.is_empty() && self.objects.is_empty()
    }

    fn to_json(&self) -> Value {
        json!({
            "nodes": self.nodes.len(),
            "bonds": self.bonds.len(),
            "objects": self.objects.len(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct DetailOptions {
    include_raw: bool,
    include_resource: bool,
}

pub(crate) fn targets_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
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
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected targets argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "targets requires an input file.".to_string())?;
    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    write_json_value(targets_report(&input, &document), output.as_deref(), pretty)
}

fn targets_report(input: &str, document: &ChemcoreDocument) -> Value {
    let objects = object_target_entries(document);
    let molecules = molecule_target_entries(document);
    let nodes = node_target_entries(document);
    let bonds = bond_target_entries(document);
    let target_count = 1 + objects.len() + molecules.len() + nodes.len() + bonds.len();
    let all_bounds = target_bounds_fast(document, &TargetSelector::All)
        .or_else(|| target_bounds(document, &TargetSelector::All).ok());
    json!({
        "ok": true,
        "input": input,
        "targetCount": target_count,
        "targets": {
            "all": {
                "selector": "all",
                "bounds": all_bounds.map(bounds_json),
            },
            "objects": objects,
            "molecules": molecules,
            "nodes": nodes,
            "bonds": bonds,
        }
    })
}

fn add_target_arg(target: &mut Option<TargetSelector>, next: TargetSelector) -> Result<(), String> {
    match target.take() {
        None => *target = Some(next),
        Some(existing) => {
            let mut targets = Vec::new();
            collect_selection_targets(existing, &mut targets);
            collect_selection_targets(next, &mut targets);
            *target = Some(target_from_selection_targets(targets)?);
        }
    }
    Ok(())
}

fn collect_selection_targets(target: TargetSelector, out: &mut Vec<TargetSelector>) {
    match target {
        TargetSelector::Selection(targets) => {
            for target in targets {
                collect_selection_targets(target, out);
            }
        }
        target => out.push(target),
    }
}

fn target_from_selection_targets(
    mut targets: Vec<TargetSelector>,
) -> Result<TargetSelector, String> {
    if targets.is_empty() {
        return Err("Selection requires at least one target.".to_string());
    }
    if targets.len() == 1 {
        return Ok(targets.remove(0));
    }
    if targets
        .iter()
        .any(|target| matches!(target, TargetSelector::All))
    {
        return Err("Multi-target selection uses object, molecule, node, bond, or bounds selectors; use all by itself for whole-document capture.".to_string());
    }
    Ok(TargetSelector::Selection(targets))
}

pub(crate) fn capture_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut format = None;
    let mut expansion = CropExpansion::uniform_abs(8.0);
    let mut raster = RasterOptions::default();
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selector(
                        args.get(index)
                            .ok_or_else(|| "--target requires a selector.".to_string())?,
                    )?,
                )?;
            }
            "--targets" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selection_arg(args.get(index).ok_or_else(|| {
                        "--targets requires selectors separated by semicolons.".to_string()
                    })?)?,
                )?;
            }
            "--object" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Object(
                        args.get(index)
                            .ok_or_else(|| "--object requires an object id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--molecule" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Molecule(parse_usize_arg("--molecule", args.get(index))?),
                )?;
            }
            "--node" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Node(
                        args.get(index)
                            .ok_or_else(|| "--node requires a node id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bond" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Bond(
                        args.get(index)
                            .ok_or_else(|| "--bond requires a bond id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bounds" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Bounds(parse_bounds_arg(
                        args.get(index)
                            .ok_or_else(|| "--bounds requires minX,minY,maxX,maxY.".to_string())?,
                    )?),
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
            "--format" | "-f" => {
                index += 1;
                format =
                    Some(parse_capture_format(args.get(index).ok_or_else(|| {
                        "--format requires svg or png.".to_string()
                    })?)?);
            }
            "--padding" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--padding",
                    args.get(index)
                        .ok_or_else(|| "--padding requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_top = value;
                expansion.abs_right = value;
                expansion.abs_bottom = value;
            }
            "--expand" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand",
                    args.get(index)
                        .ok_or_else(|| "--expand requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_top = value;
                expansion.abs_right = value;
                expansion.abs_bottom = value;
            }
            "--expand-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-x requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_right = value;
            }
            "--expand-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-y requires a number.".to_string())?,
                )?;
                expansion.abs_top = value;
                expansion.abs_bottom = value;
            }
            "--expand-left" => {
                index += 1;
                expansion.abs_left = parse_non_negative_f64(
                    "--expand-left",
                    args.get(index)
                        .ok_or_else(|| "--expand-left requires a number.".to_string())?,
                )?;
            }
            "--expand-right" => {
                index += 1;
                expansion.abs_right = parse_non_negative_f64(
                    "--expand-right",
                    args.get(index)
                        .ok_or_else(|| "--expand-right requires a number.".to_string())?,
                )?;
            }
            "--expand-top" => {
                index += 1;
                expansion.abs_top = parse_non_negative_f64(
                    "--expand-top",
                    args.get(index)
                        .ok_or_else(|| "--expand-top requires a number.".to_string())?,
                )?;
            }
            "--expand-bottom" => {
                index += 1;
                expansion.abs_bottom = parse_non_negative_f64(
                    "--expand-bottom",
                    args.get(index)
                        .ok_or_else(|| "--expand-bottom requires a number.".to_string())?,
                )?;
            }
            "--expand-rel" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_top = value;
                expansion.rel_right = value;
                expansion.rel_bottom = value;
            }
            "--expand-rel-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-x requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_right = value;
            }
            "--expand-rel-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-y requires a fraction.".to_string())?,
                )?;
                expansion.rel_top = value;
                expansion.rel_bottom = value;
            }
            "--expand-rel-left" => {
                index += 1;
                expansion.rel_left = parse_non_negative_f64(
                    "--expand-rel-left",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-left requires a fraction.".to_string())?,
                )?;
            }
            "--expand-rel-right" => {
                index += 1;
                expansion.rel_right = parse_non_negative_f64(
                    "--expand-rel-right",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-right requires a fraction.".to_string())?,
                )?;
            }
            "--expand-rel-top" => {
                index += 1;
                expansion.rel_top = parse_non_negative_f64(
                    "--expand-rel-top",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-top requires a fraction.".to_string())?,
                )?;
            }
            "--expand-rel-bottom" => {
                index += 1;
                expansion.rel_bottom = parse_non_negative_f64(
                    "--expand-rel-bottom",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-bottom requires a fraction.".to_string())?,
                )?;
            }
            "--scale" => {
                index += 1;
                raster.scale = parse_positive_f64(
                    "--scale",
                    args.get(index)
                        .ok_or_else(|| "--scale requires a positive number.".to_string())?,
                )?;
            }
            "--width" => {
                index += 1;
                raster.width = Some(parse_positive_u32(
                    "--width",
                    args.get(index)
                        .ok_or_else(|| "--width requires a positive integer.".to_string())?,
                )?);
            }
            "--height" => {
                index += 1;
                raster.height = Some(parse_positive_u32(
                    "--height",
                    args.get(index)
                        .ok_or_else(|| "--height requires a positive integer.".to_string())?,
                )?);
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected capture argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "capture requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "capture requires --target <object:id|molecule:index|node:id|bond:id|all>, repeated --target values, --targets, or --bounds."
            .to_string()
    })?;
    let (output, format, output_defaulted) = resolve_capture_output(output, format)?;

    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let bounds = target_bounds(&document, &target)?;
    let view_box = expanded_view_box(bounds, expansion);
    let render = capture_render_primitives(&document, &target, view_box);
    let render_output =
        write_capture_output(&render.primitives, view_box, &output, format, raster)?;
    let primitive_count = render.primitives.len();
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
            "warnings": default_capture_warnings(output_defaulted, &output),
            "output": {
                "path": output,
                "format": format.as_str(),
                "defaulted": output_defaulted,
                "verified": true,
                "bytes": render_output.bytes,
                "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
            },
            "bounds": bounds_json(bounds),
            "viewBox": view_box_json(view_box),
            "expansion": expansion.to_json(),
            "render": {
                "mode": render.mode,
                "primitiveCount": primitive_count,
                "targets": render.targets.to_json(),
            },
        }),
        None,
        pretty,
    )
}

pub(crate) fn copy_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut office_helper = None;
    let mut payload_path = None;
    let mut copy_to_clipboard = true;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" => {
                index += 1;
                target = Some(parse_target_selector(
                    args.get(index)
                        .ok_or_else(|| "--target requires a selector.".to_string())?,
                )?);
            }
            "--object" => {
                index += 1;
                target = Some(TargetSelector::Object(
                    args.get(index)
                        .ok_or_else(|| "--object requires an object id.".to_string())?
                        .clone(),
                ));
            }
            "--molecule" => {
                index += 1;
                target = Some(TargetSelector::Molecule(parse_usize_arg(
                    "--molecule",
                    args.get(index),
                )?));
            }
            "--node" => {
                index += 1;
                target = Some(TargetSelector::Node(
                    args.get(index)
                        .ok_or_else(|| "--node requires a node id.".to_string())?
                        .clone(),
                ));
            }
            "--bond" => {
                index += 1;
                target = Some(TargetSelector::Bond(
                    args.get(index)
                        .ok_or_else(|| "--bond requires a bond id.".to_string())?
                        .clone(),
                ));
            }
            "--all" => target = Some(TargetSelector::All),
            "--office-helper" => {
                index += 1;
                office_helper = Some(
                    args.get(index)
                        .ok_or_else(|| "--office-helper requires a path.".to_string())?
                        .clone(),
                );
            }
            "--payload" => {
                index += 1;
                payload_path = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--payload requires a path.".to_string())?,
                ));
            }
            "--no-copy" => copy_to_clipboard = false,
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected copy argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "copy requires an input file.".to_string())?;
    let target = target.unwrap_or(TargetSelector::All);
    if matches!(target, TargetSelector::Bounds(_)) {
        return Err("copy targets must be all, object, molecule, node, or bond; bounds are only for capture."
            .to_string());
    }

    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let clipboard_document = clipboard_document_for_target(&document, &target)?;
    let payload = clipboard_payload_for_document(&clipboard_document)?;
    let payload_defaulted = payload_path.is_none();
    let payload_path = payload_path.unwrap_or_else(default_clipboard_payload_path);
    let payload_bytes = write_clipboard_payload_file(&payload_path, &payload)?;

    let copied_helper = if copy_to_clipboard {
        Some(copy_payload_to_office_clipboard(
            &payload_path,
            office_helper.as_deref(),
        )?)
    } else {
        None
    };
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
            "warnings": default_payload_warnings(payload_defaulted, &payload_path),
            "payload": {
                "path": payload_path.display().to_string(),
                "defaulted": payload_defaulted,
                "verified": true,
                "bytes": payload_bytes,
            },
            "clipboard": {
                "copied": copy_to_clipboard,
                "helper": copied_helper.map(|path| path.display().to_string()),
                "format": "windows-office-ole",
            },
            "document": {
                "objects": clipboard_document.objects.len(),
                "resources": clipboard_document.resources.len(),
            }
        }),
        None,
        pretty,
    )
}

struct SessionDocument {
    input: String,
    engine: Engine,
    document: ChemcoreDocument,
}

impl SessionDocument {
    fn open(input: String) -> Result<Self, String> {
        let engine = load_engine_from_file(&input)?;
        let document = engine_document(&engine)?;
        Ok(Self {
            input,
            engine,
            document,
        })
    }

    fn refresh_document(&mut self) -> Result<(), String> {
        self.document = engine_document(&self.engine)?;
        Ok(())
    }

    fn summary_json(&self) -> Value {
        let fragments = self.document.editable_fragments();
        json!({
            "input": self.input,
            "revision": self.engine.revision(),
            "objects": self.document.objects.len(),
            "molecules": fragments.len(),
            "nodes": fragments.iter().map(|entry| entry.fragment.nodes.len()).sum::<usize>(),
            "bonds": fragments.iter().map(|entry| entry.fragment.bonds.len()).sum::<usize>(),
        })
    }
}

pub(crate) fn session_command(args: &[String]) -> Result<(), String> {
    let mut initial_input = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--input" | "-i" => {
                index += 1;
                initial_input = Some(
                    args.get(index)
                        .ok_or_else(|| "--input requires a path.".to_string())?
                        .clone(),
                );
            }
            value if initial_input.is_none() => initial_input = Some(value.to_string()),
            value => return Err(format!("Unexpected session argument '{value}'.")),
        }
        index += 1;
    }

    let mut session = match initial_input {
        Some(input) => Some(SessionDocument::open(input)?),
        None => None,
    };

    write_session_line(session_ready_json(session.as_ref()))?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("Failed to read session input: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                write_session_line(session_error(
                    Value::Null,
                    Value::Null,
                    "invalid_json",
                    format!("Invalid JSON request: {error}"),
                ))?;
                continue;
            }
        };
        let (response, exit) = handle_session_request(&mut session, request);
        write_session_line(response)?;
        if exit {
            break;
        }
    }
    Ok(())
}

fn handle_session_request(session: &mut Option<SessionDocument>, request: Value) -> (Value, bool) {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let op_value = request
        .get("op")
        .or_else(|| request.get("operation"))
        .or_else(|| request.get("command"))
        .cloned()
        .unwrap_or(Value::Null);
    let Some(op) = op_value.as_str() else {
        return (
            session_error(
                id,
                op_value,
                "missing_operation",
                "Session request requires op, operation, or command.".to_string(),
            ),
            false,
        );
    };
    let op = op.trim().to_ascii_lowercase();
    let result = match op.as_str() {
        "help" | "capabilities" => Ok(session_help_json()),
        "open" => session_open(session, &request),
        "close" => {
            *session = None;
            Ok(json!({ "closed": true }))
        }
        "targets" => with_session(session, |document| {
            Ok(targets_report(&document.input, &document.document))
        }),
        "detail" | "details" | "describe" | "show" => with_session(session, |document| {
            let target = request_required_target(&request)?;
            let summary_only =
                request_bool(&request, &["summaryOnly", "summary-only", "noRaw"])?.unwrap_or(false);
            let include_resource =
                request_bool(&request, &["includeResource", "include-resource"])?.unwrap_or(false);
            let options = DetailOptions {
                include_raw: !summary_only,
                include_resource,
            };
            detail_report(&document.input, &document.document, &target, options)
        }),
        "context" => with_session(session, |document| session_context(document, &request)),
        "capture" | "screenshot" => {
            with_session(session, |document| session_capture(document, &request))
        }
        "execute" | "run" => {
            with_session_mut(session, |document| session_execute(document, &request))
        }
        "save" => with_session(session, |document| session_save(document, &request)),
        "status" => with_session(session, |document| Ok(document.summary_json())),
        "exit" | "quit" => Ok(json!({ "exiting": true })),
        _ => Err(format!(
            "Unknown session operation '{op}'. Use op=help for supported operations."
        )),
    };

    let exit = matches!(op.as_str(), "exit" | "quit") && result.is_ok();
    match result {
        Ok(result) => (session_ok(id, op, result), exit),
        Err(error) => (
            session_error(id, json!(op), "operation_failed", error),
            false,
        ),
    }
}

fn session_open(session: &mut Option<SessionDocument>, request: &Value) -> Result<Value, String> {
    let input = request_required_string(request, &["input", "path", "file"])?;
    let document = SessionDocument::open(input)?;
    let summary = document.summary_json();
    *session = Some(document);
    Ok(summary)
}

fn session_context(document: &SessionDocument, request: &Value) -> Result<Value, String> {
    let target = request_required_target(request)?;
    let expansion = request_expansion(request, CropExpansion::uniform_abs(30.0))?;
    let raster = request_raster_options(request)?;
    let limit = request_usize(request, &["limit"])?.unwrap_or(200);
    let target_bounds = target_bounds(&document.document, &target)?;
    let query_view_box = expanded_view_box(target_bounds, expansion);
    let query_bounds = view_box_to_bounds(query_view_box);
    let mut report = context_report(
        &document.input,
        &document.document,
        &target,
        target_bounds,
        query_bounds,
        expansion,
        limit,
    )?;

    if let Some(capture_output) = request_string(
        request,
        &["captureOut", "capture-out", "capture_out", "screenshotOut"],
    )? {
        let format = request_capture_format(request)?
            .or_else(|| infer_capture_format_from_path(&capture_output))
            .ok_or_else(|| {
                "captureOut format is ambiguous; use .svg/.png or format=svg|png.".to_string()
            })?;
        let render = capture_render_primitives(&document.document, &target, query_view_box);
        let render_output = write_capture_output(
            &render.primitives,
            query_view_box,
            &capture_output,
            format,
            raster,
        )?;
        let primitive_count = render.primitives.len();
        set_object_field(
            &mut report,
            "capture",
            json!({
                "ok": true,
                "path": capture_output,
                "format": format.as_str(),
                "verified": true,
                "bytes": render_output.bytes,
                "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
                "viewBox": view_box_json(query_view_box),
                "render": {
                    "mode": render.mode,
                    "primitiveCount": primitive_count,
                    "targets": render.targets.to_json(),
                },
            }),
        );
    }
    Ok(report)
}

fn session_capture(document: &SessionDocument, request: &Value) -> Result<Value, String> {
    let target = request_required_target(request)?;
    let output = request_string(request, &["out", "output", "path"])?;
    let format = request_capture_format(request)?;
    let expansion = request_expansion(request, CropExpansion::uniform_abs(8.0))?;
    let raster = request_raster_options(request)?;
    let (output, format, output_defaulted) = resolve_capture_output(output, format)?;
    let bounds = target_bounds(&document.document, &target)?;
    let view_box = expanded_view_box(bounds, expansion);
    let render = capture_render_primitives(&document.document, &target, view_box);
    let render_output =
        write_capture_output(&render.primitives, view_box, &output, format, raster)?;
    let primitive_count = render.primitives.len();
    Ok(json!({
        "ok": true,
        "input": document.input,
        "target": target.to_json(),
        "warnings": default_capture_warnings(output_defaulted, &output),
        "output": {
            "path": output,
            "format": format.as_str(),
            "defaulted": output_defaulted,
            "verified": true,
            "bytes": render_output.bytes,
            "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
        },
        "bounds": bounds_json(bounds),
        "viewBox": view_box_json(view_box),
        "expansion": expansion.to_json(),
        "render": {
            "mode": render.mode,
            "primitiveCount": primitive_count,
            "targets": render.targets.to_json(),
        },
    }))
}

fn session_execute(document: &mut SessionDocument, request: &Value) -> Result<Value, String> {
    let commands = session_request_commands(request)?;
    let continue_on_error =
        request_bool(request, &["continueOnError", "continue-on-error"])?.unwrap_or(false);
    let before_revision = document.engine.revision();
    let mut results = Vec::new();
    let mut failed_indices = Vec::new();
    for (index, command) in commands.into_iter().enumerate() {
        let command_type = session_command_type_name(&command);
        let command_before_revision = document.engine.revision();
        match document.engine.execute_command_json(&command.to_string()) {
            Ok(result_text) => {
                let engine_result: Value =
                    serde_json::from_str(&result_text).map_err(|error| error.to_string())?;
                results.push(json!({
                    "index": index,
                    "ok": true,
                    "changed": engine_result.get("changed").and_then(Value::as_bool).unwrap_or(false),
                    "commandType": command_type,
                    "beforeRevision": command_before_revision,
                    "afterRevision": document.engine.revision(),
                    "result": engine_result,
                }));
            }
            Err(error) => {
                failed_indices.push(index);
                results.push(json!({
                    "index": index,
                    "ok": false,
                    "changed": false,
                    "commandType": command_type,
                    "beforeRevision": command_before_revision,
                    "afterRevision": document.engine.revision(),
                    "error": {
                        "message": error,
                    },
                }));
                if !continue_on_error {
                    break;
                }
            }
        }
    }
    document.refresh_document()?;
    let failed_count = failed_indices.len();
    Ok(json!({
        "ok": failed_count == 0,
        "commandCount": results.len(),
        "failedCount": failed_count,
        "failedIndices": failed_indices,
        "continueOnError": continue_on_error,
        "document": {
            "beforeRevision": before_revision,
            "afterRevision": document.engine.revision(),
            "revisionChanged": before_revision != document.engine.revision(),
        },
        "results": results,
    }))
}

fn session_save(document: &SessionDocument, request: &Value) -> Result<Value, String> {
    let output = request_required_string(request, &["out", "output", "path"])?;
    let format = request_string(request, &["format", "saveFormat", "save-format"])?;
    write_engine_output(&document.engine, &output, format.as_deref())?;
    Ok(json!({
        "ok": true,
        "path": output,
        "format": format.or_else(|| infer_format_from_path(&output)),
        "revision": document.engine.revision(),
    }))
}

fn with_session<F>(session: &Option<SessionDocument>, f: F) -> Result<Value, String>
where
    F: FnOnce(&SessionDocument) -> Result<Value, String>,
{
    let Some(document) = session.as_ref() else {
        return Err(
            "No document is open. Send {\"op\":\"open\",\"input\":\"path\"} first.".to_string(),
        );
    };
    f(document)
}

fn with_session_mut<F>(session: &mut Option<SessionDocument>, f: F) -> Result<Value, String>
where
    F: FnOnce(&mut SessionDocument) -> Result<Value, String>,
{
    let Some(document) = session.as_mut() else {
        return Err(
            "No document is open. Send {\"op\":\"open\",\"input\":\"path\"} first.".to_string(),
        );
    };
    f(document)
}

fn write_session_line(value: Value) -> Result<(), String> {
    let mut stdout = io::stdout();
    serde_json::to_writer(&mut stdout, &value).map_err(|error| error.to_string())?;
    stdout.write_all(b"\n").map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}

fn session_ready_json(session: Option<&SessionDocument>) -> Value {
    json!({
        "ok": true,
        "event": "ready",
        "protocol": "chemcore-cli-session-jsonl-v1",
        "input": session.map(|document| document.input.clone()),
        "document": session.map(SessionDocument::summary_json),
        "help": {
            "request": {"id": 1, "op": "help"},
            "open": {"id": 2, "op": "open", "input": "input.cdxml"},
            "capture": {"id": 3, "op": "capture", "target": "molecule:0", "out": "crop.png", "scale": 6},
            "captureSelection": {"id": 4, "op": "capture", "target": ["object:obj_a", "object:obj_b"], "out": "selection.png", "width": 1800},
            "exit": {"id": 99, "op": "exit"},
        }
    })
}

fn session_help_json() -> Value {
    json!({
        "protocol": "chemcore-cli-session-jsonl-v1",
        "transport": "stdin/stdout JSON Lines; one compact JSON response per request.",
        "operations": {
            "open": {"required": ["input"], "description": "Load a document into the session."},
            "targets": {"description": "Return stable selectors and bounds for the open document."},
            "detail": {"required": ["target"], "description": "Return one object/molecule/node/bond detail JSON."},
            "context": {"required": ["target"], "optional": ["targets", "radius", "captureOut", "scale", "width", "height", "limit"], "description": "Return nearby summaries and optionally a screenshot. target/targets may be a selector string or an array of selector strings."},
            "capture": {"required": ["target"], "optional": ["targets", "out", "format", "scale", "width", "height", "expand", "expandRel"], "description": "Write a precise crop; target/targets may be a selector string or an array. Multi-target crops use the minimum union bounds."},
            "execute": {"required": ["command or commands"], "optional": ["continueOnError"], "description": "Run one or more engine JSON commands against the in-memory document."},
            "save": {"required": ["out"], "optional": ["format"], "description": "Save the current in-memory document."},
            "status": {"description": "Return the open document summary."},
            "close": {"description": "Close the open document without saving."},
            "exit": {"description": "Terminate the session process."}
        },
        "targetSelectors": ["all", "object:<id>", "molecule:<index>", "node:<id>", "bond:<id>", "bounds:minX,minY,maxX,maxY", "selection:<selector;selector>"]
    })
}

fn session_ok(id: Value, op: String, result: Value) -> Value {
    let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(true);
    json!({
        "ok": ok,
        "id": id,
        "op": op,
        "result": result,
    })
}

fn session_error(id: Value, op: Value, kind: &str, message: String) -> Value {
    json!({
        "ok": false,
        "id": id,
        "op": op,
        "error": {
            "kind": kind,
            "message": message,
        }
    })
}

fn request_required_target(request: &Value) -> Result<TargetSelector, String> {
    request_target(request)?.ok_or_else(|| {
        "Request requires target, object, molecule, node, bond, or bounds.".to_string()
    })
}

fn request_target(request: &Value) -> Result<Option<TargetSelector>, String> {
    if let Some(target) = request.get("target") {
        return parse_target_value(target).map(Some);
    }
    if let Some(targets) = request.get("targets") {
        return parse_target_value(targets).map(Some);
    }
    if let Some(id) = request.get("object").and_then(Value::as_str) {
        return Ok(Some(TargetSelector::Object(id.to_string())));
    }
    if let Some(index) = request.get("molecule").and_then(Value::as_u64) {
        return Ok(Some(TargetSelector::Molecule(index as usize)));
    }
    if let Some(id) = request.get("node").and_then(Value::as_str) {
        return Ok(Some(TargetSelector::Node(id.to_string())));
    }
    if let Some(id) = request.get("bond").and_then(Value::as_str) {
        return Ok(Some(TargetSelector::Bond(id.to_string())));
    }
    if let Some(bounds) = request.get("bounds") {
        return parse_bounds_value(bounds)
            .map(TargetSelector::Bounds)
            .map(Some);
    }
    Ok(None)
}

fn parse_target_value(value: &Value) -> Result<TargetSelector, String> {
    if let Some(target) = value.as_str() {
        return parse_target_selector(target);
    }
    let Some(values) = value.as_array() else {
        return Err(
            "target must be a selector string or an array of selector strings.".to_string(),
        );
    };
    let mut targets = Vec::new();
    for value in values {
        let Some(target) = value.as_str() else {
            return Err("target arrays must contain selector strings.".to_string());
        };
        collect_selection_targets(parse_target_selector(target)?, &mut targets);
    }
    target_from_selection_targets(targets)
}

fn parse_bounds_value(value: &Value) -> Result<[f64; 4], String> {
    if let Some(text) = value.as_str() {
        return parse_bounds_arg(text);
    }
    let Some(values) = value.as_array() else {
        return Err("bounds must be a string or an array of four numbers.".to_string());
    };
    if values.len() != 4 {
        return Err("bounds array must contain four numbers.".to_string());
    }
    let mut out = [0.0; 4];
    for (index, value) in values.iter().enumerate() {
        out[index] = value
            .as_f64()
            .ok_or_else(|| "bounds array values must be finite numbers.".to_string())?;
        if !out[index].is_finite() {
            return Err("bounds array values must be finite numbers.".to_string());
        }
    }
    if out[2] <= out[0] || out[3] <= out[1] {
        return Err("bounds must satisfy maxX > minX and maxY > minY.".to_string());
    }
    Ok(out)
}

fn request_expansion(
    request: &Value,
    mut expansion: CropExpansion,
) -> Result<CropExpansion, String> {
    if let Some(value) = request_f64(request, &["radius", "padding", "expand"])? {
        expansion.abs_left = value;
        expansion.abs_top = value;
        expansion.abs_right = value;
        expansion.abs_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandX", "expand-x"])? {
        expansion.abs_left = value;
        expansion.abs_right = value;
    }
    if let Some(value) = request_f64(request, &["expandY", "expand-y"])? {
        expansion.abs_top = value;
        expansion.abs_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandLeft", "expand-left"])? {
        expansion.abs_left = value;
    }
    if let Some(value) = request_f64(request, &["expandRight", "expand-right"])? {
        expansion.abs_right = value;
    }
    if let Some(value) = request_f64(request, &["expandTop", "expand-top"])? {
        expansion.abs_top = value;
    }
    if let Some(value) = request_f64(request, &["expandBottom", "expand-bottom"])? {
        expansion.abs_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandRel", "expand-rel"])? {
        expansion.rel_left = value;
        expansion.rel_top = value;
        expansion.rel_right = value;
        expansion.rel_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandRelX", "expand-rel-x"])? {
        expansion.rel_left = value;
        expansion.rel_right = value;
    }
    if let Some(value) = request_f64(request, &["expandRelY", "expand-rel-y"])? {
        expansion.rel_top = value;
        expansion.rel_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandRelLeft", "expand-rel-left"])? {
        expansion.rel_left = value;
    }
    if let Some(value) = request_f64(request, &["expandRelRight", "expand-rel-right"])? {
        expansion.rel_right = value;
    }
    if let Some(value) = request_f64(request, &["expandRelTop", "expand-rel-top"])? {
        expansion.rel_top = value;
    }
    if let Some(value) = request_f64(request, &["expandRelBottom", "expand-rel-bottom"])? {
        expansion.rel_bottom = value;
    }
    Ok(expansion)
}

fn request_raster_options(request: &Value) -> Result<RasterOptions, String> {
    let mut raster = RasterOptions::default();
    if let Some(scale) = request_f64(request, &["scale"])? {
        if scale <= 0.0 {
            return Err("scale must be positive.".to_string());
        }
        raster.scale = scale;
    }
    raster.width = request_u32(request, &["width"])?;
    raster.height = request_u32(request, &["height"])?;
    Ok(raster)
}

fn request_capture_format(request: &Value) -> Result<Option<CaptureFormat>, String> {
    request_string(request, &["format"]).and_then(|value| {
        value
            .map(|format| parse_capture_format(&format))
            .transpose()
    })
}

fn session_request_commands(request: &Value) -> Result<Vec<Value>, String> {
    if let Some(command) = request.get("command").filter(|value| value.is_object()) {
        return Ok(vec![command.clone()]);
    }
    if let Some(commands) = request.get("commands").and_then(Value::as_array) {
        if commands.is_empty() {
            return Err("commands must not be empty.".to_string());
        }
        return Ok(commands.clone());
    }
    Err("execute requires command object or commands array.".to_string())
}

fn session_command_type_name(command: &Value) -> Value {
    command
        .get("type")
        .and_then(Value::as_str)
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn request_required_string(request: &Value, keys: &[&str]) -> Result<String, String> {
    request_string(request, keys)?.ok_or_else(|| {
        format!(
            "Request requires one of: {}.",
            keys.iter().copied().collect::<Vec<_>>().join(", ")
        )
    })
}

fn request_string(request: &Value, keys: &[&str]) -> Result<Option<String>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            return value
                .as_str()
                .map(|text| Some(text.to_string()))
                .ok_or_else(|| format!("{key} must be a string."));
        }
    }
    Ok(None)
}

fn request_bool(request: &Value, keys: &[&str]) -> Result<Option<bool>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            return value
                .as_bool()
                .map(Some)
                .ok_or_else(|| format!("{key} must be a boolean."));
        }
    }
    Ok(None)
}

fn request_f64(request: &Value, keys: &[&str]) -> Result<Option<f64>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            let Some(number) = value.as_f64() else {
                return Err(format!("{key} must be a number."));
            };
            if number < 0.0 || !number.is_finite() {
                return Err(format!("{key} must be a non-negative finite number."));
            }
            return Ok(Some(number));
        }
    }
    Ok(None)
}

fn request_u32(request: &Value, keys: &[&str]) -> Result<Option<u32>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            let Some(number) = value.as_u64() else {
                return Err(format!("{key} must be a positive integer."));
            };
            if number == 0 || number > u32::MAX as u64 {
                return Err(format!(
                    "{key} must be a positive integer up to {}.",
                    u32::MAX
                ));
            }
            return Ok(Some(number as u32));
        }
    }
    Ok(None)
}

fn request_usize(request: &Value, keys: &[&str]) -> Result<Option<usize>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            let Some(number) = value.as_u64() else {
                return Err(format!("{key} must be a positive integer."));
            };
            if number == 0 || number > usize::MAX as u64 {
                return Err(format!("{key} must be a positive integer."));
            }
            return Ok(Some(number as usize));
        }
    }
    Ok(None)
}

pub(crate) fn context_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut capture_output = None;
    let mut capture_format = None;
    let mut expansion = CropExpansion::uniform_abs(30.0);
    let mut raster = RasterOptions::default();
    let mut limit = 200usize;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" | "--around" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selector(
                        args.get(index)
                            .ok_or_else(|| "--target requires a selector.".to_string())?,
                    )?,
                )?;
            }
            "--targets" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selection_arg(args.get(index).ok_or_else(|| {
                        "--targets requires selectors separated by semicolons.".to_string()
                    })?)?,
                )?;
            }
            "--object" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Object(
                        args.get(index)
                            .ok_or_else(|| "--object requires an object id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--molecule" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Molecule(parse_usize_arg("--molecule", args.get(index))?),
                )?;
            }
            "--node" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Node(
                        args.get(index)
                            .ok_or_else(|| "--node requires a node id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bond" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Bond(
                        args.get(index)
                            .ok_or_else(|| "--bond requires a bond id.".to_string())?
                            .clone(),
                    ),
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
            "--capture-out" | "--screenshot-out" => {
                index += 1;
                capture_output = Some(
                    args.get(index)
                        .ok_or_else(|| "--capture-out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                capture_format =
                    Some(parse_capture_format(args.get(index).ok_or_else(|| {
                        "--format requires svg or png.".to_string()
                    })?)?);
            }
            "--limit" => {
                index += 1;
                limit = parse_usize_arg("--limit", args.get(index))?;
            }
            "--radius" | "--padding" | "--expand" => {
                index += 1;
                let value = parse_non_negative_f64(
                    args[index - 1].as_str(),
                    args.get(index)
                        .ok_or_else(|| format!("{} requires a number.", args[index - 1]))?,
                )?;
                expansion.abs_left = value;
                expansion.abs_top = value;
                expansion.abs_right = value;
                expansion.abs_bottom = value;
            }
            "--expand-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-x requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_right = value;
            }
            "--expand-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-y requires a number.".to_string())?,
                )?;
                expansion.abs_top = value;
                expansion.abs_bottom = value;
            }
            "--expand-left" => {
                index += 1;
                expansion.abs_left = parse_non_negative_f64(
                    "--expand-left",
                    args.get(index)
                        .ok_or_else(|| "--expand-left requires a number.".to_string())?,
                )?;
            }
            "--expand-right" => {
                index += 1;
                expansion.abs_right = parse_non_negative_f64(
                    "--expand-right",
                    args.get(index)
                        .ok_or_else(|| "--expand-right requires a number.".to_string())?,
                )?;
            }
            "--expand-top" => {
                index += 1;
                expansion.abs_top = parse_non_negative_f64(
                    "--expand-top",
                    args.get(index)
                        .ok_or_else(|| "--expand-top requires a number.".to_string())?,
                )?;
            }
            "--expand-bottom" => {
                index += 1;
                expansion.abs_bottom = parse_non_negative_f64(
                    "--expand-bottom",
                    args.get(index)
                        .ok_or_else(|| "--expand-bottom requires a number.".to_string())?,
                )?;
            }
            "--expand-rel" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_top = value;
                expansion.rel_right = value;
                expansion.rel_bottom = value;
            }
            "--expand-rel-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-x requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_right = value;
            }
            "--expand-rel-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-y requires a fraction.".to_string())?,
                )?;
                expansion.rel_top = value;
                expansion.rel_bottom = value;
            }
            "--scale" => {
                index += 1;
                raster.scale = parse_positive_f64(
                    "--scale",
                    args.get(index)
                        .ok_or_else(|| "--scale requires a positive number.".to_string())?,
                )?;
            }
            "--width" => {
                index += 1;
                raster.width = Some(parse_positive_u32(
                    "--width",
                    args.get(index)
                        .ok_or_else(|| "--width requires a positive integer.".to_string())?,
                )?);
            }
            "--height" => {
                index += 1;
                raster.height = Some(parse_positive_u32(
                    "--height",
                    args.get(index)
                        .ok_or_else(|| "--height requires a positive integer.".to_string())?,
                )?);
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected context argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "context requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "context requires --target <object:id|molecule:index|node:id|bond:id|all> or multiple targets via repeated --target or --targets.".to_string()
    })?;
    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let target_bounds = target_bounds(&document, &target)?;
    let query_view_box = expanded_view_box(target_bounds, expansion);
    let query_bounds = view_box_to_bounds(query_view_box);
    let mut report = context_report(
        &input,
        &document,
        &target,
        target_bounds,
        query_bounds,
        expansion,
        limit,
    )?;

    if let Some(capture_output) = capture_output.as_deref() {
        let format = capture_format
            .or_else(|| infer_capture_format_from_path(capture_output))
            .ok_or_else(|| {
                "--capture-out format is ambiguous; use .svg/.png or --format svg|png.".to_string()
            })?;
        let render = capture_render_primitives(&document, &target, query_view_box);
        let render_output = write_capture_output(
            &render.primitives,
            query_view_box,
            capture_output,
            format,
            raster,
        )?;
        let primitive_count = render.primitives.len();
        set_object_field(
            &mut report,
            "capture",
            json!({
                "ok": true,
                "path": capture_output,
                "format": format.as_str(),
                "verified": true,
                "bytes": render_output.bytes,
                "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
                "viewBox": view_box_json(query_view_box),
                "render": {
                    "mode": render.mode,
                    "primitiveCount": primitive_count,
                    "targets": render.targets.to_json(),
                },
            }),
        );
    }

    write_json_value(report, output.as_deref(), pretty)
}

pub(crate) fn detail_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut include_raw = true;
    let mut include_resource = false;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" => {
                index += 1;
                target = Some(parse_target_selector(
                    args.get(index)
                        .ok_or_else(|| "--target requires a selector.".to_string())?,
                )?);
            }
            "--object" => {
                index += 1;
                target = Some(TargetSelector::Object(
                    args.get(index)
                        .ok_or_else(|| "--object requires an object id.".to_string())?
                        .clone(),
                ));
            }
            "--molecule" => {
                index += 1;
                target = Some(TargetSelector::Molecule(parse_usize_arg(
                    "--molecule",
                    args.get(index),
                )?));
            }
            "--node" => {
                index += 1;
                target = Some(TargetSelector::Node(
                    args.get(index)
                        .ok_or_else(|| "--node requires a node id.".to_string())?
                        .clone(),
                ));
            }
            "--bond" => {
                index += 1;
                target = Some(TargetSelector::Bond(
                    args.get(index)
                        .ok_or_else(|| "--bond requires a bond id.".to_string())?
                        .clone(),
                ));
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--summary-only" | "--no-raw" => include_raw = false,
            "--raw" => include_raw = true,
            "--include-resource" => include_resource = true,
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value if target.is_none() => target = Some(parse_target_selector(value)?),
            value => return Err(format!("Unexpected detail argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "detail requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "detail requires --target <object:id|molecule:index|node:id|bond:id>.".to_string()
    })?;
    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let report = detail_report(
        &input,
        &document,
        &target,
        DetailOptions {
            include_raw,
            include_resource,
        },
    )?;
    write_json_value(report, output.as_deref(), pretty)
}

pub(crate) fn parse_target_selector(value: &str) -> Result<TargetSelector, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("all") {
        return Ok(TargetSelector::All);
    }
    if value.contains(';') {
        return parse_target_selection_arg(value);
    }
    let Some((kind, id)) = value.split_once(':') else {
        return Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, bond:<id>, bounds:<minX,minY,maxX,maxY>, or selection:<selector;selector>."
        ));
    };
    let id = id.trim();
    if id.is_empty() {
        return Err(format!(
            "Invalid target selector '{value}': target id is empty."
        ));
    }
    match kind.trim().to_ascii_lowercase().as_str() {
        "object" | "obj" => Ok(TargetSelector::Object(id.to_string())),
        "molecule" | "mol" => id
            .parse::<usize>()
            .map(TargetSelector::Molecule)
            .map_err(|_| format!("Invalid molecule target '{value}': molecule index must be a non-negative integer.")),
        "node" | "atom" => Ok(TargetSelector::Node(id.to_string())),
        "bond" => Ok(TargetSelector::Bond(id.to_string())),
        "bounds" => parse_bounds_arg(id).map(TargetSelector::Bounds),
        "selection" | "targets" => parse_target_selection_arg(id),
        _ => Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, bond:<id>, bounds:<minX,minY,maxX,maxY>, or selection:<selector;selector>."
        )),
    }
}

fn parse_target_selection_arg(value: &str) -> Result<TargetSelector, String> {
    let mut targets = Vec::new();
    for part in value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        collect_selection_targets(parse_target_selector(part)?, &mut targets);
    }
    target_from_selection_targets(targets)
}

fn parse_usize_arg(name: &str, value: Option<&String>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{name} requires a non-negative integer."))?
        .parse::<usize>()
        .map_err(|_| format!("{name} requires a non-negative integer."))
}

fn parse_non_negative_f64(name: &str, value: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a number."))?;
    if !number.is_finite() || number < 0.0 {
        return Err(format!("{name} requires a finite non-negative number."));
    }
    Ok(number)
}

fn parse_positive_f64(name: &str, value: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a positive number."))?;
    if !number.is_finite() || number <= 0.0 {
        return Err(format!("{name} requires a finite positive number."));
    }
    Ok(number)
}

fn parse_positive_u32(name: &str, value: &str) -> Result<u32, String> {
    let number = value
        .parse::<u32>()
        .map_err(|_| format!("{name} requires a positive integer."))?;
    if number == 0 {
        return Err(format!("{name} requires a positive integer."));
    }
    Ok(number)
}

fn parse_capture_format(value: &str) -> Result<CaptureFormat, String> {
    match value
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "svg" => Ok(CaptureFormat::Svg),
        "png" => Ok(CaptureFormat::Png),
        _ => Err(format!(
            "Unsupported capture format '{value}'. Expected svg or png."
        )),
    }
}

fn infer_capture_format_from_path(path: &str) -> Option<CaptureFormat> {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| parse_capture_format(extension).ok())
}

fn context_report(
    input: &str,
    document: &ChemcoreDocument,
    target: &TargetSelector,
    target_box: [f64; 4],
    query_bounds: [f64; 4],
    expansion: CropExpansion,
    limit: usize,
) -> Result<Value, String> {
    let object_infos = collect_scene_object_infos(document);
    let mut objects = object_infos
        .iter()
        .filter(|info| bounds_intersect(info.bounds, query_bounds))
        .map(|info| {
            json!({
                "selector": format!("object:{}", info.id),
                "kind": "object",
                "id": info.id,
                "type": info.object_type,
                "name": info.name,
                "visible": info.visible,
                "bounds": bounds_json(info.bounds),
                "spatial": spatial_relation_json(target_box, info.bounds),
                "selectionBoxRelation": selection_box_relation(target_box, info.bounds),
                "relationships": object_relationship_json(info),
                "isTarget": target_matches_object(target, info),
            })
        })
        .collect::<Vec<_>>();

    let mut molecules = document
        .editable_fragments()
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let bounds = target_bounds_fast(document, &TargetSelector::Molecule(index))
                .or_else(|| target_bounds(document, &TargetSelector::Molecule(index)).ok())?;
            bounds_intersect(bounds, query_bounds).then(|| {
                json!({
                    "selector": format!("molecule:{index}"),
                    "kind": "molecule",
                    "index": index,
                    "objectId": entry.object.id,
                    "resourceRef": entry.object.payload.resource_ref,
                    "nodeCount": entry.fragment.nodes.len(),
                    "bondCount": entry.fragment.bonds.len(),
                    "bounds": bounds_json(bounds),
                    "spatial": spatial_relation_json(target_box, bounds),
                    "selectionBoxRelation": selection_box_relation(target_box, bounds),
                    "isTarget": target_matches_molecule(target, index),
                })
            })
        })
        .collect::<Vec<_>>();

    let mut nodes = Vec::new();
    let mut bonds = Vec::new();
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        for node in &entry.fragment.nodes {
            let bounds = node_fast_bounds(entry.object, node);
            if bounds_intersect(bounds, query_bounds) {
                nodes.push(json!({
                    "selector": format!("node:{}", node.id),
                    "kind": "node",
                    "id": node.id,
                    "moleculeIndex": molecule_index,
                    "objectId": entry.object.id,
                    "element": node.element,
                    "atomicNumber": node.atomic_number,
                    "bounds": bounds_json(bounds),
                    "spatial": spatial_relation_json(target_box, bounds),
                    "selectionBoxRelation": selection_box_relation(target_box, bounds),
                    "isTarget": target_matches_node(target, &node.id),
                }));
            }
        }
        for bond in &entry.fragment.bonds {
            let Some(bounds) = bond_fast_bounds(entry.object, &entry.fragment.nodes, bond) else {
                continue;
            };
            if bounds_intersect(bounds, query_bounds) {
                bonds.push(json!({
                    "selector": format!("bond:{}", bond.id),
                    "kind": "bond",
                    "id": bond.id,
                    "moleculeIndex": molecule_index,
                    "objectId": entry.object.id,
                    "begin": bond.begin,
                    "end": bond.end,
                    "order": bond.order,
                    "bounds": bounds_json(bounds),
                    "spatial": spatial_relation_json(target_box, bounds),
                    "selectionBoxRelation": selection_box_relation(target_box, bounds),
                    "isTarget": target_matches_bond(target, &bond.id),
                }));
            }
        }
    }
    let selection_box =
        selection_box_summary(target_box, &objects, &molecules, &nodes, &bonds, limit);
    sort_context_entries(&mut objects);
    sort_context_entries(&mut molecules);
    sort_context_entries(&mut nodes);
    sort_context_entries(&mut bonds);
    objects.truncate(limit);
    molecules.truncate(limit);
    nodes.truncate(limit);
    bonds.truncate(limit);

    Ok(json!({
        "ok": true,
        "input": input,
        "target": target.to_json(),
        "bounds": {
            "target": bounds_json(target_box),
            "query": bounds_json(query_bounds),
        },
        "selectionBox": selection_box,
        "expansion": expansion.to_json(),
        "counts": {
            "objects": objects.len(),
            "molecules": molecules.len(),
            "nodes": nodes.len(),
            "bonds": bonds.len(),
            "limit": limit,
        },
        "relationships": target_relationships_json(target, &object_infos),
        "context": {
            "objects": objects,
            "molecules": molecules,
            "nodes": nodes,
            "bonds": bonds,
        }
    }))
}

fn detail_report(
    input: &str,
    document: &ChemcoreDocument,
    target: &TargetSelector,
    options: DetailOptions,
) -> Result<Value, String> {
    let object_infos = collect_scene_object_infos(document);
    let detail = match target {
        TargetSelector::Object(id) => object_detail_json(document, &object_infos, id, options)?,
        TargetSelector::Molecule(index) => {
            molecule_detail_json(document, &object_infos, *index, options)?
        }
        TargetSelector::Node(id) => node_detail_json(document, id, options)?,
        TargetSelector::Bond(id) => bond_detail_json(document, id, options)?,
        TargetSelector::All | TargetSelector::Bounds(_) | TargetSelector::Selection(_) => {
            return Err(
                "detail requires object:<id>, molecule:<index>, node:<id>, or bond:<id>. Use inspect for whole-document JSON."
                    .to_string(),
            );
        }
    };
    Ok(json!({
        "ok": true,
        "input": input,
        "target": target.to_json(),
        "detail": detail,
    }))
}

fn object_detail_json(
    document: &ChemcoreDocument,
    object_infos: &[SceneObjectInfo],
    id: &str,
    options: DetailOptions,
) -> Result<Value, String> {
    let object = document
        .find_scene_object(id)
        .ok_or_else(|| format!("Object target was not found: {id}."))?;
    let info = object_infos.iter().find(|info| info.id == id);
    let mut detail = json!({
        "selector": format!("object:{id}"),
        "kind": "object",
        "id": id,
        "type": object.object_type,
        "name": object.name,
        "visible": object.visible,
        "locked": object.locked,
        "zIndex": object.z_index,
        "styleRef": object.style_ref,
        "resourceRef": object.payload.resource_ref,
        "childCount": object.children.len(),
        "bounds": optional_bounds_json(
            target_bounds_fast(document, &TargetSelector::Object(id.to_string()))
                .or_else(|| target_bounds(document, &TargetSelector::Object(id.to_string())).ok())
        ),
        "relationships": info.map(object_relationship_json).unwrap_or(Value::Null),
        "references": object_references_json(document, object),
    });
    if options.include_raw {
        set_object_field(
            &mut detail,
            "raw",
            object_raw_json(document, object, options.include_resource),
        );
    }
    Ok(detail)
}

fn molecule_detail_json(
    document: &ChemcoreDocument,
    object_infos: &[SceneObjectInfo],
    index: usize,
    options: DetailOptions,
) -> Result<Value, String> {
    let fragments = document.editable_fragments();
    let entry = fragments
        .get(index)
        .ok_or_else(|| format!("Molecule index {index} was not found."))?;
    let object_id = entry.object.id.clone();
    let info = object_infos.iter().find(|info| info.id == object_id);
    let mut detail = json!({
        "selector": format!("molecule:{index}"),
        "kind": "molecule",
        "index": index,
        "objectId": entry.object.id,
        "resourceRef": entry.object.payload.resource_ref,
        "nodeCount": entry.fragment.nodes.len(),
        "bondCount": entry.fragment.bonds.len(),
        "fragmentBbox": entry.fragment.bbox,
        "bounds": optional_bounds_json(
            target_bounds_fast(document, &TargetSelector::Molecule(index))
                .or_else(|| target_bounds(document, &TargetSelector::Molecule(index)).ok())
        ),
        "relationships": info.map(object_relationship_json).unwrap_or(Value::Null),
        "references": object_references_json(document, entry.object),
    });
    if options.include_raw {
        let mut raw = Map::new();
        raw.insert("object".to_string(), json!(entry.object));
        raw.insert("fragment".to_string(), json!(entry.fragment));
        if options.include_resource {
            insert_referenced_resource_raw(&mut raw, document, entry.object);
        }
        set_object_field(&mut detail, "raw", Value::Object(raw));
    }
    Ok(detail)
}

fn node_detail_json(
    document: &ChemcoreDocument,
    id: &str,
    options: DetailOptions,
) -> Result<Value, String> {
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == id) else {
            continue;
        };
        let connected_bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == id || bond.end == id)
            .collect::<Vec<_>>();
        let mut detail = json!({
            "selector": format!("node:{id}"),
            "kind": "node",
            "id": id,
            "moleculeIndex": molecule_index,
            "objectId": entry.object.id,
            "resourceRef": entry.object.payload.resource_ref,
            "element": node.element,
            "atomicNumber": node.atomic_number,
            "position": node.position,
            "charge": node.charge,
            "numHydrogens": node.num_hydrogens,
            "labelText": node.label.as_ref().map(|label| label.text.clone()),
            "connectedBondIds": connected_bonds.iter().map(|bond| bond.id.clone()).collect::<Vec<_>>(),
            "bounds": bounds_json(node_fast_bounds(entry.object, node)),
            "references": object_references_json(document, entry.object),
        });
        if options.include_raw {
            let mut raw = Map::new();
            raw.insert("node".to_string(), json!(node));
            if options.include_resource {
                insert_referenced_resource_raw(&mut raw, document, entry.object);
            }
            set_object_field(&mut detail, "raw", Value::Object(raw));
        }
        return Ok(detail);
    }
    Err(format!("Node target was not found: {id}."))
}

fn bond_detail_json(
    document: &ChemcoreDocument,
    id: &str,
    options: DetailOptions,
) -> Result<Value, String> {
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == id) else {
            continue;
        };
        let begin_node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin);
        let end_node = entry.fragment.nodes.iter().find(|node| node.id == bond.end);
        let mut detail = json!({
            "selector": format!("bond:{id}"),
            "kind": "bond",
            "id": id,
            "moleculeIndex": molecule_index,
            "objectId": entry.object.id,
            "resourceRef": entry.object.payload.resource_ref,
            "begin": bond.begin,
            "end": bond.end,
            "order": bond.order,
            "stereo": bond.stereo,
            "lineStyles": bond.line_styles,
            "bounds": optional_bounds_json(bond_fast_bounds(entry.object, &entry.fragment.nodes, bond)),
            "endpoints": {
                "begin": begin_node.map(node_endpoint_summary_json),
                "end": end_node.map(node_endpoint_summary_json),
            },
            "references": object_references_json(document, entry.object),
        });
        if options.include_raw {
            let mut raw = Map::new();
            raw.insert("bond".to_string(), json!(bond));
            if options.include_resource {
                insert_referenced_resource_raw(&mut raw, document, entry.object);
            }
            set_object_field(&mut detail, "raw", Value::Object(raw));
        }
        return Ok(detail);
    }
    Err(format!("Bond target was not found: {id}."))
}

fn optional_bounds_json(bounds: Option<[f64; 4]>) -> Value {
    bounds.map(bounds_json).unwrap_or(Value::Null)
}

fn node_endpoint_summary_json(node: &Node) -> Value {
    json!({
        "id": node.id,
        "element": node.element,
        "atomicNumber": node.atomic_number,
        "position": node.position,
        "charge": node.charge,
        "labelText": node.label.as_ref().map(|label| label.text.clone()),
    })
}

fn object_references_json(document: &ChemcoreDocument, object: &SceneObject) -> Value {
    json!({
        "style": object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref).map(|style| style_summary_json(style_ref, style))),
        "resource": object
            .payload
            .resource_ref
            .as_ref()
            .and_then(|resource_ref| document.resources.get(resource_ref).map(|resource| resource_summary_json(resource_ref, resource))),
    })
}

fn style_summary_json(id: &str, style: &Value) -> Value {
    json!({
        "id": id,
        "kind": style.get("kind").and_then(Value::as_str),
        "stroke": style.get("stroke").and_then(Value::as_str),
        "fill": style.get("fill").cloned(),
        "strokeWidth": style.get("strokeWidth").and_then(Value::as_f64),
        "fontFamily": style.get("fontFamily").and_then(Value::as_str),
        "fontSize": style.get("fontSize").and_then(Value::as_f64),
    })
}

fn resource_summary_json(id: &str, resource: &chemcore_engine::Resource) -> Value {
    let mut summary = json!({
        "id": id,
        "type": resource.resource_type,
        "encoding": resource.encoding,
    });
    match &resource.data {
        ResourceData::Fragment(fragment) => {
            set_object_field(&mut summary, "kind", json!("fragment"));
            set_object_field(&mut summary, "nodeCount", json!(fragment.nodes.len()));
            set_object_field(&mut summary, "bondCount", json!(fragment.bonds.len()));
            set_object_field(&mut summary, "bbox", json!(fragment.bbox));
        }
        ResourceData::Text(text) => {
            set_object_field(&mut summary, "kind", json!("text"));
            set_object_field(&mut summary, "textLength", json!(text.len()));
        }
        ResourceData::Json(value) => {
            set_object_field(&mut summary, "kind", json!("json"));
            set_object_field(&mut summary, "jsonType", json!(json_value_kind(value)));
        }
    }
    summary
}

fn object_raw_json(
    document: &ChemcoreDocument,
    object: &SceneObject,
    include_resource: bool,
) -> Value {
    let mut raw = Map::new();
    raw.insert("object".to_string(), json!(object));
    if include_resource {
        insert_referenced_resource_raw(&mut raw, document, object);
    }
    Value::Object(raw)
}

fn insert_referenced_resource_raw(
    raw: &mut Map<String, Value>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    raw.insert(
        "resource".to_string(),
        json!({
            "id": resource_ref,
            "value": resource,
        }),
    );
}

fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[derive(Debug, Clone)]
struct SceneObjectInfo {
    id: String,
    object_type: String,
    name: String,
    visible: bool,
    bounds: [f64; 4],
    parent_id: Option<String>,
    ancestor_ids: Vec<String>,
    child_ids: Vec<String>,
    linked_object_ids: Vec<String>,
    link_kind: Option<String>,
    group_kind: Option<String>,
}

fn collect_scene_object_infos(document: &ChemcoreDocument) -> Vec<SceneObjectInfo> {
    let mut out = Vec::new();
    collect_scene_object_infos_inner(document, &document.objects, None, &[], &mut out);
    out
}

fn collect_scene_object_infos_inner(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    parent_id: Option<&str>,
    ancestors: &[String],
    out: &mut Vec<SceneObjectInfo>,
) {
    for object in objects {
        let mut next_ancestors = ancestors.to_vec();
        if let Some(parent_id) = parent_id {
            next_ancestors.push(parent_id.to_string());
        }
        if let Some(bounds) =
            target_bounds_fast(document, &TargetSelector::Object(object.id.clone())).or_else(|| {
                target_bounds(document, &TargetSelector::Object(object.id.clone())).ok()
            })
        {
            out.push(SceneObjectInfo {
                id: object.id.clone(),
                object_type: object.object_type.clone(),
                name: object.name.clone(),
                visible: object.visible,
                bounds,
                parent_id: parent_id.map(str::to_string),
                ancestor_ids: next_ancestors.clone(),
                child_ids: object
                    .children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect(),
                linked_object_ids: linked_object_ids(object),
                link_kind: object
                    .meta
                    .get("linkKind")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                group_kind: object
                    .meta
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        collect_scene_object_infos_inner(
            document,
            &object.children,
            Some(object.id.as_str()),
            &next_ancestors,
            out,
        );
    }
}

fn linked_object_ids(object: &SceneObject) -> Vec<String> {
    [
        "linkedTextObjectId",
        "linkedBracketObjectId",
        "bracketLabelTextObjectId",
        "bracketObjectId",
    ]
    .into_iter()
    .filter_map(|key| {
        object
            .meta
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    })
    .collect()
}

fn object_relationship_json(info: &SceneObjectInfo) -> Value {
    json!({
        "parentId": info.parent_id,
        "ancestorIds": info.ancestor_ids,
        "childIds": info.child_ids,
        "isGroup": info.object_type == "group",
        "groupKind": info.group_kind,
        "linkedObjectIds": info.linked_object_ids,
        "linkKind": info.link_kind,
    })
}

fn target_relationships_json(target: &TargetSelector, infos: &[SceneObjectInfo]) -> Value {
    match target {
        TargetSelector::Object(id) => infos
            .iter()
            .find(|info| &info.id == id)
            .map(object_relationship_json)
            .unwrap_or(Value::Null),
        TargetSelector::Molecule(index) => json!({
            "moleculeIndex": index,
        }),
        TargetSelector::Node(id) => json!({
            "nodeId": id,
        }),
        TargetSelector::Bond(id) => json!({
            "bondId": id,
        }),
        TargetSelector::Selection(targets) => json!({
            "targets": targets
                .iter()
                .map(|target| json!({
                    "target": target.to_json(),
                    "relationships": target_relationships_json(target, infos),
                }))
                .collect::<Vec<_>>(),
        }),
        TargetSelector::All | TargetSelector::Bounds(_) => Value::Null,
    }
}

fn target_matches_object(target: &TargetSelector, info: &SceneObjectInfo) -> bool {
    match target {
        TargetSelector::Object(id) => id == &info.id,
        TargetSelector::Selection(targets) => targets
            .iter()
            .any(|target| target_matches_object(target, info)),
        _ => false,
    }
}

fn target_matches_molecule(target: &TargetSelector, index: usize) -> bool {
    match target {
        TargetSelector::Molecule(target_index) => *target_index == index,
        TargetSelector::Selection(targets) => targets
            .iter()
            .any(|target| target_matches_molecule(target, index)),
        _ => false,
    }
}

fn target_matches_node(target: &TargetSelector, id: &str) -> bool {
    match target {
        TargetSelector::Node(target_id) => target_id == id,
        TargetSelector::Selection(targets) => {
            targets.iter().any(|target| target_matches_node(target, id))
        }
        _ => false,
    }
}

fn target_matches_bond(target: &TargetSelector, id: &str) -> bool {
    match target {
        TargetSelector::Bond(target_id) => target_id == id,
        TargetSelector::Selection(targets) => {
            targets.iter().any(|target| target_matches_bond(target, id))
        }
        _ => false,
    }
}

fn selection_box_relation(target_box: [f64; 4], bounds: [f64; 4]) -> &'static str {
    if bounds_contains(target_box, bounds) {
        "inside"
    } else if bounds_intersect(target_box, bounds) {
        "partial"
    } else {
        "outside"
    }
}

fn selection_box_summary(
    target_box: [f64; 4],
    objects: &[Value],
    molecules: &[Value],
    nodes: &[Value],
    bonds: &[Value],
    limit: usize,
) -> Value {
    json!({
        "bounds": bounds_json(target_box),
        "contents": {
            "objects": selection_box_entries(objects, limit),
            "molecules": selection_box_entries(molecules, limit),
            "nodes": selection_box_entries(nodes, limit),
            "bonds": selection_box_entries(bonds, limit),
        }
    })
}

fn selection_box_entries(entries: &[Value], limit: usize) -> Value {
    let mut count = 0usize;
    let mut items = Vec::new();
    for entry in entries {
        let relation = entry
            .get("selectionBoxRelation")
            .and_then(Value::as_str)
            .unwrap_or("outside");
        if relation == "outside" {
            continue;
        }
        count += 1;
        if items.len() < limit {
            items.push(selection_box_entry_summary(entry));
        }
    }
    json!({
        "count": count,
        "truncated": count > items.len(),
        "items": items,
    })
}

fn selection_box_entry_summary(entry: &Value) -> Value {
    let mut summary = Map::new();
    for key in [
        "selector",
        "kind",
        "id",
        "index",
        "objectId",
        "type",
        "name",
        "bounds",
        "selectionBoxRelation",
        "isTarget",
    ] {
        if let Some(value) = entry.get(key) {
            summary.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(summary)
}

fn spatial_relation_json(target: [f64; 4], other: [f64; 4]) -> Value {
    let target_center = bounds_center(target);
    let other_center = bounds_center(other);
    let dx = other_center[0] - target_center[0];
    let dy = other_center[1] - target_center[1];
    let gap_x = axis_gap(target[0], target[2], other[0], other[2]);
    let gap_y = axis_gap(target[1], target[3], other[1], other[3]);
    let edge_gap = (gap_x * gap_x + gap_y * gap_y).sqrt();
    json!({
        "direction": direction_for_delta(dx, dy, gap_x, gap_y),
        "centerDelta": { "x": dx, "y": dy },
        "centerDistance": (dx * dx + dy * dy).sqrt(),
        "edgeGap": edge_gap,
        "overlapsTarget": bounds_intersect(target, other),
        "containsTarget": bounds_contains(other, target),
        "insideTarget": bounds_contains(target, other),
    })
}

fn sort_context_entries(entries: &mut [Value]) {
    entries.sort_by(|left, right| {
        let left_gap = left
            .pointer("/spatial/edgeGap")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        let right_gap = right
            .pointer("/spatial/edgeGap")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        left_gap
            .partial_cmp(&right_gap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn bounds_center(bounds: [f64; 4]) -> [f64; 2] {
    [(bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5]
}

fn axis_gap(a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> f64 {
    if a_max < b_min {
        b_min - a_max
    } else if b_max < a_min {
        a_min - b_max
    } else {
        0.0
    }
}

fn direction_for_delta(dx: f64, dy: f64, gap_x: f64, gap_y: f64) -> &'static str {
    if gap_x == 0.0 && gap_y == 0.0 {
        return "overlap";
    }
    if gap_x >= gap_y {
        if dx < 0.0 {
            "left"
        } else {
            "right"
        }
    } else if dy < 0.0 {
        "above"
    } else {
        "below"
    }
}

fn bounds_intersect(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

fn bounds_contains(outer: [f64; 4], inner: [f64; 4]) -> bool {
    outer[0] <= inner[0] && outer[1] <= inner[1] && outer[2] >= inner[2] && outer[3] >= inner[3]
}

fn view_box_to_bounds(view_box: [f64; 4]) -> [f64; 4] {
    [
        view_box[0],
        view_box[1],
        view_box[0] + view_box[2],
        view_box[1] + view_box[3],
    ]
}

fn capture_render_primitives(
    document: &ChemcoreDocument,
    target: &TargetSelector,
    view_box: [f64; 4],
) -> CaptureRender {
    if matches!(target, TargetSelector::All) {
        return CaptureRender {
            primitives: render_document(document),
            mode: "full-document",
            targets: RegionRenderTargets::default(),
        };
    }

    let targets = region_render_targets(document, view_box_to_bounds(view_box));
    if targets.is_empty() {
        return CaptureRender {
            primitives: Vec::new(),
            mode: "region-empty",
            targets,
        };
    }

    let primitives =
        render_document_targets(document, &targets.nodes, &targets.bonds, &targets.objects);
    CaptureRender {
        primitives,
        mode: "region-targets",
        targets,
    }
}

fn region_render_targets(
    document: &ChemcoreDocument,
    query_bounds: [f64; 4],
) -> RegionRenderTargets {
    let mut targets = RegionRenderTargets::default();
    collect_region_scene_object_targets(document, &document.objects, query_bounds, &mut targets);
    targets
}

fn collect_region_scene_object_targets(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    query_bounds: [f64; 4],
    targets: &mut RegionRenderTargets,
) {
    for object in objects {
        if !object.visible {
            continue;
        }

        if object.object_type == "molecule"
            && collect_region_molecule_targets(document, object, query_bounds, targets)
        {
            continue;
        }

        if object.object_type == "group" {
            collect_region_scene_object_targets(document, &object.children, query_bounds, targets);
            if scene_object_fast_bounds(document, object)
                .is_some_and(|bounds| bounds_intersect(bounds, query_bounds))
            {
                targets.objects.insert(object.id.clone());
            }
            continue;
        }

        let bounds = scene_object_fast_bounds(document, object)
            .or_else(|| target_bounds(document, &TargetSelector::Object(object.id.clone())).ok());
        if bounds.is_some_and(|bounds| bounds_intersect(bounds, query_bounds)) {
            targets.objects.insert(object.id.clone());
        }
    }
}

fn collect_region_molecule_targets(
    document: &ChemcoreDocument,
    object: &SceneObject,
    query_bounds: [f64; 4],
    targets: &mut RegionRenderTargets,
) -> bool {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return false;
    };
    let Some(fragment) = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
    else {
        return false;
    };

    for node in &fragment.nodes {
        if bounds_intersect(node_fast_bounds(object, node), query_bounds) {
            targets.nodes.insert(node.id.clone());
        }
    }
    for bond in &fragment.bonds {
        let Some(bounds) = bond_fast_bounds(object, &fragment.nodes, bond) else {
            continue;
        };
        if bounds_intersect(bounds, query_bounds) {
            targets.bonds.insert(bond.id.clone());
        }
    }
    true
}

fn target_bounds_fast(document: &ChemcoreDocument, target: &TargetSelector) -> Option<[f64; 4]> {
    match target {
        TargetSelector::All => document_fast_bounds(document),
        TargetSelector::Bounds(bounds) => Some(*bounds),
        TargetSelector::Selection(targets) => {
            let mut out = None;
            for target in targets {
                include_bounds(&mut out, target_bounds_fast(document, target)?);
            }
            out
        }
        TargetSelector::Object(id) => document
            .find_scene_object(id)
            .and_then(|object| scene_object_fast_bounds(document, object)),
        TargetSelector::Molecule(index) => {
            let fragments = document.editable_fragments();
            let entry = fragments.get(*index)?;
            molecule_object_fast_bounds(document, entry.object)
        }
        TargetSelector::Node(id) => {
            for entry in document.editable_fragments() {
                if let Some(node) = entry.fragment.nodes.iter().find(|node| &node.id == id) {
                    return Some(node_fast_bounds(entry.object, node));
                }
            }
            None
        }
        TargetSelector::Bond(id) => {
            for entry in document.editable_fragments() {
                if let Some(bond) = entry.fragment.bonds.iter().find(|bond| &bond.id == id) {
                    return bond_fast_bounds(entry.object, &entry.fragment.nodes, bond);
                }
            }
            None
        }
    }
}

fn document_fast_bounds(document: &ChemcoreDocument) -> Option<[f64; 4]> {
    let mut out = None;
    for object in &document.objects {
        if !object.visible {
            continue;
        }
        if let Some(bounds) = scene_object_fast_bounds(document, object) {
            include_bounds(&mut out, bounds);
        }
    }
    out
}

fn scene_object_fast_bounds(document: &ChemcoreDocument, object: &SceneObject) -> Option<[f64; 4]> {
    if !object.visible {
        return None;
    }
    if object.object_type == "group" {
        let mut out = None;
        for child in &object.children {
            if let Some(bounds) = scene_object_fast_bounds(document, child) {
                include_bounds(&mut out, bounds);
            }
        }
        return out.or_else(|| scene_object_bbox_bounds(object));
    }
    if object.object_type == "molecule" {
        return molecule_object_fast_bounds(document, object);
    }
    scene_object_bbox_bounds(object)
}

fn molecule_object_fast_bounds(
    document: &ChemcoreDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    scene_object_bbox_bounds(object).or_else(|| {
        let resource_ref = object.payload.resource_ref.as_ref()?;
        let fragment = document.resources.get(resource_ref)?.data.as_fragment()?;
        local_bbox_world_bounds(object, fragment.bbox)
    })
}

fn scene_object_bbox_bounds(object: &SceneObject) -> Option<[f64; 4]> {
    local_bbox_world_bounds(object, object.payload.bbox?)
}

fn local_bbox_world_bounds(object: &SceneObject, bbox: [f64; 4]) -> Option<[f64; 4]> {
    let [x, y, width, height] = bbox;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let min_x = tx + x;
    let min_y = ty + y;
    let max_x = tx + x + width;
    let max_y = ty + y + height;
    if object.transform.rotate.abs() <= f64::EPSILON {
        return Some([min_x, min_y, max_x, max_y]);
    }

    let center = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
    let mut bounds = rotate_point_bounds([min_x, min_y], center, object.transform.rotate);
    for point in [[max_x, min_y], [max_x, max_y], [min_x, max_y]] {
        let rotated = rotate_point_bounds(point, center, object.transform.rotate);
        bounds[0] = bounds[0].min(rotated[0]);
        bounds[1] = bounds[1].min(rotated[1]);
        bounds[2] = bounds[2].max(rotated[2]);
        bounds[3] = bounds[3].max(rotated[3]);
    }
    Some(bounds)
}

fn include_bounds(out: &mut Option<[f64; 4]>, bounds: [f64; 4]) {
    *out = Some(match *out {
        Some(current) => [
            current[0].min(bounds[0]),
            current[1].min(bounds[1]),
            current[2].max(bounds[2]),
            current[3].max(bounds[3]),
        ],
        None => bounds,
    });
}

fn rotate_point_bounds(point: [f64; 2], center: [f64; 2], degrees: f64) -> [f64; 4] {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point[0] - center[0];
    let dy = point[1] - center[1];
    let x = center[0] + dx * cos - dy * sin;
    let y = center[1] + dx * sin + dy * cos;
    [x, y, x, y]
}

fn set_object_field(value: &mut Value, key: &str, field: Value) {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_string(), field);
    }
}

fn parse_bounds_arg(value: &str) -> Result<[f64; 4], String> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("Bounds must use minX,minY,maxX,maxY.".to_string());
    }
    let mut numbers = [0.0; 4];
    for (index, part) in parts.iter().enumerate() {
        numbers[index] = part
            .parse::<f64>()
            .map_err(|_| "Bounds values must be finite numbers.".to_string())?;
        if !numbers[index].is_finite() {
            return Err("Bounds values must be finite numbers.".to_string());
        }
    }
    if numbers[2] <= numbers[0] || numbers[3] <= numbers[1] {
        return Err("Bounds must satisfy maxX > minX and maxY > minY.".to_string());
    }
    Ok(numbers)
}

fn engine_document(engine: &Engine) -> Result<ChemcoreDocument, String> {
    serde_json::from_str(&document_json(engine)?)
        .map_err(|error| format!("Failed to parse engine document JSON: {error}"))
}

fn object_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    collect_object_target_entries(document, &document.objects, None, 0, &mut entries);
    entries
}

fn collect_object_target_entries(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    parent_id: Option<&str>,
    depth: usize,
    entries: &mut Vec<Value>,
) {
    for object in objects {
        let bounds = target_bounds_fast(document, &TargetSelector::Object(object.id.clone()))
            .or_else(|| target_bounds(document, &TargetSelector::Object(object.id.clone())).ok())
            .map(bounds_json);
        entries.push(json!({
            "selector": format!("object:{}", object.id),
            "id": object.id,
            "type": object.object_type,
            "name": object.name,
            "visible": object.visible,
            "locked": object.locked,
            "zIndex": object.z_index,
            "parentId": parent_id,
            "depth": depth,
            "resourceRef": object.payload.resource_ref,
            "children": object.children.len(),
            "bounds": bounds,
        }));
        collect_object_target_entries(
            document,
            &object.children,
            Some(object.id.as_str()),
            depth + 1,
            entries,
        );
    }
}

fn molecule_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    document
        .editable_fragments()
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let bounds = target_bounds_fast(document, &TargetSelector::Molecule(index))
                .or_else(|| target_bounds(document, &TargetSelector::Molecule(index)).ok())
                .map(bounds_json);
            json!({
                "selector": format!("molecule:{index}"),
                "index": index,
                "objectId": entry.object.id,
                "resourceRef": entry.object.payload.resource_ref,
                "nodeCount": entry.fragment.nodes.len(),
                "bondCount": entry.fragment.bonds.len(),
                "bounds": bounds,
            })
        })
        .collect()
}

fn node_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        for node in &entry.fragment.nodes {
            let position = world_node_position(entry.object, node);
            entries.push(json!({
                "selector": format!("node:{}", node.id),
                "id": node.id,
                "moleculeIndex": molecule_index,
                "objectId": entry.object.id,
                "element": node.element,
                "atomicNumber": node.atomic_number,
                "position": [position[0], position[1]],
                "hasLabel": node.label.as_ref().is_some_and(|label| label.has_visible_text()),
                "bounds": bounds_json(node_fast_bounds(entry.object, node)),
                "boundsSource": "geometry-fast",
            }));
        }
    }
    entries
}

fn bond_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        for bond in &entry.fragment.bonds {
            entries.push(json!({
                "selector": format!("bond:{}", bond.id),
                "id": bond.id,
                "moleculeIndex": molecule_index,
                "objectId": entry.object.id,
                "begin": bond.begin,
                "end": bond.end,
                "order": bond.order,
                "bounds": bond_fast_bounds(entry.object, &entry.fragment.nodes, bond).map(bounds_json),
                "boundsSource": "geometry-fast",
            }));
        }
    }
    entries
}

fn world_node_position(object: &SceneObject, node: &Node) -> [f64; 2] {
    [
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    ]
}

fn node_fast_bounds(object: &SceneObject, node: &Node) -> [f64; 4] {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    if let Some(bounds) = node.label.as_ref().and_then(|label| label.bbox()) {
        return [
            bounds[0] + tx,
            bounds[1] + ty,
            bounds[2] + tx,
            bounds[3] + ty,
        ];
    }
    let point = world_node_position(object, node);
    [
        point[0] - 4.0,
        point[1] - 4.0,
        point[0] + 4.0,
        point[1] + 4.0,
    ]
}

fn bond_fast_bounds(object: &SceneObject, nodes: &[Node], bond: &Bond) -> Option<[f64; 4]> {
    let begin = nodes.iter().find(|node| node.id == bond.begin)?;
    let end = nodes.iter().find(|node| node.id == bond.end)?;
    let begin = world_node_position(object, begin);
    let end = world_node_position(object, end);
    Some([
        begin[0].min(end[0]) - 4.0,
        begin[1].min(end[1]) - 4.0,
        begin[0].max(end[0]) + 4.0,
        begin[1].max(end[1]) + 4.0,
    ])
}

fn target_bounds(document: &ChemcoreDocument, target: &TargetSelector) -> Result<[f64; 4], String> {
    if let TargetSelector::Bounds(bounds) = target {
        return Ok(*bounds);
    }
    if let TargetSelector::Selection(targets) = target {
        let mut out = None;
        for target in targets {
            include_bounds(&mut out, target_bounds(document, target)?);
        }
        return out.ok_or_else(|| "Selection target has no members.".to_string());
    }
    if let Some(bounds) = target_bounds_fast(document, target) {
        return Ok(bounds);
    }
    let primitives = render_primitives_for_target(document, target)?;
    render_primitives_bounds(primitives.iter()).ok_or_else(|| {
        format!(
            "No visible render primitives found for target '{}'.",
            target.selector()
        )
    })
}

fn render_primitives_for_target(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Result<Vec<RenderPrimitive>, String> {
    match target {
        TargetSelector::All => Ok(render_document(document)),
        TargetSelector::Bounds(_) => Ok(render_document(document)),
        TargetSelector::Selection(targets) => {
            let mut primitives = Vec::new();
            for target in targets {
                primitives.extend(render_primitives_for_target(document, target)?);
            }
            Ok(primitives)
        }
        TargetSelector::Object(id) => {
            if document.find_scene_object(id).is_none() {
                return Err(format!("Object target not found: {id}. Run 'chemcore-cli targets <input>' to list valid selectors."));
            }
            let nodes = BTreeSet::new();
            let bonds = BTreeSet::new();
            let mut objects = BTreeSet::new();
            objects.insert(id.clone());
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
        TargetSelector::Molecule(index) => {
            let object_id = molecule_object_id(document, *index)?;
            let nodes = BTreeSet::new();
            let bonds = BTreeSet::new();
            let mut objects = BTreeSet::new();
            objects.insert(object_id);
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
        TargetSelector::Node(id) => {
            if !node_exists(document, id) {
                return Err(format!("Node target not found: {id}. Run 'chemcore-cli targets <input>' to list valid selectors."));
            }
            let mut nodes = BTreeSet::new();
            let bonds = BTreeSet::new();
            let objects = BTreeSet::new();
            nodes.insert(id.clone());
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
        TargetSelector::Bond(id) => {
            if !bond_exists(document, id) {
                return Err(format!("Bond target not found: {id}. Run 'chemcore-cli targets <input>' to list valid selectors."));
            }
            let nodes = BTreeSet::new();
            let mut bonds = BTreeSet::new();
            let objects = BTreeSet::new();
            bonds.insert(id.clone());
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
    }
}

fn molecule_object_id(document: &ChemcoreDocument, index: usize) -> Result<String, String> {
    let fragments = document.editable_fragments();
    fragments
        .get(index)
        .map(|entry| entry.object.id.clone())
        .ok_or_else(|| {
            format!(
                "Molecule target not found: molecule:{index}. Document has {} molecule target(s).",
                fragments.len()
            )
        })
}

fn node_exists(document: &ChemcoreDocument, node_id: &str) -> bool {
    document
        .editable_fragments()
        .into_iter()
        .any(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
}

fn bond_exists(document: &ChemcoreDocument, bond_id: &str) -> bool {
    document
        .editable_fragments()
        .into_iter()
        .any(|entry| entry.fragment.bonds.iter().any(|bond| bond.id == bond_id))
}

fn expanded_view_box(bounds: [f64; 4], expansion: CropExpansion) -> [f64; 4] {
    let width = (bounds[2] - bounds[0]).max(1.0);
    let height = (bounds[3] - bounds[1]).max(1.0);
    let left = expansion.left_for(width);
    let right = expansion.right_for(width);
    let top = expansion.top_for(height);
    let bottom = expansion.bottom_for(height);
    let min_x = bounds[0] - left;
    let min_y = bounds[1] - top;
    let width = (width + left + right).max(1.0);
    let height = (height + top + bottom).max(1.0);
    [min_x, min_y, width, height]
}

fn bounds_json(bounds: [f64; 4]) -> Value {
    json!({
        "minX": bounds[0],
        "minY": bounds[1],
        "maxX": bounds[2],
        "maxY": bounds[3],
        "width": bounds[2] - bounds[0],
        "height": bounds[3] - bounds[1],
    })
}

fn view_box_json(view_box: [f64; 4]) -> Value {
    json!({
        "x": view_box[0],
        "y": view_box[1],
        "width": view_box[2],
        "height": view_box[3],
        "value": view_box,
    })
}

struct CaptureWriteResult {
    pixel_size: Option<PixelSize>,
    bytes: u64,
}

fn resolve_capture_output(
    output: Option<String>,
    format: Option<CaptureFormat>,
) -> Result<(String, CaptureFormat, bool), String> {
    if let Some(output) = output {
        if output == "-" {
            return Err(
                "capture writes image data to a file; stdout is reserved for the JSON manifest."
                    .to_string(),
            );
        }
        let format = format
            .or_else(|| infer_capture_format_from_path(&output))
            .ok_or_else(|| {
                "Capture output format is ambiguous; use --out <path.svg|path.png> or --format svg|png."
                    .to_string()
            })?;
        return Ok((output, format, false));
    }

    let format = format.unwrap_or(CaptureFormat::Png);
    Ok((
        default_capture_output_path(format).display().to_string(),
        format,
        true,
    ))
}

fn default_capture_output_path(format: CaptureFormat) -> PathBuf {
    default_output_dir().join(format!(
        "capture-{}-{}.{}",
        std::process::id(),
        timestamp_millis(),
        format.as_str()
    ))
}

fn default_output_dir() -> PathBuf {
    std::env::temp_dir().join(DEFAULT_OUTPUT_DIR_NAME)
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn default_capture_warnings(defaulted: bool, path: &str) -> Vec<Value> {
    if defaulted {
        vec![json!({
            "kind": "default_output_path",
            "message": "--out was not provided; capture wrote a PNG to the default temp path. Pass --out <path> to choose a persistent location.",
            "path": path,
        })]
    } else {
        Vec::new()
    }
}

fn default_payload_warnings(defaulted: bool, path: &Path) -> Vec<Value> {
    if defaulted {
        vec![json!({
            "kind": "default_payload_path",
            "message": "--payload was not provided; copy wrote the clipboard payload JSON to the default temp path. Pass --payload <path> to choose a persistent location.",
            "path": path.display().to_string(),
        })]
    } else {
        Vec::new()
    }
}

fn write_capture_output(
    primitives: &[RenderPrimitive],
    view_box: [f64; 4],
    output: &str,
    format: CaptureFormat,
    raster: RasterOptions,
) -> Result<CaptureWriteResult, String> {
    let svg = primitives_to_svg_viewbox(primitives, view_box, None);
    match format {
        CaptureFormat::Svg => {
            let bytes = write_text_output(Some(output), &svg)?;
            Ok(CaptureWriteResult {
                pixel_size: None,
                bytes,
            })
        }
        CaptureFormat::Png => {
            let pixel_size = pixel_size_for_view_box(view_box, raster)?;
            let bytes = write_svg_png_output(&svg, view_box, output, pixel_size)?;
            Ok(CaptureWriteResult {
                pixel_size: Some(pixel_size),
                bytes,
            })
        }
    }
}

fn pixel_size_for_view_box(view_box: [f64; 4], raster: RasterOptions) -> Result<PixelSize, String> {
    let source_width = view_box[2].max(1.0);
    let source_height = view_box[3].max(1.0);
    let (width, height) = match (raster.width, raster.height) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => {
            let height = ((width as f64) * source_height / source_width)
                .round()
                .max(1.0) as u32;
            (width, height)
        }
        (None, Some(height)) => {
            let width = ((height as f64) * source_width / source_height)
                .round()
                .max(1.0) as u32;
            (width, height)
        }
        (None, None) => (
            (source_width * raster.scale).round().max(1.0) as u32,
            (source_height * raster.scale).round().max(1.0) as u32,
        ),
    };
    validate_png_size(width, height)?;
    Ok(PixelSize {
        width,
        height,
        scale_x: width as f64 / source_width,
        scale_y: height as f64 / source_height,
    })
}

fn validate_png_size(width: u32, height: u32) -> Result<(), String> {
    if width > MAX_CAPTURE_SIDE_PX || height > MAX_CAPTURE_SIDE_PX {
        return Err(format!(
            "PNG capture dimensions {width}x{height} exceed the side limit {MAX_CAPTURE_SIDE_PX}px. Use --scale, --width, or --height to request a smaller image."
        ));
    }
    let pixels = width as u64 * height as u64;
    if pixels > MAX_CAPTURE_PIXELS {
        return Err(format!(
            "PNG capture dimensions {width}x{height} require {pixels} pixels, above the limit {MAX_CAPTURE_PIXELS}. Use --scale, --width, or --height to request a smaller image."
        ));
    }
    Ok(())
}

fn write_svg_png_output(
    svg: &str,
    view_box: [f64; 4],
    output: &str,
    pixel_size: PixelSize,
) -> Result<u64, String> {
    let pixmap = render_svg_png_pixmap(svg, view_box, pixel_size)?;
    ensure_output_parent_path(Path::new(output))?;
    pixmap
        .save_png(output)
        .map_err(|error| format!("Failed to write PNG {output}: {error}"))?;
    verify_file_written(Path::new(output), 8, "PNG capture")
}

fn render_svg_png_pixmap(
    svg: &str,
    view_box: [f64; 4],
    pixel_size: PixelSize,
) -> Result<tiny_skia::Pixmap, String> {
    let svg = svg_with_explicit_size(svg, view_box);
    let options = usvg_options_with_system_fonts();
    let tree = usvg::Tree::from_str(&svg, &options)
        .map_err(|error| format!("Failed to parse capture SVG for PNG output: {error}"))?;
    let mut pixmap = tiny_skia::Pixmap::new(pixel_size.width, pixel_size.height)
        .ok_or_else(|| "Failed to allocate PNG pixmap.".to_string())?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(pixel_size.scale_x as f32, pixel_size.scale_y as f32),
        &mut pixmap_mut,
    );
    Ok(pixmap)
}

fn usvg_options_with_system_fonts() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options.fontdb = capture_font_database();
    options.font_family = "Arial".to_string();
    options
}

fn capture_font_database() -> Arc<fontdb::Database> {
    static FONT_DB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    FONT_DB
        .get_or_init(|| {
            let mut database = fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn svg_with_explicit_size(svg: &str, view_box: [f64; 4]) -> String {
    svg.replacen(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"",
        &format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"",
            view_box[2].max(1.0),
            view_box[3].max(1.0)
        ),
        1,
    )
}

fn clipboard_document_for_target(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Result<ChemcoreDocument, String> {
    let bounds = target_bounds(document, target)?;
    let mut clipboard_document = match target {
        TargetSelector::All => document.clone(),
        TargetSelector::Object(id) => clipboard_document_for_object(document, id)?,
        TargetSelector::Molecule(index) => {
            let object_id = molecule_object_id(document, *index)?;
            clipboard_document_for_object(document, &object_id)?
        }
        TargetSelector::Node(id) => {
            clipboard_document_for_fragment_target(document, Some(id.as_str()), None)?
        }
        TargetSelector::Bond(id) => {
            clipboard_document_for_fragment_target(document, None, Some(id.as_str()))?
        }
        TargetSelector::Bounds(_) => {
            return Err("Bounds targets cannot be copied as editable Office objects.".to_string())
        }
        TargetSelector::Selection(_) => {
            return Err("Selection targets cannot be copied as a single editable Office object. Use copy all, object, molecule, node, or bond.".to_string())
        }
    };
    clipboard_document.document.id = "doc_clipboard_selection".to_string();
    clipboard_document.document.title = "Chemcore Clipboard Selection".to_string();
    set_clipboard_selection_bounds_meta(&mut clipboard_document, bounds);
    Ok(clipboard_document)
}

fn clipboard_document_for_object(
    document: &ChemcoreDocument,
    object_id: &str,
) -> Result<ChemcoreDocument, String> {
    let objects = clone_scene_object_path_by_id(&document.objects, object_id)
        .ok_or_else(|| format!("Object target not found: {object_id}."))?;
    let mut out = document.clone();
    out.objects = objects;
    Ok(out)
}

fn clone_scene_object_path_by_id(
    objects: &[SceneObject],
    object_id: &str,
) -> Option<Vec<SceneObject>> {
    for object in objects {
        if object.id == object_id {
            return Some(vec![object.clone()]);
        }
        if let Some(children) = clone_scene_object_path_by_id(&object.children, object_id) {
            let mut clone = object.clone();
            clone.children = children;
            return Some(vec![clone]);
        }
    }
    None
}

fn clipboard_document_for_fragment_target(
    document: &ChemcoreDocument,
    node_id: Option<&str>,
    bond_id: Option<&str>,
) -> Result<ChemcoreDocument, String> {
    for entry in document.editable_fragments() {
        let Some(resource_ref) = entry.object.payload.resource_ref.clone() else {
            continue;
        };
        let mut selected_node_ids = BTreeSet::new();
        let mut selected_bond_ids = BTreeSet::new();
        if let Some(node_id) = node_id {
            if entry.fragment.nodes.iter().any(|node| node.id == node_id) {
                selected_node_ids.insert(node_id.to_string());
            } else {
                continue;
            }
        }
        if let Some(bond_id) = bond_id {
            let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == bond_id) else {
                continue;
            };
            selected_bond_ids.insert(bond.id.clone());
            selected_node_ids.insert(bond.begin.clone());
            selected_node_ids.insert(bond.end.clone());
        }

        let nodes = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| selected_node_ids.contains(&node.id))
            .cloned()
            .collect::<Vec<_>>();
        if nodes.is_empty() {
            continue;
        }
        let bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                selected_bond_ids.contains(&bond.id)
                    && selected_node_ids.contains(&bond.begin)
                    && selected_node_ids.contains(&bond.end)
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut fragment = entry.fragment.clone();
        fragment.nodes = nodes;
        fragment.bonds = bonds;
        fragment.bbox = fragment_clipboard_bounds(&fragment.nodes);

        let mut object = entry.object.clone();
        object.payload.bbox = Some(fragment.bbox);

        let mut resource = document
            .resources
            .get(&resource_ref)
            .ok_or_else(|| format!("Missing molecule resource '{resource_ref}'."))?
            .clone();
        resource.data = ResourceData::Fragment(fragment);

        let mut out = document.clone();
        out.objects = vec![object];
        out.resources.insert(resource_ref, resource);
        return Ok(out);
    }
    match (node_id, bond_id) {
        (Some(id), _) => Err(format!("Node target not found: {id}.")),
        (_, Some(id)) => Err(format!("Bond target not found: {id}.")),
        _ => Err("No fragment target was provided.".to_string()),
    }
}

fn fragment_clipboard_bounds(nodes: &[Node]) -> [f64; 4] {
    let Some(first) = nodes.first() else {
        return [0.0, 0.0, 1.0, 1.0];
    };
    let mut min_x = first.position[0];
    let mut min_y = first.position[1];
    let mut max_x = first.position[0];
    let mut max_y = first.position[1];
    for node in nodes {
        min_x = min_x.min(node.position[0]);
        min_y = min_y.min(node.position[1]);
        max_x = max_x.max(node.position[0]);
        max_y = max_y.max(node.position[1]);
        if let Some(bounds) = node.label.as_ref().and_then(|label| label.bbox()) {
            min_x = min_x.min(bounds[0]);
            min_y = min_y.min(bounds[1]);
            max_x = max_x.max(bounds[2]);
            max_y = max_y.max(bounds[3]);
        }
    }
    [min_x, min_y, max_x.max(min_x + 1.0), max_y.max(min_y + 1.0)]
}

fn set_clipboard_selection_bounds_meta(document: &mut ChemcoreDocument, bounds: [f64; 4]) {
    if !document.document.meta.is_object() {
        document.document.meta = json!({});
    }
    let Some(meta) = document.document.meta.as_object_mut() else {
        return;
    };
    let clipboard = meta.entry("clipboard").or_insert_with(|| json!({}));
    if !clipboard.is_object() {
        *clipboard = json!({});
    }
    if let Some(clipboard) = clipboard.as_object_mut() {
        clipboard.insert(
            "selectionBounds".to_string(),
            json!({
                "minX": bounds[0],
                "minY": bounds[1],
                "maxX": bounds[2],
                "maxY": bounds[3],
            }),
        );
    }
}

fn clipboard_payload_for_document(document: &ChemcoreDocument) -> Result<Value, String> {
    let chemcore_document_json =
        serde_json::to_string(document).map_err(|error| error.to_string())?;
    let render_list_json =
        serde_json::to_string(&render_document(document)).map_err(|error| error.to_string())?;
    let cdxml = document_to_cdxml(document);
    let svg = document_to_svg(document);
    Ok(json!({
        "text": cdxml,
        "chemcoreFragmentJson": Value::Null,
        "chemcoreDocumentJson": chemcore_document_json,
        "renderListJson": render_list_json,
        "cdxml": cdxml,
        "svg": svg,
    }))
}

fn default_clipboard_payload_path() -> PathBuf {
    default_output_dir().join(format!(
        "copy-payload-{}-{}.json",
        std::process::id(),
        timestamp_millis()
    ))
}

fn write_clipboard_payload_file(path: &Path, payload: &Value) -> Result<u64, String> {
    ensure_output_parent_path(path)?;
    let text = serde_json::to_string_pretty(payload).map_err(|error| error.to_string())?;
    let expected_bytes = text.len() as u64;
    fs::write(path, text.as_bytes()).map_err(|error| {
        format!(
            "Failed to write clipboard payload {}: {error}",
            path.display()
        )
    })?;
    verify_file_written_exact(path, expected_bytes, "clipboard payload")
}

#[cfg(windows)]
fn copy_payload_to_office_clipboard(
    payload_path: &Path,
    office_helper: Option<&str>,
) -> Result<PathBuf, String> {
    let helper = resolve_office_helper(office_helper)?;
    let output = Command::new(&helper)
        .arg("--copy-clipboard-payload")
        .arg(payload_path)
        .output()
        .map_err(|error| format!("Failed to launch {}: {error}", helper.display()))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Office clipboard helper failed with exit code {:?}. payload={} stdout='{}' stderr='{}'",
            output.status.code(),
            payload_path.display(),
            stdout,
            stderr
        ));
    }
    Ok(helper)
}

#[cfg(not(windows))]
fn copy_payload_to_office_clipboard(
    payload_path: &Path,
    _office_helper: Option<&str>,
) -> Result<PathBuf, String> {
    Err(format!(
        "Copying to the Office/OLE clipboard is only supported on Windows. Payload was written to {}.",
        payload_path.display()
    ))
}

#[cfg(windows)]
fn resolve_office_helper(office_helper: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = office_helper {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!("Office helper was not found: {}.", path.display()));
    }

    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("chemcore-office.exe"));
            candidates.push(dir.join("resources").join("chemcore-office.exe"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("resources").join("chemcore-office.exe"));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(
            cwd.join("target")
                .join("release")
                .join("chemcore-office.exe"),
        );
        candidates.push(cwd.join("target").join("debug").join("chemcore-office.exe"));
    }

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "chemcore-office.exe was not found. Pass --office-helper <path>. Checked: {}",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn infers_png_capture_format_from_output_path() {
        assert_eq!(
            infer_capture_format_from_path("capture.png"),
            Some(CaptureFormat::Png)
        );
        assert_eq!(
            infer_capture_format_from_path("capture.SVG"),
            Some(CaptureFormat::Svg)
        );
        assert!(parse_capture_format("jpeg").is_err());
    }

    #[test]
    fn capture_output_defaults_to_temp_png() {
        let (path, format, defaulted) = resolve_capture_output(None, None).unwrap();
        assert_eq!(format, CaptureFormat::Png);
        assert!(defaulted);
        assert!(Path::new(&path).starts_with(default_output_dir()));
        assert_eq!(
            Path::new(&path)
                .extension()
                .and_then(|value| value.to_str()),
            Some("png")
        );
    }

    #[test]
    fn explicit_capture_output_without_extension_requires_format() {
        assert!(resolve_capture_output(Some("capture".to_string()), None).is_err());
        let (path, format, defaulted) =
            resolve_capture_output(Some("capture".to_string()), Some(CaptureFormat::Svg)).unwrap();
        assert_eq!(path, "capture");
        assert_eq!(format, CaptureFormat::Svg);
        assert!(!defaulted);
    }

    #[test]
    fn default_output_warnings_are_machine_readable() {
        let capture_warnings = default_capture_warnings(true, "capture.png");
        assert_eq!(capture_warnings[0]["kind"], "default_output_path");
        assert_eq!(capture_warnings[0]["path"], "capture.png");
        assert!(default_capture_warnings(false, "capture.png").is_empty());

        let payload_warnings = default_payload_warnings(true, Path::new("payload.json"));
        assert_eq!(payload_warnings[0]["kind"], "default_payload_path");
        assert_eq!(payload_warnings[0]["path"], "payload.json");
        assert!(default_payload_warnings(false, Path::new("payload.json")).is_empty());
    }

    #[test]
    fn expands_view_box_with_absolute_and_relative_sides() {
        let view_box = expanded_view_box(
            [10.0, 20.0, 30.0, 60.0],
            CropExpansion {
                abs_left: 1.0,
                abs_top: 2.0,
                abs_right: 3.0,
                abs_bottom: 4.0,
                rel_left: 0.1,
                rel_top: 0.25,
                rel_right: 0.2,
                rel_bottom: 0.0,
            },
        );
        assert_close(view_box[0], 7.0);
        assert_close(view_box[1], 8.0);
        assert_close(view_box[2], 30.0);
        assert_close(view_box[3], 56.0);
    }

    #[test]
    fn derives_png_height_from_fixed_width() {
        let pixel_size = pixel_size_for_view_box(
            [0.0, 0.0, 100.0, 50.0],
            RasterOptions {
                scale: 4.0,
                width: Some(1000),
                height: None,
            },
        )
        .unwrap();
        assert_eq!(pixel_size.width, 1000);
        assert_eq!(pixel_size.height, 500);
        assert_close(pixel_size.scale_x, 10.0);
        assert_close(pixel_size.scale_y, 10.0);
    }

    #[test]
    fn png_capture_renders_svg_text() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 30"><text x="5" y="22" font-size="22" font-family="sans-serif" fill="#000000">CN</text></svg>"##;
        let pixmap = render_svg_png_pixmap(
            svg,
            [0.0, 0.0, 80.0, 30.0],
            PixelSize {
                width: 800,
                height: 300,
                scale_x: 10.0,
                scale_y: 10.0,
            },
        )
        .unwrap();
        let dark_pixels = pixmap
            .data()
            .chunks_exact(4)
            .filter(|pixel| pixel[0] < 240 || pixel[1] < 240 || pixel[2] < 240)
            .count();
        assert!(
            dark_pixels > 500,
            "text-only SVG should produce visible non-white PNG pixels, got {dark_pixels}"
        );
    }

    #[test]
    fn detail_report_returns_raw_object_without_expanding_resource_by_default() {
        let document = ChemcoreDocument::blank();
        let report = detail_report(
            "blank.ccjs",
            &document,
            &TargetSelector::Object("obj_editor_molecule".to_string()),
            DetailOptions {
                include_raw: true,
                include_resource: false,
            },
        )
        .unwrap();
        assert_eq!(report["ok"], true);
        assert_eq!(report["detail"]["id"], "obj_editor_molecule");
        assert_eq!(
            report["detail"]["references"]["resource"]["id"],
            "mol_editor"
        );
        assert_eq!(
            report["detail"]["raw"]["object"]["id"],
            "obj_editor_molecule"
        );
        assert!(report["detail"]["raw"].get("resource").is_none());
    }

    #[test]
    fn detail_report_can_suppress_raw_or_include_molecule_fragment() {
        let document = ChemcoreDocument::blank();
        let summary = detail_report(
            "blank.ccjs",
            &document,
            &TargetSelector::Object("obj_editor_molecule".to_string()),
            DetailOptions {
                include_raw: false,
                include_resource: false,
            },
        )
        .unwrap();
        assert!(summary["detail"].get("raw").is_none());

        let molecule = detail_report(
            "blank.ccjs",
            &document,
            &TargetSelector::Molecule(0),
            DetailOptions {
                include_raw: true,
                include_resource: false,
            },
        )
        .unwrap();
        assert_eq!(molecule["detail"]["kind"], "molecule");
        assert_eq!(
            molecule["detail"]["raw"]["fragment"]["schema"],
            "chemcore.molecule.fragment2d"
        );
    }
}
