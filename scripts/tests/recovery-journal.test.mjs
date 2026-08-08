import assert from "node:assert/strict";
import test from "node:test";
import {
  appendRecoveryPatch,
  createRecoveryJournal,
  DurableRecoveryJournalStore,
  parseRecoveryJournal,
  recoveryJournalJsonl,
} from "../../viewer/recovery_journal.js";
import { createDocumentRecoveryManager, recoveryDocumentKey } from "../../viewer/document_recovery.js";

test("browser recovery journal verifies its hash chain", async () => {
  const journal = await createRecoveryJournal("ab".repeat(32));
  await appendRecoveryPatch(journal, { beforeRevision: 0, revision: 1, upsertEntities: [] });
  await appendRecoveryPatch(journal, { beforeRevision: 1, revision: 2, deletedEntityIds: ["a"] });
  const jsonl = recoveryJournalJsonl(journal);
  assert.deepEqual((await parseRecoveryJournal(jsonl)).journal, journal);
  await assert.rejects(
    () => parseRecoveryJournal(jsonl.replace("deletedEntityIds", "deletedEntityIxs")),
    /hash mismatch/,
  );
});

test("browser recovery ignores only an unterminated final record", async () => {
  const journal = await createRecoveryJournal("cd".repeat(32));
  await appendRecoveryPatch(journal, { beforeRevision: 0, revision: 1 });
  const truncated = `${recoveryJournalJsonl(journal)}{\"schema\":`;
  const recovered = await parseRecoveryJournal(truncated, { recoverTruncatedTail: true });
  assert.equal(recovered.journal.records.length, 1);
  assert.equal(recovered.ignoredTruncatedTail, true);
  await assert.rejects(() => parseRecoveryJournal(`${truncated}\n`, { recoverTruncatedTail: true }), /Invalid journal/);
});

test("document recovery manager compacts only after an explicit checkpoint", async () => {
  const values = new Map();
  const store = {
    get: async (key) => values.get(key) || null,
    put: async (key, value) => values.set(key, value),
    delete: async (key) => values.delete(key),
  };
  const manager = createDocumentRecoveryManager(store);
  const base = { format: { name: "chemsema", version: "0.2" }, document: { id: "doc" } };
  await manager.append("id:doc", base, { beforeRevision: 0, revision: 1 });
  assert.equal((await manager.recover("id:doc", base)).patches.length, 1);
  assert.equal((await manager.recover("id:doc", { ...base, changed: true })).baseMismatch, true);
  await manager.compact("id:doc");
  assert.equal(values.has("id:doc"), false);
});

test("desktop recovery uses a same-document sidecar while browser documents use IndexedDB", async () => {
  const sidecars = new Map();
  const browser = new Map();
  const desktop = {
    available: true,
    readRecoveryJournal: async (path) => sidecars.get(path) || null,
    writeRecoveryJournal: async (path, value) => sidecars.set(path, value),
    deleteRecoveryJournal: async (path) => sidecars.delete(path),
  };
  const browserStore = {
    get: async (key) => browser.get(key) || null,
    put: async (key, value) => browser.set(key, value),
    delete: async (key) => browser.delete(key),
  };
  const store = new DurableRecoveryJournalStore(desktop, browserStore);
  const pathKey = recoveryDocumentKey({ document: { id: "doc" } }, "D:\\Data\\sample.ccjz");
  assert.equal(pathKey, "path:D:/Data/sample.ccjz");
  await store.put(pathKey, "desktop journal");
  await store.put("id:untitled", "browser journal");
  assert.equal(sidecars.get("D:/Data/sample.ccjz"), "desktop journal");
  assert.equal(browser.get("id:untitled"), "browser journal");
  await store.delete(pathKey);
  assert.equal(sidecars.size, 0);
});
