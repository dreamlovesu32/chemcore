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

test("production canvas and secondary controls expose stable accessibility locators", async () => {
  const html = await readFile(join(repositoryRoot, "viewer", "index.html"), "utf8");
  assert.match(html, /id="viewer-container"[^>]+role="application"[^>]+aria-label="Drawing canvas"/);
  assert.match(html, /id="secondary-toolbar"[^>]+role="toolbar"[^>]+aria-label="Secondary toolbar"/);
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
        vm: { id: profile.vm.id, generation: 2, cpuUnits: 8, memoryMaximumBytes: 20 * 1024 ** 3, automaticCheckpoints: false, checkpointId: profile.vm.checkpoint.id, checkpointName: profile.vm.checkpoint.name },
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
      vm: { id: profile.vm.id, generation: 2, cpuUnits: 8, memoryMaximumBytes: 20 * 1024 ** 3, automaticCheckpoints: false, checkpointId: profile.vm.checkpoint.id, checkpointName: profile.vm.checkpoint.name },
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
  assert.equal((await coordinator.candidateInput("click", { x: 34, y: 212 }, { modifiers: ["Shift"] })).agent.foreground.processId, 300);
  assert.deepEqual(invokedArgs.slice(invokedArgs.indexOf("-InputX"), invokedArgs.indexOf("-InputX") + 4), ["-InputX", "34", "-InputY", "212"]);
  assert.deepEqual(invokedArgs.slice(invokedArgs.indexOf("-InputModifiers"), invokedArgs.indexOf("-InputModifiers") + 2), ["-InputModifiers", "Shift"]);
  await assert.rejects(coordinator.candidateInput("click", { x: Number.NaN, y: 1 }), /must be integers/);
  await assert.rejects(coordinator.candidateInput("click", { x: 1, y: 1 }, { modifiers: ["Windows"] }), /allowlisted/);
});

test("candidate action sends one validated versioned transaction", async () => {
  const profile = await readValidatedDocument(profilePath);
  let invokedArgs;
  const guestPath = "C:\\ChemSemaGuiTest\\candidate\\abc\\chemsema-desktop.exe";
  const agent = {
    schema: "chemsema.gui.guest-agent.v1", agentVersion: "0.1.0", processId: 100, sessionId: 1,
    account: "guest\\chemsema-test", inputDesktop: "Default", interactiveReady: true,
    foreground: { windowHandle: 200, processId: 300, sessionId: 1, executable: guestPath, title: "ChemSema", className: "Tauri Window", rect: [0, 0, 1000, 800], clientRect: [8, 1, 992, 792] },
  };
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor(args) {
      invokedArgs = args;
      return result({
        operation: "action-transaction", candidate: { guestPath, sha256: "abc" },
        transaction: { schema: "chemsema.gui.action-transaction-receipt.v1", input: agent, before: {}, after: {}, completion: { actionable: true } },
      });
    },
  });
  const receipt = await coordinator.candidateAction(
    { kind: "click", x: 34, y: 212, button: "left", modifiers: ["Shift"] },
    { kind: "actionable", timeoutMs: 8000 },
    30000,
    "activate-bond-tool",
  );
  assert.equal(receipt.transaction.input.foreground.processId, 300);
  const encoded = invokedArgs[invokedArgs.indexOf("-ActionRequestBase64") + 1];
  const request = JSON.parse(Buffer.from(encoded, "base64").toString("utf8"));
  assert.equal(request.schema, "chemsema.gui.action-transaction.v1");
  assert.equal(request.actionId, "activate-bond-tool");
  assert.deepEqual(request.input, { kind: "click", x: 34, y: 212, button: "left", modifiers: ["Shift"] });
  await assert.rejects(
    coordinator.candidateAction(
      { kind: "click", x: 34, y: 212, button: "left" },
      { kind: "actionable", timeoutMs: 8001 },
      12000,
      "activate-bond-tool",
    ),
    /at least 30000 ms/,
  );
  await assert.rejects(
    coordinator.candidateAction(
      { kind: "click", x: 34, y: 212, button: "left" },
      { kind: "actionable", timeoutMs: 26001 },
      30000,
      "activate-bond-tool",
    ),
    /leave 15000 ms/,
  );
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
  const querySource = coordinatorSource.slice(queryStart, queryHereStringEnd);
  assert.match(querySource, /foreach\(\$root in \$roots\)/);
  assert.match(querySource, /\[double\]::IsInfinity\(\$_\)/);
  assert.match(querySource, /hasKeyboardFocus=\$element\.Current\.HasKeyboardFocus/);
  assert.match(querySource, /topLevelClassName=\$root\.Current\.ClassName/);
  assert.match(querySource, /topLevels=\$topLevels/);
  assert.match(querySource, /AutomationIdProperty/);
  assert.match(querySource, /ExpectedControlType/);
});

test("coordinator emits structured guest and UIA results as UTF-8", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const cdpSource = await readFile(join(packageRoot, "scripts", "guest-cdp.ps1"), "utf8");
  assert.match(source, /\[Console\]::OutputEncoding = \$script:Utf8WithoutBom/);
  assert.match(source, /Get-Content -Raw -Encoding UTF8 -LiteralPath \$resultPath \| ConvertFrom-Json/);
  assert.doesNotMatch(`${source}\n${cdpSource}`, /Get-Content -Raw(?! -Encoding UTF8)/);
  assert.match(source, /\$OutputEncoding = \$script:Utf8WithoutBom/);
});

test("candidate input uses the persistent bounded channel rather than per-action scheduled tasks", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const start = source.indexOf("function Invoke-CandidateInput");
  const end = source.indexOf("function Get-ServiceAgentAttestation", start);
  const input = source.slice(start, end);
  assert.match(input, /chemsema\.gui\.guest-agent-request\.v1/);
  assert.match(input, /Persistent input response identity is invalid/);
  assert.doesNotMatch(input, /ScheduledTask|Start-ScheduledTask/);
  assert.match(input, /'text'/);
  assert.match(input, /--text-base64/);
  assert.match(input, /--modifiers/);
  assert.match(input, /Pointer modifiers are not allowlisted/);
});

test("CDP observation uses a persistent bounded channel rather than per-request process launch", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const start = source.indexOf("function Invoke-CdpBridge");
  const end = source.indexOf("function Start-PersistentCdpAgent", start);
  const bridge = source.slice(start, end);
  assert.match(bridge, /chemsema\.gui\.cdp-request\.v1/);
  assert.match(bridge, /chemsema\.gui\.cdp-response\.v1/);
  assert.match(bridge, /Persistent CDP response identity is invalid/);
  assert.doesNotMatch(bridge, /powershell\.exe|ScheduledTask|ScriptSource/);
  assert.match(bridge, /if \(\$decodedRequest\.mode -eq 'artifact-export'\) \{ 90 \} else \{ 20 \}/);
  assert.match(bridge, /AddSeconds\(\$ReceiptTimeoutSeconds\)/);
  const agentStart = source.slice(end, source.indexOf("function Stop-PersistentCdpAgent", end));
  assert.match(agentStart, /-UserId 'SYSTEM' -LogonType ServiceAccount/);
  assert.doesNotMatch(agentStart, /LogonType Interactive/);
  const guestSource = await readFile(join(packageRoot, "scripts", "guest-cdp.ps1"), "utf8");
  assert.match(guestSource, /'distinct-count', 'distinct-count-state', 'text', 'text-state'/);
  assert.match(guestSource, /DOM observation requires a selector of 1 to 2048 characters/);
  assert.match(guestSource, /elements\.length === 1 \? elements\[0\]\.textContent : null/);
  const countBranch = guestSource.slice(
    guestSource.indexOf("} elseif ($request.mode -in @('count', 'count-state'"),
    guestSource.indexOf("} elseif ($request.mode -in @('text', 'text-state'"),
  );
  assert.match(countBranch, /\$countExpression = if \(\$distinct\)/);
  assert.match(countBranch, /\$expression = @"/);
  const textBranch = guestSource.slice(
    guestSource.indexOf("} elseif ($request.mode -in @('text', 'text-state'"),
    guestSource.indexOf("} else {", guestSource.indexOf("} elseif ($request.mode -in @('text', 'text-state'")),
  );
  assert.doesNotMatch(textBranch, /\$countExpression/);
  assert.match(guestSource, /'entity-rects-state'/);
  assert.match(guestSource, /Entity rectangle observation requires 1 to 16 unique ids/);
  assert.match(guestSource, /getBBox/);
  assert.match(guestSource, /rootMatrix\.inverse\(\)\.multiply\(elementMatrix\)/);
  assert.match(guestSource, /worldRect/);
  assert.match(guestSource, /'data-object-id', 'data-node-id', 'data-bond-id'/);
  assert.match(guestSource, /'artifact-export'/);
  assert.match(guestSource, /'trace-start'/);
  assert.match(guestSource, /'trace-mark'/);
  assert.match(guestSource, /performance\.mark/);
  assert.match(guestSource, /Tracing\.start/);
  assert.match(guestSource, /transferMode = 'ReturnAsStream'/);
  assert.match(guestSource, /Tracing\.end/);
  assert.match(guestSource, /Tracing\.tracingComplete/);
  assert.match(guestSource, /IO\.read/);
  assert.match(guestSource, /if \(-not \[string\]::IsNullOrEmpty\(\$chunkData\)\)/);
  assert.match(guestSource, /return ,\$output\.ToArray\(\)/);
  assert.match(guestSource, /performance-trace\.json\.gz/);
  assert.doesNotMatch(guestSource, /__chemsemaDebug/);
  assert.doesNotMatch(guestSource, /document\.ccjs\.json/);
  assert.match(guestSource, /Page\.captureScreenshot/);
  assert.match(guestSource, /64 \* 1024 \* 1024/);
  assert.match(guestSource, /chemsema\.gui\.guest-artifact-export\.v1/);
  assert.match(guestSource, /webview\.log/);
});

test("production artifacts use SHA-verified PowerShell Direct file transfer", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const start = source.indexOf("function Receive-GuestArtifacts");
  const end = source.indexOf("function Start-PersistentCdpAgent", start);
  const transfer = source.slice(start, end);
  assert.match(transfer, /Copy-Item[^\n]+-FromSession \$session/);
  assert.match(transfer, /Guest artifact .* changed before transfer/);
  assert.match(transfer, /failed SHA-256 verification after transfer/);
  assert.match(transfer, /64 \* 1024 \* 1024/);
  assert.doesNotMatch(transfer, /FromBase64String\(\$artifact\./);
});

test("document output is confined to an immutable guest root and verified after transfer", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const start = source.indexOf("function Assert-DocumentOutputIdentity");
  const end = source.indexOf("function Start-PersistentCdpAgent", start);
  const documentTransfer = source.slice(start, end);
  assert.match(documentTransfer, /\^\[a-f0-9\]\{32\}\$/);
  assert.match(documentTransfer, /Join-Path \$GuestTestRoot 'documents'/);
  assert.match(documentTransfer, /escaped the dedicated guest test root/);
  assert.match(documentTransfer, /64 \* 1024 \* 1024/);
  assert.match(documentTransfer, /Copy-Item[^\r\n]+-FromSession \$session/);
  assert.equal((documentTransfer.match(/Get-FileHash/g) || []).length, 2);
});

test("document output coordinator rejects unsafe names before guest execution", async () => {
  const profile = await readValidatedDocument(profilePath);
  let invoked = false;
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor: () => { invoked = true; return result({}); },
  });
  await assert.rejects(coordinator.prepareDocumentOutput("..\\escape.ccjs"), /safe CCJS filename/);
  await assert.rejects(coordinator.prepareDocumentOutput("roundtrip.cdx"), /safe CCJS filename/);
  assert.equal(invoked, false);
});

test("candidate launch enables a bounded WebView log inside the content-addressed directory", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const start = source.indexOf("function Start-Candidate");
  const end = source.indexOf("function Activate-Candidate", start);
  const launch = source.slice(start, end);
  assert.match(launch, /Join-Path \(Split-Path -Parent \$CandidatePath\) 'webview\.log'/);
  assert.match(launch, /--enable-logging/);
  assert.match(launch, /--log-file=\$logPath/);
});

test("CDP distinct object observation requires an allowlisted identity attribute", async () => {
  const profile = await readValidatedDocument(profilePath);
  let encodedRequest = null;
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor(args) {
      const requestIndex = args.indexOf("-CdpRequestBase64");
      encodedRequest = JSON.parse(Buffer.from(args[requestIndex + 1], "base64").toString("utf8"));
      return result({ bridge: { schema: "chemsema.gui.cdp-bridge.v1", status: "passed", value: 2 } });
    },
  });
  assert.equal(await coordinator.cdpBridge({ mode: "distinct-count", selector: "[data-object-id]", attribute: "data-object-id" }), 2);
  assert.deepEqual(encodedRequest, { mode: "distinct-count", selector: "[data-object-id]", attribute: "data-object-id" });
  await assert.rejects(
    coordinator.cdpBridge({ mode: "distinct-count", selector: "[data-object-id]", attribute: "class" }),
    /allowlisted identity attribute/,
  );
});

test("CDP text observation preserves an exact bounded selector request", async () => {
  const profile = await readValidatedDocument(profilePath);
  let encodedRequest = null;
  const exact = { count: 1, text: "A  B\nC" };
  const coordinator = new HyperVCoordinator(profile, {
    environment: { LOCALAPPDATA: "C:\\Users\\tester\\AppData\\Local" },
    executor(args) {
      const requestIndex = args.indexOf("-CdpRequestBase64");
      encodedRequest = JSON.parse(Buffer.from(args[requestIndex + 1], "base64").toString("utf8"));
      return result({ bridge: { schema: "chemsema.gui.cdp-bridge.v1", status: "passed", value: exact } });
    },
  });
  assert.deepEqual(await coordinator.cdpBridge({ mode: "text-state", selector: ".text-editor-display" }), exact);
  assert.deepEqual(encodedRequest, { mode: "text-state", selector: ".text-editor-display" });
  await assert.rejects(coordinator.cdpBridge({ mode: "text", selector: "x".repeat(2049) }), /selector of 1 to 2048 characters/);
});

test("production world geometry targets are restricted to the rendered page", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "guest-cdp.ps1"), "utf8");
  assert.match(source, /query\.strategy === 'world-geometry'/);
  assert.match(source, /query\.value !== 'page-background'/);
  assert.match(source, /\[data-layer="page-background"\]/);
});

test("production semantic targets expose native text and select controls", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "guest-cdp.ps1"), "utf8");
  assert.match(source, /TEXTAREA:'textbox'/);
  assert.match(source, /SELECT:'combobox'/);
  assert.match(source, /element\.tagName === 'INPUT'/);
  assert.match(source, /number:'spinbutton'/);
  assert.match(source, /checkbox:'checkbox'/);
  assert.match(source, /radio:'radio'/);
  assert.match(source, /element\.labels \|\| \[\]/);
  assert.match(source, /aria-labelledby/);
  assert.match(source, /label\.querySelector\(':scope > span'\)/);
  assert.match(source, /clone\.querySelectorAll\('input, select, textarea, button, option, em'\)/);
});

test("production entity targets use a real SVG geometry midpoint and retain bounded fallbacks", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "guest-cdp.ps1"), "utf8");
  assert.match(source, /query\.strategy === 'entity-id'/);
  assert.match(source, /\[data-object-id=/);
  assert.match(source, /hasAttribute\('data-renderer'\)/);
  assert.match(source, /matches\.find\(candidate/);
  assert.match(source, /target\.strategy === 'entity-id'/);
  assert.match(source, /rect\.width > 0 \|\| rect\.height > 0/);
  assert.match(source, /visibleElements/);
  assert.match(source, /visibleRenderRoots/);
  assert.match(source, /visibleRenderRoots\.length \|\| \(visibleElements\.length \? 1 : 0\)/);
  assert.match(source, /hasAttribute\('data-renderer'\) && visibleCandidate/);
  assert.match(source, /getAttribute\('data-object-type'\) === 'group'/);
  assert.match(source, /querySelectorAll\('\[data-role\^="document-"\], \[data-bond-id\], \[data-node-id\]'\)/);
  assert.match(source, /\[data-role="document-graphic"\], path, line, polyline, polygon, circle, ellipse, rect/);
  assert.match(source, /candidate\.getTotalLength\(\)/);
  assert.match(source, /candidate\.getPointAtLength\(length \* 0\.5\)/);
  assert.match(source, /candidate\.getScreenCTM\(\)/);
  assert.match(source, /new DOMPoint\(midpoint\.x, midpoint\.y\)\.matrixTransform\(matrix\)/);
  assert.match(source, /length > best\.length/);
  assert.match(source, /geometryPointerRect\(element\) \|\| semanticPointerElement\.getBoundingClientRect\(\)/);
  assert.match(source, /screenRects\.flatMap/);
  assert.match(source, /worldPoints\.push/);
});

test("production action transaction uses one guest invocation for before, input, completion, and after", async () => {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const source = await readFile(join(packageRoot, "scripts", "hyperv-coordinator.ps1"), "utf8");
  const broker = await readFile(join(packageRoot, "scripts", "hyperv-action-broker.ps1"), "utf8");
  const start = source.indexOf("function Invoke-ActionTransaction");
  const end = source.indexOf("function Get-ServiceAgentAttestation", start);
  const transaction = source.slice(start, end);
  assert.equal((transaction.match(/Invoke-Guest/g) || []).length, 1);
  assert.match(transaction, /mode='state'/);
  assert.match(transaction, /'count-state'/);
  assert.match(transaction, /Action transaction pointer modifiers are not unique allowlisted values/);
  assert.match(transaction, /\$null -ne \$_/);
  assert.match(transaction, /'distinct-count-state'/);
  assert.match(transaction, /'dom-text'/);
  assert.match(transaction, /mode='text-state'/);
  assert.match(transaction, /-ceq \$expectedText/);
  assert.match(transaction, /'entity-rect-deltas'/);
  assert.match(transaction, /maximumDeltaWorld/);
  assert.match(transaction, /'input-channel'/);
  assert.match(transaction, /'cdp-channel'/);
  assert.match(transaction, /\$channelReceiptTimeoutMs = 15000/);
  assert.match(transaction, /Send-ChannelRequest 'cdp-channel'.*\$channelReceiptTimeoutMs/);
  assert.match(transaction, /Mark-Trace 'start'/);
  assert.match(transaction, /Mark-Trace 'input-before'/);
  assert.match(transaction, /Mark-Trace 'input-after'/);
  assert.match(transaction, /Mark-Trace 'complete'/);
  assert.match(transaction, /chemsema\.gui\.action-transaction-receipt\.v1/);
  assert.match(source, /ChemSemaGuiPersistentSession/);
  assert.match(broker, /New-PSSession -VMId/);
  assert.match(broker, /Broker only accepts action-transaction operations/);
  assert.doesNotMatch(broker, /Invoke-Expression|\biex\b/i);
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
