export function createEditorStateRuntimeHost(scope) {
  const { state, editorState, document, selectionChemistrySummary, isEditingRustDocument, parseEngineJson, renderDocumentPrimitiveChange, currentEditorRenderList, currentEditorInteractionRenderList, syncEditorSelectionRenderListFromEngine, currentDocumentBoundsContainsPoint, documentBoundsHitPadScreenPx, invalidateEditorEngineReadCache, renderEditorOverlay, syncSelectCursorForPoint, finishActiveTextEditor, engineHost, syncTextSymbolPaletteFromEngine, commandEngine, transientNotificationHost, defaultEditorViewBox, clearZoomHandoffs, syncEngineToolState, syncDocumentFromEngine, renderSecondaryToolbar, markCurrentDocumentSaved, canSaveCurrentDocument, updateCanvasContextMenuAvailability } = scope;

  async function renderSelectionOnlyUpdate(point, syncCursor = syncSelectCursorForPoint, options = {}) {
    const renderList = options.useInteractionList === false
      ? currentEditorRenderList()
      : syncEditorSelectionRenderListFromEngine();
    renderEditorOverlay(renderList);
    if (point && typeof syncCursor === "function") {
      await syncCursor(point);
    }
    if (options.deferEngineReads) {
      return;
    }
    syncSelectionChemistrySummary();
    refreshCommandAvailability();
  }

  async function selectClickTarget(point, additive = false) {
    if (!additive && !currentDocumentBoundsContainsPoint(point, documentBoundsHitPadScreenPx)) {
      await state.editorEngine.clearSelection?.();
      invalidateEditorEngineReadCache();
      return;
    }
    await state.editorEngine.selectAtPoint(point.x, point.y, additive);
  }

  function formatSelectionSummaryMass(value, digits) {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric.toFixed(digits) : "";
  }

  function clampMassDigits(value) {
    const numeric = Math.trunc(Number(value));
    if (!Number.isFinite(numeric)) {
      return 2;
    }
    return Math.max(0, Math.min(8, numeric));
  }

  function setMassDigits(value) {
    const next = clampMassDigits(value);
    if (editorState.massDigits === next) {
      return;
    }
    editorState.massDigits = next;
    syncSelectionChemistrySummary();
  }

  function massPrecisionIcon(direction) {
    const path = direction === "up"
      ? "M6 9 12 3l6 6"
      : "M6 5l6 6 6-6";
    return `<svg viewBox="0 0 24 14" aria-hidden="true"><path d="${path}"/></svg>`;
  }

  function makeMassPrecisionControl() {
    const control = document.createElement("span");
    control.className = "selection-mass-precision";
    const value = document.createElement("span");
    value.className = "selection-mass-precision-value";
    value.textContent = String(clampMassDigits(editorState.massDigits));
    const buttons = document.createElement("span");
    buttons.className = "selection-mass-precision-buttons";
    const increase = document.createElement("button");
    increase.className = "selection-mass-precision-button";
    increase.type = "button";
    increase.title = "Increase mass decimals";
    increase.setAttribute("aria-label", "Increase mass decimals");
    increase.innerHTML = massPrecisionIcon("up");
    increase.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      setMassDigits(editorState.massDigits + 1);
    });
    const decrease = document.createElement("button");
    decrease.className = "selection-mass-precision-button";
    decrease.type = "button";
    decrease.title = "Decrease mass decimals";
    decrease.setAttribute("aria-label", "Decrease mass decimals");
    decrease.innerHTML = massPrecisionIcon("down");
    decrease.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      setMassDigits(editorState.massDigits - 1);
    });
    buttons.append(increase, decrease);
    control.append(value, buttons);
    return control;
  }

  function makeSelectionSummaryItem(label, value, accessory = null) {
    const item = document.createElement("span");
    item.className = "selection-chemistry-summary-item";
    const labelNode = document.createElement("span");
    labelNode.className = "selection-chemistry-summary-label";
    labelNode.textContent = label;
    const valueNode = document.createElement("span");
    valueNode.className = "selection-chemistry-summary-value";
    valueNode.textContent = value;
    item.append(labelNode, valueNode);
    if (accessory) {
      item.append(accessory);
    }
    return item;
  }

  function appendFormulaText(container, formula) {
    const parts = String(formula || "").match(/[A-Z][a-z]?|\d+/g) || [];
    for (const part of parts) {
      const node = document.createElement(/^\d+$/.test(part) ? "sub" : "span");
      node.textContent = part;
      container.append(node);
    }
  }

  function syncSelectionChemistrySummary() {
    if (!selectionChemistrySummary) {
      return;
    }
    selectionChemistrySummary.replaceChildren();
    const summary = isEditingRustDocument()
      ? parseEngineJson(state.editorEngine?.selectionChemistrySummaryJson?.(), null)
      : null;
    if (!summary?.formula) {
      return;
    }
    const formula = document.createElement("span");
    formula.className = "selection-chemistry-summary-item selection-chemistry-summary-formula";
    appendFormulaText(formula, summary.formula);
    editorState.massDigits = clampMassDigits(editorState.massDigits);
    const formulaWeight = formatSelectionSummaryMass(summary.formulaWeight, editorState.massDigits);
    const exactMass = formatSelectionSummaryMass(summary.exactMass, editorState.massDigits);
    selectionChemistrySummary.append(formula);
    selectionChemistrySummary.append(makeMassPrecisionControl());
    if (formulaWeight) {
      selectionChemistrySummary.append(makeSelectionSummaryItem("Formula Weight", formulaWeight));
    }
    if (exactMass) {
      selectionChemistrySummary.append(makeSelectionSummaryItem("Exact Mass", exactMass));
    }
    const paste = document.createElement("button");
    paste.type = "button";
    paste.className = "selection-analysis-paste";
    paste.textContent = "Paste";
    paste.title = "Paste analysis as linked text";
    paste.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      void pasteSelectionAnalysisCaption();
    });
    selectionChemistrySummary.append(paste);
  }

  async function pasteSelectionAnalysisCaption() {
    try {
      const result = await commandEngine.executeEngineCommand(
        {
          type: "paste-analysis-caption",
          payload: { digits: clampMassDigits(editorState.massDigits) },
        },
        () => state.editorEngine?.pasteSelectionAnalysisCaption?.(
          clampMassDigits(editorState.massDigits),
        ),
      );
      renderDocumentPrimitiveChange(result);
      invalidateEditorEngineReadCache();
      syncSelectionChemistrySummary();
      refreshCommandAvailability();
    } catch (error) {
      console.error("[chemsema] failed to paste linked analysis text", error);
      transientNotificationHost?.show?.(
        `Could not paste analysis text: ${error instanceof Error ? error.message : String(error)}`,
        { error: true, duration: 3600 },
      );
    }
  }

  async function resetEditorEngine() {
    await finishActiveTextEditor(false);
    await state.editorEngine?.free?.();
    state.editorEngine = engineHost.createEngineSession();
    await state.editorEngine.ready?.();
    syncTextSymbolPaletteFromEngine();
    commandEngine.resetRevision();
    state.runtimeViewBox = defaultEditorViewBox();
    state.lastEditFocusPoint = null;
    clearZoomHandoffs();
    state.currentFileName = null;
    state.currentFilePath = null;
    state.savedDocumentJson = null;
    state.savedRevision = null;
    await syncEngineToolState();
    await syncDocumentFromEngine();
    renderSecondaryToolbar();
    markCurrentDocumentSaved();
  }

  async function resetDocumentEngine() {
    await state.documentEngine?.free?.();
    state.documentEngine = engineHost.createEngineSession();
    await state.documentEngine.ready?.();
  }

  function refreshCommandAvailability() {
    const undoButton = document.querySelector('[data-command="undo"]');
    const redoButton = document.querySelector('[data-command="redo"]');
    const saveButtons = document.querySelectorAll('[data-command="save"]');
    if (undoButton) {
      undoButton.disabled = !state.editorEngine?.canUndo?.();
    }
    if (redoButton) {
      redoButton.disabled = !state.editorEngine?.canRedo?.();
    }
    for (const saveButton of saveButtons) {
      saveButton.disabled = !canSaveCurrentDocument();
    }
    void updateCanvasContextMenuAvailability();
  }

  function uniformValue(values) {
    const normalized = values.filter((value) => value != null && value !== "");
    if (!normalized.length) {
      return null;
    }
    return normalized.every((value) => value === normalized[0]) ? normalized[0] : null;
  }

  return { renderSelectionOnlyUpdate, selectClickTarget, formatSelectionSummaryMass, clampMassDigits, setMassDigits, massPrecisionIcon, makeMassPrecisionControl, makeSelectionSummaryItem, appendFormulaText, syncSelectionChemistrySummary, resetEditorEngine, resetDocumentEngine, refreshCommandAvailability, uniformValue };
}
