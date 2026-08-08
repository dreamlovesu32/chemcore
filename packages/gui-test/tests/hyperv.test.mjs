import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { guiTestsDir, repositoryRoot } from "../src/protocol/paths.mjs";
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

test("production canvas exposes a stable accessibility locator", async () => {
  const html = await readFile(join(repositoryRoot, "viewer", "index.html"), "utf8");
  assert.match(html, /id="viewer-container"[^>]+role="application"[^>]+aria-label="Drawing canvas"/);
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

test("desktop baseline validates every policy used to suppress post-logon interruptions", async () => {
  const profile = await readValidatedDocument(profilePath);
  const responses = [
    result({ operation: "guest-attest", guest: { identity: "guest\\chemsema-test", vmicvmsession: "Running", interactiveAccountMatches: true } }),
    result({
      operation: "configure-desktop-baseline",
      baseline: {
        scope: "dedicated-test-user",
        changed: true,
        settings: {
          scoobeSystemSettingEnabled: 0,
          contentDeliveryAllowed: 0,
          oemPreInstalledAppsEnabled: 0,
          preInstalledAppsEnabled: 0,
          preInstalledAppsEverEnabled: 0,
          silentInstalledAppsEnabled: 0,
          systemPaneSuggestionsEnabled: 0,
          rotatingLockScreenEnabled: 0,
          rotatingLockScreenOverlayEnabled: 0,
          contentDeliverySoftLandingEnabled: 0,
          subscribedContent310093Enabled: 0,
          subscribedContent338389Enabled: 0,
        },
      },
    }),
  ];
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => responses.shift(),
  });
  assert.equal((await coordinator.configureDesktopBaseline()).baseline.changed, true);
});

test("desktop baseline treats an absent registry value as first-run configuration", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const baselineStart = source.indexOf("function Configure-DesktopBaseline");
  const baselineEnd = source.indexOf("function Get-InteractiveAgentAttestation", baselineStart);
  const baseline = source.slice(baselineStart, baselineEnd);
  assert.match(baseline, /RegistryKeyPermissionCheck\]::ReadWriteSubTree/);
  assert.match(baseline, /Desktop baseline registry path is outside the dedicated user allowlist/);
  assert.match(baseline, /\.GetValue\(\$Name, \$null, \[Microsoft\.Win32\.RegistryValueOptions\]::DoNotExpandEnvironmentNames\)/);
  assert.match(baseline, /\.SetValue\(\$Name, \$Value, \[Microsoft\.Win32\.RegistryValueKind\]::DWord\)/);
  assert.match(baseline, /Desktop baseline cannot set \$\{Path\}::\$\{Name\}/);
  assert.match(baseline, /Desktop baseline value \$Name was not persisted/);
  assert.doesNotMatch(baseline, /Get-ItemPropertyValue/);
  assert.doesNotMatch(baseline, /New-ItemProperty/);
});

test("candidate deployment is content-addressed and launch is interactive", async () => {
  const profile = await readValidatedDocument(profilePath);
  const responses = [
    result({ operation: "guest-attest", guest: { identity: "guest\\chemsema-test", vmicvmsession: "Running", interactiveAccountMatches: true } }),
    result({ operation: "launch-candidate", candidate: { guestPath: "C:\\ChemSemaGuiTest\\candidate\\abc\\chemsema-desktop.exe", sha256: "abc", processId: 42, sessionId: 1 } }),
  ];
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => responses.shift(),
  });
  const launch = await coordinator.launchCandidate();
  assert.equal(launch.candidate.processId, 42);
  assert.equal(launch.candidate.sessionId, 1);
});

test("candidate input passes integer coordinates and validates the returned foreground", async () => {
  const profile = await readValidatedDocument(profilePath);
  let invokedArgs;
  const guestPath = "C:\\ChemSemaGuiTest\\candidate\\abc\\chemsema-desktop.exe";
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor(args) {
      invokedArgs = args;
      return result({
        operation: "input-click",
        candidate: { guestPath, sha256: "abc" },
        agent: {
          schema: "chemsema.gui.guest-agent.v1",
          agentVersion: "0.1.0",
          processId: 100,
          sessionId: 1,
          account: "guest\\chemsema-test",
          inputDesktop: "Default",
          interactiveReady: true,
          foreground: { windowHandle: 200, processId: 300, sessionId: 1, executable: guestPath, title: "ChemSema", className: "Tauri Window", rect: [0, 0, 1000, 800], clientRect: [8, 1, 992, 792] },
        },
      });
    },
  });
  assert.equal((await coordinator.candidateInput("click", { x: 34, y: 212 })).agent.foreground.processId, 300);
  assert.deepEqual(invokedArgs.slice(invokedArgs.indexOf("-InputX"), invokedArgs.indexOf("-InputX") + 4), ["-InputX", "34", "-InputY", "212"]);
  await assert.rejects(coordinator.candidateInput("click", { x: Number.NaN, y: 1 }), /must be integers/);
});

test("interactive launcher is hidden, test-only CDP is loopback, and blocker removal is allowlisted", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const coordinatorSource = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const agentMain = await readFile(join(repositoryRoot, "crates", "chemsema-gui-test-agent", "src", "main.rs"), "utf8");
  const agentWindows = await readFile(join(repositoryRoot, "crates", "chemsema-gui-test-agent", "src", "windows.rs"), "utf8");
  assert.match(agentMain, /windows_subsystem = "windows"/);
  assert.match(coordinatorSource, /--remote-debugging-port=9223/);
  assert.doesNotMatch(coordinatorSource, /--remote-debugging-address/);
  assert.match(agentWindows, /Microsoft\.Windows\.CloudExperienceHost_cw5n1h2txyewy!App/);
  assert.match(agentWindows, /Windows\.UI\.Core\.CoreWindow/);
  const queryStart = coordinatorSource.indexOf("function Query-Uia");
  const queryHereStringEnd = coordinatorSource.indexOf("\n'@", queryStart);
  const cdpFunction = coordinatorSource.indexOf("function Invoke-CdpBridge");
  assert(queryStart >= 0 && queryHereStringEnd > queryStart && cdpFunction > queryHereStringEnd, "CDP bridge must not be embedded in the generated UIA script");
});

test("service-session agent attestation cannot claim interactive readiness", async () => {
  const profile = await readValidatedDocument(profilePath);
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({
      operation: "agent-attest-service",
      agent: {
        schema: "chemsema.gui.guest-agent.v1",
        agentVersion: "0.1.0",
        processId: 100,
        sessionId: 0,
        account: "guest\\chemsema-test",
        inputDesktop: null,
        interactiveReady: false,
        foreground: null,
      },
    }),
  });
  assert.equal((await coordinator.attestServiceAgent()).agent.sessionId, 0);
});

test("interactive agent attestation requires the dedicated unlocked session", async () => {
  const profile = await readValidatedDocument(profilePath);
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => result({
      operation: "agent-attest-interactive",
      agent: {
        schema: "chemsema.gui.guest-agent.v1",
        agentVersion: "0.1.0",
        processId: 100,
        sessionId: 2,
        account: "guest\\chemsema-test",
        inputDesktop: "Default",
        interactiveReady: true,
        foreground: {
          windowHandle: 200,
          processId: 300,
          sessionId: 2,
          executable: "C:\\Windows\\explorer.exe",
          title: "Desktop",
          className: "Shell_TrayWnd",
          rect: [0, 0, 1920, 1080],
          clientRect: [0, 0, 1920, 1080],
        },
      },
    }),
  });
  assert.equal((await coordinator.attestInteractiveAgent()).agent.interactiveReady, true);
});
