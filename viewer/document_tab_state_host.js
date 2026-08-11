export function createDocumentTabStateHost(scope) {
  const { state, documentTabs, documentTabsRoot, tabStateKeys, getActiveDocumentTabId, setActiveDocumentTabId, getActiveTextEditor, setActiveTextEditor, setActiveSelectionGesture, getDocumentTabInteractions, textEditorLayer, getZoomPercent, setStoredZoomPercent, syncWindowTitle, syncEngineToolState, syncZoomControl, renderSecondaryToolbar, renderDocument, refreshCommandAvailability, finishActiveTextEditor, desktopFileHost, syncDocumentFromEngine, scheduleDeferredDocumentSync, confirmCloseDocumentTab, resetEditorEngine, fitView, escapeHtml } = scope;

  function activeDocumentTab() {
    return documentTabs.find((tab) => tab.id === getActiveDocumentTabId()) || null;
  }

  function createDocumentTab(title = "Untitled") {
    return {
      id: `doc-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      title,
      zoomPercent: 100,
      currentPath: null,
      currentFileName: null,
      currentFilePath: null,
      savedDocumentJson: null,
      savedRevision: null,
      unsavedDocument: false,
      oleSyncedDocumentJson: null,
      oleSyncedRevision: null,
      currentDocument: null,
      editorEngine: null,
      documentEngine: null,
      coreRenderList: null,
      runtimeViewBox: null,
      lastEditFocusPoint: null,
      activeBracketDragStart: null,
      zoomHandoffs: [],
    };
  }

  function ensureDocumentTab() {
    if (activeDocumentTab()) {
      return activeDocumentTab();
    }
    const tab = createDocumentTab();
    documentTabs.push(tab);
    setActiveDocumentTabId(tab.id);
    return tab;
  }

  function saveActiveDocumentTabState() {
    const tab = activeDocumentTab();
    if (!tab) {
      return;
    }
    for (const key of tabStateKeys) {
      tab[key] = state[key];
    }
    tab.zoomPercent = getZoomPercent();
    tab.title = documentTitleFromState();
    syncWindowTitle();
  }

  async function restoreDocumentTabState(tab) {
    for (const key of tabStateKeys) {
      state[key] = tab[key];
    }
    setStoredZoomPercent(Number(tab.zoomPercent || 100));
    setActiveSelectionGesture(null);
    textEditorLayer.replaceChildren();
    setActiveTextEditor(null);
    await syncEngineToolState();
    syncZoomControl();
    renderSecondaryToolbar();
    renderDocument();
    renderDocumentTabs();
    syncWindowTitle();
  }

  function documentTitleFromState() {
    const fileName = state.currentFileName || fileNameFromPath(state.currentPath) || fileNameFromPath(state.currentFilePath);
    if (fileName) {
      return fileName;
    }
    const title = String(state.currentDocument?.document?.title || "").trim();
    return title || "Untitled";
  }

  function documentTitleWithDirtyMarker(title, dirty) {
    return dirty ? `${title || "Untitled"} *` : title || "Untitled";
  }

  function currentDocumentSaveFingerprint() {
    return state.currentDocument ? JSON.stringify(state.currentDocument) : null;
  }

  function currentDocumentRevision() {
    const revision = state.editorEngine?.revision?.();
    return Number.isFinite(Number(revision)) ? Number(revision) : null;
  }

  function markCurrentDocumentSaved() {
    state.savedDocumentJson = null;
    state.savedRevision = currentDocumentRevision();
    state.unsavedDocument = false;
    const tab = activeDocumentTab();
    if (tab) {
      tab.savedDocumentJson = state.savedDocumentJson;
      tab.savedRevision = state.savedRevision;
      tab.unsavedDocument = false;
    }
    refreshCommandAvailability();
    renderDocumentTabs();
    syncWindowTitle();
  }

  function markCurrentDocumentRecovered() {
    state.unsavedDocument = true;
    const tab = activeDocumentTab();
    if (tab) {
      tab.unsavedDocument = true;
    }
    refreshCommandAvailability();
    renderDocumentTabs();
    syncWindowTitle();
  }

  function activeTextEditorIsDirty() {
    return Boolean(getActiveTextEditor()?.hasUserEdited);
  }

  function activeTextEditorIsNewTextObject() {
    const target = getActiveTextEditor()?.session?.target;
    if (target?.kind !== "text-object") {
      return false;
    }
    return !(target.objectId || target.object_id);
  }

  function activeTextEditorHasVisibleText() {
    const text = String(getActiveTextEditor()?.plainText || "");
    return text.trim().length > 0;
  }

  async function closeActiveTextEditorForToolAction() {
    if (!getActiveTextEditor()) {
      return false;
    }
    const shouldCommit = !activeTextEditorIsNewTextObject() || activeTextEditorHasVisibleText();
    return finishActiveTextEditor(shouldCommit);
  }

  function currentDocumentIsDirty() {
    if (activeTextEditorIsDirty()) {
      return true;
    }
    if (state.unsavedDocument) {
      return true;
    }
    const revision = currentDocumentRevision();
    if (revision != null && state.savedRevision != null) {
      return revision !== state.savedRevision;
    }
    const fingerprint = currentDocumentSaveFingerprint();
    return !!fingerprint && state.savedDocumentJson != null && fingerprint !== state.savedDocumentJson;
  }

  function canSaveCurrentDocument() {
    return currentDocumentIsDirty();
  }

  function isOleEditFilePath(path) {
    const fileName = fileNameFromPath(path).toLowerCase();
    return fileName.startsWith("chemsema-ole-edit-") && fileName.endsWith(".ccjs");
  }

  function markCurrentDocumentOfficeSynced() {
    state.oleSyncedDocumentJson = currentDocumentSaveFingerprint();
    state.oleSyncedRevision = currentDocumentRevision();
    const tab = activeDocumentTab();
    if (tab) {
      tab.oleSyncedDocumentJson = state.oleSyncedDocumentJson;
      tab.oleSyncedRevision = state.oleSyncedRevision;
    }
  }

  function tabDocumentFingerprint(tab) {
    return tab?.currentDocument ? JSON.stringify(tab.currentDocument) : null;
  }

  function tabDocumentRevision(tab) {
    try {
      const revision = tab?.editorEngine?.revision?.();
      return Number.isFinite(Number(revision)) ? Number(revision) : null;
    } catch {
      return null;
    }
  }

  function documentTabIsDirty(tab) {
    if (!tab) {
      return false;
    }
    if (tab.id === getActiveDocumentTabId() && activeTextEditorIsDirty()) {
      return true;
    }
    if (tab.unsavedDocument) {
      return true;
    }
    const revision = tabDocumentRevision(tab);
    if (revision != null && tab.savedRevision != null) {
      return revision !== tab.savedRevision;
    }
    const fingerprint = tabDocumentFingerprint(tab);
    return !!fingerprint && tab.savedDocumentJson != null && fingerprint !== tab.savedDocumentJson;
  }

  async function buildOleEditPayloadForTab(tab) {
    const rawDocumentJson = await tab?.editorEngine?.documentJson?.();
    const documentJson = rawDocumentJson
      ? `${String(rawDocumentJson).trimEnd()}\n`
      : tab?.currentDocument
        ? `${JSON.stringify(tab.currentDocument, null, 2)}\n`
        : null;
    if (!documentJson || !String(documentJson).trim()) {
      return null;
    }
    let cdxml = null;
    try {
      cdxml = await tab.editorEngine?.documentCdxml?.() || null;
    } catch (error) {
      console.warn("Failed to build OLE edit CDXML payload", error);
    }
    return {
      chemsemaFragmentJson: null,
      chemsemaDocumentJson: documentJson,
      renderListJson: tab.editorEngine?.renderListJson?.() || null,
      cdxml,
      svg: null,
      text: cdxml,
    };
  }

  async function syncOleEditDocumentTabToOffice(tab, options = {}) {
    if (
      !(desktopFileHost?.writeOleEditPayload || desktopFileHost?.writeTransientPath)
      || !tab?.currentFilePath
      || !isOleEditFilePath(tab.currentFilePath)
    ) {
      return false;
    }
    if (!tab.currentDocument) {
      return false;
    }
    const fingerprint = tabDocumentFingerprint(tab);
    const revision = tabDocumentRevision(tab);
    if (
      !options.force
      && tab.oleSyncedDocumentJson === fingerprint
      && (revision == null || tab.oleSyncedRevision === revision)
    ) {
      return false;
    }
    const payload = await buildOleEditPayloadForTab(tab);
    if (!payload) {
      return false;
    }
    if (desktopFileHost.writeOleEditPayload) {
      await desktopFileHost.writeOleEditPayload(tab.currentFilePath, payload);
    } else {
      await desktopFileHost.writeTransientPath(tab.currentFilePath, payload.chemsemaDocumentJson);
    }
    tab.oleSyncedDocumentJson = fingerprint;
    tab.oleSyncedRevision = revision;
    if (options.markSaved) {
      tab.savedDocumentJson = fingerprint;
      tab.savedRevision = revision;
    }
    if (tab.id === getActiveDocumentTabId()) {
      state.oleSyncedDocumentJson = tab.oleSyncedDocumentJson;
      state.oleSyncedRevision = tab.oleSyncedRevision;
      if (options.markSaved) {
        state.savedDocumentJson = tab.savedDocumentJson;
        state.savedRevision = tab.savedRevision;
      }
    }
    refreshCommandAvailability();
    return true;
  }

  async function autoSaveAllOleEditDocumentTabs() {
    await finishActiveTextEditor(true);
    if (state.editorEngine) {
      await syncDocumentFromEngine();
    }
    saveActiveDocumentTabState();
    for (const tab of documentTabs) {
      await syncOleEditDocumentTabToOffice(tab, { force: true });
    }
  }

  async function handleDocumentCommandCommitted(event) {
    if (event.deferDocumentSync) {
      renderDocumentTabs();
      syncWindowTitle();
      refreshCommandAvailability();
      scheduleDeferredDocumentSync();
      console.debug?.("[chemsema] document command committed", {
        type: event.commandType,
        revision: event.revision,
        source: event.source,
        deferredSync: true,
      });
      return;
    }
    saveActiveDocumentTabState();
    const tab = activeDocumentTab();
    if (tab) {
      await syncOleEditDocumentTabToOffice(tab);
    }
    renderDocumentTabs();
    syncWindowTitle();
    refreshCommandAvailability();
    console.debug?.("[chemsema] document command committed", {
      type: event.commandType,
      revision: event.revision,
      source: event.source,
    });
  }

  function fileNameFromPath(path) {
    return String(path || "").split(/[\\/]/).filter(Boolean).pop() || "";
  }

  function normalizedFilePathKey(path) {
    const value = String(path || "").trim();
    if (!value) {
      return "";
    }
    return value.replace(/\\/g, "/").replace(/\/+/g, "/").toLowerCase();
  }

  function documentTabForFilePath(path) {
    const key = normalizedFilePathKey(path);
    if (!key) {
      return null;
    }
    return documentTabs.find((tab) => normalizedFilePathKey(tab.currentFilePath) === key) || null;
  }

  function updateActiveDocumentTabTitle() {
    const tab = activeDocumentTab();
    if (!tab) {
      return;
    }
    const nextTitle = documentTitleFromState();
    if (tab.title !== nextTitle) {
      tab.title = nextTitle;
      renderDocumentTabs();
    }
  }

  function renderDocumentTabs() {
    if (!documentTabsRoot) {
      return;
    }
    documentTabsRoot.innerHTML = documentTabs.map((tab) => {
      const active = tab.id === getActiveDocumentTabId();
      const baseTitle = tab.title || "Untitled";
      const title = escapeHtml(documentTitleWithDirtyMarker(baseTitle, documentTabIsDirty(tab)));
      const dragging = getDocumentTabInteractions()?.isDetaching(tab.id);
      return `
        <div class="document-tab${active ? " is-active" : ""}${dragging ? " is-dragging" : ""}" role="tab" tabindex="0" aria-selected="${active ? "true" : "false"}" data-document-tab-id="${tab.id}" title="${title}">
          <span class="document-tab-title">${title}</span>
          <button class="document-tab-close" type="button" data-document-tab-close="${tab.id}" aria-label="Close ${title}" title="Close">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10"/><path d="M17 7 7 17"/></svg>
          </button>
        </div>
      `;
    }).join("");
  }

  async function activateDocumentTab(tabId) {
    if (tabId === getActiveDocumentTabId()) {
      return;
    }
    const nextTab = documentTabs.find((tab) => tab.id === tabId);
    if (!nextTab) {
      return;
    }
    await finishActiveTextEditor(true);
    saveActiveDocumentTabState();
    setActiveDocumentTabId(nextTab.id);
    await restoreDocumentTabState(nextTab);
  }

  async function closeDocumentTab(tabId, options = {}) {
    const activeTabIdBeforeClose = getActiveDocumentTabId();
    let index = documentTabs.findIndex((tab) => tab.id === tabId);
    if (index < 0) {
      return false;
    }
    if (!options.skipUnsavedPrompt && !await confirmCloseDocumentTab(tabId)) {
      return false;
    }
    index = documentTabs.findIndex((tab) => tab.id === tabId);
    if (index < 0) {
      return true;
    }
    const closing = documentTabs[index];
    const wasActive = closing.id === getActiveDocumentTabId();
    if (wasActive) {
      await finishActiveTextEditor(true);
      saveActiveDocumentTabState();
    }
    await syncOleEditDocumentTabToOffice(closing, { force: true });
    await closing.editorEngine?.free?.();
    await closing.documentEngine?.free?.();
    documentTabs.splice(index, 1);
    if (!documentTabs.length) {
      const tab = createDocumentTab();
      documentTabs.push(tab);
      setActiveDocumentTabId(tab.id);
      await restoreDocumentTabState(tab);
      await resetEditorEngine();
      renderDocument();
      fitView();
      saveActiveDocumentTabState();
      renderDocumentTabs();
      return true;
    }
    if (wasActive) {
      const previousActiveTab = activeTabIdBeforeClose !== tabId
        ? documentTabs.find((tab) => tab.id === activeTabIdBeforeClose)
        : null;
      const nextTab = previousActiveTab || documentTabs[Math.max(0, Math.min(index, documentTabs.length - 1))];
      setActiveDocumentTabId(nextTab.id);
      await restoreDocumentTabState(nextTab);
    } else {
      renderDocumentTabs();
    }
    return true;
  }

  return { activeDocumentTab, createDocumentTab, ensureDocumentTab, saveActiveDocumentTabState, restoreDocumentTabState, documentTitleFromState, documentTitleWithDirtyMarker, currentDocumentSaveFingerprint, currentDocumentRevision, markCurrentDocumentSaved, markCurrentDocumentRecovered, activeTextEditorIsDirty, activeTextEditorIsNewTextObject, activeTextEditorHasVisibleText, closeActiveTextEditorForToolAction, currentDocumentIsDirty, canSaveCurrentDocument, isOleEditFilePath, markCurrentDocumentOfficeSynced, tabDocumentFingerprint, tabDocumentRevision, documentTabIsDirty, buildOleEditPayloadForTab, syncOleEditDocumentTabToOffice, autoSaveAllOleEditDocumentTabs, handleDocumentCommandCommitted, fileNameFromPath, normalizedFilePathKey, documentTabForFilePath, updateActiveDocumentTabTitle, renderDocumentTabs, activateDocumentTab, closeDocumentTab };
}
