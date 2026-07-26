import assert from "node:assert/strict";
import test from "node:test";
import { parseEngineJson } from "../../viewer/engine_bridge.js";

test("optional engine JSON results return their declared empty value without warning", () => {
  const originalWarn = console.warn;
  const warnings = [];
  console.warn = (...args) => warnings.push(args);
  try {
    assert.equal(parseEngineJson(undefined, null), null);
    assert.deepEqual(parseEngineJson(null, []), []);
    assert.deepEqual(parseEngineJson("", {}), {});
    assert.deepEqual(parseEngineJson('{"ok":true}', null), { ok: true });
  } finally {
    console.warn = originalWarn;
  }
  assert.deepEqual(warnings, []);
});
