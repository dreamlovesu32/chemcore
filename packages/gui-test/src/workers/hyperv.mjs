import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { assertValidDocument } from "../protocol/validate.mjs";

const scriptPath = join(dirname(dirname(dirname(fileURLToPath(import.meta.url)))), "scripts", "hyperv-coordinator.ps1");
const repositoryRoot = dirname(dirname(dirname(dirname(dirname(fileURLToPath(import.meta.url))))));
const defaultAgentPath = join(repositoryRoot, "target", "release", "chemsema-gui-test-agent.exe");

export function expandWindowsEnvironment(template, environment = process.env) {
  return template.replace(/%([^%]+)%/g, (_, name) => {
    const key = Object.keys(environment).find((candidate) => candidate.toLowerCase() === name.toLowerCase());
    if (!key || !environment[key]) {
      throw new Error(`Worker profile references unavailable environment variable %${name}%.`);
    }
    return environment[key];
  });
}

function defaultExecutor(args) {
  return spawnSync("powershell.exe", args, {
    encoding: "utf8",
    windowsHide: true,
    shell: false,
    maxBuffer: 10 * 1024 * 1024,
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

  argumentsFor(operation) {
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
    ];
  }

  async execute(operation) {
    await this.validateProfile();
    return parseResult(this.executor(this.argumentsFor(operation)), operation);
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

  async attestServiceAgent() {
    const result = await this.execute("agent-attest-service");
    const agent = result.agent || {};
    await assertValidDocument(agent, "service agent attestation");
    if (agent.sessionId !== 0 || agent.interactiveReady !== false) {
      throw new Error("Service agent attestation unexpectedly claimed an interactive desktop.");
    }
    return result;
  }

  async stop() {
    return this.execute("stop");
  }
}
