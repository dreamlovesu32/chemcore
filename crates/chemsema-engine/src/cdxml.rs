use crate::{
    Bond, BondLineStyles, BondLineWeights, BondStereo, ChemSemaDocument, DocumentInfo,
    DocumentLayout, DocumentStyleInfo, DocumentTextStyle, DoubleBond, DrawingSpace, FormatInfo,
    InterchangeDocument, InterchangeObject, InterchangeProperty, LabelRun, MoleculeFragment, Node,
    NodeLabel, ObjectPayload, Page, PaperSize, Resource, ResourceData, SceneObject, Transform,
    EPSILON,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod colors;
mod export;
mod import_bonds;
mod import_chemical_properties;
mod import_defaults;
mod import_fragments;
mod import_geometry_constraints;
mod import_groups;
mod import_logical_objects;
mod import_nodes;
mod import_objects;
mod import_scaling;
mod import_topology;
mod line_spacing;
mod parse_values;
mod template_library;
mod text_runs;
pub(crate) mod xml;

use self::colors::CdxmlColorTable;
pub use self::export::document_to_cdxml;
use self::import_bonds::*;
use self::import_chemical_properties::{import_chemical_properties, source_entity_map};
use self::import_defaults::*;
use self::import_fragments::*;
use self::import_geometry_constraints::{
    annotation_basis_links, append_geometry_constraint_objects,
    normalize_imported_annotation_displays,
};
use self::import_groups::*;
use self::import_logical_objects::import_logical_objects;
use self::import_nodes::*;
use self::import_objects::{
    append_bio_shape_objects, append_bracket_objects, append_curve_objects,
    append_embedded_image_objects, append_gel_electrophoresis_objects, append_line_objects,
    append_orbital_shape_objects, append_plasmid_map_objects, append_shape_objects,
    append_spectrum_objects, append_synthesized_enhanced_stereo_text_objects,
    append_table_shape_objects, append_text_objects, append_tlc_plate_shape_objects,
    associate_table_cell_contents, import_reactions_and_stoichiometry_grids,
    parse_cdxml_curve_points, validate_bio_shape_nodes,
};
pub(crate) use self::import_scaling::normalize_cdxml_document_for_editing;
use self::import_topology::*;
use self::line_spacing::*;
pub(crate) use self::parse_values::element_symbol;
use self::parse_values::*;
pub use self::template_library::{
    apply_template_library_layout_json, parse_cdxml_template_documents, template_document_icon_svg,
    template_library_layout_dialog_json, template_library_layout_json,
    template_library_palette_json, TemplateGridLayout,
};
use self::text_runs::{label_display_runs, label_display_runs_from_source_runs, label_source_run};
use self::xml::descendants;
pub(crate) use self::xml::parse_xml_tree;
pub(crate) use self::xml::XmlNode;

#[derive(Debug, Clone, Copy)]
struct CdxmlDefaults {
    bond_length: f64,
    line_width: f64,
    bold_width: f64,
    hash_spacing: f64,
    bond_spacing: f64,
    margin_width: f64,
    label_size: f64,
    caption_size: f64,
    chain_angle: f64,
    label_font: u32,
    caption_font: u32,
    label_face: u32,
    caption_face: u32,
    label_justification: CdxmlJustification,
    caption_justification: CdxmlJustification,
    line_height: Option<CdxmlLineHeight>,
    label_line_height: Option<CdxmlLineHeight>,
    caption_line_height: Option<CdxmlLineHeight>,
    fractional_widths: bool,
    interpret_chemically: Option<bool>,
    show_atom_query: bool,
    show_atom_stereo: bool,
    show_atom_enhanced_stereo: bool,
    show_atom_number: bool,
    show_residue_id: bool,
    show_bond_query: bool,
    show_bond_rxn: bool,
    show_bond_stereo: bool,
    show_terminal_carbon_labels: bool,
    show_non_terminal_carbon_labels: bool,
    hide_implicit_hydrogens: bool,
    print_margins: [f64; 4],
    color: u32,
}

impl Default for CdxmlDefaults {
    fn default() -> Self {
        Self {
            bond_length: crate::DEFAULT_BOND_LENGTH,
            line_width: crate::DEFAULT_BOND_STROKE,
            bold_width: crate::BOLD_BOND_WIDTH_PT.value(),
            hash_spacing: crate::DEFAULT_HASH_SPACING_PT.value(),
            bond_spacing: crate::DEFAULT_BOND_SPACING_PERCENT,
            margin_width: crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value(),
            label_size: crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT,
            caption_size: crate::DEFAULT_TEXT_FONT_SIZE_PT,
            chain_angle: 120.0,
            label_font: 3,
            caption_font: 3,
            // ChemDraw omits a zero-valued LabelFace when it normalizes CDXML.
            // Treat an entirely absent face as regular text; chemical/formula
            // styling must come from an inherited or run-level face value.
            label_face: 0,
            caption_face: 0,
            label_justification: CdxmlJustification::Auto,
            caption_justification: CdxmlJustification::Left,
            line_height: None,
            label_line_height: None,
            caption_line_height: None,
            fractional_widths: true,
            interpret_chemically: None,
            show_atom_query: true,
            show_atom_stereo: false,
            show_atom_enhanced_stereo: true,
            show_atom_number: false,
            show_residue_id: false,
            show_bond_query: true,
            show_bond_rxn: true,
            show_bond_stereo: false,
            show_terminal_carbon_labels: false,
            show_non_terminal_carbon_labels: false,
            hide_implicit_hydrogens: false,
            print_margins: [36.0, 36.0, 36.0, 36.0],
            color: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CdxmlJustification {
    Auto,
    Left,
    Center,
    Right,
    Full,
    Above,
    Below,
    Best,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CdxmlLineHeight {
    Variable,
    Auto,
    Fixed(f64),
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedCdxmlLineSpacing {
    line_height: f64,
    line_advances: Vec<f64>,
    mode: &'static str,
}

impl CdxmlJustification {
    fn as_cdxml(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
            Self::Full => "Full",
            Self::Above => "Above",
            Self::Below => "Below",
            Self::Best => "Best",
        }
    }
}

fn imported_document_text_style(
    font: u32,
    face: u32,
    size: f64,
    color: u32,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    line_height: CdxmlLineHeight,
) -> DocumentTextStyle {
    let font = font.to_string();
    let color = color.to_string();
    let run = label_source_run("", face, &font, &color, size, colors, fonts);
    let (line_height, line_height_mode) = match line_height {
        CdxmlLineHeight::Fixed(value) if value > 1.0 => (value, "fixed"),
        CdxmlLineHeight::Variable => (crate::molecule_label_line_advance(size), "variable"),
        _ => (chemdraw_auto_run_line_height(&run, size), "auto"),
    };
    DocumentTextStyle {
        font_family: run.font_family.unwrap_or_else(|| "Arial".to_string()),
        font_size: run.font_size.unwrap_or(size),
        fill: run.fill.unwrap_or_else(|| "#000000".to_string()),
        font_weight: run.font_weight.unwrap_or(400),
        font_style: run.font_style.unwrap_or_else(|| "normal".to_string()),
        underline: run.underline.unwrap_or(false),
        outline: run.outline.unwrap_or(false),
        shadow: run.shadow.unwrap_or(false),
        script: run.script.unwrap_or_else(|| "normal".to_string()),
        line_height: round2(line_height),
        line_height_mode: line_height_mode.to_string(),
    }
}

fn imported_document_layout(
    root: &XmlNode,
    defaults: CdxmlDefaults,
    mut content_page: Page,
) -> Result<(Page, DocumentLayout), String> {
    let page = root.children.iter().find(|child| child.name == "page");
    let width_pages = page
        .and_then(|page| parse_u32(page.attr("WidthPages")))
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(1);
    let height_pages = page
        .and_then(|page| parse_u32(page.attr("HeightPages")))
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(1);
    let drawing_space = match page
        .and_then(|page| page.attr("DrawingSpace"))
        .unwrap_or("pages")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "poster" | "1" => DrawingSpace::Poster,
        _ => DrawingSpace::Pages,
    };
    let page_overlap = page
        .and_then(|page| parse_f64(page.attr("PageOverlap")))
        .unwrap_or(0.0)
        .max(0.0);
    let total_width = page
        .and_then(|page| parse_f64(page.attr("Width")))
        .or_else(|| {
            page.and_then(|page| parse_bbox(page.attr("BoundingBox")))
                .map(|bbox| bbox[2] - bbox[0])
        })
        .unwrap_or(content_page.width)
        .max(1.0);
    let total_height = page
        .and_then(|page| parse_f64(page.attr("Height")))
        .or_else(|| {
            page.and_then(|page| parse_bbox(page.attr("BoundingBox")))
                .map(|bbox| bbox[3] - bbox[1])
        })
        .unwrap_or(content_page.height)
        .max(1.0);
    let paper_width = match drawing_space {
        DrawingSpace::Pages => total_width / f64::from(width_pages),
        DrawingSpace::Poster => {
            (total_width + page_overlap * f64::from(width_pages.saturating_sub(1)))
                / f64::from(width_pages)
        }
    }
    .max(1.0);
    let paper_height = match drawing_space {
        DrawingSpace::Pages => total_height / f64::from(height_pages),
        DrawingSpace::Poster => {
            (total_height + page_overlap * f64::from(height_pages.saturating_sub(1)))
                / f64::from(height_pages)
        }
    }
    .max(1.0);
    let magnification_percent = parse_f64(root.attr("Magnification"))
        .map(|value| value / 10.0)
        .filter(|value| (1.0..=999.0).contains(value))
        .unwrap_or(100.0);
    let legacy_splitter_position_ids = page
        .and_then(|page| page.attr("SplitterPositions"))
        .map(|value| {
            value
                .split_whitespace()
                .filter(|part| !part.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let splitters = page
        .into_iter()
        .flat_map(|page| page.direct_children("splitter"))
        .enumerate()
        .map(|(index, splitter)| {
            Ok(crate::PageSplitter {
                id: splitter
                    .attr("id")
                    .filter(|id| !id.trim().is_empty())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("page_splitter_{}", index + 1)),
                position: parse_xy(splitter.attr("p")),
                page_definition: crate::PageDefinition::from_cdxml(
                    splitter.attr("PageDefinition"),
                )?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let layout = DocumentLayout {
        drawing_space,
        paper: PaperSize {
            width: round2(paper_width),
            height: round2(paper_height),
        },
        width_pages,
        height_pages,
        auto_paginate: true,
        page_origin: page
            .and_then(|page| parse_bbox(page.attr("BoundingBox")))
            .map(|bounds| [bounds[0], bounds[1]]),
        margins: defaults.print_margins,
        page_overlap: round2(page_overlap),
        print_trim_marks: page
            .and_then(|page| parse_cdxml_bool(page.attr("PrintTrimMarks")))
            .unwrap_or(false),
        header: page
            .and_then(|page| page.attr("Header"))
            .unwrap_or("")
            .to_string(),
        header_position: page
            .and_then(|page| parse_f64(page.attr("HeaderPosition")))
            .unwrap_or(36.0)
            .max(0.0),
        footer: page
            .and_then(|page| page.attr("Footer"))
            .unwrap_or("")
            .to_string(),
        footer_position: page
            .and_then(|page| parse_f64(page.attr("FooterPosition")))
            .unwrap_or(36.0)
            .max(0.0),
        magnification_percent,
        page_definition: crate::PageDefinition::from_cdxml(
            page.and_then(|page| page.attr("PageDefinition")),
        )?,
        splitters,
        legacy_splitter_position_ids,
        fix_in_place_extent: parse_xy(root.attr("FixInPlaceExtent")),
        fix_in_place_gap: parse_xy(root.attr("FixInPlaceGap")),
    };
    content_page.width = content_page.width.max(layout.total_width());
    content_page.height = content_page.height.max(layout.total_height());
    Ok((content_page, layout))
}

pub fn parse_cdxml_document(cdxml: &str, title: Option<&str>) -> Result<ChemSemaDocument, String> {
    let mut root = parse_xml_tree(cdxml)?;
    normalize_repeated_text_objects(&mut root)?;
    validate_external_connection_values(&root)?;
    validate_bio_shape_nodes(&root)?;
    let source_tree = interchange_object_from_xml(&root);
    let defaults = cdxml_defaults(&root);
    let colors = CdxmlColorTable::from_cdxml(&root);
    let fonts = cdxml_font_table(&root);
    let mut styles = default_cdxml_styles(defaults);
    let mut resources = BTreeMap::new();
    let mut objects = Vec::new();

    let fragments = display_fragments(&root);
    let display_fragment_ids: BTreeSet<String> = fragments
        .iter()
        .filter_map(|fragment| fragment.attr("id").map(ToString::to_string))
        .collect();
    let bonded_node_ids = cdxml_bonded_node_ids(&root);
    let topology_only_cdxmlwriter = root.attr("CreationProgram") == Some("CDXMLWriter");
    let mut molecule_index = 1usize;
    for fragment in &fragments {
        let node_positions = cdxml_fragment_node_positions(
            fragment,
            defaults.bond_length,
            topology_only_cdxmlwriter,
        )?;
        let Some(bbox) = cdxml_fragment_bbox(fragment, defaults.bond_length, &node_positions)
        else {
            continue;
        };
        let Some(resource) =
            normalize_fragment(fragment, bbox, &node_positions, defaults, &colors, &fonts)?
        else {
            continue;
        };
        for component in split_cdxml_fragment_components(resource, bbox) {
            let resource_id = format!("mol_{:03}", molecule_index);
            let component_meta = cdxml_fragment_component_meta(
                fragment.attr("id"),
                component.component_index,
                component.component_count,
            );
            resources.insert(
                resource_id.clone(),
                Resource {
                    resource_type: "molecule_fragment2d".to_string(),
                    encoding: "chemsema.molecule.fragment2d".to_string(),
                    data: ResourceData::Fragment(component.fragment),
                    meta: component_meta.clone(),
                },
            );
            objects.push(SceneObject {
                id: format!("obj_mol_{:03}", molecule_index),
                object_type: "molecule".to_string(),
                name: format!("molecule {}", molecule_index),
                visible: true,
                locked: false,
                z_index: parse_i32(fragment.attr("Z")).unwrap_or(10),
                transform: Transform {
                    translate: [round2(component.bbox_abs[0]), round2(component.bbox_abs[1])],
                    rotate: 0.0,
                    scale: [1.0, 1.0],
                },
                style_ref: Some("style_molecule_default".to_string()),
                link_policy: Default::default(),
                meta: component_meta,
                payload: ObjectPayload {
                    resource_ref: Some(resource_id),
                    bbox: Some([
                        0.0,
                        0.0,
                        round2(component.bbox_abs[2] - component.bbox_abs[0]),
                        round2(component.bbox_abs[3] - component.bbox_abs[1]),
                    ]),
                    spectrum: None,
                    geometry: None,
                    constraint: None,
                    table: None,
                    stoichiometry_grid: None,
                    gel_electrophoresis: None,
                    plasmid_map: None,
                    bio_shape: None,
                    extra: BTreeMap::new(),
                },
                children: Vec::new(),
            });
            molecule_index += 1;
        }
    }
    let generic_root = root_without_plasmid_maps(&root);
    append_line_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_curve_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_shape_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_orbital_shape_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_table_shape_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_tlc_plate_shape_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_gel_electrophoresis_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_bio_shape_objects(&generic_root, &mut objects, &mut styles, defaults, &colors);
    append_plasmid_map_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_spectrum_objects(
        &generic_root,
        &mut objects,
        &mut styles,
        defaults,
        &colors,
        &fonts,
    )?;
    append_embedded_image_objects(&generic_root, &mut objects, &mut resources);
    append_bracket_objects(&generic_root, &mut objects, defaults, &colors);
    append_text_objects(
        &generic_root,
        &mut objects,
        &mut styles,
        defaults,
        &colors,
        &fonts,
        &display_fragment_ids,
        &bonded_node_ids,
    );
    append_synthesized_enhanced_stereo_text_objects(
        &generic_root,
        &mut objects,
        &mut styles,
        defaults,
        &colors,
        &fonts,
    );
    associate_table_cell_contents(&generic_root, &mut objects);
    append_geometry_constraint_objects(
        &generic_root,
        &mut objects,
        &resources,
        &mut styles,
        defaults,
        &colors,
        &fonts,
    );
    let reaction_schemes =
        import_reactions_and_stoichiometry_grids(&root, &mut objects, defaults, &colors, &fonts);
    let (chemical_properties, chemical_property_links) =
        import_chemical_properties(&root, &objects, &resources);
    let logical_objects =
        import_logical_objects(&root, &objects, &resources, &reaction_schemes, &colors);
    apply_cdxml_groups(&root, &mut objects);
    let label_style = imported_document_text_style(
        defaults.label_font,
        defaults.label_face,
        defaults.label_size,
        defaults.color,
        &colors,
        &fonts,
        defaults
            .label_line_height
            .or(defaults.line_height)
            .unwrap_or(CdxmlLineHeight::Variable),
    );
    let caption_style = imported_document_text_style(
        defaults.caption_font,
        defaults.caption_face,
        defaults.caption_size,
        defaults.color,
        &colors,
        &fonts,
        defaults
            .caption_line_height
            .or(defaults.line_height)
            .unwrap_or(CdxmlLineHeight::Auto),
    );
    let content_page = page_from_objects(&objects, colors.background());
    let (page, layout) = imported_document_layout(&root, defaults, content_page)?;
    let mut document = ChemSemaDocument {
        format: FormatInfo {
            name: "chemsema".to_string(),
            version: "0.1".to_string(),
            unit: "pt".to_string(),
        },
        document: DocumentInfo {
            id: "doc_cdxml_import".to_string(),
            title: title.unwrap_or("Imported CDXML").to_string(),
            page,
            layout,
            meta: json!({
                "createdBy": "chemsema",
                "sourceFormat": "cdxml",
                "nativeImport": true,
                "import": {
                    "cdxml": {
                        "defaults": {
                            "bondLength": defaults.bond_length,
                            "lineWidth": defaults.line_width,
                            "boldWidth": defaults.bold_width,
                            "hashSpacing": defaults.hash_spacing,
                            "bondSpacing": defaults.bond_spacing,
                            "marginWidth": defaults.margin_width,
                            "chainAngle": defaults.chain_angle,
                            "labelStyle": label_style,
                            "captionStyle": caption_style,
                            "labelJustification": defaults.label_justification.as_cdxml(),
                            "captionJustification": defaults.caption_justification.as_cdxml(),
                            "lineHeight": empty_as_null(root.attr("LineHeight")),
                            "labelLineHeight": empty_as_null(root.attr("LabelLineHeight")),
                            "captionLineHeight": empty_as_null(root.attr("CaptionLineHeight")),
                            "fractionalWidths": defaults.fractional_widths,
                            "interpretChemically": defaults.interpret_chemically,
                            "showAtomQuery": defaults.show_atom_query,
                            "showAtomStereo": defaults.show_atom_stereo,
                            "showAtomEnhancedStereo": defaults.show_atom_enhanced_stereo,
                            "showAtomNumber": defaults.show_atom_number,
                            "showResidueID": defaults.show_residue_id,
                            "showBondQuery": defaults.show_bond_query,
                            "showBondRxn": defaults.show_bond_rxn,
                            "showBondStereo": defaults.show_bond_stereo,
                            "showTerminalCarbonLabels": defaults.show_terminal_carbon_labels,
                            "showNonTerminalCarbonLabels": defaults.show_non_terminal_carbon_labels,
                            "hideImplicitHydrogens": defaults.hide_implicit_hydrogens,
                            "printMargins": defaults.print_margins,
                            "foregroundColor": colors.foreground(),
                        }
                    }
                },
            }),
        },
        style: DocumentStyleInfo {
            preset: "default".to_string(),
            defaults: BTreeMap::from([
                ("bondLength".to_string(), defaults.bond_length),
                ("chainAngle".to_string(), defaults.chain_angle),
                ("lineWidth".to_string(), defaults.line_width),
                ("boldWidth".to_string(), defaults.bold_width),
                (
                    "wedgeWidth".to_string(),
                    cdxml_import_wedge_width(defaults.line_width, defaults.bold_width),
                ),
                ("hashSpacing".to_string(), defaults.hash_spacing),
                ("bondSpacing".to_string(), defaults.bond_spacing),
                ("marginWidth".to_string(), defaults.margin_width),
                ("graphicLineWidth".to_string(), defaults.line_width),
            ]),
            label_style,
            caption_style,
        },
        styles,
        objects,
        links: Vec::new(),
        logical_objects,
        reaction_schemes,
        chemical_properties,
        resources,
        interchange: BTreeMap::from([(
            "cdxml".to_string(),
            InterchangeDocument {
                format: "cdxml".to_string(),
                version: root.attr("ChemDrawVersion").map(ToString::to_string),
                root: source_tree,
            },
        )]),
    };
    document
        .links
        .extend(annotation_basis_links(&document.objects));
    normalize_imported_annotation_displays(&mut document);
    document.links.extend(chemical_property_links);
    let linked_scene_ids = document
        .links
        .iter()
        .filter(|relation| relation.kind == "chemical-property-display")
        .flat_map(|relation| relation.endpoints.iter())
        .map(|endpoint| endpoint.entity_id.clone())
        .collect::<BTreeSet<_>>();
    for entity_id in linked_scene_ids {
        if let Some(object) = document.find_scene_object_mut(&entity_id) {
            object.link_policy = crate::LinkPolicy::Linked;
        }
    }
    crate::normalize_text_object_payloads(&mut document);
    crate::normalize_shape_object_payloads(&mut document);
    crate::normalize_arrow_object_payloads(&mut document);
    crate::normalize_fragment_label_payloads(&mut document);
    restore_authored_multiline_character_attachment_geometry(&mut document);
    infer_nmr_assignment_nucleus(&mut document);
    Ok(document)
}

fn infer_nmr_assignment_nucleus(document: &mut ChemSemaDocument) {
    let text = document
        .scene_objects()
        .into_iter()
        .filter(|object| object.object_type == "text")
        .filter_map(|object| object.payload.extra.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let nucleus = if text.contains("ChemNMR 13C Estimation")
        || text.contains("Protocol of the C-13 NMR Prediction")
    {
        crate::NmrNucleus::Carbon13
    } else if text.contains("ChemNMR 1H Estimation")
        || text.contains("Protocol of the H-1 NMR Prediction")
    {
        crate::NmrNucleus::Hydrogen1
    } else {
        return;
    };
    for resource in document.resources.values_mut() {
        let Some(fragment) = resource.data.as_fragment_mut() else {
            continue;
        };
        for assignment in fragment
            .nodes
            .iter_mut()
            .flat_map(|node| node.nmr_assignments.iter_mut())
        {
            if assignment.nucleus == crate::NmrNucleus::Unknown {
                assignment.nucleus = nucleus;
            }
        }
    }
}

fn restore_authored_multiline_character_attachment_geometry(document: &mut ChemSemaDocument) {
    fn collect_resource_origins(
        objects: &[SceneObject],
        parent: [f64; 2],
        origins: &mut BTreeMap<String, [f64; 2]>,
    ) {
        for object in objects {
            let origin = [
                parent[0] + object.transform.translate[0],
                parent[1] + object.transform.translate[1],
            ];
            if let Some(resource_ref) = object.payload.resource_ref.as_ref() {
                origins.insert(resource_ref.clone(), origin);
            }
            collect_resource_origins(&object.children, origin, origins);
        }
    }

    let mut origins = BTreeMap::new();
    collect_resource_origins(&document.objects, [0.0, 0.0], &mut origins);
    for (resource_id, resource) in &mut document.resources {
        let Some(origin) = origins.get(resource_id).copied() else {
            continue;
        };
        let Some(fragment) = resource.data.as_fragment_mut() else {
            continue;
        };
        for node in &mut fragment.nodes {
            let has_character_attachment = fragment.bonds.iter().any(|bond| {
                (bond.begin == node.id
                    && bond
                        .meta
                        .pointer("/endpointAttachments/begin/characterIndex")
                        .is_some())
                    || (bond.end == node.id
                        && bond
                            .meta
                            .pointer("/endpointAttachments/end/characterIndex")
                            .is_some())
            });
            let Some(label) = node.label.as_mut().filter(|label| {
                has_character_attachment
                    && label
                        .source_text
                        .as_deref()
                        .unwrap_or(&label.text)
                        .contains('\n')
            }) else {
                continue;
            };
            let imported = label.meta.pointer("/import/cdxml");
            let text_position = imported
                .and_then(|value| value.get("textPosition"))
                .and_then(Value::as_array)
                .filter(|values| values.len() >= 2)
                .and_then(|values| Some([values[0].as_f64()?, values[1].as_f64()?]));
            let imported_bbox = imported
                .and_then(|value| value.get("boundingBox"))
                .and_then(Value::as_array)
                .filter(|values| values.len() >= 4)
                .and_then(|values| {
                    Some([
                        values[0].as_f64()?,
                        values[1].as_f64()?,
                        values[2].as_f64()?,
                        values[3].as_f64()?,
                    ])
                });
            if let (Some(current), Some(authored)) = (label.position, text_position) {
                let target = [
                    round2(authored[0] - origin[0]),
                    round2(authored[1] - origin[1]),
                ];
                crate::translate_node_label_geometry(
                    label,
                    target[0] - current[0],
                    target[1] - current[1],
                );
                label.position = Some(target);
            }
            if let Some([x1, y1, x2, y2]) = imported_bbox {
                let bbox = [
                    round2(x1 - origin[0]),
                    round2(y1 - origin[1]),
                    round2(x2 - origin[0]),
                    round2(y2 - origin[1]),
                ];
                label.box_field = Some(bbox);
                label.box_value = Some(bbox);
            }
            let font_size = label
                .font_size
                .unwrap_or(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
            let margin_width = label
                .meta
                .pointer("/import/cdxml/marginWidth")
                .and_then(Value::as_f64)
                .unwrap_or(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value());
            let mut glyph_start = label.position.unwrap_or(node.position);
            if matches!(label.align.as_deref(), Some("right" | "center")) {
                if let Some(bbox) = label.bbox() {
                    glyph_start[0] = bbox[0];
                }
            }
            let geometry = crate::glyph_kernel::build_label_glyph_geometry_with_profile(
                if label.line_runs.is_empty() {
                    &label.runs
                } else {
                    &[]
                },
                &label.line_runs,
                glyph_start,
                label.bbox(),
                font_size,
                label
                    .line_height
                    .unwrap_or_else(|| crate::molecule_label_line_advance(font_size)),
                &label.line_advances,
                node.position,
                crate::GlyphClipProfile::from_margin_width(margin_width),
            );
            label.glyph_polygons = geometry.glyph_polygons;
            label.glyph_clip_polygons = geometry.clip_polygons;
        }
    }
}

const CDXML_EDITING_OUTPUT_SCALE: f64 = 1.0;

/// Resolve the position of a parent node from its embedded connection table.
/// CDXML permits nodes that own an embedded fragment to omit `p`: their
/// attachment position is then the external connection point of that fragment.
/// When that point also omits `p`, its incident bond continues the direction of
/// the adjacent, positioned bond by one document bond length.
///
/// Explicit compatibility rule for topology-only output emitted by
/// `CreationProgram="CDXMLWriter"`. Other CDXML producers must provide `n@p`.
///
#[derive(Debug)]
struct CdxmlFragmentComponent {
    fragment: MoleculeFragment,
    bbox_abs: [f64; 4],
    component_index: usize,
    component_count: usize,
}

fn root_without_plasmid_maps(root: &XmlNode) -> XmlNode {
    let mut copy = root.clone();
    remove_plasmid_map_children(&mut copy);
    copy
}

fn remove_plasmid_map_children(node: &mut XmlNode) {
    node.children.retain(|child| !child.is("plasmidmap"));
    for child in &mut node.children {
        remove_plasmid_map_children(child);
    }
}

#[cfg(test)]
mod interchange_tests {
    use super::*;

    #[test]
    fn repeated_text_id_parts_merge_once_and_empty_parts_are_no_ops() {
        let source = r#"<CDXML BondLength="14.4">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <t id="50" p="10 20" Justification="Left" InterpretChemically="no">
      <s font="3" size="10" face="0">first</s>
    </t>
    <t id="50" p="10 20" Justification="Left" InterpretChemically="no">
      <s font="3" size="10" face="2"> second</s>
    </t>
    <t id="50" p="10 20" Justification="Left" InterpretChemically="no"/>
  </page>
</CDXML>"#;
        let document = parse_cdxml_document(source, Some("repeated text"))
            .expect("compatible repeated text parts normalize");
        let texts = document
            .scene_objects()
            .into_iter()
            .filter(|object| object.kind() == crate::SceneObjectKind::Text)
            .collect::<Vec<_>>();
        assert_eq!(texts.len(), 1);
        assert_eq!(
            texts[0].payload.extra.get("text").and_then(Value::as_str),
            Some("first second")
        );
        assert_eq!(
            texts[0]
                .payload
                .extra
                .get("runs")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );

        let saved = document_to_cdxml(&document);
        let saved_root = parse_xml_tree(&saved).expect("exported CDXML parses");
        let saved_parts = descendants(&saved_root)
            .into_iter()
            .filter(|node| node.is("t") && node.attr("id") == Some("50"))
            .collect::<Vec<_>>();
        assert_eq!(saved_parts.len(), 1, "{saved}");
        assert_eq!(saved_parts[0].full_text().trim(), "first second");
        let reopened =
            parse_cdxml_document(&saved, Some("reopened")).expect("normalized export reopens");
        assert_eq!(
            reopened
                .scene_objects()
                .into_iter()
                .filter(|object| object.kind() == crate::SceneObjectKind::Text)
                .count(),
            1
        );
    }

    #[test]
    fn repeated_text_id_with_conflicting_object_geometry_is_rejected() {
        let source = r#"<CDXML><page id="1">
          <t id="50" p="10 20"><s>first</s></t>
          <t id="50" p="30 40"><s>second</s></t>
        </page></CDXML>"#;
        let error = parse_cdxml_document(source, Some("conflict"))
            .expect_err("one object id cannot identify two text geometries");
        assert!(error.contains("text id '50'"));
        assert!(error.contains("conflicting 'p' values"));
    }

    #[test]
    fn native_text_replaces_same_identity_text_inside_transparent_fragment() {
        let source = r#"<CDXML><page id="1">
          <fragment id="4"><t id="5" p="20 30"><s>X2</s></t></fragment>
        </page></CDXML>"#;
        let document =
            parse_cdxml_document(source, Some("wrapped text")).expect("wrapped text imports");
        assert_eq!(
            document
                .scene_objects()
                .into_iter()
                .filter(|object| object.kind() == crate::SceneObjectKind::Text)
                .count(),
            1
        );

        let saved = document_to_cdxml(&document);
        let root = parse_xml_tree(&saved).expect("saved CDXML parses");
        assert_eq!(
            descendants(&root)
                .into_iter()
                .filter(|node| node.is("t") && node.attr("id") == Some("5"))
                .count(),
            1,
            "{saved}"
        );
        let reopened =
            parse_cdxml_document(&saved, Some("wrapped text reopened")).expect("text reopens");
        assert_eq!(
            reopened
                .scene_objects()
                .into_iter()
                .filter(|object| object.kind() == crate::SceneObjectKind::Text)
                .count(),
            1
        );
    }

    #[test]
    fn synthesized_enhanced_stereo_display_is_not_an_independent_text_object() {
        let source = r#"<CDXML ShowAtomEnhancedStereo="yes"><page id="1">
          <fragment id="4">
            <n id="5" p="20 30" EnhancedStereoType="Absolute"/>
          </fragment>
        </page></CDXML>"#;
        let document =
            parse_cdxml_document(source, Some("derived stereo")).expect("stereo imports");
        assert_eq!(
            document
                .scene_objects()
                .into_iter()
                .filter(|object| {
                    object.meta.get("synthetic").and_then(Value::as_bool) == Some(true)
                })
                .count(),
            1
        );

        let saved = document_to_cdxml(&document);
        assert!(saved.contains("EnhancedStereoType=\"Absolute\""), "{saved}");
        assert!(!saved.contains(">abs</s>"), "{saved}");
        let reopened =
            parse_cdxml_document(&saved, Some("derived stereo reopened")).expect("stereo reopens");
        assert_eq!(
            reopened
                .scene_objects()
                .into_iter()
                .filter(|object| {
                    object.meta.get("synthetic").and_then(Value::as_bool) == Some(true)
                })
                .count(),
            1
        );
    }

    #[test]
    fn singleton_placeholder_fragment_stays_a_molecule_for_every_producer() {
        let source = r#"<CDXML CreationProgram="ChemDraw 23" BondLength="14.4">
          <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
          <page id="1">
            <fragment id="20" BoundingBox="10 10 25 20">
              <n id="21" p="12 15" NodeType="Unspecified" Warning="Parentheses don't match.">
                <t p="10 18" BoundingBox="10 10 25 20"><s font="3" size="10">(</s></t>
              </n>
            </fragment>
          </page>
        </CDXML>"#;
        let document =
            parse_cdxml_document(source, Some("singleton")).expect("singleton fragment imports");
        assert_eq!(
            document
                .scene_objects()
                .iter()
                .filter(|object| object.kind() == crate::SceneObjectKind::Molecule)
                .count(),
            1
        );
        assert!(!document
            .scene_objects()
            .iter()
            .any(|object| object.kind() == crate::SceneObjectKind::Text));
        let fragment = document.resources["mol_001"]
            .data
            .as_fragment()
            .expect("molecule resource");
        assert_eq!(fragment.nodes.len(), 1);
        assert_eq!(
            fragment.nodes[0]
                .label
                .as_ref()
                .map(|label| label.text.as_str()),
            Some("(")
        );

        let saved = document_to_cdxml(&document);
        let reopened =
            parse_cdxml_document(&saved, Some("reopened")).expect("singleton export reopens");
        assert_eq!(
            reopened
                .scene_objects()
                .iter()
                .filter(|object| object.kind() == crate::SceneObjectKind::Molecule)
                .count(),
            1
        );
        assert!(!reopened
            .scene_objects()
            .iter()
            .any(|object| object.kind() == crate::SceneObjectKind::Text));
    }

    #[test]
    fn unpositioned_fragment_wrapper_exposes_its_positioned_embedded_fragment() {
        let source = r#"<CDXML BondLength="14.4"><page id="1">
          <fragment id="20">
            <n id="21" NodeType="Fragment">
              <fragment id="30">
                <n id="31" p="10 10"/>
                <n id="32" p="24.4 10"/>
                <b id="33" B="31" E="32"/>
              </fragment>
              <t><s>Et</s></t>
            </n>
          </fragment>
        </page></CDXML>"#;
        let document =
            parse_cdxml_document(source, Some("wrapper")).expect("embedded fragment imports");
        let molecules = document
            .scene_objects()
            .into_iter()
            .filter(|object| object.kind() == crate::SceneObjectKind::Molecule)
            .collect::<Vec<_>>();
        assert_eq!(molecules.len(), 1);
        assert_eq!(
            molecules[0].meta.get("fragmentId").and_then(Value::as_str),
            Some("30")
        );
        let fragment = molecules[0]
            .payload
            .resource_ref
            .as_ref()
            .and_then(|id| document.resources.get(id))
            .and_then(|resource| resource.data.as_fragment())
            .expect("embedded molecule resource");
        assert_eq!(fragment.nodes.len(), 2);
        assert_eq!(fragment.bonds.len(), 1);

        let saved = document_to_cdxml(&document);
        assert_eq!(
            saved.matches("id=\"30\"").count(),
            1,
            "the regenerated display fragment must not also survive inside its transparent wrapper: {saved}"
        );
        assert!(
            saved.contains("id=\"20\"") && saved.contains(">Et</s>"),
            "unmodeled wrapper metadata remains available without duplicating its native fragment: {saved}"
        );
        let reopened =
            parse_cdxml_document(&saved, Some("reopened wrapper")).expect("saved CDXML reopens");
        let reopened_molecules = reopened
            .scene_objects()
            .into_iter()
            .filter(|object| object.kind() == crate::SceneObjectKind::Molecule)
            .collect::<Vec<_>>();
        assert_eq!(reopened_molecules.len(), 1);
        let reopened_fragment = reopened_molecules[0]
            .payload
            .resource_ref
            .as_ref()
            .and_then(|id| reopened.resources.get(id))
            .and_then(|resource| resource.data.as_fragment())
            .expect("reopened embedded molecule resource");
        assert_eq!(reopened_fragment.nodes.len(), 2);
        assert_eq!(reopened_fragment.bonds.len(), 1);
        let saved_again = document_to_cdxml(&reopened);
        assert_eq!(saved_again.matches("id=\"30\"").count(), 1);
    }

    #[test]
    fn cdxml_unmodeled_official_fields_and_objects_roundtrip_through_ccjs() {
        let source = r#"<CDXML CreationProgram="ChemDraw 23" CreationDate="20260723090000" BoundingBox="0 0 120 80">
  <page id="1" BoundingBox="0 0 120 80" Width="120" Height="80">
    <annotation id="2" Keyword="source" Content="confidential" />
  </page>
</CDXML>"#;
        let mut document = parse_cdxml_document(source, Some("fields")).expect("CDXML parses");
        let tree = document
            .interchange
            .get_mut("cdxml")
            .expect("source tree is stored");
        assert_eq!(tree.root.properties["CreationDate"].value, "20260723090000");
        tree.root.properties.get_mut("CreationDate").unwrap().value = "20260723100000".to_string();

        let saved = document_to_cdxml(&document);
        assert!(saved.contains("CreationDate=\"20260723100000\""));
        assert!(saved.contains("<annotation"));
        assert!(saved.contains("Keyword=\"source\""));
        assert!(saved.contains("Content=\"confidential\""));
    }

    #[test]
    fn native_constraint_imports_as_one_live_annotation_and_roundtrips_through_cdx() {
        let source = r#"<CDXML BondLength="14.4" LineWidth="0.6" HashSpacing="2.5">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="10"><n id="101" p="40 40"/><n id="102" p="60 40"/><b id="103" B="101" E="102"/></fragment>
    <constraint id="201" ConstraintType="Distance" ConstraintMin="0" ConstraintMax="0" BasisObjects="101 102">
      <objecttag TagType="Unknown" Name="distance"><t p="44.37 37.23"><s font="3" size="7.5" color="0">0 Å</s></t></objecttag>
    </constraint>
  </page>
</CDXML>"#;
        let document = parse_cdxml_document(source, Some("distance")).expect("constraint parses");
        let constraints = document
            .scene_objects()
            .into_iter()
            .filter(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .collect::<Vec<_>>();
        assert_eq!(constraints.len(), 1);
        assert!(
            constraints[0]
                .payload
                .constraint
                .as_ref()
                .expect("constraint payload")
                .display
                .auto_value
        );
        assert_eq!(
            constraints[0]
                .payload
                .constraint
                .as_ref()
                .expect("constraint payload")
                .display
                .positioning_type,
            crate::AnnotationPositioningType::Auto
        );
        assert!(!document
            .scene_objects()
            .iter()
            .any(|object| object.kind() == crate::SceneObjectKind::Text));
        let saved = document_to_cdxml(&document);
        assert!(saved.contains("<constraint"));
        assert!(saved.contains(">0 Å</s>"));
        assert!(!saved.contains("PositioningType=\"auto\""));

        let cdx = crate::document_to_cdx(&document).expect("constraint exports to CDX");
        let reopened = crate::parse_cdx_document(&cdx, Some("distance")).expect("CDX reopens");
        let reopened_constraint = reopened
            .scene_objects()
            .into_iter()
            .find(|object| object.kind() == crate::SceneObjectKind::Constraint)
            .expect("constraint survives CDX");
        assert_eq!(
            reopened_constraint
                .payload
                .constraint
                .as_ref()
                .expect("constraint payload")
                .basis_entity_ids
                .len(),
            2
        );
    }

    #[test]
    fn edited_constraint_text_imports_as_an_explicit_non_updating_value() {
        let source = r#"<CDXML BondLength="14.4"><page id="1">
          <fragment id="10"><n id="101" p="40 40"/><n id="102" p="60 40"/></fragment>
          <constraint id="201" ConstraintType="Distance" ConstraintMin="0" ConstraintMax="0" BasisObjects="101 102">
            <objecttag TagType="Unknown" Name="distance"><t p="50 35"><s>custom</s></t></objecttag>
          </constraint>
        </page></CDXML>"#;
        let document = parse_cdxml_document(source, Some("edited")).expect("constraint parses");
        let constraint = document
            .scene_objects()
            .into_iter()
            .find_map(|object| object.payload.constraint.as_ref())
            .expect("constraint payload");
        assert!(!constraint.display.auto_value);
        assert_eq!(constraint.display.text_override.as_deref(), Some("custom"));
        let saved = document_to_cdxml(&document);
        assert!(saved.contains(">custom</s>"), "{saved}");
    }
}
