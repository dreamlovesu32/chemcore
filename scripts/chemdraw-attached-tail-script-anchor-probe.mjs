import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { launchBrowser } from "./playwright-browser.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outDir = path.join(root, "tmp", "chemdraw-attached-tail-script-anchor-probe");
const sourceDir = path.join(outDir, "sources");
const oracleDir = path.join(outDir, "oracle");

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function document({
  alignment,
  runs,
  textPosition = [100, 102.73],
  boundingBox = null,
  justification = alignment,
}) {
  const neighborX = alignment === "Right" ? 114.4 : alignment === "Left" ? 85.6 : 100;
  const neighborY = alignment === "Center" ? 114.4 : 100;
  const boxAttribute = boundingBox ? ` BoundingBox="${boundingBox.join(" ")}"` : "";
  const runXml = runs.map((run) => (
    `<s font="4" size="${run.size ?? 7}" face="${run.face ?? 96}" color="0">${escapeXml(run.text)}</s>`
  )).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE CDXML SYSTEM "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd" >
<CDXML BondLength="14.4">
  <fonttable><font id="4" charset="0" name="Times New Roman"/></fonttable>
  <colortable><color r="0" g="0" b="0"/></colortable>
  <page>
    <fragment>
      <n id="1" p="100 100" NodeType="Unspecified" LabelDisplay="${alignment}" AS="N">
        <t id="3" p="${textPosition.join(" ")}"${boxAttribute} LabelAlignment="${alignment}" LabelJustification="${justification}" Justification="${justification}" InterpretChemically="yes">${runXml}</t>
      </n>
      <n id="2" p="${neighborX} ${neighborY}" AS="N"/>
      <b id="4" B="1" E="2"/>
    </fragment>
  </page>
</CDXML>
`;
}

function attributes(tag) {
  return Object.fromEntries(
    [...tag.matchAll(/([A-Za-z_][\w:.-]*)="([^"]*)"/g)]
      .map((match) => [match[1], match[2]]),
  );
}

function savedText(saved) {
  const tag = saved.match(/<t\b[^>]*>/)?.[0] ?? "";
  return {
    attributes: attributes(tag),
    runs: [...saved.matchAll(/<s\b([^>]*)>([\s\S]*?)<\/s>/g)].map((match) => ({
      ...attributes(match[1]),
      text: match[2],
    })),
  };
}

function svgTexts(svg) {
  return [...svg.matchAll(/<text\b([^>]*)>([\s\S]*?)<\/text>/g)].map((match) => ({
    ...attributes(match[1]),
    text: match[2].replace(/<[^>]+>/g, ""),
  }));
}

async function renderedTextBoxes(page, svg) {
  await page.setContent(svg);
  const texts = await page.locator("text").evaluateAll((elements) => elements.map((element) => {
    const box = element.getBBox();
    const rect = element.getBoundingClientRect();
    return {
      text: element.textContent.trim(),
      x: box.x,
      y: box.y,
      width: box.width,
      height: box.height,
      screenX: rect.x,
      screenY: rect.y,
      screenWidth: rect.width,
      screenHeight: rect.height,
    };
  }));
  const paths = await page.locator("path").evaluateAll((elements) => elements.map((element) => {
    const rect = element.getBoundingClientRect();
    return {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
    };
  }));
  return { texts, paths };
}

const bodies = [
  {
    name: "plain",
    runs: [{ text: "(Aax)", face: 96 }],
  },
  {
    name: "tail-subscript",
    runs: [{ text: "(Aax)", face: 96 }, { text: "n", face: 34 }],
  },
  {
    name: "tail-superscript",
    runs: [{ text: "(Aax)", face: 96 }, { text: "n", face: 66 }],
  },
  {
    name: "tail-small-normal",
    runs: [{ text: "(Aax)", face: 96 }, { text: "n", face: 2, size: 5.25 }],
  },
  {
    name: "internal-subscript",
    runs: [{ text: "F", face: 96 }, { text: "3", face: 32 }, { text: "C", face: 96 }],
  },
  {
    name: "leading-subscript",
    runs: [{ text: "3", face: 32 }, { text: "(Aax)", face: 96 }],
  },
  {
    name: "leading-superscript",
    runs: [{ text: "+", face: 64 }, { text: "(Aax)", face: 96 }],
  },
  {
    name: "two-leading-scripts",
    runs: [{ text: "+", face: 64 }, { text: "3", face: 32 }, { text: "(Aax)", face: 96 }],
  },
  {
    name: "two-tail-scripts",
    runs: [{ text: "(Aax)", face: 96 }, { text: "n", face: 34 }, { text: "+", face: 66 }],
  },
];
const variants = ["Left", "Center", "Right"].flatMap((alignment) => (
  bodies.map((body) => ({
    name: `${alignment.toLowerCase()}-${body.name}`,
    alignment,
    runs: body.runs,
  }))
));
variants.push(
  {
    name: "right-authored-box-tail-subscript",
    alignment: "Right",
    runs: bodies.find((body) => body.name === "tail-subscript").runs,
    textPosition: [101.31, 102.67],
    boundingBox: [82.36, 97.01, 101.31, 104.42],
    justification: "Right",
  },
  {
    name: "right-authored-box-tail-subscript-shifted",
    alignment: "Right",
    runs: bodies.find((body) => body.name === "tail-subscript").runs,
    textPosition: [102.31, 102.67],
    boundingBox: [83.36, 97.01, 102.31, 104.42],
    justification: "Right",
  },
  {
    name: "right-authored-box-left-justified",
    alignment: "Right",
    runs: bodies.find((body) => body.name === "tail-subscript").runs,
    textPosition: [82.36, 102.67],
    boundingBox: [82.36, 97.01, 101.31, 104.42],
    justification: "Left",
  },
);

await fs.mkdir(sourceDir, { recursive: true });
const inputs = [];
for (const variant of variants) {
  const input = path.join(sourceDir, `${variant.name}.cdxml`);
  await fs.writeFile(input, document(variant), "utf8");
  inputs.push(input);
}

const jobs = await generateChemDrawOracle({
  inputs,
  outDir: oracleDir,
  formats: ["svg", "cdxml"],
});

const browser = await launchBrowser();
const page = await browser.newPage();
const results = [];
try {
  for (let index = 0; index < jobs.length; index += 1) {
    const [saved, svg] = await Promise.all([
      fs.readFile(jobs[index].outputs.cdxml, "utf8"),
      fs.readFile(jobs[index].outputs.svg, "utf8"),
    ]);
    results.push({
      name: variants[index].name,
      requested: variants[index],
      saved: savedText(saved),
      svgTexts: svgTexts(svg),
      rendered: await renderedTextBoxes(page, svg),
    });
  }
} finally {
  await browser.close();
}

const reportPath = path.join(outDir, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify({ results }, null, 2)}\n`, "utf8");
console.log(JSON.stringify({ reportPath, count: results.length }, null, 2));
