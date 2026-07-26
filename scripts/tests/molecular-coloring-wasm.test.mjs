import assert from "node:assert/strict";
import fs from "node:fs/promises";
import test from "node:test";

import initEngine, { WasmEngine } from "../../viewer/engine/chemsema_engine.js";

const engineModule = new WebAssembly.Module(
  await fs.readFile(new URL("../../viewer/engine/chemsema_engine_bg.wasm", import.meta.url)),
);
await initEngine({ module_or_path: engineModule });

const CYCLOHEXANE = `<CDXML BondLength="30"><page id="1"><fragment id="10">
  <n id="11" p="85 30"/><n id="12" p="124 52.5"/><n id="13" p="124 97.5"/>
  <n id="14" p="85 120"/><n id="15" p="46 97.5"/><n id="16" p="46 52.5"/>
  <b id="21" B="11" E="12"/><b id="22" B="12" E="13"/><b id="23" B="13" E="14"/>
  <b id="24" B="14" E="15"/><b id="25" B="15" E="16"/><b id="26" B="16" E="11"/>
</fragment></page></CDXML>`;

test("WASM right-click ring fill is stored and rendered in full and targeted lists", () => {
  const engine = new WasmEngine();
  try {
    engine.loadDocumentCdxml(CYCLOHEXANE);
    assert.equal(engine.selectComponentAtPoint(85, 30, false), true);

    const result = JSON.parse(engine.executeCommandJson(
      JSON.stringify({ type: "apply-ring-fill", color: "#00ffff" }),
    ));
    assert.equal(result.changed, true, `${JSON.stringify(result)}\n${engine.stateJson()}`);
    assert.match(engine.documentJson(), /"coloredAreas":\[\{/);

    const full = JSON.parse(engine.renderListJson());
    assert.ok(full.some((primitive) => (
      primitive.kind === "polygon"
      && primitive.role === "document-molecular-color"
      && primitive.fill === "#00ffff"
    )));

    const targeted = JSON.parse(engine.renderTargetsJson(JSON.stringify({
      nodes: ["13"],
      bonds: [],
      objects: [],
    })));
    assert.ok(targeted.some((primitive) => (
      primitive.kind === "polygon"
      && primitive.role === "document-molecular-color"
      && primitive.fill === "#00ffff"
      && primitive.bondId === "21"
    )));
  } finally {
    engine.free();
  }
});
