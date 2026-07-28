import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const refresh = process.argv.includes("--refresh");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(
  root,
  outputArg ?? "tmp/chemdraw-multiglyph-axis-contact-probe",
);
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");

const fonts = [
  { name: "Arial", id: 3, slug: "arial" },
  { name: "Times New Roman", id: 4, slug: "times" },
  { name: "Calibri", id: 5, slug: "calibri" },
];

const families = [];
for (const font of fonts) {
  for (const size of [8, 10, 14]) {
    for (const marginWidth of [0, 1.6, 3]) {
      families.push({
        name: `bottom-${font.slug}-${size}-margin-${String(marginWidth).replace(".", "_")}`,
        font,
        size,
        marginWidth,
        lineWidth: 0.6,
        direction: "bottom",
        decoratedRuns: [
          { text: "CH", face: 0 },
          { text: "3", face: 32 },
        ],
      });
    }
  }
}
for (const lineWidth of [1.5]) {
  families.push({
    name: `bottom-arial-10-line-${String(lineWidth).replace(".", "_")}`,
    font: fonts[0],
    size: 10,
    marginWidth: 1.6,
    lineWidth,
    direction: "bottom",
    decoratedRuns: [
      { text: "CH", face: 0 },
      { text: "3", face: 32 },
    ],
  });
}
for (const direction of ["top", "left", "right"]) {
  families.push({
    name: `${direction}-arial-10`,
    font: fonts[0],
    size: 10,
    marginWidth: 1.6,
    lineWidth: 0.6,
    direction,
    decoratedRuns: direction === "top"
      ? [{ text: "CH", face: 0 }, { text: "+", face: 64 }]
      : [{ text: "CH", face: 0 }, { text: "3", face: 32 }],
  });
}
const probes = families.flatMap((family) => [
  {
    ...family,
    name: `${family.name}-base`,
    pair: family.name,
    kind: "base",
    runs: [{ text: "C", face: 0 }],
  },
  {
    ...family,
    name: `${family.name}-decorated`,
    pair: family.name,
    kind: "decorated",
    runs: family.decoratedRuns,
  },
]);

function fixed(value) {
  return Number(value).toFixed(4).replace(/\.?0+$/, "");
}

function xmlEscape(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function sourceFor(probe) {
  const target = { x: 60, y: 50 };
  const length = 24;
  const offsets = {
    bottom: [0, length],
    top: [0, -length],
    left: [-length, 0],
    right: [length, 0],
  };
  const [dx, dy] = offsets[probe.direction];
  const source = { x: target.x + dx, y: target.y + dy };
  const textX = target.x - probe.size * 0.361;
  const baseline = target.y + probe.size * 0.39;
  const box = [
    textX,
    baseline - probe.size * 0.846,
    textX + probe.size * 3.2,
    baseline + probe.size * 0.62,
  ];
  const runs = probe.runs
    .map((run) => `<s font="${probe.font.id}" size="${probe.size}" face="${run.face}" color="0">${xmlEscape(run.text)}</s>`)
    .join("");
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemDraw 23.1.2.7" BoundingBox="0 0 120 120"
 FractionalWidths="yes" InterpretChemically="yes"
 ShowTerminalCarbonLabels="no" ShowNonTerminalCarbonLabels="no"
 LabelFont="${probe.font.id}" LabelSize="${probe.size}" LabelFace="0"
 CaptionFont="${probe.font.id}" CaptionSize="${probe.size}"
 LineWidth="${fixed(probe.lineWidth)}" BoldWidth="2"
 BondLength="14.4" BondSpacing="18"
 HashSpacing="2.5" MarginWidth="${fixed(probe.marginWidth)}">
  <fonttable>
    ${fonts.map((font) => `<font id="${font.id}" charset="iso-8859-1" name="${font.name}"/>`).join("")}
  </fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="3" p="${fixed(source.x)} ${fixed(source.y)}" AS="N"/>
      <n id="4" p="${fixed(target.x)} ${fixed(target.y)}" AS="N" NumHydrogens="0">
        <t p="${fixed(textX)} ${fixed(baseline)}"
           BoundingBox="${box.map(fixed).join(" ")}"
           LabelJustification="Left" LabelAlignment="Left">${runs}</t>
      </n>
      <b id="5" B="3" E="4"/>
    </fragment>
  </page>
</CDXML>
`;
}

function attribute(tag, name) {
  return tag.match(new RegExp(`\\b${name}="([^"]*)"`, "i"))?.[1] ?? null;
}

function transformedPath(tag) {
  const data = attribute(tag, "d");
  const transform = attribute(tag, "transform");
  if (!data || !transform) return null;
  const matrix = transform
    .match(/matrix\(([^)]*)\)/i)?.[1]
    ?.trim()
    .split(/[\s,]+/)
    .map(Number);
  if (!matrix || matrix.length !== 6 || matrix.some((value) => !Number.isFinite(value))) {
    return null;
  }
  const numbers = data.match(/[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g)?.map(Number) ?? [];
  if (numbers.length < 6 || numbers.length % 2 !== 0) return null;
  const [a, b, c, d, e, f] = matrix;
  const points = [];
  for (let index = 0; index < numbers.length; index += 2) {
    const x = numbers[index];
    const y = numbers[index + 1];
    points.push({ x: a * x + c * y + e, y: b * x + d * y + f });
  }
  return { points, svgUnitsPerPoint: Math.hypot(a, b) * 20 };
}

function measureVisibleLength(svg, probe) {
  const axes = {
    bottom: [0, 1],
    top: [0, -1],
    left: [-1, 0],
    right: [1, 0],
  };
  const [axisX, axisY] = axes[probe.direction];
  const paths = (svg.match(/<path\b[^>]*>/gi) ?? [])
    .filter((tag) => /\bfill="#000000"/i.test(tag))
    .map(transformedPath)
    .filter(Boolean)
    .map((entry) => {
      const projections = entry.points.map((point) => point.x * axisX + point.y * axisY);
      return {
        ...entry,
        span: Math.max(...projections) - Math.min(...projections),
      };
    })
    .sort((left, right) => right.span - left.span);
  const bond = paths[0];
  if (!bond) throw new Error(`${probe.name}: no filled bond path`);
  return bond.span / bond.svgUnitsPerPoint;
}

await fs.mkdir(sourceDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
for (const probe of probes) {
  probe.input = path.join(sourceDir, `${probe.name}.cdxml`);
  probe.svg = path.join(oracleDir, `${probe.name}.chemdraw.svg`);
  await fs.writeFile(probe.input, sourceFor(probe), "utf8");
}

const missing = [];
for (const probe of probes) {
  if (refresh) {
    missing.push(probe);
    continue;
  }
  try {
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
  measurements.push({
    pair: probe.pair,
    kind: probe.kind,
    direction: probe.direction,
    font: probe.font.name,
    size: probe.size,
    marginWidth: probe.marginWidth,
    lineWidth: probe.lineWidth,
    visibleLength: measureVisibleLength(svg, probe),
  });
}

const comparisons = families.map((family) => {
  const base = measurements.find((entry) =>
    entry.pair === family.name && entry.kind === "base");
  const decorated = measurements.find((entry) =>
    entry.pair === family.name && entry.kind === "decorated");
  return {
    name: family.name,
    direction: family.direction,
    font: family.font.name,
    size: family.size,
    marginWidth: family.marginWidth,
    lineWidth: family.lineWidth,
    baseVisibleLength: base.visibleLength,
    decoratedVisibleLength: decorated.visibleLength,
    delta: decorated.visibleLength - base.visibleLength,
  };
});

await fs.writeFile(
  path.join(outDir, "measurements.json"),
  `${JSON.stringify({ measurements, comparisons }, null, 2)}\n`,
  "utf8",
);

// ChemDraw's SVG bond coordinates are quantized in 0.05-0.10 pt steps for
// these templates. The paired formula labels must agree within one such step;
// a whole-label vertical ymin/ymax rule misses by several points.
const mismatches = comparisons.filter((entry) => Math.abs(entry.delta) > 0.12);
if (mismatches.length > 0) {
  console.error(JSON.stringify(mismatches, null, 2));
  throw new Error(
    `Multi-glyph axial locality mismatched ${mismatches.length} ChemDraw pairs`,
  );
}

console.log(JSON.stringify({
  count: comparisons.length,
  maximumAbsoluteDelta: Math.max(...comparisons.map((entry) => Math.abs(entry.delta))),
  output: path.join(outDir, "measurements.json"),
}));
