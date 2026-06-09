use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{Engine, ObjectSettingsPatch, ACS_DOCUMENT_1996_PRESET, DEFAULT_DOCUMENT_STYLE_PRESET};
use crate::{
    render_document, render_primitives_bounds, Bond, ChemcoreDocument, EditorOptions,
    ObjectSettings, Point, SceneObject, WorldPt, DEFAULT_BOND_LENGTH,
};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::BTreeSet;

impl Engine {
    pub fn options(&self) -> &EditorOptions {
        &self.options
    }

    pub fn document_style_preset(&self) -> &str {
        &self.document_style_preset
    }

    pub fn set_bond_length_world_pt(&mut self, length: WorldPt) {
        self.options.bond_length = if length.value() > 0.0 {
            length.value()
        } else {
            DEFAULT_BOND_LENGTH
        };
    }

    pub fn set_bond_length(&mut self, length: f64) {
        self.set_bond_length_world_pt(WorldPt(length));
    }

    pub fn object_settings(&self) -> ObjectSettings {
        ObjectSettings::from(&self.options)
    }

    pub fn object_settings_dialog_json(&self) -> String {
        serde_json::to_string(&self.object_settings_dialog_payload("cm"))
            .unwrap_or_else(|_| "{}".to_string())
    }

    pub fn apply_object_settings_dialog_json(
        &mut self,
        settings_json: &str,
    ) -> Result<bool, String> {
        let value: JsonValue =
            serde_json::from_str(settings_json).map_err(|error| error.to_string())?;
        let unit = value
            .get("unit")
            .and_then(JsonValue::as_str)
            .unwrap_or("cm");
        let values = value
            .get("values")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| "Object settings payload must include values.".to_string())?;
        let settings = SelectedObjectSettings {
            bond_length: parse_optional_object_setting(values, "bondLength", unit, true)?,
            line_width: parse_optional_object_setting(values, "lineWidth", unit, true)?,
            bold_width: parse_optional_object_setting(values, "boldWidth", unit, true)?,
            bond_spacing: parse_optional_object_setting(values, "bondSpacing", unit, false)?,
            margin_width: parse_optional_object_setting(values, "marginWidth", unit, true)?,
            hash_spacing: parse_optional_object_setting(values, "hashSpacing", unit, true)?,
        };
        Ok(self.apply_object_settings_to_selection(settings))
    }

    pub fn apply_object_settings(&mut self, settings: ObjectSettings) -> bool {
        let Some(next_options) = object_settings_to_options(settings) else {
            return false;
        };
        if editor_options_match_visible_object_settings(&self.options, &next_options) {
            return false;
        }
        let command = super::command::EditorCommand::ApplyObjectSettings { settings };
        self.with_command(command, |engine| {
            engine.apply_object_settings_untracked(next_options)
        })
    }

    fn apply_object_settings_untracked(&mut self, next_options: EditorOptions) -> bool {
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
        update_document_object_settings_defaults(&mut self.state.document, &next_options);
        if let Some(mut entry) = self.state.document.editable_fragment_mut() {
            refresh_attached_node_label_geometry_for_all_nodes(
                entry.fragment,
                entry.object.transform.translate,
                next_options.bond_stroke_world_pt().value(),
            );
            entry.update_bounds();
        }
        self.options = next_options;
        self.document_style_preset = "custom".to_string();
        self.clear_interaction();
        true
    }

    pub fn has_object_settings_fields(&self) -> bool {
        !self.object_settings_fields("cm").is_empty()
    }

    fn object_settings_dialog_payload(&self, unit: &str) -> JsonValue {
        let unit = if unit == "pt" { "pt" } else { "cm" };
        serde_json::json!({
            "unit": unit,
            "units": ["cm", "pt"],
            "fields": self.object_settings_fields(unit),
        })
    }

    fn object_settings_fields(&self, unit: &str) -> Vec<JsonValue> {
        let mut fields = Vec::new();
        let selected_bonds = self.selected_object_settings_bonds();
        let selected_graphics = self.selected_object_settings_graphics();

        let bond_lengths = selected_bonds
            .iter()
            .filter_map(|bond| self.bond_length_value(bond))
            .collect::<Vec<_>>();
        if let Some(value) = object_setting_field_value(bond_lengths) {
            fields.push(object_settings_dialog_field(
                "bondLength",
                "Bond Length",
                value,
                unit,
            ));
        }

        let mut line_widths = selected_bonds
            .iter()
            .map(|bond| bond.stroke_width)
            .collect::<Vec<_>>();
        line_widths.extend(
            selected_graphics
                .iter()
                .filter_map(|object| self.graphic_stroke_width_value(object)),
        );
        if let Some(value) = object_setting_field_value(line_widths) {
            fields.push(object_settings_dialog_field(
                "lineWidth",
                "Line Width",
                value,
                unit,
            ));
        }

        let bold_widths = selected_bonds
            .iter()
            .filter(|bond| bond_uses_bold_width(bond))
            .map(|bond| bond.bold_width.unwrap_or(self.options.bold_bond_width))
            .collect::<Vec<_>>();
        if let Some(value) = object_setting_field_value(bold_widths) {
            fields.push(object_settings_dialog_field(
                "boldWidth",
                "Bold Width",
                value,
                unit,
            ));
        }

        let bond_spacings = selected_bonds
            .iter()
            .filter(|bond| bond.order >= 2)
            .map(|bond| bond.bond_spacing.unwrap_or(self.options.bond_spacing))
            .collect::<Vec<_>>();
        if let Some(value) = object_setting_field_value(bond_spacings) {
            fields.push(object_settings_dialog_field(
                "bondSpacing",
                "Double Spacing",
                value,
                "%",
            ));
        }

        let margin_widths = selected_bonds
            .iter()
            .map(|bond| bond.margin_width.unwrap_or(self.options.margin_width))
            .collect::<Vec<_>>();
        if let Some(value) = object_setting_field_value(margin_widths) {
            fields.push(object_settings_dialog_field(
                "marginWidth",
                "Margin Width",
                value,
                unit,
            ));
        }

        let hash_spacings = selected_bonds
            .iter()
            .filter(|bond| bond_uses_hash_spacing(bond))
            .map(|bond| bond.hash_spacing.unwrap_or(self.options.hash_spacing))
            .collect::<Vec<_>>();
        if let Some(value) = object_setting_field_value(hash_spacings) {
            fields.push(object_settings_dialog_field(
                "hashSpacing",
                "Hash Spacing",
                value,
                unit,
            ));
        }

        fields
    }

    pub(super) fn apply_object_settings_to_selection(
        &mut self,
        settings: SelectedObjectSettings,
    ) -> bool {
        if settings.is_empty() || self.state.selection.is_empty() {
            return false;
        }
        let bond_ids = self.state.selection.bonds.clone();
        let object_ids = self.state.selection.arrow_objects.clone();
        self.with_command(
            super::command::EditorCommand::ApplyObjectSettingsToSelection {
                bond_ids,
                object_ids,
                settings: settings.into(),
            },
            |engine| engine.apply_object_settings_to_selection_untracked(settings),
        )
    }

    fn apply_object_settings_to_selection_untracked(
        &mut self,
        settings: SelectedObjectSettings,
    ) -> bool {
        self.push_undo_snapshot();
        let mut changed = false;
        let selected_bonds: BTreeSet<String> = self.state.selection.bonds.iter().cloned().collect();
        let selected_graphics: BTreeSet<String> =
            self.state.selection.arrow_objects.iter().cloned().collect();

        let stroke_width = self.options.bond_stroke_world_pt().value();
        if !selected_bonds.is_empty() {
            if let Some(mut entry) = self.state.document.editable_fragment_mut() {
                let object_translate = entry.object.transform.translate;
                for bond_index in 0..entry.fragment.bonds.len() {
                    if !selected_bonds.contains(&entry.fragment.bonds[bond_index].id) {
                        continue;
                    }
                    changed |= apply_settings_to_bond(
                        entry.fragment,
                        bond_index,
                        &settings,
                        &self.options,
                    );
                }
                if changed {
                    refresh_attached_node_label_geometry_for_all_nodes(
                        entry.fragment,
                        object_translate,
                        stroke_width,
                    );
                    entry.update_bounds();
                }
            }
        }

        for object_id in selected_graphics {
            changed |= self.apply_object_settings_to_graphic(&object_id, &settings);
        }

        if !changed {
            self.undo_stack.pop();
            return false;
        }
        self.clear_interaction();
        true
    }

    fn apply_object_settings_to_graphic(
        &mut self,
        object_id: &str,
        settings: &SelectedObjectSettings,
    ) -> bool {
        let Some(line_width) = settings.line_width else {
            return false;
        };
        let Some(object) = self.state.document.find_scene_object(object_id) else {
            return false;
        };
        if !object_has_line_width_setting(object) {
            return false;
        }
        let style_id = format!("style_{object_id}_object_settings");
        let mut changed = false;
        if let Some(style_ref) = object.style_ref.as_ref() {
            let mut style = self
                .state
                .document
                .styles
                .get(style_ref)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "kind": object.object_type }));
            if set_json_number(&mut style, "strokeWidth", line_width) {
                self.state.document.styles.insert(style_id.clone(), style);
                if let Some(object) = self.state.document.find_scene_object_mut(object_id) {
                    changed |= object.style_ref.as_deref() != Some(style_id.as_str());
                    object.style_ref = Some(style_id);
                }
            }
        } else if let Some(object) = self.state.document.find_scene_object_mut(object_id) {
            changed |= set_payload_number(object, "strokeWidth", line_width);
        }
        changed
    }

    fn selected_object_settings_bonds(&self) -> Vec<&Bond> {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .bonds
            .iter()
            .map(String::as_str)
            .collect();
        self.state
            .document
            .editable_fragment()
            .map(|entry| {
                entry
                    .fragment
                    .bonds
                    .iter()
                    .filter(|bond| selected.contains(bond.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn selected_object_settings_graphics(&self) -> Vec<&SceneObject> {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .arrow_objects
            .iter()
            .map(String::as_str)
            .collect();
        self.state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| selected.contains(object.id.as_str()))
            .filter(|object| object_has_line_width_setting(object))
            .collect()
    }

    fn bond_length_value(&self, bond: &Bond) -> Option<f64> {
        let entry = self.state.document.editable_fragment()?;
        let begin = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.begin)?
            .point();
        let end = entry
            .fragment
            .nodes
            .iter()
            .find(|node| node.id == bond.end)?
            .point();
        Some(begin.distance(end))
    }

    fn graphic_stroke_width_value(&self, object: &SceneObject) -> Option<f64> {
        object
            .payload
            .extra
            .get("strokeWidth")
            .and_then(JsonValue::as_f64)
            .or_else(|| {
                object
                    .style_ref
                    .as_ref()
                    .and_then(|style_ref| self.state.document.styles.get(style_ref))
                    .and_then(|style| style.get("strokeWidth"))
                    .and_then(JsonValue::as_f64)
            })
    }

    pub fn set_document_style_preset(&mut self, preset: &str) -> bool {
        let preset = normalize_document_style_preset(preset).to_string();
        self.with_command(
            super::command::EditorCommand::ApplyDocumentStyle {
                preset: preset.clone(),
            },
            |engine| engine.set_document_style_preset_untracked(&preset),
        )
    }

    fn set_document_style_preset_untracked(&mut self, preset: &str) -> bool {
        let preset = normalize_document_style_preset(preset);
        let before_document_json = serde_json::to_string(&self.state.document).unwrap_or_default();
        let before_settings = self.object_settings();
        let before_preset = self.document_style_preset.clone();

        self.push_undo_snapshot();
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
        update_document_object_settings_defaults(&mut self.state.document, &next_options);
        if let Some(mut entry) = self.state.document.editable_fragment_mut() {
            refresh_attached_node_label_geometry_for_all_nodes(
                entry.fragment,
                entry.object.transform.translate,
                next_options.bond_stroke_world_pt().value(),
            );
            entry.update_bounds();
        }
        self.options = next_options;
        self.document_style_preset = preset.to_string();
        self.clear_interaction();
        let after_document_json = serde_json::to_string(&self.state.document).unwrap_or_default();
        let changed = before_document_json != after_document_json
            || before_settings != self.object_settings()
            || before_preset != self.document_style_preset;
        if !changed {
            self.undo_stack.pop();
        }
        changed
    }
}

fn object_settings_to_options(settings: ObjectSettings) -> Option<EditorOptions> {
    let bond_length = positive_or_none(settings.bond_length)?;
    let line_width = positive_or_none(settings.line_width)?;
    let bold_width = positive_or_none(settings.bold_width)?;
    let bond_spacing = positive_or_none(settings.bond_spacing)?;
    let margin_width = positive_or_none(settings.margin_width)?;
    let hash_spacing = positive_or_none(settings.hash_spacing)?;
    Some(EditorOptions {
        bond_length,
        bond_stroke_width: line_width,
        bold_bond_width: bold_width,
        wedge_width: derived_wedge_width(bold_width),
        label_clip_margin: derived_label_clip_margin(margin_width),
        hash_spacing,
        bond_spacing,
        margin_width,
        graphic_stroke_width: line_width,
    })
}

fn object_settings_dialog_field(
    key: &str,
    label: &str,
    value: ObjectSettingFieldValue,
    unit: &str,
) -> JsonValue {
    let display_value = value.value.map(|value| {
        if unit == "%" {
            value
        } else {
            display_length(value, unit)
        }
    });
    serde_json::json!({
        "key": key,
        "label": label,
        "value": display_value.map(round3),
        "values": {
            "cm": value.value.map(|value| if unit == "%" { round3(value) } else { round3(value / crate::PT_PER_CM) }),
            "pt": value.value.map(|value| if unit == "%" { round3(value) } else { round3(value) }),
        },
        "unit": unit,
        "mixed": value.mixed,
    })
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn display_length(value: f64, unit: &str) -> f64 {
    if unit == "pt" {
        value
    } else {
        value / crate::PT_PER_CM
    }
}

fn parse_object_setting(
    values: &JsonMap<String, JsonValue>,
    key: &str,
    unit: &str,
    is_length: bool,
) -> Result<f64, String> {
    let value = values
        .get(key)
        .and_then(JsonValue::as_f64)
        .ok_or_else(|| format!("{key} must be a number."))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{key} must be greater than 0."));
    }
    if is_length && unit != "pt" {
        Ok(value * crate::PT_PER_CM)
    } else {
        Ok(value)
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct SelectedObjectSettings {
    pub(super) bond_length: Option<f64>,
    pub(super) line_width: Option<f64>,
    pub(super) bold_width: Option<f64>,
    pub(super) bond_spacing: Option<f64>,
    pub(super) margin_width: Option<f64>,
    pub(super) hash_spacing: Option<f64>,
}

impl SelectedObjectSettings {
    fn is_empty(&self) -> bool {
        self.bond_length.is_none()
            && self.line_width.is_none()
            && self.bold_width.is_none()
            && self.bond_spacing.is_none()
            && self.margin_width.is_none()
            && self.hash_spacing.is_none()
    }
}

impl From<SelectedObjectSettings> for ObjectSettingsPatch {
    fn from(settings: SelectedObjectSettings) -> Self {
        Self {
            bond_length: settings.bond_length,
            line_width: settings.line_width,
            bold_width: settings.bold_width,
            bond_spacing: settings.bond_spacing,
            margin_width: settings.margin_width,
            hash_spacing: settings.hash_spacing,
        }
    }
}

fn parse_optional_object_setting(
    values: &JsonMap<String, JsonValue>,
    key: &str,
    unit: &str,
    is_length: bool,
) -> Result<Option<f64>, String> {
    if !values.contains_key(key) {
        return Ok(None);
    }
    parse_object_setting(values, key, unit, is_length).map(Some)
}

#[derive(Clone, Copy)]
struct ObjectSettingFieldValue {
    value: Option<f64>,
    mixed: bool,
}

fn object_setting_field_value(values: Vec<f64>) -> Option<ObjectSettingFieldValue> {
    let values = values
        .into_iter()
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect::<Vec<_>>();
    let first = *values.first()?;
    let mixed = values
        .iter()
        .any(|value| (*value - first).abs() > crate::EPSILON);
    Some(ObjectSettingFieldValue {
        value: (!mixed).then_some(first),
        mixed,
    })
}

fn bond_uses_bold_width(bond: &Bond) -> bool {
    bond.line_weights.main == crate::BondLineWeight::Bold
        || bond.line_weights.left == crate::BondLineWeight::Bold
        || bond.line_weights.right == crate::BondLineWeight::Bold
        || bond
            .stereo
            .as_ref()
            .is_some_and(|stereo| stereo.kind.contains("wedge"))
}

fn bond_uses_hash_spacing(bond: &Bond) -> bool {
    bond.meta
        .get("contextMenuBondStyle")
        .and_then(JsonValue::as_str)
        == Some("single-hashed")
        || bond
            .stereo
            .as_ref()
            .is_some_and(|stereo| stereo.kind.contains("hashed"))
}

fn object_has_line_width_setting(object: &SceneObject) -> bool {
    matches!(
        object.object_type.as_str(),
        "line" | "shape" | "bracket" | "symbol"
    )
}

fn apply_settings_to_bond(
    fragment: &mut crate::MoleculeFragment,
    bond_index: usize,
    settings: &SelectedObjectSettings,
    options: &EditorOptions,
) -> bool {
    let mut changed = false;
    if let Some(length) = settings.bond_length {
        changed |= set_bond_length(fragment, bond_index, length);
    }
    let bond = &mut fragment.bonds[bond_index];
    if let Some(value) = settings.line_width {
        if (bond.stroke_width - value).abs() > crate::EPSILON {
            bond.stroke_width = value;
            changed = true;
        }
    }
    if let Some(value) = settings.bold_width {
        if bond_uses_bold_width(bond) {
            changed |= set_bond_option_number(&mut bond.bold_width, value);
            changed |= set_bond_option_number(&mut bond.wedge_width, derived_wedge_width(value));
        }
    }
    if let Some(value) = settings.bond_spacing {
        if bond.order >= 2 {
            changed |= set_bond_option_number(&mut bond.bond_spacing, value);
        }
    }
    if let Some(value) = settings.margin_width {
        changed |= set_bond_option_number(&mut bond.margin_width, value);
        changed |= set_bond_option_number(
            &mut bond.label_clip_margin,
            derived_label_clip_margin(value),
        );
    }
    if let Some(value) = settings.hash_spacing {
        if bond_uses_hash_spacing(bond) {
            changed |= set_bond_option_number(&mut bond.hash_spacing, value);
        }
    }
    if bond.bold_width.is_none() && bond_uses_bold_width(bond) {
        changed |= set_bond_option_number(&mut bond.bold_width, options.bold_bond_width);
    }
    changed
}

fn set_bond_length(fragment: &mut crate::MoleculeFragment, bond_index: usize, length: f64) -> bool {
    let begin_id = fragment.bonds[bond_index].begin.clone();
    let end_id = fragment.bonds[bond_index].end.clone();
    let Some(begin_index) = fragment.nodes.iter().position(|node| node.id == begin_id) else {
        return false;
    };
    let Some(end_index) = fragment.nodes.iter().position(|node| node.id == end_id) else {
        return false;
    };
    let begin = fragment.nodes[begin_index].point();
    let end = fragment.nodes[end_index].point();
    let current = begin.distance(end);
    if current <= crate::EPSILON || (current - length).abs() <= crate::EPSILON {
        return false;
    }
    let center = Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
    let direction = Point::new((end.x - begin.x) / current, (end.y - begin.y) / current);
    let half = length * 0.5;
    fragment.nodes[begin_index].position = [
        crate::round2(center.x - direction.x * half),
        crate::round2(center.y - direction.y * half),
    ];
    fragment.nodes[end_index].position = [
        crate::round2(center.x + direction.x * half),
        crate::round2(center.y + direction.y * half),
    ];
    true
}

fn set_bond_option_number(slot: &mut Option<f64>, value: f64) -> bool {
    if slot.is_some_and(|current| (current - value).abs() <= crate::EPSILON) {
        return false;
    }
    *slot = Some(value);
    true
}

fn set_payload_number(object: &mut SceneObject, key: &str, value: f64) -> bool {
    if object
        .payload
        .extra
        .get(key)
        .and_then(JsonValue::as_f64)
        .is_some_and(|current| (current - value).abs() <= crate::EPSILON)
    {
        return false;
    }
    object
        .payload
        .extra
        .insert(key.to_string(), serde_json::json!(crate::round2(value)));
    true
}

fn set_json_number(value: &mut JsonValue, key: &str, number: f64) -> bool {
    let Some(object) = value.as_object_mut() else {
        return false;
    };
    if object
        .get(key)
        .and_then(JsonValue::as_f64)
        .is_some_and(|current| (current - number).abs() <= crate::EPSILON)
    {
        return false;
    }
    object.insert(key.to_string(), serde_json::json!(crate::round2(number)));
    true
}

fn positive_or_none(value: f64) -> Option<f64> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

fn editor_options_match_visible_object_settings(
    current: &EditorOptions,
    next: &EditorOptions,
) -> bool {
    (current.bond_length - next.bond_length).abs() <= crate::EPSILON
        && (current.bond_stroke_width - next.bond_stroke_width).abs() <= crate::EPSILON
        && (current.bold_bond_width - next.bold_bond_width).abs() <= crate::EPSILON
        && (current.hash_spacing - next.hash_spacing).abs() <= crate::EPSILON
        && (current.bond_spacing - next.bond_spacing).abs() <= crate::EPSILON
        && (current.margin_width - next.margin_width).abs() <= crate::EPSILON
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
            label_clip_margin: crate::ACS_LABEL_GEOMETRY_CLIP_MARGIN_PT.value(),
            hash_spacing: 2.5,
            bond_spacing: 18.0,
            margin_width: crate::ACS_BOND_MARGIN_WIDTH_PT.value(),
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
    options.label_clip_margin = if has_cdxml_defaults {
        0.0
    } else {
        derived_label_clip_margin(options.margin_width)
    };
    options
}

pub(super) fn editor_options_from_imported_cdxml_document(
    document: &ChemcoreDocument,
) -> EditorOptions {
    let mut options = editor_options_from_document(document);
    let editing_scale = document
        .document
        .meta
        .pointer("/import/cdxml/editingScale")
        .and_then(JsonValue::as_f64)
        .unwrap_or(1.0);
    if (editing_scale - 1.0).abs() > crate::EPSILON {
        options.bond_length *= editing_scale;
        options.bond_stroke_width *= editing_scale;
        options.bold_bond_width *= editing_scale;
        options.wedge_width *= editing_scale;
        options.label_clip_margin *= editing_scale;
        options.hash_spacing *= editing_scale;
        options.margin_width *= editing_scale;
        options.graphic_stroke_width *= editing_scale;
    }
    options
}

fn derived_wedge_width(bold_width: f64) -> f64 {
    (bold_width * 1.5).max(crate::DEFAULT_BOND_STROKE)
}

fn derived_label_clip_margin(margin_width: f64) -> f64 {
    (margin_width - crate::ACS_LABEL_GEOMETRY_CLIP_MARGIN_PT.value()).max(0.0)
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
                label.font_size = Some(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
                for run in &mut label.runs {
                    run.font_size = Some(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
                }
                for line in &mut label.line_runs {
                    for run in line {
                        run.font_size = Some(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
                    }
                }
            }
        }
        for bond in &mut fragment.bonds {
            bond.stroke_width = options.bond_stroke_world_pt().value();
            bond.bold_width = Some(options.bold_bond_width_world_pt().value());
            bond.wedge_width = Some(options.wedge_width_world_pt().value());
            bond.label_clip_margin = Some(options.label_clip_margin_world_pt().value());
            bond.hash_spacing = Some(options.hash_spacing_world_pt().value());
            bond.bond_spacing = Some(options.bond_spacing_percent());
            bond.margin_width = Some(options.margin_width_world_pt().value());
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
            "molecule" => Some(options.bond_stroke_world_pt().value()),
            "stroke" | "shape" => existing_style_has_stroke_width(object)
                .then_some(options.graphic_stroke_world_pt().value()),
            _ => None,
        };
        if let Some(width) = target_width {
            object.insert("strokeWidth".to_string(), json_number(width));
        }
        match kind.as_str() {
            "molecule" => {
                object.insert(
                    "fontSize".to_string(),
                    json_number(crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT),
                );
            }
            "text" => {
                object.insert(
                    "fontSize".to_string(),
                    json_number(crate::DEFAULT_TEXT_FONT_SIZE_PT),
                );
            }
            _ => {}
        }
    }
    let graphic_width = options.graphic_stroke_world_pt().value();
    for object in &mut document.objects {
        apply_graphic_stroke_width_to_object(object, graphic_width);
    }
}

fn existing_style_has_stroke_width(object: &serde_json::Map<String, JsonValue>) -> bool {
    object
        .get("strokeWidth")
        .and_then(JsonValue::as_f64)
        .is_some_and(|value| value > crate::EPSILON)
}

fn apply_graphic_stroke_width_to_object(object: &mut SceneObject, width: f64) {
    match object.object_type.as_str() {
        "bracket" | "symbol" => {
            update_positive_extra_number(&mut object.payload.extra, "strokeWidth", width);
            update_positive_extra_number(&mut object.payload.extra, "symbolLineWidth", width);
        }
        _ => {}
    }
    for child in &mut object.children {
        apply_graphic_stroke_width_to_object(child, width);
    }
}

fn update_positive_extra_number(
    object: &mut std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    value: f64,
) {
    if object
        .get(key)
        .and_then(JsonValue::as_f64)
        .is_some_and(|current| current > crate::EPSILON)
    {
        object.insert(key.to_string(), json_number(value));
    }
}

fn update_document_object_settings_defaults(
    document: &mut ChemcoreDocument,
    options: &EditorOptions,
) {
    let Some(meta) = document.document.meta.as_object_mut() else {
        document.document.meta = JsonValue::Object(JsonMap::new());
        return update_document_object_settings_defaults(document, options);
    };
    let import = meta
        .entry("import".to_string())
        .or_insert_with(|| JsonValue::Object(JsonMap::new()));
    if !import.is_object() {
        *import = JsonValue::Object(JsonMap::new());
    }
    let Some(import) = import.as_object_mut() else {
        return;
    };
    let cdxml = import
        .entry("cdxml".to_string())
        .or_insert_with(|| JsonValue::Object(JsonMap::new()));
    if !cdxml.is_object() {
        *cdxml = JsonValue::Object(JsonMap::new());
    }
    let Some(cdxml) = cdxml.as_object_mut() else {
        return;
    };
    let defaults = cdxml
        .entry("defaults".to_string())
        .or_insert_with(|| JsonValue::Object(JsonMap::new()));
    if !defaults.is_object() {
        *defaults = JsonValue::Object(JsonMap::new());
    }
    let Some(defaults) = defaults.as_object_mut() else {
        return;
    };
    defaults.insert(
        "bondLength".to_string(),
        json_number(options.bond_length_world_pt().value()),
    );
    defaults.insert(
        "lineWidth".to_string(),
        json_number(options.bond_stroke_world_pt().value()),
    );
    defaults.insert(
        "boldWidth".to_string(),
        json_number(options.bold_bond_width_world_pt().value()),
    );
    defaults.insert(
        "hashSpacing".to_string(),
        json_number(options.hash_spacing_world_pt().value()),
    );
    defaults.insert(
        "bondSpacing".to_string(),
        json_number(options.bond_spacing_percent()),
    );
    defaults.insert(
        "marginWidth".to_string(),
        json_number(options.margin_width_world_pt().value()),
    );
}
