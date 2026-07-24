use super::{ChemicalAnalysisFormat, CommandTargetSet, Engine};
use crate::{AtomRadical, Bond, MoleculeFragment, SceneObject};
use chemsema_chemical_graph::{
    AtomV2, BondKindV2, BondV2, ChemicalGraphV2, ComponentV2, DativeDirectionV2,
    DoubleBondRelationV2, GraphAssumptionV2, GraphSemanticsV2, NomenclatureRequestV1,
    RadicalStateV2, StereoElementV2, StereoReferenceV2, TetrahedralParityV2,
    CHEMICAL_GRAPH_V2_SCHEMA,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    pub(super) fn selected_single_molecule_fragment(
        &self,
    ) -> Result<(&SceneObject, MoleculeFragment), String> {
        self.complete_molecule_fragment_for_targets(&CommandTargetSet::default())
    }

    pub fn chemical_graph_v2_json(&self) -> Result<String, String> {
        let (_, graph, _) = self.chemical_graph_v2_for_targets(&CommandTargetSet::default())?;
        serde_json::to_string(&graph.normalized()?).map_err(|error| error.to_string())
    }

    pub fn nomenclature_request_json(&self) -> Result<String, String> {
        let (object, graph, _) = self.chemical_graph_v2_for_targets(&CommandTargetSet::default())?;
        let request =
            NomenclatureRequestV1::new_preferred_iupac_name(&object.id, graph.normalized()?)?;
        serde_json::to_string(&request).map_err(|error| error.to_string())
    }

    pub(super) fn chemical_graph_v2_for_targets(
        &self,
        targets: &CommandTargetSet,
    ) -> Result<(&SceneObject, ChemicalGraphV2, MoleculeFragment), String> {
        let (object, fragment) = self.complete_molecule_fragment_for_targets(targets)?;
        let analysis = self.chemical_analysis_output(ChemicalAnalysisFormat::Smiles, targets)?;
        let graph = graph_from_fragment(&fragment, &analysis)?;
        Ok((object, graph, fragment))
    }

    pub(super) fn complete_molecule_fragment_for_targets(
        &self,
        targets: &CommandTargetSet,
    ) -> Result<(&SceneObject, MoleculeFragment), String> {
        let selected_object_ids = if targets.objects.is_empty() {
            self.state
                .selection
                .molecule_objects
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        } else {
            targets
                .objects
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        };
        let selected_node_ids = if targets.nodes.is_empty() {
            self.state
                .selection
                .nodes
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        } else {
            targets
                .nodes
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        };
        let selected_bond_ids = if targets.bonds.is_empty() {
            self.state
                .selection
                .bonds
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        } else {
            targets
                .bonds
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        };
        let mut candidates = Vec::new();
        for entry in self.state.document.editable_fragments() {
            let object_selected = selected_object_ids.contains(entry.object.id.as_str());
            let nodes = if object_selected {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .map(|node| node.id.as_str())
                    .collect::<BTreeSet<_>>()
            } else {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .filter(|node| selected_node_ids.contains(node.id.as_str()))
                    .map(|node| node.id.as_str())
                    .collect::<BTreeSet<_>>()
            };
            if nodes.is_empty()
                || entry.fragment.bonds.iter().any(|bond| {
                    nodes.contains(bond.begin.as_str()) != nodes.contains(bond.end.as_str())
                })
            {
                continue;
            }
            let bonds = entry
                .fragment
                .bonds
                .iter()
                .filter(|bond| {
                    nodes.contains(bond.begin.as_str()) && nodes.contains(bond.end.as_str())
                })
                .collect::<Vec<_>>();
            if !object_selected
                && !selected_bond_ids.is_empty()
                && bonds
                    .iter()
                    .any(|bond| !selected_bond_ids.contains(bond.id.as_str()))
            {
                continue;
            }
            let fragment = MoleculeFragment {
                schema: entry.fragment.schema.clone(),
                bbox: entry.fragment.bbox,
                nodes: entry
                    .fragment
                    .nodes
                    .iter()
                    .filter(|node| nodes.contains(node.id.as_str()))
                    .cloned()
                    .collect(),
                bonds: bonds.into_iter().cloned().collect(),
                meta: entry.fragment.meta.clone(),
            };
            if crate::molecule_fragment_connected_components(&fragment).len() == 1 {
                candidates.push((entry.object, fragment));
            }
        }
        if candidates.len() != 1 {
            return Err("select exactly one complete, connected molecule".to_string());
        }
        Ok(candidates.remove(0))
    }
}

fn graph_from_fragment(
    fragment: &MoleculeFragment,
    analysis: &Value,
) -> Result<ChemicalGraphV2, String> {
    let atoms = fragment
        .nodes
        .iter()
        .map(|node| {
            if node.is_placeholder
                || node.external_connection.is_some()
                || !node.atom_properties.element_list.is_empty()
                || !node.atom_properties.generic_list.is_empty()
            {
                return Err(format!(
                    "atom '{}' is a query, pseudo, or external-connection atom; the determined-molecule V2 profile cannot represent it",
                    node.id
                ));
            }
            let isotope = node
                .atom_properties
                .isotope_mass
                .filter(|value| *value > 0)
                .map(|value| value as u16);
            let formal_charge = i16::try_from(node.charge)
                .map_err(|_| format!("atom '{}' charge is outside ChemicalGraphV2", node.id))?;
            Ok(AtomV2 {
                id: node.id.clone(),
                atomic_number: node.atomic_number,
                isotope,
                formal_charge,
                radical: match node.atom_properties.radical {
                    AtomRadical::None => RadicalStateV2::None,
                    AtomRadical::Singlet => RadicalStateV2::Singlet,
                    AtomRadical::Doublet => RadicalStateV2::Doublet,
                    AtomRadical::Triplet => RadicalStateV2::Triplet,
                },
                implicit_hydrogens: crate::engine::formula_hydrogen_count_for_node(
                    fragment,
                    &node.id,
                ),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let bonds = fragment
        .bonds
        .iter()
        .map(bond_v2)
        .collect::<Result<Vec<_>, String>>()?;
    let mut stereo = tetrahedral_stereo(fragment, analysis)?;
    stereo.extend(double_bond_stereo(fragment, analysis)?);
    let components = molecule_components(fragment)
        .into_iter()
        .enumerate()
        .map(|(index, atoms)| ComponentV2 {
            id: format!("component-{}", index + 1),
            atoms,
            count: 1,
        })
        .collect::<Vec<_>>();
    let graph = ChemicalGraphV2 {
        schema: CHEMICAL_GRAPH_V2_SCHEMA.to_string(),
        semantics: GraphSemanticsV2::default(),
        atoms,
        bonds,
        stereo,
        components,
        assumptions: vec![GraphAssumptionV2 {
            code: "implicit-hydrogens-resolved".to_string(),
            detail: Some(
                "Resolved by the ChemSema chemistry layer before semantic export.".to_string(),
            ),
        }],
        interactions: Vec::new(),
    };
    graph.validate()?;
    Ok(graph)
}

fn bond_v2(bond: &Bond) -> Result<BondV2, String> {
    if !bond.properties.query_orders.is_empty()
        || bond.properties.topology != crate::BondTopology::Unspecified
    {
        return Err(format!(
            "bond '{}' contains query semantics outside the determined-molecule V2 profile",
            bond.id
        ));
    }
    let aromatic = bond
        .meta
        .pointer("/chemistry/smiles/kind")
        .and_then(Value::as_str)
        == Some("aromatic");
    let dative_donor = bond
        .meta
        .pointer("/chemistry/smiles/dativeDonorNode")
        .and_then(Value::as_str);
    let kind = if dative_donor.is_some()
        || bond
            .meta
            .get("cdxml")
            .and_then(|value| value.get("order"))
            .and_then(Value::as_str)
            .is_some_and(|value| value.eq_ignore_ascii_case("dative"))
    {
        BondKindV2::Dative
    } else if aromatic {
        BondKindV2::Aromatic
    } else {
        match bond.order {
            1 => BondKindV2::Single,
            2 => BondKindV2::Double,
            3 => BondKindV2::Triple,
            4 => BondKindV2::Quadruple,
            order => {
                return Err(format!(
                    "bond '{}' has unsupported semantic order {}",
                    bond.id, order
                ))
            }
        }
    };
    let dative_direction = if kind == BondKindV2::Dative {
        let donor = dative_donor.unwrap_or(bond.begin.as_str()).to_string();
        let acceptor = if donor == bond.begin {
            bond.end.clone()
        } else if donor == bond.end {
            bond.begin.clone()
        } else {
            return Err(format!(
                "dative bond '{}' donor is not an endpoint",
                bond.id
            ));
        };
        Some(DativeDirectionV2 { donor, acceptor })
    } else {
        None
    };
    Ok(BondV2 {
        id: bond.id.clone(),
        atoms: [bond.begin.clone(), bond.end.clone()],
        kind,
        dative_direction,
    })
}

fn tetrahedral_stereo(
    fragment: &MoleculeFragment,
    analysis: &Value,
) -> Result<Vec<StereoElementV2>, String> {
    analysis
        .get("tetrahedralCenters")
        .and_then(Value::as_array)
        .ok_or_else(|| "chemistry analysis omitted tetrahedralCenters".to_string())?
        .iter()
        .map(|center| {
            let atom_index = center
                .get("atomIndex")
                .and_then(Value::as_u64)
                .ok_or_else(|| "tetrahedral center omitted atomIndex".to_string())?
                as usize;
            let center_node = fragment
                .nodes
                .get(atom_index)
                .ok_or_else(|| "tetrahedral center atomIndex is outside fragment".to_string())?;
            let references = center
                .get("ligandOrder")
                .and_then(Value::as_array)
                .ok_or_else(|| "tetrahedral center omitted ligandOrder".to_string())?
                .iter()
                .map(|ligand| match ligand.get("kind").and_then(Value::as_str) {
                    Some("atom") => {
                        let index = ligand
                            .get("value")
                            .and_then(Value::as_u64)
                            .ok_or_else(|| "atom ligand omitted value".to_string())?
                            as usize;
                        Ok(StereoReferenceV2::Atom(
                            fragment
                                .nodes
                                .get(index)
                                .ok_or_else(|| "atom ligand index is outside fragment".to_string())?
                                .id
                                .clone(),
                        ))
                    }
                    Some("hydrogen") => Ok(StereoReferenceV2::ImplicitHydrogen),
                    _ => Err("tetrahedral center has unsupported ligand kind".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let references: [StereoReferenceV2; 4] = references
                .try_into()
                .map_err(|_| "tetrahedral center must have four references".to_string())?;
            let parity = match center.get("smilesParity").and_then(Value::as_str) {
                Some("clockwise") => TetrahedralParityV2::Clockwise,
                Some("anticlockwise") => TetrahedralParityV2::Anticlockwise,
                _ => return Err("tetrahedral center omitted semantic parity".to_string()),
            };
            Ok(StereoElementV2::Tetrahedral {
                id: format!("tetrahedral-{}", center_node.id),
                center: center_node.id.clone(),
                references,
                parity,
            })
        })
        .collect()
}

fn double_bond_stereo(
    fragment: &MoleculeFragment,
    analysis: &Value,
) -> Result<Vec<StereoElementV2>, String> {
    let mut result = BTreeMap::new();
    let analyzed = analysis
        .get("doubleBondStereo")
        .and_then(Value::as_array)
        .ok_or_else(|| "chemistry analysis omitted doubleBondStereo".to_string())?;
    for value in analyzed {
        let bond_index = json_index(value, "bondIndex")?;
        let begin_reference_index = json_index(value, "beginReferenceBond")?;
        let end_reference_index = json_index(value, "endReferenceBond")?;
        let bond = fragment
            .bonds
            .get(bond_index)
            .ok_or_else(|| "double-bond stereo bondIndex is outside fragment".to_string())?;
        let begin_reference = fragment
            .bonds
            .get(begin_reference_index)
            .ok_or_else(|| "double-bond stereo begin reference is outside fragment".to_string())?;
        let end_reference = fragment
            .bonds
            .get(end_reference_index)
            .ok_or_else(|| "double-bond stereo end reference is outside fragment".to_string())?;
        result.insert(
            bond.id.clone(),
            StereoElementV2::DoubleBond {
                id: format!("double-bond-{}", bond.id),
                bond: bond.id.clone(),
                left_reference: other_endpoint(begin_reference, &bond.begin)?,
                right_reference: other_endpoint(end_reference, &bond.end)?,
                relation: match value.get("configuration").and_then(Value::as_str) {
                    Some("z") => DoubleBondRelationV2::Together,
                    Some("e") => DoubleBondRelationV2::Opposite,
                    _ => return Err("double-bond stereo omitted E/Z configuration".to_string()),
                },
            },
        );
    }
    for bond in &fragment.bonds {
        if result.contains_key(&bond.id) {
            continue;
        }
        let relation = match bond.stereo.as_ref().map(|stereo| stereo.kind.as_str()) {
            Some("cis") => DoubleBondRelationV2::Together,
            Some("trans") => DoubleBondRelationV2::Opposite,
            _ => continue,
        };
        let left = adjacent_reference(fragment, bond, &bond.begin);
        let right = adjacent_reference(fragment, bond, &bond.end);
        if let (Some(left_reference), Some(right_reference)) = (left, right) {
            result.insert(
                bond.id.clone(),
                StereoElementV2::DoubleBond {
                    id: format!("double-bond-{}", bond.id),
                    bond: bond.id.clone(),
                    left_reference,
                    right_reference,
                    relation,
                },
            );
        }
    }
    Ok(result.into_values().collect())
}

fn json_index(value: &Value, field: &str) -> Result<usize, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .ok_or_else(|| format!("double-bond stereo omitted {field}"))
}

fn other_endpoint(reference: &Bond, center: &str) -> Result<String, String> {
    if reference.begin == center {
        Ok(reference.end.clone())
    } else if reference.end == center {
        Ok(reference.begin.clone())
    } else {
        Err("double-bond reference does not meet its bond end".to_string())
    }
}

fn adjacent_reference(fragment: &MoleculeFragment, bond: &Bond, center: &str) -> Option<String> {
    fragment.bonds.iter().find_map(|candidate| {
        if candidate.id == bond.id {
            None
        } else if candidate.begin == center {
            Some(candidate.end.clone())
        } else if candidate.end == center {
            Some(candidate.begin.clone())
        } else {
            None
        }
    })
}

fn molecule_components(fragment: &MoleculeFragment) -> Vec<Vec<String>> {
    let mut adjacency = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), Vec::new()))
        .collect::<BTreeMap<_, Vec<&str>>>();
    for bond in &fragment.bonds {
        adjacency
            .get_mut(bond.begin.as_str())
            .expect("fragment bond endpoint")
            .push(bond.end.as_str());
        adjacency
            .get_mut(bond.end.as_str())
            .expect("fragment bond endpoint")
            .push(bond.begin.as_str());
    }
    let mut remaining = adjacency.keys().copied().collect::<BTreeSet<_>>();
    let mut components = Vec::new();
    while let Some(start) = remaining.iter().next().copied() {
        remaining.remove(start);
        let mut pending = vec![start];
        let mut atoms = Vec::new();
        while let Some(atom) = pending.pop() {
            atoms.push(atom.to_string());
            for neighbor in &adjacency[atom] {
                if remaining.remove(neighbor) {
                    pending.push(neighbor);
                }
            }
        }
        atoms.sort();
        components.push(atoms);
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorCommand;

    #[test]
    fn native_graph_export_is_strict_and_has_no_drawing_geometry() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "C[C@H](O)F".to_string(),
                x: 20.0,
                y: 20.0,
            })
            .unwrap();
        assert!(engine.select_all());
        let json = engine.chemical_graph_v2_json().unwrap();
        let graph: ChemicalGraphV2 = serde_json::from_str(&json).unwrap();
        graph.validate().unwrap();
        assert_eq!(graph.stereo.len(), 1);
        assert!(!json.contains("position"));
        assert!(!json.contains("bbox"));
        assert!(!json.contains("font"));
    }
}
