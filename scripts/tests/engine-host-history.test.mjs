import assert from "node:assert/strict";
import test from "node:test";
import { TauriEngineSession } from "../../viewer/engine_host.js";

function historySession(method, result) {
  const calls = [];
  const session = Object.create(TauriEngineSession.prototype);
  session.layoutEngine = { [method]: () => { calls.push(`local:${method}`); return result; } };
  session.syncLocalMutationState = (options) => calls.push(["sync", options]);
  session.runNativeMutationInBackground = (command) => calls.push(["native", command]);
  session.invokeMutation = (command) => calls.push(["fallback", command]);
  return { session, calls };
}

for (const [method, nativeCommand] of [["undo", "desktop_engine_undo"], ["redo", "desktop_engine_redo"]]) {
  test(`desktop hybrid ${method} mutates the visible local engine before mirroring native state`, () => {
    const { session, calls } = historySession(method, true);
    assert.equal(session[method](), true);
    assert.deepEqual(calls, [
      `local:${method}`,
      ["sync", { dirtyExports: true }],
      ["native", nativeCommand],
    ]);
  });
}

test("native-only history retains the synchronous mutation path", () => {
  const session = Object.create(TauriEngineSession.prototype);
  session.layoutEngine = null;
  session.invokeMutation = (command) => command;
  assert.equal(session.undo(), "desktop_engine_undo");
  assert.equal(session.redo(), "desktop_engine_redo");
});
