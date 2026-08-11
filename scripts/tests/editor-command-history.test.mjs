import assert from "node:assert/strict";
import test from "node:test";
import { createEditorCommandController } from "../../viewer/editor_command_controller.js";

for (const command of ["undo", "redo"]) {
  test(`${command} uses the versioned command pipeline instead of a stale direct mutation result`, async () => {
    const calls = [];
    const editorEngine = {
      [command]: () => { throw new Error(`direct ${command} must not run`); },
    };
    const controller = createEditorCommandController({
      state: () => ({ editorEngine }),
      isEditingRustDocument: () => true,
      commandEngine: {
        executeEngineCommand: async (receivedCommand, apply) => {
          calls.push({ receivedCommand, apply });
          return { changed: true, command: { type: receivedCommand } };
        },
      },
      renderDocumentChange: (result) => calls.push({ rendered: result.command.type }),
      renderDocument: () => { throw new Error("full render fallback must not run"); },
    });

    assert.equal(await controller.runEditorCommand(command), true);
    assert.equal(calls[0].receivedCommand, command);
    assert.equal(calls[0].apply, undefined);
    assert.deepEqual(calls[1], { rendered: command });
  });
}

test("history retains the direct fallback when no command engine exists", async () => {
  let undoCalls = 0;
  const controller = createEditorCommandController({
    state: () => ({ editorEngine: { undo: () => { undoCalls += 1; return true; } } }),
    isEditingRustDocument: () => true,
    syncDocumentFromEngine: async () => {},
    renderDocumentChange: () => true,
  });
  assert.equal(await controller.runEditorCommand("undo"), true);
  assert.equal(undoCalls, 1);
});

test("select all invalidates the revision-stable interaction cache before rendering", async () => {
  const calls = [];
  const controller = createEditorCommandController({
    state: () => ({ editorEngine: { selectAll: () => { calls.push("select"); return true; } } }),
    isEditingRustDocument: () => true,
    activateEditorTool: async () => calls.push("activate"),
    invalidateEditorEngineReadCache: () => calls.push("invalidate"),
    renderEditorOverlay: () => calls.push("render"),
    refreshCommandAvailability: () => calls.push("refresh"),
  });

  assert.equal(await controller.runEditorCommand("select-all"), true);
  assert.deepEqual(calls, ["activate", "select", "invalidate", "render", "refresh"]);
});
