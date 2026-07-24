export function createTextEditCommitHost(scope) {
  const { getActiveTextEditor, setActiveTextEditor, textEditorLayer, commandResultForTextEditorTarget, window, renderDocumentChange, renderEditorOverlay, currentEditorRenderList, editorSessionToEngineSession, commandEngine, state, editorRootBaseStyle, editorState, defaultTextEditorLineHeight, runsPlainText, editorRootFontFamily, normalizeEditorSourceRunsModel, applyTextInlineStyle, isEditingRustDocument, showKernelNoticeDialog } = scope;

  async function finishActiveTextEditor(commit = true) {
    if (!getActiveTextEditor()) {
      return false;
    }
    const { root, session, input } = getActiveTextEditor();
    const activeTextTargetResult = commandResultForTextEditorTarget(session?.target);
    input?.blur?.();
    const selection = window.getSelection?.();
    selection?.removeAllRanges?.();
    const nextSession = buildCommittedTextSession(session, root);
    const bracketLabelObjectId = getActiveTextEditor()?.bracketLabelObjectId || null;
    textEditorLayer.replaceChildren();
    setActiveTextEditor(null);
    if (!commit) {
      if (activeTextTargetResult) {
        renderDocumentChange(activeTextTargetResult);
      } else {
        renderEditorOverlay(currentEditorRenderList());
      }
      return false;
    }
    const engineSessionJson = JSON.stringify(editorSessionToEngineSession(nextSession));
    const result = await commandEngine.executeEngineCommand(
      {
        type: bracketLabelObjectId ? "apply-bracket-label-text" : "apply-text-edit",
        payload: {
          target: nextSession.target || null,
          bracketObjectId: bracketLabelObjectId,
        },
      },
      () => bracketLabelObjectId
        ? state.editorEngine?.applyBracketLabelText?.(bracketLabelObjectId, engineSessionJson)
        : state.editorEngine?.applyTextEdit?.(engineSessionJson),
    );
    renderDocumentChange(result);
    const pendingDialogJson = await state.editorEngine?.takePendingDialogJson?.();
    if (pendingDialogJson) {
      try {
        const pendingDialog = JSON.parse(pendingDialogJson);
        if (pendingDialog?.message) {
          await showKernelNoticeDialog?.(pendingDialog);
        }
      } catch {
        // The engine returns either a valid dialog specification or JSON null.
      }
    }
    return Boolean(result.changed);
  }

  function buildCommittedTextSession(session, root) {
    const sourceRuns = normalizeEditorSourceRuns(
      getActiveTextEditor()?.sourceRuns || [],
      editorRootBaseStyle(root),
    );
    const anchorOffset = getActiveTextEditor()?.layout?.anchorOffset || { x: 0, y: 0 };
    const baseFontSize = Number.parseFloat(root.dataset.baseFontSize || `${editorState.textFontSize}`)
      || editorState.textFontSize;
    const baseLineHeight = Number.parseFloat(root.dataset.baseLineHeight || `${defaultTextEditorLineHeight(baseFontSize)}`)
      || defaultTextEditorLineHeight(baseFontSize);
    return {
      ...session,
      text: runsPlainText(sourceRuns),
      sourceRuns,
      fontFamily: editorRootFontFamily(root),
      fontSize: baseFontSize,
      fill: cssColorToHex(root.style.color || editorState.textColor),
      align: root.style.textAlign || editorState.textAlign,
      lineHeight: baseLineHeight,
      anchorOffset: session.target?.kind === "endpoint-label"
        ? [anchorOffset.x, anchorOffset.y]
        : undefined,
      defaultChemical: root.dataset.defaultChemical === "true",
    };
  }

  function normalizeEditorSourceRuns(runs, defaultStyle) {
    return normalizeEditorSourceRunsModel(runs, defaultStyle, cssColorToHex);
  }

  function cssColorToHex(color) {
    if (!color) {
      return "#000000";
    }
    if (/^#[0-9a-fA-F]{6}$/.test(color)) {
      return color.toLowerCase();
    }
    if (/^#[0-9a-fA-F]{3}$/.test(color)) {
      return `#${color[1]}${color[1]}${color[2]}${color[2]}${color[3]}${color[3]}`.toLowerCase();
    }
    const match = color.match(/\d+/g);
    if (!match || match.length < 3) {
      return color;
    }
    return `#${match.slice(0, 3).map((value) => Number(value).toString(16).padStart(2, "0")).join("")}`;
  }

  async function applySelectionColor(color) {
    const normalized = cssColorToHex(color);
    editorState.selectionColor = normalized;
    if (getActiveTextEditor()) {
      applyTextInlineStyle({ color: normalized });
      return true;
    }
    if (!isEditingRustDocument() || !state.editorEngine?.applyColorToSelection) {
      return false;
    }
    const result = await commandEngine.executeEngineCommand(
      {
        type: "apply-selection-color",
        payload: { color: normalized },
      },
      () => state.editorEngine.applyColorToSelection(normalized),
    );
    const changed = !!result.changed;
    if (!changed) {
      return false;
    }
    renderDocumentChange(result);
    return true;
  }

  return { finishActiveTextEditor, buildCommittedTextSession, normalizeEditorSourceRuns, cssColorToHex, applySelectionColor };
}
