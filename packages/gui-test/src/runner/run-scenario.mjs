import { randomUUID } from "node:crypto";
import { evidenceKey } from "../protocol/canonical.mjs";
import { assertValidDocument } from "../protocol/validate.mjs";

function oraclePassed(oracle, observed, expectedDiagnostics) {
  if (oracle.kind === "dom-count") {
    return oracle.operator === "eq" ? observed === oracle.value : observed >= oracle.value;
  }
  if (oracle.kind === "no-unexpected-diagnostics") {
    return observed.every((diagnostic) => expectedDiagnostics.some((expected) => diagnostic.includes(expected)));
  }
  return false;
}

function withinBudget(promise, budgetMs, label) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${label} exceeded its ${budgetMs} ms action budget.`)), budgetMs);
    }),
  ]).finally(() => clearTimeout(timer));
}

export async function runScenario({ scenario, driver, candidate = {}, componentClosure = [], artifacts = [] }) {
  await assertValidDocument(scenario, scenario.id || "scenario");
  if (!scenario.drivers.includes(driver.name)) {
    throw new Error(`Scenario ${scenario.id} does not allow driver ${driver.name}.`);
  }
  const missingCapabilities = scenario.capabilities.filter((capability) => !driver.capabilities().includes(capability));
  if (missingCapabilities.length) {
    throw new Error(`Driver ${driver.name} lacks required capabilities: ${missingCapabilities.join(", ")}.`);
  }

  const started = Date.now();
  const actionReceipts = [];
  const oracleResults = [];
  let status = "passed";
  let failure = null;
  try {
    await driver.prepare(scenario.profile);
    await driver.launch(candidate);
    for (const action of scenario.actions) {
      const actionStarted = Date.now();
      const startedAt = new Date(actionStarted).toISOString();
      let resolvedTarget = null;
      let before = { revision: null, window: {} };
      try {
        const result = await withinBudget((async () => {
          resolvedTarget = await driver.resolve(action.target);
          if (typeof driver.executeAction === "function") {
            const transaction = await driver.executeAction(action);
            before = transaction.before;
            return { completion: transaction.completion, after: transaction.after };
          }
          before = await driver.actionState();
          await driver.perform(action);
          const completion = await driver.waitForCompletion(action.completion);
          const after = await driver.actionState();
          return { completion, after };
        })(), action.budgetMs, `Action ${action.id}`);
        const ended = Date.now();
        actionReceipts.push({
          actionId: action.id,
          inputType: action.type,
          resolvedTarget,
          startedAt,
          endedAt: new Date(ended).toISOString(),
          durationMs: ended - actionStarted,
          status: "completed",
          before,
          after: result.after,
          completion: result.completion,
          diagnostics: [],
          artifacts: [],
        });
      } catch (error) {
        const ended = Date.now();
        let after = before;
        try {
          after = await driver.actionState();
        } catch {
          // The driver may already be unavailable; preserve the last known state.
        }
        actionReceipts.push({
          actionId: action.id,
          inputType: action.type,
          resolvedTarget,
          startedAt,
          endedAt: new Date(ended).toISOString(),
          durationMs: ended - actionStarted,
          status: "failed",
          before,
          after,
          completion: null,
          diagnostics: [error.message],
          artifacts: [],
        });
        throw error;
      }
    }
    for (const oracle of scenario.oracles) {
      const observed = await driver.observe(oracle);
      const passed = oraclePassed(oracle, observed, scenario.expectedDiagnostics || []);
      oracleResults.push({ oracleId: oracle.id, kind: oracle.kind, passed, observed });
      if (!passed) {
        throw new Error(`Oracle ${oracle.id} failed: observed ${JSON.stringify(observed)}.`);
      }
    }
  } catch (error) {
    status = "failed";
    failure = { name: error.name, message: error.message, stack: error.stack || null };
  }

  let environment = {};
  let diagnostics = [];
  try {
    environment = await driver.environment();
    diagnostics = await driver.observe({ kind: "no-unexpected-diagnostics" });
  } catch (error) {
    diagnostics.push(`artifact-collection: ${error.message}`);
  }
  try {
    await driver.collectArtifacts(status === "passed" ? "sample" : "failure");
  } finally {
    await driver.shutdown();
  }

  const ended = Date.now();
  const report = {
    schema: "chemsema.gui.run.v1",
    runId: randomUUID(),
    scenarioId: scenario.id,
    driver: driver.name,
    status,
    startedAt: new Date(started).toISOString(),
    endedAt: new Date(ended).toISOString(),
    durationMs: ended - started,
    environment,
    actions: actionReceipts,
    oracles: oracleResults,
    diagnostics,
    failure,
    evidenceKey: evidenceKey({ scenario, driver: driver.name, environment, componentClosure, artifacts }),
  };
  await assertValidDocument(report, `run report for ${scenario.id}`);
  return report;
}
