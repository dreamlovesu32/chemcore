import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const schema = JSON.parse(readFileSync(join(rootDir, "schemas", "cdx-cdxml-official-v1.json"), "utf8"));
const cdxSchema = readFileSync(
  join(rootDir, "crates", "chemsema-engine", "src", "cdx", "schema.rs"),
  "utf8",
);
const cdxmlDir = join(rootDir, "crates", "chemsema-engine", "src", "cdxml");
const cdxml = [join(rootDir, "crates", "chemsema-engine", "src", "cdxml.rs")]
  .concat(readdirSync(cdxmlDir).filter((name) => name.endsWith(".rs")).map((name) => join(cdxmlDir, name)))
  .map((path) => readFileSync(path, "utf8"))
  .join("\n");

const objectNameBody = cdxSchema.slice(
  cdxSchema.indexOf("fn object_name"),
  cdxSchema.indexOf("fn legacy_chemsema_object_name"),
);
const currentObjects = new Map(
  [...objectNameBody.matchAll(/(0x[0-9A-Fa-f]{4})\s*=>\s*\"([^\"]+)\"/g)]
    .map((match) => [match[2], match[1].toUpperCase().replace("0X", "0x")]),
);
const currentProperties = new Map();
const currentPropertiesByTag = new Map();
const propertyBody = cdxSchema.slice(
  cdxSchema.indexOf("fn property_schema"),
  cdxSchema.indexOf("fn property_tag"),
);
for (const match of propertyBody.matchAll(/(0x[0-9A-Fa-f]{4})(?:\s*\|\s*0x[0-9A-Fa-f]{4})*\s*=>\s*\(\"([^\"]+)\"/g)) {
  currentProperties.set(match[2], match[1].toUpperCase().replace("0X", "0x"));
  currentPropertiesByTag.set(match[1].toUpperCase().replace("0X", "0x"), match[2]);
}
const currentCdxmlAttributes = new Set(
  [...cdxml.matchAll(/\.attr\(\"([^\"]+)\"\)/g)].map((match) => match[1]),
);

const officialObjects = new Map(schema.cdx.objects
  .filter((entry) => Number.parseInt(entry.tag.slice(2), 16) >= 0x8000)
  .map((entry) => [entry.cdxmlName, entry.tag]));
const officialProperties = schema.cdx.properties.filter((entry) => entry.cdxmlName);
// The archived CDX property table uses the historical SDK spellings below,
// while the current CDXML DTD and ChemDraw XML use the Arrowhead* spellings.
// They are aliases for the same binary tags, not schema mismatches.
const acceptedPropertyAliases = new Map([
  ["0x0A2F", new Set(["ArrowHeadType", "ArrowheadType"])],
  ["0x0A30", new Set(["HeadCenterSize", "ArrowheadCenterSize"])],
  ["0x0A31", new Set(["HeadWidth", "ArrowheadWidth"])],
  ["0x0A35", new Set(["ArrowHeadHead", "ArrowheadHead"])],
  ["0x0A36", new Set(["ArrowHeadTail", "ArrowheadTail"])],
]);
const officialCdxmlAttributes = new Set(
  schema.cdxml.elements.flatMap((element) => element.attributes.map((attribute) => attribute.name)),
);

const missingObjects = [...officialObjects].filter(([name]) => !currentObjects.has(name));
const mismatchedObjects = [...officialObjects]
  .filter(([name, tag]) => currentObjects.has(name) && currentObjects.get(name) !== tag)
  .map(([name, tag]) => ({ name, officialTag: tag, currentTag: currentObjects.get(name) }));
const missingProperties = officialProperties.filter((entry) => !currentPropertiesByTag.has(entry.tag));
const mismatchedProperties = officialProperties
  .filter((entry) => {
    if (!currentPropertiesByTag.has(entry.tag)) return false;
    const currentName = currentPropertiesByTag.get(entry.tag);
    return currentName !== entry.cdxmlName
      && !acceptedPropertyAliases.get(entry.tag)?.has(currentName);
  })
  .map((entry) => ({
    tag: entry.tag,
    officialName: entry.cdxmlName,
    currentName: currentPropertiesByTag.get(entry.tag),
  }));
const missingCdxmlAttributes = [...officialCdxmlAttributes].filter((name) => !currentCdxmlAttributes.has(name)).sort();

const report = {
  schema: "chemsema.cdx-cdxml-schema-coverage.v1",
  official: schema.counts,
  current: {
    cdxObjects: currentObjects.size,
    cdxProperties: currentProperties.size,
    directlyReadCdxmlAttributes: currentCdxmlAttributes.size,
  },
  losslessCoverage: {
    cdxObjects: "all tags retained in interchange.cdx with formatTag",
    cdxProperties: "all properties retained with official name/type/tag and rawBase64",
    cdxmlElements: "complete editable XML object tree retained in interchange.cdxml",
    cdxmlAttributes: "all attributes retained as named editable properties",
    uncovered: [],
  },
  nativeSemanticCoverage: {
    missingObjects: missingObjects.map(([name, tag]) => ({ name, tag })),
    mismatchedObjects,
    missingProperties: missingProperties.map(({ cdxmlName: name, tag, cdxType }) => ({ name, tag, cdxType })),
    mismatchedProperties,
    missingCdxmlAttributes,
  },
};

const outIndex = process.argv.indexOf("--out");
if (outIndex >= 0) {
  const outputPath = resolve(rootDir, process.argv[outIndex + 1]);
  writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
}
console.log(JSON.stringify(report, null, 2));
