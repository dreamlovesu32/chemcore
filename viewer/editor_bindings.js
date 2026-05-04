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
      if (options.runEditorCommand(command)) {
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
  document.addEventListener("keydown", (event) => {
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
    if (command && options.runEditorCommand(command)) {
      event.preventDefault();
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
  options.secondaryToolbar?.addEventListener("click", (event) => {
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
  } else if (value?.startsWith("text-")) {
    const colors = { "text-black": "#000000", "text-red": "#ff0000", "text-blue": "#0000ff", "text-green": "#0a8f3c" };
    editorState.textColor = colors[value] || editorState.textColor;
    options.applyTextInlineStyle({ color: editorState.textColor });
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
  } else if (value?.startsWith("shape-color-")) {
    const colors = {
      "shape-color-black": "#000000",
      "shape-color-red": "#ff0000",
      "shape-color-blue": "#0000ff",
      "shape-color-green": "#008000",
    };
    editorState.shapeColor = colors[value] || editorState.shapeColor;
  }
  options.syncEngineToolState();
  if (arrowOptionChanged) {
    options.applyArrowOptionsToSelection();
  }
  options.renderSecondaryToolbar();
  options.focusActiveTextEditor();
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
