import fs from "node:fs";
import path from "node:path";
import { initSync, WasmEngine } from "../viewer/engine/chemcore_engine.js";

const repoRoot = process.cwd();
const wasmBytes = fs.readFileSync(path.join(repoRoot, "viewer/engine/chemcore_engine_bg.wasm"));
initSync({ module: wasmBytes });

const defaultFiles = [
  "tmp/rest.cdxml",
  "tmp/dash.cdxml",
  "tmp/dash-acs.cdxml",
  "tmp/orbital.cdxml",
  "tmp/orbital-acs.cdxml",
];

const files = process.argv.slice(2);
const inputs = files.length ? files : defaultFiles;

function collectSceneObjects(objects, out = []) {
  for (const object of objects || []) {
    out.push({
      id: object.id,
      type: object.type,
      kind: object.payload?.kind || null,
      metaSource: object.meta?.source || null,
    });
    collectSceneObjects(object.children, out);
  }
  return out;
}

let failureCount = 0;

for (const relativePath of inputs) {
  const absolutePath = path.resolve(repoRoot, relativePath);
  if (!fs.existsSync(absolutePath)) {
    console.error(`[missing] ${relativePath}`);
    failureCount += 1;
    continue;
  }

  const cdxml = fs.readFileSync(absolutePath, "utf8");
  const engine = new WasmEngine();
  engine.loadDocumentCdxml(cdxml);

  const documentData = JSON.parse(engine.documentJson());
  const renderList = JSON.parse(engine.renderListJson());
  const primitiveObjectIds = new Set(renderList.map((primitive) => primitive.objectId).filter(Boolean));
  const sceneObjects = collectSceneObjects(documentData.objects);
  const missingObjects = sceneObjects.filter((object) => object.type !== "group" && !primitiveObjectIds.has(object.id));

  if (missingObjects.length) {
    failureCount += 1;
    console.error(`\n[missing-core-primitives] ${relativePath}`);
    for (const object of missingObjects) {
      console.error(`  - ${object.id} (${object.type}${object.kind ? `/${object.kind}` : ""})`);
    }
    continue;
  }

  console.log(`[ok] ${relativePath} :: ${sceneObjects.length} objects, ${primitiveObjectIds.size} primitive-backed`);
}

process.exitCode = failureCount ? 1 : 0;
