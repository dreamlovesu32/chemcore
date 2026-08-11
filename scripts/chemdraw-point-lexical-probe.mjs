import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(root, outputArg ?? "tmp/chemdraw-point-lexical-probe");
const sourceDir = path.join(outDir, "source");
const oracleDir = path.join(outDir, "chemdraw");

const probes = [
  ["valid", "150 100", "150 100"],
  ["invalid-second", "150 foofoo", "150 0"],
  ["invalid-first", "foofoo 100", "0 0"],
  ["missing-second", "150", "150 0"],
  ["empty", "", "0 0"],
  ["extra-token", "150 100 extra", "150 100"],
  ["numeric-prefix-first", "150foo 100", "150 0"],
  ["numeric-prefix-second", "150 100foo", "150 100"],
  ["exponent", "1.5e2 1e2", "150 100"],
  ["signed", "+150 -20", "150 -20"],
  ["nan-first", "NaN 100", "0.25 100"],
  ["nan-second", "150 NaN", "150 0.25"],
  ["nan-lowercase", "nan 100", "0.25 100"],
  ["infinity-first", "Infinity 100", "0.25 100"],
  ["negative-infinity-first", "-Infinity 100", "0.25 100"],
  ["short-inf-first", "inf 100", "0.25 100"],
  ["overflow-first", "1e999 100", "0.25 100"],
  ["underflow-first", "1e-999 100", "0 100"],
  ["incomplete-exponent-first", "1e 100", "1 0"],
  ["leading-dot-first", ".5 100", "0.50 100"],
  ["comma", "150,100", "150 0"],
];

function source(point) {
  const escaped = point
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;");
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema ChemDraw point lexical probe" BondLength="30" LineWidth="1">
  <page id="1" BoundingBox="0 0 240 200">
    <fragment id="10">
      <n id="1" p="100 100"/>
      <n id="2" p="${escaped}"/>
      <b id="3" B="1" E="2"/>
    </fragment>
  </page>
</CDXML>
`;
}

function attribute(tag, name) {
  return tag.match(new RegExp(`\\s${name}="([^"]*)"`, "i"))?.[1] ?? null;
}

function exportedPoint(cdxml) {
  const node = cdxml.match(/<n\b[^>]*\bid="2"[^>]*>/i)?.[0];
  return node ? attribute(node, "p") : null;
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const results = [];
for (const [name, point, expectedPoint] of probes) {
  const input = path.join(sourceDir, `${name}.cdxml`);
  const output = path.join(oracleDir, `${name}.chemdraw.cdxml`);
  await fs.writeFile(input, source(point), "utf8");
  try {
    await generateChemDrawOracle({
      outDir: oracleDir,
      formats: ["cdxml", "svg"],
      inputs: [input],
    });
    const exported = await fs.readFile(output, "utf8");
    const actualPoint = exportedPoint(exported);
    if (actualPoint !== expectedPoint) {
      throw new Error(`${name}: expected exported p=${expectedPoint}, received ${actualPoint}`);
    }
    results.push({
      name,
      inputPoint: point,
      status: "opened",
      expectedPoint,
      exportedPoint: actualPoint,
    });
  } catch (error) {
    results.push({
      name,
      inputPoint: point,
      status: "rejected",
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

console.log(JSON.stringify(results, null, 2));
