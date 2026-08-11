use super::*;

fn hydrated_subtree(object: &SceneObject, added_ids: &BTreeSet<String>) -> SceneObject {
    let mut value = object.clone();
    value.children = object
        .children
        .iter()
        .filter(|child| added_ids.contains(&child.id))
        .map(|child| hydrated_subtree(child, added_ids))
        .collect();
    value
}

fn merge_hydration_into_history_document(
    document: &mut ChemSemaDocument,
    incoming: &ChemSemaDocument,
    added_ids: &BTreeSet<String>,
    added_resource_ids: &BTreeSet<String>,
) {
    fn visit<'a>(
        objects: &'a [SceneObject],
        parent: Option<&str>,
        out: &mut Vec<(&'a SceneObject, Option<String>)>,
    ) {
        for object in objects {
            out.push((object, parent.map(str::to_string)));
            visit(&object.children, Some(&object.id), out);
        }
    }

    let mut incoming_objects = Vec::new();
    visit(&incoming.objects, None, &mut incoming_objects);
    for (object, parent_id) in incoming_objects.iter().filter(|(object, parent)| {
        added_ids.contains(&object.id)
            && parent
                .as_ref()
                .is_none_or(|parent_id| !added_ids.contains(parent_id))
    }) {
        let value = hydrated_subtree(object, added_ids);
        if let Some(parent) = parent_id
            .as_deref()
            .and_then(|id| document.find_scene_object_mut(id))
        {
            parent.children.push(value);
        } else {
            document.objects.push(value);
        }
    }
    for id in added_resource_ids {
        if let Some(resource) = incoming.resources.get(id) {
            document.resources.insert(id.clone(), resource.clone());
        }
    }
    for link in &incoming.links {
        if link
            .endpoints
            .iter()
            .any(|endpoint| added_ids.contains(&endpoint.entity_id))
            && !document.links.iter().any(|existing| existing.id == link.id)
        {
            document.links.push(link.clone());
        }
    }
    for id in &incoming.orders.reading {
        if added_ids.contains(id) && !document.orders.reading.contains(id) {
            document.orders.reading.push(id.clone());
        }
    }
}

fn merge_hydration_into_history_entry(
    entry: &mut HistoryEntry,
    incoming: &ChemSemaDocument,
    added_ids: &BTreeSet<String>,
    added_resource_ids: &BTreeSet<String>,
) {
    if let HistorySnapshot::Document { before, after } = &mut entry.snapshot {
        merge_hydration_into_history_document(before, incoming, added_ids, added_resource_ids);
        if let Some(after) = after {
            merge_hydration_into_history_document(after, incoming, added_ids, added_resource_ids);
        }
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState {
                document: ChemSemaDocument::blank(),
                tool: ToolState::default(),
                selection: SelectionState::default(),
                overlay: OverlayState::default(),
            },
            drag: None,
            arrow_drag: None,
            arrow_edit_drag: None,
            tlc_spot_drag: None,
            orbital_drag: None,
            selection_drag: None,
            selection_rotate_drag: None,
            selection_resize_drag: None,
            template_drag: None,
            shape_drag: None,
            shape_edit_drag: None,
            bracket_edit_drag: None,
            bracket_drag: None,
            pending_select_target: None,
            pointer_bond_target: None,
            clipboard: None,
            options: EditorOptions::default(),
            document_style_preset: DEFAULT_DOCUMENT_STYLE_PRESET.to_string(),
            next_id: 1,
            revision: 0,
            last_command_result: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            command_context: Vec::new(),
            command_before_snapshot: None,
            pending_dialog: None,
            spatial_index: std::cell::RefCell::new(None),
        }
    }

    pub fn state(&self) -> &EngineState {
        &self.state
    }

    pub fn state_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.state)
    }

    pub fn document_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.state.document)
    }

    pub fn document_patch_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.current_document_patch())
    }

    pub fn spatial_query_json(&self, bounds: [f64; 4]) -> serde_json::Result<String> {
        serde_json::to_string(&self.spatial_query(bounds))
    }

    pub fn document_cdxml(&self) -> String {
        crate::document_to_cdxml(&self.state.document)
    }

    pub fn document_cdx(&self) -> Result<Vec<u8>, String> {
        crate::document_to_cdx(&self.state.document)
    }

    pub fn document_sdf(&self) -> Result<String, String> {
        crate::document_to_sdf(&self.state.document)
    }

    pub fn document_svg(&self) -> String {
        crate::document_to_svg(&self.state.document)
    }

    pub fn document_colors(&self) -> Vec<String> {
        collect_document_colors(&self.state.document)
    }

    pub fn render_bounds(&self, scope: RenderBoundsScope) -> Option<[f64; 4]> {
        if scope == RenderBoundsScope::Selection {
            return self.selection_bounds();
        }
        let primitives = self.render_list();
        render_primitives_bounds(
            primitives
                .iter()
                .filter(|primitive| render_bounds_scope_accepts(scope, primitive)),
        )
    }

    pub fn load_document_json(&mut self, json: &str) -> Result<(), String> {
        let legacy_spatial_symbol_links = serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/format/version")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .is_some_and(|version| version == "0.1");
        let mut document = crate::parse_document_json(json)?;
        refresh_repeating_units(&mut document);
        let options = editor_options_from_document(&document);
        let document_style_preset = document_style_preset_from_document(&document).to_string();
        sync_document_style_info_from_options(&mut document, &document_style_preset, &options);
        self.state.document = document;
        self.options = options;
        self.document_style_preset = document_style_preset;
        // A CCJS 0.2 snapshot is authoritative. Do not run editing-time
        // inference here: even "refresh" operations can change declared
        // relations or label/bond geometry and break save/reopen identity.
        // Importers and edit commands materialize derived chemistry before a
        // snapshot is written; validation reports stale or invalid data.
        // Attached symbol chemistry is an explicit cross-object invariant and
        // must still be reconciled for externally authored CCJS snapshots.
        // Reconcile declared attachments only: a snapshot load must not invent
        // a new spatial auto-link merely because a symbol happens to be near
        // an atom.
        if legacy_spatial_symbol_links {
            // CCJS 0.1 had no durable relation index, so spatial recovery is
            // part of its migration into the explicit 0.2 model.
            self.refresh_symbol_chemistry();
        } else {
            self.refresh_loaded_symbol_chemistry();
        }
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.command_context.clear();
        self.revision = 0;
        self.last_command_result = None;
        self.pending_dialog = None;
        *self.spatial_index.borrow_mut() = None;
        self.next_id = self.infer_next_id();
        Ok(())
    }

    /// Merge a cumulative partial CCJS snapshot into the open document
    /// without resetting revision or undo history. Existing loaded objects and
    /// resources remain authoritative so viewport hydration cannot overwrite
    /// user edits; only newly available chunks are added and hierarchy is
    /// reconciled from the cumulative snapshot.
    pub fn hydrate_document_json(&mut self, json: &str) -> Result<usize, String> {
        let incoming = crate::parse_document_json(json)?;
        if incoming.document.id != self.state.document.document.id {
            return Err("cannot hydrate chunks from a different document".to_string());
        }
        fn flatten(objects: &[SceneObject], out: &mut BTreeMap<String, SceneObject>) {
            for object in objects {
                let mut value = object.clone();
                value.children.clear();
                out.insert(value.id.clone(), value);
                flatten(&object.children, out);
            }
        }
        fn rebuild(
            mut object: SceneObject,
            current: &mut BTreeMap<String, SceneObject>,
        ) -> SceneObject {
            let incoming_children = std::mem::take(&mut object.children);
            let mut value = current.remove(&object.id).unwrap_or(object);
            value.children = incoming_children
                .into_iter()
                .map(|child| rebuild(child, current))
                .collect();
            value
        }
        let mut current = BTreeMap::new();
        flatten(&self.state.document.objects, &mut current);
        let existing_ids = current.keys().cloned().collect::<BTreeSet<_>>();
        let mut incoming_ids = BTreeSet::new();
        fn collect_ids(objects: &[SceneObject], ids: &mut BTreeSet<String>) {
            for object in objects {
                ids.insert(object.id.clone());
                collect_ids(&object.children, ids);
            }
        }
        collect_ids(&incoming.objects, &mut incoming_ids);
        let added_ids = incoming_ids
            .difference(&existing_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let added_resource_ids = incoming
            .resources
            .keys()
            .filter(|id| !self.state.document.resources.contains_key(*id))
            .cloned()
            .collect::<BTreeSet<_>>();
        for entry in self.undo_stack.iter_mut().chain(self.redo_stack.iter_mut()) {
            merge_hydration_into_history_entry(entry, &incoming, &added_ids, &added_resource_ids);
        }
        let added = added_ids.len();
        let mut objects = incoming
            .objects
            .into_iter()
            .map(|object| rebuild(object, &mut current))
            .collect::<Vec<_>>();
        objects.extend(current.into_values());
        self.state.document.objects = objects;
        for (id, resource) in incoming.resources {
            self.state.document.resources.entry(id).or_insert(resource);
        }
        // The cumulative viewport snapshot already contains local edits for
        // the loaded region. Replacing these indexes is therefore required:
        // extending them would resurrect links or order entries deleted by
        // the user before a later chunk was hydrated.
        self.state.document.links = incoming.links;
        self.state.document.orders = incoming.orders;
        *self.spatial_index.borrow_mut() = None;
        self.next_id = self.infer_next_id();
        Ok(added)
    }

    pub fn load_cdxml_document(&mut self, cdxml: &str) -> Result<(), String> {
        let mut document = crate::parse_cdxml_document(cdxml, None)?;
        crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
        self.load_imported_document(document)
    }

    pub fn load_cdx_document(&mut self, cdx: &[u8]) -> Result<(), String> {
        let mut document = crate::parse_cdx_document(cdx, None)?;
        crate::cdxml::normalize_cdxml_document_for_editing(&mut document);
        self.load_imported_document(document)
    }

    pub fn load_sdf_document(&mut self, sdf: &str) -> Result<(), String> {
        let document = crate::parse_sdf_document(sdf, None)?;
        self.load_imported_document(document)
    }

    pub(super) fn load_imported_document(
        &mut self,
        mut document: ChemSemaDocument,
    ) -> Result<(), String> {
        refresh_repeating_units(&mut document);
        self.state.document = document;
        self.next_id = self.infer_next_id();
        self.link_imported_repeat_unit_labels_untracked();
        refresh_repeating_units(&mut self.state.document);
        let options = editor_options_from_imported_cdxml_document(&self.state.document);
        let document_style_preset =
            document_style_preset_from_document(&self.state.document).to_string();
        sync_document_style_info_from_options(
            &mut self.state.document,
            &document_style_preset,
            &options,
        );
        self.options = options;
        self.document_style_preset = document_style_preset;
        self.refresh_loaded_symbol_chemistry();
        refresh_element_valence_recognition_for_all_editable_fragments(&mut self.state.document);
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.command_context.clear();
        self.revision = 0;
        self.pending_dialog = None;
        self.last_command_result = None;
        self.next_id = self.infer_next_id();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_hydration_preserves_local_edits_revision_and_undo() {
        let mut engine = Engine::new();
        let result: JsonValue = serde_json::from_str(
            &engine
                .execute_command_json(
                    r#"{"type":"add-text","position":{"x":20.0,"y":20.0},"text":"local edit"}"#,
                )
                .expect("local edit succeeds"),
        )
        .expect("command result parses");
        let local_id = result["created"]["objects"][0]
            .as_str()
            .expect("created object id")
            .to_string();
        let local_before = engine
            .state
            .document
            .find_scene_object(&local_id)
            .expect("local object exists")
            .clone();
        let revision_before = engine.revision();
        assert!(engine.can_undo());

        let mut cumulative = engine.state.document.clone();
        cumulative
            .find_scene_object_mut(&local_id)
            .expect("incoming stale object exists")
            .name = "stale archive value".to_string();
        let mut remote = local_before.clone();
        remote.id = "hydrated_remote".to_string();
        remote.name = "newly hydrated".to_string();
        cumulative.objects.push(remote);

        let added = engine
            .hydrate_document_json(
                &serde_json::to_string(&cumulative).expect("cumulative document serializes"),
            )
            .expect("viewport hydration succeeds");

        assert_eq!(added, 1);
        assert_eq!(engine.revision(), revision_before);
        assert!(engine.can_undo());
        assert_eq!(
            engine
                .state
                .document
                .find_scene_object(&local_id)
                .expect("local object survives")
                .name,
            local_before.name,
        );
        assert!(engine
            .state
            .document
            .find_scene_object("hydrated_remote")
            .is_some());

        assert!(engine.undo());
        assert!(engine.state.document.find_scene_object(&local_id).is_none());
        assert!(engine
            .state
            .document
            .find_scene_object("hydrated_remote")
            .is_some());
    }
}
