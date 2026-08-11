export function normalizeDesktopPath(value) {
  if (typeof value === "string") {
    const path = value.trim();
    return path || null;
  }
  if (!value || typeof value !== "object") {
    return null;
  }
  for (const key of ["path", "filePath", "fullPath", "filepath"]) {
    const path = normalizeDesktopPath(value[key]);
    if (path) {
      return path;
    }
  }
  return null;
}

export function openPathsFromDesktopPayload(payload) {
  const source = Array.isArray(payload)
    ? payload
    : Array.isArray(payload?.paths)
      ? payload.paths
      : [payload?.path ?? payload?.filePath ?? payload?.fullPath].filter((value) => value != null);
  return source
    .map((value) => normalizeDesktopPath(value))
    .filter(Boolean);
}

function requireDesktopPath(value, action) {
  const path = normalizeDesktopPath(value);
  if (!path) {
    throw new Error(`Invalid file path for ${action}.`);
  }
  return path;
}

function traceValue(value) {
  if (value instanceof Error) {
    return {
      name: value.name,
      message: value.message,
      stack: value.stack,
    };
  }
  if (value && typeof value === "object") {
    try {
      return JSON.parse(JSON.stringify(value, (_key, innerValue) => {
        if (innerValue instanceof Error) {
          return {
            name: innerValue.name,
            message: innerValue.message,
            stack: innerValue.stack,
          };
        }
        return innerValue;
      }));
    } catch {
      return String(value);
    }
  }
  return value;
}

export class DesktopFileHost {
  constructor() {
    this.invoke = null;
    this.listen = null;
    this.tauriWindow = null;
    this.unlisteners = [];
  }

  get available() {
    return typeof this.invoke === "function";
  }

  get usesCustomWindowChrome() {
    return true;
  }

  initialize() {
    this.invoke = globalThis.__TAURI__?.core?.invoke || null;
    this.listen = globalThis.__TAURI__?.event?.listen || null;
    this.tauriWindow = globalThis.__TAURI__?.window || null;
    return this;
  }

  currentWindow() {
    return this.tauriWindow?.getCurrentWindow?.() || null;
  }

  async chooseOpenPath() {
    return this.invoke("desktop_file_choose_open");
  }

  async setWindowTitle(title) {
    return this.invoke("desktop_window_set_title", { title });
  }

  async minimizeWindow() {
    return this.invoke("desktop_window_minimize");
  }

  async toggleMaximizeWindow() {
    return this.invoke("desktop_window_toggle_maximize");
  }

  async closeWindow() {
    return this.invoke("desktop_window_close");
  }

  async destroyWindow() {
    if (typeof this.invoke === "function") {
      try {
        return await this.invoke("desktop_window_destroy");
      } catch (error) {
        console.warn("desktop_window_destroy failed; falling back to Tauri window destroy", error);
      }
    }
    const currentWindow = this.currentWindow();
    if (typeof currentWindow?.destroy === "function") {
      return currentWindow.destroy();
    }
    return this.closeWindow();
  }

  async startWindowDrag() {
    return this.invoke("desktop_window_start_dragging");
  }

  async isWindowMaximized() {
    return this.invoke("desktop_window_is_maximized");
  }

  async detachDocumentWindow(document, screenX = null, screenY = null) {
    return this.invoke("desktop_window_detach_document", { document, screenX, screenY });
  }

  async takeDetachedDocument() {
    return this.invoke("desktop_window_take_detached_document");
  }

  async chooseSavePath(suggestedName) {
    return this.invoke("desktop_file_choose_save", { suggestedName });
  }

  async chooseExportSavePath(suggestedName, extension) {
    return this.invoke("desktop_file_choose_export_save", { suggestedName, extension });
  }

  async traceEvent(event, detail = null) {
    if (typeof this.invoke !== "function") {
      return;
    }
    const payload = {
      event,
      detail: traceValue(detail),
      timestamp: Date.now(),
    };
    try {
      await this.invoke("desktop_trace_event", { message: JSON.stringify(payload) });
    } catch {
      // Tracing must never break document operations.
    }
  }

  async readPath(path) {
    const normalizedPath = requireDesktopPath(path, "open");
    await this.traceEvent("desktopFileHost.readPath.begin", { path: normalizedPath });
    try {
      const opened = await this.invoke("desktop_file_read_path", { path: normalizedPath });
      await this.traceEvent("desktopFileHost.readPath.ok", {
        path: normalizedPath,
        openedPath: opened?.path,
        fileName: opened?.fileName,
        format: opened?.format,
        textLength: typeof opened?.text === "string" ? opened.text.length : null,
      });
      return opened;
    } catch (error) {
      await this.traceEvent("desktopFileHost.readPath.error", { path: normalizedPath, error });
      throw error;
    }
  }

  async readBinaryPath(path) {
    return this.invoke("desktop_file_read_binary_path", {
      path: requireDesktopPath(path, "read image"),
    });
  }

  async writePath(path, content, format = null) {
    return this.invoke("desktop_file_write_path", { path: requireDesktopPath(path, "save"), content, format });
  }

  async readRecoveryJournal(path) {
    return this.invoke("desktop_file_read_recovery_journal", {
      path: requireDesktopPath(path, "read recovery journal"),
    });
  }

  async writeRecoveryJournal(path, content) {
    return this.invoke("desktop_file_write_recovery_journal", {
      path: requireDesktopPath(path, "write recovery journal"),
      content,
    });
  }

  async deleteRecoveryJournal(path) {
    return this.invoke("desktop_file_delete_recovery_journal", {
      path: requireDesktopPath(path, "delete recovery journal"),
    });
  }

  async writeTransientPath(path, content) {
    return this.invoke("desktop_file_write_transient_path", { path: requireDesktopPath(path, "save"), content });
  }

  async writeOleEditPayload(path, payload) {
    return this.invoke("desktop_file_write_ole_edit_payload", { path: requireDesktopPath(path, "save"), payload });
  }

  async writeBase64(path, contentBase64) {
    return this.invoke("desktop_file_write_base64", { path: requireDesktopPath(path, "export"), contentBase64 });
  }

  async exportEmf(path, payload) {
    return this.invoke("desktop_file_export_emf", {
      path: requireDesktopPath(path, "export"),
      payload,
    });
  }

  async confirmApplyStylePreset(presetName, message) {
    return this.invoke("desktop_dialog_confirm_style_preset", { presetName, message });
  }

  async recentFiles() {
    return this.invoke("desktop_recent_files");
  }

  async clearRecentFiles() {
    return this.invoke("desktop_clear_recent_files");
  }

  async takeStartupOpenPaths() {
    return this.invoke("desktop_take_startup_open_paths");
  }

  async writeClipboard(payload) {
    return this.invoke("desktop_clipboard_write", { payload });
  }

  async readClipboard() {
    return this.invoke("desktop_clipboard_read");
  }

  async listenMenu(handler) {
    if (typeof this.listen !== "function") {
      return;
    }
    const unlisten = await this.listen("chemsema-desktop-menu", (event) => {
      handler(event?.payload?.command || "");
    });
    this.unlisteners.push(unlisten);
  }

  async listenOpenPaths(handler) {
    if (typeof this.listen !== "function") {
      return;
    }
    const unlisten = await this.listen("chemsema-desktop-open-paths", (event) => {
      const paths = openPathsFromDesktopPayload(event?.payload);
      void this.traceEvent("desktopFileHost.openPaths.event", { payload: event?.payload, paths });
      handler(paths, event?.payload || null);
    });
    this.unlisteners.push(unlisten);
  }

  async listenWindowCloseRequested(handler) {
    const currentWindow = this.currentWindow();
    if (typeof currentWindow?.onCloseRequested !== "function") {
      return;
    }
    const unlisten = await currentWindow.onCloseRequested(handler);
    this.unlisteners.push(unlisten);
  }

  dispose() {
    for (const unlisten of this.unlisteners.splice(0)) {
      unlisten?.();
    }
  }
}

function pathFileName(value) {
  const text = String(value || "").trim();
  if (!text) {
    return "";
  }
  try {
    const decoded = decodeURIComponent(text);
    const lastSegment = decoded.split(/[\\/]/).filter(Boolean).pop() || "";
    return lastSegment.split("?")[0].split("#")[0];
  } catch {
    return text.split(/[\\/]/).filter(Boolean).pop() || "";
  }
}

function formatFromPath(value) {
  const fileName = pathFileName(value).toLowerCase();
  const extension = fileName.includes(".") ? fileName.split(".").pop() : "";
  return extension || null;
}

export class HarmonyFileHost {
  constructor() {
    this.bridge = null;
    this.pending = new Map();
    this.nextRequestId = 1;
    this.previousResolver = null;
    this.clipboardPayload = null;
  }

  get available() {
    return typeof this.bridge?.postMessage === "function";
  }

  get usesCustomWindowChrome() {
    return false;
  }

  initialize() {
    this.bridge = globalThis.chemsemaHarmony || null;
    if (!this.available) {
      return this;
    }
    this.previousResolver = globalThis.__chemsemaHarmonyResolve;
    globalThis.__chemsemaHarmonyResolve = (id, responseJson) => {
      if (this.pending.has(id)) {
        this.resolveRequest(id, responseJson);
        return true;
      }
      return this.previousResolver?.(id, responseJson) ?? false;
    };
    return this;
  }

  resolveRequest(id, responseJson) {
    const entry = this.pending.get(id);
    if (!entry) {
      return;
    }
    this.pending.delete(id);
    clearTimeout(entry.timer);
    let response = null;
    try {
      response = typeof responseJson === "string" ? JSON.parse(responseJson) : responseJson;
    } catch (error) {
      entry.reject(error);
      return;
    }
    if (!response?.ok) {
      entry.reject(new Error(response?.error || "Harmony bridge command failed."));
      return;
    }
    entry.resolve(response.value ?? null);
  }

  invoke(command, payload = {}) {
    if (!this.available) {
      return Promise.reject(new Error("Harmony bridge is not available."));
    }
    const id = `harmony-${Date.now()}-${this.nextRequestId++}`;
    const message = JSON.stringify({ id, command, payload });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Harmony bridge command timed out: ${command}`));
      }, 60000);
      this.pending.set(id, { command, resolve, reject, timer });
      try {
        const accepted = this.bridge.postMessage(message);
        if (accepted === false || accepted === "false") {
          clearTimeout(timer);
          this.pending.delete(id);
          reject(new Error(`Harmony bridge rejected command: ${command}`));
        }
      } catch (error) {
        clearTimeout(timer);
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  async chooseOpenPath() {
    const selected = await this.invoke("chooseOpenPath");
    return normalizeDesktopPath(selected);
  }

  async chooseSavePath(suggestedName) {
    const selected = await this.invoke("chooseSavePath", { suggestedName });
    return normalizeDesktopPath(selected);
  }

  async chooseExportSavePath(suggestedName, extension) {
    const selected = await this.invoke("chooseExportSavePath", { suggestedName, extension });
    return normalizeDesktopPath(selected);
  }

  async readPath(path) {
    const normalizedPath = requireDesktopPath(path, "open");
    const opened = await this.invoke("readPath", { path: normalizedPath });
    return {
      path: opened?.path || normalizedPath,
      fileName: opened?.fileName || pathFileName(normalizedPath) || "Untitled",
      format: opened?.format || formatFromPath(normalizedPath),
      text: opened?.text || "",
    };
  }

  async writePath(path, content, format = null) {
    const normalizedPath = requireDesktopPath(path, "save");
    const saved = await this.invoke("writePath", { path: normalizedPath, content, format });
    return {
      path: saved?.path || normalizedPath,
      fileName: saved?.fileName || pathFileName(normalizedPath),
    };
  }

  async writeTransientPath(path, content) {
    return this.writePath(path, content, formatFromPath(path));
  }

  async writeBase64(path, contentBase64) {
    const normalizedPath = requireDesktopPath(path, "export");
    const saved = await this.invoke("writeBase64", { path: normalizedPath, contentBase64 });
    return {
      path: saved?.path || normalizedPath,
      fileName: saved?.fileName || pathFileName(normalizedPath),
    };
  }

  async exportEmf() {
    throw new Error("EMF export is not available on HarmonyOS.");
  }

  async writeClipboard(payload) {
    this.clipboardPayload = payload || null;
    return this.invoke("writeClipboard", { payload });
  }

  async readClipboard() {
    if (this.clipboardPayload) {
      return this.clipboardPayload;
    }
    return null;
  }

  async setWindowTitle(title) {
    return this.invoke("setWindowTitle", { title });
  }

  async traceEvent(event, detail = null) {
    try {
      await this.invoke("traceEvent", { event, detail: traceValue(detail), timestamp: Date.now() });
    } catch {
      // Tracing must never break document operations.
    }
  }

  async confirmApplyStylePreset() {
    return true;
  }

  async recentFiles() {
    return [];
  }

  async clearRecentFiles() {
    return true;
  }

  async takeStartupOpenPaths() {
    return [];
  }

  async takeDetachedDocument() {
    return null;
  }

  async listenMenu() {}
  async listenOpenPaths() {}
  async listenWindowCloseRequested() {}
  async minimizeWindow() {}
  async toggleMaximizeWindow() {}
  async closeWindow() {}
  async destroyWindow() {}
  async startWindowDrag() {}
  async isWindowMaximized() { return false; }

  dispose() {
    for (const entry of this.pending.values()) {
      clearTimeout(entry.timer);
      entry.reject(new Error("Harmony file host was disposed."));
    }
    this.pending.clear();
    if (globalThis.__chemsemaHarmonyResolve && this.previousResolver) {
      globalThis.__chemsemaHarmonyResolve = this.previousResolver;
    }
  }
}

export function createDesktopFileHost() {
  const host = new DesktopFileHost().initialize();
  if (host.available) {
    return host;
  }
  const harmonyHost = new HarmonyFileHost().initialize();
  return harmonyHost.available ? harmonyHost : null;
}
