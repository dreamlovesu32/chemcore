use super::*;

pub(super) fn terminal_node_drag_axis(
    entry: crate::EditableFragment<'_>,
    node_id: &str,
) -> Option<(Point, f64)> {
    let incident: Vec<_> = entry
        .fragment
        .bonds
        .iter()
        .filter(|bond| bond.begin == node_id || bond.end == node_id)
        .collect();
    if incident.len() != 1 {
        return None;
    }
    let bond = incident[0];
    let other_id = if bond.begin == node_id {
        &bond.end
    } else {
        &bond.begin
    };
    let node = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)?;
    let other = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == *other_id)?;
    let node_point = entry.world_point_for_node(node);
    let pivot = entry.world_point_for_node(other);
    let length = if pivot.distance(node_point) <= crate::EPSILON {
        DEFAULT_BOND_LENGTH
    } else {
        pivot.distance(node_point)
    };
    Some((pivot, length))
}

pub(super) fn selection_drag_changes_document(
    drag: &SelectionMoveDrag,
    point: Point,
    alt_key: bool,
) -> bool {
    match &drag.mode {
        SelectionMoveMode::TerminalNode { pivot, length, .. } => {
            if point.distance(drag.start) <= crate::EPSILON {
                return false;
            }
            let target = terminal_drag_target(*pivot, *length, point, alt_key);
            drag.node_originals.iter().any(|node| {
                (node.position[0] - target.x).abs() > 1.0e-9
                    || (node.position[1] - target.y).abs() > 1.0e-9
            })
        }
        SelectionMoveMode::Translate => point.distance(drag.start) > crate::EPSILON,
    }
}

pub(super) fn apply_selection_drag_to_document(
    engine: &mut Engine,
    drag: &SelectionMoveDrag,
    point: Point,
    alt_key: bool,
) {
    let delta_x = point.x - drag.start.x;
    let delta_y = point.y - drag.start.y;

    for original in &drag.text_originals {
        let Some(object) = engine
            .state
            .document
            .find_scene_object_mut(&original.object_id)
        else {
            continue;
        };
        if matches!(drag.mode, SelectionMoveMode::Translate) {
            *object = translated_scene_object(&original.object, delta_x, delta_y);
        }
    }

    let stroke_width = engine.options.bond_stroke_world_pt().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    match &drag.mode {
        SelectionMoveMode::Translate => {
            for original in &drag.node_originals {
                if let Some(node) = entry
                    .fragment
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == original.node_id)
                {
                    node.position = [
                        round2(original.position[0] + delta_x),
                        round2(original.position[1] + delta_y),
                    ];
                    node.label = original
                        .label
                        .as_ref()
                        .map(|label| translated_node_label_geometry(label, delta_x, delta_y));
                }
            }
        }
        SelectionMoveMode::TerminalNode {
            node_id,
            pivot,
            length,
        } => {
            let target = terminal_drag_target(*pivot, *length, point, alt_key);
            if let Some(node) = entry
                .fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
            {
                node.position = [
                    round2(target.x - object_translate[0]),
                    round2(target.y - object_translate[1]),
                ];
            }
        }
    }
    match drag.mode {
        SelectionMoveMode::Translate => {
            if let Some(bbox) = drag.fragment_bbox_original {
                let translated = translate_size_bbox(bbox, delta_x, delta_y);
                entry.fragment.bbox = translated;
                entry.object.payload.bbox = Some(translated);
            } else {
                entry.update_bounds();
            }
        }
        SelectionMoveMode::TerminalNode { .. } => {
            refresh_attached_node_label_geometry_for_all_nodes(
                entry.fragment,
                object_translate,
                stroke_width,
            );
            entry.update_bounds();
        }
    }
}

pub(super) fn selection_rotate_delta_degrees(
    drag: &SelectionRotateDrag,
    point: Point,
    alt_key: bool,
) -> f64 {
    let raw = signed_angle_delta(drag.start_angle, angle_between(drag.center, point));
    if alt_key {
        return raw;
    }
    (raw / 15.0).round() * 15.0
}

pub(super) fn signed_angle_delta(start: f64, end: f64) -> f64 {
    let mut delta = (end - start) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta <= -180.0 {
        delta += 360.0;
    }
    delta
}

pub(super) fn rotate_point_around(point: Point, center: Point, degrees: f64) -> Point {
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

pub(super) fn apply_selection_rotation_to_document(
    engine: &mut Engine,
    drag: &SelectionRotateDrag,
    angle: f64,
) {
    for original in &drag.text_originals {
        let Some(object) = engine
            .state
            .document
            .find_scene_object_mut(&original.object_id)
        else {
            continue;
        };
        *object = rotated_scene_object(&original.object, drag.center, angle);
    }

    let stroke_width = engine.options.bond_stroke_world_pt().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    for original in &drag.node_originals {
        let original_world = Point::new(
            object_translate[0] + original.position[0],
            object_translate[1] + original.position[1],
        );
        let next = rotate_point_around(original_world, drag.center, angle);
        if let Some(node) = entry
            .fragment
            .nodes
            .iter_mut()
            .find(|node| node.id == original.node_id)
        {
            node.position = [
                round2(next.x - object_translate[0]),
                round2(next.y - object_translate[1]),
            ];
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
}

fn rotated_scene_object(original: &SceneObject, center: Point, degrees: f64) -> SceneObject {
    let mut object = original.clone();
    if original.object_type == "group" {
        object.children = original
            .children
            .iter()
            .map(|child| rotated_scene_object(child, center, degrees))
            .collect();
        return object;
    }

    let original_translate = Point::new(
        original.transform.translate[0],
        original.transform.translate[1],
    );

    match original.object_type.as_str() {
        "line" => rotate_payload_points_to_next_local(
            &mut object,
            original_translate,
            original_translate,
            center,
            degrees,
        ),
        "shape" if shape_uses_absolute_points(original) => rotate_payload_points_to_next_local(
            &mut object,
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            center,
            degrees,
        ),
        "shape" | "bracket" | "symbol" => {
            rotate_bbox_based_object(&mut object, original, center, degrees);
        }
        "text" => {
            let next_translate = rotate_point_around(original_translate, center, degrees);
            object.transform.translate = [round2(next_translate.x), round2(next_translate.y)];
            object.transform.rotate = round2(object.transform.rotate + degrees);
        }
        _ => {
            let next_translate = rotate_point_around(original_translate, center, degrees);
            object.transform.translate = [round2(next_translate.x), round2(next_translate.y)];
        }
    }
    object
}

fn rotate_bbox_based_object(
    object: &mut SceneObject,
    original: &SceneObject,
    center: Point,
    degrees: f64,
) {
    let original_translate = Point::new(
        original.transform.translate[0],
        original.transform.translate[1],
    );
    if let Some([x, y, width, height]) = original.payload.bbox {
        let original_center = Point::new(
            original_translate.x + x + width * 0.5,
            original_translate.y + y + height * 0.5,
        );
        let next_center = rotate_point_around(original_center, center, degrees);
        object.transform.translate = [
            round2(original_translate.x + next_center.x - original_center.x),
            round2(original_translate.y + next_center.y - original_center.y),
        ];
    } else {
        let next_translate = rotate_point_around(original_translate, center, degrees);
        object.transform.translate = [round2(next_translate.x), round2(next_translate.y)];
    }
    object.transform.rotate = round2(object.transform.rotate + degrees);
}

fn rotate_payload_points_to_next_local(
    object: &mut SceneObject,
    original_translate: Point,
    next_translate: Point,
    center: Point,
    degrees: f64,
) {
    for key in ["center", "majorAxisEnd", "minorAxisEnd"] {
        rotate_extra_point(
            &mut object.payload.extra,
            key,
            original_translate,
            next_translate,
            center,
            degrees,
        );
    }
    rotate_extra_point_array(
        &mut object.payload.extra,
        "points",
        original_translate,
        next_translate,
        center,
        degrees,
    );
    if let Some(geometry) = object
        .payload
        .extra
        .get_mut("arrowGeometry")
        .and_then(JsonValue::as_object_mut)
    {
        for key in ["center", "majorAxisEnd", "minorAxisEnd"] {
            rotate_json_object_point(
                geometry,
                key,
                original_translate,
                next_translate,
                center,
                degrees,
            );
        }
    }
}

fn rotate_extra_point(
    extra: &mut BTreeMap<String, JsonValue>,
    key: &str,
    original_translate: Point,
    next_translate: Point,
    center: Point,
    degrees: f64,
) {
    let Some(point) = extra.get(key).and_then(json_array_to_point) else {
        return;
    };
    let next = rotate_local_point_to_next_local(
        point,
        original_translate,
        next_translate,
        center,
        degrees,
    );
    extra.insert(key.to_string(), json!([round2(next.x), round2(next.y)]));
}

fn rotate_json_object_point(
    object: &mut serde_json::Map<String, JsonValue>,
    key: &str,
    original_translate: Point,
    next_translate: Point,
    center: Point,
    degrees: f64,
) {
    let Some(point) = object.get(key).and_then(json_array_to_point) else {
        return;
    };
    let next = rotate_local_point_to_next_local(
        point,
        original_translate,
        next_translate,
        center,
        degrees,
    );
    object.insert(key.to_string(), json!([round2(next.x), round2(next.y)]));
}

fn rotate_extra_point_array(
    extra: &mut BTreeMap<String, JsonValue>,
    key: &str,
    original_translate: Point,
    next_translate: Point,
    center: Point,
    degrees: f64,
) {
    let Some(points) = extra.get_mut(key).and_then(JsonValue::as_array_mut) else {
        return;
    };
    for value in points {
        let Some(point) = json_array_to_point(value) else {
            continue;
        };
        let next = rotate_local_point_to_next_local(
            point,
            original_translate,
            next_translate,
            center,
            degrees,
        );
        *value = json!([round2(next.x), round2(next.y)]);
    }
}

fn rotate_local_point_to_next_local(
    point: Point,
    original_translate: Point,
    next_translate: Point,
    center: Point,
    degrees: f64,
) -> Point {
    let world = Point::new(
        original_translate.x + point.x,
        original_translate.y + point.y,
    );
    let rotated = rotate_point_around(world, center, degrees);
    Point::new(rotated.x - next_translate.x, rotated.y - next_translate.y)
}

pub(super) fn terminal_drag_target(
    pivot: Point,
    length: f64,
    point: Point,
    alt_key: bool,
) -> Point {
    if alt_key {
        return point;
    }
    let angle = nearest_angle(angle_between(pivot, point), GLOBAL_SNAP_ANGLES);
    pivot.translated(direction_from_angle(angle).scaled(length))
}

const SELECTION_RESIZE_MIN_SCALE: f64 = 0.05;

pub(super) fn selection_resize_scale(drag: &SelectionResizeDrag, point: Point) -> (f64, f64) {
    let bounds = drag.bounds;
    if drag.handle.is_corner() {
        let pivot = selection_resize_pivot(drag.handle, bounds);
        let handle = selection_resize_handle_point(drag.handle, bounds);
        let original = Point::new(handle.x - pivot.x, handle.y - pivot.y);
        let current = Point::new(point.x - pivot.x, point.y - pivot.y);
        let denominator = original.x * original.x + original.y * original.y;
        if denominator <= crate::EPSILON {
            return (1.0, 1.0);
        }
        let scale = ((current.x * original.x + current.y * original.y) / denominator)
            .max(SELECTION_RESIZE_MIN_SCALE);
        return (scale, scale);
    }

    match drag.handle {
        SelectionResizeHandle::East => (
            ((point.x - bounds.min_x) / bounds.width()).max(SELECTION_RESIZE_MIN_SCALE),
            1.0,
        ),
        SelectionResizeHandle::West => (
            ((bounds.max_x - point.x) / bounds.width()).max(SELECTION_RESIZE_MIN_SCALE),
            1.0,
        ),
        SelectionResizeHandle::South => (
            1.0,
            ((point.y - bounds.min_y) / bounds.height()).max(SELECTION_RESIZE_MIN_SCALE),
        ),
        SelectionResizeHandle::North => (
            1.0,
            ((bounds.max_y - point.y) / bounds.height()).max(SELECTION_RESIZE_MIN_SCALE),
        ),
        SelectionResizeHandle::NorthEast
        | SelectionResizeHandle::NorthWest
        | SelectionResizeHandle::SouthEast
        | SelectionResizeHandle::SouthWest => unreachable!("corner handles returned above"),
    }
}

pub(super) fn apply_selection_resize_to_document(
    engine: &mut Engine,
    drag: &SelectionResizeDrag,
    scale_x: f64,
    scale_y: f64,
) {
    let pivot = selection_resize_pivot(drag.handle, drag.bounds);
    apply_selection_scale_to_document(
        engine,
        &drag.node_originals,
        &drag.object_originals,
        pivot,
        scale_x,
        scale_y,
    );
}

pub(super) fn apply_selection_scale_to_document(
    engine: &mut Engine,
    node_originals: &[NodeMoveOriginal],
    object_originals: &[ObjectResizeOriginal],
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    for original in object_originals {
        let Some(object) = engine
            .state
            .document
            .find_scene_object_mut(&original.object.id)
        else {
            continue;
        };
        *object = resized_scene_object(&original.object, pivot, scale_x, scale_y);
    }

    let stroke_width = engine.options.bond_stroke_world_pt().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    for original in node_originals {
        let original_world = Point::new(
            object_translate[0] + original.position[0],
            object_translate[1] + original.position[1],
        );
        let next = scale_point_from_pivot(original_world, pivot, scale_x, scale_y);
        if let Some(node) = entry
            .fragment
            .nodes
            .iter_mut()
            .find(|node| node.id == original.node_id)
        {
            node.position = [
                round2(next.x - object_translate[0]),
                round2(next.y - object_translate[1]),
            ];
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
    drop(entry);
    engine.refresh_symbol_chemistry();
}

fn selection_resize_pivot(handle: SelectionResizeHandle, bounds: AxisBounds) -> Point {
    match handle {
        SelectionResizeHandle::North => Point::new(bounds.center_x(), bounds.max_y),
        SelectionResizeHandle::South => Point::new(bounds.center_x(), bounds.min_y),
        SelectionResizeHandle::East => Point::new(bounds.min_x, bounds.center_y()),
        SelectionResizeHandle::West => Point::new(bounds.max_x, bounds.center_y()),
        SelectionResizeHandle::NorthEast => Point::new(bounds.min_x, bounds.max_y),
        SelectionResizeHandle::NorthWest => Point::new(bounds.max_x, bounds.max_y),
        SelectionResizeHandle::SouthEast => Point::new(bounds.min_x, bounds.min_y),
        SelectionResizeHandle::SouthWest => Point::new(bounds.max_x, bounds.min_y),
    }
}

fn selection_resize_handle_point(handle: SelectionResizeHandle, bounds: AxisBounds) -> Point {
    match handle {
        SelectionResizeHandle::North => Point::new(bounds.center_x(), bounds.min_y),
        SelectionResizeHandle::South => Point::new(bounds.center_x(), bounds.max_y),
        SelectionResizeHandle::East => Point::new(bounds.max_x, bounds.center_y()),
        SelectionResizeHandle::West => Point::new(bounds.min_x, bounds.center_y()),
        SelectionResizeHandle::NorthEast => Point::new(bounds.max_x, bounds.min_y),
        SelectionResizeHandle::NorthWest => Point::new(bounds.min_x, bounds.min_y),
        SelectionResizeHandle::SouthEast => Point::new(bounds.max_x, bounds.max_y),
        SelectionResizeHandle::SouthWest => Point::new(bounds.min_x, bounds.max_y),
    }
}

fn scale_point_from_pivot(point: Point, pivot: Point, scale_x: f64, scale_y: f64) -> Point {
    Point::new(
        pivot.x + (point.x - pivot.x) * scale_x,
        pivot.y + (point.y - pivot.y) * scale_y,
    )
}

fn resized_scene_object(
    original: &SceneObject,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) -> SceneObject {
    let mut object = original.clone();
    let scale_transform = object_transform_participates_in_render(original);
    let original_translate = Point::new(
        original.transform.translate[0],
        original.transform.translate[1],
    );
    let next_translate = if scale_transform {
        scale_point_from_pivot(original_translate, pivot, scale_x, scale_y)
    } else {
        original_translate
    };
    object.transform.translate = [round2(next_translate.x), round2(next_translate.y)];

    resize_payload_bbox(
        &mut object,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    resize_payload_box(
        &mut object,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    resize_payload_points(
        &mut object,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    resize_payload_named_points(
        &mut object,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    resize_text_dimensions(&mut object, scale_x, scale_y);
    resize_graphic_dimensions(&mut object, scale_x, scale_y);
    if original.object_type == "group" {
        object.children = original
            .children
            .iter()
            .map(|child| resized_scene_object(child, pivot, scale_x, scale_y))
            .collect();
    }
    object
}

fn translated_scene_object(original: &SceneObject, delta_x: f64, delta_y: f64) -> SceneObject {
    let mut object = original.clone();
    if original.object_type == "line" {
        translate_line_payload_points(&mut object, delta_x, delta_y);
        return object;
    }
    object.transform.translate = [
        round2(original.transform.translate[0] + delta_x),
        round2(original.transform.translate[1] + delta_y),
    ];
    if shape_uses_absolute_points(&object) {
        translate_absolute_shape_points(&mut object, delta_x, delta_y);
    }
    if original.object_type == "group" {
        object.children = original
            .children
            .iter()
            .map(|child| translated_scene_object(child, delta_x, delta_y))
            .collect();
    }
    object
}

fn translate_line_payload_points(object: &mut SceneObject, delta_x: f64, delta_y: f64) {
    translate_extra_point_array(&mut object.payload.extra, "points", delta_x, delta_y);
    if let Some(geometry) = object
        .payload
        .extra
        .get_mut("arrowGeometry")
        .and_then(JsonValue::as_object_mut)
    {
        for key in ["center", "majorAxisEnd", "minorAxisEnd"] {
            translate_json_object_point(geometry, key, delta_x, delta_y);
        }
        translate_json_object_bbox(geometry, "boundingBox", delta_x, delta_y);
    }
    if let Some([x, y, width, height]) = object.payload.bbox {
        object.payload.bbox = Some([round2(x + delta_x), round2(y + delta_y), width, height]);
    }
}

fn translate_extra_point_array(
    extra: &mut BTreeMap<String, JsonValue>,
    key: &str,
    delta_x: f64,
    delta_y: f64,
) {
    let Some(points) = extra.get_mut(key).and_then(JsonValue::as_array_mut) else {
        return;
    };
    for value in points {
        let Some(point) = json_array_to_point(value) else {
            continue;
        };
        *value = json!([round2(point.x + delta_x), round2(point.y + delta_y)]);
    }
}

fn translate_json_object_point(
    object: &mut serde_json::Map<String, JsonValue>,
    key: &str,
    delta_x: f64,
    delta_y: f64,
) {
    let Some(point) = object.get(key).and_then(json_array_to_point) else {
        return;
    };
    object.insert(
        key.to_string(),
        json!([round2(point.x + delta_x), round2(point.y + delta_y)]),
    );
}

fn translate_json_object_bbox(
    object: &mut serde_json::Map<String, JsonValue>,
    key: &str,
    delta_x: f64,
    delta_y: f64,
) {
    let Some(bbox) = json_array_to_box(object.get(key)) else {
        return;
    };
    object.insert(
        key.to_string(),
        json!([
            round2(bbox[0] + delta_x),
            round2(bbox[1] + delta_y),
            round2(bbox[2] + delta_x),
            round2(bbox[3] + delta_y)
        ]),
    );
}

fn translate_absolute_shape_points(object: &mut SceneObject, delta_x: f64, delta_y: f64) {
    for key in ["center", "majorAxisEnd", "minorAxisEnd"] {
        let Some(point) = object.payload.extra.get(key).and_then(json_array_to_point) else {
            continue;
        };
        object.payload.extra.insert(
            key.to_string(),
            json!([round2(point.x + delta_x), round2(point.y + delta_y)]),
        );
    }
    if let Some([x, y, width, height]) = object.payload.bbox {
        object.payload.bbox = Some([round2(x + delta_x), round2(y + delta_y), width, height]);
    }
}

fn translated_node_label_geometry(
    original: &crate::NodeLabel,
    delta_x: f64,
    delta_y: f64,
) -> crate::NodeLabel {
    let mut label = original.clone();
    translate_node_label_geometry(&mut label, delta_x, delta_y);
    label
}

fn translate_node_label_geometry(label: &mut crate::NodeLabel, delta_x: f64, delta_y: f64) {
    if delta_x.abs() <= crate::EPSILON && delta_y.abs() <= crate::EPSILON {
        return;
    }
    if let Some(position) = &mut label.position {
        position[0] = round2(position[0] + delta_x);
        position[1] = round2(position[1] + delta_y);
    }
    if let Some(bounds) = &mut label.box_field {
        translate_box(bounds, delta_x, delta_y);
    }
    if let Some(bounds) = &mut label.box_value {
        translate_box(bounds, delta_x, delta_y);
    }
    for polygon in &mut label.glyph_polygons {
        for point in polygon {
            point[0] = round2(point[0] + delta_x);
            point[1] = round2(point[1] + delta_y);
        }
    }
}

fn translate_box(bounds: &mut [f64; 4], delta_x: f64, delta_y: f64) {
    bounds[0] = round2(bounds[0] + delta_x);
    bounds[1] = round2(bounds[1] + delta_y);
    bounds[2] = round2(bounds[2] + delta_x);
    bounds[3] = round2(bounds[3] + delta_y);
}

fn translate_size_bbox(bounds: [f64; 4], delta_x: f64, delta_y: f64) -> [f64; 4] {
    [
        round2(bounds[0] + delta_x),
        round2(bounds[1] + delta_y),
        bounds[2],
        bounds[3],
    ]
}

fn object_transform_participates_in_render(object: &SceneObject) -> bool {
    match object.object_type.as_str() {
        "text" | "bracket" | "symbol" => true,
        "shape" => !shape_uses_absolute_points(object),
        _ => false,
    }
}

fn shape_uses_absolute_points(object: &SceneObject) -> bool {
    object.object_type == "shape"
        && matches!(
            object
                .payload
                .extra
                .get("kind")
                .and_then(JsonValue::as_str)
                .unwrap_or("rect"),
            "circle" | "ellipse"
        )
}

fn resize_payload_bbox(
    object: &mut SceneObject,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    let Some(bbox) = object.payload.bbox else {
        return;
    };
    object.payload.bbox = Some(scale_local_box(
        bbox,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    ));
}

fn resize_payload_box(
    object: &mut SceneObject,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    let Some(box_value) = json_array_to_box(object.payload.extra.get("box")) else {
        return;
    };
    object.payload.extra.insert(
        "box".to_string(),
        json!(scale_local_box(
            box_value,
            original_translate,
            next_translate,
            pivot,
            scale_x,
            scale_y,
        )),
    );
}

fn scale_local_box(
    bbox: [f64; 4],
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) -> [f64; 4] {
    let min = scale_local_point_to_next_local(
        Point::new(bbox[0], bbox[1]),
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    let max = scale_local_point_to_next_local(
        Point::new(bbox[0] + bbox[2], bbox[1] + bbox[3]),
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    [
        round2(min.x.min(max.x)),
        round2(min.y.min(max.y)),
        round2((max.x - min.x).abs()),
        round2((max.y - min.y).abs()),
    ]
}

fn resize_payload_points(
    object: &mut SceneObject,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    let keys = ["points"];
    for key in keys {
        let Some(points) = object.payload.extra.get(key).and_then(JsonValue::as_array) else {
            continue;
        };
        let next_points = points
            .iter()
            .filter_map(|value| {
                let point = json_array_to_point(value)?;
                let next = scale_local_point_to_next_local(
                    point,
                    original_translate,
                    next_translate,
                    pivot,
                    scale_x,
                    scale_y,
                );
                Some(json!([round2(next.x), round2(next.y)]))
            })
            .collect::<Vec<_>>();
        object
            .payload
            .extra
            .insert(key.to_string(), JsonValue::Array(next_points));
    }
}

fn resize_payload_named_points(
    object: &mut SceneObject,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    for key in ["center", "majorAxisEnd", "minorAxisEnd"] {
        resize_extra_point(
            &mut object.payload.extra,
            key,
            original_translate,
            next_translate,
            pivot,
            scale_x,
            scale_y,
        );
    }
    let Some(geometry) = object
        .payload
        .extra
        .get_mut("arrowGeometry")
        .and_then(JsonValue::as_object_mut)
    else {
        return;
    };
    for key in ["center", "majorAxisEnd", "minorAxisEnd"] {
        resize_object_point(
            geometry,
            key,
            original_translate,
            next_translate,
            pivot,
            scale_x,
            scale_y,
        );
    }
}

fn resize_extra_point(
    extra: &mut BTreeMap<String, JsonValue>,
    key: &str,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    let Some(point) = extra.get(key).and_then(json_array_to_point) else {
        return;
    };
    let next = scale_local_point_to_next_local(
        point,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    extra.insert(key.to_string(), json!([round2(next.x), round2(next.y)]));
}

fn resize_object_point(
    object: &mut serde_json::Map<String, JsonValue>,
    key: &str,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) {
    let Some(point) = object.get(key).and_then(json_array_to_point) else {
        return;
    };
    let next = scale_local_point_to_next_local(
        point,
        original_translate,
        next_translate,
        pivot,
        scale_x,
        scale_y,
    );
    object.insert(key.to_string(), json!([round2(next.x), round2(next.y)]));
}

fn scale_local_point_to_next_local(
    point: Point,
    original_translate: Point,
    next_translate: Point,
    pivot: Point,
    scale_x: f64,
    scale_y: f64,
) -> Point {
    let world = Point::new(
        original_translate.x + point.x,
        original_translate.y + point.y,
    );
    let scaled = scale_point_from_pivot(world, pivot, scale_x, scale_y);
    Point::new(scaled.x - next_translate.x, scaled.y - next_translate.y)
}

fn resize_text_dimensions(object: &mut SceneObject, scale_x: f64, scale_y: f64) {
    if object.object_type != "text" {
        return;
    }
    let text_scale = if (scale_x - 1.0).abs() <= crate::EPSILON {
        scale_y
    } else if (scale_y - 1.0).abs() <= crate::EPSILON {
        1.0
    } else {
        (scale_x.abs() + scale_y.abs()) * 0.5
    };
    if (text_scale - 1.0).abs() <= crate::EPSILON {
        return;
    }
    scale_extra_number(&mut object.payload.extra, "fontSize", text_scale);
    scale_extra_number(&mut object.payload.extra, "lineHeight", text_scale);
    for key in ["runs", "sourceRuns", "displayRuns"] {
        scale_run_font_sizes(&mut object.payload.extra, key, text_scale);
    }
}

fn resize_graphic_dimensions(object: &mut SceneObject, scale_x: f64, scale_y: f64) {
    let dimension_scale = if (scale_x - 1.0).abs() <= crate::EPSILON {
        scale_y
    } else if (scale_y - 1.0).abs() <= crate::EPSILON {
        scale_x
    } else {
        (scale_x.abs() + scale_y.abs()) * 0.5
    };
    if (dimension_scale - 1.0).abs() <= crate::EPSILON {
        return;
    }
    scale_extra_number(&mut object.payload.extra, "cornerRadius", dimension_scale);
    if matches!(object.object_type.as_str(), "bracket" | "symbol") {
        scale_extra_number(&mut object.payload.extra, "strokeWidth", dimension_scale);
    }
    let Some(arrow_head) = object
        .payload
        .extra
        .get_mut("arrowHead")
        .and_then(JsonValue::as_object_mut)
    else {
        return;
    };
    for key in ["length", "centerLength", "width"] {
        if let Some(value) = arrow_head.get(key).and_then(JsonValue::as_f64) {
            arrow_head.insert(key.to_string(), json!(round2(value * dimension_scale)));
        }
    }
}

fn scale_extra_number(extra: &mut BTreeMap<String, JsonValue>, key: &str, scale: f64) {
    let Some(value) = extra.get(key).and_then(JsonValue::as_f64) else {
        return;
    };
    extra.insert(key.to_string(), json!(round2(value * scale)));
}

fn scale_run_font_sizes(extra: &mut BTreeMap<String, JsonValue>, key: &str, scale: f64) {
    let Some(runs) = extra.get_mut(key).and_then(JsonValue::as_array_mut) else {
        return;
    };
    for run in runs {
        let Some(object) = run.as_object_mut() else {
            continue;
        };
        let Some(value) = object.get("fontSize").and_then(JsonValue::as_f64) else {
            continue;
        };
        object.insert("fontSize".to_string(), json!(round2(value * scale)));
    }
}

fn json_array_to_box(value: Option<&JsonValue>) -> Option<[f64; 4]> {
    let coords = value?.as_array()?;
    Some([
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
        coords.get(2)?.as_f64()?,
        coords.get(3)?.as_f64()?,
    ])
}

fn json_array_to_point(value: &JsonValue) -> Option<Point> {
    let coords = value.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}
