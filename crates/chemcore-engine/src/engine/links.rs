use super::{EditorCommand, Engine};
use crate::{refresh_repeating_units, ChemcoreDocument, Point, SceneObject, SelectionState};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const IMPORT_COUNT_LABEL_SEARCH_PAD: f64 = 50.0;
const BOUNDS_EPSILON: f64 = 1e-6;

impl Engine {
    pub fn selection_can_link_bracket_text(&self) -> bool {
        let Some(pair) = selected_bracket_text_pair(&self.state.document, &self.state.selection)
        else {
            return false;
        };
        !objects_are_linked(&self.state.document, &pair.bracket_id, &pair.text_id)
    }

    pub fn selection_can_unlink_bracket_text(&self) -> bool {
        let Some(pair) = selected_bracket_text_pair(&self.state.document, &self.state.selection)
        else {
            return false;
        };
        objects_are_linked(&self.state.document, &pair.bracket_id, &pair.text_id)
    }

    pub fn link_selection(&mut self) -> bool {
        let object_ids = selected_scene_object_ids(&self.state.selection);
        self.with_command(EditorCommand::LinkSelection { object_ids }, |engine| {
            engine.link_selection_untracked()
        })
    }

    fn link_selection_untracked(&mut self) -> bool {
        let Some(pair) = selected_bracket_text_pair(&self.state.document, &self.state.selection)
        else {
            return false;
        };
        if objects_are_linked(&self.state.document, &pair.bracket_id, &pair.text_id) {
            return false;
        }
        self.push_undo_snapshot();
        if !self.link_bracket_text_objects_untracked(&pair.bracket_id, &pair.text_id) {
            self.undo_stack.pop();
            return false;
        }
        true
    }

    pub fn unlink_selection(&mut self) -> bool {
        let object_ids = selected_scene_object_ids(&self.state.selection);
        self.with_command(EditorCommand::UnlinkSelection { object_ids }, |engine| {
            engine.unlink_selection_untracked()
        })
    }

    fn unlink_selection_untracked(&mut self) -> bool {
        let Some(pair) = selected_bracket_text_pair(&self.state.document, &self.state.selection)
        else {
            return false;
        };
        if !objects_are_linked(&self.state.document, &pair.bracket_id, &pair.text_id) {
            return false;
        }
        self.push_undo_snapshot();
        if !self.unlink_bracket_text_objects_untracked(&pair.bracket_id, &pair.text_id) {
            self.undo_stack.pop();
            return false;
        }
        true
    }

    pub(super) fn link_bracket_text_objects_untracked(
        &mut self,
        bracket_id: &str,
        text_id: &str,
    ) -> bool {
        let mut changed = false;
        if let Some(bracket) = self.state.document.find_scene_object_mut(bracket_id) {
            if scene_object_is_bracket_like(bracket) {
                changed |= set_meta_object_field(
                    &mut bracket.meta,
                    "linkedTextObjectId",
                    Some(json!(text_id)),
                );
            }
        }
        if let Some(text) = self.state.document.find_scene_object_mut(text_id) {
            if text.object_type == "text" {
                changed |=
                    set_meta_object_field(&mut text.meta, "linkKind", Some(json!("bracket-label")));
                changed |= set_meta_object_field(
                    &mut text.meta,
                    "linkedBracketObjectId",
                    Some(json!(bracket_id)),
                );
                changed |= set_meta_object_field(&mut text.meta, "repeatUnitDetached", None);
                changed |= set_meta_object_field(&mut text.meta, "bracketObjectId", None);
            }
        }
        changed |= refresh_repeating_units(&mut self.state.document);
        changed
    }

    pub(super) fn unlink_bracket_text_objects_untracked(
        &mut self,
        bracket_id: &str,
        text_id: &str,
    ) -> bool {
        let mut changed = false;
        if let Some(bracket) = self.state.document.find_scene_object_mut(bracket_id) {
            if scene_object_is_bracket_like(bracket) {
                changed |= set_meta_object_field(&mut bracket.meta, "linkedTextObjectId", None);
                changed |=
                    set_meta_object_field(&mut bracket.meta, "bracketLabelTextObjectId", None);
            }
        }
        if let Some(text) = self.state.document.find_scene_object_mut(text_id) {
            if text.object_type == "text" {
                changed |= set_meta_object_field(&mut text.meta, "linkKind", None);
                changed |= set_meta_object_field(&mut text.meta, "linkedBracketObjectId", None);
                changed |= set_meta_object_field(&mut text.meta, "bracketObjectId", None);
                changed |= set_meta_object_field(&mut text.meta, "repeatUnitDetached", None);
            }
        }
        changed |= refresh_repeating_units(&mut self.state.document);
        changed
    }

    pub(super) fn link_imported_repeat_unit_labels_untracked(&mut self) -> bool {
        let pairs = imported_repeat_unit_label_pairs(&self.state.document);
        let mut changed = false;
        for (bracket_id, text_id) in pairs {
            if objects_are_linked(&self.state.document, &bracket_id, &text_id) {
                continue;
            }
            changed |= self.link_bracket_text_objects_untracked(&bracket_id, &text_id);
        }
        changed
    }
}

#[derive(Debug, Clone)]
struct SelectedBracketTextPair {
    bracket_id: String,
    text_id: String,
}

fn selected_scene_object_ids(selection: &SelectionState) -> Vec<String> {
    selection
        .arrow_objects
        .iter()
        .chain(selection.text_objects.iter())
        .cloned()
        .collect()
}

fn selected_bracket_text_pair(
    document: &ChemcoreDocument,
    selection: &SelectionState,
) -> Option<SelectedBracketTextPair> {
    if selection.arrow_objects.len() != 1 || selection.text_objects.len() != 1 {
        return None;
    }
    let bracket_id = selection.arrow_objects[0].clone();
    let text_id = selection.text_objects[0].clone();
    let bracket = document.find_scene_object(&bracket_id)?;
    let text = document.find_scene_object(&text_id)?;
    (scene_object_is_bracket_like(bracket) && text.object_type == "text").then_some(
        SelectedBracketTextPair {
            bracket_id,
            text_id,
        },
    )
}

fn objects_are_linked(document: &ChemcoreDocument, bracket_id: &str, text_id: &str) -> bool {
    let bracket_matches = document
        .find_scene_object(bracket_id)
        .and_then(|object| object.meta.get("linkedTextObjectId"))
        .and_then(Value::as_str)
        == Some(text_id);
    let text_matches = document
        .find_scene_object(text_id)
        .and_then(|object| object.meta.get("linkedBracketObjectId"))
        .and_then(Value::as_str)
        == Some(bracket_id);
    bracket_matches || text_matches
}

fn scene_object_is_bracket_like(object: &SceneObject) -> bool {
    object.object_type == "bracket"
        || (object.object_type == "group"
            && object.meta.get("kind").and_then(Value::as_str) == Some("bracket-group"))
}

fn imported_repeat_unit_label_pairs(document: &ChemcoreDocument) -> Vec<(String, String)> {
    let brackets = imported_bracket_candidates(document);
    let counts = imported_count_candidates(document);
    let mut used_text_ids = BTreeSet::new();
    let mut pairs = Vec::new();

    for bracket in brackets {
        let Some(count) = best_imported_count_for_bracket(&bracket, &counts, &used_text_ids) else {
            continue;
        };
        used_text_ids.insert(count.object_id.clone());
        pairs.push((bracket.object_id, count.object_id.clone()));
    }

    pairs
}

#[derive(Debug, Clone)]
struct ImportedBracketCandidate {
    object_id: String,
    bounds: [f64; 4],
}

#[derive(Debug, Clone)]
struct ImportedCountCandidate {
    object_id: String,
    bounds: [f64; 4],
}

fn imported_bracket_candidates(document: &ChemcoreDocument) -> Vec<ImportedBracketCandidate> {
    document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "bracket" && object.visible)
        .filter_map(|object| {
            Some(ImportedBracketCandidate {
                object_id: object.id.clone(),
                bounds: object_world_bounds(object)?,
            })
        })
        .collect()
}

fn imported_count_candidates(document: &ChemcoreDocument) -> Vec<ImportedCountCandidate> {
    document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text" && object.visible)
        .filter_map(|object| {
            let text = payload_string(object, "text")?;
            let trimmed = text.trim();
            if trimmed.is_empty() || !trimmed.chars().all(|character| character.is_ascii_digit()) {
                return None;
            }
            let value = trimmed.parse::<u32>().ok()?;
            if value < 2 {
                return None;
            }
            Some(ImportedCountCandidate {
                object_id: object.id.clone(),
                bounds: object_world_bounds(object)?,
            })
        })
        .collect()
}

fn best_imported_count_for_bracket<'a>(
    bracket: &ImportedBracketCandidate,
    counts: &'a [ImportedCountCandidate],
    used_text_ids: &BTreeSet<String>,
) -> Option<&'a ImportedCountCandidate> {
    let [left, top, right, bottom] = bracket.bounds;
    let min_x = right - ((right - left).abs() * 0.35).max(16.0);
    let max_x = right + IMPORT_COUNT_LABEL_SEARCH_PAD;
    let min_y = bottom - ((bottom - top).abs() * 0.35).max(16.0);
    let max_y = bottom + IMPORT_COUNT_LABEL_SEARCH_PAD;
    let anchor = Point::new(right, bottom);
    counts
        .iter()
        .filter(|count| !used_text_ids.contains(&count.object_id))
        .filter(|count| {
            let center = bounds_center(count.bounds);
            center.x >= min_x && center.x <= max_x && center.y >= min_y && center.y <= max_y
        })
        .min_by(|left_count, right_count| {
            bounds_center(left_count.bounds)
                .distance(anchor)
                .total_cmp(&bounds_center(right_count.bounds).distance(anchor))
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

fn payload_string(object: &SceneObject, key: &str) -> Option<String> {
    object
        .payload
        .extra
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn bounds_center(bounds: [f64; 4]) -> Point {
    Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
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
