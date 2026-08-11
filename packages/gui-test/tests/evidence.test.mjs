import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { writeEvidenceBundle } from "../src/evidence/write-bundle.mjs";
import { FakeDriver } from "../src/drivers/fake.mjs";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { runScenario } from "../src/runner/run-scenario.mjs";

test("evidence bundle stores the validated report as an immutable SHA-256 object", async () => {
  const root = await mkdtemp(join(tmpdir(), "chemsema-gui-evidence-"));
  try {
    const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
    const { report, artifactPayloads } = await runScenario({ scenario, driver: new FakeDriver() });
    const first = await writeEvidenceBundle({ report, artifactPayloads, root });
    const second = await writeEvidenceBundle({ report, artifactPayloads, root });
    assert.equal(first.objectPath, second.objectPath);
    assert.equal(first.manifest.artifacts[0].sha256, first.objectPath.match(/[0-9a-f]{64}/)?.[0]);
    assert.deepEqual(JSON.parse(await readFile(first.objectPath, "utf8")), report);
    assert.equal(first.manifest.artifacts[0].retention, "sample");
    assert.equal(first.manifest.artifacts[0].name, "run-report.json");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("evidence bundle stores and verifies driver artifacts by content hash", async () => {
  const root = await mkdtemp(join(tmpdir(), "chemsema-gui-evidence-payload-"));
  try {
    const scenario = await readValidatedDocument(join(guiTestsDir, "scenarios", "core", "draw-single-bond.json"));
    const driver = new FakeDriver();
    driver.collectArtifacts = async () => [{ name: "final-state.json", mediaType: "application/json", bytes: Buffer.from("{}\n") }];
    const { report, artifactPayloads } = await runScenario({ scenario, driver });
    const bundle = await writeEvidenceBundle({ report, artifactPayloads, root });
    assert.equal(bundle.manifest.artifacts.length, 2);
    assert.equal(bundle.manifest.artifacts[1].name, "final-state.json");
    assert.equal(await readFile(bundle.artifactObjectPaths[0], "utf8"), "{}\n");
    const tampered = [{ ...artifactPayloads[0], bytes: Buffer.from("tampered") }];
    await assert.rejects(writeEvidenceBundle({ report, artifactPayloads: tampered, root }), /does not match/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
