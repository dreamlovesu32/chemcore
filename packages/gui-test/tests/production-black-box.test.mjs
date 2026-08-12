import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { evaluateUiState, uiStateRequest } from "../src/oracles/ui-state.mjs";
import { gzipSync } from "node:zlib";
import { ProductionBlackBoxDriver, summarizePerformanceTrace } from "../src/drivers/production-black-box.mjs";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { runScenario } from "../src/runner/run-scenario.mjs";
import { selectionRectWithMinimumSize } from "../../../viewer/editor_overlay.js";

function foreground() {
  return {
    processId: 40,
    executable: "C:\\ChemSemaGuiTest\\candidate\\abc\\chemsema-desktop.exe",
    clientRect: [8, 1, 1036, 780],
  };
}

test("canvas focus and selection controls have explicit accessible frontend geometry", async () => {
  const [index, app, styles, overlay, contextMenu] = await Promise.all([
    readFile(new URL("../../../viewer/index.html", import.meta.url), "utf8"),
    readFile(new URL("../../../viewer/app.js", import.meta.url), "utf8"),
    readFile(new URL("../../../viewer/styles.css", import.meta.url), "utf8"),
    readFile(new URL("../../../viewer/editor_overlay.js", import.meta.url), "utf8"),
    readFile(new URL("../../../viewer/editor_context_menu.js", import.meta.url), "utf8"),
  ]);
  assert.match(index, /id="viewer-container"[^>]+role="application"[^>]+tabindex="0"/);
  assert.match(app, /viewerContainer\.focus\(\{ preventScroll: true \}\)/);
  assert.match(app, /focusCanvas: \(\) => viewerContainer\?\.focus\(\{ preventScroll: true \}\)/);
  assert.match(contextMenu, /contextMenuShouldRestoreCanvasFocus\(document\.activeElement, document\)/);
  assert.match(contextMenu, /options\.focusCanvas\?\.\(\)/);
  assert.match(styles, /\.viewer-container:focus-visible\s*\{[^}]*outline: 2px solid var\(--active\)/s);
  assert.match(overlay, /SELECTION_RESIZE_HANDLE_SCREEN_PX = 6/);
  assert.match(overlay, /SELECTION_BOND_BOX_MIN_SCREEN_PX = 12/);
  assert.match(overlay, /SELECTION_ROTATE_HANDLE_RADIUS_SCREEN_PX = 4/);
});

test("nested context submenus paint above overlapping ancestor-menu siblings", async () => {
  const styles = await readFile(new URL("../../../viewer/styles.css", import.meta.url), "utf8");
  assert.match(styles, /\.canvas-context-submenu\s*\{[^}]*position: absolute;[^}]*z-index: 1;/s);
});

test("bond selection boxes retain a centered minimum interactive size", () => {
  assert.deepEqual(
    selectionRectWithMinimumSize({ kind: "rect", role: "selection-bond", x: 10, y: 20, width: 2, height: 0 }, 12),
    { kind: "rect", role: "selection-bond", x: 5, y: 14, width: 12, height: 12 },
  );
  assert.deepEqual(
    selectionRectWithMinimumSize({ kind: "rect", role: "selection-bond", x: 10, y: 20, width: 30, height: 4 }, 12),
    { kind: "rect", role: "selection-bond", x: 10, y: 16, width: 30, height: 12 },
  );
});

test("production driver advertises every capability required by registered production scenarios", async () => {
  const scenarioRoot = join(guiTestsDir, "scenarios", "core");
  const driverCapabilities = new Set(new ProductionBlackBoxDriver({ coordinator: {} }).capabilities());
  const scenarioFiles = (await readdir(scenarioRoot)).filter((name) => name.endsWith(".json"));
  for (const name of scenarioFiles) {
    const scenario = await readValidatedDocument(join(scenarioRoot, name));
    if (!scenario.drivers.includes("production-black-box")) continue;
    const missing = scenario.capabilities.filter((capability) => !driverCapabilities.has(capability));
    assert.deepEqual(missing, [], `${scenario.id} requires unadvertised production capabilities`);
  }
});

test("an exact bond identity selector resolves the collective logical center across reordered rendered primitives", async () => {
  const primitives = [
    { tag: "line", visible: true, disabled: false, rect: [100, 200, 101, 201] },
    { tag: "line", visible: true, disabled: false, rect: [139, 204, 140, 205] },
    { tag: "polygon", visible: true, disabled: false, rect: [100, 204, 140, 205] },
  ];
  const driver = new ProductionBlackBoxDriver({ coordinator: {
    async cdpBridge() { return { scopeCount: null, matches: primitives }; },
  } });
  const exact = { strategy: "selector", value: '[data-role="document-bond"][data-bond-id="b_3"]' };
  assert.deepEqual((await driver.resolve(exact)).match, {
    ...primitives[0],
    rect: [100, 200, 140, 205],
  });
  await assert.rejects(
    driver.resolve({ strategy: "selector", value: "line.mol-bond-stroked" }),
    /resolved to 3 visible actionable elements/,
  );
});

test("an exact node identity selector resolves one logical atom across isotope label fragments", async () => {
  const fragments = [
    { tag: "text", name: "CH4", visible: true, disabled: false, rect: [661, 491, 686, 508] },
    { tag: "text", name: "2", visible: true, disabled: false, rect: [656, 485, 661, 493] },
  ];
  const driver = new ProductionBlackBoxDriver({ coordinator: {
    async cdpBridge() { return { scopeCount: null, matches: fragments }; },
  } });
  const exact = { strategy: "selector", value: '[data-node-id="n_1"]' };
  assert.deepEqual((await driver.resolve(exact)).match, {
    ...fragments[0],
    rect: [656, 485, 686, 508],
  });
  await assert.rejects(
    driver.resolve({ strategy: "selector", value: '[data-role="document-text"]' }),
    /resolved to 2 visible actionable elements/,
  );
});

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
  scenario.actions[1].modifiers = ["Shift"];
  const { report, artifactPayloads } = await runScenario({ scenario, driver: new ProductionBlackBoxDriver({ coordinator }) });
  assert.equal(report.status, "passed");
  assert.deepEqual(inputs[0], { kind: "click", x: 34, y: 212, button: "left" });
  assert.deepEqual(inputs[1], { kind: "drag", from: [480, 439], to: [577, 439], steps: 8, button: "left", modifiers: ["Shift"] });
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
    "performance-summary.json",
  ]);
  assert.equal(artifactPayloads.length, 6);
  const summary = JSON.parse(artifactPayloads.find((artifact) => artifact.descriptor.name === "performance-summary.json").bytes);
  assert.equal(summary.schema, "chemsema.gui.performance-summary.v1");
  assert.equal(summary.eventCount, 1);
});

test("performance trace summary retains bounded long-task and hotspot evidence", () => {
  const summary = summarizePerformanceTrace({ traceEvents: [
    { ph: "X", name: "RunTask", cat: "timeline", dur: 125_500, ts: 10, pid: 1, tid: 2 },
    { ph: "X", name: "Layout", cat: "timeline", dur: 75_250, ts: 20, pid: 1, tid: 2 },
    { ph: "X", name: "Paint", cat: "timeline", dur: 25_000, ts: 30, pid: 1, tid: 2 },
    { ph: "R", name: "chemsema-action:draw-bond:input-after", cat: "blink.user_timing", ts: 40, pid: 1, tid: 2 },
  ] });
  assert.equal(summary.eventCount, 4);
  assert.equal(summary.longTaskCount, 2);
  assert.equal(summary.maxLongTaskMs, 125.5);
  assert.deepEqual(summary.topLongTasks.map((event) => event.name), ["RunTask", "Layout"]);
  assert.deepEqual(summary.hotspots.find((entry) => entry.name === "Layout"), { name: "Layout", count: 1, totalMs: 75.25, maxMs: 75.25 });
  assert.deepEqual(summary.actionMarkers, [{ name: "chemsema-action:draw-bond:input-after", timestampUs: 40, processId: 1, threadId: 2 }]);
  assert.equal(summary.actionMarkersTruncated, false);
});

test("production black-box resolves bounded literal text without exposing its content in receipts", async () => {
  const driver = new ProductionBlackBoxDriver();
  const resolved = await driver.resolveActionInput({ type: "text", text: "H2SO4" });
  assert.deepEqual(resolved.input, { kind: "text", text: "H2SO4" });
  assert.deepEqual(resolved.result, { kind: "text", textLength: 5 });
});

test("replace-existing text physically targets, selects, and types before exact value completion", async () => {
  const inputs = [];
  const target = { strategy: "selector", value: '.atom-property-dialog input[name="value"]' };
  const driver = new ProductionBlackBoxDriver({ coordinator: {
    async candidateInput(kind, coordinates, options) {
      inputs.push({ kind, coordinates, options });
      return { agent: { foreground: foreground() } };
    },
  } });
  driver.foreground = foreground();
  driver.webviewState = { viewport: { width: 1028, height: 779 } };
  driver.targets.set(JSON.stringify(target), { rect: [300, 240, 500, 280] });
  const result = await driver.performResolvedInput(
    { kind: "text", text: "17", replaceExisting: true },
    { type: "text", text: "17", replaceExisting: true, target },
  );
  assert.deepEqual(inputs, [
    { kind: "click", coordinates: { x: 408, y: 261 }, options: { button: "left" } },
    { kind: "key", coordinates: { key: "Control+A" }, options: undefined },
    { kind: "text", coordinates: { text: "17" }, options: undefined },
  ]);
  assert.deepEqual(result, { kind: "text", textLength: 2, replaceExisting: true });
});

test("production black-box requires one exact DOM text match and retains its action state", async () => {
  const state = { revision: 7, appScript: "app.js", engine: null, window: { title: "Untitled *" }, rendered: { bonds: 0, nodes: 0 } };
  const requests = [];
  const driver = new ProductionBlackBoxDriver({
    coordinator: {
      async cdpBridge(request) {
        requests.push(request);
        return { count: 1, text: "Original text", state };
      },
    },
  });
  assert.deepEqual(
    await driver.waitForCompletion({ kind: "dom-text", selector: "[data-object-id=\"obj_text_1\"]", text: "Original text", timeoutMs: 100 }),
    { observedText: "Original text" },
  );
  assert.deepEqual(requests, [{ mode: "text-state", selector: "[data-object-id=\"obj_text_1\"]" }]);
  assert.deepEqual(await driver.actionState(), state);
});

test("production black-box maps relative click positions inside a semantic target", async () => {
  const target = { strategy: "automation-id", value: "viewer-container" };
  const driver = new ProductionBlackBoxDriver();
  driver.foreground = foreground();
  driver.webviewState = { viewport: { width: 1028, height: 779 } };
  driver.targets.set(JSON.stringify(target), { rect: [52, 136, 1028, 739] });
  const resolved = await driver.resolveActionInput({ type: "click", target, at: { x: 0.25, y: 0.75 } });
  assert.deepEqual(resolved.input, { kind: "click", x: 304, y: 589, button: "left" });
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

test("failed document inspection retains the transferred bytes and bounded diagnostic", async () => {
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
        return artifacts.map((name) => ({
          name,
          bytes: name === "performance-trace.json.gz"
            ? gzipSync(Buffer.from('{"traceEvents":[{"name":"test"}]}'))
            : Buffer.from(name),
        }));
      },
    },
  });
  driver.savedDocument = {
    transfer: { bytes: Buffer.from('{"schema":"chemsema.ccjs.v0.2"}'), size: 34, sha256: "a".repeat(64) },
    reports: null,
    inspectionError: new Error("chemical validation failed"),
  };
  const payloads = await driver.collectArtifacts();
  assert(payloads.some((artifact) => artifact.name === "saved-document.ccjs"));
  const diagnostic = payloads.find((artifact) => artifact.name === "saved-document-inspect-error.json");
  assert(diagnostic);
  assert.match(diagnostic.bytes.toString("utf8"), /chemical validation failed/);
});
test("UI state oracle evaluates focus hover disabled styles geometry and DPI without screenshots", () => {
  const expectation = {
    selector: ".selection-overlay",
    referenceSelector: "[data-object-id]",
    count: 1,
    visibleCount: 1,
    focusedCount: 1,
    focusWithinCount: 1,
    hoverCount: 1,
    disabledCount: 0,
    styles: [
      { property: "cursor", operator: "eq", value: "pointer" },
      { property: "boxShadow", operator: "neq", value: "none" },
    ],
    rect: { minWidth: 5, maxWidth: 20, minHeight: 5, maxHeight: 20 },
    geometry: { relation: "contains-reference", tolerancePx: 2 },
    viewport: { devicePixelRatio: 1.5, minWidth: 1000, minHeight: 700 },
  };
  assert.deepEqual(uiStateRequest(expectation), {
    mode: "ui-state",
    selector: ".selection-overlay",
    referenceSelector: "[data-object-id]",
    styleProperties: ["cursor", "boxShadow"],
  });
  const result = evaluateUiState({
    count: 1,
    visibleCount: 1,
    focusedCount: 1,
    focusWithinCount: 1,
    hoverCount: 1,
    disabledCount: 0,
    rects: [[8, 8, 18, 18]],
    unionRect: [8, 8, 18, 18],
    styleValues: { cursor: ["pointer"], boxShadow: ["rgb(1, 2, 3) 0px 0px 2px"] },
    reference: { unionRect: [10, 10, 16, 16], truncated: false },
    viewport: { width: 1028, height: 779, devicePixelRatio: 1.5 },
    truncated: false,
  }, expectation);
  assert.equal(result.passed, true, result.failures.join("\n"));
  assert.equal(evaluateUiState({ ...result.observed, hoverCount: 0 }, expectation).passed, false);
  assert.equal(evaluateUiState({ ...result.observed, truncated: true }, expectation).passed, false);
  assert.equal(evaluateUiState(result.observed, { ...expectation, geometry: { relation: "matches-reference", tolerancePx: 2 } }).passed, true);
  assert.equal(evaluateUiState({ ...result.observed, unionRect: [0, 0, 30, 30] }, { ...expectation, geometry: { relation: "matches-reference", tolerancePx: 2 } }).passed, false);
});
