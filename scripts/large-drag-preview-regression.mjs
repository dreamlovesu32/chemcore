import { spawn } from "node:child_process";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const nodeCount = Number(process.env.CHEMCORE_LARGE_DRAG_NODE_COUNT || 1400);
const maxDragMs = Number(process.env.CHEMCORE_LARGE_DRAG_MAX_MS || 1800);
const maxPreviewSettleMs = Number(process.env.CHEMCORE_LARGE_DRAG_SETTLE_MAX_MS || 900);

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
    const isTargetLabelAtom = index === 700;
    nodes.push({
      id: `n${index}`,
      element: isTargetLabelAtom ? "O" : "C",
      atomicNumber: isTargetLabelAtom ? 8 : 6,
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
      id: "doc_large_drag_regression",
      title: "Large drag regression",
      page: { width: 820, height: Math.max(620, height), background: "#ffffff" },
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
        bbox: [0, 0, 820, Math.max(620, height)],
        extra: {},
      },
      meta: null,
      children: [],
    }],
    resources: {
      mol_large: {
        type: "molecule_fragment2d",
        encoding: "chemcore.molecule.fragment2d",
        data: {
          schema: "chemcore.molecule.fragment2d",
          bbox: [0, 0, 820, Math.max(620, height)],
          nodes,
          bonds,
          meta: null,
        },
        meta: null,
      },
    },
  };
}

async function openViewer(browser) {
  const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
  const errors = [];
  const phase = { name: "open" };
  await page.addInitScript(() => {
    window.__chemcoreRegressionPhase = "open";
    window.__chemcoreRegressionConsoleErrors = [];
    const originalError = console.error.bind(console);
    console.error = (...args) => {
      window.__chemcoreRegressionConsoleErrors.push({
        phase: window.__chemcoreRegressionPhase || "unknown",
        text: args.map((arg) => String(arg)).join(" "),
        stack: new Error().stack || "",
      });
      originalError(...args);
    };
  });
  page.on("console", (message) => {
    if (message.type() === "error") {
      const location = message.location();
      errors.push(`${phase.name}: ${message.text()} @ ${location.url}:${location.lineNumber}:${location.columnNumber}`);
    }
  });
  page.on("pageerror", (error) => errors.push(`${phase.name}: ${error.message}`));
  await page.goto(`${baseUrl}?largeDragRegression=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => !!window.__chemcoreDebug?.loadDocumentForTest, null, { timeout: 20000 });
  return { page, errors, phase };
}

async function main() {
  const server = await ensureServer();
  let browser = null;
  try {
    browser = await chromium.launch({ headless: true });
    const { page, errors, phase } = await openViewer(browser);
    const documentData = makeLargeChainDocument(nodeCount);
    phase.name = "load";
    await page.evaluate(() => { window.__chemcoreRegressionPhase = "load"; });
    await page.evaluate(async (doc) => {
      await window.__chemcoreDebug.loadDocumentForTest(doc);
      window.__chemcoreDebug.renderStats.documentRenderCount = 0;
    }, documentData);
    errors.length = 0;
    const loaded = await page.evaluate(() => ({
      objects: window.__chemcoreDebug.document.objects,
      resourceKeys: Object.keys(window.__chemcoreDebug.document.resources || {}),
      resource: window.__chemcoreDebug.document.resources?.mol_large || null,
      nodeCount: window.__chemcoreDebug.document.resources?.mol_large?.data?.nodes?.length || 0,
      bondCount: window.__chemcoreDebug.document.resources?.mol_large?.data?.bonds?.length || 0,
      firstNode: window.__chemcoreDebug.document.resources?.mol_large?.data?.nodes?.[0] || null,
      firstBond: window.__chemcoreDebug.document.resources?.mol_large?.data?.bonds?.[0] || null,
      renderCount: JSON.parse(window.__chemcoreDebug.getRenderListJson()).length,
      documentHtmlLength: document.querySelector('[data-layer="document-content"]')?.outerHTML.length || 0,
    }));
    assert(loaded.renderCount > 0, `Synthetic large document rendered no primitives: ${JSON.stringify(loaded).slice(0, 1200)}`);
    await page.locator('button[data-tool="select"]').click();

    phase.name = "select";
    await page.evaluate(() => { window.__chemcoreRegressionPhase = "select"; });
    const target = await page.evaluate(() => {
      const node = window.__chemcoreDebug.document.resources.mol_large.data.nodes[700];
      const client = window.__chemcoreDebug.worldToClient(node.position[0], node.position[1]);
      return { id: node.id, x: client.x, y: client.y };
    });

    await page.mouse.move(target.x, target.y);
    await page.mouse.click(target.x, target.y);
    await page.waitForTimeout(80);
    const before = await page.evaluate(() => ({
      renderCount: window.__chemcoreDebug.renderStats.documentRenderCount,
      htmlLength: document.querySelector('[data-layer="document-content"]')?.outerHTML.length || 0,
      selection: window.__chemcoreDebug.engineState?.selection || null,
      activeTool: document.querySelector("[data-tool].is-active")?.getAttribute("data-tool") || null,
    }));

    phase.name = "drag";
    await page.evaluate(() => {
      window.__chemcoreRegressionPhase = "drag";
      window.__chemcoreDebug.backendMovePreviewStats = { samples: [] };
      window.__chemcoreDebug.backendPreviewSchedulerStats = { runs: 0, backendRuns: 0, errors: [] };
    });
    await page.mouse.move(target.x, target.y);
    await page.mouse.down();
    const started = performance.now();
    for (let step = 1; step <= 12; step += 1) {
      await page.mouse.move(target.x + step * 3, target.y + step * 1.5);
      await page.waitForTimeout(16);
    }
    const dragMs = performance.now() - started;
    const settleStarted = performance.now();
    await page.waitForTimeout(200);
    const previewSettleMs = performance.now() - settleStarted;
    const during = await page.evaluate(() => ({
      partialChildren: document.querySelector('[data-layer="document-partial-bond-preview"]')?.childElementCount || 0,
      previewMask: !!document.querySelector('[data-role="preview-document-mask"]'),
      renderCount: window.__chemcoreDebug.renderStats.documentRenderCount,
      gesture: window.__chemcoreDebug.activeSelectionGesture || null,
      previewStats: window.__chemcoreDebug.backendMovePreviewStats || { samples: [] },
      schedulerStats: window.__chemcoreDebug.backendPreviewSchedulerStats || { runs: 0, backendRuns: 0, errors: [] },
    }));

    phase.name = "commit";
    await page.evaluate(() => { window.__chemcoreRegressionPhase = "commit"; });
    await page.mouse.up();
    await page.waitForTimeout(1000);
    const after = await page.evaluate(() => ({
      renderCount: window.__chemcoreDebug.renderStats.documentRenderCount,
      partialExists: !!document.querySelector('[data-layer="document-partial-bond-preview"]'),
      partialChildren: document.querySelector('[data-layer="document-partial-bond-preview"]')?.childElementCount || 0,
      previewExists: !!document.querySelector('[data-layer="document-batch-preview"]'),
      transformed: document.querySelectorAll(".is-preview-transforming").length,
      gesture: window.__chemcoreDebug.activeSelectionGesture || null,
      htmlLength: document.querySelector('[data-layer="document-content"]')?.outerHTML.length || 0,
      movedBondCount: document.querySelectorAll('[data-bond-id="b699"], [data-bond-id="b700"]').length,
    }));

    assert(during.partialChildren === 0, `Drag used a front-end partial-bond preview instead of backend primitives: ${JSON.stringify(during)}`);
    assert(!during.previewMask, "Drag used a full document preview mask.");
    assert(during.renderCount === before.renderCount, "Dragging triggered renderDocument().");
    assert(after.renderCount === before.renderCount, "Pointerup triggered renderDocument().");
    assert(!after.partialExists, `Pointerup left partial-bond preview layer behind: ${JSON.stringify({ before, during, after })}`);
    assert(!after.previewExists, "Pointerup left batch preview layer behind.");
    assert(after.transformed === 0, "Pointerup left preview-transforming DOM nodes behind.");
    assert(after.gesture === null, "Pointerup left an active selection gesture behind.");
    assert(after.movedBondCount > 0, "Committed DOM no longer contains moved bond primitives.");
    assert(after.htmlLength > before.htmlLength * 0.9, "Committed DOM unexpectedly collapsed after drag.");
    const previewSamples = during.previewStats.samples || [];
    const sortedTotals = previewSamples.map((sample) => sample.totalMs).sort((a, b) => a - b);
    const p95Total = sortedTotals[Math.max(0, Math.ceil(sortedTotals.length * 0.95) - 1)] || 0;
    assert(previewSamples.length >= 6, `Backend drag preview did not update continuously: ${JSON.stringify({
      previewStats: during.previewStats,
      schedulerStats: during.schedulerStats,
      gesture: during.gesture,
      partialChildren: during.partialChildren,
      previewMask: during.previewMask,
    })}`);
    assert(previewSamples.every((sample) => sample.patched), `Backend drag preview had unpatched samples: ${JSON.stringify(during.previewStats)}`);
    assert(p95Total < 32, `Backend drag preview p95 took ${p95Total.toFixed(1)}ms, expected < 32ms: ${JSON.stringify(during.previewStats)}`);
    assert(dragMs < maxDragMs, `Large drag took ${dragMs.toFixed(1)}ms, expected < ${maxDragMs}ms.`);
    assert(previewSettleMs < maxPreviewSettleMs, `Backend preview settled in ${previewSettleMs.toFixed(1)}ms, expected < ${maxPreviewSettleMs}ms.`);
    const actionableErrors = errors.filter((entry) => !entry.includes("null pointer passed to rust"));
    const consoleStacks = await page.evaluate(() => window.__chemcoreRegressionConsoleErrors || []);
    const postLoadConsoleStacks = consoleStacks.filter((entry) => entry.phase !== "open" && entry.phase !== "load");
    assert(
      !actionableErrors.length,
      `Viewer console errors:\n${actionableErrors.join("\n")}\n${JSON.stringify(postLoadConsoleStacks, null, 2)}`,
    );
    await page.close();
    console.log(`[large-drag-preview-regression] ok (${nodeCount} nodes, drag ${dragMs.toFixed(1)}ms, settle ${previewSettleMs.toFixed(1)}ms, preview p95 ${p95Total.toFixed(1)}ms/${previewSamples.length} samples)`);
  } finally {
    await browser?.close();
    server?.kill();
  }
}

await main();
