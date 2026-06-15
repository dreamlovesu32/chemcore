use super::*;

pub(super) fn selected_text_object_ids(engine: &Engine) -> BTreeSet<String> {
    let overlay = group_selection_overlay(engine);
    let mut ids: BTreeSet<String> = engine
        .state
        .selection
        .text_objects
        .iter()
        .cloned()
        .collect();
    ids.extend(engine.state.selection.arrow_objects.iter().cloned());
    ids.retain(|object_id| !overlay.selected_group_hides_object(object_id));
    ids
}

pub(super) fn selected_movable_node_ids(engine: &Engine) -> Vec<String> {
    let overlay = group_selection_overlay(engine);
    if engine
        .state
        .document
        .editable_fragment()
        .is_some_and(|entry| overlay.selected_group_hides_object(&entry.object.id))
    {
        return Vec::new();
    }
    let mut node_ids: BTreeSet<String> = engine.state.selection.nodes.iter().cloned().collect();
    node_ids.extend(engine.state.selection.label_nodes.iter().cloned());
    let Some(entry) = engine.state.document.editable_fragment() else {
        return node_ids.into_iter().collect();
    };
    for bond_id in &engine.state.selection.bonds {
        let Some(bond) = entry.fragment.bonds.iter().find(|bond| &bond.id == bond_id) else {
            continue;
        };
        node_ids.insert(bond.begin.clone());
        node_ids.insert(bond.end.clone());
    }
    node_ids.into_iter().collect()
}

pub(super) fn object_arrow_curve(object: &crate::SceneObject) -> f64 {
    object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|value| value.get("curve"))
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.0)
}

pub(super) fn snapped_arrow_endpoint(pivot: Point, point: Point, alt_key: bool) -> Point {
    let length = pivot.distance(point);
    if length <= crate::EPSILON {
        return pivot;
    }
    let angle = if alt_key {
        angle_between(pivot, point)
    } else {
        nearest_angle(angle_between(pivot, point), GLOBAL_SNAP_ANGLES)
    };
    pivot.translated(direction_from_angle(angle).scaled(length))
}

pub(super) fn snapped_arrow_curve_from_point(
    start: Point,
    end: Point,
    point: Point,
    alt_key: bool,
) -> f64 {
    let chord = Point::new(end.x - start.x, end.y - start.y);
    let chord_length = start.distance(end);
    if chord_length <= crate::EPSILON {
        return 0.0;
    }
    let mid = Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
    let ux = chord.x / chord_length;
    let uy = chord.y / chord_length;
    let normal_x = -uy;
    let normal_y = ux;
    let sagitta = (point.x - mid.x) * normal_x + (point.y - mid.y) * normal_y;
    let mut degrees = (4.0 * (2.0 * sagitta / chord_length).atan()).to_degrees();
    degrees = degrees.clamp(-270.0, 270.0);
    if !alt_key {
        degrees = (degrees / 15.0).round() * 15.0;
    }
    if degrees.abs() < 0.5 {
        0.0
    } else {
        degrees
    }
}

fn refresh_arrow_arc_geometry(object: &mut crate::SceneObject) {
    let curve = object_arrow_curve(object);
    let Some((start, end)) = crate::arrow_payload_line_endpoints(&object.payload.extra) else {
        object.payload.extra.remove("arrowGeometry");
        return;
    };
    if let Some(geometry) = crate::default_arrow_arc_geometry_payload(start, end, curve) {
        object
            .payload
            .extra
            .insert("arrowGeometry".to_string(), geometry);
    } else {
        object.payload.extra.remove("arrowGeometry");
    }
}

pub(super) fn update_arrow_object_points(
    engine: &mut Engine,
    object_id: &str,
    start: Point,
    end: Point,
) -> bool {
    let Some(object) = engine
        .state
        .document
        .objects
        .iter_mut()
        .find(|object| object.id == object_id && object.object_type == "line")
    else {
        return false;
    };
    let tx = object.transform.translate[0];
    let ty = object.transform.translate[1];
    let next_points = json!([[start.x - tx, start.y - ty], [end.x - tx, end.y - ty]]);
    if object.payload.extra.get("points") == Some(&next_points) {
        return false;
    }
    object
        .payload
        .extra
        .insert("points".to_string(), next_points);
    refresh_arrow_arc_geometry(object);
    true
}

pub(super) fn update_arrow_object_curve(engine: &mut Engine, object_id: &str, curve: f64) -> bool {
    let Some(object) = engine
        .state
        .document
        .objects
        .iter_mut()
        .find(|object| object.id == object_id && object.object_type == "line")
    else {
        return false;
    };
    let mut arrow_head = object
        .payload
        .extra
        .get("arrowHead")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let Some(arrow_head_object) = arrow_head.as_object_mut() else {
        return false;
    };
    let rounded_curve = (curve * 1000.0).round() / 1000.0;
    arrow_head_object.insert("curve".to_string(), json!(rounded_curve));
    let kind = arrow_head_object
        .get("kind")
        .and_then(JsonValue::as_str)
        .unwrap_or("solid")
        .to_ascii_lowercase();
    if kind == "open" {
        arrow_head_object.insert("curve".to_string(), json!(0.0));
    } else if kind != "hollow" && kind != "equilibrium" && kind != "unequal-equilibrium" {
        let next_kind = if rounded_curve < -crate::EPSILON {
            "curved"
        } else if rounded_curve > crate::EPSILON {
            "curved-mirror"
        } else {
            "solid"
        };
        arrow_head_object.insert("kind".to_string(), json!(next_kind));
    }
    if object.payload.extra.get("arrowHead") == Some(&arrow_head) {
        return false;
    }
    object
        .payload
        .extra
        .insert("arrowHead".to_string(), arrow_head);
    refresh_arrow_arc_geometry(object);
    true
}
