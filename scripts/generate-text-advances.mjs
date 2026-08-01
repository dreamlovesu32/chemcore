import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import opentype from "opentype.js";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outlinePath = join(repoRoot, "shared", "glyph_outlines.json");
const outputPath = join(repoRoot, "shared", "text_advances.json");
const fontRoot = resolve(process.env.CHEMSEMA_FONT_DIR || "C:/Windows/Fonts");
const outlines = JSON.parse(readFileSync(outlinePath, "utf8"));
const ascii = Array.from({ length: 95 }, (_, index) => String.fromCharCode(32 + index));

function u16(buffer, offset) {
  return buffer.readUInt16BE(offset);
}

function u32(buffer, offset) {
  return buffer.readUInt32BE(offset);
}

function extractFirstTtcFont(source) {
  if (source.toString("ascii", 0, 4) !== "ttcf") return source;
  const fontOffset = u32(source, 12);
  const tableCount = u16(source, fontOffset + 4);
  const headerSize = 12 + tableCount * 16;
  const records = [];
  let outputSize = headerSize;
  for (let index = 0; index < tableCount; index += 1) {
    const recordOffset = fontOffset + 12 + index * 16;
    const length = u32(source, recordOffset + 12);
    outputSize = (outputSize + 3) & ~3;
    records.push({ recordOffset, outputOffset: outputSize, length });
    outputSize += (length + 3) & ~3;
  }
  const output = Buffer.alloc(outputSize);
  source.copy(output, 0, fontOffset, fontOffset + headerSize);
  for (const record of records) {
    const sourceOffset = u32(source, record.recordOffset + 8);
    source.copy(
      output,
      record.outputOffset,
      sourceOffset,
      sourceOffset + record.length,
    );
    output.writeUInt32BE(record.outputOffset, record.recordOffset - fontOffset + 8);
  }
  return output;
}

function parseFont(path) {
  const source = extractFirstTtcFont(readFileSync(path));
  const arrayBuffer = source.buffer.slice(
    source.byteOffset,
    source.byteOffset + source.byteLength,
  );
  return opentype.parse(arrayBuffer);
}

const families = {};
for (const [familyName, family] of Object.entries(outlines.families)) {
  const faces = {};
  for (const [faceName, face] of Object.entries(family.faces)) {
    const font = parseFont(join(fontRoot, face.sourceFont));
    const advances = {};
    for (const character of ascii) {
      const glyph = font.charToGlyph(character);
      advances[character] = Number((glyph.advanceWidth / font.unitsPerEm).toFixed(8));
    }
    faces[faceName] = {
      sourceFont: face.sourceFont,
      advances,
    };
  }
  families[familyName] = { faces };
}

writeFileSync(
  outputPath,
  `${JSON.stringify({ version: 1, aliases: outlines.aliases, families })}\n`,
  "utf8",
);
console.log(outputPath);
