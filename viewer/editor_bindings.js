import { arrowTypeSupportsHeadSize } from "./toolbar.js";

const HOVER_ENDPOINT_SHORTCUT_LABELS = {
  h: "H",
  n: "N",
  o: "O",
  s: "S",
  P: "P",
  p: "Ph",
  f: "F",
  l: "Cl",
  b: "Br",
  i: "I",
  m: "Me",
  S: "Si",
  N: "Na",
  B: "B",
  d: "D",
};

export function bindEditorControls(options) {
  bindCommandButtons(options);
  bindFileInput(options);
  bindZoomInput(options);
  bindKeyboard(options);
  bindDesktopCommands(options);
  bindToolButtons(options);
  bindDocumentStylePreset(options);
  bindSecondaryToolbar(options);
}

function bindCommandButtons(options) {
  document.querySelectorAll("[data-command]").forEach((button) => {
    button.addEventListener("click", async () => {
      const command = button.dataset.command;
      if (command === "open") {
        await runSafe(options.chooseAndOpenDocumentTab, "Open failed", "Failed to open document");
        return;
      }
      if (command === "new") {
        await runSafe(options.newDocumentTab, "New failed", "Failed to create document tab");
        return;
      }
      if (command === "save") {
        await runSafe(options.saveCurrentDocument, "Save failed", "Failed to save document");
        return;
      }
      if (command === "save-as") {
        await runSafe(options.saveCurrentDocumentAs, "Save failed", "Failed to save document");
        return;
      }
      if (command === "save-cdxml") {
        await runSafe(options.saveCurrentDocumentCdxml, "Save CDXML failed", "Failed to save CDXML");
        return;
      }
      if (command === "save-svg") {
        await runSafe(options.saveCurrentDocumentSvg, "Save SVG failed", "Failed to save SVG");
        return;
      }
      if (command === "save-pdf") {
        await runSafe(options.saveCurrentDocumentPdf, "Save PDF failed", "Failed to save PDF");
        return;
      }
      if (command === "save-emf") {
        await runSafe(options.saveCurrentDocumentEmf, "Save EMF failed", "Failed to save EMF");
        return;
      }
      if (await options.runEditorCommand(command)) {
        return;
      }
      if (command === "zoom-in") {
        options.setZoomPercent(options.nextZoomStep(1));
      } else if (command === "zoom-out") {
        options.setZoomPercent(options.nextZoomStep(-1));
      } else if (command === "fit") {
        options.fitView();
      }
    });
  });

  async function runSafe(action, alertPrefix, logMessage) {
    try {
      await action();
    } catch (error) {
      if (!options.isAbortError(error)) {
        console.error(logMessage, error);
        window.alert?.(`${alertPrefix}: ${error.message || error}`);
      }
    }
  }
}

function bindDesktopCommands(options) {
  if (!options.desktopFileHost?.available) {
    return;
  }
  const runSafe = async (action, alertPrefix, logMessage) => {
    try {
      await action();
    } catch (error) {
      if (!options.isAbortError(error)) {
        console.error(logMessage, error);
        window.alert?.(`${alertPrefix}: ${error.message || error}`);
      }
    }
  };
  const runCommand = async (command) => {
    if (!command) {
      return;
    }
    if (command === "open") {
      await runSafe(options.chooseAndOpenDocumentTab, "Open failed", "Failed to open document");
      return;
    }
    if (command === "new") {
      await runSafe(options.newDocumentTab, "New failed", "Failed to create document tab");
      return;
    }
    if (command === "save") {
      await runSafe(options.saveCurrentDocument, "Save failed", "Failed to save document");
      return;
    }
    if (command === "save-as") {
      await runSafe(options.saveCurrentDocumentAs, "Save failed", "Failed to save document");
      return;
    }
    if (command === "save-cdxml") {
      await runSafe(options.saveCurrentDocumentCdxml, "Save CDXML failed", "Failed to save CDXML");
      return;
    }
    if (command === "save-svg") {
      await runSafe(options.saveCurrentDocumentSvg, "Save SVG failed", "Failed to save SVG");
      return;
    }
    if (command === "save-pdf") {
      await runSafe(options.saveCurrentDocumentPdf, "Save PDF failed", "Failed to save PDF");
      return;
    }
    if (command === "save-emf") {
      await runSafe(options.saveCurrentDocumentEmf, "Save EMF failed", "Failed to save EMF");
      return;
    }
    if (await options.runEditorCommand(command)) {
      return;
    }
    if (command === "zoom-in") {
      options.setZoomPercent(options.nextZoomStep(1));
    } else if (command === "zoom-out") {
      options.setZoomPercent(options.nextZoomStep(-1));
    } else if (command === "fit") {
      options.fitView();
    }
  };

  options.desktopFileHost.listenMenu(runCommand);
  options.desktopFileHost.listenOpenPaths(async (paths) => {
    for (const path of paths) {
      if (path) {
        await runSafe(() => options.openDocumentPathInTab(path), "Open failed", "Failed to open dropped document");
      }
    }
  });
}

function bindFileInput(options) {
  options.openFileInput.addEventListener("change", async () => {
    const [file] = Array.from(options.openFileInput.files || []);
    options.openFileInput.value = "";
    try {
      await options.openDocumentFileInTab(file);
    } catch (error) {
      console.error("Failed to open document", error);
      window.alert?.(`Open failed: ${error.message || error}`);
    }
  });
}

function bindZoomInput(options) {
  options.zoomInput?.addEventListener("change", () => {
    const parsed = Number.parseInt(String(options.zoomInput.value || ""), 10);
    options.setZoomPercent(Number.isFinite(parsed) ? parsed : options.getZoomPercent());
  });
}

function bindKeyboard(options) {
  document.addEventListener("keydown", async (event) => {
    if (await runGlobalFileShortcut(event, options)) {
      return;
    }
    const target = event.target;
    if (options.getActiveTextEditor()?.root?.contains?.(target)) {
      if (event.key === "Escape") {
        await options.finishActiveTextEditor(false);
        event.preventDefault();
      }
      return;
    }
    if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement) {
      return;
    }
    const command = keyboardCommand(event);
    if (command && options.isEditingRustDocument()) {
      event.preventDefault();
      await options.runEditorCommand(command);
      return;
    }
    if (await runHoverEndpointShortcut(event, options)) {
      event.preventDefault();
    }
  });
}

async function runGlobalFileShortcut(event, options) {
  const commandKey = event.ctrlKey || event.metaKey;
  if (!commandKey || event.altKey) {
    return false;
  }
  const key = event.key.toLowerCase();
  const run = async (action, label) => {
    event.preventDefault();
    try {
      await action();
    } catch (error) {
      if (!options.isAbortError(error)) {
        console.error(`${label} failed`, error);
        window.alert?.(`${label} failed: ${error.message || error}`);
      }
    }
  };
  if (key === "n") {
    await run(options.newDocumentTab, "New");
    return true;
  }
  if (key === "o") {
    await run(options.chooseAndOpenDocumentTab, "Open");
    return true;
  }
  if (key === "s") {
    await run(event.shiftKey ? options.saveCurrentDocumentAs : options.saveCurrentDocument, "Save");
    return true;
  }
  return false;
}

function keyboardCommand(event) {
  const commandKey = event.ctrlKey || event.metaKey;
  if (commandKey && event.key.toLowerCase() === "z" && !event.shiftKey) {
    return "undo";
  }
  if ((commandKey && event.key.toLowerCase() === "y") || (commandKey && event.shiftKey && event.key.toLowerCase() === "z")) {
    return "redo";
  }
  if (commandKey && event.key.toLowerCase() === "c") {
    return "copy";
  }
  if (commandKey && event.key.toLowerCase() === "x") {
    return "cut";
  }
  if (commandKey && event.key.toLowerCase() === "v") {
    return "paste";
  }
  if (commandKey && event.key.toLowerCase() === "a") {
    return "select-all";
  }
  if (event.key === "Delete" || event.key === "Backspace") {
    return "delete";
  }
  return null;
}

function hoverEndpointShortcutLabelForEvent(event, options) {
  if (!options.isEditingRustDocument()) {
    return null;
  }
  if (event.ctrlKey || event.metaKey || event.altKey) {
    return null;
  }
  if (event.key === "c") {
    return "C";
  }
  return HOVER_ENDPOINT_SHORTCUT_LABELS[event.key] || null;
}

async function runHoverEndpointShortcut(event, options) {
  const label = hoverEndpointShortcutLabelForEvent(event, options);
  if (!label) {
    return false;
  }
  const result = options.commandEngine?.executeEngineCommand
    ? await options.commandEngine.executeEngineCommand(
      {
        type: "replace-endpoint-label",
        payload: { label },
      },
      () => options.state.editorEngine?.replaceHoveredEndpointLabel?.(label),
    )
    : { changed: await options.state.editorEngine?.replaceHoveredEndpointLabel?.(label) };
  const changed = !!result.changed;
  if (!changed) {
    return false;
  }
  options.renderDocument();
  return true;
}

function bindToolButtons(options) {
  document.querySelectorAll("[data-tool]").forEach((button) => {
    button.addEventListener("click", async () => {
      await setActiveTool(button, options);
    });
  });
}

async function setActiveTool(toolButton, options) {
  const { editorState, state } = options;
  const nextTool = toolButton?.dataset?.tool || editorState.activeTool;
  if (editorState.activeTool === "text" && nextTool !== "text") {
    await options.finishActiveTextEditor(true);
  }
  if (editorState.activeTool === "select" && nextTool !== "select") {
    options.clearActiveSelectionGesture();
  }
  if (nextTool !== "bracket") {
    state.activeBracketDragStart = null;
  }
  editorState.activeTool = nextTool;
  document.querySelectorAll("[data-tool]").forEach((button) => {
    button.classList.toggle("is-active", button.dataset.tool === editorState.activeTool);
  });
  await options.syncEngineToolState();
  options.renderSecondaryToolbar();
  options.syncCanvasCursor();
  if (options.isEditingRustDocument()) {
    options.renderEditorOverlay(options.currentEditorRenderList());
  }
}

function bindDocumentStylePreset(options) {
  const button = options.documentStyleButton;
  const menu = options.documentStyleMenu;
  if (!button || !menu) {
    return;
  }

  const closeMenu = () => {
    menu.hidden = true;
    button.setAttribute("aria-expanded", "false");
  };
  const toggleMenu = () => {
    const nextHidden = !menu.hidden;
    menu.hidden = nextHidden;
    button.setAttribute("aria-expanded", nextHidden ? "false" : "true");
  };

  button.addEventListener("click", (event) => {
    event.preventDefault();
    toggleMenu();
  });

  menu.addEventListener("click", async (event) => {
    const item = event.target.closest("[data-document-style-preset]");
    if (!item) {
      return;
    }
    event.preventDefault();
    const preset = item.dataset.documentStylePreset || "default";
    closeMenu();
    await options.finishActiveTextEditor(true);
    const confirmed = await options.confirmApplyDocumentStylePreset?.(preset);
    if (!confirmed) {
      return;
    }
    const result = options.commandEngine?.executeEngineCommand
      ? await options.commandEngine.executeEngineCommand(
        {
          type: "apply-document-style",
          payload: {
            preset,
            scope: "document",
          },
        },
        () => options.state.editorEngine?.setDocumentStylePreset?.(preset),
        { compareDocument: true },
      )
      : { changed: await options.state.editorEngine?.setDocumentStylePreset?.(preset) };
    await options.syncEngineToolState();
    if (options.isEditingRustDocument() && result.changed !== false) {
      options.renderDocument();
    }
  });

  document.addEventListener("pointerdown", (event) => {
    if (menu.hidden || button.contains(event.target) || menu.contains(event.target)) {
      return;
    }
    closeMenu();
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      closeMenu();
    }
  });
}

function bindSecondaryToolbar(options) {
  bindToolbarColorPickers(options);

  options.secondaryToolbar?.addEventListener("click", async (event) => {
    if (await handleColorPickerClick(event, options)) {
      return;
    }
    const button = event.target.closest("[data-secondary-value]");
    if (!button) {
      return;
    }
    await handleSecondaryToolbarValue(button.dataset.secondaryValue, options);
  });

  options.secondaryToolbar?.addEventListener("change", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement || target instanceof HTMLSelectElement)) {
      return;
    }
    const control = target.dataset.textControl;
    if (control === "font") {
      options.editorState.textFontFamily = target.value || options.editorState.textFontFamily;
      options.applyTextInlineStyle({ fontFamily: options.editorState.textFontFamily });
    } else if (control === "size") {
      const size = Number(target.value || options.editorState.textFontSize);
      if (Number.isFinite(size) && size > 0) {
        options.setTextFontSize(size);
        options.applyTextInlineStyle({ fontSize: `${options.editorState.textFontSize}px` });
      }
    }
    options.renderSecondaryToolbar();
    options.focusActiveTextEditor();
  });
}

let suppressColorPickerClickUntil = 0;

function bindToolbarColorPickers(options) {
  let drag = null;
  const stopDragListeners = () => {
    window.removeEventListener("pointermove", handleDragMove);
    window.removeEventListener("pointerup", finishDrag);
    window.removeEventListener("pointercancel", cancelDrag);
  };
  const clearHovered = () => {
    document.querySelectorAll(".color-panel-swatch.is-hovered, .color-panel-other.is-hovered")
      .forEach((node) => node.classList.remove("is-hovered"));
  };
  const openPicker = (picker, pointerX = null) => {
    if (!picker) {
      return;
    }
    closeColorPickers(picker);
    picker.classList.add("is-open");
    const rect = picker.getBoundingClientRect();
    const left = Math.max(4, Math.min(window.innerWidth - 138, (pointerX ?? rect.left) - 5));
    picker.style.setProperty("--color-panel-left", `${left}px`);
    picker.style.setProperty("--color-panel-top", `${Math.min(window.innerHeight - 150, rect.bottom + 6)}px`);
  };
  const targetAtPointer = (event) => {
    const element = document.elementFromPoint(event.clientX, event.clientY);
    return element?.closest?.("[data-color-swatch-value], [data-color-other]") || null;
  };
  const updateDragHover = (event) => {
    clearHovered();
    const target = targetAtPointer(event);
    target?.classList?.add("is-hovered");
    return target;
  };
  const handleDragMove = (event) => {
    if (!drag || drag.pointerId !== event.pointerId || !drag.opened) {
      return;
    }
    updateDragHover(event);
    event.preventDefault();
  };
  const finishDrag = async (event) => {
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    window.clearTimeout(drag.timer);
    const activeDrag = drag;
    drag = null;
    stopDragListeners();
    if (!activeDrag.opened) {
      return;
    }
    suppressColorPickerClickUntil = performance.now() + 450;
    const target = updateDragHover(event);
    clearHovered();
    if (target?.dataset?.colorSwatchValue) {
      await applyToolbarColor(activeDrag.picker?.dataset?.colorPrefix, target.dataset.colorSwatchValue, options);
    } else if (target?.hasAttribute?.("data-color-other")) {
      openColorDialog(currentColorForPrefix(activeDrag.picker?.dataset?.colorPrefix, options), async (color) => {
        await applyToolbarColor(activeDrag.picker?.dataset?.colorPrefix, color, options);
      }, options);
    }
    closeColorPickers();
    event.preventDefault();
  };
  const cancelDrag = (event) => {
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    window.clearTimeout(drag.timer);
    drag = null;
    stopDragListeners();
    clearHovered();
    closeColorPickers();
  };

  options.secondaryToolbar?.addEventListener("pointerdown", (event) => {
    const button = event.target.closest(".color-picker-button");
    if (!button) {
      return;
    }
    const picker = button.closest(".color-picker");
    const startsOnArrow = Boolean(event.target.closest("[data-color-picker-arrow]"));
    drag = {
      picker,
      pointerId: event.pointerId,
      opened: false,
      timer: window.setTimeout(() => {
        drag.opened = true;
        openPicker(picker, event.clientX);
      }, startsOnArrow ? 120 : 360),
    };
    button.setPointerCapture?.(event.pointerId);
    window.addEventListener("pointermove", handleDragMove);
    window.addEventListener("pointerup", finishDrag);
    window.addEventListener("pointercancel", cancelDrag);
  });

  document.addEventListener("pointerdown", (event) => {
    if (!event.target.closest?.(".color-picker")) {
      closeColorPickers();
    }
  });
}

async function handleColorPickerClick(event, options) {
  if (performance.now() < suppressColorPickerClickUntil) {
    event.preventDefault();
    event.stopPropagation();
    return true;
  }
  const swatch = event.target.closest("[data-color-swatch-value]");
  if (swatch) {
    const picker = swatch.closest(".color-picker");
    await applyToolbarColor(picker?.dataset?.colorPrefix, swatch.dataset.colorSwatchValue, options);
    closeColorPickers();
    event.preventDefault();
    return true;
  }
  const other = event.target.closest("[data-color-other]");
  if (other) {
    const picker = other.closest(".color-picker");
    openColorDialog(currentColorForPrefix(picker?.dataset?.colorPrefix, options), async (color) => {
      await applyToolbarColor(picker?.dataset?.colorPrefix, color, options);
    }, options);
    closeColorPickers();
    event.preventDefault();
    return true;
  }
  const arrow = event.target.closest("[data-color-picker-arrow]");
  const arrowButton = event.target.closest(".color-picker-button");
  const arrowByPosition = arrowButton && (() => {
    const rect = arrowButton.getBoundingClientRect();
    return event.clientX >= rect.right - 15 && event.clientY >= rect.bottom - 15;
  })();
  if (arrow || arrowByPosition) {
    const picker = (arrow || arrowButton).closest(".color-picker");
    if (picker?.classList.contains("is-open")) {
      picker.classList.remove("is-open");
    } else {
      closeColorPickers(picker);
      const rect = picker.getBoundingClientRect();
      picker.style.setProperty("--color-panel-left", `${Math.max(4, Math.min(window.innerWidth - 138, rect.left - 5))}px`);
      picker.style.setProperty("--color-panel-top", `${Math.min(window.innerHeight - 150, rect.bottom + 6)}px`);
      picker.classList.add("is-open");
    }
    event.preventDefault();
    return true;
  }
  return false;
}

function closeColorPickers(except = null) {
  document.querySelectorAll(".color-picker.is-open").forEach((picker) => {
    if (picker !== except) {
      picker.classList.remove("is-open");
    }
  });
}

async function handleSecondaryToolbarValue(value, options) {
  const { editorState } = options;
  let arrowOptionChanged = false;
  if (value?.startsWith("text-align-")) {
    editorState.textAlign = value.replace("text-align-", "");
    options.applyTextAlignment(editorState.textAlign);
  } else if (value === "text-bold") {
    editorState.textBold = !editorState.textBold;
    options.applyTextFormatCommand("bold");
  } else if (value === "text-italic") {
    editorState.textItalic = !editorState.textItalic;
    options.applyTextFormatCommand("italic");
  } else if (value === "text-underline") {
    editorState.textUnderline = !editorState.textUnderline;
    options.applyTextFormatCommand("underline");
  } else if (value === "text-chemical") {
    if (editorState.textScript === "chemical") {
      editorState.textScript = "normal";
      options.applyTextScript("normal");
    } else {
      editorState.textScript = "chemical";
      options.applyChemicalFormat();
    }
  } else if (value === "text-subscript") {
    editorState.textScript = "subscript";
    options.applyTextScript("subscript");
  } else if (value === "text-superscript") {
    editorState.textScript = "superscript";
    options.applyTextScript("superscript");
  } else if (value === "text-color-apply") {
    await applyToolbarColor("text-color", editorState.textColor, options);
  } else if (value?.startsWith("text-color-")) {
    const color = colorFromToolbarValue(value, "text-color-");
    if (color) {
      await applyToolbarColor("text-color", color, options);
    }
  } else if (value === "selection-color-apply") {
    await applyToolbarColor("selection-color", editorState.selectionColor || editorState.textColor, options);
  } else if (value?.startsWith("selection-color-")) {
    const color = colorFromToolbarValue(value, "selection-color-");
    if (color) {
      await applyToolbarColor("selection-color", color, options);
    }
  } else if (value === "select-free" || value === "select-box") {
    editorState.selectMode = value.replace("select-", "");
  } else if (/^(align-|distribute-|flip-)/.test(value || "")) {
    await options.applySelectionArrangeCommand(value);
  } else if (value?.startsWith("bond-")) {
    editorState.bondType = value.replace("bond-", "");
  } else if (value?.startsWith("arrow-type-")) {
    editorState.arrowType = value.replace("arrow-type-", "");
    arrowOptionChanged = normalizeArrowEndpointOptions(editorState);
  } else if (value?.startsWith("arrow-size-")) {
    editorState.arrowHeadSize = value.replace("arrow-size-", "");
    arrowOptionChanged = true;
  } else if (value?.startsWith("arrow-curve-")) {
    editorState.arrowCurve = value.replace("arrow-curve-", "");
    arrowOptionChanged = true;
  } else if (value === "arrow-line") {
    editorState.arrowHeadStyle = "none";
    editorState.arrowTailStyle = "none";
    editorState.arrowHead = false;
    editorState.arrowTail = false;
    arrowOptionChanged = true;
  } else if (value === "arrow-head") {
    editorState.arrowHeadStyle = editorState.arrowHeadStyle === "full" ? "none" : "full";
    editorState.arrowHead = editorState.arrowHeadStyle !== "none";
    arrowOptionChanged = true;
  } else if (value === "arrow-tail") {
    editorState.arrowTailStyle = editorState.arrowTailStyle === "full" ? "none" : "full";
    editorState.arrowTail = editorState.arrowTailStyle !== "none";
    arrowOptionChanged = true;
  } else if (value === "arrow-head-left" || value === "arrow-head-right") {
    const next = value === "arrow-head-left" ? "left" : "right";
    editorState.arrowHeadStyle = editorState.arrowHeadStyle === next ? "none" : next;
    editorState.arrowHead = editorState.arrowHeadStyle !== "none";
    arrowOptionChanged = true;
  } else if (value === "arrow-tail-left" || value === "arrow-tail-right") {
    const next = value === "arrow-tail-left" ? "left" : "right";
    editorState.arrowTailStyle = editorState.arrowTailStyle === next ? "none" : next;
    editorState.arrowTail = editorState.arrowTailStyle !== "none";
    arrowOptionChanged = true;
  } else if (value === "arrow-nogo-cross" || value === "arrow-nogo-hash") {
    const next = value === "arrow-nogo-cross" ? "cross" : "hash";
    editorState.arrowNoGo = editorState.arrowNoGo === next ? "none" : next;
    arrowOptionChanged = true;
  } else if (value === "arrow-bold") {
    editorState.arrowBold = !editorState.arrowBold;
    arrowOptionChanged = true;
  } else if (value?.startsWith("bracket-kind-")) {
    editorState.bracketKind = value.replace("bracket-kind-", "");
  } else if (value?.startsWith("symbol-kind-")) {
    editorState.symbolKind = value.replace("symbol-kind-", "");
  } else if (value?.startsWith("shape-kind-")) {
    editorState.shapeKind = value.replace("shape-kind-", "");
  } else if (value?.startsWith("shape-style-")) {
    editorState.shapeStyle = value.replace("shape-style-", "");
  } else if (value?.startsWith("orbital-template-")) {
    editorState.orbitalTemplate = value.replace("orbital-template-", "");
  } else if (value?.startsWith("orbital-style-")) {
    editorState.orbitalStyle = value.replace("orbital-style-", "");
  } else if (value?.startsWith("orbital-phase-")) {
    editorState.orbitalPhase = value.replace("orbital-phase-", "");
  } else if (value?.startsWith("ring-") || value === "benzene") {
    editorState.template = value;
  } else if (value === "shape-color-apply") {
    await applyToolbarColor("shape-color", editorState.shapeColor, options);
  } else if (value?.startsWith("shape-color-")) {
    const color = colorFromToolbarValue(value, "shape-color-");
    if (color) {
      await applyToolbarColor("shape-color", color, options);
    }
  } else if (value === "orbital-color-apply") {
    await applyToolbarColor("orbital-color", editorState.orbitalColor || editorState.shapeColor, options);
  } else if (value?.startsWith("orbital-color-")) {
    const color = colorFromToolbarValue(value, "orbital-color-");
    if (color) {
      await applyToolbarColor("orbital-color", color, options);
    }
  }
  await options.syncEngineToolState();
  if (arrowOptionChanged) {
    await options.applyArrowOptionsToSelection();
  }
  options.renderSecondaryToolbar();
  options.focusActiveTextEditor();
}

function currentColorForPrefix(prefix, options) {
  if (prefix === "orbital-color") {
    return options.editorState.orbitalColor || options.editorState.shapeColor;
  }
  if (prefix === "shape-color") {
    return options.editorState.shapeColor;
  }
  if (prefix === "selection-color") {
    return options.editorState.selectionColor || options.editorState.textColor;
  }
  return options.editorState.textColor;
}

async function applyToolbarColor(prefix, color, options) {
  const normalized = normalizeHexColor(color) || "#000000";
  const { editorState } = options;
  if (prefix === "orbital-color") {
    editorState.orbitalColor = normalized;
    await options.syncEngineToolState();
    await options.applySelectionColor?.(normalized);
  } else if (prefix === "shape-color") {
    editorState.shapeColor = normalized;
    await options.syncEngineToolState();
    await options.applySelectionColor?.(normalized);
  } else if (prefix === "selection-color") {
    editorState.selectionColor = normalized;
    editorState.textColor = normalized;
    editorState.shapeColor = normalized;
    await options.applySelectionColor?.(normalized);
  } else {
    editorState.textColor = normalized;
    if (options.getActiveTextEditor?.()) {
      options.applyTextInlineStyle({ color: normalized });
    } else {
      await options.applySelectionColor?.(normalized);
    }
  }
  options.renderSecondaryToolbar();
  options.focusActiveTextEditor();
}

function colorFromToolbarValue(value, prefix) {
  const hex = String(value || "").slice(prefix.length);
  return /^[0-9a-fA-F]{6}$/.test(hex) ? `#${hex.toLowerCase()}` : null;
}

export function openColorDialog(currentColor, onPick, options) {
  const selected = normalizeHexColor(currentColor) || "#000000";
  if (typeof options.colorHost?.chooseColor !== "function") {
    return;
  }
  options.colorHost.chooseColor(selected, colorDialogCustomColors(options))
    .then((color) => {
      const normalized = normalizeHexColor(color);
      if (normalized) {
        onPick(normalized);
      }
    })
    .catch((error) => {
      console.warn("[chemcore] color host failed to choose a color", error);
    });
}

function colorDialogCustomColors(options) {
  const colors = (options.getDocumentColors?.() || []).map(normalizeHexColor).filter(Boolean);
  return colors.filter((color, index) => colors.indexOf(color) === index).slice(0, 16);
}

function normalizeHexColor(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(raw)) {
    return raw;
  }
  if (/^#[0-9a-f]{3}$/.test(raw)) {
    return `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`;
  }
  const match = raw.match(/^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/);
  if (match) {
    return rgbToHex(match[1], match[2], match[3]);
  }
  return null;
}

function rgbToHex(r, g, b) {
  return `#${[r, g, b].map((value) => clampRgb(value).toString(16).padStart(2, "0")).join("")}`;
}

function clampRgb(value) {
  return Math.max(0, Math.min(255, Number.parseInt(String(value || 0), 10) || 0));
}

function normalizeArrowEndpointOptions(editorState) {
  if (arrowTypeSupportsHeadSize(editorState.arrowType)) {
    return true;
  }
  if (editorState.arrowType === "hollow" || editorState.arrowType === "open") {
    editorState.arrowHeadSize = "large";
  }
  if (editorState.arrowHeadStyle === "left" || editorState.arrowHeadStyle === "right") {
    editorState.arrowHeadStyle = "full";
  }
  if (editorState.arrowTailStyle === "left" || editorState.arrowTailStyle === "right") {
    editorState.arrowTailStyle = "full";
  }
  editorState.arrowHead = editorState.arrowHeadStyle !== "none";
  editorState.arrowTail = editorState.arrowTailStyle !== "none";
  editorState.arrowNoGo = "none";
  return true;
}
