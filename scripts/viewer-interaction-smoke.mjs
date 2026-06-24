import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const defaultLargeCdxml = "C:\\Users\\Dream\\OneDrive\\Desktop\\钯催化-jjb.cdxml";
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
  assert(!errors.length, `Viewer console errors during bond drawing: ${errors.join("\\n")}`);
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
  const target = await page.evaluate(() => [...document.querySelectorAll("[data-node-id]")]
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
      && entry.y < innerHeight - 80)[0]);
  assert(target, "Large CDXML did not expose a visible node target.");

  await page.mouse.move(target.x, target.y);
  await page.waitForTimeout(250);
  const hover = await page.evaluate(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    return overlay?.querySelectorAll('[data-role^="hover-"]').length || 0;
  });
  assert(hover > 0, "Large CDXML select hover did not render a hover overlay.");

  await page.mouse.down();
  await page.mouse.move(target.x + 24, target.y + 12, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(250);
  const afterDrag = await page.evaluate(() => {
    const overlay = document.querySelector('[data-layer="editor-overlay"]');
    return {
      hovers: overlay?.querySelectorAll('[data-role^="hover-"]').length || 0,
      previews: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
      gesture: window.__chemcoreDebug.state.activeSelectionGesture || null,
    };
  });
  await page.close();
  assert(afterDrag.previews === 0, "Drag left preview overlay behind.");
  assert(afterDrag.gesture === null, "Drag left an active selection gesture behind.");
  assert(!errors.length, `Viewer console errors during large-file hover: ${errors.join("\\n")}`);
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
