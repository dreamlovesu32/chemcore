import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourcePath = path.resolve(
  process.argv[2] ?? path.join(root, "tmp", "probe-circle-decoded.cdxml"),
);
const outDir = path.resolve(
  process.argv[3] ?? path.join(root, "tmp", "chemdraw-attached-label-position-probe"),
);

function attributes(startTag) {
  return Object.fromEntries(
    [...startTag.matchAll(/([A-Za-z][\w:-]*)="([^"]*)"/g)]
      .map((match) => [match[1], match[2]]),
  );
}

function shiftPair(value, dx, dy) {
  const numbers = value.trim().split(/\s+/).map(Number);
  if (numbers.length < 2 || numbers.some((number) => !Number.isFinite(number))) {
    throw new Error(`Expected an x/y pair, got ${value}`);
  }
  numbers[0] += dx;
  numbers[1] += dy;
  return numbers.join(" ");
}

function shiftBox(value, dx, dy) {
  const numbers = value.trim().split(/\s+/).map(Number);
  if (numbers.length < 4 || numbers.some((number) => !Number.isFinite(number))) {
    throw new Error(`Expected a bounding box, got ${value}`);
  }
  numbers[0] += dx;
  numbers[2] += dx;
  numbers[1] += dy;
  numbers[3] += dy;
  return numbers.join(" ");
}

function replaceAttribute(startTag, name, update) {
  const pattern = new RegExp(`(${name}=")([^"]*)(")`);
  if (!pattern.test(startTag)) throw new Error(`Missing ${name} in ${startTag}`);
  return startTag.replace(pattern, (_, before, value, after) =>
    `${before}${update(value)}${after}`);
}

function mutateFirstHcoLabel(source, { textDx = 0, textDy = 0, boxDx = 0, boxDy = 0 }) {
  const pattern = /<t\b[^>]*>\s*<s\b[^>]*>HCO<\/s>/;
  const match = source.match(pattern);
  if (!match) throw new Error("Could not find the first HCO label");
  let replacement = match[0];
  if (textDx || textDy) {
    replacement = replaceAttribute(replacement, "p", (value) =>
      shiftPair(value, textDx, textDy));
  }
  if (boxDx || boxDy) {
    replacement = replaceAttribute(replacement, "BoundingBox", (value) =>
      shiftBox(value, boxDx, boxDy));
  }
  return source.replace(pattern, replacement);
}

function mutateNodePosition(source, nodeId, dx, dy) {
  const pattern = new RegExp(`<n\\b[^>]*\\bid="${nodeId}"[^>]*>`);
  const match = source.match(pattern);
  if (!match) throw new Error(`Could not find node ${nodeId}`);
  const replacement = replaceAttribute(match[0], "p", (value) => shiftPair(value, dx, dy));
  return source.replace(pattern, replacement);
}

function firstHcoTextAttributes(cdxml) {
  const match = cdxml.match(/<t\b[^>]*>\s*<s\b[^>]*>HCO<\/s>/);
  return match ? attributes(match[0]) : null;
}

function firstHcoSvgTransform(svg) {
  const match = svg.match(
    /<text\b([^>]*)>\s*HCO<\/text>/,
  );
  return match ? attributes(`<text ${match[1]}>`).transform ?? null : null;
}

async function main() {
  const source = await fs.readFile(sourcePath, "utf8");
  const variants = [
    { name: "baseline", source },
    {
      name: "text-x-plus-8",
      source: mutateFirstHcoLabel(source, { textDx: 8 }),
    },
    {
      name: "text-y-plus-8",
      source: mutateFirstHcoLabel(source, { textDy: 8 }),
    },
    {
      name: "box-x-plus-8",
      source: mutateFirstHcoLabel(source, { boxDx: 8 }),
    },
    {
      name: "box-y-plus-8",
      source: mutateFirstHcoLabel(source, { boxDy: 8 }),
    },
    {
      name: "text-box-x-plus-8",
      source: mutateFirstHcoLabel(source, { textDx: 8, boxDx: 8 }),
    },
    {
      name: "node-x-plus-8",
      source: mutateNodePosition(source, "193", 8, 0),
    },
    {
      name: "node-y-plus-8",
      source: mutateNodePosition(source, "193", 0, 8),
    },
  ];
  const sourceDir = path.join(outDir, "sources");
  const oracleDir = path.join(outDir, "oracle");
  await fs.mkdir(sourceDir, { recursive: true });
  const inputs = [];
  for (const variant of variants) {
    const input = path.join(sourceDir, `${variant.name}.cdxml`);
    await fs.writeFile(input, variant.source, "utf8");
    inputs.push(input);
  }
  const jobs = await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["svg", "cdxml"],
    inputs,
  });
  const results = [];
  for (let index = 0; index < jobs.length; index += 1) {
    const [svg, savedCdxml] = await Promise.all([
      fs.readFile(jobs[index].outputs.svg, "utf8"),
      fs.readFile(jobs[index].outputs.cdxml, "utf8"),
    ]);
    results.push({
      name: variants[index].name,
      svgTransform: firstHcoSvgTransform(svg),
      savedText: firstHcoTextAttributes(savedCdxml),
    });
  }
  const report = {
    schema: "chemsema.chemdraw-attached-label-position-probe.v1",
    source: sourcePath,
    results,
  };
  const reportPath = path.join(outDir, "report.json");
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  console.log(JSON.stringify({ reportPath, results }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exit(1);
});
