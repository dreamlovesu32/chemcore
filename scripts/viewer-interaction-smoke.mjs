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
  const hadPreview = await page.evaluate(() => !!document.querySelector('[data-role="preview-bond"]'));
  await page.mouse.up();
  await page.waitForTimeout(250);
  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    return {
      previewLeft: !!document.querySelector('[data-role^="preview-"]'),
      changed: !!command?.changed,
      bondTargets: command?.targets?.bonds?.length || command?.created?.bonds?.length || 0,
      hasRenderedBond: /data-bond-id=/.test(document.querySelector("#viewer-svg")?.outerHTML || ""),
    };
  });
  await page.close();
  assert(hadPreview, "Bond drag did not show a preview.");
  assert(!result.previewLeft, "Bond preview remained after pointerup.");
  assert(result.changed && result.bondTargets > 0 && result.hasRenderedBond, "Bond drag did not commit a rendered bond.");
  assert(!errors.length, `Viewer console errors during bond drawing: ${errors.join("\n")}`);
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
    await page.waitForTimeout(250);
    if (item.closeText) {
      await page.keyboard.press("Escape");
      await page.waitForTimeout(50);
    }
    const after = await page.evaluate(() => {
      const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
      const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      return {
        changed: !!command?.changed,
        targets: command?.targets || null,
        created: command?.created || null,
        objectCount: flatten(window.__chemcoreDebug.engineState.document.objects || [])
          .filter((object) => (object.type || object.objectType || object.object_type) !== "molecule")
          .length,
        shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
      };
    });
    assert(after.changed, `${item.tool} first drag after tool switch did not commit: ${JSON.stringify(after)}`);
    assert(after.objectCount >= before.objectCount + item.expectedObjects, `${item.tool} first drag after tool switch did not create an object: ${JSON.stringify({ before, after })}`);
    assert(!after.shieldActive, `${item.tool} pointerup left pointer shield active.`);
  }

  await page.close();
  assert(!errors.length, `Viewer console errors during creation visibility regression: ${errors.join("\n")}`);
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
  const entries = [];
  for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
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
    for (const node of fragment.nodes || []) {
      if (!Array.isArray(node.position) || !degree.get(node.id)) {
        continue;
      }
      const x = Number(translate[0] || 0) + Number(node.position[0] || 0);
      const y = Number(translate[1] || 0) + Number(node.position[1] || 0);
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
        label: node.label?.text || node.label?.sourceText || "",
        element: node.element || "",
        degree: degree.get(node.id) || 0,
      });
    }
  }
  const hover = [...document.querySelectorAll("[data-node-id]")]
    .map((element) => {
      const rect = element.getBoundingClientRect();
      return {
        id: element.getAttribute("data-node-id"),
        x: rect.x + rect.width / 2,
        y: rect.y + rect.height / 2,
        w: rect.width,
        h: rect.height,
      };
    })
    .filter((entry) => entry.w >= 3
      && entry.h >= 2
      && entry.x > 80
      && entry.x < innerWidth - 80
      && entry.y > 120
      && entry.y < innerHeight - 80)[0] || null;
  return {
    hover,
    label: entries.find((entry) => entry.label && entry.degree > 0) || null,
    atom: entries.find((entry) => !entry.label && (!entry.element || entry.element === "C") && entry.degree > 0) || null,
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
  const during = await page.evaluate(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    const partial = document.querySelector('[data-layer="document-partial-bond-preview"]');
    return {
      partialChildren: partial?.childElementCount || 0,
      hasDocumentMask: !!overlay?.querySelector('[data-role="preview-document-mask"]'),
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
  assert(!during.hasDocumentMask, `${kind} drag fell back to full document preview mask.`);
  assert(!after.partial, `${kind} drag left partial bond preview behind.`);
  assert(after.transformed === 0, `${kind} drag left transformed document nodes behind.`);
  assert(after.previews === 0, `${kind} drag left preview overlay behind.`);
  assert(after.gesture === null, `${kind} drag left an active selection gesture behind.`);
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

async function resetViewerUi(page) {
  await page.keyboard.press("Escape").catch(() => {});
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
  assert(elapsed < thresholdMs, `${label} committed too slowly: ${elapsed.toFixed(1)}ms (action=${actionMs}ms wait=${waitMs}ms)`);
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
      const objectId = result?.targets?.objects?.[0]
        || result?.created?.objects?.[0]
        || result?.updated?.objects?.[0]
        || "";
      return result?.changed
        && objectId.startsWith("obj_symbol")
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
  const targets = await page.evaluate(largeFileTargetFinder);
  assert(targets.hover, "Large CDXML did not expose a visible hover target.");
  assert(targets.label, "Large CDXML did not expose a draggable label node target.");
  assert(targets.atom, "Large CDXML did not expose a draggable atom node target.");

  await page.mouse.move(targets.hover.x, targets.hover.y);
  await page.waitForTimeout(250);
  const hover = await page.evaluate(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    return overlay?.querySelectorAll('[data-role^="hover-"]').length || 0;
  });
  assert(hover > 0, "Large CDXML select hover did not render a hover overlay.");

  await verifyLargeFileSelectionLatency(page, targets.hover);
  await verifyLargeDragTarget(page, targets.label, "Label");
  await verifyLargeDragTarget(page, targets.atom, "Atom");
  const latency = await verifyLargeFileCommitLatency(page);
  await page.close();
  console.log(`[viewer-interaction-smoke] large commit latency bracket=${latency.bracketMs.toFixed(1)}ms symbol=${latency.symbolMs.toFixed(1)}ms bond=${latency.bondMs.toFixed(1)}ms`);
  assert(!errors.length, `Viewer console errors during large-file hover: ${errors.join("\n")}`);
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
  await verifyCreationDragKeepsCanvasVisibleAfterToolSwitch(browser);
  await verifyDragHandleCursors(browser);
  await verifyLargeFileHoverAndDrag(browser);
  console.log("[viewer-interaction-smoke] ok");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
