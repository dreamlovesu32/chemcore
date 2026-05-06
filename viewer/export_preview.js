const JPEG_QUALITY = 0.94;
const MAX_CANVAS_SIDE = 4096;
const CSS_PX_TO_PT = 72 / 96;

export async function pdfPreviewBase64FromSvg(svgText) {
  const metrics = svgMetrics(svgText);
  const image = await imageFromSvg(svgText);
  const scale = Math.min(
    2,
    MAX_CANVAS_SIDE / Math.max(metrics.width, metrics.height),
  );
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(metrics.width * scale));
  canvas.height = Math.max(1, Math.round(metrics.height * scale));
  const context = canvas.getContext("2d", { alpha: false });
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  const jpegBytes = await canvasJpegBytes(canvas);
  const pdfBytes = buildSingleImagePdf({
    jpegBytes,
    imageWidth: canvas.width,
    imageHeight: canvas.height,
    pageWidthPt: Math.max(1, metrics.width * CSS_PX_TO_PT),
    pageHeightPt: Math.max(1, metrics.height * CSS_PX_TO_PT),
  });
  return uint8ToBase64(pdfBytes);
}

function svgMetrics(svgText) {
  const document = new DOMParser().parseFromString(svgText, "image/svg+xml");
  const root = document.documentElement;
  const viewBox = String(root.getAttribute("viewBox") || "")
    .trim()
    .split(/\s+/)
    .map(Number);
  const width = parseSvgLength(root.getAttribute("width")) || viewBox[2] || 600;
  const height = parseSvgLength(root.getAttribute("height")) || viewBox[3] || 400;
  return {
    width: Math.max(1, width),
    height: Math.max(1, height),
  };
}

function parseSvgLength(value) {
  if (!value) {
    return null;
  }
  const parsed = Number.parseFloat(String(value));
  return Number.isFinite(parsed) ? parsed : null;
}

async function imageFromSvg(svgText) {
  const blob = new Blob([svgText], { type: "image/svg+xml" });
  const url = URL.createObjectURL(blob);
  try {
    const image = new Image();
    image.decoding = "async";
    image.src = url;
    await image.decode();
    return image;
  } finally {
    URL.revokeObjectURL(url);
  }
}

async function canvasJpegBytes(canvas) {
  const blob = await new Promise((resolve, reject) => {
    canvas.toBlob((result) => {
      if (result) {
        resolve(result);
      } else {
        reject(new Error("Failed to render PDF preview image."));
      }
    }, "image/jpeg", JPEG_QUALITY);
  });
  return new Uint8Array(await blob.arrayBuffer());
}

function buildSingleImagePdf({
  jpegBytes,
  imageWidth,
  imageHeight,
  pageWidthPt,
  pageHeightPt,
}) {
  const content = `q\n${formatPdfNumber(pageWidthPt)} 0 0 ${formatPdfNumber(pageHeightPt)} 0 0 cm\n/Im0 Do\nQ\n`;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>\n",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n",
    `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${formatPdfNumber(pageWidthPt)} ${formatPdfNumber(pageHeightPt)}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>\n`,
    concatBytes(
      asciiBytes(`<< /Type /XObject /Subtype /Image /Width ${imageWidth} /Height ${imageHeight} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${jpegBytes.length} >>\nstream\n`),
      jpegBytes,
      asciiBytes("\nendstream\n"),
    ),
    `<< /Length ${asciiBytes(content).length} >>\nstream\n${content}endstream\n`,
  ];

  const chunks = [asciiBytes("%PDF-1.4\n%\xE2\xE3\xCF\xD3\n")];
  const offsets = [0];
  for (let index = 0; index < objects.length; index += 1) {
    offsets.push(byteLength(chunks));
    chunks.push(asciiBytes(`${index + 1} 0 obj\n`));
    chunks.push(typeof objects[index] === "string" ? asciiBytes(objects[index]) : objects[index]);
    chunks.push(asciiBytes("endobj\n"));
  }
  const xrefOffset = byteLength(chunks);
  const xref = [
    "xref\n",
    `0 ${objects.length + 1}\n`,
    "0000000000 65535 f \n",
    ...offsets.slice(1).map((offset) => `${String(offset).padStart(10, "0")} 00000 n \n`),
    "trailer\n",
    `<< /Size ${objects.length + 1} /Root 1 0 R >>\n`,
    "startxref\n",
    `${xrefOffset}\n`,
    "%%EOF\n",
  ].join("");
  chunks.push(asciiBytes(xref));
  return concatBytes(...chunks);
}

function formatPdfNumber(value) {
  return Number(value).toFixed(2).replace(/\.?0+$/, "");
}

function asciiBytes(value) {
  return new Uint8Array(Array.from(String(value), (character) => character.charCodeAt(0) & 0xff));
}

function concatBytes(...chunks) {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const merged = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    merged.set(chunk, offset);
    offset += chunk.length;
  }
  return merged;
}

function byteLength(chunks) {
  return chunks.reduce((sum, chunk) => sum + chunk.length, 0);
}

function uint8ToBase64(bytes) {
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return btoa(binary);
}
