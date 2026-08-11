use super::*;

pub(super) fn object_style<'a>(
    document: &'a ChemSemaDocument,
    object: &SceneObject,
) -> Option<&'a Value> {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
}

pub(super) fn style_string_value(style: &Value, key: &str) -> Option<String> {
    style.get(key)?.as_str().map(ToString::to_string)
}

pub(super) fn style_nullable_string_value(style: &Value, key: &str) -> Option<String> {
    let value = style.get(key)?;
    if value.is_null() {
        return None;
    }
    value.as_str().map(ToString::to_string)
}

pub(super) fn style_number_value(style: &Value, key: &str) -> Option<f64> {
    style.get(key)?.as_f64()
}

pub(super) fn style_number_array(style: &Value, key: &str) -> Option<Vec<f64>> {
    Some(
        style
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(Value::as_f64)
            .collect(),
    )
}

pub(super) fn payload_string_cdxml(payload: &ObjectPayload, key: &str) -> Option<String> {
    payload.extra.get(key)?.as_str().map(ToString::to_string)
}

pub(super) fn payload_point_cdxml(payload: &ObjectPayload, key: &str) -> Option<Point> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

pub(super) fn payload_points_cdxml(payload: &ObjectPayload, key: &str) -> Vec<Point> {
    payload
        .extra
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|point| {
            let coords = point.as_array()?;
            Some(Point::new(
                coords.first()?.as_f64()?,
                coords.get(1)?.as_f64()?,
            ))
        })
        .collect()
}

pub(super) fn payload_nested_point_cdxml(
    payload: &ObjectPayload,
    group: &str,
    key: &str,
) -> Option<Point> {
    let coords = payload.extra.get(group)?.get(key)?.as_array()?;
    Some(Point::new(
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
    ))
}

pub(super) fn payload_bbox_cdxml(payload: &ObjectPayload, key: &str) -> Option<[f64; 4]> {
    let coords = payload.extra.get(key)?.as_array()?;
    Some([
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
        coords.get(2)?.as_f64()?,
        coords.get(3)?.as_f64()?,
    ])
}

pub(super) fn payload_nested_bbox_cdxml(
    payload: &ObjectPayload,
    group: &str,
    key: &str,
) -> Option<[f64; 4]> {
    let coords = payload.extra.get(group)?.get(key)?.as_array()?;
    Some([
        coords.first()?.as_f64()?,
        coords.get(1)?.as_f64()?,
        coords.get(2)?.as_f64()?,
        coords.get(3)?.as_f64()?,
    ])
}

pub(super) fn object_local_point(object: &SceneObject, point: [f64; 2]) -> Point {
    Point::new(
        object.transform.translate[0] + point[0],
        object.transform.translate[1] + point[1],
    )
}

pub(super) fn molecule_world_bbox(
    object: &SceneObject,
    fragment: &MoleculeFragment,
) -> Option<[f64; 4]> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;
    if let Some([x, y, width, height]) = object.payload.bbox {
        min_x = min_x.min(object.transform.translate[0] + x);
        min_y = min_y.min(object.transform.translate[1] + y);
        max_x = max_x.max(object.transform.translate[0] + x + width);
        max_y = max_y.max(object.transform.translate[1] + y + height);
        found = true;
    }
    if !found {
        for node in &fragment.nodes {
            let point = object_local_point(object, node.position);
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
            found = true;
            if let Some(label) = &node.label {
                if let Some(bbox) = label.bbox() {
                    let bbox = translate_bbox(bbox, object.transform.translate);
                    min_x = min_x.min(bbox[0]);
                    min_y = min_y.min(bbox[1]);
                    max_x = max_x.max(bbox[2]);
                    max_y = max_y.max(bbox[3]);
                }
            }
        }
        let pad = 12.0;
        min_x -= pad;
        min_y -= pad;
        max_x += pad;
        max_y += pad;
    }
    found.then_some([min_x, min_y, max_x, max_y])
}

pub(super) fn translate_bbox(bbox: [f64; 4], translate: [f64; 2]) -> [f64; 4] {
    [
        bbox[0] + translate[0],
        bbox[1] + translate[1],
        bbox[2] + translate[0],
        bbox[3] + translate[1],
    ]
}

pub(super) fn imported_cdxml_label_attr<'a>(label: &'a NodeLabel, name: &str) -> Option<&'a str> {
    label
        .meta
        .get("import")?
        .get("cdxml")?
        .get(name)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn imported_cdxml_inherited_label_line_height_is_unchanged(label: &NodeLabel) -> bool {
    let Some(imported) = label.meta.pointer("/import/cdxml") else {
        return false;
    };
    if imported_cdxml_label_attr(label, "labelLineHeight").is_some()
        || imported_cdxml_label_attr(label, "lineHeight").is_some()
    {
        return false;
    }
    let Some(imported_mode) = imported
        .get("resolvedLineHeightMode")
        .and_then(Value::as_str)
    else {
        return false;
    };
    if imported_mode != label.line_height_mode {
        return false;
    }
    if imported_mode != "fixed" {
        return true;
    }
    let Some(imported_height) = imported.get("resolvedLineHeight").and_then(Value::as_f64) else {
        return false;
    };
    label
        .line_height
        .is_some_and(|height| (height - imported_height).abs() <= 1e-6)
}

pub(super) fn cdxml_generated_node_label_is_automatic(label: &NodeLabel) -> bool {
    let generated_without_explicit_edit = |key: &str| {
        let Some(meta) = label.meta.get(key) else {
            return false;
        };
        meta.get("source").and_then(Value::as_str) == Some("cdxml-generated")
            && meta.get("userEdited").and_then(Value::as_bool) != Some(true)
    };
    generated_without_explicit_edit("implicitHydrogenLabel")
        || generated_without_explicit_edit("queryListLabel")
        || generated_without_explicit_edit("carbonDisplayLabel")
        || generated_without_explicit_edit("carbonValenceLabel")
}

pub(super) fn imported_cdxml_object_attr<'a>(
    object: &'a SceneObject,
    name: &str,
) -> Option<&'a str> {
    object
        .meta
        .get("import")?
        .get("cdxml")?
        .get(name)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn bond_endpoint_attachment(bond: &Bond, endpoint: &str) -> Option<u64> {
    bond.meta
        .pointer(&format!("/endpointAttachments/{endpoint}"))
        .and_then(|attachment| attachment.get("characterIndex"))
        .and_then(Value::as_u64)
}

pub(super) fn imported_cdxml_crossing_bonds(bond: &Bond) -> impl Iterator<Item = &str> {
    bond.meta
        .pointer("/import/cdxml/crossingBonds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn cdxml_bond_crossing_scope(object: &SceneObject) -> String {
    object
        .meta
        .pointer("/import/cdxml/fragmentId")
        .and_then(Value::as_str)
        .map(|id| format!("cdxml-fragment:{id}"))
        .unwrap_or_else(|| format!("object:{}", object.id))
}

pub(super) fn collect_cdxml_bond_export_keys(
    document: &ChemSemaDocument,
    objects: &[SceneObject],
    keys: &mut Vec<(String, String)>,
) {
    for object in objects {
        if !object.visible {
            continue;
        }
        if object.object_type == "molecule" {
            if let Some(fragment) = object
                .payload
                .resource_ref
                .as_ref()
                .and_then(|resource_ref| document.resources.get(resource_ref))
                .and_then(|resource| resource.data.as_fragment())
            {
                let scope = cdxml_bond_crossing_scope(object);
                keys.extend(
                    fragment
                        .bonds
                        .iter()
                        .map(|bond| (scope.clone(), bond.id.clone())),
                );
            }
        }
        collect_cdxml_bond_export_keys(document, &object.children, keys);
    }
}
