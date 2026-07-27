import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const DEFAULT_SOURCE = "C:/ProgramData/PerkinElmerInformatics/ChemOffice2021/ChemDraw/ChemDraw Items";
const DEFAULT_OUTPUT = "viewer/template-libraries";

function parseArgs(argv) {
  const options = {
    source: DEFAULT_SOURCE,
    out: DEFAULT_OUTPUT,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--source") options.source = argv[++index];
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
    if (tag.name === "page" && stack.length === 1) {
      if (tag.selfClosing) pages.push([tag.start, tag.end]);
      else currentPageStart = tag.start;
    }
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

async function sourceCdxml(sourceDir, file, scratchDir) {
  const rawPath = path.join(scratchDir, `${path.basename(file, ".ctp")}.native.cdxml`);
  await nativeDecode(path.join(sourceDir, file), rawPath);
  return fs.readFile(rawPath, "utf8");
}

function templateGridLayout(cdxml) {
  const match = cdxml.match(/<templategrid\b([^>]*)\/?>/i);
  if (!match) throw new Error("The native CTP decode omitted its required templategrid.");
  const attrs = Object.fromEntries(
    [...match[1].matchAll(/([A-Za-z][\w:-]*)\s*=\s*(["'])(.*?)\2/g)]
      .map((entry) => [entry[1], entry[3]]),
  );
  const rows = Number(attrs.NumRows);
  const columns = Number(attrs.NumColumns);
  const paneHeight = Number(attrs.PaneHeight);
  const extent = String(attrs.extent || "").trim().split(/\s+/).map(Number);
  if (!Number.isInteger(rows) || rows <= 0
      || !Number.isInteger(columns) || columns <= 0
      || !Number.isFinite(paneHeight) || paneHeight <= 0
      || extent.length !== 2 || extent.some((value) => !Number.isFinite(value) || value <= 0)) {
    throw new Error("The native CTP decode produced an invalid typed templategrid.");
  }
  return { rows, columns, paneHeight, extent, readingOrder: "row-major" };
}

function pageHasContent(pageCdxml) {
  const page = pageCdxml.match(/<page\b[^>]*(?:\/>|>([\s\S]*)<\/page>)/i);
  return Boolean(page?.[1]?.trim());
}

export async function extractTemplateLibraries(options) {
  const sourceDir = path.resolve(options.source);
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
    const cdxml = await sourceCdxml(sourceDir, file, scratchDir);
    const layout = templateGridLayout(cdxml);
    const slots = directPageSlices(cdxml);
    if (slots.length > layout.rows * layout.columns) {
      throw new Error(`${name} has ${slots.length} page slots but its grid holds ${layout.rows * layout.columns}.`);
    }
    const pages = slots.filter(pageHasContent);
    if (!pages.length) throw new Error(`${name} contains no non-empty templates.`);
    const outputName = `${id}.cdxml`;
    await fs.writeFile(path.join(outputDir, outputName), cdxml, "utf8");
    libraries.push({
      id,
      name,
      path: `./template-libraries/${outputName}`,
      templateCount: pages.length,
      iconCdxml: pages[0],
      layout: {
        ...layout,
        occupiedCells: pages.length,
        emptyCells: layout.rows * layout.columns - pages.length,
      },
      sha256: sha256(cdxml),
    });
    process.stdout.write(`[TEMPLATE] ${name}: ${pages.length}\n`);
  }

  const catalog = {
    schema: "chemsema.template-library-catalog.v2",
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
    console.log("Usage: node scripts/extract-chemdraw-template-libraries.mjs [--source dir] [--out dir]");
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
