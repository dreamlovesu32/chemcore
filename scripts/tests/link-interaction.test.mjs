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

test("a fast pointer sequence waits for asynchronous selection gesture setup", async () => {
  const previousWindow = globalThis.window;
  globalThis.window = {};
  const calls = [];
  let gesture = null;
  const engine = {
    async pointerMove(x, y) {
      calls.push(["hover", x, y]);
    },
    async beginSelectionMove(x, y) {
      calls.push(["begin-start", x, y]);
      await new Promise((resolve) => setTimeout(resolve, 20));
      calls.push(["begin-end", x, y]);
      return true;
    },
    stateJson() {
      return JSON.stringify({ selection: { arrowObjects: ["obj_line_1"] } });
    },
    async updateSelectionMove(x, y) {
      calls.push(["update", x, y]);
      return true;
    },
    async finishSelectionMove(x, y) {
      calls.push(["finish", x, y]);
      return true;
    },
    async clearInteraction() {},
  };
  const svg = {
    setPointerCapture() {},
    releasePointerCapture() {},
    querySelector() { return null; },
  };
  const options = {
    routeEditorPointerEvents: () => true,
    editorState: () => ({ activeTool: "select", selectMode: "box", elementPlacementActive: false }),
    svgPointFromEvent: (event) => event.point,
    state: () => ({ editorEngine: engine }),
    viewerSvg: () => svg,
    documentBoundsContainsPoint: () => true,
    selectionBoundsContainsPoint: () => true,
    selectionHitContainsPoint: () => true,
    selectionResizeHandleHit: () => null,
    selectionRotateHandleHit: () => null,
    parseEngineJson: (source, fallback) => source ? JSON.parse(source) : fallback,
    setActiveSelectionGesture: (next) => { gesture = next; },
    activeSelectionGesture: () => gesture,
    pointDistance: (left, right) => Math.hypot(left.x - right.x, left.y - right.y),
    cssPxToPt: (value) => value,
    syncSelectCursorForPoint: async () => {},
    renderEditorOverlay: () => {},
    currentEditorOverlayRenderList: () => [],
    applyDocumentObjectPreviewTransform: () => false,
    selectionNeedsBackendMovePreview: () => false,
    renderDocumentChange: () => true,
    renderDocument: () => {},
    syncDocumentFromEngine: async () => {},
    clearDocumentObjectPreviewTransform: () => {},
    syncCanvasCursor: () => {},
    setLastEditFocusPoint: () => {},
  };
  const event = (point, { button = 0, buttons = 1 } = {}) => ({
    point,
    button,
    buttons,
    pointerId: 1,
    altKey: false,
    shiftKey: false,
    preventDefault() {},
  });

  const controller = createEditorPointerController(options);
  const pendingDown = controller.handleEditorPointerDown(event({ x: 10, y: 10 }));
  const pendingMove = controller.handleEditorPointerMove(event({ x: 30, y: 10 }));
  const pendingUp = controller.handleEditorPointerUp(event({ x: 30, y: 10 }, { buttons: 0 }));
  await Promise.all([pendingDown, pendingMove, pendingUp]);

  assert.deepEqual(calls, [
    ["hover", 10, 10],
    ["begin-start", 10, 10],
    ["begin-end", 10, 10],
    ["update", 30, 10],
    ["finish", 30, 10],
  ]);
  if (previousWindow === undefined) {
    delete globalThis.window;
  } else {
    globalThis.window = previousWindow;
  }
});
