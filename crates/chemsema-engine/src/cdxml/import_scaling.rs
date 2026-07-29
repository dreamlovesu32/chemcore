use super::*;

pub(crate) fn normalize_cdxml_document_for_editing(document: &mut ChemSemaDocument) {
    scale_cdxml_document_for_editing(document);
}

pub(super) fn scale_cdxml_document_for_editing(document: &mut ChemSemaDocument) {
    if document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .is_some()
    {
        return;
    }

    let factor = CDXML_EDITING_OUTPUT_SCALE;
    document.document.page.width = round2(document.document.page.width * factor);
    document.document.page.height = round2(document.document.page.height * factor);
    let layout = &mut document.document.layout;
    layout.paper.width = round2(layout.paper.width * factor);
    layout.paper.height = round2(layout.paper.height * factor);
    for margin in &mut layout.margins {
        *margin = round2(*margin * factor);
    }
    layout.page_overlap = round2(layout.page_overlap * factor);
    layout.header_position = round2(layout.header_position * factor);
    layout.footer_position = round2(layout.footer_position * factor);
    if let Some(origin) = &mut layout.page_origin {
        origin[0] = round2(origin[0] * factor);
        origin[1] = round2(origin[1] * factor);
    }
    for splitter in &mut layout.splitters {
        if let Some(position) = &mut splitter.position {
            position[0] = round2(position[0] * factor);
            position[1] = round2(position[1] * factor);
        }
    }
    for point in [
        &mut layout.fix_in_place_extent,
        &mut layout.fix_in_place_gap,
    ]
    .into_iter()
    .flatten()
    {
        point[0] = round2(point[0] * factor);
        point[1] = round2(point[1] * factor);
    }
    for style in document.styles.values_mut() {
        scale_json_value_by_key("", style, factor);
    }
    scale_scene_objects_for_editing(&mut document.objects, factor);
    for resource in document.resources.values_mut() {
        if let ResourceData::Fragment(fragment) = &mut resource.data {
            scale_fragment_for_editing(fragment, factor);
        }
    }
    if let Some(cdxml_meta) = document
        .document
        .meta
        .get_mut("import")
        .and_then(|value| value.get_mut("cdxml"))
        .and_then(Value::as_object_mut)
    {
        cdxml_meta.insert("editingScale".to_string(), json!(factor));
    }
}

pub(super) fn scale_scene_objects_for_editing(objects: &mut [SceneObject], factor: f64) {
    for object in objects {
        object.transform.translate[0] = round2(object.transform.translate[0] * factor);
        object.transform.translate[1] = round2(object.transform.translate[1] * factor);
        if let Some(bbox) = &mut object.payload.bbox {
            scale_bbox_for_editing(bbox, factor);
        }
        for (key, value) in &mut object.payload.extra {
            if key == "arrowHead" {
                continue;
            }
            scale_json_value_by_key(key, value, factor);
        }
        scale_scene_objects_for_editing(&mut object.children, factor);
    }
}

pub(super) fn scale_fragment_for_editing(fragment: &mut MoleculeFragment, factor: f64) {
    scale_bbox_for_editing(&mut fragment.bbox, factor);
    for node in &mut fragment.nodes {
        scale_point_array_for_editing(&mut node.position, factor);
        if let Some(label) = &mut node.label {
            if let Some(position) = &mut label.position {
                scale_point_array_for_editing(position, factor);
            }
            if let Some(bbox) = &mut label.box_field {
                scale_bbox_for_editing(bbox, factor);
            }
            if let Some(bbox) = &mut label.box_value {
                scale_bbox_for_editing(bbox, factor);
            }
            if let Some(font_size) = &mut label.font_size {
                *font_size = round2(*font_size * factor);
            }
            for polygon in &mut label.glyph_polygons {
                for point in polygon {
                    scale_point_array_for_editing(point, factor);
                }
            }
            for polygon in &mut label.glyph_clip_polygons {
                for point in polygon {
                    scale_point_array_for_editing(point, factor);
                }
            }
            scale_label_runs_for_editing(&mut label.runs, factor);
            for runs in &mut label.line_runs {
                scale_label_runs_for_editing(runs, factor);
            }
        }
    }
    for bond in &mut fragment.bonds {
        bond.stroke_width = round2(bond.stroke_width * factor);
        scale_optional_number_for_editing(&mut bond.bold_width, factor);
        scale_optional_number_for_editing(&mut bond.wedge_width, factor);
        scale_optional_number_for_editing(&mut bond.label_clip_margin, factor);
        scale_optional_number_for_editing(&mut bond.hash_spacing, factor);
        scale_optional_number_for_editing(&mut bond.margin_width, factor);
    }
}

pub(super) fn scale_label_runs_for_editing(runs: &mut [LabelRun], factor: f64) {
    for run in runs {
        if let Some(font_size) = &mut run.font_size {
            *font_size = round2(*font_size * factor);
        }
    }
}

pub(super) fn scale_optional_number_for_editing(value: &mut Option<f64>, factor: f64) {
    if let Some(number) = value {
        *number = round2(*number * factor);
    }
}

pub(super) fn scale_bbox_for_editing(bbox: &mut [f64; 4], factor: f64) {
    for value in bbox {
        *value = round2(*value * factor);
    }
}

pub(super) fn scale_point_array_for_editing(point: &mut [f64; 2], factor: f64) {
    point[0] = round2(point[0] * factor);
    point[1] = round2(point[1] * factor);
}

pub(super) fn scale_json_value_by_key(key: &str, value: &mut Value, factor: f64) {
    if scale_json_key_as_length_scalar(key) || scale_json_key_as_length_array(key) {
        scale_all_json_numbers(value, factor);
        return;
    }
    match value {
        Value::Array(items) => {
            for item in items {
                scale_json_value_by_key("", item, factor);
            }
        }
        Value::Object(map) => {
            for (child_key, child_value) in map {
                if child_key == "arrowHead" {
                    continue;
                }
                scale_json_value_by_key(child_key, child_value, factor);
            }
        }
        _ => {}
    }
}

pub(super) fn scale_all_json_numbers(value: &mut Value, factor: f64) {
    match value {
        Value::Number(number) => {
            if let Some(raw) = number.as_f64() {
                *value = json!(round2(raw * factor));
            }
        }
        Value::Array(items) => {
            for item in items {
                scale_all_json_numbers(item, factor);
            }
        }
        Value::Object(map) => {
            for child_value in map.values_mut() {
                scale_all_json_numbers(child_value, factor);
            }
        }
        _ => {}
    }
}

pub(super) fn scale_json_key_as_length_scalar(key: &str) -> bool {
    matches!(
        key,
        "width"
            | "height"
            | "x"
            | "y"
            | "rx"
            | "ry"
            | "radius"
            | "strokeWidth"
            | "fontSize"
            | "lineHeight"
            | "anchorOffsetX"
            | "baselineOffset"
            | "wrapWidth"
            | "pad"
            | "padding"
            | "cornerRadius"
            | "shadowSize"
            | "dashSpacing"
    )
}

pub(super) fn scale_json_key_as_length_array(key: &str) -> bool {
    matches!(
        key,
        "bbox"
            | "box"
            | "boxField"
            | "boundingBox"
            | "cdxmlBoundingBox"
            | "position"
            | "textPosition"
            | "textOffsetFromNode"
            | "translate"
            | "points"
            | "center"
            | "majorAxisEnd"
            | "minorAxisEnd"
            | "axisStart"
            | "axisEnd"
            | "anchorOffset"
            | "glyphPolygons"
            | "lineAdvances"
            | "dashArray"
    )
}
