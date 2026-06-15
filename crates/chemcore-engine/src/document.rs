use crate::{
    round2, Point, DEFAULT_BOND_LENGTH_PT, DEFAULT_BOND_STROKE_PT,
    DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT, DEFAULT_PAGE_HEIGHT_PT, DEFAULT_PAGE_WIDTH_PT,
    DEFAULT_TEXT_BLOCK_PADDING_PT, DEFAULT_TEXT_FONT_SIZE_PT, DEFAULT_TEXT_LINE_HEIGHT_PT, EPSILON,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const DEFAULT_PAGE_WIDTH: f64 = DEFAULT_PAGE_WIDTH_PT;
pub const DEFAULT_PAGE_HEIGHT: f64 = DEFAULT_PAGE_HEIGHT_PT;
pub const DEFAULT_BOND_LENGTH: f64 = DEFAULT_BOND_LENGTH_PT;
pub const DEFAULT_BOND_STROKE: f64 = DEFAULT_BOND_STROKE_PT;

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
                "fontSize": DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT
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
    ensure_document_json_pt_unit(&mut value)?;
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

    if matches!(kind, "equilibrium" | "unequal-equilibrium") {
        let shaft_spacing = object_number(arrow_head, "shaftSpacing")
            .or_else(|| object_number(arrow_head, "shaft_spacing"))
            .filter(|value| *value > 0.0)
            .unwrap_or(3.0);
        arrow_head.insert("shaftSpacing".to_string(), json!(round2(shaft_spacing)));
        if kind == "unequal-equilibrium" {
            let ratio = object_number(arrow_head, "equilibriumRatio")
                .or_else(|| object_number(arrow_head, "equilibrium_ratio"))
                .filter(|value| *value > 1.0)
                .unwrap_or(3.0);
            arrow_head.insert("equilibriumRatio".to_string(), json!(round2(ratio)));
        } else {
            arrow_head.remove("equilibriumRatio");
            arrow_head.remove("equilibrium_ratio");
        }
    }
}

fn object_number(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object.get(key)?.as_f64().filter(|value| value.is_finite())
}

fn canonical_arrow_head_kind(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "hollow" => "hollow",
        "angle" | "open" | "retrosynthetic" => "open",
        "equilibrium" => "equilibrium",
        "unequal-equilibrium" | "unequilibrium" | "unbalanced-equilibrium" => "unequal-equilibrium",
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
        .unwrap_or(DEFAULT_TEXT_FONT_SIZE_PT);
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
        .unwrap_or(DEFAULT_TEXT_LINE_HEIGHT_PT);
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
        let node_positions: BTreeMap<String, [f64; 2]> = fragment
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.position))
            .collect();
        let anchor_sides: BTreeMap<String, ImportedLabelAnchorSide> = fragment
            .nodes
            .iter()
            .filter_map(|node| {
                imported_label_anchor_side_for_node(node, &fragment.bonds, &node_positions)
                    .map(|side| (node.id.clone(), side))
            })
            .collect();
        for node in &mut fragment.nodes {
            if let Some(label) = &mut node.label {
                normalize_node_label_payload(
                    label,
                    node.position,
                    node.atomic_number,
                    anchor_sides.get(&node.id).copied(),
                );
            }
        }
    }
}

fn normalize_node_label_payload(
    label: &mut NodeLabel,
    node_position: [f64; 2],
    node_atomic_number: u8,
    anchor_side: Option<ImportedLabelAnchorSide>,
) {
    if label.position.is_none() {
        label.position = Some(node_position);
    }
    if label.font_size.is_none() {
        label.font_size = Some(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
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
            .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
        let position = label.position.unwrap_or(node_position);
        label.box_field = Some(default_node_label_box(position, &label.text, font_size));
    }
    if label.glyph_polygons.is_empty() || label.meta.pointer("/import/cdxml/boundingBox").is_some()
    {
        rebuild_node_label_glyph_polygons(label, node_position, node_atomic_number, anchor_side);
    }
}

fn rebuild_node_label_glyph_polygons(
    label: &mut NodeLabel,
    node_position: [f64; 2],
    node_atomic_number: u8,
    anchor_side: Option<ImportedLabelAnchorSide>,
) {
    if !label.has_visible_text() {
        label.glyph_polygons.clear();
        return;
    }

    let position = label.position.unwrap_or(node_position);
    let font_size = label
        .font_size
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    let local_bbox = label.bbox();
    let align = label.align.as_deref().unwrap_or("left");
    let line_runs = if label.line_runs.is_empty() {
        &[][..]
    } else {
        label.line_runs.as_slice()
    };
    let single_line_runs = if line_runs.is_empty() {
        label.runs.as_slice()
    } else {
        &[][..]
    };

    label.glyph_polygons = if align == "center" {
        let width = local_bbox
            .map(|bbox| (bbox[2] - bbox[0]).abs())
            .filter(|width| *width > EPSILON)
            .unwrap_or_else(|| {
                (label.text.chars().count() as f64 * font_size * 0.55).max(font_size)
            });
        crate::build_label_glyph_polygons(
            single_line_runs,
            line_runs,
            [round2(position[0] - width * 0.5), position[1]],
            local_bbox,
            font_size,
        )
    } else if align == "right" {
        let width = local_bbox
            .map(|bbox| (bbox[2] - bbox[0]).abs())
            .filter(|width| *width > EPSILON)
            .unwrap_or_else(|| {
                (label.text.chars().count() as f64 * font_size * 0.55).max(font_size)
            });
        crate::build_label_glyph_polygons(
            single_line_runs,
            line_runs,
            [round2(position[0] - width), position[1]],
            local_bbox,
            font_size,
        )
    } else {
        crate::build_label_glyph_polygons(
            single_line_runs,
            line_runs,
            position,
            local_bbox,
            font_size,
        )
    };

    if !imported_cdxml_bullet_carbon_node_label(label, node_atomic_number) {
        align_imported_node_label_glyph_anchor(label, node_position, anchor_side);
    }
}

fn imported_cdxml_bullet_carbon_node_label(label: &NodeLabel, node_atomic_number: u8) -> bool {
    node_atomic_number == 6
        && label.attachment.as_deref() == Some("node")
        && label.source_text.as_deref().unwrap_or(label.text.as_str()) == "•"
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
        && label.meta.pointer("/import/cdxml/textPosition").is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImportedLabelAnchorSide {
    Left,
    Right,
}

fn imported_label_anchor_side_for_node(
    node: &Node,
    bonds: &[Bond],
    node_positions: &BTreeMap<String, [f64; 2]>,
) -> Option<ImportedLabelAnchorSide> {
    node.label.as_ref()?;
    let mut side = None;
    for bond in bonds {
        let other_id = if bond.begin == node.id {
            &bond.end
        } else if bond.end == node.id {
            &bond.begin
        } else {
            continue;
        };
        let other_position = node_positions.get(other_id)?;
        let dx = other_position[0] - node.position[0];
        if dx.abs() <= EPSILON {
            continue;
        }
        let next_side = if dx > 0.0 {
            ImportedLabelAnchorSide::Right
        } else {
            ImportedLabelAnchorSide::Left
        };
        if side.is_some_and(|current| current != next_side) {
            return None;
        }
        side = Some(next_side);
    }
    side
}

fn align_imported_node_label_glyph_anchor(
    label: &mut NodeLabel,
    node_position: [f64; 2],
    anchor_side: Option<ImportedLabelAnchorSide>,
) {
    if label.attachment.as_deref() != Some("node")
        || label.meta.pointer("/import/cdxml/boundingBox").is_none()
    {
        return;
    }
    let Some(anchor) = imported_node_label_anchor_point(label, anchor_side) else {
        return;
    };
    let delta_x = round2(node_position[0] - anchor[0]);
    let delta_y = round2(node_position[1] - anchor[1]);
    if delta_x.abs() > EPSILON || delta_y.abs() > EPSILON {
        for polygon in &mut label.glyph_polygons {
            for point in polygon {
                point[0] = round2(point[0] + delta_x);
                point[1] = round2(point[1] + delta_y);
            }
        }
    }

    if let Some(bbox) = &mut label.box_field {
        if delta_x.abs() > EPSILON || delta_y.abs() > EPSILON {
            bbox[0] = round2(bbox[0] + delta_x);
            bbox[1] = round2(bbox[1] + delta_y);
            bbox[2] = round2(bbox[2] + delta_x);
            bbox[3] = round2(bbox[3] + delta_y);
            if let Some(position) = &mut label.position {
                position[0] = round2(position[0] + delta_x);
                position[1] = round2(position[1] + delta_y);
            }
            if let Some(box_value) = &mut label.box_value {
                box_value[0] = round2(box_value[0] + delta_x);
                box_value[1] = round2(box_value[1] + delta_y);
                box_value[2] = round2(box_value[2] + delta_x);
                box_value[3] = round2(box_value[3] + delta_y);
            }
        }
    } else if let Some(bbox) = &mut label.box_value {
        if delta_x.abs() > EPSILON || delta_y.abs() > EPSILON {
            bbox[0] = round2(bbox[0] + delta_x);
            bbox[1] = round2(bbox[1] + delta_y);
            bbox[2] = round2(bbox[2] + delta_x);
            bbox[3] = round2(bbox[3] + delta_y);
            if let Some(position) = &mut label.position {
                position[0] = round2(position[0] + delta_x);
                position[1] = round2(position[1] + delta_y);
            }
        }
    }
}

fn imported_node_label_anchor_point(
    label: &NodeLabel,
    anchor_side: Option<ImportedLabelAnchorSide>,
) -> Option<[f64; 2]> {
    if label.glyph_polygons.is_empty() {
        return None;
    }
    if label.align.as_deref() == Some("center") && label.glyph_polygons.len() > 1 {
        let x = label
            .bbox()
            .map(|bbox| (bbox[0] + bbox[2]) * 0.5)
            .or_else(|| glyph_polygon_bounds(&label.glyph_polygons).map(|b| (b[0] + b[2]) * 0.5))?;
        let first_glyph_bounds = glyph_single_polygon_bounds(label.glyph_polygons.first()?)?;
        return Some([x, (first_glyph_bounds[1] + first_glyph_bounds[3]) * 0.5]);
    }
    if let Some(anchor_side) = anchor_side {
        return imported_node_label_side_anchor_point(label, anchor_side);
    }
    let polygon = if label.align.as_deref() == Some("right") {
        label.glyph_polygons.last()?
    } else {
        label.glyph_polygons.first()?
    };
    glyph_single_polygon_bounds(polygon)
        .map(|bounds| [(bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5])
}

fn imported_node_label_side_anchor_point(
    label: &NodeLabel,
    anchor_side: ImportedLabelAnchorSide,
) -> Option<[f64; 2]> {
    let characters: Vec<char> = label.text.chars().collect();
    let candidate = match anchor_side {
        ImportedLabelAnchorSide::Left => label
            .glyph_polygons
            .iter()
            .enumerate()
            .find(|(index, _)| {
                characters
                    .get(*index)
                    .is_some_and(|ch| ch.is_ascii_alphabetic())
            })
            .map(|(_, polygon)| polygon)
            .or_else(|| {
                label.glyph_polygons.iter().min_by(|left, right| {
                    glyph_polygon_center_x(left).total_cmp(&glyph_polygon_center_x(right))
                })
            }),
        ImportedLabelAnchorSide::Right => label
            .glyph_polygons
            .iter()
            .enumerate()
            .rev()
            .find(|(index, _)| {
                characters
                    .get(*index)
                    .is_some_and(|ch| ch.is_ascii_alphabetic())
            })
            .map(|(_, polygon)| polygon)
            .or_else(|| {
                label.glyph_polygons.iter().max_by(|left, right| {
                    glyph_polygon_center_x(left).total_cmp(&glyph_polygon_center_x(right))
                })
            }),
    }?;
    glyph_single_polygon_bounds(candidate)
        .map(|bounds| [(bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5])
}

fn glyph_polygon_center_x(polygon: &[[f64; 2]]) -> f64 {
    glyph_single_polygon_bounds(polygon)
        .map(|bounds| (bounds[0] + bounds[2]) * 0.5)
        .unwrap_or(0.0)
}

fn glyph_single_polygon_bounds(polygon: &[[f64; 2]]) -> Option<[f64; 4]> {
    let mut iter = polygon.iter();
    let first = iter.next()?;
    let mut min_x = first[0];
    let mut min_y = first[1];
    let mut max_x = first[0];
    let mut max_y = first[1];
    for point in iter {
        min_x = min_x.min(point[0]);
        min_y = min_y.min(point[1]);
        max_x = max_x.max(point[0]);
        max_y = max_y.max(point[1]);
    }
    Some([min_x, min_y, max_x, max_y])
}

fn glyph_polygon_bounds(polygons: &[Vec<[f64; 2]>]) -> Option<[f64; 4]> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;
    for polygon in polygons {
        for point in polygon {
            found = true;
            min_x = min_x.min(point[0]);
            min_y = min_y.min(point[1]);
            max_x = max_x.max(point[0]);
            max_y = max_y.max(point[1]);
        }
    }
    found.then_some([min_x, min_y, max_x, max_y])
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

fn ensure_document_json_pt_unit(value: &mut Value) -> Result<(), String> {
    if !value.is_object() {
        return Ok(());
    }
    let Some(format) = value.get_mut("format").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    if let Some(unit) = format.get("unit").and_then(Value::as_str) {
        if unit.eq_ignore_ascii_case("pt") {
            return Ok(());
        }
        return Err(format!(
            "Unsupported chemcore document unit '{unit}'. Current development files must use pt."
        ));
    }
    format.insert("unit".to_string(), Value::String("pt".to_string()));
    Ok(())
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
    Wavy,
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
        self.fragment.bbox = fragment_content_bbox(&self.fragment.nodes).unwrap_or([
            0.0,
            0.0,
            DEFAULT_PAGE_WIDTH,
            DEFAULT_PAGE_HEIGHT,
        ]);
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
        min_x = min_x.min(node.position[0] - DEFAULT_TEXT_BLOCK_PADDING_PT);
        min_y = min_y.min(node.position[1] - DEFAULT_TEXT_BLOCK_PADDING_PT);
        max_x = max_x.max(node.position[0] + DEFAULT_TEXT_BLOCK_PADDING_PT);
        max_y = max_y.max(node.position[1] + DEFAULT_TEXT_BLOCK_PADDING_PT);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon_bounds(polygon: &[[f64; 2]]) -> Option<[f64; 4]> {
        let mut iter = polygon.iter();
        let first = iter.next()?;
        let mut min_x = first[0];
        let mut min_y = first[1];
        let mut max_x = first[0];
        let mut max_y = first[1];
        for point in iter {
            min_x = min_x.min(point[0]);
            min_y = min_y.min(point[1]);
            max_x = max_x.max(point[0]);
            max_y = max_y.max(point[1]);
        }
        Some([min_x, min_y, max_x, max_y])
    }

    fn glyph_center(label: &NodeLabel, index: usize) -> Point {
        let bounds = polygon_bounds(
            label
                .glyph_polygons
                .get(index)
                .expect("glyph polygon should exist"),
        )
        .expect("glyph polygon should have bounds");
        Point::new((bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5)
    }

    #[test]
    fn imported_label_anchor_follows_horizontal_bond_side() {
        let document = parse_document_json(
            &json!({
                "format": { "name": "chemcore", "version": "0.1" },
                "document": {
                    "id": "doc_anchor_side",
                    "title": "anchor side",
                    "page": { "width": 80.0, "height": 40.0, "background": "#ffffff" }
                },
                "objects": [{
                    "id": "obj_molecule_001",
                    "type": "molecule",
                    "visible": true,
                    "zIndex": 10,
                    "payload": { "resourceRef": "mol_001" }
                }],
                "resources": {
                    "mol_001": {
                        "type": "molecule_fragment2d",
                        "encoding": "chemcore.molecule.fragment2d",
                        "data": {
                            "schema": "chemcore.molecule.fragment2d",
                            "bbox": [0.0, 0.0, 80.0, 40.0],
                            "nodes": [{
                                "id": "left_label",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [10.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": {
                                    "text": "Ph",
                                    "sourceText": "Ph",
                                    "position": [6.78, 13.63],
                                    "box": [6.78, 5.43, 19.08, 15.93],
                                    "runs": [{ "text": "Ph", "fontFamily": "Arial", "fontSize": 10.0 }],
                                    "align": "left",
                                    "anchor": "start",
                                    "attachment": "node",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "meta": { "import": { "cdxml": { "boundingBox": [6.78, 5.43, 19.08, 15.93] } } }
                                }
                            }, {
                                "id": "left_neighbor",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [24.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }, {
                                "id": "right_label",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [50.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0,
                                "label": {
                                    "text": "Ph",
                                    "sourceText": "Ph",
                                    "position": [46.78, 13.63],
                                    "box": [46.78, 5.43, 59.08, 15.93],
                                    "runs": [{ "text": "Ph", "fontFamily": "Arial", "fontSize": 10.0 }],
                                    "align": "left",
                                    "anchor": "start",
                                    "attachment": "node",
                                    "fontFamily": "Arial",
                                    "fontSize": 10.0,
                                    "meta": { "import": { "cdxml": { "boundingBox": [46.78, 5.43, 59.08, 15.93] } } }
                                }
                            }, {
                                "id": "right_neighbor",
                                "element": "C",
                                "atomicNumber": 6,
                                "position": [36.0, 10.0],
                                "charge": 0,
                                "numHydrogens": 0
                            }],
                            "bonds": [{
                                "id": "b_right",
                                "begin": "left_label",
                                "end": "left_neighbor",
                                "order": 1
                            }, {
                                "id": "b_left",
                                "begin": "right_label",
                                "end": "right_neighbor",
                                "order": 1
                            }]
                        }
                    }
                }
            })
            .to_string(),
        )
        .expect("document should parse");
        let fragment = document
            .resources
            .get("mol_001")
            .and_then(|resource| resource.data.as_fragment())
            .expect("fragment should exist");
        let left_label_node = fragment
            .nodes
            .iter()
            .find(|node| node.id == "left_label")
            .expect("left label node");
        let left_label = left_label_node.label.as_ref().expect("left label");
        let right_label_node = fragment
            .nodes
            .iter()
            .find(|node| node.id == "right_label")
            .expect("right label node");
        let right_label = right_label_node.label.as_ref().expect("right label");

        assert!(
            glyph_center(left_label, 1).distance(left_label_node.point()) < 0.01,
            "right-side bond should anchor Ph on h: node={left_label_node:?}, label={left_label:?}"
        );
        assert!(
            glyph_center(right_label, 0).distance(right_label_node.point()) < 0.01,
            "left-side bond should anchor Ph on P: node={right_label_node:?}, label={right_label:?}"
        );
    }

    #[test]
    fn parse_document_json_rebuilds_fragment_label_glyph_polygons() {
        let mut document = ChemcoreDocument::blank();
        document.resources.insert(
            "frag_1".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemcore.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment {
                    schema: "chemcore.molecule.fragment2d".to_string(),
                    bbox: [0.0, 0.0, 20.0, 20.0],
                    nodes: vec![Node {
                        id: "n1".to_string(),
                        element: "N".to_string(),
                        atomic_number: 7,
                        position: [10.0, 10.0],
                        charge: 0,
                        num_hydrogens: 0,
                        is_external_connection_point: false,
                        is_placeholder: false,
                        label: Some(NodeLabel {
                            text: "N".to_string(),
                            source_text: Some("N".to_string()),
                            position: Some([10.0, 10.0]),
                            box_field: None,
                            runs: vec![LabelRun {
                                text: "N".to_string(),
                                font_family: Some("Arial".to_string()),
                                font_size: Some(10.0),
                                fill: Some("#000000".to_string()),
                                font_weight: Some(400),
                                font_style: Some("normal".to_string()),
                                underline: None,
                                script: Some("normal".to_string()),
                            }],
                            line_runs: Vec::new(),
                            lines: Vec::new(),
                            align: Some("left".to_string()),
                            layout: None,
                            attachment: Some("node".to_string()),
                            anchor: Some("start".to_string()),
                            font_family: Some("Arial".to_string()),
                            fill: Some("#000000".to_string()),
                            font_size: Some(10.0),
                            glyph_polygons: vec![vec![
                                [0.0, 0.0],
                                [1.0, 0.0],
                                [1.0, 1.0],
                                [0.0, 1.0],
                            ]],
                            box_value: Some([10.0, 2.0, 17.2, 10.0]),
                            meta: json!({
                                "import": {
                                    "cdxml": {
                                        "boundingBox": [26.4, 24.95, 33.62, 36.45]
                                    }
                                }
                            }),
                        }),
                        meta: Value::Null,
                    }],
                    bonds: Vec::new(),
                    meta: Value::Null,
                }),
                meta: Value::Null,
            },
        );

        normalize_fragment_label_payloads(&mut document);

        let resource = document.resources.get("frag_1").expect("resource");
        let fragment = resource.data.as_fragment().expect("fragment");
        let label = fragment.nodes[0].label.as_ref().expect("label");

        assert_eq!(label.text, "N");
        assert_eq!(label.glyph_polygons.len(), 1);
        assert!(
            label.glyph_polygons[0].len() > 4,
            "stale glyph polygon should be rebuilt using current kernel geometry: {:?}",
            label.glyph_polygons[0]
        );
    }

    #[test]
    fn rebuild_left_aligned_label_glyph_polygons_uses_label_baseline() {
        let mut document = ChemcoreDocument::blank();
        document.resources.insert(
            "frag_1".to_string(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemcore.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(MoleculeFragment {
                    schema: "chemcore.molecule.fragment2d".to_string(),
                    bbox: [0.0, 0.0, 60.0, 60.0],
                    nodes: vec![Node {
                        id: "n1".to_string(),
                        element: "N".to_string(),
                        atomic_number: 7,
                        position: [30.0, 30.0],
                        charge: 0,
                        num_hydrogens: 0,
                        is_external_connection_point: false,
                        is_placeholder: false,
                        label: Some(NodeLabel {
                            text: "N".to_string(),
                            source_text: Some("N".to_string()),
                            position: Some([26.4, 33.9]),
                            box_field: Some([26.4, 24.95, 33.62, 36.45]),
                            runs: vec![LabelRun {
                                text: "N".to_string(),
                                font_family: Some("Arial".to_string()),
                                font_size: Some(10.0),
                                fill: Some("#000000".to_string()),
                                font_weight: Some(400),
                                font_style: Some("normal".to_string()),
                                underline: None,
                                script: Some("chemical".to_string()),
                            }],
                            line_runs: Vec::new(),
                            lines: Vec::new(),
                            align: Some("left".to_string()),
                            layout: None,
                            attachment: Some("node".to_string()),
                            anchor: Some("start".to_string()),
                            font_family: Some("Arial".to_string()),
                            fill: Some("#000000".to_string()),
                            font_size: Some(10.0),
                            glyph_polygons: Vec::new(),
                            box_value: None,
                            meta: json!({
                                "import": {
                                    "cdxml": {
                                        "boundingBox": [26.4, 24.95, 33.62, 36.45]
                                    }
                                }
                            }),
                        }),
                        meta: Value::Null,
                    }],
                    bonds: Vec::new(),
                    meta: Value::Null,
                }),
                meta: Value::Null,
            },
        );

        normalize_fragment_label_payloads(&mut document);

        let resource = document.resources.get("frag_1").expect("resource");
        let fragment = resource.data.as_fragment().expect("fragment");
        let label = fragment.nodes[0].label.as_ref().expect("label");
        let bounds = polygon_bounds(&label.glyph_polygons[0]).expect("bounds");
        let center_y = (bounds[1] + bounds[3]) * 0.5;

        assert!(
            (center_y - 30.0).abs() < 0.01,
            "single-glyph imported node labels should be vertically recentered onto the node position, got bounds={bounds:?}",
        );
    }
}
