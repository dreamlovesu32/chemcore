import assert from "node:assert/strict";
import { RuntimeGate } from "../viewer/runtime_gate.js";

class FakeElement {
  constructor() {
    this.dataset = {};
    this.attributes = new Map();
    this.hidden = false;
    this.textContent = "";
    this.listeners = new Map();
  }

  setAttribute(name, value) {
    this.attributes.set(name, value);
  }

  addEventListener(name, listener) {
    this.listeners.set(name, listener);
  }

  click() {
    this.listeners.get("click")?.();
  }
}

function fixture() {
  const elements = {
    body: new FakeElement(),
    titlebar: new FakeElement(),
    editorShell: new FakeElement(),
    gate: new FakeElement(),
    title: new FakeElement(),
    message: new FakeElement(),
    details: new FakeElement(),
    reloadButton: new FakeElement(),
  };
  let reloads = 0;
  const runtimeGate = new RuntimeGate({
    ...elements,
    reload: () => {
      reloads += 1;
    },
  });
  return { elements, runtimeGate, reloads: () => reloads };
}

{
  const { elements, runtimeGate } = fixture();
  assert.equal(elements.body.dataset.runtimeState, "loading");
  assert.equal(elements.editorShell.hidden, true);
  assert.equal(elements.titlebar.hidden, true);
  assert.equal(elements.gate.hidden, false);

  runtimeGate.ready();
  assert.equal(elements.body.dataset.runtimeState, "ready");
  assert.equal(elements.editorShell.hidden, false);
  assert.equal(elements.titlebar.hidden, false);
  assert.equal(elements.gate.hidden, true);
}

{
  const { elements, runtimeGate, reloads } = fixture();
  runtimeGate.failed(new Error("WASM backend missing"));
  assert.equal(elements.body.dataset.runtimeState, "failed");
  assert.equal(elements.editorShell.hidden, true);
  assert.equal(elements.titlebar.hidden, true);
  assert.equal(elements.gate.hidden, false);
  assert.equal(elements.reloadButton.hidden, false);
  assert.match(elements.details.textContent, /WASM backend missing/);
  elements.reloadButton.click();
  assert.equal(reloads(), 1);
}

console.log("[runtime-gate-regression] ok");
