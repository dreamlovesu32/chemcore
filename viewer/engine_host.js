import initializeChemcoreEngine, { WasmEngine } from "./engine/chemcore_engine.js";

class WasmEngineHost {
  constructor() {
    this.kind = "wasm";
    this.native = false;
  }

  async initialize() {
    await initializeChemcoreEngine(new URL("./engine/chemcore_engine_bg.wasm?v=20260627-bracket-hit-cursor", import.meta.url));
    return this;
  }

  createEngineSession() {
    return createManagedEngineSession(new WasmEngine());
  }
}

function createManagedEngineSession(session) {
  if (!session || typeof session !== "object" || session.__chemcoreManagedFree) {
    return session;
  }
  const originalFree = typeof session.free === "function" ? session.free.bind(session) : null;
  if (!originalFree) {
    return session;
  }
  let freed = false;
  Object.defineProperty(session, "__chemcoreManagedFree", {
    configurable: false,
    enumerable: false,
    value: true,
  });
  session.free = async () => {
    if (freed) {
      return false;
    }
    freed = true;
    return originalFree();
  };
  return session;
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
      interactionRenderListJson: "[]",
      renderBoundsJson: new Map(),
      selectionChemistrySummaryJson: "null",
      documentColorsJson: "[]",
      documentStylePreset: "default",
      revision: 0,
      lastCommandResultJson: "null",
      canUndo: false,
      canRedo: false,
      documentCdxml: null,
      documentCdx: null,
      documentSdf: null,
      documentSvg: null,
    };
    this.exportDirty = true;
    this.operation = Promise.resolve();
    this.nativeBackgroundOperation = Promise.resolve();
    this.coalescedNativeMutations = new Map();
    this.pendingSelectionMoveBegin = null;
    this.localSelectionMoveActive = false;
    this.pendingSelectionRotateBegin = null;
    this.localSelectionRotateActive = false;
    this.pendingSelectionResizeBegin = null;
    this.localSelectionResizeActive = false;
    this.pendingArrowEditBegin = null;
    this.localArrowEditActive = false;
    this.pendingShapeEditBegin = null;
    this.localShapeEditActive = false;
    this.activeTool = "";
    this.readyPromise = this.initializeSession();
  }

  async initializeSession() {
    if (!this.sessionId) {
      this.sessionId = await this.invoke("desktop_engine_create");
    }
    await this.refreshSnapshot("document");
    return this;
  }

  ready() {
    return this.readyPromise;
  }

  async free() {
    try {
      await this.ready();
      return await this.invoke("desktop_engine_free", { sessionId: this.sessionId });
    } finally {
      await this.layoutEngine?.free?.();
      this.layoutEngine = null;
    }
  }

  async invokeMutation(command, args = {}, options = {}) {
    const refresh = options.refresh ?? "document";
    const dirtyExports = options.dirtyExports ?? (refresh === "all" || refresh === "document");
    const run = async () => {
      await this.ready();
      await this.nativeBackgroundOperation.catch(() => {});
      const result = await this.invoke(command, { sessionId: this.sessionId, ...args });
      if (dirtyExports) {
        this.markExportsDirty();
      }
      if (refresh === "all" || refresh === "document") {
        await this.refreshSnapshot("document");
      } else if (refresh === "documentState") {
        await this.refreshSnapshot("documentState");
      } else if (refresh === "selection") {
        await this.refreshSnapshot("selection");
      } else if (refresh === "interaction") {
        await this.refreshSnapshot("interaction");
      } else if (refresh === "state") {
        await this.refreshSnapshot("state");
      } else if (refresh === "exports") {
        this.markExportsDirty();
      }
      return result;
    };
    const next = this.operation.catch(() => {}).then(run);
    this.operation = next;
    return next;
  }

  markExportsDirty() {
    this.exportDirty = true;
    this.cache.documentCdxml = null;
    this.cache.documentCdx = null;
    this.cache.documentSdf = null;
    this.cache.documentSvg = null;
  }

  hasLocalInteraction() {
    return this.localSelectionMoveActive
      || this.localSelectionRotateActive
      || this.localSelectionResizeActive
      || this.localArrowEditActive
      || this.localShapeEditActive;
  }

  syncCacheFromLayout({ document = false, interaction = false } = {}) {
    if (!this.layoutEngine) {
      return;
    }
    if (document && this.layoutEngine.documentJson) {
      this.cache.documentJson = this.layoutEngine.documentJson();
    }
    if (this.layoutEngine.stateJson) {
      this.cache.stateJson = this.layoutEngine.stateJson();
    }
    if (interaction && this.layoutEngine.interactionRenderListJson) {
      this.cache.interactionRenderListJson = this.layoutEngine.interactionRenderListJson();
    }
    if (this.layoutEngine.lastCommandResultJson) {
      this.cache.lastCommandResultJson = this.layoutEngine.lastCommandResultJson();
    }
    if (this.layoutEngine.revision) {
      this.cache.revision = Number(this.layoutEngine.revision());
    }
    if (this.layoutEngine.canUndo) {
      this.cache.canUndo = !!this.layoutEngine.canUndo();
    }
    if (this.layoutEngine.canRedo) {
      this.cache.canRedo = !!this.layoutEngine.canRedo();
    }
  }

  async executeNativeMutation(command, args = {}, options = {}) {
    const refresh = options.refresh ?? "document";
    const dirtyExports = options.dirtyExports ?? (refresh === "all" || refresh === "document" || refresh === "documentState");
    const waitFor = options.waitFor || null;
    if (waitFor) {
      await waitFor.catch(() => false);
    }
    await this.ready();
    const result = await this.invoke(command, { sessionId: this.sessionId, ...args });
    if (dirtyExports) {
      this.markExportsDirty();
    }
    if (!this.layoutEngine) {
      if (refresh === "all" || refresh === "document") {
        await this.refreshSnapshot("document");
      } else if (refresh === "documentState") {
        await this.refreshSnapshot("documentState");
      } else if (refresh === "selection") {
        await this.refreshSnapshot("selection");
      } else if (refresh === "interaction") {
        await this.refreshSnapshot("interaction");
      } else if (refresh === "state") {
        await this.refreshSnapshot("state");
      }
    }
    return result;
  }

  runNativeMutationInBackground(command, args = {}, options = {}) {
    const run = () => this.executeNativeMutation(command, args, options);
    this.nativeBackgroundOperation = this.nativeBackgroundOperation.catch(() => {}).then(run);
    void this.nativeBackgroundOperation.catch((error) => {
      console.warn("[chemcore] background native mutation failed", command, error);
    });
    return this.nativeBackgroundOperation;
  }

  runCoalescedNativeMutationInBackground(key, command, args = {}, options = {}) {
    const existing = this.coalescedNativeMutations.get(key);
    if (existing) {
      existing.command = command;
      existing.args = args;
      existing.options = options;
      return existing.promise;
    }
    const entry = {
      command,
      args,
      options,
      promise: null,
    };
    const run = async () => {
      const latest = this.coalescedNativeMutations.get(key) || entry;
      this.coalescedNativeMutations.delete(key);
      return this.executeNativeMutation(latest.command, latest.args, latest.options);
    };
    entry.promise = this.nativeBackgroundOperation.catch(() => {}).then(run);
    this.coalescedNativeMutations.set(key, entry);
    this.nativeBackgroundOperation = entry.promise;
    void entry.promise.catch((error) => {
      console.warn("[chemcore] coalesced background native mutation failed", command, error);
    });
    return entry.promise;
  }

  syncLocalMutationState({ dirtyExports = false } = {}) {
    if (dirtyExports) {
      this.markExportsDirty();
    }
    this.syncCacheFromLayout({ interaction: true });
  }

  activeToolUsesLocalPointer() {
    return [
      "bond",
      "arrow",
      "bracket",
      "symbol",
      "element",
      "shape",
      "tlc-plate",
      "orbital",
      "templates",
      "chain",
    ].includes(this.activeTool);
  }

  syncLayoutDocumentJson(json = this.cache.documentJson) {
    if (typeof json !== "string") {
      return;
    }
    if (this.layoutEngine?.loadDocumentJson) {
      this.layoutEngine.loadDocumentJson(json);
    }
  }

  applySnapshot(snapshot) {
    if (!snapshot || typeof snapshot !== "object") {
      return;
    }
    if (snapshot.documentJson != null) {
      this.cache.documentJson = snapshot.documentJson;
      this.syncLayoutDocumentJson(snapshot.documentJson);
    }
    if (snapshot.stateJson != null) {
      this.cache.stateJson = snapshot.stateJson;
    }
    if (snapshot.renderListJson != null) {
      this.cache.renderListJson = snapshot.renderListJson;
    }
    if (snapshot.interactionRenderListJson != null) {
      this.cache.interactionRenderListJson = snapshot.interactionRenderListJson;
    }
    if (snapshot.allBoundsJson != null) {
      this.cache.renderBoundsJson.set("all", snapshot.allBoundsJson);
    }
    if (snapshot.documentBoundsJson != null) {
      this.cache.renderBoundsJson.set("document", snapshot.documentBoundsJson);
    }
    if (snapshot.selectionBoundsJson != null) {
      this.cache.renderBoundsJson.set("selection", snapshot.selectionBoundsJson);
    }
    if (snapshot.selectionChemistrySummaryJson != null) {
      this.cache.selectionChemistrySummaryJson = snapshot.selectionChemistrySummaryJson;
    }
    if (snapshot.documentColorsJson != null) {
      this.cache.documentColorsJson = snapshot.documentColorsJson;
    }
    if (snapshot.documentStylePreset != null) {
      this.cache.documentStylePreset = snapshot.documentStylePreset;
    }
    if (snapshot.revision != null) {
      this.cache.revision = Number(snapshot.revision) || 0;
    }
    if (snapshot.lastCommandResultJson != null) {
      this.cache.lastCommandResultJson = snapshot.lastCommandResultJson;
    }
    if (snapshot.canUndo != null) {
      this.cache.canUndo = Boolean(snapshot.canUndo);
    }
    if (snapshot.canRedo != null) {
      this.cache.canRedo = Boolean(snapshot.canRedo);
    }
  }

  async refreshSnapshot(mode = "document") {
    const snapshotJson = await this.invoke("desktop_engine_snapshot_json", { sessionId: this.sessionId, mode });
    this.applySnapshot(safeJsonParse(snapshotJson, null));
    return this;
  }

  async refreshRenderState() {
    return this.refreshSnapshot("document");
  }

  async refreshExports() {
    if (!this.exportDirty && this.cache.documentCdxml != null && this.cache.documentSvg != null) {
      return this;
    }
    await this.ready();
    await this.nativeBackgroundOperation.catch(() => {});
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
    return this;
  }

  async loadDocumentJson(json) {
    this.syncLayoutDocumentJson(json);
    return this.invokeMutation("desktop_engine_load_document_json", { json }, { refresh: "document" });
  }

  async loadDocumentCdxml(cdxml) {
    const result = await this.invokeMutation("desktop_engine_load_document_cdxml", { cdxml }, { refresh: "document" });
    this.syncLayoutDocumentJson();
    return result;
  }

  async loadDocumentCdx(cdx) {
    if (this.layoutEngine?.loadDocumentCdx) {
      this.layoutEngine.loadDocumentCdx(cdx);
      const cdxml = this.layoutEngine.documentCdxml();
      return this.loadDocumentCdxml(cdxml);
    }
    throw new Error("CDX import is unavailable.");
  }

  async loadDocumentSdf(sdf) {
    const result = await this.invokeMutation("desktop_engine_load_document_sdf", { sdf }, { refresh: "document" });
    this.syncLayoutDocumentJson();
    return result;
  }

  documentJson() {
    if (this.layoutEngine?.documentJson) {
      return this.layoutEngine.documentJson();
    }
    return this.cache.documentJson || "";
  }

  stateJson() {
    if (this.hasLocalInteraction() && this.layoutEngine?.stateJson) {
      return this.layoutEngine.stateJson();
    }
    return this.cache.stateJson || "";
  }

  renderListJson() {
    return this.cache.renderListJson || "[]";
  }

  renderTargetsJson(requestJson = "{}") {
    return this.layoutEngine?.renderTargetsJson?.(requestJson) || "[]";
  }

  interactionRenderListJson() {
    if (this.hasLocalInteraction() && this.layoutEngine?.interactionRenderListJson) {
      return this.layoutEngine.interactionRenderListJson();
    }
    return this.cache.interactionRenderListJson || this.cache.renderListJson || "[]";
  }

  renderBoundsJson(scope = "all") {
    if (this.layoutEngine?.renderBoundsJson) {
      return this.layoutEngine.renderBoundsJson(scope);
    }
    return this.cache.renderBoundsJson.get(scope) || this.cache.renderBoundsJson.get("all") || "null";
  }

  selectionBoundsJson() {
    if (this.layoutEngine?.selectionBoundsJson) {
      return this.layoutEngine.selectionBoundsJson();
    }
    return this.renderBoundsJson("selection");
  }

  selectionChemistrySummaryJson() {
    return this.cache.selectionChemistrySummaryJson || "null";
  }

  async documentCdxml() {
    if (this.layoutEngine?.documentCdxml) {
      return this.layoutEngine.documentCdxml();
    }
    await this.refreshExports();
    return this.cache.documentCdxml || "";
  }

  async documentCdx() {
    if (this.layoutEngine?.documentCdx) {
      return this.layoutEngine.documentCdx();
    }
    throw new Error("CDX export is unavailable.");
  }

  async documentSdf() {
    if (this.layoutEngine?.documentSdf) {
      return this.layoutEngine.documentSdf();
    }
    await this.ready();
    return this.invoke("desktop_engine_document_sdf", { sessionId: this.sessionId });
  }

  async documentSvg() {
    if (this.layoutEngine?.documentSvg) {
      return this.layoutEngine.documentSvg();
    }
    await this.refreshExports();
    return this.cache.documentSvg || "";
  }

  documentColorsJson() {
    return this.cache.documentColorsJson || "[]";
  }

  setTool(activeTool, bondVariant) {
    this.activeTool = activeTool;
    if (this.layoutEngine?.setTool) {
      this.layoutEngine.setTool(activeTool, bondVariant);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_tool",
        "desktop_engine_set_tool",
        { activeTool, bondVariant },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_tool", { activeTool, bondVariant }, { refresh: "state", dirtyExports: false });
  }

  setShapeOptions(kind, style, color) {
    if (this.layoutEngine?.setShapeOptions) {
      this.layoutEngine.setShapeOptions(kind, style, color);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_shape_options",
        "desktop_engine_set_shape_options",
        { kind, style, color },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_shape_options", { kind, style, color }, { refresh: "state", dirtyExports: false });
  }

  setOrbitalOptions(template, style, phase, color) {
    if (this.layoutEngine?.setOrbitalOptions) {
      this.layoutEngine.setOrbitalOptions(template, style, phase, color);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_orbital_options",
        "desktop_engine_set_orbital_options",
        { template, style, phase, color },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_orbital_options", {
      template,
      style,
      phase,
      color,
    }, { refresh: "state", dirtyExports: false });
  }

  setTemplate(template) {
    if (this.layoutEngine?.setTemplate) {
      this.layoutEngine.setTemplate(template);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_template",
        "desktop_engine_set_template",
        { template },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_template", { template }, { refresh: "state", dirtyExports: false });
  }

  setBracketOptions(kind) {
    if (this.layoutEngine?.setBracketOptions) {
      this.layoutEngine.setBracketOptions(kind);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_bracket_options",
        "desktop_engine_set_bracket_options",
        { kind },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_bracket_options", { kind }, { refresh: "state", dirtyExports: false });
  }

  setSymbolOptions(kind) {
    if (this.layoutEngine?.setSymbolOptions) {
      this.layoutEngine.setSymbolOptions(kind);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_symbol_options",
        "desktop_engine_set_symbol_options",
        { kind },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_symbol_options", { kind }, { refresh: "state", dirtyExports: false });
  }

  setElementOptions(symbol, atomicNumber) {
    if (this.layoutEngine?.setElementOptions) {
      this.layoutEngine.setElementOptions(symbol, atomicNumber);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_element_options",
        "desktop_engine_set_element_options",
        { symbol, atomicNumber },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation(
      "desktop_engine_set_element_options",
      { symbol, atomicNumber },
      { refresh: "state", dirtyExports: false },
    );
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

  revision() {
    if (this.layoutEngine?.revision) {
      const revision = Number(this.layoutEngine.revision()) || 0;
      this.cache.revision = revision;
      return revision;
    }
    return this.cache.revision || 0;
  }

  lastCommandResultJson() {
    if (this.layoutEngine?.lastCommandResultJson) {
      const resultJson = this.layoutEngine.lastCommandResultJson();
      if (typeof resultJson === "string") {
        this.cache.lastCommandResultJson = resultJson;
        return resultJson;
      }
    }
    return this.cache.lastCommandResultJson || "null";
  }

  executeCommandJson(commandJson) {
    return this.invokeMutation("desktop_engine_execute_command_json", { commandJson }, { refresh: "document" });
  }

  async objectSettingsDialogJson() {
    await this.ready();
    return this.invoke("desktop_engine_object_settings_dialog_json", { sessionId: this.sessionId });
  }

  toolbarColorPaletteJson(customColorsJson = "[]") {
    return this.layoutEngine?.toolbarColorPaletteJson?.(customColorsJson) || JSON.stringify({ colors: [], otherLabel: "Other..." });
  }

  colorDialogPaletteJson(currentColor = "#000000", customColorsJson = "[]") {
    return this.layoutEngine?.colorDialogPaletteJson?.(currentColor, customColorsJson)
      || JSON.stringify({ selected: currentColor, basicColors: [], customColors: [] });
  }

  textSymbolPaletteJson() {
    return this.layoutEngine?.textSymbolPaletteJson?.() || JSON.stringify({ groups: [] });
  }

  elementPaletteJson() {
    return this.layoutEngine?.elementPaletteJson?.() || JSON.stringify({ elements: [] });
  }

  bondToolIconSvg(variant, strokeWidth, boldWidth) {
    return this.layoutEngine?.bondToolIconSvg?.(variant, strokeWidth, boldWidth) || "";
  }

  arrowToolIconSvg(kind) {
    return this.layoutEngine?.arrowToolIconSvg?.(kind) || "";
  }

  shapeToolIconSvg(kind, style) {
    return this.layoutEngine?.shapeToolIconSvg?.(kind, style) || "";
  }

  symbolToolIconSvg(kind) {
    return this.layoutEngine?.symbolToolIconSvg?.(kind) || "";
  }

  orbitalToolIconSvg(template, style, phase) {
    return this.layoutEngine?.orbitalToolIconSvg?.(template, style, phase) || "";
  }

  chainToolIconSvg(strokeWidth) {
    return this.layoutEngine?.chainToolIconSvg?.(strokeWidth) || "";
  }

  textFormatIconSvg(kind) {
    return this.layoutEngine?.textFormatIconSvg?.(kind) || "";
  }

  applyElementPaletteJson(selectionJson) {
    return this.invokeMutation(
      "desktop_engine_apply_element_palette_json",
      { selectionJson },
      { refresh: "state", dirtyExports: false },
    ).then((changed) => {
      this.layoutEngine?.applyElementPaletteJson?.(selectionJson);
      return changed;
    });
  }

  applyObjectSettingsDialogJson(settingsJson) {
    this.syncLayoutDocumentJson();
    return this.invokeMutation("desktop_engine_apply_object_settings_dialog_json", { settingsJson }, { refresh: "all" });
  }

  setArrowOptions(variant, headSize, head, tail, bold) {
    if (this.layoutEngine?.setArrowOptions) {
      this.layoutEngine.setArrowOptions(variant, headSize, head, tail, bold);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_arrow_options",
        "desktop_engine_set_arrow_options",
        { variant, headSize, head, tail, bold },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_set_arrow_options", {
      variant,
      headSize,
      head,
      tail,
      bold,
    });
  }

  setArrowEndpointOptions(variant, headSize, curve, headStyle, tailStyle, noGo, bold) {
    if (this.layoutEngine?.setArrowEndpointOptions) {
      this.layoutEngine.setArrowEndpointOptions(variant, headSize, curve, headStyle, tailStyle, noGo, bold);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "set_arrow_endpoint_options",
        "desktop_engine_set_arrow_endpoint_options",
        { variant, headSize, curve, headStyle, tailStyle, noGo, bold },
        { refresh: "state", dirtyExports: false },
      );
      return undefined;
    }
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
    if (this.activeToolUsesLocalPointer() && this.layoutEngine?.pointerMove) {
      this.layoutEngine.pointerMove(x, y, altKey);
      this.syncCacheFromLayout({ interaction: true });
      this.runCoalescedNativeMutationInBackground(
        "pointer_move",
        "desktop_engine_pointer_move",
        { x, y, altKey },
        { refresh: "interaction", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_pointer_move", { x, y, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  pointerDown(x, y, altKey) {
    if (this.activeToolUsesLocalPointer() && this.layoutEngine?.pointerDown) {
      this.layoutEngine.pointerDown(x, y, altKey);
      this.syncCacheFromLayout({ interaction: true });
      this.runNativeMutationInBackground(
        "desktop_engine_pointer_down",
        { x, y, altKey },
        { refresh: "interaction", dirtyExports: false },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_pointer_down", { x, y, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  pointerUp(x, y, altKey) {
    if (this.activeToolUsesLocalPointer() && this.layoutEngine?.pointerUp) {
      this.layoutEngine.pointerUp(x, y, altKey);
      this.markExportsDirty();
      this.syncCacheFromLayout({ interaction: true });
      this.runNativeMutationInBackground(
        "desktop_engine_pointer_up",
        { x, y, altKey },
        { refresh: "state", dirtyExports: true },
      );
      return undefined;
    }
    return this.invokeMutation("desktop_engine_pointer_up", { x, y, altKey }, { refresh: "document" });
  }

  selectAtPoint(x, y, additive) {
    if (this.layoutEngine?.selectAtPoint) {
      const result = this.layoutEngine.selectAtPoint(x, y, additive);
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_select_at_point",
        { x, y, additive },
        { refresh: "selection", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_select_at_point", { x, y, additive }, { refresh: "selection", dirtyExports: false });
  }

  selectComponentAtPoint(x, y, additive) {
    if (this.layoutEngine?.selectComponentAtPoint) {
      const result = this.layoutEngine.selectComponentAtPoint(x, y, additive);
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_select_component_at_point",
        { x, y, additive },
        { refresh: "selection", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_select_component_at_point", { x, y, additive }, { refresh: "selection", dirtyExports: false });
  }

  selectInRect(x1, y1, x2, y2, additive) {
    if (this.layoutEngine?.selectInRect) {
      const result = this.layoutEngine.selectInRect(x1, y1, x2, y2, additive);
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_select_in_rect",
        { x1, y1, x2, y2, additive },
        { refresh: "selection", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_select_in_rect", { x1, y1, x2, y2, additive }, { refresh: "selection", dirtyExports: false });
  }

  selectInPolygon(pointsJson, additive) {
    if (this.layoutEngine?.selectInPolygon) {
      const result = this.layoutEngine.selectInPolygon(pointsJson, additive);
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_select_in_polygon",
        { pointsJson, additive },
        { refresh: "selection", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_select_in_polygon", { pointsJson, additive }, { refresh: "selection", dirtyExports: false });
  }

  selectAll() {
    if (this.layoutEngine?.selectAll) {
      const result = this.layoutEngine.selectAll();
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_select_all",
        {},
        { refresh: "selection", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_select_all", {}, { refresh: "selection", dirtyExports: false });
  }

  clearSelection() {
    if (this.layoutEngine?.clearSelection) {
      const result = this.layoutEngine.clearSelection();
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_clear_selection",
        {},
        { refresh: "selection", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_clear_selection", {}, { refresh: "selection", dirtyExports: false });
  }

  async contextHitTestJson(x, y) {
    await this.ready();
    return this.invoke("desktop_engine_context_hit_test_json", { sessionId: this.sessionId, x, y });
  }

  async contextMenuJson(hitJson, hasPaste) {
    await this.ready();
    return this.invoke("desktop_engine_context_menu_json", { sessionId: this.sessionId, hitJson, hasPaste });
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
    if (this.layoutEngine?.hoverArrowAction) {
      return this.layoutEngine.hoverArrowAction(x, y);
    }
    return this.invoke("desktop_engine_hover_arrow_action", { sessionId: this.sessionId, x, y });
  }

  beginHoverArrowEdit(x, y) {
    if (this.layoutEngine?.beginHoverArrowEdit) {
      const action = this.layoutEngine.beginHoverArrowEdit(x, y);
      if (!action) {
        return "";
      }
      this.localArrowEditActive = true;
      this.syncLocalMutationState();
      this.pendingArrowEditBegin = this.runNativeMutationInBackground(
        "desktop_engine_begin_hover_arrow_edit",
        { x, y },
        { refresh: "interaction", dirtyExports: false },
      );
      return action;
    }
    return this.invokeMutation("desktop_engine_begin_hover_arrow_edit", { x, y }, { refresh: "interaction", dirtyExports: false });
  }

  updateHoverArrowEdit(x, y, altKey) {
    if (this.layoutEngine?.updateHoverArrowEdit) {
      return this.layoutEngine.updateHoverArrowEdit(x, y, altKey);
    }
    return this.invokeMutation("desktop_engine_update_hover_arrow_edit", { x, y, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  finishHoverArrowEdit(x, y, altKey) {
    if (this.layoutEngine?.finishHoverArrowEdit) {
      const changed = this.layoutEngine.finishHoverArrowEdit(x, y, altKey);
      if (changed) {
        this.syncLocalMutationState({ dirtyExports: true });
      }
      const pendingBegin = this.pendingArrowEditBegin;
      this.pendingArrowEditBegin = null;
      this.localArrowEditActive = false;
      if (changed || pendingBegin) {
        this.runNativeMutationInBackground(
          "desktop_engine_finish_hover_arrow_edit",
          { x, y, altKey },
          { refresh: "state", dirtyExports: changed, waitFor: pendingBegin },
        );
      }
      return changed;
    }
    return this.invokeMutation(
      "desktop_engine_finish_hover_arrow_edit",
      { x, y, altKey },
      { refresh: "interaction", dirtyExports: false },
    );
  }

  hoverShapeAction(x, y) {
    if (this.layoutEngine?.hoverShapeAction) {
      return this.layoutEngine.hoverShapeAction(x, y);
    }
    return this.invoke("desktop_engine_hover_shape_action", { sessionId: this.sessionId, x, y });
  }

  beginHoverShapeEdit(x, y) {
    if (this.layoutEngine?.beginHoverShapeEdit) {
      const action = this.layoutEngine.beginHoverShapeEdit(x, y);
      if (!action) {
        return "";
      }
      this.localShapeEditActive = true;
      this.syncLocalMutationState();
      this.pendingShapeEditBegin = this.runNativeMutationInBackground(
        "desktop_engine_begin_hover_shape_edit",
        { x, y },
        { refresh: "interaction", dirtyExports: false },
      );
      return action;
    }
    return this.invokeMutation("desktop_engine_begin_hover_shape_edit", { x, y }, { refresh: "interaction", dirtyExports: false });
  }

  updateHoverShapeEdit(x, y, altKey) {
    if (this.layoutEngine?.updateHoverShapeEdit) {
      return this.layoutEngine.updateHoverShapeEdit(x, y, altKey);
    }
    return this.invokeMutation("desktop_engine_update_hover_shape_edit", { x, y, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  finishHoverShapeEdit(x, y, altKey) {
    if (this.layoutEngine?.finishHoverShapeEdit) {
      const changed = this.layoutEngine.finishHoverShapeEdit(x, y, altKey);
      if (changed) {
        this.syncLocalMutationState({ dirtyExports: true });
      }
      const pendingBegin = this.pendingShapeEditBegin;
      this.pendingShapeEditBegin = null;
      this.localShapeEditActive = false;
      if (changed || pendingBegin) {
        this.runNativeMutationInBackground(
          "desktop_engine_finish_hover_shape_edit",
          { x, y, altKey },
          { refresh: "state", dirtyExports: changed, waitFor: pendingBegin },
        );
      }
      return changed;
    }
    return this.invokeMutation(
      "desktop_engine_finish_hover_shape_edit",
      { x, y, altKey },
      { refresh: "interaction", dirtyExports: false },
    );
  }

  activeArrowEditDegrees() {
    if (this.localArrowEditActive && this.layoutEngine?.activeArrowEditDegrees) {
      return this.layoutEngine.activeArrowEditDegrees();
    }
    return 0;
  }

  beginSelectionMove(x, y, additive, altKey) {
    if (this.layoutEngine?.beginSelectionMove) {
      const began = this.layoutEngine.beginSelectionMove(x, y, additive, altKey);
      if (!began) {
        return false;
      }
      this.localSelectionMoveActive = true;
      this.syncLocalMutationState();
      this.pendingSelectionMoveBegin = this.runNativeMutationInBackground(
        "desktop_engine_begin_selection_move",
        { x, y, additive, altKey },
        { refresh: "interaction", dirtyExports: false },
      );
      return true;
    }
    return this.invokeMutation("desktop_engine_begin_selection_move", { x, y, additive, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  updateSelectionMove(x, y, altKey) {
    if (this.layoutEngine?.updateSelectionMove) {
      return this.layoutEngine.updateSelectionMove(x, y, altKey);
    }
    return this.invokeMutation("desktop_engine_update_selection_move", { x, y, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  finishSelectionMove(x, y, altKey) {
    if (this.layoutEngine?.finishSelectionMove) {
      const changed = this.layoutEngine.finishSelectionMove(x, y, altKey);
      if (changed) {
        this.syncLocalMutationState({ dirtyExports: true });
      }
      const pendingBegin = this.pendingSelectionMoveBegin;
      this.pendingSelectionMoveBegin = null;
      this.localSelectionMoveActive = false;
      if (changed || pendingBegin) {
        this.runNativeMutationInBackground(
          "desktop_engine_finish_selection_move",
          { x, y, altKey },
          { refresh: "state", dirtyExports: changed, waitFor: pendingBegin },
        );
      }
      return changed;
    }
    return this.invokeMutation("desktop_engine_finish_selection_move", { x, y, altKey });
  }

  beginSelectionRotate(x, y) {
    if (this.layoutEngine?.beginSelectionRotate) {
      const began = this.layoutEngine.beginSelectionRotate(x, y);
      if (!began) {
        return false;
      }
      this.localSelectionRotateActive = true;
      this.syncLocalMutationState();
      this.pendingSelectionRotateBegin = this.runNativeMutationInBackground(
        "desktop_engine_begin_selection_rotate",
        { x, y },
        { refresh: "interaction", dirtyExports: false },
      );
      return true;
    }
    return this.invokeMutation("desktop_engine_begin_selection_rotate", { x, y }, { refresh: "interaction", dirtyExports: false });
  }

  updateSelectionRotate(x, y, altKey) {
    if (this.layoutEngine?.updateSelectionRotate) {
      return this.layoutEngine.updateSelectionRotate(x, y, altKey);
    }
    return this.invokeMutation("desktop_engine_update_selection_rotate", { x, y, altKey }, { refresh: "interaction", dirtyExports: false });
  }

  finishSelectionRotate(x, y, altKey) {
    if (this.layoutEngine?.finishSelectionRotate) {
      const changed = this.layoutEngine.finishSelectionRotate(x, y, altKey);
      if (changed) {
        this.syncLocalMutationState({ dirtyExports: true });
      }
      const pendingBegin = this.pendingSelectionRotateBegin;
      this.pendingSelectionRotateBegin = null;
      this.localSelectionRotateActive = false;
      if (changed || pendingBegin) {
        this.runNativeMutationInBackground(
          "desktop_engine_finish_selection_rotate",
          { x, y, altKey },
          { refresh: "state", dirtyExports: changed, waitFor: pendingBegin },
        );
      }
      return changed;
    }
    return this.invokeMutation("desktop_engine_finish_selection_rotate", { x, y, altKey });
  }

  beginSelectionResize(handle, x, y) {
    if (this.layoutEngine?.beginSelectionResize) {
      const began = this.layoutEngine.beginSelectionResize(handle, x, y);
      if (!began) {
        return false;
      }
      this.localSelectionResizeActive = true;
      this.syncLocalMutationState();
      this.pendingSelectionResizeBegin = this.runNativeMutationInBackground(
        "desktop_engine_begin_selection_resize",
        { handle, x, y },
        { refresh: "interaction", dirtyExports: false },
      );
      return true;
    }
    return this.invokeMutation("desktop_engine_begin_selection_resize", { handle, x, y }, { refresh: "interaction", dirtyExports: false });
  }

  updateSelectionResize(x, y) {
    if (this.layoutEngine?.updateSelectionResize) {
      return this.layoutEngine.updateSelectionResize(x, y);
    }
    return this.invokeMutation("desktop_engine_update_selection_resize", { x, y }, { refresh: "interaction", dirtyExports: false });
  }

  finishSelectionResize(x, y) {
    if (this.layoutEngine?.finishSelectionResize) {
      const changed = this.layoutEngine.finishSelectionResize(x, y);
      if (changed) {
        this.syncLocalMutationState({ dirtyExports: true });
      }
      const pendingBegin = this.pendingSelectionResizeBegin;
      this.pendingSelectionResizeBegin = null;
      this.localSelectionResizeActive = false;
      if (changed || pendingBegin) {
        this.runNativeMutationInBackground(
          "desktop_engine_finish_selection_resize",
          { x, y },
          { refresh: "state", dirtyExports: changed, waitFor: pendingBegin },
        );
      }
      return changed;
    }
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

  async selectionNumericDialogJson(kind) {
    await this.ready();
    return this.invoke("desktop_engine_selection_numeric_dialog_json", { sessionId: this.sessionId, kind });
  }

  applySelectionNumericDialogJson(payloadJson) {
    return this.invokeMutation("desktop_engine_apply_selection_numeric_dialog_json", { payloadJson });
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

  linkSelection() {
    return this.invokeMutation("desktop_engine_link_selection");
  }

  unlinkSelection() {
    return this.invokeMutation("desktop_engine_unlink_selection");
  }

  joinSelection() {
    return this.invokeMutation("desktop_engine_join_selection");
  }

  applyColorToSelection(color) {
    return this.invokeMutation("desktop_engine_apply_color_to_selection", { color });
  }

  applyShapeStyleToSelection(style) {
    return this.invokeMutation("desktop_engine_apply_shape_style_to_selection", { style });
  }

  applyOrbitalTemplateToSelection(template) {
    return this.invokeMutation("desktop_engine_apply_orbital_template_to_selection", { template });
  }

  applyOrbitalStyleToSelection(style) {
    return this.invokeMutation("desktop_engine_apply_orbital_style_to_selection", { style });
  }

  applyOrbitalPhaseToSelection(phase) {
    return this.invokeMutation("desktop_engine_apply_orbital_phase_to_selection", { phase });
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

  applyHoveredBondStyle(style) {
    return this.invokeMutation("desktop_engine_apply_hovered_bond_style", { style });
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
    if (this.layoutEngine?.clearInteraction) {
      const result = this.layoutEngine.clearInteraction();
      this.syncLocalMutationState();
      this.runNativeMutationInBackground(
        "desktop_engine_clear_interaction",
        {},
        { refresh: "interaction", dirtyExports: false },
      );
      return result;
    }
    return this.invokeMutation("desktop_engine_clear_interaction", {}, { refresh: "interaction", dirtyExports: false });
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
    return this.invokeMutation("desktop_engine_copy_selection", {}, { refresh: "state", dirtyExports: false });
  }

  async hasClipboard() {
    await this.ready();
    return this.invoke("desktop_engine_has_clipboard", { sessionId: this.sessionId });
  }

  async clipboardSelectionJson() {
    await this.ready();
    return this.invoke("desktop_engine_clipboard_selection_json", { sessionId: this.sessionId });
  }

  async clipboardDocumentJson() {
    await this.ready();
    return this.invoke("desktop_engine_clipboard_document_json", { sessionId: this.sessionId });
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
    if (this.layoutEngine?.beginTextEdit) {
      const sessionJson = this.layoutEngine.beginTextEdit(x, y);
      this.syncCacheFromLayout({ interaction: true });
      this.runNativeMutationInBackground(
        "desktop_engine_begin_text_edit",
        { x, y },
        { refresh: "interaction", dirtyExports: false },
      );
      return sessionJson;
    }
    return this.invokeMutation("desktop_engine_begin_text_edit", { x, y }, { refresh: "interaction", dirtyExports: false });
  }

  applyTextEdit(sessionJson) {
    return this.invokeMutation("desktop_engine_apply_text_edit", { sessionJson }, { refresh: "all" });
  }

  applyBracketLabelText(bracketId, sessionJson) {
    return this.invokeMutation(
      "desktop_engine_apply_bracket_label_text",
      { bracketId, sessionJson },
      { refresh: "all" },
    );
  }

  async pendingGraphicObjectId() {
    if (this.layoutEngine?.pendingGraphicObjectId) {
      return this.layoutEngine.pendingGraphicObjectId();
    }
    await this.ready();
    return this.invoke("desktop_engine_pending_graphic_object_id", { sessionId: this.sessionId });
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
    return createManagedEngineSession(new TauriEngineSession(this.invoke, {
      layoutEngine: new WasmEngine(),
    }));
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
  if (engineOverride === "desktop-hybrid" || engineOverride === "tauri") {
    return "desktop-hybrid";
  }
  return globalThis.__TAURI_INTERNALS__ ? "desktop-hybrid" : "wasm";
}

export function createEngineHost(kind = detectEngineHostKind()) {
  if (kind === "tauri-native") {
    return new TauriEngineHost();
  }
  if (kind === "desktop-hybrid" || kind === "tauri") {
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
