use super::{parse_cdxml_document, parse_xml_tree, XmlNode};
use crate::{
    primitives_to_svg_viewbox, render_document, render_primitives_bounds, ChemSemaDocument,
};
use serde_json::json;
use std::fmt::Write;

const ICON_PAD_RATIO: f64 = 0.08;
const EMPTY_ICON_SIZE: f64 = 24.0;

/// Splits a ChemDraw template-library CDXML document into one native document
/// per root page. ChemDraw stores every palette item as a page; nested pages
/// (for example table cells) remain part of their containing template.
pub fn parse_cdxml_template_documents(
    cdxml: &str,
    title: Option<&str>,
) -> Result<Vec<ChemSemaDocument>, String> {
    let root = parse_xml_tree(cdxml)?;
    if root.name != "CDXML" {
        return Err("template library root must be CDXML".to_string());
    }
    // ChemDraw pads some CTP libraries with self-closing, childless pages.
    // They are document-layout pages, not palette entries, and ChemDraw does
    // not expose them in the template window.
    let pages = root
        .direct_children("page")
        .filter(|page| !page.children.is_empty() || !page.text.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let pages = if pages.is_empty() {
        root.direct_children("templategrid")
            .flat_map(|grid| grid.direct_children("page"))
            .filter(|page| !page.children.is_empty() || !page.text.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>()
    } else {
        pages
    };
    if pages.is_empty() {
        return Err("template library contains no root template pages".to_string());
    }

    pages
        .into_iter()
        .enumerate()
        .map(|(index, page)| {
            let source_name = template_page_name(&page);
            let mut item_root = root.clone();
            item_root.children = root
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
    let documents = parse_cdxml_template_documents(cdxml, Some(library_name))?;
    let templates = documents
        .iter()
        .enumerate()
        .map(|(index, document)| {
            Ok(json!({
                "id": format!("{library_id}:{:03}", index + 1),
                "index": index,
                "label": document.document.title,
                "iconSvg": template_document_icon_svg(document),
                "documentJson": serde_json::to_string(document)
                    .map_err(|error| error.to_string())?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "type": "template-library-palette",
        "library": {
            "id": library_id,
            "name": library_name,
            "templateCount": templates.len(),
        },
        "templates": templates,
    })
    .to_string())
}

fn template_page_name(page: &XmlNode) -> Option<String> {
    page.direct_children("annotation")
        .find(|annotation| annotation.attr("Keyword") == Some("Name"))
        .and_then(|annotation| annotation.attr("Content"))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
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
        let source = r#"<CDXML BondLength="14.4"><page id="1"><fragment id="2"><n id="3" p="0 0"/><n id="4" p="14.4 0"/><b id="5" B="3" E="4"/></fragment></page></CDXML>"#;
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
}
