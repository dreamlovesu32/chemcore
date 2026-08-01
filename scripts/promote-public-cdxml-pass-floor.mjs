import fs from "node:fs/promises";
import path from "node:path";
import {
  classifyPassFloorRegressions,
  reuseReportCompatibilityErrors,
  strictOriginal338PassFloorErrors,
  STRICT_PASS_FLOOR_PATH,
} from "./public-cdxml-visual-gate.mjs";
import {
  collectCurrentGalleryProvenance,
  provenanceMismatches,
} from "./public-cdxml-provenance.mjs";

function reportArgument(argv) {
  const index = argv.indexOf("--report");
  if (index < 0 || !argv[index + 1]) {
    throw new Error("Usage: node scripts/promote-public-cdxml-pass-floor.mjs --report <strict-report.json>");
  }
  if (argv.length !== 2) {
    throw new Error("Only --report <strict-report.json> is accepted");
  }
  return path.resolve(argv[index + 1]);
}

async function main() {
  const reportPath = reportArgument(process.argv.slice(2));
  const [report, passFloor] = await Promise.all([
    fs.readFile(reportPath, "utf8").then(JSON.parse),
    fs.readFile(STRICT_PASS_FLOOR_PATH, "utf8").then(JSON.parse),
  ]);
  if (report.enforcement?.mode !== "strict-original-338") {
    throw new Error("Pass-floor promotion requires a strict-original-338 report");
  }
  if (
    report.selection?.cohort?.name !== "original-338"
    || report.selection?.cohort?.expected !== 338
    || report.selection?.cohort?.selected !== 338
    || report.summary?.total !== 338
  ) {
    throw new Error("Pass-floor promotion requires the exact original-338 cohort");
  }
  if (report.summary?.errors !== 0) {
    throw new Error("Pass-floor promotion refuses a report with analysis errors");
  }
  if (report.galleryProvenance?.repository?.dirty) {
    throw new Error("Pass-floor promotion refuses a gallery rendered from a dirty repository");
  }
  const currentProvenance = collectCurrentGalleryProvenance(report.galleryProvenance);
  const mismatches = currentProvenance
    ? provenanceMismatches(report.galleryProvenance, currentProvenance)
    : ["missing-or-unsupported-provenance"];
  if (mismatches.length) {
    throw new Error(`Pass-floor promotion refuses stale provenance: ${mismatches.join(", ")}`);
  }
  const galleryDir = path.resolve(report.gallery);
  const manifest = JSON.parse(
    await fs.readFile(path.join(galleryDir, "manifest.json"), "utf8"),
  );
  const reportCompatibilityErrors = await reuseReportCompatibilityErrors(
    report,
    manifest,
    galleryDir,
    {},
  );
  if (reportCompatibilityErrors.length) {
    throw new Error(
      `Pass-floor promotion refuses an incompatible report: ${
        reportCompatibilityErrors.join("; ")
      }`,
    );
  }
  const validationErrors = strictOriginal338PassFloorErrors(
    passFloor,
    report.cases,
    report,
    { strictOriginal338: true },
  );
  if (validationErrors.length) {
    throw new Error(`Current pass floor is invalid: ${validationErrors.join("; ")}`);
  }
  const floorRegressions = classifyPassFloorRegressions(report.cases, passFloor);
  if (
    floorRegressions.length
    || report.delta?.regressions?.length
    || report.delta?.continuousRegressions?.length
  ) {
    throw new Error("Pass-floor promotion refuses a report containing regressions");
  }
  const protectedPasses = [...new Set([
    ...passFloor.protectedPasses,
    ...report.cases
      .filter((entry) => entry.status === "pass")
      .map((entry) => entry.relativeCdxml.replaceAll("\\", "/")),
  ])].sort();
  const promoted = {
    ...passFloor,
    minimumPassed: protectedPasses.length,
    source: {
      commit: report.galleryProvenance.repository.head,
      repositoryIdentity: report.galleryProvenance.repository.identity,
      gateReportGeneratedAt: report.generatedAt,
    },
    protectedPasses,
  };
  await fs.writeFile(
    STRICT_PASS_FLOOR_PATH,
    `${JSON.stringify(promoted, null, 2)}\n`,
  );
  console.log(JSON.stringify({
    passFloor: STRICT_PASS_FLOOR_PATH,
    before: passFloor.protectedPasses.length,
    after: protectedPasses.length,
    added: protectedPasses.length - passFloor.protectedPasses.length,
  }));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exit(1);
});
