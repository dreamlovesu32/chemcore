import assert from "node:assert/strict";
import { join } from "node:path";
import test from "node:test";
import { FakeDriver } from "../src/drivers/fake.mjs";
import { selectImpactedScenarios } from "../src/impact/select.mjs";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { runScenario } from "../src/runner/run-scenario.mjs";
import { ResourceBudget } from "../src/scheduler/resource-budget.mjs";

test("the versioned bond scenario executes through the fake driver", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
  const report = await runScenario({ scenario, driver: new FakeDriver() });
  assert.equal(report.status, "passed");
  assert.equal(report.actions.length, 2);
  assert(report.actions.every((receipt) => receipt.status === "completed"));
  assert(report.oracles.every((oracle) => oracle.passed));
});

test("impact selection follows the transitive source to scenario closure", async () => {
  const graph = await readValidatedDocument(join(guiTestsDir, "coverage", "impact-v1.json"));
  assert.deepEqual(selectImpactedScenarios(graph, ["viewer/app.js"]), ["scenario.core.bond.draw-single"]);
  assert.deepEqual(selectImpactedScenarios(graph, ["docs/readme.md"]), []);
});

test("aggregate scheduler limits fail closed at 10 CPU units and 30 GiB", () => {
  const budget = new ResourceBudget();
  assert.deepEqual(budget.admit("interactive-a", { cpuUnits: 4, memoryGiB: 10 }), { cpuUnits: 4, memoryGiB: 10 });
  assert.deepEqual(budget.admit("interactive-b", { cpuUnits: 4, memoryGiB: 10 }), { cpuUnits: 8, memoryGiB: 20 });
  assert.deepEqual(budget.admit("coordinator", { cpuUnits: 2, memoryGiB: 10 }), { cpuUnits: 10, memoryGiB: 30 });
  assert.throws(() => budget.admit("overflow", { cpuUnits: 1, memoryGiB: 1 }), /Resource budget exceeded/);
  assert.deepEqual(budget.release("interactive-b"), { cpuUnits: 6, memoryGiB: 20 });
});
