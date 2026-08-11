import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(repoRoot, "tmp", "chemdraw-coordinate-free-layout-probe");
const inputDir = path.join(outDir, "input");
const oracleDir = path.join(outDir, "oracle");

const probes = [
  { name: "path4", nodes: 4, edges: [[1, 2], [2, 3], [3, 4]] },
  { name: "cycle3", nodes: 3, edges: [[1, 2], [2, 3], [3, 1]] },
  { name: "cycle4", nodes: 4, edges: [[1, 2], [2, 3], [3, 4], [4, 1]] },
  { name: "cycle5", nodes: 5, edges: [[1, 2], [2, 3], [3, 4], [4, 5], [5, 1]] },
  { name: "cycle6", nodes: 6, edges: [[1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 1]] },
  { name: "cycle7", nodes: 7, edges: [[1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 1]] },
  { name: "cycle8", nodes: 8, edges: [[1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 8], [8, 1]] },
  { name: "cycle9", nodes: 9, edges: [[1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 7], [7, 8], [8, 9], [9, 1]] },
  { name: "star3", nodes: 4, edges: [[1, 2], [1, 3], [1, 4]] },
  { name: "star4", nodes: 5, edges: [[1, 2], [1, 3], [1, 4], [1, 5]] },
  { name: "branched5", nodes: 5, edges: [[1, 2], [2, 3], [2, 4], [4, 5]] },
  {
    name: "fused6x6",
    nodes: 10,
    edges: [
      [1, 2], [2, 3], [3, 4], [4, 5], [5, 6], [6, 1],
      [3, 7], [7, 8], [8, 9], [9, 10], [10, 4],
    ],
  },
];

function inputXml(probe) {
  const nodes = Array.from({ length: probe.nodes }, (_, index) =>
    `      <n id="${index + 1}" Element="6" AbnormalValence="yes"/>`,
  ).join("\n");
  const bonds = probe.edges.map(([begin, end], index) =>
    `      <b id="${probe.nodes + index + 1}" B="${begin}" E="${end}"/>`,
  ).join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="17">
  <page id="100">
    <fragment id="101">
${nodes}
${bonds}
    </fragment>
  </page>
</CDXML>
`;
}

function attributes(tag) {
  return Object.fromEntries(
    [...tag.matchAll(/([A-Za-z_][\w:.-]*)="([^"]*)"/g)].map((match) => [match[1], match[2]]),
  );
}

function nodePositions(xml) {
  return [...xml.matchAll(/<n\b[\s\S]*?\/>/gi)]
    .map((match) => attributes(match[0]))
    .filter((attrs) => attrs.id && attrs.p)
    .map((attrs) => ({ id: attrs.id, point: attrs.p.split(/\s+/).map(Number) }));
}

function distance(left, right) {
  return Math.hypot(left[0] - right[0], left[1] - right[1]);
}

await fs.mkdir(inputDir, { recursive: true });
const inputs = [];
for (const probe of probes) {
  const input = path.join(inputDir, `${probe.name}.cdxml`);
  await fs.writeFile(input, inputXml(probe), "utf8");
  inputs.push(input);
}

await generateChemDrawOracle({
  inputs,
  outDir: oracleDir,
  formats: ["cdxml"],
});

const report = [];
for (const probe of probes) {
  const output = path.join(oracleDir, `${probe.name}.chemdraw.cdxml`);
  const xml = await fs.readFile(output, "utf8");
  const positions = nodePositions(xml);
  const byId = new Map(positions.map((entry) => [Number(entry.id), entry.point]));
  const bondLengths = probe.edges.map(([begin, end]) =>
    distance(byId.get(begin), byId.get(end)),
  );
  report.push({
    name: probe.name,
    edges: probe.edges,
    positions,
    bondLengths,
    maxBondLengthError: Math.max(...bondLengths.map((length) => Math.abs(length - 17))),
  });
}

const reportPath = path.join(outDir, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
console.log(reportPath);
