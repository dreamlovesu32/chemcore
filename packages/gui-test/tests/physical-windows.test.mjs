import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { createWorkerCoordinator } from "../src/workers/create.mjs";
import { readScenarioReport, renameAtomicWithRetry, validatePhysicalQueue } from "../src/workers/physical-daemon.mjs";
import { HyperVCoordinator } from "../src/workers/hyperv.mjs";
import { PhysicalWindowsCoordinator } from "../src/workers/physical-windows.mjs";

const physicalProfile = {
  schema: "chemsema.gui.worker-profile.v1",
  id: "physical-worker-test",
  kind: "physical-windows",
  machine: {
    computerName: "GUI-WORKER",
    machineIdSha256: "a".repeat(64),
  },
  resources: { cpuUnits: 12, memoryGiB: 32 },
  account: { name: "GUI-WORKER\\tester", sessionId: 1 },
  testRootTemplate: "%LOCALAPPDATA%\\ChemSema\\gui-test\\physical-worker-test",
  isolation: {
    dedicatedPhysicalMachine: true,
    hostInput: true,
    hostClipboard: true,
    hostProjectMount: false,
    interactiveDesktopExclusive: true,
  },
};

test("worker factory keeps Hyper-V and physical Windows as distinct adapters", () => {
  const physical = createWorkerCoordinator(physicalProfile, { candidateVerifier: () => null });
  assert.ok(physical instanceof PhysicalWindowsCoordinator);
  const hyperV = createWorkerCoordinator({ kind: "hyper-v" }, { candidateVerifier: () => null });
  assert.ok(hyperV instanceof HyperVCoordinator);
  assert.throws(() => createWorkerCoordinator({ kind: "physical" }), /Unsupported GUI worker kind/);
});

test("physical worker profile validates a dedicated real desktop without credentials or VM identity", async () => {
  const coordinator = new PhysicalWindowsCoordinator(physicalProfile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    candidateVerifier: () => null,
  });
  await coordinator.validateProfile();
  assert.equal(coordinator.testRoot, "C:\\Users\\tester\\AppData\\Local\\ChemSema\\gui-test\\physical-worker-test");
  assert.equal("credential" in physicalProfile, false);
  assert.equal("vm" in physicalProfile, false);
});

test("physical worker source binds input to account session process executable and bounded run root", async () => {
  const source = await readFile(new URL("../src/workers/physical-windows.mjs", import.meta.url), "utf8");
  assert.match(source, /expectedAgentAccount: this\.profile\.account\.name/);
  assert.match(source, /expectedAgentSessionId: this\.profile\.account\.sessionId/);
  assert.match(source, /expectedProcessId: process\.processId/);
  assert.match(source, /expectedExecutable: process\.guestPath/);
  assert.match(source, /allowedRunRoot: this\.runsRoot/);
  assert.doesNotMatch(source, /new HyperVCoordinator/);
});

test("physical worker uses persistent file channels and detached OS processes", async () => {
  const source = await readFile(new URL("../src/workers/physical-windows.mjs", import.meta.url), "utf8");
  assert.match(source, /detached: true/);
  assert.match(source, /chemsema\.gui\.guest-agent-request\.v1/);
  assert.match(source, /chemsema\.gui\.cdp-request\.v1/);
  assert.match(source, /taskkill\.exe/);
  assert.match(source, /physical-background-launch\.ps1/);
  assert.match(source, /SHA256\]::Create/);
  assert.doesNotMatch(source, /SHA256\]::HashData/);
});

test("persistent CDP evidence hashing does not depend on PowerShell module auto-loading", async () => {
  const source = await readFile(new URL("../scripts/guest-cdp.ps1", import.meta.url), "utf8");
  assert.match(source, /SHA256\]::Create/);
  assert.doesNotMatch(source, /Get-FileHash/);
});

test("persistent CDP JSON parsing preserves keys that differ only by casing", async () => {
  const source = await readFile(new URL("../scripts/guest-cdp.ps1", import.meta.url), "utf8");
  assert.match(source, /System\.Web\.Script\.Serialization\.JavaScriptSerializer/);
  assert.match(source, /CdpJsonSerializer\.DeserializeObject/);
  assert.doesNotMatch(source, /GetString\(\$stream\.ToArray\(\)\) \| ConvertFrom-Json/);
});

test("physical CDP UI observations are bounded and style-allowlisted", async () => {
  const coordinator = new PhysicalWindowsCoordinator(physicalProfile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    candidateVerifier: () => null,
  });
  coordinator.validateCdpRequest({ mode: "ui-state", selector: "#viewer-container", referenceSelector: "[data-bond-id]", styleProperties: ["cursor", "outlineStyle"] });
  assert.throws(() => coordinator.validateCdpRequest({ mode: "ui-state", selector: "#viewer", styleProperties: ["position"] }), /allowlisted/);
  assert.throws(() => coordinator.validateCdpRequest({ mode: "ui-state", selector: "#viewer", referenceSelector: "" }), /reference/);
  const source = await readFile(new URL("../scripts/guest-cdp.ps1", import.meta.url), "utf8");
  assert.match(source, /hoverCount/);
  assert.match(source, /focusWithinCount/);
  assert.match(source, /devicePixelRatio/);
  assert.match(source, /if \(\$null -eq \$request\.styleProperties\)[\s\S]*,\(\[object\[\]\]::new\(0\)\)[\s\S]*,@\(\$request\.styleProperties\)/);
});

test("native input agent opts into physical pixel coordinates for mixed-DPI dialogs", async () => {
  const source = await readFile(new URL("../../../crates/chemsema-gui-test-agent/src/windows.rs", import.meta.url), "utf8");
  assert.match(source, /DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2/);
  assert.match(source, /enable_physical_pixel_coordinates\(\)\?/);
});

test("physical daemon queue is bounded to unique versioned scenario files", () => {
  const queue = {
    schema: "chemsema.gui.physical-queue.v1",
    id: "smoke",
    scenarios: ["tests/gui/scenarios/core/draw-single-bond-production.json"],
  };
  assert.equal(validatePhysicalQueue(queue), queue);
  assert.throws(() => validatePhysicalQueue({ ...queue, scenarios: ["../outside.json"] }), /scenario path/);
  assert.throws(() => validatePhysicalQueue({ ...queue, scenarios: [...queue.scenarios, ...queue.scenarios] }), /duplicate/);
});

test("physical daemon records leases heartbeats checkpoints resource floors and evidence hashes", async () => {
  const source = await readFile(new URL("../src/workers/physical-daemon.mjs", import.meta.url), "utf8");
  assert.match(source, /physical-daemon-lease\.v1/);
  assert.match(source, /physical-daemon-heartbeat\.v1/);
  assert.match(source, /physical-daemon-checkpoint\.v1/);
  assert.match(source, /Free physical memory fell below/);
  assert.match(source, /evidenceManifestSha256/);
  assert.match(source, /refuses to bind a candidate to a dirty worktree/);
});

test("physical daemon retries only transient Windows atomic replace contention", async () => {
  let calls = 0;
  await renameAtomicWithRetry("source.tmp", "state.json", {
    attempts: 4,
    wait: async () => {},
    renameFile: async () => {
      calls += 1;
      if (calls < 3) throw Object.assign(new Error("temporarily locked"), { code: "EPERM" });
    },
  });
  assert.equal(calls, 3);
  await assert.rejects(
    renameAtomicWithRetry("source.tmp", "state.json", {
      wait: async () => {},
      renameFile: async () => { throw Object.assign(new Error("bad path"), { code: "ENOENT" }); },
    }),
    /bad path/,
  );
});

test("physical daemon preserves a missing child report as the primary paused failure", async () => {
  const missing = Object.assign(new Error("not found"), { code: "ENOENT" });
  assert.equal(await readScenarioReport("missing.json", { read: async () => { throw missing; } }), null);
  await assert.rejects(readScenarioReport("blocked.json", { read: async () => { throw Object.assign(new Error("denied"), { code: "EACCES" }); } }), /denied/);
  const source = await readFile(new URL("../src/workers/physical-daemon.mjs", import.meta.url), "utf8");
  assert.match(source, /status = "paused-failure"/);
  assert.match(source, /status: "missing-report"/);
  assert.match(source, /without producing a run report/);
});
