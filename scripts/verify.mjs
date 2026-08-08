import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const generatedEngineDir = join(rootDir, "viewer", "engine");

function generatedEngineSnapshot(directory = generatedEngineDir) {
  const snapshot = new Map();
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      for (const [nestedPath, hash] of generatedEngineSnapshot(path)) {
        snapshot.set(nestedPath, hash);
      }
      continue;
    }
    const key = relative(generatedEngineDir, path).replace(/\\/g, "/");
    const hash = createHash("sha256").update(readFileSync(path)).digest("hex");
    snapshot.set(key, hash);
  }
  return snapshot;
}

function changedGeneratedFiles(before, after) {
  return [...new Set([...before.keys(), ...after.keys()])]
    .filter((path) => before.get(path) !== after.get(path))
    .sort();
}

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

run("cargo", ["build", "-p", "chemsema-office", "-p", "chemsema-cli", "--release"]);
run("cargo", ["test"]);
run(process.execPath, ["scripts/ccjz-conformance.mjs"]);
run("cargo", [
  "run",
  "--release",
  "--quiet",
  "-p",
  "chemsema-container",
  "--example",
  "ccjz_performance",
]);
run(process.execPath, ["scripts/check-cdx-cdxml-field-ledger.mjs"]);
const generatedBefore = generatedEngineSnapshot();
run(process.execPath, ["scripts/build-engine-wasm.mjs"]);
run(process.execPath, ["scripts/desktop-hybrid-latency-regression.mjs"]);
run(process.execPath, ["--check", "viewer/app.js"]);
run(process.execPath, [
  "--test",
  "scripts/tests/ccjz-container.test.mjs",
  "scripts/tests/recovery-journal.test.mjs",
  "scripts/tests/ccjs-v02-view.test.mjs",
  "scripts/tests/link-interaction.test.mjs",
  "scripts/tests/nmr-prediction-host.test.mjs",
  "scripts/tests/nmr-prediction-provider.test.mjs",
  "scripts/tests/nmr-prediction-e2e.test.mjs",
  "scripts/tests/public-cdxml-failure-ledger.test.mjs",
  "scripts/tests/public-cdxml-impact.test.mjs",
  "scripts/tests/public-cdxml-visual-gate.test.mjs",
]);

const generatedAfter = generatedEngineSnapshot();
const generatedChanges = changedGeneratedFiles(generatedBefore, generatedAfter);
const blockingChanges = generatedChanges.filter(
  (path) => !(process.env.CI === "true" && path === "chemsema_engine_bg.wasm"),
);

if (blockingChanges.length) {
  console.error("Generated viewer engine artifacts changed during verification:");
  for (const path of blockingChanges) {
    console.error(`  viewer/engine/${path}`);
  }
  console.error("Run `npm run build:engine-wasm` and include the refreshed artifacts.");
  process.exit(1);
}
if (generatedChanges.length) {
  console.warn(
    "CI rebuilt viewer/engine/chemsema_engine_bg.wasm with platform-local binary metadata; non-binary generated files are clean.",
  );
}
