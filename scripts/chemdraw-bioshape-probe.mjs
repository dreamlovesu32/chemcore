import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.resolve(root, process.argv[2] ?? "tmp/chemdraw-bioshape-probe");
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");

export const BIO_SHAPE_TYPES = Object.freeze([
  "1SubstrateEnzyme",
  "2SubstrateEnzyme",
  "Receptor",
  "GProteinAlpha",
  "GProteinBeta",
  "GProteinGamma",
  "Immunoglobin",
  "IonChannel",
  "EndoplasmicReticulum",
  "Golgi",
  "MembraneLine",
  "MembraneArc",
  "MembraneEllipse",
  "MembraneMicelle",
  "DNA",
  "HelixProtein",
  "Mitochondrion",
  "Cloud",
  "tRNA",
  "RibosomeA",
  "RibosomeB",
]);

function stem(type) {
  return type.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
}

function documentXml(type) {
  return `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema BioShape probe"
  BoundingBox="0 0 240 200"
  FractionalWidths="yes"
  InterpretChemically="yes"
  LabelFont="3"
  LabelSize="10"
  CaptionFont="3"
  CaptionSize="10"
  HashSpacing="2.5"
  MarginWidth="1.6"
  LineWidth="0.6"
  BoldWidth="2"
  BondLength="14.4"
  BondSpacing="18"
  ChainAngle="120">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="1" BoundingBox="0 0 240 200">
    <bioshape id="10"
      BioShapeType="${type}"
      xyz="120 100 0"
      MajorAxisEnd3D="180 100 0"
      MinorAxisEnd3D="120 140 0"
      FillType="Shaded"
      LineType="Solid"
      LineWidth="0.6"
      BoldWidth="2"
      CylinderDistance="2.1"
      CylinderHeight="13.1"
      CylinderWidth="4.1"
      DNAWaveHeight="12.1"
      DNAWaveLength="18.1"
      DNAWaveOffset="3.1"
      DNAWaveWidth="2.1"
      EnzymeHeight="22.1"
      EnzymeReceptorSize="24.1"
      EnzymeWidth="32.1"
      GolgiHeight="21.1"
      GolgiLength="31.1"
      GolgiWidth="4.1"
      GproteinLowerHeight="23.1"
      GproteinUpperHeight="24.1"
      HelixProteinExtra="3.1"
      ImmunoglobinHeight="25.1"
      ImmunoglobinWidth="26.1"
      MembraneElementSize="4.1"
      MembraneEndAngle="5"
      MembraneMajorAxisSize="27.1"
      MembraneMinorAxisSize="28.1"
      MembraneStartAngle="-85"
      NeckHeight="29.1"
      NeckWidth="23.1"
      PipeWidth="0.7"
      color="3"/>
  </page>
</CDXML>
`;
}

await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const jobs = [];
for (const type of BIO_SHAPE_TYPES) {
  const name = stem(type);
  const input = path.join(inputDir, `${name}.cdxml`);
  await fs.writeFile(input, documentXml(type), "utf8");
  jobs.push({ type, name, input });
}

await generateChemDrawOracle({
  inputs: jobs.map((job) => job.input),
  outputNames: jobs.map((job) => job.name),
  outDir: oracleDir,
  formats: ["cdxml", "cdx", "svg", "emf"],
});

const manifest = [];
for (const job of jobs) {
  const cdxml = await fs.readFile(path.join(oracleDir, `${job.name}.chemdraw.cdxml`), "utf8");
  const tag = cdxml.match(/<bioshape\b[\s\S]*?\/>/i)?.[0] ?? null;
  manifest.push({
    type: job.type,
    input: path.relative(root, job.input),
    cdxml: path.relative(root, path.join(oracleDir, `${job.name}.chemdraw.cdxml`)),
    cdx: path.relative(root, path.join(oracleDir, `${job.name}.chemdraw.cdx`)),
    svg: path.relative(root, path.join(oracleDir, `${job.name}.chemdraw.svg`)),
    emf: path.relative(root, path.join(oracleDir, `${job.name}.chemdraw.emf`)),
    normalizedTag: tag,
  });
}
await fs.writeFile(
  path.join(outDir, "manifest.json"),
  `${JSON.stringify({ schema: "chemsema.chemdraw-bioshape-probe.v1", cases: manifest }, null, 2)}\n`,
  "utf8",
);

console.log(`[BIOSHAPE] ${manifest.length} ChemDraw cases written to ${outDir}`);
