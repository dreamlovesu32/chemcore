use crate::{
    round2, Point, DEFAULT_BOND_LENGTH_CM, DEFAULT_BOND_STROKE_CM,
    DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM, DEFAULT_PAGE_HEIGHT_CM, DEFAULT_PAGE_WIDTH_CM,
    DEFAULT_TEXT_BLOCK_PADDING_CM, DEFAULT_TEXT_FONT_SIZE_CM, DEFAULT_TEXT_LINE_HEIGHT_CM, EPSILON,
    PT_PER_CM,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const DEFAULT_PAGE_WIDTH: f64 = DEFAULT_PAGE_WIDTH_CM;
pub const DEFAULT_PAGE_HEIGHT: f64 = DEFAULT_PAGE_HEIGHT_CM;
pub const DEFAULT_BOND_LENGTH: f64 = DEFAULT_BOND_LENGTH_CM;
pub const DEFAULT_BOND_STROKE: f64 = DEFAULT_BOND_STROKE_CM;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChemcoreDocument {
    pub format: FormatInfo,
    pub document: DocumentInfo,
    #[serde(default)]
    pub styles: BTreeMap<String, Value>,
    #[serde(default)]
    pub objects: Vec<SceneObject>,
    #[serde(default)]
    pub resources: BTreeMap<String, Resource>,
}

impl ChemcoreDocument {
    pub fn blank() -> Self {
        let mut styles = BTreeMap::new();
        styles.insert(
            "style_molecule_default".to_string(),
            json!({
                "kind": "molecule",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "fontFamily": "Arial",
                "fontSize": DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM
            }),
        );
        styles.insert(
            "style_arrow_default".to_string(),
            json!({
                "kind": "stroke",
                "stroke": "#000000",
                "strokeWidth": DEFAULT_BOND_STROKE,
                "lineCap": "butt",
                "lineJoin": "miter"
            }),
        );

        let mut resources = BTreeMap::new();
        resources.insert(
            "mol_editor".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemcore.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment::blank()),
                meta: Value::Null,
            },
        );

        Self {
            format: FormatInfo {
                name: "chemcore".to_string(),
                version: "0.1".to_string(),
                unit: "pt".to_string(),
            },
            document: DocumentInfo {
                id: "doc_editor_untitled".to_string(),
                title: "Untitled".to_string(),
                page: Page {
                    width: DEFAULT_PAGE_WIDTH,
                    height: DEFAULT_PAGE_HEIGHT,
                    background: "#ffffff".to_string(),
                },
                meta: Value::Null,
            },
            styles,
            objects: vec![SceneObject {
                id: "obj_editor_molecule".to_string(),
                object_type: "molecule".to_string(),
                name: "molecule".to_string(),
                visible: true,
                locked: false,
                z_index: 10,
                transform: Transform::identity(),
                style_ref: Some("style_molecule_default".to_string()),
                meta: Value::Null,
                payload: ObjectPayload {
                    resource_ref: Some("mol_editor".to_string()),
                    bbox: Some([0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT]),
                    extra: BTreeMap::new(),
                },
                children: Vec::new(),
            }],
            resources,
        }
    }

    pub fn editable_fragment_mut(&mut self) -> Option<EditableFragmentMut<'_>> {
        let object = first_molecule_object_mut(&mut self.objects)?;
        let resource_ref = object.payload.resource_ref.clone()?;
        let resource = self.resources.get_mut(&resource_ref)?;
        let fragment = resource.data.as_fragment_mut()?;
        Some(EditableFragmentMut { object, fragment })
    }

    pub fn editable_fragment(&self) -> Option<EditableFragment<'_>> {
        let object = first_molecule_object(&self.objects)?;
        let resource_ref = object.payload.resource_ref.as_ref()?;
        let resource = self.resources.get(resource_ref)?;
        let fragment = resource.data.as_fragment()?;
        Some(EditableFragment { object, fragment })
    }

    pub fn scene_objects(&self) -> Vec<&SceneObject> {
        let mut out = Vec::new();
        collect_scene_objects(&self.objects, &mut out);
        out
    }

    pub fn find_scene_object(&self, object_id: &str) -> Option<&SceneObject> {
        find_scene_object(&self.objects, object_id)
    }

    pub fn find_scene_object_mut(&mut self, object_id: &str) -> Option<&mut SceneObject> {
        find_scene_object_mut(&mut self.objects, object_id)
    }

    pub fn remove_scene_objects_by_id(
        &mut self,
        object_ids: &std::collections::BTreeSet<&str>,
    ) -> usize {
        remove_scene_objects_by_id(&mut self.objects, object_ids)
    }
}

fn collect_scene_objects<'a>(objects: &'a [SceneObject], out: &mut Vec<&'a SceneObject>) {
    for object in objects {
        out.push(object);
        collect_scene_objects(&object.children, out);
    }
}

fn find_scene_object<'a>(objects: &'a [SceneObject], object_id: &str) -> Option<&'a SceneObject> {
    for object in objects {
        if object.id == object_id {
            return Some(object);
        }
        if let Some(found) = find_scene_object(&object.children, object_id) {
            return Some(found);
        }
    }
    None
}

fn find_scene_object_mut<'a>(
    objects: &'a mut [SceneObject],
    object_id: &str,
) -> Option<&'a mut SceneObject> {
    for object in objects {
        if object.id == object_id {
            return Some(object);
        }
        if let Some(found) = find_scene_object_mut(&mut object.children, object_id) {
            return Some(found);
        }
    }
    None
}

fn first_molecule_object(objects: &[SceneObject]) -> Option<&SceneObject> {
    for object in objects {
        if object.object_type == "molecule" {
            return Some(object);
        }
        if let Some(found) = first_molecule_object(&object.children) {
            return Some(found);
        }
    }
    None
}

fn first_molecule_object_mut(objects: &mut [SceneObject]) -> Option<&mut SceneObject> {
    for object in objects {
        if object.object_type == "molecule" {
            return Some(object);
        }
        if let Some(found) = first_molecule_object_mut(&mut object.children) {
            return Some(found);
        }
    }
    None
}

fn remove_scene_objects_by_id(
    objects: &mut Vec<SceneObject>,
    object_ids: &std::collections::BTreeSet<&str>,
) -> usize {
    let before = objects.len();
    objects.retain(|object| !object_ids.contains(object.id.as_str()));
    let mut removed = before - objects.len();
    for object in objects {
        removed += remove_scene_objects_by_id(&mut object.children, object_ids);
    }
    removed
}

pub fn parse_document_json(json: &str) -> Result<ChemcoreDocument, String> {
    let mut value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if document_json_uses_legacy_cm(&value) {
        scale_document_json_value(&mut value, PT_PER_CM);
    }
    ensure_document_json_unit(&mut value);
    let mut document: ChemcoreDocument =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    normalize_text_object_payloads(&mut document);
    normalize_shape_object_payloads(&mut document);
    normalize_arrow_object_payloads(&mut document);
    normalize_fragment_label_payloads(&mut document);
    Ok(document)
}

pub(crate) fn normalize_arrow_object_payloads(document: &mut ChemcoreDocument) {
    for object in &mut document.objects {
        if object.object_type == "line" {
            normalize_arrow_payload_extra(&mut object.payload.extra);
        }
    }
}

pub(crate) fn normalize_arrow_payload_extra(extra: &mut BTreeMap<String, Value>) {
    normalize_arrow_head_payload(extra);
    let curve = arrow_payload_curve(extra);
    if curve.abs() <= EPSILON {
        return;
    }
    if arrow_payload_geometry_is_valid(extra) {
        return;
    }
    let Some((start, end)) = arrow_payload_line_endpoints(extra) else {
        return;
    };
    if let Some(geometry) = default_arrow_arc_geometry_payload(start, end, curve) {
        extra.insert("arrowGeometry".to_string(), geometry);
    }
}

pub(crate) fn default_arrow_arc_geometry_payload(
    start: Point,
    end: Point,
    curve: f64,
) -> Option<Value> {
    let chord = Point::new(end.x - start.x, end.y - start.y);
    let chord_length = start.distance(end);
    if chord_length <= EPSILON || curve.abs() <= EPSILON {
        return None;
    }
    let sweep = -curve.to_radians();
    let half = sweep.abs() * 0.5;
    let sin_half = half.sin().abs();
    if sin_half <= EPSILON {
        return None;
    }
    let ux = chord.x / chord_length;
    let uy = chord.y / chord_length;
    let radius = chord_length / (2.0 * sin_half);
    let offset = radius * half.cos() * sweep.signum();
    let center = Point::new(
        (start.x + end.x) * 0.5 - uy * offset,
        (start.y + end.y) * 0.5 + ux * offset,
    );
    Some(json!({
        "center": [round2(center.x), round2(center.y)],
        "majorAxisEnd": [round2(center.x + radius), round2(center.y)],
        "minorAxisEnd": [round2(center.x), round2(center.y + radius)]
    }))
}

fn normalize_arrow_head_payload(extra: &mut BTreeMap<String, Value>) {
    let legacy_head = extra
        .get("head")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let legacy_tail = extra
        .get("tail")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let Some(Value::Object(arrow_head)) = extra.get_mut("arrowHead") else {
        return;
    };

    let length = object_number(arrow_head, "length")
        .filter(|value| *value > EPSILON)
        .unwrap_or(crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO);
    arrow_head.insert("length".to_string(), json!(round2(length)));

    let center_length = object_number(arrow_head, "centerLength")
        .or_else(|| object_number(arrow_head, "center_length"))
        .filter(|value| *value > 0.0)
        .unwrap_or(length * 0.875);
    arrow_head.insert("centerLength".to_string(), json!(round2(center_length)));

    let width = object_number(arrow_head, "width")
        .filter(|value| *value >= 0.0)
        .unwrap_or(length * 0.25);
    arrow_head.insert("width".to_string(), json!(round2(width)));

    let kind = arrow_head
        .get("kind")
        .and_then(Value::as_str)
        .map(canonical_arrow_head_kind)
        .unwrap_or("solid");
    arrow_head.insert("kind".to_string(), json!(kind));

    let curve = object_number(arrow_head, "curve").unwrap_or(0.0);
    arrow_head.insert("curve".to_string(), json!(round2(curve)));

    let head = arrow_head
        .get("head")
        .and_then(Value::as_str)
        .map(canonical_arrow_endpoint_payload)
        .unwrap_or_else(|| canonical_legacy_arrow_endpoint(legacy_head.as_deref(), "end"));
    arrow_head.insert("head".to_string(), json!(head));

    let tail = arrow_head
        .get("tail")
        .and_then(Value::as_str)
        .map(canonical_arrow_endpoint_payload)
        .unwrap_or_else(|| canonical_legacy_arrow_endpoint(legacy_tail.as_deref(), "start"));
    arrow_head.insert("tail".to_string(), json!(tail));

    let bold = arrow_head
        .get("bold")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    arrow_head.insert("bold".to_string(), json!(bold));

    let no_go = arrow_head
        .get("noGo")
        .or_else(|| arrow_head.get("no_go"))
        .and_then(Value::as_str)
        .map(canonical_arrow_no_go)
        .unwrap_or("none");
    arrow_head.insert("noGo".to_string(), json!(no_go));
}

fn object_number(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key)?.as_f64().filter(|value| value.is_finite())
}

fn canonical_arrow_head_kind(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "hollow" => "hollow",
        "angle" | "open" | "retrosynthetic" => "open",
        _ => "solid",
    }
}

fn canonical_arrow_endpoint_payload(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "full" => "full",
        "half-left" | "halfleft" | "left" | "top" => "half-left",
        "half-right" | "halfright" | "right" | "bottom" => "half-right",
        _ => "none",
    }
}

fn canonical_legacy_arrow_endpoint(value: Option<&str>, enabled: &str) -> &'static str {
    if value.is_some_and(|value| {
        value.eq_ignore_ascii_case(enabled) || value.eq_ignore_ascii_case("both")
    }) {
        "full"
    } else {
        "none"
    }
}

fn canonical_arrow_no_go(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "cross" => "cross",
        "hash" => "hash",
        _ => "none",
    }
}

pub(crate) fn arrow_payload_line_endpoints(
    extra: &BTreeMap<String, Value>,
) -> Option<(Point, Point)> {
    let points = extra.get("points")?.as_array()?;
    let start = points.first()?.as_array()?;
    let end = points.get(1)?.as_array()?;
    Some((
        Point::new(start.first()?.as_f64()?, start.get(1)?.as_f64()?),
        Point::new(end.first()?.as_f64()?, end.get(1)?.as_f64()?),
    ))
}

fn arrow_payload_curve(extra: &BTreeMap<String, Value>) -> f64 {
    extra
        .get("arrowHead")
        .and_then(|value| value.get("curve"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn arrow_payload_geometry_is_valid(extra: &BTreeMap<String, Value>) -> bool {
    ["center", "majorAxisEnd", "minorAxisEnd"]
        .into_iter()
        .all(|key| {
            extra
                .get("arrowGeometry")
                .and_then(|geometry| geometry.get(key))
                .and_then(Value::as_array)
                .is_some_and(|coords| {
                    coords.first().and_then(Value::as_f64).is_some()
                        && coords.get(1).and_then(Value::as_f64).is_some()
                })
        })
}

pub(crate) fn normalize_text_object_payloads(document: &mut ChemcoreDocument) {
    let styles = document.styles.clone();
    for object in &mut document.objects {
        if object.object_type != "text" {
            continue;
        }
        normalize_text_object_payload_defaults(object, &styles);
        let align = object
            .payload
            .extra
            .get("align")
            .and_then(Value::as_str)
            .unwrap_or("left");
        let anchor_x = match align {
            "center" => 0.5,
            "right" => 1.0,
            _ => continue,
        };
        let Some(mut box_value) = object
            .payload
            .extra
            .get("box")
            .cloned()
            .and_then(|value| serde_json::from_value::<[f64; 4]>(value).ok())
        else {
            continue;
        };
        if box_value[0].abs() > crate::EPSILON
            || box_value[2] <= crate::EPSILON
            || !box_value.iter().all(|value| value.is_finite())
        {
            continue;
        }
        box_value[0] = round2(-box_value[2] * anchor_x);
        object
            .payload
            .extra
            .insert("box".to_string(), json!(box_value));
        if let Some(bbox) = object.payload.bbox.as_mut() {
            if bbox[0].abs() <= crate::EPSILON
                && (bbox[2] - box_value[2]).abs() <= crate::EPSILON
                && bbox.iter().all(|value| value.is_finite())
            {
                bbox[0] = box_value[0];
            }
        }
    }
}

fn normalize_text_object_payload_defaults(
    object: &mut SceneObject,
    styles: &BTreeMap<String, Value>,
) {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| styles.get(style_ref));
    let font_size = object
        .payload
        .extra
        .get("fontSize")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| style.and_then(|style| style_number(style, "fontSize")))
        .or_else(|| style.and_then(|style| style_number(style, "font_size")))
        .unwrap_or(DEFAULT_TEXT_FONT_SIZE_CM);
    object
        .payload
        .extra
        .insert("fontSize".to_string(), json!(round2(font_size)));

    let line_height = object
        .payload
        .extra
        .get("lineHeight")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(DEFAULT_TEXT_LINE_HEIGHT_CM);
    object
        .payload
        .extra
        .insert("lineHeight".to_string(), json!(round2(line_height)));

    object
        .payload
        .extra
        .entry("align".to_string())
        .or_insert_with(|| json!("left"));
    object
        .payload
        .extra
        .entry("preserveLines".to_string())
        .or_insert_with(|| json!(false));

    if text_payload_box(&object.payload.extra).is_none() {
        let text = object
            .payload
            .extra
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("");
        let box_value = object
            .payload
            .bbox
            .filter(valid_bbox)
            .unwrap_or_else(|| default_text_object_box(text, font_size, line_height));
        object.payload.extra.insert(
            "box".to_string(),
            json!([
                round2(box_value[0]),
                round2(box_value[1]),
                round2(box_value[2]),
                round2(box_value[3])
            ]),
        );
    }
}

fn style_number(style: &Value, key: &str) -> Option<f64> {
    style
        .get(key)?
        .as_f64()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn text_payload_box(extra: &BTreeMap<String, Value>) -> Option<[f64; 4]> {
    let value = extra.get("box")?;
    serde_json::from_value::<[f64; 4]>(value.clone())
        .ok()
        .filter(valid_bbox)
}

fn default_text_object_box(text: &str, font_size: f64, line_height: f64) -> [f64; 4] {
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f64;
    let line_count = text.lines().count().max(1) as f64;
    [
        0.0,
        0.0,
        (max_chars * font_size * 0.55).max(font_size),
        (line_count * line_height).max(font_size),
    ]
}

fn valid_bbox(bbox: &[f64; 4]) -> bool {
    bbox.iter().all(|value| value.is_finite())
}

pub(crate) fn normalize_shape_object_payloads(document: &mut ChemcoreDocument) {
    for object in &mut document.objects {
        if object.object_type != "shape" {
            continue;
        }
        let kind = object
            .payload
            .extra
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("rect")
            .to_string();
        if !matches!(kind.as_str(), "circle" | "ellipse") {
            continue;
        }
        if shape_oval_geometry_is_valid(&object.payload.extra) {
            continue;
        }
        let Some([x, y, width, height]) = object.payload.bbox.filter(valid_bbox) else {
            continue;
        };
        let center = Point::new(
            object.transform.translate[0] + x + width * 0.5,
            object.transform.translate[1] + y + height * 0.5,
        );
        let major = Point::new(center.x + width.abs() * 0.5, center.y);
        let minor_radius = if height.abs() > EPSILON {
            height.abs() * 0.5
        } else {
            width.abs() * 0.5
        };
        let minor = Point::new(center.x, center.y + minor_radius);
        object.payload.extra.insert(
            "center".to_string(),
            json!([round2(center.x), round2(center.y)]),
        );
        object.payload.extra.insert(
            "majorAxisEnd".to_string(),
            json!([round2(major.x), round2(major.y)]),
        );
        object.payload.extra.insert(
            "minorAxisEnd".to_string(),
            json!([round2(minor.x), round2(minor.y)]),
        );
    }
}

fn shape_oval_geometry_is_valid(extra: &BTreeMap<String, Value>) -> bool {
    ["center", "majorAxisEnd", "minorAxisEnd"]
        .into_iter()
        .all(|key| {
            extra
                .get(key)
                .and_then(Value::as_array)
                .is_some_and(|coords| {
                    coords.first().and_then(Value::as_f64).is_some()
                        && coords.get(1).and_then(Value::as_f64).is_some()
                })
        })
}

pub(crate) fn normalize_fragment_label_payloads(document: &mut ChemcoreDocument) {
    for resource in document.resources.values_mut() {
        let Some(fragment) = resource.data.as_fragment_mut() else {
            continue;
        };
        for node in &mut fragment.nodes {
            if let Some(label) = &mut node.label {
                normalize_node_label_payload(label, node.position);
            }
        }
    }
}

fn normalize_node_label_payload(label: &mut NodeLabel, node_position: [f64; 2]) {
    if label.position.is_none() {
        label.position = Some(node_position);
    }
    if label.font_size.is_none() {
        label.font_size = Some(DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM);
    }
    if label.align.is_none() {
        label.align = Some("left".to_string());
    }
    if label.font_family.is_none() {
        label.font_family = Some("Arial".to_string());
    }
    if label.fill.is_none() {
        label.fill = Some("#000000".to_string());
    }
    if label.box_value.is_none() && label.box_field.is_none() {
        let font_size = label
            .font_size
            .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM);
        let position = label.position.unwrap_or(node_position);
        label.box_field = Some(default_node_label_box(position, &label.text, font_size));
    }
}

fn default_node_label_box(position: [f64; 2], text: &str, font_size: f64) -> [f64; 4] {
    let line_count = text.lines().count().max(1) as f64;
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f64;
    let width = (max_chars * font_size * 0.58).max(font_size);
    let height = (line_count * font_size * 1.25).max(font_size);
    [
        round2(position[0]),
        round2(position[1] - font_size),
        round2(position[0] + width),
        round2(position[1] - font_size + height),
    ]
}

fn document_json_uses_legacy_cm(value: &Value) -> bool {
    let unit = value
        .get("format")
        .and_then(|format| format.get("unit"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if unit.eq_ignore_ascii_case("cm") {
        return true;
    }
    if !unit.is_empty() {
        return false;
    }

    let width = value
        .get("document")
        .and_then(|document| document.get("page"))
        .and_then(|page| page.get("width"))
        .and_then(Value::as_f64);
    let height = value
        .get("document")
        .and_then(|document| document.get("page"))
        .and_then(|page| page.get("height"))
        .and_then(Value::as_f64);
    matches!((width, height), (Some(width), Some(height)) if width <= 100.0 && height <= 100.0)
}

fn ensure_document_json_unit(value: &mut Value) {
    if !value.is_object() {
        return;
    }
    let Some(format) = value.get_mut("format").and_then(Value::as_object_mut) else {
        return;
    };
    format.insert("unit".to_string(), Value::String("pt".to_string()));
}

fn scale_document_json_value(value: &mut Value, factor: f64) {
    scale_json_value_by_key("", value, factor);
}

fn scale_json_value_by_key(key: &str, value: &mut Value, factor: f64) {
    if scale_key_as_length_scalar(key) {
        scale_all_numbers(value, factor);
        return;
    }
    match value {
        Value::Array(items) if scale_key_as_length_array(key) => {
            for item in items {
                scale_all_numbers(item, factor);
            }
        }
        Value::Array(items) => {
            for item in items {
                scale_json_value_by_key("", item, factor);
            }
        }
        Value::Object(object) => {
            for (child_key, child_value) in object {
                scale_json_value_by_key(child_key, child_value, factor);
            }
        }
        _ => {}
    }
}

fn scale_all_numbers(value: &mut Value, factor: f64) {
    match value {
        Value::Number(number) => {
            if let Some(scaled) = number
                .as_f64()
                .and_then(|value| serde_json::Number::from_f64(value * factor))
            {
                *number = scaled;
            }
        }
        Value::Array(items) => {
            for item in items {
                scale_all_numbers(item, factor);
            }
        }
        Value::Object(object) => {
            for child_value in object.values_mut() {
                scale_all_numbers(child_value, factor);
            }
        }
        _ => {}
    }
}

fn scale_key_as_length_scalar(key: &str) -> bool {
    matches!(
        key,
        "width"
            | "height"
            | "x"
            | "y"
            | "strokeWidth"
            | "fontSize"
            | "lineHeight"
            | "wrapWidth"
            | "pad"
            | "padding"
    )
}

fn scale_key_as_length_array(key: &str) -> bool {
    matches!(
        key,
        "bbox"
            | "box"
            | "boxField"
            | "boundingBox"
            | "position"
            | "translate"
            | "points"
            | "center"
            | "majorAxisEnd"
            | "minorAxisEnd"
            | "anchorOffset"
            | "glyphPolygons"
    )
}

fn default_format_unit() -> String {
    "pt".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub name: String,
    pub version: String,
    #[serde(default = "default_format_unit")]
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub title: String,
    pub page: Page,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default)]
    pub transform: Transform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_ref: Option<String>,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub payload: ObjectPayload,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SceneObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub translate: [f64; 2],
    pub rotate: f64,
    pub scale: [f64; 2],
}

impl Transform {
    pub const fn identity() -> Self {
        Self {
            translate: [0.0, 0.0],
            rotate: 0.0,
            scale: [1.0, 1.0],
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub encoding: String,
    pub data: ResourceData,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceData {
    Fragment(MoleculeFragment),
    Text(String),
    Json(Value),
}

impl ResourceData {
    pub fn as_fragment(&self) -> Option<&MoleculeFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub fn as_fragment_mut(&mut self) -> Option<&mut MoleculeFragment> {
        match self {
            Self::Fragment(fragment) => Some(fragment),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeFragment {
    pub schema: String,
    pub bbox: [f64; 4],
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub bonds: Vec<Bond>,
    #[serde(default)]
    pub meta: Value,
}

impl MoleculeFragment {
    pub fn blank() -> Self {
        Self {
            schema: "chemcore.molecule.fragment2d".to_string(),
            bbox: [0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT],
            nodes: Vec::new(),
            bonds: Vec::new(),
            meta: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub element: String,
    pub atomic_number: u8,
    pub position: [f64; 2],
    pub charge: i32,
    pub num_hydrogens: u8,
    #[serde(default)]
    pub is_external_connection_point: bool,
    #[serde(default)]
    pub is_placeholder: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<NodeLabel>,
    #[serde(default)]
    pub meta: Value,
}

impl Node {
    pub fn carbon(id: String, point: Point) -> Self {
        Self {
            id,
            element: "C".to_string(),
            atomic_number: 6,
            position: [round2(point.x), round2(point.y)],
            charge: 0,
            num_hydrogens: 0,
            is_external_connection_point: false,
            is_placeholder: false,
            label: None,
            meta: Value::Null,
        }
    }

    pub fn point(&self) -> Point {
        Point::new(self.position[0], self.position[1])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeLabel {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub box_field: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<LabelRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub line_runs: Vec<Vec<LabelRun>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glyph_polygons: Vec<Vec<[f64; 2]>>,
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub box_value: Option<[f64; 4]>,
    #[serde(default)]
    pub meta: Value,
}

impl NodeLabel {
    pub fn bbox(&self) -> Option<[f64; 4]> {
        self.box_value.or(self.box_field)
    }

    pub fn has_visible_text(&self) -> bool {
        !self.text.trim().is_empty()
    }

    pub fn glyph_polygons(&self) -> Vec<Vec<Point>> {
        self.glyph_polygons
            .iter()
            .map(|polygon| {
                polygon
                    .iter()
                    .map(|point| Point::new(point[0], point[1]))
                    .collect()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LabelRun {
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bond {
    pub id: String,
    pub begin: String,
    pub end: String,
    pub order: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double: Option<DoubleBond>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereo: Option<BondStereo>,
    #[serde(default)]
    pub stroke_width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wedge_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_clip_margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_width: Option<f64>,
    #[serde(default)]
    pub line_styles: BondLineStyles,
    #[serde(default)]
    pub line_weights: BondLineWeights,
    #[serde(default)]
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubleBond {
    pub placement: DoubleBondPlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_exit_side: Option<DoubleBondPlacement>,
    #[serde(default)]
    pub frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BondLineStyles {
    #[serde(default)]
    pub main: BondLinePattern,
    #[serde(default)]
    pub left: BondLinePattern,
    #[serde(default)]
    pub right: BondLinePattern,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BondLineWeights {
    #[serde(default)]
    pub main: BondLineWeight,
    #[serde(default)]
    pub left: BondLineWeight,
    #[serde(default)]
    pub right: BondLineWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BondLinePattern {
    #[default]
    Solid,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BondLineWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BondStereo {
    pub kind: String,
    pub wide_end: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DoubleBondPlacement {
    Left,
    Right,
    Center,
}

pub struct EditableFragment<'a> {
    pub object: &'a SceneObject,
    pub fragment: &'a MoleculeFragment,
}

impl EditableFragment<'_> {
    pub fn world_point_for_node(&self, node: &Node) -> Point {
        Point::new(
            self.object.transform.translate[0] + node.position[0],
            self.object.transform.translate[1] + node.position[1],
        )
    }
}

pub struct EditableFragmentMut<'a> {
    pub object: &'a mut SceneObject,
    pub fragment: &'a mut MoleculeFragment,
}

impl EditableFragmentMut<'_> {
    pub fn world_point_for_node(&self, node: &Node) -> Point {
        Point::new(
            self.object.transform.translate[0] + node.position[0],
            self.object.transform.translate[1] + node.position[1],
        )
    }

    pub fn local_point(&self, point: Point) -> Point {
        Point::new(
            point.x - self.object.transform.translate[0],
            point.y - self.object.transform.translate[1],
        )
    }

    pub fn update_bounds(&mut self) {
        self.fragment.bbox = fragment_content_bbox(&self.fragment.nodes)
            .unwrap_or([0.0, 0.0, DEFAULT_PAGE_WIDTH, DEFAULT_PAGE_HEIGHT]);
        self.object.payload.bbox = Some(self.fragment.bbox);
    }
}

fn fragment_content_bbox(nodes: &[Node]) -> Option<[f64; 4]> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;

    for node in nodes {
        min_x = min_x.min(node.position[0] - DEFAULT_TEXT_BLOCK_PADDING_CM);
        min_y = min_y.min(node.position[1] - DEFAULT_TEXT_BLOCK_PADDING_CM);
        max_x = max_x.max(node.position[0] + DEFAULT_TEXT_BLOCK_PADDING_CM);
        max_y = max_y.max(node.position[1] + DEFAULT_TEXT_BLOCK_PADDING_CM);
        found = true;

        if let Some(label) = &node.label {
            if let Some([x1, y1, x2, y2]) = label.bbox() {
                min_x = min_x.min(x1);
                min_y = min_y.min(y1);
                max_x = max_x.max(x2);
                max_y = max_y.max(y2);
                found = true;
            }
        }
    }

    found.then_some([
        round2(min_x),
        round2(min_y),
        round2((max_x - min_x).max(1.0)),
        round2((max_y - min_y).max(1.0)),
    ])
}
