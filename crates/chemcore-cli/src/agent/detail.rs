use super::*;

pub(crate) fn detail_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut include_raw = true;
    let mut include_resource = false;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" => {
                index += 1;
                target = Some(parse_target_selector(
                    args.get(index)
                        .ok_or_else(|| "--target requires a selector.".to_string())?,
                )?);
            }
            "--object" => {
                index += 1;
                target = Some(TargetSelector::Object(
                    args.get(index)
                        .ok_or_else(|| "--object requires an object id.".to_string())?
                        .clone(),
                ));
            }
            "--molecule" => {
                index += 1;
                target = Some(TargetSelector::Molecule(parse_usize_arg(
                    "--molecule",
                    args.get(index),
                )?));
            }
            "--node" => {
                index += 1;
                target = Some(TargetSelector::Node(
                    args.get(index)
                        .ok_or_else(|| "--node requires a node id.".to_string())?
                        .clone(),
                ));
            }
            "--bond" => {
                index += 1;
                target = Some(TargetSelector::Bond(
                    args.get(index)
                        .ok_or_else(|| "--bond requires a bond id.".to_string())?
                        .clone(),
                ));
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--summary-only" | "--no-raw" => include_raw = false,
            "--raw" => include_raw = true,
            "--include-resource" => include_resource = true,
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value if target.is_none() => target = Some(parse_target_selector(value)?),
            value => return Err(format!("Unexpected detail argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "detail requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "detail requires --target <object:id|molecule:index|node:id|bond:id>.".to_string()
    })?;
    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let report = detail_report(
        &input,
        &document,
        &target,
        DetailOptions {
            include_raw,
            include_resource,
        },
    )?;
    write_json_value(report, output.as_deref(), pretty)
}

pub(super) fn detail_report(
    input: &str,
    document: &ChemcoreDocument,
    target: &TargetSelector,
    options: DetailOptions,
) -> Result<Value, String> {
    let object_infos = collect_scene_object_infos(document);
    let detail = match target {
        TargetSelector::Object(id) => object_detail_json(document, &object_infos, id, options)?,
        TargetSelector::Molecule(index) => {
            molecule_detail_json(document, &object_infos, *index, options)?
        }
        TargetSelector::Node(id) => node_detail_json(document, id, options)?,
        TargetSelector::Bond(id) => bond_detail_json(document, id, options)?,
        TargetSelector::All | TargetSelector::Bounds(_) | TargetSelector::Selection(_) => {
            return Err(
                "detail requires object:<id>, molecule:<index>, node:<id>, or bond:<id>. Use inspect for whole-document JSON."
                    .to_string(),
            );
        }
    };
    Ok(json!({
        "ok": true,
        "input": input,
        "target": target.to_json(),
        "detail": detail,
    }))
}

pub(super) fn object_detail_json(
    document: &ChemcoreDocument,
    object_infos: &[SceneObjectInfo],
    id: &str,
    options: DetailOptions,
) -> Result<Value, String> {
    let object = document
        .find_scene_object(id)
        .ok_or_else(|| format!("Object target was not found: {id}."))?;
    let info = object_infos.iter().find(|info| info.id == id);
    let mut detail = json!({
        "selector": format!("object:{id}"),
        "kind": "object",
        "id": id,
        "type": object.object_type,
        "name": object.name,
        "visible": object.visible,
        "locked": object.locked,
        "zIndex": object.z_index,
        "styleRef": object.style_ref,
        "resourceRef": object.payload.resource_ref,
        "childCount": object.children.len(),
        "bounds": optional_bounds_json(
            target_bounds_fast(document, &TargetSelector::Object(id.to_string()))
                .or_else(|| target_bounds(document, &TargetSelector::Object(id.to_string())).ok())
        ),
        "relationships": info.map(object_relationship_json).unwrap_or(Value::Null),
        "references": object_references_json(document, object),
    });
    if options.include_raw {
        set_object_field(
            &mut detail,
            "raw",
            object_raw_json(document, object, options.include_resource),
        );
    }
    Ok(detail)
}

pub(super) fn molecule_detail_json(
    document: &ChemcoreDocument,
    object_infos: &[SceneObjectInfo],
    index: usize,
    options: DetailOptions,
) -> Result<Value, String> {
    let fragments = document.editable_fragments();
    let entry = fragments
        .get(index)
        .ok_or_else(|| format!("Molecule index {index} was not found."))?;
    let object_id = entry.object.id.clone();
    let info = object_infos.iter().find(|info| info.id == object_id);
    let mut detail = json!({
        "selector": format!("molecule:{index}"),
        "kind": "molecule",
        "index": index,
        "objectId": entry.object.id,
        "resourceRef": entry.object.payload.resource_ref,
        "nodeCount": entry.fragment.nodes.len(),
        "bondCount": entry.fragment.bonds.len(),
        "fragmentBbox": entry.fragment.bbox,
        "bounds": optional_bounds_json(
            target_bounds_fast(document, &TargetSelector::Molecule(index))
                .or_else(|| target_bounds(document, &TargetSelector::Molecule(index)).ok())
        ),
        "relationships": info.map(object_relationship_json).unwrap_or(Value::Null),
        "references": object_references_json(document, entry.object),
    });
    if options.include_raw {
        let mut raw = Map::new();
        raw.insert("object".to_string(), json!(entry.object));
        raw.insert("fragment".to_string(), json!(entry.fragment));
        if options.include_resource {
            insert_referenced_resource_raw(&mut raw, document, entry.object);
        }
        set_object_field(&mut detail, "raw", Value::Object(raw));
    }
    Ok(detail)
}

pub(super) fn node_detail_json(
    document: &ChemcoreDocument,
    id: &str,
    options: DetailOptions,
) -> Result<Value, String> {
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == id) else {
            continue;
        };
        let connected_bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.begin == id || bond.end == id)
            .collect::<Vec<_>>();
        let mut detail = json!({
            "selector": format!("node:{id}"),
            "kind": "node",
            "id": id,
            "moleculeIndex": molecule_index,
            "objectId": entry.object.id,
            "resourceRef": entry.object.payload.resource_ref,
            "element": node.element,
            "atomicNumber": node.atomic_number,
            "position": node.position,
            "charge": node.charge,
            "numHydrogens": node.num_hydrogens,
            "labelText": node.label.as_ref().map(|label| label.text.clone()),
            "connectedBondIds": connected_bonds.iter().map(|bond| bond.id.clone()).collect::<Vec<_>>(),
            "bounds": bounds_json(node_fast_bounds(entry.object, node)),
            "references": object_references_json(document, entry.object),
        });
        if options.include_raw {
            let mut raw = Map::new();
            raw.insert("node".to_string(), json!(node));
            if options.include_resource {
                insert_referenced_resource_raw(&mut raw, document, entry.object);
            }
            set_object_field(&mut detail, "raw", Value::Object(raw));
        }
        return Ok(detail);
    }
    Err(format!("Node target was not found: {id}."))
}

pub(super) fn bond_detail_json(
    document: &ChemcoreDocument,
    id: &str,
    options: DetailOptions,
) -> Result<Value, String> {
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == id) else {
            continue;
        };
        let begin_node = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin);
        let end_node = entry.fragment.nodes.iter().find(|node| node.id == bond.end);
        let mut detail = json!({
            "selector": format!("bond:{id}"),
            "kind": "bond",
            "id": id,
            "moleculeIndex": molecule_index,
            "objectId": entry.object.id,
            "resourceRef": entry.object.payload.resource_ref,
            "begin": bond.begin,
            "end": bond.end,
            "order": bond.order,
            "stereo": bond.stereo,
            "lineStyles": bond.line_styles,
            "bounds": optional_bounds_json(bond_fast_bounds(entry.object, &entry.fragment.nodes, bond)),
            "endpoints": {
                "begin": begin_node.map(node_endpoint_summary_json),
                "end": end_node.map(node_endpoint_summary_json),
            },
            "references": object_references_json(document, entry.object),
        });
        if options.include_raw {
            let mut raw = Map::new();
            raw.insert("bond".to_string(), json!(bond));
            if options.include_resource {
                insert_referenced_resource_raw(&mut raw, document, entry.object);
            }
            set_object_field(&mut detail, "raw", Value::Object(raw));
        }
        return Ok(detail);
    }
    Err(format!("Bond target was not found: {id}."))
}

pub(super) fn optional_bounds_json(bounds: Option<[f64; 4]>) -> Value {
    bounds.map(bounds_json).unwrap_or(Value::Null)
}

pub(super) fn node_endpoint_summary_json(node: &Node) -> Value {
    json!({
        "id": node.id,
        "element": node.element,
        "atomicNumber": node.atomic_number,
        "position": node.position,
        "charge": node.charge,
        "labelText": node.label.as_ref().map(|label| label.text.clone()),
    })
}

pub(super) fn object_references_json(document: &ChemcoreDocument, object: &SceneObject) -> Value {
    json!({
        "style": object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref).map(|style| style_summary_json(style_ref, style))),
        "resource": object
            .payload
            .resource_ref
            .as_ref()
            .and_then(|resource_ref| document.resources.get(resource_ref).map(|resource| resource_summary_json(resource_ref, resource))),
    })
}

pub(super) fn style_summary_json(id: &str, style: &Value) -> Value {
    json!({
        "id": id,
        "kind": style.get("kind").and_then(Value::as_str),
        "stroke": style.get("stroke").and_then(Value::as_str),
        "fill": style.get("fill").cloned(),
        "strokeWidth": style.get("strokeWidth").and_then(Value::as_f64),
        "fontFamily": style.get("fontFamily").and_then(Value::as_str),
        "fontSize": style.get("fontSize").and_then(Value::as_f64),
    })
}

pub(super) fn resource_summary_json(id: &str, resource: &chemcore_engine::Resource) -> Value {
    let mut summary = json!({
        "id": id,
        "type": resource.resource_type,
        "encoding": resource.encoding,
    });
    match &resource.data {
        ResourceData::Fragment(fragment) => {
            set_object_field(&mut summary, "kind", json!("fragment"));
            set_object_field(&mut summary, "nodeCount", json!(fragment.nodes.len()));
            set_object_field(&mut summary, "bondCount", json!(fragment.bonds.len()));
            set_object_field(&mut summary, "bbox", json!(fragment.bbox));
        }
        ResourceData::Text(text) => {
            set_object_field(&mut summary, "kind", json!("text"));
            set_object_field(&mut summary, "textLength", json!(text.len()));
        }
        ResourceData::Json(value) => {
            set_object_field(&mut summary, "kind", json!("json"));
            set_object_field(&mut summary, "jsonType", json!(json_value_kind(value)));
        }
    }
    summary
}

pub(super) fn object_raw_json(
    document: &ChemcoreDocument,
    object: &SceneObject,
    include_resource: bool,
) -> Value {
    let mut raw = Map::new();
    raw.insert("object".to_string(), json!(object));
    if include_resource {
        insert_referenced_resource_raw(&mut raw, document, object);
    }
    Value::Object(raw)
}

pub(super) fn insert_referenced_resource_raw(
    raw: &mut Map<String, Value>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    raw.insert(
        "resource".to_string(),
        json!({
            "id": resource_ref,
            "value": resource,
        }),
    );
}

pub(super) fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
