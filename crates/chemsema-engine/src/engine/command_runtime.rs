use super::*;

impl Engine {
    pub fn execute_command_json(&mut self, command_json: &str) -> Result<String, String> {
        let command: EditorCommand =
            serde_json::from_str(command_json).map_err(|error| error.to_string())?;
        let result = self.execute_command(command)?;
        serde_json::to_string(&result).map_err(|error| error.to_string())
    }

    pub(super) fn execute_immediate_command(
        &mut self,
        command: EditorCommand,
    ) -> Result<CommandResult, String> {
        match command.clone() {
            EditorCommand::LoadDocument {
                format,
                content,
                bytes,
            } => self.execute_load_document_command(command, format, &content, &bytes),
            EditorCommand::ExportDocument { format } => {
                self.execute_export_document_command(command, format)
            }
            EditorCommand::ConvertDocument {
                from,
                to,
                content,
                bytes,
            } => self.execute_convert_document_command(command, from, to, &content, &bytes),
            EditorCommand::InspectDocument { include } => {
                Ok(self
                    .readonly_command_result(Some(command), self.inspect_document_output(&include)))
            }
            EditorCommand::ChemicalAnalysis { format, targets } => {
                let output = self.chemical_analysis_output(format, &targets)?;
                Ok(self.readonly_command_result(Some(command), output))
            }
            EditorCommand::SelectTargets { targets } => {
                let changed = self.select_targets_direct(&targets);
                Ok(self
                    .readonly_command_result(Some(command), self.selection_command_output(changed)))
            }
            EditorCommand::SelectAll => {
                let changed = self.select_all();
                Ok(self
                    .readonly_command_result(Some(command), self.selection_command_output(changed)))
            }
            EditorCommand::ClearSelection => {
                let changed = self.clear_selection();
                Ok(self
                    .readonly_command_result(Some(command), self.selection_command_output(changed)))
            }
            EditorCommand::PlanBond {
                begin,
                cursor,
                angle,
                bond_length,
                order,
                variant,
            } => {
                let output = self.plan_bond_command_output(
                    begin,
                    cursor,
                    angle,
                    bond_length,
                    order,
                    variant,
                );
                Ok(self.readonly_command_result(Some(command), output))
            }
            EditorCommand::PlanTemplate {
                template,
                x,
                y,
                anchor,
                bond_id,
                cursor,
                angle,
                bond_length,
                side,
            } => {
                let output = self.plan_template_command_output(
                    template,
                    x,
                    y,
                    anchor,
                    bond_id,
                    cursor,
                    angle,
                    bond_length,
                    side,
                )?;
                Ok(self.readonly_command_result(Some(command), output))
            }
            _ => unreachable!("immediate commands are classified before dispatch"),
        }
    }

    pub(super) fn execute_creation_command(
        &mut self,
        command: EditorCommand,
    ) -> Result<bool, String> {
        let changed = match command.clone() {
            EditorCommand::InsertSmiles { smiles, x, y } => {
                let molecule =
                    chemsema_chemistry::parse_smiles(&smiles).map_err(|error| error.to_string())?;
                self.with_command(command, |engine| {
                    engine.insert_smiles_untracked(&molecule, &smiles, Point::new(x, y))
                })
            }
            EditorCommand::AddBond {
                begin,
                end,
                order,
                variant,
                wide_end,
                double_placement,
                double,
                line_weights,
                stroke,
                endpoint_attachments,
            } => {
                let previous_tool = self.state.tool.clone();
                self.state.tool.bond_variant = variant;
                let changed = self.add_bond_between_with_style_override(
                    bond_anchor_from_command(begin),
                    bond_anchor_from_command(end),
                    order,
                    wide_end,
                    command_double_bond_override(double_placement, double),
                    line_weights,
                    stroke,
                    endpoint_attachments,
                );
                self.state.tool = previous_tool;
                changed
            }
            EditorCommand::AddArrow {
                begin,
                end,
                variant,
                head_size,
                curve,
                head_style,
                tail_style,
                head,
                tail,
                bold,
                no_go,
            } => {
                let previous_tool = self.state.tool.clone();
                self.state.tool.arrow_variant = variant;
                self.state.tool.arrow_head_size = head_size;
                self.state.tool.arrow_curve = curve;
                self.state.tool.arrow_head_style = head_style;
                self.state.tool.arrow_tail_style = tail_style;
                self.state.tool.arrow_head = head;
                self.state.tool.arrow_tail = tail;
                self.state.tool.arrow_bold = bold;
                self.state.tool.arrow_no_go = no_go;
                let changed = self
                    .add_arrow_between(point_from_command(&begin), point_from_command(&end))
                    .is_some();
                self.state.tool = previous_tool;
                changed
            }
            EditorCommand::AddShape {
                kind,
                style,
                color,
                begin,
                end,
            } => self.with_command(command, |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.shape_kind = kind;
                engine.state.tool.shape_style = style;
                engine.state.tool.shape_color = color;
                let start = point_from_command(&begin);
                let current = point_from_command(&end);
                let drag = ShapeDragState {
                    pointer_start: start,
                    start,
                    current,
                    anchor: ShapeDrawAnchor {
                        kind: ShapeDrawAnchorKind::Free,
                        point: start,
                        bounds: None,
                    },
                    has_dragged: start.distance(current) > crate::EPSILON,
                };
                let changed = engine.insert_shape_from_drag(&drag);
                engine.state.tool = previous_tool;
                changed
            }),
            EditorCommand::AddBracket { kind, begin, end } => {
                self.with_command(command, |engine| {
                    let previous_tool = engine.state.tool.clone();
                    engine.state.tool.bracket_kind = kind;
                    let drag = BracketDragState {
                        start: point_from_command(&begin),
                        current: point_from_command(&end),
                        symbol_anchor: None,
                        has_dragged: true,
                    };
                    let changed = engine.insert_bracket_from_drag(&drag);
                    engine.state.tool = previous_tool;
                    changed
                })
            }
            EditorCommand::AddSymbol { kind, center } => self.with_command(command, |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.symbol_kind = kind;
                let changed = engine.insert_bracket_symbol(point_from_command(&center));
                engine.state.tool = previous_tool;
                changed
            }),
            EditorCommand::AddElement {
                symbol,
                atomic_number,
                center,
            } => self.with_command(command, |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.element_symbol = symbol;
                engine.state.tool.element_atomic_number = atomic_number;
                let changed = engine.insert_periodic_element(point_from_command(&center));
                engine.state.tool = previous_tool;
                changed
            }),
            EditorCommand::AddText { position, content } => {
                self.with_command(command, |engine| engine.add_text_direct(position, content))
            }
            EditorCommand::AddImage {
                mime_type,
                data_base64,
                pixel_width,
                pixel_height,
                position,
                width,
                height,
                source_name,
            } => self.with_command(command, |engine| {
                engine.add_image_direct(
                    &mime_type,
                    &data_base64,
                    pixel_width,
                    pixel_height,
                    position,
                    width,
                    height,
                    source_name.as_deref(),
                )
            }),
            EditorCommand::AddOrbital {
                template,
                style,
                phase,
                color,
                center,
                end,
            } => self.with_command(command, |engine| {
                let previous_tool = engine.state.tool.clone();
                engine.state.tool.orbital_template = template;
                engine.state.tool.orbital_style = style;
                engine.state.tool.orbital_phase = phase;
                engine.state.tool.orbital_color = color;
                let drag = OrbitalDragState {
                    anchor: point_from_command(&center),
                    current: point_from_command(&end),
                    has_dragged: true,
                };
                let changed = engine.insert_orbital_from_drag(&drag);
                engine.state.tool = previous_tool;
                changed
            }),
            _ => unreachable!("creation commands are classified before dispatch"),
        };
        Ok(changed)
    }

    pub(super) fn completed_command_result(&mut self, changed: bool) -> CommandResult {
        if !changed && self.last_command_result.is_none() {
            self.last_command_result = Some(self.unchanged_command_result());
        }
        self.last_command_result
            .clone()
            .unwrap_or_else(|| self.unchanged_command_result())
    }

    fn execute_selection_semantic_command(&mut self, command: EditorCommand) -> bool {
        match command {
            EditorCommand::SetInterpretChemicallyForSelection { enabled } => {
                self.set_interpret_chemically_for_selection(enabled)
            }
            EditorCommand::SetImplicitHydrogenCountForSelection { count } => {
                self.set_implicit_hydrogen_count_for_selection(count)
            }
            EditorCommand::SetAtomPropertyForSelection { property, value } => {
                self.set_atom_property_for_selection(&property, value.as_deref())
            }
            EditorCommand::SetBondPropertyForSelection { property, value } => {
                self.set_bond_property_for_selection(&property, value.as_deref())
            }
            EditorCommand::SetChemicalCheckForSelection { enabled } => {
                self.set_chemical_check_for_selection(enabled)
            }
            _ => unreachable!("selection semantic commands are classified before dispatch"),
        }
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> Result<CommandResult, String> {
        self.last_command_result = None;
        if editor_command_is_immediate(&command) {
            return self.execute_immediate_command(command);
        }
        if editor_command_is_creation(&command) {
            let changed = self.execute_creation_command(command)?;
            return Ok(self.completed_command_result(changed));
        }
        if editor_command_is_selection_semantic(&command) {
            let changed = self.execute_selection_semantic_command(command);
            return Ok(self.completed_command_result(changed));
        }
        let changed = match command.clone() {
            EditorCommand::LoadDocument { .. }
            | EditorCommand::ExportDocument { .. }
            | EditorCommand::ConvertDocument { .. }
            | EditorCommand::InspectDocument { .. }
            | EditorCommand::ChemicalAnalysis { .. }
            | EditorCommand::SelectTargets { .. }
            | EditorCommand::SelectAll
            | EditorCommand::ClearSelection
            | EditorCommand::PlanBond { .. }
            | EditorCommand::PlanTemplate { .. } => {
                unreachable!("immediate commands are dispatched before the main command match")
            }
            EditorCommand::InsertSmiles { .. }
            | EditorCommand::AddBond { .. }
            | EditorCommand::AddArrow { .. }
            | EditorCommand::AddShape { .. }
            | EditorCommand::AddBracket { .. }
            | EditorCommand::AddSymbol { .. }
            | EditorCommand::AddElement { .. }
            | EditorCommand::AddText { .. }
            | EditorCommand::AddImage { .. }
            | EditorCommand::AddOrbital { .. } => {
                unreachable!("creation commands are dispatched before the main command match")
            }
            EditorCommand::Undo => self.undo(),
            EditorCommand::Redo => self.redo(),
            EditorCommand::SetTextRuns { object_id, content } => self
                .with_command(command.clone(), |engine| {
                    engine.set_text_runs_direct(&object_id, content)
                }),
            EditorCommand::SetNodeLabelRuns { node_id, content } => self
                .with_command(command.clone(), |engine| {
                    engine.set_node_label_runs_direct(&node_id, content)
                }),
            EditorCommand::SetNodeCharge { node_id, charge } => self
                .with_command(command.clone(), |engine| {
                    engine.set_node_charge_direct(&node_id, charge)
                }),
            EditorCommand::ReplaceNodeLabel { node_id, label } => self
                .with_command(command.clone(), |engine| {
                    engine.replace_node_label_untracked(&node_id, &label)
                }),
            EditorCommand::MoveTlcSpot { .. }
            | EditorCommand::MoveSelection
            | EditorCommand::RotateSelection
            | EditorCommand::ResizeSelection
            | EditorCommand::EditArrowGeometry { .. }
            | EditorCommand::EditShapeGeometry { .. }
            | EditorCommand::ApplyTextEdit { .. } => {
                return Err(format!(
                    "Command '{}' requires an active interaction context.",
                    editor_command_type_name(&command)
                ));
            }
            EditorCommand::ApplyArrowStyle {
                object_ids,
                variant,
                head_size,
                curve,
                head_style,
                tail_style,
                head,
                tail,
                bold,
                no_go,
            } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_arrow_options_to_selection(
                    variant, head_size, curve, head_style, tail_style, head, tail, bold, no_go,
                )
            }
            EditorCommand::CycleBondStyle { bond_id, variant } => {
                let previous_tool = self.state.tool.clone();
                self.state.tool.bond_variant = variant;
                let changed = self.cycle_bond_center_style(&bond_id);
                self.state.tool = previous_tool;
                changed
            }
            EditorCommand::DeleteSelection => self.delete_selection(),
            EditorCommand::DeleteTargets { targets } => self
                .with_command(command.clone(), |engine| {
                    engine.delete_targets_direct(&targets)
                }),
            EditorCommand::DeleteFocusedAtPoint { x, y, source } => self.delete_focused_at_point(
                Point::new(x, y),
                match source {
                    FocusedDeleteSource::DeleteTool => FocusedDeleteMode::DeleteToolClick,
                    FocusedDeleteSource::CommandKey => FocusedDeleteMode::CommandKey,
                },
            ),
            EditorCommand::PasteClipboard => self.paste_clipboard(),
            EditorCommand::CutSelection => self.cut_selection(),
            EditorCommand::InsertTemplate {
                template,
                x,
                y,
                anchor,
                bond_id,
                cursor,
                angle,
                bond_length,
                side,
            } => self.insert_template_command(
                template,
                x,
                y,
                anchor,
                bond_id,
                cursor,
                angle,
                bond_length,
                side,
            ),
            EditorCommand::ApplySelectionArrange { command } => {
                self.apply_selection_arrange_command(&command)
            }
            EditorCommand::ApplySelectionOrder {
                object_ids,
                command,
            } => {
                if !object_ids.is_empty() {
                    self.state.selection =
                        scene_object_selection_from_ids(&self.state.document, &object_ids);
                }
                self.apply_selection_order_command(&command)
            }
            EditorCommand::ApplySelectionColor { color } => self.apply_color_to_selection(&color),
            EditorCommand::ApplyShapeStyle { object_ids, style } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_shape_style_to_selection(&style)
            }
            EditorCommand::ApplyBracketKind { object_ids, kind } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_bracket_kind_to_selection(&kind)
            }
            EditorCommand::ApplyOrbitalTemplate {
                object_ids,
                template,
            } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_orbital_template_to_selection(&template)
            }
            EditorCommand::ApplyOrbitalStyle { object_ids, style } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_orbital_style_to_selection(&style)
            }
            EditorCommand::ApplyOrbitalPhase { object_ids, phase } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_orbital_phase_to_selection(&phase)
            }
            EditorCommand::ApplyLineStyle { object_ids, style } => {
                if !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_line_style_to_selection(&style)
            }
            EditorCommand::ApplyBondStyle { bond_ids, style } => {
                let bond_ids = if bond_ids.is_empty() {
                    self.state.selection.bonds.clone()
                } else {
                    bond_ids
                };
                self.with_command(command.clone(), |engine| {
                    engine.apply_bond_style_to_bond_ids_untracked(&bond_ids, &style)
                })
            }
            EditorCommand::ApplyTextStyle {
                text_object_ids,
                label_node_ids,
                node_ids,
                command,
                value,
            } => {
                if !text_object_ids.is_empty() || !label_node_ids.is_empty() || !node_ids.is_empty()
                {
                    self.state.selection = SelectionState {
                        text_objects: text_object_ids,
                        label_nodes: label_node_ids,
                        nodes: node_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_text_style_to_selection(&command, &value)
            }
            EditorCommand::SetInterpretChemicallyForSelection { .. }
            | EditorCommand::SetImplicitHydrogenCountForSelection { .. }
            | EditorCommand::SetAtomPropertyForSelection { .. }
            | EditorCommand::SetBondPropertyForSelection { .. }
            | EditorCommand::SetChemicalCheckForSelection { .. } => {
                unreachable!("selection semantic commands are dispatched before the main match")
            }
            EditorCommand::ExpandLabelsInSelection => self.expand_labels_in_selection(),
            EditorCommand::CenterSelectionOnPage => self.center_selection_on_page(),
            EditorCommand::GroupSelection { object_ids } => {
                if !object_ids.is_empty() {
                    self.state.selection =
                        scene_object_selection_from_ids(&self.state.document, &object_ids);
                }
                self.group_selection()
            }
            EditorCommand::UngroupSelection { object_ids } => {
                if !object_ids.is_empty() {
                    self.state.selection =
                        scene_object_selection_from_ids(&self.state.document, &object_ids);
                }
                self.ungroup_selection()
            }
            EditorCommand::LinkSelection { object_ids } => {
                if !object_ids.is_empty() {
                    self.state.selection =
                        scene_object_selection_from_ids(&self.state.document, &object_ids);
                }
                self.link_selection()
            }
            EditorCommand::UnlinkSelection { object_ids } => {
                if !object_ids.is_empty() {
                    self.state.selection =
                        scene_object_selection_from_ids(&self.state.document, &object_ids);
                }
                self.unlink_selection()
            }
            EditorCommand::JoinSelection => self.join_selection(),
            EditorCommand::MoveTargets { targets, delta } => self
                .with_command(command.clone(), |engine| {
                    engine.move_targets_by_delta(&targets, delta)
                }),
            EditorCommand::RotateTargets {
                targets,
                center,
                degrees,
            } => self.with_command(command.clone(), |engine| {
                engine.rotate_targets_by_degrees(&targets, center, degrees)
            }),
            EditorCommand::ScaleTargets {
                targets,
                scale_x,
                scale_y,
                pivot,
            } => self.with_command(command.clone(), |engine| {
                engine.scale_targets_by_factors(&targets, scale_x, scale_y, pivot)
            }),
            EditorCommand::ScaleSelection { percent } => self.scale_selection(percent),
            EditorCommand::ApplyObjectSettings { settings } => self.apply_object_settings(settings),
            EditorCommand::ApplyObjectSettingsToSelection {
                bond_ids,
                object_ids,
                settings,
            } => {
                if !bond_ids.is_empty() || !object_ids.is_empty() {
                    self.state.selection = SelectionState {
                        bonds: bond_ids,
                        arrow_objects: object_ids,
                        ..SelectionState::default()
                    };
                }
                self.apply_object_settings_to_selection(SelectedObjectSettings {
                    bond_length: settings.bond_length,
                    line_width: settings.line_width,
                    bold_width: settings.bold_width,
                    bond_spacing: settings.bond_spacing,
                    margin_width: settings.margin_width,
                    hash_spacing: settings.hash_spacing,
                })
            }
            EditorCommand::ApplyDocumentStyle { preset } => self.set_document_style_preset(&preset),
            EditorCommand::SetArrowGeometry {
                object_id,
                begin,
                end,
                curve,
                head_style,
                tail_style,
            } => self.with_command(command.clone(), |engine| {
                engine.set_arrow_geometry_direct(
                    &object_id,
                    point_from_command(&begin),
                    point_from_command(&end),
                    curve,
                    head_style,
                    tail_style,
                )
            }),
            EditorCommand::SetShapeGeometry {
                object_id,
                begin,
                end,
            } => self.with_command(command.clone(), |engine| {
                engine.set_shape_geometry_direct(
                    &object_id,
                    point_from_command(&begin),
                    point_from_command(&end),
                )
            }),
            EditorCommand::ReplaceHoveredEndpointLabel { label } => {
                self.replace_hovered_endpoint_label(&label)
            }
        };
        Ok(self.completed_command_result(changed))
    }
}
