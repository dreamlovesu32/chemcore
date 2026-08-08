# Recovery Journal v1

Status: current recovery-log contract for CCJS 0.2. Machine-readable record schema: [`schemas/journal-v1.schema.json`](../../schemas/journal-v1.schema.json).

The journal is LF-terminated JSONL outside the document snapshot: a same-directory sidecar on native files and an IndexedDB record stream in browsers. Every record carries `chemsema.journal.v1`, a one-based contiguous sequence, the base snapshot SHA-256, the preceding record SHA-256, one `chemsema.document.patch.v1` object, and its own SHA-256 computed from the canonical record without `recordSha256`.

Recovery verifies the base snapshot, sequence, base hash, hash chain, record hash, and patch revision before applying a record. A malformed durable record is corruption and stops recovery. Only an incomplete final record without LF may be ignored, with the ignored tail reported. After a new snapshot is durably saved and verified, compaction removes the old journal; the next edit starts a new journal bound to the new snapshot hash. Old records are never removed before that checkpoint succeeds.
