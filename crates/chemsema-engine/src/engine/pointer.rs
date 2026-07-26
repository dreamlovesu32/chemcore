use super::*;

impl Engine {
    pub fn set_tool_state(&mut self, tool: ToolState) {
        let previous_tool = self.state.tool.active_tool;
        let next_tool = tool.active_tool;
        let changed_tool = previous_tool != next_tool;
        self.state.tool = tool;
        if changed_tool && previous_tool == Tool::Select && next_tool != Tool::Select {
            self.state.selection = SelectionState::default();
        }
        self.clear_interaction();
        if changed_tool && next_tool == Tool::Select {
            self.select_pending_target_for_select_tool();
        } else if changed_tool {
            self.pending_select_target = None;
        }
    }

    pub(super) fn clear_overlay(&mut self) {
        // Overlay state is transient UI feedback. Clearing it without touching
        // selection or history prevents hover/focus from sticking after commits.
        self.state.overlay = OverlayState::default();
    }

    pub fn pointer_move(&mut self, event: PointerEvent) {
        let point = event.point();
        let endpoint_hit_radius = self.endpoint_hit_radius();
        let endpoint_focus_radius = self.endpoint_focus_radius();
        if self.state.tool.active_tool == Tool::Select {
            self.hover_select_target(point);
            return;
        }
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_move_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_move_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Orbital {
            self.pointer_move_orbital(event);
            return;
        }
        if matches!(self.state.tool.active_tool, Tool::Shape | Tool::TlcPlate) {
            self.pointer_move_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Bracket {
            self.pointer_move_bracket(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Symbol {
            self.pointer_move_symbol(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Element {
            self.pointer_move_element(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
            self.state.overlay.hover_shape = None;
            self.state.overlay.preview = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.hover_endpoint = None;
            if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: Some(object_id),
                    node_id: None,
                });
                return;
            }
            let endpoint_hit = hit_test_endpoint(&self.state.document, point, endpoint_hit_radius);
            if endpoint_hit
                .as_ref()
                .is_some_and(|endpoint| endpoint.distance <= endpoint_focus_radius)
            {
                self.state.overlay.hover_endpoint = endpoint_hit;
                return;
            }
            if let Some(center) =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
            {
                self.state.overlay.hover_bond_center = Some(center);
                return;
            }
            if let Some(endpoint) = endpoint_hit {
                self.state.overlay.hover_endpoint = Some(endpoint);
            }
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.drag = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_arrow = None;
            self.state.overlay.hover_shape = None;
            self.state.overlay.preview = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.hover_endpoint = None;
            if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: None,
                    node_id: Some(node_id),
                });
                return;
            }
            if let Some((object_id, bounds)) = self.hit_test_text_object(point) {
                self.state.overlay.hover_text_box = Some(HoverTextBox {
                    bounds,
                    object_id: Some(object_id),
                    node_id: None,
                });
                return;
            }
            if let Some(endpoint) =
                hit_test_endpoint(&self.state.document, point, endpoint_hit_radius)
            {
                if endpoint.label_anchor.is_some() {
                    if let Some(entry) = self.state.document.editable_fragment() {
                        if let Some(node) = entry
                            .fragment
                            .nodes
                            .iter()
                            .find(|node| node.id == endpoint.node_id)
                        {
                            if let Some(bounds) =
                                endpoint_label_world_bounds(node, entry.object.transform.translate)
                            {
                                self.state.overlay.hover_text_box = Some(HoverTextBox {
                                    bounds,
                                    object_id: None,
                                    node_id: Some(endpoint.node_id),
                                });
                                return;
                            }
                        }
                    }
                }
                self.state.overlay.hover_endpoint = Some(endpoint);
            }
            return;
        }
        if self.state.tool.active_tool == Tool::Table {
            self.pointer_move_table(event);
            return;
        }
        if !can_focus_endpoint(&self.state.tool) {
            self.clear_interaction();
            return;
        }

        if let Some(mut drag) = self.drag.take() {
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            if drag.has_dragged {
                self.state.overlay.hover_endpoint = None;
                let target = self.drag_target_endpoint(&drag.anchor, point);
                let end = if let Some(target) = target {
                    self.state.overlay.hover_endpoint = Some(target.clone());
                    drag.target = Some(BondAnchor {
                        node_id: Some(target.node_id.clone()),
                        object_id: Some(target.object_id.clone()),
                        point: target.point,
                        label_anchor: target.label_anchor.clone(),
                    });
                    target.point
                } else if drag.free_length {
                    drag.target = None;
                    point
                } else {
                    drag.target = None;
                    let angle = snapped_angle_for_anchor(&self.state.document, &drag.anchor, point);
                    endpoint_from_angle_for_document(
                        &self.state.document,
                        &drag.anchor,
                        angle,
                        self.options.bond_length_world_pt().value(),
                    )
                };
                drag.preview_end = Some(end);
                self.state.overlay.preview = Some(BondPreview {
                    start: drag.anchor.point,
                    end,
                });
            }
            self.drag = Some(drag);
            return;
        }

        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.hover_text_box = None;
        let endpoint_hit = hit_test_endpoint(&self.state.document, point, endpoint_hit_radius);
        if endpoint_hit
            .as_ref()
            .is_some_and(|endpoint| endpoint.distance <= endpoint_focus_radius)
        {
            self.state.overlay.hover_endpoint = endpoint_hit;
            return;
        }
        if can_focus_bond_center(&self.state.tool) {
            if let Some(center) =
                hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
            {
                self.state.overlay.hover_bond_center = Some(center);
                return;
            }
        }
        self.state.overlay.hover_endpoint = endpoint_hit;
    }

    pub(super) fn pointer_move_element(&mut self, event: PointerEvent) {
        let point = event.point();
        self.drag = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_arrow = None;
        self.state.overlay.hover_shape = None;
        self.state.overlay.preview = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.hover_endpoint = None;

        if let Some((node_id, bounds)) = self.hit_test_endpoint_label_box(point) {
            self.state.overlay.hover_text_box = Some(HoverTextBox {
                bounds,
                object_id: None,
                node_id: Some(node_id),
            });
            return;
        }
        self.state.overlay.hover_endpoint =
            hit_test_endpoint(&self.state.document, point, self.endpoint_hit_radius());
    }

    pub(super) fn element_replacement_node_at_point(&self, point: Point) -> Option<String> {
        self.hit_test_endpoint_label_box(point)
            .map(|(node_id, _)| node_id)
            .or_else(|| {
                hit_test_endpoint(&self.state.document, point, self.endpoint_hit_radius())
                    .map(|hit| hit.node_id)
            })
    }

    pub fn pointer_down(&mut self, event: PointerEvent) {
        if self.state.tool.active_tool == Tool::Select {
            return;
        }
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_down_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Table {
            self.pointer_down_table(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_down_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Orbital {
            self.pointer_down_orbital(event);
            return;
        }
        if matches!(self.state.tool.active_tool, Tool::Shape | Tool::TlcPlate) {
            self.pointer_down_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Bracket {
            self.pointer_down_bracket(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Symbol {
            self.pointer_down_symbol(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Element {
            let point = event.point();
            self.state.selection = SelectionState::default();
            if let Some(node_id) = self.element_replacement_node_at_point(point) {
                let label = self.state.tool.element_symbol.clone();
                let target_node_id = node_id.clone();
                let target_label = label.clone();
                self.with_command(
                    EditorCommand::ReplaceNodeLabel { node_id, label },
                    |engine| engine.replace_node_label_untracked(&target_node_id, &target_label),
                );
                return;
            }
            self.clear_interaction();
            let symbol = self.state.tool.element_symbol.clone();
            let atomic_number = self.state.tool.element_atomic_number;
            self.with_command(
                EditorCommand::AddElement {
                    symbol,
                    atomic_number,
                    center: CommandAnchor::from(point),
                },
                |engine| engine.insert_periodic_element(point),
            );
            return;
        }
        if self.state.tool.active_tool == Tool::Delete {
            self.state.selection = SelectionState::default();
            self.clear_interaction();
            let point = event.point();
            self.with_command(
                EditorCommand::DeleteFocusedAtPoint {
                    x: point.x,
                    y: point.y,
                    source: FocusedDeleteSource::DeleteTool,
                },
                |engine| engine.delete_focused_at_point(point, FocusedDeleteMode::DeleteToolClick),
            );
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.state.selection = SelectionState::default();
            self.clear_interaction();
            self.state.overlay.hover_endpoint = hit_test_endpoint(
                &self.state.document,
                event.point(),
                self.endpoint_hit_radius(),
            );
            return;
        }
        if !can_draw_bond(&self.state.tool) {
            if can_focus_bond_center(&self.state.tool) {
                if let Some(hit) = hit_test_bond_center(
                    &self.state.document,
                    event.point(),
                    BOND_CENTER_HIT_RADIUS,
                ) {
                    self.cycle_bond_center_style(&hit.bond_id);
                }
            }
            return;
        }
        let point = event.point();
        let endpoint_hit =
            hit_test_endpoint(&self.state.document, point, self.endpoint_hit_radius());
        if let Some(endpoint) = endpoint_hit
            .clone()
            .filter(|endpoint| endpoint.distance <= self.endpoint_focus_radius())
        {
            self.clear_overlay();
            self.drag = Some(DragState {
                anchor: BondAnchor {
                    node_id: Some(endpoint.node_id),
                    object_id: Some(endpoint.object_id),
                    point: endpoint.point,
                    label_anchor: endpoint.label_anchor,
                },
                start: point,
                has_dragged: false,
                free_length: event.alt_key,
                preview_end: None,
                target: None,
            });
            return;
        }
        if let Some(hit) = hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS)
        {
            self.cycle_bond_center_style(&hit.bond_id);
            return;
        }
        if let Some(endpoint) = endpoint_hit {
            self.clear_overlay();
            self.drag = Some(DragState {
                anchor: BondAnchor {
                    node_id: Some(endpoint.node_id),
                    object_id: Some(endpoint.object_id),
                    point: endpoint.point,
                    label_anchor: endpoint.label_anchor,
                },
                start: point,
                has_dragged: false,
                free_length: event.alt_key,
                preview_end: None,
                target: None,
            });
            return;
        }
        let Some(anchor) = anchor_from_point(&self.state.document, point) else {
            return;
        };
        self.clear_overlay();
        self.drag = Some(DragState {
            anchor,
            start: point,
            has_dragged: false,
            free_length: false,
            preview_end: None,
            target: None,
        });
    }

    pub fn pointer_up(&mut self, event: PointerEvent) {
        if self.state.tool.active_tool == Tool::Arrow {
            self.pointer_up_arrow(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Table {
            self.pointer_up_table(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Text {
            self.state.overlay.hover_endpoint = hit_test_endpoint(
                &self.state.document,
                event.point(),
                self.endpoint_hit_radius(),
            );
            return;
        }
        if self.state.tool.active_tool == Tool::Templates {
            self.pointer_up_template(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Orbital {
            self.pointer_up_orbital(event);
            return;
        }
        if matches!(self.state.tool.active_tool, Tool::Shape | Tool::TlcPlate) {
            self.pointer_up_shape(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Bracket {
            self.pointer_up_bracket(event);
            return;
        }
        if self.state.tool.active_tool == Tool::Symbol {
            self.pointer_up_symbol(event);
            return;
        }
        let Some(drag) = self.drag.take() else {
            return;
        };
        let end_anchor = if drag.has_dragged {
            if let Some(target) = drag.target {
                target
            } else if drag.free_length {
                BondAnchor {
                    node_id: None,
                    object_id: drag.anchor.object_id.clone(),
                    point: drag.preview_end.unwrap_or_else(|| event.point()),
                    label_anchor: None,
                }
            } else {
                let end = drag.preview_end.unwrap_or_else(|| {
                    let angle =
                        snapped_angle_for_anchor(&self.state.document, &drag.anchor, event.point());
                    endpoint_from_angle_for_document(
                        &self.state.document,
                        &drag.anchor,
                        angle,
                        self.options.bond_length_world_pt().value(),
                    )
                });
                BondAnchor {
                    node_id: None,
                    object_id: drag.anchor.object_id.clone(),
                    point: end,
                    label_anchor: None,
                }
            }
        } else {
            let angle = default_angle_for_anchor_for_variant(
                &self.state.document,
                &drag.anchor,
                self.state.tool.bond_variant,
            );
            let end = endpoint_from_angle_for_document(
                &self.state.document,
                &drag.anchor,
                angle,
                self.options.bond_length_world_pt().value(),
            );
            self.endpoint_anchor_near(&drag.anchor, end)
                .unwrap_or(BondAnchor {
                    node_id: None,
                    object_id: drag.anchor.object_id.clone(),
                    point: end,
                    label_anchor: None,
                })
        };
        self.state.overlay.preview = None;
        let _ = self.add_bond_between(drag.anchor, end_anchor, self.pending_bond_order());
        // Do not synthesize a hover target at the committed endpoint; the next
        // pointer move should be the source of truth for focus feedback.
        self.clear_overlay();
    }

    pub fn clear_interaction(&mut self) {
        self.drag = None;
        self.arrow_drag = None;
        self.arrow_edit_drag = None;
        self.tlc_spot_drag = None;
        self.orbital_drag = None;
        self.selection_drag = None;
        self.selection_rotate_drag = None;
        self.selection_resize_drag = None;
        self.template_drag = None;
        self.shape_drag = None;
        self.shape_edit_drag = None;
        self.bracket_edit_drag = None;
        self.bracket_drag = None;
        self.pointer_bond_target = None;
        self.state.overlay = OverlayState::default();
    }

    pub(super) fn note_pending_select_target(&mut self, target: PendingSelectTarget) {
        self.pending_select_target = Some(target);
    }

    pub fn pending_graphic_object_id(&self) -> Option<&str> {
        match self.pending_select_target.as_ref() {
            Some(PendingSelectTarget::GraphicObject(object_id)) => Some(object_id.as_str()),
            _ => None,
        }
    }

    pub(super) fn select_pending_target_for_select_tool(&mut self) {
        let Some(target) = self.pending_select_target.take() else {
            return;
        };
        let Some(selection) = self.selection_for_pending_target(&target) else {
            return;
        };
        self.state.selection = selection;
    }

    pub(super) fn selection_for_pending_target(
        &self,
        target: &PendingSelectTarget,
    ) -> Option<SelectionState> {
        match target {
            PendingSelectTarget::GraphicObject(object_id) => {
                let object = self
                    .state
                    .document
                    .objects
                    .iter()
                    .find(|object| object.id == *object_id)?;
                if object.object_type == "text" {
                    return None;
                }
                if object.object_type == "group"
                    && object.meta.get("kind").and_then(JsonValue::as_str) == Some("bracket-group")
                {
                    let arrow_objects: Vec<String> = object
                        .children
                        .iter()
                        .filter(|child| child.visible && child.object_type != "text")
                        .map(|child| child.id.clone())
                        .collect();
                    return (!arrow_objects.is_empty()).then_some(SelectionState {
                        arrow_objects,
                        ..SelectionState::default()
                    });
                }
                Some(SelectionState {
                    arrow_objects: vec![object_id.clone()],
                    ..SelectionState::default()
                })
            }
            PendingSelectTarget::SceneObjects {
                arrow_objects,
                text_objects,
            } => {
                let mut selection = SelectionState::default();
                for object_id in arrow_objects {
                    if self
                        .state
                        .document
                        .find_scene_object(object_id)
                        .is_some_and(|object| object.object_type != "text")
                    {
                        selection.arrow_objects.push(object_id.clone());
                    }
                }
                for object_id in text_objects {
                    if self
                        .state
                        .document
                        .find_scene_object(object_id)
                        .is_some_and(|object| object.object_type == "text")
                    {
                        selection.text_objects.push(object_id.clone());
                    }
                }
                (!selection.is_empty()).then_some(selection)
            }
            PendingSelectTarget::TextObject(object_id) => self
                .state
                .document
                .objects
                .iter()
                .any(|object| object.id == *object_id && object.object_type == "text")
                .then(|| SelectionState {
                    text_objects: vec![object_id.clone()],
                    ..SelectionState::default()
                }),
            PendingSelectTarget::MoleculeNode(node_id) => {
                self.selection_for_molecule_component_containing_node(node_id)
            }
            PendingSelectTarget::MoleculeBond(bond_id) => {
                self.selection_for_molecule_component_containing_bond(bond_id)
            }
            PendingSelectTarget::MoleculeSelection { nodes, bonds } => Some(SelectionState {
                nodes: nodes.clone(),
                bonds: bonds.clone(),
                ..SelectionState::default()
            }),
        }
    }

    pub(super) fn selection_for_molecule_component_containing_bond(
        &self,
        bond_id: &str,
    ) -> Option<SelectionState> {
        let entry = self.state.document.editable_fragment()?;
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)?;
        self.selection_for_molecule_component_containing_node(&bond.begin)
    }

    pub(super) fn selection_for_molecule_component_containing_node(
        &self,
        node_id: &str,
    ) -> Option<SelectionState> {
        let entry = self.state.document.editable_fragment()?;
        if !entry.fragment.nodes.iter().any(|node| node.id == node_id) {
            return None;
        }
        let nodes = connected_component_node_ids_for_fragment(entry.fragment, node_id);
        let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
        let bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                node_set.contains(bond.begin.as_str()) && node_set.contains(bond.end.as_str())
            })
            .map(|bond| bond.id.clone())
            .collect::<Vec<_>>();
        let covers_whole_molecule_object =
            nodes.len() == entry.fragment.nodes.len() && bonds.len() == entry.fragment.bonds.len();
        let mut selection = SelectionState {
            nodes,
            bonds,
            ..SelectionState::default()
        };
        if covers_whole_molecule_object {
            selection.molecule_objects.push(entry.object.id.clone());
        }
        Some(selection)
    }
}
