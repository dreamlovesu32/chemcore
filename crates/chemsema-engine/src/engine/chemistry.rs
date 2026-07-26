use super::{ChemicalAnalysisFormat, CommandTargetSet, Engine, PendingSelectTarget};
use crate::{
    Bond, BondLineStyles, BondLineWeights, BondStereo, DoubleBond, DoubleBondPlacement,
    MoleculeFragment, Node, Point, SelectionState, Tool,
};
use chemsema_chemistry::{
    layout_2d, molecular_properties, sanitize, write_canonical_smiles, write_smiles,
    Atom as ChemicalAtom, Bond as ChemicalBond, BondDirection as ChemicalBondDirection,
    BondKind as ChemicalBondKind, ChiralLigand as ChemicalChiralLigand,
    Chirality as ChemicalChirality, LayoutOptions, Molecule,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    pub(super) fn insert_smiles_untracked(
        &mut self,
        molecule: &Molecule,
        source_smiles: &str,
        anchor: Point,
    ) -> bool {
        if molecule.atoms.is_empty() {
            return false;
        }
        let local_anchor = self
            .state
            .document
            .editable_fragment()
            .map(|entry| {
                Point::new(
                    anchor.x - entry.object.transform.translate[0],
                    anchor.y - entry.object.transform.translate[1],
                )
            })
            .unwrap_or(anchor);
        let points = layout_2d(molecule, LayoutOptions::default());
        let min_x = points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = points
            .iter()
            .map(|point| point.y)
            .fold(f64::INFINITY, f64::min);
        let node_ids = (0..molecule.atoms.len())
            .map(|_| self.next_id("n"))
            .collect::<Vec<_>>();
        let bond_ids = (0..molecule.bonds.len())
            .map(|_| self.next_id("b"))
            .collect::<Vec<_>>();
        let import_id = self.next_id("smiles");

        let sanitization = match sanitize(molecule) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let aromatic_double = sanitization
            .aromatic_double_bonds
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let aromatic_bond_counts = (0..molecule.atoms.len())
            .map(|atom_index| {
                molecule
                    .bonds
                    .iter()
                    .filter(|bond| {
                        bond.kind == ChemicalBondKind::Aromatic
                            && (bond.begin == atom_index || bond.end == atom_index)
                    })
                    .count()
            })
            .collect::<Vec<_>>();
        let display_stereo_centers = display_stereobond_centers(molecule);
        let nodes = molecule
            .atoms
            .iter()
            .enumerate()
            .map(|(index, atom)| Node {
                id: node_ids[index].clone(),
                element: atom.symbol.clone(),
                atomic_number: atom.atomic_number,
                position: [
                    crate::round2(local_anchor.x + points[index].x - min_x),
                    crate::round2(local_anchor.y + points[index].y - min_y),
                ],
                charge: atom.charge,
                num_hydrogens: atom
                    .explicit_hydrogens
                    .saturating_add(sanitization.implicit_hydrogens[index]),
                highlight_color: None,
                external_connection: (atom.atomic_number == 0)
                    .then(crate::ExternalConnection::default),
                is_placeholder: atom.atomic_number == 0,
                label: None,
                atom_properties: crate::AtomProperties::default(),
                nmr_assignments: Vec::new(),
                meta: json!({
                    "chemistry": {
                        "smiles": {
                            "importId": import_id,
                            "source": source_smiles,
                            "atomIndex": index,
                            "sourceAtomicNumber": atom.atomic_number,
                            "sourceSymbol": atom.symbol,
                            "sourceCharge": atom.charge,
                            "isotope": atom.isotope,
                            "aromatic": atom.aromatic,
                            "aromaticBondCount": aromatic_bond_counts[index],
                            "chirality": chirality_name(atom.chirality),
                            "chiralOrder": atom.chiral_order,
                            "atomMap": atom.atom_map,
                            "explicitHydrogens": atom.explicit_hydrogens,
                            "implicitHydrogens": sanitization.implicit_hydrogens[index],
                            "noImplicit": atom.no_implicit,
                        }
                    }
                }),
            })
            .collect::<Vec<_>>();
        let bonds = molecule
            .bonds
            .iter()
            .enumerate()
            .map(|(index, bond)| {
                let order = display_bond_order(bond.kind, aromatic_double.contains(&index));
                let center = display_stereo_centers.get(&index).copied();
                let stereo = center.map(|(center, parity)| BondStereo {
                    kind: display_wedge_kind(molecule, &points, center, index, parity),
                    wide_end: if bond.begin == center { "end" } else { "begin" }.to_string(),
                });
                let stereo_kind = stereo.as_ref().map(|value| value.kind.clone());
                let stereo_wide_end = stereo.as_ref().map(|value| value.wide_end.clone());
                Bond {
                    id: bond_ids[index].clone(),
                    begin: node_ids[bond.begin].clone(),
                    end: node_ids[bond.end].clone(),
                    order,
                    highlight_color: None,
                    properties: Default::default(),
                    double: (order == 2).then_some(DoubleBond {
                        placement: DoubleBondPlacement::Center,
                        center_exit_side: None,
                        frozen: false,
                    }),
                    stereo,
                    stroke_width: crate::DEFAULT_BOND_STROKE,
                    stroke: None,
                    bold_width: Some(crate::BOLD_BOND_WIDTH_PT.value()),
                    wedge_width: Some(crate::SOLID_WEDGE_WIDTH_PT.value()),
                    label_clip_margin: None,
                    hash_spacing: Some(crate::DEFAULT_HASH_SPACING_PT.value()),
                    bond_spacing: Some(crate::DEFAULT_BOND_SPACING_PERCENT),
                    margin_width: Some(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()),
                    line_styles: BondLineStyles::default(),
                    line_weights: BondLineWeights::default(),
                    meta: json!({
                        "chemistry": {
                            "smiles": {
                                "importId": import_id,
                                "bondIndex": index,
                                "kind": bond_kind_name(bond.kind),
                                "displayOrder": order,
                                "direction": bond_direction_name(bond.direction),
                                "stereoKind": stereo_kind,
                                "stereoWideEnd": stereo_wide_end,
                                "stereoCenterNode": center.map(|(center, _)| node_ids[center].as_str()),
                                "dativeDonorNode": if bond.kind == ChemicalBondKind::Dative {
                                    Some(node_ids[bond.begin].as_str())
                                } else {
                                    None
                                },
                            }
                        }
                    }),
                }
            })
            .collect::<Vec<_>>();

        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            return false;
        };
        if molecule.components().len() > 1 {
            if !entry.object.meta.is_object() {
                entry.object.meta = json!({});
            }
            entry.object.meta["preserveDisconnectedComponents"] = Value::Bool(true);
        }
        entry.fragment.nodes.extend(nodes);
        entry.fragment.bonds.extend(bonds);
        entry.update_bounds();
        self.refresh_symbol_chemistry();

        // Anchor the final rendered selection box, not merely the first atom,
        // at the context-click point.
        let previous_selection = self.state.selection.clone();
        self.state.selection = SelectionState {
            nodes: node_ids.clone(),
            bonds: bond_ids.clone(),
            ..SelectionState::default()
        };
        if let Some(bounds) = self.selection_bounds() {
            let dx = anchor.x - bounds[0];
            let dy = anchor.y - bounds[1];
            if let Some(entry) = self.state.document.editable_fragment_mut() {
                let selected = node_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
                for node in &mut entry.fragment.nodes {
                    if selected.contains(node.id.as_str()) {
                        node.position[0] = crate::round2(node.position[0] + dx);
                        node.position[1] = crate::round2(node.position[1] + dy);
                    }
                }
            }
            self.refresh_symbol_chemistry();
        }
        self.state.selection = previous_selection;
        if self.state.tool.active_tool == Tool::Select {
            self.state.selection = SelectionState {
                nodes: node_ids,
                bonds: bond_ids,
                ..SelectionState::default()
            };
            self.pending_select_target = None;
        } else {
            self.pending_select_target = Some(PendingSelectTarget::MoleculeSelection {
                nodes: node_ids,
                bonds: bond_ids,
            });
        }
        true
    }

    pub(super) fn chemical_analysis_output(
        &self,
        format: ChemicalAnalysisFormat,
        targets: &CommandTargetSet,
    ) -> Result<Value, String> {
        let molecule = self.chemical_molecule_for_targets(targets)?;
        let properties = molecular_properties(&molecule).map_err(|error| error.to_string())?;
        match format {
            ChemicalAnalysisFormat::ChemicalGraphV2 => {
                let (_, graph, _) = self.chemical_graph_v2_for_targets(targets)?;
                Ok(json!({
                    "format": "chemical-graph-v2",
                    "value": graph.normalized()?,
                    "provider": "chemsema-chemical-graph",
                }))
            }
            ChemicalAnalysisFormat::Smiles => {
                let (value, canonical, canonical_reason) = match write_canonical_smiles(&molecule) {
                    Ok(value) => (value, true, None),
                    Err(canonical_error) => (
                        write_smiles(&molecule).map_err(|error| error.to_string())?,
                        false,
                        Some(canonical_error.to_string()),
                    ),
                };
                let sanitization = sanitize(&molecule).map_err(|error| error.to_string())?;
                Ok(json!({
                    "format": "smiles",
                    "value": value,
                    "canonical": canonical,
                    "canonicalReason": canonical_reason,
                    "isomeric": true,
                    "provider": "chemsema-chemistry",
                    "implicitHydrogens": sanitization.implicit_hydrogens,
                    "doubleBondStereo": sanitization.double_bond_stereo,
                    "tetrahedralCenters": sanitization.tetrahedral_centers,
                    "properties": properties,
                }))
            }
            ChemicalAnalysisFormat::Inchi | ChemicalAnalysisFormat::InchiKey => {
                let molfile = chemsema_chemistry::write_molfile_v2000(&molecule)?;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let result = chemsema_inchi::from_molfile(&molfile)?;
                    let (name, value) = match format {
                        ChemicalAnalysisFormat::Inchi => ("inchi", result.inchi),
                        ChemicalAnalysisFormat::InchiKey => ("inchi-key", result.inchikey),
                        ChemicalAnalysisFormat::Smiles
                        | ChemicalAnalysisFormat::ChemicalGraphV2 => unreachable!(),
                    };
                    Ok(json!({
                        "format": name,
                        "value": value,
                        "provider": "IUPAC InChI",
                        "providerVersion": "1.07.5",
                        "standard": true,
                        "properties": properties,
                    }))
                }
                #[cfg(target_arch = "wasm32")]
                {
                    Ok(json!({
                        "format": match format {
                            ChemicalAnalysisFormat::Inchi => "inchi",
                            ChemicalAnalysisFormat::InchiKey => "inchi-key",
                            ChemicalAnalysisFormat::Smiles
                            | ChemicalAnalysisFormat::ChemicalGraphV2 => unreachable!(),
                        },
                        "molfile": molfile,
                        "provider": "IUPAC InChI WebAssembly",
                        "providerVersion": "1.07.5",
                        "requiresBrowserHost": true,
                        "properties": properties,
                    }))
                }
            }
        }
    }

    /// Minimal chemistry analysis used by ChemicalGraph export.
    ///
    /// Graph construction needs resolved stereo, but it must not pay for or
    /// depend on canonical SMILES generation. Highly symmetric coordination
    /// drawings can make canonical linearization disproportionately expensive
    /// even though their native graph is already determined.
    pub(super) fn chemical_graph_stereo_analysis(
        &self,
        targets: &CommandTargetSet,
    ) -> Result<Value, String> {
        let molecule = self.chemical_molecule_for_targets(targets)?;
        let sanitization = sanitize(&molecule).map_err(|error| error.to_string())?;
        Ok(json!({
            "implicitHydrogens": sanitization.implicit_hydrogens,
            "doubleBondStereo": sanitization.double_bond_stereo,
            "tetrahedralCenters": sanitization.tetrahedral_centers,
        }))
    }

    fn chemical_molecule_for_targets(
        &self,
        targets: &CommandTargetSet,
    ) -> Result<Molecule, String> {
        let entry = self
            .state
            .document
            .editable_fragment()
            .ok_or_else(|| "document has no editable molecule".to_string())?;
        let target_nodes = if !targets.nodes.is_empty() {
            targets.nodes.iter().cloned().collect::<BTreeSet<_>>()
        } else if !self.state.selection.nodes.is_empty() {
            self.state
                .selection
                .nodes
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
        } else if targets.objects.contains(&entry.object.id)
            || self
                .state
                .selection
                .molecule_objects
                .contains(&entry.object.id)
        {
            entry
                .fragment
                .nodes
                .iter()
                .map(|node| node.id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        if target_nodes.is_empty() {
            return Err("select one complete molecule before chemical analysis".to_string());
        }
        if let Some(bond) = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| target_nodes.contains(&bond.begin) != target_nodes.contains(&bond.end))
        {
            return Err(format!(
                "selection is not a complete molecule; bond '{}' crosses its boundary",
                bond.id
            ));
        }
        let selected_nodes = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| target_nodes.contains(&node.id))
            .collect::<Vec<_>>();
        if selected_nodes.len() != target_nodes.len() {
            return Err("selection contains an unknown atom".to_string());
        }
        let index_by_id = selected_nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let selected_index_by_source = selected_nodes
            .iter()
            .enumerate()
            .filter_map(|(selected, node)| {
                meta_u32(&node.meta, "/chemistry/smiles/atomIndex")
                    .map(|source| (source as usize, selected))
            })
            .collect::<BTreeMap<_, _>>();
        let aromatic_semantics_current = selected_nodes.iter().all(|node| {
            let Some(expected) = meta_u32(&node.meta, "/chemistry/smiles/aromaticBondCount") else {
                return true;
            };
            let current = entry
                .fragment
                .bonds
                .iter()
                .filter(|bond| {
                    (bond.begin == node.id || bond.end == node.id)
                        && meta_str(&bond.meta, "/chemistry/smiles/kind") == Some("aromatic")
                        && meta_u8(&bond.meta, "/chemistry/smiles/displayOrder") == Some(bond.order)
                })
                .count() as u32;
            current == expected
        });
        let stereo_semantics_current = selected_nodes
            .iter()
            .map(|node| {
                let has_source_chirality =
                    meta_str(&node.meta, "/chemistry/smiles/chirality").is_some();
                let current = !has_source_chirality
                    || entry.fragment.bonds.iter().any(|bond| {
                        meta_str(&bond.meta, "/chemistry/smiles/stereoCenterNode")
                            == Some(node.id.as_str())
                            && bond.stereo.as_ref().is_some_and(|stereo| {
                                meta_str(&bond.meta, "/chemistry/smiles/stereoKind")
                                    == Some(stereo.kind.as_str())
                                    && meta_str(&bond.meta, "/chemistry/smiles/stereoWideEnd")
                                        == Some(stereo.wide_end.as_str())
                            })
                    });
                (node.id.as_str(), current)
            })
            .collect::<BTreeMap<_, _>>();
        let mut atoms = selected_nodes
            .iter()
            .map(|node| {
                let source_matches = node_smiles_source_matches(node);
                let chirality_source_matches = source_matches
                    && stereo_semantics_current
                        .get(node.id.as_str())
                        .copied()
                        .unwrap_or(true);
                ChemicalAtom {
                    atomic_number: node.atomic_number,
                    symbol: node.element.clone(),
                    isotope: source_matches
                        .then(|| meta_u16(&node.meta, "/chemistry/smiles/isotope"))
                        .flatten(),
                    charge: node.charge,
                    explicit_hydrogens: if source_matches {
                        meta_u8(&node.meta, "/chemistry/smiles/explicitHydrogens").unwrap_or(0)
                    } else {
                        0
                    },
                    aromatic: source_matches
                        && aromatic_semantics_current
                        && meta_bool(&node.meta, "/chemistry/smiles/aromatic").unwrap_or(false),
                    chirality: if chirality_source_matches {
                        match meta_str(&node.meta, "/chemistry/smiles/chirality") {
                            Some("clockwise") => Some(ChemicalChirality::Clockwise),
                            Some("anticlockwise") => Some(ChemicalChirality::Anticlockwise),
                            _ => None,
                        }
                    } else {
                        None
                    },
                    chiral_order: if chirality_source_matches {
                        node.meta
                            .pointer("/chemistry/smiles/chiralOrder")
                            .cloned()
                            .and_then(|value| {
                                serde_json::from_value::<Vec<ChemicalChiralLigand>>(value).ok()
                            })
                            .map(|order| {
                                order
                                    .into_iter()
                                    .filter_map(|ligand| match ligand {
                                        ChemicalChiralLigand::Hydrogen => {
                                            Some(ChemicalChiralLigand::Hydrogen)
                                        }
                                        ChemicalChiralLigand::Atom(source) => {
                                            selected_index_by_source
                                                .get(&source)
                                                .copied()
                                                .map(ChemicalChiralLigand::Atom)
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    },
                    atom_map: meta_u32(&node.meta, "/chemistry/smiles/atomMap"),
                    no_implicit: source_matches
                        && meta_bool(&node.meta, "/chemistry/smiles/noImplicit").unwrap_or(false),
                }
            })
            .collect::<Vec<_>>();
        let bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| target_nodes.contains(&bond.begin) && target_nodes.contains(&bond.end))
            .map(|bond| {
                let semantic_kind = (meta_u8(&bond.meta, "/chemistry/smiles/displayOrder")
                    == Some(bond.order))
                .then(|| meta_str(&bond.meta, "/chemistry/smiles/kind"))
                .flatten();
                let kind = match semantic_kind {
                    Some("aromatic") if aromatic_semantics_current => ChemicalBondKind::Aromatic,
                    Some("aromatic") => match bond.order {
                        2 => ChemicalBondKind::Double,
                        3 => ChemicalBondKind::Triple,
                        4 => ChemicalBondKind::Quadruple,
                        _ => ChemicalBondKind::Single,
                    },
                    Some("dative") => ChemicalBondKind::Dative,
                    Some("quadruple") => ChemicalBondKind::Quadruple,
                    Some("triple") => ChemicalBondKind::Triple,
                    Some("double") => ChemicalBondKind::Double,
                    Some("single") => ChemicalBondKind::Single,
                    _ => match bond.order {
                        2 => ChemicalBondKind::Double,
                        3 => ChemicalBondKind::Triple,
                        4 => ChemicalBondKind::Quadruple,
                        _ => ChemicalBondKind::Single,
                    },
                };
                let mut begin = index_by_id[bond.begin.as_str()];
                let mut end = index_by_id[bond.end.as_str()];
                if kind == ChemicalBondKind::Dative
                    && meta_str(&bond.meta, "/chemistry/smiles/dativeDonorNode")
                        == Some(bond.end.as_str())
                {
                    std::mem::swap(&mut begin, &mut end);
                }
                ChemicalBond {
                    begin,
                    end,
                    kind,
                    direction: match meta_str(&bond.meta, "/chemistry/smiles/direction") {
                        Some("up") => Some(ChemicalBondDirection::Up),
                        Some("down") => Some(ChemicalBondDirection::Down),
                        _ => None,
                    },
                }
            })
            .collect::<Vec<_>>();
        infer_wedge_chirality(
            entry.fragment,
            &target_nodes,
            &index_by_id,
            &mut atoms,
            &bonds,
        )?;
        Ok(Molecule { atoms, bonds })
    }
}

fn display_bond_order(kind: ChemicalBondKind, aromatic_double: bool) -> u8 {
    match kind {
        ChemicalBondKind::Double => 2,
        ChemicalBondKind::Triple => 3,
        ChemicalBondKind::Quadruple => 4,
        ChemicalBondKind::Aromatic if aromatic_double => 2,
        _ => 1,
    }
}

fn display_stereobond_centers(molecule: &Molecule) -> BTreeMap<usize, (usize, ChemicalChirality)> {
    let mut result = BTreeMap::new();
    let mut used = BTreeSet::new();
    for (center, atom) in molecule.atoms.iter().enumerate() {
        let Some(parity) = atom.chirality else {
            continue;
        };
        let chosen = atom.chiral_order.iter().find_map(|ligand| {
            let ChemicalChiralLigand::Atom(neighbor) = ligand else {
                return None;
            };
            molecule
                .bonds
                .iter()
                .enumerate()
                .find_map(|(bond_index, bond)| {
                    (bond.kind == ChemicalBondKind::Single
                        && !used.contains(&bond_index)
                        && ((bond.begin == center && bond.end == *neighbor)
                            || (bond.end == center && bond.begin == *neighbor)))
                        .then_some(bond_index)
                })
        });
        if let Some(bond_index) = chosen {
            used.insert(bond_index);
            result.insert(bond_index, (center, parity));
        }
    }
    result
}

fn display_wedge_kind(
    molecule: &Molecule,
    points: &[chemsema_chemistry::Point2],
    center: usize,
    chosen_bond: usize,
    desired: ChemicalChirality,
) -> String {
    let mut neighbors = molecule
        .bonds
        .iter()
        .filter_map(|bond| {
            if bond.kind != ChemicalBondKind::Single {
                return None;
            }
            if bond.begin == center {
                Some(bond.end)
            } else if bond.end == center {
                Some(bond.begin)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    neighbors.sort_unstable();
    let chosen = if molecule.bonds[chosen_bond].begin == center {
        molecule.bonds[chosen_bond].end
    } else {
        molecule.bonds[chosen_bond].begin
    };
    let mut vectors = neighbors
        .iter()
        .map(|neighbor| {
            [
                points[*neighbor].x - points[center].x,
                -(points[*neighbor].y - points[center].y),
                f64::from(*neighbor == chosen),
            ]
        })
        .collect::<Vec<_>>();
    if neighbors.len() == 3 && molecule.atoms[center].explicit_hydrogens == 1 {
        let sum = vectors.iter().fold([0.0; 3], |mut sum, value| {
            for axis in 0..3 {
                sum[axis] += value[axis];
            }
            sum
        });
        vectors.push([-sum[0], -sum[1], -sum[2]]);
    }
    let solid_matches = if vectors.len() == 4 {
        let a = subtract3(vectors[1], vectors[0]);
        let b = subtract3(vectors[2], vectors[0]);
        let c = subtract3(vectors[3], vectors[0]);
        (dot3(a, cross3(b, c)) < 0.0) == (desired == ChemicalChirality::Anticlockwise)
    } else {
        desired == ChemicalChirality::Anticlockwise
    };
    if solid_matches {
        "solid-wedge".to_string()
    } else {
        "hashed-wedge".to_string()
    }
}

fn chirality_name(value: Option<ChemicalChirality>) -> Option<&'static str> {
    match value {
        Some(ChemicalChirality::Clockwise) => Some("clockwise"),
        Some(ChemicalChirality::Anticlockwise) => Some("anticlockwise"),
        None => None,
    }
}

fn bond_kind_name(value: ChemicalBondKind) -> &'static str {
    match value {
        ChemicalBondKind::Single => "single",
        ChemicalBondKind::Double => "double",
        ChemicalBondKind::Triple => "triple",
        ChemicalBondKind::Quadruple => "quadruple",
        ChemicalBondKind::Aromatic => "aromatic",
        ChemicalBondKind::Dative => "dative",
    }
}

fn bond_direction_name(value: Option<ChemicalBondDirection>) -> Option<&'static str> {
    match value {
        Some(ChemicalBondDirection::Up) => Some("up"),
        Some(ChemicalBondDirection::Down) => Some("down"),
        None => None,
    }
}

fn infer_wedge_chirality(
    fragment: &MoleculeFragment,
    target_nodes: &BTreeSet<String>,
    index_by_id: &BTreeMap<&str, usize>,
    atoms: &mut [ChemicalAtom],
    bonds: &[ChemicalBond],
) -> Result<(), String> {
    for display_bond in fragment
        .bonds
        .iter()
        .filter(|bond| target_nodes.contains(&bond.begin) && target_nodes.contains(&bond.end))
    {
        let Some(stereo) = display_bond.stereo.as_ref() else {
            continue;
        };
        if !matches!(
            stereo.kind.as_str(),
            "solid-wedge" | "hashed-wedge" | "hollow-wedge"
        ) {
            continue;
        }
        let center_id = match stereo.wide_end.as_str() {
            "end" => display_bond.begin.as_str(),
            "begin" => display_bond.end.as_str(),
            _ => continue,
        };
        let center = index_by_id[center_id];
        if atoms[center].chirality.is_some() {
            continue;
        }
        let mut neighbors = bonds
            .iter()
            .filter_map(|bond| {
                if bond.kind != ChemicalBondKind::Single {
                    return None;
                }
                if bond.begin == center {
                    Some(bond.end)
                } else if bond.end == center {
                    Some(bond.begin)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        neighbors.sort_unstable();
        neighbors.dedup();
        if !(3..=4).contains(&neighbors.len()) {
            return Err(format!(
                "wedge at atom '{}' does not describe a tetrahedral center",
                center_id
            ));
        }
        let center_node = fragment
            .nodes
            .iter()
            .find(|node| node.id == center_id)
            .unwrap();
        if let Some(source_order) = center_node
            .meta
            .pointer("/import/cdxml/bondOrdering")
            .and_then(Value::as_str)
        {
            let ordered = source_order
                .split_whitespace()
                .filter_map(|bond_id| {
                    let bond = fragment.bonds.iter().find(|bond| bond.id == bond_id)?;
                    let other = if bond.begin == center_id {
                        bond.end.as_str()
                    } else if bond.end == center_id {
                        bond.begin.as_str()
                    } else {
                        return None;
                    };
                    index_by_id.get(other).copied()
                })
                .collect::<Vec<_>>();
            if ordered.len() == neighbors.len()
                && ordered.iter().copied().collect::<BTreeSet<_>>().len() == neighbors.len()
                && ordered.iter().all(|neighbor| neighbors.contains(neighbor))
            {
                neighbors = ordered;
            }
        }
        let elevated_id = if center_id == display_bond.begin {
            display_bond.end.as_str()
        } else {
            display_bond.begin.as_str()
        };
        let elevated = index_by_id[elevated_id];
        let z = if stereo.kind == "hashed-wedge" {
            -1.0
        } else {
            1.0
        };
        let mut vectors = neighbors
            .iter()
            .map(|neighbor| {
                let node_id = index_by_id
                    .iter()
                    .find_map(|(id, index)| (*index == *neighbor).then_some(*id))
                    .unwrap();
                let node = fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == node_id)
                    .unwrap();
                [
                    node.position[0] - center_node.position[0],
                    -(node.position[1] - center_node.position[1]),
                    if *neighbor == elevated { z } else { 0.0 },
                ]
            })
            .collect::<Vec<_>>();
        let mut order = neighbors
            .iter()
            .copied()
            .map(ChemicalChiralLigand::Atom)
            .collect::<Vec<_>>();
        if neighbors.len() == 3 {
            let sum = vectors.iter().fold([0.0; 3], |mut sum, value| {
                for axis in 0..3 {
                    sum[axis] += value[axis];
                }
                sum
            });
            vectors.push([-sum[0], -sum[1], -sum[2]]);
            order.push(ChemicalChiralLigand::Hydrogen);
            atoms[center].explicit_hydrogens = 1;
            atoms[center].no_implicit = true;
        }
        let a = subtract3(vectors[1], vectors[0]);
        let b = subtract3(vectors[2], vectors[0]);
        let c = subtract3(vectors[3], vectors[0]);
        let volume = dot3(a, cross3(b, c));
        if volume.abs() < 1e-8 {
            return Err(format!(
                "wedge at atom '{}' has degenerate geometry",
                center_id
            ));
        }
        atoms[center].chirality = Some(if volume < 0.0 {
            ChemicalChirality::Anticlockwise
        } else {
            ChemicalChirality::Clockwise
        });
        atoms[center].chiral_order = order;
    }
    Ok(())
}

fn subtract3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn meta_str<'a>(meta: &'a Value, pointer: &str) -> Option<&'a str> {
    meta.pointer(pointer).and_then(Value::as_str)
}

fn meta_bool(meta: &Value, pointer: &str) -> Option<bool> {
    meta.pointer(pointer).and_then(Value::as_bool)
}

fn meta_u32(meta: &Value, pointer: &str) -> Option<u32> {
    meta.pointer(pointer)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

fn meta_u16(meta: &Value, pointer: &str) -> Option<u16> {
    meta.pointer(pointer)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

fn meta_u8(meta: &Value, pointer: &str) -> Option<u8> {
    meta.pointer(pointer)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

fn node_smiles_source_matches(node: &Node) -> bool {
    meta_u8(&node.meta, "/chemistry/smiles/sourceAtomicNumber")
        .is_none_or(|value| value == node.atomic_number)
        && meta_str(&node.meta, "/chemistry/smiles/sourceSymbol")
            .is_none_or(|value| value == node.element)
        && node
            .meta
            .pointer("/chemistry/smiles/sourceCharge")
            .and_then(Value::as_i64)
            .is_none_or(|value| value == i64::from(node.charge))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorCommand;

    #[test]
    fn smiles_insert_is_one_undo_step_and_switch_selects_all_components() {
        let mut engine = Engine::new();
        let result = engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "c1ccccc1.CC".to_string(),
                x: 120.0,
                y: 80.0,
            })
            .unwrap();
        assert!(result.changed);
        assert_eq!(result.undo_depth, 1);
        assert_eq!(
            engine
                .state()
                .document
                .editable_fragment()
                .unwrap()
                .fragment
                .nodes
                .len(),
            8
        );
        let mut tool = engine.state().tool.clone();
        tool.active_tool = Tool::Select;
        engine.set_tool_state(tool);
        assert_eq!(engine.state().selection.nodes.len(), 8);
        assert_eq!(engine.state().selection.bonds.len(), 7);
        assert!(engine.undo());
        assert!(engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .fragment
            .nodes
            .is_empty());
    }

    #[test]
    fn invalid_smiles_does_not_mutate_or_create_history() {
        let mut engine = Engine::new();
        let before = engine.document_json().unwrap();
        let error = engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "C1CC".to_string(),
                x: 0.0,
                y: 0.0,
            })
            .unwrap_err();
        assert!(error.contains("ring 1"));
        assert_eq!(engine.document_json().unwrap(), before);
        assert!(!engine.can_undo());
    }

    #[test]
    fn selected_import_round_trips_to_smiles() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "[13C@@H:7](F)(Cl)[NH3+]".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let mut tool = engine.state().tool.clone();
        tool.active_tool = Tool::Select;
        engine.set_tool_state(tool);
        let output = engine
            .chemical_analysis_output(ChemicalAnalysisFormat::Smiles, &CommandTargetSet::default())
            .unwrap();
        let reparsed = chemsema_chemistry::parse_smiles(output["value"].as_str().unwrap()).unwrap();
        assert_eq!(reparsed.atoms.len(), 4);
        assert!(reparsed.atoms.iter().any(|atom| atom.charge == 1));
        assert_eq!(output["canonical"], true);
        assert!(output["canonicalReason"].is_null());
        assert_eq!(output["tetrahedralCenters"][0]["cip"], "R");
    }

    #[test]
    fn imported_smiles_uses_sanitized_hydrogen_semantics_and_canonical_output() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "CCO".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let entry = engine.state().document.editable_fragment().unwrap();
        let oxygen = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.element == "O")
            .unwrap();
        assert_eq!(oxygen.num_hydrogens, 1);
        let targets = CommandTargetSet {
            objects: vec![entry.object.id.clone()],
            ..CommandTargetSet::default()
        };
        let output = engine
            .chemical_analysis_output(ChemicalAnalysisFormat::Smiles, &targets)
            .unwrap();
        assert_eq!(output["canonical"], true);
        assert_eq!(output["implicitHydrogens"].as_array().unwrap().len(), 3);
        assert_eq!(output["properties"]["formula"], "C2H6O");

        let mut bracketed = Engine::new();
        bracketed
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "[O]".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        assert_eq!(
            bracketed
                .state()
                .document
                .editable_fragment()
                .unwrap()
                .fragment
                .nodes[0]
                .num_hydrogens,
            0
        );
    }

    #[test]
    fn tetrahedral_import_has_one_wedge_and_official_inchi_stereo() {
        fn analyze(smiles: &str) -> (String, String) {
            let mut engine = Engine::new();
            engine
                .execute_command(EditorCommand::InsertSmiles {
                    smiles: smiles.to_string(),
                    x: 10.0,
                    y: 10.0,
                })
                .unwrap();
            let entry = engine.state().document.editable_fragment().unwrap();
            assert_eq!(
                entry
                    .fragment
                    .bonds
                    .iter()
                    .filter(|bond| bond.stereo.is_some())
                    .count(),
                1
            );
            let targets = CommandTargetSet {
                objects: vec![entry.object.id.clone()],
                ..CommandTargetSet::default()
            };
            let smiles_output = engine
                .chemical_analysis_output(ChemicalAnalysisFormat::Smiles, &targets)
                .unwrap();
            let inchi_output = engine
                .chemical_analysis_output(ChemicalAnalysisFormat::Inchi, &targets)
                .unwrap();
            (
                smiles_output["tetrahedralCenters"][0]["cip"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                inchi_output["value"].as_str().unwrap().to_string(),
            )
        }

        let l = analyze("N[C@@H](C)C(=O)O");
        let d = analyze("N[C@H](C)C(=O)O");
        assert_eq!(l.0, "S");
        assert_eq!(d.0, "R");
        assert_eq!(
            l.1,
            "InChI=1S/C3H7NO2/c1-2(4)3(5)6/h2H,4H2,1H3,(H,5,6)/t2-/m0/s1"
        );
        assert_ne!(l.1, d.1);
        assert!(d.1.contains("/t2-/m1/s1"), "{}", d.1);
    }

    #[test]
    fn displayed_wedge_can_reconstruct_tetrahedral_smiles_without_source_parity() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "N[C@@H](C)C(=O)O".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let entry = engine.state.document.editable_fragment_mut().unwrap();
        for node in &mut entry.fragment.nodes {
            if node
                .meta
                .pointer("/chemistry/smiles/chirality")
                .is_some_and(|value| !value.is_null())
            {
                node.meta["chemistry"]["smiles"]["chirality"] = Value::Null;
                node.meta["chemistry"]["smiles"]["chiralOrder"] = Value::Null;
            }
        }
        let object = entry.object.id.clone();
        let output = engine
            .chemical_analysis_output(
                ChemicalAnalysisFormat::Smiles,
                &CommandTargetSet {
                    objects: vec![object],
                    ..CommandTargetSet::default()
                },
            )
            .unwrap();
        assert_eq!(output["tetrahedralCenters"][0]["cip"], "S");
    }

    #[test]
    fn editing_imported_wedge_invalidates_source_chirality() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "N[C@@H](C)C(=O)O".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let entry = engine.state.document.editable_fragment_mut().unwrap();
        let wedge = entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.stereo.is_some())
            .unwrap();
        let stereo = wedge.stereo.as_mut().unwrap();
        stereo.kind = if stereo.kind == "solid-wedge" {
            "hashed-wedge".to_string()
        } else {
            "solid-wedge".to_string()
        };
        let object = entry.object.id.clone();
        let output = engine
            .chemical_analysis_output(
                ChemicalAnalysisFormat::Smiles,
                &CommandTargetSet {
                    objects: vec![object],
                    ..CommandTargetSet::default()
                },
            )
            .unwrap();
        assert_eq!(output["tetrahedralCenters"][0]["cip"], "R");
    }

    #[test]
    fn adjacent_tetrahedral_centers_receive_distinct_display_wedges() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "N[C@@H](Cl)[C@@H](Cl)C".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let entry = engine.state().document.editable_fragment().unwrap();
        assert_eq!(
            entry
                .fragment
                .bonds
                .iter()
                .filter(|bond| bond.stereo.is_some())
                .count(),
            2
        );
    }

    #[test]
    fn disconnected_import_stays_one_record_after_save_and_reload() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "c1ccccc1.CC".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let saved = engine.document_json().unwrap();
        let mut reopened = Engine::new();
        reopened.load_document_json(&saved).unwrap();
        assert_eq!(reopened.state().document.editable_fragments().len(), 1);
        let entry = reopened.state().document.editable_fragment().unwrap();
        assert_eq!(entry.fragment.nodes.len(), 8);
        assert_eq!(entry.fragment.bonds.len(), 7);
    }

    #[test]
    fn edited_aromatic_display_bond_invalidates_imported_aromatic_authority() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "c1ccccc1".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let object_id = engine
            .state()
            .document
            .editable_fragment()
            .unwrap()
            .object
            .id
            .clone();
        let entry = engine.state.document.editable_fragment_mut().unwrap();
        entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.order == 2)
            .unwrap()
            .order = 1;
        let output = engine
            .chemical_analysis_output(
                ChemicalAnalysisFormat::Smiles,
                &CommandTargetSet {
                    objects: vec![object_id],
                    ..CommandTargetSet::default()
                },
            )
            .unwrap();
        assert!(!output["value"].as_str().unwrap().contains(':'));
        assert_eq!(output["properties"]["formula"], "C6H8");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn official_inchi_backend_analyzes_selected_molecule() {
        let mut engine = Engine::new();
        engine
            .execute_command(EditorCommand::InsertSmiles {
                smiles: "CCO".to_string(),
                x: 10.0,
                y: 10.0,
            })
            .unwrap();
        let targets = CommandTargetSet {
            objects: vec![engine
                .state()
                .document
                .editable_fragment()
                .unwrap()
                .object
                .id
                .clone()],
            ..CommandTargetSet::default()
        };
        let inchi = engine
            .chemical_analysis_output(ChemicalAnalysisFormat::Inchi, &targets)
            .unwrap();
        let key = engine
            .chemical_analysis_output(ChemicalAnalysisFormat::InchiKey, &targets)
            .unwrap();
        assert_eq!(inchi["value"], "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3");
        assert_eq!(key["value"], "LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn official_inchi_backend_preserves_e_z_stereo() {
        let analyze = |smiles: &str| {
            let mut engine = Engine::new();
            engine
                .execute_command(EditorCommand::InsertSmiles {
                    smiles: smiles.to_string(),
                    x: 10.0,
                    y: 10.0,
                })
                .unwrap();
            let object = engine
                .state()
                .document
                .editable_fragment()
                .unwrap()
                .object
                .id
                .clone();
            engine
                .chemical_analysis_output(
                    ChemicalAnalysisFormat::Inchi,
                    &CommandTargetSet {
                        objects: vec![object],
                        ..CommandTargetSet::default()
                    },
                )
                .unwrap()["value"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let e = analyze("F/C=C/F");
        let z = analyze("F/C=C\\F");
        // PubChem CID 5365501 (trans/E) and CID 5462921 (cis/Z).
        assert_eq!(e, "InChI=1S/C2H2F2/c3-1-2-4/h1-2H/b2-1+");
        assert_eq!(z, "InChI=1S/C2H2F2/c3-1-2-4/h1-2H/b2-1-");
    }
}
