use crate::{
    ArrowCurve, ArrowEndpointStyle, ArrowHeadSize, ArrowNoGo, ArrowVariant, BondVariant,
    BracketKind, Engine, OrbitalPhase, OrbitalStyle, OrbitalTemplate, Point, PointerEvent,
    RenderBoundsScope, ShapeKind, ShapeStyle, Tool, ToolState, WorldPoint, WorldPt,
};
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
        let current = self.inner.state().tool.clone();
        self.inner.set_tool_state(ToolState {
            active_tool: parse_tool(active_tool),
            bond_variant: parse_bond_variant(bond_variant),
            arrow_variant: current.arrow_variant,
            arrow_head_size: current.arrow_head_size,
            arrow_curve: current.arrow_curve,
            arrow_head_style: current.arrow_head_style,
            arrow_tail_style: current.arrow_tail_style,
            arrow_head: current.arrow_head,
            arrow_tail: current.arrow_tail,
            arrow_bold: current.arrow_bold,
            arrow_no_go: current.arrow_no_go,
            shape_kind: current.shape_kind,
            shape_style: current.shape_style,
            shape_color: current.shape_color,
            orbital_template: current.orbital_template,
            orbital_style: current.orbital_style,
            orbital_phase: current.orbital_phase,
            orbital_color: current.orbital_color,
            bracket_kind: current.bracket_kind,
            symbol_kind: current.symbol_kind,
            element_symbol: current.element_symbol,
            element_atomic_number: current.element_atomic_number,
            template: current.template,
        });
    }

    #[wasm_bindgen(js_name = setShapeOptions)]
    pub fn set_shape_options(&mut self, kind: &str, style: &str, color: &str) {
        let mut tool = self.inner.state().tool.clone();
        tool.shape_kind = parse_shape_kind(kind);
        tool.shape_style = parse_shape_style(style);
        tool.shape_color = color.to_string();
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = setTemplate)]
    pub fn set_template(&mut self, template: &str) {
        let mut tool = self.inner.state().tool.clone();
        tool.template = template.to_string();
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = setOrbitalOptions)]
    pub fn set_orbital_options(&mut self, template: &str, style: &str, phase: &str, color: &str) {
        let mut tool = self.inner.state().tool.clone();
        tool.orbital_template = parse_orbital_template(template);
        tool.orbital_style = parse_orbital_style(style);
        tool.orbital_phase = parse_orbital_phase(phase);
        tool.orbital_color = color.to_string();
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = setBracketOptions)]
    pub fn set_bracket_options(&mut self, kind: &str) {
        let mut tool = self.inner.state().tool.clone();
        tool.bracket_kind = parse_bracket_kind(kind);
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = setSymbolOptions)]
    pub fn set_symbol_options(&mut self, kind: &str) {
        let mut tool = self.inner.state().tool.clone();
        tool.symbol_kind = parse_bracket_kind(kind);
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = setElementOptions)]
    pub fn set_element_options(&mut self, symbol: &str, atomic_number: u8) {
        let mut tool = self.inner.state().tool.clone();
        tool.element_symbol = symbol.to_string();
        tool.element_atomic_number = atomic_number;
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = setDocumentStylePreset)]
    pub fn set_document_style_preset(&mut self, preset: &str) {
        self.inner.set_document_style_preset(preset);
    }

    #[wasm_bindgen(js_name = documentStylePreset)]
    pub fn document_style_preset(&self) -> String {
        self.inner.document_style_preset().to_string()
    }

    #[wasm_bindgen(js_name = objectSettingsDialogJson)]
    pub fn object_settings_dialog_json(&self) -> String {
        self.inner.object_settings_dialog_json()
    }

    #[wasm_bindgen(js_name = toolbarColorPaletteJson)]
    pub fn toolbar_color_palette_json(&self, custom_colors_json: &str) -> String {
        self.inner.toolbar_color_palette_json(custom_colors_json)
    }

    #[wasm_bindgen(js_name = colorDialogPaletteJson)]
    pub fn color_dialog_palette_json(
        &self,
        current_color: &str,
        custom_colors_json: &str,
    ) -> String {
        self.inner
            .color_dialog_palette_json(current_color, custom_colors_json)
    }

    #[wasm_bindgen(js_name = textSymbolPaletteJson)]
    pub fn text_symbol_palette_json(&self) -> String {
        self.inner.text_symbol_palette_json()
    }

    #[wasm_bindgen(js_name = elementPaletteJson)]
    pub fn element_palette_json(&self) -> String {
        self.inner.element_palette_json()
    }

    #[wasm_bindgen(js_name = bondToolIconSvg)]
    pub fn bond_tool_icon_svg(&self, variant: &str, stroke_width: f64, bold_width: f64) -> String {
        Engine::bond_tool_icon_svg(parse_bond_variant(variant), stroke_width, bold_width)
    }

    #[wasm_bindgen(js_name = arrowToolIconSvg)]
    pub fn arrow_tool_icon_svg(&self, kind: &str) -> String {
        Engine::arrow_tool_icon_svg(kind)
    }

    #[wasm_bindgen(js_name = shapeToolIconSvg)]
    pub fn shape_tool_icon_svg(&self, kind: &str, style: &str) -> String {
        Engine::shape_tool_icon_svg(parse_shape_kind(kind), parse_shape_style(style))
    }

    #[wasm_bindgen(js_name = symbolToolIconSvg)]
    pub fn symbol_tool_icon_svg(&self, kind: &str) -> String {
        Engine::symbol_tool_icon_svg(parse_bracket_kind(kind))
    }

    #[wasm_bindgen(js_name = orbitalToolIconSvg)]
    pub fn orbital_tool_icon_svg(&self, template: &str, style: &str, phase: &str) -> String {
        Engine::orbital_tool_icon_svg(
            parse_orbital_template(template),
            parse_orbital_style(style),
            parse_orbital_phase(phase),
        )
    }

    #[wasm_bindgen(js_name = chainToolIconSvg)]
    pub fn chain_tool_icon_svg(&self, stroke_width: f64) -> String {
        Engine::chain_tool_icon_svg(stroke_width)
    }

    #[wasm_bindgen(js_name = textFormatIconSvg)]
    pub fn text_format_icon_svg(&self, kind: &str) -> String {
        Engine::text_format_icon_svg(kind)
    }

    #[wasm_bindgen(js_name = selectionChemistrySummaryJson)]
    pub fn selection_chemistry_summary_json(&self) -> String {
        self.inner.selection_chemistry_summary_json()
    }

    #[wasm_bindgen(js_name = applyElementPaletteJson)]
    pub fn apply_element_palette_json(&mut self, selection_json: &str) -> Result<bool, JsValue> {
        self.inner
            .apply_element_palette_json(selection_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = applyObjectSettingsDialogJson)]
    pub fn apply_object_settings_dialog_json(
        &mut self,
        settings_json: &str,
    ) -> Result<bool, JsValue> {
        self.inner
            .apply_object_settings_dialog_json(settings_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = setArrowOptions)]
    pub fn set_arrow_options(
        &mut self,
        variant: &str,
        head_size: &str,
        head: bool,
        tail: bool,
        bold: bool,
    ) {
        let mut tool = self.inner.state().tool.clone();
        tool.arrow_variant = parse_arrow_variant(variant);
        tool.arrow_head_size = parse_arrow_head_size(head_size);
        tool.arrow_curve = ArrowCurve::Arc270;
        tool.arrow_head_style = if head {
            ArrowEndpointStyle::Full
        } else {
            ArrowEndpointStyle::None
        };
        tool.arrow_tail_style = if tail {
            ArrowEndpointStyle::Full
        } else {
            ArrowEndpointStyle::None
        };
        tool.arrow_head = head;
        tool.arrow_tail = tail;
        tool.arrow_bold = bold;
        tool.arrow_no_go = ArrowNoGo::None;
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = applyArrowOptionsToSelection)]
    pub fn apply_arrow_options_to_selection(
        &mut self,
        variant: &str,
        head_size: &str,
        head: bool,
        tail: bool,
        bold: bool,
    ) -> bool {
        self.inner.apply_arrow_options_to_selection(
            parse_arrow_variant(variant),
            parse_arrow_head_size(head_size),
            ArrowCurve::Arc270,
            if head {
                ArrowEndpointStyle::Full
            } else {
                ArrowEndpointStyle::None
            },
            if tail {
                ArrowEndpointStyle::Full
            } else {
                ArrowEndpointStyle::None
            },
            head,
            tail,
            bold,
            ArrowNoGo::None,
        )
    }

    #[wasm_bindgen(js_name = setArrowEndpointOptions)]
    pub fn set_arrow_endpoint_options(
        &mut self,
        variant: &str,
        head_size: &str,
        curve: &str,
        head_style: &str,
        tail_style: &str,
        no_go: &str,
        bold: bool,
    ) {
        let mut tool = self.inner.state().tool.clone();
        tool.arrow_variant = parse_arrow_variant(variant);
        tool.arrow_head_size = parse_arrow_head_size(head_size);
        tool.arrow_curve = parse_arrow_curve(curve);
        tool.arrow_head_style = parse_arrow_endpoint_style(head_style);
        tool.arrow_tail_style = parse_arrow_endpoint_style(tail_style);
        tool.arrow_head = tool.arrow_head_style != ArrowEndpointStyle::None;
        tool.arrow_tail = tool.arrow_tail_style != ArrowEndpointStyle::None;
        tool.arrow_no_go = parse_arrow_no_go(no_go);
        tool.arrow_bold = bold;
        self.inner.set_tool_state(tool);
    }

    #[wasm_bindgen(js_name = applyArrowEndpointOptionsToSelection)]
    pub fn apply_arrow_endpoint_options_to_selection(
        &mut self,
        variant: &str,
        head_size: &str,
        curve: &str,
        head_style: &str,
        tail_style: &str,
        no_go: &str,
        bold: bool,
    ) -> bool {
        let head_style = parse_arrow_endpoint_style(head_style);
        let tail_style = parse_arrow_endpoint_style(tail_style);
        self.inner.apply_arrow_options_to_selection(
            parse_arrow_variant(variant),
            parse_arrow_head_size(head_size),
            parse_arrow_curve(curve),
            head_style,
            tail_style,
            head_style != ArrowEndpointStyle::None,
            tail_style != ArrowEndpointStyle::None,
            bold,
            parse_arrow_no_go(no_go),
        )
    }

    #[wasm_bindgen(js_name = pointerMove)]
    pub fn pointer_move(&mut self, x: f64, y: f64, alt_key: bool) {
        self.inner.pointer_move(PointerEvent::from_world_point(
            WorldPoint::new(WorldPt(x), WorldPt(y)),
            None,
            alt_key,
        ));
    }

    #[wasm_bindgen(js_name = pointerDown)]
    pub fn pointer_down(&mut self, x: f64, y: f64, alt_key: bool) {
        self.inner.pointer_down(PointerEvent::from_world_point(
            WorldPoint::new(WorldPt(x), WorldPt(y)),
            Some(0),
            alt_key,
        ));
    }

    #[wasm_bindgen(js_name = pointerUp)]
    pub fn pointer_up(&mut self, x: f64, y: f64, alt_key: bool) {
        self.inner.pointer_up(PointerEvent::from_world_point(
            WorldPoint::new(WorldPt(x), WorldPt(y)),
            Some(0),
            alt_key,
        ));
    }

    #[wasm_bindgen(js_name = selectAtPoint)]
    pub fn select_at_point(&mut self, x: f64, y: f64, additive: bool) {
        self.inner.select_at_point(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            additive,
        );
    }

    #[wasm_bindgen(js_name = selectComponentAtPoint)]
    pub fn select_component_at_point(&mut self, x: f64, y: f64, additive: bool) -> bool {
        self.inner.select_component_at_point(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            additive,
        )
    }

    #[wasm_bindgen(js_name = selectInRect)]
    pub fn select_in_rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, additive: bool) {
        self.inner.select_in_rect(
            Point::from_world(WorldPoint::new(WorldPt(x1), WorldPt(y1))),
            Point::from_world(WorldPoint::new(WorldPt(x2), WorldPt(y2))),
            additive,
        );
    }

    #[wasm_bindgen(js_name = selectInPolygon)]
    pub fn select_in_polygon(&mut self, points_json: &str, additive: bool) -> Result<(), JsValue> {
        let raw_points: Vec<[f64; 2]> = serde_json::from_str(points_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let points = raw_points
            .into_iter()
            .map(|point| Point::from_world(WorldPoint::new(WorldPt(point[0]), WorldPt(point[1]))))
            .collect();
        self.inner.select_in_polygon(points, additive);
        Ok(())
    }

    #[wasm_bindgen(js_name = selectAll)]
    pub fn select_all(&mut self) -> bool {
        self.inner.select_all()
    }

    #[wasm_bindgen(js_name = beginTlcSpotDragJson)]
    pub fn begin_tlc_spot_drag_json(&mut self, x: f64, y: f64) -> Result<Option<String>, JsValue> {
        let hit = self
            .inner
            .begin_tlc_spot_drag(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))));
        hit.map(|value| {
            serde_json::to_string(&value).map_err(|error| JsValue::from_str(&error.to_string()))
        })
        .transpose()
    }

    #[wasm_bindgen(js_name = tlcSpotHitTestJson)]
    pub fn tlc_spot_hit_test_json(&self, x: f64, y: f64) -> Result<Option<String>, JsValue> {
        let hit = self
            .inner
            .tlc_spot_hit_test(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))));
        hit.map(|value| {
            serde_json::to_string(&value).map_err(|error| JsValue::from_str(&error.to_string()))
        })
        .transpose()
    }

    #[wasm_bindgen(js_name = tlcLaneGuideHitTestJson)]
    pub fn tlc_lane_guide_hit_test_json(&self, x: f64, y: f64) -> Result<Option<String>, JsValue> {
        let hit = self
            .inner
            .tlc_lane_guide_hit_test(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))));
        hit.map(|value| {
            serde_json::to_string(&value).map_err(|error| JsValue::from_str(&error.to_string()))
        })
        .transpose()
    }

    #[wasm_bindgen(js_name = updateTlcSpotDragJson)]
    pub fn update_tlc_spot_drag_json(&mut self, x: f64, y: f64) -> Result<Option<String>, JsValue> {
        let hit = self
            .inner
            .update_tlc_spot_drag(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))));
        hit.map(|value| {
            serde_json::to_string(&value).map_err(|error| JsValue::from_str(&error.to_string()))
        })
        .transpose()
    }

    #[wasm_bindgen(js_name = finishTlcSpotDragJson)]
    pub fn finish_tlc_spot_drag_json(&mut self, x: f64, y: f64) -> Result<Option<String>, JsValue> {
        let hit = self
            .inner
            .finish_tlc_spot_drag(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))));
        hit.map(|value| {
            serde_json::to_string(&value).map_err(|error| JsValue::from_str(&error.to_string()))
        })
        .transpose()
    }

    #[wasm_bindgen(js_name = clearSelection)]
    pub fn clear_selection(&mut self) -> bool {
        self.inner.clear_selection()
    }

    #[wasm_bindgen(js_name = contextHitTestJson)]
    pub fn context_hit_test_json(&self, x: f64, y: f64) -> String {
        self.inner
            .context_hit_test_json(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
    }

    #[wasm_bindgen(js_name = contextMenuJson)]
    pub fn context_menu_json(&self, hit_json: &str, has_paste: bool) -> String {
        self.inner.context_menu_json(hit_json, has_paste)
    }

    #[wasm_bindgen(js_name = selectionContainsPoint)]
    pub fn selection_contains_point(&self, x: f64, y: f64) -> bool {
        self.inner
            .selection_contains_point(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
    }

    #[wasm_bindgen(js_name = hoverArrowAction)]
    pub fn hover_arrow_action(&self, x: f64, y: f64) -> String {
        self.inner
            .hover_arrow_action_at_point(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
            .to_string()
    }

    #[wasm_bindgen(js_name = beginHoverArrowEdit)]
    pub fn begin_hover_arrow_edit(&mut self, x: f64, y: f64) -> String {
        self.inner
            .begin_hover_arrow_edit(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
            .to_string()
    }

    #[wasm_bindgen(js_name = updateHoverArrowEdit)]
    pub fn update_hover_arrow_edit(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.update_hover_arrow_edit(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = finishHoverArrowEdit)]
    pub fn finish_hover_arrow_edit(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.finish_hover_arrow_edit(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = hoverShapeAction)]
    pub fn hover_shape_action(&self, x: f64, y: f64) -> String {
        self.inner
            .hover_shape_action_at_point(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
            .to_string()
    }

    #[wasm_bindgen(js_name = beginHoverShapeEdit)]
    pub fn begin_hover_shape_edit(&mut self, x: f64, y: f64) -> String {
        self.inner
            .begin_hover_shape_edit(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
            .to_string()
    }

    #[wasm_bindgen(js_name = updateHoverShapeEdit)]
    pub fn update_hover_shape_edit(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.update_hover_shape_edit(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = finishHoverShapeEdit)]
    pub fn finish_hover_shape_edit(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.finish_hover_shape_edit(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = activeArrowEditDegrees)]
    pub fn active_arrow_edit_degrees(&self) -> f64 {
        self.inner.active_arrow_edit_degrees()
    }

    #[wasm_bindgen(js_name = beginSelectionMove)]
    pub fn begin_selection_move(&mut self, x: f64, y: f64, additive: bool, alt_key: bool) -> bool {
        self.inner.begin_selection_move_at_point(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            additive,
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = updateSelectionMove)]
    pub fn update_selection_move(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.update_selection_move(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = finishSelectionMove)]
    pub fn finish_selection_move(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.finish_selection_move(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = beginSelectionRotate)]
    pub fn begin_selection_rotate(&mut self, x: f64, y: f64) -> bool {
        self.inner
            .begin_selection_rotate(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
    }

    #[wasm_bindgen(js_name = updateSelectionRotate)]
    pub fn update_selection_rotate(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.update_selection_rotate(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = finishSelectionRotate)]
    pub fn finish_selection_rotate(&mut self, x: f64, y: f64, alt_key: bool) -> bool {
        self.inner.finish_selection_rotate(
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
            alt_key,
        )
    }

    #[wasm_bindgen(js_name = beginSelectionResize)]
    pub fn begin_selection_resize(&mut self, handle: &str, x: f64, y: f64) -> bool {
        self.inner.begin_selection_resize(
            handle,
            Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))),
        )
    }

    #[wasm_bindgen(js_name = updateSelectionResize)]
    pub fn update_selection_resize(&mut self, x: f64, y: f64) -> bool {
        self.inner
            .update_selection_resize(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
    }

    #[wasm_bindgen(js_name = finishSelectionResize)]
    pub fn finish_selection_resize(&mut self, x: f64, y: f64) -> bool {
        self.inner
            .finish_selection_resize(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
    }

    #[wasm_bindgen(js_name = applySelectionArrangeCommand)]
    pub fn apply_selection_arrange_command(&mut self, command: &str) -> bool {
        self.inner.apply_selection_arrange_command(command)
    }

    #[wasm_bindgen(js_name = scaleSelection)]
    pub fn scale_selection(&mut self, percent: f64) -> bool {
        self.inner.scale_selection(percent)
    }

    #[wasm_bindgen(js_name = rotateSelectionDegrees)]
    pub fn rotate_selection_degrees(&mut self, degrees: f64) -> bool {
        self.inner.rotate_selection_degrees(degrees)
    }

    #[wasm_bindgen(js_name = selectionNumericDialogJson)]
    pub fn selection_numeric_dialog_json(&self, kind: &str) -> String {
        self.inner.selection_numeric_dialog_json(kind)
    }

    #[wasm_bindgen(js_name = applySelectionNumericDialogJson)]
    pub fn apply_selection_numeric_dialog_json(
        &mut self,
        payload_json: &str,
    ) -> Result<bool, JsValue> {
        self.inner
            .apply_selection_numeric_dialog_json(payload_json)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = applySelectionOrderCommand)]
    pub fn apply_selection_order_command(&mut self, command: &str) -> bool {
        self.inner.apply_selection_order_command(command)
    }

    #[wasm_bindgen(js_name = groupSelection)]
    pub fn group_selection(&mut self) -> bool {
        self.inner.group_selection()
    }

    #[wasm_bindgen(js_name = ungroupSelection)]
    pub fn ungroup_selection(&mut self) -> bool {
        self.inner.ungroup_selection()
    }

    #[wasm_bindgen(js_name = joinSelection)]
    pub fn join_selection(&mut self) -> bool {
        self.inner.join_selection()
    }

    #[wasm_bindgen(js_name = applyColorToSelection)]
    pub fn apply_color_to_selection(&mut self, color: &str) -> bool {
        self.inner.apply_color_to_selection(color)
    }

    #[wasm_bindgen(js_name = applyShapeStyleToSelection)]
    pub fn apply_shape_style_to_selection(&mut self, style: &str) -> bool {
        self.inner.apply_shape_style_to_selection(style)
    }

    #[wasm_bindgen(js_name = applyOrbitalTemplateToSelection)]
    pub fn apply_orbital_template_to_selection(&mut self, template: &str) -> bool {
        self.inner.apply_orbital_template_to_selection(template)
    }

    #[wasm_bindgen(js_name = applyOrbitalStyleToSelection)]
    pub fn apply_orbital_style_to_selection(&mut self, style: &str) -> bool {
        self.inner.apply_orbital_style_to_selection(style)
    }

    #[wasm_bindgen(js_name = applyOrbitalPhaseToSelection)]
    pub fn apply_orbital_phase_to_selection(&mut self, phase: &str) -> bool {
        self.inner.apply_orbital_phase_to_selection(phase)
    }

    #[wasm_bindgen(js_name = applyBracketKindToSelection)]
    pub fn apply_bracket_kind_to_selection(&mut self, kind: &str) -> bool {
        self.inner.apply_bracket_kind_to_selection(kind)
    }

    #[wasm_bindgen(js_name = applyLineStyleToSelection)]
    pub fn apply_line_style_to_selection(&mut self, style: &str) -> bool {
        self.inner.apply_line_style_to_selection(style)
    }

    #[wasm_bindgen(js_name = applyBondStyleToSelection)]
    pub fn apply_bond_style_to_selection(&mut self, style: &str) -> bool {
        self.inner.apply_bond_style_to_selection(style)
    }

    #[wasm_bindgen(js_name = applyHoveredBondStyle)]
    pub fn apply_hovered_bond_style(&mut self, style: &str) -> bool {
        self.inner.apply_hovered_bond_style(style)
    }

    #[wasm_bindgen(js_name = applyTextStyleToSelection)]
    pub fn apply_text_style_to_selection(&mut self, command: &str, value: &str) -> bool {
        self.inner.apply_text_style_to_selection(command, value)
    }

    #[wasm_bindgen(js_name = setChemicalCheckForSelection)]
    pub fn set_chemical_check_for_selection(&mut self, enabled: bool) -> bool {
        self.inner.set_chemical_check_for_selection(enabled)
    }

    #[wasm_bindgen(js_name = expandLabelsInSelection)]
    pub fn expand_labels_in_selection(&mut self) -> bool {
        self.inner.expand_labels_in_selection()
    }

    #[wasm_bindgen(js_name = centerSelectionOnPage)]
    pub fn center_selection_on_page(&mut self) -> bool {
        self.inner.center_selection_on_page()
    }

    #[wasm_bindgen(js_name = executeCommandJson)]
    pub fn execute_command_json(&mut self, command_json: &str) -> Result<String, JsValue> {
        self.inner
            .execute_command_json(command_json)
            .map_err(|error| JsValue::from_str(&error))
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

    #[wasm_bindgen(js_name = loadDocumentCdxml)]
    pub fn load_document_cdxml(&mut self, cdxml: &str) -> Result<(), JsValue> {
        self.inner
            .load_cdxml_document(cdxml)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = loadDocumentCdx)]
    pub fn load_document_cdx(&mut self, cdx: &[u8]) -> Result<(), JsValue> {
        self.inner
            .load_cdx_document(cdx)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = loadDocumentSdf)]
    pub fn load_document_sdf(&mut self, sdf: &str) -> Result<(), JsValue> {
        self.inner
            .load_sdf_document(sdf)
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

    #[wasm_bindgen(js_name = copySelection)]
    pub fn copy_selection(&mut self) -> bool {
        self.inner.copy_selection()
    }

    #[wasm_bindgen(js_name = hasClipboard)]
    pub fn has_clipboard(&self) -> bool {
        self.inner.has_clipboard()
    }

    #[wasm_bindgen(js_name = clipboardSelectionJson)]
    pub fn clipboard_selection_json(&self) -> Result<Option<String>, JsValue> {
        self.inner
            .clipboard_selection_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = clipboardDocumentJson)]
    pub fn clipboard_document_json(&self) -> Result<Option<String>, JsValue> {
        self.inner
            .clipboard_document_json()
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = cutSelection)]
    pub fn cut_selection(&mut self) -> bool {
        self.inner.cut_selection()
    }

    #[wasm_bindgen(js_name = pasteClipboard)]
    pub fn paste_clipboard(&mut self) -> bool {
        self.inner.paste_clipboard()
    }

    #[wasm_bindgen(js_name = pasteClipboardJson)]
    pub fn paste_clipboard_json(&mut self, json: &str) -> Result<bool, JsValue> {
        self.inner
            .paste_clipboard_json(json)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = replaceHoveredEndpointLabel)]
    pub fn replace_hovered_endpoint_label(&mut self, label: &str) -> bool {
        self.inner.replace_hovered_endpoint_label(label)
    }

    #[wasm_bindgen(js_name = beginTextEdit)]
    pub fn begin_text_edit(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let session = self
            .inner
            .begin_text_edit(Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y))))
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

    #[wasm_bindgen(js_name = previewTextEditLayout)]
    pub fn preview_text_edit_layout(&self, request_json: &str) -> Result<String, JsValue> {
        let request: crate::TextEditLayoutRequest = serde_json::from_str(request_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        serde_json::to_string(&self.inner.preview_text_edit_layout(&request))
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

    #[wasm_bindgen(js_name = revision)]
    pub fn revision(&self) -> u64 {
        self.inner.revision()
    }

    #[wasm_bindgen(js_name = lastCommandResultJson)]
    pub fn last_command_result_json(&self) -> Result<String, JsValue> {
        self.inner
            .last_command_result_json()
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = historyJson)]
    pub fn history_json(&self) -> Result<String, JsValue> {
        self.inner
            .history_json()
            .map_err(|error| JsValue::from_str(&error.to_string()))
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

    #[wasm_bindgen(js_name = documentCdxml)]
    pub fn document_cdxml(&self) -> String {
        self.inner.document_cdxml()
    }

    #[wasm_bindgen(js_name = documentCdx)]
    pub fn document_cdx(&self) -> Result<Vec<u8>, JsValue> {
        self.inner
            .document_cdx()
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = documentSdf)]
    pub fn document_sdf(&self) -> Result<String, JsValue> {
        self.inner
            .document_sdf()
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = documentSvg)]
    pub fn document_svg(&self) -> String {
        self.inner.document_svg()
    }

    #[wasm_bindgen(js_name = documentColorsJson)]
    pub fn document_colors_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.document_colors())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = renderListJson)]
    pub fn render_list_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.render_list())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = interactionRenderListJson)]
    pub fn interaction_render_list_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.interaction_render_list())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen(js_name = renderBoundsJson)]
    pub fn render_bounds_json(&self, scope: &str) -> String {
        let scope = match scope {
            "document" => RenderBoundsScope::Document,
            "selection" => RenderBoundsScope::Selection,
            _ => RenderBoundsScope::All,
        };
        match self.inner.render_bounds(scope) {
            Some([min_x, min_y, max_x, max_y]) => serde_json::json!({
                "minX": min_x,
                "minY": min_y,
                "maxX": max_x,
                "maxY": max_y,
            })
            .to_string(),
            None => "null".to_string(),
        }
    }

    #[wasm_bindgen(js_name = selectionBoundsJson)]
    pub fn selection_bounds_json(&self) -> String {
        match self.inner.selection_bounds() {
            Some([min_x, min_y, max_x, max_y]) => serde_json::json!({
                "minX": min_x,
                "minY": min_y,
                "maxX": max_x,
                "maxY": max_y,
            })
            .to_string(),
            None => "null".to_string(),
        }
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
        "arrow" => Tool::Arrow,
        "bracket" => Tool::Bracket,
        "symbol" => Tool::Symbol,
        "element" => Tool::Element,
        "delete" => Tool::Delete,
        "text" => Tool::Text,
        "shape" => Tool::Shape,
        "tlc-plate" | "tlcPlate" => Tool::TlcPlate,
        "orbital" => Tool::Orbital,
        "templates" | "chain" => Tool::Templates,
        _ => Tool::Select,
    }
}

fn parse_orbital_template(value: &str) -> OrbitalTemplate {
    match value {
        "p" => OrbitalTemplate::P,
        "dxy" => OrbitalTemplate::Dxy,
        "oval" => OrbitalTemplate::Oval,
        "hybrid" => OrbitalTemplate::Hybrid,
        "dz2" => OrbitalTemplate::Dz2,
        "lobe" => OrbitalTemplate::Lobe,
        _ => OrbitalTemplate::S,
    }
}

fn parse_orbital_style(value: &str) -> OrbitalStyle {
    match value {
        "filled" => OrbitalStyle::Filled,
        "shaded" => OrbitalStyle::Shaded,
        _ => OrbitalStyle::Hollow,
    }
}

fn parse_orbital_phase(value: &str) -> OrbitalPhase {
    match value {
        "minus" => OrbitalPhase::Minus,
        _ => OrbitalPhase::Plus,
    }
}

fn parse_bracket_kind(value: &str) -> BracketKind {
    match value {
        "square" => BracketKind::Square,
        "curly" => BracketKind::Curly,
        "double-dagger" | "doubleDagger" => BracketKind::DoubleDagger,
        "dagger" => BracketKind::Dagger,
        "circle-plus" | "circlePlus" => BracketKind::CirclePlus,
        "plus" => BracketKind::Plus,
        "radical-cation" | "radicalCation" => BracketKind::RadicalCation,
        "lone-pair" | "lonePair" => BracketKind::LonePair,
        "circle-minus" | "circleMinus" => BracketKind::CircleMinus,
        "minus" => BracketKind::Minus,
        "radical-anion" | "radicalAnion" => BracketKind::RadicalAnion,
        "electron" => BracketKind::Electron,
        _ => BracketKind::Round,
    }
}

fn parse_arrow_variant(value: &str) -> ArrowVariant {
    match value {
        "curved" => ArrowVariant::Curved,
        "curved-mirror" => ArrowVariant::CurvedMirror,
        "hollow" => ArrowVariant::Hollow,
        "open" => ArrowVariant::Open,
        "equilibrium" => ArrowVariant::Equilibrium,
        "unequal-equilibrium" => ArrowVariant::UnequalEquilibrium,
        _ => ArrowVariant::Solid,
    }
}

fn parse_shape_kind(value: &str) -> ShapeKind {
    match value {
        "ellipse" => ShapeKind::Ellipse,
        "round-rect" | "roundRect" => ShapeKind::RoundRect,
        "rect" => ShapeKind::Rect,
        "cross-table" | "crossTable" => ShapeKind::CrossTable,
        "tlc-plate" | "tlcPlate" => ShapeKind::TlcPlate,
        _ => ShapeKind::Circle,
    }
}

fn parse_shape_style(value: &str) -> ShapeStyle {
    match value {
        "dashed" => ShapeStyle::Dashed,
        "shaded" => ShapeStyle::Shaded,
        "filled" => ShapeStyle::Filled,
        "shadowed" | "shadow" => ShapeStyle::Shadowed,
        _ => ShapeStyle::Solid,
    }
}

fn parse_arrow_curve(value: &str) -> ArrowCurve {
    match value {
        "180" | "arc-180" | "arc180" => ArrowCurve::Arc180,
        "120" | "arc-120" | "arc120" => ArrowCurve::Arc120,
        "90" | "arc-90" | "arc90" => ArrowCurve::Arc90,
        _ => ArrowCurve::Arc270,
    }
}

fn parse_arrow_head_size(value: &str) -> ArrowHeadSize {
    match value {
        "large" => ArrowHeadSize::Large,
        "medium" => ArrowHeadSize::Medium,
        "small" => ArrowHeadSize::Small,
        _ => ArrowHeadSize::Small,
    }
}

fn parse_arrow_endpoint_style(value: &str) -> ArrowEndpointStyle {
    match value {
        "full" => ArrowEndpointStyle::Full,
        "left" | "top" | "half-left" => ArrowEndpointStyle::Left,
        "right" | "bottom" | "half-right" => ArrowEndpointStyle::Right,
        _ => ArrowEndpointStyle::None,
    }
}

fn parse_arrow_no_go(value: &str) -> ArrowNoGo {
    match value {
        "cross" => ArrowNoGo::Cross,
        "hash" => ArrowNoGo::Hash,
        _ => ArrowNoGo::None,
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
        "wavy" => BondVariant::Wavy,
        "wedge" => BondVariant::Wedge,
        "hashed-wedge" => BondVariant::HashedWedge,
        "hollow-wedge" => BondVariant::HollowWedge,
        _ => BondVariant::Single,
    }
}
