import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outputRoot = path.resolve(
  process.argv[2] || path.join(repoRoot, "tmp", "chemdraw-collapsed-fragment-layout-probe"),
);
const sourceRoot = path.join(outputRoot, "sources");
const oracleRoot = path.join(outputRoot, "oracle");

function escapeXml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function wrapperFragment({ index, label, innerX, innerY, external = false }) {
  const fragmentId = 1000 + index * 10;
  const wrapperId = fragmentId + 1;
  const innerFragmentId = fragmentId + 2;
  const firstId = fragmentId + 3;
  const secondId = fragmentId + 4;
  const externalId = fragmentId + 5;
  const bonds = external
    ? `<n id="${externalId}" NodeType="ExternalConnectionPoint"/>
       <b id="${fragmentId + 6}" B="${firstId}" E="${secondId}"/>
       <b id="${fragmentId + 7}" B="${secondId}" E="${externalId}"/>`
    : `<b id="${fragmentId + 6}" B="${firstId}" E="${secondId}"/>`;
  return {
    wrapperId: String(wrapperId),
    xml: `<fragment id="${fragmentId}">
      <n id="${wrapperId}" NodeType="Fragment">
        <fragment id="${innerFragmentId}">
          <n id="${firstId}" p="${innerX} ${innerY}"/>
          <n id="${secondId}" p="${innerX + 14.4} ${innerY}"/>
          ${bonds}
        </fragment>
        <t><s font="3" size="10">${escapeXml(label)}</s></t>
      </n>
    </fragment>`,
  };
}

function scenario(name, labels, options = {}) {
  const bondLength = options.bondLength ?? 14.4;
  const wrappers = labels.map((label, index) => wrapperFragment({
    index,
    label,
    innerX: 100 + index * 73,
    innerY: 200 + index * 11,
    external: options.externalIndex === index,
  }));
  const positioned = options.positioned
    ? `<fragment id="900">
         <n id="901" p="${options.positioned[0]} ${options.positioned[1]}"/>
         <n id="902" p="${options.positioned[0] + 14.4} ${options.positioned[1]}"/>
         <b id="903" B="901" E="902"/>
       </fragment>`
    : "";
  const graphic = options.graphic
    ? `<graphic id="950" BoundingBox="${options.graphic.join(" ")}" GraphicType="Rectangle"/>`
    : "";
  const pageAttributes = options.pageAttributes
    ? ` ${options.pageAttributes}`
    : "";
  return {
    name,
    wrapperIds: wrappers.map((entry) => entry.wrapperId),
    source: `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="${bondLength}" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1"${pageAttributes}>
    ${positioned}
    ${graphic}
    ${wrappers.map((entry) => entry.xml).join("\n")}
  </page>
</CDXML>`,
  };
}

function anchoredWrapper(index, neighborX, neighborY) {
  const fragmentId = 1000 + index * 10;
  return {
    wrapperId: String(fragmentId + 1),
    xml: `<fragment id="${fragmentId}">
      <n id="${fragmentId + 1}" NodeType="Fragment">
        <fragment id="${fragmentId + 2}">
          <n id="${fragmentId + 3}" p="100 110"/>
          <n id="${fragmentId + 4}" p="112.47 102.8"/>
          <n id="${fragmentId + 5}" NodeType="ExternalConnectionPoint"/>
          <b id="${fragmentId + 6}" B="${fragmentId + 3}" E="${fragmentId + 4}"/>
          <b id="${fragmentId + 7}" B="${fragmentId + 4}" E="${fragmentId + 5}"/>
        </fragment>
        <t><s font="3" size="10">Anchor${index + 1}</s></t>
      </n>
      <n id="${fragmentId + 8}" p="${neighborX} ${neighborY}" NodeType="GenericNickname">
        <t p="${neighborX} ${neighborY}"><s font="3" size="10">M</s></t>
      </n>
      <b id="${fragmentId + 9}" B="${fragmentId + 8}" E="${fragmentId + 1}"/>
    </fragment>`,
  };
}

function anchoredScenario(anchorIndex, count = 6) {
  const ordinaryByIndex = new Map(
    Array.from({ length: count }, (_, index) => index)
      .filter((index) => index !== anchorIndex)
      .map((index) => [index, wrapperFragment({
        index,
        label: `A${index + 1}`,
        innerX: 100 + index * 73,
        innerY: 200 + index * 11,
      })]),
  );
  const anchored = anchoredWrapper(anchorIndex, 125, 110);
  const wrappers = Array.from({ length: count }, (_, index) => index)
    .map((index) => (index === anchorIndex ? anchored : ordinaryByIndex.get(index)));
  return {
    name: `anchored-count-${count}-slot-${anchorIndex + 1}`,
    wrapperIds: wrappers.map((entry) => entry.wrapperId),
    source: `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="14.4" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1">${wrappers.map((entry) => entry.xml).join("\n")}</page>
</CDXML>`,
  };
}

function multiAnchoredScenario(name, count, anchorIndices, options = {}) {
  const anchorSet = new Set(anchorIndices);
  const wrappers = Array.from({ length: count }, (_, index) => (
    anchorSet.has(index)
      ? anchoredWrapper(index, 125 + index * 70, 110 + index * 35)
      : wrapperFragment({
        index,
        label: `A${index + 1}`,
        innerX: 100 + index * 73,
        innerY: 200 + index * 11,
      })
  ));
  const pageAttributes = options.pageAttributes
    ? ` ${options.pageAttributes}`
    : "";
  const positioned = options.positioned
    ? `<fragment id="900">
         <n id="901" p="${options.positioned[0]} ${options.positioned[1]}"/>
         <n id="902" p="${options.positioned[0] + 14.4} ${options.positioned[1]}"/>
         <b id="903" B="901" E="902"/>
       </fragment>`
    : "";
  const graphic = options.graphic
    ? `<graphic id="950" BoundingBox="${options.graphic.join(" ")}" GraphicType="Rectangle"/>`
    : "";
  const bondLength = options.bondLength ?? 14.4;
  return {
    name,
    wrapperIds: wrappers.map((entry) => entry.wrapperId),
    source: `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="${bondLength}" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1"${pageAttributes}>${positioned}${graphic}${wrappers.map((entry) => entry.xml).join("\n")}</page>
</CDXML>`,
  };
}

function substitutedRingScenario(name, options = {}) {
  const center = options.center ?? [300, 300];
  const inputBondLength = options.inputBondLength ?? 14.4;
  const rootBondLength = options.rootBondLength ?? 14.4;
  const substituentIndices = options.substituentIndices ?? [0];
  const ringIds = Array.from({ length: 6 }, (_, index) => 2001 + index);
  const ringNodes = ringIds.map((id, index) => {
    const angle = index * Math.PI / 3;
    const x = center[0] + inputBondLength * Math.cos(angle);
    const y = center[1] + inputBondLength * Math.sin(angle);
    return `<n id="${id}" p="${x.toFixed(4)} ${y.toFixed(4)}"/>`;
  });
  const ringBonds = ringIds.map((id, index) => (
    `<b id="${2101 + index}" B="${id}" E="${ringIds[(index + 1) % ringIds.length]}"${index % 2 ? ' Order="2"' : ""}/>`
  ));
  const wrappers = substituentIndices.map((ringIndex, index) => {
    const wrapperId = 2201 + index * 10;
    const fragmentId = wrapperId + 1;
    return {
      wrapperId: String(wrapperId),
      node: `<n id="${wrapperId}" NodeType="Fragment">
        <fragment id="${fragmentId}">
          <n id="${fragmentId + 1}" p="100 100"/>
          <n id="${fragmentId + 2}" p="114.4 100"/>
          <n id="${fragmentId + 3}" NodeType="ExternalConnectionPoint"/>
          <b id="${fragmentId + 4}" B="${fragmentId + 1}" E="${fragmentId + 2}"/>
          <b id="${fragmentId + 5}" B="${fragmentId + 2}" E="${fragmentId + 3}"/>
        </fragment>
        <t><s font="3" size="10">R${index + 1}</s></t>
      </n>`,
      bond: `<b id="${2301 + index}" B="${ringIds[ringIndex]}" E="${wrapperId}"/>`,
    };
  });
  const positioned = options.positioned
    ? `<fragment id="900"><n id="901" p="${options.positioned[0]} ${options.positioned[1]}"/><n id="902" p="${options.positioned[0] + inputBondLength} ${options.positioned[1]}"/><b id="903" B="901" E="902"/></fragment>`
    : "";
  const graphic = options.graphic
    ? `<graphic id="950" BoundingBox="${options.graphic.join(" ")}" GraphicType="Rectangle"/>`
    : "";
  return {
    name,
    wrapperIds: wrappers.map((entry) => entry.wrapperId),
    trackedIds: [...ringIds.map(String), ...wrappers.map((entry) => entry.wrapperId)],
    source: `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="${rootBondLength}" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" HeightPages="1" WidthPages="1">${positioned}${graphic}
    <fragment id="2000">${ringNodes.join("")}${wrappers.map((entry) => entry.node).join("")}${ringBonds.join("")}${wrappers.map((entry) => entry.bond).join("")}</fragment>
  </page>
</CDXML>`,
  };
}

function attributes(startTag) {
  return Object.fromEntries(
    [...startTag.matchAll(/([A-Za-z][\w:-]*)="([^"]*)"/g)]
      .map((match) => [match[1], match[2]]),
  );
}

function nodePosition(cdxml, id) {
  for (const match of cdxml.matchAll(/<n\b[^>]*>/g)) {
    const attrs = attributes(match[0]);
    if (attrs.id === id) return attrs.p || null;
  }
  return null;
}

function firstElementAttributes(cdxml, name) {
  const match = cdxml.match(new RegExp(`<${name}\\b[^>]*>`));
  return match ? attributes(match[0]) : null;
}

function nestedElementById(xml, name, id) {
  const startPattern = new RegExp(`<${name}\\b(?=[^>]*\\bid="${id}")[^>]*>`);
  const startMatch = startPattern.exec(xml);
  if (!startMatch) throw new Error(`Missing <${name} id="${id}">`);
  const tokenPattern = new RegExp(`</?${name}\\b[^>]*>`, "g");
  let depth = 0;
  for (const match of xml.slice(startMatch.index).matchAll(tokenPattern)) {
    const token = match[0];
    if (token.startsWith(`</${name}`)) depth -= 1;
    else if (!token.endsWith("/>")) depth += 1;
    if (depth === 0) {
      return xml.slice(startMatch.index, startMatch.index + match.index + token.length);
    }
  }
  throw new Error(`Unclosed <${name} id="${id}">`);
}

function isolatedRealFragmentScenario(name, source, fragmentId, trackedIds) {
  const fragment = nestedElementById(source, "fragment", fragmentId);
  return {
    name,
    wrapperIds: trackedIds,
    trackedIds,
    source: `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10">
  <fonttable><font id="2" charset="utf-8" name="Arial"/><font id="3" charset="utf-8" name="Times New Roman"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" HeightPages="1" WidthPages="1">${fragment}</page>
</CDXML>`,
  };
}

async function main() {
  await fs.mkdir(sourceRoot, { recursive: true });
  await fs.mkdir(oracleRoot, { recursive: true });
  const scenarios = [];
  for (let count = 1; count <= 16; count += 1) {
    scenarios.push(scenario(
      `count-${count}`,
      Array.from({ length: count }, (_, index) => `A${index + 1}`),
    ));
  }
  scenarios.push(scenario("label-widths", ["A", "LongAlias", "Et", "VeryLongAlias"]));
  for (const bondLength of [10, 15, 17, 30]) {
    scenarios.push(scenario(
      `count-6-bond-${bondLength}`,
      ["A1", "A2", "A3", "A4", "A5", "A6"],
      { bondLength },
    ));
  }
  scenarios.push(scenario("positioned-origin", ["A1", "A2", "A3", "A4"], {
    positioned: [0, 0],
  }));
  scenarios.push(scenario("positioned-far", ["A1", "A2", "A3", "A4"], {
    positioned: [300, 300],
  }));
  for (const count of [1, 2, 4, 6, 8, 9, 12, 15, 16]) {
    scenarios.push(scenario(
      `page-1x1-count-${count}`,
      Array.from({ length: count }, (_, index) => `A${index + 1}`),
      { pageAttributes: 'HeightPages="1" WidthPages="1"' },
    ));
  }
  for (const pageAttributes of [
    'HeightPages="2" WidthPages="1"',
    'HeightPages="1" WidthPages="2"',
    'HeightPages="2" WidthPages="2"',
    'HeightPages="1" WidthPages="1" BoundingBox="0 0 612 792"',
  ]) {
    const slug = pageAttributes.replaceAll(/[^A-Za-z0-9]+/g, "-").replaceAll(/^-|-$/g, "");
    scenarios.push(scenario(
      `page-${slug}-count-6`,
      ["A1", "A2", "A3", "A4", "A5", "A6"],
      { pageAttributes },
    ));
  }
  for (const positioned of [[0, 0], [300, 300], [430, 488], [900, 1200]]) {
    scenarios.push(scenario(
      `page-1x1-positioned-${positioned.join("-")}`,
      ["A1", "A2", "A3", "A4", "A5", "A6"],
      { pageAttributes: 'HeightPages="1" WidthPages="1"', positioned },
    ));
  }
  for (const graphic of [
    [0, 100, 100, 0],
    [0, 488, 430, 233],
    [0, 780, 600, 0],
    [700, 1200, 1000, 900],
  ]) {
    scenarios.push(scenario(
      `page-1x1-graphic-${graphic.join("-")}`,
      ["A1", "A2", "A3", "A4", "A5", "A6"],
      { pageAttributes: 'HeightPages="1" WidthPages="1"', graphic },
    ));
  }
  scenarios.push(scenario("external-second", ["A1", "Anchor", "A3", "A4", "A5", "A6"], {
    externalIndex: 1,
  }));
  for (let count = 2; count <= 8; count += 1) {
    for (let anchorIndex = 0; anchorIndex < count; anchorIndex += 1) {
      scenarios.push(anchoredScenario(anchorIndex, count));
    }
  }
  scenarios.push(multiAnchoredScenario("multi-anchor-6-slots-2-5", 6, [1, 4]));
  scenarios.push(multiAnchoredScenario(
    "page-1x1-multi-anchor-6-slots-2-5",
    6,
    [1, 4],
    { pageAttributes: 'HeightPages="1" WidthPages="1"' },
  ));
  for (const options of [
    { bondLength: 30 },
    { bondLength: 30, positioned: [300, 300] },
    { bondLength: 30, graphic: [0, 488, 430, 233] },
    { bondLength: 30, positioned: [300, 300], graphic: [0, 488, 430, 233] },
  ]) {
    const suffix = [
      "bond-30",
      options.positioned ? "positioned" : null,
      options.graphic ? "graphic" : null,
    ].filter(Boolean).join("-");
    scenarios.push(multiAnchoredScenario(
      `page-1x1-multi-anchor-15-nine-anchors-${suffix}`,
      15,
      [3, 4, 5, 6, 8, 9, 10, 11, 12],
      { ...options, pageAttributes: 'HeightPages="1" WidthPages="1"' },
    ));
  }
  scenarios.push(substitutedRingScenario("ring-one-root-14", {
    substituentIndices: [0],
  }));
  scenarios.push(substitutedRingScenario("ring-one-root-30", {
    rootBondLength: 30,
    substituentIndices: [0],
  }));
  scenarios.push(substitutedRingScenario("ring-four-root-30", {
    rootBondLength: 30,
    substituentIndices: [0, 1, 3, 5],
  }));
  scenarios.push(substitutedRingScenario("ring-four-root-30-positioned", {
    rootBondLength: 30,
    substituentIndices: [0, 1, 3, 5],
    positioned: [100, 100],
  }));
  scenarios.push(substitutedRingScenario("ring-four-root-30-graphic", {
    rootBondLength: 30,
    substituentIndices: [0, 1, 3, 5],
    graphic: [0, 488, 430, 233],
  }));
  scenarios.push(substitutedRingScenario("ring-four-root-30-suzuki-position-graphic", {
    center: [478, 336],
    rootBondLength: 30,
    substituentIndices: [0, 1, 3, 5],
    graphic: [0, 488, 430, 233],
  }));
  const publicCorpus = path.join(repoRoot, "tmp", "public-corpus-pilot");
  const suzuki1Path = path.join(
    publicCorpus,
    "indigo", "api", "tests", "integration", "tests", "formats", "ref", "Suzuki_Rxn1.cdxml",
  );
  const suzuki2Path = path.join(
    publicCorpus,
    "indigo", "api", "tests", "integration", "tests", "formats", "ref", "Suzuki_Rxn2.cdxml",
  );
  try {
    const [suzuki1, suzuki2] = await Promise.all([
      fs.readFile(suzuki1Path, "utf8"),
      fs.readFile(suzuki2Path, "utf8"),
    ]);
    scenarios.push(isolatedRealFragmentScenario(
      "isolated-suzuki-rxn1-fragment-101",
      suzuki1,
      "101",
      ["102", "103", "116", "422", "427", "432", "437"],
    ));
    scenarios.push(isolatedRealFragmentScenario(
      "isolated-suzuki-rxn1-fragment-137",
      suzuki1,
      "137",
      ["138", "139", "140", "502", "507", "512", "517", "522"],
    ));
    scenarios.push(isolatedRealFragmentScenario(
      "isolated-suzuki-rxn2-fragment-84",
      suzuki2,
      "84",
      ["85", "86", "99", "414", "419", "424", "429"],
    ));
    scenarios.push(isolatedRealFragmentScenario(
      "isolated-suzuki-rxn2-fragment-120",
      suzuki2,
      "120",
      ["121", "122", "123", "494", "499", "504", "509", "514"],
    ));
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  scenarios.push(multiAnchoredScenario(
    "page-1x1-multi-anchor-15-nine-anchors",
    15,
    [3, 4, 5, 6, 8, 9, 10, 11, 12],
    { pageAttributes: 'HeightPages="1" WidthPages="1"' },
  ));

  const inputs = [];
  for (const entry of scenarios) {
    const input = path.join(sourceRoot, `${entry.name}.cdxml`);
    await fs.writeFile(input, entry.source, "utf8");
    inputs.push(input);
  }
  const jobs = await generateChemDrawOracle({
    outDir: oracleRoot,
    formats: ["cdxml"],
    inputs,
  });
  const results = [];
  for (let index = 0; index < scenarios.length; index += 1) {
    const entry = scenarios[index];
    const saved = await fs.readFile(jobs[index].outputs.cdxml, "utf8");
    results.push({
      name: entry.name,
      page: firstElementAttributes(saved, "page"),
      positions: (entry.trackedIds ?? entry.wrapperIds)
        .map((id) => ({ id, p: nodePosition(saved, id) })),
    });
  }
  const summary = {
    schema: "chemsema.chemdraw-collapsed-fragment-layout-probe.v1",
    generatedAt: new Date().toISOString(),
    results,
  };
  const summaryPath = path.join(outputRoot, "summary.json");
  await fs.writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
  console.log(JSON.stringify({ summaryPath, results }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exit(1);
});
