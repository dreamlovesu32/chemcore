import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const args = process.argv.slice(2);
const reuse = args.includes("--reuse");
const outputRoot = path.resolve(
  args.find((argument) => !argument.startsWith("--")) ?? "tmp/chemdraw-hash-bond-probe",
);
const sourceDir = path.join(outputRoot, "cdxml");
const oracleDir = path.join(outputRoot, "chemdraw");
await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const cases = [];
function add(name, options = {}) {
  cases.push({
    name,
    length: options.length ?? 30,
    angle: options.angle ?? 0,
    lineWidth: options.lineWidth ?? 1,
    boldWidth: options.boldWidth ?? 4,
    hashSpacing: options.hashSpacing ?? 2.7,
    order: options.order ?? "1",
    display: Object.hasOwn(options, "display") ? options.display : "Hash",
    display2: options.display2 ?? null,
    doublePosition: options.doublePosition ?? null,
    bondOverrides: options.bondOverrides ?? {},
  });
}

for (const angle of [0, 30, 45, 90, 180, -90, -135]) {
  add(`direction-${String(angle).replace("-", "m")}`, { angle });
}
for (const length of [0.5, 0.99, 1, 1.01, 2, 3.69, 3.7, 3.71, 6.39, 6.4, 6.41, 10, 14.4, 30]) {
  add(`length-${String(length).replace(".", "_")}`, { length });
}
add("acs", { length: 14.4, lineWidth: 0.6, boldWidth: 2, hashSpacing: 2.5 });
add("line-width-0_6", { lineWidth: 0.6 });
add("line-width-1_4", { lineWidth: 1.4 });
add("bold-width-2", { boldWidth: 2 });
add("bold-width-6", { boldWidth: 6 });
add("hash-spacing-2", { hashSpacing: 2 });
add("hash-spacing-3_4", { hashSpacing: 3.4 });
add("bond-overrides", {
  bondOverrides: { LineWidth: 0.6, BoldWidth: 6, HashSpacing: 2 },
});

for (const order of ["1", "1.5", "2", "3"]) {
  const positions = order === "2" ? ["Center", "Left", "Right"] : [null];
  for (const doublePosition of positions) {
    const suffix = doublePosition ? `-${doublePosition.toLowerCase()}` : "";
    add(`order-${order}-display-hash${suffix}`, { order, doublePosition });
    add(`order-${order}-display2-hash${suffix}`, {
      order,
      display: null,
      display2: "Hash",
      doublePosition,
    });
    add(`order-${order}-solid-display2-hash${suffix}`, {
      order,
      display: "Solid",
      display2: "Hash",
      doublePosition,
    });
    add(`order-${order}-display-hash-display2-hash${suffix}`, {
      order,
      display: "Hash",
      display2: "Hash",
      doublePosition,
    });
  }
}

function fixed(value) {
  return Number(value).toFixed(4).replace(/\.?(?:0+)$/, "");
}

for (const entry of cases) {
  const radians = (entry.angle * Math.PI) / 180;
  entry.begin = [30, 30];
  entry.end = [
    entry.begin[0] + entry.length * Math.cos(radians),
    entry.begin[1] + entry.length * Math.sin(radians),
  ];
  const attrs = [
    `Order="${entry.order}"`,
    entry.display ? `Display="${entry.display}"` : null,
    entry.display2 ? `Display2="${entry.display2}"` : null,
    entry.doublePosition ? `DoublePosition="${entry.doublePosition}"` : null,
    ...Object.entries(entry.bondOverrides).map(([key, value]) => `${key}="${value}"`),
  ].filter(Boolean).join(" ");
  entry.input = path.join(sourceDir, `${entry.name}.cdxml`);
  await fs.writeFile(entry.input, `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BoundingBox="-20 -20 100 100" LineWidth="${entry.lineWidth}" BoldWidth="${entry.boldWidth}" HashSpacing="${entry.hashSpacing}" BondLength="${entry.length}" BondSpacing="18">
  <page id="1" BoundingBox="-20 -20 100 100"><fragment id="2">
    <n id="3" p="${fixed(entry.begin[0])} ${fixed(entry.begin[1])}"/>
    <n id="4" p="${fixed(entry.end[0])} ${fixed(entry.end[1])}"/>
    <b id="5" B="3" E="4" ${attrs}/>
  </fragment></page>
</CDXML>
`, "utf8");
}

const oracleInputs = [];
for (const entry of cases) {
  const svgPath = path.join(oracleDir, `${entry.name}.chemdraw.svg`);
  if (!reuse || !(await fs.stat(svgPath).then(() => true, () => false))) {
    oracleInputs.push(entry.input);
  }
}
if (oracleInputs.length > 0) {
  await generateChemDrawOracle({
    outDir: oracleDir,
    formats: ["svg"],
    inputs: oracleInputs,
  });
}

function blackPaths(svg) {
  return [...svg.matchAll(/<path\b[^>]*fill="#000000"[^>]*d="([^"]+)"[^>]*\/>/gi)]
    .map((match) => [...match[1].matchAll(/-?\d+(?:\.\d+)?/g)].map((value) => Number(value[0])));
}

function boundsAlongAxis(numbers, unit, normal) {
  const points = [];
  for (let index = 0; index + 1 < numbers.length; index += 2) {
    points.push([numbers[index] / 20, numbers[index + 1] / 20]);
  }
  const along = points.map(([x, y]) => x * unit[0] + y * unit[1]);
  const across = points.map(([x, y]) => x * normal[0] + y * normal[1]);
  return {
    alongMin: Math.min(...along),
    alongMax: Math.max(...along),
    alongLength: Math.max(...along) - Math.min(...along),
    acrossLength: Math.max(...across) - Math.min(...across),
  };
}

const rounded = (value) => Number(value.toFixed(4));
const measurements = [];
const failures = [];
for (const entry of cases) {
  const svgPath = path.join(oracleDir, `${entry.name}.chemdraw.svg`);
  const svg = await fs.readFile(svgPath, "utf8");
  const unit = [
    (entry.end[0] - entry.begin[0]) / entry.length,
    (entry.end[1] - entry.begin[1]) / entry.length,
  ];
  const normal = [-unit[1], unit[0]];
  const beginProjection = entry.begin[0] * unit[0] + entry.begin[1] * unit[1];
  const endProjection = beginProjection + entry.length;
  const lineWidth = entry.bondOverrides.LineWidth ?? entry.lineWidth;
  const boldWidth = entry.bondOverrides.BoldWidth ?? entry.boldWidth;
  const hashSpacing = entry.bondOverrides.HashSpacing ?? entry.hashSpacing;
  const shapes = blackPaths(svg).map((numbers) => boundsAlongAxis(numbers, unit, normal));
  const stripes = shapes
    .filter((shape) =>
      Math.abs(shape.alongLength - lineWidth) < 0.08
      && Math.abs(shape.acrossLength - boldWidth) < 0.12)
    .sort((left, right) => left.alongMin - right.alongMin);
  const active = entry.display === "Hash"
    && (entry.order === "1" || (entry.order === "1.5" && entry.display2 === null));
  const expectedCount = !active || entry.length < lineWidth
    ? 0
    : Math.max(1, 1 + Math.floor((entry.length - lineWidth + 1e-9) / hashSpacing));
  const pitches = stripes.slice(1).map((shape, index) => shape.alongMin - stripes[index].alongMin);
  const expectedPitch = expectedCount > 1
    ? (entry.length - lineWidth) / (expectedCount - 1)
    : null;
  const beginInset = stripes.length ? stripes[0].alongMin - beginProjection : null;
  const endInset = stripes.length ? endProjection - stripes.at(-1).alongMax : null;
  const matches = stripes.length === expectedCount
    && stripes.every((stripe) => Math.abs(stripe.alongLength - lineWidth) < 0.08)
    && stripes.every((stripe) => Math.abs(stripe.acrossLength - boldWidth) < 0.12)
    && (expectedCount === 0 || Math.abs(beginInset) < 0.08)
    && (expectedCount <= 1 || Math.abs(endInset) < 0.08)
    && (expectedCount !== 1 || Math.abs(endInset - (entry.length - lineWidth)) < 0.08)
    && (expectedPitch === null || pitches.every((pitch) => Math.abs(pitch - expectedPitch) < 0.08));
  const measurement = {
    name: entry.name,
    inputs: {
      length: entry.length,
      angle: entry.angle,
      lineWidth,
      boldWidth,
      hashSpacing,
      order: entry.order,
      display: entry.display,
      display2: entry.display2,
      doublePosition: entry.doublePosition,
    },
    stripeCount: stripes.length,
    expectedCount,
    stripeAxisLengths: [...new Set(stripes.map((shape) => rounded(shape.alongLength)))],
    stripeCrossLengths: [...new Set(stripes.map((shape) => rounded(shape.acrossLength)))],
    pitches: pitches.map(rounded),
    beginInset: beginInset === null ? null : rounded(beginInset),
    endInset: endInset === null ? null : rounded(endInset),
    matches,
  };
  measurements.push(measurement);
  if (!matches) failures.push(measurement);
}

const reportPath = path.join(outputRoot, "measurements.json");
await fs.writeFile(reportPath, `${JSON.stringify(measurements, null, 2)}\n`, "utf8");
if (failures.length > 0) {
  throw new Error(`Hash-bond rule mismatched ${failures.length} ChemDraw samples: ${failures.map((item) => item.name).join(", ")}`);
}
console.log(JSON.stringify({ reportPath, cases: measurements.length, failures: 0 }, null, 2));
