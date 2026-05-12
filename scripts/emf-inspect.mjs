import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RECORD_NAMES = new Map([
  [1, "EMR_HEADER"],
  [2, "EMR_POLYBEZIER"],
  [3, "EMR_POLYGON"],
  [4, "EMR_POLYLINE"],
  [5, "EMR_POLYBEZIERTO"],
  [6, "EMR_POLYLINETO"],
  [7, "EMR_POLYPOLYLINE"],
  [8, "EMR_POLYPOLYGON"],
  [9, "EMR_SETWINDOWEXTEX"],
  [10, "EMR_SETWINDOWORGEX"],
  [11, "EMR_SETVIEWPORTEXTEX"],
  [12, "EMR_SETVIEWPORTORGEX"],
  [17, "EMR_SETMAPMODE"],
  [18, "EMR_SETBKMODE"],
  [19, "EMR_SETPOLYFILLMODE"],
  [21, "EMR_SETSTRETCHBLTMODE"],
  [22, "EMR_SETTEXTALIGN"],
  [24, "EMR_SETTEXTCOLOR"],
  [25, "EMR_SETBKCOLOR"],
  [27, "EMR_MOVETOEX"],
  [33, "EMR_SAVEDC"],
  [34, "EMR_RESTOREDC"],
  [35, "EMR_SETWORLDTRANSFORM"],
  [36, "EMR_MODIFYWORLDTRANSFORM"],
  [37, "EMR_SELECTOBJECT"],
  [38, "EMR_CREATEPEN"],
  [39, "EMR_CREATEBRUSHINDIRECT"],
  [40, "EMR_DELETEOBJECT"],
  [42, "EMR_ELLIPSE"],
  [43, "EMR_RECTANGLE"],
  [44, "EMR_ROUNDRECT"],
  [54, "EMR_LINETO"],
  [56, "EMR_POLYDRAW"],
  [58, "EMR_SETMITERLIMIT"],
  [59, "EMR_BEGINPATH"],
  [60, "EMR_ENDPATH"],
  [61, "EMR_CLOSEFIGURE"],
  [62, "EMR_FILLPATH"],
  [63, "EMR_STROKEANDFILLPATH"],
  [64, "EMR_STROKEPATH"],
  [67, "EMR_SELECTCLIPPATH"],
  [70, "EMR_GDICOMMENT"],
  [76, "EMR_BITBLT"],
  [77, "EMR_STRETCHBLT"],
  [80, "EMR_SETDIBITSTODEVICE"],
  [81, "EMR_STRETCHDIBITS"],
  [82, "EMR_EXTCREATEFONTINDIRECTW"],
  [83, "EMR_EXTTEXTOUTA"],
  [84, "EMR_EXTTEXTOUTW"],
  [85, "EMR_POLYBEZIER16"],
  [86, "EMR_POLYGON16"],
  [87, "EMR_POLYLINE16"],
  [88, "EMR_POLYBEZIERTO16"],
  [89, "EMR_POLYLINETO16"],
  [90, "EMR_POLYPOLYLINE16"],
  [91, "EMR_POLYPOLYGON16"],
  [92, "EMR_POLYDRAW16"],
  [95, "EMR_EXTCREATEPEN"],
  [96, "EMR_POLYTEXTOUTA"],
  [97, "EMR_POLYTEXTOUTW"],
  [108, "EMR_SMALLTEXTOUT"],
  [98, "EMR_SETICMMODE"],
  [114, "EMR_ALPHABLEND"],
  [116, "EMR_TRANSPARENTBLT"],
  [118, "EMR_GRADIENTFILL"],
  [120, "EMR_SETTEXTJUSTIFICATION"],
]);

const EMFPLUS_COMMENT_IDENTIFIER = 0x2b464d45;

const EMFPLUS_RECORD_NAMES = new Map([
  [0x4001, "EmfPlusHeader"],
  [0x4002, "EmfPlusEndOfFile"],
  [0x4003, "EmfPlusComment"],
  [0x4004, "EmfPlusGetDC"],
  [0x4008, "EmfPlusObject"],
  [0x4009, "EmfPlusClear"],
  [0x400a, "EmfPlusFillRects"],
  [0x400b, "EmfPlusDrawRects"],
  [0x400c, "EmfPlusFillPolygon"],
  [0x400d, "EmfPlusDrawLines"],
  [0x400e, "EmfPlusFillEllipse"],
  [0x400f, "EmfPlusDrawEllipse"],
  [0x4014, "EmfPlusFillPath"],
  [0x4015, "EmfPlusDrawPath"],
  [0x4019, "EmfPlusDrawBeziers"],
  [0x401c, "EmfPlusDrawString"],
  [0x401d, "EmfPlusSetRenderingOrigin"],
  [0x401e, "EmfPlusSetAntiAliasMode"],
  [0x401f, "EmfPlusSetTextRenderingHint"],
  [0x4021, "EmfPlusSetInterpolationMode"],
  [0x4022, "EmfPlusSetPixelOffsetMode"],
  [0x4024, "EmfPlusSetCompositingQuality"],
  [0x4025, "EmfPlusSave"],
  [0x4026, "EmfPlusRestore"],
  [0x402a, "EmfPlusSetWorldTransform"],
  [0x402c, "EmfPlusMultiplyWorldTransform"],
  [0x402d, "EmfPlusTranslateWorldTransform"],
  [0x402e, "EmfPlusScaleWorldTransform"],
  [0x402f, "EmfPlusRotateWorldTransform"],
  [0x4030, "EmfPlusSetPageTransform"],
  [0x4031, "EmfPlusResetClip"],
  [0x4032, "EmfPlusSetClipRect"],
  [0x4033, "EmfPlusSetClipPath"],
  [0x4037, "EmfPlusStrokeFillPath"],
]);

function u32(buffer, offset) {
  return offset + 4 <= buffer.length ? buffer.readUInt32LE(offset) : null;
}

function i32(buffer, offset) {
  return offset + 4 <= buffer.length ? buffer.readInt32LE(offset) : null;
}

function f32(buffer, offset) {
  return offset + 4 <= buffer.length ? buffer.readFloatLE(offset) : null;
}

function rect(buffer, offset) {
  if (offset + 16 > buffer.length) return null;
  return {
    left: i32(buffer, offset),
    top: i32(buffer, offset + 4),
    right: i32(buffer, offset + 8),
    bottom: i32(buffer, offset + 12),
  };
}

function point(buffer, offset, short = false) {
  if (short) {
    if (offset + 4 > buffer.length) return null;
    return { x: buffer.readInt16LE(offset), y: buffer.readInt16LE(offset + 2) };
  }
  if (offset + 8 > buffer.length) return null;
  return { x: i32(buffer, offset), y: i32(buffer, offset + 4) };
}

function colorref(value) {
  if (value == null) return null;
  const r = value & 0xff;
  const g = (value >> 8) & 0xff;
  const b = (value >> 16) & 0xff;
  return `#${[r, g, b].map((part) => part.toString(16).padStart(2, "0")).join("")}`;
}

function readUtf16Z(buffer, offset, maxCodeUnits) {
  const chars = [];
  for (let i = 0; i < maxCodeUnits && offset + i * 2 + 1 < buffer.length; i += 1) {
    const code = buffer.readUInt16LE(offset + i * 2);
    if (code === 0) break;
    chars.push(code);
  }
  return String.fromCharCode(...chars);
}

function readText(buffer, recordOffset, recordSize, emrTextOffset, wide) {
  const nChars = u32(buffer, emrTextOffset + 8) ?? 0;
  const offString = u32(buffer, emrTextOffset + 12) ?? 0;
  const offDx = u32(buffer, emrTextOffset + 36) ?? 0;
  const stringOffset = recordOffset + offString;
  const byteLength = wide ? nChars * 2 : nChars;
  let text = "";
  if (stringOffset >= recordOffset && stringOffset + byteLength <= recordOffset + recordSize) {
    text = wide
      ? buffer.toString("utf16le", stringOffset, stringOffset + byteLength)
      : buffer.toString("latin1", stringOffset, stringOffset + byteLength);
  }
  return {
    reference: point(buffer, emrTextOffset),
    chars: nChars,
    options: u32(buffer, emrTextOffset + 16),
    bounds: rect(buffer, emrTextOffset + 20),
    hasDx: offDx !== 0,
    text,
  };
}

function decodeGdiComment(buffer, offset, size) {
  const dataSize = u32(buffer, offset + 8) ?? 0;
  const identifier = u32(buffer, offset + 12);
  const identifierText =
    offset + 16 <= buffer.length ? buffer.toString("latin1", offset + 12, offset + 16) : "";
  const comment = { dataSize, identifier, identifierText };
  if (identifier === EMFPLUS_COMMENT_IDENTIFIER && offset + 28 <= offset + size) {
    const recordType = buffer.readUInt16LE(offset + 16);
    const flags = buffer.readUInt16LE(offset + 18);
    const emfPlusSize = u32(buffer, offset + 20);
    const emfPlusDataSize = u32(buffer, offset + 24);
    Object.assign(comment, {
      emfPlus: {
        type: recordType,
        name: EMFPLUS_RECORD_NAMES.get(recordType) ?? `EmfPlus_0x${recordType.toString(16)}`,
        flags,
        size: emfPlusSize,
        dataSize: emfPlusDataSize,
      },
    });
  }
  return comment;
}

function decodeRecord(buffer, offset, type, size) {
  const name = RECORD_NAMES.get(type) ?? `EMR_${type}`;
  const info = {};
  if (type === 1) {
    Object.assign(info, {
      bounds: rect(buffer, offset + 8),
      frame: rect(buffer, offset + 24),
      signature: buffer.toString("latin1", offset + 40, Math.min(offset + 44, buffer.length)),
      version: u32(buffer, offset + 44),
      bytes: u32(buffer, offset + 48),
      records: u32(buffer, offset + 52),
      handles: buffer.readUInt16LE(offset + 56),
      descriptionChars: u32(buffer, offset + 64),
      descriptionOffset: u32(buffer, offset + 68),
      device: { cx: i32(buffer, offset + 72), cy: i32(buffer, offset + 76) },
      millimeters: { cx: i32(buffer, offset + 80), cy: i32(buffer, offset + 84) },
    });
  } else if ([3, 4, 2, 5, 6].includes(type)) {
    Object.assign(info, { bounds: rect(buffer, offset + 8), pointCount: u32(buffer, offset + 24) });
  } else if ([85, 86, 87, 88, 89].includes(type)) {
    Object.assign(info, { bounds: rect(buffer, offset + 8), pointCount: u32(buffer, offset + 24) });
  } else if ([42, 43, 44].includes(type)) {
    Object.assign(info, { bounds: rect(buffer, offset + 8) });
  } else if ([27, 54].includes(type)) {
    Object.assign(info, { point: point(buffer, offset + 8) });
  } else if (type === 37 || type === 40) {
    Object.assign(info, { object: u32(buffer, offset + 8) });
  } else if (type === 38) {
    Object.assign(info, {
      object: u32(buffer, offset + 8),
      style: u32(buffer, offset + 12),
      width: i32(buffer, offset + 16),
      color: colorref(u32(buffer, offset + 24)),
    });
  } else if (type === 39) {
    Object.assign(info, {
      object: u32(buffer, offset + 8),
      style: u32(buffer, offset + 12),
      color: colorref(u32(buffer, offset + 16)),
      hatch: u32(buffer, offset + 20),
    });
  } else if (type === 95) {
    const styleEntries = u32(buffer, offset + 48) ?? 0;
    const styles = [];
    for (let i = 0; i < styleEntries && offset + 52 + i * 4 + 4 <= offset + size; i += 1) {
      styles.push(u32(buffer, offset + 52 + i * 4));
    }
    Object.assign(info, {
      object: u32(buffer, offset + 8),
      penStyle: u32(buffer, offset + 28),
      width: u32(buffer, offset + 32),
      brushStyle: u32(buffer, offset + 36),
      color: colorref(u32(buffer, offset + 40)),
      styleEntries,
      styles,
    });
  } else if (type === 82) {
    Object.assign(info, {
      object: u32(buffer, offset + 8),
      height: i32(buffer, offset + 12),
      width: i32(buffer, offset + 16),
      escapement: i32(buffer, offset + 20),
      weight: i32(buffer, offset + 28),
      italic: buffer[offset + 32] === 1,
      underline: buffer[offset + 33] === 1,
      charset: buffer[offset + 35],
      face: readUtf16Z(buffer, offset + 40, 32),
    });
  } else if (type === 83 || type === 84) {
    Object.assign(info, {
      bounds: rect(buffer, offset + 8),
      graphicsMode: u32(buffer, offset + 24),
      scale: { x: f32(buffer, offset + 28), y: f32(buffer, offset + 32) },
      text: readText(buffer, offset, size, offset + 36, type === 84),
    });
  } else if ([17, 18, 19, 21, 22, 98].includes(type)) {
    Object.assign(info, { value: u32(buffer, offset + 8) });
  } else if (type === 24 || type === 25) {
    Object.assign(info, { color: colorref(u32(buffer, offset + 8)) });
  } else if (type === 70) {
    Object.assign(info, decodeGdiComment(buffer, offset, size));
  } else if (type === 81) {
    Object.assign(info, {
      bounds: rect(buffer, offset + 8),
      dest: {
        x: i32(buffer, offset + 24),
        y: i32(buffer, offset + 28),
        width: i32(buffer, offset + 72),
        height: i32(buffer, offset + 76),
      },
      source: {
        x: i32(buffer, offset + 32),
        y: i32(buffer, offset + 36),
        width: i32(buffer, offset + 40),
        height: i32(buffer, offset + 44),
      },
      bitmapInfoBytes: u32(buffer, offset + 52),
      bitmapBitsBytes: u32(buffer, offset + 60),
      rasterOperation: u32(buffer, offset + 68),
    });
  }
  return { index: null, offset, type, name, size, ...info };
}

export async function inspectEmf(inputPath, options = {}) {
  const buffer = await fs.readFile(inputPath);
  const records = [];
  const typeCounts = {};
  let offset = 0;
  while (offset + 8 <= buffer.length) {
    const type = u32(buffer, offset);
    const size = u32(buffer, offset + 4);
    if (!type || !size || size < 8 || offset + size > buffer.length) {
      records.push({ index: records.length, offset, type, name: "INVALID", size, invalid: true });
      break;
    }
    const record = decodeRecord(buffer, offset, type, size);
    record.index = records.length;
    records.push(record);
    typeCounts[record.name] = (typeCounts[record.name] ?? 0) + 1;
    offset += size;
    if (type === 14) break;
  }
  const header = records.find((record) => record.type === 1) ?? null;
  const interesting = records.filter((record) =>
    /POLY|TEXT|FONT|PEN|BRUSH|PATH|ELLIPSE|RECTANGLE|STRETCH|BITBLT|ALPHA|GRADIENT|WORLDTRANSFORM|CLIP|GDICOMMENT/.test(
      record.name
    )
  );
  const includeRecords = options.includeRecords ?? true;
  return {
    path: inputPath,
    bytes: buffer.length,
    parsedBytes: offset,
    header,
    recordCount: records.length,
    typeCounts,
    interesting: interesting.slice(0, options.maxInteresting ?? 200),
    records: includeRecords ? records : undefined,
  };
}

export function inspectionMarkdown(inspection) {
  const lines = [];
  lines.push(`# EMF Inspection`);
  lines.push("");
  lines.push(`- File: \`${inspection.path}\``);
  lines.push(`- Bytes: ${inspection.bytes}`);
  lines.push(`- Records: ${inspection.recordCount}`);
  if (inspection.header) {
    lines.push(`- Header bounds: \`${JSON.stringify(inspection.header.bounds)}\``);
    lines.push(`- Header frame: \`${JSON.stringify(inspection.header.frame)}\``);
    lines.push(`- Device mm: \`${JSON.stringify(inspection.header.millimeters)}\``);
  }
  lines.push("");
  lines.push(`## Record Counts`);
  lines.push("");
  for (const [name, count] of Object.entries(inspection.typeCounts).sort((a, b) => b[1] - a[1])) {
    lines.push(`- ${name}: ${count}`);
  }
  lines.push("");
  lines.push(`## Interesting Records`);
  lines.push("");
  for (const record of inspection.interesting.slice(0, 80)) {
    const shallow = { ...record };
    delete shallow.offset;
    delete shallow.size;
    lines.push(`- #${record.index} ${record.name}: \`${JSON.stringify(shallow)}\``);
  }
  lines.push("");
  return lines.join("\n");
}

function parseArgs(argv) {
  const args = { inputs: [], includeRecords: true };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--out") args.out = argv[++i];
    else if (arg === "--summary") args.summary = argv[++i];
    else if (arg === "--no-records") args.includeRecords = false;
    else if (arg === "--help" || arg === "-h") args.help = true;
    else args.inputs.push(arg);
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || args.inputs.length !== 1) {
    console.log("Usage: node scripts/emf-inspect.mjs [--out out.json] [--summary out.md] <file.emf>");
    return;
  }
  const inspection = await inspectEmf(args.inputs[0], { includeRecords: args.includeRecords });
  if (args.out) {
    await fs.mkdir(path.dirname(args.out), { recursive: true });
    await fs.writeFile(args.out, JSON.stringify(inspection, null, 2), "utf8");
  }
  const markdown = inspectionMarkdown(inspection);
  if (args.summary) {
    await fs.mkdir(path.dirname(args.summary), { recursive: true });
    await fs.writeFile(args.summary, markdown, "utf8");
  } else {
    console.log(markdown);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
