use super::*;
use crate::{
    ChemicalProperty, ChemicalPropertyCalculationState, ChemicalPropertyType,
    ChemicalPropertyValueOrigin, LinkEndpoint, LinkRelation,
};

pub(super) fn import_chemical_properties(
    root: &XmlNode,
    objects: &[SceneObject],
    resources: &BTreeMap<String, Resource>,
) -> (Vec<ChemicalProperty>, Vec<LinkRelation>) {
    let source_entities = source_entity_map(root, objects, resources);
    let mut properties = Vec::new();
    let mut links = Vec::new();
    for (index, node) in descendants(root)
        .into_iter()
        .filter(|node| node.is("chemicalproperty"))
        .enumerate()
    {
        let source_id = node.attr("id").map(ToString::to_string);
        let id = source_id
            .as_deref()
            .map(|id| format!("chemical_property_{id}"))
            .unwrap_or_else(|| format!("chemical_property_imported_{:03}", index + 1));
        let property_type = parse_property_type(node.attr("ChemicalPropertyType"));
        let is_active = parse_cdxml_bool(node.attr("ChemicalPropertyIsActive")).unwrap_or(false);
        let display_object_id = node
            .attr("ChemicalPropertyDisplayID")
            .and_then(|id| source_entities.get(id))
            .and_then(|ids| {
                ids.iter().find(|id| {
                    objects
                        .iter()
                        .flat_map(flatten_scene_object)
                        .any(|object| object.id == **id && object.object_type == "text")
                })
            })
            .cloned();
        let mut basis_entity_ids = Vec::new();
        let mut unresolved_basis_ids = Vec::new();
        for source_basis_id in node.attr("BasisObjects").unwrap_or("").split_whitespace() {
            if let Some(entity_ids) = source_entities.get(source_basis_id) {
                for entity_id in entity_ids {
                    if !basis_entity_ids.contains(entity_id) {
                        basis_entity_ids.push(entity_id.clone());
                    }
                }
            } else {
                unresolved_basis_ids.push(source_basis_id.to_string());
            }
        }
        let calculation_state = if !is_active {
            ChemicalPropertyCalculationState::Static
        } else if property_type.is_chemical_name() {
            ChemicalPropertyCalculationState::Stale
        } else {
            ChemicalPropertyCalculationState::Unsupported
        };
        let property = ChemicalProperty {
            id: id.clone(),
            source_id,
            property_type,
            basis_entity_ids: basis_entity_ids.clone(),
            unresolved_basis_ids,
            display_object_id: display_object_id.clone(),
            is_active,
            value_origin: ChemicalPropertyValueOrigin::Imported,
            calculation_state,
            last_calculated_value: None,
        };
        if let Some(display_id) = display_object_id {
            let mut endpoints = basis_entity_ids
                .into_iter()
                .map(|entity_id| LinkEndpoint {
                    entity_id,
                    role: "basis".to_string(),
                })
                .collect::<Vec<_>>();
            endpoints.push(LinkEndpoint {
                entity_id: display_id,
                role: "display".to_string(),
            });
            links.push(LinkRelation {
                id: format!("link_{id}"),
                kind: "chemical-property-display".to_string(),
                endpoints,
                data: json!({ "chemicalPropertyId": id, "inference": "declared" }),
            });
        }
        properties.push(property);
    }
    (properties, links)
}

fn parse_property_type(value: Option<&str>) -> ChemicalPropertyType {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return ChemicalPropertyType::undefined();
    };
    if value.eq_ignore_ascii_case("Unspecified") || value == "0" {
        return ChemicalPropertyType::unspecified();
    }
    if value.eq_ignore_ascii_case("ChemicalName") || value == "1" {
        return ChemicalPropertyType::chemical_name();
    }
    if let Ok(code) = value.parse::<u32>() {
        return ChemicalPropertyType {
            code: Some(code),
            name: None,
        };
    }
    ChemicalPropertyType {
        code: None,
        name: Some(value.to_string()),
    }
}

pub(super) fn source_entity_map(
    root: &XmlNode,
    objects: &[SceneObject],
    resources: &BTreeMap<String, Resource>,
) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::<String, Vec<String>>::new();
    for object in objects.iter().flat_map(flatten_scene_object) {
        for source_id in [
            object.meta.get("textId").and_then(Value::as_str),
            object.meta.get("fragmentId").and_then(Value::as_str),
            object.meta.get("graphicId").and_then(Value::as_str),
            object.meta.get("spectrumId").and_then(Value::as_str),
            object
                .meta
                .pointer("/import/cdxml/sourceId")
                .and_then(Value::as_str),
        ]
        .into_iter()
        .flatten()
        {
            push_unique(&mut map, source_id, &object.id);
        }
        if let Some(resource_ref) = object.payload.resource_ref.as_deref() {
            if let Some(ResourceData::Fragment(fragment)) =
                resources.get(resource_ref).map(|resource| &resource.data)
            {
                for node in &fragment.nodes {
                    push_unique(&mut map, &node.id, &node.id);
                }
                for bond in &fragment.bonds {
                    let source_id = bond
                        .meta
                        .pointer("/import/cdxml/sourceId")
                        .and_then(Value::as_str)
                        .unwrap_or(&bond.id);
                    push_unique(&mut map, source_id, &bond.id);
                }
            }
        }
    }
    map_superseded_graphics(root, &mut map);
    map_unmodeled_containers(root, &mut map);
    map
}

fn map_superseded_graphics(root: &XmlNode, map: &mut BTreeMap<String, Vec<String>>) {
    for node in descendants(root) {
        let (Some(source_id), Some(replacement_id)) = (node.attr("id"), node.attr("SupersededBy"))
        else {
            continue;
        };
        let Some(replacements) = map.get(replacement_id).cloned() else {
            continue;
        };
        for replacement in replacements {
            push_unique(map, source_id, &replacement);
        }
    }
}

fn map_unmodeled_containers(
    node: &XmlNode,
    map: &mut BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut descendants = Vec::new();
    for child in &node.children {
        for entity_id in map_unmodeled_containers(child, map) {
            if !descendants.contains(&entity_id) {
                descendants.push(entity_id);
            }
        }
    }
    if let Some(source_id) = node.attr("id").filter(|id| is_assigned_source_id(id)) {
        if let Some(existing) = map.get(source_id) {
            return existing.clone();
        }
        if !descendants.is_empty() {
            map.insert(source_id.to_string(), descendants.clone());
            return descendants;
        }
    }
    descendants
}

fn push_unique(map: &mut BTreeMap<String, Vec<String>>, source_id: &str, entity_id: &str) {
    if !is_assigned_source_id(source_id) {
        return;
    }
    let entries = map.entry(source_id.to_string()).or_default();
    if !entries.iter().any(|id| id == entity_id) {
        entries.push(entity_id.to_string());
    }
}

fn is_assigned_source_id(source_id: &str) -> bool {
    !matches!(source_id.trim(), "" | "0")
}

pub(super) fn flatten_scene_object(object: &SceneObject) -> Vec<&SceneObject> {
    let mut objects = vec![object];
    for child in &object.children {
        objects.extend(flatten_scene_object(child));
    }
    objects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_parser_distinguishes_absent_unspecified_known_and_custom_values() {
        assert_eq!(parse_property_type(None), ChemicalPropertyType::undefined());
        assert_eq!(
            parse_property_type(Some("Unspecified")),
            ChemicalPropertyType::unspecified()
        );
        assert_eq!(
            parse_property_type(Some("1")),
            ChemicalPropertyType::chemical_name()
        );
        assert_eq!(
            parse_property_type(Some("32769")),
            ChemicalPropertyType {
                code: Some(32769),
                name: None
            }
        );
        assert_eq!(
            parse_property_type(Some("org.example.LogP")),
            ChemicalPropertyType {
                code: None,
                name: Some("org.example.LogP".to_string())
            }
        );
    }
}
