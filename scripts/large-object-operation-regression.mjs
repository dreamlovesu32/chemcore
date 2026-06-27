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
const maxCreationCommitMs = Number(process.env.CHEMCORE_OBJECT_OP_MAX_CREATION_COMMIT_MS || 80);
const maxInteractionCommitMs = Number(process.env.CHEMCORE_OBJECT_OP_MAX_INTERACTION_COMMIT_MS || 120);
const maxBlankPointerDownMs = Number(process.env.CHEMCORE_OBJECT_OP_MAX_BLANK_POINTER_DOWN_MS || 20);
const timingSamples = [];

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

function makeManyFragmentDocument(count) {
  const fragments = Math.max(1, Math.ceil(count / 10));
  const objects = [];
  const resources = {};
  for (let fragmentIndex = 0; fragmentIndex < fragments; fragmentIndex += 1) {
    const nodes = [];
    const bonds = [];
    const column = fragmentIndex % 40;
    const row = Math.floor(fragmentIndex / 40);
    const translate = [60 + column * 70, 60 + row * 70];
    for (let localIndex = 0; localIndex < 10; localIndex += 1) {
      const x = 8 + localIndex * 5;
      const y = 18 + (localIndex % 2) * 6;
      nodes.push({
        id: `mf_${fragmentIndex}_n${localIndex}`,
        element: "O",
        atomicNumber: 8,
        position: [x, y],
        charge: 0,
        numHydrogens: 0,
        label: {
          text: "OH",
          position: [x, y],
          box: [x - 5, y - 8, x + 12, y + 4],
          fontSize: 11,
          glyphPolygons: [[
            [x - 5, y - 8],
            [x + 12, y - 8],
            [x + 12, y + 4],
            [x - 5, y + 4],
          ]],
        },
        meta: null,
      });
      if (localIndex > 0) {
        bonds.push({
          id: `mf_${fragmentIndex}_b${localIndex - 1}`,
          begin: `mf_${fragmentIndex}_n${localIndex - 1}`,
          end: `mf_${fragmentIndex}_n${localIndex}`,
          order: 1,
          strokeWidth: 1,
          meta: null,
        });
      }
    }
    const resourceId = `mf_${fragmentIndex}`;
    resources[resourceId] = {
      id: resourceId,
      type: "molecule_fragment2d",
      encoding: "chemcore.molecule.fragment2d",
      data: {
        bbox: [0, 0, 70, 40],
        nodes,
        bonds,
      },
    };
    objects.push({
      id: `obj_${resourceId}`,
      type: "molecule",
      name: `fragment ${fragmentIndex}`,
      visible: true,
      locked: false,
      zIndex: 10 + fragmentIndex,
      transform: { translate, rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: {
        resourceRef: resourceId,
        bbox: [0, 0, 70, 40],
        extra: {},
      },
      meta: null,
      children: [],
    });
  }
  return {
    format: { name: "chemcore", version: "0.1", unit: "pt" },
    document: {
      id: "doc_many_fragment_hit_test",
      title: "Many fragment hit-test regression",
      page: { width: 3200, height: 1800, background: "#ffffff" },
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
    objects,
    resources,
  };
}

function makeWideBracketSideDocument() {
  return {
    format: { name: "chemcore", version: "0.1", unit: "pt" },
    document: {
      id: "doc_wide_bracket_hit_test",
      title: "Wide bracket hit test",
      page: { width: 240, height: 120, background: "#ffffff" },
      meta: null,
    },
    objects: [
      {
        id: "obj_bracket_group",
        type: "group",
        name: "bracket-group",
        visible: true,
        locked: false,
        zIndex: 9,
        transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
        meta: { kind: "bracket-group" },
        payload: { bbox: [40, 20, 118, 70] },
        children: [
          {
            id: "obj_left_bracket",
            type: "bracket",
            name: "left bracket",
            visible: true,
            locked: false,
            zIndex: 10,
            transform: { translate: [40, 20], rotate: 0, scale: [1, 1] },
            payload: {
              bbox: [0, 0, 18, 70],
              kind: "square",
              side: "left",
              stroke: "#000000",
              strokeWidth: 1,
            },
            children: [],
          },
          {
            id: "obj_right_bracket",
            type: "bracket",
            name: "right bracket",
            visible: true,
            locked: false,
            zIndex: 11,
            transform: { translate: [140, 20], rotate: 0, scale: [1, 1] },
            payload: {
              bbox: [0, 0, 18, 70],
              kind: "square",
              side: "right",
              stroke: "#000000",
              strokeWidth: 1,
            },
            children: [],
          },
        ],
      },
    ],
    resources: {},
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
  page.on("pageerror", (error) => errors.push(error.stack || error.message));
  page.__chemcoreErrors = errors;
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

async function resetCommitStats(page) {
  await page.evaluate(() => {
    window.__chemcoreDebug.creationCommitStats = { samples: [] };
    window.__chemcoreDebug.interactionCommitStats = { samples: [] };
  });
}

function recordCommitTiming(label, kind, sample, maxMs) {
  assert(sample, `${label} did not record a ${kind} commit timing sample`);
  assert(
    Number.isFinite(Number(sample.totalMs)),
    `${label} timing sample has no totalMs: ${JSON.stringify(sample)}`,
  );
  timingSamples.push({
    label,
    kind,
    totalMs: sample.totalMs,
    executeMs: sample.executeMs,
    commandType: sample.commandType || null,
  });
  assert(
    sample.totalMs < maxMs,
    `${label} ${kind} commit took ${sample.totalMs.toFixed(1)}ms, expected < ${maxMs}ms; sample=${JSON.stringify(sample)}`,
  );
}

async function recordLatestInteractionTiming(page, label) {
  const sample = await page.evaluate(() => window.__chemcoreDebug.interactionCommitStats?.last || null);
  recordCommitTiming(label, "interaction", sample, maxInteractionCommitMs);
}

async function assertBlankBondPointerDownUsesFragmentBounds(page) {
  const result = await page.evaluate(async (doc) => {
    await window.__chemcoreDebug.loadDocumentForTest(doc);
    const engine = window.__chemcoreDebug.state.editorEngine;
    engine.setTool("bond", "single");
    const point = { x: 3150, y: 1750 };
    const start = performance.now();
    engine.pointerDown(point.x, point.y, false);
    const pointerDownMs = performance.now() - start;
    engine.pointerUp(point.x + 80, point.y, false);
    return { pointerDownMs };
  }, makeManyFragmentDocument(nodeCount));
  timingSamples.push({
    label: "blank bond pointer down",
    kind: "hit-test",
    totalMs: result.pointerDownMs,
  });
  assert(
    result.pointerDownMs < maxBlankPointerDownMs,
    `Blank bond pointerDown took ${result.pointerDownMs.toFixed(1)}ms, expected < ${maxBlankPointerDownMs}ms`,
  );
}

async function drawBondUsesPrimitivePatch(page) {
  const result = await page.evaluate(async () => {
    const debug = window.__chemcoreDebug;
    await debug.resetEditorEngine();
    await debug.syncDocument();
    const engine = debug.state.editorEngine;
    debug.objectPrimitivePatchStats = null;
    debug.renderStats.captureRenderListStacks = true;
    debug.renderStats.documentRenderCount = 0;
    debug.renderStats.renderListJsonCount = 0;
    debug.renderStats.lastRenderListJsonStack = "";
    const command = {
      type: "add-bond",
      begin: { x: 100, y: 100 },
      end: { x: 148, y: 100 },
      order: 1,
      variant: "single",
    };
    const start = performance.now();
    const commandResult = await debug.commandEngine.executeEngineCommand(
      command,
      () => engine.executeCommandJson(JSON.stringify(command)),
      { sync: false, deferDocumentSync: true, source: "regression" },
    );
    const commandMs = performance.now() - start;
    const renderStart = performance.now();
    const patched = debug.renderDocumentChange(commandResult);
    const renderMs = performance.now() - renderStart;
    const targetBondIds = commandResult?.targets?.bonds || [];
    const targetNodeIds = commandResult?.targets?.nodes || [];
    const countTargetDom = (attribute, ids) => ids.reduce((count, id) => (
      count + document.querySelectorAll(`[${attribute}="${CSS.escape(id)}"]`).length
    ), 0);
    const targetBondTags = targetBondIds.flatMap((id) => (
      [...document.querySelectorAll(`[data-bond-id="${CSS.escape(id)}"]`)]
        .map((element) => element.tagName.toLowerCase())
    ));
    return {
      changed: !!commandResult?.changed,
      patched,
      targets: commandResult?.targets || null,
      created: commandResult?.created || null,
      commandMs,
      renderMs,
      targetBondDomCount: countTargetDom("data-bond-id", targetBondIds),
      targetBondTags,
      targetNodeDomCount: countTargetDom("data-node-id", targetNodeIds),
      objectPrimitivePatchStats: debug.objectPrimitivePatchStats || null,
      documentRenderCount: debug.renderStats.documentRenderCount || 0,
      renderListJsonCount: debug.renderStats.renderListJsonCount || 0,
    };
  });
  timingSamples.push({
    label: "bond command patch",
    kind: "creation",
    totalMs: result.commandMs + result.renderMs,
    executeMs: result.commandMs,
  });
  assert(result.changed && result.patched, `Bond was not patched: ${JSON.stringify(result)}`);
  assert((result.targets?.nodes || []).length > 0, `Bond command did not target nodes: ${JSON.stringify(result)}`);
  assert((result.targets?.bonds || []).length > 0, `Bond command did not target bonds: ${JSON.stringify(result)}`);
  assert(result.targetBondDomCount > 0, `Bond primitive DOM was not patched: ${JSON.stringify(result)}`);
  assert(result.targetBondTags.includes("line"), `Simple bond should render as a stroked line: ${JSON.stringify(result)}`);
  assert(
    !result.objectPrimitivePatchStats?.patched,
    `Bond creation repainted the molecule object instead of primitive targets: ${JSON.stringify(result.objectPrimitivePatchStats)}`,
  );
  assertNoFullRefresh("bond draw", result);
  await page.evaluate((doc) => window.__chemcoreDebug.loadDocumentForTest(doc), makeLargeChainDocument(nodeCount));
  await page.waitForFunction(() => window.__chemcoreDebug?.document?.resources?.mol_large?.data?.nodes?.length > 0);
}

async function drawBondWithMouseKeepsPreviewUntilPatch(page) {
  await page.evaluate(async () => {
    const debug = window.__chemcoreDebug;
    await debug.resetEditorEngine();
    await debug.syncDocument();
  });
  await page.locator('button[data-tool="bond"]').click();
  const drag = await page.evaluate(() => ({
    start: window.__chemcoreDebug.worldToClient(100, 100),
    end: window.__chemcoreDebug.worldToClient(148, 100),
  }));
  await page.mouse.move(drag.start.x, drag.start.y);
  await page.mouse.down();
  await page.mouse.move(drag.end.x, drag.end.y, { steps: 6 });
  await page.waitForFunction(() => (
    document.querySelector(".canvas-drag-preview-svg")?.childElementCount > 0
  ));
  await resetRenderStats(page);
  await resetCommitStats(page);
  await page.evaluate(() => {
    window.__chemcoreDebug.objectPrimitivePatchStats = null;
    window.__chemcoreBondFlashProbe = { running: true, samples: [] };
    const sample = () => {
      const probe = window.__chemcoreBondFlashProbe;
      if (!probe?.running) {
        return;
      }
      probe.samples.push({
        t: performance.now(),
        previewCount: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
        bondCount: document.querySelectorAll('[data-layer="document-content"] [data-bond-id]').length,
        shieldActive: !!document.querySelector(".canvas-pointer-shield.is-active"),
      });
      requestAnimationFrame(sample);
    };
    sample();
  });
  await page.mouse.up();
  await page.waitForTimeout(120);
  const result = await page.evaluate(() => {
    const probe = window.__chemcoreBondFlashProbe || { samples: [] };
    probe.running = false;
    const samples = probe.samples || [];
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    const targetBondIds = command?.targets?.bonds || [];
    const firstBondIndex = samples.findIndex((sample) => sample.bondCount > 0);
    const beforeBond = firstBondIndex >= 0 ? samples.slice(0, firstBondIndex) : samples;
    return {
      changed: !!command?.changed,
      targets: command?.targets || null,
      created: command?.created || null,
      sampleCount: samples.length,
      firstBondIndex,
      targetBondTags: targetBondIds.flatMap((id) => (
        [...document.querySelectorAll(`[data-bond-id="${CSS.escape(id)}"]`)]
          .map((element) => element.tagName.toLowerCase())
      )),
      gapSamples: beforeBond.filter((sample) => sample.previewCount === 0 && sample.bondCount === 0),
      overlapSamples: samples.filter((sample) => sample.previewCount > 0 && sample.bondCount > 0),
      firstSamples: samples.slice(0, 8),
      lastSamples: samples.slice(-8),
      objectPrimitivePatchStats: window.__chemcoreDebug.objectPrimitivePatchStats || null,
      documentRenderCount: window.__chemcoreDebug.renderStats.documentRenderCount || 0,
      renderListJsonCount: window.__chemcoreDebug.renderStats.renderListJsonCount || 0,
      commitStats: window.__chemcoreDebug.creationCommitStats?.last || null,
    };
  });
  assert(result.changed, `Mouse bond was not created: ${JSON.stringify(result)}`);
  assert((result.targets?.bonds || []).length > 0, `Mouse bond did not target bonds: ${JSON.stringify(result)}`);
  assert(result.firstBondIndex >= 0, `Mouse bond never reached document DOM: ${JSON.stringify(result)}`);
  assert(result.targetBondTags.includes("line"), `Mouse bond should render as a stroked line: ${JSON.stringify(result)}`);
  assert(
    result.gapSamples.length === 0,
    `Mouse bond preview disappeared before committed DOM was patched: ${JSON.stringify(result)}`,
  );
  assert(
    result.overlapSamples.length === 0,
    `Mouse bond preview remained visible after committed DOM was patched: ${JSON.stringify(result)}`,
  );
  assert(
    !result.objectPrimitivePatchStats?.patched,
    `Mouse bond repainted the molecule object instead of primitive targets: ${JSON.stringify(result.objectPrimitivePatchStats)}`,
  );
  recordCommitTiming("mouse bond draw", "creation", result.commitStats, maxCreationCommitMs);
  assertNoFullRefresh("mouse bond draw", result);
  await page.evaluate((doc) => window.__chemcoreDebug.loadDocumentForTest(doc), makeLargeChainDocument(nodeCount));
  await page.waitForFunction(() => window.__chemcoreDebug?.document?.resources?.mol_large?.data?.nodes?.length > 0);
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
    const elements = Array.from(document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(id)}"]`));
    const visible = elements.filter((element) => getComputedStyle(element).visibility !== "hidden");
    return { total: elements.length, visible: visible.length };
  }, objectId);
  assert(result.total > 0, `${label} has no original object DOM: ${JSON.stringify(result)}`);
  assert(result.visible === 0, `${label} left original object visible: ${JSON.stringify(result)}`);
}

async function assertObjectEditPreviewVisible(page, objectId, label) {
  const result = await page.evaluate((id) => {
    const elements = Array.from(document.querySelectorAll(`[data-layer="editor-overlay"] [data-object-id="${CSS.escape(id)}"]`));
    const visible = elements.filter((element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.visibility !== "hidden"
        && style.display !== "none"
        && (rect.width > 0 || rect.height > 0);
    });
    return { total: elements.length, visible: visible.length };
  }, objectId);
  assert(result.visible > 0, `${label} did not render a live edit preview: ${JSON.stringify(result)}`);
}

async function drawCurvedArrow(page) {
  await page.locator('button[data-tool="arrow"]').click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
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
  await resetCommitStats(page);
  await page.mouse.move(930, 210);
  await page.mouse.down();
  await page.mouse.move(1120, 210, { steps: 10 });
  await page.mouse.up();
  await page.waitForTimeout(400);
  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    const objectId = command?.targets?.objects?.[0] || command?.created?.objects?.[0] || null;
    return {
      objectId,
      changed: !!command?.changed,
      command,
      activeTool: window.__chemcoreDebug.engineState?.tool?.activeTool
        || window.__chemcoreDebug.engineState?.tool?.active_tool
        || null,
      activeGesture: window.__chemcoreDebug.activeSelectionGesture || null,
      primitiveCount: objectId
        ? JSON.parse(window.__chemcoreDebug.state.editorEngine.renderTargetsJson(JSON.stringify({ objects: [objectId] })) || "[]").length
        : 0,
      domCount: objectId
        ? document.querySelectorAll(`[data-object-id="${CSS.escape(objectId)}"]`).length
        : 0,
      patchStats: window.__chemcoreDebug.objectPrimitivePatchStats || null,
      commitStats: window.__chemcoreDebug.creationCommitStats?.last || null,
      scriptSrc: document.querySelector('script[type="module"]')?.src || "",
    };
  });
  result.errors = page.__chemcoreErrors || [];
  assert(result.changed && result.objectId, `Curved arrow was not created: ${JSON.stringify(result)}`);
  assert(result.domCount > 0, `Curved arrow DOM was not patched: ${JSON.stringify(result)}`);
  recordCommitTiming("curved arrow draw", "creation", result.commitStats, maxCreationCommitMs);
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

async function assertObjectControlHandleStyle(page, role, label) {
  const result = await page.evaluate((expectedRole) => {
    const handle = document.querySelector(`.editor-object-control-handle[data-role="${CSS.escape(expectedRole)}"]`);
    if (!handle) {
      return { found: false };
    }
    const svg = handle.ownerSVGElement;
    const matrix = svg?.getScreenCTM?.();
    const scale = Math.max(Math.abs(matrix?.a || 1), Math.abs(matrix?.d || 1));
    const radiusPx = Number(handle.getAttribute("r") || 0) * scale;
    const style = getComputedStyle(handle);
    return {
      found: true,
      tagName: handle.tagName.toLowerCase(),
      radiusPx,
      fill: style.fill,
      stroke: style.stroke,
    };
  }, role);
  assert(
    result.found
      && result.tagName === "circle"
      && Math.abs(result.radiusPx - 1.5) < 0.2
      && (result.fill === "none" || result.fill === "rgba(0, 0, 0, 0)"),
    `${label} control handle style was not unified: ${JSON.stringify(result)}`,
  );
}

async function dragArrowCurve(page, objectId) {
  await page.locator('button[data-tool="select"]').click();
  await page.evaluate(() => window.__chemcoreDebug.state.editorEngine.clearSelection?.());
  const handle = await findArrowHandle(page, objectId, "curve");
  await resetRenderStats(page);
  await page.mouse.move(handle.x, handle.y);
  await page.waitForFunction(
    () => !!document.querySelector('.editor-object-control-handle[data-role="hover-arrow-handle"]'),
    null,
    { timeout: 1000 },
  );
  await assertObjectControlHandleStyle(page, "hover-arrow-handle", "Arrow");
  await page.mouse.down();
  await page.waitForTimeout(80);
  await assertNoPreviewMask(page, "arrow curve pointerdown");
  assertNoFullRefresh("arrow curve pointerdown", await renderStats(page));
  await resetCommitStats(page);
  await page.mouse.move(handle.x, handle.y + 70, { steps: 12 });
  await assertNoPreviewMask(page, "arrow curve drag");
  await assertObjectHiddenForPreview(page, objectId, "arrow curve drag");
  await assertObjectEditPreviewVisible(page, objectId, "arrow curve drag");
  await page.mouse.up();
  await page.waitForTimeout(500);
  await recordLatestInteractionTiming(page, "arrow curve drag");
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
  await resetCommitStats(page);
  await page.mouse.move(handle.x - 26, handle.y - 22, { steps: 8 });
  await assertObjectHiddenForPreview(page, objectId, "arrow style drag");
  await assertObjectEditPreviewVisible(page, objectId, "arrow style drag");
  await page.mouse.up();
  await page.waitForTimeout(400);
  await recordLatestInteractionTiming(page, "arrow style drag");
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
  await resetCommitStats(page);
  await page.mouse.move(handle.x + 80, handle.y + 50, { steps: 12 });
  await assertObjectHiddenForPreview(page, objectId, "shape handle drag");
  await assertObjectEditPreviewVisible(page, objectId, "shape handle drag");
  await page.mouse.up();
  await page.waitForTimeout(400);
  await recordLatestInteractionTiming(page, "shape handle drag");
}

async function drawShape(page) {
  await page.locator('button[data-tool="shape"]').click();
  await resetRenderStats(page);
  await resetCommitStats(page);
  await page.mouse.move(920, 330);
  await page.mouse.down();
  await page.mouse.move(1070, 430, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(350);
  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    const objectId = command?.targets?.objects?.[0] || command?.created?.objects?.[0] || null;
    return {
      objectId,
      changed: !!command?.changed,
      domCount: objectId
        ? document.querySelectorAll(`[data-object-id="${CSS.escape(objectId)}"]`).length
        : 0,
      commitStats: window.__chemcoreDebug.creationCommitStats?.last || null,
    };
  });
  assert(result.changed && result.objectId, `Shape was not created: ${JSON.stringify(result)}`);
  assert(result.domCount > 0, `Shape DOM was not patched: ${JSON.stringify(result)}`);
  recordCommitTiming("shape draw", "creation", result.commitStats, maxCreationCommitMs);
  assertNoFullRefresh("shape draw", await renderStats(page));
  return result.objectId;
}

async function drawBracketOpensTextEditor(page) {
  await page.locator('button[data-tool="bracket"]').click();
  await resetRenderStats(page);
  await resetCommitStats(page);
  await page.mouse.move(910, 560);
  await page.mouse.down();
  await page.mouse.move(1050, 700, { steps: 8 });
  const started = performance.now();
  await page.mouse.up();
  await page.waitForFunction(() => !!window.__chemcoreDebug.activeTextEditor, null, { timeout: 1000 });
  const elapsed = performance.now() - started;
  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    const objectId = command?.targets?.objects?.[0] || command?.created?.objects?.[0] || null;
    return {
      objectId,
      domCount: objectId
        ? document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`).length
        : 0,
      activeTextEditor: !!window.__chemcoreDebug.activeTextEditor,
      bracketLabelObjectId: window.__chemcoreDebug.activeTextEditor?.bracketLabelObjectId || null,
      documentRenderCount: window.__chemcoreDebug.renderStats.documentRenderCount || 0,
      renderListJsonCount: window.__chemcoreDebug.renderStats.renderListJsonCount || 0,
      commitStats: window.__chemcoreDebug.creationCommitStats?.last || null,
    };
  });
  assert(result.domCount > 0, `Bracket DOM was not patched: ${JSON.stringify(result)}`);
  assert(result.activeTextEditor, `Bracket text editor did not open: ${JSON.stringify(result)}`);
  assert(result.bracketLabelObjectId, `Bracket text editor is not linked to bracket: ${JSON.stringify(result)}`);
  assert(elapsed < 350, `Bracket text editor opened too slowly: ${elapsed.toFixed(1)}ms`);
  recordCommitTiming("bracket draw", "creation", result.commitStats, maxCreationCommitMs);
  assertNoFullRefresh("bracket draw text editor", result);
}

async function assertSquareBracketHandleDrag(page) {
  const setup = await page.evaluate(async () => {
    const svg = document.querySelector("#viewer-svg");
    const matrix = svg?.getScreenCTM?.()?.inverse?.();
    const clientToWorld = (x, y) => {
      const point = new DOMPoint(x, y).matrixTransform(matrix);
      return { x: point.x, y: point.y };
    };
    const begin = clientToWorld(900, 520);
    const end = clientToWorld(1050, 700);
    const command = {
      type: "add-bracket",
      kind: "square",
      begin,
      end,
    };
    const result = JSON.parse(await window.__chemcoreDebug.state.editorEngine.executeCommandJson(JSON.stringify(command)));
    await window.__chemcoreDebug.syncDocument();
    const visit = (object, parent = null, out = []) => {
      out.push({ object, parent });
      for (const child of object.children || []) {
        visit(child, object, out);
      }
      return out;
    };
    const entries = (window.__chemcoreDebug.document.objects || []).flatMap((object) => visit(object));
    const side = entries.find(({ object }) => (
      (object.type || object.objectType || object.object_type) === "bracket"
      && (object.payload?.kind || object.payload?.extra?.kind) === "square"
      && (object.payload?.side || object.payload?.extra?.side)
    ));
    const object = side?.object;
    const parent = side?.parent;
    const bbox = object?.payload?.bbox || [];
    const translate = object?.transform?.translate || [0, 0];
    const rotate = Number(object?.transform?.rotate || 0);
    const handleX = (object.payload?.side || object.payload?.extra?.side) === "right" ? Number(bbox[2] || 0) : 0;
    const tx = Number(translate[0] || 0) + Number(bbox[0] || 0);
    const ty = Number(translate[1] || 0) + Number(bbox[1] || 0);
    const width = Number(bbox[2] || 0);
    const height = Number(bbox[3] || 0);
    const center = { x: tx + width * 0.5, y: ty + height * 0.5 };
    const rotatePoint = (point, degrees) => {
      const radians = degrees * Math.PI / 180;
      const cos = Math.cos(radians);
      const sin = Math.sin(radians);
      const dx = point.x - center.x;
      const dy = point.y - center.y;
      return { x: center.x + dx * cos - dy * sin, y: center.y + dx * sin + dy * cos };
    };
    const top = rotatePoint({ x: tx + handleX, y: ty }, rotate);
    const bottom = rotatePoint({ x: tx + handleX, y: ty + height }, rotate);
    const client = window.__chemcoreDebug.worldToClient(top.x, top.y);
    const domBefore = parent?.id
      ? document.querySelector(`[data-layer="document-content"] [data-object-id="${CSS.escape(parent.id)}"]`)?.getBoundingClientRect()
      : null;
    return {
      result,
      parentId: parent?.id || "",
      sideId: object?.id || "",
      beforeHeight: height,
      top,
      bottom,
      client,
      domBefore: domBefore ? { x: domBefore.x, y: domBefore.y, width: domBefore.width, height: domBefore.height } : null,
    };
  });
  assert(setup.parentId && setup.sideId && setup.client, `Square bracket test setup failed: ${JSON.stringify(setup)}`);
  await page.locator('button[data-tool="select"]').click();
  await page.waitForFunction(() => document.querySelector('button[data-tool="select"]')?.classList.contains("is-active"));
  await page.waitForTimeout(50);
  await resetRenderStats(page);
  await page.mouse.move(setup.client.x, setup.client.y);
  await page.mouse.down();
  await page.mouse.move(setup.client.x, setup.client.y - 70, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(250);
  const result = await page.evaluate(({ sideId, parentId, beforeHeight }) => {
    const documentData = JSON.parse(window.__chemcoreDebug.state.editorEngine.documentJson());
    const objects = [];
    const visit = (object, parent = null) => {
      objects.push({ object, parent });
      for (const child of object.children || []) {
        visit(child, object);
      }
    };
    for (const object of documentData.objects || []) {
      visit(object);
    }
    const side = objects.find(({ object }) => object.id === sideId)?.object || null;
    const domAfter = document.querySelector(`[data-layer="document-content"] [data-object-id="${CSS.escape(parentId)}"]`)?.getBoundingClientRect();
    const command = JSON.parse(window.__chemcoreDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    return {
      changed: !!command?.changed,
      targets: command?.targets || null,
      afterHeight: Number(side?.payload?.bbox?.[3] || 0),
      beforeHeight,
      domAfter: domAfter ? { x: domAfter.x, y: domAfter.y, width: domAfter.width, height: domAfter.height } : null,
      documentRenderCount: window.__chemcoreDebug.renderStats.documentRenderCount || 0,
      renderListJsonCount: window.__chemcoreDebug.renderStats.renderListJsonCount || 0,
      lastRenderListJsonStack: window.__chemcoreDebug.renderStats.lastRenderListJsonStack || "",
      lastCommandSync: window.__chemcoreDebug.renderStats.lastCommandSync || null,
    };
  }, setup);
  assert(result.changed, `Square bracket handle drag did not commit: ${JSON.stringify({ setup, result })}`);
  assert(Math.abs(result.afterHeight - result.beforeHeight) > 1, `Square bracket handle drag did not resize object: ${JSON.stringify({ setup, result })}`);
  assert(result.domAfter?.height > 0, `Square bracket handle drag removed DOM: ${JSON.stringify({ setup, result })}`);
  assertNoFullRefresh("square bracket handle drag", result);
}

async function assertBracketInteriorDoesNotHitBracket(page) {
  const setup = await page.evaluate(async (doc) => {
    await window.__chemcoreDebug.loadDocumentForTest(doc);
    await window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    await window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    const documentData = JSON.parse(window.__chemcoreDebug.state.editorEngine.documentJson());
    const objects = [];
    const visit = (object) => {
      objects.push(object);
      for (const child of object.children || []) {
        visit(child);
      }
    };
    for (const object of documentData.objects || []) {
      visit(object);
    }
    const left = objects.find((object) => object.id === "obj_left_bracket");
    const right = objects.find((object) => object.id === "obj_right_bracket");
    const before = {
      left: [...(left?.transform?.translate || [])],
      right: [...(right?.transform?.translate || [])],
    };
    const leftInterior = window.__chemcoreDebug.worldToClient(49, 55);
    const leftDragEnd = window.__chemcoreDebug.worldToClient(55, 60);
    const betweenSides = window.__chemcoreDebug.worldToClient(100, 55);
    const leftStroke = window.__chemcoreDebug.worldToClient(40.5, 55);
    return { before, leftInterior, leftDragEnd, betweenSides, leftStroke };
  }, makeWideBracketSideDocument());
  assert(setup.leftInterior && setup.betweenSides && setup.leftStroke, `Bracket hit setup failed: ${JSON.stringify(setup)}`);
  await page.locator('button[data-tool="select"]').click();
  await page.waitForFunction(() => document.querySelector('button[data-tool="select"]')?.classList.contains("is-active"));

  await page.mouse.move(setup.leftInterior.x, setup.leftInterior.y);
  await page.waitForTimeout(100);
  let hover = await page.evaluate(() => ({
    handles: document.querySelectorAll('[data-layer="editor-overlay"] [data-role="hover-shape-handle"]').length,
    fastHover: window.__chemcoreDebug.fastSelectHoverStats || null,
  }));
  assert(hover.handles === 0, `Bracket interior incorrectly showed hover handles: ${JSON.stringify(hover)}`);

  await page.mouse.move(setup.betweenSides.x, setup.betweenSides.y);
  await page.waitForTimeout(100);
  hover = await page.evaluate(() => ({
    handles: document.querySelectorAll('[data-layer="editor-overlay"] [data-role="hover-shape-handle"]').length,
    fastHover: window.__chemcoreDebug.fastSelectHoverStats || null,
  }));
  assert(hover.handles === 0, `Space between bracket sides incorrectly showed hover handles: ${JSON.stringify(hover)}`);

  await page.mouse.move(setup.leftStroke.x, setup.leftStroke.y);
  await page.waitForTimeout(100);
  hover = await page.evaluate(() => ({
    handles: document.querySelectorAll('[data-layer="editor-overlay"] [data-role="hover-shape-handle"]').length,
    fastHover: window.__chemcoreDebug.fastSelectHoverStats || null,
  }));
  assert(hover.handles > 0, `Bracket stroke did not show hover handles: ${JSON.stringify(hover)}`);

  await page.evaluate(() => {
    window.__chemcoreDebug.state.editorEngine.clearSelection?.();
    window.__chemcoreDebug.state.editorEngine.clearInteraction?.();
    document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
  });
  await page.mouse.move(setup.leftInterior.x, setup.leftInterior.y);
  await page.mouse.down();
  await page.mouse.move(setup.leftDragEnd.x, setup.leftDragEnd.y, { steps: 4 });
  await page.mouse.up();
  await page.waitForTimeout(150);
  const result = await page.evaluate((before) => {
    const documentData = JSON.parse(window.__chemcoreDebug.state.editorEngine.documentJson());
    const objects = [];
    const visit = (object) => {
      objects.push(object);
      for (const child of object.children || []) {
        visit(child);
      }
    };
    for (const object of documentData.objects || []) {
      visit(object);
    }
    const left = objects.find((object) => object.id === "obj_left_bracket");
    const right = objects.find((object) => object.id === "obj_right_bracket");
    const selection = window.__chemcoreDebug.engineState?.selection || {};
    return {
      before,
      leftTranslate: left?.transform?.translate || null,
      rightTranslate: right?.transform?.translate || null,
      selection,
      handles: document.querySelectorAll('[data-layer="editor-overlay"] [data-role="hover-shape-handle"]').length,
    };
  }, setup.before);
  assert(
    JSON.stringify(result.leftTranslate) === JSON.stringify(setup.before.left)
      && JSON.stringify(result.rightTranslate) === JSON.stringify(setup.before.right),
    `Dragging bracket interior moved a bracket: ${JSON.stringify(result)}`,
  );
  assert(
    !(result.selection?.arrowObjects?.length || result.selection?.arrow_objects?.length),
    `Dragging bracket interior selected a bracket: ${JSON.stringify(result)}`,
  );
}

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({ headless: true });
  const { page, errors } = await openViewer(browser);
  await drawBondUsesPrimitivePatch(page);
  await drawBondWithMouseKeepsPreviewUntilPatch(page);
  const arrowId = await drawCurvedArrow(page);
  await assertArrowEndpointPointerDown(page, arrowId);
  await dragArrowStyleHandle(page, arrowId);
  await dragArrowCurve(page, arrowId);
  const shapeId = await drawShape(page);
  await assertShapePointerDown(page, shapeId);
  await drawBracketOpensTextEditor(page);
  await assertSquareBracketHandleDrag(page);
  await assertBracketInteriorDoesNotHitBracket(page);
  await assertBlankBondPointerDownUsesFragmentBounds(page);
  await page.close();
  assert(!errors.length, `Viewer console errors:\n${errors.join("\n")}`);
  const timingSummary = timingSamples
    .map((sample) => `${sample.label} ${sample.totalMs.toFixed(1)}ms`)
    .join(", ");
  console.log(`[large-object-operation-regression] ok (${nodeCount} nodes; ${timingSummary})`);
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
