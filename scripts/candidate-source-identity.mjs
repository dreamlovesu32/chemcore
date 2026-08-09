import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const repositoryRoot = dirname(dirname(fileURLToPath(import.meta.url)));
export const defaultDesktopCandidatePath = join(repositoryRoot, "target", "release", "chemsema-desktop.exe");
export const defaultDesktopManifestPath = join(repositoryRoot, "target", "release", "chemsema-desktop.build-manifest.json");

const exactInputs = new Set([
  "Cargo.lock",
  "Cargo.toml",
  "package-lock.json",
  "package.json",
  "scripts/build-engine-wasm.mjs",
  "scripts/candidate-source-identity.mjs",
  "scripts/desktop-tauri-fast.mjs",
  "scripts/desktop-tauri.mjs",
]);
const inputPrefixes = ["apps/chemsema-desktop/", "crates/", "viewer/"];

function repositoryInputPaths(rootDir) {
  const result = spawnSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], {
    cwd: rootDir,
    encoding: "buffer",
    shell: false,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`Cannot enumerate desktop candidate inputs: ${String(result.stderr || "git ls-files failed")}`);
  }
  return result.stdout
    .toString("utf8")
    .split("\0")
    .filter(Boolean)
    .map((path) => path.replaceAll("\\", "/"))
    .filter((path) => exactInputs.has(path) || inputPrefixes.some((prefix) => path.startsWith(prefix)))
    .filter((path) => existsSync(join(rootDir, path)))
    .sort();
}

export function currentDesktopSourceIdentity(rootDir = repositoryRoot) {
  const paths = repositoryInputPaths(rootDir);
  const hash = createHash("sha256");
  for (const path of paths) {
    hash.update(path, "utf8");
    hash.update("\0");
    hash.update(readFileSync(join(rootDir, path)));
    hash.update("\0");
  }
  return { sha256: hash.digest("hex"), fileCount: paths.length };
}

function fileSha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

export function writeDesktopCandidateManifest({
  candidatePath = defaultDesktopCandidatePath,
  manifestPath = defaultDesktopManifestPath,
  sourceIdentity = currentDesktopSourceIdentity,
} = {}) {
  if (!existsSync(candidatePath)) throw new Error(`Desktop candidate does not exist: ${candidatePath}`);
  const source = sourceIdentity();
  const manifest = {
    schema: "chemsema.desktop-candidate-build.v1",
    candidate: relative(repositoryRoot, resolve(candidatePath)).replaceAll("\\", "/"),
    candidateSha256: fileSha256(candidatePath),
    sourceSha256: source.sha256,
    sourceFileCount: source.fileCount,
    builtAt: new Date().toISOString(),
  };
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return manifest;
}

export function verifyDesktopCandidateManifest({
  candidatePath = defaultDesktopCandidatePath,
  manifestPath = defaultDesktopManifestPath,
  sourceIdentity = currentDesktopSourceIdentity,
} = {}) {
  if (!existsSync(candidatePath)) throw new Error(`Desktop candidate does not exist: ${candidatePath}`);
  if (!existsSync(manifestPath)) {
    throw new Error("Desktop candidate build manifest is missing; run npm run desktop:build-fast before production GUI tests.");
  }
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (manifest.schema !== "chemsema.desktop-candidate-build.v1") {
    throw new Error("Desktop candidate build manifest has an unsupported schema.");
  }
  const candidateSha256 = fileSha256(candidatePath);
  if (manifest.candidateSha256 !== candidateSha256) {
    throw new Error("Desktop candidate bytes do not match the build manifest; rebuild before production GUI tests.");
  }
  const source = sourceIdentity();
  if (manifest.sourceSha256 !== source.sha256 || manifest.sourceFileCount !== source.fileCount) {
    throw new Error("Desktop candidate is stale for the current source closure; run npm run desktop:build-fast before production GUI tests.");
  }
  return { ...manifest, candidateSha256 };
}
