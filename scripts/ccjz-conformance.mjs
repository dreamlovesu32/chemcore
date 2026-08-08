import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { decodeCcjz, encodeCcjz } from "../viewer/ccjz_container.js";

const attachmentBytes = new TextEncoder().encode("independent attachment reader\n");
const attachmentSha256 = createHash("sha256").update(attachmentBytes).digest("hex");
const sample = {
  format: { name: "chemsema", version: "0.2", unit: "pt", profile: "snapshot" },
  document: { title: "cross implementation" },
  entities: { scene: [{ id: "one", type: "text" }, { id: "two", type: "text" }] },
  hierarchy: { roots: ["one", "two"] },
  resources: {
    payload: { encoding: "utf8", data: "independent-reader" },
    spectrum: {
      type: "binary",
      data: {
        storage: "ccjz-attachment",
        mediaType: "application/octet-stream",
        byteLength: attachmentBytes.byteLength,
        sha256: attachmentSha256,
      },
    },
  },
};

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8", shell: false });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed:\n${result.stderr || result.stdout}`);
  return result.stdout;
}

const directory = mkdtempSync(join(tmpdir(), "chemsema-ccjz-conformance-"));
try {
  const source = join(directory, "source.ccjs");
  const browserArchive = join(directory, "browser.ccjz");
  const rustArchive = join(directory, "rust.ccjz");
  writeFileSync(source, JSON.stringify(sample));
  writeFileSync(browserArchive, await encodeCcjz(JSON.stringify(sample), {
    sceneChunkRecords: 1,
    attachments: [{
      id: "spectrum",
      mediaType: "application/octet-stream",
      extension: "bin",
      bytes: attachmentBytes,
    }],
  }));

  const rustDecoded = JSON.parse(run("cargo", ["run", "-q", "-p", "chemsema-container", "--example", "ccjz_tool", "--", "decode", browserArchive]));
  const pythonDecoded = JSON.parse(run("python", ["tools/ccjz_reader.py", browserArchive]));
  assert.deepEqual(rustDecoded, sample);
  assert.deepEqual(pythonDecoded, sample);

  run("cargo", ["run", "-q", "-p", "chemsema-container", "--example", "ccjz_tool", "--", "encode", source, rustArchive]);
  const rustBytes = readFileSync(rustArchive);
  assert.deepEqual(JSON.parse(await decodeCcjz(rustBytes)), sample);
  assert.deepEqual(JSON.parse(run("python", ["tools/ccjz_reader.py", rustArchive])), sample);
  console.log("[ccjz-conformance] ok (Rust, browser JavaScript, and independent Python reader)");
} finally {
  rmSync(directory, { recursive: true, force: true });
}
