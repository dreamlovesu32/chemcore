use super::*;

pub(crate) fn targets_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected targets argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "targets requires an input file.".to_string())?;
    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    write_json_value(targets_report(&input, &document), output.as_deref(), pretty)
}

pub(super) fn targets_report(input: &str, document: &ChemcoreDocument) -> Value {
    let objects = object_target_entries(document);
    let molecules = molecule_target_entries(document);
    let nodes = node_target_entries(document);
    let bonds = bond_target_entries(document);
    let target_count = 1 + objects.len() + molecules.len() + nodes.len() + bonds.len();
    let all_bounds = target_bounds_fast(document, &TargetSelector::All)
        .or_else(|| target_bounds(document, &TargetSelector::All).ok());
    json!({
        "ok": true,
        "input": input,
        "targetCount": target_count,
        "targets": {
            "all": {
                "selector": "all",
                "bounds": all_bounds.map(bounds_json),
            },
            "objects": objects,
            "molecules": molecules,
            "nodes": nodes,
            "bonds": bonds,
        }
    })
}

pub(super) fn object_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    collect_object_target_entries(document, &document.objects, None, 0, &mut entries);
    entries
}

pub(super) fn collect_object_target_entries(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    parent_id: Option<&str>,
    depth: usize,
    entries: &mut Vec<Value>,
) {
    for object in objects {
        let bounds = target_bounds_fast(document, &TargetSelector::Object(object.id.clone()))
            .or_else(|| target_bounds(document, &TargetSelector::Object(object.id.clone())).ok())
            .map(bounds_json);
        entries.push(json!({
            "selector": format!("object:{}", object.id),
            "id": object.id,
            "type": object.object_type,
            "name": object.name,
            "visible": object.visible,
            "locked": object.locked,
            "zIndex": object.z_index,
            "parentId": parent_id,
            "depth": depth,
            "resourceRef": object.payload.resource_ref,
            "children": object.children.len(),
            "bounds": bounds,
        }));
        collect_object_target_entries(
            document,
            &object.children,
            Some(object.id.as_str()),
            depth + 1,
            entries,
        );
    }
}

pub(super) fn molecule_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    document
        .editable_fragments()
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let bounds = target_bounds_fast(document, &TargetSelector::Molecule(index))
                .or_else(|| target_bounds(document, &TargetSelector::Molecule(index)).ok())
                .map(bounds_json);
            json!({
                "selector": format!("molecule:{index}"),
                "index": index,
                "objectId": entry.object.id,
                "resourceRef": entry.object.payload.resource_ref,
                "nodeCount": entry.fragment.nodes.len(),
                "bondCount": entry.fragment.bonds.len(),
                "bounds": bounds,
            })
        })
        .collect()
}

pub(super) fn node_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        for node in &entry.fragment.nodes {
            let position = world_node_position(entry.object, node);
            entries.push(json!({
                "selector": format!("node:{}", node.id),
                "id": node.id,
                "moleculeIndex": molecule_index,
                "objectId": entry.object.id,
                "element": node.element,
                "atomicNumber": node.atomic_number,
                "position": [position[0], position[1]],
                "hasLabel": node.label.as_ref().is_some_and(|label| label.has_visible_text()),
                "bounds": bounds_json(node_fast_bounds(entry.object, node)),
                "boundsSource": "geometry-fast",
            }));
        }
    }
    entries
}

pub(super) fn bond_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        for bond in &entry.fragment.bonds {
            entries.push(json!({
                "selector": format!("bond:{}", bond.id),
                "id": bond.id,
                "moleculeIndex": molecule_index,
                "objectId": entry.object.id,
                "begin": bond.begin,
                "end": bond.end,
                "order": bond.order,
                "bounds": bond_fast_bounds(entry.object, &entry.fragment.nodes, bond).map(bounds_json),
                "boundsSource": "geometry-fast",
            }));
        }
    }
    entries
}
