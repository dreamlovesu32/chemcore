import assert from "node:assert/strict";
import test from "node:test";

import { createEditorPointerController } from "../../viewer/editor_pointer_controller.js";

function controllerFixture(activeTool = "select") {
  const calls = [];
  const engine = {
    async selectComponentAtPoint(x, y, additive) {
      calls.push(["group", x, y, additive]);
      return true;
    },
    async selectLinkedAtPoint(x, y, additive) {
      calls.push(["link", x, y, additive]);
      return true;
    },
  };
  let rendered = 0;
  let prevented = 0;
  const controller = createEditorPointerController({
    routeEditorPointerEvents: () => true,
    editorState: () => ({ activeTool }),
    svgPointFromEvent: () => ({ x: 12, y: 34 }),
    state: () => ({ editorEngine: engine }),
    setActiveSelectionGesture: () => {},
    renderSelectionOnlyUpdate: async () => {
      rendered += 1;
    },
  });
  const event = (altKey, shiftKey = false) => ({
    altKey,
    shiftKey,
    preventDefault() {
      prevented += 1;
    },
  });
  return {
    calls,
    controller,
    event,
    rendered: () => rendered,
    prevented: () => prevented,
  };
}

test("ordinary double-click selects the enclosing group", async () => {
  const fixture = controllerFixture();
  await fixture.controller.handleEditorDoubleClick(fixture.event(false, true));
  assert.deepEqual(fixture.calls, [["group", 12, 34, true]]);
  assert.equal(fixture.rendered(), 1);
  assert.equal(fixture.prevented(), 1);
});

test("Alt+double-click selects the typed Link component", async () => {
  const fixture = controllerFixture();
  await fixture.controller.handleEditorDoubleClick(fixture.event(true));
  assert.deepEqual(fixture.calls, [["link", 12, 34, false]]);
  assert.equal(fixture.rendered(), 1);
  assert.equal(fixture.prevented(), 1);
});

test("double-click Link navigation is unavailable outside the select tool", async () => {
  const fixture = controllerFixture("bond");
  await fixture.controller.handleEditorDoubleClick(fixture.event(true));
  assert.deepEqual(fixture.calls, []);
  assert.equal(fixture.rendered(), 0);
  assert.equal(fixture.prevented(), 0);
});
