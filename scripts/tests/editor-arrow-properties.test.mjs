import assert from "node:assert/strict";
import test from "node:test";
import { createEditorRuntimeHost } from "../../viewer/editor_runtime_host.js";

test("arrow property edits send only the requested fields through the shared command path", async () => {
  const calls = [];
  const patch = { headSize: "large" };
  const state = {
    editorEngine: {
      applyArrowStylePatchToSelection: (json) => {
        calls.push({ wasmPatch: JSON.parse(json) });
        return true;
      },
    },
  };
  const host = createEditorRuntimeHost({
    state,
    editorState: {},
    isEditingRustDocument: () => true,
    commandEngine: {
      executeEngineCommand: async (command, apply) => {
        calls.push({ command });
        return { changed: !!apply(), command: { type: command.type } };
      },
    },
    renderDocumentChange: (result) => calls.push({ rendered: result.command.type }),
  });

  assert.equal(await host.applyArrowOptionsToSelection(patch), true);
  assert.deepEqual(calls, [
    {
      command: {
        type: "apply-arrow-style-patch",
        payload: { changes: { headSize: "large" } },
      },
    },
    { wasmPatch: { headSize: "large" } },
    { rendered: "apply-arrow-style-patch" },
  ]);
});
