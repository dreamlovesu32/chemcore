use super::*;

pub(super) fn import_logical_objects(
    root: &XmlNode,
    objects: &[SceneObject],
    resources: &BTreeMap<String, Resource>,
    _reaction_schemes: &[crate::ReactionSchemeData],
    colors: &CdxmlColorTable,
) -> crate::LogicalObjectData {
    let source_map = source_entity_map(root, objects, resources);
    let mut data = crate::LogicalObjectData::default();
    import_alternative_groups(root, &source_map, colors, &mut data);
    import_bracketed_groups(root, objects, &source_map, &mut data);
    import_sequences(root, &source_map, &mut data);
    import_attached_metadata(root, None, &source_map, &mut data);
    data
}

fn import_alternative_groups(
    root: &XmlNode,
    source_map: &BTreeMap<String, Vec<String>>,
    colors: &CdxmlColorTable,
    data: &mut crate::LogicalObjectData,
) {
    let alternative_source_ids = descendants(root)
        .into_iter()
        .filter(|node| node.is("altgroup"))
        .enumerate()
        .filter_map(|(index, node)| {
            Some((
                node.attr("id")?.to_string(),
                logical_id(node.attr("id"), "alternative_group", index + 1),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut index = 1;
    for node in descendants(root)
        .into_iter()
        .filter(|node| node.is("altgroup"))
    {
        let source_id = node.attr("id");
        let id = logical_id(source_id, "alternative_group", index);
        index += 1;
        let mut member_entity_ids = Vec::new();
        let mut unresolved_member_source_ids = Vec::new();
        for child in &node.children {
            let Some(child_id) = child.attr("id") else {
                continue;
            };
            extend_resolved_or_unresolved(
                child_id,
                source_map,
                &mut member_entity_ids,
                &mut unresolved_member_source_ids,
            );
        }
        let mut attachment_node_ids = Vec::new();
        if let Some(source_id) = source_id {
            for attachment in descendants(root).into_iter().filter(|candidate| {
                candidate.is("n") && candidate.attr("AltGroupID") == Some(source_id)
            }) {
                if let Some(attachment_id) = attachment.attr("id") {
                    extend_unique(
                        &mut attachment_node_ids,
                        source_map.get(attachment_id).into_iter().flatten().cloned(),
                    );
                }
            }
        }
        let (superseded_by_id, unresolved_superseded_by_source_id) =
            resolve_logical_or_entity_reference(
                node.attr("SupersededBy"),
                source_map,
                &alternative_source_ids,
            );
        data.alternative_groups.push(crate::AlternativeGroupData {
            id,
            member_entity_ids,
            unresolved_member_source_ids,
            attachment_node_ids,
            valence: node
                .attr("Valence")
                .and_then(|value| value.parse::<i16>().ok()),
            position: parse_point(node.attr("p")),
            bounding_box: parse_bbox(node.attr("BoundingBox")),
            text_frame: parse_bbox(node.attr("TextFrame")),
            group_frame: parse_bbox(node.attr("GroupFrame")),
            opacity: parse_f64(node.attr("alpha")),
            color: node.attr("color").map(|color| colors.resolve(Some(color))),
            z_index: node.attr("Z").and_then(|value| value.parse::<i16>().ok()),
            visible: parse_bool(node.attr("Visible"), true),
            ignore_warnings: parse_bool(node.attr("IgnoreWarnings"), false),
            warning: node.attr("Warning").map(ToString::to_string),
            superseded_by_id,
            unresolved_superseded_by_source_id,
            binding_origin: crate::LogicalBindingOrigin::Imported,
        });
    }
}

fn import_bracketed_groups(
    root: &XmlNode,
    objects: &[SceneObject],
    source_map: &BTreeMap<String, Vec<String>>,
    data: &mut crate::LogicalObjectData,
) {
    let bracket_group_source_ids = descendants(root)
        .into_iter()
        .filter(|node| node.is("bracketedgroup"))
        .enumerate()
        .filter_map(|(index, node)| {
            Some((
                node.attr("id")?.to_string(),
                logical_id(node.attr("id"), "bracketed_group", index + 1),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut group_index = 1;
    let mut attachment_index = 1;
    let mut crossing_index = 1;
    for node in descendants(root)
        .into_iter()
        .filter(|node| node.is("bracketedgroup"))
    {
        let mut bracket_object_ids = Vec::new();
        let mut unresolved_bracket_source_ids = Vec::new();
        let mut attachments = Vec::new();
        for attachment in node.direct_children("bracketattachment") {
            let graphic_source_id = attachment.attr("GraphicID");
            let exact_graphic_id = graphic_source_id.and_then(|source_id| {
                objects
                    .iter()
                    .find_map(|object| find_scene_by_graphic_id(object, source_id))
                    .map(|object| object.id.clone())
            });
            let (bracket_object_id, unresolved_bracket_source_id) =
                if let Some(exact_graphic_id) = exact_graphic_id {
                    (Some(exact_graphic_id), None)
                } else {
                    resolved_reference(graphic_source_id, source_map)
                };
            if let Some(id) = &bracket_object_id {
                push_unique(&mut bracket_object_ids, id.clone());
            }
            if let Some(id) = &unresolved_bracket_source_id {
                push_unique(&mut unresolved_bracket_source_ids, id.clone());
            }
            let mut crossing_bonds = Vec::new();
            for crossing in attachment.direct_children("crossingbond") {
                let (bond_id, unresolved_bond_source_id) =
                    resolved_reference(crossing.attr("BondID"), source_map);
                let (inner_atom_id, unresolved_inner_atom_source_id) =
                    resolved_reference(crossing.attr("InnerAtomID"), source_map);
                crossing_bonds.push(crate::CrossingBondData {
                    id: logical_id(crossing.attr("id"), "crossing_bond", crossing_index),
                    bond_id,
                    unresolved_bond_source_id,
                    inner_atom_id,
                    unresolved_inner_atom_source_id,
                });
                crossing_index += 1;
            }
            attachments.push(crate::BracketAttachmentData {
                id: logical_id(
                    attachment.attr("id"),
                    "bracket_attachment",
                    attachment_index,
                ),
                bracket_object_id,
                unresolved_bracket_source_id,
                crossing_bonds,
            });
            attachment_index += 1;
        }
        let mut bracketed_entity_ids = Vec::new();
        let mut unresolved_bracketed_source_ids = Vec::new();
        for source_id in split_ids(node.attr("BracketedObjectIDs")) {
            extend_resolved_or_unresolved(
                source_id,
                source_map,
                &mut bracketed_entity_ids,
                &mut unresolved_bracketed_source_ids,
            );
        }
        let nested_group_ids = node
            .direct_children("bracketedgroup")
            .filter_map(|child| child.attr("id"))
            .filter_map(|source_id| bracket_group_source_ids.get(source_id).cloned())
            .collect();
        data.bracketed_groups.push(crate::BracketedGroupData {
            id: logical_id(node.attr("id"), "bracketed_group", group_index),
            bracket_object_ids,
            unresolved_bracket_source_ids,
            bracketed_entity_ids,
            unresolved_bracketed_source_ids,
            nested_group_ids,
            attachments,
            usage: crate::BracketUsage::from_cdxml(node.attr("BracketUsage")),
            component_order: node
                .attr("ComponentOrder")
                .and_then(|value| value.parse::<i16>().ok()),
            polymer_repeat_pattern: crate::PolymerRepeatPattern::from_cdxml(
                node.attr("PolymerRepeatPattern"),
            ),
            polymer_flip_type: crate::PolymerFlipType::from_cdxml(node.attr("PolymerFlipType")),
            repeat_count: parse_f64(node.attr("RepeatCount")),
            sru_label: node.attr("SRULabel").map(ToString::to_string),
            binding_origin: crate::LogicalBindingOrigin::Imported,
        });
        group_index += 1;
    }
}

fn import_sequences(
    root: &XmlNode,
    source_map: &BTreeMap<String, Vec<String>>,
    data: &mut crate::LogicalObjectData,
) {
    let mut sequence_index = 1;
    let mut cross_reference_index = 1;
    for node in descendants(root) {
        if node.is("sequence") {
            data.sequences.push(crate::SequenceData {
                id: logical_id(node.attr("id"), "sequence", sequence_index),
                identifier: node
                    .attr("SequenceIdentifier")
                    .unwrap_or_default()
                    .to_string(),
                text_object_ids: child_text_object_ids(node, source_map),
                binding_origin: crate::LogicalBindingOrigin::Imported,
            });
            sequence_index += 1;
        } else if node.is("crossreference") {
            data.cross_references.push(crate::CrossReferenceData {
                id: logical_id(node.attr("id"), "cross_reference", cross_reference_index),
                identifier: node
                    .attr("CrossReferenceIdentifier")
                    .unwrap_or_default()
                    .to_string(),
                sequence_identifier: node
                    .attr("CrossReferenceSequence")
                    .unwrap_or_default()
                    .to_string(),
                container: node
                    .attr("CrossReferenceContainer")
                    .map(ToString::to_string),
                document: node.attr("CrossReferenceDocument").map(ToString::to_string),
                text_object_ids: child_text_object_ids(node, source_map),
                binding_origin: crate::LogicalBindingOrigin::Imported,
            });
            cross_reference_index += 1;
        }
    }
}

fn import_attached_metadata(
    node: &XmlNode,
    parent_source_id: Option<&str>,
    source_map: &BTreeMap<String, Vec<String>>,
    data: &mut crate::LogicalObjectData,
) {
    let mut local_object_tag_index = data.object_tags.len() + 1;
    let mut local_annotation_index = data.annotations.len() + 1;
    let mut local_registry_index = data.registry_numbers.len() + 1;
    let mut local_representation_index = data.representations.len() + 1;
    for child in &node.children {
        match child.name.as_str() {
            // A constraint owns its objecttag as the editable display for the
            // constraint value. It is imported by the constraint model and is
            // therefore not a second, independent ObjectTagData relation.
            "objecttag" if !node.is("constraint") => {
                let (owner_entity_id, unresolved_owner_source_id) =
                    owner_reference(parent_source_id, source_map);
                data.object_tags.push(crate::ObjectTagData {
                    id: logical_id(child.attr("id"), "object_tag", local_object_tag_index),
                    owner_entity_id,
                    unresolved_owner_source_id,
                    name: child.attr("Name").unwrap_or_default().to_string(),
                    display_name: child.attr("DisplayName").map(ToString::to_string),
                    tag_type: crate::ObjectTagType::from_cdxml(child.attr("TagType")),
                    value: child.attr("Value").map(ToString::to_string),
                    positioning_type: child
                        .attr("PositioningType")
                        .and_then(crate::AnnotationPositioningType::from_cdxml)
                        .unwrap_or_default(),
                    positioning_angle: parse_f64(child.attr("PositioningAngle")),
                    positioning_offset: parse_point(child.attr("PositioningOffset")),
                    persistent: parse_bool(child.attr("Persistent"), true),
                    tracking: parse_bool(child.attr("Tracking"), true),
                    visible: parse_bool(child.attr("Visible"), true),
                    display_object_ids: child_text_object_ids(child, source_map),
                    binding_origin: crate::LogicalBindingOrigin::Imported,
                });
                local_object_tag_index += 1;
            }
            "annotation" => {
                let (owner_entity_id, unresolved_owner_source_id) =
                    owner_reference(parent_source_id, source_map);
                data.annotations.push(crate::AnnotationData {
                    id: logical_id(child.attr("id"), "annotation", local_annotation_index),
                    owner_entity_id,
                    unresolved_owner_source_id,
                    keyword: child.attr("Keyword").map(ToString::to_string),
                    content: child.attr("Content").map(ToString::to_string),
                    binding_origin: crate::LogicalBindingOrigin::Imported,
                });
                local_annotation_index += 1;
            }
            "regnum" => {
                let (owner_entity_id, unresolved_owner_source_id) =
                    owner_reference(parent_source_id, source_map);
                data.registry_numbers.push(crate::RegistryNumberData {
                    id: logical_id(child.attr("id"), "registry_number", local_registry_index),
                    owner_entity_id,
                    unresolved_owner_source_id,
                    authority: child
                        .attr("RegistryAuthority")
                        .unwrap_or_default()
                        .to_string(),
                    number: child.attr("RegistryNumber").unwrap_or_default().to_string(),
                    binding_origin: crate::LogicalBindingOrigin::Imported,
                });
                local_registry_index += 1;
            }
            "represent" => {
                let (owner_entity_id, unresolved_owner_source_id) =
                    owner_reference(parent_source_id, source_map);
                let (target_entity_id, unresolved_target_source_id) =
                    resolved_reference(child.attr("object"), source_map);
                data.representations.push(crate::RepresentationData {
                    id: format!("representation_{local_representation_index}"),
                    owner_entity_id,
                    unresolved_owner_source_id,
                    target_entity_id,
                    unresolved_target_source_id,
                    attribute: child.attr("attribute").unwrap_or_default().to_string(),
                    binding_origin: crate::LogicalBindingOrigin::Imported,
                });
                local_representation_index += 1;
            }
            _ => {}
        }
        let next_parent = child.attr("id").or(parent_source_id);
        import_attached_metadata(child, next_parent, source_map, data);
    }
}

fn find_scene_by_graphic_id<'a>(
    object: &'a SceneObject,
    source_id: &str,
) -> Option<&'a SceneObject> {
    if object.meta.get("graphicId").and_then(Value::as_str) == Some(source_id) {
        return Some(object);
    }
    object
        .children
        .iter()
        .find_map(|child| find_scene_by_graphic_id(child, source_id))
}

fn child_text_object_ids(
    node: &XmlNode,
    source_map: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut ids = Vec::new();
    for child in node.direct_children("t") {
        if let Some(source_id) = child.attr("id") {
            extend_unique(
                &mut ids,
                source_map.get(source_id).into_iter().flatten().cloned(),
            );
        }
    }
    ids
}

fn owner_reference(
    source_id: Option<&str>,
    source_map: &BTreeMap<String, Vec<String>>,
) -> (Option<String>, Option<String>) {
    let Some(source_id) = source_id else {
        return (None, None);
    };
    match source_map.get(source_id).map(Vec::as_slice) {
        Some([entity_id]) => (Some(entity_id.clone()), None),
        _ => (None, Some(source_id.to_string())),
    }
}

fn resolved_reference(
    source_id: Option<&str>,
    source_map: &BTreeMap<String, Vec<String>>,
) -> (Option<String>, Option<String>) {
    let Some(source_id) = source_id else {
        return (None, None);
    };
    match source_map.get(source_id).map(Vec::as_slice) {
        Some([entity_id]) => (Some(entity_id.clone()), None),
        _ => (None, Some(source_id.to_string())),
    }
}

fn resolve_logical_or_entity_reference(
    source_id: Option<&str>,
    source_map: &BTreeMap<String, Vec<String>>,
    logical_source_ids: &BTreeMap<String, String>,
) -> (Option<String>, Option<String>) {
    let Some(source_id) = source_id else {
        return (None, None);
    };
    if let Some(logical_id) = logical_source_ids.get(source_id) {
        return (Some(logical_id.clone()), None);
    }
    match source_map.get(source_id).map(Vec::as_slice) {
        Some([entity_id]) => (Some(entity_id.clone()), None),
        _ => (None, Some(source_id.to_string())),
    }
}

fn extend_resolved_or_unresolved(
    source_id: &str,
    source_map: &BTreeMap<String, Vec<String>>,
    resolved: &mut Vec<String>,
    unresolved: &mut Vec<String>,
) {
    if let Some(entity_ids) = source_map.get(source_id) {
        extend_unique(resolved, entity_ids.iter().cloned());
    } else {
        push_unique(unresolved, source_id.to_string());
    }
}

fn extend_unique(out: &mut Vec<String>, values: impl IntoIterator<Item = String>) {
    for value in values {
        push_unique(out, value);
    }
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !out.contains(&value) {
        out.push(value);
    }
}

fn split_ids(value: Option<&str>) -> Vec<&str> {
    value.into_iter().flat_map(str::split_whitespace).collect()
}

fn logical_id(source_id: Option<&str>, prefix: &str, index: usize) -> String {
    source_id
        .filter(|id| !id.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{prefix}_{index}"))
}

fn parse_point(value: Option<&str>) -> Option<[f64; 2]> {
    let values = value?
        .split_whitespace()
        .filter_map(|value| value.parse::<f64>().ok())
        .collect::<Vec<_>>();
    (values.len() == 2).then_some([values[0], values[1]])
}

fn parse_bool(value: Option<&str>, default: bool) -> bool {
    match value {
        Some(value) if value.eq_ignore_ascii_case("yes") || value == "1" => true,
        Some(value) if value.eq_ignore_ascii_case("no") || value == "0" => false,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    const LOGICAL_OBJECTS_CDXML: &str = r#"
<CDXML BondLength="30" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/><color r="1" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 500 400">
    <fragment id="10">
      <n id="11" p="50 100" Element="6">
        <objecttag id="12" Name="catalog" DisplayName="Catalog" TagType="String" Value="A-1" PositioningType="offset" PositioningOffset="2 3">
          <t id="15" p="50 85"><s font="3" size="8" color="2">A-1</s></t>
        </objecttag>
        <annotation id="16" Keyword="review" Content="approved"/>
      </n>
      <n id="13" p="80 100" Element="6" AltGroupID="30"/>
      <b id="14" B="11" E="13"/>
      <regnum id="17" RegistryAuthority="CAS" RegistryNumber="50-00-0"/>
    </fragment>
    <graphic id="20" GraphicType="Bracket" BracketType="Square" BoundingBox="35 70 40 130"/>
    <graphic id="21" GraphicType="Bracket" BracketType="Square" BoundingBox="90 70 95 130"/>
    <bracketedgroup id="22" BracketedObjectIDs="10" BracketUsage="SRU" ComponentOrder="2" PolymerRepeatPattern="HeadToTail" PolymerFlipType="NoFlip" RepeatCount="3" SRULabel="n">
      <bracketattachment id="23" GraphicID="20">
        <crossingbond id="24" BondID="14" InnerAtomID="11"/>
      </bracketattachment>
      <bracketattachment id="25" GraphicID="21"/>
      <bracketedgroup id="26" BracketedObjectIDs="10" BracketUsage="Component">
        <bracketattachment id="27" GraphicID="20"/>
      </bracketedgroup>
    </bracketedgroup>
    <altgroup id="30" Valence="1" p="200 100" BoundingBox="150 50 250 150" alpha="0.5" color="4" Z="12" SupersededBy="10">
      <fragment id="31"><n id="32" p="180 100" Element="8"/></fragment>
    </altgroup>
    <sequence id="40" SequenceIdentifier="sequence-alpha">
      <t id="41" p="50 180"><s font="3" size="10" color="2">ACGT</s></t>
    </sequence>
    <crossreference id="42" CrossReferenceIdentifier="xref-alpha" CrossReferenceSequence="sequence-alpha">
      <t id="43" p="50 210"><s font="3" size="10" color="2">see sequence</s></t>
    </crossreference>
    <graphic id="50" GraphicType="Symbol" SymbolType="Plus" BoundingBox="300 100 310 110">
      <represent object="11" attribute="Element"/>
    </graphic>
  </page>
</CDXML>
"#;

    #[test]
    fn imports_all_nr_017_logical_object_families_into_native_ccjs() {
        let mut document =
            crate::parse_cdxml_document(LOGICAL_OBJECTS_CDXML, Some("logical")).unwrap();
        let logical = &document.logical_objects;
        assert_eq!(logical.alternative_groups.len(), 1);
        assert_eq!(logical.alternative_groups[0].attachment_node_ids, ["13"]);
        assert_eq!(logical.bracketed_groups.len(), 2);
        assert_eq!(logical.bracketed_groups[0].usage, crate::BracketUsage::Sru);
        assert_eq!(logical.bracketed_groups[0].nested_group_ids, ["26"]);
        assert_eq!(logical.bracketed_groups[0].component_order, Some(2));
        assert_eq!(logical.alternative_groups[0].valence, Some(1));
        assert_eq!(logical.alternative_groups[0].position, Some([200.0, 100.0]));
        assert_eq!(logical.alternative_groups[0].opacity, Some(0.5));
        assert_eq!(
            logical.alternative_groups[0].color.as_deref(),
            Some("#ff0000")
        );
        assert_eq!(logical.alternative_groups[0].z_index, Some(12));
        assert!(logical.alternative_groups[0].superseded_by_id.is_some());
        assert_eq!(
            logical.bracketed_groups[0].polymer_repeat_pattern,
            crate::PolymerRepeatPattern::HeadToTail
        );
        assert_eq!(
            logical.bracketed_groups[0].attachments[0].crossing_bonds[0]
                .bond_id
                .as_deref(),
            Some("14")
        );
        assert!(logical.bracketed_groups[0].attachments[0]
            .bracket_object_id
            .is_some());
        assert!(logical.bracketed_groups[0].attachments[0]
            .unresolved_bracket_source_id
            .is_none());
        assert_eq!(logical.sequences[0].identifier, "sequence-alpha");
        assert_eq!(
            logical.cross_references[0].sequence_identifier,
            "sequence-alpha"
        );
        let catalog = logical
            .object_tags
            .iter()
            .find(|tag| tag.name == "catalog")
            .unwrap();
        assert_eq!(catalog.owner_entity_id.as_deref(), Some("11"));
        assert_eq!(catalog.value.as_deref(), Some("A-1"));
        assert_eq!(
            logical.annotations[0].owner_entity_id.as_deref(),
            Some("11")
        );
        assert_eq!(logical.registry_numbers[0].authority, "CAS");
        assert_eq!(
            logical.representations[0].target_entity_id.as_deref(),
            Some("11")
        );

        let scene_ids = document
            .scene_objects()
            .into_iter()
            .map(|object| object.id.clone())
            .collect();
        let node_ids = document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| entry.fragment.nodes.iter().map(|node| node.id.clone()))
            .collect();
        let bond_ids = document
            .editable_fragments()
            .into_iter()
            .flat_map(|entry| entry.fragment.bonds.iter().map(|bond| bond.id.clone()))
            .collect();
        logical.validate(&scene_ids, &node_ids, &bond_ids).unwrap();

        let json = serde_json::to_string(&document).unwrap();
        let reopened = crate::parse_document_json(&json).unwrap();
        assert_eq!(reopened.logical_objects, document.logical_objects);

        document.logical_objects.object_tags[0].value = Some("A-2".to_string());
        let mut second_catalog = document.logical_objects.object_tags[0].clone();
        second_catalog.id = "tag_secondary".to_string();
        second_catalog.value = Some("B-7".to_string());
        second_catalog.display_object_ids.clear();
        document.logical_objects.object_tags.push(second_catalog);
        document.logical_objects.annotations[0].content = Some("edited".to_string());
        document.logical_objects.registry_numbers[0].number = "64-17-5".to_string();
        document.logical_objects.bracketed_groups[0].repeat_count = Some(4.0);
        document.logical_objects.sequences[0].identifier = "sequence-beta".to_string();
        document.logical_objects.cross_references[0].sequence_identifier =
            "sequence-beta".to_string();
        let exported = crate::document_to_cdxml(&document);
        let round_trip = crate::parse_cdxml_document(&exported, Some("logical edited")).unwrap();
        assert_eq!(
            round_trip.logical_objects.object_tags[0].value.as_deref(),
            Some("A-2")
        );
        assert_eq!(
            round_trip
                .logical_objects
                .object_tags
                .iter()
                .filter(|tag| tag.name == "catalog")
                .count(),
            2
        );
        assert_eq!(
            round_trip.logical_objects.annotations[0].content.as_deref(),
            Some("edited")
        );
        assert_eq!(
            round_trip.logical_objects.registry_numbers[0].number,
            "64-17-5"
        );
        assert_eq!(
            round_trip.logical_objects.bracketed_groups[0].repeat_count,
            Some(4.0)
        );
        assert_eq!(
            round_trip.logical_objects.bracketed_groups[0].component_order,
            Some(2)
        );
        assert_eq!(
            round_trip.logical_objects.cross_references[0].sequence_identifier,
            "sequence-beta"
        );
        assert_eq!(exported.matches("<altgroup").count(), 1);
        assert_eq!(exported.matches("<bracketedgroup").count(), 2);
        assert_eq!(
            round_trip.logical_objects.bracketed_groups[0].nested_group_ids,
            [round_trip.logical_objects.bracketed_groups[1].id.clone()]
        );
        assert!(exported.contains("alpha=\"0.5\""));
        assert!(exported.contains("color=\""));
        assert!(exported.contains("SupersededBy=\""));
        assert_eq!(exported.matches("<sequence").count(), 1);
        assert_eq!(exported.matches("<crossreference").count(), 1);
        assert_eq!(exported.matches("Name=\"catalog\"").count(), 2);

        let cdx = crate::cdxml_to_cdx(&exported).unwrap();
        let cdx_round_trip = crate::parse_cdx_document(&cdx, Some("logical cdx")).unwrap();
        assert_eq!(
            cdx_round_trip.logical_objects.registry_numbers[0].number,
            "64-17-5"
        );
        assert_eq!(
            cdx_round_trip.logical_objects.bracketed_groups[0].repeat_count,
            Some(4.0)
        );
        assert_eq!(
            cdx_round_trip.logical_objects.bracketed_groups[0].component_order,
            Some(2)
        );
        assert_eq!(
            cdx_round_trip.logical_objects.alternative_groups[0].valence,
            Some(1)
        );
        assert_eq!(
            cdx_round_trip.logical_objects.bracketed_groups[0].nested_group_ids,
            ["26"]
        );
        assert_eq!(
            cdx_round_trip.logical_objects.alternative_groups[0]
                .color
                .as_deref(),
            Some("#ff0000")
        );
    }
}
