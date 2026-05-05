export const CHEMCORE_TEXT_EXTENSION = ".ccjs";
export const CHEMCORE_COMPRESSED_EXTENSION = ".ccjz";
export const CHEMCORE_TEXT_MIME = "application/vnd.chemcore+json";
export const CHEMCORE_COMPRESSED_MIME = "application/vnd.chemcore+gzip";

export function documentTitleForFileName(documentData) {
  const rawTitle = String(documentData?.document?.title || "chemcore-document").trim();
  const safeTitle = rawTitle
    .replace(/[\\/:*?"<>|]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${safeTitle || "chemcore-document"}${CHEMCORE_COMPRESSED_EXTENSION}`;
}

export function saveFormatFromFileName(fileName) {
  const lowerName = String(fileName || "").toLowerCase();
  if (lowerName.endsWith(CHEMCORE_TEXT_EXTENSION)) {
    return "ccjs";
  }
  if (lowerName.endsWith(CHEMCORE_COMPRESSED_EXTENSION)) {
    return "ccjz";
  }
  if (lowerName.endsWith(".svg")) {
    return "svg";
  }
  if (lowerName.endsWith(".cdxml")) {
    return "cdxml";
  }
  return "ccjz";
}

export function baseNameWithoutDocumentExtension(fileName) {
  return String(fileName || "")
    .replace(/\.ccjz$/i, "")
    .replace(/\.ccjs$/i, "")
    .replace(/\.cdxml$/i, "")
    .replace(/\.svg$/i, "");
}

export function looksLikeCdxmlFile(file, text) {
  const name = (file?.name || "").toLowerCase();
  const type = (file?.type || "").toLowerCase();
  if (name.endsWith(".cdxml") || type.includes("cdxml")) {
    return true;
  }
  return /^\s*(?:<\?xml[^>]*>\s*)?<CDXML\b/i.test(text);
}

export function looksLikeCompressedChemcoreFile(file) {
  const name = (file?.name || "").toLowerCase();
  const type = (file?.type || "").toLowerCase();
  return name.endsWith(CHEMCORE_COMPRESSED_EXTENSION) || type.includes("gzip");
}

export async function compressChemcoreText(text) {
  if (!globalThis.CompressionStream) {
    throw new Error("This browser cannot write compressed .ccjz files.");
  }
  const stream = new Blob([text], { type: CHEMCORE_TEXT_MIME })
    .stream()
    .pipeThrough(new CompressionStream("gzip"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

export async function decompressChemcoreText(bytes) {
  if (!globalThis.DecompressionStream) {
    throw new Error("This browser cannot open compressed .ccjz files.");
  }
  const stream = new Blob([bytes], { type: CHEMCORE_COMPRESSED_MIME })
    .stream()
    .pipeThrough(new DecompressionStream("gzip"));
  return new Response(stream).text();
}

export function chemcoreOpenAcceptTypes() {
  return [{
    description: "ChemCore CCJS/CCJZ or CDXML",
    accept: {
      [CHEMCORE_COMPRESSED_MIME]: [CHEMCORE_COMPRESSED_EXTENSION],
      [CHEMCORE_TEXT_MIME]: [CHEMCORE_TEXT_EXTENSION],
      "text/xml": [".cdxml"],
      "application/xml": [".cdxml"],
      "application/x-cdxml": [".cdxml"],
      "chemical/x-cdxml": [".cdxml"],
      "application/vnd.cambridgesoft.cdxml": [".cdxml"],
    },
  }];
}

export function chemcoreOpenAcceptString() {
  return [
    CHEMCORE_COMPRESSED_EXTENSION,
    CHEMCORE_TEXT_EXTENSION,
    ".cdxml",
    CHEMCORE_COMPRESSED_MIME,
    CHEMCORE_TEXT_MIME,
    "text/xml",
    "application/xml",
    "application/x-cdxml",
    "chemical/x-cdxml",
    "application/vnd.cambridgesoft.cdxml",
  ].join(",");
}

export function downloadTextFile(content, fileName, mimeType) {
  const blob = new Blob([content], { type: mimeType });
  downloadBlobFile(blob, fileName);
}

export function downloadBinaryFile(content, fileName, mimeType) {
  const blob = new Blob([content], { type: mimeType });
  downloadBlobFile(blob, fileName);
}

export function downloadBlobFile(blob, fileName) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}
