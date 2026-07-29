import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const profileIndex = process.argv.indexOf("--profile");
const profile = profileIndex >= 0 ? process.argv[profileIndex + 1] : "vertical";
if (!["vertical", "horizontal-attachment"].includes(profile)) {
  throw new Error(`Unsupported profile ${profile}`);
}
const outDir = path.join(
  root,
  "tmp",
  profile === "vertical"
    ? "chemdraw-flat-baseline-axis-contact-probe"
    : "chemdraw-horizontal-attachment-axis-contact-probe",
);
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

function document({
  font,
  fontId,
  size,
  marginWidth,
  text,
  direction,
  attachment,
}) {
  const neighbor = {
    up: [100, 84],
    down: [100, 116],
    left: [84, 100],
    right: [116, 100],
  }[direction];
  const beginAttach = attachment === "indexed"
    ? ` BeginAttach="${direction === "left" ? 0 : [...text].length - 1}"`
    : "";
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd" >
<CDXML BondLength="16" LineWidth="0.5" MarginWidth="${marginWidth}" LabelFont="${fontId}" LabelSize="${size}">
  <fonttable><font id="${fontId}" charset="0" name="${escapeXml(font)}"/></fonttable>
  <colortable><color r="0" g="0" b="0"/></colortable>
  <page>
    <fragment>
      <n id="1" p="100 100" NodeType="Unspecified" LabelDisplay="Center">
        <t id="3" p="100 102.65" LabelAlignment="Center" LabelJustification="Center" Justification="Center" InterpretChemically="yes">
          <s font="${fontId}" size="${size}" face="96" color="0">${escapeXml(text)}</s>
        </t>
      </n>
      <n id="2" p="${neighbor.join(" ")}"/>
      <b id="4" B="1" E="2"${beginAttach}/>
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
  const text = attributes(cdxml.match(/<t\b([^>]*)>/)?.[1] ?? "");
  return {
    label: point(nodes.find((entry) => entry.id === "1").p),
    neighbor: point(nodes.find((entry) => entry.id === "2").p),
    text,
  };
}

function numericPoints(value, scale = 1) {
  const numbers = [...value.matchAll(/-?(?:\d+(?:\.\d*)?|\.\d+)/g)]
    .map((match) => Number(match[0]) / scale);
  const points = [];
  for (let index = 0; index + 1 < numbers.length; index += 2) {
    points.push({ x: numbers[index], y: numbers[index + 1] });
  }
  return points;
}

function oracleBondPolygon(svg) {
  const paths = [...svg.matchAll(/<path\b([^>]*)\/?>/g)]
    .map((match) => attributes(match[1]))
    .filter((entry) => entry.d);
  if (paths.length !== 1) {
    throw new Error(`Expected one ChemDraw bond path, found ${paths.length}`);
  }
  return numericPoints(paths[0].d, 20);
}

function candidateBondPolygon(svg) {
  const polygons = [...svg.matchAll(/<polygon\b([^>]*)\/?>/g)]
    .map((match) => attributes(match[1]))
    .filter((entry) => entry.points);
  if (polygons.length !== 1) {
    throw new Error(`Expected one ChemSema bond polygon, found ${polygons.length}`);
  }
  return numericPoints(polygons[0].points);
}

function retreatFromLabel(label, neighbor, polygon) {
  const dx = neighbor.x - label.x;
  const dy = neighbor.y - label.y;
  const length = Math.hypot(dx, dy);
  const unit = { x: dx / length, y: dy / length };
  return Math.min(...polygon.map((entry) => (
    (entry.x - label.x) * unit.x + (entry.y - label.y) * unit.y
  )));
}

const fonts = [
  { name: "Times New Roman", id: 4 },
  { name: "Arial", id: 5 },
  { name: "Calibri", id: 6 },
];
const sizes = [7, 10, 14];
const margins = [0.5, 1.25, 2.5];
const glyphs = profile === "vertical"
  ? ["T", "I", "O", "y", "g"]
  : ["Tyr", "Lys", "Arg", "Gly"];
const directions = profile === "vertical" ? ["up", "down"] : ["left", "right"];
const attachments = profile === "vertical" ? ["none"] : ["none", "indexed"];
const variants = [];
for (const font of fonts) {
  for (const size of sizes) {
    for (const marginWidth of margins) {
      for (const text of glyphs) {
        for (const direction of directions) {
          for (const attachment of attachments) {
            variants.push({
              name: [
                font.name.toLowerCase().replaceAll(" ", "-"),
                `s${size}`,
                `m${String(marginWidth).replace(".", "_")}`,
                text,
                direction,
                attachment,
              ].join("_"),
              font: font.name,
              fontId: font.id,
              size,
              marginWidth,
              text,
              direction,
              attachment,
            });
          }
        }
      }
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
const cli = path.join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "chemsema-cli.exe" : "chemsema-cli",
);
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
      env: {
        ...process.env,
        CHEMSEMA_CLI_DISABLE_CACHE: "1",
      },
    });
  }
}));

const results = [];
for (let index = 0; index < variants.length; index += 1) {
  const [saved, oracleSvg, candidateSvg] = await Promise.all([
    fs.readFile(oracleOutputs[index].cdxml, "utf8"),
    fs.readFile(oracleOutputs[index].svg, "utf8"),
    fs.readFile(candidateOutputs[index], "utf8"),
  ]);
  const geometry = savedGeometry(saved);
  const oracleRetreat = retreatFromLabel(
    geometry.label,
    geometry.neighbor,
    oracleBondPolygon(oracleSvg),
  );
  const candidateRetreat = retreatFromLabel(
    geometry.label,
    geometry.neighbor,
    candidateBondPolygon(candidateSvg),
  );
  results.push({
    ...variants[index],
    savedText: geometry.text,
    oracleRetreat,
    candidateRetreat,
    delta: candidateRetreat - oracleRetreat,
  });
}

const reportPath = path.join(outDir, "report.json");
await fs.writeFile(
  reportPath,
  `${JSON.stringify({ profile, results }, null, 2)}\n`,
  "utf8",
);
const absoluteErrors = results.map((entry) => Math.abs(entry.delta));
console.log(JSON.stringify({
  reportPath,
  profile,
  count: results.length,
  meanAbsoluteError: absoluteErrors.reduce((sum, value) => sum + value, 0)
    / absoluteErrors.length,
  maximumAbsoluteError: Math.max(...absoluteErrors),
}, null, 2));
