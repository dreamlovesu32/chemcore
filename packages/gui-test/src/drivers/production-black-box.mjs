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
  }

  capabilities() {
    return ["gui.public-input", "editor.bond.draw", "editor.history.undo-redo", "oracle.dom", "oracle.diagnostics", "desktop.production"];
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
    if (action.type === "key") {
      const receipt = await this.coordinator.candidateInput("key", { key: action.key });
      this.foreground = receipt.agent.foreground;
      return { kind: "key", key: action.key };
    }
    const geometry = await this.inputGeometry(action.target);
    const [left, top, right, bottom] = geometry.rect;
    if (action.type === "click") {
      const [x, y] = geometry.screen([(left + right) / 2, (top + bottom) / 2]);
      const receipt = await this.coordinator.candidateInput("click", { x, y }, { button: action.button || "left" });
      this.foreground = receipt.agent.foreground;
      return { kind: "click", screen: [x, y] };
    }
    if (action.type === "drag") {
      const from = geometry.screen([left + (right - left) * action.from.x, top + (bottom - top) * action.from.y]);
      const to = geometry.screen([left + (right - left) * action.to.x, top + (bottom - top) * action.to.y]);
      const receipt = await this.coordinator.candidateInput("drag", { from, to }, { button: action.button || "left", steps: action.steps });
      this.foreground = receipt.agent.foreground;
      return { kind: "drag", from, to };
    }
    throw new Error(`Production black-box input type ${action.type} is not implemented.`);
  }

  async actionState() {
    if (this.completedActionState) {
      const state = this.completedActionState;
      this.completedActionState = null;
      return state;
    }
    const state = await this.coordinator.cdpBridge({ mode: "state" });
    return { revision: state.revision, appScript: state.appScript, engine: state.engine, window: state.window, rendered: state.rendered };
  }

  async waitForCompletion(completion) {
    if (completion.kind === "actionable") return { actionable: true };
    if (completion.kind === "quiescent") return { quiescent: true };
    if (completion.kind === "dom-count") {
      const observed = await retry(async () => {
        const result = await this.coordinator.cdpBridge({ mode: "count-state", selector: completion.selector });
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
    return [];
  }

  async shutdown() {
    if (this.coordinator) {
      try { await this.coordinator.stopCdpAgent(); } catch { /* VM shutdown remains the final cleanup boundary. */ }
      try { await this.coordinator.stopInputAgent(); } catch { /* VM shutdown remains the final cleanup boundary. */ }
      await this.coordinator.stop();
    }
  }
}
