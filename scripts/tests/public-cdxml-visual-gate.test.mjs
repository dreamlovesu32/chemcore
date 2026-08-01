import assert from "node:assert/strict";
import test from "node:test";
import {
  applyCandidateViewportGate,
  boundedLocalTopologyEquivalent,
  candidateViewportGateReasons,
  classifyAnalyzedVisualMetrics,
  classifyContinuousBaselineRegressions,
  classifyPassFloorRegressions,
  defaultGatePolicy,
  detailGateReasons,
  gatePolicy,
  nearExactFixedDefectEquivalent,
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
import { passFloorMigrationErrors } from "../migrate-public-cdxml-pass-floor.mjs";

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
  const floorCase = protectedVisualCase({
    relativeCdxml: "source\\red.cdxml",
    status: "fail",
    artifactHashes: { reference: "oracle", candidate: "candidate" },
    reasons: ["b", "a", "b"],
    referenceCoverage: 0.9,
    candidateCoverage: 0.8,
    largestMissing: { area: 10, span: 4, box: { x: 1 } },
    error: "must not persist",
  });
  assert.deepEqual(floorCase, {
    relativeCdxml: "source/red.cdxml",
    status: "fail",
    artifactHashes: { reference: "oracle" },
    reasons: ["a", "b"],
    referenceCoverage: 0.9,
    candidateCoverage: 0.8,
    largestMissing: { area: 10, span: 4 },
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
    protectedCases: selected.map((entry, index) => ({
      ...entry,
      status: index < 2 ? "pass" : "fail",
      artifactHashes: { reference: `reference-${index}` },
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
  assert.deepEqual(classifyPassFloorRegressions(baseline.cases, passFloor), [{
    relativeCdxml: "source/0001.cdxml",
    before: "protected-pass",
    after: "fail",
  }]);
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
  const report = (entries) => ({
    cacheIdentity: definition.cacheIdentity,
    policy: defaultGatePolicy(),
    cases: entries,
  });
  const improved = structuredClone(cases);
  improved[0].status = "pass";
  assert.deepEqual(passFloorMigrationErrors(report(cases), report(improved)), []);
  improved[1].status = "fail";
  assert.deepEqual(passFloorMigrationErrors(report(cases), report(improved)), [
    "current candidate has 1 same-gate pass-to-fail regressions",
  ]);

  const continuouslyWorse = structuredClone(cases);
  cases[2].status = "fail";
  continuouslyWorse[2].status = "fail";
  cases[2].largestMissing = { area: 10, span: 8 };
  continuouslyWorse[2].largestMissing = { area: 12, span: 8 };
  assert.deepEqual(
    passFloorMigrationErrors(report(cases), report(continuouslyWorse)),
    ["current candidate has 1 continuous metric regressions"],
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
