import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const refresh = process.argv.includes("--refresh");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(
  root,
  outputArg ?? "tmp/chemdraw-label-vertical-anchor-probe",
);
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");

const fonts = [
  { id: 3, name: "Arial", slug: "arial" },
  { id: 4, name: "Helvetica", slug: "helvetica" },
  { id: 5, name: "Times New Roman", slug: "times" },
];
const probeSizes = [8, 9, 9.1, 9.9, 9.95, 10, 14, 14.45];
const labels = [
  { text: "N", baseText: "N", element: 7, hydrogens: 0 },
  { text: "O", baseText: "O", element: 8, hydrogens: 0 },
  { text: "N+", baseText: "N", splitRuns: 2, element: 7, hydrogens: 0, charge: 1 },
  { text: "O-", baseText: "O", splitRuns: 2, element: 8, hydrogens: 0, charge: -1 },
  { text: "NH2", baseText: "NH", splitRuns: 2, element: 7, hydrogens: 2 },
  { text: "OH", element: 8, hydrogens: 1 },
  { text: "CH3", baseText: "CH", splitRuns: 2, element: 6, hydrogens: 3, showCarbon: true },
];

function fixed(value) {
  return Number(value).toFixed(4).replace(/\.?0+$/, "");
}

function sourceFor(font, size, authoredOffset, creator) {
  const rows = labels.map((label, index) => {
    const nodeId = 100 + index * 3;
    const y = 30 + index * 28;
    const baseline = y + authoredOffset;
    const charge = label.charge ? ` Charge="${label.charge}"` : "";
    const showCarbon = label.showCarbon ? ` ShowTerminalCarbonLabels="yes"` : "";
    return `
      <n id="${nodeId}" p="20 ${y}" AS="N"/>
      <n id="${nodeId + 1}" p="60 ${y}" AS="N" Element="${label.element}"
         NumHydrogens="${label.hydrogens}" NeedsClean="yes"${charge}${showCarbon}>
        <t p="56 ${fixed(baseline)}" BoundingBox="54 ${fixed(y - size)} 76 ${fixed(baseline)}"
           LabelJustification="Left" LabelAlignment="Left">
          <s font="${font.id}" size="${size}" face="96" color="0">${label.text}</s>
        </t>
      </n>
      <b id="${nodeId + 2}" B="${nodeId}" E="${nodeId + 1}"/>`;
  }).join("");
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="${creator}" BoundingBox="0 0 100 230"
 FractionalWidths="yes" InterpretChemically="yes"
 ShowTerminalCarbonLabels="no" ShowNonTerminalCarbonLabels="no"
 LabelFont="${font.id}" LabelSize="${size}" LabelFace="96"
 CaptionFont="${font.id}" CaptionSize="${size}"
 LineWidth="0.6" BoldWidth="2" BondLength="14.4" BondSpacing="18"
 HashSpacing="2.5" MarginWidth="1.6">
  <fonttable>${fonts.map((entry) => `<font id="${entry.id}" charset="iso-8859-1" name="${entry.name}"/>`).join("")}</fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 100 230"><fragment id="2">${rows}
  </fragment></page>
</CDXML>
`;
}

function attribute(tag, name) {
  return tag.match(new RegExp(`\\b${name}="([^"]*)"`, "i"))?.[1] ?? null;
}

function matrix(tag) {
  const values = attribute(tag, "transform")
    ?.match(/matrix\(([^)]*)\)/i)?.[1]
    ?.trim()
    .split(/[\s,]+/)
    .map(Number);
  if (!values || values.length !== 6 || values.some((value) => !Number.isFinite(value))) {
    return null;
  }
  return values;
}

function textEntries(svg) {
  return [...svg.matchAll(/<text\b[^>]*>[\s\S]*?<\/text>/gi)].map((match) => {
    const tag = match[0];
    const transform = matrix(tag);
    const text = tag
      .replace(/<[^>]+>/g, "")
      .replaceAll("&amp;", "&")
      .replaceAll("&lt;", "<")
      .replaceAll("&gt;", ">")
      .trim();
    return { text, baselineY: transform?.[5] };
  });
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
const probes = [];
const currentCreator = "ChemDraw 22.2.0";
for (const font of fonts) {
  for (const size of probeSizes) {
    for (const authoredOffset of [3.9, 15]) {
      const name = `${font.slug}-${size}-source-${String(authoredOffset).replace(".", "_")}`;
      const input = path.join(sourceDir, `${name}.cdxml`);
      const svg = path.join(oracleDir, `${name}.chemdraw.svg`);
      await fs.writeFile(input, sourceFor(font, size, authoredOffset, currentCreator), "utf8");
      probes.push({ name, font, size, authoredOffset, creator: currentCreator, input, svg });
    }
  }
}
for (const creator of [
  "ChemDraw 12.0.2.1076",
  "ChemDraw 16.0.0.82",
  "ChemDraw 18.1.0.535",
  "ChemDraw 20.0.0.38",
  "ChemDraw 21.0.0.28",
  "ChemDraw 23.1.2.7",
]) {
  for (const authoredOffset of [3.9, 15]) {
    const versionSlug = creator.replace(/^ChemDraw /, "").replaceAll(".", "_");
    const name = `creator-${versionSlug}-arial-10-source-${String(authoredOffset).replace(".", "_")}`;
    const input = path.join(sourceDir, `${name}.cdxml`);
    const svg = path.join(oracleDir, `${name}.chemdraw.svg`);
    await fs.writeFile(input, sourceFor(fonts[0], 10, authoredOffset, creator), "utf8");
    probes.push({ name, font: fonts[0], size: 10, authoredOffset, creator, input, svg });
  }
}

const missing = [];
for (const probe of probes) {
  try {
    if (refresh) throw new Error("refresh");
    await fs.access(probe.svg);
  } catch {
    missing.push(probe);
  }
}
if (missing.length > 0) {
  await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["svg"],
    inputs: missing.map((probe) => probe.input),
  });
}

const measurements = [];
for (const probe of probes) {
  const svg = await fs.readFile(probe.svg, "utf8");
  const entries = textEntries(svg);
  const pathTag = svg.match(/<path\b[^>]*>/i)?.[0];
  const documentMatrix = pathTag && matrix(pathTag);
  if (!documentMatrix) throw new Error(`${probe.name}: no document path transform`);
  const expectedTextNodes = labels.reduce((count, label) => count + (label.splitRuns ?? 1), 0);
  if (entries.length !== expectedTextNodes) {
    throw new Error(`${probe.name}: expected ${expectedTextNodes} text nodes, found ${entries.length}`);
  }
  let textIndex = 0;
  for (let index = 0; index < labels.length; index += 1) {
    const label = labels[index];
    const entry = entries[textIndex];
    if (entry.text !== (label.baseText ?? label.text) || !Number.isFinite(entry.baselineY)) {
      throw new Error(`${probe.name}: text order mismatch at ${index}: ${JSON.stringify(entry)}`);
    }
    textIndex += label.splitRuns ?? 1;
    const sourceY = 30 + index * 28;
    const nodeSvgY = documentMatrix[3] * sourceY * 20 + documentMatrix[5];
    const svgUnitsPerPoint = Math.abs(documentMatrix[3]) * 20;
    const baselineOffset = (entry.baselineY - nodeSvgY) / svgUnitsPerPoint;
    measurements.push({
      font: probe.font.name,
      size: probe.size,
      creator: probe.creator,
      authoredOffset: probe.authoredOffset,
      label: label.text,
      baselineOffset,
      baselineRatio: baselineOffset / probe.size,
    });
  }
}

const report = { version: 1, measurements };
await fs.writeFile(path.join(outDir, "report.json"), `${JSON.stringify(report, null, 2)}\n`, "utf8");
for (const font of fonts) {
  for (const size of probeSizes) {
    const rows = measurements.filter(
      (entry) => entry.creator === currentCreator && entry.font === font.name && entry.size === size && entry.authoredOffset === 3.9,
    );
    const companion = measurements.filter(
      (entry) => entry.creator === currentCreator && entry.font === font.name && entry.size === size && entry.authoredOffset === 15,
    );
    console.log(JSON.stringify({
      font: font.name,
      size,
      authoredPositionIgnored: rows.every((entry, index) =>
        Math.abs(entry.baselineOffset - companion[index].baselineOffset) < 1e-6),
      ratios: Object.fromEntries(rows.map((entry) => [entry.label, Number(entry.baselineRatio.toFixed(6))])),
    }));
  }
}
for (const creator of [...new Set(measurements.map((entry) => entry.creator))]) {
  if (creator === currentCreator) continue;
  const rows = measurements.filter(
    (entry) => entry.creator === creator && entry.authoredOffset === 3.9,
  );
  const companion = measurements.filter(
    (entry) => entry.creator === creator && entry.authoredOffset === 15,
  );
  console.log(JSON.stringify({
    creator,
    font: "Arial",
    size: 10,
    authoredPositionIgnored: rows.every((entry, index) =>
      Math.abs(entry.baselineOffset - companion[index].baselineOffset) < 1e-6),
    ratios: Object.fromEntries(rows.map((entry) => [entry.label, Number(entry.baselineRatio.toFixed(6))])),
  }));
}
