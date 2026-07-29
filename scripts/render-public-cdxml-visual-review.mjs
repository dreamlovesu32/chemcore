import { execFile } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { promisify } from "node:util";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { launchBrowser } from "./playwright-browser.mjs";
import { matchesPublicCdxmlCasePattern } from "./public-cdxml-case-filter.mjs";
import {
  collectGalleryProvenance,
  sha256File,
} from "./public-cdxml-provenance.mjs";

const execFileAsync = promisify(execFile);
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const IMAGE_ALIGNMENT_ALGORITHM = "chemdraw-declared-scale-global-translation-v9";

export function publicCdxmlCliEnvironment(baseEnvironment = process.env) {
  return {
    ...baseEnvironment,
    CHEMSEMA_CLI_DISABLE_CACHE: "1",
  };
}

function parseArgs(argv) {
  const args = {
    root: "tmp/public-corpus-pilot",
    outDir: "tmp/public-cdxml-visual-review",
    report: "tmp/public-cdxml-roundtrip-label-audit/report.json",
    all: false,
    incremental: false,
    allowDirty: false,
    oracleGallery: null,
    jobs: 4,
    patterns: [],
    cli: process.platform === "win32"
      ? "target/debug/chemsema-cli.exe"
      : "target/debug/chemsema-cli",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--root") args.root = argv[++index];
    else if (arg === "--out") args.outDir = argv[++index];
    else if (arg === "--report") args.report = argv[++index];
    else if (arg === "--all") args.all = true;
    else if (arg === "--incremental") args.incremental = true;
    else if (arg === "--allow-dirty") args.allowDirty = true;
    else if (arg === "--oracle-gallery") args.oracleGallery = argv[++index];
    else if (arg === "--jobs") args.jobs = Number(argv[++index]);
    else if (arg === "--only") args.patterns.push(argv[++index]);
    else if (arg === "--cli") args.cli = argv[++index];
    else if (arg === "--help" || arg === "-h") args.help = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return args;
}

export async function mapWithConcurrency(items, jobs, mapper) {
  if (!Number.isInteger(jobs) || jobs < 1) {
    throw new Error("--jobs must be a positive integer");
  }
  const results = Array(items.length);
  let nextIndex = 0;
  await Promise.all(Array.from(
    { length: Math.min(jobs, items.length) },
    async (_, workerIndex) => {
      while (nextIndex < items.length) {
        const index = nextIndex;
        nextIndex += 1;
        results[index] = await mapper(items[index], index, workerIndex);
      }
    },
  ));
  return results;
}

async function walk(directory) {
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await walk(absolute));
    else files.push(absolute);
  }
  return files;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function safeName(value) {
  return value
    .replace(/\.cdxml$/i, "")
    .replace(/[^a-z0-9._-]+/gi, "_")
    .replace(/^_+|_+$/g, "");
}

function dataUrl(buffer, mimeType) {
  return `data:${mimeType};base64,${buffer.toString("base64")}`;
}

export async function computeImageAlignment(page, referenceDataUrl, chemsemaDataUrl) {
  return page.evaluate(async ({ referenceDataUrl, chemsemaDataUrl }) => {
    async function loadImage(src) {
      const image = new Image();
      image.decoding = "sync";
      image.src = src;
      await image.decode();
      return image;
    }

    function numericLength(value) {
      const match = /^\s*([-+0-9.eE]+)(?:px)?\s*$/.exec(value ?? "");
      if (!match) return null;
      const number = Number(match[1]);
      return Number.isFinite(number) && number > 0 ? number : null;
    }

    function viewBoxOf(svg, width, height) {
      const values = (svg.getAttribute("viewBox") ?? "")
        .trim()
        .split(/[\s,]+/)
        .map(Number);
      if (values.length === 4 && values.every(Number.isFinite) && values[2] > 0 && values[3] > 0) {
        return { x: values[0], y: values[1], width: values[2], height: values[3] };
      }
      return { x: 0, y: 0, width, height };
    }

    async function imageFrame(src, image) {
      const fallback = {
        width: image.naturalWidth,
        height: image.naturalHeight,
        viewBox: {
          x: 0,
          y: 0,
          width: image.naturalWidth,
          height: image.naturalHeight,
        },
        svg: null,
      };
      if (!src.startsWith("data:image/svg+xml")) return fallback;
      const source = await (await fetch(src)).text();
      const document = new DOMParser().parseFromString(source, "image/svg+xml");
      const svg = document.documentElement;
      if (svg.localName !== "svg" || document.querySelector("parsererror")) return fallback;
      const preliminaryViewBox = viewBoxOf(svg, image.naturalWidth, image.naturalHeight);
      const width = numericLength(svg.getAttribute("width")) ?? preliminaryViewBox.width;
      const height = numericLength(svg.getAttribute("height")) ?? preliminaryViewBox.height;
      if (!(width > 0 && height > 0)) return fallback;
      return {
        width,
        height,
        viewBox: viewBoxOf(svg, width, height),
        svg,
      };
    }

    function matrixValues(value) {
      const number = "([-+0-9.eE]+)";
      const separator = "[\\s,]+";
      const match = new RegExp(
        `^matrix\\(\\s*${number}${separator}${number}${separator}${number}${separator}`
          + `${number}${separator}${number}${separator}${number}\\s*\\)$`,
      ).exec(value ?? "");
      if (!match) return null;
      const values = match.slice(1).map(Number);
      return values.every(Number.isFinite) ? values : null;
    }

    function chemDrawUniformScale(frame) {
      if (!frame.svg) return null;
      const counts = new Map();
      for (const element of frame.svg.querySelectorAll("[transform]")) {
        const values = matrixValues(element.getAttribute("transform"));
        if (!values) continue;
        const [a, b, c, d] = values;
        if (
          a <= 0 || d <= 0
          || Math.abs(b) > 1e-9 || Math.abs(c) > 1e-9
          || Math.abs(a - d) > 1e-6
        ) continue;
        const normalized = Number(a.toFixed(9));
        counts.set(normalized, (counts.get(normalized) ?? 0) + 1);
      }
      const entries = [...counts.entries()].sort((left, right) => right[1] - left[1]);
      return entries.length === 0
        ? null
        : { scale: entries[0][0], count: entries[0][1], scaleCount: entries.length };
    }

    function uniformViewportScale(frame) {
      const x = frame.width / frame.viewBox.width;
      const y = frame.height / frame.viewBox.height;
      const tolerance = Math.max(1e-9, Math.max(Math.abs(x), Math.abs(y)) * 1e-6);
      return Math.abs(x - y) <= tolerance && x > 0 ? (x + y) / 2 : null;
    }

    function contentFrameAlignment(
      referenceFrame,
      chemsemaFrame,
      referenceGeometry,
      chemsemaGeometry,
    ) {
      const chemDraw = chemDrawUniformScale(referenceFrame);
      const referenceViewportScale = uniformViewportScale(referenceFrame);
      const chemsemaViewportScale = uniformViewportScale(chemsemaFrame);
      if (!chemDraw || !referenceViewportScale || !chemsemaViewportScale) return null;
      const worldScale = chemDraw.scale * 20 * referenceViewportScale;
      const scale = worldScale / chemsemaViewportScale;
      const referenceCenter = {
        x: referenceGeometry.bbox.left + referenceGeometry.bbox.width / 2,
        y: referenceGeometry.bbox.top + referenceGeometry.bbox.height / 2,
      };
      const chemsemaCenter = {
        x: chemsemaGeometry.bbox.left + chemsemaGeometry.bbox.width / 2,
        y: chemsemaGeometry.bbox.top + chemsemaGeometry.bbox.height / 2,
      };
      return {
        scale,
        dx: referenceCenter.x - chemsemaCenter.x * scale,
        dy: referenceCenter.y - chemsemaCenter.y * scale,
        chemDrawTransformCount: chemDraw.count,
        chemDrawScaleCount: chemDraw.scaleCount,
        chemDrawMatrixScale: chemDraw.scale,
        referenceContentBounds: referenceGeometry.bbox,
        chemsemaContentBounds: chemsemaGeometry.bbox,
      };
    }

    function imageInkGeometry(image, frame, maxDimension) {
      const scale = maxDimension / Math.max(frame.width, frame.height, 1);
      const width = Math.max(1, Math.ceil(frame.width * scale));
      const height = Math.max(1, Math.ceil(frame.height * scale));
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      context.fillStyle = "#ffffff";
      context.fillRect(0, 0, width, height);
      context.drawImage(image, 0, 0, width, height);
      const pixels = context.getImageData(0, 0, width, height).data;
      let left = width;
      let top = height;
      let right = -1;
      let bottom = -1;
      let sumX = 0;
      let sumY = 0;
      let count = 0;
      for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
          const offset = (y * width + x) * 4;
          if (pixels[offset] + pixels[offset + 1] + pixels[offset + 2] >= 740) continue;
          left = Math.min(left, x);
          top = Math.min(top, y);
          right = Math.max(right, x);
          bottom = Math.max(bottom, y);
          sumX += x;
          sumY += y;
          count += 1;
        }
      }
      if (count === 0) {
        return {
          bbox: { left: 0, top: 0, width: frame.width, height: frame.height },
          centroid: { x: frame.width / 2, y: frame.height / 2 },
        };
      }
      return {
        bbox: {
          left: left / scale,
          top: top / scale,
          width: (right - left + 1) / scale,
          height: (bottom - top + 1) / scale,
        },
        centroid: { x: sumX / count / scale, y: sumY / count / scale },
      };
    }

    function maskForReference(image, frame, analysisScale, padding) {
      const width = Math.max(1, Math.ceil(frame.width * analysisScale) + padding * 2);
      const height = Math.max(1, Math.ceil(frame.height * analysisScale) + padding * 2);
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      context.fillStyle = "#ffffff";
      context.fillRect(0, 0, width, height);
      context.drawImage(
        image,
        padding,
        padding,
        frame.width * analysisScale,
        frame.height * analysisScale,
      );
      const pixels = context.getImageData(0, 0, width, height).data;
      const mask = new Uint8Array(width * height);
      let count = 0;
      for (let index = 0; index < mask.length; index += 1) {
        const offset = index * 4;
        if (pixels[offset] + pixels[offset + 1] + pixels[offset + 2] < 740) {
          mask[index] = 1;
          count += 1;
        }
      }
      return { width, height, mask, count };
    }

    function candidateInkPoints(image, frame, reference, analysisScale, scale, dx, dy, padding) {
      const canvas = document.createElement("canvas");
      canvas.width = reference.width;
      canvas.height = reference.height;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      context.fillStyle = "#ffffff";
      context.fillRect(0, 0, canvas.width, canvas.height);
      context.drawImage(
        image,
        padding + dx * analysisScale,
        padding + dy * analysisScale,
        frame.width * scale * analysisScale,
        frame.height * scale * analysisScale,
      );
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
      const points = [];
      for (let y = 0; y < canvas.height; y += 1) {
        for (let x = 0; x < canvas.width; x += 1) {
          const offset = (y * canvas.width + x) * 4;
          if (pixels[offset] + pixels[offset + 1] + pixels[offset + 2] < 740) {
            points.push(y * canvas.width + x);
          }
        }
      }
      return points;
    }

    function bestTranslation(reference, points, radius) {
      let best = { dx: 0, dy: 0, iou: -1, overlap: 0 };
      for (let dy = -radius; dy <= radius; dy += 1) {
        for (let dx = -radius; dx <= radius; dx += 1) {
          let overlap = 0;
          for (const index of points) {
            const y = Math.floor(index / reference.width) + dy;
            const x = index % reference.width + dx;
            if (
              x >= 0 && x < reference.width && y >= 0 && y < reference.height
              && reference.mask[y * reference.width + x]
            ) {
              overlap += 1;
            }
          }
          const union = reference.count + points.length - overlap;
          const iou = union === 0 ? 1 : overlap / union;
          if (
            iou > best.iou
            || (iou === best.iou && Math.abs(dx) + Math.abs(dy) < Math.abs(best.dx) + Math.abs(best.dy))
          ) {
            best = { dx, dy, iou, overlap };
          }
        }
      }
      return best;
    }

    const [referenceImage, chemsemaImage] = await Promise.all([
      loadImage(referenceDataUrl),
      loadImage(chemsemaDataUrl),
    ]);
    const [referenceFrame, chemsemaFrame] = await Promise.all([
      imageFrame(referenceDataUrl, referenceImage),
      imageFrame(chemsemaDataUrl, chemsemaImage),
    ]);
    const [referenceGeometry, chemsemaGeometry] = [
      imageInkGeometry(referenceImage, referenceFrame, 220),
      imageInkGeometry(chemsemaImage, chemsemaFrame, 220),
    ];
    const widthScale = referenceGeometry.bbox.width / Math.max(chemsemaGeometry.bbox.width, 1e-6);
    const heightScale = referenceGeometry.bbox.height / Math.max(chemsemaGeometry.bbox.height, 1e-6);
    const initialScale = Math.sqrt(widthScale * heightScale);

    async function search(
      maxDimension,
      centerScale,
      scaleStep,
      scaleRadius,
      shiftRadius,
      centerAlignment = null,
    ) {
      const analysisScale = maxDimension / Math.max(referenceFrame.width, referenceFrame.height, 1);
      const padding = shiftRadius + 10;
      const reference = maskForReference(referenceImage, referenceFrame, analysisScale, padding);
      let best = null;
      for (let scaleIndex = -scaleRadius; scaleIndex <= scaleRadius; scaleIndex += 1) {
        const scale = centerScale * (1 + scaleIndex * scaleStep);
        const baseDx = centerAlignment
          ? centerAlignment.dx + chemsemaGeometry.centroid.x * (centerAlignment.scale - scale)
          : referenceGeometry.centroid.x - chemsemaGeometry.centroid.x * scale;
        const baseDy = centerAlignment
          ? centerAlignment.dy + chemsemaGeometry.centroid.y * (centerAlignment.scale - scale)
          : referenceGeometry.centroid.y - chemsemaGeometry.centroid.y * scale;
        const points = candidateInkPoints(
          chemsemaImage,
          chemsemaFrame,
          reference,
          analysisScale,
          scale,
          baseDx,
          baseDy,
          padding,
        );
        const shift = bestTranslation(reference, points, shiftRadius);
        const candidate = {
          scale,
          dx: baseDx + shift.dx / analysisScale,
          dy: baseDy + shift.dy / analysisScale,
          iou: shift.iou,
        };
        if (!best || candidate.iou > best.iou) best = candidate;
      }
      return best;
    }

    const contentFrameGeometry = [
      imageInkGeometry(referenceImage, referenceFrame, 1600),
      imageInkGeometry(chemsemaImage, chemsemaFrame, 1600),
    ];
    let declared = contentFrameAlignment(
      referenceFrame,
      chemsemaFrame,
      contentFrameGeometry[0],
      contentFrameGeometry[1],
    );
    if (declared) {
      // Bounding-box centers are only a starting point: labels can legitimately
      // make the two ink extents asymmetric. Search a broad low-resolution
      // translation field first, then refine without changing ChemDraw's
      // declared scale. This finds the global overlap instead of locking onto
      // a text-heavy local optimum near the initial center.
      const coarseTranslation = await search(
        180,
        declared.scale,
        0,
        0,
        40,
        declared,
      );
      const refinedTranslation = await search(
        720,
        declared.scale,
        0,
        0,
        16,
        coarseTranslation,
      );
      const preciseTranslation = await search(
        1440,
        declared.scale,
        0,
        0,
        6,
        refinedTranslation,
      );
      declared = {
        ...declared,
        dx: preciseTranslation.dx,
        dy: preciseTranslation.dy,
        registrationIou: preciseTranslation.iou,
      };
      const analysisScale = 720 / Math.max(referenceFrame.width, referenceFrame.height, 1);
      const reference = maskForReference(referenceImage, referenceFrame, analysisScale, 10);
      const points = candidateInkPoints(
        chemsemaImage,
        chemsemaFrame,
        reference,
        analysisScale,
        declared.scale,
        declared.dx,
        declared.dy,
        10,
      );
      const score = bestTranslation(reference, points, 0);
      return {
        algorithm: "chemdraw-declared-scale-global-translation-v9",
        basis: "declared-scale-global-translation",
        scale: declared.scale,
        dx: declared.dx,
        dy: declared.dy,
        iou: score.iou,
        referenceWidth: referenceFrame.width,
        referenceHeight: referenceFrame.height,
        chemsemaWidth: chemsemaFrame.width,
        chemsemaHeight: chemsemaFrame.height,
        vectorFrame: {
          chemDrawTransformCount: declared.chemDrawTransformCount,
          chemDrawScaleCount: declared.chemDrawScaleCount,
          chemDrawMatrixScale: declared.chemDrawMatrixScale,
          referenceContentBounds: declared.referenceContentBounds,
          chemsemaContentBounds: declared.chemsemaContentBounds,
          registrationIou: declared.registrationIou,
          referenceViewBox: referenceFrame.viewBox,
          chemsemaViewBox: chemsemaFrame.viewBox,
        },
      };
    }

    const coarse = await search(180, initialScale, 0.005, 4, 20);
    let refined = await search(360, coarse.scale, 0.00125, 2, 6, coarse);
    // The 360 px pass can settle on a text-heavy local optimum whose scale is
    // slightly wrong for long bonds and arrows. Reopen a wider scale interval
    // at medium resolution, then finish with a narrow high-resolution pass.
    const stabilized = Math.max(referenceFrame.width, referenceFrame.height) >= 400
      ? await search(720, refined.scale, 0.0003125, 12, 6, refined)
      : refined;
    const precise = Math.max(referenceFrame.width, referenceFrame.height) >= 400
      ? await search(1440, stabilized.scale, 0.00015625, 3, 5, stabilized)
      : stabilized;
    return {
      algorithm: "chemdraw-declared-scale-global-translation-v9",
      basis: "ink-overlap",
      scale: precise.scale,
      dx: precise.dx,
      dy: precise.dy,
      iou: precise.iou,
      referenceWidth: referenceFrame.width,
      referenceHeight: referenceFrame.height,
      chemsemaWidth: chemsemaFrame.width,
      chemsemaHeight: chemsemaFrame.height,
    };
  }, { referenceDataUrl, chemsemaDataUrl });
}

function reviewScreenshotHtml(item, referenceDataUrl, chemsemaDataUrl, alignment) {
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<style>
  * { box-sizing: border-box; }
  html, body { margin: 0; width: 2520px; min-height: 1080px; background: #f4f5f7; color: #15171a; font-family: "Segoe UI", "Microsoft YaHei", Arial, sans-serif; }
  body { padding: 24px 30px 30px; }
  header { height: 76px; display: flex; flex-direction: column; justify-content: center; }
  h1 { margin: 0; font-size: 23px; line-height: 32px; }
  .source { color: #60656d; font-size: 14px; line-height: 22px; }
  main { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; }
  section { min-width: 0; }
  h2 { margin: 0 0 8px; font-size: 18px; line-height: 26px; }
  .panel { position: relative; width: 1218px; height: 920px; overflow: hidden; background: #fff; border: 1px solid #cfd3d8; border-radius: 6px; }
  img { position: absolute; display: block; max-width: none; max-height: none; }
</style>
</head>
<body>
  <header>
    <h1>${escapeHtml(item.number)} / ${escapeHtml(item.total)} · ${escapeHtml(item.title)}</h1>
    <div class="source">${escapeHtml(item.relativeCdxml)}</div>
  </header>
  <main>
    <section><h2>${escapeHtml(item.referenceLabel)}</h2><div class="panel" id="referencePanel"><img id="referenceImage" src="${referenceDataUrl}"></div></section>
    <section><h2>${escapeHtml(item.chemsemaLabel)}</h2><div class="panel" id="chemsemaPanel"><img id="chemsemaImage" src="${chemsemaDataUrl}"></div></section>
  </main>
<script>
(() => {
  const alignment = ${JSON.stringify(alignment)};
  const referenceImage = document.getElementById("referenceImage");
  const chemsemaImage = document.getElementById("chemsemaImage");
  function place(panel, image, width, height, scale, dx, dy) {
    const padding = 16;
    const fit = Math.min(
      (panel.clientWidth - padding * 2) / alignment.referenceWidth,
      (panel.clientHeight - padding * 2) / alignment.referenceHeight,
    );
    const originX = (panel.clientWidth - alignment.referenceWidth * fit) / 2;
    const originY = (panel.clientHeight - alignment.referenceHeight * fit) / 2;
    image.style.width = width * scale * fit + "px";
    image.style.height = height * scale * fit + "px";
    image.style.left = originX + dx * fit + "px";
    image.style.top = originY + dy * fit + "px";
  }
  Promise.allSettled([referenceImage.decode(), chemsemaImage.decode()]).then(() => {
    place(
      document.getElementById("referencePanel"),
      referenceImage,
      alignment.referenceWidth,
      alignment.referenceHeight,
      1,
      0,
      0,
    );
    place(
      document.getElementById("chemsemaPanel"),
      chemsemaImage,
      alignment.chemsemaWidth,
      alignment.chemsemaHeight,
      alignment.scale,
      alignment.dx,
      alignment.dy,
    );
    document.body.dataset.ready = "yes";
  });
})();
</script>
</body>
</html>`;
}

export function viewerHtml(items) {
  const data = JSON.stringify(items).replaceAll("<", "\\u003c");
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>公共 CDXML 肉眼审查</title>
<style>
  :root { color-scheme: light; font-family: "Segoe UI", "Microsoft YaHei", Arial, sans-serif; }
  * { box-sizing: border-box; }
  html, body { margin: 0; height: 100%; color: #17191c; background: #f2f3f5; }
  body { display: grid; grid-template-columns: 310px minmax(0, 1fr); }
  aside { overflow: auto; border-right: 1px solid #ccd0d5; background: #fff; padding: 16px 12px; }
  aside h1 { margin: 0 6px 4px; font-size: 18px; }
  .summary { margin: 0 6px 14px; color: #676c74; font-size: 13px; }
  .item-button { width: 100%; margin: 0 0 5px; padding: 8px 10px; border: 1px solid transparent; border-radius: 5px; background: transparent; text-align: left; cursor: pointer; font-size: 13px; }
  .item-button:hover { background: #f1f4f8; }
  .item-button.active { border-color: #5591d4; background: #eaf3ff; }
  .item-button[data-state="pass"]::before { content: "✓ "; color: #198754; font-weight: 700; }
  .item-button[data-state="issue"]::before { content: "! "; color: #c33; font-weight: 700; }
  main { min-width: 0; display: grid; grid-template-rows: auto minmax(0, 1fr) auto; gap: 12px; padding: 16px 18px; }
  header { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 12px; }
  button, textarea, input { font: inherit; }
  .nav { padding: 7px 12px; border: 1px solid #bfc4ca; border-radius: 5px; background: #fff; cursor: pointer; }
  .title { min-width: 0; }
  .title strong, .title span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .title span { margin-top: 2px; color: #676c74; font-size: 12px; }
  .mode { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
  .mode button { padding: 7px 10px; border: 1px solid #bfc4ca; border-radius: 5px; background: #fff; cursor: pointer; }
  .mode button.active { color: #fff; border-color: #2869b3; background: #2869b3; }
  .stage { min-height: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .stage.overlay { display: block; position: relative; }
  figure { min-width: 0; min-height: 0; margin: 0; display: grid; grid-template-rows: auto minmax(0, 1fr); }
  figcaption { padding: 0 0 7px; font-weight: 600; }
  .canvas { position: relative; min-height: 0; overflow: hidden; border: 1px solid #cbd0d6; border-radius: 6px; background: #fff; }
  .canvas img { position: absolute; display: block; max-width: none; max-height: none; user-select: none; -webkit-user-drag: none; }
  .box-layer { position: absolute; inset: 0; pointer-events: none; }
  .box-layer.drawing-enabled { pointer-events: auto; cursor: crosshair; touch-action: none; }
  .diff-box { position: absolute; border: 2px solid #e53935; background: #e5393514; box-shadow: 0 0 0 1px #fff8 inset; pointer-events: none; }
  .diff-box.draft { border-style: dashed; background: #ff980018; }
  .stage.overlay figure { position: absolute; inset: 0; grid-template-rows: 0 minmax(0, 1fr); }
  .stage.overlay figure figcaption { visibility: hidden; }
  .stage.overlay figure:last-child { pointer-events: none; }
  .stage.overlay .canvas { border: 0; background: transparent; overflow: hidden; }
  .stage.overlay figure:first-child .canvas { border: 1px solid #cbd0d6; background: #fff; }
  .stage.overlay figure:last-child img { opacity: var(--overlay-opacity, .5); }
  .stage.overlay figure:last-child .box-layer { display: none; }
  .review { display: grid; grid-template-columns: auto auto minmax(240px, 1fr) auto; gap: 10px; align-items: center; }
  .review button { padding: 8px 13px; border: 1px solid #bfc4ca; border-radius: 5px; background: #fff; cursor: pointer; }
  .review button.pass.active { color: #fff; border-color: #198754; background: #198754; }
  .review button.issue.active { color: #fff; border-color: #c33; background: #c33; }
  textarea { width: 100%; height: 42px; resize: vertical; border: 1px solid #bfc4ca; border-radius: 5px; padding: 8px; }
  .overlay-controls { display: none; align-items: center; gap: 8px; font-size: 13px; }
  .overlay-controls.visible { display: flex; }
  .save-status { min-width: 120px; color: #5f6873; font-size: 12px; text-align: right; }
  @media (max-width: 1000px) { body { grid-template-columns: 230px minmax(0, 1fr); } .stage { grid-template-columns: 1fr; overflow: auto; } figure { min-height: 55vh; } }
</style>
</head>
<body>
<aside>
  <h1>公共 CDXML 肉眼审查</h1>
  <p class="summary"><span id="progress"></span> · 共 ${items.length} 组</p>
  <div id="list"></div>
</aside>
<main>
  <header>
    <button class="nav" id="previous">← 上一张</button>
    <div class="title"><strong id="title"></strong><span id="source"></span></div>
    <div class="mode">
      <button id="sideMode" class="active">左右对照</button>
      <button id="overlayMode">透明叠加</button>
      <span class="overlay-controls" id="overlayControls">参考图<input id="opacity" type="range" min="0" max="100" value="50">ChemSema</span>
      <button id="boxMode" title="在任一侧拖动框选差异；另一侧会同步显示">框选差异</button>
      <button id="undoBox" title="撤销当前图片的最后一个框">撤销框</button>
      <button id="clearBoxes" title="清空当前图片的所有框">清空框</button>
      <button id="exportReview" title="导出备注、判定和框选坐标">导出审查结果</button>
      <button class="nav" id="next">下一张 →</button>
    </div>
  </header>
  <div class="stage" id="stage">
    <figure><figcaption id="referenceLabel"></figcaption><div class="canvas" id="referenceCanvas"><img id="reference"><div class="box-layer" id="referenceBoxes"></div></div></figure>
    <figure><figcaption id="chemsemaLabel"></figcaption><div class="canvas" id="chemsemaCanvas"><img id="chemsema"><div class="box-layer" id="chemsemaBoxes"></div></div></figure>
  </div>
  <div class="review">
    <button class="pass" id="pass">✓ 通过</button>
    <button class="issue" id="issue">! 有问题</button>
    <textarea id="notes" placeholder="可选：记录标签、位置、分子或其他问题；内容会保存在当前浏览器。"></textarea>
    <span class="save-status" id="saveStatus">自动保存已开启</span>
  </div>
</main>
<script>
  const items = ${data};
  const storageKey = "chemsema-public-cdxml-visual-review-v2";
  const legacyStorageKey = "chemsema-public-cdxml-visual-review-v1";
  function loadSavedState() {
    try {
      return JSON.parse(
        localStorage.getItem(storageKey) || localStorage.getItem(legacyStorageKey) || "{}",
      );
    } catch {
      return {};
    }
  }
  const saved = loadSavedState();
  saved.__ui = saved.__ui || {};
  const hashIndex = Number(location.hash.slice(1));
  let index = Math.max(
    0,
    Math.min(items.length - 1, Number.isFinite(hashIndex) && hashIndex > 0
      ? hashIndex - 1
      : Number(saved.__ui.index || 0)),
  );
  let mode = saved.__ui.mode === "overlay" ? "overlay" : "side";
  let boxModeEnabled = Boolean(saved.__ui.boxMode);
  let drawing = null;
  let layoutGeneration = 0;
  const list = document.getElementById("list");
  const referenceImage = document.getElementById("reference");
  const chemsemaImage = document.getElementById("chemsema");
  const referenceCanvas = document.getElementById("referenceCanvas");
  const chemsemaCanvas = document.getElementById("chemsemaCanvas");
  const referenceBoxes = document.getElementById("referenceBoxes");
  const chemsemaBoxes = document.getElementById("chemsemaBoxes");
  const buttons = items.map((item, itemIndex) => {
    const button = document.createElement("button");
    button.className = "item-button";
    button.textContent = item.label;
    button.onclick = () => show(itemIndex);
    list.append(button);
    return button;
  });
  function persist() {
    saved.__ui = {
      ...(saved.__ui || {}),
      index,
      mode,
      boxMode: boxModeEnabled,
      opacity: Number(document.getElementById("opacity").value),
    };
    try {
      localStorage.setItem(storageKey, JSON.stringify(saved));
      const now = new Date();
      document.getElementById("saveStatus").textContent = "已自动保存 " + now.toLocaleTimeString();
    } catch {
      document.getElementById("saveStatus").textContent = "保存失败：浏览器已禁用本地存储";
    }
  }
  function currentState() {
    const id = items[index].id;
    saved[id] = saved[id] || {};
    saved[id].boxes = Array.isArray(saved[id].boxes) ? saved[id].boxes : [];
    return saved[id];
  }
  function updateProgress() {
    const reviewed = items.filter((item) => saved[item.id]?.state).length;
    document.getElementById("progress").textContent = "已审 " + reviewed;
  }
  function updateItemControls() {
    const state = currentState();
    document.getElementById("pass").classList.toggle("active", state.state === "pass");
    document.getElementById("issue").classList.toggle("active", state.state === "issue");
    document.getElementById("undoBox").disabled = state.boxes.length === 0;
    document.getElementById("clearBoxes").disabled = state.boxes.length === 0;
    buttons.forEach((button, buttonIndex) => {
      button.classList.toggle("active", buttonIndex === index);
      button.dataset.state = saved[items[buttonIndex].id]?.state || "";
    });
    updateProgress();
  }
  function canonicalAlignment(item) {
    const alignment = item.alignment || {};
    const referenceWidth = Number(alignment.referenceWidth || referenceImage.naturalWidth || 1);
    const referenceHeight = Number(alignment.referenceHeight || referenceImage.naturalHeight || 1);
    const chemsemaWidth = Number(alignment.chemsemaWidth || chemsemaImage.naturalWidth || 1);
    const chemsemaHeight = Number(alignment.chemsemaHeight || chemsemaImage.naturalHeight || 1);
    const scale = Number(alignment.scale || Math.min(
      referenceWidth / Math.max(chemsemaWidth, 1),
      referenceHeight / Math.max(chemsemaHeight, 1),
    ));
    return {
      referenceWidth,
      referenceHeight,
      chemsemaWidth,
      chemsemaHeight,
      scale,
      dx: Number(
        alignment.dx
        ?? (referenceWidth - chemsemaWidth * scale) / 2
      ),
      dy: Number(
        alignment.dy
        ?? (referenceHeight - chemsemaHeight * scale) / 2
      ),
    };
  }
  function placeImage(canvas, image, alignment, width, height, scale, dx, dy) {
    const padding = 12;
    const fit = Math.max(1e-6, Math.min(
      Math.max(1, canvas.clientWidth - padding * 2) / alignment.referenceWidth,
      Math.max(1, canvas.clientHeight - padding * 2) / alignment.referenceHeight,
    ));
    const originX = (canvas.clientWidth - alignment.referenceWidth * fit) / 2;
    const originY = (canvas.clientHeight - alignment.referenceHeight * fit) / 2;
    image.style.width = width * scale * fit + "px";
    image.style.height = height * scale * fit + "px";
    image.style.left = originX + dx * fit + "px";
    image.style.top = originY + dy * fit + "px";
    canvas.reviewLayout = { fit, originX, originY, ...alignment };
  }
  async function layoutImages(item) {
    const generation = ++layoutGeneration;
    await Promise.all([
      referenceImage.decode().catch(() => undefined),
      chemsemaImage.decode().catch(() => undefined),
    ]);
    if (generation !== layoutGeneration || item !== items[index]) return;
    const alignment = canonicalAlignment(item);
    placeImage(
      referenceCanvas,
      referenceImage,
      alignment,
      alignment.referenceWidth,
      alignment.referenceHeight,
      1,
      0,
      0,
    );
    placeImage(
      chemsemaCanvas,
      chemsemaImage,
      alignment,
      alignment.chemsemaWidth,
      alignment.chemsemaHeight,
      alignment.scale,
      alignment.dx,
      alignment.dy,
    );
    renderBoxes();
  }
  function canonicalPoint(canvas, event) {
    const layout = canvas.reviewLayout;
    if (!layout) return null;
    const bounds = canvas.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(
        layout.referenceWidth,
        (event.clientX - bounds.left - layout.originX) / layout.fit,
      )),
      y: Math.max(0, Math.min(
        layout.referenceHeight,
        (event.clientY - bounds.top - layout.originY) / layout.fit,
      )),
    };
  }
  function normalizedBox(start, end) {
    return {
      x: Math.min(start.x, end.x),
      y: Math.min(start.y, end.y),
      width: Math.abs(end.x - start.x),
      height: Math.abs(end.y - start.y),
    };
  }
  function renderBoxLayer(layer, canvas, boxes, draft) {
    layer.replaceChildren();
    const layout = canvas.reviewLayout;
    if (!layout) return;
    for (const entry of [...boxes, ...(draft ? [draft] : [])]) {
      const element = document.createElement("div");
      element.className = "diff-box" + (entry === draft ? " draft" : "");
      element.style.left = layout.originX + entry.x * layout.fit + "px";
      element.style.top = layout.originY + entry.y * layout.fit + "px";
      element.style.width = entry.width * layout.fit + "px";
      element.style.height = entry.height * layout.fit + "px";
      layer.append(element);
    }
  }
  function renderBoxes() {
    const boxes = currentState().boxes;
    const draft = drawing ? normalizedBox(drawing.start, drawing.current) : null;
    renderBoxLayer(referenceBoxes, referenceCanvas, boxes, draft);
    renderBoxLayer(chemsemaBoxes, chemsemaCanvas, boxes, draft);
  }
  function setBoxMode(enabled) {
    boxModeEnabled = enabled;
    document.getElementById("boxMode").classList.toggle("active", enabled);
    referenceBoxes.classList.toggle("drawing-enabled", enabled);
    chemsemaBoxes.classList.toggle("drawing-enabled", enabled);
    persist();
  }
  function bindBoxDrawing(layer, canvas) {
    layer.addEventListener("pointerdown", (event) => {
      if (!boxModeEnabled || event.button !== 0) return;
      const point = canonicalPoint(canvas, event);
      if (!point) return;
      event.preventDefault();
      layer.setPointerCapture(event.pointerId);
      drawing = { pointerId: event.pointerId, start: point, current: point };
      renderBoxes();
    });
    layer.addEventListener("pointermove", (event) => {
      if (!drawing || drawing.pointerId !== event.pointerId) return;
      const point = canonicalPoint(canvas, event);
      if (!point) return;
      drawing.current = point;
      renderBoxes();
    });
    const finish = (event) => {
      if (!drawing || drawing.pointerId !== event.pointerId) return;
      const point = canonicalPoint(canvas, event);
      if (point) drawing.current = point;
      const box = normalizedBox(drawing.start, drawing.current);
      drawing = null;
      if (box.width >= 1 && box.height >= 1) {
        const state = currentState();
        state.boxes.push(box);
        state.state = "issue";
        persist();
        updateItemControls();
      }
      renderBoxes();
    };
    layer.addEventListener("pointerup", finish);
    layer.addEventListener("pointercancel", (event) => {
      if (!drawing || drawing.pointerId !== event.pointerId) return;
      drawing = null;
      renderBoxes();
    });
  }
  function show(nextIndex) {
    index = (nextIndex + items.length) % items.length;
    const item = items[index];
    location.hash = String(index + 1);
    document.getElementById("title").textContent = item.label;
    document.getElementById("source").textContent = item.relativeCdxml;
    referenceImage.src = item.reference;
    chemsemaImage.src = item.chemsema;
    document.getElementById("referenceLabel").textContent = item.referenceLabel;
    document.getElementById("chemsemaLabel").textContent = item.chemsemaLabel;
    document.getElementById("notes").value = currentState().notes || "";
    drawing = null;
    updateItemControls();
    buttons[index].scrollIntoView({ block: "nearest" });
    persist();
    layoutImages(item);
  }
  function setState(state) {
    const id = items[index].id;
    saved[id] = { ...(saved[id] || {}), state: saved[id]?.state === state ? "" : state };
    persist();
    show(index);
  }
  document.getElementById("previous").onclick = () => show(index - 1);
  document.getElementById("next").onclick = () => show(index + 1);
  document.getElementById("pass").onclick = () => setState("pass");
  document.getElementById("issue").onclick = () => setState("issue");
  document.getElementById("boxMode").onclick = () => setBoxMode(!boxModeEnabled);
  document.getElementById("undoBox").onclick = () => {
    currentState().boxes.pop();
    persist();
    updateItemControls();
    renderBoxes();
  };
  document.getElementById("clearBoxes").onclick = () => {
    if (currentState().boxes.length === 0 || !confirm("清空当前图片的所有差异框？")) return;
    currentState().boxes = [];
    persist();
    updateItemControls();
    renderBoxes();
  };
  document.getElementById("notes").oninput = (event) => {
    const id = items[index].id;
    saved[id] = { ...(saved[id] || {}), notes: event.target.value };
    persist();
  };
  document.getElementById("exportReview").onclick = () => {
    persist();
    const reviews = items.map((item) => {
      const state = saved[item.id] || {};
      const notes = String(state.notes || "");
      const boxes = Array.isArray(state.boxes) ? state.boxes : [];
      const hasAnnotation = notes.trim().length > 0 || boxes.length > 0;
      return {
        id: item.id,
        label: item.label,
        relativeCdxml: item.relativeCdxml,
        verdict: hasAnnotation || state.state === "issue" ? "issue" : "pass",
        notes,
        boxes,
      };
    });
    const payload = {
      schema: "chemsema-public-cdxml-visual-review-v1",
      exportedAt: new Date().toISOString(),
      summary: {
        total: reviews.length,
        passed: reviews.filter((entry) => entry.verdict === "pass").length,
        issues: reviews.filter((entry) => entry.verdict === "issue").length,
      },
      reviews,
    };
    const blob = new Blob([JSON.stringify(payload, null, 2) + "\\n"], { type: "application/json" });
    const href = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.download = "chemsema-visual-review.json";
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(href), 1000);
  };
  document.getElementById("sideMode").onclick = () => {
    mode = "side";
    document.getElementById("stage").classList.remove("overlay");
    document.getElementById("sideMode").classList.add("active");
    document.getElementById("overlayMode").classList.remove("active");
    document.getElementById("overlayControls").classList.remove("visible");
    persist();
    requestAnimationFrame(() => layoutImages(items[index]));
  };
  document.getElementById("overlayMode").onclick = () => {
    mode = "overlay";
    document.getElementById("stage").classList.add("overlay");
    document.getElementById("sideMode").classList.remove("active");
    document.getElementById("overlayMode").classList.add("active");
    document.getElementById("overlayControls").classList.add("visible");
    persist();
    requestAnimationFrame(() => layoutImages(items[index]));
  };
  document.getElementById("opacity").oninput = (event) => {
    document.getElementById("stage").style.setProperty("--overlay-opacity", Number(event.target.value) / 100);
    persist();
  };
  bindBoxDrawing(referenceBoxes, referenceCanvas);
  bindBoxDrawing(chemsemaBoxes, chemsemaCanvas);
  new ResizeObserver(() => layoutImages(items[index])).observe(document.getElementById("stage"));
  addEventListener("keydown", (event) => {
    if (event.target.matches("textarea, input")) return;
    if (event.key === "ArrowLeft") show(index - 1);
    if (event.key === "ArrowRight") show(index + 1);
    if (event.key.toLowerCase() === "p") setState("pass");
    if (event.key.toLowerCase() === "i") setState("issue");
    if (event.key.toLowerCase() === "b") setBoxMode(!boxModeEnabled);
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z" && currentState().boxes.length) {
      event.preventDefault();
      currentState().boxes.pop();
      persist();
      updateItemControls();
      renderBoxes();
    }
  });
  addEventListener("pagehide", persist);
  document.getElementById("opacity").value = String(saved.__ui.opacity ?? 50);
  document.getElementById("stage").style.setProperty(
    "--overlay-opacity",
    Number(document.getElementById("opacity").value) / 100,
  );
  if (mode === "overlay") document.getElementById("overlayMode").click();
  else document.getElementById("sideMode").click();
  setBoxMode(boxModeEnabled);
  show(index);
</script>
</body>
</html>`;
}

function placeholderSvg(title, detail) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800" viewBox="0 0 1200 800">
  <rect width="1200" height="800" fill="#fff"/>
  <rect x="80" y="120" width="1040" height="560" rx="18" fill="#fff7f7" stroke="#c83b3b" stroke-width="3"/>
  <text x="120" y="235" font-family="Segoe UI, Microsoft YaHei, sans-serif" font-size="42" font-weight="700" fill="#9f2424">${escapeHtml(title)}</text>
  <foreignObject x="120" y="285" width="960" height="300"><div xmlns="http://www.w3.org/1999/xhtml" style="font:28px/1.5 'Segoe UI','Microsoft YaHei',sans-serif;color:#333;white-space:pre-wrap;overflow-wrap:anywhere">${escapeHtml(detail)}</div></foreignObject>
</svg>`;
}

function svgHasDrawableContent(svg) {
  return /<(?:path|polygon|polyline|line|circle|ellipse|rect|text)\b/i.test(svg);
}

export function oracleGalleryCorpusErrors(recorded, current) {
  const errors = [];
  if (recorded?.corpus?.manifestSha256 !== current?.corpus?.manifestSha256) {
    errors.push("corpus-manifest");
  }
  const recordedSources = new Map(
    (recorded?.corpus?.sources ?? []).map((source) => [source.id, source.actualRevision]),
  );
  const currentSources = new Map(
    (current?.corpus?.sources ?? []).map((source) => [source.id, source.actualRevision]),
  );
  const sourceIds = new Set([...recordedSources.keys(), ...currentSources.keys()]);
  for (const sourceId of [...sourceIds].sort()) {
    if (recordedSources.get(sourceId) !== currentSources.get(sourceId)) {
      errors.push(`corpus-source:${sourceId}`);
    }
  }
  return errors;
}

async function retainedOracleGallery(oracleGallery, currentProvenance) {
  if (!oracleGallery) return null;
  const galleryDir = path.resolve(oracleGallery);
  const manifestPath = path.join(galleryDir, "manifest.json");
  const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
  const compatibilityErrors = oracleGalleryCorpusErrors(
    manifest.provenance,
    currentProvenance,
  );
  if (compatibilityErrors.length) {
    throw new Error(
      `Retained ChemDraw oracle gallery is incompatible: ${compatibilityErrors.join(", ")}`,
    );
  }
  return {
    galleryDir,
    manifestPath,
    manifestSha256: sha256File(manifestPath),
    itemsByPath: new Map(
      manifest.items.map((item) => [item.relativeCdxml.replaceAll("\\", "/"), item]),
    ),
  };
}

async function fullCorpusPairs(
  root,
  reportPath,
  outDir,
  patterns = [],
  oracleGallery = null,
  currentProvenance = null,
) {
  const report = JSON.parse(await fs.readFile(path.resolve(reportPath), "utf8"));
  const selectedCases = patterns.length === 0
    ? report.cases
    : report.cases.filter((entry) =>
      patterns.some((pattern) => matchesPublicCdxmlCasePattern(entry, pattern)));
  const pairs = selectedCases.map((entry) => {
    const input = path.join(root, entry.source, entry.path);
    const title = path.basename(input, path.extname(input));
    const oracleName = `${entry.caseId}_${entry.source}_${safeName(title)}`;
    return {
      cdxml: input,
      caseId: entry.caseId,
      format: entry.format,
      status: entry.status,
      oracleName,
      referenceLabel: "ChemDraw 渲染",
      chemsemaLabel: "ChemSema 首次导入",
    };
  });

  const oracleDir = path.join(outDir, "chemdraw-oracle");
  await fs.mkdir(oracleDir, { recursive: true });
  const retainedOracle = await retainedOracleGallery(oracleGallery, currentProvenance);
  const candidates = [];
  let retainedCount = 0;
  for (const pair of pairs) {
    if (["expected-reject", "skipped"].includes(pair.status)) continue;
    const output = path.join(oracleDir, `${pair.oracleName}.chemdraw.svg`);
    if (await fs.stat(output).then(() => true, () => false)) continue;

    // A completed gallery item is also a durable ChemDraw oracle cache. This
    // includes placeholders for files ChemDraw cannot open, so repeated visual
    // review runs never relaunch ChemDraw merely to rediscover the same failure.
    const relativeCdxml = path.relative(root, pair.cdxml).replaceAll("\\", "/");
    const itemId = `${pair.caseId}_${safeName(relativeCdxml)}`;
    if (retainedOracle) {
      const retainedItem = retainedOracle.itemsByPath.get(relativeCdxml);
      if (!retainedItem?.reference) {
        throw new Error(
          `Retained ChemDraw oracle gallery is missing ${relativeCdxml}`,
        );
      }
      const retainedReference = path.resolve(
        retainedOracle.galleryDir,
        retainedItem.reference,
      );
      await fs.access(retainedReference);
      await fs.copyFile(retainedReference, output);
      retainedCount += 1;
      continue;
    }
    const retainedReference = path.join(outDir, "items", itemId, "reference.svg");
    if (await fs.stat(retainedReference).then(() => true, () => false)) {
      await fs.copyFile(retainedReference, output);
      continue;
    }
    candidates.push(pair);
  }
  if (retainedOracle) {
    console.log(
      `[CHEMDRAW] retained ${retainedCount} oracle SVGs from ${retainedOracle.galleryDir}`,
    );
  }
  const chunkSize = 32;
  for (let offset = 0; offset < candidates.length; offset += chunkSize) {
    const chunk = candidates.slice(offset, offset + chunkSize);
    try {
      await generateChemDrawOracle({
        outDir: oracleDir,
        formats: ["svg"],
        inputs: chunk.map((pair) => pair.cdxml),
        outputNames: chunk.map((pair) => pair.oracleName),
      });
    } catch (error) {
      console.warn(`[CHEMDRAW] batch ${offset + 1}-${offset + chunk.length} failed; retrying individually`);
      for (const pair of chunk) {
        try {
          await generateChemDrawOracle({
            outDir: oracleDir,
            formats: ["svg"],
            inputs: [pair.cdxml],
            outputNames: [pair.oracleName],
          });
        } catch (individualError) {
          pair.referenceError = individualError instanceof Error ? individualError.message : String(individualError);
        }
      }
    }
  }

  for (const pair of pairs) {
    pair.reference = path.join(oracleDir, `${pair.oracleName}.chemdraw.svg`);
  }
  return pairs;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/render-public-cdxml-visual-review.mjs [--root corpus] [--out directory] [--cli chemsema-cli] [--all --report report.json] [--oracle-gallery retained-gallery] [--only case-or-path] [--incremental] [--jobs n] [--allow-dirty]");
    return;
  }

  const root = path.resolve(args.root);
  const outDir = path.resolve(args.outDir);
  const cli = path.resolve(args.cli);
  if (args.oracleGallery && !args.all) {
    throw new Error("--oracle-gallery requires --all");
  }
  await fs.access(cli);
  const provenance = collectGalleryProvenance({
    repoRoot,
    cliPath: cli,
    corpusRoot: root,
    corpusManifestPath: path.join(repoRoot, "benchmarks", "public-cdxml", "manifest.json"),
    roundtripReportPath: path.resolve(args.report),
  });
  if (provenance.repository.dirty && !args.allowDirty) {
    throw new Error(
      "Refusing to generate a canonical public gallery from a dirty repository. "
      + "Commit the changes or pass --allow-dirty for an explicitly non-release development run.",
    );
  }
  const retainedManifest = args.incremental
    ? JSON.parse(await fs.readFile(path.join(outDir, "manifest.json"), "utf8"))
    : null;
  const allFiles = await walk(root);
  const pairs = args.all
    ? await fullCorpusPairs(
      root,
      args.report,
      outDir,
      args.patterns,
      args.oracleGallery,
      provenance,
    )
    : allFiles
      .filter((file) => file.toLowerCase().endsWith(".cdxml"))
      .map((cdxml) => ({
        cdxml,
        png: cdxml.replace(/\.cdxml$/i, ".png"),
        referenceLabel: "公共参考图（原始 PNG）",
        chemsemaLabel: "ChemSema 导入结果（完整 SVG 渲染）",
      }))
      .filter(({ png }) => allFiles.includes(png))
      .sort((left, right) => left.cdxml.localeCompare(right.cdxml, "en"));

  if (pairs.length === 0) throw new Error(`No matching CDXML/PNG pairs found under ${root}`);
  if (args.oracleGallery) {
    const retainedManifestPath = path.resolve(args.oracleGallery, "manifest.json");
    provenance.oracleGallery = {
      path: path.resolve(args.oracleGallery),
      manifestSha256: sha256File(retainedManifestPath),
    };
  }
  await fs.mkdir(path.join(outDir, "items"), { recursive: true });

  const browser = await launchBrowser({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 2520, height: 1080 },
    deviceScaleFactor: 1.5,
  });
  const workerCount = Math.min(args.jobs, pairs.length);
  const pages = await Promise.all(
    Array.from({ length: workerCount }, () => context.newPage()),
  );
  let completed = 0;
  let manifestItems;

  try {
    manifestItems = await mapWithConcurrency(pairs, args.jobs, async (pair, index, workerIndex) => {
      const page = pages[workerIndex];
      const relativeCdxml = path.relative(root, pair.cdxml).replaceAll("\\", "/");
      const prefix = String(index + 1).padStart(3, "0");
      const id = pair.caseId
        ? `${pair.caseId}_${safeName(relativeCdxml)}`
        : `${prefix}_${safeName(relativeCdxml)}`;
      const itemDir = path.join(outDir, "items", id);
      const referenceExtension = args.all ? ".svg" : ".png";
      const referencePath = path.join(itemDir, `reference${referenceExtension}`);
      const chemsemaPath = path.join(itemDir, "chemsema.svg");
      const comparisonPath = path.join(itemDir, "comparison.png");
      await fs.mkdir(itemDir, { recursive: true });
      if (args.all && (pair.referenceError || !await fs.stat(pair.reference).then(() => true, () => false))) {
        await fs.writeFile(
          referencePath,
          placeholderSvg("ChemDraw 无法渲染", pair.referenceError ?? `案例状态：${pair.status}`),
        );
      } else {
        await fs.copyFile(args.all ? pair.reference : pair.png, referencePath);
      }
      try {
        await execFileAsync(cli, ["convert", pair.cdxml, chemsemaPath], {
          cwd: process.cwd(),
          maxBuffer: 16 * 1024 * 1024,
          // Gallery workers run concurrently. The CLI cache is a local
          // interactive optimization, not a parallel build artifact; sharing
          // it across workers can return another conversion's derived state
          // and make the visual gate nondeterministic.
          env: publicCdxmlCliEnvironment(),
        });
        const renderedSvg = await fs.readFile(chemsemaPath, "utf8");
        if (!svgHasDrawableContent(renderedSvg)) {
          throw new Error("ChemSema conversion succeeded but produced no drawable SVG primitives.");
        }
      } catch (error) {
        const detail = error?.stderr || error?.message || String(error);
        await fs.writeFile(chemsemaPath, placeholderSvg("ChemSema 按预期未导入", detail));
      }

      const [referenceBuffer, chemsemaBuffer] = await Promise.all([
        fs.readFile(referencePath),
        fs.readFile(chemsemaPath),
      ]);
      const referenceDataUrl = dataUrl(
        referenceBuffer,
        args.all ? "image/svg+xml" : "image/png",
      );
      const chemsemaDataUrl = dataUrl(chemsemaBuffer, "image/svg+xml");
      const alignment = await computeImageAlignment(
        page,
        referenceDataUrl,
        chemsemaDataUrl,
      );
      const screenshotItem = {
        number: String(index + 1),
        total: String(pairs.length),
        title: path.basename(pair.cdxml, path.extname(pair.cdxml)),
        relativeCdxml,
        referenceLabel: pair.referenceLabel,
        chemsemaLabel: pair.chemsemaLabel,
      };
      await page.setContent(
        reviewScreenshotHtml(
          screenshotItem,
          referenceDataUrl,
          chemsemaDataUrl,
          alignment,
        ),
        { waitUntil: "networkidle" },
      );
      await page.waitForFunction(() => document.body.dataset.ready === "yes");
      await page.screenshot({ path: comparisonPath, fullPage: true });

      const manifestItem = {
        id,
        label: `${String(index + 1).padStart(3, "0")} · ${screenshotItem.title}`,
        relativeCdxml,
        sourceCdxml: pair.cdxml,
        sourceReference: args.all ? pair.reference : pair.png,
        referenceLabel: pair.referenceLabel,
        chemsemaLabel: pair.chemsemaLabel,
        format: pair.format ?? "cdxml",
        status: pair.status ?? "reference-pair",
        candidateProvenance: {
          repositoryIdentity: provenance.repository.identity,
          cliSha256: provenance.cli.sha256,
        },
        alignment,
        reference: `items/${id}/reference${referenceExtension}`,
        chemsema: `items/${id}/chemsema.svg`,
        comparison: `items/${id}/comparison.png`,
      };
      completed += 1;
      console.log(`[${completed}/${pairs.length}] worker=${workerIndex + 1} ${relativeCdxml}`);
      return manifestItem;
    });
  } finally {
    await browser.close();
  }

  const finalManifestItems = retainedManifest
    ? mergeIncrementalManifestItems(retainedManifest.items, manifestItems)
    : manifestItems;
  await Promise.all([
    fs.writeFile(path.join(outDir, "index.html"), viewerHtml(finalManifestItems)),
    fs.writeFile(
      path.join(outDir, "manifest.json"),
      `${JSON.stringify({
        generatedAt: new Date().toISOString(),
        count: finalManifestItems.length,
        incremental: Boolean(retainedManifest),
        updatedCount: manifestItems.length,
        provenance,
        items: finalManifestItems,
      }, null, 2)}\n`,
    ),
  ]);
  console.log(`Review gallery: ${pathToFileURL(path.join(outDir, "index.html")).href}`);
}

export function mergeIncrementalManifestItems(retainedItems, updatedItems) {
  const updates = new Map(updatedItems.map((item) => [item.id, item]));
  const merged = retainedItems.map((item) => {
    const update = updates.get(item.id);
    return update ? { ...update, label: item.label } : item;
  });
  const retainedIds = new Set(retainedItems.map((item) => item.id));
  merged.push(...updatedItems.filter((item) => !retainedIds.has(item.id)));
  return merged;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack || error.message : String(error));
    process.exit(1);
  });
}
