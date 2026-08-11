import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { canonicalJson, evidenceKey } from "../src/protocol/canonical.mjs";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { assertValidDocument, readValidatedDocument, validateDocument } from "../src/protocol/validate.mjs";

const scenarioPath = join(guiTestsDir, "scenarios", "core", "draw-single-bond.json");
const multiObjectScenarioPath = join(guiTestsDir, "scenarios", "core", "multi-bond-clipboard-delete-production.json");
const mixedObjectScenarioPath = join(guiTestsDir, "scenarios", "core", "mixed-bond-arrow-clipboard-production.json");
const nestedGroupScenarioPath = join(guiTestsDir, "scenarios", "core", "nested-mixed-group-clipboard-production.json");
const regionAdditiveScenarioPath = join(guiTestsDir, "scenarios", "core", "region-additive-mixed-cardinalities-production.json");
const crossDocumentClipboardScenarioPath = join(guiTestsDir, "scenarios", "core", "cross-document-clipboard-production.json");
const lockedTransformScenarioPath = join(guiTestsDir, "scenarios", "core", "locked-transform-production.json");
const lockedMoleculeArrowTransformScenarioPath = join(guiTestsDir, "scenarios", "core", "locked-molecule-arrow-transform-production.json");
const lockedGroupAncestorTransformScenarioPath = join(guiTestsDir, "scenarios", "core", "locked-group-ancestor-transform-production.json");
const textLineSpacingScenarioPath = join(guiTestsDir, "scenarios", "core", "text-line-spacing-validation-production.json");
const textExistingEditScenarioPath = join(guiTestsDir, "scenarios", "core", "text-existing-edit-history-production.json");
const frontendSelectionScenarioPath = join(guiTestsDir, "scenarios", "core", "frontend-selection-geometry-production.json");

test("scenario, coverage registry, and impact graph validate", async () => {
  await readValidatedDocument(scenarioPath);
  await readValidatedDocument(multiObjectScenarioPath);
  await readValidatedDocument(mixedObjectScenarioPath);
  await readValidatedDocument(nestedGroupScenarioPath);
  await readValidatedDocument(regionAdditiveScenarioPath);
  await readValidatedDocument(crossDocumentClipboardScenarioPath);
  await readValidatedDocument(lockedTransformScenarioPath);
  await readValidatedDocument(lockedMoleculeArrowTransformScenarioPath);
  await readValidatedDocument(lockedGroupAncestorTransformScenarioPath);
  await readValidatedDocument(textLineSpacingScenarioPath);
  await readValidatedDocument(textExistingEditScenarioPath);
  await readValidatedDocument(frontendSelectionScenarioPath);
  await readValidatedDocument(join(guiTestsDir, "coverage", "registry-v1.json"));
  await readValidatedDocument(join(guiTestsDir, "coverage", "impact-v1.json"));
  await readValidatedDocument(join(guiTestsDir, "environments", "windows-gui-worker-current.json"));
  await assertValidDocument({
    schema: "chemsema.gui.guest-agent.v1",
    agentVersion: "0.1.0",
    processId: 100,
    sessionId: 0,
    account: "guest\\chemsema-test",
    inputDesktop: null,
    interactiveReady: false,
    foreground: null,
  }, "guest agent fixture");
  await assertValidDocument({ schema: "chemsema.gui.cdp-server.v1", status: "ready", processId: 101, sessionId: 0, account: "NT AUTHORITY\\SYSTEM", port: 9223 }, "CDP server fixture");
  await assertValidDocument({ schema: "chemsema.gui.cdp-request.v1", id: "a".repeat(32), requestBase64: "e30=" }, "CDP request fixture");
  await assertValidDocument({
    schema: "chemsema.gui.cdp-response.v1",
    id: "a".repeat(32),
    status: "passed",
    bridge: { schema: "chemsema.gui.cdp-bridge.v1", status: "passed" },
  }, "CDP response fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction.v1",
    actionId: "activate-bond-tool",
    input: { kind: "click", x: 10, y: 20, button: "left", modifiers: ["Shift"] },
    completion: { kind: "actionable", timeoutMs: 8000 },
    budgetMs: 30000,
  }, "action transaction fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction.v1",
    actionId: "select-two-objects",
    input: { kind: "key", key: "Control+A" },
    completion: {
      kind: "dom-distinct-count",
      selector: "[data-role=\"document-graphic\"][data-object-id]",
      attribute: "data-object-id",
      operator: "eq",
      value: 2,
      timeoutMs: 8000,
    },
    budgetMs: 30000,
  }, "distinct object action transaction fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction.v1",
    actionId: "undo-text-edit",
    input: { kind: "key", key: "Control+Z" },
    completion: { kind: "dom-text", selector: "[data-object-id=\"obj_text_1\"]", text: "Original text", timeoutMs: 8000 },
    budgetMs: 30000,
  }, "exact DOM text action transaction fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction.v1",
    actionId: "drag-mixed-selection",
    input: { kind: "drag", from: [100, 100], to: [130, 100], steps: 8, button: "left" },
    completion: {
      kind: "entity-rect-deltas",
      entities: [
        { entityId: "obj_locked", operator: "stationary", toleranceWorld: 0.5 },
        { entityId: "obj_editable", operator: "moved", toleranceWorld: 5 },
      ],
      timeoutMs: 8000,
    },
    budgetMs: 30000,
  }, "mixed entity rectangle action transaction fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction-receipt.v1",
    input: {}, before: {}, after: {}, completion: { actionable: true },
  }, "action transaction receipt fixture");
});

test("mixed-object text selection does not depend on the engine's next object id", async () => {
  const scenario = await readValidatedDocument(frontendSelectionScenarioPath);
  const commit = scenario.actions.find((action) => action.id === "commit-text");
  const selectText = scenario.actions.find((action) => action.id === "select-text-only");
  const openMenu = scenario.actions.find((action) => action.id === "open-text-font-menu");
  const geometry = scenario.oracles.filter((oracle) => oracle.id.startsWith("text-selection-box-"));
  assert.equal(commit.completion.selector, '[data-role="document-text"]');
  assert.deepEqual(selectText.target, { strategy: "selector", value: '[data-role="document-text"]' });
  assert.equal(selectText.completion.operator, "eq");
  assert.equal(selectText.completion.value, 1);
  assert(scenario.actions.indexOf(selectText) < scenario.actions.indexOf(openMenu));
  assert.deepEqual(openMenu.target, { strategy: "selector", value: '[data-role="document-text"]' });
  assert(geometry.every((oracle) => oracle.referenceSelector === '[data-role="document-text"]'));
  assert.doesNotMatch(JSON.stringify({ commit, selectText, openMenu, geometry }), /obj_text_\d+/);
});

test("text selection geometry requires both ink containment and a tight metrics envelope", async () => {
  const scenario = await readValidatedDocument(frontendSelectionScenarioPath);
  const encloses = scenario.oracles.find((oracle) => oracle.id === "text-selection-box-encloses-current-font-geometry");
  const tight = scenario.oracles.find((oracle) => oracle.id === "text-selection-box-remains-tight-to-current-font-geometry");
  assert.deepEqual(encloses.uiExpected.geometry, { relation: "contains-reference", tolerancePx: 1 });
  assert.deepEqual(tight.uiExpected.geometry, { relation: "matches-reference", tolerancePx: 5.5 });
  assert.equal(encloses.selector, tight.selector);
  assert.equal(encloses.referenceSelector, tight.referenceSelector);
});

test("bond selection geometry targets the canonical engine selection role", async () => {
  const scenario = await readValidatedDocument(frontendSelectionScenarioPath);
  const bond = scenario.oracles.find((oracle) => oracle.id === "selection-overlay-encloses-rendered-bond");
  assert.equal(bond.selector, '[data-layer="editor-overlay"] [data-role="selection-bond"]');
  assert.doesNotMatch(bond.selector, /selection-(bond|node)-box/);
  assert.deepEqual(bond.uiExpected.rect, { minWidth: 12, minHeight: 12 });
});

test("selector targets share the bounded DOM selector limit", async () => {
  const scenario = JSON.parse(await readFile(frontendSelectionScenarioPath, "utf8"));
  const openMenu = scenario.actions.find((action) => action.id === "open-text-font-menu");
  openMenu.target.value = "x".repeat(2048);
  assert.equal((await validateDocument(scenario)).valid, true);
  openMenu.target.value += "x";
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("2048")));
});

test("scenario protocol rejects missing auditable coverage", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  delete scenario.coverage;
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("coverage")));
});

test("DOM completion selectors share one bounded protocol limit", async () => {
  const boundedSelector = `.menu-item${", .menu-item".repeat(80)}`;
  assert(boundedSelector.length > 512 && boundedSelector.length <= 2048);
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction.v1",
    actionId: "bounded-selector",
    input: { kind: "click", x: 10, y: 20, button: "left" },
    completion: { kind: "dom-count", selector: boundedSelector, operator: "eq", value: 7, timeoutMs: 8000 },
    budgetMs: 30000,
  }, "bounded DOM selector action transaction");

  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0].completion = { kind: "dom-count", selector: "x".repeat(2049), operator: "eq", value: 1, timeoutMs: 8000 };
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("2048")));
});

test("DOM text completion accepts empty exact text and rejects oversized text", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0].completion = { kind: "dom-text", selector: "[data-role=\"text-editor-display\"]", text: "", timeoutMs: 8000 };
  assert.equal((await validateDocument(scenario)).valid, true);

  scenario.actions[0].completion.text = "x".repeat(4097);
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("4096")));
});

test("native document saves reserve enough time for dismissal, attestation, and transfer", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0].completion = { kind: "document-saved", timeoutMs: 30000 };
  scenario.actions[0].budgetMs = 89999;
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("90000")));
});

test("scenario actions reserve an independent transport envelope around product completion", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0].budgetMs = 29999;
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("30000")));
});

test("scenario pointer modifiers are bounded to pointer actions", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0] = { ...scenario.actions[0], type: "key", key: "Escape", modifiers: ["Shift"] };
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
});

test("scenario relative click positions are bounded and click-only", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0].at = { x: 0.25, y: 0.75 };
  await assertValidDocument(scenario, "relative click scenario");

  scenario.actions[0].at.x = 1.01;
  let result = await validateDocument(scenario);
  assert.equal(result.valid, false);

  scenario.actions[0] = { ...scenario.actions[0], type: "key", key: "Escape", at: { x: 0.5, y: 0.5 } };
  delete scenario.actions[0].button;
  result = await validateDocument(scenario);
  assert.equal(result.valid, false);
});

test("scenario text input accepts exactly one bounded literal or declared source", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0] = {
    ...scenario.actions[0],
    type: "text",
    text: "ChemSema text",
  };
  delete scenario.actions[0].button;
  await assertValidDocument(scenario, "literal text scenario");

  scenario.actions[0].textSource = "document-output-path";
  let result = await validateDocument(scenario);
  assert.equal(result.valid, false);

  delete scenario.actions[0].text;
  await assertValidDocument(scenario, "sourced text scenario");

  scenario.actions[0].text = "x".repeat(4097);
  delete scenario.actions[0].textSource;
  result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("4096")));
});

test("canonical JSON and evidence keys ignore object key order", () => {
  assert.equal(canonicalJson({ b: 2, a: { d: 4, c: 3 } }), canonicalJson({ a: { c: 3, d: 4 }, b: 2 }));
  const left = evidenceKey({ scenario: { b: 2, a: 1 }, driver: "fake", environment: { z: 2, a: 1 } });
  const right = evidenceKey({ environment: { a: 1, z: 2 }, driver: "fake", scenario: { a: 1, b: 2 } });
  assert.equal(left, right);
  assert.match(left, /^[0-9a-f]{64}$/);
});

test("run report schema rejects an invalid evidence key", async () => {
  await assert.rejects(
    assertValidDocument({ schema: "chemsema.gui.run.v1", evidenceKey: "bad" }, "invalid run"),
    /schema validation/,
  );
});
