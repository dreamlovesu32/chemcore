import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-carbon-valence-label-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

function documentXml({ bondOrders, numHydrogens, attributes = "" }) {
  const neighbors = [
    [130, 100],
    [85, 74.0192],
    [85, 125.9808],
    [100, 70],
  ];
  const nodes = bondOrders
    .map((_, index) => {
      const [x, y] = neighbors[index];
      return `<n id="${102 + index}" p="${x} ${y}" Element="6"/>`;
    })
    .join("\n      ");
  const bonds = bondOrders
    .map(
      (order, index) =>
        `<b id="${201 + index}" B="101" E="${102 + index}" Order="${order}"/>`,
    )
    .join("\n      ");
  const hydrogenAttribute =
    numHydrogens === null ? "" : ` NumHydrogens="${numHydrogens}"`;
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema carbon valence label probe"
 BoundingBox="0 0 200 200" ShowTerminalCarbonLabels="no"
 ShowNonTerminalCarbonLabels="no" HideImplicitHydrogens="no"
 BondLength="30" LineWidth="0.6" BoldWidth="2" MarginWidth="1.6"
 LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 200 200">
    <fragment id="10">
      <n id="101" p="100 100" Element="6"${hydrogenAttribute} ${attributes}/>
      ${nodes}
      ${bonds}
    </fragment>
  </page>
</CDXML>`;
}

function textContent(svg) {
  return [...svg.matchAll(/<text\b[^>]*>([\s\S]*?)<\/text>/gi)]
    .map((match) => match[1].replace(/<[^>]*>/g, "").replace(/\s+/g, "").trim())
    .filter(Boolean);
}

const cases = [];
for (const [shape, bondOrders] of [
  ["isolated", []],
  ["single-terminal", [1]],
  ["double-terminal", [2]],
  ["two-single", [1, 1]],
  ["single-double", [1, 2]],
  ["two-double", [2, 2]],
  ["three-single", [1, 1, 1]],
  ["four-single", [1, 1, 1, 1]],
]) {
  for (const numHydrogens of [null, 0, 1, 2, 3, 4]) {
    cases.push({
      name: `${shape}-h-${numHydrogens === null ? "absent" : numHydrogens}`,
      bondOrders,
      numHydrogens,
    });
  }
}
for (const [name, attributes] of [
  ["abnormal-valence", 'AbnormalValence="yes"'],
  ["charge-plus", 'Charge="1"'],
  ["charge-minus", 'Charge="-1"'],
  ["radical-doublet", 'Radical="Doublet"'],
  ["radical-singlet", 'Radical="Singlet"'],
  ["radical-triplet", 'Radical="Triplet"'],
  ["isotope-13", 'Isotope="13"'],
]) {
  cases.push({ name: `two-single-h-0-${name}`, bondOrders: [1, 1], numHydrogens: 0, attributes });
  cases.push({ name: `three-single-h-0-${name}`, bondOrders: [1, 1, 1], numHydrogens: 0, attributes });
  cases.push({ name: `two-single-h-2-${name}`, bondOrders: [1, 1], numHydrogens: 2, attributes });
  cases.push({ name: `three-single-h-1-${name}`, bondOrders: [1, 1, 1], numHydrogens: 1, attributes });
  cases.push({ name: `two-single-h-absent-${name}`, bondOrders: [1, 1], numHydrogens: null, attributes });
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
  formats: ["cdxml", "svg"],
});

const results = [];
for (const probe of cases) {
  const svg = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.svg`), "utf8");
  const saved = await fs.readFile(path.join(oracleDir, `${probe.name}.chemdraw.cdxml`), "utf8");
  const texts = textContent(svg);
  const savedTarget = new RegExp(
    '<n\\b(?=[^>]*\\bid="101")[\\s\\S]*?<\\/n>|<n\\b(?=[^>]*\\bid="101")[^>]*/>',
    "i",
  ).exec(saved)?.[0] ?? "";
  results.push({
    ...probe,
    bondOrderSum: probe.bondOrders.reduce((sum, order) => sum + order, 0),
    svgTexts: texts,
    targetTextMaterializedOnSave: /<t\b/i.test(savedTarget),
  });
}

await fs.writeFile(
  path.join(outDir, "summary.json"),
  `${JSON.stringify(results, null, 2)}\n`,
  "utf8",
);
for (const result of results) {
  console.log(
    `${result.name}: valence=${result.bondOrderSum} texts=${result.svgTexts.join("|") || "(none)"} savedText=${result.targetTextMaterializedOnSave}`,
  );
}
console.log(`[carbon-valence-label-probe] summary=${path.relative(root, path.join(outDir, "summary.json"))}`);
