use super::*;

pub(super) fn add_target_arg(
    target: &mut Option<TargetSelector>,
    next: TargetSelector,
) -> Result<(), String> {
    match target.take() {
        None => *target = Some(next),
        Some(existing) => {
            let mut targets = Vec::new();
            collect_selection_targets(existing, &mut targets);
            collect_selection_targets(next, &mut targets);
            *target = Some(target_from_selection_targets(targets)?);
        }
    }
    Ok(())
}

pub(super) fn collect_selection_targets(target: TargetSelector, out: &mut Vec<TargetSelector>) {
    match target {
        TargetSelector::Selection(targets) => {
            for target in targets {
                collect_selection_targets(target, out);
            }
        }
        target => out.push(target),
    }
}

pub(super) fn target_from_selection_targets(
    mut targets: Vec<TargetSelector>,
) -> Result<TargetSelector, String> {
    if targets.is_empty() {
        return Err("Selection requires at least one target.".to_string());
    }
    if targets.len() == 1 {
        return Ok(targets.remove(0));
    }
    if targets
        .iter()
        .any(|target| matches!(target, TargetSelector::All))
    {
        return Err("Multi-target selection uses object, molecule, node, bond, or bounds selectors; use all by itself for whole-document capture.".to_string());
    }
    Ok(TargetSelector::Selection(targets))
}

pub(crate) fn parse_target_selector(value: &str) -> Result<TargetSelector, String> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("all") {
        return Ok(TargetSelector::All);
    }
    if value.contains(';') {
        return parse_target_selection_arg(value);
    }
    let Some((kind, id)) = value.split_once(':') else {
        return Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, bond:<id>, bounds:<minX,minY,maxX,maxY>, or selection:<selector;selector>."
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
        "selection" | "targets" => parse_target_selection_arg(id),
        _ => Err(format!(
            "Invalid target selector '{value}'. Expected all, object:<id>, molecule:<index>, node:<id>, bond:<id>, bounds:<minX,minY,maxX,maxY>, or selection:<selector;selector>."
        )),
    }
}

pub(super) fn parse_target_selection_arg(value: &str) -> Result<TargetSelector, String> {
    let mut targets = Vec::new();
    for part in value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        collect_selection_targets(parse_target_selector(part)?, &mut targets);
    }
    target_from_selection_targets(targets)
}

pub(super) fn parse_usize_arg(name: &str, value: Option<&String>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{name} requires a non-negative integer."))?
        .parse::<usize>()
        .map_err(|_| format!("{name} requires a non-negative integer."))
}

pub(super) fn parse_non_negative_f64(name: &str, value: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a number."))?;
    if !number.is_finite() || number < 0.0 {
        return Err(format!("{name} requires a finite non-negative number."));
    }
    Ok(number)
}

pub(super) fn parse_positive_f64(name: &str, value: &str) -> Result<f64, String> {
    let number = value
        .parse::<f64>()
        .map_err(|_| format!("{name} requires a positive number."))?;
    if !number.is_finite() || number <= 0.0 {
        return Err(format!("{name} requires a finite positive number."));
    }
    Ok(number)
}

pub(super) fn parse_positive_u32(name: &str, value: &str) -> Result<u32, String> {
    let number = value
        .parse::<u32>()
        .map_err(|_| format!("{name} requires a positive integer."))?;
    if number == 0 {
        return Err(format!("{name} requires a positive integer."));
    }
    Ok(number)
}

pub(super) fn parse_capture_format(value: &str) -> Result<CaptureFormat, String> {
    match value
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "svg" => Ok(CaptureFormat::Svg),
        "png" => Ok(CaptureFormat::Png),
        _ => Err(format!(
            "Unsupported capture format '{value}'. Expected svg or png."
        )),
    }
}

pub(super) fn infer_capture_format_from_path(path: &str) -> Option<CaptureFormat> {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| parse_capture_format(extension).ok())
}

pub(super) fn target_bounds_fast(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Option<[f64; 4]> {
    match target {
        TargetSelector::All => document_fast_bounds(document),
        TargetSelector::Bounds(bounds) => Some(*bounds),
        TargetSelector::Selection(targets) => {
            let mut out = None;
            for target in targets {
                include_bounds(&mut out, target_bounds_fast(document, target)?);
            }
            out
        }
        TargetSelector::Object(id) => document
            .find_scene_object(id)
            .and_then(|object| scene_object_fast_bounds(document, object)),
        TargetSelector::Molecule(index) => {
            let fragments = document.editable_fragments();
            let entry = fragments.get(*index)?;
            molecule_object_fast_bounds(document, entry.object)
        }
        TargetSelector::Node(id) => {
            for entry in document.editable_fragments() {
                if let Some(node) = entry.fragment.nodes.iter().find(|node| &node.id == id) {
                    return Some(node_fast_bounds(entry.object, node));
                }
            }
            None
        }
        TargetSelector::Bond(id) => {
            for entry in document.editable_fragments() {
                if let Some(bond) = entry.fragment.bonds.iter().find(|bond| &bond.id == id) {
                    return bond_fast_bounds(entry.object, &entry.fragment.nodes, bond);
                }
            }
            None
        }
    }
}

pub(super) fn document_fast_bounds(document: &ChemcoreDocument) -> Option<[f64; 4]> {
    let mut out = None;
    for object in &document.objects {
        if !object.visible {
            continue;
        }
        if let Some(bounds) = scene_object_fast_bounds(document, object) {
            include_bounds(&mut out, bounds);
        }
    }
    out
}

pub(super) fn scene_object_fast_bounds(
    document: &ChemcoreDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    if !object.visible {
        return None;
    }
    if object.object_type == "group" {
        let mut out = None;
        for child in &object.children {
            if let Some(bounds) = scene_object_fast_bounds(document, child) {
                include_bounds(&mut out, bounds);
            }
        }
        return out.or_else(|| scene_object_bbox_bounds(object));
    }
    if object.object_type == "molecule" {
        return molecule_object_fast_bounds(document, object);
    }
    scene_object_bbox_bounds(object)
}

pub(super) fn molecule_object_fast_bounds(
    document: &ChemcoreDocument,
    object: &SceneObject,
) -> Option<[f64; 4]> {
    scene_object_bbox_bounds(object).or_else(|| {
        let resource_ref = object.payload.resource_ref.as_ref()?;
        let fragment = document.resources.get(resource_ref)?.data.as_fragment()?;
        local_bbox_world_bounds(object, fragment.bbox)
    })
}

pub(super) fn scene_object_bbox_bounds(object: &SceneObject) -> Option<[f64; 4]> {
    local_bbox_world_bounds(object, object.payload.bbox?)
}

pub(super) fn local_bbox_world_bounds(object: &SceneObject, bbox: [f64; 4]) -> Option<[f64; 4]> {
    let [x, y, width, height] = bbox;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let min_x = tx + x;
    let min_y = ty + y;
    let max_x = tx + x + width;
    let max_y = ty + y + height;
    if object.transform.rotate.abs() <= f64::EPSILON {
        return Some([min_x, min_y, max_x, max_y]);
    }

    let center = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
    let mut bounds = rotate_point_bounds([min_x, min_y], center, object.transform.rotate);
    for point in [[max_x, min_y], [max_x, max_y], [min_x, max_y]] {
        let rotated = rotate_point_bounds(point, center, object.transform.rotate);
        bounds[0] = bounds[0].min(rotated[0]);
        bounds[1] = bounds[1].min(rotated[1]);
        bounds[2] = bounds[2].max(rotated[2]);
        bounds[3] = bounds[3].max(rotated[3]);
    }
    Some(bounds)
}

pub(super) fn include_bounds(out: &mut Option<[f64; 4]>, bounds: [f64; 4]) {
    *out = Some(match *out {
        Some(current) => [
            current[0].min(bounds[0]),
            current[1].min(bounds[1]),
            current[2].max(bounds[2]),
            current[3].max(bounds[3]),
        ],
        None => bounds,
    });
}

pub(super) fn rotate_point_bounds(point: [f64; 2], center: [f64; 2], degrees: f64) -> [f64; 4] {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point[0] - center[0];
    let dy = point[1] - center[1];
    let x = center[0] + dx * cos - dy * sin;
    let y = center[1] + dx * sin + dy * cos;
    [x, y, x, y]
}

pub(super) fn set_object_field(value: &mut Value, key: &str, field: Value) {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_string(), field);
    }
}

pub(super) fn parse_bounds_arg(value: &str) -> Result<[f64; 4], String> {
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

pub(super) fn engine_document(engine: &Engine) -> Result<ChemcoreDocument, String> {
    serde_json::from_str(&document_json(engine)?)
        .map_err(|error| format!("Failed to parse engine document JSON: {error}"))
}

pub(super) fn world_node_position(object: &SceneObject, node: &Node) -> [f64; 2] {
    [
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    ]
}

pub(super) fn node_fast_bounds(object: &SceneObject, node: &Node) -> [f64; 4] {
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

pub(super) fn bond_fast_bounds(
    object: &SceneObject,
    nodes: &[Node],
    bond: &Bond,
) -> Option<[f64; 4]> {
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

pub(super) fn target_bounds(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Result<[f64; 4], String> {
    if let TargetSelector::Bounds(bounds) = target {
        return Ok(*bounds);
    }
    if let TargetSelector::Selection(targets) = target {
        let mut out = None;
        for target in targets {
            include_bounds(&mut out, target_bounds(document, target)?);
        }
        return out.ok_or_else(|| "Selection target has no members.".to_string());
    }
    if let Some(bounds) = target_bounds_fast(document, target) {
        return Ok(bounds);
    }
    let primitives = render_primitives_for_target(document, target)?;
    render_primitives_bounds(primitives.iter()).ok_or_else(|| {
        format!(
            "No visible render primitives found for target '{}'.",
            target.selector()
        )
    })
}

pub(super) fn render_primitives_for_target(
    document: &ChemcoreDocument,
    target: &TargetSelector,
) -> Result<Vec<RenderPrimitive>, String> {
    match target {
        TargetSelector::All => Ok(render_document(document)),
        TargetSelector::Bounds(_) => Ok(render_document(document)),
        TargetSelector::Selection(targets) => {
            let mut primitives = Vec::new();
            for target in targets {
                primitives.extend(render_primitives_for_target(document, target)?);
            }
            Ok(primitives)
        }
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

pub(super) fn molecule_object_id(
    document: &ChemcoreDocument,
    index: usize,
) -> Result<String, String> {
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

pub(super) fn node_exists(document: &ChemcoreDocument, node_id: &str) -> bool {
    document
        .editable_fragments()
        .into_iter()
        .any(|entry| entry.fragment.nodes.iter().any(|node| node.id == node_id))
}

pub(super) fn bond_exists(document: &ChemcoreDocument, bond_id: &str) -> bool {
    document
        .editable_fragments()
        .into_iter()
        .any(|entry| entry.fragment.bonds.iter().any(|bond| bond.id == bond_id))
}

pub(super) fn expanded_view_box(bounds: [f64; 4], expansion: CropExpansion) -> [f64; 4] {
    let width = (bounds[2] - bounds[0]).max(1.0);
    let height = (bounds[3] - bounds[1]).max(1.0);
    let left = expansion.left_for(width);
    let right = expansion.right_for(width);
    let top = expansion.top_for(height);
    let bottom = expansion.bottom_for(height);
    let min_x = bounds[0] - left;
    let min_y = bounds[1] - top;
    let width = (width + left + right).max(1.0);
    let height = (height + top + bottom).max(1.0);
    [min_x, min_y, width, height]
}

pub(super) fn bounds_json(bounds: [f64; 4]) -> Value {
    json!({
        "minX": bounds[0],
        "minY": bounds[1],
        "maxX": bounds[2],
        "maxY": bounds[3],
        "width": bounds[2] - bounds[0],
        "height": bounds[3] - bounds[1],
    })
}

pub(super) fn view_box_json(view_box: [f64; 4]) -> Value {
    json!({
        "x": view_box[0],
        "y": view_box[1],
        "width": view_box[2],
        "height": view_box[3],
        "value": view_box,
    })
}
