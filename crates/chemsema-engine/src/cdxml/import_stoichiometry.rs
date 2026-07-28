use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::cdxml) fn import_reactions_and_stoichiometry_grids(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Vec<crate::ReactionSchemeData> {
    let source_map = source_entity_map(objects);
    let mut schemes = Vec::new();
    for (scheme_index, scheme_node) in descendants(root)
        .into_iter()
        .filter(|node| node.is("scheme"))
        .enumerate()
    {
        let mut scheme = crate::ReactionSchemeData {
            id: scheme_node
                .attr("id")
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("reaction_scheme_{}", scheme_index + 1)),
            steps: Vec::new(),
        };
        for (step_index, step_node) in scheme_node.direct_children("step").enumerate() {
            scheme
                .steps
                .push(import_reaction_step(step_node, step_index, &source_map));
        }
        if !scheme.steps.is_empty() {
            schemes.push(scheme);
        }
    }
    if schemes.is_empty() {
        let steps = descendants(root)
            .into_iter()
            .filter(|node| node.is("step"))
            .enumerate()
            .map(|(index, node)| import_reaction_step(node, index, &source_map))
            .collect::<Vec<_>>();
        if !steps.is_empty() {
            schemes.push(crate::ReactionSchemeData {
                id: "reaction_scheme_1".to_string(),
                steps,
            });
        }
    }
    let steps = schemes
        .iter()
        .flat_map(|scheme| scheme.steps.iter())
        .collect::<Vec<_>>();
    append_stoichiometry_grids(root, objects, &source_map, &steps, defaults, colors, fonts);
    schemes
}

fn import_reaction_step(
    node: &XmlNode,
    index: usize,
    source_map: &BTreeMap<String, String>,
) -> crate::ReactionStepData {
    let reactant_entity_ids = mapped_id_list(node.attr("ReactionStepReactants"), source_map);
    let product_entity_ids = mapped_id_list(node.attr("ReactionStepProducts"), source_map);
    let arrow_object_ids = mapped_id_list(node.attr("ReactionStepArrows"), source_map);
    let interpretation_state = if reactant_entity_ids.is_empty()
        || product_entity_ids.is_empty()
        || arrow_object_ids.is_empty()
    {
        crate::ReactionInterpretationState::Invalid
    } else {
        crate::ReactionInterpretationState::Current
    };
    crate::ReactionStepData {
        id: node
            .attr("id")
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("reaction_step_{}", index + 1)),
        link_policy: crate::LinkPolicy::Linked,
        binding_origin: crate::LogicalBindingOrigin::Imported,
        reactant_entity_ids,
        product_entity_ids,
        arrow_object_ids,
        plus_object_ids: mapped_id_list(node.attr("ReactionStepPlusses"), source_map),
        objects_above_arrow: mapped_id_list(node.attr("ReactionStepObjectsAboveArrow"), source_map),
        objects_below_arrow: mapped_id_list(node.attr("ReactionStepObjectsBelowArrow"), source_map),
        atom_mappings: import_atom_mappings(node, source_map),
        interpretation_state,
    }
}

fn import_atom_mappings(
    node: &XmlNode,
    source_map: &BTreeMap<String, String>,
) -> Vec<crate::ReactionAtomMapping> {
    let (value, origin) = if let Some(value) = node.attr("ReactionStepAtomMapManual") {
        (value, crate::ReactionAtomMappingOrigin::Manual)
    } else if let Some(value) = node.attr("ReactionStepAtomMapAuto") {
        (value, crate::ReactionAtomMappingOrigin::Automatic)
    } else if let Some(value) = node.attr("ReactionStepAtomMap") {
        (value, crate::ReactionAtomMappingOrigin::Imported)
    } else {
        return Vec::new();
    };
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .chunks_exact(2)
        .filter_map(|pair| {
            Some(crate::ReactionAtomMapping {
                reactant_atom_id: source_map.get(pair[0])?.clone(),
                product_atom_id: source_map.get(pair[1])?.clone(),
                origin,
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn append_stoichiometry_grids(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    source_map: &BTreeMap<String, String>,
    steps: &[&crate::ReactionStepData],
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) {
    let mut next_z = objects
        .iter()
        .map(|object| object.z_index)
        .max()
        .unwrap_or(10)
        + 1;
    for (grid_index, node) in descendants(root)
        .into_iter()
        .filter(|node| node.is("stoichiometrygrid"))
        .enumerate()
    {
        let bounds = parse_bbox(node.attr("BoundingBox")).unwrap_or([
            80.0,
            80.0,
            80.0 + 260.0,
            80.0 + 180.0,
        ]);
        let mut components = Vec::new();
        let mut rows = Vec::new();
        let mut data = Vec::new();
        for (component_index, component_node) in node.direct_children("sgcomponent").enumerate() {
            let source_reference = component_node.attr("ComponentReferenceID");
            let resolved_reference =
                source_reference.and_then(|source| source_map.get(source).cloned());
            let component_id = component_node
                .attr("id")
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    format!("sg_component_{}_{}", grid_index + 1, component_index + 1)
                });
            let is_header = parse_bool(component_node.attr("ComponentIsHeader"), false);
            let role = if is_header {
                crate::StoichiometryComponentRole::Header
            } else if parse_bool(component_node.attr("ComponentIsReactant"), false) {
                crate::StoichiometryComponentRole::Reactant
            } else if resolved_reference.as_ref().is_some_and(|reference| {
                steps
                    .iter()
                    .any(|step| step.product_entity_ids.contains(reference))
            }) {
                crate::StoichiometryComponentRole::Product
            } else {
                crate::StoichiometryComponentRole::Reagent
            };
            components.push(crate::StoichiometryComponent {
                id: component_id.clone(),
                role,
                reference_entity_id: resolved_reference,
                unresolved_reference_id: source_reference
                    .filter(|source| !source_map.contains_key(*source))
                    .map(ToString::to_string),
                is_header,
                visible: parse_bool(component_node.attr("Visible"), true),
                width: parse_number(component_node.attr("Width")).unwrap_or(72.0),
            });
            for (datum_index, datum_node) in component_node.direct_children("sgdatum").enumerate() {
                let property_type = datum_node
                    .attr("SGPropertyType")
                    .unwrap_or("Unspecified")
                    .to_string();
                let data_type = datum_node.attr("SGDataType").unwrap_or("Text").to_string();
                let row_id = rows
                    .iter()
                    .find(|row: &&crate::StoichiometryRow| {
                        row.property_type == property_type && row.data_type == data_type
                    })
                    .map(|row| row.id.clone())
                    .unwrap_or_else(|| {
                        let row_id = format!("sg_row_{}_{}", grid_index + 1, rows.len() + 1);
                        rows.push(crate::StoichiometryRow {
                            id: row_id.clone(),
                            property_type: property_type.clone(),
                            data_type: data_type.clone(),
                            label: property_type.clone(),
                            default_unit: None,
                            visible: true,
                            height: 18.0,
                        });
                        row_id
                    });
                let display = datum_node.attr("SGDataValue").unwrap_or("").to_string();
                data.push(crate::StoichiometryDatum {
                    id: datum_node
                        .attr("id")
                        .map(ToString::to_string)
                        .unwrap_or_else(|| {
                            format!(
                                "sg_datum_{}_{}_{}",
                                grid_index + 1,
                                component_index + 1,
                                datum_index + 1
                            )
                        }),
                    component_id: component_id.clone(),
                    row_id,
                    value: crate::StoichiometryValue {
                        canonical: numeric_prefix(&display).unwrap_or_default(),
                        display,
                        unit: None,
                    },
                    origin: if parse_bool(datum_node.attr("IsEdited"), false) {
                        crate::StoichiometryValueOrigin::Authored
                    } else {
                        crate::StoichiometryValueOrigin::Imported
                    },
                    is_edited: parse_bool(datum_node.attr("IsEdited"), false),
                    is_hidden: parse_bool(datum_node.attr("IsHidden"), false),
                    is_read_only: parse_bool(datum_node.attr("IsReadOnly"), false),
                    visible: parse_bool(datum_node.attr("Visible"), true),
                    calculation_state: crate::StoichiometryCalculationState::Current,
                });
            }
        }
        let referenced = components
            .iter()
            .filter_map(|component| component.reference_entity_id.as_ref())
            .cloned()
            .collect::<BTreeSet<_>>();
        let candidates = steps
            .iter()
            .filter(|step| {
                let members = step
                    .reactant_entity_ids
                    .iter()
                    .chain(step.product_entity_ids.iter())
                    .chain(step.objects_above_arrow.iter())
                    .chain(step.objects_below_arrow.iter())
                    .cloned()
                    .collect::<BTreeSet<_>>();
                !referenced.is_empty() && referenced.is_subset(&members)
            })
            .collect::<Vec<_>>();
        let source_reaction_step_id = (candidates.len() == 1).then(|| candidates[0].id.clone());
        let binding_state = if source_reaction_step_id.is_some() {
            crate::StoichiometryBindingState::Current
        } else if referenced.is_empty() {
            crate::StoichiometryBindingState::Detached
        } else {
            crate::StoichiometryBindingState::Unresolved
        };
        let grid = crate::StoichiometryGridData {
            source_reaction_step_id,
            binding_origin: crate::StoichiometryBindingOrigin::Imported,
            binding_state,
            anchor_mode: crate::StoichiometryAnchorMode::Fixed,
            components,
            rows,
            data,
            style: crate::StoichiometryGridStyle {
                line_width: parse_number(node.attr("LineWidth")).unwrap_or(defaults.line_width),
                bold_width: parse_number(node.attr("BoldWidth")).unwrap_or(defaults.bold_width),
                margin_width: parse_number(node.attr("MarginWidth"))
                    .unwrap_or(defaults.margin_width),
                color: colors.resolve(node.attr("color")),
                label_font: node
                    .attr("LabelFont")
                    .and_then(|id| fonts.get(id))
                    .cloned()
                    .unwrap_or_else(|| "Arial".to_string()),
                label_size: parse_number(node.attr("LabelSize")).unwrap_or(defaults.label_size),
                label_face: node
                    .attr("LabelFace")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
            },
        };
        objects.push(SceneObject {
            id: format!("obj_stoichiometry_grid_{:03}", grid_index + 1),
            object_type: "stoichiometry-grid".to_string(),
            name: format!("stoichiometry grid {}", grid_index + 1),
            visible: parse_bool(node.attr("Visible"), true),
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(next_z),
            transform: Transform {
                translate: [round2(bounds[0]), round2(bounds[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: crate::LinkPolicy::Auto,
            meta: json!({
                "source": "cdxml",
                "stoichiometryGridId": node.attr("id")
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bounds[2] - bounds[0]),
                    round2(bounds[3] - bounds[1]),
                ]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: Some(grid),
                gel_electrophoresis: None,
                plasmid_map: None,
                bio_shape: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        });
        next_z += 1;
    }
}

fn source_entity_map(objects: &[SceneObject]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for object in objects {
        for source in [
            object.meta.get("fragmentId").and_then(Value::as_str),
            object.meta.get("graphicId").and_then(Value::as_str),
            object.meta.get("curveId").and_then(Value::as_str),
            object
                .meta
                .pointer("/import/cdxml/sourceId")
                .and_then(Value::as_str),
            object
                .meta
                .pointer("/import/cdxml/fragmentId")
                .and_then(Value::as_str),
        ]
        .into_iter()
        .flatten()
        {
            out.entry(source.to_string())
                .or_insert_with(|| object.id.clone());
        }
    }
    out
}

fn mapped_id_list(value: Option<&str>, source_map: &BTreeMap<String, String>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter_map(|id| source_map.get(id).cloned())
        .collect()
}

fn parse_bbox(value: Option<&str>) -> Option<[f64; 4]> {
    let values = value?
        .split_whitespace()
        .filter_map(|value| value.parse::<f64>().ok())
        .collect::<Vec<_>>();
    (values.len() == 4).then_some([values[0], values[1], values[2], values[3]])
}

fn parse_number(value: Option<&str>) -> Option<f64> {
    value?.parse().ok()
}

fn parse_bool(value: Option<&str>, default: bool) -> bool {
    match value {
        Some("yes" | "true" | "1") => true,
        Some("no" | "false" | "0") => false,
        _ => default,
    }
}

fn numeric_prefix(value: &str) -> Option<String> {
    let token = value.split_whitespace().next()?;
    token.parse::<f64>().ok().map(|_| token.to_string())
}
