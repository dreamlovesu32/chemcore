use super::*;

pub(crate) fn context_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut capture_output = None;
    let mut capture_format = None;
    let mut expansion = CropExpansion::uniform_abs(30.0);
    let mut raster = RasterOptions::default();
    let mut limit = 200usize;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" | "--around" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selector(
                        args.get(index)
                            .ok_or_else(|| "--target requires a selector.".to_string())?,
                    )?,
                )?;
            }
            "--targets" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selection_arg(args.get(index).ok_or_else(|| {
                        "--targets requires selectors separated by semicolons.".to_string()
                    })?)?,
                )?;
            }
            "--object" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Object(
                        args.get(index)
                            .ok_or_else(|| "--object requires an object id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--molecule" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Molecule(parse_usize_arg("--molecule", args.get(index))?),
                )?;
            }
            "--node" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Node(
                        args.get(index)
                            .ok_or_else(|| "--node requires a node id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bond" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Bond(
                        args.get(index)
                            .ok_or_else(|| "--bond requires a bond id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--capture-out" | "--screenshot-out" => {
                index += 1;
                capture_output = Some(
                    args.get(index)
                        .ok_or_else(|| "--capture-out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                capture_format =
                    Some(parse_capture_format(args.get(index).ok_or_else(|| {
                        "--format requires svg or png.".to_string()
                    })?)?);
            }
            "--limit" => {
                index += 1;
                limit = parse_usize_arg("--limit", args.get(index))?;
            }
            "--radius" | "--padding" | "--expand" => {
                index += 1;
                let value = parse_non_negative_f64(
                    args[index - 1].as_str(),
                    args.get(index)
                        .ok_or_else(|| format!("{} requires a number.", args[index - 1]))?,
                )?;
                expansion.abs_left = value;
                expansion.abs_top = value;
                expansion.abs_right = value;
                expansion.abs_bottom = value;
            }
            "--expand-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-x requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_right = value;
            }
            "--expand-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-y requires a number.".to_string())?,
                )?;
                expansion.abs_top = value;
                expansion.abs_bottom = value;
            }
            "--expand-left" => {
                index += 1;
                expansion.abs_left = parse_non_negative_f64(
                    "--expand-left",
                    args.get(index)
                        .ok_or_else(|| "--expand-left requires a number.".to_string())?,
                )?;
            }
            "--expand-right" => {
                index += 1;
                expansion.abs_right = parse_non_negative_f64(
                    "--expand-right",
                    args.get(index)
                        .ok_or_else(|| "--expand-right requires a number.".to_string())?,
                )?;
            }
            "--expand-top" => {
                index += 1;
                expansion.abs_top = parse_non_negative_f64(
                    "--expand-top",
                    args.get(index)
                        .ok_or_else(|| "--expand-top requires a number.".to_string())?,
                )?;
            }
            "--expand-bottom" => {
                index += 1;
                expansion.abs_bottom = parse_non_negative_f64(
                    "--expand-bottom",
                    args.get(index)
                        .ok_or_else(|| "--expand-bottom requires a number.".to_string())?,
                )?;
            }
            "--expand-rel" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_top = value;
                expansion.rel_right = value;
                expansion.rel_bottom = value;
            }
            "--expand-rel-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-x requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_right = value;
            }
            "--expand-rel-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-y requires a fraction.".to_string())?,
                )?;
                expansion.rel_top = value;
                expansion.rel_bottom = value;
            }
            "--scale" => {
                index += 1;
                raster.scale = parse_positive_f64(
                    "--scale",
                    args.get(index)
                        .ok_or_else(|| "--scale requires a positive number.".to_string())?,
                )?;
            }
            "--width" => {
                index += 1;
                raster.width = Some(parse_positive_u32(
                    "--width",
                    args.get(index)
                        .ok_or_else(|| "--width requires a positive integer.".to_string())?,
                )?);
            }
            "--height" => {
                index += 1;
                raster.height = Some(parse_positive_u32(
                    "--height",
                    args.get(index)
                        .ok_or_else(|| "--height requires a positive integer.".to_string())?,
                )?);
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected context argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "context requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "context requires --target <object:id|molecule:index|node:id|bond:id|all> or multiple targets via repeated --target or --targets.".to_string()
    })?;
    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let target_bounds = target_bounds(&document, &target)?;
    let query_view_box = expanded_view_box(target_bounds, expansion);
    let query_bounds = view_box_to_bounds(query_view_box);
    let mut report = context_report(
        &input,
        &document,
        &target,
        target_bounds,
        query_bounds,
        expansion,
        limit,
    )?;

    if let Some(capture_output) = capture_output.as_deref() {
        let format = capture_format
            .or_else(|| infer_capture_format_from_path(capture_output))
            .ok_or_else(|| {
                "--capture-out format is ambiguous; use .svg/.png or --format svg|png.".to_string()
            })?;
        let render = capture_render_primitives(&document, &target, query_view_box);
        let render_output = write_capture_output(
            &render.primitives,
            query_view_box,
            capture_output,
            format,
            raster,
        )?;
        let primitive_count = render.primitives.len();
        set_object_field(
            &mut report,
            "capture",
            json!({
                "ok": true,
                "path": capture_output,
                "format": format.as_str(),
                "verified": true,
                "bytes": render_output.bytes,
                "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
                "viewBox": view_box_json(query_view_box),
                "render": {
                    "mode": render.mode,
                    "primitiveCount": primitive_count,
                    "targets": render.targets.to_json(),
                },
            }),
        );
    }

    write_json_value(report, output.as_deref(), pretty)
}

pub(super) fn context_report(
    input: &str,
    document: &ChemcoreDocument,
    target: &TargetSelector,
    target_box: [f64; 4],
    query_bounds: [f64; 4],
    expansion: CropExpansion,
    limit: usize,
) -> Result<Value, String> {
    let object_infos = collect_scene_object_infos(document);
    let mut objects = object_infos
        .iter()
        .filter(|info| bounds_intersect(info.bounds, query_bounds))
        .map(|info| {
            json!({
                "selector": format!("object:{}", info.id),
                "kind": "object",
                "id": info.id,
                "type": info.object_type,
                "name": info.name,
                "visible": info.visible,
                "bounds": bounds_json(info.bounds),
                "spatial": spatial_relation_json(target_box, info.bounds),
                "selectionBoxRelation": selection_box_relation(target_box, info.bounds),
                "relationships": object_relationship_json(info),
                "isTarget": target_matches_object(target, info),
            })
        })
        .collect::<Vec<_>>();

    let mut molecules = document
        .editable_fragments()
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let bounds = target_bounds_fast(document, &TargetSelector::Molecule(index))
                .or_else(|| target_bounds(document, &TargetSelector::Molecule(index)).ok())?;
            bounds_intersect(bounds, query_bounds).then(|| {
                json!({
                    "selector": format!("molecule:{index}"),
                    "kind": "molecule",
                    "index": index,
                    "objectId": entry.object.id,
                    "resourceRef": entry.object.payload.resource_ref,
                    "nodeCount": entry.fragment.nodes.len(),
                    "bondCount": entry.fragment.bonds.len(),
                    "bounds": bounds_json(bounds),
                    "spatial": spatial_relation_json(target_box, bounds),
                    "selectionBoxRelation": selection_box_relation(target_box, bounds),
                    "isTarget": target_matches_molecule(target, index),
                })
            })
        })
        .collect::<Vec<_>>();

    let mut nodes = Vec::new();
    let mut bonds = Vec::new();
    for (molecule_index, entry) in document.editable_fragments().into_iter().enumerate() {
        for node in &entry.fragment.nodes {
            let bounds = node_fast_bounds(entry.object, node);
            if bounds_intersect(bounds, query_bounds) {
                nodes.push(json!({
                    "selector": format!("node:{}", node.id),
                    "kind": "node",
                    "id": node.id,
                    "moleculeIndex": molecule_index,
                    "objectId": entry.object.id,
                    "element": node.element,
                    "atomicNumber": node.atomic_number,
                    "bounds": bounds_json(bounds),
                    "spatial": spatial_relation_json(target_box, bounds),
                    "selectionBoxRelation": selection_box_relation(target_box, bounds),
                    "isTarget": target_matches_node(target, &node.id),
                }));
            }
        }
        for bond in &entry.fragment.bonds {
            let Some(bounds) = bond_fast_bounds(entry.object, &entry.fragment.nodes, bond) else {
                continue;
            };
            if bounds_intersect(bounds, query_bounds) {
                bonds.push(json!({
                    "selector": format!("bond:{}", bond.id),
                    "kind": "bond",
                    "id": bond.id,
                    "moleculeIndex": molecule_index,
                    "objectId": entry.object.id,
                    "begin": bond.begin,
                    "end": bond.end,
                    "order": bond.order,
                    "bounds": bounds_json(bounds),
                    "spatial": spatial_relation_json(target_box, bounds),
                    "selectionBoxRelation": selection_box_relation(target_box, bounds),
                    "isTarget": target_matches_bond(target, &bond.id),
                }));
            }
        }
    }
    let selection_box =
        selection_box_summary(target_box, &objects, &molecules, &nodes, &bonds, limit);
    sort_context_entries(&mut objects);
    sort_context_entries(&mut molecules);
    sort_context_entries(&mut nodes);
    sort_context_entries(&mut bonds);
    objects.truncate(limit);
    molecules.truncate(limit);
    nodes.truncate(limit);
    bonds.truncate(limit);

    Ok(json!({
        "ok": true,
        "input": input,
        "target": target.to_json(),
        "bounds": {
            "target": bounds_json(target_box),
            "query": bounds_json(query_bounds),
        },
        "selectionBox": selection_box,
        "expansion": expansion.to_json(),
        "counts": {
            "objects": objects.len(),
            "molecules": molecules.len(),
            "nodes": nodes.len(),
            "bonds": bonds.len(),
            "limit": limit,
        },
        "relationships": target_relationships_json(target, &object_infos),
        "context": {
            "objects": objects,
            "molecules": molecules,
            "nodes": nodes,
            "bonds": bonds,
        }
    }))
}

pub(super) struct SceneObjectInfo {
    pub(super) id: String,
    pub(super) object_type: String,
    pub(super) name: String,
    pub(super) visible: bool,
    pub(super) bounds: [f64; 4],
    pub(super) parent_id: Option<String>,
    pub(super) ancestor_ids: Vec<String>,
    pub(super) child_ids: Vec<String>,
    pub(super) linked_object_ids: Vec<String>,
    pub(super) link_kind: Option<String>,
    pub(super) group_kind: Option<String>,
}

pub(super) fn collect_scene_object_infos(document: &ChemcoreDocument) -> Vec<SceneObjectInfo> {
    let mut out = Vec::new();
    collect_scene_object_infos_inner(document, &document.objects, None, &[], &mut out);
    out
}

pub(super) fn collect_scene_object_infos_inner(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    parent_id: Option<&str>,
    ancestors: &[String],
    out: &mut Vec<SceneObjectInfo>,
) {
    for object in objects {
        let mut next_ancestors = ancestors.to_vec();
        if let Some(parent_id) = parent_id {
            next_ancestors.push(parent_id.to_string());
        }
        if let Some(bounds) =
            target_bounds_fast(document, &TargetSelector::Object(object.id.clone())).or_else(|| {
                target_bounds(document, &TargetSelector::Object(object.id.clone())).ok()
            })
        {
            out.push(SceneObjectInfo {
                id: object.id.clone(),
                object_type: object.object_type.clone(),
                name: object.name.clone(),
                visible: object.visible,
                bounds,
                parent_id: parent_id.map(str::to_string),
                ancestor_ids: next_ancestors.clone(),
                child_ids: object
                    .children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect(),
                linked_object_ids: linked_object_ids(object),
                link_kind: object
                    .meta
                    .get("linkKind")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                group_kind: object
                    .meta
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        collect_scene_object_infos_inner(
            document,
            &object.children,
            Some(object.id.as_str()),
            &next_ancestors,
            out,
        );
    }
}

pub(super) fn linked_object_ids(object: &SceneObject) -> Vec<String> {
    [
        "linkedTextObjectId",
        "linkedBracketObjectId",
        "bracketLabelTextObjectId",
        "bracketObjectId",
    ]
    .into_iter()
    .filter_map(|key| {
        object
            .meta
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    })
    .collect()
}

pub(super) fn object_relationship_json(info: &SceneObjectInfo) -> Value {
    json!({
        "parentId": info.parent_id,
        "ancestorIds": info.ancestor_ids,
        "childIds": info.child_ids,
        "isGroup": info.object_type == "group",
        "groupKind": info.group_kind,
        "linkedObjectIds": info.linked_object_ids,
        "linkKind": info.link_kind,
    })
}

pub(super) fn target_relationships_json(
    target: &TargetSelector,
    infos: &[SceneObjectInfo],
) -> Value {
    match target {
        TargetSelector::Object(id) => infos
            .iter()
            .find(|info| &info.id == id)
            .map(object_relationship_json)
            .unwrap_or(Value::Null),
        TargetSelector::Molecule(index) => json!({
            "moleculeIndex": index,
        }),
        TargetSelector::Node(id) => json!({
            "nodeId": id,
        }),
        TargetSelector::Bond(id) => json!({
            "bondId": id,
        }),
        TargetSelector::Selection(targets) => json!({
            "targets": targets
                .iter()
                .map(|target| json!({
                    "target": target.to_json(),
                    "relationships": target_relationships_json(target, infos),
                }))
                .collect::<Vec<_>>(),
        }),
        TargetSelector::All | TargetSelector::Bounds(_) => Value::Null,
    }
}

pub(super) fn target_matches_object(target: &TargetSelector, info: &SceneObjectInfo) -> bool {
    match target {
        TargetSelector::Object(id) => id == &info.id,
        TargetSelector::Selection(targets) => targets
            .iter()
            .any(|target| target_matches_object(target, info)),
        _ => false,
    }
}

pub(super) fn target_matches_molecule(target: &TargetSelector, index: usize) -> bool {
    match target {
        TargetSelector::Molecule(target_index) => *target_index == index,
        TargetSelector::Selection(targets) => targets
            .iter()
            .any(|target| target_matches_molecule(target, index)),
        _ => false,
    }
}

pub(super) fn target_matches_node(target: &TargetSelector, id: &str) -> bool {
    match target {
        TargetSelector::Node(target_id) => target_id == id,
        TargetSelector::Selection(targets) => {
            targets.iter().any(|target| target_matches_node(target, id))
        }
        _ => false,
    }
}

pub(super) fn target_matches_bond(target: &TargetSelector, id: &str) -> bool {
    match target {
        TargetSelector::Bond(target_id) => target_id == id,
        TargetSelector::Selection(targets) => {
            targets.iter().any(|target| target_matches_bond(target, id))
        }
        _ => false,
    }
}

pub(super) fn selection_box_relation(target_box: [f64; 4], bounds: [f64; 4]) -> &'static str {
    if bounds_contains(target_box, bounds) {
        "inside"
    } else if bounds_intersect(target_box, bounds) {
        "partial"
    } else {
        "outside"
    }
}

pub(super) fn selection_box_summary(
    target_box: [f64; 4],
    objects: &[Value],
    molecules: &[Value],
    nodes: &[Value],
    bonds: &[Value],
    limit: usize,
) -> Value {
    json!({
        "bounds": bounds_json(target_box),
        "contents": {
            "objects": selection_box_entries(objects, limit),
            "molecules": selection_box_entries(molecules, limit),
            "nodes": selection_box_entries(nodes, limit),
            "bonds": selection_box_entries(bonds, limit),
        }
    })
}

pub(super) fn selection_box_entries(entries: &[Value], limit: usize) -> Value {
    let mut count = 0usize;
    let mut items = Vec::new();
    for entry in entries {
        let relation = entry
            .get("selectionBoxRelation")
            .and_then(Value::as_str)
            .unwrap_or("outside");
        if relation == "outside" {
            continue;
        }
        count += 1;
        if items.len() < limit {
            items.push(selection_box_entry_summary(entry));
        }
    }
    json!({
        "count": count,
        "truncated": count > items.len(),
        "items": items,
    })
}

pub(super) fn selection_box_entry_summary(entry: &Value) -> Value {
    let mut summary = Map::new();
    for key in [
        "selector",
        "kind",
        "id",
        "index",
        "objectId",
        "type",
        "name",
        "bounds",
        "selectionBoxRelation",
        "isTarget",
    ] {
        if let Some(value) = entry.get(key) {
            summary.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(summary)
}

pub(super) fn spatial_relation_json(target: [f64; 4], other: [f64; 4]) -> Value {
    let target_center = bounds_center(target);
    let other_center = bounds_center(other);
    let dx = other_center[0] - target_center[0];
    let dy = other_center[1] - target_center[1];
    let gap_x = axis_gap(target[0], target[2], other[0], other[2]);
    let gap_y = axis_gap(target[1], target[3], other[1], other[3]);
    let edge_gap = (gap_x * gap_x + gap_y * gap_y).sqrt();
    json!({
        "direction": direction_for_delta(dx, dy, gap_x, gap_y),
        "centerDelta": { "x": dx, "y": dy },
        "centerDistance": (dx * dx + dy * dy).sqrt(),
        "edgeGap": edge_gap,
        "overlapsTarget": bounds_intersect(target, other),
        "containsTarget": bounds_contains(other, target),
        "insideTarget": bounds_contains(target, other),
    })
}

pub(super) fn sort_context_entries(entries: &mut [Value]) {
    entries.sort_by(|left, right| {
        let left_gap = left
            .pointer("/spatial/edgeGap")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        let right_gap = right
            .pointer("/spatial/edgeGap")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        left_gap
            .partial_cmp(&right_gap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub(super) fn bounds_center(bounds: [f64; 4]) -> [f64; 2] {
    [(bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5]
}

pub(super) fn axis_gap(a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> f64 {
    if a_max < b_min {
        b_min - a_max
    } else if b_max < a_min {
        a_min - b_max
    } else {
        0.0
    }
}

pub(super) fn direction_for_delta(dx: f64, dy: f64, gap_x: f64, gap_y: f64) -> &'static str {
    if gap_x == 0.0 && gap_y == 0.0 {
        return "overlap";
    }
    if gap_x >= gap_y {
        if dx < 0.0 {
            "left"
        } else {
            "right"
        }
    } else if dy < 0.0 {
        "above"
    } else {
        "below"
    }
}

pub(super) fn bounds_intersect(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

pub(super) fn bounds_contains(outer: [f64; 4], inner: [f64; 4]) -> bool {
    outer[0] <= inner[0] && outer[1] <= inner[1] && outer[2] >= inner[2] && outer[3] >= inner[3]
}

pub(super) fn view_box_to_bounds(view_box: [f64; 4]) -> [f64; 4] {
    [
        view_box[0],
        view_box[1],
        view_box[0] + view_box[2],
        view_box[1] + view_box[3],
    ]
}
