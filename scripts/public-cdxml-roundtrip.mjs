import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import {
  repositoryState,
  sha256File,
} from "./public-cdxml-provenance.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifestPath = join(repoRoot, "benchmarks", "public-cdxml", "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const args = process.argv.slice(2);

function option(name, fallback) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : fallback;
}

const corpusRoot = resolve(
  option(
    "--root",
    process.env.CHEMSEMA_PUBLIC_CDXML_DIR || join(repoRoot, "tmp", "public-cdxml-corpus"),
  ),
);
const outputRoot = resolve(option("--out-dir", join(repoRoot, "tmp", "public-cdxml-roundtrip")));
const reportPath = join(outputRoot, "report.json");
const summaryOutput = option("--summary-out", null);
const workRoot = join(outputRoot, "work");
const limit = Number.parseInt(option("--limit", "0"), 10);
const generations = Math.max(1, Number.parseInt(option("--generations", "3"), 10));
const strictCounts = args.includes("--strict-counts");

function discoverCli() {
  const explicit = option("--cli", process.env.CHEMSEMA_CLI);
  const suffix = process.platform === "win32" ? ".exe" : "";
  const candidates = [
    explicit,
    join(repoRoot, "target", "debug", `chemsema-cli${suffix}`),
    join(repoRoot, "target", "release", `chemsema-cli${suffix}`),
  ].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error(
      "chemsema-cli not found. Build it with `cargo build -p chemsema-cli` or pass --cli <path>.",
    );
  }
  return resolve(found);
}

const cliPath = discoverCli();
mkdirSync(workRoot, { recursive: true });

function runCli(commandArgs) {
  const result = spawnSync(cliPath, commandArgs, {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
  return {
    ok: result.status === 0,
    status: result.status,
    stdout: result.stdout?.trim() || "",
    stderr: result.stderr?.trim() || "",
    error: result.error?.message || null,
  };
}

function collectFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory).sort()) {
      if (entry === ".git") continue;
      const path = join(directory, entry);
      const stat = statSync(path);
      if (stat.isDirectory()) {
        visit(path);
      } else if ([".cdxml", ".cdx"].includes(extname(entry).toLowerCase())) {
        files.push(path);
      }
    }
  };
  visit(root);
  return files;
}

function normalizedRelative(root, path) {
  return relative(root, path).split(sep).join("/");
}

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([, item]) => item !== undefined)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, item]) => [key, canonicalize(item)]),
    );
  }
  return value;
}

function digest(value) {
  return createHash("sha256").update(JSON.stringify(canonicalize(value))).digest("hex");
}

function roundNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? Math.round(value * 100) / 100 : value;
}

function roundGeometry(value) {
  if (Array.isArray(value)) return value.map(roundGeometry);
  return roundNumber(value);
}

function flattenObjects(objects, output = []) {
  for (const object of objects || []) {
    output.push(object);
    flattenObjects(object.children, output);
  }
  return output;
}

function payloadValue(object, key) {
  return object?.payload?.[key] ?? object?.payload?.extra?.[key];
}

function fragmentForObject(document, object) {
  const resourceRef = payloadValue(object, "resourceRef");
  const resource = document.resources?.[resourceRef];
  return resource?.data?.nodes && resource?.data?.bonds ? resource.data : null;
}

function summarizeCounts(document) {
  const objects = flattenObjects(document.objects);
  const objectTypes = {};
  let molecules = 0;
  let nodes = 0;
  let bonds = 0;
  for (const object of objects) {
    objectTypes[object.type] = (objectTypes[object.type] || 0) + 1;
    if (object.type !== "molecule") continue;
    molecules += 1;
    const fragment = fragmentForObject(document, object);
    nodes += fragment?.nodes?.length || 0;
    bonds += fragment?.bonds?.length || 0;
  }
  return {
    molecules,
    nodes,
    bonds,
    objects: objects.length,
    resources: Object.keys(document.resources || {}).length,
    styles: Object.keys(document.styles || {}).length,
    objectTypes,
  };
}

function nodeSignature(node) {
  const label = node.label;
  return {
    atomicNumber: node.atomicNumber ?? null,
    element: node.element ?? null,
    charge: node.charge ?? 0,
    numHydrogens: node.numHydrogens ?? null,
    isotope: node.isotope ?? null,
    radical: node.radical ?? null,
    placeholder: node.isPlaceholder ?? false,
    externalConnection: node.externalConnection ?? null,
    label: label
      ? {
          sourceText: label.sourceText ?? null,
          runs: (label.runs || []).map((run) => ({
            text: run.text ?? "",
            script: run.script ?? null,
          })),
        }
      : null,
  };
}

function bondSignature(bond, nodeIndexes) {
  const begin = nodeIndexes.get(bond.begin) ?? -1;
  const end = nodeIndexes.get(bond.end) ?? -1;
  return {
    endpoints: [Math.min(begin, end), Math.max(begin, end)],
    order: bond.order ?? null,
    stereo: bond.stereo ?? null,
    double: bond.double
      ? { placement: bond.double.placement ?? null }
      : null,
    lineStyles: bond.lineStyles ?? null,
    endpointAttachments: bond.endpointAttachments ?? null,
  };
}

function moleculeSignatures(document, objects) {
  const signatures = objects
    .filter((object) => object.type === "molecule")
    .map((object) => {
      const fragment = fragmentForObject(document, object);
      if (!fragment) return { missingResource: true };
      const nodeIndexes = new Map(fragment.nodes.map((node, index) => [node.id, index]));
      return {
        nodes: fragment.nodes.map(nodeSignature),
        bonds: fragment.bonds
          .map((bond) => bondSignature(bond, nodeIndexes))
          .sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right))),
      };
    });
  return signatures.map((signature) => JSON.stringify(canonicalize(signature))).sort();
}

function translatedPoint(point, translate) {
  if (!Array.isArray(point) || point.length < 2) return null;
  return roundGeometry([point[0] + (translate?.[0] || 0), point[1] + (translate?.[1] || 0)]);
}

function translatedBox(box, translate) {
  if (!Array.isArray(box) || box.length < 4) return null;
  return roundGeometry([
    box[0] + (translate?.[0] || 0),
    box[1] + (translate?.[1] || 0),
    box[2] + (translate?.[0] || 0),
    box[3] + (translate?.[1] || 0),
  ]);
}

function labelSignature(label) {
  const imported = label?.meta?.import?.cdxml || {};
  return {
    sourceText: label?.sourceText ?? null,
    text: label?.text ?? null,
    lines: label?.lines ?? [],
    lineRuns: (label?.lineRuns || []).map((line) =>
      line.map((run) => ({ text: run.text ?? "", script: run.script ?? null })),
    ),
    runs: (label?.runs || []).map((run) => ({
      text: run.text ?? "",
      script: run.script ?? null,
    })),
    align: label?.align ?? null,
    anchor: label?.anchor ?? null,
    layout: label?.layout ?? null,
    cdxml: {
      labelDisplay: imported.labelDisplay ?? null,
      lineHeight: imported.lineHeight ?? null,
      labelLineHeight: imported.labelLineHeight ?? null,
      wordWrapWidth: imported.wordWrapWidth ?? null,
    },
  };
}

function moleculeLabelSignatures(document, objects) {
  return objects
    .filter((object) => object.type === "molecule")
    .flatMap((object) => {
      const fragment = fragmentForObject(document, object);
      return (fragment?.nodes || [])
        .map((node) =>
          node.label
            ? {
                atomicNumber: node.atomicNumber ?? null,
                label: labelSignature(node.label),
              }
            : null,
        )
        .filter(Boolean);
    })
    .map((signature) => JSON.stringify(canonicalize(signature)))
    .sort();
}

function moleculeLabelGeometry(document, objects) {
  return objects
    .filter((object) => object.type === "molecule")
    .flatMap((object) => {
      const fragment = fragmentForObject(document, object);
      return (fragment?.nodes || [])
        .filter((node) => node.label)
        .map((node) => ({
          key: JSON.stringify([
            node.atomicNumber ?? null,
            node.label.sourceText ?? null,
            node.label.text ?? null,
          ]),
          position: translatedPoint(node.label.position, object.transform?.translate),
          box: translatedBox(
            node.label.boxField ?? node.label.boxValue,
            object.transform?.translate,
          ),
        }));
    })
    .sort((left, right) =>
      left.key.localeCompare(right.key) ||
      JSON.stringify(left.position).localeCompare(JSON.stringify(right.position)),
    );
}

function textSignatures(objects) {
  return objects
    .filter((object) => object.type === "text")
    .map((object) => ({
      text: payloadValue(object, "text") ?? "",
      runs: (payloadValue(object, "runs") || []).map((run) => ({
        text: run.text ?? "",
        script: run.script ?? null,
      })),
      align: payloadValue(object, "align") ?? null,
      preserveLines: payloadValue(object, "preserveLines") ?? null,
      cdxml: {
        wordWrapWidth: object.meta?.import?.cdxml?.wordWrapWidth ?? null,
      },
    }))
    .map((signature) => JSON.stringify(canonicalize(signature)))
    .sort();
}

function textGeometry(objects) {
  return objects
    .filter((object) => object.type === "text")
    .map((object) => {
      const translate = object.transform?.translate ?? [0, 0];
      const box = payloadValue(object, "box");
      return {
        key: JSON.stringify([
          payloadValue(object, "text") ?? "",
          payloadValue(object, "align") ?? null,
        ]),
        position: roundGeometry(translate),
        box:
          Array.isArray(box) && box.length >= 4
            ? roundGeometry([
                translate[0] + box[0],
                translate[1] + box[1],
                translate[0] + box[0] + box[2],
                translate[1] + box[1] + box[3],
              ])
            : null,
        lineHeight: [roundNumber(payloadValue(object, "lineHeight") ?? 0)],
      };
    })
    .sort((left, right) =>
      left.key.localeCompare(right.key) ||
      JSON.stringify(left.position).localeCompare(JSON.stringify(right.position)),
    );
}

function arrowSignatures(objects) {
  return objects
    .filter((object) => object.type === "line" && payloadValue(object, "arrowHead"))
    .map((object) => {
      const arrowHead = payloadValue(object, "arrowHead");
      return {
        kind: arrowHead.kind ?? null,
        headStyle: arrowHead.head ?? null,
        tailStyle: arrowHead.tail ?? null,
        fillType: arrowHead.fillType ?? null,
        noGo: arrowHead.noGo ?? null,
        curve: arrowHead.curve ?? null,
        bold: arrowHead.bold ?? null,
        points: roundGeometry(payloadValue(object, "points") ?? []),
      };
    })
    .map((signature) => JSON.stringify(canonicalize(signature)))
    .sort();
}

function bracketSignatures(objects) {
  return objects
    .filter((object) => object.type === "bracket")
    .map((object) => ({
      kind: payloadValue(object, "kind") ?? null,
      side: payloadValue(object, "side") ?? null,
      bbox: roundGeometry(payloadValue(object, "bbox") ?? null),
      translate: roundGeometry(object.transform?.translate ?? null),
    }))
    .map((signature) => JSON.stringify(canonicalize(signature)))
    .sort();
}

function semanticSnapshot(document) {
  const objects = flattenObjects(document.objects);
  const molecules = moleculeSignatures(document, objects);
  const labels = moleculeLabelSignatures(document, objects);
  const texts = textSignatures(objects);
  const arrows = arrowSignatures(objects);
  const brackets = bracketSignatures(objects);
  const metrics = {
    moleculeObjects: objects.filter((object) => object.type === "molecule").length,
    labels: labels.length,
    textObjects: texts.length,
    arrows: arrows.length,
    headlessArrows: objects.filter(
      (object) =>
        object.type === "line" &&
        payloadValue(object, "arrowHead") &&
        (payloadValue(object, "arrowHead").head ?? "none") === "none" &&
        (payloadValue(object, "arrowHead").tail ?? "none") === "none",
    ).length,
    brackets: brackets.length,
    bracketGroups: objects.filter((object) => object.meta?.kind === "bracket-group").length,
  };
  const components = {
    molecules: digest(molecules),
    labels: digest(labels),
    texts: digest(texts),
    arrows: digest(arrows),
    brackets: digest(brackets),
  };
  return {
    hash: digest({ components, metrics }),
    components,
    metrics,
    geometry: {
      labels: moleculeLabelGeometry(document, objects),
      texts: textGeometry(objects),
    },
  };
}

function readGeneration(path) {
  const document = JSON.parse(readFileSync(path, "utf8"));
  return { counts: summarizeCounts(document), semantic: semanticSnapshot(document) };
}

const countKeys = ["molecules", "nodes", "bonds", "objects", "resources", "styles"];

function compareCounts(before, after) {
  const delta = Object.fromEntries(countKeys.map((key) => [key, (after[key] || 0) - (before[key] || 0)]));
  const objectTypes = [
    ...new Set([...Object.keys(before.objectTypes || {}), ...Object.keys(after.objectTypes || {})]),
  ].sort();
  const objectTypeDelta = Object.fromEntries(
    objectTypes.map((key) => [key, (after.objectTypes?.[key] || 0) - (before.objectTypes?.[key] || 0)]),
  );
  return {
    exact:
      Object.values(delta).every((value) => value === 0) &&
      Object.values(objectTypeDelta).every((value) => value === 0),
    delta,
    objectTypeDelta,
  };
}

function compareSemantic(before, after) {
  const changed = Object.keys(before.components).filter(
    (component) => before.components[component] !== after.components[component],
  );
  for (const component of ["labels", "texts"]) {
    if (!compareVisualGeometry(before.geometry?.[component], after.geometry?.[component])) {
      if (!changed.includes(component)) changed.push(component);
    }
  }
  return { exact: before.hash === after.hash && changed.length === 0, changed };
}

function compareVisualGeometry(before = [], after = [], tolerance = 0.5) {
  if (before.length !== after.length) return false;
  const close = (left, right) => {
    if (left == null || right == null) return left === right;
    if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) return false;
    return left.every((value, index) => Math.abs(value - right[index]) <= tolerance);
  };
  return before.every(
    (item, index) =>
      item.key === after[index]?.key &&
      close(item.position, after[index].position) &&
      close(item.box, after[index].box) &&
      close(item.lineHeight, after[index].lineHeight),
  );
}

function failureMessage(result) {
  return result.stderr || result.stdout || result.error;
}

const versionResult = runCli(["version"]);
if (!versionResult.ok) {
  throw new Error(`Unable to read CLI version metadata: ${failureMessage(versionResult)}`);
}
const cliMetadata = JSON.parse(versionResult.stdout);
const repository = repositoryState(repoRoot);
if (cliMetadata.buildIdentity !== repository.identity) {
  throw new Error(
    `CLI build identity ${cliMetadata.buildIdentity ?? "(missing)"} does not match repository `
    + `identity ${repository.identity}. Run node scripts/build-public-cdxml-cli.mjs first.`,
  );
}
const cliIdentity = {
  path: cliPath,
  version: cliMetadata.version,
  sha256: sha256File(cliPath),
  buildIdentity: cliMetadata.buildIdentity,
};
const cases = [];
let caseIndex = 0;

for (const source of manifest.sources) {
  const sourceRoot = join(corpusRoot, source.id);
  if (!existsSync(sourceRoot)) {
    throw new Error(`Missing source ${source.id}. Run npm run benchmark:cdxml-public:fetch first.`);
  }
  const specialCases = new Map((source.specialCases || []).map((item) => [item.path, item]));
  for (const inputPath of collectFiles(sourceRoot)) {
    if (limit > 0 && cases.length >= limit) break;
    caseIndex += 1;
    const relativePath = normalizedRelative(sourceRoot, inputPath);
    const special = specialCases.get(relativePath);
    const classification = special?.class || "valid";
    const format = extname(inputPath).toLowerCase().slice(1);
    const key = String(caseIndex).padStart(4, "0");
    const record = {
      caseId: key,
      source: source.id,
      path: relativePath,
      format,
      classification,
      status: "pending",
    };

    if (classification === "transport-encoded") {
      record.status = "skipped";
      record.reason = special.reason;
      cases.push(record);
      continue;
    }

    const modelPaths = Array.from({ length: generations + 1 }, (_, index) =>
      join(workRoot, `${key}-generation-${index}.ccjs`),
    );
    const roundTripPaths = Array.from({ length: generations }, (_, index) =>
      join(workRoot, `${key}-generation-${index + 1}.${format}`),
    );
    for (const path of [...modelPaths, ...roundTripPaths]) rmSync(path, { force: true });

    const initialRun = runCli(["convert", inputPath, modelPaths[0]]);
    if (classification === "invalid") {
      record.status = initialRun.ok ? "unexpected-accept" : "expected-reject";
      record.reason = special.reason;
      if (!initialRun.ok) record.error = failureMessage(initialRun);
      cases.push(record);
      continue;
    }
    if (!initialRun.ok) {
      record.status = "import-failed";
      record.error = failureMessage(initialRun);
      cases.push(record);
      continue;
    }

    record.generations = [readGeneration(modelPaths[0])];
    const sourceDeclaresBonds = format === "cdxml"
      && /<b\b/i.test(readFileSync(inputPath, "utf8"));
    if (sourceDeclaresBonds && record.generations[0].counts.bonds === 0) {
      record.status = "topology-lost";
      record.error = "Source CDXML declares bonds but the imported document contains none.";
      cases.push(record);
      continue;
    }
    let previousPath = inputPath;
    for (let generation = 1; generation <= generations; generation += 1) {
      const exportRun = runCli(["convert", previousPath, roundTripPaths[generation - 1]]);
      if (!exportRun.ok) {
        record.status = "export-failed";
        record.failedGeneration = generation;
        record.error = failureMessage(exportRun);
        break;
      }
      const importRun = runCli(["convert", roundTripPaths[generation - 1], modelPaths[generation]]);
      if (!importRun.ok) {
        record.status = "reimport-failed";
        record.failedGeneration = generation;
        record.error = failureMessage(importRun);
        break;
      }
      record.generations.push(readGeneration(modelPaths[generation]));
      previousPath = roundTripPaths[generation - 1];
    }
    if (record.status !== "pending") {
      cases.push(record);
      continue;
    }

    record.comparisons = record.generations.slice(1).map((generation, index) => ({
      from: index,
      to: index + 1,
      counts: compareCounts(record.generations[index].counts, generation.counts),
      semantic: compareSemantic(record.generations[index].semantic, generation.semantic),
    }));
    for (const generation of record.generations) delete generation.semantic.geometry;
    const initial = record.comparisons[0];
    const later = record.comparisons.slice(1);
    const laterStable = later.every((comparison) => comparison.counts.exact && comparison.semantic.exact);

    if (!laterStable) {
      record.status = "non-idempotent";
    } else if (!initial.semantic.exact) {
      record.status = "semantic-drift";
    } else if (classification === "normalization") {
      record.status = "expected-normalization";
      record.reason = special.reason;
    } else if (classification === "sanitized") {
      record.status = "expected-sanitization";
      record.reason = special.reason;
    } else if (!initial.counts.exact) {
      record.status = "count-drift";
    } else {
      record.status = "exact";
    }
    cases.push(record);
  }
  if (limit > 0 && cases.length >= limit) break;
}

const statuses = {};
const bySource = {};
for (const item of cases) {
  statuses[item.status] = (statuses[item.status] || 0) + 1;
  const source = (bySource[item.source] ||= { total: 0, statuses: {} });
  source.total += 1;
  source.statuses[item.status] = (source.statuses[item.status] || 0) + 1;
}

const unexpectedStatuses = new Set([
  "unexpected-accept",
  "import-failed",
  "export-failed",
  "reimport-failed",
  "topology-lost",
  "semantic-drift",
  "non-idempotent",
]);
const unexpectedFailures = cases.filter((item) => unexpectedStatuses.has(item.status));
const countDrift = cases.filter((item) => item.status === "count-drift");
const report = {
  schema: "chemsema.public-cdxml-roundtrip-report.v3",
  generatedAt: new Date().toISOString(),
  cli: cliIdentity,
  repositoryIdentity: repository.identity,
  generations,
  manifest: normalizedRelative(repoRoot, manifestPath),
  corpusRoot,
  summary: {
    total: cases.length,
    statuses,
    bySource,
    unexpectedFailures: unexpectedFailures.length,
    countDrift: countDrift.length,
  },
  cases,
};
mkdirSync(dirname(reportPath), { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
if (summaryOutput) {
  const summaryPath = resolve(summaryOutput);
  const summaryReport = {
    schema: "chemsema.public-cdxml-roundtrip-summary.v3",
    generatedAt: report.generatedAt,
    cli: cliIdentity,
    repositoryIdentity: repository.identity,
    generations,
    sources: manifest.sources.map((source) => ({
      id: source.id,
      revision: source.revision,
      license: source.license.spdx,
      expectedFiles: source.expectedFiles,
    })),
    summary: report.summary,
  };
  mkdirSync(dirname(summaryPath), { recursive: true });
  writeFileSync(summaryPath, `${JSON.stringify(summaryReport, null, 2)}\n`, "utf8");
  console.log(`Summary: ${summaryPath}`);
}

console.log(JSON.stringify(report.summary, null, 2));
console.log(`Report: ${reportPath}`);
if (unexpectedFailures.length > 0 || (strictCounts && countDrift.length > 0)) process.exitCode = 1;
