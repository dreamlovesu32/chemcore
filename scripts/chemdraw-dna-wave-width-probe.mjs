import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.join(root, "tmp/chemdraw-dna-wave-width-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");
await fs.mkdir(inputDir, { recursive: true });

const cases = [
  { bondLength: 10, width: 1 },
  { bondLength: 14.4, width: 0.5 },
  { bondLength: 14.4, width: 1 },
  { bondLength: 20, width: 1 },
];
const inputs = [];
const names = [];
for (const testCase of cases) {
  const name = `bond-${testCase.bondLength}-width-${testCase.width}`.replaceAll(".", "_");
  const input = path.join(inputDir, `${name}.cdxml`);
  await fs.writeFile(input, `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BoundingBox="0 0 260 220" MarginWidth="1.6" LineWidth="0.6"
  BoldWidth="2" BondLength="${testCase.bondLength}">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 260 220">
    <bioshape id="10" BioShapeType="DNA" xyz="130 110 0"
      MajorAxisEnd3D="190 110 0" MinorAxisEnd3D="130 150 0"
      FillType="None" LineType="Solid" color="3"
      DNAWaveHeight="12" DNAWaveLength="18" DNAWaveOffset="3"
      DNAWaveWidth="${testCase.width}"/>
  </page>
</CDXML>
`, "utf8");
  inputs.push(input);
  names.push(name);
}
await generateChemDrawOracle({
  inputs,
  outputNames: names,
  outDir: oracleDir,
  formats: ["cdxml"],
});
const results = [];
for (let index = 0; index < cases.length; index += 1) {
  const cdxml = await fs.readFile(
    path.join(oracleDir, `${names[index]}.chemdraw.cdxml`),
    "utf8",
  );
  results.push({
    ...cases[index],
    normalizedWidth: Number(
      cdxml.match(/\bDNAWaveWidth="([^"]+)"/)?.[1],
    ),
  });
}
await fs.writeFile(
  path.join(outDir, "report.json"),
  `${JSON.stringify(results, null, 2)}\n`,
);
console.log(JSON.stringify(results, null, 2));
