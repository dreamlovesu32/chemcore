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

  async writeBase64(path, contentBase64) {
    return this.invoke("desktop_file_write_base64", { path, contentBase64 });
  }

  async exportEmf(path, renderListJson, boundsJson) {
    return this.invoke("desktop_file_export_emf", { path, renderListJson, boundsJson });
  }

  async chooseColor(initialColor, customColors = []) {
    return this.invoke("desktop_color_choose", { initialColor, customColors });
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
