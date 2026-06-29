import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, cpus, freemem, totalmem } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const stabilityDir = join(rootDir, "tmp", "stability");
const fixtureDir = process.env.CHEMCORE_STABILITY_FIXTURE_DIR || join(stabilityDir, "fixtures");
const outputDir = join(stabilityDir, "outputs", runId);
const reportDir = join(stabilityDir, "reports");
const markdownReportPath = join(reportDir, `stability-report-${runId}.md`);
const jsonReportPath = join(reportDir, `stability-report-${runId}.json`);
const privateCdxml = process.env.CHEMCORE_STABILITY_PRIVATE_CDXML || process.env.CHEMCORE_INTERACTION_SMOKE_CDXML || "";
const privateCdxmlExists = Boolean(privateCdxml && existsSync(privateCdxml));
const commandTimeoutMs = Number(process.env.CHEMCORE_STABILITY_COMMAND_TIMEOUT_MS || 15 * 60 * 1000);
const desktopBuildTimeoutMs = Number(process.env.CHEMCORE_STABILITY_DESKTOP_BUILD_TIMEOUT_MS || 20 * 60 * 1000);

mkdirSync(fixtureDir, { recursive: true });
mkdirSync(outputDir, { recursive: true });
mkdirSync(reportDir, { recursive: true });

const redactions = [];
if (privateCdxml) {
  redactions.push(privateCdxml);
  redactions.push(privateCdxml.replaceAll("\\", "/"));
}

function rel(path) {
  return relative(rootDir, path).replaceAll("\\", "/");
}

function redact(text) {
  let out = String(text ?? "");
  for (const token of redactions.filter(Boolean)) {
    out = out.split(token).join("<private-cdxml>");
  }
  return out;
}

function quoteArg(value) {
  const text = String(value);
  if (/^[A-Za-z0-9_./:=+-]+$/.test(text)) {
    return text;
  }
  return JSON.stringify(text);
}

function commandLine(command, args) {
  return redact([command, ...args].map(quoteArg).join(" "));
}

function appendBounded(current, chunk, limit = 2_000_000) {
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
    stdout: redact(result.stdout || ""),
    stderr: redact(result.stderr || result.error?.message || ""),
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
      env: {
        ...process.env,
        CHEMCORE_STABILITY_FIXTURE_DIR: fixtureDir,
        ...(task.env || {}),
      },
      shell: false,
      windowsHide: true,
    });
  } catch (error) {
    stderr = redact(error.stack || error.message || String(error));
    const elapsedMs = Date.now() - started;
    const result = {
      id: task.id,
      layer: task.layer,
      description: task.description || "",
      command: commandLine(task.command, task.args),
      ok: false,
      exitCode: 1,
      signal: null,
      timedOut: false,
      elapsedMs,
      stdout,
      stderr,
      classification: classifyFailure(task, stdout, stderr, false),
    };
    console.log(`[stability-report] fail ${task.id} (${(elapsedMs / 1000).toFixed(1)}s)`);
    return result;
  }

  const timeout = setTimeout(() => {
    timedOut = true;
    killProcessTree(child);
  }, timeoutMs);

  child.stdout?.on("data", (chunk) => {
    stdout = appendBounded(stdout, redact(chunk.toString()));
  });
  child.stderr?.on("data", (chunk) => {
    stderr = appendBounded(stderr, redact(chunk.toString()));
  });

  const exit = await new Promise((resolve) => {
    child.on("error", (error) => {
      stderr = appendBounded(stderr, redact(error.stack || error.message));
      resolve({ code: 1, signal: null });
    });
    child.on("close", (code, signal) => resolve({ code, signal }));
  });
  clearTimeout(timeout);

  const elapsedMs = Date.now() - started;
  const ok = !timedOut && exit.code === 0;
  const result = {
    id: task.id,
    layer: task.layer,
    description: task.description || "",
    command: commandLine(task.command, task.args),
    ok,
    exitCode: exit.code,
    signal: exit.signal,
    timedOut,
    elapsedMs,
    stdout,
    stderr,
    classification: ok ? "pass" : classifyFailure(task, stdout, stderr, timedOut),
  };
  const status = ok ? "ok" : "fail";
  console.log(`[stability-report] ${status} ${task.id} (${(elapsedMs / 1000).toFixed(1)}s)`);
  return result;
}

function classifyFailure(task, stdout, stderr, timedOut) {
  const text = `${stdout}\n${stderr}`;
  if (timedOut) {
    return "timeout";
  }
  if (task.layer.includes("CLI")) {
    return "cli";
  }
  if (task.layer.includes("Browser") || /playwright|locator|page\.|AssertionError/i.test(text)) {
    return "frontend-interaction";
  }
  if (task.layer.includes("Desktop")) {
    return "desktop";
  }
  if (task.layer.includes("Build")) {
    return "build";
  }
  if (/cargo|rustc|panicked|test result/i.test(text)) {
    return "rust";
  }
  if (/SyntaxError|node --check/i.test(text)) {
    return "javascript";
  }
  return "unknown";
}

async function runStage(stage) {
  console.log(`[stability-report] stage ${stage.name}`);
  if (!stage.parallel) {
    const results = [];
    for (const task of stage.tasks) {
      results.push(await runCommand(task));
    }
    return results;
  }
  const maxParallel = Math.max(1, Math.min(stage.maxParallel || availableParallelism(), stage.tasks.length));
  const results = [];
  let next = 0;
  const workers = Array.from({ length: maxParallel }, async () => {
    while (next < stage.tasks.length) {
      const task = stage.tasks[next];
      next += 1;
      results.push(await runCommand(task));
    }
  });
  await Promise.all(workers);
  return results.sort((a, b) => stage.tasks.findIndex((task) => task.id === a.id) - stage.tasks.findIndex((task) => task.id === b.id));
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
    : { status: 1, stdout: "[]" };
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

function privateFileInfo() {
  if (!privateCdxml) {
    return { configured: false, exists: false };
  }
  if (!privateCdxmlExists) {
    return { configured: true, exists: false };
  }
  const stat = statSync(privateCdxml);
  return {
    configured: true,
    exists: true,
    sizeMB: Number((stat.size / 1024 ** 2).toFixed(2)),
  };
}

const fixtureLarge = join(fixtureDir, "synthetic-large.ccjs");
const fixtureCommands = join(fixtureDir, "synthetic-agent-commands.json");
const cliGenerated = join(outputDir, "agent-generated.ccjs");
const cliGeneratedReport = join(outputDir, "agent-generated.report.json");
const cliGeneratedDoc = join(outputDir, "agent-generated.document.json");
const syntheticInspect = join(outputDir, "synthetic-large.inspect.json");
const syntheticCdxml = join(outputDir, "synthetic-large.cdxml");
const syntheticSvg = join(outputDir, "synthetic-large.svg");
const syntheticEditedReport = join(outputDir, "synthetic-large-edited.report.json");
const syntheticEdited = join(outputDir, "synthetic-large-edited.ccjs");
const privateInspect = join(outputDir, "private-large.inspect.json");
const privateSvg = join(outputDir, "private-large.svg");
const cliBinary = join(rootDir, "target", "debug", process.platform === "win32" ? "chemcore-cli.exe" : "chemcore-cli");

const jsCheckFiles = [
  "viewer/app.js",
  "viewer/app_window_lifecycle.js",
  "viewer/browser_document_tabs.js",
  "viewer/editor_document_renderer.js",
  "viewer/editor_toolbar_host.js",
  "viewer/editor_viewport_host.js",
  "viewer/engine_host.js",
  "viewer/editor_command_engine.js",
  "viewer/editor_pointer_controller.js",
  "viewer/editor_selection_hit_model.js",
  "viewer/editor_tool_model.js",
  "viewer/toolbar.js",
  "scripts/generate-stability-fixtures.mjs",
  "scripts/stability-user-paths.mjs",
  "scripts/stability-report.mjs",
  "scripts/viewer-interaction-smoke.mjs",
  "scripts/desktop-hybrid-latency-regression.mjs",
  "scripts/large-object-operation-regression.mjs",
  "scripts/large-drag-preview-regression.mjs",
];

const cliTasks = [
  {
    id: "cli-new-agent-commands",
    layer: "CLI",
    command: cliBinary,
    args: [
      "new", rel(fixtureCommands),
      "--out", rel(cliGenerated),
      "--results", rel(cliGeneratedReport),
      "--document-json", rel(cliGeneratedDoc),
      "--inspect-after", "summary,objects,molecules,resources",
      "--continue-on-error",
      "--pretty",
    ],
  },
  {
    id: "cli-inspect-synthetic-large",
    layer: "CLI",
    command: cliBinary,
    args: [
      "inspect", rel(fixtureLarge),
      "--include", "summary,objects,molecules,resources",
      "--out", rel(syntheticInspect),
      "--pretty",
    ],
  },
  {
    id: "cli-convert-synthetic-ccjs-cdxml",
    layer: "CLI",
    command: cliBinary,
    args: ["convert", rel(fixtureLarge), rel(syntheticCdxml)],
  },
  {
    id: "cli-convert-synthetic-cdxml-svg",
    layer: "CLI",
    command: cliBinary,
    args: ["convert", rel(syntheticCdxml), rel(syntheticSvg)],
  },
  {
    id: "cli-run-agent-commands-on-synthetic-cdxml",
    layer: "CLI",
    command: cliBinary,
    args: [
      "run", rel(syntheticCdxml), rel(fixtureCommands),
      "--out", rel(syntheticEdited),
      "--results", rel(syntheticEditedReport),
      "--inspect-after", "summary,objects,molecules",
      "--continue-on-error",
      "--pretty",
    ],
  },
];

if (privateCdxmlExists) {
  cliTasks.push(
    {
      id: "cli-inspect-private-large-cdxml",
      layer: "CLI Private Large File",
      command: cliBinary,
      args: [
        "inspect", privateCdxml,
        "--include", "summary,objects,molecules,resources",
        "--out", rel(privateInspect),
        "--pretty",
      ],
    },
    {
      id: "cli-convert-private-large-cdxml-svg",
      layer: "CLI Private Large File",
      command: cliBinary,
      args: ["convert", privateCdxml, rel(privateSvg)],
      timeoutMs: commandTimeoutMs,
    },
  );
}

const stages = [
  {
    name: "fixtures",
    tasks: [{
      id: "generate-synthetic-fixtures",
      layer: "Fixture",
      command: process.execPath,
      args: ["scripts/generate-stability-fixtures.mjs"],
    }],
  },
  {
    name: "javascript syntax",
    parallel: true,
    maxParallel: Math.min(8, availableParallelism()),
    tasks: jsCheckFiles.map((file) => ({
      id: `node-check-${file.replace(/[\\/]/g, "-")}`,
      layer: "Static JS",
      command: process.execPath,
      args: ["--check", file],
    })),
  },
  {
    name: "rust workspace",
    tasks: [{
      id: "cargo-test-workspace",
      layer: "Rust",
      command: "cargo",
      args: ["test", "--workspace"],
      timeoutMs: commandTimeoutMs,
    }],
  },
  {
    name: "wasm build",
    tasks: [{
      id: "build-engine-wasm",
      layer: "Build",
      command: process.execPath,
      args: ["scripts/build-engine-wasm.mjs"],
      timeoutMs: commandTimeoutMs,
    }],
  },
  {
    name: "cli",
    tasks: [
      {
        id: "cli-build",
        layer: "CLI Build",
        command: "cargo",
        args: ["build", "-p", "chemcore-cli"],
        timeoutMs: commandTimeoutMs,
      },
      ...cliTasks,
    ],
  },
  {
    name: "browser and desktop-like interaction",
    tasks: [
      {
        id: "browser-stability-user-paths",
        layer: "Browser Interaction",
        command: process.execPath,
        args: ["scripts/stability-user-paths.mjs"],
        env: { CHEMCORE_DESKTOP_DEV_PORT: "8773" },
      },
      {
        id: "browser-viewer-interaction-smoke",
        layer: "Browser Interaction",
        command: process.execPath,
        args: ["scripts/viewer-interaction-smoke.mjs"],
        env: {
          CHEMCORE_DESKTOP_DEV_PORT: "8774",
          ...(privateCdxmlExists ? { CHEMCORE_STABILITY_PRIVATE_CDXML: privateCdxml } : {}),
        },
        timeoutMs: commandTimeoutMs,
      },
      {
        id: "desktop-hybrid-latency-regression",
        layer: "Desktop Hybrid",
        command: process.execPath,
        args: ["scripts/desktop-hybrid-latency-regression.mjs"],
        env: { CHEMCORE_DESKTOP_DEV_PORT: "8775" },
      },
      {
        id: "large-object-operation-regression",
        layer: "Browser Interaction",
        command: process.execPath,
        args: ["scripts/large-object-operation-regression.mjs"],
        env: { CHEMCORE_DESKTOP_DEV_PORT: "8776" },
        timeoutMs: commandTimeoutMs,
      },
      {
        id: "large-drag-preview-regression",
        layer: "Browser Interaction",
        command: process.execPath,
        args: ["scripts/large-drag-preview-regression.mjs"],
        env: { CHEMCORE_DESKTOP_DEV_PORT: "8777" },
        timeoutMs: commandTimeoutMs,
      },
    ],
  },
  {
    name: "desktop build",
    tasks: [{
      id: "desktop-build-fast",
      layer: "Desktop Build",
      command: process.execPath,
      args: ["scripts/desktop-tauri-fast.mjs"],
      timeoutMs: desktopBuildTimeoutMs,
    }],
  },
];

const hardware = collectHardware();
const git = collectGit();
const privateInfo = privateFileInfo();
const allResults = [];
for (const stage of stages) {
  allResults.push(...await runStage(stage));
}

const failures = allResults.filter((result) => !result.ok);
const report = {
  ok: failures.length === 0,
  runId,
  generatedAt: new Date().toISOString(),
  rootDir,
  fixtureDir,
  outputDir,
  reports: {
    markdown: markdownReportPath,
    json: jsonReportPath,
  },
  hardware,
  git,
  privateCdxml: privateInfo,
  summary: {
    total: allResults.length,
    passed: allResults.length - failures.length,
    failed: failures.length,
    failureClasses: failures.reduce((counts, failure) => {
      counts[failure.classification] = (counts[failure.classification] || 0) + 1;
      return counts;
    }, {}),
  },
  results: allResults,
};

writeFileSync(jsonReportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
writeFileSync(markdownReportPath, renderMarkdownReport(report), "utf8");

console.log(`[stability-report] wrote ${rel(markdownReportPath)}`);
console.log(`[stability-report] wrote ${rel(jsonReportPath)}`);
if (failures.length) {
  process.exit(1);
}

function escapeTable(value) {
  return String(value ?? "").replaceAll("|", "\\|").replace(/\r?\n/g, " ");
}

function tail(text, max = 6000) {
  const value = String(text || "").trim();
  if (value.length <= max) {
    return value;
  }
  return `[tail ${max} chars]\n${value.slice(value.length - max)}`;
}

function renderMarkdownReport(data) {
  const lines = [];
  lines.push("# ChemCore Stability Report");
  lines.push("");
  lines.push(`- Run: \`${data.runId}\``);
  lines.push(`- Generated: \`${data.generatedAt}\``);
  lines.push(`- Git: \`${data.git.branch || "detached"}@${data.git.commit || "unknown"}\`, dirty files: ${data.git.dirtyCount}`);
  lines.push(`- CPU: ${data.hardware.cpu.model}, logical cores: ${data.hardware.cpu.logicalCores}, available parallelism: ${data.hardware.availableParallelism}`);
  lines.push(`- Memory: ${data.hardware.memory.totalGB} GB total, ${data.hardware.memory.freeGB} GB free at start`);
  if (data.hardware.disks.length) {
    lines.push(`- Disks: ${data.hardware.disks.map((disk) => `${disk.DeviceID} ${disk.FreeGB}/${disk.SizeGB} GB free`).join("; ")}`);
  }
  const privateLabel = data.privateCdxml.configured
    ? data.privateCdxml.exists
      ? `configured (${data.privateCdxml.sizeMB} MB)`
      : "configured but missing"
    : "not configured";
  lines.push(`- Private CDXML: ${privateLabel}`);
  lines.push(`- Fixture dir: \`${rel(data.fixtureDir)}\``);
  lines.push(`- Output dir: \`${rel(data.outputDir)}\``);
  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- Result: ${data.ok ? "PASS" : "FAIL"}`);
  lines.push(`- Passed: ${data.summary.passed}/${data.summary.total}`);
  lines.push(`- Failed: ${data.summary.failed}/${data.summary.total}`);
  if (Object.keys(data.summary.failureClasses).length) {
    lines.push(`- Failure classes: ${Object.entries(data.summary.failureClasses).map(([name, count]) => `${name}=${count}`).join(", ")}`);
  }
  lines.push("");
  lines.push("## Layer Results");
  lines.push("");
  lines.push("| Layer | Test | Status | Time | Exit | Class |");
  lines.push("| --- | --- | --- | ---: | ---: | --- |");
  for (const result of data.results) {
    lines.push(`| ${escapeTable(result.layer)} | ${escapeTable(result.id)} | ${result.ok ? "PASS" : "FAIL"} | ${(result.elapsedMs / 1000).toFixed(1)}s | ${escapeTable(result.exitCode ?? result.signal ?? "")} | ${escapeTable(result.classification)} |`);
  }
  if (data.git.statusShort) {
    lines.push("");
    lines.push("## Git Status");
    lines.push("");
    lines.push("```text");
    lines.push(data.git.statusShort);
    lines.push("```");
  }
  if (data.summary.failed) {
    lines.push("");
    lines.push("## Failures");
    for (const failure of data.results.filter((result) => !result.ok)) {
      lines.push("");
      lines.push(`### ${failure.id}`);
      lines.push("");
      lines.push(`- Layer: ${failure.layer}`);
      lines.push(`- Class: ${failure.classification}`);
      lines.push(`- Duration: ${(failure.elapsedMs / 1000).toFixed(1)}s`);
      lines.push(`- Command: \`${failure.command}\``);
      if (failure.timedOut) {
        lines.push("- Timed out: yes");
      }
      const stdoutTail = tail(failure.stdout);
      const stderrTail = tail(failure.stderr);
      if (stdoutTail) {
        lines.push("");
        lines.push("stdout:");
        lines.push("```text");
        lines.push(stdoutTail);
        lines.push("```");
      }
      if (stderrTail) {
        lines.push("");
        lines.push("stderr:");
        lines.push("```text");
        lines.push(stderrTail);
        lines.push("```");
      }
    }
  }
  lines.push("");
  lines.push("## Notes");
  lines.push("");
  lines.push("- This runner is collecting-oriented: all stages are attempted even after earlier failures.");
  lines.push("- Private CDXML paths are redacted in captured command output.");
  lines.push("- Browser scripts that self-manage their dev server use isolated ports to avoid cross-test interference.");
  lines.push("");
  return `${lines.join("\n")}\n`;
}
