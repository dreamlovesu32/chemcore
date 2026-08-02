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
  ...[0.6, 1, 2].flatMap((lineWidth) =>
    [0.5, 1, 1.5, 2, 2.2, 3, 5].map((spacingAbs) => ({
      name: `absolute-width-${String(lineWidth).replace(".", "_")}-spacing-${String(spacingAbs).replace(".", "_")}`,
      length: 14.4,
      spacing: 30,
      spacingAbs,
      lineWidth,
      angle: 0,
    }))),
  ...[6, 14.4, 30].flatMap((length) =>
    [8, 18, 30].map((spacing) => ({
      name: `absolute-default-length-${String(length).replace(".", "_")}-spacing-${spacing}`,
      length,
      spacing,
      spacingAbs: 3,
      lineWidth: 0.6,
      angle: 37,
    }))),
  ...[30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330].flatMap(
    (branchAngle) => [{
      name: `endpoint-begin-angle-${branchAngle}`,
      length: 60,
      spacing: 12,
      lineWidth: 1,
      angle: 0,
      beginBranchAngle: branchAngle,
    }, {
      name: `endpoint-end-angle-${branchAngle}`,
      length: 60,
      spacing: 12,
      lineWidth: 1,
      angle: 0,
      endBranchAngle: branchAngle,
    }],
  ),
  {
    name: "endpoint-both-linear",
    length: 60,
    spacing: 12,
    lineWidth: 1,
    angle: 0,
    beginBranchAngle: 180,
    endBranchAngle: 0,
  },
  {
    name: "endpoint-both-deflection-60",
    length: 60,
    spacing: 12,
    lineWidth: 1,
    angle: 0,
    beginBranchAngle: 120,
    endBranchAngle: 60,
  },
  ...[0, 37, 90, 150].map((angle) => ({
    name: `endpoint-both-linear-direction-${angle}`,
    length: 30,
    spacing: 12,
    lineWidth: 0.6,
    angle,
    beginBranchAngle: angle + 180,
    endBranchAngle: angle,
  })),
  ...[
    { suffix: "atom-stereo-none", atomStereoNone: true },
    { suffix: "bond-stereo-none", bondStereoNone: true },
    { suffix: "atom-and-bond-stereo-none", atomStereoNone: true, bondStereoNone: true },
  ].map((variant) => ({
    name: `endpoint-both-linear-direction-150-${variant.suffix}`,
    length: 30,
    spacing: 12,
    lineWidth: 0.6,
    angle: 150,
    beginBranchAngle: 330,
    endBranchAngle: 150,
    ...variant,
  })),
  ...[0.6, 1, 2].flatMap((lineWidth) =>
    [14.4, 30, 60].map((length) => ({
      name: `endpoint-both-linear-matrix-width-${String(lineWidth).replace(".", "_")}-length-${String(length).replace(".", "_")}`,
      length,
      spacing: 12,
      lineWidth,
      angle: 150,
      beginBranchAngle: 330,
      endBranchAngle: 150,
      atomStereoNone: true,
      bondStereoNone: true,
    }))),
  ...["17.1.0.105", "18.2.0.48", "19.1.1.21", "20.1.1.125", "21.0.0.28", "22.2.0.3300"].map(
    (creationProgram) => ({
      name: `endpoint-both-linear-version-${creationProgram.replaceAll(".", "_")}`,
      length: 30,
      spacing: 12,
      lineWidth: 1,
      angle: 150,
      beginBranchAngle: 330,
      endBranchAngle: 150,
      atomStereoNone: true,
      bondStereoNone: true,
      creationProgram,
    }),
  ),
  ...[1, 2, 3, 4].map((branchDepth) => ({
    name: `endpoint-both-linear-chain-depth-${branchDepth}`,
    length: 30,
    spacing: 12,
    lineWidth: 1,
    angle: 150,
    beginBranchAngle: 330,
    endBranchAngle: 150,
    atomStereoNone: true,
    bondStereoNone: true,
    branchDepth,
  })),
  {
    name: "endpoint-both-linear-interleaved-z",
    length: 30,
    spacing: 12,
    lineWidth: 1,
    angle: 150,
    beginBranchAngle: 330,
    endBranchAngle: 150,
    atomStereoNone: true,
    bondStereoNone: true,
    creationProgram: "21.0.0.28",
    interleavedZ: true,
  },
  ...[0.005, 0.01, 0.02, 0.03, 0.04, 0.05, 0.1, 0.25, 0.5, 1].map(
    (deviation) => ({
      name: `endpoint-both-near-linear-deviation-${String(deviation).replace(".", "_")}`,
      length: 30,
      spacing: 12,
      lineWidth: 1,
      angle: 150,
      beginBranchAngle: 330 + deviation,
      endBranchAngle: 150 + deviation,
      atomStereoNone: true,
      bondStereoNone: true,
      creationProgram: "21.0.0.28",
      interleavedZ: true,
    }),
  ),
  ...[14.4, 30, 60].map((documentBondLength) => ({
    name: `endpoint-both-near-linear-document-bond-length-${String(documentBondLength).replace(".", "_")}`,
    length: 30,
    documentBondLength,
    spacing: 12,
    lineWidth: 1,
    angle: 150,
    beginBranchAngle: 330.04,
    endBranchAngle: 150.03,
    atomStereoNone: true,
    bondStereoNone: true,
    creationProgram: "21.0.0.28",
    interleavedZ: true,
  })),
  {
    name: "endpoint-both-near-linear-implicit-single-order",
    length: 30,
    documentBondLength: 30,
    spacing: 12,
    lineWidth: 1,
    angle: 150,
    beginBranchAngle: 330.04,
    endBranchAngle: 150.03,
    atomStereoNone: true,
    bondStereoNone: true,
    creationProgram: "21.0.0.28",
    interleavedZ: true,
    omitBranchOrder: true,
  },
  {
    name: "endpoint-both-near-linear-source-child-order",
    length: 30,
    documentBondLength: 30,
    spacing: 12,
    lineWidth: 1,
    angle: 150,
    beginBranchAngle: 330.04,
    endBranchAngle: 150.03,
    atomStereoNone: true,
    bondStereoNone: true,
    creationProgram: "21.0.0.28",
    interleavedZ: true,
    omitBranchOrder: true,
    nodesBeforeBonds: true,
  },
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
  const atomStereo = probe.atomStereoNone ? ' AS="N"' : "";
  const bondStereo = probe.bondStereoNone ? ' BS="N"' : "";
  const branchSource = ({
    originX,
    originY,
    angle,
    firstNodeId,
    firstBondId,
    rootNodeId,
    nodeZ,
    bondZ,
    reverseBond = false,
  }) => {
    const radians = angle * Math.PI / 180;
    const depth = probe.branchDepth ?? 1;
    const lines = [];
    let previousNodeId = rootNodeId;
    for (let index = 1; index <= depth; index += 1) {
      const nodeId = firstNodeId + index - 1;
      const bondId = firstBondId + index - 1;
      const branchX = originX + Math.cos(radians) * 30 * index;
      const branchY = originY + Math.sin(radians) * 30 * index;
      const nodeZAttribute = nodeZ == null ? "" : ` Z="${nodeZ + (index - 1) * 2}"`;
      const bondZAttribute = bondZ == null ? "" : ` Z="${bondZ + (index - 1) * 2}"`;
      const begin = reverseBond ? nodeId : previousNodeId;
      const end = reverseBond ? previousNodeId : nodeId;
      lines.push(`<n id="${nodeId}" p="${fixed(branchX)} ${fixed(branchY)}"${nodeZAttribute}${atomStereo}/>`);
      const order = probe.omitBranchOrder ? "" : ' Order="1"';
      lines.push(`<b id="${bondId}" B="${begin}" E="${end}"${order}${bondZAttribute}${bondStereo}/>`);
      previousNodeId = nodeId;
    }
    return `\n      ${lines.join("\n      ")}`;
  };
  const beginBranch = probe.beginBranchAngle == null
    ? ""
    : branchSource({
      originX: x0,
      originY: y0,
      angle: probe.beginBranchAngle,
      firstNodeId: 10,
      firstBondId: 100,
      rootNodeId: 3,
      nodeZ: probe.interleavedZ ? 1 : null,
      bondZ: probe.interleavedZ ? 4 : null,
      reverseBond: probe.interleavedZ,
    });
  const endBranch = probe.endBranchAngle == null
    ? ""
    : branchSource({
      originX: x1,
      originY: y1,
      angle: probe.endBranchAngle,
      firstNodeId: 20,
      firstBondId: 200,
      rootNodeId: 4,
      nodeZ: probe.interleavedZ ? 7 : null,
      bondZ: probe.interleavedZ ? 8 : null,
    });
  const fragmentBody = probe.nodesBeforeBonds
    ? (() => {
      const beginRadians = probe.beginBranchAngle * Math.PI / 180;
      const endRadians = probe.endBranchAngle * Math.PI / 180;
      const beginX = x0 + Math.cos(beginRadians) * 30;
      const beginY = y0 + Math.sin(beginRadians) * 30;
      const endX = x1 + Math.cos(endRadians) * 30;
      const endY = y1 + Math.sin(endRadians) * 30;
      return `<n id="10" p="${fixed(beginX)} ${fixed(beginY)}" Z="1"${atomStereo}/>
      <n id="3" p="${fixed(x0)} ${fixed(y0)}" Z="3"${atomStereo}/>
      <n id="4" p="${fixed(x1)} ${fixed(y1)}" Z="5"${atomStereo}/>
      <n id="20" p="${fixed(endX)} ${fixed(endY)}" Z="7"${atomStereo}/>
      <b id="100" B="10" E="3" Z="4"${bondStereo}/>
      <b id="5" B="3" E="4" Order="3" Z="6"${spacingAbs}${bondStereo}/>
      <b id="200" B="4" E="20" Z="8"${bondStereo}/>`;
    })()
    : `<n id="3" p="${fixed(x0)} ${fixed(y0)}"${probe.interleavedZ ? ' Z="3"' : ""}${atomStereo}/>
      <n id="4" p="${fixed(x1)} ${fixed(y1)}"${probe.interleavedZ ? ' Z="5"' : ""}${atomStereo}/>
      <b id="5" B="3" E="4" Order="3"${probe.interleavedZ ? ' Z="6"' : ""}${spacingAbs}${bondStereo}/>
      ${beginBranch}
      ${endBranch}`;
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemDraw ${probe.creationProgram ?? "22.2.0.3300"}" BoundingBox="-100 -100 250 250"
 FractionalWidths="yes" InterpretChemically="yes"
 ShowTerminalCarbonLabels="no" ShowNonTerminalCarbonLabels="no"
 LabelFont="3" LabelSize="10" LabelFace="96"
 CaptionFont="3" CaptionSize="10"
 LineWidth="${fixed(probe.lineWidth)}" BoldWidth="4"
 BondLength="${fixed(probe.documentBondLength ?? 14.4)}" BondSpacing="${fixed(probe.spacing)}"
 HashSpacing="2.5" MarginWidth="2">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="-100 -100 250 250">
    <fragment id="2">
      ${fragmentBody}
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
    transform: { a, b, c, d: dScale, e, f },
  };
}

function measureSvg(svg, probe) {
  const radians = probe.angle * Math.PI / 180;
  const axis = { x: Math.cos(radians), y: Math.sin(radians) };
  const normal = { x: -axis.y, y: axis.x };
  const allPaths = (svg.match(/<path\b[^>]*>/gi) ?? [])
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
    });
  const sourceMidpoint = {
    x: (50 + 50 + Math.cos(radians) * probe.length) * 10,
    y: (50 + 50 + Math.sin(radians) * probe.length) * 10,
  };
  const paths = [...allPaths]
    .map((entry) => {
      const { a, b, c, d, e, f } = entry.transform;
      const expectedMidpoint = {
        x: a * sourceMidpoint.x + c * sourceMidpoint.y + e,
        y: b * sourceMidpoint.x + d * sourceMidpoint.y + f,
      };
      const expectedAxisMidpoint = expectedMidpoint.x * axis.x + expectedMidpoint.y * axis.y;
      const expectedNormalCenter = expectedMidpoint.x * normal.x + expectedMidpoint.y * normal.y;
      const axisMidpoint = (entry.axisMin + entry.axisMax) * 0.5;
      return {
        ...entry,
        targetScore: Math.abs(axisMidpoint - expectedAxisMidpoint)
          + Math.abs(entry.normalCenter - expectedNormalCenter),
      };
    })
    .sort((left, right) => left.targetScore - right.targetScore)
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
  // ChemDraw 22.2 accepts BondSpacingAbs in the file but does not use it for
  // triple-bond rendering. Its presence also suppresses the authored document
  // BondSpacing, so triple bonds use ChemDraw's 15% default plus the ordinary
  // 2.5 * LineWidth floor.
  if (probe.spacingAbs != null) {
    return Math.max(probe.length * 0.15, probe.lineWidth * 2.5);
  }
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
    beginBranchAngle: probe.beginBranchAngle ?? null,
    endBranchAngle: probe.endBranchAngle ?? null,
    expectedCenterDistance: expected,
    measuredCenterDistance: measured.centerDistance,
    adjacentCenterDistances: measured.adjacentCenterDistances,
    asymmetry: measured.asymmetry,
    lanes: measured.lanes,
    laneSpanDeficits: measured.laneSpanDeficits,
    outerSpanErrors: probe.beginBranchAngle == null && probe.endBranchAngle == null
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
  if (entry.beginBranchAngle != null || entry.endBranchAngle != null) {
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
