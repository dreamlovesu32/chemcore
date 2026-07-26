use crate::{
    Bond, ChemSemaDocument, DocumentTextStyle, LabelRun, MoleculeFragment, Node, NodeLabel,
    ObjectPayload, Point, ResourceData, SceneObject,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;
use std::collections::BTreeMap;
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
mod mapping;
mod payload;
mod resources;
mod xml_writer;

use defaults::*;
use interchange::*;
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
    let generated = CdxmlDocumentWriter::new(document).write();
    let Some(source) = document.interchange.get("cdxml") else {
        return generated;
    };
    let Ok(mut root) = super::parse_xml_tree(&generated) else {
        return generated;
    };
    let mut source_root = source.root.clone();
    retain_native_chemical_properties(&mut source_root, &document.chemical_properties);
    retain_native_annotations(&mut source_root, &document.objects);
    merge_interchange_tree(&mut root, &source_root);
    serialize_cdxml_tree(&root)
}

struct CdxmlDocumentWriter<'a> {
    document: &'a ChemSemaDocument,
    next_id: u64,
    node_ids: BTreeMap<String, String>,
    bond_ids: BTreeMap<(String, String), String>,
    entity_ids: BTreeMap<String, String>,
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
        let has_native_reference_objects = !document.chemical_properties.is_empty()
            || document.scene_objects().iter().any(|object| {
                matches!(
                    object.kind(),
                    crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
                )
            });
        let preserved_interchange_id = if has_native_reference_objects {
            max_interchange_numeric_id(document)
        } else {
            0
        };
        let preserved_next_id = document
            .chemical_properties
            .iter()
            .filter_map(|property| property.source_id.as_deref())
            .filter_map(|id| id.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
            .max(preserved_interchange_id)
            .saturating_add(1);
        Self {
            document,
            next_id: preserved_next_id.max(1),
            node_ids: BTreeMap::new(),
            bond_ids: BTreeMap::new(),
            entity_ids: BTreeMap::new(),
            colors,
            fonts,
            defaults,
            editing_scale: cdxml_editing_scale(document),
        }
    }

    fn write(mut self) -> String {
        self.prepare_bond_ids();
        self.prepare_annotation_basis_ids();
        let page = &self.document.document.page;
        let width = page.width.max(1.0);
        let height = page.height.max(1.0);
        let root_bbox = format!("0 0 {} {}", fmt_num(width), fmt_num(height));
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
            fmt_margins(self.defaults.print_margins),
            self.defaults.color,
            self.colors.background_id(),
        )
        .expect("writing CDXML root should not fail");
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
        writeln!(
            out,
            "  <page id=\"{}\" BoundingBox=\"{}\" HeaderPosition=\"36\" FooterPosition=\"36\" PrintTrimMarks=\"yes\" HeightPages=\"1\" WidthPages=\"1\" Width=\"{}\" Height=\"{}\">",
            self.alloc_id(),
            root_bbox,
            fmt_num(width),
            fmt_num(height)
        )
        .expect("writing CDXML page should not fail");

        let mut objects: Vec<&SceneObject> = self
            .document
            .objects
            .iter()
            .filter(|object| object.visible)
            .collect();
        objects.sort_by(|a, b| a.z_index.cmp(&b.z_index).then_with(|| a.id.cmp(&b.id)));
        self.write_scene_objects(&mut out, &objects);
        self.write_chemical_properties(&mut out);

        out.push_str("  </page>\n");
        out.push_str("</CDXML>\n");
        out
    }

    fn write_scene_object(&mut self, out: &mut String, object: &SceneObject) {
        let attached_node_id = object.meta.get("attachedNodeId").and_then(Value::as_str);
        let annotation_role = object.meta.get("role").and_then(Value::as_str);
        if object.object_type == "text"
            && attached_node_id.is_some()
            && (annotation_role.is_some_and(|role| matches!(role, "atom_number" | "stereo"))
                || (annotation_role == Some("query")
                    && attached_node_id.is_some_and(|node_id| {
                        document_node(self.document, node_id)
                            .is_some_and(node_has_native_query_annotation)
                    })))
        {
            // These are cached displays of node semantics. The node attributes
            // below are authoritative and ChemDraw regenerates the object tags.
            return;
        }
        match object.kind() {
            crate::SceneObjectKind::Molecule => self.write_molecule_object(out, object),
            crate::SceneObjectKind::Line => self.write_line_object(out, object),
            crate::SceneObjectKind::Curve => self.write_curve_object(out, object),
            crate::SceneObjectKind::Shape => self.write_shape_object(out, object),
            crate::SceneObjectKind::Table => self.write_table_object(out, object),
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
                if content.visible {
                    self.write_scene_object(out, content);
                }
            }
            write_indent(out, 6);
            out.push_str("</page>\n");
        }
        write_indent(out, 4);
        out.push_str("</table>\n");
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
        let (attribute, data_base64) = if resource.resource_type == "image" {
            let Some(image) = resource.data.as_image() else {
                return;
            };
            let attribute = match image.mime_type.as_str() {
                "image/png" => "PNG",
                "image/jpeg" => "JPEG",
                "image/gif" => "GIF",
                "image/tiff" => "TIFF",
                "image/bmp" => "BMP",
                _ => return,
            };
            (attribute, image.data_base64)
        } else if resource.resource_type == "embedded-object" {
            let ResourceData::Json(value) = &resource.data else {
                return;
            };
            let Some(attribute) = value.get("format").and_then(Value::as_str) else {
                return;
            };
            if !matches!(
                attribute,
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
            let Some(data_base64) = value.get("dataBase64").and_then(Value::as_str) else {
                return;
            };
            (attribute, data_base64.to_string())
        } else {
            return;
        };
        let Ok(bytes) = BASE64.decode(data_base64.as_bytes()) else {
            return;
        };
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
            (attribute, encode_hex_bytes(&bytes)),
        ];
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
            .filter(|child| child.visible)
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

        let fragment_id = self.alloc_id();
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
                let cdxml_id = self
                    .entity_ids
                    .get(&node.id)
                    .cloned()
                    .unwrap_or_else(|| self.alloc_id());
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
        if node.label.as_ref().is_some_and(NodeLabel::has_visible_text)
            || !node.nmr_assignments.is_empty()
        {
            write_open_tag(out, 6, "n", attrs);
            if let Some(label) = node.label.as_ref().filter(|label| label.has_visible_text()) {
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
        let mut attrs = vec![
            ("id", self.alloc_id()),
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
        } else if let Some(line_starts) = cdxml_label_line_starts(label) {
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
        if bond
            .meta
            .pointer("/import/cdxml/aromatic")
            .and_then(Value::as_bool)
            == Some(true)
        {
            attrs.push(("Display", "Dash".to_string()));
        } else if let Some(display) = cdxml_bond_display(bond, false) {
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
                let cdxml_id = self.alloc_id();
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
                let cdxml_id = self.alloc_id();
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
                    ("BoundingBox", fmt_bbox(bbox)),
                    ("Z", object.z_index.to_string()),
                    ("color", color_id.clone()),
                ],
            );
            if let Some(lanes) = object.payload.extra.get("lanes").and_then(Value::as_array) {
                for lane in lanes {
                    write_open_tag(out, 6, "tlclane", vec![("id", self.alloc_id())]);
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
            let bracket_x = match (kind.as_str(), side) {
                ("round", "right") => bbox[0],
                ("round", _) => bbox[2],
                (_, "right") => bbox[2],
                _ => bbox[0],
            };
            let bracket_bbox = match side {
                "right" => [bracket_x, bbox[1], bracket_x, bbox[3]],
                _ => [bracket_x, bbox[3], bracket_x, bbox[1]],
            };
            write_empty_tag(
                out,
                4,
                "graphic",
                vec![
                    ("id", self.object_cdxml_id(object)),
                    ("GraphicType", "Bracket".to_string()),
                    ("BracketType", bracket_type.to_string()),
                    ("color", color_id),
                    ("BoundingBox", fmt_bbox(bracket_bbox)),
                    ("LipSize", "60".to_string()),
                    ("Z", object.z_index.to_string()),
                ],
            );
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
        let bbox = [
            object.transform.translate[0] + box_value[0],
            object.transform.translate[1] + box_value[1],
            object.transform.translate[0] + box_value[0] + box_value[2],
            object.transform.translate[1] + box_value[1] + box_value[3],
        ];
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
        let id = self.next_id;
        self.next_id += 1;
        id.to_string()
    }

    fn object_cdxml_id(&mut self, object: &SceneObject) -> String {
        if let Some(id) = self.entity_ids.get(&object.id) {
            return id.clone();
        }
        let native_annotation_source_id = matches!(
            object.kind(),
            crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
        )
        .then(|| {
            object
                .meta
                .pointer("/import/cdxml/sourceId")
                .and_then(Value::as_str)
                .filter(|id| id.parse::<u64>().is_ok())
                .map(ToString::to_string)
        })
        .flatten();
        let id = if let Some(source_id) = native_annotation_source_id {
            source_id
        } else if self
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

fn max_interchange_numeric_id(document: &ChemSemaDocument) -> u64 {
    fn visit(node: &crate::InterchangeObject) -> u64 {
        node.id
            .as_deref()
            .and_then(|id| id.parse::<u64>().ok())
            .unwrap_or(0)
            .max(node.children.iter().map(visit).max().unwrap_or(0))
    }

    document
        .interchange
        .get("cdxml")
        .map(|source| visit(&source.root))
        .unwrap_or(0)
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
