import fs from "node:fs/promises";
import path from "node:path";

const manifestPath = path.resolve(process.argv[2]);
const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
const center = manifest.axes.center;
const major = [
  manifest.axes.major[0] - center[0],
  manifest.axes.major[1] - center[1],
];
const minor = [
  manifest.axes.minor[0] - center[0],
  manifest.axes.minor[1] - center[1],
];
const determinant = major[0] * minor[1] - major[1] * minor[0];

function normalizePath(d) {
  const tokens = [...d.matchAll(/[A-Za-z]|-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => match[0],
  );
  if (tokens.shift() !== "M") throw new Error("Expected cubic path");
  const point = () => {
    const x = Number(tokens.shift()) / 20 - center[0];
    const y = Number(tokens.shift()) / 20 - center[1];
    return [
      (x * minor[1] - y * minor[0]) / determinant,
      (major[0] * y - major[1] * x) / determinant,
    ];
  };
  const points = [point()];
  while (tokens.length) {
    const command = tokens.shift();
    if (command === "Z") break;
    if (command !== "C") throw new Error(`Expected C, received ${command}`);
    points.push(point(), point(), point());
  }
  return points;
}

for (const entry of manifest.cases) {
  const svg = await fs.readFile(path.resolve(entry.svg), "utf8");
  const attributes = [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .find((value) => /stroke="(?!none)[^"]+"/i.test(value));
  const d = attributes?.match(/\bd="([^"]*)"/i)?.[1];
  if (!d) throw new Error(`${entry.value}: path missing`);
  console.log(JSON.stringify({ value: entry.value, points: normalizePath(d) }));
}
