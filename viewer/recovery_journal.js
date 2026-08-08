export const RECOVERY_JOURNAL_SCHEMA = "chemsema.journal.v1";

const encoder = new TextEncoder();

function canonicalValue(value) {
  if (Array.isArray(value)) return value.map(canonicalValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalValue(value[key])]));
  }
  return value;
}

async function sha256Hex(value) {
  const bytes = value instanceof Uint8Array ? value : encoder.encode(value);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function canonicalJson(value) {
  return JSON.stringify(canonicalValue(value));
}

function validateHash(label, value) {
  if (!/^[0-9a-fA-F]{64}$/.test(value || "")) throw new Error(`Journal ${label} SHA-256 must contain 64 hex digits.`);
}

function unsignedRecord(sequence, baseDocumentSha256, previousRecordSha256, patch) {
  return {
    schema: RECOVERY_JOURNAL_SCHEMA,
    sequence,
    baseDocumentSha256,
    ...(previousRecordSha256 ? { previousRecordSha256 } : {}),
    patch,
  };
}

export async function documentSha256(document) {
  return sha256Hex(canonicalJson(document));
}

export async function createRecoveryJournal(baseDocumentSha256) {
  validateHash("base document", baseDocumentSha256);
  return { baseDocumentSha256, records: [] };
}

export async function appendRecoveryPatch(journal, patch) {
  if (!patch || typeof patch !== "object" || Array.isArray(patch)) throw new Error("Journal patch must be a JSON object.");
  const sequence = journal.records.length + 1;
  const previousRecordSha256 = journal.records.at(-1)?.recordSha256 || null;
  const unsigned = unsignedRecord(sequence, journal.baseDocumentSha256, previousRecordSha256, patch);
  const record = { ...unsigned, recordSha256: await sha256Hex(canonicalJson(unsigned)) };
  journal.records.push(record);
  return record;
}

export function recoveryJournalJsonl(journal) {
  return journal.records.map((record) => canonicalJson(record)).join("\n") + (journal.records.length ? "\n" : "");
}

async function verifyRecord(record, previous) {
  if (record.schema !== RECOVERY_JOURNAL_SCHEMA) throw new Error(`Unsupported journal schema '${record.schema}'.`);
  validateHash("base document", record.baseDocumentSha256);
  validateHash("record", record.recordSha256);
  const sequence = previous.length + 1;
  if (record.sequence !== sequence) throw new Error(`Journal sequence mismatch at ${sequence}.`);
  if (previous.length && record.baseDocumentSha256 !== previous[0].baseDocumentSha256) {
    throw new Error("Journal base document hash changed within the chain.");
  }
  const previousHash = previous.at(-1)?.recordSha256;
  if (record.previousRecordSha256 !== previousHash) throw new Error(`Journal previous hash mismatch at sequence ${sequence}.`);
  const unsigned = unsignedRecord(sequence, record.baseDocumentSha256, previousHash, record.patch);
  if (await sha256Hex(canonicalJson(unsigned)) !== record.recordSha256) {
    throw new Error(`Journal record hash mismatch at sequence ${sequence}.`);
  }
}

export async function parseRecoveryJournal(jsonl, { recoverTruncatedTail = false } = {}) {
  const text = String(jsonl || "");
  const finalLf = !text || text.endsWith("\n");
  const lines = text.split("\n");
  const records = [];
  let ignoredTruncatedTail = false;
  for (let index = 0; index < lines.length; index += 1) {
    if (!lines[index]) continue;
    let record;
    try {
      record = JSON.parse(lines[index]);
    } catch (error) {
      if (recoverTruncatedTail && !finalLf && index === lines.length - 1) {
        ignoredTruncatedTail = true;
        break;
      }
      throw new Error(`Invalid journal record ${index + 1}: ${error.message}`);
    }
    await verifyRecord(record, records);
    records.push(record);
  }
  if (!records.length) throw new Error("Journal contains no complete records.");
  return {
    journal: { baseDocumentSha256: records[0].baseDocumentSha256, records },
    ignoredTruncatedTail,
  };
}

export class IndexedDbRecoveryJournalStore {
  constructor({ databaseName = "ChemSemaRecoveryV1", storeName = "journals" } = {}) {
    this.databaseName = databaseName;
    this.storeName = storeName;
    this.databasePromise = null;
  }

  database() {
    if (!globalThis.indexedDB) throw new Error("IndexedDB is unavailable.");
    this.databasePromise ||= new Promise((resolve, reject) => {
      const request = indexedDB.open(this.databaseName, 1);
      request.onupgradeneeded = () => request.result.createObjectStore(this.storeName);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result);
    });
    return this.databasePromise;
  }

  async get(documentKey) {
    const database = await this.database();
    return new Promise((resolve, reject) => {
      const request = database.transaction(this.storeName, "readonly").objectStore(this.storeName).get(documentKey);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => resolve(request.result || null);
    });
  }

  async put(documentKey, jsonl) {
    const database = await this.database();
    return new Promise((resolve, reject) => {
      const transaction = database.transaction(this.storeName, "readwrite");
      transaction.objectStore(this.storeName).put(jsonl, documentKey);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error || new Error("Recovery journal transaction aborted."));
    });
  }

  async delete(documentKey) {
    const database = await this.database();
    return new Promise((resolve, reject) => {
      const transaction = database.transaction(this.storeName, "readwrite");
      transaction.objectStore(this.storeName).delete(documentKey);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    });
  }
}

export class DurableRecoveryJournalStore {
  constructor(desktopFileHost, browserStore = new IndexedDbRecoveryJournalStore()) {
    this.desktopFileHost = desktopFileHost;
    this.browserStore = browserStore;
  }

  desktopPath(documentKey) {
    return this.desktopFileHost?.available && documentKey.startsWith("path:")
      ? documentKey.slice(5)
      : null;
  }

  async get(documentKey) {
    const path = this.desktopPath(documentKey);
    return path
      ? this.desktopFileHost.readRecoveryJournal(path)
      : this.browserStore.get(documentKey);
  }

  async put(documentKey, jsonl) {
    const path = this.desktopPath(documentKey);
    return path
      ? this.desktopFileHost.writeRecoveryJournal(path, jsonl)
      : this.browserStore.put(documentKey, jsonl);
  }

  async delete(documentKey) {
    const path = this.desktopPath(documentKey);
    return path
      ? this.desktopFileHost.deleteRecoveryJournal(path)
      : this.browserStore.delete(documentKey);
  }
}
