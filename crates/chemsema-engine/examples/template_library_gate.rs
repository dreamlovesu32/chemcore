use chemsema_engine::{
    cdx_to_cdxml, cdxml_to_cdx, document_to_cdxml, parse_cdxml_template_documents, render_document,
    render_primitives_bounds, template_document_icon_svg, template_library_layout_json,
    TemplateGridLayout,
};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    schema: String,
    library_count: usize,
    template_count: usize,
    libraries: Vec<Library>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Library {
    name: String,
    path: String,
    template_count: usize,
    layout: CatalogLayout,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogLayout {
    rows: u16,
    columns: u16,
    pane_height: f64,
    extent: [f64; 2],
    occupied_cells: usize,
    empty_cells: usize,
}

fn main() -> Result<(), String> {
    let root = PathBuf::from(
        env::args()
            .nth(1)
            .unwrap_or_else(|| "viewer/template-libraries".to_string()),
    );
    let catalog_path = root.join("catalog.json");
    let catalog: Catalog = serde_json::from_str(
        &fs::read_to_string(&catalog_path)
            .map_err(|error| format!("read {}: {error}", catalog_path.display()))?,
    )
    .map_err(|error| format!("parse {}: {error}", catalog_path.display()))?;

    if catalog.schema != "chemsema.template-library-catalog.v2" {
        return Err(format!("unsupported catalog schema {}", catalog.schema));
    }
    if catalog.library_count != catalog.libraries.len() {
        return Err(format!(
            "catalog libraryCount={} but libraries.len()={}",
            catalog.library_count,
            catalog.libraries.len()
        ));
    }

    let mut parsed_total = 0usize;
    let mut primitive_total = 0usize;
    for library in &catalog.libraries {
        let path = resolve_library_path(&root, &library.path);
        let cdxml = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let layout: TemplateGridLayout = serde_json::from_str(
            &template_library_layout_json(&cdxml)
                .map_err(|error| format!("parse grid {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("decode grid {}: {error}", path.display()))?;
        assert_catalog_layout(library, &layout)?;
        let binary_roundtrip = cdx_to_cdxml(
            &cdxml_to_cdx(&cdxml)
                .map_err(|error| format!("encode CTP semantics {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("decode CTP semantics {}: {error}", path.display()))?;
        let binary_layout: TemplateGridLayout = serde_json::from_str(
            &template_library_layout_json(&binary_roundtrip)
                .map_err(|error| format!("parse roundtrip grid {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("decode roundtrip grid {}: {error}", path.display()))?;
        if binary_layout != layout {
            return Err(format!(
                "{} changed TemplateGrid semantics during CDXML/CDX/CDXML roundtrip: {:?} -> {:?}",
                library.name, layout, binary_layout
            ));
        }
        let documents = parse_cdxml_template_documents(&cdxml, Some(&library.name))
            .map_err(|error| format!("parse {}: {error}", path.display()))?;
        if documents.len() != library.template_count {
            return Err(format!(
                "{} declares {} templates but parsed {}",
                library.name,
                library.template_count,
                documents.len()
            ));
        }

        for (index, document) in documents.iter().enumerate() {
            let primitives = render_document(document);
            if primitives.is_empty() || render_primitives_bounds(primitives.iter()).is_none() {
                return Err(format!(
                    "{} template {} has no bounded render primitives",
                    library.name,
                    index + 1
                ));
            }
            let icon = template_document_icon_svg(document);
            if !icon.contains("cc-kernel-template-icon") || !icon.contains("viewBox=") {
                return Err(format!(
                    "{} template {} did not produce a kernel icon",
                    library.name,
                    index + 1
                ));
            }
            serde_json::to_string(document).map_err(|error| {
                format!(
                    "{} template {} cannot be stored as CCJS: {error}",
                    library.name,
                    index + 1
                )
            })?;
            let exported = document_to_cdxml(document);
            let source_counts = document
                .interchange
                .get("cdxml")
                .map(|source| interchange_object_counts(&source.root))
                .ok_or_else(|| {
                    format!(
                        "{} template {} omitted its CDXML interchange source",
                        library.name,
                        index + 1
                    )
                })?;
            let exported_counts = template_object_counts(&exported);
            if !template_object_counts_preserved(source_counts, exported_counts) {
                let failure_path = PathBuf::from("target/template-library-gate-failure.cdxml");
                fs::write(&failure_path, &exported)
                    .map_err(|error| format!("write {}: {error}", failure_path.display()))?;
                let differences = TEMPLATE_OBJECT_TAGS
                    .iter()
                    .zip(source_counts)
                    .zip(exported_counts)
                    .filter(|((_, source), roundtrip)| source != roundtrip)
                    .map(|((tag, source), roundtrip)| format!("{tag}={source}/{roundtrip}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!(
                    "{} template {} source/CCJS/CDXML semantic object counts differ: {differences}; exported {}",
                    library.name, index + 1, failure_path.display()
                ));
            }
            primitive_total += primitives.len();
        }
        parsed_total += documents.len();
        println!(
            "[TEMPLATE-GATE] {}: {} templates",
            library.name,
            documents.len()
        );
    }

    if parsed_total != catalog.template_count {
        return Err(format!(
            "catalog templateCount={} but parsed total={parsed_total}",
            catalog.template_count
        ));
    }
    println!(
        "[TEMPLATE-GATE] PASS libraries={} templates={} primitives={primitive_total}",
        catalog.libraries.len(),
        parsed_total
    );
    Ok(())
}

fn assert_catalog_layout(library: &Library, layout: &TemplateGridLayout) -> Result<(), String> {
    let declared = &library.layout;
    if layout.rows != declared.rows
        || layout.columns != declared.columns
        || (layout.pane_height - declared.pane_height).abs() > 1e-9
        || layout.extent != declared.extent
    {
        return Err(format!(
            "{} catalog and kernel TemplateGrid fields differ",
            library.name
        ));
    }
    let occupied = layout.cells.iter().flatten().count();
    let empty = layout.cells.iter().filter(|cell| cell.is_none()).count();
    if occupied != library.template_count
        || occupied != declared.occupied_cells
        || empty != declared.empty_cells
        || occupied + empty != layout.capacity()
    {
        return Err(format!(
            "{} has inconsistent grid occupancy: occupied={occupied} empty={empty} capacity={}",
            library.name,
            layout.capacity()
        ));
    }
    Ok(())
}

fn template_object_counts_preserved(
    source: [usize; TEMPLATE_OBJECT_TAGS.len()],
    roundtrip: [usize; TEMPLATE_OBJECT_TAGS.len()],
) -> bool {
    TEMPLATE_OBJECT_TAGS.iter().enumerate().all(|(index, tag)| {
        if *tag == "t" {
            // Carbon labels may be implicit in CDXML and become explicit
            // text on export. Existing text objects must never disappear.
            roundtrip[index] >= source[index]
        } else {
            roundtrip[index] == source[index]
        }
    })
}

const TEMPLATE_OBJECT_TAGS: [&str; 9] = [
    "fragment", "n", "b", "t", "graphic", "curve", "arrow", "bioshape", "group",
];

fn template_object_counts(cdxml: &str) -> [usize; TEMPLATE_OBJECT_TAGS.len()] {
    std::array::from_fn(|index| {
        let tag = TEMPLATE_OBJECT_TAGS[index];
        if tag == "curve" {
            xml_start_tag_count_with_attribute(cdxml, tag, "CurvePoints")
        } else {
            xml_start_tag_count(cdxml, tag)
        }
    })
}

fn interchange_object_counts(
    root: &chemsema_engine::InterchangeObject,
) -> [usize; TEMPLATE_OBJECT_TAGS.len()] {
    let mut counts = [0usize; TEMPLATE_OBJECT_TAGS.len()];
    accumulate_interchange_object_counts(root, &mut counts);
    counts
}

fn accumulate_interchange_object_counts(
    object: &chemsema_engine::InterchangeObject,
    counts: &mut [usize; TEMPLATE_OBJECT_TAGS.len()],
) {
    if let Some(index) = TEMPLATE_OBJECT_TAGS
        .iter()
        .position(|tag| *tag == object.name)
    {
        if object.name != "curve" || object.properties.contains_key("CurvePoints") {
            counts[index] += 1;
        }
    }
    for child in &object.children {
        accumulate_interchange_object_counts(child, counts);
    }
}

fn xml_start_tag_count(xml: &str, tag: &str) -> usize {
    let needle = format!("<{tag}");
    xml.match_indices(&needle)
        .filter(|(index, _)| {
            xml.as_bytes()
                .get(index + needle.len())
                .is_some_and(|next| next.is_ascii_whitespace() || matches!(*next, b'/' | b'>'))
        })
        .count()
}

fn xml_start_tag_count_with_attribute(xml: &str, tag: &str, attribute: &str) -> usize {
    let needle = format!("<{tag}");
    xml.match_indices(&needle)
        .filter(|(index, _)| {
            let start = index + needle.len();
            xml.as_bytes()
                .get(start)
                .is_some_and(|next| next.is_ascii_whitespace() || matches!(*next, b'/' | b'>'))
                && xml[start..]
                    .find('>')
                    .is_some_and(|end| xml[start..start + end].contains(&format!("{attribute}=")))
        })
        .count()
}

fn resolve_library_path(root: &Path, catalog_path: &str) -> PathBuf {
    let file_name = Path::new(catalog_path)
        .file_name()
        .expect("template library catalog path must have a file name");
    root.join(file_name)
}
