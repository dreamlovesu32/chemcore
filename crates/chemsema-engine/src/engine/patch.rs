use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPatch {
    pub revision: u64,
    pub before_revision: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upsert_entities: Vec<SceneEntityPatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deleted_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy_roots: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub upsert_resources: BTreeMap<String, crate::Resource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_scope_entity_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<crate::LinkRelation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub upsert_styles: BTreeMap<String, JsonValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deleted_style_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_objects: Option<crate::LogicalObjectData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction_schemes: Option<Vec<crate::ReactionSchemeData>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chemical_properties: Option<Vec<crate::ChemicalProperty>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orders: Option<crate::DocumentOrders>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntityPatch {
    pub entity: crate::SceneObject,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_ids: Vec<String>,
}

impl Engine {
    pub(super) fn current_document_patch(&self) -> Option<DocumentPatch> {
        let result = self.last_command_result()?;
        if !result.changed {
            return None;
        }

        let mut scope_ids = result
            .created
            .nodes
            .iter()
            .chain(result.updated.nodes.iter())
            .chain(result.deleted.nodes.iter())
            .chain(result.created.bonds.iter())
            .chain(result.updated.bonds.iter())
            .chain(result.deleted.bonds.iter())
            .chain(result.created.objects.iter())
            .chain(result.updated.objects.iter())
            .chain(result.deleted.objects.iter())
            .cloned()
            .collect::<BTreeSet<_>>();

        let component_ids = result
            .created
            .nodes
            .iter()
            .chain(result.updated.nodes.iter())
            .chain(result.deleted.nodes.iter())
            .chain(result.created.bonds.iter())
            .chain(result.updated.bonds.iter())
            .chain(result.deleted.bonds.iter())
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut scene_ids = result
            .created
            .objects
            .iter()
            .chain(result.updated.objects.iter())
            .filter(|id| self.state.document.find_scene_object(id).is_some())
            .cloned()
            .collect::<BTreeSet<_>>();
        for entry in self.state.document.editable_fragments() {
            if entry
                .fragment
                .nodes
                .iter()
                .any(|node| component_ids.contains(node.id.as_str()))
                || entry
                    .fragment
                    .bonds
                    .iter()
                    .any(|bond| component_ids.contains(bond.id.as_str()))
            {
                scene_ids.insert(entry.object.id.clone());
                scope_ids.insert(entry.object.id.clone());
            }
        }

        let mut upsert_entities = Vec::new();
        let mut upsert_resources = BTreeMap::new();
        for id in &scene_ids {
            let Some(object) = self.state.document.find_scene_object(id) else {
                continue;
            };
            let mut entity = object.clone();
            let child_ids = entity
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect();
            entity.children.clear();
            if let Some(resource_id) = entity.payload.resource_ref.as_ref() {
                if let Some(resource) = self.state.document.resources.get(resource_id) {
                    upsert_resources.insert(resource_id.clone(), resource.clone());
                }
            }
            upsert_entities.push(SceneEntityPatch {
                entity,
                parent_id: self.state.document.ancestor_group_id_for_scene_object(id),
                child_ids,
            });
        }

        let relation_scope_entity_ids = scope_ids.into_iter().collect::<Vec<_>>();
        let relations = self
            .state
            .document
            .links
            .iter()
            .filter(|relation| {
                relation.endpoints.iter().any(|endpoint| {
                    relation_scope_entity_ids
                        .binary_search(&endpoint.entity_id)
                        .is_ok()
                })
            })
            .cloned()
            .collect();
        let upsert_styles = result
            .created
            .styles
            .iter()
            .chain(result.updated.styles.iter())
            .filter_map(|id| {
                self.state
                    .document
                    .styles
                    .get(id)
                    .cloned()
                    .map(|value| (id.clone(), value))
            })
            .collect();

        let object_structure_changed = !result.created.objects.is_empty()
            || !result.updated.objects.is_empty()
            || !result.deleted.objects.is_empty();
        // Scene entities, logical objects, reaction schemes, and chemical
        // properties share the command target namespace. Include the semantic
        // sections only when a target is not a current scene entity (or when a
        // deletion makes that distinction impossible). This keeps ordinary
        // geometry patches local without leaving semantic editor state stale.
        let before_scene_ids = result
            .command
            .as_ref()
            .map(|command| self.document_patch_before_scene_ids(command))
            .unwrap_or_default();
        let semantic_state_changed = result
            .deleted
            .objects
            .iter()
            .any(|id| !before_scene_ids.contains(id))
            || result
                .created
                .objects
                .iter()
                .chain(result.updated.objects.iter())
                .any(|id| self.state.document.find_scene_object(id).is_none());
        Some(DocumentPatch {
            revision: result.revision,
            before_revision: result.before_revision,
            upsert_entities,
            deleted_entity_ids: result
                .deleted
                .objects
                .iter()
                .filter(|id| before_scene_ids.contains(*id))
                .cloned()
                .collect(),
            hierarchy_roots: object_structure_changed.then(|| {
                self.state
                    .document
                    .objects
                    .iter()
                    .map(|object| object.id.clone())
                    .collect()
            }),
            upsert_resources,
            relation_scope_entity_ids,
            relations,
            upsert_styles,
            deleted_style_ids: result.deleted.styles.clone(),
            logical_objects: semantic_state_changed
                .then(|| self.state.document.logical_objects.clone()),
            reaction_schemes: semantic_state_changed
                .then(|| self.state.document.reaction_schemes.clone()),
            chemical_properties: semantic_state_changed
                .then(|| self.state.document.chemical_properties.clone()),
            orders: object_structure_changed.then(|| self.state.document.orders.clone()),
        })
    }

    fn document_patch_before_scene_ids(&self, command: &EditorCommand) -> BTreeSet<String> {
        let entry = if matches!(command, EditorCommand::Undo) {
            self.redo_stack.last()
        } else {
            self.undo_stack.last()
        };
        let Some(entry) = entry else {
            return BTreeSet::new();
        };
        let objects = match (&entry.snapshot, matches!(command, EditorCommand::Undo)) {
            (
                HistorySnapshot::Document {
                    after: Some(document),
                    ..
                },
                true,
            ) => document.scene_objects(),
            (HistorySnapshot::Document { before, .. }, _) => before.scene_objects(),
            (
                HistorySnapshot::SceneObjects {
                    after_objects: Some(objects),
                    ..
                },
                true,
            ) => scene_objects_in_roots(objects),
            (HistorySnapshot::SceneObjects { before_objects, .. }, _) => {
                scene_objects_in_roots(before_objects)
            }
        };
        objects
            .into_iter()
            .map(|object| object.id.clone())
            .collect()
    }
}

fn scene_objects_in_roots(objects: &[crate::SceneObject]) -> Vec<&crate::SceneObject> {
    fn collect<'a>(objects: &'a [crate::SceneObject], result: &mut Vec<&'a crate::SceneObject>) {
        for object in objects {
            result.push(object);
            collect(&object.children, result);
        }
    }
    let mut result = Vec::new();
    collect(objects, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_bond_edit_exposes_a_local_document_patch() {
        let mut engine = Engine::new();
        engine
            .execute_command_json(
                r#"{"type":"add-bond","begin":{"x":80.0,"y":80.0},"end":{"x":128.0,"y":80.0},"order":1,"variant":"single"}"#,
            )
            .expect("bond command succeeds");

        let patch: JsonValue = serde_json::from_str(
            &engine
                .document_patch_json()
                .expect("document patch serializes"),
        )
        .expect("document patch is JSON");
        assert_eq!(patch["beforeRevision"], 0);
        assert_eq!(patch["revision"], 1);
        assert_eq!(
            patch["upsertEntities"][0]["entity"]["id"],
            "obj_editor_molecule"
        );
        assert!(patch["upsertEntities"][0]["entity"]
            .get("children")
            .is_none());
        assert!(patch["upsertResources"].get("mol_editor").is_some());
    }

    #[test]
    fn deleted_scene_entity_is_reported_from_the_before_snapshot() {
        let mut engine = Engine::new();
        let created: JsonValue = serde_json::from_str(
            &engine
                .execute_command_json(
                    r#"{"type":"add-text","position":{"x":20.0,"y":20.0},"text":"note"}"#,
                )
                .expect("text command succeeds"),
        )
        .expect("command result parses");
        let object_id = created["created"]["objects"][0]
            .as_str()
            .expect("created object id");
        engine
            .execute_command_json(
                &serde_json::json!({
                    "type": "delete-targets",
                    "targets": { "objects": [object_id] }
                })
                .to_string(),
            )
            .expect("delete command succeeds");

        let patch: DocumentPatch = serde_json::from_str(
            &engine
                .document_patch_json()
                .expect("document patch serializes"),
        )
        .expect("document patch parses");
        assert_eq!(patch.deleted_entity_ids, vec![object_id]);
    }
}
