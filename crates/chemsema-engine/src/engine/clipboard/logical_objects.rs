use super::super::Engine;
use crate::{ChemSemaDocument, LogicalBindingOrigin, LogicalObjectData};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn selected_logical_objects(
    document: &ChemSemaDocument,
    selected_entity_ids: &BTreeSet<String>,
) -> LogicalObjectData {
    document
        .logical_objects
        .subset_for_entities(selected_entity_ids)
}

pub(super) fn remap_clipboard_logical_objects(
    engine: &mut Engine,
    logical: &mut LogicalObjectData,
    entity_id_map: &BTreeMap<String, String>,
) {
    let remap_ids = |ids: &mut Vec<String>| {
        *ids = ids
            .iter()
            .filter_map(|id| entity_id_map.get(id).cloned())
            .collect();
    };
    let alternative_id_map = logical
        .alternative_groups
        .iter()
        .map(|group| (group.id.clone(), engine.next_id("alternative_group")))
        .collect::<BTreeMap<_, _>>();
    let bracket_group_id_map = logical
        .bracketed_groups
        .iter()
        .map(|group| (group.id.clone(), engine.next_id("bracketed_group")))
        .collect::<BTreeMap<_, _>>();
    for group in &mut logical.alternative_groups {
        group.id = alternative_id_map
            .get(&group.id)
            .cloned()
            .expect("alternative-group clipboard ID was allocated");
        remap_ids(&mut group.member_entity_ids);
        remap_ids(&mut group.attachment_node_ids);
        group.superseded_by_id = group.superseded_by_id.as_ref().and_then(|id| {
            alternative_id_map
                .get(id)
                .or_else(|| bracket_group_id_map.get(id))
                .or_else(|| entity_id_map.get(id))
                .cloned()
        });
        group.unresolved_member_source_ids.clear();
        group.unresolved_superseded_by_source_id = None;
        group.binding_origin = LogicalBindingOrigin::Authored;
    }
    for group in &mut logical.bracketed_groups {
        group.id = bracket_group_id_map
            .get(&group.id)
            .cloned()
            .expect("bracketed-group clipboard ID was allocated");
        remap_ids(&mut group.bracket_object_ids);
        remap_ids(&mut group.bracketed_entity_ids);
        group.nested_group_ids = group
            .nested_group_ids
            .iter()
            .filter_map(|id| bracket_group_id_map.get(id).cloned())
            .collect();
        group.unresolved_bracket_source_ids.clear();
        group.unresolved_bracketed_source_ids.clear();
        for attachment in &mut group.attachments {
            attachment.id = engine.next_id("bracket_attachment");
            attachment.bracket_object_id = attachment
                .bracket_object_id
                .as_ref()
                .and_then(|id| entity_id_map.get(id))
                .cloned();
            attachment.unresolved_bracket_source_id = None;
            for crossing in &mut attachment.crossing_bonds {
                crossing.id = engine.next_id("crossing_bond");
                crossing.bond_id = crossing
                    .bond_id
                    .as_ref()
                    .and_then(|id| entity_id_map.get(id))
                    .cloned();
                crossing.inner_atom_id = crossing
                    .inner_atom_id
                    .as_ref()
                    .and_then(|id| entity_id_map.get(id))
                    .cloned();
                crossing.unresolved_bond_source_id = None;
                crossing.unresolved_inner_atom_source_id = None;
            }
        }
        group.binding_origin = LogicalBindingOrigin::Authored;
    }
    let mut sequence_identifier_map = BTreeMap::new();
    for sequence in &mut logical.sequences {
        sequence.id = engine.next_id("sequence");
        let next_identifier = engine.next_id("sequence_identifier");
        sequence_identifier_map.insert(sequence.identifier.clone(), next_identifier.clone());
        sequence.identifier = next_identifier;
        remap_ids(&mut sequence.text_object_ids);
        sequence.binding_origin = LogicalBindingOrigin::Authored;
    }
    for cross_reference in &mut logical.cross_references {
        cross_reference.id = engine.next_id("cross_reference");
        cross_reference.identifier = engine.next_id("cross_reference_identifier");
        if let Some(next) = sequence_identifier_map.get(&cross_reference.sequence_identifier) {
            cross_reference.sequence_identifier = next.clone();
        }
        remap_ids(&mut cross_reference.text_object_ids);
        cross_reference.binding_origin = LogicalBindingOrigin::Authored;
    }
    for tag in &mut logical.object_tags {
        tag.id = engine.next_id("object_tag");
        tag.owner_entity_id = tag
            .owner_entity_id
            .as_ref()
            .and_then(|id| entity_id_map.get(id))
            .cloned();
        remap_ids(&mut tag.display_object_ids);
        tag.binding_origin = LogicalBindingOrigin::Authored;
    }
    for annotation in &mut logical.annotations {
        annotation.id = engine.next_id("annotation");
        annotation.owner_entity_id = annotation
            .owner_entity_id
            .as_ref()
            .and_then(|id| entity_id_map.get(id))
            .cloned();
        annotation.binding_origin = LogicalBindingOrigin::Authored;
    }
    for registration in &mut logical.registry_numbers {
        registration.id = engine.next_id("registry_number");
        registration.owner_entity_id = registration
            .owner_entity_id
            .as_ref()
            .and_then(|id| entity_id_map.get(id))
            .cloned();
        registration.binding_origin = LogicalBindingOrigin::Authored;
    }
    for representation in &mut logical.representations {
        representation.id = engine.next_id("representation");
        representation.owner_entity_id = representation
            .owner_entity_id
            .as_ref()
            .and_then(|id| entity_id_map.get(id))
            .cloned();
        representation.target_entity_id = representation
            .target_entity_id
            .as_ref()
            .and_then(|id| entity_id_map.get(id))
            .cloned();
        representation.binding_origin = LogicalBindingOrigin::Authored;
    }
}

pub(super) fn append_logical_objects(target: &mut LogicalObjectData, source: LogicalObjectData) {
    target.alternative_groups.extend(source.alternative_groups);
    target.bracketed_groups.extend(source.bracketed_groups);
    target.sequences.extend(source.sequences);
    target.cross_references.extend(source.cross_references);
    target.object_tags.extend(source.object_tags);
    target.annotations.extend(source.annotations);
    target.registry_numbers.extend(source.registry_numbers);
    target.representations.extend(source.representations);
}
