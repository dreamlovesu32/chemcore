import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import net from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMSEMA_DESKTOP_DEV_PORT || 8767);
const baseUrl = `http://${host}:${port}/viewer/`;
const edgePath = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe";
const tmpDir = join(rootDir, "tmp", "gui-regression");
const guiCase = process.env.CHEMSEMA_GUI_CASE || "";
const exactTieOnly = guiCase === "exact-tie-double";
const selectionSummaryOnly = guiCase === "selection-summary";
const chemicalPropertyOnly = guiCase === "chemical-property";
const annotationOnly = guiCase === "annotation";
const documentLayoutOnly = guiCase === "document-layout";
const logicalObjectsOnly = guiCase === "logical-objects";

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

function capturePageErrors(page, errors) {
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(error.message));
}

async function installBrowserMocks(context) {
  await context.addInitScript(() => {
    const encoder = new TextEncoder();
    window.__chemsemaSavePickerWrites = [];
    window.__chemsemaSavePickerQueue = [];
    window.__chemsemaSetSavePickerQueue = (names) => {
      window.__chemsemaSavePickerQueue = Array.from(names || []);
    };
    window.confirm = () => true;

    try {
      Object.defineProperty(window, "showOpenFilePicker", {
        value: undefined,
        configurable: true,
      });
    } catch {
      window.showOpenFilePicker = undefined;
    }

    Object.defineProperty(window, "showSaveFilePicker", {
      configurable: true,
      value: async (options = {}) => {
        const name = window.__chemsemaSavePickerQueue.shift()
          || options.suggestedName
          || "chemsema-document.ccjz";
        const record = {
          name,
          suggestedName: options.suggestedName || null,
          byteLength: 0,
          text: "",
          chunkTypes: [],
          closed: false,
        };
        window.__chemsemaSavePickerWrites.push(record);
        return {
          name,
          async createWritable() {
            return {
              async write(chunk) {
                record.chunkTypes.push(chunk?.constructor?.name || typeof chunk);
                if (typeof chunk === "string") {
                  record.text += chunk;
                  record.byteLength += encoder.encode(chunk).byteLength;
                  return;
                }
                if (chunk instanceof Blob) {
                  record.byteLength += chunk.size;
                  try {
                    record.text += await chunk.text();
                  } catch {
                    // Binary blobs are still covered by byteLength.
                  }
                  return;
                }
                if (chunk instanceof ArrayBuffer) {
                  record.byteLength += chunk.byteLength;
                  return;
                }
                if (ArrayBuffer.isView(chunk)) {
                  record.byteLength += chunk.byteLength;
                  return;
                }
                if (chunk != null) {
                  const text = String(chunk);
                  record.text += text;
                  record.byteLength += encoder.encode(text).byteLength;
                }
              },
              async close() {
                record.closed = true;
              },
            };
          },
        };
      },
    });
  });
}

async function openViewer(context, errors, viewport = { width: 1400, height: 1000 }) {
  const page = await context.newPage({ viewport });
  page.setDefaultTimeout(12000);
  capturePageErrors(page, errors);
  await page.goto(`${baseUrl}?v=${Date.now()}`, { waitUntil: "domcontentloaded" });
  await waitForReady(page);
  return page;
}

async function waitForReady(page) {
  await page.waitForFunction(
    () => !!window.__chemsemaDebug?.state?.editorEngine && !!window.__chemsemaDebug?.document,
    null,
    { timeout: 30000 },
  );
}

async function drawBondWithMouse(page) {
  await page.locator('button[data-tool="bond"]').click();
  const box = await page.locator("#viewer-container").boundingBox();
  assert(box, "Viewer container is not visible.");
  const start = { x: box.x + box.width / 2 - 90, y: box.y + box.height / 2 };
  const end = { x: start.x + 130, y: start.y };
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
}

async function documentSummary(page) {
  return page.evaluate(() => {
    const doc = window.__chemsemaDebug?.document || null;
    const resources = Object.values(doc?.resources || {});
    const resourceBonds = resources.reduce((sum, resource) => sum + (resource?.data?.bonds?.length || 0), 0);
    const resourceNodes = resources.reduce((sum, resource) => sum + (resource?.data?.nodes?.length || 0), 0);
    const selectionBounds = JSON.parse(window.__chemsemaDebug?.state?.editorEngine?.selectionBoundsJson?.() || "null");
    return {
      title: doc?.document?.title || null,
      currentFileName: window.__chemsemaDebug?.state?.currentFileName || null,
      objects: doc?.objects?.length || 0,
      resourceBonds,
      resourceNodes,
      renderedBonds: document.querySelectorAll("[data-bond-id]").length,
      renderedNodes: document.querySelectorAll("[data-node-id]").length,
      selectionBounds,
      activeTool: window.__chemsemaDebug?.editorState?.activeTool || null,
      bondToolbarButtons: document.querySelectorAll('#secondary-toolbar [data-secondary-value^="bond-"]').length,
      blankToolbarIcons: [...document.querySelectorAll("#secondary-toolbar button")]
        .filter((button) => !button.querySelector("svg")).length,
    };
  });
}

async function setSaveQueue(page, names) {
  await page.evaluate((nextNames) => {
    window.__chemsemaSetSavePickerQueue(nextNames);
  }, names);
}

async function saveWrites(page) {
  return page.evaluate(() => window.__chemsemaSavePickerWrites);
}

async function waitForSaveWrite(page, index) {
  await page.waitForFunction(
    (targetIndex) => window.__chemsemaSavePickerWrites?.[targetIndex]?.closed === true,
    index,
  );
  return (await saveWrites(page))[index];
}

async function verifyToolbarAndCursor(page) {
  const tools = [
    { tool: "bond", minSecondary: 11 },
    { tool: "arrow", minSecondary: 6 },
    { tool: "shape", minSecondary: 5 },
    { tool: "templates", minSecondary: 5 },
  ];
  for (const item of tools) {
    await page.locator(`button[data-tool="${item.tool}"]`).click();
    await page.waitForFunction((tool) => {
      return document.querySelector(`button[data-tool="${tool}"]`)?.classList.contains("is-active");
    }, item.tool);
    const state = await page.evaluate((tool) => {
      const container = document.querySelector("#viewer-container");
      const svg = document.querySelector("#viewer-svg");
      const active = document.querySelector(`button[data-tool="${tool}"]`);
      return {
        tool,
        active: active?.classList.contains("is-active") || false,
        activeHasSvg: !!active?.querySelector("svg"),
        secondaryButtons: document.querySelectorAll("#secondary-toolbar button").length,
        secondarySvgs: document.querySelectorAll("#secondary-toolbar button svg").length,
        containerCursor: getComputedStyle(container).cursor,
        svgCursor: getComputedStyle(svg).cursor,
      };
    }, item.tool);
    assert.equal(state.active, true, `${item.tool} did not become active: ${JSON.stringify(state)}`);
    assert.equal(state.activeHasSvg, true, `${item.tool} tool button lost its icon: ${JSON.stringify(state)}`);
    assert(
      state.secondaryButtons >= item.minSecondary && state.secondarySvgs >= item.minSecondary,
      `${item.tool} secondary toolbar has blank/missing icons: ${JSON.stringify(state)}`,
    );
    assert(
      state.containerCursor !== "" && state.svgCursor !== "",
      `${item.tool} cursor styles were not applied: ${JSON.stringify(state)}`,
    );
  }
}

async function verifyDocumentLayoutControls(page) {
  const dimensions = await page.evaluate(() => {
    const footer = document.querySelector(".selection-status-bar")?.getBoundingClientRect();
    const template = document.querySelector("#template-panel-mode-button")?.getBoundingClientRect();
    const paper = document.querySelector("#paper-layout-mode-button")?.getBoundingClientRect();
    return {
      footerHeight: footer?.height,
      template: template && [template.width, template.height],
      paper: paper && [paper.width, paper.height],
    };
  });
  assert.equal(dimensions.footerHeight, 40, `document footer height drifted: ${JSON.stringify(dimensions)}`);
  assert.deepEqual(dimensions.template, [32, 32]);
  assert.deepEqual(dimensions.paper, [32, 32]);

  const paperButton = page.locator("#paper-layout-mode-button");
  await paperButton.click();
  await page.waitForFunction(() => (
    document.querySelector("#paper-layout-mode-button")?.getAttribute("aria-pressed") === "true"
    && document.querySelector('[data-layer="paper-layout"]')
  ));
  const pageState = await page.evaluate(() => ({
    pageCount: document.querySelectorAll('[data-layer="paper-layout"] [data-page-number]').length,
    layout: JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson()).document.layout,
  }));
  assert(pageState.pageCount >= 1, `paper view did not draw pages: ${JSON.stringify(pageState)}`);
  assert(Array.isArray(pageState.layout.pageOrigin), `paper view did not establish a page anchor: ${JSON.stringify(pageState.layout)}`);

  await paperButton.click({ button: "right" });
  await page.waitForSelector(".document-layout-quick-menu");
  const quickMenu = (await page.locator(".document-layout-quick-menu").textContent()) || "";
  assert(quickMenu.includes("A4") && quickMenu.includes("Document Layout"), `paper quick menu is incomplete: ${quickMenu}`);
  await page.getByRole("menuitem", { name: /Document Layout/ }).click();
  await page.waitForSelector(".document-layout-dialog");
  for (const field of [
    "paperWidth",
    "paperHeight",
    "widthPages",
    "heightPages",
    "pageOriginX",
    "pageOriginY",
    "marginTop",
    "marginRight",
    "marginBottom",
    "marginLeft",
    "pageOverlap",
  ]) {
    assert.equal(await page.locator(`[name="${field}"]`).count(), 1, `layout dialog is missing ${field}`);
  }
  await page.locator('[data-layout-tab="header-footer"]').click();
  assert.equal(await page.locator('[name="header"]').count(), 1);
  assert.equal(await page.locator('[name="footer"]').count(), 1);
  await page.locator('[data-layout-tab="view"]').click();
  assert.equal(await page.locator('[name="magnificationPercent"]').count(), 1);
  assert.equal(await page.locator('[name="splitterPositions"]').count(), 1);
  await page.locator('[data-layout-tab="embedding"]').click();
  assert.equal(await page.locator('[name="fixInPlaceExtentX"]').count(), 1);
  assert.equal(await page.locator('[name="fixInPlaceGapY"]').count(), 1);
  await page.getByRole("button", { name: "Cancel" }).click();
  await paperButton.click();
  await page.waitForFunction(() => (
    document.querySelector("#paper-layout-mode-button")?.getAttribute("aria-pressed") === "false"
    && !document.querySelector('[data-layer="paper-layout"]')
  ));
}

async function firstBondScreenGeometry(page) {
  return page.evaluate(() => {
    const svg = document.querySelector("#viewer-svg");
    const bond = document.querySelector('[data-role="document-bond"][data-bond-id]');
    if (!svg || !bond) {
      return null;
    }
    const toScreen = (x, y) => {
      const point = svg.createSVGPoint();
      point.x = x;
      point.y = y;
      const transformed = point.matrixTransform(svg.getScreenCTM());
      return { x: transformed.x, y: transformed.y };
    };
    if (bond.tagName.toLowerCase() === "line") {
      const start = toScreen(Number(bond.getAttribute("x1")), Number(bond.getAttribute("y1")));
      const end = toScreen(Number(bond.getAttribute("x2")), Number(bond.getAttribute("y2")));
      return {
        start,
        end,
        center: { x: (start.x + end.x) * 0.5, y: (start.y + end.y) * 0.5 },
      };
    }
    const box = bond.getBBox();
    const start = toScreen(box.x, box.y + box.height * 0.5);
    const end = toScreen(box.x + box.width, box.y + box.height * 0.5);
    return {
      start,
      end,
      center: { x: (start.x + end.x) * 0.5, y: (start.y + end.y) * 0.5 },
    };
  });
}

async function selectionOverlayRoles(page) {
  return page.evaluate(() => [...document.querySelectorAll('#viewer-svg [data-role^="selection-"]')]
    .map((element) => {
      const bounds = element.getBoundingClientRect();
      return {
        role: element.getAttribute("data-role"),
        tag: element.tagName.toLowerCase(),
        fill: element.getAttribute("fill"),
        stroke: element.getAttribute("stroke"),
        strokeWidth: element.getAttribute("stroke-width"),
        screenWidth: bounds.width,
        screenHeight: bounds.height,
      };
    }));
}

function assertBondSelectionDot(roles, label) {
  const dots = roles.filter((entry) => entry.role === "selection-bond-dot");
  assert.equal(dots.length, 1, `${label} should render exactly one selected bond center dot: ${JSON.stringify(roles)}`);
  assert(
    dots.every((dot) => dot.stroke !== "#ffffff" && dot.strokeWidth === "0"),
    `${label} selected bond dot should not paint a white stroke over the bond: ${JSON.stringify(dots)}`,
  );
}

async function verifySelectionOverlayConsistency(page) {
  await drawBondWithMouse(page);
  const geometry = await firstBondScreenGeometry(page);
  assert(geometry, "Could not locate the drawn bond for selection overlay checks.");

  await page.locator('button[data-tool="select"]').click();
  await page.mouse.click(geometry.center.x, geometry.center.y);
  await page.waitForFunction(() => document.querySelector('[data-role="selection-bond-dot"]'));
  const clickRoles = await selectionOverlayRoles(page);
  assert(clickRoles.some((entry) => entry.role === "selection-bond"), `Click-select did not render the bond box: ${JSON.stringify(clickRoles)}`);
  assert(!clickRoles.some((entry) => entry.role === "selection-box"), `Click-select should not render an outer component box: ${JSON.stringify(clickRoles)}`);
  assertBondSelectionDot(clickRoles, "Click-select");

  const margin = 18;
  await page.mouse.move(geometry.start.x - margin, geometry.start.y - margin);
  await page.mouse.down();
  await page.mouse.move(geometry.end.x + margin, geometry.end.y + margin, { steps: 8 });
  await page.mouse.up();
  await page.waitForFunction(() => document.querySelector('[data-role="selection-box"]'));
  const boxRoles = await selectionOverlayRoles(page);
  assert(boxRoles.some((entry) => entry.role === "selection-box"), `Box-select did not render the component selection box: ${JSON.stringify(boxRoles)}`);
  const componentBox = boxRoles.find((entry) => entry.role === "selection-box");
  assert(
    componentBox.screenWidth >= 2.9 && componentBox.screenHeight >= 2.9,
    `Component selection box lost its endpoint-focus-diameter minimum size: ${JSON.stringify(componentBox)}`,
  );
  assert(
    !boxRoles.some((entry) => entry.role === "selection-bond-dot"),
    `Box-selecting a complete molecule should suppress internal bond center dots: ${JSON.stringify(boxRoles)}`,
  );
  const chemistrySummary = (await page.locator("#selection-chemistry-summary").textContent()) || "";
  assert(
    chemistrySummary.includes("C2H6")
      && chemistrySummary.includes("Formula Weight")
      && chemistrySummary.includes("Exact Mass"),
    `Selected molecule did not populate the chemistry summary bar: ${JSON.stringify(chemistrySummary)}`,
  );
}

function makeShortDoubleBondDocument() {
  return {
    format: { name: "chemsema", version: "0.1" },
    document: {
      id: "doc_short_double",
      title: "short double bond",
      page: { width: 220, height: 180, background: "#ffffff" },
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        strokeWidth: 1,
        fontFamily: "Arial",
        fontSize: 10,
      },
    },
    objects: [{
      id: "obj_molecule_001",
      type: "molecule",
      visible: true,
      zIndex: 10,
      transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: { resourceRef: "mol_001" },
    }],
    resources: {
      mol_001: {
        type: "molecule_fragment2d",
        encoding: "chemsema.molecule.fragment2d",
        data: {
          schema: "chemsema.molecule.fragment2d",
          bbox: [95, 95, 20, 10],
          nodes: [
            { id: "n1", element: "C", atomicNumber: 6, position: [100, 100], charge: 0, numHydrogens: 0 },
            { id: "n2", element: "C", atomicNumber: 6, position: [110, 100], charge: 0, numHydrogens: 0 },
          ],
          bonds: [{ id: "b1", begin: "n1", end: "n2", order: 2 }],
        },
      },
    },
  };
}

function makeExactTieDoubleBondDocument() {
  return {
    format: { name: "chemsema", version: "0.1" },
    document: {
      id: "doc_exact_tie_double",
      title: "exact tie double bond",
      page: { width: 260, height: 220, background: "#ffffff" },
    },
    styles: {
      style_molecule_default: {
        kind: "molecule",
        stroke: "#000000",
        strokeWidth: 1,
        fontFamily: "Arial",
        fontSize: 10,
      },
    },
    objects: [{
      id: "obj_molecule_001",
      type: "molecule",
      visible: true,
      zIndex: 10,
      transform: { translate: [0, 0], rotate: 0, scale: [1, 1] },
      styleRef: "style_molecule_default",
      payload: { resourceRef: "mol_001" },
    }],
    resources: {
      mol_001: {
        type: "molecule_fragment2d",
        encoding: "chemsema.molecule.fragment2d",
        data: {
          schema: "chemsema.molecule.fragment2d",
          bbox: [100, 70, 30, 60],
          nodes: [
            { id: "n1", element: "C", atomicNumber: 6, position: [100, 100], charge: 0, numHydrogens: 0 },
            { id: "n2", element: "C", atomicNumber: 6, position: [130, 100], charge: 0, numHydrogens: 0 },
            { id: "n3", element: "C", atomicNumber: 6, position: [100, 70], charge: 0, numHydrogens: 0 },
            { id: "n4", element: "C", atomicNumber: 6, position: [100, 130], charge: 0, numHydrogens: 0 },
            { id: "n5", element: "C", atomicNumber: 6, position: [130, 70], charge: 0, numHydrogens: 0 },
          ],
          bonds: [
            { id: "b1", begin: "n1", end: "n2", order: 2, double: { placement: "right", frozen: false } },
            { id: "b2", begin: "n1", end: "n3", order: 1 },
            { id: "b3", begin: "n1", end: "n4", order: 1 },
            { id: "b4", begin: "n2", end: "n5", order: 1 },
          ],
        },
      },
    },
  };
}

async function verifyExactTieDoubleBondIncrementalReplacement(page) {
  await page.evaluate((doc) => window.__chemsemaDebug.loadDocumentForTest(doc), makeExactTieDoubleBondDocument());
  await page.waitForFunction(() => document.querySelectorAll('[data-layer="document-content"] [data-bond-id="b1"]').length === 2);
  await page.locator('button[data-tool="bond"]').click();
  await page.locator('button[data-secondary-value="bond-single"]').click();
  const points = await page.evaluate(() => ({
    start: window.__chemsemaDebug.worldToClient(130, 100),
    end: window.__chemsemaDebug.worldToClient(130, 130),
  }));

  await page.mouse.move(points.start.x, points.start.y);
  await page.mouse.down();
  await page.mouse.move(points.end.x, points.end.y, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(300);

  const result = await page.evaluate(() => {
    const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
    const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    const fragment = doc.resources?.mol_001?.data;
    return {
      command,
      bondCount: fragment?.bonds?.length || 0,
      placement: fragment?.bonds?.find((bond) => bond.id === "b1")?.double?.placement || null,
      centralPrimitiveCount: document.querySelectorAll(
        '[data-layer="document-content"] [data-bond-id="b1"]',
      ).length,
    };
  });
  const changedBonds = new Set([
    ...(result.command?.targets?.bonds || []),
    ...(result.command?.updated?.bonds || []),
  ]);
  assert.equal(result.bondCount, 5, `Fourth attachment was not committed: ${JSON.stringify(result)}`);
  assert.equal(result.placement, "left", `Exact tie did not follow the last attachment side: ${JSON.stringify(result)}`);
  assert(changedBonds.has("b1"), `Fourth attachment did not target the center double bond: ${JSON.stringify(result)}`);
  assert.equal(
    result.centralPrimitiveCount,
    2,
    `Incremental render left stale center-double primitives: ${JSON.stringify(result)}`,
  );
}

async function verifyDeleteToolFocusedBondCenter(page) {
  await page.evaluate((doc) => window.__chemsemaDebug.loadDocumentForTest(doc), makeShortDoubleBondDocument());
  await page.waitForFunction(() => document.querySelector('[data-bond-id="b1"]'));
  await page.locator('button[data-tool="delete"]').click();
  const center = await page.evaluate(() => window.__chemsemaDebug.worldToClient(105, 100));

  await page.mouse.move(center.x, center.y);
  await page.mouse.click(center.x, center.y);

  await page.waitForFunction(() => {
    const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return doc.resources?.mol_001?.data?.bonds?.[0]?.order === 1;
  });
  const after = await page.evaluate(() => {
    const doc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson());
    return doc.resources?.mol_001?.data || null;
  });
  assert.equal(after?.nodes?.length, 2, `Delete tool removed the short double bond endpoint: ${JSON.stringify(after)}`);
  assert.equal(after?.bonds?.length, 1, `Delete tool removed the short double bond instead of degrading it: ${JSON.stringify(after)}`);
  assert.equal(after?.bonds?.[0]?.order, 1, `Delete tool did not degrade the focused double bond: ${JSON.stringify(after)}`);
}

async function verifyCopyPasteCut(page) {
  await drawBondWithMouse(page);
  const before = await documentSummary(page);
  await page.keyboard.press("Control+A");
  await page.waitForFunction(() => JSON.parse(window.__chemsemaDebug.state.editorEngine.selectionBoundsJson() || "null"));
  const selected = await documentSummary(page);
  assert(selected.selectionBounds, `Ctrl+A did not create a selection: ${JSON.stringify(selected)}`);

  await page.locator('button[data-command="copy"]').click();
  const copied = await page.evaluate(() => {
    const engine = window.__chemsemaDebug.state.editorEngine;
    return {
      hasClipboard: engine.hasClipboard?.() || false,
      fragmentLength: engine.clipboardSelectionJson?.()?.length || 0,
      documentLength: engine.clipboardDocumentJson?.()?.length || 0,
    };
  });
  assert(copied.hasClipboard && copied.fragmentLength > 20, `Copy did not populate the internal clipboard: ${JSON.stringify(copied)}`);

  await page.locator('button[data-command="paste"]').click();
  await page.waitForFunction((count) => document.querySelectorAll("[data-bond-id]").length > count, before.renderedBonds);
  const pasted = await documentSummary(page);
  assert(pasted.renderedBonds > before.renderedBonds, `Paste did not duplicate the selected structure: ${JSON.stringify({ before, pasted })}`);

  await page.keyboard.press("Control+A");
  await page.locator('button[data-command="cut"]').click();
  try {
    await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length === 0);
  } catch (error) {
    const diagnostic = await page.evaluate(() => {
      const engine = window.__chemsemaDebug.state.editorEngine;
      return {
        document: JSON.parse(engine.documentJson()),
        selectionBounds: JSON.parse(engine.selectionBoundsJson() || "null"),
        lastCommandResult: JSON.parse(engine.lastCommandResultJson?.() || "null"),
        renderedBonds: document.querySelectorAll("[data-bond-id]").length,
      };
    });
    throw new Error(`Cut did not clear the selected document: ${JSON.stringify(diagnostic)}`, { cause: error });
  }
  const cut = await documentSummary(page);
  assert.equal(cut.renderedBonds, 0, `Cut did not remove selected content: ${JSON.stringify(cut)}`);

  await page.locator('button[data-command="paste"]').click();
  await page.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
  const restored = await documentSummary(page);
  assert(restored.renderedBonds > 0, `Paste after cut did not restore content: ${JSON.stringify(restored)}`);
}

async function dispatchImageTransfer(page, eventType) {
  await page.evaluate(async (type) => {
    const canvas = document.createElement("canvas");
    canvas.width = 2;
    canvas.height = 2;
    const context = canvas.getContext("2d");
    context.fillStyle = "#2574a9";
    context.fillRect(0, 0, 2, 2);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/png"));
    const file = new File([blob], "regression.png", { type: "image/png" });
    const transfer = new DataTransfer();
    transfer.items.add(file);
    const target = type === "drop" ? document.querySelector("#viewer-container") : document;
    const event = type === "drop"
      ? new DragEvent("drop", {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer,
        clientX: 500,
        clientY: 400,
      })
      : new ClipboardEvent("paste", {
        bubbles: true,
        cancelable: true,
        clipboardData: transfer,
      });
    target.dispatchEvent(event);
  }, eventType);
}

async function verifyImageDropAndPaste(page) {
  await dispatchImageTransfer(page, "drop");
  try {
    await page.waitForFunction(() => {
      const engine = window.__chemsemaDebug.state.editorEngine;
      const documentValue = JSON.parse(engine.documentJson());
      return documentValue.objects?.some((object) => object.type === "image")
        && window.__chemsemaDebug.editorState?.activeTool === "select"
        && JSON.parse(engine.selectionBoundsJson() || "null");
    });
  } catch (error) {
    const diagnostic = await page.evaluate(() => ({
      drop: window.__chemsemaDebug.lastFileDrop || null,
      activeTool: window.__chemsemaDebug.editorState?.activeTool || null,
      document: JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson()),
      selectionBounds: JSON.parse(
        window.__chemsemaDebug.state.editorEngine.selectionBoundsJson() || "null",
      ),
    }));
    throw new Error(`Dropped image did not become the selected object: ${JSON.stringify(diagnostic)}`, { cause: error });
  }

  const afterDrop = await page.evaluate(() => (
    JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson())
      .objects.filter((object) => object.type === "image").length
  ));
  await dispatchImageTransfer(page, "paste");
  await page.waitForFunction((count) => (
    JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson())
      .objects.filter((object) => object.type === "image").length > count
  ), afterDrop);

  const afterPaste = await page.evaluate(() => (
    JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson())
      .objects.filter((object) => object.type === "image").length
  ));
  const canvas = await page.locator("#viewer-container").boundingBox();
  assert(canvas, "Viewer container is not visible for context-menu image insertion.");
  await page.mouse.click(canvas.x + canvas.width * 0.72, canvas.y + canvas.height * 0.68, {
    button: "right",
  });
  const insertImage = page.locator(
    '.canvas-context-menu-item[data-canvas-context-command="insert-image"]',
  );
  await insertImage.waitFor();
  const chooserPromise = page.waitForEvent("filechooser");
  await insertImage.click();
  const chooser = await chooserPromise;
  const pngBase64 = await page.evaluate(async () => {
    const imageCanvas = document.createElement("canvas");
    imageCanvas.width = 2;
    imageCanvas.height = 2;
    const context = imageCanvas.getContext("2d");
    context.fillStyle = "#8e44ad";
    context.fillRect(0, 0, 2, 2);
    const blob = await new Promise((resolve) => imageCanvas.toBlob(resolve, "image/png"));
    const bytes = new Uint8Array(await blob.arrayBuffer());
    let binary = "";
    for (const value of bytes) {
      binary += String.fromCharCode(value);
    }
    return btoa(binary);
  });
  await chooser.setFiles({
    name: "context-menu-regression.png",
    mimeType: "image/png",
    buffer: Buffer.from(pngBase64, "base64"),
  });
  await page.waitForFunction((count) => (
    JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson())
      .objects.filter((object) => object.type === "image").length > count
  ), afterPaste);
}

async function verifyStructuredClipboardAcrossTabs(context, page, errors) {
  await drawBondWithMouse(page);
  await page.keyboard.press("Control+A");
  await page.locator('button[data-command="copy"]').click();
  await page.waitForFunction(async () => (
    (await navigator.clipboard.readText()).includes("<CDXML")
  ));

  const nextPagePromise = context.waitForEvent("page");
  await page.locator('button.icon-button[data-command="new"]').click();
  const nextPage = await nextPagePromise;
  nextPage.setDefaultTimeout(12000);
  capturePageErrors(nextPage, errors);
  await waitForReady(nextPage);
  assert.equal(
    await nextPage.locator("[data-bond-id]").count(),
    0,
    "A new tab unexpectedly inherited the previous tab document.",
  );

  await nextPage.locator('button[data-command="paste"]').click();
  await nextPage.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
  await nextPage.close();
}

async function verifyDesktopTabDetach(page) {
  const result = await page.evaluate(async () => {
    const [{ createDocumentTabInteractions }, { createUiActionRunner }] = await Promise.all([
      import("./document_tab_interactions.js"),
      import("./ui_action_runner.js"),
    ]);
    const fixture = document.createElement("div");
    fixture.innerHTML = `
      <div data-test-titlebar style="position:fixed;left:0;top:0;width:500px;height:42px"></div>
      <div data-test-tabs>
        <div data-document-tab-id="tab-1" style="width:160px;height:30px">Example</div>
      </div>
    `;
    document.body.appendChild(fixture);
    const root = fixture.querySelector("[data-test-tabs]");
    const titlebar = fixture.querySelector("[data-test-titlebar]");
    const tab = fixture.querySelector("[data-document-tab-id]");
    tab.setPointerCapture = () => {};
    const detached = [];
    const failures = [];
    const interactions = createDocumentTabInteractions({
      root,
      titlebar,
      detachEnabled: () => true,
      uiActions: createUiActionRunner({
        onFailure: (failure) => failures.push(failure.message),
      }),
      closeDocumentTab: async () => {},
      activateDocumentTab: async () => {},
      detachDocumentTab: async (...args) => detached.push(args),
    });
    interactions.bind();
    const dispatch = (type, clientX, clientY) => tab.dispatchEvent(new PointerEvent(type, {
      bubbles: true,
      cancelable: true,
      button: 0,
      pointerId: 7,
      clientX,
      clientY,
      screenX: clientX + 100,
      screenY: clientY + 100,
    }));
    dispatch("pointerdown", 40, 20);
    dispatch("pointermove", 65, 90);
    dispatch("pointerup", 65, 90);
    await new Promise((resolve) => setTimeout(resolve, 0));
    const outcome = {
      detached,
      failures,
      dragging: interactions.isDetaching("tab-1"),
    };
    fixture.remove();
    return outcome;
  });
  assert.deepEqual(result.failures, []);
  assert.equal(result.dragging, false, "Detached-tab drag state was not cleared.");
  assert.equal(result.detached.length, 1, "Desktop tab drag did not request a detached window.");
  assert.equal(result.detached[0][0], "tab-1");
}

async function verifySaveAsFormats(page) {
  await drawBondWithMouse(page);
  await setSaveQueue(page, ["gui-save.ccjs", "gui-export.cdxml", "gui-export.svg", "gui-save-again.ccjz"]);

  await page.locator('button[data-command="save-as"]').click();
  const ccjs = await waitForSaveWrite(page, 0);
  assert.equal(ccjs.name, "gui-save.ccjs");
  assert(ccjs.text.includes('"objects"') && JSON.parse(ccjs.text).resources, "Save As .ccjs did not write a ChemSema JSON document.");

  await page.locator('button[data-command="save-as"]').click();
  const cdxml = await waitForSaveWrite(page, 1);
  assert.equal(cdxml.name, "gui-export.cdxml");
  assert(/<CDXML\b/i.test(cdxml.text), "Save As .cdxml did not write CDXML content.");

  await page.locator('button[data-command="save-as"]').click();
  const svg = await waitForSaveWrite(page, 2);
  assert.equal(svg.name, "gui-export.svg");
  assert(/<svg\b/i.test(svg.text), "Save As .svg did not write SVG content.");

  await page.keyboard.press("Control+S");
  const ccjz = await waitForSaveWrite(page, 3);
  assert.equal(ccjz.name, "gui-save-again.ccjz");
  assert(ccjz.byteLength > 100, `Ctrl+S .ccjz save was unexpectedly small: ${JSON.stringify(ccjz)}`);
}

async function createOpenFixture(context, errors) {
  const page = await openViewer(context, errors);
  await drawBondWithMouse(page);
  const documentJson = await page.evaluate(() => window.__chemsemaDebug.state.editorEngine.documentJson());
  await page.close();
  const fixturePath = join(tmpDir, "gui-open-source.ccjs");
  writeFileSync(fixturePath, `${JSON.stringify(JSON.parse(documentJson), null, 2)}\n`, "utf8");
  return fixturePath;
}

async function verifyOpenButton(context, errors, fixturePath) {
  const page = await openViewer(context, errors);
  const popupPromise = page.waitForEvent("popup", { timeout: 8000 }).catch(() => null);
  const chooserPromise = page.waitForEvent("filechooser");
  await page.locator('button[data-command="open"]').click();
  const chooser = await chooserPromise;
  await chooser.setFiles(fixturePath);
  const popup = await popupPromise;
  const openedPage = popup || page;
  if (popup) {
    capturePageErrors(popup, errors);
    popup.setDefaultTimeout(12000);
  }
  await waitForReady(openedPage);
  await openedPage.waitForFunction(() => document.querySelectorAll("[data-bond-id]").length > 0);
  const opened = await documentSummary(openedPage);
  assert.equal(opened.currentFileName, "gui-open-source.ccjs", `Open did not preserve the file name: ${JSON.stringify(opened)}`);
  assert(opened.renderedBonds > 0 && opened.resourceBonds > 0, `Open did not render the saved document: ${JSON.stringify(opened)}`);
  await openedPage.close();
  if (popup) {
    await page.close().catch(() => {});
  }
}

async function verifyZoomAndStyleMenu(page) {
  await page.locator("#zoom-input").selectOption("150");
  const zoom = await page.locator("#zoom-input").inputValue();
  assert.equal(zoom, "150", "Zoom select did not accept 150%.");

  await page.locator("#document-style-button").click();
  await page.locator('[data-document-style-preset="acs-document-1996"]').click();
  const styleState = await page.evaluate(() => ({
    menuHidden: document.querySelector("#document-style-menu")?.hidden,
    preset: window.__chemsemaDebug?.state?.editorEngine?.documentStylePreset?.() || null,
  }));
  assert.equal(styleState.menuHidden, true, `Style menu did not close after selection: ${JSON.stringify(styleState)}`);
  assert.equal(styleState.preset, "acs-document-1996", `Style preset was not applied: ${JSON.stringify(styleState)}`);
}

async function verifyChemicalPropertyDialog(page) {
  await drawBondWithMouse(page);
  await page.locator('button[data-tool="select"]').click();
  const bond = page.locator("[data-bond-id]").first();
  const box = await bond.boundingBox();
  assert(box, "ChemicalProperty regression could not locate the molecule bond.");
  await page.keyboard.press("Control+A");
  await page.waitForFunction(() => {
    const bounds = window.__chemsemaDebug?.state?.editorEngine?.selectionBoundsJson?.();
    return bounds && JSON.parse(bounds) != null;
  });
  const viewer = await page.locator("#viewer-container").boundingBox();
  assert(viewer, "ChemicalProperty regression could not locate the viewer.");
  await page.mouse.click(viewer.x + 12, viewer.y + 12, { button: "right" });
  const command = page.locator(
    '.canvas-context-menu button[data-canvas-context-command="chemical-property-dialog"]',
  );
  await command.waitFor({ state: "visible" });
  await command.click();

  const dialog = page.locator(".chemical-property-dialog");
  await dialog.waitFor({ state: "visible" });
  await dialog.locator('input[name="value"]').fill("ethane");
  await dialog.locator('input[name="isActive"]').check();
  await dialog.locator('button[type="submit"]').click();
  await page.waitForFunction(() => {
    const properties = window.__chemsemaDebug?.document?.chemicalProperties;
    return Array.isArray(properties)
      && properties.length === 1
      && properties[0]?.propertyType?.code === 1
      && properties[0]?.isActive === true;
  });
  const result = await page.evaluate(() => {
    const documentValue = window.__chemsemaDebug.document;
    const property = documentValue.chemicalProperties[0];
    const display = documentValue.objects.find((object) => object.id === property.displayObjectId);
    return {
      value: display?.payload?.text,
      relation: documentValue.links.find(
        (candidate) => candidate.kind === "chemical-property-display",
      ),
    };
  });
  assert.equal(result.value, "ethane");
  assert.equal(result.relation?.endpoints?.length, 2);
}

async function verifyAnnotationDialog(page) {
  const cdxml = `<CDXML BondLength="14.4"><page id="1">
    <fragment id="10"><n id="101" p="80 100"/><n id="102" p="140 100"/><b id="103" B="101" E="102"/></fragment>
    <constraint id="201" ConstraintType="Distance" ConstraintMin="0.5" ConstraintMax="1.5" BasisObjects="101 102">
      <objecttag TagType="Unknown" Name="distance"><t p="110 90"><s>1.0 Å</s></t></objecttag>
    </constraint>
  </page></CDXML>`;
  await page.evaluate(async (source) => {
    const debug = window.__chemsemaDebug;
    await debug.state.editorEngine.loadDocumentCdxml(source);
    await debug.loadDocumentForTest(
      JSON.parse(debug.state.editorEngine.documentJson()),
    );
  }, cdxml);
  await page.locator('button[data-tool="select"]').click();
  const annotationId = await page.evaluate(() => {
    const documentValue = JSON.parse(
      window.__chemsemaDebug.state.editorEngine.documentJson(),
    );
    return documentValue.objects.find((object) => object.type === "constraint")?.id;
  });
  assert(annotationId, "Imported annotation did not create a native constraint object.");
  const annotationPoint = await page.evaluate(
    () => window.__chemsemaDebug.worldToClient(110, 90),
  );
  await page.mouse.click(annotationPoint.x, annotationPoint.y, { button: "right" });
  const propertiesCommand = page.locator(
    '.canvas-context-menu button[data-canvas-context-command="annotation-dialog"]',
  );
  await propertiesCommand.waitFor({ state: "visible" });
  await propertiesCommand.click();

  const dialog = page.locator(".annotation-dialog");
  await dialog.waitFor({ state: "visible" });
  assert.equal(await dialog.locator('select[name="positioningType"]').inputValue(), "auto");
  await dialog.locator('input[name="minimum"]').fill("0.75");
  await dialog.locator('input[name="maximum"]').fill("1.75");
  await dialog.locator('button[type="submit"]').click();
  await page.waitForFunction(() => {
    const documentValue = JSON.parse(
      window.__chemsemaDebug.state.editorEngine.documentJson(),
    );
    return documentValue.objects.some(
      (object) => object.type === "constraint"
        && object.payload?.constraint?.constraintType === "distance"
        && object.payload.constraint.minimum === 0.75
        && object.payload.constraint.maximum === 1.75,
    );
  });
  const result = await page.evaluate(() => {
    const documentValue = JSON.parse(
      window.__chemsemaDebug.state.editorEngine.documentJson(),
    );
    const annotation = documentValue.objects.find(
      (object) => object.type === "constraint",
    );
    return {
      basis: annotation?.payload?.constraint?.basisEntityIds,
      relation: documentValue.links.find(
        (candidate) => candidate.kind === "annotation-basis",
      ),
    };
  });
  assert.equal(result.basis?.length, 2);
  assert.equal(result.relation?.endpoints?.length, 3);
}

async function verifyLogicalObjectsDialog(page) {
  await drawBondWithMouse(page);
  await page.locator('button[data-tool="select"]').click();
  await page.keyboard.press("Control+A");
  const viewer = await page.locator("#viewer-container").boundingBox();
  assert(viewer, "Logical object regression could not locate the viewer.");
  await page.mouse.click(viewer.x + 12, viewer.y + 12, { button: "right" });
  const command = page.locator(
    '.canvas-context-menu button[data-canvas-context-command="logical-objects-dialog"]',
  );
  await command.waitFor({ state: "visible" });
  await command.click();

  const dialog = page.locator(".logical-objects-dialog");
  await dialog.waitFor({ state: "visible" });
  await dialog.locator('[data-logical-family="annotation"]').click();
  await dialog.locator("[data-logical-new]").click();
  await dialog.locator('input[name="keyword"]').fill("source");
  await dialog.locator('input[name="content"]').fill("browser regression");
  await dialog.locator('button[type="submit"]').click();
  await page.waitForFunction(() => {
    const documentValue = JSON.parse(
      window.__chemsemaDebug.state.editorEngine.documentJson(),
    );
    return documentValue.logicalObjects?.annotations?.some(
      (annotation) => annotation.keyword === "source"
        && annotation.content === "browser regression"
        && annotation.ownerEntityId,
    );
  });

  await dialog.locator('input[name="content"]').fill("browser regression edited");
  await dialog.locator('button[type="submit"]').click();
  await page.waitForFunction(() => {
    const documentValue = JSON.parse(
      window.__chemsemaDebug.state.editorEngine.documentJson(),
    );
    return documentValue.logicalObjects?.annotations?.length === 1
      && documentValue.logicalObjects.annotations[0].content === "browser regression edited";
  });

  const exported = await page.evaluate(
    () => window.__chemsemaDebug.state.editorEngine.documentCdxml(),
  );
  assert.match(exported, /<annotation\b/);
  assert.match(exported, /Keyword="source"/);
  await dialog.locator("[data-logical-delete]").click();
  await page.waitForFunction(() => {
    const documentValue = JSON.parse(
      window.__chemsemaDebug.state.editorEngine.documentJson(),
    );
    return !documentValue.logicalObjects?.annotations?.length;
  });
  await dialog.locator(".logical-objects-titlebar [data-logical-close]").click();
}

let server = null;
let browser = null;
try {
  mkdirSync(tmpDir, { recursive: true });
  server = await ensureServer();
  browser = await chromium.launch({
    headless: true,
    executablePath: existsSync(edgePath) ? edgePath : undefined,
  });
  const context = await browser.newContext({ acceptDownloads: true });
  await context.grantPermissions(["clipboard-read", "clipboard-write"], {
    origin: new URL(baseUrl).origin,
  });
  await installBrowserMocks(context);
  const errors = [];

  if (!selectionSummaryOnly && !chemicalPropertyOnly && !annotationOnly && !documentLayoutOnly && !logicalObjectsOnly) {
    const fixturePath = await createOpenFixture(context, errors);
    await verifyOpenButton(context, errors, fixturePath);

    const exactTiePage = await openViewer(context, errors);
    await verifyExactTieDoubleBondIncrementalReplacement(exactTiePage);
    await exactTiePage.close();
  }

  if (selectionSummaryOnly) {
    const page = await openViewer(context, errors);
    await verifySelectionOverlayConsistency(page);
    await page.close();
  } else if (chemicalPropertyOnly) {
    const page = await openViewer(context, errors);
    await verifyChemicalPropertyDialog(page);
    await page.close();
  } else if (annotationOnly) {
    const page = await openViewer(context, errors);
    await verifyAnnotationDialog(page);
    await page.close();
  } else if (documentLayoutOnly) {
    const page = await openViewer(context, errors);
    await verifyDocumentLayoutControls(page);
    await page.close();
  } else if (logicalObjectsOnly) {
    const page = await openViewer(context, errors);
    await verifyLogicalObjectsDialog(page);
    await page.close();
  } else if (!exactTieOnly) {
    const page = await openViewer(context, errors);
    await verifyToolbarAndCursor(page);
    await verifySelectionOverlayConsistency(page);
    await verifyDeleteToolFocusedBondCenter(page);
    await page.close();

    const editPage = await openViewer(context, errors);
    await verifyCopyPasteCut(editPage);
    await editPage.close();

    const imagePage = await openViewer(context, errors);
    await verifyImageDropAndPaste(imagePage);
    await imagePage.close();

    const crossTabPage = await openViewer(context, errors);
    await verifyStructuredClipboardAcrossTabs(context, crossTabPage, errors);
    await crossTabPage.close();

    const detachedTabPage = await openViewer(context, errors);
    await verifyDesktopTabDetach(detachedTabPage);
    await detachedTabPage.close();

    const savePage = await openViewer(context, errors);
    await verifySaveAsFormats(savePage);
    await verifyZoomAndStyleMenu(savePage);
    await savePage.close();

    const chemicalPropertyPage = await openViewer(context, errors);
    await verifyChemicalPropertyDialog(chemicalPropertyPage);
    await chemicalPropertyPage.close();

    const annotationPage = await openViewer(context, errors);
    await verifyAnnotationDialog(annotationPage);
    await annotationPage.close();
  }

  assert.equal(errors.length, 0, `GUI regression saw console/page errors:\n${errors.join("\n")}`);
  console.log(selectionSummaryOnly
    ? "[gui-regression] ok (selection summary and minimum selection box)"
    : chemicalPropertyOnly
      ? "[gui-regression] ok (ChemicalProperty context menu and kernel dialog)"
      : annotationOnly
        ? "[gui-regression] ok (Geometry/Constraint context menu and kernel dialog)"
      : documentLayoutOnly
        ? "[gui-regression] ok (40 px status bar and document layout controls)"
      : logicalObjectsOnly
        ? "[gui-regression] ok (native logical object manager and CDXML export)"
    : exactTieOnly
      ? "[gui-regression] ok (exact-tie double bond)"
      : "[gui-regression] ok (open, save-as ccjs/cdxml/svg, ctrl+s ccjz, copy/paste/cut, image drop/paste/context-menu, cross-tab structured clipboard, detached desktop tab, toolbar icons, cursors, selection overlay, delete tool, exact-tie double bond, zoom, style, annotation dialog)");
} finally {
  await browser?.close();
  if (server) {
    server.kill();
  }
}
