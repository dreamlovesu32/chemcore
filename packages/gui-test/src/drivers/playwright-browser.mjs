import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import net from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { launchBrowser } from "../../../../scripts/playwright-browser.mjs";
import { repositoryRoot } from "../protocol/paths.mjs";

function waitForPort(host, port, timeoutMs = 10000) {
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
          reject(new Error(`Timed out waiting for ${host}:${port}.`));
        } else {
          setTimeout(attempt, 100);
        }
      });
    };
    attempt();
  });
}

async function portIsOpen(host, port) {
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

export class PlaywrightBrowserDriver {
  constructor() {
    this.name = "playwright-browser";
    this.diagnostics = [];
    this.consoleMessages = [];
  }

  async prepare(profile) {
    this.profile = profile;
  }

  async launch(candidate = {}) {
    const host = candidate.host || "127.0.0.1";
    const port = Number(candidate.port || process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
    if (!await portIsOpen(host, port)) {
      this.server = spawn(process.execPath, ["scripts/desktop-dev-server.mjs"], {
        cwd: repositoryRoot,
        stdio: "ignore",
        windowsHide: true,
        env: { ...process.env, CHEMSEMA_DESKTOP_DEV_PORT: String(port) },
      });
      await waitForPort(host, port);
    }
    this.browser = await launchBrowser({ headless: true });
    this.page = await this.browser.newPage({ viewport: this.profile.viewport });
    await this.page.context().tracing.start({ screenshots: true, snapshots: true, sources: false });
    this.traceStarted = true;
    this.page.setDefaultTimeout(30000);
    this.page.on("console", (message) => {
      this.consoleMessages.push({ type: message.type(), text: message.text(), location: message.location() });
      if (message.type() === "error") {
        this.diagnostics.push(message.text());
      }
    });
    this.page.on("pageerror", (error) => {
      this.consoleMessages.push({ type: "pageerror", text: error.message, stack: error.stack || null });
      this.diagnostics.push(error.message);
    });
    await this.page.goto(candidate.url || `http://${host}:${port}/viewer/?gui-test=${Date.now()}`, { waitUntil: "domcontentloaded" });
    await this.page.waitForFunction(() => document.body.dataset.runtimeState === "ready", null, { timeout: 30000 });
    await this.page.waitForFunction(
      () => !!window.__chemsemaDebug?.state?.editorEngine && !!window.__chemsemaDebug?.document,
      null,
      { timeout: 30000 },
    );
  }

  capabilities() {
    return ["gui.public-input", "editor.bond.draw", "oracle.dom", "oracle.diagnostics"];
  }

  locatorFor(target) {
    const scope = target.scope
      ? this.page.getByRole(target.scope.role, { name: target.scope.name, exact: true })
      : this.page;
    if (target.strategy === "role") {
      return scope.getByRole(target.value, target.name ? { name: target.name, exact: true } : {});
    }
    if (target.strategy === "automation-id" || target.strategy === "test-id") {
      const id = target.value.replaceAll('"', '\\"');
      return scope.locator(`[id="${id}"]`);
    }
    throw new Error(`Playwright browser cannot resolve target strategy ${target.strategy}.`);
  }

  async resolve(target) {
    const locator = this.locatorFor(target);
    try {
      await locator.waitFor({ state: "visible", timeout: 12000 });
    } catch (error) {
      const diagnostic = await this.page.evaluate(() => ({
        runtimeState: document.body.dataset.runtimeState || null,
        editorHidden: document.querySelector(".editor-shell")?.hidden ?? null,
        bondButtons: [...document.querySelectorAll('button[aria-label="Bond"]')].map((button) => ({
          hidden: button.hidden,
          display: getComputedStyle(button).display,
          visibility: getComputedStyle(button).visibility,
          rect: button.getBoundingClientRect().toJSON(),
          outerHTML: button.outerHTML,
        })),
        toolButtons: [...document.querySelectorAll("button[data-tool]")].map((button) => ({
          tool: button.dataset.tool || null,
          ariaLabel: button.getAttribute("aria-label"),
          hidden: button.hidden,
          display: getComputedStyle(button).display,
        })),
      }));
      throw new Error(`${error.message}\nResolution diagnostic: ${JSON.stringify(diagnostic)}`);
    }
    return { target, count: await locator.count() };
  }

  async perform(action) {
    const locator = this.locatorFor(action.target);
    if (action.type === "click") {
      await locator.click({ button: action.button || "left" });
      return { kind: "click" };
    }
    if (action.type === "key") {
      await locator.press(action.key);
      return { kind: "key", key: action.key };
    }
    if (action.type === "drag") {
      const box = await locator.boundingBox();
      if (!box) {
        throw new Error(`Drag target ${action.target.value} has no visible bounding box.`);
      }
      const from = { x: box.x + box.width * action.from.x, y: box.y + box.height * action.from.y };
      const to = { x: box.x + box.width * action.to.x, y: box.y + box.height * action.to.y };
      await this.page.mouse.move(from.x, from.y);
      await this.page.mouse.down({ button: action.button || "left" });
      await this.page.mouse.move(to.x, to.y, { steps: action.steps });
      await this.page.mouse.up({ button: action.button || "left" });
      return { kind: "drag", from, to };
    }
    throw new Error(`Unsupported Playwright action ${action.type}.`);
  }

  async actionState() {
    return this.page.evaluate(() => ({
      revision: Number.isInteger(window.__chemsemaDebug?.state?.revision)
        ? window.__chemsemaDebug.state.revision
        : null,
      window: {
        href: location.href,
        title: document.title,
        visibilityState: document.visibilityState,
        focused: document.hasFocus(),
      },
      rendered: {
        bonds: document.querySelectorAll("[data-bond-id]").length,
        nodes: document.querySelectorAll("[data-node-id]").length,
      },
    }));
  }

  async waitForCompletion(completion) {
    if (completion.kind === "actionable") {
      return { actionable: true };
    }
    if (completion.kind === "quiescent") {
      await this.page.waitForTimeout(0);
      return { quiescent: true };
    }
    if (completion.kind === "dom-count") {
      await this.page.waitForFunction(
        ({ selector, operator, value }) => {
          const count = document.querySelectorAll(selector).length;
          return operator === "eq" ? count === value : count >= value;
        },
        completion,
        { timeout: completion.timeoutMs },
      );
      return { observed: await this.page.locator(completion.selector).count() };
    }
    if (completion.kind === "dom-distinct-count") {
      await this.page.waitForFunction(
        ({ selector, attribute, operator, value }) => {
          const count = new Set([...document.querySelectorAll(selector)]
            .map((element) => element.getAttribute(attribute))
            .filter(Boolean)).size;
          return operator === "eq" ? count === value : count >= value;
        },
        completion,
        { timeout: completion.timeoutMs },
      );
      return {
        observed: await this.page.locator(completion.selector).evaluateAll(
          (elements, attribute) => new Set(elements.map((element) => element.getAttribute(attribute)).filter(Boolean)).size,
          completion.attribute,
        ),
      };
    }
    throw new Error(`Unsupported completion ${completion.kind}.`);
  }

  async observe(oracle) {
    if (oracle.kind === "dom-count") {
      return this.page.locator(oracle.selector).count();
    }
    if (oracle.kind === "dom-distinct-count") {
      return this.page.locator(oracle.selector).evaluateAll(
        (elements, attribute) => new Set(elements.map((element) => element.getAttribute(attribute)).filter(Boolean)).size,
        oracle.attribute,
      );
    }
    if (oracle.kind === "no-unexpected-diagnostics") {
      return [...this.diagnostics];
    }
    throw new Error(`Unsupported Playwright oracle ${oracle.kind}.`);
  }

  async environment() {
    return {
      platform: process.platform,
      node: process.version,
      browser: await this.browser.version(),
      profile: this.profile,
    };
  }

  async collectArtifacts() {
    const traceRoot = await mkdtemp(join(tmpdir(), "chemsema-playwright-trace-"));
    const tracePath = join(traceRoot, "playwright-trace.zip");
    try {
      const [screenshot, domHtml, documentJson, state] = await Promise.all([
        this.page.screenshot({ type: "png" }),
        this.page.content(),
        this.page.evaluate(() => window.__chemsemaDebug?.state?.editorEngine?.documentJson?.() || ""),
        this.actionState(),
      ]);
      if (this.traceStarted) {
        await this.page.context().tracing.stop({ path: tracePath });
        this.traceStarted = false;
      }
      const trace = await readFile(tracePath);
      const json = (value) => Buffer.from(`${JSON.stringify(value, null, 2)}\n`, "utf8");
      return [
        { name: "final-screenshot.png", mediaType: "image/png", bytes: screenshot },
        { name: "final-state.json", mediaType: "application/json", bytes: json({ schema: "chemsema.gui.browser-snapshot.v1", state }) },
        { name: "final-dom.html", mediaType: "text/html", bytes: Buffer.from(domHtml, "utf8") },
        { name: "document.ccjs.json", mediaType: "application/json", bytes: Buffer.from(documentJson, "utf8") },
        { name: "browser-console.json", mediaType: "application/json", bytes: json(this.consoleMessages) },
        { name: "playwright-trace.zip", mediaType: "application/zip", bytes: trace },
      ];
    } finally {
      await rm(traceRoot, { recursive: true, force: true });
    }
  }

  async shutdown() {
    await this.browser?.close();
    this.server?.kill();
  }
}
