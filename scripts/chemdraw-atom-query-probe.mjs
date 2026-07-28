import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-atom-query-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

const queryCases = [
  ["free-sites-0", 'FreeSites="0"'],
  ["free-sites-1", 'FreeSites="1"'],
  ["free-sites-2", 'FreeSites="2"'],
  ["ring-none", 'RingBondCount="NoRingBonds"'],
  ["ring-as-drawn", 'RingBondCount="AsDrawn"'],
  ["ring-simple", 'RingBondCount="SimpleRing"'],
  ["ring-fusion", 'RingBondCount="Fusion"'],
  ["ring-spiro", 'RingBondCount="SpiroOrHigher"'],
  ["unsaturated-absent", 'UnsaturatedBonds="MustBeAbsent"'],
  ["unsaturated-present", 'UnsaturatedBonds="MustBePresent"'],
  ["substituents-up-to", 'SubstituentsUpTo="2"'],
  ["substituents-exactly", 'SubstituentsExactly="2"'],
  ["translation-equal", 'Translation="Equal"'],
  ["translation-broad", 'Translation="Broad"'],
  ["translation-narrow", 'Translation="Narrow"'],
  ["translation-any", 'Translation="Any"'],
  ["abnormal-valence", 'AbnormalValence="yes"'],
  ["query-combination", 'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent" SubstituentsExactly="3" Translation="Narrow"'],
  ["query-combination-isotope", 'IsotopicAbundance="Enriched" FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent" SubstituentsExactly="3" Translation="Narrow"'],
  ["query-hidden", 'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"', { showAtomQuery: false }],
  ["element-list", 'NodeType="ElementList" ElementList="7 8 15"'],
  ["element-list-not", 'NodeType="ElementList" ElementList="NOT 7 8 15"'],
  ["generic-list", 'NodeType="ElementList" GenericList="R X A"'],
  ["generic-list-not", 'NodeType="ElementList" GenericList="NOT R X A"'],
  ["mixed-list", 'NodeType="ElementList" ElementList="7 8" GenericList="R X"'],
];

for (const [direction, neighbor] of Object.entries({
  right: [130, 100],
  left: [70, 100],
  up: [100, 70],
  down: [100, 130],
})) {
  queryCases.push([
    `orientation-${direction}`,
    'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"',
    { neighbor },
  ]);
}

for (let angle = 0; angle < 360; angle += 30) {
  const radians = angle * Math.PI / 180;
  const neighbor = [
    100 + 30 * Math.cos(radians),
    100 + 30 * Math.sin(radians),
  ];
  queryCases.push([
    `orientation-angle-${angle}`,
    'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"',
    { neighbor },
  ]);
  queryCases.push([
    `implicit-hydrogen-restriction-angle-${angle}`,
    'NumHydrogens="1" ImplicitHydrogens="yes"',
    {
      element: 6,
      neighbor,
      nodeLabel: {
        text: "CH",
        labelAlignment: "Auto",
        lineStarts: null,
      },
    },
  ]);
}
for (const [name, neighbor] of [
  ["open-up-asymmetric-v", [[117.63, 124.27], [71.46, 109.27]]],
  ["open-down-asymmetric-v", [[117.63, 75.73], [71.46, 90.73]]],
]) {
  queryCases.push([
    `orientation-${name}`,
    'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"',
    { neighbor },
  ]);
  queryCases.push([
    `implicit-hydrogen-restriction-${name}`,
    'NumHydrogens="1" ImplicitHydrogens="yes"',
    {
      element: 6,
      neighbor,
      nodeLabel: {
        text: "CH",
        labelAlignment: name.includes("up") ? "Above" : "Below",
        lineStarts: "2 3",
      },
    },
  ]);
}

for (const [fontName, fontId] of [["Arial", 3], ["Times New Roman", 4]]) {
  for (const size of [8, 10, 14]) {
    queryCases.push([
      `font-${fontName.replaceAll(" ", "-").toLowerCase()}-${size}`,
      'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"',
      { fontId, fontName, size },
    ]);
    queryCases.push([
      `implicit-hydrogen-restriction-${fontName.replaceAll(" ", "-").toLowerCase()}-${size}`,
      'NumHydrogens="1" ImplicitHydrogens="yes"',
      {
        element: 6,
        fontId,
        fontName,
        size,
        nodeLabel: {
          text: "CH",
          labelAlignment: "Above",
          lineStarts: "2 3",
        },
      },
    ]);
  }
}

for (const labelAlignment of ["Above", "Below", "Left", "Right"]) {
  queryCases.push([
    `implicit-hydrogen-restriction-alignment-${labelAlignment.toLowerCase()}`,
    'NumHydrogens="1" ImplicitHydrogens="yes"',
    {
      element: 6,
      nodeLabel: {
        text: "CH",
        labelAlignment,
        lineStarts: labelAlignment === "Above" || labelAlignment === "Below" ? "2 3" : null,
      },
    },
  ]);
}
queryCases.push([
  "implicit-hydrogen-restriction-combination",
  'NumHydrogens="1" ImplicitHydrogens="yes" FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent"',
  {
    element: 6,
    nodeLabel: {
      text: "CH",
      labelAlignment: "Above",
      lineStarts: "2 3",
    },
  },
]);

const carbonCases = [
  ["carbon-default", false, false, ""],
  ["carbon-root-terminal", true, false, ""],
  ["carbon-root-nonterminal", false, true, ""],
  ["carbon-root-all", true, true, ""],
  ["carbon-node-terminal", false, false, 'ShowTerminalCarbonLabels="yes"'],
  ["carbon-node-nonterminal", false, false, 'ShowNonTerminalCarbonLabels="yes"'],
  ["carbon-node-all", false, false, 'ShowTerminalCarbonLabels="yes" ShowNonTerminalCarbonLabels="yes"'],
  ["carbon-root-all-node-off", true, true, 'ShowTerminalCarbonLabels="no" ShowNonTerminalCarbonLabels="no"'],
  ["carbon-explicit-element", false, false, 'Element="6" ShowTerminalCarbonLabels="yes" ShowNonTerminalCarbonLabels="yes"'],
];

function xmlEscape(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function documentXml({
  nodeAttributes,
  neighbor = [130, 100],
  showAtomQuery = true,
  fontId = 3,
  fontName = "Arial",
  size = 10,
  element = 7,
  nodeLabel = null,
}) {
  const nodeText = nodeLabel
    ? `<t id="111" p="${100 - 0.282 * size} ${100 - 0.654 * size}"
          LabelAlignment="${nodeLabel.labelAlignment}"
          ${nodeLabel.lineStarts ? `LineStarts="${nodeLabel.lineStarts}"` : ""}>
        <s font="${fontId}" size="${size}" face="96">${xmlEscape(nodeLabel.text)}</s>
      </t>`
    : "";
  const neighbors = Array.isArray(neighbor[0]) ? neighbor : [neighbor];
  const neighborNodes = neighbors
    .map((point, index) => `<n id="${102 + index}" p="${point[0]} ${point[1]}"/>`)
    .join("");
  const neighborBonds = neighbors
    .map((_, index) => `<b id="${202 + index}" B="101" E="${102 + index}"/>`)
    .join("");
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema atom query probe" BoundingBox="0 0 200 200"
 ShowAtomQuery="${showAtomQuery ? "yes" : "no"}"
 ShowTerminalCarbonLabels="no" ShowNonTerminalCarbonLabels="no"
 BondLength="30" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5"
 MarginWidth="1.6" LabelFont="${fontId}" LabelSize="${size}"
 CaptionFont="${fontId}" CaptionSize="${size}">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
    <font id="4" charset="iso-8859-1" name="Times New Roman"/>
  </fonttable>
  <page id="1" BoundingBox="0 0 200 200">
    <fragment id="10">
      <n id="101" p="100 100" Element="${element}" ${nodeAttributes}>${nodeText}</n>
      ${neighborNodes}
      ${neighborBonds}
    </fragment>
    <t id="201" p="20 180"><s font="${fontId}" size="${size}" color="0">${xmlEscape(fontName)} ${size}</s></t>
  </page>
</CDXML>`;
}

function carbonDocumentXml(showTerminal, showNonTerminal, nodeAttributes) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema carbon label probe" BoundingBox="0 0 240 180"
 ShowAtomQuery="yes"
 ShowTerminalCarbonLabels="${showTerminal ? "yes" : "no"}"
 ShowNonTerminalCarbonLabels="${showNonTerminal ? "yes" : "no"}"
 BondLength="30" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5"
 MarginWidth="1.6" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 240 180">
    <fragment id="10">
      <n id="101" p="50 90" ${nodeAttributes}/>
      <n id="102" p="80 90" ${nodeAttributes}/>
      <n id="103" p="110 90" ${nodeAttributes}/>
      <n id="104" p="140 90" ${nodeAttributes}/>
      <n id="105" p="110 60" ${nodeAttributes}/>
      <b id="201" B="101" E="102"/>
      <b id="202" B="102" E="103"/>
      <b id="203" B="103" E="104"/>
      <b id="204" B="103" E="105"/>
    </fragment>
  </page>
</CDXML>`;
}

function attribute(tag, name) {
  return new RegExp(`\\b${name}="([^"]*)"`, "i").exec(tag)?.[1] ?? null;
}

function compact(value) {
  return value.replace(/\s+/g, " ").trim();
}

function nodeBlock(xml, id) {
  const pattern = new RegExp(`<n\\b(?=[^>]*\\bid="${id}")[\\s\\S]*?<\\/n>|<n\\b(?=[^>]*\\bid="${id}")[^>]*/>`, "i");
  return pattern.exec(xml)?.[0] ?? "";
}

function analyzeNode(xml, id) {
  const block = nodeBlock(xml, id);
  const openTag = /^<n\b[^>]*>/i.exec(block)?.[0] ?? "";
  const objectTags = [...block.matchAll(/<objecttag\b[\s\S]*?<\/objecttag>/gi)].map((match) => {
    const text = [...match[0].matchAll(/<s\b[^>]*>([\s\S]*?)<\/s>/gi)]
      .map((part) => part[1].replace(/<[^>]*>/g, ""))
      .join("");
    const textTag = /<t\b[^>]*>/i.exec(match[0])?.[0] ?? "";
    return {
      name: attribute(match[0], "Name"),
      text,
      position: attribute(textTag, "p"),
      boundingBox: attribute(textTag, "BoundingBox"),
      runs: [...match[0].matchAll(/<s\b([^>]*)>([\s\S]*?)<\/s>/gi)].map((run) => ({
        text: run[2],
        font: attribute(run[1], "font"),
        size: attribute(run[1], "size"),
        face: attribute(run[1], "face"),
      })),
    };
  });
  return {
    openTag: compact(openTag),
    nodeType: attribute(openTag, "NodeType"),
    elementList: attribute(openTag, "ElementList"),
    genericList: attribute(openTag, "GenericList"),
    warning: attribute(openTag, "Warning"),
    directText: [...block.matchAll(/<t\b(?![\s\S]*<objecttag)[^>]*>[\s\S]*?<\/t>/gi)]
      .map((match) => [...match[0].matchAll(/<s\b[^>]*>([\s\S]*?)<\/s>/gi)]
        .map((part) => part[1].replace(/<[^>]*>/g, ""))
        .join(""))
      .join(""),
    objectTags,
  };
}

await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const inputs = [];
const names = [];
for (const [name, nodeAttributes, options = {}] of queryCases) {
  const input = path.join(inputDir, `${name}.cdxml`);
  await fs.writeFile(input, documentXml({ nodeAttributes, ...options }), "utf8");
  inputs.push(input);
  names.push(name);
}
for (const [name, showTerminal, showNonTerminal, nodeAttributes] of carbonCases) {
  const input = path.join(inputDir, `${name}.cdxml`);
  await fs.writeFile(input, carbonDocumentXml(showTerminal, showNonTerminal, nodeAttributes), "utf8");
  inputs.push(input);
  names.push(name);
}

await generateChemDrawOracle({
  inputs,
  outputNames: names,
  outDir: oracleDir,
  formats: ["cdxml", "svg"],
});

const results = [];
for (const name of names) {
  const cdxml = await fs.readFile(path.join(oracleDir, `${name}.chemdraw.cdxml`), "utf8");
  const svg = await fs.readFile(path.join(oracleDir, `${name}.chemdraw.svg`), "utf8");
  results.push({
    name,
    target: analyzeNode(cdxml, "101"),
    carbonNodes: name.startsWith("carbon-")
      ? ["101", "102", "103", "104", "105"].map((id) => ({ id, ...analyzeNode(cdxml, id) }))
      : [],
    svgText: [...svg.matchAll(/<text\b[^>]*>[\s\S]*?<\/text>/gi)].map((match) => compact(match[0])),
  });
}

await fs.writeFile(path.join(outDir, "summary.json"), `${JSON.stringify(results, null, 2)}\n`, "utf8");
for (const result of results) {
  const tags = result.target.objectTags.map((tag) => tag.text).join(" | ");
  console.log(`${result.name}: ${tags || "(no generated query tag)"}`);
}
console.log(`[atom-query-probe] summary=${path.relative(root, path.join(outDir, "summary.json"))}`);
