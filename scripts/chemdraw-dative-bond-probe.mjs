import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.resolve(root, process.argv[2] ?? "tmp/chemdraw-dative-bond-probe");
const refresh = process.argv.includes("--refresh");
const settings = [
  { name: "line-0.6-bold-2", lineWidth: 0.6, boldWidth: 2 },
  { name: "line-1-bold-2", lineWidth: 1, boldWidth: 2 },
  { name: "line-1-bold-4", lineWidth: 1, boldWidth: 4 },
  { name: "line-2-bold-4", lineWidth: 2, boldWidth: 4 },
];
const geometries = [
  { name: "length-5-right", dx: 5, dy: 0 },
  { name: "length-10-right", dx: 10, dy: 0 },
  { name: "length-30-right", dx: 30, dy: 0 },
  { name: "length-60-right", dx: 60, dy: 0 },
  { name: "length-100-right", dx: 100, dy: 0 },
  { name: "length-60-down", dx: 0, dy: 60 },
  { name: "length-60-diagonal", dx: 42.4264, dy: 42.4264 },
];

function sourceDocument(setting) {
  const fragments = geometries.map((geometry, index) => {
    const fragmentId = 10 + index * 10;
    const beginId = fragmentId + 1;
    const endId = fragmentId + 2;
    const bondId = fragmentId + 3;
    const x = 20;
    const y = 30 + index * 100;
    return `    <fragment id="${fragmentId}">
      <n id="${beginId}" p="${x} ${y}" Element="6"/>
      <n id="${endId}" p="${x + geometry.dx} ${y + geometry.dy}" Element="6"/>
      <b id="${bondId}" B="${beginId}" E="${endId}" Order="dative"/>
    </fragment>`;
  });
  const height = geometries.length * 100 + 60;
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="0 0 180 ${height}"
  LineWidth="${setting.lineWidth}" BoldWidth="${setting.boldWidth}" BondLength="30"
  MarginWidth="1.6" HashSpacing="2.7">
  <page id="1" BoundingBox="0 0 180 ${height}">
${fragments.join("\n")}
  </page>
</CDXML>
`;
}

await fs.mkdir(outDir, { recursive: true });
const inputs = [];
const outputNames = [];
for (const setting of settings) {
  const input = path.join(outDir, `${setting.name}.source.cdxml`);
  await fs.writeFile(input, sourceDocument(setting), "utf8");
  inputs.push(input);
  outputNames.push(setting.name);
}

const expected = settings.flatMap((setting) => [
  path.join(outDir, `${setting.name}.chemdraw.cdxml`),
  path.join(outDir, `${setting.name}.chemdraw.svg`),
]);
const haveOracle = !refresh && (await Promise.all(
  expected.map((file) => fs.access(file).then(() => true, () => false)),
)).every(Boolean);
if (!haveOracle) {
  await generateChemDrawOracle({
    outDir,
    formats: ["cdxml", "svg"],
    inputs,
    outputNames,
  });
}

await fs.writeFile(path.join(outDir, "manifest.json"), `${JSON.stringify({
  settings,
  geometries,
  outputs: outputNames,
}, null, 2)}\n`);
console.log(path.join(outDir, "manifest.json"));
