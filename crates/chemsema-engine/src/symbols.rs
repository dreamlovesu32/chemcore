use crate::{
    AtomRadical, ChemSemaDocument, LinkEndpoint, LinkPolicy, LinkRelation, Node, Point, SceneObject,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const ELECTRON_SYMBOL_ATTACH_RADIUS: f64 = 10.0;
const ATTACHED_ELECTRON_SYMBOLS_META: &str = "attachedElectronSymbols";
const SYMBOL_BASE_CHARGE_META: &str = "symbolBaseCharge";
const SYMBOL_BASE_RADICAL_META: &str = "symbolBaseRadicalCount";
const RADICAL_COUNT_META: &str = "radicalCount";
const CHARGE_SYMBOL_INVALID_META: &str = "chargeSymbolInvalid";
const EFFECTIVE_NUM_HYDROGENS_META: &str = "effectiveNumHydrogens";
const USER_NUM_HYDROGENS_OVERRIDE_META: &str = "numHydrogensOverride";

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
    represent_attribute: Option<String>,
}

#[derive(Debug, Clone)]
struct AttachmentCandidate {
    node_id: String,
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

pub fn refresh_attached_electron_symbols(document: &mut ChemSemaDocument) -> bool {
    refresh_attached_electron_symbols_with_mode(document, true)
}

pub fn refresh_loaded_electron_symbols(document: &mut ChemSemaDocument) -> bool {
    refresh_attached_electron_symbols_with_mode(document, false)
}

fn refresh_attached_electron_symbols_with_mode(
    document: &mut ChemSemaDocument,
    allow_new_spatial_auto_links: bool,
) -> bool {
    let mut changed = refresh_atom_symbol_links(document, allow_new_spatial_auto_links);
    let attachments = detect_symbol_attachments(document);
    changed |= refresh_symbol_object_attachment_payloads(document, &attachments);
    changed |= refresh_node_attached_electron_symbols(document, &attachments);
    changed
}

pub fn node_radical_count(node: &Node) -> i32 {
    node.meta
        .get(RADICAL_COUNT_META)
        .and_then(Value::as_i64)
        .map(|value| value as i32)
        .unwrap_or_else(|| node.atom_properties.radical.electron_count())
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
    node_user_num_hydrogens_override(node).or_else(|| {
        node.meta
            .get(EFFECTIVE_NUM_HYDROGENS_META)
            .and_then(Value::as_u64)
            .map(|value| value.min(u64::from(u8::MAX)) as u8)
    })
}

pub fn node_user_num_hydrogens_override(node: &Node) -> Option<u8> {
    node.meta
        .get(USER_NUM_HYDROGENS_OVERRIDE_META)
        .and_then(Value::as_u64)
        .map(|value| value.min(u64::from(u8::MAX)) as u8)
}

pub fn set_node_user_num_hydrogens_override(node: &mut Node, count: Option<u8>) -> bool {
    set_node_meta_value(
        node,
        USER_NUM_HYDROGENS_OVERRIDE_META,
        count.map(|count| json!(count)),
    )
}

pub fn node_has_charge_symbol_invalid(node: &Node) -> bool {
    node.meta
        .get(CHARGE_SYMBOL_INVALID_META)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn detect_symbol_attachments(document: &ChemSemaDocument) -> Vec<SymbolAttachment> {
    if document.editable_fragments().is_empty() {
        return Vec::new();
    }
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
        let represent_attribute = object
            .payload
            .extra
            .get("representAttribute")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let Some(center) = scene_object_center(object) else {
            continue;
        };
        let linked_node_id = document
            .links
            .iter()
            .filter(|relation| relation.kind == "atom-symbol")
            .find(|relation| {
                relation
                    .endpoints
                    .iter()
                    .any(|endpoint| endpoint.role == "symbol" && endpoint.entity_id == object.id)
            })
            .map(|relation| {
                let node_id = relation
                    .endpoints
                    .iter()
                    .find(|endpoint| endpoint.role == "atom")
                    .map(|endpoint| endpoint.entity_id.clone());
                (node_id, relation.data.clone())
            });
        let (node_id, attachment_source, attachment_distance) =
            if let Some((Some(node_id), relation_data)) = linked_node_id {
                let distance = attachment_candidate_for_node(document, &node_id, center)
                    .map(|candidate| candidate.distance);
                let source = if object.link_policy == LinkPolicy::Linked {
                    "explicit"
                } else if relation_data.get("source").and_then(Value::as_str)
                    == Some("cdxml-represent")
                {
                    "source"
                } else {
                    "auto"
                };
                (Some(node_id), Some(source), distance)
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
            represent_attribute,
        });
    }
    out
}

fn refresh_atom_symbol_links(
    document: &mut ChemSemaDocument,
    allow_new_spatial_auto_links: bool,
) -> bool {
    if document.editable_fragments().is_empty() {
        return false;
    }
    let candidates = document
        .objects
        .iter()
        .filter(|object| {
            object.object_type == "symbol"
                && object.visible
                && electron_symbol_chemistry(
                    object
                        .payload
                        .extra
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                )
                .is_some()
        })
        .map(|object| {
            let explicit_target = object
                .payload
                .extra
                .get("representObjectId")
                .and_then(Value::as_str);
            let candidate = scene_object_center(object).and_then(|center| {
                if let Some(node_id) = explicit_target {
                    attachment_candidate_for_node(document, node_id, center)
                } else if allow_new_spatial_auto_links {
                    nearest_attachment_candidate(document, center)
                        .filter(|candidate| candidate.distance <= ELECTRON_SYMBOL_ATTACH_RADIUS)
                } else {
                    None
                }
            });
            (
                object.id.clone(),
                object.link_policy,
                candidate,
                explicit_target.is_some(),
            )
        })
        .collect::<Vec<_>>();
    let auto_symbols = candidates
        .iter()
        .filter(|(_, policy, _, explicit_target)| {
            *policy == LinkPolicy::Auto && (allow_new_spatial_auto_links || *explicit_target)
        })
        .map(|(id, _, _, _)| id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let unlinked_symbols = candidates
        .iter()
        .filter(|(_, policy, _, _)| *policy == LinkPolicy::Unlinked)
        .map(|(id, _, _, _)| id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let before = document.links.len();
    document.links.retain(|relation| {
        if relation.kind != "atom-symbol" {
            return true;
        }
        let symbol_id = relation
            .endpoints
            .iter()
            .find(|endpoint| endpoint.role == "symbol")
            .map(|endpoint| endpoint.entity_id.as_str());
        !symbol_id.is_some_and(|id| auto_symbols.contains(id) || unlinked_symbols.contains(id))
    });
    let mut changed = before != document.links.len();
    for (symbol_id, policy, candidate, explicit_target) in candidates {
        if policy != LinkPolicy::Auto {
            continue;
        }
        let Some(candidate) = candidate else {
            continue;
        };
        document.links.push(LinkRelation {
            id: format!("link_auto_atom_{}_{}", symbol_id, candidate.node_id),
            kind: "atom-symbol".to_string(),
            endpoints: vec![
                LinkEndpoint {
                    entity_id: candidate.node_id,
                    role: "atom".to_string(),
                },
                LinkEndpoint {
                    entity_id: symbol_id,
                    role: "symbol".to_string(),
                },
            ],
            data: if explicit_target {
                json!({"source": "cdxml-represent"})
            } else {
                Value::Null
            },
        });
        changed = true;
    }
    changed
}

fn refresh_symbol_object_attachment_payloads(
    document: &mut ChemSemaDocument,
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
    document: &mut ChemSemaDocument,
    attachments: &[SymbolAttachment],
) -> bool {
    let resource_refs = document
        .editable_fragments()
        .iter()
        .filter_map(|entry| entry.object.payload.resource_ref.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut changed = false;
    for resource_ref in resource_refs {
        let Some(fragment) = document
            .resources
            .get_mut(&resource_ref)
            .and_then(|resource| resource.data.as_fragment_mut())
        else {
            continue;
        };
        changed |= refresh_fragment_attached_electron_symbols(fragment, attachments);
    }
    changed
}

fn refresh_fragment_attached_electron_symbols(
    fragment: &mut crate::MoleculeFragment,
    attachments: &[SymbolAttachment],
) -> bool {
    let mut by_node: BTreeMap<&str, Vec<&SymbolAttachment>> = BTreeMap::new();
    for attachment in attachments {
        if let Some(node_id) = attachment.node_id.as_deref() {
            by_node.entry(node_id).or_default().push(attachment);
        }
    }
    let mut connection_orders: BTreeMap<&str, i32> = BTreeMap::new();
    for bond in &fragment.bonds {
        let order = i32::from(bond.order.max(1));
        *connection_orders.entry(bond.begin.as_str()).or_default() += order;
        *connection_orders.entry(bond.end.as_str()).or_default() += order;
    }

    let mut changed = false;
    for node in &mut fragment.nodes {
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
            .map(|attachment| attachment_charge_delta(attachment, base_charge))
            .sum();
        let radical_delta: i32 = node_attachments
            .iter()
            .map(|attachment| attachment_radical_delta(attachment, base_radical))
            .sum();
        let next_charge = base_charge + charge_delta;
        if node.charge != next_charge {
            node.charge = next_charge;
            changed = true;
        }
        let attached_values: Vec<Value> = node_attachments
            .iter()
            .map(|attachment| {
                let charge_delta = attachment_charge_delta(attachment, base_charge);
                let radical_delta = attachment_radical_delta(attachment, base_radical);
                json!({
                    "symbolObjectId": attachment.symbol_object_id,
                    "sourceSymbolObjectId": attachment.symbol_object_id,
                    "kind": attachment.kind,
                    "chargeDelta": charge_delta,
                    "radicalDelta": radical_delta,
                    "requiresHydrogenRemoval": attachment_requires_hydrogen_removal(
                        attachment,
                        base_charge,
                        base_radical
                    ),
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
        let next_radical_kind = match next_radical {
            i32::MIN..=0 => AtomRadical::None,
            1 => AtomRadical::Doublet,
            _ if base_radical == 2 && node.atom_properties.radical != AtomRadical::None => {
                node.atom_properties.radical
            }
            _ if node_attachments
                .iter()
                .any(|attachment| attachment.kind == "lone-pair") =>
            {
                AtomRadical::Singlet
            }
            _ => AtomRadical::Triplet,
        };
        if node.atom_properties.radical != next_radical_kind {
            node.atom_properties.radical = next_radical_kind;
            changed = true;
        }
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
    if attachments.iter().all(|attachment| {
        attachment_charge_delta(attachment, base_charge) == 0
            && attachment_radical_delta(attachment, base_radical) == 0
    }) {
        // A represent child can point at an attribute already carried by the
        // node. Such a symbol visualizes that state; it does not create a
        // second valence change or replace the node's ordinary hydrogen rule.
        return (None, false);
    }
    if node.atomic_number != 6 {
        let ordinary_negative_removals = attachments
            .iter()
            .filter(|attachment| {
                attachment_charge_delta(attachment, base_charge) < 0
                    && attachment_radical_delta(attachment, base_radical) == 0
                    && attachment_requires_hydrogen_removal(attachment, base_charge, base_radical)
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
        .filter(|attachment| {
            attachment_requires_hydrogen_removal(attachment, base_charge, base_radical)
        })
        .count() as i32;
    if required_removals == 0 {
        return (Some(neutral_hydrogens as u8), false);
    }
    if neutral_hydrogens < required_removals {
        return (Some(neutral_hydrogens as u8), true);
    }
    (Some((neutral_hydrogens - required_removals) as u8), false)
}

fn attachment_charge_delta(attachment: &SymbolAttachment, base_charge: i32) -> i32 {
    if attachment_represents_attribute(attachment, "charge") && base_charge != 0 {
        0
    } else {
        attachment.chemistry.charge_delta
    }
}

fn attachment_radical_delta(attachment: &SymbolAttachment, base_radical: i32) -> i32 {
    if attachment_represents_attribute(attachment, "radical") && base_radical != 0 {
        0
    } else {
        attachment.chemistry.radical_delta
    }
}

fn attachment_requires_hydrogen_removal(
    attachment: &SymbolAttachment,
    base_charge: i32,
    base_radical: i32,
) -> bool {
    attachment.chemistry.requires_hydrogen_removal
        && (attachment_charge_delta(attachment, base_charge) != 0
            || attachment_radical_delta(attachment, base_radical) != 0)
}

fn attachment_represents_attribute(attachment: &SymbolAttachment, attribute: &str) -> bool {
    attachment
        .represent_attribute
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(attribute))
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
            } else {
                Some(3)
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
    document: &ChemSemaDocument,
    point: Point,
) -> Option<AttachmentCandidate> {
    let mut candidates = document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| {
            entry.fragment.nodes.iter().map(move |node| {
                best_candidate_for_node(node, entry.object.transform.translate, point)
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.distance.total_cmp(&right.distance));
    let nearest = candidates.first()?.clone();
    if candidates
        .get(1)
        .is_some_and(|next| (next.distance - nearest.distance).abs() <= 1.0e-6)
    {
        return None;
    }
    Some(nearest)
}

fn attachment_candidate_for_node(
    document: &ChemSemaDocument,
    node_id: &str,
    point: Point,
) -> Option<AttachmentCandidate> {
    document
        .editable_fragments()
        .into_iter()
        .flat_map(|entry| {
            entry
                .fragment
                .nodes
                .iter()
                .filter(move |node| node.id == node_id)
                .map(move |node| {
                    best_candidate_for_node(node, entry.object.transform.translate, point)
                })
        })
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
            "circle-plus" | "circle-minus" => {
                // ChemDraw treats a symbol graphic's vertical BoundingBox as
                // an anchor, not the circle centerline diameter. Default-style
                // charge circles are 0.30 pt smaller than that anchor.
                let diameter = (anchor_height - 0.30).max(anchor_height * 0.5);
                (diameter, diameter, Some(symbol_stroke_width(line_width)))
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_circled_charge_uses_chemdraw_anchor_inset() {
        let metrics = cdxml_symbol_metrics_from_bbox("circle-minus", [0.0, 0.0, 0.0, 10.0], 1.0);
        assert!((metrics.width - 9.70).abs() < 1.0e-9, "{metrics:?}");
        assert!((metrics.height - 9.70).abs() < 1.0e-9, "{metrics:?}");
        assert_eq!(metrics.cdxml_anchor_height, 10.0);
    }

    #[test]
    fn second_period_symbol_valence_does_not_expand_octet() {
        assert_eq!(typical_symbol_valence(7, 0, 4, 0), Some(3));
        assert_eq!(supported_hetero_hydrogens(7, 0, 4, 0), 0);
        assert_eq!(typical_symbol_valence(7, 1, 4, 0), Some(4));
        assert_eq!(typical_symbol_valence(5, 0, 4, 0), Some(3));
        assert_eq!(typical_symbol_valence(5, -1, 4, 0), Some(4));
        assert_eq!(typical_symbol_valence(8, 0, 3, 0), Some(2));
        assert_eq!(typical_symbol_valence(8, 1, 3, 0), Some(3));
    }
}
