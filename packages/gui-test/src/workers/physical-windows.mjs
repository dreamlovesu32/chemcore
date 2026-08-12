import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { assertValidDocument } from "../protocol/validate.mjs";
import { verifyDesktopCandidateManifest } from "../../../../scripts/candidate-source-identity.mjs";
import { expandWindowsEnvironment, HyperVCoordinator } from "./hyperv.mjs";

const packageRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const repositoryRoot = dirname(dirname(packageRoot));
const coordinatorScriptPath = join(packageRoot, "scripts", "physical-windows-coordinator.ps1");
const defaultAgentPath = join(repositoryRoot, "target", "release", "chemsema-gui-test-agent.exe");
const defaultCandidatePath = join(repositoryRoot, "target", "release", "chemsema-desktop.exe");
const defaultCdpScriptPath = join(packageRoot, "scripts", "guest-cdp.ps1");

function executePowerShell(args, { timeoutMs = 120000 } = {}) {
  return new Promise((resolve) => {
    const child = spawn("powershell.exe", args, { windowsHide: true, shell: false });
    let stdout = "";
    let stderr = "";
    let failure = null;
    const append = (field, chunk) => {
      if (Buffer.byteLength(field + chunk, "utf8") > 10 * 1024 * 1024) {
        failure = new Error("Physical Windows coordinator output exceeded 10 MiB.");
        child.kill();
        return field;
      }
      return field + chunk;
    };
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk.toString("utf8")); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk.toString("utf8")); });
    child.on("error", (error) => { failure = error; });
    const timer = setTimeout(() => {
      failure = new Error(`Physical Windows coordinator exceeded ${timeoutMs} ms.`);
      child.kill();
    }, timeoutMs);
    child.on("close", (status) => {
      clearTimeout(timer);
      resolve({ status: status ?? 1, stdout, stderr, error: failure });
    });
  });
}

function parseResult(result, operation) {
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`Physical Windows ${operation} failed: ${(result.stderr || result.stdout || "unknown PowerShell error").trim()}`);
  }
  const line = String(result.stdout || "").split(/\r?\n/).map((value) => value.trim()).filter(Boolean)
    .reverse().find((value) => value.startsWith("{") && value.endsWith("}"));
  if (!line) throw new Error(`Physical Windows ${operation} returned no structured attestation.`);
  return JSON.parse(line);
}

function cleanAgent(agent = {}) {
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

export class PhysicalWindowsCoordinator extends HyperVCoordinator {
  constructor(profile, {
    executor = executePowerShell,
    environment = process.env,
    candidateVerifier = executor === executePowerShell ? verifyDesktopCandidateManifest : () => null,
  } = {}) {
    super(profile, { executor, environment, candidateVerifier });
    this.atomicActionTransactions = false;
    this.account = expandWindowsEnvironment(profile.physical.accountTemplate, environment);
    this.testRoot = expandWindowsEnvironment(profile.physical.testRootTemplate, environment);
    this.stateRoot = expandWindowsEnvironment(profile.physical.stateRootTemplate, environment);
  }

  async validateProfile() {
    await assertValidDocument(this.profile, this.profile.id || "physical worker profile");
    if (this.profile.kind !== "physical-windows" || this.profile.resources.mode !== "adaptive") {
      throw new Error("Physical Windows coordinator requires an adaptive physical-windows profile.");
    }
    return this.profile;
  }

  argumentsFor(operation, extraArguments = []) {
    const leaseOwnerPid = Number.parseInt(this.environment.CHEMSEMA_GUI_LEASE_OWNER_PID || "", 10);
    return [
      "-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
      "-File", coordinatorScriptPath,
      "-Operation", operation,
      "-WorkerId", this.profile.id,
      "-ExpectedAccount", this.account,
      "-TestRoot", this.testRoot,
      "-StateRoot", this.stateRoot,
      "-CoordinatorPid", String(Number.isInteger(leaseOwnerPid) && leaseOwnerPid > 0 ? leaseOwnerPid : process.pid),
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
    const resources = result.resources || {};
    const failures = [];
    if (result.host?.platform !== "windows-physical") failures.push("host platform is not physical Windows");
    if (String(result.host?.account || "").toLowerCase() !== this.account.toLowerCase()) failures.push("current account does not match the profile");
    if (!result.host?.interactiveSession || !result.host?.explorerInSession) failures.push("current account has no interactive Explorer session");
    if (resources.availableMemoryGiB < this.profile.resources.minimumAvailableMemoryGiB) failures.push("available memory is below the adaptive safety reserve");
    if (resources.commitPercent > this.profile.resources.maximumCommitPercent) failures.push("system commit is above the adaptive safety threshold");
    if (failures.length) throw new Error(`Physical host attestation failed closed: ${failures.join("; ")}.`);
    return result;
  }

  async reset() {
    await this.attestHost();
    const result = await this.execute("reset");
    if (result.state !== "ready" || result.scope !== "test-owned-processes-only") {
      throw new Error("Physical worker reset returned an invalid bounded receipt.");
    }
    return result;
  }

  async start() {
    await this.attestHost();
    const result = await this.execute("start");
    if (!result.lease?.owned || !result.keepAwake?.running) throw new Error("Physical worker did not acquire its lease and keep-awake process.");
    return result;
  }

  async attestGuest({ requireInteractive = false } = {}) {
    const result = await this.execute("guest-attest");
    const guest = result.guest || {};
    const failures = [];
    if (String(guest.identity || "").toLowerCase() !== this.account.toLowerCase()) failures.push("physical worker account does not match the profile");
    if (requireInteractive && !guest.interactiveAccountMatches) failures.push("physical interactive desktop is not active and unlocked");
    if (failures.length) throw new Error(`Physical guest attestation failed closed: ${failures.join("; ")}.`);
    return result;
  }

  async prepareGuest() {
    await this.attestGuest();
    return this.execute("prepare-guest");
  }

  async configureAutologon() {
    throw new Error("Physical Windows workers never configure automatic logon.");
  }

  async configureDesktopBaseline() {
    const result = await this.execute("configure-desktop-baseline");
    if (result.baseline?.scope !== "current-physical-account" || result.baseline?.changed !== false || !result.baseline?.keepAwakeRunning) {
      throw new Error("Physical desktop baseline failed its non-mutating keep-awake contract.");
    }
    return result;
  }

  async startCdpAgent() {
    const result = await this.execute("start-cdp-agent", [], { timeoutMs: 30000 });
    await assertValidDocument(result.agent, "physical CDP agent readiness");
    if (result.agent?.schema !== "chemsema.gui.cdp-server.v1" || result.agent?.status !== "ready" || result.agent?.port !== 9223
      || result.agent?.sessionId === 0 || String(result.agent?.account || "").toLowerCase() !== this.account.toLowerCase()) {
      throw new Error("Physical persistent CDP agent returned an invalid readiness receipt.");
    }
    return result;
  }

  async attestInteractiveAgent() {
    const result = await this.execute("agent-attest-interactive");
    const agent = cleanAgent(result.agent);
    await assertValidDocument(agent, "physical interactive agent attestation");
    const failures = [];
    if (String(agent.account || "").toLowerCase() !== this.account.toLowerCase()) failures.push("agent account does not match the physical profile");
    if (agent.sessionId === 0 || !agent.interactiveReady || agent.inputDesktop !== "Default") failures.push("agent is not attached to an unlocked Default desktop");
    if (!agent.foreground || agent.foreground.sessionId !== agent.sessionId) failures.push("foreground window is absent or belongs to another session");
    if (failures.length) throw new Error(`Physical interactive agent attestation failed closed: ${failures.join("; ")}.`);
    return { ...result, agent };
  }

  async attestServiceAgent() {
    throw new Error("Physical Windows workers do not impersonate a service-session input agent.");
  }

  async stop() {
    return this.execute("stop", [], { timeoutMs: 30000 });
  }
}
