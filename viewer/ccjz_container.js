export const CCJZ_MIMETYPE = "application/vnd.chemsema.document+zip";
export const CCJZ_CONTAINER_SCHEMA = "chemsema.container.v1";

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });
const ROOT_PATH = "document/root.json";

function canonicalValue(value) {
  if (Array.isArray(value)) return value.map(canonicalValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value).sort().map((key) => [key, canonicalValue(value[key])]),
    );
  }
  return value;
}

function jsonBytes(value) {
  return encoder.encode(JSON.stringify(canonicalValue(value)));
}

async function sha256Hex(bytes) {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((value) => value.toString(16).padStart(2, "0")).join("");
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function concatBytes(parts) {
  const length = parts.reduce((sum, part) => sum + part.byteLength, 0);
  const result = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.byteLength;
  }
  return result;
}

function setUint64(view, offset, value) {
  view.setBigUint64(offset, BigInt(value), true);
}

function safeZipNumber(value, label) {
  const number = Number(value);
  if (!Number.isSafeInteger(number)) throw new Error(`ZIP64 ${label} exceeds JavaScript safe integer range.`);
  return number;
}

function zip64Extra({ size, offset, includeSize, includeOffset }) {
  const dataLength = (includeSize ? 16 : 0) + (includeOffset ? 8 : 0);
  if (!dataLength) return new Uint8Array();
  const extra = new Uint8Array(4 + dataLength);
  const view = new DataView(extra.buffer);
  view.setUint16(0, 0x0001, true);
  view.setUint16(2, dataLength, true);
  let cursor = 4;
  if (includeSize) {
    setUint64(view, cursor, size);
    setUint64(view, cursor + 8, size);
    cursor += 16;
  }
  if (includeOffset) setUint64(view, cursor, offset);
  return extra;
}

function zipStored(entries, { forceZip64 = false } = {}) {
  const localParts = [];
  const centralParts = [];
  let offset = 0n;
  for (const [name, content] of entries) {
    const nameBytes = encoder.encode(name);
    const bytes = content instanceof Uint8Array ? content : new Uint8Array(content);
    const size = BigInt(bytes.byteLength);
    const size64 = forceZip64 || size > 0xffffffffn;
    const offset64 = forceZip64 || offset > 0xffffffffn;
    const localExtra = zip64Extra({ size, offset, includeSize: size64, includeOffset: false });
    const crc = crc32(bytes);
    const local = new Uint8Array(30 + nameBytes.byteLength + localExtra.byteLength);
    const localView = new DataView(local.buffer);
    localView.setUint32(0, 0x04034b50, true);
    localView.setUint16(4, size64 ? 45 : 20, true);
    localView.setUint16(6, 0x0800, true);
    localView.setUint16(8, 0, true);
    localView.setUint16(10, 0, true);
    localView.setUint16(12, 0x0021, true);
    localView.setUint32(14, crc, true);
    localView.setUint32(18, size64 ? 0xffffffff : bytes.byteLength, true);
    localView.setUint32(22, size64 ? 0xffffffff : bytes.byteLength, true);
    localView.setUint16(26, nameBytes.byteLength, true);
    localView.setUint16(28, localExtra.byteLength, true);
    local.set(nameBytes, 30);
    local.set(localExtra, 30 + nameBytes.byteLength);
    localParts.push(local, bytes);

    const centralExtra = zip64Extra({ size, offset, includeSize: size64, includeOffset: offset64 });
    const central = new Uint8Array(46 + nameBytes.byteLength + centralExtra.byteLength);
    const centralView = new DataView(central.buffer);
    centralView.setUint32(0, 0x02014b50, true);
    centralView.setUint16(4, (3 << 8) | (size64 || offset64 ? 45 : 20), true);
    centralView.setUint16(6, size64 || offset64 ? 45 : 20, true);
    centralView.setUint16(8, 0x0800, true);
    centralView.setUint16(10, 0, true);
    centralView.setUint16(12, 0, true);
    centralView.setUint16(14, 0x0021, true);
    centralView.setUint32(16, crc, true);
    centralView.setUint32(20, size64 ? 0xffffffff : bytes.byteLength, true);
    centralView.setUint32(24, size64 ? 0xffffffff : bytes.byteLength, true);
    centralView.setUint16(28, nameBytes.byteLength, true);
    centralView.setUint16(30, centralExtra.byteLength, true);
    centralView.setUint32(42, offset64 ? 0xffffffff : Number(offset), true);
    central.set(nameBytes, 46);
    central.set(centralExtra, 46 + nameBytes.byteLength);
    centralParts.push(central);
    offset += BigInt(local.byteLength + bytes.byteLength);
  }
  const centralOffset = offset;
  const centralSize = BigInt(centralParts.reduce((sum, part) => sum + part.byteLength, 0));
  const archive64 = forceZip64 || entries.length > 0xffff || centralOffset > 0xffffffffn || centralSize > 0xffffffffn;
  const trailer = [];
  if (archive64) {
    const zip64Offset = centralOffset + centralSize;
    const record = new Uint8Array(56);
    const recordView = new DataView(record.buffer);
    recordView.setUint32(0, 0x06064b50, true);
    setUint64(recordView, 4, 44);
    recordView.setUint16(12, (3 << 8) | 45, true);
    recordView.setUint16(14, 45, true);
    setUint64(recordView, 24, entries.length);
    setUint64(recordView, 32, entries.length);
    setUint64(recordView, 40, centralSize);
    setUint64(recordView, 48, centralOffset);
    const locator = new Uint8Array(20);
    const locatorView = new DataView(locator.buffer);
    locatorView.setUint32(0, 0x07064b50, true);
    setUint64(locatorView, 8, zip64Offset);
    locatorView.setUint32(16, 1, true);
    trailer.push(record, locator);
  }
  const eocd = new Uint8Array(22);
  const view = new DataView(eocd.buffer);
  view.setUint32(0, 0x06054b50, true);
  view.setUint16(8, archive64 ? 0xffff : entries.length, true);
  view.setUint16(10, archive64 ? 0xffff : entries.length, true);
  view.setUint32(12, archive64 ? 0xffffffff : Number(centralSize), true);
  view.setUint32(16, archive64 ? 0xffffffff : Number(centralOffset), true);
  return concatBytes([...localParts, ...centralParts, ...trailer, eocd]);
}

function validatePath(name) {
  if (!name || name.startsWith("/") || name.startsWith("\\") || name.includes("\\")
    || name.includes(":") || name.split("/").some((part) => !part || part === "." || part === "..")) {
    throw new Error(`Unsafe or non-canonical CCJZ entry name: ${name}`);
  }
}

function manifestDescriptors(manifest) {
  return [
    manifest.root,
    ...(manifest.sceneChunks || []),
    ...Object.values(manifest.resources || {}),
    ...Object.values(manifest.attachments || {}),
  ];
}

function validateManifestEntries(manifest, entryNames) {
  const declared = new Set(["mimetype", "manifest.json"]);
  for (const entry of manifestDescriptors(manifest)) {
    if (!entry || typeof entry.path !== "string" || typeof entry.mediaType !== "string"
      || !/^[0-9a-f]{64}$/.test(entry.sha256) || !Number.isSafeInteger(entry.size) || entry.size < 0) {
      throw new Error("Invalid CCJZ manifest entry descriptor.");
    }
    validatePath(entry.path);
    declared.add(entry.path);
  }
  for (const name of entryNames) {
    if (!declared.has(name)) throw new Error(`CCJZ contains an undeclared entry: ${name}`);
  }
  for (const name of declared) {
    if (!entryNames.includes(name)) throw new Error(`CCJZ manifest declares a missing entry: ${name}`);
  }
}

function validateAttachmentDescriptor(resource, id, entry) {
  const data = resource?.data;
  if (data?.storage !== "ccjz-attachment" || data?.mediaType !== entry.mediaType
    || data?.byteLength !== entry.size || data?.sha256 !== entry.sha256) {
    throw new Error(`CCJZ attachment resource '${id}' descriptor does not match its payload.`);
  }
}

function zip64CentralValues(bytes, extraStart, extraLength, size32, compressed32, offset32) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let cursor = extraStart;
  const end = extraStart + extraLength;
  while (cursor + 4 <= end) {
    const id = view.getUint16(cursor, true);
    const length = view.getUint16(cursor + 2, true);
    cursor += 4;
    if (cursor + length > end) throw new Error("CCJZ ZIP extra field is invalid.");
    if (id === 0x0001) {
      let valueCursor = cursor;
      const read = (needed, fallback, label) => {
        if (!needed) return fallback;
        if (valueCursor + 8 > cursor + length) throw new Error("CCJZ ZIP64 extra field is incomplete.");
        const value = safeZipNumber(view.getBigUint64(valueCursor, true), label);
        valueCursor += 8;
        return value;
      };
      const size = read(size32 === 0xffffffff, size32, "entry size");
      const compressedSize = read(compressed32 === 0xffffffff, compressed32, "compressed size");
      const localOffset = read(offset32 === 0xffffffff, offset32, "local offset");
      return { size, compressedSize, localOffset };
    }
    cursor += length;
  }
  if (size32 === 0xffffffff || compressed32 === 0xffffffff || offset32 === 0xffffffff) {
    throw new Error("CCJZ ZIP64 entry is missing its ZIP64 extra field.");
  }
  return { size: size32, compressedSize: compressed32, localOffset: offset32 };
}

function zipDirectoryFromBytes(bytes, view, eocd) {
  const count16 = view.getUint16(eocd + 10, true);
  const size32 = view.getUint32(eocd + 12, true);
  const offset32 = view.getUint32(eocd + 16, true);
  if (count16 !== 0xffff && size32 !== 0xffffffff && offset32 !== 0xffffffff) {
    return { count: count16, centralSize: size32, centralOffset: offset32 };
  }
  const locator = eocd - 20;
  if (locator < 0 || view.getUint32(locator, true) !== 0x07064b50) throw new Error("CCJZ ZIP64 locator is missing.");
  const zip64Offset = safeZipNumber(view.getBigUint64(locator + 8, true), "end record offset");
  if (zip64Offset + 56 > bytes.byteLength || view.getUint32(zip64Offset, true) !== 0x06064b50) {
    throw new Error("CCJZ ZIP64 end record is invalid.");
  }
  return {
    count: safeZipNumber(view.getBigUint64(zip64Offset + 32, true), "entry count"),
    centralSize: safeZipNumber(view.getBigUint64(zip64Offset + 40, true), "central size"),
    centralOffset: safeZipNumber(view.getBigUint64(zip64Offset + 48, true), "central offset"),
  };
}

function readStoredZip(input) {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let eocd = -1;
  for (let offset = Math.max(0, bytes.byteLength - 65557); offset <= bytes.byteLength - 22; offset += 1) {
    if (view.getUint32(offset, true) === 0x06054b50) eocd = offset;
  }
  if (eocd < 0) throw new Error("CCJZ ZIP end record is missing.");
  const { count, centralOffset } = zipDirectoryFromBytes(bytes, view, eocd);
  let cursor = centralOffset;
  const entries = new Map();
  const folded = new Set();
  for (let index = 0; index < count; index += 1) {
    if (cursor + 46 > bytes.byteLength || view.getUint32(cursor, true) !== 0x02014b50) {
      throw new Error("CCJZ ZIP central directory is invalid.");
    }
    const method = view.getUint16(cursor + 10, true);
    const expectedCrc = view.getUint32(cursor + 16, true);
    const compressed32 = view.getUint32(cursor + 20, true);
    const size32 = view.getUint32(cursor + 24, true);
    const nameLength = view.getUint16(cursor + 28, true);
    const extraLength = view.getUint16(cursor + 30, true);
    const commentLength = view.getUint16(cursor + 32, true);
    const offset32 = view.getUint32(cursor + 42, true);
    const { size, compressedSize, localOffset } = zip64CentralValues(
      bytes, cursor + 46 + nameLength, extraLength, size32, compressed32, offset32,
    );
    const name = decoder.decode(bytes.subarray(cursor + 46, cursor + 46 + nameLength));
    validatePath(name);
    if (entries.has(name) || folded.has(name.toLowerCase())) throw new Error(`Duplicate CCJZ entry: ${name}`);
    if (method !== 0 || compressedSize !== size) throw new Error(`Unsupported compressed CCJZ entry: ${name}`);
    if (localOffset + 30 > bytes.byteLength || view.getUint32(localOffset, true) !== 0x04034b50) {
      throw new Error(`Invalid CCJZ local entry: ${name}`);
    }
    const localNameLength = view.getUint16(localOffset + 26, true);
    const localExtraLength = view.getUint16(localOffset + 28, true);
    const start = localOffset + 30 + localNameLength + localExtraLength;
    const content = bytes.slice(start, start + size);
    if (content.byteLength !== size || crc32(content) !== expectedCrc) throw new Error(`CCJZ CRC mismatch: ${name}`);
    entries.set(name, content);
    folded.add(name.toLowerCase());
    cursor += 46 + nameLength + extraLength + commentLength;
  }
  return entries;
}

async function blobBytes(blob, start, length) {
  if (start < 0 || length < 0 || start + length > blob.size) {
    throw new Error("CCJZ entry points outside the Blob.");
  }
  return new Uint8Array(await blob.slice(start, start + length).arrayBuffer());
}

export async function openCcjzBlob(blob, {
  maxEntries = 100_000,
  maxEntryBytes = 2 * 1024 * 1024 * 1024,
  maxTotalBytes = 4 * 1024 * 1024 * 1024,
} = {}) {
  if (!(blob instanceof Blob)) throw new Error("openCcjzBlob requires a Blob or File.");
  const tailLength = Math.min(blob.size, 65557);
  const tail = await blobBytes(blob, blob.size - tailLength, tailLength);
  const tailView = new DataView(tail.buffer, tail.byteOffset, tail.byteLength);
  let eocd = -1;
  for (let offset = 0; offset <= tail.byteLength - 22; offset += 1) {
    if (tailView.getUint32(offset, true) === 0x06054b50) eocd = offset;
  }
  if (eocd < 0) throw new Error("CCJZ ZIP end record is missing.");
  let count = tailView.getUint16(eocd + 10, true);
  let centralSize = tailView.getUint32(eocd + 12, true);
  let centralOffset = tailView.getUint32(eocd + 16, true);
  if (count === 0xffff || centralSize === 0xffffffff || centralOffset === 0xffffffff) {
    const locator = eocd - 20;
    if (locator < 0 || tailView.getUint32(locator, true) !== 0x07064b50) {
      throw new Error("CCJZ ZIP64 locator is missing.");
    }
    const zip64Offset = safeZipNumber(tailView.getBigUint64(locator + 8, true), "end record offset");
    const record = await blobBytes(blob, zip64Offset, 56);
    const recordView = new DataView(record.buffer, record.byteOffset, record.byteLength);
    if (recordView.getUint32(0, true) !== 0x06064b50) throw new Error("CCJZ ZIP64 end record is invalid.");
    count = safeZipNumber(recordView.getBigUint64(32, true), "entry count");
    centralSize = safeZipNumber(recordView.getBigUint64(40, true), "central size");
    centralOffset = safeZipNumber(recordView.getBigUint64(48, true), "central offset");
  }
  if (count > maxEntries) throw new Error(`CCJZ has too many entries: ${count}`);
  const central = await blobBytes(blob, centralOffset, centralSize);
  const view = new DataView(central.buffer, central.byteOffset, central.byteLength);
  const entries = new Map();
  const folded = new Set();
  let cursor = 0;
  let total = 0;
  const order = [];
  for (let index = 0; index < count; index += 1) {
    if (cursor + 46 > central.byteLength || view.getUint32(cursor, true) !== 0x02014b50) {
      throw new Error("CCJZ ZIP central directory is invalid.");
    }
    const method = view.getUint16(cursor + 10, true);
    const expectedCrc = view.getUint32(cursor + 16, true);
    const compressed32 = view.getUint32(cursor + 20, true);
    const size32 = view.getUint32(cursor + 24, true);
    const nameLength = view.getUint16(cursor + 28, true);
    const extraLength = view.getUint16(cursor + 30, true);
    const commentLength = view.getUint16(cursor + 32, true);
    const offset32 = view.getUint32(cursor + 42, true);
    const { size, compressedSize, localOffset } = zip64CentralValues(
      central, cursor + 46 + nameLength, extraLength, size32, compressed32, offset32,
    );
    const name = decoder.decode(central.subarray(cursor + 46, cursor + 46 + nameLength));
    validatePath(name);
    if (entries.has(name) || folded.has(name.toLowerCase())) throw new Error(`Duplicate CCJZ entry: ${name}`);
    if (method !== 0 || compressedSize !== size) throw new Error(`Unsupported compressed CCJZ entry: ${name}`);
    if (size > maxEntryBytes) throw new Error(`CCJZ entry exceeds configured limit: ${name}`);
    total += size;
    if (!Number.isSafeInteger(total) || total > maxTotalBytes) throw new Error("CCJZ expanded size exceeds configured limit.");
    entries.set(name, { name, localOffset, size, expectedCrc });
    folded.add(name.toLowerCase());
    order.push(name);
    cursor += 46 + nameLength + extraLength + commentLength;
  }
  if (order[0] !== "mimetype") throw new Error("CCJZ mimetype must be the first ZIP entry.");

  async function entryRange(name) {
    const metadata = entries.get(name);
    if (!metadata) throw new Error(`Missing CCJZ entry: ${name}`);
    const header = await blobBytes(blob, metadata.localOffset, 30);
    const headerView = new DataView(header.buffer, header.byteOffset, header.byteLength);
    if (headerView.getUint32(0, true) !== 0x04034b50) throw new Error(`Invalid CCJZ local entry: ${name}`);
    const nameLength = headerView.getUint16(26, true);
    const extraLength = headerView.getUint16(28, true);
    return { start: metadata.localOffset + 30 + nameLength + extraLength, ...metadata };
  }

  async function readEntry(name) {
    const metadata = await entryRange(name);
    const content = await blobBytes(blob, metadata.start, metadata.size);
    if (crc32(content) !== metadata.expectedCrc) throw new Error(`CCJZ CRC mismatch: ${name}`);
    return content;
  }

  if (decoder.decode(await readEntry("mimetype")) !== CCJZ_MIMETYPE) {
    throw new Error("CCJZ mimetype entry is not canonical.");
  }
  const manifest = JSON.parse(decoder.decode(await readEntry("manifest.json")));
  if (manifest.schema !== CCJZ_CONTAINER_SCHEMA || manifest.mediaType !== CCJZ_MIMETYPE
    || manifest.documentFormat !== "chemsema/0.2") throw new Error("Unsupported CCJZ manifest header.");
  validateManifestEntries(manifest, order);

  async function readVerifiedDescriptor(entry) {
    const bytes = await readEntry(entry.path);
    if (bytes.byteLength !== entry.size) throw new Error(`CCJZ entry size mismatch: ${entry.path}`);
    if (await sha256Hex(bytes) !== entry.sha256) throw new Error(`CCJZ entry SHA-256 mismatch: ${entry.path}`);
    return bytes;
  }

  return {
    manifest,
    entryNames: () => [...order],
    readEntry,
    readRoot: () => readVerifiedDescriptor(manifest.root),
    readSceneChunk: (index) => {
      const entry = manifest.sceneChunks?.[index];
      if (!entry) throw new Error(`CCJZ scene chunk index ${index} is out of range.`);
      return readVerifiedDescriptor(entry);
    },
    readResource: (id) => {
      const entry = manifest.resources?.[id];
      if (!entry) throw new Error(`Unknown CCJZ resource id '${id}'.`);
      return readVerifiedDescriptor(entry);
    },
    readAttachment: (id) => {
      const entry = manifest.attachments?.[id];
      if (!entry) throw new Error(`Unknown CCJZ attachment id '${id}'.`);
      return readVerifiedDescriptor(entry);
    },
    readAttachmentRange: async (id, offset, length) => {
      const entry = manifest.attachments?.[id];
      if (!entry) throw new Error(`Unknown CCJZ attachment id '${id}'.`);
      if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length)
        || offset < 0 || length < 0 || offset + length > entry.size) {
        throw new Error(`CCJZ attachment range is out of bounds: ${id}`);
      }
      const range = await entryRange(entry.path);
      return blobBytes(blob, range.start + offset, length);
    },
  };
}

export async function decodeCcjzBlob(blob, options = {}) {
  const prefix = await blobBytes(blob, 0, Math.min(2, blob.size));
  if (prefix[0] === 0x1f && prefix[1] === 0x8b) {
    if (!globalThis.DecompressionStream) throw new Error("This browser cannot open legacy gzip CCJZ files.");
    const stream = blob.stream().pipeThrough(new DecompressionStream("gzip"));
    return new Response(stream).text();
  }
  const reader = await openCcjzBlob(blob, options);
  const root = JSON.parse(decoder.decode(await reader.readRoot()));
  if (!Array.isArray(root?.entities?.scene) || root.entities.scene.length) throw new Error("Invalid CCJZ scene root.");
  let first = 0;
  for (let index = 0; index < (reader.manifest.sceneChunks || []).length; index += 1) {
    const descriptor = reader.manifest.sceneChunks[index];
    if (descriptor.firstRecord !== first) throw new Error("CCJZ scene chunks are not contiguous.");
    const text = decoder.decode(await reader.readSceneChunk(index));
    const records = text.split("\n").filter(Boolean).map((line) => JSON.parse(line));
    if (records.length !== descriptor.recordCount) throw new Error(`CCJZ scene record count mismatch: ${descriptor.path}`);
    root.entities.scene.push(...records);
    first += records.length;
  }
  if (!root.resources) throw new Error("Invalid CCJZ resource root.");
  for (const id of Object.keys(reader.manifest.resources || {}).sort()) {
    if (Object.hasOwn(root.resources, id)) throw new Error(`Duplicate CCJZ resource: ${id}`);
    root.resources[id] = JSON.parse(decoder.decode(await reader.readResource(id)));
  }
  for (const [id, entry] of Object.entries(reader.manifest.attachments || {})) {
    validateAttachmentDescriptor(root.resources[id], id, entry);
  }
  validateHeader(root);
  return JSON.stringify(canonicalValue(root));
}

function boundsIntersect(left, right) {
  return left[0] <= right[2] && left[2] >= right[0]
    && left[1] <= right[3] && left[3] >= right[1];
}

function collectResourceRefs(value, refs = new Set()) {
  if (Array.isArray(value)) {
    for (const item of value) collectResourceRefs(item, refs);
  } else if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) {
      if ((key === "resourceRef" || key.endsWith("ResourceRef")) && typeof child === "string") {
        refs.add(child);
      }
      collectResourceRefs(child, refs);
    }
  }
  return refs;
}

export async function openCcjzViewportSession(blob, options = {}) {
  const reader = await openCcjzBlob(blob, options);
  const root = JSON.parse(decoder.decode(await reader.readRoot()));
  if (!Array.isArray(root?.entities?.scene) || root.entities.scene.length) {
    throw new Error("Invalid CCJZ scene root.");
  }
  const loadedChunks = new Set();
  const entities = new Map();
  const resources = new Map(Object.entries(root.resources || {}));
  const exposedIds = new Set();

  async function loadResourceClosure(records) {
    const refs = collectResourceRefs(records);
    for (const id of refs) {
      if (resources.has(id)) continue;
      if (reader.manifest.resources?.[id]) {
        resources.set(id, JSON.parse(decoder.decode(await reader.readResource(id))));
      }
    }
  }

  async function loadChunk(index) {
    if (loadedChunks.has(index)) return false;
    const descriptor = reader.manifest.sceneChunks?.[index];
    if (!descriptor) return false;
    const text = decoder.decode(await reader.readSceneChunk(index));
    const records = text.split("\n").filter(Boolean).map((line) => JSON.parse(line));
    if (records.length !== descriptor.recordCount) {
      throw new Error(`CCJZ scene record count mismatch: ${descriptor.path}`);
    }
    const recordIds = records.map((record) => record.id).filter((id) => typeof id === "string");
    if (descriptor.entityIds?.length
      && (descriptor.entityIds.length !== recordIds.length
        || descriptor.entityIds.some((id, offset) => id !== recordIds[offset]))) {
      throw new Error(`CCJZ scene entityIds mismatch: ${descriptor.path}`);
    }
    const actualBounds = chunkSpatialMetadata(records).bounds || null;
    if (descriptor.bounds && actualBounds
      && (descriptor.bounds[0] > actualBounds[0] + 1e-6
        || descriptor.bounds[1] > actualBounds[1] + 1e-6
        || descriptor.bounds[2] < actualBounds[2] - 1e-6
        || descriptor.bounds[3] < actualBounds[3] - 1e-6)) {
      throw new Error(`CCJZ scene bounds are not conservative: ${descriptor.path}`);
    }
    for (const record of records) {
      entities.set(record.id, record);
      exposedIds.add(record.id);
    }
    await loadResourceClosure(records);
    loadedChunks.add(index);
    return true;
  }

  function loadedDocument() {
    const documentData = structuredClone(root);
    const loadedIds = new Set(entities.keys());
    documentData.entities.scene = [...entities.values()]
      .sort((left, right) => (left.zIndex ?? 0) - (right.zIndex ?? 0));
    documentData.resources = Object.fromEntries([...resources.entries()].sort(([a], [b]) => a.localeCompare(b)));
    if (documentData.hierarchy) {
      documentData.hierarchy.roots = (documentData.hierarchy.roots || []).filter((id) => loadedIds.has(id));
      documentData.hierarchy.children = Object.fromEntries(
        Object.entries(documentData.hierarchy.children || {})
          .filter(([id]) => loadedIds.has(id))
          .map(([id, children]) => [id, children.filter((child) => loadedIds.has(child))]),
      );
      const children = new Set(Object.values(documentData.hierarchy.children).flat());
      for (const id of loadedIds) if (!children.has(id) && !documentData.hierarchy.roots.includes(id)) {
        documentData.hierarchy.roots.push(id);
      }
    }
    documentData.relations = (documentData.relations || []).filter((relation) =>
      (relation.endpoints || []).every((endpoint) => loadedIds.has(endpoint.entityId)));
    if (documentData.orders?.reading) {
      documentData.orders.reading = documentData.orders.reading.filter((id) => loadedIds.has(id));
    }
    documentData.document = documentData.document || {};
    documentData.document.layout = {
      ...(documentData.document.layout || {}),
      chunkLoading: {
        schema: "chemsema.chunk-loading.v1",
        loadedChunks: [...loadedChunks].sort((a, b) => a - b),
        totalChunks: reader.manifest.sceneChunks?.length || 0,
        complete: loadedChunks.size === (reader.manifest.sceneChunks?.length || 0),
      },
    };
    return documentData;
  }

  return {
    manifest: reader.manifest,
    documentBounds: [0, 0, Number(root.document?.page?.width || 612), Number(root.document?.page?.height || 792)],
    async loadRegion(bounds) {
      const region = numericBounds(bounds);
      if (!region) throw new Error("CCJZ visible region must be [minX,minY,maxX,maxY].");
      let newlyLoadedChunks = 0;
      for (let index = 0; index < (reader.manifest.sceneChunks || []).length; index += 1) {
        const descriptor = reader.manifest.sceneChunks[index];
        if (!descriptor.bounds || boundsIntersect(descriptor.bounds, region)) {
          if (await loadChunk(index)) newlyLoadedChunks += 1;
        }
      }
      return { document: loadedDocument(), newlyLoadedChunks, ...this.stats() };
    },
    async materialize() {
      for (let index = 0; index < (reader.manifest.sceneChunks || []).length; index += 1) {
        await loadChunk(index);
      }
      return loadedDocument();
    },
    mergeEditedDocument(documentData) {
      if (!documentData?.entities?.scene) return;
      const currentIds = new Set(documentData.entities.scene.map((entity) => entity.id));
      for (const id of [...entities.keys()]) if (!currentIds.has(id)) entities.delete(id);
      for (const entity of documentData.entities.scene) {
        entities.set(entity.id, structuredClone(entity));
        exposedIds.add(entity.id);
      }
      for (const [id, resource] of Object.entries(documentData.resources || {})) {
        resources.set(id, structuredClone(resource));
      }

      // Replace semantic indexes for the region the editor has actually
      // seen, while retaining original cross-chunk relationships whose other
      // endpoint is still unloaded. This makes local deletion authoritative
      // without discarding future relationships.
      const retainedRelations = (root.relations || []).filter((relation) =>
        (relation.endpoints || []).some((endpoint) => !exposedIds.has(endpoint.entityId)));
      root.relations = [
        ...retainedRelations,
        ...(documentData.relations || []).map((relation) => structuredClone(relation)),
      ];

      const originalHierarchy = root.hierarchy || { roots: [], children: {} };
      const editedHierarchy = documentData.hierarchy || { roots: [], children: {} };
      const mergedChildren = {};
      for (const [parent, children] of Object.entries(originalHierarchy.children || {})) {
        const retained = children.filter((child) => !exposedIds.has(parent) || !exposedIds.has(child));
        if (retained.length) mergedChildren[parent] = retained;
      }
      for (const [parent, children] of Object.entries(editedHierarchy.children || {})) {
        mergedChildren[parent] = [
          ...(mergedChildren[parent] || []),
          ...children,
        ].filter((id, index, values) => values.indexOf(id) === index);
      }
      root.hierarchy = {
        ...originalHierarchy,
        ...structuredClone(editedHierarchy),
        roots: [
          ...(originalHierarchy.roots || []).filter((id) => !exposedIds.has(id)),
          ...(editedHierarchy.roots || []),
        ].filter((id, index, values) => values.indexOf(id) === index),
        children: mergedChildren,
      };

      root.orders = root.orders || {};
      root.orders.reading = [
        ...((root.orders.reading || []).filter((id) => !exposedIds.has(id))),
        ...((documentData.orders?.reading || [])),
      ].filter((id, index, values) => values.indexOf(id) === index);
    },
    stats() {
      return {
        loadedChunks: loadedChunks.size,
        totalChunks: reader.manifest.sceneChunks?.length || 0,
        loadedEntities: entities.size,
      };
    },
  };
}

function descriptor(path, mediaType, bytes, hash) {
  return { path, mediaType, sha256: hash, size: bytes.byteLength };
}

function numericBounds(value) {
  return Array.isArray(value) && value.length >= 4 && value.slice(0, 4).every(Number.isFinite)
    && value[0] <= value[2] && value[1] <= value[3] ? value.slice(0, 4) : null;
}

function entityBounds(entity) {
  const payload = entity?.payload || {};
  const raw = numericBounds(payload.bbox) || numericBounds(payload.boundingBox)
    || numericBounds(payload.arrowGeometry?.boundingBox) || numericBounds(payload.geometry?.boundingBox)
    || numericBounds(payload.boxField);
  if (!raw) return null;
  const [tx = 0, ty = 0] = entity.transform?.translate || [];
  const [sx = 1, sy = 1] = entity.transform?.scale || [];
  const angle = Number(entity.transform?.rotate || 0) * Math.PI / 180;
  const sin = Math.sin(angle);
  const cos = Math.cos(angle);
  const corners = [[raw[0], raw[1]], [raw[2], raw[1]], [raw[2], raw[3]], [raw[0], raw[3]]]
    .map(([x, y]) => [x * sx * cos - y * sy * sin + tx, x * sx * sin + y * sy * cos + ty]);
  return corners.reduce((bounds, [x, y]) => [
    Math.min(bounds[0], x), Math.min(bounds[1], y), Math.max(bounds[2], x), Math.max(bounds[3], y),
  ], [Infinity, Infinity, -Infinity, -Infinity]);
}

function chunkSpatialMetadata(records) {
  const bounds = records.map(entityBounds);
  const union = bounds.every(Boolean) ? bounds.reduce((left, right) => [
    Math.min(left[0], right[0]), Math.min(left[1], right[1]),
    Math.max(left[2], right[2]), Math.max(left[3], right[3]),
  ]) : null;
  return {
    ...(union ? { bounds: union } : {}),
    entityIds: records.map((record) => record?.id).filter((id) => typeof id === "string"),
  };
}

function validateHeader(documentData) {
  const format = documentData?.format;
  if (format?.name !== "chemsema" || format?.version !== "0.2"
    || format?.unit !== "pt" || format?.profile !== "snapshot") {
    throw new Error("CCJZ requires canonical chemsema/0.2 pt snapshot input.");
  }
  if (!Array.isArray(documentData?.entities?.scene)) throw new Error("CCJS v0.2 requires entities.scene.");
}

export async function encodeCcjz(text, {
  sceneChunkRecords = 1024,
  attachments = [],
  forceZip64 = false,
} = {}) {
  if (!Number.isSafeInteger(sceneChunkRecords) || sceneChunkRecords < 1) {
    throw new Error("CCJZ scene chunk size must be a positive integer.");
  }
  const source = JSON.parse(text);
  validateHeader(source);
  const scene = source.entities.scene;
  const resources = source.resources || {};
  const attachmentIds = new Set(attachments.map((attachment) => attachment.id));
  if (attachmentIds.size !== attachments.length) throw new Error("CCJZ attachment ids must be unique.");
  const root = structuredClone(source);
  root.entities.scene = [];
  root.resources = Object.fromEntries(
    Object.entries(resources).filter(([id]) => attachmentIds.has(id)),
  );
  const rootBytes = jsonBytes(root);
  const rootHash = await sha256Hex(rootBytes);
  const payloads = new Map([[ROOT_PATH, rootBytes]]);
  const sceneChunks = [];
  for (let first = 0, index = 0; first < scene.length; first += sceneChunkRecords, index += 1) {
    const records = scene.slice(first, first + sceneChunkRecords);
    const path = `entities/scene-${String(index).padStart(6, "0")}.jsonl`;
    const bytes = encoder.encode(`${records.map((record) => JSON.stringify(canonicalValue(record))).join("\n")}\n`);
    const hash = await sha256Hex(bytes);
    sceneChunks.push({
      ...descriptor(path, "application/x-ndjson", bytes, hash),
      firstRecord: first,
      recordCount: records.length,
      ...chunkSpatialMetadata(records),
    });
    payloads.set(path, bytes);
  }
  const resourceDescriptors = {};
  for (const id of Object.keys(resources).sort()) {
    if (attachmentIds.has(id)) continue;
    const bytes = jsonBytes(resources[id]);
    const hash = await sha256Hex(bytes);
    const path = `resources/${hash}.json`;
    resourceDescriptors[id] = descriptor(path, "application/vnd.chemsema.resource+json", bytes, hash);
    payloads.set(path, bytes);
  }
  const attachmentDescriptors = {};
  for (const attachment of [...attachments].sort((left, right) => left.id.localeCompare(right.id))) {
    const bytes = attachment.bytes instanceof Uint8Array
      ? attachment.bytes
      : new Uint8Array(attachment.bytes);
    const mediaType = String(attachment.mediaType || "application/octet-stream");
    const extension = String(attachment.extension || "bin").replace(/^\./, "").toLowerCase();
    if (!/^[a-z0-9-]{1,16}$/.test(extension)) throw new Error(`Unsafe CCJZ attachment extension: ${extension}`);
    const hash = await sha256Hex(bytes);
    const data = resources[attachment.id]?.data;
    if (data?.storage !== "ccjz-attachment" || data?.mediaType !== mediaType
      || data?.byteLength !== bytes.byteLength || data?.sha256 !== hash) {
      throw new Error(`CCJZ attachment resource '${attachment.id}' descriptor does not match its payload.`);
    }
    const path = `resources/${hash}.${extension}`;
    attachmentDescriptors[attachment.id] = descriptor(path, mediaType, bytes, hash);
    payloads.set(path, bytes);
  }
  const manifest = {
    schema: CCJZ_CONTAINER_SCHEMA,
    mediaType: CCJZ_MIMETYPE,
    documentFormat: "chemsema/0.2",
    root: descriptor(ROOT_PATH, "application/json", rootBytes, rootHash),
    ...(sceneChunks.length ? { sceneChunks } : {}),
    ...(Object.keys(resourceDescriptors).length ? { resources: resourceDescriptors } : {}),
    ...(Object.keys(attachmentDescriptors).length ? { attachments: attachmentDescriptors } : {}),
  };
  return zipStored([
    ["mimetype", encoder.encode(CCJZ_MIMETYPE)],
    ["manifest.json", jsonBytes(manifest)],
    ...[...payloads.entries()].sort(([left], [right]) => left.localeCompare(right)),
  ], { forceZip64 });
}

async function verified(entries, entry) {
  validatePath(entry.path);
  const bytes = entries.get(entry.path);
  if (!bytes) throw new Error(`Missing CCJZ entry: ${entry.path}`);
  if (bytes.byteLength !== entry.size) throw new Error(`CCJZ entry size mismatch: ${entry.path}`);
  if (await sha256Hex(bytes) !== entry.sha256) throw new Error(`CCJZ entry SHA-256 mismatch: ${entry.path}`);
  return bytes;
}

export async function decodeCcjz(input) {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (bytes[0] === 0x1f && bytes[1] === 0x8b) {
    if (!globalThis.DecompressionStream) throw new Error("This browser cannot open legacy gzip CCJZ files.");
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
    return new Response(stream).text();
  }
  const entries = readStoredZip(bytes);
  if (decoder.decode(entries.get("mimetype") || new Uint8Array()) !== CCJZ_MIMETYPE) {
    throw new Error("CCJZ mimetype entry is not canonical.");
  }
  const manifest = JSON.parse(decoder.decode(entries.get("manifest.json") || new Uint8Array()));
  if (manifest.schema !== CCJZ_CONTAINER_SCHEMA || manifest.mediaType !== CCJZ_MIMETYPE
    || manifest.documentFormat !== "chemsema/0.2") throw new Error("Unsupported CCJZ manifest header.");
  validateManifestEntries(manifest, [...entries.keys()]);
  const root = JSON.parse(decoder.decode(await verified(entries, manifest.root)));
  if (!Array.isArray(root?.entities?.scene) || root.entities.scene.length) throw new Error("Invalid CCJZ scene root.");
  let first = 0;
  for (const chunk of manifest.sceneChunks || []) {
    if (chunk.firstRecord !== first) throw new Error("CCJZ scene chunks are not contiguous.");
    const text = decoder.decode(await verified(entries, chunk));
    const records = text.split("\n").filter(Boolean).map((line) => JSON.parse(line));
    if (records.length !== chunk.recordCount) throw new Error(`CCJZ scene record count mismatch: ${chunk.path}`);
    root.entities.scene.push(...records);
    first += records.length;
  }
  if (!root.resources) throw new Error("Invalid CCJZ resource root.");
  for (const id of Object.keys(manifest.resources || {}).sort()) {
    if (Object.hasOwn(root.resources, id)) throw new Error(`Duplicate CCJZ resource: ${id}`);
    root.resources[id] = JSON.parse(decoder.decode(await verified(entries, manifest.resources[id])));
  }
  for (const [id, entry] of Object.entries(manifest.attachments || {})) {
    validateAttachmentDescriptor(root.resources[id], id, entry);
    await verified(entries, entry);
  }
  validateHeader(root);
  return JSON.stringify(canonicalValue(root));
}
