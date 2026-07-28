use super::*;

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
        let mut document = crate::parse_document_json(json)?;
        refresh_repeating_units(&mut document);
        let options = editor_options_from_document(&document);
        let document_style_preset = document_style_preset_from_document(&document).to_string();
        sync_document_style_info_from_options(&mut document, &document_style_preset, &options);
        self.state.document = document;
        self.options = options;
        self.document_style_preset = document_style_preset;
        self.refresh_symbol_chemistry();
        refresh_element_valence_recognition_for_all_editable_fragments(&mut self.state.document);
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.command_context.clear();
        self.revision = 0;
        self.last_command_result = None;
        self.pending_dialog = None;
        self.next_id = self.infer_next_id();
        Ok(())
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
