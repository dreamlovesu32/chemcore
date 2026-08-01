import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const refresh = process.argv.includes("--refresh");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(root, outputArg ?? "tmp/chemdraw-aromatic-chain-side-probe");
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");

const baseNodes = new Map([
  ["3", [216.8, 362.94]],
  ["4", [216.8, 392.94]],
  ["6", [242.78, 407.94]],
  ["7", [268.76, 392.94]],
  ["8", [268.76, 362.94]],
  ["9", [242.78, 347.94]],
]);
const ringBonds = [
  ["16", "9", "3"],
  ["17", "8", "9"],
  ["18", "7", "8"],
  ["19", "6", "7"],
  ["20", "4", "6"],
  ["21", "3", "4"],
];

const variants = [];
for (const targetBond of ["17", "18", "19", "20"]) {
  variants.push(
    { name: `open-aromatic-${targetBond}`, targetBond },
    { name: `open-aromatic-no-bco-${targetBond}`, targetBond, circular: "none" },
    { name: `open-aromatic-reverse-bco-${targetBond}`, targetBond, circular: "reverse" },
    { name: `open-order2-${targetBond}`, targetBond, order: "2" },
  );
}
for (const targetBond of ["16", "17", "18", "19", "20", "21"]) {
  variants.push({ name: `ring-aromatic-${targetBond}`, targetBond, closed: true });
}
for (const rotation of [30, 90, 137]) {
  variants.push({ name: `open-aromatic-18-rotation-${rotation}`, targetBond: "18", rotation });
}
variants.push(
  { name: "open-aromatic-18-mirror", targetBond: "18", mirror: true },
  { name: "open-aromatic-18-reverse-axis", targetBond: "18", reverseTarget: true },
  { name: "open-order2-18-default-display", targetBond: "18", order: "2", omitDisplay: true, omitDisplay2: true },
  { name: "open-order2-18-solid-solid", targetBond: "18", order: "2", display: "Solid", display2: "Solid" },
  { name: "open-order2-18-solid-dash", targetBond: "18", order: "2", display: "Solid", display2: "Dash" },
  { name: "open-aromatic-18-default-display", targetBond: "18", omitDisplay: true, omitDisplay2: true },
  { name: "open-aromatic-18-dash-only", targetBond: "18", omitDisplay2: true },
  { name: "open-aromatic-18-solid-dash", targetBond: "18", display: "Solid", display2: "Dash" },
  { name: "open-aromatic-18-solid-solid", targetBond: "18", display: "Solid", display2: "Solid" },
  { name: "order2-one-adjacent-at-begin", targetBond: "18", order: "2", omitDisplay: true, omitDisplay2: true, allowedBondIds: ["18", "19"] },
  { name: "order2-one-adjacent-at-end", targetBond: "18", order: "2", omitDisplay: true, omitDisplay2: true, allowedBondIds: ["17", "18"] },
  { name: "order2-two-adjacent-same-side", targetBond: "18", order: "2", omitDisplay: true, omitDisplay2: true, allowedBondIds: ["17", "18", "19"] },
  { name: "order2-two-adjacent-opposite-sides", targetBond: "18", order: "2", omitDisplay: true, omitDisplay2: true, allowedBondIds: ["17", "18", "19"], pointOverrides: { "9": [294.74, 347.94] } },
  { name: "order2-two-adjacent-collinear", targetBond: "18", order: "2", omitDisplay: true, omitDisplay2: true, allowedBondIds: ["17", "18", "19"], pointOverrides: { "6": [268.76, 422.94], "9": [268.76, 332.94] } },
);

function sourcePoint(id, variant) {
  return variant.pointOverrides?.[id] ?? baseNodes.get(id);
}

function transformedPoint(point, variant) {
  const center = [242.78, 377.94];
  let x = point[0] - center[0];
  let y = point[1] - center[1];
  if (variant.mirror) x = -x;
  const angle = ((variant.rotation ?? 0) * Math.PI) / 180;
  const rotated = [x * Math.cos(angle) - y * Math.sin(angle), x * Math.sin(angle) + y * Math.cos(angle)];
  return [rotated[0] + center[0], rotated[1] + center[1]];
}

function circularOrdering(id, mode) {
  const values = {
    "16": ["17", "0", "0", "21"],
    "17": ["18", "0", "0", "16"],
    "18": ["19", "0", "0", "17"],
    "19": ["20", "0", "0", "18"],
    "20": ["21", "0", "0", "19"],
    "21": ["16", "0", "0", "20"],
  }[id];
  if (mode === "none") return null;
  return (mode === "reverse" ? [...values].reverse() : values).join(" ");
}

function makeCdxml(variant) {
  const allowed = variant.allowedBondIds
    ? new Set(variant.allowedBondIds)
    : variant.closed
      ? new Set(ringBonds.map(([id]) => id))
      : new Set(["17", "18", "19", "20"]);
  const usedNodeIds = new Set(
    ringBonds
      .filter(([id]) => allowed.has(id))
      .flatMap(([, begin, end]) => [begin, end]),
  );
  const nodes = [...usedNodeIds]
    .map((id) => {
      const point = transformedPoint(sourcePoint(id, variant), variant);
      return `<n id="${id}" p="${point[0].toFixed(6)} ${point[1].toFixed(6)}"/>`;
    })
    .join("\n");
  const bonds = ringBonds
    .filter(([id]) => allowed.has(id))
    .map(([id, begin, end]) => {
      const ordering = circularOrdering(id, variant.circular);
      const attrs = [
        `id="${id}"`,
        `B="${id === variant.targetBond && variant.reverseTarget ? end : begin}"`,
        `E="${id === variant.targetBond && variant.reverseTarget ? begin : end}"`,
        `Order="${variant.order ?? "1.5"}"`,
        variant.omitDisplay ? null : `Display="${variant.display ?? "Dash"}"`,
        variant.omitDisplay2 ? null : `Display2="${variant.display2 ?? "Dash"}"`,
        id === variant.targetBond ? 'color="4"' : null,
        ordering ? `BondCircularOrdering="${ordering}"` : null,
      ].filter(Boolean);
      return `<b ${attrs.join(" ")}/>`;
    })
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" BondSpacing="12" LineWidth="1" BoldWidth="4" HashSpacing="2.7" color="0" bgcolor="1">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/><color r="1" g="0" b="0"/></colortable>
  <page id="1"><fragment id="2">${nodes}${bonds}</fragment></page>
</CDXML>\n`;
}

function parseTransform(value) {
  const values = value.match(/[-+]?\d*\.?\d+(?:e[-+]?\d+)?/gi)?.map(Number) ?? [];
  if (values.length !== 6) throw new Error(`unsupported SVG transform: ${value}`);
  return values;
}

function svgPoint(point, matrix) {
  return [
    matrix[0] * point[0] * 20 + matrix[2] * point[1] * 20 + matrix[4],
    matrix[1] * point[0] * 20 + matrix[3] * point[1] * 20 + matrix[5],
  ];
}

function centroidFromPathData(data, matrix) {
  const numbers = data.match(/[-+]?\d*\.?\d+(?:e[-+]?\d+)?/gi)?.map(Number) ?? [];
  const points = [];
  for (let index = 0; index + 1 < numbers.length; index += 2) {
    points.push([
      matrix[0] * numbers[index] + matrix[2] * numbers[index + 1] + matrix[4],
      matrix[1] * numbers[index] + matrix[3] * numbers[index + 1] + matrix[5],
    ]);
  }
  return [
    points.reduce((sum, point) => sum + point[0], 0) / points.length,
    points.reduce((sum, point) => sum + point[1], 0) / points.length,
  ];
}

function classify(svg, variant) {
  const target = ringBonds.find(([id]) => id === variant.targetBond);
  const beginId = variant.reverseTarget ? target[2] : target[1];
  const endId = variant.reverseTarget ? target[1] : target[2];
  const begin = transformedPoint(sourcePoint(beginId, variant), variant);
  const end = transformedPoint(sourcePoint(endId, variant), variant);
  const redPaths = [...svg.matchAll(/<path[^>]*fill="#(?:ff0000|FF0000)"[^>]*transform="([^"]+)"[^>]*d="([^"]+)"/g)];
  if (redPaths.length === 0) throw new Error(`${variant.name}: no red target paths in ChemDraw SVG`);
  const matrix = parseTransform(redPaths[0][1]);
  const beginSvg = svgPoint(begin, matrix);
  const endSvg = svgPoint(end, matrix);
  const dx = endSvg[0] - beginSvg[0];
  const dy = endSvg[1] - beginSvg[1];
  const length = Math.hypot(dx, dy);
  const normal = [-dy / length, dx / length];
  const distances = redPaths
    .map((match) => centroidFromPathData(match[2], parseTransform(match[1])))
    .map((center) => (center[0] - beginSvg[0]) * normal[0] + (center[1] - beginSvg[1]) * normal[1])
    .sort((left, right) => left - right);
  const negative = distances.filter((distance) => distance < -1).length;
  const positive = distances.filter((distance) => distance > 1).length;
  const axial = distances.length - negative - positive;
  const placement = axial > 0
    ? (negative > positive ? "right" : positive > negative ? "left" : "center")
    : Math.abs(distances[0] + distances.at(-1)) < 1
      ? "center"
      : distances.reduce((sum, distance) => sum + distance, 0) > 0
        ? "left"
        : "right";
  return { placement, pathCount: distances.length, negative, axial, positive, distances };
}

function expectedPlacement(variant) {
  if (
    variant.name === "open-aromatic-18-default-display" ||
    variant.name === "open-aromatic-18-dash-only" ||
    variant.name === "open-aromatic-18-solid-solid" ||
    variant.name === "order2-two-adjacent-collinear"
  ) {
    return "center";
  }
  if (variant.mirror || variant.reverseTarget) return "left";
  return "right";
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
for (const variant of variants) {
  variant.input = path.join(sourceDir, `${variant.name}.cdxml`);
  variant.svg = path.join(oracleDir, `${variant.name}.chemdraw.svg`);
  await fs.writeFile(variant.input, makeCdxml(variant), "utf8");
}

const missing = [];
for (const variant of variants) {
  if (refresh) {
    missing.push(variant);
    continue;
  }
  try {
    await fs.access(variant.svg);
  } catch {
    missing.push(variant);
  }
}
if (missing.length > 0) {
  await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["svg", "cdxml"],
    inputs: missing.map((variant) => variant.input),
  });
}

const results = [];
for (const variant of variants) {
  const svg = await fs.readFile(variant.svg, "utf8");
  const measured = classify(svg, variant);
  const expected = expectedPlacement(variant);
  if (measured.placement !== expected) {
    throw new Error(
      `${variant.name}: expected ${expected}, measured ${measured.placement}`,
    );
  }
  results.push({ name: variant.name, expected, ...measured });
}
console.log(JSON.stringify(results, null, 2));
