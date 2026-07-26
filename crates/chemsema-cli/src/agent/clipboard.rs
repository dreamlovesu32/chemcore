use super::*;

const EXPORT_SELECTION_MARGIN: f64 = 20.0;

pub(crate) fn copy_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut office_helper = None;
    let mut payload_path = None;
    let mut copy_to_clipboard = true;
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
            "--all" => target = Some(TargetSelector::All),
            "--office-helper" => {
                index += 1;
                office_helper = Some(
                    args.get(index)
                        .ok_or_else(|| "--office-helper requires a path.".to_string())?
                        .clone(),
                );
            }
            "--payload" => {
                index += 1;
                payload_path = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--payload requires a path.".to_string())?,
                ));
            }
            "--no-copy" => copy_to_clipboard = false,
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected copy argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "copy requires an input file.".to_string())?;
    let target = target.unwrap_or(TargetSelector::All);
    if matches!(target, TargetSelector::Bounds(_)) {
        return Err("copy targets must be all, object, molecule, node, or bond; bounds are only for capture."
            .to_string());
    }

    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let clipboard_document = clipboard_document_for_target(&document, &target)?;
    let payload = clipboard_payload_for_document(&clipboard_document)?;
    let payload_defaulted = payload_path.is_none();
    let payload_path = payload_path.unwrap_or_else(default_clipboard_payload_path);
    let payload_bytes = write_clipboard_payload_file(&payload_path, &payload)?;

    let copied_helper = if copy_to_clipboard {
        Some(copy_payload_to_office_clipboard(
            &payload_path,
            office_helper.as_deref(),
        )?)
    } else {
        None
    };
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
            "warnings": default_payload_warnings(payload_defaulted, &payload_path),
            "payload": {
                "path": payload_path.display().to_string(),
                "defaulted": payload_defaulted,
                "verified": true,
                "bytes": payload_bytes,
            },
            "clipboard": {
                "copied": copy_to_clipboard,
                "helper": copied_helper.map(|path| path.display().to_string()),
                "format": "windows-office-ole",
            },
            "document": {
                "objects": clipboard_document.objects.len(),
                "resources": clipboard_document.resources.len(),
            }
        }),
        None,
        pretty,
    )
}

pub(super) fn clipboard_document_for_target(
    document: &ChemSemaDocument,
    target: &TargetSelector,
) -> Result<ChemSemaDocument, String> {
    let bounds = target_bounds(document, target)?;
    let mut clipboard_document = match target {
        TargetSelector::All => document.clone(),
        TargetSelector::Object(id) => clipboard_document_for_object(document, id)?,
        TargetSelector::Molecule(index) => {
            let object_id = molecule_object_id(document, *index)?;
            clipboard_document_for_object(document, &object_id)?
        }
        TargetSelector::Node(id) => {
            clipboard_document_for_fragment_target(document, Some(id.as_str()), None)?
        }
        TargetSelector::Bond(id) => {
            clipboard_document_for_fragment_target(document, None, Some(id.as_str()))?
        }
        TargetSelector::Bounds(_) => {
            return Err("Bounds targets cannot be copied as editable Office objects.".to_string())
        }
        TargetSelector::Selection(_) => {
            return Err("Selection targets cannot be copied as a single editable Office object. Use copy all, object, molecule, node, or bond.".to_string())
        }
    };
    clipboard_document.document.id = "doc_clipboard_selection".to_string();
    clipboard_document.document.title = "ChemSema Clipboard Selection".to_string();
    set_clipboard_selection_bounds_meta(&mut clipboard_document, bounds);
    Ok(clipboard_document)
}

pub(crate) fn export_document_for_target(
    document: &ChemSemaDocument,
    target: &TargetSelector,
) -> Result<ChemSemaDocument, String> {
    if matches!(target, TargetSelector::Bounds(_)) {
        return Err(
            "Bounds targets cannot be exported as editable documents. Use capture for bounds crops."
                .to_string(),
        );
    }
    let bounds = target_bounds(document, target)?;
    let mut export_document = editable_document_for_target(document, target)?;
    export_document.document.id = "doc_export_selection".to_string();
    export_document.document.title = "ChemSema Export Selection".to_string();
    if !matches!(target, TargetSelector::All) {
        prune_unreferenced_resources_and_styles(&mut export_document);
        compact_document_to_bounds(&mut export_document, bounds, EXPORT_SELECTION_MARGIN);
    }
    set_export_selection_meta(&mut export_document, bounds, target);
    Ok(export_document)
}

fn editable_document_for_target(
    document: &ChemSemaDocument,
    target: &TargetSelector,
) -> Result<ChemSemaDocument, String> {
    match target {
        TargetSelector::All => Ok(document.clone()),
        TargetSelector::Object(id) => clipboard_document_for_object(document, id),
        TargetSelector::Molecule(index) => {
            let object_id = molecule_object_id(document, *index)?;
            clipboard_document_for_object(document, &object_id)
        }
        TargetSelector::Node(id) => {
            clipboard_document_for_fragment_target(document, Some(id.as_str()), None)
        }
        TargetSelector::Bond(id) => {
            clipboard_document_for_fragment_target(document, None, Some(id.as_str()))
        }
        TargetSelector::Bounds(_) => Err(
            "Bounds targets cannot be exported as editable documents. Use capture for bounds crops."
                .to_string(),
        ),
        TargetSelector::Selection(targets) => editable_document_for_selection(document, targets),
    }
}

fn editable_document_for_selection(
    document: &ChemSemaDocument,
    targets: &[TargetSelector],
) -> Result<ChemSemaDocument, String> {
    let mut out = document.clone();
    out.objects.clear();
    for target in targets {
        match target {
            TargetSelector::Object(_) | TargetSelector::Molecule(_) => {
                let selected = editable_document_for_target(document, target)?;
                for object in selected.objects {
                    merge_scene_object_path(&mut out.objects, object);
                }
            }
            TargetSelector::Selection(nested) => {
                let selected = editable_document_for_selection(document, nested)?;
                for object in selected.objects {
                    merge_scene_object_path(&mut out.objects, object);
                }
            }
            TargetSelector::All => {
                return Err("Use target 'all' by itself for whole-document export.".to_string());
            }
            TargetSelector::Node(_) | TargetSelector::Bond(_) => {
                return Err("Multi-target editable export currently supports object and molecule selectors. Export node/bond targets one at a time.".to_string());
            }
            TargetSelector::Bounds(_) => {
                return Err("Bounds targets cannot be exported as editable documents. Use capture for bounds crops.".to_string());
            }
        }
    }
    if out.objects.is_empty() {
        return Err("Selection did not resolve to any exportable object.".to_string());
    }
    Ok(out)
}

fn merge_scene_object_path(objects: &mut Vec<SceneObject>, object: SceneObject) {
    if let Some(existing) = objects
        .iter_mut()
        .find(|candidate| candidate.id == object.id)
    {
        if object.children.is_empty() {
            *existing = object;
            return;
        }
        for child in object.children {
            merge_scene_object_path(&mut existing.children, child);
        }
        return;
    }
    objects.push(object);
}

fn prune_unreferenced_resources_and_styles(document: &mut ChemSemaDocument) {
    let mut resource_refs = BTreeSet::new();
    let mut style_refs = BTreeSet::new();
    collect_scene_object_dependencies(&document.objects, &mut resource_refs, &mut style_refs);
    document
        .resources
        .retain(|id, _| resource_refs.contains(id));
    document.styles.retain(|id, _| style_refs.contains(id));
}

fn collect_scene_object_dependencies(
    objects: &[SceneObject],
    resource_refs: &mut BTreeSet<String>,
    style_refs: &mut BTreeSet<String>,
) {
    for object in objects {
        if let Some(resource_ref) = object.payload.resource_ref.as_ref() {
            resource_refs.insert(resource_ref.clone());
        }
        if let Some(style_ref) = object.style_ref.as_ref() {
            style_refs.insert(style_ref.clone());
        }
        collect_scene_object_dependencies(&object.children, resource_refs, style_refs);
    }
}

fn compact_document_to_bounds(document: &mut ChemSemaDocument, bounds: [f64; 4], margin: f64) {
    let dx = margin - bounds[0];
    let dy = margin - bounds[1];
    translate_rendered_scene_objects(&mut document.objects, dx, dy);
    document.document.page.width = (bounds[2] - bounds[0]).max(1.0) + margin * 2.0;
    document.document.page.height = (bounds[3] - bounds[1]).max(1.0) + margin * 2.0;
}

fn translate_rendered_scene_objects(objects: &mut [SceneObject], dx: f64, dy: f64) {
    for object in objects {
        if object.object_type == "group" {
            translate_rendered_scene_objects(&mut object.children, dx, dy);
            continue;
        }
        object.transform.translate[0] += dx;
        object.transform.translate[1] += dy;
    }
}

fn set_export_selection_meta(
    document: &mut ChemSemaDocument,
    bounds: [f64; 4],
    target: &TargetSelector,
) {
    if !document.document.meta.is_object() {
        document.document.meta = json!({});
    }
    let Some(meta) = document.document.meta.as_object_mut() else {
        return;
    };
    let export = meta.entry("export").or_insert_with(|| json!({}));
    if !export.is_object() {
        *export = json!({});
    }
    if let Some(export) = export.as_object_mut() {
        export.insert("selectionTarget".to_string(), target.to_json());
        export.insert(
            "selectionBounds".to_string(),
            json!({
                "minX": bounds[0],
                "minY": bounds[1],
                "maxX": bounds[2],
                "maxY": bounds[3],
            }),
        );
        export.insert(
            "selectionMargin".to_string(),
            json!(EXPORT_SELECTION_MARGIN),
        );
    }
}

pub(super) fn clipboard_document_for_object(
    document: &ChemSemaDocument,
    object_id: &str,
) -> Result<ChemSemaDocument, String> {
    let objects = clone_scene_object_path_by_id(&document.objects, object_id)
        .ok_or_else(|| format!("Object target not found: {object_id}."))?;
    let mut out = document.clone();
    out.objects = objects;
    out.reaction_schemes.clear();
    freeze_stoichiometry_grids_for_subset(&mut out.objects);
    Ok(out)
}

pub(super) fn clone_scene_object_path_by_id(
    objects: &[SceneObject],
    object_id: &str,
) -> Option<Vec<SceneObject>> {
    for object in objects {
        if object.id == object_id {
            return Some(vec![object.clone()]);
        }
        if let Some(children) = clone_scene_object_path_by_id(&object.children, object_id) {
            let mut clone = object.clone();
            clone.children = children;
            return Some(vec![clone]);
        }
    }
    None
}

pub(super) fn clipboard_document_for_fragment_target(
    document: &ChemSemaDocument,
    node_id: Option<&str>,
    bond_id: Option<&str>,
) -> Result<ChemSemaDocument, String> {
    for entry in document.editable_fragments() {
        let Some(resource_ref) = entry.object.payload.resource_ref.clone() else {
            continue;
        };
        let mut selected_node_ids = BTreeSet::new();
        let mut selected_bond_ids = BTreeSet::new();
        if let Some(node_id) = node_id {
            if entry.fragment.nodes.iter().any(|node| node.id == node_id) {
                selected_node_ids.insert(node_id.to_string());
            } else {
                continue;
            }
        }
        if let Some(bond_id) = bond_id {
            let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == bond_id) else {
                continue;
            };
            selected_bond_ids.insert(bond.id.clone());
            selected_node_ids.insert(bond.begin.clone());
            selected_node_ids.insert(bond.end.clone());
        }

        let nodes = entry
            .fragment
            .nodes
            .iter()
            .filter(|node| selected_node_ids.contains(&node.id))
            .cloned()
            .collect::<Vec<_>>();
        if nodes.is_empty() {
            continue;
        }
        let bonds = entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| {
                selected_bond_ids.contains(&bond.id)
                    && selected_node_ids.contains(&bond.begin)
                    && selected_node_ids.contains(&bond.end)
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut fragment = entry.fragment.clone();
        fragment.nodes = nodes;
        fragment.bonds = bonds;
        fragment.bbox = fragment_clipboard_bounds(&fragment.nodes);

        let mut object = entry.object.clone();
        object.payload.bbox = Some(fragment.bbox);

        let mut resource = document
            .resources
            .get(&resource_ref)
            .ok_or_else(|| format!("Missing molecule resource '{resource_ref}'."))?
            .clone();
        resource.data = ResourceData::Fragment(fragment);

        let mut out = document.clone();
        out.objects = vec![object];
        out.reaction_schemes.clear();
        freeze_stoichiometry_grids_for_subset(&mut out.objects);
        out.resources.insert(resource_ref, resource);
        return Ok(out);
    }
    match (node_id, bond_id) {
        (Some(id), _) => Err(format!("Node target not found: {id}.")),
        (_, Some(id)) => Err(format!("Bond target not found: {id}.")),
        _ => Err("No fragment target was provided.".to_string()),
    }
}

fn freeze_stoichiometry_grids_for_subset(objects: &mut [SceneObject]) {
    for object in objects {
        if let Some(grid) = object.payload.stoichiometry_grid.as_mut() {
            grid.source_reaction_step_id = None;
            grid.binding_state = chemsema_engine::StoichiometryBindingState::Detached;
            object.link_policy = chemsema_engine::LinkPolicy::Unlinked;
            for datum in &mut grid.data {
                if datum.origin == chemsema_engine::StoichiometryValueOrigin::Calculated {
                    datum.origin = chemsema_engine::StoichiometryValueOrigin::Imported;
                }
            }
        }
        freeze_stoichiometry_grids_for_subset(&mut object.children);
    }
}

pub(super) fn fragment_clipboard_bounds(nodes: &[Node]) -> [f64; 4] {
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
        if let Some(bounds) = node.label.as_ref().and_then(|label| label.bbox()) {
            min_x = min_x.min(bounds[0]);
            min_y = min_y.min(bounds[1]);
            max_x = max_x.max(bounds[2]);
            max_y = max_y.max(bounds[3]);
        }
    }
    [min_x, min_y, max_x.max(min_x + 1.0), max_y.max(min_y + 1.0)]
}

pub(super) fn set_clipboard_selection_bounds_meta(
    document: &mut ChemSemaDocument,
    bounds: [f64; 4],
) {
    if !document.document.meta.is_object() {
        document.document.meta = json!({});
    }
    let Some(meta) = document.document.meta.as_object_mut() else {
        return;
    };
    let clipboard = meta.entry("clipboard").or_insert_with(|| json!({}));
    if !clipboard.is_object() {
        *clipboard = json!({});
    }
    if let Some(clipboard) = clipboard.as_object_mut() {
        clipboard.insert(
            "selectionBounds".to_string(),
            json!({
                "minX": bounds[0],
                "minY": bounds[1],
                "maxX": bounds[2],
                "maxY": bounds[3],
            }),
        );
    }
}

pub(super) fn clipboard_payload_for_document(document: &ChemSemaDocument) -> Result<Value, String> {
    let chemsema_document_json =
        serde_json::to_string(document).map_err(|error| error.to_string())?;
    let render_list_json =
        serde_json::to_string(&render_document(document)).map_err(|error| error.to_string())?;
    let cdxml = document_to_cdxml(document);
    let svg = document_to_svg(document);
    Ok(json!({
        "text": cdxml,
        "chemsemaFragmentJson": Value::Null,
        "chemsemaDocumentJson": chemsema_document_json,
        "renderListJson": render_list_json,
        "cdxml": cdxml,
        "svg": svg,
    }))
}

pub(super) fn default_clipboard_payload_path() -> PathBuf {
    default_output_dir().join(format!(
        "copy-payload-{}-{}.json",
        std::process::id(),
        timestamp_millis()
    ))
}

pub(super) fn write_clipboard_payload_file(path: &Path, payload: &Value) -> Result<u64, String> {
    ensure_output_parent_path(path)?;
    let text = serde_json::to_string_pretty(payload).map_err(|error| error.to_string())?;
    let expected_bytes = text.len() as u64;
    fs::write(path, text.as_bytes()).map_err(|error| {
        format!(
            "Failed to write clipboard payload {}: {error}",
            path.display()
        )
    })?;
    verify_file_written_exact(path, expected_bytes, "clipboard payload")
}

#[cfg(windows)]
pub(super) fn copy_payload_to_office_clipboard(
    payload_path: &Path,
    office_helper: Option<&str>,
) -> Result<PathBuf, String> {
    let helper = resolve_office_helper(office_helper)?;
    let output = Command::new(&helper)
        .arg("--copy-clipboard-payload")
        .arg(payload_path)
        .output()
        .map_err(|error| format!("Failed to launch {}: {error}", helper.display()))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Office clipboard helper failed with exit code {:?}. payload={} stdout='{}' stderr='{}'",
            output.status.code(),
            payload_path.display(),
            stdout,
            stderr
        ));
    }
    Ok(helper)
}

#[cfg(not(windows))]
pub(super) fn copy_payload_to_office_clipboard(
    payload_path: &Path,
    _office_helper: Option<&str>,
) -> Result<PathBuf, String> {
    Err(format!(
        "Copying to the Office/OLE clipboard is only supported on Windows. Payload was written to {}.",
        payload_path.display()
    ))
}

#[cfg(windows)]
pub(super) fn resolve_office_helper(office_helper: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = office_helper {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!("Office helper was not found: {}.", path.display()));
    }

    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("chemsema-office.exe"));
            candidates.push(dir.join("resources").join("chemsema-office.exe"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("resources").join("chemsema-office.exe"));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(
            cwd.join("target")
                .join("release")
                .join("chemsema-office.exe"),
        );
        candidates.push(cwd.join("target").join("debug").join("chemsema-office.exe"));
    }

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "chemsema-office.exe was not found. Pass --office-helper <path>. Checked: {}",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    ))
}
