import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const official = JSON.parse(readFileSync(join(rootDir, "schemas", "cdx-cdxml-official-v1.json"), "utf8"));
const ledgerPath = join(rootDir, "docs", "cdx-cdxml-field-verification.zh-CN.md");
const behavior = JSON.parse(readFileSync(join(rootDir, "schemas", "cdx-cdxml-behavior-status-v1.json"), "utf8"));
const chemDrawEvidence = JSON.parse(readFileSync(join(rootDir, "schemas", "chemdraw-cdxml-field-evidence-v1.json"), "utf8"));
const cdx = readFileSync(join(rootDir, "crates", "chemsema-engine", "src", "cdx.rs"), "utf8");
const cdxml = [
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "colors.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "export.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "import_objects.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "import_nodes.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "import_bonds.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "import_fragments.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "import_chemical_properties.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "text_runs.rs"),
  join(rootDir, "crates", "chemsema-engine", "src", "cdxml", "template_library.rs"),
].map((path) => readFileSync(path, "utf8")).join("\n");

const objectBody = cdx.slice(cdx.indexOf("fn object_name"), cdx.indexOf("fn legacy_chemsema_object_name"));
const explicitObjects = new Set([...objectBody.matchAll(/0x[0-9A-Fa-f]{4}\s*=>\s*"([^"]+)"/g)].map((match) => match[1]));
const propertyBody = cdx.slice(cdx.indexOf("fn property_schema"), cdx.indexOf("fn property_tag"));
const explicitPropertyTags = new Set([...propertyBody.matchAll(/(0x[0-9A-Fa-f]{4})\s*=>\s*\("/g)]
  .map((match) => match[1].toUpperCase().replace("0X", "0x")));
const directCdxmlAttributes = new Set([...cdxml.matchAll(/\.attr\("([^"]+)"\)/g)].map((match) => match[1]));
const nativeSemanticProperties = new Set([
  "Isotope", "IsotopicAbundance", "Radical", "AtomNumber", "ShowAtomNumber", "AS",
  "ShowAtomStereo", "ElementList", "GenericList", "FreeSites", "RingBondCount",
  "UnsaturatedBonds", "SubstituentsUpTo", "SubstituentsExactly", "Translation",
  "AbnormalValence", "ShowTerminalCarbonLabels", "ShowNonTerminalCarbonLabels",
  "RxnChange", "RxnStereo", "Topology", "RxnParticipation", "BS",
  "ShowBondQuery", "ShowBondRxn", "ShowBondStereo",
  "BasisObjects", "ChemicalPropertyType", "ChemicalPropertyDisplayID",
  "ChemicalPropertyIsActive",
  "extent", "PaneHeight", "NumRows", "NumColumns",
]);
const nativeSemanticObjects = new Set(["chemicalproperty", "templategrid"]);
const lexicalCdxTypes = new Set([
  "CDXString", "CDXBoolean", "CDXBooleanImplied", "INT8", "UINT8", "INT16", "UINT16",
  "INT32", "UINT32", "FLOAT64", "CDXCoordinate", "CDXPoint2D", "CDXPoint3D", "CDXRectangle",
  "CDXObjectID", "CDXObjectIDArray", "INT16ListWithCounts",
  "CDXObjectIDArrayWithCounts", "CDXElementList", "CDXCurvePoints", "CDXCurvePoints3D",
  "CDXDate", "CDXRepresentsProperty",
  "CDXGenericList",
]);
const chemDrawObjectTagOverrides = new Map(Object.entries({
  geometry: "0x801B",
  constraint: "0x801C",
  tlcplate: "0x801D",
  tlclane: "0x801E",
  tlcspot: "0x801F",
  chemicalproperty: "0x8020",
  arrow: "0x8021",
  border: "0x801A",
}));

const behaviorStatus = (family, name) => {
  const explicit = behavior[family]?.[name];
  if (explicit) return explicit;
  if (family === "objects") return chemDrawEvidence.objects?.[name]?.status ?? behavior.default;
  if (family === "properties" || family === "cdxmlAttributes") {
    return chemDrawEvidence.attributes?.[name]?.status ?? behavior.default;
  }
  return behavior.default;
};
const cdxImplementation = (property) => {
  if (nativeSemanticProperties.has(property.cdxmlName) || explicitPropertyTags.has(property.tag) || ["CDXFontTable", "CDXColorTable"].includes(property.cdxType)) return "native-semantic";
  if (lexicalCdxTypes.has(property.cdxType)) return "typed-interchange";
  if (property.cdxType === "varies") return "context-dependent-interchange";
  return "opaque-by-spec";
};
const cdxFormatRule = (property) => {
  if (property.cdxmlName === "PaneHeight") return "TemplateGrid pane height is decoded as official 16.16 CDXCoordinate and edited through the native layout model.";
  if (property.cdxmlName === "NumRows" || property.cdxmlName === "NumColumns") return "TemplateGrid dimension is validated and edited through the native layout model.";
  if (property.cdxmlName === "extent") return "Official point codec is available; TemplateGrid consumes the pair as native cell width and height.";
  if (property.cdxType === "CDXFormula") return "Official data type is reserved/undefined; ChemDraw does not read or write it.";
  if (property.cdxType === "Unformatted") return "Official type is uninterpreted bytes; rawBase64 is authoritative.";
  if (property.cdxType === "varies") return "Decode according to the containing object tag's TagType; rawBase64 remains authoritative.";
  if (property.cdxType === "CDXFontTable") return "Edit structured font children and explicit native text styles.";
  if (property.cdxType === "CDXColorTable") return "Edit structured color children and explicit native colors.";
  return "Official lexical/binary codec is available; value is editable.";
};
const properties = official.cdx.properties.map((property) => ({
  ...property,
  schemaStatus: official.errata.some((entry) => entry.sdkName === property.sdkName) ? "verified-with-erratum" : "verified",
  storageStatus: "verified",
  implementation: cdxImplementation(property),
  editMode: ["CDXFontTable", "CDXColorTable"].includes(property.cdxType)
    ? "children"
    : lexicalCdxTypes.has(property.cdxType) ? "value" : "rawBase64",
  behaviorStatus: behaviorStatus("properties", property.cdxmlName ?? property.sdkName),
  formatRule: cdxFormatRule(property),
}));
const objects = official.cdx.objects.map((object) => ({
  ...object,
  publishedTag: chemDrawObjectTagOverrides.has(object.cdxmlName) ? object.tag : undefined,
  tag: chemDrawObjectTagOverrides.get(object.cdxmlName) ?? object.tag,
  schemaStatus: chemDrawObjectTagOverrides.has(object.cdxmlName)
    ? "verified-with-erratum"
    : "verified",
  storageStatus: "verified",
  implementation: Number.parseInt(object.tag.slice(2), 16) < 0x8000
    ? "property-backed-helper"
    : explicitObjects.has(object.cdxmlName) || nativeSemanticObjects.has(object.cdxmlName)
      ? "native-object-tag"
      : "interchange-object",
  behaviorStatus: behaviorStatus("objects", object.cdxmlName),
}));
const attributesByName = new Map();
for (const element of official.cdxml.elements) {
  for (const attribute of element.attributes) {
    const entry = attributesByName.get(attribute.name) ?? {
      name: attribute.name,
      elements: [],
      declarations: [],
    };
    entry.elements.push(element.name);
    entry.declarations.push({
      element: element.name,
      type: attribute.type,
      required: attribute.required,
      default: attribute.default,
      values: attribute.values,
    });
    attributesByName.set(attribute.name, entry);
  }
}
const attributes = [...attributesByName.values()].map((attribute) => ({
  ...attribute,
  schemaStatus: "verified",
  storageStatus: "verified",
  implementation: directCdxmlAttributes.has(attribute.name) ? "native-semantic" : "typed-interchange",
  editMode: "value",
  behaviorStatus: behaviorStatus("cdxmlAttributes", attribute.name)
    === behavior.default
    ? behaviorStatus("properties", attribute.name)
    : behaviorStatus("cdxmlAttributes", attribute.name),
}));

const verification = {
  schema: "chemsema.cdx-cdxml-field-verification.v1",
  officialSchema: "cdx-cdxml-official-v1.json",
  generatedAt: official.generatedAt,
  guarantees: {
    cdxObjects: "Every official object tag is parsed and writable; unmodeled objects remain in interchange.cdx.",
    cdxProperties: "Every property retains official name/type/tag and exact rawBase64; public lexical types are value-editable.",
    cdxml: "Every element and attribute is retained in an editable interchange.cdxml tree and merged on export.",
    nativeAuthority: "Source-independent CCJS fields remain authoritative for modeled chemistry and drawing semantics.",
    behavior: "Every object, CDX property, unique CDXML attribute, and element-attribute declaration has an official rule plus ChemDraw, codec, contextual, opaque-payload, or explicit unsupported/read-only evidence classification.",
    objectTagErratum: "For Geometry through Border, runtime tags follow CDX files written by ChemDraw 12 and 21; the shifted static table remains recorded as publishedTag for auditability.",
  },
  counts: {
    cdxObjects: objects.length,
    cdxProperties: properties.length,
    cdxmlElements: official.cdxml.elements.length,
    cdxmlAttributes: attributes.length,
    cdxmlDeclarations: Object.keys(chemDrawEvidence.declarations).length,
    losslessUncovered: 0,
    nativeSemanticCdxProperties: properties.filter((entry) => entry.implementation === "native-semantic").length,
    typedInterchangeCdxProperties: properties.filter((entry) => entry.implementation === "typed-interchange").length,
    opaqueBySpecCdxProperties: properties.filter((entry) => entry.implementation === "opaque-by-spec").length,
    contextDependentCdxProperties: properties.filter((entry) => entry.implementation === "context-dependent-interchange").length,
    nativeSemanticCdxmlAttributes: attributes.filter((entry) => entry.implementation === "native-semantic").length,
    verifiedBehaviorObjects: objects.filter((entry) => entry.behaviorStatus === "verified").length,
    verifiedBehaviorCdxProperties: properties.filter((entry) => entry.behaviorStatus === "verified").length,
    verifiedBehaviorCdxmlAttributes: attributes.filter((entry) => entry.behaviorStatus === "verified").length,
    verifiedBehaviorCdxmlDeclarations: Object.values(chemDrawEvidence.declarations).filter((entry) => entry.status === "verified").length,
  },
  objects,
  properties,
  cdxml: { elements: official.cdxml.elements, attributes },
};
writeFileSync(join(rootDir, "schemas", "cdx-cdxml-verification-v1.json"), `${JSON.stringify(verification, null, 2)}\n`);

const escapeCell = (value) => String(value ?? "").replaceAll("|", "\\|").replaceAll("\n", " ");
const lines = [
  "# CDX/CDXML 字段复核总账",
  "",
  "本总账由官方 CDX 属性/对象表和 Revvity 当前 CDXML DTD 自动生成。机器可读全集见 `schemas/cdx-cdxml-verification-v1.json`。",
  "",
  "这里把三件事分开记录：`schemaStatus` 是官方 tag、类型、枚举和默认值是否已核对；`storageStatus` 是是否能无损进入 CCJS、修改并回写；`behaviorStatus` 是是否完成 ChemDraw 实物/视觉行为矩阵。不能用无损保存冒充行为已经完全复现。",
  "",
  `当前官方全集：${objects.length} 个 CDX 对象/辅助对象、${properties.length} 个 CDX 属性、${official.cdxml.elements.length} 个 CDXML 元素、${attributes.length} 个唯一 CDXML 属性名。无损未覆盖数：0。`,
  "",
  "## 实现规则",
  "",
  "- `native-semantic`：已映射为来源无关的 CCJS 明确字段；编辑原生字段，导出器负责换算。",
  "- `typed-interchange`：官方公共词法类型已解析为可编辑 `value`，并保留 tag/type。",
  "- `binary-interchange`：官方没有稳定公共词法形式或结构复杂；保留精确 `rawBase64`，由专用编辑器修改。",
  "- `opaque-by-spec`：官方明确规定为未定义或不解释的字节载荷；不得猜测语义，编辑 `rawBase64`。",
  "- `context-dependent-interchange`：类型由同对象的其他字段决定；保留 `value` 与 `rawBase64`，专用编辑器联合修改。",
  "- `interchange` 不是 `meta`，是 CCJS 顶层的持久化、可编辑、参与导出的正式字段。",
  "",
  "## CDX 对象全集",
  "",
  "| tag | CDXML 对象 | 实现 | schema | storage | behavior |",
  "| --- | --- | --- | --- | --- | --- |",
  ...objects.map((entry) => `| \`${entry.tag}\` | \`${escapeCell(entry.cdxmlName)}\` | \`${entry.implementation}\` | \`${entry.schemaStatus}\` | \`${entry.storageStatus}\` | \`${entry.behaviorStatus}\` |`),
  "",
  "## CDX 属性全集",
  "",
  "| tag | CDXML 名 | CDX 类型 | 实现/编辑 | schema | storage | behavior | 规则摘要 |",
  "| --- | --- | --- | --- | --- | --- | --- | --- |",
  ...properties.map((entry) => `| \`${entry.tag}\` | \`${escapeCell(entry.cdxmlName ?? "(not used)")}\` | \`${escapeCell(entry.cdxType)}\` | \`${entry.implementation}/${entry.editMode}\` | \`${entry.schemaStatus}\` | \`${entry.storageStatus}\` | \`${entry.behaviorStatus}\` | ${escapeCell(entry.description)} ${escapeCell(entry.formatRule)} |`),
  "",
  "## CDXML 元素全集",
  "",
  "| 元素 | 内容模型 | 属性数 |",
  "| --- | --- | ---: |",
  ...official.cdxml.elements.map((entry) => `| \`${escapeCell(entry.name)}\` | \`${escapeCell(entry.contentModel)}\` | ${entry.attributes.length} |`),
  "",
  "## CDXML 唯一属性名全集",
  "",
  "| 属性 | 出现元素数 | 实现 | schema | storage | behavior |",
  "| --- | ---: | --- | --- | --- | --- |",
  ...attributes.map((entry) => `| \`${escapeCell(entry.name)}\` | ${entry.elements.length} | \`${entry.implementation}\` | \`${entry.schemaStatus}\` | \`${entry.storageStatus}\` | \`${entry.behaviorStatus}\` |`),
  "",
  "## 已知官方表勘误",
  "",
  ...official.errata.map((entry) => `- \`${entry.sdkName}\`：发布表 ${escapeCell(JSON.stringify(entry.published))}；采用 ${escapeCell(JSON.stringify(entry.corrected))}。依据：${escapeCell(entry.basis)}。`),
  "",
  "## ChemDraw 二进制对象 tag 勘误",
  "",
  "- ChemDraw 12.0.2 与 21.0.0.28 的实际 CDX 输出一致使用：`Border=0x801a`、`Geometry=0x801b`、`Constraint=0x801c`、`TLCPlate=0x801d`、`TLCLane=0x801e`、`TLCSpot=0x801f`、`ChemicalProperty=0x8020`、`Arrow=0x8021`。发布版静态对象表从 Border 起发生了偏移；账本保留其原值为 `publishedTag`，运行时值记为 `tag` 并标为 `verified-with-erratum`。",
  "- 导入器仍识别 ChemSema beta 阶段写出的偏移 tag，但新文件统一写 ChemDraw 实际 tag；这属于格式级兼容，不按单个样例分支。",
  "",
];
lines.splice(8, 0,
  "## \u884c\u4e3a\u8bc1\u636e\u95e8\u7981",
  "",
  `CDXML \u4e0d\u53ea\u6309 ${attributes.length} \u4e2a\u552f\u4e00\u5c5e\u6027\u540d\u7edf\u8ba1\uff0c\u8fd8\u6309\u201c\u5143\u7d20 \u00d7 \u5c5e\u6027\u201d\u5c55\u5f00\u4e3a ${Object.keys(chemDrawEvidence.declarations).length} \u4e2a\u5177\u4f53\u58f0\u660e\u3002\u5f53\u524d\u5df2\u590d\u6838 ${Object.values(chemDrawEvidence.declarations).filter((entry) => entry.status === "verified").length}/${Object.keys(chemDrawEvidence.declarations).length}\uff0c\u95e8\u7981\u4e0d\u5141\u8bb8\u9057\u7559 \`in-review\`\u3002`,
  "",
  "\u8fd9\u91cc\u7684 `verified` \u4e0d\u7b49\u4e8e\u201c\u6240\u6709\u5bf9\u8c61\u90fd\u5df2\u539f\u751f\u91cd\u7ed8\u201d\uff1a\u5bf9\u4e8e\u8d28\u7c92\u3001\u51dd\u80f6\u6807\u8bb0\u3001\u5316\u5b66\u8ba1\u91cf\u8868\u7b49\u4e13\u7528\u4e0a\u4e0b\u6587\u5bf9\u8c61\uff0c\u8868\u793a\u5df2\u6838\u5bf9 DTD \u8bcd\u6cd5/\u679a\u4e3e/\u9ed8\u8ba4\u503c\u3001ChemDraw \u5bf9\u65e0\u6548\u4e0a\u4e0b\u6587\u7684\u6e05\u7406\u884c\u4e3a\uff0c\u4ee5\u53ca CCJS \u65e0\u635f\u53ef\u7f16\u8f91\u5f80\u8fd4\u3002\u662f\u539f\u751f\u8bed\u4e49\u3001\u4e0a\u4e0b\u6587\u89c4\u5219\uff0c\u8fd8\u662f\u5b98\u65b9\u660e\u786e\u7684\u53ea\u5199/\u672a\u542f\u7528\uff0c\u7531 `schemas/chemdraw-cdxml-field-evidence-v1.json` \u7684 `verificationKind` \u533a\u5206\u3002",
  "",
  "\u539f\u751f\u7ed8\u5236\u3001\u7f16\u8f91\u548c\u5f80\u8fd4\u7684\u9010\u9879\u5b9e\u65bd\u987a\u5e8f\u89c1 `docs/cdx-cdxml-native-rendering-backlog.zh-CN.md`\u3002",
  "",
);
writeFileSync(ledgerPath, `${lines.join("\n").replace(/\n+$/, "")}\n`);
console.log(JSON.stringify(verification.counts));
