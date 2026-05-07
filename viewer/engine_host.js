import initializeChemcoreEngine, { WasmEngine } from "./engine/chemcore_engine.js";

class WasmEngineHost {
  constructor() {
    this.kind = "wasm";
    this.native = false;
  }

  async initialize() {
    await initializeChemcoreEngine();
    return this;
  }

  createEngineSession() {
    return new WasmEngine();
  }
}

class DesktopHybridEngineHost extends WasmEngineHost {
  constructor() {
    super();
    this.kind = "desktop-hybrid";
    this.desktopNative = new TauriEngineHost();
    this.desktopNativeProbe = null;
  }

  async initialize() {
    await super.initialize();
    try {
      await this.desktopNative.initialize();
      this.desktopNativeProbe = await this.desktopNative.runSmokeTest();
      console.info("[chemcore] desktop native engine probe", this.desktopNativeProbe);
    } catch (error) {
      this.desktopNativeProbe = {
        ok: false,
        error: String(error?.message || error),
      };
      console.warn("[chemcore] desktop native engine probe failed", error);
    }
    return this;
  }
}

class TauriEngineSession {
  constructor(invoke, options = {}) {
    this.invoke = invoke;
    this.sessionId = options.sessionId || null;
    this.layoutEngine = options.layoutEngine || null;
    this.cache = {
      documentJson: null,
      stateJson: null,
      renderListJson: "[]",
      renderBoundsJson: new Map(),
      documentColorsJson: "[]",
      documentStylePreset: "default",
      canUndo: false,
      canRedo: false,
      documentCdxml: null,
      documentSvg: null,
    };
    this.exportDirty = true;
    this.operation = Promise.resolve();
    this.readyPromise = this.initializeSession();
  }

  async initializeSession() {
    if (!this.sessionId) {
      this.sessionId = await this.invoke("desktop_engine_create");
    }
    await this.refreshRenderState();
    return this;
  }

  ready() {
    return this.readyPromise;
  }

  async free() {
    await this.ready();
    return this.invoke("desktop_engine_free", { sessionId: this.sessionId });
  }

  async invokeMutation(command, args = {}, options = {}) {
    const refresh = options.refresh ?? "render";
    const run = async () => {
      await this.ready();
      const result = await this.invoke(command, { sessionId: this.sessionId, ...args });
      if (refresh === "all") {
        await this.refreshAll();
      } else if (refresh === "render") {
        await this.refreshRenderState();
      } else if (refresh === "exports") {
        this.exportDirty = true;
      }
      return result;
    };
    const next = this.operation.catch(() => {}).then(run);
    this.operation = next;
    return next;
  }

  async refreshRenderState() {
    const [
      documentJson,
      stateJson,
      renderListJson,
      allBoundsJson,
      documentBoundsJson,
      selectionBoundsJson,
      documentColorsJson,
      documentStylePreset,
      canUndo,
      canRedo,
    ] = await Promise.all([
      this.invoke("desktop_engine_document_json", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_state_json", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_render_list_json", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_render_bounds_json", { sessionId: this.sessionId, scope: "all" }),
      this.invoke("desktop_engine_render_bounds_json", { sessionId: this.sessionId, scope: "document" }),
      this.invoke("desktop_engine_render_bounds_json", { sessionId: this.sessionId, scope: "selection" }),
      this.invoke("desktop_engine_document_colors_json", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_document_style_preset", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_can_undo", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_can_redo", { sessionId: this.sessionId }),
    ]);
    this.cache.documentJson = documentJson;
    this.cache.stateJson = stateJson;
    this.cache.renderListJson = renderListJson;
    this.cache.renderBoundsJson.set("all", allBoundsJson);
    this.cache.renderBoundsJson.set("document", documentBoundsJson);
    this.cache.renderBoundsJson.set("selection", selectionBoundsJson);
    this.cache.documentColorsJson = documentColorsJson;
    this.cache.documentStylePreset = documentStylePreset;
    this.cache.canUndo = Boolean(canUndo);
    this.cache.canRedo = Boolean(canRedo);
    this.exportDirty = true;
    return this;
  }

  async refreshExports() {
    if (!this.exportDirty && this.cache.documentCdxml != null && this.cache.documentSvg != null) {
      return this;
    }
    await this.ready();
    const [documentCdxml, documentSvg] = await Promise.all([
      this.invoke("desktop_engine_document_cdxml", { sessionId: this.sessionId }),
      this.invoke("desktop_engine_document_svg", { sessionId: this.sessionId }),
    ]);
    this.cache.documentCdxml = documentCdxml;
    this.cache.documentSvg = documentSvg;
    this.exportDirty = false;
    return this;
  }

  async refreshAll() {
    await this.refreshRenderState();
    await this.refreshExports();
    return this;
  }

  async loadDocumentJson(json) {
    if (this.layoutEngine?.loadDocumentJson) {
      this.layoutEngine.loadDocumentJson(json);
    }
    return this.invokeMutation("desktop_engine_load_document_json", { json }, { refresh: "all" });
  }

  async loadDocumentCdxml(cdxml) {
    const result = await this.invokeMutation("desktop_engine_load_document_cdxml", { cdxml }, { refresh: "all" });
    if (this.layoutEngine?.loadDocumentJson) {
      this.layoutEngine.loadDocumentJson(this.cache.documentJson);
    }
    return result;
  }

  documentJson() {
    return this.cache.documentJson || "";
  }

  stateJson() {
    return this.cache.stateJson || "";
  }

  renderListJson() {
    return this.cache.renderListJson || "[]";
  }

  renderBoundsJson(scope = "all") {
    return this.cache.renderBoundsJson.get(scope) || this.cache.renderBoundsJson.get("all") || "null";
  }

  async documentCdxml() {
    await this.refreshExports();
    return this.cache.documentCdxml || "";
  }

  async documentSvg() {
    await this.refreshExports();
    return this.cache.documentSvg || "";
  }

  documentColorsJson() {
    return this.cache.documentColorsJson || "[]";
  }

  setTool(activeTool, bondVariant) {
    return this.invokeMutation("desktop_engine_set_tool", { activeTool, bondVariant });
  }

  setShapeOptions(kind, style, color) {
    return this.invokeMutation("desktop_engine_set_shape_options", { kind, style, color });
  }

  setTemplate(template) {
    return this.invokeMutation("desktop_engine_set_template", { template });
  }

  setBracketOptions(kind) {
    return this.invokeMutation("desktop_engine_set_bracket_options", { kind });
  }

  setSymbolOptions(kind) {
    return this.invokeMutation("desktop_engine_set_symbol_options", { kind });
  }

  setDocumentStylePreset(preset) {
    if (this.layoutEngine?.setDocumentStylePreset) {
      this.layoutEngine.setDocumentStylePreset(preset);
    }
    return this.invokeMutation("desktop_engine_set_document_style_preset", { preset }, { refresh: "all" });
  }

  documentStylePreset() {
    return this.cache.documentStylePreset || "default";
  }

  setArrowOptions(variant, headSize, head, tail, bold) {
    return this.invokeMutation("desktop_engine_set_arrow_options", {
      variant,
      headSize,
      head,
      tail,
      bold,
    });
  }

  setArrowEndpointOptions(variant, headSize, curve, headStyle, tailStyle, noGo, bold) {
    return this.invokeMutation("desktop_engine_set_arrow_endpoint_options", {
      variant,
      headSize,
      curve,
      headStyle,
      tailStyle,
      noGo,
      bold,
    });
  }

  applyArrowOptionsToSelection(variant, headSize, head, tail, bold) {
    return this.invokeMutation("desktop_engine_apply_arrow_options_to_selection", {
      variant,
      headSize,
      head,
      tail,
      bold,
    });
  }

  applyArrowEndpointOptionsToSelection(variant, headSize, curve, headStyle, tailStyle, noGo, bold) {
    return this.invokeMutation("desktop_engine_apply_arrow_endpoint_options_to_selection", {
      variant,
      headSize,
      curve,
      headStyle,
      tailStyle,
      noGo,
      bold,
    });
  }

  pointerMove(x, y, altKey) {
    return this.invokeMutation("desktop_engine_pointer_move", { x, y, altKey });
  }

  pointerDown(x, y, altKey) {
    return this.invokeMutation("desktop_engine_pointer_down", { x, y, altKey });
  }

  pointerUp(x, y, altKey) {
    return this.invokeMutation("desktop_engine_pointer_up", { x, y, altKey });
  }

  selectAtPoint(x, y, additive) {
    return this.invokeMutation("desktop_engine_select_at_point", { x, y, additive });
  }

  selectComponentAtPoint(x, y, additive) {
    return this.invokeMutation("desktop_engine_select_component_at_point", { x, y, additive });
  }

  selectInRect(x1, y1, x2, y2, additive) {
    return this.invokeMutation("desktop_engine_select_in_rect", { x1, y1, x2, y2, additive });
  }

  selectInPolygon(pointsJson, additive) {
    return this.invokeMutation("desktop_engine_select_in_polygon", { pointsJson, additive });
  }

  selectAll() {
    return this.invokeMutation("desktop_engine_select_all");
  }

  clearSelection() {
    return this.invokeMutation("desktop_engine_clear_selection");
  }

  async contextHitTestJson(x, y) {
    await this.ready();
    return this.invoke("desktop_engine_context_hit_test_json", { sessionId: this.sessionId, x, y });
  }

  selectionContainsPoint(x, y) {
    const state = safeJsonParse(this.cache.stateJson, null);
    if (!state?.selection) {
      return false;
    }
    const bounds = safeJsonParse(this.renderBoundsJson("selection"), null);
    return Boolean(bounds && x >= bounds.minX && x <= bounds.maxX && y >= bounds.minY && y <= bounds.maxY);
  }

  hoverArrowAction(x, y) {
    return this.invoke("desktop_engine_hover_arrow_action", { sessionId: this.sessionId, x, y });
  }

  beginHoverArrowEdit(x, y) {
    return this.invokeMutation("desktop_engine_begin_hover_arrow_edit", { x, y });
  }

  updateHoverArrowEdit(x, y, altKey) {
    return this.invokeMutation("desktop_engine_update_hover_arrow_edit", { x, y, altKey });
  }

  finishHoverArrowEdit(x, y, altKey) {
    return this.invokeMutation("desktop_engine_finish_hover_arrow_edit", { x, y, altKey });
  }

  hoverShapeAction(x, y) {
    return this.invoke("desktop_engine_hover_shape_action", { sessionId: this.sessionId, x, y });
  }

  beginHoverShapeEdit(x, y) {
    return this.invokeMutation("desktop_engine_begin_hover_shape_edit", { x, y });
  }

  updateHoverShapeEdit(x, y, altKey) {
    return this.invokeMutation("desktop_engine_update_hover_shape_edit", { x, y, altKey });
  }

  finishHoverShapeEdit(x, y, altKey) {
    return this.invokeMutation("desktop_engine_finish_hover_shape_edit", { x, y, altKey });
  }

  activeArrowEditDegrees() {
    return 0;
  }

  beginSelectionMove(x, y, additive, altKey) {
    return this.invokeMutation("desktop_engine_begin_selection_move", { x, y, additive, altKey });
  }

  updateSelectionMove(x, y, altKey) {
    return this.invokeMutation("desktop_engine_update_selection_move", { x, y, altKey });
  }

  finishSelectionMove(x, y, altKey) {
    return this.invokeMutation("desktop_engine_finish_selection_move", { x, y, altKey });
  }

  beginSelectionRotate(x, y) {
    return this.invokeMutation("desktop_engine_begin_selection_rotate", { x, y });
  }

  updateSelectionRotate(x, y, altKey) {
    return this.invokeMutation("desktop_engine_update_selection_rotate", { x, y, altKey });
  }

  finishSelectionRotate(x, y, altKey) {
    return this.invokeMutation("desktop_engine_finish_selection_rotate", { x, y, altKey });
  }

  beginSelectionResize(handle, x, y) {
    return this.invokeMutation("desktop_engine_begin_selection_resize", { handle, x, y });
  }

  updateSelectionResize(x, y) {
    return this.invokeMutation("desktop_engine_update_selection_resize", { x, y });
  }

  finishSelectionResize(x, y) {
    return this.invokeMutation("desktop_engine_finish_selection_resize", { x, y });
  }

  applySelectionArrangeCommand(command) {
    return this.invokeMutation("desktop_engine_apply_selection_arrange_command", { command });
  }

  scaleSelection(percent) {
    return this.invokeMutation("desktop_engine_scale_selection", { percent });
  }

  rotateSelectionDegrees(degrees) {
    return this.invokeMutation("desktop_engine_rotate_selection_degrees", { degrees });
  }

  applySelectionOrderCommand(command) {
    return this.invokeMutation("desktop_engine_apply_selection_order_command", { command });
  }

  groupSelection() {
    return this.invokeMutation("desktop_engine_group_selection");
  }

  ungroupSelection() {
    return this.invokeMutation("desktop_engine_ungroup_selection");
  }

  applyColorToSelection(color) {
    return this.invokeMutation("desktop_engine_apply_color_to_selection", { color });
  }

  applyShapeStyleToSelection(style) {
    return this.invokeMutation("desktop_engine_apply_shape_style_to_selection", { style });
  }

  applyBracketKindToSelection(kind) {
    return this.invokeMutation("desktop_engine_apply_bracket_kind_to_selection", { kind });
  }

  applyLineStyleToSelection(style) {
    return this.invokeMutation("desktop_engine_apply_line_style_to_selection", { style });
  }

  applyBondStyleToSelection(style) {
    return this.invokeMutation("desktop_engine_apply_bond_style_to_selection", { style });
  }

  applyTextStyleToSelection(command, value) {
    return this.invokeMutation("desktop_engine_apply_text_style_to_selection", { command, value });
  }

  setChemicalCheckForSelection(enabled) {
    return this.invokeMutation("desktop_engine_set_chemical_check_for_selection", { enabled });
  }

  expandLabelsInSelection() {
    return this.invokeMutation("desktop_engine_expand_labels_in_selection");
  }

  centerSelectionOnPage() {
    return this.invokeMutation("desktop_engine_center_selection_on_page");
  }

  clearInteraction() {
    return this.invokeMutation("desktop_engine_clear_interaction");
  }

  undo() {
    return this.invokeMutation("desktop_engine_undo");
  }

  redo() {
    return this.invokeMutation("desktop_engine_redo");
  }

  canUndo() {
    return this.cache.canUndo;
  }

  canRedo() {
    return this.cache.canRedo;
  }

  deleteSelection() {
    return this.invokeMutation("desktop_engine_delete_selection");
  }

  copySelection() {
    return this.invokeMutation("desktop_engine_copy_selection");
  }

  async hasClipboard() {
    await this.ready();
    return this.invoke("desktop_engine_has_clipboard", { sessionId: this.sessionId });
  }

  async clipboardSelectionJson() {
    await this.ready();
    return this.invoke("desktop_engine_clipboard_selection_json", { sessionId: this.sessionId });
  }

  cutSelection() {
    return this.invokeMutation("desktop_engine_cut_selection");
  }

  pasteClipboard() {
    return this.invokeMutation("desktop_engine_paste_clipboard");
  }

  pasteClipboardJson(json) {
    return this.invokeMutation("desktop_engine_paste_clipboard_json", { json });
  }

  replaceHoveredEndpointLabel(label) {
    return this.invokeMutation("desktop_engine_replace_hovered_endpoint_label", { label });
  }

  beginTextEdit(x, y) {
    return this.invokeMutation("desktop_engine_begin_text_edit", { x, y });
  }

  applyTextEdit(sessionJson) {
    return this.invokeMutation("desktop_engine_apply_text_edit", { sessionJson }, { refresh: "all" });
  }

  previewTextRuns(sessionJson) {
    return this.layoutEngine?.previewTextRuns?.(sessionJson) || null;
  }

  previewTextEditLayout(requestJson) {
    return this.layoutEngine?.previewTextEditLayout?.(requestJson) || null;
  }
}

class TauriEngineHost {
  constructor() {
    this.kind = "tauri";
    this.native = true;
    this.invoke = null;
  }

  async initialize() {
    const invoke = globalThis.__TAURI__?.core?.invoke;
    if (typeof invoke !== "function") {
      throw new Error("Tauri invoke API is unavailable.");
    }
    await initializeChemcoreEngine();
    this.invoke = invoke;
    return this;
  }

  createEngineSession() {
    return new TauriEngineSession(this.invoke, {
      layoutEngine: new WasmEngine(),
    });
  }

  async runSmokeTest() {
    const session = await this.createEngineSession();
    try {
      await session.ready();
      const documentJson = session.documentJson();
      const renderListJson = session.renderListJson();
      const renderBoundsJson = session.renderBoundsJson("all");
      const documentSvg = await session.documentSvg();
      const document = JSON.parse(documentJson);
      const renderList = JSON.parse(renderListJson);
      JSON.parse(renderBoundsJson);
      return {
        ok: true,
        sessionId: session.sessionId,
        title: document?.document?.title || null,
        renderPrimitiveCount: Array.isArray(renderList) ? renderList.length : null,
        svgBytes: documentSvg.length,
      };
    } finally {
      await session.free();
    }
  }
}

export function detectEngineHostKind() {
  const engineOverride = new URL(globalThis.location?.href || "http://localhost/").searchParams.get("engine");
  if (engineOverride === "tauri-native") {
    return "tauri-native";
  }
  if (engineOverride === "desktop-hybrid") {
    return "tauri";
  }
  return globalThis.__TAURI_INTERNALS__ ? "tauri-native" : "wasm";
}

export function createEngineHost(kind = detectEngineHostKind()) {
  if (kind === "tauri-native") {
    return new TauriEngineHost();
  }
  if (kind === "tauri") {
    return new DesktopHybridEngineHost();
  }
  return new WasmEngineHost();
}

function safeJsonParse(json, fallback = null) {
  try {
    return JSON.parse(json);
  } catch {
    return fallback;
  }
}
