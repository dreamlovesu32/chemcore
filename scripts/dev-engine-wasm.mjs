import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const watchRoots = [
  join(rootDir, "Cargo.toml"),
  join(rootDir, "Cargo.lock"),
  join(rootDir, "crates", "chemcore-engine", "Cargo.toml"),
  join(rootDir, "crates", "chemcore-engine", "src"),
];

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

function watchedFiles(path) {
  const stats = statSync(path);
  if (stats.isFile()) {
    return shouldWatch(path) ? [path] : [];
  }
  const files = [];
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const childPath = join(path, entry.name);
    if (entry.isDirectory()) {
      files.push(...watchedFiles(childPath));
    } else if (entry.isFile() && shouldWatch(childPath)) {
      files.push(childPath);
    }
  }
  return files;
}

function shouldWatch(path) {
  return path.endsWith(".rs") || path.endsWith("Cargo.toml") || path.endsWith("Cargo.lock");
}

function hashInputs() {
  const hash = createHash("sha256");
  const files = watchRoots.flatMap(watchedFiles).sort((a, b) => a.localeCompare(b));
  for (const file of files) {
    hash.update(relative(rootDir, file).replaceAll("\\", "/"));
    hash.update("\0");
    hash.update(readFileSync(file));
    hash.update("\0");
  }
  return hash.digest("hex");
}

function buildEngine() {
  console.log("\n[dev:engine] rebuilding viewer engine wasm...");
  run(process.execPath, [join(rootDir, "scripts", "build-engine-wasm.mjs")]);
  console.log("[dev:engine] rebuild complete");
}

buildEngine();
let lastHash = hashInputs();

console.log("[dev:engine] watching Rust engine sources. Press Ctrl-C to stop.");
setInterval(() => {
  const nextHash = hashInputs();
  if (nextHash === lastHash) {
    return;
  }
  lastHash = nextHash;
  buildEngine();
}, 1000);
