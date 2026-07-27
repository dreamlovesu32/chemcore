import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.resolve(root, process.argv[2] ?? "tmp/chemdraw-bioshape-geometry-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

const TYPES = Object.freeze([
  "1SubstrateEnzyme", "2SubstrateEnzyme", "Receptor",
  "GProteinAlpha", "GProteinBeta", "GProteinGamma",
  "Immunoglobin", "IonChannel", "EndoplasmicReticulum", "Golgi",
  "MembraneLine", "MembraneArc", "MembraneEllipse", "MembraneMicelle",
  "DNA", "HelixProtein", "Mitochondrion", "Cloud", "tRNA",
  "RibosomeA", "RibosomeB",
]);

const PARAMETERS = Object.freeze({
  "1SubstrateEnzyme": { EnzymeReceptorSize: [12, 42] },
  Receptor: { NeckWidth: [12, 42] },
  GProteinGamma: {
    GproteinLowerHeight: [12, 42],
    GproteinUpperHeight: [12, 42],
  },
  Immunoglobin: {
    ImmunoglobinHeight: [14, 42],
    ImmunoglobinWidth: [14, 42],
  },
  Golgi: {
    GolgiHeight: [12, 38],
    GolgiLength: [16, 48],
    GolgiWidth: [2, 8],
  },
  MembraneLine: { MembraneElementSize: [2.5, 7.5] },
  MembraneArc: {
    MembraneElementSize: [2.5, 7.5],
    MembraneStartAngle: [-150, -45],
    MembraneEndAngle: [-15, 75],
    MembraneMajorAxisSize: [18, 42],
    MembraneMinorAxisSize: [18, 42],
  },
  MembraneEllipse: { MembraneElementSize: [2.5, 7.5] },
  MembraneMicelle: { MembraneElementSize: [2.5, 7.5] },
  DNA: {
    DNAWaveHeight: [7, 21],
    DNAWaveLength: [10, 30],
    DNAWaveOffset: [1.5, 6],
    DNAWaveWidth: [1, 5],
  },
  HelixProtein: {
    CylinderDistance: [1, 5],
    CylinderHeight: [8, 22],
    CylinderWidth: [2, 8],
    HelixProteinExtra: [1, 7],
    PipeWidth: [0.35, 1.8],
  },
});

const BASE_PARAMETERS = Object.freeze({
  EnzymeReceptorSize: 24,
  NeckWidth: 23,
  GproteinLowerHeight: 23,
  GproteinUpperHeight: 24,
  ImmunoglobinHeight: 25,
  ImmunoglobinWidth: 26,
  GolgiHeight: 21,
  GolgiLength: 31,
  GolgiWidth: 4,
  MembraneElementSize: 4.1,
  MembraneStartAngle: -85,
  MembraneEndAngle: 5,
  MembraneMajorAxisSize: 27,
  MembraneMinorAxisSize: 28,
  DNAWaveHeight: 12,
  DNAWaveLength: 18,
  DNAWaveOffset: 3,
  DNAWaveWidth: 2,
  CylinderDistance: 2.1,
  CylinderHeight: 13,
  CylinderWidth: 4.1,
  HelixProteinExtra: 3.1,
  PipeWidth: 0.7,
});

function stem(value) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/[^a-z0-9]+/gi, "-")
    .replace(/^-|-$/g, "")
    .toLowerCase();
}

function documentXml(testCase) {
  const attributes = Object.entries(testCase.parameters)
    .map(([key, value]) => `      ${key}="${value}"`)
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema BioShape geometry probe"
  BoundingBox="0 0 260 220" FractionalWidths="yes"
  HashSpacing="2.5" MarginWidth="1.6" LineWidth="0.6" BoldWidth="2"
  BondLength="14.4" BondSpacing="18" ChainAngle="120">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 260 220">
    <bioshape id="10"
      BioShapeType="${testCase.type}"
      xyz="${testCase.center.join(" ")}"
      MajorAxisEnd3D="${testCase.major.join(" ")}"
      MinorAxisEnd3D="${testCase.minor.join(" ")}"
      FillType="None" LineType="Solid"
      LineWidth="0.6" BoldWidth="2" color="3"
${attributes}/>
  </page>
</CDXML>
`;
}

function baseCase(type, variant, axes = {}) {
  return {
    type,
    variant,
    center: axes.center ?? [130, 110, 0],
    major: axes.major ?? [190, 110, 0],
    minor: axes.minor ?? [130, 150, 0],
    parameters: { ...BASE_PARAMETERS },
  };
}

const cases = [];
for (const type of TYPES) {
  cases.push(baseCase(type, "base"));
  cases.push(baseCase(type, "wide", { major: [220, 110, 0] }));
  cases.push(baseCase(type, "tall", { minor: [130, 180, 0] }));
  cases.push(baseCase(type, "rotated", {
    major: [172.4264, 152.4264, 0],
    minor: [101.7157, 138.2843, 0],
  }));
  for (const [parameter, values] of Object.entries(PARAMETERS[type] ?? {})) {
    for (const [level, value] of [["low", values[0]], ["high", values[1]]]) {
      const testCase = baseCase(type, `${stem(parameter)}-${level}`);
      testCase.parameters[parameter] = value;
      cases.push(testCase);
    }
  }
}

await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });
const jobs = [];
for (const testCase of cases) {
  const name = `${stem(testCase.type)}--${testCase.variant}`;
  const input = path.join(inputDir, `${name}.cdxml`);
  await fs.writeFile(input, documentXml(testCase), "utf8");
  jobs.push({ ...testCase, name, input });
}

await generateChemDrawOracle({
  inputs: jobs.map((job) => job.input),
  outputNames: jobs.map((job) => job.name),
  outDir: oracleDir,
  formats: ["cdxml", "svg"],
});

const manifest = [];
for (const job of jobs) {
  const cdxmlPath = path.join(oracleDir, `${job.name}.chemdraw.cdxml`);
  const svgPath = path.join(oracleDir, `${job.name}.chemdraw.svg`);
  const cdxml = await fs.readFile(cdxmlPath, "utf8");
  manifest.push({
    type: job.type,
    variant: job.variant,
    axes: { center: job.center, major: job.major, minor: job.minor },
    requestedParameters: job.parameters,
    normalizedTag: cdxml.match(/<bioshape\b[\s\S]*?\/>/i)?.[0] ?? null,
    input: path.relative(root, job.input),
    cdxml: path.relative(root, cdxmlPath),
    svg: path.relative(root, svgPath),
  });
}
await fs.writeFile(
  path.join(outDir, "manifest.json"),
  `${JSON.stringify({
    schema: "chemsema.chemdraw-bioshape-geometry-probe.v1",
    cases: manifest,
  }, null, 2)}\n`,
);
console.log(`[BIOSHAPE GEOMETRY] ${manifest.length} cases written to ${outDir}`);
