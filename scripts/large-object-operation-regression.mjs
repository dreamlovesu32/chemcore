import { spawn } from "node:child_process";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const nodeCount = Number(process.env.CHEMCORE_OBJECT_OP_NODE_COUNT || 8000);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function portIsOpen() {
  return new Promise((resolve) => {
    const socket = net.connect({ host, port }, () => {
      socket.end();
      resolve(true);
    });
    socket.on("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

function waitForPort(timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const socket = net.connect({ host, port }, () => {
        socket.end();
        resolve(true);
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

function makeLargeChainDocument(count) {
  const spacing = 28;
  const columns = 40;
  const nodes = [];
  const bonds = [];
  for (let index = 0; index < count; index += 1) {
    const column = index % columns;
    const row = Math.floor(index / columns);
    nodes.push({
      id: `n${index}`,
      element: "C",
      atomicNumber: 6,
      position: [40 + column * spacing, 40 + row * spacing],
      charge: 0,
      numHydrogens: 0,
      meta: null,
    });
    if (index > 0) {
      bonds.push({
        id: `b${index - 1}`,
        begin: `n${index - 1}`,
        end: `n${index}`,
        order: 1,
        strokeWidth: 1,
        meta: null,
      });
    }
  }
  const height = 120 + Math.ceil(count / columns) * spacing;
  return {
    format: { name: "chemcore", version: "0.1", unit: "pt" },
    document: {
      id: "doc_large_object_ops",
      title: "Large object operations",
      page: { width: 1300, height: Math.max(900, height), background: "#ffffff" },
      meta: null,
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        fill: "#000000",
        strokeWidth: 0.85,
        fontFamily: "Arial",
        fontSize: 11,
      },
    },
    objects: [{
      id: "obj_large_molecule",
      type: "molecule",
      name: "large molecule",
      visible: true,
      locked: false,
      zIndex: 10,
      transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: {
        resourceRef: "mol_large",
        bbox: [0, 0, 1300, Math.max(900, height)],
        extra: {},
      },
      meta: null,
      children: [],
    }],
    resources: {
      mol_large: {
        id: "mol_large",
        type: "molecule_fragment2d",
        encoding: "chemcore.molecule.fragment2d",
        data: { nodes, bonds },
      },
    },
  };
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
  await page.waitForFunction(() => !!window.__chemcoreDebug?.loadDocumentForTest, null, { timeout: 20000 });
  await page.evaluate((doc) => window.__chemcoreDebug.loadDocumentForTest(doc), makeLargeChainDocument(nodeCount));
  await page.waitForFunction(() => window.__chemcoreDebug?.document?.resources?.mol_large?.data?.nodes?.length > 0);
  return { page, errors };
}

async function resetRenderStats(page) {
  await page.evaluate(() => {
    window.__chemcoreDebug.renderStats.captureRenderListStacks = true;
    window.__chemcoreDebug.renderStats.documentRenderCount = 0;
    window.__chemcoreDebug.renderStats.renderListJsonCount = 0;
    window.__chemcoreDebug.renderStats.lastRenderListJsonStack = "";
  });
}

async function renderStats(page) {
  return page.evaluate(() => ({
    documentRenderCount: window.__chemcoreDebug.renderStats.documentRenderCount || 0,
    renderListJsonCount: window.__chemcoreDebug.renderStats.renderListJsonCount || 0,
    lastRenderListJsonStack: window.__chemcoreDebug.renderStats.lastRenderListJsonStack || "",
  }));
}

function assertNoFullRefresh(label, stats) {
  assert(stats.documentRenderCount === 0, `${label} called renderDocument(): ${JSON.stringify(stats)}`);
  assert(stats.renderListJsonCount === 0, `${label} called full renderListJson(): ${JSON.stringify(stats)}`);
}

async function assertNoPreviewMask(page, label) {
  const maskCount = await page.evaluate(() => (
    document.querySelectorAll('[data-role="preview-document-mask"]').length
  ));
  assert(maskCount === 0, `${label} displayed full-page preview mask: ${maskCount}`);
}

async function assertObjectHiddenForPreview(page, objectId, label) {
  const result = await page.evaluate((id) => {
    const elements = Array.from(document.querySelectorAll(`[data-object-id="${CSS.escape(id)}"]`));
    const visible = elements.filter((element) => getComputedStyle(element).visibility !== "hidden");
    return { total: elements.length, visible: visible.length };
  }, objectId);
  assert(result.total > 0, `${label} has no original object DOM: ${JSON.stringify(result)}`);
  assert(result.visible === 0, `${label} left original object visible: ${JSON.stringify(result)}`);
}

async function drawCurvedArrow(page) {
  await page.locator('button[data-tool="arrow"]').click();
  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.setArrowEndpointOptions(
      "curved",
      "small",
      "120",
      "full",
      "none",
      "none",
      false,
    );
  });
  await resetRenderStats(page);
  await page.mouse.move(930, 210);
  await page.mouse.down();
  await page.mouse.move(1120, 210, { steps: 10 });
  await page.mouse.up();
  await page.waitForTimeout(400);
  const result = await page.evaluate(() => {
    const lines = (window.__chemcoreDebug.document.objects || [])
      .filter((object) => (object.type || object.objectType || object.object_type) === "line");
    const object = lines[lines.length - 1] || null;
    return {
      objectId: object?.id || null,
      lineCount: lines.length,
      domCount: object?.id
        ? document.querySelectorAll(`[data-object-id="${CSS.escape(object.id)}"]`).length
        : 0,
    };
  });
  assert(result.objectId, `Curved arrow was not created: ${JSON.stringify(result)}`);
  assert(result.domCount > 0, `Curved arrow DOM was not patched: ${JSON.stringify(result)}`);
  assertNoFullRefresh("curved arrow draw", await renderStats(page));
  return result.objectId;
}

async function findArrowHandle(page, objectId, action) {
  const handle = await page.evaluate(({ id, expectedAction }) => {
    const elements = Array.from(document.querySelectorAll(`[data-object-id="${CSS.escape(id)}"]`));
    const rects = elements
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 || rect.height > 0);
    const rect = rects.length
      ? {
          left: Math.min(...rects.map((candidate) => candidate.left)),
          right: Math.max(...rects.map((candidate) => candidate.right)),
          top: Math.min(...rects.map((candidate) => candidate.top)),
          bottom: Math.max(...rects.map((candidate) => candidate.bottom)),
        }
      : null;
    if (!rect) {
      return null;
    }
    rect.width = rect.right - rect.left;
    rect.height = rect.bottom - rect.top;
    const svg = elements[0]?.ownerSVGElement;
    const matrix = svg?.getScreenCTM?.()?.inverse?.();
    if (!matrix) {
      return null;
    }
    const candidates = [];
    const divisions = 10;
    for (let row = 0; row <= divisions; row += 1) {
      for (let column = 0; column <= divisions; column += 1) {
        candidates.push({
          x: rect.left + (rect.width * column) / divisions,
          y: rect.top + (rect.height * row) / divisions,
        });
      }
    }
    for (const point of candidates) {
      const world = new DOMPoint(point.x, point.y).matrixTransform(matrix);
      if (window.__chemcoreDebug.state.editorEngine.hoverArrowAction(world.x, world.y) === expectedAction) {
        return point;
      }
    }
    return null;
  }, { id: objectId, expectedAction: action });
  assert(handle, `Could not find ${action} handle for ${objectId}`);
  return handle;
}

async function dragArrowCurve(page, objectId) {
  await page.locator('button[data-tool="select"]').click();
  await page.evaluate(() => window.__chemcoreDebug.state.editorEngine.clearSelection?.());
  const handle = await findArrowHandle(page, objectId, "curve");
  await resetRenderStats(page);
  await page.mouse.move(handle.x, handle.y);
  await page.mouse.down();
  await page.waitForTimeout(80);
  await assertNoPreviewMask(page, "arrow curve pointerdown");
  assertNoFullRefresh("arrow curve pointerdown", await renderStats(page));
  await page.mouse.move(handle.x, handle.y + 70, { steps: 12 });
  await assertNoPreviewMask(page, "arrow curve drag");
  await assertObjectHiddenForPreview(page, objectId, "arrow curve drag");
  await page.mouse.up();
  await page.waitForTimeout(500);
  const result = await page.evaluate((id) => {
    const object = (window.__chemcoreDebug.document.objects || []).find((candidate) => candidate.id === id);
    const arrowHead = object?.payload?.arrowHead || object?.payload?.extra?.arrowHead || null;
    return {
      curve: arrowHead?.curve ?? null,
      domCount: document.querySelectorAll(`[data-object-id="${CSS.escape(id)}"]`).length,
    };
  }, objectId);
  assert(result.domCount > 0, `Arrow curve DOM disappeared: ${JSON.stringify(result)}`);
  assert(result.curve !== null, `Arrow curve did not remain editable: ${JSON.stringify(result)}`);
  assertNoFullRefresh("arrow curve drag", await renderStats(page));
}

async function dragArrowStyleHandle(page, objectId) {
  await page.locator('button[data-tool="select"]').click();
  await page.evaluate(() => window.__chemcoreDebug.state.editorEngine.clearSelection?.());
  const handle = await findArrowHandle(page, objectId, "head-style");
  const before = await page.evaluate((id) => {
    const object = (window.__chemcoreDebug.document.objects || []).find((candidate) => candidate.id === id);
    const arrowHead = object?.payload?.arrowHead || object?.payload?.extra?.arrowHead || {};
    return {
      length: Number(arrowHead.length || 0),
      width: Number(arrowHead.width || 0),
    };
  }, objectId);
  await resetRenderStats(page);
  await page.mouse.move(handle.x, handle.y);
  await page.mouse.down();
  await page.waitForTimeout(80);
  await assertNoPreviewMask(page, "arrow style pointerdown");
  assertNoFullRefresh("arrow style pointerdown", await renderStats(page));
  await page.mouse.move(handle.x - 26, handle.y - 22, { steps: 8 });
  await assertObjectHiddenForPreview(page, objectId, "arrow style drag");
  await page.mouse.up();
  await page.waitForTimeout(400);
  const after = await page.evaluate((id) => {
    const object = (window.__chemcoreDebug.document.objects || []).find((candidate) => candidate.id === id);
    const arrowHead = object?.payload?.arrowHead || object?.payload?.extra?.arrowHead || {};
    return {
      length: Number(arrowHead.length || 0),
      width: Number(arrowHead.width || 0),
      domCount: document.querySelectorAll(`[data-object-id="${CSS.escape(id)}"]`).length,
    };
  }, objectId);
  assert(after.domCount > 0, `Arrow style DOM disappeared: ${JSON.stringify(after)}`);
  assert(
    Math.abs(after.length - before.length) > 0.01 || Math.abs(after.width - before.width) > 0.01,
    `Arrow style handle did not change dimensions: before=${JSON.stringify(before)} after=${JSON.stringify(after)}`,
  );
  assertNoFullRefresh("arrow style drag", await renderStats(page));
}

async function assertArrowEndpointPointerDown(page, objectId) {
  await page.locator('button[data-tool="select"]').click();
  await page.evaluate(() => window.__chemcoreDebug.state.editorEngine.clearSelection?.());
  const handle = await findArrowHandle(page, objectId, "head");
  await resetRenderStats(page);
  await page.mouse.move(handle.x, handle.y);
  await page.mouse.down();
  await page.waitForTimeout(80);
  await assertNoPreviewMask(page, "arrow endpoint pointerdown");
  assertNoFullRefresh("arrow endpoint pointerdown", await renderStats(page));
  await page.mouse.up();
}

async function findShapeHandle(page, objectId) {
  const handle = await page.evaluate((id) => {
    const elements = Array.from(document.querySelectorAll(`[data-object-id="${CSS.escape(id)}"]`));
    const rects = elements
      .map((element) => element.getBoundingClientRect())
      .filter((rect) => rect.width > 0 || rect.height > 0);
    const rect = rects.length
      ? {
          left: Math.min(...rects.map((candidate) => candidate.left)),
          right: Math.max(...rects.map((candidate) => candidate.right)),
          top: Math.min(...rects.map((candidate) => candidate.top)),
          bottom: Math.max(...rects.map((candidate) => candidate.bottom)),
        }
      : null;
    if (!rect) {
      return null;
    }
    rect.width = rect.right - rect.left;
    rect.height = rect.bottom - rect.top;
    const svg = elements[0]?.ownerSVGElement;
    const matrix = svg?.getScreenCTM?.()?.inverse?.();
    if (!matrix) {
      return null;
    }
    const candidates = [
      { x: rect.left, y: rect.top },
      { x: rect.right, y: rect.top },
      { x: rect.left, y: rect.bottom },
      { x: rect.right, y: rect.bottom },
      { x: rect.left + rect.width * 0.5, y: rect.top },
      { x: rect.right, y: rect.top + rect.height * 0.5 },
      { x: rect.left + rect.width * 0.5, y: rect.bottom },
      { x: rect.left, y: rect.top + rect.height * 0.5 },
    ];
    for (const point of candidates) {
      const world = new DOMPoint(point.x, point.y).matrixTransform(matrix);
      if (window.__chemcoreDebug.state.editorEngine.hoverShapeAction(world.x, world.y)) {
        return point;
      }
    }
    return null;
  }, objectId);
  assert(handle, `Could not find shape handle for ${objectId}`);
  return handle;
}

async function assertShapePointerDown(page, objectId) {
  await page.locator('button[data-tool="select"]').click();
  await page.evaluate(() => window.__chemcoreDebug.state.editorEngine.clearSelection?.());
  const handle = await findShapeHandle(page, objectId);
  await resetRenderStats(page);
  await page.mouse.move(handle.x, handle.y);
  await page.mouse.down();
  await page.waitForTimeout(80);
  await assertNoPreviewMask(page, "shape handle pointerdown");
  assertNoFullRefresh("shape handle pointerdown", await renderStats(page));
  await page.mouse.up();
}

async function drawShape(page) {
  await page.locator('button[data-tool="shape"]').click();
  await resetRenderStats(page);
  await page.mouse.move(920, 330);
  await page.mouse.down();
  await page.mouse.move(1070, 430, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(350);
  const result = await page.evaluate(() => {
    const shapes = (window.__chemcoreDebug.document.objects || [])
      .filter((object) => (object.type || object.objectType || object.object_type) === "shape");
    const object = shapes[shapes.length - 1] || null;
    return {
      objectId: object?.id || null,
      shapeCount: shapes.length,
      domCount: object?.id
        ? document.querySelectorAll(`[data-object-id="${CSS.escape(object.id)}"]`).length
        : 0,
    };
  });
  assert(result.objectId, `Shape was not created: ${JSON.stringify(result)}`);
  assert(result.domCount > 0, `Shape DOM was not patched: ${JSON.stringify(result)}`);
  assertNoFullRefresh("shape draw", await renderStats(page));
  return result.objectId;
}

async function drawBracketOpensTextEditor(page) {
  await page.locator('button[data-tool="bracket"]').click();
  await resetRenderStats(page);
  await page.mouse.move(910, 560);
  await page.mouse.down();
  await page.mouse.move(1050, 700, { steps: 8 });
  const started = performance.now();
  await page.mouse.up();
  await page.waitForFunction(() => !!window.__chemcoreDebug.activeTextEditor, null, { timeout: 1000 });
  const elapsed = performance.now() - started;
  const result = await page.evaluate(() => {
    const brackets = (window.__chemcoreDebug.document.objects || [])
      .filter((object) => (object.type || object.objectType || object.object_type) === "bracket");
    return {
      bracketCount: brackets.length,
      activeTextEditor: !!window.__chemcoreDebug.activeTextEditor,
      bracketLabelObjectId: window.__chemcoreDebug.activeTextEditor?.bracketLabelObjectId || null,
      documentRenderCount: window.__chemcoreDebug.renderStats.documentRenderCount || 0,
      renderListJsonCount: window.__chemcoreDebug.renderStats.renderListJsonCount || 0,
    };
  });
  assert(result.bracketCount > 0, `Bracket was not created: ${JSON.stringify(result)}`);
  assert(result.activeTextEditor, `Bracket text editor did not open: ${JSON.stringify(result)}`);
  assert(result.bracketLabelObjectId, `Bracket text editor is not linked to bracket: ${JSON.stringify(result)}`);
  assert(elapsed < 350, `Bracket text editor opened too slowly: ${elapsed.toFixed(1)}ms`);
  assertNoFullRefresh("bracket draw text editor", result);
}

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({ headless: true });
  const { page, errors } = await openViewer(browser);
  const arrowId = await drawCurvedArrow(page);
  await assertArrowEndpointPointerDown(page, arrowId);
  await dragArrowStyleHandle(page, arrowId);
  await dragArrowCurve(page, arrowId);
  const shapeId = await drawShape(page);
  await assertShapePointerDown(page, shapeId);
  await drawBracketOpensTextEditor(page);
  await page.close();
  assert(!errors.length, `Viewer console errors:\n${errors.join("\n")}`);
  console.log(`[large-object-operation-regression] ok (${nodeCount} nodes)`);
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
