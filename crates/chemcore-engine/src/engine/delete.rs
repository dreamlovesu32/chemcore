use super::text_edit::{
    apply_node_label_text_edit, refresh_attached_node_label_geometry_for_all_nodes,
    refresh_attached_node_label_geometry_for_node, TextEditSession, TextEditTarget,
};
use super::{EditorCommand, Engine};
use std::collections::BTreeSet;

const DEFAULT_TEXT_FONT_FAMILY: &str = "Arial";
const DEFAULT_TEXT_FONT_SIZE: f64 = crate::DEFAULT_TEXT_FONT_SIZE_PT;
const DEFAULT_TEXT_FILL: &str = "#000000";
const DEFAULT_TEXT_LINE_HEIGHT: f64 = crate::DEFAULT_TEXT_LINE_HEIGHT_PT;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FocusedDeleteMode {
    DeleteToolClick,
    CommandKey,
}

impl Engine {
    pub fn delete_selection(&mut self) -> bool {
        self.with_command(EditorCommand::DeleteSelection, |engine| {
            engine.delete_selection_untracked()
        })
    }

    fn delete_selection_untracked(&mut self) -> bool {
        if self.state.selection.is_empty() {
            return self.delete_focused(FocusedDeleteMode::CommandKey);
        }
        let selection = self.state.selection.clone();
        let mut changed = false;
        for object_id in &selection.text_objects {
            changed |= self.remove_text_object(Some(object_id.as_str()));
        }
        if !selection.arrow_objects.is_empty() {
            self.push_undo_snapshot();
            let selected_graphics: BTreeSet<&str> =
                selection.arrow_objects.iter().map(String::as_str).collect();
            let removed = self
                .state
                .document
                .remove_scene_objects_by_id(&selected_graphics);
            let arrow_changed = removed > 0;
            changed |= arrow_changed;
            if !arrow_changed {
                self.undo_stack.pop();
            } else {
                self.refresh_symbol_chemistry();
            }
        }
        for node_id in &selection.label_nodes {
            changed |= self.remove_endpoint_label(node_id);
        }
        if selection.nodes.is_empty() && selection.bonds.is_empty() {
            if changed {
                self.state.selection = crate::SelectionState::default();
                self.clear_interaction();
            }
            return changed;
        }
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return changed;
        };

        delete_fragment_selection(
            entry.fragment,
            selection.nodes.into_iter().collect(),
            selection.bonds.into_iter().collect(),
        );
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        drop(entry);
        self.refresh_symbol_chemistry();
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }

    fn delete_focused(&mut self, mode: FocusedDeleteMode) -> bool {
        if let Some(hover) = self.state.overlay.hover_text_box.clone() {
            if let Some(object_id) = hover.object_id {
                return self.remove_text_object(Some(object_id.as_str()));
            }
        }
        if let Some(hover) = self.state.overlay.hover_endpoint.clone() {
            if hover.label_anchor.is_some() {
                return self.remove_endpoint_label(&hover.node_id);
            }
            return match mode {
                FocusedDeleteMode::DeleteToolClick => {
                    self.remove_endpoint_connected_bonds(&hover.node_id)
                }
                FocusedDeleteMode::CommandKey => {
                    self.remove_endpoint_and_connected_bonds_for_delete_key(&hover.node_id)
                }
            };
        }
        if let Some(hover) = self.state.overlay.hover_bond_center.clone() {
            return match mode {
                FocusedDeleteMode::DeleteToolClick => {
                    self.reduce_or_delete_bond_in_delete_mode(&hover.bond_id)
                }
                FocusedDeleteMode::CommandKey => self.remove_bond(&hover.bond_id),
            };
        }
        false
    }

    pub(super) fn delete_focused_at_point(
        &mut self,
        point: crate::Point,
        mode: FocusedDeleteMode,
    ) -> bool {
        if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
            self.state.overlay.hover_text_box = Some(crate::HoverTextBox {
                bounds,
                object_id: Some(object_id.clone()),
                node_id: None,
            });
            return self.remove_text_object(Some(object_id.as_str()));
        }
        if let Some(endpoint) =
            crate::hit_test_endpoint(&self.state.document, point, crate::ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint.clone());
            if endpoint.label_anchor.is_some() {
                return self.remove_endpoint_label(&endpoint.node_id);
            }
            return match mode {
                FocusedDeleteMode::DeleteToolClick => {
                    self.remove_endpoint_connected_bonds(&endpoint.node_id)
                }
                FocusedDeleteMode::CommandKey => {
                    self.remove_endpoint_and_connected_bonds_for_delete_key(&endpoint.node_id)
                }
            };
        }
        if let Some(center) =
            crate::hit_test_bond_center(&self.state.document, point, crate::BOND_CENTER_HIT_RADIUS)
        {
            self.state.overlay.hover_bond_center = Some(center.clone());
            return match mode {
                FocusedDeleteMode::DeleteToolClick => {
                    self.reduce_or_delete_bond_in_delete_mode(&center.bond_id)
                }
                FocusedDeleteMode::CommandKey => self.remove_bond(&center.bond_id),
            };
        }
        false
    }

    fn remove_endpoint_label(&mut self, node_id: &str) -> bool {
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(node_index) = entry
            .fragment
            .nodes
            .iter()
            .position(|node| node.id == node_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let connection_angles = adjacent_angles_for_fragment_node(entry.fragment, node_id);
        let node_position = entry.fragment.nodes[node_index].position;
        let session = TextEditSession {
            target: TextEditTarget::EndpointLabel {
                node_id: node_id.to_string(),
                x: object_translate[0] + node_position[0],
                y: object_translate[1] + node_position[1],
            },
            text: "C".to_string(),
            source_runs: Vec::new(),
            font_family: Some(DEFAULT_TEXT_FONT_FAMILY.to_string()),
            font_size: Some(DEFAULT_TEXT_FONT_SIZE),
            fill: Some(DEFAULT_TEXT_FILL.to_string()),
            align: Some("left".to_string()),
            line_height: Some(DEFAULT_TEXT_LINE_HEIGHT),
            box_value: None,
            anchor_offset: None,
            preserve_lines: true,
            default_chemical: true,
        };
        let changed = {
            let node = &mut entry.fragment.nodes[node_index];
            apply_node_label_text_edit(node, "C", &session, &connection_angles, node_position)
        };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            object_translate,
            node_id,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        self.state.selection = crate::SelectionState::default();
        self.drag = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_endpoint = Some(crate::EndpointHit {
            node_id: node_id.to_string(),
            point: crate::Point::new(
                object_translate[0] + node_position[0],
                object_translate[1] + node_position[1],
            ),
            distance: 0.0,
            label_anchor: None,
        });
        true
    }

    fn remove_endpoint_connected_bonds(&mut self, node_id: &str) -> bool {
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let removed_any = entry
            .fragment
            .bonds
            .iter()
            .any(|bond| bond.begin == node_id || bond.end == node_id);
        if !removed_any {
            self.undo_stack.pop();
            return false;
        }
        entry
            .fragment
            .bonds
            .retain(|bond| bond.begin != node_id && bond.end != node_id);
        prune_unconnected_fragment_nodes(entry.fragment);
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            object_translate,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }

    fn remove_endpoint_and_connected_bonds_for_delete_key(&mut self, node_id: &str) -> bool {
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let node_exists = entry.fragment.nodes.iter().any(|node| node.id == node_id);
        if !node_exists {
            self.undo_stack.pop();
            return false;
        }
        entry
            .fragment
            .bonds
            .retain(|bond| bond.begin != node_id && bond.end != node_id);
        entry.fragment.nodes.retain(|node| node.id != node_id);
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            object_translate,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }

    fn remove_bond(&mut self, bond_id: &str) -> bool {
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let removed = delete_fragment_selection(
            entry.fragment,
            BTreeSet::new(),
            [bond_id.to_string()].into_iter().collect(),
        );
        if !removed {
            self.undo_stack.pop();
            return false;
        }
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            object_translate,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }

    fn reduce_or_delete_bond_in_delete_mode(&mut self, bond_id: &str) -> bool {
        let (order, placement) = self
            .state
            .document
            .editable_fragment()
            .and_then(|entry| entry.fragment.bonds.iter().find(|bond| bond.id == bond_id))
            .map(|bond| {
                (
                    bond.order.max(1),
                    bond.double
                        .as_ref()
                        .map(|double| double.placement)
                        .filter(|placement| *placement != crate::DoubleBondPlacement::Center),
                )
            })
            .unwrap_or((0, None));
        if order <= 1 {
            return self.remove_bond(bond_id);
        }

        let default_side = placement
            .or_else(|| self.preferred_double_bond_side(bond_id))
            .unwrap_or(crate::DoubleBondPlacement::Right);
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let object_translate = entry.object.transform.translate;
        let Some(bond) = entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.id == bond_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let changed = if bond.order == 2 {
            downgrade_bond_to_single_for_delete(bond)
        } else {
            downgrade_bond_to_side_double_for_delete(bond, default_side)
        };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        refresh_attached_node_label_geometry_for_all_nodes(
            entry.fragment,
            object_translate,
            self.options.bond_stroke_world_pt().value(),
        );
        entry.update_bounds();
        self.state.selection = crate::SelectionState::default();
        self.clear_interaction();
        true
    }
}

fn delete_fragment_selection(
    fragment: &mut crate::MoleculeFragment,
    selected_nodes: BTreeSet<String>,
    selected_bonds: BTreeSet<String>,
) -> bool {
    if selected_nodes.is_empty() && selected_bonds.is_empty() {
        return false;
    }

    let mut original_degree: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut selected_bond_degree: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for bond in &fragment.bonds {
        *original_degree.entry(bond.begin.clone()).or_default() += 1;
        *original_degree.entry(bond.end.clone()).or_default() += 1;
        if selected_bonds.contains(&bond.id) {
            *selected_bond_degree.entry(bond.begin.clone()).or_default() += 1;
            *selected_bond_degree.entry(bond.end.clone()).or_default() += 1;
        }
    }

    let mut bonds_to_remove = selected_bonds.clone();
    for bond in &fragment.bonds {
        if selected_nodes.contains(&bond.begin) || selected_nodes.contains(&bond.end) {
            bonds_to_remove.insert(bond.id.clone());
        }
    }

    let mut nodes_to_remove = selected_nodes;
    for (node_id, selected_degree) in selected_bond_degree {
        if original_degree.get(&node_id).copied().unwrap_or_default() == selected_degree {
            nodes_to_remove.insert(node_id);
        }
    }

    let previous_bonds = fragment.bonds.len();
    let previous_nodes = fragment.nodes.len();
    fragment
        .bonds
        .retain(|bond| !bonds_to_remove.contains(&bond.id));
    fragment
        .nodes
        .retain(|node| !nodes_to_remove.contains(&node.id));
    fragment.bonds.len() != previous_bonds || fragment.nodes.len() != previous_nodes
}

fn prune_unconnected_fragment_nodes(fragment: &mut crate::MoleculeFragment) {
    let connected_nodes: BTreeSet<String> = fragment
        .bonds
        .iter()
        .flat_map(|bond| [bond.begin.clone(), bond.end.clone()])
        .collect();
    fragment
        .nodes
        .retain(|node| connected_nodes.contains(&node.id));
}

fn adjacent_angles_for_fragment_node(
    fragment: &crate::MoleculeFragment,
    node_id: &str,
) -> Vec<f64> {
    let Some(node) = fragment.nodes.iter().find(|node| node.id == node_id) else {
        return Vec::new();
    };
    let point = crate::Point::new(node.position[0], node.position[1]);
    let mut out = Vec::new();
    for bond in &fragment.bonds {
        if bond.begin != node_id && bond.end != node_id {
            continue;
        }
        let other_id = if bond.begin == node_id {
            &bond.end
        } else {
            &bond.begin
        };
        let Some(other) = fragment.nodes.iter().find(|node| &node.id == other_id) else {
            continue;
        };
        out.push(crate::angle_between(
            point,
            crate::Point::new(other.position[0], other.position[1]),
        ));
    }
    out
}

fn downgrade_bond_to_single_for_delete(bond: &mut crate::Bond) -> bool {
    if bond.order <= 1 {
        return false;
    }
    bond.order = 1;
    bond.double = None;
    true
}

fn downgrade_bond_to_side_double_for_delete(
    bond: &mut crate::Bond,
    placement: crate::DoubleBondPlacement,
) -> bool {
    if bond.order <= 2 {
        return false;
    }
    bond.order = 2;
    bond.double = Some(crate::DoubleBond {
        placement,
        center_exit_side: None,
        frozen: false,
    });
    true
}
