use super::select::object_selection_bounds_for_render;
use super::text_edit::make_text_object;
use super::{CommandTargetSet, EditorCommand, Engine, TextEditSession, TextEditTarget};
use crate::{
    ChemicalProperty, ChemicalPropertyCalculationState, ChemicalPropertyType,
    ChemicalPropertyValueOrigin, LabelRun, LinkEndpoint, LinkPolicy, LinkRelation, SelectionState,
};
use chemsema_chemical_graph::NomenclatureRequestV1;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const PROPERTY_GAP: f64 = 9.0;
const FONT_SIZE: f64 = 10.0;
const LINE_HEIGHT: f64 = 12.0;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChemicalPropertyDialogSubmission {
    #[serde(default)]
    property_id: Option<String>,
    #[serde(default)]
    type_code: Option<u32>,
    #[serde(default)]
    type_name: Option<String>,
    #[serde(default)]
    value: String,
    #[serde(default)]
    is_active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChemicalPropertyResult {
    property_id: String,
    value: String,
}

impl Engine {
    pub(super) fn selection_has_chemical_property(&self) -> bool {
        self.selected_chemical_property().is_some()
    }

    pub fn chemical_property_dialog_json(&self) -> String {
        let existing = self.selected_chemical_property();
        let can_create = existing.is_none() && self.selected_single_molecule_fragment().is_ok();
        if existing.is_none() && !can_create {
            return "null".to_string();
        }
        let property_id = existing.map(|property| property.id.clone());
        let property_type = existing
            .map(|property| property.property_type.clone())
            .unwrap_or_else(ChemicalPropertyType::chemical_name);
        let value = existing
            .and_then(|property| property.display_object_id.as_deref())
            .and_then(|id| self.state.document.find_scene_object(id))
            .and_then(|object| object.payload.extra.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        json!({
            "kind": "chemical-property",
            "title": if existing.is_some() { "Chemical Property" } else { "Add Chemical Property" },
            "propertyId": property_id,
            "fields": [
                {
                    "key": "typeCode",
                    "label": "Type code",
                    "value": property_type.code,
                    "inputMode": "numeric",
                    "minimum": 0,
                    "maximum": u32::MAX,
                    "allowEmpty": true
                },
                {
                    "key": "typeName",
                    "label": "Type name",
                    "value": property_type.name.unwrap_or_default(),
                    "inputMode": "text",
                    "allowEmpty": true
                },
                {
                    "key": "value",
                    "label": "Display value",
                    "value": value,
                    "inputMode": "text",
                    "allowEmpty": true
                },
                {
                    "key": "isActive",
                    "label": "Update when the structure changes",
                    "value": existing.is_some_and(|property| property.is_active),
                    "inputMode": "checkbox"
                }
            ],
            "canDelete": existing.is_some(),
            "typeHelp": "Undefined: both fields empty. Unspecified: code 0 / Unspecified. Chemical name: code 1 / ChemicalName. Custom CDX codes should be greater than 0x8000."
        })
        .to_string()
    }

    pub fn apply_chemical_property_dialog_json(
        &mut self,
        payload_json: &str,
    ) -> Result<bool, String> {
        let payload: ChemicalPropertyDialogSubmission =
            serde_json::from_str(payload_json).map_err(|error| error.to_string())?;
        let property_type = normalized_property_type(payload.type_code, payload.type_name)?;
        let property_id = payload.property_id.clone();
        Ok(self.with_command(
            EditorCommand::ApplyChemicalProperty {
                property_id,
                property_type: property_type.clone(),
                value: payload.value.clone(),
                is_active: payload.is_active,
            },
            |engine| {
                engine.apply_chemical_property_untracked(
                    payload.property_id.as_deref(),
                    property_type,
                    &payload.value,
                    payload.is_active,
                )
            },
        ))
    }

    pub fn delete_selected_chemical_property(&mut self) -> bool {
        let Some(property_id) = self
            .selected_chemical_property()
            .map(|property| property.id.clone())
        else {
            return false;
        };
        self.with_command(
            EditorCommand::DeleteChemicalProperty {
                property_id: property_id.clone(),
            },
            |engine| engine.delete_chemical_property_untracked(&property_id, true),
        )
    }

    pub fn chemical_property_requests_json(&self) -> Result<String, String> {
        let mut requests = Vec::new();
        for property in &self.state.document.chemical_properties {
            if !property.is_active
                || property.calculation_state != ChemicalPropertyCalculationState::Stale
                || !property.property_type.is_chemical_name()
            {
                continue;
            }
            let molecule_id = self
                .chemical_property_basis_molecule_id(property)
                .ok_or_else(|| {
                    format!(
                        "chemical property '{}' does not resolve to exactly one complete molecule",
                        property.id
                    )
                })?;
            let targets = CommandTargetSet {
                objects: vec![molecule_id.clone()],
                ..CommandTargetSet::default()
            };
            let (_, graph, _) = self.chemical_graph_v2_for_targets(&targets)?;
            let request =
                NomenclatureRequestV1::new_preferred_iupac_name(&molecule_id, graph.normalized()?)?;
            requests.push(json!({
                "propertyId": property.id,
                "request": request,
            }));
        }
        serde_json::to_string(&requests).map_err(|error| error.to_string())
    }

    pub fn apply_chemical_property_result_json(
        &mut self,
        payload_json: &str,
    ) -> Result<bool, String> {
        let payload: ChemicalPropertyResult =
            serde_json::from_str(payload_json).map_err(|error| error.to_string())?;
        if payload.value.trim().is_empty() {
            return Err("chemical property result value cannot be empty".to_string());
        }
        Ok(self.with_command(
            EditorCommand::ApplyChemicalPropertyResult {
                property_id: payload.property_id.clone(),
                value: payload.value.clone(),
            },
            |engine| {
                engine
                    .apply_chemical_property_result_untracked(&payload.property_id, &payload.value)
            },
        ))
    }

    pub(super) fn chemical_property_basis_fingerprints(&self) -> BTreeMap<String, String> {
        let mut fingerprints = BTreeMap::new();
        for property in &self.state.document.chemical_properties {
            if !property.is_active {
                continue;
            }
            let Some(molecule_id) = self.chemical_property_basis_molecule_id(property) else {
                continue;
            };
            let targets = CommandTargetSet {
                objects: vec![molecule_id],
                ..CommandTargetSet::default()
            };
            let Ok((_, graph, _)) = self.chemical_graph_v2_for_targets(&targets) else {
                continue;
            };
            let Ok(normalized) = graph.normalized() else {
                continue;
            };
            if let Ok(value) = serde_json::to_string(&normalized) {
                fingerprints.insert(property.id.clone(), value);
            }
        }
        fingerprints
    }

    pub(super) fn mark_changed_chemical_properties_stale(
        &mut self,
        before: &BTreeMap<String, String>,
    ) {
        let after = self.chemical_property_basis_fingerprints();
        for property in &mut self.state.document.chemical_properties {
            if !property.is_active {
                continue;
            }
            if before.get(&property.id) == after.get(&property.id) {
                continue;
            }
            property.calculation_state = if property.property_type.is_chemical_name() {
                ChemicalPropertyCalculationState::Stale
            } else {
                ChemicalPropertyCalculationState::Unsupported
            };
        }
    }

    pub(super) fn reconcile_chemical_properties_after_document_change(&mut self) -> bool {
        let entity_ids = document_entity_ids(&self.state.document);
        let mut changed = false;
        let property_ids = self
            .state
            .document
            .chemical_properties
            .iter()
            .map(|property| property.id.clone())
            .collect::<Vec<_>>();
        for property_id in property_ids {
            let Some(index) = self
                .state
                .document
                .chemical_properties
                .iter()
                .position(|property| property.id == property_id)
            else {
                continue;
            };
            let property = &mut self.state.document.chemical_properties[index];
            let before = property.basis_entity_ids.len();
            property
                .basis_entity_ids
                .retain(|entity_id| entity_ids.contains(entity_id));
            changed |= before != property.basis_entity_ids.len();
            if property
                .display_object_id
                .as_ref()
                .is_some_and(|id| !entity_ids.contains(id))
            {
                property.display_object_id = None;
                changed = true;
            }
            if property.basis_entity_ids.is_empty() && property.unresolved_basis_ids.is_empty() {
                changed |= self.delete_chemical_property_untracked(&property_id, false);
            } else {
                changed |= self.sync_chemical_property_link(&property_id);
            }
        }
        changed
    }

    pub(super) fn detach_edited_chemical_property_display(&mut self, object_id: &str) -> bool {
        let Some(property) = self
            .state
            .document
            .chemical_properties
            .iter_mut()
            .find(|property| property.display_object_id.as_deref() == Some(object_id))
        else {
            return false;
        };
        if !property.is_active
            && property.value_origin == ChemicalPropertyValueOrigin::Authored
            && property.calculation_state == ChemicalPropertyCalculationState::Static
        {
            return false;
        }
        property.is_active = false;
        property.value_origin = ChemicalPropertyValueOrigin::Authored;
        property.calculation_state = ChemicalPropertyCalculationState::Static;
        property.last_calculated_value = None;
        self.pending_dialog = Some(json!({
            "kind": "notice",
            "title": "Automatic updating disabled",
            "message": "Editing this chemical-property display has disabled automatic updating."
        }));
        true
    }

    pub(super) fn apply_chemical_property_untracked(
        &mut self,
        property_id: Option<&str>,
        property_type: ChemicalPropertyType,
        value: &str,
        is_active: bool,
    ) -> bool {
        if let Some(property_id) = property_id {
            let Some(index) = self
                .state
                .document
                .chemical_properties
                .iter()
                .position(|property| property.id == property_id)
            else {
                return false;
            };
            self.push_undo_snapshot();
            let display_id = self.state.document.chemical_properties[index]
                .display_object_id
                .clone();
            let changed_text = display_id
                .as_deref()
                .is_some_and(|id| update_text_object(&mut self.state.document, id, value));
            let property = &mut self.state.document.chemical_properties[index];
            let next_state = calculation_state_for(is_active, &property_type);
            let changed = changed_text
                || property.property_type != property_type
                || property.is_active != is_active
                || property.value_origin != ChemicalPropertyValueOrigin::Authored
                || property.calculation_state != next_state;
            property.property_type = property_type;
            property.is_active = is_active;
            property.value_origin = ChemicalPropertyValueOrigin::Authored;
            property.calculation_state = next_state;
            property.last_calculated_value = is_active.then(|| value.to_string());
            let linked = self.sync_chemical_property_link(property_id);
            if !changed && !linked {
                self.undo_stack.pop();
                return false;
            }
            return true;
        }

        let Ok((source, _)) = self.selected_single_molecule_fragment() else {
            return false;
        };
        let source_id = source.id.clone();
        let Some(bounds) = self
            .state
            .document
            .find_scene_object(&source_id)
            .and_then(|object| object_selection_bounds_for_render(&self.state.document, object))
        else {
            return false;
        };
        self.push_undo_snapshot();
        let property_id = self.next_id("chemical_property");
        let display_id = self.next_id("obj_chemical_property");
        let width = estimated_text_width(value);
        let x = (bounds[0] + bounds[2]) * 0.5 - width * 0.5;
        let y = bounds[3] + PROPERTY_GAP;
        let run = default_run(value);
        let session = TextEditSession {
            target: TextEditTarget::TextObject {
                object_id: None,
                x,
                y,
            },
            text: value.to_string(),
            source_runs: vec![run.clone()],
            font_family: Some("Arial".to_string()),
            font_size: Some(FONT_SIZE),
            fill: Some("#000000".to_string()),
            align: Some("center".to_string()),
            line_height: Some(LINE_HEIGHT),
            box_value: Some([0.0, 0.0, width, LINE_HEIGHT]),
            anchor_offset: None,
            text_position: None,
            glyph_polygons: Vec::new(),
            preserve_lines: true,
            default_chemical: false,
            display_mode: None,
        };
        let z_index = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or_default()
            + 1;
        let mut display = make_text_object(
            &display_id,
            x,
            y,
            value,
            vec![run.clone()],
            vec![run],
            &session,
            width,
            LINE_HEIGHT,
            z_index,
        );
        display.name = "chemical-property-display".to_string();
        display.link_policy = LinkPolicy::Linked;
        display
            .payload
            .extra
            .insert("chemicalPropertyId".to_string(), json!(property_id));
        if let Some(source) = self.state.document.find_scene_object_mut(&source_id) {
            source.link_policy = LinkPolicy::Linked;
        }
        self.state.document.objects.push(display);
        self.state
            .document
            .chemical_properties
            .push(ChemicalProperty {
                id: property_id.clone(),
                source_id: None,
                property_type: property_type.clone(),
                basis_entity_ids: vec![source_id],
                unresolved_basis_ids: Vec::new(),
                display_object_id: Some(display_id.clone()),
                is_active,
                value_origin: ChemicalPropertyValueOrigin::Authored,
                calculation_state: calculation_state_for(is_active, &property_type),
                last_calculated_value: is_active.then(|| value.to_string()),
            });
        self.sync_chemical_property_link(&property_id);
        self.state.selection = SelectionState {
            text_objects: vec![display_id],
            ..SelectionState::default()
        };
        true
    }

    pub(super) fn apply_chemical_property_result_untracked(
        &mut self,
        property_id: &str,
        value: &str,
    ) -> bool {
        let Some(index) = self
            .state
            .document
            .chemical_properties
            .iter()
            .position(|property| property.id == property_id)
        else {
            return false;
        };
        if !self.state.document.chemical_properties[index].is_active
            || !self.state.document.chemical_properties[index]
                .property_type
                .is_chemical_name()
        {
            return false;
        }
        self.push_undo_snapshot();
        let display_id = self.state.document.chemical_properties[index]
            .display_object_id
            .clone();
        let changed_text = display_id
            .as_deref()
            .is_some_and(|id| update_text_object(&mut self.state.document, id, value));
        let property = &mut self.state.document.chemical_properties[index];
        let changed = changed_text
            || property.calculation_state != ChemicalPropertyCalculationState::Current
            || property.value_origin != ChemicalPropertyValueOrigin::Calculated
            || property.last_calculated_value.as_deref() != Some(value);
        property.calculation_state = ChemicalPropertyCalculationState::Current;
        property.value_origin = ChemicalPropertyValueOrigin::Calculated;
        property.last_calculated_value = Some(value.to_string());
        if !changed {
            self.undo_stack.pop();
        }
        changed
    }

    pub(super) fn delete_chemical_property_untracked(
        &mut self,
        property_id: &str,
        remove_display: bool,
    ) -> bool {
        let Some(index) = self
            .state
            .document
            .chemical_properties
            .iter()
            .position(|property| property.id == property_id)
        else {
            return false;
        };
        let property = self.state.document.chemical_properties.remove(index);
        self.state.document.links.retain(|relation| {
            !(relation.kind == "chemical-property-display"
                && relation
                    .data
                    .get("chemicalPropertyId")
                    .and_then(Value::as_str)
                    == Some(property_id))
        });
        if remove_display {
            if let Some(display_id) = property.display_object_id {
                let ids = BTreeSet::from([display_id.as_str()]);
                self.state.document.remove_scene_objects_by_id(&ids);
            }
        } else if let Some(display_id) = property.display_object_id {
            if let Some(display) = self.state.document.find_scene_object_mut(&display_id) {
                display.link_policy = LinkPolicy::Auto;
                display.payload.extra.remove("chemicalPropertyId");
            }
        }
        true
    }

    fn sync_chemical_property_link(&mut self, property_id: &str) -> bool {
        let Some(property) = self
            .state
            .document
            .chemical_properties
            .iter()
            .find(|property| property.id == property_id)
            .cloned()
        else {
            return false;
        };
        let before = self.state.document.links.clone();
        self.state.document.links.retain(|relation| {
            !(relation.kind == "chemical-property-display"
                && relation
                    .data
                    .get("chemicalPropertyId")
                    .and_then(Value::as_str)
                    == Some(property_id))
        });
        if let Some(display_id) = property.display_object_id {
            let mut endpoints = property
                .basis_entity_ids
                .iter()
                .map(|entity_id| LinkEndpoint {
                    entity_id: entity_id.clone(),
                    role: "basis".to_string(),
                })
                .collect::<Vec<_>>();
            endpoints.push(LinkEndpoint {
                entity_id: display_id,
                role: "display".to_string(),
            });
            self.state.document.links.push(LinkRelation {
                id: format!("link_{}", property.id),
                kind: "chemical-property-display".to_string(),
                endpoints,
                data: json!({
                    "chemicalPropertyId": property.id,
                    "inference": "declared"
                }),
            });
        }
        before != self.state.document.links
    }

    fn selected_chemical_property(&self) -> Option<&ChemicalProperty> {
        let selected = self
            .state
            .selection
            .text_objects
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        (selected.len() == 1).then_some(())?;
        self.state
            .document
            .chemical_properties
            .iter()
            .find(|property| {
                property
                    .display_object_id
                    .as_deref()
                    .is_some_and(|id| selected.contains(id))
            })
    }

    fn chemical_property_basis_molecule_id(&self, property: &ChemicalProperty) -> Option<String> {
        let mut molecule_ids = BTreeSet::new();
        for entity_id in &property.basis_entity_ids {
            if self
                .state
                .document
                .find_scene_object(entity_id)
                .is_some_and(|object| object.object_type == "molecule")
            {
                molecule_ids.insert(entity_id.clone());
                continue;
            }
            for entry in self.state.document.editable_fragments() {
                if entry
                    .fragment
                    .nodes
                    .iter()
                    .any(|node| node.id == *entity_id)
                    || entry
                        .fragment
                        .bonds
                        .iter()
                        .any(|bond| bond.id == *entity_id)
                {
                    molecule_ids.insert(entry.object.id.clone());
                }
            }
        }
        (molecule_ids.len() == 1).then(|| molecule_ids.into_iter().next().unwrap())
    }
}

fn normalized_property_type(
    code: Option<u32>,
    name: Option<String>,
) -> Result<ChemicalPropertyType, String> {
    let name = name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    match (code, name.as_deref()) {
        (None, None) => Ok(ChemicalPropertyType::undefined()),
        (Some(0), None | Some("Unspecified")) => Ok(ChemicalPropertyType::unspecified()),
        (Some(1), None | Some("ChemicalName")) => Ok(ChemicalPropertyType::chemical_name()),
        (None, Some("Unspecified")) => Ok(ChemicalPropertyType::unspecified()),
        (None, Some("ChemicalName")) => Ok(ChemicalPropertyType::chemical_name()),
        (Some(0), Some(_)) => Err("type code 0 must be named Unspecified".to_string()),
        (Some(1), Some(_)) => Err("type code 1 must be named ChemicalName".to_string()),
        (Some(code), _name) if code <= 0x8000 => Err(format!(
            "custom chemical property code {code} must be greater than 0x8000"
        )),
        (Some(code), name) => Ok(ChemicalPropertyType {
            code: Some(code),
            name: name.map(ToString::to_string),
        }),
        (None, Some(name)) => Ok(ChemicalPropertyType {
            code: None,
            name: Some(name.to_string()),
        }),
    }
}

fn calculation_state_for(
    is_active: bool,
    property_type: &ChemicalPropertyType,
) -> ChemicalPropertyCalculationState {
    if !is_active {
        ChemicalPropertyCalculationState::Static
    } else if property_type.is_chemical_name() {
        ChemicalPropertyCalculationState::Current
    } else {
        ChemicalPropertyCalculationState::Unsupported
    }
}

fn update_text_object(
    document: &mut crate::ChemSemaDocument,
    object_id: &str,
    value: &str,
) -> bool {
    let Some(object) = document.find_scene_object_mut(object_id) else {
        return false;
    };
    let old = object
        .payload
        .extra
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("");
    if old == value {
        return false;
    }
    let run = default_run(value);
    let width = estimated_text_width(value);
    object
        .payload
        .extra
        .insert("text".to_string(), json!(value));
    object
        .payload
        .extra
        .insert("runs".to_string(), json!([run.clone()]));
    object
        .payload
        .extra
        .insert("sourceRuns".to_string(), json!([run]));
    object
        .payload
        .extra
        .insert("box".to_string(), json!([0.0, 0.0, width, LINE_HEIGHT]));
    object.payload.bbox = Some([0.0, 0.0, width, LINE_HEIGHT]);
    true
}

fn default_run(value: &str) -> LabelRun {
    LabelRun {
        text: value.to_string(),
        font_family: Some("Arial".to_string()),
        font_size: Some(FONT_SIZE),
        fill: Some("#000000".to_string()),
        ..LabelRun::default()
    }
}

fn estimated_text_width(value: &str) -> f64 {
    crate::round2((value.chars().count() as f64 * FONT_SIZE * 0.56).max(24.0))
}

fn document_entity_ids(document: &crate::ChemSemaDocument) -> BTreeSet<String> {
    document
        .scene_objects()
        .into_iter()
        .map(|object| object.id.clone())
        .chain(document.editable_fragments().into_iter().flat_map(|entry| {
            entry
                .fragment
                .nodes
                .iter()
                .map(|node| node.id.clone())
                .chain(entry.fragment.bonds.iter().map(|bond| bond.id.clone()))
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_known_and_custom_type_pairs_without_guessing() {
        assert_eq!(
            normalized_property_type(Some(1), Some("ChemicalName".to_string())).unwrap(),
            ChemicalPropertyType::chemical_name()
        );
        assert!(normalized_property_type(Some(1), Some("Formula".to_string())).is_err());
        assert!(normalized_property_type(Some(0x8000), Some("Vendor".to_string())).is_err());
        assert_eq!(
            normalized_property_type(Some(0x8001), Some("org.example.LogP".to_string())).unwrap(),
            ChemicalPropertyType {
                code: Some(0x8001),
                name: Some("org.example.LogP".to_string())
            }
        );
    }
}
