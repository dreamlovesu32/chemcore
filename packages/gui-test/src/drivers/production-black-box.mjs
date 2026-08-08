import { randomUUID } from "node:crypto";
import { join } from "node:path";
import { guiTestsDir } from "../protocol/paths.mjs";
import { readValidatedDocument } from "../protocol/validate.mjs";
import { HyperVCoordinator } from "../workers/hyperv.mjs";

const defaultProfilePath = join(guiTestsDir, "environments", "windows-gui-worker-current.json");

async function retry(operation, { timeoutMs = 90000, intervalMs = 1000 } = {}) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  do {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      if (Date.now() + intervalMs >= deadline) break;
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  } while (Date.now() < deadline);
  throw lastError;
}

function key(target) {
  return JSON.stringify(target);
}

export class ProductionBlackBoxDriver {
  constructor({ coordinator, profilePath = defaultProfilePath } = {}) {
    this.name = "production-black-box";
    this.coordinator = coordinator;
    this.profilePath = profilePath;
    this.targets = new Map();
    this.diagnostics = [];
    this.environmentNotes = [];
  }

  async prepare(profile) {
    if (profile.pool !== "interactive-isolated-worker" || profile.runtime !== "production") {
      throw new Error("Production black-box scenarios require the interactive isolated production profile.");
    }
    this.scenarioProfile = profile;
    if (!this.coordinator) {
      this.workerProfile = await readValidatedDocument(this.profilePath);
      this.coordinator = new HyperVCoordinator(this.workerProfile);
    }
  }

  async launch() {
    const launchStartedAt = Date.now();
    const mark = (stage) => this.environmentNotes.push(`launch-stage:${stage}:${Date.now() - launchStartedAt}ms`);
    await this.coordinator.reset();
    mark("worker-reset");
    this.startReceipt = await this.coordinator.start();
    mark("worker-started");
    await retry(() => this.coordinator.installAgent());
    mark("agent-installed");
    const baseline = await this.coordinator.configureDesktopBaseline();
    mark("desktop-baseline-configured");
    const guest = await retry(() => this.coordinator.attestGuest());
    mark("guest-attested");
    if (baseline.baseline?.changed || !guest.guest?.interactiveAccountMatches) {
      if (!guest.guest?.interactiveAccountMatches) await this.coordinator.configureAutologon();
      await this.coordinator.stop();
      this.startReceipt = await this.coordinator.start();
      await retry(() => this.coordinator.attestGuest({ requireInteractive: true }));
      mark("worker-restarted");
    }
    this.installReceipt = await this.coordinator.installCandidate();
    mark("candidate-installed");
    this.launchReceipt = await this.coordinator.launchCandidate();
    mark("candidate-launched");
    const attestation = await retry(() => this.coordinator.attestInteractiveAgent());
    mark("interactive-agent-attested");
    const candidatePath = this.launchReceipt.candidate.guestPath.toLowerCase();
    if (attestation.agent.foreground?.executable?.toLowerCase() !== candidatePath) {
      const foreground = attestation.agent.foreground;
      const knownBlocker = foreground?.executable?.toLowerCase().endsWith("\\windows\\system32\\wwahost.exe")
        && foreground.className === "Windows.UI.Core.CoreWindow";
      if (knownBlocker) {
        await this.coordinator.dismissKnownBlocker();
        mark("known-blocker-dismissed");
      }
    }
    this.activationReceipt = await retry(() => this.coordinator.activateCandidate());
    this.foreground = this.activationReceipt.agent.foreground;
    mark("candidate-activated");
    await this.coordinator.startInputAgent();
    mark("persistent-input-agent-ready");
    await this.coordinator.startCdpAgent();
    mark("persistent-cdp-agent-ready");
    this.webviewState = await retry(async () => {
      const state = await this.coordinator.cdpBridge({ mode: "state" });
      if (state.runtimeState !== "ready") throw new Error(`WebView runtime is ${state.runtimeState || "not ready"}.`);
      return state;
    });
    mark("webview-ready");
    const trace = await this.coordinator.cdpBridge({ mode: "trace-start" });
    if (!trace?.started) throw new Error("WebView performance tracing did not start.");
    mark("performance-trace-started");
  }

  capabilities() {
    return [
      "gui.public-input",
      "editor.bond.draw",
      "editor.arrow.draw",
      "editor.selection.select-all",
      "editor.selection.mixed-object",
      "editor.group.group-ungroup",
      "editor.group.nested",
      "editor.clipboard.copy-paste",
      "editor.selection.delete",
      "editor.history.undo-redo",
      "oracle.dom",
      "oracle.diagnostics",
      "desktop.production",
    ];
  }

  async resolve(target) {
    const value = await this.coordinator.cdpBridge({ mode: "locate", target });
    if (target.scope && value.scopeCount !== 1) {
      throw new Error(`Target scope resolved to ${value.scopeCount} elements.`);
    }
    const matches = value.matches.filter((match) => match.visible && !match.disabled);
    if (matches.length !== 1) {
      throw new Error(`Target resolved to ${matches.length} visible actionable elements.`);
    }
    this.targets.set(key(target), matches[0]);
    return { target, match: matches[0] };
  }

  async inputGeometry(target) {
    const match = this.targets.get(key(target));
    if (!match) throw new Error("Action target was not resolved before input.");
    const client = this.foreground?.clientRect;
    if (!Array.isArray(client) || client.length !== 4) {
      throw new Error("Authorized candidate client geometry is unavailable.");
    }
    const clientWidth = client[2] - client[0];
    const clientHeight = client[3] - client[1];
    const scaleX = clientWidth / this.webviewState.viewport.width;
    const scaleY = clientHeight / this.webviewState.viewport.height;
    return {
      rect: match.rect,
      screen(point) {
        return [
          Math.round(client[0] + point[0] * scaleX),
          Math.round(client[1] + point[1] * scaleY),
        ];
      },
    };
  }

  async perform(action) {
    const { input, result } = await this.resolveActionInput(action);
    if (input.kind === "key") {
      const receipt = await this.coordinator.candidateInput("key", { key: input.key });
      this.foreground = receipt.agent.foreground;
      return result;
    }
    if (input.kind === "click") {
      const receipt = await this.coordinator.candidateInput("click", { x: input.x, y: input.y }, { button: input.button });
      this.foreground = receipt.agent.foreground;
      return result;
    }
    if (input.kind === "drag") {
      const receipt = await this.coordinator.candidateInput("drag", { from: input.from, to: input.to }, { button: input.button, steps: input.steps });
      this.foreground = receipt.agent.foreground;
      return result;
    }
    throw new Error(`Production black-box input type ${action.type} is not implemented.`);
  }

  async resolveActionInput(action) {
    if (action.type === "key") return { input: { kind: "key", key: action.key }, result: { kind: "key", key: action.key } };
    const geometry = await this.inputGeometry(action.target);
    const [left, top, right, bottom] = geometry.rect;
    if (action.type === "click") {
      const [x, y] = geometry.screen([(left + right) / 2, (top + bottom) / 2]);
      return { input: { kind: "click", x, y, button: action.button || "left" }, result: { kind: "click", screen: [x, y] } };
    }
    if (action.type === "drag") {
      const from = geometry.screen([left + (right - left) * action.from.x, top + (bottom - top) * action.from.y]);
      const to = geometry.screen([left + (right - left) * action.to.x, top + (bottom - top) * action.to.y]);
      return {
        input: { kind: "drag", from, to, steps: action.steps, button: action.button || "left" },
        result: { kind: "drag", from, to },
      };
    }
    throw new Error(`Production black-box input type ${action.type} is not implemented.`);
  }

  stateReceipt(state) {
    return { revision: state.revision, appScript: state.appScript, engine: state.engine, window: state.window, rendered: state.rendered };
  }

  async executeAction(action) {
    const { input } = await this.resolveActionInput(action);
    const receipt = await this.coordinator.candidateAction(input, action.completion, action.budgetMs);
    this.foreground = receipt.transaction.input.foreground;
    return {
      before: this.stateReceipt(receipt.transaction.before),
      after: this.stateReceipt(receipt.transaction.after),
      completion: receipt.transaction.completion,
    };
  }

  async actionState() {
    if (this.completedActionState) {
      const state = this.completedActionState;
      this.completedActionState = null;
      return state;
    }
    const state = await this.coordinator.cdpBridge({ mode: "state" });
    return this.stateReceipt(state);
  }

  async waitForCompletion(completion) {
    if (completion.kind === "actionable") return { actionable: true };
    if (completion.kind === "quiescent") return { quiescent: true };
    if (completion.kind === "dom-count" || completion.kind === "dom-distinct-count") {
      const observed = await retry(async () => {
        const result = await this.coordinator.cdpBridge({
          mode: completion.kind === "dom-distinct-count" ? "distinct-count-state" : "count-state",
          selector: completion.selector,
          ...(completion.kind === "dom-distinct-count" ? { attribute: completion.attribute } : {}),
        });
        const count = result.count;
        const passed = completion.operator === "eq" ? count === completion.value : count >= completion.value;
        if (!passed) throw new Error(`DOM count is ${count}; expected ${completion.operator} ${completion.value}.`);
        this.completedActionState = { revision: result.state.revision, appScript: result.state.appScript, engine: result.state.engine, window: result.state.window, rendered: result.state.rendered };
        return count;
      }, { timeoutMs: completion.timeoutMs, intervalMs: 200 });
      return { observed };
    }
    throw new Error(`Unsupported completion ${completion.kind}.`);
  }

  async observe(oracle) {
    if (oracle.kind === "dom-count") return this.coordinator.cdpBridge({ mode: "count", selector: oracle.selector });
    if (oracle.kind === "dom-distinct-count") return this.coordinator.cdpBridge({ mode: "distinct-count", selector: oracle.selector, attribute: oracle.attribute });
    if (oracle.kind === "no-unexpected-diagnostics") return [...this.diagnostics];
    throw new Error(`Unsupported oracle ${oracle.kind}.`);
  }

  async environment() {
    return {
      platform: "windows-hyperv",
      workerProfile: this.workerProfile?.id || null,
      vmId: this.startReceipt?.vmId || null,
      candidateSha256: this.installReceipt?.candidate?.sha256 || null,
      notes: [...this.environmentNotes],
      profile: this.scenarioProfile,
    };
  }

  async collectArtifacts() {
    const artifactId = randomUUID().replaceAll("-", "");
    const exportManifest = await this.coordinator.cdpBridge({ mode: "artifact-export", artifactId });
    const truncated = exportManifest?.artifacts?.filter((artifact) => artifact.truncated).map((artifact) => artifact.name) || [];
    if (truncated.length) throw new Error(`Production artifact export truncated required payloads: ${truncated.join(", ")}.`);
    const payloads = await this.coordinator.fetchArtifacts(exportManifest);
    const expected = ["final-screenshot.png", "final-state.json", "final-dom.html", "performance-trace.json", "webview.log"];
    const names = payloads.map((payload) => payload.name).sort();
    if (JSON.stringify(names) !== JSON.stringify([...expected].sort())) {
      throw new Error(`Production artifact export returned an unexpected payload set: ${names.join(", ")}.`);
    }
    const trace = payloads.find((payload) => payload.name === "performance-trace.json");
    let traceDocument;
    try { traceDocument = JSON.parse(trace.bytes.toString("utf8")); } catch (error) {
      throw new Error(`Production performance trace is not valid JSON: ${error.message}`);
    }
    if (!Array.isArray(traceDocument.traceEvents) || traceDocument.traceEvents.length === 0) {
      throw new Error("Production performance trace contains no trace events.");
    }
    return payloads;
  }

  async shutdown() {
    if (this.coordinator) {
      try { await this.coordinator.stopCdpAgent(); } catch { /* VM shutdown remains the final cleanup boundary. */ }
      try { await this.coordinator.stopInputAgent(); } catch { /* VM shutdown remains the final cleanup boundary. */ }
      await this.coordinator.stop();
    }
  }
}
