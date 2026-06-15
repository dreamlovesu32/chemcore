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

  async writePath(path, content, format = null) {
    return this.invoke("desktop_file_write_path", { path: requireDesktopPath(path, "save"), content, format });
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

  async exportEmf(path, renderListJson, boundsJson) {
    return this.invoke("desktop_file_export_emf", {
      path: requireDesktopPath(path, "export"),
      renderListJson,
      boundsJson,
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
    const unlisten = await this.listen("chemcore-desktop-menu", (event) => {
      handler(event?.payload?.command || "");
    });
    this.unlisteners.push(unlisten);
  }

  async listenOpenPaths(handler) {
    if (typeof this.listen !== "function") {
      return;
    }
    const unlisten = await this.listen("chemcore-desktop-open-paths", (event) => {
      const paths = openPathsFromDesktopPayload(event?.payload);
      void this.traceEvent("desktopFileHost.openPaths.event", { payload: event?.payload, paths });
      handler(paths);
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

export function createDesktopFileHost() {
  const host = new DesktopFileHost().initialize();
  return host.available ? host : null;
}
