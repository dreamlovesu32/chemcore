use super::{EditorCommand, Engine};
use crate::{
    refresh_repeating_units, ChemSemaDocument, LinkEndpoint, LinkPolicy, LinkRelation, Point,
    SceneObject, SelectionState,
};
use serde_json::Value;
use std::collections::BTreeSet;

const IMPORT_COUNT_LABEL_SEARCH_PAD: f64 = 50.0;
const BOUNDS_EPSILON: f64 = 1e-6;

impl Engine {
    pub(super) fn reconcile_links_after_document_change(&mut self) -> bool {
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
        let invalid_analysis_endpoints = self
            .state
            .document
            .links
            .iter()
            .filter(|relation| relation.kind == "analysis-caption")
            .filter(|relation| {
                relation.endpoints.iter().any(|endpoint| {
                    !scene_ids.contains(&endpoint.entity_id)
                        && !node_ids.contains(&endpoint.entity_id)
                        && !bond_ids.contains(&endpoint.entity_id)
                })
            })
            .flat_map(|relation| relation.endpoints.iter())
            .filter(|endpoint| scene_ids.contains(&endpoint.entity_id))
            .map(|endpoint| (endpoint.entity_id.clone(), endpoint.role.clone()))
            .collect::<Vec<_>>();
        for (endpoint_id, role) in invalid_analysis_endpoints {
            if let Some(object) = self.state.document.find_scene_object_mut(&endpoint_id) {
                object.link_policy = if role == "caption" {
                    LinkPolicy::Unlinked
                } else {
                    LinkPolicy::Auto
                };
            }
        }
        let before = self.state.document.links.len();
        self.state.document.links.retain(|relation| {
            relation.endpoints.iter().all(|endpoint| {
                scene_ids.contains(&endpoint.entity_id)
                    || node_ids.contains(&endpoint.entity_id)
                    || bond_ids.contains(&endpoint.entity_id)
            })
        });
        let mut changed = before != self.state.document.links.len();

        let automatic_pairs = imported_repeat_unit_label_pairs(&self.state.document)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let before = self.state.document.links.len();
        let explicit_ids = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| object.link_policy == LinkPolicy::Linked)
            .map(|object| object.id.clone())
            .collect::<BTreeSet<_>>();
        self.state.document.links.retain(|relation| {
            if relation.kind != "bracket-repeat-label" {
                return true;
            }
            let bracket = relation
                .endpoints
                .iter()
                .find(|endpoint| endpoint.role == "bracket")
                .map(|endpoint| endpoint.entity_id.as_str());
            let label = relation
                .endpoints
                .iter()
                .find(|endpoint| endpoint.role == "label")
                .map(|endpoint| endpoint.entity_id.as_str());
            let explicit = bracket.is_some_and(|id| explicit_ids.contains(id))
                || label.is_some_and(|id| explicit_ids.contains(id));
            let declared =
                relation.data.get("inference").and_then(Value::as_str) == Some("declared");
            explicit
                || declared
                || bracket.zip(label).is_some_and(|(bracket, label)| {
                    automatic_pairs.contains(&(bracket.to_string(), label.to_string()))
                })
        });
        changed |= before != self.state.document.links.len();
        changed |= self.link_imported_repeat_unit_labels_untracked();
        changed |= refresh_repeating_units(&mut self.state.document);
        changed |= crate::refresh_attached_electron_symbols(&mut self.state.document);
        changed |= self.refresh_analysis_captions();
        changed |= self.reconcile_chemical_properties_after_document_change();
        changed |= self.refresh_stoichiometry_after_document_change();
        changed
    }

    pub fn selection_can_link(&self) -> bool {
        self.selection_can_link_stoichiometry()
            || compatible_selection_link(&self.state.document, &self.state.selection)
                .is_some_and(|candidate| !relation_exists(&self.state.document, &candidate))
    }

    pub fn selection_can_link_bracket_text(&self) -> bool {
        compatible_selection_link(&self.state.document, &self.state.selection)
            .is_some_and(|relation| relation.kind == "bracket-repeat-label")
            && self.selection_can_link()
    }

    pub fn selection_can_unlink(&self) -> bool {
        let ids = selected_entity_ids(&self.state.selection);
        !ids.is_empty()
            && (self.state.document.links.iter().any(|relation| {
                relation
                    .endpoints
                    .iter()
                    .any(|ep| ids.contains(&ep.entity_id))
            }) || ids.iter().any(|id| {
                self.state
                    .document
                    .find_scene_object(id)
                    .is_some_and(|object| object.link_policy != LinkPolicy::Unlinked)
            }))
    }

    pub fn selection_can_unlink_bracket_text(&self) -> bool {
        let ids = selected_entity_ids(&self.state.selection);
        self.state.document.links.iter().any(|relation| {
            relation.kind == "bracket-repeat-label"
                && relation
                    .endpoints
                    .iter()
                    .all(|endpoint| ids.contains(&endpoint.entity_id))
        })
    }

    pub fn link_selection(&mut self) -> bool {
        let object_ids = selected_entity_ids(&self.state.selection)
            .into_iter()
            .collect();
        self.with_command(EditorCommand::LinkSelection { object_ids }, |engine| {
            engine.link_selection_untracked()
        })
    }

    fn link_selection_untracked(&mut self) -> bool {
        if self.selection_can_link_stoichiometry() {
            self.push_undo_snapshot();
            let changed = self.link_stoichiometry_selection_untracked();
            if !changed {
                self.undo_stack.pop();
            }
            return changed;
        }
        let Some(mut relation) =
            compatible_selection_link(&self.state.document, &self.state.selection)
        else {
            return false;
        };
        if relation_exists(&self.state.document, &relation) {
            return false;
        }
        self.push_undo_snapshot();
        relation.id = self.next_id("link");
        for endpoint in &relation.endpoints {
            if let Some(object) = self
                .state
                .document
                .find_scene_object_mut(&endpoint.entity_id)
            {
                object.link_policy = LinkPolicy::Linked;
            }
        }
        self.state.document.links.push(relation);
        refresh_repeating_units(&mut self.state.document);
        true
    }

    pub fn unlink_selection(&mut self) -> bool {
        let object_ids = selected_entity_ids(&self.state.selection)
            .into_iter()
            .collect();
        self.with_command(EditorCommand::UnlinkSelection { object_ids }, |engine| {
            engine.unlink_selection_untracked()
        })
    }

    fn unlink_selection_untracked(&mut self) -> bool {
        self.set_link_policy_for_selection_untracked(LinkPolicy::Unlinked)
    }

    pub fn set_link_policy_for_selection(&mut self, policy: LinkPolicy) -> bool {
        let object_ids = selected_entity_ids(&self.state.selection)
            .into_iter()
            .collect();
        self.with_command(
            EditorCommand::SetLinkPolicy { object_ids, policy },
            |engine| engine.set_link_policy_for_selection_untracked(policy),
        )
    }

    fn set_link_policy_for_selection_untracked(&mut self, policy: LinkPolicy) -> bool {
        let ids = selected_entity_ids(&self.state.selection);
        if ids.is_empty() {
            return false;
        }
        self.push_undo_snapshot();
        let mut changed = false;
        let selected_grid_ids = ids
            .iter()
            .filter(|id| {
                self.state
                    .document
                    .find_scene_object(id)
                    .is_some_and(|object| object.object_type == "stoichiometry-grid")
            })
            .cloned()
            .collect::<Vec<_>>();
        for grid_id in selected_grid_ids {
            changed |= self.bind_stoichiometry_grid_untracked(&grid_id, None, policy);
        }
        if policy == LinkPolicy::Unlinked {
            let detached_property_ids = self
                .state
                .document
                .chemical_properties
                .iter()
                .filter(|property| {
                    property
                        .display_object_id
                        .as_ref()
                        .is_some_and(|display_id| ids.contains(display_id))
                })
                .map(|property| property.id.clone())
                .collect::<Vec<_>>();
            for property_id in detached_property_ids {
                let display_id = self
                    .state
                    .document
                    .chemical_properties
                    .iter_mut()
                    .find(|property| property.id == property_id)
                    .and_then(|property| {
                        property.is_active = false;
                        property.calculation_state =
                            crate::ChemicalPropertyCalculationState::Static;
                        property.display_object_id.take()
                    });
                if let Some(display_id) = display_id {
                    if let Some(display) = self.state.document.find_scene_object_mut(&display_id) {
                        display.payload.extra.remove("chemicalPropertyId");
                    }
                    changed = true;
                }
            }
        }
        for id in &ids {
            if let Some(object) = self.state.document.find_scene_object_mut(id) {
                if object.link_policy != policy {
                    object.link_policy = policy;
                    changed = true;
                }
            }
        }
        let before = self.state.document.links.len();
        let other_endpoints = self
            .state
            .document
            .links
            .iter()
            .filter(|relation| {
                relation
                    .endpoints
                    .iter()
                    .any(|endpoint| ids.contains(&endpoint.entity_id))
            })
            .flat_map(|relation| relation.endpoints.iter())
            .filter(|endpoint| !ids.contains(&endpoint.entity_id))
            .map(|endpoint| endpoint.entity_id.clone())
            .collect::<Vec<_>>();
        self.state.document.links.retain(|relation| {
            !relation
                .endpoints
                .iter()
                .any(|endpoint| ids.contains(&endpoint.entity_id))
        });
        changed |= before != self.state.document.links.len();
        for endpoint_id in other_endpoints {
            if let Some(object) = self.state.document.find_scene_object_mut(&endpoint_id) {
                if object.link_policy == LinkPolicy::Linked {
                    object.link_policy = LinkPolicy::Auto;
                    changed = true;
                }
            }
        }
        if policy == LinkPolicy::Auto {
            if let Some(mut relation) =
                compatible_selection_link(&self.state.document, &self.state.selection)
            {
                if !relation_exists(&self.state.document, &relation) {
                    relation.id = self.next_id("link");
                    self.state.document.links.push(relation);
                    changed = true;
                }
            }
            changed |= self.link_imported_repeat_unit_labels_untracked();
        }
        changed |= refresh_repeating_units(&mut self.state.document);
        if !changed {
            self.undo_stack.pop();
        }
        changed
    }

    pub(super) fn link_bracket_text_objects_untracked(
        &mut self,
        bracket_id: &str,
        text_id: &str,
    ) -> bool {
        let candidate = bracket_repeat_relation(bracket_id, text_id);
        if relation_exists(&self.state.document, &candidate) {
            return false;
        }
        let mut relation = candidate;
        relation.id = self.next_id("link");
        relation.data = serde_json::json!({"inference": "declared"});
        self.state.document.links.push(relation);
        refresh_repeating_units(&mut self.state.document);
        true
    }

    pub(super) fn link_imported_repeat_unit_labels_untracked(&mut self) -> bool {
        let pairs = imported_repeat_unit_label_pairs(&self.state.document);
        let mut changed = false;
        for (bracket_id, text_id) in pairs {
            let bracket_policy = self
                .state
                .document
                .find_scene_object(&bracket_id)
                .map(|object| object.link_policy);
            let text_policy = self
                .state
                .document
                .find_scene_object(&text_id)
                .map(|object| object.link_policy);
            if matches!(
                bracket_policy,
                Some(LinkPolicy::Unlinked | LinkPolicy::Linked)
            ) || matches!(text_policy, Some(LinkPolicy::Unlinked | LinkPolicy::Linked))
            {
                continue;
            }
            if relation_exists(
                &self.state.document,
                &bracket_repeat_relation(&bracket_id, &text_id),
            ) {
                continue;
            }
            let mut relation = bracket_repeat_relation(&bracket_id, &text_id);
            relation.id = self.next_id("link");
            relation.data = serde_json::json!({"inference": "geometry"});
            self.state.document.links.push(relation);
            changed = true;
        }
        changed
    }
}

fn selected_entity_ids(selection: &SelectionState) -> BTreeSet<String> {
    selection
        .arrow_objects
        .iter()
        .chain(selection.text_objects.iter())
        .chain(selection.molecule_objects.iter())
        .chain(selection.nodes.iter())
        .cloned()
        .collect()
}

fn compatible_selection_link(
    document: &ChemSemaDocument,
    selection: &SelectionState,
) -> Option<LinkRelation> {
    let ids = selected_entity_ids(selection);
    if ids.len() != 2 {
        return None;
    }
    let objects = ids
        .iter()
        .filter_map(|id| document.find_scene_object(id).map(|object| (id, object)))
        .collect::<Vec<_>>();
    if objects.len() == 2 {
        let bracket = objects
            .iter()
            .find(|(_, object)| scene_object_is_bracket_like(object));
        let text = objects
            .iter()
            .find(|(_, object)| object.object_type == "text");
        if let (Some((bracket_id, _)), Some((text_id, text))) = (bracket, text) {
            let is_repeat_label = text
                .payload
                .extra
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|value| value.trim().parse::<u32>().is_ok_and(|count| count >= 2));
            if is_repeat_label {
                return Some(bracket_repeat_relation(bracket_id, text_id));
            }
        }
        let molecule = objects
            .iter()
            .find(|(_, object)| object.object_type == "molecule");
        let caption = objects.iter().find(|(_, object)| {
            object.object_type == "text" && object.payload.extra.contains_key("analysisCaption")
        });
        if let (Some((molecule_id, _)), Some((caption_id, _))) = (molecule, caption) {
            return Some(LinkRelation {
                id: String::new(),
                kind: "analysis-caption".to_string(),
                endpoints: vec![
                    LinkEndpoint {
                        entity_id: (*molecule_id).clone(),
                        role: "source".to_string(),
                    },
                    LinkEndpoint {
                        entity_id: (*caption_id).clone(),
                        role: "caption".to_string(),
                    },
                ],
                data: Value::Null,
            });
        }
    }
    let symbol = objects
        .iter()
        .find(|(_, object)| object.object_type == "symbol");
    let atom = ids
        .iter()
        .find(|id| document.find_scene_object(id).is_none());
    if let (Some((symbol_id, symbol)), Some(atom_id)) = (symbol, atom) {
        if symbol.payload.extra.contains_key("chemicalRole") {
            return Some(LinkRelation {
                id: String::new(),
                kind: "atom-symbol".to_string(),
                endpoints: vec![
                    LinkEndpoint {
                        entity_id: atom_id.clone(),
                        role: "atom".to_string(),
                    },
                    LinkEndpoint {
                        entity_id: (*symbol_id).clone(),
                        role: "symbol".to_string(),
                    },
                ],
                data: Value::Null,
            });
        }
    }
    None
}

fn bracket_repeat_relation(bracket_id: &str, text_id: &str) -> LinkRelation {
    LinkRelation {
        id: String::new(),
        kind: "bracket-repeat-label".to_string(),
        endpoints: vec![
            LinkEndpoint {
                entity_id: bracket_id.to_string(),
                role: "bracket".to_string(),
            },
            LinkEndpoint {
                entity_id: text_id.to_string(),
                role: "label".to_string(),
            },
        ],
        data: Value::Null,
    }
}

fn relation_exists(document: &ChemSemaDocument, candidate: &LinkRelation) -> bool {
    let candidate_endpoints = candidate
        .endpoints
        .iter()
        .map(|endpoint| (&endpoint.entity_id, &endpoint.role))
        .collect::<BTreeSet<_>>();
    document.links.iter().any(|relation| {
        relation.kind == candidate.kind
            && relation
                .endpoints
                .iter()
                .map(|endpoint| (&endpoint.entity_id, &endpoint.role))
                .collect::<BTreeSet<_>>()
                == candidate_endpoints
    })
}

pub(super) fn scene_object_is_bracket_like(object: &SceneObject) -> bool {
    object.object_type == "bracket"
        || (object.object_type == "group"
            && object.meta.get("kind").and_then(Value::as_str) == Some("bracket-group"))
}

fn imported_repeat_unit_label_pairs(document: &ChemSemaDocument) -> Vec<(String, String)> {
    let brackets = imported_bracket_candidates(document);
    let counts = imported_count_candidates(document);
    let mut used_text_ids = BTreeSet::new();
    let mut pairs = Vec::new();

    for bracket in brackets {
        let Some(count) = best_imported_count_for_bracket(&bracket, &counts, &used_text_ids) else {
            continue;
        };
        used_text_ids.insert(count.object_id.clone());
        pairs.push((bracket.object_id, count.object_id.clone()));
    }

    pairs
}

#[derive(Debug, Clone)]
struct ImportedBracketCandidate {
    object_id: String,
    bounds: [f64; 4],
}

#[derive(Debug, Clone)]
struct ImportedCountCandidate {
    object_id: String,
    bounds: [f64; 4],
}

fn imported_bracket_candidates(document: &ChemSemaDocument) -> Vec<ImportedBracketCandidate> {
    document
        .scene_objects()
        .into_iter()
        .filter(|object| scene_object_is_bracket_like(object) && object.visible)
        .filter_map(|object| {
            Some(ImportedBracketCandidate {
                object_id: object.id.clone(),
                bounds: object_world_bounds(object)?,
            })
        })
        .collect()
}

fn imported_count_candidates(document: &ChemSemaDocument) -> Vec<ImportedCountCandidate> {
    document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text" && object.visible)
        .filter_map(|object| {
            let text = payload_string(object, "text")?;
            let trimmed = text.trim();
            if trimmed.is_empty() || !trimmed.chars().all(|character| character.is_ascii_digit()) {
                return None;
            }
            let value = trimmed.parse::<u32>().ok()?;
            if value < 2 {
                return None;
            }
            Some(ImportedCountCandidate {
                object_id: object.id.clone(),
                bounds: object_world_bounds(object)?,
            })
        })
        .collect()
}

fn best_imported_count_for_bracket<'a>(
    bracket: &ImportedBracketCandidate,
    counts: &'a [ImportedCountCandidate],
    used_text_ids: &BTreeSet<String>,
) -> Option<&'a ImportedCountCandidate> {
    let [left, top, right, bottom] = bracket.bounds;
    let min_x = right - ((right - left).abs() * 0.35).max(16.0);
    let max_x = right + IMPORT_COUNT_LABEL_SEARCH_PAD;
    let min_y = bottom - ((bottom - top).abs() * 0.35).max(16.0);
    let max_y = bottom + IMPORT_COUNT_LABEL_SEARCH_PAD;
    let anchor = Point::new(right, bottom);
    counts
        .iter()
        .filter(|count| !used_text_ids.contains(&count.object_id))
        .filter(|count| {
            let center = bounds_center(count.bounds);
            center.x >= min_x && center.x <= max_x && center.y >= min_y && center.y <= max_y
        })
        .min_by(|left_count, right_count| {
            bounds_center(left_count.bounds)
                .distance(anchor)
                .total_cmp(&bounds_center(right_count.bounds).distance(anchor))
        })
}

fn object_world_bounds(object: &SceneObject) -> Option<[f64; 4]> {
    let [x, y, width, height] = object.payload.bbox.or_else(|| payload_box(object))?;
    if width <= BOUNDS_EPSILON || height <= BOUNDS_EPSILON {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    Some([tx + x, ty + y, tx + x + width, ty + y + height])
}

fn payload_box(object: &SceneObject) -> Option<[f64; 4]> {
    let values = object.payload.extra.get("box")?.as_array()?;
    if values.len() != 4 {
        return None;
    }
    Some([
        values[0].as_f64()?,
        values[1].as_f64()?,
        values[2].as_f64()?,
        values[3].as_f64()?,
    ])
}

fn payload_string(object: &SceneObject, key: &str) -> Option<String> {
    object
        .payload
        .extra
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn bounds_center(bounds: [f64; 4]) -> Point {
    Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LabelRun, Node, ObjectPayload, ResourceData, Transform};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn text_object(id: &str, text: &str, x: f64, y: f64) -> SceneObject {
        SceneObject {
            id: id.to_string(),
            object_type: "text".to_string(),
            name: "text".to_string(),
            visible: true,
            locked: false,
            z_index: 20,
            transform: Transform {
                translate: [x, y],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: LinkPolicy::Auto,
            meta: Value::Null,
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, 8.0, 12.0]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                extra: BTreeMap::from([
                    ("text".to_string(), json!(text)),
                    ("box".to_string(), json!([0.0, 0.0, 8.0, 12.0])),
                ]),
            },
            children: Vec::new(),
        }
    }

    fn bracket_object(id: &str) -> SceneObject {
        SceneObject {
            id: id.to_string(),
            object_type: "bracket".to_string(),
            name: "bracket".to_string(),
            visible: true,
            locked: false,
            z_index: 10,
            transform: Transform::identity(),
            style_ref: None,
            link_policy: LinkPolicy::Auto,
            meta: Value::Null,
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, 40.0, 40.0]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                extra: BTreeMap::from([
                    ("kind".to_string(), json!("square")),
                    ("box".to_string(), json!([0.0, 0.0, 40.0, 40.0])),
                ]),
            },
            children: Vec::new(),
        }
    }

    #[test]
    fn explicit_bracket_link_uses_typed_relation_and_policy() {
        let mut engine = Engine::new();
        engine
            .state
            .document
            .objects
            .push(bracket_object("bracket_1"));
        engine
            .state
            .document
            .objects
            .push(text_object("count_1", "4", 42.0, 42.0));
        engine.state.selection.arrow_objects = vec!["bracket_1".to_string()];
        engine.state.selection.text_objects = vec!["count_1".to_string()];

        assert!(engine.selection_can_link());
        let linked = engine.link_selection();
        assert!(
            linked,
            "links={:?}, bracketPolicy={:?}, textPolicy={:?}",
            engine.state.document.links,
            engine
                .state
                .document
                .find_scene_object("bracket_1")
                .map(|object| object.link_policy),
            engine
                .state
                .document
                .find_scene_object("count_1")
                .map(|object| object.link_policy)
        );
        assert_eq!(engine.state.document.links.len(), 1);
        assert_eq!(engine.state.document.links[0].kind, "bracket-repeat-label");
        assert_eq!(
            engine
                .state
                .document
                .find_scene_object("bracket_1")
                .unwrap()
                .link_policy,
            LinkPolicy::Linked
        );
        assert!(engine.unlink_selection());
        assert!(engine.state.document.links.is_empty());
        assert_eq!(
            engine
                .state
                .document
                .find_scene_object("count_1")
                .unwrap()
                .link_policy,
            LinkPolicy::Unlinked
        );
    }

    #[test]
    fn two_molecules_never_enable_link() {
        let mut engine = Engine::new();
        let mut second = engine.state.document.objects[0].clone();
        second.id = "obj_second_molecule".to_string();
        second.payload.resource_ref = None;
        engine.state.document.objects.push(second);
        engine.state.selection.molecule_objects = vec![
            "obj_editor_molecule".to_string(),
            "obj_second_molecule".to_string(),
        ];

        assert!(!engine.selection_can_link());
        let menu: Value =
            serde_json::from_str(&engine.context_menu_json(r#"{"kind":"object"}"#, false)).unwrap();
        let link_item = menu
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item.get("label").and_then(Value::as_str) == Some("Link"))
            .unwrap();
        let explicit = link_item
            .get("submenu")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .find(|item| item.get("command").and_then(Value::as_str) == Some("link"))
            .unwrap();
        assert_eq!(
            explicit.get("disabled").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn analysis_paste_creates_following_linked_caption() {
        let mut engine = Engine::new();
        let mut entry = engine.state.document.editable_fragment_mut().unwrap();
        entry
            .fragment
            .nodes
            .push(Node::carbon("node_c".to_string(), Point::new(100.0, 100.0)));
        entry.update_bounds();
        engine.state.selection = SelectionState {
            molecule_objects: vec!["obj_editor_molecule".to_string()],
            nodes: vec!["node_c".to_string()],
            ..SelectionState::default()
        };

        assert!(engine.paste_selection_analysis_caption(2));
        let relation = engine
            .state
            .document
            .links
            .iter()
            .find(|relation| relation.kind == "analysis-caption")
            .unwrap();
        let caption_id = relation
            .endpoints
            .iter()
            .find(|endpoint| endpoint.role == "caption")
            .unwrap()
            .entity_id
            .clone();
        let caption = engine
            .state
            .document
            .find_scene_object(&caption_id)
            .unwrap();
        assert_eq!(caption.link_policy, LinkPolicy::Linked);
        assert!(caption
            .payload
            .extra
            .get("text")
            .and_then(Value::as_str)
            .unwrap()
            .contains("Formula: CH4"));
        assert_eq!(
            caption
                .payload
                .extra
                .get("analysisCaption")
                .and_then(|value| value.get("anchorMode"))
                .and_then(Value::as_str),
            Some("follow")
        );
        assert!(matches!(
            engine.state.document.resources["mol_editor"].data,
            ResourceData::Fragment(_)
        ));

        let label_edited_text = caption
            .payload
            .extra
            .get("text")
            .and_then(Value::as_str)
            .unwrap()
            .replacen("Formula:", "Molecular Formula:", 1);
        let position = caption.transform.translate;
        let edit_session = |text: String| super::super::TextEditSession {
            target: super::super::TextEditTarget::TextObject {
                object_id: Some(caption_id.clone()),
                x: position[0],
                y: position[1],
            },
            text: text.clone(),
            source_runs: vec![LabelRun {
                text,
                font_family: Some("Arial".to_string()),
                font_size: Some(10.0),
                fill: Some("#000000".to_string()),
                ..LabelRun::default()
            }],
            font_family: Some("Arial".to_string()),
            font_size: Some(10.0),
            fill: Some("#000000".to_string()),
            align: Some("left".to_string()),
            line_height: Some(12.0),
            box_value: None,
            anchor_offset: None,
            text_position: None,
            glyph_polygons: Vec::new(),
            preserve_lines: true,
            default_chemical: false,
            display_mode: None,
        };
        assert!(engine.apply_text_edit(edit_session(label_edited_text.clone())));
        assert!(engine
            .state
            .document
            .links
            .iter()
            .any(|relation| relation.kind == "analysis-caption"));
        let numeric_edited_text = label_edited_text.replacen("CH4", "CH3", 1);
        assert!(engine.apply_text_edit(edit_session(numeric_edited_text)));
        assert!(!engine
            .state
            .document
            .links
            .iter()
            .any(|relation| relation.kind == "analysis-caption"));
        assert_eq!(
            engine
                .state
                .document
                .find_scene_object(&caption_id)
                .unwrap()
                .link_policy,
            LinkPolicy::Unlinked
        );
        assert!(engine
            .take_pending_dialog_json()
            .contains("auto-updating to be disabled"));
    }

    #[test]
    fn moving_analysis_caption_switches_its_anchor_to_fixed() {
        let mut engine = Engine::new();
        let mut entry = engine.state.document.editable_fragment_mut().unwrap();
        entry
            .fragment
            .nodes
            .push(Node::carbon("node_c".to_string(), Point::new(100.0, 100.0)));
        entry.update_bounds();
        engine.state.selection.molecule_objects = vec!["obj_editor_molecule".to_string()];
        assert!(engine.paste_selection_analysis_caption(2));
        let caption_id = engine.state.selection.text_objects[0].clone();

        assert!(engine.with_command(EditorCommand::MoveSelection, |engine| {
            engine.push_undo_snapshot();
            let caption = engine
                .state
                .document
                .find_scene_object_mut(&caption_id)
                .unwrap();
            caption.transform.translate[0] += 10.0;
            true
        }));

        assert_eq!(
            engine
                .state
                .document
                .find_scene_object(&caption_id)
                .and_then(|object| object.payload.extra.get("analysisCaption"))
                .and_then(|value| value.get("anchorMode"))
                .and_then(Value::as_str),
            Some("fixed")
        );
    }

    #[test]
    fn clipboard_remaps_atom_symbol_link_node_endpoint() {
        let mut engine = Engine::new();
        let mut entry = engine.state.document.editable_fragment_mut().unwrap();
        entry
            .fragment
            .nodes
            .push(Node::carbon("node_c".to_string(), Point::new(100.0, 100.0)));
        entry.update_bounds();
        let mut symbol = text_object("symbol_1", "+", 105.0, 95.0);
        symbol.object_type = "symbol".to_string();
        symbol.link_policy = LinkPolicy::Linked;
        symbol
            .payload
            .extra
            .insert("chemicalRole".to_string(), json!("charge"));
        engine.state.document.objects.push(symbol);
        engine.state.document.links.push(LinkRelation {
            id: "link_1".to_string(),
            kind: "atom-symbol".to_string(),
            endpoints: vec![
                LinkEndpoint {
                    entity_id: "node_c".to_string(),
                    role: "atom".to_string(),
                },
                LinkEndpoint {
                    entity_id: "symbol_1".to_string(),
                    role: "symbol".to_string(),
                },
            ],
            data: Value::Null,
        });
        engine.state.selection = SelectionState {
            molecule_objects: vec!["obj_editor_molecule".to_string()],
            arrow_objects: vec!["symbol_1".to_string()],
            ..SelectionState::default()
        };

        assert!(engine.copy_selection());
        assert!(engine.paste_clipboard());
        let pasted_relation = engine
            .state
            .document
            .links
            .iter()
            .find(|relation| relation.id != "link_1" && relation.kind == "atom-symbol")
            .unwrap();
        let pasted_node_id = pasted_relation
            .endpoints
            .iter()
            .find(|endpoint| endpoint.role == "atom")
            .unwrap()
            .entity_id
            .as_str();
        assert_ne!(pasted_node_id, "node_c");
        assert!(engine
            .state
            .document
            .editable_fragments()
            .iter()
            .any(|entry| entry
                .fragment
                .nodes
                .iter()
                .any(|node| node.id == pasted_node_id)));
    }
}
