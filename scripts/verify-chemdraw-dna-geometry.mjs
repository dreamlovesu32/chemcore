import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = path.resolve(import.meta.dirname, "..");
const probeRoot = path.join(root, "tmp/chemdraw-bioshape-geometry-probe");
const outputRoot = path.join(probeRoot, "chemsema", "dna");
const manifest = JSON.parse(
  await fs.readFile(path.join(probeRoot, "manifest.json"), "utf8"),
);

function blackPaths(svg) {
  return [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="#(?:000000|000)"/i.test(attributes))
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
  (candidate) => candidate.type === "DNA",
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
  const expectedPaths = blackPaths(chemdrawSvg);
  const actualPaths = blackPaths(chemsemaSvg);
  let exactTopology = expectedPaths.length === actualPaths.length;
  let maximumDelta = 0;
  let totalDelta = 0;
  let coordinateCount = 0;
  const unused = new Set(actualPaths.map((_, index) => index));
  for (const expectedPath of expectedPaths) {
    const expected = coordinates(expectedPath, 20);
    let best = null;
    for (const index of unused) {
      const actual = coordinates(actualPaths[index], 1);
      if (expected.length !== actual.length) continue;
      const deltas = expected.map((value, coordinate) =>
        Math.abs(value - actual[coordinate])
      );
      const error = Math.max(...deltas);
      if (best == null || error < best.error) best = { index, error, deltas };
    }
    if (best == null) {
      exactTopology = false;
      continue;
    }
    unused.delete(best.index);
    maximumDelta = Math.max(maximumDelta, best.error);
    totalDelta += best.deltas.reduce((sum, delta) => sum + delta, 0);
    coordinateCount += best.deltas.length;
  }
  results.push({
    variant: entry.variant,
    pathCount: [expectedPaths.length, actualPaths.length],
    exactTopology,
    maximumDelta: Number(maximumDelta.toFixed(6)),
    meanDelta: Number((totalDelta / coordinateCount).toFixed(6)),
  });
}
const report = {
  schema: "chemsema.chemdraw-dna-geometry-gate.v1",
  pass: results.every(
    (result) => result.exactTopology && result.maximumDelta <= 0.003,
  ),
  tolerance: 0.003,
  results,
};
await fs.writeFile(
  path.join(outputRoot, "report.json"),
  `${JSON.stringify(report, null, 2)}\n`,
);
console.log(JSON.stringify(report, null, 2));
if (!report.pass) process.exitCode = 1;
