impl Engine {
    pub(super) fn reconcile_logical_relations_after_document_change(&mut self) -> bool {
        let mut changed = self.prune_invalid_logical_references();
        changed |= self.reconcile_automatic_reaction_steps();
        changed
    }

    fn prune_invalid_logical_references(&mut self) -> bool {
        let scene_ids = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .map(|object| object.id.clone())
            .collect::<BTreeSet<_>>();
        let node_ids = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.clone()))
            .collect::<BTreeSet<_>>();
        let bond_ids = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.clone()))
            .collect::<BTreeSet<_>>();
        let entity_exists =
            |id: &String| scene_ids.contains(id) || node_ids.contains(id) || bond_ids.contains(id);
        let mut changed = false;
        for scheme in &mut self.state.document.reaction_schemes {
            for step in &mut scheme.steps {
                changed |= retain_existing(&mut step.reactant_entity_ids, &entity_exists);
                changed |= retain_existing(&mut step.product_entity_ids, &entity_exists);
                changed |= retain_existing(&mut step.arrow_object_ids, &entity_exists);
                changed |= retain_existing(&mut step.plus_object_ids, &entity_exists);
                changed |= retain_existing(&mut step.objects_above_arrow, &entity_exists);
                changed |= retain_existing(&mut step.objects_below_arrow, &entity_exists);
                let before = step.atom_mappings.len();
                step.atom_mappings.retain(|mapping| {
                    node_ids.contains(&mapping.reactant_atom_id)
                        && node_ids.contains(&mapping.product_atom_id)
                });
                changed |= before != step.atom_mappings.len();
                let next_state = if step.reactant_entity_ids.is_empty()
                    || step.product_entity_ids.is_empty()
                    || step.arrow_object_ids.is_empty()
                {
                    ReactionInterpretationState::Invalid
                } else {
                    ReactionInterpretationState::Current
                };
                if step.interpretation_state != next_state {
                    step.interpretation_state = next_state;
                    changed = true;
                }
            }
        }

        let logical = &mut self.state.document.logical_objects;
        for group in &mut logical.alternative_groups {
            changed |= retain_existing(&mut group.member_entity_ids, &entity_exists);
            changed |= retain_existing(&mut group.attachment_node_ids, &|id| node_ids.contains(id));
        }
        for group in &mut logical.bracketed_groups {
            changed |= retain_existing(&mut group.bracket_object_ids, &|id| scene_ids.contains(id));
            changed |= retain_existing(&mut group.bracketed_entity_ids, &entity_exists);
            let attachment_before = group.attachments.len();
            group.attachments.retain_mut(|attachment| {
                let bracket_exists = attachment
                    .bracket_object_id
                    .as_ref()
                    .is_some_and(|id| scene_ids.contains(id))
                    || attachment.unresolved_bracket_source_id.is_some();
                if !bracket_exists {
                    return false;
                }
                let crossing_before = attachment.crossing_bonds.len();
                attachment.crossing_bonds.retain(|crossing| {
                    (crossing
                        .bond_id
                        .as_ref()
                        .is_some_and(|id| bond_ids.contains(id))
                        || crossing.unresolved_bond_source_id.is_some())
                        && (crossing
                            .inner_atom_id
                            .as_ref()
                            .is_some_and(|id| node_ids.contains(id))
                            || crossing.unresolved_inner_atom_source_id.is_some())
                });
                changed |= crossing_before != attachment.crossing_bonds.len();
                true
            });
            changed |= attachment_before != group.attachments.len();
        }
        let alternative_before = logical.alternative_groups.len();
        logical.alternative_groups.retain(|group| {
            !group.member_entity_ids.is_empty()
                || !group.unresolved_member_source_ids.is_empty()
                || !group.attachment_node_ids.is_empty()
        });
        changed |= alternative_before != logical.alternative_groups.len();
        let bracket_before = logical.bracketed_groups.len();
        logical.bracketed_groups.retain(|group| {
            !group.attachments.is_empty()
                && (!group.bracketed_entity_ids.is_empty()
                    || !group.unresolved_bracketed_source_ids.is_empty())
        });
        changed |= bracket_before != logical.bracketed_groups.len();
        let logical_ids = logical
            .all_ids()
            .into_iter()
            .map(ToString::to_string)
            .collect::<BTreeSet<_>>();
        for group in &mut logical.alternative_groups {
            if group.superseded_by_id.as_ref().is_some_and(|id| {
                !entity_exists(id) && !logical_ids.contains(id)
            }) {
                group.superseded_by_id = None;
                changed = true;
            }
        }
        let bracket_group_ids = logical
            .bracketed_groups
            .iter()
            .map(|group| group.id.clone())
            .collect::<BTreeSet<_>>();
        for group in &mut logical.bracketed_groups {
            let before = group.nested_group_ids.len();
            group
                .nested_group_ids
                .retain(|id| bracket_group_ids.contains(id));
            changed |= before != group.nested_group_ids.len();
        }
        for sequence in &mut logical.sequences {
            changed |= retain_existing(&mut sequence.text_object_ids, &|id| scene_ids.contains(id));
        }
        for cross_reference in &mut logical.cross_references {
            changed |= retain_existing(&mut cross_reference.text_object_ids, &|id| {
                scene_ids.contains(id)
            });
        }
        let tag_before = logical.object_tags.len();
        logical.object_tags.retain_mut(|tag| {
            if tag
                .owner_entity_id
                .as_ref()
                .is_some_and(|id| !entity_exists(id))
            {
                return false;
            }
            changed |= retain_existing(&mut tag.display_object_ids, &|id| scene_ids.contains(id));
            true
        });
        changed |= tag_before != logical.object_tags.len();
        let annotation_before = logical.annotations.len();
        logical.annotations.retain(|annotation| {
            !annotation
                .owner_entity_id
                .as_ref()
                .is_some_and(|id| !entity_exists(id))
        });
        changed |= annotation_before != logical.annotations.len();
        let registration_before = logical.registry_numbers.len();
        logical.registry_numbers.retain(|registration| {
            !registration
                .owner_entity_id
                .as_ref()
                .is_some_and(|id| !entity_exists(id))
        });
        changed |= registration_before != logical.registry_numbers.len();
        let local_sequences = logical
            .sequences
            .iter()
            .map(|sequence| sequence.identifier.clone())
            .collect::<BTreeSet<_>>();
        let cross_reference_before = logical.cross_references.len();
        logical.cross_references.retain(|cross_reference| {
            cross_reference.container.is_some()
                || cross_reference.document.is_some()
                || local_sequences.contains(&cross_reference.sequence_identifier)
        });
        changed |= cross_reference_before != logical.cross_references.len();
        let before = logical.representations.len();
        logical.representations.retain(|representation| {
            representation
                .owner_entity_id
                .as_ref()
                .is_none_or(&entity_exists)
                && representation
                    .target_entity_id
                    .as_ref()
                    .is_none_or(&entity_exists)
        });
        changed |= before != logical.representations.len();
        changed
    }

    fn reconcile_automatic_reaction_steps(&mut self) -> bool {
        // Document mutations happen before the commit revision advances. Drop
        // any earlier query cache so automatic relation solving sees the
        // current geometry, then let the spatial index prune candidates.
        *self.spatial_index.borrow_mut() = None;
        let axes = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| object.link_policy == LinkPolicy::Auto && is_reaction_arrow(object))
            .filter_map(reaction_arrow_axis)
            .collect::<Vec<_>>();

        let mut previous_ids = BTreeMap::new();
        let mut changed = false;
        for scheme in &mut self.state.document.reaction_schemes {
            let before = scheme.steps.len();
            scheme.steps.retain(|step| {
                if step.binding_origin != LogicalBindingOrigin::Inferred {
                    return true;
                }
                if let Some(arrow_id) = step.arrow_object_ids.first() {
                    previous_ids.insert(arrow_id.clone(), step.id.clone());
                }
                false
            });
            changed |= before != scheme.steps.len();
        }
        self.state
            .document
            .reaction_schemes
            .retain(|scheme| !scheme.steps.is_empty() || scheme.id != REACTION_AUTO_SCHEME_ID);

        let explicitly_bound_arrows = self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
            .flat_map(|step| step.arrow_object_ids.iter())
            .cloned()
            .collect::<BTreeSet<_>>();
        let axes = axes
            .into_iter()
            .filter(|axis| !explicitly_bound_arrows.contains(&axis.object_id))
            .collect::<Vec<_>>();
        let inferred = self.infer_reaction_steps(&axes);
        if inferred.is_empty() {
            return changed;
        }

        let mut steps = Vec::new();
        for candidate in inferred {
            let id = previous_ids
                .remove(&candidate.arrow_id)
                .unwrap_or_else(|| self.next_id("reaction_step_auto"));
            steps.push(crate::ReactionStepData {
                id,
                link_policy: LinkPolicy::Auto,
                binding_origin: LogicalBindingOrigin::Inferred,
                reactant_entity_ids: sorted_ids(candidate.reactants),
                product_entity_ids: sorted_ids(candidate.products),
                arrow_object_ids: vec![candidate.arrow_id],
                plus_object_ids: sorted_ids(candidate.pluses),
                objects_above_arrow: sorted_ids(candidate.above),
                objects_below_arrow: sorted_ids(candidate.below),
                atom_mappings: Vec::new(),
                interpretation_state: ReactionInterpretationState::Current,
            });
        }
        if let Some(scheme) = self
            .state
            .document
            .reaction_schemes
            .iter_mut()
            .find(|scheme| scheme.id == REACTION_AUTO_SCHEME_ID)
        {
            scheme.steps = steps;
        } else {
            self.state
                .document
                .reaction_schemes
                .push(crate::ReactionSchemeData {
                    id: REACTION_AUTO_SCHEME_ID.to_string(),
                    steps,
                });
        }
        true
    }

    fn infer_reaction_steps(&self, axes: &[ReactionArrowAxis]) -> Vec<InferredReactionStep> {
        let mut steps = axes
            .iter()
            .map(|axis| {
                (
                    axis.object_id.clone(),
                    InferredReactionStep {
                        arrow_id: axis.object_id.clone(),
                        reactants: Vec::new(),
                        products: Vec::new(),
                        pluses: Vec::new(),
                        above: Vec::new(),
                        below: Vec::new(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        let scene_objects = self.state.document.scene_objects();
        // A document may contain a very large molecule resource but only a
        // handful of page entities. Building the render-derived spatial index
        // in that case walks every atom and bond merely to filter a few scene
        // objects. Direct scene scanning is both exact and substantially
        // cheaper below this bounded threshold.
        let candidate_ids = if scene_objects.len() <= 256 {
            scene_objects
                .iter()
                .map(|object| object.id.clone())
                .collect::<BTreeSet<_>>()
        } else {
            axes.iter()
                .flat_map(|axis| {
                    let axial = (axis.length * 4.0).max(crate::DEFAULT_BOND_LENGTH * 8.0);
                    let perpendicular =
                        (axis.length * 1.5).max(crate::DEFAULT_BOND_LENGTH * 3.0);
                    self.spatial_query([
                        axis.start.x.min(axis.end.x) - axial,
                        axis.start.y.min(axis.end.y) - perpendicular,
                        axis.start.x.max(axis.end.x) + axial,
                        axis.start.y.max(axis.end.y) + perpendicular,
                    ])
                    .entity_ids
                })
                .collect::<BTreeSet<_>>()
        };

        for object in scene_objects
            .into_iter()
            .filter(|object| candidate_ids.contains(&object.id))
        {
            if object.link_policy != LinkPolicy::Auto {
                continue;
            }
            let Some(center) = scene_object_center(&self.state.document, object) else {
                continue;
            };
            if object.object_type == "molecule" {
                if let Some(candidate) = unique_reaction_side_candidate(center, axes) {
                    let target = steps
                        .get_mut(&candidate.arrow_id)
                        .expect("axis and inferred step must agree");
                    match candidate.side {
                        ReactionSide::Reactant => target
                            .reactants
                            .push((object.id.clone(), candidate.projection)),
                        ReactionSide::Product => target
                            .products
                            .push((object.id.clone(), candidate.projection)),
                    }
                }
            } else if is_plus_symbol(object) {
                if let Some(candidate) = unique_reaction_side_candidate(center, axes) {
                    steps
                        .get_mut(&candidate.arrow_id)
                        .expect("axis and inferred step must agree")
                        .pluses
                        .push((object.id.clone(), candidate.projection));
                }
            } else if object.object_type == "text" {
                if let Some((arrow_id, projection, perpendicular)) =
                    unique_reaction_condition_candidate(center, axes)
                {
                    let target = steps
                        .get_mut(&arrow_id)
                        .expect("axis and inferred step must agree");
                    if perpendicular < 0.0 {
                        target.above.push((object.id.clone(), projection));
                    } else {
                        target.below.push((object.id.clone(), projection));
                    }
                }
            }
        }

        steps
            .into_values()
            .filter(|step| !step.reactants.is_empty() && !step.products.is_empty())
            .collect()
    }

    pub(super) fn selection_can_link_reaction(&self) -> bool {
        self.selected_reaction_candidate().is_some()
    }

    pub(super) fn reaction_relations_contain_any(&self, ids: &BTreeSet<String>) -> bool {
        self.state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
            .any(|step| reaction_step_entity_ids(step).any(|entity_id| ids.contains(entity_id)))
    }

    pub(super) fn detach_reaction_relations_for_entities(
        &mut self,
        ids: &BTreeSet<String>,
    ) -> bool {
        let mut changed = false;
        for scheme in &mut self.state.document.reaction_schemes {
            let before = scheme.steps.len();
            scheme.steps.retain(|step| {
                !reaction_step_entity_ids(step).any(|entity_id| ids.contains(entity_id))
            });
            changed |= before != scheme.steps.len();
        }
        let before = self.state.document.reaction_schemes.len();
        self.state
            .document
            .reaction_schemes
            .retain(|scheme| !scheme.steps.is_empty());
        changed | (before != self.state.document.reaction_schemes.len())
    }

    pub(super) fn link_reaction_selection_untracked(&mut self) -> bool {
        let Some(candidate) = self.selected_reaction_candidate() else {
            return false;
        };
        let selected_ids = reaction_candidate_entity_ids(&candidate);
        self.detach_reaction_relations_for_entities(&selected_ids);
        for id in &selected_ids {
            if let Some(object) = self.state.document.find_scene_object_mut(id) {
                object.link_policy = LinkPolicy::Linked;
            }
        }
        let step = crate::ReactionStepData {
            id: self.next_id("reaction_step"),
            link_policy: LinkPolicy::Linked,
            binding_origin: LogicalBindingOrigin::Authored,
            reactant_entity_ids: sorted_ids(candidate.reactants),
            product_entity_ids: sorted_ids(candidate.products),
            arrow_object_ids: vec![candidate.arrow_id],
            plus_object_ids: sorted_ids(candidate.pluses),
            objects_above_arrow: sorted_ids(candidate.above),
            objects_below_arrow: sorted_ids(candidate.below),
            atom_mappings: Vec::new(),
            interpretation_state: ReactionInterpretationState::Current,
        };
        if let Some(scheme) = self
            .state
            .document
            .reaction_schemes
            .iter_mut()
            .find(|scheme| scheme.id != REACTION_AUTO_SCHEME_ID)
        {
            scheme.steps.push(step);
        } else {
            let scheme_id = self.next_id("reaction_scheme");
            self.state
                .document
                .reaction_schemes
                .push(crate::ReactionSchemeData {
                    id: scheme_id,
                    steps: vec![step],
                });
        }
        true
    }

    fn selected_reaction_candidate(&self) -> Option<InferredReactionStep> {
        let selected = &self.state.selection;
        if selected.molecule_objects.len() < 2
            || !selected.nodes.is_empty()
            || !selected.bonds.is_empty()
            || !selected.label_nodes.is_empty()
        {
            return None;
        }
        let axes = selected
            .arrow_objects
            .iter()
            .filter_map(|id| self.state.document.find_scene_object(id))
            .filter(|object| is_reaction_arrow(object))
            .filter_map(reaction_arrow_axis)
            .collect::<Vec<_>>();
        let [axis] = axes.as_slice() else {
            return None;
        };
        let selected_arrow_ids = selected
            .arrow_objects
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut candidate = InferredReactionStep {
            arrow_id: axis.object_id.clone(),
            reactants: Vec::new(),
            products: Vec::new(),
            pluses: Vec::new(),
            above: Vec::new(),
            below: Vec::new(),
        };
        for id in &selected.molecule_objects {
            let object = self.state.document.find_scene_object(id)?;
            let center = scene_object_center(&self.state.document, object)?;
            let side = unique_reaction_side_candidate(center, std::slice::from_ref(axis))?;
            match side.side {
                ReactionSide::Reactant => candidate.reactants.push((id.clone(), side.projection)),
                ReactionSide::Product => candidate.products.push((id.clone(), side.projection)),
            }
        }
        for id in selected_arrow_ids
            .iter()
            .filter(|id| id.as_str() != axis.object_id)
        {
            let object = self.state.document.find_scene_object(id)?;
            if !is_plus_symbol(object) {
                return None;
            }
            let center = scene_object_center(&self.state.document, object)?;
            let side = unique_reaction_side_candidate(center, std::slice::from_ref(axis))?;
            candidate.pluses.push((id.clone(), side.projection));
        }
        for id in &selected.text_objects {
            let object = self.state.document.find_scene_object(id)?;
            let center = scene_object_center(&self.state.document, object)?;
            let (_, projection, perpendicular) =
                unique_reaction_condition_candidate(center, std::slice::from_ref(axis))?;
            if perpendicular < 0.0 {
                candidate.above.push((id.clone(), projection));
            } else {
                candidate.below.push((id.clone(), projection));
            }
        }
        (!candidate.reactants.is_empty() && !candidate.products.is_empty()).then_some(candidate)
    }
}
