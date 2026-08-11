import fs from "node:fs/promises";
import path from "node:path";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.resolve(
  root,
  process.argv[2] ?? "tmp/chemdraw-line-starts-probe",
);
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "oracle");

const cases = [
  {
    name: "caption-plain-unbroken",
    body: '<t id="20" p="72 72" LineStarts="2 3"><s font="3" size="10">ABC</s></t>',
  },
  {
    name: "caption-plain-authored-newline",
    body: '<t id="20" p="72 72" LineStarts="2 4"><s font="3" size="10">AB\nC</s></t>',
  },
  {
    name: "caption-chemical-unbroken",
    body: '<t id="20" p="72 72" InterpretChemically="yes" LineStarts="2 4 6"><s font="3" size="10" face="96">NH+</s></t>',
  },
  {
    name: "atom-chemical-above",
    body: atomLabel({ alignment: "Above", lineStarts: "2 4 6" }),
  },
  {
    name: "atom-chemical-above-no-line-starts",
    body: atomLabel({ alignment: "Above" }),
  },
  {
    name: "atom-chemical-below",
    body: atomLabel({ alignment: "Below", lineStarts: "2 4 6" }),
  },
  {
    name: "atom-chemical-left",
    body: atomLabel({ alignment: "Left", lineStarts: "2 4 6" }),
  },
  {
    name: "atom-chemical-no-alignment",
    body: atomLabel({ lineStarts: "2 4 6" }),
  },
  {
    name: "nickname-chemical-above",
    body: nicknameLabel({ alignment: "Above", lineStarts: "2 4 6", chemical: true }),
  },
  {
    name: "nickname-plain-above",
    body: nicknameLabel({ alignment: "Above", lineStarts: "2 3", chemical: false }),
  },
  {
    name: "nickname-plain-no-alignment",
    body: nicknameLabel({ lineStarts: "2 3", chemical: false }),
  },
];

function atomLabel({ alignment, lineStarts }) {
  const alignmentAttr = alignment ? ` LabelAlignment="${alignment}"` : "";
  const startsAttr = lineStarts ? ` LineStarts="${lineStarts}"` : "";
  return `<fragment id="10"><n id="11" p="72 72" Element="7" NumHydrogens="1"><t id="20" p="72 72" InterpretChemically="yes"${alignmentAttr}${startsAttr}><s font="3" size="10" face="96">NH+</s></t></n><n id="12" p="54 82"/><n id="13" p="90 82"/><b id="14" B="12" E="11"/><b id="15" B="11" E="13"/></fragment>`;
}

function nicknameLabel({ alignment, lineStarts, chemical }) {
  const alignmentAttr = alignment ? ` LabelAlignment="${alignment}"` : "";
  const startsAttr = lineStarts ? ` LineStarts="${lineStarts}"` : "";
  const chemicalAttr = chemical ? ' InterpretChemically="yes"' : ' InterpretChemically="no"';
  const face = chemical ? ' face="96"' : "";
  return `<fragment id="10"><n id="11" p="72 72" NodeType="GenericNickname"><t id="20" p="72 72"${chemicalAttr}${alignmentAttr}${startsAttr}><s font="3" size="10"${face}>NH+</s></t></n><n id="12" p="54 82"/><b id="14" B="12" E="11"/></fragment>`;
}

function documentSource(body) {
  return `<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema LineStarts probe" BoundingBox="0 0 144 144" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10" BondLength="14.4" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" MarginWidth="1.6" color="0" bgcolor="1">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
<page id="1" BoundingBox="0 0 144 144">${body}</page>
</CDXML>`;
}

function svgTextRecords(svg) {
  return [...svg.matchAll(/<text\b([^>]*)>([\s\S]*?)<\/text>/g)].map((match) => ({
    text: match[2].replace(/<[^>]+>/g, "").trim(),
    transform: /transform="([^"]+)"/.exec(match[1])?.[1] ?? null,
  }));
}

function savedTextRecord(cdxml) {
  const text = /<t\b([^>]*)>([\s\S]*?)<\/t>/.exec(cdxml);
  if (!text) return null;
  return {
    lineStarts: /\bLineStarts="([^"]+)"/.exec(text[1])?.[1] ?? null,
    labelAlignment: /\bLabelAlignment="([^"]+)"/.exec(text[1])?.[1] ?? null,
    text: [...text[2].matchAll(/<s\b[^>]*>([\s\S]*?)<\/s>/g)]
      .map((match) => match[1])
      .join(""),
  };
}

await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
const inputs = [];
for (const probe of cases) {
  const input = path.join(inputDir, `${probe.name}.cdxml`);
  await fs.writeFile(input, documentSource(probe.body), "utf8");
  inputs.push(input);
}

await generateChemDrawOracle({
  inputs,
  outDir: oracleDir,
  formats: ["cdxml", "svg"],
  outputNames: cases.map((probe) => probe.name),
});

const measurements = [];
for (const probe of cases) {
  const cdxml = await fs.readFile(
    path.join(oracleDir, `${probe.name}.chemdraw.cdxml`),
    "utf8",
  );
  const svg = await fs.readFile(
    path.join(oracleDir, `${probe.name}.chemdraw.svg`),
    "utf8",
  );
  measurements.push({
    name: probe.name,
    saved: savedTextRecord(cdxml),
    svgTexts: svgTextRecords(svg),
  });
}

const report = {
  schema: "chemsema.chemdrawLineStartsProbe.v1",
  generatedAt: new Date().toISOString(),
  officialReferences: [
    "https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LineStarts.htm",
    "https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/DataType/INT16ListWithCounts.htm",
  ],
  measurements,
};
const reportPath = path.join(outDir, "line-starts-probe.json");
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
console.log(reportPath);
