use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{Engine, ACS_DOCUMENT_1996_PRESET, DEFAULT_DOCUMENT_STYLE_PRESET};
use crate::{
    render_document, render_primitives_bounds, ChemcoreDocument, EditorOptions, Point, WorldCm,
    DEFAULT_BOND_LENGTH,
};
use serde_json::Value as JsonValue;

impl Engine {
    pub fn options(&self) -> &EditorOptions {
        &self.options
    }

    pub fn document_style_preset(&self) -> &str {
        &self.document_style_preset
    }

    pub fn set_bond_length_world_cm(&mut self, length: WorldCm) {
        self.options.bond_length = if length.value() > 0.0 {
            length.value()
        } else {
            DEFAULT_BOND_LENGTH
        };
    }

    pub fn set_bond_length(&mut self, length: f64) {
        self.set_bond_length_world_cm(WorldCm(length));
    }

    pub fn set_document_style_preset(&mut self, preset: &str) {
        let preset = normalize_document_style_preset(preset);
        let next_options = document_style_preset_options(preset);
        let scale = if self.options.bond_length > crate::EPSILON {
            next_options.bond_length / self.options.bond_length
        } else {
            1.0
        };
        if (scale - 1.0).abs() > crate::EPSILON {
            if let Some(anchor) = document_content_center(&self.state.document) {
                scale_document_for_style_preset(&mut self.state.document, scale, anchor);
            }
        }
        apply_existing_document_style_preset(&mut self.state.document, &next_options);
        if let Some(mut entry) = self.state.document.editable_fragment_mut() {
            refresh_attached_node_label_geometry_for_all_nodes(
                entry.fragment,
                entry.object.transform.translate,
                next_options.bond_stroke_world_cm().value(),
            );
            entry.update_bounds();
        }
        self.options = next_options;
        self.document_style_preset = preset.to_string();
        self.clear_interaction();
    }
}

fn normalize_document_style_preset(preset: &str) -> &'static str {
    match preset {
        ACS_DOCUMENT_1996_PRESET => ACS_DOCUMENT_1996_PRESET,
        _ => DEFAULT_DOCUMENT_STYLE_PRESET,
    }
}

fn document_style_preset_options(preset: &str) -> EditorOptions {
    match normalize_document_style_preset(preset) {
        ACS_DOCUMENT_1996_PRESET => EditorOptions {
            bond_length: 14.4,
            bond_stroke_width: 0.6,
            bold_bond_width: 2.0,
            wedge_width: 3.0,
            label_clip_margin: crate::ACS_LABEL_GEOMETRY_CLIP_MARGIN_CM.value(),
            hash_spacing: 2.5,
            bond_spacing: 18.0,
            graphic_stroke_width: 0.6,
        },
        _ => EditorOptions::default(),
    }
}

pub(super) fn editor_options_from_document(document: &ChemcoreDocument) -> EditorOptions {
    let mut options = EditorOptions::default();
    let mut has_cdxml_defaults = false;
    let mut has_bond_length = false;
    let mut has_line_width = false;
    let mut has_bold_width = false;
    let mut has_hash_spacing = false;
    let mut has_bond_spacing = false;
    if let Some(defaults) = document
        .document
        .meta
        .get("import")
        .and_then(|value| value.get("cdxml"))
        .and_then(|value| value.get("defaults"))
    {
        has_cdxml_defaults = true;
        if let Some(value) = defaults.get("bondLength").and_then(JsonValue::as_f64) {
            options.bond_length = value;
            has_bond_length = true;
        }
        if let Some(value) = defaults.get("lineWidth").and_then(JsonValue::as_f64) {
            options.bond_stroke_width = value;
            options.graphic_stroke_width = value;
            has_line_width = true;
        }
        if let Some(value) = defaults.get("boldWidth").and_then(JsonValue::as_f64) {
            options.bold_bond_width = value;
            has_bold_width = true;
        }
        if let Some(value) = defaults.get("hashSpacing").and_then(JsonValue::as_f64) {
            options.hash_spacing = value;
            has_hash_spacing = true;
        }
        if let Some(value) = defaults.get("bondSpacing").and_then(JsonValue::as_f64) {
            options.bond_spacing = value;
            has_bond_spacing = true;
        }
    }
    if has_cdxml_defaults {
        if let Some(metrics) = infer_cdxml_document_bond_metrics(document) {
            if !has_bond_length {
                options.bond_length = metrics.bond_length.unwrap_or(options.bond_length);
            }
            if !has_line_width {
                options.bond_stroke_width = metrics.line_width.unwrap_or(options.bond_stroke_width);
                options.graphic_stroke_width =
                    metrics.line_width.unwrap_or(options.graphic_stroke_width);
            }
            if !has_bold_width {
                options.bold_bond_width = metrics.bold_width.unwrap_or(options.bold_bond_width);
            }
            if !has_hash_spacing {
                options.hash_spacing = metrics.hash_spacing.unwrap_or(options.hash_spacing);
            }
            if !has_bond_spacing {
                options.bond_spacing = metrics.bond_spacing.unwrap_or(options.bond_spacing);
            }
        }
    }
    options.wedge_width = derived_wedge_width(options.bold_bond_width);
    options.label_clip_margin = derived_label_clip_margin(options.bold_bond_width);
    options
}

pub(super) fn editor_options_from_imported_cdxml_document(
    document: &ChemcoreDocument,
) -> EditorOptions {
    let mut options = editor_options_from_document(document);
    let acs = document_style_preset_options(ACS_DOCUMENT_1996_PRESET);
    if (options.bond_length - acs.bond_length).abs() <= 0.05
        && (options.bold_bond_width - acs.bold_bond_width).abs() <= 0.05
        && (options.hash_spacing - acs.hash_spacing).abs() <= 0.05
        && (options.bond_spacing - acs.bond_spacing).abs() <= 0.05
    {
        options.label_clip_margin = acs.label_clip_margin;
    }
    options
}

fn derived_wedge_width(bold_width: f64) -> f64 {
    (bold_width * 1.5).max(crate::DEFAULT_BOND_STROKE)
}

fn derived_label_clip_margin(bold_width: f64) -> f64 {
    if (bold_width - 2.0).abs() <= 0.1 {
        crate::ACS_LABEL_GEOMETRY_CLIP_MARGIN_CM.value()
    } else {
        crate::LABEL_GEOMETRY_CLIP_MARGIN_CM.value()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct InferredBondMetrics {
    bond_length: Option<f64>,
    line_width: Option<f64>,
    bold_width: Option<f64>,
    hash_spacing: Option<f64>,
    bond_spacing: Option<f64>,
}

fn infer_cdxml_document_bond_metrics(document: &ChemcoreDocument) -> Option<InferredBondMetrics> {
    let entry = document.editable_fragment()?;
    let mut lengths = Vec::new();
    let mut line_widths = Vec::new();
    let mut bold_widths = Vec::new();
    let mut hash_spacings = Vec::new();
    let mut bond_spacings = Vec::new();
    for bond in &entry.fragment.bonds {
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
        let length = entry
            .world_point_for_node(begin)
            .distance(entry.world_point_for_node(end));
        if length > crate::EPSILON {
            lengths.push(length);
        }
        if bond.stroke_width > crate::EPSILON {
            line_widths.push(bond.stroke_width);
        }
        if let Some(value) = bond.bold_width.filter(|value| *value > crate::EPSILON) {
            bold_widths.push(value);
        }
        if let Some(value) = bond.hash_spacing.filter(|value| *value > crate::EPSILON) {
            hash_spacings.push(value);
        }
        if let Some(value) = bond.bond_spacing.filter(|value| *value > crate::EPSILON) {
            bond_spacings.push(value);
        }
    }
    Some(InferredBondMetrics {
        bond_length: median_near_default(&mut lengths),
        line_width: median_near_default(&mut line_widths),
        bold_width: median_near_default(&mut bold_widths),
        hash_spacing: median_near_default(&mut hash_spacings),
        bond_spacing: median_near_default(&mut bond_spacings),
    })
}

fn median_near_default(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    Some(crate::round2(values[values.len() / 2]))
}

fn document_content_center(document: &ChemcoreDocument) -> Option<Point> {
    let primitives = render_document(document);
    let bounds = render_primitives_bounds(primitives.iter());
    bounds.map(|[min_x, min_y, max_x, max_y]| {
        Point::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
    })
}

fn scale_document_for_style_preset(document: &mut ChemcoreDocument, factor: f64, anchor: Point) {
    let page_width = document.document.page.width;
    let page_height = document.document.page.height;
    let Ok(mut value) = serde_json::to_value(&*document) else {
        return;
    };
    scale_document_json_for_style_preset(&mut value, factor, anchor);
    if let Ok(mut next_document) = serde_json::from_value::<ChemcoreDocument>(value) {
        next_document.document.page.width = page_width;
        next_document.document.page.height = page_height;
        *document = next_document;
    }
}

fn scale_document_json_for_style_preset(value: &mut JsonValue, factor: f64, anchor: Point) {
    scale_json_value_for_style_preset("", value, factor, anchor);
}

fn scale_json_value_for_style_preset(key: &str, value: &mut JsonValue, factor: f64, anchor: Point) {
    if key == "translate" {
        scale_point_array_around_anchor(value, factor, anchor);
        return;
    }
    if style_scale_key_as_length_scalar(key) {
        scale_all_json_numbers(value, factor);
        return;
    }
    match value {
        JsonValue::Array(items) if style_scale_key_as_local_length_array(key) => {
            for item in items {
                scale_all_json_numbers(item, factor);
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                scale_json_value_for_style_preset("", item, factor, anchor);
            }
        }
        JsonValue::Object(object) => {
            for (child_key, child_value) in object {
                scale_json_value_for_style_preset(child_key, child_value, factor, anchor);
            }
        }
        _ => {}
    }
}

fn scale_point_array_around_anchor(value: &mut JsonValue, factor: f64, anchor: Point) {
    let Some(items) = value.as_array_mut() else {
        return;
    };
    if items.len() < 2 {
        return;
    }
    if let Some(x) = items.first().and_then(JsonValue::as_f64) {
        items[0] = json_number(anchor.x + (x - anchor.x) * factor);
    }
    if let Some(y) = items.get(1).and_then(JsonValue::as_f64) {
        items[1] = json_number(anchor.y + (y - anchor.y) * factor);
    }
}

fn scale_all_json_numbers(value: &mut JsonValue, factor: f64) {
    match value {
        JsonValue::Number(number) => {
            if let Some(scaled) = number
                .as_f64()
                .and_then(|value| serde_json::Number::from_f64(value * factor))
            {
                *number = scaled;
            }
        }
        JsonValue::Array(items) => {
            for item in items {
                scale_all_json_numbers(item, factor);
            }
        }
        JsonValue::Object(object) => {
            for child_value in object.values_mut() {
                scale_all_json_numbers(child_value, factor);
            }
        }
        _ => {}
    }
}

fn style_scale_key_as_length_scalar(key: &str) -> bool {
    matches!(
        key,
        "width"
            | "height"
            | "x"
            | "y"
            | "strokeWidth"
            | "boldWidth"
            | "hashSpacing"
            | "wrapWidth"
            | "pad"
            | "padding"
            | "length"
            | "centerLength"
            | "cornerRadius"
            | "shadowSize"
    )
}

fn style_scale_key_as_local_length_array(key: &str) -> bool {
    matches!(
        key,
        "bbox"
            | "box"
            | "boxField"
            | "position"
            | "points"
            | "anchorOffset"
            | "glyphPolygons"
            | "center"
            | "majorAxisEnd"
            | "minorAxisEnd"
            | "dashArray"
    )
}

fn json_number(value: f64) -> JsonValue {
    serde_json::Number::from_f64(value)
        .map(JsonValue::Number)
        .unwrap_or(JsonValue::Null)
}

fn apply_existing_document_style_preset(document: &mut ChemcoreDocument, options: &EditorOptions) {
    for resource in document.resources.values_mut() {
        let Some(fragment) = resource.data.as_fragment_mut() else {
            continue;
        };
        for node in &mut fragment.nodes {
            if let Some(label) = node.label.as_mut() {
                label.font_size = Some(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM);
                for run in &mut label.runs {
                    run.font_size = Some(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM);
                }
                for line in &mut label.line_runs {
                    for run in line {
                        run.font_size = Some(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM);
                    }
                }
            }
        }
        for bond in &mut fragment.bonds {
            bond.stroke_width = options.bond_stroke_world_cm().value();
            bond.bold_width = Some(options.bold_bond_width_world_cm().value());
            bond.wedge_width = Some(options.wedge_width_world_cm().value());
            bond.label_clip_margin = Some(options.label_clip_margin_world_cm().value());
            bond.hash_spacing = Some(options.hash_spacing_world_cm().value());
            bond.bond_spacing = Some(options.bond_spacing_percent());
        }
    }
    for style in document.styles.values_mut() {
        let Some(object) = style.as_object_mut() else {
            continue;
        };
        let kind = object
            .get("kind")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string();
        let target_width = match kind.as_str() {
            "molecule" => Some(options.bond_stroke_world_cm().value()),
            "stroke" | "shape" => existing_style_has_stroke_width(object)
                .then_some(options.graphic_stroke_world_cm().value()),
            _ => None,
        };
        if let Some(width) = target_width {
            object.insert("strokeWidth".to_string(), json_number(width));
        }
        match kind.as_str() {
            "molecule" => {
                object.insert(
                    "fontSize".to_string(),
                    json_number(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM),
                );
            }
            "text" => {
                object.insert(
                    "fontSize".to_string(),
                    json_number(crate::DEFAULT_TEXT_FONT_SIZE_CM),
                );
            }
            _ => {}
        }
    }
}

fn existing_style_has_stroke_width(object: &serde_json::Map<String, JsonValue>) -> bool {
    object
        .get("strokeWidth")
        .and_then(JsonValue::as_f64)
        .is_some_and(|value| value > crate::EPSILON)
}
