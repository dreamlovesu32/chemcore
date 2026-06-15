import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    env: options.env,
    shell: false,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function wasmBuildEnv() {
  const remapPrefixes = [
    [rootDir, "."],
    [process.env.CARGO_HOME ?? join(homedir(), ".cargo"), "$CARGO_HOME"],
    [process.env.RUSTUP_HOME ?? join(homedir(), ".rustup"), "$RUSTUP_HOME"],
  ];
  const remapFlags = remapPrefixes.map(
    ([from, to]) => `--remap-path-prefix=${from}=${to}`,
  );
  const encodedFlags = [
    process.env.CARGO_ENCODED_RUSTFLAGS,
    ...remapFlags,
  ].filter(Boolean);
  return {
    ...process.env,
    CARGO_ENCODED_RUSTFLAGS: encodedFlags.join("\x1f"),
  };
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
], { env: wasmBuildEnv() });

// wasm-pack writes an ignore-all file for publishable packages. In this repo the
// viewer consumes these runtime artifacts directly, so they need to stay tracked.
rmSync(join(rootDir, "viewer", "engine", ".gitignore"), { force: true });

const viewerSharedDir = join(rootDir, "viewer", "shared");
mkdirSync(viewerSharedDir, { recursive: true });
for (const fileName of ["glyph_profiles.json", "text_symbols.json"]) {
  copyFileSync(join(rootDir, "shared", fileName), join(viewerSharedDir, fileName));
}
