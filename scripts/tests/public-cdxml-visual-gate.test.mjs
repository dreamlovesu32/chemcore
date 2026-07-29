import assert from "node:assert/strict";
import test from "node:test";
import {
  boundedLocalTopologyEquivalent,
  classifyAnalyzedVisualMetrics,
  classifyPassFloorRegressions,
  detailGateReasons,
  gatePolicy,
  nearExactFixedDefectEquivalent,
  selectVisualGateCohort,
  shouldEvaluateOriginal338PassFloor,
  strictOriginal338BaselineErrors,
  strictOriginal338ConfigurationErrors,
  strictOriginal338PassFloorErrors,
} from "../public-cdxml-visual-gate.mjs";

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
    "--baseline-report is required",
  ]);
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
    minimumTopologyComponentCount: 8,
    minimumSmallTopologyComponentCount: 3,
    minimumSmallTopologyLocalCoverage: 0.7,
    maximumTopologyCandidateComponentCount: 300,
    maxTopologyCandidateCountRatio: 0.1,
    maxRelativeComponentCenterDistance: 0.02,
    maxComponentPositionDistributionDelta: 0.03,
    minSlenderDefectCoverage: 0.98,
    minSlenderDefectLocalCoverage: 0.75,
    maxSlenderDefectArea: 24,
    maxSlenderDefectSpan: 30,
    maxSlenderDefectThickness: 1,
    minBoundedLocalCoverage: 0.96,
    minBoundedLocalWindowCoverage: 0.5,
    maxBoundedLocalDefectArea: 32,
    maxBoundedLocalDefectSpan: 32,
    minBoundedRelativeComponentCoverage: 0.877,
    boundedComponentDeltaPenalty: 0.01,
    maxBoundedComponentCountDelta: 8,
    maxTightBoundedLocalDefectSpan: 20,
    minTightBoundedRelativeComponentCoverage: 0.88,
    maxTightBoundedComponentCountDelta: 5,
    minNearExactCoverage: 0.994,
    maxNearExactDefectSpan: 15,
    maxNearExactDefectArea: 18,
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

test("strict original-338 pass floor cannot be erased by choosing a degraded baseline", () => {
  const selected = Array.from({ length: 338 }, (_, index) => ({
    relativeCdxml: `source/${String(index).padStart(4, "0")}.cdxml`,
  }));
  const baseline = {
    cases: selected.map((entry) => ({ ...entry, status: "fail" })),
  };
  baseline.cases[0].status = "pass";
  const passFloor = {
    schema: "chemsema.public-cdxml-strict-pass-floor.v1",
    cohort: { name: "original-338", expected: 338 },
    minimumPassed: 2,
    protectedPasses: ["source/0000.cdxml", "source/0001.cdxml"],
  };
  assert.deepEqual(
    strictOriginal338PassFloorErrors(
      passFloor,
      selected,
      baseline,
      { strictOriginal338: true },
    ),
    ["baseline lost protected pass source/0001.cdxml"],
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

test("bounded local topology accepts small fixed-coordinate defects", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics()), true);
});

test("bounded local topology is not diluted by a large image", () => {
  const coarse = metrics({ missingSpan: 33, extraSpan: 1 });
  coarse.totals = { referenceInk: 10_000_000, candidateInk: 10_000_000 };
  assert.equal(boundedLocalTopologyEquivalent(coarse), false);
});

test("bounded local topology rejects weak relative structure agreement", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics({
    componentDelta: 6,
    relativeCoverage: 0.92,
  })), false);
});

test("bounded local topology cannot hide a missing local label cluster", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics({
    coverage: 0.97,
    missingSpan: 27,
    extraSpan: 18,
    defectArea: 209,
    localCoverage: 0,
    componentDelta: 3,
    relativeCoverage: 0.959,
  })), false);
});

test("bounded local topology enforces fixed-area defects independently of canvas size", () => {
  const coarse = metrics({ defectArea: 32.01 });
  coarse.totals = { referenceInk: 10_000_000, candidateInk: 10_000_000 };
  assert.equal(boundedLocalTopologyEquivalent(coarse), false);
});

test("very tight defects allow a small bounded component mismatch", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics({
    missingSpan: 16,
    extraSpan: 17,
    componentDelta: 5,
    relativeCoverage: 0.89,
  })), true);
});

test("near-exact fixed defects ignore sparse-window percentages", () => {
  const coarse = metrics({ coverage: 0.994, missingSpan: 15, extraSpan: 15 });
  coarse.largestMissing.area = 18;
  coarse.largestExtra.area = 18;
  assert.equal(nearExactFixedDefectEquivalent(coarse), true);
});

test("near-exact defects remain bounded independently of image size", () => {
  const coarse = metrics({ coverage: 0.9999, missingSpan: 15.01, extraSpan: 1 });
  coarse.largestMissing.area = 1;
  coarse.largestExtra.area = 1;
  coarse.totals = { referenceInk: 10_000_000, candidateInk: 10_000_000 };
  assert.equal(nearExactFixedDefectEquivalent(coarse), false);
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
