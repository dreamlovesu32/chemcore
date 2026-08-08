import assert from "node:assert/strict";
import { join } from "node:path";
import test from "node:test";
import { gzipSync } from "node:zlib";
import { ProductionBlackBoxDriver } from "../src/drivers/production-black-box.mjs";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { runScenario } from "../src/runner/run-scenario.mjs";

function foreground() {
  return {
    processId: 40,
    executable: "C:\\ChemSemaGuiTest\\candidate\\abc\\chemsema-desktop.exe",
    clientRect: [8, 1, 1036, 780],
  };
}

test("production black-box driver maps semantic CDP targets to guarded OS input", async () => {
  let bonds = 0;
  let stopCount = 0;
  let autologonConfigured = false;
  let blockerVisible = true;
  let blockerDismissals = 0;
  let cdpAgentStarts = 0;
  let cdpAgentStops = 0;
  const cdpModes = [];
  const inputs = [];
  const coordinator = {
    async reset() { return { state: "Off" }; },
    async start() { return { vmId: "worker-id" }; },
    async installAgent() { return {}; },
    async configureDesktopBaseline() { return { baseline: { changed: false } }; },
    async attestGuest() { return { guest: { interactiveAccountMatches: autologonConfigured } }; },
    async configureAutologon() { autologonConfigured = true; return {}; },
    async installCandidate() { return { candidate: { sha256: "abc" } }; },
    async launchCandidate() { return { candidate: { guestPath: foreground().executable, sha256: "abc" } }; },
    async attestInteractiveAgent() {
      return { agent: { foreground: blockerVisible ? {
        executable: "C:\\Windows\\System32\\WWAHost.exe",
        title: "Microsoft 账户",
        className: "Windows.UI.Core.CoreWindow",
        clientRect: [0, 0, 1024, 768],
      } : foreground() } };
    },
    async dismissKnownBlocker() { blockerVisible = false; blockerDismissals += 1; return {}; },
    async activateCandidate() { return { agent: { foreground: foreground() } }; },
    async startInputAgent() { return { agent: { status: "ready" } }; },
    async stopInputAgent() { return {}; },
    async startCdpAgent() { cdpAgentStarts += 1; return { agent: { status: "ready" } }; },
    async stopCdpAgent() { cdpAgentStops += 1; return {}; },
    async prepareDocumentOutput() { return { id: "0".repeat(32), name: "roundtrip.ccjs", guestPath: "C:\\ChemSemaGuiTest\\documents\\00000000000000000000000000000000\\roundtrip.ccjs", exists: false }; },
    async cdpBridge(request) {
      cdpModes.push(request.mode);
      if (request.mode === "trace-start") return { started: true, categories: "devtools.timeline" };
      if (request.mode === "artifact-export") {
        return {
          schema: "chemsema.gui.guest-artifact-export.v1",
          artifactId: request.artifactId,
          artifacts: [
            "final-screenshot.png",
            "final-state.json",
            "final-dom.html",
            "performance-trace.json.gz",
            "webview.log",
          ].map((name) => ({ name, truncated: false })),
        };
      }
      if (request.mode === "state") {
        return { runtimeState: "ready", revision: bonds, window: { title: bonds ? "Untitled *" : "Untitled" }, viewport: { width: 1028, height: 779 }, rendered: { bonds, nodes: 0 } };
      }
      if (request.mode === "count") return bonds;
      if (request.mode === "count-state") {
        return { count: bonds, state: { revision: bonds, window: { title: bonds ? "Untitled *" : "Untitled" }, rendered: { bonds, nodes: 0 } } };
      }
      if (request.target.value === "viewer-container") {
        return { scopeCount: null, matches: [{ visible: true, disabled: false, rect: [52, 136, 1028, 739] }] };
      }
      return { scopeCount: 1, matches: [{ visible: true, disabled: false, rect: [4, 191, 48, 231] }] };
    },
    async fetchArtifacts() {
      return [
        { name: "final-screenshot.png", mediaType: "image/png", bytes: Buffer.from("png") },
        { name: "final-state.json", mediaType: "application/json", bytes: Buffer.from("{}") },
        { name: "final-dom.html", mediaType: "text/html", bytes: Buffer.from("<html></html>") },
        { name: "performance-trace.json.gz", mediaType: "application/gzip", bytes: gzipSync(Buffer.from('{"traceEvents":[{"name":"test"}]}')) },
        { name: "webview.log", mediaType: "text/plain", bytes: Buffer.from("candidate log\n") },
      ];
    },
    async candidateInput(kind, coordinates) {
      inputs.push({ kind, coordinates });
      if (kind === "drag") bonds = 1;
      return { agent: { foreground: foreground() } };
    },
    async candidateAction(input, completion) {
      const before = { revision: bonds, window: { title: bonds ? "Untitled *" : "Untitled" }, rendered: { bonds, nodes: 0 } };
      inputs.push(input);
      if (input.kind === "drag") bonds = 1;
      const after = { revision: bonds, window: { title: bonds ? "Untitled *" : "Untitled" }, rendered: { bonds, nodes: 0 } };
      return {
        transaction: {
          input: { foreground: foreground() },
          before,
          after,
          completion: completion.kind === "dom-count" ? { observed: bonds } : { actionable: true },
        },
      };
    },
    async stop() { stopCount += 1; return {}; },
  };
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond-production.json"));
  const { report, artifactPayloads } = await runScenario({ scenario, driver: new ProductionBlackBoxDriver({ coordinator }) });
  assert.equal(report.status, "passed");
  assert.deepEqual(inputs[0], { kind: "click", x: 34, y: 212, button: "left" });
  assert.deepEqual(inputs[1], { kind: "drag", from: [480, 439], to: [577, 439], steps: 8, button: "left" });
  assert.equal(stopCount, 2);
  assert.equal(autologonConfigured, true);
  assert.equal(blockerDismissals, 1);
  assert.equal(cdpAgentStarts, 1);
  assert.equal(cdpAgentStops, 1);
  assert(cdpModes.indexOf("trace-start") > cdpModes.indexOf("state"));
  assert(cdpModes.indexOf("trace-start") < cdpModes.indexOf("locate"));
  assert(cdpModes.indexOf("artifact-export") > cdpModes.indexOf("trace-start"));
  assert.deepEqual(report.artifacts.map((artifact) => artifact.name), [
    "final-screenshot.png",
    "final-state.json",
    "final-dom.html",
    "performance-trace.json.gz",
    "webview.log",
  ]);
  assert.equal(artifactPayloads.length, 5);
});

test("production artifact collection fails closed on truncated evidence", async () => {
  const driver = new ProductionBlackBoxDriver({
    coordinator: {
      async cdpBridge() {
        return {
          schema: "chemsema.gui.guest-artifact-export.v1",
          artifactId: "0".repeat(32),
          artifacts: [{ name: "final-dom.html", truncated: true }],
        };
      },
    },
  });
  await assert.rejects(driver.collectArtifacts(), /truncated required payloads: final-dom.html/);
});

test("native dialog completion refreshes candidate geometry before the next WebView action", async () => {
  let activations = 0;
  const nativeForeground = { ...foreground(), clientRect: [8, 31, 617, 472], className: "#32770" };
  const coordinator = {
    async cdpBridge(request) {
      if (request.mode === "state") return { revision: 1, window: {}, rendered: { bonds: 1 } };
      if (request.mode === "count-state") return { count: 1, state: { revision: 1, window: {}, rendered: { bonds: 1 } } };
      throw new Error(`Unexpected CDP mode ${request.mode}`);
    },
    async candidateInput() { return { agent: { foreground: nativeForeground } }; },
    async queryUiaByAutomationId() { return { query: { topLevels: [], matches: [] } }; },
    async attestInteractiveAgent() { activations += 1; return { agent: { foreground: foreground() } }; },
  };
  const driver = new ProductionBlackBoxDriver({ coordinator });
  driver.foreground = nativeForeground;
  driver.lastActionState = { revision: 1, window: { title: "roundtrip.ccjs - ChemSema" }, rendered: { bonds: 1 } };
  driver.targets.set(JSON.stringify({ strategy: "uia-automation-id", value: "1148", controlType: "ControlType.Pane", className: "Edit" }), {
    topLevelName: "打开",
    topLevelClassName: "#32770",
  });
  await driver.executeAction({
    id: "confirm-open",
    type: "key",
    key: "Enter",
    target: { strategy: "uia-automation-id", value: "1148", controlType: "ControlType.Pane", className: "Edit" },
    completion: { kind: "dom-count", selector: "[data-bond-id]", operator: "eq", value: 1, timeoutMs: 1000 },
    budgetMs: 5000,
  });
  assert.equal(activations, 1);
  assert.deepEqual(driver.foreground.clientRect, foreground().clientRect);
});

test("native dialog editing never probes the modal WebView through CDP", async () => {
  const stableState = { revision: 3, window: { title: "Untitled * - ChemSema" }, rendered: { bonds: 1 } };
  const driver = new ProductionBlackBoxDriver({
    coordinator: {
      async cdpBridge() { throw new Error("CDP must not be called while a native modal dialog is open"); },
      async candidateInput() { return { agent: { foreground: { ...foreground(), className: "#32770" } } }; },
    },
  });
  driver.lastActionState = stableState;
  const result = await driver.executeAction({
    id: "select-save-filename",
    type: "key",
    key: "Control+A",
    target: { strategy: "uia-automation-id", value: "1001", controlType: "ControlType.Pane", className: "Edit" },
    completion: { kind: "actionable", timeoutMs: 1000 },
    budgetMs: 5000,
  });
  assert.deepEqual(result.before, stableState);
  assert.deepEqual(result.after, stableState);
});

test("production artifact collection fails closed on a malformed performance trace", async () => {
  const artifacts = ["final-screenshot.png", "final-state.json", "final-dom.html", "performance-trace.json.gz", "webview.log"];
  const driver = new ProductionBlackBoxDriver({
    coordinator: {
      async cdpBridge(request) {
        return {
          schema: "chemsema.gui.guest-artifact-export.v1",
          artifactId: request.artifactId,
          artifacts: artifacts.map((name) => ({ name, truncated: false })),
        };
      },
      async fetchArtifacts() {
        return artifacts.map((name) => ({ name, bytes: Buffer.from(name === "performance-trace.json.gz" ? "not-gzip" : "evidence") }));
      },
    },
  });
  await assert.rejects(driver.collectArtifacts(), /performance trace is not valid bounded gzip JSON/);
});
