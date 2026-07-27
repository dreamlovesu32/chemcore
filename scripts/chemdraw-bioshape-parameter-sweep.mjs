import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const option = (name) => {
  const index = process.argv.indexOf(name);
  return index < 0 ? null : process.argv[index + 1];
};
const type = option("--type");
const parameter = option("--parameter");
const values = (option("--values") ?? "")
  .split(",")
  .filter(Boolean)
  .map(Number);
if (!type || !parameter || values.length < 2 || values.some((value) => !Number.isFinite(value))) {
  throw new Error(
    "Usage: --type BioShapeType --parameter FieldName --values 1,2,3",
  );
}
const stem = `${type}-${parameter}`
  .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
  .replace(/[^a-z0-9]+/gi, "-")
  .toLowerCase();
const outDir = path.resolve(
  root,
  option("--out") ?? `tmp/chemdraw-bioshape-parameter-sweep/${stem}`,
);
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");
await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const jobs = [];
for (const value of values) {
  const name = `${stem}--${String(value).replace(".", "_")}`;
  const input = path.join(inputDir, `${name}.cdxml`);
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BoundingBox="0 0 260 220" FractionalWidths="yes"
  HashSpacing="2.5" MarginWidth="1.6" LineWidth="0.6" BoldWidth="2"
  BondLength="14.4" BondSpacing="18" ChainAngle="120">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 260 220">
    <bioshape id="10" BioShapeType="${type}" xyz="130 110 0"
      MajorAxisEnd3D="190 110 0" MinorAxisEnd3D="130 150 0"
      FillType="None" LineType="Solid" LineWidth="0.6" BoldWidth="2"
      ${parameter}="${value}" color="3"/>
  </page>
</CDXML>
`;
  await fs.writeFile(input, xml, "utf8");
  jobs.push({ name, input, value });
}
await generateChemDrawOracle({
  inputs: jobs.map((job) => job.input),
  outputNames: jobs.map((job) => job.name),
  outDir: oracleDir,
  formats: ["cdxml", "svg"],
});
const cases = [];
for (const job of jobs) {
  const cdxml = path.join(oracleDir, `${job.name}.chemdraw.cdxml`);
  const svg = path.join(oracleDir, `${job.name}.chemdraw.svg`);
  const normalized = await fs.readFile(cdxml, "utf8");
  cases.push({
    value: job.value,
    input: path.relative(root, job.input),
    cdxml: path.relative(root, cdxml),
    svg: path.relative(root, svg),
    normalizedTag: normalized.match(/<bioshape\b[\s\S]*?\/>/i)?.[0] ?? null,
  });
}
await fs.writeFile(
  path.join(outDir, "manifest.json"),
  `${JSON.stringify({
    schema: "chemsema.chemdraw-bioshape-parameter-sweep.v1",
    type,
    parameter,
    axes: {
      center: [130, 110, 0],
      major: [190, 110, 0],
      minor: [130, 150, 0],
    },
    cases,
  }, null, 2)}\n`,
);
console.log(`[BIOSHAPE SWEEP] ${type}/${parameter}: ${cases.length} cases`);
