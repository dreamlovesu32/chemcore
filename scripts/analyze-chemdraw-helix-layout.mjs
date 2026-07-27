import fs from "node:fs/promises";

const manifest = JSON.parse(
  await fs.readFile(
    "tmp/chemdraw-bioshape-geometry-probe/manifest.json",
    "utf8",
  ),
);

function pathPoints(d) {
  const values = [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]) / 20,
  );
  const points = [];
  for (let index = 0; index + 1 < values.length; index += 2) {
    points.push([values[index], values[index + 1]]);
  }
  return points;
}

for (const entry of manifest.cases.filter(
  (candidate) => candidate.type === "HelixProtein",
)) {
  const svg = await fs.readFile(entry.svg, "utf8");
  const paths = [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="(?!none)[^"]+"/i.test(attributes))
    .map((attributes, index) => {
      const d = attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "";
      const points = pathPoints(d);
      const xs = points.map((point) => point[0]);
      const ys = points.map((point) => point[1]);
      const first = points[0];
      const last = points.at(-1);
      return {
        index,
        cubicCount: (d.match(/\bC\b/g) ?? []).length,
        closed: Math.hypot(last[0] - first[0], last[1] - first[1]) < 0.001,
        bounds: [
          Math.min(...xs),
          Math.min(...ys),
          Math.max(...xs),
          Math.max(...ys),
        ].map((value) => Number(value.toFixed(4))),
        start: first.map((value) => Number(value.toFixed(4))),
        end: last.map((value) => Number(value.toFixed(4))),
      };
    });
  console.log(
    JSON.stringify({
      variant: entry.variant,
      requestedParameters: entry.requestedParameters,
      pathCount: paths.length,
      closedCount: paths.filter((path) => path.closed).length,
      paths,
    }),
  );
}
