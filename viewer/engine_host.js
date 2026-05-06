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
  constructor(invoke, sessionId) {
    this.invoke = invoke;
    this.sessionId = sessionId;
  }

  async free() {
    return this.invoke("desktop_engine_free", { sessionId: this.sessionId });
  }

  async loadDocumentJson(json) {
    return this.invoke("desktop_engine_load_document_json", { sessionId: this.sessionId, json });
  }

  async loadDocumentCdxml(cdxml) {
    return this.invoke("desktop_engine_load_document_cdxml", { sessionId: this.sessionId, cdxml });
  }

  async documentJson() {
    return this.invoke("desktop_engine_document_json", { sessionId: this.sessionId });
  }

  async stateJson() {
    return this.invoke("desktop_engine_state_json", { sessionId: this.sessionId });
  }

  async renderListJson() {
    return this.invoke("desktop_engine_render_list_json", { sessionId: this.sessionId });
  }

  async renderBoundsJson(scope = "all") {
    return this.invoke("desktop_engine_render_bounds_json", { sessionId: this.sessionId, scope });
  }

  async documentCdxml() {
    return this.invoke("desktop_engine_document_cdxml", { sessionId: this.sessionId });
  }

  async documentSvg() {
    return this.invoke("desktop_engine_document_svg", { sessionId: this.sessionId });
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
    this.invoke = invoke;
    return this;
  }

  async createEngineSession() {
    const sessionId = await this.invoke("desktop_engine_create");
    return new TauriEngineSession(this.invoke, sessionId);
  }

  async runSmokeTest() {
    const session = await this.createEngineSession();
    try {
      const documentJson = await session.documentJson();
      const renderListJson = await session.renderListJson();
      const renderBoundsJson = await session.renderBoundsJson("all");
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
  return globalThis.__TAURI_INTERNALS__ ? "tauri" : "wasm";
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
