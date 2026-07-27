import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const gates = [
  "verify-chemdraw-one-substrate-geometry.mjs",
  "verify-chemdraw-dna-geometry.mjs",
  "verify-chemdraw-helix-geometry.mjs",
  "verify-chemdraw-micelle-tail-geometry.mjs",
];

const failures = [];
for (const gate of gates) {
  const result = spawnSync(process.execPath, [path.join(root, "scripts", gate)], {
    cwd: root,
    encoding: "utf8",
  });
  process.stdout.write(result.stdout ?? "");
  process.stderr.write(result.stderr ?? "");
  if (result.status !== 0) failures.push(gate);
}

if (failures.length > 0) {
  throw new Error(`BioShape geometry gates failed: ${failures.join(", ")}`);
}

console.log(`[BIOSHAPE GEOMETRY] ${gates.length} rule gates passed`);
