import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import { compareSvgPixels } from "./compare-svg-pixels.mjs";

const execFileAsync = promisify(execFile);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const probeDir = path.join(root, "tmp", "chemdraw-bioshape-probe");
const manifestPath = path.join(probeDir, "manifest.json");
const baselinePath = path.join(root, "scripts", "fixtures", "chemdraw-bioshape-visual-baseline.json");
const updateBaseline = process.argv.includes("--update-baseline");
const reportOnly = process.argv.includes("--report-only");
const cli = path.join(root, "target", "debug", "chemsema-cli.exe");
const CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT = 20 * 0.133333;

const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
if (manifest.schema !== "chemsema.chemdraw-bioshape-probe.v1" || manifest.cases.length !== 21) {
  throw new Error("Run npm run probe:chemdraw-bioshapes first; the complete 21-case cache is required.");
}
await fs.access(cli);
const outputDir = path.join(probeDir, "visual-gate");
await fs.mkdir(outputDir, { recursive: true });

const results = [];
for (const entry of manifest.cases) {
  const stem = path.basename(entry.input, ".cdxml");
  const chemsemaSvg = path.join(outputDir, `${stem}.chemsema.svg`);
  await execFileAsync(cli, ["convert", path.join(root, entry.input), chemsemaSvg], { cwd: root });
  const comparison = await compareSvgPixels({
    outDir: path.join(outputDir, stem),
    leftPath: path.join(root, entry.svg),
    rightPath: chemsemaSvg,
    leftLabel: "ChemDraw",
    rightLabel: "ChemSema",
    baseScale: 2,
    searchLimit: 16,
    rightScaleMultiplier: CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT,
  });
  if (comparison.unionPixels <= 0) {
    throw new Error(`${entry.type}: blank comparison is not a valid visual result`);
  }
  results.push({
    type: entry.type,
    mismatchRatio: comparison.differentPixels / comparison.unionPixels,
    alignedIou: comparison.bestShift.iou,
    documentWidthRatio:
      comparison.widthScale / CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT,
    documentHeightRatio:
      comparison.heightScale / CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT,
    alignmentShiftDocumentUnits: {
      x: comparison.bestShift.dx /
        (2 * CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT),
      y: comparison.bestShift.dy /
        (2 * CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT),
    },
    detailDistanceDocumentUnits: {
      p99: Math.max(
        comparison.detailDistancePixels.leftToRight.p99,
        comparison.detailDistancePixels.rightToLeft.p99,
      ) / (2 * CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT),
      p999: Math.max(
        comparison.detailDistancePixels.leftToRight.p999,
        comparison.detailDistancePixels.rightToLeft.p999,
      ) / (2 * CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT),
      maximum: Math.max(
        comparison.detailDistancePixels.leftToRight.maximum,
        comparison.detailDistancePixels.rightToLeft.maximum,
      ) / (2 * CHEMDRAW_SVG_PIXELS_PER_DOCUMENT_UNIT),
    },
    montage: path.relative(root, comparison.outputs.montagePath),
  });
  console.log(
    `[BIOSHAPE VISUAL] ${entry.type}: mismatch=${results.at(-1).mismatchRatio.toFixed(4)} ` +
      `iou=${results.at(-1).alignedIou.toFixed(4)} ` +
      `p99=${results.at(-1).detailDistanceDocumentUnits.p99.toFixed(3)}du`,
  );
}

const report = {
  schema: "chemsema.chemdraw-bioshape-visual-gate.v2",
  metric: {
    description:
      "Absolute document-unit rasterization with translation-only maximum-overlap alignment and bidirectional foreground distance quantiles.",
    fixedDocumentScale: true,
    independentContentScaling: false,
    detailSensitive: true,
    maximumOverlapAlignment: true,
  },
  cases: results,
};
await fs.writeFile(
  path.join(outputDir, "report.json"),
  `${JSON.stringify(report, null, 2)}\n`,
);

if (updateBaseline) {
  await fs.mkdir(path.dirname(baselinePath), { recursive: true });
  await fs.writeFile(baselinePath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`[BIOSHAPE VISUAL] baseline updated: ${path.relative(root, baselinePath)}`);
} else if (!reportOnly) {
  const baseline = JSON.parse(await fs.readFile(baselinePath, "utf8"));
  if (baseline.schema !== report.schema) {
    throw new Error(
      "BioShape visual baseline uses the retired size-normalized metric; " +
        "run with --update-baseline only after reviewing every v2 montage.",
    );
  }
  const expected = new Map(baseline.cases.map((entry) => [entry.type, entry]));
  const failures = results.filter((entry) => {
    const reference = expected.get(entry.type);
    if (!reference) throw new Error(`Missing visual baseline for ${entry.type}`);
    return entry.mismatchRatio > reference.mismatchRatio + 0.015
      || entry.alignedIou + 0.015 < reference.alignedIou
      || entry.detailDistanceDocumentUnits.p99 >
        reference.detailDistanceDocumentUnits.p99 + 0.2
      || entry.detailDistanceDocumentUnits.p999 >
        reference.detailDistanceDocumentUnits.p999 + 0.3;
  });
  if (failures.length) {
    throw new Error(`BioShape visual regressions: ${failures.map((entry) => entry.type).join(", ")}`);
  }
}
