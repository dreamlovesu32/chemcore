use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{EditorCommand, Engine};
use crate::{Bond, Node, SelectionState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const CLIPBOARD_PASTE_OFFSET_CM: f64 = 0.35 * crate::PT_PER_CM;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClipboardContent {
    nodes: Vec<Node>,
    bonds: Vec<Bond>,
}

impl Engine {
    pub fn copy_selection(&mut self) -> bool {
        let Some(content) = self.clipboard_content_from_selection() else {
            return false;
        };
        self.clipboard = Some(content);
        true
    }

    pub fn clipboard_selection_json(&self) -> Result<Option<String>, String> {
        self.clipboard_content_from_selection()
            .map(|content| serde_json::to_string(&content).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn cut_selection(&mut self) -> bool {
        self.with_command(EditorCommand::CutSelection, |engine| {
            engine.cut_selection_untracked()
        })
    }

    fn cut_selection_untracked(&mut self) -> bool {
        if !self.copy_selection() {
            return false;
        }
        self.delete_selection()
    }

    pub fn paste_clipboard(&mut self) -> bool {
        self.with_command(EditorCommand::PasteClipboard, |engine| {
            engine.paste_clipboard_untracked()
        })
    }

    pub fn paste_clipboard_json(&mut self, json: &str) -> Result<bool, String> {
        let content: ClipboardContent =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        self.clipboard = Some(content);
        Ok(self.paste_clipboard())
    }

    fn paste_clipboard_untracked(&mut self) -> bool {
        let Some(content) = self.clipboard.clone() else {
            return false;
        };
        if content.nodes.is_empty() {
            return false;
        }
        if self.state.document.editable_fragment().is_none() {
            return false;
        }
        self.push_undo_snapshot();
        let mut id_map = BTreeMap::new();
        let mut pasted_node_ids = Vec::new();
        let mut pasted_bond_ids = Vec::new();
        let mut nodes_to_insert = Vec::new();
        let mut bonds_to_insert = Vec::new();

        for node in &content.nodes {
            let next_id = self.next_id("n");
            id_map.insert(node.id.clone(), next_id.clone());
            let mut next = node.clone();
            next.id = next_id.clone();
            next.position[0] = crate::round2(next.position[0] + CLIPBOARD_PASTE_OFFSET_CM);
            next.position[1] = crate::round2(next.position[1] + CLIPBOARD_PASTE_OFFSET_CM);
            nodes_to_insert.push(next);
            pasted_node_ids.push(next_id);
        }

        for bond in &content.bonds {
            let (Some(begin), Some(end)) = (id_map.get(&bond.begin), id_map.get(&bond.end)) else {
                continue;
            };
            let mut next = bond.clone();
            next.id = self.next_id("b");
            next.begin = begin.clone();
            next.end = end.clone();
            pasted_bond_ids.push(next.id.clone());
            bonds_to_insert.push(next);
        }

        let stroke_width = self.options.bond_stroke_world_cm().value();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        entry.fragment.nodes.extend(nodes_to_insert);
        entry.fragment.bonds.extend(bonds_to_insert);

        let object_translate = entry.object.transform.translate;
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            object_translate,
            stroke_width,
        );
        entry.update_bounds();
        self.state.selection = SelectionState {
            nodes: pasted_node_ids,
            bonds: pasted_bond_ids,
            ..SelectionState::default()
        };
        self.clear_interaction();
        true
    }

    fn clipboard_content_from_selection(&self) -> Option<ClipboardContent> {
        if self.state.selection.is_empty() {
            return None;
        }
        let entry = self.state.document.editable_fragment()?;
        let mut node_ids: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        node_ids.extend(self.state.selection.label_nodes.iter().cloned());

        let selected_bonds: BTreeSet<&str> = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect();
        for bond in &entry.fragment.bonds {
            if selected_bonds.contains(bond.id.as_str()) {
                node_ids.insert(bond.begin.clone());
                node_ids.insert(bond.end.clone());
            }
        }

        let nodes: Vec<Node> = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| node_ids.contains(&node.id))
            .cloned()
            .collect();
        if nodes.is_empty() {
            return None;
        }

        let bonds: Vec<Bond> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                selected_bonds.contains(bond.id.as_str())
                    && node_ids.contains(&bond.begin)
                    && node_ids.contains(&bond.end)
            })
            .cloned()
            .collect();

        Some(ClipboardContent { nodes, bonds })
    }
}
