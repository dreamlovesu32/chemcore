import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(root, outputArg ?? "tmp/chemdraw-curly-bracket-anchor-probe");
const sourceDir = path.join(outDir, "source");
const oracleDir = path.join(outDir, "chemdraw");

const orientations = [
  ["horizontal-right-to-left", [180, 100, 60, 100]],
  ["horizontal-left-to-right", [60, 100, 180, 100]],
  ["vertical-bottom-to-top", [120, 160, 120, 40]],
  ["vertical-top-to-bottom", [120, 40, 120, 160]],
  ["diagonal-bottom-right-to-top-left", [180, 160, 60, 40]],
  ["diagonal-top-left-to-bottom-right", [60, 40, 180, 160]],
];
const probes = orientations.map(([orientation, bbox]) => [orientation, bbox]);

function source(bbox) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema curly bracket anchor probe" BondLength="30" LineWidth="1">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="1" g="0" b="0"/>
  </colortable>
  <page id="1" BoundingBox="0 0 240 200">
    <graphic id="2" BoundingBox="${bbox.join(" ")}" GraphicType="Bracket"
      BracketType="Curly" color="4" LipSize="60"/>
  </page>
</CDXML>
`;
}

function redCurlyPath(svg) {
  const paths = [...svg.matchAll(/<path\b[^>]*>/gi)].map((match) => match[0]);
  return paths.find((tag) => /\bstroke="#(?:ff0000|FF0000)"/.test(tag)) ?? null;
}

function attribute(tag, name) {
  return tag.match(new RegExp(`\\s${name}="([^"]*)"`, "i"))?.[1] ?? null;
}

function pathPoints(d) {
  const numbers = [...d.matchAll(/[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/g)]
    .map((match) => Number(match[0]));
  if (numbers.length % 2 !== 0 || numbers.some((value) => !Number.isFinite(value))) {
    throw new Error(`Curly path does not contain coordinate pairs: ${d}`);
  }
  const points = [];
  for (let index = 0; index < numbers.length; index += 2) {
    points.push([numbers[index], numbers[index + 1]]);
  }
  return points;
}

function signedNormalRange(points, bbox) {
  const [x1, y1, x2, y2] = bbox.map((value) => value * 20);
  const dx = x2 - x1;
  const dy = y2 - y1;
  const length = Math.hypot(dx, dy);
  const normal = [-dy / length, dx / length];
  const center = [(x1 + x2) * 0.5, (y1 + y2) * 0.5];
  const projections = points.map(
    ([x, y]) => (x - center[0]) * normal[0] + (y - center[1]) * normal[1],
  );
  return {
    minimum: Math.min(...projections),
    maximum: Math.max(...projections),
    centerError: Math.abs(Math.min(...projections) + Math.max(...projections)),
  };
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const results = [];
for (const [name, bbox] of probes) {
  const input = path.join(sourceDir, `${name}.cdxml`);
  await fs.writeFile(input, source(bbox), "utf8");
  await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["cdxml", "svg"],
    inputs: [input],
  });
  const svgPath = path.join(oracleDir, `${name}.chemdraw.svg`);
  const svg = await fs.readFile(svgPath, "utf8");
  const pathTag = redCurlyPath(svg);
  if (!pathTag) throw new Error(`${name}: ChemDraw SVG has no red curly path`);
  const d = attribute(pathTag, "d");
  if (!d) throw new Error(`${name}: ChemDraw curly path has no d attribute`);
  const range = signedNormalRange(pathPoints(d), bbox);
  if (range.minimum >= -1 || range.maximum <= 1 || range.centerError > 1) {
    throw new Error(`${name}: ordered BoundingBox is not the normal centerline: ${JSON.stringify(range)}`);
  }
  results.push({ name, bbox, ...range });
}

console.log(JSON.stringify(results, null, 2));
