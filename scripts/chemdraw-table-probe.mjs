import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-table-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

const cases = [
  {
    name: "one-cell-default",
    table: `
      <table id="10" BoundingBox="40 40 240 140" Z="1">
        <page id="11" BoundingBox="40 40 240 140" BoundsInParent="40 40 240 140"/>
      </table>`,
  },
  {
    name: "two-by-two-default",
    table: `
      <table id="20" BoundingBox="40 40 240 140" Z="1">
        <page id="21" BoundingBox="40 40 140 90" BoundsInParent="40 40 140 90"/>
        <page id="22" BoundingBox="140 40 240 90" BoundsInParent="140 40 240 90"/>
        <page id="23" BoundingBox="40 90 140 140" BoundsInParent="40 90 140 140"/>
        <page id="24" BoundingBox="140 90 240 140" BoundsInParent="140 90 240 140"/>
      </table>`,
  },
  {
    name: "two-by-two-explicit-borders",
    table: `
      <table id="30" BoundingBox="40 40 240 140" Z="1" LineWidth="0.75" color="2">
        <page id="31" BoundingBox="40 40 140 90" BoundsInParent="40 40 140 90">
          <border id="311" Side="top" LineType="Solid" LineWidth="1.5" color="3"/>
          <border id="312" Side="left" LineType="Dashed" LineWidth="0.75" color="4"/>
          <border id="313" Side="bottom" LineType="Bold" LineWidth="0.75" color="5"/>
          <border id="314" Side="right" LineType="Wavy" LineWidth="0.75" color="6"/>
        </page>
        <page id="32" BoundingBox="140 40 240 90" BoundsInParent="140 40 240 90">
          <border id="321" Side="top" LineType="Solid" LineWidth="0.75" color="2"/>
          <border id="322" Side="left" LineType="Wavy" LineWidth="0.75" color="6"/>
          <border id="323" Side="bottom" LineType="Dashed" LineWidth="0.75" color="4"/>
          <border id="324" Side="right" LineType="Bold" LineWidth="0.75" color="5"/>
        </page>
        <page id="33" BoundingBox="40 90 140 140" BoundsInParent="40 90 140 140"/>
        <page id="34" BoundingBox="140 90 240 140" BoundsInParent="140 90 240 140"/>
      </table>`,
  },
  {
    name: "cell-content",
    table: `
      <table id="40" BoundingBox="40 40 240 140" Z="1">
        <page id="41" BoundingBox="40 40 140 140" BoundsInParent="40 40 140 140">
          <t id="411" p="65 85" BoundingBox="60 75 100 95"><s font="3" size="10" face="0">Cell A</s></t>
        </page>
        <page id="42" BoundingBox="140 40 240 140" BoundsInParent="140 40 240 140">
          <fragment id="420">
            <n id="421" p="165 90" Element="6"/>
            <n id="422" p="195 90" Element="8"/>
            <b id="423" B="421" E="422" Order="1"/>
          </fragment>
        </page>
      </table>`,
  },
  {
    name: "hidden-border-candidates",
    table: `
      <table id="50" BoundingBox="40 40 240 140" Z="1">
        <page id="51" BoundingBox="40 40 140 140" BoundsInParent="40 40 140 140">
          <border id="511" Side="top" LineWidth="0"/>
          <border id="512" Side="left" LineType="Solid" LineWidth="0"/>
          <border id="513" Side="bottom" LineType="Dashed" LineWidth="0"/>
          <border id="514" Side="right" color="1"/>
        </page>
        <page id="52" BoundingBox="140 40 240 140" BoundsInParent="140 40 240 140"/>
      </table>`,
  },
];

function documentXml(probe) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema table probe" BoundingBox="0 0 300 200"
 BondLength="14.4" LineWidth="0.75" BoldWidth="2" HashSpacing="2.5"
 MarginWidth="1.6" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
    <color r="0" g="0.6" b="0"/>
    <color r="0" g="0" b="1"/>
    <color r="0.8" g="0" b="0.8"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="1" BoundingBox="0 0 300 200">
    ${probe.table}
  </page>
</CDXML>`;
}

await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const inputs = [];
for (const probe of cases) {
  const input = path.join(inputDir, `${probe.name}.cdxml`);
  await fs.writeFile(input, documentXml(probe), "utf8");
  inputs.push(input);
}

await generateChemDrawOracle({
  inputs,
  outputNames: cases.map((probe) => probe.name),
  outDir: oracleDir,
  formats: ["cdxml", "cdx", "svg", "emf"],
});

const report = [];
for (const probe of cases) {
  const prefix = path.join(oracleDir, `${probe.name}.chemdraw`);
  const cdxml = await fs.readFile(`${prefix}.cdxml`, "utf8");
  const svg = await fs.readFile(`${prefix}.svg`, "utf8");
  report.push({
    name: probe.name,
    tableTags: [...cdxml.matchAll(/<(?:table|page|border)\b[^>]*>/gi)].map(
      (match) => match[0].replace(/\s+/g, " ").trim(),
    ),
    svgElements: [...svg.matchAll(/<(?:path|polyline|line|rect|text)\b[^>]*>/gi)].map(
      (match) => match[0].replace(/\s+/g, " ").trim(),
    ),
  });
}

await fs.writeFile(
  path.join(outDir, "report.json"),
  `${JSON.stringify(report, null, 2)}\n`,
  "utf8",
);
console.log(`Wrote ${report.length} ChemDraw table probes to ${outDir}`);
