import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { basename, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createEditorInteractionScenarios } from "./viewer_interaction/editor_scenarios.mjs";
import { createLargeDocumentInteractionScenarios } from "./viewer_interaction/large_document_scenarios.mjs";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const largeCdxml = process.env.CHEMSEMA_STABILITY_PRIVATE_CDXML || process.env.CHEMSEMA_INTERACTION_SMOKE_CDXML || "";
const ENDPOINT_FEEDBACK_RADIUS_PX = 4;

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
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?v=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    () => !!window.__chemsemaDebug?.state?.editorEngine,
    null,
    { timeout: 20000 },
  );
  return { page, errors };
}

function capturePageErrors(page, errors) {
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(error.message));
}

async function waitForLargeCdxmlContent(page) {
  await page.waitForFunction(() => {
    if (!window.__chemsemaDebug?.document) {
      return false;
    }
    if (document.querySelector('[data-layer="document-content"] [data-node-id], [data-layer="document-content"] [data-bond-id]')) {
      return true;
    }
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
    for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
      const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
      const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
      if ((fragment?.nodes?.length || 0) > 0 || (fragment?.bonds?.length || 0) > 0) {
        return true;
      }
    }
    return false;
  }, null, { timeout: 60000 });
}

async function openLargeCdxmlViewer(browser) {
  const opened = await openViewer(browser);
  const popupPromise = opened.page.waitForEvent("popup", { timeout: 5000 }).catch(() => null);
  await opened.page.locator('input[type="file"]').setInputFiles(largeCdxml);
  const popup = await popupPromise;
  if (!popup) {
    await waitForLargeCdxmlContent(opened.page);
    return opened;
  }
  capturePageErrors(popup, opened.errors);
  await popup.waitForLoadState("domcontentloaded");
  await popup.waitForFunction(() => !!window.__chemsemaDebug, null, { timeout: 20000 });
  await waitForLargeCdxmlContent(popup);
  return { page: popup, errors: opened.errors, sourcePage: opened.page };
}

const {
  verifyNativeTableToolWorkflow,
  verifyBondDrawing,
  verifyBondCreationUsesKernelLocalPreview,
  verifyElementEndpointPatchUpdatesConnectedBonds,
  verifyJunctionDragUsesBackendPrimitivePatch,
  verifyTransformedArrowRenderHitAndSelection,
  verifyCursorAnchoredWheelZoom,
  verifyQuickPaletteAndSelectDragRegression,
  verifyEndpointFeedbackRules,
  verifyGraphicObjectDragTracksPointerAndSelection,
  verifyCreationDragKeepsCanvasVisibleAfterToolSwitch,
  verifyDeleteToolTemporaryToolbarAndEmptyDocument,
  verifySelectedObjectSuppressesHover,
  verifyDragHandleCursors,
} = createEditorInteractionScenarios({
  assert,
  openViewer,
  ENDPOINT_FEEDBACK_RADIUS_PX,
});

const { verifyLargeFileHoverAndDrag } = createLargeDocumentInteractionScenarios({
  assert,
  openLargeCdxmlViewer,
  largeCdxml,
  existsSync,
  basename,
});

let server = null;
let browser = null;
try {
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  await verifyNativeTableToolWorkflow(browser);
  await verifyBondDrawing(browser);
  await verifyBondCreationUsesKernelLocalPreview(browser);
  await verifyElementEndpointPatchUpdatesConnectedBonds(browser);
  await verifyJunctionDragUsesBackendPrimitivePatch(browser);
  await verifyTransformedArrowRenderHitAndSelection(browser);
  await verifyCursorAnchoredWheelZoom(browser);
  await verifyQuickPaletteAndSelectDragRegression(browser);
  await verifyEndpointFeedbackRules(browser);
  await verifyGraphicObjectDragTracksPointerAndSelection(browser);
  await verifyCreationDragKeepsCanvasVisibleAfterToolSwitch(browser);
  await verifyDeleteToolTemporaryToolbarAndEmptyDocument(browser);
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
