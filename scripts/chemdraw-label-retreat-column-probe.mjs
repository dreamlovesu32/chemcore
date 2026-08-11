import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-label-retreat-column-probe");
const sourceDir = path.join(outDir, "sources");
const oracleDir = path.join(outDir, "oracle");
const candidateDir = path.join(outDir, "candidate");
const execFileAsync = promisify(execFile);

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function pointAt(angleDegrees, distance) {
  const radians = angleDegrees * Math.PI / 180;
  return [
    100 + Math.cos(radians) * distance,
    100 + Math.sin(radians) * distance,
  ];
}

function document({ angle, marginWidth, runs, labelDisplay = "Center" }) {
  const neighbor = pointAt(angle, 16);
  const runXml = runs.map((run) => (
    `<s font="4" size="${run.size ?? 7}" face="${run.face ?? 96}" color="0">${escapeXml(run.text)}</s>`
  )).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd" >
<CDXML BondLength="16" LineWidth="0.5" MarginWidth="${marginWidth}">
  <fonttable><font id="4" charset="0" name="Times New Roman"/></fonttable>
  <colortable><color r="0" g="0" b="0"/></colortable>
  <page>
    <fragment>
      <n id="1" p="100 100" NodeType="Unspecified" LabelDisplay="${labelDisplay}">
        <t id="3" p="100 102.65" LabelAlignment="${labelDisplay}" LabelJustification="${labelDisplay}" Justification="${labelDisplay}" InterpretChemically="yes">${runXml}</t>
      </n>
      <n id="2" p="${neighbor[0].toFixed(6)} ${neighbor[1].toFixed(6)}"/>
      <b id="4" B="1" E="2"/>
    </fragment>
  </page>
</CDXML>
`;
}

function attributes(tag) {
  return Object.fromEntries(
    [...tag.matchAll(/([A-Za-z_][\w:.-]*)="([^"]*)"/g)]
      .map((match) => [match[1], match[2]]),
  );
}

function point(value) {
  const values = String(value).trim().split(/\s+/).map(Number);
  return { x: values[0], y: values[1] };
}

function savedGeometry(cdxml) {
  const nodes = [...cdxml.matchAll(/<n\b([^>]*?)\/?>/g)]
    .map((match) => attributes(match[1]))
    .filter((entry) => entry.id === "1" || entry.id === "2");
  const label = point(nodes.find((entry) => entry.id === "1").p);
  const neighbor = point(nodes.find((entry) => entry.id === "2").p);
  const textTag = cdxml.match(/<t\b([^>]*)>/)?.[1] ?? "";
  return {
    label,
    neighbor,
    text: attributes(textTag),
  };
}

function bondPolygonWorld(svg) {
  const pathTags = [...svg.matchAll(/<path\b([^>]*)\/?>/g)]
    .map((match) => attributes(match[1]))
    .filter((entry) => entry.d);
  if (pathTags.length !== 1) {
    throw new Error(`Expected one bond path, found ${pathTags.length}`);
  }
  const numbers = [...pathTags[0].d.matchAll(/-?(?:\d+(?:\.\d*)?|\.\d+)/g)]
    .map((match) => Number(match[0]));
  const points = [];
  for (let index = 0; index + 1 < numbers.length; index += 2) {
    points.push({ x: numbers[index] / 20, y: numbers[index + 1] / 20 });
  }
  return points;
}

function candidateBondPolygonWorld(svg) {
  const polygonTags = [...svg.matchAll(/<polygon\b([^>]*)\/?>/g)]
    .map((match) => attributes(match[1]))
    .filter((entry) => entry.points);
  if (polygonTags.length !== 1) {
    throw new Error(`Expected one candidate bond polygon, found ${polygonTags.length}`);
  }
  const numbers = [...polygonTags[0].points.matchAll(/-?(?:\d+(?:\.\d*)?|\.\d+)/g)]
    .map((match) => Number(match[0]));
  const points = [];
  for (let index = 0; index + 1 < numbers.length; index += 2) {
    points.push({ x: numbers[index], y: numbers[index + 1] });
  }
  return points;
}

function retreatFromLabel(label, neighbor, polygon) {
  const dx = neighbor.x - label.x;
  const dy = neighbor.y - label.y;
  const length = Math.hypot(dx, dy);
  const unit = { x: dx / length, y: dy / length };
  const projections = polygon.map((entry) => (
    (entry.x - label.x) * unit.x + (entry.y - label.y) * unit.y
  ));
  return {
    retreat: Math.min(...projections),
    farCap: Math.max(...projections),
    authoredLength: length,
  };
}

const labels = [
  { name: "lys", runs: [{ text: "Lys" }] },
  { name: "abc", runs: [{ text: "ABC" }] },
  { name: "mim", runs: [{ text: "MIM" }] },
  { name: "iii", runs: [{ text: "III" }] },
  { name: "www", runs: [{ text: "WWW" }] },
  { name: "parenthesized", runs: [{ text: "(Aax)" }] },
  {
    name: "ch3",
    runs: [{ text: "CH" }, { text: "3", face: 32 }],
  },
  {
    name: "f3c",
    runs: [{ text: "F" }, { text: "3", face: 32 }, { text: "C" }],
  },
];
const angles = [-90, -45, 0, 45, 90, 135, 180, 225];
const margins = [0.5, 1.25, 2.5];
const variants = [];
for (const label of labels) {
  for (const angle of angles) {
    for (const marginWidth of margins) {
      variants.push({
        name: `${label.name}_a${String(angle).replace("-", "m")}_m${String(marginWidth).replace(".", "_")}`,
        angle,
        marginWidth,
        runs: label.runs,
      });
    }
  }
}

await fs.mkdir(sourceDir, { recursive: true });
const inputs = [];
for (const variant of variants) {
  const input = path.join(sourceDir, `${variant.name}.cdxml`);
  await fs.writeFile(input, document(variant), "utf8");
  inputs.push(input);
}

const oracleOutputs = variants.map((variant) => ({
  svg: path.join(oracleDir, `${variant.name}.chemdraw.svg`),
  cdxml: path.join(oracleDir, `${variant.name}.chemdraw.cdxml`),
}));
const oracleComplete = (
  await Promise.all(oracleOutputs.flatMap((output) => (
    [output.svg, output.cdxml].map((file) => fs.stat(file).then(() => true, () => false))
  )))
).every(Boolean);
if (!oracleComplete) {
  await generateChemDrawOracle({
    inputs,
    outDir: oracleDir,
    formats: ["svg", "cdxml"],
  });
}

await fs.mkdir(candidateDir, { recursive: true });
const cli = path.join(root, "target", "debug", process.platform === "win32"
  ? "chemsema-cli.exe"
  : "chemsema-cli");
const candidateOutputs = variants.map((variant) => (
  path.join(candidateDir, `${variant.name}.chemsema.svg`)
));
let nextCandidate = 0;
await Promise.all(Array.from({ length: 8 }, async () => {
  while (nextCandidate < inputs.length) {
    const index = nextCandidate;
    nextCandidate += 1;
    await execFileAsync(cli, ["convert", inputs[index], candidateOutputs[index]], {
      cwd: root,
      maxBuffer: 16 * 1024 * 1024,
    });
  }
}));

const results = [];
for (let index = 0; index < variants.length; index += 1) {
  const [saved, svg, candidateSvg] = await Promise.all([
    fs.readFile(oracleOutputs[index].cdxml, "utf8"),
    fs.readFile(oracleOutputs[index].svg, "utf8"),
    fs.readFile(candidateOutputs[index], "utf8"),
  ]);
  const geometry = savedGeometry(saved);
  const polygon = bondPolygonWorld(svg);
  const candidatePolygon = candidateBondPolygonWorld(candidateSvg);
  const measured = retreatFromLabel(geometry.label, geometry.neighbor, polygon);
  const candidate = retreatFromLabel(geometry.label, geometry.neighbor, candidatePolygon);
  results.push({
    ...variants[index],
    saved: geometry,
    polygon,
    measured,
    candidatePolygon,
    candidate,
    retreatDelta: candidate.retreat - measured.retreat,
  });
}

const reportPath = path.join(outDir, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify({ results }, null, 2)}\n`, "utf8");
const absoluteErrors = results.map((entry) => Math.abs(entry.retreatDelta));
console.log(JSON.stringify({
  reportPath,
  count: results.length,
  meanAbsoluteError: absoluteErrors.reduce((sum, value) => sum + value, 0) / absoluteErrors.length,
  maximumAbsoluteError: Math.max(...absoluteErrors),
}, null, 2));
