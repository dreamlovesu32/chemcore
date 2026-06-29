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

mod capture;
mod clipboard;
mod context;
mod detail;
mod output;
mod session;
mod target;
mod targets;

#[cfg(test)]
mod tests;

pub(crate) use capture::capture_command;
pub(crate) use clipboard::copy_command;
pub(crate) use context::context_command;
pub(crate) use detail::detail_command;
pub(crate) use session::session_command;
pub(crate) use target::parse_target_selector;
pub(crate) use targets::targets_command;

use capture::*;
use context::*;
use detail::*;
use output::*;
use target::*;
use targets::*;
