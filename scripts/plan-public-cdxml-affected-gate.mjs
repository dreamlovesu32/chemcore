import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import {
  AFFECTED_PLAN_SCHEMA,
  buildFeatureIndex,
  selectAffectedCases,
} from "./public-cdxml-impact.mjs";
import {
  CACHE_IDENTITY,
  CASE_METRICS_SCHEMA,
  REPORT_SCHEMA,
} from "./public-cdxml-visual-gate.mjs";
import { defaultPublicCdxmlCliRelativePath } from "./public-cdxml-provenance.mjs";

const execFileAsync = promisify(execFile);

function parseArgs(argv) {
  const options = {
    root: "tmp/public-corpus-pilot",
    roundtripReport: "tmp/public-cdxml-roundtrip/report.json",
    gallery: "tmp/public-cdxml-chemdraw-review-all",
    impactMap: "benchmarks/public-cdxml/visual-impact-map.json",
    featureIndex: "tmp/public-cdxml-feature-index.json",
    plan: "tmp/public-cdxml-affected-gate-plan.json",
    cli: defaultPublicCdxmlCliRelativePath(),
    jobs: 8,
    extras: [],
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--root") options.root = argv[++index];
    else if (arg === "--roundtrip-report") options.roundtripReport = argv[++index];
    else if (arg === "--gallery") options.gallery = argv[++index];
    else if (arg === "--baseline-report") options.baselineReport = argv[++index];
    else if (arg === "--out") options.out = argv[++index];
    else if (arg === "--passed-gallery") options.passedGallery = argv[++index];
    else if (arg === "--impact-map") options.impactMap = argv[++index];
    else if (arg === "--feature-index") options.featureIndex = argv[++index];
    else if (arg === "--plan") options.plan = argv[++index];
    else if (arg === "--cli") options.cli = argv[++index];
    else if (arg === "--jobs") options.jobs = Number(argv[++index]);
    else if (arg === "--base") options.base = argv[++index];
    else if (arg === "--head") options.head = argv[++index];
    else if (arg === "--extra") options.extras.push(argv[++index]);
    else if (arg === "--dry-run") options.dryRun = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return options;
}

async function gitLines(args) {
  const { stdout } = await execFileAsync("git", args, { maxBuffer: 8 * 1024 * 1024 });
  return stdout.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
}

async function changedFiles(options) {
  if (options.base) {
    return gitLines(["diff", "--name-only", `${options.base}..${options.head ?? "HEAD"}`]);
  }
  const [tracked, untracked] = await Promise.all([
    gitLines(["diff", "--name-only", "HEAD"]),
    gitLines(["ls-files", "--others", "--exclude-standard"]),
  ]);
  return [...new Set([...tracked, ...untracked])];
}

async function run(command) {
  const [executable, ...args] = command;
  await new Promise((resolve, reject) => {
    const child = execFile(executable, args, { windowsHide: true }, (error) => {
      if (error) reject(error);
      else resolve();
    });
    child.stdout?.pipe(process.stdout);
    child.stderr?.pipe(process.stderr);
  });
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log("Usage: node scripts/plan-public-cdxml-affected-gate.mjs [--base ref --head ref] [--extra case] [--dry-run]");
    console.log("       [--gallery dir] [--baseline-report report.json] [--out report.json] [--feature-index index.json]");
    return;
  }
  if (!Number.isInteger(options.jobs) || options.jobs < 1) throw new Error("--jobs must be a positive integer");

  const root = path.resolve(options.root);
  const gallery = path.resolve(options.gallery);
  const roundtripReportPath = path.resolve(options.roundtripReport);
  const baselineReport = path.resolve(options.baselineReport ?? path.join(gallery, "gate-report.json"));
  const outputReport = path.resolve(options.out ?? baselineReport);
  const passedGallery = path.resolve(options.passedGallery ?? path.join(gallery, "passed.html"));
  const cli = path.resolve(options.cli);
  const [roundtripReport, impactMap, files] = await Promise.all([
    fs.readFile(roundtripReportPath, "utf8").then(JSON.parse),
    fs.readFile(path.resolve(options.impactMap), "utf8").then(JSON.parse),
    changedFiles(options),
  ]);
  const featureIndex = await buildFeatureIndex({
    root,
    report: roundtripReport,
    cli,
    jobs: options.jobs,
  });
  const featureIndexPath = path.resolve(options.featureIndex);
  await fs.mkdir(path.dirname(featureIndexPath), { recursive: true });
  await fs.writeFile(featureIndexPath, `${JSON.stringify(featureIndex, null, 2)}\n`);

  const selection = selectAffectedCases({
    changedFiles: files,
    featureIndex,
    impactMap,
    extras: options.extras,
  });
  if (!selection.selected.length) {
    throw new Error("Affected plan selected no cases; update visual-impact-map.json or pass --extra.");
  }
  const onlyArgs = selection.selected.flatMap((entry) => ["--only", entry.caseId]);
  const renderCommand = [
    process.execPath,
    "scripts/render-public-cdxml-visual-review.mjs",
    "--all",
    "--incremental",
    "--root", root,
    "--report", roundtripReportPath,
    "--out", gallery,
    "--cli", cli,
    ...onlyArgs,
  ];
  const gateCommand = [
    process.execPath,
    "scripts/public-cdxml-visual-gate.mjs",
    "--gallery", gallery,
    "--baseline-report", baselineReport,
    "--out", outputReport,
    "--passed-gallery", passedGallery,
  ];
  const plan = {
    schema: AFFECTED_PLAN_SCHEMA,
    generatedAt: new Date().toISOString(),
    changedFiles: selection.changedFiles,
    matchedImpactRules: selection.matchedRules,
    unmatchedProductionFiles: selection.unmatchedProductionFiles,
    forceFull: selection.forceFull,
    selectedCount: selection.selected.length,
    selectedCases: selection.selected.map((entry) => ({
      caseId: entry.caseId,
      relativeCdxml: entry.relativeCdxml,
      format: entry.format,
      features: entry.features,
    })),
    artifacts: {
      featureIndex: featureIndexPath,
      gallery,
      baselineReport,
      outputReport,
      passedGallery,
    },
    commands: { render: renderCommand, gate: gateCommand },
    policy: {
      affectedGateBeforeFullGate: true,
      escapedRegressionRequiresImpactMapRepair: true,
      unknownProductionChangeForcesFull: true,
    },
  };
  const planPath = path.resolve(options.plan);
  await fs.mkdir(path.dirname(planPath), { recursive: true });
  await fs.writeFile(planPath, `${JSON.stringify(plan, null, 2)}\n`);
  console.log(JSON.stringify({
    planPath,
    featureIndexPath,
    selectedCount: plan.selectedCount,
    forceFull: plan.forceFull,
    matchedImpactRules: plan.matchedImpactRules,
  }));
  if (options.dryRun) return;

  const baseline = JSON.parse(await fs.readFile(baselineReport, "utf8"));
  const unstamped = baseline.schema !== REPORT_SCHEMA
    || baseline.caseMetricsSchema !== CASE_METRICS_SCHEMA
    || baseline.cacheIdentity !== CACHE_IDENTITY
    || baseline.cases.some((entry) => !entry.artifactHashes);
  if (unstamped) {
    throw new Error(
      `Baseline report does not use the current complete gate schema. Re-run: `
      + `node scripts/public-cdxml-visual-gate.mjs --gallery "${gallery}" `
      + `--out "${baselineReport}"`,
    );
  }
  await run(renderCommand);
  await run(gateCommand);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack ?? error.message : String(error));
    process.exit(1);
  });
}
