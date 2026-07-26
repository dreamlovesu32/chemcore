use super::*;

pub(super) fn merge_interchange_tree(
    generated: &mut crate::cdxml::xml::XmlNode,
    source: &crate::InterchangeObject,
) {
    for property in source.properties.values() {
        if (source.name == "chemicalproperty"
            && matches!(
                property.name.as_str(),
                "ChemicalPropertyType"
                    | "ChemicalPropertyDisplayID"
                    | "ChemicalPropertyIsActive"
                    | "BasisObjects"
            ))
            || (source.name == "geometry"
                && matches!(
                    property.name.as_str(),
                    "GeometricFeature"
                        | "BasisObjects"
                        | "RelationValue"
                        | "PointIsDirected"
                        | "BoundingBox"
                        | "Name"
                        | "Visible"
                        | "Z"
                ))
            || (source.name == "constraint"
                && matches!(
                    property.name.as_str(),
                    "ConstraintType"
                        | "BasisObjects"
                        | "ConstraintMin"
                        | "ConstraintMax"
                        | "IgnoreUnconnectedAtoms"
                        | "DihedralIsChiral"
                        | "PointIsDirected"
                        | "BoundingBox"
                        | "Name"
                        | "Visible"
                        | "Z"
                ))
            || (source.name == "stoichiometrygrid"
                && matches!(
                    property.name.as_str(),
                    "BoundingBox"
                        | "Visible"
                        | "LineWidth"
                        | "BoldWidth"
                        | "MarginWidth"
                        | "color"
                        | "LabelFont"
                        | "LabelSize"
                        | "LabelFace"
                        | "Z"
                ))
            || (source.name == "sgcomponent"
                && matches!(
                    property.name.as_str(),
                    "ComponentIsHeader"
                        | "ComponentIsReactant"
                        | "ComponentReferenceID"
                        | "Visible"
                        | "Width"
                ))
            || (source.name == "sgdatum"
                && matches!(
                    property.name.as_str(),
                    "SGPropertyType"
                        | "SGDataType"
                        | "SGDataValue"
                        | "IsEdited"
                        | "IsHidden"
                        | "IsReadOnly"
                        | "Visible"
                ))
        {
            continue;
        }
        generated
            .attrs
            .entry(property.name.clone())
            .or_insert_with(|| property.value.clone());
    }
    if generated.text.is_empty() && !source.text.is_empty() {
        generated.text = source.text.clone();
    }

    let mut remaining = std::mem::take(&mut generated.children);
    let mut ordered = Vec::with_capacity(remaining.len().max(source.children.len()));
    for source_child in &source.children {
        let exact = remaining
            .iter()
            .position(|child| interchange_xml_exact_match(source_child, child));
        let match_index = exact.or_else(|| {
            remaining
                .iter()
                .position(|child| source_child.name == child.name)
        });
        if let Some(index) = match_index {
            let mut child = remaining.remove(index);
            merge_interchange_tree(&mut child, source_child);
            ordered.push(child);
        } else if !is_regenerated_table(&source_child.name) {
            ordered.push(xml_from_interchange(source_child));
        }
    }
    ordered.append(&mut remaining);
    generated.children = ordered;
}

pub(super) fn retain_native_annotations(
    source: &mut crate::InterchangeObject,
    objects: &[crate::SceneObject],
) {
    let allowed_ids = objects
        .iter()
        .flat_map(annotation_objects)
        .filter_map(|object| {
            object
                .meta
                .pointer("/import/cdxml/sourceId")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<std::collections::BTreeSet<_>>();
    retain_native_annotations_recursive(source, &allowed_ids);
}

fn annotation_objects(object: &crate::SceneObject) -> Vec<&crate::SceneObject> {
    let mut objects = Vec::new();
    if matches!(
        object.kind(),
        crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
    ) {
        objects.push(object);
    }
    for child in &object.children {
        objects.extend(annotation_objects(child));
    }
    objects
}

fn retain_native_annotations_recursive(
    source: &mut crate::InterchangeObject,
    allowed_ids: &std::collections::BTreeSet<&str>,
) {
    source.children.retain(|child| {
        let native_geometry =
            child.name == "geometry" && child.properties.contains_key("GeometricFeature");
        if !native_geometry && child.name != "constraint" {
            return true;
        }
        child
            .id
            .as_deref()
            .is_some_and(|id| allowed_ids.contains(id))
    });
    for child in &mut source.children {
        retain_native_annotations_recursive(child, allowed_ids);
    }
}

pub(super) fn retain_native_chemical_properties(
    source: &mut crate::InterchangeObject,
    properties: &[crate::ChemicalProperty],
) {
    let allowed_ids = properties
        .iter()
        .filter_map(|property| property.source_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    let mut remaining_without_id = properties
        .iter()
        .filter(|property| property.source_id.is_none())
        .count();
    retain_native_chemical_properties_recursive(source, &allowed_ids, &mut remaining_without_id);
}

fn retain_native_chemical_properties_recursive(
    source: &mut crate::InterchangeObject,
    allowed_ids: &std::collections::BTreeSet<&str>,
    remaining_without_id: &mut usize,
) {
    source.children.retain(|child| {
        if child.name != "chemicalproperty" {
            return true;
        }
        if let Some(id) = child.id.as_deref() {
            return allowed_ids.contains(id);
        }
        if *remaining_without_id == 0 {
            return false;
        }
        *remaining_without_id -= 1;
        true
    });
    for child in &mut source.children {
        retain_native_chemical_properties_recursive(child, allowed_ids, remaining_without_id);
    }
}

pub(super) fn interchange_xml_exact_match(
    source: &crate::InterchangeObject,
    generated: &crate::cdxml::xml::XmlNode,
) -> bool {
    source.name == generated.name
        && match (&source.id, generated.attr("id")) {
            (Some(source_id), Some(generated_id)) => source_id == generated_id,
            (None, None) => true,
            _ => false,
        }
}

pub(super) fn is_regenerated_table(name: &str) -> bool {
    matches!(name, "fonttable" | "colortable")
}

pub(super) fn xml_from_interchange(
    source: &crate::InterchangeObject,
) -> crate::cdxml::xml::XmlNode {
    crate::cdxml::xml::XmlNode {
        name: source.name.clone(),
        attrs: source
            .properties
            .values()
            .map(|property| (property.name.clone(), property.value.clone()))
            .collect(),
        text: source.text.clone(),
        children: source.children.iter().map(xml_from_interchange).collect(),
    }
}

pub(super) fn serialize_cdxml_tree(root: &crate::cdxml::xml::XmlNode) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd\" >\n",
    );
    write_xml_node(root, &mut out, 0);
    out.push('\n');
    out
}

pub(super) fn write_xml_node(node: &crate::cdxml::xml::XmlNode, out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
    out.push('<');
    out.push_str(&node.name);
    for (name, value) in &node.attrs {
        write!(out, " {}=\"{}\"", name, xml_escape_attr(value)).expect("write XML attribute");
    }
    if node.children.is_empty() && node.text.is_empty() {
        out.push_str(" />");
        return;
    }
    out.push('>');
    if !node.text.is_empty() {
        out.push_str(&xml_escape_text(&node.text));
    }
    if !node.children.is_empty() {
        out.push('\n');
        for child in &node.children {
            write_xml_node(child, out, indent + 2);
            out.push('\n');
        }
        for _ in 0..indent {
            out.push(' ');
        }
    }
    write!(out, "</{}>", node.name).expect("write XML end tag");
}
