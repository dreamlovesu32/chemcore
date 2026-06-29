const UNSAVED_CLOSE_DECISION = {
  SAVE: "save",
  DISCARD: "discard",
  CANCEL: "cancel",
};
const REPEAT_UNIT_UNGROUP_WARNING_KEY = "chemcore:hide-repeat-unit-ungroup-warning";

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
  let activeUnsavedChangesDialog = null;
  let activeRepeatUnitUngroupDialog = null;
  let windowCloseGuardInProgress = false;
  let forceWindowClose = false;

  function bindDesktopWindowChrome() {
    if (!isDesktopShell || !desktopFileHost?.available) {
      return;
    }
    void desktopFileHost.listenWindowCloseRequested?.(async (event) => {
      if (forceWindowClose) {
        return;
      }
      event?.preventDefault?.();
      await requestCloseWindow();
    });
    desktopTitlebar?.querySelectorAll("[data-window-command]").forEach((button) => {
      button.addEventListener("click", async () => {
        const command = button.dataset.windowCommand;
        if (command === "minimize") {
          await desktopFileHost.minimizeWindow?.();
        } else if (command === "maximize") {
          await desktopFileHost.toggleMaximizeWindow?.();
          await syncDesktopMaximizedState();
        } else if (command === "close") {
          await requestCloseWindow();
        }
      });
    });
    document.addEventListener("pointerdown", handleDesktopWindowDragPointerDown, true);
    desktopTitlebar?.querySelectorAll("[data-titlebar-drag-region]").forEach((region) => {
      region.addEventListener("dblclick", async (event) => {
        event.preventDefault();
        await desktopFileHost.toggleMaximizeWindow?.();
        await syncDesktopMaximizedState();
      });
      region.addEventListener("pointerdown", async (event) => {
        if (event.button !== 0 || event.detail > 1) {
          return;
        }
        await desktopFileHost.startWindowDrag?.();
      });
    });
    window.addEventListener("resize", () => {
      syncDesktopMaximizedState();
    }, { passive: true });
    syncDesktopMaximizedState();
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

  function makeUnsavedChangesButton(label, decision, className = "") {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.dataset.unsavedDecision = decision;
    if (className) {
      button.className = className;
    }
    return button;
  }

  function showUnsavedChangesDialog(title) {
    if (activeUnsavedChangesDialog) {
      return activeUnsavedChangesDialog;
    }
    activeUnsavedChangesDialog = new Promise((resolve) => {
      const previousFocus = document.activeElement;
      const root = document.createElement("div");
      root.className = "unsaved-changes-dialog";
      root.setAttribute("role", "alertdialog");
      root.setAttribute("aria-modal", "true");
      root.setAttribute("aria-labelledby", "unsaved-changes-title");
      root.setAttribute("aria-describedby", "unsaved-changes-message");

      const backdrop = document.createElement("div");
      backdrop.className = "unsaved-changes-backdrop";
      const windowDragStrip = document.createElement("div");
      windowDragStrip.className = "desktop-modal-window-drag-strip";
      windowDragStrip.dataset.desktopWindowDragRegion = "true";
      windowDragStrip.setAttribute("aria-hidden", "true");
      const panel = document.createElement("section");
      panel.className = "unsaved-changes-panel";
      const panelDragStrip = document.createElement("div");
      panelDragStrip.className = "desktop-dialog-panel-drag-strip";
      panelDragStrip.dataset.desktopWindowDragRegion = "true";
      panelDragStrip.setAttribute("aria-hidden", "true");

      const heading = document.createElement("h2");
      heading.id = "unsaved-changes-title";
      heading.className = "unsaved-changes-title";
      heading.textContent = "Save changes?";

      const message = document.createElement("p");
      message.id = "unsaved-changes-message";
      message.className = "unsaved-changes-message";
      message.textContent = `Do you want to save changes to "${title || "Untitled"}" before closing?`;

      const actions = document.createElement("div");
      actions.className = "unsaved-changes-actions";
      const saveButton = makeUnsavedChangesButton("Save", UNSAVED_CLOSE_DECISION.SAVE, "is-primary");
      const discardButton = makeUnsavedChangesButton("Don't Save", UNSAVED_CLOSE_DECISION.DISCARD);
      const cancelButton = makeUnsavedChangesButton("Cancel", UNSAVED_CLOSE_DECISION.CANCEL);
      actions.append(saveButton, discardButton, cancelButton);
      panel.append(panelDragStrip, heading, message, actions);
      root.append(backdrop, windowDragStrip, panel);

      const finish = (decision) => {
        root.remove();
        document.removeEventListener("keydown", onKeyDown, true);
        activeUnsavedChangesDialog = null;
        if (previousFocus && typeof previousFocus.focus === "function") {
          previousFocus.focus({ preventScroll: true });
        }
        resolve(decision);
      };
      const onKeyDown = (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          finish(UNSAVED_CLOSE_DECISION.CANCEL);
        }
      };
      actions.addEventListener("click", (event) => {
        const button = event.target.closest("[data-unsaved-decision]");
        if (!button) {
          return;
        }
        finish(button.dataset.unsavedDecision || UNSAVED_CLOSE_DECISION.CANCEL);
      });
      document.addEventListener("keydown", onKeyDown, true);
      document.body.append(root);
      saveButton.focus({ preventScroll: true });
    });
    return activeUnsavedChangesDialog;
  }

  function repeatUnitUngroupWarningHidden() {
    try {
      return localStorage.getItem(REPEAT_UNIT_UNGROUP_WARNING_KEY) === "true";
    } catch {
      return false;
    }
  }

  function setRepeatUnitUngroupWarningHidden(hidden) {
    try {
      if (hidden) {
        localStorage.setItem(REPEAT_UNIT_UNGROUP_WARNING_KEY, "true");
      } else {
        localStorage.removeItem(REPEAT_UNIT_UNGROUP_WARNING_KEY);
      }
    } catch {
      // Preferences are best-effort; the document command should not depend on storage.
    }
  }

  function showRepeatUnitUngroupDialog() {
    if (activeRepeatUnitUngroupDialog) {
      return activeRepeatUnitUngroupDialog;
    }
    activeRepeatUnitUngroupDialog = new Promise((resolve) => {
      const previousFocus = document.activeElement;
      const root = document.createElement("div");
      root.className = "repeat-unit-ungroup-dialog";
      root.setAttribute("role", "alertdialog");
      root.setAttribute("aria-modal", "true");
      root.setAttribute("aria-labelledby", "repeat-unit-ungroup-title");
      root.setAttribute("aria-describedby", "repeat-unit-ungroup-message");

      const backdrop = document.createElement("div");
      backdrop.className = "repeat-unit-ungroup-backdrop";
      const windowDragStrip = document.createElement("div");
      windowDragStrip.className = "desktop-modal-window-drag-strip";
      windowDragStrip.dataset.desktopWindowDragRegion = "true";
      windowDragStrip.setAttribute("aria-hidden", "true");
      const panel = document.createElement("section");
      panel.className = "repeat-unit-ungroup-panel";
      const panelDragStrip = document.createElement("div");
      panelDragStrip.className = "desktop-dialog-panel-drag-strip";
      panelDragStrip.dataset.desktopWindowDragRegion = "true";
      panelDragStrip.setAttribute("aria-hidden", "true");

      const heading = document.createElement("h2");
      heading.id = "repeat-unit-ungroup-title";
      heading.className = "repeat-unit-ungroup-title";
      heading.textContent = "Ungroup repeat unit?";

      const message = document.createElement("p");
      message.id = "repeat-unit-ungroup-message";
      message.className = "repeat-unit-ungroup-message";
      message.textContent = "Ungrouping will remove the repeat-count link from the number label. The bracket remains part of the molecule.";

      const footer = document.createElement("div");
      footer.className = "repeat-unit-ungroup-footer";
      const rememberLabel = document.createElement("label");
      rememberLabel.className = "repeat-unit-ungroup-remember";
      const rememberCheckbox = document.createElement("input");
      rememberCheckbox.type = "checkbox";
      rememberCheckbox.value = "1";
      rememberLabel.append(rememberCheckbox, document.createTextNode("Don't show again"));

      const actions = document.createElement("div");
      actions.className = "repeat-unit-ungroup-actions";
      const ungroupButton = document.createElement("button");
      ungroupButton.type = "button";
      ungroupButton.className = "is-primary";
      ungroupButton.dataset.repeatUnitUngroupDecision = "confirm";
      ungroupButton.textContent = "Ungroup";
      const cancelButton = document.createElement("button");
      cancelButton.type = "button";
      cancelButton.dataset.repeatUnitUngroupDecision = "cancel";
      cancelButton.textContent = "Cancel";
      actions.append(ungroupButton, cancelButton);
      footer.append(rememberLabel, actions);
      panel.append(panelDragStrip, heading, message, footer);
      root.append(backdrop, windowDragStrip, panel);

      const finish = (confirmed) => {
        if (confirmed && rememberCheckbox.checked) {
          setRepeatUnitUngroupWarningHidden(true);
        }
        root.remove();
        document.removeEventListener("keydown", onKeyDown, true);
        activeRepeatUnitUngroupDialog = null;
        if (previousFocus && typeof previousFocus.focus === "function") {
          previousFocus.focus({ preventScroll: true });
        }
        resolve(confirmed);
      };
      const onKeyDown = (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          finish(false);
        }
      };
      actions.addEventListener("click", (event) => {
        const button = event.target.closest("[data-repeat-unit-ungroup-decision]");
        if (!button) {
          return;
        }
        finish(button.dataset.repeatUnitUngroupDecision === "confirm");
      });
      document.addEventListener("keydown", onKeyDown, true);
      document.body.append(root);
      ungroupButton.focus({ preventScroll: true });
    });
    return activeRepeatUnitUngroupDialog;
  }

  async function confirmRepeatUnitUngroupIfNeeded() {
    if (repeatUnitUngroupWarningHidden()) {
      return true;
    }
    const hasRepeatUnitGroup = !!state.editorEngine?.selectionHasRepeatUnitGroups?.();
    if (!hasRepeatUnitGroup) {
      return true;
    }
    return showRepeatUnitUngroupDialog();
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
    const decision = await showUnsavedChangesDialog(freshTab?.title || "Untitled");
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
    confirmRepeatUnitUngroupIfNeeded,
    prepareDocumentTabForDirtyCheck,
    saveDocumentTabBeforeClose,
    confirmUnsavedChangesForTab,
    confirmCloseDocumentTab,
    confirmCloseAllDocumentTabs,
    requestCloseWindow,
    bindBrowserBeforeUnloadGuard,
  };
}
