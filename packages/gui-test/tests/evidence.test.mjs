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
    const report = await runScenario({ scenario, driver: new FakeDriver() });
    const first = await writeEvidenceBundle({ report, root });
    const second = await writeEvidenceBundle({ report, root });
    assert.equal(first.objectPath, second.objectPath);
    assert.equal(first.manifest.artifacts[0].sha256, first.objectPath.match(/[0-9a-f]{64}/)?.[0]);
    assert.deepEqual(JSON.parse(await readFile(first.objectPath, "utf8")), report);
    assert.equal(first.manifest.artifacts[0].retention, "sample");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
