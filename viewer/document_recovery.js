import {
  appendRecoveryPatch,
  createRecoveryJournal,
  documentSha256,
  parseRecoveryJournal,
  recoveryJournalJsonl,
} from "./recovery_journal.js";

export function recoveryDocumentKey(document, filePath = null, fileName = null) {
  const path = String(filePath || "").trim().replace(/\\/g, "/");
  if (path) return `path:${path}`;
  const id = String(document?.document?.id || "").trim();
  if (id) return `id:${id}`;
  const name = String(fileName || "").trim().toLowerCase();
  return name ? `name:${name}` : null;
}

export function createDocumentRecoveryManager(store) {
  async function append(documentKey, baseDocument, patch) {
    if (!documentKey) return false;
    const existing = await store.get(documentKey);
    const journal = existing
      ? (await parseRecoveryJournal(existing, { recoverTruncatedTail: true })).journal
      : await createRecoveryJournal(await documentSha256(baseDocument));
    await appendRecoveryPatch(journal, patch);
    await store.put(documentKey, recoveryJournalJsonl(journal));
    return true;
  }

  async function recover(documentKey, baseDocument) {
    if (!documentKey) return { patches: [], ignoredTruncatedTail: false };
    const jsonl = await store.get(documentKey);
    if (!jsonl) return { patches: [], ignoredTruncatedTail: false };
    const recovered = await parseRecoveryJournal(jsonl, { recoverTruncatedTail: true });
    const baseHash = await documentSha256(baseDocument);
    if (recovered.journal.baseDocumentSha256 !== baseHash) {
      return { patches: [], ignoredTruncatedTail: false, baseMismatch: true };
    }
    if (recovered.ignoredTruncatedTail) {
      await store.put(documentKey, recoveryJournalJsonl(recovered.journal));
    }
    return {
      patches: recovered.journal.records.map((record) => record.patch),
      ignoredTruncatedTail: recovered.ignoredTruncatedTail,
      baseMismatch: false,
    };
  }

  async function compact(documentKey) {
    if (!documentKey) return false;
    await store.delete(documentKey);
    return true;
  }

  return { append, recover, compact };
}
