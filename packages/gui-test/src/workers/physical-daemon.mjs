#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { closeSync, openSync } from "node:fs";
import { mkdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { freemem, loadavg, totalmem } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { verifyDesktopCandidateManifest } from "../../../../scripts/candidate-source-identity.mjs";
import { readValidatedDocument } from "../protocol/validate.mjs";

const modulePath = fileURLToPath(import.meta.url);
const repositoryRoot = dirname(dirname(dirname(dirname(dirname(modulePath)))));
const guiCliPath = join(repositoryRoot, "packages", "gui-test", "src", "cli.mjs");
const delay = (milliseconds) => new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
const now = () => new Date().toISOString();

function sha256(bytes) { return createHash("sha256").update(bytes).digest("hex"); }

export async function readScenarioReport(path, { read = readFile } = {}) {
  try {
    return await read(path);
  } catch (error) {
    if (error?.code === "ENOENT") return null;
    throw error;
  }
}

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

export async function renameAtomicWithRetry(source, destination, { renameFile = rename, wait = delay, attempts = 12 } = {}) {
  for (let attempt = 0; ; attempt += 1) {
    try {
      await renameFile(source, destination);
      return;
    } catch (error) {
      const transient = ["EPERM", "EACCES", "EBUSY"].includes(error?.code);
      if (!transient || attempt + 1 >= attempts) throw error;
      await wait(Math.min(250, 25 * (2 ** attempt)));
    }
  }
}

async function atomicJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${randomUUID().replaceAll("-", "")}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  try {
    await renameAtomicWithRetry(temporary, path);
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

function boundedStateRoot(path) {
  const root = resolve(path);
  if (root.length < 12 || relative(repositoryRoot, root) === "" || root === dirname(root)) throw new Error("Daemon state root is not a bounded machine-local directory.");
  return root;
}

export function validatePhysicalQueue(queue, root = repositoryRoot) {
  if (queue?.schema !== "chemsema.gui.physical-queue.v1" || !/^[a-z0-9][a-z0-9._-]{0,127}$/.test(queue.id || "")) throw new Error("Physical queue identity is invalid.");
  if (!Array.isArray(queue.scenarios) || queue.scenarios.length < 1 || queue.scenarios.length > 1000) throw new Error("Physical queue must contain 1 to 1000 scenarios.");
  const seen = new Set();
  const scenarioRoot = `${resolve(root, "tests", "gui", "scenarios")}\\`.toLowerCase();
  for (const entry of queue.scenarios) {
    if (typeof entry !== "string" || isAbsolute(entry) || entry.includes("..") || !entry.replaceAll("/", "\\").toLowerCase().startsWith("tests\\gui\\scenarios\\") || !entry.endsWith(".json")) throw new Error(`Physical queue scenario path ${JSON.stringify(entry)} is invalid.`);
    const resolved = resolve(root, entry);
    if (!resolved.toLowerCase().startsWith(scenarioRoot) || seen.has(resolved.toLowerCase())) throw new Error("Physical queue contains an escaped or duplicate scenario.");
    seen.add(resolved.toLowerCase());
  }
  return queue;
}

function git(args) {
  const result = spawnSync("git", args, { cwd: repositoryRoot, encoding: "utf8", shell: false });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`git ${args.join(" ")} failed: ${(result.stderr || result.stdout).trim()}`);
  return result.stdout.trim();
}

async function acquireLease(paths, owner) {
  try {
    await mkdir(paths.lease, { recursive: false });
  } catch (error) {
    if (error.code !== "EEXIST") throw error;
    let existing;
    try { existing = JSON.parse(await readFile(paths.owner, "utf8")); } catch { /* Invalid lease is stale only when it has no live owner. */ }
    if (processAlive(existing?.processId)) throw new Error(`Physical daemon lease is held by live PID ${existing.processId}.`);
    await rm(paths.lease, { recursive: true, force: true });
    await mkdir(paths.lease, { recursive: false });
  }
  await atomicJson(paths.owner, owner);
}

function statePaths(stateRoot) {
  return {
    root: stateRoot,
    state: join(stateRoot, "state.json"),
    heartbeat: join(stateRoot, "heartbeat.json"),
    checkpoint: join(stateRoot, "checkpoint.json"),
    stop: join(stateRoot, "stop.request"),
    lease: join(stateRoot, "lease"),
    owner: join(stateRoot, "lease", "owner.json"),
    stdout: join(stateRoot, "daemon.stdout.log"),
    stderr: join(stateRoot, "daemon.stderr.log"),
    reports: join(stateRoot, "reports"),
    evidence: join(stateRoot, "evidence"),
  };
}

async function runDaemon(options) {
  const profilePath = resolve(String(options.profile || ""));
  const queuePath = resolve(String(options.queue || ""));
  const stateRoot = boundedStateRoot(String(options["state-root"] || ""));
  const paths = statePaths(stateRoot);
  const profile = await readValidatedDocument(profilePath);
  if (profile.kind !== "physical-windows") throw new Error("Physical daemon requires a physical-windows worker profile.");
  const queue = validatePhysicalQueue(JSON.parse(await readFile(queuePath, "utf8")));
  await mkdir(paths.reports, { recursive: true });
  await mkdir(paths.evidence, { recursive: true });
  const branch = git(["branch", "--show-current"]);
  const commit = git(["rev-parse", "HEAD"]);
  const dirty = git(["status", "--short"]);
  if (dirty) throw new Error("Physical daemon refuses to bind a candidate to a dirty worktree.");
  const candidate = verifyDesktopCandidateManifest();
  const profileHash = sha256(await readFile(profilePath));
  const queueHash = sha256(await readFile(queuePath));
  const batchId = `${queue.id}-${Date.now()}`;
  const owner = { schema: "chemsema.gui.physical-daemon-lease.v1", processId: process.pid, batchId, acquiredAt: now(), repositoryRoot, commit, candidateSha256: candidate.candidateSha256 };
  await acquireLease(paths, owner);

  let previous = null;
  try { previous = JSON.parse(await readFile(paths.checkpoint, "utf8")); } catch { /* No trusted checkpoint yet. */ }
  const bindingMatches = previous?.commit === commit && previous?.candidateSha256 === candidate.candidateSha256 && previous?.queueSha256 === queueHash && previous?.profileSha256 === profileHash;
  const completed = bindingMatches && Array.isArray(previous.completedScenarioIds) ? [...previous.completedScenarioIds] : [];
  const results = bindingMatches && Array.isArray(previous.results) ? [...previous.results] : [];
  let currentChild = null;
  let stopping = false;
  let status = "running";
  let failure = null;
  let currentScenario = null;
  let writes = Promise.resolve();
  const snapshot = () => ({
    schema: "chemsema.gui.physical-daemon-state.v1",
    batchId, status, processId: process.pid, childProcessId: currentChild?.pid || null,
    repositoryRoot, branch, commit, candidateSha256: candidate.candidateSha256, sourceSha256: candidate.sourceSha256,
    profilePath, profileSha256: profileHash, queuePath, queueSha256: queueHash,
    currentScenario, completedScenarioIds: completed, remaining: queue.scenarios.length - completed.length,
    evidenceRoot: paths.evidence, reportsRoot: paths.reports, heartbeatAt: now(),
    resources: { totalMemoryGiB: Math.round(totalmem() / 1024 ** 3 * 100) / 100, freeMemoryGiB: Math.round(freemem() / 1024 ** 3 * 100) / 100, loadAverage: loadavg() },
    processList: [process.pid, ...(currentChild?.pid ? [currentChild.pid] : [])],
    failure, results,
  });
  const persist = () => {
    const state = snapshot();
    writes = writes.then(() => Promise.all([atomicJson(paths.state, state), atomicJson(paths.heartbeat, { schema: "chemsema.gui.physical-daemon-heartbeat.v1", batchId, processId: process.pid, status, currentScenario, at: state.heartbeatAt, resources: state.resources })]));
    return writes;
  };
  const heartbeat = setInterval(() => { void persist(); }, 15000);
  process.on("SIGINT", () => { stopping = true; });
  process.on("SIGTERM", () => { stopping = true; });

  try {
    await persist();
    for (const scenarioPath of queue.scenarios) {
      const scenario = await readValidatedDocument(resolve(repositoryRoot, scenarioPath));
      if (completed.includes(scenario.id)) continue;
      if (stopping || await stat(paths.stop).then(() => true, () => false)) { status = "stopped"; break; }
      if (freemem() < 768 * 1024 ** 2) { status = "paused-resource"; failure = { message: "Free physical memory fell below the 768 MiB safety floor." }; break; }
      currentScenario = scenario.id;
      const safeId = scenario.id.replaceAll(/[^A-Za-z0-9._-]/g, "_");
      const reportPath = join(paths.reports, `${String(completed.length + 1).padStart(4, "0")}-${safeId}.json`);
      await persist();
      const stdout = openSync(paths.stdout, "a");
      const stderr = openSync(paths.stderr, "a");
      currentChild = spawn(process.execPath, [guiCliPath, "run", resolve(repositoryRoot, scenarioPath), "--driver", "production-black-box", "--worker-profile", profilePath, "--report", reportPath, "--evidence-root", paths.evidence], { cwd: repositoryRoot, windowsHide: true, shell: false, stdio: ["ignore", stdout, stderr] });
      closeSync(stdout); closeSync(stderr);
      await persist();
      const exitCode = await new Promise((resolveExit, reject) => { currentChild.once("error", reject); currentChild.once("close", (code) => resolveExit(code ?? 1)); });
      currentChild = null;
      const reportBytes = await readScenarioReport(reportPath);
      if (!reportBytes) {
        status = "paused-failure";
        failure = {
          message: `Scenario ${scenario.id} exited with code ${exitCode} without producing a run report.`,
          receipt: { scenarioId: scenario.id, status: "missing-report", exitCode, reportPath, stderrPath: paths.stderr },
        };
        await persist();
        break;
      }
      const report = JSON.parse(reportBytes.toString("utf8"));
      const manifestPath = join(paths.evidence, "records", report.evidenceKey, report.runId, "artifact-manifest.json");
      const manifestBytes = await readFile(manifestPath);
      const receipt = { scenarioId: report.scenarioId, status: report.status, exitCode, reportPath, reportSha256: sha256(reportBytes), evidenceKey: report.evidenceKey, evidenceManifestPath: manifestPath, evidenceManifestSha256: sha256(manifestBytes), endedAt: report.endedAt };
      results.push(receipt);
      if (exitCode !== 0 || report.status !== "passed" || report.environment?.candidateSha256 !== candidate.candidateSha256) {
        status = "paused-failure";
        failure = { message: `Scenario ${scenario.id} failed or returned a mismatched candidate identity.`, receipt };
        await persist();
        break;
      }
      completed.push(scenario.id);
      currentScenario = null;
      await atomicJson(paths.checkpoint, { schema: "chemsema.gui.physical-daemon-checkpoint.v1", batchId, commit, candidateSha256: candidate.candidateSha256, profileSha256: profileHash, queueSha256: queueHash, completedScenarioIds: completed, results, updatedAt: now() });
      await persist();
    }
    if (status === "running") status = completed.length === queue.scenarios.length ? "completed" : "stopped";
  } catch (error) {
    status = "failed-infrastructure";
    failure = { name: error.name, message: error.message, stack: error.stack || null };
    process.exitCode = 1;
  } finally {
    currentScenario = null;
    clearInterval(heartbeat);
    await persist();
    await writes;
    await rm(paths.lease, { recursive: true, force: true });
  }
}

async function startDaemon(options) {
  const stateRoot = boundedStateRoot(String(options["state-root"] || ""));
  const paths = statePaths(stateRoot);
  await mkdir(stateRoot, { recursive: true });
  let existing;
  try { existing = JSON.parse(await readFile(paths.state, "utf8")); } catch { /* No previous state. */ }
  if (existing?.status === "running" && processAlive(existing.processId)) throw new Error(`Physical daemon is already running as PID ${existing.processId}.`);
  await rm(paths.stop, { force: true });
  const stdout = openSync(paths.stdout, "a");
  const stderr = openSync(paths.stderr, "a");
  const args = [modulePath, "run", "--profile", resolve(String(options.profile || "")), "--queue", resolve(String(options.queue || "")), "--state-root", stateRoot];
  const child = spawn(process.execPath, args, { cwd: repositoryRoot, detached: true, windowsHide: true, shell: false, stdio: ["ignore", stdout, stderr] });
  closeSync(stdout); closeSync(stderr); child.unref();
  const deadline = Date.now() + 15000;
  do {
    try {
      const state = JSON.parse(await readFile(paths.state, "utf8"));
      if (state.processId === child.pid && state.status === "running") return state;
      if (state.processId === child.pid && state.status !== "running") throw new Error(`Physical daemon entered ${state.status}: ${state.failure?.message || "no detail"}`);
    } catch (error) { if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error; }
    await delay(50);
  } while (Date.now() < deadline);
  throw new Error("Physical daemon did not publish a running heartbeat within 15 seconds.");
}

async function main() {
  const { positional, options } = parseOptions(process.argv.slice(2));
  const command = positional[0];
  if (command === "run") return runDaemon(options);
  const stateRoot = boundedStateRoot(String(options["state-root"] || ""));
  const paths = statePaths(stateRoot);
  if (command === "start") console.log(JSON.stringify(await startDaemon(options), null, 2));
  else if (command === "status") console.log(await readFile(paths.state, "utf8"));
  else if (command === "stop") { await mkdir(stateRoot, { recursive: true }); await writeFile(paths.stop, `${now()}\n`, "utf8"); console.log(JSON.stringify({ schema: "chemsema.gui.physical-daemon-stop.v1", requestedAt: now(), stateRoot }, null, 2)); }
  else throw new Error("usage: physical-daemon <start|run|status|stop> --profile path --queue path --state-root path");
}

if (resolve(process.argv[1] || "") === resolve(modulePath)) main().catch((error) => { console.error(error.stack || error.message); process.exitCode = 1; });
