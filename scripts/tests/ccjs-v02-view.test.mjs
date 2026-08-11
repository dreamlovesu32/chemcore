import assert from "node:assert/strict";
import test from "node:test";

import {
  applyChemSemaDocumentPatch,
  canonicalChemSemaDocumentForSave,
  inflateChemSemaDocument,
  parseEngineJson,
  setChemSemaRuntimeRevision,
} from "../../viewer/engine_bridge.js";

function sampleDocument() {
  return {
    format: { name: "chemsema", version: "0.2", unit: "pt", profile: "snapshot" },
    document: { id: "doc", title: "sample", page: {} },
    entities: {
      scene: [
        { id: "child", type: "text", payload: {} },
        { id: "group", type: "group", payload: {} },
      ],
    },
    hierarchy: {
      roots: ["group"],
      children: { group: ["child"] },
    },
    relations: [{ id: "rel", kind: "sample", endpoints: [] }],
    resources: {},
  };
}

test("v0.2 document is inflated into the disposable nested editor view", () => {
  const canonical = sampleDocument();
  const view = inflateChemSemaDocument(canonical);

  assert.equal(view.objects[0].id, "group");
  assert.equal(view.objects[0].children[0].id, "child");
  assert.equal(view.links[0].id, "rel");
  assert.equal(canonical.objects, undefined);
  assert.equal(canonical.entities.scene[1].children, undefined);
});

test("generic engine JSON is unchanged", () => {
  assert.deepEqual(parseEngineJson('{"revision":3}'), { revision: 3 });
});

test("document patch updates one entity without replacing the document", () => {
  const view = inflateChemSemaDocument(sampleDocument());
  setChemSemaRuntimeRevision(view, 0);
  const resources = view.resources;
  const applied = applyChemSemaDocumentPatch(view, {
    beforeRevision: 0,
    revision: 1,
    upsertEntities: [{
      entity: { id: "child", type: "text", name: "updated", payload: {} },
      parentId: "group",
    }],
    relationScopeEntityIds: ["child"],
    relations: [],
  });

  assert.equal(applied, true);
  assert.equal(view.objects[0].children[0].name, "updated");
  assert.equal(view.entities.scene.find((entity) => entity.id === "child").name, "updated");
  assert.equal(view.__runtimeRevision, 1);
  assert.equal(view.resources, resources);
  const saved = canonicalChemSemaDocumentForSave(view);
  assert.equal(saved.objects, undefined);
  assert.equal(saved.links, undefined);
});

test("out-of-order patch is rejected without mutating the document", () => {
  const view = inflateChemSemaDocument(sampleDocument());
  setChemSemaRuntimeRevision(view, 4);
  const applied = applyChemSemaDocumentPatch(view, {
    beforeRevision: 3,
    revision: 5,
    upsertEntities: [{ entity: { id: "child", type: "text", name: "wrong", payload: {} } }],
  });
  assert.equal(applied, false);
  assert.equal(view.objects[0].children[0].name, undefined);
});
