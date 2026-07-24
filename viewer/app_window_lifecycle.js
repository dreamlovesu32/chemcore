import { createAppDialogHost, UNSAVED_CLOSE_DECISION } from "./app_dialog_host.js";

export function createAppWindowLifecycleHost(options) {
  const {
    state,
    documentTabs,
    desktopFileHost,
    desktopTitlebar,
  } = options;
  const isDesktopShell = Boolean(options.isDesktopShell());
  const documentTabRuntime = {
    get activeDocumentTabId() {
      return options.getActiveDocumentTabId();
    },
    set activeDocumentTabId(value) {
      options.setActiveDocumentTabId(value);
    },
  };
  const activeDocumentTab = (...args) => options.activeDocumentTab(...args);
  const activeTextEditorIsDirty = (...args) => options.activeTextEditorIsDirty(...args);
  const documentTabIsDirty = (...args) => options.documentTabIsDirty(...args);
  const finishActiveTextEditor = (...args) => options.finishActiveTextEditor(...args);
  const syncDocumentFromEngine = (...args) => options.syncDocumentFromEngine(...args);
  const saveActiveDocumentTabState = (...args) => options.saveActiveDocumentTabState(...args);
  const renderDocumentTabs = (...args) => options.renderDocumentTabs(...args);
  const syncWindowTitle = (...args) => options.syncWindowTitle(...args);
  const refreshCommandAvailability = (...args) => options.refreshCommandAvailability(...args);
  const activateDocumentTab = (...args) => options.activateDocumentTab(...args);
  const saveCurrentDocument = (...args) => options.saveCurrentDocument(...args);
  const isAbortError = (...args) => options.isAbortError(...args);
  const autoSaveAllOleEditDocumentTabs = (...args) => options.autoSaveAllOleEditDocumentTabs(...args);
  const uiActions = options.uiActions;
  const dialogs = createAppDialogHost({ state });
  let windowCloseGuardInProgress = false;
  let forceWindowClose = false;

  function bindDesktopWindowChrome() {
    if (!isDesktopShell || !desktopFileHost?.available) {
      return;
    }
    void desktopFileHost.listenWindowCloseRequested?.(uiActions.listener("window.close-requested", async (event) => {
      if (forceWindowClose) {
        return;
      }
      event?.preventDefault?.();
      await requestCloseWindow();
    }));
    desktopTitlebar?.querySelectorAll("[data-window-command]").forEach((button) => {
      button.addEventListener("click", uiActions.listener("window.command", async () => {
        const command = button.dataset.windowCommand;
        if (command === "minimize") {
          await desktopFileHost.minimizeWindow?.();
        } else if (command === "maximize") {
          await desktopFileHost.toggleMaximizeWindow?.();
          await syncDesktopMaximizedState();
        } else if (command === "close") {
          await requestCloseWindow();
        }
      }));
    });
    document.addEventListener(
      "pointerdown",
      uiActions.listener("window.drag-region", handleDesktopWindowDragPointerDown),
      true,
    );
    desktopTitlebar?.querySelectorAll("[data-titlebar-drag-region]").forEach((region) => {
      region.addEventListener("dblclick", uiActions.listener("window.toggle-maximize", async (event) => {
        event.preventDefault();
        await desktopFileHost.toggleMaximizeWindow?.();
        await syncDesktopMaximizedState();
      }));
      region.addEventListener("pointerdown", uiActions.listener("window.start-drag", async (event) => {
        if (event.button !== 0 || event.detail > 1) {
          return;
        }
        await desktopFileHost.startWindowDrag?.();
      }));
    });
    window.addEventListener(
      "resize",
      uiActions.listener("window.sync-maximized", syncDesktopMaximizedState),
      { passive: true },
    );
    void uiActions.start("window.sync-maximized", syncDesktopMaximizedState);
  }

  function desktopWindowDragRegionFromEvent(event) {
    if (!isDesktopShell || !desktopFileHost?.available || !event.target?.closest) {
      return null;
    }
    const region = event.target.closest("[data-desktop-window-drag-region]");
    if (!region) {
      return null;
    }
    const interactive = event.target.closest(
      "button, input, select, textarea, a, [role='button'], [contenteditable='true']",
    );
    return interactive && region.contains(interactive) ? null : region;
  }

  async function handleDesktopWindowDragPointerDown(event) {
    const region = desktopWindowDragRegionFromEvent(event);
    if (!region || event.button !== 0 || event.detail > 1) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    await desktopFileHost.startWindowDrag?.();
  }

  async function syncDesktopMaximizedState() {
    if (!isDesktopShell || !desktopFileHost?.isWindowMaximized) {
      return;
    }
    const maximized = await desktopFileHost.isWindowMaximized().catch(() => false);
    document.body.classList.toggle("is-window-maximized", !!maximized);
  }

  async function prepareDocumentTabForDirtyCheck(tab) {
    if (!tab || tab.id !== documentTabRuntime.activeDocumentTabId) {
      return;
    }
    await finishActiveTextEditor(true);
    if (state.editorEngine) {
      await syncDocumentFromEngine();
    }
    saveActiveDocumentTabState();
    renderDocumentTabs();
    syncWindowTitle();
    refreshCommandAvailability();
  }

  async function saveDocumentTabBeforeClose(tab) {
    const target = documentTabs.find((entry) => entry.id === tab?.id);
    if (!target) {
      return true;
    }
    if (target.id !== documentTabRuntime.activeDocumentTabId) {
      await activateDocumentTab(target.id);
    }
    try {
      const saved = await saveCurrentDocument();
      if (!saved) {
        return false;
      }
      saveActiveDocumentTabState();
      renderDocumentTabs();
      syncWindowTitle();
      return true;
    } catch (error) {
      if (isAbortError(error)) {
        return false;
      }
      console.error("Save before close failed", error);
      window.alert?.(`Save failed: ${error.message || error}`);
      return false;
    }
  }

  async function confirmUnsavedChangesForTab(tab) {
    await prepareDocumentTabForDirtyCheck(tab);
    const freshTab = documentTabs.find((entry) => entry.id === tab?.id) || tab;
    if (!documentTabIsDirty(freshTab)) {
      return true;
    }
    const decision = await dialogs.showUnsavedChangesDialog(freshTab?.title || "Untitled");
    if (decision === UNSAVED_CLOSE_DECISION.CANCEL) {
      return false;
    }
    if (decision === UNSAVED_CLOSE_DECISION.DISCARD) {
      return true;
    }
    return saveDocumentTabBeforeClose(freshTab);
  }

  async function confirmCloseDocumentTab(tabId) {
    const tab = documentTabs.find((entry) => entry.id === tabId);
    if (!tab) {
      return true;
    }
    return confirmUnsavedChangesForTab(tab);
  }

  async function confirmCloseAllDocumentTabs() {
    const active = activeDocumentTab();
    const orderedTabs = [
      ...(active ? [active] : []),
      ...documentTabs.filter((tab) => tab.id !== active?.id),
    ];
    for (const tab of orderedTabs) {
      if (!documentTabs.some((entry) => entry.id === tab.id)) {
        continue;
      }
      if (!await confirmUnsavedChangesForTab(tab)) {
        return false;
      }
    }
    return true;
  }

  async function requestCloseWindow() {
    if (windowCloseGuardInProgress) {
      return false;
    }
    windowCloseGuardInProgress = true;
    try {
      if (!await confirmCloseAllDocumentTabs()) {
        return false;
      }
      await autoSaveAllOleEditDocumentTabs();
      forceWindowClose = true;
      try {
        if (desktopFileHost?.destroyWindow) {
          await desktopFileHost.destroyWindow();
        } else if (desktopFileHost?.closeWindow) {
          await desktopFileHost.closeWindow();
        } else {
          window.close();
        }
      } catch (error) {
        console.error("Window close failed", error);
        window.alert?.(`Close failed: ${error.message || error}`);
        return false;
      }
      return true;
    } finally {
      windowCloseGuardInProgress = false;
      if (!document.hidden) {
        forceWindowClose = false;
      }
    }
  }

  function bindBrowserBeforeUnloadGuard() {
    if (isDesktopShell) {
      return;
    }
    window.addEventListener("beforeunload", (event) => {
      if (!activeTextEditorIsDirty() && !documentTabs.some((tab) => documentTabIsDirty(tab))) {
        return;
      }
      event.preventDefault();
      event.returnValue = "";
    });
  }


  return {
    bindDesktopWindowChrome,
    syncDesktopMaximizedState,
    confirmRepeatUnitUngroupIfNeeded: dialogs.confirmRepeatUnitUngroupIfNeeded,
    showKernelNoticeDialog: dialogs.showKernelNoticeDialog,
    prepareDocumentTabForDirtyCheck,
    saveDocumentTabBeforeClose,
    confirmUnsavedChangesForTab,
    confirmCloseDocumentTab,
    confirmCloseAllDocumentTabs,
    requestCloseWindow,
    bindBrowserBeforeUnloadGuard,
  };
}
