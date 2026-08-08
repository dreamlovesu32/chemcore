# CCJZ Container v1

Status: current container contract for CCJS 0.2. Machine-readable manifest schema: [`schemas/ccjz-container-v1.schema.json`](../../schemas/ccjz-container-v1.schema.json).

CCJZ v1 is a deterministic ZIP container with MIME `application/vnd.chemsema.document+zip`. It is a storage protocol, not a second chemical document model. Successful assembly always produces a complete `chemsema/0.2` snapshot.

Required entries are, in order, uncompressed `mimetype`, `manifest.json`, and `document/root.json`. Scene records are ordered JSONL chunks under `entities/`; resources are content-addressed JSON entries under `resources/`. The manifest records the uncompressed size, SHA-256, and media type of every referenced entry. A scene-chunk descriptor may also carry conservative document-point `bounds` and stable `entityIds`; a writer must omit `bounds` if any record cannot be bounded safely. Paths must be normalized relative POSIX paths; traversal, absolute paths, backslashes, duplicate names, and case-fold collisions are errors.

Writers use stable JSON key ordering, LF, a fixed ZIP timestamp, stored entries, and lexicographic payload order. Equal semantic input therefore produces equal bytes within an implementation. Writers first write and flush a same-directory temporary file, reopen and verify it, then atomically replace the destination.

Readers enforce configured entry-count, per-entry, and total-expanded-size limits before extraction. They may read only the manifest, one scene chunk, one resource, or an attachment range through seek/range access. Browser readers and writers support Zip64 EOCD, locator, and entry metadata, while rejecting offsets or sizes beyond JavaScript's safe-integer range. Unknown manifest schemas are rejected. Legacy gzip `.ccjz` remains read-only compatible; current writers never create it.

Viewport consumers may load only chunks whose conservative bounds intersect the visible region; descriptors without bounds must be loaded. A save from a partial editing session must materialize all chunks first. Native saves may copy unchanged entries from the previous archive when path, size, and SHA-256 match, including opaque attachments, but must still write a new verified archive and atomically replace the previous file.

The normative implementation is `chemsema-container`. `viewer/ccjz_container.js` and `tools/ccjz_reader.py` are independent interoperability implementations. `npm run conformance:ccjz` exercises both directions across all three plus browser Zip64 and viewport-chunk behavior.
