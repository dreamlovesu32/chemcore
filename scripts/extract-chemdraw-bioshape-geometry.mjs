import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchBrowser } from "./playwright-browser.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const probeDir = path.resolve(root, process.argv[2] ?? "tmp/chemdraw-bioshape-geometry-probe");
const manifest = JSON.parse(await fs.readFile(path.join(probeDir, "manifest.json"), "utf8"));
if (manifest.schema !== "chemsema.chemdraw-bioshape-geometry-probe.v1") {
  throw new Error("Unsupported BioShape geometry probe manifest.");
}

const outputPath = path.join(probeDir, "extracted-geometry.json");
let cases = [];
try {
  const previous = JSON.parse(await fs.readFile(outputPath, "utf8"));
  if (previous.schema === "chemsema.chemdraw-bioshape-extracted-geometry.v1") {
    cases = previous.cases;
  }
} catch (error) {
  if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error;
}
const completed = new Set(cases.map((entry) => `${entry.type}/${entry.variant}`));
const writeCheckpoint = () => fs.writeFile(
  outputPath,
  `${JSON.stringify({
    schema: "chemsema.chemdraw-bioshape-extracted-geometry.v1",
    sampleSpacingPixels: 0.5,
    cases,
  })}\n`,
);

const browser = await launchBrowser({ headless: true });
try {
  const page = await browser.newPage();
  for (const entry of manifest.cases) {
    const key = `${entry.type}/${entry.variant}`;
    if (completed.has(key)) {
      console.log(`[BIOSHAPE GEOMETRY] ${key}: cached`);
      continue;
    }
    const svg = await fs.readFile(path.join(root, entry.svg), "utf8");
    await page.setContent(svg);
    const geometry = await page.evaluate(() => {
      const svgRoot = document.querySelector("svg");
      const applyMatrix = (matrix, point) => ({
        x: matrix.a * point.x + matrix.c * point.y + matrix.e,
        y: matrix.b * point.x + matrix.d * point.y + matrix.f,
      });
      const round = (value) => Math.round(value * 1e5) / 1e5;
      const paths = [...document.querySelectorAll("path")]
        .filter((element) => !element.closest("defs, clipPath"))
        .map((element, index) => {
          const style = getComputedStyle(element);
          const length = element.getTotalLength();
          const matrix = element.getCTM();
          const sampleCount = Math.max(2, Math.ceil(length / 0.5));
          const samples = [];
          for (let sampleIndex = 0; sampleIndex <= sampleCount; sampleIndex += 1) {
            const local = element.getPointAtLength(length * sampleIndex / sampleCount);
            const point = applyMatrix(matrix, local);
            samples.push([round(point.x), round(point.y)]);
          }
          const box = element.getBBox();
          const corners = [
            { x: box.x, y: box.y },
            { x: box.x + box.width, y: box.y },
            { x: box.x + box.width, y: box.y + box.height },
            { x: box.x, y: box.y + box.height },
          ].map((point) => applyMatrix(matrix, point));
          return {
            index,
            d: element.getAttribute("d"),
            transform: element.getAttribute("transform"),
            stroke: style.stroke,
            strokeWidth: round(Number.parseFloat(style.strokeWidth) || 0),
            fill: style.fill,
            closed: /[zZ]\s*$/.test(element.getAttribute("d") ?? ""),
            length: round(length),
            bounds: {
              left: round(Math.min(...corners.map((point) => point.x))),
              top: round(Math.min(...corners.map((point) => point.y))),
              right: round(Math.max(...corners.map((point) => point.x))),
              bottom: round(Math.max(...corners.map((point) => point.y))),
            },
            samples,
          };
        });
      const visibleStrokePaths = paths.filter(
        (entry) => entry.stroke !== "none" && entry.strokeWidth > 0,
      );
      let left = Infinity;
      let top = Infinity;
      let right = -Infinity;
      let bottom = -Infinity;
      let pointCount = 0;
      for (const entry of visibleStrokePaths) {
        for (const [x, y] of entry.samples) {
          left = Math.min(left, x);
          top = Math.min(top, y);
          right = Math.max(right, x);
          bottom = Math.max(bottom, y);
          pointCount += 1;
        }
      }
      return {
        viewBox: svgRoot.getAttribute("viewBox"),
        width: svgRoot.getAttribute("width"),
        height: svgRoot.getAttribute("height"),
        paths,
        visibleStrokePaths,
        strokeBounds: pointCount === 0 ? null : {
          left: round(left),
          top: round(top),
          right: round(right),
          bottom: round(bottom),
        },
      };
    });
    cases.push({
      type: entry.type,
      variant: entry.variant,
      axes: entry.axes,
      requestedParameters: entry.requestedParameters,
      normalizedTag: entry.normalizedTag,
      svg: entry.svg,
      geometry,
    });
    console.log(
      `[BIOSHAPE GEOMETRY] ${entry.type}/${entry.variant}: `
      + `${geometry.visibleStrokePaths.length} stroked paths`,
    );
    await writeCheckpoint();
  }
  await writeCheckpoint();
} finally {
  await browser.close();
}
