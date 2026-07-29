import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function parseNumberList(value, fallback) {
  if (!value) return fallback;
  const numbers = value
    .split(",")
    .map(Number)
    .filter(Number.isFinite);
  if (!numbers.length) throw new Error(`Expected a number list, got ${value}`);
  return numbers;
}

function numericRange(start, end, step) {
  const values = [];
  for (let value = start; value <= end + step * 0.25; value += step) {
    values.push(Number(value.toFixed(6)));
  }
  return values;
}

function parseArgs(argv) {
  const options = {
    profile: "coarse",
    outDir: path.join(root, "tmp", "chemdraw-label-flow-sector"),
    angles: null,
    lengths: null,
    sizes: null,
    rotations: null,
    noExport: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--profile") options.profile = argv[++index];
    else if (argument === "--out") options.outDir = path.resolve(argv[++index]);
    else if (argument === "--angles") options.angles = argv[++index];
    else if (argument === "--lengths") options.lengths = argv[++index];
    else if (argument === "--sizes") options.sizes = argv[++index];
    else if (argument === "--rotations") options.rotations = argv[++index];
    else if (argument === "--no-export") options.noExport = true;
    else if (argument === "--help" || argument === "-h") options.help = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }
  return options;
}

function profileMatrix(options) {
  const profiles = {
    coarse: {
      angles: numericRange(0, 20, 1),
      lengths: [10, 14.4, 24],
      sizes: [10],
      rotations: [0],
    },
    fine: {
      angles: numericRange(13, 17, 0.1),
      lengths: [14.4],
      sizes: [10],
      rotations: [0],
    },
    verify: {
      angles: [14.8, 14.9, 15, 15.1, 15.2],
      lengths: [10, 14.4, 24],
      sizes: [8, 10, 14],
      rotations: [0],
    },
    symmetry: {
      angles: [14.9, 15, 15.1],
      lengths: [14.4],
      sizes: [10],
      rotations: [0, 90, 180, 270],
    },
  };
  const selected = profiles[options.profile];
  if (!selected) throw new Error(`Unknown profile: ${options.profile}`);
  return {
    angles: parseNumberList(options.angles, selected.angles),
    lengths: parseNumberList(options.lengths, selected.lengths),
    sizes: parseNumberList(options.sizes, selected.sizes),
    rotations: parseNumberList(options.rotations, selected.rotations),
  };
}

function safeNumber(value) {
  return value.toFixed(4).replace("-", "m").replace(".", "p");
}

function probeName(probe) {
  return `angle-${safeNumber(probe.angle)}_rotation-${safeNumber(probe.rotation)}_length-${safeNumber(probe.length)}_size-${safeNumber(probe.size)}`;
}

function probeCdxml(probe) {
  const radians = (probe.angle + probe.rotation) * Math.PI / 180;
  const right = [
    100 + probe.length * Math.cos(radians),
    100 + probe.length * Math.sin(radians),
  ];
  const fixedRadians = (120 + probe.rotation) * Math.PI / 180;
  const lowerLeft = [
    100 + probe.length * Math.cos(fixedRadians),
    100 + probe.length * Math.sin(fixedRadians),
  ];
  const baseline = 100 + probe.size * 0.358;
  const labelWidth = probe.size * 1.282;
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd">
<CDXML BondLength="${probe.length.toFixed(4)}" LineWidth="0.60" MarginWidth="1.60"
 LabelFont="3" LabelSize="${probe.size}" LabelFace="96" LabelJustification="Auto">
 <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
 <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
 <page id="1" BoundingBox="0 0 220 220">
  <fragment id="2">
   <n id="10" p="100 100" Element="7" NumHydrogens="1" AS="N">
    <t id="11" p="100 ${baseline.toFixed(4)}"
     BoundingBox="${(100 - labelWidth).toFixed(4)} ${(baseline - probe.size * 0.716).toFixed(4)} 100 ${baseline.toFixed(4)}"
     LabelJustification="Right" Justification="Right" LabelAlignment="Right">
     <s font="3" size="${probe.size}" color="0" face="96">NH</s>
    </t>
   </n>
   <n id="12" p="${lowerLeft[0].toFixed(4)} ${lowerLeft[1].toFixed(4)}" AS="N"/>
   <n id="13" p="${right[0].toFixed(4)} ${right[1].toFixed(4)}" AS="N"/>
   <b id="20" B="10" E="12"/><b id="21" B="10" E="13"/>
  </fragment>
 </page>
</CDXML>
`;
}

function decodeText(value) {
  return value
    .replace(/<[^>]+>/g, "")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&")
    .trim();
}

function svgTextEntries(svg) {
  return [...svg.matchAll(/<text\b([^>]*)>([\s\S]*?)<\/text>/g)].map((match) => {
    const transform = match[1].match(/transform="matrix\(([^)]+)\)"/)?.[1]
      .trim()
      .split(/[\s,]+/)
      .map(Number);
    return {
      text: decodeText(match[2]),
      x: transform?.[4] ?? null,
      y: transform?.[5] ?? null,
    };
  });
}

function classifySvg(svg) {
  const entries = svgTextEntries(svg);
  if (entries.some((entry) => entry.text === "HN")) return "reverse-horizontal";
  if (entries.some((entry) => entry.text === "NH")) return "forward-horizontal";
  const hydrogen = entries.find((entry) => entry.text === "H");
  const nitrogen = entries.find((entry) => entry.text === "N");
  if (
    hydrogen
    && nitrogen
    && Number.isFinite(hydrogen.y)
    && Number.isFinite(nitrogen.y)
  ) {
    return hydrogen.y < nitrogen.y ? "stack-above" : "stack-below";
  }
  return "unclassified";
}

function savedLabelAlignment(cdxml) {
  const node = cdxml.match(/<n\b[^>]*\bid="10"[\s\S]*?<\/n>/)?.[0];
  const text = node?.match(/<t\b([^>]*)>/)?.[1];
  return text?.match(/\bLabelAlignment="([^"]+)"/)?.[1] ?? null;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log("Usage: node scripts/chemdraw-label-flow-sector-probe.mjs [--profile coarse|fine|verify|symmetry] [--out dir] [--angles a,b] [--rotations a,b] [--lengths a,b] [--sizes a,b] [--no-export]");
    return;
  }
  const matrix = profileMatrix(options);
  const probes = matrix.sizes.flatMap((size) =>
    matrix.lengths.flatMap((length) =>
      matrix.rotations.flatMap((rotation) =>
        matrix.angles.map((angle) => ({ angle, rotation, length, size })))));
  const sourceDir = path.join(options.outDir, "sources");
  const oracleDir = path.join(options.outDir, "oracle");
  await fs.mkdir(sourceDir, { recursive: true });
  const inputs = [];
  for (const probe of probes) {
    const sourcePath = path.join(sourceDir, `${probeName(probe)}.cdxml`);
    await fs.writeFile(sourcePath, probeCdxml(probe), "utf8");
    inputs.push(sourcePath);
  }
  const jobs = options.noExport
    ? inputs.map((input) => {
      const stem = path.basename(input, path.extname(input));
      return {
        input,
        outputs: {
          svg: path.join(oracleDir, `${stem}.chemdraw.svg`),
          cdxml: path.join(oracleDir, `${stem}.chemdraw.cdxml`),
        },
      };
    })
    : await generateChemDrawOracle({
      outDir: oracleDir,
      formats: ["svg", "cdxml"],
      inputs,
    });
  const results = [];
  for (let index = 0; index < probes.length; index += 1) {
    const [svg, cdxml] = await Promise.all([
      fs.readFile(jobs[index].outputs.svg, "utf8"),
      fs.readFile(jobs[index].outputs.cdxml, "utf8"),
    ]);
    results.push({
      ...probes[index],
      layout: classifySvg(svg),
      savedLabelAlignment: savedLabelAlignment(cdxml),
    });
  }
  const report = {
    schema: "chemsema.chemdraw-label-flow-sector-probe.v1",
    profile: options.profile,
    matrix,
    results,
  };
  const reportPath = path.join(options.outDir, "report.json");
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  console.log(JSON.stringify({
    reportPath,
    counts: Object.fromEntries(
      [...new Set(results.map((result) => result.layout))]
        .map((layout) => [layout, results.filter((result) => result.layout === layout).length]),
    ),
  }, null, 2));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack ?? error.message : String(error));
    process.exit(1);
  });
}
