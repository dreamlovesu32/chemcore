use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{EditorCommand, Engine, RenderBoundsScope};
use crate::{Bond, ChemcoreDocument, Node, Resource, ResourceData, SceneObject, SelectionState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const CLIPBOARD_PASTE_OFFSET_PT: f64 = 9.921_259_842_519_685;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClipboardContent {
    nodes: Vec<Node>,
    bonds: Vec<Bond>,
}

impl Engine {
    pub fn has_clipboard(&self) -> bool {
        self.clipboard
            .as_ref()
            .is_some_and(|content| !content.nodes.is_empty())
    }

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

    pub fn clipboard_document_json(&self) -> Result<Option<String>, String> {
        self.document_from_selection()
            .map(|document| serde_json::to_string(&document).map_err(|error| error.to_string()))
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
            next.position[0] = crate::round2(next.position[0] + CLIPBOARD_PASTE_OFFSET_PT);
            next.position[1] = crate::round2(next.position[1] + CLIPBOARD_PASTE_OFFSET_PT);
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

        let stroke_width = self.options.bond_stroke_world_pt().value();
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

    fn document_from_selection(&self) -> Option<ChemcoreDocument> {
        if self.state.selection.is_empty() {
            return None;
        }

        let selected_molecule = self.selected_molecule_clipboard_object();
        let mut selected_object_ids: BTreeSet<String> =
            self.state.selection.text_objects.iter().cloned().collect();
        selected_object_ids.extend(self.state.selection.arrow_objects.iter().cloned());

        let mut objects = Vec::new();
        for object in &self.state.document.objects {
            if selected_molecule
                .as_ref()
                .is_some_and(|(molecule, _, _)| molecule.id == object.id)
            {
                objects.push(selected_molecule.as_ref().unwrap().0.clone());
                continue;
            }
            clone_selected_scene_objects(object, &selected_object_ids, &mut objects);
        }
        if objects.is_empty() {
            return None;
        }

        let mut document = self.state.document.clone();
        document.document.id = "doc_clipboard_selection".to_string();
        document.document.title = "Chemcore Clipboard Selection".to_string();
        document.objects = objects;
        if let Some((_, resource_ref, resource)) = selected_molecule {
            document.resources.insert(resource_ref, resource);
        }
        if let Some(bounds) = self.render_bounds(RenderBoundsScope::Selection) {
            set_clipboard_selection_bounds_meta(&mut document, bounds);
        }
        Some(document)
    }

    fn selected_molecule_clipboard_object(&self) -> Option<(SceneObject, String, Resource)> {
        let entry = self.state.document.editable_fragment()?;
        let resource_ref = entry.object.payload.resource_ref.clone()?;

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

        let mut fragment = entry.fragment.clone();
        fragment.nodes = nodes;
        fragment.bonds = bonds;
        fragment.bbox = fragment_clipboard_bounds(&fragment.nodes);

        let mut object = entry.object.clone();
        object.payload.bbox = Some(fragment.bbox);

        let mut resource = self.state.document.resources.get(&resource_ref)?.clone();
        resource.data = ResourceData::Fragment(fragment);
        Some((object, resource_ref, resource))
    }
}

fn clone_selected_scene_objects(
    object: &SceneObject,
    selected_ids: &BTreeSet<String>,
    out: &mut Vec<SceneObject>,
) {
    if selected_ids.contains(&object.id) {
        out.push(object.clone());
        return;
    }

    let mut children = Vec::new();
    for child in &object.children {
        clone_selected_scene_objects(child, selected_ids, &mut children);
    }
    if !children.is_empty() {
        let mut clone = object.clone();
        clone.children = children;
        out.push(clone);
    }
}

fn fragment_clipboard_bounds(nodes: &[Node]) -> [f64; 4] {
    let Some(first) = nodes.first() else {
        return [0.0, 0.0, 1.0, 1.0];
    };
    let mut min_x = first.position[0];
    let mut min_y = first.position[1];
    let mut max_x = first.position[0];
    let mut max_y = first.position[1];
    for node in nodes {
        min_x = min_x.min(node.position[0]);
        min_y = min_y.min(node.position[1]);
        max_x = max_x.max(node.position[0]);
        max_y = max_y.max(node.position[1]);
        if let Some(label) = &node.label {
            if let Some([x1, y1, x2, y2]) = label.bbox() {
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }
        }
    }
    [min_x, min_y, max_x.max(min_x + 1.0), max_y.max(min_y + 1.0)]
}

fn set_clipboard_selection_bounds_meta(document: &mut ChemcoreDocument, bounds: [f64; 4]) {
    if !document.document.meta.is_object() {
        document.document.meta = serde_json::json!({});
    }
    let Some(meta) = document.document.meta.as_object_mut() else {
        return;
    };
    let clipboard = meta
        .entry("clipboard")
        .or_insert_with(|| serde_json::json!({}));
    if !clipboard.is_object() {
        *clipboard = serde_json::json!({});
    }
    if let Some(clipboard) = clipboard.as_object_mut() {
        clipboard.insert(
            "selectionBounds".to_string(),
            serde_json::json!({
                "minX": bounds[0],
                "minY": bounds[1],
                "maxX": bounds[2],
                "maxY": bounds[3],
            }),
        );
    }
}
