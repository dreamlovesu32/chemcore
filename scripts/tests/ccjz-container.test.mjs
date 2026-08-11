import assert from "node:assert/strict";
import test from "node:test";
import {
  decodeCcjz, encodeCcjz, openCcjzBlob, openCcjzViewportSession,
} from "../../viewer/ccjz_container.js";

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

test("browser CCJZ reads and writes ZIP64 without losing range access", async () => {
  const bytes = await encodeCcjz(JSON.stringify(sample), {
    sceneChunkRecords: 2,
    forceZip64: true,
  });
  assert(bytes.some((_, index) => index + 4 <= bytes.length
    && new DataView(bytes.buffer, bytes.byteOffset + index, 4).getUint32(0, true) === 0x06064b50));
  assert.deepEqual(JSON.parse(await decodeCcjz(bytes)), sample);
  const lazy = await openCcjzBlob(new Blob([bytes]));
  assert.equal(lazy.manifest.sceneChunks.length, 2);
  assert.match(new TextDecoder().decode(await lazy.readSceneChunk(1)), /"id":"c"/);

  const unsafe = bytes.slice();
  const extra = unsafe.findIndex((_, index) => index + 4 <= unsafe.length
    && unsafe[index] === 0x01 && unsafe[index + 1] === 0x00
    && unsafe[index + 2] === 0x18 && unsafe[index + 3] === 0x00);
  assert(extra >= 0);
  new DataView(unsafe.buffer, unsafe.byteOffset).setBigUint64(extra + 4, 1n << 53n, true);
  await assert.rejects(() => decodeCcjz(unsafe), /safe integer range/);
});

test("browser CCJZ reader rejects tampered stored content", async () => {
  const bytes = await encodeCcjz(JSON.stringify(sample));
  const marker = new TextEncoder().encode("browser container");
  const index = bytes.findIndex((_, offset) => marker.every((byte, inner) => bytes[offset + inner] === byte));
  assert(index >= 0);
  bytes[index] ^= 1;
  await assert.rejects(() => decodeCcjz(bytes), /CRC mismatch/);
});

test("viewport session reads only chunks intersecting the visible region", async () => {
  const document = structuredClone(sample);
  document.entities.scene = [
    { id: "left", type: "molecule", zIndex: 1, transform: { translate: [0, 0] }, payload: { bbox: [0, 0, 10, 10] } },
    { id: "right", type: "molecule", zIndex: 2, transform: { translate: [1000, 0] }, payload: { bbox: [0, 0, 10, 10] } },
  ];
  document.hierarchy.roots = ["left", "right"];
  const bytes = await encodeCcjz(JSON.stringify(document), { sceneChunkRecords: 1 });
  const session = await openCcjzViewportSession(new Blob([bytes]));
  const first = await session.loadRegion([-20, -20, 20, 20]);
  assert.equal(first.loadedChunks, 1);
  assert.deepEqual(first.document.entities.scene.map((entity) => entity.id), ["left"]);
  const second = await session.loadRegion([980, -20, 1030, 20]);
  assert.equal(second.loadedChunks, 2);
  assert.deepEqual(second.document.entities.scene.map((entity) => entity.id), ["left", "right"]);
});

test("viewport hydration keeps loaded-region relation deletions authoritative", async () => {
  const document = structuredClone(sample);
  document.entities.scene = [
    { id: "left-a", type: "molecule", zIndex: 1, payload: { bbox: [0, 0, 10, 10] } },
    { id: "left-b", type: "molecule", zIndex: 2, payload: { bbox: [20, 0, 30, 10] } },
    { id: "right", type: "molecule", zIndex: 3, payload: { bbox: [1000, 0, 1010, 10] } },
  ];
  document.hierarchy.roots = ["left-a", "left-b", "right"];
  document.relations = [{
    id: "loaded-link",
    type: "link",
    endpoints: [{ entityId: "left-a" }, { entityId: "left-b" }],
  }];
  const bytes = await encodeCcjz(JSON.stringify(document), { sceneChunkRecords: 1 });
  const session = await openCcjzViewportSession(new Blob([bytes]));
  const loaded = await session.loadRegion([-10, -10, 40, 20]);
  assert.equal(loaded.document.relations.length, 1);
  loaded.document.relations = [];
  session.mergeEditedDocument(loaded.document);

  const complete = await session.loadRegion([990, -10, 1020, 20]);
  assert.equal(complete.loadedChunks, 3);
  assert.deepEqual(complete.document.relations, []);
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
