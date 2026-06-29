import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, cpus, freemem, totalmem } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { writeStabilityFixtures } from "./generate-stability-fixtures.mjs";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const perfDir = join(rootDir, "tmp", "performance", "cli-large");
const fixtureDir = join(perfDir, "fixtures");
const outputDir = join(perfDir, "outputs", runId);
const cacheDir = join(outputDir, "cache");
const reportDir = join(perfDir, "reports");
const markdownReportPath = join(reportDir, `cli-large-performance-${runId}.md`);
const jsonReportPath = join(reportDir, `cli-large-performance-${runId}.json`);
const commandTimeoutMs = Number(process.env.CHEMCORE_CLI_PERF_TIMEOUT_MS || 10 * 60 * 1000);
const nodeCount = Number(process.env.CHEMCORE_CLI_PERF_SYNTHETIC_NODES || 6400);
const objectRepeats = Number(process.env.CHEMCORE_CLI_PERF_SYNTHETIC_OBJECT_REPEATS || 48);
const cliBinary = join(rootDir, "target", "debug", process.platform === "win32" ? "chemcore-cli.exe" : "chemcore-cli");

mkdirSync(fixtureDir, { recursive: true });
mkdirSync(outputDir, { recursive: true });
mkdirSync(cacheDir, { recursive: true });
mkdirSync(reportDir, { recursive: true });

function rel(path) {
  return relative(rootDir, path).replaceAll("\\", "/");
}

function quoteArg(value) {
  const text = String(value);
  if (/^[A-Za-z0-9_./:=+-]+$/.test(text)) {
    return text;
  }
  return JSON.stringify(text);
}

function commandLine(command, args) {
  return [command, ...args].map(quoteArg).join(" ");
}

function appendBounded(current, chunk, limit = 1_000_000) {
  const next = current + chunk;
  if (next.length <= limit) {
    return next;
  }
  return `[truncated]\n${next.slice(next.length - limit)}`;
}

function killProcessTree(child) {
  if (!child.pid) {
    return;
  }
  if (process.platform === "win32") {
    spawnSync("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], {
      stdio: "ignore",
      windowsHide: true,
    });
    return;
  }
  child.kill("SIGTERM");
}

function runSync(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    env: { ...process.env, ...(options.env || {}) },
    encoding: "utf8",
    shell: false,
    windowsHide: true,
  });
  return {
    status: result.status ?? (result.error ? 1 : 0),
    stdout: result.stdout || "",
    stderr: result.stderr || result.error?.message || "",
  };
}

async function runCommand(task) {
  const started = Date.now();
  const timeoutMs = task.timeoutMs || commandTimeoutMs;
  let stdout = "";
  let stderr = "";
  let timedOut = false;
  let child = null;
  try {
    child = spawn(task.command, task.args, {
      cwd: rootDir,
      env: { ...process.env, ...(task.env || {}) },
      shell: false,
      windowsHide: true,
    });
  } catch (error) {
    stderr = error.stack || error.message || String(error);
    return finishTask(task, started, 1, null, false, stdout, stderr);
  }

  const timeout = setTimeout(() => {
    timedOut = true;
    killProcessTree(child);
  }, timeoutMs);

  child.stdout?.on("data", (chunk) => {
    stdout = appendBounded(stdout, chunk.toString());
  });
  child.stderr?.on("data", (chunk) => {
    stderr = appendBounded(stderr, chunk.toString());
  });

  const exit = await new Promise((resolve) => {
    child.on("error", (error) => {
      stderr = appendBounded(stderr, error.stack || error.message || String(error));
      resolve({ code: 1, signal: null });
    });
    child.on("close", (code, signal) => resolve({ code, signal }));
  });
  clearTimeout(timeout);
  return finishTask(task, started, exit.code, exit.signal, timedOut, stdout, stderr);
}

async function runTask(task) {
  if (task.session) {
    return runSessionTask(task);
  }
  return runCommand(task);
}

async function runSessionTask(task) {
  const started = Date.now();
  const timeoutMs = task.timeoutMs || commandTimeoutMs;
  let stdoutBytes = 0;
  let stderr = "";
  let timedOut = false;
  let buffer = "";
  const responses = [];
  const child = spawn(task.command, task.args, {
    cwd: rootDir,
    env: { ...process.env, ...(task.env || {}) },
    shell: false,
    windowsHide: true,
  });

  const timeout = setTimeout(() => {
    timedOut = true;
    killProcessTree(child);
  }, timeoutMs);

  child.stdout?.on("data", (chunk) => {
    const text = chunk.toString();
    stdoutBytes += Buffer.byteLength(text);
    buffer += text;
    let newlineIndex = buffer.indexOf("\n");
    while (newlineIndex >= 0) {
      const line = buffer.slice(0, newlineIndex).trim();
      buffer = buffer.slice(newlineIndex + 1);
      if (line) {
        try {
          const message = JSON.parse(line);
          responses.push(summarizeSessionMessage(message));
          if (message.event === "ready") {
            for (const request of task.session.requests) {
              child.stdin.write(`${JSON.stringify(request)}\n`);
            }
          }
        } catch (error) {
          responses.push({ ok: false, error: `Invalid session JSON response: ${error.message}` });
        }
      }
      newlineIndex = buffer.indexOf("\n");
    }
  });
  child.stderr?.on("data", (chunk) => {
    stderr = appendBounded(stderr, chunk.toString());
  });

  const exit = await new Promise((resolve) => {
    child.on("error", (error) => {
      stderr = appendBounded(stderr, error.stack || error.message);
      resolve({ code: 1, signal: null });
    });
    child.on("close", (code, signal) => resolve({ code, signal }));
  });
  clearTimeout(timeout);
  const elapsedMs = Date.now() - started;
  const failedResponses = responses.filter((response) => response.ok === false);
  const ok = !timedOut && exit.code === 0 && failedResponses.length === 0;
  const result = {
    id: task.id,
    group: task.group || "",
    description: task.description || "",
    command: commandLine(task.command, task.args),
    ok,
    exitCode: exit.code,
    signal: exit.signal,
    timedOut,
    elapsedMs,
    stdoutBytes,
    stderrBytes: Buffer.byteLength(stderr),
    stdoutTail: "",
    stderrTail: tail(stderr),
    parsed: {
      protocol: "chemcore-cli-session-jsonl-v1",
      responseCount: responses.length,
      failedResponses: failedResponses.length,
      responses,
    },
    artifacts: collectArtifacts(task),
  };
  console.log(`[cli-large-performance] ${ok ? "ok" : "fail"} ${task.id} (${(elapsedMs / 1000).toFixed(2)}s)`);
  return result;
}

function summarizeSessionMessage(message) {
  const result = message.result || {};
  const capture = result.capture || result;
  return {
    ok: message.ok,
    event: message.event || null,
    id: message.id ?? null,
    op: message.op || null,
    error: message.error?.message || null,
    targetCount: result.targetCount ?? null,
    revision: result.revision ?? result.document?.afterRevision ?? null,
    render: capture.render || null,
    artifacts: result.output?.path || result.capture?.path || result.path || null,
  };
}

function finishTask(task, started, exitCode, signal, timedOut, stdout, stderr, parsedOverride = undefined) {
  const elapsedMs = Date.now() - started;
  const ok = !timedOut && exitCode === 0;
  const parsed = parsedOverride === undefined ? parseTaskJson(task, stdout) : parsedOverride;
  const artifacts = collectArtifacts(task);
  const result = {
    id: task.id,
    group: task.group || "",
    description: task.description || "",
    command: commandLine(task.command, task.args),
    ok,
    exitCode,
    signal,
    timedOut,
    elapsedMs,
    stdoutBytes: Buffer.byteLength(stdout),
    stderrBytes: Buffer.byteLength(stderr),
    stdoutTail: tail(stdout),
    stderrTail: tail(stderr),
    parsed,
    artifacts,
  };
  console.log(`[cli-large-performance] ${ok ? "ok" : "fail"} ${task.id} (${(elapsedMs / 1000).toFixed(2)}s)`);
  return result;
}

function parseTaskJson(task, stdout) {
  const source = task.jsonOutputPath && existsSync(task.jsonOutputPath)
    ? readFileSync(task.jsonOutputPath, "utf8")
    : stdout.trim();
  if (!source) {
    return null;
  }
  try {
    return JSON.parse(source);
  } catch {
    return null;
  }
}

function collectArtifacts(task) {
  return (task.artifacts || [])
    .filter((artifact) => artifact.path && existsSync(artifact.path))
    .map((artifact) => ({
      label: artifact.label,
      path: rel(artifact.path),
      bytes: statSync(artifact.path).size,
    }));
}

function tail(text, max = 4000) {
  const value = String(text || "").trim();
  if (value.length <= max) {
    return value;
  }
  return `[tail ${max} chars]\n${value.slice(value.length - max)}`;
}

function collectHardware() {
  const cpu = cpus()[0] || {};
  const diskResult = process.platform === "win32"
    ? runSync("powershell.exe", [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        "Get-CimInstance Win32_LogicalDisk -Filter \"DriveType=3\" | Select-Object DeviceID,@{n='SizeGB';e={[math]::Round($_.Size/1GB,1)}},@{n='FreeGB';e={[math]::Round($_.FreeSpace/1GB,1)}} | ConvertTo-Json -Compress",
      ])
    : { stdout: "[]" };
  let disks = [];
  try {
    const parsed = JSON.parse(diskResult.stdout || "[]");
    disks = Array.isArray(parsed) ? parsed : [parsed];
  } catch {
    disks = [];
  }
  return {
    cpu: {
      model: cpu.model || "unknown",
      logicalCores: cpus().length,
    },
    memory: {
      totalGB: Number((totalmem() / 1024 ** 3).toFixed(2)),
      freeGB: Number((freemem() / 1024 ** 3).toFixed(2)),
    },
    disks,
    availableParallelism: availableParallelism(),
  };
}

function collectGit() {
  const branch = runSync("git", ["branch", "--show-current"]);
  const commit = runSync("git", ["rev-parse", "--short", "HEAD"]);
  const status = runSync("git", ["status", "--short"]);
  return {
    branch: branch.stdout.trim(),
    commit: commit.stdout.trim(),
    dirtyCount: status.stdout.split(/\r?\n/).filter(Boolean).length,
    statusShort: status.stdout.trim(),
  };
}

function taskOutput(name, extension) {
  return join(outputDir, `${name}.${extension}`);
}

function cliTask(id, group, args, options = {}) {
  return {
    id,
    group,
    command: cliBinary,
    args,
    ...options,
    env: {
      CHEMCORE_CLI_CACHE_DIR: cacheDir,
      ...(options.env || {}),
    },
  };
}

function discoverProbeTargets(targetsPath) {
  const fallback = { node: "node:missing", bond: "bond:missing" };
  if (!existsSync(targetsPath)) {
    return fallback;
  }
  try {
    const data = JSON.parse(readFileSync(targetsPath, "utf8"));
    const nodes = data?.targets?.nodes || [];
    const bonds = data?.targets?.bonds || [];
    const node = nodes[Math.floor(nodes.length / 2)]?.selector;
    const bond = bonds[Math.floor(bonds.length / 2)]?.selector;
    return {
      node: node || fallback.node,
      bond: bond || fallback.bond,
    };
  } catch {
    return fallback;
  }
}

const fixtureManifest = writeStabilityFixtures({
  fixtureDir,
  nodeCount,
  objectRepeats,
});
const fixtureLargeCcjs = fixtureManifest.files.syntheticLargeCcjs;
const fixtureLargeCdxml = taskOutput("synthetic-large", "cdxml");
const targetsColdPath = taskOutput("targets-cold", "json");
const targetsWarmPath = taskOutput("targets-warm", "json");

const setupTasks = [
  {
    id: "build-cli-debug",
    group: "setup",
    command: "cargo",
    args: ["build", "-p", "chemcore-cli"],
    timeoutMs: commandTimeoutMs,
  },
  cliTask("convert-ccjs-to-cdxml", "setup", ["convert", rel(fixtureLargeCcjs), rel(fixtureLargeCdxml)], {
    artifacts: [{ label: "cdxml", path: fixtureLargeCdxml }],
  }),
];

const discoveryTasks = [
  cliTask("targets-cdxml-cold", "read-index", ["targets", rel(fixtureLargeCdxml), "--out", rel(taskOutput("targets-cold", "json"))], {
    jsonOutputPath: targetsColdPath,
    artifacts: [{ label: "targets", path: targetsColdPath }],
  }),
];

const allResults = [];
for (const task of [...setupTasks, ...discoveryTasks]) {
  allResults.push(await runTask(task));
}

const discoveredTargets = discoverProbeTargets(targetsColdPath);

const measuredTasks = [
  cliTask("targets-cdxml-warm", "read-index", ["targets", rel(fixtureLargeCdxml), "--out", rel(targetsWarmPath)], {
    jsonOutputPath: targetsWarmPath,
    artifacts: [{ label: "targets", path: targetsWarmPath }],
  }),
  cliTask("detail-molecule", "detail", ["detail", rel(fixtureLargeCdxml), "--target", "molecule:0", "--summary-only", "--out", rel(taskOutput("detail-molecule", "json"))], {
    jsonOutputPath: taskOutput("detail-molecule", "json"),
    artifacts: [{ label: "detail", path: taskOutput("detail-molecule", "json") }],
  }),
  cliTask("context-node-with-png", "context-capture", [
    "context", rel(fixtureLargeCdxml),
    "--target", discoveredTargets.node,
    "--radius", "120",
    "--out", rel(taskOutput("context-node", "json")),
    "--capture-out", rel(taskOutput("context-node", "png")),
    "--scale", "6",
  ], {
    jsonOutputPath: taskOutput("context-node", "json"),
    artifacts: [
      { label: "context", path: taskOutput("context-node", "json") },
      { label: "capture", path: taskOutput("context-node", "png") },
    ],
  }),
  cliTask("context-selection-with-png", "context-capture", [
    "context", rel(fixtureLargeCdxml),
    "--target", discoveredTargets.node,
    "--target", discoveredTargets.bond,
    "--expand", "120",
    "--out", rel(taskOutput("context-selection", "json")),
    "--capture-out", rel(taskOutput("context-selection", "png")),
    "--width", "1800",
  ], {
    jsonOutputPath: taskOutput("context-selection", "json"),
    artifacts: [
      { label: "context", path: taskOutput("context-selection", "json") },
      { label: "capture", path: taskOutput("context-selection", "png") },
    ],
  }),
  cliTask("capture-node-high-scale", "capture", [
    "capture", rel(fixtureLargeCdxml),
    "--target", discoveredTargets.node,
    "--out", rel(taskOutput("capture-node", "png")),
    "--scale", "10",
    "--expand", "140",
  ], {
    artifacts: [{ label: "capture", path: taskOutput("capture-node", "png") }],
  }),
  cliTask("capture-bond-wide-context", "capture", [
    "capture", rel(fixtureLargeCdxml),
    "--target", discoveredTargets.bond,
    "--out", rel(taskOutput("capture-bond", "png")),
    "--width", "1800",
    "--expand", "160",
  ], {
    artifacts: [{ label: "capture", path: taskOutput("capture-bond", "png") }],
  }),
  cliTask("capture-molecule-width", "capture", [
    "capture", rel(fixtureLargeCdxml),
    "--target", "molecule:0",
    "--out", rel(taskOutput("capture-molecule", "png")),
    "--width", "2400",
    "--expand", "8",
  ], {
    artifacts: [{ label: "capture", path: taskOutput("capture-molecule", "png") }],
  }),
  cliTask("session-read-capture-sequence", "session", ["session", rel(fixtureLargeCdxml)], {
    session: {
      requests: [
        { id: 1, op: "status" },
        { id: 2, op: "detail", target: "molecule:0", summaryOnly: true },
        {
          id: 3,
          op: "context",
          target: discoveredTargets.node,
          radius: 120,
          captureOut: rel(taskOutput("session-context-node", "png")),
          scale: 6,
        },
        {
          id: 4,
          op: "capture",
          target: discoveredTargets.bond,
          out: rel(taskOutput("session-capture-bond", "png")),
          width: 1800,
          expand: 160,
        },
        { id: 5, op: "exit" },
      ],
    },
    artifacts: [
      { label: "context", path: taskOutput("session-context-node", "png") },
      { label: "capture", path: taskOutput("session-capture-bond", "png") },
    ],
  }),
  cliTask("convert-cdxml-to-svg", "convert", ["convert", rel(fixtureLargeCdxml), rel(taskOutput("synthetic-large", "svg"))], {
    artifacts: [{ label: "svg", path: taskOutput("synthetic-large", "svg") }],
  }),
];

for (const task of measuredTasks) {
  allResults.push(await runTask(task));
}

const failures = allResults.filter((result) => !result.ok);
const measuredResults = allResults.filter((result) => result.group !== "setup");
const report = {
  ok: failures.length === 0,
  runId,
  generatedAt: new Date().toISOString(),
  rootDir,
  fixtureDir,
  outputDir,
  cacheDir,
  reports: {
    markdown: markdownReportPath,
    json: jsonReportPath,
  },
  hardware: collectHardware(),
  git: collectGit(),
  fixture: {
    synthetic: fixtureManifest.synthetic,
    files: Object.fromEntries(Object.entries(fixtureManifest.files).map(([key, value]) => [key, rel(value)])),
    generatedCdxml: rel(fixtureLargeCdxml),
    targets: {
      node: discoveredTargets.node,
      bond: discoveredTargets.bond,
    },
  },
  summary: {
    total: allResults.length,
    measured: measuredResults.length,
    passed: allResults.length - failures.length,
    failed: failures.length,
    measuredElapsedMs: measuredResults.reduce((sum, result) => sum + result.elapsedMs, 0),
  },
  results: allResults,
};

writeFileSync(jsonReportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
writeFileSync(markdownReportPath, renderMarkdownReport(report), "utf8");

console.log(`[cli-large-performance] wrote ${rel(markdownReportPath)}`);
console.log(`[cli-large-performance] wrote ${rel(jsonReportPath)}`);
if (failures.length) {
  process.exit(1);
}

function escapeTable(value) {
  return String(value ?? "").replaceAll("|", "\\|").replace(/\r?\n/g, " ");
}

function renderMarkdownReport(data) {
  const lines = [];
  lines.push("# ChemCore CLI Large-File Performance Report");
  lines.push("");
  lines.push(`- Run: \`${data.runId}\``);
  lines.push(`- Generated: \`${data.generatedAt}\``);
  lines.push(`- Git: \`${data.git.branch || "detached"}@${data.git.commit || "unknown"}\`, dirty files: ${data.git.dirtyCount}`);
  lines.push(`- CPU: ${data.hardware.cpu.model}, logical cores: ${data.hardware.cpu.logicalCores}, available parallelism: ${data.hardware.availableParallelism}`);
  lines.push(`- Memory: ${data.hardware.memory.totalGB} GB total, ${data.hardware.memory.freeGB} GB free at start`);
  if (data.hardware.disks.length) {
    lines.push(`- Disks: ${data.hardware.disks.map((disk) => `${disk.DeviceID} ${disk.FreeGB}/${disk.SizeGB} GB free`).join("; ")}`);
  }
  lines.push(`- Fixture: ${data.fixture.synthetic.nodes} nodes, ${data.fixture.synthetic.bonds} bonds, ${data.fixture.synthetic.objects} objects, ${(data.fixture.synthetic.bytes / 1024 / 1024).toFixed(2)} MB ccjs`);
  lines.push(`- Probe targets: \`${data.fixture.targets.node}\`, \`${data.fixture.targets.bond}\``);
  lines.push(`- Output dir: \`${rel(data.outputDir)}\``);
  lines.push(`- CLI cache dir: \`${rel(data.cacheDir)}\``);
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- Result: ${data.ok ? "PASS" : "FAIL"}`);
  lines.push(`- Passed: ${data.summary.passed}/${data.summary.total}`);
  lines.push(`- Measured elapsed total: ${(data.summary.measuredElapsedMs / 1000).toFixed(2)}s`);
  lines.push("");
  lines.push("## Results");
  lines.push("");
  lines.push("| Group | Task | Status | Time | Render | Artifacts |");
  lines.push("| --- | --- | --- | ---: | --- | --- |");
  for (const result of data.results) {
    lines.push(`| ${escapeTable(result.group)} | ${escapeTable(result.id)} | ${result.ok ? "PASS" : "FAIL"} | ${(result.elapsedMs / 1000).toFixed(2)}s | ${escapeTable(renderSummary(result))} | ${escapeTable(artifactSummary(result))} |`);
  }
  if (data.summary.failed) {
    lines.push("");
    lines.push("## Failures");
    for (const failure of data.results.filter((result) => !result.ok)) {
      lines.push("");
      lines.push(`### ${failure.id}`);
      lines.push("");
      lines.push(`- Command: \`${failure.command}\``);
      lines.push(`- Duration: ${(failure.elapsedMs / 1000).toFixed(2)}s`);
      if (failure.timedOut) {
        lines.push("- Timed out: yes");
      }
      if (failure.stdoutTail) {
        lines.push("");
        lines.push("stdout:");
        lines.push("```text");
        lines.push(failure.stdoutTail);
        lines.push("```");
      }
      if (failure.stderrTail) {
        lines.push("");
        lines.push("stderr:");
        lines.push("```text");
        lines.push(failure.stderrTail);
        lines.push("```");
      }
    }
  }
  lines.push("");
  lines.push("## Notes");
  lines.push("");
  lines.push("- This runner is collecting-oriented: all tasks are attempted even after earlier failures.");
  lines.push("- The synthetic fixture is generated locally under `tmp/performance` and is safe to delete.");
  lines.push("- `Render` summarizes CLI capture metadata when the command reports it.");
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function renderSummary(result) {
  if (result.parsed?.protocol === "chemcore-cli-session-jsonl-v1") {
    return (result.parsed.responses || [])
      .filter((response) => response.render)
      .map((response) => `${response.op}:${response.render.mode} primitives=${response.render.primitiveCount}`)
      .join("; ");
  }
  const capture = result.parsed?.capture || result.parsed;
  const render = capture?.render;
  if (!render) {
    return "";
  }
  const targets = render.targets
    ? ` targets n=${render.targets.nodes} b=${render.targets.bonds} o=${render.targets.objects}`
    : "";
  return `${render.mode || ""} primitives=${render.primitiveCount ?? ""}${targets}`;
}

function artifactSummary(result) {
  return (result.artifacts || [])
    .map((artifact) => `${artifact.label}:${artifact.bytes}B`)
    .join(", ");
}
