import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const url = process.argv[2] || "http://127.0.0.1:8765/viewer/";
const outputDir = process.argv[3] || path.resolve("tmp/label-anchor-regression");

fs.mkdirSync(outputDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1400, height: 1000 },
  deviceScaleFactor: 1.25,
});
page.setDefaultTimeout(8000);

function logStep(label) {
  console.log(`STEP ${label}`);
}

async function waitForReady() {
  await page.goto(url, { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.__chemcoreDebug?.state?.editorEngine && window.__chemcoreDebug?.document);
}

async function resetDocument() {
  await page.click('[data-command="new"]');
  await page.waitForFunction(() => window.__chemcoreDebug?.document && window.__chemcoreDebug?.state?.editorEngine);
}

async function engineEval(payload) {
  return page.evaluate((input) => {
    const debug = window.__chemcoreDebug;
    const engine = debug.state.editorEngine;
    const fragment = () => debug.engineState.document.resources.mol_editor.data;
    if (input.action === "drawBond") {
      engine.setTool("bond", input.bondVariant);
      engine.pointerDown(input.start.x, input.start.y, false);
      engine.pointerMove(input.end.x, input.end.y, false);
      engine.pointerUp(input.end.x, input.end.y, false);
      debug.syncDocument();
      return fragment();
    }
    if (input.action === "replaceLabel") {
      const node = fragment().nodes.find((candidate) => candidate.id === input.nodeId);
      engine.pointerMove(node.position[0], node.position[1], false);
      if (!engine.replaceHoveredEndpointLabel(input.label)) {
        throw new Error(`Failed to replace label for ${input.nodeId}`);
      }
      debug.syncDocument();
      return fragment();
    }
    if (input.action === "cycleBondToCenter") {
      const bondCenter = () => {
        const frag = fragment();
        const bond = frag.bonds.find((candidate) => candidate.id === input.bondId);
        const begin = frag.nodes.find((candidate) => candidate.id === bond.begin);
        const end = frag.nodes.find((candidate) => candidate.id === bond.end);
        return {
          x: (begin.position[0] + end.position[0]) * 0.5,
          y: (begin.position[1] + end.position[1]) * 0.5,
        };
      };
      engine.setTool("bond", "double");
      for (let index = 0; index < 4; index += 1) {
        const center = bondCenter();
        engine.pointerMove(center.x, center.y, false);
        engine.pointerDown(center.x, center.y, false);
        engine.pointerUp(center.x, center.y, false);
        debug.syncDocument();
        const placement = fragment().bonds.find((candidate) => candidate.id === input.bondId)?.double?.placement;
        if (placement === "center") {
          return fragment();
        }
      }
      throw new Error(`Failed to cycle ${input.bondId} to center`);
    }
    if (input.action === "addBranchFromLabel") {
      const frag = fragment();
      const node = frag.nodes.find((candidate) => candidate.id === input.nodeId);
      const polygon = node.label?.glyphPolygons?.find((candidate) => Array.isArray(candidate) && candidate.length);
      if (!polygon) {
        throw new Error(`No glyph polygon for ${input.nodeId}`);
      }
      let minX = Infinity;
      let minY = Infinity;
      let maxX = -Infinity;
      let maxY = -Infinity;
      for (const [x, y] of polygon) {
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
      const anchor = { x: (minX + maxX) * 0.5, y: (minY + maxY) * 0.5 };
      engine.setTool("bond", "single");
      engine.pointerMove(anchor.x, anchor.y, false);
      engine.pointerDown(anchor.x, anchor.y, false);
      engine.pointerMove(input.end.x, input.end.y, false);
      engine.pointerUp(input.end.x, input.end.y, false);
      debug.syncDocument();
      return fragment();
    }
    if (input.action === "snapshot") {
      return fragment();
    }
    throw new Error(`Unknown action: ${input.action}`);
  }, payload);
}

function polygonCenter(polygon) {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const [x, y] of polygon) {
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  }
  return {
    x: (minX + maxX) * 0.5,
    y: (minY + maxY) * 0.5,
  };
}

function terminalNode(fragment) {
  return [...fragment.nodes].sort((left, right) => left.position[0] - right.position[0]).at(-1);
}

function labelCenter(node) {
  const polygon = node.label?.glyphPolygons?.[0];
  assert(polygon?.length, `node ${node.id} has no label polygon`);
  return polygonCenter(polygon);
}

function bondAndNormal(fragment, nodeId) {
  const bond = fragment.bonds.find((candidate) => candidate.begin === nodeId || candidate.end === nodeId);
  assert(bond, `no bond for ${nodeId}`);
  const begin = fragment.nodes.find((candidate) => candidate.id === bond.begin);
  const end = fragment.nodes.find((candidate) => candidate.id === bond.end);
  const dx = end.position[0] - begin.position[0];
  const dy = end.position[1] - begin.position[1];
  const length = Math.hypot(dx, dy);
  assert(length > 0, `zero-length bond ${bond.id}`);
  return {
    bond,
    begin,
    end,
    normal: { x: -dy / length, y: dx / length },
  };
}

function normalProjection(node, center, normal) {
  return (center.x - node.position[0]) * normal.x + (center.y - node.position[1]) * normal.y;
}

async function saveWorldCrop(outputName, worldBounds) {
  await page.evaluate(() => {
    const debug = window.__chemcoreDebug;
    debug.state.editorEngine.clearInteraction();
    debug.syncDocument();
  });
  const clip = await page.evaluate((bounds) => {
    const debug = window.__chemcoreDebug;
    const topLeft = debug.worldToClient(bounds.x1, bounds.y1);
    const bottomRight = debug.worldToClient(bounds.x2, bounds.y2);
    return {
      x: Math.min(topLeft.x, bottomRight.x),
      y: Math.min(topLeft.y, bottomRight.y),
      width: Math.abs(bottomRight.x - topLeft.x),
      height: Math.abs(bottomRight.y - topLeft.y),
    };
  }, worldBounds);
  await page.screenshot({
    path: path.join(outputDir, outputName),
    clip: {
      x: Math.max(0, clip.x),
      y: Math.max(0, clip.y),
      width: Math.max(1, clip.width),
      height: Math.max(1, clip.height),
    },
  });
}

function sceneBounds(fragment, focusNodeId, padding = 18) {
  const node = fragment.nodes.find((candidate) => candidate.id === focusNodeId);
  const { bond } = bondAndNormal(fragment, focusNodeId);
  const bondNodes = [
    fragment.nodes.find((candidate) => candidate.id === bond.begin),
    fragment.nodes.find((candidate) => candidate.id === bond.end),
  ];
  const labelBox = node.label?.boxField || node.label?.box;
  const xs = bondNodes.map((candidate) => candidate.position[0]);
  const ys = bondNodes.map((candidate) => candidate.position[1]);
  if (labelBox) {
    xs.push(labelBox[0], labelBox[2]);
    ys.push(labelBox[1], labelBox[3]);
  }
  return {
    x1: Math.min(...xs) - padding,
    y1: Math.min(...ys) - padding,
    x2: Math.max(...xs) + padding,
    y2: Math.max(...ys) + padding,
  };
}

logStep("goto");
await waitForReady();

logStep("terminal-side-double");
await resetDocument();
let fragment = await engineEval({
  action: "drawBond",
  bondVariant: "double",
  start: { x: 100, y: 100 },
  end: { x: 140, y: 100 },
});
let focusNode = terminalNode(fragment);
fragment = await engineEval({
  action: "replaceLabel",
  nodeId: focusNode.id,
  label: "O",
});
focusNode = fragment.nodes.find((candidate) => candidate.id === focusNode.id);
let { bond, normal } = bondAndNormal(fragment, focusNode.id);
let center = labelCenter(focusNode);
let projection = normalProjection(focusNode, center, normal);
assert.equal(bond.double?.placement, "right");
assert(projection > 0.4, `terminal side double label did not move to side line midpoint: ${projection}`);
await saveWorldCrop("terminal-side-double.png", sceneBounds(fragment, focusNode.id));

logStep("center-double");
await resetDocument();
fragment = await engineEval({
  action: "drawBond",
  bondVariant: "double",
  start: { x: 100, y: 100 },
  end: { x: 140, y: 100 },
});
focusNode = terminalNode(fragment);
const centerBondId = fragment.bonds[0].id;
fragment = await engineEval({
  action: "cycleBondToCenter",
  bondId: centerBondId,
});
fragment = await engineEval({
  action: "replaceLabel",
  nodeId: focusNode.id,
  label: "O",
});
focusNode = fragment.nodes.find((candidate) => candidate.id === focusNode.id);
({ bond, normal } = bondAndNormal(fragment, focusNode.id));
center = labelCenter(focusNode);
projection = normalProjection(focusNode, center, normal);
assert.equal(bond.double?.placement, "center");
assert(Math.abs(projection) < 0.25, `center double label should stay on main bond anchor: ${projection}`);
await saveWorldCrop("center-double.png", sceneBounds(fragment, focusNode.id));

logStep("branched-double");
await resetDocument();
fragment = await engineEval({
  action: "drawBond",
  bondVariant: "double",
  start: { x: 100, y: 100 },
  end: { x: 140, y: 100 },
});
focusNode = terminalNode(fragment);
fragment = await engineEval({
  action: "replaceLabel",
  nodeId: focusNode.id,
  label: "O",
});
fragment = await engineEval({
  action: "addBranchFromLabel",
  nodeId: focusNode.id,
  end: { x: 172, y: 128 },
});
focusNode = fragment.nodes.find((candidate) => candidate.id === focusNode.id);
({ normal } = bondAndNormal(fragment, focusNode.id));
center = labelCenter(focusNode);
projection = normalProjection(focusNode, center, normal);
assert(Math.abs(projection) < 0.25, `branched double label should fall back to main bond anchor: ${projection}`);
await saveWorldCrop("branched-double.png", sceneBounds(fragment, focusNode.id, 24));

await browser.close();

console.log(path.join(outputDir, "terminal-side-double.png"));
console.log(path.join(outputDir, "center-double.png"));
console.log(path.join(outputDir, "branched-double.png"));
