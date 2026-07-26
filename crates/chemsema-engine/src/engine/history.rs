use super::*;

impl Engine {
    pub fn undo(&mut self) -> bool {
        let Some(mut entry) = self.undo_stack.pop() else {
            return false;
        };
        let before_revision = self.revision;
        let before_document = self.state.document.clone();
        self.capture_history_after_snapshot(&mut entry);
        self.restore_history_before_snapshot(&entry);
        self.redo_stack.push(entry);
        self.commit_command_result(EditorCommand::Undo, before_revision, before_document);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_stack.pop() else {
            return false;
        };
        if !self.history_entry_has_after_snapshot(&entry) {
            return false;
        }
        let before_revision = self.revision;
        let before_document = self.state.document.clone();
        self.restore_history_after_snapshot(&entry);
        self.undo_stack.push(entry);
        self.commit_command_result(EditorCommand::Redo, before_revision, before_document);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn last_command_result(&self) -> Option<&CommandResult> {
        self.last_command_result.as_ref()
    }

    pub fn last_command_result_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.last_command_result)
    }

    pub fn history_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.undo_stack)
    }

    pub(super) fn next_id(&mut self, prefix: &str) -> String {
        let value = self.next_id;
        self.next_id += 1;
        format!("{prefix}_{value}")
    }

    pub(super) fn with_command<F>(&mut self, command: EditorCommand, apply: F) -> bool
    where
        F: FnOnce(&mut Self) -> bool,
    {
        if !self.command_context.is_empty() {
            return apply(self);
        }
        let before_revision = self.revision;
        let use_scene_object_history = self.command_can_use_scene_object_history(&command);
        let before_document = if use_scene_object_history {
            None
        } else {
            Some(self.state.document.clone())
        };
        let before_redo_stack = self.redo_stack.clone();
        let undo_len = self.undo_stack.len();
        let chemical_property_fingerprints = self.chemical_property_basis_fingerprints();
        self.command_context.push(command.clone());
        self.command_before_snapshot = before_document;
        let applied = apply(self);
        self.command_context.pop();
        let command_before_snapshot = self.command_before_snapshot.take();
        if applied {
            self.mark_moved_analysis_captions_fixed(&command);
            self.mark_changed_chemical_properties_stale(&chemical_property_fingerprints);
            self.reconcile_links_after_document_change();
            let delta_scope = self.command_delta_scope(&command);
            if self.command_needs_repeating_unit_refresh(&command, delta_scope) {
                refresh_repeating_units(&mut self.state.document);
            }
            let delta = if use_scene_object_history {
                let before_objects = self
                    .history_before_scene_objects_for_command(undo_len, &command)
                    .expect("changed scene-object command must have a before object snapshot");
                scene_object_target_delta(before_objects, &self.state.document)
            } else {
                let command_before_document = self
                    .history_before_document_for_command(undo_len, &command)
                    .or(command_before_snapshot.as_ref())
                    .expect("changed command must have a before document snapshot");
                document_target_delta_with_scope(
                    command_before_document,
                    &self.state.document,
                    delta_scope,
                )
            };
            if command_target_delta_is_empty(&delta) {
                self.cleanup_unchanged_command_history(undo_len, &command, before_redo_stack);
                self.last_command_result = Some(self.unchanged_command_result());
                false
            } else {
                self.finalize_command_history(undo_len, command.clone());
                self.commit_command_result_delta(command, before_revision, delta);
                true
            }
        } else {
            self.cleanup_unchanged_command_history(undo_len, &command, before_redo_stack);
            self.last_command_result = Some(self.unchanged_command_result());
            false
        }
    }

    pub(super) fn with_transient_command<F>(&mut self, command: EditorCommand, apply: F) -> bool
    where
        F: FnOnce(&mut Self) -> bool,
    {
        if !self.command_context.is_empty() {
            return apply(self);
        }
        self.command_context.push(command);
        let changed = apply(self);
        self.command_context.pop();
        changed
    }

    pub(super) fn finalize_command_history(&mut self, undo_len: usize, command: EditorCommand) {
        if self.undo_stack.len() <= undo_len {
            if let Some(entry) = self.undo_stack.last_mut() {
                if history_entry_is_open_for_command(entry, &command) {
                    capture_history_after_snapshot_for_document(entry, &self.state.document);
                }
            }
            return;
        }
        let mut entries = self.undo_stack.split_off(undo_len);
        let mut entry = entries.remove(0);
        entry.command = command;
        capture_history_after_snapshot_for_document(&mut entry, &self.state.document);
        self.undo_stack.push(entry);
    }

    pub(super) fn history_before_document_for_command(
        &self,
        undo_len: usize,
        command: &EditorCommand,
    ) -> Option<&ChemSemaDocument> {
        if self.undo_stack.len() > undo_len {
            return self
                .undo_stack
                .get(undo_len)
                .and_then(history_entry_before_document);
        }
        self.undo_stack
            .iter()
            .rev()
            .find(|entry| history_entry_is_open_for_command(entry, command))
            .and_then(history_entry_before_document)
    }

    pub(super) fn history_before_scene_objects_for_command(
        &self,
        undo_len: usize,
        command: &EditorCommand,
    ) -> Option<&[SceneObject]> {
        if self.undo_stack.len() > undo_len {
            return self
                .undo_stack
                .get(undo_len)
                .and_then(history_entry_before_scene_objects);
        }
        self.undo_stack
            .iter()
            .rev()
            .find(|entry| history_entry_is_open_for_command(entry, command))
            .and_then(history_entry_before_scene_objects)
    }

    pub(super) fn cleanup_unchanged_command_history(
        &mut self,
        undo_len: usize,
        command: &EditorCommand,
        before_redo_stack: Vec<HistoryEntry>,
    ) {
        if self.undo_stack.len() > undo_len {
            self.undo_stack.truncate(undo_len);
            self.redo_stack = before_redo_stack;
            return;
        }
        if self
            .undo_stack
            .last()
            .is_some_and(|entry| history_entry_is_open_for_command(entry, command))
        {
            self.undo_stack.pop();
        }
    }

    pub(super) fn commit_command_result(
        &mut self,
        command: EditorCommand,
        before_revision: u64,
        before_document: ChemSemaDocument,
    ) {
        self.revision = self.revision.saturating_add(1);
        self.last_command_result = Some(self.command_result_from_diff(
            Some(command),
            before_revision,
            &before_document,
            &self.state.document,
        ));
    }

    pub(super) fn commit_command_result_delta(
        &mut self,
        command: EditorCommand,
        before_revision: u64,
        delta: CommandTargetDelta,
    ) {
        self.revision = self.revision.saturating_add(1);
        self.last_command_result =
            Some(self.command_result_from_delta(Some(command), before_revision, delta));
    }

    pub(super) fn command_result_from_diff(
        &self,
        command: Option<EditorCommand>,
        before_revision: u64,
        before_document: &ChemSemaDocument,
        after_document: &ChemSemaDocument,
    ) -> CommandResult {
        let delta = document_target_delta(before_document, after_document);
        self.command_result_from_delta(command, before_revision, delta)
    }

    pub(super) fn command_result_from_delta(
        &self,
        command: Option<EditorCommand>,
        before_revision: u64,
        delta: CommandTargetDelta,
    ) -> CommandResult {
        CommandResult {
            changed: !delta.created.is_empty()
                || !delta.updated.is_empty()
                || !delta.deleted.is_empty(),
            revision: self.revision,
            before_revision,
            command,
            targets: command_targets_union(&delta),
            created: delta.created,
            updated: delta.updated,
            deleted: delta.deleted,
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            undo_depth: self.undo_stack.len(),
            redo_depth: self.redo_stack.len(),
            diagnostics: BTreeMap::new(),
            output: None,
        }
    }

    pub(super) fn readonly_command_result(
        &mut self,
        command: Option<EditorCommand>,
        output: JsonValue,
    ) -> CommandResult {
        let mut result = self.unchanged_command_result();
        result.command = command;
        result.output = Some(output);
        self.last_command_result = Some(result.clone());
        result
    }

    pub(super) fn unchanged_command_result(&self) -> CommandResult {
        CommandResult::unchanged(
            self.revision,
            self.can_undo(),
            self.can_redo(),
            self.undo_stack.len(),
            self.redo_stack.len(),
        )
    }

    pub(super) fn command_delta_scope(&self, command: &EditorCommand) -> CommandDeltaScope {
        match command {
            EditorCommand::AddArrow { .. }
            | EditorCommand::ApplyArrowStyle { .. }
            | EditorCommand::AddShape { .. }
            | EditorCommand::AddTable { .. }
            | EditorCommand::EditTable { .. }
            | EditorCommand::SetTableBorders { .. }
            | EditorCommand::AnalyzeStoichiometry { .. }
            | EditorCommand::SetStoichiometryDatum { .. }
            | EditorCommand::EditStoichiometryGrid { .. }
            | EditorCommand::BindStoichiometryGrid { .. }
            | EditorCommand::AddBracket { .. }
            | EditorCommand::AddSymbol { .. }
            | EditorCommand::AddOrbital { .. }
            | EditorCommand::EditArrowGeometry { .. }
            | EditorCommand::EditShapeGeometry { .. }
            | EditorCommand::ApplyShapeStyle { .. }
            | EditorCommand::ApplyBracketKind { .. }
            | EditorCommand::ApplyOrbitalTemplate { .. }
            | EditorCommand::ApplyOrbitalStyle { .. }
            | EditorCommand::ApplyOrbitalPhase { .. }
            | EditorCommand::ApplyLineStyle { .. } => CommandDeltaScope::objects_and_styles(),
            EditorCommand::MoveSelection
            | EditorCommand::RotateSelection
            | EditorCommand::ResizeSelection
                if self.selection_targets_only_scene_objects() =>
            {
                CommandDeltaScope::objects_and_styles()
            }
            _ => CommandDeltaScope::all(),
        }
    }

    pub(super) fn selection_targets_only_scene_objects(&self) -> bool {
        self.state.selection.nodes.is_empty()
            && self.state.selection.bonds.is_empty()
            && self.state.selection.label_nodes.is_empty()
            && (!self.state.selection.arrow_objects.is_empty()
                || !self.state.selection.text_objects.is_empty())
    }

    pub(super) fn command_needs_repeating_unit_refresh(
        &self,
        command: &EditorCommand,
        delta_scope: CommandDeltaScope,
    ) -> bool {
        if delta_scope.molecule_components {
            return true;
        }
        match command {
            EditorCommand::AddBracket { .. }
            | EditorCommand::ApplyBracketKind { .. }
            | EditorCommand::GroupSelection { .. }
            | EditorCommand::UngroupSelection { .. }
            | EditorCommand::LinkSelection { .. }
            | EditorCommand::SetLinkPolicy { .. }
            | EditorCommand::UnlinkSelection { .. }
            | EditorCommand::JoinSelection => true,
            EditorCommand::MoveSelection
            | EditorCommand::RotateSelection
            | EditorCommand::ResizeSelection => {
                self.selected_scene_objects_need_repeating_unit_refresh()
            }
            _ => false,
        }
    }

    pub(super) fn selected_scene_objects_need_repeating_unit_refresh(&self) -> bool {
        self.state
            .selection
            .arrow_objects
            .iter()
            .chain(self.state.selection.text_objects.iter())
            .filter_map(|object_id| self.state.document.find_scene_object(object_id))
            .any(scene_object_needs_repeating_unit_refresh)
    }

    pub(super) fn current_history_command(&self) -> EditorCommand {
        self.command_context
            .last()
            .cloned()
            .expect("document mutation must run inside Engine::with_command")
    }

    pub(super) fn push_undo_snapshot(&mut self) {
        let command = self.current_history_command();
        if self.command_can_use_scene_object_history(&command) {
            let before_objects = self.history_scene_objects_for_command(&command);
            if !before_objects.is_empty() {
                self.undo_stack
                    .push(HistoryEntry::new_scene_objects(command, before_objects));
                self.redo_stack.clear();
                return;
            }
        }
        let before = self
            .command_before_snapshot
            .take()
            .unwrap_or_else(|| self.state.document.clone());
        self.undo_stack.push(HistoryEntry::new(command, before));
        self.redo_stack.clear();
    }

    pub(super) fn restore_document(&mut self, document: ChemSemaDocument) {
        self.state.document = document;
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.pending_select_target = None;
        self.pending_dialog = None;
        self.next_id = self.infer_next_id();
    }

    pub(super) fn capture_history_after_snapshot(&self, entry: &mut HistoryEntry) {
        capture_history_after_snapshot_for_document(entry, &self.state.document);
    }

    pub(super) fn restore_history_before_snapshot(&mut self, entry: &HistoryEntry) {
        match &entry.snapshot {
            HistorySnapshot::Document { before, .. } => {
                self.restore_document(before.as_ref().clone())
            }
            HistorySnapshot::SceneObjects { before_objects, .. } => {
                self.restore_scene_object_snapshots(before_objects);
            }
        }
    }

    pub(super) fn restore_history_after_snapshot(&mut self, entry: &HistoryEntry) {
        match &entry.snapshot {
            HistorySnapshot::Document {
                after: Some(after), ..
            } => self.restore_document(after.as_ref().clone()),
            HistorySnapshot::SceneObjects {
                after_objects: Some(after_objects),
                ..
            } => {
                self.restore_scene_object_snapshots(after_objects);
            }
            _ => {}
        }
    }

    pub(super) fn history_entry_has_after_snapshot(&self, entry: &HistoryEntry) -> bool {
        match &entry.snapshot {
            HistorySnapshot::Document { after, .. } => after.is_some(),
            HistorySnapshot::SceneObjects { after_objects, .. } => after_objects.is_some(),
        }
    }

    pub(super) fn command_can_use_scene_object_history(&self, command: &EditorCommand) -> bool {
        match command {
            EditorCommand::MoveSelection
            | EditorCommand::RotateSelection
            | EditorCommand::ResizeSelection => self.selection_targets_only_scene_objects(),
            EditorCommand::EditArrowGeometry {
                object_id: Some(_), ..
            }
            | EditorCommand::EditShapeGeometry {
                object_id: Some(_), ..
            } => true,
            _ => false,
        }
    }

    pub(super) fn history_scene_objects_for_command(
        &self,
        command: &EditorCommand,
    ) -> Vec<SceneObject> {
        let object_ids = match command {
            EditorCommand::MoveSelection
            | EditorCommand::RotateSelection
            | EditorCommand::ResizeSelection => self
                .state
                .selection
                .arrow_objects
                .iter()
                .chain(self.state.selection.text_objects.iter())
                .cloned()
                .collect::<BTreeSet<_>>(),
            EditorCommand::EditArrowGeometry {
                object_id: Some(object_id),
                ..
            }
            | EditorCommand::EditShapeGeometry {
                object_id: Some(object_id),
                ..
            } => BTreeSet::from([object_id.clone()]),
            _ => BTreeSet::new(),
        };
        object_ids
            .iter()
            .filter_map(|object_id| self.state.document.find_scene_object(object_id).cloned())
            .collect()
    }

    pub(super) fn restore_scene_object_snapshots(&mut self, objects: &[SceneObject]) {
        for object in objects {
            replace_scene_object_snapshot(&mut self.state.document.objects, object);
        }
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.pending_select_target = None;
        self.next_id = self.infer_next_id();
    }

    pub(super) fn infer_next_id(&self) -> u64 {
        let mut max_id = 0;
        for id in self
            .state
            .document
            .scene_objects()
            .iter()
            .map(|object| object.id.as_str())
        {
            if let Some((_, suffix)) = id.rsplit_once('_') {
                if let Ok(value) = suffix.parse::<u64>() {
                    max_id = max_id.max(value);
                }
            }
        }
        for entry in self.state.document.editable_fragments() {
            for id in entry
                .fragment
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .chain(entry.fragment.bonds.iter().map(|bond| bond.id.as_str()))
            {
                if let Some((_, suffix)) = id.rsplit_once('_') {
                    if let Ok(value) = suffix.parse::<u64>() {
                        max_id = max_id.max(value);
                    }
                }
            }
        }
        for id in self
            .state
            .document
            .links
            .iter()
            .map(|relation| relation.id.as_str())
        {
            if let Some((_, suffix)) = id.rsplit_once('_') {
                if let Ok(value) = suffix.parse::<u64>() {
                    max_id = max_id.max(value);
                }
            }
        }
        for id in self
            .state
            .document
            .chemical_properties
            .iter()
            .map(|property| property.id.as_str())
        {
            if let Some((_, suffix)) = id.rsplit_once('_') {
                if let Ok(value) = suffix.parse::<u64>() {
                    max_id = max_id.max(value);
                }
            }
        }
        max_id + 1
    }
}
