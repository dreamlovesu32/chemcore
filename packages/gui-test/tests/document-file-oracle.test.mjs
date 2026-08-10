import assert from "node:assert/strict";
import test from "node:test";
import { evaluateDocumentArrowProperties, evaluateDocumentReports, evaluateDocumentShapeProperties, evaluateDocumentSymbolProperties, evaluateDocumentTextProperties, validationLevelForDocumentBytes } from "../src/oracles/document-file.mjs";

const validReports = {
  inspect: {
    summary: {
      format: { name: "chemsema", version: "0.2" },
      counts: { nodes: 2, bonds: 1, molecules: 1, objects: 1 },
    },
  },
  validation: { schema: "chemsema.validation-report.v1", ok: true, issues: [] },
};

test("independent document oracle requires valid CCJS and exact chemical counts", () => {
  const expected = { nodes: 2, bonds: 1, molecules: 1, objects: 1 };
  assert.equal(evaluateDocumentReports(validReports, expected).passed, true);
  assert.equal(evaluateDocumentReports(validReports, { ...expected, bonds: 2 }).passed, false);
  assert.equal(evaluateDocumentReports({ ...validReports, validation: { ...validReports.validation, issues: [{}] } }, expected).passed, false);
  assert.equal(evaluateDocumentReports({ ...validReports, inspect: { summary: { ...validReports.inspect.summary, format: { name: "chemsema", version: "0.1" } } } }, expected).passed, false);
});

test("saved-document arrow oracle checks exact public CCJS properties", () => {
  const bytes = Buffer.from(JSON.stringify({
    styles: { style_red: { stroke: "#ff0000" } },
    entities: {
      scene: [{
        id: "obj_line_1",
        type: "line",
        styleRef: "style_red",
        payload: {
          arrowHead: {
            kind: "curved-mirror",
            curve: 120,
            length: 45,
            head: "half-right",
            tail: "half-left",
            bold: true,
            noGo: "hash",
          },
        },
      }],
    },
  }));
  const expected = [{ id: "obj_line_1", kind: "curved-mirror", curve: 120, length: 45, head: "half-right", tail: "half-left", bold: true, noGo: "hash", stroke: "#ff0000" }];
  assert.equal(evaluateDocumentArrowProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentArrowProperties(bytes, [{ ...expected[0], length: 22.5 }]).passed, false);
  assert.equal(evaluateDocumentArrowProperties(bytes, [{ ...expected[0], id: "obj_line_missing" }]).passed, false);
});

test("saved-document text oracle checks exact content and public style properties", () => {
  const bytes = Buffer.from(JSON.stringify({
    styles: { style_obj_text_1_color: { fill: "#0000ff" } },
    entities: {
      scene: [{
        id: "obj_text_1",
        type: "text",
        styleRef: "style_obj_text_1_color",
        payload: {
          text: "ChemSema H2O",
          fontFamily: "Times New Roman",
          fontSize: 24,
          align: "center",
          lineHeight: 20,
          lineHeightMode: "fixed",
          runs: [{ text: "ChemSema H2O", fontFamily: "Times New Roman", fontSize: 24, fontWeight: 700, fontStyle: "italic", underline: true, outline: true, shadow: true, script: "subscript", fill: "#0000ff" }],
          sourceRuns: [{ text: "ChemSema H2O", fontFamily: "Times New Roman", fontSize: 24, fontWeight: 700, fontStyle: "italic", underline: true, outline: true, shadow: true, script: "subscript", fill: "#0000ff" }],
          displayRuns: [{ text: "ChemSema H2O", fontFamily: "Times New Roman", fontSize: 24, fontWeight: 700, fontStyle: "italic", underline: true, outline: true, shadow: true, script: "subscript", fill: "#0000ff" }],
        },
      }],
    },
  }));
  const expected = [{ id: "obj_text_1", text: "ChemSema H2O", fontFamily: "Times New Roman", fontSize: 24, align: "center", lineHeight: 20, lineHeightMode: "fixed", bold: true, italic: true, underline: true, outline: true, shadow: true, script: "subscript", fill: "#0000ff" }];
  assert.equal(evaluateDocumentTextProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentTextProperties(bytes, [{ ...expected[0], bold: false }]).passed, false);
  assert.equal(evaluateDocumentTextProperties(bytes, [{ ...expected[0], fill: "#ff0000" }]).passed, false);
  assert.equal(evaluateDocumentTextProperties(bytes, [{ ...expected[0], lineHeight: 12 }]).passed, false);
  assert.equal(evaluateDocumentTextProperties(bytes, [{ ...expected[0], script: "superscript" }]).passed, false);

  const staleSourceRuns = Buffer.from(JSON.stringify({
    styles: { style_obj_text_1_color: { fill: "#0000ff" } },
    entities: {
      scene: [{
        id: "obj_text_1",
        type: "text",
        styleRef: "style_obj_text_1_color",
        payload: {
          text: "ChemSema H2O",
          fontFamily: "Times New Roman",
          fontSize: 24,
          align: "center",
          runs: [{ text: "ChemSema H2O", fontFamily: "Times New Roman", fontSize: 24, fontWeight: 700, fontStyle: "italic", underline: true, outline: true, shadow: true, script: "subscript", fill: "#0000ff" }],
          sourceRuns: [{ text: "ChemSema H2O", fontFamily: "Arial", fontSize: 18, fontWeight: 700, fontStyle: "italic", underline: true, outline: true, shadow: true, script: "subscript", fill: "#000000" }],
        },
      }],
    },
  }));
  assert.equal(evaluateDocumentTextProperties(staleSourceRuns, expected).passed, false);
});

test("saved-document shape oracle checks exact kind and public style properties", () => {
  const bytes = Buffer.from(JSON.stringify({
    styles: {
      style_shape_1: {
        kind: "shape",
        fill: null,
        stroke: "#000000",
        strokeWidth: 1,
        dashArray: [],
        shadow: true,
        shadowSize: 4,
      },
    },
    entities: {
      scene: [{
        id: "obj_shape_1",
        type: "shape",
        styleRef: "style_shape_1",
        payload: { kind: "circle" },
      }],
    },
  }));
  const expected = [{ id: "obj_shape_1", kind: "circle", fill: null, stroke: "#000000", strokeWidth: 1, dashArray: [], shaded: false, shadow: true, shadowSize: 4 }];
  assert.equal(evaluateDocumentShapeProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentShapeProperties(bytes, [{ ...expected[0], kind: "ellipse" }]).passed, false);
  assert.equal(evaluateDocumentShapeProperties(bytes, [{ ...expected[0], dashArray: [4] }]).passed, false);
  assert.equal(evaluateDocumentShapeProperties(bytes, [{ ...expected[0], shadow: false }]).passed, false);
});

test("saved-document symbol oracle checks exact kind and both persisted color surfaces", () => {
  const bytes = Buffer.from(JSON.stringify({
    styles: { style_obj_symbol_1_color: { kind: "symbol", fill: "#008000" } },
    entities: { scene: [{
      id: "obj_symbol_1",
      type: "symbol",
      styleRef: "style_obj_symbol_1_color",
      payload: { kind: "circle-plus", fill: "#008000", symbolStyle: "default" },
    }] },
  }));
  const expected = [{ id: "obj_symbol_1", kind: "circle-plus", payloadFill: "#008000", styleFill: "#008000", styleKind: "symbol", symbolStyle: "default" }];
  assert.equal(evaluateDocumentSymbolProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentSymbolProperties(bytes, [{ ...expected[0], kind: "plus" }]).passed, false);
  assert.equal(evaluateDocumentSymbolProperties(bytes, [{ ...expected[0], payloadFill: "#000000" }]).passed, false);
  assert.equal(evaluateDocumentSymbolProperties(bytes, [{ ...expected[0], styleFill: "#000000" }]).passed, false);
});

test("saved-document validation is chemical only when a nonempty molecular graph exists", () => {
  const arrowOnly = Buffer.from(JSON.stringify({
    resources: {
      mol_editor: { type: "molecule_fragment2d", data: { nodes: [], bonds: [] } },
    },
    entities: { scene: [{ id: "obj_line_1", type: "line" }] },
  }));
  const molecule = Buffer.from(JSON.stringify({
    resources: {
      mol_editor: { type: "molecule_fragment2d", data: { nodes: [{ id: "n1", element: "C" }], bonds: [] } },
    },
  }));
  assert.equal(validationLevelForDocumentBytes(arrowOnly), "structural");
  assert.equal(validationLevelForDocumentBytes(molecule), "chemical");
  assert.equal(validationLevelForDocumentBytes(Buffer.from("not-json")), "structural");
});
