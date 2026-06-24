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
    const docText = JSON.stringify(window.__chemcoreDebug.document || {});
    return {
      previewLeft: !!document.querySelector('[data-role^="preview-"]'),
      bondWords: (docText.match(/bond/g) || []).length,
      hasRenderedBond: /data-bond-id=/.test(document.querySelector("#viewer-svg")?.outerHTML || ""),
    };
  });
  await page.close();
  assert(hadPreview, "Bond drag did not show a preview.");
  assert(!result.previewLeft, "Bond preview remained after pointerup.");
  assert(result.bondWords >= 2 && result.hasRenderedBond, "Bond drag did not commit a rendered bond.");
  assert(!errors.length, `Viewer console errors during bond drawing: ${errors.join("\n")}`);
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
  await page.locator('button[data-tool="select"]').click();
  await page.mouse.move(target.x, target.y);
  await page.mouse.click(target.x, target.y);
  await page.waitForTimeout(120);
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

  await verifyLargeDragTarget(page, targets.label, "Label");
  await verifyLargeDragTarget(page, targets.atom, "Atom");
  await page.close();
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
  await verifyLargeFileHoverAndDrag(browser);
  console.log("[viewer-interaction-smoke] ok");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
