use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{EditorCommand, Engine, RenderBoundsScope};
use crate::{
    Bond, ChemSemaDocument, ChemicalProperty, LinkRelation, Node, Resource, ResourceData,
    SceneObject, SelectionState,
};
use chemsema_chemical_graph::{MultiCenterInteractionV2, StereoElementV2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const CLIPBOARD_PASTE_OFFSET_PT: f64 = 9.921_259_842_519_685;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClipboardContent {
    #[serde(default)]
    nodes: Vec<Node>,
    #[serde(default)]
    bonds: Vec<Bond>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    stereo: Vec<StereoElementV2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    interactions: Vec<MultiCenterInteractionV2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scene_objects: Vec<SceneObject>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    resources: BTreeMap<String, Resource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    links: Vec<LinkRelation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chemical_properties: Vec<ChemicalProperty>,
}

impl Engine {
    pub fn has_clipboard(&self) -> bool {
        self.clipboard
            .as_ref()
            .is_some_and(|content| !content.nodes.is_empty() || !content.scene_objects.is_empty())
    }

    pub fn copy_selection(&mut self) -> bool {
        let Some(content) = self.clipboard_content_from_selection() else {
            return false;
        };
        self.clipboard = Some(content);
        true
    }

    pub fn clipboard_selection_json(&self) -> Result<Option<String>, String> {
        self.clipboard_content_from_selection()
            .map(|content| serde_json::to_string(&content).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn clipboard_document_json(&self) -> Result<Option<String>, String> {
        self.document_from_selection()
            .map(|document| serde_json::to_string(&document).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn clipboard_cdxml(&self) -> Option<String> {
        self.document_from_selection()
            .map(|document| crate::document_to_cdxml(&document))
    }

    pub fn cut_selection(&mut self) -> bool {
        self.with_command(EditorCommand::CutSelection, |engine| {
            engine.cut_selection_untracked()
        })
    }

    fn cut_selection_untracked(&mut self) -> bool {
        if !self.copy_selection() {
            return false;
        }
        self.delete_selection()
    }

    pub fn paste_clipboard(&mut self) -> bool {
        self.with_command(EditorCommand::PasteClipboard, |engine| {
            engine.paste_clipboard_untracked()
        })
    }

    pub fn paste_clipboard_json(&mut self, json: &str) -> Result<bool, String> {
        let content: ClipboardContent =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        self.clipboard = Some(content);
        Ok(self.paste_clipboard())
    }

    pub fn paste_document_json(&mut self, json: &str) -> Result<bool, String> {
        let mut source = Engine::new();
        source.load_document_json(json)?;
        self.paste_external_document(source)
    }

    pub fn paste_cdxml(&mut self, cdxml: &str) -> Result<bool, String> {
        let mut source = Engine::new();
        source.load_cdxml_document(cdxml)?;
        self.paste_external_document(source)
    }

    pub fn paste_cdx(&mut self, cdx: &[u8]) -> Result<bool, String> {
        let mut source = Engine::new();
        source.load_cdx_document(cdx)?;
        self.paste_external_document(source)
    }

    fn paste_external_document(&mut self, mut source: Engine) -> Result<bool, String> {
        if !source.select_all() {
            return Ok(false);
        }
        let Some(document) = source.document_from_selection() else {
            return Ok(false);
        };
        let mut resources = BTreeMap::new();
        for object in &document.objects {
            collect_scene_object_resources(object, &document.resources, &mut resources);
        }
        let content = ClipboardContent {
            nodes: Vec::new(),
            bonds: Vec::new(),
            stereo: Vec::new(),
            interactions: Vec::new(),
            scene_objects: document.objects,
            resources,
            links: document.links,
            chemical_properties: document.chemical_properties,
        };
        self.clipboard = Some(content);
        Ok(self.paste_clipboard())
    }

    fn paste_clipboard_untracked(&mut self) -> bool {
        let Some(content) = self.clipboard.clone() else {
            return false;
        };
        if content.nodes.is_empty() && content.scene_objects.is_empty() {
            return false;
        }
        if !content.nodes.is_empty() && self.state.document.editable_fragment().is_none() {
            return false;
        }
        self.push_undo_snapshot();
        let mut id_map = BTreeMap::new();
        let mut pasted_node_ids = Vec::new();
        let mut pasted_bond_ids = Vec::new();
        let mut bond_id_map = BTreeMap::new();
        let mut nodes_to_insert = Vec::new();
        let mut bonds_to_insert = Vec::new();

        for node in &content.nodes {
            let next_id = self.next_id("n");
            id_map.insert(node.id.clone(), next_id.clone());
            let mut next = node.clone();
            next.id = next_id.clone();
            next.position[0] = crate::round2(next.position[0] + CLIPBOARD_PASTE_OFFSET_PT);
            next.position[1] = crate::round2(next.position[1] + CLIPBOARD_PASTE_OFFSET_PT);
            nodes_to_insert.push(next);
            pasted_node_ids.push(next_id);
        }

        for bond in &content.bonds {
            let (Some(begin), Some(end)) = (id_map.get(&bond.begin), id_map.get(&bond.end)) else {
                continue;
            };
            let mut next = bond.clone();
            next.id = self.next_id("b");
            bond_id_map.insert(bond.id.clone(), next.id.clone());
            next.begin = begin.clone();
            next.end = end.clone();
            pasted_bond_ids.push(next.id.clone());
            bonds_to_insert.push(next);
        }
        let (stereo_to_insert, interactions_to_insert) = remap_clipboard_semantics(
            self,
            &content.stereo,
            &content.interactions,
            &id_map,
            &bond_id_map,
        );

        if !nodes_to_insert.is_empty() {
            let stroke_width = self.options.bond_stroke_world_pt().value();
            let Some(mut entry) = self.state.document.editable_fragment_mut() else {
                self.undo_stack.pop();
                return false;
            };
            entry.fragment.nodes.extend(nodes_to_insert);
            entry.fragment.bonds.extend(bonds_to_insert);
            entry.fragment.stereo.extend(stereo_to_insert);
            entry.fragment.interactions.extend(interactions_to_insert);

            let object_translate = entry.object.transform.translate;
            refresh_attached_node_label_geometry_for_all_nodes(
                entry.fragment,
                object_translate,
                stroke_width,
            );
            entry.update_bounds();
        }

        let mut entity_id_map = id_map.clone();
        let mut resource_id_map = BTreeMap::new();
        for (source_id, resource) in &content.resources {
            let target_id = self.next_id("res");
            let mut resource = resource.clone();
            remap_clipboard_resource(self, &mut resource, &mut entity_id_map);
            self.state
                .document
                .resources
                .insert(target_id.clone(), resource);
            resource_id_map.insert(source_id.clone(), target_id);
        }
        let mut pasted_scene_ids = Vec::new();
        let mut pasted_molecule_ids = Vec::new();
        for source in &content.scene_objects {
            let source_id = source.id.clone();
            let mut object = super::select::drag::translated_scene_object(
                source,
                CLIPBOARD_PASTE_OFFSET_PT,
                CLIPBOARD_PASTE_OFFSET_PT,
            );
            remap_clipboard_scene_object(self, &mut object, &resource_id_map);
            entity_id_map.insert(source_id, object.id.clone());
            if object.object_type == "molecule" {
                pasted_molecule_ids.push(object.id.clone());
            } else {
                pasted_scene_ids.push(object.id.clone());
            }
            self.state.document.objects.push(object);
        }
        let pasted_link_start = self.state.document.links.len();
        for relation in &content.links {
            let mut relation = relation.clone();
            let mut complete = true;
            for endpoint in &mut relation.endpoints {
                if let Some(next) = entity_id_map.get(&endpoint.entity_id) {
                    endpoint.entity_id = next.clone();
                } else {
                    complete = false;
                    break;
                }
            }
            if complete {
                relation.id = self.next_id("link");
                self.state.document.links.push(relation);
            }
        }
        for property in &content.chemical_properties {
            let mut property = property.clone();
            let source_property_id = property.id.clone();
            property.id = self.next_id("chemical_property");
            entity_id_map.insert(source_property_id, property.id.clone());
            property.source_id = None;
            property.basis_entity_ids = property
                .basis_entity_ids
                .iter()
                .filter_map(|id| entity_id_map.get(id).cloned())
                .collect();
            property.display_object_id = property
                .display_object_id
                .as_ref()
                .and_then(|id| entity_id_map.get(id))
                .cloned();
            property.unresolved_basis_ids.clear();
            self.state.document.chemical_properties.push(property);
        }
        for relation in &mut self.state.document.links[pasted_link_start..] {
            if relation.kind != "chemical-property-display" {
                continue;
            }
            let Some(source_id) = relation
                .data
                .get("chemicalPropertyId")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            if let Some(property_id) = entity_id_map.get(source_id) {
                relation.data["chemicalPropertyId"] = serde_json::json!(property_id);
            }
        }
        self.state.selection = SelectionState {
            arrow_objects: pasted_scene_ids,
            molecule_objects: pasted_molecule_ids,
            nodes: pasted_node_ids,
            bonds: pasted_bond_ids,
            ..SelectionState::default()
        };
        self.clear_interaction();
        true
    }

    fn clipboard_content_from_selection(&self) -> Option<ClipboardContent> {
        if self.state.selection.is_empty() {
            return None;
        }
        let entry = self.state.document.editable_fragment();
        let mut node_ids: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        node_ids.extend(self.state.selection.label_nodes.iter().cloned());

        let selected_bonds: BTreeSet<&str> = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect();
        if let Some(entry) = entry.as_ref() {
            for bond in &entry.fragment.bonds {
                if selected_bonds.contains(bond.id.as_str()) {
                    node_ids.insert(bond.begin.clone());
                    node_ids.insert(bond.end.clone());
                }
            }
        }

        let selected_molecule_objects: BTreeSet<&str> = self
            .state
            .selection
            .molecule_objects
            .iter()
            .map(String::as_str)
            .collect();
        let fully_selected_molecule_ids: BTreeSet<String> = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .filter(|candidate| {
                selected_molecule_objects.contains(candidate.object.id.as_str())
                    && (self
                        .state
                        .selection
                        .molecule_objects
                        .contains(&candidate.object.id)
                        || (candidate
                            .fragment
                            .nodes
                            .iter()
                            .all(|node| node_ids.contains(&node.id))
                            && candidate
                                .fragment
                                .bonds
                                .iter()
                                .all(|bond| selected_bonds.contains(bond.id.as_str()))))
            })
            .map(|candidate| candidate.object.id.clone())
            .collect();
        let active_molecule_is_complete = entry
            .as_ref()
            .is_some_and(|entry| fully_selected_molecule_ids.contains(entry.object.id.as_str()));

        let nodes: Vec<Node> = entry
            .as_ref()
            .filter(|_| !active_molecule_is_complete)
            .map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .filter(|node| node_ids.contains(&node.id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let bonds: Vec<Bond> = entry
            .as_ref()
            .filter(|_| !active_molecule_is_complete)
            .map(|entry| {
                entry
                    .fragment
                    .bonds
                    .iter()
                    .filter(|bond| {
                        selected_bonds.contains(bond.id.as_str())
                            && node_ids.contains(&bond.begin)
                            && node_ids.contains(&bond.end)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let bond_ids = bonds
            .iter()
            .map(|bond| bond.id.clone())
            .collect::<BTreeSet<_>>();
        let (stereo, interactions) = entry
            .as_ref()
            .filter(|_| !active_molecule_is_complete)
            .map(|entry| crate::subset_molecule_semantics(entry.fragment, &node_ids, &bond_ids))
            .unwrap_or_default();

        let mut selected_scene_ids: BTreeSet<&str> = self
            .state
            .selection
            .arrow_objects
            .iter()
            .map(String::as_str)
            .collect();
        selected_scene_ids.extend(self.state.selection.text_objects.iter().map(String::as_str));
        selected_scene_ids.extend(fully_selected_molecule_ids.iter().map(String::as_str));
        let mut scene_objects = Vec::new();
        let mut resources = BTreeMap::new();
        for object in &self.state.document.objects {
            collect_clipboard_scene_objects(
                object,
                &selected_scene_ids,
                &self.state.document.resources,
                &mut scene_objects,
                &mut resources,
            );
        }
        if nodes.is_empty() && scene_objects.is_empty() {
            return None;
        }
        let mut selected_entity_ids = selected_scene_ids
            .iter()
            .map(|id| (*id).to_string())
            .chain(node_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        for resource in resources.values() {
            if let ResourceData::Fragment(fragment) = &resource.data {
                selected_entity_ids.extend(fragment.nodes.iter().map(|node| node.id.clone()));
                selected_entity_ids.extend(fragment.bonds.iter().map(|bond| bond.id.clone()));
            }
        }
        let links = self
            .state
            .document
            .links
            .iter()
            .filter(|relation| {
                relation
                    .endpoints
                    .iter()
                    .all(|endpoint| selected_entity_ids.contains(&endpoint.entity_id))
            })
            .cloned()
            .collect();
        let chemical_properties = self
            .state
            .document
            .chemical_properties
            .iter()
            .filter(|property| {
                property
                    .basis_entity_ids
                    .iter()
                    .all(|id| selected_entity_ids.contains(id))
                    && property
                        .display_object_id
                        .as_ref()
                        .is_none_or(|id| selected_entity_ids.contains(id))
            })
            .cloned()
            .collect();

        Some(ClipboardContent {
            nodes,
            bonds,
            stereo,
            interactions,
            scene_objects,
            resources,
            links,
            chemical_properties,
        })
    }

    fn document_from_selection(&self) -> Option<ChemSemaDocument> {
        if self.state.selection.is_empty() {
            return None;
        }

        if self.selection_covers_visible_document() {
            let mut document = self.state.document.clone();
            if let Some(bounds) = self.render_bounds(RenderBoundsScope::Selection) {
                set_clipboard_selection_bounds_meta(&mut document, bounds);
            }
            return Some(document);
        }

        let selected_molecule = self.selected_molecule_clipboard_object();
        let mut selected_object_ids: BTreeSet<String> =
            self.state.selection.text_objects.iter().cloned().collect();
        selected_object_ids.extend(self.state.selection.arrow_objects.iter().cloned());

        let mut objects = Vec::new();
        for object in &self.state.document.objects {
            if selected_molecule
                .as_ref()
                .is_some_and(|(molecule, _, _)| molecule.id == object.id)
            {
                objects.push(selected_molecule.as_ref().unwrap().0.clone());
                continue;
            }
            clone_selected_scene_objects(object, &selected_object_ids, &mut objects);
        }
        if objects.is_empty() {
            return None;
        }

        let mut document = self.state.document.clone();
        document.document.id = "doc_clipboard_selection".to_string();
        document.document.title = "ChemSema Clipboard Selection".to_string();
        document.objects = objects;
        if let Some((_, resource_ref, resource)) = selected_molecule {
            document.resources.insert(resource_ref, resource);
        }
        let mut retained = document
            .scene_objects()
            .into_iter()
            .map(|object| object.id.clone())
            .collect::<BTreeSet<_>>();
        for object in document.scene_objects() {
            let Some(resource_id) = object.payload.resource_ref.as_ref() else {
                continue;
            };
            let Some(ResourceData::Fragment(fragment)) = document
                .resources
                .get(resource_id)
                .map(|resource| &resource.data)
            else {
                continue;
            };
            retained.extend(fragment.nodes.iter().map(|node| node.id.clone()));
            retained.extend(fragment.bonds.iter().map(|bond| bond.id.clone()));
        }
        document.links.retain(|relation| {
            relation
                .endpoints
                .iter()
                .all(|endpoint| retained.contains(&endpoint.entity_id))
        });
        document.chemical_properties.retain(|property| {
            property
                .basis_entity_ids
                .iter()
                .all(|id| retained.contains(id))
                && property
                    .display_object_id
                    .as_ref()
                    .is_none_or(|id| retained.contains(id))
        });
        if let Some(bounds) = self.render_bounds(RenderBoundsScope::Selection) {
            set_clipboard_selection_bounds_meta(&mut document, bounds);
        }
        Some(document)
    }

    fn selected_molecule_clipboard_object(&self) -> Option<(SceneObject, String, Resource)> {
        let entry = self.state.document.editable_fragment()?;
        let resource_ref = entry.object.payload.resource_ref.clone()?;

        let mut node_ids: BTreeSet<String> = self.state.selection.nodes.iter().cloned().collect();
        node_ids.extend(self.state.selection.label_nodes.iter().cloned());

        let selected_bonds: BTreeSet<&str> = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect();
        for bond in &entry.fragment.bonds {
            if selected_bonds.contains(bond.id.as_str()) {
                node_ids.insert(bond.begin.clone());
                node_ids.insert(bond.end.clone());
            }
        }

        let nodes: Vec<Node> = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| node_ids.contains(&node.id))
            .cloned()
            .collect();
        if nodes.is_empty() {
            return None;
        }

        let bonds: Vec<Bond> = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                selected_bonds.contains(bond.id.as_str())
                    && node_ids.contains(&bond.begin)
                    && node_ids.contains(&bond.end)
            })
            .cloned()
            .collect();

        let mut fragment = entry.fragment.clone();
        fragment.nodes = nodes;
        fragment.bonds = bonds;
        let bond_ids = fragment
            .bonds
            .iter()
            .map(|bond| bond.id.clone())
            .collect::<BTreeSet<_>>();
        let (stereo, interactions) =
            crate::subset_molecule_semantics(entry.fragment, &node_ids, &bond_ids);
        fragment.stereo = stereo;
        fragment.interactions = interactions;
        fragment.bbox = fragment_clipboard_bounds(&fragment.nodes);

        let mut object = entry.object.clone();
        object.payload.bbox = Some(fragment.bbox);

        let mut resource = self.state.document.resources.get(&resource_ref)?.clone();
        resource.data = ResourceData::Fragment(fragment);
        Some((object, resource_ref, resource))
    }

    fn selection_covers_visible_document(&self) -> bool {
        if self.state.selection.is_empty() {
            return false;
        }

        let selected_molecules: BTreeSet<&str> = self
            .state
            .selection
            .molecule_objects
            .iter()
            .map(String::as_str)
            .collect();
        let selected_text: BTreeSet<&str> = self
            .state
            .selection
            .text_objects
            .iter()
            .map(String::as_str)
            .collect();
        let selected_graphics: BTreeSet<&str> = self
            .state
            .selection
            .arrow_objects
            .iter()
            .map(String::as_str)
            .collect();

        if self
            .state
            .document
            .editable_fragments()
            .iter()
            .any(|entry| !selected_molecules.contains(entry.object.id.as_str()))
        {
            return false;
        }

        self.state.document.objects.iter().all(|object| {
            visible_root_object_is_selected_for_clipboard(
                object,
                &selected_text,
                &selected_graphics,
                &selected_molecules,
            )
        })
    }
}

fn visible_root_object_is_selected_for_clipboard(
    object: &SceneObject,
    selected_text: &BTreeSet<&str>,
    selected_graphics: &BTreeSet<&str>,
    selected_molecules: &BTreeSet<&str>,
) -> bool {
    if !object.visible {
        return true;
    }
    match object.kind() {
        crate::SceneObjectKind::Text => selected_text.contains(object.id.as_str()),
        crate::SceneObjectKind::Line
        | crate::SceneObjectKind::Curve
        | crate::SceneObjectKind::Bracket
        | crate::SceneObjectKind::Symbol
        | crate::SceneObjectKind::Shape
        | crate::SceneObjectKind::Image
        | crate::SceneObjectKind::Spectrum
        | crate::SceneObjectKind::Group => selected_graphics.contains(object.id.as_str()),
        crate::SceneObjectKind::Molecule => selected_molecules.contains(object.id.as_str()),
    }
}

fn collect_clipboard_scene_objects(
    object: &SceneObject,
    selected_ids: &BTreeSet<&str>,
    document_resources: &BTreeMap<String, Resource>,
    out: &mut Vec<SceneObject>,
    resources: &mut BTreeMap<String, Resource>,
) {
    if selected_ids.contains(object.id.as_str()) {
        collect_scene_object_resources(object, document_resources, resources);
        out.push(object.clone());
        return;
    }
    for child in &object.children {
        collect_clipboard_scene_objects(child, selected_ids, document_resources, out, resources);
    }
}

fn collect_scene_object_resources(
    object: &SceneObject,
    document_resources: &BTreeMap<String, Resource>,
    resources: &mut BTreeMap<String, Resource>,
) {
    if let Some(resource_id) = object.payload.resource_ref.as_ref() {
        if let Some(resource) = document_resources.get(resource_id) {
            resources.insert(resource_id.clone(), resource.clone());
        }
    }
    for child in &object.children {
        collect_scene_object_resources(child, document_resources, resources);
    }
}

fn remap_clipboard_scene_object(
    engine: &mut Engine,
    object: &mut SceneObject,
    resource_id_map: &BTreeMap<String, String>,
) {
    object.id = engine.next_id("object");
    if let Some(resource_id) = object.payload.resource_ref.as_mut() {
        if let Some(target_id) = resource_id_map.get(resource_id) {
            *resource_id = target_id.clone();
        }
    }
    for child in &mut object.children {
        remap_clipboard_scene_object(engine, child, resource_id_map);
    }
}

fn remap_clipboard_resource(
    engine: &mut Engine,
    resource: &mut Resource,
    entity_ids: &mut BTreeMap<String, String>,
) {
    let ResourceData::Fragment(fragment) = &mut resource.data else {
        return;
    };
    let mut node_ids = BTreeMap::new();
    for node in &mut fragment.nodes {
        let source_id = node.id.clone();
        let target_id = engine.next_id("n");
        node.id = target_id.clone();
        node_ids.insert(source_id.clone(), target_id.clone());
        entity_ids.insert(source_id, target_id);
    }
    let mut bond_ids = BTreeMap::new();
    for bond in &mut fragment.bonds {
        let source_id = bond.id.clone();
        let target_id = engine.next_id("b");
        bond.id = target_id.clone();
        bond.begin = node_ids
            .get(&bond.begin)
            .expect("clipboard resource bond begin was retained")
            .clone();
        bond.end = node_ids
            .get(&bond.end)
            .expect("clipboard resource bond end was retained")
            .clone();
        bond_ids.insert(source_id, target_id);
    }
    let (stereo, interactions) = remap_clipboard_semantics(
        engine,
        &fragment.stereo,
        &fragment.interactions,
        &node_ids,
        &bond_ids,
    );
    fragment.stereo = stereo;
    fragment.interactions = interactions;
}

fn clone_selected_scene_objects(
    object: &SceneObject,
    selected_ids: &BTreeSet<String>,
    out: &mut Vec<SceneObject>,
) {
    if selected_ids.contains(&object.id) {
        out.push(object.clone());
        return;
    }

    let mut children = Vec::new();
    for child in &object.children {
        clone_selected_scene_objects(child, selected_ids, &mut children);
    }
    if !children.is_empty() {
        let mut clone = object.clone();
        clone.children = children;
        out.push(clone);
    }
}

fn remap_clipboard_semantics(
    engine: &mut Engine,
    stereo: &[StereoElementV2],
    interactions: &[MultiCenterInteractionV2],
    node_ids: &BTreeMap<String, String>,
    bond_ids: &BTreeMap<String, String>,
) -> (Vec<StereoElementV2>, Vec<MultiCenterInteractionV2>) {
    use chemsema_chemical_graph::{StereoCarrierV2, StereoReferenceV2};

    let source_stereo_ids = stereo
        .iter()
        .map(|element| match element {
            StereoElementV2::Tetrahedral { id, .. }
            | StereoElementV2::DoubleBond { id, .. }
            | StereoElementV2::EnhancedGroup { id, .. }
            | StereoElementV2::Extended { id, .. }
            | StereoElementV2::Conformation { id, .. }
            | StereoElementV2::Unspecified { id, .. } => id,
        })
        .cloned()
        .collect::<Vec<_>>();
    let stereo_ids = source_stereo_ids
        .iter()
        .map(|id| (id.clone(), engine.next_id("stereo")))
        .collect::<BTreeMap<_, _>>();
    let map_atom = |id: &mut String| {
        *id = node_ids
            .get(id)
            .expect("clipboard semantic atom was retained")
            .clone();
    };
    let map_bond = |id: &mut String| {
        *id = bond_ids
            .get(id)
            .expect("clipboard semantic bond was retained")
            .clone();
    };
    let map_carrier = |carrier: &mut StereoCarrierV2| match carrier {
        StereoCarrierV2::Atom(atom)
        | StereoCarrierV2::LonePair(atom)
        | StereoCarrierV2::DuplicateAtom(atom) => map_atom(atom),
        StereoCarrierV2::Bond(bond) => map_bond(bond),
        StereoCarrierV2::AtomSet(atoms) | StereoCarrierV2::Plane(atoms) => {
            atoms.iter_mut().for_each(&map_atom)
        }
        StereoCarrierV2::Axis(atoms) => atoms.iter_mut().for_each(&map_atom),
        StereoCarrierV2::Torsion(atoms) => atoms.iter_mut().for_each(&map_atom),
        StereoCarrierV2::ConjugatedDoubleBondPair(bonds) => bonds.iter_mut().for_each(&map_bond),
    };
    let remapped_stereo = stereo
        .iter()
        .cloned()
        .map(|mut element| {
            match &mut element {
                StereoElementV2::Tetrahedral {
                    id,
                    center,
                    references,
                    ..
                } => {
                    *id = stereo_ids[id].clone();
                    map_atom(center);
                    for reference in references {
                        if let StereoReferenceV2::Atom(atom) = reference {
                            map_atom(atom);
                        }
                    }
                }
                StereoElementV2::DoubleBond {
                    id,
                    bond,
                    left_reference,
                    right_reference,
                    ..
                } => {
                    *id = stereo_ids[id].clone();
                    map_bond(bond);
                    map_atom(left_reference);
                    map_atom(right_reference);
                }
                StereoElementV2::EnhancedGroup { id, members, .. } => {
                    *id = stereo_ids[id].clone();
                    for member in members {
                        *member = stereo_ids
                            .get(member)
                            .expect("clipboard enhanced stereo member was retained")
                            .clone();
                    }
                }
                StereoElementV2::Extended { id, carriers, .. }
                | StereoElementV2::Conformation { id, carriers, .. }
                | StereoElementV2::Unspecified { id, carriers, .. } => {
                    *id = stereo_ids[id].clone();
                    carriers.iter_mut().for_each(&map_carrier);
                }
            }
            element
        })
        .collect();
    let remapped_interactions = interactions
        .iter()
        .cloned()
        .map(|mut interaction| {
            interaction.id = engine.next_id("interaction");
            for atom in interaction
                .centers
                .iter_mut()
                .flat_map(|center| center.atoms.iter_mut())
            {
                map_atom(atom);
            }
            interaction
        })
        .collect();
    (remapped_stereo, remapped_interactions)
}

fn fragment_clipboard_bounds(nodes: &[Node]) -> [f64; 4] {
    let Some(first) = nodes.first() else {
        return [0.0, 0.0, 1.0, 1.0];
    };
    let mut min_x = first.position[0];
    let mut min_y = first.position[1];
    let mut max_x = first.position[0];
    let mut max_y = first.position[1];
    for node in nodes {
        min_x = min_x.min(node.position[0]);
        min_y = min_y.min(node.position[1]);
        max_x = max_x.max(node.position[0]);
        max_y = max_y.max(node.position[1]);
        if let Some(label) = &node.label {
            if let Some([x1, y1, x2, y2]) = label.bbox() {
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
            }
        }
    }
    [min_x, min_y, max_x.max(min_x + 1.0), max_y.max(min_y + 1.0)]
}

fn set_clipboard_selection_bounds_meta(document: &mut ChemSemaDocument, bounds: [f64; 4]) {
    if !document.document.meta.is_object() {
        document.document.meta = serde_json::json!({});
    }
    let Some(meta) = document.document.meta.as_object_mut() else {
        return;
    };
    let clipboard = meta
        .entry("clipboard")
        .or_insert_with(|| serde_json::json!({}));
    if !clipboard.is_object() {
        *clipboard = serde_json::json!({});
    }
    if let Some(clipboard) = clipboard.as_object_mut() {
        clipboard.insert(
            "selectionBounds".to_string(),
            serde_json::json!({
                "minX": bounds[0],
                "minY": bounds[1],
                "maxX": bounds[2],
                "maxY": bounds[3],
            }),
        );
    }
}
