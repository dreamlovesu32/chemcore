export function createEditorCommandController(options) {
  async function writeNativeClipboardFromSelection(fragmentJson = null, documentJson = undefined) {
    const state = options.state();
    if (!options.desktopFileHost?.available || !state.editorEngine) {
      return false;
    }
    try {
      const resolvedFragmentJson = fragmentJson || await state.editorEngine.clipboardSelectionJson?.() || null;
      const resolvedDocumentJson = documentJson === undefined
        ? await state.editorEngine.clipboardDocumentJson?.() || null
        : documentJson;
      if (!resolvedFragmentJson && !resolvedDocumentJson) {
        return false;
      }
      const cdxml = await state.editorEngine.documentCdxml?.() || null;
      const svg = null;
      await options.desktopFileHost.writeClipboard({
        chemcoreFragmentJson: resolvedFragmentJson,
        chemcoreDocumentJson: resolvedDocumentJson,
        renderListJson: state.editorEngine.renderListJson?.() || null,
        cdxml,
        svg,
        text: cdxml,
      });
      return true;
    } catch (error) {
      console.warn("Failed to write native clipboard", error);
    }
    return false;
  }

  async function pasteFromNativeClipboard() {
    const state = options.state();
    if (!options.desktopFileHost?.available || !state.editorEngine?.pasteClipboardJson) {
      return false;
    }
    try {
      const payload = await options.desktopFileHost.readClipboard();
      if (payload?.chemcoreFragmentJson) {
        return !!(await state.editorEngine.pasteClipboardJson(payload.chemcoreFragmentJson));
      }
    } catch (error) {
      console.warn("Failed to read native clipboard", error);
    }
    return false;
  }

  async function runEditorCommand(command) {
    const state = options.state();
    if (!options.isEditingRustDocument()) {
      return false;
    }
    let changed = false;
    let shouldRenderDocument = false;
    if (command === "undo") {
      changed = await state.editorEngine.undo();
    } else if (command === "redo") {
      changed = await state.editorEngine.redo();
    } else if (command === "copy") {
      const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
      const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
      changed = !!(await state.editorEngine.copySelection?.());
      changed = await writeNativeClipboardFromSelection(fragmentJson, documentJson) || changed;
    } else if (command === "cut") {
      const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
      const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
      changed = !!(await state.editorEngine.cutSelection?.());
      if (changed) {
        await writeNativeClipboardFromSelection(fragmentJson, documentJson);
      }
    } else if (command === "paste") {
      changed = await pasteFromNativeClipboard();
      if (!changed) {
        changed = !!(await state.editorEngine.pasteClipboard?.());
      }
    } else if (command === "delete") {
      changed = await state.editorEngine.deleteSelection();
    } else if (command === "select-all") {
      await options.activateEditorTool("select");
      changed = !!(await state.editorEngine.selectAll?.());
      shouldRenderDocument = true;
    } else {
      return false;
    }
    if (changed || shouldRenderDocument) {
      await options.syncDocumentFromEngine();
      options.renderDocument();
    } else {
      options.renderEditorOverlay();
      options.refreshCommandAvailability();
    }
    return true;
  }

  return {
    writeNativeClipboardFromSelection,
    pasteFromNativeClipboard,
    runEditorCommand,
  };
}
