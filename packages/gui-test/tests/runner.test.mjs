import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { auditCoverage } from "../src/coverage/audit.mjs";
import { FakeDriver } from "../src/drivers/fake.mjs";
import { planImpactedScenarios, selectImpactedScenarios } from "../src/impact/select.mjs";
import { guiTestsDir, repositoryRoot } from "../src/protocol/paths.mjs";
import { assertValidDocument, readValidatedDocument } from "../src/protocol/validate.mjs";
import { runScenario } from "../src/runner/run-scenario.mjs";
import { ResourceBudget } from "../src/scheduler/resource-budget.mjs";

test("the versioned bond scenario executes through the fake driver", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
  const { report } = await runScenario({ scenario, driver: new FakeDriver() });
  assert.equal(report.status, "passed");
  assert.equal(report.actions.length, 2);
  assert(report.actions.every((receipt) => receipt.status === "completed"));
  assert(report.actions.every((receipt) => receipt.phases.some((phase) => phase.name === "resolve-target" && phase.status === "completed")));
  assert(report.actions.every((receipt) => receipt.phases.some((phase) => phase.name === "execute-action" && phase.status === "completed")));
  const legacyReport = structuredClone(report);
  for (const receipt of legacyReport.actions) delete receipt.phases;
  await assertValidDocument(legacyReport, "legacy run report without phase attribution");
  assert(report.oracles.every((oracle) => oracle.passed));
  assert.deepEqual(report.artifacts, []);
});

test("the fake driver uses the same exact DOM text completion contract", async () => {
  const driver = new FakeDriver();
  await driver.perform({ fakeEffect: { kind: "set-dom-text", selector: ".display", text: "A  B\nC" } });
  assert.deepEqual(
    await driver.waitForCompletion({ kind: "dom-text", selector: ".display", text: "A  B\nC" }),
    { observedText: "A  B\nC" },
  );
  await assert.rejects(
    driver.waitForCompletion({ kind: "dom-text", selector: ".display", text: "A B\nC" }),
    /DOM text completion failed/,
  );
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
    "scenario.core.arrow.locked-mixed-properties.production",
    "scenario.core.arrow.multi-property-history.production",
    "scenario.core.arrow.property-matrix-persistence.production",
    "scenario.core.atom.charge-symbol-attachment-persistence.production",
    "scenario.core.atom.electron-symbol-attachment-persistence.production",
    "scenario.core.atom.element-label-persistence.production",
    "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
    "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
    "scenario.core.bond.draw-single",
    "scenario.core.bond.draw-single.production",
    "scenario.core.bond.ten-variant-persistence.production",
    "scenario.core.bracket.three-kind-properties-history.production",
    "scenario.core.chain.drag-count-persistence.production",
    "scenario.core.chain.endpoint-attachment-continuation.production",
    "scenario.core.chromatography.tlc-gel-mark-color-history.production",
    "scenario.core.clipboard.cross-document-mixed.production",
    "scenario.core.document.save-open-roundtrip.production",
    "scenario.core.frontend.focus-hover-disabled.production",
    "scenario.core.frontend.selection-geometry.production",
    "scenario.core.group.locked-ancestor-transform.production",
    "scenario.core.group.nested-mixed-clipboard.production",
    "scenario.core.history.undo-redo-bond.production",
    "scenario.core.orbital.seven-template-properties-history.production",
    "scenario.core.ring.bond-fusion-persistence.production",
    "scenario.core.ring.chair-benzene-persistence.production",
    "scenario.core.ring.endpoint-attachment-persistence.production",
    "scenario.core.ring.six-planar-persistence.production",
    "scenario.core.ring.vertex-bond-continuation-persistence.production",
    "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
    "scenario.core.selection.clipboard-delete-multi-bond.production",
    "scenario.core.selection.locked-molecule-arrow-transform.production",
    "scenario.core.selection.locked-partial-delete.production",
    "scenario.core.selection.locked-transform.production",
    "scenario.core.selection.region-additive-mixed-cardinalities.production",
    "scenario.core.shape.multi-kind-style-history.production",
    "scenario.core.symbol.eight-kind-color-history.production",
    "scenario.core.table.structure-border-history.production",
    "scenario.core.text.existing-edit-history.production",
    "scenario.core.text.line-spacing-validation.production",
    "scenario.core.text.multi-property-persistence.production",
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
    matchedSources: ["source.document-recovery-test", "source.gui-production-transport", "source.runtime-dependencies"],
    unmatchedPaths: [],
    expandedForUncertainty: false,
    scenarios: [
      "scenario.core.arrow.locked-mixed-properties.production",
      "scenario.core.arrow.multi-property-history.production",
      "scenario.core.arrow.property-matrix-persistence.production",
      "scenario.core.atom.charge-symbol-attachment-persistence.production",
      "scenario.core.atom.electron-symbol-attachment-persistence.production",
      "scenario.core.atom.element-label-persistence.production",
      "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
      "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
      "scenario.core.bond.draw-single",
      "scenario.core.bond.draw-single.production",
      "scenario.core.bond.ten-variant-persistence.production",
      "scenario.core.bracket.three-kind-properties-history.production",
      "scenario.core.chain.drag-count-persistence.production",
      "scenario.core.chain.endpoint-attachment-continuation.production",
      "scenario.core.chromatography.tlc-gel-mark-color-history.production",
      "scenario.core.clipboard.cross-document-mixed.production",
      "scenario.core.document.save-open-roundtrip.production",
      "scenario.core.frontend.focus-hover-disabled.production",
      "scenario.core.frontend.selection-geometry.production",
      "scenario.core.group.locked-ancestor-transform.production",
      "scenario.core.group.nested-mixed-clipboard.production",
      "scenario.core.history.undo-redo-bond.production",
      "scenario.core.orbital.seven-template-properties-history.production",
      "scenario.core.ring.bond-fusion-persistence.production",
      "scenario.core.ring.chair-benzene-persistence.production",
      "scenario.core.ring.endpoint-attachment-persistence.production",
      "scenario.core.ring.six-planar-persistence.production",
      "scenario.core.ring.vertex-bond-continuation-persistence.production",
      "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
      "scenario.core.selection.clipboard-delete-multi-bond.production",
      "scenario.core.selection.locked-molecule-arrow-transform.production",
      "scenario.core.selection.locked-partial-delete.production",
      "scenario.core.selection.locked-transform.production",
      "scenario.core.selection.region-additive-mixed-cardinalities.production",
      "scenario.core.shape.multi-kind-style-history.production",
      "scenario.core.symbol.eight-kind-color-history.production",
      "scenario.core.table.structure-border-history.production",
      "scenario.core.text.existing-edit-history.production",
      "scenario.core.text.line-spacing-validation.production",
      "scenario.core.text.multi-property-persistence.production",
    ],
  });
  assert.deepEqual(selectImpactedScenarios(graph, ["scripts/tests/recovery-journal.test.mjs"]), [
    "scenario.core.arrow.property-matrix-persistence.production",
    "scenario.core.bracket.three-kind-properties-history.production",
    "scenario.core.chromatography.tlc-gel-mark-color-history.production",
    "scenario.core.clipboard.cross-document-mixed.production",
    "scenario.core.document.save-open-roundtrip.production",
    "scenario.core.orbital.seven-template-properties-history.production",
    "scenario.core.shape.multi-kind-style-history.production",
    "scenario.core.symbol.eight-kind-color-history.production",
    "scenario.core.table.structure-border-history.production",
    "scenario.core.text.existing-edit-history.production",
    "scenario.core.text.line-spacing-validation.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["crates/chemsema-gui-test-agent/src/windows.rs"]), [
    "scenario.core.arrow.locked-mixed-properties.production",
    "scenario.core.arrow.multi-property-history.production",
    "scenario.core.arrow.property-matrix-persistence.production",
    "scenario.core.atom.charge-symbol-attachment-persistence.production",
    "scenario.core.atom.electron-symbol-attachment-persistence.production",
    "scenario.core.atom.element-label-persistence.production",
    "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
    "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
    "scenario.core.bond.draw-single.production",
    "scenario.core.bond.ten-variant-persistence.production",
    "scenario.core.bracket.three-kind-properties-history.production",
    "scenario.core.chain.drag-count-persistence.production",
    "scenario.core.chain.endpoint-attachment-continuation.production",
    "scenario.core.chromatography.tlc-gel-mark-color-history.production",
    "scenario.core.clipboard.cross-document-mixed.production",
    "scenario.core.document.save-open-roundtrip.production",
    "scenario.core.frontend.focus-hover-disabled.production",
    "scenario.core.frontend.selection-geometry.production",
    "scenario.core.group.locked-ancestor-transform.production",
    "scenario.core.group.nested-mixed-clipboard.production",
    "scenario.core.history.undo-redo-bond.production",
    "scenario.core.orbital.seven-template-properties-history.production",
    "scenario.core.ring.bond-fusion-persistence.production",
    "scenario.core.ring.chair-benzene-persistence.production",
    "scenario.core.ring.endpoint-attachment-persistence.production",
    "scenario.core.ring.six-planar-persistence.production",
    "scenario.core.ring.vertex-bond-continuation-persistence.production",
    "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
    "scenario.core.selection.clipboard-delete-multi-bond.production",
    "scenario.core.selection.locked-molecule-arrow-transform.production",
    "scenario.core.selection.locked-partial-delete.production",
    "scenario.core.selection.locked-transform.production",
    "scenario.core.selection.region-additive-mixed-cardinalities.production",
    "scenario.core.shape.multi-kind-style-history.production",
    "scenario.core.symbol.eight-kind-color-history.production",
    "scenario.core.table.structure-border-history.production",
    "scenario.core.text.existing-edit-history.production",
    "scenario.core.text.line-spacing-validation.production",
    "scenario.core.text.multi-property-persistence.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["packages/gui-test/tests/hyperv.test.mjs"]), []);
  assert.deepEqual(selectImpactedScenarios(graph, ["packages/gui-test/src/coverage/audit.mjs"]), []);
  assert.deepEqual(selectImpactedScenarios(graph, ["viewer/numeric_dialog_host.js"]), [
    "scenario.core.text.line-spacing-validation.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["packages/gui-test/src/oracles/document-file.mjs"]), [
    "scenario.core.arrow.property-matrix-persistence.production",
    "scenario.core.atom.charge-symbol-attachment-persistence.production",
    "scenario.core.atom.electron-symbol-attachment-persistence.production",
    "scenario.core.atom.element-label-persistence.production",
    "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
    "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
    "scenario.core.bond.ten-variant-persistence.production",
    "scenario.core.bracket.three-kind-properties-history.production",
    "scenario.core.chain.drag-count-persistence.production",
    "scenario.core.chain.endpoint-attachment-continuation.production",
    "scenario.core.chromatography.tlc-gel-mark-color-history.production",
    "scenario.core.document.save-open-roundtrip.production",
    "scenario.core.orbital.seven-template-properties-history.production",
    "scenario.core.ring.bond-fusion-persistence.production",
    "scenario.core.ring.chair-benzene-persistence.production",
    "scenario.core.ring.endpoint-attachment-persistence.production",
    "scenario.core.ring.six-planar-persistence.production",
    "scenario.core.ring.vertex-bond-continuation-persistence.production",
    "scenario.core.shape.multi-kind-style-history.production",
    "scenario.core.symbol.eight-kind-color-history.production",
    "scenario.core.table.structure-border-history.production",
    "scenario.core.text.existing-edit-history.production",
    "scenario.core.text.line-spacing-validation.production",
    "scenario.core.text.multi-property-persistence.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["packages/gui-test/src/oracles/ui-state.mjs"]), [
    "scenario.core.frontend.focus-hover-disabled.production",
    "scenario.core.frontend.selection-geometry.production",
  ]);
  assert.deepEqual(selectImpactedScenarios(graph, ["scripts/test.mjs"]), []);
  assert.deepEqual(selectImpactedScenarios(graph, ["packages/gui-test/src/qualification/evaluate.mjs"]), []);
  assert.deepEqual(planImpactedScenarios(graph, ["unknown/new-surface.js"]), {
    changedPaths: ["unknown/new-surface.js"],
    matchedSources: [],
    unmatchedPaths: ["unknown/new-surface.js"],
    expandedForUncertainty: true,
    scenarios: [
      "scenario.core.arrow.locked-mixed-properties.production",
      "scenario.core.arrow.multi-property-history.production",
      "scenario.core.arrow.property-matrix-persistence.production",
      "scenario.core.atom.charge-symbol-attachment-persistence.production",
      "scenario.core.atom.electron-symbol-attachment-persistence.production",
      "scenario.core.atom.element-label-persistence.production",
      "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
      "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
      "scenario.core.bond.draw-single",
      "scenario.core.bond.draw-single.production",
      "scenario.core.bond.ten-variant-persistence.production",
      "scenario.core.bracket.three-kind-properties-history.production",
      "scenario.core.chain.drag-count-persistence.production",
      "scenario.core.chain.endpoint-attachment-continuation.production",
      "scenario.core.chromatography.tlc-gel-mark-color-history.production",
      "scenario.core.clipboard.cross-document-mixed.production",
      "scenario.core.document.save-open-roundtrip.production",
      "scenario.core.frontend.focus-hover-disabled.production",
      "scenario.core.frontend.selection-geometry.production",
      "scenario.core.group.locked-ancestor-transform.production",
      "scenario.core.group.nested-mixed-clipboard.production",
      "scenario.core.history.undo-redo-bond.production",
      "scenario.core.orbital.seven-template-properties-history.production",
      "scenario.core.ring.bond-fusion-persistence.production",
      "scenario.core.ring.chair-benzene-persistence.production",
      "scenario.core.ring.endpoint-attachment-persistence.production",
      "scenario.core.ring.six-planar-persistence.production",
      "scenario.core.ring.vertex-bond-continuation-persistence.production",
      "scenario.core.selection.clipboard-delete-mixed-bond-arrow.production",
      "scenario.core.selection.clipboard-delete-multi-bond.production",
      "scenario.core.selection.locked-molecule-arrow-transform.production",
      "scenario.core.selection.locked-partial-delete.production",
      "scenario.core.selection.locked-transform.production",
      "scenario.core.selection.region-additive-mixed-cardinalities.production",
      "scenario.core.shape.multi-kind-style-history.production",
      "scenario.core.symbol.eight-kind-color-history.production",
      "scenario.core.table.structure-border-history.production",
      "scenario.core.text.existing-edit-history.production",
      "scenario.core.text.line-spacing-validation.production",
      "scenario.core.text.multi-property-persistence.production",
    ],
  });
});

test("coverage audit binds every registered source and scenario", async () => {
  const registry = await readValidatedDocument(join(guiTestsDir, "coverage", "registry-v1.json"));
  const scenarioPaths = [
    join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"),
    join(guiTestsDir, "scenarios", "core", "draw-single-bond-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-ten-variant-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-six-planar-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-chair-benzene-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-bond-fusion-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-endpoint-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-vertex-bond-continuation-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-charge-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-electron-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-lone-pair-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-negative-charge-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-radical-cation-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-radical-anion-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-element-label-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "chain-drag-count-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "chain-endpoint-attachment-continuation-production.json"),
    join(guiTestsDir, "scenarios", "core", "undo-redo-bond-production.json"),
    join(guiTestsDir, "scenarios", "core", "multi-bond-clipboard-delete-production.json"),
    join(guiTestsDir, "scenarios", "core", "mixed-bond-arrow-clipboard-production.json"),
    join(guiTestsDir, "scenarios", "core", "cross-document-clipboard-production.json"),
    join(guiTestsDir, "scenarios", "core", "region-additive-mixed-cardinalities-production.json"),
    join(guiTestsDir, "scenarios", "core", "locked-partial-delete-production.json"),
    join(guiTestsDir, "scenarios", "core", "locked-molecule-arrow-transform-production.json"),
    join(guiTestsDir, "scenarios", "core", "locked-transform-production.json"),
    join(guiTestsDir, "scenarios", "core", "locked-group-ancestor-transform-production.json"),
    join(guiTestsDir, "scenarios", "core", "multi-arrow-properties-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "locked-mixed-arrow-properties-production.json"),
    join(guiTestsDir, "scenarios", "core", "arrow-property-matrix-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "text-multi-property-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "text-line-spacing-validation-production.json"),
    join(guiTestsDir, "scenarios", "core", "text-existing-edit-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "shape-multi-kind-style-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "symbol-eight-kind-color-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "bracket-three-kind-properties-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "table-structure-border-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "orbital-seven-template-properties-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "chromatography-tlc-gel-mark-color-history-production.json"),
    join(guiTestsDir, "scenarios", "core", "nested-mixed-group-clipboard-production.json"),
    join(guiTestsDir, "scenarios", "core", "save-open-roundtrip-production.json"),
    join(guiTestsDir, "scenarios", "core", "frontend-focus-hover-disabled-production.json"),
    join(guiTestsDir, "scenarios", "core", "frontend-selection-geometry-production.json"),
  ];
  const scenarios = await Promise.all(scenarioPaths.map((path) => readValidatedDocument(path)));
  const result = await auditCoverage({ registry, scenarios, scenarioPaths });
  assert.equal(result.valid, true, result.errors.join("\n"));
  assert.equal(result.summary.entries, 42);
  assert.equal(result.summary.scenarios, 42);
  assert.equal(result.summary.gaps, 0);

  const invalidScenarios = structuredClone(scenarios);
  const invalidAction = invalidScenarios.find((scenario) => scenario.drivers.includes("production-black-box")).actions[0];
  invalidAction.completion.timeoutMs = 20000;
  invalidAction.budgetMs = 30000;
  const invalidResult = await auditCoverage({ registry, scenarios: invalidScenarios, scenarioPaths });
  assert.equal(invalidResult.valid, false);
  assert.match(invalidResult.errors.join("\n"), /must reserve 15000 ms for production input transport/);

  const wrongToolStateScenarios = structuredClone(scenarios);
  const chainActivation = wrongToolStateScenarios
    .find((scenario) => scenario.id === "core.chain.drag-count-persistence.production")
    .actions.find((action) => action.id === "activate-chain-tool");
  chainActivation.completion.selector = 'button[data-tool="chain"].is-selected';
  const wrongToolStateResult = await auditCoverage({ registry, scenarios: wrongToolStateScenarios, scenarioPaths });
  assert.equal(wrongToolStateResult.valid, false);
  assert.match(wrongToolStateResult.errors.join("\n"), /must use is-active for a primary data-tool completion/);

  const ambiguousSecondaryScenarios = structuredClone(scenarios);
  const ringChoice = ambiguousSecondaryScenarios
    .find((scenario) => scenario.id === "core.ring.bond-fusion-persistence.production")
    .actions.find((action) => action.id === "choose-ring-6");
  delete ringChoice.target.scope;
  const ambiguousSecondaryResult = await auditCoverage({ registry, scenarios: ambiguousSecondaryScenarios, scenarioPaths });
  assert.equal(ambiguousSecondaryResult.valid, false);
  assert.match(ambiguousSecondaryResult.errors.join("\n"), /must scope a secondary role target to the Secondary toolbar/);

  const wrongPaletteControlScenarios = structuredClone(scenarios);
  const elementPaletteAction = wrongPaletteControlScenarios
    .find((scenario) => scenario.id === "core.atom.element-label-persistence.production")
    .actions.find((action) => action.id === "open-element-palette");
  elementPaletteAction.target = { strategy: "role", value: "button", name: "Element", scope: { role: "complementary", name: "Main Drawing Rail" } };
  const wrongPaletteControlResult = await auditCoverage({ registry, scenarios: wrongPaletteControlScenarios, scenarioPaths });
  assert.equal(wrongPaletteControlResult.valid, false);
  assert.match(wrongPaletteControlResult.errors.join("\n"), /must target the stable Element quick-palette mode toggle/);
});

test("the planar ring matrix kills missing and wrong-member-count tool mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "ring-six-planar-persistence-production.json"));
  const activation = scenario.actions.find((action) => action.id === "activate-rings-tool");
  const choices = scenario.actions.filter((action) => action.id.startsWith("choose-ring-"));
  const insertions = scenario.actions.filter((action) => action.id.startsWith("insert-ring-"));
  assert.deepEqual(activation.target, {
    strategy: "selector",
    value: 'button[data-tool="templates"][data-tool-rail="main"]',
  }, "runtime-renamed primary tools must use stable semantic identity");
  assert.deepEqual(choices.map((action) => action.id), ["choose-ring-3", "choose-ring-4", "choose-ring-5", "choose-ring-6", "choose-ring-7", "choose-ring-8"]);
  assert.deepEqual(insertions.map((action) => action.completion.value), [3, 7, 12, 18, 25, 33]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-counts").expected, {
    nodes: 33,
    bonds: 33,
    molecules: 6,
    objects: 6,
  });
});

test("the chair and benzene matrix kills conformer omission and aromatic-order mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "ring-chair-benzene-persistence-production.json"));
  assert.deepEqual(
    scenario.actions.filter((action) => action.id.startsWith("choose-")).map((action) => action.id),
    ["choose-chair-right", "choose-chair-left", "choose-benzene"],
  );
  assert.deepEqual(
    scenario.actions.filter((action) => action.id.startsWith("insert-")).map((action) => action.completion.value),
    [6, 12, 18],
  );
  const aromatic = scenario.oracles.find((oracle) => oracle.id === "saved-benzene-alternating-bond-semantics");
  assert.deepEqual(aromatic.expected.map(({ id, order }) => [id, order]), [
    ["b_31", 2], ["b_32", 1], ["b_33", 2], ["b_34", 1], ["b_35", 2], ["b_36", 1],
  ]);
});

test("the ring fusion matrix kills disconnected-paste and duplicate-shared-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "ring-bond-fusion-persistence-production.json"));
  const draw = scenario.actions.find((action) => action.id === "draw-ring-fusion-target-bond");
  const fusion = scenario.actions.find((action) => action.id === "fuse-ring-6-on-target-bond");
  assert.equal(draw.completion.value, 1);
  assert.equal(fusion.completion.value, 6);
  assert.ok(fusion.at.x > draw.from.x && fusion.at.x < draw.to.x, "the fusion click must target the drawn bond interior");
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-counts").expected, {
    nodes: 6,
    bonds: 6,
    molecules: 1,
    objects: 1,
  });
});

test("the ring endpoint matrix kills missed-endpoint and accidental-bond-fusion mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "ring-endpoint-attachment-persistence-production.json"));
  const draw = scenario.actions.find((action) => action.id === "draw-ring-attachment-target-bond");
  const attachment = scenario.actions.find((action) => action.id === "attach-ring-6-at-target-endpoint");
  assert.equal(draw.completion.value, 1);
  assert.equal(attachment.completion.value, 7);
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-counts").expected, {
    nodes: 7,
    bonds: 7,
    molecules: 1,
    objects: 1,
  });
});

test("the ring vertex continuation matrix kills vertex-miss and disconnected-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "ring-vertex-bond-continuation-persistence-production.json"));
  const ringAttachment = scenario.actions.find((action) => action.id === "attach-ring-6-at-target-endpoint");
  const continuation = scenario.actions.find((action) => action.id === "continue-single-bond-from-ring-vertex");
  assert.equal(ringAttachment.completion.value, 7);
  assert.equal(continuation.completion.value, 8);
  assert.ok(Math.abs(continuation.from.x - 0.43772) < Number.EPSILON);
  assert.ok(continuation.to.x > continuation.from.x);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-counts").expected, {
    nodes: 8,
    bonds: 8,
    molecules: 1,
    objects: 1,
  });
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-ring-vertex-continuation-single-bond-semantics").expected.map(({ id, order }) => [id, order]),
    [["b_16", 1]],
  );
});

test("the element-label matrix kills wrong-palette, wrong-target, and node-semantic mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-element-label-persistence-production.json"));
  const palette = scenario.actions.find((action) => action.id === "open-element-palette");
  const nitrogen = scenario.actions.find((action) => action.id === "choose-nitrogen");
  const apply = scenario.actions.find((action) => action.id === "apply-nitrogen-to-target-endpoint");
  assert.deepEqual(palette.target, {
    strategy: "selector",
    value: '.quick-palette-toggle-element[data-quick-palette-mode="element"]',
  });
  assert.equal(palette.completion.selector, '.quick-palette.is-open[data-mode="element"]');
  assert.equal(nitrogen.target.value, '.periodic-element-button[data-element-symbol="N"][data-element-atomic-number="7"]');
  assert.ok(Math.abs(apply.at.x - 0.37257) < Number.EPSILON);
  assert.equal(apply.completion.selector, '[data-node-id="n_2"]');
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-node-properties").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 0, labelText: "NH2", labelSourceText: "NH2" },
  ]);
});

test("the atom charge attachment matrix kills detached-symbol, stale-charge, and stale-hydrogen mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-charge-symbol-attachment-persistence-production.json"));
  const attachment = scenario.actions.find((action) => action.id === "attach-circle-plus-to-nitrogen");
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-ammonium-node-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 1, numHydrogens: 3, labelText: "NH3", labelSourceText: "NH3" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-positive-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "circle-plus", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "charge", chargeDelta: 1, radicalDelta: 0, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the negative atom charge matrix kills wrong-kind, stale-hydrogen, and detached-symbol mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-negative-charge-symbol-attachment-persistence-production.json"));
  const oxygen = scenario.actions.find((action) => action.id === "choose-oxygen");
  const minus = scenario.actions.find((action) => action.id === "choose-circle-minus");
  const attachment = scenario.actions.find((action) => action.id === "attach-circle-minus-to-oxygen");
  assert.equal(oxygen.target.value, '.periodic-element-button[data-element-symbol="O"][data-element-atomic-number="8"]');
  assert.deepEqual(minus.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(minus.completion.selector, 'button[data-secondary-value="symbol-kind-circle-minus"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-alkoxide-node-semantics").expected, [
    { id: "n_2", element: "O", atomicNumber: 8, charge: -1, numHydrogens: 0, labelText: "O", labelSourceText: "O" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-negative-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "circle-minus", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "charge", chargeDelta: -1, radicalDelta: 0, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the radical-cation matrix kills partial-delta, stale-radical, and detached-symbol mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-radical-cation-symbol-attachment-persistence-production.json"));
  const choice = scenario.actions.find((action) => action.id === "choose-radical-cation");
  const attachment = scenario.actions.find((action) => action.id === "attach-radical-cation-to-nitrogen");
  assert.deepEqual(choice.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(choice.completion.selector, 'button[data-secondary-value="symbol-kind-radical-cation"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-radical-cation-node-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 1, numHydrogens: 2, radicalCount: 1, labelText: "NH2", labelSourceText: "NH2" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-radical-cation-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "radical-cation", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "radical-cation", chargeDelta: 1, radicalDelta: 1, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the radical-anion matrix kills ordinary-minus aliases, partial-delta, stale-radical, and detached-symbol mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-radical-anion-symbol-attachment-persistence-production.json"));
  const choice = scenario.actions.find((action) => action.id === "choose-radical-anion");
  const attachment = scenario.actions.find((action) => action.id === "attach-radical-anion-to-nitrogen");
  assert.deepEqual(choice.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(choice.completion.selector, 'button[data-secondary-value="symbol-kind-radical-anion"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-radical-anion-node-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: -1, numHydrogens: 0, radicalCount: 1, labelText: "N", labelSourceText: "N" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-radical-anion-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "radical-anion", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "radical-anion", chargeDelta: -1, radicalDelta: 1, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the electron matrix kills accidental-charge, stale-hydrogen, missing-radical, and detached-symbol mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-electron-symbol-attachment-persistence-production.json"));
  const choice = scenario.actions.find((action) => action.id === "choose-electron");
  const attachment = scenario.actions.find((action) => action.id === "attach-electron-to-nitrogen");
  assert.deepEqual(choice.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(choice.completion.selector, 'button[data-secondary-value="symbol-kind-electron"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-electron-node-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 0, numHydrogens: 1, radicalCount: 1, labelText: "NH", labelSourceText: "NH" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-electron-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "electron", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "radical", chargeDelta: 0, radicalDelta: 1, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the lone-pair matrix kills accidental charge, radical, hydrogen-removal, and detached-symbol mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-lone-pair-symbol-attachment-persistence-production.json"));
  const choice = scenario.actions.find((action) => action.id === "choose-lone-pair");
  const attachment = scenario.actions.find((action) => action.id === "attach-lone-pair-to-nitrogen");
  assert.deepEqual(choice.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(choice.completion.selector, 'button[data-secondary-value="symbol-kind-lone-pair"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-lone-pair-node-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 0, numHydrogens: 2, radicalCount: 0, labelText: "NH2", labelSourceText: "NH2" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-lone-pair-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "lone-pair", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "lone-pair", chargeDelta: 0, radicalDelta: 0, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the chain matrix kills fixed-length and off-by-one drag-count mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "chain-drag-count-persistence-production.json"));
  const activation = scenario.actions.find((action) => action.id === "activate-chain-tool");
  const drag = scenario.actions.find((action) => action.id === "drag-four-bond-chain");
  assert.equal(activation.completion.selector, 'button[data-tool="chain"].is-active');
  assert.ok(Math.abs((drag.to.x - drag.from.x) - 0.145) < Number.EPSILON);
  assert.equal(drag.completion.value, 4);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-counts").expected, {
    nodes: 5,
    bonds: 4,
    molecules: 1,
    objects: 1,
  });
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-chain-single-bond-semantics").expected.map(({ id, order }) => [id, order]),
    [["b_6", 1], ["b_7", 1], ["b_8", 1], ["b_9", 1]],
  );
});

test("the continued-chain matrix kills endpoint-miss and disconnected-chain mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "chain-endpoint-attachment-continuation-production.json"));
  const baseDrag = scenario.actions.find((action) => action.id === "drag-base-four-bond-chain");
  const continuedDrag = scenario.actions.find((action) => action.id === "drag-continued-four-bond-chain-from-endpoint");
  assert.equal(baseDrag.completion.value, 4);
  assert.equal(continuedDrag.completion.value, 8);
  assert.ok(Math.abs((continuedDrag.to.x - continuedDrag.from.x) - 0.145) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.kind === "document-counts").expected, {
    nodes: 9,
    bonds: 8,
    molecules: 1,
    objects: 1,
  });
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-continued-chain-single-bond-semantics").expected.map(({ id, order }) => [id, order]),
    [["b_14", 1], ["b_15", 1], ["b_16", 1], ["b_17", 1]],
  );
});

test("aggregate scheduler limits fail closed at 10 CPU units and 30 GiB", () => {
  const budget = new ResourceBudget();
  assert.deepEqual(budget.admit("interactive-a", { cpuUnits: 4, memoryGiB: 10 }), { cpuUnits: 4, memoryGiB: 10 });
  assert.deepEqual(budget.admit("interactive-b", { cpuUnits: 4, memoryGiB: 10 }), { cpuUnits: 8, memoryGiB: 20 });
  assert.deepEqual(budget.admit("coordinator", { cpuUnits: 2, memoryGiB: 10 }), { cpuUnits: 10, memoryGiB: 30 });
  assert.throws(() => budget.admit("overflow", { cpuUnits: 1, memoryGiB: 1 }), /Resource budget exceeded/);
  assert.deepEqual(budget.release("interactive-b"), { cpuUnits: 6, memoryGiB: 20 });
});

test("the Chinese GUI progress checklist lists every registered scenario", async () => {
  const registry = await readValidatedDocument(join(guiTestsDir, "coverage", "registry-v1.json"));
  const scenarioIds = [...new Set(registry.entries.flatMap((entry) => entry.scenarioIds || []))].sort();
  const progress = await readFile(join(repositoryRoot, "docs", "gui-test-progress.zh-CN.md"), "utf8");
  assert.match(progress, new RegExp(`登记场景：\\*\\*${scenarioIds.length}\\*\\*`));
  for (const scenarioId of scenarioIds) {
    assert.equal(progress.includes(`| \`${scenarioId}\` |`), true, `progress checklist is missing ${scenarioId}`);
  }
});
