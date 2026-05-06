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

class TauriEngineHost {
  constructor() {
    this.kind = "tauri";
    this.native = true;
  }

  async initialize() {
    throw new Error("TauriEngineHost is reserved for the native desktop engine path.");
  }

  createEngineSession() {
    throw new Error("TauriEngineHost is not wired to the editor UI yet.");
  }
}

export function detectEngineHostKind() {
  return globalThis.__TAURI_INTERNALS__ ? "tauri" : "wasm";
}

export function createEngineHost(kind = detectEngineHostKind()) {
  if (kind === "tauri-native") {
    return new TauriEngineHost();
  }
  return new WasmEngineHost();
}
