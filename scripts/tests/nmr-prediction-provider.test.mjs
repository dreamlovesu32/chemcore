import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import initNmr, {
  capabilities_json as capabilitiesJson,
  predict_json as predictJson,
} from "../../viewer/nmr-engine/chemsema_nmr.js";
import { createBundledNmrProvider } from "../../viewer/nmr_prediction_provider.js";

const wasmPath = new URL("../../viewer/nmr-engine/chemsema_nmr_bg.wasm", import.meta.url);

function methaneRequest() {
  return {
    schema: "chemsema.nmr-prediction-request.v1",
    molecule: {
      id: "methane",
      atoms: [{
        id: "c1",
        atomicNumber: 6,
        formalCharge: 0,
        radicalElectrons: 0,
        implicitHydrogens: 4,
      }],
      bonds: [],
    },
    nucleus: "1H",
    conditions: {
      solvent: "CDCl3",
      frequencyMHz: 300,
      temperatureKelvin: 298.15,
    },
  };
}

test("bundled NMR provider initializes once and preserves structured requests", async () => {
  let initializationCount = 0;
  const provider = createBundledNmrProvider({
    initialize: async () => {
      initializationCount += 1;
    },
    capabilitiesJson: () => JSON.stringify({ schema: "test.capabilities" }),
    predictJson: (requestJson) => JSON.stringify({
      schema: "test.response",
      moleculeId: JSON.parse(requestJson).molecule.id,
    }),
  });

  assert.equal((await provider.capabilities()).schema, "test.capabilities");
  assert.equal((await provider.predict(methaneRequest())).moleculeId, "methane");
  assert.equal(initializationCount, 1);
});

test("tracked WASM provider predicts a real molecule with response v2", async () => {
  const wasm = await readFile(wasmPath);
  const provider = createBundledNmrProvider({
    initialize: () => initNmr({ module_or_path: wasm }),
    capabilitiesJson,
    predictJson,
  });

  const capabilities = await provider.capabilities();
  assert.equal(capabilities.responseSchema, "chemsema.nmr-prediction-response.v2");

  const response = await provider.predict(methaneRequest());
  assert.equal(response.schema, "chemsema.nmr-prediction-response.v2");
  assert.equal(response.status, "complete");
  assert.equal(response.assignments.length, 1);
  assert.equal(response.assignments[0].integral, 4);
  assert.ok(response.assignments[0].confidenceReason);
});
