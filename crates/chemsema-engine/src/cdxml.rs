use crate::{
    Bond, BondLineStyles, BondLineWeights, BondStereo, ChemSemaDocument, DocumentInfo,
    DocumentStyleInfo, DocumentTextStyle, DoubleBond, FormatInfo, InterchangeDocument,
    InterchangeObject, InterchangeProperty, LabelRun, MoleculeFragment, Node, NodeLabel,
    ObjectPayload, Page, Resource, ResourceData, SceneObject, Transform, EPSILON,
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
mod import_nodes;
mod import_objects;
mod import_scaling;
mod import_topology;
mod line_spacing;
mod parse_values;
mod text_runs;
pub(crate) mod xml;

use self::colors::CdxmlColorTable;
pub use self::export::document_to_cdxml;
use self::import_bonds::*;
use self::import_chemical_properties::import_chemical_properties;
use self::import_defaults::*;
use self::import_fragments::*;
use self::import_geometry_constraints::{
    annotation_basis_links, append_geometry_constraint_objects,
    normalize_imported_annotation_displays,
};
use self::import_groups::*;
use self::import_nodes::*;
use self::import_objects::{
    append_bracket_objects, append_curve_objects, append_embedded_image_objects,
    append_gel_electrophoresis_objects, append_line_objects, append_orbital_shape_objects,
    append_shape_objects, append_spectrum_objects, append_synthesized_enhanced_stereo_text_objects,
    append_table_shape_objects, append_text_objects, append_tlc_plate_shape_objects,
    associate_table_cell_contents, import_reactions_and_stoichiometry_grids,
};
pub(crate) use self::import_scaling::normalize_cdxml_document_for_editing;
use self::import_topology::*;
use self::line_spacing::*;
pub(crate) use self::parse_values::element_symbol;
use self::parse_values::*;
use self::text_runs::{label_display_runs, label_display_runs_from_source_runs, label_source_run};
pub(crate) use self::xml::parse_xml_tree;
use self::xml::{descendants, XmlNode};

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

pub fn parse_cdxml_document(cdxml: &str, title: Option<&str>) -> Result<ChemSemaDocument, String> {
    let root = parse_xml_tree(cdxml)?;
    validate_external_connection_values(&root)?;
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
                    extra: BTreeMap::new(),
                },
                children: Vec::new(),
            });
            molecule_index += 1;
        }
    }
    append_line_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_curve_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_orbital_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_table_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_tlc_plate_shape_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_gel_electrophoresis_objects(&root, &mut objects, &mut styles, defaults, &colors);
    append_spectrum_objects(&root, &mut objects, &mut styles, defaults, &colors, &fonts)?;
    append_embedded_image_objects(&root, &mut objects, &mut resources);
    append_bracket_objects(&root, &mut objects, defaults, &colors);
    append_text_objects(
        &root,
        &mut objects,
        &mut styles,
        defaults,
        &colors,
        &fonts,
        &display_fragment_ids,
        &bonded_node_ids,
    );
    append_synthesized_enhanced_stereo_text_objects(
        &root,
        &mut objects,
        &mut styles,
        defaults,
        &colors,
        &fonts,
    );
    associate_table_cell_contents(&root, &mut objects);
    append_geometry_constraint_objects(
        &root,
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
    let mut document = ChemSemaDocument {
        format: FormatInfo {
            name: "chemsema".to_string(),
            version: "0.1".to_string(),
            unit: "pt".to_string(),
        },
        document: DocumentInfo {
            id: "doc_cdxml_import".to_string(),
            title: title.unwrap_or("Imported CDXML").to_string(),
            page: page_from_objects(&objects, colors.background()),
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

#[cfg(test)]
mod interchange_tests {
    use super::*;

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
        assert!(document_to_cdxml(&document).contains(">custom</s>"));
    }
}
