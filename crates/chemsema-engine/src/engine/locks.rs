use super::{EditorCommand, Engine};
use std::collections::BTreeSet;

impl Engine {
    pub fn set_selection_locked(&mut self, locked: bool) -> bool {
        let object_ids = self
            .selection_lockable_object_ids()
            .into_iter()
            .collect::<Vec<_>>();
        if object_ids.is_empty() {
            return false;
        }
        self.with_command(
            EditorCommand::SetObjectsLocked {
                object_ids: object_ids.clone(),
                locked,
            },
            |engine| engine.set_objects_locked_direct(&object_ids, locked),
        )
    }

    pub(super) fn set_objects_locked_direct(
        &mut self,
        object_ids: &[String],
        locked: bool,
    ) -> bool {
        let changed_ids = object_ids
            .iter()
            .filter(|object_id| {
                self.state
                    .document
                    .find_scene_object(object_id)
                    .is_some_and(|object| object.locked != locked)
            })
            .cloned()
            .collect::<Vec<_>>();
        if changed_ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        for object_id in changed_ids {
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                object.locked = locked;
            }
        }
        self.clear_interaction();
        true
    }

    pub(super) fn selection_lockable_object_ids(&self) -> BTreeSet<String> {
        let selection = &self.state.selection;
        let mut object_ids = selection
            .text_objects
            .iter()
            .chain(selection.arrow_objects.iter())
            .chain(selection.molecule_objects.iter())
            .cloned()
            .collect::<BTreeSet<_>>();
        let selected_nodes = selection
            .nodes
            .iter()
            .chain(selection.label_nodes.iter())
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let selected_bonds = selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for entry in self.state.document.editable_fragments() {
            let owns_selected_node = entry
                .fragment
                .nodes
                .iter()
                .any(|node| selected_nodes.contains(node.id.as_str()));
            let owns_selected_bond = entry
                .fragment
                .bonds
                .iter()
                .any(|bond| selected_bonds.contains(bond.id.as_str()));
            if owns_selected_node || owns_selected_bond {
                object_ids.insert(entry.object.id.clone());
            }
        }
        object_ids
    }

    pub(super) fn selection_objects_are_all_locked(&self) -> Option<bool> {
        let object_ids = self.selection_lockable_object_ids();
        (!object_ids.is_empty()).then(|| {
            object_ids.iter().all(|object_id| {
                self.state
                    .document
                    .find_scene_object(object_id)
                    .is_some_and(|object| object.locked)
            })
        })
    }
}
