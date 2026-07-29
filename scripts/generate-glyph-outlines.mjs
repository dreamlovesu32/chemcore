import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// Keep one authoritative generator. This compatibility entry point delegates
// to the Python/fontTools implementation that emits the version-3 multi-face
// manifest consumed by the Rust kernel.
const scriptPath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "generate-glyph-outlines.py",
);
const result = spawnSync("python", [scriptPath, ...process.argv.slice(2)], {
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}
process.exitCode = result.status ?? 1;
