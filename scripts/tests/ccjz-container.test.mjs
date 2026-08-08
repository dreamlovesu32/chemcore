import assert from "node:assert/strict";
import test from "node:test";
import { decodeCcjz, encodeCcjz, openCcjzBlob } from "../../viewer/ccjz_container.js";

const sample = {
  format: { name: "chemsema", version: "0.2", unit: "pt", profile: "snapshot" },
  document: { title: "browser container" },
  entities: { scene: [{ id: "a", type: "text" }, { id: "b", type: "text" }, { id: "c", type: "text" }] },
  hierarchy: { roots: ["a", "b", "c"] },
  resources: { z: { encoding: "base64", data: "BBBB" }, a: { data: "AAAA", encoding: "base64" } },
};

test("browser CCJZ writer is deterministic and round trips chunks", async () => {
  const first = await encodeCcjz(JSON.stringify(sample), { sceneChunkRecords: 2 });
  const second = await encodeCcjz(JSON.stringify(sample), { sceneChunkRecords: 2 });
  assert.deepEqual(first, second);
  assert.equal(new TextDecoder().decode(first.slice(0, 2)), "PK");
  assert.deepEqual(JSON.parse(await decodeCcjz(first)), sample);
  const lazy = await openCcjzBlob(new Blob([first]));
  assert.equal(lazy.manifest.sceneChunks.length, 2);
  assert.match(new TextDecoder().decode(await lazy.readSceneChunk(1)), /"id":"c"/);
  assert.deepEqual(JSON.parse(new TextDecoder().decode(await lazy.readResource("a"))), sample.resources.a);
});

test("browser CCJZ reader rejects tampered stored content", async () => {
  const bytes = await encodeCcjz(JSON.stringify(sample));
  const marker = new TextEncoder().encode("browser container");
  const index = bytes.findIndex((_, offset) => marker.every((byte, inner) => bytes[offset + inner] === byte));
  assert(index >= 0);
  bytes[index] ^= 1;
  await assert.rejects(() => decodeCcjz(bytes), /CRC mismatch/);
});

test("browser CCJZ keeps opaque attachments content-addressed", async () => {
  const payload = new TextEncoder().encode("raw-fid");
  const hash = [...new Uint8Array(await crypto.subtle.digest("SHA-256", payload))]
    .map((value) => value.toString(16).padStart(2, "0")).join("");
  const document = structuredClone(sample);
  document.resources.fid = {
    type: "nmr-fid",
    encoding: "opaque",
    data: {
      storage: "ccjz-attachment",
      mediaType: "application/vnd.chemsema.nmr-fid",
      byteLength: payload.byteLength,
      sha256: hash,
    },
  };
  const bytes = await encodeCcjz(JSON.stringify(document), {
    attachments: [{
      id: "fid",
      mediaType: "application/vnd.chemsema.nmr-fid",
      extension: "fid",
      bytes: payload,
    }],
  });
  assert.deepEqual(JSON.parse(await decodeCcjz(bytes)), document);
  const lazy = await openCcjzBlob(new Blob([bytes]));
  assert.equal(new TextDecoder().decode(await lazy.readAttachmentRange("fid", 4, 3)), "fid");
});
