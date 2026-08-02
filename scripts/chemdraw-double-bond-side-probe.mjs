import fs from "node:fs/promises";
import path from "node:path";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const CHEMDRAW_DOUBLE_BOND_MIN_ATTACHMENT_SIDE_SINE = 0.2146;

const root = path.resolve(import.meta.dirname, "..");
const refresh = process.argv.includes("--refresh");
const outputArg = process.argv.slice(2).find((value) => !value.startsWith("--"));
const outDir = path.resolve(
  root,
  outputArg ?? "tmp/chemdraw-double-bond-side-probe",
);
const sourceDir = path.join(outDir, "cdxml");
const oracleDir = path.join(outDir, "chemdraw");

const perturbations = [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 3, 10];
const probes = [
  { name: "exact-tie" },
  ...perturbations.flatMap((delta) => [
    { name: `lower-plus-${delta}`, lowerDelta: delta },
    { name: `upper-plus-${delta}`, upperDelta: delta },
  ]),
  { name: "exact-tie-bs-e", absoluteStereo: "E" },
  { name: "exact-tie-bs-z", absoluteStereo: "Z" },
  { name: "exact-tie-bco-forward", circularOrdering: "21 22 24 23" },
  { name: "exact-tie-bco-reverse", circularOrdering: "22 21 23 24" },
  {
    name: "exact-tie-bs-z-bco-forward",
    absoluteStereo: "Z",
    circularOrdering: "21 22 24 23",
  },
  {
    name: "exact-tie-bs-e-bco-reverse",
    absoluteStereo: "E",
    circularOrdering: "22 21 23 24",
  },
  { name: "exact-tie-reverse-axis", reverseAxis: true },
  {
    name: "exact-tie-reverse-axis-bco-forward",
    reverseAxis: true,
    circularOrdering: "21 22 24 23",
  },
  { name: "exact-tie-hetero", hetero: true },
  {
    name: "exact-tie-hetero-bs-z-bco-forward",
    hetero: true,
    absoluteStereo: "Z",
    circularOrdering: "21 22 24 23",
  },
  { name: "three-upper-one-lower", omitLowerEnd: true },
  { name: "one-upper-three-lower", omitUpperEnd: true },
  { name: "angle-upper-1-lower-80", upperAngle: 1, lowerAngle: 80 },
  { name: "angle-upper-80-lower-1", upperAngle: 80, lowerAngle: 1 },
  {
    name: "majority-upper-shallow",
    upperAngle: 1,
    lowerAngle: 80,
    omitLowerEnd: true,
  },
  {
    name: "majority-lower-shallow",
    upperAngle: 80,
    lowerAngle: 1,
    omitUpperEnd: true,
  },
  { name: "rotation-30", rotation: 30 },
  { name: "rotation-90", rotation: 90 },
  { name: "rotation-137", rotation: 137 },
  { name: "rotation-30-majority-upper", rotation: 30, omitLowerEnd: true },
  { name: "rotation-90-majority-lower", rotation: 90, omitUpperEnd: true },
  ...[31, 35, 40, 45, 50, 60, 70, 80].flatMap((angle) => [
    { name: `both-lower-angle-${angle}`, lowerAngle: angle },
    { name: `begin-lower-angle-${angle}`, lowerBeginAngle: angle },
    { name: `end-lower-angle-${angle}`, lowerEndAngle: angle },
  ]),
  {
    name: "cross-upper-begin-80-lower-end-80",
    upperBeginAngle: 80,
    lowerEndAngle: 80,
  },
  {
    name: "cross-lower-begin-80-upper-end-80",
    lowerBeginAngle: 80,
    upperEndAngle: 80,
  },
  {
    name: "upper-ends-89-10-lower-35-35",
    upperBeginAngle: 89,
    upperEndAngle: 10,
    lowerAngle: 35,
  },
  {
    name: "upper-ends-80-1-lower-30-30",
    upperBeginAngle: 80,
    upperEndAngle: 1,
    lowerAngle: 30,
  },
  ...[0.1, 1, 5, 8, 10, 12, 14, 15, 16, 17, 18, 20, 22, 25, 29, 30].flatMap((angle) => [
    { name: `both-upper-angle-${angle}`, upperAngle: angle, lowerAngle: 30 },
    { name: `begin-upper-angle-${angle}`, upperBeginAngle: angle, lowerAngle: 30 },
    { name: `end-upper-angle-${angle}`, upperEndAngle: angle, lowerAngle: 30 },
  ]),
  {
    name: "no-attachments",
    omitBeginUpper: true,
    omitBeginLower: true,
    omitEndUpper: true,
    omitEndLower: true,
  },
  {
    name: "cis-upper-only",
    omitBeginLower: true,
    omitEndLower: true,
  },
  {
    name: "cis-lower-only",
    omitBeginUpper: true,
    omitEndUpper: true,
  },
  ...[5, 10, 15, 20, 30, 45, 60, 75, 89, 90, 91, 105, 120]
    .flatMap((angle) => [
      {
        name: `cis-upper-angle-${angle}`,
        upperAngle: angle,
        expectedSecondaryInsetEndpoints: 2,
        omitBeginLower: true,
        omitEndLower: true,
      },
      {
        name: `begin-upper-only-angle-${angle}`,
        upperAngle: angle,
        expectedSecondaryInsetEndpoints: 1,
        omitBeginLower: true,
        omitEndUpper: true,
        omitEndLower: true,
      },
    ]),
  ...[6, 12, 20].flatMap((bondSpacing) =>
    [30, 60, 90, 120].map((angle) => ({
      name: `cis-upper-spacing-${bondSpacing}-angle-${angle}`,
      bondSpacing,
      upperAngle: angle,
      expectedSecondaryInsetEndpoints: 2,
      omitBeginLower: true,
      omitEndLower: true,
    }))),
  ...[30, 60, 90].flatMap((axisLength) =>
    [30, 60, 90, 120].map((angle) => ({
      name: `cis-upper-length-${axisLength}-angle-${angle}`,
      axisLength,
      upperAngle: angle,
      expectedSecondaryInsetEndpoints: 2,
      omitBeginLower: true,
      omitEndLower: true,
    }))),
  ...[0.4, 0.6, 1, 2].map((lineWidth) => ({
    name: `cis-upper-line-${lineWidth}-angle-90`,
    lineWidth,
    upperAngle: 90,
    expectedSecondaryInsetEndpoints: 2,
    omitBeginLower: true,
    omitEndLower: true,
  })),
  ...[2, 4, 8].map((bondSpacingAbs) => ({
    name: `cis-upper-absolute-${bondSpacingAbs}-angle-90`,
    bondSpacing: 1,
    bondSpacingAbs,
    upperAngle: 90,
    expectedSecondaryInsetEndpoints: 2,
    omitBeginLower: true,
    omitEndLower: true,
  })),
  {
    name: "trans-begin-upper-end-lower",
    omitBeginLower: true,
    omitEndUpper: true,
  },
  {
    name: "trans-begin-lower-end-upper",
    omitBeginUpper: true,
    omitEndLower: true,
  },
  {
    name: "begin-upper-only",
    omitBeginLower: true,
    omitEndUpper: true,
    omitEndLower: true,
  },
  {
    name: "begin-geminal-only",
    omitEndUpper: true,
    omitEndLower: true,
  },
  { name: "both-sides-acute-10", upperAngle: 10, lowerAngle: 10 },
  {
    name: "begin-lower-acute-only",
    lowerBeginAngle: 10,
    omitBeginUpper: true,
    omitEndUpper: true,
    omitEndLower: true,
  },
  {
    name: "begin-upper-acute-only",
    upperBeginAngle: 10,
    omitBeginLower: true,
    omitEndUpper: true,
    omitEndLower: true,
  },
  { name: "spacing-6-angle-6", bondSpacing: 6, upperAngle: 6 },
  { name: "spacing-6-angle-7", bondSpacing: 6, upperAngle: 7 },
  { name: "spacing-20-angle-21", bondSpacing: 20, upperAngle: 21 },
  { name: "spacing-20-angle-22", bondSpacing: 20, upperAngle: 22 },
  { name: "length-30-spacing-12-angle-13", axisLength: 30, upperAngle: 13 },
  { name: "length-30-spacing-12-angle-14", axisLength: 30, upperAngle: 14 },
  { name: "length-90-spacing-12-angle-13", axisLength: 90, upperAngle: 13 },
  { name: "length-90-spacing-12-angle-14", axisLength: 90, upperAngle: 14 },
  {
    name: "line-floor-angle-4",
    bondSpacing: 1,
    lineWidth: 1,
    upperAngle: 4,
  },
  {
    name: "line-floor-angle-5",
    bondSpacing: 1,
    lineWidth: 1,
    upperAngle: 5,
  },
  {
    name: "absolute-spacing-4-angle-7",
    bondSpacing: 1,
    bondSpacingAbs: 4,
    upperAngle: 7,
  },
  {
    name: "absolute-spacing-4-angle-8",
    bondSpacing: 1,
    bondSpacingAbs: 4,
    upperAngle: 8,
  },
  ...[12, 12.5, 12.9, 13, 13.1, 13.5, 13.9, 14].flatMap((angle) => [
    { name: `boundary-default-${angle}`, upperAngle: angle },
    { name: `boundary-spacing-6-${angle}`, bondSpacing: 6, upperAngle: angle },
    { name: `boundary-spacing-20-${angle}`, bondSpacing: 20, upperAngle: angle },
    { name: `boundary-length-30-${angle}`, axisLength: 30, upperAngle: angle },
    {
      name: `boundary-absolute-4-${angle}`,
      bondSpacing: 1,
      bondSpacingAbs: 4,
      upperAngle: angle,
    },
  ]),
  {
    name: "begin-lower-only",
    omitBeginUpper: true,
    omitEndUpper: true,
    omitEndLower: true,
  },
  ...[12.1, 12.2, 12.25, 12.3, 12.4].map((angle) => ({
    name: `boundary-fine-${angle}`,
    upperAngle: angle,
  })),
  { name: "boundary-rotation-30-angle-12", rotation: 30, upperAngle: 12 },
  { name: "boundary-rotation-30-angle-12.5", rotation: 30, upperAngle: 12.5 },
  { name: "boundary-rotation-90-angle-12", rotation: 90, upperAngle: 12 },
  { name: "boundary-rotation-90-angle-12.5", rotation: 90, upperAngle: 12.5 },
  ...[12.31, 12.32, 12.33, 12.34, 12.35, 12.36, 12.37, 12.38, 12.39].map((angle) => ({
    name: `boundary-ultrafine-${angle}`,
    upperAngle: angle,
  })),
  ...[12.395, 12.396, 12.397, 12.398, 12.399, 12.4].map((angle) => ({
    name: `boundary-terminal-${angle}`,
    upperAngle: angle,
  })),
];

function fixed(value) {
  return Number(value).toFixed(4).replace(/\.?0+$/, "");
}

function axisEndpoints(probe) {
  const axisLength = probe.axisLength ?? 60;
  const rotation = (probe.rotation ?? 0) * Math.PI / 180;
  const axis = { x: Math.cos(rotation), y: Math.sin(rotation) };
  return {
    axis,
    begin: { x: 130 - axis.x * axisLength * 0.5, y: 100 - axis.y * axisLength * 0.5 },
    end: { x: 130 + axis.x * axisLength * 0.5, y: 100 + axis.y * axisLength * 0.5 },
  };
}

function probeGeometry(probe) {
  const { axis, begin, end } = axisEndpoints(probe);
  const normal = { x: -axis.y, y: axis.x };
  const branchPoint = (center, alongSign, sideSign, angleDegrees, normalDelta = 0) => {
    const radians = angleDegrees * Math.PI / 180;
    return {
      x: center.x
        + axis.x * alongSign * 30 * Math.cos(radians)
        + normal.x * sideSign * (30 * Math.sin(radians) + normalDelta),
      y: center.y
        + axis.y * alongSign * 30 * Math.cos(radians)
        + normal.y * sideSign * (30 * Math.sin(radians) + normalDelta),
    };
  };
  const upperAngle = probe.upperAngle ?? 30;
  const lowerAngle = probe.lowerAngle ?? 30;
  const upperBeginAngle = probe.upperBeginAngle ?? upperAngle;
  const upperEndAngle = probe.upperEndAngle ?? upperAngle;
  const lowerBeginAngle = probe.lowerBeginAngle ?? lowerAngle;
  const lowerEndAngle = probe.lowerEndAngle ?? lowerAngle;
  const points = {
    beginUpper: branchPoint(begin, -1, -1, upperBeginAngle, probe.upperDelta ?? 0),
    beginLower: branchPoint(begin, -1, 1, lowerBeginAngle, probe.lowerDelta ?? 0),
    endUpper: branchPoint(end, 1, -1, upperEndAngle),
    endLower: branchPoint(end, 1, 1, lowerEndAngle),
  };
  return { axis, begin, end, points };
}

function sourceFor(probe) {
  const { begin, end, points } = probeGeometry(probe);
  const elements = probe.hetero
    ? { beginUpper: 53, beginLower: 9, endUpper: 35, endLower: 17 }
    : {};
  const node = (id, point, key) => {
    const element = elements[key] ? ` Element="${elements[key]}" NumHydrogens="0"` : "";
    return `<n id="${id}" p="${fixed(point.x)} ${fixed(point.y)}"${element}/>`;
  };
  const stereo = probe.absoluteStereo ? ` BS="${probe.absoluteStereo}"` : "";
  const ordering = probe.circularOrdering
    ? ` BondCircularOrdering="${probe.circularOrdering}"`
    : "";
  const axisBegin = probe.reverseAxis ? 12 : 11;
  const axisEnd = probe.reverseAxis ? 11 : 12;
  const spacingAbs = probe.bondSpacingAbs == null
    ? ""
    : ` BondSpacingAbs="${fixed(probe.bondSpacingAbs)}"`;
  const omitBeginUpper = probe.omitBeginUpper ?? false;
  const omitBeginLower = probe.omitBeginLower ?? false;
  const omitEndUpper = probe.omitEndUpper ?? probe.omitUpperEnd ?? false;
  const omitEndLower = probe.omitEndLower ?? probe.omitLowerEnd ?? false;
  const beginUpperBond = omitBeginUpper ? "" : '<b id="21" B="11" E="13"/>';
  const beginLowerBond = omitBeginLower ? "" : '<b id="22" B="11" E="14"/>';
  const endUpperBond = omitEndUpper ? "" : '<b id="23" B="12" E="15"/>';
  const endLowerBond = omitEndLower ? "" : '<b id="24" B="12" E="16"/>';
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd">
<CDXML BoundingBox="0 0 260 200" BondLength="30" BondSpacing="${fixed(probe.bondSpacing ?? 12)}"
 LineWidth="${fixed(probe.lineWidth ?? 1)}" BoldWidth="4" MarginWidth="2"
 LabelFont="3" LabelSize="10" LabelFace="96">
 <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
 <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
 <page id="1" BoundingBox="0 0 260 200">
  <fragment id="2">
   <n id="11" p="${fixed(begin.x)} ${fixed(begin.y)}"/>
   <n id="12" p="${fixed(end.x)} ${fixed(end.y)}"/>
   ${node(13, points.beginUpper, "beginUpper")}
   ${node(14, points.beginLower, "beginLower")}
   ${node(15, points.endUpper, "endUpper")}
   ${node(16, points.endLower, "endLower")}
   <b id="20" B="${axisBegin}" E="${axisEnd}" Order="2"${stereo}${ordering}${spacingAbs}/>
   ${beginUpperBond}
   ${beginLowerBond}
   ${endUpperBond}
   ${endLowerBond}
  </fragment>
 </page>
</CDXML>
`;
}

function expectedPlacement(probe) {
  const { begin, end, points } = probeGeometry(probe);
  const axisBegin = probe.reverseAxis ? end : begin;
  const axisEnd = probe.reverseAxis ? begin : end;
  const dx = axisEnd.x - axisBegin.x;
  const dy = axisEnd.y - axisBegin.y;
  const axisLength = Math.hypot(dx, dy);
  const normal = { x: -dy / axisLength, y: dx / axisLength };
  const omitEndUpper = probe.omitEndUpper ?? probe.omitUpperEnd ?? false;
  const omitEndLower = probe.omitEndLower ?? probe.omitLowerEnd ?? false;
  const attachments = [
    { node: "begin", center: begin, point: points.beginUpper, omitted: probe.omitBeginUpper ?? false },
    { node: "begin", center: begin, point: points.beginLower, omitted: probe.omitBeginLower ?? false },
    { node: "end", center: end, point: points.endUpper, omitted: omitEndUpper },
    { node: "end", center: end, point: points.endLower, omitted: omitEndLower },
  ];
  const counts = { begin: { left: 0, right: 0 }, end: { left: 0, right: 0 } };
  for (const attachment of attachments) {
    if (attachment.omitted) continue;
    const endpoint = probe.reverseAxis
      ? (attachment.node === "begin" ? "end" : "begin")
      : attachment.node;
    const vx = attachment.point.x - attachment.center.x;
    const vy = attachment.point.y - attachment.center.y;
    const length = Math.hypot(vx, vy);
    const sideScore = vx * normal.x + vy * normal.y;
    if (
      Math.abs(sideScore) / length <
      CHEMDRAW_DOUBLE_BOND_MIN_ATTACHMENT_SIDE_SINE
    ) {
      continue;
    }
    counts[endpoint][sideScore > 0 ? "left" : "right"] += 1;
  }
  const beginTotal = counts.begin.left + counts.begin.right;
  const endTotal = counts.end.left + counts.end.right;
  if (beginTotal + endTotal === 0) return "center";
  if (
    (endTotal === 0 && counts.begin.left > 0 && counts.begin.right > 0)
    || (beginTotal === 0 && counts.end.left > 0 && counts.end.right > 0)
  ) {
    return "center";
  }
  const leftCoverage = Number(counts.begin.left > 0) + Number(counts.end.left > 0);
  const rightCoverage = Number(counts.begin.right > 0) + Number(counts.end.right > 0);
  if (leftCoverage > rightCoverage) return "left";
  return "right";
}

function attribute(tag, name) {
  return tag.match(new RegExp(`\\b${name}="([^"]*)"`, "i"))?.[1] ?? null;
}

function transformMatrix(tag) {
  const transform = attribute(tag, "transform");
  if (!transform) return null;
  const matrix = transform
    .match(/matrix\(([^)]*)\)/i)?.[1]
    ?.trim()
    .split(/[\s,]+/)
    .map(Number);
  if (!matrix || matrix.length !== 6 || matrix.some((value) => !Number.isFinite(value))) {
    return null;
  }
  return matrix;
}

function transformPoint(point, matrix) {
  const [a, b, c, dScale, e, f] = matrix;
  return {
    x: a * point.x + c * point.y + e,
    y: b * point.x + dScale * point.y + f,
  };
}

function transformedPathPoints(tag) {
  const d = attribute(tag, "d");
  const matrix = transformMatrix(tag);
  if (!d || !matrix) return null;
  const numbers = d.match(/[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g)?.map(Number) ?? [];
  if (numbers.length < 4 || numbers.length % 2 !== 0) return null;
  return numbers.reduce((points, value, index) => {
    if (index % 2 !== 0) return points;
    points.push(transformPoint({ x: value, y: numbers[index + 1] }, matrix));
    return points;
  }, []);
}

function classifySvg(svg, probe) {
  const tags = (svg.match(/<path\b[^>]*>/gi) ?? [])
    .filter((tag) => /\bfill="#000000"/i.test(tag));
  const matrix = transformMatrix(tags[0]);
  if (!matrix) throw new Error(`${probe.name}: no SVG path transform`);
  const endpoints = axisEndpoints(probe);
  const sourceBegin = probe.reverseAxis ? endpoints.end : endpoints.begin;
  const sourceEnd = probe.reverseAxis ? endpoints.begin : endpoints.end;
  const begin = transformPoint({ x: sourceBegin.x * 20, y: sourceBegin.y * 20 }, matrix);
  const end = transformPoint({ x: sourceEnd.x * 20, y: sourceEnd.y * 20 }, matrix);
  const axisLength = Math.hypot(end.x - begin.x, end.y - begin.y);
  const axis = { x: (end.x - begin.x) / axisLength, y: (end.y - begin.y) / axisLength };
  const normal = { x: -axis.y, y: axis.x };
  const axisNormal = ((begin.x + end.x) * 0.5) * normal.x
    + ((begin.y + end.y) * 0.5) * normal.y;
  const paths = tags
    .map(transformedPathPoints)
    .filter(Boolean)
    .map((points) => {
      const axial = points.map((point) => point.x * axis.x + point.y * axis.y);
      const normalValues = points.map((point) => point.x * normal.x + point.y * normal.y);
      return {
        axisSpan: Math.max(...axial) - Math.min(...axial),
        normalSpan: Math.max(...normalValues) - Math.min(...normalValues),
        normalCenter: (Math.max(...normalValues) + Math.min(...normalValues)) * 0.5,
      };
    });
  const parallel = paths
    .filter((entry) => entry.axisSpan > 40 && entry.normalSpan < 8)
    .sort((left, right) => right.axisSpan - left.axisSpan)
    .slice(0, 2);
  if (parallel.length !== 2) {
    throw new Error(`${probe.name}: expected two parallel double-bond paths, found ${parallel.length}`);
  }
  const byAxisDistance = parallel
    .map((entry) => ({ ...entry, axisDistance: entry.normalCenter - axisNormal }))
    .sort((left, right) => Math.abs(left.axisDistance) - Math.abs(right.axisDistance));
  const [main, secondary] = byAxisDistance;
  const offset = secondary.normalCenter - main.normalCenter;
  const spanDelta = main.axisSpan - secondary.axisSpan;
  const logicalPlacement = Math.abs(main.axisDistance) <= 2
    ? (secondary.axisDistance > 0 ? "left" : "right")
    : "center";
  return {
    logicalPlacement,
    sourceAxisSpan: axisLength,
    axisNormal,
    mainAxisDistance: main.axisDistance,
    secondaryAxisDistance: secondary.axisDistance,
    mainSpan: main.axisSpan,
    secondarySpan: secondary.axisSpan,
    spanDelta,
    mainNormal: main.normalCenter,
    secondaryNormal: secondary.normalCenter,
    offset,
  };
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
    formats: ["svg", "cdxml"],
    inputs: missing.map((probe) => probe.input),
  });
}

const results = [];
for (const probe of probes) {
  const svg = await fs.readFile(probe.svg, "utf8");
  const measured = classifySvg(svg, probe);
  const expectedSecondarySpan = probe.expectedSecondaryInsetEndpoints
    && measured.logicalPlacement !== "center"
    ? measured.sourceAxisSpan
      - probe.expectedSecondaryInsetEndpoints
        * Math.abs(measured.secondaryAxisDistance)
        * Math.tan((probe.upperAngle ?? 30) * Math.PI / 360)
    : null;
  results.push({
    name: probe.name,
    lowerDelta: probe.lowerDelta ?? 0,
    upperDelta: probe.upperDelta ?? 0,
    absoluteStereo: probe.absoluteStereo ?? null,
    circularOrdering: probe.circularOrdering ?? null,
    reverseAxis: probe.reverseAxis ?? false,
    hetero: probe.hetero ?? false,
    omitLowerEnd: probe.omitLowerEnd ?? false,
    omitUpperEnd: probe.omitUpperEnd ?? false,
    omitBeginUpper: probe.omitBeginUpper ?? false,
    omitBeginLower: probe.omitBeginLower ?? false,
    omitEndUpper: probe.omitEndUpper ?? probe.omitUpperEnd ?? false,
    omitEndLower: probe.omitEndLower ?? probe.omitLowerEnd ?? false,
    upperAngle: probe.upperAngle ?? 30,
    lowerAngle: probe.lowerAngle ?? 30,
    upperBeginAngle: probe.upperBeginAngle ?? probe.upperAngle ?? 30,
    upperEndAngle: probe.upperEndAngle ?? probe.upperAngle ?? 30,
    lowerBeginAngle: probe.lowerBeginAngle ?? probe.lowerAngle ?? 30,
    lowerEndAngle: probe.lowerEndAngle ?? probe.lowerAngle ?? 30,
    rotation: probe.rotation ?? 0,
    axisLength: probe.axisLength ?? 60,
    bondSpacing: probe.bondSpacing ?? 12,
    bondSpacingAbs: probe.bondSpacingAbs ?? null,
    lineWidth: probe.lineWidth ?? 1,
    expectedPlacement: expectedPlacement(probe),
    expectedSecondarySpan,
    secondarySpanError: expectedSecondarySpan == null
      ? null
      : measured.secondarySpan - expectedSecondarySpan,
    ...measured,
  });
}

await fs.writeFile(
  path.join(outDir, "report.json"),
  `${JSON.stringify(results, null, 2)}\n`,
  "utf8",
);
const mismatches = results.filter(
  (entry) => entry.logicalPlacement !== entry.expectedPlacement
    || (entry.secondarySpanError != null && Math.abs(entry.secondarySpanError) > 0.02),
);
if (mismatches.length > 0) {
  console.error(JSON.stringify(mismatches, null, 2));
  throw new Error(`${mismatches.length} ChemDraw double-bond side probes violate the measured rule`);
}
console.log(JSON.stringify(results, null, 2));
