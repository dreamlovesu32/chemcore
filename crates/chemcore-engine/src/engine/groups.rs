use super::{EditorCommand, Engine};
use crate::{ObjectPayload, SceneObject, SelectionState, Transform};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const STACK_STEP: i32 = 10;

impl Engine {
    pub fn group_selection(&mut self) -> bool {
        let object_ids = selected_scene_object_ids(&self.state.selection)
            .into_iter()
            .collect();
        self.with_command(EditorCommand::GroupSelection { object_ids }, |engine| {
            engine.group_selection_untracked()
        })
    }

    fn group_selection_untracked(&mut self) -> bool {
        let selected_ids = selected_scene_object_ids(&self.state.selection);
        if selected_ids.len() < 2 {
            return false;
        }
        let group_id = self.next_id("grp");
        self.push_undo_snapshot();
        let Some(group) =
            group_selected_in_siblings(&mut self.state.document.objects, &selected_ids, group_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        self.state.selection = SelectionState {
            arrow_objects: vec![group.id],
            ..SelectionState::default()
        };
        true
    }

    pub fn ungroup_selection(&mut self) -> bool {
        let object_ids = selected_scene_object_ids(&self.state.selection)
            .into_iter()
            .collect();
        self.with_command(EditorCommand::UngroupSelection { object_ids }, |engine| {
            engine.ungroup_selection_untracked()
        })
    }

    fn ungroup_selection_untracked(&mut self) -> bool {
        let selected_ids = selected_scene_object_ids(&self.state.selection);
        if selected_ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut ungrouped = Vec::new();
        if !ungroup_selected_in_siblings(
            &mut self.state.document.objects,
            &selected_ids,
            &mut ungrouped,
        ) {
            self.undo_stack.pop();
            return false;
        }
        let mut selection = SelectionState::default();
        for object in ungrouped {
            if object.object_type == "text" {
                selection.text_objects.push(object.id);
            } else {
                selection.arrow_objects.push(object.id);
            }
        }
        self.state.selection = selection;
        true
    }

    pub fn join_selection(&mut self) -> bool {
        self.with_command(EditorCommand::JoinSelection, |engine| {
            engine.join_selection_untracked()
        })
    }

    fn join_selection_untracked(&mut self) -> bool {
        false
    }

    pub fn apply_selection_order_command(&mut self, command: &str) -> bool {
        let object_ids = selected_scene_object_ids(&self.state.selection)
            .into_iter()
            .collect();
        self.with_command(
            EditorCommand::ApplySelectionOrder {
                object_ids,
                command: command.to_string(),
            },
            |engine| engine.apply_selection_order_command_untracked(command),
        )
    }

    fn apply_selection_order_command_untracked(&mut self, command: &str) -> bool {
        let selected_ids = selected_scene_object_ids(&self.state.selection);
        if selected_ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let changed =
            apply_order_in_siblings(&mut self.state.document.objects, &selected_ids, command);
        if !changed {
            self.undo_stack.pop();
        }
        changed
    }
}

fn selected_scene_object_ids(selection: &SelectionState) -> BTreeSet<String> {
    selection
        .text_objects
        .iter()
        .chain(selection.arrow_objects.iter())
        .cloned()
        .collect()
}

fn group_selected_in_siblings(
    siblings: &mut Vec<SceneObject>,
    selected_ids: &BTreeSet<String>,
    group_id: String,
) -> Option<SceneObject> {
    let selected_indices: Vec<usize> = siblings
        .iter()
        .enumerate()
        .filter_map(|(index, object)| selected_ids.contains(&object.id).then_some(index))
        .collect();
    if selected_indices.len() == selected_ids.len() {
        let insert_index = selected_indices[0];
        let mut selected = Vec::new();
        for index in selected_indices.into_iter().rev() {
            selected.push(siblings.remove(index));
        }
        selected.reverse();
        let z_index = selected
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or_default();
        let group = SceneObject {
            id: group_id,
            object_type: "group".to_string(),
            name: "group".to_string(),
            visible: true,
            locked: false,
            z_index,
            transform: Transform::identity(),
            style_ref: None,
            meta: json!({"source": "chemcore-editor"}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra: BTreeMap::new(),
            },
            children: selected,
        };
        siblings.insert(insert_index, group.clone());
        return Some(group);
    }
    for object in siblings {
        if let Some(group) =
            group_selected_in_siblings(&mut object.children, selected_ids, group_id.clone())
        {
            return Some(group);
        }
    }
    None
}

fn ungroup_selected_in_siblings(
    siblings: &mut Vec<SceneObject>,
    selected_ids: &BTreeSet<String>,
    ungrouped: &mut Vec<SceneObject>,
) -> bool {
    let mut changed = false;
    let mut index = 0;
    while index < siblings.len() {
        if siblings[index].object_type == "group" && selected_ids.contains(&siblings[index].id) {
            let group = siblings.remove(index);
            let children = group.children;
            for child in children.iter() {
                ungrouped.push(child.clone());
            }
            for child in children.into_iter().rev() {
                siblings.insert(index, child);
            }
            changed = true;
            continue;
        }
        if ungroup_selected_in_siblings(&mut siblings[index].children, selected_ids, ungrouped) {
            changed = true;
        }
        index += 1;
    }
    changed
}

fn apply_order_in_siblings(
    siblings: &mut Vec<SceneObject>,
    selected_ids: &BTreeSet<String>,
    command: &str,
) -> bool {
    let selected_count = siblings
        .iter()
        .filter(|object| selected_ids.contains(&object.id))
        .count();
    if selected_count == selected_ids.len() {
        return apply_order_to_matching_siblings(siblings, selected_ids, command);
    }
    for object in siblings {
        if apply_order_in_siblings(&mut object.children, selected_ids, command) {
            return true;
        }
    }
    false
}

fn apply_order_to_matching_siblings(
    siblings: &mut [SceneObject],
    selected_ids: &BTreeSet<String>,
    command: &str,
) -> bool {
    if siblings.len() < 2 {
        return false;
    }
    let before: Vec<(String, i32)> = siblings
        .iter()
        .map(|object| (object.id.clone(), object.z_index))
        .collect();
    match command {
        "bring-front" | "front" => {
            let max_z = siblings
                .iter()
                .map(|object| object.z_index)
                .max()
                .unwrap_or(0);
            let mut next_z = max_z + STACK_STEP;
            for object in siblings.iter_mut() {
                if selected_ids.contains(&object.id) {
                    object.z_index = next_z;
                    next_z += STACK_STEP;
                }
            }
        }
        "send-back" | "back" => {
            let min_z = siblings
                .iter()
                .map(|object| object.z_index)
                .min()
                .unwrap_or(0);
            let mut next_z = min_z - (selected_ids.len() as i32 * STACK_STEP);
            for object in siblings.iter_mut() {
                if selected_ids.contains(&object.id) {
                    object.z_index = next_z;
                    next_z += STACK_STEP;
                }
            }
        }
        "bring-forward" | "forward" | "send-backward" | "backward" => {
            let mut order: Vec<usize> = (0..siblings.len()).collect();
            order.sort_by(|a, b| {
                siblings[*a]
                    .z_index
                    .cmp(&siblings[*b].z_index)
                    .then_with(|| siblings[*a].id.cmp(&siblings[*b].id))
            });
            if matches!(command, "bring-forward" | "forward") {
                for i in (0..order.len().saturating_sub(1)).rev() {
                    if selected_ids.contains(&siblings[order[i]].id)
                        && !selected_ids.contains(&siblings[order[i + 1]].id)
                    {
                        order.swap(i, i + 1);
                    }
                }
            } else {
                for i in 1..order.len() {
                    if selected_ids.contains(&siblings[order[i]].id)
                        && !selected_ids.contains(&siblings[order[i - 1]].id)
                    {
                        order.swap(i, i - 1);
                    }
                }
            }
            for (rank, index) in order.into_iter().enumerate() {
                siblings[index].z_index = ((rank + 1) as i32) * STACK_STEP;
            }
        }
        _ => return false,
    }
    siblings
        .iter()
        .map(|object| (object.id.clone(), object.z_index))
        .collect::<Vec<_>>()
        != before
}
