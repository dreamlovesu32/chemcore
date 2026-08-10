export class FakeDriver {
  constructor() {
    this.name = "fake";
    this.counts = new Map();
    this.texts = new Map();
    this.diagnostics = [];
  }

  async prepare(profile) {
    this.profile = profile;
  }

  async launch(candidate = {}) {
    this.candidate = candidate;
  }

  capabilities() {
    return ["gui.public-input", "editor.bond.draw"];
  }

  async resolve(target) {
    return { ...target, fake: true };
  }

  async perform(action) {
    if (action.fakeEffect?.kind === "increment-dom-count") {
      const previous = this.counts.get(action.fakeEffect.selector) || 0;
      this.counts.set(action.fakeEffect.selector, previous + action.fakeEffect.by);
    }
    if (action.fakeEffect?.kind === "set-dom-text") {
      this.texts.set(action.fakeEffect.selector, action.fakeEffect.text);
    }
    return { fakeEffect: action.fakeEffect || null };
  }

  async actionState() {
    return {
      revision: [...this.counts.values()].reduce((sum, count) => sum + count, 0),
      window: { id: "fake-window", foreground: true },
      rendered: Object.fromEntries(this.counts),
    };
  }

  async waitForCompletion(completion) {
    if (completion.kind === "dom-count") {
      const observed = this.counts.get(completion.selector) || 0;
      const passed = completion.operator === "eq" ? observed === completion.value : observed >= completion.value;
      if (!passed) {
        throw new Error(`Completion failed for ${completion.selector}: observed ${observed}, expected ${completion.operator} ${completion.value}.`);
      }
      return { observed };
    }
    if (completion.kind === "dom-text") {
      const observedText = this.texts.get(completion.selector);
      if (observedText !== completion.text) {
        throw new Error(`DOM text completion failed for ${completion.selector}: observed ${JSON.stringify(observedText)}, expected ${JSON.stringify(completion.text)}.`);
      }
      return { observedText };
    }
    return { kind: completion.kind };
  }

  async observe(oracle) {
    if (oracle.kind === "dom-count") {
      return this.counts.get(oracle.selector) || 0;
    }
    if (oracle.kind === "no-unexpected-diagnostics") {
      return [...this.diagnostics];
    }
    throw new Error(`Fake driver does not support oracle ${oracle.kind}.`);
  }

  async environment() {
    return { platform: "fake", profile: this.profile };
  }

  async collectArtifacts() {
    return [];
  }

  async shutdown() {}
}
