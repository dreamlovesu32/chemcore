use super::*;

pub(super) const ENV_DISABLE_PACKAGED_TRAILING_TRIM: &str =
    "CHEMCORE_EMF_PACKAGED_TEXT_DISABLE_TRAILING_TRIM";
pub(super) const ENV_DISABLE_PACKAGED_NOFITBLACKBOX: &str =
    "CHEMCORE_EMF_PACKAGED_TEXT_DISABLE_NOFITBLACKBOX";
pub(super) const ENV_PACKAGED_TEXT_GRIDFIT: &str = "CHEMCORE_EMF_PACKAGED_TEXT_GRIDFIT";
pub(super) const ENV_PACKAGED_PIXEL_OFFSET_HIGHQUALITY: &str =
    "CHEMCORE_EMF_PACKAGED_PIXEL_OFFSET_HIGHQUALITY";
pub(super) const ENV_PACKAGED_PIXEL_OFFSET_MODE_VALUE: &str =
    "CHEMCORE_EMF_PACKAGED_PIXEL_OFFSET_MODE_VALUE";
pub(super) const ENV_PACKAGED_CENTERED_PLAIN_GDI_WIDTH: &str =
    "CHEMCORE_EMF_PACKAGED_CENTERED_PLAIN_GDI_WIDTH";
pub(super) const ENV_PACKAGED_CENTERED_PLAIN_ZERO_LAYOUT: &str =
    "CHEMCORE_EMF_PACKAGED_CENTERED_PLAIN_ZERO_LAYOUT";
pub(super) const ENV_PACKAGED_ATTACHED_START_ZERO_LAYOUT: &str =
    "CHEMCORE_EMF_PACKAGED_ATTACHED_START_ZERO_LAYOUT";
pub(super) const ENV_PACKAGED_ATTACHED_START_TIGHT_RECT: &str =
    "CHEMCORE_EMF_PACKAGED_ATTACHED_START_TIGHT_RECT";
pub(super) const ENV_PACKAGED_NODE_LABEL_LAYOUT_EXPERIMENT: &str =
    "CHEMCORE_EMF_PACKAGED_NODE_LABEL_LAYOUT_EXPERIMENT";
pub(super) const ENV_PACKAGED_CENTERED_TEXT_TOP_BIAS_EM: &str =
    "CHEMCORE_EMF_PACKAGED_CENTERED_TEXT_TOP_BIAS_EM";
pub(super) const ENV_PACKAGED_CENTERED_SCRIPT_EXTRA_TOP_BIAS_EM: &str =
    "CHEMCORE_EMF_PACKAGED_CENTERED_SCRIPT_EXTRA_TOP_BIAS_EM";
pub(super) const ENV_DEFAULT_MULTILINE_BLACK_LABEL_Y_NUDGE_PX: &str =
    "CHEMCORE_EMF_DEFAULT_MULTILINE_BLACK_LABEL_Y_NUDGE_PX";
pub(super) const ENV_PACKAGED_SMOOTHING_MODE_VALUE: &str =
    "CHEMCORE_EMF_PACKAGED_SMOOTHING_MODE_VALUE";
pub(super) const ENV_ATTACHED_LABEL_REPLAY_PHASE_POLICY_EXPERIMENT: &str =
    "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_PHASE_POLICY_EXPERIMENT";
pub(super) const ENV_HIDE_DOCUMENT_KNOCKOUT: &str = "CHEMCORE_EMF_HIDE_DOCUMENT_KNOCKOUT";
pub(super) const ENV_SHOW_INVALID_MARKERS: &str = "CHEMCORE_EMF_SHOW_INVALID_MARKERS";
pub(super) const ENV_HIDE_DOCUMENT_TEXT: &str = "CHEMCORE_EMF_HIDE_DOCUMENT_TEXT";
pub(super) const ENV_HIDE_DOCUMENT_BOND: &str = "CHEMCORE_EMF_HIDE_DOCUMENT_BOND";
pub(super) const ENV_HIDE_DOCUMENT_GRAPHIC: &str = "CHEMCORE_EMF_HIDE_DOCUMENT_GRAPHIC";
pub(super) const ENV_BOND_PEN_CONVERSION_EXPERIMENT: &str =
    "CHEMCORE_EMF_BOND_PEN_CONVERSION_EXPERIMENT";
pub(super) const ENV_INCLUDE_OBJECT_IDS: &str = "CHEMCORE_EMF_INCLUDE_OBJECT_IDS";
pub(super) const ENV_INCLUDE_NODE_IDS: &str = "CHEMCORE_EMF_INCLUDE_NODE_IDS";

pub(super) fn preview_env_enabled(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreviewBondPenConversionMode {
    All,
    Off,
    LabeledOnly,
    LabeledOrCenterDouble,
    NoSideDouble,
    LabeledOrNonSideDouble,
    LabeledComplex,
    LabeledOrder2OrBothJunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreviewPackagedNodeLabelLayoutMode {
    PayloadSize,
    PayloadBox,
    AttachedPayloadSize,
    AttachedPayloadBox,
    SimplePayloadSize,
    SimplePayloadBox,
}

pub(super) fn preview_packaged_node_label_layout_mode() -> Option<PreviewPackagedNodeLabelLayoutMode>
{
    let raw = std::env::var_os(ENV_PACKAGED_NODE_LABEL_LAYOUT_EXPERIMENT)?;
    let value = raw.to_string_lossy();
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "off" | "none" | "disabled" => None,
        "payload-box" | "box" | "bbox" => Some(PreviewPackagedNodeLabelLayoutMode::PayloadBox),
        "payload-size" | "size" => Some(PreviewPackagedNodeLabelLayoutMode::PayloadSize),
        "attached-payload-box" | "attached-box" | "attached-bbox" => {
            Some(PreviewPackagedNodeLabelLayoutMode::AttachedPayloadBox)
        }
        "attached-payload-size" | "attached-size" => {
            Some(PreviewPackagedNodeLabelLayoutMode::AttachedPayloadSize)
        }
        "simple-payload-box" | "simple-box" | "simple-bbox" => {
            Some(PreviewPackagedNodeLabelLayoutMode::SimplePayloadBox)
        }
        "simple-payload-size" | "simple-size" => {
            Some(PreviewPackagedNodeLabelLayoutMode::SimplePayloadSize)
        }
        _ => None,
    }
}

pub(super) fn preview_bond_pen_conversion_mode() -> PreviewBondPenConversionMode {
    std::env::var(ENV_BOND_PEN_CONVERSION_EXPERIMENT)
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "0" | "false" | "off" | "none" | "disabled" => PreviewBondPenConversionMode::Off,
            "labeled-only" | "label-only" | "labeled" | "label" => {
                PreviewBondPenConversionMode::LabeledOnly
            }
            "labeled-or-center-double" | "label-or-center-double" | "labeled-center-double" => {
                PreviewBondPenConversionMode::LabeledOrCenterDouble
            }
            "no-side-double" | "except-side-double" | "disable-side-double" => {
                PreviewBondPenConversionMode::NoSideDouble
            }
            "labeled-or-non-side-double" | "label-or-non-side-double" => {
                PreviewBondPenConversionMode::LabeledOrNonSideDouble
            }
            "labeled-complex" | "label-complex" => PreviewBondPenConversionMode::LabeledComplex,
            "labeled-order2-or-both-junction"
            | "label-order2-or-both-junction"
            | "labeled-order2-both-junction" => {
                PreviewBondPenConversionMode::LabeledOrder2OrBothJunction
            }
            _ => PreviewBondPenConversionMode::All,
        })
        .unwrap_or(PreviewBondPenConversionMode::All)
}

pub(super) fn preview_bond_pen_conversion_allowed(
    mode: PreviewBondPenConversionMode,
    bond_info: Option<&super::PreviewBondInfo>,
) -> bool {
    match mode {
        PreviewBondPenConversionMode::All => true,
        PreviewBondPenConversionMode::Off => false,
        PreviewBondPenConversionMode::LabeledOnly => {
            bond_info.is_some_and(|info| info.start_has_label || info.end_has_label)
        }
        PreviewBondPenConversionMode::LabeledOrCenterDouble => bond_info
            .is_some_and(|info| info.start_has_label || info.end_has_label || info.center_double),
        PreviewBondPenConversionMode::NoSideDouble => {
            bond_info.is_some_and(|info| !info.side_double)
        }
        PreviewBondPenConversionMode::LabeledOrNonSideDouble => bond_info
            .is_some_and(|info| info.start_has_label || info.end_has_label || !info.side_double),
        PreviewBondPenConversionMode::LabeledComplex => bond_info.is_some_and(|info| {
            (info.start_has_label || info.end_has_label)
                && (info.both_junction || info.side_double || info.center_double)
        }),
        PreviewBondPenConversionMode::LabeledOrder2OrBothJunction => {
            bond_info.is_some_and(|info| {
                (info.start_has_label || info.end_has_label)
                    && (info.order >= 2 || info.both_junction)
            })
        }
    }
}

pub(super) fn preview_env_object_id_filter() -> Option<std::collections::BTreeSet<String>> {
    let raw = std::env::var(ENV_INCLUDE_OBJECT_IDS).ok()?;
    let ids = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    if ids.is_empty() {
        None
    } else {
        Some(ids)
    }
}

pub(super) fn preview_env_node_id_filter() -> Option<std::collections::BTreeSet<String>> {
    let raw = std::env::var(ENV_INCLUDE_NODE_IDS).ok()?;
    let ids = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    if ids.is_empty() {
        None
    } else {
        Some(ids)
    }
}

pub(super) fn preview_primitive_object_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Line { object_id, .. }
        | RenderPrimitive::Circle { object_id, .. }
        | RenderPrimitive::Polygon { object_id, .. }
        | RenderPrimitive::Rect { object_id, .. }
        | RenderPrimitive::Ellipse { object_id, .. }
        | RenderPrimitive::Polyline { object_id, .. }
        | RenderPrimitive::Path { object_id, .. }
        | RenderPrimitive::FilledPath { object_id, .. }
        | RenderPrimitive::Text { object_id, .. } => object_id.as_deref(),
    }
}

pub(super) fn preview_primitive_node_id(primitive: &RenderPrimitive) -> Option<&str> {
    match primitive {
        RenderPrimitive::Circle { node_id, .. }
        | RenderPrimitive::Polygon { node_id, .. }
        | RenderPrimitive::Rect { node_id, .. }
        | RenderPrimitive::Text { node_id, .. } => node_id.as_deref(),
        _ => None,
    }
}

pub(super) fn preview_is_invalid_marker_primitive(primitive: &RenderPrimitive) -> bool {
    match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. }
            if *role == RenderRole::DocumentDiagnostic =>
        {
            true
        }
        RenderPrimitive::Rect {
            role,
            fill,
            stroke,
            stroke_width,
            node_id: _,
            dash_array,
            ..
        } => {
            *role == RenderRole::DocumentGraphic
                && dash_array.is_empty()
                && fill.as_deref() == Some("none")
                && stroke.as_deref() == Some("#d32f2f")
                && (*stroke_width - 1.0).abs() < f64::EPSILON
        }
        RenderPrimitive::Circle {
            role,
            fill,
            stroke,
            stroke_width,
            node_id,
            ..
        } => {
            *role == RenderRole::DocumentGraphic
                && node_id.is_some()
                && fill == "none"
                && stroke == "#d32f2f"
                && (*stroke_width - 1.0).abs() < f64::EPSILON
        }
        _ => false,
    }
}

pub(super) fn preview_env_i32(name: &str) -> Option<i32> {
    std::env::var(name).ok()?.trim().parse::<i32>().ok()
}

pub(super) fn preview_env_f64(name: &str) -> Option<f64> {
    std::env::var(name).ok()?.trim().parse::<f64>().ok()
}

pub(super) fn preview_env_f64_or(name: &str, default: f64) -> f64 {
    preview_env_f64(name).unwrap_or(default)
}
