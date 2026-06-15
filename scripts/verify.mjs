import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: options.capture ? "pipe" : "inherit",
    encoding: options.capture ? "utf8" : undefined,
    shell: false,
  });
  if (result.error) {
    throw result.error;
  }
  if (!options.allowFailure && result.status !== 0) {
    process.exit(result.status ?? 1);
  }
  return result;
}

run("cargo", ["test"]);
run(process.execPath, ["scripts/build-engine-wasm.mjs"]);
run(process.execPath, ["--check", "viewer/app.js"]);

const status = run("git", ["status", "--porcelain", "--", "viewer/engine"], {
  capture: true,
  allowFailure: true,
});
if (status.status !== 0) {
  process.stdout.write(status.stdout || "");
  process.stderr.write(status.stderr || "");
  process.exit(status.status ?? 1);
}
const generatedChanges = status.stdout
  .trim()
  .split(/\r?\n/)
  .filter(Boolean);
const blockingChanges = generatedChanges.filter((line) => {
  const path = line.slice(3).replace(/\\/g, "/");
  return !(process.env.CI === "true" && path === "viewer/engine/chemcore_engine_bg.wasm");
});

if (blockingChanges.length) {
  run("git", ["status", "--short", "--", "viewer/engine"]);
  run("git", ["diff", "--", "viewer/engine"]);
  process.exit(1);
}
if (generatedChanges.length) {
  console.warn(
    "CI rebuilt viewer/engine/chemcore_engine_bg.wasm with platform-local binary metadata; non-binary generated files are clean.",
  );
}
