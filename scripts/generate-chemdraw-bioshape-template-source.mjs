import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const probeDir = path.resolve(root, "tmp/chemdraw-bioshape-geometry-probe");
const manifest = JSON.parse(await fs.readFile(path.join(probeDir, "manifest.json"), "utf8"));
const output = path.join(
  root,
  "crates/chemsema-engine/src/render_objects/graphics/bio_shape_templates.generated.rs",
);
const selected = new Map([
  ["EndoplasmicReticulum", "ENDOPLASMIC_RETICULUM"],
  ["Golgi", "GOLGI"],
  ["Mitochondrion", "MITOCHONDRION"],
  ["tRNA", "TRNA"],
]);
const visualStem = new Map([
  ["EndoplasmicReticulum", "endoplasmic-reticulum"],
  ["Golgi", "golgi"],
  ["Mitochondrion", "mitochondrion"],
  ["tRNA", "t-rna"],
]);
const visualAxes = {
  center: [120, 100],
  major: [180, 100],
  minor: [120, 140],
};

function normalizedPath(d, axes) {
  const tokens = [...d.matchAll(/[A-Za-z]|-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)]
    .map((match) => match[0]);
  if (tokens.shift() !== "M") throw new Error(`Expected M path: ${d}`);
  const center = axes.center;
  const major = [axes.major[0] - center[0], axes.major[1] - center[1]];
  const minor = [axes.minor[0] - center[0], axes.minor[1] - center[1]];
  const determinant = major[0] * minor[1] - major[1] * minor[0];
  const point = () => {
    const x = Number(tokens.shift()) / 20 - center[0];
    const y = Number(tokens.shift()) / 20 - center[1];
    return [
      (x * minor[1] - y * minor[0]) / determinant,
      (major[0] * y - major[1] * x) / determinant,
    ];
  };
  const start = point();
  const segments = [];
  while (tokens.length) {
    const command = tokens.shift();
    if (command === "Z") break;
    if (command !== "C") throw new Error(`Expected cubic command, received ${command}`);
    segments.push([point(), point(), point()]);
  }
  return { start, segments };
}

function normalizedPolyline(d, axes) {
  const values = [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]),
  );
  const center = axes.center;
  const major = [axes.major[0] - center[0], axes.major[1] - center[1]];
  const minor = [axes.minor[0] - center[0], axes.minor[1] - center[1]];
  const determinant = major[0] * minor[1] - major[1] * minor[0];
  const points = [];
  for (let index = 0; index + 1 < values.length; index += 2) {
    const x = values[index] / 20 - center[0];
    const y = values[index + 1] / 20 - center[1];
    points.push([
      (x * minor[1] - y * minor[0]) / determinant,
      (major[0] * y - major[1] * x) / determinant,
    ]);
  }
  return points;
}

function rustNumber(value) {
  if (Math.abs(value) < 0.000_000_5) return "0.0";
  let text = value.toFixed(6).replace(/0+$/, "").replace(/\.$/, "");
  if (!text.includes(".")) text += ".0";
  return text;
}

function rustPoint(point) {
  return `(${rustNumber(point[0])}, ${rustNumber(point[1])})`;
}

function pathBounds(d) {
  const values = [...d.matchAll(/-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/gi)].map(
    (match) => Number(match[0]),
  );
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

function templateBounds(template) {
  const points = [
    template.start,
    ...template.segments.flatMap((segment) => segment),
  ];
  return [
    Math.min(...points.map((point) => point[0])),
    Math.min(...points.map((point) => point[1])),
    Math.max(...points.map((point) => point[0])),
    Math.max(...points.map((point) => point[1])),
  ];
}

function shadeAnchorEntries(svg) {
  const entries = [];
  for (const group of svg.matchAll(
    /<g clip-path="url\(#\d+\)" >([\s\S]*?)<\/g>\s*<path\b([^>]*)>/g,
  )) {
    const paths = [...group[1].matchAll(/<path[^>]*d="([^"]+)"/g)].map(
      (match) => match[1],
    );
    if (paths.length !== 32) continue;
    const outline = group[2].match(/\bd="([^"]+)"/i)?.[1];
    if (!outline) continue;
    const outer = pathBounds(paths[0]);
    const inner = pathBounds(paths.at(-1));
    const scale =
      (inner.right - inner.left) / (outer.right - outer.left);
    const anchor = (inner.left - scale * outer.left) / (1 - scale);
    entries.push({
      anchor: (anchor - outer.left) / (outer.right - outer.left),
      outline,
    });
  }
  return entries;
}

function reorderedShadeAnchors(svg, templates) {
  const entries = shadeAnchorEntries(svg).map((entry) => ({
    ...entry,
    bounds: templateBounds(normalizedPath(entry.outline, visualAxes)),
  }));
  const shadedTemplates = templates.slice(0, entries.length);
  const unused = new Set(entries.map((_, index) => index));
  return shadedTemplates.map((template) => {
    const bounds = templateBounds(template);
    let bestIndex = null;
    let bestError = Number.POSITIVE_INFINITY;
    for (const index of unused) {
      const error = entries[index].bounds.reduce(
        (sum, value, coordinate) =>
          sum + (value - bounds[coordinate]) ** 2,
        0,
      );
      if (error < bestError) {
        bestError = error;
        bestIndex = index;
      }
    }
    if (bestIndex == null || bestError > 0.001) {
      throw new Error(`Unable to match shaded contour, error=${bestError}`);
    }
    unused.delete(bestIndex);
    return entries[bestIndex].anchor;
  });
}

const chunks = [
  "// @generated by scripts/generate-chemdraw-bioshape-template-source.mjs",
  "// Source rule: ChemDraw SVG cubic paths normalized by MajorAxis/MinorAxis.",
  "",
];
for (const [type, constant] of selected) {
  const entry = manifest.cases.find((candidate) =>
    candidate.type === type && candidate.variant === "base");
  if (!entry) throw new Error(`${type}: base probe missing`);
  const svg = await fs.readFile(path.join(root, entry.svg), "utf8");
  const pathAttributes = [...svg.matchAll(/<path\b([^>]*)>/gi)]
    .map((match) => match[1])
    .filter((attributes) => /stroke="(?!none)[^"]+"/i.test(attributes));
  const templateNames = [];
  const normalizedTemplates = [];
  for (const [index, attributes] of pathAttributes.entries()) {
    const d = attributes.match(/\bd="([^"]*)"/i)?.[1];
    if (!d) throw new Error(`${type}/${index}: path data missing`);
    const template = normalizedPath(d, entry.axes);
    normalizedTemplates.push(template);
    const segmentName = `${constant}_PATH_${index}_SEGMENTS`;
    const templateName = `${constant}_PATH_${index}`;
    chunks.push(`const ${segmentName}: [CubicSegment; ${template.segments.length}] = [`);
    for (const segment of template.segments) {
      chunks.push(
        `    cubic(${rustPoint(segment[0])}, ${rustPoint(segment[1])}, ${rustPoint(segment[2])}),`,
      );
    }
    chunks.push(
      "];",
      `const ${templateName}: CubicTemplate = CubicTemplate {`,
      `    start: ${rustPoint(template.start)},`,
      `    segments: &${segmentName},`,
      "};",
      "",
    );
    templateNames.push(`&${templateName}`);
  }
  chunks.push(
    `const ${constant}_TEMPLATES: [&CubicTemplate; ${templateNames.length}] = [`,
    ...templateNames.map((name) => `    ${name},`),
    "];",
    "",
  );
  const shadeSvg = await fs.readFile(
    path.join(
      root,
      "tmp/chemdraw-bioshape-probe/chemdraw",
      `${visualStem.get(type)}.chemdraw.svg`,
    ),
    "utf8",
  );
  const anchors = reorderedShadeAnchors(shadeSvg, normalizedTemplates);
  chunks.push(
    `const ${constant}_SHADE_ANCHORS: [f64; ${anchors.length}] = [`,
    ...anchors.map((anchor) => `    ${rustNumber(anchor)},`),
    "];",
    "",
  );
  if (type === "tRNA") {
    const clipD = shadeSvg.match(
      /<clipPath id="1" >\s*<path[^>]*d="([^"]+)"/,
    )?.[1];
    const outerD = shadeSvg.match(
      /<g clip-path="url\(#1\)" >\s*<path[^>]*d="([^"]+)"/,
    )?.[1];
    if (!clipD || !outerD) throw new Error("tRNA shaded polygons missing");
    for (const [name, points] of [
      ["TRNA_SHADE_CLIP_POLYGON", normalizedPolyline(clipD, visualAxes)],
      ["TRNA_SHADE_OUTER_POLYGON", normalizedPolyline(outerD, visualAxes)],
    ]) {
      chunks.push(
        `const ${name}: [(f64, f64); ${points.length}] = [`,
        ...points.map((point) => `    ${rustPoint(point)},`),
        "];",
        "",
      );
    }
  }
}
await fs.writeFile(output, `${chunks.join("\n")}\n`, "utf8");
console.log(`[BIOSHAPE TEMPLATE] wrote ${path.relative(root, output)}`);
