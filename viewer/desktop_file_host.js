export class DesktopFileHost {
  constructor() {
    this.invoke = null;
    this.listen = null;
    this.unlisteners = [];
  }

  get available() {
    return typeof this.invoke === "function";
  }

  initialize() {
    this.invoke = globalThis.__TAURI__?.core?.invoke || null;
    this.listen = globalThis.__TAURI__?.event?.listen || null;
    return this;
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

  async readPath(path) {
    return this.invoke("desktop_file_read_path", { path });
  }

  async writePath(path, content, format = null) {
    return this.invoke("desktop_file_write_path", { path, content, format });
  }

  async writeTransientPath(path, content) {
    return this.invoke("desktop_file_write_transient_path", { path, content });
  }

  async writeBase64(path, contentBase64) {
    return this.invoke("desktop_file_write_base64", { path, contentBase64 });
  }

  async exportEmf(path, renderListJson, boundsJson) {
    return this.invoke("desktop_file_export_emf", { path, renderListJson, boundsJson });
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
      const paths = Array.isArray(event?.payload?.paths) ? event.payload.paths : [];
      handler(paths);
    });
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
