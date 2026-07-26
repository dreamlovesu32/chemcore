export function createEditorInteractionScenarios(scope) {
  const { assert, openViewer, ENDPOINT_FEEDBACK_RADIUS_PX } = scope;

  async function verifyNativeTableToolWorkflow(browser) {
    const { page, errors } = await openViewer(browser);
    await page.locator('button[data-tool="table"]').click();
    const box = await page.locator("#viewer-container").boundingBox();
    const start = { x: box.x + box.width / 2 - 120, y: box.y + box.height / 2 - 70 };
    const end = { x: start.x + 240, y: start.y + 140 };
    await page.mouse.move(start.x, start.y);
    await page.mouse.down();
    await page.mouse.move(end.x, end.y, { steps: 6 });
    await page.mouse.up();
    const insertDialog = page.locator(".table-insert-dialog");
    await insertDialog.waitFor({ state: "visible" });
    await insertDialog.locator('input[name="rows"]').fill("3");
    await insertDialog.locator('input[name="columns"]').fill("4");
    await insertDialog.locator('button[type="submit"]').click();
    await page.waitForFunction(() => {
      const doc = JSON.parse(window.__chemsemaDebug?.state?.editorEngine?.documentJson?.() || "null");
      return doc?.objects?.some((object) => object.type === "table"
        && object.payload?.table?.rows === 3
        && object.payload?.table?.columns === 4);
    });

    const cell = await page.evaluate(() => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
      const table = doc.objects.find((object) => object.type === "table");
      const data = table.payload.table;
      const x = table.transform.translate[0] + (data.columnGuides[0] + data.columnGuides[1]) * 0.5;
      const y = table.transform.translate[1] + (data.rowGuides[0] + data.rowGuides[1]) * 0.5;
      const client = window.__chemsemaDebug.worldToClient(x, y);
      return { x: client.x, y: client.y, id: table.id };
    });
    await page.mouse.move(cell.x, cell.y);
    await page.waitForFunction((tableId) => {
      const state = JSON.parse(window.__chemsemaDebug.state.editorEngine.stateJson());
      return state.overlay?.hoverTableCell?.objectId === tableId
        || state.overlay?.hover_table_cell?.object_id === tableId;
    }, cell.id);

    await page.mouse.click(cell.x, cell.y, { button: "right" });
    await page.locator('.canvas-context-menu:not([hidden]) [data-canvas-context-command="table-edit"]')
      .filter({ hasText: "Add Row After" })
      .click();
    await page.waitForFunction((tableId) => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
      return doc.objects.find((object) => object.id === tableId)?.payload?.table?.rows === 4;
    }, cell.id);

    await page.mouse.click(cell.x, cell.y, { button: "right" });
    await page.locator('.canvas-context-menu:not([hidden]) [data-canvas-context-command="table-borders-dialog"]')
      .click();
    const bordersDialog = page.locator(".table-borders-dialog");
    await bordersDialog.waitFor({ state: "visible" });
    await bordersDialog.locator('input[name="setting"][value="all"]').check();
    await bordersDialog.locator('select[name="lineStyle"]').selectOption("dashed");
    await bordersDialog.locator('input[name="width"]').fill("1.25");
    await bordersDialog.locator('input[name="color"]').fill("#204080");
    await bordersDialog.locator('button[type="submit"]').click();
    await page.waitForFunction((tableId) => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
      const cellData = doc.objects
        .find((object) => object.id === tableId)
        ?.payload?.table?.cells?.find((candidate) => candidate.row === 0 && candidate.column === 0);
      return ["top", "left", "bottom", "right"].every((side) => {
        const border = cellData?.borders?.[side];
        return border?.visible
          && border.lineStyle === "dashed"
          && border.width === 1.25
          && border.color === "#204080";
      });
    }, cell.id);

    await page.close();
    assert(!errors.length, `Viewer console errors during table workflow: ${errors.join("\n")}`);
  }

  async function verifyBondDrawing(browser) {
    const { page, errors } = await openViewer(browser);
    await page.locator('button[data-tool="bond"]').click();
    const box = await page.locator("#viewer-container").boundingBox();
    const start = { x: box.x + box.width / 2 - 80, y: box.y + box.height / 2 };
    const end = { x: start.x + 120, y: start.y };
    await page.mouse.move(start.x, start.y);
    await page.mouse.down();
    await page.mouse.move(end.x, end.y, { steps: 8 });
    const previewState = await page.evaluate(() => {
      const previewEnd = document.querySelector('[data-role="preview-end"]');
      const matrix = previewEnd?.getScreenCTM?.();
      const scale = matrix ? Math.hypot(matrix.a, matrix.b) : 1;
      return {
        hadPreview: !!document.querySelector('[data-role="preview-bond"], [data-layer="document-bond-creation-preview"] [data-bond-id="__preview_bond"]'),
        previewEndRadiusPx: previewEnd ? Number(previewEnd.getAttribute("r") || 0) * scale : 0,
      };
    });
    await page.mouse.up();
    await page.waitForTimeout(250);
    const result = await page.evaluate(() => {
      const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      return {
        previewLeft: !!document.querySelector('[data-role^="preview-"]'),
        creationPreviewLeft: !!document.querySelector('[data-layer="document-bond-creation-preview"]'),
        dragPreviewChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
        changed: !!command?.changed,
        bondTargets: command?.targets?.bonds?.length || command?.created?.bonds?.length || 0,
        hasRenderedBond: /data-bond-id=/.test(document.querySelector("#viewer-svg")?.outerHTML || ""),
      };
    });
    await page.close();
    assert(previewState.hadPreview, "Bond drag did not show a preview.");
    assert(
      Math.abs(previewState.previewEndRadiusPx - ENDPOINT_FEEDBACK_RADIUS_PX) < 0.35,
      `Bond preview endpoint radius did not track bold bond width: ${JSON.stringify(previewState)}`,
    );
    assert(!result.previewLeft && !result.creationPreviewLeft && result.dragPreviewChildren === 0, `Bond preview remained after pointerup: ${JSON.stringify(result)}`);
    assert(result.changed && result.bondTargets > 0 && result.hasRenderedBond, "Bond drag did not commit a rendered bond.");
    assert(!errors.length, `Viewer console errors during bond drawing: ${errors.join("\n")}`);
  }
  
  async function visibleEndpointTarget(page) {
    return page.evaluate(() => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const objectType = (object) => object?.type || object?.objectType || object?.object_type;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        if (objectType(object) !== "molecule") {
          continue;
        }
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        const node = fragment?.nodes?.find((candidate) => Array.isArray(candidate.position));
        if (!node) {
          continue;
        }
        const translate = object.transform?.translate || [0, 0];
        const x = Number(translate[0] || 0) + Number(node.position[0] || 0);
        const y = Number(translate[1] || 0) + Number(node.position[1] || 0);
        const client = window.__chemsemaDebug.worldToClient(x, y);
        if (client) {
          return { x: client.x, y: client.y, nodeId: node.id };
        }
      }
      return null;
    });
  }
  
  async function documentBondCount(page) {
    return page.evaluate(() => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const objectType = (object) => object?.type || object?.objectType || object?.object_type;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      let count = 0;
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        if (objectType(object) !== "molecule") {
          continue;
        }
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        count += fragment?.bonds?.length || 0;
      }
      return count;
    });
  }
  
  async function drawTwoBondJunction(page, center) {
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(center.x, center.y);
    await page.mouse.down();
    await page.mouse.move(center.x - 95, center.y - 48, { steps: 8 });
    await page.mouse.up();
    await page.waitForTimeout(120);
    await page.mouse.move(center.x, center.y);
    await page.mouse.down();
    await page.mouse.move(center.x + 95, center.y - 48, { steps: 8 });
    await page.mouse.up();
    await page.waitForTimeout(180);
    return page.evaluate(() => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const objectType = (object) => object?.type || object?.objectType || object?.object_type;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        if (objectType(object) !== "molecule") {
          continue;
        }
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        if (!fragment?.nodes?.length || !fragment?.bonds?.length) {
          continue;
        }
        const translate = object.transform?.translate || [0, 0];
        const node = fragment.nodes.find((candidate) => (
          fragment.bonds.filter((bond) => bond.begin === candidate.id || bond.end === candidate.id).length >= 2
        ));
        if (!node) {
          continue;
        }
        const adjacentBonds = fragment.bonds
          .filter((bond) => bond.begin === node.id || bond.end === node.id)
          .map((bond) => {
            const otherId = bond.begin === node.id ? bond.end : bond.begin;
            const other = fragment.nodes.find((candidate) => candidate.id === otherId);
            return { bondId: bond.id, otherX: Number(other?.position?.[0] || 0) };
          });
        const leftBond = adjacentBonds.sort((a, b) => a.otherX - b.otherX)[0];
        const x = Number(translate[0] || 0) + Number(node.position[0] || 0);
        const y = Number(translate[1] || 0) + Number(node.position[1] || 0);
        const client = window.__chemsemaDebug.worldToClient(x, y);
        return client ? { x: client.x, y: client.y, nodeId: node.id, leftBondId: leftBond?.bondId || "" } : null;
      }
      return null;
    });
  }
  
  async function firstEndpointWithAdjacentBond(page) {
    return page.evaluate(() => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const objectType = (object) => object?.type || object?.objectType || object?.object_type;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        if (objectType(object) !== "molecule") {
          continue;
        }
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        const bond = fragment?.bonds?.[0];
        const node = bond ? fragment.nodes.find((candidate) => candidate.id === bond.begin) : null;
        if (!node) {
          continue;
        }
        const translate = object.transform?.translate || [0, 0];
        const x = Number(translate[0] || 0) + Number(node.position[0] || 0);
        const y = Number(translate[1] || 0) + Number(node.position[1] || 0);
        const client = window.__chemsemaDebug.worldToClient(x, y);
        return client ? { x: client.x, y: client.y, nodeId: node.id, bondId: bond.id } : null;
      }
      return null;
    });
  }
  
  async function documentBondMaxX(page, bondId) {
    return page.evaluate((id) => {
      const escapeCss = window.CSS?.escape || ((value) => String(value).replace(/["\\]/g, "\\$&"));
      const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-bond-id="${escapeCss(id)}"]`)];
      let maxX = -Infinity;
      for (const element of elements) {
        if (element.tagName.toLowerCase() === "polygon" || element.tagName.toLowerCase() === "polyline") {
          for (const pair of (element.getAttribute("points") || "").trim().split(/\s+/)) {
            const [x] = pair.split(",").map(Number);
            if (Number.isFinite(x)) {
              maxX = Math.max(maxX, x);
            }
          }
        } else if (element.hasAttribute("x1") || element.hasAttribute("x2")) {
          maxX = Math.max(maxX, Number(element.getAttribute("x1")), Number(element.getAttribute("x2")));
        }
      }
      return Number.isFinite(maxX) ? maxX : null;
    }, bondId);
  }
  
  async function verifyBondCreationUsesKernelLocalPreview(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(center.x - 70, center.y);
    await page.mouse.down();
    await page.mouse.move(center.x + 40, center.y, { steps: 6 });
    await page.mouse.up();
    await page.waitForTimeout(160);
  
    const endpoint = await firstEndpointWithAdjacentBond(page);
    assert(endpoint?.bondId, `Could not locate endpoint for kernel bond preview: ${JSON.stringify(endpoint)}`);
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(endpoint.x, endpoint.y);
    await page.mouse.down();
    await page.mouse.move(endpoint.x - 70, endpoint.y - 45, { steps: 8 });
    const preview = await page.evaluate((bondId) => {
      const escapeCss = window.CSS?.escape || ((value) => String(value).replace(/["\\]/g, "\\$&"));
      const layer = document.querySelector('[data-layer="document-bond-creation-preview"]');
      const original = document.querySelector(`[data-layer="document-content"] [data-bond-id="${escapeCss(bondId)}"]`);
      return {
        hasLayer: !!layer,
        hasPreviewBond: !!layer?.querySelector('[data-bond-id="__preview_bond"]'),
        hasExistingBond: !!layer?.querySelector(`[data-bond-id="${escapeCss(bondId)}"]`),
        originalHidden: original?.style.visibility === "hidden",
        overlayPreviewBond: !!document.querySelector('[data-layer="editor-overlay"] [data-role="preview-bond"]'),
        dragPreviewChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
      };
    }, endpoint.bondId);
    await page.mouse.up();
    await page.waitForTimeout(180);
    const cleared = await page.evaluate(() => !document.querySelector('[data-layer="document-bond-creation-preview"]'));
    await page.close();
    assert(preview.hasLayer && preview.hasPreviewBond && preview.hasExistingBond && preview.originalHidden, `Bond creation did not use kernel local preview: ${JSON.stringify(preview)}`);
    assert(!preview.overlayPreviewBond && preview.dragPreviewChildren === 0, `Bond creation kept duplicate frontend preview: ${JSON.stringify(preview)}`);
    assert(cleared, "Bond creation local preview remained after commit.");
    assert(!errors.length, `Viewer console errors during kernel bond preview: ${errors.join("\n")}`);
  }
  
  async function verifyElementEndpointPatchUpdatesConnectedBonds(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    const junction = await drawTwoBondJunction(page, center);
    assert(junction?.leftBondId, `Could not create junction for element patch regression: ${JSON.stringify(junction)}`);
    const beforeMaxX = await documentBondMaxX(page, junction.leftBondId);
    assert(Number.isFinite(beforeMaxX), `Could not read original bond DOM: ${JSON.stringify({ junction, beforeMaxX })}`);
  
    await page.locator(".quick-palette-toggle-element").click();
    await page.mouse.move(junction.x, junction.y);
    await page.mouse.down();
    await page.mouse.up();
    await page.waitForTimeout(180);
  
    const after = await page.evaluate((bondId) => {
      const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      const escapeCss = window.CSS?.escape || ((value) => String(value).replace(/["\\]/g, "\\$&"));
      const labelText = [...document.querySelectorAll('[data-layer="document-content"] [data-node-id]')]
        .map((element) => element.textContent || "")
        .join("\n");
      return {
        changed: !!command?.changed,
        updatedBonds: command?.updated?.bonds || [],
        targetBonds: command?.targets?.bonds || [],
        hasBondDom: !!document.querySelector(`[data-layer="document-content"] [data-bond-id="${escapeCss(bondId)}"]`),
        labelText,
      };
    }, junction.leftBondId);
    const afterMaxX = await documentBondMaxX(page, junction.leftBondId);
    await page.close();
  
    assert(after.changed && after.hasBondDom, `Element endpoint replacement did not commit/render: ${JSON.stringify(after)}`);
    assert(
      Number.isFinite(afterMaxX) && afterMaxX < beforeMaxX - 2,
      `Connected bond DOM did not update after endpoint element replacement: ${JSON.stringify({ beforeMaxX, afterMaxX, after })}`,
    );
    assert(!errors.length, `Viewer console errors during element endpoint patch: ${errors.join("\n")}`);
  }
  
  async function verifyJunctionDragUsesBackendPrimitivePatch(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    const junction = await drawTwoBondJunction(page, center);
    assert(junction?.nodeId, `Could not create junction drag target: ${JSON.stringify(junction)}`);
  
    await page.evaluate(() => {
      window.__chemsemaDebug.backendMovePreviewStats = { samples: [] };
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.move(junction.x, junction.y);
    await page.waitForTimeout(100);
    await page.mouse.down();
    await page.mouse.move(junction.x + 62, junction.y + 22, { steps: 8 });
    await page.waitForFunction((nodeId) => {
      const last = window.__chemsemaDebug.backendMovePreviewStats?.last;
      return last?.changed
        && last.nodeCount >= 1
        && last.bondCount >= 2
        && last.patched
        && (last.selection?.nodes || []).includes(nodeId);
    }, junction.nodeId, { timeout: 2500 });
  
    const preview = await page.evaluate((nodeId) => {
      const doc = window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const connectedBonds = new Set();
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        for (const bond of fragment?.bonds || []) {
          if (bond.begin === nodeId || bond.end === nodeId) {
            connectedBonds.add(bond.id);
          }
        }
      }
      const primitives = JSON.parse(window.__chemsemaDebug.state.editorEngine.renderTargetsJson(JSON.stringify({
        nodes: [nodeId],
        bonds: [...connectedBonds],
      })));
      const backendCount = primitives
        .filter((primitive) => (
          primitive.role !== "document-knockout"
          && primitive.role !== "document_knockout"
          && (
            primitive.nodeId === nodeId
            || primitive.node_id === nodeId
            || connectedBonds.has(primitive.bondId || primitive.bond_id)
          )
        ))
        .length;
      const selectors = [
        `[data-node-id="${CSS.escape(nodeId)}"]`,
        ...[...connectedBonds].map((bondId) => `[data-bond-id="${CSS.escape(bondId)}"]`),
      ];
      const domElements = [...document.querySelectorAll(`[data-layer="document-content"] ${selectors.join(",")}`)]
        .filter((element) => getComputedStyle(element).visibility !== "hidden");
      const stats = window.__chemsemaDebug.backendMovePreviewStats?.last || null;
      return {
        nodeId,
        connectedBonds: [...connectedBonds],
        backendCount,
        domCount: domElements.length,
        partialPreviewChildren: document.querySelector('[data-layer="document-partial-bond-preview"]')?.childElementCount || 0,
        stats,
      };
    }, junction.nodeId);
    await page.mouse.up();
    await page.waitForTimeout(180);
    await page.close();
  
    assert(
      preview.connectedBonds.length >= 2
        && preview.backendCount > 0
        && preview.domCount === preview.backendCount
        && preview.partialPreviewChildren === 0,
      `Junction drag did not use a clean backend primitive patch: ${JSON.stringify(preview)}`,
    );
    assert(!errors.length, `Viewer console errors during junction backend preview: ${errors.join("\n")}`);
  }
  
  async function verifyTransformedArrowRenderHitAndSelection(browser) {
    const { page, errors } = await openViewer(browser);
    const arrowObjectId = "arrow_transformed_alignment";
    const documentData = {
      format: { name: "chemsema", version: "0.1", unit: "pt" },
      document: {
        id: "doc_arrow_alignment",
        title: "Arrow alignment",
        page: { width: 640, height: 420, background: "#ffffff" },
        meta: null,
      },
      styles: {
        style_molecule_default: {
          kind: "molecule",
          stroke: "#000000",
          strokeWidth: 0.75,
          fontFamily: "Arial",
          fontSize: 10,
        },
        style_arrow_default: {
          kind: "stroke",
          stroke: "#000000",
          strokeWidth: 0.75,
          lineCap: "butt",
          lineJoin: "miter",
        },
      },
      resources: {
        mol_editor: {
          type: "molecule_fragment2d",
          encoding: "chemsema.molecule.fragment2d",
          data: { nodes: [], bonds: [] },
          meta: null,
        },
      },
      objects: [
        {
          id: "obj_editor_molecule",
          type: "molecule",
          name: "molecule",
          visible: true,
          locked: false,
          zIndex: 10,
          transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
          styleRef: "style_molecule_default",
          meta: null,
          payload: {
            resourceRef: "mol_editor",
            bbox: [0, 0, 640, 420],
          },
        },
        {
          id: arrowObjectId,
          type: "line",
          name: "arrow",
          visible: true,
          locked: false,
          zIndex: 20,
          transform: { translate: [80, -40], rotate: 0, scale: [1, 1] },
          styleRef: "style_arrow_default",
          meta: { source: "viewer-smoke" },
          payload: {
            kind: "line",
            points: [[100, 140], [250, 140]],
            head: "end",
            tail: "none",
            arrowHead: {
              kind: "solid",
              curve: 0,
              head: "full",
              tail: "none",
              length: 8,
              centerLength: 6,
              width: 6,
              bold: false,
              noGo: "none",
            },
          },
        },
      ],
    };
  
    await page.evaluate((doc) => window.__chemsemaDebug.loadDocumentForTest(doc), documentData);
    await page.waitForFunction(
      (objectId) => !!document.querySelector(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`),
      arrowObjectId,
      { timeout: 2500 },
    );
  
    const alignment = await page.evaluate((objectId) => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null");
      const object = doc.objects.find((candidate) => candidate.id === objectId);
      const translate = object?.transform?.translate || [0, 0];
      const points = object?.payload?.points || [];
      const mid = {
        x: Number(translate[0] || 0) + (Number(points[0]?.[0] || 0) + Number(points[1]?.[0] || 0)) * 0.5,
        y: Number(translate[1] || 0) + (Number(points[0]?.[1] || 0) + Number(points[1]?.[1] || 0)) * 0.5,
      };
      const client = window.__chemsemaDebug.worldToClient(mid.x, mid.y);
      const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 || rect.height > 0);
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      const domCenter = rects.length ? { x: (left + right) * 0.5, y: (top + bottom) * 0.5 } : null;
      const hit = JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(mid.x, mid.y) || "null");
      return {
        mid,
        client,
        domCenter,
        domDistance: client && domCenter ? Math.hypot(client.x - domCenter.x, client.y - domCenter.y) : null,
        hit,
        rectCount: rects.length,
      };
    }, arrowObjectId);
    assert(
      alignment.hit?.objectId === arrowObjectId && alignment.rectCount > 0 && alignment.domDistance !== null && alignment.domDistance < 18,
      `Transformed arrow render/hit coordinates diverged: ${JSON.stringify(alignment)}`,
    );
  
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.click(alignment.client.x, alignment.client.y);
    await page.waitForFunction((objectId) => {
      const selection = window.__chemsemaDebug.engineState?.selection || {};
      return (selection.arrowObjects || selection.arrow_objects || []).includes(objectId);
    }, arrowObjectId, { timeout: 1200 });
  
    await page.close();
    assert(!errors.length, `Viewer console errors during transformed arrow alignment: ${errors.join("\n")}`);
  }
  
  async function verifyCursorAnchoredWheelZoom(browser) {
    const { page, errors } = await openViewer(browser);
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
    });
    const box = await page.locator("#viewer-container").boundingBox();
    const anchor = {
      x: box.x + box.width * 0.72,
      y: box.y + box.height * 0.38,
    };
    const before = await page.evaluate(({ x, y }) => ({
      zoom: Number(document.querySelector("#zoom-input")?.value || 0),
      world: window.__chemsemaDebug.clientPointToWorld(x, y),
    }), anchor);
    await page.evaluate(({ x, y }) => {
      const container = document.querySelector("#viewer-container");
      container.dispatchEvent(new WheelEvent("wheel", {
        bubbles: true,
        cancelable: true,
        clientX: x,
        clientY: y,
        deltaY: -120,
        ctrlKey: true,
      }));
    }, anchor);
    await page.waitForTimeout(120);
    const after = await page.evaluate((world) => ({
      zoom: Number(document.querySelector("#zoom-input")?.value || 0),
      client: window.__chemsemaDebug.worldToClient(world.x, world.y),
    }), before.world);
    const drift = Math.hypot(after.client.x - anchor.x, after.client.y - anchor.y);
    await page.close();
  
    assert(after.zoom > before.zoom, `Wheel zoom did not increase zoom: ${JSON.stringify({ before, after })}`);
    assert(drift < 2.5, `Wheel zoom did not keep cursor world point anchored: ${JSON.stringify({ anchor, before, after, drift })}`);
    assert(!errors.length, `Viewer console errors during cursor anchored zoom: ${errors.join("\n")}`);
  }
  
  async function verifyQuickPaletteAndSelectDragRegression(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(center.x - 80, center.y);
    await page.mouse.down();
    await page.mouse.move(center.x + 40, center.y, { steps: 6 });
    await page.mouse.up();
    await page.waitForTimeout(150);
    const endpoint = await visibleEndpointTarget(page);
    assert(endpoint, "Could not locate endpoint for select drag regression.");
    const beforeDragBonds = await documentBondCount(page);
  
    await page.locator('button[data-tool="select"]').click();
    await page.waitForFunction(() => document.querySelector('button[data-tool="select"]')?.classList.contains("is-active"));
    await page.mouse.move(endpoint.x, endpoint.y);
    await page.mouse.down();
    await page.mouse.move(endpoint.x + 52, endpoint.y + 18, { steps: 8 });
    await page.mouse.up();
    await page.waitForTimeout(180);
    const afterDrag = await page.evaluate(() => {
      const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      return {
        commandType: command?.commandType || command?.command_type || command?.type || "",
        createdBonds: command?.created?.bonds?.length || 0,
        targetBonds: command?.targets?.bonds?.length || 0,
        activeTool: window.__chemsemaDebug.engineState?.tool?.activeTool
          || window.__chemsemaDebug.engineState?.tool?.active_tool
          || "",
        shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
      };
    });
    const afterDragBonds = await documentBondCount(page);
    assert(afterDragBonds === beforeDragBonds, `Select atom drag created bonds: ${JSON.stringify({ beforeDragBonds, afterDragBonds, afterDrag })}`);
    assert(afterDrag.activeTool === "select", `Select drag left engine on the wrong tool: ${JSON.stringify(afterDrag)}`);
    assert(!afterDrag.shieldActive, `Select drag left pointer shield active: ${JSON.stringify(afterDrag)}`);
  
    await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.classList.add("is-active"));
    await page.locator(".quick-palette-toggle-element").click();
    const elementState = await page.evaluate(() => ({
      open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
      mode: document.querySelector(".quick-palette")?.dataset.mode || "",
      selectActive: document.querySelector('button[data-tool="select"]')?.classList.contains("is-active") || false,
      shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
    }));
    assert(
      elementState.open && elementState.mode === "element" && elementState.selectActive && !elementState.shieldActive,
      `Element quick palette did not open above/clear shield: ${JSON.stringify(elementState)}`,
    );
  
    await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.classList.add("is-active"));
    await page.locator(".quick-palette-toggle-symbol").click();
    const symbolState = await page.evaluate(() => ({
      open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
      mode: document.querySelector(".quick-palette")?.dataset.mode || "",
      selectActive: document.querySelector('button[data-tool="select"]')?.classList.contains("is-active") || false,
      shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
    }));
    assert(
      symbolState.open && symbolState.mode === "symbol" && symbolState.selectActive && !symbolState.shieldActive,
      `Symbol quick palette did not open above/clear shield: ${JSON.stringify(symbolState)}`,
    );
  
    await page.close();
    assert(!errors.length, `Viewer console errors during quick palette/select regression: ${errors.join("\n")}`);
  }
  
  async function interactionFeedbackState(page) {
    return page.evaluate(() => {
      const endpoint = document.querySelector('[data-role="hover-endpoint"]');
      const matrix = endpoint?.getScreenCTM?.();
      const scale = matrix ? Math.hypot(matrix.a, matrix.b) : 1;
      return {
        hoverEndpointCount: document.querySelectorAll('[data-role="hover-endpoint"]').length,
        hoverLabelGlyphCount: document.querySelectorAll('[data-role="hover-label-glyph"]').length,
        previewEndCount: document.querySelectorAll('[data-role="preview-end"]').length,
        hoverCount: document.querySelectorAll('[data-role^="hover-"]').length,
        previewCount: document.querySelectorAll('[data-role^="preview-"]').length,
        dragPreviewChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
        endpointRadiusPx: endpoint ? Number(endpoint.getAttribute("r") || 0) * scale : 0,
      };
    });
  }
  
  async function verifyEndpointFeedbackRules(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(center.x - 90, center.y);
    await page.mouse.down();
    await page.mouse.move(center.x + 30, center.y, { steps: 6 });
    await page.mouse.up();
    await page.waitForTimeout(160);
    const endpoint = await visibleEndpointTarget(page);
    assert(endpoint, "Could not locate a visible endpoint target for feedback rules.");
  
    await page.mouse.move(endpoint.x + 80, endpoint.y + 80);
    await page.waitForTimeout(80);
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(endpoint.x, endpoint.y);
    await page.waitForTimeout(120);
    const bondHover = await interactionFeedbackState(page);
    assert(bondHover.hoverEndpointCount > 0, `Bond tool did not show endpoint hover: ${JSON.stringify(bondHover)}`);
    assert(
      Math.abs(bondHover.endpointRadiusPx - ENDPOINT_FEEDBACK_RADIUS_PX) < 0.35,
      `Endpoint hover radius did not track bold bond width: ${JSON.stringify(bondHover)}`,
    );
  
    const symbolKinds = [
      "circle-plus",
      "plus",
      "radical-cation",
      "lone-pair",
      "circle-minus",
      "minus",
      "radical-anion",
      "electron",
    ];
    await page.locator('button[data-tool="symbol"]').click();
    for (const kind of symbolKinds) {
      await page.locator(`[data-secondary-value="symbol-kind-${kind}"]`).click();
      await page.mouse.move(endpoint.x + 70, endpoint.y + 70);
      await page.waitForTimeout(40);
      await page.mouse.move(endpoint.x, endpoint.y);
      await page.waitForTimeout(120);
      const state = await interactionFeedbackState(page);
      assert(
        state.hoverEndpointCount > 0 && state.previewEndCount === 0,
        `${kind} symbol tool did not focus a bare endpoint: ${JSON.stringify(state)}`,
      );
    }
  
    for (const tool of ["arrow", "bracket", "shape", "orbital", "templates"]) {
      await page.locator(`button[data-tool="${tool}"]`).click();
      await page.mouse.move(endpoint.x + 70, endpoint.y + 70);
      await page.waitForTimeout(40);
      await page.mouse.move(endpoint.x, endpoint.y);
      await page.waitForTimeout(120);
      const state = await interactionFeedbackState(page);
      assert(
        state.hoverEndpointCount === 0 && state.previewEndCount === 0,
        `${tool} tool showed bond endpoint feedback over an atom: ${JSON.stringify(state)}`,
      );
    }
  
    await page.locator(".quick-palette-toggle-element").click();
    await page.mouse.click(endpoint.x, endpoint.y);
    await page.waitForTimeout(160);
    await page.locator('button[data-tool="symbol"]').click();
    for (const kind of symbolKinds) {
      await page.locator(`[data-secondary-value="symbol-kind-${kind}"]`).click();
      await page.mouse.move(endpoint.x + 70, endpoint.y + 70);
      await page.waitForTimeout(40);
      await page.mouse.move(endpoint.x, endpoint.y);
      await page.waitForTimeout(120);
      const state = await interactionFeedbackState(page);
      assert(
        state.hoverLabelGlyphCount > 0 && state.previewEndCount === 0,
        `${kind} symbol tool did not focus an attached label glyph: ${JSON.stringify(state)}`,
      );
    }
  
    await page.close();
    assert(!errors.length, `Viewer console errors during endpoint feedback rules: ${errors.join("\n")}`);
  }
  
  async function verifyGraphicObjectDragTracksPointerAndSelection(browser) {
    const cases = ["arrow", "symbol"];
    for (const tool of cases) {
      const { page, errors } = await openViewer(browser);
      const box = await page.locator("#viewer-container").boundingBox();
      const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  
      await page.locator(`button[data-tool="${tool}"]`).click();
      if (tool === "arrow") {
        await page.mouse.move(center.x - 80, center.y);
        await page.mouse.down();
        await page.mouse.move(center.x + 80, center.y, { steps: 8 });
        await page.mouse.up();
      } else {
        await page.mouse.click(center.x, center.y);
      }
      await page.waitForTimeout(160);
  
      const objectId = await page.evaluate(() => {
        const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
        return command?.targets?.objects?.[0] || command?.created?.objects?.[0] || "";
      });
      assert(objectId, `${tool} creation did not return an object id.`);
  
      await page.locator('button[data-tool="select"]').click();
      await page.waitForTimeout(160);
      const readGeometry = (id) => {
        const documentLayer = document.querySelector('[data-layer="document-content"]');
        const allObjectElements = [
          ...documentLayer.querySelectorAll(`[data-object-id="${CSS.escape(id)}"]`),
        ];
        const outermostObjectElements = allObjectElements.filter((element) => (
          !element.parentElement?.closest?.(`[data-object-id="${CSS.escape(id)}"]`)
        ));
        const unionRect = (elements) => elements.reduce((bounds, element) => {
          const rect = element.getBoundingClientRect();
          if (!bounds) {
            return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
          }
          return {
            left: Math.min(bounds.left, rect.left),
            top: Math.min(bounds.top, rect.top),
            right: Math.max(bounds.right, rect.right),
            bottom: Math.max(bounds.bottom, rect.bottom),
          };
        }, null);
        const objectRect = unionRect(outermostObjectElements);
        const selectionRect = document
          .querySelector('[data-layer="editor-overlay"] [data-role="selection-box"]')
          ?.getBoundingClientRect();
        return {
          objectRect,
          selectionRect: selectionRect ? {
            left: selectionRect.left,
            top: selectionRect.top,
            right: selectionRect.right,
            bottom: selectionRect.bottom,
          } : null,
          transformingCount: allObjectElements.filter((element) => (
            element.classList.contains("is-preview-transforming")
          )).length,
        };
      };
      const before = await page.evaluate(readGeometry, objectId);
      assert(before.objectRect && before.selectionRect, `${tool} selection geometry was not rendered: ${JSON.stringify(before)}`);
      const start = {
        x: (before.objectRect.left + before.objectRect.right) * 0.5,
        y: (before.objectRect.top + before.objectRect.bottom) * 0.5,
      };
      const delta = { x: 100, y: 60 };
      const end = { x: start.x + delta.x, y: start.y + delta.y };
  
      await page.mouse.move(start.x, start.y);
      await page.mouse.down();
      await page.mouse.move(end.x, end.y, { steps: 6 });
      await page.waitForTimeout(120);
      const during = await page.evaluate(readGeometry, objectId);
      const duringDelta = {
        x: during.objectRect.left - before.objectRect.left,
        y: during.objectRect.top - before.objectRect.top,
      };
      assert(
        Math.abs(duringDelta.x - delta.x) < 2 && Math.abs(duringDelta.y - delta.y) < 2,
        `${tool} preview did not track the pointer once: ${JSON.stringify({ before, during, delta, duringDelta })}`,
      );
      assert(
        during.transformingCount > 0,
        `${tool} drag did not enter the local preview path: ${JSON.stringify(during)}`,
      );
  
      await page.mouse.up();
      await page.waitForTimeout(240);
      const after = await page.evaluate(readGeometry, objectId);
      const afterDelta = {
        x: after.objectRect.left - before.objectRect.left,
        y: after.objectRect.top - before.objectRect.top,
      };
      assert(
        Math.abs(afterDelta.x - delta.x) < 2 && Math.abs(afterDelta.y - delta.y) < 2,
        `${tool} committed DOM did not match the pointer delta: ${JSON.stringify({ before, after, delta, afterDelta })}`,
      );
      assert(
        after.selectionRect.left <= after.objectRect.left + 1
          && after.selectionRect.top <= after.objectRect.top + 1
          && after.selectionRect.right >= after.objectRect.right - 1
          && after.selectionRect.bottom >= after.objectRect.bottom - 1,
        `${tool} selection box did not wrap the committed object: ${JSON.stringify({ after })}`,
      );
  
      await page.close();
      assert(!errors.length, `Viewer console errors during ${tool} object drag: ${errors.join("\n")}`);
    }
  }
  
  async function verifyCreationDragKeepsCanvasVisibleAfterToolSwitch(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  
    await page.locator('button[data-tool="bond"]').click();
    await page.mouse.move(center.x - 80, center.y);
    await page.mouse.down();
    await page.mouse.move(center.x + 40, center.y, { steps: 6 });
    await page.mouse.up();
    await page.waitForTimeout(150);
  
    const baseline = await page.evaluate(() => ({
      hasBondDom: !!document.querySelector('[data-layer="document-content"] [data-bond-id]'),
      documentChildren: document.querySelector('[data-layer="document-content"]')?.childElementCount || 0,
    }));
    assert(baseline.hasBondDom && baseline.documentChildren > 0, `Baseline visible document was not rendered: ${JSON.stringify(baseline)}`);
  
    const cases = [
      { tool: "arrow", start: [-70, 80], end: [100, 80], expectedObjects: 1 },
      { tool: "shape", start: [-70, 150], end: [60, 250], expectedObjects: 1 },
      { tool: "orbital", start: [120, 170], end: [210, 250], expectedObjects: 1 },
      { tool: "bracket", start: [170, 90], end: [310, 240], expectedObjects: 1, closeText: true },
    ];
  
    for (const item of cases) {
      await page.locator(`button[data-tool="${item.tool}"]`).click();
      const before = await page.evaluate(() => {
        const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
        return {
          objectCount: flatten(window.__chemsemaDebug.engineState.document.objects || [])
            .filter((object) => (object.type || object.objectType || object.object_type) !== "molecule")
            .length,
          shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
        };
      });
      assert(!before.shieldActive, `${item.tool} tool started with pointer shield still active.`);
  
      const [startDx, startDy] = item.start;
      const [endDx, endDy] = item.end;
      await page.mouse.move(center.x + startDx, center.y + startDy);
      await page.mouse.down();
      await page.mouse.move(center.x + endDx, center.y + endDy, { steps: 8 });
  
      const during = await page.evaluate(() => {
        const layer = document.querySelector('[data-layer="document-content"]');
        const style = layer ? getComputedStyle(layer) : null;
        return {
          visibility: layer?.style.visibility || "",
          computedVisibility: style?.visibility || "",
          display: style?.display || "",
          childCount: layer?.childElementCount || 0,
          hasBondDom: !!document.querySelector('[data-layer="document-content"] [data-bond-id]'),
          shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
          previewCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="preview-"], [data-layer="editor-overlay"] [data-object-id], .canvas-drag-preview-svg > *').length,
        };
      });
      assert(during.visibility !== "hidden" && during.computedVisibility !== "hidden", `${item.tool} drag hid the document layer: ${JSON.stringify(during)}`);
      assert(during.display !== "none" && during.childCount > 0 && during.hasBondDom, `${item.tool} drag blanked the canvas: ${JSON.stringify(during)}`);
  
      await page.mouse.up();
      await page.waitForTimeout(80);
      const afterPointerUpOverlay = await page.evaluate(() => {
        const overlay = document.querySelector('[data-layer="editor-overlay"]');
        return {
          hoverCount: overlay?.querySelectorAll('[data-role^="hover-"]').length || 0,
          previewCount: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
          overlayChildren: overlay?.childElementCount || 0,
          dragPreviewChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
        };
      });
      assert(
        afterPointerUpOverlay.hoverCount === 0
          && afterPointerUpOverlay.previewCount === 0
          && afterPointerUpOverlay.dragPreviewChildren === 0,
        `${item.tool} left hover/preview overlay after pointerup: ${JSON.stringify(afterPointerUpOverlay)}`,
      );
      await page.mouse.move(center.x - 260, center.y - 220);
      await page.waitForTimeout(120);
      const afterMoveOverlay = await page.evaluate(() => {
        const overlay = document.querySelector('[data-layer="editor-overlay"]');
        return {
          hoverCount: overlay?.querySelectorAll('[data-role^="hover-"]').length || 0,
          previewCount: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
          overlayChildren: overlay?.childElementCount || 0,
          dragPreviewChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
        };
      });
      assert(
        afterMoveOverlay.hoverCount === 0
          && afterMoveOverlay.previewCount === 0
          && afterMoveOverlay.dragPreviewChildren === 0,
        `${item.tool} hover/preview followed the cursor after commit: ${JSON.stringify(afterMoveOverlay)}`,
      );
      await page.waitForTimeout(50);
      if (item.closeText) {
        await page.keyboard.press("Escape");
        await page.waitForTimeout(50);
      }
      const after = await page.evaluate(() => {
        const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
        const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
        const objectIds = command?.targets?.objects?.length
          ? command.targets.objects
          : command?.created?.objects || [];
        return {
          changed: !!command?.changed,
          targets: command?.targets || null,
          created: command?.created || null,
          objectIds,
          objectCount: flatten(window.__chemsemaDebug.engineState.document.objects || [])
            .filter((object) => (object.type || object.objectType || object.object_type) !== "molecule")
            .length,
          shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
        };
      });
      assert(after.changed, `${item.tool} first drag after tool switch did not commit: ${JSON.stringify(after)}`);
      assert(after.objectCount >= before.objectCount + item.expectedObjects, `${item.tool} first drag after tool switch did not create an object: ${JSON.stringify({ before, after })}`);
      assert(!after.shieldActive, `${item.tool} pointerup left pointer shield active.`);
      if (after.objectIds?.length) {
        await page.locator('button[data-tool="select"]').click();
        await page.waitForTimeout(120);
        const selectState = await page.evaluate((objectIds) => ({
          objectDomCount: objectIds.reduce(
            (count, objectId) => count + document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`).length,
            0,
          ),
          selectionCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="selection-"]').length,
          hoverCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="hover-"]').length,
          previewCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="preview-"]').length,
        }), after.objectIds);
        assert(selectState.objectDomCount > 0, `${item.tool} object disappeared after switching to select: ${JSON.stringify({ after, selectState })}`);
        assert(selectState.hoverCount === 0 && selectState.previewCount === 0, `${item.tool} switching to select left hover/preview overlay: ${JSON.stringify(selectState)}`);
      }
    }
  
    await page.close();
    assert(!errors.length, `Viewer console errors during creation visibility regression: ${errors.join("\n")}`);
  }
  
  async function verifyDeleteToolTemporaryToolbarAndEmptyDocument(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    const shapes = [
      {
        start: { x: center.x - 180, y: center.y - 90 },
        end: { x: center.x - 80, y: center.y + 10 },
      },
      {
        start: { x: center.x + 60, y: center.y - 70 },
        end: { x: center.x + 170, y: center.y + 40 },
      },
    ];
  
    await page.locator('button[data-tool="shape"]').click();
    for (const shape of shapes) {
      await page.mouse.move(shape.start.x, shape.start.y);
      await page.mouse.down();
      await page.mouse.move(shape.end.x, shape.end.y, { steps: 8 });
      await page.mouse.up();
      await page.waitForTimeout(80);
    }
  
    const beforeDelete = await page.evaluate(() => {
      const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
      const shapeButton = document.querySelector('.tool-button[data-tool="shape"]');
      return {
        activeTool: window.__chemsemaDebug.editorState.activeTool,
        objectCount: flatten(window.__chemsemaDebug.engineState.document.objects || [])
          .filter((object) => (object.type || object.objectType || object.object_type) !== "molecule")
          .length,
        secondaryShapeButtons: document.querySelectorAll('#secondary-toolbar [data-secondary-value^="shape-"]').length,
        activeBackground: getComputedStyle(shapeButton).backgroundColor,
        activeBorderColor: getComputedStyle(shapeButton).borderColor,
        activeColor: getComputedStyle(shapeButton).color,
      };
    });
    assert(beforeDelete.activeTool === "shape", `Shape tool was not active before delete: ${JSON.stringify(beforeDelete)}`);
    assert(beforeDelete.objectCount >= 2, `Shape setup did not create objects before delete: ${JSON.stringify(beforeDelete)}`);
    assert(beforeDelete.secondaryShapeButtons > 0, `Shape secondary toolbar was not visible before delete: ${JSON.stringify(beforeDelete)}`);
  
    await page.locator('button[data-tool="delete"]').click();
    await page.waitForTimeout(120);
    const deleteToolbarState = await page.evaluate(() => {
      const deleteButton = document.querySelector('.icon-button[data-tool="delete"]');
      return {
        activeTool: window.__chemsemaDebug.editorState.activeTool,
        secondaryToolbarTool: window.__chemsemaDebug.editorState.secondaryToolbarTool,
        deleteActive: deleteButton?.classList.contains("is-active") || false,
        deleteBackground: getComputedStyle(deleteButton).backgroundColor,
        deleteBorderColor: getComputedStyle(deleteButton).borderColor,
        deleteColor: getComputedStyle(deleteButton).color,
        secondaryShapeButtons: document.querySelectorAll('#secondary-toolbar [data-secondary-value^="shape-"]').length,
        secondaryHtml: document.querySelector("#secondary-toolbar")?.innerHTML || "",
      };
    });
    assert(deleteToolbarState.activeTool === "delete", `Delete tool did not become active: ${JSON.stringify(deleteToolbarState)}`);
    assert(deleteToolbarState.secondaryToolbarTool === "shape", `Delete tool did not preserve shape as secondary toolbar source: ${JSON.stringify(deleteToolbarState)}`);
    assert(deleteToolbarState.secondaryShapeButtons > 0, `Delete tool hid the previous secondary toolbar: ${JSON.stringify(deleteToolbarState)}`);
    assert(deleteToolbarState.deleteActive, `Delete button did not receive active state: ${JSON.stringify(deleteToolbarState)}`);
    assert(
      deleteToolbarState.deleteBackground === beforeDelete.activeBackground
        && deleteToolbarState.deleteBorderColor === beforeDelete.activeBorderColor
        && deleteToolbarState.deleteColor === beforeDelete.activeColor,
      `Delete active style did not match tool active style: ${JSON.stringify({ beforeDelete, deleteToolbarState })}`,
    );
  
    await page.locator('#secondary-toolbar [data-secondary-value="shape-kind-rect"]').click();
    await page.waitForTimeout(120);
    const restoredToolState = await page.evaluate(() => ({
      activeTool: window.__chemsemaDebug.editorState.activeTool,
      shapeKind: window.__chemsemaDebug.editorState.shapeKind,
    }));
    assert(
      restoredToolState.activeTool === "shape" && restoredToolState.shapeKind === "rect",
      `Clicking secondary toolbar while delete was active did not restore the previous tool: ${JSON.stringify(restoredToolState)}`,
    );
  
    const shapeSelectionTarget = await page.evaluate(() => {
      const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
      const shapeIds = flatten(window.__chemsemaDebug.engineState.document.objects || [])
        .filter((object) => (object.type || object.objectType || object.object_type) === "shape")
        .map((object) => object.id);
      for (const objectId of shapeIds) {
        const rect = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
          .map((element) => element.getBoundingClientRect())
          .filter((candidate) => candidate.width > 8 && candidate.height > 8)
          .sort((a, b) => (b.width * b.height) - (a.width * a.height))[0];
        if (!rect) {
          continue;
        }
        return {
          objectId,
          center: { x: rect.left + rect.width * 0.5, y: rect.top + rect.height * 0.5 },
        };
      }
      return null;
    });
    assert(shapeSelectionTarget, "Could not find a rendered hollow shape target for delete-hit regression.");
  
    await page.locator('button[data-tool="delete"]').click();
    await page.mouse.click(shapeSelectionTarget.center.x, shapeSelectionTarget.center.y);
    await page.waitForTimeout(160);
    const unselectedInteriorDeleteState = await page.evaluate((objectId) => {
      const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
      return {
        activeTool: window.__chemsemaDebug.editorState.activeTool,
        objectExists: flatten(window.__chemsemaDebug.engineState.document.objects || []).some((object) => object.id === objectId),
        lastCommand: JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null"),
      };
    }, shapeSelectionTarget.objectId);
    assert(
      unselectedInteriorDeleteState.objectExists && unselectedInteriorDeleteState.lastCommand?.changed === false,
      `Unselected hollow shape interior acted like a delete hit: ${JSON.stringify(unselectedInteriorDeleteState)}`,
    );
  
    await page.evaluate(async () => {
      await window.__chemsemaDebug.resetEditorEngine();
    });
    await page.waitForFunction(() => document.querySelector('[data-layer="document-content"]'));
    await page.locator('button[data-tool="bond"]').click();
    const bondSegments = [
      { start: { x: center.x - 160, y: center.y - 70 }, end: { x: center.x - 60, y: center.y - 70 } },
      { start: { x: center.x + 60, y: center.y + 50 }, end: { x: center.x + 160, y: center.y + 50 } },
    ];
    for (const segment of bondSegments) {
      await page.mouse.move(segment.start.x, segment.start.y);
      await page.mouse.down();
      await page.mouse.move(segment.end.x, segment.end.y, { steps: 6 });
      await page.mouse.up();
      await page.waitForTimeout(120);
    }
  
    const documentStructureState = async () => page.evaluate(() => {
      const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      let nodeCount = 0;
      let bondCount = 0;
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        if ((object.type || object.objectType || object.object_type) !== "molecule") {
          continue;
        }
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        nodeCount += fragment?.nodes?.length || 0;
        bondCount += fragment?.bonds?.length || 0;
      }
      return {
        nodeCount,
        bondCount,
        bondDomCount: document.querySelectorAll('[data-layer="document-content"] [data-bond-id]').length,
        nodeDomCount: document.querySelectorAll('[data-layer="document-content"] [data-node-id]').length,
        documentChildren: document.querySelector('[data-layer="document-content"]')?.childElementCount || 0,
        hoverCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="hover-"]').length,
        previewCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="preview-"]').length,
      };
    });
    const beforeBondDelete = await documentStructureState();
    assert(
      beforeBondDelete.bondCount === 2 && beforeBondDelete.bondDomCount === 2,
      `Bond setup did not create two rendered bonds: ${JSON.stringify(beforeBondDelete)}`,
    );
  
    await page.locator('button[data-tool="delete"]').click();
    await page.waitForTimeout(80);
    const deletedCenters = [];
    const deleteOneVisibleBond = async () => {
      const beforeCount = await documentBondCount(page);
      const targets = await page.evaluate(() => {
        const byId = new Map();
        for (const element of document.querySelectorAll('[data-layer="document-content"] [data-bond-id]')) {
          const bondId = element.getAttribute("data-bond-id") || "";
          if (!bondId || byId.has(bondId)) {
            continue;
          }
          const rect = element.getBoundingClientRect();
          byId.set(bondId, {
            bondId,
            x: rect.left + rect.width * 0.5,
            y: rect.top + rect.height * 0.5,
          });
        }
        return [...byId.values()];
      });
      assert(targets.length > 0, `No visible bond target remained before delete: ${JSON.stringify(await documentStructureState())}`);
      await page.mouse.click(targets[0].x, targets[0].y);
      await page.waitForTimeout(160);
      const afterCount = await documentBondCount(page);
      assert(afterCount < beforeCount, `Delete tool did not remove the visible bond: ${JSON.stringify({ beforeCount, afterCount, targets, dom: await documentStructureState() })}`);
      deletedCenters.push(targets[0]);
    };
  
    while ((await documentBondCount(page)) > 0) {
      await deleteOneVisibleBond();
    }
    await page.waitForTimeout(240);
    for (const point of deletedCenters) {
      await page.mouse.move(point.x, point.y);
      await page.waitForTimeout(80);
    }
    const afterDeleteAll = {
      ...(await documentStructureState()),
      lastPatch: await page.evaluate(() => window.__chemsemaDebug.objectPrimitivePatchStats || null),
    };
    assert(
      afterDeleteAll.nodeCount === 0
        && afterDeleteAll.bondCount === 0
        && afterDeleteAll.bondDomCount === 0
        && afterDeleteAll.nodeDomCount === 0,
      `Deleting every bond left or resurrected document content: ${JSON.stringify(afterDeleteAll)}`,
    );
    assert(
      afterDeleteAll.hoverCount === 0 && afterDeleteAll.previewCount === 0,
      `Deleting every bond left stale hover or preview feedback: ${JSON.stringify(afterDeleteAll)}`,
    );
  
    await page.close();
    assert(!errors.length, `Viewer console errors during delete temporary toolbar regression: ${errors.join("\n")}`);
  }
  
  async function verifySelectedObjectSuppressesHover(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    const shapeStart = { x: center.x - 90, y: center.y - 70 };
    const shapeEnd = { x: center.x + 70, y: center.y + 60 };
    const shapeCenter = { x: (shapeStart.x + shapeEnd.x) * 0.5, y: (shapeStart.y + shapeEnd.y) * 0.5 };
    const bracketStart = { x: center.x + 150, y: center.y - 90 };
    const bracketEnd = { x: center.x + 230, y: center.y + 60 };
    const bracketHover = { x: bracketStart.x, y: (bracketStart.y + bracketEnd.y) * 0.5 };
  
    await page.locator('button[data-tool="shape"]').click();
    await page.mouse.move(shapeStart.x, shapeStart.y);
    await page.mouse.down();
    await page.mouse.move(shapeEnd.x, shapeEnd.y, { steps: 6 });
    await page.mouse.up();
    await page.mouse.move(shapeEnd.x + 80, shapeEnd.y + 80);
    await page.waitForTimeout(80);
  
    await page.locator('button[data-tool="bracket"]').click();
    await page.mouse.move(bracketStart.x, bracketStart.y);
    await page.mouse.down();
    await page.mouse.move(bracketEnd.x, bracketEnd.y, { steps: 6 });
    await page.mouse.up();
    await page.keyboard.press("Escape");
    await page.mouse.move(bracketEnd.x + 80, bracketEnd.y + 80);
    await page.waitForTimeout(80);
  
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.click(shapeCenter.x, shapeCenter.y);
    await page.waitForFunction(() => {
      const overlay = document.querySelector('[data-layer="editor-overlay"]');
      return (overlay?.querySelectorAll('[data-role^="selection-"]').length || 0) > 0;
    }, null, { timeout: 1200 });
  
    for (const point of [shapeEnd, shapeCenter]) {
      await page.mouse.move(point.x, point.y);
      await page.waitForTimeout(180);
      const overlayState = await page.evaluate(() => {
        const overlay = document.querySelector('[data-layer="editor-overlay"]');
        return {
          selectionCount: overlay?.querySelectorAll('[data-role^="selection-"]').length || 0,
          hoverCount: overlay?.querySelectorAll('[data-role^="hover-"]').length || 0,
          previewCount: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
        };
      });
      assert(overlayState.selectionCount > 0, `Selected object lost its selection overlay: ${JSON.stringify(overlayState)}`);
      assert(
        overlayState.hoverCount === 0 && overlayState.previewCount === 0,
        `Selected object showed stale hover/preview overlay: ${JSON.stringify(overlayState)}`,
      );
    }
  
    await page.mouse.move(bracketHover.x, bracketHover.y);
    await page.waitForTimeout(180);
    const fastHoverOverlayState = await page.evaluate(() => {
      const overlay = document.querySelector('[data-layer="editor-overlay"]');
      return {
        selectionCount: overlay?.querySelectorAll('[data-role^="selection-"]').length || 0,
        hoverCount: overlay?.querySelectorAll('[data-role^="hover-"]').length || 0,
        previewCount: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
      };
    });
    assert(
      fastHoverOverlayState.selectionCount > 0,
      `Fast hover over another object removed the selection overlay: ${JSON.stringify(fastHoverOverlayState)}`,
    );
    assert(
      fastHoverOverlayState.previewCount === 0,
      `Fast hover over another object left preview overlay: ${JSON.stringify(fastHoverOverlayState)}`,
    );
  
    await page.close();
    assert(!errors.length, `Viewer console errors during selected hover suppression regression: ${errors.join("\n")}`);
  }
  
  async function waitForCanvasCursor(page, x, y, expected, label) {
    await page.mouse.move(x, y);
    const readActual = ({ x: px, y: py }) => {
      const hit = document.elementFromPoint(px, py);
      const svg = document.querySelector("#viewer-svg");
      const matrix = svg?.getScreenCTM?.()?.inverse?.();
      const world = matrix ? new DOMPoint(px, py).matrixTransform(matrix) : null;
      return {
        hit: hit?.id || hit?.className || hit?.tagName || "",
        hitCursor: hit ? getComputedStyle(hit).cursor : "",
        containerCursor: getComputedStyle(document.querySelector("#viewer-container")).cursor,
        svgCursor: getComputedStyle(svg).cursor,
        shieldCursor: getComputedStyle(document.querySelector(".canvas-pointer-shield")).cursor,
        world: world ? { x: world.x, y: world.y } : null,
        shapeAction: world ? window.__chemsemaDebug?.state?.editorEngine?.hoverShapeAction?.(world.x, world.y) || "" : "",
        activeTool: window.__chemsemaDebug?.engineState?.tool?.activeTool
          || window.__chemsemaDebug?.engineState?.tool?.active_tool
          || null,
        selection: window.__chemsemaDebug?.engineState?.selection || null,
        fastSelectHoverStats: window.__chemsemaDebug?.fastSelectHoverStats || null,
      };
    };
    await page.waitForFunction(
      ({ x: px, y: py, values }) => {
        const hit = document.elementFromPoint(px, py);
        const cursors = [
          hit ? getComputedStyle(hit).cursor : "",
          getComputedStyle(document.querySelector("#viewer-container")).cursor,
          getComputedStyle(document.querySelector("#viewer-svg")).cursor,
          getComputedStyle(document.querySelector(".canvas-pointer-shield")).cursor,
        ];
        return [
          ...cursors,
        ].some((cursor) => values.includes(cursor));
      },
      { x, y, values: expected },
      { timeout: 1200 },
    ).catch(async (error) => {
      const actual = await page.evaluate(readActual, { x, y });
      throw new Error(`${label} cursor did not switch to ${expected.join("/")} at drag point: ${JSON.stringify(actual)}\n${error.message}`);
    });
    const actual = await page.evaluate(readActual, { x, y });
    assert(
      expected.includes(actual.hitCursor)
        || expected.includes(actual.containerCursor)
        || expected.includes(actual.svgCursor)
        || expected.includes(actual.shieldCursor),
      `${label} cursor did not switch to ${expected.join("/")} at drag point: ${JSON.stringify(actual)}`,
    );
    return actual;
  }
  
  async function verifyDragHandleCursors(browser) {
    const { page, errors } = await openViewer(browser);
    const box = await page.locator("#viewer-container").boundingBox();
    const center = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  
    await page.locator('button[data-tool="arrow"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const arrowStart = { x: center.x - 140, y: center.y - 80 };
    const arrowEnd = { x: center.x + 80, y: center.y - 80 };
    await page.mouse.move(arrowStart.x, arrowStart.y);
    await page.mouse.down();
    await page.mouse.move(arrowEnd.x, arrowEnd.y);
    await page.mouse.up();
    await page.waitForTimeout(120);
    await page.mouse.move(arrowEnd.x + 40, arrowEnd.y + 40);
    await page.waitForTimeout(40);
    await waitForCanvasCursor(page, arrowEnd.x, arrowEnd.y, ["move"], "Arrow endpoint");
  
    await page.locator('button[data-tool="shape"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const shapeStart = { x: center.x - 130, y: center.y + 30 };
    const shapeEnd = { x: center.x - 20, y: center.y + 140 };
    await page.mouse.move(shapeStart.x, shapeStart.y);
    await page.mouse.down();
    await page.mouse.move(shapeEnd.x, shapeEnd.y);
    await page.mouse.up();
    await page.waitForTimeout(120);
    await page.mouse.move(shapeEnd.x + 40, shapeEnd.y + 40);
    await page.waitForTimeout(40);
    await waitForCanvasCursor(
      page,
      shapeEnd.x,
      shapeEnd.y,
      ["nwse-resize", "nesw-resize", "ew-resize", "ns-resize"],
      "Shape resize handle",
    );
  
    await page.locator('button[data-tool="bracket"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const bracketStart = { x: center.x + 70, y: center.y + 20 };
    const bracketEnd = { x: center.x + 210, y: center.y + 160 };
    await page.mouse.move(bracketStart.x, bracketStart.y);
    await page.mouse.down();
    await page.mouse.move(bracketEnd.x, bracketEnd.y);
    await page.mouse.up();
    await page.waitForTimeout(120);
    await page.keyboard.press("Escape");
    await page.locator('button[data-tool="select"]').click();
    await page.waitForFunction(() => document.querySelector('button[data-tool="select"]')?.classList.contains("is-active"));
    const bracketCursorTargets = await page.evaluate(() => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const objects = (documentData.objects || []).flatMap((object) => visit(object, []));
      const sideObject = objects.find((object) => (
        (object.type || object.objectType || object.object_type) === "bracket"
        && (object.payload?.side || object.payload?.extra?.side)
      ));
      const bbox = sideObject?.payload?.bbox || [];
      const translate = sideObject?.transform?.translate || [0, 0];
      const kind = sideObject?.payload?.kind || sideObject?.payload?.extra?.kind || "round";
      const side = sideObject?.payload?.side || sideObject?.payload?.extra?.side || "left";
      const width = Number(bbox[2] || 0);
      const height = Number(bbox[3] || 0);
      const handleX = kind === "round"
        ? (side === "right" ? 0 : width)
        : (side === "right" ? width : 0);
      const tx = Number(translate[0] || 0) + Number(bbox[0] || 0);
      const ty = Number(translate[1] || 0) + Number(bbox[1] || 0);
      let body = null;
      const xRatios = [-0.15, 0, 0.1, 0.25, 0.5, 0.75, 0.9, 1, 1.15];
      const yRatios = [0.3, 0.35, 0.4, 0.45, 0.55, 0.6, 0.65, 0.7];
      for (const yRatio of yRatios) {
        for (const xRatio of xRatios) {
          const wx = tx + width * xRatio;
          const wy = ty + height * yRatio;
          const hit = JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(wx, wy) || "null");
          const action = window.__chemsemaDebug.state.editorEngine.hoverShapeAction?.(wx, wy) || "";
          if (hit?.objectId === sideObject?.id && !action) {
            body = window.__chemsemaDebug.worldToClient(wx, wy);
            break;
          }
        }
        if (body) {
          break;
        }
      }
      return {
        body,
        top: window.__chemsemaDebug.worldToClient(tx + handleX, ty),
        sideObjectId: sideObject?.id || "",
        siblingObjectId: objects.find((object) => (
          (object.type || object.objectType || object.object_type) === "bracket"
          && object.id !== sideObject?.id
        ))?.id || "",
      };
    });
    assert(bracketCursorTargets.sideObjectId && bracketCursorTargets.body && bracketCursorTargets.top, `Could not find bracket cursor targets: ${JSON.stringify(bracketCursorTargets)}`);
    await waitForCanvasCursor(
      page,
      bracketCursorTargets.body.x,
      bracketCursorTargets.body.y,
      ["grab"],
      "Selected bracket body",
    );
    await waitForCanvasCursor(
      page,
      bracketCursorTargets.top.x,
      bracketCursorTargets.top.y,
      ["grab"],
      "Selected bracket endpoint",
    );
    await page.mouse.click(center.x + 280, center.y - 160);
    await page.waitForFunction(() => {
      const state = JSON.parse(window.__chemsemaDebug?.state?.editorEngine?.stateJson?.() || "{}");
      const selection = state.selection || {};
      return !(selection.arrowObjects || selection.arrow_objects || []).length
        && !(selection.textObjects || selection.text_objects || []).length
        && !(selection.nodes || []).length
        && !(selection.bonds || []).length
        && !(selection.labelNodes || selection.label_nodes || []).length;
    });
    await waitForCanvasCursor(
      page,
      bracketCursorTargets.body.x,
      bracketCursorTargets.body.y,
      ["default"],
      "Unselected bracket body",
    );
    await waitForCanvasCursor(
      page,
      bracketCursorTargets.top.x,
      bracketCursorTargets.top.y,
      ["default"],
      "Unselected bracket endpoint",
    );
    const bracketDragBefore = await page.evaluate(({ sideObjectId, siblingObjectId }) => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const objects = (documentData.objects || []).flatMap((object) => visit(object, []));
      const side = objects.find((object) => object.id === sideObjectId);
      const sibling = objects.find((object) => object.id === siblingObjectId);
      return {
        sideTranslate: side?.transform?.translate || null,
        siblingTranslate: sibling?.transform?.translate || null,
      };
    }, bracketCursorTargets);
    await page.mouse.move(bracketCursorTargets.body.x, bracketCursorTargets.body.y);
    await page.mouse.down();
    await page.mouse.move(bracketCursorTargets.body.x + 18, bracketCursorTargets.body.y + 9, { steps: 4 });
    await waitForCanvasCursor(
      page,
      bracketCursorTargets.body.x + 18,
      bracketCursorTargets.body.y + 9,
      ["default"],
      "Unselected bracket drag",
    );
    await page.mouse.up();
    await page.waitForFunction((sideObjectId) => {
      const state = JSON.parse(window.__chemsemaDebug?.state?.editorEngine?.stateJson?.() || "{}");
      const selection = state.selection || {};
      const arrowObjects = selection.arrowObjects || selection.arrow_objects || [];
      return arrowObjects.length === 1 && arrowObjects[0] === sideObjectId;
    }, bracketCursorTargets.sideObjectId);
    const bracketDragAfter = await page.evaluate(({ sideObjectId, siblingObjectId }) => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const objects = (documentData.objects || []).flatMap((object) => visit(object, []));
      const side = objects.find((object) => object.id === sideObjectId);
      const sibling = objects.find((object) => object.id === siblingObjectId);
      return {
        sideTranslate: side?.transform?.translate || null,
        siblingTranslate: sibling?.transform?.translate || null,
      };
    }, bracketCursorTargets);
    assert(
      JSON.stringify(bracketDragAfter.sideTranslate) !== JSON.stringify(bracketDragBefore.sideTranslate),
      `Unselected bracket drag did not move the hit side: ${JSON.stringify({ bracketDragBefore, bracketDragAfter, bracketCursorTargets })}`,
    );
    assert(
      JSON.stringify(bracketDragAfter.siblingTranslate) === JSON.stringify(bracketDragBefore.siblingTranslate),
      `Unselected bracket drag moved the sibling side: ${JSON.stringify({ bracketDragBefore, bracketDragAfter, bracketCursorTargets })}`,
    );
  
    await page.close();
    assert(!errors.length, `Viewer console errors during cursor regression: ${errors.join("\n")}`);
  }

  return { verifyNativeTableToolWorkflow, verifyBondDrawing, verifyBondCreationUsesKernelLocalPreview, verifyElementEndpointPatchUpdatesConnectedBonds, verifyJunctionDragUsesBackendPrimitivePatch, verifyTransformedArrowRenderHitAndSelection, verifyCursorAnchoredWheelZoom, verifyQuickPaletteAndSelectDragRegression, verifyEndpointFeedbackRules, verifyGraphicObjectDragTracksPointerAndSelection, verifyCreationDragKeepsCanvasVisibleAfterToolSwitch, verifyDeleteToolTemporaryToolbarAndEmptyDocument, verifySelectedObjectSuppressesHover, verifyDragHandleCursors };
}
