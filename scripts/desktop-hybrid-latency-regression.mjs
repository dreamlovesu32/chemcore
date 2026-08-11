import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const nodeCount = Number(process.env.CHEMSEMA_HYBRID_LATENCY_NODE_COUNT || 5000);
const nativeDelayMs = Number(process.env.CHEMSEMA_HYBRID_FAKE_NATIVE_DELAY_MS || 250);
const documentJsonDelayMs = Number(process.env.CHEMSEMA_HYBRID_DOCUMENT_JSON_DELAY_MS || 140);
const maxLocalMs = Number(process.env.CHEMSEMA_HYBRID_LOCAL_MAX_MS || 80);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function portIsOpen() {
  return new Promise((resolve) => {
    const socket = net.connect({ host, port }, () => {
      socket.end();
      resolve(true);
    });
    socket.on("error", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

function waitForPort(timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const socket = net.connect({ host, port }, () => {
        socket.end();
        resolve(true);
      });
      socket.on("error", () => {
        socket.destroy();
        if (Date.now() >= deadline) {
          reject(new Error(`Timed out waiting for ${host}:${port}`));
        } else {
          setTimeout(attempt, 100);
        }
      });
    };
    attempt();
  });
}

async function ensureServer() {
  if (await portIsOpen()) {
    return null;
  }
  const child = spawn(process.execPath, ["scripts/desktop-dev-server.mjs"], {
    cwd: rootDir,
    stdio: "ignore",
    windowsHide: true,
  });
  await waitForPort();
  return child;
}

function makeLargeChainDocument(count) {
  const spacing = 28;
  const columns = 50;
  const nodes = [];
  const bonds = [];
  for (let index = 0; index < count; index += 1) {
    const column = index % columns;
    const row = Math.floor(index / columns);
    nodes.push({
      id: `n${index}`,
      element: index === 0 ? "O" : "C",
      atomicNumber: index === 0 ? 8 : 6,
      position: [40 + column * spacing, 40 + row * spacing],
      charge: 0,
      numHydrogens: 0,
      meta: null,
    });
    if (index > 0) {
      bonds.push({
        id: `b${index - 1}`,
        begin: `n${index - 1}`,
        end: `n${index}`,
        order: 1,
        strokeWidth: 1,
        meta: null,
      });
    }
  }
  const height = 120 + Math.ceil(count / columns) * spacing;
  return {
    format: { name: "chemsema", version: "0.1", unit: "pt" },
    document: {
      id: "doc_desktop_hybrid_latency",
      title: "Desktop hybrid latency regression",
      page: { width: 1600, height: Math.max(900, height), background: "#ffffff" },
      meta: null,
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        fill: "#000000",
        strokeWidth: 0.85,
        fontFamily: "Arial",
        fontSize: 11,
      },
    },
    objects: [{
      id: "obj_large_molecule",
      type: "molecule",
      name: "large molecule",
      visible: true,
      locked: false,
      zIndex: 10,
      transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: {
        resourceRef: "mol_large",
        bbox: [0, 0, 1600, Math.max(900, height)],
        extra: {},
      },
      meta: null,
      children: [],
    }],
    resources: {
      mol_large: {
        id: "mol_large",
        type: "molecule_fragment2d",
        encoding: "chemsema.molecule.fragment2d",
        data: { nodes, bonds },
      },
    },
  };
}

function installFakeTauriScript(delayMs) {
  return ({ nativeDelayMs: configuredDelay }) => {
    const delay = configuredDelay ?? delayMs;
    const sessions = new Map();
    let nextSessionId = 1;
    const invocations = [];
    const counts = {};
    const minimalDocument = JSON.stringify({
      format: { name: "chemsema", version: "0.1", unit: "pt" },
      document: {
        id: "fake_native_doc",
        title: "Fake native document",
        page: { width: 600, height: 400, background: "#ffffff" },
        meta: null,
      },
      styles: {},
      objects: [],
      resources: {},
    });
    const delayedCommands = new Set([
      "desktop_engine_select_at_point",
      "desktop_engine_select_in_rect",
      "desktop_engine_select_all",
      "desktop_engine_begin_selection_move",
      "desktop_engine_finish_selection_move",
      "desktop_engine_pointer_move",
      "desktop_engine_pointer_up",
      "desktop_engine_begin_hover_arrow_edit",
      "desktop_engine_finish_hover_arrow_edit",
      "desktop_engine_apply_text_edit",
      "desktop_engine_apply_bracket_label_text",
      "desktop_engine_clear_interaction",
    ]);
    const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const snapshot = (session, mode = "document") => {
      const includeDocument = mode === "document" || mode === "documentState";
      const includeRender = mode === "document";
      const includeInteraction = ["interaction", "selection", "document"].includes(mode);
      return JSON.stringify({
        documentJson: includeDocument ? session.documentJson : undefined,
        stateJson: session.stateJson,
        renderListJson: includeRender ? "[]" : undefined,
        interactionRenderListJson: includeInteraction ? "[]" : undefined,
        allBoundsJson: includeRender ? "null" : undefined,
        documentBoundsJson: includeRender ? "null" : undefined,
        selectionBoundsJson: includeInteraction ? "null" : undefined,
        selectionChemistrySummaryJson: ["selection", "document"].includes(mode) ? "null" : undefined,
        documentColorsJson: includeDocument ? "[]" : undefined,
        documentStylePreset: "default",
        revision: session.revision,
        lastCommandResultJson: session.lastCommandResultJson,
        canUndo: false,
        canRedo: false,
      });
    };
    window.__chemsemaHybridLatencyFakeNative = { invocations, counts };
    window.__TAURI__ = {
      core: {
        invoke: async (command, args = {}) => {
          invocations.push({ command, args, at: performance.now() });
          counts[command] = (counts[command] || 0) + 1;
          if (delayedCommands.has(command)) {
            await sleep(delay);
          }
          if (command === "desktop_engine_create") {
            const sessionId = nextSessionId++;
            sessions.set(sessionId, {
              documentJson: minimalDocument,
              stateJson: "{}",
              revision: 0,
              lastCommandResultJson: "null",
            });
            return sessionId;
          }
          const session = sessions.get(args.sessionId);
          if (command === "desktop_engine_free") {
            return sessions.delete(args.sessionId);
          }
          if (!session) {
            return null;
          }
          if (command === "desktop_engine_load_document_json") {
            session.documentJson = args.json || minimalDocument;
            session.revision += 1;
            return null;
          }
          if (command === "desktop_engine_snapshot_json") {
            return snapshot(session, args.mode || "document");
          }
          if (command === "desktop_engine_document_json") {
            return session.documentJson;
          }
          if (command === "desktop_engine_document_cdxml") {
            return "<CDXML></CDXML>";
          }
          if (command === "desktop_engine_document_svg") {
            return "<svg></svg>";
          }
          if (command === "desktop_engine_render_bounds_json") {
            return "null";
          }
          if (command === "desktop_engine_state_json") {
            return session.stateJson;
          }
          if (command === "desktop_engine_render_list_json" || command === "desktop_engine_interaction_render_list_json") {
            return "[]";
          }
          if (command === "desktop_engine_finish_selection_move" || command === "desktop_engine_finish_hover_arrow_edit") {
            session.revision += 1;
            return true;
          }
          return null;
        },
      },
    };
  };
}

async function main() {
  const server = await ensureServer();
  let browser = null;
  try {
    browser = await chromium.launch({
      headless: true,
      executablePath: existsSync(edgePath) ? edgePath : undefined,
    });
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    const errors = [];
    page.on("console", (message) => {
      if (message.type() === "error") {
        errors.push(message.text());
      }
    });
    page.on("pageerror", (error) => errors.push(error.stack || error.message));
    await page.addInitScript(installFakeTauriScript(nativeDelayMs), { nativeDelayMs });
      await page.goto(`${baseUrl}?engine=wasm&hybridLatencyRegression=${Date.now()}`, { waitUntil: "domcontentloaded" });
    const result = await page.evaluate(async ({ documentData, nativeDelay, documentDelay, maxMs }) => {
      const { createEngineHost } = await import(`/viewer/engine_host.js?hybridLatency=${Date.now()}`);
      const host = createEngineHost("tauri-native");
      await host.initialize();
      const session = host.createEngineSession();
      await session.ready();
      await session.loadDocumentJson(JSON.stringify(documentData));

      let documentJsonCalls = 0;
      const originalDocumentJson = session.layoutEngine.documentJson.bind(session.layoutEngine);
      session.layoutEngine.documentJson = () => {
        documentJsonCalls += 1;
        const deadline = performance.now() + documentDelay;
        while (performance.now() < deadline) {
          // Busy wait intentionally: this catches accidental synchronous whole-document serialization.
        }
        return originalDocumentJson();
      };

      const measure = async (name, fn) => {
        const callsBefore = documentJsonCalls;
        const started = performance.now();
        const raw = fn();
        const awaited = raw && typeof raw.then === "function";
        const value = awaited ? await raw : raw;
        const elapsedMs = performance.now() - started;
        return {
          name,
          elapsedMs,
          awaited,
          value: typeof value === "boolean" || typeof value === "string" ? value : null,
          documentJsonCalls: documentJsonCalls - callsBefore,
        };
      };

      const samples = [];
      session.setTool("select", "");
      session.selectAtPoint(40, 40, false);
      const localSelectionSummary = JSON.parse(session.selectionChemistrySummaryJson() || "null");

      session.setTool("arrow", "single");
      session.pointerDown(300, 200, false);
      for (let index = 0; index < 60; index += 1) {
        session.pointerMove(320 + index * 2, 200 + (index % 5), false);
      }
      samples.push(await measure("pointerUpAddArrow", () => session.pointerUp(430, 200, false)));

      session.setTool("select", "");
      samples.push(await measure("selectAtPointArrow", () => session.selectAtPoint(430, 200, false)));
      const beganMove = session.beginSelectionMove(430, 200, false, false);
      session.updateSelectionMove(486, 235, false);
      samples.push(await measure("finishSelectionMove", () => session.finishSelectionMove(486, 235, false)));

      let arrowAction = session.beginHoverArrowEdit(430, 200) || session.beginHoverArrowEdit(300, 200);
      if (!arrowAction) {
        for (const point of [[425, 200], [435, 200], [300, 200], [305, 200]]) {
          arrowAction = session.beginHoverArrowEdit(point[0], point[1]);
          if (arrowAction) {
            break;
          }
        }
      }
      if (arrowAction) {
        session.updateHoverArrowEdit(455, 215, false);
        samples.push(await measure("finishHoverArrowEdit", () => session.finishHoverArrowEdit(455, 215, false)));
      }

      const countNonMoleculeObjects = () => {
        const doc = JSON.parse(session.documentJson() || "null");
        const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
        const objects = Array.isArray(doc?.entities?.scene)
          ? doc.entities.scene
          : flatten(doc?.objects || []);
        return {
          total: objects.length,
          nonMolecule: objects.filter((object) => (object.type || object.objectType || object.object_type) !== "molecule").length,
          text: objects.filter((object) => (object.type || object.objectType || object.object_type) === "text").length,
          bracket: objects.filter((object) => (object.type || object.objectType || object.object_type) === "bracket").length,
        };
      };
      const latestBracketObjectId = () => {
        const doc = JSON.parse(session.documentJson() || "null");
        const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
        const objects = Array.isArray(doc?.entities?.scene)
          ? doc.entities.scene
          : flatten(doc?.objects || []);
        return objects
          .filter((object) => (object.type || object.objectType || object.object_type) === "bracket")
          .map((object) => object.id)
          .filter(Boolean)
          .at(-1) || "";
      };
      const nonMoleculeObjectSummaries = () => {
        const doc = JSON.parse(session.documentJson() || "null");
        const flatten = (objects) => objects.flatMap((object) => [object, ...flatten(object.children || [])]);
        const objects = Array.isArray(doc?.entities?.scene)
          ? doc.entities.scene
          : flatten(doc?.objects || []);
        return objects
          .filter((object) => (object.type || object.objectType || object.object_type) !== "molecule")
          .map((object) => ({
            id: object.id,
            type: object.type || object.objectType || object.object_type,
            name: object.name || "",
            childCount: (object.children || []).length,
            payloadKind: object.payload?.kind || object.payload?.bracketKind || object.payload?.bracket_kind || "",
          }));
      };
      const dragTool = (tool, start, end) => {
        session.setTool(tool, "single");
        session.pointerDown(start.x, start.y, false);
        session.pointerMove(end.x, end.y, false);
        session.pointerUp(end.x, end.y, false);
        return JSON.parse(session.lastCommandResultJson?.() || "null");
      };
      dragTool("arrow", { x: 120, y: 320 }, { x: 240, y: 320 });
      dragTool("shape", { x: 280, y: 300 }, { x: 360, y: 380 });
      const bracketResult = dragTool("bracket", { x: 420, y: 290 }, { x: 520, y: 390 });
      const bracketId = await session.pendingGraphicObjectId?.()
        || latestBracketObjectId()
        || bracketResult?.targets?.objects?.[0]
        || bracketResult?.created?.objects?.[0]
        || "";
      const beforeBracketLabel = countNonMoleculeObjects();
      const bracketLabelSession = {
        target: { kind: "text-object", objectId: null, x: 530, y: 392 },
        text: "3",
        sourceRuns: [],
        fontFamily: "Arial",
        fontSize: 7.5,
        fill: "#000000",
        align: "left",
        lineHeight: 9,
        box: [0, 0, 8, 9],
        preserveLines: true,
        defaultChemical: false,
      };
      samples.push(await measure(
        "applyBracketLabelText",
        () => session.applyBracketLabelText(bracketId, JSON.stringify(bracketLabelSession)),
      ));
      const afterBracketLabel = countNonMoleculeObjects();
      await new Promise((resolve) => setTimeout(resolve, nativeDelay * 4));
      const afterBracketLabelNativeCatchup = countNonMoleculeObjects();

      samples.push(await measure("clearInteraction", () => session.clearInteraction()));
      await new Promise((resolve) => setTimeout(resolve, nativeDelay * 12));
      const counts = window.__chemsemaHybridLatencyFakeNative.counts;
      const nonMoleculeObjects = nonMoleculeObjectSummaries();
      await session.free();
      return {
        samples,
        beganMove,
        arrowAction,
        bracketId,
        nonMoleculeObjects,
        beforeBracketLabel,
        afterBracketLabel,
        afterBracketLabelNativeCatchup,
        localSelectionSummary,
        documentJsonCalls,
        counts,
        maxMs,
      };
    }, {
      documentData: makeLargeChainDocument(nodeCount),
      nativeDelay: nativeDelayMs,
      documentDelay: documentJsonDelayMs,
      maxMs: maxLocalMs,
    });
    for (const sample of result.samples) {
      assert(
        sample.elapsedMs < maxLocalMs,
        `${sample.name} took ${sample.elapsedMs.toFixed(1)}ms, expected < ${maxLocalMs}ms; sample=${JSON.stringify(sample)}`,
      );
      assert(
        sample.documentJsonCalls === 0,
        `${sample.name} called documentJson() on the local commit path: ${JSON.stringify(sample)}`,
      );
      assert(!sample.awaited, `${sample.name} returned a Promise and was awaited: ${JSON.stringify(sample)}`);
    }
    assert(result.beganMove === true, `Selection move did not begin: ${JSON.stringify(result)}`);
    assert(
      result.localSelectionSummary?.formula && result.localSelectionSummary?.formulaWeight > 0,
      `Desktop hybrid selection summary did not read the immediate local selection state: ${JSON.stringify(result.localSelectionSummary)}`,
    );
    assert(result.counts.desktop_engine_finish_selection_move >= 1, `Native selection finish was not queued: ${JSON.stringify(result.counts)}`);
    assert(result.counts.desktop_engine_pointer_up >= 1, `Native pointerUp was not queued: ${JSON.stringify(result.counts)}`);
    assert(
      (result.counts.desktop_engine_pointer_move || 0) <= 2,
      `Native pointerMove backlog was not coalesced: ${JSON.stringify(result.counts)}`,
    );
    if (result.arrowAction) {
      assert(result.counts.desktop_engine_finish_hover_arrow_edit >= 1, `Native arrow finish was not queued: ${JSON.stringify(result.counts)}`);
    }
    assert(result.bracketId, `Bracket drag did not create a bracket object: ${JSON.stringify(result)}`);
    assert(
      result.afterBracketLabel.nonMolecule >= result.beforeBracketLabel.nonMolecule + 1,
      `Bracket label commit removed newly created objects: ${JSON.stringify(result)}`,
    );
    assert(
      result.afterBracketLabelNativeCatchup.nonMolecule >= result.afterBracketLabel.nonMolecule,
      `Native catch-up rewound local bracket label state: ${JSON.stringify(result)}`,
    );
    assert(
      result.counts.desktop_engine_apply_bracket_label_text >= 1,
      `Native bracket label commit was not queued: ${JSON.stringify(result.counts)}`,
    );
    assert(!errors.length, `Viewer console errors:\n${errors.join("\n")}`);
    await page.close();
    const summary = result.samples
      .map((sample) => `${sample.name} ${sample.elapsedMs.toFixed(1)}ms`)
      .join(", ");
    console.log(`[desktop-hybrid-latency-regression] ok (${nodeCount} nodes, fake native ${nativeDelayMs}ms, documentJson trap ${documentJsonDelayMs}ms; ${summary})`);
  } finally {
    await browser?.close();
    server?.kill();
  }
}

await main();
