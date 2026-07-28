use super::*;

pub(super) fn normalize_fragment(
    fragment: &XmlNode,
    bbox: [f64; 4],
    node_positions: &BTreeMap<String, [f64; 2]>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Result<Option<MoleculeFragment>, String> {
    let origin = [bbox[0], bbox[1]];
    let nodes: Vec<Node> = fragment
        .direct_children("n")
        .filter_map(|node| normalize_node(node, origin, node_positions, colors, fonts, defaults))
        .collect();
    let node_ids: BTreeSet<String> = nodes.iter().map(|node| node.id.clone()).collect();
    let bonds: Vec<Bond> = fragment
        .direct_children("b")
        .enumerate()
        .filter_map(|(index, bond)| {
            normalize_bond(bond, index, &node_ids, &nodes, defaults, colors)
        })
        .collect();
    let known_bonds = bonds
        .iter()
        .map(|bond| bond.id.as_str())
        .collect::<BTreeSet<_>>();
    let colored_areas = fragment
        .children
        .iter()
        .filter(|area| area.is("ColoredMolecularArea") || area.is("coloredmoleculararea"))
        .enumerate()
        .map(|(index, area)| {
            let basis_bonds = area
                .attr("BasisObjects")
                .ok_or_else(|| "ColoredMolecularArea is missing BasisObjects.".to_string())?
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>();
            if basis_bonds.is_empty()
                || !basis_bonds
                    .iter()
                    .all(|bond_id| known_bonds.contains(bond_id.as_str()))
            {
                return Err(format!(
                    "ColoredMolecularArea '{}' references a missing or non-bond basis object.",
                    area.attr("id").unwrap_or("<missing id>")
                ));
            }
            Ok(crate::ColoredMolecularArea {
                id: area
                    .attr("id")
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("colored_area_{}", index + 1)),
                color: colors.resolve(area.attr("bgcolor")),
                basis_bonds,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if nodes.is_empty() {
        return Ok(None);
    }
    let mut fragment = MoleculeFragment {
        schema: "chemsema.molecule.fragment2d".to_string(),
        bbox: [
            0.0,
            0.0,
            round2(bbox[2] - bbox[0]),
            round2(bbox[3] - bbox[1]),
        ],
        nodes,
        bonds,
        colored_areas,
        stereo: Vec::new(),
        interactions: Vec::new(),
        meta: json!({
            "import": {
                "cdxml": {
                    "fragmentId": fragment.attr("id"),
                    "bboxAbs": bbox,
                    "z": parse_i32(fragment.attr("Z")),
                }
            }
        }),
    };
    crate::engine::refresh_attached_node_label_geometry_for_all_nodes_with_profile(
        &mut fragment,
        origin,
        defaults.line_width,
        Some(crate::GlyphClipProfile::from_margin_width(
            defaults.margin_width,
        )),
    );
    infer_cdxml_ring_double_bond_placements(&mut fragment);
    if let Some(area) = fragment
        .colored_areas
        .iter()
        .find(|area| crate::ordered_colored_area_node_ids(&fragment, &area.basis_bonds).is_none())
    {
        return Err(format!(
            "ColoredMolecularArea '{}' must reference exactly one connected simple ring.",
            area.id
        ));
    }
    import_native_cdxml_molecule_semantics(&mut fragment)?;
    Ok(Some(fragment))
}

fn import_native_cdxml_molecule_semantics(fragment: &mut MoleculeFragment) -> Result<(), String> {
    use chemsema_chemical_graph::{
        EnhancedStereoKindV2, InteractionCenterV2, InteractionKindV2, InteractionRoleV2,
        MultiCenterInteractionV2, StereoElementV2,
    };

    let known_nodes = fragment
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    for proxy in fragment.nodes.iter().filter(|node| {
        node.meta
            .pointer("/import/cdxml/nodeType")
            .and_then(Value::as_str)
            == Some("MultiAttachment")
    }) {
        let incident = fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == proxy.id || bond.end == proxy.id)
            .collect::<Vec<_>>();
        if incident.is_empty() {
            // ChemDraw also uses an unbonded MultiAttachment node as a
            // standalone display marker. It has no molecular relationship to
            // normalize until a proxy bond identifies the other center.
            continue;
        }
        let source = proxy
            .meta
            .pointer("/import/cdxml/attachments")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "CDXML MultiAttachment node '{}' has no Attachments list",
                    proxy.id
                )
            })?;
        let attachments = source
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let unique = attachments
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if attachments.is_empty()
            || unique.len() != attachments.len()
            || unique
                .iter()
                .any(|atom| *atom == proxy.id || !known_nodes.contains(atom))
        {
            return Err(format!(
                "CDXML MultiAttachment node '{}' has an empty, repeated, self, or missing attachment",
                proxy.id
            ));
        }
        let mut acceptors = BTreeSet::new();
        for bond in incident {
            let acceptor = if bond.begin == proxy.id {
                bond.end.clone()
            } else {
                bond.begin.clone()
            };
            let acceptor_is_proxy = fragment.nodes.iter().any(|node| {
                node.id == acceptor
                    && node
                        .meta
                        .pointer("/import/cdxml/nodeType")
                        .and_then(Value::as_str)
                        == Some("MultiAttachment")
            });
            if acceptor_is_proxy
                || attachments.contains(&acceptor)
                || !acceptors.insert(acceptor.clone())
            {
                return Err(format!(
                    "CDXML MultiAttachment node '{}' has an invalid or repeated acceptor '{}'",
                    proxy.id, acceptor
                ));
            }
        }
        let mut centers = vec![InteractionCenterV2 {
            role: InteractionRoleV2::Donor,
            atoms: attachments,
        }];
        centers.extend(acceptors.into_iter().map(|acceptor| InteractionCenterV2 {
            role: InteractionRoleV2::Acceptor,
            atoms: vec![acceptor],
        }));
        fragment.interactions.push(MultiCenterInteractionV2 {
            id: format!("cdxml-multi-{}", proxy.id),
            kind: InteractionKindV2::Coordination,
            centers,
        });
    }

    let mut groups = BTreeMap::<(String, u32), (EnhancedStereoKindV2, Vec<String>)>::new();
    for node in &fragment.nodes {
        let Some(source_kind) = node
            .meta
            .pointer("/import/cdxml/enhancedStereoType")
            .and_then(Value::as_str)
        else {
            continue;
        };
        let (kind_key, kind) = match source_kind.to_ascii_lowercase().as_str() {
            "absolute" => ("absolute", EnhancedStereoKindV2::Absolute),
            "and" => ("and", EnhancedStereoKindV2::And),
            "or" => ("or", EnhancedStereoKindV2::Or),
            _ => {
                return Err(format!(
                    "CDXML node '{}' has unsupported EnhancedStereoType '{}'",
                    node.id, source_kind
                ))
            }
        };
        let group_number = node
            .meta
            .pointer("/import/cdxml/enhancedStereoGroupNum")
            .and_then(Value::as_str)
            .map(|value| {
                value.parse::<u32>().map_err(|_| {
                    format!(
                        "CDXML node '{}' has invalid EnhancedStereoGroupNum '{}'",
                        node.id, value
                    )
                })
            })
            .transpose()?
            .unwrap_or(1);
        if group_number == 0 {
            return Err(format!(
                "CDXML node '{}' has zero EnhancedStereoGroupNum",
                node.id
            ));
        }
        groups
            .entry((kind_key.to_string(), group_number))
            .or_insert_with(|| (kind, Vec::new()))
            .1
            .push(format!("tetrahedral-{}", node.id));
    }
    fragment.stereo.extend(groups.into_iter().map(
        |((kind, number), (group_kind, mut members))| {
            members.sort();
            StereoElementV2::EnhancedGroup {
                id: format!("cdxml-enhanced-{kind}-{number}"),
                group_kind,
                members,
            }
        },
    ));
    Ok(())
}

pub(super) fn split_cdxml_fragment_components(
    fragment: MoleculeFragment,
    source_bbox_abs: [f64; 4],
) -> Vec<CdxmlFragmentComponent> {
    let components = crate::molecule_fragment_connected_components(&fragment);
    if components.len() <= 1 {
        return vec![CdxmlFragmentComponent {
            fragment,
            bbox_abs: source_bbox_abs,
            component_index: 0,
            component_count: 1,
        }];
    }

    let component_count = components.len();
    components
        .into_iter()
        .enumerate()
        .filter_map(|(component_index, node_ids)| {
            let mut nodes: Vec<Node> = fragment
                .nodes
                .iter()
                .filter(|node| node_ids.contains(&node.id))
                .cloned()
                .collect();
            let bonds: Vec<Bond> = fragment
                .bonds
                .iter()
                .filter(|bond| node_ids.contains(&bond.begin) && node_ids.contains(&bond.end))
                .cloned()
                .collect();
            let bond_ids = bonds
                .iter()
                .map(|bond| bond.id.clone())
                .collect::<BTreeSet<_>>();
            let (stereo, interactions) =
                crate::subset_molecule_semantics(&fragment, &node_ids, &bond_ids);
            let colored_areas = fragment
                .colored_areas
                .iter()
                .filter(|area| area.basis_bonds.iter().all(|id| bond_ids.contains(id)))
                .cloned()
                .collect();
            if !cdxml_component_has_visible_molecule_content(&nodes, &bonds) {
                return None;
            }

            let local_bounds = crate::molecule_component_bounds(&nodes).unwrap_or([
                0.0,
                0.0,
                fragment.bbox[2].max(1.0),
                fragment.bbox[3].max(1.0),
            ]);
            let delta_x = -local_bounds[0];
            let delta_y = -local_bounds[1];
            for node in &mut nodes {
                node.position[0] = round2(node.position[0] + delta_x);
                node.position[1] = round2(node.position[1] + delta_y);
                if let Some(label) = &mut node.label {
                    crate::translate_node_label_geometry(label, delta_x, delta_y);
                }
            }

            let bbox_abs = [
                round2(source_bbox_abs[0] + local_bounds[0]),
                round2(source_bbox_abs[1] + local_bounds[1]),
                round2(source_bbox_abs[0] + local_bounds[2]),
                round2(source_bbox_abs[1] + local_bounds[3]),
            ];
            let mut component_fragment = MoleculeFragment {
                schema: fragment.schema.clone(),
                bbox: [
                    0.0,
                    0.0,
                    round2((local_bounds[2] - local_bounds[0]).max(1.0)),
                    round2((local_bounds[3] - local_bounds[1]).max(1.0)),
                ],
                nodes,
                bonds,
                colored_areas,
                stereo,
                interactions,
                meta: fragment.meta.clone(),
            };
            annotate_cdxml_component_fragment_meta(
                &mut component_fragment,
                source_bbox_abs,
                bbox_abs,
                component_index,
                component_count,
            );
            Some(CdxmlFragmentComponent {
                fragment: component_fragment,
                bbox_abs,
                component_index,
                component_count,
            })
        })
        .collect()
}

pub(super) fn cdxml_component_has_visible_molecule_content(nodes: &[Node], bonds: &[Bond]) -> bool {
    !bonds.is_empty()
        || nodes.iter().any(|node| {
            node.atomic_number != 6
                || node
                    .meta
                    .pointer("/import/cdxml/nodeType")
                    .and_then(Value::as_str)
                    == Some("MultiAttachment")
                || node
                    .meta
                    .pointer("/import/cdxml/hDot")
                    .and_then(Value::as_bool)
                    == Some(true)
                || node
                    .meta
                    .pointer("/import/cdxml/hDash")
                    .and_then(Value::as_bool)
                    == Some(true)
                || node
                    .label
                    .as_ref()
                    .is_some_and(|label| label.has_visible_text())
        })
}

pub(super) fn annotate_cdxml_component_fragment_meta(
    fragment: &mut MoleculeFragment,
    source_bbox_abs: [f64; 4],
    bbox_abs: [f64; 4],
    component_index: usize,
    component_count: usize,
) {
    let Some(cdxml_meta) = fragment
        .meta
        .get_mut("import")
        .and_then(|value| value.get_mut("cdxml"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    cdxml_meta.insert("sourceFragmentBboxAbs".to_string(), json!(source_bbox_abs));
    cdxml_meta.insert("bboxAbs".to_string(), json!(bbox_abs));
    cdxml_meta.insert("componentIndex".to_string(), json!(component_index));
    cdxml_meta.insert("componentCount".to_string(), json!(component_count));
}

pub(super) fn cdxml_fragment_component_meta(
    fragment_id: Option<&str>,
    component_index: usize,
    component_count: usize,
) -> Value {
    let mut cdxml = serde_json::Map::new();
    cdxml.insert("fragmentId".to_string(), json!(fragment_id));
    if component_count > 1 {
        cdxml.insert("componentIndex".to_string(), json!(component_index));
        cdxml.insert("componentCount".to_string(), json!(component_count));
    }
    json!({
        "source": "cdxml",
        "import": { "cdxml": cdxml },
        "fragmentId": fragment_id,
    })
}
