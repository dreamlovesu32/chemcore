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
    "scenario.core.atom.implicit-hydrogen-visibility-history-persistence.production",
    "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
    "scenario.core.atom.minus-symbol-attachment-persistence.production",
    "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
    "scenario.core.atom.periodic-alkali-alkaline-free-placement.production",
    "scenario.core.atom.periodic-common-value-matrix.production",
    "scenario.core.atom.periodic-group-fifteen-sixteen-free-placement.production",
    "scenario.core.atom.periodic-group-seventeen-free-placement.production",
    "scenario.core.atom.periodic-group-thirteen-fourteen-free-placement.production",
    "scenario.core.atom.periodic-noble-lanthanide-free-placement.production",
    "scenario.core.atom.periodic-period-four-transition-free-placement.production",
    "scenario.core.atom.periodic-period-seven-main-group-free-placement.production",
    "scenario.core.atom.periodic-representative-value-matrix.production",
    "scenario.core.atom.plus-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
    "scenario.core.bond.absolute-stereo-value-matrix.production",
    "scenario.core.bond.bold-center-click-style-cycle.production",
    "scenario.core.bond.center-click-cycle.production",
    "scenario.core.bond.dashed-center-click-style-cycle.production",
    "scenario.core.bond.dashed-double-center-click-cycle.production",
    "scenario.core.bond.double-placement-value-matrix.production",
    "scenario.core.bond.draw-single",
    "scenario.core.bond.draw-single.production",
    "scenario.core.bond.hashed-hollow-wedge-endpoint-reversal.production",
    "scenario.core.bond.query-order-value-matrix.production",
    "scenario.core.bond.reaction-participation-history-persistence.production",
    "scenario.core.bond.reaction-participation-value-matrix.production",
    "scenario.core.bond.ten-variant-persistence.production",
    "scenario.core.bond.topology-value-matrix.production",
    "scenario.core.bond.triple-hash-wavy-center-replacement.production",
    "scenario.core.bond.visibility-value-matrix.production",
    "scenario.core.bond.wedge-endpoint-reversal.production",
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
      "scenario.core.atom.implicit-hydrogen-visibility-history-persistence.production",
      "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
      "scenario.core.atom.minus-symbol-attachment-persistence.production",
      "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
      "scenario.core.atom.periodic-alkali-alkaline-free-placement.production",
      "scenario.core.atom.periodic-common-value-matrix.production",
      "scenario.core.atom.periodic-group-fifteen-sixteen-free-placement.production",
      "scenario.core.atom.periodic-group-seventeen-free-placement.production",
      "scenario.core.atom.periodic-group-thirteen-fourteen-free-placement.production",
      "scenario.core.atom.periodic-noble-lanthanide-free-placement.production",
      "scenario.core.atom.periodic-period-four-transition-free-placement.production",
      "scenario.core.atom.periodic-period-seven-main-group-free-placement.production",
      "scenario.core.atom.periodic-representative-value-matrix.production",
      "scenario.core.atom.plus-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
      "scenario.core.bond.absolute-stereo-value-matrix.production",
      "scenario.core.bond.bold-center-click-style-cycle.production",
      "scenario.core.bond.center-click-cycle.production",
      "scenario.core.bond.dashed-center-click-style-cycle.production",
      "scenario.core.bond.dashed-double-center-click-cycle.production",
      "scenario.core.bond.double-placement-value-matrix.production",
      "scenario.core.bond.draw-single",
      "scenario.core.bond.draw-single.production",
      "scenario.core.bond.hashed-hollow-wedge-endpoint-reversal.production",
      "scenario.core.bond.query-order-value-matrix.production",
      "scenario.core.bond.reaction-participation-history-persistence.production",
      "scenario.core.bond.reaction-participation-value-matrix.production",
      "scenario.core.bond.ten-variant-persistence.production",
      "scenario.core.bond.topology-value-matrix.production",
      "scenario.core.bond.triple-hash-wavy-center-replacement.production",
      "scenario.core.bond.visibility-value-matrix.production",
      "scenario.core.bond.wedge-endpoint-reversal.production",
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
    "scenario.core.atom.implicit-hydrogen-visibility-history-persistence.production",
    "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
    "scenario.core.atom.minus-symbol-attachment-persistence.production",
    "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
    "scenario.core.atom.periodic-alkali-alkaline-free-placement.production",
    "scenario.core.atom.periodic-common-value-matrix.production",
    "scenario.core.atom.periodic-group-fifteen-sixteen-free-placement.production",
    "scenario.core.atom.periodic-group-seventeen-free-placement.production",
    "scenario.core.atom.periodic-group-thirteen-fourteen-free-placement.production",
    "scenario.core.atom.periodic-noble-lanthanide-free-placement.production",
    "scenario.core.atom.periodic-period-four-transition-free-placement.production",
    "scenario.core.atom.periodic-period-seven-main-group-free-placement.production",
    "scenario.core.atom.periodic-representative-value-matrix.production",
    "scenario.core.atom.plus-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
    "scenario.core.bond.absolute-stereo-value-matrix.production",
    "scenario.core.bond.bold-center-click-style-cycle.production",
    "scenario.core.bond.center-click-cycle.production",
    "scenario.core.bond.dashed-center-click-style-cycle.production",
    "scenario.core.bond.dashed-double-center-click-cycle.production",
    "scenario.core.bond.double-placement-value-matrix.production",
    "scenario.core.bond.draw-single.production",
    "scenario.core.bond.hashed-hollow-wedge-endpoint-reversal.production",
    "scenario.core.bond.query-order-value-matrix.production",
    "scenario.core.bond.reaction-participation-history-persistence.production",
    "scenario.core.bond.reaction-participation-value-matrix.production",
    "scenario.core.bond.ten-variant-persistence.production",
    "scenario.core.bond.topology-value-matrix.production",
    "scenario.core.bond.triple-hash-wavy-center-replacement.production",
    "scenario.core.bond.visibility-value-matrix.production",
    "scenario.core.bond.wedge-endpoint-reversal.production",
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
    "scenario.core.atom.implicit-hydrogen-visibility-history-persistence.production",
    "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
    "scenario.core.atom.minus-symbol-attachment-persistence.production",
    "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
    "scenario.core.atom.periodic-alkali-alkaline-free-placement.production",
    "scenario.core.atom.periodic-common-value-matrix.production",
    "scenario.core.atom.periodic-group-fifteen-sixteen-free-placement.production",
    "scenario.core.atom.periodic-group-seventeen-free-placement.production",
    "scenario.core.atom.periodic-group-thirteen-fourteen-free-placement.production",
    "scenario.core.atom.periodic-noble-lanthanide-free-placement.production",
    "scenario.core.atom.periodic-period-four-transition-free-placement.production",
    "scenario.core.atom.periodic-period-seven-main-group-free-placement.production",
    "scenario.core.atom.periodic-representative-value-matrix.production",
    "scenario.core.atom.plus-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
    "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
    "scenario.core.bond.absolute-stereo-value-matrix.production",
    "scenario.core.bond.bold-center-click-style-cycle.production",
    "scenario.core.bond.center-click-cycle.production",
    "scenario.core.bond.dashed-center-click-style-cycle.production",
    "scenario.core.bond.dashed-double-center-click-cycle.production",
    "scenario.core.bond.double-placement-value-matrix.production",
    "scenario.core.bond.hashed-hollow-wedge-endpoint-reversal.production",
    "scenario.core.bond.query-order-value-matrix.production",
    "scenario.core.bond.reaction-participation-history-persistence.production",
    "scenario.core.bond.reaction-participation-value-matrix.production",
    "scenario.core.bond.ten-variant-persistence.production",
    "scenario.core.bond.topology-value-matrix.production",
    "scenario.core.bond.triple-hash-wavy-center-replacement.production",
    "scenario.core.bond.visibility-value-matrix.production",
    "scenario.core.bond.wedge-endpoint-reversal.production",
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
      "scenario.core.atom.implicit-hydrogen-visibility-history-persistence.production",
      "scenario.core.atom.lone-pair-symbol-attachment-persistence.production",
      "scenario.core.atom.minus-symbol-attachment-persistence.production",
      "scenario.core.atom.negative-charge-symbol-attachment-persistence.production",
      "scenario.core.atom.periodic-alkali-alkaline-free-placement.production",
      "scenario.core.atom.periodic-common-value-matrix.production",
      "scenario.core.atom.periodic-group-fifteen-sixteen-free-placement.production",
      "scenario.core.atom.periodic-group-seventeen-free-placement.production",
      "scenario.core.atom.periodic-group-thirteen-fourteen-free-placement.production",
      "scenario.core.atom.periodic-noble-lanthanide-free-placement.production",
      "scenario.core.atom.periodic-period-four-transition-free-placement.production",
      "scenario.core.atom.periodic-period-seven-main-group-free-placement.production",
      "scenario.core.atom.periodic-representative-value-matrix.production",
      "scenario.core.atom.plus-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-anion-symbol-attachment-persistence.production",
      "scenario.core.atom.radical-cation-symbol-attachment-persistence.production",
      "scenario.core.bond.absolute-stereo-value-matrix.production",
      "scenario.core.bond.bold-center-click-style-cycle.production",
      "scenario.core.bond.center-click-cycle.production",
      "scenario.core.bond.dashed-center-click-style-cycle.production",
      "scenario.core.bond.dashed-double-center-click-cycle.production",
      "scenario.core.bond.double-placement-value-matrix.production",
      "scenario.core.bond.draw-single",
      "scenario.core.bond.draw-single.production",
      "scenario.core.bond.hashed-hollow-wedge-endpoint-reversal.production",
      "scenario.core.bond.query-order-value-matrix.production",
      "scenario.core.bond.reaction-participation-history-persistence.production",
      "scenario.core.bond.reaction-participation-value-matrix.production",
      "scenario.core.bond.ten-variant-persistence.production",
      "scenario.core.bond.topology-value-matrix.production",
      "scenario.core.bond.triple-hash-wavy-center-replacement.production",
      "scenario.core.bond.visibility-value-matrix.production",
      "scenario.core.bond.wedge-endpoint-reversal.production",
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
    join(guiTestsDir, "scenarios", "core", "bond-absolute-stereo-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-bold-center-click-style-cycle-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-center-click-cycle-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-dashed-center-click-style-cycle-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-dashed-double-center-click-cycle-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-double-placement-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-hashed-hollow-wedge-endpoint-reversal-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-wedge-endpoint-reversal-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-query-order-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-topology-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-triple-hash-wavy-center-replacement-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-visibility-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-reaction-participation-history-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "bond-reaction-participation-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-six-planar-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-chair-benzene-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-bond-fusion-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-endpoint-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "ring-vertex-bond-continuation-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-charge-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-electron-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-lone-pair-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-minus-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-negative-charge-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-plus-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-radical-cation-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-radical-anion-symbol-attachment-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-element-label-persistence-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-alkali-alkaline-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-common-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-group-fifteen-sixteen-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-group-seventeen-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-group-thirteen-fourteen-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-noble-lanthanide-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-period-four-transition-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-period-seven-main-group-free-placement-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-periodic-representative-value-matrix-production.json"),
    join(guiTestsDir, "scenarios", "core", "atom-implicit-hydrogen-visibility-history-persistence-production.json"),
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
  assert.equal(result.summary.entries, 44);
  assert.equal(result.summary.scenarios, 68);
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

  const wrongPrimitiveTargetScenarios = structuredClone(scenarios);
  const hydrogenMenuAction = wrongPrimitiveTargetScenarios
    .find((scenario) => scenario.id === "core.atom.implicit-hydrogen-visibility-history-persistence.production")
    .actions.find((action) => action.id === "open-automatic-hydrogen-menu");
  hydrogenMenuAction.target = { strategy: "entity-id", value: "n_2" };
  const wrongPrimitiveTargetResult = await auditCoverage({ registry, scenarios: wrongPrimitiveTargetScenarios, scenarioPaths });
  assert.equal(wrongPrimitiveTargetResult.valid, false);
  assert.match(wrongPrimitiveTargetResult.errors.join("\n"), /entity-id resolves scene object ids only/);

  const unadvertisedCapabilityScenarios = structuredClone(scenarios);
  const reactionScenario = unadvertisedCapabilityScenarios
    .find((scenario) => scenario.id === "core.bond.reaction-participation-history-persistence.production");
  reactionScenario.capabilities = reactionScenario.capabilities
    .filter((capability) => capability !== "editor.bond.reaction-participation")
    .concat("editor.bond.unadvertised-mutant");
  const unadvertisedCapabilityResult = await auditCoverage({ registry, scenarios: unadvertisedCapabilityScenarios, scenarioPaths });
  assert.equal(unadvertisedCapabilityResult.valid, false);
  assert.match(unadvertisedCapabilityResult.errors.join("\n"), /requires capabilities not advertised by production-black-box: editor\.bond\.unadvertised-mutant/);

  const staleBondSelectionScenarios = structuredClone(scenarios);
  const staleBondSelectionScenario = staleBondSelectionScenarios
    .find((scenario) => scenario.id === "core.bond.reaction-participation-history-persistence.production");
  staleBondSelectionScenario.actions = staleBondSelectionScenario.actions
    .filter((action) => action.id !== "clear-created-molecule-selection");
  const staleBondSelectionResult = await auditCoverage({ registry, scenarios: staleBondSelectionScenarios, scenarioPaths });
  assert.equal(staleBondSelectionResult.valid, false);
  assert.match(staleBondSelectionResult.errors.join("\n"), /must immediately clear stale selection on page-background before opening its first bond-specific context menu/);
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

test("the representative periodic matrix kills swapped-number, truncated-row, wrong-target, and display-only mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-representative-value-matrix-production.json"));
  const expectedValues = [
    ["hydrogen", "H", 1, "n_2", 0.15257, 0.34],
    ["fluorine", "F", 9, "n_5", 0.37257, 0.34],
    ["silicon", "Si", 14, "n_8", 0.59257, 0.34],
    ["sulfur", "S", 16, "n_11", 0.81257, 0.34],
    ["iron", "Fe", 26, "n_14", 0.15257, 0.64],
    ["bromine", "Br", 35, "n_17", 0.37257, 0.64],
    ["uranium", "U", 92, "n_20", 0.59257, 0.64],
    ["oganesson", "Og", 118, "n_23", 0.81257, 0.64],
  ];
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `open-element-palette-for-${name}`).target.value),
    Array(8).fill('.quick-palette-toggle-element[data-quick-palette-mode="element"]'),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `apply-${name}`).completion.selector),
    expectedValues.map(([, , , nodeId]) => `[data-node-id="${nodeId}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `apply-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-representative-element-semantics").expected.map(({ id, element, atomicNumber, charge }) => ({ id, element, atomicNumber, charge })),
    expectedValues.map(([, element, atomicNumber, id]) => ({ id, element, atomicNumber, charge: 0 })),
  );
  assert.equal(scenario.oracles.find((oracle) => oracle.id === "saved-representative-element-counts").expected.nodes, 16);
});

test("the common periodic matrix kills untested-value swaps, wrong endpoints, and label-only mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-common-value-matrix-production.json"));
  const expectedValues = [
    ["boron", "B", 5, "n_2", 0.15257, 0.34, "BH2"],
    ["phosphorus", "P", 15, "n_5", 0.37257, 0.34, "PH2"],
    ["chlorine", "Cl", 17, "n_8", 0.59257, 0.34, "Cl"],
    ["iodine", "I", 53, "n_11", 0.81257, 0.34, "I"],
    ["sodium", "Na", 11, "n_14", 0.15257, 0.64, "Na"],
    ["magnesium", "Mg", 12, "n_17", 0.37257, 0.64, "Mg"],
    ["copper", "Cu", 29, "n_20", 0.59257, 0.64, "Cu"],
    ["gold", "Au", 79, "n_23", 0.81257, 0.64, "Au"],
  ];
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `apply-${name}`).completion.selector),
    expectedValues.map(([, , , nodeId]) => `[data-node-id="${nodeId}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `apply-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-common-element-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id, , , labelText]) => ({ id, element, atomicNumber, charge: 0, labelText, labelSourceText: labelText })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-common-element-counts").expected, {
    nodes: 16,
    bonds: 8,
    molecules: 8,
    objects: 8,
  });
});

test("the noble and lanthanide free-placement matrix kills bonded-only, collapsed-object, row-truncation, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-noble-lanthanide-free-placement-production.json"));
  const expectedValues = [
    ["helium", "He", 2, "n_1", 0.15, 0.34],
    ["neon", "Ne", 10, "n_2", 0.37, 0.34],
    ["argon", "Ar", 18, "n_3", 0.59, 0.34],
    ["krypton", "Kr", 36, "n_4", 0.81, 0.34],
    ["xenon", "Xe", 54, "n_5", 0.15, 0.64],
    ["radon", "Rn", 86, "n_6", 0.37, 0.64],
    ["lanthanum", "La", 57, "n_7", 0.59, 0.64],
    ["lutetium", "Lu", 71, "n_8", 0.81, 0.64],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).completion.selector),
    expectedValues.map(([, , , nodeId]) => `[data-node-id="${nodeId}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-noble-lanthanide-free-atom-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id]) => ({ id, element, atomicNumber, charge: 0, labelText: element, labelSourceText: element })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-free-atom-counts").expected, {
    nodes: 8,
    bonds: 0,
    molecules: 8,
    objects: 8,
  });
});

test("the alkali and alkaline-earth free-placement matrix kills adjacent-column, vertical-row, collapsed-object, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-alkali-alkaline-free-placement-production.json"));
  const expectedValues = [
    ["lithium", "Li", 3, "n_1", 0.15, 0.34],
    ["beryllium", "Be", 4, "n_2", 0.37, 0.34],
    ["potassium", "K", 19, "n_3", 0.59, 0.34],
    ["calcium", "Ca", 20, "n_4", 0.81, 0.34],
    ["rubidium", "Rb", 37, "n_5", 0.15, 0.64],
    ["strontium", "Sr", 38, "n_6", 0.37, 0.64],
    ["cesium", "Cs", 55, "n_7", 0.59, 0.64],
    ["barium", "Ba", 56, "n_8", 0.81, 0.64],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).completion.selector),
    expectedValues.map(([, , , nodeId]) => `[data-node-id="${nodeId}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-group-one-two-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id]) => ({ id, element, atomicNumber, charge: 0, labelText: element, labelSourceText: element })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-group-one-two-counts").expected, {
    nodes: 8,
    bonds: 0,
    molecules: 8,
    objects: 8,
  });
});

test("the Group 13 and 14 free-placement matrix kills cross-column, late-row, bare-symbol, wrong-valence, collapsed-object, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-group-thirteen-fourteen-free-placement-production.json"));
  const expectedValues = [
    ["aluminum", "Al", 13, "n_1", 0.15, 0.34, 3, "AlH3"],
    ["gallium", "Ga", 31, "n_2", 0.37, 0.34, 3, "GaH3"],
    ["indium", "In", 49, "n_3", 0.59, 0.34, 3, "InH3"],
    ["thallium", "Tl", 81, "n_4", 0.81, 0.34, 3, "TlH3"],
    ["germanium", "Ge", 32, "n_5", 0.15, 0.64, 4, "GeH4"],
    ["tin", "Sn", 50, "n_6", 0.37, 0.64, 4, "SnH4"],
    ["lead", "Pb", 82, "n_7", 0.59, 0.64, 4, "PbH4"],
    ["flerovium", "Fl", 114, "n_8", 0.81, 0.64, 4, "FlH4"],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).completion.selector),
    expectedValues.map(([, , , nodeId]) => `[data-node-id="${nodeId}"]`),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-group-thirteen-fourteen-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id, , , numHydrogens, labelText]) => ({ id, element, atomicNumber, charge: 0, numHydrogens, labelText, labelSourceText: labelText })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-group-thirteen-fourteen-counts").expected, {
    nodes: 8,
    bonds: 0,
    molecules: 8,
    objects: 8,
  });
});

test("the Group 15 and 16 free-placement matrix kills cross-column, late-row, bare-symbol, wrong-valence, collapsed-object, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-group-fifteen-sixteen-free-placement-production.json"));
  const expectedValues = [
    ["arsenic", "As", 33, "n_1", 0.15, 0.34, 3, "AsH3"],
    ["antimony", "Sb", 51, "n_2", 0.37, 0.34, 3, "SbH3"],
    ["bismuth", "Bi", 83, "n_3", 0.59, 0.34, 3, "BiH3"],
    ["moscovium", "Mc", 115, "n_4", 0.81, 0.34, 3, "McH3"],
    ["selenium", "Se", 34, "n_5", 0.15, 0.64, 2, "SeH2"],
    ["tellurium", "Te", 52, "n_6", 0.37, 0.64, 2, "TeH2"],
    ["polonium", "Po", 84, "n_7", 0.59, 0.64, 2, "PoH2"],
    ["livermorium", "Lv", 116, "n_8", 0.81, 0.64, 2, "LvH2"],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).completion.selector),
    expectedValues.map(([, , , nodeId]) => `[data-node-id="${nodeId}"]`),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-group-fifteen-sixteen-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id, , , numHydrogens, labelText]) => ({ id, element, atomicNumber, charge: 0, numHydrogens, labelText, labelSourceText: labelText })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-group-fifteen-sixteen-counts").expected, {
    nodes: 8,
    bonds: 0,
    molecules: 8,
    objects: 8,
  });
});

test("the remaining Group 17 free-placement cell kills missing-row, swapped-value, bare-symbol, wrong-valence, collapsed-object, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-group-seventeen-free-placement-production.json"));
  const expectedValues = [
    ["astatine", "At", 85, "n_1", 0.35, 0.49, "AtH"],
    ["tennessine", "Ts", 117, "n_2", 0.65, 0.49, "TsH"],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-group-seventeen-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id, , , labelText]) => ({ id, element, atomicNumber, charge: 0, numHydrogens: 1, labelText, labelSourceText: labelText })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-group-seventeen-counts").expected, {
    nodes: 2,
    bonds: 0,
    molecules: 2,
    objects: 2,
  });
});

test("the remaining period-four transition cell kills missing-row, row-order, main-group-valence, collapsed-object, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-period-four-transition-free-placement-production.json"));
  const expectedValues = [
    ["scandium", "Sc", 21, "n_1", 0.15, 0.34],
    ["titanium", "Ti", 22, "n_2", 0.37, 0.34],
    ["vanadium", "V", 23, "n_3", 0.59, 0.34],
    ["chromium", "Cr", 24, "n_4", 0.81, 0.34],
    ["manganese", "Mn", 25, "n_5", 0.15, 0.64],
    ["cobalt", "Co", 27, "n_6", 0.37, 0.64],
    ["nickel", "Ni", 28, "n_7", 0.59, 0.64],
    ["zinc", "Zn", 30, "n_8", 0.81, 0.64],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-period-four-transition-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id]) => ({ id, element, atomicNumber, charge: 0, numHydrogens: 0, labelText: element, labelSourceText: element })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-period-four-transition-counts").expected, {
    nodes: 8,
    bonds: 0,
    molecules: 8,
    objects: 8,
  });
});

test("the remaining period-seven main-group cell kills missing-row, cross-group, zero-versus-H3, collapsed-object, and accidental-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-periodic-period-seven-main-group-free-placement-production.json"));
  const expectedValues = [
    ["francium", "Fr", 87, "n_1", 0.25, 0.49, 0, "Fr"],
    ["radium", "Ra", 88, "n_2", 0.5, 0.49, 0, "Ra"],
    ["nihonium", "Nh", 113, "n_3", 0.75, 0.49, 3, "NhH3"],
  ];
  assert.equal(scenario.actions.some((action) => action.id === "activate-single-bond-tool"), false);
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `choose-${name}`).target.value),
    expectedValues.map(([, symbol, atomicNumber]) => `.periodic-element-button[data-element-symbol="${symbol}"][data-element-atomic-number="${atomicNumber}"]`),
  );
  assert.deepEqual(
    expectedValues.map(([name]) => scenario.actions.find((action) => action.id === `place-${name}`).at),
    expectedValues.map(([, , , , x, y]) => ({ x, y })),
  );
  assert.deepEqual(
    scenario.oracles.find((oracle) => oracle.id === "saved-period-seven-main-group-semantics").expected,
    expectedValues.map(([, element, atomicNumber, id, , , numHydrogens, labelText]) => ({ id, element, atomicNumber, charge: 0, numHydrogens, labelText, labelSourceText: labelText })),
  );
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-period-seven-main-group-counts").expected, {
    nodes: 3,
    bonds: 0,
    molecules: 3,
    objects: 3,
  });
});

test("the implicit-hydrogen matrix kills wrong-menu-value, missing-history, and automatic-zero mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-implicit-hydrogen-visibility-history-persistence-production.json"));
  const initialMenu = scenario.actions.find((action) => action.id === "open-automatic-hydrogen-menu");
  const hiddenMenu = scenario.actions.find((action) => action.id === "open-hidden-hydrogen-menu");
  const restoredMenu = scenario.actions.find((action) => action.id === "open-restored-automatic-menu");
  const hide = scenario.actions.find((action) => action.id === "hide-implicit-hydrogens");
  const restore = scenario.actions.find((action) => action.id === "restore-automatic-hydrogens");
  const undo = scenario.actions.find((action) => action.id === "undo-second-hide");
  const redo = scenario.actions.find((action) => action.id === "redo-second-hide");
  assert.deepEqual(initialMenu.target, { strategy: "selector", value: '[data-node-id="n_2"]' });
  assert.deepEqual(hiddenMenu.target, initialMenu.target);
  assert.deepEqual(restoredMenu.target, initialMenu.target);
  assert.match(initialMenu.completion.selector, /data-canvas-context-value="auto"/);
  assert.equal(hide.target.name, "Hide");
  assert.equal(hide.completion.text, "N");
  assert.equal(restore.target.name, "Automatic");
  assert.equal(restore.completion.text, "NH2");
  assert.equal(undo.key, "Control+Z");
  assert.equal(undo.completion.text, "NH2");
  assert.equal(redo.key, "Control+Y");
  assert.equal(redo.completion.text, "N");
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-explicit-hidden-hydrogen-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 0, numHydrogens: 0, numHydrogensOverride: 0, labelText: "N", labelSourceText: "N" },
  ]);
});

test("the bond reaction matrix kills wrong-enum, missing-annotation, and missing-history mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-reaction-participation-history-persistence-production.json"));
  const initialMenu = scenario.actions.find((action) => action.id === "open-initial-bond-reaction-menu");
  const clearSelection = scenario.actions.find((action) => action.id === "clear-created-molecule-selection");
  const apply = scenario.actions.find((action) => action.id === "set-make-and-change");
  const changedMenu = scenario.actions.find((action) => action.id === "open-changed-bond-reaction-menu");
  const undo = scenario.actions.find((action) => action.id === "undo-reaction-participation");
  const redo = scenario.actions.find((action) => action.id === "redo-reaction-participation");
  assert.deepEqual(initialMenu.target, { strategy: "selector", value: 'line[data-bond-id="b_3"]' });
  assert.equal(scenario.actions.indexOf(clearSelection), scenario.actions.indexOf(initialMenu) - 1);
  assert.deepEqual(clearSelection.target, { strategy: "world-geometry", value: "page-background" });
  assert.equal(clearSelection.completion.selector, '[data-layer="editor-overlay"] > *');
  assert.match(initialMenu.completion.selector, /reaction-participation:unspecified/);
  assert.equal(apply.target.name, "Make and Change");
  assert.equal(apply.completion.text, "Rxn");
  assert.match(changedMenu.completion.selector, /reaction-participation:make-and-change/);
  assert.equal(undo.key, "Control+Z");
  assert.equal(undo.completion.value, 0);
  assert.equal(redo.key, "Control+Y");
  assert.equal(redo.completion.text, "Rxn");
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-make-and-change-bond-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null, reactionParticipation: "make-and-change" },
  ]);
});

test("the bond query-order matrix kills skipped-value, stale-annotation, wrong-order, and stale-persistence mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-query-order-value-matrix-production.json"));
  assert.ok(scenario.capabilities.includes("editor.bond.query-order"));
  const appliedValues = scenario.actions
    .filter((action) => action.id.startsWith("set-"))
    .map((action) => action.target.name);
  assert.deepEqual(appliedValues, [
    "Single or Double (S/D)",
    "Single or Aromatic (S/A)",
    "Double or Aromatic (D/A)",
    "None",
    "Double or Aromatic (D/A)",
  ]);

  const verifiedValues = scenario.actions
    .filter((action) => action.id.startsWith("open-") && action.id.endsWith("-menu"))
    .map((action) => action.completion.selector.match(/query-orders:([a-z-]+)/)?.[1]);
  assert.deepEqual(verifiedValues, [
    "none",
    "single-double",
    "single-aromatic",
    "double-aromatic",
    "none",
    "double-aromatic",
  ]);

  for (const [id, text] of [
    ["set-single-double", "S/D"],
    ["set-single-aromatic", "S/A"],
    ["set-double-aromatic", "D/A"],
    ["set-final-double-aromatic", "D/A"],
  ]) {
    const action = scenario.actions.find((candidate) => candidate.id === id);
    assert.equal(action.completion.selector, 'text[data-bond-id="b_3"]');
    assert.equal(action.completion.text, text);
  }
  const clear = scenario.actions.find((action) => action.id === "set-cleared-none");
  assert.equal(clear.completion.selector, 'text[data-bond-id="b_3"]');
  assert.equal(clear.completion.value, 0);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-double-aromatic-query-order-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null, queryOrders: ["double", "aromatic"] },
  ]);
});

test("the bond topology matrix kills skipped-value, stale-annotation, wrong-enum, and stale-persistence mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-topology-value-matrix-production.json"));
  assert.ok(scenario.capabilities.includes("editor.bond.topology"));
  const appliedValues = scenario.actions
    .filter((action) => action.id.startsWith("set-"))
    .map((action) => action.target.name);
  assert.deepEqual(appliedValues, ["Ring", "Chain", "Ring or Chain", "Unspecified", "Ring or Chain"]);

  const verifiedValues = scenario.actions
    .filter((action) => action.id.startsWith("open-") && action.id.endsWith("-menu"))
    .map((action) => action.completion.selector.match(/topology:([a-z-]+)/)?.[1]);
  assert.deepEqual(verifiedValues, ["unspecified", "ring", "chain", "ring-or-chain", "unspecified", "ring-or-chain"]);

  for (const [id, text] of [
    ["set-ring", "Rng"],
    ["set-chain", "Chn"],
    ["set-ring-or-chain", "R/C"],
    ["set-final-ring-or-chain", "R/C"],
  ]) {
    const action = scenario.actions.find((candidate) => candidate.id === id);
    assert.equal(action.completion.selector, 'text[data-bond-id="b_3"]');
    assert.equal(action.completion.text, text);
  }
  const clear = scenario.actions.find((action) => action.id === "set-cleared-unspecified");
  assert.equal(clear.completion.selector, 'text[data-bond-id="b_3"]');
  assert.equal(clear.completion.value, 0);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-ring-or-chain-topology-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null, topology: "ring-or-chain" },
  ]);
});

test("the absolute bond stereo matrix kills skipped-value, hidden-display, stale-annotation, wrong-enum, and stale-persistence mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-absolute-stereo-value-matrix-production.json"));
  assert.ok(scenario.capabilities.includes("editor.bond.absolute-stereo"));
  assert.ok(scenario.capabilities.includes("editor.bond.visibility"));
  assert.equal(scenario.actions.find((action) => action.id === "set-show-stereo").target.name, "Show");

  const appliedValues = scenario.actions
    .filter((action) => ["set-e", "set-z", "set-none", "set-unspecified", "set-final-z"].includes(action.id))
    .map((action) => action.target.name);
  assert.deepEqual(appliedValues, ["E", "Z", "None", "Unspecified", "Z"]);

  const verifiedValues = scenario.actions
    .filter((action) => action.id.startsWith("open-") && action.completion.selector?.includes("absolute-stereo:"))
    .map((action) => action.completion.selector.match(/absolute-stereo:([a-z]+)/)?.[1]);
  assert.deepEqual(verifiedValues, ["unspecified", "e", "z", "none", "unspecified", "z"]);

  assert.equal(scenario.actions.find((action) => action.id === "set-e").completion.text, "(E)");
  assert.equal(scenario.actions.find((action) => action.id === "set-z").completion.text, "(Z)");
  assert.equal(scenario.actions.find((action) => action.id === "set-final-z").completion.text, "(Z)");
  assert.equal(scenario.actions.find((action) => action.id === "set-none").completion.value, 0);
  assert.equal(scenario.actions.find((action) => action.id === "set-unspecified").completion.value, 0);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-z-show-stereo-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null, absoluteStereo: "z", showStereo: true },
  ]);
});

test("the bond visibility matrix kills skipped-value, coupled-display, stale-annotation, and dropped-override mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-visibility-value-matrix-production.json"));
  assert.ok(scenario.capabilities.includes("editor.bond.visibility"));
  for (const [prefix, expected] of [
    ["set-query-", ["Show", "Inherit Document Setting", "Hide"]],
    ["set-reaction-", ["Show", "Inherit Document Setting", "Hide"]],
    ["set-stereo-", ["Hide", "Inherit Document Setting", "Show"]],
  ]) {
    assert.deepEqual(
      scenario.actions.filter((action) => action.id.startsWith(prefix)).map((action) => action.target.name),
      expected,
    );
  }

  assert.match(scenario.actions.find((action) => action.id === "open-query-inherit-menu").completion.selector, /show-query:inherit/);
  assert.match(scenario.actions.find((action) => action.id === "open-query-show-menu").completion.selector, /show-query:true/);
  assert.match(scenario.actions.find((action) => action.id === "open-reaction-property-menu").completion.selector, /show-query:false/);
  assert.match(scenario.actions.find((action) => action.id === "open-reaction-inherit-menu").completion.selector, /show-reaction:inherit/);
  assert.match(scenario.actions.find((action) => action.id === "open-reaction-show-menu").completion.selector, /show-reaction:true/);
  assert.match(scenario.actions.find((action) => action.id === "open-absolute-stereo-menu").completion.selector, /show-reaction:false/);
  assert.match(scenario.actions.find((action) => action.id === "open-stereo-inherit-menu").completion.selector, /show-stereo:inherit/);
  assert.match(scenario.actions.find((action) => action.id === "open-stereo-hide-menu").completion.selector, /show-stereo:false/);
  assert.match(scenario.actions.find((action) => action.id === "open-final-visibility-menu").completion.selector, /show-stereo:true/);

  assert.equal(scenario.actions.find((action) => action.id === "set-query-hide").completion.value, 0);
  assert.equal(scenario.actions.find((action) => action.id === "set-reaction-hide").completion.value, 0);
  assert.equal(scenario.actions.find((action) => action.id === "set-stereo-hide").completion.value, 0);
  assert.equal(scenario.actions.find((action) => action.id === "set-stereo-show").completion.text, "(Z)");
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-independent-visibility-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null, topology: "ring", reactionParticipation: "make-and-change", absoluteStereo: "z", showQuery: false, showReaction: false, showStereo: true },
  ]);
});

test("the double-bond placement matrix kills skipped-value, aliased-side, stale-checkmark, and dropped-persistence mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-double-placement-value-matrix-production.json"));
  assert.ok(scenario.capabilities.includes("editor.bond.double-placement"));
  const clearSelection = scenario.actions.find((action) => action.id === "clear-created-molecule-selection");
  const initialMenu = scenario.actions.find((action) => action.id === "open-initial-center-menu");
  assert.equal(scenario.actions.indexOf(clearSelection), scenario.actions.indexOf(initialMenu) - 1);
  assert.deepEqual(initialMenu.target, { strategy: "selector", value: 'line[data-bond-id="b_3"]' });

  const appliedValues = scenario.actions
    .filter((action) => action.id.startsWith("set-double-") || action.id === "set-final-double-left")
    .map((action) => action.target.name);
  assert.deepEqual(appliedValues, ["Left", "Right", "Center", "Left"]);

  const verifiedValues = scenario.actions
    .filter((action) => action.id.startsWith("open-") && action.completion.selector?.includes("data-canvas-context-value"))
    .map((action) => action.completion.selector.match(/double-(left|right|center)/)?.[1]);
  assert.deepEqual(verifiedValues, ["center", "left", "right", "center", "left"]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-left-double-placement-semantics").expected, [
    { id: "b_3", order: 2, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", doublePlacement: "left", stereoKind: null, wideEnd: null },
  ]);
});

test("the wedge endpoint-reversal cell kills ignored-click, recreated-bond, and wrong-wide-end mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-wedge-endpoint-reversal-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.endpoint-reversal"));
  const draw = scenario.actions.find((action) => action.id === "draw-solid-wedge");
  const reverse = scenario.actions.find((action) => action.id === "reverse-wedge-at-center");
  assert.equal(scenario.actions.indexOf(reverse), scenario.actions.indexOf(draw) + 1);
  assert.deepEqual(reverse.target, { strategy: "selector", value: '[data-bond-id="b_3"]' });
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-reversed-wedge-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: "solid-wedge", wideEnd: "begin" },
  ]);
});

test("the Hashed/Hollow wedge cell kills cross-kind, cross-target, recreated-bond, and wrong-wide-end mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-hashed-hollow-wedge-endpoint-reversal-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.endpoint-reversal"));
  const reversals = scenario.actions.filter((action) => action.id.startsWith("reverse-"));
  assert.deepEqual(reversals.map((action) => action.id), [
    "reverse-hashed-wedge-at-center",
    "reverse-hollow-wedge-at-center",
  ]);
  assert.deepEqual(reversals.map((action) => action.target.value), [
    '[data-role="document-bond"][data-bond-id="b_3"]',
    '[data-role="document-bond"][data-bond-id="b_6"]',
  ]);
  assert.deepEqual(reversals.map((action) => action.completion.value), [1, 2]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-reversed-stereo-wedge-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: "hashed-wedge", wideEnd: "begin" },
    { id: "b_6", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: "hollow-wedge", wideEnd: "begin" },
  ]);
});

test("the Triple/Hash/Wavy replacement cell kills wrong-style, stale-double, cross-target, and recreated-bond mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-triple-hash-wavy-center-replacement-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.center-click-cycle"));
  const replacements = scenario.actions.filter((action) => action.id.startsWith("replace-"));
  assert.deepEqual(replacements.map((action) => action.id), [
    "replace-first-with-triple",
    "replace-second-with-hash",
    "replace-third-with-wavy",
  ]);
  assert.deepEqual(replacements.map((action) => action.target.value), [
    '[data-role="document-bond"][data-bond-id="b_3"]',
    '[data-role="document-bond"][data-bond-id="b_6"]',
    '[data-role="document-bond"][data-bond-id="b_9"]',
  ]);
  assert.ok(replacements.every((action) => action.completion.kind === "dom-distinct-count"));
  assert.ok(replacements.every((action) => action.completion.attribute === "data-bond-id" && action.completion.value === 3));
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-triple-hash-wavy-replacement-semantics").expected, [
    { id: "b_3", order: 3, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null },
    { id: "b_6", order: 1, mainLineStyle: "hash", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null },
    { id: "b_9", order: 1, mainLineStyle: "wavy", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null },
  ]);
});

test("the Single-tool center-click cycle kills skipped-state, duplicate-bond, and wrong-final-side mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-center-click-cycle-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.center-click-cycle"));
  const cycleActions = scenario.actions.filter((action) => action.id.startsWith("cycle-"));
  assert.deepEqual(cycleActions.map((action) => action.id), [
    "cycle-single-to-left-double",
    "cycle-left-double-to-center",
    "cycle-center-to-right-double",
  ]);
  assert.ok(cycleActions.every((action) => action.target.value === '[data-bond-id="b_3"]'));
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-right-double-after-center-cycle").expected, [
    { id: "b_3", order: 2, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", doublePlacement: "right", stereoKind: null, wideEnd: null },
  ]);
});

test("the directly drawn Dashed-solid-double cycle kills wrong-default, premature-stop, wrong-side, and stale-line-style mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-dashed-double-center-click-cycle-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.center-click-cycle"));
  const logicalBondActions = scenario.actions.filter((action) => action.id === "draw-centered-right-dashed-cycle-target" || action.id.startsWith("cycle-"));
  assert.deepEqual(logicalBondActions.filter((action) => action.id.startsWith("cycle-")).map((action) => action.id), [
    "cycle-centered-right-dashed-to-left",
    "cycle-left-to-centered-left-dashed",
    "cycle-centered-left-dashed-to-right",
  ]);
  assert.ok(logicalBondActions.filter((action) => action.id.startsWith("cycle-")).every((action) => action.target.value === '[data-role="document-bond"][data-bond-id="b_3"]'));
  assert.ok(logicalBondActions.every((action) => action.completion.kind === "dom-distinct-count"));
  assert.ok(logicalBondActions.every((action) => action.completion.selector === "[data-bond-id]"));
  assert.ok(logicalBondActions.every((action) => action.completion.attribute === "data-bond-id"));
  assert.ok(logicalBondActions.every((action) => action.completion.value === 1));
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-right-dashed-double-after-full-cycle").expected, [
    { id: "b_3", order: 2, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "dashed", mainLineWeight: "normal", doublePlacement: "right", stereoKind: null, wideEnd: null },
  ]);
});

test("the Dashed-bond style cycle kills skipped-main-dash, skipped-centered-both-dashed, wrong-exit-side, and duplicate mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-dashed-center-click-style-cycle-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.center-click-cycle"));
  const setupActions = scenario.actions.filter((action) => action.id.startsWith("setup-"));
  const cycleActions = scenario.actions.filter((action) => action.id.startsWith("cycle-"));
  assert.deepEqual(setupActions.map((action) => action.id), [
    "setup-single-to-left-double",
    "setup-left-double-to-center",
    "setup-center-to-right-double",
  ]);
  assert.deepEqual(cycleActions.map((action) => action.id), [
    "cycle-right-solid-to-right-outer-dashed",
    "cycle-right-outer-dashed-to-both-dashed",
    "cycle-right-both-dashed-to-centered-both-dashed",
    "cycle-centered-both-dashed-to-left-outer-dashed",
  ]);
  const logicalClicks = [...setupActions, ...cycleActions];
  assert.ok(logicalClicks.every((action) => action.target.value === '[data-role="document-bond"][data-bond-id="b_3"]'));
  assert.ok(logicalClicks.every((action) => action.completion.kind === "dom-distinct-count"));
  assert.ok(logicalClicks.every((action) => action.completion.attribute === "data-bond-id" && action.completion.value === 1));
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-left-outer-dashed-after-full-style-cycle").expected, [
    { id: "b_3", order: 2, mainLineStyle: "solid", leftLineStyle: "dashed", rightLineStyle: "solid", mainLineWeight: "normal", doublePlacement: "left", stereoKind: null, wideEnd: null },
  ]);
});

test("the Bold-bond style cycle kills skipped-centered states, wrong-exit-side, stale-weight, and duplicate mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-bold-center-click-style-cycle-production.json"));
  assert.ok(scenario.coverage.features.includes("editor.bond.center-click-cycle"));
  const cycleActions = scenario.actions.filter((action) => action.id.startsWith("cycle-"));
  assert.deepEqual(cycleActions.map((action) => action.id), [
    "cycle-plain-single-to-bold-single",
    "cycle-bold-single-to-right-main-bold",
    "cycle-right-main-bold-to-centered-right-outer-bold",
    "cycle-centered-right-outer-bold-to-left-main-bold",
    "cycle-left-main-bold-to-centered-left-outer-bold",
    "cycle-centered-left-outer-bold-to-right-main-bold",
  ]);
  assert.ok(cycleActions.every((action) => action.target.value === '[data-role="document-bond"][data-bond-id="b_3"]'));
  assert.ok(cycleActions.every((action) => action.completion.kind === "dom-distinct-count"));
  assert.ok(cycleActions.every((action) => action.completion.attribute === "data-bond-id" && action.completion.value === 1));
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-right-main-bold-after-full-style-cycle").expected, [
    { id: "b_3", order: 2, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "bold", doublePlacement: "right", stereoKind: null, wideEnd: null },
  ]);
});

test("the bond reaction value matrix kills skipped-value, wrong-display, and stale-persistence mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "bond-reaction-participation-value-matrix-production.json"));
  const appliedValues = scenario.actions
    .filter((action) => action.id.startsWith("set-"))
    .map((action) => action.target.name);
  assert.deepEqual(appliedValues, [
    "Reaction Center",
    "Unspecified",
    "Make or Break",
    "Change Type",
    "Not Reaction Center",
    "No Change",
    "Unmapped",
  ]);

  const verifiedValues = scenario.actions
    .filter((action) => action.id.startsWith("open-") && action.id.endsWith("-menu"))
    .map((action) => action.completion.selector.match(/reaction-participation:([a-z-]+)/)?.[1]);
  assert.deepEqual(verifiedValues, [
    "unspecified",
    "reaction-center",
    "unspecified",
    "make-or-break",
    "change-type",
    "not-reaction-center",
    "no-change",
    "unmapped",
  ]);

  for (const id of ["set-reaction-center", "set-make-or-break", "set-change-type"]) {
    const action = scenario.actions.find((candidate) => candidate.id === id);
    assert.equal(action.completion.selector, 'text[data-bond-id="b_3"]');
    assert.equal(action.completion.text, "Rxn");
  }
  for (const id of ["set-explicit-unspecified", "set-not-reaction-center"]) {
    const action = scenario.actions.find((candidate) => candidate.id === id);
    assert.equal(action.completion.selector, 'text[data-bond-id="b_3"]');
    assert.equal(action.completion.value, 0);
  }
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-unmapped-bond-semantics").expected, [
    { id: "b_3", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null, reactionParticipation: "unmapped" },
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

test("the uncircled plus matrix kills circle-plus aliases and detached-charge mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-plus-symbol-attachment-persistence-production.json"));
  const choice = scenario.actions.find((action) => action.id === "choose-plus");
  const attachment = scenario.actions.find((action) => action.id === "attach-plus-to-nitrogen");
  assert.deepEqual(choice.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(choice.completion.selector, 'button[data-secondary-value="symbol-kind-plus"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-plus-ammonium-node-semantics").expected, [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 1, numHydrogens: 3, labelText: "NH3", labelSourceText: "NH3" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-plus-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "plus", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "charge", chargeDelta: 1, radicalDelta: 0, attachedAtomId: "n_2", attachmentSource: "auto" },
  ]);
});

test("the uncircled minus matrix kills circle-minus aliases and detached-charge mutants", async () => {
  const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "atom-minus-symbol-attachment-persistence-production.json"));
  const choice = scenario.actions.find((action) => action.id === "choose-minus");
  const attachment = scenario.actions.find((action) => action.id === "attach-minus-to-oxygen");
  assert.deepEqual(choice.target.scope, { role: "toolbar", name: "Secondary toolbar" });
  assert.equal(choice.completion.selector, 'button[data-secondary-value="symbol-kind-minus"].is-selected');
  assert.ok(Math.abs(attachment.at.x - 0.37257) < Number.EPSILON);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-minus-alkoxide-node-semantics").expected, [
    { id: "n_2", element: "O", atomicNumber: 8, charge: -1, numHydrogens: 0, labelText: "O", labelSourceText: "O" },
  ]);
  assert.deepEqual(scenario.oracles.find((oracle) => oracle.id === "saved-minus-symbol-attachment-semantics").expected, [
    { id: "obj_symbol_4", kind: "minus", payloadFill: "#000000", symbolStyle: "default", chemicalRole: "charge", chargeDelta: -1, radicalDelta: 0, attachedAtomId: "n_2", attachmentSource: "auto" },
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
