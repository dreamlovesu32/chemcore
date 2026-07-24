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

const CHIRAL_HYDROXYMETHYL_CDXML = `<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 300 220">
    <fragment id="7">
      <n id="8" p="40 80"/>
      <n id="9" p="80 55"/>
      <n id="10" p="120 80"/>
      <n id="11" p="160 55"/>
      <n id="12" p="120 125"/>
      <n id="13" p="160 150" Element="8"/>
      <b id="21" B="8" E="9" Order="1"/>
      <b id="22" B="9" E="10" Order="1"/>
      <b id="23" B="10" E="11" Order="1" Display="WedgeBegin"/>
      <b id="24" B="10" E="12" Order="1"/>
      <b id="25" B="12" E="13" Order="1"/>
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
  const nativeGraph = JSON.parse(engine.chemicalGraphV2Json());
  assert.equal(nativeGraph.schema, "chemsema-nomenclature/chemical-graph/2");
  assert.equal(nativeGraph.semantics.normalization, "chemsema-chemical-graph-normalization/1");
  assert.equal("position" in nativeGraph.atoms[0], false);
  const nomenclatureRequest = JSON.parse(engine.nomenclatureRequestJson());
  assert.equal(nomenclatureRequest.schema, "chemsema.nomenclature-request.v1");
  assert.deepEqual(nomenclatureRequest.graph, nativeGraph);
  const nmrRequest = JSON.parse(engine.nmrPredictionRequestJson("1H"));
  assert.deepEqual(nmrRequest.graph, nativeGraph);

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

test("real WASM path preserves V2 stereo and splits the reviewed adjacent polar CH2", async () => {
  const [engineWasm, nmrWasm] = await Promise.all([
    readFile(new URL("../../viewer/engine/chemsema_engine_bg.wasm", import.meta.url)),
    readFile(new URL("../../viewer/nmr-engine/chemsema_nmr_bg.wasm", import.meta.url)),
  ]);
  await initEngine({ module_or_path: engineWasm });
  await initNmr({ module_or_path: nmrWasm });
  const engine = new WasmEngine();
  engine.loadDocumentCdxml(CHIRAL_HYDROXYMETHYL_CDXML);
  assert.equal(engine.selectAll(), true);

  const request = JSON.parse(engine.nmrPredictionRequestJson("1H"));
  assert.equal(request.schema, "chemsema.nmr-prediction-request.v2");
  assert.equal(request.graph.schema, "chemsema-nomenclature/chemical-graph/2");
  assert.equal(request.graph.stereo.filter((item) => item.kind === "tetrahedral").length, 1);
  assert.equal(request.assignedCipDescriptors.length, 1);

  const response = JSON.parse(predictJson(JSON.stringify(request)));
  const hydroxymethylSites = response.assignments.filter(
    (assignment) => assignment.atomIds.length === 1 && assignment.atomIds[0] === "12",
  );
  assert.deepEqual(
    hydroxymethylSites.map((assignment) => assignment.siteIds[0]).sort(),
    ["h:12:diastereotopic-a", "h:12:diastereotopic-b"],
  );
  assert.ok(response.couplings.some(
    (coupling) => coupling.atomIds[0] === "12"
      && coupling.atomIds[1] === "12"
      && coupling.valueHz === -12.4,
  ));
  engine.free();
});
