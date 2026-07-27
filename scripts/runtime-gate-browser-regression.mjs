import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_RUNTIME_GATE_PORT || 8771);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";

async function waitForServer(timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(baseUrl);
      if (response.ok) {
        return;
      }
    } catch {
      // The explicit timeout below owns the unavailable-server state.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`runtime gate server did not start at ${baseUrl}`);
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
  browser = await chromium.launch({
    headless: true,
    ...(existsSync(edgePath) ? { executablePath: edgePath } : {}),
  });

  const readyPage = await browser.newPage();
  await readyPage.goto(baseUrl, { waitUntil: "domcontentloaded" });
  await readyPage.locator('body[data-runtime-state="ready"]').waitFor();
  assert.equal(await readyPage.locator(".editor-shell").isVisible(), true);
  assert.equal(await readyPage.locator("#runtime-gate").isVisible(), false);

  const blockedContext = await browser.newContext();
  const blockedPage = await blockedContext.newPage();
  await blockedPage.route("**/chemsema_engine_bg.wasm*", (route) => route.abort("failed"));
  await blockedPage.goto(`${baseUrl}?backend=blocked`, { waitUntil: "domcontentloaded" });
  await blockedPage.locator('body[data-runtime-state="failed"]').waitFor();
  assert.equal(await blockedPage.locator(".editor-shell").isVisible(), false);
  assert.equal(await blockedPage.locator("#desktop-titlebar").isVisible(), false);
  assert.equal(await blockedPage.locator("#runtime-gate").isVisible(), true);
  assert.match(
    await blockedPage.locator("#runtime-gate-message").textContent(),
    /editor has been disabled/,
  );
  await blockedContext.close();

  console.log("[runtime-gate-browser-regression] ok");
} finally {
  await browser?.close();
  server.kill();
}
