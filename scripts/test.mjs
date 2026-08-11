import { spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    shell: false,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

run("cargo", ["test"]);
run(process.execPath, ["--check", "viewer/app.js"]);
run(process.execPath, [
  "--test",
  "scripts/tests/ccjz-container.test.mjs",
  "scripts/tests/recovery-journal.test.mjs",
  "scripts/tests/ccjs-v02-view.test.mjs",
  "scripts/tests/engine-host-history.test.mjs",
  "scripts/tests/editor-context-menu-position.test.mjs",
  "scripts/tests/editor-command-history.test.mjs",
  "scripts/tests/editor-arrow-properties.test.mjs",
  "scripts/tests/editor-viewport-host.test.mjs",
  "scripts/tests/link-interaction.test.mjs",
  "scripts/tests/numeric-dialog-host.test.mjs",
  "scripts/tests/nmr-prediction-host.test.mjs",
  "scripts/tests/nmr-prediction-provider.test.mjs",
  "scripts/tests/nmr-prediction-e2e.test.mjs",
  "scripts/tests/public-cdxml-failure-ledger.test.mjs",
  "scripts/tests/public-cdxml-impact.test.mjs",
  "scripts/tests/public-cdxml-visual-gate.test.mjs",
  "packages/gui-test/tests/protocol.test.mjs",
  "packages/gui-test/tests/runner.test.mjs",
  "packages/gui-test/tests/hyperv.test.mjs",
]);
