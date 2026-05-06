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
if (status.stdout.trim()) {
  run("git", ["status", "--short", "--", "viewer/engine"]);
  run("git", ["diff", "--", "viewer/engine"]);
  process.exit(1);
}
