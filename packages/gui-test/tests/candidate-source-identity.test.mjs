import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  repositoryRoot,
  verifyDesktopCandidateManifest,
  writeDesktopCandidateManifest,
} from "../../../scripts/candidate-source-identity.mjs";

test("desktop candidate manifest binds both executable bytes and the current source closure", () => {
  const root = mkdtempSync(join(tmpdir(), "chemsema-candidate-identity-"));
  const candidatePath = join(root, "candidate.exe");
  const manifestPath = join(root, "candidate.build-manifest.json");
  const sourceIdentity = () => ({ sha256: "a".repeat(64), fileCount: 42 });
  try {
    writeFileSync(candidatePath, "candidate-v1");
    const manifest = writeDesktopCandidateManifest({ candidatePath, manifestPath, sourceIdentity });
    assert.equal(manifest.sourceSha256, "a".repeat(64));
    assert.equal(
      verifyDesktopCandidateManifest({ candidatePath, manifestPath, sourceIdentity }).candidateSha256,
      manifest.candidateSha256,
    );

    writeFileSync(candidatePath, "candidate-v2");
    assert.throws(
      () => verifyDesktopCandidateManifest({ candidatePath, manifestPath, sourceIdentity }),
      /candidate bytes do not match/i,
    );

    writeFileSync(candidatePath, "candidate-v1");
    assert.throws(
      () => verifyDesktopCandidateManifest({
        candidatePath,
        manifestPath,
        sourceIdentity: () => ({ sha256: "b".repeat(64), fileCount: 42 }),
      }),
      /stale for the current source closure/i,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fast desktop candidates rebuild the WASM included by the WebView", () => {
  const source = readFileSync(join(repositoryRoot, "scripts", "desktop-tauri-fast.mjs"), "utf8");
  assert.match(source, /beforeBuildCommand:\s*"node \.\.\/\.\.\/scripts\/build-engine-wasm\.mjs"/);
  assert.doesNotMatch(source, /CHEMSEMA_FAST_BUILD_WASM/);
});
