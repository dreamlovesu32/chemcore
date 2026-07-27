import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import net from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";

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

function waitForPort(timeoutMs = 8000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const socket = net.connect({ host, port }, () => {
        socket.end();
        resolve();
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
  if (await portIsOpen()) return null;
  const child = spawn(process.execPath, ["scripts/desktop-dev-server.mjs"], {
    cwd: rootDir,
    stdio: "ignore",
    windowsHide: true,
  });
  await waitForPort();
  return child;
}

const server = await ensureServer();
const browser = await chromium.launch({
  headless: true,
  executablePath: edgePath,
});
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
page.on("console", (message) => {
  if (message.type() === "error") errors.push(message.text());
});

try {
  await page.goto(`${baseUrl}?plasmid=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    () => !!window.__chemsemaDebug?.state?.editorEngine && !!window.__chemsemaDebug?.document,
  );

  await page.locator("[data-tool-rail-toggle]").click();
  await page.locator('[data-tool-rail-toggle][aria-pressed="true"]').waitFor();
  assert.equal(
    await page.locator('[data-tool-rail="main"]:visible').count(),
    0,
    "switching rails should replace every main drawing tool",
  );
  assert.equal(
    await page.locator('[data-tool-rail="biology"]:visible').count(),
    10,
    "the Biology-Assisted Drawing Rail should expose every family",
  );
  assert.equal(
    await page.locator('button[data-tool="select"]:visible').count(),
    1,
    "selection should remain available in both rails",
  );

  await page.locator('button[data-tool="biodraw"][data-bio-family="enzyme"]').click();
  await page.locator('[data-secondary-value="biodraw-kind-two-substrate-enzyme"]').click();
  const canvas = page.locator("#viewer-container");
  const canvasBox = await canvas.boundingBox();
  assert.ok(canvasBox, "canvas should be measurable");
  await page.mouse.move(canvasBox.x + 540, canvasBox.y + 360);
  await page.mouse.down();
  await page.mouse.move(canvasBox.x + 640, canvasBox.y + 405, { steps: 6 });
  await page.mouse.up();
  await page.waitForFunction(() => {
    const documentValue = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return documentValue.objects?.some((object) =>
      object.payload?.bioShape?.kind === "two-substrate-enzyme");
  });

  await page.locator('button[data-tool="biodraw"][data-bio-family="plasmid"]').click();
  await page.waitForFunction(
    () => window.__chemsemaDebug?.editorState?.activeTool === "biodraw",
  );
  await page.waitForFunction(
    () => window.__chemsemaDebug?.editorState?.bioDrawKind === "plasmid-map",
  );
  assert.equal(
    await page.locator('[data-secondary-value="biodraw-kind-plasmid-map"]').count(),
    1,
  );

  await page.mouse.move(canvasBox.x + 820, canvasBox.y + 500);
  await page.mouse.down();
  await page.mouse.move(canvasBox.x + 880, canvasBox.y + 540, { steps: 5 });
  await page.mouse.up();
  const dialog = page.locator(".plasmid-map-dialog");
  await dialog.waitFor({ state: "visible" });
  await dialog.locator('[name="numberBasePairs"]').fill("12000");
  await dialog.locator('[data-plasmid-add="region"]').click();
  const region = dialog.locator('[data-plasmid-row="region"]').first();
  await region.locator('[name="start"]').fill("11000");
  await region.locator('[name="end"]').fill("1000");
  await region.locator('[name="arrowAtEnd"]').check();
  await region.locator('[name="fill"]').selectOption("shaded");
  await dialog.locator('[data-plasmid-add="marker"]').click();
  const marker = dialog.locator('[data-plasmid-row="marker"]').first();
  await marker.locator('[name="position"]').fill("1000");
  await marker.locator('[name="label"]').fill("ori");
  const nativeInvalid = await dialog.locator(":invalid").evaluateAll((elements) =>
    elements.map((element) => ({
      name: element.getAttribute("name"),
      value: element.value,
      message: element.validationMessage,
    })));
  assert.deepEqual(nativeInvalid, [], `native form validation failed: ${JSON.stringify(nativeInvalid)}`);
  await dialog.locator("form").evaluate((form) => form.requestSubmit());
  await dialog.waitFor({ state: "detached" });

  await page.waitForFunction(() => {
    const documentValue = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return documentValue.objects?.some((object) =>
      object.payload?.plasmidMap?.numberBasePairs === 12000);
  });
  const firstDocument = await page.evaluate(
    () => JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson()),
  );
  const mapObject = firstDocument.objects.find((object) => object.payload?.plasmidMap);
  assert.ok(mapObject, "creation should produce a native plasmid object");
  assert.equal(mapObject.payload.plasmidMap.regions[0].start, 1000);
  assert.equal(mapObject.payload.plasmidMap.regions[0].end, 11000);
  assert.equal(mapObject.payload.plasmidMap.regions[0].arrowAtEnd, true);
  assert.equal(mapObject.payload.plasmidMap.markers[0].label, "ori");

  const svg = await page.evaluate(
    () => window.__chemsemaDebug.state.editorEngine.documentSvg(),
  );
  assert.match(svg, />12000 bp</);
  assert.match(svg, />ori</);

  await page.locator('button[data-tool="select"]').click();
  const bounds = mapObject.payload.bbox;
  const center = await page.evaluate(
    ([object, box]) => window.__chemsemaDebug.worldToClient(
      object.transform.translate[0] + box[0] + box[2] / 2,
      object.transform.translate[1] + box[1] + box[3] / 2 - object.payload.plasmidMap.radius,
    ),
    [mapObject, bounds],
  );
  await page.mouse.click(center.x, center.y);
  await page.mouse.click(center.x, center.y, { button: "right" });
  const editItem = page.locator('[data-canvas-context-command="plasmid-map-dialog"]');
  await editItem.waitFor({ state: "visible" });
  await editItem.click();
  await dialog.waitFor({ state: "visible" });
  await dialog.locator('[name="showBasePairs"]').uncheck();
  await dialog.locator("form").evaluate((form) => form.requestSubmit());
  await page.waitForFunction(() => {
    const documentValue = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return documentValue.objects?.some((object) =>
      object.payload?.plasmidMap?.showBasePairs === false);
  });

  const selectionBeforeRailSwitch = await page.evaluate(
    async () => Promise.resolve(window.__chemsemaDebug.state.editorEngine.clipboardSelectionJson()),
  );
  assert.ok(selectionBeforeRailSwitch, "the edited plasmid should remain selected");
  await page.locator("[data-tool-rail-toggle]").click();
  await page.locator('[data-tool-rail-toggle][aria-pressed="false"]').waitFor();
  assert.equal(
    await page.locator('[data-tool-rail="main"]:visible').count(),
    11,
    "switching back should restore the complete Main Drawing Rail",
  );
  assert.equal(
    await page.evaluate(() => window.__chemsemaDebug.editorState.activeTool),
    "select",
    "rail switching should leave selection as the active tool",
  );
  const selectionAfterRailSwitch = await page.evaluate(
    async () => Promise.resolve(window.__chemsemaDebug.state.editorEngine.clipboardSelectionJson()),
  );
  assert.equal(
    selectionAfterRailSwitch,
    selectionBeforeRailSwitch,
    "rail switching must preserve the current document selection",
  );

  assert.deepEqual(errors, [], `browser errors: ${errors.join("\n")}`);
  console.log("[plasmid-map-regression] ok");
} finally {
  await browser.close();
  server?.kill();
}
