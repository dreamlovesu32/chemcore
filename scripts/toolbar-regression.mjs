import assert from "node:assert/strict";
import fs from "node:fs";
import {
  renderSecondaryToolbarHtml,
  syncPrimaryToolButtons,
} from "../viewer/toolbar.js";

const baseEditorState = {
  activeTool: "bond",
  bondType: "single",
  bondIconSvgs: {},
  arrowIconSvgs: {},
  shapeIconSvgs: {},
  symbolIconSvgs: {},
  orbitalIconSvgs: {},
  chainIconSvg: "",
  colorPalette: null,
  documentColors: [],
  elementPalette: null,
};

const html = renderSecondaryToolbarHtml(baseEditorState);
const bondButtons = [...html.matchAll(/data-secondary-value="bond-[^"]+"/g)];
const svgCount = [...html.matchAll(/<svg\b/g)];

assert.equal(bondButtons.length, 11, "bond toolbar should render every bond tool");
assert.equal(svgCount.length, 11, "bond toolbar buttons should not render blank icons when engine icons are unavailable");
assert.match(
  html,
  /cc-bond-icon-static/,
  "bond toolbar should use its declared static icons before kernel icons are ready",
);

const textHtml = renderSecondaryToolbarHtml({
  ...baseEditorState,
  activeTool: "text",
  textFontFamily: "Aptos Display",
  textFontSize: 16,
  textColor: "#000000",
  textAlign: "left",
  textBold: false,
  textItalic: false,
  textUnderline: false,
  textOutline: false,
  textShadow: false,
  textScript: "normal",
  textIconSvgs: {},
});
assert.match(textHtml, /<input[^>]+data-text-control="font"[^>]+list="text-font-options"/, "font family control should accept arbitrary names");
assert.match(textHtml, /value="Aptos Display"/, "font family control should retain an imported custom family");
assert.match(textHtml, /data-secondary-value="text-outline"/, "text toolbar should expose outline");
assert.match(textHtml, /data-secondary-value="text-shadow"/, "text toolbar should expose shadow");

const chromatographyHtml = renderSecondaryToolbarHtml({
  ...baseEditorState,
  activeTool: "tlc-plate",
  chromatographyKind: "gel-plate",
  shapeColor: "#000000",
});
assert.match(
  chromatographyHtml,
  /data-secondary-value="chromatography-kind-tlc-plate"/,
  "chromatography toolbar should expose the TLC plate",
);
assert.match(
  chromatographyHtml,
  /class="secondary-button is-selected"[^>]*data-secondary-value="chromatography-kind-gel-plate"/,
  "chromatography toolbar should expose and retain the gel plate",
);

const chromatographyButton = {
  dataset: { tool: "tlc-plate" },
  innerHTML: "",
  attributes: {},
  classList: { toggle() {} },
  setAttribute(name, value) {
    this.attributes[name] = value;
  },
};
const chromatographyRoot = {
  querySelectorAll(selector) {
    return selector === "[data-tool]" ? [chromatographyButton] : [];
  },
  querySelector(selector) {
    return selector === '.tool-button[data-tool="tlc-plate"]' ? chromatographyButton : null;
  },
};
syncPrimaryToolButtons(
  { ...baseEditorState, activeTool: "select", chromatographyKind: "gel-plate" },
  chromatographyRoot,
);
assert.match(
  chromatographyButton.innerHTML,
  /M9\.1 7\.2h2\.25/,
  "the left chromatography tool should retain the chosen gel icon after switching to select",
);
assert.equal(
  chromatographyButton.attributes.title,
  "Gel electrophoresis plate",
  "the left chromatography tool title should retain the chosen gel tool",
);

const bioDrawHtml = renderSecondaryToolbarHtml({
  ...baseEditorState,
  activeTool: "biodraw",
  bioDrawFamily: "plasmid",
  bioDrawKind: "plasmid-map",
  bioDrawFillType: "shaded",
  bioDrawLineType: "solid",
  shapeColor: "#000000",
});
assert.match(
  bioDrawHtml,
  /class="secondary-button is-selected"[^>]*data-secondary-value="biodraw-kind-plasmid-map"/,
  "BioDraw toolbar should expose and retain the plasmid-map tool",
);
assert.match(
  bioDrawHtml,
  /data-color-prefix="shape-color"/,
  "BioDraw toolbar should expose the shared object color control",
);

const membraneHtml = renderSecondaryToolbarHtml({
  ...baseEditorState,
  activeTool: "biodraw",
  bioDrawFamily: "membrane",
  bioDrawKind: "membrane-arc",
  bioDrawFillType: "none",
  bioDrawLineType: "dashed",
  shapeColor: "#336699",
});
assert.equal(
  [...membraneHtml.matchAll(/data-secondary-value="biodraw-kind-membrane-/g)].length,
  4,
  "the membrane family should expose all four official membrane objects",
);
assert.match(
  membraneHtml,
  /class="secondary-button is-selected"[^>]*data-secondary-value="biodraw-kind-membrane-arc"/,
  "a biology family should retain its chosen concrete object",
);
assert.match(
  membraneHtml,
  /class="secondary-button is-selected"[^>]*data-secondary-value="bio-fill-none"/,
  "BioShape fill should be retained independently from the family",
);
assert.match(
  membraneHtml,
  /class="secondary-button is-selected"[^>]*data-secondary-value="bio-line-dashed"/,
  "BioShape outline should be retained independently from the family",
);

const indexHtml = fs.readFileSync(new URL("../viewer/index.html", import.meta.url), "utf8");
assert.equal(
  [...indexHtml.matchAll(/data-tool="select"/g)].length,
  1,
  "selection must be the single shared tool outside both replaceable rail bodies",
);
assert.equal(
  [...indexHtml.matchAll(/data-tool-rail="biology"/g)].length,
  10,
  "the Biology-Assisted Drawing Rail should expose ten stable families",
);
assert.equal(
  [...indexHtml.matchAll(/data-tool-rail-toggle/g)].length,
  1,
  "the left rail should have one dedicated bottom switch",
);
assert.doesNotMatch(
  indexHtml,
  /data-tool="select"[^>]+data-tool-rail=/,
  "selection must not disappear when either rail body is replaced",
);
assert.match(indexHtml, /id="template-panel-mode-button"/, "the 40 px footer should expose the template entry");
assert.match(indexHtml, /id="paper-layout-mode-button"/, "the 40 px footer should expose the paper-layout toggle");
const stylesCss = fs.readFileSync(new URL("../viewer/styles.css", import.meta.url), "utf8");
assert.match(
  stylesCss,
  /--selection-status-bar-height:\s*40px/,
  "the document status bar should have the agreed 40 px height",
);

console.log("[toolbar-regression] ok");
