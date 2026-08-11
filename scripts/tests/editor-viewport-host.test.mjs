import assert from "node:assert/strict";
import test from "node:test";
import { hasUsableEditorContentBounds, visibleWorldSizeForDimensions } from "../../viewer/editor_viewport_host.js";

test("blank or invalid render bounds cannot collapse the editor viewport", () => {
  assert.equal(hasUsableEditorContentBounds(null), false);
  assert.equal(hasUsableEditorContentBounds({ minX: 0, minY: 0, maxX: 0, maxY: 0 }), false);
  assert.equal(hasUsableEditorContentBounds({ minX: 0, minY: 0, maxX: Number.NaN, maxY: 10 }), false);
});

test("one-dimensional and rectangular content remain valid fit targets", () => {
  assert.equal(hasUsableEditorContentBounds({ minX: 10, minY: 10, maxX: 10, maxY: 40 }), true);
  assert.equal(hasUsableEditorContentBounds({ minX: 10, minY: 10, maxX: 40, maxY: 10 }), true);
  assert.equal(hasUsableEditorContentBounds({ minX: 10, minY: 10, maxX: 40, maxY: 50 }), true);
});

test("pre-layout zero-sized containers use the full default workspace", () => {
  assert.deepEqual(visibleWorldSizeForDimensions(0, 0, 4 / 3), { width: 900, height: 600 });
  assert.deepEqual(visibleWorldSizeForDimensions(1028, 779, 1), { width: 1028, height: 779 });
  assert.deepEqual(visibleWorldSizeForDimensions(1028, 779, 2), { width: 514, height: 389.5 });
});
