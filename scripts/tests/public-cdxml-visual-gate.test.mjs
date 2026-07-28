import assert from "node:assert/strict";
import test from "node:test";
import {
  boundedLocalTopologyEquivalent,
  classifyAnalyzedVisualMetrics,
  detailGateReasons,
  nearExactFixedDefectEquivalent,
  selectVisualGateCohort,
} from "../public-cdxml-visual-gate.mjs";

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
