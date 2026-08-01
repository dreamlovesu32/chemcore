import fs from "node:fs/promises";
import path from "node:path";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.resolve(
  root,
  process.argv[2] ?? "tmp/chemdraw-corner-radius-probe",
);
const oracleDir = path.join(outDir, "oracle");

const cornerRadii = [100, 200, 400, 600, 800, 1200];
const lineWidths = [0.4, 0.6, 1, 2];

function documentSource({ shadow }) {
  const cells = [];
  let id = 10;
  for (const [row, lineWidth] of lineWidths.entries()) {
    for (const [column, cornerRadius] of cornerRadii.entries()) {
      const left = 20 + column * 70;
      const top = 20 + row * 60;
      const rectangleType = shadow ? "RoundEdge Shadow" : "RoundEdge";
      const shadowSize = shadow ? ' ShadowSize="400"' : "";
      cells.push(
        `    <graphic id="${id++}" BoundingBox="${left} ${top} ${left + 44} ${top + 30}" GraphicType="Rectangle" RectangleType="${rectangleType}" CornerRadius="${cornerRadius}"${shadowSize} LineWidth="${lineWidth}"/>`,
      );
    }
  }
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemSema corner-radius probe" BoundingBox="0 0 450 270" LineWidth="0.6" BoldWidth="2" BondLength="14.4" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 450 270">
${cells.join("\n")}
  </page>
</CDXML>
`;
}

function defaultsDocumentSource() {
  const cases = [
    { name: "missing-corner", rectangleType: "RoundEdge", attrs: 'LineWidth="0.6"' },
    { name: "zero-corner", rectangleType: "RoundEdge", attrs: 'CornerRadius="0" LineWidth="0.6"' },
    { name: "inherited-line", rectangleType: "RoundEdge", attrs: 'CornerRadius="600"' },
    { name: "bold-line", rectangleType: "RoundEdge Bold", attrs: 'CornerRadius="600"' },
    { name: "filled", rectangleType: "RoundEdge Filled", attrs: 'CornerRadius="600"' },
    { name: "shaded", rectangleType: "RoundEdge Shaded", attrs: 'CornerRadius="600"' },
  ];
  const cells = cases.map(({ name, rectangleType, attrs }, index) => {
    const left = 20 + index * 70;
    return `    <!-- ${name} -->\n    <graphic id="${100 + index}" BoundingBox="${left} 20 ${left + 44} 50" GraphicType="Rectangle" RectangleType="${rectangleType}" ${attrs}/>`;
  });
  return `<?xml version="1.0" encoding="UTF-8" ?>
<CDXML CreationProgram="ChemSema corner-radius defaults probe" BoundingBox="0 0 450 90" LineWidth="0.6" BoldWidth="2" BondLength="14.4" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 450 90">
${cells.join("\n")}
  </page>
</CDXML>
`;
}

await fs.mkdir(outDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const inputs = [];
const outputNames = [];
for (const variant of ["plain", "shadow"]) {
  const input = path.join(outDir, `corner-radius-${variant}.cdxml`);
  await fs.writeFile(input, documentSource({ shadow: variant === "shadow" }), "utf8");
  inputs.push(input);
  outputNames.push(`corner-radius-${variant}`);
}
const defaultsInput = path.join(outDir, "corner-radius-defaults.cdxml");
await fs.writeFile(defaultsInput, defaultsDocumentSource(), "utf8");
inputs.push(defaultsInput);
outputNames.push("corner-radius-defaults");

await generateChemDrawOracle({
  inputs,
  outDir: oracleDir,
  formats: ["svg"],
  outputNames,
});

function pathElements(svg) {
  return [...svg.matchAll(/<path\b([^>]*)\/>/g)].map((match) => {
    const attributes = match[1];
    const d = attributes.match(/\bd="([^"]+)"/)?.[1];
    if (!d) {
      return null;
    }
    return {
      attributes,
      d,
      numbers: [...d.matchAll(/-?\d+(?:\.\d+)?/g)].map((number) =>
        Number(number[0]),
      ),
    };
  }).filter(Boolean);
}

function measureRoundedPath(path, left, bottom) {
  const numbers = path.numbers;
  if (numbers.length < 8 || Math.abs(numbers[0] - left) > 0.01) {
    throw new Error(`unexpected rounded path start: ${path.d}`);
  }
  const firstCurveEndX = numbers[6];
  const radiusX = Math.abs(firstCurveEndX - left) <= 0.01
    ? numbers[12] - left
    : firstCurveEndX - left;
  return {
    radiusX: radiusX / 20,
    radiusY: (bottom - numbers[1]) / 20,
    strokeWidthInternal: Number(
      path.attributes.match(/\bstroke-width="([^"]+)"/)?.[1] ?? 0,
    ),
  };
}

function assertClose(actual, expected, context) {
  if (Math.abs(actual - expected) > 0.001) {
    throw new Error(`${context}: expected ${expected}, measured ${actual}`);
  }
}

const report = { schema: "chemsema.chemdrawCornerRadiusProbe.v1", matrix: {}, defaults: [] };
for (const variant of ["plain", "shadow"]) {
  const svg = await fs.readFile(
    path.join(oracleDir, `corner-radius-${variant}.chemdraw.svg`),
    "utf8",
  );
  const outlines = pathElements(svg).filter(
    (entry) => entry.attributes.includes('stroke="#000000"')
      && entry.attributes.includes('fill="none"'),
  );
  if (outlines.length !== cornerRadii.length * lineWidths.length) {
    throw new Error(`${variant}: expected 24 outlines, found ${outlines.length}`);
  }
  report.matrix[variant] = outlines.map((outline, index) => {
    const row = Math.floor(index / cornerRadii.length);
    const column = index % cornerRadii.length;
    const lineWidth = lineWidths[row];
    const cornerRadius = cornerRadii[column];
    const left = (20 + column * 70) * 20;
    const bottom = (20 + row * 60 + 30) * 20;
    const measured = measureRoundedPath(outline, left, bottom);
    const nominal = cornerRadius / 100 * lineWidth;
    const expectedX = Math.min(nominal, 22);
    const expectedY = Math.min(nominal, 15);
    assertClose(measured.radiusX, expectedX, `${variant} rx ${lineWidth}/${cornerRadius}`);
    assertClose(measured.radiusY, expectedY, `${variant} ry ${lineWidth}/${cornerRadius}`);
    return { lineWidth, cornerRadius, nominal, expectedX, expectedY, ...measured };
  });
}

const defaultsSvg = await fs.readFile(
  path.join(oracleDir, "corner-radius-defaults.chemdraw.svg"),
  "utf8",
);
const defaultPaths = pathElements(defaultsSvg);
for (const [index, name] of [
  "missing-corner",
  "zero-corner",
  "inherited-line",
  "bold-line",
  "filled",
  "shaded",
].entries()) {
  const left = (20 + index * 70) * 20;
  const bottom = 50 * 20;
  const outline = defaultPaths.find(
    (entry) => Math.abs(entry.numbers[0] - left) <= 0.01
      && entry.attributes.includes('stroke="#000000"')
      && entry.attributes.includes('fill="none"')
      && entry.d.includes(" C "),
  );
  if (!outline) {
    throw new Error(`${name}: outer rounded path not found`);
  }
  const measured = measureRoundedPath(outline, left, bottom);
  assertClose(measured.radiusX, 3.6, `${name} rx`);
  assertClose(measured.radiusY, 3.6, `${name} ry`);
  report.defaults.push({ name, ...measured });
}

const reportPath = path.join(outDir, "corner-radius-probe.json");
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");

for (const input of inputs) {
  console.log(input);
}
for (const outputName of outputNames) {
  console.log(path.join(oracleDir, `${outputName}.chemdraw.svg`));
}
console.log(reportPath);
