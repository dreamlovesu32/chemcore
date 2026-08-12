import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { createWorkerCoordinator } from "../src/workers/create.mjs";
import { PhysicalWindowsCoordinator } from "../src/workers/physical-windows.mjs";

const profilePath = join(guiTestsDir, "environments", "windows-physical-gui-worker-current.json");

function result(value) {
  return { status: 0, stdout: `${JSON.stringify(value)}\n`, stderr: "" };
}

test("physical worker profile binds the current account and uses adaptive safety reserves", async () => {
  const profile = await readValidatedDocument(profilePath);
  assert.equal(profile.kind, "physical-windows");
  assert.equal(profile.resources.mode, "adaptive");
  assert.equal("cpuUnits" in profile.resources, false);
  assert.equal("memoryGiB" in profile.resources, false);
  const coordinator = createWorkerCoordinator(profile, {
    environment: { USERDOMAIN: "HOST", USERNAME: "tester", LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({}),
  });
  assert(coordinator instanceof PhysicalWindowsCoordinator);
  assert.equal(coordinator.account, "HOST\\tester");
});

test("physical host attestation fails closed below the adaptive memory reserve", async () => {
  const profile = await readValidatedDocument(profilePath);
  const coordinator = new PhysicalWindowsCoordinator(profile, {
    environment: { USERDOMAIN: "HOST", USERNAME: "tester", LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({
      host: { platform: "windows-physical", account: "HOST\\tester", interactiveSession: true, explorerInSession: true },
      resources: { availableMemoryGiB: 3, commitPercent: 50 },
    }),
  });
  await assert.rejects(coordinator.attestHost(), /available memory/);
});

test("physical coordinator never configures autologon and only stops test-owned processes", async () => {
  const profile = await readValidatedDocument(profilePath);
  const coordinator = new PhysicalWindowsCoordinator(profile, {
    environment: { USERDOMAIN: "HOST", USERNAME: "tester", LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({}),
  });
  await assert.rejects(coordinator.configureAutologon(), /never configure automatic logon/);
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "physical-windows-coordinator.ps1"), "utf8");
  assert.match(source, /scope='test-owned-processes-only'/);
  assert.match(source, /Refusing to stop PID/);
  assert.doesNotMatch(source, /Stop-Computer|Restart-Computer|Stop-VM|Set-ItemProperty.+Winlogon/i);
});

test("production driver has an explicit non-atomic physical action path", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "src", "drivers", "production-black-box.mjs"), "utf8");
  assert.match(source, /atomicActionTransactions === false/);
  assert.match(source, /capture-before-state/);
  assert.match(source, /capture-after-state/);
});

test("physical daemon is single-stream, checkpointed, resource-aware, and never retries product failures", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "src", "workers", "physical-daemon.mjs"), "utf8");
  assert.match(source, /open\(context\.lockPath, "wx"\)/);
  assert.match(source, /physical-daemon-checkpoint\.v1/);
  assert.match(source, /minimumAvailableMemoryGiB/);
  assert.match(source, /refuses to start from a dirty repository/);
  assert.match(source, /verifyDesktopCandidateManifest/);
  assert.match(source, /Scenario .* failed; queue paused without retry/);
  assert.match(source, /stopped-by-request/);
  assert.doesNotMatch(source, /Promise\.all\([^)]*production-black-box/);
});
