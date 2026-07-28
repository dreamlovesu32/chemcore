import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchBrowser } from "./playwright-browser.mjs";
import {
  computeImageAlignment,
  mapWithConcurrency,
  viewerHtml,
} from "./render-public-cdxml-visual-review.mjs";

const DEFAULTS = Object.freeze({
  gallery: "tmp/public-cdxml-chemdraw-review-all",
  out: "tmp/public-cdxml-visual-gate/report.json",
  jobs: 4,
  analysisScale: 2,
  tolerance: 1.5,
  tileSize: 256,
  halo: 24,
  localWindow: 48,
  localStride: 24,
  minimumWindowInk: 4,
  minCoverage: 0.75,
  maxDefectArea: 8,
  maxDefectSpan: 12,
  detailAnalysisScale: 4,
  detailTolerance: 0.25,
  detailLocalWindow: 24,
  detailLocalStride: 12,
  detailMinimumWindowInk: 12,
  maxComponentCountDelta: 1,
  maxEnclosedSmallComponentDimensionDelta: 2.75,
  maxRepeatedMicroDefects: 7,
  maxRepeatedMicroDefectArea: 2,
  minRepeatedMicroCoverage: 0.9,
  minimumTopologyComponentCount: 8,
  minimumSmallTopologyComponentCount: 3,
  minimumSmallTopologyLocalCoverage: 0.7,
  maximumTopologyCandidateComponentCount: 300,
  maxTopologyCandidateCountRatio: 0.1,
  maxRelativeComponentCenterDistance: 0.02,
  maxComponentPositionDistributionDelta: 0.03,
  minStrongPixelCoverage: 0.99,
  minStrongPixelLocalCoverage: 0.9,
  maxStrongPixelComponentCountDelta: 2,
  minSlenderDefectCoverage: 0.98,
  minSlenderDefectLocalCoverage: 0.75,
  maxSlenderDefectArea: 24,
  maxSlenderDefectSpan: 30,
  maxSlenderDefectThickness: 1,
  minBoundedLocalCoverage: 0.96,
  maxBoundedLocalDefectSpan: 32,
  minBoundedRelativeComponentCoverage: 0.877,
  boundedComponentDeltaPenalty: 0.01,
  maxBoundedComponentCountDelta: 8,
  maxTightBoundedLocalDefectSpan: 20,
  minTightBoundedRelativeComponentCoverage: 0.88,
  maxTightBoundedComponentCountDelta: 5,
  minNearExactCoverage: 0.994,
  maxNearExactDefectSpan: 15,
  maxNearExactDefectArea: 18,
});

const ALIGNMENT_ALGORITHM = "ink-iou-coarse-refined-precision-v5";
const CACHE_IDENTITY = "chemsema-public-cdxml-visual-gate-cache-v7";

function parseArgs(argv) {
  const options = { ...DEFAULTS, patterns: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--gallery") options.gallery = argv[++index];
    else if (arg === "--out") options.out = argv[++index];
    else if (arg === "--passed-gallery") options.passedGallery = argv[++index];
    else if (arg === "--reuse-report") options.reuseReport = argv[++index];
    else if (arg === "--baseline-report") options.baselineReport = argv[++index];
    else if (arg === "--stamp-report") options.stampReport = argv[++index];
    else if (arg === "--only") options.patterns.push(argv[++index]);
    else if (arg === "--limit") options.limit = Number(argv[++index]);
    else if (arg === "--jobs") options.jobs = Number(argv[++index]);
    else if (arg === "--analysis-scale") options.analysisScale = Number(argv[++index]);
    else if (arg === "--tolerance") options.tolerance = Number(argv[++index]);
    else if (arg === "--tile-size") options.tileSize = Number(argv[++index]);
    else if (arg === "--halo") options.halo = Number(argv[++index]);
    else if (arg === "--local-window") options.localWindow = Number(argv[++index]);
    else if (arg === "--local-stride") options.localStride = Number(argv[++index]);
    else if (arg === "--minimum-window-ink") options.minimumWindowInk = Number(argv[++index]);
    else if (arg === "--min-coverage") options.minCoverage = Number(argv[++index]);
    else if (arg === "--max-defect-area") options.maxDefectArea = Number(argv[++index]);
    else if (arg === "--max-defect-span") options.maxDefectSpan = Number(argv[++index]);
    else if (arg === "--detail-analysis-scale") options.detailAnalysisScale = Number(argv[++index]);
    else if (arg === "--detail-tolerance") options.detailTolerance = Number(argv[++index]);
    else if (arg === "--detail-local-window") options.detailLocalWindow = Number(argv[++index]);
    else if (arg === "--detail-local-stride") options.detailLocalStride = Number(argv[++index]);
    else if (arg === "--detail-minimum-window-ink") options.detailMinimumWindowInk = Number(argv[++index]);
    else if (arg === "--max-component-count-delta") options.maxComponentCountDelta = Number(argv[++index]);
    else if (arg === "--max-enclosed-small-component-dimension-delta") options.maxEnclosedSmallComponentDimensionDelta = Number(argv[++index]);
    else if (arg === "--max-repeated-micro-defects") options.maxRepeatedMicroDefects = Number(argv[++index]);
    else if (arg === "--max-repeated-micro-defect-area") options.maxRepeatedMicroDefectArea = Number(argv[++index]);
    else if (arg === "--min-repeated-micro-coverage") options.minRepeatedMicroCoverage = Number(argv[++index]);
    else if (arg === "--minimum-topology-component-count") options.minimumTopologyComponentCount = Number(argv[++index]);
    else if (arg === "--minimum-small-topology-component-count") options.minimumSmallTopologyComponentCount = Number(argv[++index]);
    else if (arg === "--minimum-small-topology-local-coverage") options.minimumSmallTopologyLocalCoverage = Number(argv[++index]);
    else if (arg === "--maximum-topology-candidate-component-count") options.maximumTopologyCandidateComponentCount = Number(argv[++index]);
    else if (arg === "--max-topology-candidate-count-ratio") options.maxTopologyCandidateCountRatio = Number(argv[++index]);
    else if (arg === "--max-relative-component-center-distance") options.maxRelativeComponentCenterDistance = Number(argv[++index]);
    else if (arg === "--max-component-position-distribution-delta") options.maxComponentPositionDistributionDelta = Number(argv[++index]);
    else if (arg === "--min-strong-pixel-coverage") options.minStrongPixelCoverage = Number(argv[++index]);
    else if (arg === "--min-strong-pixel-local-coverage") options.minStrongPixelLocalCoverage = Number(argv[++index]);
    else if (arg === "--max-strong-pixel-component-count-delta") options.maxStrongPixelComponentCountDelta = Number(argv[++index]);
    else if (arg === "--min-slender-defect-coverage") options.minSlenderDefectCoverage = Number(argv[++index]);
    else if (arg === "--min-slender-defect-local-coverage") options.minSlenderDefectLocalCoverage = Number(argv[++index]);
    else if (arg === "--max-slender-defect-area") options.maxSlenderDefectArea = Number(argv[++index]);
    else if (arg === "--max-slender-defect-span") options.maxSlenderDefectSpan = Number(argv[++index]);
    else if (arg === "--max-slender-defect-thickness") options.maxSlenderDefectThickness = Number(argv[++index]);
    else if (arg === "--min-bounded-local-coverage") options.minBoundedLocalCoverage = Number(argv[++index]);
    else if (arg === "--max-bounded-local-defect-span") options.maxBoundedLocalDefectSpan = Number(argv[++index]);
    else if (arg === "--min-bounded-relative-component-coverage") options.minBoundedRelativeComponentCoverage = Number(argv[++index]);
    else if (arg === "--bounded-component-delta-penalty") options.boundedComponentDeltaPenalty = Number(argv[++index]);
    else if (arg === "--max-bounded-component-count-delta") options.maxBoundedComponentCountDelta = Number(argv[++index]);
    else if (arg === "--max-tight-bounded-local-defect-span") options.maxTightBoundedLocalDefectSpan = Number(argv[++index]);
    else if (arg === "--min-tight-bounded-relative-component-coverage") options.minTightBoundedRelativeComponentCoverage = Number(argv[++index]);
    else if (arg === "--max-tight-bounded-component-count-delta") options.maxTightBoundedComponentCountDelta = Number(argv[++index]);
    else if (arg === "--report-only") options.reportOnly = true;
    else if (arg === "--self-test") options.selfTest = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return options;
}

function validateOptions(options) {
  for (const key of [
    "analysisScale", "tolerance", "tileSize", "halo", "localWindow", "localStride",
    "minimumWindowInk", "detailAnalysisScale", "detailTolerance", "detailLocalWindow",
    "detailLocalStride", "detailMinimumWindowInk",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] <= 0) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be positive`);
    }
  }
  if (!Number.isInteger(options.jobs) || options.jobs < 1) {
    throw new Error("--jobs must be a positive integer");
  }
  for (const key of [
    "minCoverage", "minRepeatedMicroCoverage", "minimumSmallTopologyLocalCoverage",
    "minStrongPixelCoverage", "minStrongPixelLocalCoverage",
    "minSlenderDefectCoverage", "minSlenderDefectLocalCoverage",
    "minBoundedLocalCoverage", "minBoundedRelativeComponentCoverage",
    "minTightBoundedRelativeComponentCoverage", "boundedComponentDeltaPenalty",
    "maxRelativeComponentCenterDistance", "maxTopologyCandidateCountRatio",
    "maxComponentPositionDistributionDelta",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] < 0 || options[key] > 1) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be between 0 and 1`);
    }
  }
  for (const key of [
    "maxDefectArea", "maxDefectSpan", "maxComponentCountDelta",
    "maxEnclosedSmallComponentDimensionDelta", "maxRepeatedMicroDefects",
    "maxRepeatedMicroDefectArea", "minimumTopologyComponentCount",
    "minimumSmallTopologyComponentCount", "maxStrongPixelComponentCountDelta",
    "maxSlenderDefectArea", "maxSlenderDefectSpan", "maxSlenderDefectThickness",
    "maxBoundedLocalDefectSpan", "maxBoundedComponentCountDelta",
    "maxTightBoundedLocalDefectSpan", "maxTightBoundedComponentCountDelta",
    "maximumTopologyCandidateComponentCount",
  ]) {
    if (!Number.isFinite(options[key]) || options[key] < 0) {
      throw new Error(`--${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)} must be non-negative`);
    }
  }
  if (options.halo <= options.tolerance) {
    throw new Error("--halo must be larger than --tolerance");
  }
  if (options.halo < options.localWindow / 2) {
    throw new Error("--halo must be at least half of --local-window");
  }
  if (options.halo <= options.detailTolerance) {
    throw new Error("--halo must be larger than --detail-tolerance");
  }
  if (options.halo < options.detailLocalWindow / 2) {
    throw new Error("--halo must be at least half of --detail-local-window");
  }
}

function mimeType(filePath) {
  return path.extname(filePath).toLowerCase() === ".png" ? "image/png" : "image/svg+xml";
}

async function fileDataUrl(filePath) {
  const bytes = await fs.readFile(filePath);
  return `data:${mimeType(filePath)};base64,${bytes.toString("base64")}`;
}

async function sha256File(filePath) {
  const bytes = await fs.readFile(filePath);
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

async function artifactHashes(galleryDir, item) {
  const [reference, candidate] = await Promise.all([
    sha256File(path.resolve(galleryDir, item.reference)),
    sha256File(path.resolve(galleryDir, item.chemsema)),
  ]);
  return { reference, candidate };
}

function reportsUseSameGateDefinition(report, options) {
  return report?.cacheIdentity === CACHE_IDENTITY
    && JSON.stringify(report.policy) === JSON.stringify(gatePolicy(options));
}

function artifactHashesEqual(left, right) {
  return left?.reference === right?.reference && left?.candidate === right?.candidate;
}

export function classifyBaselineChanges(cases, baselineCases) {
  const changes = cases.flatMap((entry) => {
    const before = baselineCases.get(entry.relativeCdxml)?.status;
    return before && before !== entry.status
      ? [{ relativeCdxml: entry.relativeCdxml, before, after: entry.status }]
      : [];
  });
  return {
    changes,
    regressions: changes.filter((entry) => entry.before === "pass" && entry.after !== "pass"),
    improvements: changes.filter((entry) => entry.before !== "pass" && entry.after === "pass"),
  };
}

async function stampExistingReport(manifest, reportPath, galleryDir) {
  const report = JSON.parse(await fs.readFile(reportPath, "utf8"));
  if (report.gallery && path.resolve(report.gallery) !== galleryDir) {
    throw new Error(`Report gallery does not match --gallery: ${report.gallery}`);
  }
  const casesByPath = new Map(report.cases.map((entry) => [entry.relativeCdxml, entry]));
  let stamped = 0;
  for (const item of manifest.items) {
    const entry = casesByPath.get(item.relativeCdxml);
    if (!entry) continue;
    entry.artifactHashes = await artifactHashes(galleryDir, item);
    stamped += 1;
  }
  report.cacheIdentity = CACHE_IDENTITY;
  report.gallery = galleryDir;
  report.cache = { stamped, reused: 0, analyzed: 0 };
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  return { reportPath, stamped, cacheIdentity: CACHE_IDENTITY };
}

async function oracleIsUnavailable(filePath) {
  if (path.extname(filePath).toLowerCase() !== ".svg") return false;
  const source = await fs.readFile(filePath, "utf8");
  return source.includes("ChemDraw 无法渲染");
}

export async function analyzeAlignedImages(page, referenceDataUrl, candidateDataUrl, alignment, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return page.evaluate(async ({ referenceDataUrl, candidateDataUrl, alignment, settings }) => {
    async function loadImage(src) {
      const image = new Image();
      image.decoding = "sync";
      image.src = src;
      await image.decode();
      return image;
    }

    function maskFromCanvas(canvas, threshold = 740) {
      const pixels = canvas.getContext("2d", { willReadFrequently: true })
        .getImageData(0, 0, canvas.width, canvas.height).data;
      const mask = new Uint8Array(canvas.width * canvas.height);
      let ink = 0;
      for (let index = 0; index < mask.length; index += 1) {
        const offset = index * 4;
        if (pixels[offset] + pixels[offset + 1] + pixels[offset + 2] < threshold) {
          mask[index] = 1;
          ink += 1;
        }
      }
      return { mask, ink };
    }

    function dilate(mask, width, height, radius) {
      const output = new Uint8Array(mask.length);
      const offsets = [];
      for (let dy = -radius; dy <= radius; dy += 1) {
        for (let dx = -radius; dx <= radius; dx += 1) {
          if (dx * dx + dy * dy <= radius * radius) offsets.push([dx, dy]);
        }
      }
      for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
          if (!mask[y * width + x]) continue;
          for (const [dx, dy] of offsets) {
            const nextX = x + dx;
            const nextY = y + dy;
            if (nextX >= 0 && nextX < width && nextY >= 0 && nextY < height) {
              output[nextY * width + nextX] = 1;
            }
          }
        }
      }
      return output;
    }

    function differenceMask(source, dilatedOther) {
      const output = new Uint8Array(source.length);
      for (let index = 0; index < source.length; index += 1) {
        output[index] = source[index] && !dilatedOther[index] ? 1 : 0;
      }
      return output;
    }

    function components(mask, width, height, originX, originY, pixelScale, core) {
      const seen = new Uint8Array(mask.length);
      const queue = new Int32Array(mask.length);
      const result = [];
      for (let start = 0; start < mask.length; start += 1) {
        if (!mask[start] || seen[start]) continue;
        let head = 0;
        let tail = 0;
        queue[tail++] = start;
        seen[start] = 1;
        let area = 0;
        let left = width;
        let top = height;
        let right = -1;
        let bottom = -1;
        while (head < tail) {
          const index = queue[head++];
          const y = Math.floor(index / width);
          const x = index - y * width;
          area += 1;
          left = Math.min(left, x);
          top = Math.min(top, y);
          right = Math.max(right, x);
          bottom = Math.max(bottom, y);
          for (let dy = -1; dy <= 1; dy += 1) {
            for (let dx = -1; dx <= 1; dx += 1) {
              if (dx === 0 && dy === 0) continue;
              const nextX = x + dx;
              const nextY = y + dy;
              if (nextX < 0 || nextX >= width || nextY < 0 || nextY >= height) continue;
              const next = nextY * width + nextX;
              if (!mask[next] || seen[next]) continue;
              seen[next] = 1;
              queue[tail++] = next;
            }
          }
        }
        const centerX = originX + (left + right + 1) / (2 * pixelScale);
        const centerY = originY + (top + bottom + 1) / (2 * pixelScale);
        if (
          centerX < core.x || centerX >= core.x + core.width
          || centerY < core.y || centerY >= core.y + core.height
        ) continue;
        const widthRef = (right - left + 1) / pixelScale;
        const heightRef = (bottom - top + 1) / pixelScale;
        result.push({
          area: area / (pixelScale * pixelScale),
          span: Math.hypot(widthRef, heightRef),
          box: {
            x: originX + left / pixelScale,
            y: originY + top / pixelScale,
            width: widthRef,
            height: heightRef,
          },
        });
      }
      return result;
    }

    function renderTile(referenceImage, candidateImage, tile) {
      const pixelScale = settings.analysisScale;
      const width = Math.max(1, Math.ceil(tile.width * pixelScale));
      const height = Math.max(1, Math.ceil(tile.height * pixelScale));
      function render(image, imageScale, dx, dy) {
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        context.fillStyle = "#ffffff";
        context.fillRect(0, 0, width, height);
        context.drawImage(
          image,
          (dx - tile.x) * pixelScale,
          (dy - tile.y) * pixelScale,
          image.naturalWidth * imageScale * pixelScale,
          image.naturalHeight * imageScale * pixelScale,
        );
        return canvas;
      }
      return {
        reference: render(referenceImage, 1, 0, 0),
        candidate: render(candidateImage, alignment.scale, alignment.dx, alignment.dy),
      };
    }

    const [referenceImage, candidateImage] = await Promise.all([
      loadImage(referenceDataUrl),
      loadImage(candidateDataUrl),
    ]);
    const domain = {
      left: Math.floor(Math.min(0, alignment.dx)),
      top: Math.floor(Math.min(0, alignment.dy)),
      right: Math.ceil(Math.max(
        referenceImage.naturalWidth,
        alignment.dx + candidateImage.naturalWidth * alignment.scale,
      )),
      bottom: Math.ceil(Math.max(
        referenceImage.naturalHeight,
        alignment.dy + candidateImage.naturalHeight * alignment.scale,
      )),
    };
    const radius = Math.max(1, Math.ceil(settings.tolerance * settings.analysisScale));
    const totals = {
      referenceInk: 0,
      candidateInk: 0,
      missingInk: 0,
      extraInk: 0,
      tileCount: 0,
      inkTileCount: 0,
    };
    const local = {
      referenceCoverage: 1,
      candidateCoverage: 1,
      referenceBox: null,
      candidateBox: null,
      windowCount: 0,
    };
    let largestMissing = { area: 0, span: 0, box: null };
    let largestExtra = { area: 0, span: 0, box: null };
    const topDefects = [];
    let compactDefectCount = 0;
    const inkComponents = { reference: [], candidate: [] };

    function recordDefects(kind, entries) {
      for (const entry of entries) {
        const shortSide = Math.min(entry.box.width, entry.box.height);
        const longSide = Math.max(entry.box.width, entry.box.height);
        if (
          entry.area >= 0.5
          && entry.span >= 2.5
          && entry.span <= 12
          && shortSide > 0
          && longSide / shortSide <= 4
        ) compactDefectCount += 1;
        const areaScore = settings.maxDefectArea === 0
          ? (entry.area === 0 ? 0 : Number.POSITIVE_INFINITY)
          : entry.area / settings.maxDefectArea;
        const spanScore = settings.maxDefectSpan === 0
          ? (entry.span === 0 ? 0 : Number.POSITIVE_INFINITY)
          : entry.span / settings.maxDefectSpan;
        const scored = {
          ...entry,
          kind,
          score: Math.max(areaScore, spanScore),
        };
        topDefects.push(scored);
        topDefects.sort((left, right) => right.score - left.score);
        if (topDefects.length > 12) topDefects.length = 12;
        const current = kind === "missing" ? largestMissing : largestExtra;
        if (entry.area > current.area) current.area = entry.area;
        if (entry.span > current.span) current.span = entry.span;
        if (!current.box || scored.score > current.score) {
          current.box = entry.box;
          current.score = scored.score;
        }
      }
    }

    for (let coreY = domain.top; coreY < domain.bottom; coreY += settings.tileSize) {
      for (let coreX = domain.left; coreX < domain.right; coreX += settings.tileSize) {
        const core = {
          x: coreX,
          y: coreY,
          width: Math.min(settings.tileSize, domain.right - coreX),
          height: Math.min(settings.tileSize, domain.bottom - coreY),
        };
        const tile = {
          x: core.x - settings.halo,
          y: core.y - settings.halo,
          width: core.width + settings.halo * 2,
          height: core.height + settings.halo * 2,
        };
        const rendered = renderTile(referenceImage, candidateImage, tile);
        const reference = maskFromCanvas(rendered.reference);
        const candidate = maskFromCanvas(rendered.candidate);
        const candidateDilated = dilate(candidate.mask, rendered.reference.width, rendered.reference.height, radius);
        const referenceDilated = dilate(reference.mask, rendered.reference.width, rendered.reference.height, radius);
        const missing = differenceMask(reference.mask, candidateDilated);
        const extra = differenceMask(candidate.mask, referenceDilated);
        const coreLeft = Math.round(settings.halo * settings.analysisScale);
        const coreTop = coreLeft;
        const coreRight = Math.min(
          rendered.reference.width,
          coreLeft + Math.ceil(core.width * settings.analysisScale),
        );
        const coreBottom = Math.min(
          rendered.reference.height,
          coreTop + Math.ceil(core.height * settings.analysisScale),
        );
        let tileHasInk = false;
        for (let y = coreTop; y < coreBottom; y += 1) {
          for (let x = coreLeft; x < coreRight; x += 1) {
            const index = y * rendered.reference.width + x;
            totals.referenceInk += reference.mask[index];
            totals.candidateInk += candidate.mask[index];
            totals.missingInk += missing[index];
            totals.extraInk += extra[index];
            if (reference.mask[index] || candidate.mask[index]) tileHasInk = true;
          }
        }
        totals.tileCount += 1;
        if (tileHasInk) totals.inkTileCount += 1;
        const firstWindowColumn = Math.ceil(
          (core.x - domain.left - settings.localStride / 2) / settings.localStride,
        );
        const lastWindowColumn = Math.floor(
          (core.x + core.width - domain.left - settings.localStride / 2 - 1e-9)
            / settings.localStride,
        );
        const firstWindowRow = Math.ceil(
          (core.y - domain.top - settings.localStride / 2) / settings.localStride,
        );
        const lastWindowRow = Math.floor(
          (core.y + core.height - domain.top - settings.localStride / 2 - 1e-9)
            / settings.localStride,
        );
        for (let windowRow = firstWindowRow; windowRow <= lastWindowRow; windowRow += 1) {
          for (let windowColumn = firstWindowColumn; windowColumn <= lastWindowColumn; windowColumn += 1) {
            const centerX = domain.left + settings.localStride / 2 + windowColumn * settings.localStride;
            const centerY = domain.top + settings.localStride / 2 + windowRow * settings.localStride;
            const windowBox = {
              x: centerX - settings.localWindow / 2,
              y: centerY - settings.localWindow / 2,
              width: settings.localWindow,
              height: settings.localWindow,
            };
            const left = Math.max(0, Math.floor((windowBox.x - tile.x) * settings.analysisScale));
            const top = Math.max(0, Math.floor((windowBox.y - tile.y) * settings.analysisScale));
            const right = Math.min(
              rendered.reference.width,
              Math.ceil((windowBox.x + windowBox.width - tile.x) * settings.analysisScale),
            );
            const bottom = Math.min(
              rendered.reference.height,
              Math.ceil((windowBox.y + windowBox.height - tile.y) * settings.analysisScale),
            );
            let referenceInk = 0;
            let candidateInk = 0;
            let missingInk = 0;
            let extraInk = 0;
            for (let y = top; y < bottom; y += 1) {
              for (let x = left; x < right; x += 1) {
                const index = y * rendered.reference.width + x;
                referenceInk += reference.mask[index];
                candidateInk += candidate.mask[index];
                missingInk += missing[index];
                extraInk += extra[index];
              }
            }
            const minimumInk = settings.minimumWindowInk
              * settings.analysisScale * settings.analysisScale;
            if (referenceInk >= minimumInk) {
              const coverage = 1 - missingInk / referenceInk;
              if (coverage < local.referenceCoverage) {
                local.referenceCoverage = coverage;
                local.referenceBox = windowBox;
              }
            }
            if (candidateInk >= minimumInk) {
              const coverage = 1 - extraInk / candidateInk;
              if (coverage < local.candidateCoverage) {
                local.candidateCoverage = coverage;
                local.candidateBox = windowBox;
              }
            }
            if (referenceInk >= minimumInk || candidateInk >= minimumInk) local.windowCount += 1;
          }
        }
        if (!reference.ink && !candidate.ink) continue;
        for (const [kind, mask] of [["reference", reference.mask], ["candidate", candidate.mask]]) {
          for (const entry of components(
            mask,
            rendered.reference.width,
            rendered.reference.height,
            tile.x,
            tile.y,
            settings.analysisScale,
            core,
          )) {
            if (entry.area >= 0.25) inkComponents[kind].push(entry);
          }
        }
        recordDefects(
          "missing",
          components(
            missing,
            rendered.reference.width,
            rendered.reference.height,
            tile.x,
            tile.y,
            settings.analysisScale,
            core,
          ),
        );
        recordDefects(
          "extra",
          components(
            extra,
            rendered.reference.width,
            rendered.reference.height,
            tile.x,
            tile.y,
            settings.analysisScale,
            core,
          ),
        );
      }
    }

    const referenceCoverage = totals.referenceInk === 0
      ? (totals.candidateInk === 0 ? 1 : 0)
      : 1 - totals.missingInk / totals.referenceInk;
    const candidateCoverage = totals.candidateInk === 0
      ? (totals.referenceInk === 0 ? 1 : 0)
      : 1 - totals.extraInk / totals.candidateInk;
    const unmatchedCandidate = new Set(inkComponents.candidate.map((_, index) => index));
    let smallComponentDimensionDelta = 0;
    let smallComponentDimensionMismatch = null;
    let enclosedSmallComponentDimensionDelta = 0;
    let enclosedSmallComponentDimensionMismatch = null;
    let matchedComponentCount = 0;
    let maximumMatchedCenterDistance = 0;
    function isEnclosedComponent(component, allComponents) {
      const centerX = component.box.x + component.box.width / 2;
      const centerY = component.box.y + component.box.height / 2;
      return allComponents.some((container) =>
        container !== component
        && container.box.width > component.box.width + 1
        && container.box.height > component.box.height + 1
        && centerX > container.box.x
        && centerX < container.box.x + container.box.width
        && centerY > container.box.y
        && centerY < container.box.y + container.box.height);
    }
    for (const referenceComponent of inkComponents.reference) {
      const referenceCenter = {
        x: referenceComponent.box.x + referenceComponent.box.width / 2,
        y: referenceComponent.box.y + referenceComponent.box.height / 2,
      };
      let best = null;
      for (const index of unmatchedCandidate) {
        const candidateComponent = inkComponents.candidate[index];
        const candidateCenter = {
          x: candidateComponent.box.x + candidateComponent.box.width / 2,
          y: candidateComponent.box.y + candidateComponent.box.height / 2,
        };
        const distance = Math.hypot(
          referenceCenter.x - candidateCenter.x,
          referenceCenter.y - candidateCenter.y,
        );
        const dimensionDistance =
          Math.abs(referenceComponent.box.width - candidateComponent.box.width)
          + Math.abs(referenceComponent.box.height - candidateComponent.box.height);
        const cost = distance + dimensionDistance * 0.25;
        if (distance <= 2 && (!best || cost < best.cost)) {
          best = { index, distance, cost };
        }
      }
      if (!best) continue;
      const candidateComponent = inkComponents.candidate[best.index];
      unmatchedCandidate.delete(best.index);
      matchedComponentCount += 1;
      maximumMatchedCenterDistance = Math.max(maximumMatchedCenterDistance, best.distance);
      const maximumDimension = Math.max(
        referenceComponent.box.width,
        referenceComponent.box.height,
        candidateComponent.box.width,
        candidateComponent.box.height,
      );
      const minimumDimension = Math.min(
        referenceComponent.box.width,
        referenceComponent.box.height,
        candidateComponent.box.width,
        candidateComponent.box.height,
      );
      if (maximumDimension <= 30 && minimumDimension <= 5) {
        const dimensionDelta = Math.max(
          Math.abs(referenceComponent.box.width - candidateComponent.box.width),
          Math.abs(referenceComponent.box.height - candidateComponent.box.height),
        );
        if (dimensionDelta > smallComponentDimensionDelta) {
          smallComponentDimensionDelta = dimensionDelta;
          smallComponentDimensionMismatch = {
            reference: referenceComponent,
            candidate: candidateComponent,
            centerDistance: best.distance,
          };
        }
        if (
          dimensionDelta > enclosedSmallComponentDimensionDelta
          && isEnclosedComponent(referenceComponent, inkComponents.reference)
          && isEnclosedComponent(candidateComponent, inkComponents.candidate)
        ) {
          enclosedSmallComponentDimensionDelta = dimensionDelta;
          enclosedSmallComponentDimensionMismatch = {
            reference: referenceComponent,
            candidate: candidateComponent,
            centerDistance: best.distance,
          };
        }
      }
    }
    const domainWidth = Math.max(domain.right - domain.left, 1);
    const domainHeight = Math.max(domain.bottom - domain.top, 1);
    const relativePairs = [];
    for (const [referenceIndex, referenceComponent] of inkComponents.reference.entries()) {
      const referenceCenter = {
        x: referenceComponent.box.x + referenceComponent.box.width / 2,
        y: referenceComponent.box.y + referenceComponent.box.height / 2,
      };
      for (const [candidateIndex, candidateComponent] of inkComponents.candidate.entries()) {
        const candidateCenter = {
          x: candidateComponent.box.x + candidateComponent.box.width / 2,
          y: candidateComponent.box.y + candidateComponent.box.height / 2,
        };
        const relativeDistance = Math.hypot(
          (referenceCenter.x - candidateCenter.x) / domainWidth,
          (referenceCenter.y - candidateCenter.y) / domainHeight,
        );
        if (relativeDistance <= settings.maxRelativeComponentCenterDistance) {
          const dimensionDistance =
            Math.abs(referenceComponent.box.width - candidateComponent.box.width) / domainWidth
            + Math.abs(referenceComponent.box.height - candidateComponent.box.height) / domainHeight;
          relativePairs.push({
            referenceIndex,
            candidateIndex,
            cost: relativeDistance + dimensionDistance * 0.25,
          });
        }
      }
    }
    relativePairs.sort((left, right) => left.cost - right.cost);
    const relativeMatchedReference = new Set();
    const relativeMatchedCandidate = new Set();
    for (const pair of relativePairs) {
      if (
        relativeMatchedReference.has(pair.referenceIndex)
        || relativeMatchedCandidate.has(pair.candidateIndex)
      ) continue;
      relativeMatchedReference.add(pair.referenceIndex);
      relativeMatchedCandidate.add(pair.candidateIndex);
    }
    const relativeMatchedComponentCount = relativeMatchedReference.size;
    function sortedNormalizedCenters(components, axis) {
      const origin = axis === "x" ? domain.left : domain.top;
      const extent = axis === "x" ? domainWidth : domainHeight;
      return components
        .map((component) => (
          component.box[axis] + component.box[axis === "x" ? "width" : "height"] / 2 - origin
        ) / extent)
        .sort((left, right) => left - right);
    }
    function meanSortedDelta(first, second) {
      if (first.length !== second.length || first.length === 0) return 1;
      return first.reduce((total, value, index) =>
        total + Math.abs(value - second[index]), 0) / first.length;
    }
    const componentPositionDistributionDelta = Math.max(
      meanSortedDelta(
        sortedNormalizedCenters(inkComponents.reference, "x"),
        sortedNormalizedCenters(inkComponents.candidate, "x"),
      ),
      meanSortedDelta(
        sortedNormalizedCenters(inkComponents.reference, "y"),
        sortedNormalizedCenters(inkComponents.candidate, "y"),
      ),
    );
    const reasons = [];
    if (local.referenceCoverage < settings.minCoverage) reasons.push("local-reference-coverage");
    if (local.candidateCoverage < settings.minCoverage) reasons.push("local-candidate-coverage");
    if (largestMissing.area > settings.maxDefectArea) reasons.push("missing-detail-area");
    if (largestExtra.area > settings.maxDefectArea) reasons.push("extra-detail-area");
    if (largestMissing.span > settings.maxDefectSpan) reasons.push("missing-detail-span");
    if (largestExtra.span > settings.maxDefectSpan) reasons.push("extra-detail-span");

    return {
      passed: reasons.length === 0,
      reasons,
      referenceCoverage,
      candidateCoverage,
      local,
      largestMissing,
      largestExtra,
      topDefects,
      detailFeatures: {
        compactDefectCount,
        referenceComponentCount: inkComponents.reference.length,
        candidateComponentCount: inkComponents.candidate.length,
        componentCountDelta: Math.abs(
          inkComponents.reference.length - inkComponents.candidate.length,
        ),
        matchedComponentCount,
        componentMatchCoverage: matchedComponentCount / Math.max(
          inkComponents.reference.length,
          inkComponents.candidate.length,
          1,
        ),
        unmatchedReferenceComponentCount:
          inkComponents.reference.length - matchedComponentCount,
        unmatchedCandidateComponentCount: unmatchedCandidate.size,
        maximumMatchedCenterDistance,
        relativeMatchedComponentCount,
        relativeComponentMatchCoverage: relativeMatchedComponentCount / Math.max(
          inkComponents.reference.length,
          inkComponents.candidate.length,
          1,
        ),
        componentPositionDistributionDelta,
        unmatchedRelativeReferenceComponents: inkComponents.reference
          .filter((_, index) => !relativeMatchedReference.has(index))
          .slice(0, 12),
        unmatchedRelativeCandidateComponents: inkComponents.candidate
          .filter((_, index) => !relativeMatchedCandidate.has(index))
          .slice(0, 12),
        smallComponentDimensionDelta,
        smallComponentDimensionMismatch,
        enclosedSmallComponentDimensionDelta,
        enclosedSmallComponentDimensionMismatch,
      },
      totals,
      domain,
      settings: {
        analysisScale: settings.analysisScale,
        tolerance: settings.tolerance,
        tileSize: settings.tileSize,
        halo: settings.halo,
        localWindow: settings.localWindow,
        localStride: settings.localStride,
        minimumWindowInk: settings.minimumWindowInk,
        minCoverage: settings.minCoverage,
        maxDefectArea: settings.maxDefectArea,
        maxDefectSpan: settings.maxDefectSpan,
      },
    };
  }, { referenceDataUrl, candidateDataUrl, alignment, settings });
}

export function detailGateReasons(detail, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  const reasons = [];
  if (detail.detailFeatures.componentCountDelta > settings.maxComponentCountDelta) {
    reasons.push("detail-component-count");
  }
  if (
    detail.detailFeatures.enclosedSmallComponentDimensionDelta
    > settings.maxEnclosedSmallComponentDimensionDelta
  ) {
    reasons.push("detail-enclosed-small-component-dimension");
  }
  const repeatedMicroDefects =
    detail.detailFeatures.compactDefectCount > settings.maxRepeatedMicroDefects
    && detail.largestMissing.area <= settings.maxRepeatedMicroDefectArea
    && detail.largestExtra.area <= settings.maxRepeatedMicroDefectArea
    && detail.local.referenceCoverage >= settings.minRepeatedMicroCoverage
    && detail.local.candidateCoverage >= settings.minRepeatedMicroCoverage;
  if (repeatedMicroDefects) reasons.push("detail-repeated-micro-defects");
  return reasons;
}

export function fineTopologyEquivalent(detail, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  const features = detail.detailFeatures;
  return Math.min(features.referenceComponentCount, features.candidateComponentCount)
      >= settings.minimumSmallTopologyComponentCount
    && features.componentCountDelta === 0
    && features.componentPositionDistributionDelta
      <= settings.maxComponentPositionDistributionDelta;
}

export function fineTopologyCandidate(coarse, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  const features = coarse.detailFeatures;
  const maximumCount = Math.max(
    features.referenceComponentCount,
    features.candidateComponentCount,
  );
  const minimumCount = Math.min(
    features.referenceComponentCount,
    features.candidateComponentCount,
  );
  const enoughComponents = minimumCount >= settings.minimumTopologyComponentCount
    || (
      minimumCount >= settings.minimumSmallTopologyComponentCount
      && Math.min(coarse.local.referenceCoverage, coarse.local.candidateCoverage)
        >= settings.minimumSmallTopologyLocalCoverage
    );
  return enoughComponents
    && maximumCount <= settings.maximumTopologyCandidateComponentCount
    && features.componentCountDelta / Math.max(maximumCount, 1)
      <= settings.maxTopologyCandidateCountRatio;
}

export function strongPixelEquivalent(coarse, detail, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return coarse.passed
    && Math.min(coarse.referenceCoverage, coarse.candidateCoverage)
      >= settings.minStrongPixelCoverage
    && Math.min(coarse.local.referenceCoverage, coarse.local.candidateCoverage)
      >= settings.minStrongPixelLocalCoverage
    && detail.detailFeatures.componentCountDelta
      <= settings.maxStrongPixelComponentCountDelta;
}

function defectThickness(defect) {
  if (!defect || defect.area === 0) return 0;
  return defect.span > 0 ? defect.area / defect.span : Number.POSITIVE_INFINITY;
}

export function slenderDefectEquivalent(coarse, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return Math.min(coarse.referenceCoverage, coarse.candidateCoverage)
      >= settings.minSlenderDefectCoverage
    && Math.min(coarse.local.referenceCoverage, coarse.local.candidateCoverage)
      >= settings.minSlenderDefectLocalCoverage
    && coarse.largestMissing.area <= settings.maxSlenderDefectArea
    && coarse.largestExtra.area <= settings.maxSlenderDefectArea
    && coarse.largestMissing.span <= settings.maxSlenderDefectSpan
    && coarse.largestExtra.span <= settings.maxSlenderDefectSpan
    && defectThickness(coarse.largestMissing) <= settings.maxSlenderDefectThickness
    && defectThickness(coarse.largestExtra) <= settings.maxSlenderDefectThickness;
}

export function boundedLocalTopologyEquivalent(coarse, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  const features = coarse.detailFeatures;
  const componentDelta = features.componentCountDelta;
  const relativeCoverage = features.relativeComponentMatchCoverage;
  const maximumDefectSpan = Math.max(
    coarse.largestMissing.span,
    coarse.largestExtra.span,
  );
  if (
    Math.min(coarse.referenceCoverage, coarse.candidateCoverage)
      < settings.minBoundedLocalCoverage
    || maximumDefectSpan > settings.maxBoundedLocalDefectSpan
    || componentDelta > settings.maxBoundedComponentCountDelta
  ) {
    return false;
  }
  const tightLocalDefect =
    maximumDefectSpan <= settings.maxTightBoundedLocalDefectSpan
    && componentDelta <= settings.maxTightBoundedComponentCountDelta
    && relativeCoverage >= settings.minTightBoundedRelativeComponentCoverage;
  const topologyAdjustedCoverage =
    settings.minBoundedRelativeComponentCoverage
    + settings.boundedComponentDeltaPenalty * componentDelta;
  return tightLocalDefect || relativeCoverage >= topologyAdjustedCoverage;
}

export function nearExactFixedDefectEquivalent(coarse, options = {}) {
  const settings = { ...DEFAULTS, ...options };
  return Math.min(coarse.referenceCoverage, coarse.candidateCoverage)
      >= settings.minNearExactCoverage
    && coarse.largestMissing.span <= settings.maxNearExactDefectSpan
    && coarse.largestExtra.span <= settings.maxNearExactDefectSpan
    && coarse.largestMissing.area <= settings.maxNearExactDefectArea
    && coarse.largestExtra.area <= settings.maxNearExactDefectArea;
}

function detailAnalysisOptions(options) {
  return {
    analysisScale: options.detailAnalysisScale,
    tolerance: options.detailTolerance,
    tileSize: options.tileSize,
    halo: options.halo,
    localWindow: options.detailLocalWindow,
    localStride: options.detailLocalStride,
    minimumWindowInk: options.detailMinimumWindowInk,
    minCoverage: 0,
    maxDefectArea: Number.MAX_SAFE_INTEGER,
    maxDefectSpan: Number.MAX_SAFE_INTEGER,
  };
}

function gatePolicy(options) {
  return {
    coordinateSpace: "ChemDraw reference image coordinates",
    alignment: "uniform scale plus translation maximizing binary-ink overlap",
    canvasWhitespaceIncluded: false,
    caseWeighting: "one case, one vote",
    comparison: "coarse fixed-window coverage and defects, followed by fine connected-component and repeated-micro-defect checks",
    pass: {
      minimumFixedWindowReferenceCoverage: options.minCoverage,
      minimumFixedWindowCandidateCoverage: options.minCoverage,
      maximumLocalDefectArea: options.maxDefectArea,
      maximumLocalDefectSpan: options.maxDefectSpan,
      maximumFineComponentCountDelta: options.maxComponentCountDelta,
      maximumEnclosedSmallComponentDimensionDelta: options.maxEnclosedSmallComponentDimensionDelta,
      maximumRepeatedMicroDefects: options.maxRepeatedMicroDefects,
      maximumRepeatedMicroDefectArea: options.maxRepeatedMicroDefectArea,
      minimumRepeatedMicroCoverage: options.minRepeatedMicroCoverage,
      minimumTopologyComponentCount: options.minimumTopologyComponentCount,
      minimumSmallTopologyComponentCount: options.minimumSmallTopologyComponentCount,
      minimumSmallTopologyLocalCoverage: options.minimumSmallTopologyLocalCoverage,
      maximumTopologyCandidateComponentCount:
        options.maximumTopologyCandidateComponentCount,
      maximumTopologyCandidateCountRatio: options.maxTopologyCandidateCountRatio,
      maximumRelativeComponentCenterDistance: options.maxRelativeComponentCenterDistance,
      maximumComponentPositionDistributionDelta:
        options.maxComponentPositionDistributionDelta,
      minimumStrongPixelCoverage: options.minStrongPixelCoverage,
      minimumStrongPixelLocalCoverage: options.minStrongPixelLocalCoverage,
      maximumStrongPixelComponentCountDelta: options.maxStrongPixelComponentCountDelta,
      minimumSlenderDefectCoverage: options.minSlenderDefectCoverage,
      minimumSlenderDefectLocalCoverage: options.minSlenderDefectLocalCoverage,
      maximumSlenderDefectArea: options.maxSlenderDefectArea,
      maximumSlenderDefectSpan: options.maxSlenderDefectSpan,
      maximumSlenderDefectThickness: options.maxSlenderDefectThickness,
      minimumBoundedLocalCoverage: options.minBoundedLocalCoverage,
      maximumBoundedLocalDefectSpan: options.maxBoundedLocalDefectSpan,
      minimumBoundedRelativeComponentCoverage:
        options.minBoundedRelativeComponentCoverage,
      boundedComponentDeltaPenalty: options.boundedComponentDeltaPenalty,
      maximumBoundedComponentCountDelta: options.maxBoundedComponentCountDelta,
      maximumTightBoundedLocalDefectSpan:
        options.maxTightBoundedLocalDefectSpan,
      minimumTightBoundedRelativeComponentCoverage:
        options.minTightBoundedRelativeComponentCoverage,
      maximumTightBoundedComponentCountDelta:
        options.maxTightBoundedComponentCountDelta,
      minimumNearExactCoverage: options.minNearExactCoverage,
      maximumNearExactDefectSpan: options.maxNearExactDefectSpan,
      maximumNearExactDefectArea: options.maxNearExactDefectArea,
    },
    raster: {
      pixelsPerReferenceUnit: options.analysisScale,
      toleranceReferenceUnits: options.tolerance,
      tileSizeReferenceUnits: options.tileSize,
      haloReferenceUnits: options.halo,
      localWindowReferenceUnits: options.localWindow,
      localStrideReferenceUnits: options.localStride,
      minimumWindowInkAreaReferenceUnits: options.minimumWindowInk,
    },
    detailRaster: {
      pixelsPerReferenceUnit: options.detailAnalysisScale,
      toleranceReferenceUnits: options.detailTolerance,
      localWindowReferenceUnits: options.detailLocalWindow,
      localStrideReferenceUnits: options.detailLocalStride,
      minimumWindowInkAreaReferenceUnits: options.detailMinimumWindowInk,
    },
  };
}

async function writePassedGallery(manifest, report, galleryDir, requestedPath) {
  const passedGalleryPath = path.resolve(
    requestedPath ?? path.join(galleryDir, "passed.html"),
  );
  const passedIds = new Set(report.cases
    .filter((entry) => entry.status === "pass")
    .map((entry) => entry.id));
  const passedItems = manifest.items.filter((item) => passedIds.has(item.id));
  await fs.mkdir(path.dirname(passedGalleryPath), { recursive: true });
  await fs.writeFile(passedGalleryPath, viewerHtml(passedItems));
  return { passedGalleryPath, count: passedItems.length };
}

async function runSelfTest(options) {
  const svg = (width, height, detail, common = "") => `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <rect width="100%" height="100%" fill="white"/>
    <path d="M 20 40 L 100 40 M 60 20 L 60 80 ${detail} ${common}" fill="none" stroke="black" stroke-width="2"/>
  </svg>`;
  const data = (source) => `data:image/svg+xml;base64,${Buffer.from(source).toString("base64")}`;
  const browser = await launchBrowser({ headless: true });
  const page = await browser.newPage();
  try {
    const alignment = { scale: 1, dx: 0, dy: 0 };
    const small = await analyzeAlignedImages(page, data(svg(128, 96, "M 105 40 L 120 40")), data(svg(128, 96, "")), alignment, options);
    const distantCorrectDetail = "M 1000 900 L 1800 900 M 1400 500 L 1400 1300";
    const large = await analyzeAlignedImages(
      page,
      data(svg(2048, 1536, "M 105 40 L 120 40", distantCorrectDetail)),
      data(svg(2048, 1536, "", distantCorrectDetail)),
      alignment,
      options,
    );
    const identical = await analyzeAlignedImages(page, data(svg(2048, 1536, "")), data(svg(2048, 1536, "")), alignment, options);
    const areaDelta = Math.abs(small.largestMissing.area - large.largestMissing.area);
    const spanDelta = Math.abs(small.largestMissing.span - large.largestMissing.span);
    if (areaDelta > 0.01 || spanDelta > 0.01 || small.passed !== large.passed) {
      throw new Error(`size-independence regression: ${JSON.stringify({ small, large })}`);
    }
    if (!identical.passed) throw new Error(`identical-image regression: ${JSON.stringify(identical)}`);
    const syntheticDetail = {
      detailFeatures: {
        compactDefectCount: options.maxRepeatedMicroDefects + 1,
        componentCountDelta: options.maxComponentCountDelta + 1,
        enclosedSmallComponentDimensionDelta:
          options.maxEnclosedSmallComponentDimensionDelta + 0.25,
      },
      largestMissing: { area: options.maxRepeatedMicroDefectArea },
      largestExtra: { area: options.maxRepeatedMicroDefectArea },
      local: { referenceCoverage: 1, candidateCoverage: 1 },
    };
    const expectedDetailReasons = [
      "detail-component-count",
      "detail-enclosed-small-component-dimension",
      "detail-repeated-micro-defects",
    ];
    const actualDetailReasons = detailGateReasons(syntheticDetail, options);
    if (JSON.stringify(actualDetailReasons) !== JSON.stringify(expectedDetailReasons)) {
      throw new Error(`detail-classifier regression: ${JSON.stringify(actualDetailReasons)}`);
    }
    const topologyDetail = {
      detailFeatures: {
        referenceComponentCount: options.minimumTopologyComponentCount,
        candidateComponentCount: options.minimumTopologyComponentCount,
        componentCountDelta: 0,
        componentPositionDistributionDelta:
          options.maxComponentPositionDistributionDelta,
      },
    };
    if (!fineTopologyEquivalent(topologyDetail, options)) {
      throw new Error("fine-topology acceptance regression");
    }
    topologyDetail.detailFeatures.componentPositionDistributionDelta += 0.001;
    if (fineTopologyEquivalent(topologyDetail, options)) {
      throw new Error("fine-topology position threshold regression");
    }
    const smallTopologyCoarse = {
      local: {
        referenceCoverage: options.minimumSmallTopologyLocalCoverage,
        candidateCoverage: options.minimumSmallTopologyLocalCoverage,
      },
      detailFeatures: {
        referenceComponentCount: options.minimumSmallTopologyComponentCount,
        candidateComponentCount: options.minimumSmallTopologyComponentCount,
        componentCountDelta: 0,
      },
    };
    if (!fineTopologyCandidate(smallTopologyCoarse, options)) {
      throw new Error("small-topology candidate regression");
    }
    smallTopologyCoarse.local.referenceCoverage -= 0.001;
    if (fineTopologyCandidate(smallTopologyCoarse, options)) {
      throw new Error("small-topology local-coverage negative-control regression");
    }
    const slenderCoarse = {
      passed: false,
      referenceCoverage: options.minSlenderDefectCoverage,
      candidateCoverage: options.minSlenderDefectCoverage,
      local: {
        referenceCoverage: options.minSlenderDefectLocalCoverage,
        candidateCoverage: options.minSlenderDefectLocalCoverage,
      },
      largestMissing: { area: 12, span: 12 },
      largestExtra: { area: 0, span: 0 },
    };
    if (!slenderDefectEquivalent(slenderCoarse, options)) {
      throw new Error("slender-defect acceptance regression");
    }
    slenderCoarse.largestMissing.area = options.maxSlenderDefectArea + 0.01;
    if (slenderDefectEquivalent(slenderCoarse, options)) {
      throw new Error("slender-defect area negative-control regression");
    }
    console.log(JSON.stringify({
      passed: true,
      areaDelta,
      spanDelta,
      defectVerdict: small.passed,
      detailReasons: actualDetailReasons,
    }));
  } finally {
    await browser.close();
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log("Usage: node scripts/public-cdxml-visual-gate.mjs [--gallery dir] [--out report.json] [--passed-gallery html] [--only text] [--limit n] [--jobs n] [--report-only]");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --reuse-report report.json [--gallery dir] [--passed-gallery html]");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --gallery dir --stamp-report report.json");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --gallery dir --baseline-report report.json --out report.json");
    console.log("       node scripts/public-cdxml-visual-gate.mjs --self-test");
    return;
  }
  validateOptions(options);
  if (options.selfTest) {
    await runSelfTest(options);
    return;
  }

  const galleryDir = path.resolve(options.gallery);
  const manifestPath = path.join(galleryDir, "manifest.json");
  const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
  if (options.stampReport) {
    console.log(JSON.stringify(await stampExistingReport(
      manifest,
      path.resolve(options.stampReport),
      galleryDir,
    )));
    return;
  }
  if (options.reuseReport) {
    const report = JSON.parse(await fs.readFile(path.resolve(options.reuseReport), "utf8"));
    console.log(JSON.stringify(await writePassedGallery(
      manifest,
      report,
      galleryDir,
      options.passedGallery,
    )));
    return;
  }
  let items = manifest.items.filter((item) => !["expected-reject", "skipped"].includes(item.status));
  if (options.patterns.length) {
    items = items.filter((item) => options.patterns.some((pattern) =>
      `${item.id}\n${item.relativeCdxml}`.toLowerCase().includes(pattern.toLowerCase())));
  }
  if (Number.isFinite(options.limit)) items = items.slice(0, Math.max(0, options.limit));
  if (!items.length) throw new Error("No visual-gate cases matched the requested filters");

  const baselineReport = options.baselineReport
    ? JSON.parse(await fs.readFile(path.resolve(options.baselineReport), "utf8"))
    : null;
  const baselineCases = reportsUseSameGateDefinition(baselineReport, options)
    ? new Map(baselineReport.cases.map((entry) => [entry.relativeCdxml, entry]))
    : new Map();

  const browser = await launchBrowser({ headless: true });
  const context = await browser.newContext();
  const workerCount = Math.min(options.jobs, items.length);
  const pages = await Promise.all(
    Array.from({ length: workerCount }, () => context.newPage()),
  );
  let completed = 0;
  let cases;
  try {
    cases = await mapWithConcurrency(items, options.jobs, async (item, index, workerIndex) => {
      const activePage = pages[workerIndex];
      const referencePath = path.resolve(galleryDir, item.reference);
      const candidatePath = path.resolve(galleryDir, item.chemsema);
      const hashes = await artifactHashes(galleryDir, item);
      const baselineCase = baselineCases.get(item.relativeCdxml);
      if (baselineCase && artifactHashesEqual(baselineCase.artifactHashes, hashes)) {
        completed += 1;
        if (completed % 100 === 0 || completed === items.length) {
          console.log(`[CACHE ${completed}/${items.length}] reused unchanged visual-gate results`);
        }
        return {
          ...baselineCase,
          id: item.id,
          relativeCdxml: item.relativeCdxml,
          artifactHashes: hashes,
          cacheStatus: "reused",
        };
      }
      try {
        if (await oracleIsUnavailable(referencePath)) {
          completed += 1;
          console.log(`[${completed}/${items.length}] worker=${workerIndex + 1} UNAVAILABLE ${item.relativeCdxml}`);
          return {
            id: item.id,
            relativeCdxml: item.relativeCdxml,
            status: "unavailable",
            reason: "ChemDraw oracle is unavailable",
            artifactHashes: hashes,
            cacheStatus: "analyzed",
          };
        }
        const [referenceDataUrl, candidateDataUrl] = await Promise.all([
          fileDataUrl(referencePath),
          fileDataUrl(candidatePath),
        ]);
        const alignment = item.alignment?.algorithm === ALIGNMENT_ALGORITHM
          ? item.alignment
          : await computeImageAlignment(activePage, referenceDataUrl, candidateDataUrl);
        const coarseMetrics = await analyzeAlignedImages(
          activePage,
          referenceDataUrl,
          candidateDataUrl,
          alignment,
          options,
        );
        const coarseTopologyCandidate = fineTopologyCandidate(coarseMetrics, options);
        const boundedLocalEquivalent = boundedLocalTopologyEquivalent(coarseMetrics, options);
        const nearExactEquivalent = nearExactFixedDefectEquivalent(coarseMetrics, options);
        const detailMetrics = coarseMetrics.passed
          || coarseTopologyCandidate
          || boundedLocalEquivalent
          || nearExactEquivalent
          ? await analyzeAlignedImages(
            activePage,
            referenceDataUrl,
            candidateDataUrl,
            alignment,
            detailAnalysisOptions(options),
          )
          : null;
        const detailReasons = detailMetrics ? detailGateReasons(detailMetrics, options) : [];
        if (detailMetrics && strongPixelEquivalent(coarseMetrics, detailMetrics, options)) {
          const componentReason = detailReasons.indexOf("detail-component-count");
          if (componentReason >= 0) detailReasons.splice(componentReason, 1);
        }
        if (boundedLocalEquivalent) {
          const componentReason = detailReasons.indexOf("detail-component-count");
          if (componentReason >= 0) detailReasons.splice(componentReason, 1);
        }
        if (nearExactEquivalent) {
          const componentReason = detailReasons.indexOf("detail-component-count");
          if (componentReason >= 0) detailReasons.splice(componentReason, 1);
        }
        const topologyEquivalent = detailMetrics
          ? fineTopologyEquivalent(detailMetrics, options)
          : false;
        const slenderEquivalent = slenderDefectEquivalent(coarseMetrics, options);
        const coarseAccepted = coarseMetrics.passed
          || topologyEquivalent
          || slenderEquivalent
          || boundedLocalEquivalent
          || nearExactEquivalent;
        const metrics = {
          ...coarseMetrics,
          passed: coarseAccepted && detailReasons.length === 0,
          reasons: [
            ...(coarseAccepted ? [] : coarseMetrics.reasons),
            ...detailReasons,
          ],
          coarsePassed: coarseMetrics.passed,
          coarseAcceptedByFineTopology: !coarseMetrics.passed && topologyEquivalent,
          coarseAcceptedBySlenderDefect: !coarseMetrics.passed && slenderEquivalent,
          coarseAcceptedByBoundedLocalTopology:
            !coarseMetrics.passed && boundedLocalEquivalent,
          coarseAcceptedByNearExactFixedDefect:
            !coarseMetrics.passed && nearExactEquivalent,
          detail: detailMetrics ? {
            local: detailMetrics.local,
            largestMissing: detailMetrics.largestMissing,
            largestExtra: detailMetrics.largestExtra,
            topDefects: detailMetrics.topDefects,
            detailFeatures: detailMetrics.detailFeatures,
            settings: detailMetrics.settings,
          } : null,
        };
        completed += 1;
        console.log(`[${completed}/${items.length}] worker=${workerIndex + 1} ${metrics.passed ? "PASS" : "FAIL"} ${item.relativeCdxml}`);
        return {
          id: item.id,
          relativeCdxml: item.relativeCdxml,
          status: metrics.passed ? "pass" : "fail",
          alignment,
          artifactHashes: hashes,
          cacheStatus: "analyzed",
          ...metrics,
        };
      } catch (error) {
        completed += 1;
        console.log(`[${completed}/${items.length}] worker=${workerIndex + 1} ERROR ${item.relativeCdxml}`);
        return {
          id: item.id,
          relativeCdxml: item.relativeCdxml,
          status: "error",
          error: error instanceof Error ? error.stack ?? error.message : String(error),
          artifactHashes: hashes,
          cacheStatus: "analyzed",
        };
      }
    });
  } finally {
    await browser.close();
  }

  const passed = cases.filter((entry) => entry.status === "pass").length;
  const failed = cases.filter((entry) => entry.status === "fail").length;
  const errors = cases.filter((entry) => entry.status === "error").length;
  const unavailable = cases.filter((entry) => entry.status === "unavailable").length;
  const comparable = passed + failed;
  const reused = cases.filter((entry) => entry.cacheStatus === "reused").length;
  const analyzed = cases.length - reused;
  const delta = classifyBaselineChanges(cases, baselineCases);
  const report = {
    schema: "chemsema-public-cdxml-visual-gate-v1",
    cacheIdentity: CACHE_IDENTITY,
    generatedAt: new Date().toISOString(),
    gallery: galleryDir,
    policy: gatePolicy(options),
    summary: {
      total: cases.length,
      passed,
      failed,
      errors,
      unavailable,
      comparable,
      passRate: comparable ? passed / comparable : 0,
    },
    cache: {
      baselineReport: options.baselineReport ? path.resolve(options.baselineReport) : null,
      reused,
      analyzed,
    },
    delta,
    cases,
  };
  const outputPath = path.resolve(options.out);
  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  await fs.writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
  const passedGallery = await writePassedGallery(
    manifest,
    report,
    galleryDir,
    options.passedGallery,
  );
  console.log(JSON.stringify({
    outputPath,
    ...passedGallery,
    ...report.summary,
    cache: report.cache,
    improvements: delta.improvements.length,
    regressions: delta.regressions.length,
  }));
  const baselineMode = baselineCases.size > 0;
  if (!options.reportOnly && (errors || (baselineMode ? delta.regressions.length : failed))) {
    process.exitCode = 1;
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack ?? error.message : String(error));
    process.exit(1);
  });
}
