import { randomUUID } from "node:crypto";
import { join } from "node:path";
import { gunzipSync } from "node:zlib";
import { guiTestsDir } from "../protocol/paths.mjs";
import { readValidatedDocument } from "../protocol/validate.mjs";
import { evaluateDocumentArrowProperties, evaluateDocumentBondProperties, evaluateDocumentBracketProperties, evaluateDocumentChromatographyProperties, evaluateDocumentNodeProperties, evaluateDocumentOrbitalProperties, evaluateDocumentReports, evaluateDocumentShapeProperties, evaluateDocumentSymbolProperties, evaluateDocumentTableProperties, evaluateDocumentTextProperties, inspectDocumentBytes } from "../oracles/document-file.mjs";
import { evaluateUiState, uiStateRequest } from "../oracles/ui-state.mjs";
import { createWorkerCoordinator } from "../workers/create.mjs";

const defaultProfilePath = join(guiTestsDir, "environments", "windows-gui-worker-current.json");

export const productionBlackBoxCapabilities = Object.freeze([
  "gui.public-input",
  "editor.bond.draw",
  "editor.bond.query-order",
  "editor.bond.topology",
  "editor.bond.reaction-participation",
  "editor.bond.absolute-stereo",
  "editor.bond.double-placement",
  "editor.bond.visibility",
  "editor.ring.insert",
  "editor.chain.draw",
  "editor.arrow.draw",
  "editor.arrow.properties",
  "editor.text.create",
  "editor.text.edit-existing",
  "editor.text.properties",
  "editor.shape.draw",
  "editor.shape.properties",
  "editor.symbol.draw",
  "editor.symbol.properties",
  "editor.symbol.atom-attachment",
  "editor.bracket.draw",
  "editor.bracket.properties",
  "editor.table.draw",
  "editor.table.properties",
  "editor.orbital.draw",
  "editor.orbital.properties",
  "editor.chromatography.draw",
  "editor.chromatography.properties",
  "editor.selection.select-all",
  "editor.selection.region",
  "editor.selection.additive",
  "editor.selection.mixed-object",
  "editor.object.lock",
  "editor.context-menu",
  "editor.group.group-ungroup",
  "editor.group.nested",
  "editor.group.locked-ancestor",
  "editor.clipboard.copy-paste",
  "editor.clipboard.cross-document",
  "editor.selection.delete",
  "editor.selection.delete-partial",
  "editor.selection.transform-partial",
  "editor.history.undo-redo",
  "editor.atom.element",
  "editor.atom.implicit-hydrogens",
  "document.save-as",
  "document.open",
  "document.close-discard",
  "document.roundtrip",
  "document.new",
  "oracle.dom",
  "oracle.ui-state",
  "frontend.focus",
  "frontend.hover",
  "frontend.disabled-state",
  "frontend.selection-overlay",
  "frontend.dpi",
  "oracle.diagnostics",
  "oracle.document-file",
  "desktop.production",
]);

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

export function summarizePerformanceTrace(traceDocument) {
  const events = traceDocument.traceEvents;
  const allActionMarkers = events.filter((event) => typeof event?.name === "string" && event.name.startsWith("chemsema-action:"));
  const actionMarkers = allActionMarkers.slice(0, 512).map((event) => ({
    name: event.name,
    timestampUs: Number.isFinite(event.ts) ? event.ts : null,
    processId: Number.isInteger(event.pid) ? event.pid : null,
    threadId: Number.isInteger(event.tid) ? event.tid : null,
  }));
  const completed = events.filter((event) => event?.ph === "X" && Number.isFinite(event.dur) && event.dur >= 50_000);
  const topLongTasks = [...completed]
    .sort((left, right) => right.dur - left.dur)
    .slice(0, 25)
    .map((event) => ({
      name: String(event.name || "unknown").slice(0, 128),
      category: String(event.cat || "").slice(0, 256),
      durationMs: Math.round(event.dur) / 1000,
      timestampUs: Number.isFinite(event.ts) ? event.ts : null,
      processId: Number.isInteger(event.pid) ? event.pid : null,
      threadId: Number.isInteger(event.tid) ? event.tid : null,
      ...(typeof event.args?.url === "string" ? { url: event.args.url.slice(0, 2048) } : {}),
      ...(typeof event.args?.data?.functionName === "string" ? { functionName: event.args.data.functionName.slice(0, 256) } : {}),
      ...(typeof event.args?.data?.url === "string" ? { scriptUrl: event.args.data.url.slice(0, 2048) } : {}),
    }));
  const hotspotNames = new Set(["EventDispatch", "Layout", "UpdateLayoutTree", "Paint", "Commit", "RasterTask", "RunMicrotasks", "RunTask"]);
  const hotspots = new Map();
  for (const event of events) {
    if (event?.ph !== "X" || !Number.isFinite(event.dur) || !hotspotNames.has(event.name)) continue;
    const current = hotspots.get(event.name) || { name: event.name, count: 0, totalMs: 0, maxMs: 0 };
    const durationMs = event.dur / 1000;
    current.count += 1;
    current.totalMs += durationMs;
    current.maxMs = Math.max(current.maxMs, durationMs);
    hotspots.set(event.name, current);
  }
  return {
    schema: "chemsema.gui.performance-summary.v1",
    eventCount: events.length,
    actionMarkers,
    actionMarkersTruncated: allActionMarkers.length > actionMarkers.length,
    longTaskThresholdMs: 50,
    longTaskCount: completed.length,
    maxLongTaskMs: topLongTasks[0]?.durationMs || 0,
    topLongTasks,
    hotspots: [...hotspots.values()].sort((left, right) => right.totalMs - left.totalMs).map((entry) => ({
      ...entry,
      totalMs: Math.round(entry.totalMs * 1000) / 1000,
      maxMs: Math.round(entry.maxMs * 1000) / 1000,
    })),
  };
}

export class ProductionBlackBoxDriver {
  constructor({ coordinator, profilePath = defaultProfilePath } = {}) {
    this.name = "production-black-box";
    this.coordinator = coordinator;
    this.profilePath = profilePath;
    this.targets = new Map();
    this.diagnostics = [];
    this.environmentNotes = [];
    this.activeActionPhases = [];
  }

  async prepare(profile) {
    if (profile.pool !== "interactive-isolated-worker" || profile.runtime !== "production") {
      throw new Error("Production black-box scenarios require the interactive isolated production profile.");
    }
    this.scenarioProfile = profile;
    if (!this.coordinator) {
      this.workerProfile = await readValidatedDocument(this.profilePath);
      this.coordinator = createWorkerCoordinator(this.workerProfile);
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
    this.documentOutput = await this.coordinator.prepareDocumentOutput("roundtrip.ccjs");
    mark("document-output-prepared");
  }

  capabilities() {
    return [...productionBlackBoxCapabilities];
  }

  async resolve(target) {
    if (target.strategy === "uia-automation-id") {
      const value = await this.coordinator.queryUiaByAutomationId(target.value, { controlType: target.controlType });
      const matches = value.query.matches
        .filter((match) => !match.offscreen && match.enabled && match.className === target.className)
        .map((match) => ({ ...match, visible: true, disabled: false, coordinateSpace: "screen" }));
      if (matches.length !== 1) throw new Error(`Native UI target resolved to ${matches.length} visible enabled elements.`);
      this.targets.set(key(target), matches[0]);
      return { target, match: matches[0] };
    }
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
    if (match.coordinateSpace === "screen") {
      return { rect: match.rect, screen: (point) => point.map((value) => Math.round(value)) };
    }
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
      const receipt = await this.coordinator.candidateInput("click", { x: input.x, y: input.y }, { button: input.button, modifiers: input.modifiers });
      this.foreground = receipt.agent.foreground;
      return result;
    }
    if (input.kind === "drag") {
      const receipt = await this.coordinator.candidateInput("drag", { from: input.from, to: input.to }, { button: input.button, steps: input.steps, modifiers: input.modifiers });
      this.foreground = receipt.agent.foreground;
      return result;
    }
    if (input.kind === "text") {
      const receipt = await this.coordinator.candidateInput("text", { text: input.text });
      this.foreground = receipt.agent.foreground;
      return result;
    }
    throw new Error(`Production black-box input type ${action.type} is not implemented.`);
  }

  async resolveActionInput(action) {
    if (action.type === "key") return { input: { kind: "key", key: action.key }, result: { kind: "key", key: action.key } };
    if (action.type === "text") {
      if (typeof action.text === "string") {
        return { input: { kind: "text", text: action.text }, result: { kind: "text", textLength: action.text.length } };
      }
      if (action.textSource !== "document-output-path" || !this.documentOutput?.guestPath) throw new Error("Document output path is unavailable for text input.");
      return { input: { kind: "text", text: this.documentOutput.guestPath }, result: { kind: "text", textSource: action.textSource } };
    }
    const geometry = await this.inputGeometry(action.target);
    const [left, top, right, bottom] = geometry.rect;
    if (action.type === "click") {
      const at = action.at || { x: 0.5, y: 0.5 };
      const [x, y] = geometry.screen([left + (right - left) * at.x, top + (bottom - top) * at.y]);
      const input = { kind: "click", x, y, button: action.button || "left" };
      if (action.modifiers?.length) input.modifiers = [...action.modifiers];
      return { input, result: { kind: "click", screen: [x, y], modifiers: input.modifiers || [] } };
    }
    if (action.type === "drag") {
      const from = geometry.screen([left + (right - left) * action.from.x, top + (bottom - top) * action.from.y]);
      const to = geometry.screen([left + (right - left) * action.to.x, top + (bottom - top) * action.to.y]);
      const input = { kind: "drag", from, to, steps: action.steps, button: action.button || "left" };
      if (action.modifiers?.length) input.modifiers = [...action.modifiers];
      return { input, result: { kind: "drag", from, to, modifiers: input.modifiers || [] } };
    }
    throw new Error(`Production black-box input type ${action.type} is not implemented.`);
  }

  stateReceipt(state) {
    return { revision: state.revision, appScript: state.appScript, engine: state.engine, window: state.window, rendered: state.rendered };
  }

  async executeAction(action) {
    this.activeActionPhases = [];
    const phase = async (name, operation) => {
      const entry = { name, durationMs: 0, status: "in-progress", startedAt: Date.now() };
      this.activeActionPhases.push(entry);
      try {
        const result = await operation();
        entry.durationMs = Date.now() - entry.startedAt;
        entry.status = "completed";
        return result;
      } catch (error) {
        entry.durationMs = Date.now() - entry.startedAt;
        entry.status = "failed";
        throw error;
      }
    };
    const { input } = await this.resolveActionInput(action);
    if (action.target.strategy === "uia-automation-id" || ["uia-visible", "document-saved"].includes(action.completion.kind) || input.kind === "text") {
      const nativeTarget = action.target.strategy === "uia-automation-id";
      if (nativeTarget && !this.lastActionState) {
        throw new Error("Native dialog input requires a stable WebView state captured before the dialog opened.");
      }
      const before = nativeTarget ? this.lastActionState : await phase("capture-before-state", () => this.actionState());
      const result = await phase("perform-input", () => this.performResolvedInput(input, action));
      const closesNativeDialog = nativeTarget && action.completion.kind !== "actionable" && action.completion.kind !== "uia-visible";
      if (closesNativeDialog) {
        await phase("wait-native-dismissal", () => this.waitForNativeTargetDismissal(action.target, action.completion.timeoutMs));
        const attestation = await phase("reattest-interactive-agent", () => retry(() => this.coordinator.attestInteractiveAgent(), { timeoutMs: action.completion.timeoutMs, intervalMs: 200 }));
        this.foreground = attestation.agent.foreground;
      }
      const completion = await phase("wait-completion", () => this.waitForCompletion(action.completion));
      const nativeDialogRemains = action.completion.kind === "uia-visible" || (nativeTarget && !closesNativeDialog);
      const after = nativeDialogRemains ? before : await phase("capture-after-state", () => this.actionState());
      return { before, after, completion, result, phases: this.actionProgress() };
    }
    const receipt = await this.coordinator.candidateAction(input, action.completion, action.budgetMs, action.id);
    this.foreground = receipt.transaction.input.foreground;
    this.lastActionState = this.stateReceipt(receipt.transaction.after);
    return {
      before: this.stateReceipt(receipt.transaction.before),
      after: this.lastActionState,
      completion: receipt.transaction.completion,
      phases: [],
    };
  }

  actionProgress() {
    return this.activeActionPhases.map(({ startedAt, ...entry }) => ({
      ...entry,
      durationMs: entry.status === "in-progress" ? Date.now() - startedAt : entry.durationMs,
    }));
  }

  async performResolvedInput(input, action) {
    if (input.kind === "key") {
      const receipt = await this.coordinator.candidateInput("key", { key: input.key });
      this.foreground = receipt.agent.foreground;
      return { kind: "key", key: input.key };
    }
    if (input.kind === "text") {
      const receipt = await this.coordinator.candidateInput("text", { text: input.text });
      this.foreground = receipt.agent.foreground;
      return action.textSource
        ? { kind: "text", textSource: action.textSource }
        : { kind: "text", textLength: input.text.length };
    }
    if (input.kind === "click") {
      const receipt = await this.coordinator.candidateInput("click", { x: input.x, y: input.y }, { button: input.button, modifiers: input.modifiers });
      this.foreground = receipt.agent.foreground;
      return { kind: "click", screen: [input.x, input.y] };
    }
    if (input.kind === "drag") {
      const receipt = await this.coordinator.candidateInput("drag", { from: input.from, to: input.to }, { button: input.button, steps: input.steps, modifiers: input.modifiers });
      this.foreground = receipt.agent.foreground;
      return { kind: "drag", from: input.from, to: input.to };
    }
    throw new Error(`Unsupported resolved input ${input.kind}.`);
  }

  async waitForNativeTargetDismissal(target, timeoutMs) {
    const resolved = this.targets.get(key(target));
    if (!resolved?.topLevelClassName || !resolved?.topLevelName) {
      throw new Error(`Native UI target ${target.value} has no captured top-level dialog identity.`);
    }
    return retry(async () => {
      const value = await this.coordinator.queryUiaByAutomationId(target.value, { controlType: target.controlType });
      const matches = value.query.matches.filter((match) =>
        !match.offscreen && match.enabled && match.className === target.className
      );
      if (matches.length !== 0) throw new Error(`Native UI target ${target.value} is still visible after input.`);
      const dialogRemains = value.query.topLevels?.some((root) =>
        !root.offscreen && root.className === resolved.topLevelClassName && root.name === resolved.topLevelName
      );
      if (dialogRemains) throw new Error(`Native dialog ${resolved.topLevelName} is still visible after input.`);
      return { dismissed: true, automationId: target.value, topLevelName: resolved.topLevelName };
    }, { timeoutMs, intervalMs: 200 });
  }

  async actionState() {
    if (this.completedActionState) {
      const state = this.completedActionState;
      this.completedActionState = null;
      this.lastActionState = state;
      return state;
    }
    const state = await this.coordinator.cdpBridge({ mode: "state" });
    this.lastActionState = this.stateReceipt(state);
    return this.lastActionState;
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
    if (completion.kind === "dom-text") {
      const observedText = await retry(async () => {
        const result = await this.coordinator.cdpBridge({ mode: "text-state", selector: completion.selector });
        if (result.count !== 1) throw new Error(`DOM text selector matched ${result.count} elements; expected exactly 1.`);
        if (result.text !== completion.text) throw new Error(`DOM text did not exactly match the expected ${completion.text.length}-character value.`);
        this.completedActionState = this.stateReceipt(result.state);
        return result.text;
      }, { timeoutMs: completion.timeoutMs, intervalMs: 200 });
      return { observedText };
    }
    if (completion.kind === "uia-visible") {
      const resolved = await retry(() => this.resolve(completion.target), { timeoutMs: completion.timeoutMs, intervalMs: 200 });
      return { visible: true, automationId: resolved.match.automationId, controlType: resolved.match.controlType };
    }
    if (completion.kind === "document-saved") {
      const document = await retry(() => this.ensureSavedDocument(), { timeoutMs: completion.timeoutMs, intervalMs: 250 });
      return { saved: true, size: document.transfer.size, sha256: document.transfer.sha256 };
    }
    throw new Error(`Unsupported completion ${completion.kind}.`);
  }

  async observe(oracle) {
    if (oracle.kind === "dom-count") return this.coordinator.cdpBridge({ mode: "count", selector: oracle.selector });
    if (oracle.kind === "dom-distinct-count") return this.coordinator.cdpBridge({ mode: "distinct-count", selector: oracle.selector, attribute: oracle.attribute });
    if (oracle.kind === "ui-state") {
      const observed = await this.coordinator.cdpBridge(uiStateRequest({ selector: oracle.selector, referenceSelector: oracle.referenceSelector, ...oracle.uiExpected }));
      return evaluateUiState(observed, oracle.uiExpected);
    }
    if (oracle.kind === "no-unexpected-diagnostics") return [...this.diagnostics];
    if (oracle.kind === "document-counts") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentReports(document.reports, oracle.expected);
    }
    if (oracle.kind === "document-bond-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentBondProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-node-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentNodeProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-arrow-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentArrowProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-text-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentTextProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-shape-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentShapeProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-orbital-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentOrbitalProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-chromatography-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentChromatographyProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-symbol-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentSymbolProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-bracket-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentBracketProperties(document.transfer.bytes, oracle.expected);
    }
    if (oracle.kind === "document-table-properties") {
      const document = await this.ensureSavedDocument();
      return evaluateDocumentTableProperties(document.transfer.bytes, oracle.expected);
    }
    throw new Error(`Unsupported oracle ${oracle.kind}.`);
  }

  async ensureSavedDocument() {
    if (this.savedDocument?.inspectionError) throw this.savedDocument.inspectionError;
    if (this.savedDocument) return this.savedDocument;
    if (!this.documentOutput) throw new Error("No bounded document output was prepared.");
    const transfer = await this.coordinator.fetchDocumentOutput(this.documentOutput);
    this.savedDocument = { transfer, reports: null, inspectionError: null };
    try {
      this.savedDocument.reports = await inspectDocumentBytes(transfer.bytes);
    } catch (error) {
      this.savedDocument.inspectionError = error;
      throw error;
    }
    return this.savedDocument;
  }

  async environment() {
    const physical = this.workerProfile?.kind === "physical-windows";
    return {
      platform: physical ? "windows-physical" : "windows-hyperv",
      workerProfile: this.workerProfile?.id || null,
      vmId: physical ? null : this.startReceipt?.vmId || null,
      machineIdSha256: physical ? this.startReceipt?.machineIdSha256 || null : null,
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
    const expected = ["final-screenshot.png", "final-state.json", "final-dom.html", "performance-trace.json.gz", "webview.log"];
    const names = payloads.map((payload) => payload.name).sort();
    if (JSON.stringify(names) !== JSON.stringify([...expected].sort())) {
      throw new Error(`Production artifact export returned an unexpected payload set: ${names.join(", ")}.`);
    }
    const trace = payloads.find((payload) => payload.name === "performance-trace.json.gz");
    let traceDocument;
    try {
      const decompressed = gunzipSync(trace.bytes, { maxOutputLength: 256 * 1024 * 1024 });
      traceDocument = JSON.parse(decompressed.toString("utf8"));
    } catch (error) {
      throw new Error(`Production performance trace is not valid bounded gzip JSON: ${error.message}`);
    }
    if (!Array.isArray(traceDocument.traceEvents) || traceDocument.traceEvents.length === 0) {
      throw new Error("Production performance trace contains no trace events.");
    }
    payloads.push({
      name: "performance-summary.json",
      mediaType: "application/json",
      bytes: Buffer.from(JSON.stringify(summarizePerformanceTrace(traceDocument), null, 2)),
    });
    if (this.savedDocument) {
      payloads.push({ name: "saved-document.ccjs", mediaType: "application/vnd.chemsema.document+json", bytes: this.savedDocument.transfer.bytes });
      payloads.push({
        name: this.savedDocument.inspectionError ? "saved-document-inspect-error.json" : "saved-document-inspect.json",
        mediaType: "application/json",
        bytes: Buffer.from(JSON.stringify({
          schema: "chemsema.gui.document-oracle.v1",
          size: this.savedDocument.transfer.size,
          sha256: this.savedDocument.transfer.sha256,
          ...(this.savedDocument.inspectionError
            ? { error: this.savedDocument.inspectionError.message }
            : this.savedDocument.reports),
        }), "utf8"),
      });
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
