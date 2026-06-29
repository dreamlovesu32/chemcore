use crate::{document_json, load_engine_from_file, write_json_value, write_text_output};
use chemcore_engine::{
    document_to_cdxml, document_to_svg, primitives_to_svg_viewbox, render_document,
    render_document_targets, render_primitives_bounds, Bond, ChemcoreDocument, Engine, Node,
    RenderPrimitive, ResourceData, SceneObject,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TargetSelector {
    All,
    Object(String),
    Molecule(usize),
    Node(String),
    Bond(String),
    Bounds([f64; 4]),
}

impl TargetSelector {
    fn selector(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Object(id) => format!("object:{id}"),
            Self::Molecule(index) => format!("molecule:{index}"),
            Self::Node(id) => format!("node:{id}"),
            Self::Bond(id) => format!("bond:{id}"),
            Self::Bounds(bounds) => format!(
                "bounds:{},{},{},{}",
                bounds[0], bounds[1], bounds[2], bounds[3]
            ),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Object(_) => "object",
            Self::Molecule(_) => "molecule",
            Self::Node(_) => "node",
            Self::Bond(_) => "bond",
            Self::Bounds(_) => "bounds",
        }
    }

    fn to_json(&self) -> Value {
        match self {
            Self::All => json!({ "kind": self.kind(), "selector": self.selector() }),
            Self::Object(id) => {
                json!({ "kind": self.kind(), "selector": self.selector(), "id": id })
            }
            Self::Molecule(index) => {
                json!({ "kind": self.kind(), "selector": self.selector(), "index": index })
            }
            Self::Node(id) | Self::Bond(id) => {
                json!({ "kind": self.kind(), "selector": self.selector(), "id": id })
            }
            Self::Bounds(bounds) => json!({
                "kind": self.kind(),
                "selector": self.selector(),
                "bounds": bounds_json(*bounds),
            }),
        }
    }
}

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
    let objects = object_target_entries(&document);
    let molecules = molecule_target_entries(&document);
    let nodes = node_target_entries(&document);
    let bonds = bond_target_entries(&document);
    let target_count = 1 + objects.len() + molecules.len() + nodes.len() + bonds.len();
    let all_bounds = target_bounds(&document, &TargetSelector::All).ok();
    write_json_value(
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
        }),
        output.as_deref(),
        pretty,
    )
}

pub(crate) fn capture_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut padding = 8.0;
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
            "--bounds" => {
                index += 1;
                target = Some(TargetSelector::Bounds(parse_bounds_arg(
                    args.get(index)
                        .ok_or_else(|| "--bounds requires minX,minY,maxX,maxY.".to_string())?,
                )?));
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--padding" => {
                index += 1;
                padding = parse_non_negative_f64(
                    "--padding",
                    args.get(index)
                        .ok_or_else(|| "--padding requires a number.".to_string())?,
                )?;
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected capture argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "capture requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "capture requires --target <object:id|molecule:index|node:id|bond:id|all> or --bounds."
            .to_string()
    })?;
    let output = output.ok_or_else(|| "capture requires --out <path.svg>.".to_string())?;
    if output == "-" {
        return Err(
            "capture writes image data to --out; stdout is reserved for the JSON manifest."
                .to_string(),
        );
    }
    ensure_svg_output_path(&output)?;

    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let bounds = target_bounds(&document, &target)?;
    let view_box = expanded_view_box(bounds, padding);
    let primitives = render_document(&document);
    let svg = primitives_to_svg_viewbox(&primitives, view_box, None);
    write_text_output(Some(&output), &svg)?;
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
            "output": {
                "path": output,
                "format": "svg",
            },
            "bounds": bounds_json(bounds),
            "viewBox": view_box_json(view_box),
            "padding": padding,
        }),
        None,
        pretty,
    )
}

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
    let payload_path = payload_path.unwrap_or_else(default_clipboard_payload_path);
    write_clipboard_payload_file(&payload_path, &payload)?;

    let copied_helper = if copy_to_clipboard {
        Some(copy_payload_to_office_clipboard(
            &payload_path,
            office_helper.as_deref(),
        )?)
    } else {
        None
    };
    let payload_bytes = fs::metadata(&payload_path)
        .ok()
        .map(|metadata| metadata.len());
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
            "payload": {
                "path": payload_path.display().to_string(),
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

pub(crate) fn parse_target_selector(value: &str) -> Result<TargetSelector, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("all") {
        return Ok(TargetSelector::All);
    }
    let Some((kind, id)) = value.split_once(':') else {
        return Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, or bond:<id>."
        ));
    };
    let id = id.trim();
    if id.is_empty() {
        return Err(format!(
            "Invalid target selector '{value}': target id is empty."
        ));
    }
    match kind.trim().to_ascii_lowercase().as_str() {
        "object" | "obj" => Ok(TargetSelector::Object(id.to_string())),
        "molecule" | "mol" => id
            .parse::<usize>()
            .map(TargetSelector::Molecule)
            .map_err(|_| format!("Invalid molecule target '{value}': molecule index must be a non-negative integer.")),
        "node" | "atom" => Ok(TargetSelector::Node(id.to_string())),
        "bond" => Ok(TargetSelector::Bond(id.to_string())),
        "bounds" => parse_bounds_arg(id).map(TargetSelector::Bounds),
        _ => Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, or bond:<id>."
        )),
    }
}

fn parse_usize_arg(name: &str, value: Option<&String>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{name} requires a non-negative integer."))?
        .parse::<usize>()
        .map_err(|_| format!("{name} requires a non-negative integer."))
}

fn parse_non_negative_f64(name: &str, value: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a number."))?;
    if !number.is_finite() || number < 0.0 {
        return Err(format!("{name} requires a finite non-negative number."));
    }
    Ok(number)
}

fn parse_bounds_arg(value: &str) -> Result<[f64; 4], String> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("Bounds must use minX,minY,maxX,maxY.".to_string());
    }
    let mut numbers = [0.0; 4];
    for (index, part) in parts.iter().enumerate() {
        numbers[index] = part
            .parse::<f64>()
            .map_err(|_| "Bounds values must be finite numbers.".to_string())?;
        if !numbers[index].is_finite() {
            return Err("Bounds values must be finite numbers.".to_string());
        }
    }
    if numbers[2] <= numbers[0] || numbers[3] <= numbers[1] {
        return Err("Bounds must satisfy maxX > minX and maxY > minY.".to_string());
    }
    Ok(numbers)
}

fn engine_document(engine: &Engine) -> Result<ChemcoreDocument, String> {
    serde_json::from_str(&document_json(engine)?)
        .map_err(|error| format!("Failed to parse engine document JSON: {error}"))
}

fn object_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    let mut entries = Vec::new();
    collect_object_target_entries(document, &document.objects, None, 0, &mut entries);
    entries
}

fn collect_object_target_entries(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    parent_id: Option<&str>,
    depth: usize,
    entries: &mut Vec<Value>,
) {
    for object in objects {
        let bounds = target_bounds(document, &TargetSelector::Object(object.id.clone()))
            .ok()
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

fn molecule_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
    document
        .editable_fragments()
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let bounds = target_bounds(document, &TargetSelector::Molecule(index))
                .ok()
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

fn node_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
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

fn bond_target_entries(document: &ChemcoreDocument) -> Vec<Value> {
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

fn world_node_position(object: &SceneObject, node: &Node) -> [f64; 2] {
    [
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    ]
}

fn node_fast_bounds(object: &SceneObject, node: &Node) -> [f64; 4] {
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    if let Some(bounds) = node.label.as_ref().and_then(|label| label.bbox()) {
        return [
            bounds[0] + tx,
            bounds[1] + ty,
            bounds[2] + tx,
            bounds[3] + ty,
        ];
    }
    let point = world_node_position(object, node);
    [
        point[0] - 4.0,
        point[1] - 4.0,
        point[0] + 4.0,
        point[1] + 4.0,
    ]
}

fn bond_fast_bounds(object: &SceneObject, nodes: &[Node], bond: &Bond) -> Option<[f64; 4]> {
    let begin = nodes.iter().find(|node| node.id == bond.begin)?;
    let end = nodes.iter().find(|node| node.id == bond.end)?;
    let begin = world_node_position(object, begin);
    let end = world_node_position(object, end);
    Some([
        begin[0].min(end[0]) - 4.0,
        begin[1].min(end[1]) - 4.0,
        begin[0].max(end[0]) + 4.0,
        begin[1].max(end[1]) + 4.0,
    ])
}

fn target_bounds(document: &ChemcoreDocument, target: &TargetSelector) -> Result<[f64; 4], String> {
    if let TargetSelector::Bounds(bounds) = target {
        return Ok(*bounds);
    }
    let primitives = render_primitives_for_target(document, target)?;
    render_primitives_bounds(primitives.iter()).ok_or_else(|| {
        format!(
            "No visible render primitives found for target '{}'.",
            target.selector()
        )
    })
}

fn render_primitives_for_target(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Result<Vec<RenderPrimitive>, String> {
    match target {
        TargetSelector::All => Ok(render_document(document)),
        TargetSelector::Bounds(_) => Ok(render_document(document)),
        TargetSelector::Object(id) => {
            if document.find_scene_object(id).is_none() {
                return Err(format!("Object target not found: {id}. Run 'chemcore-cli targets <input>' to list valid selectors."));
            }
            let nodes = BTreeSet::new();
            let bonds = BTreeSet::new();
            let mut objects = BTreeSet::new();
            objects.insert(id.clone());
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
        TargetSelector::Molecule(index) => {
            let object_id = molecule_object_id(document, *index)?;
            let nodes = BTreeSet::new();
            let bonds = BTreeSet::new();
            let mut objects = BTreeSet::new();
            objects.insert(object_id);
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
        TargetSelector::Node(id) => {
            if !node_exists(document, id) {
                return Err(format!("Node target not found: {id}. Run 'chemcore-cli targets <input>' to list valid selectors."));
            }
            let mut nodes = BTreeSet::new();
            let bonds = BTreeSet::new();
            let objects = BTreeSet::new();
            nodes.insert(id.clone());
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
        TargetSelector::Bond(id) => {
            if !bond_exists(document, id) {
                return Err(format!("Bond target not found: {id}. Run 'chemcore-cli targets <input>' to list valid selectors."));
            }
            let nodes = BTreeSet::new();
            let mut bonds = BTreeSet::new();
            let objects = BTreeSet::new();
            bonds.insert(id.clone());
            Ok(render_document_targets(document, &nodes, &bonds, &objects))
        }
    }
}

fn molecule_object_id(document: &ChemcoreDocument, index: usize) -> Result<String, String> {
    let fragments = document.editable_fragments();
    fragments
        .get(index)
        .map(|entry| entry.object.id.clone())
        .ok_or_else(|| {
            format!(
                "Molecule target not found: molecule:{index}. Document has {} molecule target(s).",
                fragments.len()
            )
        })
}

fn node_exists(document: &ChemcoreDocument, node_id: &str) -> bool {
    document
        .editable_fragments()
        .into_iter()
        .any(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
}

fn bond_exists(document: &ChemcoreDocument, bond_id: &str) -> bool {
    document
        .editable_fragments()
        .into_iter()
        .any(|entry| entry.fragment.bonds.iter().any(|bond| bond.id == bond_id))
}

fn expanded_view_box(bounds: [f64; 4], padding: f64) -> [f64; 4] {
    let min_x = bounds[0] - padding;
    let min_y = bounds[1] - padding;
    let width = (bounds[2] - bounds[0] + padding * 2.0).max(1.0);
    let height = (bounds[3] - bounds[1] + padding * 2.0).max(1.0);
    [min_x, min_y, width, height]
}

fn bounds_json(bounds: [f64; 4]) -> Value {
    json!({
        "minX": bounds[0],
        "minY": bounds[1],
        "maxX": bounds[2],
        "maxY": bounds[3],
        "width": bounds[2] - bounds[0],
        "height": bounds[3] - bounds[1],
    })
}

fn view_box_json(view_box: [f64; 4]) -> Value {
    json!({
        "x": view_box[0],
        "y": view_box[1],
        "width": view_box[2],
        "height": view_box[3],
        "value": view_box,
    })
}

fn ensure_svg_output_path(path: &str) -> Result<(), String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());
    if extension.as_deref() != Some("svg") {
        return Err("capture currently writes SVG only. Use --out <path.svg>; for example: chemcore-cli capture input.cdxml --target all --out capture.svg."
            .to_string());
    }
    Ok(())
}

fn clipboard_document_for_target(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Result<ChemcoreDocument, String> {
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
    };
    clipboard_document.document.id = "doc_clipboard_selection".to_string();
    clipboard_document.document.title = "Chemcore Clipboard Selection".to_string();
    set_clipboard_selection_bounds_meta(&mut clipboard_document, bounds);
    Ok(clipboard_document)
}

fn clipboard_document_for_object(
    document: &ChemcoreDocument,
    object_id: &str,
) -> Result<ChemcoreDocument, String> {
    let objects = clone_scene_object_path_by_id(&document.objects, object_id)
        .ok_or_else(|| format!("Object target not found: {object_id}."))?;
    let mut out = document.clone();
    out.objects = objects;
    Ok(out)
}

fn clone_scene_object_path_by_id(
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

fn clipboard_document_for_fragment_target(
    document: &ChemcoreDocument,
    node_id: Option<&str>,
    bond_id: Option<&str>,
) -> Result<ChemcoreDocument, String> {
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
        out.resources.insert(resource_ref, resource);
        return Ok(out);
    }
    match (node_id, bond_id) {
        (Some(id), _) => Err(format!("Node target not found: {id}.")),
        (_, Some(id)) => Err(format!("Bond target not found: {id}.")),
        _ => Err("No fragment target was provided.".to_string()),
    }
}

fn fragment_clipboard_bounds(nodes: &[Node]) -> [f64; 4] {
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

fn set_clipboard_selection_bounds_meta(document: &mut ChemcoreDocument, bounds: [f64; 4]) {
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

fn clipboard_payload_for_document(document: &ChemcoreDocument) -> Result<Value, String> {
    let chemcore_document_json =
        serde_json::to_string(document).map_err(|error| error.to_string())?;
    let render_list_json =
        serde_json::to_string(&render_document(document)).map_err(|error| error.to_string())?;
    let cdxml = document_to_cdxml(document);
    let svg = document_to_svg(document);
    Ok(json!({
        "text": cdxml,
        "chemcoreFragmentJson": Value::Null,
        "chemcoreDocumentJson": chemcore_document_json,
        "renderListJson": render_list_json,
        "cdxml": cdxml,
        "svg": svg,
    }))
}

fn default_clipboard_payload_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "chemcore-cli-copy-{}-{timestamp}.json",
        std::process::id()
    ))
}

fn write_clipboard_payload_file(path: &Path, payload: &Value) -> Result<(), String> {
    ensure_output_parent_path(path)?;
    let text = serde_json::to_string_pretty(payload).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| {
        format!(
            "Failed to write clipboard payload {}: {error}",
            path.display()
        )
    })
}

fn ensure_output_parent_path(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create output directory {}: {error}",
            parent.display()
        )
    })
}

#[cfg(windows)]
fn copy_payload_to_office_clipboard(
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
fn copy_payload_to_office_clipboard(
    payload_path: &Path,
    _office_helper: Option<&str>,
) -> Result<PathBuf, String> {
    Err(format!(
        "Copying to the Office/OLE clipboard is only supported on Windows. Payload was written to {}.",
        payload_path.display()
    ))
}

#[cfg(windows)]
fn resolve_office_helper(office_helper: Option<&str>) -> Result<PathBuf, String> {
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
            candidates.push(dir.join("chemcore-office.exe"));
            candidates.push(dir.join("resources").join("chemcore-office.exe"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("resources").join("chemcore-office.exe"));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(
            cwd.join("target")
                .join("release")
                .join("chemcore-office.exe"),
        );
        candidates.push(cwd.join("target").join("debug").join("chemcore-office.exe"));
    }

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "chemcore-office.exe was not found. Pass --office-helper <path>. Checked: {}",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("; ")
    ))
}
