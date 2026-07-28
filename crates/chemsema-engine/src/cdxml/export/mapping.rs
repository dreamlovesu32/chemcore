use super::*;

pub(super) fn cdxml_node_label_alignment(label: &NodeLabel) -> &'static str {
    if label.layout.as_deref() == Some("attached-group-above") {
        "Above"
    } else if label.layout.as_deref() == Some("attached-group-below") {
        "Below"
    } else if label.layout.as_deref() == Some("attached-group-center") {
        "Center"
    } else {
        "Auto"
    }
}

pub(super) fn cdxml_node_label_interpret_chemically(label: &NodeLabel) -> bool {
    if let Some(value) = label.meta.get("defaultChemical").and_then(Value::as_bool) {
        return value;
    }
    label_source_runs_for_export(label)
        .unwrap_or_else(|| label.runs.clone())
        .iter()
        .any(|run| run.script.as_deref() == Some("chemical"))
}

pub(super) fn cdxml_node_num_hydrogens_for_export(node: &Node) -> Option<u8> {
    if let Some(value) = crate::node_user_num_hydrogens_override(node) {
        return Some(value);
    }
    if let Some(value) = node
        .meta
        .pointer("/import/cdxml/explicitNumHydrogens")
        .and_then(Value::as_u64)
    {
        return Some(value.min(u64::from(u8::MAX)) as u8);
    }
    (node.num_hydrogens > 0).then_some(node.num_hydrogens)
}

pub(super) fn cdxml_label_line_starts(authored_text: &str) -> Option<String> {
    // ChemDraw can display one authored chemical label as several directional
    // rows (for example H above N) without authored line breaks. That layout is
    // carried by LabelAlignment and must not be serialized as LineStarts.
    // LineStarts is emitted only for LF characters that are actually present
    // in the authored text stream.
    if !authored_text.contains('\n') {
        return None;
    }
    let mut starts = authored_text
        .bytes()
        .enumerate()
        .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset + 1))
        .collect::<Vec<_>>();
    starts.push(authored_text.len());
    Some(
        starts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(super) fn label_source_runs_for_export(label: &NodeLabel) -> Option<Vec<LabelRun>> {
    label
        .meta
        .get("sourceRuns")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .filter(|runs| !runs.is_empty())
}

pub(super) fn cdxml_bond_display(bond: &Bond, second: bool) -> Option<&'static str> {
    if let Some(stereo) = &bond.stereo {
        if stereo.kind == "solid-wedge" {
            return Some(if stereo.wide_end == "end" {
                "WedgeBegin"
            } else {
                "WedgeEnd"
            });
        }
        if stereo.kind == "hashed-wedge" {
            return Some(if stereo.wide_end == "end" {
                "WedgedHashBegin"
            } else {
                "WedgedHashEnd"
            });
        }
        if stereo.kind == "hollow-wedge" {
            return Some(if stereo.wide_end == "end" {
                "HollowWedgeBegin"
            } else {
                "HollowWedgeEnd"
            });
        }
    }
    if second {
        let (line_style, line_weight) = match bond.double.as_ref().map(|double| double.placement) {
            Some(crate::DoubleBondPlacement::Left) => {
                (bond.line_styles.left, bond.line_weights.left)
            }
            _ => (bond.line_styles.right, bond.line_weights.right),
        };
        if line_style == crate::BondLinePattern::Dashed {
            return Some("Dash");
        }
        if line_weight == crate::BondLineWeight::Bold {
            return Some("Bold");
        }
        return None;
    }
    if bond.line_styles.main == crate::BondLinePattern::Dashed {
        return Some("Dash");
    }
    if bond.line_styles.main == crate::BondLinePattern::Wavy {
        return Some("Wavy");
    }
    if bond.line_weights.main == crate::BondLineWeight::Bold {
        return Some("Bold");
    }
    None
}

pub(super) fn cdxml_arrow_kind(value: Option<&Value>) -> &'static str {
    match value
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase()
        .as_str()
    {
        "hollow" => "Hollow",
        "open" | "angle" | "retrosynthetic" => "Angle",
        "equilibrium" | "unequal-equilibrium" => "Equilibrium",
        _ => "Solid",
    }
}

pub(super) fn cdxml_arrow_equilibrium_ratio(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase();
    let ratio = value
        .get("equilibriumRatio")
        .or_else(|| value.get("equilibrium_ratio"))
        .and_then(Value::as_f64)
        .filter(|ratio| ratio.is_finite() && *ratio > 1.0)
        .unwrap_or_else(|| {
            if kind == "unequal-equilibrium" {
                3.0
            } else {
                1.0
            }
        });
    (ratio > 1.0).then_some(ratio)
}

pub(super) fn cdxml_arrowhead_type_attr(arrow_kind: &str) -> &str {
    if arrow_kind == "Equilibrium" {
        "Solid"
    } else {
        arrow_kind
    }
}

pub(super) fn cdxml_arrow_endpoint_position(
    payload: &ObjectPayload,
    arrow: Option<&Value>,
    key: &str,
    legacy_enabled_value: &str,
) -> &'static str {
    if let Some(value) = arrow
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .and_then(cdxml_arrow_endpoint_style)
    {
        return value;
    }
    if payload_string_cdxml(payload, key)
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(legacy_enabled_value))
    {
        "Full"
    } else {
        "None"
    }
}

pub(super) fn cdxml_arrow_endpoint_style(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "full" => Some("Full"),
        "half-left" | "halfleft" | "left" | "top" => Some("HalfLeft"),
        "half-right" | "halfright" | "right" | "bottom" => Some("HalfRight"),
        "none" => Some("None"),
        _ => None,
    }
}

pub(super) fn cdxml_curve_endpoint_name(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "half" | "half-left" | "halfleft" | "left" | "top" => "HalfLeft",
        "half-right" | "halfright" | "right" | "bottom" => "HalfRight",
        "full" => "Full",
        _ => "None",
    }
}

pub(super) fn cdxml_arrow_size_attribute(value: f64) -> f64 {
    value * 100.0
}

pub(super) fn cdxml_arrow_fill_type(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Some("None"),
        "solid" => Some("Solid"),
        "shaded" => Some("Shaded"),
        _ => None,
    }
}

pub(super) fn cdxml_symbol_anchor_bbox(
    center_x: f64,
    center_y: f64,
    anchor_width: f64,
    anchor_height: f64,
) -> [f64; 4] {
    if anchor_width.abs() > crate::EPSILON {
        [center_x, center_y, center_x - anchor_width, center_y]
    } else if anchor_height.abs() > crate::EPSILON {
        [center_x, center_y, center_x, center_y + anchor_height]
    } else {
        [center_x, center_y, center_x, center_y]
    }
}

pub(super) fn cdxml_arrow_no_go(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "cross" => Some("Cross"),
        "hash" => Some("Hash"),
        _ => None,
    }
}

pub(super) fn cdxml_arrow_object_reference(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(super) fn push_cdxml_shape_type_flag(out: &mut String, enabled: bool, flag: &str) {
    if !enabled {
        return;
    }
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(flag);
}

pub(super) fn cdxml_orbital_type(template: &str, style: &str, phase: &str) -> &'static str {
    match (template, style, phase) {
        ("s", "shaded", _) => "sShaded",
        ("s", "filled", _) => "sFilled",
        ("s", _, _) => "s",
        ("p", "filled", _) => "pFilled",
        ("p", _, _) => "p",
        ("dxy", "filled", _) => "dxyFilled",
        ("dxy", _, _) => "dxy",
        ("oval", "shaded", _) => "ovalShaded",
        ("oval", "filled", _) => "ovalFilled",
        ("oval", _, _) => "oval",
        ("hybrid", "filled", "minus") => "hybridMinusFilled",
        ("hybrid", _, "minus") => "hybridMinus",
        ("hybrid", "filled", _) => "hybridPlusFilled",
        ("hybrid", _, _) => "hybridPlus",
        ("dz2", "filled", "minus") => "dz2MinusFilled",
        ("dz2", _, "minus") => "dz2Minus",
        ("dz2", "filled", _) => "dz2PlusFilled",
        ("dz2", _, _) => "dz2Plus",
        ("lobe", "shaded", _) => "lobeShaded",
        ("lobe", "filled", _) => "lobeFilled",
        ("lobe", _, _) => "lobe",
        _ => "s",
    }
}

pub(super) fn cdxml_justification(value: Option<&str>) -> &'static str {
    match value.unwrap_or("").to_ascii_lowercase().as_str() {
        "center" | "middle" => "Center",
        "right" | "end" => "Right",
        "full" | "justify" => "Full",
        "above" => "Above",
        "below" => "Below",
        "auto" => "Auto",
        "best" => "Best",
        _ => "Left",
    }
}
