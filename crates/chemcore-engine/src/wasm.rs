use crate::{BondVariant, Engine, PointerEvent, Tool, ToolState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmEngine {
    inner: Engine,
}

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Engine::new(),
        }
    }

    #[wasm_bindgen(js_name = setTool)]
    pub fn set_tool(&mut self, active_tool: &str, bond_variant: &str) {
        self.inner.set_tool_state(ToolState {
            active_tool: parse_tool(active_tool),
            bond_variant: parse_bond_variant(bond_variant),
        });
    }

    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&mut self, x: f64, y: f64, alt_key: bool) {
        self.inner.pointer_move(PointerEvent {
            x,
            y,
            button: None,
            alt_key,
        });
    }

    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(&mut self, x: f64, y: f64, alt_key: bool) {
        self.inner.pointer_down(PointerEvent {
            x,
            y,
            button: Some(0),
            alt_key,
        });
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self, x: f64, y: f64, alt_key: bool) {
        self.inner.pointer_up(PointerEvent {
            x,
            y,
            button: Some(0),
            alt_key,
        });
    }

    #[wasm_bindgen(js_name = clearInteraction)]
    pub fn clear_interaction(&mut self) {
        self.inner.clear_interaction();
    }

    #[wasm_bindgen(js_name = loadDocumentJson)]
    pub fn load_document_json(&mut self, json: &str) -> Result<(), JsValue> {
        self.inner
            .load_document_json(json)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn undo(&mut self) -> bool {
        self.inner.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.inner.redo()
    }

    #[wasm_bindgen(js_name = deleteSelection)]
    pub fn delete_selection(&mut self) -> bool {
        self.inner.delete_selection()
    }

    #[wasm_bindgen(js_name = replaceHoveredEndpointLabel)]
    pub fn replace_hovered_endpoint_label(&mut self, label: &str) -> bool {
        self.inner.replace_hovered_endpoint_label(label)
    }

    #[wasm_bindgen(js_name = beginTextEdit)]
    pub fn begin_text_edit(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let session = self
            .inner
            .begin_text_edit(crate::Point::new(x, y))
            .ok_or_else(|| JsValue::from_str("No text edit target"))?;
        serde_json::to_string(&session).map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = applyTextEdit)]
    pub fn apply_text_edit(&mut self, session_json: &str) -> Result<bool, JsValue> {
        let session: crate::TextEditSession = serde_json::from_str(session_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        Ok(self.inner.apply_text_edit(session))
    }

    #[wasm_bindgen(js_name = previewTextRuns)]
    pub fn preview_text_runs(&self, session_json: &str) -> Result<String, JsValue> {
        let session: crate::TextEditSession = serde_json::from_str(session_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let (source_runs, display_runs) = self.inner.preview_text_runs(&session);
        serde_json::to_string(&serde_json::json!({
            "sourceRuns": source_runs,
            "displayRuns": display_runs,
        }))
        .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = canUndo)]
    pub fn can_undo(&self) -> bool {
        self.inner.can_undo()
    }

    #[wasm_bindgen(js_name = canRedo)]
    pub fn can_redo(&self) -> bool {
        self.inner.can_redo()
    }

    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> Result<String, JsValue> {
        self.inner
            .state_json()
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = documentJson)]
    pub fn document_json(&self) -> Result<String, JsValue> {
        self.inner
            .document_json()
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = renderListJson)]
    pub fn render_list_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.render_list())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_tool(value: &str) -> Tool {
    match value {
        "bond" => Tool::Bond,
        "text" => Tool::Text,
        "shape" => Tool::Shape,
        "templates" => Tool::Templates,
        _ => Tool::Select,
    }
}

fn parse_bond_variant(value: &str) -> BondVariant {
    match value {
        "double" => BondVariant::Double,
        "triple" => BondVariant::Triple,
        "dashed" => BondVariant::Dashed,
        "dashed-double" => BondVariant::DashedDouble,
        "bold" => BondVariant::Bold,
        "bold-dashed" => BondVariant::BoldDashed,
        "wedge" => BondVariant::Wedge,
        "hashed-wedge" => BondVariant::HashedWedge,
        _ => BondVariant::Single,
    }
}
