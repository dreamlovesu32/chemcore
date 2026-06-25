use crate::{ChemcoreDocument, MoleculeFragment, Node, Point, SceneObject};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const BOUNDS_EPSILON: f64 = 1e-6;

#[derive(Debug, Clone)]
struct BracketCandidate {
    object_id: String,
    kind: String,
    repeat_count: Option<u32>,
    bounds: [f64; 4],
}

#[derive(Debug, Clone)]
struct RepeatCountCandidate {
    object_id: Option<String>,
    linked_bracket_object_id: Option<String>,
    text: String,
    value: u32,
    bounds: [f64; 4],
}

#[derive(Debug, Clone)]
struct BoundaryBond {
    role: &'static str,
    bond_id: String,
    internal_atom_id: String,
    external_atom_id: String,
    order: u8,
}

#[derive(Debug, Clone)]
struct RepeatingUnitPlan {
    unit_id: String,
    bracket_object_id: String,
    bracket_kind: String,
    count_text_object_id: Option<String>,
    count_text: String,
    repeat_count: u32,
    atom_ids: Vec<String>,
    internal_bond_ids: Vec<String>,
    left_boundary: BoundaryBond,
    right_boundary: BoundaryBond,
    expansion: Value,
}

pub fn refresh_repeating_units(document: &mut ChemcoreDocument) -> bool {
    let plans = detect_repeating_units(document);
    let next_units: Vec<Value> = plans.iter().map(repeating_unit_json).collect();
    let mut changed = false;

    for_each_scene_object_mut(&mut document.objects, &mut |object| {
        if matches!(object.object_type.as_str(), "bracket" | "text") {
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitId", None);
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitRole", None);
        }
        if object.object_type == "group" {
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitId", None);
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitRole", None);
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitGroup", None);
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitBracketObjectId", None);
            changed |= set_meta_object_field(&mut object.meta, "repeatUnitCountTextObjectId", None);
        }
    });

    for plan in &plans {
        if let Some(object) = document.find_scene_object_mut(&plan.bracket_object_id) {
            changed |=
                set_meta_object_field(&mut object.meta, "repeatUnitId", Some(json!(plan.unit_id)));
            changed |=
                set_meta_object_field(&mut object.meta, "repeatUnitRole", Some(json!("bracket")));
        }
        if let Some(count_text_object_id) = plan.count_text_object_id.as_deref() {
            if let Some(object) = document.find_scene_object_mut(count_text_object_id) {
                changed |= set_meta_object_field(
                    &mut object.meta,
                    "repeatUnitId",
                    Some(json!(plan.unit_id)),
                );
                changed |=
                    set_meta_object_field(&mut object.meta, "repeatUnitRole", Some(json!("count")));
            }
        }
    }
    if let Some(entry) = document.editable_fragment_mut() {
        changed |= set_meta_object_field(
            &mut entry.fragment.meta,
            "repeatingUnits",
            if next_units.is_empty() {
                None
            } else {
                Some(Value::Array(next_units))
            },
        );
    }

    changed
}

fn detect_repeating_units(document: &ChemcoreDocument) -> Vec<RepeatingUnitPlan> {
    let Some(entry) = document.editable_fragment() else {
        return Vec::new();
    };
    let fragment = entry.fragment;
    let object_translate = entry.object.transform.translate;
    let brackets = bracket_candidates(document);
    let counts = text_count_candidates(document);
    let mut used_count_texts = BTreeSet::new();
    let mut plans = Vec::new();

    for bracket in brackets {
        let inline_count;
        let count = if let Some(count) =
            best_count_label_for_bracket(&bracket, &counts, &used_count_texts)
        {
            count
        } else if let Some(value) = bracket.repeat_count {
            inline_count = RepeatCountCandidate {
                object_id: None,
                linked_bracket_object_id: None,
                text: value.to_string(),
                value,
                bounds: bracket.bounds,
            };
            &inline_count
        } else {
            continue;
        };
        let Some(plan) = build_repeating_unit_plan(fragment, object_translate, &bracket, count)
        else {
            continue;
        };
        if let Some(object_id) = count.object_id.as_deref() {
            used_count_texts.insert(object_id.to_string());
        }
        plans.push(plan);
    }

    plans
}

fn bracket_candidates(document: &ChemcoreDocument) -> Vec<BracketCandidate> {
    document
        .scene_objects()
        .into_iter()
        .filter(|object| scene_object_is_bracket_candidate(object) && object.visible)
        .filter_map(|object| {
            let bounds = object_world_bounds(object)?;
            Some(BracketCandidate {
                object_id: object.id.clone(),
                kind: bracket_candidate_kind(object),
                repeat_count: object
                    .meta
                    .get("repeatCount")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .filter(|value| *value >= 2),
                bounds,
            })
        })
        .collect()
}

fn scene_object_is_bracket_candidate(object: &SceneObject) -> bool {
    object.object_type == "bracket"
        || (object.object_type == "group"
            && object.meta.get("kind").and_then(Value::as_str) == Some("bracket-group"))
}

fn bracket_candidate_kind(object: &SceneObject) -> String {
    payload_string(object, "kind")
        .or_else(|| {
            object
                .children
                .iter()
                .find(|child| child.object_type == "bracket")
                .and_then(|child| payload_string(child, "kind"))
        })
        .unwrap_or_else(|| "round".to_string())
}

fn text_count_candidates(document: &ChemcoreDocument) -> Vec<RepeatCountCandidate> {
    document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text" && object.visible)
        .filter_map(|object| {
            if object.meta.get("linkKind").and_then(Value::as_str) != Some("bracket-label") {
                return None;
            }
            let linked_bracket_object_id = object
                .meta
                .get("linkedBracketObjectId")
                .and_then(Value::as_str)
                .map(ToString::to_string)?;
            let text = payload_string(object, "text")?;
            let trimmed = text.trim();
            if trimmed.is_empty() || !trimmed.chars().all(|character| character.is_ascii_digit()) {
                return None;
            }
            let value = trimmed.parse::<u32>().ok()?;
            if value < 2 {
                return None;
            }
            Some(RepeatCountCandidate {
                object_id: Some(object.id.clone()),
                linked_bracket_object_id: Some(linked_bracket_object_id),
                text: trimmed.to_string(),
                value,
                bounds: object_world_bounds(object)?,
            })
        })
        .collect()
}

fn best_count_label_for_bracket<'a>(
    bracket: &BracketCandidate,
    counts: &'a [RepeatCountCandidate],
    used_count_texts: &BTreeSet<String>,
) -> Option<&'a RepeatCountCandidate> {
    counts
        .iter()
        .filter(|count| {
            count
                .object_id
                .as_ref()
                .is_some_and(|object_id| !used_count_texts.contains(object_id))
        })
        .filter(|count| {
            count.linked_bracket_object_id.as_deref() == Some(bracket.object_id.as_str())
        })
        .min_by(|left_count, right_count| left_count.bounds[0].total_cmp(&right_count.bounds[0]))
}

fn build_repeating_unit_plan(
    fragment: &MoleculeFragment,
    object_translate: [f64; 2],
    bracket: &BracketCandidate,
    count: &RepeatCountCandidate,
) -> Option<RepeatingUnitPlan> {
    let internal_atom_ids: Vec<String> = fragment
        .nodes
        .iter()
        .filter(|node| {
            point_in_bounds(world_point_for_node(node, object_translate), bracket.bounds)
        })
        .map(|node| node.id.clone())
        .collect();
    if internal_atom_ids.is_empty() {
        return None;
    }
    let internal_atom_set: BTreeSet<&str> = internal_atom_ids.iter().map(String::as_str).collect();
    let internal_bond_ids: Vec<String> = fragment
        .bonds
        .iter()
        .filter(|bond| {
            internal_atom_set.contains(bond.begin.as_str())
                && internal_atom_set.contains(bond.end.as_str())
        })
        .map(|bond| bond.id.clone())
        .collect();

    let mut left_boundaries = Vec::new();
    let mut right_boundaries = Vec::new();
    let mut other_crossing_count = 0;
    for bond in &fragment.bonds {
        let begin_inside = internal_atom_set.contains(bond.begin.as_str());
        let end_inside = internal_atom_set.contains(bond.end.as_str());
        if begin_inside == end_inside {
            continue;
        }
        let (internal_id, external_id) = if begin_inside {
            (bond.begin.as_str(), bond.end.as_str())
        } else {
            (bond.end.as_str(), bond.begin.as_str())
        };
        let internal = fragment.nodes.iter().find(|node| node.id == internal_id)?;
        let external = fragment.nodes.iter().find(|node| node.id == external_id)?;
        let internal_point = world_point_for_node(internal, object_translate);
        let external_point = world_point_for_node(external, object_translate);
        let boundary = BoundaryBond {
            role: "",
            bond_id: bond.id.clone(),
            internal_atom_id: internal_id.to_string(),
            external_atom_id: external_id.to_string(),
            order: bond.order,
        };
        if crosses_left_boundary(internal_point, external_point, bracket.bounds) {
            left_boundaries.push(BoundaryBond {
                role: "left",
                ..boundary
            });
        } else if crosses_right_boundary(internal_point, external_point, bracket.bounds) {
            right_boundaries.push(BoundaryBond {
                role: "right",
                ..boundary
            });
        } else {
            other_crossing_count += 1;
        }
    }
    if left_boundaries.len() != 1 || right_boundaries.len() != 1 || other_crossing_count != 0 {
        return None;
    }
    let left_boundary = left_boundaries.remove(0);
    let right_boundary = right_boundaries.remove(0);
    if left_boundary.order != right_boundary.order {
        return None;
    }
    let expansion = build_repeating_unit_expansion(
        fragment,
        count.value,
        &internal_atom_ids,
        &internal_bond_ids,
        &left_boundary,
        &right_boundary,
    )?;
    let unit_id = format!("ru_{}", stable_id_fragment(&bracket.object_id));
    Some(RepeatingUnitPlan {
        unit_id,
        bracket_object_id: bracket.object_id.clone(),
        bracket_kind: bracket.kind.clone(),
        count_text_object_id: count.object_id.clone(),
        count_text: count.text.clone(),
        repeat_count: count.value,
        atom_ids: internal_atom_ids,
        internal_bond_ids,
        left_boundary,
        right_boundary,
        expansion,
    })
}

fn build_repeating_unit_expansion(
    fragment: &MoleculeFragment,
    repeat_count: u32,
    internal_atom_ids: &[String],
    internal_bond_ids: &[String],
    left_boundary: &BoundaryBond,
    right_boundary: &BoundaryBond,
) -> Option<Value> {
    let internal_atom_set: BTreeSet<&str> = internal_atom_ids.iter().map(String::as_str).collect();
    let mut atoms = Vec::new();
    let mut bonds = Vec::new();

    for repeat_index in 1..=repeat_count {
        for node_id in internal_atom_ids {
            let node = fragment.nodes.iter().find(|node| node.id == *node_id)?;
            atoms.push(json!({
                "id": expanded_atom_id(node_id, repeat_index),
                "element": node.element,
                "atomicNumber": node.atomic_number,
                "charge": node.charge,
                "numHydrogens": crate::engine::formula_hydrogen_count_for_node(fragment, node.id.as_str()),
                "radicalCount": crate::node_radical_count(node),
                "electronSymbols": crate::node_attached_electron_symbols(node),
                "sourceAtomId": node.id,
                "repeatIndex": repeat_index,
            }));
        }
        for bond_id in internal_bond_ids {
            let bond = fragment.bonds.iter().find(|bond| bond.id == *bond_id)?;
            if !internal_atom_set.contains(bond.begin.as_str())
                || !internal_atom_set.contains(bond.end.as_str())
            {
                return None;
            }
            bonds.push(json!({
                "id": format!("{}_r{}", bond.id, repeat_index),
                "begin": expanded_atom_id(&bond.begin, repeat_index),
                "end": expanded_atom_id(&bond.end, repeat_index),
                "order": bond.order,
                "sourceBondId": bond.id,
                "repeatIndex": repeat_index,
            }));
        }
    }

    for repeat_index in 1..repeat_count {
        bonds.push(json!({
            "id": format!("repeat_link_{}_{}", repeat_index, repeat_index + 1),
            "begin": expanded_atom_id(&right_boundary.internal_atom_id, repeat_index),
            "end": expanded_atom_id(&left_boundary.internal_atom_id, repeat_index + 1),
            "order": left_boundary.order,
            "sourceBoundaryBondIds": [left_boundary.bond_id, right_boundary.bond_id],
            "repeatLink": true,
            "fromRepeatIndex": repeat_index,
            "toRepeatIndex": repeat_index + 1,
        }));
    }

    Some(json!({
        "schema": "chemcore.repeatingUnitExpansion.v1",
        "complete": true,
        "count": repeat_count,
        "atoms": atoms,
        "bonds": bonds,
        "attachments": [
            {
                "role": "leftExternal",
                "atomId": expanded_atom_id(&left_boundary.internal_atom_id, 1),
                "externalAtomId": left_boundary.external_atom_id,
                "sourceBondId": left_boundary.bond_id,
                "order": left_boundary.order,
            },
            {
                "role": "rightExternal",
                "atomId": expanded_atom_id(&right_boundary.internal_atom_id, repeat_count),
                "externalAtomId": right_boundary.external_atom_id,
                "sourceBondId": right_boundary.bond_id,
                "order": right_boundary.order,
            }
        ],
    }))
}

fn repeating_unit_json(plan: &RepeatingUnitPlan) -> Value {
    json!({
        "schema": "chemcore.repeatingUnit.v1",
        "id": plan.unit_id,
        "kind": "multiple-group",
        "bracketKind": plan.bracket_kind,
        "bracketObjectId": plan.bracket_object_id,
        "countTextObjectId": plan.count_text_object_id,
        "repeatCount": {
            "kind": "integer",
            "value": plan.repeat_count,
            "sourceText": plan.count_text,
        },
        "atomIds": plan.atom_ids,
        "internalBondIds": plan.internal_bond_ids,
        "boundaryBonds": [
            boundary_bond_json(&plan.left_boundary),
            boundary_bond_json(&plan.right_boundary),
        ],
        "expansion": plan.expansion,
    })
}

fn boundary_bond_json(boundary: &BoundaryBond) -> Value {
    json!({
        "role": boundary.role,
        "bondId": boundary.bond_id,
        "internalAtomId": boundary.internal_atom_id,
        "externalAtomId": boundary.external_atom_id,
        "order": boundary.order,
    })
}

fn object_world_bounds(object: &SceneObject) -> Option<[f64; 4]> {
    let [x, y, width, height] = object.payload.bbox.or_else(|| payload_box(object))?;
    if width <= BOUNDS_EPSILON || height <= BOUNDS_EPSILON {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    Some([tx + x, ty + y, tx + x + width, ty + y + height])
}

fn payload_string(object: &SceneObject, key: &str) -> Option<String> {
    object
        .payload
        .extra
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn payload_box(object: &SceneObject) -> Option<[f64; 4]> {
    let values = object.payload.extra.get("box")?.as_array()?;
    if values.len() != 4 {
        return None;
    }
    Some([
        values[0].as_f64()?,
        values[1].as_f64()?,
        values[2].as_f64()?,
        values[3].as_f64()?,
    ])
}

fn world_point_for_node(node: &Node, object_translate: [f64; 2]) -> Point {
    Point::new(
        node.position[0] + object_translate[0],
        node.position[1] + object_translate[1],
    )
}

fn point_in_bounds(point: Point, bounds: [f64; 4]) -> bool {
    point.x > bounds[0] + BOUNDS_EPSILON
        && point.x < bounds[2] - BOUNDS_EPSILON
        && point.y > bounds[1] + BOUNDS_EPSILON
        && point.y < bounds[3] - BOUNDS_EPSILON
}

fn crosses_left_boundary(internal: Point, external: Point, bounds: [f64; 4]) -> bool {
    external.x < bounds[0] - BOUNDS_EPSILON
        && segment_crosses_vertical_between_y(internal, external, bounds[0], bounds[1], bounds[3])
}

fn crosses_right_boundary(internal: Point, external: Point, bounds: [f64; 4]) -> bool {
    external.x > bounds[2] + BOUNDS_EPSILON
        && segment_crosses_vertical_between_y(internal, external, bounds[2], bounds[1], bounds[3])
}

fn segment_crosses_vertical_between_y(
    start: Point,
    end: Point,
    x: f64,
    min_y: f64,
    max_y: f64,
) -> bool {
    let dx = end.x - start.x;
    if dx.abs() <= BOUNDS_EPSILON {
        return false;
    }
    let t = (x - start.x) / dx;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let y = start.y + (end.y - start.y) * t;
    y >= min_y - BOUNDS_EPSILON && y <= max_y + BOUNDS_EPSILON
}

fn for_each_scene_object_mut(
    objects: &mut [SceneObject],
    visit: &mut impl FnMut(&mut SceneObject),
) {
    for object in objects {
        visit(object);
        for_each_scene_object_mut(&mut object.children, visit);
    }
}

fn expanded_atom_id(source_atom_id: &str, repeat_index: u32) -> String {
    format!("{source_atom_id}_r{repeat_index}")
}

fn stable_id_fragment(id: &str) -> String {
    id.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn set_meta_object_field(meta_value: &mut Value, key: &str, value: Option<Value>) -> bool {
    if !meta_value.is_object() {
        if value.is_none() {
            return false;
        }
        *meta_value = Value::Object(serde_json::Map::new());
    }
    let Some(object) = meta_value.as_object_mut() else {
        return false;
    };
    match value {
        Some(next) => {
            if object.get(key) == Some(&next) {
                return false;
            }
            object.insert(key.to_string(), next);
            true
        }
        None => {
            let changed = object.remove(key).is_some();
            if object.is_empty() {
                *meta_value = Value::Null;
            }
            changed
        }
    }
}
