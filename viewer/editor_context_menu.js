export function createCanvasContextMenuHost(options) {
  let activeContextMenuState = null;

  function uniformValue(values) {
    const normalized = values.filter((value) => value != null && value !== "");
    if (!normalized.length) {
      return null;
    }
    return normalized.every((value) => value === normalized[0]) ? normalized[0] : null;
  }

  async function currentClipboardHasPasteContent() {
    const state = options.state();
    if (!state.editorEngine) {
      return false;
    }
    try {
      if (await Promise.resolve(state.editorEngine.hasClipboard?.())) {
        return true;
      }
    } catch (error) {
      console.warn("Failed to inspect engine clipboard", error);
    }
    if (options.hasPortableClipboard?.()) {
      return true;
    }
    if (!options.desktopFileHost?.available || !state.editorEngine.pasteClipboardJson) {
      return false;
    }
    try {
      const payload = await options.desktopFileHost.readClipboard();
      return Boolean(
        payload?.chemsemaFragmentJson
        || payload?.chemsemaDocumentJson
        || payload?.cdxml
        || String(payload?.text || "").includes("<CDXML")
        || payload?.imageDataBase64,
      );
    } catch (error) {
      console.warn("Failed to inspect native clipboard", error);
      return false;
    }
  }

  function createCanvasContextMenu() {
    const menu = document.createElement("div");
    menu.className = "canvas-context-menu";
    menu.hidden = true;
    menu.setAttribute("role", "menu");
    menu.setAttribute("aria-label", "Canvas menu");

    menu.addEventListener("contextmenu", (event) => {
      event.preventDefault();
    });
    menu.addEventListener("click", (event) => {
      const item = event.target.closest("[data-canvas-context-command]");
      if (!item || item.disabled || item.dataset.hasSubmenu === "true") {
        return;
      }
      const command = item.dataset.canvasContextCommand;
      const value = item.dataset.canvasContextValue || "";
      void runCanvasContextMenuCommand(command, value);
    });

    return menu;
  }

  const canvasContextMenu = createCanvasContextMenu();

  async function updateCanvasContextMenuAvailability() {
    if (!canvasContextMenu || canvasContextMenu.hidden) {
      return;
    }
    renderCanvasContextMenu(await buildCanvasContextMenuItems(activeContextMenuState?.hit || { kind: "canvas" }));
  }

  function hideCanvasContextMenu() {
    if (canvasContextMenu.hidden) {
      return;
    }
    canvasContextMenu.hidden = true;
  }

  async function finishTemporaryContextSelection() {
    if (!activeContextMenuState?.temporarySelection) {
      activeContextMenuState = null;
      return;
    }
    activeContextMenuState = null;
    if (await options.state().editorEngine?.clearSelection?.()) {
      await options.renderSelectionOnlyUpdate?.();
    }
  }

  function closeCanvasContextMenu() {
    hideCanvasContextMenu();
    void finishTemporaryContextSelection();
  }

  function canvasContextMenuItem(item, depth = 0) {
    if (item.type === "separator") {
      const separator = document.createElement("div");
      separator.className = "canvas-context-menu-separator";
      separator.setAttribute("role", "separator");
      return separator;
    }

    const entry = document.createElement("div");
    entry.className = "canvas-context-menu-entry";
    if (item.submenu?.length) {
      entry.classList.add("has-submenu");
    }

    const button = document.createElement("button");
    button.type = "button";
    button.className = "canvas-context-menu-item";
    button.dataset.canvasContextCommand = item.command || "";
    button.dataset.canvasContextValue = item.value || "";
    button.dataset.hasSubmenu = item.submenu?.length ? "true" : "false";
    button.disabled = !!item.disabled;
    button.setAttribute("role", "menuitem");
    if (item.checked) {
      button.classList.add("is-checked");
      button.setAttribute("aria-checked", "true");
    }

    const check = document.createElement("span");
    check.className = "canvas-context-menu-check";
    check.textContent = item.checked ? "*" : "";
    const label = document.createElement("span");
    label.className = "canvas-context-menu-label";
    label.textContent = item.label || "";
    const shortcut = document.createElement("span");
    shortcut.className = "canvas-context-menu-shortcut";
    shortcut.textContent = item.submenu?.length ? ">" : item.shortcut || "";
    button.append(check, label, shortcut);
    entry.appendChild(button);

    if (item.submenu?.length) {
      const submenu = document.createElement("div");
      submenu.className = "canvas-context-submenu";
      submenu.setAttribute("role", "menu");
      if (depth >= 1) {
        submenu.classList.add("is-nested");
      }
      item.submenu.forEach((child) => submenu.appendChild(canvasContextMenuItem(child, depth + 1)));
      entry.appendChild(submenu);
    }
    return entry;
  }

  function renderCanvasContextMenu(items) {
    canvasContextMenu.innerHTML = "";
    items.forEach((item) => canvasContextMenu.appendChild(canvasContextMenuItem(item)));
  }

  async function openCanvasContextMenu(event) {
    event.preventDefault();
    event.stopPropagation();
    if (!options.isEditingRustDocument()) {
      closeCanvasContextMenu();
      return;
    }
    const point = options.svgPointFromEvent(event);
    let hit = await contextHitTest(point);
    const temporarySelection = options.editorState().activeTool !== "select"
      && hit.kind !== "canvas"
      && !hit.selected;
    if (hit.kind !== "canvas" && !hit.selected) {
      await options.selectClickTarget(point, false);
      await options.renderSelectionOnlyUpdate(point);
      hit = await contextHitTest(point);
    }
    activeContextMenuState = {
      hit,
      point,
      temporarySelection,
      actionTaken: false,
    };
    renderCanvasContextMenu(await buildCanvasContextMenuItems(hit));
    canvasContextMenu.hidden = false;
    const margin = 6;
    const width = canvasContextMenu.offsetWidth;
    const height = canvasContextMenu.offsetHeight;
    const left = Math.max(margin, Math.min(event.clientX, window.innerWidth - width - margin));
    const top = Math.max(margin, Math.min(event.clientY, window.innerHeight - height - margin));
    canvasContextMenu.style.left = `${left}px`;
    canvasContextMenu.style.top = `${top}px`;
    canvasContextMenu.querySelector("button:not(:disabled):not([data-has-submenu='true'])")?.focus?.({ preventScroll: true });
  }

  async function contextHitTest(point) {
    if (!options.state().editorEngine?.contextHitTestJson) {
      return { kind: "canvas" };
    }
    try {
      return options.parseEngineJson(
        await options.state().editorEngine.contextHitTestJson(point.x, point.y),
        { kind: "canvas" },
      ) || { kind: "canvas" };
    } catch (error) {
      console.warn("Failed to hit-test context menu target", error);
      return { kind: "canvas" };
    }
  }

  function styleColorForObject(object) {
    const style = options.state().currentDocument?.styles?.[object?.styleRef];
    return options.cssColorToHex(
      object?.payload?.fill
      || object?.payload?.stroke
      || style?.fill
      || style?.stroke
      || object?.payload?.color
      || "#000000",
    );
  }

  function selectedUniformColor() {
    const info = options.currentSelectionInfo();
    const colors = [];
    for (const object of info.sceneObjects) {
      colors.push(styleColorForObject(object));
    }
    for (const bond of info.bonds) {
      colors.push(options.cssColorToHex(bond.stroke || "#000000"));
    }
    for (const node of info.labelNodes.concat(info.nodes)) {
      colors.push(options.cssColorToHex(node.label?.fill || "#000000"));
    }
    return uniformValue(colors);
  }

  function lineObjectStyle(object) {
    const style = options.state().currentDocument?.styles?.[object?.styleRef] || {};
    const arrowHead = object?.payload?.arrowHead || {};
    if (arrowHead.bold) {
      return "bold";
    }
    if (Array.isArray(style.dashArray) && style.dashArray.length) {
      return "dashed";
    }
    return "plain";
  }

  function selectedUniformLineStyle() {
    const lines = options.selectedSceneObjects().filter((object) => object.type === "line");
    return uniformValue(lines.map(lineObjectStyle));
  }

  function selectedUniformArrowEndpoint(endpoint) {
    const lines = options.selectedSceneObjects().filter((object) => object.type === "line");
    return uniformValue(lines.map((object) => object.payload?.arrowHead?.[endpoint] || "none"));
  }

  async function buildCanvasContextMenuItems(hit) {
    if (!options.state().editorEngine?.contextMenuJson) {
      return [];
    }
    const hasPaste = await currentClipboardHasPasteContent();
    return options.parseEngineJson(
      await options.state().editorEngine.contextMenuJson(JSON.stringify(hit || { kind: "canvas" }), hasPaste),
      [],
    ) || [];
  }

  async function executeAtomProperty(value, executeDocumentCommand) {
    const separatorIndex = value.indexOf(":");
    const property = separatorIndex >= 0 ? value.slice(0, separatorIndex) : value;
    let propertyValue = separatorIndex >= 0 ? value.slice(separatorIndex + 1) : "";
    if (propertyValue === "__prompt__") {
      const entered = await options.atomPropertyDialogHost?.choose(property);
      if (entered == null) {
        await finishTemporaryContextSelection();
        return false;
      }
      propertyValue = entered;
    }
    return executeDocumentCommand(
      {
        type: "set-atom-property-for-selection",
        payload: { property, value: propertyValue || null },
      },
      () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
        type: "set-atom-property-for-selection",
        property,
        value: propertyValue || null,
      })),
    );
  }

  async function executeBondProperty(value, executeDocumentCommand) {
    const separatorIndex = value.indexOf(":");
    const property = separatorIndex >= 0 ? value.slice(0, separatorIndex) : value;
    const propertyValue = separatorIndex >= 0 ? value.slice(separatorIndex + 1) : "";
    return executeDocumentCommand(
      {
        type: "set-bond-property-for-selection",
        payload: { property, value: propertyValue || null },
      },
      () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
        type: "set-bond-property-for-selection",
        property,
        value: propertyValue || null,
      })),
    );
  }

  async function runCanvasContextMenuDialogCommand(command, value, executeDocumentCommand) {
    if (command === "stoichiometry-datum-dialog") {
      let spec = null;
      try {
        spec = JSON.parse(value || "null");
      } catch {
        spec = null;
      }
      if (spec) {
        await options.numericDialogHost?.choosePayload?.(spec, async (nextValue) => {
          const payload = {
            type: "set-stoichiometry-datum",
            objectId: spec.objectId,
            componentId: spec.componentId,
            rowId: spec.rowId,
            value: String(nextValue),
            unit: spec.field?.unit || null,
          };
          return executeDocumentCommand(
            payload,
            () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify(payload)),
          );
        });
      }
      return { handled: true, changed: false };
    }
    if (command === "table-borders-dialog") {
      let spec = null;
      try {
        spec = JSON.parse(value || "null");
      } catch {
        spec = null;
      }
      if (spec) {
        await options.tableDialogHost?.chooseBorders(spec);
      }
      return { handled: true, changed: false };
    }
    if (command === "plasmid-map-dialog") {
      let spec = null;
      try {
        spec = JSON.parse(value || "null");
      } catch {
        spec = null;
      }
      const decision = await options.plasmidMapDialogHost?.choose(spec);
      if (!decision) {
        return { handled: true, changed: false };
      }
      const changed = await executeDocumentCommand(
        {
          type: "set-plasmid-map",
          objectId: spec.objectId,
          data: decision,
        },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "set-plasmid-map",
          objectId: spec.objectId,
          data: decision,
        })),
      );
      return { handled: true, changed };
    }
    if (command === "bio-shape-dialog") {
      let spec = null;
      try {
        spec = JSON.parse(value || "null");
      } catch {
        spec = null;
      }
      const decision = await options.bioShapeDialogHost?.choose(spec);
      if (!decision) {
        return { handled: true, changed: false };
      }
      const payload = {
        type: "set-bio-shape",
        objectId: spec.objectId,
        data: decision,
      };
      const changed = await executeDocumentCommand(
        payload,
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify(payload)),
      );
      return { handled: true, changed };
    }
    if (command === "image-crop-dialog") {
      let spec = null;
      try {
        spec = JSON.parse(value || "null");
      } catch {
        spec = null;
      }
      const decision = await options.imageCropDialogHost?.choose(spec);
      if (!decision) {
        return { handled: true, changed: false };
      }
      const payload = {
        type: "set-image-crop",
        objectId: decision.objectId,
        crop: decision.crop,
      };
      const changed = await executeDocumentCommand(
        payload,
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify(payload)),
      );
      return { handled: true, changed };
    }
    if (command === "text-line-spacing") {
      await options.numericDialogHost.choose("line-height");
      return { handled: true, changed: false };
    }
    if (command === "smiles-dialog") {
      const point = activeContextMenuState?.point;
      if (point) {
        await options.smilesDialogHost?.open(point);
      }
      return { handled: true, changed: false };
    }
    if (command === "chemical-property-dialog") {
      const decision = await options.chemicalPropertyDialogHost?.choose();
      if (decision?.action === "apply") {
        const changed = await executeDocumentCommand(
          { type: "apply-chemical-property", payload: decision.payload },
          () => options.state().editorEngine?.applyChemicalPropertyDialogJson?.(
            JSON.stringify(decision.payload),
          ),
        );
        return { handled: true, changed };
      }
      if (decision?.action === "delete") {
        const changed = await executeDocumentCommand(
          "delete-chemical-property",
          () => options.state().editorEngine?.deleteSelectedChemicalProperty?.(),
        );
        return { handled: true, changed };
      }
      return { handled: true, changed: false };
    }
    if (command === "logical-objects-dialog") {
      await options.logicalObjectsDialogHost?.open();
      return { handled: true, changed: false };
    }
    if (command === "create-annotation") {
      const decision = await options.annotationDialogHost?.choose(value);
      if (!decision) {
        return { handled: true, changed: false };
      }
      const changed = await executeDocumentCommand(
        {
          type: "create-annotation",
          annotation: decision.annotation,
          properties: decision.properties,
        },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "create-annotation",
          annotation: decision.annotation,
          properties: decision.properties,
        })),
      );
      return { handled: true, changed };
    }
    if (command === "annotation-dialog") {
      const decision = await options.annotationDialogHost?.choose("selected");
      if (!decision?.objectId) {
        return { handled: true, changed: false };
      }
      const changed = await executeDocumentCommand(
        {
          type: "update-annotation",
          objectId: decision.objectId,
          properties: decision.properties,
        },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "update-annotation",
          objectId: decision.objectId,
          properties: decision.properties,
        })),
      );
      return { handled: true, changed };
    }
    if (command === "chemical-copy") {
      await copyChemicalAnalysis(value);
      return { handled: true, changed: false };
    }
    if (command === "nmr-predict") {
      try {
        await options.nmrPredictionHost.predict(value);
      } catch (error) {
        const message = String(error?.message || error || "NMR prediction failed")
          .replace(/^Error:\s*/i, "");
        options.transientNotificationHost?.show(message, { error: true, duration: 4200 });
      }
      return { handled: true, changed: false };
    }
    if (command === "object-settings") {
      await options.objectSettingsHost.chooseObjectSettings();
      return { handled: true, changed: false };
    }
    if (command === "scale-dialog" || command === "rotate-dialog") {
      await options.numericDialogHost.choose(command === "scale-dialog" ? "scale" : "rotate");
      return { handled: true, changed: false };
    }
    return { handled: false, changed: false };
  }

  async function runCanvasContextMenuCommand(command, value) {
    if (!command || command === "noop") {
      return;
    }
    if (activeContextMenuState) {
      activeContextMenuState.actionTaken = true;
    }
    hideCanvasContextMenu();
    let changed = false;
    const executeDocumentCommand = async (chemsemaCommand, apply) => {
      if (!options.commandEngine?.executeEngineCommand) {
        throw new Error("Canvas context-menu commands require the shared command engine.");
      }
      const result = await options.commandEngine.executeEngineCommand(chemsemaCommand, apply);
      if (result.changed) {
        options.renderDocumentChange?.(result) || options.renderDocument();
      }
      return !!result.changed;
    };
    const dialogResult = await runCanvasContextMenuDialogCommand(
      command,
      value,
      executeDocumentCommand,
    );
    if (dialogResult.handled) {
      if (!dialogResult.changed) {
        options.renderEditorOverlay();
        options.refreshCommandAvailability();
      }
      await finishTemporaryContextSelection();
      return;
    }
    if (["cut", "copy", "paste", "delete", "select-all"].includes(command)) {
      changed = await options.runEditorCommand(command);
    } else if (command === "image-crop-reset") {
      const payload = { type: "set-image-crop", objectId: value, crop: null };
      changed = await executeDocumentCommand(
        payload,
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify(payload)),
      );
    } else if (command === "insert-image") {
      options.openImageFilePickerAt?.(activeContextMenuState?.point || null);
      await finishTemporaryContextSelection();
      return;
    } else if (command === "order") {
      changed = await executeDocumentCommand(
        { type: "apply-selection-order", payload: { command: value } },
        () => options.state().editorEngine?.applySelectionOrderCommand?.(value),
      );
    } else if (command === "arrange") {
      changed = await executeDocumentCommand(
        { type: "apply-selection-arrange", payload: { command: value } },
        () => options.state().editorEngine?.applySelectionArrangeCommand?.(value),
      );
    } else if (command === "group") {
      changed = await executeDocumentCommand("group-selection", () => options.state().editorEngine?.groupSelection?.());
    } else if (command === "ungroup") {
      changed = await executeDocumentCommand("ungroup-selection", () => options.state().editorEngine?.ungroupSelection?.());
    } else if (command === "set-selection-locked") {
      changed = await executeDocumentCommand({
        type: "set-objects-locked",
        objectIds: [],
        locked: value === "true",
      });
    } else if (command === "link") {
      changed = await executeDocumentCommand("link-selection", () => options.state().editorEngine?.linkSelection?.());
    } else if (command === "link-policy") {
      changed = await executeDocumentCommand(
        { type: "set-link-policy", payload: { policy: value } },
        () => options.state().editorEngine?.setLinkPolicyForSelection?.(value),
      );
    } else if (command === "unlink") {
      changed = await executeDocumentCommand("unlink-selection", () => options.state().editorEngine?.unlinkSelection?.());
    } else if (command === "color") {
      changed = await options.applySelectionColor(value);
    } else if (command === "color-other") {
      options.openColorDialog(selectedUniformColor() || options.editorState().selectionColor || "#000000", async (color) => {
        await options.applySelectionColor(color);
        await finishTemporaryContextSelection();
      }, { colorHost: options.colorHost });
      return;
    } else if (command === "molecular-highlight" || command === "molecular-highlight-remove") {
      changed = await executeDocumentCommand(
        { type: "apply-molecular-highlight", color: command.endsWith("-remove") ? null : value },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "apply-molecular-highlight",
          color: command.endsWith("-remove") ? null : value,
        })),
      );
    } else if (command === "ring-fill" || command === "ring-fill-remove") {
      changed = await executeDocumentCommand(
        { type: "apply-ring-fill", color: command.endsWith("-remove") ? null : value },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "apply-ring-fill",
          color: command.endsWith("-remove") ? null : value,
        })),
      );
    } else if (command === "molecular-highlight-other" || command === "ring-fill-other") {
      const commandType = command === "molecular-highlight-other"
        ? "apply-molecular-highlight"
        : "apply-ring-fill";
      options.openColorDialog("#00ff00", async (color) => {
        await executeDocumentCommand(
          { type: commandType, color },
          () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
            type: commandType,
            color,
          })),
        );
        await finishTemporaryContextSelection();
      }, { colorHost: options.colorHost });
      return;
    } else if (command === "shape-style") {
      changed = await executeDocumentCommand(
        { type: "apply-shape-style", payload: { changes: { shapeStyle: value } } },
        () => options.state().editorEngine?.applyShapeStyleToSelection?.(value),
      );
    } else if (command === "table-edit") {
      const [objectId, rowText, columnText, action] = value.split(":");
      changed = await executeDocumentCommand(
        {
          type: "edit-table",
          objectId,
          row: Number(rowText),
          column: Number(columnText),
          action,
        },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "edit-table",
          objectId,
          row: Number(rowText),
          column: Number(columnText),
          action,
        })),
      );
    } else if (command === "analyze-stoichiometry") {
      changed = await executeDocumentCommand(
        { type: "analyze-stoichiometry", reactionStepId: null },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "analyze-stoichiometry",
          reactionStepId: null,
        })),
      );
    } else if (command === "stoichiometry-grid-edit") {
      const first = value.indexOf(":");
      const second = value.indexOf(":", first + 1);
      const objectId = first >= 0 ? value.slice(0, first) : value;
      const action = first >= 0 && second >= 0 ? value.slice(first + 1, second) : "";
      const entityId = second >= 0 ? value.slice(second + 1) : "";
      changed = await executeDocumentCommand(
        {
          type: "edit-stoichiometry-grid",
          objectId,
          action,
          entityId: entityId || null,
        },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "edit-stoichiometry-grid",
          objectId,
          action,
          entityId: entityId || null,
        })),
      );
    } else if (command === "orbital-template") {
      changed = await executeDocumentCommand(
        { type: "apply-orbital-style", payload: { changes: { template: value } } },
        () => options.state().editorEngine?.applyOrbitalTemplateToSelection?.(value),
      );
    } else if (command === "orbital-style") {
      changed = await executeDocumentCommand(
        { type: "apply-orbital-style", payload: { changes: { style: value } } },
        () => options.state().editorEngine?.applyOrbitalStyleToSelection?.(value),
      );
    } else if (command === "orbital-phase") {
      changed = await executeDocumentCommand(
        { type: "apply-orbital-style", payload: { changes: { phase: value } } },
        () => options.state().editorEngine?.applyOrbitalPhaseToSelection?.(value),
      );
    } else if (command === "bracket-kind") {
      changed = await executeDocumentCommand(
        { type: "apply-bracket-style", payload: { changes: { kind: value } } },
        () => options.state().editorEngine?.applyBracketKindToSelection?.(value),
      );
    } else if (command === "line-style") {
      changed = await executeDocumentCommand(
        { type: "apply-line-style", payload: { changes: { lineStyle: value } } },
        () => options.state().editorEngine?.applyLineStyleToSelection?.(value),
      );
    } else if (command === "bond-style") {
      changed = await executeDocumentCommand(
        { type: "apply-bond-style", payload: { changes: { variant: value } } },
        () => options.state().editorEngine?.applyBondStyleToSelection?.(value),
      );
    } else if (command === "text-style") {
      const separatorIndex = value.indexOf(":");
      const styleCommand = separatorIndex >= 0 ? value.slice(0, separatorIndex) : value;
      const styleValue = separatorIndex >= 0 ? value.slice(separatorIndex + 1) : "";
      changed = await executeDocumentCommand(
        { type: "apply-text-style", payload: { changes: { [styleCommand]: styleValue } } },
        () => options.state().editorEngine?.applyTextStyleToSelection?.(styleCommand, styleValue),
      );
    } else if (command === "chemical-check" || command === "interpret-chemically") {
      changed = await executeDocumentCommand(
        { type: "apply-text-style", payload: { changes: { interpretChemically: value !== "off" } } },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "set-interpret-chemically-for-selection",
          enabled: value !== "off",
        })),
      );
    } else if (command === "implicit-hydrogen-count") {
      const count = value === "auto" ? null : Number.parseInt(value, 10);
      changed = await executeDocumentCommand(
        { type: "apply-text-style", payload: { changes: { implicitHydrogenCount: count } } },
        () => options.state().editorEngine?.executeCommandJson?.(JSON.stringify({
          type: "set-implicit-hydrogen-count-for-selection",
          count,
        })),
      );
    } else if (command === "atom-property") {
      changed = await executeAtomProperty(value, executeDocumentCommand);
    } else if (command === "bond-property") {
      changed = await executeBondProperty(value, executeDocumentCommand);
    } else if (command === "expand-label") {
      changed = await executeDocumentCommand("expand-labels", () => options.state().editorEngine?.expandLabelsInSelection?.());
    } else if (command === "center-page") {
      changed = await executeDocumentCommand("center-selection-on-page", () => options.state().editorEngine?.centerSelectionOnPage?.());
    } else if (command === "edit-text") {
      const point = activeContextMenuState?.point;
      if (point) {
        await options.openTextEditorAt(point);
        changed = true;
      }
    } else if (command === "arrow-bold") {
      syncEditorArrowStateFromSelectedLine();
      options.editorState().arrowBold = selectedUniformLineStyle() !== "bold";
      changed = await options.applyArrowOptionsToSelection();
    } else if (command === "arrow-endpoint") {
      syncEditorArrowStateFromSelectedLine();
      const [endpoint, style] = value.split(":");
      const nextStyle = style || "none";
      if (isEquilibriumArrowType(options.editorState().arrowType) && (nextStyle === "left" || nextStyle === "right")) {
        options.editorState().arrowHeadStyle = nextStyle;
        options.editorState().arrowTailStyle = nextStyle;
        options.editorState().arrowHead = true;
        options.editorState().arrowTail = true;
      } else if (endpoint === "head") {
        options.editorState().arrowHeadStyle = selectedUniformArrowEndpoint("head") === endpointStylePayloadName(nextStyle) ? "none" : nextStyle;
        options.editorState().arrowHead = options.editorState().arrowHeadStyle !== "none";
      } else {
        options.editorState().arrowTailStyle = selectedUniformArrowEndpoint("tail") === endpointStylePayloadName(nextStyle) ? "none" : nextStyle;
        options.editorState().arrowTail = options.editorState().arrowTailStyle !== "none";
      }
      changed = await options.applyArrowOptionsToSelection();
    }
    if (!changed) {
      options.renderEditorOverlay();
      options.refreshCommandAvailability();
    }
    await finishTemporaryContextSelection();
  }

  async function copyChemicalAnalysis(format) {
    const displayName = format === "inchi-key" ? "InChIKey"
      : format === "inchi" ? "InChI"
        : "SMILES";
    try {
      const result = await options.commandEngine.executeCommand({
        type: "chemical-analysis",
        format,
        targets: {},
        meta: { source: "chemical-analysis-menu" },
      }, { sync: false });
      let value = String(result?.output?.value || "");
      if (!value && result?.output?.molfile && format !== "smiles") {
        const identifiers = await options.inchiHost.analyzeMolfile(result.output.molfile);
        value = format === "inchi-key" ? identifiers.inchikey : identifiers.inchi;
      }
      if (!value) {
        throw new Error(displayName + " was not generated.");
      }
      if (options.desktopFileHost?.available) {
        await options.desktopFileHost.writeClipboard({ text: value });
      } else if (globalThis.navigator?.clipboard?.writeText) {
        await globalThis.navigator.clipboard.writeText(value);
      } else {
        legacyCopyText(value);
      }
      options.transientNotificationHost?.show(displayName + " copied");
    } catch (error) {
      const message = String(error?.message || error || displayName + " generation failed")
        .replace(/^Error:\s*/i, "");
      options.transientNotificationHost?.show(message, { error: true, duration: 3600 });
    }
  }

  function legacyCopyText(value) {
    const textarea = document.createElement("textarea");
    textarea.value = value;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const copied = document.execCommand("copy");
    textarea.remove();
    if (!copied) {
      throw new Error("Clipboard access is unavailable.");
    }
  }

  function endpointStylePayloadName(style) {
    if (style === "left") {
      return "half-left";
    }
    if (style === "right") {
      return "half-right";
    }
    return style;
  }

  function syncEditorArrowStateFromSelectedLine() {
    const line = options.selectedSceneObjects().find((object) => object.type === "line");
    if (!line) {
      return;
    }
    const arrowHead = line.payload?.arrowHead || {};
    const kind = arrowHead.kind || "solid";
    if (["solid", "curved", "curved-mirror", "hollow", "open", "equilibrium", "unequal-equilibrium"].includes(kind)) {
      options.editorState().arrowType = kind;
    }
    options.editorState().arrowHeadSize = arrowHeadSizeFromPayload(arrowHead, kind);
    const curve = Math.abs(Number(arrowHead.curve || 0));
    if (curve >= 260) {
      options.editorState().arrowCurve = "270";
    } else if (curve >= 150) {
      options.editorState().arrowCurve = "180";
    } else if (curve >= 105) {
      options.editorState().arrowCurve = "120";
    } else if (curve >= 60) {
      options.editorState().arrowCurve = "90";
    }
    const head = arrowHead.head || "none";
    const tail = arrowHead.tail || "none";
    options.editorState().arrowHeadStyle = head === "half-left" ? "left" : head === "half-right" ? "right" : head;
    options.editorState().arrowTailStyle = tail === "half-left" ? "left" : tail === "half-right" ? "right" : tail;
    options.editorState().arrowHead = options.editorState().arrowHeadStyle !== "none";
    options.editorState().arrowTail = options.editorState().arrowTailStyle !== "none";
    options.editorState().arrowNoGo = arrowHead.noGo === "cross" || arrowHead.noGo === "hash" ? arrowHead.noGo : "none";
    options.editorState().arrowBold = !!arrowHead.bold;
  }

  function arrowHeadSizeFromPayload(arrowHead, kind) {
    const length = Number(arrowHead.length || 0);
    if (kind === "hollow" || kind === "open") {
      return length >= 9 ? "large" : "small";
    }
    if (length >= 18) {
      return "large";
    }
    if (length >= 12.5) {
      return "medium";
    }
    return "small";
  }

  function isEquilibriumArrowType(type) {
    return type === "equilibrium" || type === "unequal-equilibrium";
  }

  return {
    canvasContextMenu,
    updateCanvasContextMenuAvailability,
    closeCanvasContextMenu,
    openCanvasContextMenu,
    isHidden: () => canvasContextMenu.hidden,
    containsTarget: (target) => canvasContextMenu.contains(target),
  };
}
