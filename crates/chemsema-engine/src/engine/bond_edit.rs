use super::*;

impl Engine {
    pub fn add_single_bond(&mut self, anchor: BondAnchor, end: Point) {
        self.add_bond_between(
            anchor.clone(),
            BondAnchor {
                node_id: None,
                object_id: anchor.object_id.clone(),
                point: end,
                label_anchor: None,
            },
            1,
        );
    }

    pub fn add_single_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor) -> bool {
        self.add_bond_between(anchor, end, 1)
    }

    pub fn add_bond_between(&mut self, anchor: BondAnchor, end: BondAnchor, order: u8) -> bool {
        self.add_bond_between_with_double_override(anchor, end, order, None, None)
    }

    pub(super) fn add_bond_between_with_double_override(
        &mut self,
        anchor: BondAnchor,
        end: BondAnchor,
        order: u8,
        explicit_double: Option<DoubleBond>,
        line_weights_override: Option<crate::BondLineWeights>,
    ) -> bool {
        self.add_bond_between_with_style_override(
            anchor,
            end,
            order,
            None,
            explicit_double,
            line_weights_override,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_bond_between_with_style_override(
        &mut self,
        anchor: BondAnchor,
        end: BondAnchor,
        order: u8,
        wide_end_override: Option<String>,
        explicit_double: Option<DoubleBond>,
        line_weights_override: Option<crate::BondLineWeights>,
        stroke_override: Option<String>,
        endpoint_attachments: Option<serde_json::Value>,
    ) -> bool {
        let command = EditorCommand::AddBond {
            begin: CommandAnchor::from(&anchor),
            end: CommandAnchor::from(&end),
            order,
            variant: self.state.tool.bond_variant,
            wide_end: wide_end_override.clone(),
            double_placement: explicit_double.as_ref().map(|double| double.placement),
            double: None,
            line_weights: line_weights_override.clone(),
            stroke: stroke_override.clone(),
            endpoint_attachments: endpoint_attachments.clone(),
        };
        self.with_command(command, |engine| {
            engine.add_bond_between_untracked(
                anchor,
                end,
                order,
                wide_end_override,
                explicit_double,
                line_weights_override,
                stroke_override,
                endpoint_attachments,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_bond_between_untracked(
        &mut self,
        anchor: BondAnchor,
        end: BondAnchor,
        order: u8,
        wide_end_override: Option<String>,
        explicit_double: Option<DoubleBond>,
        line_weights_override: Option<crate::BondLineWeights>,
        stroke_override: Option<String>,
        endpoint_attachments: Option<serde_json::Value>,
    ) -> bool {
        if anchor
            .object_id
            .as_ref()
            .zip(end.object_id.as_ref())
            .is_some_and(|(left, right)| left != right)
        {
            return false;
        }
        let target_anchor = if anchor.node_id.is_some() || anchor.object_id.is_some() {
            &anchor
        } else {
            &end
        };
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists_for_anchor(target_anchor, begin_id, end_id) {
                return false;
            }
        }
        self.push_undo_snapshot();
        self.state.selection = SelectionState::default();
        let begin_id = match &anchor.node_id {
            Some(node_id) => node_id.clone(),
            None => self.insert_carbon_for_anchor(target_anchor, anchor.point),
        };
        let end_id = match &end.node_id {
            Some(node_id) => node_id.clone(),
            None => self.insert_carbon_for_anchor(target_anchor, end.point),
        };
        if begin_id == end_id || self.bond_exists_for_anchor(target_anchor, &begin_id, &end_id) {
            self.undo_stack.pop();
            return false;
        }
        let bond_id = self.next_id("b");
        let pending_line_styles = self.pending_line_styles();
        let pending_line_weights =
            line_weights_override.unwrap_or_else(|| self.pending_line_weights());
        let pending_stereo = self.pending_bond_stereo_with_wide_end(wide_end_override.as_deref());
        let order = order.max(1);
        let pending_double = if order >= 2 { explicit_double } else { None }.or_else(|| {
            self.pending_double_state_for_new_bond_in_anchor_fragment(
                target_anchor,
                &begin_id,
                &end_id,
                order,
            )
        });
        let stroke_width = self.options.bond_stroke_world_pt().value();
        let bold_width = self.options.bold_bond_width_world_pt().value();
        let wedge_width = self.options.wedge_width_world_pt().value();
        let hash_spacing = self.options.hash_spacing_world_pt().value();
        let bond_spacing = self.options.bond_spacing_percent();
        let margin_width = self.options.margin_width_world_pt().value();
        let mut entry = self
            .editable_fragment_mut_for_anchor(target_anchor)
            .expect("blank document always has an editable fragment");
        entry.fragment.bonds.push(Bond {
            id: bond_id.clone(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order,
            highlight_color: None,
            properties: Default::default(),
            double: pending_double,
            stereo: pending_stereo,
            stroke_width,
            stroke: stroke_override,
            bold_width: Some(bold_width),
            wedge_width: Some(wedge_width),
            label_clip_margin: None,
            hash_spacing: Some(hash_spacing),
            bond_spacing: Some(bond_spacing),
            bond_spacing_absolute: None,
            margin_width: Some(margin_width),
            line_styles: pending_line_styles,
            line_weights: pending_line_weights,
            meta: endpoint_attachments
                .map(|attachments| serde_json::json!({ "endpointAttachments": attachments }))
                .unwrap_or(serde_json::Value::Null),
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            &bond_id,
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            &bond_id,
        );
        refresh_attached_node_label_geometry_for_node(
            entry.fragment,
            entry.object.transform.translate,
            &begin_id,
            stroke_width,
        );
        if end_id != begin_id {
            refresh_attached_node_label_geometry_for_node(
                entry.fragment,
                entry.object.transform.translate,
                &end_id,
                stroke_width,
            );
        }
        entry.update_bounds();
        self.note_pending_select_target(PendingSelectTarget::MoleculeBond(bond_id));
        true
    }

    pub(super) fn preview_document(&self) -> Option<ChemSemaDocument> {
        if let Some(preview_document) = self.selection_move_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.template_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.table_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.shape_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.orbital_preview_document() {
            return Some(preview_document);
        }
        if let Some(drag) = self.arrow_drag.as_ref().filter(|drag| drag.has_dragged) {
            let end = drag.end?;
            let mut document = self.state.document.clone();
            let style_id = self.arrow_style_id();
            ensure_arrow_style(&mut document, &style_id, self.options.graphic_stroke_width);
            document.objects.push(self.arrow_scene_object(
                drag.start,
                end,
                "__preview_arrow".to_string(),
                style_id,
            ));
            return Some(document);
        }
        let drag = self.drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let end_anchor = if let Some(target) = drag.target.clone() {
            target
        } else {
            BondAnchor {
                node_id: None,
                object_id: drag.anchor.object_id.clone(),
                point: drag.preview_end?,
                label_anchor: None,
            }
        };
        self.document_with_preview_bond(&drag.anchor, &end_anchor, self.pending_bond_order())
    }

    pub(super) fn preview_document_shell(&self) -> ChemSemaDocument {
        ChemSemaDocument {
            format: self.state.document.format.clone(),
            document: self.state.document.document.clone(),
            style: self.state.document.style.clone(),
            styles: self.state.document.styles.clone(),
            objects: Vec::new(),
            links: Vec::new(),
            orders: Default::default(),
            logical_objects: Default::default(),
            reaction_schemes: Vec::new(),
            chemical_properties: Vec::new(),
            resources: BTreeMap::new(),
            interchange: BTreeMap::new(),
        }
    }

    pub(super) fn preview_overlay_document(&self) -> Option<ChemSemaDocument> {
        if let Some(preview_document) = self.template_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.table_preview_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.shape_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.orbital_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(preview_document) = self.bracket_preview_overlay_document() {
            return Some(preview_document);
        }
        if let Some(drag) = self.arrow_drag.as_ref().filter(|drag| drag.has_dragged) {
            let end = drag.end?;
            let mut document = self.preview_document_shell();
            let style_id = self.arrow_style_id();
            ensure_arrow_style(&mut document, &style_id, self.options.graphic_stroke_width);
            document.objects.push(self.arrow_scene_object(
                drag.start,
                end,
                "__preview_arrow".to_string(),
                style_id,
            ));
            return Some(document);
        }
        self.preview_bond_overlay_document()
    }

    pub(super) fn drag_target_endpoint(
        &self,
        anchor: &BondAnchor,
        point: Point,
    ) -> Option<EndpointHit> {
        hit_test_endpoint_excluding(
            &self.state.document,
            point,
            self.endpoint_hit_radius(),
            anchor.node_id.as_deref(),
        )
    }

    pub(super) fn endpoint_anchor_near(
        &self,
        anchor: &BondAnchor,
        point: Point,
    ) -> Option<BondAnchor> {
        let target = self.drag_target_endpoint(anchor, point)?;
        Some(BondAnchor {
            node_id: Some(target.node_id),
            object_id: Some(target.object_id),
            point: target.point,
            label_anchor: target.label_anchor,
        })
    }

    pub(super) fn editable_fragment_for_anchor(
        &self,
        anchor: &BondAnchor,
    ) -> Option<EditableFragment<'_>> {
        if let Some(object_id) = anchor.object_id.as_deref() {
            if let Some(entry) = self
                .state
                .document
                .editable_fragments()
                .into_iter()
                .find(|entry| entry.object.id == object_id)
            {
                return Some(entry);
            }
        }
        if let Some(node_id) = anchor.node_id.as_deref() {
            if let Some(entry) = self
                .state
                .document
                .editable_fragments()
                .into_iter()
                .find(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
            {
                return Some(entry);
            }
        }
        self.state.document.editable_fragment()
    }

    pub(super) fn editable_fragment_object_id_for_anchor(
        &self,
        anchor: &BondAnchor,
    ) -> Option<String> {
        self.editable_fragment_for_anchor(anchor)
            .map(|entry| entry.object.id.clone())
    }

    pub(super) fn editable_fragment_mut_for_anchor(
        &mut self,
        anchor: &BondAnchor,
    ) -> Option<EditableFragmentMut<'_>> {
        let object_id = self.editable_fragment_object_id_for_anchor(anchor)?;
        if self.state.document.find_scene_object(&object_id).is_some() {
            self.state
                .document
                .editable_fragment_mut_for_object(&object_id)
        } else {
            self.state.document.editable_fragment_mut()
        }
    }

    pub(super) fn bond_exists_for_anchor(
        &self,
        anchor: &BondAnchor,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        self.editable_fragment_for_anchor(anchor)
            .is_some_and(|entry| self.bond_exists_in_fragment(entry.fragment, begin_id, end_id))
    }

    pub(super) fn bond_exists_in_document(
        &self,
        document: &ChemSemaDocument,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        let Some(entry) = document.editable_fragment() else {
            return false;
        };
        self.bond_exists_in_fragment(entry.fragment, begin_id, end_id)
    }

    pub(super) fn bond_exists_in_fragment(
        &self,
        fragment: &crate::MoleculeFragment,
        begin_id: &str,
        end_id: &str,
    ) -> bool {
        fragment.bonds.iter().any(|bond| {
            (bond.begin == begin_id && bond.end == end_id)
                || (bond.begin == end_id && bond.end == begin_id)
        })
    }

    pub(super) fn insert_carbon_for_anchor(&mut self, anchor: &BondAnchor, point: Point) -> String {
        let node_id = self.next_id("n");
        let entry = self
            .editable_fragment_mut_for_anchor(anchor)
            .expect("blank document always has an editable fragment");
        let local = entry.local_point(point);
        entry
            .fragment
            .nodes
            .push(crate::Node::carbon(node_id.clone(), local));
        node_id
    }

    pub(super) fn insert_periodic_element(&mut self, point: Point) -> bool {
        let Some((element, atomic_number)) = element_symbol_info(&self.state.tool.element_symbol)
        else {
            return false;
        };
        self.push_undo_snapshot();
        let node_id = self.next_id("n");
        let entry = self
            .state
            .document
            .editable_fragment_mut()
            .expect("blank document always has an editable fragment");
        let local = entry.local_point(point);
        let num_hydrogens = standalone_element_hydrogen_count(atomic_number);
        let label_text = implicit_hydrogen_label_text_for_count(element, num_hydrogens);
        let label = if element == "C" && num_hydrogens == 0 {
            None
        } else {
            Some(make_periodic_element_node_label(
                &label_text,
                [local.x, local.y],
            ))
        };
        let mut node = crate::Node {
            id: node_id.clone(),
            element: element.to_string(),
            atomic_number,
            position: [round2(local.x), round2(local.y)],
            charge: 0,
            num_hydrogens,
            highlight_color: None,
            external_connection: None,
            is_placeholder: false,
            label,
            atom_properties: crate::AtomProperties::default(),
            nmr_assignments: Vec::new(),
            meta: serde_json::Value::Null,
        };
        mark_shortcut_implicit_hydrogen_label(&mut node, &label_text);
        entry.fragment.nodes.push(node);
        self.state.selection = SelectionState::default();
        true
    }

    pub(super) fn preferred_double_bond_side(&self, bond_id: &str) -> Option<DoubleBondPlacement> {
        let entry = self.state.document.editable_fragment()?;
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id && (bond.order == 1 || bond.order == 2))?;
        preferred_double_bond_side_for_segment(
            entry.fragment,
            &bond.begin,
            &bond.end,
            Some(&bond.id),
        )
    }

    pub(super) fn should_default_center_double_bond(&self, bond_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id && (bond.order == 1 || bond.order == 2))
        else {
            return false;
        };
        should_default_center_double_bond_for_segment(
            entry.fragment,
            &bond.begin,
            &bond.end,
            Some(&bond.id),
        )
    }

    pub(super) fn pending_bond_order(&self) -> u8 {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble => 2,
            BondVariant::Triple => 3,
            _ => 1,
        }
    }

    pub(super) fn pending_double_state_for_new_bond(
        &self,
        begin_id: &str,
        end_id: &str,
        order: u8,
    ) -> Option<DoubleBond> {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble if order >= 2 => {
                let placement = if self.should_default_center_for_new_bond(begin_id, end_id) {
                    DoubleBondPlacement::Center
                } else {
                    let entry = self.state.document.editable_fragment()?;
                    automatic_double_bond_placement_for_segment(
                        entry.fragment,
                        begin_id,
                        end_id,
                        None,
                    )
                };
                Some(DoubleBond {
                    placement,
                    center_exit_side: None,
                    frozen: false,
                })
            }
            _ => None,
        }
    }

    pub(super) fn pending_double_state_for_new_bond_in_anchor_fragment(
        &self,
        anchor: &BondAnchor,
        begin_id: &str,
        end_id: &str,
        order: u8,
    ) -> Option<DoubleBond> {
        match self.state.tool.bond_variant {
            BondVariant::Double | BondVariant::DashedDouble if order >= 2 => {
                let entry = self.editable_fragment_for_anchor(anchor)?;
                let placement = if should_default_center_double_bond_for_segment(
                    entry.fragment,
                    begin_id,
                    end_id,
                    None,
                ) {
                    DoubleBondPlacement::Center
                } else {
                    automatic_double_bond_placement_for_segment(
                        entry.fragment,
                        begin_id,
                        end_id,
                        None,
                    )
                };
                Some(DoubleBond {
                    placement,
                    center_exit_side: None,
                    frozen: false,
                })
            }
            _ => None,
        }
    }

    pub(super) fn should_default_center_for_new_bond(&self, begin_id: &str, end_id: &str) -> bool {
        let Some(entry) = self.state.document.editable_fragment() else {
            return false;
        };
        should_default_center_double_bond_for_segment(entry.fragment, begin_id, end_id, None)
    }

    pub(super) fn pending_line_styles(&self) -> BondLineStyles {
        match self.state.tool.bond_variant {
            BondVariant::Dashed => {
                return BondLineStyles {
                    main: BondLinePattern::Dashed,
                    ..BondLineStyles::default()
                };
            }
            BondVariant::BoldDashed => {
                return BondLineStyles {
                    main: BondLinePattern::Hash,
                    ..BondLineStyles::default()
                };
            }
            BondVariant::DashedDouble => {
                return BondLineStyles {
                    right: BondLinePattern::Dashed,
                    ..BondLineStyles::default()
                };
            }
            BondVariant::Wavy => {
                return BondLineStyles {
                    main: BondLinePattern::Wavy,
                    ..BondLineStyles::default()
                };
            }
            _ => {}
        }
        BondLineStyles::default()
    }

    pub(super) fn pending_bond_stereo(&self) -> Option<BondStereo> {
        self.pending_bond_stereo_with_wide_end(None)
    }

    pub(super) fn pending_bond_stereo_with_wide_end(
        &self,
        wide_end_override: Option<&str>,
    ) -> Option<BondStereo> {
        let wide_end = match wide_end_override {
            Some("begin") => "begin",
            Some("end") => "end",
            _ => "end",
        };
        match self.state.tool.bond_variant {
            BondVariant::Wedge => Some(BondStereo {
                kind: "solid-wedge".to_string(),
                wide_end: wide_end.to_string(),
            }),
            BondVariant::HashedWedge => Some(BondStereo {
                kind: "hashed-wedge".to_string(),
                wide_end: wide_end.to_string(),
            }),
            BondVariant::HollowWedge => Some(BondStereo {
                kind: "hollow-wedge".to_string(),
                wide_end: wide_end.to_string(),
            }),
            _ => None,
        }
    }

    pub(super) fn pending_line_weights(&self) -> BondLineWeights {
        match self.state.tool.bond_variant {
            BondVariant::Bold => {
                return BondLineWeights {
                    main: BondLineWeight::Bold,
                    ..BondLineWeights::default()
                };
            }
            _ => {}
        }
        BondLineWeights::default()
    }
}
