import assert from "node:assert/strict";
import test from "node:test";
import { failureLedgerResolutionStatus } from "../build-public-cdxml-failure-ledger.mjs";

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
