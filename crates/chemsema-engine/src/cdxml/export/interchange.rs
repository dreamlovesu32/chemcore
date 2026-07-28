use super::*;

pub(super) fn remove_regenerated_scene_objects(
    source: &mut crate::InterchangeObject,
    document: &crate::ChemSemaDocument,
) {
    let mut regenerated = std::collections::BTreeSet::new();
    let regenerated_fragments = document
        .objects
        .iter()
        .flat_map(scene_objects_recursive)
        .filter(|object| object.object_type == "molecule")
        .filter_map(|object| {
            object
                .meta
                .pointer("/import/cdxml/fragmentId")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut regenerated_idless_curves = std::collections::BTreeSet::new();
    for object in &document.objects {
        collect_regenerated_scene_object_ids(
            object,
            &mut regenerated,
            &mut regenerated_idless_curves,
        );
    }
    for area_id in document
        .resources
        .values()
        .filter_map(|resource| resource.data.as_fragment())
        .flat_map(|fragment| fragment.colored_areas.iter().map(|area| area.id.clone()))
    {
        regenerated.insert(("ColoredMolecularArea", area_id.clone()));
        regenerated.insert(("coloredmoleculararea", area_id));
    }
    remove_regenerated_scene_objects_recursive(
        source,
        &regenerated,
        &regenerated_fragments,
        &regenerated_idless_curves,
    );
}

fn scene_objects_recursive(object: &crate::SceneObject) -> Vec<&crate::SceneObject> {
    let mut objects = vec![object];
    for child in &object.children {
        objects.extend(scene_objects_recursive(child));
    }
    objects
}

fn collect_regenerated_scene_object_ids(
    object: &crate::SceneObject,
    out: &mut std::collections::BTreeSet<(&'static str, String)>,
    idless_curves: &mut std::collections::BTreeSet<String>,
) {
    for (meta_key, source_tag) in [
        ("curveId", "curve"),
        ("graphicId", "graphic"),
        ("bioShapeId", "bioshape"),
        ("textId", "t"),
    ] {
        if let Some(id) = object
            .meta
            .get(meta_key)
            .and_then(serde_json::Value::as_str)
        {
            let is_arrow = meta_key == "graphicId"
                && object
                    .meta
                    .pointer("/import/cdxml/kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("arrow");
            if is_arrow {
                // CDX/CDXML uses both the legacy Graphic+ArrowType form and
                // the modern Arrow object for the same semantic object. A
                // native arrow replaces either representation; retaining the
                // legacy Graphic while appending the modern Arrow would add
                // one coincident arrow on every save.
                out.insert(("graphic", id.to_string()));
                out.insert(("arrow", id.to_string()));
            } else {
                out.insert((source_tag, id.to_string()));
            }
        }
    }
    if let Some(ids) = object
        .meta
        .get("graphicIds")
        .and_then(serde_json::Value::as_array)
    {
        for id in ids.iter().filter_map(serde_json::Value::as_str) {
            out.insert(("graphic", id.to_string()));
        }
    }
    if object
        .meta
        .get("curveId")
        .is_some_and(serde_json::Value::is_null)
    {
        if let Some(fingerprint) = object
            .meta
            .get("curveFingerprint")
            .and_then(serde_json::Value::as_str)
        {
            idless_curves.insert(fingerprint.to_string());
        }
    }
    for child in &object.children {
        collect_regenerated_scene_object_ids(child, out, idless_curves);
    }
}

fn remove_regenerated_scene_objects_recursive(
    source: &mut crate::InterchangeObject,
    regenerated: &std::collections::BTreeSet<(&'static str, String)>,
    regenerated_fragments: &std::collections::BTreeSet<String>,
    regenerated_idless_curves: &std::collections::BTreeSet<String>,
) {
    let is_embedded_fragment_owner = source.name == "n";
    source.children.retain(|child| {
        let represented_by_id = child.id.as_ref().is_some_and(|id| {
            regenerated
                .iter()
                .any(|(tag, regenerated_id)| *tag == child.name && regenerated_id == id)
        });
        let represented_idless_curve = child.name == "curve"
            && child.id.is_none()
            && child
                .properties
                .get("CurvePoints")
                .is_some_and(|property| regenerated_idless_curves.contains(&property.value));
        let regenerated_embedded_fragment = is_embedded_fragment_owner
            && child.name == "fragment"
            && child
                .id
                .as_ref()
                .is_some_and(|id| regenerated_fragments.contains(id));
        !represented_by_id && !represented_idless_curve && !regenerated_embedded_fragment
    });
    for child in &mut source.children {
        remove_regenerated_scene_objects_recursive(
            child,
            regenerated,
            regenerated_fragments,
            regenerated_idless_curves,
        );
    }
}

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
        let renumbered_fragment = exact
            .is_none()
            .then(|| unique_renumbered_fragment_match(source_child, &remaining))
            .flatten();
        let match_index = exact.or(renumbered_fragment).or_else(|| {
            if source_child.id.is_some()
                && matches!(source_child.name.as_str(), "fragment" | "group")
            {
                // Fragment and group IDs define ownership boundaries. If an
                // identified source boundary has no exact generated peer, it
                // must never be grafted onto the next nearby boundary.
                //
                // Nodes and bonds are different: CDXML permits their IDs to
                // overlap, while our regenerated tree uses globally unique
                // IDs. They can therefore be renumbered even inside the same
                // matched fragment and must retain ordered matching there.
                None
            } else if matches!(
                source_child.name.as_str(),
                "page" | "fragment" | "n" | "b" | "t" | "s" | "group"
            ) {
                remaining
                    .iter()
                    .position(|child| source_child.name == child.name)
            } else if source_child.id.is_none() {
                remaining
                    .iter()
                    .position(|child| source_child.name == child.name && child.attr("id").is_none())
            } else if matches!(
                source_child.name.as_str(),
                "graphic" | "curve" | "arrow" | "bioshape"
            ) {
                None
            } else {
                let mut matches = remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, child)| source_child.name == child.name);
                let first = matches.next().map(|(index, _)| index);
                if matches.next().is_none() {
                    first
                } else {
                    None
                }
            }
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

fn unique_renumbered_fragment_match(
    source: &crate::InterchangeObject,
    generated: &[crate::cdxml::xml::XmlNode],
) -> Option<usize> {
    if source.name != "fragment" || source.id.is_none() {
        return None;
    }
    let source_shape = direct_fragment_graph_shape_from_interchange(source)?;
    let source_centroid = direct_fragment_node_centroid_from_interchange(source);
    let mut matches = generated.iter().enumerate().filter(|(_, candidate)| {
        candidate.name == "fragment"
            && direct_fragment_graph_shape_from_xml(candidate) == Some(source_shape)
            && fragment_centroids_match(
                source_centroid,
                direct_fragment_node_centroid_from_xml(candidate),
            )
    });
    let first = matches.next().map(|(index, _)| index);
    if matches.next().is_none() {
        first
    } else {
        None
    }
}

fn fragment_centroids_match(source: Option<(f64, f64)>, generated: Option<(f64, f64)>) -> bool {
    match (source, generated) {
        (Some((source_x, source_y)), Some((generated_x, generated_y))) => {
            let dx = source_x - generated_x;
            let dy = source_y - generated_y;
            dx * dx + dy * dy <= 0.05_f64.powi(2)
        }
        (None, None) => true,
        _ => false,
    }
}

fn direct_fragment_graph_shape_from_interchange(
    fragment: &crate::InterchangeObject,
) -> Option<(usize, usize)> {
    let shape = (
        fragment
            .children
            .iter()
            .filter(|child| child.name == "n")
            .count(),
        fragment
            .children
            .iter()
            .filter(|child| child.name == "b")
            .count(),
    );
    (shape.0 + shape.1 > 0).then_some(shape)
}

fn direct_fragment_node_centroid_from_interchange(
    fragment: &crate::InterchangeObject,
) -> Option<(f64, f64)> {
    point_centroid(fragment.children.iter().filter_map(|child| {
        (child.name == "n")
            .then(|| child.properties.get("p"))
            .flatten()
            .and_then(|property| parse_cdxml_point(&property.value))
    }))
}

fn direct_fragment_node_centroid_from_xml(
    fragment: &crate::cdxml::xml::XmlNode,
) -> Option<(f64, f64)> {
    point_centroid(
        fragment
            .children
            .iter()
            .filter_map(|child| (child.name == "n").then(|| child.attr("p")).flatten())
            .filter_map(parse_cdxml_point),
    )
}

fn parse_cdxml_point(value: &str) -> Option<(f64, f64)> {
    let mut values = value
        .split_whitespace()
        .filter_map(|part| part.parse().ok());
    Some((values.next()?, values.next()?))
}

fn point_centroid(points: impl Iterator<Item = (f64, f64)>) -> Option<(f64, f64)> {
    let mut count = 0_u32;
    let mut x = 0.0;
    let mut y = 0.0;
    for (point_x, point_y) in points {
        count += 1;
        x += point_x;
        y += point_y;
    }
    (count > 0).then(|| (x / f64::from(count), y / f64::from(count)))
}

fn direct_fragment_graph_shape_from_xml(
    fragment: &crate::cdxml::xml::XmlNode,
) -> Option<(usize, usize)> {
    let shape = (
        fragment
            .children
            .iter()
            .filter(|child| child.name == "n")
            .count(),
        fragment
            .children
            .iter()
            .filter(|child| child.name == "b")
            .count(),
    );
    (shape.0 + shape.1 > 0).then_some(shape)
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

pub(super) fn retain_native_plasmid_maps(
    source: &mut crate::InterchangeObject,
    objects: &[crate::SceneObject],
) {
    let has_native_map = objects.iter().any(scene_object_contains_native_plasmid);
    if has_native_map {
        remove_plasmid_maps_recursive(source);
    }
}

fn scene_object_contains_native_plasmid(object: &crate::SceneObject) -> bool {
    object.payload.plasmid_map.is_some()
        || object
            .children
            .iter()
            .any(scene_object_contains_native_plasmid)
}

fn remove_plasmid_maps_recursive(source: &mut crate::InterchangeObject) {
    source.children.retain(|child| child.name != "plasmidmap");
    for child in &mut source.children {
        remove_plasmid_maps_recursive(child);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renumbered_fragment_matches_only_by_unique_direct_graph_shape() {
        let source_xml = crate::cdxml::parse_xml_tree(
            r#"<page><fragment id="3" SourceTag="kept">
                <n id="2" p="10 10"/><n id="4" p="20 10"/>
                <n id="6" p="20 20"/><n id="8" p="30 10"/>
                <b id="5" B="2" E="4"/><b id="7" B="4" E="6"/><b id="9" B="4" E="8"/>
            </fragment></page>"#,
        )
        .expect("source XML parses");
        let source = crate::cdxml::interchange_object_from_xml(&source_xml);
        let mut generated = crate::cdxml::parse_xml_tree(
            r#"<page><fragment id="27">
                <n id="28" p="10.01 10"/><n id="29" p="20.01 10"/>
                <n id="30" p="20.01 20"/><n id="31" p="30.01 10"/>
                <b id="32" B="28" E="29"/><b id="33" B="29" E="30"/><b id="34" B="29" E="31"/>
            </fragment><fragment id="35">
                <n id="36" p="110 110"/><n id="37" p="120 110"/>
                <n id="38" p="120 120"/><n id="39" p="130 110"/>
                <b id="40" B="36" E="37"/><b id="41" B="37" E="38"/><b id="42" B="37" E="39"/>
            </fragment></page>"#,
        )
        .expect("generated XML parses");

        merge_interchange_tree(&mut generated, &source);

        let fragments = generated
            .children
            .iter()
            .filter(|child| child.name == "fragment")
            .collect::<Vec<_>>();
        assert_eq!(fragments.len(), 2);
        assert_eq!(fragments[0].attr("id"), Some("27"));
        assert_eq!(fragments[0].attr("SourceTag"), Some("kept"));
        assert_eq!(
            fragments[0]
                .children
                .iter()
                .filter(|child| child.name == "n")
                .count(),
            4
        );
    }

    #[test]
    fn structurally_different_fragment_boundary_is_not_grafted() {
        let source_xml = crate::cdxml::parse_xml_tree(
            r#"<page><fragment id="20" SourceTag="wrapper"><n id="21"/></fragment></page>"#,
        )
        .expect("source XML parses");
        let source = crate::cdxml::interchange_object_from_xml(&source_xml);
        let mut generated = crate::cdxml::parse_xml_tree(
            r#"<page><fragment id="30">
                <n id="31"/><n id="32"/><b id="33" B="31" E="32"/>
            </fragment></page>"#,
        )
        .expect("generated XML parses");

        merge_interchange_tree(&mut generated, &source);

        assert_eq!(
            generated
                .children
                .iter()
                .filter(|child| child.name == "fragment")
                .count(),
            2
        );
        let regenerated = generated
            .children
            .iter()
            .find(|child| child.attr("id") == Some("30"))
            .expect("regenerated fragment remains separate");
        assert_eq!(regenerated.attr("SourceTag"), None);
    }
}
