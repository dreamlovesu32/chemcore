import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import initEngine, { WasmEngine } from "../../viewer/engine/chemsema_engine.js";
import initNmr, {
  capabilities_json as capabilitiesJson,
  predict_json as predictJson,
} from "../../viewer/nmr-engine/chemsema_nmr.js";
import { createNmrPredictionHost } from "../../viewer/nmr_prediction_host.js";
import { createBundledNmrProvider } from "../../viewer/nmr_prediction_provider.js";

const ETHANE_CDXML = `<?xml version="1.0" encoding="UTF-8"?>
<CDXML CreationProgram="ChemSema NMR test" BoundingBox="0 0 120 80"
 FractionalWidths="yes" InterpretChemically="yes" LabelFont="3" LabelSize="10"
 CaptionFont="3" CaptionSize="10" BondLength="14.4" LineWidth="0.6"
 BoldWidth="2" HashSpacing="2.5" MarginWidth="1.6">
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 120 80">
    <fragment id="2">
      <n id="3" p="45 40"/>
      <n id="4" p="75 40"/>
      <b id="5" B="3" E="4" Order="1"/>
    </fragment>
  </page>
</CDXML>`;

test("real ChemSema and NMR WASM modules generate a native result tab", async () => {
  const [engineWasm, nmrWasm] = await Promise.all([
    readFile(new URL("../../viewer/engine/chemsema_engine_bg.wasm", import.meta.url)),
    readFile(new URL("../../viewer/nmr-engine/chemsema_nmr_bg.wasm", import.meta.url)),
  ]);
  await initEngine({ module_or_path: engineWasm });

  const engine = new WasmEngine();
  engine.loadDocumentCdxml(ETHANE_CDXML);
  assert.equal(engine.selectAll(), true);

  const provider = createBundledNmrProvider({
    initialize: () => initNmr({ module_or_path: nmrWasm }),
    capabilitiesJson,
    predictJson,
  });
  const opened = [];
  const host = createNmrPredictionHost({
    engine: () => engine,
    provider: () => provider,
    openDocumentTab: async (documentData, title) => {
      opened.push({ documentData, title });
    },
  });

  assert.equal(await host.predict("1H"), true);
  assert.equal(opened.length, 1);
  assert.equal(opened[0].title, "ChemNMR 1H Estimation");
  assert.equal(opened[0].documentData.document.meta.kind, "nmr-prediction-result");
  assert.ok(opened[0].documentData.objects.some((object) => object.type === "spectrum"));
  assert.equal(await host.predict("13C"), true);
  assert.equal(opened.length, 2);
  assert.equal(opened[1].title, "ChemNMR 13C Estimation");
  assert.equal(opened[1].documentData.document.meta.prediction.nucleus, "13C");
  assert.ok(opened[1].documentData.objects.some((object) => object.type === "spectrum"));
  engine.free();
});
