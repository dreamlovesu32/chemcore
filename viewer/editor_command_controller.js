let portableClipboardPayload = null;

function portableClipboardHtml(payload) {
  const fragmentBytes = new TextEncoder().encode(payload.chemsemaFragmentJson || "");
  let fragmentBinary = "";
  for (let offset = 0; offset < fragmentBytes.length; offset += 0x8000) {
    fragmentBinary += String.fromCharCode(...fragmentBytes.subarray(offset, offset + 0x8000));
  }
  const fragmentBase64 = btoa(fragmentBinary);
  const payloadBytes = new TextEncoder().encode(JSON.stringify(payload));
  let payloadBinary = "";
  for (let offset = 0; offset < payloadBytes.length; offset += 0x8000) {
    payloadBinary += String.fromCharCode(...payloadBytes.subarray(offset, offset + 0x8000));
  }
  const payloadBase64 = btoa(payloadBinary);
  const encoded = encodeURIComponent(JSON.stringify(payload))
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  return `<div data-chemsema-clipboard-v1="${encoded}" data-chemsema-payload-base64="${payloadBase64}" data-chemsema-clipboard-base64="${fragmentBase64}"></div>`;
}

async function writeBrowserClipboard(payload) {
  portableClipboardPayload = payload;
  if (!navigator.clipboard?.write || typeof ClipboardItem === "undefined") {
    return false;
  }
  const values = {
    "text/plain": new Blob([payload.cdxml || ""], { type: "text/plain" }),
    "text/html": new Blob([portableClipboardHtml(payload)], { type: "text/html" }),
  };
  await navigator.clipboard.write([new ClipboardItem(values)]);
  return true;
}

export function createEditorCommandController(options) {
  let commandOperation = Promise.resolve();

  function reportCommandFailure(command, error) {
    const normalized = error instanceof Error ? error : new Error(String(error));
    console.error(`[chemsema] editor command '${command}' failed`, normalized);
    options.transientNotificationHost?.show?.(
      `Command failed: ${normalized.message}`,
      { error: true, duration: 3600 },
    );
  }

  async function writeClipboardFromSelection(fragmentJson = null, documentJson = undefined) {
    const state = options.state();
    if (!state.editorEngine) {
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
      const cdxml = await state.editorEngine.clipboardCdxml?.() || null;
      const svg = null;
      const payload = {
        chemsemaFragmentJson: resolvedFragmentJson,
        chemsemaDocumentJson: resolvedDocumentJson,
        renderListJson: resolvedDocumentJson ? null : state.editorEngine.renderListJson?.() || null,
        cdxml,
        svg,
        text: cdxml,
      };
      portableClipboardPayload = payload;
      if (options.desktopFileHost?.available) {
        await options.desktopFileHost.writeClipboard(payload);
      } else {
        await writeBrowserClipboard(payload);
      }
      return true;
    } catch (error) {
      console.warn("Failed to write native clipboard", error);
    }
    return false;
  }

  async function pasteStructuredPayload(payload, source = "system") {
    const state = options.state();
    if (!state.editorEngine?.pasteClipboardJson || !payload) {
      return false;
    }
    const command = { type: "paste-clipboard", payload: { source } };
    if (payload.chemsemaFragmentJson) {
      try {
        const result = await executeDocumentCommand(command, () => (
          state.editorEngine.pasteClipboardJson(payload.chemsemaFragmentJson)
        ));
        if (result?.changed ?? result) {
          return result;
        }
      } catch (error) {
        console.warn("ChemSema clipboard fragment was not readable", error);
      }
    }
    if (payload.chemsemaDocumentJson && state.editorEngine.pasteDocumentJson) {
      try {
        const result = await executeDocumentCommand(command, () => (
          state.editorEngine.pasteDocumentJson(payload.chemsemaDocumentJson)
        ));
        if (result?.changed ?? result) {
          return result;
        }
      } catch (error) {
        console.warn("ChemSema clipboard document was not readable", error);
      }
    }
    const cdxml = payload.cdxml
      || (String(payload.text || "").includes("<CDXML") ? payload.text : null);
    if (cdxml && state.editorEngine.pasteCdxml) {
      try {
        const result = await executeDocumentCommand(
          command,
          () => state.editorEngine.pasteCdxml(cdxml),
        );
        if (result?.changed ?? result) {
          return result;
        }
      } catch (error) {
        console.warn("Chemical clipboard interchange was not readable", error);
      }
    }
    if (payload.imageDataBase64 && payload.imageMimeType && options.insertClipboardImage) {
      return options.insertClipboardImage({
        fileName: "Clipboard image",
        mimeType: payload.imageMimeType,
        dataBase64: payload.imageDataBase64,
        pixelWidth: payload.imagePixelWidth,
        pixelHeight: payload.imagePixelHeight,
      });
    }
    return false;
  }

  async function pasteFromSystemClipboard(eventPayload = null) {
    try {
      if (eventPayload) {
        portableClipboardPayload = eventPayload;
        const changed = await pasteStructuredPayload(eventPayload, "browser");
        if (changed?.changed ?? changed) {
          return changed;
        }
      }
      if (options.desktopFileHost?.available) {
        const changed = await pasteStructuredPayload(
          await options.desktopFileHost.readClipboard(),
          "native",
        );
        if (changed?.changed ?? changed) {
          return changed;
        }
      } else if (!eventPayload && navigator.clipboard?.readText) {
        const text = await navigator.clipboard.readText();
        const changed = await pasteStructuredPayload({ text }, "browser-text");
        if (changed?.changed ?? changed) {
          return changed;
        }
      }
      return pasteStructuredPayload(portableClipboardPayload, "portable");
    } catch (error) {
      console.warn("Failed to read system clipboard", error);
      return pasteStructuredPayload(portableClipboardPayload, "portable");
    }
  }

  async function executeDocumentCommand(command, apply, executeOptions = {}) {
    if (options.commandEngine?.executeEngineCommand) {
      const result = await options.commandEngine.executeEngineCommand(command, apply, executeOptions);
      return result;
    }
    const changed = !!(await apply());
    if (changed) {
      await options.syncDocumentFromEngine?.();
    }
    return { changed };
  }

  function runEditorCommand(command, commandPayload = null) {
    const task = commandOperation.then(() => executeEditorCommand(command, commandPayload));
    commandOperation = task.then(
      () => undefined,
      (error) => reportCommandFailure(command, error),
    );
    return task;
  }

  async function executeEditorCommand(command, commandPayload = null) {
    const state = options.state();
    if (!options.isEditingRustDocument()) {
      return false;
    }
    let changed = false;
    const commandChanged = (value) => value?.changed ?? Boolean(value);
    if (command === "undo") {
      changed = await executeDocumentCommand(
        "undo",
        options.commandEngine?.executeEngineCommand ? undefined : () => state.editorEngine.undo(),
      );
    } else if (command === "redo") {
      changed = await executeDocumentCommand(
        "redo",
        options.commandEngine?.executeEngineCommand ? undefined : () => state.editorEngine.redo(),
      );
    } else if (command === "copy") {
      const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
      const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
      changed = !!(await state.editorEngine.copySelection?.());
      changed = await writeClipboardFromSelection(fragmentJson, documentJson) || changed;
      options.renderEditorOverlay();
      options.refreshCommandAvailability();
      return changed;
    } else if (command === "cut") {
      const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
      const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
      changed = await executeDocumentCommand("cut-selection", () => state.editorEngine.cutSelection?.());
      if (commandChanged(changed)) {
        await writeClipboardFromSelection(fragmentJson, documentJson);
      }
    } else if (command === "paste") {
      changed = await pasteFromSystemClipboard(commandPayload?.clipboardPayload || null);
      if (!commandChanged(changed)) {
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
      changed = { changed: !!(await state.editorEngine.selectAll?.()) };
      options.renderEditorOverlay();
      options.refreshCommandAvailability();
      return changed.changed;
    } else if (command === "group-selection") {
      changed = await executeDocumentCommand("group-selection", () => state.editorEngine.groupSelection?.());
    } else if (command === "ungroup-selection") {
      changed = await executeDocumentCommand("ungroup-selection", () => state.editorEngine.ungroupSelection?.());
    } else if (command === "link-selection") {
      changed = await executeDocumentCommand("link-selection", () => state.editorEngine.linkSelection?.());
    } else if (command === "set-link-policy") {
      changed = await executeDocumentCommand(
        { type: "set-link-policy", payload: { policy: commandPayload?.policy || "auto" } },
        () => state.editorEngine.setLinkPolicyForSelection?.(commandPayload?.policy || "auto"),
      );
    } else if (command === "unlink-selection") {
      changed = await executeDocumentCommand("unlink-selection", () => state.editorEngine.unlinkSelection?.());
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
    if (commandChanged(changed)) {
      options.renderDocumentChange?.(changed) || options.renderDocument();
    } else {
      options.renderEditorOverlay();
      options.refreshCommandAvailability();
    }
    return true;
  }

  return {
    writeClipboardFromSelection,
    pasteFromSystemClipboard,
    writeNativeClipboardFromSelection: writeClipboardFromSelection,
    pasteFromNativeClipboard: pasteFromSystemClipboard,
    hasPortableClipboard: () => Boolean(portableClipboardPayload),
    runEditorCommand,
  };
}
