use super::*;
use crate::{
    Bond, BondLinePattern, BondLineWeight, DoubleBondPlacement, LabelRun, Node, NodeLabel,
    SceneObject,
};
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeSet;

impl Engine {
    pub fn context_menu_json(&self, hit_json: &str, has_paste: bool) -> String {
        let hit: JsonValue =
            serde_json::from_str(hit_json).unwrap_or_else(|_| json!({ "kind": "canvas" }));
        serde_json::to_string(&self.context_menu_items(&hit, has_paste))
            .unwrap_or_else(|_| "[]".to_string())
    }

    pub fn selection_numeric_dialog_json(&self, kind: &str) -> String {
        let payload = match kind {
            "scale" => json!({
                "kind": "scale",
                "title": "Scale",
                "field": { "key": "percent", "label": "Scale", "value": 100.0, "unit": "%" }
            }),
            "rotate" => json!({
                "kind": "rotate",
                "title": "Rotate",
                "field": { "key": "degrees", "label": "Angle", "value": 90.0, "unit": "degrees" }
            }),
            "line-height" => json!({
                "kind": "line-height",
                "title": "Line Spacing",
                "field": { "key": "lineHeight", "label": "Line Spacing", "value": self.selected_uniform_text_line_height().unwrap_or(12.0), "unit": "pt" }
            }),
            _ => json!({ "kind": "", "title": "", "field": null }),
        };
        payload.to_string()
    }

    pub fn apply_selection_numeric_dialog_json(
        &mut self,
        payload_json: &str,
    ) -> Result<bool, String> {
        let payload: JsonValue =
            serde_json::from_str(payload_json).map_err(|error| error.to_string())?;
        let kind = payload
            .get("kind")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        let value = payload
            .get("value")
            .and_then(JsonValue::as_f64)
            .ok_or_else(|| "Dialog value must be a number.".to_string())?;
        if !value.is_finite() {
            return Err("Dialog value must be finite.".to_string());
        }
        Ok(match kind {
            "scale" if value > 0.0 => self.scale_selection(value),
            "rotate" => self.rotate_selection_degrees(value),
            "line-height" if value > 0.0 => {
                self.apply_text_style_to_selection("line-height", &value.to_string())
            }
            "scale" | "line-height" => {
                return Err("Dialog value must be greater than 0.".to_string());
            }
            _ => false,
        })
    }

    fn context_menu_items(&self, hit: &JsonValue, has_paste: bool) -> Vec<JsonValue> {
        let hit_kind = hit
            .get("kind")
            .and_then(JsonValue::as_str)
            .unwrap_or("canvas");
        if hit_kind == "canvas" {
            return self.clipboard_items(true, has_paste);
        }

        let selected_count = self.context_selection_count();
        let selected_types = self.selected_object_types();
        let single_object_type = self.single_selected_object_type();
        let mut items = self.clipboard_items(false, has_paste);

        if selected_count > 1 || selected_types.contains("group") {
            items.extend([
                separator(),
                item("Bring Forward", "order", "bring-forward"),
                item("Send Backward", "order", "send-backward"),
                item("Bring to Front", "order", "bring-front"),
                item("Send to Back", "order", "send-back"),
                separator(),
                item("Flip Horizontal", "arrange", "flip-h"),
                item("Flip Vertical", "arrange", "flip-v"),
                item("Rotate...", "rotate-dialog", ""),
                item("Scale...", "scale-dialog", ""),
                separator(),
                self.color_menu(),
                group_menu(
                    selected_types.contains("group"),
                    self.selected_scene_object_count(),
                ),
                separator(),
                self.object_settings_item(),
            ]);
            return items;
        }

        if hit_kind == "bond" || !self.state.selection.bonds.is_empty() {
            items.extend([
                separator(),
                self.bond_type_menu(),
                separator(),
                self.color_menu(),
                self.object_settings_item(),
            ]);
            return items;
        }

        if hit_kind == "atom"
            || hit_kind == "label"
            || !self.state.selection.nodes.is_empty()
            || !self.state.selection.label_nodes.is_empty()
        {
            items.extend([
                separator(),
                item("Edit Label", "edit-text", ""),
                json!({"label": "Expand Label", "command": "expand-label", "disabled": !self.selected_can_expand_label()}),
            ]);
            if self.selected_text_target_count() > 0 {
                items.extend([
                    separator(),
                    self.text_font_menu(),
                    self.text_style_menu(),
                    self.text_size_menu(),
                    self.text_alignment_menu(),
                ]);
            }
            items.extend([
                separator(),
                json!({"label": "Chemical Check", "command": "chemical-check", "value": if self.selected_chemical_check_enabled() { "off" } else { "on" }, "checked": self.selected_chemical_check_enabled()}),
                separator(),
                self.color_menu(),
                self.object_settings_item(),
            ]);
            return items;
        }

        match single_object_type.as_deref() {
            Some("line") => {
                items.extend([
                    separator(),
                    self.line_style_menu(),
                    separator(),
                    self.arrowheads_menu(),
                    separator(),
                    order_subitems_flat(),
                    separator(),
                    transform_subitems_flat(true),
                    separator(),
                    self.color_menu(),
                    self.object_settings_item(),
                ]);
            }
            Some("shape") => {
                let is_orbital = self
                    .selected_scene_objects()
                    .first()
                    .and_then(|object| payload_string(object, "kind"))
                    .is_some_and(|kind| kind == "orbital");
                items.push(separator());
                if is_orbital {
                    items.push(self.orbital_template_menu());
                    items.push(self.orbital_style_menu());
                    items.push(self.orbital_phase_menu());
                } else {
                    items.push(self.shape_style_menu());
                }
                items.extend([
                    separator(),
                    order_subitems_flat(),
                    json!({"label": "Center on Page", "command": "center-page"}),
                    separator(),
                    transform_subitems_flat(true),
                    separator(),
                    self.color_menu(),
                    self.object_settings_item(),
                ]);
            }
            Some("bracket") => {
                items.extend([
                    separator(),
                    self.bracket_type_menu(),
                    separator(),
                    order_subitems_flat(),
                    separator(),
                    transform_subitems_flat(true),
                    separator(),
                    self.color_menu(),
                    self.object_settings_item(),
                ]);
            }
            Some("symbol") => {
                items.extend([
                    separator(),
                    order_subitems_flat(),
                    json!({"label": "Center on Page", "command": "center-page"}),
                    separator(),
                    transform_subitems_flat(true),
                    separator(),
                    self.color_menu(),
                    self.object_settings_item(),
                ]);
            }
            Some("text") => {
                items.extend([
                    separator(),
                    item("Edit Text", "edit-text", ""),
                    separator(),
                    self.text_font_menu(),
                    self.text_style_menu(),
                    self.text_size_menu(),
                    self.text_alignment_menu(),
                    item("Line Spacing...", "text-line-spacing", ""),
                    separator(),
                    order_subitems_flat(),
                    json!({"label": "Center on Page", "command": "center-page"}),
                    separator(),
                    self.color_menu(),
                    self.object_settings_item(),
                ]);
            }
            _ => {
                items.extend([
                    separator(),
                    order_subitems_flat(),
                    separator(),
                    self.color_menu(),
                    self.object_settings_item(),
                ]);
            }
        }
        items
    }

    fn clipboard_items(&self, include_select_all: bool, has_paste: bool) -> Vec<JsonValue> {
        let has_selection = !self.state.selection.is_empty();
        let mut items = vec![
            json!({"label": "Cut", "command": "cut", "shortcut": "Ctrl+X", "disabled": !has_selection}),
            json!({"label": "Copy", "command": "copy", "shortcut": "Ctrl+C", "disabled": !has_selection}),
            json!({"label": "Paste", "command": "paste", "shortcut": "Ctrl+V", "disabled": !has_paste}),
        ];
        if include_select_all {
            items.push(json!({"label": "Select All", "command": "select-all", "shortcut": "Ctrl+A", "disabled": !self.document_has_selectable_content()}));
        } else {
            items.push(json!({"label": "Delete", "command": "delete", "disabled": !has_selection}));
        }
        items
    }

    fn context_selection_count(&self) -> usize {
        self.selected_scene_object_count()
            + self.state.selection.nodes.len()
            + self.state.selection.bonds.len()
            + self.state.selection.label_nodes.len()
    }

    fn selected_scene_object_count(&self) -> usize {
        self.state.selection.text_objects.len() + self.state.selection.arrow_objects.len()
    }

    fn selected_object_types(&self) -> BTreeSet<&str> {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .text_objects
            .iter()
            .chain(self.state.selection.arrow_objects.iter())
            .map(String::as_str)
            .collect();
        self.state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| selected.contains(object.id.as_str()))
            .map(|object| object.object_type.as_str())
            .collect()
    }

    fn single_selected_object_type(&self) -> Option<String> {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .text_objects
            .iter()
            .chain(self.state.selection.arrow_objects.iter())
            .map(String::as_str)
            .collect();
        if selected.len() != 1 {
            return None;
        }
        self.state
            .document
            .scene_objects()
            .into_iter()
            .find(|object| selected.contains(object.id.as_str()))
            .map(|object| object.object_type.clone())
    }

    fn document_has_selectable_content(&self) -> bool {
        self.state
            .document
            .objects
            .iter()
            .any(|object| object.visible)
            || self
                .state
                .document
                .editable_fragment()
                .is_some_and(|entry| {
                    !entry.fragment.nodes.is_empty() || !entry.fragment.bonds.is_empty()
                })
    }

    fn selected_can_expand_label(&self) -> bool {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .nodes
            .iter()
            .chain(self.state.selection.label_nodes.iter())
            .map(String::as_str)
            .collect();
        self.state
            .document
            .editable_fragment()
            .is_some_and(|entry| {
                entry.fragment.nodes.iter().any(|node| {
                    selected.contains(node.id.as_str())
                        && label_recognition_status(node) == Some("recognized")
                        && label_recognition_expansion_complete(node) != Some(false)
                })
            })
    }

    fn selected_chemical_check_enabled(&self) -> bool {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .nodes
            .iter()
            .chain(self.state.selection.label_nodes.iter())
            .map(String::as_str)
            .collect();
        self.state
            .document
            .editable_fragment()
            .map_or(true, |entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .filter(|node| selected.contains(node.id.as_str()))
                    .all(|node| {
                        node.meta.get("chemicalCheck").and_then(JsonValue::as_bool) != Some(false)
                            && node
                                .label
                                .as_ref()
                                .and_then(|label| label.meta.get("chemicalCheck"))
                                .and_then(JsonValue::as_bool)
                                != Some(false)
                    })
            })
    }

    fn color_menu(&self) -> JsonValue {
        color_menu(self.selected_uniform_color().as_deref())
    }

    fn shape_style_menu(&self) -> JsonValue {
        shape_style_menu(self.selected_uniform_shape_style().as_deref())
    }

    fn bracket_type_menu(&self) -> JsonValue {
        bracket_type_menu(self.selected_uniform_bracket_kind().as_deref())
    }

    fn line_style_menu(&self) -> JsonValue {
        line_style_menu(self.selected_uniform_line_style().as_deref())
    }

    fn arrowheads_menu(&self) -> JsonValue {
        arrowheads_menu(
            self.selected_uniform_arrow_endpoint("head").as_deref(),
            self.selected_uniform_arrow_endpoint("tail").as_deref(),
        )
    }

    fn bond_type_menu(&self) -> JsonValue {
        bond_type_menu(self.selected_uniform_bond_style().as_deref())
    }

    fn orbital_template_menu(&self) -> JsonValue {
        orbital_template_menu(self.selected_uniform_orbital_template().as_deref())
    }

    fn orbital_style_menu(&self) -> JsonValue {
        orbital_style_menu(self.selected_uniform_orbital_style().as_deref())
    }

    fn orbital_phase_menu(&self) -> JsonValue {
        orbital_phase_menu(self.selected_uniform_orbital_phase().as_deref())
    }

    fn text_font_menu(&self) -> JsonValue {
        text_font_menu(self.selected_uniform_text_font_family().as_deref())
    }

    fn text_size_menu(&self) -> JsonValue {
        text_size_menu(self.selected_uniform_text_font_size())
    }

    fn text_style_menu(&self) -> JsonValue {
        text_style_menu(self.selected_text_style_state())
    }

    fn text_alignment_menu(&self) -> JsonValue {
        text_alignment_menu(self.selected_uniform_text_align().as_deref())
    }

    fn object_settings_item(&self) -> JsonValue {
        json!({
            "label": "Object Settings...",
            "command": "object-settings",
            "disabled": !self.has_object_settings_fields(),
        })
    }

    fn selected_scene_objects(&self) -> Vec<&SceneObject> {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .text_objects
            .iter()
            .chain(self.state.selection.arrow_objects.iter())
            .map(String::as_str)
            .collect();
        self.state
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| selected.contains(object.id.as_str()))
            .collect()
    }

    fn selected_bonds(&self) -> Vec<&Bond> {
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

    fn selected_label_nodes(&self) -> Vec<&Node> {
        let selected: BTreeSet<&str> = self
            .state
            .selection
            .nodes
            .iter()
            .chain(self.state.selection.label_nodes.iter())
            .map(String::as_str)
            .collect();
        self.state
            .document
            .editable_fragment()
            .map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .filter(|node| selected.contains(node.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn selected_text_objects(&self) -> Vec<&SceneObject> {
        self.selected_scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "text")
            .collect()
    }

    fn selected_text_target_count(&self) -> usize {
        self.selected_text_objects().len()
            + self
                .selected_label_nodes()
                .into_iter()
                .filter(|node| node.label.is_some())
                .count()
    }

    fn selected_uniform_color(&self) -> Option<String> {
        let mut colors = Vec::new();
        for object in self.selected_scene_objects() {
            colors.push(style_color_for_object(&self.state.document, object));
        }
        for bond in self.selected_bonds() {
            colors.push(css_color_to_hex(
                bond.stroke.as_deref().unwrap_or("#000000"),
            ));
        }
        for node in self.selected_label_nodes() {
            if let Some(label) = &node.label {
                colors.push(css_color_to_hex(label.fill.as_deref().unwrap_or("#000000")));
            }
        }
        uniform_value(colors)
    }

    fn selected_uniform_shape_style(&self) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "shape")
                .map(|object| shape_style_for_object(&self.state.document, object))
                .collect(),
        )
    }

    fn selected_uniform_bracket_kind(&self) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "bracket")
                .map(|object| {
                    object
                        .payload
                        .extra
                        .get("kind")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("round")
                        .to_string()
                })
                .collect(),
        )
    }

    fn selected_uniform_line_style(&self) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "line")
                .map(|object| line_object_style(&self.state.document, object))
                .collect(),
        )
    }

    fn selected_uniform_arrow_endpoint(&self, endpoint: &str) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "line")
                .map(|object| {
                    object
                        .payload
                        .extra
                        .get("arrowHead")
                        .and_then(|arrow| arrow.get(endpoint))
                        .and_then(JsonValue::as_str)
                        .unwrap_or("none")
                        .to_string()
                })
                .collect(),
        )
    }

    fn selected_uniform_bond_style(&self) -> Option<String> {
        uniform_value(
            self.selected_bonds()
                .into_iter()
                .map(bond_style_key)
                .collect(),
        )
    }

    fn selected_uniform_orbital_template(&self) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "shape")
                .filter(|object| payload_string(object, "kind").as_deref() == Some("orbital"))
                .map(|object| {
                    payload_string(object, "orbitalTemplate").unwrap_or_else(|| "s".to_string())
                })
                .collect(),
        )
    }

    fn selected_uniform_orbital_style(&self) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "shape")
                .filter(|object| payload_string(object, "kind").as_deref() == Some("orbital"))
                .map(|object| {
                    payload_string(object, "orbitalStyle").unwrap_or_else(|| "hollow".to_string())
                })
                .collect(),
        )
    }

    fn selected_uniform_orbital_phase(&self) -> Option<String> {
        uniform_value(
            self.selected_scene_objects()
                .into_iter()
                .filter(|object| object.object_type == "shape")
                .filter(|object| payload_string(object, "kind").as_deref() == Some("orbital"))
                .map(|object| {
                    payload_string(object, "orbitalPhase").unwrap_or_else(|| "plus".to_string())
                })
                .collect(),
        )
    }

    fn selected_uniform_text_font_family(&self) -> Option<String> {
        let mut values = Vec::new();
        for object in self.selected_text_objects() {
            values.push(text_object_font_family(object));
        }
        for node in self.selected_label_nodes() {
            if let Some(label) = &node.label {
                values.push(label_font_family(label));
            }
        }
        uniform_value(values)
    }

    fn selected_uniform_text_font_size(&self) -> Option<f64> {
        let mut values = Vec::new();
        for object in self.selected_text_objects() {
            values.push(normalize_toolbar_font_size(text_object_font_size(object)));
        }
        for node in self.selected_label_nodes() {
            if let Some(label) = &node.label {
                values.push(normalize_toolbar_font_size(label_font_size(label)));
            }
        }
        uniform_value(values)
    }

    fn selected_uniform_text_align(&self) -> Option<String> {
        let mut values = Vec::new();
        for object in self.selected_text_objects() {
            values.push(
                object
                    .payload
                    .extra
                    .get("align")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("left")
                    .to_string(),
            );
        }
        for node in self.selected_label_nodes() {
            if let Some(label) = &node.label {
                values.push(label.align.as_deref().unwrap_or("left").to_string());
            }
        }
        uniform_value(values)
    }

    fn selected_uniform_text_line_height(&self) -> Option<f64> {
        let mut values: Vec<Option<f64>> = Vec::new();
        for object in self.selected_text_objects() {
            values.push(Some(
                object
                    .payload
                    .extra
                    .get("lineHeight")
                    .and_then(JsonValue::as_f64)
                    .unwrap_or(crate::DEFAULT_TEXT_LINE_HEIGHT_PT),
            ));
        }
        for node in self.selected_label_nodes() {
            if node.label.is_some() {
                values.push(None);
            }
        }
        uniform_value(values).flatten().filter(|value| *value > 0.0)
    }

    fn selected_text_style_state(&self) -> TextStyleState {
        let mut bold = Vec::new();
        let mut italic = Vec::new();
        let mut underline = Vec::new();
        let mut superscript = Vec::new();
        let mut subscript = Vec::new();
        let mut formula = Vec::new();
        for object in self.selected_text_objects() {
            push_json_run_flags(
                text_object_runs(object),
                text_object_plain_text(object),
                &mut bold,
                &mut italic,
                &mut underline,
                &mut superscript,
                &mut subscript,
                &mut formula,
            );
        }
        for node in self.selected_label_nodes() {
            if let Some(label) = &node.label {
                push_label_run_flags(
                    label,
                    &mut bold,
                    &mut italic,
                    &mut underline,
                    &mut superscript,
                    &mut subscript,
                    &mut formula,
                );
            }
        }
        TextStyleState {
            bold: uniform_value(bold),
            italic: uniform_value(italic),
            underline: uniform_value(underline),
            superscript: uniform_value(superscript),
            subscript: uniform_value(subscript),
            formula: uniform_value(formula),
        }
    }
}

fn label_recognition_status(node: &Node) -> Option<&str> {
    node.meta
        .get("labelRecognition")
        .or_else(|| {
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("labelRecognition"))
        })
        .and_then(|value| value.get("status"))
        .and_then(JsonValue::as_str)
}

fn label_recognition_expansion_complete(node: &Node) -> Option<bool> {
    node.meta
        .get("labelRecognition")
        .or_else(|| {
            node.label
                .as_ref()
                .and_then(|label| label.meta.get("labelRecognition"))
        })
        .and_then(|value| value.get("expansion"))
        .and_then(|value| value.get("complete"))
        .and_then(JsonValue::as_bool)
}

fn separator() -> JsonValue {
    json!({ "type": "separator" })
}

fn item(label: &str, command: &str, value: &str) -> JsonValue {
    if value.is_empty() {
        json!({ "label": label, "command": command })
    } else {
        json!({ "label": label, "command": command, "value": value })
    }
}

fn checked_item(label: &str, command: &str, value: &str, checked: bool) -> JsonValue {
    if value.is_empty() {
        json!({ "label": label, "command": command, "checked": checked })
    } else {
        json!({ "label": label, "command": command, "value": value, "checked": checked })
    }
}

fn submenu(label: &str, children: Vec<JsonValue>) -> JsonValue {
    json!({ "label": label, "submenu": children })
}

fn color_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Color",
        vec![
            checked_item("Black", "color", "#000000", current == Some("#000000")),
            checked_item("Red", "color", "#ff0000", current == Some("#ff0000")),
            checked_item("Blue", "color", "#0000ff", current == Some("#0000ff")),
            checked_item("Green", "color", "#008000", current == Some("#008000")),
            checked_item("Yellow", "color", "#ffff00", current == Some("#ffff00")),
            checked_item("Orange", "color", "#ffa500", current == Some("#ffa500")),
            checked_item("Purple", "color", "#800080", current == Some("#800080")),
            checked_item("Gray", "color", "#808080", current == Some("#808080")),
            item("Other...", "color-other", ""),
        ],
    )
}

fn group_menu(has_group: bool, scene_count: usize) -> JsonValue {
    submenu(
        "Group",
        vec![
            json!({"label": "Group", "command": "group", "disabled": scene_count < 2}),
            json!({"label": "Ungroup", "command": "ungroup", "disabled": !has_group}),
        ],
    )
}

fn order_subitems_flat() -> JsonValue {
    submenu(
        "Order",
        vec![
            item("Bring Forward", "order", "bring-forward"),
            item("Send Backward", "order", "send-backward"),
            item("Bring to Front", "order", "bring-front"),
            item("Send to Back", "order", "send-back"),
        ],
    )
}

fn transform_subitems_flat(include_flip: bool) -> JsonValue {
    let mut children = Vec::new();
    if include_flip {
        children.push(item("Flip Horizontal", "arrange", "flip-h"));
        children.push(item("Flip Vertical", "arrange", "flip-v"));
    }
    children.push(item("Rotate...", "rotate-dialog", ""));
    children.push(item("Scale...", "scale-dialog", ""));
    submenu("Transform", children)
}

fn shape_style_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Shape Style",
        vec![
            checked_item("Plain", "shape-style", "plain", current == Some("plain")),
            checked_item("Dashed", "shape-style", "dashed", current == Some("dashed")),
            checked_item("Filled", "shape-style", "filled", current == Some("filled")),
            checked_item("Shaded", "shape-style", "shaded", current == Some("shaded")),
            checked_item("Faded", "shape-style", "faded", current == Some("faded")),
            checked_item(
                "Shadowed",
                "shape-style",
                "shadowed",
                current == Some("shadowed"),
            ),
        ],
    )
}

fn bracket_type_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Bracket Type",
        vec![
            checked_item(
                "Parentheses",
                "bracket-kind",
                "round",
                current == Some("round"),
            ),
            checked_item(
                "Square Brackets",
                "bracket-kind",
                "square",
                current == Some("square"),
            ),
            checked_item("Braces", "bracket-kind", "curly", current == Some("curly")),
        ],
    )
}

fn line_style_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Line Style",
        vec![
            checked_item("Plain", "line-style", "plain", current == Some("plain")),
            checked_item("Dashed", "line-style", "dashed", current == Some("dashed")),
            checked_item("Bold", "line-style", "bold", current == Some("bold")),
        ],
    )
}

fn arrowheads_menu(head: Option<&str>, tail: Option<&str>) -> JsonValue {
    submenu(
        "Arrowheads",
        vec![
            checked_item(
                "Full Arrow at Start",
                "arrow-endpoint",
                "tail:full",
                tail == Some("full"),
            ),
            checked_item(
                "Full Arrow at End",
                "arrow-endpoint",
                "head:full",
                head == Some("full"),
            ),
            checked_item(
                "Half Arrow at Start Left",
                "arrow-endpoint",
                "tail:left",
                matches!(tail, Some("half-left" | "left")),
            ),
            checked_item(
                "Half Arrow at Start Right",
                "arrow-endpoint",
                "tail:right",
                matches!(tail, Some("half-right" | "right")),
            ),
            checked_item(
                "Half Arrow at End Left",
                "arrow-endpoint",
                "head:left",
                matches!(head, Some("half-left" | "left")),
            ),
            checked_item(
                "Half Arrow at End Right",
                "arrow-endpoint",
                "head:right",
                matches!(head, Some("half-right" | "right")),
            ),
        ],
    )
}

fn bond_type_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Bond Type",
        vec![
            submenu(
                "Single",
                vec![
                    checked_item(
                        "Plain",
                        "bond-style",
                        "single-plain",
                        current == Some("single-plain"),
                    ),
                    checked_item(
                        "Dashed",
                        "bond-style",
                        "single-dashed",
                        current == Some("single-dashed"),
                    ),
                    checked_item(
                        "Hashed",
                        "bond-style",
                        "single-hashed",
                        current == Some("single-hashed"),
                    ),
                    checked_item(
                        "Hashed Wedged",
                        "bond-style",
                        "single-hashed-wedged",
                        current == Some("single-hashed-wedged"),
                    ),
                    checked_item(
                        "Bold",
                        "bond-style",
                        "single-bold",
                        current == Some("single-bold"),
                    ),
                    checked_item(
                        "Bold Wedged",
                        "bond-style",
                        "single-bold-wedged",
                        current == Some("single-bold-wedged"),
                    ),
                    checked_item(
                        "Hollow Wedged",
                        "bond-style",
                        "single-hollow-wedged",
                        current == Some("single-hollow-wedged"),
                    ),
                    checked_item(
                        "Wavy",
                        "bond-style",
                        "single-wavy",
                        current == Some("single-wavy"),
                    ),
                ],
            ),
            submenu(
                "Double",
                vec![
                    checked_item(
                        "Left",
                        "bond-style",
                        "double-left",
                        current == Some("double-left"),
                    ),
                    checked_item(
                        "Right",
                        "bond-style",
                        "double-right",
                        current == Some("double-right"),
                    ),
                    checked_item(
                        "Center",
                        "bond-style",
                        "double-center",
                        current == Some("double-center"),
                    ),
                    checked_item(
                        "Bold",
                        "bond-style",
                        "double-bold",
                        current == Some("double-bold"),
                    ),
                    checked_item(
                        "Dashed",
                        "bond-style",
                        "double-dashed",
                        current == Some("double-dashed"),
                    ),
                    checked_item(
                        "Double Dashed",
                        "bond-style",
                        "double-double-dashed",
                        current == Some("double-double-dashed"),
                    ),
                ],
            ),
            submenu(
                "Triple",
                vec![checked_item(
                    "Plain",
                    "bond-style",
                    "triple-plain",
                    current == Some("triple-plain"),
                )],
            ),
        ],
    )
}

fn orbital_template_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Orbital Template",
        vec![
            checked_item("s", "orbital-template", "s", current == Some("s")),
            checked_item("p", "orbital-template", "p", current == Some("p")),
            checked_item("dxy", "orbital-template", "dxy", current == Some("dxy")),
            checked_item("oval", "orbital-template", "oval", current == Some("oval")),
            checked_item(
                "hybrid",
                "orbital-template",
                "hybrid",
                current == Some("hybrid"),
            ),
            checked_item("dz2", "orbital-template", "dz2", current == Some("dz2")),
            checked_item("lobe", "orbital-template", "lobe", current == Some("lobe")),
        ],
    )
}

fn orbital_style_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Orbital Style",
        vec![
            checked_item(
                "Hollow",
                "orbital-style",
                "hollow",
                current == Some("hollow"),
            ),
            checked_item(
                "Filled",
                "orbital-style",
                "filled",
                current == Some("filled"),
            ),
            checked_item(
                "Shaded",
                "orbital-style",
                "shaded",
                current == Some("shaded"),
            ),
        ],
    )
}

fn orbital_phase_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Orbital Phase",
        vec![
            checked_item("Plus", "orbital-phase", "plus", current == Some("plus")),
            checked_item("Minus", "orbital-phase", "minus", current == Some("minus")),
        ],
    )
}

fn text_font_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Font",
        [
            "Arial",
            "Helvetica",
            "TeX Gyre Heros",
            "Times New Roman",
            "Courier New",
        ]
        .into_iter()
        .map(|font| {
            checked_item(
                font,
                "text-style",
                &format!("font-family:{font}"),
                current == Some(font),
            )
        })
        .collect(),
    )
}

fn text_size_menu(current: Option<f64>) -> JsonValue {
    let mut sizes = vec![5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0, 16.0, 18.0, 24.0];
    if let Some(current) = current {
        if !sizes
            .iter()
            .any(|size| (*size - current).abs() < crate::EPSILON)
        {
            sizes.push(current);
            sizes.sort_by(|left, right| left.total_cmp(right));
        }
    }
    submenu(
        "Size",
        sizes
            .into_iter()
            .map(|size| {
                checked_item(
                    &format_toolbar_font_size(size),
                    "text-style",
                    &format!("font-size:{}", format_toolbar_font_size(size)),
                    current.is_some_and(|current| (current - size).abs() < crate::EPSILON),
                )
            })
            .collect(),
    )
}

fn text_style_menu(current: TextStyleState) -> JsonValue {
    submenu(
        "Style",
        vec![
            toggle_style_item("Bold", "bold", current.bold),
            toggle_style_item("Italic", "italic", current.italic),
            toggle_style_item("Underline", "underline", current.underline),
            toggle_style_item("Superscript", "superscript", current.superscript),
            toggle_style_item("Subscript", "subscript", current.subscript),
            toggle_style_item("Formula", "formula", current.formula),
        ],
    )
}

fn text_alignment_menu(current: Option<&str>) -> JsonValue {
    submenu(
        "Alignment",
        vec![
            checked_item("Left", "text-style", "align:left", current == Some("left")),
            checked_item(
                "Center",
                "text-style",
                "align:center",
                current == Some("center"),
            ),
            checked_item(
                "Right",
                "text-style",
                "align:right",
                current == Some("right"),
            ),
            checked_item(
                "Justified",
                "text-style",
                "align:justify",
                current == Some("justify"),
            ),
        ],
    )
}

#[derive(Default)]
struct TextStyleState {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    superscript: Option<bool>,
    subscript: Option<bool>,
    formula: Option<bool>,
}

fn toggle_style_item(label: &str, command: &str, current: Option<bool>) -> JsonValue {
    let checked = current == Some(true);
    checked_item(
        label,
        "text-style",
        &format!("{command}:{}", if checked { "off" } else { "on" }),
        checked,
    )
}

fn uniform_value<T>(values: Vec<T>) -> Option<T>
where
    T: PartialEq + Clone,
{
    let first = values.first()?.clone();
    values.iter().all(|value| *value == first).then_some(first)
}

fn style_color_for_object(document: &crate::ChemcoreDocument, object: &SceneObject) -> String {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    css_color_to_hex(
        payload_string(object, "fill")
            .or_else(|| payload_string(object, "stroke"))
            .or_else(|| style.and_then(|style| style_string(style, "fill")))
            .or_else(|| style.and_then(|style| style_string(style, "stroke")))
            .or_else(|| payload_string(object, "color"))
            .as_deref()
            .unwrap_or("#000000"),
    )
}

fn shape_style_for_object(document: &crate::ChemcoreDocument, object: &SceneObject) -> String {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    if style.is_some_and(|style| truthy_field(style, "shadow") || truthy_field(style, "shadowed")) {
        return "shadowed".to_string();
    }
    if style.is_some_and(|style| truthy_field(style, "faded")) {
        return "faded".to_string();
    }
    if style.is_some_and(|style| truthy_field(style, "shaded")) {
        return "shaded".to_string();
    }
    if style.is_some_and(|style| {
        style.get("fill").is_some_and(|value| !value.is_null())
            && !style.get("stroke").is_some_and(|value| !value.is_null())
    }) {
        return "filled".to_string();
    }
    if style.is_some_and(|style| {
        style
            .get("dashArray")
            .and_then(JsonValue::as_array)
            .is_some_and(|items| !items.is_empty())
    }) {
        return "dashed".to_string();
    }
    "plain".to_string()
}

fn line_object_style(document: &crate::ChemcoreDocument, object: &SceneObject) -> String {
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    if object
        .payload
        .extra
        .get("arrowHead")
        .and_then(|arrow| arrow.get("bold"))
        .and_then(JsonValue::as_bool)
        == Some(true)
    {
        return "bold".to_string();
    }
    if style.is_some_and(|style| {
        style
            .get("dashArray")
            .and_then(JsonValue::as_array)
            .is_some_and(|items| !items.is_empty())
    }) {
        return "dashed".to_string();
    }
    "plain".to_string()
}

fn bond_style_key(bond: &Bond) -> String {
    if bond
        .meta
        .get("contextMenuBondStyle")
        .and_then(JsonValue::as_str)
        == Some("single-wavy")
    {
        return "single-wavy".to_string();
    }
    if bond.order == 3 {
        return "triple-plain".to_string();
    }
    if bond.order == 2 {
        let placement = bond
            .double
            .as_ref()
            .map(|double| double.placement)
            .unwrap_or(DoubleBondPlacement::Center);
        if bond.line_styles.left == BondLinePattern::Dashed
            && bond.line_styles.right == BondLinePattern::Dashed
        {
            return "double-double-dashed".to_string();
        }
        if bond.line_styles.main == BondLinePattern::Dashed {
            return "double-dashed".to_string();
        }
        if bond.line_weights.main == BondLineWeight::Bold {
            return "double-bold".to_string();
        }
        return match placement {
            DoubleBondPlacement::Left => "double-left",
            DoubleBondPlacement::Right => "double-right",
            DoubleBondPlacement::Center => "double-center",
        }
        .to_string();
    }
    if bond
        .meta
        .get("contextMenuBondStyle")
        .and_then(JsonValue::as_str)
        == Some("single-hashed")
    {
        return "single-hashed".to_string();
    }
    let stereo = bond
        .stereo
        .as_ref()
        .map(|stereo| stereo.kind.as_str())
        .unwrap_or("");
    if stereo.contains("hashed") {
        return "single-hashed-wedged".to_string();
    }
    if stereo.contains("hollow-wedge") {
        return "single-hollow-wedged".to_string();
    }
    if stereo.contains("wedge") {
        return "single-bold-wedged".to_string();
    }
    if bond.line_weights.main == BondLineWeight::Bold {
        return "single-bold".to_string();
    }
    if bond.line_styles.main == BondLinePattern::Dashed {
        return "single-dashed".to_string();
    }
    "single-plain".to_string()
}

fn text_object_runs(object: &SceneObject) -> Vec<&JsonValue> {
    object
        .payload
        .extra
        .get("sourceRuns")
        .and_then(JsonValue::as_array)
        .filter(|runs| !runs.is_empty())
        .or_else(|| {
            object
                .payload
                .extra
                .get("runs")
                .and_then(JsonValue::as_array)
        })
        .map(|runs| runs.iter().collect())
        .unwrap_or_default()
}

fn text_object_plain_text(object: &SceneObject) -> String {
    object
        .payload
        .extra
        .get("text")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            text_object_runs(object)
                .into_iter()
                .filter_map(|run| run.get("text").and_then(JsonValue::as_str))
                .collect()
        })
}

fn text_object_font_family(object: &SceneObject) -> String {
    payload_string(object, "fontFamily")
        .or_else(|| {
            text_object_runs(object)
                .first()
                .and_then(|run| run.get("fontFamily"))
                .and_then(JsonValue::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "Arial".to_string())
}

fn text_object_font_size(object: &SceneObject) -> f64 {
    payload_number(object, "fontSize")
        .or_else(|| {
            text_object_runs(object)
                .first()
                .and_then(|run| run.get("fontSize"))
                .and_then(JsonValue::as_f64)
        })
        .unwrap_or(crate::DEFAULT_TEXT_FONT_SIZE_PT)
}

fn label_runs(label: &NodeLabel) -> Vec<&LabelRun> {
    if !label.runs.is_empty() {
        return label.runs.iter().collect();
    }
    label.line_runs.iter().flatten().collect()
}

fn label_font_family(label: &NodeLabel) -> String {
    label
        .font_family
        .clone()
        .or_else(|| {
            label_runs(label)
                .first()
                .and_then(|run| run.font_family.clone())
        })
        .unwrap_or_else(|| "Arial".to_string())
}

fn label_font_size(label: &NodeLabel) -> f64 {
    label
        .font_size
        .or_else(|| label_runs(label).first().and_then(|run| run.font_size))
        .unwrap_or(crate::DEFAULT_TEXT_FONT_SIZE_PT)
}

#[allow(clippy::too_many_arguments)]
fn push_json_run_flags(
    runs: Vec<&JsonValue>,
    plain_text: String,
    bold: &mut Vec<bool>,
    italic: &mut Vec<bool>,
    underline: &mut Vec<bool>,
    superscript: &mut Vec<bool>,
    subscript: &mut Vec<bool>,
    formula: &mut Vec<bool>,
) {
    if runs.is_empty() && !plain_text.is_empty() {
        bold.push(false);
        italic.push(false);
        underline.push(false);
        superscript.push(false);
        subscript.push(false);
        formula.push(false);
        return;
    }
    for run in runs {
        bold.push(
            run.get("fontWeight")
                .and_then(JsonValue::as_u64)
                .unwrap_or(400)
                >= 600,
        );
        italic.push(
            run.get("fontStyle")
                .and_then(JsonValue::as_str)
                .unwrap_or("normal")
                == "italic",
        );
        underline.push(run.get("underline").and_then(JsonValue::as_bool) == Some(true));
        let script = run
            .get("script")
            .and_then(JsonValue::as_str)
            .unwrap_or("normal");
        superscript.push(script == "superscript");
        subscript.push(script == "subscript");
        formula.push(script == "chemical");
    }
}

#[allow(clippy::too_many_arguments)]
fn push_label_run_flags(
    label: &NodeLabel,
    bold: &mut Vec<bool>,
    italic: &mut Vec<bool>,
    underline: &mut Vec<bool>,
    superscript: &mut Vec<bool>,
    subscript: &mut Vec<bool>,
    formula: &mut Vec<bool>,
) {
    let runs = label_runs(label);
    if runs.is_empty() && !label.text.is_empty() {
        bold.push(false);
        italic.push(false);
        underline.push(false);
        superscript.push(false);
        subscript.push(false);
        formula.push(false);
        return;
    }
    for run in runs {
        bold.push(run.font_weight.unwrap_or(400) >= 600);
        italic.push(run.font_style.as_deref().unwrap_or("normal") == "italic");
        underline.push(run.underline.unwrap_or(false));
        let script = run.script.as_deref().unwrap_or("normal");
        superscript.push(script == "superscript");
        subscript.push(script == "subscript");
        formula.push(script == "chemical");
    }
}

fn payload_string(object: &SceneObject, key: &str) -> Option<String> {
    object
        .payload
        .extra
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
}

fn payload_number(object: &SceneObject, key: &str) -> Option<f64> {
    object.payload.extra.get(key).and_then(JsonValue::as_f64)
}

fn style_string(style: &JsonValue, key: &str) -> Option<String> {
    style
        .get(key)
        .filter(|value| !value.is_null())
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
}

fn truthy_field(style: &JsonValue, key: &str) -> bool {
    style.get(key).and_then(JsonValue::as_bool) == Some(true)
}

fn css_color_to_hex(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = normalize_hex_color(&value) {
        return hex;
    }
    if let Some(hex) = rgb_color_to_hex(&value) {
        return hex;
    }
    match value.as_str() {
        "black" => "#000000",
        "red" => "#ff0000",
        "blue" => "#0000ff",
        "green" => "#008000",
        "yellow" => "#ffff00",
        "orange" => "#ffa500",
        "purple" => "#800080",
        "gray" | "grey" => "#808080",
        _ => value.as_str(),
    }
    .to_string()
}

fn normalize_hex_color(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#')?;
    if hex.len() == 3 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let mut out = String::from("#");
        for ch in hex.chars() {
            out.push(ch);
            out.push(ch);
        }
        return Some(out);
    }
    if hex.len() >= 6 && hex.chars().take(6).all(|ch| ch.is_ascii_hexdigit()) {
        return Some(format!("#{}", &hex[..6]));
    }
    None
}

fn rgb_color_to_hex(value: &str) -> Option<String> {
    let inner = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix("rgba(")
                .and_then(|value| value.strip_suffix(')'))
        })?;
    let parts: Vec<u8> = inner
        .split(',')
        .take(3)
        .map(|part| {
            part.trim()
                .parse::<f64>()
                .ok()
                .map(|value| value.round() as i32)
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .map(|value| value.clamp(0, 255) as u8)
        .collect();
    (parts.len() == 3).then(|| format!("#{:02x}{:02x}{:02x}", parts[0], parts[1], parts[2]))
}

fn normalize_toolbar_font_size(value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return 10.0;
    }
    let rounded = value.round();
    if (value - rounded).abs() < 0.05 {
        return rounded;
    }
    (value * 10.0).round() / 10.0
}

fn format_toolbar_font_size(value: f64) -> String {
    let normalized = normalize_toolbar_font_size(value);
    if (normalized - normalized.round()).abs() < crate::EPSILON {
        format!("{}", normalized.round() as i64)
    } else {
        format!("{normalized:.1}")
    }
}
