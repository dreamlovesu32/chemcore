import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import {
  classifyBaselineChanges,
  classifyContinuousBaselineRegressions,
  passFloorGateDefinition,
  protectedVisualCases,
  reuseReportCompatibilityErrors,
  STRICT_PASS_FLOOR_PATH,
  STRICT_PASS_FLOOR_SCHEMA,
} from "./public-cdxml-visual-gate.mjs";

const execFileAsync = promisify(execFile);

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--previous-report") options.previousReport = argv[++index];
    else if (arg === "--current-report") options.currentReport = argv[++index];
    else if (arg === "--reviewed-retirements") options.reviewedRetirements = argv[++index];
    else throw new Error(`Unknown argument: ${arg}`);
  }
  if (!options.previousReport || !options.currentReport) {
    throw new Error(
      "Usage: node scripts/migrate-public-cdxml-pass-floor.mjs "
      + "--previous-report <same-gate-frozen-candidate-report.json> "
      + "--current-report <same-gate-current-candidate-report.json> "
      + "[--reviewed-retirements gate-definition-retirements.json]",
    );
  }
  return {
    previousReport: path.resolve(options.previousReport),
    currentReport: path.resolve(options.currentReport),
    reviewedRetirements: options.reviewedRetirements
      ? path.resolve(options.reviewedRetirements)
      : null,
  };
}

function sha256(source) {
  return crypto.createHash("sha256").update(source).digest("hex");
}

async function requireCommittedRepositoryFile(filePath) {
  const relativePath = path.relative(process.cwd(), filePath).replaceAll("\\", "/");
  if (!relativePath || relativePath === ".." || relativePath.startsWith("../")) {
    throw new Error("reviewed retirements must be stored inside the repository");
  }
  try {
    await execFileAsync("git", ["ls-files", "--error-unmatch", "--", relativePath], {
      cwd: process.cwd(),
      windowsHide: true,
    });
    await execFileAsync("git", ["diff", "--quiet", "HEAD", "--", relativePath], {
      cwd: process.cwd(),
      windowsHide: true,
    });
  } catch {
    throw new Error("reviewed retirements must exactly match a committed repository file");
  }
}

function exactCohortErrors(report, label) {
  const errors = [];
  if (
    report?.selection?.cohort?.name !== "original-338"
    || report?.selection?.cohort?.expected !== 338
    || report?.selection?.cohort?.selected !== 338
    || report?.summary?.total !== 338
    || report?.cases?.length !== 338
  ) {
    errors.push(`${label} report is not the exact original-338 cohort`);
  }
  if (report?.summary?.errors !== 0) {
    errors.push(`${label} report contains analysis errors`);
  }
  if (report?.galleryProvenance?.repository?.dirty) {
    errors.push(`${label} report came from a dirty gallery`);
  }
  return errors;
}

async function verifiedReport(reportPath, label) {
  const source = await fs.readFile(reportPath, "utf8");
  const report = JSON.parse(source);
  const galleryDir = path.resolve(report.gallery ?? "");
  const manifest = JSON.parse(
    await fs.readFile(path.join(galleryDir, "manifest.json"), "utf8"),
  );
  const errors = [
    ...exactCohortErrors(report, label),
    ...await reuseReportCompatibilityErrors(report, manifest, galleryDir, {
      allowGateDefinitionUpgrade: false,
    }),
  ];
  if (errors.length) {
    throw new Error(`Invalid ${label} report: ${errors.join("; ")}`);
  }
  return { report, sha256: sha256(source) };
}

export function passFloorMigrationErrors(
  previousReport,
  currentReport,
  oldFloor = null,
  reviewedRetirements = null,
) {
  const errors = [];
  const expectedDefinition = passFloorGateDefinition();
  for (const [label, report] of [
    ["previous", previousReport],
    ["current", currentReport],
  ]) {
    if (
      report.policy == null
      || sha256(JSON.stringify(report.policy)) !== expectedDefinition.policySha256
    ) {
      errors.push(`${label} report uses a different gate policy`);
    }
  }
  if (JSON.stringify(previousReport.policy) !== JSON.stringify(currentReport.policy)) {
    errors.push("previous and current reports use different gate policies");
  }
  const previousCases = new Map(previousReport.cases.map((entry) => [
    entry.relativeCdxml.replaceAll("\\", "/"),
    entry,
  ]));
  const currentCases = new Map(currentReport.cases.map((entry) => [
    entry.relativeCdxml.replaceAll("\\", "/"),
    entry,
  ]));
  if (previousCases.size !== 338 || currentCases.size !== 338) {
    errors.push("migration reports must each contain 338 unique paths");
    return errors;
  }
  for (const [relativeCdxml, previous] of previousCases) {
    const current = currentCases.get(relativeCdxml);
    if (!current) {
      errors.push(`current report is missing ${relativeCdxml}`);
      break;
    }
    if (previous.artifactHashes?.reference !== current.artifactHashes?.reference) {
      errors.push(`ChemDraw oracle changed for ${relativeCdxml}`);
      break;
    }
  }
  if (oldFloor) {
    const previousRepository = previousReport.galleryProvenance?.repository;
    if (
      previousRepository?.head !== oldFloor.source?.commit
      || previousRepository?.identity !== oldFloor.source?.repositoryIdentity
    ) {
      errors.push("previous report is not the repository state protected by the old floor");
    }
    const oldProtectedCases = new Map((oldFloor.protectedCases ?? []).map((entry) => [
      entry.relativeCdxml.replaceAll("\\", "/"),
      entry,
    ]));
    const retiredPaths = new Set(reviewedRetirements?.paths ?? []);
    if (reviewedRetirements) {
      const canonicalRetirements = [...retiredPaths].sort();
      if (
        reviewedRetirements.schema
          !== "chemsema.public-cdxml-gate-definition-retirements.v1"
        || reviewedRetirements.fromCacheIdentity
          !== oldFloor.gateDefinition?.cacheIdentity
        || reviewedRetirements.toCacheIdentity
          !== expectedDefinition.cacheIdentity
        || typeof reviewedRetirements.reason !== "string"
        || reviewedRetirements.reason.length === 0
        || retiredPaths.size !== (reviewedRetirements.paths ?? []).length
        || JSON.stringify(canonicalRetirements)
          !== JSON.stringify(reviewedRetirements.paths)
      ) {
        errors.push("reviewed gate-definition retirements are invalid");
      }
    }
    const oldProtectedPasses = new Set(oldFloor.protectedPasses ?? []);
    for (const retiredPath of retiredPaths) {
      if (
        !oldProtectedPasses.has(retiredPath)
        || previousCases.get(retiredPath)?.status !== "fail"
      ) {
        errors.push(`invalid reviewed retirement ${retiredPath}`);
        break;
      }
    }
    const unreviewedRetirements = [];
    for (const relativeCdxml of oldProtectedPasses) {
      if (
        oldProtectedCases.get(relativeCdxml)?.status !== "pass"
      ) {
        errors.push(`old floor has an inconsistent protected pass ${relativeCdxml}`);
        break;
      }
      if (previousCases.get(relativeCdxml)?.status === "pass") continue;
      if (retiredPaths.has(relativeCdxml)) continue;
      unreviewedRetirements.push(relativeCdxml);
    }
    if (unreviewedRetirements.length) {
      errors.push(
        `previous report retires ${unreviewedRetirements.length} old protected passes without review`,
      );
    }
    const currentPasses = currentReport.cases.filter((entry) => entry.status === "pass").length;
    if (currentPasses < oldFloor.minimumPassed - retiredPaths.size) {
      errors.push("current report would lower the protected pass floor");
    }
  }
  const changes = classifyBaselineChanges(currentReport.cases, previousCases);
  if (changes.regressions.length) {
    errors.push(
      `current candidate has ${changes.regressions.length} same-gate pass-to-fail regressions`,
    );
  }
  const continuousRegressions = classifyContinuousBaselineRegressions(
    currentReport.cases,
    previousCases,
  );
  if (continuousRegressions.length) {
    errors.push(
      `current candidate has ${continuousRegressions.length} continuous metric regressions`,
    );
  }
  return errors;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.reviewedRetirements) {
    await requireCommittedRepositoryFile(options.reviewedRetirements);
  }
  const [previous, current, oldFloor, retirementsSource] = await Promise.all([
    verifiedReport(options.previousReport, "previous"),
    verifiedReport(options.currentReport, "current"),
    fs.readFile(STRICT_PASS_FLOOR_PATH, "utf8").then(JSON.parse),
    options.reviewedRetirements
      ? fs.readFile(options.reviewedRetirements, "utf8")
      : Promise.resolve(null),
  ]);
  const reviewedRetirements = retirementsSource
    ? JSON.parse(retirementsSource)
    : null;
  const errors = passFloorMigrationErrors(
    previous.report,
    current.report,
    oldFloor,
    reviewedRetirements,
  );
  if (errors.length) throw new Error(`Pass-floor migration refused: ${errors.join("; ")}`);
  const previousCases = new Map(previous.report.cases.map((entry) => [
    entry.relativeCdxml.replaceAll("\\", "/"),
    entry,
  ]));
  const changes = classifyBaselineChanges(current.report.cases, previousCases);
  const protectedPasses = current.report.cases
    .filter((entry) => entry.status === "pass")
    .map((entry) => entry.relativeCdxml.replaceAll("\\", "/"))
    .sort();
  const migrated = {
    schema: STRICT_PASS_FLOOR_SCHEMA,
    gateDefinition: passFloorGateDefinition(),
    cohort: { name: "original-338", expected: 338 },
    minimumPassed: protectedPasses.length,
    source: {
      commit: current.report.galleryProvenance.repository.head,
      repositoryIdentity: current.report.galleryProvenance.repository.identity,
      gateReportGeneratedAt: current.report.generatedAt,
    },
    migration: {
      previousSchema: oldFloor.schema,
      previousMinimumPassed: oldFloor.minimumPassed,
      previousReportSha256: previous.sha256,
      currentReportSha256: current.sha256,
      sameGateRegressions: 0,
      sameGateImprovements: changes.improvements.length,
      reviewedRetirements: reviewedRetirements ? {
        path: path.relative(process.cwd(), options.reviewedRetirements)
          .replaceAll("\\", "/"),
        sha256: sha256(retirementsSource),
        reason: reviewedRetirements.reason,
        count: reviewedRetirements.paths.length,
      } : null,
    },
    protectedPasses,
    protectedCases: protectedVisualCases(current.report.cases),
  };
  await fs.writeFile(
    STRICT_PASS_FLOOR_PATH,
    `${JSON.stringify(migrated, null, 2)}\n`,
  );
  console.log(JSON.stringify({
    passFloor: STRICT_PASS_FLOOR_PATH,
    before: oldFloor.minimumPassed,
    after: migrated.minimumPassed,
    sameGateImprovements: changes.improvements.length,
    sameGateRegressions: 0,
  }));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack ?? error.message : String(error));
    process.exit(1);
  });
}
