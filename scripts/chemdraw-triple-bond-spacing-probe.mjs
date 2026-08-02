import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const refresh = process.argv.includes("--refresh");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(
  root,
  outputArg ?? "tmp/chemdraw-triple-bond-spacing-probe",
);
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");

const probes = [
  { name: "length-6", length: 6, spacing: 18, lineWidth: 0.6, angle: 0 },
  { name: "length-14_4", length: 14.4, spacing: 18, lineWidth: 0.6, angle: 0 },
  { name: "length-30", length: 30, spacing: 18, lineWidth: 0.6, angle: 0 },
  { name: "spacing-8", length: 14.4, spacing: 8, lineWidth: 0.6, angle: 0 },
  { name: "spacing-30", length: 14.4, spacing: 30, lineWidth: 0.6, angle: 0 },
  { name: "floor-medium", length: 6, spacing: 8, lineWidth: 1, angle: 0 },
  { name: "floor-wide", length: 6, spacing: 8, lineWidth: 2, angle: 0 },
  { name: "floor-long-wide", length: 14.4, spacing: 8, lineWidth: 2, angle: 0 },
  { name: "direction-37", length: 14.4, spacing: 18, lineWidth: 0.6, angle: 37 },
  { name: "direction-90", length: 14.4, spacing: 18, lineWidth: 0.6, angle: 90 },
  {
    name: "absolute-2_2",
    length: 14.4,
    spacing: 8,
    spacingAbs: 2.2,
    lineWidth: 0.6,
    angle: 0,
  },
  {
    name: "absolute-floor",
    length: 14.4,
    spacing: 30,
    spacingAbs: 0.5,
    lineWidth: 2,
    angle: 0,
  },
  ...[30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330].map(
    (branchAngle) => ({
      name: `endpoint-angle-${branchAngle}`,
      length: 60,
      spacing: 12,
      lineWidth: 1,
      angle: 0,
      branchAngle,
    }),
  ),
];

function fixed(value) {
  return Number(value).toFixed(4).replace(/\.?0+$/, "");
}

function sourceFor(probe) {
  const x0 = 50;
  const y0 = 50;
  const radians = probe.angle * Math.PI / 180;
  const x1 = x0 + Math.cos(radians) * probe.length;
  const y1 = y0 + Math.sin(radians) * probe.length;
  const spacingAbs = probe.spacingAbs == null
    ? ""
    : ` BondSpacingAbs="${fixed(probe.spacingAbs)}"`;
  const branch = probe.branchAngle == null
    ? ""
    : (() => {
      const branchRadians = probe.branchAngle * Math.PI / 180;
      const branchX = x0 + Math.cos(branchRadians) * 30;
      const branchY = y0 + Math.sin(branchRadians) * 30;
      return `
      <n id="6" p="${fixed(branchX)} ${fixed(branchY)}"/>
      <b id="7" B="3" E="6" Order="1"/>`;
    })();
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemDraw 22.2.0.3300" BoundingBox="0 0 110 110"
 FractionalWidths="yes" InterpretChemically="yes"
 ShowTerminalCarbonLabels="no" ShowNonTerminalCarbonLabels="no"
 LabelFont="3" LabelSize="10" LabelFace="96"
 CaptionFont="3" CaptionSize="10"
 LineWidth="${fixed(probe.lineWidth)}" BoldWidth="4"
 BondLength="14.4" BondSpacing="${fixed(probe.spacing)}"
 HashSpacing="2.5" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 110 110">
    <fragment id="2">
      <n id="3" p="${fixed(x0)} ${fixed(y0)}"/>
      <n id="4" p="${fixed(x1)} ${fixed(y1)}"/>
      <b id="5" B="3" E="4" Order="3"${spacingAbs}/>
      ${branch}
    </fragment>
  </page>
</CDXML>
`;
}

function attribute(tag, name) {
  return tag.match(new RegExp(`\\b${name}="([^"]*)"`, "i"))?.[1] ?? null;
}

function transformedPathPoints(tag) {
  const d = attribute(tag, "d");
  const transform = attribute(tag, "transform");
  if (!d || !transform) return null;
  const matrix = transform
    .match(/matrix\(([^)]*)\)/i)?.[1]
    ?.trim()
    .split(/[\s,]+/)
    .map(Number);
  if (!matrix || matrix.length !== 6 || matrix.some((value) => !Number.isFinite(value))) {
    return null;
  }
  const numbers = d.match(/[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g)?.map(Number) ?? [];
  if (numbers.length < 6 || numbers.length % 2 !== 0) return null;
  const [a, b, c, dScale, e, f] = matrix;
  const points = [];
  for (let index = 0; index < numbers.length; index += 2) {
    const x = numbers[index];
    const y = numbers[index + 1];
    points.push({
      x: a * x + c * y + e,
      y: b * x + dScale * y + f,
    });
  }
  return {
    points,
    svgUnitsPerPoint: Math.hypot(a, b) * 20,
  };
}

function measureSvg(svg, probe) {
  const radians = probe.angle * Math.PI / 180;
  const axis = { x: Math.cos(radians), y: Math.sin(radians) };
  const normal = { x: -axis.y, y: axis.x };
  const paths = (svg.match(/<path\b[^>]*>/gi) ?? [])
    .filter((tag) => /\bfill="#000000"/i.test(tag))
    .map(transformedPathPoints)
    .filter(Boolean)
    .map((entry) => {
      const axisValues = entry.points.map((point) => point.x * axis.x + point.y * axis.y);
      const normalValues = entry.points.map((point) => point.x * normal.x + point.y * normal.y);
      return {
        ...entry,
        axisMin: Math.min(...axisValues),
        axisMax: Math.max(...axisValues),
        axisSpan: Math.max(...axisValues) - Math.min(...axisValues),
        normalCenter: (
          Math.max(...normalValues) + Math.min(...normalValues)
        ) * 0.5,
      };
    })
    .sort((left, right) => right.axisSpan - left.axisSpan)
    .slice(0, 3);
  if (paths.length !== 3) {
    throw new Error(`${probe.name}: expected three bond paths, found ${paths.length}`);
  }
  const centers = paths.map((entry) => entry.normalCenter).sort((left, right) => left - right);
  const lanes = paths
    .map((entry) => ({
      normalCenter: entry.normalCenter,
      axisMin: entry.axisMin,
      axisMax: entry.axisMax,
      axisSpan: entry.axisSpan,
    }))
    .sort((left, right) => left.normalCenter - right.normalCenter);
  const maximumAxisSpan = Math.max(...lanes.map((lane) => lane.axisSpan));
  const svgUnitsPerPoint = paths.reduce(
    (sum, entry) => sum + entry.svgUnitsPerPoint,
    0,
  ) / paths.length;
  const adjacent = [
    (centers[1] - centers[0]) / svgUnitsPerPoint,
    (centers[2] - centers[1]) / svgUnitsPerPoint,
  ];
  return {
    centers,
    adjacentCenterDistances: adjacent,
    centerDistance: (adjacent[0] + adjacent[1]) * 0.5,
    asymmetry: Math.abs(adjacent[0] - adjacent[1]),
    lanes,
    laneSpanDeficits: lanes.map((lane) => maximumAxisSpan - lane.axisSpan),
    svgUnitsPerPoint,
  };
}

function expectedCenterDistance(probe) {
  if (probe.spacingAbs != null) return Math.max(probe.spacingAbs, probe.lineWidth * 2.5);
  return Math.max(probe.length * probe.spacing / 100, probe.lineWidth * 2.5);
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
  const measured = measureSvg(svg, probe);
  const expected = expectedCenterDistance(probe);
  measurements.push({
    name: probe.name,
    length: probe.length,
    spacing: probe.spacing,
    spacingAbs: probe.spacingAbs ?? null,
    lineWidth: probe.lineWidth,
    angle: probe.angle,
    branchAngle: probe.branchAngle ?? null,
    expectedCenterDistance: expected,
    measuredCenterDistance: measured.centerDistance,
    adjacentCenterDistances: measured.adjacentCenterDistances,
    asymmetry: measured.asymmetry,
    lanes: measured.lanes,
    laneSpanDeficits: measured.laneSpanDeficits,
    outerSpanErrors: probe.branchAngle == null
      ? null
      : [measured.lanes[0], measured.lanes[2]].map(
        (lane) => lane.axisSpan - probe.length * measured.svgUnitsPerPoint,
      ),
    svgUnitsPerPoint: measured.svgUnitsPerPoint,
    delta: measured.centerDistance - expected,
  });
}

await fs.writeFile(
  path.join(outDir, "measurements.json"),
  `${JSON.stringify(measurements, null, 2)}\n`,
  "utf8",
);

const mismatches = measurements.filter((entry) => {
  if (entry.branchAngle != null) {
    return entry.outerSpanErrors.some((error) => Math.abs(error) > 0.015);
  }
  const spacingTolerance = entry.spacingAbs == null ? 0.015 : 0.05;
  return Math.abs(entry.delta) > spacingTolerance || entry.asymmetry > 0.015;
});
if (mismatches.length > 0) {
  console.error(JSON.stringify(mismatches, null, 2));
  throw new Error(
    `Triple-bond spacing rule mismatched ${mismatches.length} ChemDraw samples`,
  );
}

console.log(JSON.stringify({
  count: measurements.length,
  maximumAbsoluteDelta: Math.max(...measurements.map((entry) => Math.abs(entry.delta))),
  output: path.join(outDir, "measurements.json"),
}));
