use super::{parse_cdxml_document, parse_xml_tree, XmlNode};
use crate::{
    primitives_to_svg_viewbox, render_document, render_primitives_bounds, ChemSemaDocument,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeSet, fmt::Write};

const ICON_PAD_RATIO: f64 = 0.08;
const EMPTY_ICON_SIZE: f64 = 24.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateGridLayout {
    pub rows: u16,
    pub columns: u16,
    pub pane_height: f64,
    pub extent: [f64; 2],
    pub cells: Vec<Option<usize>>,
}

impl TemplateGridLayout {
    pub fn capacity(&self) -> usize {
        usize::from(self.rows) * usize::from(self.columns)
    }

    pub fn validate(&self, template_count: usize) -> Result<(), String> {
        if self.rows == 0 || self.columns == 0 {
            return Err("template grid rows and columns must be positive".to_string());
        }
        if !self.pane_height.is_finite() || self.pane_height <= 0.0 {
            return Err("template grid paneHeight must be a positive finite number".to_string());
        }
        if self
            .extent
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err("template grid extent values must be positive finite numbers".to_string());
        }
        if self.cells.len() != self.capacity() {
            return Err(format!(
                "template grid has {} cells but rows × columns requires {}",
                self.cells.len(),
                self.capacity()
            ));
        }
        let mut seen = vec![false; template_count];
        for index in self.cells.iter().flatten().copied() {
            if index >= template_count {
                return Err(format!(
                    "template grid cell refers to template {index}, but only {template_count} templates exist"
                ));
            }
            if std::mem::replace(&mut seen[index], true) {
                return Err(format!(
                    "template grid refers to template {index} more than once"
                ));
            }
        }
        if let Some(index) = seen.iter().position(|seen| !seen) {
            return Err(format!("template grid does not contain template {index}"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ParsedTemplateLibrary {
    root: XmlNode,
    grid: XmlNode,
    pages: Vec<XmlNode>,
    layout: TemplateGridLayout,
}

/// Splits a ChemDraw template-library CDXML document into one native document
/// per root page. ChemDraw stores every palette item as a page; nested pages
/// (for example table cells) remain part of their containing template.
pub fn parse_cdxml_template_documents(
    cdxml: &str,
    title: Option<&str>,
) -> Result<Vec<ChemSemaDocument>, String> {
    let parsed = parse_template_library(cdxml)?;
    parsed
        .pages
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, page)| {
            let source_name = template_page_name(&page);
            let mut item_root = parsed.root.clone();
            item_root.children = parsed
                .root
                .children
                .iter()
                .filter(|child| child.name != "page" && child.name != "templategrid")
                .cloned()
                .chain(std::iter::once(page))
                .collect();
            let source = xml_tree_to_string(&item_root);
            let item_title = source_name.unwrap_or_else(|| {
                title
                    .map(|title| format!("{title} {}", index + 1))
                    .unwrap_or_else(|| format!("Template {}", index + 1))
            });
            parse_cdxml_document(&source, Some(&item_title))
        })
        .collect()
}

pub fn template_document_icon_svg(document: &ChemSemaDocument) -> String {
    let primitives = render_document(document);
    let Some([min_x, min_y, max_x, max_y]) = render_primitives_bounds(primitives.iter()) else {
        return primitives_to_svg_viewbox(
            &[],
            [0.0, 0.0, EMPTY_ICON_SIZE, EMPTY_ICON_SIZE],
            Some("chemsema-icon cc-kernel-template-icon"),
        );
    };
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let side = width.max(height) * (1.0 + ICON_PAD_RATIO * 2.0);
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    primitives_to_svg_viewbox(
        &primitives,
        [center_x - side * 0.5, center_y - side * 0.5, side, side],
        Some("chemsema-icon cc-kernel-template-icon"),
    )
}

pub fn template_library_palette_json(
    library_id: &str,
    library_name: &str,
    cdxml: &str,
) -> Result<String, String> {
    let parsed = parse_template_library(cdxml)?;
    let documents = parse_cdxml_template_documents(cdxml, Some(library_name))?;
    let templates = documents
        .iter()
        .zip(parsed.pages.iter())
        .enumerate()
        .map(|(index, (document, page))| {
            Ok(json!({
                "id": template_page_id(library_id, page, index)?,
                "index": index,
                "label": document.document.title,
                "iconSvg": template_document_icon_svg(document),
                "documentJson": serde_json::to_string(document)
                    .map_err(|error| error.to_string())?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let cells = parsed
        .layout
        .cells
        .iter()
        .map(|cell| {
            cell.map(|index| template_page_id(library_id, &parsed.pages[index], index))
                .transpose()
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "schema": "chemsema.template-library.v1",
        "type": "template-library-palette",
        "library": {
            "id": library_id,
            "name": library_name,
            "templateCount": templates.len(),
            "layout": {
                "rows": parsed.layout.rows,
                "columns": parsed.layout.columns,
                "paneHeight": parsed.layout.pane_height,
                "extent": parsed.layout.extent,
                "cells": cells,
                "readingOrder": "row-major",
            },
        },
        "templates": templates,
    })
    .to_string())
}

pub fn template_library_layout_json(cdxml: &str) -> Result<String, String> {
    let parsed = parse_template_library(cdxml)?;
    serde_json::to_string(&parsed.layout).map_err(|error| error.to_string())
}

pub fn template_library_layout_dialog_json(cdxml: &str) -> Result<String, String> {
    let parsed = parse_template_library(cdxml)?;
    Ok(json!({
        "type": "template-library-layout-dialog",
        "title": "Template Library Layout",
        "data": parsed.layout,
        "fields": [
            {"key": "rows", "label": "Rows", "kind": "integer", "minimum": 1, "maximum": 256},
            {"key": "columns", "label": "Columns", "kind": "integer", "minimum": 1, "maximum": 256},
            {"key": "paneHeight", "label": "Pane height", "kind": "number", "minimumExclusive": 0, "unit": "CDX coordinate"},
            {"key": "extent.0", "label": "Cell width", "kind": "number", "minimumExclusive": 0, "unit": "CDX coordinate"},
            {"key": "extent.1", "label": "Cell height", "kind": "number", "minimumExclusive": 0, "unit": "CDX coordinate"}
        ],
        "templateCount": parsed.pages.len(),
        "readingOrder": "row-major",
    })
    .to_string())
}

pub fn apply_template_library_layout_json(
    cdxml: &str,
    layout_json: &str,
) -> Result<String, String> {
    let parsed = parse_template_library(cdxml)?;
    let layout: TemplateGridLayout =
        serde_json::from_str(layout_json).map_err(|error| error.to_string())?;
    layout.validate(parsed.pages.len())?;

    let mut grid = parsed.grid;
    grid.attrs
        .insert("NumRows".to_string(), layout.rows.to_string());
    grid.attrs
        .insert("NumColumns".to_string(), layout.columns.to_string());
    grid.attrs.insert(
        "PaneHeight".to_string(),
        format_cdxml_number(layout.pane_height),
    );
    grid.attrs.insert(
        "extent".to_string(),
        format!(
            "{} {}",
            format_cdxml_number(layout.extent[0]),
            format_cdxml_number(layout.extent[1])
        ),
    );
    // TemplateGrid is EMPTY in the official CDXML content model. Older files
    // with nested pages are normalized to the same root-page order as CTP.
    grid.children.clear();
    grid.text.clear();

    let mut replacement = layout
        .cells
        .iter()
        .map(|cell| {
            cell.map(|index| parsed.pages[index].clone())
                .unwrap_or_else(|| XmlNode {
                    name: "page".to_string(),
                    ..XmlNode::default()
                })
        })
        .collect::<Vec<_>>();
    replacement.push(grid);

    let mut root = parsed.root;
    let mut inserted = false;
    let mut children = Vec::with_capacity(root.children.len() + replacement.len());
    for child in root.children {
        if child.name == "page" || child.name == "templategrid" {
            if !inserted {
                children.append(&mut replacement);
                inserted = true;
            }
        } else {
            children.push(child);
        }
    }
    if !inserted {
        children.append(&mut replacement);
    }
    root.children = children;
    Ok(xml_tree_to_string(&root))
}

fn parse_template_library(cdxml: &str) -> Result<ParsedTemplateLibrary, String> {
    let root = parse_xml_tree(cdxml)?;
    if root.name != "CDXML" {
        return Err("template library root must be CDXML".to_string());
    }
    let grids = root.direct_children("templategrid").collect::<Vec<_>>();
    if grids.len() != 1 {
        return Err(format!(
            "template library must contain exactly one root templategrid; found {}",
            grids.len()
        ));
    }
    let grid = grids[0].clone();
    let rows = parse_positive_u16_attr(&grid, "NumRows")?;
    let columns = parse_positive_u16_attr(&grid, "NumColumns")?;
    let pane_height = parse_positive_number_attr(&grid, "PaneHeight")?;
    let extent = parse_positive_pair_attr(&grid, "extent")?;
    let capacity = usize::from(rows) * usize::from(columns);

    let direct_pages = root.direct_children("page").cloned().collect::<Vec<_>>();
    let source_pages = if direct_pages.is_empty() {
        grid.direct_children("page").cloned().collect::<Vec<_>>()
    } else {
        direct_pages
    };
    if source_pages.len() > capacity {
        return Err(format!(
            "template library contains {} page slots but its {} × {} grid has capacity {}",
            source_pages.len(),
            rows,
            columns,
            capacity
        ));
    }

    let mut pages = Vec::new();
    let mut cells = Vec::with_capacity(capacity);
    for page in source_pages {
        if page_is_empty(&page) {
            cells.push(None);
        } else {
            let index = pages.len();
            pages.push(page);
            cells.push(Some(index));
        }
    }
    cells.resize(capacity, None);
    if pages.is_empty() {
        return Err("template library contains no non-empty template pages".to_string());
    }
    let mut page_ids = BTreeSet::new();
    for (index, page) in pages.iter().enumerate() {
        let page_id = page
            .attr("id")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                format!(
                    "template library page {} is missing the stable page id required for editing",
                    index + 1
                )
            })?;
        if !page_ids.insert(page_id) {
            return Err(format!(
                "template library contains duplicate page id {page_id}"
            ));
        }
    }
    let layout = TemplateGridLayout {
        rows,
        columns,
        pane_height,
        extent,
        cells,
    };
    layout.validate(pages.len())?;
    Ok(ParsedTemplateLibrary {
        root,
        grid,
        pages,
        layout,
    })
}

fn page_is_empty(page: &XmlNode) -> bool {
    page.children.is_empty() && page.text.trim().is_empty()
}

fn parse_positive_u16_attr(node: &XmlNode, key: &str) -> Result<u16, String> {
    let value = node
        .attr(key)
        .ok_or_else(|| format!("templategrid is missing required {key}"))?
        .parse::<u16>()
        .map_err(|_| format!("templategrid {key} must be a positive INT16 value"))?;
    if value == 0 {
        return Err(format!("templategrid {key} must be positive"));
    }
    Ok(value)
}

fn parse_positive_number_attr(node: &XmlNode, key: &str) -> Result<f64, String> {
    let value = node
        .attr(key)
        .ok_or_else(|| format!("templategrid is missing required {key}"))?
        .parse::<f64>()
        .map_err(|_| format!("templategrid {key} must be numeric"))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(format!(
            "templategrid {key} must be a positive finite number"
        ));
    }
    Ok(value)
}

fn parse_positive_pair_attr(node: &XmlNode, key: &str) -> Result<[f64; 2], String> {
    let values = node
        .attr(key)
        .ok_or_else(|| format!("templategrid is missing required {key}"))?
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("templategrid {key} must contain two numbers"))?;
    if values.len() != 2
        || values
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(format!(
            "templategrid {key} must contain two positive finite numbers"
        ));
    }
    Ok([values[0], values[1]])
}

fn format_cdxml_number(value: f64) -> String {
    let mut value = format!("{value:.8}");
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    value
}

fn template_page_name(page: &XmlNode) -> Option<String> {
    page.direct_children("annotation")
        .find(|annotation| annotation.attr("Keyword") == Some("Name"))
        .and_then(|annotation| annotation.attr("Content"))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn template_page_id(library_id: &str, page: &XmlNode, index: usize) -> Result<String, String> {
    let page_id = page
        .attr("id")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "template library page {} is missing the stable page id required for editing",
                index + 1
            )
        })?;
    Ok(format!("{library_id}:page-{page_id}"))
}

fn xml_tree_to_string(root: &XmlNode) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_xml_node(&mut out, root);
    out
}

fn write_xml_node(out: &mut String, node: &XmlNode) {
    write!(out, "<{}", node.name).expect("write XML tag");
    for (key, value) in &node.attrs {
        write!(out, " {}=\"{}\"", key, escape_xml_attribute(value)).expect("write XML attribute");
    }
    if node.children.is_empty() && node.text.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    if !node.text.is_empty() {
        out.push_str(&escape_xml_text(&node.text));
    }
    for child in &node.children {
        write_xml_node(out, child);
    }
    write!(out, "</{}>", node.name).expect("write XML close tag");
}

fn escape_xml_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_pages_become_independent_template_documents() {
        let source = r#"<CDXML BondLength="14.4">
          <fonttable><font id="3" name="Arial" charset="iso-8859-1"/></fonttable>
          <page id="10" BoundingBox="0 0 80 80">
            <fragment id="11"><n id="12" p="10 10"/><n id="13" p="24.4 10"/><b id="14" B="12" E="13"/></fragment>
          </page>
          <page id="20" BoundingBox="0 0 80 80">
            <fragment id="21"><n id="22" p="10 10"/><n id="23" p="24.4 10"/><n id="24" p="17.2 22.47"/><b id="25" B="22" E="23"/><b id="26" B="23" E="24"/><b id="27" B="24" E="22"/></fragment>
          </page>
          <templategrid NumRows="1" NumColumns="2" PaneHeight="80" extent="80 80"/>
        </CDXML>"#;
        let documents = parse_cdxml_template_documents(source, Some("Test")).unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0].resources.len(), 1);
        assert_eq!(documents[1].resources.len(), 1);
        assert_eq!(
            documents[0]
                .resources
                .values()
                .next()
                .and_then(|resource| resource.data.as_fragment())
                .map(|fragment| fragment.bonds.len()),
            Some(1)
        );
        assert_eq!(
            documents[1]
                .resources
                .values()
                .next()
                .and_then(|resource| resource.data.as_fragment())
                .map(|fragment| fragment.bonds.len()),
            Some(3)
        );
    }

    #[test]
    fn template_icons_are_square_kernel_renders() {
        let source = r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2"><n id="3" p="0 0"/><n id="4" p="14.4 0"/><b id="5" B="3" E="4"/></fragment></page><templategrid NumRows="1" NumColumns="1" PaneHeight="24" extent="24 24"/></CDXML>"#;
        let document = parse_cdxml_template_documents(source, None)
            .unwrap()
            .remove(0);
        let svg = template_document_icon_svg(&document);
        assert!(svg.contains("cc-kernel-template-icon"));
        assert!(
            svg.contains("<line") || svg.contains("<polygon") || svg.contains("<path"),
            "{svg}"
        );
        let view_box = svg
            .split("viewBox=\"")
            .nth(1)
            .and_then(|value| value.split('"').next())
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>();
        assert_eq!(view_box[2], view_box[3]);
    }

    #[test]
    fn grid_layout_preserves_empty_cells_and_stable_page_ownership() {
        let source = r#"<CDXML>
          <page id="10"><fragment id="11"><n id="12" p="0 0"/></fragment></page>
          <page/>
          <page id="20"><fragment id="21"><n id="22" p="20 0"/></fragment></page>
          <templategrid NumRows="2" NumColumns="3" PaneHeight="25.25" extent="2.75 3.5" VendorFlag="kept"/>
        </CDXML>"#;
        let layout: TemplateGridLayout =
            serde_json::from_str(&template_library_layout_json(source).unwrap()).unwrap();
        assert_eq!(layout.rows, 2);
        assert_eq!(layout.columns, 3);
        assert_eq!(layout.pane_height, 25.25);
        assert_eq!(layout.extent, [2.75, 3.5]);
        assert_eq!(layout.cells, vec![Some(0), None, Some(1), None, None, None]);

        let moved = TemplateGridLayout {
            rows: 2,
            columns: 2,
            pane_height: 30.5,
            extent: [4.0, 5.0],
            cells: vec![Some(1), None, Some(0), None],
        };
        let edited =
            apply_template_library_layout_json(source, &serde_json::to_string(&moved).unwrap())
                .unwrap();
        assert!(edited.contains("VendorFlag=\"kept\""));
        assert!(edited.contains("NumRows=\"2\""));
        assert!(edited.contains("NumColumns=\"2\""));
        assert!(edited.contains("PaneHeight=\"30.5\""));
        let palette: serde_json::Value =
            serde_json::from_str(&template_library_palette_json("test", "Test", &edited).unwrap())
                .unwrap();
        assert_eq!(palette["library"]["layout"]["cells"][0], "test:page-20");
        assert!(palette["library"]["layout"]["cells"][1].is_null());
        assert_eq!(palette["library"]["layout"]["cells"][2], "test:page-10");
        assert!(palette["library"]["layout"]["cells"][3].is_null());
    }

    #[test]
    fn grid_layout_rejects_implicit_loss_duplicate_ownership_and_bad_dimensions() {
        let source = r#"<CDXML>
          <page id="10"><fragment id="11"><n id="12" p="0 0"/></fragment></page>
          <page id="20"><fragment id="21"><n id="22" p="20 0"/></fragment></page>
          <templategrid NumRows="1" NumColumns="2" PaneHeight="20" extent="3 3"/>
        </CDXML>"#;
        let duplicate = TemplateGridLayout {
            rows: 1,
            columns: 2,
            pane_height: 20.0,
            extent: [3.0, 3.0],
            cells: vec![Some(0), Some(0)],
        };
        assert!(apply_template_library_layout_json(
            source,
            &serde_json::to_string(&duplicate).unwrap()
        )
        .unwrap_err()
        .contains("more than once"));

        let too_small = source.replace("NumColumns=\"2\"", "NumColumns=\"1\"");
        assert!(template_library_layout_json(&too_small)
            .unwrap_err()
            .contains("capacity"));
        let missing_grid = source.replace(
            r#"<templategrid NumRows="1" NumColumns="2" PaneHeight="20" extent="3 3"/>"#,
            "",
        );
        assert!(template_library_layout_json(&missing_grid)
            .unwrap_err()
            .contains("exactly one"));
        let duplicate_page_id = source.replace(r#"page id="20""#, r#"page id="10""#);
        assert!(template_library_layout_json(&duplicate_page_id)
            .unwrap_err()
            .contains("duplicate page id"));
    }

    #[test]
    fn grid_cell_ownership_survives_cdx_roundtrip() {
        let source = r#"<CDXML>
          <page id="10"><fragment id="11"><n id="12" p="0 0"/></fragment></page>
          <page id="20"><fragment id="21"><n id="22" p="20 0"/></fragment></page>
          <templategrid NumRows="2" NumColumns="2" PaneHeight="25.25" extent="2.75 2.75"/>
        </CDXML>"#;
        let moved = TemplateGridLayout {
            rows: 2,
            columns: 2,
            pane_height: 25.25,
            extent: [2.75, 2.75],
            cells: vec![Some(1), None, Some(0), None],
        };
        let edited =
            apply_template_library_layout_json(source, &serde_json::to_string(&moved).unwrap())
                .unwrap();
        let cdx = crate::cdxml_to_cdx(&edited).unwrap();
        let roundtrip = crate::cdx_to_cdxml(&cdx).unwrap();
        let palette: serde_json::Value = serde_json::from_str(
            &template_library_palette_json("test", "Test", &roundtrip).unwrap(),
        )
        .unwrap();
        assert_eq!(palette["library"]["layout"]["rows"], 2);
        assert_eq!(palette["library"]["layout"]["columns"], 2);
        assert_eq!(palette["library"]["layout"]["paneHeight"], 25.25);
        assert_eq!(palette["library"]["layout"]["extent"], json!([2.75, 2.75]));
        assert_eq!(palette["library"]["layout"]["cells"][0], "test:page-20");
        assert!(palette["library"]["layout"]["cells"][1].is_null());
        assert_eq!(palette["library"]["layout"]["cells"][2], "test:page-10");
        assert!(palette["library"]["layout"]["cells"][3].is_null());
    }
}
