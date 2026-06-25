use super::*;
use crate::{round2, MoleculeFragment, Node, ObjectPayload, Vector};
use serde_json::Map;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl Engine {
    pub fn apply_shape_style_to_selection(&mut self, style: &str) -> bool {
        let style = normalize_shape_style_name(style);
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            EditorCommand::ApplyShapeStyle {
                object_ids,
                style: style.clone(),
            },
            |engine| engine.apply_shape_style_to_selection_untracked(&style),
        )
    }

    fn apply_shape_style_to_selection_untracked(&mut self, style: &str) -> bool {
        let selected: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let updates = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| selected.contains(&object.id) && object.object_type == "shape")
            .map(|object| {
                let color = selected_object_style_color(&self.state.document, object);
                (
                    object.id.clone(),
                    format!("style_{}_shape_{}", object.id, style.replace('-', "_")),
                    shape_style_json(
                        style,
                        &color,
                        self.options.graphic_stroke_world_pt().value(),
                        self.options.hash_spacing,
                    ),
                )
            })
            .collect::<Vec<_>>();
        if updates.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        for (object_id, style_id, style_value) in updates {
            self.state
                .document
                .styles
                .insert(style_id.clone(), style_value);
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                if object.style_ref.as_deref() != Some(style_id.as_str()) {
                    object.style_ref = Some(style_id);
                    changed = true;
                }
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_shape = None;
        true
    }

    pub fn apply_bracket_kind_to_selection(&mut self, kind: &str) -> bool {
        let kind = normalize_bracket_kind_name(kind);
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            EditorCommand::ApplyBracketKind {
                object_ids,
                kind: kind.clone(),
            },
            |engine| engine.apply_bracket_kind_to_selection_untracked(&kind),
        )
    }

    pub fn apply_orbital_template_to_selection(&mut self, template: &str) -> bool {
        let template = normalize_orbital_template_name(template);
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            EditorCommand::ApplyOrbitalTemplate {
                object_ids,
                template: template.clone(),
            },
            |engine| engine.apply_orbital_template_to_selection_untracked(&template),
        )
    }

    fn apply_orbital_template_to_selection_untracked(&mut self, template: &str) -> bool {
        let selected: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let ids = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| {
                selected.contains(&object.id)
                    && object.object_type == "shape"
                    && payload_string(&object.payload, "kind").as_deref() == Some("orbital")
            })
            .map(|object| object.id.clone())
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        for object_id in ids {
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                changed |=
                    set_payload_string(&mut object.payload.extra, "orbitalTemplate", template);
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_shape = None;
        true
    }

    pub fn apply_orbital_style_to_selection(&mut self, style: &str) -> bool {
        let style = normalize_orbital_style_name(style);
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            EditorCommand::ApplyOrbitalStyle {
                object_ids,
                style: style.clone(),
            },
            |engine| engine.apply_orbital_style_to_selection_untracked(&style),
        )
    }

    fn apply_orbital_style_to_selection_untracked(&mut self, style: &str) -> bool {
        let selected: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let updates = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| {
                selected.contains(&object.id)
                    && object.object_type == "shape"
                    && payload_string(&object.payload, "kind").as_deref() == Some("orbital")
            })
            .map(|object| {
                let color = selected_object_style_color(&self.state.document, object);
                (
                    object.id.clone(),
                    format!("style_{}_orbital_{}", object.id, style.replace('-', "_")),
                    orbital_style_json(
                        style,
                        &color,
                        self.options.graphic_stroke_world_pt().value(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        if updates.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        for (object_id, style_id, style_value) in updates {
            self.state
                .document
                .styles
                .insert(style_id.clone(), style_value);
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                changed |= set_payload_string(&mut object.payload.extra, "orbitalStyle", style);
                if object.style_ref.as_deref() != Some(style_id.as_str()) {
                    object.style_ref = Some(style_id);
                    changed = true;
                }
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_shape = None;
        true
    }

    pub fn apply_orbital_phase_to_selection(&mut self, phase: &str) -> bool {
        let phase = normalize_orbital_phase_name(phase);
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            EditorCommand::ApplyOrbitalPhase {
                object_ids,
                phase: phase.clone(),
            },
            |engine| engine.apply_orbital_phase_to_selection_untracked(&phase),
        )
    }

    fn apply_orbital_phase_to_selection_untracked(&mut self, phase: &str) -> bool {
        let selected: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let ids = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| {
                selected.contains(&object.id)
                    && object.object_type == "shape"
                    && payload_string(&object.payload, "kind").as_deref() == Some("orbital")
            })
            .map(|object| object.id.clone())
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        for object_id in ids {
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                changed |= set_payload_string(&mut object.payload.extra, "orbitalPhase", phase);
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_shape = None;
        true
    }

    fn apply_bracket_kind_to_selection_untracked(&mut self, kind: &str) -> bool {
        let selected: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let mut ids = Vec::new();
        collect_selected_bracket_kind_target_ids(&self.state.document.objects, &selected, &mut ids);
        if ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        for object_id in ids {
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                changed |= set_payload_string(&mut object.payload.extra, "kind", &kind);
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_shape = None;
        true
    }

    pub fn apply_line_style_to_selection(&mut self, style: &str) -> bool {
        let style = normalize_line_style_name(style);
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            EditorCommand::ApplyLineStyle {
                object_ids,
                style: style.clone(),
            },
            |engine| engine.apply_line_style_to_selection_untracked(&style),
        )
    }

    fn apply_line_style_to_selection_untracked(&mut self, style: &str) -> bool {
        let selected: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        let updates = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| selected.contains(&object.id) && object.object_type == "line")
            .map(|object| {
                let color = selected_object_style_color(&self.state.document, object);
                (
                    object.id.clone(),
                    format!("style_{}_line_{}", object.id, style.replace('-', "_")),
                    line_style_json(
                        style,
                        &color,
                        self.options.graphic_stroke_world_pt().value(),
                        self.options.hash_spacing,
                    ),
                )
            })
            .collect::<Vec<_>>();
        if updates.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        for (object_id, style_id, style_value) in updates {
            self.state
                .document
                .styles
                .insert(style_id.clone(), style_value);
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                if object.style_ref.as_deref() != Some(style_id.as_str()) {
                    object.style_ref = Some(style_id);
                    changed = true;
                }
                changed |= set_line_arrow_bold(object, style == "bold");
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_arrow = None;
        true
    }

    pub fn apply_bond_style_to_selection(&mut self, style: &str) -> bool {
        let style = normalize_bond_style_name(style);
        let bond_ids = self.state.selection.bonds.clone();
        self.with_command(
            EditorCommand::ApplyBondStyle {
                bond_ids,
                style: style.clone(),
            },
            |engine| engine.apply_bond_style_to_selection_untracked(&style),
        )
    }

    fn apply_bond_style_to_selection_untracked(&mut self, style: &str) -> bool {
        let bond_ids = self.state.selection.bonds.clone();
        self.apply_bond_style_to_bond_ids_untracked(&bond_ids, style)
    }

    pub fn apply_hovered_bond_style(&mut self, style: &str) -> bool {
        let style = normalize_bond_style_name(style);
        let Some(bond_id) = self
            .state
            .overlay
            .hover_bond_center
            .as_ref()
            .map(|hover| hover.bond_id.clone())
            .or_else(|| self.pointer_bond_target.clone())
        else {
            return false;
        };
        self.with_command(
            EditorCommand::ApplyBondStyle {
                bond_ids: vec![bond_id.clone()],
                style: style.clone(),
            },
            |engine| engine.apply_bond_style_to_bond_ids_untracked(&[bond_id.clone()], &style),
        )
    }

    pub(crate) fn apply_bond_style_to_bond_ids_untracked(
        &mut self,
        bond_ids: &[String],
        style: &str,
    ) -> bool {
        let style = normalize_bond_style_name(style);
        let selected: BTreeSet<String> = bond_ids.iter().cloned().collect();
        if selected.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let stroke_width = self.options.bond_stroke_world_pt().value();
        let bold_width = self.options.bold_bond_width_world_pt().value();
        let wedge_width = self.options.wedge_width_world_pt().value();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let mut changed = false;
        for bond in &mut entry.fragment.bonds {
            if selected.contains(&bond.id) {
                changed |= apply_bond_style_key(bond, &style, bold_width, wedge_width);
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            object_translate,
            stroke_width,
        );
        entry.update_bounds();
        self.state.overlay.hover_bond_center = None;
        self.pointer_bond_target = None;
        true
    }

    pub fn apply_text_style_to_selection(&mut self, command: &str, value: &str) -> bool {
        let command = normalize_text_style_command(command);
        let value = value.trim().to_string();
        let text_object_ids = self.state.selection.text_objects.clone();
        let label_node_ids = self.state.selection.label_nodes.clone();
        let node_ids = self.state.selection.nodes.clone();
        self.with_command(
            EditorCommand::ApplyTextStyle {
                text_object_ids,
                label_node_ids,
                node_ids,
                command: command.clone(),
                value: value.clone(),
            },
            |engine| engine.apply_text_style_to_selection_untracked(&command, &value),
        )
    }

    fn apply_text_style_to_selection_untracked(&mut self, command: &str, value: &str) -> bool {
        if self.state.selection.text_objects.is_empty()
            && self.state.selection.label_nodes.is_empty()
            && self.state.selection.nodes.is_empty()
        {
            return false;
        }
        self.push_undo_snapshot();
        let text_ids: BTreeSet<String> =
            self.state.selection.text_objects.iter().cloned().collect();
        let mut changed = false;
        for object_id in text_ids {
            if let Some(object) = self.state.document.find_scene_object_mut(&object_id) {
                if object.object_type == "text" {
                    changed |= apply_text_object_style(object, command, value);
                }
            }
        }
        changed |= self.apply_text_style_to_selected_labels(command, value);
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_text_box = None;
        true
    }

    fn apply_text_style_to_selected_labels(&mut self, command: &str, value: &str) -> bool {
        let selected_labels: BTreeSet<String> =
            self.state.selection.label_nodes.iter().cloned().collect();
        let selected_nodes: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        if selected_labels.is_empty() && selected_nodes.is_empty() {
            return false;
        }
        let stroke_width = self.options.bond_stroke_world_pt().value();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let mut changed = false;
        for node in &mut entry.fragment.nodes {
            if !selected_labels.contains(&node.id) && !selected_nodes.contains(&node.id) {
                continue;
            }
            if let Some(label) = &mut node.label {
                changed |= apply_node_label_style(label, command, value);
            }
        }
        if changed {
            refresh_attached_node_label_geometry_for_all_nodes(
                entry.fragment,
                object_translate,
                stroke_width,
            );
            entry.update_bounds();
        }
        changed
    }

    pub fn set_chemical_check_for_selection(&mut self, enabled: bool) -> bool {
        self.with_command(
            EditorCommand::SetChemicalCheckForSelection { enabled },
            |engine| engine.set_chemical_check_for_selection_untracked(enabled),
        )
    }

    fn set_chemical_check_for_selection_untracked(&mut self, enabled: bool) -> bool {
        let selected_labels: BTreeSet<String> =
            self.state.selection.label_nodes.iter().cloned().collect();
        let selected_nodes: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        if selected_labels.is_empty() && selected_nodes.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let Some(entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let mut changed = false;
        for node in &mut entry.fragment.nodes {
            if !selected_labels.contains(&node.id) && !selected_nodes.contains(&node.id) {
                continue;
            }
            changed |= set_value_object_bool(&mut node.meta, "chemicalCheck", enabled);
            if let Some(label) = &mut node.label {
                changed |= set_value_object_bool(&mut label.meta, "chemicalCheck", enabled);
            }
        }
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_text_box = None;
        true
    }

    pub fn expand_labels_in_selection(&mut self) -> bool {
        self.with_command(EditorCommand::ExpandLabelsInSelection, |engine| {
            engine.expand_labels_in_selection_untracked()
        })
    }

    fn expand_labels_in_selection_untracked(&mut self) -> bool {
        let selected_labels: BTreeSet<String> =
            self.state.selection.label_nodes.iter().cloned().collect();
        let selected_nodes: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        if selected_labels.is_empty() && selected_nodes.is_empty() {
            return false;
        }

        self.push_undo_snapshot();
        let stroke_width = self.options.bond_stroke_world_pt().value();
        let bold_width = self.options.bold_bond_width_world_pt().value();
        let wedge_width = self.options.wedge_width_world_pt().value();
        let label_clip_margin = self.options.label_clip_margin_world_pt().value();
        let hash_spacing = self.options.hash_spacing_world_pt().value();
        let bond_spacing = self.options.bond_spacing_percent();
        let margin_width = self.options.margin_width_world_pt().value();
        let bond_length = self.options.bond_length_world_pt().value();
        let mut expanded_node_ids = Vec::new();

        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let target_ids = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| selected_labels.contains(&node.id) || selected_nodes.contains(&node.id))
            .filter_map(|node| label_expansion_from_node(node).map(|_| node.id.clone()))
            .collect::<Vec<_>>();
        if target_ids.is_empty() {
            self.undo_stack.pop();
            return false;
        }

        let mut changed = false;
        for node_id in target_ids {
            if expand_label_node_in_fragment(
                entry.fragment,
                &node_id,
                bond_length,
                stroke_width,
                bold_width,
                wedge_width,
                label_clip_margin,
                hash_spacing,
                bond_spacing,
                margin_width,
                &mut expanded_node_ids,
            ) {
                changed = true;
            }
        }

        if !changed {
            self.undo_stack.pop();
            return false;
        }
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            stroke_width,
        );
        entry.update_bounds();
        drop(entry);
        refresh_element_valence_recognition_for_all_nodes(
            self.state
                .document
                .editable_fragment_mut()
                .expect("editable fragment should still exist")
                .fragment,
        );
        self.refresh_symbol_chemistry();
        self.state.selection = SelectionState {
            nodes: expanded_node_ids,
            ..SelectionState::default()
        };
        self.clear_interaction();
        true
    }
}

#[allow(clippy::too_many_arguments)]
fn expand_label_node_in_fragment(
    fragment: &mut MoleculeFragment,
    node_id: &str,
    bond_length: f64,
    stroke_width: f64,
    bold_width: f64,
    wedge_width: f64,
    label_clip_margin: f64,
    hash_spacing: f64,
    bond_spacing: f64,
    margin_width: f64,
    expanded_node_ids: &mut Vec<String>,
) -> bool {
    let Some(node_index) = fragment.nodes.iter().position(|node| node.id == node_id) else {
        return false;
    };
    let Some(expansion) = label_expansion_from_node(&fragment.nodes[node_index]) else {
        return false;
    };
    let Some(atoms) = expansion.get("atoms").and_then(JsonValue::as_array) else {
        return false;
    };
    if atoms.is_empty() {
        return false;
    }
    let Some(bonds) = expansion.get("bonds").and_then(JsonValue::as_array) else {
        return false;
    };
    let attachments = expansion
        .get("attachments")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let target_node = fragment.nodes[node_index].clone();
    let connected_bond_indices = fragment
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| (bond.begin == node_id || bond.end == node_id).then_some(index))
        .collect::<Vec<_>>();
    let outward = expansion_outward_direction(fragment, &target_node, &connected_bond_indices);
    let positions = expansion_atom_positions(
        atoms,
        bonds,
        &attachments,
        target_node.point(),
        outward,
        bond_length,
    );

    let mut used_node_ids: BTreeSet<String> =
        fragment.nodes.iter().map(|node| node.id.clone()).collect();
    let mut used_bond_ids: BTreeSet<String> =
        fragment.bonds.iter().map(|bond| bond.id.clone()).collect();
    let mut atom_id_map = BTreeMap::new();
    let mut next_nodes = Vec::new();
    for atom in atoms {
        let Some(source_id) = atom.get("id").and_then(JsonValue::as_str) else {
            continue;
        };
        let id = unique_expansion_id(&mut used_node_ids, node_id, source_id);
        let position = positions
            .get(source_id)
            .copied()
            .unwrap_or_else(|| target_node.point());
        atom_id_map.insert(source_id.to_string(), id.clone());
        expanded_node_ids.push(id.clone());
        next_nodes.push(expansion_atom_to_node(atom, id, position));
    }
    if next_nodes.is_empty() {
        return false;
    }

    let attachment_targets = expansion_attachment_targets(&attachments, &atom_id_map);
    for (connected_index, bond_index) in connected_bond_indices.iter().copied().enumerate() {
        let Some(target_id) = attachment_targets
            .get(connected_index)
            .or_else(|| attachment_targets.first())
        else {
            continue;
        };
        let bond = &mut fragment.bonds[bond_index];
        if bond.begin == node_id {
            bond.begin = target_id.clone();
        }
        if bond.end == node_id {
            bond.end = target_id.clone();
        }
    }

    for bond in bonds {
        let Some(begin) = bond
            .get("begin")
            .and_then(JsonValue::as_str)
            .and_then(|id| atom_id_map.get(id))
            .cloned()
        else {
            continue;
        };
        let Some(end) = bond
            .get("end")
            .and_then(JsonValue::as_str)
            .and_then(|id| atom_id_map.get(id))
            .cloned()
        else {
            continue;
        };
        if begin == end || bond_exists(fragment, &begin, &end) {
            continue;
        }
        let order = bond
            .get("order")
            .and_then(JsonValue::as_u64)
            .unwrap_or(1)
            .clamp(1, 3) as u8;
        fragment.bonds.push(expansion_bond(
            unique_expansion_id(&mut used_bond_ids, node_id, "b"),
            begin,
            end,
            order,
            stroke_width,
            bold_width,
            wedge_width,
            label_clip_margin,
            hash_spacing,
            bond_spacing,
            margin_width,
        ));
    }

    fragment.nodes.remove(node_index);
    fragment.nodes.extend(next_nodes);
    true
}

fn label_expansion_from_node(node: &Node) -> Option<JsonValue> {
    let recognition = node
        .meta
        .get("labelRecognition")
        .or_else(|| node.label.as_ref()?.meta.get("labelRecognition"))?;
    if recognition.get("status").and_then(JsonValue::as_str) != Some("recognized") {
        return None;
    }
    let expansion = recognition.get("expansion")?;
    if expansion.get("complete").and_then(JsonValue::as_bool) != Some(true) {
        return None;
    }
    Some(expansion.clone())
}

fn expansion_outward_direction(
    fragment: &MoleculeFragment,
    node: &Node,
    connected_bond_indices: &[usize],
) -> Vector {
    let node_point = node.point();
    connected_bond_indices
        .first()
        .and_then(|index| fragment.bonds.get(*index))
        .and_then(|bond| {
            let neighbor_id = if bond.begin == node.id {
                &bond.end
            } else {
                &bond.begin
            };
            fragment
                .nodes
                .iter()
                .find(|candidate| &candidate.id == neighbor_id)
        })
        .map(|neighbor| {
            Vector::new(
                node_point.x - neighbor.position[0],
                node_point.y - neighbor.position[1],
            )
        })
        .filter(|vector| vector.length() > crate::EPSILON)
        .map(|vector| vector.normalized())
        .unwrap_or_else(|| Vector::new(1.0, 0.0))
}

fn expansion_atom_positions(
    atoms: &[JsonValue],
    bonds: &[JsonValue],
    attachments: &[JsonValue],
    origin: Point,
    outward: Vector,
    bond_length: f64,
) -> BTreeMap<String, Point> {
    let atom_ids = atoms
        .iter()
        .filter_map(|atom| {
            atom.get("id")
                .and_then(JsonValue::as_str)
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    let root = attachments
        .iter()
        .filter_map(|attachment| attachment.get("atomId").and_then(JsonValue::as_str))
        .find(|atom_id| atom_ids.iter().any(|id| id == atom_id))
        .map(ToString::to_string)
        .or_else(|| atom_ids.first().cloned());
    let Some(root) = root else {
        return BTreeMap::new();
    };

    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for bond in bonds {
        let Some(begin) = bond.get("begin").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(end) = bond.get("end").and_then(JsonValue::as_str) else {
            continue;
        };
        adjacency
            .entry(begin.to_string())
            .or_default()
            .push(end.to_string());
        adjacency
            .entry(end.to_string())
            .or_default()
            .push(begin.to_string());
    }

    let mut positions = BTreeMap::new();
    let mut queue = VecDeque::new();
    positions.insert(root.clone(), origin);
    queue.push_back((root, outward));
    while let Some((atom_id, direction)) = queue.pop_front() {
        let Some(parent_position) = positions.get(&atom_id).copied() else {
            continue;
        };
        let neighbors = adjacency.remove(&atom_id).unwrap_or_default();
        let mut child_index = 0usize;
        for neighbor in neighbors {
            if positions.contains_key(&neighbor) {
                continue;
            }
            let angle = match child_index {
                0 => 0.0,
                1 => 120.0,
                2 => -120.0,
                _ => 60.0 * child_index as f64,
            };
            let next_direction = rotate_vector(direction, angle);
            let position = parent_position.translated(next_direction.scaled(bond_length));
            positions.insert(
                neighbor.clone(),
                Point::new(round2(position.x), round2(position.y)),
            );
            queue.push_back((neighbor, next_direction));
            child_index += 1;
        }
    }
    positions
}

fn expansion_attachment_targets(
    attachments: &[JsonValue],
    atom_id_map: &BTreeMap<String, String>,
) -> Vec<String> {
    attachments
        .iter()
        .filter_map(|attachment| {
            attachment
                .get("atomId")
                .and_then(JsonValue::as_str)
                .and_then(|source_id| atom_id_map.get(source_id))
                .cloned()
        })
        .collect()
}

fn expansion_atom_to_node(atom: &JsonValue, id: String, position: Point) -> Node {
    let element = atom
        .get("element")
        .and_then(JsonValue::as_str)
        .unwrap_or("C")
        .to_string();
    let label = atom
        .get("label")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string);
    let atomic_number = atomic_number_for_element(&element);
    Node {
        id,
        element: element.clone(),
        atomic_number,
        position: [round2(position.x), round2(position.y)],
        charge: atom
            .get("formalCharge")
            .and_then(JsonValue::as_i64)
            .unwrap_or(0) as i32,
        num_hydrogens: atom
            .get("numHydrogens")
            .and_then(JsonValue::as_u64)
            .unwrap_or(0) as u8,
        is_external_connection_point: false,
        is_placeholder: false,
        label: label
            .filter(|text| atomic_number == 0 || text != &element)
            .map(|text| crate::NodeLabel {
                text,
                source_text: None,
                position: None,
                box_field: None,
                runs: Vec::new(),
                line_runs: Vec::new(),
                lines: Vec::new(),
                align: None,
                layout: None,
                attachment: None,
                anchor: None,
                font_family: None,
                fill: None,
                font_size: None,
                glyph_polygons: Vec::new(),
                box_value: None,
                meta: JsonValue::Null,
            }),
        meta: json!({"source": "label-expansion"}),
    }
}

#[allow(clippy::too_many_arguments)]
fn expansion_bond(
    id: String,
    begin: String,
    end: String,
    order: u8,
    stroke_width: f64,
    bold_width: f64,
    wedge_width: f64,
    label_clip_margin: f64,
    hash_spacing: f64,
    bond_spacing: f64,
    margin_width: f64,
) -> Bond {
    Bond {
        id,
        begin,
        end,
        order,
        double: (order == 2).then_some(DoubleBond {
            placement: DoubleBondPlacement::Center,
            center_exit_side: None,
            frozen: false,
        }),
        stereo: None,
        stroke_width,
        stroke: None,
        bold_width: Some(bold_width),
        wedge_width: Some(wedge_width),
        label_clip_margin: Some(label_clip_margin),
        hash_spacing: Some(hash_spacing),
        bond_spacing: Some(bond_spacing),
        margin_width: Some(margin_width),
        line_styles: BondLineStyles::default(),
        line_weights: BondLineWeights::default(),
        meta: json!({"source": "label-expansion"}),
    }
}

fn unique_expansion_id(used: &mut BTreeSet<String>, prefix: &str, source_id: &str) -> String {
    let clean_source = source_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    let base = format!("{prefix}_{clean_source}");
    if used.insert(base.clone()) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}_{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("finite id search should return")
}

fn bond_exists(fragment: &MoleculeFragment, begin: &str, end: &str) -> bool {
    fragment.bonds.iter().any(|bond| {
        (bond.begin == begin && bond.end == end) || (bond.begin == end && bond.end == begin)
    })
}

fn rotate_vector(vector: Vector, degrees: f64) -> Vector {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    Vector::new(
        vector.x * cos - vector.y * sin,
        vector.x * sin + vector.y * cos,
    )
}

fn atomic_number_for_element(element: &str) -> u8 {
    match element {
        "H" => 1,
        "B" => 5,
        "C" => 6,
        "N" => 7,
        "O" => 8,
        "F" => 9,
        "Si" => 14,
        "P" => 15,
        "S" => 16,
        "Cl" => 17,
        "Br" => 35,
        "I" => 53,
        _ => 0,
    }
}

fn normalize_shape_style_name(style: &str) -> String {
    match style.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "dashed" => "dashed",
        "filled" | "fill" => "filled",
        "shaded" | "shade" => "shaded",
        "faded" | "fade" => "faded",
        "shadowed" | "shadow" => "shadowed",
        _ => "plain",
    }
    .to_string()
}

fn normalize_orbital_template_name(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "p" => "p",
        "dxy" => "dxy",
        "oval" => "oval",
        "hybrid" => "hybrid",
        "dz2" => "dz2",
        "lobe" => "lobe",
        _ => "s",
    }
    .to_string()
}

fn normalize_orbital_style_name(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "filled" => "filled",
        "shaded" => "shaded",
        _ => "hollow",
    }
    .to_string()
}

fn normalize_orbital_phase_name(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "minus" => "minus",
        _ => "plus",
    }
    .to_string()
}

fn normalize_bracket_kind_name(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "square" | "square-brackets" => "square",
        "curly" | "brace" | "braces" => "curly",
        _ => "round",
    }
    .to_string()
}

fn normalize_line_style_name(style: &str) -> String {
    match style.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "dashed" => "dashed",
        "bold" => "bold",
        _ => "plain",
    }
    .to_string()
}

fn normalize_bond_style_name(style: &str) -> String {
    match style.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "single-plain" | "single" => "single-plain",
        "single-dashed" | "dashed" => "single-dashed",
        "single-hashed" | "hashed" => "single-hashed",
        "single-hashed-wedged" | "hashed-wedged" | "hashed-wedge" => "single-hashed-wedged",
        "single-bold" | "bold" => "single-bold",
        "single-bold-wedged" | "bold-wedged" | "wedge" | "wedged" => "single-bold-wedged",
        "single-hollow-wedged" | "hollow-wedged" | "hollow-wedge" => "single-hollow-wedged",
        "single-wavy" | "wavy" => "single-wavy",
        "double-left" => "double-left",
        "double-right" => "double-right",
        "double-center" | "double" => "double-center",
        "double-bold" => "double-bold",
        "double-dashed" => "double-dashed",
        "double-double-dashed" => "double-double-dashed",
        "triple-plain" | "triple" => "triple-plain",
        _ => "single-plain",
    }
    .to_string()
}

fn normalize_text_style_command(command: &str) -> String {
    match command
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .as_str()
    {
        "font" | "font-family" => "font-family",
        "size" | "font-size" => "font-size",
        "align" | "alignment" => "align",
        "line-height" | "line-spacing" => "line-height",
        "italic" => "italic",
        "underline" => "underline",
        "superscript" => "superscript",
        "subscript" => "subscript",
        "formula" | "chemical" => "formula",
        _ => "bold",
    }
    .to_string()
}

fn collect_selected_bracket_kind_target_ids(
    objects: &[SceneObject],
    selected: &BTreeSet<String>,
    out: &mut Vec<String>,
) {
    for object in objects {
        if selected.contains(&object.id) {
            if object.object_type == "bracket" {
                out.push(object.id.clone());
            } else if object.object_type == "group"
                && object.meta.get("kind").and_then(JsonValue::as_str) == Some("bracket-group")
            {
                collect_child_bracket_ids(object, out);
            }
        }
        collect_selected_bracket_kind_target_ids(&object.children, selected, out);
    }
}

fn collect_child_bracket_ids(object: &SceneObject, out: &mut Vec<String>) {
    for child in &object.children {
        if child.object_type == "bracket" {
            out.push(child.id.clone());
        }
        collect_child_bracket_ids(child, out);
    }
}

fn selected_object_style_color(document: &ChemcoreDocument, object: &SceneObject) -> String {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| {
            style_string_value(style, "stroke")
                .or_else(|| style_string_value(style, "fill"))
                .or_else(|| style_string_value(style, "color"))
        })
        .or_else(|| payload_string(&object.payload, "stroke"))
        .or_else(|| payload_string(&object.payload, "fill"))
        .unwrap_or_else(|| "#000000".to_string())
}

fn shape_style_json(style: &str, color: &str, stroke_width: f64, dash_spacing: f64) -> JsonValue {
    match style {
        "dashed" => json!({
            "kind": "shape",
            "fill": null,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [dash_spacing],
        }),
        "filled" => json!({
            "kind": "shape",
            "fill": color,
            "stroke": null,
            "strokeWidth": 0.0,
            "dashArray": [],
        }),
        "shaded" => json!({
            "kind": "shape",
            "fill": color,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [],
            "shaded": true,
        }),
        "faded" => json!({
            "kind": "shape",
            "fill": null,
            "stroke": faded_color(color),
            "strokeWidth": stroke_width,
            "dashArray": [],
            "faded": true,
        }),
        "shadowed" => json!({
            "kind": "shape",
            "fill": null,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [],
            "shadow": true,
            "shadowSize": 4.0,
        }),
        _ => json!({
            "kind": "shape",
            "fill": null,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [],
        }),
    }
}

fn orbital_style_json(style: &str, color: &str, stroke_width: f64) -> JsonValue {
    match style {
        "filled" => json!({
            "kind": "shape",
            "fill": color,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [],
        }),
        "shaded" => json!({
            "kind": "shape",
            "fill": color,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [],
            "shaded": true,
        }),
        _ => json!({
            "kind": "shape",
            "fill": null,
            "stroke": color,
            "strokeWidth": stroke_width,
            "dashArray": [],
        }),
    }
}

fn line_style_json(style: &str, color: &str, stroke_width: f64, dash_spacing: f64) -> JsonValue {
    let width = if style == "bold" {
        (stroke_width * 2.0).max(stroke_width)
    } else {
        stroke_width
    };
    json!({
        "kind": "stroke",
        "stroke": color,
        "strokeWidth": width,
        "lineCap": "butt",
        "lineJoin": "miter",
        "dashArray": if style == "dashed" { json!([dash_spacing]) } else { json!([]) },
    })
}

fn apply_bond_style_key(bond: &mut Bond, style: &str, bold_width: f64, wedge_width: f64) -> bool {
    let before = serde_json::to_value(&*bond).ok();
    bond.bold_width = Some(bold_width);
    bond.wedge_width = Some(wedge_width);
    match style {
        "single-dashed" => {
            set_single_common(bond);
            bond.line_styles.main = BondLinePattern::Dashed;
        }
        "single-hashed" => {
            set_single_common(bond);
            bond.line_styles.main = BondLinePattern::Dashed;
            bond.meta = merge_object_meta_string(bond.meta.clone(), "contextMenuBondStyle", style);
        }
        "single-hashed-wedged" => {
            set_single_common(bond);
            bond.stereo = Some(BondStereo {
                kind: "hashed-wedge".to_string(),
                wide_end: existing_wide_end(bond),
            });
        }
        "single-bold" => {
            set_single_common(bond);
            bond.line_weights.main = BondLineWeight::Bold;
        }
        "single-bold-wedged" => {
            set_single_common(bond);
            bond.stereo = Some(BondStereo {
                kind: "solid-wedge".to_string(),
                wide_end: existing_wide_end(bond),
            });
        }
        "single-hollow-wedged" => {
            set_single_common(bond);
            bond.stereo = Some(BondStereo {
                kind: "hollow-wedge".to_string(),
                wide_end: existing_wide_end(bond),
            });
        }
        "single-wavy" => {
            set_single_common(bond);
            replace_with_plain_wavy_bond_style(bond);
        }
        "double-left" => set_double_common(bond, DoubleBondPlacement::Left),
        "double-right" => set_double_common(bond, DoubleBondPlacement::Right),
        "double-center" => set_double_common(bond, DoubleBondPlacement::Center),
        "double-bold" => {
            set_double_common(bond, DoubleBondPlacement::Center);
            bond.line_weights.main = BondLineWeight::Bold;
        }
        "double-dashed" => {
            set_double_common(bond, DoubleBondPlacement::Center);
            bond.line_styles.main = BondLinePattern::Dashed;
        }
        "double-double-dashed" => {
            set_double_common(bond, DoubleBondPlacement::Center);
            bond.line_styles.left = BondLinePattern::Dashed;
            bond.line_styles.right = BondLinePattern::Dashed;
        }
        "triple-plain" => {
            bond.order = 3;
            bond.double = None;
            bond.stereo = None;
            bond.line_styles = BondLineStyles::default();
            bond.line_weights = BondLineWeights::default();
            bond.meta = clear_object_meta_key(bond.meta.clone(), "contextMenuBondStyle");
        }
        _ => {
            set_single_common(bond);
        }
    }
    serde_json::to_value(&*bond).ok() != before
}

fn set_single_common(bond: &mut Bond) {
    bond.order = 1;
    bond.double = None;
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.meta = clear_object_meta_key(bond.meta.clone(), "contextMenuBondStyle");
}

fn set_double_common(bond: &mut Bond, placement: DoubleBondPlacement) {
    bond.order = 2;
    bond.double = Some(DoubleBond {
        placement,
        center_exit_side: None,
        frozen: true,
    });
    bond.stereo = None;
    bond.line_styles = BondLineStyles::default();
    bond.line_weights = BondLineWeights::default();
    bond.meta = clear_object_meta_key(bond.meta.clone(), "contextMenuBondStyle");
}

fn existing_wide_end(bond: &Bond) -> String {
    bond.stereo
        .as_ref()
        .map(|stereo| stereo.wide_end.clone())
        .unwrap_or_else(|| "end".to_string())
}

fn set_line_arrow_bold(object: &mut SceneObject, bold: bool) -> bool {
    let Some(arrow_head) = object
        .payload
        .extra
        .get_mut("arrowHead")
        .and_then(JsonValue::as_object_mut)
    else {
        return false;
    };
    let previous = arrow_head
        .get("bold")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    if previous == bold {
        return false;
    }
    arrow_head.insert("bold".to_string(), json!(bold));
    true
}

fn apply_text_object_style(object: &mut SceneObject, command: &str, value: &str) -> bool {
    let mut changed = false;
    match command {
        "font-family" => {
            changed |= set_payload_string(&mut object.payload.extra, "fontFamily", value);
            changed |=
                apply_text_object_runs(object, |run| set_style_string(run, "fontFamily", value));
        }
        "font-size" => {
            let Some(size) = parse_positive_number(value) else {
                return false;
            };
            changed |= set_payload_number(&mut object.payload.extra, "fontSize", size);
            changed |=
                apply_text_object_runs(object, |run| set_style_number(run, "fontSize", size));
        }
        "align" => {
            changed |= set_payload_string(
                &mut object.payload.extra,
                "align",
                normalize_text_align(value),
            );
        }
        "line-height" => {
            let Some(line_height) = parse_positive_number(value) else {
                return false;
            };
            changed |= set_payload_number(&mut object.payload.extra, "lineHeight", line_height);
        }
        "italic" => {
            let enabled = parse_enabled_value(value);
            changed |= apply_text_object_runs(object, |run| {
                set_style_string(run, "fontStyle", if enabled { "italic" } else { "normal" })
            });
        }
        "underline" => {
            let enabled = parse_enabled_value(value);
            changed |=
                apply_text_object_runs(object, |run| set_style_bool(run, "underline", enabled));
        }
        "superscript" | "subscript" | "formula" => {
            let script = if parse_enabled_value(value) {
                if command == "formula" {
                    "chemical"
                } else {
                    command
                }
            } else {
                "normal"
            };
            changed |=
                apply_text_object_runs(object, |run| set_style_string(run, "script", script));
        }
        _ => {
            let enabled = parse_enabled_value(value);
            changed |= apply_text_object_runs(object, |run| {
                set_style_number(run, "fontWeight", if enabled { 700.0 } else { 400.0 })
            });
        }
    }
    changed
}

fn apply_text_object_runs<F>(object: &mut SceneObject, mut apply: F) -> bool
where
    F: FnMut(&mut Map<String, JsonValue>) -> bool,
{
    ensure_text_object_runs(object);
    let mut changed = false;
    for key in ["runs", "sourceRuns", "displayRuns"] {
        let Some(runs) = object
            .payload
            .extra
            .get_mut(key)
            .and_then(JsonValue::as_array_mut)
        else {
            continue;
        };
        for run in runs {
            if let Some(run) = run.as_object_mut() {
                changed |= apply(run);
            }
        }
    }
    changed
}

fn ensure_text_object_runs(object: &mut SceneObject) {
    if object
        .payload
        .extra
        .get("runs")
        .and_then(JsonValue::as_array)
        .is_some_and(|runs| !runs.is_empty())
    {
        return;
    }
    let text = payload_string(&object.payload, "text").unwrap_or_default();
    if text.is_empty() {
        return;
    }
    let font_family =
        payload_string(&object.payload, "fontFamily").unwrap_or_else(|| "Arial".to_string());
    let font_size =
        payload_number(&object.payload, "fontSize").unwrap_or(crate::DEFAULT_TEXT_FONT_SIZE_PT);
    let fill = payload_string(&object.payload, "fill").unwrap_or_else(|| "#000000".to_string());
    let run = json!({
        "text": text,
        "fontFamily": font_family,
        "fontSize": font_size,
        "fill": fill,
        "fontWeight": 400,
        "fontStyle": "normal",
        "underline": false,
        "script": "normal",
    });
    object
        .payload
        .extra
        .insert("runs".to_string(), json!([run.clone()]));
    object
        .payload
        .extra
        .insert("sourceRuns".to_string(), json!([run.clone()]));
    object
        .payload
        .extra
        .insert("displayRuns".to_string(), json!([run]));
}

fn apply_node_label_style(label: &mut crate::NodeLabel, command: &str, value: &str) -> bool {
    ensure_label_runs(label);
    let mut changed = false;
    match command {
        "font-family" => {
            changed |= set_option_string(&mut label.font_family, value);
            for_label_runs(label, |run| {
                set_label_run_string(&mut run.font_family, value, "Arial")
            });
        }
        "font-size" => {
            let Some(size) = parse_positive_number(value) else {
                return false;
            };
            changed |= set_option_number(&mut label.font_size, size);
            for_label_runs(label, |run| set_label_run_number(&mut run.font_size, size));
        }
        "align" => {
            changed |= set_option_string(&mut label.align, normalize_text_align(value));
        }
        "italic" => {
            let style = if parse_enabled_value(value) {
                "italic"
            } else {
                "normal"
            };
            for_label_runs(label, |run| {
                set_label_run_string(&mut run.font_style, style, "normal")
            });
            changed = true;
        }
        "underline" => {
            let enabled = parse_enabled_value(value);
            for_label_runs(label, |run| set_label_run_bool(&mut run.underline, enabled));
            changed = true;
        }
        "superscript" | "subscript" | "formula" => {
            let script = if parse_enabled_value(value) {
                if command == "formula" {
                    "chemical"
                } else {
                    command
                }
            } else {
                "normal"
            };
            for_label_runs(label, |run| {
                set_label_run_string(&mut run.script, script, "normal")
            });
            changed = true;
        }
        "line-height" => {}
        _ => {
            let weight = if parse_enabled_value(value) { 700 } else { 400 };
            for_label_runs(label, |run| set_label_run_u32(&mut run.font_weight, weight));
            changed = true;
        }
    }
    changed
}

fn ensure_label_runs(label: &mut crate::NodeLabel) {
    if !label.runs.is_empty() || !label.line_runs.is_empty() || label.text.is_empty() {
        return;
    }
    label.runs.push(crate::LabelRun {
        text: label.text.clone(),
        font_family: label.font_family.clone(),
        font_size: label.font_size,
        fill: label.fill.clone(),
        font_weight: Some(400),
        font_style: Some("normal".to_string()),
        underline: Some(false),
        script: Some("normal".to_string()),
    });
}

fn for_label_runs<F>(label: &mut crate::NodeLabel, mut apply: F)
where
    F: FnMut(&mut crate::LabelRun),
{
    for run in &mut label.runs {
        apply(run);
    }
    for line in &mut label.line_runs {
        for run in line {
            apply(run);
        }
    }
}

fn parse_enabled_value(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "off" | "no" | "normal"
    )
}

fn parse_positive_number(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (parsed.is_finite() && parsed > 0.0).then_some(crate::round2(parsed))
}

fn normalize_text_align(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "center" | "middle" => "center",
        "right" => "right",
        "justify" | "justified" => "justify",
        _ => "left",
    }
}

fn style_string_value(style: &JsonValue, key: &str) -> Option<String> {
    let value = style.get(key)?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(ToString::to_string)
}

fn payload_string(payload: &ObjectPayload, key: &str) -> Option<String> {
    payload
        .extra
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
}

fn payload_number(payload: &ObjectPayload, key: &str) -> Option<f64> {
    payload.extra.get(key).and_then(JsonValue::as_f64)
}

fn set_payload_string(map: &mut BTreeMap<String, JsonValue>, key: &str, value: &str) -> bool {
    if map.get(key).and_then(JsonValue::as_str) == Some(value) {
        return false;
    }
    map.insert(key.to_string(), json!(value));
    true
}

fn set_payload_number(map: &mut BTreeMap<String, JsonValue>, key: &str, value: f64) -> bool {
    if map
        .get(key)
        .and_then(JsonValue::as_f64)
        .is_some_and(|current| (current - value).abs() <= crate::EPSILON)
    {
        return false;
    }
    map.insert(key.to_string(), json!(value));
    true
}

fn set_style_string(map: &mut Map<String, JsonValue>, key: &str, value: &str) -> bool {
    if map.get(key).and_then(JsonValue::as_str) == Some(value) {
        return false;
    }
    map.insert(key.to_string(), json!(value));
    true
}

fn set_style_number(map: &mut Map<String, JsonValue>, key: &str, value: f64) -> bool {
    if map
        .get(key)
        .and_then(JsonValue::as_f64)
        .is_some_and(|current| (current - value).abs() <= crate::EPSILON)
    {
        return false;
    }
    map.insert(key.to_string(), json!(value));
    true
}

fn set_style_bool(map: &mut Map<String, JsonValue>, key: &str, value: bool) -> bool {
    if map.get(key).and_then(JsonValue::as_bool) == Some(value) {
        return false;
    }
    map.insert(key.to_string(), json!(value));
    true
}

fn set_option_string(target: &mut Option<String>, value: &str) -> bool {
    if target.as_deref() == Some(value) {
        return false;
    }
    *target = Some(value.to_string());
    true
}

fn set_option_number(target: &mut Option<f64>, value: f64) -> bool {
    if target.is_some_and(|current| (current - value).abs() <= crate::EPSILON) {
        return false;
    }
    *target = Some(value);
    true
}

fn set_label_run_string(target: &mut Option<String>, value: &str, default: &str) {
    *target = if value == default {
        None
    } else {
        Some(value.to_string())
    };
}

fn set_label_run_number(target: &mut Option<f64>, value: f64) {
    *target = Some(value);
}

fn set_label_run_bool(target: &mut Option<bool>, value: bool) {
    *target = if value { Some(true) } else { None };
}

fn set_label_run_u32(target: &mut Option<u32>, value: u32) {
    *target = if value == 400 { None } else { Some(value) };
}

fn set_value_object_bool(value: &mut JsonValue, key: &str, enabled: bool) -> bool {
    if !value.is_object() {
        *value = json!({});
    }
    let Some(object) = value.as_object_mut() else {
        return false;
    };
    if object.get(key).and_then(JsonValue::as_bool) == Some(enabled) {
        return false;
    }
    object.insert(key.to_string(), json!(enabled));
    true
}

fn merge_object_meta_string(meta: JsonValue, key: &str, value: &str) -> JsonValue {
    let mut object = meta.as_object().cloned().unwrap_or_default();
    object.insert(key.to_string(), json!(value));
    JsonValue::Object(object)
}

fn clear_object_meta_key(meta: JsonValue, key: &str) -> JsonValue {
    let Some(mut object) = meta.as_object().cloned() else {
        return meta;
    };
    object.remove(key);
    JsonValue::Object(object)
}

fn faded_color(color: &str) -> String {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return "#999999".to_string();
    }
    let Ok(red) = u8::from_str_radix(&hex[0..2], 16) else {
        return "#999999".to_string();
    };
    let Ok(green) = u8::from_str_radix(&hex[2..4], 16) else {
        return "#999999".to_string();
    };
    let Ok(blue) = u8::from_str_radix(&hex[4..6], 16) else {
        return "#999999".to_string();
    };
    let blend = |channel: u8| ((channel as f64 * 0.45) + 255.0 * 0.55).round() as u8;
    format!("#{:02x}{:02x}{:02x}", blend(red), blend(green), blend(blue))
}
