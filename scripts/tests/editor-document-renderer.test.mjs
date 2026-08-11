import assert from "node:assert/strict";
import test from "node:test";

import { selectedUnlockedDocumentPreviewObjectIds } from "../../viewer/editor_document_renderer.js";

test("document move preview excludes objects locked directly or by an ancestor", () => {
  const document = {
    objects: [
      { id: "editable", locked: false, children: [] },
      { id: "locked", locked: true, children: [] },
      {
        id: "locked-group",
        locked: true,
        children: [{ id: "locked-child", locked: false, children: [] }],
      },
    ],
  };
  const selection = {
    arrowObjects: ["editable", "locked", "locked-child"],
    textObjects: ["editable"],
  };

  assert.deepEqual(
    selectedUnlockedDocumentPreviewObjectIds(document, selection),
    ["editable"],
  );
});

test("document move preview preserves selected objects absent from a stale document snapshot", () => {
  assert.deepEqual(
    selectedUnlockedDocumentPreviewObjectIds(
      { objects: [] },
      { arrow_objects: ["not-yet-synced"] },
    ),
    ["not-yet-synced"],
  );
});
