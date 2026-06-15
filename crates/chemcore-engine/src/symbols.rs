use crate::{ChemcoreDocument, Node, Point, SceneObject};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const ELECTRON_SYMBOL_ATTACH_RADIUS: f64 = 10.0;
const ATTACHED_ELECTRON_SYMBOLS_META: &str = "attachedElectronSymbols";
const SYMBOL_BASE_CHARGE_META: &str = "symbolBaseCharge";
const SYMBOL_BASE_RADICAL_META: &str = "symbolBaseRadicalCount";
const RADICAL_COUNT_META: &str = "radicalCount";
const CHARGE_SYMBOL_INVALID_META: &str = "chargeSymbolInvalid";
const EFFECTIVE_NUM_HYDROGENS_META: &str = "effectiveNumHydrogens";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CdxmlSymbolStyle {
    Default,
    Acs,
}

#[derive(Debug, Clone, Copy)]
pub struct CdxmlSymbolMetrics {
    pub width: f64,
    pub height: f64,
    pub stroke_width: Option<f64>,
    pub cdxml_anchor_width: f64,
    pub cdxml_anchor_height: f64,
    pub line_width: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ElectronSymbolChemistry {
    pub role: &'static str,
    pub charge_delta: i32,
    pub radical_delta: i32,
    pub requires_hydrogen_removal: bool,
}

#[derive(Debug, Clone)]
struct SymbolAttachment {
    symbol_object_id: String,
    kind: String,
    node_id: Option<String>,
    attachment_source: Option<&'static str>,
    attachment_distance: Option<f64>,
    chemistry: ElectronSymbolChemistry,
}

#[derive(Debug, Clone)]
struct AttachmentCandidate {
    node_id: String,
    source: &'static str,
    distance: f64,
}

pub fn electron_symbol_chemistry(kind: &str) -> Option<ElectronSymbolChemistry> {
    match kind {
        "circle-plus" | "plus" => Some(ElectronSymbolChemistry {
            role: "charge",
            charge_delta: 1,
            radical_delta: 0,
            requires_hydrogen_removal: true,
        }),
        "circle-minus" | "minus" => Some(ElectronSymbolChemistry {
            role: "charge",
            charge_delta: -1,
            radical_delta: 0,
            requires_hydrogen_removal: true,
        }),
        "radical-cation" => Some(ElectronSymbolChemistry {
            role: "radical-cation",
            charge_delta: 1,
            radical_delta: 1,
            requires_hydrogen_removal: false,
        }),
        "radical-anion" => Some(ElectronSymbolChemistry {
            role: "radical-anion",
            charge_delta: -1,
            radical_delta: 1,
            requires_hydrogen_removal: false,
        }),
        "electron" => Some(ElectronSymbolChemistry {
            role: "radical",
            charge_delta: 0,
            radical_delta: 1,
            requires_hydrogen_removal: true,
        }),
        "lone-pair" => Some(ElectronSymbolChemistry {
            role: "lone-pair",
            charge_delta: 0,
            radical_delta: 0,
            requires_hydrogen_removal: false,
        }),
        _ => None,
    }
}

pub fn refresh_attached_electron_symbols(document: &mut ChemcoreDocument) -> bool {
    let attachments = detect_symbol_attachments(document);
    let mut changed = refresh_symbol_object_attachment_payloads(document, &attachments);
    changed |= refresh_node_attached_electron_symbols(document, &attachments);
    changed
}

pub fn node_radical_count(node: &Node) -> i32 {
    node.meta
        .get(RADICAL_COUNT_META)
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32
}

pub fn node_attached_electron_symbols(node: &Node) -> Vec<Value> {
    node.meta
        .get(ATTACHED_ELECTRON_SYMBOLS_META)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn node_effective_num_hydrogens(node: &Node) -> u8 {
    node_effective_num_hydrogens_override(node).unwrap_or(node.num_hydrogens)
}

pub fn node_effective_num_hydrogens_override(node: &Node) -> Option<u8> {
    node.meta
        .get(EFFECTIVE_NUM_HYDROGENS_META)
        .and_then(Value::as_u64)
        .map(|value| value.min(u64::from(u8::MAX)) as u8)
}

pub fn node_has_charge_symbol_invalid(node: &Node) -> bool {
    node.meta
        .get(CHARGE_SYMBOL_INVALID_META)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn detect_symbol_attachments(document: &ChemcoreDocument) -> Vec<SymbolAttachment> {
    let Some(entry) = document.editable_fragment() else {
        return Vec::new();
    };
    let object_translate = entry.object.transform.translate;
    let mut out = Vec::new();
    for object in &document.objects {
        if object.object_type != "symbol" || !object.visible {
            continue;
        }
        let Some(kind) = object
            .payload
            .extra
            .get("kind")
            .and_then(Value::as_str)
            .map(ToString::to_string)
        else {
            continue;
        };
        let Some(chemistry) = electron_symbol_chemistry(&kind) else {
            continue;
        };
        let Some(center) = scene_object_center(object) else {
            continue;
        };
        let candidate = nearest_attachment_candidate(entry.fragment, object_translate, center);
        let (node_id, attachment_source, attachment_distance) = if let Some(candidate) =
            candidate.filter(|candidate| candidate.distance <= ELECTRON_SYMBOL_ATTACH_RADIUS)
        {
            (
                Some(candidate.node_id),
                Some(candidate.source),
                Some(candidate.distance),
            )
        } else {
            (None, None, None)
        };
        out.push(SymbolAttachment {
            symbol_object_id: object.id.clone(),
            kind,
            node_id,
            attachment_source,
            attachment_distance,
            chemistry,
        });
    }
    out
}

fn refresh_symbol_object_attachment_payloads(
    document: &mut ChemcoreDocument,
    attachments: &[SymbolAttachment],
) -> bool {
    let mut by_id: BTreeMap<&str, &SymbolAttachment> = BTreeMap::new();
    for attachment in attachments {
        by_id.insert(attachment.symbol_object_id.as_str(), attachment);
    }
    let mut changed = false;
    for object in &mut document.objects {
        if object.object_type != "symbol" {
            continue;
        }
        let Some(attachment) = by_id.get(object.id.as_str()) else {
            continue;
        };
        changed |= set_payload_value(
            object,
            "chemicalRole",
            Some(json!(attachment.chemistry.role)),
        );
        changed |= set_payload_value(
            object,
            "chargeDelta",
            Some(json!(attachment.chemistry.charge_delta)),
        );
        changed |= set_payload_value(
            object,
            "radicalDelta",
            Some(json!(attachment.chemistry.radical_delta)),
        );
        changed |= set_payload_value(
            object,
            "attachedAtomId",
            attachment.node_id.as_ref().map(|node_id| json!(node_id)),
        );
        changed |= set_payload_value(
            object,
            "attachmentSource",
            attachment.attachment_source.map(|source| json!(source)),
        );
        changed |= set_payload_value(
            object,
            "attachmentDistance",
            attachment
                .attachment_distance
                .map(|distance| json!(distance)),
        );
    }
    changed
}

fn refresh_node_attached_electron_symbols(
    document: &mut ChemcoreDocument,
    attachments: &[SymbolAttachment],
) -> bool {
    let Some(entry) = document.editable_fragment_mut() else {
        return false;
    };
    let mut by_node: BTreeMap<&str, Vec<&SymbolAttachment>> = BTreeMap::new();
    for attachment in attachments {
        if let Some(node_id) = attachment.node_id.as_deref() {
            by_node.entry(node_id).or_default().push(attachment);
        }
    }
    let mut connection_orders: BTreeMap<&str, i32> = BTreeMap::new();
    for bond in &entry.fragment.bonds {
        let order = i32::from(bond.order.max(1));
        *connection_orders.entry(bond.begin.as_str()).or_default() += order;
        *connection_orders.entry(bond.end.as_str()).or_default() += order;
    }

    let mut changed = false;
    for node in &mut entry.fragment.nodes {
        let old_symbol_charge = attached_symbol_charge_sum(node);
        let old_symbol_radical = attached_symbol_radical_sum(node);
        let had_symbol_state = old_symbol_charge != 0
            || old_symbol_radical != 0
            || node.meta.get(SYMBOL_BASE_CHARGE_META).is_some()
            || node.meta.get(SYMBOL_BASE_RADICAL_META).is_some()
            || node.meta.get(ATTACHED_ELECTRON_SYMBOLS_META).is_some()
            || node.meta.get(RADICAL_COUNT_META).is_some()
            || node.meta.get(EFFECTIVE_NUM_HYDROGENS_META).is_some()
            || node.meta.get(CHARGE_SYMBOL_INVALID_META).is_some();
        let node_attachments = by_node.remove(node.id.as_str()).unwrap_or_default();
        if node_attachments.is_empty() && !had_symbol_state {
            continue;
        }
        let base_charge = node
            .meta
            .get(SYMBOL_BASE_CHARGE_META)
            .and_then(Value::as_i64)
            .map(|value| value as i32)
            .unwrap_or(node.charge - old_symbol_charge);
        let base_radical = node
            .meta
            .get(SYMBOL_BASE_RADICAL_META)
            .and_then(Value::as_i64)
            .map(|value| value as i32)
            .unwrap_or(node_radical_count(node) - old_symbol_radical);
        let charge_delta: i32 = node_attachments
            .iter()
            .map(|attachment| attachment.chemistry.charge_delta)
            .sum();
        let radical_delta: i32 = node_attachments
            .iter()
            .map(|attachment| attachment.chemistry.radical_delta)
            .sum();
        let next_charge = base_charge + charge_delta;
        if node.charge != next_charge {
            node.charge = next_charge;
            changed = true;
        }
        let attached_values: Vec<Value> = node_attachments
            .iter()
            .map(|attachment| {
                json!({
                    "symbolObjectId": attachment.symbol_object_id,
                    "sourceSymbolObjectId": attachment.symbol_object_id,
                    "kind": attachment.kind,
                    "chargeDelta": attachment.chemistry.charge_delta,
                    "radicalDelta": attachment.chemistry.radical_delta,
                    "requiresHydrogenRemoval": attachment.chemistry.requires_hydrogen_removal,
                })
            })
            .collect();
        changed |= set_node_meta_value(
            node,
            ATTACHED_ELECTRON_SYMBOLS_META,
            if attached_values.is_empty() {
                None
            } else {
                Some(Value::Array(attached_values))
            },
        );
        changed |= set_node_meta_value(node, SYMBOL_BASE_CHARGE_META, Some(json!(base_charge)));
        changed |= set_node_meta_value(node, SYMBOL_BASE_RADICAL_META, Some(json!(base_radical)));
        let next_radical = base_radical + radical_delta;
        changed |= set_node_meta_value(
            node,
            RADICAL_COUNT_META,
            if next_radical == 0 {
                None
            } else {
                Some(json!(next_radical))
            },
        );

        let connection_order = *connection_orders.get(node.id.as_str()).unwrap_or(&0);
        let (effective_hydrogens, invalid) = effective_hydrogens_and_invalid(
            node,
            connection_order,
            base_charge,
            base_radical,
            &node_attachments,
        );
        changed |= set_node_meta_value(
            node,
            EFFECTIVE_NUM_HYDROGENS_META,
            effective_hydrogens.map(|value| json!(value)),
        );
        changed |= set_node_meta_value(
            node,
            CHARGE_SYMBOL_INVALID_META,
            if invalid { Some(json!(true)) } else { None },
        );
    }
    changed
}

fn effective_hydrogens_and_invalid(
    node: &Node,
    connection_order: i32,
    base_charge: i32,
    base_radical: i32,
    attachments: &[&SymbolAttachment],
) -> (Option<u8>, bool) {
    if node.is_placeholder {
        return (None, false);
    }
    if node.atomic_number != 6 {
        let ordinary_negative_removals = attachments
            .iter()
            .filter(|attachment| {
                attachment.chemistry.charge_delta < 0 && attachment.chemistry.radical_delta == 0
            })
            .count() as i32;
        if ordinary_negative_removals == 0 {
            return (None, false);
        }
        let neutral_hydrogens = supported_hetero_hydrogens(
            node.atomic_number,
            base_charge,
            base_radical,
            connection_order,
        );
        return (None, neutral_hydrogens < ordinary_negative_removals);
    }
    let neutral_hydrogens = (4 - connection_order).clamp(0, 4);
    let required_removals = attachments
        .iter()
        .filter(|attachment| attachment.chemistry.requires_hydrogen_removal)
        .count() as i32;
    if required_removals == 0 {
        return (Some(neutral_hydrogens as u8), false);
    }
    if neutral_hydrogens < required_removals {
        return (Some(neutral_hydrogens as u8), true);
    }
    (Some((neutral_hydrogens - required_removals) as u8), false)
}

fn supported_hetero_hydrogens(
    atomic_number: u8,
    charge: i32,
    radical_count: i32,
    connection_order: i32,
) -> i32 {
    let Some(valence) =
        typical_symbol_valence(atomic_number, charge, connection_order, radical_count)
    else {
        return 0;
    };
    let charge_hydrogen_penalty = symbol_hydrogen_charge_penalty(atomic_number, charge);
    (valence - radical_count - connection_order - charge_hydrogen_penalty).clamp(0, 9)
}

fn third_period_main_group_valence_series(atomic_number: u8) -> Option<(i32, i32)> {
    match atomic_number {
        13 | 31 | 49 | 81 | 113 => Some((3, 3)),
        14 | 32 | 50 | 82 | 114 => Some((4, 4)),
        15 | 33 | 51 | 83 | 115 => Some((3, 5)),
        16 | 34 | 52 | 84 | 116 => Some((2, 6)),
        17 | 35 | 53 | 85 | 117 => Some((1, 7)),
        _ => None,
    }
}

fn third_period_main_group_target_valence(atomic_number: u8, used_valence: i32) -> Option<i32> {
    let (base_valence, max_valence) = third_period_main_group_valence_series(atomic_number)?;
    if used_valence >= max_valence {
        return Some(max_valence);
    }
    let mut target = base_valence;
    while target < used_valence {
        target += 2;
    }
    Some(target.min(max_valence))
}

fn symbol_hydrogen_charge_penalty(atomic_number: u8, charge: i32) -> i32 {
    if third_period_main_group_valence_series(atomic_number).is_some() {
        charge.abs()
    } else if charge > 0 {
        0
    } else {
        charge.abs()
    }
}

fn typical_symbol_valence(
    atomic_number: u8,
    charge: i32,
    connection_order: i32,
    radical_count: i32,
) -> Option<i32> {
    if let Some(target_valence) = third_period_main_group_target_valence(
        atomic_number,
        connection_order + radical_count + charge.abs(),
    ) {
        return Some(target_valence);
    }

    match atomic_number {
        5 => Some(if charge == -1 { 4 } else { 3 }),
        7 => {
            if charge == 1 {
                Some(4)
            } else if charge < 0 {
                Some(3)
            } else if charge == 2 || radical_count + connection_order + charge.abs() <= 3 {
                Some(3)
            } else {
                Some(5)
            }
        }
        8 => Some(if charge >= 1 { 3 } else { 2 }),
        9 => Some(1),
        _ => None,
    }
}

fn attached_symbol_charge_sum(node: &Node) -> i32 {
    node_attached_electron_symbols(node)
        .iter()
        .filter_map(|value| value.get("chargeDelta").and_then(Value::as_i64))
        .map(|value| value as i32)
        .sum()
}

fn attached_symbol_radical_sum(node: &Node) -> i32 {
    node_attached_electron_symbols(node)
        .iter()
        .filter_map(|value| value.get("radicalDelta").and_then(Value::as_i64))
        .map(|value| value as i32)
        .sum()
}

fn nearest_attachment_candidate(
    fragment: &crate::MoleculeFragment,
    object_translate: [f64; 2],
    point: Point,
) -> Option<AttachmentCandidate> {
    fragment
        .nodes
        .iter()
        .map(|node| best_candidate_for_node(node, object_translate, point))
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
}

fn best_candidate_for_node(
    node: &Node,
    object_translate: [f64; 2],
    point: Point,
) -> AttachmentCandidate {
    let node_point = Point::new(
        object_translate[0] + node.position[0],
        object_translate[1] + node.position[1],
    );
    let mut best = AttachmentCandidate {
        node_id: node.id.clone(),
        source: "endpoint",
        distance: point.distance(node_point),
    };
    if let Some(label) = &node.label {
        if let Some(bbox) = label.bbox() {
            let world_box = [
                bbox[0] + object_translate[0],
                bbox[1] + object_translate[1],
                bbox[2] + object_translate[0],
                bbox[3] + object_translate[1],
            ];
            let distance = point_to_box_distance(point, world_box);
            if distance < best.distance {
                best = AttachmentCandidate {
                    node_id: node.id.clone(),
                    source: "label",
                    distance,
                };
            }
        }
    }
    best
}

fn point_to_box_distance(point: Point, bbox: [f64; 4]) -> f64 {
    let dx = if point.x < bbox[0] {
        bbox[0] - point.x
    } else if point.x > bbox[2] {
        point.x - bbox[2]
    } else {
        0.0
    };
    let dy = if point.y < bbox[1] {
        bbox[1] - point.y
    } else if point.y > bbox[3] {
        point.y - bbox[3]
    } else {
        0.0
    };
    dx.hypot(dy)
}

fn scene_object_center(object: &SceneObject) -> Option<Point> {
    let [x, y, width, height] = object.payload.bbox?;
    Some(Point::new(
        object.transform.translate[0] + x + width * 0.5,
        object.transform.translate[1] + y + height * 0.5,
    ))
}

fn set_payload_value(object: &mut SceneObject, key: &str, value: Option<Value>) -> bool {
    match value {
        Some(value) => {
            if object.payload.extra.get(key) == Some(&value) {
                false
            } else {
                object.payload.extra.insert(key.to_string(), value);
                true
            }
        }
        None => object.payload.extra.remove(key).is_some(),
    }
}

fn set_node_meta_value(node: &mut Node, key: &str, value: Option<Value>) -> bool {
    if !node.meta.is_object() {
        if value.is_none() {
            return false;
        }
        node.meta = json!({});
    }
    let Some(object) = node.meta.as_object_mut() else {
        return false;
    };
    match value {
        Some(value) => {
            if object.get(key) == Some(&value) {
                false
            } else {
                object.insert(key.to_string(), value);
                true
            }
        }
        None => object.remove(key).is_some(),
    }
}

pub fn cdxml_symbol_style_from_line_width(line_width: f64) -> CdxmlSymbolStyle {
    if line_width <= 0.75 {
        CdxmlSymbolStyle::Acs
    } else {
        CdxmlSymbolStyle::Default
    }
}

pub fn cdxml_symbol_style_name(style: CdxmlSymbolStyle) -> &'static str {
    match style {
        CdxmlSymbolStyle::Default => "default",
        CdxmlSymbolStyle::Acs => "acs",
    }
}

pub fn cdxml_symbol_style_from_name(value: &str) -> CdxmlSymbolStyle {
    if value.eq_ignore_ascii_case("acs") {
        CdxmlSymbolStyle::Acs
    } else {
        CdxmlSymbolStyle::Default
    }
}

pub fn default_cdxml_symbol_metrics(kind: &str) -> CdxmlSymbolMetrics {
    cdxml_symbol_metrics_for_line_width(kind, 1.0)
}

pub fn cdxml_symbol_metrics(kind: &str, style: CdxmlSymbolStyle) -> CdxmlSymbolMetrics {
    cdxml_symbol_metrics_for_line_width(
        kind,
        match style {
            CdxmlSymbolStyle::Default => 1.0,
            CdxmlSymbolStyle::Acs => 0.6,
        },
    )
}

pub fn cdxml_symbol_metrics_for_line_width(kind: &str, line_width: f64) -> CdxmlSymbolMetrics {
    let style = cdxml_symbol_style_from_line_width(line_width);
    let anchor_width = cdxml_symbol_anchor_width(kind, style);
    let anchor_height = cdxml_symbol_anchor_height(kind);
    cdxml_symbol_metrics_from_anchor(kind, anchor_width, anchor_height, line_width)
}

pub fn cdxml_symbol_metrics_from_bbox(
    kind: &str,
    bbox: [f64; 4],
    line_width: f64,
) -> CdxmlSymbolMetrics {
    cdxml_symbol_metrics_from_anchor(
        kind,
        (bbox[2] - bbox[0]).abs(),
        (bbox[3] - bbox[1]).abs(),
        line_width,
    )
}

pub fn cdxml_symbol_metrics_from_anchor(
    kind: &str,
    anchor_width: f64,
    anchor_height: f64,
    line_width: f64,
) -> CdxmlSymbolMetrics {
    let style = cdxml_symbol_style_from_line_width(line_width);
    let anchor_width = if anchor_width > 0.0 {
        anchor_width
    } else {
        cdxml_symbol_anchor_width(kind, style)
    };
    let anchor_height = if anchor_height > 0.0 {
        anchor_height
    } else {
        cdxml_symbol_anchor_height(kind)
    };
    let (width, height, stroke_width) = match style {
        CdxmlSymbolStyle::Default => match kind {
            "double-dagger" | "dagger" => (4.0, 7.0, None),
            "circle-plus" | "circle-minus" => (
                anchor_height,
                anchor_height,
                Some(symbol_stroke_width(line_width)),
            ),
            "plus" => (4.3335, 4.3335, None),
            "minus" => (4.3335, 0.8, None),
            "radical-cation" => (6.75, 4.333, None),
            "radical-anion" => (6.75, 1.6665, None),
            "lone-pair" => (5.417, 1.6665, None),
            "electron" => {
                let diameter = cdxml_electron_symbol_diameter(anchor_height);
                (diameter, diameter, None)
            }
            _ => (8.0, 8.0, None),
        },
        CdxmlSymbolStyle::Acs => match kind {
            "double-dagger" | "dagger" => (3.6, 6.6, None),
            "circle-plus" | "circle-minus" => {
                let diameter = (anchor_height - 0.32).max(anchor_height * 0.5);
                (diameter, diameter, Some(symbol_stroke_width(line_width)))
            }
            "plus" => (3.9335, 3.9335, None),
            "minus" => (3.9335, 0.5, None),
            "radical-cation" => (3.3, 2.2, None),
            "radical-anion" => (3.3, 0.8, None),
            "lone-pair" => (2.6, 0.8, None),
            "electron" => {
                let diameter = cdxml_electron_symbol_diameter(anchor_height);
                (diameter, diameter, None)
            }
            _ => (8.0, 8.0, None),
        },
    };
    CdxmlSymbolMetrics {
        width,
        height,
        stroke_width,
        cdxml_anchor_width: anchor_width,
        cdxml_anchor_height: anchor_height,
        line_width,
    }
}

pub fn symbol_stroke_width(line_width: f64) -> f64 {
    (line_width * 0.8).max(0.5)
}

fn cdxml_electron_symbol_diameter(anchor_height: f64) -> f64 {
    // ChemDraw's exported SVG doubles CDXML coordinates, so its 4/9 SVG-space
    // electron diameter maps back to 2/9 in document coordinates.
    anchor_height * 2.0 / 9.0
}

pub fn cdxml_symbol_anchor_width(kind: &str, style: CdxmlSymbolStyle) -> f64 {
    match kind {
        "radical-cation" | "radical-anion" | "lone-pair" => match style {
            CdxmlSymbolStyle::Default => 3.75,
            CdxmlSymbolStyle::Acs => 1.8,
        },
        _ => 0.0,
    }
}

pub fn cdxml_symbol_anchor_height(kind: &str) -> f64 {
    match kind {
        "radical-cation" | "radical-anion" | "lone-pair" => 0.0,
        _ => 7.5,
    }
}
