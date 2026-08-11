import assert from "node:assert/strict";
import test from "node:test";
import {
  automaticClusters,
  failureLedgerInputArgumentErrors,
  failureLedgerRegisteredCaseIds,
  failureLedgerResolutionStatus,
  visualFailureScale,
} from "../build-public-cdxml-failure-ledger.mjs";

test("failure ledger requires every immutable run input explicitly", () => {
  assert.deepEqual(
    failureLedgerInputArgumentErrors({}),
    [
      "--roundtrip is required",
      "--visual is required",
      "--baseline is required",
      "--features is required",
    ],
  );
  assert.deepEqual(
    failureLedgerInputArgumentErrors({
      roundtrip: "roundtrip.json",
      visual: "visual.json",
      baseline: "baseline.json",
      features: "features.json",
    }),
    [],
  );
});

test("failure ledger keeps every non-passing visual case active", () => {
  for (const visualStatus of ["fail", "error", "unavailable"]) {
    assert.equal(
      failureLedgerResolutionStatus("exact", visualStatus),
      "active",
      visualStatus,
    );
  }
});

test("failure ledger distinguishes confirmed passes from cases outside the visual gate", () => {
  assert.equal(
    failureLedgerResolutionStatus("exact", "pass"),
    "currently-passing",
  );
  assert.equal(
    failureLedgerResolutionStatus("exact", undefined),
    "not-gated",
  );
});

test("roundtrip failures remain active regardless of visual status", () => {
  for (const roundtripStatus of [
    "import-failed",
    "export-failed",
    "reimport-failed",
    "topology-lost",
    "semantic-drift",
    "non-idempotent",
    "count-drift",
  ]) {
    assert.equal(
      failureLedgerResolutionStatus(roundtripStatus, "pass"),
      "active",
      roundtripStatus,
    );
  }
});

test("a clean roundtrip run still registers every current visual-gate case", () => {
  const roundtripCases = [
    { caseId: "a", source: "source", path: "a.cdxml", status: "exact" },
    { caseId: "b", source: "source", path: "b.cdx", status: "exact" },
  ];
  assert.deepEqual(
    failureLedgerRegisteredCaseIds({
      roundtripCases,
      visualCases: [
        { relativeCdxml: "source/a.cdxml" },
        { relativeCdxml: "source\\b.cdx" },
      ],
    }),
    ["a", "b"],
  );
});

test("the ledger rejects a visual case absent from the roundtrip authority", () => {
  assert.throws(
    () => failureLedgerRegisteredCaseIds({
      roundtripCases: [],
      visualCases: [{ relativeCdxml: "source/missing.cdxml" }],
    }),
    /missing from the roundtrip report/,
  );
});

test("visual mismatch clusters retain severity and exact gate reasons", () => {
  const roundtripCase = { status: "exact", comparisons: [] };
  const visualCase = {
    status: "fail",
    coarsePassed: true,
    referenceCoverage: 1,
    candidateCoverage: 1,
    reasons: ["detail-repeated-micro-defects"],
  };
  assert.equal(visualFailureScale(visualCase), "detail-only");
  assert.deepEqual(
    automaticClusters(roundtripCase, visualCase),
    [
      "visual-detail-only-mismatch",
      "visual-pixel-mismatch",
      "visual-reason-detail-repeated-micro-defects",
    ],
  );
  assert.equal(visualFailureScale({
    status: "fail",
    coarsePassed: false,
    referenceCoverage: 0.79,
    candidateCoverage: 0.95,
  }), "major");
  assert.equal(visualFailureScale({
    status: "fail",
    coarsePassed: false,
    referenceCoverage: 0.9,
    candidateCoverage: 0.91,
  }), "local");
});
