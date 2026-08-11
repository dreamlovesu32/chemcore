import fs from "node:fs/promises";
import crypto from "node:crypto";
import path from "node:path";
import process from "node:process";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
if (!process.argv[2]) {
  throw new Error(
    "Usage: node scripts/chemdraw-collapsed-fragment-bond-style-probe.mjs "
      + "<ChemDraw-saved-US20190278121A1-20190912-C00009.cdxml> [output-dir]",
  );
}
const source = path.resolve(process.argv[2]);
const outputRoot = path.resolve(
  process.argv[3]
    ?? path.join(root, "tmp", "chemdraw-collapsed-fragment-bond-style-probe"),
);
const sourceDir = path.join(outputRoot, "source");
const oracleDir = path.join(outputRoot, "oracle");

function replaceBondAttribute(xml, id, attribute, value) {
  let matched = false;
  const result = xml.replace(/<b\b[\s\S]*?\/>/g, (tag) => {
    if (!new RegExp(`\\bid="${id}"(?:\\s|/|>)`).test(tag)) {
      return tag;
    }
    matched = true;
    const attributePattern = new RegExp(`\\s${attribute}="[^"]*"`);
    if (value == null) {
      return tag.replace(attributePattern, "");
    }
    if (attributePattern.test(tag)) {
      return tag.replace(attributePattern, ` ${attribute}="${value}"`);
    }
    return tag.replace(/\/>$/, ` ${attribute}="${value}"\n/>`);
  });
  if (!matched) {
    throw new Error(`Bond ${id} was not found in ${source}.`);
  }
  return result;
}

function replaceNodeAttribute(xml, id, attribute, value) {
  let matched = false;
  const result = xml.replace(/<n\b[^>]*>/g, (tag) => {
    if (!new RegExp(`\\bid="${id}"(?:\\s|/|>)`).test(tag)) {
      return tag;
    }
    matched = true;
    const attributePattern = new RegExp(`\\s${attribute}="[^"]*"`);
    if (attributePattern.test(tag)) {
      return tag.replace(attributePattern, ` ${attribute}="${value}"`);
    }
    return tag.replace(/>$/, ` ${attribute}="${value}">`);
  });
  if (!matched) {
    throw new Error(`Node ${id} was not found in ${source}.`);
  }
  return result;
}

function completeBeginFragmentConnection(xml, internalDisplay) {
  let result = replaceNodeAttribute(xml, "56", "BondOrdering", "57 59 233");
  result = result.replace(
    /(<fragment\b[^>]*\bid="301"[^>]*\bConnectionOrder=")244 245("[^>]*>)/,
    (_, prefix, suffix) => `${prefix}244 245 9001${suffix}`,
  );
  const display = internalDisplay === "Dash" ? ' Display="Dash"' : "";
  const fragmentPattern = /(<fragment\b[^>]*\bid="301"[^>]*>[\s\S]*?)(<\/fragment>)/;
  if (!fragmentPattern.test(result)) {
    throw new Error("Nested fragment 301 was not found.");
  }
  return result.replace(
    fragmentPattern,
    `$1<n id="9001" p="138.00 68.00" NodeType="ExternalConnectionPoint"/>\n`
      + `<b id="9002" B="9001" E="243"${display}/>\n$2`,
  );
}

function flattenCollapsedNode(xml, nodeId, fragmentId) {
  let result = replaceNodeAttribute(xml, nodeId, "NodeType", "Unspecified");
  const fragmentPattern = new RegExp(
    `<fragment\\b[^>]*\\bid="${fragmentId}"[^>]*>[\\s\\S]*?<\\/fragment>`,
  );
  if (!fragmentPattern.test(result)) {
    throw new Error(`Nested fragment ${fragmentId} was not found.`);
  }
  return result.replace(fragmentPattern, "");
}

const baseline = await fs.readFile(source, "utf8");
const probes = [
  { name: "baseline", xml: baseline },
  {
    name: "begin-internal-dash",
    xml: replaceBondAttribute(baseline, "247", "Display", "Dash"),
  },
  {
    name: "outer-solid",
    xml: replaceBondAttribute(baseline, "233", "Display", "Solid"),
  },
  {
    name: "begin-internal-dash-outer-solid",
    xml: replaceBondAttribute(
      replaceBondAttribute(baseline, "247", "Display", "Dash"),
      "233",
      "Display",
      "Solid",
    ),
  },
  {
    name: "without-begin-attach",
    xml: replaceBondAttribute(baseline, "233", "BeginAttach", null),
  },
  {
    name: "begin-node-unspecified",
    xml: replaceNodeAttribute(baseline, "56", "NodeType", "Unspecified"),
  },
  {
    name: "end-node-unspecified",
    xml: replaceNodeAttribute(baseline, "128", "NodeType", "Unspecified"),
  },
  {
    name: "both-nodes-unspecified",
    xml: replaceNodeAttribute(
      replaceNodeAttribute(baseline, "56", "NodeType", "Unspecified"),
      "128",
      "NodeType",
      "Unspecified",
    ),
  },
  {
    name: "complete-begin-mapping-solid-internal",
    xml: completeBeginFragmentConnection(baseline, "Solid"),
  },
  {
    name: "complete-begin-mapping-dash-internal",
    xml: completeBeginFragmentConnection(baseline, "Dash"),
  },
  {
    name: "flatten-begin-node",
    xml: flattenCollapsedNode(baseline, "56", "301"),
  },
  {
    name: "flatten-end-node",
    xml: flattenCollapsedNode(baseline, "128", "311"),
  },
  {
    name: "flatten-both-nodes",
    xml: flattenCollapsedNode(
      flattenCollapsedNode(baseline, "56", "301"),
      "128",
      "311",
    ),
  },
];

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
for (const probe of probes) {
  probe.input = path.join(sourceDir, `${probe.name}.cdxml`);
  await fs.writeFile(probe.input, probe.xml);
}

await generateChemDrawOracle({
  outDir: oracleDir,
  formats: ["svg", "cdxml"],
  inputs: probes.map((probe) => probe.input),
});

const report = [];
for (const probe of probes) {
  const svgPath = path.join(oracleDir, `${probe.name}.chemdraw.svg`);
  const cdxmlPath = path.join(oracleDir, `${probe.name}.chemdraw.cdxml`);
  const svg = await fs.readFile(svgPath, "utf8");
  report.push({
    name: probe.name,
    pathCount: [...svg.matchAll(/<path\b/g)].length,
    svgSha256: crypto.createHash("sha256").update(svg).digest("hex"),
    svg: path.relative(root, svgPath),
    cdxml: path.relative(root, cdxmlPath),
  });
}

const byName = new Map(report.map((probe) => [probe.name, probe]));
const baselineResult = byName.get("baseline");
const solidEquivalentNames = [
  "begin-internal-dash",
  "outer-solid",
  "begin-internal-dash-outer-solid",
  "begin-node-unspecified",
  "end-node-unspecified",
  "both-nodes-unspecified",
  "complete-begin-mapping-solid-internal",
  "complete-begin-mapping-dash-internal",
];
for (const name of solidEquivalentNames) {
  const result = byName.get(name);
  if (result.svgSha256 !== baselineResult.svgSha256) {
    throw new Error(`${name} changed the collapsed-fragment solid display.`);
  }
}
for (const name of ["flatten-begin-node", "flatten-end-node", "flatten-both-nodes"]) {
  const result = byName.get(name);
  if (result.pathCount !== baselineResult.pathCount + 14) {
    throw new Error(
      `${name} should replace one solid path with 15 dashed paths; `
        + `expected ${baselineResult.pathCount + 14}, received ${result.pathCount}.`,
    );
  }
}

const reportPath = path.join(outputRoot, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify({ source, probes: report }, null, 2)}\n`);
console.log(JSON.stringify({ report: path.relative(root, reportPath), probes: report }, null, 2));
