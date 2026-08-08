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

test("scenario, coverage registry, and impact graph validate", async () => {
  await readValidatedDocument(scenarioPath);
  await readValidatedDocument(multiObjectScenarioPath);
  await readValidatedDocument(mixedObjectScenarioPath);
  await readValidatedDocument(nestedGroupScenarioPath);
  await readValidatedDocument(regionAdditiveScenarioPath);
  await readValidatedDocument(crossDocumentClipboardScenarioPath);
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
    input: { kind: "click", x: 10, y: 20, button: "left", modifiers: ["Shift"] },
    completion: { kind: "actionable", timeoutMs: 8000 },
    budgetMs: 12000,
  }, "action transaction fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction.v1",
    input: { kind: "key", key: "Control+A" },
    completion: {
      kind: "dom-distinct-count",
      selector: "[data-role=\"document-graphic\"][data-object-id]",
      attribute: "data-object-id",
      operator: "eq",
      value: 2,
      timeoutMs: 8000,
    },
    budgetMs: 12000,
  }, "distinct object action transaction fixture");
  await assertValidDocument({
    schema: "chemsema.gui.action-transaction-receipt.v1",
    input: {}, before: {}, after: {}, completion: { actionable: true },
  }, "action transaction receipt fixture");
});

test("scenario protocol rejects missing auditable coverage", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  delete scenario.coverage;
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
  assert(result.errors.some((error) => error.includes("coverage")));
});

test("scenario pointer modifiers are bounded to pointer actions", async () => {
  const scenario = JSON.parse(await readFile(scenarioPath, "utf8"));
  scenario.actions[0] = { ...scenario.actions[0], type: "key", key: "Escape", modifiers: ["Shift"] };
  const result = await validateDocument(scenario);
  assert.equal(result.valid, false);
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
