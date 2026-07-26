use super::*;

pub(super) fn document_node<'a>(document: &'a ChemSemaDocument, node_id: &str) -> Option<&'a Node> {
    document.resources.values().find_map(|resource| {
        resource
            .data
            .as_fragment()
            .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == node_id))
    })
}

pub(super) fn preserved_cdxml_bond_order(bond: &Bond) -> Option<String> {
    let source = bond
        .meta
        .pointer("/import/cdxml/order")
        .and_then(Value::as_str)?;
    let aromatic = bond
        .meta
        .pointer("/import/cdxml/aromatic")
        .and_then(Value::as_bool)
        == Some(true);
    if (aromatic && source == "1.5") || (bond.order == 1 && source.eq_ignore_ascii_case("dative")) {
        Some(source.to_string())
    } else {
        None
    }
}

pub(super) fn collect_document_colors(document: &ChemSemaDocument, colors: &mut CdxmlColorTable) {
    colors.ensure(&document.document.page.background);
    colors.ensure(&document.style.label_style.fill);
    colors.ensure(&document.style.caption_style.fill);
    if let Some(foreground) = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/foregroundColor")
        .and_then(Value::as_str)
    {
        colors.ensure(foreground);
    }
    for style in document.styles.values() {
        for key in ["stroke", "fill", "color", "background", "backgroundColor"] {
            if let Some(color) = style_nullable_string_value(style, key) {
                colors.ensure(&color);
            }
        }
    }
    for object in &document.objects {
        if let Some(style) = object_style(document, object) {
            for key in ["stroke", "fill", "color"] {
                if let Some(color) = style_nullable_string_value(style, key) {
                    colors.ensure(&color);
                }
            }
        }
        if object.object_type == "text" {
            if let Some(runs) = object
                .payload
                .extra
                .get("runs")
                .cloned()
                .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            {
                for run in runs {
                    if let Some(fill) = run.fill {
                        colors.ensure(&fill);
                    }
                }
            }
        }
        if let Some(table) = object.payload.table.as_ref() {
            colors.ensure(&table.default_border.color);
            for cell in &table.cells {
                for border in [
                    cell.borders.top.as_ref(),
                    cell.borders.left.as_ref(),
                    cell.borders.bottom.as_ref(),
                    cell.borders.right.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    colors.ensure(&border.color);
                }
            }
        }
    }
    for resource in document.resources.values() {
        let Some(fragment) = resource.data.as_fragment() else {
            continue;
        };
        for node in &fragment.nodes {
            if let Some(color) = &node.highlight_color {
                colors.ensure(color);
            }
            for label in node.label.iter().chain(
                node.nmr_assignments
                    .iter()
                    .map(|assignment| &assignment.label),
            ) {
                if let Some(fill) = &label.fill {
                    colors.ensure(fill);
                }
                for run in &label.runs {
                    if let Some(fill) = &run.fill {
                        colors.ensure(fill);
                    }
                }
            }
        }
        for bond in &fragment.bonds {
            if let Some(color) = &bond.highlight_color {
                colors.ensure(color);
            }
            if let Some(stroke) = &bond.stroke {
                colors.ensure(stroke);
            }
        }
        for area in &fragment.colored_areas {
            colors.ensure(&area.color);
        }
    }
}

pub(super) fn collect_document_fonts(document: &ChemSemaDocument, fonts: &mut CdxmlFontTable) {
    fonts.ensure(&document.style.label_style.font_family);
    fonts.ensure(&document.style.caption_style.font_family);
    for style in document.styles.values() {
        if let Some(font_family) = style_string_value(style, "fontFamily") {
            fonts.ensure(&font_family);
        }
    }
    for object in &document.objects {
        if object.object_type == "text" {
            if let Some(runs) = object
                .payload
                .extra
                .get("runs")
                .cloned()
                .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            {
                for run in runs {
                    if let Some(font_family) = run.font_family {
                        fonts.ensure(&font_family);
                    }
                }
            }
        }
    }
    for resource in document.resources.values() {
        let Some(fragment) = resource.data.as_fragment() else {
            continue;
        };
        for node in &fragment.nodes {
            for label in node.label.iter().chain(
                node.nmr_assignments
                    .iter()
                    .map(|assignment| &assignment.label),
            ) {
                if let Some(font_family) = &label.font_family {
                    fonts.ensure(font_family);
                }
                for run in &label.runs {
                    if let Some(font_family) = &run.font_family {
                        fonts.ensure(font_family);
                    }
                }
            }
        }
    }
}
