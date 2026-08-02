import crypto from "node:crypto";
import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export const FEATURE_INDEX_SCHEMA = "chemsema.public_cdxml.feature_index.v4";
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
  addIf(
    features,
    /<objecttag\b[^>]*\bname\s*=\s*["']query["']/.test(text)
      || /\b(?:implicithydrogens|freesites|ringbondcount|unsaturatedbonds|substituentsupto|substituentsexactly|translation|isotopicabundance)\s*=/.test(text)
      || /\bnodetype\s*=\s*["'](?:elementlist|elementlistnickname|genericnickname)["']/.test(text)
      || /\b(?:elementlist|genericlist)\s*=/.test(text),
    "query",
  );
  addIf(features, /\bshow(?:bond|atom)query\s*=/.test(text), "query-visibility");
  addIf(features, /wedgedhash/.test(text), "hashed-wedge");
  addIf(features, /display\s*=\s*["']wedge(?:begin|end)["']/.test(text), "solid-wedge");
  addIf(features, /display\s*=\s*["'](?:dash|hash)["']/.test(text), "dashed-bond");
  addIf(features, /display\s*=\s*["']wavy["']/.test(text), "wavy-bond");
  addIf(features, /\border\s*=\s*["']dative["']/.test(text), "dative-bond");
  addIf(
    features,
    /\border\s*=\s*["'](?:1\.5(?:0+)?|2(?:\.0+)?)["']/.test(text),
    "double-bond",
  );
  addIf(features, /\border\s*=\s*["']3(?:\.0+)?["']/.test(text), "triple-bond");
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

async function featuresFromCdx({ cli, sourcePath, temporaryRoot, caseId }) {
  const convertedPath = path.join(temporaryRoot, `${caseId}.cdxml`);
  await execFileAsync(cli, [
    "convert", sourcePath, convertedPath, "--format", "cdxml",
  ], { maxBuffer: 32 * 1024 * 1024 });
  const features = new Set(featuresFromCdxml(await fs.readFile(convertedPath, "utf8")));
  features.delete("cdxml");
  features.add("cdx");
  return [...features].sort();
}

function conservativeCdxFeatures() {
  return [
    "arrow", "bond", "bracket", "cdx", "dashed-bond", "double-bond", "enhanced-stereo",
    "dative-bond", "external-connection", "graphics", "hashed-wedge", "hydrogen-marker",
    "multi-attachment", "nickname", "object-tag", "query", "solid-wedge", "triple-bond",
    "symbol", "text", "wavy-bond",
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
  const temporaryRoot = await fs.mkdtemp(path.join(path.resolve(root), ".feature-index-"));
  let cases;
  try {
    cases = await mapConcurrent(report.cases, Math.max(1, jobs), async (entry) => {
      const sourcePath = path.resolve(root, entry.source, entry.path);
      const bytes = await fs.readFile(sourcePath);
      let features;
      let inspectionError = null;
      if (entry.format === "cdxml") {
        features = featuresFromCdxml(bytes.toString("utf8"));
      } else {
        try {
          features = await featuresFromCdx({
            cli,
            sourcePath,
            temporaryRoot,
            caseId: entry.caseId,
          });
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
  } finally {
    await fs.rm(temporaryRoot, { recursive: true, force: true });
  }
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
