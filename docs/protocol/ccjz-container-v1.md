# CCJZ Container v1

Status: current container contract for CCJS 0.2. Machine-readable manifest schema: [`schemas/ccjz-container-v1.schema.json`](../../schemas/ccjz-container-v1.schema.json).

CCJZ v1 is a deterministic ZIP container with MIME `application/vnd.chemsema.document+zip`. It is a storage protocol, not a second chemical document model. Successful assembly always produces a complete `chemsema/0.2` snapshot.

Required entries are, in order, uncompressed `mimetype`, `manifest.json`, and `document/root.json`. Scene records are ordered JSONL chunks under `entities/`; resources are content-addressed JSON entries under `resources/`. The manifest records the uncompressed size, SHA-256, and media type of every referenced entry. Paths must be normalized relative POSIX paths; traversal, absolute paths, backslashes, duplicate names, and case-fold collisions are errors.

Writers use stable JSON key ordering, LF, a fixed ZIP timestamp, stored entries, and lexicographic payload order. Equal semantic input therefore produces equal bytes within an implementation. Writers first write and flush a same-directory temporary file, reopen and verify it, then atomically replace the destination.

Readers enforce configured entry-count, per-entry, and total-expanded-size limits before extraction. They may read only the manifest, one scene chunk, or one resource through seek/range access. Unknown manifest schemas are rejected. Legacy gzip `.ccjz` remains read-only compatible; current writers never create it.

The normative implementation is `chemsema-container`. `viewer/ccjz_container.js` and `tools/ccjz_reader.py` are independent interoperability implementations. `npm run conformance:ccjz` exercises both directions across all three.
