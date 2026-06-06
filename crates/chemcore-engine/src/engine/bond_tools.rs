use super::*;

fn refresh_attached_label_geometry_for_bond_endpoints(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    stroke_width: f64,
    begin_id: &str,
    end_id: &str,
) {
    refresh_attached_node_label_geometry_for_node(
        fragment,
        object_translate,
        begin_id,
        stroke_width,
    );
    if end_id != begin_id {
        refresh_attached_node_label_geometry_for_node(
            fragment,
            object_translate,
            end_id,
            stroke_width,
        );
    }
}

impl Engine {
    pub(super) fn document_with_preview_bond(
        &self,
        anchor: &BondAnchor,
        end: &BondAnchor,
        order: u8,
    ) -> Option<ChemcoreDocument> {
        let mut document = self.state.document.clone();
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists_in_document(&document, begin_id, end_id) {
                return None;
            }
        }
        let mut entry = document.editable_fragment_mut()?;
        let begin_id = match &anchor.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(anchor.point);
                let node_id = "__preview_node_begin".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        let end_id = match &end.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(end.point);
                let node_id = "__preview_node_end".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        if begin_id == end_id || self.bond_exists_in_fragment(entry.fragment, &begin_id, &end_id) {
            return None;
        }
        entry.fragment.bonds.push(Bond {
            id: "__preview_bond".to_string(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: self.pending_double_state_for_new_bond(&begin_id, &end_id, order.max(1)),
            stereo: self.pending_bond_stereo(),
            stroke_width: self.options.bond_stroke_world_pt().value(),
            stroke: None,
            bold_width: Some(self.options.bold_bond_width_world_pt().value()),
            wedge_width: Some(self.options.wedge_width_world_pt().value()),
            label_clip_margin: Some(self.options.label_clip_margin_world_pt().value()),
            hash_spacing: Some(self.options.hash_spacing_world_pt().value()),
            bond_spacing: Some(self.options.bond_spacing_percent()),
            margin_width: Some(self.options.margin_width_world_pt().value()),
            line_styles: self.pending_line_styles(),
            line_weights: self.pending_line_weights(),
            meta: serde_json::Value::Null,
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            "__preview_bond",
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            "__preview_bond",
        );
        refresh_attached_label_geometry_for_bond_endpoints(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_pt().value(),
            &begin_id,
            &end_id,
        );
        entry.update_bounds();
        Some(document)
    }

    pub fn cycle_bond_center_style(&mut self, bond_id: &str) -> bool {
        self.with_command(
            EditorCommand::CycleBondStyle {
                bond_id: bond_id.to_string(),
                variant: self.state.tool.bond_variant,
            },
            |engine| engine.cycle_bond_center_style_untracked(bond_id),
        )
    }

    pub(super) fn cycle_bond_center_style_untracked(&mut self, bond_id: &str) -> bool {
        let (current_order, was_double_before) = self
            .state
            .document
            .editable_fragment()
            .and_then(|entry| entry.fragment.bonds.iter().find(|bond| bond.id == bond_id))
            .map(|bond| (bond.order, bond.order == 2 && bond.double.is_some()))
            .unwrap_or((1, false));
        let default_side = self
            .preferred_double_bond_side(bond_id)
            .unwrap_or(DoubleBondPlacement::Right);
        let default_placement =
            if current_order == 1 && self.should_default_center_double_bond(bond_id) {
                DoubleBondPlacement::Center
            } else {
                default_side
            };
        let should_freeze_after_change = was_double_before;
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.id == bond_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let changed = match self.state.tool.bond_variant {
            BondVariant::Single => apply_single_tool_center_style(bond, default_placement),
            BondVariant::Double => apply_double_tool_center_style(bond, default_placement),
            BondVariant::Triple => replace_with_plain_triple_bond_style(bond),
            BondVariant::Dashed => cycle_dashed_bond_center_style(bond, default_placement),
            BondVariant::DashedDouble => {
                cycle_dashed_double_bond_tool_center_style(bond, default_placement)
            }
            BondVariant::Bold => cycle_bold_bond_center_style(bond, default_placement),
            BondVariant::BoldDashed => replace_with_bold_dashed_bond_style(bond),
            BondVariant::Wavy => replace_with_plain_wavy_bond_style(bond),
            BondVariant::Wedge | BondVariant::HashedWedge | BondVariant::HollowWedge => {
                replace_with_stereo_bond_style(bond, self.state.tool.bond_variant)
            }
        };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        if let Some(double) = bond.double.as_mut() {
            double.frozen = should_freeze_after_change;
        }
        let Some((begin_id, end_id)) = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .map(|bond| (bond.begin.clone(), bond.end.clone()))
        else {
            self.undo_stack.pop();
            return false;
        };
        refresh_attached_label_geometry_for_bond_endpoints(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_pt().value(),
            &begin_id,
            &end_id,
        );
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.note_pending_select_target(PendingSelectTarget::MoleculeBond(bond_id.to_string()));
        true
    }
}
