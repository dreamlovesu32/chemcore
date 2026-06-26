import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const defaultLargeCdxml = `C:\\Users\\Dream\\OneDrive\\Desktop\\${"\u94af\u50ac\u5316-jjb.cdxml"}`;
const largeCdxml = process.env.CHEMCORE_INTERACTION_SMOKE_CDXML || defaultLargeCdxml;

function waitForPort(timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolvePort, reject) => {
    const attempt = () => {
      const socket = net.connect({ host, port }, () => {
        socket.end();
        resolvePort(true);
      });
      socket.on("error", () => {
        socket.destroy();
        if (Date.now() >= deadline) {
          reject(new Error(`Timed out waiting for ${host}:${port}`));
        } else {
          setTimeout(attempt, 100);
        }
      });
    };
    attempt();
  });
}

function portIsOpen() {
  return new Promise((resolvePort) => {
    const socket = net.connect({ host, port }, () => {
      socket.end();
      resolvePort(true);
    });
    socket.on("error", () => {
      socket.destroy();
      resolvePort(false);
    });
  });
}

async function ensureServer() {
  if (await portIsOpen()) {
    return null;
  }
  const child = spawn(process.execPath, ["scripts/desktop-dev-server.mjs"], {
    cwd: rootDir,
    stdio: "ignore",
    windowsHide: true,
  });
  await waitForPort();
  return child;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function openViewer(browser) {
  const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(`${baseUrl}?v=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => !!window.__chemcoreDebug, null, { timeout: 20000 });
  return { page, errors };
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
      hadPreview: !!document.querySelector('[data-role="preview-bond"]'),
      previewEndRadiusPx: previewEnd ? Number(previewEnd.getAttribute("r") || 0) * scale : 0,
    };
  });
  await page.mouse.up();
  await page.waitForTimeout(250);
  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    return {
      previewLeft: !!document.querySelector('[data-role^="preview-"]'),
      dragPreviewChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
      changed: !!command?.changed,
      bondTargets: command?.targets?.bonds?.length || command?.created?.bonds?.length || 0,
      hasRenderedBond: /data-bond-id=/.test(document.querySelector("#viewer-svg")?.outerHTML || ""),
    };
  });
  await page.close();
  assert(previewState.hadPreview, "Bond drag did not show a preview.");
  assert(
    Math.abs(previewState.previewEndRadiusPx - 1.5) < 0.25,
    `Bond preview endpoint radius was not unified: ${JSON.stringify(previewState)}`,
  );
  assert(!result.previewLeft && result.dragPreviewChildren === 0, `Bond preview remained after pointerup: ${JSON.stringify(result)}`);
  assert(result.changed && result.bondTargets > 0 && result.hasRenderedBond, "Bond drag did not commit a rendered bond.");
  assert(!errors.length, `Viewer console errors during bond drawing: ${errors.join("\n")}`);
}

async function visibleEndpointTarget(page) {
  return page.evaluate(() => {
    const doc = JSON.parse(window.__chemcoreDebug.state.editorEngine.documentJson?.() || "null")
      || window.__chemcoreDebug.document;
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
      const client = window.__chemcoreDebug.worldToClient(x, y);
      if (client) {
        return { x: client.x, y: client.y, nodeId: node.id };
      }
    }
    return null;
  });
}

async function interactionFeedbackState(page) {
  return page.evaluate(() => {
    const endpoint = document.querySelector('[data-role="hover-endpoint"]');
    const matrix = endpoint?.getScreenCTM?.();
    const scale = matrix ? Math.hypot(matrix.a, matrix.b) : 1;
    return {
      hoverEndpointCount: document.querySelectorAll('[data-role="hover-endpoint"]').length,
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
    Math.abs(bondHover.endpointRadiusPx - 1.5) < 0.25,
    `Endpoint hover radius was not unified: ${JSON.stringify(bondHover)}`,
  );

  for (const tool of ["arrow", "bracket", "symbol", "shape", "orbital", "templates"]) {
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

  await page.close();
  assert(!errors.length, `Viewer console errors during endpoint feedback rules: ${errors.join("\n")}`);
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
        objectCount: flatten(window.__chemcoreDebug.engineState.document.objects || [])
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
      const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      const objectIds = command?.targets?.objects?.length
        ? command.targets.objects
        : command?.created?.objects || [];
      return {
        changed: !!command?.changed,
        targets: command?.targets || null,
        created: command?.created || null,
        objectIds,
        objectCount: flatten(window.__chemcoreDebug.engineState.document.objects || [])
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
  await page.waitForFunction(
    ({ x: px, y: py, values }) => {
      const hit = document.elementFromPoint(px, py);
      const cursors = [
        hit ? getComputedStyle(hit).cursor : "",
        getComputedStyle(document.querySelector("#viewer-container")).cursor,
        getComputedStyle(document.querySelector("#viewer-svg")).cursor,
        getComputedStyle(document.querySelector(".canvas-pointer-shield")).cursor,
      ];
      return cursors.some((cursor) => values.includes(cursor));
    },
    { x, y, values: expected },
    { timeout: 1200 },
  );
  const actual = await page.evaluate(({ x: px, y: py }) => {
    const hit = document.elementFromPoint(px, py);
    return {
      hit: hit?.id || hit?.className || hit?.tagName || "",
      hitCursor: hit ? getComputedStyle(hit).cursor : "",
      containerCursor: getComputedStyle(document.querySelector("#viewer-container")).cursor,
      svgCursor: getComputedStyle(document.querySelector("#viewer-svg")).cursor,
      shieldCursor: getComputedStyle(document.querySelector(".canvas-pointer-shield")).cursor,
    };
  }, { x, y });
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
  await waitForCanvasCursor(
    page,
    bracketStart.x,
    bracketStart.y + 70,
    ["nwse-resize", "nesw-resize", "ew-resize", "ns-resize"],
    "Bracket resize handle",
  );

  await page.close();
  assert(!errors.length, `Viewer console errors during cursor regression: ${errors.join("\n")}`);
}

function largeFileTargetFinder() {
  const doc = window.__chemcoreDebug.document;
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
  const objectType = (object) => object?.type || object?.objectType || object?.object_type;
  const allObjects = (doc.objects || []).flatMap((candidate) => visit(candidate, []));
  const objectClientRect = (objectId) => {
    const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
      .filter((element) => !element.classList.contains("document-diagnostic-marker"));
    if (!elements.length) {
      return null;
    }
    const rects = elements.map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0);
    if (!rects.length) {
      return null;
    }
    const left = Math.min(...rects.map((rect) => rect.left));
    const top = Math.min(...rects.map((rect) => rect.top));
    const right = Math.max(...rects.map((rect) => rect.right));
    const bottom = Math.max(...rects.map((rect) => rect.bottom));
    return {
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
      centerX: (left + right) * 0.5,
      centerY: (top + bottom) * 0.5,
    };
  };
  const entries = [];
  const bondEntries = [];
  for (const object of allObjects) {
    if (objectType(object) !== "molecule") {
      continue;
    }
    const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
    const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
    if (!fragment?.nodes?.length) {
      continue;
    }
    const degree = new Map();
    for (const bond of fragment.bonds || []) {
      degree.set(bond.begin, (degree.get(bond.begin) || 0) + 1);
      degree.set(bond.end, (degree.get(bond.end) || 0) + 1);
    }
    const translate = object.transform?.translate || [0, 0];
    const nodePositions = new Map();
    for (const node of fragment.nodes || []) {
      if (!Array.isArray(node.position) || !degree.get(node.id)) {
        continue;
      }
      const x = Number(translate[0] || 0) + Number(node.position[0] || 0);
      const y = Number(translate[1] || 0) + Number(node.position[1] || 0);
      nodePositions.set(node.id, { x, y });
      const client = window.__chemcoreDebug.worldToClient(x, y);
      if (!client
        || client.x <= 80
        || client.x >= innerWidth - 80
        || client.y <= 120
        || client.y >= innerHeight - 80) {
        continue;
      }
      entries.push({
        id: node.id,
        x: client.x,
        y: client.y,
        worldX: x,
        worldY: y,
        objectId: object.id,
        label: node.label?.text || node.label?.sourceText || "",
        element: node.element || "",
        degree: degree.get(node.id) || 0,
      });
    }
    for (const bond of fragment.bonds || []) {
      const begin = nodePositions.get(bond.begin);
      const end = nodePositions.get(bond.end);
      if (!begin || !end) {
        continue;
      }
      const client = window.__chemcoreDebug.worldToClient(
        (begin.x + end.x) * 0.5,
        (begin.y + end.y) * 0.5,
      );
      if (!client
        || client.x <= 80
        || client.x >= innerWidth - 80
        || client.y <= 120
        || client.y >= innerHeight - 80) {
        continue;
      }
      bondEntries.push({
        id: bond.id,
        x: client.x,
        y: client.y,
      });
    }
  }
  const visibleObjectTarget = (type) => allObjects
    .filter((object) => objectType(object) === type && object.visible !== false)
    .map((object) => {
      const rect = objectClientRect(object.id);
      const bbox = object.payload?.bbox;
      const translate = object.transform?.translate || [0, 0];
      const boundsCenter = Array.isArray(bbox)
        ? window.__chemcoreDebug.worldToClient(
          Number(translate[0] || 0) + Number(bbox[0] || 0) + Number(bbox[2] || 0) * 0.5,
          Number(translate[1] || 0) + Number(bbox[1] || 0) + Number(bbox[3] || 0) * 0.5,
        )
        : null;
      if (!rect
        || (boundsCenter?.x ?? rect.centerX) <= 80
        || (boundsCenter?.x ?? rect.centerX) >= innerWidth - 80
        || (boundsCenter?.y ?? rect.centerY) <= 120
        || (boundsCenter?.y ?? rect.centerY) >= innerHeight - 80) {
        return null;
      }
      return {
        id: object.id,
        x: boundsCenter?.x ?? rect.centerX,
        y: boundsCenter?.y ?? rect.centerY,
        rect,
      };
    })
    .find(Boolean) || null;
  const bracket = allObjects
    .filter((object) => objectType(object) === "bracket" && object.visible !== false)
    .map((object) => {
      const rect = objectClientRect(object.id);
      const bbox = object.payload?.bbox;
      const translate = object.transform?.translate || [0, 0];
      if (!rect || !Array.isArray(bbox)) {
        return null;
      }
      const hover = window.__chemcoreDebug.worldToClient(
        Number(translate[0] || 0) + Number(bbox[0] || 0),
        Number(translate[1] || 0) + Number(bbox[1] || 0) + Number(bbox[3] || 0) * 0.5,
      );
      if (!hover
        || hover.x <= 80
        || hover.x >= innerWidth - 80
        || hover.y <= 120
        || hover.y >= innerHeight - 80) {
        return null;
      }
      return {
        id: object.id,
        x: hover.x,
        y: hover.y,
        rect,
      };
    })
    .find(Boolean) || null;
  const hover = [...document.querySelectorAll("[data-node-id]")]
    .map((element) => {
      const rect = element.getBoundingClientRect();
      return {
        id: element.getAttribute("data-node-id"),
        x: rect.x + rect.width / 2,
        y: rect.y + rect.height / 2,
        w: rect.width,
        h: rect.height,
        diagnostic: element.classList.contains("document-diagnostic-marker"),
      };
    })
    .filter((entry) => !entry.diagnostic)
    .filter((entry) => entry.w >= 3
      && entry.h >= 2
      && entry.x > 80
      && entry.x < innerWidth - 80
      && entry.y > 120
      && entry.y < innerHeight - 80)[0] || null;
  const bondTarget = [...document.querySelectorAll("[data-bond-id]")]
    .map((element) => {
      const rect = element.getBoundingClientRect();
      return {
        id: element.getAttribute("data-bond-id"),
        x: rect.x + rect.width / 2,
        y: rect.y + rect.height / 2,
        w: rect.width,
        h: rect.height,
      };
    })
    .filter((entry) => entry.id
      && entry.w >= 3
      && entry.h >= 3
      && entry.x > 80
      && entry.x < innerWidth - 80
      && entry.y > 120
      && entry.y < innerHeight - 80)[0] || null;
  const invalidDiagnostic = [...document.querySelectorAll(".document-diagnostic-marker[data-node-id]")]
    .map((marker) => {
      const id = marker.getAttribute("data-node-id");
      const nodeEntry = entries.find((entry) => entry.id === id);
      const anchor = [...document.querySelectorAll(`[data-node-id="${CSS.escape(id)}"]`)]
        .find((element) => element !== marker && !element.classList.contains("document-diagnostic-marker"));
      if (!anchor && !nodeEntry) {
        return null;
      }
      const markerRect = marker.getBoundingClientRect();
      const anchorRect = anchor?.getBoundingClientRect();
      return {
        id,
        x: nodeEntry?.x ?? (anchorRect.x + anchorRect.width / 2),
        y: nodeEntry?.y ?? (anchorRect.y + anchorRect.height / 2),
        markerX: markerRect.x + markerRect.width / 2,
        markerY: markerRect.y + markerRect.height / 2,
      };
    })
    .filter(Boolean)
    .find((entry) => entry.x > 80
      && entry.x < innerWidth - 80
      && entry.y > 120
      && entry.y < innerHeight - 80) || null;
  return {
    hover,
    bond: bondEntries[0] || bondTarget,
    label: entries.find((entry) => entry.label && entry.degree > 0) || null,
    atom: entries.find((entry) => !entry.label && (!entry.element || entry.element === "C") && entry.degree > 0) || null,
    bracket,
    textObject: visibleObjectTarget("text"),
    invalidDiagnostic,
  };
}

async function verifyLargeDragTarget(page, target, kind) {
  await page.keyboard.press("Escape").catch(() => {});
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    window.__chemcoreDebug.clearActiveSelectionGesture?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.locator('button[data-tool="select"]').click();
  await page.mouse.move(target.x, target.y);
  await page.waitForTimeout(180);
  await page.mouse.move(target.x, target.y);
  const beforeNodePosition = await page.evaluate((nodeId) => {
    const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-node-id="${CSS.escape(nodeId)}"]`)]
      .filter((element) => getComputedStyle(element).visibility !== "hidden")
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0);
    if (!rects.length) {
      return null;
    }
    const left = Math.min(...rects.map((rect) => rect.left));
    const top = Math.min(...rects.map((rect) => rect.top));
    const right = Math.max(...rects.map((rect) => rect.right));
    const bottom = Math.max(...rects.map((rect) => rect.bottom));
    return { x: (left + right) * 0.5, y: (top + bottom) * 0.5 };
  }, target.id);
  await page.mouse.down();
  await page.mouse.move(target.x + 24, target.y + 12, { steps: 6 });
  const backendDomMatches = (nodeId) => {
    const doc = window.__chemcoreDebug.document;
    const connectedBonds = new Set();
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
      const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
      const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
      for (const bond of fragment?.bonds || []) {
        if (bond.begin === nodeId || bond.end === nodeId) {
          connectedBonds.add(bond.id);
        }
      }
    }
    const renderList = JSON.parse(window.__chemcoreDebug.state.editorEngine.renderTargetsJson(JSON.stringify({
      nodes: [nodeId],
      bonds: [...connectedBonds],
    })));
    const backendCount = renderList
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
    const domCount = [...document.querySelectorAll(`[data-layer="document-content"] ${selectors.join(",")}`)].length;
    return {
      connectedBonds: [...connectedBonds],
      backendCount,
      domCount,
      matches: backendCount > 0 && backendCount === domCount,
      partialChildren: document.querySelector('[data-layer="document-partial-bond-preview"]')?.childElementCount || 0,
      gesture: window.__chemcoreDebug.activeSelectionGesture || null,
    };
  };
  await page.evaluate((source) => {
    window.__viewerSmokeBackendDomMatches = eval(`(${source})`);
  }, backendDomMatches.toString());
  try {
    await page.waitForFunction((nodeId) => {
      return (window.__viewerSmokeBackendDomMatches || (() => ({ matches: false })))(nodeId).matches;
    }, target.id, { timeout: 5000 });
  } catch (error) {
    const diagnostics = await page.evaluate(
      ([nodeId, source]) => {
        window.__viewerSmokeBackendDomMatches = eval(`(${source})`);
        return window.__viewerSmokeBackendDomMatches(nodeId);
      },
      [target.id, backendDomMatches.toString()],
    );
    throw new Error(`${kind} backend DOM did not match: ${JSON.stringify(diagnostics).slice(0, 1600)}`);
  }
  const duringNodePosition = await page.evaluate(([nodeId, before]) => {
    const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-node-id="${CSS.escape(nodeId)}"]`)]
      .filter((element) => getComputedStyle(element).visibility !== "hidden");
    const rects = elements
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0);
    if (!rects.length) {
      return { count: 0, oldVisibleCount: 0, x: null, y: null };
    }
    const left = Math.min(...rects.map((rect) => rect.left));
    const top = Math.min(...rects.map((rect) => rect.top));
    const right = Math.max(...rects.map((rect) => rect.right));
    const bottom = Math.max(...rects.map((rect) => rect.bottom));
    const oldVisibleCount = before
      ? rects.filter((rect) => {
        const cx = rect.left + rect.width * 0.5;
        const cy = rect.top + rect.height * 0.5;
        return Math.hypot(cx - before.x, cy - before.y) < 3;
      }).length
      : 0;
    return {
      count: rects.length,
      oldVisibleCount,
      x: (left + right) * 0.5,
      y: (top + bottom) * 0.5,
    };
  }, [target.id, beforeNodePosition]);
  const previewMoved = beforeNodePosition && duringNodePosition.x != null
    ? Math.hypot(duringNodePosition.x - beforeNodePosition.x, duringNodePosition.y - beforeNodePosition.y)
    : 0;
  assert(
    previewMoved > 6,
    `${kind} did not visually follow drag before mouseup: ${JSON.stringify({ target, beforeNodePosition, duringNodePosition, previewMoved })}`,
  );
  assert(
    duringNodePosition.oldVisibleCount === 0,
    `${kind} left a visible stale node primitive at the drag origin: ${JSON.stringify({ target, beforeNodePosition, duringNodePosition })}`,
  );
  const during = await page.evaluate(() => {
    const partial = document.querySelector('[data-layer="document-partial-bond-preview"]');
    return {
      partialChildren: partial?.childElementCount || 0,
      transformed: document.querySelectorAll(".is-preview-transforming").length,
    };
  });
  await page.mouse.up();
  await page.waitForTimeout(250);
  const after = await page.evaluate(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    return {
      previews: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
      partial: !!document.querySelector('[data-layer="document-partial-bond-preview"]'),
      transformed: document.querySelectorAll(".is-preview-transforming").length,
      gesture: window.__chemcoreDebug.activeSelectionGesture || null,
    };
  });
  assert(during.partialChildren === 0, `${kind} drag used front-end partial bond preview.`);
  assert(!after.partial, `${kind} drag left partial bond preview behind.`);
  assert(after.transformed === 0, `${kind} drag left transformed document nodes behind.`);
  assert(after.previews === 0, `${kind} drag left preview overlay behind.`);
  assert(after.gesture === null, `${kind} drag left an active selection gesture behind.`);
  const commandTargets = await page.evaluate((nodeId) => {
    const raw = window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null";
    const result = JSON.parse(raw);
    const targetNodes = new Set([
      ...(result?.targets?.nodes || []),
      ...(result?.updated?.nodes || []),
      ...(result?.created?.nodes || []),
      ...(result?.deleted?.nodes || []),
    ]);
    return {
      changed: !!result?.changed,
      nodeIncluded: targetNodes.has(nodeId),
      nodes: [...targetNodes].slice(0, 20),
      command: result?.command || null,
    };
  }, target.id);
  assert(
    commandTargets.changed && commandTargets.nodeIncluded,
    `${kind} drag commit did not report the moved node for incremental rendering: ${JSON.stringify({ target, commandTargets })}`,
  );
}

async function verifyLargeRegionSelectionDoesNotDragGroup(page, target) {
  await page.keyboard.press("Escape").catch(() => {});
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    window.__chemcoreDebug.clearActiveSelectionGesture?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.locator('button[data-tool="select"]').click();
  const selected = await page.evaluate((regionTarget) => {
    const engine = window.__chemcoreDebug.state.editorEngine;
    const doc = JSON.parse(engine.documentJson());
    const objectTypeById = new Map();
    const visit = (object) => {
      if (!object) {
        return;
      }
      objectTypeById.set(object.id, object.type || object.objectType || object.object_type);
      for (const child of object.children || []) {
        visit(child);
      }
    };
    for (const object of doc.objects || []) {
      visit(object);
    }
    engine.selectInRect(
      regionTarget.worldX - 24,
      regionTarget.worldY - 24,
      regionTarget.worldX + 24,
      regionTarget.worldY + 24,
      false,
    );
    const selection = JSON.parse(engine.stateJson()).selection || {};
    return {
      selection,
      selectedGroups: (selection.arrowObjects || [])
        .filter((objectId) => objectTypeById.get(objectId) === "group"),
    };
  }, target);
  assert(
    selected.selectedGroups.length === 0,
    `Large CDXML region selection captured parent groups: ${JSON.stringify({ target, selected })}`,
  );
  assert(
    (selected.selection.nodes || []).includes(target.id)
      || (selected.selection.labelNodes || []).includes(target.id),
    `Large CDXML region selection did not include target node: ${JSON.stringify({ target, selected })}`,
  );

  const moved = await page.evaluate((regionTarget) => {
    const engine = window.__chemcoreDebug.state.editorEngine;
    const nodePosition = (doc, nodeId) => {
      for (const resource of Object.values(doc.resources || {})) {
        for (const node of resource?.data?.nodes || []) {
          if (node.id === nodeId) {
            return node.position;
          }
        }
      }
      return null;
    };
    const groupTransforms = (doc) => {
      const out = {};
      const visit = (object) => {
        if (!object) {
          return;
        }
        const type = object.type || object.objectType || object.object_type;
        if (type === "group") {
          out[object.id] = object.transform?.translate || [0, 0];
        }
        for (const child of object.children || []) {
          visit(child);
        }
      };
      for (const object of doc.objects || []) {
        visit(object);
      }
      return out;
    };
    const beforeDoc = JSON.parse(engine.documentJson());
    const beforeNode = nodePosition(beforeDoc, regionTarget.id);
    const beforeGroups = groupTransforms(beforeDoc);
    const began = engine.beginSelectionMove(regionTarget.worldX, regionTarget.worldY, false, false);
    const updated = engine.updateSelectionMove(regionTarget.worldX + 10, regionTarget.worldY, false);
    const finished = engine.finishSelectionMove(regionTarget.worldX + 10, regionTarget.worldY, false);
    const afterDoc = JSON.parse(engine.documentJson());
    const afterNode = nodePosition(afterDoc, regionTarget.id);
    const afterGroups = groupTransforms(afterDoc);
    const command = JSON.parse(engine.lastCommandResultJson?.() || "null");
    return { began, updated, finished, beforeNode, afterNode, beforeGroups, afterGroups, command };
  }, target);
  const dx = (moved.afterNode?.[0] ?? NaN) - (moved.beforeNode?.[0] ?? NaN);
  assert(
    moved.began && moved.updated && moved.finished && Math.abs(dx - 10) < 0.01,
    `Large CDXML region-selected molecule node did not move correctly: ${JSON.stringify({ target, moved })}`,
  );
  assert(
    JSON.stringify(moved.beforeGroups) === JSON.stringify(moved.afterGroups),
    `Large CDXML region-selected molecule moved parent group transforms: ${JSON.stringify({ target, moved })}`,
  );
  assert(
    (moved.command?.targets?.nodes || []).includes(target.id),
    `Large CDXML region-selected molecule drag did not report moved node target: ${JSON.stringify({ target, moved })}`,
  );
  await page.evaluate(async () => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    await window.__chemcoreDebug.syncDocument?.();
  });
}

function selectionItemCount(selection) {
  if (!selection) {
    return 0;
  }
  return (selection.textObjects?.length || 0)
    + (selection.arrowObjects?.length || 0)
    + (selection.labelNodes?.length || 0)
    + (selection.nodes?.length || 0)
    + (selection.bonds?.length || 0);
}

async function verifyLargeFileSelectionLatency(page, target) {
  await page.locator('button[data-tool="select"]').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
  const blank = { x: 1180, y: 820 };

  await page.mouse.move(target.x, target.y);
  let stepStarted = Date.now();
  await page.mouse.down();
  const selectDownMs = Date.now() - stepStarted;
  stepStarted = Date.now();
  await page.mouse.up();
  const selectUpMs = Date.now() - stepStarted;
  await page.waitForFunction(() => {
    const selection = window.__chemcoreDebug.engineState?.selection;
    const count = (selection?.textObjects?.length || 0)
      + (selection?.arrowObjects?.length || 0)
      + (selection?.labelNodes?.length || 0)
      + (selection?.nodes?.length || 0)
      + (selection?.bonds?.length || 0);
    return count > 0 && (document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0) > 0;
  }, null, { timeout: 1000 });
  const selected = await page.evaluate(() => ({
    overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
    selection: window.__chemcoreDebug.engineState?.selection || null,
  }));
  assert(selectionItemCount(selected.selection) > 0 && selected.overlayChildren > 0, `Large CDXML selection box did not appear: ${JSON.stringify(selected)}`);
  assert(
    selectDownMs + selectUpMs < 500,
    `Large CDXML selection box appeared too slowly: ${JSON.stringify({ selectDownMs, selectUpMs, selected })}`,
  );

  await page.mouse.move(blank.x, blank.y);
  stepStarted = Date.now();
  await page.mouse.down();
  const clearDownMs = Date.now() - stepStarted;
  stepStarted = Date.now();
  await page.mouse.up();
  const clearUpMs = Date.now() - stepStarted;
  await page.waitForFunction(() => {
    const selection = window.__chemcoreDebug.engineState?.selection;
    const count = (selection?.textObjects?.length || 0)
      + (selection?.arrowObjects?.length || 0)
      + (selection?.labelNodes?.length || 0)
      + (selection?.nodes?.length || 0)
      + (selection?.bonds?.length || 0);
    return count === 0 && (document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0) === 0;
  }, null, { timeout: 1000 });
  const cleared = await page.evaluate(() => ({
    overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
    selection: window.__chemcoreDebug.engineState?.selection || null,
  }));
  assert(selectionItemCount(cleared.selection) === 0 && cleared.overlayChildren === 0, `Large CDXML blank click did not clear selection: ${JSON.stringify(cleared)}`);
  assert(
    clearDownMs + clearUpMs < 350,
    `Large CDXML blank click cleared selection too slowly: ${JSON.stringify({ clearDownMs, clearUpMs, cleared })}`,
  );
  await page.keyboard.press("Escape").catch(() => {});
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    window.__chemcoreDebug.clearActiveSelectionGesture?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.mouse.move(blank.x, blank.y);
  await page.waitForTimeout(30);
}

async function verifyDiagnosticMarkerHidesDuringDrag(page, target, dragTarget = target) {
  if (!target || !dragTarget) {
    return;
  }
  await page.keyboard.press("Escape").catch(() => {});
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    window.__chemcoreDebug.clearActiveSelectionGesture?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.locator('button[data-tool="select"]').click();
  const before = await page.evaluate((nodeId) => {
    const markers = [...document.querySelectorAll(`.document-diagnostic-marker[data-node-id="${CSS.escape(nodeId)}"]`)];
    return {
      count: markers.length,
      totalDiagnostics: document.querySelectorAll(".document-diagnostic-marker").length,
      visibleDiagnostics: [...document.querySelectorAll(".document-diagnostic-marker")]
        .filter((element) => getComputedStyle(element).visibility !== "hidden").length,
    };
  }, target.id);
  if (!before.count) {
    return;
  }
  await page.mouse.click(dragTarget.x, dragTarget.y);
  await page.waitForFunction((id) => {
    const selection = window.__chemcoreDebug.engineState?.selection || window.__chemcoreDebug.getEngineState?.()?.selection || {};
    return (selection.nodes || []).includes(id)
      || (selection.labelNodes || []).includes(id)
      || (selection.textObjects || []).includes(id)
      || (selection.arrowObjects || []).includes(id);
  }, dragTarget.id, { timeout: 1200 });
  await page.mouse.move(dragTarget.x, dragTarget.y);
  await page.waitForTimeout(180);
  await page.mouse.down();
  await page.mouse.move(dragTarget.x + 42, dragTarget.y + 18, { steps: 6 });
  const during = await page.evaluate((nodeId) => {
    const markers = [...document.querySelectorAll(`.document-diagnostic-marker[data-node-id="${CSS.escape(nodeId)}"]`)];
    return {
      count: markers.length,
      totalDiagnostics: document.querySelectorAll(".document-diagnostic-marker").length,
      visibleDiagnostics: [...document.querySelectorAll(".document-diagnostic-marker")]
        .filter((element) => getComputedStyle(element).visibility !== "hidden").length,
      previewDiagnostics: document.querySelectorAll('[data-layer="document-partial-bond-preview"] .document-diagnostic-marker').length,
      gesture: window.__chemcoreDebug.getActiveSelectionGesture?.() || null,
      selection: window.__chemcoreDebug.engineState?.selection || window.__chemcoreDebug.getEngineState?.()?.selection || null,
      previewStats: window.__chemcoreDebug.backendMovePreviewStats?.last || null,
    };
  }, target.id);
  await page.mouse.up();
  await page.waitForFunction(() => [...document.querySelectorAll(".document-diagnostic-marker")]
    .some((element) => getComputedStyle(element).visibility !== "hidden"), null, { timeout: 1000 });
  const after = await page.evaluate(() => ({
    totalDiagnostics: document.querySelectorAll(".document-diagnostic-marker").length,
    visibleDiagnostics: [...document.querySelectorAll(".document-diagnostic-marker")]
      .filter((element) => getComputedStyle(element).visibility !== "hidden").length,
  }));
  assert(during.count === before.count, `Diagnostic marker duplicated during drag: ${JSON.stringify({ before, during, target })}`);
  assert(during.totalDiagnostics <= before.totalDiagnostics + 2, `Diagnostic marker count ballooned during drag: ${JSON.stringify({ before, during, target })}`);
  assert(during.previewDiagnostics === 0, `Diagnostic markers were drawn into partial preview layer: ${JSON.stringify({ before, during, target })}`);
  assert(during.visibleDiagnostics === 0, `Diagnostic markers remained visible during drag: ${JSON.stringify({ before, during, target, dragTarget })}`);
  assert(after.totalDiagnostics <= before.totalDiagnostics + 2 && after.visibleDiagnostics > 0, `Diagnostic markers did not restore after drag: ${JSON.stringify({ before, during, after, target })}`);
}

async function verifyBracketHoverFocus(page, target) {
  if (!target) {
    return;
  }
  await page.keyboard.press("Escape").catch(() => {});
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    window.__chemcoreDebug.clearActiveSelectionGesture?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.locator('button[data-tool="select"]').click();
  await page.mouse.move(target.x, target.y);
  const started = Date.now();
  await page.waitForFunction(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    return (overlay?.querySelectorAll('[data-role="hover-shape-handle"]').length || 0) > 0;
  }, null, { timeout: 800 });
  const elapsed = Date.now() - started;
  const debug = await page.evaluate(() => ({
    fastHover: window.__chemcoreDebug.fastSelectHoverStats || null,
    overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
    handles: document.querySelectorAll('[data-role="hover-shape-handle"]').length,
    handleStyle: (() => {
      const handle = document.querySelector('.editor-object-control-handle[data-role="hover-shape-handle"]');
      if (!handle) {
        return null;
      }
      const matrix = handle.ownerSVGElement?.getScreenCTM?.();
      const scale = Math.max(Math.abs(matrix?.a || 1), Math.abs(matrix?.d || 1));
      const style = getComputedStyle(handle);
      return {
        tagName: handle.tagName.toLowerCase(),
        radiusPx: Number(handle.getAttribute("r") || 0) * scale,
        fill: style.fill,
      };
    })(),
  }));
  assert(elapsed < 350, `Large CDXML bracket hover focus was delayed: ${JSON.stringify({ elapsed, target, debug })}`);
  assert(
    debug.handleStyle?.tagName === "circle"
      && Math.abs(debug.handleStyle.radiusPx - 1.5) < 0.2
      && (debug.handleStyle.fill === "none" || debug.handleStyle.fill === "rgba(0, 0, 0, 0)"),
    `Large CDXML bracket hover control handle style was not unified: ${JSON.stringify({ target, debug })}`,
  );
}

async function verifyMixedObjectFollowsStructureDrag(page, structureTarget, objectTarget, kind) {
  if (!structureTarget || !objectTarget) {
    return;
  }
  await page.keyboard.press("Escape").catch(() => {});
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    window.__chemcoreDebug.clearActiveSelectionGesture?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.locator('button[data-tool="select"]').click();
  await page.mouse.click(structureTarget.x, structureTarget.y);
  await page.keyboard.down("Shift");
  await page.mouse.click(objectTarget.x, objectTarget.y);
  await page.keyboard.up("Shift");
  try {
    await page.waitForFunction((objectId) => {
      const selection = window.__chemcoreDebug.engineState?.selection || {};
      const hasStructure = (selection.nodes || []).length > 0
        || (selection.labelNodes || []).length > 0
        || (selection.bonds || []).length > 0;
      return hasStructure
        && ((selection.textObjects || []).includes(objectId) || (selection.arrowObjects || []).includes(objectId));
    }, objectTarget.id, { timeout: 1200 });
  } catch (error) {
    const diagnostics = await page.evaluate((target) => {
      const matrix = document.querySelector("#viewer-svg")?.getScreenCTM?.();
      const world = matrix
        ? new DOMPoint(target.x, target.y).matrixTransform(matrix.inverse())
        : null;
      return {
        selection: window.__chemcoreDebug.engineState?.selection || window.__chemcoreDebug.getEngineState?.()?.selection || null,
        world: world ? { x: world.x, y: world.y } : null,
        contextHit: world ? window.__chemcoreDebug.state.editorEngine.contextHitTestJson?.(world.x, world.y) : null,
        object: window.__chemcoreDebug.document?.objects
          ?.flatMap((object) => {
            const out = [];
            const visit = (candidate) => {
              if (!candidate) return;
              out.push(candidate);
              for (const child of candidate.children || []) visit(child);
            };
            visit(object);
            return out;
          })
          .find((object) => object.id === target.id) || null,
      };
    }, objectTarget);
    throw new Error(`${kind} mixed selection did not include both targets: ${JSON.stringify({ ...diagnostics, structureTarget, objectTarget })}`);
  }
  const dragPoint = objectTarget;
  const before = await page.evaluate((objectId) => {
    const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
      .filter((element) => !element.classList.contains("document-diagnostic-marker"))
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0);
    const left = Math.min(...rects.map((rect) => rect.left));
    const top = Math.min(...rects.map((rect) => rect.top));
    const right = Math.max(...rects.map((rect) => rect.right));
    const bottom = Math.max(...rects.map((rect) => rect.bottom));
    return {
      count: rects.length,
      x: (left + right) * 0.5,
      y: (top + bottom) * 0.5,
      centers: rects.slice(0, 200).map((rect) => ({
        x: rect.left + rect.width * 0.5,
        y: rect.top + rect.height * 0.5,
      })),
    };
  }, objectTarget.id);
  await page.mouse.move(dragPoint.x, dragPoint.y);
  await page.mouse.down();
  await page.mouse.move(dragPoint.x + 42, dragPoint.y + 18, { steps: 6 });
  try {
    await page.waitForFunction((objectId) => {
      const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .filter((element) => !element.classList.contains("document-diagnostic-marker"));
      return elements.some((element) => element.classList.contains("is-preview-transforming"));
    }, objectTarget.id, { timeout: 700 });
  } catch (error) {
    const diagnostics = await page.evaluate((objectId) => ({
      gesture: window.__chemcoreDebug.getActiveSelectionGesture?.() || null,
      selection: window.__chemcoreDebug.engineState?.selection || window.__chemcoreDebug.getEngineState?.()?.selection || null,
      previewStats: window.__chemcoreDebug.backendMovePreviewStats?.last || null,
      schedulerStats: window.__chemcoreDebug.backendPreviewSchedulerStats || null,
      elements: [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .map((element) => ({
          tag: element.tagName,
          classes: element.getAttribute("class"),
          transform: element.getAttribute("transform"),
          styleTransform: element.style.transform,
        })),
    }), objectTarget.id);
    throw new Error(`${kind} mixed drag did not apply preview transform: ${JSON.stringify({ diagnostics, structureTarget, dragPoint, objectTarget })}`);
  }
  const during = await page.evaluate(([objectId, beforeCenters]) => {
    const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
      .filter((element) => !element.classList.contains("document-diagnostic-marker"))
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 && rect.height > 0);
    const left = Math.min(...rects.map((rect) => rect.left));
    const top = Math.min(...rects.map((rect) => rect.top));
    const right = Math.max(...rects.map((rect) => rect.right));
    const bottom = Math.max(...rects.map((rect) => rect.bottom));
    const staleCenters = rects.filter((rect) => {
      const cx = rect.left + rect.width * 0.5;
      const cy = rect.top + rect.height * 0.5;
      return (beforeCenters || []).some((before) => Math.hypot(cx - before.x, cy - before.y) < 2);
    }).length;
    return {
      count: rects.length,
      x: (left + right) * 0.5,
      y: (top + bottom) * 0.5,
      staleCenters,
      transforming: [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .filter((element) => element.classList.contains("is-preview-transforming")).length,
    };
  }, [objectTarget.id, before.centers]);
  await page.mouse.up();
  await page.waitForTimeout(150);
  const moved = Math.hypot(during.x - before.x, during.y - before.y);
  assert(before.count > 0 && during.count > 0, `${kind} mixed drag target was not rendered: ${JSON.stringify({ before, during, objectTarget })}`);
  assert(moved > 12, `${kind} did not follow mixed molecule drag preview: ${JSON.stringify({ before, during, moved, structureTarget, objectTarget })}`);
  assert(during.transforming > 0, `${kind} mixed drag did not use object preview transform: ${JSON.stringify({ before, during, objectTarget })}`);
  assert(during.staleCenters === 0, `${kind} left stale object primitives at the drag origin: ${JSON.stringify({ before, during, structureTarget, objectTarget })}`);
}

async function resetViewerUi(page) {
  await page.keyboard.press("Escape").catch(() => {});
  await page.keyboard.press("Escape").catch(() => {});
  await page.waitForFunction(() => !window.__chemcoreDebug?.activeTextEditor, null, { timeout: 800 }).catch(() => {});
  await page.waitForTimeout(30);
}

async function measureCommitLatency(page, label, action, predicate, predicateArg = null, thresholdMs = 350) {
  const started = await page.evaluate(() => performance.now());
  const actionStarted = Date.now();
  await action();
  const actionMs = Date.now() - actionStarted;
  const waitStarted = Date.now();
  await page.waitForFunction(predicate, predicateArg, { timeout: 1500 });
  const waitMs = Date.now() - waitStarted;
  const elapsed = await page.evaluate((start) => performance.now() - start, started);
  if (elapsed >= thresholdMs) {
    const diagnostics = await page.evaluate((start) => ({
      measureStartedAt: start,
      measureEndedAt: performance.now(),
      commitTiming: window.__chemcoreDebug?.creationCommitStats?.last || null,
      lastCommandResult: JSON.parse(window.__chemcoreDebug?.state?.editorEngine?.lastCommandResultJson?.() || "null"),
    }), started);
    assert(false, `${label} committed too slowly: ${elapsed.toFixed(1)}ms (action=${actionMs}ms wait=${waitMs}ms) ${JSON.stringify(diagnostics)}`);
  }
  return elapsed;
}

async function verifyLargeFileCommitLatency(page) {
  const box = await page.locator("#viewer-container").boundingBox();
  const bracketStart = { x: box.x + box.width - 360, y: box.y + box.height - 330 };
  const bracketEnd = { x: bracketStart.x + 130, y: bracketStart.y + 120 };
  await resetViewerUi(page);
  await page.locator('button[data-tool="bracket"]').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
  await page.evaluate(() => {
    const engine = window.__chemcoreDebug.state.editorEngine;
    window.__viewerSmokeEngineTimings = [];
    for (const name of ["pointerMove", "interactionRenderListJson"]) {
      const original = engine?.[name];
      if (typeof original !== "function" || original.__viewerSmokeWrapped) {
        continue;
      }
      const wrapped = function (...args) {
        const start = performance.now();
        const result = original.apply(this, args);
        window.__viewerSmokeEngineTimings.push({
          name,
          ms: performance.now() - start,
        });
        return result;
      };
      wrapped.__viewerSmokeWrapped = true;
      engine[name] = wrapped;
    }
  });
  const bracketBefore = await page.evaluate(() => ({
    activeTool: window.__chemcoreDebug.engineState?.tool?.activeTool
      || window.__chemcoreDebug.engineState?.tool?.active_tool
      || null,
    selection: window.__chemcoreDebug.engineState?.selection || null,
    activeGesture: window.__chemcoreDebug.activeSelectionGesture || null,
    overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
    documentChildren: document.querySelector('[data-layer="document-content"]')?.childElementCount || 0,
    documentPointerEvents: document.querySelector('[data-layer="document-content"]')?.getAttribute("pointer-events") || getComputedStyle(document.querySelector('[data-layer="document-content"]')).pointerEvents,
    totalSvgElements: document.querySelectorAll("#viewer-svg *").length,
  }));
  const bracketMs = await measureCommitLatency(
    page,
    "Large CDXML bracket label editor",
    async () => {
      let stepStarted = Date.now();
      await page.mouse.move(bracketStart.x, bracketStart.y);
      const moveMs = Date.now() - stepStarted;
      stepStarted = Date.now();
      await page.mouse.down();
      const shieldAfterDown = await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.className || "");
      const downMs = Date.now() - stepStarted;
      stepStarted = Date.now();
      await page.mouse.move(bracketEnd.x, bracketEnd.y);
      const dragMs = Date.now() - stepStarted;
      stepStarted = Date.now();
      await page.mouse.up();
      const upMs = Date.now() - stepStarted;
      await page.evaluate((timing) => {
        window.__viewerSmokeBracketTiming = timing;
      }, { moveMs, downMs, dragMs, upMs, shieldAfterDown });
    },
    () => !!window.__chemcoreDebug.activeTextEditor?.bracketLabelObjectId,
    null,
    60000,
  );
  const bracketTiming = await page.evaluate(() => window.__viewerSmokeBracketTiming || null);
  const engineTimings = await page.evaluate(() => window.__viewerSmokeEngineTimings || []);
  const bracketActiveMs = (bracketTiming?.downMs || 0) + (bracketTiming?.dragMs || 0) + (bracketTiming?.upMs || 0);
  assert(bracketActiveMs < 1500, `Large CDXML bracket label editor committed too slowly: ${bracketMs.toFixed(1)}ms ${JSON.stringify({ bracketTiming, bracketActiveMs, bracketBefore, engineTimings: engineTimings.slice(-20) })}`);

  await resetViewerUi(page);
  await page.locator('button[data-tool="symbol"]').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
  const symbolPoint = { x: bracketStart.x - 70, y: bracketStart.y + 35 };
  const symbolMs = await measureCommitLatency(
    page,
    "Large CDXML charge symbol",
    async () => {
      await page.mouse.click(symbolPoint.x, symbolPoint.y);
    },
    () => {
      const result = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      const objectId = [
        ...(result?.targets?.objects || []),
        ...(result?.created?.objects || []),
        ...(result?.updated?.objects || []),
      ].find((id) => String(id || "").startsWith("obj_symbol")) || "";
      return result?.changed
        && objectId
        && document.querySelectorAll(`[data-object-id="${CSS.escape(objectId)}"]`).length > 0;
    },
  );

  await resetViewerUi(page);
  await page.locator('button[data-tool="bond"]').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
  const bondStart = { x: bracketStart.x - 160, y: bracketStart.y + 170 };
  const bondEnd = { x: bondStart.x + 115, y: bondStart.y };
  const bondMs = await measureCommitLatency(
    page,
    "Large CDXML bond hover cleanup",
    async () => {
      await page.mouse.move(bondStart.x, bondStart.y);
      await page.mouse.down();
      await page.mouse.move(bondEnd.x, bondEnd.y);
      await page.mouse.up();
    },
    () => {
      const overlay = document.querySelector('[data-layer="editor-overlay"]');
      return !overlay?.querySelector('[data-role^="preview-"], [data-role^="hover-"]');
    },
    null,
    3000,
  );

  await resetViewerUi(page);
  await page.locator('button[data-tool="orbital"]').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
  const orbitalStart = { x: bondStart.x + 210, y: bondStart.y + 70 };
  const orbitalEnd = { x: orbitalStart.x + 90, y: orbitalStart.y + 85 };
  await page.mouse.move(orbitalStart.x, orbitalStart.y);
  await page.mouse.down();
  await page.mouse.move(orbitalEnd.x, orbitalEnd.y, { steps: 6 });
  await page.mouse.up();
  await page.waitForFunction(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    return !!(command?.targets?.objects?.[0] || command?.created?.objects?.[0]);
  }, null, { timeout: 1500 });
  const orbitalObjectId = await page.evaluate(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    return command?.targets?.objects?.[0] || command?.created?.objects?.[0] || "";
  });
  await page.locator('button[data-tool="select"]').click();
  await page.waitForTimeout(160);
  const orbitalSelectState = await page.evaluate((objectId) => ({
    objectDomCount: document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`).length,
    selectionCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="selection-"]').length,
    hoverCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="hover-"]').length,
    previewCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="preview-"]').length,
  }), orbitalObjectId);
  assert(orbitalSelectState.objectDomCount > 0, `Large CDXML orbital disappeared after switching to select: ${JSON.stringify({ orbitalObjectId, orbitalSelectState })}`);
  assert(
    orbitalSelectState.hoverCount === 0 && orbitalSelectState.previewCount === 0,
    `Large CDXML orbital switch to select left hover/preview overlay: ${JSON.stringify({ orbitalObjectId, orbitalSelectState })}`,
  );

  return { bracketMs, symbolMs, bondMs };
}

async function verifyLargeFileHoverAndDrag(browser) {
  if (!existsSync(largeCdxml)) {
    console.log(`[viewer-interaction-smoke] skipping large-file hover; missing ${largeCdxml}`);
    return;
  }
  const { page, errors } = await openViewer(browser);
  await page.locator('input[type="file"]').setInputFiles(largeCdxml);
  await page.waitForFunction(() => (window.__chemcoreDebug?.document?.objects?.length || 0) > 0, null, {
    timeout: 60000,
  });
  await page.locator('button[data-tool="select"]').click();
  let targets = await page.evaluate(largeFileTargetFinder);
  assert(targets.hover, "Large CDXML did not expose a visible hover target.");
  assert(targets.label, "Large CDXML did not expose a draggable label node target.");
  assert(targets.atom, "Large CDXML did not expose a draggable atom node target.");
  assert(targets.bracket, "Large CDXML did not expose a visible bracket target.");

  await page.mouse.move(targets.hover.x, targets.hover.y);
  await page.waitForTimeout(250);
  const hover = await page.evaluate(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    return overlay?.querySelectorAll('[data-role^="hover-"]').length || 0;
  });
  assert(hover > 0, "Large CDXML select hover did not render a hover overlay.");

  await verifyLargeFileSelectionLatency(page, targets.hover);
  await verifyDiagnosticMarkerHidesDuringDrag(page, targets.invalidDiagnostic, targets.textObject || targets.bracket || targets.label || targets.atom);
  targets = await page.evaluate(largeFileTargetFinder);
  await verifyBracketHoverFocus(page, targets.bracket);
  await verifyLargeRegionSelectionDoesNotDragGroup(page, targets.atom);
  targets = await page.evaluate(largeFileTargetFinder);
  await verifyLargeDragTarget(page, targets.label, "Label");
  targets = await page.evaluate(largeFileTargetFinder);
  await verifyMixedObjectFollowsStructureDrag(page, targets.atom || targets.label, targets.textObject || targets.bracket, "Large CDXML text/bracket");
  await page.close();
  const { page: latencyPage, errors: latencyErrors } = await openViewer(browser);
  await latencyPage.locator('input[type="file"]').setInputFiles(largeCdxml);
  await latencyPage.waitForFunction(() => (window.__chemcoreDebug?.document?.objects?.length || 0) > 0, null, {
    timeout: 60000,
  });
  const latency = await verifyLargeFileCommitLatency(latencyPage);
  await latencyPage.close();
  console.log(`[viewer-interaction-smoke] large commit latency bracket=${latency.bracketMs.toFixed(1)}ms symbol=${latency.symbolMs.toFixed(1)}ms bond=${latency.bondMs.toFixed(1)}ms`);
  assert(!errors.length && !latencyErrors.length, `Viewer console errors during large-file hover: ${[...errors, ...latencyErrors].join("\n")}`);
}

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  await verifyBondDrawing(browser);
  await verifyEndpointFeedbackRules(browser);
  await verifyCreationDragKeepsCanvasVisibleAfterToolSwitch(browser);
  await verifySelectedObjectSuppressesHover(browser);
  await verifyDragHandleCursors(browser);
  await verifyLargeFileHoverAndDrag(browser);
  console.log("[viewer-interaction-smoke] ok");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
