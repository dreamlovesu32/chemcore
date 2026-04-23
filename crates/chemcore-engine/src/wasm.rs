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
    pub fn pointer_move(&mut self, x: f64, y: f64) {
        self.inner.pointer_move(PointerEvent { x, y, button: None });
    }

    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(&mut self, x: f64, y: f64) {
        self.inner.pointer_down(PointerEvent {
            x,
            y,
            button: Some(0),
        });
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self, x: f64, y: f64) {
        self.inner.pointer_up(PointerEvent {
            x,
            y,
            button: Some(0),
        });
    }

    #[wasm_bindgen(js_name = clearInteraction)]
    pub fn clear_interaction(&mut self) {
        self.inner.clear_interaction();
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
