import fs from "node:fs/promises";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    ours: null,
    reference: null,
    region: null,
    output: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--region") {
      const [left, top, right, bottom] = argv[++index]
        .split(",")
        .map((value) => Number.parseFloat(value.trim()));
      args.region = { left, top, right, bottom };
    } else if (arg === "--output") {
      args.output = argv[++index];
    } else if (!args.ours) {
      args.ours = arg;
    } else if (!args.reference) {
      args.reference = arg;
    }
  }
  return args;
}

function u16(buffer, offset) {
  return offset + 2 <= buffer.length ? buffer.readUInt16LE(offset) : null;
}

function u32(buffer, offset) {
  return offset + 4 <= buffer.length ? buffer.readUInt32LE(offset) : null;
}

function f32(buffer, offset) {
  return offset + 4 <= buffer.length ? buffer.readFloatLE(offset) : null;
}

function normalizeText(text) {
  return text.replaceAll(" ", "<sp>");
}

function extractDrawStrings(buffer, recordsJson, region) {
  const output = [];
  for (const record of recordsJson.records ?? []) {
    if (record?.name !== "EMR_GDICOMMENT" || record?.identifierText !== "EMF+") {
      continue;
    }
    const dataSize = u32(buffer, record.offset + 8) ?? 0;
    let cursor = record.offset + 16;
    const end = Math.min(record.offset + record.size, record.offset + 12 + dataSize);
    while (cursor + 12 <= end) {
      const type = u16(buffer, cursor);
      const flags = u16(buffer, cursor + 2);
      const size = u32(buffer, cursor + 4);
      const payloadSize = u32(buffer, cursor + 8);
      if (!size || size < 12 || cursor + size > end) break;
      if (type === 0x401c) {
        const charCount = u32(buffer, cursor + 20) ?? 0;
        const x = f32(buffer, cursor + 24);
        const y = f32(buffer, cursor + 28);
        const width = f32(buffer, cursor + 32);
        const height = f32(buffer, cursor + 36);
        const text = buffer.toString("utf16le", cursor + 40, cursor + 40 + charCount * 2);
        if (
          region &&
          (x < region.left || x > region.right || y < region.top || y > region.bottom)
        ) {
          cursor += size;
          continue;
        }
        output.push({
          recordIndex: record.index,
          flags,
          formatId: u32(buffer, cursor + 16),
          brushId: u32(buffer, cursor + 12),
          text,
          x,
          y,
          width,
          height,
        });
      }
      cursor += size;
    }
  }
  return output;
}

function alignSequences(ours, reference) {
  const rows = ours.length + 1;
  const cols = reference.length + 1;
  const dp = Array.from({ length: rows }, () => Array(cols).fill(0));
  for (let i = 1; i < rows; i += 1) {
    for (let j = 1; j < cols; j += 1) {
      if (ours[i - 1].text === reference[j - 1].text) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }
  const aligned = [];
  let i = ours.length;
  let j = reference.length;
  while (i > 0 || j > 0) {
    if (
      i > 0 &&
      j > 0 &&
      ours[i - 1].text === reference[j - 1].text &&
      dp[i][j] === dp[i - 1][j - 1] + 1
    ) {
      aligned.push({ ours: ours[i - 1], reference: reference[j - 1], kind: "match" });
      i -= 1;
      j -= 1;
      continue;
    }
    if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      aligned.push({ ours: null, reference: reference[j - 1], kind: "reference-only" });
      j -= 1;
      continue;
    }
    aligned.push({ ours: ours[i - 1], reference: null, kind: "ours-only" });
    i -= 1;
  }
  aligned.reverse();
  return aligned;
}

function buildMarkdown(args, aligned) {
  const lines = [];
  lines.push("# EmfPlusDrawString Compare");
  lines.push("");
  lines.push(`- ours: \`${args.ours}\``);
  lines.push(`- reference: \`${args.reference}\``);
  if (args.region) {
    lines.push(
      `- region: \`${args.region.left},${args.region.top},${args.region.right},${args.region.bottom}\``
    );
  }
  lines.push("");
  lines.push(
    "| kind | token | ours rec | ours x | ours y | ours w | ours h | ref rec | ref x | ref y | ref w | ref h | dx | dy | dw | dh |"
  );
  lines.push("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");

  let matched = 0;
  let dxSum = 0;
  let dySum = 0;

  for (const row of aligned) {
    const token = normalizeText(row.ours?.text ?? row.reference?.text ?? "");
    const dx =
      row.ours && row.reference ? row.ours.x - row.reference.x : null;
    const dy =
      row.ours && row.reference ? row.ours.y - row.reference.y : null;
    const dw =
      row.ours && row.reference ? row.ours.width - row.reference.width : null;
    const dh =
      row.ours && row.reference ? row.ours.height - row.reference.height : null;
    if (Number.isFinite(dx) && Number.isFinite(dy)) {
      matched += 1;
      dxSum += dx;
      dySum += dy;
    }
    lines.push(
      `| ${row.kind} | \`${token}\` | ${row.ours?.recordIndex ?? ""} | ${
        row.ours?.x?.toFixed(3) ?? ""
      } | ${row.ours?.y?.toFixed(3) ?? ""} | ${row.ours?.width?.toFixed(3) ?? ""} | ${
        row.ours?.height?.toFixed(3) ?? ""
      } | ${row.reference?.recordIndex ?? ""} | ${
        row.reference?.x?.toFixed(3) ?? ""
      } | ${row.reference?.y?.toFixed(3) ?? ""} | ${
        row.reference?.width?.toFixed(3) ?? ""
      } | ${row.reference?.height?.toFixed(3) ?? ""} | ${
        Number.isFinite(dx) ? dx.toFixed(3) : ""
      } | ${Number.isFinite(dy) ? dy.toFixed(3) : ""} |`
      + ` ${Number.isFinite(dw) ? dw.toFixed(3) : ""} | ${Number.isFinite(dh) ? dh.toFixed(3) : ""} |`
    );
  }

  lines.push("");
  lines.push("## Summary");
  lines.push("");
  lines.push(`- matched: ${matched}`);
  if (matched > 0) {
    lines.push(`- avg dx: ${(dxSum / matched).toFixed(3)}`);
    lines.push(`- avg dy: ${(dySum / matched).toFixed(3)}`);
  }

  return `${lines.join("\n")}\n`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.ours || !args.reference) {
    console.error(
      "Usage: node scripts/emf-drawstring-compare.mjs <ours.emf> <reference.emf> [--region left,top,right,bottom] [--output file.md]"
    );
    process.exit(1);
  }

  const oursPath = path.resolve(args.ours);
  const referencePath = path.resolve(args.reference);
  const [oursBuffer, referenceBuffer, oursJsonText, referenceJsonText] = await Promise.all([
    fs.readFile(oursPath),
    fs.readFile(referencePath),
    fs.readFile(`${oursPath}.records.json`, "utf8"),
    fs.readFile(`${referencePath}.records.json`, "utf8"),
  ]);

  const ours = extractDrawStrings(oursBuffer, JSON.parse(oursJsonText), args.region);
  const reference = extractDrawStrings(
    referenceBuffer,
    JSON.parse(referenceJsonText),
    args.region
  );
  const aligned = alignSequences(ours, reference);
  const markdown = buildMarkdown(args, aligned);
  if (args.output) {
    await fs.writeFile(path.resolve(args.output), markdown, "utf8");
  } else {
    process.stdout.write(markdown);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exit(1);
});
