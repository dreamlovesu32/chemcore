import { CCJZ_MIMETYPE, decodeCcjz, decodeCcjzBlob, encodeCcjz } from "./ccjz_container.js";

export const CHEMSEMA_TEXT_EXTENSION = ".ccjs";
export const CHEMSEMA_COMPRESSED_EXTENSION = ".ccjz";
export const CHEMSEMA_TEXT_MIME = "application/vnd.chemsema+json";
export const CHEMSEMA_COMPRESSED_MIME = CCJZ_MIMETYPE;
export const CHEMDRAW_CDX_MIME = "chemical/x-cdx";
export const MDL_SDF_MIME = "chemical/x-mdl-sdfile";

export function documentTitleForFileName(documentData) {
  const rawTitle = String(documentData?.document?.title || "chemsema-document").trim();
  const safeTitle = rawTitle
    .replace(/[\\/:*?"<>|]+/g, "-")
    .replace(/\s+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${safeTitle || "chemsema-document"}${CHEMSEMA_COMPRESSED_EXTENSION}`;
}

export function saveFormatFromFileName(fileName) {
  const lowerName = String(fileName || "").toLowerCase();
  if (lowerName.endsWith(CHEMSEMA_TEXT_EXTENSION)) {
    return "ccjs";
  }
  if (lowerName.endsWith(CHEMSEMA_COMPRESSED_EXTENSION)) {
    return "ccjz";
  }
  if (lowerName.endsWith(".svg")) {
    return "svg";
  }
  if (lowerName.endsWith(".cdxml")) {
    return "cdxml";
  }
  if (lowerName.endsWith(".cdx")) {
    return "cdx";
  }
  if (lowerName.endsWith(".sdf") || lowerName.endsWith(".sd")) {
    return "sdf";
  }
  return "ccjz";
}

export function baseNameWithoutDocumentExtension(fileName) {
  return String(fileName || "")
    .replace(/\.ccjz$/i, "")
    .replace(/\.ccjs$/i, "")
    .replace(/\.cdxml$/i, "")
    .replace(/\.cdx$/i, "")
    .replace(/\.sdf$/i, "")
    .replace(/\.sd$/i, "")
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

export function looksLikeCdxFile(file, bytes = null) {
  const name = (file?.name || "").toLowerCase();
  const type = (file?.type || "").toLowerCase();
  if (name.endsWith(".cdx") || type.includes("cdx")) {
    return true;
  }
  if (bytes && bytes.byteLength >= 8) {
    const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    return String.fromCharCode(...view.slice(0, 8)) === "VjCD0100";
  }
  return false;
}

export function looksLikeSdfFile(file, text = "") {
  const name = (file?.name || "").toLowerCase();
  const type = (file?.type || "").toLowerCase();
  if (name.endsWith(".sdf") || name.endsWith(".sd") || type.includes("mdl-sdfile") || type.includes("sdf")) {
    return true;
  }
  return /(?:^|\n)M  END(?:\n|\r\n?)\s*\$\$\$\$/i.test(text);
}

export function looksLikeCompressedChemSemaFile(file) {
  const name = (file?.name || "").toLowerCase();
  const type = (file?.type || "").toLowerCase();
  return name.endsWith(CHEMSEMA_COMPRESSED_EXTENSION)
    || type === CHEMSEMA_COMPRESSED_MIME
    || type.includes("gzip");
}

export async function compressChemSemaText(text) {
  return encodeCcjz(text);
}

export async function decompressChemSemaText(bytes) {
  if (bytes instanceof Blob) {
    return decodeCcjzBlob(bytes);
  }
  return decodeCcjz(bytes);
}

export function chemsemaOpenAcceptTypes() {
  return [{
    description: "ChemSema, ChemDraw, or SDF",
    accept: {
      [CHEMSEMA_COMPRESSED_MIME]: [CHEMSEMA_COMPRESSED_EXTENSION],
      [CHEMSEMA_TEXT_MIME]: [CHEMSEMA_TEXT_EXTENSION],
      "text/xml": [".cdxml"],
      "application/xml": [".cdxml"],
      "application/x-cdxml": [".cdxml"],
      "chemical/x-cdxml": [".cdxml"],
      "application/vnd.cambridgesoft.cdxml": [".cdxml"],
      [CHEMDRAW_CDX_MIME]: [".cdx"],
      "application/x-cdx": [".cdx"],
      "application/vnd.cambridgesoft.cdx": [".cdx"],
      [MDL_SDF_MIME]: [".sdf", ".sd"],
      "chemical/x-mdl-sdfile": [".sdf", ".sd"],
      "chemical/x-sdf": [".sdf", ".sd"],
    },
  }];
}

export function chemsemaOpenAcceptString() {
  return [
    CHEMSEMA_COMPRESSED_EXTENSION,
    CHEMSEMA_TEXT_EXTENSION,
    ".cdxml",
    ".cdx",
    CHEMSEMA_COMPRESSED_MIME,
    CHEMSEMA_TEXT_MIME,
    "text/xml",
    "application/xml",
    "application/x-cdxml",
    "chemical/x-cdxml",
    "application/vnd.cambridgesoft.cdxml",
    CHEMDRAW_CDX_MIME,
    "application/x-cdx",
    "application/vnd.cambridgesoft.cdx",
    ".sdf",
    ".sd",
    MDL_SDF_MIME,
    "chemical/x-mdl-sdfile",
    "chemical/x-sdf",
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
