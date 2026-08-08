import assert from "node:assert/strict";
import { join } from "node:path";
import test from "node:test";
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
  const inputs = [];
  const coordinator = {
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
    async cdpBridge(request) {
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
    async candidateInput(kind, coordinates) {
      inputs.push({ kind, coordinates });
      if (kind === "drag") bonds = 1;
      return { agent: { foreground: foreground() } };
    },
    async stop() { stopCount += 1; return {}; },
  };
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond-production.json"));
  const report = await runScenario({ scenario, driver: new ProductionBlackBoxDriver({ coordinator }) });
  assert.equal(report.status, "passed");
  assert.deepEqual(inputs[0], { kind: "click", coordinates: { x: 34, y: 212 } });
  assert.deepEqual(inputs[1], { kind: "drag", coordinates: { from: [480, 439], to: [577, 439] } });
  assert.equal(stopCount, 2);
  assert.equal(autologonConfigured, true);
  assert.equal(blockerDismissals, 1);
});
