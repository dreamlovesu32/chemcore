import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { compareSvgPixels } from "./compare-svg-pixels.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const geometryOnly = process.argv.includes("--geometry-only");
const probeArgument = process.argv.slice(2).find((argument) => !argument.startsWith("--"));
const probeDir = path.resolve(
  root,
  probeArgument ?? "tmp/chemdraw-bioshape-geometry-probe",
);
const manifest = JSON.parse(await fs.readFile(path.join(probeDir, "manifest.json"), "utf8"));
if (manifest.schema !== "chemsema.chemdraw-bioshape-geometry-probe.v1") {
  throw new Error("Unsupported BioShape geometry probe manifest.");
}
const comparisonDir = path.join(probeDir, "analysis");
await fs.mkdir(comparisonDir, { recursive: true });

function svgStats(svg) {
  const paths = [...svg.matchAll(/<path\b([^>]*)>/gi)].map((match) => match[1]);
  const circles = [...svg.matchAll(/<circle\b/gi)].length;
  const ellipses = [...svg.matchAll(/<ellipse\b/gi)].length;
  return {
    pathCount: paths.length,
    circleCount: circles,
    ellipseCount: ellipses,
    strokedPathCount: paths.filter((value) => /stroke="(?!none)[^"]+"/i.test(value)).length,
    filledPathCount: paths.filter((value) => /fill="(?!none)[^"]+"/i.test(value)).length,
    cubicPathCount: paths.filter((value) => /\bd="[^"]*\bC\b/i.test(value)).length,
    arcPathCount: paths.filter((value) => /\bd="[^"]*\bA\b/i.test(value)).length,
  };
}

function normalizedCubicPaths(svg, axes) {
  const pathTags = [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="(?!none)[^"]+"/i.test(attributes));
  const center = axes.center;
  const major = [axes.major[0] - center[0], axes.major[1] - center[1]];
  const minor = [axes.minor[0] - center[0], axes.minor[1] - center[1]];
  const determinant = major[0] * minor[1] - major[1] * minor[0];
  if (Math.abs(determinant) < 1e-9) throw new Error("Degenerate BioShape axes.");
  return pathTags.map((attributes) => {
    const d = attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "";
    if (/[^MCLCZ0-9eE+.,\s-]/.test(d)) return null;
    const numbers = [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)]
      .map((match) => Number(match[0]) / 20);
    if (numbers.length % 2 !== 0) return null;
    const normalized = [];
    for (let index = 0; index < numbers.length; index += 2) {
      const dx = numbers[index] - center[0];
      const dy = numbers[index + 1] - center[1];
      normalized.push(
        (dx * minor[1] - dy * minor[0]) / determinant,
        (major[0] * dy - major[1] * dx) / determinant,
      );
    }
    return normalized;
  });
}

function affineTemplateError(basePaths, candidatePaths) {
  if (
    basePaths.some((path) => path === null)
    || candidatePaths.some((path) => path === null)
    || basePaths.length !== candidatePaths.length
  ) {
    return null;
  }
  let maximum = 0;
  for (let pathIndex = 0; pathIndex < basePaths.length; pathIndex += 1) {
    const base = basePaths[pathIndex];
    const candidate = candidatePaths[pathIndex];
    if (base.length !== candidate.length) return null;
    for (let index = 0; index < base.length; index += 1) {
      maximum = Math.max(maximum, Math.abs(base[index] - candidate[index]));
    }
  }
  return maximum;
}

const groups = new Map();
for (const entry of manifest.cases) {
  const group = groups.get(entry.type) ?? [];
  group.push(entry);
  groups.set(entry.type, group);
}

const reports = [];
for (const [type, entries] of groups) {
  const base = entries.find((entry) => entry.variant === "base");
  if (!base) throw new Error(`${type}: base case missing`);
  const basePath = path.join(root, base.svg);
  const baseSvg = await fs.readFile(basePath, "utf8");
  const baseNormalizedPaths = normalizedCubicPaths(baseSvg, base.axes);
  const variants = [];
  for (const entry of entries) {
    const svgPath = path.join(root, entry.svg);
    const svg = await fs.readFile(svgPath, "utf8");
    const variant = {
      name: entry.variant,
      stats: svgStats(svg),
      normalizedTag: entry.normalizedTag,
      affineTemplateError: affineTemplateError(
        baseNormalizedPaths,
        normalizedCubicPaths(svg, entry.axes),
      ),
    };
    if (!geometryOnly && entry !== base && entry.variant !== "rotated") {
      const comparison = await compareSvgPixels({
        outDir: path.join(comparisonDir, `${type}-${entry.variant}`),
        leftPath: basePath,
        rightPath: svgPath,
        leftLabel: `${type} base`,
        rightLabel: entry.variant,
        baseScale: 2,
        searchLimit: 12,
      });
      variant.aspectSensitivePixelMismatch =
        comparison.differentPixels / comparison.unionPixels;
      variant.alignedIou = comparison.bestShift.iou;
    }
    variants.push(variant);
  }
  reports.push({ type, baseStats: svgStats(baseSvg), variants });
  console.log(`[BIOSHAPE RULE] ${type}: paths=${svgStats(baseSvg).pathCount} variants=${variants.length}`);
}

await fs.writeFile(
  path.join(probeDir, "analysis.json"),
  `${JSON.stringify({
    schema: "chemsema.chemdraw-bioshape-geometry-analysis.v1",
    reports,
  }, null, 2)}\n`,
);
