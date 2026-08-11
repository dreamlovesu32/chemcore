impl Engine {
    pub fn logical_objects_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.state.document.logical_objects)
            .map_err(|error| error.to_string())
    }

    pub fn logical_objects_dialog_json(&self) -> Result<String, String> {
        let selected_entity_ids = if self.state.selection.ordered_entities.is_empty() {
            let mut seen = BTreeSet::new();
            self.state
                .selection
                .molecule_objects
                .iter()
                .chain(self.state.selection.arrow_objects.iter())
                .chain(self.state.selection.text_objects.iter())
                .chain(self.state.selection.label_nodes.iter())
                .chain(self.state.selection.nodes.iter())
                .chain(self.state.selection.bonds.iter())
                .filter(|id| seen.insert((*id).clone()))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            self.state.selection.ordered_entities.clone()
        };
        let logical = &self.state.document.logical_objects;
        let selected_owner = selected_entity_ids.first().cloned();
        let selected_target = selected_entity_ids.get(1).cloned();
        let reaction_steps = self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| {
                scheme.steps.iter().map(|step| {
                    let mut value = serde_json::to_value(step).expect("reaction step serializes");
                    value
                        .as_object_mut()
                        .expect("reaction step serializes as object")
                        .insert("schemeId".to_string(), Value::String(scheme.id.clone()));
                    value
                })
            })
            .collect::<Vec<_>>();
        let default_scheme_id = self
            .state
            .document
            .reaction_schemes
            .first()
            .map(|scheme| scheme.id.clone())
            .unwrap_or_default();
        serde_json::to_string(&serde_json::json!({
            "kind": "logical-objects",
            "title": "Logical Objects",
            "selectedEntityIds": selected_entity_ids,
            "families": [
                logical_family("reaction-scheme", "Reaction Schemes", &self.state.document.reaction_schemes, reaction_scheme_fields(), serde_json::json!({"id":"","steps":[]})),
                logical_value_family("reaction-step", "Reaction Steps", reaction_steps, reaction_step_fields(), serde_json::json!({"id":"","schemeId":default_scheme_id,"linkPolicy":"linked","bindingOrigin":"authored","reactantEntityIds":[],"productEntityIds":[],"arrowObjectIds":[],"plusObjectIds":[],"objectsAboveArrow":[],"objectsBelowArrow":[],"atomMappings":[],"interpretationState":"current"})),
                logical_family("alternative-group", "Alternative Groups", &logical.alternative_groups, alternative_group_fields(), serde_json::json!({"id":"","memberEntityIds":selected_entity_ids,"attachmentNodeIds":[],"visible":true,"ignoreWarnings":false,"bindingOrigin":"authored"})),
                logical_family("bracketed-group", "Bracketed Groups", &logical.bracketed_groups, bracketed_group_fields(), serde_json::json!({"id":"","bracketObjectIds":[],"bracketedEntityIds":selected_entity_ids,"nestedGroupIds":[],"attachments":[],"usage":"unspecified","polymerRepeatPattern":"either-unknown","polymerFlipType":"unspecified","bindingOrigin":"authored"})),
                logical_family("sequence", "Sequences", &logical.sequences, sequence_fields(), serde_json::json!({"id":"","identifier":"","textObjectIds":selected_entity_ids,"bindingOrigin":"authored"})),
                logical_family("cross-reference", "Cross References", &logical.cross_references, cross_reference_fields(), serde_json::json!({"id":"","identifier":"","sequenceIdentifier":"","textObjectIds":selected_entity_ids,"bindingOrigin":"authored"})),
                logical_family("object-tag", "Object Tags", &logical.object_tags, object_tag_fields(), serde_json::json!({"id":"","ownerEntityId":selected_owner,"name":"","tagType":"unknown","positioningType":"auto","persistent":true,"tracking":true,"visible":true,"displayObjectIds":[],"bindingOrigin":"authored"})),
                logical_family("annotation", "Metadata Annotations", &logical.annotations, annotation_fields(), serde_json::json!({"id":"","ownerEntityId":selected_owner,"bindingOrigin":"authored"})),
                logical_family("registry-number", "Registry Numbers", &logical.registry_numbers, registry_number_fields(), serde_json::json!({"id":"","ownerEntityId":selected_owner,"authority":"","number":"","bindingOrigin":"authored"})),
                logical_family("representation", "Representations", &logical.representations, representation_fields(), serde_json::json!({"id":"","ownerEntityId":selected_owner,"targetEntityId":selected_target,"attribute":"","bindingOrigin":"authored"})),
            ]
        }))
        .map_err(|error| error.to_string())
    }

    pub fn set_logical_object_json(
        &mut self,
        kind: &str,
        value_json: &str,
    ) -> Result<bool, String> {
        let value = serde_json::from_str(value_json).map_err(|error| error.to_string())?;
        self.set_logical_object_value(kind, value)
    }

    pub fn set_logical_object_value(&mut self, kind: &str, value: Value) -> Result<bool, String> {
        if matches!(kind, "reaction-scheme" | "reaction-step") {
            return self.set_reaction_logical_value(kind, value);
        }
        let mut logical = self.state.document.logical_objects.clone();
        let canonical_value = upsert_logical_value(self, &mut logical, kind, value)?;
        self.validate_logical_candidate(&logical)?;
        if logical == self.state.document.logical_objects {
            return Ok(false);
        }
        let command = super::EditorCommand::SetLogicalObject {
            kind: kind.to_string(),
            value: canonical_value,
        };
        Ok(self.with_command(command, |engine| {
            engine.push_undo_snapshot();
            engine.state.document.logical_objects = logical;
            true
        }))
    }

    pub fn delete_logical_object(&mut self, kind: &str, id: &str) -> Result<bool, String> {
        if matches!(kind, "reaction-scheme" | "reaction-step") {
            return self.delete_reaction_logical_value(kind, id);
        }
        let mut logical = self.state.document.logical_objects.clone();
        if !delete_logical_value(&mut logical, kind, id)? {
            return Ok(false);
        }
        self.validate_logical_candidate(&logical)?;
        Ok(self.with_command(
            super::EditorCommand::DeleteLogicalObject {
                kind: kind.to_string(),
                id: id.to_string(),
            },
            |engine| {
                engine.push_undo_snapshot();
                engine.state.document.logical_objects = logical;
                true
            },
        ))
    }

    pub fn reorder_logical_object(
        &mut self,
        kind: &str,
        id: &str,
        index: usize,
    ) -> Result<bool, String> {
        if matches!(kind, "reaction-scheme" | "reaction-step") {
            return self.reorder_reaction_logical_value(kind, id, index);
        }
        let mut logical = self.state.document.logical_objects.clone();
        if !reorder_logical_value(&mut logical, kind, id, index)? {
            return Ok(false);
        }
        self.validate_logical_candidate(&logical)?;
        Ok(self.with_command(
            super::EditorCommand::ReorderLogicalObject {
                kind: kind.to_string(),
                id: id.to_string(),
                index,
            },
            |engine| {
                engine.push_undo_snapshot();
                engine.state.document.logical_objects = logical;
                true
            },
        ))
    }

    pub(super) fn execute_logical_object_command(
        &mut self,
        command: super::EditorCommand,
    ) -> Result<bool, String> {
        match command {
            super::EditorCommand::SetLogicalObject { kind, value } => {
                self.set_logical_object_value(&kind, value)
            }
            super::EditorCommand::DeleteLogicalObject { kind, id } => {
                self.delete_logical_object(&kind, &id)
            }
            super::EditorCommand::ReorderLogicalObject { kind, id, index } => {
                self.reorder_logical_object(&kind, &id, index)
            }
            _ => Err("command is not a logical-object command".to_string()),
        }
    }

    fn validate_logical_candidate(&self, logical: &crate::LogicalObjectData) -> Result<(), String> {
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
        logical.validate(&scene_ids, &node_ids, &bond_ids)?;
        let reaction_ids = self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| {
                std::iter::once(scheme.id.as_str())
                    .chain(scheme.steps.iter().map(|step| step.id.as_str()))
            })
            .collect::<BTreeSet<_>>();
        if let Some(id) = logical
            .all_ids()
            .into_iter()
            .find(|id| reaction_ids.contains(id))
        {
            return Err(format!(
                "logical object id '{id}' collides with a reaction object"
            ));
        }
        Ok(())
    }

    fn validate_document_candidate(
        &self,
        document: &crate::ChemSemaDocument,
    ) -> Result<(), String> {
        let json = serde_json::to_string(document).map_err(|error| error.to_string())?;
        crate::parse_document_json(&json).map(|_| ())
    }

    fn set_reaction_logical_value(&mut self, kind: &str, mut value: Value) -> Result<bool, String> {
        let mut document = self.state.document.clone();
        let canonical = match kind {
            "reaction-scheme" => {
                let mut scheme: crate::ReactionSchemeData =
                    serde_json::from_value(value).map_err(|error| error.to_string())?;
                if scheme.id.trim().is_empty() {
                    scheme.id = self.next_id("reaction_scheme");
                }
                if let Some(existing) = document
                    .reaction_schemes
                    .iter_mut()
                    .find(|existing| existing.id == scheme.id)
                {
                    *existing = scheme.clone();
                } else {
                    document.reaction_schemes.push(scheme.clone());
                }
                serde_json::to_value(scheme).map_err(|error| error.to_string())?
            }
            "reaction-step" => {
                let scheme_id = value
                    .get("schemeId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let scheme_id = if scheme_id.is_empty() {
                    self.next_id("reaction_scheme")
                } else {
                    scheme_id
                };
                value
                    .as_object_mut()
                    .ok_or_else(|| "reaction step must be an object".to_string())?
                    .remove("schemeId");
                let mut step: crate::ReactionStepData =
                    serde_json::from_value(value).map_err(|error| error.to_string())?;
                if step.id.trim().is_empty() {
                    step.id = self.next_id("reaction_step");
                }
                let previous_position = document.reaction_schemes.iter().find_map(|scheme| {
                    scheme
                        .steps
                        .iter()
                        .position(|existing| existing.id == step.id)
                        .map(|index| (scheme.id.clone(), index))
                });
                for scheme in &mut document.reaction_schemes {
                    scheme.steps.retain(|existing| existing.id != step.id);
                }
                if let Some(scheme) = document
                    .reaction_schemes
                    .iter_mut()
                    .find(|scheme| scheme.id == scheme_id)
                {
                    let index = previous_position
                        .filter(|(previous_scheme_id, _)| previous_scheme_id == &scheme_id)
                        .map(|(_, index)| index.min(scheme.steps.len()))
                        .unwrap_or(scheme.steps.len());
                    scheme.steps.insert(index, step.clone());
                } else {
                    document.reaction_schemes.push(crate::ReactionSchemeData {
                        id: scheme_id.clone(),
                        steps: vec![step.clone()],
                    });
                }
                let mut canonical =
                    serde_json::to_value(step).map_err(|error| error.to_string())?;
                canonical
                    .as_object_mut()
                    .expect("reaction step serializes as object")
                    .insert("schemeId".to_string(), Value::String(scheme_id));
                canonical
            }
            _ => return Err(format!("unsupported logical object kind '{kind}'")),
        };
        self.validate_document_candidate(&document)?;
        if document == self.state.document {
            return Ok(false);
        }
        Ok(self.with_command(
            super::EditorCommand::SetLogicalObject {
                kind: kind.to_string(),
                value: canonical,
            },
            |engine| {
                engine.push_undo_snapshot();
                engine.state.document = document;
                true
            },
        ))
    }

    fn delete_reaction_logical_value(&mut self, kind: &str, id: &str) -> Result<bool, String> {
        let mut document = self.state.document.clone();
        let removed_step_ids = match kind {
            "reaction-scheme" => {
                let removed = document
                    .reaction_schemes
                    .iter()
                    .find(|scheme| scheme.id == id)
                    .map(|scheme| {
                        scheme
                            .steps
                            .iter()
                            .map(|step| step.id.clone())
                            .collect::<BTreeSet<_>>()
                    })
                    .unwrap_or_default();
                document.reaction_schemes.retain(|scheme| scheme.id != id);
                removed
            }
            "reaction-step" => {
                let mut removed = BTreeSet::new();
                for scheme in &mut document.reaction_schemes {
                    if scheme.steps.iter().any(|step| step.id == id) {
                        removed.insert(id.to_string());
                    }
                    scheme.steps.retain(|step| step.id != id);
                }
                removed
            }
            _ => return Err(format!("unsupported logical object kind '{kind}'")),
        };
        if removed_step_ids.is_empty()
            && document.reaction_schemes == self.state.document.reaction_schemes
        {
            return Ok(false);
        }
        detach_stoichiometry_grids_for_removed_steps(&mut document.objects, &removed_step_ids);
        self.validate_document_candidate(&document)?;
        Ok(self.with_command(
            super::EditorCommand::DeleteLogicalObject {
                kind: kind.to_string(),
                id: id.to_string(),
            },
            |engine| {
                engine.push_undo_snapshot();
                engine.state.document = document;
                true
            },
        ))
    }

    fn reorder_reaction_logical_value(
        &mut self,
        kind: &str,
        id: &str,
        index: usize,
    ) -> Result<bool, String> {
        let mut document = self.state.document.clone();
        let changed = match kind {
            "reaction-scheme" => {
                reorder_by_id(&mut document.reaction_schemes, id, index, |scheme| {
                    &scheme.id
                })
            }
            "reaction-step" => document
                .reaction_schemes
                .iter_mut()
                .find(|scheme| scheme.steps.iter().any(|step| step.id == id))
                .is_some_and(|scheme| reorder_by_id(&mut scheme.steps, id, index, |step| &step.id)),
            _ => return Err(format!("unsupported logical object kind '{kind}'")),
        };
        if !changed {
            return Ok(false);
        }
        self.validate_document_candidate(&document)?;
        Ok(self.with_command(
            super::EditorCommand::ReorderLogicalObject {
                kind: kind.to_string(),
                id: id.to_string(),
                index,
            },
            |engine| {
                engine.push_undo_snapshot();
                engine.state.document = document;
                true
            },
        ))
    }

}
