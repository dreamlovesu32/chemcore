use super::*;
use crate::{
    ConstraintData, ConstraintType, GeometryData, GeometryFeature, LinkEndpoint, LinkPolicy,
    LinkRelation,
};

pub(super) fn append_geometry_constraint_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    resources: &BTreeMap<String, Resource>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) {
    let source_entities =
        super::import_chemical_properties::source_entity_map(root, objects, resources);
    let mut imported_source_ids = BTreeMap::<String, String>::new();
    for (index, node) in descendants(root)
        .into_iter()
        .filter(|node| {
            (node.is("geometry") && node.attr("GeometricFeature").is_some())
                || node.is("constraint")
        })
        .enumerate()
    {
        let source_id = node.attr("id").map(ToString::to_string);
        let id = source_id
            .as_deref()
            .map(|id| format!("{}_{}", node.name, id))
            .unwrap_or_else(|| format!("{}_imported_{:03}", node.name, index + 1));
        if let Some(source_id) = &source_id {
            imported_source_ids.insert(source_id.clone(), id.clone());
        }
        let mut basis_entity_ids = Vec::new();
        let mut unresolved_basis_ids = Vec::new();
        for basis_id in node.attr("BasisObjects").unwrap_or("").split_whitespace() {
            if let Some(entity_ids) = source_entities.get(basis_id) {
                for entity_id in entity_ids {
                    if !basis_entity_ids.contains(entity_id) {
                        basis_entity_ids.push(entity_id.clone());
                    }
                }
            } else {
                unresolved_basis_ids.push(basis_id.to_string());
            }
        }
        let bbox = parse_bbox(node.attr("BoundingBox")).map(|bbox| {
            [
                round2(bbox[0]),
                round2(bbox[1]),
                round2(bbox[2] - bbox[0]),
                round2(bbox[3] - bbox[1]),
            ]
        });
        let geometry = if node.is("geometry") {
            node.attr("GeometricFeature")
                .and_then(GeometryFeature::from_cdxml)
                .map(|feature| GeometryData {
                    feature,
                    basis_entity_ids: basis_entity_ids.clone(),
                    unresolved_basis_ids: unresolved_basis_ids.clone(),
                    relation_value: parse_f64(node.attr("RelationValue")),
                    point_is_directed: parse_cdxml_bool(node.attr("PointIsDirected"))
                        .unwrap_or(false),
                })
        } else {
            None
        };
        let constraint = if node.is("constraint") {
            node.attr("ConstraintType")
                .and_then(ConstraintType::from_cdxml)
                .map(|constraint_type| ConstraintData {
                    constraint_type,
                    basis_entity_ids,
                    unresolved_basis_ids,
                    minimum: parse_f64(node.attr("ConstraintMin")),
                    maximum: parse_f64(node.attr("ConstraintMax")),
                    ignore_unconnected_atoms: parse_cdxml_bool(node.attr("IgnoreUnconnectedAtoms"))
                        .unwrap_or(false),
                    dihedral_is_chiral: parse_cdxml_bool(node.attr("DihedralIsChiral"))
                        .unwrap_or(false),
                    point_is_directed: parse_cdxml_bool(node.attr("PointIsDirected"))
                        .unwrap_or(false),
                    display: annotation_display(node, defaults, colors, fonts),
                })
        } else {
            None
        };
        if geometry.is_none() && constraint.is_none() {
            continue;
        }
        let style_id = format!("style_annotation_{:03}", index + 1);
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "annotation",
                "stroke": colors.resolve(node.attr("color")),
                "strokeWidth": parse_f64(node.attr("LineWidth")).unwrap_or(defaults.line_width),
                "hashSpacing": parse_f64(node.attr("HashSpacing")).unwrap_or(defaults.hash_spacing),
            }),
        );
        objects.push(SceneObject {
            id,
            object_type: node.name.clone(),
            name: node.attr("Name").unwrap_or("").to_string(),
            visible: parse_cdxml_bool(node.attr("Visible")).unwrap_or(true),
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(0),
            transform: Transform::identity(),
            style_ref: Some(style_id),
            link_policy: LinkPolicy::Linked,
            meta: json!({
                "import": {
                    "cdxml": {
                        "sourceId": source_id,
                        "kind": node.name,
                    }
                }
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox,
                spectrum: None,
                geometry,
                constraint,
                table: None,
                stoichiometry_grid: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        });
    }
    if imported_source_ids.is_empty() {
        return;
    }
    for object in objects.iter_mut().filter(|object| {
        matches!(
            object.kind(),
            crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
        )
    }) {
        let (basis, unresolved) = if let Some(geometry) = object.payload.geometry.as_mut() {
            (
                &mut geometry.basis_entity_ids,
                &mut geometry.unresolved_basis_ids,
            )
        } else if let Some(constraint) = object.payload.constraint.as_mut() {
            (
                &mut constraint.basis_entity_ids,
                &mut constraint.unresolved_basis_ids,
            )
        } else {
            continue;
        };
        let mut still_unresolved = Vec::new();
        for source_id in unresolved.drain(..) {
            if let Some(entity_id) = imported_source_ids.get(&source_id) {
                basis.push(entity_id.clone());
            } else {
                still_unresolved.push(source_id);
            }
        }
        *unresolved = still_unresolved;
    }
}

fn annotation_display(
    node: &XmlNode,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> crate::AnnotationDisplay {
    let Some(tag) = node.direct_children("objecttag").next() else {
        return crate::AnnotationDisplay::default();
    };
    let Some(text) = tag.direct_children("t").next() else {
        return crate::AnnotationDisplay::default();
    };
    let run = text.direct_children("s").next();
    let default_font_id = defaults.caption_font.to_string();
    let font_id = run
        .and_then(|run| run.attr("font"))
        .or_else(|| text.attr("font"))
        .unwrap_or(default_font_id.as_str());
    let face = run
        .and_then(|run| parse_u32(run.attr("face")))
        .or_else(|| parse_u32(text.attr("face")))
        .unwrap_or(0);
    let font_size = run
        .and_then(|run| parse_f64(run.attr("size")))
        .or_else(|| parse_f64(text.attr("size")))
        .or(Some(defaults.caption_size));
    let color_id = run
        .and_then(|run| run.attr("color"))
        .or_else(|| text.attr("color"))
        .unwrap_or("0");
    let positioning_type = tag
        .attr("PositioningType")
        .and_then(crate::AnnotationPositioningType::from_cdxml)
        .unwrap_or_default();
    let source_text = text.full_text();
    crate::AnnotationDisplay {
        auto_value: true,
        text_override: (!source_text.is_empty()).then_some(source_text),
        position: parse_xy(text.attr("p")),
        positioning_type,
        positioning_angle: parse_f64(tag.attr("PositioningAngle")),
        positioning_offset: parse_xy(tag.attr("PositioningOffset")),
        font_family: Some(
            fonts
                .get(font_id)
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        font_size,
        fill: Some(colors.resolve(Some(color_id))),
        font_weight: if face & 1 != 0 { 700 } else { 400 },
        italic: face & 2 != 0,
        underline: face & 4 != 0,
        indicator_visible: !tag
            .attr("Visible")
            .is_some_and(|value| value.eq_ignore_ascii_case("no")),
    }
}

pub(super) fn normalize_imported_annotation_displays(document: &mut ChemSemaDocument) {
    let ids = document
        .scene_objects()
        .into_iter()
        .filter(|object| object.kind() == crate::SceneObjectKind::Constraint)
        .map(|object| object.id.clone())
        .collect::<Vec<_>>();
    for id in ids {
        let Some(object) = document.find_scene_object_mut(&id) else {
            continue;
        };
        let Some(constraint) = object.payload.constraint.as_mut() else {
            continue;
        };
        let source_text = constraint.display.text_override.take();
        let expected = crate::geometry_constraints::constraint_value_text(constraint);
        if source_text.as_deref() == expected.as_deref() {
            constraint.display.auto_value = true;
        } else {
            constraint.display.auto_value = false;
            constraint.display.text_override = source_text;
        }
    }
}

pub(super) fn annotation_basis_links(objects: &[SceneObject]) -> Vec<LinkRelation> {
    objects
        .iter()
        .flat_map(annotation_basis_links_for_object)
        .collect()
}

fn annotation_basis_links_for_object(object: &SceneObject) -> Vec<LinkRelation> {
    let mut links = Vec::new();
    let basis = object
        .payload
        .geometry
        .as_ref()
        .map(|geometry| geometry.basis_entity_ids.as_slice())
        .or_else(|| {
            object
                .payload
                .constraint
                .as_ref()
                .map(|constraint| constraint.basis_entity_ids.as_slice())
        });
    if let Some(basis) = basis {
        links.push(LinkRelation {
            id: format!("link_annotation_basis_{}", object.id),
            kind: "annotation-basis".to_string(),
            endpoints: std::iter::once(LinkEndpoint {
                entity_id: object.id.clone(),
                role: "annotation".to_string(),
            })
            .chain(basis.iter().map(|basis_id| LinkEndpoint {
                entity_id: basis_id.clone(),
                role: "basis".to_string(),
            }))
            .collect(),
            data: Value::Null,
        });
    }
    for child in &object.children {
        links.extend(annotation_basis_links_for_object(child));
    }
    links
}
