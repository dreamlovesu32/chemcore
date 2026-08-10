import { spawn } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertValidDocument } from "../protocol/validate.mjs";
import { verifyDesktopCandidateManifest } from "../../../../scripts/candidate-source-identity.mjs";

const scriptPath = join(dirname(dirname(dirname(fileURLToPath(import.meta.url)))), "scripts", "hyperv-coordinator.ps1");
const repositoryRoot = dirname(dirname(dirname(dirname(dirname(fileURLToPath(import.meta.url))))));
const defaultAgentPath = join(repositoryRoot, "target", "release", "chemsema-gui-test-agent.exe");
const defaultCandidatePath = join(repositoryRoot, "target", "release", "chemsema-desktop.exe");
const defaultCdpScriptPath = join(repositoryRoot, "packages", "gui-test", "scripts", "guest-cdp.ps1");
const actionBrokerPath = join(repositoryRoot, "packages", "gui-test", "scripts", "hyperv-action-broker.ps1");

export function expandWindowsEnvironment(template, environment = process.env) {
  return template.replace(/%([^%]+)%/g, (_, name) => {
    const key = Object.keys(environment).find((candidate) => candidate.toLowerCase() === name.toLowerCase());
    if (!key || !environment[key]) {
      throw new Error(`Worker profile references unavailable environment variable %${name}%.`);
    }
    return environment[key];
  });
}

function defaultExecutor(args, { timeoutMs = 120000 } = {}) {
  return new Promise((resolve) => {
    const child = spawn("powershell.exe", args, { windowsHide: true, shell: false });
    let stdout = "";
    let stderr = "";
    let failure = null;
    const append = (field, chunk) => {
      if (Buffer.byteLength(field + chunk, "utf8") > 10 * 1024 * 1024) {
        failure = new Error("Hyper-V coordinator output exceeded 10 MiB.");
        child.kill();
        return field;
      }
      return field + chunk;
    };
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk.toString("utf8")); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk.toString("utf8")); });
    child.on("error", (error) => { failure = error; });
    const timer = setTimeout(() => {
      failure = new Error(`Hyper-V coordinator exceeded ${timeoutMs} ms.`);
      child.kill();
    }, timeoutMs);
    child.on("close", (status) => {
      clearTimeout(timer);
      resolve({ status: status ?? 1, stdout, stderr, error: failure });
    });
  });
}

class PersistentActionExecutor {
  constructor({ profile, environment }) {
    this.profile = profile;
    this.environment = environment;
    this.pending = new Map();
    this.stdout = "";
  }

  async start() {
    if (this.child) return this.ready;
    const credentialPath = expandWindowsEnvironment(this.profile.credential.pathTemplate, this.environment);
    this.ready = new Promise((resolve, reject) => { this.resolveReady = resolve; this.rejectReady = reject; });
    this.child = spawn("powershell.exe", [
      "-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
      "-File", actionBrokerPath,
      "-CoordinatorPath", scriptPath,
      "-VmId", this.profile.vm.id,
      "-CredentialPath", credentialPath,
    ], { windowsHide: true, shell: false, stdio: ["pipe", "pipe", "pipe"] });
    this.child.stdout.on("data", (chunk) => this.onStdout(chunk.toString("utf8")));
    this.child.stderr.on("data", (chunk) => { this.stderr = `${this.stderr || ""}${chunk.toString("utf8")}`.slice(-65536); });
    this.child.on("error", (error) => this.failAll(error));
    this.child.on("close", (code) => this.failAll(new Error(`Persistent action broker exited with status ${code}: ${this.stderr || "no stderr"}`)));
    const timer = setTimeout(() => this.rejectReady(new Error("Persistent action broker did not become ready within 30 seconds.")), 30000);
    try { await this.ready; } finally { clearTimeout(timer); }
  }

  onStdout(chunk) {
    this.stdout += chunk;
    const lines = this.stdout.split(/\r?\n/);
    this.stdout = lines.pop() || "";
    for (const line of lines.filter(Boolean)) {
      let message;
      try { message = JSON.parse(line); } catch { this.failAll(new Error("Persistent action broker emitted malformed JSON.")); continue; }
      if (message.schema === "chemsema.gui.host-action-broker.v1") {
        if (message.status === "ready") this.resolveReady(message);
        else this.rejectReady(new Error(message.message || "Persistent action broker failed."));
        continue;
      }
      const pending = this.pending.get(message.id);
      if (!pending || message.schema !== "chemsema.gui.host-action-response.v1") { this.failAll(new Error("Persistent action broker response identity is invalid.")); continue; }
      this.pending.delete(message.id);
      clearTimeout(pending.timer);
      pending.resolve({ status: message.status, stdout: message.stdout || "", stderr: message.stderr || "" });
    }
  }

  failAll(error) {
    this.rejectReady?.(error);
    for (const pending of this.pending.values()) { clearTimeout(pending.timer); pending.reject(error); }
    this.pending.clear();
  }

  async execute(fullArguments, { timeoutMs }) {
    await this.start();
    const fileIndex = fullArguments.indexOf("-File");
    if (fileIndex < 0 || fullArguments[fileIndex + 1] !== scriptPath) throw new Error("Action broker received an invalid coordinator command.");
    const id = randomUUID().replaceAll("-", "");
    const request = { schema: "chemsema.gui.host-action-request.v1", id, arguments: fullArguments.slice(fileIndex + 2) };
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`Persistent action broker exceeded ${timeoutMs} ms.`)); }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      this.child.stdin.write(`${JSON.stringify(request)}\n`, "utf8");
    });
  }

  async close() {
    if (!this.child) return;
    const child = this.child;
    this.child = null;
    child.stdin.end();
    await new Promise((resolve) => { const timer = setTimeout(() => { child.kill(); resolve(); }, 10000); child.once("close", () => { clearTimeout(timer); resolve(); }); });
  }
}

function parseResult(result, operation) {
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Hyper-V ${operation} failed: ${(result.stderr || result.stdout || "unknown PowerShell error").trim()}`);
  }
  const lines = String(result.stdout || "").split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const jsonLine = [...lines].reverse().find((line) => line.startsWith("{") && line.endsWith("}"));
  if (!jsonLine) {
    throw new Error(`Hyper-V ${operation} returned no structured attestation.`);
  }
  return JSON.parse(jsonLine);
}

function cleanAgentAttestation(agent = {}) {
  const foreground = agent.foreground ? {
    windowHandle: agent.foreground.windowHandle,
    processId: agent.foreground.processId,
    sessionId: agent.foreground.sessionId,
    executable: agent.foreground.executable,
    title: agent.foreground.title,
    className: agent.foreground.className,
    rect: agent.foreground.rect,
    clientRect: agent.foreground.clientRect,
  } : null;
  return {
    schema: agent.schema,
    agentVersion: agent.agentVersion,
    processId: agent.processId,
    sessionId: agent.sessionId,
    account: agent.account,
    inputDesktop: agent.inputDesktop ?? null,
    interactiveReady: agent.interactiveReady,
    foreground,
  };
}

export class HyperVCoordinator {
  constructor(profile, {
    executor = defaultExecutor,
    environment = process.env,
    candidateVerifier = executor === defaultExecutor ? verifyDesktopCandidateManifest : () => null,
  } = {}) {
    this.profile = profile;
    this.executor = executor;
    this.environment = environment;
    this.candidateVerifier = candidateVerifier;
    this.actionExecutor = executor === defaultExecutor ? new PersistentActionExecutor({ profile, environment }) : null;
  }

  async validateProfile() {
    await assertValidDocument(this.profile, this.profile.id || "worker profile");
    if (this.profile.resources.cpuUnits + 2 > 10 || this.profile.resources.memoryGiB + 10 > 30) {
      throw new Error(`Worker profile ${this.profile.id} exceeds the aggregate host reserve contract.`);
    }
    return this.profile;
  }

  argumentsFor(operation, extraArguments = []) {
    const credentialPath = expandWindowsEnvironment(this.profile.credential.pathTemplate, this.environment);
    return [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy", "Bypass",
      "-File", scriptPath,
      "-Operation", operation,
      "-VmId", this.profile.vm.id,
      "-CheckpointId", this.profile.vm.checkpoint.id,
      "-CredentialPath", credentialPath,
      "-GuestAccount", this.profile.guest.account,
      "-GuestTestRoot", this.profile.guest.testRoot,
      "-HostAgentPath", defaultAgentPath,
      "-HostCandidatePath", defaultCandidatePath,
      "-HostCdpScriptPath", defaultCdpScriptPath,
      ...extraArguments,
    ];
  }

  async execute(operation, extraArguments = [], { timeoutMs = 120000 } = {}) {
    await this.validateProfile();
    return parseResult(await this.executor(this.argumentsFor(operation, extraArguments), { timeoutMs }), operation);
  }

  async attestHost() {
    const result = await this.execute("host-attest");
    const failures = [];
    if (!result.host?.hyperVAdministrator) failures.push("current host token is not in Hyper-V Administrators");
    if (result.host?.vmms !== "Running" || result.host?.vmcompute !== "Running") failures.push("Hyper-V services are not running");
    if (String(result.vm?.id || "").toLowerCase() !== this.profile.vm.id.toLowerCase() || result.vm?.generation !== this.profile.vm.generation) failures.push("VM identity or generation does not match the profile");
    if (result.vm?.automaticCheckpoints !== false) failures.push("automatic checkpoints are enabled");
    if (String(result.vm?.checkpointId || "").toLowerCase() !== this.profile.vm.checkpoint.id.toLowerCase() || result.vm?.checkpointName !== this.profile.vm.checkpoint.name) failures.push("deterministic baseline checkpoint does not match the profile");
    if (result.vm?.cpuUnits > this.profile.resources.cpuUnits) failures.push("VM CPU allocation exceeds the profile");
    if (result.vm?.memoryMaximumBytes > this.profile.resources.memoryGiB * 1024 ** 3) failures.push("VM maximum memory exceeds the profile");
    if (!result.credential?.exists) failures.push("encrypted PowerShell Direct credential is unavailable");
    if (failures.length) {
      throw new Error(`Host attestation failed closed: ${failures.join("; ")}.`);
    }
    return result;
  }

  async start() {
    await this.attestHost();
    return this.execute("start");
  }

  async reset() {
    await this.actionExecutor?.close();
    await this.attestHost();
    const result = await this.execute("reset", [], { timeoutMs: 120000 });
    if (String(result.checkpoint?.id || "").toLowerCase() !== this.profile.vm.checkpoint.id.toLowerCase() || result.state !== "Off") {
      throw new Error("Deterministic worker reset returned an invalid receipt.");
    }
    return result;
  }

  async attestGuest({ requireInteractive = false } = {}) {
    const result = await this.execute("guest-attest");
    const guest = result.guest || {};
    const failures = [];
    if (!String(guest.identity || "").toLowerCase().endsWith(`\\${this.profile.guest.account.toLowerCase()}`)) failures.push("PowerShell Direct guest identity does not match the dedicated account");
    if (guest.vmicvmsession !== "Running") failures.push("PowerShell Direct integration service is not running");
    if (requireInteractive && !guest.interactiveAccountMatches) failures.push("dedicated interactive guest session is not active and unlocked");
    if (failures.length) {
      throw new Error(`Guest attestation failed closed: ${failures.join("; ")}.`);
    }
    return result;
  }

  async prepareGuest() {
    await this.attestGuest();
    return this.execute("prepare-guest");
  }

  async installAgent() {
    await this.prepareGuest();
    return this.execute("install-agent");
  }

  async configureAutologon() {
    await this.attestGuest();
    const result = await this.execute("configure-autologon");
    if (result.autologon?.lsaSecretStored !== true
      || result.autologon?.plainRegistryPasswordPresent !== false
      || result.autologon?.autoAdminLogon !== "1") {
      throw new Error("Autologon configuration failed its secret-storage contract.");
    }
    return result;
  }

  async configureDesktopBaseline() {
    await this.attestGuest();
    const result = await this.execute("configure-desktop-baseline");
    const settings = result.baseline?.settings || {};
    const valid = result.baseline?.scope === "dedicated-test-user"
      && settings.scoobeSystemSettingEnabled === 0
      && settings.contentDeliveryAllowed === 0
      && settings.oemPreInstalledAppsEnabled === 0
      && settings.preInstalledAppsEnabled === 0
      && settings.preInstalledAppsEverEnabled === 0
      && settings.silentInstalledAppsEnabled === 0
      && settings.systemPaneSuggestionsEnabled === 0
      && settings.rotatingLockScreenEnabled === 0
      && settings.rotatingLockScreenOverlayEnabled === 0
      && settings.contentDeliverySoftLandingEnabled === 0
      && settings.subscribedContent310093Enabled === 0
      && settings.subscribedContent338389Enabled === 0;
    if (!valid) throw new Error("Desktop baseline failed its post-logon experience policy contract.");
    return result;
  }

  async installCandidate() {
    this.candidateVerifier();
    await this.prepareGuest();
    const result = await this.execute("install-candidate");
    if (!result.candidate?.sha256 || !result.candidate?.guestPath) {
      throw new Error("Candidate installation returned no content identity.");
    }
    return result;
  }

  async launchCandidate() {
    await this.attestGuest({ requireInteractive: true });
    const result = await this.execute("launch-candidate");
    const candidate = result.candidate || {};
    const failures = [];
    if (!candidate.sha256 || !candidate.guestPath) failures.push("candidate content identity is absent");
    if (!Number.isInteger(candidate.processId) || candidate.processId <= 0) failures.push("candidate PID is invalid");
    if (!Number.isInteger(candidate.sessionId) || candidate.sessionId === 0) failures.push("candidate is not in an interactive session");
    if (failures.length) {
      throw new Error(`Candidate launch failed closed: ${failures.join("; ")}.`);
    }
    return result;
  }

  async activateCandidate() {
    const result = await this.execute("activate-candidate", [], { timeoutMs: 20000 });
    const agent = cleanAgentAttestation(result.agent);
    await assertValidDocument(agent, "candidate activation attestation");
    const expectedExecutable = result.candidate?.guestPath;
    const failures = [];
    if (!agent.interactiveReady || agent.sessionId === 0 || agent.inputDesktop !== "Default") failures.push("agent is not on the interactive Default desktop");
    if (!agent.foreground || agent.foreground.processId <= 0) failures.push("candidate did not become foreground");
    if (!expectedExecutable || agent.foreground?.executable?.toLowerCase() !== expectedExecutable.toLowerCase()) failures.push("foreground executable is not the content-addressed candidate");
    if (failures.length) {
      throw new Error(`Candidate activation failed closed: ${failures.join("; ")}.`);
    }
    return { ...result, agent };
  }

  async dismissKnownBlocker() {
    const result = await this.execute("dismiss-known-blocker");
    const agent = cleanAgentAttestation(result.agent);
    await assertValidDocument(agent, "blocker dismissal attestation");
    return { ...result, agent };
  }

  async queryUia(name, { scopeName } = {}) {
    if (!name) throw new Error("UI Automation query requires an exact accessible name.");
    const extra = ["-AutomationName", name];
    if (scopeName) extra.push("-AutomationScopeName", scopeName);
    const result = await this.execute("uia-query", extra);
    if (result.query?.name !== name || !Array.isArray(result.query?.matches)) {
      throw new Error("UI Automation query returned an invalid receipt.");
    }
    return result;
  }

  async queryUiaByAutomationId(automationId, { controlType, scopeName } = {}) {
    if (!automationId) throw new Error("UI Automation query requires an exact automation id.");
    const extra = ["-AutomationId", automationId];
    if (controlType) extra.push("-AutomationControlType", controlType);
    if (scopeName) extra.push("-AutomationScopeName", scopeName);
    const result = await this.execute("uia-query", extra);
    if (result.query?.automationId !== automationId || !Array.isArray(result.query?.matches)) {
      throw new Error("UI Automation id query returned an invalid receipt.");
    }
    return result;
  }

  async cdpBridge(request) {
    if (!["locate", "state", "count", "count-state", "distinct-count", "distinct-count-state", "entity-rects-state", "trace-start", "artifact-export"].includes(request?.mode)) {
      throw new Error("CDP bridge requires a supported fixed mode.");
    }
    if (["count", "count-state", "distinct-count", "distinct-count-state"].includes(request.mode)
      && (typeof request.selector !== "string" || !request.selector || request.selector.length > 2048)) {
      throw new Error("CDP count observation requires a selector of 1 to 2048 characters.");
    }
    if (request.mode === "entity-rects-state" && (!Array.isArray(request.entityIds) || request.entityIds.length < 1 || request.entityIds.length > 16 || new Set(request.entityIds).size !== request.entityIds.length || request.entityIds.some((id) => typeof id !== "string" || !id || id.length > 128))) {
      throw new Error("CDP entity rectangle observation requires 1 to 16 unique bounded ids.");
    }
    if (request.mode.startsWith("distinct-count")) {
      if (typeof request.selector !== "string" || !request.selector || !["data-object-id", "data-node-id", "data-bond-id"].includes(request.attribute)) {
        throw new Error("CDP distinct-count requires a selector and an allowlisted identity attribute.");
      }
    }
    if (request.mode === "artifact-export" && !/^[a-f0-9]{32}$/.test(request.artifactId || "")) {
      throw new Error("CDP artifact export requires a 32-character lowercase hexadecimal identity.");
    }
    const encoded = Buffer.from(JSON.stringify(request), "utf8").toString("base64");
    const timeoutMs = request.mode === "artifact-export" ? 110000 : 40000;
    const result = await this.execute("cdp-bridge", ["-CdpRequestBase64", encoded], { timeoutMs });
    if (result.bridge?.schema !== "chemsema.gui.cdp-bridge.v1" || result.bridge?.status !== "passed") {
      throw new Error("CDP bridge returned an invalid receipt.");
    }
    return result.bridge.value;
  }

  async fetchArtifacts(exportManifest) {
    if (exportManifest?.schema !== "chemsema.gui.guest-artifact-export.v1" || !Array.isArray(exportManifest.artifacts)) {
      throw new Error("Guest artifact export manifest is invalid.");
    }
    const hostRoot = await mkdtemp(join(tmpdir(), "chemsema-gui-artifacts-"));
    try {
      const encoded = Buffer.from(JSON.stringify(exportManifest), "utf8").toString("base64");
      const result = await this.execute("fetch-artifacts", ["-ArtifactManifestBase64", encoded, "-HostArtifactRoot", hostRoot], { timeoutMs: 180000 });
      if (result.transfer?.schema !== "chemsema.gui.host-artifact-transfer.v1" || !Array.isArray(result.transfer.artifacts)) {
        throw new Error("Host artifact transfer receipt is invalid.");
      }
      const resolvedRoot = `${resolve(hostRoot)}\\`;
      const payloads = [];
      for (const artifact of result.transfer.artifacts) {
        const hostPath = resolve(artifact.hostPath || "");
        if (!hostPath.startsWith(resolvedRoot)) throw new Error(`Transferred artifact ${artifact.name} escaped the host staging root.`);
        const bytes = await readFile(hostPath);
        const actualHash = createHash("sha256").update(bytes).digest("hex");
        if (bytes.length !== artifact.size || actualHash !== artifact.sha256) {
          throw new Error(`Transferred artifact ${artifact.name} failed host SHA-256 verification.`);
        }
        payloads.push({ name: artifact.name, mediaType: artifact.mediaType, bytes });
      }
      return payloads;
    } finally {
      await rm(hostRoot, { recursive: true, force: true });
    }
  }

  async prepareDocumentOutput(name = "roundtrip.ccjs") {
    if (!/^[a-z0-9][a-z0-9._-]{0,95}\.ccjs$/.test(name)) {
      throw new Error("Document output name must be a bounded safe CCJS filename.");
    }
    const id = randomUUID().replaceAll("-", "");
    const result = await this.execute("prepare-document-output", ["-DocumentOutputId", id, "-DocumentOutputName", name]);
    const output = result.output || {};
    if (output.id !== id || output.name !== name || output.exists !== false || typeof output.guestPath !== "string") {
      throw new Error("Document output preparation returned an invalid receipt.");
    }
    return output;
  }

  async fetchDocumentOutput(output) {
    if (!/^[a-f0-9]{32}$/.test(output?.id || "") || !/^[a-z0-9][a-z0-9._-]{0,95}\.ccjs$/.test(output?.name || "")) {
      throw new Error("Document output receipt is invalid.");
    }
    const hostRoot = await mkdtemp(join(tmpdir(), "chemsema-gui-document-"));
    try {
      const result = await this.execute("fetch-document-output", [
        "-DocumentOutputId", output.id,
        "-DocumentOutputName", output.name,
        "-HostArtifactRoot", hostRoot,
      ], { timeoutMs: 60000 });
      const received = result.output || {};
      const hostPath = resolve(received.hostPath || "");
      const boundedRoot = `${resolve(hostRoot)}\\`;
      if (received.id !== output.id || received.name !== output.name || !hostPath.startsWith(boundedRoot)) {
        throw new Error("Transferred document output escaped the host staging root.");
      }
      const bytes = await readFile(hostPath);
      const sha256 = createHash("sha256").update(bytes).digest("hex");
      if (bytes.length !== received.size || sha256 !== received.sha256) {
        throw new Error("Transferred document output failed host SHA-256 verification.");
      }
      return { ...received, bytes };
    } finally {
      await rm(hostRoot, { recursive: true, force: true });
    }
  }

  async startCdpAgent() {
    const result = await this.execute("start-cdp-agent", [], { timeoutMs: 30000 });
    await assertValidDocument(result.agent, "persistent CDP agent readiness");
    if (result.agent?.schema !== "chemsema.gui.cdp-server.v1" || result.agent?.status !== "ready" || result.agent?.port !== 9223 || result.agent?.sessionId !== 0 || !result.agent?.account?.toLowerCase().endsWith("\\system")) {
      throw new Error("Persistent CDP agent returned an invalid readiness receipt.");
    }
    return result;
  }

  async stopCdpAgent() {
    return this.execute("stop-cdp-agent", [], { timeoutMs: 20000 });
  }

  async candidateAction(input, completion, budgetMs) {
    const minimumTransactionBudgetMs = 30000;
    const transportReserveMs = 4000;
    if (!Number.isInteger(budgetMs) || budgetMs < minimumTransactionBudgetMs) {
      throw new Error(`Candidate action end-to-end budget must be at least ${minimumTransactionBudgetMs} ms so host/guest transport cannot consume the product completion window.`);
    }
    if (!Number.isInteger(budgetMs) || !Number.isInteger(completion?.timeoutMs) || completion.timeoutMs + transportReserveMs > budgetMs) {
      throw new Error(`Candidate action completion timeout must leave ${transportReserveMs} ms inside the end-to-end action budget for target resolution and transport.`);
    }
    const request = { schema: "chemsema.gui.action-transaction.v1", input, completion, budgetMs };
    await assertValidDocument(request, "candidate action transaction request");
    const encoded = Buffer.from(JSON.stringify(request), "utf8").toString("base64");
    const argumentsForAction = this.argumentsFor("action-transaction", ["-ActionRequestBase64", encoded]);
    const raw = this.actionExecutor
      ? await this.actionExecutor.execute(argumentsForAction, { timeoutMs: Math.max(30000, budgetMs + 10000) })
      : await this.executor(argumentsForAction, { timeoutMs: Math.max(30000, budgetMs + 10000) });
    const result = parseResult(raw, "action-transaction");
    const agent = cleanAgentAttestation(result.transaction?.input);
    const transaction = { ...result.transaction, input: agent };
    await assertValidDocument(agent, "candidate action transaction input attestation");
    await assertValidDocument(transaction, "candidate action transaction receipt");
    if (!agent.interactiveReady || agent.foreground?.executable?.toLowerCase() !== result.candidate?.guestPath?.toLowerCase()) {
      throw new Error("Candidate action transaction failed foreground identity validation.");
    }
    return { ...result, transaction };
  }

  async candidateInput(kind, coordinates, { button = "left", steps = 8, modifiers = [] } = {}) {
    const extra = ["-InputButton", button];
    if (!Array.isArray(modifiers) || modifiers.length > 3 || new Set(modifiers).size !== modifiers.length || modifiers.some((value) => !["Shift", "Control", "Alt"].includes(value))) {
      throw new Error("Candidate pointer modifiers must be unique allowlisted values.");
    }
    if (modifiers.length) extra.push("-InputModifiers", modifiers.join(","));
    if (kind === "click") {
      if (![coordinates.x, coordinates.y].every(Number.isInteger)) {
        throw new Error("Candidate click coordinates must be integers.");
      }
      extra.push("-InputX", String(coordinates.x), "-InputY", String(coordinates.y));
    } else if (kind === "drag") {
      if (![...coordinates.from, ...coordinates.to, steps].every(Number.isInteger)) {
        throw new Error("Candidate drag coordinates and steps must be integers.");
      }
      extra.push(
        "-InputFromX", String(coordinates.from[0]), "-InputFromY", String(coordinates.from[1]),
        "-InputToX", String(coordinates.to[0]), "-InputToY", String(coordinates.to[1]),
        "-InputSteps", String(steps),
      );
    } else if (kind === "key") {
      if (typeof coordinates?.key !== "string" || !coordinates.key) throw new Error("Keyboard input requires a shortcut.");
      extra.push("-InputKey", coordinates.key);
    } else if (kind === "text") {
      if (typeof coordinates?.text !== "string" || !coordinates.text || coordinates.text.length > 4096) throw new Error("Text input requires 1..4096 characters.");
      extra.push("-InputTextBase64", Buffer.from(coordinates.text, "utf8").toString("base64"));
    } else {
      throw new Error(`Unsupported candidate input kind ${kind}.`);
    }
    const result = await this.execute(`input-${kind}`, extra);
    const agent = cleanAgentAttestation(result.agent);
    await assertValidDocument(agent, `candidate ${kind} attestation`);
    if (!agent.interactiveReady || agent.foreground?.executable?.toLowerCase() !== result.candidate?.guestPath?.toLowerCase()) {
      throw new Error(`Candidate ${kind} receipt failed foreground identity validation.`);
    }
    return { ...result, agent };
  }

  async startInputAgent() {
    const result = await this.execute("start-input-agent", [], { timeoutMs: 30000 });
    if (result.agent?.schema !== "chemsema.gui.guest-agent-server.v1" || result.agent?.status !== "ready") {
      throw new Error("Persistent input agent returned an invalid readiness receipt.");
    }
    return result;
  }

  async stopInputAgent() {
    return this.execute("stop-input-agent", [], { timeoutMs: 20000 });
  }

  async attestServiceAgent() {
    const result = await this.execute("agent-attest-service");
    const agent = cleanAgentAttestation(result.agent);
    await assertValidDocument(agent, "service agent attestation");
    if (agent.sessionId !== 0 || agent.interactiveReady !== false) {
      throw new Error("Service agent attestation unexpectedly claimed an interactive desktop.");
    }
    return { ...result, agent };
  }

  async attestInteractiveAgent() {
    const result = await this.execute("agent-attest-interactive");
    const agent = cleanAgentAttestation(result.agent);
    await assertValidDocument(agent, "interactive agent attestation");
    const failures = [];
    if (!String(agent.account).toLowerCase().endsWith(`\\${this.profile.guest.account.toLowerCase()}`)) failures.push("agent account does not match the dedicated guest account");
    if (agent.sessionId === 0 || agent.interactiveReady !== true || agent.inputDesktop !== "Default") failures.push("agent is not attached to an unlocked interactive Default desktop");
    if (!agent.foreground || agent.foreground.sessionId !== agent.sessionId) failures.push("foreground window is absent or belongs to another session");
    if (failures.length) {
      throw new Error(`Interactive agent attestation failed closed: ${failures.join("; ")}.`);
    }
    return { ...result, agent };
  }

  async stop() {
    await this.actionExecutor?.close();
    return this.execute("stop");
  }
}
