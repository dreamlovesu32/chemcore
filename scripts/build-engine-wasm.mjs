import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism, homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const wasmPackToolchain = JSON.parse(
  readFileSync(join(rootDir, "tools", "wasm-pack.json"), "utf8"),
);

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
  const jobs = process.env.CHEMSEMA_BUILD_JOBS
    || process.env.CARGO_BUILD_JOBS
    || String(Math.max(1, availableParallelism()));
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
    CARGO_BUILD_JOBS: jobs,
  };
}

function assertWasmPackVersion() {
  const result = spawnSync("wasm-pack", ["--version"], {
    cwd: rootDir,
    encoding: "utf8",
    shell: false,
  });
  if (result.error) {
    console.error(
      `wasm-pack ${wasmPackToolchain.version} is required; install the pinned version declared in tools/wasm-pack.json.`,
    );
    process.exit(1);
  }
  if (result.status !== 0) {
    process.stdout.write(result.stdout || "");
    process.stderr.write(result.stderr || "");
    process.exit(result.status ?? 1);
  }
  const match = /^wasm-pack\s+(\S+)\s*$/.exec(result.stdout.trim());
  const actual = match?.[1] || "unknown";
  if (actual !== wasmPackToolchain.version) {
    console.error(
      `wasm-pack ${wasmPackToolchain.version} is required by tools/wasm-pack.json; found ${actual}.`,
    );
    process.exit(1);
  }
}

function normalizeGeneratedText(filePath) {
  const content = readFileSync(filePath, "utf8").replace(/\r\n/g, "\n");
  writeFileSync(filePath, content.endsWith("\n") ? content : `${content}\n`);
}

assertWasmPackVersion();
run("wasm-pack", [
  "build",
  "--target",
  "web",
  "--out-dir",
  join(rootDir, "viewer", "engine"),
  // Keep local builds deterministic even when wasm-pack's bundled wasm-opt is unavailable or misconfigured.
  "--no-opt",
  join(rootDir, "crates", "chemsema-engine"),
  "--features",
  "wasm",
], { env: wasmBuildEnv() });

// wasm-pack writes an ignore-all file for publishable packages. In this repo the
// viewer consumes these runtime artifacts directly, so they need to stay tracked.
rmSync(join(rootDir, "viewer", "engine", ".gitignore"), { force: true });
for (const fileName of ["package.json", "LICENSE"]) {
  normalizeGeneratedText(join(rootDir, "viewer", "engine", fileName));
}

const viewerSharedDir = join(rootDir, "viewer", "shared");
mkdirSync(viewerSharedDir, { recursive: true });
for (const fileName of ["glyph_profiles.json", "text_symbols.json"]) {
  copyFileSync(join(rootDir, "shared", fileName), join(viewerSharedDir, fileName));
}
