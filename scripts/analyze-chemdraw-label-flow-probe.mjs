import fs from "node:fs/promises";
import path from "node:path";

function parseArgs(argv) {
  const options = { report: null };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--report") options.report = path.resolve(argv[++index]);
    else if (argument === "--help" || argument === "-h") options.help = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }
  return options;
}

function normalizeDegrees(angle) {
  return ((angle % 360) + 360) % 360;
}

function quantizedDirection(angle, rotation, length) {
  const radians = (angle + rotation) * Math.PI / 180;
  const x = Number((100 + length * Math.cos(radians)).toFixed(4)) - 100;
  const y = Number((100 + length * Math.sin(radians)).toFixed(4)) - 100;
  return normalizeDegrees(Math.atan2(y, x) * 180 / Math.PI);
}

function classifyBisector(bisector) {
  if (bisector <= 67.5 || bisector >= 292.5) return "reverse-horizontal";
  if (bisector < 112.5) return "stack-above";
  if (bisector <= 247.5) return "forward-horizontal";
  return "stack-below";
}

const GAP_TIE_EPSILON_DEG = 0.001;
const NEAR_TRIGONAL_GAP_DEVIATION_DEG = 3;

function largestGapPrediction(result) {
  const nominalAngles = [result.angle, result.fixedAngle];
  if (result.thirdAngle !== undefined) nominalAngles.push(result.thirdAngle);
  if (result.fourthAngle !== undefined) nominalAngles.push(result.fourthAngle);
  const angles = nominalAngles
    .map((angle) => quantizedDirection(angle, result.rotation, result.length))
    .sort((left, right) => left - right);
  const gaps = angles.map((angle, index) => {
    const next = index + 1 === angles.length ? angles[0] + 360 : angles[index + 1];
    return { index, gap: next - angle };
  });
  const largestGap = Math.max(...gaps.map((entry) => entry.gap));
  const winners = gaps.filter(
    (entry) => Math.abs(entry.gap - largestGap) <= GAP_TIE_EPSILON_DEG,
  );
  if (
    angles.length === 2
    && winners.length === 2
    && Math.abs(largestGap - 180) <= GAP_TIE_EPSILON_DEG
  ) {
    const axis = angles[0] % 180;
    const prediction = axis <= 22.5 + GAP_TIE_EPSILON_DEG
      || axis >= 157.5 - GAP_TIE_EPSILON_DEG
      ? "stack-above"
      : axis <= 90 + GAP_TIE_EPSILON_DEG
        ? "forward-horizontal"
        : "reverse-horizontal";
    return {
      angles,
      largestGap,
      axis,
      prediction,
      tiedGaps: winners.length,
    };
  }
  const nearTrigonal = angles.length === 3 && gaps.every(
    (entry) =>
      Math.abs(entry.gap - 120)
      <= NEAR_TRIGONAL_GAP_DEVIATION_DEG + GAP_TIE_EPSILON_DEG,
  );
  if (nearTrigonal) {
    const phase = normalizeDegrees(
      angles.reduce((sum, angle, index) => sum + angle - index * 120, 0) / 3,
    ) % 120;
    const prediction = phase <= 60 || phase >= 112.5
      ? "forward-horizontal"
      : phase <= 67.5
        ? "reverse-horizontal"
        : "stack-above";
    return {
      angles,
      largestGap,
      phase,
      prediction,
      tiedGaps: winners.length,
      nearTrigonal: true,
    };
  }
  const winner = winners.length === 1
    ? winners[0]
    : winners
      .map((entry) => {
        const gapStart = angles[entry.index];
        const gapMidpoint = normalizeDegrees(gapStart + entry.gap * 0.5);
        const clockwiseFromUp = normalizeDegrees(gapMidpoint - 270);
        return {
          ...entry,
          gapMidpoint,
          clockwiseFromUp,
          rightAxisDistance: Math.min(gapMidpoint, 360 - gapMidpoint),
          distanceFromUp: Math.min(clockwiseFromUp, 360 - clockwiseFromUp),
        };
      })
      .sort((left, right) =>
        Number(left.rightAxisDistance > GAP_TIE_EPSILON_DEG)
        - Number(right.rightAxisDistance > GAP_TIE_EPSILON_DEG)
        || left.distanceFromUp - right.distanceFromUp
        || left.clockwiseFromUp - right.clockwiseFromUp)[0];
  const occupiedStart = winner.index + 1 === angles.length
    ? angles[0]
    : angles[winner.index + 1];
  const occupiedSpan = 360 - largestGap;
  const bisector = normalizeDegrees(occupiedStart + occupiedSpan * 0.5);
  return {
    angles,
    largestGap,
    bisector,
    prediction: classifyBisector(bisector),
    tiedGaps: winners.length,
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help || !options.report) {
    console.log("Usage: node scripts/analyze-chemdraw-label-flow-probe.mjs --report path");
    return;
  }
  const report = JSON.parse(await fs.readFile(options.report, "utf8"));
  const mismatches = [];
  const ties = [];
  let resolvedTieCount = 0;
  for (const result of report.results) {
    const analysis = largestGapPrediction(result);
    const record = {
      nominalAngles: [
        result.angle,
        result.fixedAngle,
        result.thirdAngle,
        result.fourthAngle,
      ]
        .filter((angle) => angle !== undefined),
      actualAngles: analysis.angles,
      bisector: analysis.bisector ?? null,
      tiedGaps: analysis.tiedGaps,
      nearTrigonal: analysis.nearTrigonal ?? false,
      prediction: analysis.prediction,
      actual: result.layout,
    };
    if (analysis.tiedGaps > 1 && analysis.prediction !== null) resolvedTieCount += 1;
    if (analysis.prediction === null) ties.push(record);
    else if (analysis.prediction !== result.layout) mismatches.push(record);
  }
  const summary = {
    schema: "chemsema.chemdraw-label-flow-analysis.v1",
    report: options.report,
    total: report.results.length,
    mismatches,
    ties,
    resolvedTieCount,
  };
  console.log(JSON.stringify(summary, null, 2));
  if (mismatches.length || ties.length) process.exitCode = 1;
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exit(1);
});
