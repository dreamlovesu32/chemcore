import fs from "node:fs/promises";

const manifest = JSON.parse(
  await fs.readFile("tmp/chemdraw-bioshape-geometry-probe/manifest.json", "utf8"),
);

function strokedPaths(svg) {
  return [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="(?!none)[^"]+"/i.test(attributes))
    .map((attributes) => attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "");
}

function points(d) {
  const values = [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]) / 20,
  );
  return Array.from({ length: values.length / 2 }, (_, index) => [
    values[index * 2],
    values[index * 2 + 1],
  ]);
}

for (const entry of manifest.cases.filter(
  (candidate) => candidate.type === "HelixProtein",
)) {
  const svg = await fs.readFile(entry.svg, "utf8");
  const paths = strokedPaths(svg);
  const cylinderCount = (paths.length - 1) / 3;
  const rightIndex = Math.floor(cylinderCount / 2);
  const leftIndex = rightIndex + 1 + cylinderCount * 2;
  console.log(JSON.stringify({
    variant: entry.variant,
    parameters: {
      cylinderDistance: entry.requestedParameters.CylinderDistance,
      cylinderHeight: entry.requestedParameters.CylinderHeight,
      cylinderWidth: entry.requestedParameters.CylinderWidth,
      helixProteinExtra: entry.requestedParameters.HelixProteinExtra,
      pipeWidth: entry.requestedParameters.PipeWidth,
    },
    cylinderCount,
    right: points(paths[rightIndex]),
    left: points(paths[leftIndex]),
  }));
}
