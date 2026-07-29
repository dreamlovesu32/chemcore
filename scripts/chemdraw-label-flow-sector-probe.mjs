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
    fixedAngles: null,
    pairOffset: null,
    label: "NH",
    nodeType: "element",
    face: 96,
    connectionCount: 2,
    tripleOffsets: null,
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
    else if (argument === "--fixed-angles") options.fixedAngles = argv[++index];
    else if (argument === "--pair-offset") options.pairOffset = Number(argv[++index]);
    else if (argument === "--label") options.label = argv[++index];
    else if (argument === "--node-type") options.nodeType = argv[++index];
    else if (argument === "--face") options.face = Number(argv[++index]);
    else if (argument === "--connection-count") options.connectionCount = Number(argv[++index]);
    else if (argument === "--triple-offsets") options.tripleOffsets = argv[++index];
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
      fixedAngles: [120],
      lengths: [10, 14.4, 24],
      sizes: [10],
      rotations: [0],
    },
    fine: {
      angles: numericRange(13, 17, 0.1),
      fixedAngles: [120],
      lengths: [14.4],
      sizes: [10],
      rotations: [0],
    },
    verify: {
      angles: [14.8, 14.9, 15, 15.1, 15.2],
      fixedAngles: [120],
      lengths: [10, 14.4, 24],
      sizes: [8, 10, 14],
      rotations: [0],
    },
    symmetry: {
      angles: [14.9, 15, 15.1],
      fixedAngles: [120],
      lengths: [14.4],
      sizes: [10],
      rotations: [0, 90, 180, 270],
    },
  };
  const selected = profiles[options.profile];
  if (!selected) throw new Error(`Unknown profile: ${options.profile}`);
  return {
    angles: parseNumberList(options.angles, selected.angles),
    fixedAngles: parseNumberList(options.fixedAngles, selected.fixedAngles),
    lengths: parseNumberList(options.lengths, selected.lengths),
    sizes: parseNumberList(options.sizes, selected.sizes),
    rotations: parseNumberList(options.rotations, selected.rotations),
  };
}

function safeNumber(value) {
  return value.toFixed(4).replace("-", "m").replace(".", "p");
}

function probeName(probe) {
  const third = probe.thirdAngle === undefined
    ? ""
    : `_third-${safeNumber(probe.thirdAngle)}`;
  const fourth = probe.fourthAngle === undefined
    ? ""
    : `_fourth-${safeNumber(probe.fourthAngle)}`;
  return `angle-${safeNumber(probe.angle)}_fixed-${safeNumber(probe.fixedAngle)}${third}${fourth}_rotation-${safeNumber(probe.rotation)}_length-${safeNumber(probe.length)}_size-${safeNumber(probe.size)}`;
}

function probeCdxml(probe, options) {
  const connectionAngles = [probe.angle, probe.fixedAngle];
  if (probe.thirdAngle !== undefined) connectionAngles.push(probe.thirdAngle);
  if (probe.fourthAngle !== undefined) connectionAngles.push(probe.fourthAngle);
  const endpoints = connectionAngles.map((angle) => {
    const radians = (angle + probe.rotation) * Math.PI / 180;
    return [
      100 + probe.length * Math.cos(radians),
      100 + probe.length * Math.sin(radians),
    ];
  });
  const endpointNodes = endpoints
    .map((point, index) =>
      `   <n id="${12 + index}" p="${point[0].toFixed(4)} ${point[1].toFixed(4)}" AS="N"/>`)
    .join("\n");
  const endpointBonds = endpoints
    .map((_, index) => `<b id="${20 + index}" B="10" E="${12 + index}"/>`)
    .join("");
  const baseline = 100 + probe.size * 0.358;
  const labelWidth = probe.size * Math.max(1.282, options.label.length * 0.65);
  const nodeAttributes = options.nodeType === "fragment"
    ? 'NodeType="Fragment"'
    : 'Element="7" NumHydrogens="1"';
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd">
<CDXML BondLength="${probe.length.toFixed(4)}" LineWidth="0.60" MarginWidth="1.60"
 LabelFont="3" LabelSize="${probe.size}" LabelFace="96" LabelJustification="Auto">
 <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
 <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
 <page id="1" BoundingBox="0 0 220 220">
  <fragment id="2">
   <n id="10" p="100 100" ${nodeAttributes} AS="N">
    <t id="11" p="100 ${baseline.toFixed(4)}"
     BoundingBox="${(100 - labelWidth).toFixed(4)} ${(baseline - probe.size * 0.716).toFixed(4)} 100 ${baseline.toFixed(4)}"
     LabelJustification="Right" Justification="Right" LabelAlignment="Right">
     <s font="3" size="${probe.size}" color="0" face="${options.face}">${options.label}</s>
    </t>
   </n>
${endpointNodes}
   ${endpointBonds}
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
    console.log("Usage: node scripts/chemdraw-label-flow-sector-probe.mjs [--profile coarse|fine|verify|symmetry] [--out dir] [--angles a,b] [--fixed-angles a,b] [--pair-offset degrees] [--connection-count 2|3|4] [--triple-offsets a,b] [--label text] [--node-type element|fragment] [--face value] [--rotations a,b] [--lengths a,b] [--sizes a,b] [--no-export]");
    return;
  }
  if (options.pairOffset !== null && !Number.isFinite(options.pairOffset)) {
    throw new Error("--pair-offset must be a finite number");
  }
  if (!["element", "fragment"].includes(options.nodeType)) {
    throw new Error("--node-type must be element or fragment");
  }
  if (!Number.isFinite(options.face)) {
    throw new Error("--face must be a finite number");
  }
  if (![2, 3, 4].includes(options.connectionCount)) {
    throw new Error("--connection-count must be 2, 3, or 4");
  }
  const matrix = profileMatrix(options);
  const tripleOffsets = options.tripleOffsets === null
    ? null
    : parseNumberList(options.tripleOffsets, []);
  if (tripleOffsets !== null && tripleOffsets.length !== 2) {
    throw new Error("--triple-offsets requires exactly two comma-separated offsets");
  }
  const angleTuples = options.connectionCount === 4
    ? matrix.angles.flatMap((angle, firstIndex) =>
      matrix.angles.slice(firstIndex + 1).flatMap((fixedAngle, secondOffset) => {
        const secondIndex = firstIndex + secondOffset + 1;
        return matrix.angles.slice(secondIndex + 1).flatMap((thirdAngle, thirdOffset) => {
          const thirdIndex = secondIndex + thirdOffset + 1;
          return matrix.angles.slice(thirdIndex + 1).map((fourthAngle) => ({
            angle,
            fixedAngle,
            thirdAngle,
            fourthAngle,
          }));
        });
      }))
    : options.connectionCount === 3
    ? tripleOffsets === null
      ? matrix.angles.flatMap((angle, firstIndex) =>
        matrix.angles.slice(firstIndex + 1).flatMap((fixedAngle, secondOffset) =>
          matrix.angles.slice(firstIndex + secondOffset + 2).map((thirdAngle) => ({
            angle,
            fixedAngle,
            thirdAngle,
          }))))
      : matrix.angles.map((angle) => ({
        angle,
        fixedAngle: angle + tripleOffsets[0],
        thirdAngle: angle + tripleOffsets[1],
      }))
    : options.pairOffset === null
      ? matrix.fixedAngles.flatMap((fixedAngle) =>
        matrix.angles.map((angle) => ({ angle, fixedAngle })))
      : matrix.angles.map((angle) => ({
        angle,
        fixedAngle: angle + options.pairOffset,
      }));
  const probes = matrix.sizes.flatMap((size) =>
    matrix.lengths.flatMap((length) =>
      matrix.rotations.flatMap((rotation) =>
        angleTuples.map((tuple) => ({
          ...tuple,
          rotation,
          length,
          size,
        })))));
  const sourceDir = path.join(options.outDir, "sources");
  const oracleDir = path.join(options.outDir, "oracle");
  await fs.mkdir(sourceDir, { recursive: true });
  const inputs = [];
  for (const probe of probes) {
    const sourcePath = path.join(sourceDir, `${probeName(probe)}.cdxml`);
    await fs.writeFile(sourcePath, probeCdxml(probe, options), "utf8");
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
    const textEntries = svgTextEntries(svg);
    results.push({
      ...probes[index],
      layout: classifySvg(svg),
      visibleText: textEntries.map((entry) => entry.text),
      savedLabelAlignment: savedLabelAlignment(cdxml),
    });
  }
  const report = {
    schema: "chemsema.chemdraw-label-flow-sector-probe.v2",
    profile: options.profile,
    pairOffset: options.pairOffset,
    connectionCount: options.connectionCount,
    tripleOffsets,
    label: options.label,
    nodeType: options.nodeType,
    face: options.face,
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
