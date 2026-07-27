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
            "splitterPositions",
            "fixInPlaceExtent",
            "fixInPlaceGap",
        ] {
            assert!(data.get(key).is_some(), "missing {key}");
        }
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
            splitter_positions: vec![101.0, 202.0],
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
            "SplitterPositions=\"101 202\"",
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
        assert_eq!(actual.splitter_positions, vec![101.0, 202.0]);
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
        assert_eq!(cdx_layout.splitter_positions, vec![101.0, 202.0]);
        assert_eq!(cdx_layout.fix_in_place_extent, Some([320.0, 240.0]));
        assert_eq!(cdx_layout.fix_in_place_gap, Some([8.0, 9.0]));
        assert_eq!(cdx_layout.page_origin, Some([42.0, 48.0]));
    }
}
