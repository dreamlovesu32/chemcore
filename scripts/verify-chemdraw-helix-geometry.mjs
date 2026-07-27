import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = path.resolve(import.meta.dirname, "..");
const probeRoot = path.join(root, "tmp/chemdraw-bioshape-geometry-probe");
const outputRoot = path.join(probeRoot, "chemsema", "helix-protein");
const manifest = JSON.parse(
  await fs.readFile(path.join(probeRoot, "manifest.json"), "utf8"),
);

function strokedPaths(svg) {
  return [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="(?!none)[^"]+"/i.test(attributes))
    .map((attributes) => attributes.match(/\bd="([^"]*)"/i)?.[1] ?? "");
}

function coordinates(d, divisor) {
  return [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]) / divisor,
  );
}

await fs.mkdir(outputRoot, { recursive: true });
const results = [];
for (const entry of manifest.cases.filter(
  (candidate) => candidate.type === "HelixProtein",
)) {
  const output = path.join(outputRoot, `${entry.variant}.svg`);
  const conversion = spawnSync(
    path.join(root, "target/debug/chemsema-cli.exe"),
    ["convert", path.join(root, entry.input), output],
    { cwd: root, encoding: "utf8" },
  );
  if (conversion.status !== 0) {
    throw new Error(`${entry.variant}: conversion failed\n${conversion.stderr}`);
  }
  const [chemdrawSvg, chemsemaSvg] = await Promise.all([
    fs.readFile(path.join(root, entry.svg), "utf8"),
    fs.readFile(output, "utf8"),
  ]);
  const left = strokedPaths(chemdrawSvg);
  const right = strokedPaths(chemsemaSvg);
  if (left.length !== right.length) {
    results.push({
      variant: entry.variant,
      pathCount: [left.length, right.length],
      exactTopology: false,
    });
    continue;
  }
  let maximumDelta = 0;
  let totalDelta = 0;
  let coordinateCount = 0;
  let exactTopology = true;
  let worst = null;
  for (let index = 0; index < left.length; index += 1) {
    const expected = coordinates(left[index], 20);
    const actual = coordinates(right[index], 1);
    if (expected.length !== actual.length) {
      exactTopology = false;
      continue;
    }
    for (let coordinate = 0; coordinate < expected.length; coordinate += 1) {
      const delta = Math.abs(expected[coordinate] - actual[coordinate]);
      if (delta > maximumDelta) {
        maximumDelta = delta;
        worst = {
          path: index,
          coordinate,
          expected: expected[coordinate],
          actual: actual[coordinate],
        };
      }
      totalDelta += delta;
      coordinateCount += 1;
    }
  }
  results.push({
    variant: entry.variant,
    pathCount: left.length,
    exactTopology,
    maximumDelta: Number(maximumDelta.toFixed(6)),
    meanDelta: Number((totalDelta / coordinateCount).toFixed(6)),
    worst,
  });
}

const report = {
  schema: "chemsema.chemdraw-helix-geometry-gate.v1",
  pass: results.every(
    (result) =>
      result.exactTopology &&
      result.maximumDelta != null &&
      result.maximumDelta <= 0.001,
  ),
  tolerance: 0.001,
  results,
};
await fs.writeFile(
  path.join(outputRoot, "report.json"),
  `${JSON.stringify(report, null, 2)}\n`,
);
console.log(JSON.stringify(report, null, 2));
if (!report.pass) process.exitCode = 1;
