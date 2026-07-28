import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.resolve(root, process.argv[2] ?? "tmp/chemdraw-arrow-outline-probe");
const sourceDir = path.join(outDir, "source");
const oracleDir = path.join(outDir, "chemdraw");

const cases = [
  { name: "solid-line-0_25", lineWidth: 0.25, head: "Full", type: "Solid" },
  { name: "solid-line-0_6", lineWidth: 0.6, head: "Full", type: "Solid" },
  { name: "solid-line-1", lineWidth: 1, head: "Full", type: "Solid" },
  { name: "solid-line-2", lineWidth: 2, head: "Full", type: "Solid" },
  { name: "solid-line-4", lineWidth: 4, head: "Full", type: "Solid" },
  { name: "solid-head-500", lineWidth: 1, head: "Full", type: "Solid", headSize: 500 },
  { name: "solid-head-1500", lineWidth: 1, head: "Full", type: "Solid", headSize: 1500 },
  { name: "solid-half-left", lineWidth: 1, head: "HalfLeft", type: "Solid" },
  { name: "solid-half-right", lineWidth: 1, head: "HalfRight", type: "Solid" },
  { name: "hollow", lineWidth: 1, head: "Full", type: "Hollow" },
  { name: "angle", lineWidth: 1, head: "Full", type: "Angle" },
];

function sourceDocument(entry) {
  const headSize = entry.headSize === undefined ? "" : ` HeadSize="${entry.headSize}"`;
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="0 0 180 80" LineWidth="${entry.lineWidth}" BoldWidth="4" HashSpacing="2.7">
  <page id="1" BoundingBox="0 0 180 80">
    <arrow id="10" Tail3D="10 40 0" Head3D="150 40 0" ArrowheadHead="${entry.head}" ArrowheadType="${entry.type}"${headSize}/>
  </page>
</CDXML>
`;
}

function attributes(tag) {
  const values = {};
  for (const match of tag.matchAll(/([A-Za-z][A-Za-z0-9:-]*)="([^"]*)"/g)) {
    values[match[1]] = match[2];
  }
  return values;
}

function pathRows(svg) {
  return [...svg.matchAll(/<path\b[^>]*\/>/gi)].map((match) => attributes(match[0]));
}

function transformScale(transform) {
  const match = /^matrix\(\s*([-+0-9.eE]+)\s+[-+0-9.eE]+\s+[-+0-9.eE]+\s+([-+0-9.eE]+)/.exec(
    transform ?? "",
  );
  if (!match) {
    return 1;
  }
  const xScale = Math.abs(Number(match[1]));
  const yScale = Math.abs(Number(match[2]));
  if (Math.abs(xScale - yScale) > 1e-6) {
    throw new Error(`Non-uniform ChemDraw SVG transform: ${transform}`);
  }
  return xScale;
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
const inputs = [];
for (const entry of cases) {
  const sourcePath = path.join(sourceDir, `${entry.name}.cdxml`);
  await fs.writeFile(sourcePath, sourceDocument(entry), "utf8");
  inputs.push(sourcePath);
}

await generateChemDrawOracle({
  outDir: oracleDir,
  formats: ["svg"],
  inputs,
  outputNames: cases.map((entry) => entry.name),
});

const results = [];
for (const entry of cases) {
  const svg = await fs.readFile(path.join(oracleDir, `${entry.name}.chemdraw.svg`), "utf8");
  const paths = pathRows(svg);
  const filled = paths.filter((row) => row.fill && row.fill !== "none");
  const outlined = paths.filter((row) => row.stroke && row.stroke !== "none");
  const duplicatePairs = filled.flatMap((fillRow) =>
    outlined
      .filter((strokeRow) => strokeRow.d === fillRow.d)
      .map((strokeRow) => ({
        rawStrokeWidth: Number(strokeRow["stroke-width"]),
        transformScale: transformScale(strokeRow.transform),
        renderedStrokeWidth:
          Number(strokeRow["stroke-width"]) * transformScale(strokeRow.transform),
      })),
  );
  results.push({
    ...entry,
    filledPathCount: filled.length,
    outlinedPathCount: outlined.length,
    duplicatePairs,
  });
}

await fs.writeFile(
  path.join(outDir, "report.json"),
  `${JSON.stringify({ cases: results }, null, 2)}\n`,
  "utf8",
);
console.log(path.join(outDir, "report.json"));
