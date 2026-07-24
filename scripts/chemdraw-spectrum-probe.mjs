import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-spectrum-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

const samples = [
  0, 0.05, 0.12, 0.08, 0.02, 0, 0.1, 0.8, 0.2, 0, -0.1, -0.4, -0.05, 0,
  0.05, 0.2, 1, 0.35, 0.08, 0,
];

const cases = [
  {
    name: "nmr-ppm",
    attrs: {
      BoundingBox: "40 40 280 160",
      Class: "NMR",
      XType: "PartsPerMillion",
      XAxisLabel: "ppm",
      XLow: "0",
      XSpacing: "0.5",
      YType: "ArbitraryUnits",
    },
  },
  {
    name: "infrared-transmittance",
    attrs: {
      BoundingBox: "40 40 280 160",
      Class: "Infrared",
      XType: "Wavenumbers",
      XAxisLabel: "cm-1",
      XLow: "400",
      XSpacing: "200",
      YType: "PercentTransmittance",
      YAxisLabel: "%T",
    },
  },
  {
    name: "scaled-y",
    attrs: {
      BoundingBox: "40 40 280 160",
      Class: "NMR",
      XType: "PartsPerMillion",
      XAxisLabel: "ppm",
      XLow: "-2",
      XSpacing: "1",
      YLow: "10",
      YScale: "2",
      YType: "ArbitraryUnits",
    },
  },
  {
    name: "reversed-bbox",
    attrs: {
      BoundingBox: "280 160 40 40",
      Class: "NMR",
      XType: "PartsPerMillion",
      XAxisLabel: "ppm",
      XLow: "0",
      XSpacing: "-0.5",
      YType: "ArbitraryUnits",
    },
  },
  {
    name: "styled",
    attrs: {
      BoundingBox: "40 40 280 160",
      Class: "NMR",
      XType: "PartsPerMillion",
      XAxisLabel: "ppm",
      XLow: "0",
      XSpacing: "0.5",
      YType: "ArbitraryUnits",
      LineWidth: "2",
      color: "3",
      LabelFont: "4",
      LabelSize: "14",
      LabelFace: "1",
    },
  },
];

function attributes(values) {
  return Object.entries(values)
    .map(
      ([name, value]) =>
        `${name}="${String(value).replaceAll("&", "&amp;").replaceAll('"', "&quot;")}"`,
    )
    .join(" ");
}

function documentXml(probe) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema spectrum probe" BoundingBox="0 0 340 220"
 BondLength="14.4" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5"
 MarginWidth="1.6" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="0.8" g="0.1" b="0.1"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
    <font id="4" charset="iso-8859-1" name="Times New Roman"/>
  </fonttable>
  <page id="1" BoundingBox="0 0 340 220">
    <spectrum id="10" Z="1" ${attributes(probe.attrs)}>
      ${samples.join(" ")}
    </spectrum>
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
  const spectrumTag = /<spectrum\b[^>]*>/i.exec(cdxml)?.[0] ?? null;
  const spectrumText =
    /<spectrum\b[^>]*>([\s\S]*?)<\/spectrum>/i.exec(cdxml)?.[1]?.trim() ??
    null;
  const svg = await fs.readFile(`${prefix}.svg`, "utf8");
  report.push({
    name: probe.name,
    spectrumTag,
    spectrumText,
    svgElements: [
      ...svg.matchAll(/<(?:path|polyline|line|text)\b[^>]*>/gi),
    ].map((match) => match[0].replace(/\s+/g, " ").trim()),
  });
}

await fs.writeFile(
  path.join(outDir, "report.json"),
  `${JSON.stringify(report, null, 2)}\n`,
  "utf8",
);
console.log(`Wrote ${report.length} ChemDraw spectrum probes to ${outDir}`);
