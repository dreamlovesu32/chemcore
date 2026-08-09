import assert from "node:assert/strict";
import test from "node:test";
import { evaluateDocumentArrowProperties, evaluateDocumentReports, validationLevelForDocumentBytes } from "../src/oracles/document-file.mjs";

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
