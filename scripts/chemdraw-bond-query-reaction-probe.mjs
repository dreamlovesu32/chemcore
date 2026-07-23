import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { inspectEmf } from "./emf-inspect.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-bond-query-reaction-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

const directions = {
  right: [[70, 70], [130, 70]],
  left: [[130, 70], [70, 70]],
  down: [[70, 50], [70, 110]],
  up: [[70, 110], [70, 50]],
  downRight: [[70, 50], [122, 80]],
  upRight: [[70, 110], [122, 80]],
  short: [[70, 70], [82, 70]],
};

const cases = [];
const add = (name, attributes, options = {}) => cases.push({ name, attributes, ...options });

for (const order of ["1 2", "1 1.5", "1 3", "1.5 2", "2 3", "1 1.5 2", "1 2 3", "1 1.5 2 3"]) {
  add(`order-${order.replaceAll(".", "_").replaceAll(" ", "-")}`, `Order="${order}"`);
}
for (const topology of ["Unspecified", "Ring", "Chain", "RingOrChain"]) {
  add(`topology-${topology.toLowerCase()}`, `Topology="${topology}"`);
}
for (const participation of [
  "Unspecified",
  "ReactionCenter",
  "MakeOrBreak",
  "ChangeType",
  "MakeAndChange",
  "NotReactionCenter",
  "NoChange",
  "Unmapped",
]) {
  add(`reaction-${participation.toLowerCase()}`, `RxnParticipation="${participation}"`);
}
for (const stereo of ["U", "N", "E", "Z"]) {
  add(`stereo-${stereo.toLowerCase()}`, `BS="${stereo}"`);
}
for (const stereo of ["Unspecified", "Inversion", "Retention"]) {
  add(`atom-rxn-stereo-${stereo.toLowerCase()}`, "", { atomAttributes: `RxnChange="yes" RxnStereo="${stereo}"` });
}
add("atom-rxn-inversion-only", "", { atomAttributes: 'RxnStereo="Inversion"' });
add("atom-rxn-retention-only", "", { atomAttributes: 'RxnStereo="Retention"' });
add("atom-rxn-combined-query", "", {
  atomAttributes: 'FreeSites="2" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent" RxnChange="yes" RxnStereo="Inversion"',
});
add(
  "combined-all",
  'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange" BS="E"',
  { atomAttributes: 'RxnChange="yes" RxnStereo="Inversion"' },
);
for (const [direction, points] of Object.entries(directions)) {
  add(
    `direction-${direction}`,
    'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange" BS="E"',
    { points },
  );
}
add("per-bond-hide-all", 'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange" BS="E" ShowBondQuery="no" ShowBondRxn="no" ShowBondStereo="no"');
add("root-hide-all", 'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange" BS="E"', {
  rootShow: { query: false, reaction: false, stereo: false },
});
add("root-hide-bond-show", 'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange" BS="E" ShowBondQuery="yes" ShowBondRxn="yes" ShowBondStereo="yes"', {
  rootShow: { query: false, reaction: false, stereo: false },
});
add("single-query-vertical", 'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange"', {
  points: directions.down,
});
add("single-stereo-vertical", 'BS="E"', { points: directions.down });
add("single-query-down-right", 'Order="1 2" Topology="Ring" RxnParticipation="MakeAndChange"', {
  points: directions.downRight,
});
add("single-stereo-down-right", 'BS="E"', { points: directions.downRight });
add("atom-rxn-full-order", "", {
  atomAttributes: 'SubstituentsExactly="3" RingBondCount="SimpleRing" UnsaturatedBonds="MustBePresent" Translation="Narrow" IsotopicAbundance="Enriched" RxnChange="yes" RxnStereo="Inversion"',
});

const requestedNames = new Set(process.argv.slice(2));
const selectedCases = requestedNames.size
  ? cases.filter((probe) => requestedNames.has(probe.name))
  : cases;

function xmlEscape(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function documentXml(probe) {
  const [[x1, y1], [x2, y2]] = probe.points ?? directions.right;
  const show = probe.rootShow ?? { query: true, reaction: true, stereo: true };
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema bond query reaction probe" BoundingBox="0 0 200 160"
 ShowBondQuery="${show.query ? "yes" : "no"}"
 ShowBondRxn="${show.reaction ? "yes" : "no"}"
 ShowBondStereo="${show.stereo ? "yes" : "no"}"
 ShowAtomQuery="yes" ShowAtomStereo="yes"
 BondLength="60" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5"
 MarginWidth="1.6" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 200 160">
    <fragment id="10">
      <n id="101" p="${x1} ${y1}" Element="6" ${probe.atomAttributes ?? ""}/>
      <n id="102" p="${x2} ${y2}" Element="6"/>
      <b id="103" B="101" E="102" ${probe.attributes}/>
    </fragment>
    <t id="201" p="20 145"><s font="3" size="8" color="0">${xmlEscape(probe.name)}</s></t>
  </page>
</CDXML>`;
}

function attribute(tag, name) {
  return new RegExp(`\\b${name}="([^"]*)"`, "i").exec(tag)?.[1] ?? null;
}

function compact(value) {
  return value.replace(/\s+/g, " ").trim();
}

function objectBlock(xml, element, id) {
  const pattern = new RegExp(
    `<${element}\\b(?=[^>]*\\bid="${id}")[\\s\\S]*?<\\/${element}>|<${element}\\b(?=[^>]*\\bid="${id}")[^>]*/>`,
    "i",
  );
  return pattern.exec(xml)?.[0] ?? "";
}

function analyzeObject(xml, element, id) {
  const block = objectBlock(xml, element, id);
  const openTag = new RegExp(`^<${element}\\b[^>]*>`, "i").exec(block)?.[0] ?? "";
  const objectTags = [...block.matchAll(/<objecttag\b[\s\S]*?<\/objecttag>/gi)].map((match) => {
    const text = [...match[0].matchAll(/<s\b[^>]*>([\s\S]*?)<\/s>/gi)]
      .map((part) => part[1].replace(/<[^>]*>/g, ""))
      .join("");
    const textTag = /<t\b[^>]*>/i.exec(match[0])?.[0] ?? "";
    return {
      name: attribute(match[0], "Name"),
      visible: attribute(match[0], "Visible"),
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
    objectTags,
  };
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

const propertyTags = {
  atomRxnChange: 0x0427,
  atomRxnStereo: 0x0428,
  topology: 0x0606,
  reaction: 0x0607,
  absoluteStereo: 0x060a,
  showQuery: 0x060c,
  showStereo: 0x060d,
  showReaction: 0x060f,
};
const results = [];
for (const probe of selectedCases) {
  const cdxml = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.cdxml`), "utf8");
  const svg = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.svg`), "utf8");
  const cdx = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.cdx`));
  const emf = await inspectEmf(path.join(oracleDir, `${probe.name}.chemdraw.emf`), { includeRecords: true });
  results.push({
    name: probe.name,
    source: {
      attributes: probe.attributes,
      atomAttributes: probe.atomAttributes ?? "",
      rootShow: probe.rootShow ?? { query: true, reaction: true, stereo: true },
      points: probe.points ?? directions.right,
    },
    bond: analyzeObject(cdxml, "b", "103"),
    atom: analyzeObject(cdxml, "n", "101"),
    svgText: [...svg.matchAll(/<text\b[^>]*>[\s\S]*?<\/text>/gi)].map((match) => compact(match[0])),
    emfText: emf.records
      .filter((record) => record.name === "EMR_EXTTEXTOUTW" || record.name === "EMR_EXTTEXTOUTA")
      .map((record) => ({ text: record.text, reference: record.reference, bounds: record.bounds })),
    cdxProperties: Object.fromEntries(
      Object.entries(propertyTags).map(([name, tag]) => [name, cdxPropertyPayloads(cdx, tag)]),
    ),
  });
}

await fs.writeFile(path.join(outDir, "summary.json"), `${JSON.stringify(results, null, 2)}\n`, "utf8");
for (const result of results) {
  const bondTags = result.bond.objectTags.map((tag) => `${tag.name}:${tag.text}`).join(" | ");
  const atomTags = result.atom.objectTags.map((tag) => `${tag.name}:${tag.text}`).join(" | ");
  console.log(`${result.name}: bond=${bondTags || "(none)"} atom=${atomTags || "(none)"}`);
}
console.log(`[bond-query-reaction-probe] summary=${path.relative(root, path.join(outDir, "summary.json"))}`);
