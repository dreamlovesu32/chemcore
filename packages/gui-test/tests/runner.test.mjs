import assert from "node:assert/strict";
import { join } from "node:path";
import test from "node:test";
import { auditCoverage } from "../src/coverage/audit.mjs";
import { FakeDriver } from "../src/drivers/fake.mjs";
import { planImpactedScenarios, selectImpactedScenarios } from "../src/impact/select.mjs";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { runScenario } from "../src/runner/run-scenario.mjs";
import { ResourceBudget } from "../src/scheduler/resource-budget.mjs";

test("the versioned bond scenario executes through the fake driver", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
  const { report } = await runScenario({ scenario, driver: new FakeDriver() });
  assert.equal(report.status, "passed");
  assert.equal(report.actions.length, 2);
  assert(report.actions.every((receipt) => receipt.status === "completed"));
  assert(report.oracles.every((oracle) => oracle.passed));
  assert.deepEqual(report.artifacts, []);
});

test("artifact collection failure fails the run but still shuts the driver down", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
  const driver = new FakeDriver();
  let shutdowns = 0;
  driver.collectArtifacts = async () => { throw new Error("snapshot unavailable"); };
  driver.shutdown = async () => { shutdowns += 1; };
  const { report } = await runScenario({ scenario, driver });
  assert.equal(report.status, "failed");
  assert.match(report.failure.message, /Artifact collection failed/);
  assert(report.diagnostics.includes("artifact-collection: snapshot unavailable"));
  assert.equal(shutdowns, 1);
});

test("shutdown failure retains already collected artifacts as failure evidence", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
  const driver = new FakeDriver();
  driver.collectArtifacts = async () => [{ name: "final-state.json", mediaType: "application/json", bytes: Buffer.from("{}") }];
  driver.shutdown = async () => { throw new Error("shutdown unavailable"); };
  const { report, artifactPayloads } = await runScenario({ scenario, driver });
  assert.equal(report.status, "failed");
  assert.match(report.failure.message, /Driver shutdown failed/);
  assert.equal(report.artifacts[0].retention, "failure");
  assert.equal(artifactPayloads[0].descriptor.retention, "failure");
});

test("impact selection follows the transitive source to scenario closure", async () => {
  const graph = await readValidatedDocument(join(guiTestsDir, "coverage", "impact-v1.json"));
  assert.deepEqual(selectImpactedScenarios(graph, ["viewer/app.js"]), [
    "scenario.core.bond.draw-single",
    "scenario.core.bond.draw-single.production",
    "scenario.core.clipboard.cross-document-mixed.production",
    "scenario.core.document.save-open-roundtrip.production",
    "scenario.core.group.nested-mixed-clipboard.production",
    "scenario.core.history.undo-redo-bond.production",
    "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
    "scenario.core.selection.clipboard-delete-multi-bond.production",
    "scenario.core.selection.locked-partial-delete.production",
    "scenario.core.selection.region-additive-mixed-cardinalities.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["docs/readme.md"]), []);
  assert.deepEqual(planImpactedScenarios(graph, [
    "Cargo.lock",
    "crates/chemsema-gui-test-agent/src/windows.rs",
    "scripts/tests/recovery-journal.test.mjs",
  ]), {
    changedPaths: [
      "Cargo.lock",
      "crates/chemsema-gui-test-agent/src/windows.rs",
      "scripts/tests/recovery-journal.test.mjs",
    ],
    matchedSources: ["source.document-recovery-test", "source.gui-production-transport", "source.test-entrypoints"],
    unmatchedPaths: [],
    expandedForUncertainty: false,
    scenarios: [
      "scenario.core.bond.draw-single",
      "scenario.core.bond.draw-single.production",
      "scenario.core.clipboard.cross-document-mixed.production",
      "scenario.core.document.save-open-roundtrip.production",
      "scenario.core.group.nested-mixed-clipboard.production",
      "scenario.core.history.undo-redo-bond.production",
      "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
      "scenario.core.selection.clipboard-delete-multi-bond.production",
      "scenario.core.selection.locked-partial-delete.production",
      "scenario.core.selection.region-additive-mixed-cardinalities.production",
    ],
  });
  assert.deepEqual(selectImpactedScenarios(graph, ["scripts/tests/recovery-journal.test.mjs"]), [
    "scenario.core.clipboard.cross-document-mixed.production",
    "scenario.core.document.save-open-roundtrip.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["crates/chemsema-gui-test-agent/src/windows.rs"]), [
    "scenario.core.bond.draw-single.production",
    "scenario.core.clipboard.cross-document-mixed.production",
    "scenario.core.document.save-open-roundtrip.production",
    "scenario.core.group.nested-mixed-clipboard.production",
    "scenario.core.history.undo-redo-bond.production",
    "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
    "scenario.core.selection.clipboard-delete-multi-bond.production",
    "scenario.core.selection.locked-partial-delete.production",
    "scenario.core.selection.region-additive-mixed-cardinalities.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["packages/gui-test/tests/hyperv.test.mjs"]), []);
  assert.deepEqual(planImpactedScenarios(graph, ["unknown/new-surface.js"]), {
    changedPaths: ["unknown/new-surface.js"],
    matchedSources: [],
    unmatchedPaths: ["unknown/new-surface.js"],
    expandedForUncertainty: true,
    scenarios: [
      "scenario.core.bond.draw-single",
      "scenario.core.bond.draw-single.production",
      "scenario.core.clipboard.cross-document-mixed.production",
      "scenario.core.document.save-open-roundtrip.production",
      "scenario.core.group.nested-mixed-clipboard.production",
      "scenario.core.history.undo-redo-bond.production",
      "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
      "scenario.core.selection.clipboard-delete-multi-bond.production",
      "scenario.core.selection.locked-partial-delete.production",
      "scenario.core.selection.region-additive-mixed-cardinalities.production",
    ],
  });
});

test("coverage audit binds every registered source and scenario", async () => {
  const registry = await readValidatedDocument(join(guiTestsDir, "coverage", "registry-v1.json"));
  const scenarioPaths = [
    join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"),
    join(guiTestsDir, "scenarios", "core", "draw-single-bond-production.json"),
    join(guiTestsDir, "scenarios", "core", "undo-redo-bond-production.json"),
    join(guiTestsDir, "scenarios", "core", "multi-bond-clipboard-delete-production.json"),
    join(guiTestsDir, "scenarios", "core", "mixed-bond-arrow-clipboard-production.json"),
    join(guiTestsDir, "scenarios", "core", "cross-document-clipboard-production.json"),
    join(guiTestsDir, "scenarios", "core", "region-additive-mixed-cardinalities-production.json"),
    join(guiTestsDir, "scenarios", "core", "locked-partial-delete-production.json"),
    join(guiTestsDir, "scenarios", "core", "nested-mixed-group-clipboard-production.json"),
    join(guiTestsDir, "scenarios", "core", "save-open-roundtrip-production.json"),
  ];
  const scenarios = await Promise.all(scenarioPaths.map((path) => readValidatedDocument(path)));
  const result = await auditCoverage({ registry, scenarios, scenarioPaths });
  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.equal(result.summary.entries, 24);
  assert.equal(result.summary.scenarios, 10);
  assert.equal(result.summary.gaps, 0);
});

test("aggregate scheduler limits fail closed at 10 CPU units and 30 GiB", () => {
  const budget = new ResourceBudget();
  assert.deepEqual(budget.admit("interactive-a", { cpuUnits: 4, memoryGiB: 10 }), { cpuUnits: 4, memoryGiB: 10 });
  assert.deepEqual(budget.admit("interactive-b", { cpuUnits: 4, memoryGiB: 10 }), { cpuUnits: 8, memoryGiB: 20 });
  assert.deepEqual(budget.admit("coordinator", { cpuUnits: 2, memoryGiB: 10 }), { cpuUnits: 10, memoryGiB: 30 });
  assert.throws(() => budget.admit("overflow", { cpuUnits: 1, memoryGiB: 1 }), /Resource budget exceeded/);
  assert.deepEqual(budget.release("interactive-b"), { cpuUnits: 6, memoryGiB: 20 });
});
