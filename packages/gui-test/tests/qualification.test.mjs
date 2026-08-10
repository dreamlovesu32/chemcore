import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { evaluateQualification } from "../src/qualification/evaluate.mjs";
import { verifyQualificationEvidence } from "../src/qualification/verify-evidence.mjs";
import { assertValidDocument } from "../src/protocol/validate.mjs";

const candidate = "a".repeat(64);
const evidenceKey = "b".repeat(64);

function report({ runId, scenarioId, status = "passed", driver = "production-black-box", candidateSha256 = candidate }) {
  return { runId, scenarioId, status, driver, evidenceKey, environment: { candidateSha256 } };
}

function evidenceAudit() {
  return { diagnostics: [], summary: { manifestCount: 2, artifactCount: 12, totalBytes: 1024, verifiedHashes: 12 } };
}

test("qualification keeps an earlier failed run blocking after a later pass", () => {
  const result = evaluateQualification({
    expectedScenarioIds: ["scenario.a"],
    reports: [
      report({ runId: "00000000-0000-4000-8000-000000000001", scenarioId: "scenario.a", status: "failed" }),
      report({ runId: "00000000-0000-4000-8000-000000000002", scenarioId: "scenario.a", status: "passed" }),
    ],
    evidenceAudit: evidenceAudit(),
  });
  assert.equal(result.status, "failed");
  assert.equal(result.failedRunCount, 1);
  assert.equal(result.failedScenarioCount, 1);
  assert.deepEqual(result.scenarioResults[0].failedRuns, ["00000000-0000-4000-8000-000000000001"]);
  assert.deepEqual(result.scenarioResults[0].passedRuns, ["00000000-0000-4000-8000-000000000002"]);
});

test("qualification passes only a complete single-candidate clean closure", async () => {
  const result = evaluateQualification({
    expectedScenarioIds: ["scenario.a", "scenario.browser"],
    reports: [
      report({ runId: "00000000-0000-4000-8000-000000000003", scenarioId: "scenario.a" }),
      report({ runId: "00000000-0000-4000-8000-000000000004", scenarioId: "scenario.browser", driver: "playwright-browser", candidateSha256: null }),
    ],
    evidenceAudit: evidenceAudit(),
  });
  assert.equal(result.status, "passed");
  assert.equal(result.passedScenarioCount, 2);
  assert.equal(result.candidateSha256, candidate);
  assert.deepEqual(result.diagnostics, []);
  await assertValidDocument({
    schema: "chemsema.gui.qualification.v1",
    qualificationId: "00000000-0000-4000-8000-000000000008",
    generatedAt: "2026-08-11T00:00:00.000Z",
    ...result,
  }, "qualification fixture");
});

test("qualification fails closed on missing scenarios, candidate mixing, and evidence diagnostics", () => {
  const result = evaluateQualification({
    expectedScenarioIds: ["scenario.a", "scenario.missing"],
    reports: [
      report({ runId: "00000000-0000-4000-8000-000000000005", scenarioId: "scenario.a" }),
      report({ runId: "00000000-0000-4000-8000-000000000006", scenarioId: "scenario.a", candidateSha256: "c".repeat(64) }),
    ],
    evidenceAudit: { diagnostics: ["evidence:tampered"], summary: { manifestCount: 1, artifactCount: 1, totalBytes: 1, verifiedHashes: 0 } },
  });
  assert.equal(result.status, "failed");
  assert.equal(result.missingScenarioCount, 1);
  assert(result.diagnostics.includes("production-candidate-count:2"));
  assert(result.diagnostics.includes("evidence:tampered"));
});

test("evidence qualification rejects a manifest URI that escapes its root", async () => {
  const root = await mkdtemp(join(tmpdir(), "chemsema-qualification-"));
  try {
    const runId = "00000000-0000-4000-8000-000000000007";
    const recordRoot = join(root, "records", evidenceKey, runId);
    await mkdir(recordRoot, { recursive: true });
    await writeFile(join(recordRoot, "artifact-manifest.json"), JSON.stringify({
      schema: "chemsema.gui.artifact-manifest.v1",
      runId,
      artifacts: [{
        name: "run-report.json", mediaType: "application/json", uri: "../outside.json", size: 2,
        sha256: "d".repeat(64), retention: "failure",
      }],
    }), "utf8");
    const audit = await verifyQualificationEvidence({ reports: [{ runId, evidenceKey }], evidenceRoot: root });
    assert.equal(audit.summary.manifestCount, 0);
    assert.equal(audit.diagnostics.length, 1);
    assert.match(audit.diagnostics[0], /escapes evidence root/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
