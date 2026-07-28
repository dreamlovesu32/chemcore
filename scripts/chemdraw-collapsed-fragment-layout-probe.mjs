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
  return {
    name,
    wrapperIds: wrappers.map((entry) => entry.wrapperId),
    source: `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="${bondLength}" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1">
    ${positioned}
    ${wrappers.map((entry) => entry.xml).join("\n")}
  </page>
</CDXML>`,
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
  const fragmentId = 1000 + anchorIndex * 10;
  const anchored = {
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
        <t><s font="3" size="10">Anchor</s></t>
      </n>
      <n id="${fragmentId + 8}" p="125 110" NodeType="GenericNickname">
        <t p="125 110"><s font="3" size="10">M</s></t>
      </n>
      <b id="${fragmentId + 9}" B="${fragmentId + 8}" E="${fragmentId + 1}"/>
    </fragment>`,
  };
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
  scenarios.push(scenario("external-second", ["A1", "Anchor", "A3", "A4", "A5", "A6"], {
    externalIndex: 1,
  }));
  for (let count = 2; count <= 8; count += 1) {
    for (let anchorIndex = 0; anchorIndex < count; anchorIndex += 1) {
      scenarios.push(anchoredScenario(anchorIndex, count));
    }
  }

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
      positions: entry.wrapperIds.map((id) => ({ id, p: nodePosition(saved, id) })),
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
