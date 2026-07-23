use super::*;

pub(super) fn normalize_fragment(
    fragment: &XmlNode,
    bbox: [f64; 4],
    node_positions: &BTreeMap<String, [f64; 2]>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Option<MoleculeFragment> {
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
    if nodes.is_empty() {
        return None;
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
    apply_cdxml_carbon_label_display(&mut fragment, defaults, fonts);
    crate::engine::refresh_attached_node_label_geometry_for_all_nodes_with_profile(
        &mut fragment,
        origin,
        defaults.line_width,
        Some(crate::GlyphClipProfile::from_margin_width(
            defaults.margin_width,
        )),
    );
    infer_cdxml_ring_double_bond_placements(&mut fragment);
    Some(fragment)
}

fn apply_cdxml_carbon_label_display(
    fragment: &mut MoleculeFragment,
    defaults: CdxmlDefaults,
    fonts: &BTreeMap<String, String>,
) {
    let plans: Vec<(String, u8)> = fragment
        .nodes
        .iter()
        .filter(|node| node.atomic_number == 6 && node.label.is_none() && !node.is_placeholder)
        .filter_map(|node| {
            let bond_count = fragment
                .bonds
                .iter()
                .filter(|bond| bond.begin == node.id || bond.end == node.id)
                .count();
            let show = if bond_count <= 1 {
                node.atom_properties
                    .show_terminal_carbon_label
                    .unwrap_or(defaults.show_terminal_carbon_labels)
            } else {
                node.atom_properties
                    .show_non_terminal_carbon_label
                    .unwrap_or(defaults.show_non_terminal_carbon_labels)
            };
            show.then(|| {
                (
                    node.id.clone(),
                    crate::engine::formula_hydrogen_count_for_node(fragment, &node.id),
                )
            })
        })
        .collect();
    for (node_id, hydrogen_count) in plans {
        let Some(node) = fragment.nodes.iter_mut().find(|node| node.id == node_id) else {
            continue;
        };
        node.num_hydrogens = hydrogen_count;
        let text = crate::engine::implicit_hydrogen_label_text_for_count("C", hydrogen_count);
        let mut label = crate::engine::make_periodic_element_node_label(&text, node.position);
        label.font_size = Some(defaults.label_size);
        label.font_family = fonts.get(&defaults.label_font.to_string()).cloned();
        label.meta = json!({"carbonDisplayLabel": {"source": "cdxml-generated"}});
        node.label = Some(label);
    }
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
