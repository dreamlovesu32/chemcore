import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { makeSyntheticLargeDocument } from "./generate-stability-fixtures.mjs";
import { engineToolForUiTool } from "../viewer/editor_tool_model.js";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const syntheticNodeCount = Number(process.env.CHEMCORE_STABILITY_SYNTHETIC_BROWSER_NODES || 6500);

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

function capturePageErrors(page, errors) {
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(error.stack || error.message));
}

async function openViewer(browser) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?stability=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => !!window.__chemcoreDebug?.state?.editorEngine, null, { timeout: 20000 });
  return { page, errors };
}

async function activateTool(page, tool) {
  await page.locator(`button[data-tool="${tool}"]`).click();
  await page.waitForFunction((expectedTool) => {
    const state = window.__chemcoreDebug?.editorState || {};
    return state?.activeTool === expectedTool
      && document.querySelector(`button[data-tool="${CSS.escape(expectedTool)}"]`)?.classList.contains("is-active");
  }, tool, { timeout: 2000 });
}

async function viewerBox(page) {
  const box = await page.locator("#viewer-container").boundingBox();
  assert(box, "Viewer container was not visible.");
  return box;
}

async function dragOnCanvas(page, tool, start, end, steps = 8) {
  await activateTool(page, tool);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps });
  await page.mouse.up();
  await page.waitForTimeout(160);
}

async function clickCanvas(page, tool, point) {
  await activateTool(page, tool);
  await page.mouse.click(point.x, point.y);
  await page.waitForTimeout(140);
}

async function documentSnapshot(page) {
  return page.evaluate(() => {
    const parse = (text) => {
      try {
        return JSON.parse(text || "null");
      } catch {
        return null;
      }
    };
    const debug = window.__chemcoreDebug;
    const doc = parse(debug?.state?.editorEngine?.documentJson?.()) || debug?.document || null;
    const objects = [];
    const visit = (object, parentId = null) => {
      if (!object) {
        return;
      }
      const type = object.type || object.objectType || object.object_type || "";
      objects.push({
        id: object.id,
        type,
        parentId,
        text: object.payload?.text || object.payload?.extra?.text || "",
      });
      for (const child of object.children || []) {
        visit(child, object.id);
      }
    };
    for (const object of doc?.objects || []) {
      visit(object);
    }
    const typeCounts = {};
    for (const object of objects) {
      typeCounts[object.type] = (typeCounts[object.type] || 0) + 1;
    }
    let nodeCount = 0;
    let bondCount = 0;
    for (const resource of Object.values(doc?.resources || {})) {
      nodeCount += resource?.data?.nodes?.length || 0;
      bondCount += resource?.data?.bonds?.length || 0;
    }
    return {
      objectIds: objects.map((object) => object.id).filter(Boolean),
      objects,
      typeCounts,
      objectCount: objects.length,
      nodeCount,
      bondCount,
      activeTool: debug?.editorState?.activeTool || "",
      elementPlacementActive: !!debug?.editorState?.elementPlacementActive,
      engineTool: debug?.engineState?.tool?.activeTool || debug?.engineState?.tool?.active_tool || "",
      activeTextEditor: !!debug?.activeTextEditor,
    };
  });
}

async function documentGeometrySnapshot(page) {
  return page.evaluate(() => {
    const parse = (text) => {
      try {
        return JSON.parse(text || "null");
      } catch {
        return null;
      }
    };
    const debug = window.__chemcoreDebug;
    const engine = debug?.state?.editorEngine;
    const doc = parse(engine?.documentJson?.()) || debug?.document || null;
    const engineState = parse(engine?.stateJson?.()) || null;
    const nodes = [];
    const bonds = [];
    for (const [resourceId, resource] of Object.entries(doc?.resources || {})) {
      for (const node of resource?.data?.nodes || []) {
        const [x, y] = node.position || [];
        const client = typeof debug?.worldToClient === "function"
          ? debug.worldToClient(Number(x), Number(y))
          : null;
        nodes.push({
          id: node.id,
          resourceId,
          x: Number(x),
          y: Number(y),
          clientX: client?.x ?? null,
          clientY: client?.y ?? null,
        });
      }
      for (const bond of resource?.data?.bonds || []) {
        bonds.push({
          id: bond.id,
          resourceId,
          begin: bond.begin,
          end: bond.end,
          order: bond.order ?? null,
        });
      }
    }
    const objectAnchor = (object) => {
      const payload = object?.payload || {};
      if (Array.isArray(payload.bbox)) {
        return { x: Number(payload.bbox[0]), y: Number(payload.bbox[1]) };
      }
      if (Array.isArray(payload.geometry?.boundingBox)) {
        return {
          x: Number(payload.geometry.boundingBox[0]),
          y: Number(payload.geometry.boundingBox[1]),
        };
      }
      if (payload.begin && payload.end) {
        return {
          x: Math.min(Number(payload.begin.x || payload.begin[0] || 0), Number(payload.end.x || payload.end[0] || 0)),
          y: Math.min(Number(payload.begin.y || payload.begin[1] || 0), Number(payload.end.y || payload.end[1] || 0)),
        };
      }
      if (payload.center) {
        return {
          x: Number(payload.center.x || payload.center[0] || 0),
          y: Number(payload.center.y || payload.center[1] || 0),
        };
      }
      return null;
    };
    const objects = [];
    const visit = (object, parentId = null) => {
      if (!object) {
        return;
      }
      objects.push({
        id: object.id,
        type: object.type || object.objectType || object.object_type || "",
        parentId,
        anchor: objectAnchor(object),
        childCount: object.children?.length || 0,
      });
      for (const child of object.children || []) {
        visit(child, object.id);
      }
    };
    for (const object of doc?.objects || []) {
      visit(object);
    }
    const selection = engineState?.selection || {};
    const selected = {
      nodes: selection.nodes || [],
      bonds: selection.bonds || [],
      labelNodes: selection.labelNodes || selection.label_nodes || [],
      textObjects: selection.textObjects || selection.text_objects || [],
      arrowObjects: selection.arrowObjects || selection.arrow_objects || [],
    };
    return {
      nodes,
      bonds,
      objects,
      selected,
      activeGestureKind: debug?.activeSelectionGesture?.kind || "",
      leftoverPreviewLayerCount: document.querySelectorAll(
        '[data-layer="document-partial-bond-preview"], [data-layer="document-batch-preview"], .is-preview-transforming',
      ).length,
      backendMovePreviewLast: debug?.backendMovePreviewStats?.last || null,
    };
  });
}

function assertSnapshotContains(before, after, label) {
  const afterIds = new Set(after.objectIds);
  const missing = before.objectIds.filter((id) => !afterIds.has(id));
  assert(!missing.length, `${label} removed existing objects: ${JSON.stringify({ missing: missing.slice(0, 20), before, after })}`);
  assert(after.nodeCount >= before.nodeCount, `${label} reduced node count: ${JSON.stringify({ before, after })}`);
  assert(after.bondCount >= before.bondCount, `${label} reduced bond count: ${JSON.stringify({ before, after })}`);
}

function selectionAnchorCount(selection) {
  return (selection?.nodes?.length || 0)
    + (selection?.bonds?.length || 0)
    + (selection?.labelNodes?.length || 0)
    + (selection?.textObjects?.length || 0)
    + (selection?.arrowObjects?.length || 0);
}

function pointDelta(before, after) {
  if (!before || !after) {
    return Infinity;
  }
  return Math.hypot(Number(after.x) - Number(before.x), Number(after.y) - Number(before.y));
}

function byId(entries) {
  return new Map(entries.map((entry) => [entry.id, entry]));
}

async function selectionBoundsClientCenter(page) {
  return page.evaluate(() => {
    const parse = (text) => {
      try {
        return JSON.parse(text || "null");
      } catch {
        return null;
      }
    };
    const debug = window.__chemcoreDebug;
    const bounds = parse(debug?.state?.editorEngine?.renderBoundsJson?.("selection") || "null");
    if (!bounds || typeof debug?.worldToClient !== "function") {
      return null;
    }
    const world = {
      x: (Number(bounds.minX) + Number(bounds.maxX)) / 2,
      y: (Number(bounds.minY) + Number(bounds.maxY)) / 2,
    };
    const client = debug.worldToClient(world.x, world.y);
    return client ? { x: client.x, y: client.y, world } : null;
  });
}

async function firstVisibleNodeTarget(page) {
  return page.evaluate(() => {
    const candidates = [...document.querySelectorAll('[data-layer="document-content"] [data-node-id]')]
      .filter((element) => !element.classList.contains("document-diagnostic-marker"))
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          id: element.getAttribute("data-node-id"),
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2,
          area: rect.width * rect.height,
          visible: rect.width > 1 && rect.height > 1
            && rect.right >= 0 && rect.bottom >= 0
            && rect.left <= window.innerWidth && rect.top <= window.innerHeight,
        };
      })
      .filter((entry) => entry.visible);
    candidates.sort((a, b) => b.area - a.area);
    if (candidates[0]) {
      return candidates[0];
    }
    const parse = (text) => {
      try {
        return JSON.parse(text || "null");
      } catch {
        return null;
      }
    };
    const debug = window.__chemcoreDebug;
    const doc = parse(debug?.state?.editorEngine?.documentJson?.()) || debug?.document || null;
    if (!doc || typeof debug?.worldToClient !== "function") {
      return null;
    }
    const fallback = [];
    for (const resource of Object.values(doc.resources || {})) {
      for (const node of resource?.data?.nodes || []) {
        const position = node.position || [];
        const client = debug.worldToClient(Number(position[0]), Number(position[1]));
        if (!client) {
          continue;
        }
        fallback.push({
          id: node.id,
          x: client.x,
          y: client.y,
          area: 0,
          visible: client.x >= 0 && client.y >= 0 && client.x <= window.innerWidth && client.y <= window.innerHeight,
        });
      }
    }
    return fallback.find((entry) => entry.visible) || fallback[0] || null;
  });
}

async function verifyPaletteAccessibleUnderTools(page) {
  const tools = ["bond", "select", "text", "arrow", "bracket", "symbol", "shape", "orbital", "templates", "chain"];
  for (const tool of tools) {
    await activateTool(page, tool);
    await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.classList.add("is-active"));
    await page.locator(".quick-palette-toggle-element").click();
    await page.waitForFunction(() => {
      const state = window.__chemcoreDebug?.editorState || {};
      const engineTool = window.__chemcoreDebug?.engineState?.tool?.activeTool
        || window.__chemcoreDebug?.engineState?.tool?.active_tool
        || "";
      return document.querySelector(".quick-palette")?.classList.contains("is-open")
        && document.querySelector(".quick-palette")?.dataset.mode === "element"
        && state.elementPlacementActive
        && engineTool === "element";
    });
    const elementState = await page.evaluate((expectedTool) => {
      const editorState = window.__chemcoreDebug?.editorState || {};
      return {
        open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
        mode: document.querySelector(".quick-palette")?.dataset.mode || "",
        activeButtons: [...document.querySelectorAll(".tool-button.is-active")].map((button) => button.dataset.tool),
        activeTool: editorState.activeTool,
        elementPlacementActive: !!editorState.elementPlacementActive,
        engineTool: window.__chemcoreDebug?.engineState?.tool?.activeTool
          || window.__chemcoreDebug?.engineState?.tool?.active_tool
          || "",
        shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
        expectedTool,
      };
    }, tool);
    assert(
      elementState.open
        && elementState.mode === "element"
        && elementState.activeTool === tool
        && elementState.activeButtons.includes(tool)
        && elementState.elementPlacementActive
        && elementState.engineTool === "element"
        && !elementState.shieldActive,
      `Element quick palette failed under ${tool}: ${JSON.stringify(elementState)}`,
    );

    await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.classList.add("is-active"));
    await page.locator(".quick-palette-toggle-symbol").click();
    const expectedEngineTool = engineToolForUiTool(tool);
    try {
      await page.waitForFunction(({ expectedTool, expectedEngineTool }) => {
        const state = window.__chemcoreDebug?.editorState || {};
        const engineTool = window.__chemcoreDebug?.engineState?.tool?.activeTool
          || window.__chemcoreDebug?.engineState?.tool?.active_tool
          || "";
        return document.querySelector(".quick-palette")?.dataset.mode === "symbol"
          && state.activeTool === expectedTool
          && !state.elementPlacementActive
          && engineTool === expectedEngineTool;
      }, { expectedTool: tool, expectedEngineTool }, { timeout: 2000 });
    } catch (error) {
      const diagnostic = await page.evaluate(({ expectedTool, expectedEngineTool }) => {
        const editorState = window.__chemcoreDebug?.editorState || {};
        return {
          open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
          mode: document.querySelector(".quick-palette")?.dataset.mode || "",
          activeButtons: [...document.querySelectorAll(".tool-button.is-active")].map((button) => button.dataset.tool),
          activeTool: editorState.activeTool,
          elementPlacementActive: !!editorState.elementPlacementActive,
          engineTool: window.__chemcoreDebug?.engineState?.tool?.activeTool
            || window.__chemcoreDebug?.engineState?.tool?.active_tool
            || "",
          shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
          expectedTool,
          expectedEngineTool,
        };
      }, { expectedTool: tool, expectedEngineTool });
      throw new Error(`Symbol quick palette did not stabilize under ${tool}: ${JSON.stringify(diagnostic)}\n${error.message}`);
    }
    const symbolState = await page.evaluate(({ expectedTool, expectedEngineTool }) => {
      const editorState = window.__chemcoreDebug?.editorState || {};
      return {
        open: document.querySelector(".quick-palette")?.classList.contains("is-open") || false,
        mode: document.querySelector(".quick-palette")?.dataset.mode || "",
        activeButtons: [...document.querySelectorAll(".tool-button.is-active")].map((button) => button.dataset.tool),
        activeTool: editorState.activeTool,
        elementPlacementActive: !!editorState.elementPlacementActive,
        engineTool: window.__chemcoreDebug?.engineState?.tool?.activeTool
          || window.__chemcoreDebug?.engineState?.tool?.active_tool
          || "",
        shieldActive: document.querySelector(".canvas-pointer-shield")?.classList.contains("is-active") || false,
        expectedTool,
        expectedEngineTool,
      };
    }, { expectedTool: tool, expectedEngineTool });
    assert(
      symbolState.open
        && symbolState.mode === "symbol"
        && symbolState.activeTool === tool
        && symbolState.activeButtons.includes(tool)
        && !symbolState.elementPlacementActive
        && symbolState.engineTool === engineToolForUiTool(tool)
        && !symbolState.shieldActive,
      `Symbol quick palette failed under ${tool}: ${JSON.stringify(symbolState)}`,
    );
    await page.mouse.click(18, 18);
    await page.waitForTimeout(80);
  }
}

async function verifySelectAtomDragDoesNotCreateBonds(page) {
  const box = await viewerBox(page);
  const start = { x: box.x + box.width / 2 - 160, y: box.y + box.height / 2 - 40 };
  const end = { x: start.x + 120, y: start.y };
  await dragOnCanvas(page, "bond", start, end);
  const before = await documentSnapshot(page);
  const target = await firstVisibleNodeTarget(page);
  assert(target, `No atom target after creating a bond: ${JSON.stringify(before)}`);
  await activateTool(page, "select");
  for (const [dx, dy] of [[42, 12], [-24, 30], [36, -20], [-18, -16]]) {
    await page.mouse.move(target.x, target.y);
    await page.mouse.down();
    await page.mouse.move(target.x + dx, target.y + dy, { steps: 7 });
    await page.mouse.up();
    await page.waitForTimeout(130);
  }
  const after = await documentSnapshot(page);
  assert(after.bondCount === before.bondCount, `Select atom drags created bonds: ${JSON.stringify({ before, after })}`);
  assert(after.activeTool === "select" && after.engineTool === "select", `Select drag left wrong tool state: ${JSON.stringify(after)}`);
}

async function verifyPartialSelectionDragKeepsTopology(page) {
  const box = await viewerBox(page);
  const y = box.y + box.height / 2 - 30;
  await dragOnCanvas(page, "bond", { x: box.x + 350, y }, { x: box.x + 470, y });
  let geometry = await documentGeometrySnapshot(page);
  assert(geometry.nodes.length >= 2 && geometry.bonds.length >= 1, `Initial chain bond was not created: ${JSON.stringify(geometry)}`);
  const firstRight = [...geometry.nodes].sort((a, b) => b.x - a.x)[0];
  assert(Number.isFinite(firstRight.clientX) && Number.isFinite(firstRight.clientY), `Right endpoint is not visible: ${JSON.stringify(firstRight)}`);
  await dragOnCanvas(
    page,
    "bond",
    { x: firstRight.clientX, y: firstRight.clientY },
    { x: firstRight.clientX + 120, y: firstRight.clientY },
  );
  geometry = await documentGeometrySnapshot(page);
  assert(geometry.nodes.length >= 3 && geometry.bonds.length >= 2, `Three-node chain was not created: ${JSON.stringify(geometry)}`);
  const sortedNodes = [...geometry.nodes].sort((a, b) => a.x - b.x);
  const left = sortedNodes[0];
  const middle = sortedNodes[Math.floor(sortedNodes.length / 2)];
  const terminal = sortedNodes[sortedNodes.length - 1];
  const beforeByNode = byId(geometry.nodes);

  await activateTool(page, "select");
  await page.mouse.click(terminal.clientX, terminal.clientY);
  await page.waitForTimeout(160);
  const selected = await documentGeometrySnapshot(page);
  assert(
    selected.selected.nodes.includes(terminal.id)
      && !selected.selected.nodes.includes(left.id)
      && selected.selected.bonds.length === 0,
    `Terminal-only selection failed: ${JSON.stringify({ terminal, left, selected: selected.selected })}`,
  );

  await page.mouse.move(terminal.clientX, terminal.clientY);
  await page.mouse.down();
  await page.mouse.move(terminal.clientX + 86, terminal.clientY - 34, { steps: 8 });
  await page.waitForTimeout(80);
  const duringDrag = await documentGeometrySnapshot(page);
  assert(duringDrag.activeGestureKind === "move", `Partial selection did not enter move gesture: ${JSON.stringify(duringDrag)}`);
  const usedBackendPreview = !!duringDrag.backendMovePreviewLast?.changed
    && !!duringDrag.backendMovePreviewLast?.patched
    && duringDrag.backendMovePreviewLast.nodeCount > 0;
  assert(
    duringDrag.leftoverPreviewLayerCount > 0 || usedBackendPreview,
    `Partial node drag did not use a recognized preview path: ${JSON.stringify(duringDrag)}`,
  );
  await page.mouse.up();
  await page.waitForTimeout(260);

  const after = await documentGeometrySnapshot(page);
  const afterByNode = byId(after.nodes);
  assert(after.nodes.length === geometry.nodes.length, `Partial drag changed node count: ${JSON.stringify({ before: geometry, after })}`);
  assert(after.bonds.length === geometry.bonds.length, `Partial drag changed bond count: ${JSON.stringify({ before: geometry, after })}`);
  assert(pointDelta(beforeByNode.get(terminal.id), afterByNode.get(terminal.id)) > 5, `Terminal atom did not move: ${JSON.stringify({ terminal, after })}`);
  assert(pointDelta(beforeByNode.get(left.id), afterByNode.get(left.id)) < 0.5, `Unselected left atom moved: ${JSON.stringify({ left, after })}`);
  assert(pointDelta(beforeByNode.get(middle.id), afterByNode.get(middle.id)) < 0.5, `Unselected middle atom moved: ${JSON.stringify({ middle, after })}`);
  assert(after.leftoverPreviewLayerCount === 0, `Partial drag left preview DOM behind: ${JSON.stringify(after)}`);
}

async function verifyMixedSelectionDragCommitsCleanly(page) {
  const box = await viewerBox(page);
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  await dragOnCanvas(page, "bond", { x: cx - 260, y: cy - 80 }, { x: cx - 135, y: cy - 40 });
  await dragOnCanvas(page, "arrow", { x: cx - 45, y: cy - 90 }, { x: cx + 115, y: cy - 90 });
  await dragOnCanvas(page, "shape", { x: cx + 165, y: cy - 120 }, { x: cx + 260, y: cy - 30 });
  await clickCanvas(page, "symbol", { x: cx + 315, y: cy - 70 });

  const before = await documentGeometrySnapshot(page);
  assert(before.nodes.length >= 2 && before.objects.length >= 3, `Mixed setup did not create enough content: ${JSON.stringify(before)}`);
  await activateTool(page, "select");
  await page.mouse.move(cx - 300, cy - 155);
  await page.mouse.down();
  await page.mouse.move(cx + 345, cy + 5, { steps: 7 });
  await page.mouse.up();
  await page.waitForTimeout(180);

  const selected = await documentGeometrySnapshot(page);
  assert(
    selected.selected.nodes.length > 0 && selected.selected.arrowObjects.length > 0,
    `Mixed marquee did not select both structure and graphic objects: ${JSON.stringify(selected.selected)}`,
  );
  assert(selectionAnchorCount(selected.selected) >= 3, `Mixed marquee selected too little: ${JSON.stringify(selected.selected)}`);

  const center = await selectionBoundsClientCenter(page);
  assert(center, `No selection bounds for mixed drag: ${JSON.stringify(selected)}`);
  await page.mouse.move(center.x, center.y);
  await page.mouse.down();
  await page.mouse.move(center.x + 92, center.y + 58, { steps: 10 });
  await page.waitForTimeout(80);
  const duringDrag = await documentGeometrySnapshot(page);
  assert(duringDrag.activeGestureKind === "move", `Mixed selection did not enter move gesture: ${JSON.stringify(duringDrag)}`);
  await page.mouse.up();
  await page.waitForTimeout(300);

  const after = await documentGeometrySnapshot(page);
  const beforeNodes = byId(before.nodes);
  const afterNodes = byId(after.nodes);
  const beforeObjects = byId(before.objects.filter((object) => object.anchor));
  const afterObjects = byId(after.objects.filter((object) => object.anchor));
  const movedNodeCount = before.nodes.filter((node) => pointDelta(node, afterNodes.get(node.id)) > 5).length;
  const movedObjectCount = [...beforeObjects.values()]
    .filter((object) => pointDelta(object.anchor, afterObjects.get(object.id)?.anchor) > 5)
    .length;
  assert(after.nodes.length === before.nodes.length, `Mixed drag changed node count: ${JSON.stringify({ before, after })}`);
  assert(after.bonds.length === before.bonds.length, `Mixed drag changed bond count: ${JSON.stringify({ before, after })}`);
  assert(after.objects.length >= before.objects.length, `Mixed drag removed objects: ${JSON.stringify({ before, after })}`);
  assert(movedNodeCount > 0, `Mixed drag did not move selected structure: ${JSON.stringify({ before, after })}`);
  assert(movedObjectCount > 0, `Mixed drag did not move selected graphic objects: ${JSON.stringify({ before, after })}`);
  assert(after.leftoverPreviewLayerCount === 0, `Mixed drag left preview DOM behind: ${JSON.stringify(after)}`);
}

async function verifyBracketLabelCommitPreservesNewObjects(page) {
  const box = await viewerBox(page);
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  await dragOnCanvas(page, "bond", { x: cx - 260, y: cy - 140 }, { x: cx - 145, y: cy - 100 });
  await dragOnCanvas(page, "arrow", { x: cx - 60, y: cy - 150 }, { x: cx + 90, y: cy - 150 });
  await dragOnCanvas(page, "shape", { x: cx + 150, y: cy - 170 }, { x: cx + 245, y: cy - 90 });
  await clickCanvas(page, "symbol", { x: cx + 305, y: cy - 130 });
  await dragOnCanvas(page, "orbital", { x: cx - 240, y: cy + 45 }, { x: cx - 140, y: cy + 132 });
  await dragOnCanvas(page, "bracket", { x: cx + 20, y: cy + 20 }, { x: cx + 150, y: cy + 140 });
  await page.waitForFunction(() => !!window.__chemcoreDebug?.activeTextEditor, null, { timeout: 1500 });
  await page.locator(".text-editor-input").focus();
  await page.keyboard.type("3");
  await page.waitForFunction(() => window.__chemcoreDebug?.activeTextEditor?.plainText === "3", null, { timeout: 1000 });
  const beforeCommit = await documentSnapshot(page);
  await page.mouse.click(box.x + 28, box.y + 28);
  await page.waitForFunction(() => !window.__chemcoreDebug?.activeTextEditor, null, { timeout: 2000 });
  await page.waitForTimeout(220);
  const afterCommit = await documentSnapshot(page);
  assertSnapshotContains(beforeCommit, afterCommit, "Bracket label blank-click commit");
  assert(
    (afterCommit.typeCounts.text || 0) >= 1
      && afterCommit.objects.some((object) => object.type === "text" && object.text.includes("3")),
    `Bracket label text was not committed: ${JSON.stringify({ beforeCommit, afterCommit })}`,
  );
  for (const type of ["line", "shape", "bracket", "symbol"]) {
    assert((afterCommit.typeCounts[type] || 0) > 0, `Missing ${type} after mixed creation commit: ${JSON.stringify(afterCommit)}`);
  }
}

async function verifySyntheticLargeMixedOperations(page) {
  const synthetic = makeSyntheticLargeDocument({ nodeCount: syntheticNodeCount, objectRepeats: 36 });
  await page.evaluate((doc) => window.__chemcoreDebug.loadDocumentForTest(doc), synthetic);
  await page.waitForFunction((expected) => {
    const doc = window.__chemcoreDebug?.document;
    return (doc?.resources?.mol_large?.data?.nodes?.length || 0) >= expected;
  }, syntheticNodeCount, { timeout: 20000 });
  const loaded = await documentSnapshot(page);
  assert(loaded.nodeCount >= syntheticNodeCount, `Synthetic large document did not load: ${JSON.stringify(loaded)}`);

  await activateTool(page, "shape");
  await page.locator(".quick-palette-toggle-element").click();
  await page.waitForFunction(() => {
    const state = window.__chemcoreDebug?.editorState || {};
    return state.activeTool === "shape" && state.elementPlacementActive;
  });
  await page.locator(".quick-palette-toggle-symbol").click();
  await page.waitForFunction(() => {
    const state = window.__chemcoreDebug?.editorState || {};
    return state.activeTool === "shape" && !state.elementPlacementActive
      && document.querySelector(".quick-palette")?.dataset.mode === "symbol";
  });
  await page.mouse.click(16, 16);

  const target = await firstVisibleNodeTarget(page);
  assert(target, "Synthetic large document exposed no visible atom target.");
  await activateTool(page, "select");
  await page.mouse.move(target.x, target.y);
  await page.mouse.down();
  await page.mouse.move(target.x + 34, target.y + 18, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(150);

  const box = await viewerBox(page);
  await dragOnCanvas(
    page,
    "bracket",
    { x: box.x + box.width - 320, y: box.y + box.height - 240 },
    { x: box.x + box.width - 180, y: box.y + box.height - 120 },
  );
  await page.waitForFunction(() => !!window.__chemcoreDebug?.activeTextEditor, null, { timeout: 1500 });
  await page.locator(".text-editor-input").focus();
  await page.keyboard.type("2");
  const beforeCommit = await documentSnapshot(page);
  await page.mouse.click(box.x + 24, box.y + 24);
  await page.waitForFunction(() => !window.__chemcoreDebug?.activeTextEditor, null, { timeout: 2000 });
  await page.waitForTimeout(220);
  const afterCommit = await documentSnapshot(page);
  assertSnapshotContains(beforeCommit, afterCommit, "Synthetic large bracket label commit");
  assert(afterCommit.nodeCount >= syntheticNodeCount, `Large document nodes disappeared: ${JSON.stringify(afterCommit)}`);
  assert(afterCommit.bondCount >= loaded.bondCount, `Large document bonds disappeared: ${JSON.stringify({ loaded, afterCommit })}`);
}

async function runScenario(browser, name, scenario) {
  const { page, errors } = await openViewer(browser);
  const started = performance.now();
  try {
    await scenario(page);
    assert(!errors.length, `Console errors:\n${errors.join("\n")}`);
    const elapsed = performance.now() - started;
    return { name, ok: true, elapsedMs: elapsed };
  } catch (error) {
    return {
      name,
      ok: false,
      elapsedMs: performance.now() - started,
      message: error?.stack || error?.message || String(error),
      consoleErrors: errors,
    };
  } finally {
    await page.close().catch(() => {});
  }
}

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  const scenarios = [
    ["quick palettes under every tool", verifyPaletteAccessibleUnderTools],
    ["select atom drag does not draw bonds", verifySelectAtomDragDoesNotCreateBonds],
    ["partial selection drag keeps topology", verifyPartialSelectionDragKeepsTopology],
    ["mixed selection drag commits cleanly", verifyMixedSelectionDragCommitsCleanly],
    ["bracket label commit preserves new objects", verifyBracketLabelCommitPreservesNewObjects],
    ["synthetic large mixed operations", verifySyntheticLargeMixedOperations],
  ];
  const results = [];
  for (const [name, scenario] of scenarios) {
    const result = await runScenario(browser, name, scenario);
    results.push(result);
    const label = result.ok ? "ok" : "fail";
    console.log(`[stability-user-paths] ${label} ${name} (${result.elapsedMs.toFixed(1)}ms)`);
  }
  const failures = results.filter((result) => !result.ok);
  if (failures.length) {
    console.error(JSON.stringify({ ok: false, failures }, null, 2));
    process.exit(1);
  }
  const totalMs = results.reduce((sum, result) => sum + result.elapsedMs, 0);
  console.log(`[stability-user-paths] ok (${results.length} scenarios, ${totalMs.toFixed(1)}ms)`);
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
