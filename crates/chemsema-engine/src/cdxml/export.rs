use crate::{
    Bond, ChemSemaDocument, DocumentTextStyle, LabelRun, MoleculeFragment, Node, NodeLabel,
    ObjectPayload, Point, ResourceData, SceneObject,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn plasmid_map_point(center: Point, radius: f64, angle_degrees: f64) -> Point {
    let radians = angle_degrees.to_radians();
    Point::new(
        center.x + radius * radians.sin(),
        center.y - radius * radians.cos(),
    )
}
use std::fmt::Write;

fn exported_external_connection_type(value: crate::ExternalConnectionType) -> Option<&'static str> {
    match value {
        crate::ExternalConnectionType::Unspecified => None,
        crate::ExternalConnectionType::Diamond => Some("Diamond"),
        crate::ExternalConnectionType::Star => Some("Star"),
        crate::ExternalConnectionType::PolymerBead => Some("PolymerBead"),
        crate::ExternalConnectionType::Wavy => Some("Wavy"),
        crate::ExternalConnectionType::Residue => Some("Residue"),
        crate::ExternalConnectionType::Peptide => Some("Peptide"),
        crate::ExternalConnectionType::Dna => Some("DNA"),
        crate::ExternalConnectionType::Rna => Some("RNA"),
        crate::ExternalConnectionType::Terminus => Some("Terminus"),
        crate::ExternalConnectionType::Sulfide => Some("Sulfide"),
        crate::ExternalConnectionType::Nucleotide => Some("Nucleotide"),
        crate::ExternalConnectionType::UnlinkedBranch => Some("UnlinkedBranch"),
    }
}

mod defaults;
mod interchange;
mod logical_objects;
mod mapping;
mod payload;
mod resources;
mod xml_writer;

use defaults::*;
use interchange::*;
use logical_objects::*;
use mapping::*;
use payload::*;
use resources::*;
use xml_writer::*;

use super::{
    colors::{rgb_fractions, CdxmlColorTable},
    CdxmlDefaults, CdxmlJustification,
};

fn format_query_list(values: &[String], excluded: bool) -> String {
    let body = values.join(" ");
    if excluded {
        format!("NOT {body}")
    } else {
        body
    }
}

fn table_border_line_type(style: crate::TableLineStyle) -> Option<&'static str> {
    match style {
        crate::TableLineStyle::Solid => None,
        crate::TableLineStyle::Dashed => Some("Dashed"),
        crate::TableLineStyle::Bold => Some("Bold"),
        crate::TableLineStyle::Wavy => Some("Wavy"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CdxmlEnhancedStereo {
    kind: &'static str,
    group_number: Option<u32>,
}

fn cdxml_enhanced_stereo_by_node(
    fragment: &MoleculeFragment,
) -> BTreeMap<String, CdxmlEnhancedStereo> {
    use chemsema_chemical_graph::{EnhancedStereoKindV2, StereoElementV2};

    let mut groups = fragment
        .stereo
        .iter()
        .filter_map(|element| {
            let StereoElementV2::EnhancedGroup {
                id,
                group_kind,
                members,
            } = element
            else {
                return None;
            };
            Some((id, *group_kind, members))
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| left.0.cmp(right.0));
    let mut next_and = 1u32;
    let mut next_or = 1u32;
    let mut result = BTreeMap::new();
    for (id, kind, members) in groups {
        let (source_kind, group_number) = match kind {
            EnhancedStereoKindV2::Absolute => ("Absolute", None),
            EnhancedStereoKindV2::And => {
                let number = trailing_positive_integer(id).unwrap_or(next_and);
                next_and = next_and.max(number + 1);
                ("And", Some(number))
            }
            EnhancedStereoKindV2::Or => {
                let number = trailing_positive_integer(id).unwrap_or(next_or);
                next_or = next_or.max(number + 1);
                ("Or", Some(number))
            }
        };
        for member in members {
            if let Some(node_id) = member.strip_prefix("tetrahedral-") {
                result.insert(
                    node_id.to_string(),
                    CdxmlEnhancedStereo {
                        kind: source_kind,
                        group_number,
                    },
                );
            }
        }
    }
    result
}

fn trailing_positive_integer(value: &str) -> Option<u32> {
    let digits = value
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    digits.parse::<u32>().ok().filter(|number| *number > 0)
}

fn node_has_native_query_annotation(node: &Node) -> bool {
    let properties = &node.atom_properties;
    properties.isotopic_abundance != crate::IsotopicAbundance::Unspecified
        || properties.free_sites.is_some()
        || properties.ring_bond_count != crate::RingBondCount::Unspecified
        || properties.unsaturated_bonds != crate::UnsaturatedBonds::Unspecified
        || properties.substituents_up_to.is_some()
        || properties.substituents_exactly.is_some()
        || properties.translation != crate::QueryTranslation::Equal
        || properties.reaction_change
        || properties.reaction_stereo != crate::AtomReactionStereo::Unspecified
        || node
            .meta
            .pointer("/import/cdxml/restrictImplicitHydrogens")
            .and_then(Value::as_bool)
            == Some(true)
}

pub fn document_to_cdxml(document: &ChemSemaDocument) -> String {
    let (generated, entity_ids) = CdxmlDocumentWriter::new(document).write();
    if document.interchange.get("cdxml").is_none() && document.logical_objects.is_empty() {
        return generated;
    }
    let Ok(mut root) = super::parse_xml_tree(&generated) else {
        return generated;
    };
    if let Some(source) = document.interchange.get("cdxml") {
        let mut source_root = source.root.clone();
        remove_regenerated_scene_objects(&mut source_root, document);
        remove_native_logical_objects(&mut source_root);
        retain_native_chemical_properties(&mut source_root, &document.chemical_properties);
        retain_native_annotations(&mut source_root, &document.objects);
        retain_native_plasmid_maps(&mut source_root, &document.objects);
        merge_interchange_tree(&mut root, &source_root);
    }
    apply_native_logical_objects(&mut root, document, &entity_ids);
    serialize_cdxml_tree(&root)
}

struct CdxmlDocumentWriter<'a> {
    document: &'a ChemSemaDocument,
    next_id: u64,
    reserved_ids: BTreeSet<u64>,
    used_ids: BTreeSet<u64>,
    source_page_id: Option<String>,
    node_ids: BTreeMap<String, String>,
    bond_ids: BTreeMap<(String, String), String>,
    entity_ids: BTreeMap<String, String>,
    page_splitter_ids: BTreeMap<String, String>,
    colors: CdxmlColorTable,
    fonts: CdxmlFontTable,
    defaults: CdxmlDefaults,
    editing_scale: f64,
}

impl<'a> CdxmlDocumentWriter<'a> {
    fn new(document: &'a ChemSemaDocument) -> Self {
        let mut colors = CdxmlColorTable::for_export(&document.document.page.background);
        collect_document_colors(document, &mut colors);
        let mut fonts = CdxmlFontTable::default();
        collect_document_fonts(document, &mut fonts);
        let mut defaults = export_cdxml_defaults(document);
        defaults.label_font = fonts
            .id_for(&document.style.label_style.font_family)
            .parse()
            .unwrap_or(3);
        defaults.caption_font = fonts
            .id_for(&document.style.caption_style.font_family)
            .parse()
            .unwrap_or(3);
        let foreground = document
            .document
            .meta
            .pointer("/import/cdxml/defaults/foregroundColor")
            .and_then(Value::as_str)
            .unwrap_or(&document.style.label_style.fill);
        defaults.color = colors.id_for(foreground).parse().unwrap_or(0);
        let source_root = document.interchange.get("cdxml").map(|source| &source.root);
        let mut reserved_ids = BTreeSet::new();
        if let Some(root) = source_root {
            collect_interchange_numeric_ids(root, &mut reserved_ids);
        }
        reserved_ids.extend(
            document
                .chemical_properties
                .iter()
                .filter_map(|property| property.source_id.as_deref())
                .filter_map(|id| id.parse::<u64>().ok()),
        );
        let source_page_id = source_root
            .and_then(|root| root.children.iter().find(|child| child.name == "page"))
            .and_then(|page| page.id.clone());
        Self {
            document,
            next_id: 1,
            reserved_ids,
            used_ids: BTreeSet::new(),
            source_page_id,
            node_ids: BTreeMap::new(),
            bond_ids: BTreeMap::new(),
            entity_ids: BTreeMap::new(),
            page_splitter_ids: BTreeMap::new(),
            colors,
            fonts,
            defaults,
            editing_scale: cdxml_editing_scale(document),
        }
    }

    fn write(mut self) -> (String, BTreeMap<String, String>) {
        self.prepare_bond_ids();
        self.prepare_annotation_basis_ids();
        self.prepare_page_splitter_ids();
        let layout = &self.document.document.layout;
        let rendered = crate::render_document(self.document);
        let resolved = layout.resolve(crate::render_primitives_bounds(rendered.iter()));
        let width = resolved.total_width.max(1.0);
        let height = resolved.total_height.max(1.0);
        let root_bbox = format!(
            "{} {} {} {}",
            fmt_num(resolved.origin[0]),
            fmt_num(resolved.origin[1]),
            fmt_num(resolved.origin[0] + width),
            fmt_num(resolved.origin[1] + height)
        );
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        out.push_str("<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n");
        write!(
            out,
            "<CDXML CreationProgram=\"ChemSema\" ModificationProgram=\"{}\" Name=\"{}\" BoundingBox=\"{}\" WindowPosition=\"0 0\" WindowSize=\"-32768 -32768\" WindowIsZoomed=\"yes\" FractionalWidths=\"{}\" InterpretChemically=\"{}\" ShowAtomQuery=\"{}\" ShowAtomStereo=\"{}\" ShowAtomEnhancedStereo=\"{}\" ShowAtomNumber=\"{}\" ShowResidueID=\"{}\" ShowBondQuery=\"{}\" ShowBondRxn=\"{}\" ShowBondStereo=\"{}\" ShowTerminalCarbonLabels=\"{}\" ShowNonTerminalCarbonLabels=\"{}\" HideImplicitHydrogens=\"{}\" LabelFont=\"{}\" LabelSize=\"{}\" LabelFace=\"{}\" CaptionFont=\"{}\" CaptionSize=\"{}\" CaptionFace=\"{}\" LineWidth=\"{}\" BoldWidth=\"{}\" BondLength=\"{}\" BondSpacing=\"{}\" HashSpacing=\"{}\" MarginWidth=\"{}\" ChainAngle=\"{}\" LabelJustification=\"{}\" CaptionJustification=\"{}\" PrintMargins=\"{}\" color=\"{}\" bgcolor=\"{}\"",
            concat!("ChemSema/", env!("CARGO_PKG_VERSION"), ";cdx-tags=chemdraw"),
            xml_escape_attr(&self.document.document.title),
            root_bbox,
            fmt_cdxml_bool(self.defaults.fractional_widths),
            fmt_cdxml_bool(self.defaults.interpret_chemically.unwrap_or(true)),
            fmt_cdxml_bool(self.defaults.show_atom_query),
            fmt_cdxml_bool(self.defaults.show_atom_stereo),
            fmt_cdxml_bool(self.defaults.show_atom_enhanced_stereo),
            fmt_cdxml_bool(self.defaults.show_atom_number),
            fmt_cdxml_bool(self.defaults.show_residue_id),
            fmt_cdxml_bool(self.defaults.show_bond_query),
            fmt_cdxml_bool(self.defaults.show_bond_rxn),
            fmt_cdxml_bool(self.defaults.show_bond_stereo),
            fmt_cdxml_bool(self.defaults.show_terminal_carbon_labels),
            fmt_cdxml_bool(self.defaults.show_non_terminal_carbon_labels),
            fmt_cdxml_bool(self.defaults.hide_implicit_hydrogens),
            self.defaults.label_font,
            fmt_num(self.defaults.label_size),
            self.defaults.label_face,
            self.defaults.caption_font,
            fmt_num(self.defaults.caption_size),
            self.defaults.caption_face,
            fmt_num(self.defaults.line_width),
            fmt_num(self.defaults.bold_width),
            fmt_num(self.defaults.bond_length),
            fmt_num(self.defaults.bond_spacing),
            fmt_num(self.defaults.hash_spacing),
            fmt_num(self.defaults.margin_width),
            fmt_num(self.defaults.chain_angle),
            self.defaults.label_justification.as_cdxml(),
            self.defaults.caption_justification.as_cdxml(),
            fmt_margins(layout.margins),
            self.defaults.color,
            self.colors.background_id(),
        )
        .expect("writing CDXML root should not fail");
        write!(
            out,
            " Magnification=\"{}\"",
            fmt_num(layout.magnification_percent * 10.0)
        )
        .expect("writing CDXML magnification should not fail");
        if let Some([x, y]) = layout.fix_in_place_extent {
            write!(out, " FixInPlaceExtent=\"{} {}\"", fmt_num(x), fmt_num(y))
                .expect("writing CDXML in-place extent should not fail");
        }
        if let Some([x, y]) = layout.fix_in_place_gap {
            write!(out, " FixInPlaceGap=\"{} {}\"", fmt_num(x), fmt_num(y))
                .expect("writing CDXML in-place gap should not fail");
        }
        for (name, xml_name) in [
            ("lineHeight", "LineHeight"),
            ("labelLineHeight", "LabelLineHeight"),
            ("captionLineHeight", "CaptionLineHeight"),
        ] {
            if let Some(value) = self
                .document
                .document
                .meta
                .pointer(&format!("/import/cdxml/defaults/{name}"))
                .and_then(Value::as_str)
            {
                write!(out, " {xml_name}=\"{}\"", xml_escape_attr(value))
                    .expect("writing CDXML line-height default should not fail");
            }
        }
        out.push_str(">\n");
        self.write_color_table(&mut out);
        self.write_font_table(&mut out);
        let page_id = self
            .claim_source_id(self.source_page_id.clone())
            .unwrap_or_else(|| self.alloc_id());
        let mut page_attrs = vec![
            ("id", page_id),
            ("BoundingBox", root_bbox.clone()),
            (
                "DrawingSpace",
                match layout.drawing_space {
                    crate::DrawingSpace::Pages => "pages".to_string(),
                    crate::DrawingSpace::Poster => "poster".to_string(),
                },
            ),
            ("HeaderPosition", fmt_num(layout.header_position)),
            ("FooterPosition", fmt_num(layout.footer_position)),
            (
                "PrintTrimMarks",
                fmt_cdxml_bool(layout.print_trim_marks).to_string(),
            ),
            ("HeightPages", resolved.height_pages.to_string()),
            ("WidthPages", resolved.width_pages.to_string()),
            ("Width", fmt_num(width)),
            ("Height", fmt_num(height)),
        ];
        if layout.drawing_space == crate::DrawingSpace::Poster || layout.page_overlap > 0.0 {
            page_attrs.push(("PageOverlap", fmt_num(layout.page_overlap)));
        }
        if !layout.header.is_empty() {
            page_attrs.push(("Header", layout.header.clone()));
        }
        if !layout.footer.is_empty() {
            page_attrs.push(("Footer", layout.footer.clone()));
        }
        if layout.page_definition != crate::PageDefinition::Undefined {
            page_attrs.push((
                "PageDefinition",
                layout.page_definition.as_cdxml().to_string(),
            ));
        }
        if !layout.legacy_splitter_position_ids.is_empty() {
            page_attrs.push((
                "SplitterPositions",
                layout
                    .legacy_splitter_position_ids
                    .iter()
                    .map(|id| self.page_splitter_ids.get(id).unwrap_or(id))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
        }
        write_open_tag(&mut out, 2, "page", page_attrs);

        let mut objects: Vec<&SceneObject> = self
            .document
            .objects
            .iter()
            .filter(|object| object.visible || object.kind() == crate::SceneObjectKind::Text)
            .collect();
        objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        self.write_scene_objects(&mut out, &objects);
        self.write_reaction_schemes(&mut out);
        self.write_chemical_properties(&mut out);
        self.write_page_splitters(&mut out);

        out.push_str("  </page>\n");
        out.push_str("</CDXML>\n");
        (out, self.entity_ids)
    }

    fn write_scene_object(&mut self, out: &mut String, object: &SceneObject) {
        let attached_node_id = object.meta.get("attachedNodeId").and_then(Value::as_str);
        let annotation_role = object.meta.get("role").and_then(Value::as_str);
        let is_object_tag_display = self
            .document
            .logical_objects
            .object_tags
            .iter()
            .any(|tag| tag.display_object_ids.iter().any(|id| id == &object.id));
        if object.object_type == "text"
            && attached_node_id.is_some()
            && !is_object_tag_display
            && (annotation_role.is_some_and(|role| matches!(role, "atom_number" | "stereo"))
                || (annotation_role == Some("query")
                    && attached_node_id.is_some_and(|node_id| {
                        document_node(self.document, node_id)
                            .is_some_and(node_has_native_query_annotation)
                    })))
        {
            // Unlinked cached displays are derived from the native node
            // properties below. A Text explicitly owned by an ObjectTag is
            // instead part of that relation and must be emitted inside it.
            return;
        }
        match object.kind() {
            crate::SceneObjectKind::Molecule => self.write_molecule_object(out, object),
            crate::SceneObjectKind::Line => self.write_line_object(out, object),
            crate::SceneObjectKind::Curve => self.write_curve_object(out, object),
            crate::SceneObjectKind::Shape => self.write_shape_object(out, object),
            crate::SceneObjectKind::Table => self.write_table_object(out, object),
            crate::SceneObjectKind::StoichiometryGrid => {
                self.write_stoichiometry_grid_object(out, object)
            }
            crate::SceneObjectKind::Image => self.write_image_object(out, object),
            crate::SceneObjectKind::Spectrum => self.write_spectrum_object(out, object),
            crate::SceneObjectKind::Bracket | crate::SceneObjectKind::Symbol => {
                self.write_bracket_object(out, object)
            }
            crate::SceneObjectKind::Text => self.write_text_object(out, object),
            crate::SceneObjectKind::Group => self.write_group_object(out, object),
            crate::SceneObjectKind::Geometry => self.write_geometry_object(out, object),
            crate::SceneObjectKind::Constraint => self.write_constraint_object(out, object),
        }
    }

    fn write_table_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(table) = object.payload.table.as_ref() else {
            return;
        };
        let (Some(&left), Some(&right), Some(&top), Some(&bottom)) = (
            table.column_guides.first(),
            table.column_guides.last(),
            table.row_guides.first(),
            table.row_guides.last(),
        ) else {
            return;
        };
        let tx = object.transform.translate[0];
        let ty = object.transform.translate[1];
        let bbox = [tx + left, ty + top, tx + right, ty + bottom];
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("BoundingBox", fmt_bbox(bbox)),
            ("color", self.colors.id_for(&table.default_border.color)),
            ("LineWidth", fmt_num(table.default_border.width)),
            ("Z", object.z_index.to_string()),
        ];
        if let Some(line_type) = table_border_line_type(table.default_border.line_style) {
            attrs.push(("LineType", line_type.to_string()));
        }
        write_open_tag(out, 4, "table", attrs);
        for cell in &table.cells {
            if cell.row >= table.rows || cell.column >= table.columns {
                continue;
            }
            let bounds = [
                tx + table.column_guides[cell.column],
                ty + table.row_guides[cell.row],
                tx + table.column_guides[cell.column + 1],
                ty + table.row_guides[cell.row + 1],
            ];
            write_open_tag(
                out,
                6,
                "page",
                vec![
                    ("id", self.alloc_id()),
                    ("BoundingBox", fmt_bbox(bounds)),
                    ("BoundsInParent", fmt_bbox(bounds)),
                    ("HeaderPosition", "36".to_string()),
                    ("FooterPosition", "36".to_string()),
                    ("PrintTrimMarks", "yes".to_string()),
                    ("HeightPages", "1".to_string()),
                    ("WidthPages", "1".to_string()),
                ],
            );
            for (side, border) in [
                ("top", cell.borders.top.as_ref()),
                ("left", cell.borders.left.as_ref()),
                ("bottom", cell.borders.bottom.as_ref()),
                ("right", cell.borders.right.as_ref()),
            ] {
                let Some(border) = border else {
                    continue;
                };
                let mut attrs = vec![
                    ("id", self.alloc_id()),
                    ("Side", side.to_string()),
                    (
                        "LineWidth",
                        fmt_num(if border.visible { border.width } else { 0.0 }),
                    ),
                    ("color", self.colors.id_for(&border.color)),
                ];
                if let Some(line_type) = table_border_line_type(border.line_style) {
                    attrs.push(("LineType", line_type.to_string()));
                }
                write_empty_tag(out, 8, "border", attrs);
            }
            for content_id in &cell.content_object_ids {
                let Some(content) = self.document.find_scene_object(content_id) else {
                    continue;
                };
                if content.visible || content.kind() == crate::SceneObjectKind::Text {
                    self.write_scene_object(out, content);
                }
            }
            write_indent(out, 6);
            out.push_str("</page>\n");
        }
        write_indent(out, 4);
        out.push_str("</table>\n");
    }

    fn write_stoichiometry_grid_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(grid) = object.payload.stoichiometry_grid.as_ref() else {
            return;
        };
        let [x, y, width, height] = object.payload.bbox.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        let bbox = [
            object.transform.translate[0] + x,
            object.transform.translate[1] + y,
            object.transform.translate[0] + x + width,
            object.transform.translate[1] + y + height,
        ];
        write_open_tag(
            out,
            4,
            "stoichiometrygrid",
            vec![
                ("id", self.object_cdxml_id(object)),
                ("BoundingBox", fmt_bbox(bbox)),
                (
                    "Visible",
                    if object.visible { "yes" } else { "no" }.to_string(),
                ),
                ("LineWidth", fmt_num(grid.style.line_width)),
                ("BoldWidth", fmt_num(grid.style.bold_width)),
                ("MarginWidth", fmt_num(grid.style.margin_width)),
                ("color", self.colors.id_for(&grid.style.color)),
                ("LabelFont", self.fonts.id_for(&grid.style.label_font)),
                ("LabelSize", fmt_num(grid.style.label_size)),
                ("LabelFace", grid.style.label_face.to_string()),
                ("Z", object.z_index.to_string()),
            ],
        );
        for component in &grid.components {
            let reference_id = (object.link_policy != crate::LinkPolicy::Unlinked)
                .then(|| {
                    component
                        .reference_entity_id
                        .as_ref()
                        .and_then(|entity_id| self.document.find_scene_object(entity_id))
                        .map(|source| self.object_cdxml_id(source))
                        .or_else(|| component.unresolved_reference_id.clone())
                })
                .flatten();
            let mut attrs = vec![
                ("id", component.id.clone()),
                (
                    "ComponentIsHeader",
                    if component.is_header { "yes" } else { "no" }.to_string(),
                ),
                (
                    "ComponentIsReactant",
                    if component.role == crate::StoichiometryComponentRole::Reactant {
                        "yes"
                    } else {
                        "no"
                    }
                    .to_string(),
                ),
                (
                    "Visible",
                    if component.visible { "yes" } else { "no" }.to_string(),
                ),
                ("Width", fmt_num(component.width)),
            ];
            if let Some(reference_id) = reference_id {
                attrs.push(("ComponentReferenceID", reference_id));
            }
            write_open_tag(out, 6, "sgcomponent", attrs);
            for datum in grid
                .data
                .iter()
                .filter(|datum| datum.component_id == component.id)
            {
                let Some(row) = grid.rows.iter().find(|row| row.id == datum.row_id) else {
                    continue;
                };
                write_empty_tag(
                    out,
                    8,
                    "sgdatum",
                    vec![
                        ("id", datum.id.clone()),
                        ("SGPropertyType", row.property_type.clone()),
                        ("SGDataType", row.data_type.clone()),
                        (
                            "SGDataValue",
                            if datum.value.display.is_empty() {
                                datum.value.canonical.clone()
                            } else {
                                datum.value.display.clone()
                            },
                        ),
                        (
                            "IsEdited",
                            if datum.is_edited { "yes" } else { "no" }.to_string(),
                        ),
                        (
                            "IsHidden",
                            if datum.is_hidden { "yes" } else { "no" }.to_string(),
                        ),
                        (
                            "IsReadOnly",
                            if datum.is_read_only { "yes" } else { "no" }.to_string(),
                        ),
                        (
                            "Visible",
                            if datum.visible { "yes" } else { "no" }.to_string(),
                        ),
                    ],
                );
            }
            out.push_str("      </sgcomponent>\n");
        }
        out.push_str("    </stoichiometrygrid>\n");
    }

    fn write_reaction_schemes(&mut self, out: &mut String) {
        for scheme in &self.document.reaction_schemes.clone() {
            let scheme_id = self
                .claim_source_id(Some(scheme.id.clone()))
                .unwrap_or_else(|| self.alloc_id());
            write_open_tag(out, 4, "scheme", vec![("id", scheme_id)]);
            for step in &scheme.steps {
                let step_id = self
                    .claim_source_id(Some(step.id.clone()))
                    .unwrap_or_else(|| self.alloc_id());
                let mut attrs = vec![("id", step_id)];
                for (name, ids) in [
                    ("ReactionStepReactants", &step.reactant_entity_ids),
                    ("ReactionStepProducts", &step.product_entity_ids),
                    ("ReactionStepPlusses", &step.plus_object_ids),
                    ("ReactionStepArrows", &step.arrow_object_ids),
                    ("ReactionStepObjectsAboveArrow", &step.objects_above_arrow),
                    ("ReactionStepObjectsBelowArrow", &step.objects_below_arrow),
                ] {
                    let value = ids
                        .iter()
                        .filter_map(|id| self.entity_ids.get(id).cloned())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !value.is_empty() {
                        attrs.push((name, value));
                    }
                }
                for (origin, name) in [
                    (
                        crate::ReactionAtomMappingOrigin::Manual,
                        "ReactionStepAtomMapManual",
                    ),
                    (
                        crate::ReactionAtomMappingOrigin::Automatic,
                        "ReactionStepAtomMapAuto",
                    ),
                    (
                        crate::ReactionAtomMappingOrigin::Imported,
                        "ReactionStepAtomMap",
                    ),
                ] {
                    let mappings = step
                        .atom_mappings
                        .iter()
                        .filter(|mapping| mapping.origin == origin)
                        .filter_map(|mapping| {
                            Some(format!(
                                "{} {}",
                                self.entity_ids.get(&mapping.reactant_atom_id)?,
                                self.entity_ids.get(&mapping.product_atom_id)?
                            ))
                        })
                        .collect::<Vec<_>>();
                    if !mappings.is_empty() {
                        attrs.push((name, mappings.join(" ")));
                    }
                }
                write_empty_tag(out, 6, "step", attrs);
            }
            out.push_str("    </scheme>\n");
        }
    }

    fn write_page_splitters(&mut self, out: &mut String) {
        for splitter in &self.document.document.layout.splitters {
            let id = self
                .page_splitter_ids
                .get(&splitter.id)
                .cloned()
                .expect("page splitter ID was prepared");
            let mut attrs = vec![("id", id)];
            if let Some([x, y]) = splitter.position {
                attrs.push(("p", format!("{} {}", fmt_num(x), fmt_num(y))));
            }
            if splitter.page_definition != crate::PageDefinition::Undefined {
                attrs.push((
                    "PageDefinition",
                    splitter.page_definition.as_cdxml().to_string(),
                ));
            }
            write_empty_tag(out, 4, "splitter", attrs);
        }
    }

    fn prepare_page_splitter_ids(&mut self) {
        for splitter in &self.document.document.layout.splitters {
            let id = self
                .claim_source_id(Some(splitter.id.clone()))
                .unwrap_or_else(|| self.alloc_id());
            self.page_splitter_ids.insert(splitter.id.clone(), id);
        }
    }

    fn write_geometry_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(geometry) = object.payload.geometry.as_ref() else {
            return;
        };
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("Z", object.z_index.to_string()),
            ("GeometricFeature", geometry.feature.as_cdxml().to_string()),
        ];
        if let Some(style) = object_style(self.document, object) {
            if let Some(value) = style_number_value(style, "strokeWidth") {
                attrs.push(("LineWidth", fmt_num(value)));
            }
            if let Some(value) = style_number_value(style, "hashSpacing") {
                attrs.push(("HashSpacing", fmt_num(value)));
            }
            if let Some(value) = style_string_value(style, "stroke") {
                attrs.push(("color", self.colors.id_for(&value)));
            }
        }
        if let Some(value) = geometry.relation_value {
            attrs.push(("RelationValue", fmt_num(value)));
        }
        if geometry.point_is_directed {
            attrs.push(("PointIsDirected", fmt_cdxml_bool(true).to_string()));
        }
        if !object.name.is_empty() {
            attrs.push(("Name", object.name.clone()));
        }
        if !object.visible {
            attrs.push(("Visible", fmt_cdxml_bool(false).to_string()));
        }
        if let Some(bbox) = object.payload.bbox {
            attrs.push(("BoundingBox", fmt_bbox(bbox)));
        }
        let basis =
            self.annotation_basis_ids(&geometry.basis_entity_ids, &geometry.unresolved_basis_ids);
        if !basis.is_empty() {
            attrs.push(("BasisObjects", basis.join(" ")));
        }
        write_empty_tag(out, 4, "geometry", attrs);
    }

    fn write_constraint_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(constraint) = object.payload.constraint.as_ref() else {
            return;
        };
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("Z", object.z_index.to_string()),
            (
                "ConstraintType",
                constraint.constraint_type.as_cdxml().to_string(),
            ),
        ];
        if let Some(style) = object_style(self.document, object) {
            if let Some(value) = style_number_value(style, "strokeWidth") {
                attrs.push(("LineWidth", fmt_num(value)));
            }
            if let Some(value) = style_number_value(style, "hashSpacing") {
                attrs.push(("HashSpacing", fmt_num(value)));
            }
            if let Some(value) = style_string_value(style, "stroke") {
                attrs.push(("color", self.colors.id_for(&value)));
            }
        }
        if let Some(value) = constraint.minimum {
            attrs.push(("ConstraintMin", fmt_num(value)));
        }
        if let Some(value) = constraint.maximum {
            attrs.push(("ConstraintMax", fmt_num(value)));
        }
        if constraint.ignore_unconnected_atoms {
            attrs.push(("IgnoreUnconnectedAtoms", fmt_cdxml_bool(true).to_string()));
        }
        if constraint.dihedral_is_chiral {
            attrs.push(("DihedralIsChiral", fmt_cdxml_bool(true).to_string()));
        }
        if constraint.point_is_directed {
            attrs.push(("PointIsDirected", fmt_cdxml_bool(true).to_string()));
        }
        if !object.name.is_empty() {
            attrs.push(("Name", object.name.clone()));
        }
        if !object.visible {
            attrs.push(("Visible", fmt_cdxml_bool(false).to_string()));
        }
        if let Some(bbox) = object.payload.bbox {
            attrs.push(("BoundingBox", fmt_bbox(bbox)));
        }
        let basis = self.annotation_basis_ids(
            &constraint.basis_entity_ids,
            &constraint.unresolved_basis_ids,
        );
        if !basis.is_empty() {
            attrs.push(("BasisObjects", basis.join(" ")));
        }
        let text = crate::geometry_constraints::constraint_value_text(constraint);
        let position = constraint_label_position(self.document, object, constraint);
        let (Some(text), Some(position)) = (text, position) else {
            write_empty_tag(out, 4, "constraint", attrs);
            return;
        };
        write_open_tag(out, 4, "constraint", attrs);
        let mut tag_attrs = vec![
            ("TagType", "Unknown".to_string()),
            (
                "Name",
                match constraint.constraint_type {
                    crate::ConstraintType::Distance => "distance",
                    crate::ConstraintType::Angle => "angle",
                    crate::ConstraintType::ExclusionSphere => "exclusionSphere",
                }
                .to_string(),
            ),
        ];
        if constraint.display.positioning_type != crate::AnnotationPositioningType::Auto {
            tag_attrs.push((
                "PositioningType",
                constraint.display.positioning_type.as_cdxml().to_string(),
            ));
        }
        if constraint.display.positioning_type == crate::AnnotationPositioningType::Angle {
            if let Some(angle) = constraint.display.positioning_angle {
                tag_attrs.push(("PositioningAngle", fmt_num(angle)));
            }
        }
        if constraint.display.positioning_type == crate::AnnotationPositioningType::Offset {
            if let Some(offset) = constraint.display.positioning_offset {
                tag_attrs.push((
                    "PositioningOffset",
                    format!("{} {}", fmt_num(offset[0]), fmt_num(offset[1])),
                ));
            }
        }
        if !constraint.display.indicator_visible {
            tag_attrs.push(("Visible", fmt_cdxml_bool(false).to_string()));
        }
        write_open_tag(out, 6, "objecttag", tag_attrs);
        write_open_tag(
            out,
            8,
            "t",
            vec![
                ("p", fmt_point(position)),
                ("CaptionLineHeight", "variable".to_string()),
            ],
        );
        let mut face = 0;
        if constraint.display.font_weight >= 600 {
            face |= 1;
        }
        if constraint.display.italic {
            face |= 2;
        }
        if constraint.display.underline {
            face |= 4;
        }
        let mut run_attrs = vec![
            (
                "font",
                self.fonts
                    .id_for(constraint.display.font_family.as_deref().unwrap_or("Arial")),
            ),
            ("size", fmt_num(constraint.display.font_size.unwrap_or(7.5))),
            (
                "color",
                self.colors
                    .id_for(constraint.display.fill.as_deref().unwrap_or("#000000")),
            ),
        ];
        if face != 0 {
            run_attrs.push(("face", face.to_string()));
        }
        write_text_tag(out, 10, "s", run_attrs, &text);
        out.push_str("        </t>\n");
        out.push_str("      </objecttag>\n");
        out.push_str("    </constraint>\n");
    }

    fn annotation_basis_ids(&self, basis: &[String], unresolved: &[String]) -> Vec<String> {
        basis
            .iter()
            .filter_map(|entity_id| self.entity_ids.get(entity_id).cloned())
            .chain(unresolved.iter().cloned())
            .collect()
    }

    fn write_spectrum_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(spectrum) = object.payload.spectrum.as_ref() else {
            return;
        };
        if spectrum.validate().is_err() {
            return;
        }
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        if width <= crate::EPSILON || height <= crate::EPSILON {
            return;
        }
        let left = object.transform.translate[0] + x;
        let top = object.transform.translate[1] + y;
        let right = left + width;
        let bottom = top + height;
        let style = object_style(self.document, object);
        let stroke = style
            .and_then(|style| style_nullable_string_value(style, "stroke"))
            .unwrap_or_else(|| "#000000".to_string());
        let line_width = style
            .and_then(|style| style_number_value(style, "strokeWidth"))
            .unwrap_or(self.defaults.line_width);
        let font_family = style
            .and_then(|style| style_string_value(style, "fontFamily"))
            .unwrap_or_else(|| self.document.style.label_style.font_family.clone());
        let font_size = style
            .and_then(|style| style_number_value(style, "fontSize"))
            .unwrap_or(self.document.style.label_style.font_size);
        let mut face = 0u32;
        if style
            .and_then(|style| style_number_value(style, "fontWeight"))
            .unwrap_or(400.0)
            >= 600.0
        {
            face |= 1;
        }
        if style
            .and_then(|style| style_string_value(style, "fontStyle"))
            .as_deref()
            == Some("italic")
        {
            face |= 2;
        }
        if style
            .and_then(|style| style.get("underline"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            face |= 4;
        }
        if style
            .and_then(|style| style.get("outline"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            face |= 8;
        }
        if style
            .and_then(|style| style.get("shadow"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            face |= 16;
        }
        face |= match style
            .and_then(|style| style_string_value(style, "script"))
            .unwrap_or_else(|| "normal".to_string())
            .as_str()
        {
            "subscript" => 32,
            "superscript" => 64,
            "chemical" => 96,
            _ => 0,
        };
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("BoundingBox", fmt_bbox([left, top, right, bottom])),
            ("Z", object.z_index.to_string()),
            ("XSpacing", fmt_num(spectrum.x_spacing)),
            ("XLow", fmt_num(spectrum.x_low)),
            ("XType", spectrum.x_type.as_cdxml().to_string()),
            ("YType", spectrum.y_type.as_cdxml().to_string()),
            ("Class", spectrum.class.as_cdxml().to_string()),
            ("LineWidth", fmt_num(line_width)),
            ("color", self.colors.id_for(&stroke)),
            ("LabelFont", self.fonts.id_for(&font_family)),
            ("LabelSize", fmt_num(font_size)),
        ];
        if face != 0 {
            attrs.push(("LabelFace", face.to_string()));
        }
        if !spectrum.x_axis_label.is_empty() {
            attrs.push(("XAxisLabel", spectrum.x_axis_label.clone()));
        }
        if !spectrum.y_axis_label.is_empty() {
            attrs.push(("YAxisLabel", spectrum.y_axis_label.clone()));
        }
        if spectrum.y_low.abs() > crate::EPSILON {
            attrs.push(("YLow", fmt_num(spectrum.y_low)));
        }
        if (spectrum.y_scale - 1.0).abs() > crate::EPSILON {
            attrs.push(("YScale", fmt_num(spectrum.y_scale)));
        }
        write_open_tag(out, 4, "spectrum", attrs);
        out.push('\n');
        for chunk in spectrum.data_points.chunks(8) {
            out.push_str("      ");
            out.push_str(
                &chunk
                    .iter()
                    .map(|value| fmt_num(*value))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            out.push('\n');
        }
        out.push_str("    </spectrum>\n");
    }

    fn write_image_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
            return;
        };
        let Some(resource) = self.document.resources.get(resource_ref) else {
            return;
        };
        let Ok(crop) = object.payload.image_crop() else {
            return;
        };
        let mut payloads = Vec::<(&str, String)>::new();
        let embedded_attribute: String;
        let mut uncompressed_size = None;
        if resource.resource_type == "image" {
            let Some(image) = resource.data.as_image() else {
                return;
            };
            let image = if let Some(crop) = crop {
                let Ok(cropped) = crate::cropped_image_resource(&image, crop) else {
                    return;
                };
                cropped
            } else {
                image
            };
            let attribute = match image.mime_type.as_str() {
                "image/png" => "PNG",
                "image/jpeg" => "JPEG",
                "image/gif" => "GIF",
                "image/tiff" => "TIFF",
                "image/bmp" => "BMP",
                _ => return,
            };
            payloads.push((attribute, image.data_base64));
        } else if resource.resource_type == "embedded-object" {
            let Some(embedded) = resource.data.as_embedded_object() else {
                return;
            };
            if !matches!(
                embedded.format.as_str(),
                "TIFF"
                    | "EnhancedMetafile"
                    | "CompressedEnhancedMetafile"
                    | "WindowsMetafile"
                    | "CompressedWindowsMetafile"
                    | "OLEObject"
                    | "CompressedOLEObject"
                    | "PDF"
                    | "MacPICT"
            ) {
                return;
            }
            embedded_attribute = embedded.format;
            payloads.push((embedded_attribute.as_str(), embedded.data_base64));
            uncompressed_size = embedded.uncompressed_size;
            if let Some(preview) = embedded.preview {
                let preview = if let Some(crop) = crop {
                    let Ok(cropped) = crate::cropped_image_resource(&preview, crop) else {
                        return;
                    };
                    cropped
                } else {
                    preview
                };
                payloads.push(("PNG", preview.data_base64));
            }
        } else {
            return;
        }
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        let scale_x = object.transform.scale[0];
        let scale_y = object.transform.scale[1];
        let left = object.transform.translate[0] + x * scale_x;
        let top = object.transform.translate[1] + y * scale_y;
        let right = left + width * scale_x;
        let bottom = top + height * scale_y;
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("BoundingBox", fmt_bbox([left, top, right, bottom])),
            ("Z", object.z_index.to_string()),
        ];
        for (attribute, data_base64) in payloads {
            let Ok(bytes) = BASE64.decode(data_base64.as_bytes()) else {
                return;
            };
            let encoded = if matches!(
                attribute,
                "CompressedEnhancedMetafile" | "CompressedWindowsMetafile" | "CompressedOLEObject"
            ) {
                BASE64.encode(bytes)
            } else {
                encode_hex_bytes(&bytes)
            };
            attrs.push((attribute, encoded));
        }
        if let Some(size) = uncompressed_size {
            let size_attribute = attrs.iter().find_map(|(attribute, _)| match *attribute {
                "CompressedEnhancedMetafile" => Some("UncompressedEnhancedMetafileSize"),
                "CompressedWindowsMetafile" => Some("UncompressedWindowsMetafileSize"),
                "CompressedOLEObject" => Some("UncompressedOLEObjectSize"),
                _ => None,
            });
            if let Some(size_attribute) = size_attribute {
                attrs.push((size_attribute, size.to_string()));
            }
        }
        if object.transform.rotate.abs() > crate::EPSILON {
            attrs.push(("RotationAngle", fmt_num(object.transform.rotate)));
        }
        write_open_tag(out, 4, "embeddedobject", attrs);
        out.push_str("</embeddedobject>\n");
    }

    fn write_scene_objects(&mut self, out: &mut String, objects: &[&SceneObject]) {
        let mut emitted = std::collections::BTreeSet::new();
        let table_content_ids = objects
            .iter()
            .filter_map(|object| object.payload.table.as_ref())
            .flat_map(|table| table.cells.iter())
            .flat_map(|cell| cell.content_object_ids.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>();
        for object in objects {
            if emitted.contains(&object.id) || table_content_ids.contains(&object.id) {
                continue;
            }
            if object.object_type == "molecule" {
                let scope = cdxml_bond_crossing_scope(object);
                if scope.starts_with("cdxml-fragment:") {
                    let components: Vec<_> = objects
                        .iter()
                        .copied()
                        .filter(|candidate| {
                            candidate.object_type == "molecule"
                                && cdxml_bond_crossing_scope(candidate) == scope
                        })
                        .collect();
                    if components.len() > 1 {
                        emitted.extend(components.iter().map(|component| component.id.clone()));
                        self.write_molecule_objects_as_fragment(out, &components);
                        continue;
                    }
                }
            }
            emitted.insert(object.id.clone());
            self.write_scene_object(out, object);
        }
    }

    fn write_group_object(&mut self, out: &mut String, object: &SceneObject) {
        if object.children.is_empty() {
            return;
        }
        if object.meta.get("kind").and_then(Value::as_str) == Some("bracket-group") {
            self.write_scene_object_children(out, object);
            return;
        }
        let mut scratch = self.document.clone();
        scratch.objects = object.children.clone();
        let bbox = crate::render_primitives_bounds(crate::render_document(&scratch).iter())
            .or(object.payload.bbox.map(|bbox| {
                [
                    object.transform.translate[0] + bbox[0],
                    object.transform.translate[1] + bbox[1],
                    object.transform.translate[0] + bbox[0] + bbox[2],
                    object.transform.translate[1] + bbox[1] + bbox[3],
                ]
            }))
            .unwrap_or([
                object.transform.translate[0],
                object.transform.translate[1],
                object.transform.translate[0] + 1.0,
                object.transform.translate[1] + 1.0,
            ]);
        writeln!(
            out,
            "    <group id=\"{}\" BoundingBox=\"{}\" Z=\"{}\">",
            self.object_cdxml_id(object),
            fmt_bbox(bbox),
            object.z_index
        )
        .expect("writing group should not fail");

        self.write_scene_object_children(out, object);
        out.push_str("    </group>\n");
    }

    fn write_scene_object_children(&mut self, out: &mut String, object: &SceneObject) {
        let mut children: Vec<&SceneObject> = object
            .children
            .iter()
            .filter(|child| child.visible || child.kind() == crate::SceneObjectKind::Text)
            .collect();
        children.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        self.write_scene_objects(out, &children);
    }

    fn write_color_table(&self, out: &mut String) {
        out.push_str("  <colortable>\n");
        for color in self.colors.colors() {
            let (r, g, b) = rgb_fractions(color);
            writeln!(
                out,
                "    <color r=\"{}\" g=\"{}\" b=\"{}\"/>",
                fmt_num(r),
                fmt_num(g),
                fmt_num(b)
            )
            .expect("writing color table should not fail");
        }
        out.push_str("  </colortable>\n");
    }

    fn write_font_table(&self, out: &mut String) {
        out.push_str("  <fonttable>\n");
        for (id, name) in self.fonts.fonts() {
            writeln!(
                out,
                "    <font id=\"{}\" charset=\"iso-8859-1\" name=\"{}\"/>",
                id,
                xml_escape_attr(name),
            )
            .expect("writing font table should not fail");
        }
        out.push_str("  </fonttable>\n");
    }

    fn write_molecule_object(&mut self, out: &mut String, object: &SceneObject) {
        self.write_molecule_objects_as_fragment(out, &[object]);
    }

    fn write_molecule_objects_as_fragment(&mut self, out: &mut String, objects: &[&SceneObject]) {
        let components: Vec<_> = objects
            .iter()
            .filter_map(|object| {
                object
                    .payload
                    .resource_ref
                    .as_ref()
                    .and_then(|resource_ref| self.document.resources.get(resource_ref))
                    .and_then(|resource| resource.data.as_fragment())
                    .map(|fragment| (*object, fragment))
            })
            .filter(|(_, fragment)| !fragment.nodes.is_empty())
            .collect();
        if components.is_empty() {
            return;
        }

        let source_fragment_id = components
            .first()
            .and_then(|(object, _)| {
                object
                    .meta
                    .pointer("/import/cdxml/fragmentId")
                    .and_then(Value::as_str)
            })
            .filter(|source_id| {
                components.iter().all(|(object, _)| {
                    object
                        .meta
                        .pointer("/import/cdxml/fragmentId")
                        .and_then(Value::as_str)
                        == Some(*source_id)
                })
            })
            .map(ToString::to_string);
        let fragment_id = self
            .claim_source_id(source_fragment_id)
            .unwrap_or_else(|| self.alloc_id());
        for (object, _) in &components {
            self.entity_ids
                .insert(object.id.clone(), fragment_id.clone());
        }
        let bbox = components
            .iter()
            .filter_map(|(object, fragment)| molecule_world_bbox(object, fragment))
            .reduce(|left, right| {
                [
                    left[0].min(right[0]),
                    left[1].min(right[1]),
                    left[2].max(right[2]),
                    left[3].max(right[3]),
                ]
            })
            .unwrap_or([0.0, 0.0, 1.0, 1.0]);
        let z_index = components
            .iter()
            .map(|(object, _)| object.z_index)
            .min()
            .unwrap_or(10);
        writeln!(
            out,
            "    <fragment id=\"{}\" BoundingBox=\"{}\" Z=\"{}\">",
            fragment_id,
            fmt_bbox(bbox),
            z_index
        )
        .expect("writing fragment should not fail");

        let mut node_ids = BTreeMap::new();
        for (_, fragment) in &components {
            for node in &fragment.nodes {
                let cdxml_id = self.entity_ids.get(&node.id).cloned().unwrap_or_else(|| {
                    self.claim_source_id(Some(node.id.clone()))
                        .unwrap_or_else(|| self.alloc_id())
                });
                node_ids.insert(node.id.clone(), cdxml_id);
            }
        }
        self.node_ids.extend(node_ids.clone());
        self.entity_ids.extend(node_ids.clone());
        for (object, fragment) in &components {
            let enhanced_stereo = cdxml_enhanced_stereo_by_node(fragment);
            for node in &fragment.nodes {
                self.write_node(
                    out,
                    object,
                    node,
                    &node_ids[&node.id],
                    enhanced_stereo.get(&node.id),
                );
            }
        }
        for (object, fragment) in &components {
            let crossing_scope = cdxml_bond_crossing_scope(object);
            for bond in &fragment.bonds {
                let Some(cdxml_id) = self
                    .bond_ids
                    .get(&(crossing_scope.clone(), bond.id.clone()))
                    .cloned()
                else {
                    continue;
                };
                self.write_bond(out, bond, &cdxml_id, &node_ids, &crossing_scope);
            }
            for area in &fragment.colored_areas {
                let basis_objects = area
                    .basis_bonds
                    .iter()
                    .filter_map(|bond_id| {
                        self.bond_ids
                            .get(&(crossing_scope.clone(), bond_id.clone()))
                            .cloned()
                    })
                    .collect::<Vec<_>>();
                if basis_objects.len() != area.basis_bonds.len() {
                    continue;
                }
                writeln!(
                    out,
                    "      <ColoredMolecularArea id=\"{}\" bgcolor=\"{}\" BasisObjects=\"{}\"/>",
                    self.alloc_id(),
                    self.colors.id_for(&area.color),
                    basis_objects.join(" ")
                )
                .expect("writing colored molecular area should not fail");
            }
        }
        out.push_str("    </fragment>\n");
    }

    fn write_node(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        node: &Node,
        cdxml_id: &str,
        enhanced_stereo: Option<&CdxmlEnhancedStereo>,
    ) {
        let point = object_local_point(object, node.position);
        let label_text = node
            .label
            .as_ref()
            .and_then(|label| {
                label
                    .source_text
                    .as_ref()
                    .or(Some(&label.text))
                    .filter(|text| !text.trim().is_empty())
            })
            .cloned();
        let is_plain_carbon =
            node.atomic_number == 6 && label_text.is_none() && !node.is_placeholder;
        let is_nickname = node.is_placeholder;
        let is_query_list = !node.atom_properties.element_list.is_empty()
            || !node.atom_properties.generic_list.is_empty();
        let mut attrs = vec![("id", cdxml_id.to_string()), ("p", fmt_point(point))];
        attrs.push(("Z", object.z_index.to_string()));
        if let Some(color) = &node.highlight_color {
            attrs.push(("highlightColor", self.colors.id_for(color)));
        }
        if !is_query_list
            && !is_plain_carbon
            && node.atomic_number > 0
            && (!is_nickname || node.atomic_number != 6)
        {
            attrs.push(("Element", node.atomic_number.to_string()));
        }
        let imported_node_type = node
            .meta
            .pointer("/import/cdxml/nodeType")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty());
        if is_query_list {
            attrs.push(("NodeType", "ElementList".to_string()));
        } else if node.external_connection.is_some() {
            attrs.push(("NodeType", "ExternalConnectionPoint".to_string()));
        } else if let Some(node_type) = imported_node_type {
            attrs.push(("NodeType", node_type.to_string()));
        } else if is_nickname {
            attrs.push(("NodeType", "Nickname".to_string()));
        }
        if let Some(connection) = node.external_connection.as_ref() {
            if let Some(connection_type) =
                exported_external_connection_type(connection.connection_type)
            {
                attrs.push(("ExternalConnectionType", connection_type.to_string()));
            }
            if let Some(number) = connection.number {
                attrs.push(("ExternalConnectionNum", number.to_string()));
            }
        }
        if let Some(stereo) = enhanced_stereo {
            attrs.push(("EnhancedStereoType", stereo.kind.to_string()));
            if let Some(group_number) = stereo.group_number {
                attrs.push(("EnhancedStereoGroupNum", group_number.to_string()));
            }
        }
        if !node.atom_properties.element_list.is_empty() {
            attrs.push((
                "ElementList",
                format_query_list(
                    &node
                        .atom_properties
                        .element_list
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                    node.atom_properties.element_list_excluded,
                ),
            ));
        }
        if !node.atom_properties.generic_list.is_empty() {
            attrs.push((
                "GenericList",
                format_query_list(
                    &node.atom_properties.generic_list,
                    node.atom_properties.generic_list_excluded,
                ),
            ));
        }
        if let Some(label) = node.label.as_ref() {
            if let Some(display) = imported_cdxml_label_attr(label, "labelDisplay") {
                attrs.push(("LabelDisplay", display.to_string()));
            } else if label.layout.as_deref() == Some("attached-group-center")
                && label.meta.pointer("/import/cdxml").is_none()
            {
                attrs.push(("LabelDisplay", "Center".to_string()));
            }
        }
        if node.charge != 0 {
            attrs.push(("Charge", node.charge.to_string()));
        }
        if let Some(isotope_mass) = node.atom_properties.isotope_mass {
            attrs.push(("Isotope", isotope_mass.to_string()));
        }
        let abundance = match node.atom_properties.isotopic_abundance {
            crate::IsotopicAbundance::Unspecified => None,
            crate::IsotopicAbundance::Any => Some("Any"),
            crate::IsotopicAbundance::Natural => Some("Natural"),
            crate::IsotopicAbundance::Enriched => Some("Enriched"),
            crate::IsotopicAbundance::Deficient => Some("Deficient"),
            crate::IsotopicAbundance::Nonnatural => Some("Nonnatural"),
        };
        if let Some(abundance) = abundance {
            attrs.push(("IsotopicAbundance", abundance.to_string()));
        }
        let effective_radical_count = crate::node_radical_count(node);
        let radical = match (effective_radical_count, &node.atom_properties.radical) {
            (0, _) => None,
            (2, crate::AtomRadical::Singlet)
                if crate::node_attached_electron_symbols(node).is_empty() =>
            {
                Some("Singlet")
            }
            (1, _) => Some("Doublet"),
            (_, _) => Some("Triplet"),
        };
        if let Some(radical) = radical {
            attrs.push(("Radical", radical.to_string()));
        }
        if let Some(atom_number) = node
            .atom_properties
            .atom_number
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            attrs.push(("AtomNumber", atom_number.to_string()));
        }
        if let Some(show) = node.atom_properties.show_atom_number {
            attrs.push((
                "ShowAtomNumber",
                if show { "yes" } else { "no" }.to_string(),
            ));
        }
        if let Some(show) = node.atom_properties.show_atom_stereo {
            attrs.push((
                "ShowAtomStereo",
                if show { "yes" } else { "no" }.to_string(),
            ));
        }
        if let Some(value) = node.atom_properties.free_sites {
            attrs.push(("FreeSites", value.to_string()));
        }
        if let Some(value) = node.atom_properties.show_atom_query {
            attrs.push((
                "ShowAtomQuery",
                if value { "yes" } else { "no" }.to_string(),
            ));
        }
        let ring_bond_count = match node.atom_properties.ring_bond_count {
            crate::RingBondCount::Unspecified => None,
            crate::RingBondCount::NoRingBonds => Some("NoRingBonds"),
            crate::RingBondCount::AsDrawn => Some("AsDrawn"),
            crate::RingBondCount::SimpleRing => Some("SimpleRing"),
            crate::RingBondCount::Fusion => Some("Fusion"),
            crate::RingBondCount::SpiroOrHigher => Some("SpiroOrHigher"),
        };
        if let Some(value) = ring_bond_count {
            attrs.push(("RingBondCount", value.to_string()));
        }
        let unsaturated_bonds = match node.atom_properties.unsaturated_bonds {
            crate::UnsaturatedBonds::Unspecified => None,
            crate::UnsaturatedBonds::MustBeAbsent => Some("MustBeAbsent"),
            crate::UnsaturatedBonds::MustBePresent => Some("MustBePresent"),
        };
        if let Some(value) = unsaturated_bonds {
            attrs.push(("UnsaturatedBonds", value.to_string()));
        }
        if let Some(value) = node.atom_properties.substituents_up_to {
            attrs.push(("SubstituentsUpTo", value.to_string()));
        }
        if let Some(value) = node.atom_properties.substituents_exactly {
            attrs.push(("SubstituentsExactly", value.to_string()));
        }
        let translation = match node.atom_properties.translation {
            crate::QueryTranslation::Equal => None,
            crate::QueryTranslation::Broad => Some("Broad"),
            crate::QueryTranslation::Narrow => Some("Narrow"),
            crate::QueryTranslation::Any => Some("Any"),
        };
        if let Some(value) = translation {
            attrs.push(("Translation", value.to_string()));
        }
        if node.atom_properties.abnormal_valence {
            attrs.push(("AbnormalValence", "yes".to_string()));
        }
        if node.atom_properties.reaction_change {
            attrs.push(("RxnChange", "yes".to_string()));
        }
        let reaction_stereo = match node.atom_properties.reaction_stereo {
            crate::AtomReactionStereo::Unspecified => None,
            crate::AtomReactionStereo::Inversion => Some("Inversion"),
            crate::AtomReactionStereo::Retention => Some("Retention"),
        };
        if let Some(value) = reaction_stereo {
            attrs.push(("RxnStereo", value.to_string()));
        }
        if let Some(value) = node.atom_properties.show_terminal_carbon_label {
            attrs.push((
                "ShowTerminalCarbonLabels",
                if value { "yes" } else { "no" }.to_string(),
            ));
        }
        if let Some(value) = node.atom_properties.show_non_terminal_carbon_label {
            attrs.push((
                "ShowNonTerminalCarbonLabels",
                if value { "yes" } else { "no" }.to_string(),
            ));
        }
        if let Some(num_hydrogens) = cdxml_node_num_hydrogens_for_export(node) {
            attrs.push(("NumHydrogens", num_hydrogens.to_string()));
        }
        if let Some(implicit_hydrogens) = node
            .meta
            .pointer("/import/cdxml/implicitHydrogens")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            attrs.push(("ImplicitHydrogens", implicit_hydrogens.to_string()));
        }
        attrs.push((
            "AS",
            node.atom_properties
                .cip_stereo
                .clone()
                .unwrap_or_else(|| "N".to_string()),
        ));
        let exported_label = node.label.as_ref().filter(|label| {
            label.has_visible_text() && !cdxml_generated_node_label_is_automatic(label)
        });
        if exported_label.is_some() || !node.nmr_assignments.is_empty() {
            write_open_tag(out, 6, "n", attrs);
            if let Some(label) = exported_label {
                self.write_node_label(out, object, node, label);
            }
            for assignment in &node.nmr_assignments {
                self.write_nmr_assignment(out, object, node, assignment);
            }
            out.push_str("      </n>\n");
        } else {
            write_empty_tag(out, 6, "n", attrs);
        }
    }

    fn write_node_label(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        node: &Node,
        label: &NodeLabel,
    ) {
        self.write_node_label_at(out, object, node, label, 8);
    }

    fn write_node_label_at(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        node: &Node,
        label: &NodeLabel,
        indent: usize,
    ) {
        let text = label.source_text.as_deref().unwrap_or(&label.text);
        let Some(font_size) = label.font_size else {
            return;
        };
        let position = label
            .position
            .map(|position| object_local_point(object, position))
            .unwrap_or_else(|| object_local_point(object, node.position));
        let Some(bbox) = label
            .bbox()
            .map(|bbox| translate_bbox(bbox, object.transform.translate))
        else {
            return;
        };
        let label_alignment = imported_cdxml_label_attr(label, "labelAlignment")
            .unwrap_or_else(|| cdxml_node_label_alignment(label));
        let label_justification = imported_cdxml_label_attr(label, "labelJustification")
            .unwrap_or_else(|| cdxml_justification(label.align.as_deref()));
        let label_id = self
            .claim_source_id(
                label
                    .meta
                    .pointer("/import/cdxml/sourceId")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            )
            .unwrap_or_else(|| self.alloc_id());
        let mut attrs = vec![
            ("id", label_id),
            ("p", fmt_point(position)),
            ("BoundingBox", fmt_bbox(bbox)),
            ("LabelAlignment", label_alignment.to_string()),
            ("LabelJustification", label_justification.to_string()),
            (
                "InterpretChemically",
                if cdxml_node_label_interpret_chemically(label) {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
            ),
            ("UTF8Text", text.to_string()),
            (
                "color",
                self.colors
                    .id_for(label.fill.as_deref().unwrap_or("#000000")),
            ),
        ];
        if let Some(justification) = imported_cdxml_label_attr(label, "justification") {
            attrs.push(("Justification", justification.to_string()));
        }
        for (name, xml_name) in [
            ("lineHeight", "LineHeight"),
            ("labelLineHeight", "LabelLineHeight"),
            ("wordWrapWidth", "WordWrapWidth"),
        ] {
            if let Some(value) = imported_cdxml_label_attr(label, name) {
                attrs.push((xml_name, value.to_string()));
            }
        }
        if imported_cdxml_label_attr(label, "labelLineHeight").is_none()
            && imported_cdxml_label_attr(label, "lineHeight").is_none()
            && !imported_cdxml_inherited_label_line_height_is_unchanged(label)
        {
            match label.line_height_mode.as_str() {
                "variable" => attrs.push(("LabelLineHeight", "variable".to_string())),
                "auto" => attrs.push(("LabelLineHeight", "auto".to_string())),
                _ => {
                    if let Some(line_height) = label
                        .line_height
                        .filter(|value| value.is_finite() && *value > 1.0)
                    {
                        attrs.push(("LabelLineHeight", fmt_num(line_height)));
                    }
                }
            }
        }
        if let Some(line_starts) = imported_cdxml_label_attr(label, "lineStarts") {
            attrs.push(("LineStarts", line_starts.to_string()));
        } else if let Some(line_starts) = cdxml_label_line_starts(text) {
            attrs.push(("LineStarts", line_starts));
        }
        write_open_tag(out, indent, "t", attrs);
        self.write_label_runs(out, indent + 2, label, text, font_size);
        writeln!(out, "{:indent$}</t>", "", indent = indent)
            .expect("writing node label close tag should not fail");
    }

    fn write_nmr_assignment(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        node: &Node,
        assignment: &crate::NmrAssignment,
    ) {
        if assignment.validate().is_err() {
            return;
        }
        write_open_tag(
            out,
            8,
            "objecttag",
            vec![
                ("TagType", "String".to_string()),
                ("Name", "/CS/CD/assign".to_string()),
                (
                    "Value",
                    format!(
                        "{}-{},",
                        fmt_num(assignment.range_low_ppm),
                        fmt_num(assignment.range_high_ppm)
                    ),
                ),
            ],
        );
        self.write_node_label_at(out, object, node, &assignment.label, 10);
        out.push_str("        </objecttag>\n");
    }

    fn write_bond(
        &mut self,
        out: &mut String,
        bond: &Bond,
        cdxml_id: &str,
        node_ids: &BTreeMap<String, String>,
        crossing_scope: &str,
    ) {
        self.entity_ids
            .insert(bond.id.clone(), cdxml_id.to_string());
        let (Some(begin), Some(end)) = (node_ids.get(&bond.begin), node_ids.get(&bond.end)) else {
            return;
        };
        let mut attrs = vec![
            ("id", cdxml_id.to_string()),
            (
                "Z",
                bond.meta
                    .pointer("/import/cdxml/z")
                    .and_then(Value::as_i64)
                    .unwrap_or(1)
                    .to_string(),
            ),
            ("B", begin.clone()),
            ("E", end.clone()),
            (
                "Order",
                if bond.properties.query_orders.len() >= 2 {
                    bond.properties
                        .query_orders
                        .iter()
                        .map(|value| value.cdxml_value())
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    preserved_cdxml_bond_order(bond)
                        .unwrap_or_else(|| bond.order.max(1).to_string())
                },
            ),
        ];
        let topology = match bond.properties.topology {
            crate::BondTopology::Unspecified => None,
            crate::BondTopology::Ring => Some("Ring"),
            crate::BondTopology::Chain => Some("Chain"),
            crate::BondTopology::RingOrChain => Some("RingOrChain"),
        };
        if let Some(value) = topology {
            attrs.push(("Topology", value.to_string()));
        }
        let reaction = match bond.properties.reaction_participation {
            crate::BondReactionParticipation::Unspecified => None,
            crate::BondReactionParticipation::ReactionCenter => Some("ReactionCenter"),
            crate::BondReactionParticipation::MakeOrBreak => Some("MakeOrBreak"),
            crate::BondReactionParticipation::ChangeType => Some("ChangeType"),
            crate::BondReactionParticipation::MakeAndChange => Some("MakeAndChange"),
            crate::BondReactionParticipation::NotReactionCenter => Some("NotReactionCenter"),
            crate::BondReactionParticipation::NoChange => Some("NoChange"),
            crate::BondReactionParticipation::Unmapped => Some("Unmapped"),
        };
        if let Some(value) = reaction {
            attrs.push(("RxnParticipation", value.to_string()));
        }
        let absolute_stereo = match bond.properties.absolute_stereo {
            crate::BondAbsoluteStereo::Unspecified => None,
            crate::BondAbsoluteStereo::None => Some("N"),
            crate::BondAbsoluteStereo::E => Some("E"),
            crate::BondAbsoluteStereo::Z => Some("Z"),
        };
        if let Some(value) = absolute_stereo {
            attrs.push(("BS", value.to_string()));
        }
        for (name, value) in [
            ("ShowBondQuery", bond.properties.show_query),
            ("ShowBondRxn", bond.properties.show_reaction),
            ("ShowBondStereo", bond.properties.show_stereo),
        ] {
            if let Some(value) = value {
                attrs.push((name, if value { "yes" } else { "no" }.to_string()));
            }
        }
        let crossing_bonds: Vec<_> = imported_cdxml_crossing_bonds(bond)
            .filter_map(|source_id| {
                self.bond_ids
                    .get(&(crossing_scope.to_string(), source_id.to_string()))
                    .cloned()
            })
            .collect();
        if !crossing_bonds.is_empty() {
            attrs.push(("CrossingBonds", crossing_bonds.join(" ")));
        }
        if let Some(value) = bond_endpoint_attachment(bond, "begin") {
            attrs.push(("BeginAttach", value.to_string()));
        }
        if let Some(value) = bond_endpoint_attachment(bond, "end") {
            attrs.push(("EndAttach", value.to_string()));
        }
        if canonicalizes_topology_only_aromatic_dash(bond) {
            // ChemDraw lays out coordinate-free Order=1.5 + Dash transport
            // bonds as ordinary solid single bonds and saves that normalized
            // result. Emit the explicit default so interchange merging cannot
            // resurrect the superseded source Display.
            attrs.push(("Display", "Solid".to_string()));
        } else if let Some(display) = cdxml_bond_display(bond, false) {
            attrs.push(("Display", display.to_string()));
        } else if let Some(display) = bond
            .meta
            .pointer("/import/cdxml/display")
            .and_then(Value::as_str)
            .filter(|display| !display.is_empty())
        {
            // Order=1.5 and Display are independent. Preserve an authored
            // display when it remains the document's live display semantics.
            attrs.push(("Display", display.to_string()));
        }
        if let Some(display2) = cdxml_bond_display(bond, true) {
            attrs.push(("Display2", display2.to_string()));
        }
        if let Some(stroke) = &bond.stroke {
            attrs.push(("color", self.colors.id_for(stroke)));
        }
        if let Some(color) = &bond.highlight_color {
            attrs.push(("highlightColor", self.colors.id_for(color)));
        }
        if let Some(double) = &bond.double {
            attrs.push((
                "DoublePosition",
                match double.placement {
                    crate::DoubleBondPlacement::Left => "Left",
                    crate::DoubleBondPlacement::Right => "Right",
                    crate::DoubleBondPlacement::Center => "Center",
                }
                .to_string(),
            ));
        }
        if bond.stroke_width > 0.0 {
            attrs.push(("LineWidth", fmt_num(bond.stroke_width)));
        }
        if let Some(value) = bond.bold_width {
            attrs.push(("BoldWidth", fmt_num(value)));
        }
        if let Some(value) = bond.hash_spacing {
            attrs.push(("HashSpacing", fmt_num(value)));
        }
        if let Some(value) = bond.bond_spacing {
            attrs.push(("BondSpacing", fmt_num(value)));
        }
        if let Some(value) = bond.margin_width {
            attrs.push(("MarginWidth", fmt_num(value)));
        }
        write_empty_tag(out, 6, "b", attrs);
    }

    fn prepare_bond_ids(&mut self) {
        let mut keys = Vec::new();
        collect_cdxml_bond_export_keys(self.document, &self.document.objects, &mut keys);
        for key in keys {
            if !self.bond_ids.contains_key(&key) {
                let cdxml_id = self
                    .claim_source_id(self.imported_bond_source_id(&key))
                    .unwrap_or_else(|| self.alloc_id());
                self.bond_ids.insert(key, cdxml_id);
            }
        }
    }

    fn prepare_annotation_basis_ids(&mut self) {
        let node_ids = self
            .document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.clone()))
            .collect::<Vec<_>>();
        for node_id in node_ids {
            if !self.entity_ids.contains_key(&node_id) {
                let cdxml_id = self
                    .claim_source_id(Some(node_id.clone()))
                    .unwrap_or_else(|| self.alloc_id());
                self.node_ids.insert(node_id.clone(), cdxml_id.clone());
                self.entity_ids.insert(node_id, cdxml_id);
            }
        }
        let annotation_ids = self
            .document
            .objects
            .iter()
            .filter(|object| {
                matches!(
                    object.kind(),
                    crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
                )
            })
            .map(|object| object.id.clone())
            .collect::<Vec<_>>();
        for object_id in annotation_ids {
            if !self.entity_ids.contains_key(&object_id) {
                let cdxml_id = self.alloc_id();
                self.entity_ids.insert(object_id, cdxml_id);
            }
        }
    }

    fn write_line_object(&mut self, out: &mut String, object: &SceneObject) {
        let points = payload_points_cdxml(&object.payload, "points");
        if points.len() < 2 {
            return;
        }
        let tail = points[0].translated(crate::Vector::new(
            object.transform.translate[0],
            object.transform.translate[1],
        ));
        let head = points[points.len() - 1].translated(crate::Vector::new(
            object.transform.translate[0],
            object.transform.translate[1],
        ));
        let arrow = object.payload.extra.get("arrowHead");
        let head_position = cdxml_arrow_endpoint_position(&object.payload, arrow, "head", "end");
        let tail_position = cdxml_arrow_endpoint_position(&object.payload, arrow, "tail", "start");
        let has_head = head_position != "None";
        let has_tail = tail_position != "None";
        let style = object_style(self.document, object);
        let stroke = style
            .and_then(|style| style_string_value(style, "stroke"))
            .unwrap_or_else(|| "#000000".to_string());
        let stroke_width = style
            .and_then(|style| style_number_value(style, "strokeWidth"))
            .unwrap_or(crate::DEFAULT_BOND_STROKE);
        let dashed = style
            .and_then(|style| style_number_array(style, "dashArray"))
            .is_some_and(|dash_array| !dash_array.is_empty());
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("Head3D", fmt_point3(head)),
            ("Tail3D", fmt_point3(tail)),
            ("LineWidth", fmt_num(stroke_width)),
            ("color", self.colors.id_for(&stroke)),
            ("Z", object.z_index.to_string()),
        ];
        let is_arrow = arrow.is_some()
            || object
                .meta
                .pointer("/import/cdxml/kind")
                .and_then(Value::as_str)
                == Some("arrow");
        if is_arrow || has_head || has_tail {
            let bold = arrow
                .and_then(|value| value.get("bold"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match (bold, dashed) {
                (true, true) => attrs.push(("LineType", "Bold Dashed".to_string())),
                (true, false) => attrs.push(("LineType", "Bold".to_string())),
                (false, true) => attrs.push(("LineType", "Dashed".to_string())),
                (false, false) => {}
            }
            if let Some(fill_type) = arrow
                .and_then(|value| value.get("fillType").or_else(|| value.get("fill_type")))
                .and_then(Value::as_str)
                .and_then(cdxml_arrow_fill_type)
            {
                attrs.push(("FillType", fill_type.to_string()));
            }
            if let Some(bbox) =
                payload_nested_bbox_cdxml(&object.payload, "arrowGeometry", "boundingBox")
            {
                attrs.push((
                    "BoundingBox",
                    fmt_bbox(translate_bbox(bbox, object.transform.translate)),
                ));
            }
            if let Some(center) =
                payload_nested_point_cdxml(&object.payload, "arrowGeometry", "center")
            {
                attrs.push((
                    "Center3D",
                    fmt_point3(center.translated(crate::Vector::new(
                        object.transform.translate[0],
                        object.transform.translate[1],
                    ))),
                ));
            }
            if let Some(major) =
                payload_nested_point_cdxml(&object.payload, "arrowGeometry", "majorAxisEnd")
            {
                attrs.push((
                    "MajorAxisEnd3D",
                    fmt_point3(major.translated(crate::Vector::new(
                        object.transform.translate[0],
                        object.transform.translate[1],
                    ))),
                ));
            }
            if let Some(minor) =
                payload_nested_point_cdxml(&object.payload, "arrowGeometry", "minorAxisEnd")
            {
                attrs.push((
                    "MinorAxisEnd3D",
                    fmt_point3(minor.translated(crate::Vector::new(
                        object.transform.translate[0],
                        object.transform.translate[1],
                    ))),
                ));
            }
            attrs.push(("ArrowheadHead", head_position.to_string()));
            attrs.push(("ArrowheadTail", tail_position.to_string()));
            let arrow_kind = cdxml_arrow_kind(arrow);
            attrs.push((
                "ArrowheadType",
                cdxml_arrowhead_type_attr(arrow_kind).to_string(),
            ));
            if let Some(value) = arrow
                .and_then(|value| value.get("length"))
                .and_then(Value::as_f64)
            {
                attrs.push(("HeadSize", fmt_num(cdxml_arrow_size_attribute(value))));
            }
            if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("centerLength")
                        .or_else(|| value.get("center_length"))
                })
                .and_then(Value::as_f64)
            {
                let value = cdxml_arrow_size_attribute(value);
                attrs.push(("ArrowheadCenterSize", fmt_num(value)));
            }
            if arrow_kind == "Equilibrium" {
                let value = arrow
                    .and_then(|value| {
                        value
                            .get("shaftSpacing")
                            .or_else(|| value.get("shaft_spacing"))
                    })
                    .and_then(Value::as_f64)
                    .unwrap_or(3.0);
                let value = cdxml_arrow_size_attribute(value);
                attrs.push(("ArrowShaftSpacing", fmt_num(value)));
                if let Some(value) = cdxml_arrow_equilibrium_ratio(arrow) {
                    attrs.push(("ArrowEquilibriumRatio", fmt_num(value * 100.0)));
                }
            } else if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("shaftSpacing")
                        .or_else(|| value.get("shaft_spacing"))
                })
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
            {
                attrs.push((
                    "ArrowShaftSpacing",
                    fmt_num(cdxml_arrow_size_attribute(value)),
                ));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("width"))
                .and_then(Value::as_f64)
            {
                attrs.push(("ArrowheadWidth", fmt_num(cdxml_arrow_size_attribute(value))));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("curve"))
                .and_then(Value::as_f64)
                .filter(|value| value.abs() > crate::EPSILON)
            {
                attrs.push(("AngularSize", fmt_num(value)));
            }
            if let Some(value) = arrow
                .and_then(|value| {
                    value
                        .get("curveSpacing")
                        .or_else(|| value.get("curve_spacing"))
                })
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
            {
                attrs.push(("CurveSpacing", fmt_num(cdxml_arrow_size_attribute(value))));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("noGo").or_else(|| value.get("no_go")))
                .and_then(Value::as_str)
                .and_then(cdxml_arrow_no_go)
            {
                attrs.push(("NoGo", value.to_string()));
            }
            if arrow
                .and_then(|value| value.get("dipole"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                attrs.push(("Dipole", "yes".to_string()));
            }
            if arrow
                .and_then(|value| value.get("closed"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                attrs.push(("Closed", "yes".to_string()));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("source"))
                .and_then(cdxml_arrow_object_reference)
            {
                attrs.push(("ArrowSource", value));
            }
            if let Some(value) = arrow
                .and_then(|value| value.get("target"))
                .and_then(cdxml_arrow_object_reference)
            {
                attrs.push(("ArrowTarget", value));
            }
            write_empty_tag(out, 4, "arrow", attrs);
        } else {
            if dashed {
                attrs.push(("LineType", "Dashed".to_string()));
            }
            attrs.push(("GraphicType", "Line".to_string()));
            write_empty_tag(out, 4, "graphic", attrs);
        }
    }

    fn write_curve_object(&mut self, out: &mut String, object: &SceneObject) {
        let points = payload_points_cdxml(&object.payload, "curvePoints");
        if points.len() < 6 || !(points.len() - 3).is_multiple_of(3) {
            return;
        }
        let translated = points
            .iter()
            .map(|point| {
                point.translated(crate::Vector::new(
                    object.transform.translate[0],
                    object.transform.translate[1],
                ))
            })
            .collect::<Vec<_>>();
        let curve_points = translated
            .iter()
            .flat_map(|point| [fmt_num(point.x), fmt_num(point.y)])
            .collect::<Vec<_>>()
            .join(" ");
        let style = object_style(self.document, object);
        let stroke = style
            .and_then(|style| style_string_value(style, "stroke"))
            .unwrap_or_else(|| "#000000".to_string());
        let stroke_width = style
            .and_then(|style| style_number_value(style, "strokeWidth"))
            .unwrap_or(crate::DEFAULT_BOND_STROKE);
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("CurvePoints", curve_points),
            (
                "CurveType",
                object
                    .payload
                    .extra
                    .get("curveType")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .to_string(),
            ),
            ("LineWidth", fmt_num(stroke_width)),
            ("color", self.colors.id_for(&stroke)),
            ("Z", object.z_index.to_string()),
        ];
        let head =
            payload_string_cdxml(&object.payload, "head").unwrap_or_else(|| "none".to_string());
        let tail =
            payload_string_cdxml(&object.payload, "tail").unwrap_or_else(|| "none".to_string());
        if head != "none" {
            attrs.push((
                "ArrowheadHead",
                cdxml_curve_endpoint_name(&head).to_string(),
            ));
        }
        if tail != "none" {
            attrs.push((
                "ArrowheadTail",
                cdxml_curve_endpoint_name(&tail).to_string(),
            ));
        }
        if head != "none" || tail != "none" {
            attrs.push((
                "ArrowheadType",
                payload_string_cdxml(&object.payload, "arrowheadType")
                    .unwrap_or_else(|| "Solid".to_string()),
            ));
        }
        for (payload_key, attribute) in [
            ("headLength", "HeadSize"),
            ("headCenterLength", "ArrowheadCenterSize"),
            ("headWidth", "ArrowheadWidth"),
        ] {
            if let Some(value) = object
                .payload
                .extra
                .get(payload_key)
                .and_then(Value::as_f64)
            {
                attrs.push((attribute, fmt_num(value * 100.0)));
            }
        }
        write_empty_tag(out, 4, "curve", attrs);
    }

    fn write_shape_object(&mut self, out: &mut String, object: &SceneObject) {
        if object.payload.bio_shape.is_some() {
            self.write_bio_shape_object(out, object);
            return;
        }
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        let kind =
            payload_string_cdxml(&object.payload, "kind").unwrap_or_else(|| "rect".to_string());
        let style = object_style(self.document, object);
        let stroke = style.and_then(|style| style_nullable_string_value(style, "stroke"));
        let fill = style.and_then(|style| style_nullable_string_value(style, "fill"));
        let color = fill.as_deref().or(stroke.as_deref()).unwrap_or("#000000");
        let filled = fill.is_some() && stroke.is_none();
        let shaded = style
            .and_then(|style| style.get("shaded"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let shadowed = style
            .and_then(|style| style.get("shadow"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let dashed = style
            .and_then(|style| style_number_array(style, "dashArray"))
            .is_some_and(|dash| !dash.is_empty());
        let shadow_size = style
            .and_then(|style| style_number_value(style, "shadowSize"))
            .unwrap_or(4.0);
        if matches!(kind.as_str(), "circle" | "ellipse") {
            let Some(center) = payload_point_cdxml(&object.payload, "center") else {
                return;
            };
            let Some(major) = payload_point_cdxml(&object.payload, "majorAxisEnd") else {
                return;
            };
            let Some(minor) = payload_point_cdxml(&object.payload, "minorAxisEnd") else {
                return;
            };
            let bbox = [x, y, x + width, y + height];
            let mut oval_type = String::new();
            if kind == "circle" {
                oval_type.push_str("Circle");
            }
            push_cdxml_shape_type_flag(&mut oval_type, dashed, "Dashed");
            push_cdxml_shape_type_flag(&mut oval_type, shaded, "Shaded");
            push_cdxml_shape_type_flag(&mut oval_type, filled, "Filled");
            push_cdxml_shape_type_flag(&mut oval_type, shadowed, "Shadowed");
            let mut attrs = vec![
                ("id", self.object_cdxml_id(object)),
                ("GraphicType", "Oval".to_string()),
                ("BoundingBox", fmt_bbox(bbox)),
                ("Center3D", fmt_point3(center)),
                ("MajorAxisEnd3D", fmt_point3(major)),
                ("MinorAxisEnd3D", fmt_point3(minor)),
                ("OvalType", oval_type),
                ("color", self.colors.id_for(color)),
                ("Z", object.z_index.to_string()),
            ];
            if let Some(stroke_width) =
                style.and_then(|style| style_number_value(style, "strokeWidth"))
            {
                attrs.push(("LineWidth", fmt_num(stroke_width)));
            }
            if shadowed {
                attrs.push(("ShadowSize", fmt_num(shadow_size * 100.0)));
            }
            write_empty_tag(out, 4, "graphic", attrs);
            return;
        }
        let bbox = [
            object.transform.translate[0] + x,
            object.transform.translate[1] + y,
            object.transform.translate[0] + x + width,
            object.transform.translate[1] + y + height,
        ];
        if kind == "orbital" {
            self.write_orbital_shape_object(out, object, color, style);
            return;
        }
        if kind == "gelPlate" {
            self.write_gel_electrophoresis_shape_object(out, object, bbox);
            return;
        }
        if kind == "plasmidMap" {
            self.write_plasmid_map_shape_object(out, object);
            return;
        }
        if kind == "tlcPlate" {
            let plate_id = self.object_cdxml_id(object);
            let color_id = self.colors.id_for(color);
            let origin_fraction = object
                .payload
                .extra
                .get("originFraction")
                .and_then(Value::as_f64)
                .unwrap_or(0.1);
            let solvent_fraction = object
                .payload
                .extra
                .get("solventFrontFraction")
                .and_then(Value::as_f64)
                .unwrap_or(0.1);
            let bool_attr = |key: &str, default_value: bool| {
                if object
                    .payload
                    .extra
                    .get(key)
                    .and_then(Value::as_bool)
                    .unwrap_or(default_value)
                {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }
            };
            write_open_tag(
                out,
                4,
                "tlcplate",
                vec![
                    ("id", plate_id),
                    ("OriginFraction", fmt_num(origin_fraction)),
                    ("SolventFrontFraction", fmt_num(solvent_fraction)),
                    ("ShowOrigin", bool_attr("showOrigin", true)),
                    ("ShowSolventFront", bool_attr("showSolventFront", true)),
                    ("TopLeft", fmt_point(Point::new(bbox[0], bbox[1]))),
                    ("TopRight", fmt_point(Point::new(bbox[2], bbox[1]))),
                    ("BottomRight", fmt_point(Point::new(bbox[2], bbox[3]))),
                    ("BottomLeft", fmt_point(Point::new(bbox[0], bbox[3]))),
                    ("ShowBorders", bool_attr("showBorders", true)),
                    ("ShowSideTicks", bool_attr("showSideTicks", true)),
                    ("Transparent", bool_attr("transparent", false)),
                    ("BoundingBox", fmt_bbox(bbox)),
                    ("Z", object.z_index.to_string()),
                    ("color", color_id.clone()),
                    (
                        "alpha",
                        fmt_num(
                            object
                                .payload
                                .extra
                                .get("alpha")
                                .and_then(Value::as_f64)
                                .unwrap_or(1.0)
                                .clamp(0.0, 1.0)
                                * 65535.0,
                        ),
                    ),
                    (
                        "HashSpacing",
                        fmt_num(
                            object
                                .payload
                                .extra
                                .get("dashSpacing")
                                .and_then(Value::as_f64)
                                .unwrap_or(self.defaults.hash_spacing),
                        ),
                    ),
                    (
                        "BoldWidth",
                        fmt_num(
                            object
                                .payload
                                .extra
                                .get("boldWidth")
                                .and_then(Value::as_f64)
                                .unwrap_or(self.defaults.bold_width),
                        ),
                    ),
                    (
                        "MarginWidth",
                        fmt_num(
                            object
                                .payload
                                .extra
                                .get("marginWidth")
                                .and_then(Value::as_f64)
                                .unwrap_or(self.defaults.margin_width),
                        ),
                    ),
                    (
                        "LabelFont",
                        object
                            .payload
                            .extra
                            .get("labelFont")
                            .and_then(Value::as_u64)
                            .unwrap_or(3)
                            .to_string(),
                    ),
                    (
                        "LabelSize",
                        fmt_num(
                            object
                                .payload
                                .extra
                                .get("labelSize")
                                .and_then(Value::as_f64)
                                .unwrap_or(10.0),
                        ),
                    ),
                    (
                        "LabelFace",
                        object
                            .payload
                            .extra
                            .get("labelFace")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                            .to_string(),
                    ),
                ],
            );
            if let Some(lanes) = object.payload.extra.get("lanes").and_then(Value::as_array) {
                for lane in lanes {
                    write_open_tag(
                        out,
                        6,
                        "tlclane",
                        vec![
                            ("id", self.alloc_id()),
                            (
                                "Visible",
                                if lane.get("visible").and_then(Value::as_bool).unwrap_or(true) {
                                    "yes"
                                } else {
                                    "no"
                                }
                                .to_string(),
                            ),
                        ],
                    );
                    if let Some(spots) = lane.get("spots").and_then(Value::as_array) {
                        for spot in spots {
                            let mut attrs = vec![
                                ("id", self.alloc_id()),
                                (
                                    "Rf",
                                    fmt_num(spot.get("rf").and_then(Value::as_f64).unwrap_or(0.15)),
                                ),
                                (
                                    "Tail",
                                    fmt_num(
                                        spot.get("tail").and_then(Value::as_f64).unwrap_or(0.0),
                                    ),
                                ),
                                (
                                    "Width",
                                    fmt_num(self.cdxml_tlc_spot_extent(
                                        spot.get("width").and_then(Value::as_f64),
                                    )),
                                ),
                                (
                                    "Height",
                                    fmt_num(self.cdxml_tlc_spot_extent(
                                        spot.get("height").and_then(Value::as_f64),
                                    )),
                                ),
                                (
                                    "CurveType",
                                    spot.get("curveType")
                                        .and_then(Value::as_i64)
                                        .unwrap_or(128)
                                        .to_string(),
                                ),
                                ("color", color_id.clone()),
                            ];
                            if spot.get("showRf").and_then(Value::as_bool).unwrap_or(false) {
                                attrs.push(("ShowRf", "yes".to_string()));
                            }
                            if !spot.get("visible").and_then(Value::as_bool).unwrap_or(true) {
                                attrs.push(("Visible", "no".to_string()));
                            }
                            if let Some(alpha) = spot.get("alpha").and_then(Value::as_f64) {
                                attrs.push((
                                    "alpha",
                                    fmt_num((alpha.clamp(0.0, 1.0) * 65535.0).round()),
                                ));
                            }
                            if let Some(spot_color) = spot.get("color").and_then(Value::as_str) {
                                attrs.retain(|(name, _)| *name != "color");
                                attrs.push(("color", self.colors.id_for(spot_color)));
                            }
                            if let Some(z) = spot.get("zIndex").and_then(Value::as_i64) {
                                attrs.push(("Z", z.to_string()));
                            }
                            write_empty_tag(out, 8, "tlcspot", attrs);
                        }
                    }
                    write_indent(out, 6);
                    out.push_str("</tlclane>\n");
                }
            }
            write_indent(out, 4);
            out.push_str("</tlcplate>\n");
            return;
        }
        let mut rectangle_type = String::new();
        if kind == "roundRect" {
            rectangle_type.push_str("RoundEdge");
        }
        if kind == "rect" && !dashed && !shaded && !filled && !shadowed {
            rectangle_type.push_str("Plain");
        }
        push_cdxml_shape_type_flag(&mut rectangle_type, dashed, "Dashed");
        push_cdxml_shape_type_flag(&mut rectangle_type, shaded, "Shaded");
        push_cdxml_shape_type_flag(&mut rectangle_type, filled, "Filled");
        push_cdxml_shape_type_flag(&mut rectangle_type, shadowed, "Shadow");
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("GraphicType", "Rectangle".to_string()),
            ("BoundingBox", fmt_bbox(bbox)),
            ("RectangleType", rectangle_type),
            ("color", self.colors.id_for(color)),
            ("Z", object.z_index.to_string()),
        ];
        if let Some(radius) = object
            .payload
            .extra
            .get("cornerRadius")
            .and_then(Value::as_f64)
        {
            attrs.push(("CornerRadius", fmt_num(radius * 100.0)));
        }
        if let Some(stroke_width) = style.and_then(|style| style_number_value(style, "strokeWidth"))
        {
            attrs.push(("LineWidth", fmt_num(stroke_width)));
        }
        if shadowed {
            attrs.push(("ShadowSize", fmt_num(shadow_size * 100.0)));
        }
        write_empty_tag(out, 4, "graphic", attrs);
    }

    fn write_bio_shape_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some(data) = object.payload.bio_shape.as_ref() else {
            return;
        };
        let world = |point: [f64; 3]| {
            let scaled_x = point[0] * object.transform.scale[0];
            let scaled_y = point[1] * object.transform.scale[1];
            let angle = object.transform.rotate.to_radians();
            [
                object.transform.translate[0] + scaled_x * angle.cos() - scaled_y * angle.sin(),
                object.transform.translate[1] + scaled_x * angle.sin() + scaled_y * angle.cos(),
                point[2],
            ]
        };
        let center = world(data.center);
        let major = world(data.major_axis_end);
        let minor = world(data.minor_axis_end);
        let major_vector = [major[0] - center[0], major[1] - center[1]];
        let minor_vector = [minor[0] - center[0], minor[1] - center[1]];
        let corners = [
            [
                center[0] + major_vector[0] + minor_vector[0],
                center[1] + major_vector[1] + minor_vector[1],
            ],
            [
                center[0] + major_vector[0] - minor_vector[0],
                center[1] + major_vector[1] - minor_vector[1],
            ],
            [
                center[0] - major_vector[0] + minor_vector[0],
                center[1] - major_vector[1] + minor_vector[1],
            ],
            [
                center[0] - major_vector[0] - minor_vector[0],
                center[1] - major_vector[1] - minor_vector[1],
            ],
        ];
        let min_x = corners
            .iter()
            .map(|point| point[0])
            .fold(f64::INFINITY, f64::min);
        let min_y = corners
            .iter()
            .map(|point| point[1])
            .fold(f64::INFINITY, f64::min);
        let max_x = corners
            .iter()
            .map(|point| point[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = corners
            .iter()
            .map(|point| point[1])
            .fold(f64::NEG_INFINITY, f64::max);
        let fmt_xyz = |point: [f64; 3]| {
            format!(
                "{} {} {}",
                fmt_num(point[0]),
                fmt_num(point[1]),
                fmt_num(point[2])
            )
        };
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("xyz", fmt_xyz(center)),
            ("BoundingBox", fmt_bbox([min_x, min_y, max_x, max_y])),
            ("BioShapeType", data.kind.cdxml_name().to_string()),
            ("MajorAxisEnd3D", fmt_xyz(major)),
            ("MinorAxisEnd3D", fmt_xyz(minor)),
            ("FillType", data.fill_type.cdxml_name().to_string()),
            ("LineType", data.line_type.cdxml_name().to_string()),
            ("LineWidth", fmt_num(data.line_width)),
            ("BoldWidth", fmt_num(data.bold_width)),
            ("MarginWidth", fmt_num(data.margin_width)),
            ("HashSpacing", fmt_num(data.hash_spacing)),
            ("FadePercent", fmt_num(data.fade_percent * 100.0)),
            ("color", self.colors.id_for(&data.color)),
            (
                "Visible",
                if object.visible { "yes" } else { "no" }.to_string(),
            ),
            ("Z", object.z_index.to_string()),
        ];
        if let Some(alpha) = data.alpha {
            attrs.push(("alpha", fmt_num(alpha * 100.0)));
        }
        macro_rules! push_parameter {
            ($field:ident, $name:literal) => {
                if let Some(value) = data.parameters.$field {
                    attrs.push(($name, fmt_num(value)));
                }
            };
        }
        push_parameter!(cylinder_distance, "CylinderDistance");
        push_parameter!(cylinder_height, "CylinderHeight");
        push_parameter!(cylinder_width, "CylinderWidth");
        push_parameter!(dna_wave_height, "DNAWaveHeight");
        push_parameter!(dna_wave_length, "DNAWaveLength");
        push_parameter!(dna_wave_offset, "DNAWaveOffset");
        push_parameter!(dna_wave_width, "DNAWaveWidth");
        push_parameter!(enzyme_height, "EnzymeHeight");
        push_parameter!(enzyme_receptor_size, "EnzymeReceptorSize");
        push_parameter!(enzyme_width, "EnzymeWidth");
        push_parameter!(golgi_height, "GolgiHeight");
        push_parameter!(golgi_length, "GolgiLength");
        push_parameter!(golgi_width, "GolgiWidth");
        push_parameter!(gprotein_lower_height, "GproteinLowerHeight");
        push_parameter!(gprotein_upper_height, "GproteinUpperHeight");
        push_parameter!(helix_protein_extra, "HelixProteinExtra");
        push_parameter!(immunoglobulin_height, "ImmunoglobinHeight");
        push_parameter!(immunoglobulin_width, "ImmunoglobinWidth");
        push_parameter!(membrane_element_size, "MembraneElementSize");
        push_parameter!(membrane_end_angle, "MembraneEndAngle");
        push_parameter!(membrane_major_axis_size, "MembraneMajorAxisSize");
        push_parameter!(membrane_minor_axis_size, "MembraneMinorAxisSize");
        push_parameter!(membrane_start_angle, "MembraneStartAngle");
        push_parameter!(neck_height, "NeckHeight");
        push_parameter!(neck_width, "NeckWidth");
        push_parameter!(pipe_width, "PipeWidth");
        write_empty_tag(out, 4, "bioshape", attrs);
    }

    fn write_gel_electrophoresis_shape_object(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        bbox: [f64; 4],
    ) {
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        let Some(gel) = object.payload.gel_electrophoresis.as_ref() else {
            return;
        };
        let color_id = self.colors.id_for(&gel.color);
        let corners =
            gel.corners
                .unwrap_or([[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]]);
        let absolute = corners.map(|point| {
            Point::new(
                object.transform.translate[0] + x + point[0],
                object.transform.translate[1] + y + point[1],
            )
        });
        let yes_no = |value: bool| if value { "yes" } else { "no" }.to_string();
        write_open_tag(
            out,
            4,
            "gepplate",
            vec![
                ("id", self.object_cdxml_id(object)),
                ("BoundingBox", fmt_bbox(bbox)),
                ("TopLeft", fmt_point(absolute[0])),
                ("TopRight", fmt_point(absolute[1])),
                ("BottomRight", fmt_point(absolute[2])),
                ("BottomLeft", fmt_point(absolute[3])),
                ("StartRange", fmt_num(gel.start_range)),
                ("EndRange", fmt_num(gel.end_range)),
                ("UnitID", gel.unit_id.to_string()),
                ("ShowScale", yes_no(gel.show_scale)),
                ("ShowBorders", yes_no(gel.show_borders)),
                ("Transparent", yes_no(gel.transparent)),
                ("LineWidth", fmt_num(gel.line_width)),
                ("BoldWidth", fmt_num(gel.bold_width)),
                ("AxisWidth", fmt_num(gel.axis_width)),
                ("MarginWidth", fmt_num(gel.margin_width)),
                ("HashSpacing", fmt_num(gel.hash_spacing)),
                ("LabelFont", gel.label_font.to_string()),
                ("LabelSize", fmt_num(gel.label_size)),
                ("LabelFace", gel.label_face.to_string()),
                ("LabelsAngle", fmt_num(gel.labels_angle)),
                ("LabelText", gel.label_text.clone()),
                (
                    "alpha",
                    fmt_num((gel.alpha.clamp(0.0, 1.0) * 65535.0).round()),
                ),
                ("Visible", yes_no(object.visible)),
                ("Z", object.z_index.to_string()),
                ("color", color_id),
            ],
        );
        for lane in &gel.lanes {
            let lane_id = self
                .claim_source_id(Some(lane.id.clone()))
                .unwrap_or_else(|| self.alloc_id());
            write_open_tag(
                out,
                6,
                "geplane",
                vec![
                    ("id", lane_id),
                    ("LabelText", lane.label_text.clone()),
                    ("Visible", yes_no(lane.visible)),
                ],
            );
            for band in &lane.bands {
                let band_id = self
                    .claim_source_id(Some(band.id.clone()))
                    .unwrap_or_else(|| self.alloc_id());
                write_empty_tag(
                    out,
                    8,
                    "gepband",
                    vec![
                        ("id", band_id),
                        ("BandValue", fmt_num(band.value)),
                        (
                            "Width",
                            fmt_num(self.cdxml_tlc_spot_extent(Some(band.width))),
                        ),
                        (
                            "Height",
                            fmt_num(self.cdxml_tlc_spot_extent(Some(band.height))),
                        ),
                        ("CurveType", band.curve_type.to_string()),
                        ("ShowValue", yes_no(band.show_value)),
                        ("Visible", yes_no(band.visible)),
                        (
                            "alpha",
                            fmt_num((band.alpha.clamp(0.0, 1.0) * 65535.0).round()),
                        ),
                        ("color", self.colors.id_for(&band.color)),
                        ("Z", band.z_index.to_string()),
                    ],
                );
            }
            write_indent(out, 6);
            out.push_str("</geplane>\n");
        }
        write_indent(out, 4);
        out.push_str("</gepplate>\n");
    }

    fn write_plasmid_map_shape_object(&mut self, out: &mut String, object: &SceneObject) {
        let (Some([x, y, width, height]), Some(plasmid)) =
            (object.payload.bbox, object.payload.plasmid_map.as_ref())
        else {
            return;
        };
        let center = Point::new(
            object.transform.translate[0] + x + width * 0.5,
            object.transform.translate[1] + y + height * 0.5,
        );
        let color_id = self.colors.id_for(&plasmid.color);
        write_open_tag(
            out,
            4,
            "plasmidmap",
            vec![
                ("id", self.object_cdxml_id(object)),
                ("NumberBasePairs", plasmid.number_base_pairs.to_string()),
                ("RingRadius", fmt_num((plasmid.radius * 65536.0).round())),
                ("p", fmt_point(center)),
                ("LineWidth", fmt_num(plasmid.line_width)),
                ("BoldWidth", fmt_num(plasmid.bold_width)),
                ("MarginWidth", fmt_num(plasmid.margin_width)),
                ("LabelFont", plasmid.label_font.to_string()),
                ("LabelSize", fmt_num(plasmid.label_size)),
                ("LabelFace", plasmid.label_face.to_string()),
                ("color", color_id.clone()),
                (
                    "Visible",
                    if object.visible { "yes" } else { "no" }.to_string(),
                ),
                ("Z", object.z_index.to_string()),
            ],
        );
        self.write_plasmid_map_center(out, plasmid, center, &color_id);
        self.write_plasmid_map_regions(out, plasmid, center);
        self.write_plasmid_map_markers(out, plasmid, center);
        write_indent(out, 4);
        out.push_str("</plasmidmap>\n");
    }

    fn write_plasmid_map_center(
        &mut self,
        out: &mut String,
        plasmid: &crate::PlasmidMapData,
        center: Point,
        color_id: &str,
    ) {
        if plasmid.show_base_pairs {
            write_open_tag(
                out,
                6,
                "t",
                vec![
                    ("p", fmt_point(center)),
                    ("CaptionJustification", "Center".to_string()),
                    ("Justification", "Center".to_string()),
                    ("LineHeight", "auto".to_string()),
                ],
            );
            write_text_tag(
                out,
                8,
                "s",
                vec![
                    ("font", plasmid.label_font.to_string()),
                    ("size", fmt_num(plasmid.label_size)),
                    ("color", color_id.to_string()),
                ],
                &format!("{} bp", plasmid.number_base_pairs),
            );
            write_indent(out, 6);
            out.push_str("</t>\n");
        }
        let major = Point::new(center.x + plasmid.radius, center.y);
        let minor = Point::new(center.x, center.y + plasmid.radius);
        write_empty_tag(
            out,
            6,
            "graphic",
            vec![
                (
                    "BoundingBox",
                    fmt_bbox([major.x, major.y, center.x, center.y]),
                ),
                ("GraphicType", "Oval".to_string()),
                ("OvalType", "Circle".to_string()),
                ("Center3D", fmt_point3(center)),
                ("MajorAxisEnd3D", fmt_point3(major)),
                ("MinorAxisEnd3D", fmt_point3(minor)),
                ("color", color_id.to_string()),
            ],
        );
    }

    fn write_plasmid_map_regions(
        &mut self,
        out: &mut String,
        plasmid: &crate::PlasmidMapData,
        center: Point,
    ) {
        let major = Point::new(center.x + plasmid.radius, center.y);
        let minor = Point::new(center.x, center.y + plasmid.radius);
        for region in &plasmid.regions {
            let start_point = plasmid_map_point(
                center,
                plasmid.radius + region.offset,
                plasmid.angle_degrees(region.start),
            );
            let end_point = plasmid_map_point(
                center,
                plasmid.radius + region.offset,
                plasmid.angle_degrees(region.end),
            );
            let mut attrs = vec![
                ("id", self.alloc_id()),
                ("RegionStart", region.start.to_string()),
                ("RegionEnd", region.end.to_string()),
                ("RegionOffset", fmt_num((region.offset * 100.0).round())),
                (
                    "FillType",
                    if region.filled {
                        "Solid"
                    } else if region.shaded {
                        "Shaded"
                    } else if region.faded {
                        "Faded"
                    } else {
                        "None"
                    }
                    .to_string(),
                ),
                ("ArrowheadType", "Hollow".to_string()),
                ("HeadSize", "600".to_string()),
                ("ArrowheadCenterSize", "600".to_string()),
                ("ArrowheadWidth", "150".to_string()),
                ("AngularSize", "300".to_string()),
                ("ArrowShaftSpacing", fmt_num((region.width * 100.0).round())),
                ("Head3D", fmt_point3(end_point)),
                ("Tail3D", fmt_point3(start_point)),
                ("Center3D", fmt_point3(center)),
                ("MajorAxisEnd3D", fmt_point3(major)),
                ("MinorAxisEnd3D", fmt_point3(minor)),
                (
                    "alpha",
                    fmt_num((region.alpha.clamp(0.0, 1.0) * 65535.0).round()),
                ),
                ("color", self.colors.id_for(&region.color)),
            ];
            if region.arrow_at_start {
                attrs.push(("ArrowheadHead", "Full".to_string()));
            }
            if region.arrow_at_end {
                attrs.push(("ArrowheadTail", "Full".to_string()));
            }
            write_empty_tag(out, 6, "plasmidregion", attrs);
        }
    }

    fn write_plasmid_map_markers(
        &mut self,
        out: &mut String,
        plasmid: &crate::PlasmidMapData,
        center: Point,
    ) {
        for marker in &plasmid.markers {
            let position_angle = plasmid.angle_degrees(marker.position);
            let label_angle = marker.label_angle.unwrap_or(position_angle);
            let ring_point = plasmid_map_point(center, plasmid.radius, position_angle);
            let label_point = plasmid_map_point(
                center,
                plasmid.radius + marker.offset.max(plasmid.margin_width),
                label_angle,
            );
            let marker_angle =
                marker.position as f64 / plasmid.number_base_pairs as f64 * 600.0 * 65536.0;
            write_open_tag(
                out,
                6,
                "plasmidmarker",
                vec![
                    ("id", self.alloc_id()),
                    ("MarkerOffset", fmt_num((marker.offset * 100.0).round())),
                    ("MarkerAngle", fmt_num(marker_angle.round())),
                    ("TagType", "Long".to_string()),
                    ("CaptionJustification", "Center".to_string()),
                    ("Name", "marker".to_string()),
                    ("Value", marker.position.to_string()),
                    ("color", self.colors.id_for(&marker.color)),
                ],
            );
            write_open_tag(
                out,
                8,
                "t",
                vec![
                    ("p", fmt_point(label_point)),
                    ("CaptionJustification", "Center".to_string()),
                    ("Justification", "Center".to_string()),
                    ("LineHeight", "auto".to_string()),
                ],
            );
            write_text_tag(
                out,
                10,
                "s",
                vec![
                    ("font", plasmid.label_font.to_string()),
                    ("size", fmt_num(plasmid.label_size)),
                    ("color", self.colors.id_for(&marker.color)),
                ],
                &marker.label,
            );
            write_indent(out, 8);
            out.push_str("</t>\n");
            write_empty_tag(
                out,
                8,
                "arrow",
                vec![
                    ("id", self.alloc_id()),
                    ("FillType", "None".to_string()),
                    ("ArrowheadType", "Solid".to_string()),
                    ("Head3D", fmt_point3(ring_point)),
                    ("Tail3D", fmt_point3(label_point)),
                    ("color", self.colors.id_for(&marker.color)),
                ],
            );
            write_indent(out, 6);
            out.push_str("</plasmidmarker>\n");
        }
    }

    fn write_orbital_shape_object(
        &mut self,
        out: &mut String,
        object: &SceneObject,
        color: &str,
        style: Option<&Value>,
    ) {
        let template = payload_string_cdxml(&object.payload, "orbitalTemplate")
            .unwrap_or_else(|| "s".to_string());
        let render_style = payload_string_cdxml(&object.payload, "orbitalStyle")
            .unwrap_or_else(|| "hollow".to_string());
        let phase = payload_string_cdxml(&object.payload, "orbitalPhase")
            .unwrap_or_else(|| "plus".to_string());
        let orbital_type = cdxml_orbital_type(&template, &render_style, &phase);
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("GraphicType", "Orbital".to_string()),
            ("OrbitalType", orbital_type.to_string()),
            ("color", self.colors.id_for(color)),
            ("Z", object.z_index.to_string()),
        ];
        if matches!(template.as_str(), "s" | "oval") {
            let Some(center) = payload_point_cdxml(&object.payload, "center") else {
                return;
            };
            let Some(major) = payload_point_cdxml(&object.payload, "majorAxisEnd") else {
                return;
            };
            let Some(minor) = payload_point_cdxml(&object.payload, "minorAxisEnd") else {
                return;
            };
            let radius_x = center.distance(major);
            let radius_y = center.distance(minor);
            let bbox = [
                center.x - radius_x,
                center.y - radius_y,
                center.x + radius_x,
                center.y + radius_y,
            ];
            attrs.push(("BoundingBox", fmt_bbox(bbox)));
            attrs.push(("Center3D", fmt_point3(center)));
            attrs.push(("MajorAxisEnd3D", fmt_point3(major)));
            attrs.push(("MinorAxisEnd3D", fmt_point3(minor)));
            if template == "s" {
                let oval_type = match render_style.as_str() {
                    "shaded" => "Circle Shaded",
                    "filled" => "Circle Filled",
                    _ => "Circle",
                };
                attrs.push(("OvalType", oval_type.to_string()));
            } else {
                let oval_type = match render_style.as_str() {
                    "shaded" => "Shaded",
                    "filled" => "Filled",
                    _ => "",
                };
                if !oval_type.is_empty() {
                    attrs.push(("OvalType", oval_type.to_string()));
                }
            }
            write_empty_tag(out, 4, "graphic", attrs);
            return;
        }
        let Some(start) = payload_point_cdxml(&object.payload, "axisStart") else {
            return;
        };
        let Some(end) = payload_point_cdxml(&object.payload, "axisEnd") else {
            return;
        };
        attrs.push(("BoundingBox", fmt_bbox([end.x, end.y, start.x, start.y])));
        if let Some(stroke_width) = style.and_then(|style| style_number_value(style, "strokeWidth"))
        {
            attrs.push(("LineWidth", fmt_num(stroke_width)));
        }
        write_empty_tag(out, 4, "graphic", attrs);
    }

    fn write_bracket_object(&mut self, out: &mut String, object: &SceneObject) {
        let Some([x, y, width, height]) = object.payload.bbox else {
            return;
        };
        let bbox = [
            object.transform.translate[0] + x,
            object.transform.translate[1] + y,
            object.transform.translate[0] + x + width,
            object.transform.translate[1] + y + height,
        ];
        let kind =
            payload_string_cdxml(&object.payload, "kind").unwrap_or_else(|| "round".to_string());
        if object.object_type == "symbol" {
            let color = payload_string_cdxml(&object.payload, "fill")
                .unwrap_or_else(|| "#000000".to_string());
            let color_id = self.colors.id_for(&color);
            let symbol_type = match kind.as_str() {
                "double-dagger" => "DoubleDagger",
                "dagger" => "Dagger",
                "circle-plus" => "CirclePlus",
                "plus" => "Plus",
                "radical-cation" => "RadicalCation",
                "lone-pair" => "LonePair",
                "circle-minus" => "CircleMinus",
                "minus" => "Minus",
                "radical-anion" => "RadicalAnion",
                "electron" => "Electron",
                _ => "Dagger",
            };
            let style = object
                .payload
                .extra
                .get("symbolStyle")
                .and_then(Value::as_str)
                .map(crate::cdxml_symbol_style_from_name)
                .unwrap_or(crate::CdxmlSymbolStyle::Default);
            let anchor_width = object
                .payload
                .extra
                .get("symbolAnchorWidth")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| crate::cdxml_symbol_anchor_width(&kind, style));
            let anchor_height = object
                .payload
                .extra
                .get("symbolAnchorHeight")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| crate::cdxml_symbol_anchor_height(&kind));
            let center_x = (bbox[0] + bbox[2]) * 0.5;
            let center_y = (bbox[1] + bbox[3]) * 0.5;
            let symbol_bbox =
                cdxml_symbol_anchor_bbox(center_x, center_y, anchor_width, anchor_height);
            let attrs = vec![
                ("id", self.object_cdxml_id(object)),
                ("GraphicType", "Symbol".to_string()),
                ("SymbolType", symbol_type.to_string()),
                ("color", color_id),
                ("BoundingBox", fmt_bbox(symbol_bbox)),
                ("Z", object.z_index.to_string()),
            ];
            let represented_node = object
                .payload
                .extra
                .get("attachedAtomId")
                .and_then(Value::as_str)
                .and_then(|node_id| self.node_ids.get(node_id));
            let represented_attribute = object
                .payload
                .extra
                .get("representAttribute")
                .and_then(Value::as_str);
            if let (Some(node_id), Some(attribute)) = (represented_node, represented_attribute) {
                write_open_tag(out, 4, "graphic", attrs);
                write_empty_tag(
                    out,
                    6,
                    "represent",
                    vec![
                        ("attribute", attribute.to_string()),
                        ("object", node_id.clone()),
                    ],
                );
                out.push_str("    </graphic>\n");
            } else {
                write_empty_tag(out, 4, "graphic", attrs);
            }
            return;
        }

        let color = payload_string_cdxml(&object.payload, "stroke")
            .unwrap_or_else(|| "#000000".to_string());
        let color_id = self.colors.id_for(&color);
        let bracket_type = match kind.as_str() {
            "square" => "Square",
            "curly" => "Curly",
            _ => "Round",
        };
        if let Some(side) = object.payload.extra.get("side").and_then(Value::as_str) {
            // CDX/CDXML Graphic BoundingBox is an ordered pair of bracket
            // spine endpoints. Derive those endpoints from the live rotated
            // side geometry; serializing the axis-aligned payload box loses
            // arbitrary-angle and horizontal brackets.
            let handle_x = match (kind.as_str(), side) {
                ("square", "left") | ("round", "right") | ("curly", "right") => 0.0,
                _ => width,
            };
            let center = Point::new(
                object.transform.translate[0] + x + width * 0.5,
                object.transform.translate[1] + y + height * 0.5,
            );
            let top = crate::rotate_point_around(
                Point::new(
                    object.transform.translate[0] + x + handle_x,
                    object.transform.translate[1] + y,
                ),
                center,
                object.transform.rotate,
            );
            let bottom = crate::rotate_point_around(
                Point::new(
                    object.transform.translate[0] + x + handle_x,
                    object.transform.translate[1] + y + height,
                ),
                center,
                object.transform.rotate,
            );
            let bracket_bbox = if side == "right" {
                [top.x, top.y, bottom.x, bottom.y]
            } else {
                [bottom.x, bottom.y, top.x, top.y]
            };
            let lip_size = object
                .payload
                .extra
                .get("lipSize")
                .and_then(Value::as_i64)
                .unwrap_or(60);
            let mut attrs = vec![
                ("id", self.object_cdxml_id(object)),
                ("GraphicType", "Bracket".to_string()),
                ("BracketType", bracket_type.to_string()),
                ("color", color_id),
                ("BoundingBox", fmt_bbox(bracket_bbox)),
                ("LipSize", lip_size.to_string()),
                ("Z", object.z_index.to_string()),
            ];
            if let Some(stroke_width) = object
                .payload
                .extra
                .get("strokeWidth")
                .and_then(Value::as_f64)
            {
                attrs.push(("LineWidth", fmt_num(stroke_width)));
            }
            write_empty_tag(out, 4, "graphic", attrs);
            return;
        }
        let left_x = bbox[0];
        let right_x = bbox[2];
        let top = bbox[1];
        let bottom = bbox[3];
        write_empty_tag(
            out,
            4,
            "graphic",
            vec![
                ("id", self.object_cdxml_id(object)),
                ("GraphicType", "Bracket".to_string()),
                ("BracketType", bracket_type.to_string()),
                ("color", color_id.clone()),
                ("BoundingBox", fmt_bbox([left_x, bottom, left_x, top])),
                ("LipSize", "60".to_string()),
                ("Z", object.z_index.to_string()),
            ],
        );
        write_empty_tag(
            out,
            4,
            "graphic",
            vec![
                ("id", self.alloc_id()),
                ("GraphicType", "Bracket".to_string()),
                ("BracketType", bracket_type.to_string()),
                ("color", color_id),
                ("BoundingBox", fmt_bbox([right_x, top, right_x, bottom])),
                ("LipSize", "60".to_string()),
                ("Z", (object.z_index + 1).to_string()),
            ],
        );
    }

    fn write_text_object(&mut self, out: &mut String, object: &SceneObject) {
        if object
            .meta
            .get("synthetic")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            // Enhanced-stereo labels synthesized from native node fields are
            // a derived display, not an independent CDXML Text object. The
            // node fields regenerate the display in ChemDraw and on import.
            return;
        }
        let text = payload_string_cdxml(&object.payload, "text").unwrap_or_default();
        let is_chemical_property_display = self
            .document
            .chemical_properties
            .iter()
            .any(|property| property.display_object_id.as_deref() == Some(object.id.as_str()));
        if text.trim().is_empty() && !is_chemical_property_display {
            return;
        }
        let style = object_style(self.document, object);
        let Some(font_size) = object
            .payload
            .extra
            .get("fontSize")
            .and_then(Value::as_f64)
            .or_else(|| style.and_then(|style| style_number_value(style, "fontSize")))
        else {
            return;
        };
        let color = style
            .and_then(|style| style_nullable_string_value(style, "fill"))
            .unwrap_or_else(|| "#000000".to_string());
        let font_family = style
            .and_then(|style| style_string_value(style, "fontFamily"))
            .unwrap_or_else(|| "Arial".to_string());
        let Some(box_value) = payload_bbox_cdxml(&object.payload, "box").or(object.payload.bbox)
        else {
            return;
        };
        let baseline_offset = object
            .payload
            .extra
            .get("baselineOffset")
            .and_then(Value::as_f64)
            .unwrap_or(font_size * 0.82);
        let anchor_offset_x = object
            .payload
            .extra
            .get("anchorOffsetX")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let anchor = Point::new(
            object.transform.translate[0] + anchor_offset_x,
            object.transform.translate[1] + baseline_offset,
        );
        let mut bbox = [
            object.transform.translate[0] + box_value[0],
            object.transform.translate[1] + box_value[1],
            object.transform.translate[0] + box_value[0] + box_value[2],
            object.transform.translate[1] + box_value[1] + box_value[3],
        ];
        if object.meta.get("role").and_then(Value::as_str) == Some("enhanced_stereo") {
            if let Some((node_point, vector)) = object
                .meta
                .get("attachedNodeId")
                .and_then(Value::as_str)
                .and_then(|node_id| {
                    Some((
                        document_node_world_point(self.document, node_id)?,
                        payload_point_cdxml(&object.payload, "automaticPositioningVector")?,
                    ))
                })
            {
                let center = node_point.translated(crate::Vector::new(vector.x, vector.y));
                bbox = [
                    center.x - box_value[2] * 0.5,
                    center.y - box_value[3] * 0.5,
                    center.x + box_value[2] * 0.5,
                    center.y + box_value[3] * 0.5,
                ];
            }
        }
        let mut attrs = vec![
            ("id", self.object_cdxml_id(object)),
            ("p", fmt_point(anchor)),
            ("BoundingBox", fmt_bbox(bbox)),
            (
                "CaptionJustification",
                cdxml_justification(payload_string_cdxml(&object.payload, "align").as_deref())
                    .to_string(),
            ),
            ("Z", object.z_index.to_string()),
            ("UTF8Text", text.clone()),
        ];
        if !object.visible {
            attrs.push(("Visible", "no".to_string()));
        }
        for (name, xml_name) in [
            ("justification", "Justification"),
            ("lineHeight", "LineHeight"),
            ("captionLineHeight", "CaptionLineHeight"),
            ("wordWrapWidth", "WordWrapWidth"),
            ("lineStarts", "LineStarts"),
        ] {
            if let Some(value) = imported_cdxml_object_attr(object, name) {
                attrs.push((xml_name, value.to_string()));
            }
        }
        let inherited_caption_line_height = self
            .document
            .document
            .meta
            .pointer("/import/cdxml/defaults/captionLineHeight")
            .and_then(Value::as_str);
        let should_materialize_caption_line_height = object.meta.pointer("/import/cdxml").is_none()
            || (imported_cdxml_object_attr(object, "lineHeight").is_some()
                && inherited_caption_line_height.is_none());
        if imported_cdxml_object_attr(object, "captionLineHeight").is_none()
            && should_materialize_caption_line_height
        {
            match object
                .payload
                .extra
                .get("lineHeightMode")
                .and_then(Value::as_str)
                .unwrap_or("fixed")
            {
                "variable" => attrs.push(("CaptionLineHeight", "variable".to_string())),
                "auto" => attrs.push(("CaptionLineHeight", "auto".to_string())),
                _ => {
                    if let Some(line_height) = object
                        .payload
                        .extra
                        .get("lineHeight")
                        .and_then(Value::as_f64)
                    {
                        attrs.push((
                            "CaptionLineHeight",
                            fmt_num(line_height.clamp(0.0, i16::MAX as f64)),
                        ));
                    }
                }
            }
        }
        write_open_tag(out, 4, "t", attrs);
        let runs = object
            .payload
            .extra
            .get("runs")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
            .unwrap_or_default();
        self.write_runs(out, 6, &runs, &text, font_size, &color, &font_family);
        out.push_str("    </t>\n");
    }

    fn write_label_runs(
        &mut self,
        out: &mut String,
        indent: usize,
        label: &NodeLabel,
        default_text: &str,
        default_size: f64,
    ) {
        let source_runs = label_source_runs_for_export(label);
        let runs = source_runs.as_deref().unwrap_or(&label.runs);
        self.write_runs(
            out,
            indent,
            runs,
            default_text,
            default_size,
            label.fill.as_deref().unwrap_or("#000000"),
            label.font_family.as_deref().unwrap_or("Arial"),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn write_runs(
        &mut self,
        out: &mut String,
        indent: usize,
        runs: &[LabelRun],
        default_text: &str,
        default_size: f64,
        default_color: &str,
        default_font_family: &str,
    ) {
        if runs.is_empty() {
            let attrs = vec![
                ("font", self.fonts.id_for(default_font_family)),
                ("size", fmt_num(default_size)),
                ("color", self.colors.id_for(default_color)),
            ];
            write_text_tag(out, indent, "s", attrs, default_text);
            return;
        }
        for run in runs {
            if run.text.is_empty() {
                continue;
            }
            let mut face = 0;
            if run.font_weight.unwrap_or(400) >= 600 {
                face |= 1;
            }
            if run.font_style.as_deref() == Some("italic") {
                face |= 2;
            }
            if run.underline.unwrap_or(false) {
                face |= 4;
            }
            if run.outline.unwrap_or(false) {
                face |= 8;
            }
            if run.shadow.unwrap_or(false) {
                face |= 16;
            }
            match run.script.as_deref() {
                Some("subscript") => face |= 32,
                Some("superscript") => face |= 64,
                Some("chemical") => face |= 96,
                _ => {}
            }
            let mut attrs = vec![
                (
                    "font",
                    self.fonts
                        .id_for(run.font_family.as_deref().unwrap_or(default_font_family)),
                ),
                ("size", fmt_num(run.font_size.unwrap_or(default_size))),
                (
                    "color",
                    self.colors.id_for(run.fill.as_deref().unwrap_or("#000000")),
                ),
            ];
            if face != 0 {
                attrs.push(("face", face.to_string()));
            }
            write_text_tag(out, indent, "s", attrs, &run.text);
        }
    }

    fn alloc_id(&mut self) -> String {
        while self.reserved_ids.contains(&self.next_id) || self.used_ids.contains(&self.next_id) {
            self.next_id += 1;
        }
        let id = self.next_id;
        self.used_ids.insert(id);
        self.next_id += 1;
        id.to_string()
    }

    fn claim_source_id(&mut self, source_id: Option<String>) -> Option<String> {
        let source_id = source_id?;
        let numeric_id = source_id.parse::<u64>().ok().filter(|id| *id > 0)?;
        self.used_ids.insert(numeric_id).then_some(source_id)
    }

    fn imported_bond_source_id(&self, key: &(String, String)) -> Option<String> {
        self.document
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "molecule")
            .filter(|object| cdxml_bond_crossing_scope(object) == key.0)
            .filter_map(|object| {
                object
                    .payload
                    .resource_ref
                    .as_ref()
                    .and_then(|resource_ref| self.document.resources.get(resource_ref))
                    .and_then(|resource| resource.data.as_fragment())
            })
            .flat_map(|fragment| fragment.bonds.iter())
            .find(|bond| bond.id == key.1)
            .and_then(|bond| {
                bond.meta
                    .pointer("/import/cdxml/sourceId")
                    .and_then(Value::as_str)
            })
            .map(ToString::to_string)
    }

    fn object_cdxml_id(&mut self, object: &SceneObject) -> String {
        if let Some(id) = self.entity_ids.get(&object.id) {
            return id.clone();
        }
        let imported_source_id = object
            .meta
            .pointer("/import/cdxml/sourceId")
            .or_else(|| object.meta.pointer("/import/cdxml/id"))
            .or_else(|| object.meta.pointer("/import/cdxml/groupId"))
            .and_then(Value::as_str)
            .or_else(|| {
                [
                    "textId",
                    "graphicId",
                    "curveId",
                    "tableId",
                    "stoichiometryGridId",
                    "tlcPlateId",
                    "gelPlateId",
                    "bioShapeId",
                    "spectrumId",
                    "groupId",
                ]
                .into_iter()
                .find_map(|name| object.meta.get(name).and_then(Value::as_str))
            })
            .map(ToString::to_string);
        if let Some(id) = self.claim_source_id(imported_source_id) {
            self.entity_ids.insert(object.id.clone(), id.clone());
            return id;
        }
        let id = if self
            .document
            .chemical_properties
            .iter()
            .any(|property| property.display_object_id.as_deref() == Some(object.id.as_str()))
        {
            object
                .meta
                .get("textId")
                .and_then(Value::as_str)
                .filter(|id| id.parse::<u64>().is_ok())
                .map(ToString::to_string)
                .unwrap_or_else(|| self.alloc_id())
        } else {
            self.alloc_id()
        };
        self.entity_ids.insert(object.id.clone(), id.clone());
        id
    }

    fn write_chemical_properties(&mut self, out: &mut String) {
        for property in &self.document.chemical_properties {
            let id = property
                .source_id
                .clone()
                .unwrap_or_else(|| self.alloc_id());
            let mut attrs = vec![("id", id)];
            if let Some(value) = property.property_type.cdxml_value() {
                attrs.push(("ChemicalPropertyType", value));
            }
            if property.is_active {
                attrs.push(("ChemicalPropertyIsActive", "yes".to_string()));
            }
            if let Some(display_id) = property
                .display_object_id
                .as_deref()
                .and_then(|entity_id| self.entity_ids.get(entity_id))
            {
                attrs.push(("ChemicalPropertyDisplayID", display_id.clone()));
            }
            let basis = property
                .basis_entity_ids
                .iter()
                .filter_map(|entity_id| self.entity_ids.get(entity_id).cloned())
                .chain(property.unresolved_basis_ids.iter().cloned())
                .collect::<Vec<_>>();
            if !basis.is_empty() {
                attrs.push(("BasisObjects", basis.join(" ")));
            }
            write_open_tag(out, 4, "chemicalproperty", attrs);
            out.push_str("</chemicalproperty>\n");
        }
    }

    fn cdxml_tlc_spot_extent(&self, extent: Option<f64>) -> f64 {
        let Some(extent) = extent else {
            return 327680.0;
        };
        if extent > 1024.0 {
            return extent;
        }
        (extent / self.editing_scale.max(crate::EPSILON) * 65536.0).round()
    }
}

fn constraint_label_position(
    document: &ChemSemaDocument,
    object: &SceneObject,
    constraint: &crate::ConstraintData,
) -> Option<Point> {
    let automatic =
        || match crate::geometry_constraints::evaluate_annotation(document, object).ok()? {
            crate::geometry_constraints::EvaluatedAnnotation::Distance { start, end, .. } => Some(
                Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5 - 3.0),
            ),
            crate::geometry_constraints::EvaluatedAnnotation::Angle { points, .. } => {
                points.get(1).copied()
            }
            crate::geometry_constraints::EvaluatedAnnotation::ExclusionSphere {
                center,
                radius_angstrom,
            } => Some(Point::new(
                center.x,
                center.y
                    - radius_angstrom
                        * document
                            .style
                            .defaults
                            .get("bondLength")
                            .copied()
                            .unwrap_or(crate::DEFAULT_BOND_LENGTH_PT)
                        / 1.5,
            )),
            crate::geometry_constraints::EvaluatedAnnotation::Geometry(_) => None,
        };
    match constraint.display.positioning_type {
        crate::AnnotationPositioningType::Auto => automatic(),
        crate::AnnotationPositioningType::Absolute | crate::AnnotationPositioningType::Angle => {
            constraint
                .display
                .position
                .map(|point| Point::new(point[0], point[1]))
        }
        crate::AnnotationPositioningType::Offset => {
            let base = automatic()?;
            let [dx, dy] = constraint.display.positioning_offset?;
            Some(Point::new(base.x + dx, base.y + dy))
        }
    }
}

fn collect_interchange_numeric_ids(node: &crate::InterchangeObject, ids: &mut BTreeSet<u64>) {
    if let Some(id) = node
        .id
        .as_deref()
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
    {
        ids.insert(id);
    }
    for child in &node.children {
        collect_interchange_numeric_ids(child, ids);
    }
}

#[derive(Debug, Clone)]
struct CdxmlFontTable {
    fonts: Vec<(String, String)>,
    ids: BTreeMap<String, String>,
    next_id: u64,
}

impl Default for CdxmlFontTable {
    fn default() -> Self {
        let mut table = Self {
            fonts: Vec::new(),
            ids: BTreeMap::new(),
            next_id: 4,
        };
        table.insert_with_id("3", "Arial");
        table
    }
}

impl CdxmlFontTable {
    fn normalize_name(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "Arial".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn insert_with_id(&mut self, id: &str, name: &str) {
        let normalized = Self::normalize_name(name);
        self.ids.insert(normalized.clone(), id.to_string());
        self.fonts.push((id.to_string(), normalized));
    }

    fn ensure(&mut self, name: &str) -> String {
        let normalized = Self::normalize_name(name);
        if let Some(id) = self.ids.get(&normalized) {
            return id.clone();
        }
        let id = self.next_id.to_string();
        self.next_id += 1;
        self.insert_with_id(&id, &normalized);
        id
    }

    fn id_for(&self, name: &str) -> String {
        let normalized = Self::normalize_name(name);
        self.ids
            .get(&normalized)
            .cloned()
            .unwrap_or_else(|| "3".to_string())
    }

    fn fonts(&self) -> &[(String, String)] {
        &self.fonts
    }
}
