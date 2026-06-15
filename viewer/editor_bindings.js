import {
  SHAPE_TOOL_ICON_STYLES,
  SHAPE_TOOL_STYLE_KINDS,
  arrowTypeSupportsHeadSize,
} from "./toolbar.js";

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

const TOOL_SHORTCUTS = {
  " ": { tool: "select" },
  x: { tool: "bond", secondaryValue: "bond-single" },
  e: { tool: "arrow" },
  t: { tool: "text" },
  g: { tool: "tlc-plate" },
};

const BOND_TOOL_SHORTCUTS = {
  "1": "bond-single",
  "2": "bond-double",
  "3": "bond-triple",
  d: "bond-dashed",
  b: "bond-bold",
  w: "bond-wavy",
  y: "bond-wedge",
  h: "bond-hashed-wedge",
};

const HOVER_BOND_SHORTCUT_STYLES = {
  "1": "single-plain",
  "2": "double-center",
  "3": "triple-plain",
  d: "single-dashed",
  b: "single-bold",
  w: "single-wavy",
  y: "single-bold-wedged",
  h: "single-hashed-wedged",
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
        void options.desktopFileHost?.traceEvent?.("editorBindings.runSafe.error", {
          alertPrefix,
          logMessage,
          error,
        });
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
        await options.desktopFileHost?.traceEvent?.("editorBindings.desktopRunSafe.error", {
          alertPrefix,
          logMessage,
          error,
        });
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
    await options.desktopFileHost?.traceEvent?.("editorBindings.openPaths.begin", { paths });
    for (const path of paths) {
      if (path) {
        await runSafe(() => options.openDocumentPathInTab(path), "Open failed", "Failed to open dropped document");
      }
    }
    await options.desktopFileHost?.traceEvent?.("editorBindings.openPaths.done", { count: paths.length });
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
      return;
    }
    if (await runHoverBondShortcut(event, options)) {
      event.preventDefault();
      return;
    }
    if (await runEditorToolShortcut(event, options)) {
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
  if (commandKey && !event.shiftKey && event.key.toLowerCase() === "g") {
    return "group-selection";
  }
  if (commandKey && event.shiftKey && event.key.toLowerCase() === "g") {
    return "ungroup-selection";
  }
  if (commandKey && !event.shiftKey && event.key.toLowerCase() === "j") {
    return "join-selection";
  }
  if (!commandKey && !event.altKey && event.key === "F2") {
    return "bring-front";
  }
  if (!commandKey && !event.altKey && event.key === "F3") {
    return "send-back";
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

async function runHoverBondShortcut(event, options) {
  if (!options.isEditingRustDocument() || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || event.repeat) {
    return false;
  }
  const style = HOVER_BOND_SHORTCUT_STYLES[event.key.toLowerCase()];
  if (!style) {
    return false;
  }
  const usesCommandEngine = Boolean(options.commandEngine?.executeEngineCommand);
  const result = usesCommandEngine
    ? await options.commandEngine.executeEngineCommand(
      {
        type: "apply-bond-style",
        payload: { source: "hover", style },
      },
      () => options.state.editorEngine?.applyHoveredBondStyle?.(style),
    )
    : { changed: await options.state.editorEngine?.applyHoveredBondStyle?.(style) };
  const changed = !!result.changed;
  if (!changed) {
    return false;
  }
  if (!usesCommandEngine) {
    await options.syncDocumentFromEngine?.();
  }
  options.renderDocument();
  return true;
}

async function runEditorToolShortcut(event, options) {
  if (!options.isEditingRustDocument() || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || event.repeat) {
    return false;
  }
  const key = event.key === " " ? " " : event.key.toLowerCase();
  const bondValue = BOND_TOOL_SHORTCUTS[key];
  if (bondValue) {
    await options.activateEditorTool?.("bond");
    await handleSecondaryToolbarValue(bondValue, options);
    return true;
  }
  const shortcut = TOOL_SHORTCUTS[key];
  if (!shortcut) {
    return false;
  }
  await options.activateEditorTool?.(shortcut.tool);
  if (shortcut.secondaryValue) {
    await handleSecondaryToolbarValue(shortcut.secondaryValue, options);
  }
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
  const { editorState } = options;
  const nextTool = toolButton?.dataset?.tool || editorState.activeTool;
  const elementPaletteWasOpen = Boolean(document.querySelector('.quick-palette.is-open[data-mode="element"]'));
  if (nextTool === "element") {
    document.dispatchEvent(new CustomEvent(
      elementPaletteWasOpen ? "chemcore:quick-palette-toggle" : "chemcore:quick-palette-open-mode",
      { detail: { mode: "element" } },
    ));
    return;
  }
  await options.activateEditorTool?.(nextTool);
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
    const rect = picker.getBoundingClientRect();
    const left = Math.max(4, Math.min(window.innerWidth - 138, (pointerX ?? rect.left) - 5));
    picker.style.setProperty("--color-panel-left", `${left}px`);
    picker.style.setProperty("--color-panel-top", `${Math.min(window.innerHeight - 150, rect.bottom + 6)}px`);
    picker.classList.add("is-open");
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
    drag = {
      picker,
      pointerId: event.pointerId,
      opened: false,
      timer: window.setTimeout(() => {
        drag.opened = true;
        openPicker(picker, event.clientX);
      }, 360),
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
  if (editorState.elementPlacementActive && !value?.startsWith("element-symbol-")) {
    editorState.elementPlacementActive = false;
  }
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
  } else if (value === "arrow-type-nogo-cross" || value === "arrow-type-nogo-hash") {
    editorState.arrowType = "solid";
    editorState.arrowHeadSize = "small";
    editorState.arrowCurve = "270";
    editorState.arrowHeadStyle = "full";
    editorState.arrowTailStyle = "none";
    editorState.arrowHead = true;
    editorState.arrowTail = false;
    editorState.arrowNoGo = value === "arrow-type-nogo-cross" ? "cross" : "hash";
    editorState.arrowBold = false;
    arrowOptionChanged = true;
  } else if (value?.startsWith("arrow-type-")) {
    const previousArrowType = editorState.arrowType;
    editorState.arrowType = value.replace("arrow-type-", "");
    if (isOpenArrowType(editorState.arrowType) && !isOpenArrowType(previousArrowType)) {
      editorState.arrowHeadSize = "large";
    }
    if (isEquilibriumArrowType(editorState.arrowType) && !isEquilibriumArrowType(previousArrowType)) {
      editorState.arrowHeadSize = "small";
    }
    normalizeArrowToolbarStyle(editorState);
    arrowOptionChanged = true;
  } else if (value?.startsWith("arrow-size-")) {
    editorState.arrowHeadSize = value.replace("arrow-size-", "");
    normalizeArrowToolbarStyle(editorState);
    arrowOptionChanged = true;
  } else if (value?.startsWith("arrow-curve-")) {
    editorState.arrowCurve = value.replace("arrow-curve-", "");
    normalizeCurvedArrowStyle(editorState);
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
    const shouldCancel = editorState.arrowHeadStyle === next;
    editorState.arrowHeadStyle = shouldCancel ? "full" : next;
    normalizeArrowToolbarStyle(editorState);
    arrowOptionChanged = true;
  } else if (value === "arrow-tail-left" || value === "arrow-tail-right") {
    const next = value === "arrow-tail-left" ? "left" : "right";
    editorState.arrowTailStyle = editorState.arrowTailStyle === next ? "none" : next;
    editorState.arrowTail = editorState.arrowTailStyle !== "none";
    arrowOptionChanged = true;
  } else if (value === "arrow-head-full") {
    editorState.arrowHeadStyle = "full";
    normalizeArrowToolbarStyle(editorState);
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
  } else if (value?.startsWith("element-symbol-")) {
    const [, symbol, atomicNumber] = value.match(/^element-symbol-([A-Za-z]{1,2})-(\d{1,3})$/) || [];
    if (symbol) {
      await options.applyElementPaletteSelection?.(symbol);
      editorState.elementSymbol = symbol;
      editorState.elementAtomicNumber = Number(atomicNumber) || editorState.elementAtomicNumber || 15;
      if (options.getActiveTextEditor?.()) {
        options.insertElementSymbol?.(symbol);
      }
    }
  } else if (value?.startsWith("shape-kind-")) {
    const nextShapeKind = value.replace("shape-kind-", "");
    const nextShapeStyle = shapeStyleForKind(editorState, nextShapeKind);
    editorState.shapeKind = nextShapeKind;
    editorState.shapeStyle = nextShapeStyle;
  } else if (value?.startsWith("shape-style-")) {
    const shapeSelection = shapeSelectionFromToolbarValue(value);
    if (shapeSelection) {
      editorState.shapeKind = shapeSelection.kind;
      editorState.shapeStyle = shapeSelection.style;
      setShapeStyleForKind(editorState, shapeSelection.kind, shapeSelection.style);
    } else {
      editorState.shapeStyle = value.replace("shape-style-", "");
      setShapeStyleForKind(editorState, editorState.shapeKind, editorState.shapeStyle);
    }
  } else if (value?.startsWith("orbital-combo-")) {
    const [, template, style, phase] = value.match(/^orbital-combo-([a-z0-9]+)-([a-z]+)-([a-z]+)$/) || [];
    if (template && style && phase) {
      editorState.orbitalTemplate = template;
      editorState.orbitalStyle = style;
      editorState.orbitalPhase = phase;
    }
  } else if (value?.startsWith("orbital-template-")) {
    editorState.orbitalTemplate = value.replace("orbital-template-", "");
  } else if (value?.startsWith("orbital-style-")) {
    editorState.orbitalStyle = value.replace("orbital-style-", "");
  } else if (value?.startsWith("orbital-phase-")) {
    editorState.orbitalPhase = value.replace("orbital-phase-", "");
  } else if (value?.startsWith("ring-") || value?.startsWith("chair-") || value === "benzene") {
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
  options.syncCanvasCursor();
  options.focusActiveTextEditor();
}

function shapeSelectionFromToolbarValue(value) {
  const prefix = "shape-style-";
  if (!value?.startsWith(prefix)) {
    return null;
  }
  const body = value.slice(prefix.length);
  for (const kind of SHAPE_TOOL_STYLE_KINDS) {
    const style = body.startsWith(`${kind}-`) ? body.slice(kind.length + 1) : null;
    if (SHAPE_TOOL_ICON_STYLES.includes(style)) {
      return { kind, style };
    }
  }
  return null;
}

function shapeStyleForKind(editorState, kind) {
  if (!SHAPE_TOOL_STYLE_KINDS.includes(kind)) {
    return "solid";
  }
  const saved = editorState.shapeStyleByKind?.[kind];
  if (SHAPE_TOOL_ICON_STYLES.includes(saved)) {
    return saved;
  }
  if (editorState.shapeKind === kind && SHAPE_TOOL_ICON_STYLES.includes(editorState.shapeStyle)) {
    return editorState.shapeStyle;
  }
  return "solid";
}

function setShapeStyleForKind(editorState, kind, style) {
  if (!SHAPE_TOOL_STYLE_KINDS.includes(kind) || !SHAPE_TOOL_ICON_STYLES.includes(style)) {
    return;
  }
  editorState.shapeStyleByKind = {
    ...(editorState.shapeStyleByKind || {}),
    [kind]: style,
  };
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

function normalizeSolidArrowStyle(editorState) {
  editorState.arrowType = "solid";
  editorState.arrowHeadSize = normalizedArrowHeadSize(editorState.arrowHeadSize);
  editorState.arrowHeadStyle = normalizedArrowHeadStyle(editorState.arrowHeadStyle);
  editorState.arrowTailStyle = "none";
  editorState.arrowHead = true;
  editorState.arrowTail = false;
  editorState.arrowNoGo = "none";
  editorState.arrowBold = false;
}

function normalizedArrowHeadSize(size) {
  return size === "large" || size === "medium" || size === "small" ? size : "small";
}

function normalizeArrowToolbarStyle(editorState) {
  if (editorState.arrowType === "solid") {
    normalizeSolidArrowStyle(editorState);
    return;
  }
  if (isEquilibriumArrowType(editorState.arrowType)) {
    normalizeEquilibriumArrowStyle(editorState);
    return;
  }
  if (editorState.arrowType === "curved" || editorState.arrowType === "curved-mirror") {
    normalizeCurvedArrowStyle(editorState);
    return;
  }
  if (isOpenArrowType(editorState.arrowType)) {
    normalizeOpenArrowStyle(editorState);
    return;
  }
  normalizeArrowEndpointOptions(editorState);
}

function normalizeCurvedArrowStyle(editorState) {
  editorState.arrowType = editorState.arrowType === "curved-mirror" ? "curved-mirror" : "curved";
  editorState.arrowHeadSize = "small";
  editorState.arrowCurve = normalizedArrowCurve(editorState.arrowCurve);
  editorState.arrowHeadStyle = normalizedArrowHeadStyle(editorState.arrowHeadStyle);
  editorState.arrowTailStyle = "none";
  editorState.arrowHead = true;
  editorState.arrowTail = false;
  editorState.arrowNoGo = "none";
  editorState.arrowBold = false;
}

function normalizeEquilibriumArrowStyle(editorState) {
  editorState.arrowType = editorState.arrowType === "unequal-equilibrium" ? "unequal-equilibrium" : "equilibrium";
  editorState.arrowHeadSize = normalizedArrowHeadSize(editorState.arrowHeadSize);
  editorState.arrowCurve = "270";
  editorState.arrowHeadStyle = editorState.arrowHeadStyle === "right" ? "right" : "left";
  editorState.arrowTailStyle = editorState.arrowHeadStyle;
  editorState.arrowHead = true;
  editorState.arrowTail = true;
  editorState.arrowNoGo = "none";
  editorState.arrowBold = false;
}

function normalizedArrowHeadStyle(style) {
  return style === "left" || style === "right" ? style : "full";
}

function normalizedArrowCurve(curve) {
  return curve === "270" || curve === "180" || curve === "120" || curve === "90" ? curve : "270";
}

function normalizeOpenArrowStyle(editorState) {
  editorState.arrowHeadSize = normalizedOpenArrowHeadSize(editorState.arrowHeadSize);
  editorState.arrowHeadStyle = "full";
  editorState.arrowTailStyle = "none";
  editorState.arrowHead = true;
  editorState.arrowTail = false;
  editorState.arrowNoGo = "none";
  editorState.arrowBold = false;
}

function normalizedOpenArrowHeadSize(size) {
  return size === "large" ? "large" : "small";
}

function isOpenArrowType(type) {
  return type === "hollow" || type === "open";
}

function isEquilibriumArrowType(type) {
  return type === "equilibrium" || type === "unequal-equilibrium";
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
