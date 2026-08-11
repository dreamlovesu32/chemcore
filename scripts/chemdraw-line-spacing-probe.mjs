import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";
import { inspectEmf } from "./emf-inspect.mjs";

const XML_ESCAPE = new Map([
  ["&", "&amp;"],
  ["<", "&lt;"],
  [">", "&gt;"],
  ['"', "&quot;"],
]);

function escapeXml(value) {
  return String(value).replace(/[&<>"]/g, (character) => XML_ESCAPE.get(character));
}

function parseArgs(argv) {
  const args = { outDir: "tmp/chemdraw-line-spacing-probe" };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--out") args.outDir = argv[++index];
    else if (argv[index] === "--attached-only") args.attachedOnly = true;
    else if (argv[index] === "--help" || argv[index] === "-h") args.help = true;
  }
  return args;
}

function probeCases() {
  const cases = [];
  const add = (values) => cases.push({ id: cases.length + 1, ...values });
  const faces = [
    ["regular", 0],
    ["bold", 1],
    ["italic", 2],
    ["bold-italic", 3],
    ["subscript", 32],
    ["superscript", 64],
    ["chemical", 96],
  ];

  for (const [faceName, face] of faces) {
    add({ role: "caption", font: 3, fontName: "Arial", size: 10, faceName, face, lineHeight: "auto" });
  }
  for (const [faceName, face] of faces) {
    add({ role: "label", font: 3, fontName: "Arial", size: 10, faceName, face, lineHeight: "auto" });
  }
  for (const font of [
    [3, "Arial"],
    [4, "Times New Roman"],
    [5, "Calibri"],
    [6, "Helvetica"],
  ]) {
    for (const size of [8, 10, 12, 18]) {
      add({ role: "caption", font: font[0], fontName: font[1], size, faceName: "regular", face: 0, lineHeight: "auto" });
      add({ role: "label", font: font[0], fontName: font[1], size, faceName: "regular", face: 0, lineHeight: "auto" });
    }
  }
  for (const lineHeight of ["variable", "8", "11.75", "14", "20"]) {
    add({ role: "caption", font: 3, fontName: "Arial", size: 10, faceName: "regular", face: 0, lineHeight });
    add({ role: "label", font: 3, fontName: "Arial", size: 10, faceName: "regular", face: 0, lineHeight });
  }
  return cases;
}

function styledText(probe) {
  const first = `A${String(probe.id).padStart(3, "0")}`;
  const second = `B${String(probe.id).padStart(3, "0")}`;
  const face = probe.face ? ` face="${probe.face}"` : "";
  return {
    first,
    second,
    xml: `<s font="${probe.font}" size="${probe.size}" color="0"${face}>${first}\n${second}</s>`,
  };
}

function documentXml(cases) {
  const rows = cases.map((probe, index) => {
    const x = probe.role === "caption" ? 72 : 360;
    const y = 50 + index * 42;
    const { xml } = styledText(probe);
    const specific = probe.role === "caption" ? "CaptionLineHeight" : "LabelLineHeight";
    const text = `<t p="${x} ${y}" BoundingBox="${x} ${y - probe.size} ${x + 90} ${y + 30}" ${specific}="${probe.lineHeight}" LineHeight="${probe.lineHeight}" LineStarts="5 9">${xml}</t>`;
    if (probe.role === "caption") return text;
    return `<fragment id="${1000 + probe.id}"><n id="${2000 + probe.id}" p="${x} ${y}" NodeType="GenericNickname" NeedsClean="yes">${text}</n></fragment>`;
  });
  return `<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema line-spacing probe" BoundingBox="0 0 700 ${cases.length * 42 + 100}" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10" LabelLineHeight="auto" CaptionLineHeight="auto" LineHeight="auto" LabelJustification="Auto" CaptionJustification="Left" BondLength="14.4" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" MarginWidth="1.6" color="0" bgcolor="1">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable>
<font id="3" charset="iso-8859-1" name="Arial"/>
<font id="4" charset="iso-8859-1" name="Times New Roman"/>
<font id="5" charset="iso-8859-1" name="Calibri"/>
<font id="6" charset="iso-8859-1" name="Helvetica"/>
</fonttable>
<page id="1" BoundingBox="0 0 700 ${cases.length * 42 + 100}" HeaderPosition="36" FooterPosition="36" HeightPages="1" WidthPages="1">
${rows.join("\n")}
</page></CDXML>`;
}

function detailCases() {
  const cases = [];
  const add = (name, values) => cases.push({ name, ...values });
  const line = (text, font = 3, size = 10, face = 0) => ({ text, font, size, face });
  for (const role of ["caption", "label"]) {
    for (const mode of ["auto", "variable"]) {
      for (const [font, fontName] of [[3, "Arial"], [4, "Times New Roman"], [5, "Calibri"]]) {
        add(`${role}-${mode}-${fontName.replaceAll(" ", "-").toLowerCase()}-plain`, {
          role, mode, fontName, lines: [line("A", font), line("g", font), line("Q", font)],
        });
      }
      add(`${role}-${mode}-arial-mixed-face`, {
        role, mode, fontName: "Arial", lines: [line("A"), line("g", 3, 10, 64), line("Q", 3, 10, 32)],
      });
      add(`${role}-${mode}-arial-mixed-size`, {
        role, mode, fontName: "Arial", lines: [line("A", 3, 8), line("g", 3, 18), line("Q", 3, 12)],
      });
      add(`${role}-${mode}-arial-bold-italic`, {
        role, mode, fontName: "Arial", lines: [line("A", 3, 10, 1), line("g", 3, 10, 2), line("Q", 3, 10, 3)],
      });
    }
  }
  for (const sequence of [
    ["A", "A", "A"],
    ["g", "g", "g"],
    ["Q", "Q", "Q"],
    ["A", "Q", "A"],
    ["g", "A", "g"],
    ["H", "p", "H"],
    ["[", "]", "["],
  ]) {
    add(`caption-variable-arial-sequence-${cases.length}`, {
      role: "caption",
      mode: "variable",
      fontName: "Arial",
      lines: sequence.map((text) => line(text)),
    });
  }
  for (const [font, fontName] of [[3, "Arial"], [4, "Times New Roman"], [5, "Calibri"]]) {
    for (const text of ["A", "g", "Q"]) {
      add(`caption-auto-${fontName.replaceAll(" ", "-").toLowerCase()}-${text.codePointAt(0)}`, {
        role: "caption",
        mode: "auto",
        fontName,
        lines: [line(text, font), line(text, font), line(text, font)],
      });
    }
  }
  return cases;
}

function detailDocumentXml(probe) {
  const x = 72;
  const y = 80;
  let textLength = 0;
  const lineStarts = [];
  const styled = probe.lines.map((entry, index) => {
    const face = entry.face ? ` face="${entry.face}"` : "";
    const newline = index + 1 < probe.lines.length ? "\n" : "";
    textLength += entry.text.length + newline.length;
    lineStarts.push(textLength);
    return `<s font="${entry.font}" size="${entry.size}" color="0"${face}>${entry.text}${newline}</s>`;
  }).join("");
  const specific = probe.role === "caption" ? "CaptionLineHeight" : "LabelLineHeight";
  const text = `<t p="${x} ${y}" BoundingBox="${x} 40 180 140" ${specific}="${probe.mode}" LineHeight="${probe.mode}" LineStarts="${lineStarts.join(" ")}">${styled}</t>`;
  const body = probe.role === "caption"
    ? text
    : `<fragment id="10"><n id="11" p="${x} ${y}" NodeType="GenericNickname" NeedsClean="yes">${text}</n></fragment>`;
  return `<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema line-spacing detail probe" BoundingBox="0 0 240 180" LabelFont="3" LabelSize="10" CaptionFont="3" CaptionSize="10" LabelJustification="Auto" CaptionJustification="Left" BondLength="14.4" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" MarginWidth="1.6" color="0" bgcolor="1">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable><font id="3" charset="iso-8859-1" name="Arial"/><font id="4" charset="iso-8859-1" name="Times New Roman"/><font id="5" charset="iso-8859-1" name="Calibri"/></fonttable>
<page id="1" BoundingBox="0 0 240 180" HeaderPosition="36" FooterPosition="36" HeightPages="1" WidthPages="1">${body}</page>
</CDXML>`;
}

function svgTexts(svg) {
  const texts = [];
  const pattern = /<text\b([^>]*)>([\s\S]*?)<\/text>/g;
  for (const match of svg.matchAll(pattern)) {
    const attributes = match[1];
    const text = match[2].replace(/<[^>]+>/g, "").trim();
    const transform = /transform="matrix\(([^)]+)\)"/.exec(attributes)?.[1]
      .trim()
      .split(/[ ,]+/)
      .map(Number);
    const fontSize = Number(/font-size="([^"]+)px"/.exec(attributes)?.[1]);
    if (transform?.length === 6) texts.push({ text, x: transform[4], y: transform[5], scale: transform[0], fontSize });
  }
  return texts;
}

async function runDetailProbe(outDir) {
  const cases = detailCases();
  const inputDir = path.join(outDir, "detail-inputs");
  const oracleDir = path.join(outDir, "detail-oracle");
  await fs.mkdir(inputDir, { recursive: true });
  const inputs = [];
  for (const probe of cases) {
    const input = path.join(inputDir, `${probe.name}.cdxml`);
    await fs.writeFile(input, detailDocumentXml(probe), "utf8");
    inputs.push(input);
  }
  await generateChemDrawOracle({ inputs, outDir: oracleDir, formats: ["cdxml", "svg", "emf"] });
  const measurements = [];
  for (const probe of cases) {
    const svgPath = path.join(oracleDir, `${probe.name}.chemdraw.svg`);
    const emfPath = path.join(oracleDir, `${probe.name}.chemdraw.emf`);
    const svgRecords = svgTexts(await fs.readFile(svgPath, "utf8"));
    const emf = await inspectEmf(emfPath, { includeRecords: true });
    const emfRecords = emf.records
      .filter((record) => record.name === "EMR_EXTTEXTOUTW" || record.name === "EMR_EXTTEXTOUTA")
      .map((record) => ({ text: record.text?.text, reference: record.text?.reference }));
    const expectedTexts = probe.lines.map((line) => line.text);
    const consumeExpected = (records) => {
      let cursor = 0;
      return expectedTexts.map((expected) => {
        const found = records.slice(cursor).findIndex((record) => record.text === expected);
        if (found < 0) return undefined;
        cursor += found + 1;
        return records[cursor - 1];
      });
    };
    const svgLines = consumeExpected(svgRecords);
    const emfLines = consumeExpected(emfRecords);
    const firstLine = svgLines[0];
    const pixelsPerPoint = firstLine
      ? firstLine.scale * firstLine.fontSize / probe.lines[0].size
      : null;
    const advances = [];
    for (let index = 1; index < probe.lines.length; index += 1) {
      const svgAdvancePx = svgLines[index] && svgLines[index - 1]
        ? svgLines[index].y - svgLines[index - 1].y
        : null;
      const emfAdvancePx = emfLines[index]?.reference && emfLines[index - 1]?.reference
        ? emfLines[index].reference.y - emfLines[index - 1].reference.y
        : null;
      advances.push({
        afterLine: index,
        svgPixels: svgAdvancePx == null ? null : Number(svgAdvancePx.toFixed(4)),
        svgPoints: svgAdvancePx == null || !pixelsPerPoint ? null : Number((svgAdvancePx / pixelsPerPoint).toFixed(4)),
        emfPixels: emfAdvancePx,
        emfPoints: emfAdvancePx == null || !pixelsPerPoint ? null : Number((emfAdvancePx / pixelsPerPoint).toFixed(4)),
      });
    }
    measurements.push({ ...probe, pixelsPerPoint, advances });
  }
  const reportPath = path.join(outDir, "detail-report.json");
  await fs.writeFile(reportPath, `${JSON.stringify({ generatedAt: new Date().toISOString(), measurements }, null, 2)}\n`, "utf8");
  return reportPath;
}

function attachedAtomCases() {
  const cases = [];
  const add = (values) => cases.push({ name: `atom-${cases.length + 1}`, ...values });
  for (const alignment of ["Above", "Below"]) {
    for (const size of [7, 10, 14, 18]) {
      for (const lineHeight of [undefined, "variable", "auto", "8", "12", "18"]) {
        add({
          alignment,
          size,
          lineHeight,
          font: 3,
          fontName: "Arial",
          element: 7,
          symbol: "N",
          text: "NH",
          marginWidth: 1.6,
        });
      }
    }
  }
  for (const [font, fontName] of [[3, "Arial"], [4, "Times New Roman"], [5, "Calibri"]]) {
    add({ alignment: "Above", size: 10, lineHeight: "12", font, fontName, element: 7, symbol: "N", text: "NH", marginWidth: 1.6 });
    add({ alignment: "Below", size: 10, lineHeight: "12", font, fontName, element: 7, symbol: "N", text: "NH", marginWidth: 1.6 });
  }
  for (const marginWidth of [0.8, 1.6, 2, 3.2]) {
    for (const lineHeight of [undefined, "variable"]) {
      add({ alignment: "Above", size: 10, lineHeight, font: 3, fontName: "Arial", element: 7, symbol: "N", text: "NH", marginWidth });
    }
  }
  for (const [element, symbol, text] of [
    [8, "O", "OH"],
    [15, "P", "PH"],
    [16, "S", "SH"],
  ]) {
    add({ alignment: "Above", size: 10, lineHeight: "variable", font: 3, fontName: "Arial", element, symbol, text, marginWidth: 1.6 });
  }
  return cases;
}

function attachedAtomDocumentXml(probe) {
  const lineHeight = probe.lineHeight == null ? "" : ` LabelLineHeight="${probe.lineHeight}"`;
  return `<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema attached atom line-spacing probe" BoundingBox="0 0 180 180" LabelFont="3" LabelSize="10" LabelJustification="Auto" BondLength="14.4" LineWidth="0.6" BoldWidth="2" HashSpacing="2.5" MarginWidth="${probe.marginWidth}" color="0" bgcolor="1">
<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
<fonttable><font id="3" charset="iso-8859-1" name="Arial"/><font id="4" charset="iso-8859-1" name="Times New Roman"/><font id="5" charset="iso-8859-1" name="Calibri"/></fonttable>
<page id="1" BoundingBox="0 0 180 180" HeaderPosition="36" FooterPosition="36" HeightPages="1" WidthPages="1">
<fragment id="10"><n id="11" p="90 90" Element="${probe.element}" NumHydrogens="1" NeedsClean="yes" AS="N"><t id="12" p="86.42 93.9" BoundingBox="86.42 73.7 93.62 97.7" InterpretChemically="yes" LabelAlignment="${probe.alignment}" LabelJustification="Left"${lineHeight} LineStarts="2 3"><s font="${probe.font}" size="${probe.size}" color="0" face="96">${probe.text}</s></t></n><n id="13" p="72 100" AS="N"/><n id="14" p="108 100" AS="N"/><b id="15" B="13" E="11"/><b id="16" B="11" E="14"/></fragment>
</page></CDXML>`;
}

async function runAttachedAtomProbe(outDir) {
  const cases = attachedAtomCases();
  const inputDir = path.join(outDir, "attached-atom-inputs");
  const oracleDir = path.join(outDir, "attached-atom-oracle");
  await fs.mkdir(inputDir, { recursive: true });
  const inputs = [];
  for (const probe of cases) {
    const input = path.join(inputDir, `${probe.name}.cdxml`);
    await fs.writeFile(input, attachedAtomDocumentXml(probe), "utf8");
    inputs.push(input);
  }
  await generateChemDrawOracle({ inputs, outDir: oracleDir, formats: ["cdxml", "svg", "emf"] });
  const measurements = [];
  for (const probe of cases) {
    const svgPath = path.join(oracleDir, `${probe.name}.chemdraw.svg`);
    const emfPath = path.join(oracleDir, `${probe.name}.chemdraw.emf`);
    const savedPath = path.join(oracleDir, `${probe.name}.chemdraw.cdxml`);
    const svgRecords = svgTexts(await fs.readFile(svgPath, "utf8"));
    const emf = await inspectEmf(emfPath, { includeRecords: true });
    const emfRecords = emf.records
      .filter((record) => record.name === "EMR_EXTTEXTOUTW" || record.name === "EMR_EXTTEXTOUTA")
      .map((record) => ({ text: record.text?.text, reference: record.text?.reference }));
    const svgH = svgRecords.find((record) => record.text === "H");
    const svgElement = svgRecords.find((record) => record.text === probe.symbol);
    const emfH = emfRecords.find((record) => record.text === "H");
    const emfElement = emfRecords.find((record) => record.text === probe.symbol);
    const pixelsPerPoint = svgElement ? svgElement.scale * svgElement.fontSize / probe.size : null;
    const savedCdxml = await fs.readFile(savedPath, "utf8");
    measurements.push({
      ...probe,
      svgTexts: svgRecords,
      svgAdvancePoints: svgH && svgElement && pixelsPerPoint
        ? Number(((svgElement.y - svgH.y) / pixelsPerPoint).toFixed(4))
        : null,
      emfAdvance: emfH?.reference && emfElement?.reference
        ? Number((emfElement.reference.y - emfH.reference.y).toFixed(4))
        : null,
      savedLabelLineHeight: /LabelLineHeight="([^"]+)"/.exec(savedCdxml)?.[1] ?? null,
    });
  }
  const reportPath = path.join(outDir, "attached-atom-report.json");
  await fs.writeFile(reportPath, `${JSON.stringify({ generatedAt: new Date().toISOString(), measurements }, null, 2)}\n`, "utf8");
  return reportPath;
}

function pairSpacing(records, first, second, pickPoint) {
  const a = records.find((record) => record.text === first);
  const b = records.find((record) => record.text === second);
  if (!a || !b) return null;
  const firstPoint = pickPoint(a);
  const secondPoint = pickPoint(b);
  if (!firstPoint || !secondPoint) return null;
  return Number((secondPoint.y - firstPoint.y).toFixed(4));
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/chemdraw-line-spacing-probe.mjs [--out dir]");
    return;
  }
  const outDir = path.resolve(args.outDir);
  await fs.mkdir(outDir, { recursive: true });
  if (args.attachedOnly) {
    console.log(await runAttachedAtomProbe(outDir));
    return;
  }
  const cases = probeCases();
  const input = path.join(outDir, "line-spacing-probe.cdxml");
  await fs.writeFile(input, documentXml(cases), "utf8");
  await generateChemDrawOracle({ inputs: [input], outDir, formats: ["cdxml", "svg", "emf"] });

  const svgPath = path.join(outDir, "line-spacing-probe.chemdraw.svg");
  const emfPath = path.join(outDir, "line-spacing-probe.chemdraw.emf");
  const savedCdxmlPath = path.join(outDir, "line-spacing-probe.chemdraw.cdxml");
  const svgRecords = svgTexts(await fs.readFile(svgPath, "utf8"));
  const emf = await inspectEmf(emfPath, { includeRecords: true });
  const emfRecords = emf.records
    .filter((record) => record.name === "EMR_EXTTEXTOUTW" || record.name === "EMR_EXTTEXTOUTA")
    .map((record) => ({ text: record.text?.text, reference: record.text?.reference }));
  const savedCdxml = await fs.readFile(savedCdxmlPath, "utf8");

  const measurements = cases.map((probe) => {
    const { first, second } = styledText(probe);
    const saved = new RegExp(`<s[^>]*>${escapeXml(first)}\\s*${escapeXml(second)}<\\/s>`).test(savedCdxml);
    return {
      ...probe,
      first,
      second,
      saved,
      svgBaselineAdvance: pairSpacing(svgRecords, first, second, (record) => record),
      emfReferenceAdvance: pairSpacing(emfRecords, first, second, (record) => record.reference),
    };
  });
  const report = {
    generatedAt: new Date().toISOString(),
    files: { input, savedCdxmlPath, svgPath, emfPath },
    measurements,
    unmatchedSvgText: svgRecords.filter((record) => /^[AB]\d{3}$/.test(record.text)).length,
    unmatchedEmfText: emfRecords.filter((record) => /^[AB]\d{3}$/.test(record.text)).length,
  };
  const reportPath = path.join(outDir, "report.json");
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  const detailReportPath = await runDetailProbe(outDir);
  const attachedAtomReportPath = await runAttachedAtomProbe(outDir);
  console.log(reportPath);
  console.log(detailReportPath);
  console.log(attachedAtomReportPath);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exit(1);
  });
}
