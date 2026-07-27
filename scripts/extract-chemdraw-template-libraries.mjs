import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const DEFAULT_SOURCE = "C:/ProgramData/PerkinElmerInformatics/ChemOffice2021/ChemDraw/ChemDraw Items";
const DEFAULT_CONVERTED = "target/all-template-cdxml";
const DEFAULT_OUTPUT = "viewer/template-libraries";

function parseArgs(argv) {
  const options = {
    source: DEFAULT_SOURCE,
    converted: DEFAULT_CONVERTED,
    out: DEFAULT_OUTPUT,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--source") options.source = argv[++index];
    else if (argument === "--converted") options.converted = argv[++index];
    else if (argument === "--out") options.out = argv[++index];
    else if (argument === "--help" || argument === "-h") options.help = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }
  return options;
}

function slugify(value) {
  return value
    .normalize("NFKD")
    .replace(/[^\w]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
}

function sha256(value) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function xmlTagAt(xml, start) {
  let index = start + 1;
  let quote = null;
  for (; index < xml.length; index += 1) {
    const character = xml[index];
    if (quote) {
      if (character === quote) quote = null;
      continue;
    }
    if (character === `"` || character === `'`) {
      quote = character;
      continue;
    }
    if (character === ">") break;
  }
  if (index >= xml.length) throw new Error("Unterminated XML tag.");
  const source = xml.slice(start, index + 1);
  const closing = /^<\s*\//.test(source);
  const declaration = /^<\s*[!?]/.test(source);
  const selfClosing = /\/\s*>$/.test(source);
  const match = source.match(/^<\s*\/?\s*([^\s/>]+)/);
  return {
    start,
    end: index + 1,
    source,
    name: match?.[1] || "",
    closing,
    declaration,
    selfClosing,
  };
}

function directPageSlices(xml) {
  const stack = [];
  const pages = [];
  let rootStartTag = null;
  let rootEndTag = null;
  let currentPageStart = null;
  for (let cursor = 0; cursor < xml.length;) {
    const start = xml.indexOf("<", cursor);
    if (start < 0) break;
    const tag = xmlTagAt(xml, start);
    cursor = tag.end;
    if (tag.declaration || !tag.name) continue;
    if (tag.closing) {
      const name = stack.pop();
      if (name !== tag.name) {
        throw new Error(`Unbalanced XML: expected </${name}> and found </${tag.name}>.`);
      }
      if (tag.name === "page" && stack.length === 1 && currentPageStart !== null) {
        pages.push([currentPageStart, tag.end]);
        currentPageStart = null;
      }
      if (tag.name === "CDXML" && stack.length === 0) rootEndTag = tag.source;
      continue;
    }
    if (tag.name === "CDXML" && stack.length === 0) rootStartTag = tag.source;
    if (tag.name === "page" && stack.length === 1) currentPageStart = tag.start;
    if (!tag.selfClosing) stack.push(tag.name);
  }
  if (!rootStartTag || !rootEndTag) throw new Error("CDXML root tags were not found.");
  if (!pages.length) throw new Error("No direct ChemDraw template pages were found.");

  const rootOpenEnd = xml.indexOf(">", xml.indexOf("<CDXML")) + 1;
  const firstPageStart = pages[0][0];
  const globalPrefix = xml.slice(rootOpenEnd, firstPageStart);
  return pages.map(([start, end]) =>
    `${xml.slice(0, rootOpenEnd)}${globalPrefix}${xml.slice(start, end)}${rootEndTag}`,
  );
}

async function nativeDecode(sourcePath, outputPath) {
  const result = spawnSync(
    "cargo",
    ["run", "-q", "-p", "chemsema-engine", "--example", "cdx_to_cdxml", "--", sourcePath, outputPath],
    { cwd: path.resolve("."), encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `Native CDX decode failed for ${sourcePath}:\n${result.stdout || ""}\n${result.stderr || ""}`,
    );
  }
}

async function sourceCdxml(sourceDir, convertedDir, file, scratchDir) {
  const convertedPath = path.join(convertedDir, `${path.basename(file, ".ctp")}.chemdraw.cdxml`);
  try {
    const converted = await fs.readFile(convertedPath, "utf8");
    if (converted.includes("<CDXML") && converted.includes("<page")) return converted;
  } catch {
    // The native CDX decoder below is the explicit path for libraries that
    // ChemDraw's COM SaveAs cannot convert without crashing.
  }
  const rawPath = path.join(scratchDir, `${path.basename(file, ".ctp")}.native.cdxml`);
  await nativeDecode(path.join(sourceDir, file), rawPath);
  return fs.readFile(rawPath, "utf8");
}

export async function extractTemplateLibraries(options) {
  const sourceDir = path.resolve(options.source);
  const convertedDir = path.resolve(options.converted);
  const outputDir = path.resolve(options.out);
  const scratchDir = path.join(path.resolve("target"), "template-library-native-cdxml");
  await fs.mkdir(outputDir, { recursive: true });
  await fs.mkdir(scratchDir, { recursive: true });

  const sourceFiles = (await fs.readdir(sourceDir))
    .filter((file) => file.toLowerCase().endsWith(".ctp"))
    .sort((left, right) => left.localeCompare(right, "en"));
  const libraries = [];
  for (const file of sourceFiles) {
    const name = path.basename(file, ".ctp");
    const id = slugify(name);
    const cdxml = await sourceCdxml(sourceDir, convertedDir, file, scratchDir);
    const pages = directPageSlices(cdxml);
    const outputName = `${id}.cdxml`;
    await fs.writeFile(path.join(outputDir, outputName), cdxml, "utf8");
    libraries.push({
      id,
      name,
      path: `./template-libraries/${outputName}`,
      templateCount: pages.length,
      iconCdxml: pages[0],
      sha256: sha256(cdxml),
    });
    process.stdout.write(`[TEMPLATE] ${name}: ${pages.length}\n`);
  }

  const catalog = {
    schema: "chemsema.template-library-catalog.v1",
    source: {
      product: "ChemDraw Professional",
      version: "21.0",
      format: "ctp/cdx",
    },
    libraryCount: libraries.length,
    templateCount: libraries.reduce((sum, library) => sum + library.templateCount, 0),
    libraries,
  };
  await fs.writeFile(
    path.join(outputDir, "catalog.json"),
    `${JSON.stringify(catalog, null, 2)}\n`,
    "utf8",
  );
  return catalog;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log("Usage: node scripts/extract-chemdraw-template-libraries.mjs [--source dir] [--converted dir] [--out dir]");
    return;
  }
  const catalog = await extractTemplateLibraries(options);
  console.log(`[TEMPLATE] libraries=${catalog.libraryCount} templates=${catalog.templateCount}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack || error.message : String(error));
    process.exit(1);
  });
}
