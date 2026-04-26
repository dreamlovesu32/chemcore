import { fillTextEditorContent as renderTextEditorContent } from "./text_editor_render.js";

export function createTextEditorController(deps) {
  function renderEditorContent(root, session, selectionOffsets = null) {
    renderTextEditorContent(root, session, selectionOffsets, {
      resolveDisplayRuns: (nextSession) => deps.displayRunsForEditor(
        Array.isArray(nextSession.sourceRuns) && nextSession.sourceRuns.length
          ? nextSession.sourceRuns
          : nextSession.text
            ? [{ text: String(nextSession.text || ""), script: nextSession.defaultChemical ? "chemical" : "normal" }]
            : [],
        deps.getActiveEditor()?.root || root,
        { target: nextSession.target },
      ),
      renderRunNode: deps.renderRunNode,
    });
  }

  function getEditor() {
    return deps.getActiveEditor();
  }

  function setEditor(editor) {
    deps.setActiveEditor(editor);
    return editor;
  }

  function currentEditorSelectionOffsets() {
    const editor = getEditor();
    if (!editor) {
      return null;
    }
    return editor.selection
      ? deps.normalizeSelection(editor.plainText, editor.selection)
      : null;
  }

  function setActiveEditorSelection(selectionOffsets, syncDom = true) {
    const editor = getEditor();
    if (!editor) {
      return null;
    }
    const normalized = deps.normalizeSelection(editor.plainText, selectionOffsets);
    editor.selection = normalized;
    if (syncDom) {
      deps.updateCustomEditorChrome();
    }
    return normalized;
  }

  function focusActiveTextEditor() {
    getEditor()?.input?.focus?.();
  }

  function setPendingEditorStyle(style, caretOffset) {
    const editor = getEditor();
    if (!editor) {
      return;
    }
    editor.pendingStyle = { ...style };
    editor.pendingStyleCaretOffset = caretOffset;
    editor.preferredCaretX = null;
    editor.root.dataset.defaultChemical = style.script === "chemical" ? "true" : "false";
    editor.root.style.fontFamily = style.fontFamily || editor.root.style.fontFamily;
    editor.root.style.fontSize = `${Number(style.fontSize || deps.editorState.textFontSize)}px`;
    editor.root.style.color = style.fill || editor.root.style.color;
  }

  function styleAtEditorOffset(offset) {
    const editor = getEditor();
    if (!editor?.root) {
      return null;
    }
    return deps.styleAtOffset(
      offset,
      editor.sourceRuns || [],
      deps.editorRootBaseStyle(editor.root),
      deps.cssColorToHex,
    );
  }

  function currentInsertionStyle(selectionOffsets) {
    const editor = getEditor();
    const offset = selectionOffsets?.start || 0;
    if (editor?.pendingStyle && editor.pendingStyleCaretOffset === offset) {
      return { ...editor.pendingStyle };
    }
    return styleAtEditorOffset(offset);
  }

  function syncPendingEditorStyleWithSelection() {
    const editor = getEditor();
    if (!editor) {
      return;
    }
    const selectionOffsets = currentEditorSelectionOffsets();
    if (!selectionOffsets || !selectionOffsets.collapsed || selectionOffsets.start !== editor.pendingStyleCaretOffset) {
      editor.pendingStyle = null;
      editor.pendingStyleCaretOffset = null;
    }
  }

  function renderActiveTextEditorFromModel(selectionOffsets = null) {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    const requestedSelection = selectionOffsets || editor.selection || {
      anchor: deps.textLength(editor.plainText),
      focus: deps.textLength(editor.plainText),
    };
    editor.isNormalizingChemical = true;
    const baseStyle = deps.editorRootBaseStyle(editor.root);
    const preview = deps.previewTextRunsFromKernel(editor.sourceRuns, editor.root, {
      target: editor.session?.target,
    });
    editor.sourceRuns = preview?.sourceRuns
      || deps.normalizeRuns(editor.sourceRuns, baseStyle);
    editor.plainText = deps.runsPlainText(editor.sourceRuns);
    const nextSelection = deps.normalizeSelection(editor.plainText, requestedSelection);
    renderEditorContent(editor.display || editor.root, {
      sourceRuns: editor.sourceRuns,
      text: editor.plainText,
      defaultChemical: editor.root.dataset.defaultChemical === "true",
      target: editor.session?.target,
    }, nextSelection);
    editor.layoutCache = null;
    setActiveEditorSelection(nextSelection, false);
    editor.isNormalizingChemical = false;
    deps.syncTextEditorSize();
  }

  function replaceEditorSelectionWithRuns(insertedRuns, selectionOffsets = currentEditorSelectionOffsets(), options = {}) {
    const editor = getEditor();
    if (!editor?.root || !selectionOffsets) {
      return;
    }
    const baseStyle = deps.editorRootBaseStyle(editor.root);
    const runs = deps.normalizeRuns(editor.sourceRuns || [], baseStyle);
    const { before, after } = deps.splitRunsForSelection(runs, selectionOffsets.start, selectionOffsets.end);
    const nextRuns = deps.normalizeRuns([...before, ...insertedRuns, ...after], baseStyle);
    const insertedLength = deps.textLength(deps.runsPlainText(insertedRuns));
    editor.sourceRuns = nextRuns;
    editor.hasUserEdited = true;
    const caret = selectionOffsets.start + insertedLength;
    const nextSelection = options.keepInsertedSelected
      ? { anchor: selectionOffsets.start, focus: caret }
      : { anchor: caret, focus: caret };
    renderActiveTextEditorFromModel(nextSelection);
    if (insertedRuns.length && options.updatePendingStyle !== false && !options.keepInsertedSelected) {
      setPendingEditorStyle(insertedRuns[insertedRuns.length - 1], caret);
    }
  }

  function replaceEditorSelectionWithText(text, style = null, selectionOffsets = currentEditorSelectionOffsets(), options = {}) {
    if (!selectionOffsets || text == null) {
      return;
    }
    const insertionStyle = style || currentInsertionStyle(selectionOffsets);
    replaceEditorSelectionWithRuns([{
      ...insertionStyle,
      text,
    }], selectionOffsets, options);
  }

  function applyCompositionText(text, selectionOffsets = currentEditorSelectionOffsets()) {
    const editor = getEditor();
    if (!editor?.root || !selectionOffsets) {
      return;
    }
    const composition = editor.composition || {};
    const targetSelection = composition.selection || selectionOffsets;
    const style = composition.style || currentInsertionStyle(selectionOffsets);
    replaceEditorSelectionWithText(text, style, targetSelection, {
      keepInsertedSelected: true,
      updatePendingStyle: false,
    });
    editor.composition = {
      active: true,
      style,
      selection: deps.normalizeSelection(editor.plainText, {
        anchor: targetSelection.start,
        focus: targetSelection.start + deps.textLength(text),
      }),
    };
  }

  function deleteEditorRange(selectionOffsets, direction = "backward") {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    let target = selectionOffsets;
    if (!target) {
      return;
    }
    if (target.collapsed) {
      const plainText = editor.plainText || deps.runsPlainText(editor.sourceRuns || []);
      if (direction === "backward") {
        if (target.start <= 0) {
          return;
        }
        target = {
          anchor: target.start - 1,
          focus: target.start,
          start: target.start - 1,
          end: target.start,
          collapsed: false,
        };
      } else {
        if (target.start >= deps.textLength(plainText)) {
          return;
        }
        target = {
          anchor: target.start,
          focus: target.start + 1,
          start: target.start,
          end: target.start + 1,
          collapsed: false,
        };
      }
    }
    replaceEditorSelectionWithRuns([], target);
  }

  function mutateEditorSelectionRuns(mutateRun, mutateCollapsedStyle = null) {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    const selectionOffsets = currentEditorSelectionOffsets();
    if (!selectionOffsets) {
      return;
    }
    if (selectionOffsets.collapsed) {
      const currentStyle = currentInsertionStyle(selectionOffsets);
      const nextStyle = mutateCollapsedStyle
        ? mutateCollapsedStyle({ ...currentStyle })
        : mutateRun({ ...currentStyle });
      setPendingEditorStyle(nextStyle, selectionOffsets.start);
      return;
    }
    const baseStyle = deps.editorRootBaseStyle(editor.root);
    const runs = deps.normalizeRuns(editor.sourceRuns || [], baseStyle);
    const { before, selected, after } = deps.splitRunsForSelection(runs, selectionOffsets.start, selectionOffsets.end);
    const nextSelected = selected.map((run) => mutateRun({ ...run }));
    editor.sourceRuns = deps.normalizeRuns([...before, ...nextSelected, ...after], baseStyle);
    editor.hasUserEdited = true;
    renderActiveTextEditorFromModel(selectionOffsets);
  }

  function handleTextEditorBeforeInput(event, root) {
    const editor = getEditor();
    if (!editor || editor.root !== root) {
      return;
    }
    const selectionOffsets = currentEditorSelectionOffsets();
    if (!selectionOffsets) {
      return;
    }
    const inputType = event.inputType || "";
    if (inputType === "insertCompositionText") {
      event.preventDefault();
      applyCompositionText(event.data || "", selectionOffsets);
      return;
    }
    if (event.isComposing) {
      return;
    }
    if (inputType === "insertText") {
      event.preventDefault();
      replaceEditorSelectionWithText(event.data || "", currentInsertionStyle(selectionOffsets), selectionOffsets);
      return;
    }
    if (inputType === "insertParagraph" || inputType === "insertLineBreak") {
      event.preventDefault();
      replaceEditorSelectionWithText("\n", currentInsertionStyle(selectionOffsets), selectionOffsets);
      return;
    }
    if (inputType === "deleteContentBackward") {
      event.preventDefault();
      deleteEditorRange(selectionOffsets, "backward");
      return;
    }
    if (inputType === "deleteContentForward") {
      event.preventDefault();
      deleteEditorRange(selectionOffsets, "forward");
    }
  }

  function applyTextFormatCommand(command) {
    const desiredWeight = deps.editorState.textBold ? 700 : 400;
    const desiredStyle = deps.editorState.textItalic ? "italic" : "normal";
    const desiredUnderline = Boolean(deps.editorState.textUnderline);
    if (command === "bold") {
      mutateEditorSelectionRuns((run) => ({ ...run, fontWeight: desiredWeight }), (style) => ({ ...style, fontWeight: desiredWeight }));
    } else if (command === "italic") {
      mutateEditorSelectionRuns((run) => ({ ...run, fontStyle: desiredStyle }), (style) => ({ ...style, fontStyle: desiredStyle }));
    } else if (command === "underline") {
      mutateEditorSelectionRuns((run) => ({ ...run, underline: desiredUnderline }), (style) => ({ ...style, underline: desiredUnderline }));
    }
  }

  function applyTextScript(script) {
    const nextScript = script === "subscript" ? "subscript" : "superscript";
    mutateEditorSelectionRuns((run) => ({ ...run, script: nextScript }), (style) => ({ ...style, script: nextScript }));
  }

  function applyTextInlineStyle(styles) {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    mutateEditorSelectionRuns((run) => ({
      ...run,
      fontFamily: styles.fontFamily || run.fontFamily,
      fontSize: styles.fontSize ? Number.parseFloat(styles.fontSize) : run.fontSize,
      fill: styles.color ? deps.cssColorToHex(styles.color) : run.fill,
    }), (style) => ({
      ...style,
      fontFamily: styles.fontFamily || style.fontFamily,
      fontSize: styles.fontSize ? Number.parseFloat(styles.fontSize) : style.fontSize,
      fill: styles.color ? deps.cssColorToHex(styles.color) : style.fill,
    }));
  }

  function applyChemicalFormat() {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    mutateEditorSelectionRuns((run) => ({ ...run, script: "chemical" }), (style) => ({ ...style, script: "chemical" }));
  }

  function insertTextAtSelection(text) {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    const selectionOffsets = currentEditorSelectionOffsets();
    if (!selectionOffsets) {
      return;
    }
    replaceEditorSelectionWithText(text, currentInsertionStyle(selectionOffsets), selectionOffsets);
  }

  function moveEditorSelectionHorizontally(direction, extend) {
    const editor = getEditor();
    const selection = currentEditorSelectionOffsets();
    if (!selection || !editor) {
      return;
    }
    if (!extend && !selection.collapsed) {
      const collapseTo = direction < 0 ? selection.start : selection.end;
      editor.preferredCaretX = null;
      setActiveEditorSelection({ anchor: collapseTo, focus: collapseTo }, false);
      renderActiveTextEditorFromModel();
      return;
    }
    const delta = direction < 0 ? -1 : 1;
    const targetFocus = Math.max(0, Math.min(deps.textLength(editor.plainText), selection.focus + delta));
    const next = extend
      ? { anchor: selection.anchor, focus: targetFocus }
      : { anchor: targetFocus, focus: targetFocus };
    editor.preferredCaretX = null;
    setActiveEditorSelection(next, false);
    renderActiveTextEditorFromModel();
  }

  function moveEditorSelectionToBoundary(boundary, extend) {
    const editor = getEditor();
    const selection = currentEditorSelectionOffsets();
    if (!selection || !editor) {
      return;
    }
    const lineIndex = deps.editorLineIndexForOffset(selection.focus);
    const layout = deps.buildEditorCaretLayout();
    const line = layout?.lines?.[Math.max(0, lineIndex)];
    const target = boundary === "start"
      ? (line?.caretOffsets?.[0]?.offset ?? 0)
      : (line?.caretOffsets?.[line.caretOffsets.length - 1]?.offset ?? deps.textLength(editor.plainText));
    const next = extend
      ? { anchor: selection.anchor, focus: target }
      : { anchor: target, focus: target };
    editor.preferredCaretX = null;
    setActiveEditorSelection(next, false);
    renderActiveTextEditorFromModel();
  }

  function moveEditorSelectionVertically(direction, extend) {
    const editor = getEditor();
    const selection = currentEditorSelectionOffsets();
    const layout = deps.buildEditorCaretLayout();
    if (!selection || !layout || !editor) {
      return;
    }
    const currentLineIndex = deps.editorLineIndexForOffset(selection.focus);
    const nextLineIndex = Math.max(0, Math.min(layout.lines.length - 1, currentLineIndex + direction));
    if (nextLineIndex === currentLineIndex) {
      return;
    }
    const currentRect = deps.measureEditorCaretRect(selection.focus);
    const preferredX = editor.preferredCaretX ?? currentRect?.x ?? 0;
    editor.preferredCaretX = preferredX;
    const nextFocus = deps.nearestOffsetOnLine(layout.lines[nextLineIndex], preferredX);
    const next = extend
      ? { anchor: selection.anchor, focus: nextFocus }
      : { anchor: nextFocus, focus: nextFocus };
    setActiveEditorSelection(next, false);
    renderActiveTextEditorFromModel();
  }

  function handleTextEditorKeyDown(event) {
    const editor = getEditor();
    if (!editor?.root) {
      return;
    }
    if (event.isComposing) {
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "a") {
      event.preventDefault();
      setActiveEditorSelection({ anchor: 0, focus: deps.textLength(editor.plainText) }, false);
      renderActiveTextEditorFromModel();
      return;
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      moveEditorSelectionHorizontally(-1, event.shiftKey);
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      moveEditorSelectionHorizontally(1, event.shiftKey);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveEditorSelectionVertically(-1, event.shiftKey);
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveEditorSelectionVertically(1, event.shiftKey);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      moveEditorSelectionToBoundary("start", event.shiftKey);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      moveEditorSelectionToBoundary("end", event.shiftKey);
    }
  }

  function handleTextEditorPointerDown(event) {
    const editor = getEditor();
    if (!editor?.root || event.button !== 0 || event.currentTarget !== editor.root) {
      return;
    }
    event.preventDefault();
    const offset = deps.editorOffsetFromPointerEvent(event);
    const anchor = event.shiftKey && editor.selection ? editor.selection.anchor : offset;
    editor.dragSelecting = true;
    editor.preferredCaretX = null;
    editor.root.setPointerCapture?.(event.pointerId);
    setActiveEditorSelection({ anchor, focus: offset }, false);
    renderActiveTextEditorFromModel();
    focusActiveTextEditor();
  }

  function handleTextEditorPointerMove(event) {
    const editor = getEditor();
    if (!editor?.dragSelecting) {
      return;
    }
    event.preventDefault();
    const offset = deps.editorOffsetFromPointerEvent(event);
    const anchor = editor.selection?.anchor ?? offset;
    setActiveEditorSelection({ anchor, focus: offset }, false);
    renderActiveTextEditorFromModel();
  }

  function handleTextEditorPointerUp(event) {
    const editor = getEditor();
    if (!editor?.dragSelecting) {
      return;
    }
    event.preventDefault();
    editor.dragSelecting = false;
    editor.root.releasePointerCapture?.(event.pointerId);
    syncPendingEditorStyleWithSelection();
  }

  function openTextEditorSession(session) {
    if (!deps.textEditorLayer) {
      return;
    }
    const shouldSelectAll = Boolean(
      deps.textLength(session.text || "")
        || (Array.isArray(session.sourceRuns) && session.sourceRuns.some((run) => deps.textLength(run?.text || ""))),
    );
    const root = document.createElement("div");
    root.className = "text-editor";
    root.dataset.defaultChemical = session.defaultChemical ? "true" : "false";
    root.dataset.baseFontSize = String(Number(session.fontSize || deps.editorState.textFontSize));
    root.dataset.baseLineHeight = String(Number(session.lineHeight || deps.defaultTextEditorLineHeight(session.fontSize || deps.editorState.textFontSize)));
    root.style.fontFamily = session.fontFamily || deps.editorState.textFontFamily;
    const fontSize = Number(session.fontSize || deps.editorState.textFontSize);
    const lineHeight = Number(session.lineHeight || deps.defaultTextEditorLineHeight(fontSize));
    root.style.fontSize = `${fontSize}px`;
    root.style.lineHeight = `${lineHeight}px`;
    root.style.color = session.fill || deps.editorState.textColor;
    root.style.textAlign = session.align || "left";
    root.style.minWidth = "8px";
    root.style.minHeight = `${lineHeight}px`;
    const display = document.createElement("div");
    display.className = "text-editor-display";
    const caret = document.createElement("div");
    caret.className = "text-editor-caret";
    const input = document.createElement("textarea");
    input.className = "text-editor-input";
    input.spellcheck = false;
    input.setAttribute("aria-label", "text editor input");
    root.appendChild(display);
    root.appendChild(caret);
    root.appendChild(input);
    renderEditorContent(display, session);
    deps.textEditorLayer.replaceChildren(root);
    const sourceRuns = deps.editorSourceRunsFromSession(session, root);
    const editor = setEditor({
      root,
      display,
      caret,
      input,
      session,
      sourceRuns,
      plainText: deps.runsPlainText(sourceRuns),
      selection: null,
      pendingStyle: null,
      pendingStyleCaretOffset: null,
      hasUserEdited: false,
      isNormalizingChemical: false,
      dragSelecting: false,
      preferredCaretX: null,
      layoutCache: null,
      composition: null,
    });
    deps.syncTextToolbarStateFromSession(session);
    deps.positionActiveTextEditor();
    if (shouldSelectAll) {
      setActiveEditorSelection({ anchor: 0, focus: deps.textLength(editor.plainText) }, false);
    } else {
      const offset = deps.textLength(editor.plainText);
      setActiveEditorSelection({ anchor: offset, focus: offset }, false);
    }
    renderActiveTextEditorFromModel();
    input.focus();
    input.addEventListener("beforeinput", (event) => {
      handleTextEditorBeforeInput(event, root);
    });
    input.addEventListener("input", () => {
      const current = getEditor();
      if (current?.root === root) {
        current.hasUserEdited = true;
      }
      if (!getEditor()?.composition?.active) {
        input.value = "";
      }
    });
    input.addEventListener("compositionstart", () => {
      const current = getEditor();
      if (current?.root !== root) {
        return;
      }
      const selectionOffsets = currentEditorSelectionOffsets();
      current.composition = {
        active: true,
        style: selectionOffsets ? currentInsertionStyle(selectionOffsets) : deps.editorRootBaseStyle(root),
        selection: selectionOffsets,
      };
    });
    input.addEventListener("compositionend", () => {
      const current = getEditor();
      if (current?.root !== root) {
        return;
      }
      const composition = current.composition;
      current.composition = null;
      input.value = "";
      const focus = composition?.selection?.end ?? currentEditorSelectionOffsets()?.end;
      if (Number.isFinite(focus)) {
        setActiveEditorSelection({ anchor: focus, focus }, false);
        renderActiveTextEditorFromModel();
        if (composition?.style) {
          setPendingEditorStyle(composition.style, focus);
        }
      }
    });
    input.addEventListener("keydown", (event) => {
      handleTextEditorKeyDown(event);
    });
    root.addEventListener("pointerdown", (event) => {
      handleTextEditorPointerDown(event);
    });
    root.addEventListener("pointermove", (event) => {
      handleTextEditorPointerMove(event);
    });
    root.addEventListener("pointerup", (event) => {
      handleTextEditorPointerUp(event);
    });
    root.addEventListener("keyup", () => {
      syncPendingEditorStyleWithSelection();
      deps.syncTextEditorSize();
    });
    root.addEventListener("mouseup", () => {
      syncPendingEditorStyleWithSelection();
    });
    root.addEventListener("paste", (event) => {
      event.preventDefault();
      const text = event.clipboardData?.getData("text/plain") || "";
      insertTextAtSelection(text);
    });
  }

  return {
    focusActiveTextEditor,
    openTextEditorSession,
    currentEditorSelectionOffsets,
    setActiveEditorSelection,
    renderActiveTextEditorFromModel,
    syncPendingEditorStyleWithSelection,
    handleTextEditorBeforeInput,
    handleTextEditorKeyDown,
    handleTextEditorPointerDown,
    handleTextEditorPointerMove,
    handleTextEditorPointerUp,
    applyTextFormatCommand,
    applyTextScript,
    applyTextInlineStyle,
    applyChemicalFormat,
    insertTextAtSelection,
    setPendingEditorStyle,
    currentInsertionStyle,
  };
}
