import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { inspectEmf } from "./emf-inspect.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-external-connection-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

const directions = {
  right: [[60, 80], [120, 80]],
  left: [[140, 80], [80, 80]],
  down: [[90, 45], [90, 105]],
  up: [[90, 125], [90, 65]],
  downRight: [[55, 50], [107, 80]],
  upRight: [[55, 110], [107, 80]],
};

const cases = [];
const add = (name, options = {}) => cases.push({ name, ...options });

add("absent");
for (const type of ["Unspecified", "Diamond", "Star", "PolymerBead", "Wavy"]) {
  add(`type-${type.toLowerCase()}`, { type });
}
for (const [direction, points] of Object.entries(directions)) {
  for (const type of ["Diamond", "Star", "PolymerBead", "Wavy"]) {
    add(`${type.toLowerCase()}-${direction}`, { type, points });
  }
}
for (const bondLength of [20, 40, 60, 90]) {
  for (const type of ["Diamond", "Star", "PolymerBead", "Wavy"]) {
    add(`${type.toLowerCase()}-bond-length-${bondLength}`, { type, bondLength });
  }
}
for (const lineWidth of [0.3, 0.6, 1, 2]) {
  for (const type of ["Diamond", "Star", "PolymerBead", "Wavy"]) {
    add(`${type.toLowerCase()}-line-width-${String(lineWidth).replace(".", "_")}`, {
      type,
      lineWidth,
    });
  }
}
for (const labelSize of [8, 10, 14, 18]) {
  for (const type of ["Diamond", "Star", "PolymerBead", "Wavy"]) {
    add(`${type.toLowerCase()}-label-size-${labelSize}`, { type, labelSize });
  }
}
for (const number of [1, 2, 12]) {
  add(`diamond-number-${number}`, { type: "Diamond", number });
}
for (const type of [
  "Residue",
  "Peptide",
  "DNA",
  "RNA",
  "Terminus",
  "Sulfide",
  "Nucleotide",
  "UnlinkedBranch",
]) {
  add(`semantic-${type.toLowerCase()}`, { type });
}
for (const display of ["Dash", "Bold", "WedgeEnd", "WedgedHashEnd"]) {
  for (const type of ["Diamond", "Star", "PolymerBead", "Wavy"]) {
    add(`${type.toLowerCase()}-bond-${display.toLowerCase()}`, { type, display });
  }
}
for (const type of ["Diamond", "Star", "PolymerBead", "Wavy"]) {
  if (type !== "Wavy") {
    add(`${type.toLowerCase()}-unconnected`, { type, unconnected: true });
  }
  add(`${type.toLowerCase()}-two-bonds`, { type, twoBonds: true });
}

const requestedNames = new Set(process.argv.slice(2));
const selectedCases = requestedNames.size
  ? cases.filter((probe) => requestedNames.has(probe.name))
  : cases;

function externalAttributes(probe) {
  return [
    'NodeType="ExternalConnectionPoint"',
    probe.type ? `ExternalConnectionType="${probe.type}"` : "",
    probe.number != null ? `ExternalConnectionNum="${probe.number}"` : "",
  ].filter(Boolean).join(" ");
}

function documentXml(probe) {
  const [[x1, y1], [x2, y2]] = probe.points ?? directions.right;
  const lineWidth = probe.lineWidth ?? 0.6;
  const bondLength = probe.bondLength ?? 60;
  const labelSize = probe.labelSize ?? 10;
  const display = probe.display ? `Display="${probe.display}"` : "";
  const secondNode = probe.twoBonds
    ? `<n id="104" p="${x2} ${y2 + 45}" Element="6"/>
      <b id="105" B="104" E="102"/>`
    : "";
  const bond = probe.unconnected
    ? ""
    : `<b id="103" B="101" E="102" ${display}/>`;
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema external connection probe"
 BoundingBox="0 0 200 170" BondLength="${bondLength}" LineWidth="${lineWidth}"
 BoldWidth="2" HashSpacing="2.5" MarginWidth="1.6"
 LabelFont="3" LabelSize="${labelSize}" CaptionFont="3" CaptionSize="10">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 200 170">
    <fragment id="10">
      <n id="101" p="${x1} ${y1}" Element="6"/>
      <n id="102" p="${x2} ${y2}" ${externalAttributes(probe)}/>
      ${bond}
      ${secondNode}
    </fragment>
    <t id="201" p="15 155"><s font="3" size="8" color="0">${probe.name}</s></t>
  </page>
</CDXML>`;
}

function attribute(tag, name) {
  return new RegExp(`\\b${name}="([^"]*)"`, "i").exec(tag)?.[1] ?? null;
}

function compact(value) {
  return value.replace(/\s+/g, " ").trim();
}

function nodeOpenTag(xml, id) {
  return new RegExp(`<n\\b(?=[^>]*\\bid="${id}")[^>]*>`, "i").exec(xml)?.[0]
    ?? new RegExp(`<n\\b(?=[^>]*\\bid="${id}")[^>]*/>`, "i").exec(xml)?.[0]
    ?? "";
}

function svgElements(svg) {
  return [...svg.matchAll(/<(path|polygon|polyline|line|circle|ellipse|rect)\b[^>]*>/gi)]
    .map((match) => compact(match[0]));
}

function cdxPropertyPayloads(buffer, tag) {
  const payloads = [];
  for (let offset = 0; offset + 5 <= buffer.length; offset += 1) {
    if (buffer.readUInt16LE(offset) !== tag) continue;
    let length = buffer.readUInt16LE(offset + 2);
    let dataOffset = offset + 4;
    if (length === 0xffff && dataOffset + 4 <= buffer.length) {
      length = buffer.readUInt32LE(dataOffset);
      dataOffset += 4;
    }
    if (length <= 16 && dataOffset + length <= buffer.length) {
      payloads.push([...buffer.subarray(dataOffset, dataOffset + length)]);
    }
  }
  return payloads;
}

await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const inputs = [];
for (const probe of selectedCases) {
  const input = path.join(inputDir, `${probe.name}.cdxml`);
  await fs.writeFile(input, documentXml(probe), "utf8");
  inputs.push(input);
}

await generateChemDrawOracle({
  inputs,
  outputNames: selectedCases.map((probe) => probe.name),
  outDir: oracleDir,
  formats: ["cdxml", "cdx", "svg", "emf"],
});

const results = [];
for (const probe of selectedCases) {
  const cdxml = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.cdxml`), "utf8");
  const svg = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.svg`), "utf8");
  const cdx = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.cdx`));
  const emf = await inspectEmf(path.join(oracleDir, `${probe.name}.chemdraw.emf`), {
    includeRecords: true,
  });
  const nodeTag = nodeOpenTag(cdxml, "102");
  results.push({
    name: probe.name,
    source: probe,
    node: {
      openTag: compact(nodeTag),
      nodeType: attribute(nodeTag, "NodeType"),
      externalConnectionType: attribute(nodeTag, "ExternalConnectionType"),
      externalConnectionNum: attribute(nodeTag, "ExternalConnectionNum"),
      position: attribute(nodeTag, "p"),
    },
    svg: {
      viewBox: attribute(svg, "viewBox"),
      elements: svgElements(svg),
    },
    emf: emf.records
      .filter((record) => [
        "EMR_POLYGON",
        "EMR_POLYLINE",
        "EMR_POLYBEZIER",
        "EMR_ELLIPSE",
        "EMR_EXTTEXTOUTW",
      ].includes(record.name))
      .map((record) => ({
        name: record.name,
        bounds: record.bounds,
        points: record.points,
        text: record.text,
      })),
    cdxExternalConnectionType: cdxPropertyPayloads(cdx, 0x0440),
  });
}

await fs.writeFile(path.join(outDir, "summary.json"), `${JSON.stringify(results, null, 2)}\n`);
for (const result of results) {
  console.log(
    `${result.name}: type=${result.node.externalConnectionType ?? "(absent)"} `
      + `svg=${result.svg.elements.length} cdx=${JSON.stringify(result.cdxExternalConnectionType)}`,
  );
}
console.log(`[external-connection-probe] summary=${path.relative(root, path.join(outDir, "summary.json"))}`);
