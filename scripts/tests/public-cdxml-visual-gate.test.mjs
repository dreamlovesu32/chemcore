import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";
import { deflateRawSync, inflateRawSync } from "node:zlib";
import {
  applyCandidateViewportGate,
  candidateViewportGateReasons,
  classifyAnalyzedVisualMetrics,
  classifyContinuousBaselineRegressions,
  classifyPassFloorRegressions,
  defaultGatePolicy,
  detailGateReasons,
  encodeSpatialFloor,
  gateDefinitionUpgradeConfigurationErrors,
  gatePolicy,
  historicalDocumentAlignment,
  passFloorGateDefinition,
  passFloorGateDefinitionErrors,
  protectedVisualCase,
  protectedVisualCases,
  protectedVisualFloorOracleErrors,
  regressionBaselineCasesForGate,
  selectVisualGateCohort,
  shouldEvaluateOriginal338PassFloor,
  STRICT_PASS_FLOOR_SCHEMA,
  strictOriginal338BaselineErrors,
  strictOriginal338ConfigurationErrors,
  strictOriginal338PassFloorErrors,
} from "../public-cdxml-visual-gate.mjs";
import {
  passFloorMigrationErrors,
  requireCommittedRepositoryFile,
} from "../migrate-public-cdxml-pass-floor.mjs";

function maskWordsFromPoints(points) {
  const byWord = new Map();
  for (const [x, y] of points) {
    const linear = y * 48 + x;
    const word = Math.floor(linear / 32);
    const bit = linear % 32;
    byWord.set(word, ((byWord.get(word) ?? 0) | (1 << bit)) >>> 0);
  }
  return [...byWord.entries()].sort((left, right) => left[0] - right[0]);
}

function spatialCellFromPoints({ column = 0, row = 0, missing = [], extra = [] } = {}) {
  return {
    column,
    row,
    missingMaskWords: maskWordsFromPoints(missing),
    extraMaskWords: maskWordsFromPoints(extra),
  };
}

function spatialCell({
  column = 0,
  row = 0,
  missingPixels = 0,
  extraPixels = 0,
  missingX = 1,
  missingY = 1,
  extraX = 1,
  extraY = 1,
} = {}) {
  const points = (count, startX, startY) => Array.from({ length: count }, (_, index) => [
    (startX + index) % 48,
    startY + Math.floor((startX + index) / 48),
  ]);
  return spatialCellFromPoints({
    column,
    row,
    missing: points(missingPixels, missingX, missingY),
    extra: points(extraPixels, extraX, extraY),
  });
}

function spatialFloor(cells = [], detail = false) {
  return encodeSpatialFloor(cells, detail ? 4 : 2, detail ? 12 : 24);
}

function regressionAlignment() {
  return {
    algorithm: "chemdraw-declared-transform-origin-v12",
    scale: 1,
    dx: 0,
    dy: 0,
    vectorFrame: { chemsemaViewBox: { x: 0, y: 0, width: 48, height: 48 } },
  };
}

test("candidate viewport margin rejects edge clipping independently of image size", () => {
  const clipped = {
    applicable: true,
    inkPixels: 100,
    minimumMargin: 0.25,
    margins: { left: 0.25, top: 8, right: 8, bottom: 8 },
    width: 100000,
    height: 100000,
  };
  assert.deepEqual(candidateViewportGateReasons(clipped), ["candidate-viewport-ink-margin"]);
  const result = applyCandidateViewportGate({ passed: true, reasons: [] }, clipped);
  assert.equal(result.passed, false);
  assert.deepEqual(result.reasons, ["candidate-viewport-ink-margin"]);
});

test("candidate viewport margin accepts the renderer export margin", () => {
  const valid = {
    applicable: true,
    inkPixels: 100,
    minimumMargin: 7.75,
    margins: { left: 7.75, top: 8, right: 8, bottom: 8 },
  };
  assert.deepEqual(candidateViewportGateReasons(valid), []);
  assert.equal(
    applyCandidateViewportGate({ passed: true, reasons: [] }, valid).passed,
    true,
  );
});

test("complete original-338 diagnostics always evaluate the protected pass floor", () => {
  const complete = {
    name: "original-338",
    expected: 338,
    selected: 338,
  };
  assert.equal(shouldEvaluateOriginal338PassFloor(complete, 338), true);
  assert.equal(shouldEvaluateOriginal338PassFloor(complete, 337), false);
  assert.equal(
    shouldEvaluateOriginal338PassFloor({ ...complete, name: "another-cohort" }, 338),
    false,
  );
});

test("continuous regression metrics only guard cases that remain red", () => {
  const baseline = new Map([
    ["red.cdxml", { status: "fail", largestMissing: { area: 10 } }],
    ["green.cdxml", { status: "pass", largestMissing: { area: 10 } }],
    ["improved.cdxml", { status: "fail", largestMissing: { area: 10 } }],
  ]);
  const current = [
    { relativeCdxml: "red.cdxml", status: "fail", largestMissing: { area: 12 } },
    { relativeCdxml: "green.cdxml", status: "pass", largestMissing: { area: 12 } },
    { relativeCdxml: "improved.cdxml", status: "pass", largestMissing: { area: 12 } },
  ];
  assert.deepEqual(
    classifyContinuousBaselineRegressions(current, baseline).map((entry) => entry.relativeCdxml),
    ["red.cdxml"],
  );
});

test("continuous regression metrics reject deterioration even with a countervailing gain", () => {
  const baseline = new Map([
    ["tradeoff.cdxml", {
      status: "fail",
      reasons: ["old-defect"],
      largestMissing: { area: 10 },
      largestExtra: { area: 10 },
    }],
    ["pure.cdxml", {
      status: "fail",
      reasons: ["old-defect"],
      largestMissing: { area: 10 },
      largestExtra: { area: 10 },
    }],
  ]);
  const current = [
    {
      relativeCdxml: "tradeoff.cdxml",
      status: "fail",
      reasons: ["new-defect"],
      largestMissing: { area: 12 },
      largestExtra: { area: 8 },
    },
    {
      relativeCdxml: "pure.cdxml",
      status: "fail",
      reasons: ["old-defect", "new-defect"],
      largestMissing: { area: 12 },
      largestExtra: { area: 10 },
    },
  ];
  assert.deepEqual(
    classifyContinuousBaselineRegressions(current, baseline).map((entry) => entry.relativeCdxml),
    ["tradeoff.cdxml", "pure.cdxml"],
  );
});

test("continuous regression metrics ratchet fixed local windows without spatial cancellation", () => {
  const baseline = new Map([["tradeoff.cdxml", {
    status: "fail",
    reasons: ["local-reference-coverage"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ column: 0, missingPixels: 4 }),
      spatialCell({ column: 1, missingPixels: 40 }),
    ]) },
  }]]);
  const current = [{
    relativeCdxml: "tradeoff.cdxml",
    status: "fail",
    reasons: ["local-reference-coverage"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ column: 0, missingPixels: 8 }),
      spatialCell({ column: 1, missingPixels: 32 }),
    ]) },
  }];
  const regressions = classifyContinuousBaselineRegressions(current, baseline);
  assert.equal(regressions.length, 1);
  assert.deepEqual(
    regressions[0].reasons.map((entry) => entry.metric),
    ["local.spatialFloor.missingUnsupportedArea"],
  );
});

test("continuous regression metrics protect detail cells on already-red images", () => {
  const baseline = new Map([["detail.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    detail: { local: { spatialFloor: spatialFloor([
      spatialCell({ missingPixels: 4 }),
    ], true) } },
  }]]);
  const current = [{
    relativeCdxml: "detail.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    detail: { local: { spatialFloor: spatialFloor([
      spatialCell({ missingPixels: 12 }),
    ], true) } },
  }];
  const regressions = classifyContinuousBaselineRegressions(current, baseline);
  assert.equal(regressions.length, 1);
  assert.match(regressions[0].reasons[0].metric, /^detail\.local\.spatialFloor/);
});

test("continuous regression metrics reject deleted spatial floors", () => {
  const baseline = new Map([["missing-array.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor() },
  }]]);
  const regressions = classifyContinuousBaselineRegressions([{
    relativeCdxml: "missing-array.cdxml",
    status: "fail",
    reasons: ["old-defect"],
  }], baseline);
  assert.equal(regressions.length, 1);
  assert.deepEqual(regressions[0].reasons[0], {
    metric: "local.spatialFloor",
    direction: "present",
    before: "spatial-floor",
    after: null,
    tolerance: 0,
  });
});

test("removing a candidate-only extra window is an improvement", () => {
  const baseline = new Map([["extra-fixed.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ extraPixels: 8 }),
    ]) },
  }]]);
  assert.deepEqual(classifyContinuousBaselineRegressions([{
    relativeCdxml: "extra-fixed.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor() },
  }], baseline), []);
});

test("spatial occupancy prevents same-size relocation from cancelling", () => {
  const baseline = new Map([["moved.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ missingPixels: 8, missingX: 1 }),
    ]) },
  }]]);
  const regressions = classifyContinuousBaselineRegressions([{
    relativeCdxml: "moved.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ missingPixels: 8, missingX: 5 }),
    ]) },
  }], baseline);
  assert.equal(regressions.length, 1);
  assert.equal(regressions[0].reasons[0].direction, "lower");
});

test("a nearby defect relocation is accepted only when its local mismatch mass shrinks", () => {
  const baseline = new Map([["improved-move.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ missingPixels: 8, missingX: 1 }),
    ]) },
  }]]);
  const current = [{
    relativeCdxml: "improved-move.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ missingPixels: 4, missingX: 5 }),
    ]) },
  }];
  assert.deepEqual(classifyContinuousBaselineRegressions(current, baseline), []);
});

test("a smaller defect cannot spend retired mismatch mass from a distant feature", () => {
  const baseline = new Map([["distant-move.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ column: 0, missingPixels: 8 }),
    ]) },
  }]]);
  const regressions = classifyContinuousBaselineRegressions([{
    relativeCdxml: "distant-move.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCell({ column: 2, missingPixels: 4 }),
    ]) },
  }], baseline);
  assert.equal(regressions.length, 1);
  assert.equal(regressions[0].reasons[0].metric, "local.spatialFloor.missingUnsupportedArea");
});

test("sub-tolerance raster motion remains supported across a spatial cell boundary", () => {
  const baseline = new Map([["cross-cell.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCellFromPoints({ column: 0, missing: [[47, 1]] }),
    ]) },
  }]]);
  assert.deepEqual(classifyContinuousBaselineRegressions([{
    relativeCdxml: "cross-cell.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([
      spatialCellFromPoints({ column: 1, missing: [[0, 1]] }),
    ]) },
  }], baseline), []);
});

test("historical registration follows the current SVG crop in document coordinates", () => {
  const historical = {
    algorithm: "chemdraw-declared-transform-origin-v12",
    scale: 2.5,
    dx: 10,
    dy: 20,
    vectorFrame: { chemsemaViewBox: { x: -4, y: 8 } },
  };
  const current = {
    algorithm: historical.algorithm,
    scale: 2.5,
    dx: 99,
    dy: 101,
    chemsemaWidth: 70,
    chemsemaHeight: 80,
    vectorFrame: { chemsemaViewBox: { x: 6, y: -2 } },
  };
  const aligned = historicalDocumentAlignment(historical, current);
  assert.equal(aligned.dx, 35);
  assert.equal(aligned.dy, -5);
  assert.equal(aligned.chemsemaWidth, 70);
  assert.equal(aligned.vectorFrame, current.vectorFrame);
});

test("spatial occupancy distinguishes equal count, bounds, and centroid", () => {
  const baselineCell = spatialCellFromPoints({ missing: [
    [0, 0], [0, 5], [0, 10], [10, 0], [10, 5], [10, 10],
  ] });
  const currentCell = spatialCellFromPoints({ missing: [
    [0, 0], [4, 4], [4, 6], [6, 4], [6, 6], [10, 10],
  ] });
  const baseline = new Map([["same-statistics.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([baselineCell]) },
  }]]);
  const regressions = classifyContinuousBaselineRegressions([{
    relativeCdxml: "same-statistics.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: spatialFloor([currentCell]) },
  }], baseline);
  assert.equal(regressions.length, 1);
  assert.equal(regressions[0].reasons[0].metric, "local.spatialFloor.missingUnsupportedArea");
});

test("small defects split across many cells cannot evade the absolute budget", () => {
  const baselineCells = Array.from({ length: 100 }, (_, index) =>
    spatialCell({ column: index + 100, missingPixels: 1 }));
  const currentCells = Array.from({ length: 100 }, (_, index) =>
    spatialCell({ column: index, missingPixels: 1 }));
  const baseline = new Map([["split.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    detail: { local: { spatialFloor: spatialFloor(baselineCells, true) } },
  }]]);
  const regressions = classifyContinuousBaselineRegressions([{
    relativeCdxml: "split.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    detail: { local: { spatialFloor: spatialFloor(currentCells, true) } },
  }], baseline);
  assert.equal(regressions.length, 1);
  assert.equal(
    regressions[0].reasons[0].metric,
    "detail.local.spatialFloor.missingUnsupportedArea",
  );
});

test("spatial floor compression bytes are not part of canonical identity", () => {
  const baselineFloor = spatialFloor([spatialCell({ missingPixels: 8 })]);
  const currentFloor = structuredClone(baselineFloor);
  const raw = inflateRawSync(Buffer.from(currentFloor.data, "base64"));
  const compressed = deflateRawSync(raw, { level: 1 });
  currentFloor.data = compressed.toString("base64");
  currentFloor.compressedBytes = compressed.length;
  const baseline = new Map([["compression.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: baselineFloor },
  }]]);
  assert.deepEqual(classifyContinuousBaselineRegressions([{
    relativeCdxml: "compression.cdxml",
    status: "fail",
    reasons: ["old-defect"],
    local: { spatialFloor: currentFloor },
  }], baseline), []);
});

test("continuous regression metrics cannot disappear to evade the floor", () => {
  const baseline = new Map([["red.cdxml", {
    status: "fail",
    reasons: ["old-defect"],
    largestMissing: { area: 10 },
  }]]);
  const current = [{
    relativeCdxml: "red.cdxml",
    status: "fail",
    reasons: ["old-defect"],
  }];
  const regressions = classifyContinuousBaselineRegressions(current, baseline);
  assert.equal(regressions.length, 1);
  assert.equal(regressions[0].reasons[0].direction, "present");
});

test("protected visual cases keep only authoritative all-case floor data", () => {
  const coarseSpatialFloor = spatialFloor([
    spatialCell({ column: 1, row: 1, missingPixels: 4 }),
    spatialCell({ column: 2, row: 1, missingPixels: 4 }),
  ]);
  const floorCase = protectedVisualCase({
    relativeCdxml: "source\\red.cdxml",
    status: "fail",
    artifactHashes: { reference: "oracle", candidate: "candidate" },
    reasons: ["b", "a", "b"],
    referenceCoverage: 0.9,
    candidateCoverage: 0.8,
    largestMissing: { area: 10, span: 4, box: { x: 1 } },
    local: { spatialFloor: coarseSpatialFloor },
    error: "must not persist",
  });
  assert.deepEqual(floorCase, {
    relativeCdxml: "source/red.cdxml",
    status: "fail",
    artifactHashes: { reference: "oracle" },
    analysisLayers: { coarse: true, detail: false },
    reasons: ["a", "b"],
    referenceCoverage: 0.9,
    candidateCoverage: 0.8,
    largestMissing: { area: 10, span: 4 },
    local: { spatialFloor: coarseSpatialFloor },
  });
  assert.deepEqual(protectedVisualFloorOracleErrors({
    protectedCases: [floorCase],
  }, new Map([["source/red.cdxml", "changed-oracle"]])), [
    "protected ChemDraw oracle changed for source/red.cdxml",
  ]);
  assert.deepEqual(protectedVisualCases([
    { relativeCdxml: "a-b.cdxml", status: "pass", artifactHashes: { reference: "b" } },
    { relativeCdxml: "a_b.cdxml", status: "pass", artifactHashes: { reference: "a" } },
  ]).map((entry) => entry.relativeCdxml), ["a_b.cdxml", "a-b.cdxml"]);
});

test("strict regression history comes from the committed floor, never a chosen cache report", () => {
  const baseline = regressionBaselineCasesForGate({
    evaluatePassFloor: true,
    passFloorDefinitionErrors: [],
    strictPassFloor: {
      protectedCases: [{ relativeCdxml: "case.cdxml", status: "pass" }],
    },
    sameGateDefinition: true,
    baselineReport: {
      cases: [{ relativeCdxml: "case.cdxml", status: "fail" }],
    },
  });
  assert.equal(baseline.get("case.cdxml").status, "pass");
});

function metrics({
  coverage = 0.98,
  missingSpan = 18,
  extraSpan = 18,
  componentDelta = 3,
  relativeCoverage = 0.95,
  defectArea = 20,
  localCoverage = 0.8,
} = {}) {
  return {
    passed: false,
    reasons: ["synthetic-coarse-failure"],
    referenceCoverage: coverage,
    candidateCoverage: coverage,
    local: {
      referenceCoverage: localCoverage,
      candidateCoverage: localCoverage,
    },
    largestMissing: { area: defectArea, span: missingSpan },
    largestExtra: { area: defectArea, span: extraSpan },
    detailFeatures: {
      componentCountDelta: componentDelta,
      relativeComponentMatchCoverage: relativeCoverage,
    },
  };
}

test("strict original-338 mode rejects diagnostic escape hatches", () => {
  const errors = strictOriginal338ConfigurationErrors({
    strictOriginal338: true,
    allowDirtyGallery: true,
    allowStaleGallery: true,
    reportOnly: true,
    patterns: ["one-case"],
    limit: 1,
    cohort: "other",
    baselineReport: null,
  });
  assert.deepEqual(errors, [
    "--allow-dirty-gallery is forbidden",
    "--allow-stale-gallery is forbidden",
    "--report-only is forbidden",
    "--only is forbidden",
    "--limit is forbidden",
    "--cohort must be original-338",
  ]);
});

test("gate-definition upgrade diagnostics cannot act as a release gate", () => {
  const errors = gateDefinitionUpgradeConfigurationErrors({
    gateDefinitionUpgrade: true,
    strictOriginal338: true,
    reportOnly: false,
    reuseReport: "cached.json",
    baselineReport: "baseline.json",
    patterns: ["one-case"],
    limit: 1,
    cohort: "other",
  });
  assert.deepEqual(errors, [
    "--strict-original-338 is forbidden",
    "--report-only is required",
    "--reuse-report is forbidden",
    "--baseline-report is forbidden",
    "--only is forbidden",
    "--limit is forbidden",
    "--cohort must be original-338",
  ]);
  assert.deepEqual(gateDefinitionUpgradeConfigurationErrors({
    gateDefinitionUpgrade: true,
    strictOriginal338: false,
    reportOnly: true,
    patterns: [],
    cohort: "original-338",
  }), []);
});

test("strict original-338 mode requires the exact same 338 paths across gate upgrades", () => {
  const cases = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
  }));
  const options = {
    strictOriginal338: true,
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
    maxComponentCountDelta: 1,
    maxEnclosedSmallComponentDimensionDelta: 2.75,
    maxRepeatedMicroDefects: 20,
    maxRepeatedMicroDefectArea: 5,
    minRepeatedMicroCoverage: 0.75,
    maxRelativeComponentCenterDistance: 0.02,
  };
  const baseline = {
    // Regression history is intentionally older than the current analysis
    // definition. It remains authoritative for pass -> fail transitions, but
    // cannot be reused as an analysis cache or alignment lock.
    cacheIdentity: "chemsema-public-cdxml-visual-gate-cache-v12",
    selection: {
      cohort: { name: "original-338", expected: 338, selected: 338 },
    },
    policy: { ...gatePolicy(options), alignment: "retired alignment policy" },
    cases,
  };
  assert.deepEqual(strictOriginal338BaselineErrors(baseline, cases, options), []);
  const changed = structuredClone(cases);
  changed[337].relativeCdxml = "source/replacement.cdxml";
  assert.match(
    strictOriginal338BaselineErrors(baseline, changed, options).join("\n"),
    /baseline is missing selected path source\/replacement\.cdxml/,
  );
});

test("strict original-338 pass floor is authoritative over a degraded cache baseline", () => {
  const options = { strictOriginal338: true };
  const selected = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
  }));
  const baseline = {
    cases: selected.map((entry) => ({ ...entry, status: "fail" })),
  };
  baseline.cases[0].status = "pass";
  const passFloor = {
    schema: STRICT_PASS_FLOOR_SCHEMA,
    gateDefinition: passFloorGateDefinition(options),
    cohort: { name: "original-338", expected: 338 },
    minimumPassed: 2,
    protectedPasses: ["source/0000.cdxml", "source/0001.cdxml"],
    protectedCases: selected.map((entry, index) => index < 2 ? {
      ...entry,
      status: "pass",
      artifactHashes: { reference: `reference-${index}` },
    } : protectedVisualCase({
      ...entry,
      status: "fail",
      alignment: regressionAlignment(),
      artifactHashes: { reference: `reference-${index}` },
      reasons: ["synthetic-failure"],
      referenceCoverage: 0.9,
      candidateCoverage: 0.9,
      local: {
        referenceCoverage: 0.8,
        candidateCoverage: 0.8,
        spatialFloor: spatialFloor(),
      },
      largestMissing: { area: 2, span: 2 },
      largestExtra: { area: 2, span: 2 },
      totals: { missingInk: 0, extraInk: 0 },
      domain: { left: 0, top: 0, right: 48, bottom: 48 },
      detailFeatures: {
        compactDefectCount: 0,
        componentMatchCoverage: 0.9,
        relativeComponentMatchCoverage: 0.9,
        componentPositionDistributionDelta: 0.01,
        independentComponentCountDelta: 0,
        unmatchedReferenceComponentCount: 0,
        unmatchedCandidateComponentCount: 0,
        smallComponentDimensionDelta: 0,
        enclosedSmallComponentDimensionDelta: 0,
        maximumMatchedCenterDistance: 0,
        maximumMatchedDimensionDelta: 0,
      },
      detail: {
        totals: { missingInk: 0, extraInk: 0 },
        domain: { left: 0, top: 0, right: 48, bottom: 48 },
        local: {
          referenceCoverage: 0.8,
          candidateCoverage: 0.8,
          spatialFloor: spatialFloor([], true),
        },
        largestMissing: { area: 2, span: 2 },
        largestExtra: { area: 2, span: 2 },
        detailFeatures: {
          compactDefectCount: 0,
          componentMatchCoverage: 0.9,
          relativeComponentMatchCoverage: 0.9,
          componentPositionDistributionDelta: 0.01,
          independentComponentCountDelta: 0,
        },
      },
    })),
  };
  assert.deepEqual(
    strictOriginal338PassFloorErrors(
      passFloor,
      selected,
      baseline,
      options,
    ),
    [],
  );
  const missingSpatialFloor = structuredClone(passFloor);
  delete missingSpatialFloor.protectedCases[2].local.spatialFloor;
  assert.match(
    strictOriginal338PassFloorErrors(
      missingSpatialFloor,
      selected,
      baseline,
      options,
    ).join("\n"),
    /missing local\.spatialFloor/,
  );
  const corruptSpatialFloor = structuredClone(passFloor);
  corruptSpatialFloor.protectedCases[2].local.spatialFloor.data = "corrupt";
  assert.match(
    strictOriginal338PassFloorErrors(
      corruptSpatialFloor,
      selected,
      baseline,
      options,
    ).join("\n"),
    /invalid local\.spatialFloor/,
  );
  assert.deepEqual(classifyPassFloorRegressions(baseline.cases, passFloor), [{
    relativeCdxml: "source/0001.cdxml",
    before: "protected-pass",
    after: "fail",
  }]);
});

test("complete original-338 diagnostics validate the floor outside strict mode", () => {
  const selected = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
  }));
  const passFloor = {
    schema: STRICT_PASS_FLOOR_SCHEMA,
    gateDefinition: passFloorGateDefinition(),
    cohort: { name: "original-338", expected: 338 },
    minimumPassed: 337,
    protectedPasses: selected.slice(1).map((entry) => entry.relativeCdxml),
    protectedCases: selected.map((entry, index) => ({
      ...entry,
      status: index === 0 ? "fail" : "pass",
      artifactHashes: { reference: `reference-${index}` },
      ...(index === 0 ? { analysisLayers: { coarse: true, detail: true } } : {}),
      ...(index === 0 ? { regressionAlignment: regressionAlignment() } : {}),
    })),
  };
  assert.match(
    strictOriginal338PassFloorErrors(
      passFloor,
      selected,
      null,
      { strictOriginal338: false },
      true,
    ).join("\n"),
    /invalid metrics/,
  );
});

test("pass floors are bound to one exact gate definition", () => {
  const floor = {
    schema: STRICT_PASS_FLOOR_SCHEMA,
    gateDefinition: passFloorGateDefinition(),
  };
  assert.deepEqual(passFloorGateDefinitionErrors(floor), []);
  floor.gateDefinition.cacheIdentity = "retired-gate";
  assert.deepEqual(passFloorGateDefinitionErrors(floor), [
    "pass floor was established by a different gate definition",
  ]);
});

test("pass-floor migration requires zero same-gate candidate regressions", () => {
  const cases = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
    status: index === 0 ? "fail" : "pass",
    artifactHashes: { reference: `reference-${index}`, candidate: `old-${index}` },
  }));
  const definition = passFloorGateDefinition();
  const report = (entries, repository = { head: "old-commit", identity: "old-identity" }) => ({
    cacheIdentity: definition.cacheIdentity,
    policy: defaultGatePolicy(),
    galleryProvenance: { repository },
    cases: entries,
  });
  const improved = structuredClone(cases);
  improved[0].status = "pass";
  assert.deepEqual(passFloorMigrationErrors(report(cases), report(improved)), []);
  improved[1].status = "fail";
  assert.deepEqual(passFloorMigrationErrors(report(cases), report(improved)), [
    "current candidate has 1 same-gate pass-to-fail regressions",
  ]);

  cases[2].artifactHashes.candidate = "a".repeat(64);
  const continuouslyWorse = structuredClone(cases);
  cases[2].status = "fail";
  continuouslyWorse[2].status = "fail";
  cases[2].totals = { missingInk: 10, extraInk: 10 };
  cases[2].detail = { totals: { missingInk: 20, extraInk: 20 } };
  continuouslyWorse[2].totals = { missingInk: 9, extraInk: 10 };
  continuouslyWorse[2].detail = { totals: { missingInk: 19, extraInk: 20 } };
  cases[2].largestMissing = { area: 10, span: 8 };
  continuouslyWorse[2].largestMissing = { area: 12, span: 8 };
  assert.deepEqual(
    passFloorMigrationErrors(report(cases), report(continuouslyWorse)),
    ["current candidate has 1 continuous metric regressions"],
  );
  continuouslyWorse[2].artifactHashes.candidate = "b".repeat(64);
  const reviewedRendererMigration = {
    schema: "chemsema.public-cdxml-reviewed-renderer-migration.v1",
    rule: "verified-renderer-rule",
    fromRepository: { head: "old-commit", identity: "old-identity" },
    toRepository: { head: "new-commit", identity: "new-identity" },
    evidence: {
      rendererCommit: "renderer-commit",
      probe: "scripts/probe.mjs",
      rules: "docs/rules.md#verified-renderer-rule",
    },
    cases: [{
      relativeCdxml: "source/0002.cdxml",
      previousCandidateSha256: "a".repeat(64),
      currentCandidateSha256: "b".repeat(64),
    }],
  };
  assert.deepEqual(passFloorMigrationErrors(
    report(cases),
    report(continuouslyWorse, { head: "new-commit", identity: "new-identity" }),
    null,
    null,
    reviewedRendererMigration,
  ), []);
  continuouslyWorse[2].regressionFloor = {
    totals: { missingInk: 11, extraInk: 10 },
    detail: { totals: { missingInk: 19, extraInk: 20 } },
  };
  assert.match(passFloorMigrationErrors(
    report(cases),
    report(continuouslyWorse, { head: "new-commit", identity: "new-identity" }),
    null,
    null,
    reviewedRendererMigration,
  ).join("\n"), /increases coarse mismatch mass.*20 -> 21/);
  delete continuouslyWorse[2].regressionFloor;
  reviewedRendererMigration.cases[0].relativeCdxml = "source/0003.cdxml";
  assert.match(passFloorMigrationErrors(
    report(cases),
    report(continuouslyWorse, { head: "new-commit", identity: "new-identity" }),
    null,
    null,
    reviewedRendererMigration,
  ).join("\n"), /does not exactly cover continuous regressions/);
});

test("pass-floor migration is bound to the old floor repository and cannot shrink", () => {
  const cases = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
    status: index === 0 ? "fail" : "pass",
    artifactHashes: { reference: `reference-${index}` },
  }));
  const report = (entries, identity = "old-identity") => ({
    policy: defaultGatePolicy(),
    galleryProvenance: {
      repository: { head: "old-commit", identity },
    },
    cases: entries,
  });
  const oldFloor = {
    minimumPassed: 337,
    source: { commit: "old-commit", repositoryIdentity: "old-identity" },
    protectedPasses: cases.slice(1).map((entry) => entry.relativeCdxml),
    protectedCases: cases,
  };
  assert.deepEqual(
    passFloorMigrationErrors(report(cases), report(cases), oldFloor),
    [],
  );
  assert.match(
    passFloorMigrationErrors(
      report(cases, "wrong-identity"),
      report(cases),
      oldFloor,
    ).join("\n"),
    /not the repository state protected by the old floor/,
  );
  const degraded = structuredClone(cases);
  degraded[1].status = "fail";
  assert.match(
    passFloorMigrationErrors(report(cases), report(degraded), oldFloor).join("\n"),
    /lower the protected pass floor/,
  );
});

test("pass-floor migration permits only an explicit gate-definition retirement", () => {
  const frozenCases = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
    status: index === 0 ? "fail" : "pass",
    artifactHashes: { reference: `reference-${index}` },
  }));
  const report = {
    policy: defaultGatePolicy(),
    galleryProvenance: {
      repository: { head: "old-commit", identity: "old-identity" },
    },
    cases: frozenCases,
  };
  const definition = passFloorGateDefinition();
  const oldFloor = {
    gateDefinition: { cacheIdentity: "retired-gate" },
    minimumPassed: 338,
    source: { commit: "old-commit", repositoryIdentity: "old-identity" },
    protectedPasses: frozenCases.map((entry) => entry.relativeCdxml),
    protectedCases: frozenCases.map((entry) => ({ ...entry, status: "pass" })),
  };
  const retirements = {
    schema: "chemsema.public-cdxml-gate-definition-retirements.v1",
    fromCacheIdentity: "retired-gate",
    toCacheIdentity: definition.cacheIdentity,
    reason: "retired-size-dependent-rule",
    paths: ["source/0000.cdxml"],
  };
  assert.match(
    passFloorMigrationErrors(report, report, oldFloor).join("\n"),
    /without review/,
  );
  assert.deepEqual(
    passFloorMigrationErrors(report, report, oldFloor, retirements),
    [],
  );
});

test("review artifacts must be clean tracked files inside the repository", async () => {
  await assert.doesNotReject(requireCommittedRepositoryFile(
    path.resolve("package.json"),
    "review artifact",
  ));
  await assert.rejects(
    requireCommittedRepositoryFile(
      path.resolve("tmp", "untracked-review-artifact.json"),
      "review artifact",
    ),
    /exactly match a committed repository file/,
  );
  await assert.rejects(
    requireCommittedRepositoryFile(
      path.resolve("..", "outside-review-artifact.json"),
      "review artifact",
    ),
    /must be stored inside the repository/,
  );
});

test("strict original-338 pass floor reports protected pass regressions independently", () => {
  const current = [
    { relativeCdxml: "source/0000.cdxml", status: "pass" },
    { relativeCdxml: "source/0001.cdxml", status: "fail" },
  ];
  const passFloor = {
    protectedPasses: ["source/0000.cdxml", "source/0001.cdxml", "source/0002.cdxml"],
  };
  assert.deepEqual(classifyPassFloorRegressions(current, passFloor), [
    {
      relativeCdxml: "source/0001.cdxml",
      before: "protected-pass",
      after: "fail",
    },
    {
      relativeCdxml: "source/0002.cdxml",
      before: "protected-pass",
      after: "missing",
    },
  ]);
});

test("strong global pixel agreement cannot erase fine component evidence", () => {
  const coarse = metrics({
    coverage: 0.999,
    missingSpan: 4,
    extraSpan: 4,
    defectArea: 2,
    componentDelta: 0,
    localCoverage: 0.98,
  });
  coarse.passed = true;
  coarse.reasons = [];
  const detail = {
    local: { referenceCoverage: 0.9, candidateCoverage: 0.9 },
    largestMissing: { area: 1, span: 2 },
    largestExtra: { area: 1, span: 2 },
    topDefects: [],
    settings: {},
    detailFeatures: {
      compactDefectCount: 0,
      componentCountDelta: 2,
      independentComponentCountDelta: 2,
      enclosedSmallComponentDimensionDelta: 0,
      referenceComponentCount: 7,
      candidateComponentCount: 5,
      componentPositionDistributionDelta: 1,
    },
  };
  const result = classifyAnalyzedVisualMetrics(coarse, detail);
  assert.equal(result.passed, false);
  assert.deepEqual(result.reasons, ["detail-component-count"]);
});

test("global image size cannot bypass a failed fixed-window coarse gate", () => {
  const coarse = metrics({
    coverage: 0.999999,
    missingSpan: 13,
    extraSpan: 1,
    defectArea: 9,
    localCoverage: 0.74,
  });
  coarse.totals = {
    referenceInk: 100_000_000,
    candidateInk: 100_000_000,
  };
  const detail = {
    local: { referenceCoverage: 1, candidateCoverage: 1 },
    largestMissing: { area: 0, span: 0 },
    largestExtra: { area: 0, span: 0 },
    topDefects: [],
    detailFeatures: {
      independentComponentCountDelta: 0,
      enclosedSmallComponentDimensionDelta: 0,
      compactDefectCount: 0,
      displacedDefectPairs: [],
    },
  };
  const classified = classifyAnalyzedVisualMetrics(coarse, detail);
  assert.equal(classified.passed, false);
  assert.deepEqual(classified.reasons, ["synthetic-coarse-failure"]);
});

test("spatially supported raster seams do not become independent component evidence", () => {
  const detail = {
    local: { referenceCoverage: 0.99, candidateCoverage: 0.99 },
    largestMissing: { area: 1, span: 2 },
    largestExtra: { area: 1, span: 2 },
    detailFeatures: {
      compactDefectCount: 0,
      componentCountDelta: 3,
      independentComponentCountDelta: 0,
      enclosedSmallComponentDimensionDelta: 0,
      displacedDefectPairs: [],
    },
  };
  assert.deepEqual(detailGateReasons(detail), []);
});

test("zero-tolerance repeated join defects are rejected as a detail pattern", () => {
  const detail = {
    local: {
      referenceCoverage: 0.78,
      candidateCoverage: 0.78,
    },
    largestMissing: { area: 4.8, span: 15.9 },
    largestExtra: { area: 4.5, span: 14.5 },
    detailFeatures: {
      compactDefectCount: 41,
      componentCountDelta: 0,
      independentComponentCountDelta: 0,
      enclosedSmallComponentDimensionDelta: 0,
    },
  };
  assert.deepEqual(detailGateReasons(detail), ["detail-repeated-micro-defects"]);
});

test("component-supported matching defects are rejected as one displaced detail", () => {
  const detail = {
    local: { referenceCoverage: 1, candidateCoverage: 1 },
    largestMissing: { area: 12, span: 6 },
    largestExtra: { area: 11, span: 6 },
    detailFeatures: {
      compactDefectCount: 0,
      componentCountDelta: 0,
      independentComponentCountDelta: 0,
      enclosedSmallComponentDimensionDelta: 0,
      displacedDefectPairs: [{
        relation: "unmatched",
      }],
    },
    topDefects: [
      {
        kind: "missing",
        area: 12,
        box: { x: 10, y: 10, width: 5, height: 6 },
      },
      {
        kind: "extra",
        area: 11,
        box: { x: 10, y: 17, width: 5, height: 6 },
      },
    ],
  };
  assert.deepEqual(detailGateReasons(detail), ["detail-displaced-component"]);
});

test("lookalike defects without component identity are not paired as one displacement", () => {
  const detail = {
    local: { referenceCoverage: 1, candidateCoverage: 1 },
    largestMissing: { area: 12, span: 6 },
    largestExtra: { area: 11, span: 6 },
    detailFeatures: {
      compactDefectCount: 0,
      componentCountDelta: 0,
      independentComponentCountDelta: 0,
      enclosedSmallComponentDimensionDelta: 0,
    },
    topDefects: [
      {
        kind: "missing",
        area: 12,
        box: { x: 10, y: 10, width: 5, height: 6 },
      },
      {
        kind: "extra",
        area: 11,
        box: { x: 10, y: 17, width: 5, height: 6 },
      },
    ],
  };
  assert.deepEqual(detailGateReasons(detail), []);
});

test("cohort selection is exact and reports missing gallery members", () => {
  const ledger = {
    cases: [
      { cohort: "original", relativeCdxml: "source/a.cdxml" },
      { cohort: "original", relativeCdxml: "source\\b.cdxml" },
      { cohort: "extra", relativeCdxml: "source/c.cdxml" },
    ],
  };
  const result = selectVisualGateCohort([
    { id: "a", relativeCdxml: "source/a.cdxml" },
    { id: "c", relativeCdxml: "source/c.cdxml" },
  ], ledger, "original");
  assert.deepEqual(result.items.map((item) => item.id), ["a"]);
  assert.equal(result.expected, 2);
  assert.deepEqual(result.missingPaths, ["source/b.cdxml"]);
});
