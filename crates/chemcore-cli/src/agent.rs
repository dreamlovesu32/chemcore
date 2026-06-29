use crate::{
    document_json, ensure_output_parent_path, load_engine_from_file, verify_file_written,
    verify_file_written_exact, write_json_value, write_text_output,
};
use chemcore_engine::{
    document_to_cdxml, document_to_svg, primitives_to_svg_viewbox, render_document,
    render_document_targets, render_primitives_bounds, Bond, ChemcoreDocument, Engine, Node,
    RenderPrimitive, ResourceData, SceneObject,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
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
    let objects = object_target_entries(&document);
    let molecules = molecule_target_entries(&document);
    let nodes = node_target_entries(&document);
    let bonds = bond_target_entries(&document);
    let target_count = 1 + objects.len() + molecules.len() + nodes.len() + bonds.len();
    let all_bounds = target_bounds(&document, &TargetSelector::All).ok();
    write_json_value(
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
        }),
        output.as_deref(),
        pretty,
    )
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
            "--bounds" => {
                index += 1;
                target = Some(TargetSelector::Bounds(parse_bounds_arg(
                    args.get(index)
                        .ok_or_else(|| "--bounds requires minX,minY,maxX,maxY.".to_string())?,
                )?));
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
        "capture requires --target <object:id|molecule:index|node:id|bond:id|all> or --bounds."
            .to_string()
    })?;
    let (output, format, output_defaulted) = resolve_capture_output(output, format)?;

    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let bounds = target_bounds(&document, &target)?;
    let view_box = expanded_view_box(bounds, expansion);
    let primitives = render_document(&document);
    let render_output = write_capture_output(&primitives, view_box, &output, format, raster)?;
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
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
        "context requires --target <object:id|molecule:index|node:id|bond:id|all>.".to_string()
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
        let primitives = render_document(&document);
        let render_output =
            write_capture_output(&primitives, query_view_box, capture_output, format, raster)?;
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
    let Some((kind, id)) = value.split_once(':') else {
        return Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, or bond:<id>."
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
        _ => Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, or bond:<id>."
        )),
    }
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
                "relationships": object_relationship_json(info),
                "isTarget": target_matches_object(target, info),
            })
        })
        .collect::<Vec<_>>();
    sort_context_entries(&mut objects);
    objects.truncate(limit);

    let mut molecules = document
        .editable_fragments()
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let bounds = target_bounds(document, &TargetSelector::Molecule(index)).ok()?;
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
                    "isTarget": matches!(target, TargetSelector::Molecule(target_index) if *target_index == index),
                })
            })
        })
        .collect::<Vec<_>>();
    sort_context_entries(&mut molecules);
    molecules.truncate(limit);

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
                    "isTarget": matches!(target, TargetSelector::Node(id) if id == &node.id),
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
                    "isTarget": matches!(target, TargetSelector::Bond(id) if id == &bond.id),
                }));
            }
        }
    }
    sort_context_entries(&mut nodes);
    sort_context_entries(&mut bonds);
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
        TargetSelector::All | TargetSelector::Bounds(_) => {
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
        "bounds": optional_bounds_json(target_bounds(document, &TargetSelector::Object(id.to_string())).ok()),
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
        "bounds": optional_bounds_json(target_bounds(document, &TargetSelector::Molecule(index)).ok()),
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
        if let Ok(bounds) = target_bounds(document, &TargetSelector::Object(object.id.clone())) {
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
        TargetSelector::All | TargetSelector::Bounds(_) => Value::Null,
    }
}

fn target_matches_object(target: &TargetSelector, info: &SceneObjectInfo) -> bool {
    matches!(target, TargetSelector::Object(id) if id == &info.id)
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
        let bounds = target_bounds(document, &TargetSelector::Object(object.id.clone()))
            .ok()
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
            let bounds = target_bounds(document, &TargetSelector::Molecule(index))
                .ok()
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
    let svg = svg_with_explicit_size(svg, view_box);
    let options = usvg::Options::default();
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
    ensure_output_parent_path(Path::new(output))?;
    pixmap
        .save_png(output)
        .map_err(|error| format!("Failed to write PNG {output}: {error}"))?;
    verify_file_written(Path::new(output), 8, "PNG capture")
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
