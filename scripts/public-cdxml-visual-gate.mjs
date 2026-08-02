import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { deflateRawSync, inflateRawSync } from "node:zlib";
import { launchBrowser } from "./playwright-browser.mjs";
import {
  computeImageAlignment,
  IMAGE_ALIGNMENT_ALGORITHM,
  mapWithConcurrency,
  sourceAlignmentPolicy,
  viewerHtml,
} from "./render-public-cdxml-visual-review.mjs";
import {
  collectCurrentGalleryProvenance,
  provenanceMismatches,
} from "./public-cdxml-provenance.mjs";
import { matchesPublicCdxmlCasePattern } from "./public-cdxml-case-filter.mjs";

const DEFAULTS = Object.freeze({
  gallery: "tmp/public-cdxml-chemdraw-review-all",
  out: "tmp/public-cdxml-visual-gate/report.json",
  jobs: 4,
  allowDirtyGallery: false,
  allowStaleGallery: false,
  strictOriginal338: false,
  gateDefinitionUpgrade: false,
  cohortLedger: "benchmarks/public-cdxml/failure-ledger.json",
  cohort: null,
  analysisScale: 2,
  tolerance: 1.5,
  tileSize: 256,
  halo: 24,
  localWindow: 48,
  localStride: 24,
  minimumWindowInk: 4,
  minCoverage: 0.75,
  maxDefectArea: 8,
  maxDefectSpan: 12,
  detailAnalysisScale: 4,
  detailTolerance: 0,
  detailLocalWindow: 24,
  detailLocalStride: 12,
  detailMinimumWindowInk: 12,
  // Smaller than a normal ChemDraw stroke. An unmatched raster component
  // inside this distance of opposite-side ink is a split/merged stroke, not
  // independent evidence that an object exists on only one side.
  componentSupportGap: 0.5,
  maxComponentCountDelta: 1,
  maxEnclosedSmallComponentDimensionDelta: 2.75,
  maxRepeatedMicroDefects: 20,
  maxRepeatedMicroDefectArea: 5,
  minRepeatedMicroCoverage: 0.75,
  minDisplacedDefectArea: 8,
  minDisplacedDefectDistance: 2,
  maxDisplacedDefectAreaRatio: 1.35,
  maxDisplacedDefectDimensionDelta: 1.5,
  maxRelativeComponentCenterDistance: 0.02,
  candidateViewportAnalysisScale: 4,
  maxCandidateViewportPixels: 16_000_000,
  minCandidateViewportInkMargin: 4,
  // Continuous floors must rasterize unchanged SVG primitives at the same
  // absolute pixel positions even when the export crop grows or shrinks.
  // The baseline viewport is expanded once and then reused verbatim. A later
  // candidate outside that registered viewport is rejected as non-comparable
  // instead of silently changing the raster phase.
  candidateRegressionRasterPadding: 32,
});

const ALIGNMENT_ALGORITHM = IMAGE_ALIGNMENT_ALGORITHM;
export const CACHE_IDENTITY = "chemsema-public-cdxml-visual-gate-cache-v25";
export const REPORT_SCHEMA = "chemsema-public-cdxml-visual-gate-v2";
export const CASE_METRICS_SCHEMA =
  "chemsema.public-cdxml-visual-case-metrics.v2";
export const STRICT_PASS_FLOOR_SCHEMA =
  "chemsema.public-cdxml-strict-pass-floor.v5";
export const STRICT_PASS_FLOOR_PATH = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "benchmarks",
  "public-cdxml",
  "strict-pass-floor.json",
);

function parseArgs(argv) {
  const options = { ...DEFAULTS, patterns: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--gallery") options.gallery = argv[++index];
    else if (arg === "--out") options.out = argv[++index];
    else if (arg === "--passed-gallery") options.passedGallery = argv[++index];
    else if (arg === "--reuse-report") options.reuseReport = argv[++index];
    else if (arg === "--baseline-report") options.baselineReport = argv[++index];
    else if (arg === "--only") options.patterns.push(argv[++index]);
    else if (arg === "--limit") options.limit = Number(argv[++index]);
    else if (arg === "--jobs") options.jobs = Number(argv[++index]);
    else if (arg === "--cohort-ledger") options.cohortLedger = argv[++index];
    else if (arg === "--cohort") options.cohort = argv[++index];
    else if (arg === "--allow-dirty-gallery") options.allowDirtyGallery = true;
    else if (arg === "--allow-stale-gallery") options.allowStaleGallery = true;
    else if (arg === "--strict-original-338") options.strictOriginal338 = true;
    else if (arg === "--gate-definition-upgrade") options.gateDefinitionUpgrade = true;
    else if (arg === "--analysis-scale") options.analysisScale = Number(argv[++index]);
    else if (arg === "--tolerance") options.tolerance = Number(argv[++index]);
    else if (arg === "--tile-size") options.tileSize = Number(argv[++index]);
    else if (arg === "--halo") options.halo = Number(argv[++index]);
    else if (arg === "--local-window") options.localWindow = Number(argv[++index]);
    else if (arg === "--local-stride") options.localStride = Number(argv[++index]);
    else if (arg === "--minimum-window-ink") options.minimumWindowInk = Number(argv[++index]);
    else if (arg === "--min-coverage") options.minCoverage = Number(argv[++index]);
    else if (arg === "--max-defect-area") options.maxDefectArea = Number(argv[++index]);
    else if (arg === "--max-defect-span") options.maxDefectSpan = Number(argv[++index]);
    else if (arg === "--detail-analysis-scale") options.detailAnalysisScale = Number(argv[++index]);
    else if (arg === "--detail-tolerance") options.detailTolerance = Number(argv[++index]);
    else if (arg === "--detail-local-window") options.detailLocalWindow = Number(argv[++index]);
    else if (arg === "--detail-local-stride") options.detailLocalStride = Number(argv[++index]);
    else if (arg === "--detail-minimum-window-ink") options.detailMinimumWindowInk = Number(argv[++index]);
    else if (arg === "--max-component-count-delta") options.maxComponentCountDelta = Number(argv[++index]);
    else if (arg === "--max-enclosed-small-component-dimension-delta") options.maxEnclosedSmallComponentDimensionDelta = Number(argv[++index]);
    else if (arg === "--max-repeated-micro-defects") options.maxRepeatedMicroDefects = Number(argv[++index]);
    else if (arg === "--max-repeated-micro-defect-area") options.maxRepeatedMicroDefectArea = Number(argv[++index]);
    else if (arg === "--min-repeated-micro-coverage") options.minRepeatedMicroCoverage = Number(argv[++index]);
    else if (arg === "--min-displaced-defect-area") options.minDisplacedDefectArea = Number(argv[++index]);
    else if (arg === "--min-displaced-defect-distance") options.minDisplacedDefectDistance = Number(argv[++index]);
    else if (arg === "--max-displaced-defect-area-ratio") options.maxDisplacedDefectAreaRatio = Number(argv[++index]);
    else if (arg === "--max-displaced-defect-dimension-delta") options.maxDisplacedDefectDimensionDelta = Number(argv[++index]);
    else if (arg === "--max-relative-component-center-distance") options.maxRelativeComponentCenterDistance = Number(argv[++index]);
    else if (arg === "--candidate-viewport-analysis-scale") options.candidateViewportAnalysisScale = Number(argv[++index]);
    else if (arg === "--max-candidate-viewport-pixels") options.maxCandidateViewportPixels = Number(argv[++index]);
    else if (arg === "--min-candidate-viewport-ink-margin") options.minCandidateViewportInkMargin = Number(argv[++index]);
    else if (arg === "--candidate-regression-raster-padding") options.candidateRegressionRasterPadding = Number(argv[++index]);
    else if (arg === "--report-only") options.reportOnly = true;
    else if (arg === "--self-test") options.selfTest = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return options;
}

export function strictOriginal338ConfigurationErrors(options) {
  if (!options.strictOriginal338) return [];
  const errors = [];
  if (options.allowDirtyGallery) errors.push("--allow-dirty-gallery is forbidden");
  if (options.allowStaleGallery) errors.push("--allow-stale-gallery is forbidden");
  if (options.reportOnly) errors.push("--report-only is forbidden");
  if (options.reuseReport) errors.push("--reuse-report is forbidden");
  if (options.patterns?.length) errors.push("--only is forbidden");
  if (Number.isFinite(options.limit)) errors.push("--limit is forbidden");
  if (options.cohort && options.cohort !== "original-338") {
    errors.push("--cohort must be original-338");
  }
  return errors;
}

export function gateDefinitionUpgradeConfigurationErrors(options) {
  if (!options.gateDefinitionUpgrade) return [];
  const errors = [];
  if (options.strictOriginal338) errors.push("--strict-original-338 is forbidden");
  if (!options.reportOnly) errors.push("--report-only is required");
  if (options.reuseReport) errors.push("--reuse-report is forbidden");
  if (options.baselineReport) errors.push("--baseline-report is forbidden");
  if (options.patterns?.length) errors.push("--only is forbidden");
  if (Number.isFinite(options.limit)) errors.push("--limit is forbidden");
  if (options.cohort && options.cohort !== "original-338") {
    errors.push("--cohort must be original-338");
  }
  return errors;
}

export function strictOriginal338BaselineErrors(baselineReport, selectedItems, options) {
  if (!options.strictOriginal338) return [];
  if (!baselineReport) return [];
  const errors = [];
  const cohort = baselineReport?.selection?.cohort;
  if (
    cohort?.name !== "original-338"
    || cohort?.expected !== 338
    || cohort?.selected !== 338
  ) {
    errors.push("baseline report is not the exact original-338 cohort");
  }
  const baselinePaths = new Set(
    (baselineReport?.cases ?? []).map((entry) => entry.relativeCdxml),
  );
  const selectedPaths = new Set(selectedItems.map((entry) => entry.relativeCdxml));
  if (baselinePaths.size !== 338 || selectedPaths.size !== 338) {
    errors.push("baseline and current selection must each contain 338 unique paths");
  } else {
    for (const relativeCdxml of selectedPaths) {
      if (!baselinePaths.has(relativeCdxml)) {
        errors.push(`baseline is missing selected path ${relativeCdxml}`);
        break;
      }
    }
    for (const relativeCdxml of baselinePaths) {
      if (!selectedPaths.has(relativeCdxml)) {
        errors.push(`baseline contains an unexpected path ${relativeCdxml}`);
        break;
      }
    }
  }
  return errors;
}

function corpusIdentity(provenance) {
  const corpus = provenance?.corpus;
  if (!corpus?.manifestSha256 || !Array.isArray(corpus.sources)) return null;
  return JSON.stringify({
    manifestSha256: corpus.manifestSha256,
    sources: corpus.sources
      .map((source) => ({
        id: source.id,
        actualRevision: source.actualRevision,
      }))
      .sort((left, right) => String(left.id).localeCompare(String(right.id))),
  });
}

export function visualBaselineCompatibilityErrors(
  baselineReport,
  currentGalleryProvenance,
  currentReferenceHashes,
  options = {},
) {
  const errors = [];
  if (baselineReport?.schema !== REPORT_SCHEMA) {
    errors.push("baseline report schema is missing or unsupported");
  }
  if (baselineReport?.caseMetricsSchema !== CASE_METRICS_SCHEMA) {
    errors.push("baseline case metrics schema is missing or unsupported");
  }
  if (
    baselineReport?.galleryProvenance?.schema
    !== "chemsema.public-cdxml-gallery-provenance.v1"
  ) {
    errors.push("baseline gallery provenance is missing or unsupported");
  }
  const baselineCorpus = corpusIdentity(baselineReport?.galleryProvenance);
  const currentCorpus = corpusIdentity(currentGalleryProvenance);
  if (!baselineCorpus || !currentCorpus || baselineCorpus !== currentCorpus) {
    errors.push("baseline and current corpus identities differ");
  }
  const seenPaths = new Set();
  const baselineCases = new Map();
  for (const entry of baselineReport?.cases ?? []) {
    const relativeCdxml = normalizedCasePath(entry.relativeCdxml);
    if (!relativeCdxml || seenPaths.has(relativeCdxml)) {
      errors.push("baseline contains a missing or duplicate case path");
      break;
    }
    seenPaths.add(relativeCdxml);
    baselineCases.set(relativeCdxml, entry);
    const metricErrors = visualCaseMetricContractErrors(entry, options);
    if (metricErrors.length) {
      errors.push(`baseline case ${relativeCdxml} has invalid metrics: ${metricErrors.join(", ")}`);
      break;
    }
  }
  for (const [relativeCdxml, referenceHash] of currentReferenceHashes) {
    const baselineCase = baselineCases.get(normalizedCasePath(relativeCdxml));
    if (!baselineCase) {
      errors.push(`baseline is missing current case ${relativeCdxml}`);
      break;
    }
    if (!baselineCase.artifactHashes?.reference) {
      errors.push(`baseline is missing reference hash for ${relativeCdxml}`);
      break;
    }
    if (baselineCase.artifactHashes.reference !== referenceHash) {
      errors.push(`ChemDraw oracle changed for ${relativeCdxml}`);
      break;
    }
  }
  return errors;
}

function normalizedCasePath(relativeCdxml) {
  return String(relativeCdxml ?? "").replaceAll("\\", "/");
}

export function strictOriginal338PassFloorErrors(
  passFloor,
  selectedItems,
  _baselineReport,
  options,
  evaluatePassFloor = options.strictOriginal338,
) {
  if (!evaluatePassFloor) return [];
  const errors = [];
  if (passFloor?.schema !== STRICT_PASS_FLOOR_SCHEMA) {
    errors.push("pass floor has a missing or unsupported schema");
  }
  const expectedGateDefinition = passFloorGateDefinition(options);
  if (
    JSON.stringify(passFloor?.gateDefinition)
    !== JSON.stringify(expectedGateDefinition)
  ) {
    errors.push("pass floor was established by a different gate definition");
  }
  if (
    passFloor?.cohort?.name !== "original-338"
    || passFloor?.cohort?.expected !== 338
  ) {
    errors.push("pass floor is not bound to the exact original-338 cohort");
  }
  if (!Array.isArray(passFloor?.protectedPasses) || !passFloor.protectedPasses.length) {
    errors.push("pass floor must protect at least one passing case");
    return errors;
  }
  const protectedPaths = passFloor.protectedPasses.map(normalizedCasePath);
  const protectedSet = new Set(protectedPaths);
  if (protectedSet.size !== protectedPaths.length) {
    errors.push("pass floor contains duplicate paths");
  }
  const canonicalPaths = [...protectedPaths].sort();
  if (JSON.stringify(canonicalPaths) !== JSON.stringify(protectedPaths)) {
    errors.push("pass floor paths must be sorted");
  }
  if (passFloor.minimumPassed !== protectedSet.size) {
    errors.push("pass floor minimumPassed does not match protectedPasses");
  }
  const selectedPaths = new Set(
    selectedItems.map((entry) => normalizedCasePath(entry.relativeCdxml)),
  );
  for (const relativeCdxml of protectedSet) {
    if (!selectedPaths.has(relativeCdxml)) {
      errors.push(`pass floor contains a path outside the current cohort: ${relativeCdxml}`);
      break;
    }
  }
  if (!Array.isArray(passFloor?.protectedCases)) {
    errors.push("pass floor must contain protectedCases");
    return errors;
  }
  const protectedCases = passFloor.protectedCases;
  const protectedCasePaths = protectedCases.map((entry) =>
    normalizedCasePath(entry.relativeCdxml));
  const protectedCaseSet = new Set(protectedCasePaths);
  if (protectedCases.length !== 338 || protectedCaseSet.size !== 338) {
    errors.push("pass floor must protect exactly 338 unique cases");
  }
  const canonicalProtectedCasePaths = [...protectedCasePaths]
    .sort((left, right) => left.localeCompare(right));
  if (JSON.stringify(canonicalProtectedCasePaths) !== JSON.stringify(protectedCasePaths)) {
    errors.push("pass floor protectedCases must be sorted");
  }
  for (const entry of protectedCases) {
    const relativeCdxml = normalizedCasePath(entry.relativeCdxml);
    if (!selectedPaths.has(relativeCdxml)) {
      errors.push(`pass floor contains a protected case outside the cohort: ${relativeCdxml}`);
      break;
    }
    if (!["pass", "fail", "unavailable"].includes(entry.status)) {
      errors.push(`pass floor has invalid status for ${relativeCdxml}`);
      break;
    }
    if (!entry.artifactHashes?.reference) {
      errors.push(`pass floor is missing the ChemDraw oracle hash for ${relativeCdxml}`);
      break;
    }
    if (entry.status === "pass" && !protectedSet.has(relativeCdxml)) {
      errors.push(`pass floor protectedCases has an unlisted pass: ${relativeCdxml}`);
      break;
    }
    if (entry.status !== "pass" && protectedSet.has(relativeCdxml)) {
      errors.push(`pass floor protectedPasses has a non-pass case: ${relativeCdxml}`);
      break;
    }
    if (entry.status === "fail") {
      if (
        entry.analysisLayers?.coarse !== true
        || entry.analysisLayers?.detail !== true
      ) {
        errors.push(`pass floor is missing analysis layer metadata for ${relativeCdxml}`);
        break;
      }
      const regressionAlignment = entry.regressionAlignment;
      if (
        regressionAlignment?.algorithm !== ALIGNMENT_ALGORITHM
        || !Number.isFinite(regressionAlignment.scale)
        || regressionAlignment.scale <= 0
        || !Number.isFinite(regressionAlignment.dx)
        || !Number.isFinite(regressionAlignment.dy)
        || !Number.isFinite(regressionAlignment.vectorFrame?.chemsemaViewBox?.x)
        || !Number.isFinite(regressionAlignment.vectorFrame?.chemsemaViewBox?.y)
      ) {
        errors.push(`pass floor is missing regression alignment for ${relativeCdxml}`);
        break;
      }
      const metricErrors = visualCaseMetricContractErrors(entry, options);
      if (metricErrors.length) {
        errors.push(
          `pass floor has invalid metrics for ${relativeCdxml}: ${metricErrors.join(", ")}`,
        );
        break;
      }
    }
  }
  return errors;
}

function validateOptions(options) {
  for (const key of [
    "analysisScale", "tileSize", "halo", "localWindow", "localStride",
    "minimumWindowInk", "detailAnalysisScale", "detailLocalWindow",
    "detailLocalStride", "detailMinimumWindowInk",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] <= 0) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be positive`);
    }
  }
  for (const key of [
    "tolerance",
    "detailTolerance",
    "componentSupportGap",
    "candidateRegressionRasterPadding",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] < 0) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be non-negative`);
    }
  }
  if (!Number.isInteger(options.jobs) || options.jobs < 1) {
    throw new Error("--jobs must be a positive integer");
  }
  for (const key of [
    "minCoverage", "minRepeatedMicroCoverage",
    "maxRelativeComponentCenterDistance",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] < 0 || options[key] > 1) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be between 0 and 1`);
    }
  }
  for (const key of [
    "maxDefectArea", "maxDefectSpan", "maxComponentCountDelta",
    "maxEnclosedSmallComponentDimensionDelta", "maxRepeatedMicroDefects",
    "maxRepeatedMicroDefectArea", "minDisplacedDefectArea",
    "minDisplacedDefectDistance", "maxDisplacedDefectDimensionDelta",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] < 0) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be non-negative`);
    }
  }
  if (
    !Number.isFinite(options.maxDisplacedDefectAreaRatio)
    || options.maxDisplacedDefectAreaRatio < 1
  ) {
    throw new Error("--max-displaced-defect-area-ratio must be at least 1");
  }
  if (options.halo <= options.tolerance) {
    throw new Error("--halo must be larger than --tolerance");
  }
  if (options.halo < options.localWindow / 2) {
    throw new Error("--halo must be at least half of --local-window");
  }
  if (options.halo <= options.detailTolerance) {
    throw new Error("--halo must be larger than --detail-tolerance");
  }
  if (options.halo < options.detailLocalWindow / 2) {
    throw new Error("--halo must be at least half of --detail-local-window");
  }
  for (const [label, value] of [
    ["analysis-scale × tile-size", options.analysisScale * options.tileSize],
    ["analysis-scale × halo", options.analysisScale * options.halo],
    ["analysis-scale × local-stride", options.analysisScale * options.localStride],
    ["detail-analysis-scale × tile-size", options.detailAnalysisScale * options.tileSize],
    ["detail-analysis-scale × halo", options.detailAnalysisScale * options.halo],
    [
      "detail-analysis-scale × detail-local-stride",
      options.detailAnalysisScale * options.detailLocalStride,
    ],
  ]) {
    if (!Number.isInteger(value)) {
      throw new Error(`${label} must define an integral pixel lattice`);
    }
  }
}

function mimeType(filePath) {
  return path.extname(filePath).toLowerCase() === ".png" ? "image/png" : "image/svg+xml";
}

async function fileDataUrl(filePath) {
  const bytes = await fs.readFile(filePath);
  return `data:${mimeType(filePath)};base64,${bytes.toString("base64")}`;
}

async function sha256File(filePath) {
  const bytes = await fs.readFile(filePath);
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

async function artifactHashes(galleryDir, item) {
  const [reference, candidate] = await Promise.all([
    sha256File(path.resolve(galleryDir, item.reference)),
    sha256File(path.resolve(galleryDir, item.chemsema)),
  ]);
  return { reference, candidate };
}

function reportsUseSameGateDefinition(report, options) {
  return report?.cacheIdentity === CACHE_IDENTITY
    && JSON.stringify(report.policy) === JSON.stringify(gatePolicy(options));
}

function artifactHashesEqual(left, right) {
  return left?.reference === right?.reference && left?.candidate === right?.candidate;
}

export function classifyBaselineChanges(cases, baselineCases) {
  const changes = cases.flatMap((entry) => {
    const before = baselineCases.get(normalizedCasePath(entry.relativeCdxml))?.status;
    return before && before !== entry.status
      ? [{ relativeCdxml: entry.relativeCdxml, before, after: entry.status }]
      : [];
  });
  return {
    changes,
    regressions: changes.filter((entry) => entry.before === "pass" && entry.after !== "pass"),
    improvements: changes.filter((entry) => entry.before !== "pass" && entry.after === "pass"),
  };
}

const CONTINUOUS_REGRESSION_METRICS = Object.freeze([
  { path: "referenceCoverage", direction: "higher", tolerance: 0.002 },
  { path: "candidateCoverage", direction: "higher", tolerance: 0.002 },
  { path: "local.referenceCoverage", direction: "higher", tolerance: 0.01 },
  { path: "local.candidateCoverage", direction: "higher", tolerance: 0.01 },
  { path: "largestMissing.area", direction: "lower", tolerance: 0.5 },
  { path: "largestExtra.area", direction: "lower", tolerance: 0.5 },
  { path: "largestMissing.span", direction: "lower", tolerance: 0.5 },
  { path: "largestExtra.span", direction: "lower", tolerance: 0.5 },
  { path: "detailFeatures.compactDefectCount", direction: "lower", tolerance: 1 },
  {
    path: "detailFeatures.componentMatchCoverage",
    direction: "higher",
    tolerance: 0.01,
  },
  {
    path: "detailFeatures.relativeComponentMatchCoverage",
    direction: "higher",
    tolerance: 0.01,
  },
  {
    path: "detailFeatures.componentPositionDistributionDelta",
    direction: "lower",
    tolerance: 0.01,
  },
  {
    path: "detailFeatures.independentComponentCountDelta",
    direction: "lower",
    tolerance: 1,
  },
  {
    path: "detailFeatures.unmatchedReferenceComponentCount",
    direction: "lower",
    tolerance: 1,
  },
  {
    path: "detailFeatures.unmatchedCandidateComponentCount",
    direction: "lower",
    tolerance: 1,
  },
  {
    path: "detailFeatures.smallComponentDimensionDelta",
    direction: "lower",
    tolerance: 0.5,
  },
  {
    path: "detailFeatures.enclosedSmallComponentDimensionDelta",
    direction: "lower",
    tolerance: 0.5,
  },
  {
    path: "detailFeatures.maximumMatchedCenterDistance",
    direction: "lower",
    tolerance: 0.5,
  },
  {
    path: "detailFeatures.maximumMatchedDimensionDelta",
    direction: "lower",
    tolerance: 0.5,
  },
  { path: "detail.local.referenceCoverage", direction: "higher", tolerance: 0.01 },
  { path: "detail.local.candidateCoverage", direction: "higher", tolerance: 0.01 },
  { path: "detail.largestMissing.area", direction: "lower", tolerance: 0.25 },
  { path: "detail.largestExtra.area", direction: "lower", tolerance: 0.25 },
  { path: "detail.largestMissing.span", direction: "lower", tolerance: 0.25 },
  { path: "detail.largestExtra.span", direction: "lower", tolerance: 0.25 },
  {
    path: "detail.detailFeatures.compactDefectCount",
    direction: "lower",
    tolerance: 1,
  },
  {
    path: "detail.detailFeatures.componentMatchCoverage",
    direction: "higher",
    tolerance: 0.01,
  },
  {
    path: "detail.detailFeatures.relativeComponentMatchCoverage",
    direction: "higher",
    tolerance: 0.01,
  },
  {
    path: "detail.detailFeatures.componentPositionDistributionDelta",
    direction: "lower",
    tolerance: 0.01,
  },
  {
    path: "detail.detailFeatures.independentComponentCountDelta",
    direction: "lower",
    tolerance: 1,
  },
]);

const LOCAL_WINDOW_REGRESSION_TOLERANCES = Object.freeze({
  coarse: Object.freeze({
    defectArea: 0.5,
    center: 0.75,
    improvementWindow: 12,
  }),
  detail: Object.freeze({
    defectArea: 0.25,
    center: 0.75,
    improvementWindow: 12,
  }),
});

const SPATIAL_FLOOR_ENCODING = "chemsema-spatial-floor-deflate-mask-v2";
const MAX_SPATIAL_FLOOR_CELLS = 1_000_000;
const MAX_SPATIAL_FLOOR_BYTES = 64 * 1024 * 1024;

function spatialCellPixelSize(analysisScale, stride) {
  const size = analysisScale * stride;
  if (
    !Number.isFinite(analysisScale)
    || analysisScale <= 0
    || !Number.isFinite(stride)
    || stride <= 0
    || !Number.isInteger(size)
    || size <= 0
  ) throw new Error("spatial floor requires an integral positive pixel lattice");
  return size;
}

function normalizedMaskWords(words, wordCount) {
  if (!Array.isArray(words)) throw new Error("spatial mask words must be an array");
  const normalized = words.map((entry) => {
    if (
      !Array.isArray(entry)
      || entry.length !== 2
      || !Number.isInteger(entry[0])
      || entry[0] < 0
      || entry[0] >= wordCount
      || !Number.isInteger(entry[1])
      || entry[1] <= 0
      || entry[1] > 0xffff_ffff
    ) throw new Error("invalid spatial mask word");
    return [entry[0], entry[1] >>> 0];
  }).sort((left, right) => left[0] - right[0]);
  for (let index = 1; index < normalized.length; index += 1) {
    if (normalized[index][0] === normalized[index - 1][0]) {
      throw new Error("duplicate spatial mask word");
    }
  }
  return normalized;
}

function popcount32(value) {
  let bits = value >>> 0;
  bits -= (bits >>> 1) & 0x5555_5555;
  bits = (bits & 0x3333_3333) + ((bits >>> 2) & 0x3333_3333);
  return (((bits + (bits >>> 4)) & 0x0f0f_0f0f) * 0x0101_0101) >>> 24;
}

function spatialMaskPixelCount(words) {
  return words.reduce((total, [, bits]) => total + popcount32(bits), 0);
}

function normalizedSpatialCell(cell, cellPixelSize) {
  if (!Number.isInteger(cell?.column) || !Number.isInteger(cell?.row)) {
    throw new Error("invalid spatial cell coordinate");
  }
  const wordCount = Math.ceil((cellPixelSize * cellPixelSize) / 32);
  const missingMaskWords = normalizedMaskWords(cell.missingMaskWords ?? [], wordCount);
  const extraMaskWords = normalizedMaskWords(cell.extraMaskWords ?? [], wordCount);
  const capacity = cellPixelSize * cellPixelSize;
  for (const words of [missingMaskWords, extraMaskWords]) {
    const last = words.at(-1);
    if (last && last[0] * 32 + (32 - Math.clz32(last[1])) > capacity) {
      throw new Error("spatial mask exceeds its cell");
    }
  }
  if (!missingMaskWords.length && !extraMaskWords.length) {
    throw new Error("spatial floor cannot store an empty cell");
  }
  return {
    column: cell.column,
    row: cell.row,
    missingMaskWords,
    extraMaskWords,
  };
}

function spatialFloorRecordBytes(cells) {
  return cells.reduce((total, cell) => total + 16
    + cell.missingMaskWords.length * 8
    + cell.extraMaskWords.length * 8, 0);
}

function encodeSpatialFloorRecords(cells) {
  const bytes = Buffer.allocUnsafe(spatialFloorRecordBytes(cells));
  let offset = 0;
  for (const cell of cells) {
    bytes.writeInt32LE(cell.column, offset);
    bytes.writeInt32LE(cell.row, offset + 4);
    bytes.writeUInt32LE(cell.missingMaskWords.length, offset + 8);
    offset += 12;
    for (const [wordIndex, bits] of cell.missingMaskWords) {
      bytes.writeUInt32LE(wordIndex, offset);
      bytes.writeUInt32LE(bits, offset + 4);
      offset += 8;
    }
    bytes.writeUInt32LE(cell.extraMaskWords.length, offset);
    offset += 4;
    for (const [wordIndex, bits] of cell.extraMaskWords) {
      bytes.writeUInt32LE(wordIndex, offset);
      bytes.writeUInt32LE(bits, offset + 4);
      offset += 8;
    }
  }
  return bytes;
}

export function encodeSpatialFloor(cells, analysisScale, stride) {
  const cellPixelSize = spatialCellPixelSize(analysisScale, stride);
  const canonical = [...(cells ?? [])]
    .map((cell) => normalizedSpatialCell(cell, cellPixelSize))
    .sort((left, right) => left.row - right.row || left.column - right.column);
  if (canonical.length > MAX_SPATIAL_FLOOR_CELLS) {
    throw new Error("spatial floor has too many cells");
  }
  for (let index = 1; index < canonical.length; index += 1) {
    if (
      canonical[index].row === canonical[index - 1].row
      && canonical[index].column === canonical[index - 1].column
    ) throw new Error("duplicate spatial floor cell");
  }
  const bytes = encodeSpatialFloorRecords(canonical);
  if (bytes.length > MAX_SPATIAL_FLOOR_BYTES) {
    throw new Error("spatial floor exceeds the decoded size limit");
  }
  const compressed = deflateRawSync(bytes, { level: 9 });
  const missingPixels = canonical.reduce((total, cell) =>
    total + spatialMaskPixelCount(cell.missingMaskWords), 0);
  const extraPixels = canonical.reduce((total, cell) =>
    total + spatialMaskPixelCount(cell.extraMaskWords), 0);
  return {
    encoding: SPATIAL_FLOOR_ENCODING,
    analysisScale,
    stride,
    count: canonical.length,
    missingPixels,
    extraPixels,
    rawBytes: bytes.length,
    compressedBytes: compressed.length,
    recordsSha256: crypto.createHash("sha256").update(bytes).digest("hex"),
    data: compressed.toString("base64"),
  };
}

function decodeSpatialFloor(floor) {
  const cellPixelSize = spatialCellPixelSize(floor?.analysisScale, floor?.stride);
  if (
    floor?.encoding !== SPATIAL_FLOOR_ENCODING
    || !Number.isInteger(floor.count)
    || floor.count < 0
    || floor.count > MAX_SPATIAL_FLOOR_CELLS
    || !Number.isInteger(floor.missingPixels)
    || floor.missingPixels < 0
    || !Number.isInteger(floor.extraPixels)
    || floor.extraPixels < 0
    || !Number.isInteger(floor.rawBytes)
    || floor.rawBytes < 0
    || floor.rawBytes > MAX_SPATIAL_FLOOR_BYTES
    || !Number.isInteger(floor.compressedBytes)
    || floor.compressedBytes < 0
    || floor.compressedBytes > MAX_SPATIAL_FLOOR_BYTES
    || !/^[0-9a-f]{64}$/.test(floor.recordsSha256 ?? "")
    || typeof floor.data !== "string"
  ) throw new Error("invalid spatial floor header");
  const compressed = Buffer.from(floor.data, "base64");
  if (
    compressed.length !== floor.compressedBytes
    || compressed.toString("base64") !== floor.data
  ) throw new Error("invalid spatial floor payload");
  const bytes = inflateRawSync(compressed, { maxOutputLength: MAX_SPATIAL_FLOOR_BYTES });
  if (
    bytes.length !== floor.rawBytes
    || crypto.createHash("sha256").update(bytes).digest("hex") !== floor.recordsSha256
  ) throw new Error("invalid spatial floor records");
  const wordCount = Math.ceil((cellPixelSize * cellPixelSize) / 32);
  const cells = [];
  let offset = 0;
  const readUInt32 = () => {
    if (offset + 4 > bytes.length) throw new Error("truncated spatial floor");
    const value = bytes.readUInt32LE(offset);
    offset += 4;
    return value;
  };
  const readInt32 = () => {
    if (offset + 4 > bytes.length) throw new Error("truncated spatial floor");
    const value = bytes.readInt32LE(offset);
    offset += 4;
    return value;
  };
  const readMaskWords = () => {
    const length = readUInt32();
    if (length > wordCount || offset + length * 8 > bytes.length) {
      throw new Error("invalid spatial mask length");
    }
    const words = [];
    for (let index = 0; index < length; index += 1) {
      words.push([readUInt32(), readUInt32()]);
    }
    return normalizedMaskWords(words, wordCount);
  };
  for (let index = 0; index < floor.count; index += 1) {
    const cell = normalizedSpatialCell({
      column: readInt32(),
      row: readInt32(),
      missingMaskWords: readMaskWords(),
      extraMaskWords: readMaskWords(),
    }, cellPixelSize);
    const previous = cells.at(-1);
    if (
      previous
      && (cell.row < previous.row
        || (cell.row === previous.row && cell.column <= previous.column))
    ) throw new Error("spatial floor cells are not canonical and unique");
    cells.push(cell);
  }
  if (offset !== bytes.length) throw new Error("spatial floor has trailing records");
  const missingPixels = cells.reduce((total, cell) =>
    total + spatialMaskPixelCount(cell.missingMaskWords), 0);
  const extraPixels = cells.reduce((total, cell) =>
    total + spatialMaskPixelCount(cell.extraMaskWords), 0);
  if (missingPixels !== floor.missingPixels || extraPixels !== floor.extraPixels) {
    throw new Error("spatial floor pixel totals do not match");
  }
  return cells;
}

function spatialFloorIsCanonical(floor, analysisScale, stride) {
  try {
    decodeSpatialFloor(floor);
    return floor.analysisScale === analysisScale && floor.stride === stride;
  } catch {
    return false;
  }
}

function spatialMaskCellIndex(cells, kind) {
  return new Map(cells.map((cell) => [
    `${cell.column},${cell.row}`,
    new Map(cell[`${kind}MaskWords`]),
  ]));
}

function spatialMaskContains(index, x, y, cellPixelSize) {
  const column = Math.floor(x / cellPixelSize);
  const row = Math.floor(y / cellPixelSize);
  const localX = x - column * cellPixelSize;
  const localY = y - row * cellPixelSize;
  const localIndex = localY * cellPixelSize + localX;
  const word = index.get(`${column},${row}`)?.get(Math.floor(localIndex / 32));
  return word != null && ((word >>> (localIndex % 32)) & 1) === 1;
}

function unsupportedSpatialPixels(current, other, kind, cellPixelSize, offsets) {
  const otherIndex = spatialMaskCellIndex(other, kind);
  const pixels = [];
  for (const cell of current) {
    for (const [wordIndex, encodedBits] of cell[`${kind}MaskWords`]) {
      let bits = encodedBits >>> 0;
      while (bits !== 0) {
        const lowestBit = bits & -bits;
        const bit = 31 - Math.clz32(lowestBit);
        const localIndex = wordIndex * 32 + bit;
        const x = cell.column * cellPixelSize + localIndex % cellPixelSize;
        const y = cell.row * cellPixelSize + Math.floor(localIndex / cellPixelSize);
        if (!offsets.some(({ dx, dy }) =>
          spatialMaskContains(otherIndex, x + dx, y + dy, cellPixelSize))) {
          pixels.push({ x, y, kind });
        }
        bits = (bits & (bits - 1)) >>> 0;
      }
    }
  }
  return pixels;
}

function spatialPointComponents(pixels) {
  const remaining = new Map(pixels.map((pixel) => [`${pixel.x},${pixel.y}`, pixel]));
  const components = [];
  for (const seed of pixels) {
    const seedKey = `${seed.x},${seed.y}`;
    if (!remaining.has(seedKey)) continue;
    remaining.delete(seedKey);
    const queue = [seed];
    let head = 0;
    let count = 0;
    let left = seed.x;
    let right = seed.x;
    let top = seed.y;
    let bottom = seed.y;
    while (head < queue.length) {
      const point = queue[head++];
      count += 1;
      left = Math.min(left, point.x);
      right = Math.max(right, point.x);
      top = Math.min(top, point.y);
      bottom = Math.max(bottom, point.y);
      for (let dy = -1; dy <= 1; dy += 1) {
        for (let dx = -1; dx <= 1; dx += 1) {
          if (dx === 0 && dy === 0) continue;
          const key = `${point.x + dx},${point.y + dy}`;
          const neighbor = remaining.get(key);
          if (!neighbor) continue;
          remaining.delete(key);
          queue.push(neighbor);
        }
      }
    }
    components.push({ count, left, right, top, bottom, pixels: queue });
  }
  return components;
}

function spatialPointRowIndex(pixels) {
  const rows = new Map();
  for (const { x, y } of pixels) {
    const xs = rows.get(y) ?? [];
    xs.push(x);
    rows.set(y, xs);
  }
  for (const xs of rows.values()) xs.sort((left, right) => left - right);
  return rows;
}

function lowerBound(values, target) {
  let left = 0;
  let right = values.length;
  while (left < right) {
    const middle = Math.floor((left + right) / 2);
    if (values[middle] < target) left = middle + 1;
    else right = middle;
  }
  return left;
}

function spatialPointsInBox(rows, left, top, right, bottom) {
  let count = 0;
  for (let y = top; y <= bottom; y += 1) {
    const xs = rows.get(y);
    if (!xs) continue;
    count += lowerBound(xs, right + 1) - lowerBound(xs, left);
  }
  return count;
}

function locallyUnexplainedSpatialPixels(
  unsupported,
  currentPixels,
  baselinePixels,
  improvementRadiusPixels,
) {
  const currentRows = spatialPointRowIndex(currentPixels);
  const baselineRows = spatialPointRowIndex(baselinePixels);
  return spatialPointComponents(unsupported).flatMap((component) => {
    const left = Math.floor(component.left - improvementRadiusPixels);
    const right = Math.ceil(component.right + improvementRadiusPixels);
    const top = Math.floor(component.top - improvementRadiusPixels);
    const bottom = Math.ceil(component.bottom + improvementRadiusPixels);
    const currentCount = spatialPointsInBox(currentRows, left, top, right, bottom);
    const baselineCount = spatialPointsInBox(baselineRows, left, top, right, bottom);
    return currentCount < baselineCount ? [] : component.pixels;
  });
}

function unsupportedSpatialPixelSummary(
  current,
  baseline,
  cellPixelSize,
  radiusPixels,
  improvementWindowPixels,
) {
  const offsets = [];
  const integerRadius = Math.ceil(radiusPixels);
  for (let dy = -integerRadius; dy <= integerRadius; dy += 1) {
    for (let dx = -integerRadius; dx <= integerRadius; dx += 1) {
      const distance = Math.hypot(dx, dy);
      if (distance <= radiusPixels + 1e-9) offsets.push({ dx, dy, distance });
    }
  }
  offsets.sort((left, right) => left.distance - right.distance
    || left.dy - right.dy || left.dx - right.dx);
  const unsupported = ["missing", "extra"].flatMap((kind) =>
    unsupportedSpatialPixels(current, baseline, kind, cellPixelSize, offsets));
  const currentPixels = ["missing", "extra"].flatMap((kind) =>
    unsupportedSpatialPixels(current, [], kind, cellPixelSize, []));
  const baselinePixels = ["missing", "extra"].flatMap((kind) =>
    unsupportedSpatialPixels(baseline, [], kind, cellPixelSize, []));
  const unexplained = locallyUnexplainedSpatialPixels(
    unsupported,
    currentPixels,
    baselinePixels,
    improvementWindowPixels,
  );
  return Object.fromEntries(["missing", "extra"].map((kind) => {
    const pixels = unexplained.filter((pixel) => pixel.kind === kind);
    return [kind, {
      count: pixels.length,
      examples: pixels.slice(0, 8).map(({ x, y }) => ({ x, y })),
    }];
  }));
}

function spatialFloorRegressionReasons(currentFloor, baselineFloor, prefix, tolerances) {
  if (!baselineFloor) return [];
  if (!currentFloor) {
    return [{
      metric: prefix,
      direction: "present",
      before: "spatial-floor",
      after: null,
      tolerance: 0,
    }];
  }
  let baseline;
  let current;
  try {
    baseline = decodeSpatialFloor(baselineFloor);
    current = decodeSpatialFloor(currentFloor);
    if (
      currentFloor.analysisScale !== baselineFloor.analysisScale
      || currentFloor.stride !== baselineFloor.stride
    ) throw new Error("incompatible spatial floor");
  } catch {
    return [{
      metric: prefix,
      direction: "valid",
      before: true,
      after: false,
      tolerance: 0,
    }];
  }
  const scale = baselineFloor.analysisScale;
  const reasons = [];
  const cellPixelSize = spatialCellPixelSize(scale, baselineFloor.stride);
  const unsupportedByKind = unsupportedSpatialPixelSummary(
    current,
    baseline,
    cellPixelSize,
    tolerances.center * scale,
    tolerances.improvementWindow * scale,
  );
  for (const kind of ["missing", "extra"]) {
    const unsupported = unsupportedByKind[kind];
    const unsupportedArea = unsupported.count / (scale * scale);
    if (unsupportedArea <= tolerances.defectArea) continue;
    reasons.push({
      metric: `${prefix}.${kind}UnsupportedArea`,
      direction: "lower",
      before: 0,
      after: unsupportedArea,
      tolerance: tolerances.defectArea,
      examples: unsupported.examples,
    });
  }
  return reasons;
}

function finiteMetricAt(entry, metricPath) {
  let value = entry;
  for (const segment of metricPath.split(".")) value = value?.[segment];
  return Number.isFinite(value) ? value : null;
}

function metricRegression(metric, before, after) {
  if (metric.direction === "higher") return before - after > metric.tolerance;
  return after - before > metric.tolerance;
}

function setMetricAt(entry, metricPath, value) {
  const segments = metricPath.split(".");
  let target = entry;
  for (const segment of segments.slice(0, -1)) {
    target[segment] ??= {};
    target = target[segment];
  }
  target[segments.at(-1)] = value;
}

export function visualCaseMetricContractErrors(entry, options = {}) {
  if (!["pass", "fail", "error", "unavailable"].includes(entry?.status)) {
    return ["missing or invalid status"];
  }
  if (!["pass", "fail"].includes(entry.status)) return [];
  const settings = { ...DEFAULTS, ...options };
  const errors = [];
  for (const metric of CONTINUOUS_REGRESSION_METRICS) {
    if (finiteMetricAt(entry, metric.path) === null) {
      errors.push(`missing ${metric.path}`);
    }
  }
  for (const [label, floor, scale, stride, totals, domain] of [
    ["local.spatialFloor", entry.local?.spatialFloor,
      settings.analysisScale, settings.localStride, entry.totals, entry.domain],
    ["detail.local.spatialFloor", entry.detail?.local?.spatialFloor,
      settings.detailAnalysisScale, settings.detailLocalStride,
      entry.detail?.totals, entry.detail?.domain],
  ]) {
    if (!floor) {
      errors.push(`missing ${label}`);
      continue;
    }
    if (!spatialFloorIsCanonical(floor, scale, stride)) {
      errors.push(`invalid ${label}`);
      continue;
    }
    if (
      floor.missingPixels !== totals?.missingInk
      || floor.extraPixels !== totals?.extraInk
    ) {
      errors.push(`${label} pixel totals do not match the analyzed layer`);
    }
    if (![domain?.left, domain?.top, domain?.right, domain?.bottom].every(Number.isFinite)) {
      errors.push(`missing ${label} domain`);
      continue;
    }
    const cells = decodeSpatialFloor(floor);
    const minimumColumn = Math.floor(domain.left / stride);
    const maximumColumn = Math.floor((domain.right - 1 / scale) / stride);
    const minimumRow = Math.floor(domain.top / stride);
    const maximumRow = Math.floor((domain.bottom - 1 / scale) / stride);
    if (cells.some((cell) => cell.column < minimumColumn || cell.column > maximumColumn
      || cell.row < minimumRow || cell.row > maximumRow)) {
      errors.push(`${label} contains a cell outside the analyzed domain`);
    }
  }
  return errors;
}

export function protectedVisualCase(entry) {
  const protectedCase = {
    relativeCdxml: normalizedCasePath(entry.relativeCdxml),
    status: entry.status,
    artifactHashes: { reference: entry.artifactHashes?.reference ?? null },
  };
  if (entry.status !== "fail") return protectedCase;
  if (entry.alignment != null) {
    protectedCase.regressionAlignment = {
      algorithm: entry.alignment.algorithm,
      scale: entry.alignment.scale,
      dx: entry.alignment.dx,
      dy: entry.alignment.dy,
      ...(Number.isFinite(entry.alignment.chemsemaWidth)
        ? { chemsemaWidth: entry.alignment.chemsemaWidth }
        : {}),
      ...(Number.isFinite(entry.alignment.chemsemaHeight)
        ? { chemsemaHeight: entry.alignment.chemsemaHeight }
        : {}),
      vectorFrame: {
        chemsemaViewBox: entry.alignment.vectorFrame?.chemsemaViewBox,
      },
    };
  }
  protectedCase.analysisLayers = {
    coarse: true,
    detail: entry.detail != null,
  };
  protectedCase.reasons = [...new Set(entry.reasons ?? [])].sort();
  for (const metric of CONTINUOUS_REGRESSION_METRICS) {
    const value = finiteMetricAt(entry, metric.path);
    if (value !== null) setMetricAt(protectedCase, metric.path, value);
  }
  protectedCase.local ??= {};
  protectedCase.local.spatialFloor = entry.local?.spatialFloor ?? null;
  if (entry.totals != null) protectedCase.totals = entry.totals;
  if (entry.domain != null) protectedCase.domain = entry.domain;
  if (entry.detail != null) {
    protectedCase.detail ??= {};
    protectedCase.detail.local ??= {};
    protectedCase.detail.local.spatialFloor =
      entry.detail.local?.spatialFloor ?? null;
    if (entry.detail.totals != null) protectedCase.detail.totals = entry.detail.totals;
    if (entry.detail.domain != null) protectedCase.detail.domain = entry.detail.domain;
  }
  return protectedCase;
}

export function protectedVisualCases(cases) {
  return cases.map(protectedVisualCase).sort((left, right) =>
    left.relativeCdxml.localeCompare(right.relativeCdxml));
}

export function protectedVisualFloorOracleErrors(passFloor, currentReferenceHashes) {
  const floorCases = new Map(
    (passFloor?.protectedCases ?? []).map((entry) => [
      normalizedCasePath(entry.relativeCdxml),
      entry,
    ]),
  );
  const errors = [];
  for (const [relativeCdxml, referenceHash] of currentReferenceHashes) {
    const floorCase = floorCases.get(normalizedCasePath(relativeCdxml));
    if (!floorCase) {
      errors.push(`pass floor is missing current case ${relativeCdxml}`);
      break;
    }
    if (floorCase.artifactHashes?.reference !== referenceHash) {
      errors.push(`protected ChemDraw oracle changed for ${relativeCdxml}`);
      break;
    }
  }
  return errors;
}

export function regressionBaselineCasesForGate({
  evaluatePassFloor,
  passFloorDefinitionErrors,
  strictPassFloor,
  sameGateDefinition,
  baselineReport,
}) {
  const sourceCases = evaluatePassFloor && !passFloorDefinitionErrors.length
    ? strictPassFloor?.protectedCases ?? []
    : sameGateDefinition
      ? baselineReport?.cases ?? []
      : [];
  return new Map(sourceCases.map((entry) => [
    normalizedCasePath(entry.relativeCdxml),
    entry,
  ]));
}

export function classifyContinuousBaselineRegressions(cases, baselineCases) {
  return cases.flatMap((entry) => {
    const relativeCdxml = normalizedCasePath(entry.relativeCdxml);
    const baseline = baselineCases.get(relativeCdxml);
    if (!baseline) return [];
    const reasons = [];
    const baselineComparable = ["pass", "fail"].includes(baseline.status);
    const currentComparable = ["pass", "fail"].includes(entry.status);
    if (baselineComparable && !currentComparable) {
      reasons.push({
        metric: "status",
        direction: "comparable",
        before: baseline.status,
        after: entry.status,
        tolerance: 0,
      });
    }
    // The pass floor and discrete baseline delta protect every green case.
    // Continuous metrics exist for the different problem of preventing an
    // already-red case from accumulating new or larger defects while it is
    // still red. Applying these non-monotonic registered-image statistics to
    // pass->pass or fail->pass transitions can label an objectively improved
    // glyph position as a regression after the global translation is
    // recomputed.
    if (baseline.status !== "fail" || entry.status !== "fail") {
      return reasons.length ? [{
        relativeCdxml,
        beforeStatus: baseline.status,
        afterStatus: entry.status,
        reasons,
      }] : [];
    }
    // Current pass/fail classification uses the current candidate's own best
    // registration. Continuous comparison is a different question: compare
    // the changed candidate at the historical registration so that removing
    // an outlying wrong object cannot translate every residual mismatch and
    // manufacture a whole-image regression.
    const comparison = entry.regressionFloor ?? entry;
    const hasSpatialFloor = baseline.local?.spatialFloor
      && comparison.local?.spatialFloor;
    if (!hasSpatialFloor) {
      const previousReasons = new Set(baseline.reasons ?? []);
      const newGateReasons = [...new Set(comparison.reasons ?? [])]
        .filter((reason) => !previousReasons.has(reason))
        .sort();
      for (const reason of newGateReasons) {
        reasons.push({
          metric: `reason:${reason}`,
          direction: "absent",
          before: false,
          after: true,
          tolerance: 0,
        });
      }
      for (const metric of CONTINUOUS_REGRESSION_METRICS) {
        const before = finiteMetricAt(baseline, metric.path);
        const after = finiteMetricAt(comparison, metric.path);
        if (before === null) continue;
        if (after === null) {
          reasons.push({
            metric: metric.path,
            direction: "present",
            before,
            after: null,
            tolerance: 0,
          });
          continue;
        }
        if (!metricRegression(metric, before, after)) continue;
        reasons.push({
          metric: metric.path,
          direction: metric.direction,
          before,
          after,
          tolerance: metric.tolerance,
        });
      }
    }
    reasons.push(...spatialFloorRegressionReasons(
      comparison.local?.spatialFloor,
      baseline.local?.spatialFloor,
      "local.spatialFloor",
      LOCAL_WINDOW_REGRESSION_TOLERANCES.coarse,
    ));
    reasons.push(...spatialFloorRegressionReasons(
      comparison.detail?.local?.spatialFloor,
      baseline.detail?.local?.spatialFloor,
      "detail.local.spatialFloor",
      LOCAL_WINDOW_REGRESSION_TOLERANCES.detail,
    ));
    // A gain in one metric cannot cancel deterioration in another. Registered
    // image metrics are not additive, and allowing a trade-off here let a new
    // local defect hide behind an unrelated coverage improvement. A deliberate
    // trade-off must be reviewed and promoted into the committed floor.
    return reasons.length ? [{
      relativeCdxml,
      beforeStatus: baseline.status,
      afterStatus: entry.status,
      reasons,
    }] : [];
  });
}

export function classifyPassFloorRegressions(cases, passFloor) {
  const currentCases = new Map(
    cases.map((entry) => [normalizedCasePath(entry.relativeCdxml), entry]),
  );
  return (passFloor?.protectedPasses ?? []).flatMap((relativeCdxml) => {
    const normalizedPath = normalizedCasePath(relativeCdxml);
    const after = currentCases.get(normalizedPath)?.status ?? "missing";
    return after === "pass"
      ? []
      : [{ relativeCdxml: normalizedPath, before: "protected-pass", after }];
  });
}

export function shouldEvaluateOriginal338PassFloor(cohortSelection, selectedCount) {
  return cohortSelection?.name === "original-338"
    && cohortSelection.expected === 338
    && cohortSelection.selected === 338
    && selectedCount === 338;
}

export function selectVisualGateCohort(items, ledger, cohort) {
  const selectedPaths = new Set(
    ledger.cases
      .filter((entry) => entry.cohort === cohort)
      .map((entry) => entry.relativeCdxml.replaceAll("\\", "/")),
  );
  const selectedItems = items.filter((item) =>
    selectedPaths.has(item.relativeCdxml.replaceAll("\\", "/")));
  const foundPaths = new Set(
    selectedItems.map((item) => item.relativeCdxml.replaceAll("\\", "/")),
  );
  return {
    items: selectedItems,
    expected: selectedPaths.size,
    missingPaths: [...selectedPaths].filter((relativePath) => !foundPaths.has(relativePath)),
  };
}

async function oracleIsUnavailable(filePath) {
  if (path.extname(filePath).toLowerCase() !== ".svg") return false;
  const source = await fs.readFile(filePath, "utf8");
  return source.includes("ChemDraw 无法渲染");
}

export async function analyzeCandidateViewport(page, candidateDataUrl, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return page.evaluate(async ({ candidateDataUrl, settings }) => {
    if (!candidateDataUrl.startsWith("data:image/svg+xml")) {
      return { applicable: false, reason: "candidate-is-not-svg" };
    }
    const requestedScale = settings.candidateViewportAnalysisScale;
    if (!(Number.isFinite(requestedScale) && requestedScale > 0)) {
      throw new Error("candidate viewport analysis scale must be positive");
    }
    if (!(Number.isFinite(settings.maxCandidateViewportPixels)
      && settings.maxCandidateViewportPixels > 0)) {
      throw new Error("candidate viewport pixel budget must be positive");
    }
    function numericLength(value) {
      const match = /^\s*([-+0-9.eE]+)(?:px)?\s*$/.exec(value ?? "");
      if (!match) return null;
      const number = Number(match[1]);
      return Number.isFinite(number) && number > 0 ? number : null;
    }
    const source = await (await fetch(candidateDataUrl)).text();
    const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
    const svg = parsed.documentElement;
    if (svg.localName !== "svg" || parsed.querySelector("parsererror")) {
      throw new Error("candidate viewport gate could not parse the SVG");
    }
    const viewBox = (svg.getAttribute("viewBox") ?? "")
      .trim()
      .split(/[\s,]+/)
      .map(Number);
    const validViewBox = viewBox.length === 4
      && viewBox.every(Number.isFinite)
      && viewBox[2] > 0
      && viewBox[3] > 0;
    const width = numericLength(svg.getAttribute("width"))
      ?? (validViewBox ? viewBox[2] : null);
    const height = numericLength(svg.getAttribute("height"))
      ?? (validViewBox ? viewBox[3] : null);
    if (!(width > 0 && height > 0)) {
      throw new Error("candidate viewport gate found no positive SVG viewport");
    }
    const scale = Math.min(
      requestedScale,
      Math.sqrt(settings.maxCandidateViewportPixels / (width * height)),
    );
    const pixelWidth = Math.max(1, Math.ceil(width * scale));
    const pixelHeight = Math.max(1, Math.ceil(height * scale));
    svg.setAttribute("width", `${pixelWidth}px`);
    svg.setAttribute("height", `${pixelHeight}px`);
    const normalizedSource = `data:image/svg+xml;charset=utf-8,${
      encodeURIComponent(new XMLSerializer().serializeToString(svg))
    }`;
    const image = new Image();
    image.decoding = "sync";
    image.src = normalizedSource;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = pixelWidth;
    canvas.height = pixelHeight;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, pixelWidth, pixelHeight);
    context.drawImage(image, 0, 0, pixelWidth, pixelHeight);
    const pixels = context.getImageData(0, 0, pixelWidth, pixelHeight).data;
    let inkPixels = 0;
    let left = pixelWidth;
    let top = pixelHeight;
    let right = -1;
    let bottom = -1;
    for (let y = 0; y < pixelHeight; y += 1) {
      for (let x = 0; x < pixelWidth; x += 1) {
        const offset = (y * pixelWidth + x) * 4;
        if (pixels[offset] + pixels[offset + 1] + pixels[offset + 2] >= 740) continue;
        inkPixels += 1;
        left = Math.min(left, x);
        top = Math.min(top, y);
        right = Math.max(right, x);
        bottom = Math.max(bottom, y);
      }
    }
    if (!inkPixels) {
      return {
        applicable: true,
        width,
        height,
        requestedAnalysisScale: requestedScale,
        effectiveAnalysisScale: scale,
        inkPixels: 0,
        margins: null,
        minimumMargin: null,
      };
    }
    const margins = {
      left: left * width / pixelWidth,
      top: top * height / pixelHeight,
      right: (pixelWidth - 1 - right) * width / pixelWidth,
      bottom: (pixelHeight - 1 - bottom) * height / pixelHeight,
    };
    return {
      applicable: true,
      width,
      height,
      requestedAnalysisScale: requestedScale,
      effectiveAnalysisScale: scale,
      inkPixels,
      margins,
      minimumMargin: Math.min(...Object.values(margins)),
    };
  }, { candidateDataUrl, settings });
}

export function candidateViewportGateReasons(candidateViewport, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  if (
    !candidateViewport?.applicable
    || !candidateViewport.inkPixels
    || !Number.isFinite(candidateViewport.minimumMargin)
  ) return [];
  return candidateViewport.minimumMargin + 1e-9 < settings.minCandidateViewportInkMargin
    ? ["candidate-viewport-ink-margin"]
    : [];
}

export function applyCandidateViewportGate(metrics, candidateViewport, options = {}) {
  const viewportReasons = candidateViewportGateReasons(candidateViewport, options);
  return {
    ...metrics,
    passed: metrics.passed && viewportReasons.length === 0,
    reasons: [...new Set([...(metrics.reasons ?? []), ...viewportReasons])],
    candidateViewport,
  };
}

export async function analyzeAlignedImages(page, referenceDataUrl, candidateDataUrl, alignment, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return page.evaluate(async ({ referenceDataUrl, candidateDataUrl, alignment, settings }) => {
    function numericLength(value) {
      const match = /^\s*([-+0-9.eE]+)(?:px)?\s*$/.exec(value ?? "");
      if (!match) return null;
      const number = Number(match[1]);
      return Number.isFinite(number) && number > 0 ? number : null;
    }

    function viewBoxSize(svg) {
      const values = (svg.getAttribute("viewBox") ?? "")
        .trim()
        .split(/[\s,]+/)
        .map(Number);
      return values.length === 4
        && values.every(Number.isFinite)
        && values[2] > 0
        && values[3] > 0
        ? { x: values[0], y: values[1], width: values[2], height: values[3] }
        : null;
    }

    function expandedViewBox(viewBox, padding) {
      return {
        x: viewBox.x - padding,
        y: viewBox.y - padding,
        width: viewBox.width + padding * 2,
        height: viewBox.height + padding * 2,
      };
    }

    function containsViewBox(container, content) {
      const epsilon = 1e-9;
      return content.x >= container.x - epsilon
        && content.y >= container.y - epsilon
        && content.x + content.width <= container.x + container.width + epsilon
        && content.y + content.height <= container.y + container.height + epsilon;
    }

    async function prepareImage(src, imageScale, dx, dy, rasterOptions = {}) {
      if (!(Number.isFinite(imageScale) && imageScale > 0)) {
        throw new Error("visual gate alignment scale must be positive");
      }
      if (!src.startsWith("data:image/svg+xml")) {
        const image = await loadImage(src);
        return {
          image,
          width: image.naturalWidth,
          height: image.naturalHeight,
          pixelAligned: false,
        };
      }
      const source = await (await fetch(src)).text();
      const document = new DOMParser().parseFromString(source, "image/svg+xml");
      const svg = document.documentElement;
      if (svg.localName !== "svg" || document.querySelector("parsererror")) {
        throw new Error("visual gate could not parse an SVG input");
      }
      const viewBox = viewBoxSize(svg);
      const width = numericLength(svg.getAttribute("width")) ?? viewBox?.width;
      const height = numericLength(svg.getAttribute("height")) ?? viewBox?.height;
      if (!(width > 0 && height > 0)) {
        throw new Error("visual gate SVG input has no positive viewport");
      }
      const sourceViewBox = viewBox ?? { x: 0, y: 0, width, height };
      const requestedRasterViewBox = rasterOptions.viewBox ?? expandedViewBox(
        sourceViewBox,
        rasterOptions.padding ?? 0,
      );
      if (
        ![requestedRasterViewBox.x, requestedRasterViewBox.y,
          requestedRasterViewBox.width, requestedRasterViewBox.height].every(Number.isFinite)
        || requestedRasterViewBox.width <= 0
        || requestedRasterViewBox.height <= 0
      ) throw new Error("visual gate candidate raster viewport is invalid");
      if (!containsViewBox(requestedRasterViewBox, sourceViewBox)) {
        throw new Error(
          "visual gate candidate exceeds its historical regression raster viewport",
        );
      }
      const pixelScale = settings.analysisScale;
      const phase = (value) => {
        const fractional = value - Math.floor(value);
        return fractional < 0 ? fractional + 1 : fractional;
      };
      const pixelsPerSourceX = width * imageScale * pixelScale / sourceViewBox.width;
      const pixelsPerSourceY = height * imageScale * pixelScale / sourceViewBox.height;
      const sourceToViewportX = width / sourceViewBox.width;
      const sourceToViewportY = height / sourceViewBox.height;
      const rasterLeft = dx
        + (requestedRasterViewBox.x - sourceViewBox.x) * sourceToViewportX * imageScale;
      const rasterTop = dy
        + (requestedRasterViewBox.y - sourceViewBox.y) * sourceToViewportY * imageScale;
      const phaseX = phase(rasterLeft * pixelScale);
      const phaseY = phase(rasterTop * pixelScale);
      const rasterWidth = Math.max(
        1,
        Math.ceil(phaseX + requestedRasterViewBox.width * pixelsPerSourceX),
      );
      const rasterHeight = Math.max(
        1,
        Math.ceil(phaseY + requestedRasterViewBox.height * pixelsPerSourceY),
      );
      svg.setAttribute("width", `${rasterWidth}px`);
      svg.setAttribute("height", `${rasterHeight}px`);
      svg.setAttribute("preserveAspectRatio", "none");
      svg.setAttribute("viewBox", [
        requestedRasterViewBox.x - phaseX / pixelsPerSourceX,
        requestedRasterViewBox.y - phaseY / pixelsPerSourceY,
        rasterWidth / pixelsPerSourceX,
        rasterHeight / pixelsPerSourceY,
      ].join(" "));
      const normalizedSource = `data:image/svg+xml;charset=utf-8,${
        encodeURIComponent(new XMLSerializer().serializeToString(svg))
      }`;
      return {
        image: await loadImage(normalizedSource),
        width,
        height,
        pixelAligned: true,
        originPixelX: Math.floor(rasterLeft * pixelScale),
        originPixelY: Math.floor(rasterTop * pixelScale),
      };
    }

    async function loadImage(src) {
      const image = new Image();
      image.decoding = "sync";
      image.src = src;
      await image.decode();
      return image;
    }

    function maskFromCanvas(canvas, threshold = 740) {
      const pixels = canvas.getContext("2d", { willReadFrequently: true })
        .getImageData(0, 0, canvas.width, canvas.height).data;
      const mask = new Uint8Array(canvas.width * canvas.height);
      let ink = 0;
      for (let index = 0; index < mask.length; index += 1) {
        const offset = index * 4;
        if (pixels[offset] + pixels[offset + 1] + pixels[offset + 2] < threshold) {
          mask[index] = 1;
          ink += 1;
        }
      }
      return { mask, ink };
    }

    function dilate(mask, width, height, radius) {
      const output = new Uint8Array(mask.length);
      const offsets = [];
      for (let dy = -radius; dy <= radius; dy += 1) {
        for (let dx = -radius; dx <= radius; dx += 1) {
          if (dx * dx + dy * dy <= radius * radius) offsets.push([dx, dy]);
        }
      }
      for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
          if (!mask[y * width + x]) continue;
          for (const [dx, dy] of offsets) {
            const nextX = x + dx;
            const nextY = y + dy;
            if (nextX >= 0 && nextX < width && nextY >= 0 && nextY < height) {
              output[nextY * width + nextX] = 1;
            }
          }
        }
      }
      return output;
    }

    function differenceMask(source, dilatedOther) {
      const output = new Uint8Array(source.length);
      for (let index = 0; index < source.length; index += 1) {
        output[index] = source[index] && !dilatedOther[index] ? 1 : 0;
      }
      return output;
    }

    function components(mask, width, height, originX, originY, pixelScale, core) {
      const seen = new Uint8Array(mask.length);
      const queue = new Int32Array(mask.length);
      const result = [];
      for (let start = 0; start < mask.length; start += 1) {
        if (!mask[start] || seen[start]) continue;
        let head = 0;
        let tail = 0;
        queue[tail++] = start;
        seen[start] = 1;
        let area = 0;
        let left = width;
        let top = height;
        let right = -1;
        let bottom = -1;
        while (head < tail) {
          const index = queue[head++];
          const y = Math.floor(index / width);
          const x = index - y * width;
          area += 1;
          left = Math.min(left, x);
          top = Math.min(top, y);
          right = Math.max(right, x);
          bottom = Math.max(bottom, y);
          for (let dy = -1; dy <= 1; dy += 1) {
            for (let dx = -1; dx <= 1; dx += 1) {
              if (dx === 0 && dy === 0) continue;
              const nextX = x + dx;
              const nextY = y + dy;
              if (nextX < 0 || nextX >= width || nextY < 0 || nextY >= height) continue;
              const next = nextY * width + nextX;
              if (!mask[next] || seen[next]) continue;
              seen[next] = 1;
              queue[tail++] = next;
            }
          }
        }
        const centerX = originX + (left + right + 1) / (2 * pixelScale);
        const centerY = originY + (top + bottom + 1) / (2 * pixelScale);
        if (
          centerX < core.x || centerX >= core.x + core.width
          || centerY < core.y || centerY >= core.y + core.height
        ) continue;
        const widthRef = (right - left + 1) / pixelScale;
        const heightRef = (bottom - top + 1) / pixelScale;
        result.push({
          area: area / (pixelScale * pixelScale),
          span: Math.hypot(widthRef, heightRef),
          box: {
            x: originX + left / pixelScale,
            y: originY + top / pixelScale,
            width: widthRef,
            height: heightRef,
          },
        });
      }
      return result;
    }

    function renderTile(referenceFrame, candidateFrame, tile) {
      const pixelScale = settings.analysisScale;
      const width = Math.max(1, Math.ceil(tile.width * pixelScale));
      const height = Math.max(1, Math.ceil(tile.height * pixelScale));
      function render(frame, imageScale, dx, dy) {
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        context.fillStyle = "#ffffff";
        context.fillRect(0, 0, width, height);
        if (frame.pixelAligned) {
          context.drawImage(
            frame.image,
            frame.originPixelX - tile.x * pixelScale,
            frame.originPixelY - tile.y * pixelScale,
          );
        } else {
          context.drawImage(
            frame.image,
            (dx - tile.x) * pixelScale,
            (dy - tile.y) * pixelScale,
            frame.width * imageScale * pixelScale,
            frame.height * imageScale * pixelScale,
          );
        }
        return canvas;
      }
      return {
        reference: render(referenceFrame, 1, 0, 0),
        candidate: render(candidateFrame, alignment.scale, alignment.dx, alignment.dy),
      };
    }

    const [referenceFrame, candidateFrame] = await Promise.all([
      prepareImage(referenceDataUrl, 1, 0, 0),
      prepareImage(candidateDataUrl, alignment.scale, alignment.dx, alignment.dy, {
        padding: settings.candidateRegressionRasterPadding,
        viewBox: settings.candidateRasterViewBox,
      }),
    ]);
    const referenceWidth = referenceFrame.width;
    const referenceHeight = referenceFrame.height;
    const candidateWidth = candidateFrame.width;
    const candidateHeight = candidateFrame.height;

    const domain = {
      left: Math.floor(Math.min(0, alignment.dx)),
      top: Math.floor(Math.min(0, alignment.dy)),
      right: Math.ceil(Math.max(
        referenceWidth,
        alignment.dx + candidateWidth * alignment.scale,
      )),
      bottom: Math.ceil(Math.max(
        referenceHeight,
        alignment.dy + candidateHeight * alignment.scale,
      )),
    };
    const radius = Math.max(0, Math.ceil(settings.tolerance * settings.analysisScale));
    const totals = {
      referenceInk: 0,
      candidateInk: 0,
      missingInk: 0,
      extraInk: 0,
      tileCount: 0,
      inkTileCount: 0,
    };
    const local = {
      referenceCoverage: 1,
      candidateCoverage: 1,
      referenceBox: null,
      candidateBox: null,
      windowCount: 0,
      spatialCells: [],
    };
    const spatialCellPixelSize = settings.localStride * settings.analysisScale;
    if (!Number.isInteger(spatialCellPixelSize) || spatialCellPixelSize <= 0) {
      throw new Error("spatial analysis requires an integral positive pixel lattice");
    }
    const spatialCellsByKey = new Map();
    function recordSpatialPixel(kind, globalPixelX, globalPixelY) {
      const column = Math.floor(globalPixelX / spatialCellPixelSize);
      const row = Math.floor(globalPixelY / spatialCellPixelSize);
      const localX = globalPixelX - column * spatialCellPixelSize;
      const localY = globalPixelY - row * spatialCellPixelSize;
      const localIndex = localY * spatialCellPixelSize + localX;
      const wordIndex = Math.floor(localIndex / 32);
      const bit = localIndex % 32;
      const key = `${column},${row}`;
      let cell = spatialCellsByKey.get(key);
      if (!cell) {
        cell = {
          column,
          row,
          missingMaskWords: new Map(),
          extraMaskWords: new Map(),
        };
        spatialCellsByKey.set(key, cell);
      }
      const words = cell[`${kind}MaskWords`];
      words.set(wordIndex, ((words.get(wordIndex) ?? 0) | (1 << bit)) >>> 0);
    }
    let largestMissing = { area: 0, span: 0, box: null };
    let largestExtra = { area: 0, span: 0, box: null };
    const topDefects = [];
    let compactDefectCount = 0;
    const inkComponents = { reference: [], candidate: [] };

    function recordDefects(kind, entries) {
      for (const entry of entries) {
        const shortSide = Math.min(entry.box.width, entry.box.height);
        const longSide = Math.max(entry.box.width, entry.box.height);
        if (
          entry.area >= 0.5
          && entry.span >= 2.5
          && entry.span <= 12
          && shortSide > 0
          && longSide / shortSide <= 4
        ) compactDefectCount += 1;
        const areaScore = settings.maxDefectArea === 0
          ? (entry.area === 0 ? 0 : Number.POSITIVE_INFINITY)
          : entry.area / settings.maxDefectArea;
        const spanScore = settings.maxDefectSpan === 0
          ? (entry.span === 0 ? 0 : Number.POSITIVE_INFINITY)
          : entry.span / settings.maxDefectSpan;
        const scored = {
          ...entry,
          kind,
          score: Math.max(areaScore, spanScore),
        };
        topDefects.push(scored);
        topDefects.sort((left, right) => right.score - left.score);
        if (topDefects.length > 12) topDefects.length = 12;
        const current = kind === "missing" ? largestMissing : largestExtra;
        if (entry.area > current.area) current.area = entry.area;
        if (entry.span > current.span) current.span = entry.span;
        if (!current.box || scored.score > current.score) {
          current.box = entry.box;
          current.score = scored.score;
        }
      }
    }

    // Tile and local-window lattices are anchored in ChemDraw reference
    // coordinates, not at the union viewport edge. An SVG viewBox is merely a
    // crop; moving that crop must not move every sampling window or split the
    // same connected stroke at different tile boundaries.
    const gridLeft = Math.floor(domain.left / settings.tileSize) * settings.tileSize;
    const gridTop = Math.floor(domain.top / settings.tileSize) * settings.tileSize;
    const gridRight = Math.ceil(domain.right / settings.tileSize) * settings.tileSize;
    const gridBottom = Math.ceil(domain.bottom / settings.tileSize) * settings.tileSize;
    for (let coreY = gridTop; coreY < gridBottom; coreY += settings.tileSize) {
      for (let coreX = gridLeft; coreX < gridRight; coreX += settings.tileSize) {
        const core = {
          x: Math.max(coreX, domain.left),
          y: Math.max(coreY, domain.top),
          width: Math.min(coreX + settings.tileSize, domain.right)
            - Math.max(coreX, domain.left),
          height: Math.min(coreY + settings.tileSize, domain.bottom)
            - Math.max(coreY, domain.top),
        };
        if (core.width <= 0 || core.height <= 0) continue;
        const tile = {
          x: core.x - settings.halo,
          y: core.y - settings.halo,
          width: core.width + settings.halo * 2,
          height: core.height + settings.halo * 2,
        };
        const rendered = renderTile(referenceFrame, candidateFrame, tile);
        const reference = maskFromCanvas(rendered.reference);
        const candidate = maskFromCanvas(rendered.candidate);
        const candidateDilated = dilate(candidate.mask, rendered.reference.width, rendered.reference.height, radius);
        const referenceDilated = dilate(reference.mask, rendered.reference.width, rendered.reference.height, radius);
        const missing = differenceMask(reference.mask, candidateDilated);
        const extra = differenceMask(candidate.mask, referenceDilated);
        const coreLeft = Math.round(settings.halo * settings.analysisScale);
        const coreTop = coreLeft;
        const coreRight = Math.min(
          rendered.reference.width,
          coreLeft + Math.ceil(core.width * settings.analysisScale),
        );
        const coreBottom = Math.min(
          rendered.reference.height,
          coreTop + Math.ceil(core.height * settings.analysisScale),
        );
        let tileHasInk = false;
        for (let y = coreTop; y < coreBottom; y += 1) {
          for (let x = coreLeft; x < coreRight; x += 1) {
            const index = y * rendered.reference.width + x;
            totals.referenceInk += reference.mask[index];
            totals.candidateInk += candidate.mask[index];
            totals.missingInk += missing[index];
            totals.extraInk += extra[index];
            const globalPixelX = Math.round(core.x * settings.analysisScale)
              + x - coreLeft;
            const globalPixelY = Math.round(core.y * settings.analysisScale)
              + y - coreTop;
            if (missing[index]) recordSpatialPixel("missing", globalPixelX, globalPixelY);
            if (extra[index]) recordSpatialPixel("extra", globalPixelX, globalPixelY);
            if (reference.mask[index] || candidate.mask[index]) tileHasInk = true;
          }
        }
        totals.tileCount += 1;
        if (tileHasInk) totals.inkTileCount += 1;
        const firstWindowColumn = Math.ceil(
          (core.x - settings.localStride / 2) / settings.localStride,
        );
        const lastWindowColumn = Math.floor(
          (core.x + core.width - settings.localStride / 2 - 1e-9) / settings.localStride,
        );
        const firstWindowRow = Math.ceil(
          (core.y - settings.localStride / 2) / settings.localStride,
        );
        const lastWindowRow = Math.floor(
          (core.y + core.height - settings.localStride / 2 - 1e-9) / settings.localStride,
        );
        for (let windowRow = firstWindowRow; windowRow <= lastWindowRow; windowRow += 1) {
          for (let windowColumn = firstWindowColumn; windowColumn <= lastWindowColumn; windowColumn += 1) {
            const centerX = settings.localStride / 2 + windowColumn * settings.localStride;
            const centerY = settings.localStride / 2 + windowRow * settings.localStride;
            const windowBox = {
              x: centerX - settings.localWindow / 2,
              y: centerY - settings.localWindow / 2,
              width: settings.localWindow,
              height: settings.localWindow,
            };
            const left = Math.max(0, Math.floor((windowBox.x - tile.x) * settings.analysisScale));
            const top = Math.max(0, Math.floor((windowBox.y - tile.y) * settings.analysisScale));
            const right = Math.min(
              rendered.reference.width,
              Math.ceil((windowBox.x + windowBox.width - tile.x) * settings.analysisScale),
            );
            const bottom = Math.min(
              rendered.reference.height,
              Math.ceil((windowBox.y + windowBox.height - tile.y) * settings.analysisScale),
            );
            let referenceInk = 0;
            let candidateInk = 0;
            let missingInk = 0;
            let extraInk = 0;
            for (let y = top; y < bottom; y += 1) {
              for (let x = left; x < right; x += 1) {
                const index = y * rendered.reference.width + x;
                referenceInk += reference.mask[index];
                candidateInk += candidate.mask[index];
                missingInk += missing[index];
                extraInk += extra[index];
              }
            }
            const minimumInk = settings.minimumWindowInk
              * settings.analysisScale * settings.analysisScale;
            let referenceCoverage = null;
            let candidateCoverage = null;
            if (referenceInk >= minimumInk) {
              referenceCoverage = 1 - missingInk / referenceInk;
              if (referenceCoverage < local.referenceCoverage) {
                local.referenceCoverage = referenceCoverage;
                local.referenceBox = windowBox;
              }
            }
            if (candidateInk >= minimumInk) {
              candidateCoverage = 1 - extraInk / candidateInk;
              if (candidateCoverage < local.candidateCoverage) {
                local.candidateCoverage = candidateCoverage;
                local.candidateBox = windowBox;
              }
            }
            if (referenceInk >= minimumInk || candidateInk >= minimumInk) {
              local.windowCount += 1;
            }
          }
        }
        if (!reference.ink && !candidate.ink) continue;
        for (const [kind, mask] of [["reference", reference.mask], ["candidate", candidate.mask]]) {
          for (const entry of components(
            mask,
            rendered.reference.width,
            rendered.reference.height,
            tile.x,
            tile.y,
            settings.analysisScale,
            core,
          )) {
            if (entry.area >= 0.25) inkComponents[kind].push(entry);
          }
        }
        recordDefects(
          "missing",
          components(
            missing,
            rendered.reference.width,
            rendered.reference.height,
            tile.x,
            tile.y,
            settings.analysisScale,
            core,
          ),
        );
        recordDefects(
          "extra",
          components(
            extra,
            rendered.reference.width,
            rendered.reference.height,
            tile.x,
            tile.y,
            settings.analysisScale,
            core,
          ),
        );
      }
    }

    const referenceCoverage = totals.referenceInk === 0
      ? (totals.candidateInk === 0 ? 1 : 0)
      : 1 - totals.missingInk / totals.referenceInk;
    const candidateCoverage = totals.candidateInk === 0
      ? (totals.referenceInk === 0 ? 1 : 0)
      : 1 - totals.extraInk / totals.candidateInk;
    const unmatchedCandidate = new Set(inkComponents.candidate.map((_, index) => index));
    const matchedReference = new Set();
    const matchedComponentPairs = [];
    let smallComponentDimensionDelta = 0;
    let smallComponentDimensionMismatch = null;
    let enclosedSmallComponentDimensionDelta = 0;
    let enclosedSmallComponentDimensionMismatch = null;
    let matchedComponentCount = 0;
    let maximumMatchedCenterDistance = 0;
    let maximumMatchedDimensionDelta = 0;
    let maximumMatchedDimensionMismatch = null;
    const matchedComponentDimensionMismatches = [];
    function isEnclosedComponent(component, allComponents) {
      const centerX = component.box.x + component.box.width / 2;
      const centerY = component.box.y + component.box.height / 2;
      return allComponents.some((container) =>
        container !== component
        && container.box.width > component.box.width + 1
        && container.box.height > component.box.height + 1
        && centerX > container.box.x
        && centerX < container.box.x + container.box.width
        && centerY > container.box.y
        && centerY < container.box.y + container.box.height);
    }
    for (const [referenceIndex, referenceComponent] of inkComponents.reference.entries()) {
      const referenceCenter = {
        x: referenceComponent.box.x + referenceComponent.box.width / 2,
        y: referenceComponent.box.y + referenceComponent.box.height / 2,
      };
      let best = null;
      for (const index of unmatchedCandidate) {
        const candidateComponent = inkComponents.candidate[index];
        const candidateCenter = {
          x: candidateComponent.box.x + candidateComponent.box.width / 2,
          y: candidateComponent.box.y + candidateComponent.box.height / 2,
        };
        const distance = Math.hypot(
          referenceCenter.x - candidateCenter.x,
          referenceCenter.y - candidateCenter.y,
        );
        const dimensionDistance =
          Math.abs(referenceComponent.box.width - candidateComponent.box.width)
          + Math.abs(referenceComponent.box.height - candidateComponent.box.height);
        const cost = distance + dimensionDistance * 0.25;
        if (distance <= 2 && (!best || cost < best.cost)) {
          best = { index, distance, cost };
        }
      }
      if (!best) continue;
      const candidateComponent = inkComponents.candidate[best.index];
      unmatchedCandidate.delete(best.index);
      matchedReference.add(referenceIndex);
      matchedComponentPairs.push({
        referenceIndex,
        candidateIndex: best.index,
        centerDistance: best.distance,
      });
      matchedComponentCount += 1;
      maximumMatchedCenterDistance = Math.max(maximumMatchedCenterDistance, best.distance);
      const dimensionDelta = Math.max(
        Math.abs(referenceComponent.box.width - candidateComponent.box.width),
        Math.abs(referenceComponent.box.height - candidateComponent.box.height),
      );
      if (dimensionDelta > maximumMatchedDimensionDelta) {
        maximumMatchedDimensionDelta = dimensionDelta;
        maximumMatchedDimensionMismatch = {
          reference: referenceComponent,
          candidate: candidateComponent,
          centerDistance: best.distance,
        };
      }
      matchedComponentDimensionMismatches.push({
        dimensionDelta,
        reference: referenceComponent,
        candidate: candidateComponent,
        centerDistance: best.distance,
      });
      matchedComponentDimensionMismatches.sort(
        (left, right) => right.dimensionDelta - left.dimensionDelta,
      );
      if (matchedComponentDimensionMismatches.length > 12) {
        matchedComponentDimensionMismatches.length = 12;
      }
      const maximumDimension = Math.max(
        referenceComponent.box.width,
        referenceComponent.box.height,
        candidateComponent.box.width,
        candidateComponent.box.height,
      );
      const minimumDimension = Math.min(
        referenceComponent.box.width,
        referenceComponent.box.height,
        candidateComponent.box.width,
        candidateComponent.box.height,
      );
      if (maximumDimension <= 30 && minimumDimension <= 5) {
        if (dimensionDelta > smallComponentDimensionDelta) {
          smallComponentDimensionDelta = dimensionDelta;
          smallComponentDimensionMismatch = {
            reference: referenceComponent,
            candidate: candidateComponent,
            centerDistance: best.distance,
          };
        }
        if (
          dimensionDelta > enclosedSmallComponentDimensionDelta
          && isEnclosedComponent(referenceComponent, inkComponents.reference)
          && isEnclosedComponent(candidateComponent, inkComponents.candidate)
        ) {
          enclosedSmallComponentDimensionDelta = dimensionDelta;
          enclosedSmallComponentDimensionMismatch = {
            reference: referenceComponent,
            candidate: candidateComponent,
            centerDistance: best.distance,
          };
        }
      }
    }
    const domainWidth = Math.max(domain.right - domain.left, 1);
    const domainHeight = Math.max(domain.bottom - domain.top, 1);
    const relativePairs = [];
    for (const [referenceIndex, referenceComponent] of inkComponents.reference.entries()) {
      const referenceCenter = {
        x: referenceComponent.box.x + referenceComponent.box.width / 2,
        y: referenceComponent.box.y + referenceComponent.box.height / 2,
      };
      for (const [candidateIndex, candidateComponent] of inkComponents.candidate.entries()) {
        const candidateCenter = {
          x: candidateComponent.box.x + candidateComponent.box.width / 2,
          y: candidateComponent.box.y + candidateComponent.box.height / 2,
        };
        const relativeDistance = Math.hypot(
          (referenceCenter.x - candidateCenter.x) / domainWidth,
          (referenceCenter.y - candidateCenter.y) / domainHeight,
        );
        if (relativeDistance <= settings.maxRelativeComponentCenterDistance) {
          const dimensionDistance =
            Math.abs(referenceComponent.box.width - candidateComponent.box.width) / domainWidth
            + Math.abs(referenceComponent.box.height - candidateComponent.box.height) / domainHeight;
          relativePairs.push({
            referenceIndex,
            candidateIndex,
            cost: relativeDistance + dimensionDistance * 0.25,
          });
        }
      }
    }
    relativePairs.sort((left, right) => left.cost - right.cost);
    const relativeMatchedReference = new Set();
    const relativeMatchedCandidate = new Set();
    for (const pair of relativePairs) {
      if (
        relativeMatchedReference.has(pair.referenceIndex)
        || relativeMatchedCandidate.has(pair.candidateIndex)
      ) continue;
      relativeMatchedReference.add(pair.referenceIndex);
      relativeMatchedCandidate.add(pair.candidateIndex);
    }
    const relativeMatchedComponentCount = relativeMatchedReference.size;
    function sortedNormalizedCenters(components, axis) {
      const origin = axis === "x" ? domain.left : domain.top;
      const extent = axis === "x" ? domainWidth : domainHeight;
      return components
        .map((component) => (
          component.box[axis] + component.box[axis === "x" ? "width" : "height"] / 2 - origin
        ) / extent)
        .sort((left, right) => left - right);
    }
    function meanSortedDelta(first, second) {
      if (first.length !== second.length || first.length === 0) return 1;
      return first.reduce((total, value, index) =>
        total + Math.abs(value - second[index]), 0) / first.length;
    }
    const componentPositionDistributionDelta = Math.max(
      meanSortedDelta(
        sortedNormalizedCenters(inkComponents.reference, "x"),
        sortedNormalizedCenters(inkComponents.candidate, "x"),
      ),
      meanSortedDelta(
        sortedNormalizedCenters(inkComponents.reference, "y"),
        sortedNormalizedCenters(inkComponents.candidate, "y"),
      ),
    );
    function boxIntersectionArea(first, second) {
      const width = Math.max(
        0,
        Math.min(first.x + first.width, second.x + second.width)
          - Math.max(first.x, second.x),
      );
      const height = Math.max(
        0,
        Math.min(first.y + first.height, second.y + second.height)
          - Math.max(first.y, second.y),
      );
      return width * height;
    }
    function boxesHaveSpatialSupport(first, second) {
      const horizontalGap = Math.max(
        0,
        first.x - second.x - second.width,
        second.x - first.x - first.width,
      );
      const verticalGap = Math.max(
        0,
        first.y - second.y - second.height,
        second.y - first.y - first.height,
      );
      return horizontalGap <= settings.componentSupportGap
        && verticalGap <= settings.componentSupportGap;
    }
    const independentUnmatchedReferenceComponents = inkComponents.reference.filter(
      (component, index) => !matchedReference.has(index)
        && !inkComponents.candidate.some(
          (opposite) => boxesHaveSpatialSupport(component.box, opposite.box),
        ),
    );
    const independentUnmatchedCandidateComponents = inkComponents.candidate.filter(
      (component, index) => unmatchedCandidate.has(index)
        && !inkComponents.reference.some(
          (opposite) => boxesHaveSpatialSupport(component.box, opposite.box),
        ),
    );
    const independentComponentCountDelta = Math.abs(
      independentUnmatchedReferenceComponents.length
      - independentUnmatchedCandidateComponents.length,
    );
    function componentCenter(component) {
      return {
        x: component.box.x + component.box.width / 2,
        y: component.box.y + component.box.height / 2,
      };
    }
    function overlappingComponentIndices(defect, componentsToSearch) {
      return componentsToSearch
        .map((component, index) => ({ component, index }))
        .filter(({ component }) => boxIntersectionArea(defect.box, component.box) > 0)
        .map(({ index }) => index);
    }
    const matchedPairKeys = new Set(
      matchedComponentPairs.map(
        ({ referenceIndex, candidateIndex }) => `${referenceIndex}:${candidateIndex}`,
      ),
    );
    const displacedDefectPairs = [];
    const displacedMissing = topDefects.filter(
      (entry) => entry.kind === "missing" && entry.area >= settings.minDisplacedDefectArea,
    );
    const displacedExtra = topDefects.filter(
      (entry) => entry.kind === "extra" && entry.area >= settings.minDisplacedDefectArea,
    );
    for (const missing of displacedMissing) {
      for (const extra of displacedExtra) {
        const smallerArea = Math.min(missing.area, extra.area);
        const areaRatio = Math.max(missing.area, extra.area) / smallerArea;
        const dimensionDelta = Math.max(
          Math.abs(missing.box.width - extra.box.width),
          Math.abs(missing.box.height - extra.box.height),
        );
        const defectCenterDistance = Math.hypot(
          missing.box.x + missing.box.width / 2 - extra.box.x - extra.box.width / 2,
          missing.box.y + missing.box.height / 2 - extra.box.y - extra.box.height / 2,
        );
        if (
          areaRatio > settings.maxDisplacedDefectAreaRatio
          || dimensionDelta > settings.maxDisplacedDefectDimensionDelta
          || defectCenterDistance < settings.minDisplacedDefectDistance
          || defectCenterDistance > settings.localWindow
        ) continue;

        const referenceIndices = overlappingComponentIndices(
          missing,
          inkComponents.reference,
        );
        const candidateIndices = overlappingComponentIndices(
          extra,
          inkComponents.candidate,
        );
        let supportingComponents = null;
        for (const referenceIndex of referenceIndices) {
          const referenceComponent = inkComponents.reference[referenceIndex];
          const referenceCenter = componentCenter(referenceComponent);
          for (const candidateIndex of candidateIndices) {
            const candidateComponent = inkComponents.candidate[candidateIndex];
            const candidateCenter = componentCenter(candidateComponent);
            const componentCenterDistance = Math.hypot(
              referenceCenter.x - candidateCenter.x,
              referenceCenter.y - candidateCenter.y,
            );
            const componentAreaRatio =
              Math.max(referenceComponent.area, candidateComponent.area)
              / Math.min(referenceComponent.area, candidateComponent.area);
            const componentDimensionDelta = Math.max(
              Math.abs(referenceComponent.box.width - candidateComponent.box.width),
              Math.abs(referenceComponent.box.height - candidateComponent.box.height),
            );
            const isMovedMatchedComponent =
              matchedPairKeys.has(`${referenceIndex}:${candidateIndex}`)
              && componentCenterDistance >= settings.minDisplacedDefectDistance
              && componentCenterDistance <= settings.localWindow;
            const isMovedUnmatchedComponent =
              !matchedReference.has(referenceIndex)
              && unmatchedCandidate.has(candidateIndex)
              && componentAreaRatio <= settings.maxDisplacedDefectAreaRatio
              && componentDimensionDelta <= settings.maxDisplacedDefectDimensionDelta
              && componentCenterDistance >= settings.minDisplacedDefectDistance
              && componentCenterDistance <= settings.localWindow;
            if (!isMovedMatchedComponent && !isMovedUnmatchedComponent) continue;
            supportingComponents = {
              reference: referenceComponent,
              candidate: candidateComponent,
              centerDistance: componentCenterDistance,
              relation: isMovedMatchedComponent ? "matched" : "unmatched",
            };
            break;
          }
          if (supportingComponents) break;
        }
        if (!supportingComponents) continue;
        displacedDefectPairs.push({
          missing,
          extra,
          defectCenterDistance,
          supportingComponents,
        });
        if (displacedDefectPairs.length >= 12) break;
      }
      if (displacedDefectPairs.length >= 12) break;
    }
    const reasons = [];
    if (local.referenceCoverage < settings.minCoverage) reasons.push("local-reference-coverage");
    if (local.candidateCoverage < settings.minCoverage) reasons.push("local-candidate-coverage");
    if (largestMissing.area > settings.maxDefectArea) reasons.push("missing-detail-area");
    if (largestExtra.area > settings.maxDefectArea) reasons.push("extra-detail-area");
    if (largestMissing.span > settings.maxDefectSpan) reasons.push("missing-detail-span");
    if (largestExtra.span > settings.maxDefectSpan) reasons.push("extra-detail-span");

    local.spatialCells = [...spatialCellsByKey.values()]
      .map((cell) => ({
        column: cell.column,
        row: cell.row,
        missingMaskWords: [...cell.missingMaskWords.entries()]
          .sort((left, right) => left[0] - right[0]),
        extraMaskWords: [...cell.extraMaskWords.entries()]
          .sort((left, right) => left[0] - right[0]),
      }))
      .sort((left, right) => left.row - right.row || left.column - right.column);
    function maskPixelCount(cells, kind) {
      let count = 0;
      for (const cell of cells) {
        for (const [, encodedBits] of cell[`${kind}MaskWords`]) {
          let bits = encodedBits >>> 0;
          while (bits !== 0) {
            bits = (bits & (bits - 1)) >>> 0;
            count += 1;
          }
        }
      }
      return count;
    }
    if (
      maskPixelCount(local.spatialCells, "missing") !== totals.missingInk
      || maskPixelCount(local.spatialCells, "extra") !== totals.extraInk
    ) throw new Error("spatial occupancy does not match analyzed mismatch totals");

    return {
      passed: reasons.length === 0,
      reasons,
      referenceCoverage,
      candidateCoverage,
      local,
      largestMissing,
      largestExtra,
      topDefects,
      detailFeatures: {
        compactDefectCount,
        referenceComponentCount: inkComponents.reference.length,
        candidateComponentCount: inkComponents.candidate.length,
        componentCountDelta: Math.abs(
          inkComponents.reference.length - inkComponents.candidate.length,
        ),
        independentUnmatchedReferenceComponentCount:
          independentUnmatchedReferenceComponents.length,
        independentUnmatchedCandidateComponentCount:
          independentUnmatchedCandidateComponents.length,
        independentComponentCountDelta,
        matchedComponentCount,
        componentMatchCoverage: matchedComponentCount / Math.max(
          inkComponents.reference.length,
          inkComponents.candidate.length,
          1,
        ),
        unmatchedReferenceComponentCount:
          inkComponents.reference.length - matchedComponentCount,
        unmatchedCandidateComponentCount: unmatchedCandidate.size,
        maximumMatchedCenterDistance,
        maximumMatchedDimensionDelta,
        maximumMatchedDimensionMismatch,
        matchedComponentDimensionMismatches,
        relativeMatchedComponentCount,
        relativeComponentMatchCoverage: relativeMatchedComponentCount / Math.max(
          inkComponents.reference.length,
          inkComponents.candidate.length,
          1,
        ),
        componentPositionDistributionDelta,
        displacedDefectPairs,
        unmatchedRelativeReferenceComponents: inkComponents.reference
          .filter((_, index) => !relativeMatchedReference.has(index))
          .slice(0, 12),
        unmatchedRelativeCandidateComponents: inkComponents.candidate
          .filter((_, index) => !relativeMatchedCandidate.has(index))
          .slice(0, 12),
        smallComponentDimensionDelta,
        smallComponentDimensionMismatch,
        enclosedSmallComponentDimensionDelta,
        enclosedSmallComponentDimensionMismatch,
      },
      totals,
      domain,
      settings: {
        analysisScale: settings.analysisScale,
        componentSupportGap: settings.componentSupportGap,
        tolerance: settings.tolerance,
        tileSize: settings.tileSize,
        halo: settings.halo,
        localWindow: settings.localWindow,
        localStride: settings.localStride,
        minimumWindowInk: settings.minimumWindowInk,
        minCoverage: settings.minCoverage,
        maxDefectArea: settings.maxDefectArea,
        maxDefectSpan: settings.maxDefectSpan,
      },
    };
  }, { referenceDataUrl, candidateDataUrl, alignment, settings });
}

export function detailGateReasons(detail, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  const reasons = [];
  if (
    detail.detailFeatures.independentComponentCountDelta
    > settings.maxComponentCountDelta
  ) {
    reasons.push("detail-component-count");
  }
  if (
    detail.detailFeatures.enclosedSmallComponentDimensionDelta
    > settings.maxEnclosedSmallComponentDimensionDelta
  ) {
    reasons.push("detail-enclosed-small-component-dimension");
  }
  const repeatedMicroDefects =
    detail.detailFeatures.compactDefectCount > settings.maxRepeatedMicroDefects
    && detail.largestMissing.area <= settings.maxRepeatedMicroDefectArea
    && detail.largestExtra.area <= settings.maxRepeatedMicroDefectArea
    && detail.local.referenceCoverage >= settings.minRepeatedMicroCoverage
    && detail.local.candidateCoverage >= settings.minRepeatedMicroCoverage;
  if (repeatedMicroDefects) reasons.push("detail-repeated-micro-defects");
  // Difference masks alone cannot identify object identity: two stems in
  // neighboring glyphs or two parallel frame edges can have nearly identical
  // boxes. The image analyzer therefore records a displacement only when the
  // missing and extra masks both belong to the same matched ink-component pair
  // or to one size-compatible unmatched pair. This keeps the gate sensitive to
  // a moved subscript while preventing repeated details from cross-pairing.
  if (detail.detailFeatures.displacedDefectPairs?.length) {
    reasons.push("detail-displaced-component");
  }
  return reasons;
}

export function classifyAnalyzedVisualMetrics(coarseMetrics, detailMetrics, options = {}) {
  const detailReasons = detailMetrics ? detailGateReasons(detailMetrics, options) : [];
  return {
    ...coarseMetrics,
    passed: coarseMetrics.passed && detailReasons.length === 0,
    reasons: [
      ...(coarseMetrics.passed ? [] : coarseMetrics.reasons),
      ...detailReasons,
    ],
    coarsePassed: coarseMetrics.passed,
    detail: detailMetrics ? {
      local: detailMetrics.local,
      largestMissing: detailMetrics.largestMissing,
      largestExtra: detailMetrics.largestExtra,
      topDefects: detailMetrics.topDefects,
      detailFeatures: detailMetrics.detailFeatures,
      totals: detailMetrics.totals,
      domain: detailMetrics.domain,
      settings: detailMetrics.settings,
    } : null,
  };
}

function detailAnalysisOptions(options) {
  return {
    analysisScale: options.detailAnalysisScale,
    tolerance: options.detailTolerance,
    tileSize: options.tileSize,
    halo: options.halo,
    localWindow: options.detailLocalWindow,
    localStride: options.detailLocalStride,
    minimumWindowInk: options.detailMinimumWindowInk,
    minCoverage: 0,
    maxDefectArea: Number.MAX_SAFE_INTEGER,
    maxDefectSpan: Number.MAX_SAFE_INTEGER,
    candidateRegressionRasterPadding:
      options.candidateRegressionRasterPadding,
    candidateRasterViewBox: options.candidateRasterViewBox,
  };
}

function compactSpatialMetrics(metrics) {
  const layers = [
    { local: metrics.local, settings: metrics.settings },
    { local: metrics.detail?.local, settings: metrics.detail?.settings },
  ];
  for (const layer of layers) {
    if (!layer.local || !layer.settings) continue;
    layer.local.spatialFloor = encodeSpatialFloor(
      layer.local.spatialCells,
      layer.settings.analysisScale,
      layer.settings.localStride,
    );
    delete layer.local.spatialCells;
  }
  return metrics;
}

function fixedAlignmentRegressionMetrics(coarseMetrics, detailMetrics, options) {
  return compactSpatialMetrics(classifyAnalyzedVisualMetrics(
    coarseMetrics,
    detailMetrics,
    options,
  ));
}

export function historicalDocumentAlignment(historicalAlignment, currentAlignment) {
  const historicalViewBox = historicalAlignment?.vectorFrame?.chemsemaViewBox;
  const currentViewBox = currentAlignment?.vectorFrame?.chemsemaViewBox;
  if (
    historicalAlignment?.algorithm !== ALIGNMENT_ALGORITHM
    || currentAlignment?.algorithm !== ALIGNMENT_ALGORITHM
    || !Number.isFinite(historicalAlignment.scale)
    || !Number.isFinite(historicalAlignment.dx)
    || !Number.isFinite(historicalAlignment.dy)
    || !Number.isFinite(historicalViewBox?.x)
    || !Number.isFinite(historicalViewBox?.y)
    || !Number.isFinite(currentViewBox?.x)
    || !Number.isFinite(currentViewBox?.y)
  ) return null;
  return {
    ...historicalAlignment,
    // SVG export crops to its current viewBox. Preserve the historical map
    // from ChemSema document coordinates to ChemDraw reference coordinates,
    // not the obsolete crop's top-left pixel.
    dx: historicalAlignment.dx
      + historicalAlignment.scale * (currentViewBox.x - historicalViewBox.x),
    dy: historicalAlignment.dy
      + historicalAlignment.scale * (currentViewBox.y - historicalViewBox.y),
    chemsemaWidth: currentAlignment.chemsemaWidth,
    chemsemaHeight: currentAlignment.chemsemaHeight,
    vectorFrame: currentAlignment.vectorFrame,
  };
}

export function candidateRegressionRasterViewBox(alignment, padding) {
  const viewBox = alignment?.vectorFrame?.chemsemaViewBox;
  if (
    ![viewBox?.x, viewBox?.y, viewBox?.width, viewBox?.height].every(Number.isFinite)
    || viewBox.width <= 0
    || viewBox.height <= 0
    || !Number.isFinite(padding)
    || padding < 0
  ) return null;
  return {
    x: viewBox.x - padding,
    y: viewBox.y - padding,
    width: viewBox.width + padding * 2,
    height: viewBox.height + padding * 2,
  };
}

export function gatePolicy(options) {
  return {
    coordinateSpace: "ChemDraw reference image coordinates",
    alignment:
      "authored document bounds plus a unique ChemDraw vector matrix fix scale and origin; "
      + "layout-relative sources keep declared scale and search translation; references without "
      + "a unique vector matrix search scale and translation; historical "
      + "pass protection never changes current-image registration; continuous comparison of "
      + "an already-failing case separately preserves its historical document-coordinate map "
      + "while compensating for changes to the exported SVG crop",
    canvasWhitespaceIncluded: false,
    caseWeighting: "one case, one vote",
    comparison: "coarse fixed-window coverage and defects, followed by spatially independent fine connected-component and repeated-micro-defect checks",
    regressionProtection:
      "every failed case retains its historical document-coordinate registration and compressed fixed-grid coarse and detail missing/extra occupancy masks; same-kind mismatch support inside a fixed absolute tolerance is stable, while an unsupported connected defect is accepted only when pooled missing-plus-extra mismatch mass decreases inside its fixed 12-reference-unit neighborhood; unresolved pixels accumulate globally so distant improvements cannot cancel a new local defect",
    regressionTolerances: {
      coarseUnsupportedArea:
        LOCAL_WINDOW_REGRESSION_TOLERANCES.coarse.defectArea,
      coarseSupportRadius:
        LOCAL_WINDOW_REGRESSION_TOLERANCES.coarse.center,
      coarseLocalImprovementRadius:
        LOCAL_WINDOW_REGRESSION_TOLERANCES.coarse.improvementWindow,
      detailUnsupportedArea:
        LOCAL_WINDOW_REGRESSION_TOLERANCES.detail.defectArea,
      detailSupportRadius:
        LOCAL_WINDOW_REGRESSION_TOLERANCES.detail.center,
      detailLocalImprovementRadius:
        LOCAL_WINDOW_REGRESSION_TOLERANCES.detail.improvementWindow,
    },
    pass: {
      minimumCandidateViewportInkMargin: options.minCandidateViewportInkMargin,
      minimumFixedWindowReferenceCoverage: options.minCoverage,
      minimumFixedWindowCandidateCoverage: options.minCoverage,
      maximumLocalDefectArea: options.maxDefectArea,
      maximumLocalDefectSpan: options.maxDefectSpan,
      fineComponentSupportGap: options.componentSupportGap,
      maximumFineComponentCountDelta: options.maxComponentCountDelta,
      maximumEnclosedSmallComponentDimensionDelta: options.maxEnclosedSmallComponentDimensionDelta,
      maximumRepeatedMicroDefects: options.maxRepeatedMicroDefects,
      maximumRepeatedMicroDefectArea: options.maxRepeatedMicroDefectArea,
      minimumRepeatedMicroCoverage: options.minRepeatedMicroCoverage,
      minimumDisplacedDefectArea: options.minDisplacedDefectArea,
      minimumDisplacedDefectDistance: options.minDisplacedDefectDistance,
      maximumDisplacedDefectAreaRatio: options.maxDisplacedDefectAreaRatio,
      maximumDisplacedDefectDimensionDelta:
        options.maxDisplacedDefectDimensionDelta,
    },
    analysis: {
      maximumRelativeComponentCenterDistance:
        options.maxRelativeComponentCenterDistance,
    },
    raster: {
      pixelsPerReferenceUnit: options.analysisScale,
      toleranceReferenceUnits: options.tolerance,
      tileSizeReferenceUnits: options.tileSize,
      haloReferenceUnits: options.halo,
      localWindowReferenceUnits: options.localWindow,
      localStrideReferenceUnits: options.localStride,
      minimumWindowInkAreaReferenceUnits: options.minimumWindowInk,
      spatialRegressionCellReferenceUnits: options.localStride,
    },
    candidateViewportRaster: {
      pixelsPerCandidateViewportUnit: options.candidateViewportAnalysisScale,
      maximumRasterPixels: options.maxCandidateViewportPixels,
      minimumInkMarginCandidateViewportUnits: options.minCandidateViewportInkMargin,
      scope: "candidate SVG self-consistency before ChemDraw alignment",
    },
    continuousRegressionRaster: {
      candidateViewportPaddingUnits:
        options.candidateRegressionRasterPadding,
      viewportPolicy:
        "the baseline candidate SVG viewBox plus fixed padding is reused verbatim; a later crop outside that registered viewport is rejected as non-comparable",
      reason:
        "unchanged SVG text and strokes must retain identical absolute raster positions when unrelated geometry changes the export crop",
    },
    detailRaster: {
      pixelsPerReferenceUnit: options.detailAnalysisScale,
      toleranceReferenceUnits: options.detailTolerance,
      localWindowReferenceUnits: options.detailLocalWindow,
      localStrideReferenceUnits: options.detailLocalStride,
      minimumWindowInkAreaReferenceUnits: options.detailMinimumWindowInk,
      spatialRegressionCellReferenceUnits: options.detailLocalStride,
      svgViewportNormalization:
        "root SVG viewport is normalized to its final aligned reference-unit size before rasterization",
    },
  };
}

export function defaultGatePolicy() {
  return gatePolicy(DEFAULTS);
}

export function passFloorGateDefinition(options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return {
    cacheIdentity: CACHE_IDENTITY,
    alignmentAlgorithm: ALIGNMENT_ALGORITHM,
    policySha256: crypto
      .createHash("sha256")
      .update(JSON.stringify(gatePolicy(settings)))
      .digest("hex"),
  };
}

export function passFloorGateDefinitionErrors(passFloor, options = {}) {
  const errors = [];
  if (passFloor?.schema !== STRICT_PASS_FLOOR_SCHEMA) {
    errors.push("pass floor has a missing or unsupported schema");
  }
  if (
    JSON.stringify(passFloor?.gateDefinition)
    !== JSON.stringify(passFloorGateDefinition(options))
  ) {
    errors.push("pass floor was established by a different gate definition");
  }
  return errors;
}

async function writePassedGallery(manifest, report, galleryDir, requestedPath) {
  const passedGalleryPath = path.resolve(
    requestedPath ?? path.join(galleryDir, "passed.html"),
  );
  const passedIds = new Set(report.cases
    .filter((entry) => entry.status === "pass")
    .map((entry) => entry.id));
  const passedItems = manifest.items.filter((item) => passedIds.has(item.id));
  await fs.mkdir(path.dirname(passedGalleryPath), { recursive: true });
  await fs.writeFile(passedGalleryPath, viewerHtml(passedItems));
  return { passedGalleryPath, count: passedItems.length };
}

export async function reuseReportCompatibilityErrors(
  report,
  manifest,
  galleryDir,
  options,
) {
  const errors = [];
  const settings = { ...DEFAULTS, ...options };
  if (report?.schema !== REPORT_SCHEMA) {
    errors.push("report schema is missing or unsupported");
  }
  if (report?.caseMetricsSchema !== CASE_METRICS_SCHEMA) {
    errors.push("report case metrics schema is missing or unsupported");
  }
  if (!options.allowGateDefinitionUpgrade && report?.cacheIdentity !== CACHE_IDENTITY) {
    errors.push("report uses a different gate definition");
  }
  if (JSON.stringify(report?.policy) !== JSON.stringify(gatePolicy(settings))) {
    errors.push("report policy differs from the requested gate policy");
  }
  if (!report.gallery || path.resolve(report.gallery) !== galleryDir) {
    errors.push("report gallery differs from the requested gallery");
  }
  if (
    JSON.stringify(report?.galleryProvenance)
    !== JSON.stringify(manifest.provenance ?? null)
  ) {
    errors.push("report gallery provenance differs from the current manifest");
  }
  const items = new Map(
    manifest.items.map((item) => [
      normalizedCasePath(item.relativeCdxml),
      item,
    ]),
  );
  const seen = new Set();
  for (const entry of report?.cases ?? []) {
    const relativeCdxml = normalizedCasePath(entry.relativeCdxml);
    const item = items.get(relativeCdxml);
    if (!item || seen.has(relativeCdxml)) {
      errors.push(`report contains an unknown or duplicate case ${relativeCdxml}`);
      break;
    }
    seen.add(relativeCdxml);
    const metricErrors = visualCaseMetricContractErrors(entry, settings);
    if (metricErrors.length) {
      errors.push(
        `report has incomplete case metrics for ${relativeCdxml}: ${metricErrors.join(", ")}`,
      );
      break;
    }
    const currentHashes = await artifactHashes(galleryDir, item);
    if (!artifactHashesEqual(entry.artifactHashes, currentHashes)) {
      errors.push(`report artifacts changed for ${relativeCdxml}`);
      break;
    }
  }
  return errors;
}

async function runSelfTest(options) {
  const svg = (width, height, detail, common = "") => `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <rect width="100%" height="100%" fill="white"/>
    <path d="M 20 40 L 100 40 M 60 20 L 60 80 ${detail} ${common}" fill="none" stroke="black" stroke-width="2"/>
  </svg>`;
  const data = (source) => `data:image/svg+xml;base64,${Buffer.from(source).toString("base64")}`;
  const browser = await launchBrowser({ headless: true });
  const page = await browser.newPage();
  try {
    const vectorReference = `<svg xmlns="http://www.w3.org/2000/svg" width="120px" height="80px" viewBox="0 0 120 80">
      <path transform="matrix(0.125 0 0 0.125 7 -11)" d="M 200 200 L 400 400" fill="none" stroke="black" stroke-width="16"/>
    </svg>`;
    const vectorCandidate = `<svg xmlns="http://www.w3.org/2000/svg" width="48.25" height="32.75" viewBox="-3 5 48.25 32.75">
      <path d="M 10 10 L 20 20" fill="none" stroke="black" stroke-width="0.8"/>
    </svg>`;
    const vectorAlignment = await computeImageAlignment(
      page,
      data(vectorReference),
      data(vectorCandidate),
    );
    const vectorExpected = { scale: 2.5, dx: -0.5, dy: 1.5 };
    if (
      vectorAlignment.algorithm !== ALIGNMENT_ALGORITHM
      || vectorAlignment.basis !== "declared-transform-origin"
      || Math.abs(vectorAlignment.scale - vectorExpected.scale) > 1e-9
      || Math.abs(vectorAlignment.dx - vectorExpected.dx) > 1e-9
      || Math.abs(vectorAlignment.dy - vectorExpected.dy) > 1e-9
      || vectorAlignment.chemsemaWidth !== 48.25
      || vectorAlignment.chemsemaHeight !== 32.75
    ) {
      throw new Error(
        `deterministic vector-frame alignment regression: ${JSON.stringify(vectorAlignment)}`,
      );
    }
    const layoutRelativeAlignment = await computeImageAlignment(
      page,
      data(vectorReference),
      data(vectorCandidate),
      "ink-overlap-source-without-document-bounds",
    );
    if (
      layoutRelativeAlignment.algorithm !== ALIGNMENT_ALGORITHM
      || layoutRelativeAlignment.basis !== "declared-scale-ink-translation"
      || Math.abs(layoutRelativeAlignment.scale - vectorExpected.scale) > 1e-9
    ) {
      throw new Error(
        `layout-relative fixed-scale alignment regression: ${JSON.stringify(layoutRelativeAlignment)}`,
      );
    }
    const croppedVectorCandidate = vectorCandidate
      .replace('height="32.75"', 'height="30.75"')
      .replace('viewBox="-3 5 48.25 32.75"', 'viewBox="-3 7 48.25 30.75"');
    const croppedVectorAlignment = await computeImageAlignment(
      page,
      data(vectorReference),
      data(croppedVectorCandidate),
    );
    const vectorWorldTranslation = (alignment, viewBoxY, height, viewBoxHeight) => ({
      x: alignment.dx - alignment.scale * -3,
      y: alignment.dy - alignment.scale * viewBoxY * height / viewBoxHeight,
    });
    const originalWorld = vectorWorldTranslation(vectorAlignment, 5, 32.75, 32.75);
    const croppedWorld = vectorWorldTranslation(croppedVectorAlignment, 7, 30.75, 30.75);
    if (
      Math.abs(vectorAlignment.scale - croppedVectorAlignment.scale) > 1e-9
      || Math.abs(originalWorld.x - croppedWorld.x) > 1e-9
      || Math.abs(originalWorld.y - croppedWorld.y) > 1e-9
    ) {
      throw new Error(
        `SVG crop changed document-world registration: ${JSON.stringify({
          vectorAlignment,
          croppedVectorAlignment,
          originalWorld,
          croppedWorld,
        })}`,
      );
    }
    const cropStableReference = `<svg xmlns="http://www.w3.org/2000/svg" width="1749" height="1376" viewBox="0 0 1749 1376"></svg>`;
    const cropStableBody = `
      <text x="0" y="0" font-size="200" font-family="Arial" transform="matrix(0.05 0 0 0.05 565 286)">[byproducts]</text>
      <polyline points="605.46,302.39 605.46,400.39" fill="none" stroke="#000" stroke-width="1"/>`;
    const oldCrop = `<svg xmlns="http://www.w3.org/2000/svg" width="650.57891" height="527.83" viewBox="-7.7 -15.83 650.57891 527.83">${cropStableBody}</svg>`;
    const newCrop = `<svg xmlns="http://www.w3.org/2000/svg" width="669.296723" height="527.83" viewBox="-26.417813 -15.83 669.296723 527.83">${cropStableBody}</svg>`;
    const oldCropAlignment = {
      algorithm: ALIGNMENT_ALGORITHM,
      scale: 2.6666600000000003,
      dx: 36.55213466666666,
      dy: -16.7069778,
      vectorFrame: {
        chemsemaViewBox: {
          x: -7.7,
          y: -15.83,
          width: 650.57891,
          height: 527.83,
        },
      },
    };
    const newCropAlignment = {
      ...oldCropAlignment,
      dx: -13.361908547913341,
      vectorFrame: {
        chemsemaViewBox: {
          x: -26.417813,
          y: -15.83,
          width: 669.296723,
          height: 527.83,
        },
      },
    };
    const cropStableOptions = detailAnalysisOptions(options);
    const oldCropMetrics = await analyzeAlignedImages(
      page,
      data(cropStableReference),
      data(oldCrop),
      oldCropAlignment,
      cropStableOptions,
    );
    const newCropMetrics = await analyzeAlignedImages(
      page,
      data(cropStableReference),
      data(newCrop),
      newCropAlignment,
      {
        ...cropStableOptions,
        candidateRasterViewBox: candidateRegressionRasterViewBox(
          oldCropAlignment,
          options.candidateRegressionRasterPadding,
        ),
      },
    );
    const oldCropFloor = encodeSpatialFloor(
      oldCropMetrics.local.spatialCells,
      cropStableOptions.analysisScale,
      cropStableOptions.localStride,
    );
    const newCropFloor = encodeSpatialFloor(
      newCropMetrics.local.spatialCells,
      cropStableOptions.analysisScale,
      cropStableOptions.localStride,
    );
    if (oldCropFloor.recordsSha256 !== newCropFloor.recordsSha256) {
      throw new Error(
        `historical raster viewport changed unchanged SVG ink: ${JSON.stringify({
          old: oldCropFloor.recordsSha256,
          current: newCropFloor.recordsSha256,
        })}`,
      );
    }
    const outsideCrop = newCrop.replace(
      'viewBox="-26.417813 -15.83 669.296723 527.83"',
      'viewBox="-50 -15.83 692.87891 527.83"',
    ).replace('width="669.296723"', 'width="692.87891"');
    let outsideCropRejected = false;
    try {
      await analyzeAlignedImages(
        page,
        data(cropStableReference),
        data(outsideCrop),
        {
          ...newCropAlignment,
          dx: oldCropAlignment.dx
            + oldCropAlignment.scale * (-50 - -7.7),
          vectorFrame: {
            chemsemaViewBox: {
              x: -50,
              y: -15.83,
              width: 692.87891,
              height: 527.83,
            },
          },
        },
        {
          ...cropStableOptions,
          candidateRasterViewBox: candidateRegressionRasterViewBox(
            oldCropAlignment,
            options.candidateRegressionRasterPadding,
          ),
        },
      );
    } catch (error) {
      outsideCropRejected = /exceeds its historical regression raster viewport/.test(
        error instanceof Error ? error.message : String(error),
      );
    }
    if (!outsideCropRejected) {
      throw new Error("candidate outside the historical raster viewport was not rejected");
    }
    const clippedOverflowMetrics = await analyzeAlignedImages(
      page,
      data('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"></svg>'),
      data('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><path d="M -20 50 L -10 50" stroke="#000" stroke-width="2"/></svg>'),
      { scale: 1, dx: 0, dy: 0 },
      options,
    );
    if (clippedOverflowMetrics.totals.candidateInk !== 0) {
      throw new Error(
        `padded raster leaked ink outside the authored domain: ${
          clippedOverflowMetrics.totals.candidateInk
        }`,
      );
    }
    const viewportReference = `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80" viewBox="0 0 120 80">
      <rect width="120" height="80" fill="white"/>
      <text x="12" y="36" font-family="Arial" font-size="18">NH<tspan baseline-shift="sub" font-size="12">3</tspan><tspan baseline-shift="super" font-size="12">+</tspan></text>
      <path d="M 12 54 L 108 54" fill="none" stroke="black" stroke-width="1.5"/>
    </svg>`;
    const viewportCandidate = viewportReference
      .replace('width="120"', 'width="45"')
      .replace('height="80"', 'height="30"');
    const viewportAlignment = {
      scale: 120 / 45,
      dx: 0,
      dy: 0,
      referenceWidth: 120,
      referenceHeight: 80,
      chemsemaWidth: 45,
      chemsemaHeight: 30,
    };
    const viewportEquivalent = await analyzeAlignedImages(
      page,
      data(viewportReference),
      data(viewportCandidate),
      viewportAlignment,
      options,
    );
    if (
      !viewportEquivalent.passed
      || viewportEquivalent.largestMissing.area !== 0
      || viewportEquivalent.largestExtra.area !== 0
      || viewportEquivalent.detailFeatures.componentCountDelta !== 0
    ) {
      throw new Error(
        `SVG viewport normalization regression: ${JSON.stringify(viewportEquivalent)}`,
      );
    }
    const validViewport = await analyzeCandidateViewport(
      page,
      data(`<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80" viewBox="0 0 120 80">
        <path d="M 8.5 8.5 L 111.5 71.5" fill="none" stroke="black" stroke-width="1"/>
      </svg>`),
      options,
    );
    if (candidateViewportGateReasons(validViewport, options).length) {
      throw new Error(`valid candidate viewport margin regression: ${JSON.stringify(validViewport)}`);
    }
    const clippedViewport = await analyzeCandidateViewport(
      page,
      data(`<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="100" viewBox="0 0 2000 100">
        <path d="M 0 10 L 0 90 M 100 50 L 1900 50" fill="none" stroke="black" stroke-width="2"/>
      </svg>`),
      options,
    );
    if (!candidateViewportGateReasons(clippedViewport, options)
      .includes("candidate-viewport-ink-margin")) {
      throw new Error(`clipped candidate viewport escaped gate: ${JSON.stringify(clippedViewport)}`);
    }
    const fractionalCandidateWidth = 453.471516;
    const fractionalCandidateHeight = 127.29;
    const fractionalScale = 2.66666;
    const fractionalReferenceWidth = fractionalCandidateWidth * fractionalScale;
    const fractionalReferenceHeight = fractionalCandidateHeight * fractionalScale;
    const fractionalContent = `
      <rect x="94.1" y="256.16" width="453.471516" height="127.29" fill="white"/>
      <text x="113.25" y="309.75" font-family="Arial" font-size="13.5">NH<tspan baseline-shift="sub" font-size="9">3</tspan><tspan baseline-shift="super" font-size="9">+</tspan></text>
      <path d="M 110.5 346.25 L 524.75 346.25" fill="none" stroke="black" stroke-width="0.75"/>`;
    const fractionalSvg = (width, height) =>
      `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="94.1 256.16 453.471516 127.29">${fractionalContent}</svg>`;
    const fractionalAlignment = {
      scale: fractionalScale,
      dx: 0,
      dy: 0,
      referenceWidth: fractionalReferenceWidth,
      referenceHeight: fractionalReferenceHeight,
      chemsemaWidth: fractionalCandidateWidth,
      chemsemaHeight: fractionalCandidateHeight,
    };
    for (const analysisScale of [2, 4]) {
      const fractionalEquivalent = await analyzeAlignedImages(
        page,
        data(fractionalSvg(fractionalReferenceWidth, fractionalReferenceHeight)),
        data(fractionalSvg(fractionalCandidateWidth, fractionalCandidateHeight)),
        fractionalAlignment,
        { ...options, analysisScale, tolerance: 0 },
      );
      if (
        fractionalEquivalent.largestMissing.area !== 0
        || fractionalEquivalent.largestExtra.area !== 0
        || fractionalEquivalent.detailFeatures.componentCountDelta !== 0
      ) {
        throw new Error(
          `fractional SVG viewport normalization regression at ${analysisScale}x: ${
            JSON.stringify(fractionalEquivalent)
          }`,
        );
      }
    }
    const cropBody = `
      <text x="18.25" y="28.5" font-family="Arial" font-size="13.5">NH<tspan baseline-shift="sub" font-size="9">3</tspan></text>
      <path d="M 12.5 42.25 L 106.75 42.25 M 60.5 18.25 L 60.5 68.75" fill="none" stroke="black" stroke-width="1.25"/>`;
    const cropBefore = `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80" viewBox="0 0 120 80">${cropBody}</svg>`;
    const cropAfter = `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="76" viewBox="0 4 120 76">${cropBody}</svg>`;
    const emptyCrop = `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80" viewBox="0 0 120 80"></svg>`;
    for (const analysisScale of [2, 4]) {
      const cropBaseline = await analyzeAlignedImages(
        page,
        data(cropBefore),
        data(cropBefore),
        { scale: 1, dx: 0, dy: 0 },
        { ...options, analysisScale, tolerance: 0 },
      );
      const cropEquivalent = await analyzeAlignedImages(
        page,
        data(cropBefore),
        data(cropAfter),
        { scale: 1, dx: 0, dy: 4 },
        { ...options, analysisScale, tolerance: 0 },
      );
      if (
        cropEquivalent.referenceCoverage !== 1
        || cropEquivalent.candidateCoverage !== 1
        || cropEquivalent.local.referenceCoverage !== 1
        || cropEquivalent.local.candidateCoverage !== 1
        || cropEquivalent.largestMissing.area !== 0
        || cropEquivalent.largestExtra.area !== 0
        || cropEquivalent.detailFeatures.componentCountDelta !== 0
        || JSON.stringify(cropEquivalent.local.spatialCells)
          !== JSON.stringify(cropBaseline.local.spatialCells)
      ) {
        throw new Error(
          `SVG crop moved fixed gate lattices at ${analysisScale}x: ${
            JSON.stringify(cropEquivalent)
          }`,
        );
      }
      const tileBaseline = await analyzeAlignedImages(
        page,
        data(cropBefore),
        data(emptyCrop),
        { scale: 1, dx: 0, dy: 0 },
        { ...options, analysisScale, tolerance: 0, tileSize: 256 },
      );
      const tileVariant = await analyzeAlignedImages(
        page,
        data(cropBefore),
        data(emptyCrop),
        { scale: 1, dx: 0, dy: 0 },
        { ...options, analysisScale, tolerance: 0, tileSize: 240 },
      );
      if (
        tileVariant.totals.missingInk !== tileBaseline.totals.missingInk
        || tileVariant.totals.extraInk !== tileBaseline.totals.extraInk
        || JSON.stringify(tileVariant.local.spatialCells)
          !== JSON.stringify(tileBaseline.local.spatialCells)
      ) {
        throw new Error(`tile boundaries changed spatial occupancy at ${analysisScale}x`);
      }
    }
    const staleFrameEquivalent = await analyzeAlignedImages(
      page,
      data(fractionalSvg(fractionalReferenceWidth, fractionalReferenceHeight)),
      data(fractionalSvg(fractionalCandidateWidth, fractionalCandidateHeight)),
      {
        ...fractionalAlignment,
        referenceWidth: 999,
        referenceHeight: 777,
        chemsemaWidth: 333,
        chemsemaHeight: 222,
      },
      { ...options, analysisScale: 4, tolerance: 0 },
    );
    if (
      staleFrameEquivalent.largestMissing.area !== 0
      || staleFrameEquivalent.largestExtra.area !== 0
      || staleFrameEquivalent.detailFeatures.componentCountDelta !== 0
      || staleFrameEquivalent.domain.right
        !== Math.ceil(fractionalReferenceWidth)
      || staleFrameEquivalent.domain.bottom
        !== Math.ceil(fractionalReferenceHeight)
    ) {
      throw new Error(
        `stale alignment frame dimensions affected current SVG analysis: ${
          JSON.stringify(staleFrameEquivalent)
        }`,
      );
    }
    const vectorCoarse = await analyzeAlignedImages(
      page,
      data(vectorReference),
      data(vectorCandidate),
      vectorAlignment,
      options,
    );
    const vectorDetail = await analyzeAlignedImages(
      page,
      data(vectorReference),
      data(vectorCandidate),
      vectorAlignment,
      detailAnalysisOptions(options),
    );
    const vectorClassification = classifyAnalyzedVisualMetrics(
      vectorCoarse,
      vectorDetail,
      options,
    );
    if (!vectorClassification.passed) {
      throw new Error(
        `declared vector matrix did not survive aligned raster analysis: ${
          JSON.stringify(vectorClassification)
        }`,
      );
    }
    const wrongScriptCandidate = viewportReference.replace(
      '<tspan baseline-shift="sub" font-size="12">3</tspan><tspan baseline-shift="super" font-size="12">+</tspan>',
      '<tspan baseline-shift="super" font-size="12">3+</tspan>',
    );
    const wrongScriptCoarse = await analyzeAlignedImages(
      page,
      data(viewportReference),
      data(wrongScriptCandidate),
      {
        scale: 1,
        dx: 0,
        dy: 0,
        referenceWidth: 120,
        referenceHeight: 80,
        chemsemaWidth: 120,
        chemsemaHeight: 80,
      },
      options,
    );
    const wrongScriptDetail = await analyzeAlignedImages(
      page,
      data(viewportReference),
      data(wrongScriptCandidate),
      {
        scale: 1,
        dx: 0,
        dy: 0,
        referenceWidth: 120,
        referenceHeight: 80,
        chemsemaWidth: 120,
        chemsemaHeight: 80,
      },
      {
        ...options,
        analysisScale: options.detailAnalysisScale,
        tolerance: options.detailTolerance,
        tileSize: options.detailLocalWindow,
        halo: options.detailLocalWindow,
        localWindow: options.detailLocalWindow,
        localStride: options.detailLocalStride,
        minimumWindowInk: options.detailMinimumWindowInk,
      },
    );
    const wrongScriptClassification = classifyAnalyzedVisualMetrics(
      wrongScriptCoarse,
      wrongScriptDetail,
      options,
    );
    if (
      wrongScriptClassification.passed
      || !wrongScriptClassification.reasons.includes("detail-displaced-component")
    ) {
      throw new Error(
        `chemical script displacement escaped the gate: ${
          JSON.stringify(wrongScriptClassification)
        }`,
      );
    }
    const alignment = { scale: 1, dx: 0, dy: 0 };
    const small = await analyzeAlignedImages(page, data(svg(128, 96, "M 105 40 L 120 40")), data(svg(128, 96, "")), alignment, options);
    const distantCorrectDetail = "M 1000 900 L 1800 900 M 1400 500 L 1400 1300";
    const large = await analyzeAlignedImages(
      page,
      data(svg(2048, 1536, "M 105 40 L 120 40", distantCorrectDetail)),
      data(svg(2048, 1536, "", distantCorrectDetail)),
      alignment,
      options,
    );
    const identical = await analyzeAlignedImages(page, data(svg(2048, 1536, "")), data(svg(2048, 1536, "")), alignment, options);
    const areaDelta = Math.abs(small.largestMissing.area - large.largestMissing.area);
    const spanDelta = Math.abs(small.largestMissing.span - large.largestMissing.span);
    if (areaDelta > 0.01 || spanDelta > 0.01 || small.passed !== large.passed) {
      throw new Error(`size-independence regression: ${JSON.stringify({ small, large })}`);
    }
    if (!identical.passed) throw new Error(`identical-image regression: ${JSON.stringify(identical)}`);
    const syntheticDetail = {
      detailFeatures: {
        compactDefectCount: options.maxRepeatedMicroDefects + 1,
        componentCountDelta: options.maxComponentCountDelta + 1,
        enclosedSmallComponentDimensionDelta:
          options.maxEnclosedSmallComponentDimensionDelta + 0.25,
      },
      largestMissing: { area: options.maxRepeatedMicroDefectArea },
      largestExtra: { area: options.maxRepeatedMicroDefectArea },
      local: { referenceCoverage: 1, candidateCoverage: 1 },
      topDefects: [],
    };
    const expectedDetailReasons = [
      "detail-enclosed-small-component-dimension",
      "detail-repeated-micro-defects",
    ];
    const actualDetailReasons = detailGateReasons(syntheticDetail, options);
    if (JSON.stringify(actualDetailReasons) !== JSON.stringify(expectedDetailReasons)) {
      throw new Error(`detail-classifier regression: ${JSON.stringify(actualDetailReasons)}`);
    }
    const displacedDetail = {
      detailFeatures: {
        compactDefectCount: 2,
        componentCountDelta: 0,
        enclosedSmallComponentDimensionDelta: 0,
        displacedDefectPairs: [{
          relation: "synthetic-moved-component",
        }],
      },
      largestMissing: { area: 12 },
      largestExtra: { area: 12 },
      local: { referenceCoverage: 0.8, candidateCoverage: 0.8 },
      topDefects: [
        {
          kind: "missing",
          area: 12,
          box: { x: 10, y: 20, width: 5, height: 8 },
        },
        {
          kind: "extra",
          area: 11.5,
          box: { x: 10, y: 10, width: 5, height: 8 },
        },
      ],
    };
    const displacedReasons = detailGateReasons(displacedDetail, options);
    if (JSON.stringify(displacedReasons) !== '["detail-displaced-component"]') {
      throw new Error(
        `displaced-component classifier regression: ${JSON.stringify(displacedReasons)}`,
      );
    }
    console.log(JSON.stringify({
      passed: true,
      areaDelta,
      spanDelta,
      defectVerdict: small.passed,
      detailReasons: actualDetailReasons,
      cropStableFloor: oldCropFloor.recordsSha256,
      outsideCropRejected,
      clippedOverflowInk: clippedOverflowMetrics.totals.candidateInk,
    }));
  } finally {
    await browser.close();
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log("Usage: node scripts/public-cdxml-visual-gate.mjs [--gallery dir] [--out report.json] [--passed-gallery html] [--cohort name] [--cohort-ledger file] [--only text] [--limit n] [--jobs n] [--allow-dirty-gallery] [--allow-stale-gallery] [--report-only] [--strict-original-338] [--gate-definition-upgrade]");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --reuse-report report.json [--gallery dir] [--passed-gallery html]");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --gallery dir --baseline-report report.json --out report.json");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --self-test");
    return;
  }
  validateOptions(options);
  const strictConfigurationErrors = strictOriginal338ConfigurationErrors(options);
  if (strictConfigurationErrors.length) {
    throw new Error(
      `Invalid --strict-original-338 configuration: ${strictConfigurationErrors.join("; ")}`,
    );
  }
  const upgradeConfigurationErrors = gateDefinitionUpgradeConfigurationErrors(options);
  if (upgradeConfigurationErrors.length) {
    throw new Error(
      `Invalid --gate-definition-upgrade configuration: ${upgradeConfigurationErrors.join("; ")}`,
    );
  }
  if (options.strictOriginal338 || options.gateDefinitionUpgrade) {
    options.cohort = "original-338";
  }
  if (options.selfTest) {
    await runSelfTest(options);
    return;
  }

  const galleryDir = path.resolve(options.gallery);
  const manifestPath = path.join(galleryDir, "manifest.json");
  const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
  const currentProvenance = collectCurrentGalleryProvenance(manifest.provenance);
  const provenanceErrors = currentProvenance
    ? provenanceMismatches(manifest.provenance, currentProvenance)
    : ["missing-or-unsupported-provenance"];
  if (manifest.provenance?.repository?.dirty && !options.allowDirtyGallery) {
    provenanceErrors.push("dirty-gallery");
  }
  if (provenanceErrors.length && !options.allowStaleGallery) {
    throw new Error(
      `Public CDXML gallery provenance is invalid (${[...new Set(provenanceErrors)].join(", ")}). `
      + "Regenerate the gallery from the current clean repository, or use "
      + "--allow-stale-gallery only for an explicitly non-release diagnostic run.",
    );
  }
  if (options.reuseReport) {
    const report = JSON.parse(await fs.readFile(path.resolve(options.reuseReport), "utf8"));
    const reuseErrors = await reuseReportCompatibilityErrors(
      report,
      manifest,
      galleryDir,
      options,
    );
    if (reuseErrors.length) {
      throw new Error(`Cannot reuse visual-gate report: ${reuseErrors.join("; ")}`);
    }
    console.log(JSON.stringify(await writePassedGallery(
      manifest,
      report,
      galleryDir,
      options.passedGallery,
    )));
    return;
  }
  let items = manifest.items.filter((item) => !["expected-reject", "skipped"].includes(item.status));
  let cohortSelection = null;
  if (options.cohort) {
    const ledgerPath = path.resolve(options.cohortLedger);
    const ledger = JSON.parse(await fs.readFile(ledgerPath, "utf8"));
    const selection = selectVisualGateCohort(items, ledger, options.cohort);
    if (!selection.expected) {
      throw new Error(`Cohort ${options.cohort} is empty or absent in ${ledgerPath}`);
    }
    if (selection.missingPaths.length) {
      throw new Error(
        `${selection.missingPaths.length} cohort cases are missing from the gallery. `
        + `First missing case: ${selection.missingPaths[0]}`,
      );
    }
    items = selection.items;
    cohortSelection = {
      name: options.cohort,
      ledger: ledgerPath,
      expected: selection.expected,
      selected: items.length,
    };
  }
  if (options.patterns.length) {
    items = items.filter((item) => options.patterns.some((pattern) =>
      matchesPublicCdxmlCasePattern(item, pattern)));
  }
  if (Number.isFinite(options.limit)) items = items.slice(0, Math.max(0, options.limit));
  if (!items.length) throw new Error("No visual-gate cases matched the requested filters");
  const expectedRepositoryIdentity = currentProvenance?.repository?.identity;
  const expectedCliSha256 = currentProvenance?.cli?.sha256;
  const staleItems = items.filter((item) =>
    item.candidateProvenance?.repositoryIdentity !== expectedRepositoryIdentity
    || item.candidateProvenance?.cliSha256 !== expectedCliSha256);
  if (staleItems.length && !options.allowStaleGallery) {
    throw new Error(
      `${staleItems.length} selected gallery items were not rendered by the current repository/CLI identity. `
      + `First stale item: ${staleItems[0].relativeCdxml}. Regenerate before running the gate.`,
    );
  }

  const baselineReport = options.baselineReport
    ? JSON.parse(await fs.readFile(path.resolve(options.baselineReport), "utf8"))
    : null;
  const evaluatePassFloor = options.strictOriginal338
    || shouldEvaluateOriginal338PassFloor(cohortSelection, items.length);
  const strictPassFloor = evaluatePassFloor
    ? JSON.parse(await fs.readFile(STRICT_PASS_FLOOR_PATH, "utf8"))
    : null;
  const passFloorDefinitionErrors = evaluatePassFloor
    ? passFloorGateDefinitionErrors(strictPassFloor, options)
    : [];
  if (options.gateDefinitionUpgrade && passFloorDefinitionErrors.length === 0) {
    throw new Error(
      "--gate-definition-upgrade is only valid while the committed pass floor uses an older definition",
    );
  }
  const strictBaselineErrors = strictOriginal338BaselineErrors(
    baselineReport,
    items,
    options,
  );
  if (strictBaselineErrors.length) {
    throw new Error(
      `Invalid --strict-original-338 baseline: ${strictBaselineErrors.join("; ")}`,
    );
  }
  const currentReferenceHashes = baselineReport || evaluatePassFloor
    ? new Map(await Promise.all(
      items.map(async (item) => [
        normalizedCasePath(item.relativeCdxml),
        await sha256File(path.resolve(galleryDir, item.reference)),
      ]),
    ))
    : new Map();
  if (baselineReport) {
    const compatibilityErrors = visualBaselineCompatibilityErrors(
      baselineReport,
      manifest.provenance,
      currentReferenceHashes,
      options,
    );
    if (compatibilityErrors.length) {
      throw new Error(
        `Incompatible --baseline-report: ${compatibilityErrors.join("; ")}`,
      );
    }
  }
  if (evaluatePassFloor && !passFloorDefinitionErrors.length) {
    const oracleErrors = protectedVisualFloorOracleErrors(
      strictPassFloor,
      currentReferenceHashes,
    );
    if (oracleErrors.length) {
      throw new Error(`Invalid protected visual floor: ${oracleErrors.join("; ")}`);
    }
  }
  const strictPassFloorErrors = options.gateDefinitionUpgrade
    ? []
    : strictOriginal338PassFloorErrors(
      strictPassFloor,
      items,
      baselineReport,
      options,
      evaluatePassFloor,
    );
  if (strictPassFloorErrors.length) {
    throw new Error(
      `Invalid --strict-original-338 pass floor: ${strictPassFloorErrors.join("; ")}`,
    );
  }
  // A same-definition baseline can safely provide cached classifications for
  // byte-identical artifacts. Changed candidates must always use their own
  // current-image registration: carrying an old candidate's translation into
  // a new render makes the classification depend on history and can turn a
  // corrected local feature into a false regression. Regression history has a
  // different lifetime and survives gate-definition upgrades.
  const sameGateDefinition = reportsUseSameGateDefinition(baselineReport, options);
  const analysisBaselineCases = sameGateDefinition
    ? new Map(baselineReport.cases.map((entry) => [
      normalizedCasePath(entry.relativeCdxml),
      entry,
    ]))
    : new Map();
  const regressionBaselineCases = regressionBaselineCasesForGate({
    evaluatePassFloor,
    passFloorDefinitionErrors,
    strictPassFloor,
    sameGateDefinition,
    baselineReport,
  });

  const browser = await launchBrowser({ headless: true });
  const context = await browser.newContext();
  const workerCount = Math.min(options.jobs, items.length);
  const pages = await Promise.all(
    Array.from({ length: workerCount }, () => context.newPage()),
  );
  let completed = 0;
  let cases;
  try {
    cases = await mapWithConcurrency(items, options.jobs, async (item, index, workerIndex) => {
      const activePage = pages[workerIndex];
      const referencePath = path.resolve(galleryDir, item.reference);
      const candidatePath = path.resolve(galleryDir, item.chemsema);
      const hashes = await artifactHashes(galleryDir, item);
      const baselineCase = analysisBaselineCases.get(
        normalizedCasePath(item.relativeCdxml),
      );
      if (baselineCase && artifactHashesEqual(baselineCase.artifactHashes, hashes)) {
        completed += 1;
        if (completed % 100 === 0 || completed === items.length) {
          console.log(`[CACHE ${completed}/${items.length}] reused unchanged visual-gate results`);
        }
        return {
          ...baselineCase,
          id: item.id,
          relativeCdxml: item.relativeCdxml,
          artifactHashes: hashes,
          cacheStatus: "reused",
        };
      }
      try {
        if (await oracleIsUnavailable(referencePath)) {
          completed += 1;
          console.log(`[${completed}/${items.length}] worker=${workerIndex + 1} UNAVAILABLE ${item.relativeCdxml}`);
          return {
            id: item.id,
            relativeCdxml: item.relativeCdxml,
            status: "unavailable",
            reason: "ChemDraw oracle is unavailable",
            artifactHashes: hashes,
            cacheStatus: "analyzed",
          };
        }
        const [referenceDataUrl, candidateDataUrl] = await Promise.all([
          fileDataUrl(referencePath),
          fileDataUrl(candidatePath),
        ]);
        const candidateViewport = await analyzeCandidateViewport(
          activePage,
          candidateDataUrl,
          options,
        );
        const currentFrameAlignment = item.alignment?.algorithm === ALIGNMENT_ALGORITHM
          ? item.alignment
          : await computeImageAlignment(
            activePage,
            referenceDataUrl,
            candidateDataUrl,
            item.sourceAlignmentPolicy ?? sourceAlignmentPolicy(
              item.format,
              await fs.readFile(item.sourceCdxml),
            ),
          );
        const alignment = currentFrameAlignment;
        const coarseMetrics = await analyzeAlignedImages(
          activePage,
          referenceDataUrl,
          candidateDataUrl,
          alignment,
          options,
        );
        // Detail analysis is also a regression floor, not only a pass
        // classifier. Run it for every comparable case so a small new defect
        // cannot hide inside an already-red coarse image.
        const detailMetrics = await analyzeAlignedImages(
          activePage,
          referenceDataUrl,
          candidateDataUrl,
          alignment,
          detailAnalysisOptions(options),
        );
        const regressionBaseline = regressionBaselineCases.get(
          normalizedCasePath(item.relativeCdxml),
        );
        const historicalAlignment = historicalDocumentAlignment(
          regressionBaseline?.regressionAlignment ?? regressionBaseline?.alignment,
          alignment,
        );
        let regressionFloor = null;
        if (
          regressionBaseline?.status === "fail"
          && historicalAlignment
        ) {
          const baselineRasterViewBox = candidateRegressionRasterViewBox(
            regressionBaseline.regressionAlignment ?? regressionBaseline.alignment,
            options.candidateRegressionRasterPadding,
          );
          const currentRasterViewBox = candidateRegressionRasterViewBox(
            alignment,
            options.candidateRegressionRasterPadding,
          );
          const sameAlignment = ["scale", "dx", "dy"].every((key) =>
            Number.isFinite(historicalAlignment[key])
              && Math.abs(historicalAlignment[key] - alignment[key]) <= 1e-9);
          const sameRasterViewBox = baselineRasterViewBox
            && currentRasterViewBox
            && ["x", "y", "width", "height"].every((key) =>
              Math.abs(baselineRasterViewBox[key] - currentRasterViewBox[key]) <= 1e-9);
          if (sameAlignment && sameRasterViewBox) {
            regressionFloor = fixedAlignmentRegressionMetrics(
              structuredClone(coarseMetrics),
              structuredClone(detailMetrics),
              options,
            );
          } else {
            if (!baselineRasterViewBox) {
              throw new Error(
                "visual gate baseline has no valid regression raster viewport",
              );
            }
            const regressionOptions = {
              ...options,
              candidateRasterViewBox: baselineRasterViewBox,
            };
            const historicalCoarseMetrics = await analyzeAlignedImages(
              activePage,
              referenceDataUrl,
              candidateDataUrl,
              historicalAlignment,
              regressionOptions,
            );
            const historicalDetailMetrics = await analyzeAlignedImages(
              activePage,
              referenceDataUrl,
              candidateDataUrl,
              historicalAlignment,
              detailAnalysisOptions(regressionOptions),
            );
            regressionFloor = fixedAlignmentRegressionMetrics(
              historicalCoarseMetrics,
              historicalDetailMetrics,
              regressionOptions,
            );
          }
        }
        const metrics = compactSpatialMetrics(applyCandidateViewportGate(
          classifyAnalyzedVisualMetrics(coarseMetrics, detailMetrics, options),
          candidateViewport,
          options,
        ));
        const analyzedCase = {
          id: item.id,
          relativeCdxml: item.relativeCdxml,
          status: metrics.passed ? "pass" : "fail",
          alignment,
          artifactHashes: hashes,
          cacheStatus: "analyzed",
          ...(regressionFloor ? { regressionFloor } : {}),
          ...metrics,
        };
        const metricErrors = visualCaseMetricContractErrors(analyzedCase, options);
        if (metricErrors.length) {
          throw new Error(`visual metric contract failed: ${metricErrors.join(", ")}`);
        }
        completed += 1;
        console.log(`[${completed}/${items.length}] worker=${workerIndex + 1} ${metrics.passed ? "PASS" : "FAIL"} ${item.relativeCdxml}`);
        return analyzedCase;
      } catch (error) {
        completed += 1;
        console.log(`[${completed}/${items.length}] worker=${workerIndex + 1} ERROR ${item.relativeCdxml}`);
        return {
          id: item.id,
          relativeCdxml: item.relativeCdxml,
          status: "error",
          error: error instanceof Error ? error.stack ?? error.message : String(error),
          artifactHashes: hashes,
          cacheStatus: "analyzed",
        };
      }
    });
  } finally {
    await browser.close();
  }

  const finalMetricErrors = cases.flatMap((entry) =>
    visualCaseMetricContractErrors(entry, options)
      .map((error) => `${entry.relativeCdxml}: ${error}`));
  if (finalMetricErrors.length) {
    throw new Error(`visual report metric contract failed: ${finalMetricErrors.join("; ")}`);
  }

  const passed = cases.filter((entry) => entry.status === "pass").length;
  const failed = cases.filter((entry) => entry.status === "fail").length;
  const errors = cases.filter((entry) => entry.status === "error").length;
  const unavailable = cases.filter((entry) => entry.status === "unavailable").length;
  const comparable = passed + failed;
  const reused = cases.filter((entry) => entry.cacheStatus === "reused").length;
  const analyzed = cases.length - reused;
  const delta = classifyBaselineChanges(cases, regressionBaselineCases);
  const continuousRegressions = classifyContinuousBaselineRegressions(
    cases,
    regressionBaselineCases,
  );
  const protectedPassRegressions = classifyPassFloorRegressions(
    cases,
    passFloorDefinitionErrors.length ? null : strictPassFloor,
  );
  const report = {
    schema: REPORT_SCHEMA,
    caseMetricsSchema: CASE_METRICS_SCHEMA,
    cacheIdentity: CACHE_IDENTITY,
    generatedAt: new Date().toISOString(),
    gallery: galleryDir,
    galleryProvenance: manifest.provenance ?? null,
    selection: {
      cohort: cohortSelection,
      patterns: options.patterns,
      limit: Number.isFinite(options.limit) ? options.limit : null,
    },
    enforcement: options.strictOriginal338
      ? {
        mode: "strict-original-338",
        cleanGalleryRequired: true,
        currentGalleryRequired: true,
        exactBaselineScopeRequired: true,
        committedAllCaseFloorRequired: true,
        zeroRegressionsRequired: true,
        zeroContinuousRegressionsRequired: true,
        cumulativePassFloorRequired: true,
        passFloor: {
          path: STRICT_PASS_FLOOR_PATH,
          schema: strictPassFloor.schema,
          minimumPassed: strictPassFloor.minimumPassed,
          source: strictPassFloor.source,
        },
      }
      : options.gateDefinitionUpgrade
        ? {
          mode: "gate-definition-upgrade-diagnostic",
          exactCohortRequired: true,
          reportOnly: true,
          committedFloorEnforcementDeferredToMigration: true,
        }
        : { mode: "standard" },
    policy: gatePolicy(options),
    summary: {
      total: cases.length,
      passed,
      failed,
      errors,
      unavailable,
      comparable,
      passRate: comparable ? passed / comparable : 0,
    },
    cache: {
      baselineReport: options.baselineReport ? path.resolve(options.baselineReport) : null,
      sameGateDefinition,
      reused,
      analyzed,
    },
    delta: {
      comparisonMode: sameGateDefinition
        ? "same-gate-definition"
        : evaluatePassFloor && !passFloorDefinitionErrors.length
          ? "committed-all-case-floor"
          : baselineReport
            ? "pass-floor-only"
            : "none",
      ...delta,
      continuousRegressions,
    },
    passFloorEvaluation: evaluatePassFloor ? {
      applicable: passFloorDefinitionErrors.length === 0,
      definitionErrors: passFloorDefinitionErrors,
    } : null,
    protectedPassRegressions,
    cases,
  };
  const outputPath = path.resolve(options.out);
  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  const passedGallery = await writePassedGallery(
    manifest,
    report,
    galleryDir,
    options.passedGallery,
  );
  console.log(JSON.stringify({
    outputPath,
    ...passedGallery,
    ...report.summary,
    cache: report.cache,
    improvements: delta.improvements.length,
    regressions: delta.regressions.length,
    continuousRegressions: continuousRegressions.length,
    protectedPassRegressions: protectedPassRegressions.length,
  }));
  const baselineMode = options.strictOriginal338 || sameGateDefinition;
  if (!options.reportOnly && (
    errors
    || protectedPassRegressions.length
    || (baselineMode
      ? delta.regressions.length || continuousRegressions.length
      : failed)
  )) {
    process.exitCode = 1;
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack ?? error.message : String(error));
    process.exit(1);
  });
}
