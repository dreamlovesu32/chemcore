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
        renderListJson: resolvedDocumentJson ? null : state.editorEngine.renderListJson?.() || null,
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
        return !!(await executeDocumentCommand(
          {
            type: "paste-clipboard",
            payload: { source: "native" },
          },
          () => state.editorEngine.pasteClipboardJson(payload.chemcoreFragmentJson),
        ));
      }
    } catch (error) {
      console.warn("Failed to read native clipboard", error);
    }
    return false;
  }

  async function executeDocumentCommand(command, apply, executeOptions = {}) {
    if (options.commandEngine?.executeEngineCommand) {
      const result = await options.commandEngine.executeEngineCommand(command, apply, executeOptions);
      return !!result.changed;
    }
    return !!(await apply());
  }

  async function runEditorCommand(command) {
    const state = options.state();
    if (!options.isEditingRustDocument()) {
      return false;
    }
    let changed = false;
    let shouldRenderDocument = false;
    if (command === "undo") {
      changed = await executeDocumentCommand("undo", () => state.editorEngine.undo());
    } else if (command === "redo") {
      changed = await executeDocumentCommand("redo", () => state.editorEngine.redo());
    } else if (command === "copy") {
      const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
      const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
      changed = !!(await state.editorEngine.copySelection?.());
      changed = await writeNativeClipboardFromSelection(fragmentJson, documentJson) || changed;
    } else if (command === "cut") {
      const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
      const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
      changed = await executeDocumentCommand("cut-selection", () => state.editorEngine.cutSelection?.());
      if (changed) {
        await writeNativeClipboardFromSelection(fragmentJson, documentJson);
      }
    } else if (command === "paste") {
      changed = await pasteFromNativeClipboard();
      if (!changed) {
        changed = await executeDocumentCommand(
          {
            type: "paste-clipboard",
            payload: { source: "internal" },
          },
          () => state.editorEngine.pasteClipboard?.(),
        );
      }
    } else if (command === "delete") {
      changed = await executeDocumentCommand("delete-selection", () => state.editorEngine.deleteSelection());
    } else if (command === "select-all") {
      await options.activateEditorTool("select");
      changed = !!(await state.editorEngine.selectAll?.());
      shouldRenderDocument = true;
    } else if (command === "group-selection") {
      changed = await executeDocumentCommand("group-selection", () => state.editorEngine.groupSelection?.());
    } else if (command === "ungroup-selection") {
      changed = await executeDocumentCommand("ungroup-selection", () => state.editorEngine.ungroupSelection?.());
    } else if (command === "join-selection") {
      changed = await executeDocumentCommand("join-selection", () => state.editorEngine.joinSelection?.());
    } else if (command === "bring-front" || command === "send-back") {
      changed = await executeDocumentCommand(
        { type: "apply-selection-order", payload: { command } },
        () => state.editorEngine.applySelectionOrderCommand?.(command),
      );
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
