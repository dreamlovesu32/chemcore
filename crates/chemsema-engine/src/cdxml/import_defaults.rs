use super::*;

pub(super) fn cdxml_defaults(root: &XmlNode) -> CdxmlDefaults {
    let defaults = CdxmlDefaults::default();
    CdxmlDefaults {
        bond_length: parse_f64(root.attr("BondLength")).unwrap_or(crate::DEFAULT_BOND_LENGTH),
        line_width: parse_f64(root.attr("LineWidth")).unwrap_or(crate::DEFAULT_BOND_STROKE),
        bold_width: parse_f64(root.attr("BoldWidth")).unwrap_or(crate::BOLD_BOND_WIDTH_PT.value()),
        hash_spacing: parse_f64(root.attr("HashSpacing"))
            .unwrap_or(crate::DEFAULT_HASH_SPACING_PT.value()),
        bond_spacing: parse_f64(root.attr("BondSpacing"))
            .unwrap_or(crate::DEFAULT_BOND_SPACING_PERCENT),
        margin_width: parse_f64(root.attr("MarginWidth"))
            .unwrap_or(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()),
        label_size: parse_f64(root.attr("LabelSize"))
            .unwrap_or(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT),
        caption_size: parse_f64(root.attr("CaptionSize"))
            .unwrap_or(crate::DEFAULT_TEXT_FONT_SIZE_PT),
        chain_angle: parse_f64(root.attr("ChainAngle")).unwrap_or(defaults.chain_angle),
        label_font: parse_u32(root.attr("LabelFont")).unwrap_or(defaults.label_font),
        caption_font: parse_u32(root.attr("CaptionFont")).unwrap_or(defaults.caption_font),
        label_face: parse_u32(root.attr("LabelFace")).unwrap_or(defaults.label_face),
        caption_face: parse_u32(root.attr("CaptionFace")).unwrap_or(defaults.caption_face),
        label_justification: parse_cdxml_justification(root.attr("LabelJustification"))
            .unwrap_or(defaults.label_justification),
        caption_justification: parse_cdxml_justification(root.attr("CaptionJustification"))
            .unwrap_or(defaults.caption_justification),
        line_height: parse_cdxml_line_height(root.attr("LineHeight")),
        label_line_height: parse_cdxml_line_height(root.attr("LabelLineHeight")),
        caption_line_height: parse_cdxml_line_height(root.attr("CaptionLineHeight")),
        fractional_widths: parse_cdxml_bool(root.attr("FractionalWidths"))
            .unwrap_or(defaults.fractional_widths),
        interpret_chemically: parse_cdxml_bool(root.attr("InterpretChemically")),
        show_atom_query: parse_cdxml_bool(root.attr("ShowAtomQuery"))
            .unwrap_or(defaults.show_atom_query),
        show_atom_stereo: parse_cdxml_bool(root.attr("ShowAtomStereo"))
            .unwrap_or(defaults.show_atom_stereo),
        show_atom_enhanced_stereo: parse_cdxml_bool(root.attr("ShowAtomEnhancedStereo"))
            .unwrap_or(defaults.show_atom_enhanced_stereo),
        show_atom_number: parse_cdxml_bool(root.attr("ShowAtomNumber"))
            .unwrap_or(defaults.show_atom_number),
        show_residue_id: parse_cdxml_bool(root.attr("ShowResidueID"))
            .unwrap_or(defaults.show_residue_id),
        show_bond_query: parse_cdxml_bool(root.attr("ShowBondQuery"))
            .unwrap_or(defaults.show_bond_query),
        show_bond_rxn: parse_cdxml_bool(root.attr("ShowBondRxn")).unwrap_or(defaults.show_bond_rxn),
        show_bond_stereo: parse_cdxml_bool(root.attr("ShowBondStereo"))
            .unwrap_or(defaults.show_bond_stereo),
        show_terminal_carbon_labels: parse_cdxml_bool(root.attr("ShowTerminalCarbonLabels"))
            .unwrap_or(defaults.show_terminal_carbon_labels),
        show_non_terminal_carbon_labels: parse_cdxml_bool(root.attr("ShowNonTerminalCarbonLabels"))
            .unwrap_or(defaults.show_non_terminal_carbon_labels),
        hide_implicit_hydrogens: parse_cdxml_bool(root.attr("HideImplicitHydrogens"))
            .unwrap_or(defaults.hide_implicit_hydrogens),
        print_margins: parse_cdxml_margins(root.attr("PrintMargins"))
            .unwrap_or(defaults.print_margins),
        color: parse_u32(root.attr("color")).unwrap_or(defaults.color),
    }
}

pub(super) fn parse_cdxml_bool(value: Option<&str>) -> Option<bool> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

pub(super) fn parse_cdxml_justification(value: Option<&str>) -> Option<CdxmlJustification> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(CdxmlJustification::Auto),
        "left" | "start" => Some(CdxmlJustification::Left),
        "center" | "middle" => Some(CdxmlJustification::Center),
        "right" | "end" => Some(CdxmlJustification::Right),
        "full" => Some(CdxmlJustification::Full),
        "above" => Some(CdxmlJustification::Above),
        "below" => Some(CdxmlJustification::Below),
        "best" => Some(CdxmlJustification::Best),
        _ => None,
    }
}

pub(super) fn parse_cdxml_margins(value: Option<&str>) -> Option<[f64; 4]> {
    let parts: Vec<f64> = value?
        .split_whitespace()
        .take(4)
        .filter_map(|part| part.parse().ok())
        .collect();
    (parts.len() == 4).then_some([parts[0], parts[1], parts[2], parts[3]])
}

pub(super) fn default_cdxml_styles(defaults: CdxmlDefaults) -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "style_molecule_default".to_string(),
            json!({
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": defaults.line_width,
                "fontFamily": "Arial",
                "fontSize": defaults.label_size,
            }),
        ),
        (
            "style_text_default".to_string(),
            json!({
                "kind": "text",
                "fontFamily": "Arial",
                "fontSize": defaults.caption_size,
                "fontWeight": 400,
                "fill": "#000000",
                "stroke": null,
            }),
        ),
        (
            "style_arrow_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": defaults.line_width,
                "lineCap": "butt",
                "lineJoin": "miter",
                "dashArray": [],
            }),
        ),
        (
            "style_line_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": defaults.line_width,
                "lineCap": "butt",
                "lineJoin": "miter",
                "dashArray": [],
            }),
        ),
    ])
}

pub(super) fn cdxml_font_table(root: &XmlNode) -> BTreeMap<String, String> {
    let mut fonts = BTreeMap::from([("3".to_string(), "Arial".to_string())]);
    if let Some(table) = descendants(root)
        .into_iter()
        .find(|node| node.is("fonttable"))
    {
        for font in table.direct_children("font") {
            if let (Some(id), Some(name)) = (font.attr("id"), font.attr("name")) {
                fonts.insert(id.to_string(), name.to_string());
            }
        }
    }
    fonts
}

pub(super) fn display_fragments(root: &XmlNode) -> Vec<&XmlNode> {
    let mut fragments = Vec::new();
    collect_display_fragments(root, false, &mut fragments);
    fragments
}

pub(super) fn collect_display_fragments<'a>(
    node: &'a XmlNode,
    inside_display_fragment: bool,
    fragments: &mut Vec<&'a XmlNode>,
) {
    let is_display_fragment = !inside_display_fragment && cdxml_node_is_display_fragment(node);
    if is_display_fragment {
        fragments.push(node);
    }
    let next_inside_display_fragment = inside_display_fragment || is_display_fragment;
    for child in &node.children {
        collect_display_fragments(child, next_inside_display_fragment, fragments);
    }
}

pub(super) fn cdxml_node_is_display_fragment(node: &XmlNode) -> bool {
    if !node.is("fragment") {
        return false;
    }
    let has_bond = node.direct_children("b").next().is_some();
    let has_node = node.direct_children("n").next().is_some();
    has_bond || has_node
}
