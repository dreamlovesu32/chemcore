import { access } from "node:fs/promises";
import { relative } from "node:path";
import { repositoryRoot } from "../protocol/paths.mjs";
import { candidateActionBudgetIsValid, candidateActionTransportReserveMs } from "../workers/action-budget.mjs";

export async function auditCoverage({ registry, scenarios, scenarioPaths = [] }) {
  const errors = [];
  const warnings = [];
  const scenarioIds = new Set(scenarios.map((scenario) => scenario.id));
  const registeredScenarioIds = new Set();
  const entryIds = new Set();

  for (const entry of registry.entries) {
    if (entryIds.has(entry.id)) {
      errors.push(`Duplicate coverage entry id: ${entry.id}`);
    }
    entryIds.add(entry.id);
    try {
      await access(new URL(entry.source, `file:///${repositoryRoot.replaceAll("\\", "/")}/`));
    } catch {
      errors.push(`Coverage source does not exist: ${entry.source}`);
    }
    for (const scenarioId of entry.scenarioIds) {
      registeredScenarioIds.add(scenarioId);
      if (!scenarioIds.has(scenarioId)) {
        errors.push(`Coverage entry ${entry.id} references unknown scenario ${scenarioId}.`);
      }
    }
  }

  for (const scenarioId of scenarioIds) {
    if (!registeredScenarioIds.has(scenarioId)) {
      errors.push(`Scenario ${scenarioId} is not mapped by the coverage registry.`);
    }
  }
  for (const scenario of scenarios.filter((candidate) => candidate.drivers.includes("production-black-box"))) {
    for (const action of scenario.actions) {
      if (!candidateActionBudgetIsValid(action.budgetMs, action.completion?.timeoutMs)) {
        errors.push(`Scenario ${scenario.id} action ${action.id} must reserve ${candidateActionTransportReserveMs} ms for production input transport.`);
      }
      const selector = action.completion?.selector;
      if (typeof selector === "string" && selector.includes(".is-selected") && selector.includes("[data-tool=")) {
        errors.push(`Scenario ${scenario.id} action ${action.id} must use is-active for a primary data-tool completion.`);
      }
      if (typeof selector === "string" && selector.includes(".is-active") && selector.includes("[data-secondary-value=")) {
        errors.push(`Scenario ${scenario.id} action ${action.id} must use is-selected for a secondary data-secondary-value completion.`);
      }
      if (typeof selector === "string" && selector.includes("[data-secondary-value=") && action.target?.strategy === "role") {
        if (action.target.scope?.role !== "toolbar" || action.target.scope?.name !== "Secondary toolbar") {
          errors.push(`Scenario ${scenario.id} action ${action.id} must scope a secondary role target to the Secondary toolbar.`);
        }
      }
    }
  }
  for (const entry of registry.entries.filter((candidate) => candidate.status === "gap")) {
    warnings.push(`Declared coverage gap: ${entry.id}`);
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
    summary: {
      entries: registry.entries.length,
      scenarios: scenarios.length,
      migrated: registry.entries.filter((entry) => entry.status === "migrated").length,
      partiallyMigrated: registry.entries.filter((entry) => entry.status === "partially-migrated").length,
      inventoried: registry.entries.filter((entry) => entry.status === "inventoried").length,
      gaps: registry.entries.filter((entry) => entry.status === "gap").length,
      scenarioFiles: scenarioPaths.map((path) => relative(repositoryRoot, path).replaceAll("\\", "/")),
    },
  };
}
