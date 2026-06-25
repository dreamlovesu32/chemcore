use super::*;

pub(super) fn clear_select_hover_overlay(engine: &mut Engine) {
    engine.state.overlay.hover_bond_center = None;
    engine.state.overlay.hover_arrow = None;
    engine.state.overlay.hover_shape = None;
    engine.state.overlay.hover_text_box = None;
    engine.state.overlay.hover_endpoint = None;
    engine.state.overlay.preview = None;
    engine.pointer_bond_target = None;
}

#[derive(Default)]
pub(super) struct GroupSelectionOverlay {
    complete_group_ids: BTreeSet<String>,
    hidden_descendant_ids: BTreeSet<String>,
    selected_group_descendant_ids: BTreeSet<String>,
}

impl GroupSelectionOverlay {
    pub(super) fn group_is_complete(&self, object_id: &str) -> bool {
        self.complete_group_ids.contains(object_id)
    }

    pub(super) fn hides_object(&self, object_id: &str) -> bool {
        self.hidden_descendant_ids.contains(object_id)
    }

    pub(super) fn selected_group_hides_object(&self, object_id: &str) -> bool {
        self.selected_group_descendant_ids.contains(object_id)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ObjectSelectionCoverage {
    None,
    Partial,
    Complete,
}

pub(super) fn group_selection_overlay(engine: &Engine) -> GroupSelectionOverlay {
    let mut overlay = GroupSelectionOverlay::default();
    for object in &engine.state.document.objects {
        collect_complete_group_selection(
            &engine.state.document,
            &engine.state.selection,
            object,
            &mut overlay,
        );
    }
    overlay
}

fn collect_complete_group_selection(
    document: &crate::ChemcoreDocument,
    selection: &SelectionState,
    object: &crate::SceneObject,
    overlay: &mut GroupSelectionOverlay,
) {
    if object.object_type != "group" {
        return;
    }
    if scene_object_selection_coverage(document, selection, object)
        == ObjectSelectionCoverage::Complete
    {
        overlay.complete_group_ids.insert(object.id.clone());
        collect_descendant_ids(object, &mut overlay.hidden_descendant_ids);
        if selection
            .arrow_objects
            .iter()
            .any(|object_id| object_id == &object.id)
        {
            collect_descendant_ids(object, &mut overlay.selected_group_descendant_ids);
        }
        return;
    }
    for child in &object.children {
        collect_complete_group_selection(document, selection, child, overlay);
    }
}

fn collect_descendant_ids(object: &crate::SceneObject, out: &mut BTreeSet<String>) {
    for child in &object.children {
        out.insert(child.id.clone());
        collect_descendant_ids(child, out);
    }
}

fn scene_object_selection_coverage(
    document: &crate::ChemcoreDocument,
    selection: &SelectionState,
    object: &crate::SceneObject,
) -> ObjectSelectionCoverage {
    if !object.visible {
        return ObjectSelectionCoverage::None;
    }
    match object.object_type.as_str() {
        "text" => selected_coverage(selection.text_objects.iter(), &object.id),
        "line" | "bracket" | "symbol" | "shape" => {
            selected_coverage(selection.arrow_objects.iter(), &object.id)
        }
        "molecule" => molecule_selection_coverage(document, selection, object),
        "group" => group_selection_coverage(document, selection, object),
        _ => ObjectSelectionCoverage::None,
    }
}

fn selected_coverage<'a>(
    selected_ids: impl Iterator<Item = &'a String>,
    object_id: &str,
) -> ObjectSelectionCoverage {
    if selected_ids
        .into_iter()
        .any(|selected_id| selected_id == object_id)
    {
        ObjectSelectionCoverage::Complete
    } else {
        ObjectSelectionCoverage::None
    }
}

fn group_selection_coverage(
    document: &crate::ChemcoreDocument,
    selection: &SelectionState,
    object: &crate::SceneObject,
) -> ObjectSelectionCoverage {
    let group_id_selected = selection
        .arrow_objects
        .iter()
        .any(|object_id| object_id == &object.id);
    let mut selectable_child_count = 0usize;
    let mut complete_child_count = 0usize;
    let mut any_child_selected = false;

    for child in object.children.iter().filter(|child| child.visible) {
        let coverage = scene_object_selection_coverage(document, selection, child);
        if coverage == ObjectSelectionCoverage::None && !scene_object_is_selectable(child) {
            continue;
        }
        selectable_child_count += 1;
        match coverage {
            ObjectSelectionCoverage::Complete => {
                complete_child_count += 1;
                any_child_selected = true;
            }
            ObjectSelectionCoverage::Partial => any_child_selected = true,
            ObjectSelectionCoverage::None => {}
        }
    }

    if group_id_selected && !selection.region {
        return ObjectSelectionCoverage::Complete;
    }
    if selectable_child_count > 0 && complete_child_count == selectable_child_count {
        return ObjectSelectionCoverage::Complete;
    }
    if group_id_selected || any_child_selected {
        ObjectSelectionCoverage::Partial
    } else {
        ObjectSelectionCoverage::None
    }
}

fn scene_object_is_selectable(object: &crate::SceneObject) -> bool {
    matches!(
        object.object_type.as_str(),
        "text" | "line" | "bracket" | "symbol" | "shape" | "molecule" | "group"
    )
}

fn molecule_selection_coverage(
    document: &crate::ChemcoreDocument,
    selection: &SelectionState,
    object: &crate::SceneObject,
) -> ObjectSelectionCoverage {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return ObjectSelectionCoverage::None;
    };
    let Some(fragment) = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
    else {
        return ObjectSelectionCoverage::None;
    };

    let selected_nodes: BTreeSet<&str> = selection.nodes.iter().map(String::as_str).collect();
    let selected_label_nodes: BTreeSet<&str> =
        selection.label_nodes.iter().map(String::as_str).collect();
    let selected_bonds: BTreeSet<&str> = selection.bonds.iter().map(String::as_str).collect();
    let any_selected = fragment.nodes.iter().any(|node| {
        selected_nodes.contains(node.id.as_str()) || selected_label_nodes.contains(node.id.as_str())
    }) || fragment
        .bonds
        .iter()
        .any(|bond| selected_bonds.contains(bond.id.as_str()));
    if !any_selected {
        return ObjectSelectionCoverage::None;
    }

    let all_nodes_selected = fragment
        .nodes
        .iter()
        .all(|node| selected_nodes.contains(node.id.as_str()));
    let all_bonds_selected = fragment
        .bonds
        .iter()
        .all(|bond| selected_bonds.contains(bond.id.as_str()));
    if all_nodes_selected && all_bonds_selected {
        ObjectSelectionCoverage::Complete
    } else {
        ObjectSelectionCoverage::Partial
    }
}

pub(super) fn render_selected_text_boxes(
    engine: &Engine,
    overlay: &GroupSelectionOverlay,
    out: &mut Vec<RenderPrimitive>,
) {
    let selected_text_objects: BTreeSet<&str> = engine
        .state
        .selection
        .text_objects
        .iter()
        .map(String::as_str)
        .collect();
    for object in engine.state.document.scene_objects() {
        if !selected_text_objects.contains(object.id.as_str()) {
            continue;
        }
        if overlay.hides_object(&object.id) {
            continue;
        }
        let Some(bounds) = text_object_world_bounds(object) else {
            continue;
        };
        push_selection_box(
            out,
            AxisBounds::from_array(bounds),
            RenderRole::SelectionTextBox,
        );
    }
}

pub(super) fn render_selected_arrow_handles(
    engine: &Engine,
    overlay: &GroupSelectionOverlay,
    out: &mut Vec<RenderPrimitive>,
) {
    for object in engine.state.document.scene_objects() {
        if overlay.hides_object(&object.id) {
            continue;
        }
        if object.object_type == "group" {
            if !overlay.group_is_complete(&object.id) {
                continue;
            }
        } else if !engine.state.selection.arrow_objects.contains(&object.id) {
            continue;
        }
        if let Some(bounds) = scene_object_selection_bounds(&engine.state.document, object) {
            push_selection_box(out, bounds, RenderRole::SelectionBox);
        }
    }
}

pub(super) fn scene_object_selection_bounds(
    document: &crate::ChemcoreDocument,
    object: &crate::SceneObject,
) -> Option<AxisBounds> {
    if object.object_type == "text" {
        return text_object_world_bounds(object).map(AxisBounds::from_array);
    }
    if object.object_type == "shape" {
        return shape_object_visual_bounds(document, object)
            .map(AxisBounds::from_array)
            .or_else(|| shape_object_selection_bounds(object))
            .or_else(|| object_bbox_selection_bounds(object));
    }
    if object.object_type == "group" {
        return group_object_selection_bounds(document, object);
    }
    if object.object_type == "molecule" {
        return molecule_object_selection_bounds(document, object);
    }
    if object.object_type == "line" {
        return line_object_visual_bounds(document, object)
            .map(AxisBounds::from_array)
            .or_else(|| arrow_object_selection_bounds(document, object))
            .or_else(|| object_bbox_selection_bounds(object));
    }
    if matches!(object.object_type.as_str(), "bracket" | "symbol") {
        return bracket_object_visual_bounds(document, object)
            .map(AxisBounds::from_array)
            .or_else(|| object_bbox_selection_bounds(object));
    }
    arrow_object_selection_bounds(document, object)
}

fn group_object_selection_bounds(
    document: &crate::ChemcoreDocument,
    object: &crate::SceneObject,
) -> Option<AxisBounds> {
    let mut out = None;
    for child in &object.children {
        if !child.visible {
            continue;
        }
        if let Some(bounds) = scene_object_selection_bounds(document, child) {
            include_optional_bounds(&mut out, bounds);
        }
    }
    out.or_else(|| object_bbox_selection_bounds(object))
}

fn molecule_object_selection_bounds(
    document: &crate::ChemcoreDocument,
    object: &crate::SceneObject,
) -> Option<AxisBounds> {
    object_bbox_selection_bounds(object)
        .or_else(|| molecule_fragment_selection_bounds(document, object))
}

fn molecule_fragment_selection_bounds(
    document: &crate::ChemcoreDocument,
    object: &crate::SceneObject,
) -> Option<AxisBounds> {
    let resource_ref = object.payload.resource_ref.as_ref()?;
    let fragment = document.resources.get(resource_ref)?.data.as_fragment()?;
    let entry = crate::EditableFragment { object, fragment };
    let component = ComponentSelection {
        node_ids: fragment.nodes.iter().map(|node| node.id.clone()).collect(),
        label_node_ids: fragment
            .nodes
            .iter()
            .filter_map(|node| node.label.as_ref().map(|_| node.id.clone()))
            .collect(),
        bond_ids: fragment.bonds.iter().map(|bond| bond.id.clone()).collect(),
        complete: true,
    };
    component_selection_items(document, &entry, &component)
        .into_iter()
        .map(|item| item.bounds)
        .reduce(|mut bounds, item_bounds| {
            bounds.include_bounds(item_bounds);
            bounds
        })
}

pub(super) fn arrow_object_selection_bounds(
    document: &crate::ChemcoreDocument,
    object: &crate::SceneObject,
) -> Option<AxisBounds> {
    let points = line_object_points(object);
    if points.len() < 2 {
        return None;
    }
    let stroke_width = line_object_graphic_stroke_width(document, object);
    let mut handles = arrow_object_handle_points(object, &points, stroke_width).into_iter();
    let first = handles.next()?;
    let mut bounds = AxisBounds::around_point(first, 0.0);
    for handle in handles {
        bounds.include_point(handle);
    }
    Some(bounds)
}

pub(super) fn object_bbox_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    let [x, y, width, height] = object.payload.bbox?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    if object.transform.rotate.abs() > crate::EPSILON {
        let center = Point::new(tx + x + width * 0.5, ty + y + height * 0.5);
        let mut bounds = AxisBounds::around_point(
            rotate_point_around(Point::new(tx + x, ty + y), center, object.transform.rotate),
            0.0,
        );
        for point in [
            Point::new(tx + x + width, ty + y),
            Point::new(tx + x + width, ty + y + height),
            Point::new(tx + x, ty + y + height),
        ] {
            bounds.include_point(rotate_point_around(point, center, object.transform.rotate));
        }
        return Some(bounds);
    }
    Some(AxisBounds::new(
        tx + x,
        ty + y,
        tx + x + width,
        ty + y + height,
    ))
}

fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
    let radians = degrees.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

fn shape_object_selection_bounds(object: &crate::SceneObject) -> Option<AxisBounds> {
    let kind = object
        .payload
        .extra
        .get("kind")
        .and_then(JsonValue::as_str)
        .unwrap_or("rect");
    if kind == "circle" {
        let center = shape_payload_point(object, "center")?;
        let radius = center.distance(shape_payload_point(object, "majorAxisEnd")?);
        if radius <= crate::EPSILON {
            return None;
        }
        return Some(AxisBounds::new(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        ));
    }
    if kind == "ellipse" {
        let center = shape_payload_point(object, "center")?;
        let major = shape_payload_point(object, "majorAxisEnd")?;
        let minor = shape_payload_point(object, "minorAxisEnd")?;
        let major_x = major.x - center.x;
        let major_y = major.y - center.y;
        let minor_x = minor.x - center.x;
        let minor_y = minor.y - center.y;
        let extent_x = (major_x * major_x + minor_x * minor_x).sqrt();
        let extent_y = (major_y * major_y + minor_y * minor_y).sqrt();
        if extent_x <= crate::EPSILON || extent_y <= crate::EPSILON {
            return None;
        }
        return Some(AxisBounds::new(
            center.x - extent_x,
            center.y - extent_y,
            center.x + extent_x,
            center.y + extent_y,
        ));
    }
    object_bbox_selection_bounds(object)
}

fn shape_payload_point(object: &crate::SceneObject, key: &str) -> Option<Point> {
    object
        .payload
        .extra
        .get(key)
        .and_then(JsonValue::as_array)
        .and_then(|coords| {
            Some(Point::new(
                coords.first()?.as_f64()?,
                coords.get(1)?.as_f64()?,
            ))
        })
}

pub(super) fn render_selected_fragment_content(
    engine: &Engine,
    overlay: &GroupSelectionOverlay,
    out: &mut Vec<RenderPrimitive>,
) {
    for entry in engine.state.document.editable_fragments() {
        if overlay.hides_object(&entry.object.id) {
            continue;
        }

        for component in selected_component_summaries_for_entry(engine, &entry) {
            let items = component_selection_items(&engine.state.document, &entry, &component);
            if items.is_empty() {
                continue;
            }
            if component.complete {
                let group_bounds = items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
                    acc.include_bounds(item.bounds);
                    acc
                });
                push_selection_box(out, group_bounds, RenderRole::SelectionBox);
                continue;
            }
            if items.len() == 1 {
                let item = items[0];
                push_selection_item_box(out, item);
                push_selection_bond_dot(out, item.center);
                continue;
            }
            let group_bounds = items.iter().skip(1).fold(items[0].bounds, |mut acc, item| {
                acc.include_bounds(item.bounds);
                acc
            });
            push_selection_box(out, group_bounds, RenderRole::SelectionBox);
            for item in items {
                push_selection_bond_dot(out, item.center);
            }
        }
    }
}

pub(super) fn selected_component_summaries_for_entry(
    engine: &Engine,
    entry: &crate::EditableFragment<'_>,
) -> Vec<ComponentSelection> {
    if engine.state.selection.nodes.is_empty()
        && engine.state.selection.label_nodes.is_empty()
        && engine.state.selection.bonds.is_empty()
    {
        return Vec::new();
    }
    let selected_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .nodes
        .iter()
        .map(String::as_str)
        .collect();
    let selected_bonds: BTreeSet<&str> = engine
        .state
        .selection
        .bonds
        .iter()
        .map(String::as_str)
        .collect();
    let selected_label_nodes: BTreeSet<&str> = engine
        .state
        .selection
        .label_nodes
        .iter()
        .map(String::as_str)
        .collect();
    let node_index: BTreeMap<&str, usize> = entry
        .fragment
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    let mut adjacency = vec![Vec::new(); entry.fragment.nodes.len()];
    let mut bond_endpoints = Vec::with_capacity(entry.fragment.bonds.len());
    for bond in &entry.fragment.bonds {
        let endpoints = node_index
            .get(bond.begin.as_str())
            .zip(node_index.get(bond.end.as_str()))
            .map(|(begin, end)| (*begin, *end));
        if let Some((begin, end)) = endpoints {
            adjacency[begin].push(end);
            adjacency[end].push(begin);
        }
        bond_endpoints.push(endpoints);
    }
    let mut visited = vec![false; entry.fragment.nodes.len()];
    let mut components = Vec::new();

    for start_index in 0..entry.fragment.nodes.len() {
        if visited[start_index] {
            continue;
        }
        let mut stack = vec![start_index];
        let mut component_node_indices = Vec::new();
        visited[start_index] = true;
        while let Some(index) = stack.pop() {
            component_node_indices.push(index);
            for neighbor in &adjacency[index] {
                if !visited[*neighbor] {
                    visited[*neighbor] = true;
                    stack.push(*neighbor);
                }
            }
        }
        let component_node_lookup: BTreeSet<usize> =
            component_node_indices.iter().copied().collect();
        let component_bond_indices: Vec<usize> = bond_endpoints
            .iter()
            .enumerate()
            .filter_map(|(index, endpoints)| {
                let (begin, end) = (*endpoints)?;
                (component_node_lookup.contains(&begin) && component_node_lookup.contains(&end))
                    .then_some(index)
            })
            .collect();
        let component_selected_nodes: Vec<String> = component_node_indices
            .iter()
            .filter_map(|index| {
                let node_id = &entry.fragment.nodes[*index].id;
                selected_nodes
                    .contains(node_id.as_str())
                    .then(|| node_id.clone())
            })
            .collect();
        let component_selected_label_nodes: Vec<String> = component_node_indices
            .iter()
            .filter_map(|index| {
                let node_id = &entry.fragment.nodes[*index].id;
                selected_label_nodes
                    .contains(node_id.as_str())
                    .then(|| node_id.clone())
            })
            .collect();
        let component_selected_bonds: Vec<String> = component_bond_indices
            .iter()
            .filter_map(|index| {
                let bond_id = &entry.fragment.bonds[*index].id;
                selected_bonds
                    .contains(bond_id.as_str())
                    .then(|| bond_id.clone())
            })
            .collect();
        if component_selected_nodes.is_empty()
            && component_selected_label_nodes.is_empty()
            && component_selected_bonds.is_empty()
        {
            continue;
        }
        let all_nodes_selected = component_node_indices
            .iter()
            .all(|index| selected_nodes.contains(entry.fragment.nodes[*index].id.as_str()));
        let all_bonds_selected = component_bond_indices
            .iter()
            .all(|index| selected_bonds.contains(entry.fragment.bonds[*index].id.as_str()));
        components.push(ComponentSelection {
            node_ids: component_selected_nodes,
            label_node_ids: component_selected_label_nodes,
            bond_ids: component_selected_bonds,
            complete: all_nodes_selected && all_bonds_selected,
        });
    }

    components
}

#[derive(Default)]
pub(super) struct BracketComponentObjectSelection {
    pub arrow_object_ids: Vec<String>,
    pub text_object_ids: Vec<String>,
}

pub(super) fn bracket_object_ids_containing_component(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component_node_ids: &[String],
) -> BracketComponentObjectSelection {
    let mut sample_points = Vec::new();
    for node_id in component_node_ids {
        if let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) {
            sample_points.push(entry.world_point_for_node(node));
        }
    }
    for bond in &entry.fragment.bonds {
        if !component_node_ids.contains(&bond.begin) || !component_node_ids.contains(&bond.end) {
            continue;
        }
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        sample_points.push(midpoint(
            entry.world_point_for_node(begin),
            entry.world_point_for_node(end),
        ));
    }

    let mut object_ids = BracketComponentObjectSelection::default();
    collect_bracket_object_ids_containing_points(
        &document.objects,
        &sample_points,
        document,
        &mut object_ids,
    );
    object_ids
}

fn collect_bracket_object_ids_containing_points(
    objects: &[crate::SceneObject],
    sample_points: &[Point],
    document: &crate::ChemcoreDocument,
    out: &mut BracketComponentObjectSelection,
) {
    for object in objects {
        if !object.visible {
            continue;
        }
        if object.object_type == "bracket"
            || (object.object_type == "group"
                && object.meta.get("kind").and_then(serde_json::Value::as_str)
                    == Some("bracket-group"))
        {
            if let Some(bounds) = object_bbox_selection_bounds(object) {
                if sample_points
                    .iter()
                    .any(|point| point_in_bounds(*point, bounds))
                {
                    push_unique_string(&mut out.arrow_object_ids, object.id.clone());
                    collect_linked_bracket_count_text_ids(
                        document,
                        &object.id,
                        &mut out.text_object_ids,
                    );
                }
            }
        }
        collect_bracket_object_ids_containing_points(
            &object.children,
            sample_points,
            document,
            out,
        );
    }
}

fn collect_linked_bracket_count_text_ids(
    document: &crate::ChemcoreDocument,
    bracket_id: &str,
    out: &mut Vec<String>,
) {
    for object in document.scene_objects() {
        if object.object_type != "text" || !object.visible {
            continue;
        }
        if object
            .meta
            .get("linkKind")
            .and_then(serde_json::Value::as_str)
            != Some("bracket-label")
        {
            continue;
        }
        if object
            .meta
            .get("linkedBracketObjectId")
            .and_then(serde_json::Value::as_str)
            != Some(bracket_id)
        {
            continue;
        }
        if object
            .meta
            .get("repeatUnitRole")
            .and_then(serde_json::Value::as_str)
            == Some("count")
        {
            push_unique_string(out, object.id.clone());
        }
    }
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

pub(super) fn component_selection_bounds_fast(
    entry: &crate::EditableFragment<'_>,
    component: &ComponentSelection,
) -> Option<AxisBounds> {
    let mut out = None;
    let mut label_node_ids = component.label_node_ids.clone();
    if component.complete {
        for node_id in &component.node_ids {
            if !label_node_ids.iter().any(|existing| existing == node_id) {
                label_node_ids.push(node_id.clone());
            }
        }
    }

    let mut label_items_added = 0usize;
    for node_id in &label_node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
        else {
            continue;
        };
        include_optional_bounds(&mut out, AxisBounds::from_array(bounds));
        label_items_added += 1;
    }

    let include_node_boxes =
        !component.complete || (component.bond_ids.is_empty() && label_items_added == 0);
    if include_node_boxes {
        for node_id in &component.node_ids {
            let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
                continue;
            };
            include_optional_bounds(
                &mut out,
                AxisBounds::around_point(
                    entry.world_point_for_node(node),
                    SELECTION_NODE_BOX_SIZE / 2.0,
                ),
            );
        }
    }

    for bond_id in &component.bond_ids {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == *bond_id) else {
            continue;
        };
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        include_optional_bounds(
            &mut out,
            AxisBounds::new(begin_point.x, begin_point.y, end_point.x, end_point.y)
                .expanded(SELECTION_BOND_DOT_RADIUS.max(SELECTION_BOX_STROKE_WIDTH)),
        );
    }

    out
}

pub(super) fn component_selection_items(
    document: &crate::ChemcoreDocument,
    entry: &crate::EditableFragment<'_>,
    component: &ComponentSelection,
) -> Vec<FragmentSelectionItem> {
    let mut items = Vec::new();
    let mut label_node_ids = component.label_node_ids.clone();
    if component.complete {
        for node_id in &component.node_ids {
            if !label_node_ids.iter().any(|existing| existing == node_id) {
                label_node_ids.push(node_id.clone());
            }
        }
    }
    let mut label_items_added = 0usize;
    for node_id in &label_node_ids {
        let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
            continue;
        };
        let Some(bounds) = endpoint_label_world_bounds(node, entry.object.transform.translate)
        else {
            continue;
        };
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Label,
            bounds: AxisBounds::from_array(bounds),
            center: Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5),
        });
        label_items_added += 1;
    }
    let include_node_boxes =
        !component.complete || (component.bond_ids.is_empty() && label_items_added == 0);
    if include_node_boxes {
        for node_id in &component.node_ids {
            let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == *node_id) else {
                continue;
            };
            let center = entry.world_point_for_node(node);
            items.push(FragmentSelectionItem {
                kind: FragmentItemKind::Node,
                bounds: AxisBounds::around_point(center, SELECTION_NODE_BOX_SIZE / 2.0),
                center,
            });
        }
    }
    for bond_id in &component.bond_ids {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == *bond_id) else {
            continue;
        };
        let Some(begin) = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)
        else {
            continue;
        };
        let Some(end) = entry.fragment.nodes.iter().find(|node| node.id == bond.end) else {
            continue;
        };
        let begin_point = entry.world_point_for_node(begin);
        let end_point = entry.world_point_for_node(end);
        let bounds = fragment_bond_visual_bounds(document, entry.object, entry.fragment, bond)
            .map(AxisBounds::from_array)
            .unwrap_or_else(|| {
                AxisBounds::new(begin_point.x, begin_point.y, end_point.x, end_point.y)
            });
        items.push(FragmentSelectionItem {
            kind: FragmentItemKind::Bond,
            bounds,
            center: bounds.center(),
        });
    }
    items
}
