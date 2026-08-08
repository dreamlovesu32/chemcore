import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { guiTestsDir } from "../src/protocol/paths.mjs";
import { readValidatedDocument } from "../src/protocol/validate.mjs";
import { expandWindowsEnvironment, HyperVCoordinator } from "../src/workers/hyperv.mjs";

const profilePath = join(guiTestsDir, "environments", "windows-gui-worker-current.json");

function result(value) {
  return { status: 0, stdout: `${JSON.stringify(value)}\n`, stderr: "" };
}

test("worker profile contains no secret and expands its external credential path", async () => {
  const profile = await readValidatedDocument(profilePath);
  assert.equal(JSON.stringify(profile).toLowerCase().includes("password"), false);
  assert.equal(
    expandWindowsEnvironment(profile.credential.pathTemplate, { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" }),
    "C:\\Users\\tester\\AppData\\Local\\ChemSema\\gui-test\\credentials\\windows-gui-worker-current.credential.xml",
  );
});

test("host attestation verifies identity, services, VM bounds, and encrypted credential", async () => {
  const profile = await readValidatedDocument(profilePath);
  let invokedArgs = null;
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor(args) {
      invokedArgs = args;
      return result({
        schema: "chemsema.gui.worker-attestation.v1",
        operation: "host-attest",
        host: { hyperVAdministrator: true, vmms: "Running", vmcompute: "Running" },
        vm: { id: profile.vm.id, generation: 2, cpuUnits: 8, memoryMaximumBytes: 20 * 1024 ** 3 },
        credential: { exists: true },
      });
    },
  });
  const attestation = await coordinator.attestHost();
  assert.equal(attestation.operation, "host-attest");
  assert(invokedArgs.includes("host-attest"));
  assert.equal(invokedArgs.some((argument) => /password/i.test(argument)), false);
});

test("host and interactive guest attestation fail closed", async () => {
  const profile = await readValidatedDocument(profilePath);
  const hostCoordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({
      host: { hyperVAdministrator: false, vmms: "Running", vmcompute: "Running" },
      vm: { id: profile.vm.id, generation: 2, cpuUnits: 8, memoryMaximumBytes: 20 * 1024 ** 3 },
      credential: { exists: true },
    }),
  });
  await assert.rejects(hostCoordinator.attestHost(), /failed closed/);

  const guestCoordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({
      operation: "guest-attest",
      guest: { identity: "WINDOWS11\\chemsema-test", vmicvmsession: "Running", interactiveAccountMatches: false },
    }),
  });
  await assert.rejects(guestCoordinator.attestGuest({ requireInteractive: true }), /interactive guest session/);
});

test("coordinator shutdown is graceful and never forces power off", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  assert.match(source, /Stop-VM -VM \$vm\s*$/m);
  assert.doesNotMatch(source, /Stop-VM[^\r\n]+-(?:Force|TurnOff|Save|Shutdown)\b/i);
});
