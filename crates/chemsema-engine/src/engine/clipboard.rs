use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{EditorCommand, Engine, RenderBoundsScope};
use crate::{
    adjacent_directions, angle_between, hit_test_bond, hit_test_endpoint, Bond, BondAnchor,
    ChemSemaDocument, ChemicalProperty, ColoredMolecularArea, LinkRelation, Node, Point, Resource,
    ResourceData, SceneObject, SelectionState, BOND_HIT_RADIUS,
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
    colored_areas: Vec<ColoredMolecularArea>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    stereo: Vec<StereoElementV2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    interactions: Vec<MultiCenterInteractionV2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scene_objects: Vec<SceneObject>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    resources: BTreeMap<String, Resource>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    styles: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    links: Vec<LinkRelation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chemical_properties: Vec<ChemicalProperty>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    reaction_schemes: Vec<crate::ReactionSchemeData>,
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

    pub fn insert_document_template_json_at(
        &mut self,
        template_id: &str,
        json: &str,
        x: f64,
        y: f64,
    ) -> Result<bool, String> {
        let mut source = Engine::new();
        source.load_document_json(json)?;
        if !source.select_all() {
            return Err("template document has no selectable content".to_string());
        }
        let source_bond_length = source.options.bond_length_world_pt().value();
        let target_bond_length = self.options.bond_length_world_pt().value();
        if source_bond_length > crate::EPSILON {
            let scale_percent = target_bond_length / source_bond_length * 100.0;
            if (scale_percent - 100.0).abs() > crate::EPSILON {
                source.scale_selection(scale_percent);
            }
        }
        let target_point = Point::new(x, y);
        let target_endpoint = hit_test_endpoint(
            &self.state.document,
            target_point,
            self.endpoint_hit_radius(),
        )
        .filter(|endpoint| endpoint.distance <= self.endpoint_focus_radius());
        let source_node_anchor = optional_primary_template_node_anchor(&source.state.document)?;
        let target = if let (Some(endpoint), Some(source_anchor)) =
            (target_endpoint, source_node_anchor)
        {
            align_template_node_to_endpoint(
                &mut source,
                &self.state.document,
                &source_anchor,
                &endpoint,
            )?;
            DocumentTemplateTarget::Endpoint(endpoint)
        } else if let Some(hit) = hit_test_bond(&self.state.document, target_point, BOND_HIT_RADIUS)
        {
            if let Some(source_bond) =
                optional_primary_template_bond_anchor(&source.state.document)?
            {
                let target_bond = document_template_bond_anchor(&self.state.document, &hit.bond_id)
                    .ok_or_else(|| {
                        format!("template target bond '{}' was not found", hit.bond_id)
                    })?;
                align_template_bond_to_bond(&mut source, &source_bond, &target_bond)?;
                DocumentTemplateTarget::Bond(target_bond)
            } else {
                center_template_document_at(&mut source.state.document, target_point)?;
                DocumentTemplateTarget::Center
            }
        } else {
            center_template_document_at(&mut source.state.document, target_point)?;
            DocumentTemplateTarget::Center
        };
        let Some(document) = source.document_from_selection() else {
            return Ok(false);
        };
        let content = clipboard_content_from_document(document);
        let previous_clipboard = self.clipboard.replace(content);
        let changed = self.with_command(
            EditorCommand::InsertTemplate {
                template: template_id.to_string(),
                x,
                y,
                anchor: None,
                bond_id: match &target {
                    DocumentTemplateTarget::Bond(target) => Some(target.bond_id.clone()),
                    _ => None,
                },
                cursor: None,
                angle: None,
                bond_length: None,
                side: None,
            },
            |engine| {
                if !engine.paste_clipboard_untracked() {
                    return false;
                }
                match &target {
                    DocumentTemplateTarget::Center => true,
                    DocumentTemplateTarget::Endpoint(target) => {
                        engine.merge_pasted_template_node(target)
                    }
                    DocumentTemplateTarget::Bond(target) => {
                        engine.merge_pasted_template_bond(target)
                    }
                }
            },
        );
        self.clipboard = previous_clipboard;
        Ok(changed)
    }

    fn merge_pasted_template_node(&mut self, target: &crate::EndpointHit) -> bool {
        let pasted_object_id = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .filter(|entry| entry.object.id != target.object_id)
            .filter_map(|entry| {
                let distance = entry
                    .fragment
                    .nodes
                    .iter()
                    .map(|node| entry.world_point_for_node(node).distance(target.point))
                    .min_by(f64::total_cmp)?;
                Some((distance, entry.object.id.clone()))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .filter(|(distance, _)| *distance <= crate::EPSILON.max(0.02))
            .map(|(_, object_id)| object_id);
        let Some(pasted_object_id) = pasted_object_id else {
            return false;
        };
        let Some(pasted_entry) = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .find(|entry| entry.object.id == pasted_object_id)
        else {
            return false;
        };
        let pasted_translate = pasted_entry.object.transform.translate;
        let Some(pasted_anchor_id) = pasted_entry
            .fragment
            .nodes
            .iter()
            .min_by(|left, right| {
                pasted_entry
                    .world_point_for_node(left)
                    .distance(target.point)
                    .total_cmp(
                        &pasted_entry
                            .world_point_for_node(right)
                            .distance(target.point),
                    )
            })
            .map(|node| node.id.clone())
        else {
            return false;
        };
        self.finish_pasted_template_fragment_merge(
            &pasted_object_id,
            pasted_translate,
            pasted_entry.fragment.clone(),
            &target.object_id,
            &[(pasted_anchor_id.clone(), target.node_id.clone())],
            &[],
            &BTreeSet::from([pasted_anchor_id]),
            &BTreeSet::new(),
        )
    }

    fn merge_pasted_template_bond(&mut self, target: &DocumentTemplateBondAnchor) -> bool {
        let candidate = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .filter(|entry| entry.object.id != target.object_id)
            .filter_map(|entry| {
                entry
                    .fragment
                    .bonds
                    .iter()
                    .filter_map(|bond| {
                        let begin = entry
                            .fragment
                            .nodes
                            .iter()
                            .find(|node| node.id == bond.begin)
                            .map(|node| entry.world_point_for_node(node))?;
                        let end = entry
                            .fragment
                            .nodes
                            .iter()
                            .find(|node| node.id == bond.end)
                            .map(|node| entry.world_point_for_node(node))?;
                        let direct = begin.distance(target.begin) + end.distance(target.end);
                        Some((
                            direct,
                            entry.object.id.clone(),
                            entry.object.transform.translate,
                            entry.fragment.clone(),
                            bond.clone(),
                        ))
                    })
                    .min_by(|left, right| left.0.total_cmp(&right.0))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0));
        let Some((distance, pasted_object_id, pasted_translate, pasted_fragment, pasted_bond)) =
            candidate
        else {
            return false;
        };
        if distance > crate::EPSILON.max(0.04) {
            return false;
        }
        let pasted_bond_id = pasted_bond.id.clone();
        let pasted_begin_id = pasted_bond.begin.clone();
        let pasted_end_id = pasted_bond.end.clone();
        self.finish_pasted_template_fragment_merge(
            &pasted_object_id,
            pasted_translate,
            pasted_fragment,
            &target.object_id,
            &[
                (pasted_begin_id.clone(), target.begin_id.clone()),
                (pasted_end_id.clone(), target.end_id.clone()),
            ],
            &[(pasted_bond_id.clone(), target.bond_id.clone())],
            &BTreeSet::from([pasted_begin_id, pasted_end_id]),
            &BTreeSet::from([pasted_bond_id]),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_pasted_template_fragment_merge(
        &mut self,
        pasted_object_id: &str,
        pasted_translate: [f64; 2],
        mut fragment: crate::MoleculeFragment,
        target_object_id: &str,
        node_replacements: &[(String, String)],
        bond_replacements: &[(String, String)],
        removed_node_ids: &BTreeSet<String>,
        removed_bond_ids: &BTreeSet<String>,
    ) -> bool {
        fragment
            .nodes
            .retain(|node| !removed_node_ids.contains(&node.id));
        fragment
            .bonds
            .retain(|bond| !removed_bond_ids.contains(&bond.id));
        for bond in &mut fragment.bonds {
            for (source, target) in node_replacements {
                if bond.begin == *source {
                    bond.begin = target.clone();
                }
                if bond.end == *source {
                    bond.end = target.clone();
                }
            }
        }
        remap_fragment_semantics_for_template_fusion(
            &mut fragment,
            node_replacements,
            bond_replacements,
        );
        let target_translate = self
            .state
            .document
            .editable_fragments()
            .into_iter()
            .find(|entry| entry.object.id == target_object_id)
            .map(|entry| entry.object.transform.translate);
        let Some(target_translate) = target_translate else {
            return false;
        };
        for node in &mut fragment.nodes {
            node.position = [
                crate::round2(pasted_translate[0] + node.position[0] - target_translate[0]),
                crate::round2(pasted_translate[1] + node.position[1] - target_translate[1]),
            ];
        }
        let inserted_node_ids = fragment
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let inserted_bond_ids = fragment
            .bonds
            .iter()
            .map(|bond| bond.id.clone())
            .collect::<Vec<_>>();
        {
            let stroke_width = self.options.bond_stroke_world_pt().value();
            let Some(mut target_entry) = self
                .state
                .document
                .editable_fragment_mut_for_object(target_object_id)
            else {
                return false;
            };
            target_entry.fragment.nodes.extend(fragment.nodes);
            target_entry.fragment.bonds.extend(fragment.bonds);
            target_entry
                .fragment
                .colored_areas
                .extend(fragment.colored_areas);
            target_entry.fragment.stereo.extend(fragment.stereo);
            target_entry
                .fragment
                .interactions
                .extend(fragment.interactions);
            refresh_attached_node_label_geometry_for_all_nodes(
                target_entry.fragment,
                target_translate,
                stroke_width,
            );
            target_entry.update_bounds();
        }
        let removed_ids = BTreeSet::from([pasted_object_id]);
        self.state.document.remove_scene_objects_by_id(&removed_ids);
        let entity_replacements = node_replacements
            .iter()
            .chain(bond_replacements)
            .cloned()
            .chain(std::iter::once((
                pasted_object_id.to_string(),
                target_object_id.to_string(),
            )))
            .collect::<BTreeMap<_, _>>();
        remap_document_references_for_template_fusion(
            &mut self.state.document,
            &entity_replacements,
        );
        self.state.selection = SelectionState {
            molecule_objects: vec![target_object_id.to_string()],
            nodes: node_replacements
                .iter()
                .map(|(_, target)| target.clone())
                .chain(inserted_node_ids)
                .collect(),
            bonds: inserted_bond_ids,
            ..SelectionState::default()
        };
        true
    }

    fn paste_external_document(&mut self, mut source: Engine) -> Result<bool, String> {
        if !source.select_all() {
            return Ok(false);
        }
        let Some(document) = source.document_from_selection() else {
            return Ok(false);
        };
        let content = clipboard_content_from_document(document);
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
        let colored_areas_to_insert = content
            .colored_areas
            .iter()
            .filter_map(|area| {
                let basis_bonds = area
                    .basis_bonds
                    .iter()
                    .map(|id| bond_id_map.get(id).cloned())
                    .collect::<Option<Vec<_>>>()?;
                Some(ColoredMolecularArea {
                    id: self.next_id("colored_area"),
                    color: area.color.clone(),
                    basis_bonds,
                })
            })
            .collect::<Vec<_>>();

        if !nodes_to_insert.is_empty() {
            let stroke_width = self.options.bond_stroke_world_pt().value();
            let Some(mut entry) = self.state.document.editable_fragment_mut() else {
                self.undo_stack.pop();
                return false;
            };
            entry.fragment.nodes.extend(nodes_to_insert);
            entry.fragment.bonds.extend(bonds_to_insert);
            entry.fragment.colored_areas.extend(colored_areas_to_insert);
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
        let mut style_id_map = BTreeMap::new();
        for (source_id, style) in &content.styles {
            if let Some((target_id, _)) = self
                .state
                .document
                .styles
                .iter()
                .find(|(_, target)| *target == style)
            {
                style_id_map.insert(source_id.clone(), target_id.clone());
                continue;
            }
            let target_id = self.next_id("style");
            self.state
                .document
                .styles
                .insert(target_id.clone(), style.clone());
            style_id_map.insert(source_id.clone(), target_id);
        }
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
            preallocate_clipboard_scene_ids(self, source, &mut entity_id_map);
        }
        for scheme in &content.reaction_schemes {
            for step in &scheme.steps {
                entity_id_map.insert(step.id.clone(), self.next_id("reaction_step"));
            }
        }
        for source in &content.scene_objects {
            let mut object = super::select::drag::translated_scene_object(
                source,
                CLIPBOARD_PASTE_OFFSET_PT,
                CLIPBOARD_PASTE_OFFSET_PT,
            );
            remap_clipboard_scene_object(
                &mut object,
                &resource_id_map,
                &style_id_map,
                &entity_id_map,
            );
            if object.object_type == "molecule" {
                pasted_molecule_ids.push(object.id.clone());
            } else {
                pasted_scene_ids.push(object.id.clone());
            }
            self.state.document.objects.push(object);
        }
        for scheme in &content.reaction_schemes {
            let mut scheme = scheme.clone();
            scheme.id = self.next_id("reaction_scheme");
            scheme.steps = scheme
                .steps
                .into_iter()
                .filter_map(|mut step| {
                    step.id = entity_id_map.get(&step.id)?.clone();
                    remap_reaction_step(&mut step, &entity_id_map).then_some(step)
                })
                .collect();
            if !scheme.steps.is_empty() {
                self.state.document.reaction_schemes.push(scheme);
            }
        }
        for object_id in &pasted_scene_ids {
            reconcile_pasted_stoichiometry_grid(
                &mut self.state.document,
                object_id,
                &entity_id_map,
            );
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
        let colored_areas = entry
            .as_ref()
            .filter(|_| !active_molecule_is_complete)
            .map(|entry| {
                entry
                    .fragment
                    .colored_areas
                    .iter()
                    .filter(|area| area.basis_bonds.iter().all(|id| bond_ids.contains(id)))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let mut selected_scene_ids: BTreeSet<&str> = self
            .state
            .selection
            .arrow_objects
            .iter()
            .map(String::as_str)
            .collect();
        for object_id in &self.state.selection.arrow_objects {
            let Some(table) = self
                .state
                .document
                .find_scene_object(object_id)
                .and_then(|object| object.payload.table.as_ref())
            else {
                continue;
            };
            selected_scene_ids.extend(
                table
                    .cells
                    .iter()
                    .flat_map(|cell| cell.content_object_ids.iter().map(String::as_str)),
            );
        }
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
        retain_copyable_annotations(&mut scene_objects, &selected_entity_ids);
        selected_entity_ids = scene_objects
            .iter()
            .flat_map(scene_object_ids)
            .chain(selected_entity_ids)
            .collect();
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
        let reaction_schemes =
            selected_reaction_schemes(&self.state.document, &selected_entity_ids);

        Some(ClipboardContent {
            nodes,
            bonds,
            colored_areas,
            stereo,
            interactions,
            scene_objects,
            resources,
            styles: self.state.document.styles.clone(),
            links,
            chemical_properties,
            reaction_schemes,
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
        for object_id in &self.state.selection.arrow_objects {
            let Some(table) = self
                .state
                .document
                .find_scene_object(object_id)
                .and_then(|object| object.payload.table.as_ref())
            else {
                continue;
            };
            selected_object_ids.extend(
                table
                    .cells
                    .iter()
                    .flat_map(|cell| cell.content_object_ids.iter().cloned()),
            );
        }

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
        retain_copyable_annotations(&mut document.objects, &retained);
        retained = document
            .scene_objects()
            .into_iter()
            .map(|object| object.id.clone())
            .collect();
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
        document.reaction_schemes = selected_reaction_schemes(&document, &retained);
        freeze_unbound_stoichiometry_grids(&mut document);
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
        fragment.colored_areas.retain(|area| {
            area.basis_bonds
                .iter()
                .all(|bond_id| bond_ids.contains(bond_id))
        });
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
        | crate::SceneObjectKind::Table
        | crate::SceneObjectKind::StoichiometryGrid
        | crate::SceneObjectKind::Image
        | crate::SceneObjectKind::Spectrum
        | crate::SceneObjectKind::Geometry
        | crate::SceneObjectKind::Constraint
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

enum DocumentTemplateTarget {
    Center,
    Endpoint(crate::EndpointHit),
    Bond(DocumentTemplateBondAnchor),
}

#[derive(Clone)]
struct DocumentTemplateBondAnchor {
    object_id: String,
    bond_id: String,
    begin_id: String,
    end_id: String,
    begin: Point,
    end: Point,
}

struct TemplateNodeAnchor {
    node_id: String,
    direction: Option<f64>,
}

struct TemplateBondAnchor {
    bond_id: String,
    begin_id: String,
    end_id: String,
    begin: Point,
    end: Point,
}

fn optional_primary_template_node_anchor(
    document: &ChemSemaDocument,
) -> Result<Option<TemplateNodeAnchor>, String> {
    let Some(entry) = document.editable_fragments().into_iter().next() else {
        return Ok(None);
    };
    let node = entry
        .fragment
        .nodes
        .first()
        .ok_or_else(|| "template molecular fragment has no attachment node".to_string())?;
    Ok(Some(TemplateNodeAnchor {
        node_id: node.id.clone(),
        direction: adjacent_directions(&entry, &node.id).into_iter().next(),
    }))
}

fn optional_primary_template_bond_anchor(
    document: &ChemSemaDocument,
) -> Result<Option<TemplateBondAnchor>, String> {
    let Some(entry) = document.editable_fragments().into_iter().next() else {
        return Ok(None);
    };
    let Some(bond) = entry.fragment.bonds.first() else {
        return Ok(None);
    };
    let begin = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.begin)
        .map(|node| entry.world_point_for_node(node))
        .ok_or_else(|| "template fusion bond begin node was not found".to_string())?;
    let end = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == bond.end)
        .map(|node| entry.world_point_for_node(node))
        .ok_or_else(|| "template fusion bond end node was not found".to_string())?;
    Ok(Some(TemplateBondAnchor {
        bond_id: bond.id.clone(),
        begin_id: bond.begin.clone(),
        end_id: bond.end.clone(),
        begin,
        end,
    }))
}

fn document_template_bond_anchor(
    document: &ChemSemaDocument,
    bond_id: &str,
) -> Option<DocumentTemplateBondAnchor> {
    document.editable_fragments().into_iter().find_map(|entry| {
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)?;
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
            .map(|node| entry.world_point_for_node(node))?;
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)
            .map(|node| entry.world_point_for_node(node))?;
        Some(DocumentTemplateBondAnchor {
            object_id: entry.object.id.clone(),
            bond_id: bond.id.clone(),
            begin_id: bond.begin.clone(),
            end_id: bond.end.clone(),
            begin,
            end,
        })
    })
}

fn align_template_node_to_endpoint(
    source: &mut Engine,
    target_document: &ChemSemaDocument,
    source_anchor: &TemplateNodeAnchor,
    target: &crate::EndpointHit,
) -> Result<(), String> {
    let target_anchor = BondAnchor {
        node_id: Some(target.node_id.clone()),
        object_id: Some(target.object_id.clone()),
        point: target.point,
        label_anchor: target.label_anchor.clone(),
    };
    let target_direction = molecular_template_attachment_direction(target_document, &target_anchor);
    if let Some(source_direction) = source_anchor.direction {
        source
            .rotate_selection_degrees(crate::normalize_angle(target_direction - source_direction));
    }
    let next_anchor = template_node_world_point(&source.state.document, &source_anchor.node_id)
        .ok_or_else(|| {
            "template primary attachment node disappeared during rotation".to_string()
        })?;
    translate_document_objects(
        &mut source.state.document,
        target.point.x - next_anchor.x - CLIPBOARD_PASTE_OFFSET_PT,
        target.point.y - next_anchor.y - CLIPBOARD_PASTE_OFFSET_PT,
    );
    Ok(())
}

fn align_template_bond_to_bond(
    source: &mut Engine,
    source_bond: &TemplateBondAnchor,
    target: &DocumentTemplateBondAnchor,
) -> Result<(), String> {
    let source_length = source_bond.begin.distance(source_bond.end);
    let target_length = target.begin.distance(target.end);
    if source_length <= crate::EPSILON || target_length <= crate::EPSILON {
        return Err("template fusion bond must have nonzero length".to_string());
    }
    let scale_percent = target_length / source_length * 100.0;
    if (scale_percent - 100.0).abs() > crate::EPSILON {
        source.scale_selection(scale_percent);
    }
    let scaled = template_bond_world_points(
        &source.state.document,
        &source_bond.begin_id,
        &source_bond.end_id,
    )
    .ok_or_else(|| {
        format!(
            "template fusion bond '{}' disappeared during scaling",
            source_bond.bond_id
        )
    })?;
    source.rotate_selection_degrees(crate::normalize_angle(
        angle_between(target.begin, target.end) - angle_between(scaled.0, scaled.1),
    ));
    let rotated = template_bond_world_points(
        &source.state.document,
        &source_bond.begin_id,
        &source_bond.end_id,
    )
    .ok_or_else(|| {
        format!(
            "template fusion bond '{}' disappeared during rotation",
            source_bond.bond_id
        )
    })?;
    translate_document_objects(
        &mut source.state.document,
        target.begin.x - rotated.0.x - CLIPBOARD_PASTE_OFFSET_PT,
        target.begin.y - rotated.0.y - CLIPBOARD_PASTE_OFFSET_PT,
    );
    Ok(())
}

fn center_template_document_at(
    document: &mut ChemSemaDocument,
    target: Point,
) -> Result<(), String> {
    let primitives = crate::render_document(document);
    let [min_x, min_y, max_x, max_y] = crate::render_primitives_bounds(primitives.iter())
        .ok_or_else(|| "template document has no visible content".to_string())?;
    translate_document_objects(
        document,
        target.x - (min_x + max_x) * 0.5 - CLIPBOARD_PASTE_OFFSET_PT,
        target.y - (min_y + max_y) * 0.5 - CLIPBOARD_PASTE_OFFSET_PT,
    );
    Ok(())
}

fn template_node_world_point(document: &ChemSemaDocument, node_id: &str) -> Option<Point> {
    document.editable_fragments().into_iter().find_map(|entry| {
        entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .map(|node| entry.world_point_for_node(node))
    })
}

fn template_bond_world_points(
    document: &ChemSemaDocument,
    begin_id: &str,
    end_id: &str,
) -> Option<(Point, Point)> {
    Some((
        template_node_world_point(document, begin_id)?,
        template_node_world_point(document, end_id)?,
    ))
}

fn molecular_template_attachment_direction(
    document: &ChemSemaDocument,
    anchor: &BondAnchor,
) -> f64 {
    let Some(node_id) = anchor.node_id.as_deref() else {
        return crate::default_angle_for_anchor(document, anchor);
    };
    let directions = document
        .editable_fragments()
        .into_iter()
        .find(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
        .map(|entry| adjacent_directions(&entry, node_id))
        .unwrap_or_default();
    match directions.as_slice() {
        [direction] => crate::normalize_angle(direction + 180.0),
        _ => crate::default_angle_for_anchor(document, anchor),
    }
}

fn translate_document_objects(document: &mut ChemSemaDocument, delta_x: f64, delta_y: f64) {
    document.objects = document
        .objects
        .iter()
        .map(|object| super::select::drag::translated_scene_object(object, delta_x, delta_y))
        .collect();
}

fn clipboard_content_from_document(document: ChemSemaDocument) -> ClipboardContent {
    let mut resources = BTreeMap::new();
    for object in &document.objects {
        collect_scene_object_resources(object, &document.resources, &mut resources);
    }
    ClipboardContent {
        nodes: Vec::new(),
        bonds: Vec::new(),
        colored_areas: Vec::new(),
        stereo: Vec::new(),
        interactions: Vec::new(),
        scene_objects: document.objects,
        resources,
        styles: document.styles,
        links: document.links,
        chemical_properties: document.chemical_properties,
        reaction_schemes: document.reaction_schemes,
    }
}

fn preallocate_clipboard_scene_ids(
    engine: &mut Engine,
    object: &SceneObject,
    entity_id_map: &mut BTreeMap<String, String>,
) {
    entity_id_map.insert(object.id.clone(), engine.next_id("object"));
    for child in &object.children {
        preallocate_clipboard_scene_ids(engine, child, entity_id_map);
    }
}

fn remap_clipboard_scene_object(
    object: &mut SceneObject,
    resource_id_map: &BTreeMap<String, String>,
    style_id_map: &BTreeMap<String, String>,
    entity_id_map: &BTreeMap<String, String>,
) {
    object.id = entity_id_map
        .get(&object.id)
        .expect("clipboard scene id was preallocated")
        .clone();
    if let Some(resource_id) = object.payload.resource_ref.as_mut() {
        if let Some(target_id) = resource_id_map.get(resource_id) {
            *resource_id = target_id.clone();
        }
    }
    if let Some(style_id) = object.style_ref.as_mut() {
        if let Some(target_id) = style_id_map.get(style_id) {
            *style_id = target_id.clone();
        }
    }
    if let Some(geometry) = object.payload.geometry.as_mut() {
        remap_annotation_ids(
            &mut geometry.basis_entity_ids,
            &mut geometry.unresolved_basis_ids,
            entity_id_map,
        );
    }
    if let Some(constraint) = object.payload.constraint.as_mut() {
        remap_annotation_ids(
            &mut constraint.basis_entity_ids,
            &mut constraint.unresolved_basis_ids,
            entity_id_map,
        );
    }
    if let Some(table) = object.payload.table.as_mut() {
        for (index, cell) in table.cells.iter_mut().enumerate() {
            cell.id = format!("{}_cell_{}_{}_{}", object.id, cell.row, cell.column, index);
            cell.content_object_ids = cell
                .content_object_ids
                .iter()
                .filter_map(|id| entity_id_map.get(id).cloned())
                .collect();
        }
    }
    if let Some(grid) = object.payload.stoichiometry_grid.as_mut() {
        for component in &mut grid.components {
            component.reference_entity_id = component
                .reference_entity_id
                .as_ref()
                .and_then(|id| entity_id_map.get(id))
                .cloned();
        }
        grid.source_reaction_step_id = grid
            .source_reaction_step_id
            .as_ref()
            .and_then(|id| entity_id_map.get(id))
            .cloned();
        if grid.source_reaction_step_id.is_none() {
            grid.binding_state = crate::StoichiometryBindingState::Detached;
            object.link_policy = crate::LinkPolicy::Unlinked;
            for datum in &mut grid.data {
                if datum.origin == crate::StoichiometryValueOrigin::Calculated {
                    datum.origin = crate::StoichiometryValueOrigin::Imported;
                }
            }
        }
    }
    for child in &mut object.children {
        remap_clipboard_scene_object(child, resource_id_map, style_id_map, entity_id_map);
    }
}

fn selected_reaction_schemes(
    document: &ChemSemaDocument,
    selected_entity_ids: &BTreeSet<String>,
) -> Vec<crate::ReactionSchemeData> {
    document
        .reaction_schemes
        .iter()
        .filter_map(|scheme| {
            let steps = scheme
                .steps
                .iter()
                .filter(|step| {
                    reaction_step_entity_ids(step).all(|id| selected_entity_ids.contains(id))
                })
                .cloned()
                .collect::<Vec<_>>();
            (!steps.is_empty()).then(|| crate::ReactionSchemeData {
                id: scheme.id.clone(),
                steps,
            })
        })
        .collect()
}

fn reaction_step_entity_ids(step: &crate::ReactionStepData) -> impl Iterator<Item = &String> {
    step.reactant_entity_ids
        .iter()
        .chain(step.product_entity_ids.iter())
        .chain(step.arrow_object_ids.iter())
        .chain(step.plus_object_ids.iter())
        .chain(step.objects_above_arrow.iter())
        .chain(step.objects_below_arrow.iter())
}

fn remap_reaction_step(
    step: &mut crate::ReactionStepData,
    entity_id_map: &BTreeMap<String, String>,
) -> bool {
    let remap_ids = |ids: &mut Vec<String>| {
        *ids = ids
            .iter()
            .filter_map(|id| entity_id_map.get(id).cloned())
            .collect();
    };
    remap_ids(&mut step.reactant_entity_ids);
    remap_ids(&mut step.product_entity_ids);
    remap_ids(&mut step.arrow_object_ids);
    remap_ids(&mut step.plus_object_ids);
    remap_ids(&mut step.objects_above_arrow);
    remap_ids(&mut step.objects_below_arrow);
    step.atom_mappings = step
        .atom_mappings
        .iter()
        .filter_map(|mapping| {
            Some(crate::ReactionAtomMapping {
                reactant_atom_id: entity_id_map.get(&mapping.reactant_atom_id)?.clone(),
                product_atom_id: entity_id_map.get(&mapping.product_atom_id)?.clone(),
                origin: mapping.origin,
            })
        })
        .collect();
    !step.reactant_entity_ids.is_empty()
        && !step.product_entity_ids.is_empty()
        && !step.arrow_object_ids.is_empty()
}

fn reconcile_pasted_stoichiometry_grid(
    document: &mut ChemSemaDocument,
    object_id: &str,
    entity_id_map: &BTreeMap<String, String>,
) {
    let valid_steps = document
        .reaction_schemes
        .iter()
        .flat_map(|scheme| scheme.steps.iter())
        .map(|step| step.id.clone())
        .collect::<BTreeSet<_>>();
    let Some(object) = document.find_scene_object_mut(object_id) else {
        return;
    };
    let Some(grid) = object.payload.stoichiometry_grid.as_mut() else {
        return;
    };
    if let Some(source_id) = grid.source_reaction_step_id.clone() {
        if !valid_steps.contains(&source_id) {
            grid.source_reaction_step_id = entity_id_map.get(&source_id).cloned();
        }
    }
    if grid
        .source_reaction_step_id
        .as_deref()
        .is_none_or(|id| !valid_steps.contains(id))
    {
        grid.source_reaction_step_id = None;
        grid.binding_state = crate::StoichiometryBindingState::Detached;
        object.link_policy = crate::LinkPolicy::Unlinked;
        for datum in &mut grid.data {
            if datum.origin == crate::StoichiometryValueOrigin::Calculated {
                datum.origin = crate::StoichiometryValueOrigin::Imported;
            }
        }
    }
}

fn freeze_unbound_stoichiometry_grids(document: &mut ChemSemaDocument) {
    let valid_steps = document
        .reaction_schemes
        .iter()
        .flat_map(|scheme| scheme.steps.iter())
        .map(|step| step.id.clone())
        .collect::<BTreeSet<_>>();
    freeze_unbound_stoichiometry_scene_objects(&mut document.objects, &valid_steps);
}

fn freeze_unbound_stoichiometry_scene_objects(
    objects: &mut [SceneObject],
    valid_steps: &BTreeSet<String>,
) {
    for object in objects {
        let Some(grid) = object.payload.stoichiometry_grid.as_mut() else {
            freeze_unbound_stoichiometry_scene_objects(&mut object.children, valid_steps);
            continue;
        };
        let binding_valid = grid
            .source_reaction_step_id
            .as_ref()
            .is_some_and(|id| valid_steps.contains(id));
        if !binding_valid {
            grid.source_reaction_step_id = None;
            grid.binding_state = crate::StoichiometryBindingState::Detached;
            object.link_policy = crate::LinkPolicy::Unlinked;
            for datum in &mut grid.data {
                if datum.origin == crate::StoichiometryValueOrigin::Calculated {
                    datum.origin = crate::StoichiometryValueOrigin::Imported;
                }
            }
        }
        freeze_unbound_stoichiometry_scene_objects(&mut object.children, valid_steps);
    }
}

fn remap_annotation_ids(
    basis_entity_ids: &mut Vec<String>,
    unresolved_basis_ids: &mut Vec<String>,
    entity_id_map: &BTreeMap<String, String>,
) {
    *basis_entity_ids = basis_entity_ids
        .iter()
        .filter_map(|id| entity_id_map.get(id).cloned())
        .collect();
    unresolved_basis_ids.clear();
}

fn annotation_basis_ids(object: &SceneObject) -> &[String] {
    object
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
        })
        .unwrap_or(&[])
}

fn retain_copyable_annotations(
    objects: &mut Vec<SceneObject>,
    selected_entity_ids: &BTreeSet<String>,
) {
    objects.retain_mut(|object| {
        retain_copyable_annotations(&mut object.children, selected_entity_ids);
        if matches!(
            object.kind(),
            crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
        ) {
            return annotation_basis_ids(object)
                .iter()
                .all(|basis_id| selected_entity_ids.contains(basis_id));
        }
        true
    });
}

fn scene_object_ids(object: &SceneObject) -> Vec<String> {
    let mut ids = vec![object.id.clone()];
    for child in &object.children {
        ids.extend(scene_object_ids(child));
    }
    ids
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
    for area in &mut fragment.colored_areas {
        area.id = engine.next_id("colored_area");
        area.basis_bonds = area
            .basis_bonds
            .iter()
            .map(|id| {
                bond_ids
                    .get(id)
                    .expect("clipboard colored area bond was retained")
                    .clone()
            })
            .collect();
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

fn remap_fragment_semantics_for_template_fusion(
    fragment: &mut crate::MoleculeFragment,
    node_replacements: &[(String, String)],
    bond_replacements: &[(String, String)],
) {
    use chemsema_chemical_graph::{StereoCarrierV2, StereoReferenceV2};

    let node_map = node_replacements
        .iter()
        .cloned()
        .collect::<BTreeMap<_, _>>();
    let bond_map = bond_replacements
        .iter()
        .cloned()
        .collect::<BTreeMap<_, _>>();
    let replace_node = |id: &mut String| {
        if let Some(next) = node_map.get(id) {
            *id = next.clone();
        }
    };
    let replace_bond = |id: &mut String| {
        if let Some(next) = bond_map.get(id) {
            *id = next.clone();
        }
    };
    let replace_carrier = |carrier: &mut StereoCarrierV2| match carrier {
        StereoCarrierV2::Atom(atom)
        | StereoCarrierV2::LonePair(atom)
        | StereoCarrierV2::DuplicateAtom(atom) => replace_node(atom),
        StereoCarrierV2::Bond(bond) => replace_bond(bond),
        StereoCarrierV2::AtomSet(atoms) | StereoCarrierV2::Plane(atoms) => {
            atoms.iter_mut().for_each(&replace_node)
        }
        StereoCarrierV2::Axis(atoms) => atoms.iter_mut().for_each(&replace_node),
        StereoCarrierV2::Torsion(atoms) => atoms.iter_mut().for_each(&replace_node),
        StereoCarrierV2::ConjugatedDoubleBondPair(bonds) => {
            bonds.iter_mut().for_each(&replace_bond)
        }
    };

    for element in &mut fragment.stereo {
        match element {
            StereoElementV2::Tetrahedral {
                center, references, ..
            } => {
                replace_node(center);
                for reference in references {
                    if let StereoReferenceV2::Atom(atom) = reference {
                        replace_node(atom);
                    }
                }
            }
            StereoElementV2::DoubleBond {
                bond,
                left_reference,
                right_reference,
                ..
            } => {
                replace_bond(bond);
                replace_node(left_reference);
                replace_node(right_reference);
            }
            StereoElementV2::EnhancedGroup { .. } => {}
            StereoElementV2::Extended { carriers, .. }
            | StereoElementV2::Conformation { carriers, .. }
            | StereoElementV2::Unspecified { carriers, .. } => {
                carriers.iter_mut().for_each(&replace_carrier);
            }
        }
    }
    for interaction in &mut fragment.interactions {
        for atom in interaction
            .centers
            .iter_mut()
            .flat_map(|center| center.atoms.iter_mut())
        {
            replace_node(atom);
        }
    }
    for area in &mut fragment.colored_areas {
        for bond in &mut area.basis_bonds {
            replace_bond(bond);
        }
        area.basis_bonds.dedup();
    }
}

fn remap_document_references_for_template_fusion(
    document: &mut ChemSemaDocument,
    replacements: &BTreeMap<String, String>,
) {
    let replace = |id: &mut String| {
        if let Some(next) = replacements.get(id) {
            *id = next.clone();
        }
    };
    for relation in &mut document.links {
        for endpoint in &mut relation.endpoints {
            replace(&mut endpoint.entity_id);
        }
        remap_exact_entity_strings(&mut relation.data, replacements);
    }
    for property in &mut document.chemical_properties {
        for id in &mut property.basis_entity_ids {
            replace(id);
        }
        if let Some(id) = &mut property.display_object_id {
            replace(id);
        }
        for id in &mut property.unresolved_basis_ids {
            replace(id);
        }
    }
    for scheme in &mut document.reaction_schemes {
        for step in &mut scheme.steps {
            for id in step
                .reactant_entity_ids
                .iter_mut()
                .chain(step.product_entity_ids.iter_mut())
                .chain(step.arrow_object_ids.iter_mut())
                .chain(step.plus_object_ids.iter_mut())
                .chain(step.objects_above_arrow.iter_mut())
                .chain(step.objects_below_arrow.iter_mut())
            {
                replace(id);
            }
            for mapping in &mut step.atom_mappings {
                replace(&mut mapping.reactant_atom_id);
                replace(&mut mapping.product_atom_id);
            }
        }
    }
    for object in &mut document.objects {
        remap_scene_object_payload_references(object, replacements);
    }
}

fn remap_scene_object_payload_references(
    object: &mut SceneObject,
    replacements: &BTreeMap<String, String>,
) {
    let mut payload = serde_json::to_value(&object.payload)
        .expect("scene object payload must remain serializable");
    remap_exact_entity_strings(&mut payload, replacements);
    object.payload =
        serde_json::from_value(payload).expect("remapped scene object payload must remain valid");
    for child in &mut object.children {
        remap_scene_object_payload_references(child, replacements);
    }
}

fn remap_exact_entity_strings(
    value: &mut serde_json::Value,
    replacements: &BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::String(text) => {
            if let Some(next) = replacements.get(text) {
                *text = next.clone();
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                remap_exact_entity_strings(value, replacements);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                remap_exact_entity_strings(value, replacements);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
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
