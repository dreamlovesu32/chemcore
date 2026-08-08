import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { assertValidDocument } from "../protocol/validate.mjs";

const scriptPath = join(dirname(dirname(dirname(fileURLToPath(import.meta.url)))), "scripts", "hyperv-coordinator.ps1");
const repositoryRoot = dirname(dirname(dirname(dirname(dirname(fileURLToPath(import.meta.url))))));
const defaultAgentPath = join(repositoryRoot, "target", "release", "chemsema-gui-test-agent.exe");
const defaultCandidatePath = join(repositoryRoot, "target", "release", "chemsema-desktop.exe");
const defaultCdpScriptPath = join(repositoryRoot, "packages", "gui-test", "scripts", "guest-cdp.ps1");

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
  constructor(profile, { executor = defaultExecutor, environment = process.env } = {}) {
    this.profile = profile;
    this.executor = executor;
    this.environment = environment;
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

  async cdpBridge(request) {
    if (!["locate", "state", "count", "count-state"].includes(request?.mode)) {
      throw new Error("CDP bridge requires a supported fixed mode.");
    }
    const encoded = Buffer.from(JSON.stringify(request), "utf8").toString("base64");
    const result = await this.execute("cdp-bridge", ["-CdpRequestBase64", encoded]);
    if (result.bridge?.schema !== "chemsema.gui.cdp-bridge.v1" || result.bridge?.status !== "passed") {
      throw new Error("CDP bridge returned an invalid receipt.");
    }
    return result.bridge.value;
  }

  async candidateInput(kind, coordinates, { button = "left", steps = 8 } = {}) {
    const extra = ["-InputButton", button];
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
    return this.execute("stop");
  }
}
