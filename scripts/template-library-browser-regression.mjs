import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { launchBrowser } from "./playwright-browser.mjs";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_TEMPLATE_LIBRARY_PORT || 8772);
const baseUrl = `http://${host}:${port}/viewer/`;

async function waitForServer(timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      if ((await fetch(baseUrl)).ok) return;
    } catch {
      // The deadline below owns the unavailable-server failure.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`template-library server did not start at ${baseUrl}`);
}

const server = spawn(process.execPath, ["scripts/desktop-dev-server.mjs"], {
  cwd: rootDir,
  env: {
    ...process.env,
    CHEMSEMA_DESKTOP_DEV_HOST: host,
    CHEMSEMA_DESKTOP_DEV_PORT: String(port),
  },
  stdio: "ignore",
  windowsHide: true,
});

let browser;
try {
  await waitForServer();
  browser = await launchBrowser({ headless: true });
  const context = await browser.newContext({
    acceptDownloads: true,
    viewport: { width: 1440, height: 1000 },
  });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await page.locator('body[data-runtime-state="ready"]').waitFor();
  await page.evaluate(() => localStorage.removeItem("chemsema.template-library-state.v2"));
  await page.locator("[data-template-rail-toggle]").click();
  const libraryButton = page.locator('[data-template-library-id="advanced-biodraw"]');
  await libraryButton.waitFor();
  await libraryButton.click();

  const grid = page.locator(".template-palette-grid");
  await grid.waitFor();
  assert.equal(await grid.evaluate((node) => node.style.getPropertyValue("--template-columns")), "6");
  assert.equal(await grid.locator("[data-template-cell]").count(), 48);
  assert.equal(await grid.locator(".template-palette-empty-cell").count(), 4);
  assert.ok(
    Number.parseFloat(await grid.evaluate((node) =>
      node.style.getPropertyValue("--template-pane-height"))) > 700,
    "PaneHeight should expose all eight Advanced BioDraw rows at the shared cell scale",
  );

  await libraryButton.click({ button: "right" });
  const dialog = page.locator(".template-grid-dialog");
  await dialog.waitFor();
  assert.equal(await dialog.locator('[name="rows"]').inputValue(), "8");
  assert.equal(await dialog.locator('[name="columns"]').inputValue(), "6");
  assert.equal(await dialog.locator('[name="paneHeight"]').inputValue(), "25.25");
  await dialog.locator('[name="rows"]').fill("7");
  await dialog.locator('[name="columns"]').fill("7");
  await dialog.locator('button[type="submit"]').click();
  await page.locator(".template-grid-dialog").waitFor({ state: "detached" });
  assert.equal(await grid.evaluate((node) => node.style.getPropertyValue("--template-columns")), "7");
  assert.equal(await grid.locator("[data-template-cell]").count(), 49);
  assert.equal(await grid.locator(".template-palette-empty-cell").count(), 5);

  const first = grid.locator("[data-template-cell]").first();
  const last = grid.locator("[data-template-cell]").last();
  assert.equal(await first.locator("[data-template-id]").count(), 1);
  assert.equal(await last.locator("[data-template-id]").count(), 0);
  await grid.evaluate((node) => {
    const cells = node.querySelectorAll("[data-template-cell]");
    const transfer = new DataTransfer();
    cells[0].dispatchEvent(new DragEvent("dragstart", {
      bubbles: true,
      cancelable: true,
      dataTransfer: transfer,
    }));
    cells[48].dispatchEvent(new DragEvent("dragover", {
      bubbles: true,
      cancelable: true,
      dataTransfer: transfer,
    }));
    cells[48].dispatchEvent(new DragEvent("drop", {
      bubbles: true,
      cancelable: true,
      dataTransfer: transfer,
    }));
  });
  await page.waitForFunction(() => {
    const cells = document.querySelectorAll(".template-palette-grid [data-template-cell]");
    return cells.length === 49 && !cells[0].querySelector("[data-template-id]")
      && cells[48].querySelector("[data-template-id]");
  });

  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("chemsema.template-library-state.v2") || "{}"));
  assert.equal(saved.layouts["advanced-biodraw"].rows, 7);
  assert.equal(saved.layouts["advanced-biodraw"].columns, 7);
  assert.equal(saved.layouts["advanced-biodraw"].cells[0], null);
  assert.equal(saved.layouts["advanced-biodraw"].cells[48], 0);
  const downloadPromise = page.waitForEvent("download");
  await page.locator("[data-template-export]").click();
  const download = await downloadPromise;
  assert.equal(download.suggestedFilename(), "Advanced BioDraw.cdxml");
  await page.locator("[data-template-reset]").click();
  assert.equal(await grid.evaluate((node) => node.style.getPropertyValue("--template-columns")), "6");
  assert.equal(await grid.locator(".template-palette-empty-cell").count(), 4);
  const reset = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("chemsema.template-library-state.v2") || "{}"));
  assert.equal(reset.layouts["advanced-biodraw"], undefined);
  assert.deepEqual(errors, [], `browser errors: ${errors.join("\n")}`);
  console.log("[template-library-browser-regression] ok");
} finally {
  await browser?.close();
  server.kill();
}
