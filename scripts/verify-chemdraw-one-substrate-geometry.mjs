import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = path.resolve(import.meta.dirname, "..");
const sweepRoot = path.join(
  root,
  "tmp/chemdraw-bioshape-parameter-sweep/1-substrate-enzyme-enzyme-receptor-size",
);
const outputRoot = path.join(sweepRoot, "chemsema");
const manifest = JSON.parse(
  await fs.readFile(path.join(sweepRoot, "manifest.json"), "utf8"),
);

function strokedPath(svg) {
  const attributes = [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .find((entry) => /stroke="(?!none)[^"]+"/i.test(entry));
  return attributes?.match(/\bd="([^"]*)"/i)?.[1] ?? "";
}

function coordinates(d, divisor) {
  return [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]) / divisor,
  );
}

await fs.mkdir(outputRoot, { recursive: true });
const results = [];
for (const entry of manifest.cases) {
  const output = path.join(outputRoot, `${entry.value}.svg`);
  const conversion = spawnSync(
    path.join(root, "target/debug/chemsema-cli.exe"),
    ["convert", path.join(root, entry.input), output],
    { cwd: root, encoding: "utf8" },
  );
  if (conversion.status !== 0) {
    throw new Error(`${entry.value}: conversion failed\n${conversion.stderr}`);
  }
  const [chemdrawSvg, chemsemaSvg] = await Promise.all([
    fs.readFile(path.join(root, entry.svg), "utf8"),
    fs.readFile(output, "utf8"),
  ]);
  const expected = coordinates(strokedPath(chemdrawSvg), 20);
  const actual = coordinates(strokedPath(chemsemaSvg), 1);
  let maximumDelta = 0;
  let totalDelta = 0;
  for (let index = 0; index < expected.length; index += 1) {
    const delta = Math.abs(expected[index] - actual[index]);
    maximumDelta = Math.max(maximumDelta, delta);
    totalDelta += delta;
  }
  results.push({
    value: entry.value,
    exactTopology: expected.length === actual.length,
    maximumDelta: Number(maximumDelta.toFixed(6)),
    meanDelta: Number((totalDelta / expected.length).toFixed(6)),
  });
}
const report = {
  schema: "chemsema.chemdraw-one-substrate-geometry-gate.v1",
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
