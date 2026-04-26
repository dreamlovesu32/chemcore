import assert from "node:assert/strict";
import path from "node:path";
import { chromium } from "playwright";

const url = process.argv[2] || "http://127.0.0.1:8765/viewer/";
const output = process.argv[3] || path.resolve("tmp/text-editor-regression.png");

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1600, height: 1200 },
  deviceScaleFactor: 1.25,
});
page.setDefaultTimeout(8000);

const consoleMessages = [];
page.on("console", (message) => {
  const text = message.text();
  if (
    message.type() === "warning"
    && text.includes("WebAssembly.instantiateStreaming failed")
  ) {
    return;
  }
  consoleMessages.push(`[${message.type()}] ${text}`);
});

function logStep(label) {
  console.log(`STEP ${label}`);
}

function pointInBox(box, xRatio, yRatio) {
  return {
    x: box.x + box.width * xRatio,
    y: box.y + box.height * yRatio,
  };
}

async function clickTool(tool) {
  await page.click(`.tool-button[data-tool="${tool}"]`);
}

async function clientPointFromWorld(x, y) {
  return page.evaluate(({ worldX, worldY }) => {
    const svg = document.getElementById("viewer-svg");
    const point = new DOMPoint(worldX, worldY).matrixTransform(svg.getScreenCTM());
    return { x: point.x, y: point.y };
  }, { worldX: x, worldY: y });
}

async function currentEditorText() {
  return page.locator(".text-editor-display").innerText();
}

async function selectedEditorText() {
  return page.evaluate(() => Array.from(
    document.querySelectorAll(".text-editor-run.is-selected"),
    (node) => node.textContent || "",
  ).join(""));
}

logStep("goto");
await page.goto(url, { waitUntil: "networkidle" });
logStep("wait-engine");
await page.waitForFunction(() => window.__chemcoreDebug?.state?.editorEngine && window.__chemcoreDebug?.document);

logStep("new-doc");
await page.click('[data-command="new"]');
await page.waitForFunction(() => window.__chemcoreDebug?.document && Array.isArray(window.__chemcoreDebug.document.objects));

const svg = page.locator("#viewer-svg");
const viewerContainer = page.locator("#viewer-container");
const svgBox = await svg.boundingBox();
assert(svgBox, "viewer svg is not visible");

const bondStart = pointInBox(svgBox, 0.52, 0.58);
const bondEnd = { x: bondStart.x + 110, y: bondStart.y - 85 };
logStep("draw-bond");
await page.mouse.move(bondStart.x, bondStart.y);
await page.mouse.down();
await page.mouse.move(bondEnd.x, bondEnd.y, { steps: 12 });
await page.mouse.up();
await page.waitForFunction(() => window.__chemcoreDebug?.document?.objects?.length > 0);
const endpointWorld = await page.evaluate(() => {
  const point = window.__chemcoreDebug.engineState.document.resources.mol_editor.data.nodes[1].position;
  return { x: point[0], y: point[1] };
});
const endpointClient = await clientPointFromWorld(endpointWorld.x, endpointWorld.y);

logStep("open-label-editor");
await clickTool("text");
await page.mouse.click(endpointClient.x, endpointClient.y);
await page.waitForSelector(".text-editor");
assert.equal(await page.evaluate(() => window.__chemcoreDebug.activeTextEditor?.session?.target?.kind), "endpoint-label");
logStep("type-chemical");
assert.equal(await page.evaluate(() => window.__chemcoreDebug.insertEditorText("H2SO4")), true);
await page.waitForFunction(() => Array.from(
  document.querySelectorAll('.text-editor-display [data-script="subscript"]'),
  (node) => node.textContent || "",
).join("") === "24");
assert.equal(await currentEditorText(), "H2SO4");

logStep("commit-label");
const blankPoint = await clientPointFromWorld(220, 120);
await page.mouse.click(blankPoint.x, blankPoint.y);
await page.waitForFunction(() => Array.from(
  document.querySelectorAll("#viewer-svg text"),
  (node) => node.textContent || "",
).some((text) => text.includes("H2SO4")));

logStep("reopen-label");
const labelNode = page.locator("#viewer-svg text").filter({ hasText: "H2SO4" }).first();
await labelNode.waitFor();
const labelBox = await labelNode.boundingBox();
assert(labelBox, "committed label is not visible");
await page.mouse.click(labelBox.x + labelBox.width / 2, labelBox.y + labelBox.height / 2);
await page.waitForFunction(() => document.querySelector(".text-editor-display")?.textContent?.includes("H2SO4"));
assert.equal(await selectedEditorText(), "H2SO4");

logStep("zoom-editor");
const editorBeforeZoom = await page.locator(".text-editor").boundingBox();
assert(editorBeforeZoom, "text editor is not visible before zoom");
await page.$eval("#zoom-input", (input) => {
  input.value = "200%";
  input.dispatchEvent(new Event("change", { bubbles: true }));
});
await page.waitForTimeout(150);
const editorAfterZoom = await page.locator(".text-editor").boundingBox();
assert(editorAfterZoom, "text editor is not visible after zoom");
assert(
  editorAfterZoom.width > editorBeforeZoom.width * 1.5,
  `editor width did not scale with zoom: before=${editorBeforeZoom.width}, after=${editorAfterZoom.width}`,
);
assert(
  editorAfterZoom.height > editorBeforeZoom.height * 1.5,
  `editor height did not scale with zoom: before=${editorBeforeZoom.height}, after=${editorAfterZoom.height}`,
);

logStep("plain-text");
const visibleCanvasBox = await viewerContainer.boundingBox();
assert(visibleCanvasBox, "viewer container is not visible after zoom");
const plainTextPoint = pointInBox(visibleCanvasBox, 0.18, 0.32);
await page.mouse.click(plainTextPoint.x, plainTextPoint.y);
await page.waitForSelector(".text-editor");
await page.waitForFunction(() => window.__chemcoreDebug.activeTextEditor?.session?.target?.kind === "text-object");
assert.equal(await page.evaluate(() => window.__chemcoreDebug.insertEditorText("Hello")), true);
assert.equal(await currentEditorText(), "Hello");
const hasScriptedRuns = await page.evaluate(() => document.querySelector('.text-editor-display [data-script="subscript"], .text-editor-display [data-script="superscript"]') !== null);
assert.equal(hasScriptedRuns, false, "plain text editor unexpectedly applied chemical scripts");

logStep("commit-plain-text");
const secondBlankPoint = pointInBox(visibleCanvasBox, 0.72, 0.18);
await page.mouse.click(secondBlankPoint.x, secondBlankPoint.y);
await page.waitForFunction(() => Array.from(
  document.querySelectorAll("#viewer-svg text"),
  (node) => node.textContent || "",
).some((text) => text.includes("Hello")));

const plainTextNode = page.locator("#viewer-svg text").filter({ hasText: "Hello" }).first();
await plainTextNode.waitFor();
const plainTextBox = await plainTextNode.boundingBox();
assert(plainTextBox, "committed plain text is not visible");
await page.mouse.click(plainTextBox.x + plainTextBox.width / 2, plainTextBox.y + plainTextBox.height / 2);
await page.waitForFunction(() => document.querySelector(".text-editor-display")?.textContent?.includes("Hello"));
assert.equal(await selectedEditorText(), "Hello");

logStep("screenshot");
await page.screenshot({ path: output, fullPage: true });
await browser.close();

if (consoleMessages.length) {
  console.log(consoleMessages.join("\n"));
}
console.log(output);
