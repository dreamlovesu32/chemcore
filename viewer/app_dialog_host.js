const UNSAVED_CLOSE_DECISION = {
  SAVE: "save",
  DISCARD: "discard",
  CANCEL: "cancel",
};
const REPEAT_UNIT_UNGROUP_WARNING_KEY = "chemsema:hide-repeat-unit-ungroup-warning";

export function createAppDialogHost({ state }) {
  let activeUnsavedChangesDialog = null;
  let activeRepeatUnitUngroupDialog = null;
  let activeKernelNoticeDialog = null;

  function showKernelNoticeDialog(spec) {
    if (!spec?.message || activeKernelNoticeDialog) {
      return activeKernelNoticeDialog || Promise.resolve();
    }
    activeKernelNoticeDialog = new Promise((resolve) => {
      const previousFocus = document.activeElement;
      const root = document.createElement("div");
      root.className = "repeat-unit-ungroup-dialog";
      root.setAttribute("role", "alertdialog");
      root.setAttribute("aria-modal", "true");
      const backdrop = document.createElement("div");
      backdrop.className = "repeat-unit-ungroup-backdrop";
      const panel = document.createElement("section");
      panel.className = "repeat-unit-ungroup-panel";
      const heading = document.createElement("h2");
      heading.className = "repeat-unit-ungroup-title";
      heading.textContent = spec.title || "ChemSema";
      const message = document.createElement("p");
      message.className = "repeat-unit-ungroup-message";
      message.textContent = spec.message;
      const actions = document.createElement("div");
      actions.className = "repeat-unit-ungroup-actions";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "is-primary";
      button.textContent = spec.buttons?.[0]?.label || "OK";
      actions.append(button);
      panel.append(heading, message, actions);
      root.append(backdrop, panel);
      const finish = () => {
        root.remove();
        document.removeEventListener("keydown", onKeyDown, true);
        activeKernelNoticeDialog = null;
        previousFocus?.focus?.({ preventScroll: true });
        resolve();
      };
      const onKeyDown = (event) => {
        if (event.key === "Escape" || event.key === "Enter") {
          event.preventDefault();
          finish();
        }
      };
      button.addEventListener("click", finish);
      document.addEventListener("keydown", onKeyDown, true);
      document.body.append(root);
      button.focus({ preventScroll: true });
    });
    return activeKernelNoticeDialog;
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
        if (button) {
          finish(button.dataset.unsavedDecision || UNSAVED_CLOSE_DECISION.CANCEL);
        }
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
      // Preference persistence is independent from the document command.
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
        previousFocus?.focus?.({ preventScroll: true });
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
        if (button) {
          finish(button.dataset.repeatUnitUngroupDecision === "confirm");
        }
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
    if (!state.editorEngine?.selectionHasRepeatUnitGroups?.()) {
      return true;
    }
    return showRepeatUnitUngroupDialog();
  }

  return {
    showUnsavedChangesDialog,
    showKernelNoticeDialog,
    confirmRepeatUnitUngroupIfNeeded,
  };
}

export { UNSAVED_CLOSE_DECISION };
