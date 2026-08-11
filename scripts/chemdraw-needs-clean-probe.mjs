import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-needs-clean-probe");
const sourceDir = path.join(outDir, "sources");
const oracleDir = path.join(outDir, "oracle");

function document({ bondLength, positions, needsClean = [] }) {
  const rootAttrs = bondLength == null ? "" : ` BondLength="${bondLength}"`;
  const elements = new Map([[2, 7], [4, 8], [5, 8]]);
  const nodes = positions.map(([x, y], index) => {
    const id = index + 1;
    const element = elements.has(id) ? ` Element="${elements.get(id)}" NumHydrogens="0"` : "";
    const clean = needsClean.includes(id) ? ' NeedsClean="yes"' : "";
    return `<n id="${id}" p="${x} ${y}"${element}${clean}/>`;
  }).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd" >
<CDXML${rootAttrs}><page><fragment>${nodes}<b B="1" E="2"/><b B="2" E="3"/><b B="3" E="4" Order="2"/><b B="3" E="5"/><b B="5" E="6"/></fragment></page></CDXML>
`;
}

function twoLabelDocument(spacing, order = 1) {
  const orderAttribute = order === 1 ? "" : ` Order="${order}"`;
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd" >
<CDXML><page><fragment>
  <n id="1" p="0 0" Element="7" NumHydrogens="0"/>
  <n id="2" p="${spacing} 0" Element="7" NumHydrogens="0"/>
  <b B="1" E="2"${orderAttribute}/>
</fragment></page></CDXML>
`;
}

function attributes(tag) {
  return Object.fromEntries(
    [...tag.matchAll(/([A-Za-z_][\w:.-]*)="([^"]*)"/g)]
      .map((match) => [match[1], match[2]]),
  );
}

function nodePositions(cdxml) {
  return [...cdxml.matchAll(/<n\b[^>]*>/g)]
    .map((match) => attributes(match[0]))
    .filter((attrs) => attrs.id && attrs.p)
    .map((attrs) => ({ id: attrs.id, p: attrs.p, needsClean: attrs.NeedsClean ?? null }));
}

function svgInkBounds(svg) {
  const viewBox = svg.match(/\bviewBox="([^"]+)"/)?.[1] ?? null;
  const dimensions = Object.fromEntries(
    [...svg.matchAll(/\b(width|height)="([^"]+)"/g)].map((match) => [match[1], match[2]]),
  );
  const textTransforms = [...svg.matchAll(/<text\b[^>]*\btransform="([^"]+)"[^>]*>/g)]
    .map((match) => match[1]);
  const paths = [...svg.matchAll(/<path\b[^>]*\btransform="([^"]+)"[^>]*\bd="([^"]+)"/g)]
    .map((match) => ({ transform: match[1], d: match[2].trim() }));
  return { viewBox, ...dimensions, textTransforms, paths };
}

const compact = [[0, 0], [1, 0], [2, 0], [3, 0], [2, 1], [3, 1]];
const normal = compact.map(([x, y]) => [x * 14.4, y * 14.4]);
const variants = [
  { name: "compact-no-flag", xml: document({ positions: compact }) },
  { name: "compact-n-needs-clean", xml: document({ positions: compact, needsClean: [2] }) },
  { name: "compact-carbon-needs-clean", xml: document({ positions: compact, needsClean: [1] }) },
  { name: "compact-all-needs-clean", xml: document({ positions: compact, needsClean: [1, 2, 3, 4, 5, 6] }) },
  { name: "compact-explicit-bond-length", xml: document({ positions: compact, needsClean: [2], bondLength: 14.4 }) },
  { name: "normal-no-flag", xml: document({ positions: normal }) },
  { name: "normal-n-needs-clean", xml: document({ positions: normal, needsClean: [2] }) },
  ...[0.5, 2, 4, 8, 10, 12].map((spacing) => ({
    name: `spacing-${String(spacing).replace(".", "_")}`,
    xml: document({
      positions: compact.map(([x, y]) => [x * spacing, y * spacing]),
      needsClean: [2],
    }),
  })),
  ...[0.5, 1, 2, 4, 8, 14.4].flatMap((spacing) => [1, 2].map((order) => ({
    name: `two-label-order-${order}-spacing-${String(spacing).replace(".", "_")}`,
    xml: twoLabelDocument(spacing, order),
  }))),
];

await fs.mkdir(sourceDir, { recursive: true });
const inputs = [];
for (const variant of variants) {
  const input = path.join(sourceDir, `${variant.name}.cdxml`);
  await fs.writeFile(input, variant.xml, "utf8");
  inputs.push(input);
}

const jobs = await generateChemDrawOracle({
  inputs,
  outDir: oracleDir,
  formats: ["svg", "cdxml"],
});

const results = [];
for (let index = 0; index < jobs.length; index += 1) {
  const [saved, svg] = await Promise.all([
    fs.readFile(jobs[index].outputs.cdxml, "utf8"),
    fs.readFile(jobs[index].outputs.svg, "utf8"),
  ]);
  results.push({
    name: variants[index].name,
    nodes: nodePositions(saved),
    svg: svgInkBounds(svg),
  });
}

const reportPath = path.join(outDir, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify({ results }, null, 2)}\n`, "utf8");
console.log(JSON.stringify({ reportPath, results }, null, 2));
