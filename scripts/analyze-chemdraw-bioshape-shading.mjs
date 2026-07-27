import fs from "node:fs";
import path from "node:path";

const inputPath = path.resolve(
  process.argv[2] ??
    "tmp/chemdraw-bioshape-probe/chemdraw/dna.chemdraw.svg",
);
const svg = fs.readFileSync(inputPath, "utf8");

function numbers(d) {
  return [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]),
  );
}

function bounds(d) {
  const values = numbers(d);
  const xs = [];
  const ys = [];
  for (let index = 0; index + 1 < values.length; index += 2) {
    xs.push(values[index]);
    ys.push(values[index + 1]);
  }
  return {
    left: Math.min(...xs),
    top: Math.min(...ys),
    right: Math.max(...xs),
    bottom: Math.max(...ys),
  };
}

function inferTransform(outer, inner) {
  const scaleX =
    (inner.right - inner.left) / (outer.right - outer.left);
  const scaleY =
    (inner.bottom - inner.top) / (outer.bottom - outer.top);
  const anchorX = (inner.left - scaleX * outer.left) / (1 - scaleX);
  const anchorY = (inner.top - scaleY * outer.top) / (1 - scaleY);
  return {
    scaleX,
    scaleY,
    anchorX,
    anchorY,
    anchorFractionX: (anchorX - outer.left) / (outer.right - outer.left),
    anchorFractionY: (anchorY - outer.top) / (outer.bottom - outer.top),
  };
}

function points(d) {
  const values = numbers(d);
  const result = [];
  for (let index = 0; index + 1 < values.length; index += 2) {
    result.push({ x: values[index], y: values[index + 1] });
  }
  return result;
}

function pointInPolygon(point, polygon) {
  let inside = false;
  let previous = polygon.at(-1);
  for (const current of polygon) {
    if (
      (current.y > point.y) !== (previous.y > point.y) &&
      point.x <
        ((previous.x - current.x) * (point.y - current.y)) /
          (previous.y - current.y) +
          current.x
    ) {
      inside = !inside;
    }
    previous = current;
  }
  return inside;
}

function polygonCentroid(d, box) {
  const polygon = points(d);
  let twiceArea = 0;
  let x = 0;
  let y = 0;
  for (let index = 0; index < polygon.length; index += 1) {
    const current = polygon[index];
    const next = polygon[(index + 1) % polygon.length];
    const cross = current.x * next.y - next.x * current.y;
    twiceArea += cross;
    x += (current.x + next.x) * cross;
    y += (current.y + next.y) * cross;
  }
  return {
    xFraction:
      (x / (3 * twiceArea) - box.left) / (box.right - box.left),
    yFraction:
      (y / (3 * twiceArea) - box.top) / (box.bottom - box.top),
  };
}

function diagonalIntervals(d, box) {
  const polygon = points(d);
  const intervals = [];
  let start = null;
  for (let index = 0; index <= 100_000; index += 1) {
    const fraction = index / 100_000;
    const inside = pointInPolygon(
      {
        x: box.left + (box.right - box.left) * fraction,
        y: box.top + (box.bottom - box.top) * fraction,
      },
      polygon,
    );
    if (inside && start == null) start = fraction;
    if (!inside && start != null) {
      intervals.push([start, (index - 1) / 100_000]);
      start = null;
    }
  }
  if (start != null) intervals.push([start, 1]);
  return intervals;
}

const groupPattern =
  /<g clip-path="url\(#(\d+)\)" >([\s\S]*?)<\/g>/g;
const groups = [];
for (const match of svg.matchAll(groupPattern)) {
  const paths = [...match[2].matchAll(/<path[^>]*d="([^"]+)"/g)].map(
    (pathMatch) => pathMatch[1],
  );
  if (paths.length < 2) continue;
  const outer = bounds(paths[0]);
  const inner = bounds(paths.at(-1));
  const intervals = diagonalIntervals(paths[0], outer);
  const transform = inferTransform(outer, inner);
  const selectedInterval =
    intervals.find(
      ([start, end]) =>
        transform.anchorFractionX >= start - 0.0001 &&
        transform.anchorFractionX <= end + 0.0001,
    ) ?? null;
  groups.push({
    clipId: match[1],
    layerCount: paths.length,
    outer,
    inner,
    ...transform,
    layerScales: paths.map((pathData) => {
      const box = bounds(pathData);
      return (box.right - box.left) / (outer.right - outer.left);
    }),
    polygonCentroid: polygonCentroid(paths[0], outer),
    diagonalIntervals: intervals,
    selectedInterval,
    selectedMidpoint:
      selectedInterval == null
        ? null
        : (selectedInterval[0] + selectedInterval[1]) / 2,
  });
}

console.log(JSON.stringify({ inputPath, groups }, null, 2));
