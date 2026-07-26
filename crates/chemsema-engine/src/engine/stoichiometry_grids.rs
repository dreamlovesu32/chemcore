use super::*;
use std::collections::{BTreeMap, BTreeSet};

const STOICHIOMETRY_HEADER_HEIGHT: f64 = 28.0;
const STOICHIOMETRY_LABEL_COLUMN_WIDTH: f64 = 86.0;
const STOICHIOMETRY_COMPONENT_WIDTH: f64 = 72.0;
const STOICHIOMETRY_ROW_HEIGHT: f64 = 18.0;
const STOICHIOMETRY_GAP_BELOW_REACTION: f64 = 24.0;

impl Engine {
    pub(super) fn stoichiometry_cell_at_point(
        &self,
        object_id: &str,
        point: Point,
    ) -> Option<serde_json::Value> {
        let object = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.object_type == "stoichiometry-grid")?;
        let grid = object.payload.stoichiometry_grid.as_ref()?;
        let local_x = point.x - object.transform.translate[0];
        let local_y = point.y - object.transform.translate[1];
        let mut x = 0.0;
        let component = grid
            .components
            .iter()
            .filter(|component| component.visible)
            .find(|component| {
                let contains = local_x >= x && local_x <= x + component.width;
                x += component.width;
                contains
            })?;
        if local_y < STOICHIOMETRY_HEADER_HEIGHT || component.is_header {
            return Some(json!({
                "componentId": component.id,
                "header": true,
            }));
        }
        let mut y = STOICHIOMETRY_HEADER_HEIGHT;
        let row = grid.rows.iter().filter(|row| row.visible).find(|row| {
            let contains = local_y >= y && local_y <= y + row.height;
            y += row.height;
            contains
        })?;
        let datum = grid
            .data
            .iter()
            .find(|datum| datum.component_id == component.id && datum.row_id == row.id)?;
        Some(json!({
            "componentId": component.id,
            "rowId": row.id,
            "propertyType": row.property_type,
            "label": row.label,
            "dataType": row.data_type,
            "value": datum.value.display,
            "unit": datum.value.unit.as_deref().or(row.default_unit.as_deref()).unwrap_or(""),
            "readOnly": datum.is_read_only,
            "hidden": datum.is_hidden,
            "visible": datum.visible,
        }))
    }

    pub fn can_analyze_stoichiometry(&self) -> bool {
        self.selected_reaction_step_id().is_some() || self.can_create_reaction_step_from_selection()
    }

    pub fn analyze_stoichiometry(&mut self, reaction_step_id: Option<&str>) -> bool {
        self.with_command(
            EditorCommand::AnalyzeStoichiometry {
                reaction_step_id: reaction_step_id.map(ToString::to_string),
            },
            |engine| engine.analyze_stoichiometry_untracked(reaction_step_id),
        )
    }

    pub(super) fn analyze_stoichiometry_untracked(
        &mut self,
        reaction_step_id: Option<&str>,
    ) -> bool {
        let step_id = if let Some(id) = reaction_step_id {
            self.find_reaction_step(id).map(|step| step.id.clone())
        } else {
            self.selected_reaction_step_id()
        }
        .or_else(|| self.create_reaction_step_from_selection_untracked());
        let Some(step_id) = step_id else {
            return false;
        };
        let Some(step) = self.find_reaction_step(&step_id).cloned() else {
            return false;
        };
        let member_ids = step
            .reactant_entity_ids
            .iter()
            .chain(step.product_entity_ids.iter())
            .chain(step.objects_above_arrow.iter())
            .chain(step.objects_below_arrow.iter())
            .cloned()
            .collect::<Vec<_>>();
        let Some(bounds) = self.bounds_for_scene_entities(&member_ids) else {
            return false;
        };
        let object_id = self.next_id("obj_stoichiometry_grid");
        let mut components = vec![crate::StoichiometryComponent {
            id: format!("{object_id}_header"),
            role: crate::StoichiometryComponentRole::Header,
            reference_entity_id: None,
            unresolved_reference_id: None,
            is_header: true,
            visible: true,
            width: STOICHIOMETRY_LABEL_COLUMN_WIDTH,
        }];
        for (role, ids) in [
            (
                crate::StoichiometryComponentRole::Reactant,
                &step.reactant_entity_ids,
            ),
            (
                crate::StoichiometryComponentRole::Product,
                &step.product_entity_ids,
            ),
            (
                crate::StoichiometryComponentRole::Reagent,
                &step.objects_above_arrow,
            ),
            (
                crate::StoichiometryComponentRole::Condition,
                &step.objects_below_arrow,
            ),
        ] {
            for entity_id in ids {
                components.push(crate::StoichiometryComponent {
                    id: format!("{object_id}_component_{}", components.len()),
                    role,
                    reference_entity_id: Some(entity_id.clone()),
                    unresolved_reference_id: None,
                    is_header: false,
                    visible: true,
                    width: STOICHIOMETRY_COMPONENT_WIDTH,
                });
            }
        }
        let rows = default_stoichiometry_rows(&object_id);
        let mut data = Vec::new();
        for component in components.iter().filter(|component| !component.is_header) {
            for row in &rows {
                data.push(crate::StoichiometryDatum {
                    id: format!("{object_id}_datum_{}_{}", component.id, row.id),
                    component_id: component.id.clone(),
                    row_id: row.id.clone(),
                    value: Default::default(),
                    origin: crate::StoichiometryValueOrigin::Empty,
                    is_edited: false,
                    is_hidden: false,
                    is_read_only: matches!(
                        normalized_property(&row.property_type),
                        "formula" | "molecularweight"
                    ),
                    visible: true,
                    calculation_state: crate::StoichiometryCalculationState::Empty,
                });
            }
        }
        let width = components
            .iter()
            .filter(|component| component.visible)
            .map(|component| component.width)
            .sum::<f64>();
        let height = STOICHIOMETRY_HEADER_HEIGHT + rows.iter().map(|row| row.height).sum::<f64>();
        let left = (bounds[0] + bounds[2] - width) * 0.5;
        let top = bounds[3] + STOICHIOMETRY_GAP_BELOW_REACTION;
        let grid = crate::StoichiometryGridData {
            source_reaction_step_id: Some(step_id),
            binding_origin: crate::StoichiometryBindingOrigin::Authored,
            binding_state: crate::StoichiometryBindingState::Current,
            anchor_mode: crate::StoichiometryAnchorMode::Follow,
            components,
            rows,
            data,
            style: crate::StoichiometryGridStyle {
                line_width: self.options.graphic_stroke_width,
                bold_width: self
                    .state
                    .document
                    .style
                    .defaults
                    .get("boldWidth")
                    .copied()
                    .unwrap_or(1.5),
                margin_width: self
                    .state
                    .document
                    .style
                    .defaults
                    .get("marginWidth")
                    .copied()
                    .unwrap_or(2.0),
                ..Default::default()
            },
        };
        self.state.document.objects.push(SceneObject {
            id: object_id.clone(),
            object_type: "stoichiometry-grid".to_string(),
            name: "Stoichiometry Grid".to_string(),
            visible: true,
            locked: false,
            z_index: self.next_shape_z_index(),
            transform: crate::Transform {
                translate: [round2(left), round2(top)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: crate::LinkPolicy::Linked,
            meta: json!({"source": "authored"}),
            payload: crate::ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, round2(width), round2(height)]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: Some(grid),
                gel_electrophoresis: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        });
        self.refresh_stoichiometry_grid_values(&object_id);
        self.note_pending_select_target(PendingSelectTarget::GraphicObject(object_id));
        true
    }

    pub(super) fn set_stoichiometry_datum_untracked(
        &mut self,
        object_id: &str,
        component_id: &str,
        row_id: &str,
        value: &str,
        unit: Option<&str>,
    ) -> bool {
        let trimmed = value.trim();
        let row = self
            .state
            .document
            .find_scene_object(object_id)
            .and_then(|object| object.payload.stoichiometry_grid.as_ref())
            .and_then(|grid| grid.rows.iter().find(|row| row.id == row_id))
            .cloned();
        let Some(row) = row else {
            return false;
        };
        if row_uses_numeric_value(&row.property_type) && !trimmed.is_empty() {
            if trimmed.parse::<f64>().is_err()
                || unit.is_some_and(|unit| !unit_allowed_for_property(&row.property_type, unit))
            {
                return false;
            }
        }
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        let Some(grid) = object.payload.stoichiometry_grid.as_mut() else {
            return false;
        };
        let Some(datum) = grid
            .data
            .iter_mut()
            .find(|datum| datum.component_id == component_id && datum.row_id == row_id)
        else {
            return false;
        };
        if datum.is_read_only {
            return false;
        }
        let next_value = crate::StoichiometryValue {
            canonical: trimmed.to_string(),
            display: trimmed.to_string(),
            unit: unit.map(ToString::to_string),
        };
        if datum.value == next_value && datum.origin == crate::StoichiometryValueOrigin::Authored {
            return false;
        }
        datum.value = next_value;
        datum.origin = if trimmed.is_empty() {
            crate::StoichiometryValueOrigin::Empty
        } else {
            crate::StoichiometryValueOrigin::Authored
        };
        datum.is_edited = !trimmed.is_empty();
        datum.calculation_state = if trimmed.is_empty() {
            crate::StoichiometryCalculationState::Empty
        } else {
            crate::StoichiometryCalculationState::Current
        };
        self.refresh_stoichiometry_grid_values(object_id);
        true
    }

    pub(super) fn edit_stoichiometry_grid_untracked(
        &mut self,
        object_id: &str,
        action: &str,
        entity_id: Option<&str>,
    ) -> bool {
        if action == "refresh" {
            return self.refresh_stoichiometry_grid_values(object_id);
        }
        if action == "detach" {
            return self.bind_stoichiometry_grid_untracked(
                object_id,
                None,
                crate::LinkPolicy::Unlinked,
            );
        }
        let add_component_reference_exists = entity_id.is_some_and(|reference_id| {
            self.state
                .document
                .find_scene_object(reference_id)
                .is_some()
        });
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        let Some(grid) = object.payload.stoichiometry_grid.as_mut() else {
            return false;
        };
        let changed = match action {
            "delete-row" => entity_id.is_some_and(|row_id| {
                let before = grid.rows.len();
                grid.rows.retain(|row| row.id != row_id);
                grid.data.retain(|datum| datum.row_id != row_id);
                before != grid.rows.len()
            }),
            "delete-component" => entity_id.is_some_and(|component_id| {
                let before = grid.components.len();
                grid.components
                    .retain(|component| component.id != component_id || component.is_header);
                grid.data.retain(|datum| datum.component_id != component_id);
                before != grid.components.len()
            }),
            "toggle-row-visible" => entity_id
                .and_then(|row_id| grid.rows.iter_mut().find(|row| row.id == row_id))
                .is_some_and(|row| {
                    row.visible = !row.visible;
                    true
                }),
            "toggle-component-visible" => entity_id
                .and_then(|component_id| {
                    grid.components
                        .iter_mut()
                        .find(|component| component.id == component_id)
                })
                .is_some_and(|component| {
                    component.visible = !component.visible;
                    true
                }),
            "toggle-datum-hidden" | "toggle-datum-read-only" => entity_id
                .and_then(|cell_id| cell_id.split_once('|'))
                .and_then(|(component_id, row_id)| {
                    grid.data
                        .iter_mut()
                        .find(|datum| datum.component_id == component_id && datum.row_id == row_id)
                })
                .is_some_and(|datum| {
                    if action == "toggle-datum-hidden" {
                        datum.is_hidden = !datum.is_hidden;
                    } else {
                        datum.is_read_only = !datum.is_read_only;
                    }
                    true
                }),
            action if action.starts_with("set-component-role-") => entity_id
                .and_then(|component_id| {
                    grid.components
                        .iter_mut()
                        .find(|component| component.id == component_id && !component.is_header)
                })
                .is_some_and(|component| {
                    let role = match action.trim_start_matches("set-component-role-") {
                        "reactant" => crate::StoichiometryComponentRole::Reactant,
                        "product" => crate::StoichiometryComponentRole::Product,
                        "reagent" => crate::StoichiometryComponentRole::Reagent,
                        "condition" => crate::StoichiometryComponentRole::Condition,
                        "unspecified" => crate::StoichiometryComponentRole::Unspecified,
                        _ => return false,
                    };
                    if component.role == role {
                        return false;
                    }
                    component.role = role;
                    true
                }),
            "add-row" => entity_id.is_some_and(|property_type| {
                if grid
                    .rows
                    .iter()
                    .any(|row| row.property_type.eq_ignore_ascii_case(property_type))
                {
                    return false;
                }
                let row_id = format!("{object_id}_row_{}", grid.rows.len() + 1);
                grid.rows.push(crate::StoichiometryRow {
                    id: row_id.clone(),
                    property_type: property_type.to_string(),
                    data_type: if row_uses_numeric_value(property_type) {
                        "Number".to_string()
                    } else {
                        "Text".to_string()
                    },
                    label: property_type.to_string(),
                    default_unit: None,
                    visible: true,
                    height: STOICHIOMETRY_ROW_HEIGHT,
                });
                for component in grid
                    .components
                    .iter()
                    .filter(|component| !component.is_header)
                {
                    grid.data.push(crate::StoichiometryDatum {
                        id: format!("{object_id}_datum_{}_{}", component.id, row_id),
                        component_id: component.id.clone(),
                        row_id: row_id.clone(),
                        value: Default::default(),
                        origin: Default::default(),
                        is_edited: false,
                        is_hidden: false,
                        is_read_only: false,
                        visible: true,
                        calculation_state: Default::default(),
                    });
                }
                true
            }),
            "add-component" => entity_id.is_some_and(|reference_id| {
                if !add_component_reference_exists
                    || grid.components.iter().any(|component| {
                        component.reference_entity_id.as_deref() == Some(reference_id)
                    })
                {
                    return false;
                }
                let component_id = format!("{object_id}_component_{}", grid.components.len() + 1);
                grid.components.push(crate::StoichiometryComponent {
                    id: component_id.clone(),
                    role: crate::StoichiometryComponentRole::Reagent,
                    reference_entity_id: Some(reference_id.to_string()),
                    unresolved_reference_id: None,
                    is_header: false,
                    visible: true,
                    width: STOICHIOMETRY_COMPONENT_WIDTH,
                });
                for row in &grid.rows {
                    grid.data.push(crate::StoichiometryDatum {
                        id: format!("{object_id}_datum_{}_{}", component_id, row.id),
                        component_id: component_id.clone(),
                        row_id: row.id.clone(),
                        value: Default::default(),
                        origin: Default::default(),
                        is_edited: false,
                        is_hidden: false,
                        is_read_only: matches!(
                            normalized_property(&row.property_type),
                            "formula" | "molecularweight"
                        ),
                        visible: true,
                        calculation_state: Default::default(),
                    });
                }
                true
            }),
            _ => false,
        };
        if changed {
            resize_stoichiometry_bbox(object);
        }
        changed
    }

    pub(super) fn bind_stoichiometry_grid_untracked(
        &mut self,
        object_id: &str,
        reaction_step_id: Option<&str>,
        policy: crate::LinkPolicy,
    ) -> bool {
        let resolved_step_id = match policy {
            crate::LinkPolicy::Unlinked => None,
            crate::LinkPolicy::Linked => reaction_step_id
                .and_then(|step_id| self.find_reaction_step(step_id))
                .map(|step| step.id.clone()),
            crate::LinkPolicy::Auto => self.unique_reaction_step_for_grid(object_id),
        };
        if policy != crate::LinkPolicy::Unlinked && resolved_step_id.is_none() {
            return false;
        }
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        let Some(grid) = object.payload.stoichiometry_grid.as_mut() else {
            return false;
        };
        let changed = object.link_policy != policy
            || grid.source_reaction_step_id != resolved_step_id
            || (policy == crate::LinkPolicy::Unlinked
                && grid.binding_state != crate::StoichiometryBindingState::Detached);
        if !changed {
            return false;
        }
        object.link_policy = policy;
        grid.source_reaction_step_id = resolved_step_id;
        grid.binding_origin = match policy {
            crate::LinkPolicy::Linked => crate::StoichiometryBindingOrigin::Authored,
            crate::LinkPolicy::Auto => crate::StoichiometryBindingOrigin::Inferred,
            crate::LinkPolicy::Unlinked => crate::StoichiometryBindingOrigin::None,
        };
        grid.binding_state = if policy == crate::LinkPolicy::Unlinked {
            crate::StoichiometryBindingState::Detached
        } else {
            crate::StoichiometryBindingState::Current
        };
        if policy == crate::LinkPolicy::Unlinked {
            for datum in &mut grid.data {
                if datum.origin == crate::StoichiometryValueOrigin::Calculated {
                    datum.origin = crate::StoichiometryValueOrigin::Imported;
                }
            }
        }
        true
    }

    pub(super) fn selection_can_link_stoichiometry(&self) -> bool {
        self.stoichiometry_link_candidate().is_some()
    }

    pub(super) fn stoichiometry_relation_bundle(
        &self,
        seeds: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let mut related = BTreeSet::new();
        for step in self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
        {
            let member_ids = step
                .reactant_entity_ids
                .iter()
                .chain(step.product_entity_ids.iter())
                .chain(step.arrow_object_ids.iter())
                .chain(step.plus_object_ids.iter())
                .chain(step.objects_above_arrow.iter())
                .chain(step.objects_below_arrow.iter())
                .collect::<BTreeSet<_>>();
            let grid_ids = self
                .state
                .document
                .scene_objects()
                .into_iter()
                .filter(|object| {
                    object
                        .payload
                        .stoichiometry_grid
                        .as_ref()
                        .and_then(|grid| grid.source_reaction_step_id.as_deref())
                        == Some(step.id.as_str())
                        && object.link_policy != crate::LinkPolicy::Unlinked
                })
                .map(|object| object.id.as_str())
                .collect::<BTreeSet<_>>();
            if seeds
                .iter()
                .any(|seed| member_ids.contains(seed) || grid_ids.contains(seed.as_str()))
            {
                related.extend(member_ids.into_iter().cloned());
                related.extend(grid_ids.into_iter().map(ToString::to_string));
            }
        }
        related
    }

    pub(super) fn link_stoichiometry_selection_untracked(&mut self) -> bool {
        let Some((grid_id, step_id)) = self.stoichiometry_link_candidate() else {
            return false;
        };
        self.bind_stoichiometry_grid_untracked(&grid_id, Some(&step_id), crate::LinkPolicy::Linked)
    }

    pub(super) fn refresh_stoichiometry_after_document_change(&mut self) -> bool {
        let existing = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .map(|object| object.id.clone())
            .collect::<BTreeSet<_>>();
        let mut changed = false;
        for scheme in &mut self.state.document.reaction_schemes {
            for step in &mut scheme.steps {
                for ids in [
                    &mut step.reactant_entity_ids,
                    &mut step.product_entity_ids,
                    &mut step.arrow_object_ids,
                    &mut step.plus_object_ids,
                    &mut step.objects_above_arrow,
                    &mut step.objects_below_arrow,
                ] {
                    let before = ids.len();
                    ids.retain(|id| existing.contains(id));
                    changed |= before != ids.len();
                }
                let next_state = if step.reactant_entity_ids.is_empty()
                    || step.product_entity_ids.is_empty()
                    || step.arrow_object_ids.is_empty()
                {
                    crate::ReactionInterpretationState::Invalid
                } else {
                    crate::ReactionInterpretationState::Current
                };
                changed |= step.interpretation_state != next_state;
                step.interpretation_state = next_state;
            }
        }
        let grid_ids = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "stoichiometry-grid")
            .map(|object| object.id.clone())
            .collect::<Vec<_>>();
        for grid_id in grid_ids {
            let source_id = self
                .state
                .document
                .find_scene_object(&grid_id)
                .and_then(|object| object.payload.stoichiometry_grid.as_ref())
                .and_then(|grid| grid.source_reaction_step_id.clone());
            let source_valid = source_id.as_deref().is_some_and(|id| {
                self.find_reaction_step(id).is_some_and(|step| {
                    step.interpretation_state == crate::ReactionInterpretationState::Current
                })
            });
            if source_id.is_some() && !source_valid {
                if let Some(object) = self.state.document.find_scene_object_mut(&grid_id) {
                    if let Some(grid) = object.payload.stoichiometry_grid.as_mut() {
                        grid.source_reaction_step_id = None;
                        grid.binding_state = crate::StoichiometryBindingState::Orphaned;
                        object.link_policy = crate::LinkPolicy::Unlinked;
                        for datum in &mut grid.data {
                            if datum.origin == crate::StoichiometryValueOrigin::Calculated {
                                datum.origin = crate::StoichiometryValueOrigin::Imported;
                            }
                        }
                        changed = true;
                    }
                }
            } else if source_valid {
                changed |= self.follow_stoichiometry_grid_anchor(&grid_id);
                changed |= self.refresh_stoichiometry_grid_values(&grid_id);
            }
        }
        changed
    }

    fn refresh_stoichiometry_grid_values(&mut self, object_id: &str) -> bool {
        let component_summaries = self
            .state
            .document
            .find_scene_object(object_id)
            .and_then(|object| object.payload.stoichiometry_grid.as_ref())
            .map(|grid| {
                grid.components
                    .iter()
                    .filter_map(|component| {
                        let reference = component.reference_entity_id.as_deref()?;
                        Some((
                            component.id.clone(),
                            self.chemistry_summary_for_molecule_object(reference),
                            component.role,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        let Some(grid) = object.payload.stoichiometry_grid.as_mut() else {
            return false;
        };
        let row_map = grid
            .rows
            .iter()
            .map(|row| (normalized_property(&row.property_type), row.id.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut changed = false;
        for datum in &mut grid.data {
            let derived_row = grid
                .rows
                .iter()
                .find(|row| row.id == datum.row_id)
                .map(|row| normalized_property(&row.property_type))
                .is_some_and(|property| !matches!(property, "formula" | "molecularweight"));
            if derived_row && datum.origin == crate::StoichiometryValueOrigin::Calculated {
                datum.value = Default::default();
                datum.origin = crate::StoichiometryValueOrigin::Empty;
                datum.calculation_state = crate::StoichiometryCalculationState::Empty;
                changed = true;
            } else if datum.origin == crate::StoichiometryValueOrigin::Authored {
                datum.calculation_state = crate::StoichiometryCalculationState::Current;
            }
        }
        for (component_id, summary, _) in &component_summaries {
            let Some(summary) = summary else {
                continue;
            };
            for (property, value) in [
                ("formula", summary.formula.clone()),
                ("molecularweight", format!("{:.4}", summary.formula_weight)),
            ] {
                let Some(row_id) = row_map.get(property) else {
                    continue;
                };
                if let Some(datum) = grid
                    .data
                    .iter_mut()
                    .find(|datum| datum.component_id == *component_id && datum.row_id == *row_id)
                {
                    let next = crate::StoichiometryValue {
                        canonical: if property == "molecularweight" {
                            summary.formula_weight.to_string()
                        } else {
                            String::new()
                        },
                        display: value,
                        unit: (property == "molecularweight").then(|| "g/mol".to_string()),
                    };
                    changed |= datum.value != next
                        || datum.origin != crate::StoichiometryValueOrigin::Calculated;
                    datum.value = next;
                    datum.origin = crate::StoichiometryValueOrigin::Calculated;
                    datum.is_read_only = true;
                    datum.calculation_state = crate::StoichiometryCalculationState::Current;
                }
            }
        }
        let mass_row = row_map.get("mass").cloned();
        let amount_row = row_map.get("amount").cloned();
        let equivalent_row = row_map.get("equivalents").cloned();
        let mw_row = row_map.get("molecularweight").cloned();
        for (component_id, _, _) in &component_summaries {
            let mw = mw_row
                .as_deref()
                .and_then(|row_id| datum_numeric_value(grid, component_id, row_id))
                .map(|(value, _)| value);
            let mass = mass_row
                .as_deref()
                .and_then(|row_id| datum_numeric_value(grid, component_id, row_id));
            let amount = amount_row
                .as_deref()
                .and_then(|row_id| datum_numeric_value(grid, component_id, row_id));
            if let (Some(mw), Some((mass_value, mass_unit)), Some(amount_row_id)) =
                (mw, mass.clone(), amount_row.as_deref())
            {
                if amount.is_none() {
                    let grams = mass_to_grams(mass_value, mass_unit.as_deref());
                    changed |= set_calculated_numeric(
                        grid,
                        component_id,
                        amount_row_id,
                        grams / mw * 1000.0,
                        "mmol",
                    );
                }
            }
            if let (Some(mw), Some((amount_value, amount_unit)), Some(mass_row_id)) =
                (mw, amount.clone(), mass_row.as_deref())
            {
                if mass.is_none() {
                    let moles = amount_to_moles(amount_value, amount_unit.as_deref());
                    changed |= set_calculated_numeric(
                        grid,
                        component_id,
                        mass_row_id,
                        moles * mw * 1000.0,
                        "mg",
                    );
                }
            }
        }
        for (component_id, _, _) in &component_summaries {
            changed |= derive_stoichiometry_component_values(grid, component_id, &row_map);
        }
        if let (Some(amount_row_id), Some(equivalent_row_id)) =
            (amount_row.as_deref(), equivalent_row.as_deref())
        {
            let limiting = component_summaries
                .iter()
                .filter(|(_, _, role)| *role == crate::StoichiometryComponentRole::Reactant)
                .filter_map(|(component_id, _, _)| {
                    let (value, unit) = datum_numeric_value(grid, component_id, amount_row_id)?;
                    let moles = amount_to_moles(value, unit.as_deref());
                    (moles > 0.0).then_some(moles)
                })
                .min_by(f64::total_cmp);
            if let Some(limiting) = limiting {
                let yield_row = row_map.get("yield");
                for (component_id, _, role) in &component_summaries {
                    if let Some((value, unit)) =
                        datum_numeric_value(grid, component_id, amount_row_id)
                    {
                        let moles = amount_to_moles(value, unit.as_deref());
                        changed |= set_calculated_numeric(
                            grid,
                            component_id,
                            equivalent_row_id,
                            moles / limiting,
                            "eq",
                        );
                        if *role == crate::StoichiometryComponentRole::Product {
                            if let Some(yield_row_id) = yield_row {
                                changed |= set_calculated_numeric(
                                    grid,
                                    component_id,
                                    yield_row_id,
                                    moles / limiting * 100.0,
                                    "%",
                                );
                            }
                        }
                    }
                }
            }
        }
        resize_stoichiometry_bbox(object);
        changed
    }

    fn selected_reaction_step_id(&self) -> Option<String> {
        let selected_arrows = self
            .state
            .selection
            .arrow_objects
            .iter()
            .collect::<BTreeSet<_>>();
        let candidates = self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
            .filter(|step| {
                step.arrow_object_ids
                    .iter()
                    .any(|id| selected_arrows.contains(id))
            })
            .collect::<Vec<_>>();
        (candidates.len() == 1).then(|| candidates[0].id.clone())
    }

    fn can_create_reaction_step_from_selection(&self) -> bool {
        self.state.selection.arrow_objects.len() == 1
            && self.state.selection.molecule_objects.len() >= 2
    }

    fn create_reaction_step_from_selection_untracked(&mut self) -> Option<String> {
        if !self.can_create_reaction_step_from_selection() {
            return None;
        }
        let arrow_id = self.state.selection.arrow_objects[0].clone();
        let arrow = self.state.document.find_scene_object(&arrow_id)?;
        let (start, end) = crate::arrow_payload_line_endpoints(&arrow.payload.extra)?;
        let axis_x = end.x - start.x;
        let axis_y = end.y - start.y;
        if axis_x.hypot(axis_y) <= crate::EPSILON {
            return None;
        }
        let center = Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
        let mut reactants = Vec::new();
        let mut products = Vec::new();
        for object_id in &self.state.selection.molecule_objects {
            let object = self.state.document.find_scene_object(object_id)?;
            let [x, y, width, height] = object.payload.bbox?;
            let molecule_center = Point::new(
                object.transform.translate[0] + x + width * 0.5,
                object.transform.translate[1] + y + height * 0.5,
            );
            let projection =
                (molecule_center.x - center.x) * axis_x + (molecule_center.y - center.y) * axis_y;
            if projection < 0.0 {
                reactants.push(object_id.clone());
            } else {
                products.push(object_id.clone());
            }
        }
        if reactants.is_empty() || products.is_empty() {
            return None;
        }
        let step_id = self.next_id("reaction_step");
        let step = crate::ReactionStepData {
            id: step_id.clone(),
            reactant_entity_ids: reactants,
            product_entity_ids: products,
            arrow_object_ids: vec![arrow_id],
            plus_object_ids: Vec::new(),
            objects_above_arrow: self.state.selection.text_objects.clone(),
            objects_below_arrow: Vec::new(),
            atom_mappings: Vec::new(),
            interpretation_state: crate::ReactionInterpretationState::Current,
        };
        if let Some(scheme) = self.state.document.reaction_schemes.first_mut() {
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
        Some(step_id)
    }

    fn find_reaction_step(&self, step_id: &str) -> Option<&crate::ReactionStepData> {
        self.state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
            .find(|step| step.id == step_id)
    }

    fn unique_reaction_step_for_grid(&self, object_id: &str) -> Option<String> {
        let grid = self
            .state
            .document
            .find_scene_object(object_id)?
            .payload
            .stoichiometry_grid
            .as_ref()?;
        let references = grid
            .components
            .iter()
            .filter_map(|component| component.reference_entity_id.as_ref())
            .collect::<BTreeSet<_>>();
        if references.is_empty() {
            return None;
        }
        let candidates = self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
            .filter(|step| {
                let members = step
                    .reactant_entity_ids
                    .iter()
                    .chain(step.product_entity_ids.iter())
                    .chain(step.objects_above_arrow.iter())
                    .chain(step.objects_below_arrow.iter())
                    .collect::<BTreeSet<_>>();
                references.is_subset(&members)
                    && grid.components.iter().all(|component| {
                        component
                            .reference_entity_id
                            .as_ref()
                            .is_none_or(|reference| match component.role {
                                crate::StoichiometryComponentRole::Reactant => {
                                    step.reactant_entity_ids.contains(reference)
                                }
                                crate::StoichiometryComponentRole::Product => {
                                    step.product_entity_ids.contains(reference)
                                }
                                crate::StoichiometryComponentRole::Reagent => {
                                    step.objects_above_arrow.contains(reference)
                                }
                                crate::StoichiometryComponentRole::Condition => {
                                    step.objects_below_arrow.contains(reference)
                                }
                                _ => true,
                            })
                    })
            })
            .collect::<Vec<_>>();
        (candidates.len() == 1).then(|| candidates[0].id.clone())
    }

    fn stoichiometry_link_candidate(&self) -> Option<(String, String)> {
        let grid_ids = self
            .state
            .selection
            .arrow_objects
            .iter()
            .filter(|id| {
                self.state
                    .document
                    .find_scene_object(id)
                    .is_some_and(|object| object.object_type == "stoichiometry-grid")
            })
            .cloned()
            .collect::<Vec<_>>();
        let arrow_ids = self
            .state
            .selection
            .arrow_objects
            .iter()
            .filter(|id| {
                self.state
                    .document
                    .find_scene_object(id)
                    .is_some_and(|object| object.object_type == "line")
            })
            .collect::<BTreeSet<_>>();
        if grid_ids.len() != 1 || arrow_ids.len() != 1 {
            return None;
        }
        let candidates = self
            .state
            .document
            .reaction_schemes
            .iter()
            .flat_map(|scheme| scheme.steps.iter())
            .filter(|step| {
                step.arrow_object_ids
                    .iter()
                    .any(|id| arrow_ids.contains(id))
            })
            .collect::<Vec<_>>();
        (candidates.len() == 1).then(|| (grid_ids[0].clone(), candidates[0].id.clone()))
    }

    fn bounds_for_scene_entities(&self, ids: &[String]) -> Option<[f64; 4]> {
        ids.iter()
            .filter_map(|id| self.state.document.find_scene_object(id))
            .filter_map(scene_object_bbox)
            .reduce(union_bounds)
    }

    fn follow_stoichiometry_grid_anchor(&mut self, object_id: &str) -> bool {
        let (step_id, mode, width) = self
            .state
            .document
            .find_scene_object(object_id)
            .and_then(|object| {
                let grid = object.payload.stoichiometry_grid.as_ref()?;
                Some((
                    grid.source_reaction_step_id.clone()?,
                    grid.anchor_mode,
                    object.payload.bbox?[2],
                ))
            })
            .unwrap_or_default();
        if mode != crate::StoichiometryAnchorMode::Follow {
            return false;
        }
        let Some(step) = self.find_reaction_step(&step_id) else {
            return false;
        };
        let ids = step
            .reactant_entity_ids
            .iter()
            .chain(step.product_entity_ids.iter())
            .chain(step.arrow_object_ids.iter())
            .cloned()
            .collect::<Vec<_>>();
        let Some(bounds) = self.bounds_for_scene_entities(&ids) else {
            return false;
        };
        let next = [
            round2((bounds[0] + bounds[2] - width) * 0.5),
            round2(bounds[3] + STOICHIOMETRY_GAP_BELOW_REACTION),
        ];
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        if object.transform.translate == next {
            return false;
        }
        object.transform.translate = next;
        true
    }
}

fn default_stoichiometry_rows(object_id: &str) -> Vec<crate::StoichiometryRow> {
    [
        ("Formula", "Text", None),
        ("MolecularWeight", "Number", Some("g/mol")),
        ("Mass", "Number", Some("mg")),
        ("Amount", "Number", Some("mmol")),
        ("Equivalents", "Number", Some("eq")),
        ("Concentration", "Number", Some("M")),
        ("Volume", "Number", Some("mL")),
        ("Density", "Number", Some("g/mL")),
        ("Yield", "Number", Some("%")),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (property, data_type, unit))| crate::StoichiometryRow {
            id: format!("{object_id}_row_{}", index + 1),
            property_type: property.to_string(),
            data_type: data_type.to_string(),
            label: property_label(property).to_string(),
            default_unit: unit.map(ToString::to_string),
            visible: true,
            height: STOICHIOMETRY_ROW_HEIGHT,
        },
    )
    .collect()
}

fn property_label(property: &str) -> &str {
    match normalized_property(property) {
        "molecularweight" => "Mol. Wt.",
        "equivalents" => "Equiv.",
        _ => property,
    }
}

fn normalized_property(property: &str) -> &'static str {
    match property
        .to_ascii_lowercase()
        .replace([' ', '_', '-'], "")
        .as_str()
    {
        "formula" => "formula",
        "molecularweight" | "formulaweight" | "molwt" => "molecularweight",
        "mass" => "mass",
        "amount" | "moles" => "amount",
        "equivalent" | "equivalents" | "equiv" => "equivalents",
        "concentration" => "concentration",
        "volume" => "volume",
        "density" => "density",
        "yield" | "percentyield" => "yield",
        _ => "custom",
    }
}

fn row_uses_numeric_value(property: &str) -> bool {
    normalized_property(property) != "formula" && normalized_property(property) != "custom"
}

fn unit_allowed_for_property(property: &str, unit: &str) -> bool {
    match normalized_property(property) {
        "molecularweight" => matches!(unit, "g/mol"),
        "mass" => matches!(unit, "g" | "mg" | "µg" | "μg" | "ug"),
        "amount" => matches!(unit, "mol" | "mmol" | "µmol" | "μmol" | "umol"),
        "equivalents" => matches!(unit, "eq"),
        "concentration" => matches!(unit, "M" | "mM"),
        "volume" => matches!(unit, "L" | "mL" | "µL" | "μL" | "uL"),
        "density" => matches!(unit, "g/mL"),
        "yield" => matches!(unit, "%"),
        _ => unit.is_empty(),
    }
}

fn datum_numeric_value(
    grid: &crate::StoichiometryGridData,
    component_id: &str,
    row_id: &str,
) -> Option<(f64, Option<String>)> {
    let datum = grid
        .data
        .iter()
        .find(|datum| datum.component_id == component_id && datum.row_id == row_id)?;
    let value = datum.value.canonical.parse().ok()?;
    Some((value, datum.value.unit.clone()))
}

fn derive_stoichiometry_component_values(
    grid: &mut crate::StoichiometryGridData,
    component_id: &str,
    rows: &BTreeMap<&str, String>,
) -> bool {
    let mut changed = false;
    for _ in 0..3 {
        let mw = base_value(grid, component_id, rows, "molecularweight");
        let mass_g = base_value(grid, component_id, rows, "mass");
        let amount_mol = base_value(grid, component_id, rows, "amount");
        let concentration_molar = base_value(grid, component_id, rows, "concentration");
        let volume_l = base_value(grid, component_id, rows, "volume");
        let density_g_ml = base_value(grid, component_id, rows, "density");

        if let (Some(mw), Some(mass_g), Some(row_id)) = (mw, mass_g, rows.get("amount")) {
            changed |=
                set_calculated_numeric(grid, component_id, row_id, mass_g / mw * 1000.0, "mmol");
        }
        if let (Some(mw), Some(amount_mol), Some(row_id)) = (mw, amount_mol, rows.get("mass")) {
            changed |=
                set_calculated_numeric(grid, component_id, row_id, amount_mol * mw * 1000.0, "mg");
        }
        if let (Some(concentration), Some(volume), Some(row_id)) =
            (concentration_molar, volume_l, rows.get("amount"))
        {
            changed |= set_calculated_numeric(
                grid,
                component_id,
                row_id,
                concentration * volume * 1000.0,
                "mmol",
            );
        }
        if let (Some(amount), Some(concentration), Some(row_id)) =
            (amount_mol, concentration_molar, rows.get("volume"))
        {
            if concentration > 0.0 {
                changed |= set_calculated_numeric(
                    grid,
                    component_id,
                    row_id,
                    amount / concentration * 1000.0,
                    "mL",
                );
            }
        }
        if let (Some(amount), Some(volume), Some(row_id)) =
            (amount_mol, volume_l, rows.get("concentration"))
        {
            if volume > 0.0 {
                changed |= set_calculated_numeric(grid, component_id, row_id, amount / volume, "M");
            }
        }
        if let (Some(density), Some(volume), Some(row_id)) =
            (density_g_ml, volume_l, rows.get("mass"))
        {
            changed |= set_calculated_numeric(
                grid,
                component_id,
                row_id,
                density * volume * 1000.0 * 1000.0,
                "mg",
            );
        }
        if let (Some(mass), Some(density), Some(row_id)) =
            (mass_g, density_g_ml, rows.get("volume"))
        {
            if density > 0.0 {
                changed |= set_calculated_numeric(grid, component_id, row_id, mass / density, "mL");
            }
        }
        if let (Some(mass), Some(volume), Some(row_id)) = (mass_g, volume_l, rows.get("density")) {
            if volume > 0.0 {
                changed |= set_calculated_numeric(
                    grid,
                    component_id,
                    row_id,
                    mass / (volume * 1000.0),
                    "g/mL",
                );
            }
        }
    }
    changed |= mark_stoichiometry_inconsistencies(grid, component_id, rows);
    changed
}

fn base_value(
    grid: &crate::StoichiometryGridData,
    component_id: &str,
    rows: &BTreeMap<&str, String>,
    property: &str,
) -> Option<f64> {
    let row_id = rows.get(property)?;
    let (value, unit) = datum_numeric_value(grid, component_id, row_id)?;
    Some(match property {
        "mass" => mass_to_grams(value, unit.as_deref()),
        "amount" => amount_to_moles(value, unit.as_deref()),
        "concentration" => match unit.as_deref().unwrap_or("M") {
            "mM" => value / 1000.0,
            _ => value,
        },
        "volume" => match unit.as_deref().unwrap_or("L") {
            "mL" => value / 1000.0,
            "µL" | "μL" | "uL" => value / 1_000_000.0,
            _ => value,
        },
        _ => value,
    })
}

fn mark_stoichiometry_inconsistencies(
    grid: &mut crate::StoichiometryGridData,
    component_id: &str,
    rows: &BTreeMap<&str, String>,
) -> bool {
    let mut inconsistent = BTreeSet::new();
    let mw = base_value(grid, component_id, rows, "molecularweight");
    let mass = base_value(grid, component_id, rows, "mass");
    let amount = base_value(grid, component_id, rows, "amount");
    if let (Some(mw), Some(mass), Some(amount)) = (mw, mass, amount) {
        if !approximately_equal(mass, amount * mw) {
            inconsistent.extend(["mass", "amount"]);
        }
    }
    let concentration = base_value(grid, component_id, rows, "concentration");
    let volume = base_value(grid, component_id, rows, "volume");
    if let (Some(amount), Some(concentration), Some(volume)) = (amount, concentration, volume) {
        if !approximately_equal(amount, concentration * volume) {
            inconsistent.extend(["amount", "concentration", "volume"]);
        }
    }
    let density = base_value(grid, component_id, rows, "density");
    if let (Some(mass), Some(density), Some(volume)) = (mass, density, volume) {
        if !approximately_equal(mass, density * volume * 1000.0) {
            inconsistent.extend(["mass", "density", "volume"]);
        }
    }
    let mut changed = false;
    for property in inconsistent {
        let Some(row_id) = rows.get(property) else {
            continue;
        };
        let Some(datum) = grid.data.iter_mut().find(|datum| {
            datum.component_id == component_id
                && datum.row_id == *row_id
                && datum.origin == crate::StoichiometryValueOrigin::Authored
        }) else {
            continue;
        };
        changed |= datum.calculation_state != crate::StoichiometryCalculationState::Inconsistent;
        datum.calculation_state = crate::StoichiometryCalculationState::Inconsistent;
    }
    changed
}

fn approximately_equal(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1e-12);
    (left - right).abs() / scale <= 1e-4
}

fn set_calculated_numeric(
    grid: &mut crate::StoichiometryGridData,
    component_id: &str,
    row_id: &str,
    value: f64,
    unit: &str,
) -> bool {
    if !value.is_finite() {
        return false;
    }
    let Some(datum) = grid
        .data
        .iter_mut()
        .find(|datum| datum.component_id == component_id && datum.row_id == row_id)
    else {
        return false;
    };
    if datum.origin == crate::StoichiometryValueOrigin::Authored {
        return false;
    }
    let display = format!("{value:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    let next = crate::StoichiometryValue {
        canonical: value.to_string(),
        display,
        unit: Some(unit.to_string()),
    };
    let changed = datum.value != next
        || datum.origin != crate::StoichiometryValueOrigin::Calculated
        || datum.calculation_state != crate::StoichiometryCalculationState::Current;
    datum.value = next;
    datum.origin = crate::StoichiometryValueOrigin::Calculated;
    datum.is_edited = false;
    datum.calculation_state = crate::StoichiometryCalculationState::Current;
    changed
}

fn mass_to_grams(value: f64, unit: Option<&str>) -> f64 {
    match unit.unwrap_or("g") {
        "mg" => value / 1000.0,
        "µg" | "μg" | "ug" => value / 1_000_000.0,
        _ => value,
    }
}

fn amount_to_moles(value: f64, unit: Option<&str>) -> f64 {
    match unit.unwrap_or("mol") {
        "mmol" => value / 1000.0,
        "µmol" | "μmol" | "umol" => value / 1_000_000.0,
        _ => value,
    }
}

fn scene_object_bbox(object: &SceneObject) -> Option<[f64; 4]> {
    let [x, y, width, height] = object.payload.bbox?;
    Some([
        object.transform.translate[0] + x,
        object.transform.translate[1] + y,
        object.transform.translate[0] + x + width,
        object.transform.translate[1] + y + height,
    ])
}

fn union_bounds(left: [f64; 4], right: [f64; 4]) -> [f64; 4] {
    [
        left[0].min(right[0]),
        left[1].min(right[1]),
        left[2].max(right[2]),
        left[3].max(right[3]),
    ]
}

fn resize_stoichiometry_bbox(object: &mut SceneObject) {
    let Some(grid) = object.payload.stoichiometry_grid.as_ref() else {
        return;
    };
    let width = grid
        .components
        .iter()
        .filter(|component| component.visible)
        .map(|component| component.width)
        .sum::<f64>();
    let height = STOICHIOMETRY_HEADER_HEIGHT
        + grid
            .rows
            .iter()
            .filter(|row| row.visible)
            .map(|row| row.height)
            .sum::<f64>();
    object.payload.bbox = Some([0.0, 0.0, round2(width), round2(height)]);
}
