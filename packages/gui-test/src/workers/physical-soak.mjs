#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { closeSync, openSync } from "node:fs";
import { mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { freemem, totalmem } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { verifyDesktopCandidateManifest } from "../../../../scripts/candidate-source-identity.mjs";
import { readValidatedDocument } from "../protocol/validate.mjs";
import { renameAtomicWithRetry, startDaemon, validatePhysicalQueue } from "./physical-daemon.mjs";

const modulePath = fileURLToPath(import.meta.url);
const repositoryRoot = dirname(dirname(dirname(dirname(dirname(modulePath)))));
const delay = (milliseconds) => new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
const now = () => new Date().toISOString();
const terminalDaemonStatuses = new Set(["completed", "paused-failure", "failed-infrastructure", "paused-resource", "stopped"]);

function sha256(bytes) { return createHash("sha256").update(bytes).digest("hex"); }

function parseOptions(args) {
  const options = {};
  const positional = [];
  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (!value.startsWith("--")) { positional.push(value); continue; }
    const next = args[index + 1];
    if (!next || next.startsWith("--")) options[value.slice(2)] = true;
    else { options[value.slice(2)] = next; index += 1; }
  }
  return { positional, options };
}

function processAlive(processId) {
  if (!Number.isInteger(processId) || processId <= 0) return false;
  try { process.kill(processId, 0); return true; } catch { return false; }
}

function git(args) {
  const result = spawnSync("git", args, { cwd: repositoryRoot, encoding: "utf8", shell: false });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`git ${args.join(" ")} failed: ${(result.stderr || result.stdout).trim()}`);
  return result.stdout.trim();
}

function boundedStateRoot(path) {
  const root = resolve(path);
  if (root.length < 12 || relative(repositoryRoot, root) === "" || root === dirname(root)) throw new Error("Soak state root is not a bounded machine-local directory.");
  return root;
}

export function validateSoakTargets({ durationHours, scenarioExecutions }) {
  const hours = Number(durationHours);
  const executions = Number(scenarioExecutions);
  if (!Number.isFinite(hours) || hours <= 0 || hours > 168) throw new Error("Physical soak duration must be greater than 0 and at most 168 hours.");
  if (!Number.isInteger(executions) || executions < 1 || executions > 1_000_000) throw new Error("Physical soak scenario target must be an integer from 1 to 1000000.");
  return { durationHours: hours, scenarioExecutions: executions };
}

async function atomicJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${randomUUID().replaceAll("-", "")}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  try { await renameAtomicWithRetry(temporary, path); }
  catch (error) { await rm(temporary, { force: true }); throw error; }
}

async function readJsonStable(path) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try { return JSON.parse(await readFile(path, "utf8")); }
    catch (error) {
      if (error?.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error;
      await delay(100);
    }
  }
  throw new Error(`Timed out reading stable JSON state ${path}.`);
}

function pathsFor(root) {
  return {
    root,
    state: join(root, "state.json"),
    heartbeat: join(root, "heartbeat.json"),
    checkpoint: join(root, "checkpoint.json"),
    stop: join(root, "stop.request"),
    lease: join(root, "lease"),
    owner: join(root, "lease", "owner.json"),
    cycles: join(root, "cycles"),
    stdout: join(root, "soak.stdout.log"),
    stderr: join(root, "soak.stderr.log"),
  };
}

async function acquireLease(paths, owner) {
  try { await mkdir(paths.lease, { recursive: false }); }
  catch (error) {
    if (error.code !== "EEXIST") throw error;
    let existing;
    try { existing = JSON.parse(await readFile(paths.owner, "utf8")); } catch { /* An unreadable owner is stale only when it has no live PID. */ }
    if (processAlive(existing?.processId)) throw new Error(`Physical soak lease is held by live PID ${existing.processId}.`);
    await rm(paths.lease, { recursive: true, force: true });
    await mkdir(paths.lease, { recursive: false });
  }
  await atomicJson(paths.owner, owner);
}

async function runSoak(options) {
  const profilePath = resolve(String(options.profile || ""));
  const queuePath = resolve(String(options.queue || ""));
  const stateRoot = boundedStateRoot(String(options["state-root"] || ""));
  const targets = validateSoakTargets({ durationHours: options["duration-hours"], scenarioExecutions: options["scenario-executions"] });
  const paths = pathsFor(stateRoot);
  const profile = await readValidatedDocument(profilePath);
  if (profile.kind !== "physical-windows") throw new Error("Physical soak requires a physical-windows worker profile.");
  const queue = validatePhysicalQueue(JSON.parse(await readFile(queuePath, "utf8")));
  await mkdir(paths.cycles, { recursive: true });
  const branch = git(["branch", "--show-current"]);
  const commit = git(["rev-parse", "HEAD"]);
  if (git(["status", "--short"])) throw new Error("Physical soak refuses to bind a candidate to a dirty worktree.");
  const candidate = verifyDesktopCandidateManifest();
  const profileSha256 = sha256(await readFile(profilePath));
  const queueSha256 = sha256(await readFile(queuePath));
  let previousCheckpoint = null;
  let previousState = null;
  try { previousCheckpoint = JSON.parse(await readFile(paths.checkpoint, "utf8")); } catch { /* No trusted checkpoint yet. */ }
  try { previousState = JSON.parse(await readFile(paths.state, "utf8")); } catch { /* No previous supervisor state. */ }
  const matchesBinding = (value) => value?.commit === commit
    && value?.candidateSha256 === candidate.candidateSha256
    && value?.sourceSha256 === candidate.sourceSha256
    && value?.profileSha256 === profileSha256
    && value?.queueSha256 === queueSha256
    && value?.targetDurationHours === targets.durationHours
    && value?.targetScenarioExecutions === targets.scenarioExecutions;
  const trustedCheckpoint = matchesBinding(previousCheckpoint) ? previousCheckpoint : null;
  const trustedState = matchesBinding(previousState) ? previousState : null;
  const startedAt = trustedCheckpoint?.startedAt || trustedState?.startedAt || now();
  const targetEndsAt = trustedCheckpoint?.targetEndsAt || trustedState?.targetEndsAt || new Date(Date.now() + targets.durationHours * 60 * 60 * 1000).toISOString();
  const soakId = trustedCheckpoint?.soakId || trustedState?.soakId || `${queue.id}-soak-${Date.now()}`;
  await acquireLease(paths, { schema: "chemsema.gui.physical-soak-lease.v1", soakId, processId: process.pid, repositoryRoot, commit, candidateSha256: candidate.candidateSha256, acquiredAt: startedAt });

  let status = "running";
  let failure = null;
  let currentCycle = null;
  let completedCycles = trustedCheckpoint?.completedCycles || 0;
  let scenarioExecutions = trustedCheckpoint?.scenarioExecutions || 0;
  const cycleResults = Array.isArray(trustedCheckpoint?.cycleResults) ? [...trustedCheckpoint.cycleResults] : [];
  let recoveryCycle = trustedState?.status === "running" ? trustedState.currentCycle : null;
  let stopping = false;
  let writes = Promise.resolve();
  const snapshot = () => ({
    schema: "chemsema.gui.physical-soak-state.v1",
    soakId, status, processId: process.pid, repositoryRoot, branch, commit,
    candidateSha256: candidate.candidateSha256, sourceSha256: candidate.sourceSha256,
    profilePath, profileSha256, queuePath, queueSha256,
    startedAt, targetEndsAt, targetDurationHours: targets.durationHours,
    targetScenarioExecutions: targets.scenarioExecutions,
    completedCycles, scenarioExecutions, currentCycle,
    heartbeatAt: now(),
    resources: {
      totalMemoryGiB: Math.round(totalmem() / 1024 ** 3 * 100) / 100,
      freeMemoryGiB: Math.round(freemem() / 1024 ** 3 * 100) / 100,
    },
    stopPath: paths.stop, failure, cycleResults,
  });
  const persist = () => {
    const state = snapshot();
    writes = writes.then(() => Promise.all([
      atomicJson(paths.state, state),
      atomicJson(paths.heartbeat, { schema: "chemsema.gui.physical-soak-heartbeat.v1", soakId, processId: process.pid, status, currentCycle, scenarioExecutions, at: state.heartbeatAt, resources: state.resources }),
    ]));
    return writes;
  };
  const heartbeat = setInterval(() => { void persist(); }, 15000);
  process.on("SIGINT", () => { stopping = true; });
  process.on("SIGTERM", () => { stopping = true; });

  try {
    await persist();
    while (Date.now() < Date.parse(targetEndsAt) || scenarioExecutions < targets.scenarioExecutions) {
      if (stopping || await stat(paths.stop).then(() => true, () => false)) { status = "stopped"; break; }
      const cycleNumber = completedCycles + 1;
      const cycleRoot = join(paths.cycles, String(cycleNumber).padStart(6, "0"));
      let childState = null;
      if (recoveryCycle?.cycleNumber === cycleNumber && recoveryCycle.statePath === join(cycleRoot, "state.json")) {
        try { childState = await readJsonStable(recoveryCycle.statePath); } catch { /* Restart the child from its own checkpoint. */ }
        if (!childState || (!terminalDaemonStatuses.has(childState.status) && !processAlive(childState.processId))) childState = await startDaemon({ profile: profilePath, queue: queuePath, "state-root": cycleRoot });
      } else {
        childState = await startDaemon({ profile: profilePath, queue: queuePath, "state-root": cycleRoot });
      }
      recoveryCycle = null;
      if (childState.commit !== commit || childState.candidateSha256 !== candidate.candidateSha256 || childState.sourceSha256 !== candidate.sourceSha256 || childState.profileSha256 !== profileSha256 || childState.queueSha256 !== queueSha256) {
        throw new Error("Physical soak child binding does not match the immutable soak candidate.");
      }
      currentCycle = { cycleNumber, batchId: childState.batchId, statePath: join(cycleRoot, "state.json"), processId: childState.processId, startedAt: now() };
      await persist();
      let terminal;
      while (!terminal) {
        if (stopping || await stat(paths.stop).then(() => true, () => false)) await writeFile(join(cycleRoot, "stop.request"), `${now()}\n`, "utf8");
        const state = await readJsonStable(currentCycle.statePath);
        if (terminalDaemonStatuses.has(state.status)) terminal = state;
        else await delay(5000);
      }
      const receipt = {
        cycleNumber, batchId: terminal.batchId, status: terminal.status,
        statePath: currentCycle.statePath, candidateSha256: terminal.candidateSha256,
        commit: terminal.commit, completedScenarioIds: terminal.completedScenarioIds,
        results: terminal.results, endedAt: terminal.heartbeatAt,
      };
      cycleResults.push(receipt);
      currentCycle = null;
      if (stopping || await stat(paths.stop).then(() => true, () => false)) {
        status = "stopped";
        await persist();
        break;
      }
      if (terminal.status !== "completed" || terminal.remaining !== 0 || terminal.failure || terminal.results.some((result) => result.status !== "passed")) {
        status = terminal.status === "paused-resource" ? "paused-resource" : "paused-failure";
        failure = { message: `Physical soak cycle ${cycleNumber} entered ${terminal.status}.`, receipt, childFailure: terminal.failure };
        break;
      }
      completedCycles += 1;
      scenarioExecutions += terminal.results.length;
      await atomicJson(paths.checkpoint, {
        schema: "chemsema.gui.physical-soak-checkpoint.v1", soakId, commit,
        candidateSha256: candidate.candidateSha256, sourceSha256: candidate.sourceSha256,
        profileSha256, queueSha256, targetDurationHours: targets.durationHours,
        targetScenarioExecutions: targets.scenarioExecutions, startedAt, targetEndsAt,
        completedCycles, scenarioExecutions, cycleResults,
        lastCycle: receipt, updatedAt: now(),
      });
      await persist();
    }
    if (status === "running") status = "completed";
  } catch (error) {
    status = "failed-infrastructure";
    failure = { name: error.name, message: error.message, stack: error.stack || null };
    process.exitCode = 1;
  } finally {
    currentCycle = null;
    clearInterval(heartbeat);
    await persist();
    await writes;
    await rm(paths.lease, { recursive: true, force: true });
  }
}

async function startSoak(options) {
  const stateRoot = boundedStateRoot(String(options["state-root"] || ""));
  const paths = pathsFor(stateRoot);
  await mkdir(stateRoot, { recursive: true });
  let existing;
  try { existing = JSON.parse(await readFile(paths.state, "utf8")); } catch { /* No previous state. */ }
  if (existing?.status === "running" && processAlive(existing.processId)) throw new Error(`Physical soak is already running as PID ${existing.processId}.`);
  await rm(paths.stop, { force: true });
  const stdout = openSync(paths.stdout, "a");
  const stderr = openSync(paths.stderr, "a");
  const args = [modulePath, "run", "--profile", resolve(String(options.profile || "")), "--queue", resolve(String(options.queue || "")), "--state-root", stateRoot, "--duration-hours", String(options["duration-hours"]), "--scenario-executions", String(options["scenario-executions"])];
  const child = spawn(process.execPath, args, { cwd: repositoryRoot, detached: true, windowsHide: true, shell: false, stdio: ["ignore", stdout, stderr] });
  closeSync(stdout); closeSync(stderr); child.unref();
  const deadline = Date.now() + 15000;
  do {
    try {
      const state = JSON.parse(await readFile(paths.state, "utf8"));
      if (state.processId === child.pid && state.status === "running") return state;
      if (state.processId === child.pid && state.status !== "running") throw new Error(`Physical soak entered ${state.status}: ${state.failure?.message || "no detail"}`);
    } catch (error) { if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error; }
    await delay(50);
  } while (Date.now() < deadline);
  throw new Error("Physical soak did not publish a running heartbeat within 15 seconds.");
}

async function main() {
  const { positional, options } = parseOptions(process.argv.slice(2));
  const command = positional[0];
  if (command === "run") return runSoak(options);
  const stateRoot = boundedStateRoot(String(options["state-root"] || ""));
  const paths = pathsFor(stateRoot);
  if (command === "start") console.log(JSON.stringify(await startSoak(options), null, 2));
  else if (command === "status") console.log(await readFile(paths.state, "utf8"));
  else if (command === "stop") {
    await mkdir(stateRoot, { recursive: true });
    await writeFile(paths.stop, `${now()}\n`, "utf8");
    console.log(JSON.stringify({ schema: "chemsema.gui.physical-soak-stop.v1", requestedAt: now(), stateRoot }, null, 2));
  } else throw new Error("usage: physical-soak <start|run|status|stop> --profile path --queue path --state-root path --duration-hours hours --scenario-executions count");
}

if (resolve(process.argv[1] || "") === resolve(modulePath)) main().catch((error) => { console.error(error.stack || error.message); process.exitCode = 1; });
