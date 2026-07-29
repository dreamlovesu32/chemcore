import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const UNEXPECTED_ROUNDTRIP_STATUSES = new Set([
  "import-failed",
  "export-failed",
  "reimport-failed",
  "topology-lost",
  "semantic-drift",
  "non-idempotent",
]);
const ROUNDTRIP_GATE_FAILURE_STATUSES = new Set([
  ...UNEXPECTED_ROUNDTRIP_STATUSES,
  "count-drift",
]);

export function failureLedgerResolutionStatus(roundtripStatus, visualStatus) {
  if (
    ROUNDTRIP_GATE_FAILURE_STATUSES.has(roundtripStatus)
    || (visualStatus !== undefined && visualStatus !== "pass")
  ) {
    return "active";
  }
  return visualStatus === "pass" ? "currently-passing" : "not-gated";
}

function parseArgs(argv) {
  const options = {
    roundtrip: "tmp/public-cdxml-roundtrip-current/report.json",
    visual: "tmp/public-cdxml-chemdraw-review-all/gate-current.json",
    baseline: "tmp/public-cdxml-chemdraw-review-all/gate-next20-final.json",
    features: "tmp/public-cdxml-feature-index-current.json",
    rules: "benchmarks/public-cdxml/failure-rules.json",
    out: "benchmarks/public-cdxml/failure-ledger.json",
    expected: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--roundtrip") options.roundtrip = argv[++index];
    else if (argument === "--visual") options.visual = argv[++index];
    else if (argument === "--baseline") options.baseline = argv[++index];
    else if (argument === "--features") options.features = argv[++index];
    else if (argument === "--rules") options.rules = argv[++index];
    else if (argument === "--out") options.out = argv[++index];
    else if (argument === "--expected") options.expected = Number(argv[++index]);
    else throw new Error(`Unknown argument: ${argument}`);
  }
  return options;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(path.resolve(filePath), "utf8"));
}

function changedComponents(comparisons = []) {
  return comparisons.map((comparison) => ({
    from: comparison.from,
    to: comparison.to,
    countsExact: comparison.counts?.exact ?? null,
    countDelta: comparison.counts?.delta ?? null,
    objectTypeDelta: comparison.counts?.objectTypeDelta ?? null,
    semanticExact: comparison.semantic?.exact ?? null,
    changed: comparison.semantic?.changed ?? [],
  }));
}

function automaticClusters(roundtripCase, visualCase) {
  const clusters = new Set();
  const comparisons = roundtripCase.comparisons ?? [];
  const firstChanged = comparisons[0]?.semantic?.changed ?? [];
  const laterStable = comparisons.slice(1).every(
    (comparison) => comparison.counts?.exact && comparison.semantic?.exact,
  );
  if (roundtripCase.status === "semantic-drift") {
    if (laterStable && firstChanged.length === 1 && firstChanged[0] === "labels") {
      clusters.add("roundtrip-first-generation-label-drift");
    } else if (laterStable) {
      clusters.add("roundtrip-first-generation-mixed-drift");
    } else {
      clusters.add("roundtrip-semantic-drift");
    }
  } else if (roundtripCase.status === "non-idempotent") {
    const changed = new Set(
      comparisons.slice(1).flatMap((comparison) => comparison.semantic?.changed ?? []),
    );
    if (changed.size === 1 && changed.has("labels")) {
      clusters.add("roundtrip-non-idempotent-labels");
    } else if (changed.size) {
      clusters.add("roundtrip-non-idempotent-mixed-semantics");
    } else {
      clusters.add("roundtrip-non-idempotent-counts");
    }
  } else if (roundtripCase.status === "topology-lost") {
    clusters.add("roundtrip-topology-lost");
  } else if (roundtripCase.status === "count-drift") {
    clusters.add("roundtrip-count-drift");
  } else if (roundtripCase.status.includes("failed")) {
    clusters.add(`roundtrip-${roundtripCase.status}`);
  }
  if (visualCase?.status === "fail") {
    clusters.add("visual-pixel-mismatch");
    if ((visualCase.reasons ?? []).some((reason) => reason.includes("component"))) {
      clusters.add("visual-component-topology-mismatch");
    }
  } else if (visualCase?.status === "unavailable") {
    clusters.add("visual-oracle-unavailable");
  } else if (!visualCase) {
    clusters.add("visual-not-gated");
  }
  return [...clusters].sort();
}

function visualSnapshot(visualCase, baselineStatus) {
  if (!visualCase) return null;
  return {
    status: visualCase.status,
    baselineStatus: baselineStatus ?? null,
    regression: baselineStatus === "pass" && visualCase.status !== "pass",
    reasons: visualCase.reasons ?? [],
    referenceCoverage: visualCase.referenceCoverage ?? null,
    candidateCoverage: visualCase.candidateCoverage ?? null,
    largestMissing: visualCase.largestMissing ?? null,
    largestExtra: visualCase.largestExtra ?? null,
    componentCountDelta: visualCase.detailFeatures?.componentCountDelta ?? null,
  };
}

function countBy(entries, values) {
  const counts = {};
  for (const entry of entries) {
    for (const value of values(entry)) counts[value] = (counts[value] ?? 0) + 1;
  }
  return Object.fromEntries(
    Object.entries(counts).sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0])),
  );
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const roundtrip = readJson(options.roundtrip);
  const visual = readJson(options.visual);
  const baseline = readJson(options.baseline);
  const features = readJson(options.features);
  const rules = readJson(options.rules);
  const outputPath = path.resolve(options.out);
  const existing = fs.existsSync(outputPath) ? readJson(outputPath) : { cases: [] };
  const existingById = new Map(existing.cases.map((entry) => [entry.caseId, entry]));
  const visualByPath = new Map(visual.cases.map((entry) => [entry.relativeCdxml, entry]));
  const baselineByPath = new Map(baseline.cases.map((entry) => [entry.relativeCdxml, entry]));
  const featuresById = new Map(features.cases.map((entry) => [entry.caseId, entry]));
  const rulesByCaseId = new Map();
  for (const rule of rules.rules) {
    for (const caseId of rule.caseIds) {
      const matches = rulesByCaseId.get(caseId) ?? [];
      matches.push(rule);
      rulesByCaseId.set(caseId, matches);
    }
  }
  const failures = roundtrip.cases.filter((entry) =>
    ROUNDTRIP_GATE_FAILURE_STATUSES.has(entry.status));
  if (Number.isFinite(options.expected) && failures.length !== options.expected) {
    throw new Error(`Expected ${options.expected} unexpected failures, found ${failures.length}`);
  }
  const currentById = new Map(roundtrip.cases.map((entry) => [entry.caseId, entry]));
  const registeredIds = new Set([
    ...existing.cases.map((entry) => entry.caseId),
    ...failures.map((entry) => entry.caseId),
  ]);
  const cases = [...registeredIds].sort().map((caseId) => {
    const entry = currentById.get(caseId);
    if (!entry) throw new Error(`Registered case ${caseId} is missing from the current report`);
    const relativeCdxml = `${entry.source}/${entry.path}`.replaceAll("\\", "/");
    const visualCase = visualByPath.get(relativeCdxml);
    const previous = existingById.get(entry.caseId);
    const confirmedRules = rulesByCaseId.get(entry.caseId) ?? [];
    const resolutionStatus = failureLedgerResolutionStatus(
      entry.status,
      visualCase?.status,
    );
    const currentClusters = automaticClusters(entry, visualCase);
    const historicalClusters = [
      ...new Set([
        ...(previous?.triage?.historicalClusters ?? []),
        ...(previous?.triage?.automaticClusters ?? []),
        ...currentClusters,
      ]),
    ].sort();
    return {
      caseId: entry.caseId,
      cohort: previous
        ? previous.cohort ?? "original-338"
        : "additional-gate-case",
      source: entry.source,
      path: entry.path,
      relativeCdxml,
      format: entry.format,
      features: featuresById.get(entry.caseId)?.features ?? [],
      roundtrip: {
        status: entry.status,
        failedGeneration: entry.failedGeneration ?? null,
        error: entry.error ?? null,
        comparisons: changedComponents(entry.comparisons),
      },
      visual: visualSnapshot(
        visualCase,
        baselineByPath.get(relativeCdxml)?.status,
      ),
      triage: {
        resolutionStatus,
        automaticClusters: currentClusters,
        historicalClusters,
        reviewStatus: confirmedRules.length
          ? "verified"
          : previous?.triage?.reviewStatus ?? "pending",
        confirmedRuleIds: confirmedRules.map((rule) => rule.id),
        confirmedRootCause: confirmedRules.length
          ? confirmedRules.map((rule) => rule.summary).join("; ")
          : previous?.triage?.confirmedRootCause ?? null,
        ruleEvidence: confirmedRules.length
          ? [...new Set(confirmedRules.flatMap((rule) => rule.evidence))]
          : previous?.triage?.ruleEvidence ?? [],
        notes: previous?.triage?.notes ?? null,
      },
    };
  });
  const ledger = {
    schema: "chemsema.public-cdxml-failure-ledger.v1",
    generatedAt: new Date().toISOString(),
    inputs: {
      roundtripReport: path.relative(process.cwd(), path.resolve(options.roundtrip)).replaceAll("\\", "/"),
      visualReport: path.relative(process.cwd(), path.resolve(options.visual)).replaceAll("\\", "/"),
      baselineReport: path.relative(process.cwd(), path.resolve(options.baseline)).replaceAll("\\", "/"),
      featureIndex: path.relative(process.cwd(), path.resolve(options.features)).replaceAll("\\", "/"),
      ruleRegistry: path.relative(process.cwd(), path.resolve(options.rules)).replaceAll("\\", "/"),
    },
    summary: {
      totalRegistered: cases.length,
      originalCohort: cases.filter((entry) => entry.cohort === "original-338").length,
      additionalGateCases: cases.filter(
        (entry) => entry.cohort === "additional-gate-case",
      ).length,
      active: cases.filter((entry) => entry.triage.resolutionStatus === "active").length,
      currentlyPassing: cases.filter(
        (entry) => entry.triage.resolutionStatus === "currently-passing",
      ).length,
      notGated: cases.filter(
        (entry) => entry.triage.resolutionStatus === "not-gated",
      ).length,
      byRoundtripStatus: countBy(cases, (entry) => [entry.roundtrip.status]),
      byAutomaticCluster: countBy(cases, (entry) => entry.triage.automaticClusters),
      byHistoricalCluster: countBy(cases, (entry) => entry.triage.historicalClusters),
      byVisualStatus: countBy(cases, (entry) => [entry.visual?.status ?? "not-gated"]),
      byReviewStatus: countBy(cases, (entry) => [entry.triage.reviewStatus]),
    },
    cases,
  };
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(ledger, null, 2)}\n`);
  console.log(JSON.stringify(ledger.summary, null, 2));
  console.log(`Ledger: ${outputPath}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
