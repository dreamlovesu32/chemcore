import assert from "node:assert/strict";
import test from "node:test";
import { evaluateDocumentArrowProperties, evaluateDocumentBondProperties, evaluateDocumentBracketProperties, evaluateDocumentChromatographyProperties, evaluateDocumentNodeProperties, evaluateDocumentOrbitalProperties, evaluateDocumentReports, evaluateDocumentShapeProperties, evaluateDocumentSymbolProperties, evaluateDocumentTableProperties, evaluateDocumentTextProperties, validationLevelForDocumentBytes } from "../src/oracles/document-file.mjs";

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

test("document counts reject collapsing disconnected GUI fragments into one molecule", () => {
  const disconnectedReports = {
    inspect: {
      summary: {
        format: { name: "chemsema", version: "0.2" },
        counts: { nodes: 20, bonds: 10, molecules: 10, objects: 10 },
      },
    },
    validation: { schema: "chemsema.validation-report.v1", ok: true, issues: [] },
  };
  const exact = { nodes: 20, bonds: 10, molecules: 10, objects: 10 };
  assert.equal(evaluateDocumentReports(disconnectedReports, exact).passed, true);
  assert.equal(evaluateDocumentReports(disconnectedReports, { ...exact, molecules: 1, objects: 1 }).passed, false);
});

test("saved-document bond oracle kills order, line-style, weight, and stereo mutants", () => {
  const bytes = Buffer.from(JSON.stringify({
    resources: {
      mol: {
        type: "molecule_fragment2d",
        data: {
          bonds: [
            { id: "b_double", order: 2, lineStyles: { main: "solid", left: "solid", right: "solid" }, lineWeights: { main: "normal" } },
            { id: "b_hash", order: 1, lineStyles: { main: "hash", left: "solid", right: "solid" }, lineWeights: { main: "normal" } },
            { id: "b_wedge", order: 1, lineStyles: { main: "solid", left: "solid", right: "solid" }, lineWeights: { main: "normal" }, stereo: { kind: "solid-wedge", wideEnd: "end" } },
          ],
        },
      },
    },
  }));
  const expected = [
    { id: "b_double", order: 2, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null },
    { id: "b_hash", order: 1, mainLineStyle: "hash", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: null, wideEnd: null },
    { id: "b_wedge", order: 1, mainLineStyle: "solid", leftLineStyle: "solid", rightLineStyle: "solid", mainLineWeight: "normal", stereoKind: "solid-wedge", wideEnd: "end" },
  ];
  assert.equal(evaluateDocumentBondProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentBondProperties(bytes, [{ ...expected[0], order: 1 }]).passed, false);
  assert.equal(evaluateDocumentBondProperties(bytes, [{ ...expected[1], mainLineStyle: "dashed" }]).passed, false);
  assert.equal(evaluateDocumentBondProperties(bytes, [{ ...expected[2], stereoKind: "hashed-wedge" }]).passed, false);
  assert.equal(evaluateDocumentBondProperties(bytes, [{ ...expected[2], wideEnd: "begin" }]).passed, false);
});

test("saved-document node oracle kills element, atomic-number, charge, and label mutants", () => {
  const bytes = Buffer.from(JSON.stringify({ resources: { mol: { type: "molecule_fragment2d", data: { nodes: [
    { id: "n_2", element: "N", atomicNumber: 7, charge: 0, label: { text: "N", sourceText: "N" } },
  ] } } } }));
  const expected = [{ id: "n_2", element: "N", atomicNumber: 7, charge: 0, labelText: "N", labelSourceText: "N" }];
  assert.equal(evaluateDocumentNodeProperties(bytes, expected).passed, true);
  for (const mutant of [
    { element: "C" }, { atomicNumber: 6 }, { charge: 1 }, { labelText: "C" }, { labelSourceText: "C" },
  ]) {
    assert.equal(evaluateDocumentNodeProperties(bytes, [{ ...expected[0], ...mutant }]).passed, false);
  }
  assert.equal(evaluateDocumentNodeProperties(bytes, [{ ...expected[0], id: "n_missing" }]).passed, false);
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

test("saved-document orbital oracle checks exact properties and template-compatible geometry", () => {
  const document = {
    styles: { style_orbital_1: { kind: "shape", fill: "#000000", stroke: null, strokeWidth: 1, dashArray: [] } },
    entities: { scene: [{
      id: "obj_shape_orbital_1",
      type: "shape",
      styleRef: "style_orbital_1",
      payload: {
        kind: "orbital",
        orbitalTemplate: "dz2",
        orbitalStyle: "filled",
        orbitalPhase: "plus",
        orbitalColor: "#000000",
        axisStart: [30, 40],
        axisEnd: [30, 70],
        bbox: [7.5, 17.5, 45, 75],
        angle: 90,
      },
    }] },
  };
  const bytes = Buffer.from(JSON.stringify(document));
  const expected = [{ id: "obj_shape_orbital_1", kind: "orbital", template: "dz2", orbitalStyle: "filled", phase: "plus", color: "#000000", geometryValid: true, axisLength: 30, bboxSize: [45, 75], angle: 90, fill: "#000000", stroke: null, strokeWidth: 1, dashArray: [], shaded: false }];
  assert.equal(evaluateDocumentOrbitalProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentOrbitalProperties(bytes, [{ ...expected[0], phase: "minus" }]).passed, false);
  const zeroAxis = Buffer.from(JSON.stringify({ ...document, entities: { scene: [{ ...document.entities.scene[0], payload: { ...document.entities.scene[0].payload, axisEnd: [30, 40] } }] } }));
  assert.equal(evaluateDocumentOrbitalProperties(zeroAxis, expected).passed, false);
  const staleEllipseGeometry = Buffer.from(JSON.stringify({ ...document, entities: { scene: [{ ...document.entities.scene[0], payload: { ...document.entities.scene[0].payload, center: [30, 40] } }] } }));
  assert.equal(evaluateDocumentOrbitalProperties(staleEllipseGeometry, expected).passed, false);
  const wrongVisualEnvelope = Buffer.from(JSON.stringify({ ...document, entities: { scene: [{ ...document.entities.scene[0], payload: { ...document.entities.scene[0].payload, bbox: [7.5, 17.5, 44, 75] } }] } }));
  assert.equal(evaluateDocumentOrbitalProperties(wrongVisualEnvelope, expected).passed, false);
});

test("saved-document chromatography oracle checks TLC and gel marks plus internal colors", () => {
  const bytes = Buffer.from(JSON.stringify({
    styles: { tlc_style: { stroke: "#0000ff", fill: null }, gel_style: { stroke: "#0000ff", fill: null } },
    entities: { scene: [
      { id: "obj_shape_1", type: "shape", styleRef: "tlc_style", payload: { kind: "tlcPlate", showOrigin: true, showSolventFront: true, showBorders: true, showSideTicks: true, lanes: [
        { offset: 0.25, spots: [{ rf: 0.15, color: "#0000ff" }] },
        { offset: 0.75, spots: [{ rf: 0.5, color: "#0000ff" }] },
      ] } },
      { id: "obj_shape_2", type: "shape", styleRef: "gel_style", payload: { kind: "gelPlate", gelElectrophoresis: { color: "#0000ff", lanes: [
        { bands: [{ value: 0.5, color: "#0000ff" }] },
        { bands: [{ value: 0.75, color: "#0000ff" }] },
      ] } } },
    ] },
  }));
  const expected = [
    { id: "obj_shape_1", kind: "tlcPlate", laneCount: 2, firstMarkValues: [0.15, 0.5], color: "#0000ff", showOrigin: true, showSolventFront: true, showBorders: true, showSideTicks: true },
    { id: "obj_shape_2", kind: "gelPlate", laneCount: 2, firstMarkValues: [0.5, 0.75], color: "#0000ff" },
  ];
  assert.equal(evaluateDocumentChromatographyProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentChromatographyProperties(bytes, [{ ...expected[0], firstMarkValues: [0.15, 0.6] }, expected[1]]).passed, false);
  const staleBand = Buffer.from(bytes.toString("utf8").replace('"value":0.75,"color":"#0000ff"', '"value":0.75,"color":"#ff0000"'));
  assert.equal(evaluateDocumentChromatographyProperties(staleBand, expected).passed, false);
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

test("saved-document bracket oracle checks paired group membership and visible side properties", () => {
  const bytes = Buffer.from(JSON.stringify({
    entities: { scene: [{
      id: "obj_bracket_1",
      type: "group",
    },
    { id: "obj_bracket_1_left", type: "bracket", payload: { kind: "curly", side: "left", stroke: "#008000" } },
    { id: "obj_bracket_1_right", type: "bracket", payload: { kind: "curly", side: "right", stroke: "#008000" } }] },
    hierarchy: { roots: ["obj_bracket_1"], children: { obj_bracket_1: ["obj_bracket_1_left", "obj_bracket_1_right"] } },
  }));
  const expected = [{ id: "obj_bracket_1", children: [
    { id: "obj_bracket_1_left", kind: "curly", side: "left", stroke: "#008000" },
    { id: "obj_bracket_1_right", kind: "curly", side: "right", stroke: "#008000" },
  ] }];
  assert.equal(evaluateDocumentBracketProperties(bytes, expected).passed, true);
  const wrongHierarchy = Buffer.from(JSON.stringify({
    ...JSON.parse(bytes.toString("utf8")),
    hierarchy: { roots: ["obj_bracket_1"], children: { obj_bracket_1: ["obj_bracket_1_right", "obj_bracket_1_left"] } },
  }));
  assert.equal(evaluateDocumentBracketProperties(wrongHierarchy, expected).passed, false);
  assert.equal(evaluateDocumentBracketProperties(bytes, [{ ...expected[0], children: [{ ...expected[0].children[0], side: "right" }, expected[0].children[1]] }]).passed, false);
  assert.equal(evaluateDocumentBracketProperties(bytes, [{ ...expected[0], children: [{ ...expected[0].children[0], stroke: "#000000" }, expected[0].children[1]] }]).passed, false);
});

test("saved-document table oracle checks structure, unique cells, guides, alignment, and borders", () => {
  const border = { visible: true, lineStyle: "dashed", width: 1.5, color: "#000000" };
  const bytes = Buffer.from(JSON.stringify({
    entities: { scene: [{
      id: "obj_table_1",
      type: "table",
      payload: { table: {
        rows: 2,
        columns: 2,
        rowGuides: [0, 40, 80],
        columnGuides: [0, 60, 120],
        cells: [
          { id: "c00", row: 0, column: 0, horizontalAlignment: "right", verticalAlignment: "bottom", borders: { top: border, left: border, bottom: border, right: border } },
          { id: "c01", row: 0, column: 1, horizontalAlignment: "left", verticalAlignment: "middle", borders: {} },
          { id: "c10", row: 1, column: 0, horizontalAlignment: "left", verticalAlignment: "middle", borders: {} },
          { id: "c11", row: 1, column: 1, horizontalAlignment: "left", verticalAlignment: "middle", borders: {} },
        ],
      } },
    }] },
  }));
  const expected = [{ id: "obj_table_1", rows: 2, columns: 2, cells: [{ row: 0, column: 0, horizontalAlignment: "right", verticalAlignment: "bottom", borders: { top: border, left: border, bottom: border, right: border } }] }];
  assert.equal(evaluateDocumentTableProperties(bytes, expected).passed, true);
  assert.equal(evaluateDocumentTableProperties(bytes, [{ ...expected[0], columns: 3 }]).passed, false);
  const duplicateIds = Buffer.from(bytes.toString("utf8").replace('"id":"c11"', '"id":"c10"'));
  assert.equal(evaluateDocumentTableProperties(duplicateIds, expected).passed, false);
  assert.equal(evaluateDocumentTableProperties(bytes, [{ ...expected[0], cells: [{ ...expected[0].cells[0], horizontalAlignment: "center" }] }]).passed, false);
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
