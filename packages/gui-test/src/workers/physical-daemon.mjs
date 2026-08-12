#!/usr/bin/env node
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { closeSync, openSync } from "node:fs";
import { mkdir, open, readFile, readdir, rename, rm, stat, unlink, writeFile } from "node:fs/promises";
import { freemem, totalmem } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { guiTestsDir, repositoryRoot, scenarioDir } from "../protocol/paths.mjs";
import { readValidatedDocument } from "../protocol/validate.mjs";
import { expandWindowsEnvironment } from "./hyperv.mjs";
import { PhysicalWindowsCoordinator } from "./physical-windows.mjs";
import { verifyDesktopCandidateManifest } from "../../../../scripts/candidate-source-identity.mjs";

const sourcePath = fileURLToPath(import.meta.url);
const cliPath = join(dirname(dirname(sourcePath)), "cli.mjs");

function parseOptions(args) {
  const positional = [];
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (!value.startsWith("--")) { positional.push(value); continue; }
    const key = value.slice(2);
    const next = args[index + 1];
    if (!next || next.startsWith("--")) options[key] = true;
    else { options[key] = next; index += 1; }
  }
  return { positional, options };
}

async function atomicJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${process.pid}.${Date.now()}.${Math.random().toString(16).slice(2)}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  await rename(temporary, path);
}

async function sha256File(path) {
  const hash = createHash("sha256");
  hash.update(await readFile(path));
  return hash.digest("hex");
}

function isLiveProcess(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; } catch { return false; }
}

async function readJson(path) {
  try { return JSON.parse(await readFile(path, "utf8")); } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

async function scenarioFiles(directory = scenarioDir) {
  const entries = await readdir(directory, { withFileTypes: true });
  const result = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...await scenarioFiles(path));
    else if (entry.isFile() && entry.name.endsWith(".json")) result.push(path);
  }
  return result.sort();
}

async function physicalProductionScenarios() {
  const result = [];
  for (const path of await scenarioFiles()) {
    const scenario = await readValidatedDocument(path);
    if (scenario.drivers.includes("production-black-box")) result.push({ id: scenario.id, path });
  }
  return result;
}

async function command(file, args, options = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(file, args, { cwd: repositoryRoot, windowsHide: true, shell: false, ...options });
    let stdout = "";
    let stderr = "";
    child.stdout?.on("data", (chunk) => { stdout = `${stdout}${chunk}`.slice(-1024 * 1024); });
    child.stderr?.on("data", (chunk) => { stderr = `${stderr}${chunk}`.slice(-1024 * 1024); });
    child.on("error", reject);
    child.on("close", (code) => resolvePromise({ code: code ?? 1, stdout, stderr, pid: child.pid }));
  });
}

async function repositoryIdentity() {
  const [head, branch, status] = await Promise.all([
    command("git", ["rev-parse", "HEAD"]),
    command("git", ["branch", "--show-current"]),
    command("git", ["status", "--porcelain=v1"]),
  ]);
  if (head.code || branch.code || status.code) throw new Error("Cannot bind the physical daemon to the repository identity.");
  const manifestPath = join(repositoryRoot, "target", "release", "chemsema-desktop.build-manifest.json");
  return {
    repositoryRoot,
    head: head.stdout.trim(),
    branch: branch.stdout.trim(),
    dirty: Boolean(status.stdout.trim()),
    dirtyPaths: status.stdout.split(/\r?\n/).filter(Boolean).slice(0, 512),
    candidateManifestPath: manifestPath,
    candidateManifestSha256: await sha256File(manifestPath),
    candidate: await readJson(manifestPath),
  };
}

async function daemonContext(profilePath) {
  const resolvedProfile = resolve(profilePath || join(guiTestsDir, "environments", "windows-physical-gui-worker-current.json"));
  const profile = await readValidatedDocument(resolvedProfile);
  if (profile.kind !== "physical-windows") throw new Error("Physical daemon requires a physical-windows worker profile.");
  const stateRoot = expandWindowsEnvironment(profile.physical.stateRootTemplate);
  const daemonRoot = join(stateRoot, "daemon");
  return {
    profile,
    profilePath: resolvedProfile,
    stateRoot,
    daemonRoot,
    statePath: join(daemonRoot, "state.json"),
    checkpointPath: join(daemonRoot, "checkpoint.json"),
    lockPath: join(daemonRoot, "daemon.lock"),
    stopPath: join(daemonRoot, "stop.request"),
    launchPath: join(daemonRoot, "launch.json"),
    evidenceRoot: join(stateRoot, "evidence"),
    reportsRoot: join(stateRoot, "reports"),
  };
}

async function acquireLock(context) {
  await mkdir(context.daemonRoot, { recursive: true });
  try {
    const handle = await open(context.lockPath, "wx");
    await handle.writeFile(`${JSON.stringify({ schema: "chemsema.gui.physical-daemon-lock.v1", pid: process.pid, acquiredAt: new Date().toISOString() })}\n`, "utf8");
    return handle;
  } catch (error) {
    if (error.code !== "EEXIST") throw error;
    const lock = await readJson(context.lockPath);
    if (isLiveProcess(lock?.pid)) throw new Error(`Physical daemon is already running as PID ${lock.pid}.`);
    await unlink(context.lockPath);
    return acquireLock(context);
  }
}

function resourceSnapshot() {
  const free = freemem();
  const total = totalmem();
  return {
    capturedAt: new Date().toISOString(),
    totalMemoryGiB: Math.round(total / 1024 ** 3 * 1000) / 1000,
    availableMemoryGiB: Math.round(free / 1024 ** 3 * 1000) / 1000,
    availablePercent: Math.round(free / total * 10000) / 100,
  };
}

async function waitForResourceReserve(context, state) {
  while (true) {
    const resources = resourceSnapshot();
    if (resources.availableMemoryGiB >= context.profile.resources.minimumAvailableMemoryGiB) return resources;
    state.status = "resource-paused";
    state.resources = resources;
    state.message = `Waiting for ${context.profile.resources.minimumAvailableMemoryGiB} GiB available memory safety reserve.`;
    state.heartbeatAt = new Date().toISOString();
    await atomicJson(context.statePath, state);
    if (await stat(context.stopPath).then(() => true, () => false)) return null;
    await new Promise((resolvePromise) => setTimeout(resolvePromise, context.profile.resources.pollIntervalMs));
  }
}

function safeScenarioName(id) { return id.replaceAll(/[^a-z0-9._-]/g, "_"); }

export async function runDaemon({ profilePath, selectedScenarioIds = [] } = {}) {
  const context = await daemonContext(profilePath);
  const all = await physicalProductionScenarios();
  const selected = selectedScenarioIds.length ? all.filter((entry) => selectedScenarioIds.includes(entry.id)) : all;
  const missing = selectedScenarioIds.filter((id) => !selected.some((entry) => entry.id === id));
  if (missing.length) throw new Error(`Unknown production scenarios: ${missing.join(", ")}.`);
  const repository = await repositoryIdentity();
  if (repository.dirty) throw new Error("Physical daemon refuses to bind a production queue to a dirty repository.");
  verifyDesktopCandidateManifest();
  const lock = await acquireLock(context);
  await rm(context.stopPath, { force: true });
  const state = {
    schema: "chemsema.gui.physical-daemon-state.v1",
    workerId: context.profile.id,
    daemonPid: process.pid,
    status: "starting",
    startedAt: new Date().toISOString(),
    heartbeatAt: new Date().toISOString(),
    repository,
    queue: selected.map((entry) => entry.id),
    completed: [],
    failed: [],
    current: null,
    resources: resourceSnapshot(),
    lastExitCode: null,
  };
  let stateWrite = Promise.resolve();
  const writeState = () => {
    stateWrite = stateWrite.then(() => atomicJson(context.statePath, state));
    return stateWrite;
  };
  const heartbeat = setInterval(() => {
    state.heartbeatAt = new Date().toISOString();
    state.resources = resourceSnapshot();
    stateWrite = stateWrite.then(() => atomicJson(context.statePath, state)).catch(() => {});
  }, 5000);
  try {
    await writeState();
    for (let index = 0; index < selected.length; index += 1) {
      if (await stat(context.stopPath).then(() => true, () => false)) { state.status = "stopped-by-request"; break; }
      const resources = await waitForResourceReserve(context, state);
      if (!resources) { state.status = "stopped-by-request"; break; }
      const scenario = selected[index];
      const runRoot = join(context.reportsRoot, `${new Date().toISOString().replaceAll(/[:.]/g, "-")}-${safeScenarioName(scenario.id)}`);
      const reportPath = join(runRoot, "run-report.json");
      await mkdir(runRoot, { recursive: true });
      state.status = "running";
      state.current = { scenarioId: scenario.id, scenarioPath: scenario.path, index, startedAt: new Date().toISOString(), reportPath, runRoot };
      state.resources = resources;
      state.message = null;
      await writeState();
      const child = spawn(process.execPath, [
        cliPath, "run", scenario.path,
        "--driver", "production-black-box",
        "--profile", context.profilePath,
        "--report", reportPath,
        "--evidence-root", context.evidenceRoot,
      ], {
        cwd: repositoryRoot,
        windowsHide: true,
        shell: false,
        env: { ...process.env, CHEMSEMA_GUI_LEASE_OWNER_PID: String(process.pid) },
        stdio: ["ignore", "pipe", "pipe"],
      });
      state.current.processId = child.pid;
      await writeState();
      let stdout = "";
      let stderr = "";
      child.stdout.on("data", (chunk) => { stdout = `${stdout}${chunk}`.slice(-1024 * 1024); });
      child.stderr.on("data", (chunk) => { stderr = `${stderr}${chunk}`.slice(-1024 * 1024); });
      const exitCode = await new Promise((resolvePromise, reject) => { child.on("error", reject); child.on("close", (code) => resolvePromise(code ?? 1)); });
      const report = await readJson(reportPath);
      const outcome = {
        scenarioId: scenario.id,
        reportPath,
        runId: report?.runId || null,
        status: report?.status || "missing-report",
        exitCode,
        endedAt: new Date().toISOString(),
        evidenceKey: report?.evidenceKey || null,
        candidateSha256: report?.environment?.candidateSha256 || null,
        stdoutTail: stdout.slice(-65536),
        stderrTail: stderr.slice(-65536),
      };
      state.lastExitCode = exitCode;
      state.current = null;
      if (await stat(context.stopPath).then(() => true, () => false)) {
        state.status = "stopped-by-request";
        state.message = `Scenario ${scenario.id} was stopped by an explicit request.`;
        await writeState();
        break;
      }
      if (exitCode === 0 && report?.status === "passed") {
        state.completed.push(outcome);
        await atomicJson(context.checkpointPath, { schema: "chemsema.gui.physical-daemon-checkpoint.v1", workerId: context.profile.id, repository, queue: state.queue, completed: state.completed, failed: state.failed, updatedAt: new Date().toISOString() });
      } else {
        state.failed.push(outcome);
        state.status = "paused-failure";
        state.message = `Scenario ${scenario.id} failed; queue paused without retry.`;
        await writeState();
        break;
      }
    }
    if (state.status === "running" || state.status === "starting" || state.status === "resource-paused") {
      state.status = state.failed.length ? "paused-failure" : "completed";
    }
    state.endedAt = new Date().toISOString();
    state.heartbeatAt = state.endedAt;
    await writeState();
    return state;
  } finally {
    clearInterval(heartbeat);
    await stateWrite.catch(() => {});
    await lock.close();
    await rm(context.lockPath, { force: true });
  }
}

export async function startDaemon({ profilePath, selectedScenarioIds = [] } = {}) {
  const context = await daemonContext(profilePath);
  const repository = await repositoryIdentity();
  if (repository.dirty) throw new Error("Physical daemon refuses to start from a dirty repository.");
  verifyDesktopCandidateManifest();
  const existing = await readJson(context.statePath);
  if (isLiveProcess(existing?.daemonPid) && ["starting", "running", "resource-paused"].includes(existing.status)) {
    throw new Error(`Physical daemon is already active as PID ${existing.daemonPid}.`);
  }
  await mkdir(context.daemonRoot, { recursive: true });
  await rm(context.stopPath, { force: true });
  const argumentsForDaemon = [sourcePath, "run", "--profile", context.profilePath];
  if (selectedScenarioIds.length) argumentsForDaemon.push("--scenarios", selectedScenarioIds.join(","));
  const outPath = join(context.daemonRoot, "daemon.stdout.log");
  const errorPath = join(context.daemonRoot, "daemon.stderr.log");
  const stdout = openSync(outPath, "a");
  const stderr = openSync(errorPath, "a");
  const child = spawn(process.execPath, argumentsForDaemon, { cwd: repositoryRoot, detached: true, windowsHide: true, shell: false, stdio: ["ignore", stdout, stderr] });
  closeSync(stdout);
  closeSync(stderr);
  child.unref();
  const launch = { schema: "chemsema.gui.physical-daemon-launch.v1", pid: child.pid, launchedAt: new Date().toISOString(), profilePath: context.profilePath, selectedScenarioIds, stdout: outPath, stderr: errorPath };
  await atomicJson(context.launchPath, launch);
  return launch;
}

export async function daemonStatus({ profilePath } = {}) {
  const context = await daemonContext(profilePath);
  const state = await readJson(context.statePath);
  const checkpoint = await readJson(context.checkpointPath);
  return { schema: "chemsema.gui.physical-daemon-status.v1", state, checkpoint, processLive: isLiveProcess(state?.daemonPid), stopRequested: await stat(context.stopPath).then(() => true, () => false) };
}

export async function stopDaemon({ profilePath } = {}) {
  const context = await daemonContext(profilePath);
  await mkdir(context.daemonRoot, { recursive: true });
  await writeFile(context.stopPath, `${JSON.stringify({ schema: "chemsema.gui.physical-daemon-stop.v1", requestedAt: new Date().toISOString() })}\n`, "utf8");
  const state = await readJson(context.statePath);
  if (isLiveProcess(state?.current?.processId)) process.kill(state.current.processId);
  const coordinator = new PhysicalWindowsCoordinator(context.profile);
  await coordinator.stop();
  return daemonStatus({ profilePath });
}

async function main() {
  const [operation, ...raw] = process.argv.slice(2);
  const { options } = parseOptions(raw);
  const selectedScenarioIds = String(options.scenarios || "").split(",").map((value) => value.trim()).filter(Boolean);
  let result;
  if (operation === "run") result = await runDaemon({ profilePath: options.profile, selectedScenarioIds });
  else if (operation === "start") result = await startDaemon({ profilePath: options.profile, selectedScenarioIds });
  else if (operation === "status") result = await daemonStatus({ profilePath: options.profile });
  else if (operation === "stop") result = await stopDaemon({ profilePath: options.profile });
  else throw new Error("Usage: physical-daemon <start|run|status|stop> [--profile path] [--scenarios id,id]");
  console.log(JSON.stringify(result, null, 2));
}

if (process.argv[1] && resolve(process.argv[1]) === sourcePath) {
  main().catch((error) => { console.error(`[physical-daemon] ${error.stack || error.message}`); process.exitCode = 1; });
}
