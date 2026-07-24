use super::{ChemicalAnalysisFormat, CommandTargetSet, Engine};
use crate::{
    round2, AtomRadical, ChemSemaDocument, LabelRun, MoleculeFragment, NmrAssignment,
    NmrAssignmentQuality, NmrNucleus, NodeLabel, ObjectPayload, Resource, ResourceData,
    SceneObject, SpectrumClass, SpectrumData, SpectrumXAxisType, SpectrumYAxisType, Transform,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const RESULT_PAGE_WIDTH: f64 = 523.32;
const RESULT_PAGE_HEIGHT: f64 = 769.92;
const MOLECULE_LEFT: f64 = 28.8;
const MOLECULE_TOP: f64 = 58.0;
const SPECTRUM_LEFT: f64 = 14.4;
const SPECTRUM_TOP: f64 = 119.85;
const SPECTRUM_WIDTH: f64 = 450.0;
const SPECTRUM_HEIGHT: f64 = 200.0;

fn tetrahedral_stereo_from_analysis(
    fragment: &MoleculeFragment,
    analysis: &Value,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let mut stereo = Vec::new();
    let mut descriptors = BTreeMap::<String, String>::new();
    let centers = analysis
        .get("tetrahedralCenters")
        .and_then(Value::as_array)
        .ok_or_else(|| "chemistry analysis omitted tetrahedralCenters".to_string())?;
    for center in centers {
        let atom_index = center
            .get("atomIndex")
            .and_then(Value::as_u64)
            .ok_or_else(|| "tetrahedral center omitted atomIndex".to_string())?
            as usize;
        let center_node = fragment
            .nodes
            .get(atom_index)
            .ok_or_else(|| "tetrahedral center atomIndex is outside the fragment".to_string())?;
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
                    let node = fragment
                        .nodes
                        .get(index)
                        .ok_or_else(|| "atom ligand index is outside the fragment".to_string())?;
                    Ok(json!({"kind": "atom", "value": node.id}))
                }
                Some("hydrogen") => Ok(json!({"kind": "implicitHydrogen"})),
                _ => Err("tetrahedral center has an unsupported ligand kind".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if references.len() != 4 {
            return Err(format!(
                "tetrahedral center '{}' does not have four semantic references",
                center_node.id
            ));
        }
        let parity = match center.get("smilesParity").and_then(Value::as_str) {
            Some("clockwise") => "clockwise",
            Some("anticlockwise") => "anticlockwise",
            _ => return Err("tetrahedral center omitted semantic parity".to_string()),
        };
        if let Some(cip) = center
            .get("cip")
            .and_then(Value::as_str)
            .filter(|value| matches!(*value, "R" | "S" | "r" | "s"))
        {
            descriptors.insert(center_node.id.clone(), cip.to_string());
        }
        stereo.push(json!({
            "kind": "tetrahedral",
            "id": format!("tetrahedral-{}", center_node.id),
            "center": center_node.id,
            "references": references,
            "parity": parity,
        }));
    }
    let descriptors = descriptors
        .into_iter()
        .map(|(atom_id, descriptor)| {
            json!({
                "atomId": atom_id,
                "descriptor": descriptor,
            })
        })
        .collect();
    Ok((stereo, descriptors))
}

fn double_bond_stereo(fragment: &MoleculeFragment, analysis: &Value) -> Result<Vec<Value>, String> {
    let mut result = BTreeMap::<String, Value>::new();
    let analyzed = analysis
        .get("doubleBondStereo")
        .and_then(Value::as_array)
        .ok_or_else(|| "chemistry analysis omitted doubleBondStereo".to_string())?;
    for stereo in analyzed {
        let bond_index = stereo
            .get("bondIndex")
            .and_then(Value::as_u64)
            .ok_or_else(|| "double-bond stereo omitted bondIndex".to_string())?
            as usize;
        let begin_reference_index = stereo
            .get("beginReferenceBond")
            .and_then(Value::as_u64)
            .ok_or_else(|| "double-bond stereo omitted beginReferenceBond".to_string())?
            as usize;
        let end_reference_index = stereo
            .get("endReferenceBond")
            .and_then(Value::as_u64)
            .ok_or_else(|| "double-bond stereo omitted endReferenceBond".to_string())?
            as usize;
        let bond = fragment
            .bonds
            .get(bond_index)
            .ok_or_else(|| "double-bond stereo bondIndex is outside the fragment".to_string())?;
        let begin_reference_bond = fragment.bonds.get(begin_reference_index).ok_or_else(|| {
            "double-bond stereo beginReferenceBond is outside the fragment".to_string()
        })?;
        let end_reference_bond = fragment.bonds.get(end_reference_index).ok_or_else(|| {
            "double-bond stereo endReferenceBond is outside the fragment".to_string()
        })?;
        let other_endpoint = |reference: &crate::Bond, center: &str| {
            if reference.begin == center {
                Some(reference.end.clone())
            } else if reference.end == center {
                Some(reference.begin.clone())
            } else {
                None
            }
        };
        let left_reference = other_endpoint(begin_reference_bond, &bond.begin)
            .ok_or_else(|| "begin reference bond does not meet the double bond".to_string())?;
        let right_reference = other_endpoint(end_reference_bond, &bond.end)
            .ok_or_else(|| "end reference bond does not meet the double bond".to_string())?;
        let relation = match stereo.get("configuration").and_then(Value::as_str) {
            Some("z") => "together",
            Some("e") => "opposite",
            _ => return Err("double-bond stereo omitted E/Z configuration".to_string()),
        };
        result.insert(
            bond.id.clone(),
            json!({
                "kind": "doubleBond",
                "id": format!("double-bond-{}", bond.id),
                "bond": bond.id,
                "leftReference": left_reference,
                "rightReference": right_reference,
                "relation": relation,
            }),
        );
    }

    // CDXML can carry a semantic cis/trans relation directly even when it did
    // not originate from directional SMILES bonds. This is a separate explicit
    // input representation, not a geometry-derived guess.
    for bond in &fragment.bonds {
        if result.contains_key(&bond.id) {
            continue;
        }
        let Some(relation) = bond
            .stereo
            .as_ref()
            .and_then(|stereo| match stereo.kind.as_str() {
                "cis" => Some("together"),
                "trans" => Some("opposite"),
                _ => None,
            })
        else {
            continue;
        };
        let left_reference = fragment.bonds.iter().find_map(|candidate| {
            if candidate.id == bond.id {
                None
            } else if candidate.begin == bond.begin && candidate.end != bond.end {
                Some(candidate.end.as_str())
            } else if candidate.end == bond.begin && candidate.begin != bond.end {
                Some(candidate.begin.as_str())
            } else {
                None
            }
        });
        let right_reference = fragment.bonds.iter().find_map(|candidate| {
            if candidate.id == bond.id {
                None
            } else if candidate.begin == bond.end && candidate.end != bond.begin {
                Some(candidate.end.as_str())
            } else if candidate.end == bond.end && candidate.begin != bond.begin {
                Some(candidate.begin.as_str())
            } else {
                None
            }
        });
        if let (Some(left_reference), Some(right_reference)) = (left_reference, right_reference) {
            result.insert(
                bond.id.clone(),
                json!({
                    "kind": "doubleBond",
                    "id": format!("double-bond-{}", bond.id),
                    "bond": bond.id,
                    "leftReference": left_reference,
                    "rightReference": right_reference,
                    "relation": relation,
                }),
            );
        }
    }
    Ok(result.into_values().collect())
}

fn molecule_components(fragment: &MoleculeFragment) -> Vec<Vec<String>> {
    let mut adjacency = BTreeMap::<&str, Vec<&str>>::new();
    for node in &fragment.nodes {
        adjacency.entry(node.id.as_str()).or_default();
    }
    for bond in &fragment.bonds {
        adjacency
            .entry(bond.begin.as_str())
            .or_default()
            .push(bond.end.as_str());
        adjacency
            .entry(bond.end.as_str())
            .or_default()
            .push(bond.begin.as_str());
    }

    let mut remaining = adjacency.keys().copied().collect::<BTreeSet<_>>();
    let mut components = Vec::new();
    while let Some(start) = remaining.iter().next().copied() {
        let mut pending = vec![start];
        let mut atoms = Vec::new();
        remaining.remove(start);
        while let Some(atom) = pending.pop() {
            atoms.push(atom.to_string());
            if let Some(neighbors) = adjacency.get(atom) {
                for neighbor in neighbors {
                    if remaining.remove(neighbor) {
                        pending.push(neighbor);
                    }
                }
            }
        }
        atoms.sort();
        components.push(atoms);
    }
    components
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionResponse {
    schema: String,
    engine_version: String,
    rule_set_version: String,
    status: PredictionStatus,
    molecule_id: String,
    nucleus: NmrNucleus,
    conditions: PredictionConditions,
    assignments: Vec<PredictionAssignment>,
    couplings: Vec<PredictionCoupling>,
    peaks: Vec<PredictionPeak>,
    diagnostics: Vec<PredictionDiagnostic>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum PredictionStatus {
    Complete,
    Partial,
    Unsupported,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionConditions {
    solvent: String,
    #[serde(rename = "frequencyMHz")]
    frequency_mhz: f64,
    temperature_kelvin: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionAssignment {
    site_ids: Vec<String>,
    atom_ids: Vec<String>,
    shift_ppm: f64,
    integral: f64,
    confidence: NmrAssignmentQuality,
    confidence_reason: String,
    equivalence_class: String,
    contributions: Vec<PredictionContribution>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionContribution {
    rule_id: String,
    value_ppm: f64,
    role: String,
    source_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionCoupling {
    site_ids: [String; 2],
    atom_ids: [String; 2],
    nuclei: [String; 2],
    value_hz: f64,
    rule_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionPeak {
    assignment_indexes: Vec<usize>,
    center_ppm: f64,
    intensity: f64,
    line_positions_ppm: Vec<f64>,
    line_intensities: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PredictionDiagnostic {
    code: String,
    message: String,
    atom_ids: Vec<String>,
}

impl Engine {
    pub fn nmr_prediction_request_json(&self, nucleus: &str) -> Result<String, String> {
        let nucleus = match nucleus {
            "1H" => NmrNucleus::Hydrogen1,
            "13C" => NmrNucleus::Carbon13,
            _ => return Err("NMR nucleus must be 1H or 13C".to_string()),
        };
        let (object, fragment) = self.selected_single_molecule_fragment()?;
        let analysis = self.chemical_analysis_output(
            ChemicalAnalysisFormat::Smiles,
            &CommandTargetSet {
                objects: vec![object.id.clone()],
                ..CommandTargetSet::default()
            },
        )?;
        let (mut stereo, assigned_cip_descriptors) =
            tetrahedral_stereo_from_analysis(&fragment, &analysis)?;
        let atoms = fragment
            .nodes
            .iter()
            .map(|node| {
                let radical = match node.atom_properties.radical {
                    AtomRadical::None => "none",
                    AtomRadical::Singlet => "singlet",
                    AtomRadical::Doublet => "doublet",
                    AtomRadical::Triplet => "triplet",
                };
                json!({
                    "id": node.id,
                    "atomicNumber": node.atomic_number,
                    "isotope": node.atom_properties.isotope_mass.filter(|value| *value > 0),
                    "formalCharge": node.charge,
                    "radical": radical,
                    "implicitHydrogens": crate::engine::formula_hydrogen_count_for_node(
                        &fragment,
                        &node.id,
                    ),
                })
            })
            .collect::<Vec<_>>();
        let bonds = fragment
            .bonds
            .iter()
            .map(|bond| -> Result<Value, String> {
                let aromatic = bond
                    .meta
                    .get("chemistry")
                    .and_then(|value| value.get("smiles"))
                    .and_then(|value| value.get("kind"))
                    .and_then(Value::as_str)
                    == Some("aromatic");
                let order = match (aromatic, bond.order) {
                    (true, _) => "aromatic",
                    (false, 1) => "single",
                    (false, 2) => "double",
                    (false, 3) => "triple",
                    _ => {
                        return Err(format!(
                            "bond '{}' has unsupported NMR prediction order {}",
                            bond.id, bond.order
                        ));
                    }
                };
                Ok(json!({
                    "id": bond.id,
                    "atoms": [bond.begin.clone(), bond.end.clone()],
                    "kind": order,
                    "dativeDirection": null,
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;
        stereo.extend(double_bond_stereo(&fragment, &analysis)?);
        let components = molecule_components(&fragment)
            .into_iter()
            .enumerate()
            .map(|(index, atoms)| {
                json!({
                    "id": format!("component-{}", index + 1),
                    "atoms": atoms,
                    "count": 1,
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "schema": "chemsema.nmr-prediction-request.v2",
            "moleculeId": object.id,
            "graph": {
                "schema": "chemsema-nomenclature/chemical-graph/2",
                "atoms": atoms,
                "bonds": bonds,
                "stereo": stereo,
                "components": components,
                "assumptions": [{
                    "code": "implicit-hydrogens-resolved",
                    "detail": "Resolved from the selected ChemSema molecular fragment before NMR prediction."
                }],
                "interactions": [],
            },
            "assignedCipDescriptors": assigned_cip_descriptors,
            "nucleus": nucleus,
            "conditions": {
                "solvent": "CDCl3",
                "frequencyMHz": 400.0,
                "temperatureKelvin": 298.15,
            }
        })
        .to_string())
    }

    /// Builds a new, editable ChemDraw-style NMR result document without
    /// mutating the source document or its selection.
    pub fn nmr_result_document_json(&self, response_json: &str) -> Result<String, String> {
        let response: PredictionResponse =
            serde_json::from_str(response_json).map_err(|error| error.to_string())?;
        validate_prediction_response(&response)?;
        let (source_object, mut fragment) = self.selected_single_molecule_fragment()?;
        validate_assignment_atom_ids(&fragment, &response)?;

        normalize_result_fragment(&mut fragment);
        attach_assignments(&mut fragment, &response);
        let molecule_width = (fragment.bbox[2] - fragment.bbox[0]).max(1.0);
        let molecule_height = (fragment.bbox[3] - fragment.bbox[1]).max(1.0);
        let molecule_object = SceneObject {
            id: "obj_nmr_molecule".to_string(),
            object_type: "molecule".to_string(),
            name: "molecule".to_string(),
            visible: true,
            locked: false,
            z_index: 2,
            transform: Transform {
                translate: [MOLECULE_LEFT, MOLECULE_TOP],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: source_object.style_ref.clone(),
            meta: json!({
                "nmrPrediction": {
                    "sourceMoleculeObjectId": source_object.id,
                    "moleculeId": response.molecule_id,
                }
            }),
            payload: ObjectPayload {
                resource_ref: Some("mol_nmr_result".to_string()),
                bbox: Some([0.0, 0.0, molecule_width, molecule_height]),
                spectrum: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        };

        let title = nucleus_title(response.nucleus);
        let protocol = prediction_protocol(&response);
        let mut document = ChemSemaDocument::blank();
        document.document.id = "doc_nmr_prediction".to_string();
        document.document.title = title.clone();
        document.document.page.width = RESULT_PAGE_WIDTH;
        document.document.page.height = RESULT_PAGE_HEIGHT;
        document.document.meta = json!({
            "createdBy": "chemsema",
            "kind": "nmr-prediction-result",
            "prediction": {
                "schema": response.schema,
                "engineVersion": response.engine_version,
                "ruleSetVersion": response.rule_set_version,
                "moleculeId": response.molecule_id,
                "nucleus": response.nucleus,
                "conditions": {
                    "solvent": response.conditions.solvent,
                    "frequencyMHz": response.conditions.frequency_mhz,
                    "temperatureKelvin": response.conditions.temperature_kelvin,
                }
            }
        });
        document.styles = self.state.document.styles.clone();
        document.style = self.state.document.style.clone();
        document.resources.clear();
        document.resources.insert(
            "mol_nmr_result".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemsema.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(fragment),
                meta: Value::Null,
            },
        );
        document.objects = vec![
            title_object(response.nucleus),
            molecule_object,
            quality_legend_object(),
            spectrum_object(&response),
            text_object(
                "obj_nmr_protocol",
                SPECTRUM_LEFT,
                327.10,
                356.46,
                protocol_height(&protocol),
                10,
                &protocol,
                vec![plain_run(&protocol, 9.0, "#000000")],
                9.0,
                10.35,
            ),
        ];
        serde_json::to_string(&document).map_err(|error| error.to_string())
    }

    pub(super) fn selected_single_molecule_fragment(
        &self,
    ) -> Result<(&SceneObject, MoleculeFragment), String> {
        if self.state.selection.molecule_objects.len() > 1
            || !self.state.selection.text_objects.is_empty()
            || !self.state.selection.arrow_objects.is_empty()
        {
            return Err("select exactly one complete, connected molecule".to_string());
        }
        let selected_object_ids = self
            .state
            .selection
            .molecule_objects
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let selected_node_ids = self
            .state
            .selection
            .nodes
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let selected_bond_ids = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
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
            if nodes.is_empty() {
                continue;
            }
            if entry.fragment.bonds.iter().any(|bond| {
                nodes.contains(bond.begin.as_str()) != nodes.contains(bond.end.as_str())
            }) {
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
            if crate::molecule_fragment_connected_components(&fragment).len() != 1 {
                continue;
            }
            candidates.push((entry.object, fragment));
        }
        if candidates.len() != 1 {
            return Err("select exactly one complete, connected molecule".to_string());
        }
        Ok(candidates.remove(0))
    }
}

fn validate_prediction_response(response: &PredictionResponse) -> Result<(), String> {
    if response.schema != "chemsema.nmr-prediction-response.v2" {
        return Err(format!(
            "unsupported NMR prediction response schema '{}'",
            response.schema
        ));
    }
    if response.status == PredictionStatus::Unsupported {
        return Err("NMR prediction is unsupported for this molecule".to_string());
    }
    if response.nucleus == NmrNucleus::Unknown {
        return Err("NMR prediction response must identify 1H or 13C".to_string());
    }
    if response.assignments.is_empty() {
        return Err("NMR prediction response contains no assignments".to_string());
    }
    if !response.conditions.frequency_mhz.is_finite()
        || !response.conditions.temperature_kelvin.is_finite()
    {
        return Err("NMR prediction conditions must be finite".to_string());
    }
    for assignment in &response.assignments {
        if assignment.atom_ids.is_empty()
            || assignment.site_ids.is_empty()
            || !assignment.shift_ppm.is_finite()
            || !assignment.integral.is_finite()
            || assignment.confidence_reason.trim().is_empty()
        {
            return Err("NMR prediction contains an invalid assignment".to_string());
        }
    }
    for coupling in &response.couplings {
        if coupling.site_ids.iter().any(|id| id.trim().is_empty())
            || coupling.atom_ids.iter().any(|id| id.trim().is_empty())
            || coupling
                .nuclei
                .iter()
                .any(|nucleus| !matches!(nucleus.as_str(), "1H" | "19F"))
            || !coupling.value_hz.is_finite()
        {
            return Err("NMR prediction contains an invalid coupling".to_string());
        }
    }
    for peak in &response.peaks {
        if peak
            .assignment_indexes
            .iter()
            .any(|index| *index >= response.assignments.len())
        {
            return Err("NMR peak refers to an unknown assignment".to_string());
        }
    }
    Ok(())
}

fn validate_assignment_atom_ids(
    fragment: &MoleculeFragment,
    response: &PredictionResponse,
) -> Result<(), String> {
    let node_ids = fragment
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    for id in response
        .assignments
        .iter()
        .flat_map(|assignment| assignment.atom_ids.iter())
        .chain(
            response
                .couplings
                .iter()
                .flat_map(|coupling| coupling.atom_ids.iter()),
        )
    {
        if !node_ids.contains(id.as_str()) {
            return Err(format!(
                "NMR assignment atom '{}' is not in the selected molecule",
                id
            ));
        }
    }
    Ok(())
}

fn normalize_result_fragment(fragment: &mut MoleculeFragment) {
    let min_x = fragment
        .nodes
        .iter()
        .map(|node| node.position[0])
        .fold(f64::INFINITY, f64::min);
    let min_y = fragment
        .nodes
        .iter()
        .map(|node| node.position[1])
        .fold(f64::INFINITY, f64::min);
    let max_x = fragment
        .nodes
        .iter()
        .map(|node| node.position[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = fragment
        .nodes
        .iter()
        .map(|node| node.position[1])
        .fold(f64::NEG_INFINITY, f64::max);
    for node in &mut fragment.nodes {
        node.position[0] = round2(node.position[0] - min_x);
        node.position[1] = round2(node.position[1] - min_y);
        if let Some(label) = &mut node.label {
            crate::translate_node_label_geometry(label, -min_x, -min_y);
        }
        node.nmr_assignments.clear();
    }
    fragment.bbox = [0.0, 0.0, (max_x - min_x).max(1.0), (max_y - min_y).max(1.0)];
}

fn attach_assignments(fragment: &mut MoleculeFragment, response: &PredictionResponse) {
    let center = [
        (fragment.bbox[0] + fragment.bbox[2]) * 0.5,
        (fragment.bbox[1] + fragment.bbox[3]) * 0.5,
    ];
    for assignment in &response.assignments {
        for atom_id in &assignment.atom_ids {
            let Some(node) = fragment.nodes.iter_mut().find(|node| node.id == *atom_id) else {
                continue;
            };
            let index = node.nmr_assignments.len();
            let label = assignment_label(
                node.position,
                center,
                index,
                response.nucleus,
                assignment.shift_ppm,
                assignment.confidence,
            );
            node.nmr_assignments.push(NmrAssignment {
                nucleus: response.nucleus,
                shift_ppm: assignment.shift_ppm,
                range_low_ppm: assignment.shift_ppm,
                range_high_ppm: assignment.shift_ppm,
                quality: assignment.confidence,
                label,
            });
        }
    }
}

fn assignment_label(
    point: [f64; 2],
    center: [f64; 2],
    stack_index: usize,
    nucleus: NmrNucleus,
    shift: f64,
    quality: NmrAssignmentQuality,
) -> NodeLabel {
    let text = match nucleus {
        NmrNucleus::Carbon13 => format!("{shift:.1}"),
        _ => format!("{shift:.2}"),
    };
    let fill = quality_color(quality).to_string();
    let dx = point[0] - center[0];
    let dy = point[1] - center[1];
    let horizontal = dx.abs() >= dy.abs() && dx.abs() > 0.01;
    let offset = stack_index as f64 * 8.25;
    let (position, align) = if horizontal && dx < 0.0 {
        ([point[0] + 0.14, point[1] + 8.58 + offset], "right")
    } else if horizontal {
        ([point[0] - 0.14, point[1] + 8.58 + offset], "left")
    } else if dy > 0.0 {
        ([point[0], point[1] + 8.58 + offset], "center")
    } else {
        ([point[0], point[1] - 0.14 - offset], "center")
    };
    let width = (text.chars().count() as f64 * 4.1).max(8.0);
    let bbox = match align {
        "right" => [
            position[0] - width,
            position[1] - 8.65,
            position[0],
            position[1],
        ],
        "center" => [
            position[0] - width * 0.5,
            position[1] - 8.65,
            position[0] + width * 0.5,
            position[1],
        ],
        _ => [
            position[0],
            position[1] - 8.65,
            position[0] + width,
            position[1],
        ],
    };
    NodeLabel {
        text: text.clone(),
        source_text: Some(text.clone()),
        position: Some(position),
        box_field: Some(bbox),
        runs: vec![plain_run(&text, 7.5, &fill)],
        line_runs: Vec::new(),
        lines: Vec::new(),
        align: Some(align.to_string()),
        layout: None,
        attachment: None,
        anchor: None,
        font_family: Some("Arial".to_string()),
        fill: Some(fill),
        font_size: Some(7.5),
        line_height: Some(8.625),
        line_height_mode: "auto".to_string(),
        line_advances: Vec::new(),
        glyph_polygons: Vec::new(),
        glyph_clip_polygons: Vec::new(),
        box_value: None,
        meta: json!({"interpretChemically": false}),
    }
}

fn title_object(nucleus: NmrNucleus) -> SceneObject {
    let isotope = if nucleus == NmrNucleus::Carbon13 {
        "13"
    } else {
        "1"
    };
    let symbol = if nucleus == NmrNucleus::Carbon13 {
        "C"
    } else {
        "H"
    };
    let text = format!("ChemNMR {isotope}{symbol} Estimation");
    let runs = vec![
        plain_run("ChemNMR ", 12.0, "#000000"),
        LabelRun {
            text: isotope.to_string(),
            script: Some("superscript".to_string()),
            ..plain_run("", 12.0, "#000000")
        },
        plain_run(&format!("{symbol} Estimation"), 12.0, "#000000"),
    ];
    text_object(
        "obj_nmr_title",
        25.0,
        16.05,
        145.0,
        16.95,
        1,
        &text,
        runs,
        12.0,
        13.8,
    )
}

fn quality_legend_object() -> SceneObject {
    let text = "Estimation quality is indicated by color: good, medium, rough";
    let runs = vec![
        plain_run(
            "Estimation quality is indicated by color: ",
            10.0,
            "#000000",
        ),
        plain_run("good", 10.0, "#0000ff"),
        plain_run(", ", 10.0, "#000000"),
        plain_run("medium", 10.0, "#ff00ff"),
        plain_run(", ", 10.0, "#000000"),
        plain_run("rough", 10.0, "#ff0000"),
    ];
    text_object(
        "obj_nmr_quality",
        SPECTRUM_LEFT,
        96.50,
        269.03,
        11.5,
        9,
        text,
        runs,
        10.0,
        11.5,
    )
}

fn spectrum_object(response: &PredictionResponse) -> SceneObject {
    let spectrum = simulated_spectrum(response);
    SceneObject {
        id: "obj_nmr_spectrum".to_string(),
        object_type: "spectrum".to_string(),
        name: "spectrum".to_string(),
        visible: true,
        locked: false,
        z_index: 8,
        transform: Transform {
            translate: [SPECTRUM_LEFT, SPECTRUM_TOP],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: json!({"nmrPrediction": true}),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: Some([0.0, 0.0, SPECTRUM_WIDTH, SPECTRUM_HEIGHT]),
            spectrum: Some(spectrum),
            extra: BTreeMap::new(),
        },
        children: Vec::new(),
    }
}

fn simulated_spectrum(response: &PredictionResponse) -> SpectrumData {
    let max_shift = response
        .peaks
        .iter()
        .flat_map(|peak| peak.line_positions_ppm.iter().copied())
        .chain(
            response
                .assignments
                .iter()
                .map(|assignment| assignment.shift_ppm),
        )
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::max);
    let (x_high, x_spacing) = if response.nucleus == NmrNucleus::Carbon13 {
        (
            ((max_shift * 1.1 + 0.8) * 10.0).ceil().max(30.0) / 10.0,
            0.1,
        )
    } else {
        ((max_shift + 0.25).ceil().max(3.0), 0.0)
    };
    let x_spacing = if x_spacing == 0.0 {
        x_high / 12_000.0
    } else {
        x_spacing
    };
    let count = (x_high / x_spacing).floor() as usize + 1;
    let mut data = vec![0.0; count];
    for peak in &response.peaks {
        let positions = if peak.line_positions_ppm.is_empty() {
            vec![peak.center_ppm]
        } else {
            peak.line_positions_ppm.clone()
        };
        for (line_index, position) in positions.iter().enumerate() {
            let intensity = peak
                .line_intensities
                .get(line_index)
                .copied()
                .unwrap_or(peak.intensity)
                .max(0.0);
            if response.nucleus == NmrNucleus::Carbon13 {
                let index = (*position / x_spacing).round() as isize;
                if let Some(value) = usize::try_from(index)
                    .ok()
                    .and_then(|index| data.get_mut(index))
                {
                    *value += intensity;
                }
            } else {
                let half_width = 0.0025;
                let center_index = (*position / x_spacing).round() as isize;
                let radius = (half_width * 20.0 / x_spacing).ceil() as isize;
                for index in center_index - radius..=center_index + radius {
                    let Ok(index) = usize::try_from(index) else {
                        continue;
                    };
                    let Some(value) = data.get_mut(index) else {
                        continue;
                    };
                    let x = index as f64 * x_spacing;
                    let ratio = (x - position) / half_width;
                    *value += intensity / (1.0 + ratio * ratio);
                }
            }
        }
    }
    if response.peaks.is_empty() {
        for assignment in &response.assignments {
            let index = (assignment.shift_ppm / x_spacing).round() as usize;
            if let Some(value) = data.get_mut(index) {
                *value += assignment.integral.max(0.0);
            }
        }
    }
    SpectrumData {
        class: SpectrumClass::Nmr,
        x_low: 0.0,
        x_spacing,
        x_type: SpectrumXAxisType::PartsPerMillion,
        x_axis_label: "PPM".to_string(),
        y_low: 0.0,
        y_scale: 1.0,
        y_type: SpectrumYAxisType::Unknown,
        y_axis_label: String::new(),
        data_points: data,
    }
}

fn prediction_protocol(response: &PredictionResponse) -> String {
    let nucleus = if response.nucleus == NmrNucleus::Carbon13 {
        "C-13"
    } else {
        "H-1"
    };
    let mut lines = vec![
        format!(
            "Protocol of the {nucleus} NMR Prediction (Solvent={} {:.0} MHz):",
            response.conditions.solvent, response.conditions.frequency_mhz
        ),
        String::new(),
        "Atom     Shift    Contribution   Rule".to_string(),
        String::new(),
    ];
    for assignment in &response.assignments {
        lines.push(format!(
            "{}  {:>8.2}",
            assignment.atom_ids.join(","),
            assignment.shift_ppm
        ));
        for contribution in &assignment.contributions {
            lines.push(format!(
                "             {:>8.2}   {} ({}, {})",
                contribution.value_ppm,
                contribution.role,
                contribution.rule_id,
                contribution.source_id
            ));
        }
        if assignment.contributions.is_empty() {
            lines.push(format!(
                "                        {}",
                assignment.equivalence_class
            ));
        }
        lines.push(format!(
            "                        confidence: {}",
            assignment.confidence_reason
        ));
    }
    if !response.couplings.is_empty() {
        lines.extend([
            String::new(),
            "H-1 NMR Coupling Constant Prediction".to_string(),
        ]);
        for coupling in &response.couplings {
            lines.push(format!(
                "{}({}:{}) - {}({}:{})  {:.2} Hz  {}",
                coupling.site_ids[0],
                coupling.nuclei[0],
                coupling.atom_ids[0],
                coupling.site_ids[1],
                coupling.nuclei[1],
                coupling.atom_ids[1],
                coupling.value_hz,
                coupling.rule_id
            ));
        }
    }
    for diagnostic in &response.diagnostics {
        lines.push(format!(
            "{}: {} [{}]",
            diagnostic.code,
            diagnostic.message,
            diagnostic.atom_ids.join(",")
        ));
    }
    lines.join("\n")
}

fn protocol_height(text: &str) -> f64 {
    text.lines().count().max(1) as f64 * 10.35 + 3.0
}

#[allow(clippy::too_many_arguments)]
fn text_object(
    id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    z_index: i32,
    text: &str,
    runs: Vec<LabelRun>,
    font_size: f64,
    line_height: f64,
) -> SceneObject {
    let mut extra = BTreeMap::new();
    extra.insert("text".to_string(), json!(text));
    extra.insert("align".to_string(), json!("left"));
    extra.insert("valign".to_string(), json!("top"));
    extra.insert("preserveLines".to_string(), json!(true));
    extra.insert("fontFamily".to_string(), json!("Arial"));
    extra.insert("fontSize".to_string(), json!(font_size));
    extra.insert("fill".to_string(), json!("#000000"));
    extra.insert("lineHeight".to_string(), json!(line_height));
    extra.insert("lineHeightMode".to_string(), json!("auto"));
    extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
    extra.insert("runs".to_string(), json!(runs));
    extra.insert("sourceRuns".to_string(), json!(runs));
    SceneObject {
        id: id.to_string(),
        object_type: "text".to_string(),
        name: "text".to_string(),
        visible: true,
        locked: false,
        z_index,
        transform: Transform {
            translate: [x, y],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        meta: json!({"interpretChemically": false}),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: Some([0.0, 0.0, width, height]),
            spectrum: None,
            extra,
        },
        children: Vec::new(),
    }
}

fn plain_run(text: &str, size: f64, fill: &str) -> LabelRun {
    LabelRun {
        text: text.to_string(),
        font_family: Some("Arial".to_string()),
        font_size: Some(size),
        fill: Some(fill.to_string()),
        font_weight: Some(400),
        font_style: Some("normal".to_string()),
        underline: Some(false),
        outline: Some(false),
        shadow: Some(false),
        script: Some("normal".to_string()),
    }
}

fn nucleus_title(nucleus: NmrNucleus) -> String {
    match nucleus {
        NmrNucleus::Hydrogen1 => "ChemNMR 1H Estimation".to_string(),
        NmrNucleus::Carbon13 => "ChemNMR 13C Estimation".to_string(),
        NmrNucleus::Unknown => "ChemNMR Estimation".to_string(),
    }
}

fn quality_color(quality: NmrAssignmentQuality) -> &'static str {
    match quality {
        NmrAssignmentQuality::Good => "#0000ff",
        NmrAssignmentQuality::Medium => "#ff00ff",
        NmrAssignmentQuality::Rough => "#ff0000",
        NmrAssignmentQuality::Unknown => "#000000",
    }
}
