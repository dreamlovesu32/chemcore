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
        await runSafe(options.chooseAndOpenDocument, "Open failed", "Failed to open document");
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
      } else if (command === "new") {
        options.state.currentPath = null;
        options.state.currentFileName = null;
        options.state.currentFilePath = null;
        options.resetEditorEngine();
        options.renderDocument();
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
      await runSafe(options.chooseAndOpenDocument, "Open failed", "Failed to open document");
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
    if (command === "new") {
      options.state.currentPath = null;
      options.state.currentFileName = null;
      options.state.currentFilePath = null;
      options.resetEditorEngine();
      options.renderDocument();
      options.fitView();
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
    const [path] = paths;
    if (!path) {
      return;
    }
    await runSafe(() => options.openDocumentPath(path), "Open failed", "Failed to open dropped document");
  });
  options.desktopFileHost.takeStartupOpenPaths?.().then(async (paths) => {
    const [path] = Array.isArray(paths) ? paths : [];
    if (path) {
      await runSafe(() => options.openDocumentPath(path), "Open failed", "Failed to open startup document");
    }
  });
}

function bindFileInput(options) {
  options.openFileInput.addEventListener("change", async () => {
    const [file] = Array.from(options.openFileInput.files || []);
    options.openFileInput.value = "";
    try {
      await options.openDocumentFile(file);
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
    const target = event.target;
    if (options.getActiveTextEditor()?.root?.contains?.(target)) {
      if (event.key === "Escape") {
        options.finishActiveTextEditor(false);
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
    if (runHoverEndpointShortcut(event, options)) {
      event.preventDefault();
    }
  });
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

function runHoverEndpointShortcut(event, options) {
  const label = hoverEndpointShortcutLabelForEvent(event, options);
  if (!label) {
    return false;
  }
  const changed = options.state.editorEngine?.replaceHoveredEndpointLabel?.(label);
  if (!changed) {
    return false;
  }
  options.syncDocumentFromEngine();
  options.renderDocument();
  return true;
}

function bindToolButtons(options) {
  document.querySelectorAll("[data-tool]").forEach((button) => {
    button.addEventListener("click", () => {
      setActiveTool(button, options);
    });
  });
}

function setActiveTool(toolButton, options) {
  const { editorState, state } = options;
  const nextTool = toolButton?.dataset?.tool || editorState.activeTool;
  if (editorState.activeTool === "text" && nextTool !== "text") {
    options.finishActiveTextEditor(true);
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
  options.syncEngineToolState();
  options.renderSecondaryToolbar();
  options.syncCanvasCursor();
  if (options.isEditingRustDocument()) {
    options.renderEditorOverlay(options.currentEditorRenderList());
  }
}

function bindDocumentStylePreset(options) {
  options.documentStylePresetInput?.addEventListener("change", (event) => {
    options.finishActiveTextEditor(true);
    options.editorState.documentStylePreset = event.target.value || "default";
    options.syncEngineToolState();
    if (options.isEditingRustDocument()) {
      options.syncDocumentFromEngine();
      options.renderDocument();
    }
  });
}

function bindSecondaryToolbar(options) {
  bindToolbarColorPickers(options);

  options.secondaryToolbar?.addEventListener("click", (event) => {
    if (handleColorPickerClick(event, options)) {
      return;
    }
    const button = event.target.closest("[data-secondary-value]");
    if (!button) {
      return;
    }
    handleSecondaryToolbarValue(button.dataset.secondaryValue, options);
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

function bindToolbarColorPickers(options) {
  let drag = null;
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
    const left = Math.max(4, Math.min(window.innerWidth - 176, (pointerX ?? rect.left) - 5));
    picker.style.setProperty("--color-panel-left", `${left}px`);
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
  });

  options.secondaryToolbar?.addEventListener("pointermove", (event) => {
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    if (!drag.opened) {
      return;
    }
    updateDragHover(event);
    event.preventDefault();
  });

  options.secondaryToolbar?.addEventListener("pointerup", (event) => {
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    window.clearTimeout(drag.timer);
    const activeDrag = drag;
    drag = null;
    if (!activeDrag.opened) {
      return;
    }
    const target = updateDragHover(event);
    clearHovered();
    if (target?.dataset?.colorSwatchValue) {
      applyToolbarColor(activeDrag.picker?.dataset?.colorPrefix, target.dataset.colorSwatchValue, options);
      closeColorPickers();
    } else if (target?.hasAttribute?.("data-color-other")) {
      openColorDialog(currentColorForPrefix(activeDrag.picker?.dataset?.colorPrefix, options), (color) => {
        applyToolbarColor(activeDrag.picker?.dataset?.colorPrefix, color, options);
      }, options);
      closeColorPickers();
    }
    event.preventDefault();
  });

  document.addEventListener("pointerdown", (event) => {
    if (!event.target.closest?.(".color-picker")) {
      closeColorPickers();
    }
  });
}

function handleColorPickerClick(event, options) {
  const swatch = event.target.closest("[data-color-swatch-value]");
  if (swatch) {
    const picker = swatch.closest(".color-picker");
    applyToolbarColor(picker?.dataset?.colorPrefix, swatch.dataset.colorSwatchValue, options);
    closeColorPickers();
    event.preventDefault();
    return true;
  }
  const other = event.target.closest("[data-color-other]");
  if (other) {
    const picker = other.closest(".color-picker");
    openColorDialog(currentColorForPrefix(picker?.dataset?.colorPrefix, options), (color) => {
      applyToolbarColor(picker?.dataset?.colorPrefix, color, options);
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
      picker.style.setProperty("--color-panel-left", `${Math.max(4, Math.min(window.innerWidth - 176, rect.left - 5))}px`);
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

function handleSecondaryToolbarValue(value, options) {
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
    applyToolbarColor("text-color", editorState.textColor, options);
  } else if (value?.startsWith("text-color-")) {
    const color = colorFromToolbarValue(value, "text-color-");
    if (color) {
      applyToolbarColor("text-color", color, options);
    }
  } else if (value === "selection-color-apply") {
    applyToolbarColor("selection-color", editorState.selectionColor || editorState.textColor, options);
  } else if (value?.startsWith("selection-color-")) {
    const color = colorFromToolbarValue(value, "selection-color-");
    if (color) {
      applyToolbarColor("selection-color", color, options);
    }
  } else if (value === "select-free" || value === "select-box") {
    editorState.selectMode = value.replace("select-", "");
  } else if (/^(align-|distribute-|flip-)/.test(value || "")) {
    options.applySelectionArrangeCommand(value);
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
  } else if (value?.startsWith("ring-") || value === "benzene") {
    editorState.template = value;
  } else if (value === "shape-color-apply") {
    applyToolbarColor("shape-color", editorState.shapeColor, options);
  } else if (value?.startsWith("shape-color-")) {
    const color = colorFromToolbarValue(value, "shape-color-");
    if (color) {
      applyToolbarColor("shape-color", color, options);
    }
  }
  options.syncEngineToolState();
  if (arrowOptionChanged) {
    options.applyArrowOptionsToSelection();
  }
  options.renderSecondaryToolbar();
  options.focusActiveTextEditor();
}

function currentColorForPrefix(prefix, options) {
  if (prefix === "shape-color") {
    return options.editorState.shapeColor;
  }
  if (prefix === "selection-color") {
    return options.editorState.selectionColor || options.editorState.textColor;
  }
  return options.editorState.textColor;
}

function applyToolbarColor(prefix, color, options) {
  const normalized = normalizeHexColor(color) || "#000000";
  const { editorState } = options;
  if (prefix === "shape-color") {
    editorState.shapeColor = normalized;
    options.syncEngineToolState();
    options.applySelectionColor?.(normalized);
  } else if (prefix === "selection-color") {
    editorState.selectionColor = normalized;
    editorState.textColor = normalized;
    editorState.shapeColor = normalized;
    options.applySelectionColor?.(normalized);
  } else {
    editorState.textColor = normalized;
    if (options.getActiveTextEditor?.()) {
      options.applyTextInlineStyle({ color: normalized });
    } else {
      options.applySelectionColor?.(normalized);
    }
  }
  options.renderSecondaryToolbar();
  options.focusActiveTextEditor();
}

function colorFromToolbarValue(value, prefix) {
  const hex = String(value || "").slice(prefix.length);
  return /^[0-9a-fA-F]{6}$/.test(hex) ? `#${hex.toLowerCase()}` : null;
}

function openColorDialog(currentColor, onPick, options) {
  const existing = document.querySelector(".color-dialog-backdrop");
  existing?.remove();
  let selected = normalizeHexColor(currentColor) || "#000000";
  const backdrop = document.createElement("div");
  backdrop.className = "color-dialog-backdrop";
  backdrop.innerHTML = colorDialogHtml(selected, options);
  document.body.appendChild(backdrop);
  const picker = backdrop.querySelector(".color-dialog-native");
  const preview = backdrop.querySelector(".color-dialog-preview");
  const hexInput = backdrop.querySelector('[data-color-field="hex"]');
  const rgbInputs = Array.from(backdrop.querySelectorAll("[data-rgb-field]"));
  const close = () => backdrop.remove();
  const sync = (color) => {
    selected = normalizeHexColor(color) || selected;
    picker.value = selected;
    preview.style.setProperty("--swatch", selected);
    hexInput.value = selected.toUpperCase();
    const { r, g, b } = hexToRgb(selected);
    for (const input of rgbInputs) {
      input.value = String({ r, g, b }[input.dataset.rgbField]);
    }
    backdrop.querySelectorAll(".color-dialog-chip").forEach((chip) => {
      chip.classList.toggle("is-selected", normalizeHexColor(chip.dataset.colorDialogValue) === selected);
    });
  };
  picker.addEventListener("input", () => sync(picker.value));
  hexInput.addEventListener("change", () => sync(hexInput.value));
  for (const input of rgbInputs) {
    input.addEventListener("change", () => {
      const values = Object.fromEntries(rgbInputs.map((field) => [
        field.dataset.rgbField,
        clampRgb(field.value),
      ]));
      sync(rgbToHex(values.r, values.g, values.b));
    });
  }
  backdrop.addEventListener("click", (event) => {
    if (event.target === backdrop || event.target.closest("[data-color-dialog-cancel]") || event.target.closest(".color-dialog-close")) {
      close();
      return;
    }
    const chip = event.target.closest("[data-color-dialog-value]");
    if (chip) {
      sync(chip.dataset.colorDialogValue);
      return;
    }
    if (event.target.closest("[data-color-dialog-ok]")) {
      onPick(selected);
      close();
    }
  });
  backdrop.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      close();
    }
  });
  backdrop.tabIndex = -1;
  backdrop.focus();
  sync(selected);
}

function colorDialogHtml(selected, options) {
  const basics = [
    "#ff7777", "#ffff77", "#77ff77", "#00e878", "#77e6e6", "#006bd6", "#f46bb4", "#ee66ee",
    "#ff0000", "#ffff00", "#66ff00", "#00ff3b", "#1fd6d6", "#0b75a8", "#ff00dd", "#ff0090",
    "#8b3d3d", "#ff7438", "#00e800", "#007a68", "#004b88", "#7a7de0", "#820047", "#f20073",
    "#900000", "#ff7900", "#007000", "#007748", "#0000ff", "#00007d", "#800080", "#7500ff",
    "#4b0000", "#8a4b00", "#004b00", "#004b4b", "#000075", "#00004b", "#3d003d", "#310075",
    "#000000", "#808000", "#808040", "#808080", "#408080", "#c0c0c0", "#3a003a", "#ffffff",
  ];
  const extras = (options.getDocumentColors?.() || [])
    .map(normalizeHexColor)
    .filter(Boolean)
    .filter((color, index, list) => list.indexOf(color) === index)
    .filter((color) => !basics.includes(color));
  const basicChips = [...basics, ...extras.slice(0, 16)].map((color) => colorChipHtml(color, selected)).join("");
  const customChips = Array.from({ length: 16 }, (_, index) => {
    const color = extras[index + 16] || "#000000";
    return colorChipHtml(color, selected);
  }).join("");
  const { r, g, b } = hexToRgb(selected);
  return `
    <div class="color-dialog" role="dialog" aria-modal="true" aria-label="颜色">
      <div class="color-dialog-titlebar">
        <span>颜色</span>
        <button class="color-dialog-close" type="button" aria-label="Close">×</button>
      </div>
      <div class="color-dialog-body">
        <section>
          <p class="color-dialog-label">基本颜色(B):</p>
          <div class="color-dialog-basic-grid">${basicChips}</div>
          <div class="color-dialog-custom">
            <p class="color-dialog-label">自定义颜色(C):</p>
            <div class="color-dialog-custom-grid">${customChips}</div>
          </div>
        </section>
        <section class="color-dialog-main">
          <input class="color-dialog-native" type="color" value="${selected}" aria-label="Color picker">
          <div>
            <div class="color-dialog-controls">
              <div class="color-dialog-preview" style="--swatch:${selected}"></div>
              <div class="color-dialog-fields">
                <label class="color-dialog-field"><span>Hex:</span><input data-color-field="hex" value="${selected.toUpperCase()}"></label>
                <label class="color-dialog-field"><span>红(R):</span><input data-rgb-field="r" type="number" min="0" max="255" value="${r}"></label>
                <label class="color-dialog-field"><span>绿(G):</span><input data-rgb-field="g" type="number" min="0" max="255" value="${g}"></label>
                <label class="color-dialog-field"><span>蓝(U):</span><input data-rgb-field="b" type="number" min="0" max="255" value="${b}"></label>
              </div>
            </div>
            <div class="color-dialog-actions">
              <button type="button" data-color-dialog-ok>确定</button>
              <button type="button" data-color-dialog-cancel>取消</button>
            </div>
          </div>
        </section>
      </div>
    </div>
  `;
}

function colorChipHtml(color, selected) {
  return `<button class="color-dialog-chip${normalizeHexColor(color) === normalizeHexColor(selected) ? " is-selected" : ""}" type="button" data-color-dialog-value="${color}" style="--swatch:${color}" aria-label="${color}"></button>`;
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

function hexToRgb(color) {
  const hex = normalizeHexColor(color) || "#000000";
  return {
    r: Number.parseInt(hex.slice(1, 3), 16),
    g: Number.parseInt(hex.slice(3, 5), 16),
    b: Number.parseInt(hex.slice(5, 7), 16),
  };
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
