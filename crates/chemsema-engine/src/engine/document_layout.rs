use super::*;

impl Engine {
    fn document_content_bounds(&self) -> Option<[f64; 4]> {
        let primitives = crate::render_document(&self.state.document);
        crate::render_primitives_bounds(primitives.iter())
    }

    pub fn resolved_document_layout(&self) -> crate::ResolvedDocumentLayout {
        self.state
            .document
            .document
            .layout
            .resolve(self.document_content_bounds())
    }

    pub fn document_layout_dialog_json(&self) -> String {
        let layout = &self.state.document.document.layout;
        let data = serde_json::to_value(layout).expect("document layout serializes");
        serde_json::to_string(&json!({
            "kind": "document-layout",
            "title": "Document Layout",
            "data": data,
            "resolved": self.resolved_document_layout(),
            "paperPresets": [
                {"key": "a4", "label": "A4", "width": 595.275590551, "height": 841.88976378},
                {"key": "a3", "label": "A3", "width": 841.88976378, "height": 1190.551181102},
                {"key": "letter", "label": "Letter", "width": 612.0, "height": 792.0},
                {"key": "legal", "label": "Legal", "width": 612.0, "height": 1008.0},
                {"key": "tabloid", "label": "Tabloid", "width": 792.0, "height": 1224.0}
            ],
            "headerFooterTokens": [
                {"token": "&f", "label": "File name"},
                {"token": "&p", "label": "Page number"},
                {"token": "&d", "label": "Print date"},
                {"token": "&t", "label": "Print time"},
                {"token": "&l", "label": "Left section"},
                {"token": "&c", "label": "Center section"},
                {"token": "&r", "label": "Right section"}
            ]
        }))
        .expect("document layout dialog serializes")
    }

    pub fn initialize_document_layout_direct(&mut self) -> bool {
        if self.state.document.document.layout.page_origin.is_some() {
            return false;
        }
        let resolved = self.resolved_document_layout();
        self.state.document.document.layout.page_origin = Some(resolved.anchor_origin);
        true
    }

    pub fn set_document_layout_direct(&mut self, layout: crate::DocumentLayout) -> bool {
        if self.state.document.document.layout == layout {
            return false;
        }
        self.state.document.document.layout = layout;
        true
    }

    pub(super) fn validate_document_layout_candidate(
        &self,
        layout: &crate::DocumentLayout,
    ) -> Result<(), String> {
        let mut document = self.state.document.clone();
        document.document.layout = layout.clone();
        let json = serde_json::to_string(&document).map_err(|error| error.to_string())?;
        crate::parse_document_json(&json).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_layout_initialization_records_centered_page_origin() {
        let mut engine = Engine::new();
        assert!(engine.state.document.document.layout.page_origin.is_none());
        assert!(engine.initialize_document_layout_direct());
        let origin = engine
            .state
            .document
            .document
            .layout
            .page_origin
            .expect("origin");
        assert!(origin[0].is_finite());
        assert!(origin[1].is_finite());
        assert!(!engine.initialize_document_layout_direct());
    }

    #[test]
    fn layout_dialog_is_kernel_owned_and_lists_every_interchange_setting() {
        let value: serde_json::Value =
            serde_json::from_str(&Engine::new().document_layout_dialog_json()).unwrap();
        let data = &value["data"];
        for key in [
            "drawingSpace",
            "paper",
            "widthPages",
            "heightPages",
            "autoPaginate",
            "pageOrigin",
            "margins",
            "pageOverlap",
            "printTrimMarks",
            "header",
            "headerPosition",
            "footer",
            "footerPosition",
            "magnificationPercent",
            "pageDefinition",
            "splitters",
            "legacySplitterPositionIds",
            "fixInPlaceExtent",
            "fixInPlaceGap",
        ] {
            assert!(data.get(key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn legacy_numeric_splitter_positions_migrate_to_explicit_object_ids() {
        let layout: crate::DocumentLayout = serde_json::from_value(json!({
            "splitterPositions": [101, 202.5]
        }))
        .unwrap();
        assert_eq!(
            layout.legacy_splitter_position_ids,
            ["101".to_string(), "202.5".to_string()]
        );
        let serialized = serde_json::to_value(layout).unwrap();
        assert!(serialized.get("splitterPositions").is_none());
        assert_eq!(
            serialized["legacySplitterPositionIds"],
            json!(["101", "202.5"])
        );
    }

    #[test]
    fn layout_command_rejects_splitter_ids_that_collide_with_document_entities() {
        let mut engine = Engine::new();
        let mut layout = engine.state.document.document.layout.clone();
        layout.splitters.push(crate::PageSplitter {
            id: "obj_editor_molecule".to_string(),
            position: None,
            page_definition: crate::PageDefinition::Undefined,
        });
        let error = engine
            .execute_command(crate::engine::EditorCommand::SetDocumentLayout { layout })
            .unwrap_err();
        assert!(error.contains("collides"), "{error}");
    }

    #[test]
    fn cdxml_rejects_unknown_page_definitions_instead_of_falling_back() {
        let error = crate::parse_cdxml_document(
            r#"<CDXML><page PageDefinition="FutureLayout"/></CDXML>"#,
            None,
        )
        .unwrap_err();
        assert!(error.contains("unsupported PageDefinition"), "{error}");
    }

    #[test]
    fn automatic_pagination_retains_original_page_while_expanding_in_every_direction() {
        let layout = crate::DocumentLayout {
            paper: crate::PaperSize {
                width: 100.0,
                height: 120.0,
            },
            page_origin: Some([30.0, 45.0]),
            ..crate::DocumentLayout::default()
        };
        let first = layout.resolve(Some([50.0, 70.0, 110.0, 140.0]));
        assert_eq!(first.origin, [30.0, 45.0]);
        assert_eq!(first.anchor_origin, [30.0, 45.0]);
        assert_eq!(first.prepended_pages, [0, 0]);
        let expanded = layout.resolve(Some([40.0, 60.0, 260.0, 250.0]));
        assert_eq!(expanded.origin, [30.0, 45.0]);
        assert_eq!(expanded.anchor_origin, [30.0, 45.0]);
        assert!(expanded.width_pages > first.width_pages);
        assert!(expanded.height_pages > first.height_pages);
        let expanded_left = layout.resolve(Some([-20.0, -90.0, 260.0, 250.0]));
        assert_eq!(expanded_left.origin, [-70.0, -195.0]);
        assert_eq!(expanded_left.anchor_origin, [30.0, 45.0]);
        assert_eq!(expanded_left.prepended_pages, [1, 2]);
    }

    #[test]
    fn cdxml_roundtrip_preserves_complete_document_layout() {
        let mut document = crate::ChemSemaDocument::blank();
        document.document.layout = crate::DocumentLayout {
            drawing_space: crate::DrawingSpace::Poster,
            paper: crate::PaperSize {
                width: 612.0,
                height: 792.0,
            },
            width_pages: 2,
            height_pages: 3,
            auto_paginate: false,
            page_origin: Some([42.0, 48.0]),
            margins: [30.0, 31.0, 32.0, 33.0],
            page_overlap: 12.0,
            print_trim_marks: true,
            header: "&lChemSema&c&f&r&p".to_string(),
            header_position: 24.0,
            footer: "&c&d &t".to_string(),
            footer_position: 25.0,
            magnification_percent: 150.0,
            page_definition: crate::PageDefinition::Reaction1,
            splitters: vec![
                crate::PageSplitter {
                    id: "301".to_string(),
                    position: Some([101.0, 202.0]),
                    page_definition: crate::PageDefinition::Center,
                },
                crate::PageSplitter {
                    id: "302".to_string(),
                    position: None,
                    page_definition: crate::PageDefinition::UserDefined,
                },
            ],
            legacy_splitter_position_ids: vec!["301".to_string(), "302".to_string()],
            fix_in_place_extent: Some([320.0, 240.0]),
            fix_in_place_gap: Some([8.0, 9.0]),
        };
        let cdxml = crate::document_to_cdxml(&document);
        for field in [
            "DrawingSpace=\"poster\"",
            "WidthPages=\"2\"",
            "HeightPages=\"3\"",
            "PageOverlap=\"12\"",
            "PrintTrimMarks=\"yes\"",
            "HeaderPosition=\"24\"",
            "FooterPosition=\"25\"",
            "Magnification=\"1500\"",
            "PageDefinition=\"Reaction1\"",
            "SplitterPositions=\"301 302\"",
            "<splitter id=\"301\" p=\"101 202\" PageDefinition=\"Center\"/>",
            "<splitter id=\"302\" PageDefinition=\"UserDefined\"/>",
            "FixInPlaceExtent=\"320 240\"",
            "FixInPlaceGap=\"8 9\"",
        ] {
            assert!(cdxml.contains(field), "missing {field}\n{cdxml}");
        }
        let reopened = crate::parse_cdxml_document(&cdxml, None).expect("roundtrip parses");
        let actual = reopened.document.layout;
        assert_eq!(actual.drawing_space, crate::DrawingSpace::Poster);
        assert_eq!(actual.width_pages, 2);
        assert_eq!(actual.height_pages, 3);
        assert_eq!(actual.margins, [30.0, 31.0, 32.0, 33.0]);
        assert_eq!(actual.page_overlap, 12.0);
        assert!(actual.print_trim_marks);
        assert_eq!(actual.header, "&lChemSema&c&f&r&p");
        assert_eq!(actual.footer, "&c&d &t");
        assert_eq!(actual.magnification_percent, 150.0);
        assert_eq!(actual.page_definition, crate::PageDefinition::Reaction1);
        assert_eq!(actual.splitters, document.document.layout.splitters);
        assert_eq!(
            actual.legacy_splitter_position_ids,
            vec!["301".to_string(), "302".to_string()]
        );
        assert_eq!(actual.fix_in_place_extent, Some([320.0, 240.0]));
        assert_eq!(actual.fix_in_place_gap, Some([8.0, 9.0]));
        assert_eq!(actual.page_origin, Some([42.0, 48.0]));

        let cdx = crate::document_to_cdx(&document).expect("layout CDX writes");
        let decoded_cdx = crate::cdx_to_cdxml(&cdx).expect("layout CDX decodes");
        let cdx_reopened = crate::parse_cdx_document(&cdx, None).expect("layout CDX reopens");
        let cdx_layout = cdx_reopened.document.layout;
        assert_eq!(
            cdx_layout.drawing_space,
            crate::DrawingSpace::Poster,
            "{decoded_cdx}"
        );
        assert_eq!(cdx_layout.width_pages, 2);
        assert_eq!(cdx_layout.height_pages, 3);
        assert_eq!(cdx_layout.page_overlap, 12.0);
        assert!(cdx_layout.print_trim_marks);
        assert_eq!(cdx_layout.header, "&lChemSema&c&f&r&p");
        assert_eq!(cdx_layout.footer, "&c&d &t");
        assert_eq!(cdx_layout.magnification_percent, 150.0);
        assert_eq!(cdx_layout.page_definition, crate::PageDefinition::Reaction1);
        assert_eq!(cdx_layout.splitters, document.document.layout.splitters);
        assert_eq!(
            cdx_layout.legacy_splitter_position_ids,
            vec!["301".to_string(), "302".to_string()]
        );
        assert_eq!(cdx_layout.fix_in_place_extent, Some([320.0, 240.0]));
        assert_eq!(cdx_layout.fix_in_place_gap, Some([8.0, 9.0]));
        assert_eq!(cdx_layout.page_origin, Some([42.0, 48.0]));
    }
}
