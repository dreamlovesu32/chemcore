import assert from "node:assert/strict";
import test from "node:test";

import { createNmrPredictionHost } from "../../viewer/nmr_prediction_host.js";
import { createBrowserDocumentTabs } from "../../viewer/browser_document_tabs.js";

test("NMR host reuses the engine request and result-document adapters", async () => {
  const calls = [];
  const engine = {
    async nmrPredictionRequestJson(nucleus) {
      calls.push(["request", nucleus]);
      return JSON.stringify({ schema: "request", nucleus });
    },
    async nmrResultDocumentJson(responseJson) {
      calls.push(["document", JSON.parse(responseJson)]);
      return JSON.stringify({
        document: { title: "ChemNMR 1H Estimation" },
        objects: [],
        resources: {},
      });
    },
  };
  const opened = [];
  const host = createNmrPredictionHost({
    engine: () => engine,
    provider: () => ({
      predict(request) {
        calls.push(["predict", request]);
        return { schema: "response", nucleus: request.nucleus };
      },
    }),
    openDocumentTab(documentData, title) {
      opened.push({ documentData, title });
    },
  });

  assert.equal(await host.predict("1H"), true);
  assert.deepEqual(calls, [
    ["request", "1H"],
    ["predict", { schema: "request", nucleus: "1H" }],
    ["document", { schema: "response", nucleus: "1H" }],
  ]);
  assert.equal(opened[0].title, "ChemNMR 1H Estimation");
  assert.equal(opened[0].documentData.document.title, "ChemNMR 1H Estimation");
});

test("NMR host reports a missing predictor instead of silently substituting data", async () => {
  const host = createNmrPredictionHost({
    engine: () => ({
      nmrPredictionRequestJson: () => "{}",
      nmrResultDocumentJson: () => "{}",
    }),
    provider: () => null,
    openDocumentTab() {},
  });
  await assert.rejects(() => host.predict("13C"), /rules are not installed/i);
});

test("NMR host applies explicit prediction settings before provider execution", async () => {
  let captured = null;
  const host = createNmrPredictionHost({
    engine: () => ({
      nmrPredictionRequestJson: () => JSON.stringify({
        nucleus: "1H",
        conditions: {
          solvent: "CDCl3",
          frequencyMHz: 400,
          temperatureKelvin: 298.15,
        },
      }),
      nmrResultDocumentJson: () => "{}",
    }),
    provider: () => ({
      predict(request) {
        captured = request;
        return { schema: "response" };
      },
    }),
    openDocumentTab() {},
  });

  await host.predict("1H", {
    solvent: "DMSO-d6",
    frequencyMHz: 600,
  });
  assert.deepEqual(captured.conditions, {
    solvent: "DMSO-d6",
    frequencyMHz: 600,
    temperatureKelvin: 298.15,
  });
  await assert.rejects(
    () => host.predict("1H", { solvent: "benzene-d6" }),
    /unsupported nmr solvent/i,
  );
});

test("generated NMR result opens in a new unsaved editor tab", async () => {
  const state = {};
  const documentTabs = [{ id: "source", title: "Source" }];
  let activeId = "source";
  const host = createBrowserDocumentTabs({
    state,
    documentTabs,
    desktopFileHost: null,
    openFileInput: null,
    isDesktopShell: () => true,
    appRuntimeReady: () => Promise.resolve(),
    getActiveDocumentTabId: () => activeId,
    setActiveDocumentTabId: (value) => { activeId = value; },
    finishActiveTextEditor: async () => {},
    saveActiveDocumentTabState: () => {},
    createDocumentTab: (title) => ({ id: "generated", title }),
    restoreDocumentTabState: async () => {},
    loadJsonDocumentIntoEditor: async (documentData) => {
      state.currentDocument = documentData;
      state.unsavedDocument = false;
    },
    renderDocumentTabs: () => {},
    fitView: () => {},
    closeDocumentTab: async () => {},
    activateDocumentTab: async (id) => { activeId = id; },
  });
  const documentData = {
    document: { title: "ChemNMR 1H Estimation" },
    objects: [],
    resources: {},
  };

  await host.openGeneratedDocumentTab(documentData, "ChemNMR 1H Estimation");

  assert.equal(documentTabs.length, 2);
  assert.equal(activeId, "generated");
  assert.equal(state.currentDocument, documentData);
  assert.equal(state.unsavedDocument, true);
});
