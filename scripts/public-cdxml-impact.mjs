import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export const FEATURE_INDEX_SCHEMA = "chemsema.public_cdxml.feature_index.v1";
export const AFFECTED_PLAN_SCHEMA = "chemsema.public_cdxml.affected_gate_plan.v1";

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function addIf(features, condition, name) {
  if (condition) features.add(name);
}

export function featuresFromCdxml(source) {
  const text = source.toLowerCase();
  const features = new Set(["cdxml"]);
  addIf(features, /<b\b/.test(text), "bond");
  addIf(features, /<t\b|<s\b/.test(text), "text");
  addIf(features, /<graphic\b/.test(text), "graphics");
  addIf(features, /<objecttag\b/.test(text), "object-tag");
  addIf(features, /enhancedstereo/.test(text), "enhanced-stereo");
  addIf(features, /name\s*=\s*["']query["']|showbondquery|showatomquery/.test(text), "query");
  addIf(features, /wedgedhash/.test(text), "hashed-wedge");
  addIf(features, /display\s*=\s*["']wedge(?:begin|end)["']/.test(text), "solid-wedge");
  addIf(features, /display\s*=\s*["'](?:dash|hash)["']/.test(text), "dashed-bond");
  addIf(features, /nodetype\s*=\s*["']nickname["']/.test(text), "nickname");
  addIf(features, /nodetype\s*=\s*["']externalconnectionpoint["']/.test(text), "external-connection");
  addIf(features, /bracketedgroup|bracketattachment|graphictype\s*=\s*["']bracket["']/.test(text), "bracket");
  addIf(features, /<symbol\b|symboltype/.test(text), "symbol");
  addIf(features, /nodetype\s*=\s*["']multiattachment["']/.test(text), "multi-attachment");
  addIf(features, /\bhdot\s*=|\bhdash\s*=/.test(text), "hydrogen-marker");
  addIf(features, /<arrow\b|arrowhead|arrowtail/.test(text), "arrow");
  addIf(features, /crossingbonds/.test(text), "bond-crossing");
  return [...features].sort();
}

function featuresFromInspection(inspection) {
  const features = new Set(["cdx"]);
  for (const molecule of inspection.molecules ?? []) {
    if ((molecule.bonds ?? []).length) features.add("bond");
    for (const bond of molecule.bonds ?? []) {
      const kind = bond.stereo?.kind;
      if (kind === "hashed-wedge") features.add("hashed-wedge");
      if (kind === "solid-wedge" || kind === "hollow-wedge") features.add("solid-wedge");
      if (Object.values(bond.lineStyles ?? {}).some((style) => style === "dashed" || style === "hash")) {
        features.add("dashed-bond");
      }
    }
    for (const node of molecule.nodes ?? []) {
      if (node.label) features.add("text");
      if (node.isPlaceholder) features.add("nickname");
      if (node.externalConnection) features.add("external-connection");
      const nodeType = node.meta?.import?.cdxml?.nodeType;
      if (nodeType === "MultiAttachment") features.add("multi-attachment");
      if (node.meta?.import?.cdxml?.hDot || node.meta?.import?.cdxml?.hDash) features.add("hydrogen-marker");
      if (node.meta?.import?.cdxml?.enhancedStereoType) features.add("enhanced-stereo");
    }
  }
  for (const object of inspection.objects ?? []) {
    const type = String(object.type ?? "");
    if (type === "text") features.add("text");
    if (type.includes("bracket")) features.add("bracket");
    if (type.includes("arrow")) features.add("arrow");
    if (type === "symbol") features.add("symbol");
    if (type && type !== "molecule" && type !== "text") features.add("graphics");
  }
  return [...features].sort();
}

function conservativeCdxFeatures() {
  return [
    "arrow", "bond", "bracket", "cdx", "dashed-bond", "enhanced-stereo",
    "external-connection", "graphics", "hashed-wedge", "hydrogen-marker",
    "multi-attachment", "nickname", "object-tag", "query", "solid-wedge",
    "symbol", "text",
  ];
}

async function mapConcurrent(values, concurrency, mapper) {
  const output = new Array(values.length);
  let cursor = 0;
  async function worker() {
    while (cursor < values.length) {
      const index = cursor;
      cursor += 1;
      output[index] = await mapper(values[index], index);
    }
  }
  await Promise.all(Array.from({ length: Math.min(concurrency, values.length) }, worker));
  return output;
}

export async function buildFeatureIndex({ root, report, cli, jobs = 8 }) {
  const cases = await mapConcurrent(report.cases, Math.max(1, jobs), async (entry) => {
    const sourcePath = path.resolve(root, entry.source, entry.path);
    const bytes = await fs.readFile(sourcePath);
    let features;
    let inspectionError = null;
    if (entry.format === "cdxml") {
      features = featuresFromCdxml(bytes.toString("utf8"));
    } else {
      try {
        const { stdout } = await execFileAsync(cli, [
          "inspect", sourcePath, "--include", "objects,molecules",
        ], { maxBuffer: 32 * 1024 * 1024 });
        features = featuresFromInspection(JSON.parse(stdout));
      } catch (error) {
        features = conservativeCdxFeatures();
        inspectionError = error instanceof Error ? error.message : String(error);
      }
    }
    return {
      caseId: entry.caseId,
      relativeCdxml: `${entry.source}/${entry.path}`.replaceAll("\\", "/"),
      sourcePath,
      format: entry.format,
      status: entry.status,
      sourceHash: sha256(bytes),
      features,
      ...(inspectionError ? { inspectionError } : {}),
    };
  });
  const byFeature = {};
  for (const entry of cases) {
    for (const feature of entry.features) {
      (byFeature[feature] ??= []).push(entry.caseId);
    }
  }
  return {
    schema: FEATURE_INDEX_SCHEMA,
    generatedAt: new Date().toISOString(),
    count: cases.length,
    byFeature,
    cases,
  };
}

function pathMatchesRule(file, rule) {
  return (rule.pathEquals ?? []).includes(file)
    || (rule.pathSubstrings ?? []).some((part) => file.includes(part))
    || (rule.pathPrefixes ?? []).some((prefix) => file.startsWith(prefix));
}

export function selectAffectedCases({ changedFiles, featureIndex, impactMap, extras = [] }) {
  const normalizedFiles = [...new Set(changedFiles.map((file) => file.replaceAll("\\", "/")))].sort();
  const matchedRules = impactMap.rules.filter((rule) =>
    normalizedFiles.some((file) => pathMatchesRule(file, rule)));
  const ignored = (file) => (impactMap.ignoredPathPrefixes ?? []).some((prefix) => file.startsWith(prefix));
  const production = (file) => (impactMap.productionPathPrefixes ?? []).some((prefix) => file.startsWith(prefix));
  const unmatchedProductionFiles = normalizedFiles.filter((file) =>
    production(file) && !ignored(file) && !matchedRules.some((rule) => pathMatchesRule(file, rule)));
  const forceFull = matchedRules.some((rule) => rule.full)
    || (unmatchedProductionFiles.length > 0 && impactMap.unknownProductionChange === "full");
  const extraMatches = (entry) => extras.some((extra) => {
    const needle = String(extra).toLowerCase();
    return entry.caseId === needle || entry.relativeCdxml.toLowerCase().includes(needle);
  });
  const regressionIds = new Set(matchedRules.flatMap((rule) => rule.regressionCases ?? []));
  const selected = featureIndex.cases.filter((entry) => {
    if (forceFull || extraMatches(entry) || regressionIds.has(entry.caseId)) return true;
    return matchedRules.some((rule) => {
      if (rule.formats?.length && !rule.formats.includes(entry.format)) return false;
      return rule.features?.some((feature) => entry.features.includes(feature));
    });
  });
  return {
    changedFiles: normalizedFiles,
    matchedRules: matchedRules.map((rule) => rule.name),
    unmatchedProductionFiles,
    forceFull,
    selected,
  };
}
