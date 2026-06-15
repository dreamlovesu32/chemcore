import { spawnSync } from "node:child_process";
import { availableParallelism } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mkdirSync, writeFileSync } from "node:fs";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const appDir = join(rootDir, "apps", "chemcore-desktop");
const tauriCli = join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const targetDir = join(rootDir, "target");
const targetExe = join(targetDir, "release", "chemcore-desktop.exe");
const fastConfigPath = join(targetDir, "tauri-fast.conf.json");
const forwardedArgs = process.argv.slice(2);

function escapePowerShellString(value) {
  return String(value).replaceAll("'", "''");
}

function findRunningTargetExe() {
  if (process.platform !== "win32") {
    return [];
  }
  const script = [
    "$ErrorActionPreference = 'SilentlyContinue'",
    "$target = '" + escapePowerShellString(targetExe) + "'",
    "Get-CimInstance Win32_Process -Filter \"name = 'chemcore-desktop.exe'\" |",
    "  Where-Object { $_.ExecutablePath -eq $target } |",
    "  Select-Object -ExpandProperty ProcessId",
  ].join("\n");
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
    { encoding: "utf8", shell: false },
  );
  if (result.status !== 0) {
    return [];
  }
  return result.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

const runningPids = findRunningTargetExe();
if (runningPids.length > 0) {
  console.error(
    [
      `chemcore-desktop.exe is still running from ${targetExe}.`,
      `Close it before rebuilding, or Windows will keep the executable locked.`,
      `Running PID(s): ${runningPids.join(", ")}`,
    ].join("\n"),
  );
  process.exit(1);
}

mkdirSync(targetDir, { recursive: true });
writeFileSync(
  fastConfigPath,
  JSON.stringify({
    build: {
      beforeBuildCommand: process.env.CHEMCORE_FAST_BUILD_WASM === "1"
        ? "node ../../scripts/build-engine-wasm.mjs"
        : "",
    },
    bundle: {
      active: false,
    },
  }),
);

const jobs = Math.max(1, availableParallelism());
const result = spawnSync(
  process.execPath,
  [
    tauriCli,
    "build",
    "--no-bundle",
    "--ci",
    "--config",
    fastConfigPath,
    ...forwardedArgs,
  ],
  {
    cwd: appDir,
    env: {
      ...process.env,
      CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || String(jobs),
    },
    stdio: "inherit",
    shell: false,
  },
);

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 0);
