import { access } from "node:fs/promises";
import { relative } from "node:path";
import { repositoryRoot } from "../protocol/paths.mjs";
import { candidateActionBudgetIsValid, candidateActionTransportReserveMs } from "../workers/action-budget.mjs";
import { productionBlackBoxCapabilities } from "../drivers/production-black-box.mjs";

const productionCapabilitySet = new Set(productionBlackBoxCapabilities);

function clearsSelectionViaBlankPage(action) {
  return action?.type === "click"
    && action.button === "left"
    && action.target?.strategy === "world-geometry"
    && action.target.value === "page-background"
    && action.completion?.kind === "dom-count"
    && action.completion.selector === '[data-layer="editor-overlay"] > *'
    && action.completion.operator === "eq"
    && action.completion.value === 0;
}

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
    const missingCapabilities = scenario.capabilities.filter((capability) => !productionCapabilitySet.has(capability));
    if (missingCapabilities.length > 0) {
      errors.push(`Scenario ${scenario.id} requires capabilities not advertised by production-black-box: ${missingCapabilities.join(", ")}.`);
    }
    let bondPropertyMenuOpened = false;
    for (const [actionIndex, action] of scenario.actions.entries()) {
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
      if (typeof selector === "string" && selector.includes('.quick-palette.is-open[data-mode="element"]')) {
        const targetIsElementModeToggle = action.target?.strategy === "selector"
          && action.target.value === '.quick-palette-toggle-element[data-quick-palette-mode="element"]';
        if (!targetIsElementModeToggle) {
          errors.push(`Scenario ${scenario.id} action ${action.id} must target the stable Element quick-palette mode toggle.`);
        }
      }
      if (action.target?.strategy === "entity-id" && /^(?:n|b)_\d+$/.test(action.target.value)) {
        errors.push(`Scenario ${scenario.id} action ${action.id} must target a chemical node or bond with its data-node-id or data-bond-id selector; entity-id resolves scene object ids only.`);
      }
      const opensBondPropertyMenu = action.type === "click"
        && action.button === "right"
        && action.target?.strategy === "selector"
        && action.target.value.includes("[data-bond-id=")
        && action.completion?.selector?.includes('[data-canvas-context-command="bond-property"]');
      if (opensBondPropertyMenu) {
        if (!bondPropertyMenuOpened && !clearsSelectionViaBlankPage(scenario.actions[actionIndex - 1])) {
          errors.push(`Scenario ${scenario.id} action ${action.id} must immediately clear stale selection on page-background before opening its first bond-specific context menu.`);
        }
        bondPropertyMenuOpened = true;
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
