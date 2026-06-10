use super::*;

pub(super) fn molecule_stroke(document: &ChemcoreDocument, object: &SceneObject) -> String {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_string(style, "stroke"))
        .unwrap_or_else(|| CHEMCORE_INK.to_string())
}

pub(super) fn bond_stroke_width(
    document: &ChemcoreDocument,
    object: &SceneObject,
    bond: &Bond,
) -> f64 {
    if bond.stroke_width > 0.0 {
        return bond.stroke_width;
    }
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| {
            style_number(style, "strokeWidth").or_else(|| style_number(style, "stroke_width"))
        })
        .unwrap_or(DEFAULT_BOND_STROKE)
}

pub(super) fn style_string(style: &JsonValue, key: &str) -> Option<String> {
    style.get(key)?.as_str().map(ToString::to_string)
}

pub(super) fn style_nullable_string(style: &JsonValue, key: &str) -> Option<String> {
    let value = style.get(key)?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(ToString::to_string)
}

pub(super) fn style_number(style: &JsonValue, key: &str) -> Option<f64> {
    style.get(key)?.as_f64()
}

pub(super) fn style_number_array(style: &JsonValue, key: &str) -> Option<Vec<f64>> {
    Some(
        style
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(JsonValue::as_f64)
            .collect(),
    )
}

pub(super) fn payload_string(payload: &ObjectPayload, key: &str) -> Option<String> {
    payload.extra.get(key)?.as_str().map(ToString::to_string)
}

pub(super) fn payload_number(payload: &ObjectPayload, key: &str) -> Option<f64> {
    payload.extra.get(key)?.as_f64()
}

pub(super) fn payload_bool(payload: &ObjectPayload, key: &str) -> Option<bool> {
    payload.extra.get(key)?.as_bool()
}

pub(super) fn payload_points(payload: &ObjectPayload, key: &str) -> Vec<Point> {
    payload
        .extra
        .get(key)
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            let coords = value.as_array()?;
            Some(Point::new(
                coords.first()?.as_f64()?,
                coords.get(1)?.as_f64()?,
            ))
        })
        .collect()
}

fn payload_nested_point(payload: &ObjectPayload, group: &str, key: &str) -> Option<Point> {
    let coords = payload.extra.get(group)?.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

pub(super) fn payload_box_width(payload: &ObjectPayload, key: &str) -> Option<f64> {
    let coords = payload.extra.get(key)?.as_array()?;
    coords.get(2)?.as_f64()
}

pub(super) fn payload_runs(payload: &ObjectPayload, key: &str) -> Vec<LabelRun> {
    payload
        .extra
        .get(key)
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LabelRun>>(value).ok())
        .unwrap_or_default()
}

pub(super) fn payload_arrow_head(
    payload: &ObjectPayload,
    key: &str,
    stroke_width: f64,
) -> Option<ArrowHeadGeometry> {
    let value = payload.extra.get(key)?;
    let length = value.get("length").and_then(JsonValue::as_f64)?;
    let scale = if stroke_width > EPSILON {
        stroke_width
    } else {
        DEFAULT_BOND_STROKE
    };
    Some(ArrowHeadGeometry {
        length: length * scale,
        center_length: value
            .get("centerLength")
            .or_else(|| value.get("center_length"))
            .and_then(JsonValue::as_f64)?
            * scale,
        width: value.get("width").and_then(JsonValue::as_f64)? * scale,
        shaft_spacing: value
            .get("shaftSpacing")
            .or_else(|| value.get("shaft_spacing"))
            .and_then(JsonValue::as_f64)
            .unwrap_or(3.0)
            * scale,
        equilibrium_ratio: value
            .get("equilibriumRatio")
            .or_else(|| value.get("equilibrium_ratio"))
            .and_then(JsonValue::as_f64)
            .filter(|value| value.is_finite() && *value > 1.0)
            .unwrap_or(1.0),
        kind: value
            .get("kind")
            .and_then(JsonValue::as_str)
            .map(arrow_head_kind)?,
        curve: value.get("curve").and_then(JsonValue::as_f64)?,
        bold: value.get("bold").and_then(JsonValue::as_bool)?,
        no_go: value
            .get("noGo")
            .or_else(|| value.get("no_go"))
            .and_then(JsonValue::as_str)
            .map(arrow_no_go_geometry)?,
    })
}

pub(super) fn payload_arrow_arc_geometry(
    payload: &ObjectPayload,
    key: &str,
) -> Option<ArrowArcGeometry> {
    Some(ArrowArcGeometry {
        center: payload_nested_point(payload, key, "center")?,
        major_axis_end: payload_nested_point(payload, key, "majorAxisEnd")?,
        minor_axis_end: payload_nested_point(payload, key, "minorAxisEnd")?,
    })
}

pub(super) fn arrow_head_kind(value: &str) -> ArrowHeadKind {
    match value.to_ascii_lowercase().as_str() {
        "hollow" => ArrowHeadKind::Hollow,
        "angle" | "open" | "retrosynthetic" => ArrowHeadKind::Open,
        "equilibrium" | "unequal-equilibrium" => ArrowHeadKind::Equilibrium,
        _ => ArrowHeadKind::Solid,
    }
}

pub(super) fn arrow_no_go_geometry(value: &str) -> ArrowNoGoGeometry {
    match value.to_ascii_lowercase().as_str() {
        "cross" => ArrowNoGoGeometry::Cross,
        "hash" => ArrowNoGoGeometry::Hash,
        _ => ArrowNoGoGeometry::None,
    }
}
