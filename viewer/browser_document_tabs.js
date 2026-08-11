import { base64ToBytes, bytesToBase64 } from "./binary_helpers.js";
import { normalizeDesktopPath } from "./desktop_file_host.js";
import {
  chemsemaOpenAcceptTypes,
  decompressChemSemaText,
  looksLikeCdxFile,
  looksLikeCdxmlFile,
  looksLikeCompressedChemSemaFile,
  looksLikeSdfFile,
  saveFormatFromFileName,
} from "./file_io.js";

const BROWSER_PENDING_DOCUMENT_KEY_PREFIX = "chemsema:pending-browser-document:";
const BROWSER_PENDING_DOCUMENT_PARAM = "chemsemaPendingDocument";
const BROWSER_PENDING_DOCUMENT_WAIT_MS = 10000;
const BROWSER_PENDING_DOCUMENT_WAIT_INTERVAL_MS = 50;
const BROWSER_PENDING_DOCUMENT_RESERVED_STATUS = "__chemsema_pending_document_reserved__";

export function createBrowserDocumentTabs(options) {
  const {
    state,
    documentTabs,
    desktopFileHost,
    openFileInput,
  } = options;
  const documentTabRuntime = {
    get activeDocumentTabId() {
      return options.getActiveDocumentTabId();
    },
    set activeDocumentTabId(value) {
      options.setActiveDocumentTabId(value);
    },
  };
  const isDesktopShell = Boolean(options.isDesktopShell());
  const canUseBrowserTabs = !isDesktopShell && !desktopFileHost?.available;
  const appRuntimeReady = options.appRuntimeReady();
  const finishActiveTextEditor = (...args) => options.finishActiveTextEditor(...args);
  const syncDocumentFromEngine = (...args) => options.syncDocumentFromEngine(...args);
  const saveActiveDocumentTabState = (...args) => options.saveActiveDocumentTabState(...args);
  const documentTitleFromState = (...args) => options.documentTitleFromState(...args);
  const closeDocumentTab = (...args) => options.closeDocumentTab(...args);
  const loadJsonDocumentIntoEditor = (...args) => options.loadJsonDocumentIntoEditor(...args);
  const currentDocumentRevision = (...args) => options.currentDocumentRevision(...args);
  const refreshCommandAvailability = (...args) => options.refreshCommandAvailability(...args);
  const setZoomPercent = (...args) => options.setZoomPercent(...args);
  const renderDocumentTabs = (...args) => options.renderDocumentTabs(...args);
  const loadCdxDocumentIntoEditor = (...args) => options.loadCdxDocumentIntoEditor(...args);
  const openDocumentText = (...args) => options.openDocumentText(...args);
  const activeDocumentTabIsBlankUntitled = (...args) => options.activeDocumentTabIsBlankUntitled(...args);
  const currentDocumentIsDirty = (...args) => options.currentDocumentIsDirty(...args);
  const createDocumentTab = (...args) => options.createDocumentTab(...args);
  const restoreDocumentTabState = (...args) => options.restoreDocumentTabState(...args);
  const resetEditorEngine = (...args) => options.resetEditorEngine(...args);
  const renderDocument = (...args) => options.renderDocument(...args);
  const fitView = (...args) => options.fitView(...args);
  const documentTabForFilePath = (...args) => options.documentTabForFilePath(...args);
  const activateDocumentTab = (...args) => options.activateDocumentTab(...args);
  const activeDocumentTab = (...args) => options.activeDocumentTab(...args);
  const fileNameFromPath = (...args) => options.fileNameFromPath(...args);
  const openDocumentPath = (...args) => options.openDocumentPath(...args);
  const openDocumentFile = (...args) => options.openDocumentFile(...args);

  function browserTabUrlForPendingDocument(id) {
    const url = new URL(window.location.href);
    url.searchParams.set(BROWSER_PENDING_DOCUMENT_PARAM, id);
    return url.toString();
  }
  
  function browserPendingDocumentStorageKey(id) {
    return `${BROWSER_PENDING_DOCUMENT_KEY_PREFIX}${id}`;
  }
  
  function createBrowserPendingDocumentId() {
    return `doc-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  }
  
  function storeBrowserPendingDocumentPayload(id, payload) {
    localStorage.setItem(browserPendingDocumentStorageKey(id), JSON.stringify(payload));
  }
  
  function reserveBrowserPendingDocumentPayload(id) {
    localStorage.setItem(browserPendingDocumentStorageKey(id), JSON.stringify({
      status: BROWSER_PENDING_DOCUMENT_RESERVED_STATUS,
      createdAt: Date.now(),
    }));
  }
  
  function isReservedBrowserPendingDocument(raw) {
    if (!raw) {
      return false;
    }
    try {
      return JSON.parse(raw)?.status === BROWSER_PENDING_DOCUMENT_RESERVED_STATUS;
    } catch {
      return false;
    }
  }
  
  function clearBrowserPendingDocumentUrlParam() {
    if (!canUseBrowserTabs || typeof window === "undefined") {
      return;
    }
    const url = new URL(window.location.href);
    if (!url.searchParams.has(BROWSER_PENDING_DOCUMENT_PARAM)) {
      return;
    }
    url.searchParams.delete(BROWSER_PENDING_DOCUMENT_PARAM);
    window.history.replaceState(window.history.state, "", url.toString());
  }
  
  function openBrowserTab(url, { focus = true } = {}) {
    if (typeof window === "undefined") {
      return null;
    }
    const opened = window.open(url.toString(), "_blank");
    if (!opened) {
      return null;
    }
    if (focus) {
      try {
        opened.focus?.();
      } catch {
        // Browser focus policy decides whether this is honored.
      }
    }
    try {
      opened.opener = null;
    } catch {
      // Some browsers expose a read-only opener proxy.
    }
    return opened;
  }
  
  function focusBrowserTab(opened) {
    try {
      opened?.focus?.();
    } catch {
      // Browser focus policy decides whether this is honored.
    }
  }
  
  function openBrowserBlankDocumentTab() {
    if (typeof window === "undefined") {
      return false;
    }
    const url = new URL(window.location.href);
    url.searchParams.delete(BROWSER_PENDING_DOCUMENT_PARAM);
    return !!openBrowserTab(url);
  }
  
  function reserveBrowserPendingDocumentTab() {
    if (typeof window === "undefined") {
      return null;
    }
    const id = createBrowserPendingDocumentId();
    reserveBrowserPendingDocumentPayload(id);
    const opened = openBrowserTab(browserTabUrlForPendingDocument(id), { focus: false });
    if (!opened) {
      localStorage.removeItem(browserPendingDocumentStorageKey(id));
    }
    return opened ? { id, opened } : null;
  }
  
  async function browserPendingDocumentPayloadFromFile(file) {
    if (!file) {
      return null;
    }
    if (looksLikeCdxFile(file)) {
      const bytes = new Uint8Array(await file.arrayBuffer());
      return {
        dataBase64: bytesToBase64(bytes),
        fileName: file.name || null,
        filePath: null,
        format: "cdx",
      };
    }
    const text = looksLikeCompressedChemSemaFile(file)
      ? await decompressChemSemaText(file)
      : await file.text();
    return {
      text,
      fileName: file.name || null,
      filePath: null,
      format: looksLikeCdxmlFile(file, text)
        ? "cdxml"
        : looksLikeSdfFile(file, text)
          ? "sdf"
          : saveFormatFromFileName(file.name),
    };
  }
  
  async function writeBrowserFileToPendingDocument(id, file) {
    const payload = await browserPendingDocumentPayloadFromFile(file);
    if (!payload) {
      localStorage.removeItem(browserPendingDocumentStorageKey(id));
      return false;
    }
    storeBrowserPendingDocumentPayload(id, payload);
    return true;
  }
  
  async function openBrowserFileInNewTab(file) {
    const payload = await browserPendingDocumentPayloadFromFile(file);
    if (!payload) {
      return false;
    }
    const id = createBrowserPendingDocumentId();
    storeBrowserPendingDocumentPayload(id, payload);
    const opened = openBrowserTab(browserTabUrlForPendingDocument(id));
    if (!opened) {
      localStorage.removeItem(browserPendingDocumentStorageKey(id));
    }
    return !!opened;
  }
  
  async function openBrowserDroppedFilesInNewTabs(files) {
    const droppedFiles = Array.from(files || []).filter(Boolean);
    if (!droppedFiles.length || !canUseBrowserTabs) {
      return { opened: [], remainingFiles: droppedFiles };
    }
    const opened = [];
    const remainingFiles = [];
    for (const file of droppedFiles) {
      const reserved = reserveBrowserPendingDocumentTab();
      if (reserved) {
        opened.push({ id: reserved.id, file, opened: reserved.opened });
      } else {
        remainingFiles.push(file);
      }
    }
    if (!opened.length) {
      return { opened, remainingFiles };
    }
    focusBrowserTab(opened[opened.length - 1]?.opened);
    const writes = await Promise.allSettled(
      opened.map(async ({ id, file }) => {
        try {
          await writeBrowserFileToPendingDocument(id, file);
        } catch (error) {
          localStorage.removeItem(browserPendingDocumentStorageKey(id));
          throw error;
        }
      }),
    );
    const rejected = writes.find((result) => result.status === "rejected");
    if (rejected) {
      throw rejected.reason;
    }
    return { opened, remainingFiles };
  }
  
  function wait(ms) {
    return new Promise((resolve) => {
      window.setTimeout(resolve, ms);
    });
  }
  
  async function takeBrowserPendingDocument() {
    if (!canUseBrowserTabs || typeof window === "undefined") {
      return null;
    }
    const id = new URL(window.location.href).searchParams.get(BROWSER_PENDING_DOCUMENT_PARAM);
    if (!id) {
      return null;
    }
    const key = browserPendingDocumentStorageKey(id);
    const deadline = Date.now() + BROWSER_PENDING_DOCUMENT_WAIT_MS;
    while (true) {
      const raw = localStorage.getItem(key);
      if (!raw) {
        clearBrowserPendingDocumentUrlParam();
        return null;
      }
      if (!isReservedBrowserPendingDocument(raw)) {
        localStorage.removeItem(key);
        clearBrowserPendingDocumentUrlParam();
        try {
          return JSON.parse(raw);
        } catch {
          return null;
        }
      }
      if (Date.now() >= deadline) {
        localStorage.removeItem(key);
        clearBrowserPendingDocumentUrlParam();
        return null;
      }
      await wait(BROWSER_PENDING_DOCUMENT_WAIT_INTERVAL_MS);
    }
  }
  
  async function documentSnapshotFromTab(tab) {
    if (!tab) {
      return null;
    }
    if (tab.id === documentTabRuntime.activeDocumentTabId) {
      await finishActiveTextEditor(true);
      if (state.editorEngine) {
        await syncDocumentFromEngine();
      }
      saveActiveDocumentTabState();
    }
    const freshTab = documentTabs.find((entry) => entry.id === tab.id) || tab;
    if (!freshTab.currentDocument) {
      return null;
    }
    return {
      title: freshTab.title || documentTitleFromState(),
      fileName: freshTab.currentFileName || null,
      filePath: freshTab.currentFilePath || null,
      documentJson: JSON.stringify(freshTab.currentDocument),
      savedDocumentJson: freshTab.savedDocumentJson || null,
      savedRevision: freshTab.savedRevision ?? null,
      zoomPercent: Number(freshTab.zoomPercent || 100),
    };
  }
  
  async function detachDocumentTab(tabId, screenX = null, screenY = null) {
    if (!desktopFileHost?.detachDocumentWindow) {
      return false;
    }
    const tab = documentTabs.find((entry) => entry.id === tabId);
    const snapshot = await documentSnapshotFromTab(tab);
    if (!snapshot) {
      return false;
    }
    await desktopFileHost.detachDocumentWindow(snapshot, screenX, screenY);
    await closeDocumentTab(tabId, { skipUnsavedPrompt: true });
    return true;
  }
  
  async function loadDetachedDocumentPayload(payload) {
    if (!payload?.documentJson) {
      return false;
    }
    const documentData = JSON.parse(payload.documentJson);
    await loadJsonDocumentIntoEditor(documentData, payload.fileName || null, payload.filePath || null);
    if (typeof payload.savedDocumentJson === "string") {
      state.savedDocumentJson = payload.savedDocumentJson;
      state.savedRevision = Number.isFinite(Number(payload.savedRevision))
        ? Number(payload.savedRevision)
        : currentDocumentRevision();
      refreshCommandAvailability();
    }
    if (Number.isFinite(Number(payload.zoomPercent))) {
      setZoomPercent(Number(payload.zoomPercent));
    }
    saveActiveDocumentTabState();
    renderDocumentTabs();
    return true;
  }
  
  async function loadBrowserPendingDocumentPayload(payload) {
    if (payload?.format === "cdx" && payload?.dataBase64) {
      await loadCdxDocumentIntoEditor(base64ToBytes(payload.dataBase64), payload.fileName || null, payload.filePath || null);
      saveActiveDocumentTabState();
      renderDocumentTabs();
      return true;
    }
    if (!payload?.text) {
      return false;
    }
    await openDocumentText(payload.text, payload.fileName || null, payload.filePath || null, payload.format || null);
    saveActiveDocumentTabState();
    renderDocumentTabs();
    return true;
  }
  
  async function newDocumentTab() {
    await appRuntimeReady;
    await finishActiveTextEditor(true);
    const reuseActiveTab = canUseBrowserTabs && activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
    saveActiveDocumentTabState();
    if (canUseBrowserTabs && !reuseActiveTab && openBrowserBlankDocumentTab()) {
      return;
    }
    if (!reuseActiveTab) {
      const tab = createDocumentTab();
      documentTabs.push(tab);
      documentTabRuntime.activeDocumentTabId = tab.id;
      await restoreDocumentTabState(tab);
    }
    await resetEditorEngine();
    renderDocument();
    fitView();
    saveActiveDocumentTabState();
    renderDocumentTabs();
  }

  async function openGeneratedDocumentTab(documentData, title = "Untitled") {
    await appRuntimeReady;
    await finishActiveTextEditor(true);
    saveActiveDocumentTabState();
    const previousTabId = documentTabRuntime.activeDocumentTabId;
    const tab = createDocumentTab(title);
    documentTabs.push(tab);
    documentTabRuntime.activeDocumentTabId = tab.id;
    await restoreDocumentTabState(tab);
    try {
      await loadJsonDocumentIntoEditor(documentData, null, null);
      state.unsavedDocument = true;
      saveActiveDocumentTabState();
      renderDocumentTabs();
      fitView();
    } catch (error) {
      await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
      if (previousTabId && documentTabRuntime.activeDocumentTabId !== previousTabId) {
        await activateDocumentTab(previousTabId);
      }
      throw error;
    }
  }
  
  async function openDocumentPathInTab(path) {
    const normalizedPath = normalizeDesktopPath(path);
    void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.begin", { path, normalizedPath });
    if (!normalizedPath) {
      void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.skipInvalid", { path });
      return;
    }
    await appRuntimeReady;
    await finishActiveTextEditor(true);
    saveActiveDocumentTabState();
    const existingTab = documentTabForFilePath(normalizedPath);
    if (existingTab) {
      void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.activateExisting", {
        normalizedPath,
        tabId: existingTab.id,
      });
      await activateDocumentTab(existingTab.id);
      return;
    }
    const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
    const previousTabId = documentTabRuntime.activeDocumentTabId;
    let tab = activeDocumentTab();
    void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.plan", {
      normalizedPath,
      reuseActiveTab,
      previousTabId,
      activeTabId: tab?.id || null,
    });
    if (!reuseActiveTab) {
      tab = createDocumentTab(fileNameFromPath(normalizedPath) || "Loading...");
      documentTabs.push(tab);
      documentTabRuntime.activeDocumentTabId = tab.id;
      void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.createdTab", {
        normalizedPath,
        tabId: tab.id,
      });
      await restoreDocumentTabState(tab);
    }
    try {
      await openDocumentPath(normalizedPath);
      saveActiveDocumentTabState();
      renderDocumentTabs();
      void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.ok", {
        normalizedPath,
        tabId: documentTabRuntime.activeDocumentTabId,
      });
    } catch (error) {
      await desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.error", {
        normalizedPath,
        tabId: tab?.id || null,
        previousTabId,
        error,
      });
      if (!reuseActiveTab) {
        await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
      }
      if (previousTabId && documentTabRuntime.activeDocumentTabId !== previousTabId) {
        await activateDocumentTab(previousTabId);
      }
      throw error;
    }
  }
  
  async function openDocumentFileInTab(file) {
    if (!file) {
      return;
    }
    if (canUseBrowserTabs && await openBrowserFileInNewTab(file)) {
      return;
    }
    await appRuntimeReady;
    await finishActiveTextEditor(true);
    const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
    saveActiveDocumentTabState();
    const previousTabId = documentTabRuntime.activeDocumentTabId;
    let tab = activeDocumentTab();
    if (!reuseActiveTab) {
      tab = createDocumentTab(file.name || "Loading...");
      documentTabs.push(tab);
      documentTabRuntime.activeDocumentTabId = tab.id;
      await restoreDocumentTabState(tab);
    }
    try {
      await openDocumentFile(file);
      saveActiveDocumentTabState();
      renderDocumentTabs();
    } catch (error) {
      if (!reuseActiveTab) {
        await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
      }
      if (previousTabId && documentTabRuntime.activeDocumentTabId !== previousTabId) {
        await activateDocumentTab(previousTabId);
      }
      throw error;
    }
  }
  
  async function openDroppedDocumentFileInTab(file) {
    if (!file) {
      return;
    }
    await appRuntimeReady;
    await finishActiveTextEditor(true);
    const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
    saveActiveDocumentTabState();
    const previousTabId = documentTabRuntime.activeDocumentTabId;
    let tab = activeDocumentTab();
    if (!reuseActiveTab) {
      tab = createDocumentTab(file.name || "Loading...");
      documentTabs.push(tab);
      documentTabRuntime.activeDocumentTabId = tab.id;
      await restoreDocumentTabState(tab);
    }
    try {
      await openDocumentFile(file);
      saveActiveDocumentTabState();
      renderDocumentTabs();
    } catch (error) {
      if (!reuseActiveTab) {
        await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
      }
      if (previousTabId && documentTabRuntime.activeDocumentTabId !== previousTabId) {
        await activateDocumentTab(previousTabId);
      }
      throw error;
    }
  }
  
  async function openDroppedDocumentFilesInTabs(files) {
    const droppedFiles = Array.from(files || []).filter(Boolean);
    if (!droppedFiles.length) {
      return;
    }
    const { remainingFiles } = await openBrowserDroppedFilesInNewTabs(droppedFiles);
    for (const file of remainingFiles) {
      await openDroppedDocumentFileInTab(file);
    }
  }
  
  async function chooseAndOpenDocumentTab() {
    if (desktopFileHost?.available) {
      const path = await desktopFileHost.chooseOpenPath();
      if (path) {
        await openDocumentPathInTab(path);
      }
      return;
    }
    if (window.showOpenFilePicker) {
      const [handle] = await window.showOpenFilePicker({
        multiple: false,
        types: chemsemaOpenAcceptTypes(),
        excludeAcceptAllOption: false,
      });
      if (handle) {
        await openDocumentFileInTab(await handle.getFile());
      }
      return;
    }
    openFileInput.click();
  }
  
  async function confirmApplyDocumentStylePreset(preset) {
    const label = preset === "acs-document-1996" ? "ACS 1996" : "Default";
    const message = `Apply ${label} to this document? This will rescale the drawing and update existing bond, label, and graphic metrics.`;
    if (desktopFileHost?.confirmApplyStylePreset) {
      return desktopFileHost.confirmApplyStylePreset(label, message);
    }
    return window.confirm(message);
  }
  
  

  return {
    detachDocumentTab,
    loadDetachedDocumentPayload,
    takeBrowserPendingDocument,
    loadBrowserPendingDocumentPayload,
    newDocumentTab,
    openGeneratedDocumentTab,
    openDocumentPathInTab,
    openDocumentFileInTab,
    openDroppedDocumentFileInTab,
    openDroppedDocumentFilesInTabs,
    chooseAndOpenDocumentTab,
    confirmApplyDocumentStylePreset,
  };
}
