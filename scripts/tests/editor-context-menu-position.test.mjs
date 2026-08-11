import assert from "node:assert/strict";
import test from "node:test";
import {
  contextMenuShouldRestoreCanvasFocus,
  contextMenuViewportBounds,
  contextSubmenuPlacement,
} from "../../viewer/editor_context_menu.js";

test("canvas context menus restore focus only when no interactive owner remains", () => {
  const body = {};
  const documentElement = {};
  const documentRoot = { body, documentElement };
  assert.equal(contextMenuShouldRestoreCanvasFocus(null, documentRoot), true);
  assert.equal(contextMenuShouldRestoreCanvasFocus(body, documentRoot), true);
  assert.equal(contextMenuShouldRestoreCanvasFocus(documentElement, documentRoot), true);
  assert.equal(contextMenuShouldRestoreCanvasFocus({}, documentRoot), false);
});

test("context menus use the Windows work area instead of the obscured viewport edge", () => {
  assert.deepEqual(contextMenuViewportBounds({
    innerWidth: 1028,
    innerHeight: 779,
    screenX: 0,
    screenY: 0,
    availLeft: 0,
    availTop: 0,
    availWidth: 1028,
    availHeight: 731,
  }), {
    left: 0,
    top: 0,
    right: 1028,
    bottom: 731,
  });
});

test("a tall submenu shifts above the taskbar work-area boundary", () => {
  const placement = contextSubmenuPlacement(
    { left: 360, top: 620, right: 570, bottom: 648 },
    { left: 568, top: 616, right: 774, bottom: 760 },
    { left: 0, top: 0, right: 1028, bottom: 731 },
  );
  assert.equal(placement.offsetTop, -39);
  assert.equal(placement.openLeft, false);
});

test("a submenu flips left when it would cross the work-area right edge", () => {
  const placement = contextSubmenuPlacement(
    { left: 830, top: 100, right: 1020, bottom: 128 },
    { left: 1018, top: 96, right: 1224, bottom: 240 },
    { left: 0, top: 0, right: 1028, bottom: 731 },
  );
  assert.equal(placement.openLeft, true);
});
