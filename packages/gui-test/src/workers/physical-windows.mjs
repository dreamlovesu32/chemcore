import { spawn } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { copyFile, mkdir, open, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, parse, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertValidDocument } from "../protocol/validate.mjs";
import { verifyDesktopCandidateManifest } from "../../../../scripts/candidate-source-identity.mjs";
import { expandWindowsEnvironment } from "./hyperv.mjs";
import { candidateActionBudgetIsValid } from "./action-budget.mjs";

const repositoryRoot = dirname(dirname(dirname(dirname(dirname(fileURLToPath(import.meta.url))))));
const defaultAgentPath = join(repositoryRoot, "target", "release", "chemsema-gui-test-agent.exe");
const defaultCandidatePath = join(repositoryRoot, "target", "release", "chemsema-desktop.exe");
const defaultCdpScriptPath = join(repositoryRoot, "packages", "gui-test", "scripts", "guest-cdp.ps1");
const defaultUiaScriptPath = join(repositoryRoot, "packages", "gui-test", "scripts", "physical-uia-query.ps1");
const backgroundLaunchScriptPath = join(repositoryRoot, "packages", "gui-test", "scripts", "physical-background-launch.ps1");

const delay = (milliseconds) => new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function fileIdentity(path) {
  const bytes = await readFile(path);
  return { bytes: bytes.length, sha256: sha256(bytes) };
}

async function runProcess(executable, args, { cwd, env, timeoutMs = 30000 } = {}) {
  return new Promise((resolveProcess) => {
    const child = spawn(executable, args, { cwd, env, windowsHide: true, shell: false });
    let stdout = "";
    let stderr = "";
    let failure = null;
    const append = (current, chunk) => `${current}${chunk}`.slice(-10 * 1024 * 1024);
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk.toString("utf8")); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk.toString("utf8")); });
    child.on("error", (error) => { failure = error; });
    const timer = setTimeout(() => {
      failure = new Error(`${executable} exceeded ${timeoutMs} ms.`);
      child.kill();
    }, timeoutMs);
    child.on("close", (status) => {
      clearTimeout(timer);
      resolveProcess({ status: status ?? 1, stdout, stderr, error: failure });
    });
  });
}

function parseJsonResult(result, operation) {
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Physical Windows ${operation} failed: ${(result.stderr || result.stdout || "unknown error").trim()}`);
  const line = String(result.stdout).split(/\r?\n/).map((value) => value.trim()).filter(Boolean).at(-1);
  if (!line) throw new Error(`Physical Windows ${operation} returned no structured result.`);
  try { return JSON.parse(line); } catch { throw new Error(`Physical Windows ${operation} returned malformed JSON.`); }
}

async function waitForJson(path, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  do {
    try { return JSON.parse(await readFile(path, "utf8")); } catch (error) {
      if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error;
    }
    await delay(20);
  } while (Date.now() < deadline);
  throw new Error(`${label} returned no receipt within ${timeoutMs} ms.`);
}

async function atomicJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${randomUUID().replaceAll("-", "")}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value)}\n`, "utf8");
  await rename(temporary, path);
}

function cleanAgentAttestation(agent = {}) {
  return {
    schema: agent.schema,
    agentVersion: agent.agentVersion,
    processId: agent.processId,
    sessionId: agent.sessionId,
    account: agent.account,
    inputDesktop: agent.inputDesktop ?? null,
    interactiveReady: agent.interactiveReady,
    foreground: agent.foreground ? {
      windowHandle: agent.foreground.windowHandle,
      processId: agent.foreground.processId,
      sessionId: agent.foreground.sessionId,
      executable: agent.foreground.executable,
      title: agent.foreground.title,
      className: agent.foreground.className,
      rect: agent.foreground.rect,
      clientRect: agent.foreground.clientRect,
    } : null,
  };
}

export class PhysicalWindowsCoordinator {
  constructor(profile, {
    environment = process.env,
    agentPath = defaultAgentPath,
    candidatePath = defaultCandidatePath,
    cdpScriptPath = defaultCdpScriptPath,
    uiaScriptPath = defaultUiaScriptPath,
    candidateVerifier = verifyDesktopCandidateManifest,
  } = {}) {
    this.profile = profile;
    this.environment = environment;
    this.agentSourcePath = agentPath;
    this.candidateSourcePath = candidatePath;
    this.cdpScriptPath = cdpScriptPath;
    this.uiaScriptPath = uiaScriptPath;
    this.candidateVerifier = candidateVerifier;
  }

  get testRoot() { return resolve(expandWindowsEnvironment(this.profile.testRootTemplate, this.environment)); }
  get agentPath() { return join(this.testRoot, "agent", "chemsema-gui-test-agent.exe"); }
  get stateRoot() { return join(this.testRoot, "state"); }
  get runsRoot() { return join(this.testRoot, "runs"); }
  get inputChannel() { return join(this.testRoot, "input-channel"); }
  get cdpChannel() { return join(this.testRoot, "cdp-channel"); }

  async validateProfile() {
    await assertValidDocument(this.profile, this.profile.id || "physical worker profile");
    if (this.profile.kind !== "physical-windows") throw new Error("Physical coordinator requires a physical-windows profile.");
    const root = this.testRoot;
    if (root === parse(root).root || root.length < 8) throw new Error("Physical worker test root is not bounded.");
    return this.profile;
  }

  async powershell(script, args = [], timeoutMs = 30000) {
    return parseJsonResult(await runProcess("powershell.exe", [
      "-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
      ...script,
      ...args,
    ], { timeoutMs }), args[0] || script.at(-1));
  }

  async attestHost() {
    await this.validateProfile();
    const source = [
      "$identity=[Security.Principal.WindowsIdentity]::GetCurrent().Name",
      "$session=[Diagnostics.Process]::GetCurrentProcess().SessionId",
      "$guid=(Get-ItemPropertyValue -LiteralPath 'HKLM:\\SOFTWARE\\Microsoft\\Cryptography' -Name MachineGuid).ToLowerInvariant()",
      "$algorithm=[Security.Cryptography.SHA256]::Create()",
      "try{$sha=$algorithm.ComputeHash([Text.Encoding]::UTF8.GetBytes($guid))}finally{$algorithm.Dispose()}",
      "$hash=($sha|ForEach-Object{$_.ToString('x2')}) -join ''",
      "[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='host-attest';host=[ordered]@{computerName=$env:COMPUTERNAME;account=$identity;sessionId=$session;machineIdSha256=$hash}}|ConvertTo-Json -Compress",
    ].join(";");
    const result = await this.powershell(["-Command", source]);
    const host = result.host || {};
    const failures = [];
    if (host.computerName?.toLowerCase() !== this.profile.machine.computerName.toLowerCase()) failures.push("computer name does not match the physical profile");
    if (host.machineIdSha256 !== this.profile.machine.machineIdSha256) failures.push("machine identity hash does not match the physical profile");
    if (host.account?.toLowerCase() !== this.profile.account.name.toLowerCase()) failures.push("Windows account does not match the physical profile");
    if (host.sessionId !== this.profile.account.sessionId || host.sessionId === 0) failures.push("interactive session does not match the physical profile");
    if (failures.length) throw new Error(`Physical host attestation failed closed: ${failures.join("; ")}.`);
    return result;
  }

  async reset() {
    await this.attestHost();
    await this.stop();
    for (const directory of [this.runsRoot, this.inputChannel, this.cdpChannel, join(this.testRoot, "documents"), join(this.testRoot, "artifacts")]) {
      await rm(directory, { recursive: true, force: true });
    }
    await mkdir(this.stateRoot, { recursive: true });
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "reset", machineIdSha256: this.profile.machine.machineIdSha256, state: "Ready" };
  }

  async start() {
    const host = await this.attestHost();
    await mkdir(this.stateRoot, { recursive: true });
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "start", machineIdSha256: host.host.machineIdSha256, state: "Running", startedByCoordinator: false };
  }

  async attestGuest({ requireInteractive = false } = {}) {
    const host = await this.attestHost();
    const guest = {
      computerName: host.host.computerName,
      identity: host.host.account,
      sessionId: host.host.sessionId,
      interactiveUser: host.host.account,
      interactiveAccountMatches: host.host.sessionId === this.profile.account.sessionId,
      physicalDesktop: true,
    };
    if (requireInteractive && !guest.interactiveAccountMatches) throw new Error("Physical interactive desktop is not the authorized session.");
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "guest-attest", guest };
  }

  async prepareGuest() {
    await this.attestGuest({ requireInteractive: true });
    await mkdir(this.testRoot, { recursive: true });
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "prepare-guest", testRoot: this.testRoot };
  }

  async installAgent() {
    await this.prepareGuest();
    const source = await fileIdentity(this.agentSourcePath);
    await mkdir(dirname(this.agentPath), { recursive: true });
    let reused = false;
    try { reused = (await fileIdentity(this.agentPath)).sha256 === source.sha256; } catch (error) { if (error.code !== "ENOENT") throw error; }
    if (!reused) await copyFile(this.agentSourcePath, this.agentPath);
    const installed = await fileIdentity(this.agentPath);
    if (installed.sha256 !== source.sha256) throw new Error("Installed physical input agent failed SHA-256 verification.");
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "install-agent", agent: { guestPath: this.agentPath, ...installed, reused } };
  }

  async configureAutologon() { throw new Error("Physical worker policy forbids automatic logon configuration."); }

  async configureDesktopBaseline() {
    await this.attestGuest({ requireInteractive: true });
    const command = "$ErrorActionPreference='Stop'; powercfg.exe /change standby-timeout-ac 0 | Out-Null; powercfg.exe /change hibernate-timeout-ac 0 | Out-Null; [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='configure-desktop-baseline';baseline=[ordered]@{scope='current-physical-test-account';changed=$false;standbyAcMinutes=0;hibernateAcMinutes=0;autologonConfigured=$false}}|ConvertTo-Json -Depth 4 -Compress";
    return this.powershell(["-Command", command]);
  }

  candidatePathForHash(hash) { return join(this.testRoot, "candidate", hash, "chemsema-desktop.exe"); }

  async installCandidate() {
    this.candidateVerifier();
    await this.prepareGuest();
    const source = await fileIdentity(this.candidateSourcePath);
    const installedPath = this.candidatePathForHash(source.sha256);
    await mkdir(dirname(installedPath), { recursive: true });
    let reused = false;
    try { reused = (await fileIdentity(installedPath)).sha256 === source.sha256; } catch (error) { if (error.code !== "ENOENT") throw error; }
    if (!reused) await copyFile(this.candidateSourcePath, installedPath);
    const installed = await fileIdentity(installedPath);
    if (installed.sha256 !== source.sha256) throw new Error("Installed physical candidate failed SHA-256 verification.");
    this.candidate = { guestPath: installedPath, ...installed, reused };
    await atomicJson(join(this.stateRoot, "candidate-identity.json"), this.candidate);
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "install-candidate", candidate: this.candidate };
  }

  async candidateIdentity() {
    if (!this.candidate) this.candidate = JSON.parse(await readFile(join(this.stateRoot, "candidate-identity.json"), "utf8"));
    const actual = await fileIdentity(this.candidate.guestPath);
    if (actual.sha256 !== this.candidate.sha256) throw new Error("Physical candidate content identity changed.");
    return this.candidate;
  }

  async launchCandidate() {
    await this.attestGuest({ requireInteractive: true });
    const candidate = await this.candidateIdentity();
    const logPath = join(dirname(candidate.guestPath), "webview.log");
    await rm(logPath, { force: true });
    const child = spawn(candidate.guestPath, [], {
      cwd: dirname(candidate.guestPath),
      env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--force-renderer-accessibility --remote-debugging-port=9223 --enable-logging --log-file=${logPath} --v=1` },
      detached: true,
      windowsHide: true,
      shell: false,
      stdio: "ignore",
    });
    child.unref();
    const receipt = { ...candidate, processId: child.pid, sessionId: this.profile.account.sessionId };
    await atomicJson(join(this.stateRoot, "candidate-process.json"), receipt);
    await delay(1000);
    const attestation = await this.runAgent(["attest"]);
    if (!attestation.foreground && child.exitCode !== null) throw new Error("Physical candidate exited before exposing a window.");
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "launch-candidate", candidate: receipt };
  }

  async runAgent(args, timeoutMs = 20000) {
    const result = await runProcess(this.agentPath, args, { timeoutMs });
    return cleanAgentAttestation(parseJsonResult(result, `agent ${args[0]}`));
  }

  async processReceipt() {
    const candidate = await this.candidateIdentity();
    const process = JSON.parse(await readFile(join(this.stateRoot, "candidate-process.json"), "utf8"));
    if (process.guestPath.toLowerCase() !== candidate.guestPath.toLowerCase() || process.sha256 !== candidate.sha256 || !Number.isInteger(process.processId)) {
      throw new Error("Physical candidate process receipt is invalid.");
    }
    return process;
  }

  async guard(prefix) {
    const process = await this.processReceipt();
    const runDirectory = join(this.runsRoot, `${prefix}-${randomUUID().replaceAll("-", "")}`);
    const guardPath = join(runDirectory, "guard.json");
    const guard = {
      expectedAgentAccount: this.profile.account.name,
      expectedAgentSessionId: this.profile.account.sessionId,
      expectedProcessId: process.processId,
      expectedExecutable: process.guestPath,
      allowedRunRoot: this.runsRoot,
      runDirectory,
    };
    await atomicJson(guardPath, guard);
    return { guardPath, process, runDirectory };
  }

  async activateCandidate() {
    const { guardPath, process } = await this.guard("activate");
    const agent = await this.runAgent(["activate", "--guard", guardPath]);
    if (!agent.interactiveReady || agent.foreground?.processId !== process.processId || agent.foreground?.executable?.toLowerCase() !== process.guestPath.toLowerCase()) {
      throw new Error("Physical candidate activation failed foreground identity validation.");
    }
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "activate-candidate", candidate: process, agent };
  }

  async dismissKnownBlocker() { throw new Error("Physical worker does not automatically dismiss account or system windows."); }

  async attestInteractiveAgent() {
    const agent = await this.runAgent(["attest"]);
    const failures = [];
    if (agent.account?.toLowerCase() !== this.profile.account.name.toLowerCase()) failures.push("agent account does not match the physical profile");
    if (agent.sessionId !== this.profile.account.sessionId || !agent.interactiveReady || agent.inputDesktop !== "Default") failures.push("agent is not attached to the authorized unlocked desktop");
    if (!agent.foreground || agent.foreground.sessionId !== agent.sessionId) failures.push("foreground window is absent or belongs to another session");
    if (failures.length) throw new Error(`Physical interactive agent attestation failed closed: ${failures.join("; ")}.`);
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "agent-attest-interactive", agent };
  }

  async startDetached(name, executable, args, readyPath, { powershellLauncher = false } = {}) {
    await rm(dirname(readyPath), { recursive: true, force: true });
    await mkdir(dirname(readyPath), { recursive: true });
    await mkdir(this.stateRoot, { recursive: true });
    const stdoutPath = join(this.stateRoot, `${name}.stdout.log`);
    const stderrPath = join(this.stateRoot, `${name}.stderr.log`);
    let processId;
    if (powershellLauncher) {
      const encodedArguments = Buffer.from(JSON.stringify(args), "utf8").toString("base64");
      const receiptPath = join(this.stateRoot, `${name}-launch.json`);
      await rm(receiptPath, { force: true });
      const launcher = spawn("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", backgroundLaunchScriptPath, "-Executable", executable, "-ArgumentsBase64", encodedArguments, "-StdoutPath", stdoutPath, "-StderrPath", stderrPath, "-ReceiptPath", receiptPath], { windowsHide: true, shell: false, stdio: "ignore" });
      launcher.unref();
      const launched = await waitForJson(receiptPath, 15000, `${name} launcher`);
      if (launched.schema !== "chemsema.gui.physical-background-process.v1" || !Number.isInteger(launched.processId)) throw new Error(`${name} background launcher returned an invalid receipt.`);
      processId = launched.processId;
    } else {
      const stdout = await open(stdoutPath, "w");
      const stderr = await open(stderrPath, "w");
      const child = spawn(executable, args, { detached: true, windowsHide: true, shell: false, stdio: ["ignore", stdout.fd, stderr.fd] });
      await stdout.close();
      await stderr.close();
      child.unref();
      processId = child.pid;
    }
    await atomicJson(join(this.stateRoot, `${name}-process.json`), { processId, executable, args, ready: null, stdoutPath, stderrPath });
    let ready;
    try {
      ready = await waitForJson(readyPath, 20000, name);
    } catch (error) {
      const stderrText = await readFile(stderrPath, "utf8").catch(() => "");
      throw new Error(`${error.message}${stderrText.trim() ? ` ${stderrText.trim()}` : ""}`);
    }
    await atomicJson(join(this.stateRoot, `${name}-process.json`), { processId, executable, args, ready, stdoutPath, stderrPath });
    return ready;
  }

  async startInputAgent() {
    const ready = await this.startDetached("input-agent", this.agentPath, ["serve", "--allowed-root", this.testRoot, "--channel-root", this.inputChannel], join(this.inputChannel, "ready.json"));
    if (ready.schema !== "chemsema.gui.guest-agent-server.v1" || ready.status !== "ready") throw new Error("Physical input agent returned an invalid readiness receipt.");
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "start-input-agent", agent: ready };
  }

  async startCdpAgent() {
    const ready = await this.startDetached("cdp-agent", "powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-File", this.cdpScriptPath, "-AllowedRoot", this.testRoot, "-ChannelRoot", this.cdpChannel], join(this.cdpChannel, "ready.json"), { powershellLauncher: true });
    if (ready.schema !== "chemsema.gui.cdp-server.v1" || ready.status !== "ready" || ready.port !== 9223 || ready.sessionId !== this.profile.account.sessionId || ready.account?.toLowerCase() !== this.profile.account.name.toLowerCase()) {
      throw new Error("Physical CDP agent returned an invalid readiness receipt.");
    }
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "start-cdp-agent", agent: ready };
  }

  async stopChannel(name, channel) {
    try { await writeFile(join(channel, "shutdown"), "\n", "utf8"); } catch (error) { if (error.code !== "ENOENT") throw error; }
    const deadline = Date.now() + 10000;
    let state;
    try { state = JSON.parse(await readFile(join(this.stateRoot, `${name}-process.json`), "utf8")); } catch (error) { if (error.code !== "ENOENT") throw error; }
    if (state?.processId) {
      do {
        if (!(await this.processDetails(state.processId)).exists) break;
        await delay(50);
      } while (Date.now() < deadline);
      if ((await this.processDetails(state.processId)).exists) await this.terminateRecordedProcess(state, name);
    }
    return { schema: "chemsema.gui.worker-attestation.v1", operation: `stop-${name}`, agent: { status: "stopped" } };
  }

  stopInputAgent() { return this.stopChannel("input-agent", this.inputChannel); }
  stopCdpAgent() { return this.stopChannel("cdp-agent", this.cdpChannel); }

  async processDetails(processId) {
    if (!Number.isInteger(processId) || processId <= 0) throw new Error("Recorded process id is invalid.");
    const command = `$p=Get-CimInstance Win32_Process -Filter \"ProcessId = ${processId}\" -ErrorAction SilentlyContinue;if($null-eq $p){[ordered]@{exists=$false}|ConvertTo-Json -Compress}else{[ordered]@{exists=$true;processId=[int]$p.ProcessId;executable=[string]$p.ExecutablePath;commandLine=[string]$p.CommandLine}|ConvertTo-Json -Compress}`;
    return this.powershell(["-Command", command]);
  }

  async terminateRecordedProcess(state, label) {
    const details = await this.processDetails(state.processId);
    if (!details.exists) return;
    const expectedExecutable = resolve(state.executable || state.guestPath || "").toLowerCase();
    const actualExecutable = resolve(details.executable || "").toLowerCase();
    const executableMatches = expectedExecutable === resolve("powershell.exe").toLowerCase()
      ? actualExecutable.endsWith("\\windowspowershell\\v1.0\\powershell.exe") || actualExecutable.endsWith("\\powershell.exe")
      : actualExecutable === expectedExecutable;
    const identityArguments = (state.args || []).filter((value) => typeof value === "string" && value.includes("\\") && value.length >= 8);
    if (!executableMatches || identityArguments.some((value) => !String(details.commandLine).toLowerCase().includes(value.toLowerCase()))) {
      throw new Error(`Refusing to terminate ${label}; recorded PID ${state.processId} no longer has the authorized process identity.`);
    }
    const killed = await runProcess("taskkill.exe", ["/PID", String(state.processId), "/T", "/F"], { timeoutMs: 15000 });
    if (killed.status !== 0 && (await this.processDetails(state.processId)).exists) throw new Error(`Failed to terminate authorized ${label} PID ${state.processId}.`);
  }

  async sendChannel(channel, requestSchema, responseSchema, fields, timeoutMs) {
    await readFile(join(channel, "ready.json"), "utf8");
    const id = randomUUID().replaceAll("-", "");
    const responsePath = join(channel, "outbox", `${id}.json`);
    await atomicJson(join(channel, "inbox", `${id}.json`), { schema: requestSchema, id, ...fields });
    const response = await waitForJson(responsePath, timeoutMs, channel);
    if (response.schema !== responseSchema || response.id !== id) throw new Error("Physical worker channel response identity is invalid.");
    if (response.status !== "passed") throw new Error(`Physical worker channel failed: ${response.message || "unknown failure"}`);
    return response;
  }

  validateCdpRequest(request) {
    const modes = ["locate", "state", "count", "count-state", "distinct-count", "distinct-count-state", "text", "text-state", "entity-rects-state", "ui-state", "trace-start", "trace-mark", "artifact-export"];
    if (!modes.includes(request?.mode)) throw new Error("CDP bridge requires a supported fixed mode.");
    if (["count", "count-state", "distinct-count", "distinct-count-state", "text", "text-state"].includes(request.mode) && (typeof request.selector !== "string" || !request.selector || request.selector.length > 2048)) throw new Error("CDP DOM observation requires a bounded selector.");
    if (request.mode === "ui-state") {
      const styles = ["backgroundColor", "borderColor", "boxShadow", "cursor", "display", "fill", "opacity", "outlineColor", "outlineStyle", "outlineWidth", "pointerEvents", "stroke", "strokeWidth", "visibility"];
      if (typeof request.selector !== "string" || !request.selector || request.selector.length > 2048) throw new Error("CDP UI state observation requires a bounded selector.");
      if (request.referenceSelector !== undefined && (typeof request.referenceSelector !== "string" || !request.referenceSelector || request.referenceSelector.length > 2048)) throw new Error("CDP UI state reference requires a bounded selector.");
      if (request.styleProperties !== undefined && (!Array.isArray(request.styleProperties) || request.styleProperties.length > styles.length || new Set(request.styleProperties).size !== request.styleProperties.length || request.styleProperties.some((property) => !styles.includes(property)))) throw new Error("CDP UI state styles must be unique allowlisted properties.");
    }
    if (request.mode.startsWith("distinct-count") && !["data-object-id", "data-node-id", "data-bond-id"].includes(request.attribute)) throw new Error("CDP distinct-count requires an allowlisted identity attribute.");
    if (request.mode === "artifact-export" && !/^[a-f0-9]{32}$/.test(request.artifactId || "")) throw new Error("CDP artifact export identity is invalid.");
    if (request.mode === "trace-mark" && !/^chemsema-action:[A-Za-z0-9._:-]{1,220}$/.test(request.name || "")) throw new Error("CDP trace mark is invalid.");
  }

  async cdpBridge(request) {
    this.validateCdpRequest(request);
    const requestBase64 = Buffer.from(JSON.stringify(request), "utf8").toString("base64");
    const response = await this.sendChannel(this.cdpChannel, "chemsema.gui.cdp-request.v1", "chemsema.gui.cdp-response.v1", { requestBase64 }, request.mode === "artifact-export" ? 90000 : 20000);
    if (response.bridge?.schema !== "chemsema.gui.cdp-bridge.v1" || response.bridge?.status !== "passed") throw new Error("Physical CDP bridge returned an invalid receipt.");
    return response.bridge.value;
  }

  inputArguments(kind, coordinates, { button = "left", steps = 8, modifiers = [] } = {}, guardPath) {
    if (!Array.isArray(modifiers) || modifiers.length > 3 || new Set(modifiers).size !== modifiers.length || modifiers.some((value) => !["Shift", "Control", "Alt"].includes(value))) throw new Error("Candidate pointer modifiers must be unique allowlisted values.");
    let args;
    if (kind === "click" && [coordinates.x, coordinates.y].every(Number.isInteger)) args = ["click", "--guard", guardPath, "--x", String(coordinates.x), "--y", String(coordinates.y), "--button", button];
    else if (kind === "drag" && [...coordinates.from, ...coordinates.to, steps].every(Number.isInteger)) args = ["drag", "--guard", guardPath, "--from-x", String(coordinates.from[0]), "--from-y", String(coordinates.from[1]), "--to-x", String(coordinates.to[0]), "--to-y", String(coordinates.to[1]), "--steps", String(steps), "--button", button];
    else if (kind === "key" && typeof coordinates.key === "string" && coordinates.key) args = ["key", "--guard", guardPath, "--key", coordinates.key];
    else if (kind === "text" && typeof coordinates.text === "string" && coordinates.text.length >= 1 && coordinates.text.length <= 4096) args = ["text", "--guard", guardPath, "--text-base64", Buffer.from(coordinates.text, "utf8").toString("base64")];
    else throw new Error(`Unsupported or invalid candidate input ${kind}.`);
    if (["click", "drag"].includes(kind) && modifiers.length) args.push("--modifiers", modifiers.join(","));
    return args;
  }

  async candidateInput(kind, coordinates, options = {}) {
    const { guardPath, process } = await this.guard(`input-${kind}`);
    const args = this.inputArguments(kind, coordinates, options, guardPath);
    const response = await this.sendChannel(this.inputChannel, "chemsema.gui.guest-agent-request.v1", "chemsema.gui.guest-agent-response.v1", { args }, 8000);
    const agent = cleanAgentAttestation(response.result);
    await assertValidDocument(agent, `physical candidate ${kind} attestation`);
    if (!agent.interactiveReady || agent.foreground?.processId !== process.processId || agent.foreground?.executable?.toLowerCase() !== process.guestPath.toLowerCase()) throw new Error(`Physical candidate ${kind} failed foreground identity validation.`);
    return { schema: "chemsema.gui.worker-attestation.v1", operation: `input-${kind}`, candidate: process, agent };
  }

  async queryUiaByAutomationId(automationId, { controlType, scopeName } = {}) { return this.queryUia(null, { automationId, controlType, scopeName }); }

  async queryUia(name, { automationId, controlType, scopeName } = {}) {
    if (!name && !automationId) throw new Error("UI Automation query requires an exact name or automation id.");
    const process = await this.processReceipt();
    const args = ["-File", this.uiaScriptPath, "-TargetProcessId", String(process.processId)];
    if (name) args.push("-ExactName", name);
    if (automationId) args.push("-ExactAutomationId", automationId);
    if (controlType) args.push("-ExpectedControlType", controlType);
    if (scopeName) args.push("-ScopeName", scopeName);
    const query = await this.powershell(args, [], 30000);
    if (!Array.isArray(query.matches) || query.processId !== process.processId) throw new Error("Physical UI Automation query returned an invalid receipt.");
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "uia-query", query };
  }

  async candidateAction(input, completion, budgetMs, actionId) {
    if (!candidateActionBudgetIsValid(budgetMs, completion?.timeoutMs)) throw new Error("Candidate action budget does not preserve the required transport reserve.");
    const request = { schema: "chemsema.gui.action-transaction.v1", actionId, input, completion, budgetMs };
    await assertValidDocument(request, "physical candidate action transaction request");
    const observe = (value) => this.cdpBridge(value);
    const mark = (phase) => observe({ mode: "trace-mark", name: `chemsema-action:${actionId}:${phase}` });
    let beforeObservation;
    await mark("start");
    if (completion.kind === "entity-rect-deltas") beforeObservation = await observe({ mode: "entity-rects-state", entityIds: completion.entities.map((entity) => entity.entityId) });
    else beforeObservation = await observe({ mode: "state" });
    const before = beforeObservation.state || beforeObservation;
    await mark("input-before");
    const coordinates = input.kind === "click" ? { x: input.x, y: input.y } : input.kind === "drag" ? { from: input.from, to: input.to } : { key: input.key };
    const inputReceipt = await this.candidateInput(input.kind, coordinates, { button: input.button, steps: input.steps, modifiers: input.modifiers });
    await mark("input-after");
    const deadline = Date.now() + completion.timeoutMs;
    let observed;
    let passed = false;
    do {
      if (completion.kind === "dom-count" || completion.kind === "dom-distinct-count") {
        observed = await observe({ mode: completion.kind === "dom-count" ? "count-state" : "distinct-count-state", selector: completion.selector, ...(completion.attribute ? { attribute: completion.attribute } : {}) });
        passed = completion.operator === "eq" ? observed.count === completion.value : observed.count >= completion.value;
      } else if (completion.kind === "dom-text") {
        observed = await observe({ mode: "text-state", selector: completion.selector });
        passed = observed.count === 1 && observed.text === completion.text;
      } else if (completion.kind === "entity-rect-deltas") {
        observed = await observe({ mode: "entity-rects-state", entityIds: completion.entities.map((entity) => entity.entityId) });
        const values = completion.entities.map((expectation) => {
          const first = beforeObservation.entities.find((entity) => entity.entityId === expectation.entityId);
          const last = observed.entities.find((entity) => entity.entityId === expectation.entityId);
          if (!first || !last || first.matchCount !== 1 || last.matchCount !== 1 || !first.visible || !last.visible) throw new Error(`Entity rectangle condition is ambiguous for ${expectation.entityId}.`);
          const maximumDeltaWorld = Math.max(...last.worldRect.map((value, index) => Math.abs(value - first.worldRect[index])));
          return { ...expectation, maximumDeltaWorld, beforeWorldRect: first.worldRect, afterWorldRect: last.worldRect, beforeRect: first.rect, afterRect: last.rect, passed: expectation.operator === "stationary" ? maximumDeltaWorld <= expectation.toleranceWorld : maximumDeltaWorld > expectation.toleranceWorld };
        });
        observed.observedEntities = values;
        passed = values.every((value) => value.passed);
      } else {
        observed = await observe({ mode: "state" });
        passed = true;
      }
      if (!passed) await delay(20);
    } while (!passed && Date.now() < deadline);
    if (!passed) throw new Error(`Physical action completion ${completion.kind} did not pass within ${completion.timeoutMs} ms.`);
    const after = observed.state || observed;
    const completionReceipt = completion.kind === "dom-text" ? { observedText: observed.text }
      : ["dom-count", "dom-distinct-count"].includes(completion.kind) ? { observed: observed.count }
        : completion.kind === "entity-rect-deltas" ? { entities: observed.observedEntities }
          : completion.kind === "actionable" ? { actionable: true } : { quiescent: true };
    await mark("complete");
    const transaction = { schema: "chemsema.gui.action-transaction-receipt.v1", input: inputReceipt.agent, before, after, completion: completionReceipt };
    await assertValidDocument(transaction, "physical candidate action transaction receipt");
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "action-transaction", candidate: await this.processReceipt(), transaction };
  }

  async prepareDocumentOutput(name = "roundtrip.ccjs") {
    if (!/^[a-z0-9][a-z0-9._-]{0,95}\.ccjs$/.test(name)) throw new Error("Document output name must be a bounded safe CCJS filename.");
    const id = randomUUID().replaceAll("-", "");
    const directory = join(this.testRoot, "documents", id);
    await rm(directory, { recursive: true, force: true });
    await mkdir(directory, { recursive: true });
    return { id, name, guestPath: join(directory, name), exists: false };
  }

  async fetchDocumentOutput(output) {
    if (!/^[a-f0-9]{32}$/.test(output?.id || "") || !/^[a-z0-9][a-z0-9._-]{0,95}\.ccjs$/.test(output?.name || "")) throw new Error("Document output receipt is invalid.");
    const expected = resolve(join(this.testRoot, "documents", output.id, output.name));
    if (resolve(output.guestPath).toLowerCase() !== expected.toLowerCase()) throw new Error("Document output escaped the physical worker root.");
    const deadline = Date.now() + 30000;
    let bytes;
    do {
      try { bytes = await readFile(expected); if (bytes.length) break; } catch (error) { if (error.code !== "ENOENT") throw error; }
      await delay(50);
    } while (Date.now() < deadline);
    if (!bytes?.length || bytes.length > 64 * 1024 * 1024) throw new Error("Physical document output is absent or has an invalid size.");
    return { ...output, hostPath: expected, size: bytes.length, sha256: sha256(bytes), bytes };
  }

  async fetchArtifacts(manifest) {
    if (manifest?.schema !== "chemsema.gui.guest-artifact-export.v1" || !/^[a-f0-9]{32}$/.test(manifest.artifactId || "") || !Array.isArray(manifest.artifacts) || manifest.artifacts.length < 1 || manifest.artifacts.length > 16) throw new Error("Physical artifact export manifest is invalid.");
    const root = `${resolve(join(this.testRoot, "artifacts", manifest.artifactId))}\\`.toLowerCase();
    const names = new Set();
    const payloads = [];
    for (const artifact of manifest.artifacts) {
      const path = resolve(artifact.guestPath || "");
      if (!/^[a-z0-9][a-z0-9._-]{0,127}$/.test(artifact.name || "") || names.has(artifact.name) || !path.toLowerCase().startsWith(root) || !artifact.mediaType?.includes("/")) throw new Error("Physical artifact path or metadata is invalid.");
      names.add(artifact.name);
      const bytes = await readFile(path);
      if (bytes.length !== artifact.size || bytes.length > 64 * 1024 * 1024 || sha256(bytes) !== artifact.sha256) throw new Error(`Physical artifact ${artifact.name} failed SHA-256 verification.`);
      payloads.push({ name: artifact.name, mediaType: artifact.mediaType, bytes });
    }
    return payloads;
  }

  async stop() {
    for (const [name, channel] of [["cdp-agent", this.cdpChannel], ["input-agent", this.inputChannel]]) {
      try { await this.stopChannel(name, channel); } catch { /* Continue to the PID-bound candidate cleanup. */ }
    }
    let process;
    try { process = JSON.parse(await readFile(join(this.stateRoot, "candidate-process.json"), "utf8")); } catch (error) { if (error.code !== "ENOENT") throw error; }
    if (Number.isInteger(process?.processId) && process?.guestPath) {
      const candidate = await this.candidateIdentity();
      if (candidate.guestPath.toLowerCase() !== process.guestPath.toLowerCase() || candidate.sha256 !== process.sha256) throw new Error("Refusing to stop a candidate whose identity receipt changed.");
      await this.terminateRecordedProcess({ ...process, executable: process.guestPath, args: [process.guestPath] }, "candidate");
    }
    return { schema: "chemsema.gui.worker-attestation.v1", operation: "stop", machineIdSha256: this.profile.machine.machineIdSha256, state: "Stopped" };
  }
}
