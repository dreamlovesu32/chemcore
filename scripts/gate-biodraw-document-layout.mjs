import { spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));

function run(command, args, environment = {}) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    env: { ...process.env, ...environment },
    stdio: "inherit",
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run("cargo", ["test", "-p", "chemsema-engine", "--test", "bio_shapes"]);
run(process.execPath, ["scripts/chemdraw-bioshape-geometry-gate.mjs"]);
run(process.execPath, ["scripts/chemdraw-bioshape-visual-gate.mjs"]);
run(process.execPath, ["--test", "scripts/tests/document-layout.test.mjs"]);
run(process.execPath, ["scripts/plasmid-map-regression.mjs"]);
run(
  process.execPath,
  ["scripts/gui-regression.mjs"],
  { CHEMSEMA_GUI_CASE: "document-layout" },
);

console.log("[BIODRAW + DOCUMENT LAYOUT] native closure gate passed");
