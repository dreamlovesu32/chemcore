import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

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

run("wasm-pack", [
  "build",
  join(rootDir, "crates", "chemcore-engine"),
  "--target",
  "web",
  "--out-dir",
  join(rootDir, "viewer", "engine"),
  "--features",
  "wasm",
]);

// wasm-pack writes an ignore-all file for publishable packages. In this repo the
// viewer consumes these runtime artifacts directly, so they need to stay tracked.
rmSync(join(rootDir, "viewer", "engine", ".gitignore"), { force: true });

const viewerSharedDir = join(rootDir, "viewer", "shared");
mkdirSync(viewerSharedDir, { recursive: true });
for (const fileName of ["glyph_profiles.json", "text_symbols.json"]) {
  copyFileSync(join(rootDir, "shared", fileName), join(viewerSharedDir, fileName));
}
