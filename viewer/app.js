import {
  parseEngineJson,
  primitivesForObject,
  renderBoundsFromEngine,
  renderListFromEngine,
} from "./engine_bridge.js";
import { createColorHost } from "./color_host.js";
import { createObjectSettingsHost } from "./object_settings_host.js";
import { createNumericDialogHost } from "./numeric_dialog_host.js";
import { createDesktopFileHost } from "./desktop_file_host.js";
import { createEngineHost } from "./engine_host.js";
import { bindEditorControls } from "./editor_bindings.js";
import { createDocumentFlow } from "./document_flow.js";
import {
  chemcoreOpenAcceptString,
  chemcoreOpenAcceptTypes,
  decompressChemcoreText,
  looksLikeCdxmlFile,
  looksLikeCompressedChemcoreFile,
  saveFormatFromFileName,
} from "./file_io.js";
import {
  boundsCenter,
  boundsSize,
  boundsToKey,
  intersectBounds,
  paddedViewBoxFromBounds,
  pointDistance,
  rectContainsBounds,
} from "./geometry.js";
import {
  TEXT_FONT_OPTIONS,
  TEXT_FONT_SIZE_OPTIONS,
  formatToolbarFontSize,
  normalizeToolbarFontSize,
  renderSecondaryToolbarHtml,
  syncPrimaryChromeIcons,
  syncPrimaryToolButtons,
} from "./toolbar.js";
import {
  displayLabelFontFamily,
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";
import {
  editorScriptScale as computeEditorScriptScale,
  estimateTextRunsWidth as computeEstimateTextRunsWidth,
  normalizeSharedGlyphProfiles,
  textLength,
} from "./text_metrics.js";
import {
  normalizeEditorSelectionOffsets as normalizeEditorSelectionOffsetsModel,
  normalizeEditorSourceRuns as normalizeEditorSourceRunsModel,
  runsPlainText,
  splitRunsForSelection,
  styleAtEditorOffset as styleAtEditorOffsetModel,
} from "./text_editor_model.js";
import {
  editorSourceRunsFromSession as createEditorSourceRunsFromSession,
} from "./text_editor_render.js";
import { createTextEditorController } from "./text_editor_controller.js";
import {
  createTextSymbolPalette,
  loadTextSymbolCatalog,
} from "./text_symbol_palette.js";
import {
  renderLineObject,
  renderShapeObject,
  renderTextObject,
} from "./object_fallbacks.js";
import {
  primitiveStrokeWidthValue,
  renderCorePrimitive,
} from "./primitive_dom_renderer.js";
import {
  CSS_PX_PER_CM,
  cmToCssPx,
  cssPxToCm,
  displayMetrics,
  mapLengthArray,
} from "./units.js";

const SAMPLE_FILES = [
  "../tmp/examples/02-13/2017-2-13/oleObject1.ccjs",
  "../tmp/examples/02-13/2017-2-13/oleObject2.ccjs",
  "../tmp/examples/02-13/2017-2-13/oleObject3.ccjs",
  "../tmp/examples/02-13/2017-2-13/oleObject4.ccjs",
  "../tmp/examples/02-13/lm 2017-2-13  working report/oleObject1.ccjs",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject1.ccjs",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject2.ccjs",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject3.ccjs",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject4.ccjs",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject5.ccjs",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject6.ccjs",
];

const VIEW_MODE = document.body.dataset.viewMode || "document";
const LABEL_DEBUG_MODE = VIEW_MODE === "label-debug";

const state = {
  currentPath: LABEL_DEBUG_MODE ? SAMPLE_FILES[0] : null,
  currentFileName: null,
  currentFilePath: null,
  currentDocument: null,
  editorEngine: null,
  documentEngine: null,
  coreRenderList: null,
  runtimeViewBox: null,
  lastEditFocusPoint: null,
  activeBracketDragStart: null,
  zoomHandoffs: [],
  programmaticScrollTimer: null,
  isProgrammaticScroll: false,
  expectedProgrammaticScroll: null,
  displayMetrics: displayMetrics(),
  pendingTextSymbol: null,
};
const engineHost = createEngineHost();
const desktopFileHost = createDesktopFileHost();
const colorHost = createColorHost();
const objectSettingsHost = createObjectSettingsHost({
  root: document.body,
  engine: () => state.editorEngine,
  onApply: async () => {
    await syncDocumentFromEngine();
    renderDocument();
  },
});
const numericDialogHost = createNumericDialogHost({
  root: document.body,
  engine: () => state.editorEngine,
  onApply: async () => {
    await syncDocumentFromEngine();
    renderDocument();
  },
});
const isDesktopShell = !!desktopFileHost?.available;
let sharedGlyphProfiles = null;
const sharedGlyphProfilesReady = loadSharedGlyphProfiles();

document.body.classList.toggle("desktop-shell", isDesktopShell);
document.body.classList.toggle("browser-shell", !isDesktopShell);

if (typeof window !== "undefined") {
  window.__chemcoreDebug = {
    state,
    get document() {
      return state.currentDocument;
    },
    get engineState() {
      return currentEditorEngineState();
    },
    get activeTextEditor() {
      return activeTextEditor;
    },
    get displayMetrics() {
      return state.displayMetrics;
    },
    get engineHost() {
      return engineHost;
    },
    get desktopFileHost() {
      return desktopFileHost;
    },
    insertEditorText(text) {
      if (!activeTextEditor) {
        return false;
      }
      for (const character of Array.from(String(text || ""))) {
        textEditorController.insertTextAtSelection(character);
      }
      return true;
    },
    async syncDocument() {
      await state.editorEngine?.ready?.();
      await syncDocumentFromEngine();
      renderDocument();
      return state.currentDocument;
    },
    worldToClient(x, y) {
      const matrix = viewerSvg?.getScreenCTM?.();
      if (!matrix) {
        return null;
      }
      const point = new DOMPoint(x, y).matrixTransform(matrix);
      return { x: point.x, y: point.y };
    },
  };
}

const DEFAULT_TEXT_FONT_SIZE = 10;
const BOND_STROKE = 1.0;
const CHEMDRAW_PAGE_BACKGROUND = "#ffffff";
const DEFAULT_WORKSPACE_WIDTH = 900;
const DEFAULT_WORKSPACE_HEIGHT = 600;
const EDITOR_VIEW_BUFFER_RATIO = 0.6;
const EDITOR_AUTO_EXPAND_TRIGGER_RATIO = 0.18;
const EDITOR_FIT_PADDING_RATIO = 0.08;
const ZOOM_STEP_LEVELS = [12, 25, 50, 75, 100, 150, 200, 400, 600, 800];
const ZOOM_MIN_PERCENT = ZOOM_STEP_LEVELS[0];
const ZOOM_MAX_PERCENT = ZOOM_STEP_LEVELS[ZOOM_STEP_LEVELS.length - 1];
const SELECTION_ROTATE_HANDLE_OFFSET_PX = 26;
const SELECTION_ROTATE_HANDLE_RADIUS_PX = 6;
const SELECTION_ROTATE_HANDLE_HIT_RADIUS_PX = 12;
const SELECTION_RESIZE_HANDLE_SIZE_PX = 3;
const SELECTION_RESIZE_HANDLE_HIT_RADIUS_PX = 14;
const SELECTION_RESIZE_MIN_SCALE = 0.05;
const DELETE_CURSOR_SVG = encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
    <rect x="4" y="4" width="8" height="8" fill="#ffffff" stroke="#000000" stroke-width="1"/>
  </svg>`,
);
const DELETE_CURSOR = `url("data:image/svg+xml,${DELETE_CURSOR_SVG}") 8 8, crosshair`;

const sampleSelect = document.getElementById("sample-select");
const reloadButton = document.getElementById("reload-button");
const fitButton = document.getElementById("fit-button");
const toggleMolecules = document.getElementById("toggle-molecules");
const toggleLines = document.getElementById("toggle-lines");
const toggleTexts = document.getElementById("toggle-texts");
const docMeta = document.getElementById("doc-meta");
const viewerTitle = document.getElementById("viewer-title");
const viewerStats = document.getElementById("viewer-stats");
const viewerSvg = document.getElementById("viewer-svg");
const viewerContainer = document.getElementById("viewer-container");
const secondaryToolbar = document.getElementById("secondary-toolbar");
const desktopTitlebar = document.getElementById("desktop-titlebar");
const documentTabsRoot = document.getElementById("document-tabs");
const documentStyleButton = document.getElementById("document-style-button");
const documentStyleMenu = document.getElementById("document-style-menu");
const openFileInput = document.createElement("input");
openFileInput.type = "file";
openFileInput.accept = chemcoreOpenAcceptString();
openFileInput.className = "visually-hidden";
document.body.appendChild(openFileInput);
const textEditorLayer = document.createElement("div");
textEditorLayer.className = "text-editor-layer";
viewerContainer?.appendChild(textEditorLayer);
const canvasContextMenu = createCanvasContextMenu();
document.body.appendChild(canvasContextMenu);
let activeContextMenuState = null;
let textSymbolPalette = null;
const textSymbolCatalogReady = loadTextSymbolCatalog().then((catalog) => {
  textSymbolPalette = createTextSymbolPalette({
    mount: viewerContainer,
    catalog,
    onSelect: insertTextSymbol,
  });
  return textSymbolPalette;
});
const appRuntimeReady = Promise.all([
  engineHost.initialize(),
  sharedGlyphProfilesReady,
  textSymbolCatalogReady,
]);

syncPrimaryChromeIcons();
bindDesktopWindowChrome();

if (sampleSelect) {
  for (const samplePath of SAMPLE_FILES) {
    const option = document.createElement("option");
    option.value = samplePath;
    option.textContent = samplePath.replace("../tmp/examples/", "");
    sampleSelect.appendChild(option);
  }

  sampleSelect.value = state.currentPath;
  sampleSelect.addEventListener("change", async (event) => {
    state.currentPath = event.target.value;
    await loadAndRender();
  });
}

reloadButton?.addEventListener("click", async () => {
  await loadAndRender();
});

fitButton?.addEventListener("click", () => {
  fitView();
});

toggleMolecules?.addEventListener("change", () => renderDocument());
toggleLines?.addEventListener("change", () => renderDocument());
toggleTexts?.addEventListener("change", () => renderDocument());

const zoomInput = document.getElementById("zoom-input");
let zoomPercent = 100;
const documentTabs = [];
let activeDocumentTabId = null;
const BROWSER_PENDING_DOCUMENT_KEY_PREFIX = "chemcore:pending-browser-document:";
const BROWSER_PENDING_DOCUMENT_PARAM = "chemcorePendingDocument";
let activeTitlebarTabDrag = null;
let detachingDocumentTabId = null;
let suppressNextDocumentTabClick = false;
let activeDocumentPreviewObjectIds = new Set();
let activeDocumentPreviewLayer = false;
let activeDocumentPreviewTransform = "";

const syncWindowTitle = () => {
  updateActiveDocumentTabTitle();
  const title = activeDocumentTab()?.title || String(viewerTitle?.textContent || "Untitled").trim() || "Untitled";
  document.title = `${title} - Chemcore`;
  desktopFileHost?.setWindowTitle?.(title).catch?.(() => {});
};

if (viewerTitle) {
  new MutationObserver(syncWindowTitle).observe(viewerTitle, {
    childList: true,
    characterData: true,
    subtree: true,
  });
  syncWindowTitle();
}

const editorState = {
  activeTool: "bond",
  selectMode: "box",
  bondType: "single",
  textFontFamily: "Arial",
  textFontSize: cmToCssPx(DEFAULT_TEXT_FONT_SIZE),
  textColor: "#000000",
  selectionColor: "#000000",
  textAlign: "left",
  textBold: false,
  textItalic: false,
  textUnderline: false,
  textScript: "normal",
  arrowType: "solid",
  arrowHeadSize: "small",
  arrowCurve: "270",
  arrowHeadStyle: "full",
  arrowTailStyle: "none",
  arrowHead: true,
  arrowTail: false,
  arrowBold: false,
  arrowNoGo: "none",
  shapeKind: "circle",
  shapeStyle: "solid",
  shapeColor: "#000000",
  documentColors: [],
  bracketKind: "round",
  symbolKind: "circle-plus",
  template: "ring-6",
};
let activeTextEditor = null;
let activeSelectionGesture = null;

const TAB_STATE_KEYS = [
  "currentPath",
  "currentFileName",
  "currentFilePath",
  "currentDocument",
  "editorEngine",
  "documentEngine",
  "coreRenderList",
  "runtimeViewBox",
  "lastEditFocusPoint",
  "activeBracketDragStart",
  "zoomHandoffs",
];

function activeDocumentTab() {
  return documentTabs.find((tab) => tab.id === activeDocumentTabId) || null;
}

function createDocumentTab(title = "Untitled") {
  return {
    id: `doc-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    title,
    zoomPercent: 100,
    currentPath: null,
    currentFileName: null,
    currentFilePath: null,
    currentDocument: null,
    editorEngine: null,
    documentEngine: null,
    coreRenderList: null,
    runtimeViewBox: null,
    lastEditFocusPoint: null,
    activeBracketDragStart: null,
    zoomHandoffs: [],
  };
}

function ensureDocumentTab() {
  if (activeDocumentTab()) {
    return activeDocumentTab();
  }
  const tab = createDocumentTab();
  documentTabs.push(tab);
  activeDocumentTabId = tab.id;
  return tab;
}

function saveActiveDocumentTabState() {
  const tab = activeDocumentTab();
  if (!tab) {
    return;
  }
  for (const key of TAB_STATE_KEYS) {
    tab[key] = state[key];
  }
  tab.zoomPercent = zoomPercent;
  tab.title = documentTitleFromState();
}

async function restoreDocumentTabState(tab) {
  for (const key of TAB_STATE_KEYS) {
    state[key] = tab[key];
  }
  zoomPercent = Number(tab.zoomPercent || 100);
  activeSelectionGesture = null;
  textEditorLayer.replaceChildren();
  activeTextEditor = null;
  await syncEngineToolState();
  syncZoomControl();
  renderSecondaryToolbar();
  renderDocument();
  renderDocumentTabs();
  syncWindowTitle();
}

function documentTitleFromState() {
  const fileName = state.currentFileName || fileNameFromPath(state.currentPath) || fileNameFromPath(state.currentFilePath);
  if (fileName) {
    return fileName;
  }
  const title = String(state.currentDocument?.document?.title || "").trim();
  return title || "Untitled";
}

function fileNameFromPath(path) {
  return String(path || "").split(/[\\/]/).filter(Boolean).pop() || "";
}

function escapeHtml(value) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function updateActiveDocumentTabTitle() {
  const tab = activeDocumentTab();
  if (!tab) {
    return;
  }
  const nextTitle = documentTitleFromState();
  if (tab.title !== nextTitle) {
    tab.title = nextTitle;
    renderDocumentTabs();
  }
}

function renderDocumentTabs() {
  if (!documentTabsRoot) {
    return;
  }
  documentTabsRoot.innerHTML = documentTabs.map((tab) => {
    const active = tab.id === activeDocumentTabId;
    const title = escapeHtml(tab.title || "Untitled");
    const dragging = tab.id === detachingDocumentTabId;
    return `
      <div class="document-tab${active ? " is-active" : ""}${dragging ? " is-dragging" : ""}" role="tab" tabindex="0" aria-selected="${active ? "true" : "false"}" data-document-tab-id="${tab.id}" title="${title}">
        <span class="document-tab-title">${title}</span>
        <button class="document-tab-close" type="button" data-document-tab-close="${tab.id}" aria-label="Close ${title}" title="Close">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10"/><path d="M17 7 7 17"/></svg>
        </button>
      </div>
    `;
  }).join("");
}

function documentTabElement(tabId) {
  if (!documentTabsRoot || !tabId) {
    return null;
  }
  return Array.from(documentTabsRoot.querySelectorAll("[data-document-tab-id]"))
    .find((element) => element.dataset.documentTabId === tabId) || null;
}

function setDetachingDocumentTabId(tabId) {
  if (detachingDocumentTabId === tabId) {
    return;
  }
  if (detachingDocumentTabId) {
    documentTabElement(detachingDocumentTabId)?.classList.remove("is-dragging");
  }
  detachingDocumentTabId = tabId;
  if (detachingDocumentTabId) {
    documentTabElement(detachingDocumentTabId)?.classList.add("is-dragging");
  }
}

async function activateDocumentTab(tabId) {
  if (tabId === activeDocumentTabId) {
    return;
  }
  const nextTab = documentTabs.find((tab) => tab.id === tabId);
  if (!nextTab) {
    return;
  }
  await finishActiveTextEditor(true);
  saveActiveDocumentTabState();
  activeDocumentTabId = nextTab.id;
  await restoreDocumentTabState(nextTab);
}

async function closeDocumentTab(tabId) {
  const index = documentTabs.findIndex((tab) => tab.id === tabId);
  if (index < 0) {
    return;
  }
  const closing = documentTabs[index];
  const wasActive = closing.id === activeDocumentTabId;
  if (wasActive) {
    await finishActiveTextEditor(true);
    saveActiveDocumentTabState();
  }
  await closing.editorEngine?.free?.();
  await closing.documentEngine?.free?.();
  documentTabs.splice(index, 1);
  if (!documentTabs.length) {
    const tab = createDocumentTab();
    documentTabs.push(tab);
    activeDocumentTabId = tab.id;
    await restoreDocumentTabState(tab);
    await resetEditorEngine();
    renderDocument();
    fitView();
    saveActiveDocumentTabState();
    renderDocumentTabs();
    return;
  }
  if (wasActive) {
    const nextTab = documentTabs[Math.max(0, Math.min(index, documentTabs.length - 1))];
    activeDocumentTabId = nextTab.id;
    await restoreDocumentTabState(nextTab);
  } else {
    renderDocumentTabs();
  }
}

documentTabsRoot?.addEventListener("click", async (event) => {
  if (suppressNextDocumentTabClick) {
    suppressNextDocumentTabClick = false;
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  const close = event.target.closest("[data-document-tab-close]");
  if (close) {
    event.stopPropagation();
    await closeDocumentTab(close.dataset.documentTabClose);
    return;
  }
  const tab = event.target.closest("[data-document-tab-id]");
  if (tab) {
    await activateDocumentTab(tab.dataset.documentTabId);
  }
});

documentTabsRoot?.addEventListener("keydown", async (event) => {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }
  const tab = event.target.closest("[data-document-tab-id]");
  if (!tab) {
    return;
  }
  event.preventDefault();
  await activateDocumentTab(tab.dataset.documentTabId);
});

documentTabsRoot?.addEventListener("pointerdown", (event) => {
  if (!isDesktopShell || event.button !== 0 || event.target.closest("[data-document-tab-close]")) {
    return;
  }
  const tab = event.target.closest("[data-document-tab-id]");
  if (!tab) {
    return;
  }
  activeTitlebarTabDrag = {
    tabId: tab.dataset.documentTabId,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    screenX: event.screenX,
    screenY: event.screenY,
    dragging: false,
  };
  tab.setPointerCapture?.(event.pointerId);
});

documentTabsRoot?.addEventListener("pointermove", (event) => {
  const drag = activeTitlebarTabDrag;
  if (!drag || drag.pointerId !== event.pointerId) {
    return;
  }
  drag.screenX = event.screenX;
  drag.screenY = event.screenY;
  const dx = event.clientX - drag.startX;
  const dy = event.clientY - drag.startY;
  if (!drag.dragging && Math.hypot(dx, dy) >= 8) {
    drag.dragging = true;
  }
  const titlebarBottom = desktopTitlebar?.getBoundingClientRect().bottom || 42;
  const shouldDetach = drag.dragging && event.clientY > titlebarBottom + 18;
  setDetachingDocumentTabId(shouldDetach ? drag.tabId : null);
});

documentTabsRoot?.addEventListener("pointerup", async (event) => {
  const drag = activeTitlebarTabDrag;
  if (!drag || drag.pointerId !== event.pointerId) {
    return;
  }
  activeTitlebarTabDrag = null;
  const shouldDetach = detachingDocumentTabId === drag.tabId;
  setDetachingDocumentTabId(null);
  if (shouldDetach) {
    suppressNextDocumentTabClick = true;
    event.preventDefault();
    event.stopPropagation();
    await detachDocumentTab(drag.tabId, drag.screenX, drag.screenY);
  }
});

documentTabsRoot?.addEventListener("pointercancel", () => {
  activeTitlebarTabDrag = null;
  setDetachingDocumentTabId(null);
});

function bindDesktopWindowChrome() {
  if (!isDesktopShell || !desktopFileHost?.available) {
    return;
  }
  desktopTitlebar?.querySelectorAll("[data-window-command]").forEach((button) => {
    button.addEventListener("click", async () => {
      const command = button.dataset.windowCommand;
      if (command === "minimize") {
        await desktopFileHost.minimizeWindow?.();
      } else if (command === "maximize") {
        await desktopFileHost.toggleMaximizeWindow?.();
        await syncDesktopMaximizedState();
      } else if (command === "close") {
        await desktopFileHost.closeWindow?.();
      }
    });
  });
  desktopTitlebar?.querySelectorAll("[data-titlebar-drag-region]").forEach((region) => {
    region.addEventListener("dblclick", async (event) => {
      event.preventDefault();
      await desktopFileHost.toggleMaximizeWindow?.();
      await syncDesktopMaximizedState();
    });
    region.addEventListener("pointerdown", async (event) => {
      if (event.button !== 0 || event.detail > 1) {
        return;
      }
      await desktopFileHost.startWindowDrag?.();
    });
  });
  window.addEventListener("resize", () => {
    syncDesktopMaximizedState();
  }, { passive: true });
  syncDesktopMaximizedState();
}

async function syncDesktopMaximizedState() {
  if (!isDesktopShell || !desktopFileHost?.isWindowMaximized) {
    return;
  }
  const maximized = await desktopFileHost.isWindowMaximized().catch(() => false);
  document.body.classList.toggle("is-window-maximized", !!maximized);
}

async function loadSharedGlyphProfiles() {
  const candidates = [
    new URL("./shared/glyph_profiles.json", import.meta.url),
    new URL("./glyph_profiles.json", import.meta.url),
    new URL("../shared/glyph_profiles.json", import.meta.url),
    new URL("/shared/glyph_profiles.json", window.location.href),
  ];
  let lastStatus = "not attempted";
  for (const url of candidates) {
    const response = await fetch(url);
    if (response.ok) {
      sharedGlyphProfiles = normalizeSharedGlyphProfiles(await response.json());
      return sharedGlyphProfiles;
    }
    lastStatus = `${response.status} ${url.href}`;
  }
  throw new Error(`Failed to load shared glyph profiles: ${lastStatus}`);
}

function isEditingRustDocument() {
  return !LABEL_DEBUG_MODE && !state.currentPath && state.editorEngine;
}

async function syncEngineToolState() {
  if (!state.editorEngine) {
    return;
  }
  await state.editorEngine.ready?.();
  await state.editorEngine.setTool(editorState.activeTool, editorState.bondType);
  await state.editorEngine.setTemplate?.(editorState.template);
  await state.editorEngine.setShapeOptions?.(
    editorState.shapeKind,
    editorState.shapeStyle,
    editorState.shapeColor,
  );
  await state.editorEngine.setBracketOptions?.(editorState.bracketKind);
  await state.editorEngine.setSymbolOptions?.(editorState.symbolKind);
  if (state.editorEngine.setArrowEndpointOptions) {
    await state.editorEngine.setArrowEndpointOptions(
      editorState.arrowType,
      editorState.arrowHeadSize,
      editorState.arrowCurve,
      editorState.arrowHeadStyle,
      editorState.arrowTailStyle,
      editorState.arrowNoGo,
      editorState.arrowBold,
    );
    return;
  }
  await state.editorEngine.setArrowOptions?.(
    editorState.arrowType,
    editorState.arrowHeadSize,
    editorState.arrowHead,
    editorState.arrowTail,
    editorState.arrowBold,
  );
}

function mapRunsFontSize(runs, convert) {
  return Array.isArray(runs)
    ? runs.map((run) => ({
      ...run,
      fontSize: run.fontSize == null ? run.fontSize : convert(Number(run.fontSize)),
    }))
    : runs;
}

function mapTextSessionLengths(session, convert) {
  if (!session || typeof session !== "object") {
    return session;
  }
  const rawBox = session.box ?? session.boxValue;
  return {
    ...session,
    fontSize: session.fontSize == null ? session.fontSize : convert(Number(session.fontSize)),
    lineHeight: session.lineHeight == null ? session.lineHeight : convert(Number(session.lineHeight)),
    box: mapLengthArray(rawBox, convert),
    boxValue: mapLengthArray(session.boxValue, convert),
    anchorOffset: mapLengthArray(session.anchorOffset, convert),
    sourceRuns: mapRunsFontSize(session.sourceRuns, convert),
  };
}

function engineSessionToEditorSession(session) {
  return mapTextSessionLengths(session, cmToCssPx);
}

function editorSessionToEngineSession(session) {
  return mapTextSessionLengths(session, cssPxToCm);
}

function mapTextEditLayoutLengths(layout, convert) {
  if (!layout || typeof layout !== "object") {
    return layout;
  }
  return {
    ...layout,
    lineHeight: layout.lineHeight == null ? layout.lineHeight : convert(Number(layout.lineHeight)),
    width: layout.width == null ? layout.width : convert(Number(layout.width)),
    height: layout.height == null ? layout.height : convert(Number(layout.height)),
    anchorOffset: Array.isArray(layout.anchorOffset)
      ? {
        x: convert(Number(layout.anchorOffset[0] || 0)),
        y: convert(Number(layout.anchorOffset[1] || 0)),
      }
      : layout.anchorOffset,
    sourceRuns: mapRunsFontSize(layout.sourceRuns, convert),
    displayRuns: mapRunsFontSize(layout.displayRuns, convert),
    lines: Array.isArray(layout.lines)
      ? layout.lines.map((line) => ({
        ...line,
        x: line.x == null ? line.x : convert(Number(line.x)),
        y: line.y == null ? line.y : convert(Number(line.y)),
        baselineY: line.baselineY == null ? line.baselineY : convert(Number(line.baselineY)),
        height: line.height == null ? line.height : convert(Number(line.height)),
        runs: mapRunsFontSize(line.runs, convert),
        caretOffsets: Array.isArray(line.caretOffsets)
          ? line.caretOffsets.map((caret) => ({
            ...caret,
            x: caret.x == null ? caret.x : convert(Number(caret.x)),
          }))
          : line.caretOffsets,
      }))
      : layout.lines,
    caretPositions: Array.isArray(layout.caretPositions)
      ? layout.caretPositions.map((caret) => ({
        ...caret,
        x: caret.x == null ? caret.x : convert(Number(caret.x)),
        y: caret.y == null ? caret.y : convert(Number(caret.y)),
        height: caret.height == null ? caret.height : convert(Number(caret.height)),
      }))
      : layout.caretPositions,
    selectionRects: Array.isArray(layout.selectionRects)
      ? layout.selectionRects.map((rect) => ({
        ...rect,
        x: rect.x == null ? rect.x : convert(Number(rect.x)),
        y: rect.y == null ? rect.y : convert(Number(rect.y)),
        width: rect.width == null ? rect.width : convert(Number(rect.width)),
        height: rect.height == null ? rect.height : convert(Number(rect.height)),
      }))
      : layout.selectionRects,
  };
}

function previewTextEditLayoutFromKernel(session, selectionOffsets = null) {
  if (!state.editorEngine?.previewTextEditLayout) {
    return null;
  }
  const preview = parseEngineJson(
    state.editorEngine.previewTextEditLayout(JSON.stringify({
      session: editorSessionToEngineSession(session),
      selection: selectionOffsets
        ? {
          anchor: Number(selectionOffsets.anchor ?? 0),
          focus: Number(selectionOffsets.focus ?? selectionOffsets.anchor ?? 0),
        }
        : null,
    })),
    null,
  );
  return preview ? mapTextEditLayoutLengths(preview, cmToCssPx) : null;
}

function editorCssFontFamily(fontFamily) {
  return displayLabelFontFamily(fontFamily || "Arial");
}

function editorRootFontFamily(root) {
  return String(root?.dataset?.fontFamilyRaw || editorState.textFontFamily || "Arial").trim() || "Arial";
}

function applyEditorRootFontFamily(root, fontFamily) {
  if (!root) {
    return;
  }
  const rawFontFamily = String(fontFamily || editorState.textFontFamily || "Arial").trim() || "Arial";
  root.dataset.fontFamilyRaw = rawFontFamily;
  root.style.fontFamily = editorCssFontFamily(rawFontFamily);
}

function cloneViewBox(viewBox) {
  return {
    x: viewBox.x,
    y: viewBox.y,
    width: viewBox.width,
    height: viewBox.height,
  };
}

function pageViewBox(page) {
  return { x: 0, y: 0, width: page.width, height: page.height };
}

function visibleWorldSize(scale = viewportScale()) {
  if (!viewerContainer || scale <= 0) {
    return {
      width: DEFAULT_WORKSPACE_WIDTH,
      height: DEFAULT_WORKSPACE_HEIGHT,
    };
  }
  return {
    width: Math.max(1, viewerContainer.clientWidth / scale),
    height: Math.max(1, viewerContainer.clientHeight / scale),
  };
}

function viewportScaleForZoom(percent) {
  return CSS_PX_PER_CM * (closestZoomStep(percent) / 100);
}

function visibleWorldRect(scale = viewportScale()) {
  const viewBox = activeViewBox();
  if (!viewerContainer || scale <= 0) {
    return {
      minX: viewBox.x,
      minY: viewBox.y,
      maxX: viewBox.x + viewBox.width,
      maxY: viewBox.y + viewBox.height,
    };
  }
  const minX = viewBox.x + viewerContainer.scrollLeft / scale;
  const minY = viewBox.y + viewerContainer.scrollTop / scale;
  return {
    minX,
    minY,
    maxX: minX + viewerContainer.clientWidth / scale,
    maxY: minY + viewerContainer.clientHeight / scale,
  };
}

function visibleWorldRectForCenter(center, scale) {
  const visible = visibleWorldSize(scale);
  return {
    minX: center.x - visible.width / 2,
    minY: center.y - visible.height / 2,
    maxX: center.x + visible.width / 2,
    maxY: center.y + visible.height / 2,
  };
}

function editorViewportMetrics(scale = viewportScale()) {
  const visible = visibleWorldSize(scale);
  const bufferX = visible.width * EDITOR_VIEW_BUFFER_RATIO;
  const bufferY = visible.height * EDITOR_VIEW_BUFFER_RATIO;
  return {
    visibleWidth: visible.width,
    visibleHeight: visible.height,
    bufferX,
    bufferY,
    triggerX: visible.width * EDITOR_AUTO_EXPAND_TRIGGER_RATIO,
    triggerY: visible.height * EDITOR_AUTO_EXPAND_TRIGGER_RATIO,
    fitPaddingX: visible.width * EDITOR_FIT_PADDING_RATIO,
    fitPaddingY: visible.height * EDITOR_FIT_PADDING_RATIO,
    minCanvasWidth: visible.width + bufferX * 2,
    minCanvasHeight: visible.height + bufferY * 2,
  };
}

function defaultEditorViewBox() {
  const metrics = editorViewportMetrics();
  return {
    x: -metrics.minCanvasWidth / 2,
    y: -metrics.minCanvasHeight / 2,
    width: metrics.minCanvasWidth,
    height: metrics.minCanvasHeight,
  };
}

function activeViewBox() {
  if (state.runtimeViewBox) {
    return cloneViewBox(state.runtimeViewBox);
  }
  const page = state.currentDocument?.document?.page;
  return page ? pageViewBox(page) : defaultEditorViewBox();
}

function viewportScale() {
  return CSS_PX_PER_CM * zoomScale();
}

function zoomScale() {
  return zoomPercent / 100;
}

function refreshDisplayMetrics() {
  const next = displayMetrics();
  const previous = state.displayMetrics;
  state.displayMetrics = next;
  if (
    previous
    && Math.abs(previous.devicePixelRatio - next.devicePixelRatio) > 0.001
    && viewerSvg
  ) {
    applyViewerViewport();
  }
  return next;
}

let displayResolutionQuery = null;

function watchDisplayMetrics() {
  if (typeof window === "undefined") {
    return;
  }
  const refresh = () => {
    refreshDisplayMetrics();
    updateDocumentMeta();
  };
  window.addEventListener("resize", refresh, { passive: true });
  window.visualViewport?.addEventListener?.("resize", refresh, { passive: true });

  const bindResolutionQuery = () => {
    displayResolutionQuery?.removeEventListener?.("change", handleResolutionChange);
    displayResolutionQuery = window.matchMedia?.(`(resolution: ${window.devicePixelRatio || 1}dppx)`) || null;
    displayResolutionQuery?.addEventListener?.("change", handleResolutionChange);
  };
  const handleResolutionChange = () => {
    refresh();
    bindResolutionQuery();
  };
  bindResolutionQuery();
}

function currentViewportCenterWorld() {
  const viewBox = activeViewBox();
  const scale = viewportScale();
  if (!viewerContainer || scale <= 0) {
    return {
      x: viewBox.x + viewBox.width / 2,
      y: viewBox.y + viewBox.height / 2,
    };
  }
  return {
    x: viewBox.x + (viewerContainer.scrollLeft + viewerContainer.clientWidth / 2) / scale,
    y: viewBox.y + (viewerContainer.scrollTop + viewerContainer.clientHeight / 2) / scale,
  };
}

function currentEditableFragmentData() {
  const documentData = state.currentDocument;
  if (!documentData?.objects || !documentData?.resources) {
    return null;
  }
  const object = documentData.objects.find((candidate) => candidate.type === "molecule" || candidate.object_type === "molecule");
  const resourceRef = object?.payload?.resourceRef || object?.payload?.resource_ref;
  const fragment = resourceRef ? documentData.resources?.[resourceRef]?.data : null;
  if (!object || !fragment?.nodes || !fragment?.bonds) {
    return null;
  }
  return { object, fragment };
}

function worldPointForFragmentPosition(object, position) {
  if (!Array.isArray(position) || position.length < 2) {
    return null;
  }
  const translate = object?.transform?.translate || [0, 0];
  return {
    x: Number(translate[0] || 0) + Number(position[0] || 0),
    y: Number(translate[1] || 0) + Number(position[1] || 0),
  };
}

function worldPointForFragmentNode(object, node) {
  return worldPointForFragmentPosition(object, node?.position);
}

function worldToScreenPoint(point) {
  if (!point) {
    return null;
  }
  const viewBox = activeViewBox();
  const scale = viewportScale();
  return {
    x: (point.x - viewBox.x) * scale - (viewerContainer?.scrollLeft || 0),
    y: (point.y - viewBox.y) * scale - (viewerContainer?.scrollTop || 0),
  };
}

function worldToLayerPoint(point) {
  if (!point) {
    return null;
  }
  const viewBox = activeViewBox();
  const scale = viewportScale();
  return {
    x: (point.x - viewBox.x) * scale,
    y: (point.y - viewBox.y) * scale,
  };
}

function selectionZoomCenterWorld() {
  const engineState = currentEditorEngineState();
  const selection = engineState?.selection;
  if (!selection || (!selection.nodes?.length && !selection.bonds?.length)) {
    return null;
  }
  const entry = currentEditableFragmentData();
  if (!entry) {
    return null;
  }
  const nodeById = new Map(entry.fragment.nodes.map((node) => [node.id, node]));
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  let hasPoint = false;

  function includePoint(point) {
    if (!point) {
      return;
    }
    minX = Math.min(minX, point.x);
    minY = Math.min(minY, point.y);
    maxX = Math.max(maxX, point.x);
    maxY = Math.max(maxY, point.y);
    hasPoint = true;
  }

  function worldPointForNode(node) {
    return worldPointForFragmentNode(entry.object, node);
  }

  for (const nodeId of selection.nodes || []) {
    includePoint(worldPointForNode(nodeById.get(nodeId)));
  }

  for (const bondId of selection.bonds || []) {
    const bond = entry.fragment.bonds.find((candidate) => candidate.id === bondId);
    if (!bond) {
      continue;
    }
    includePoint(worldPointForNode(nodeById.get(bond.begin)));
    includePoint(worldPointForNode(nodeById.get(bond.end)));
  }

  if (!hasPoint) {
    return null;
  }
  return {
    x: (minX + maxX) / 2,
    y: (minY + maxY) / 2,
  };
}

function documentContentBoundsForZoom() {
  return currentRenderBounds("document");
}

function zoomFocusBounds() {
  const selectionBounds = isEditingRustDocument() ? currentRenderBounds("selection") : null;
  const bounds = selectionBounds || documentContentBoundsForZoom();
  if (!bounds) {
    return null;
  }
  return {
    bounds,
    center: boundsCenter(bounds),
    kind: selectionBounds ? "selection" : "content",
    key: `${selectionBounds ? "selection" : "content"}:${boundsToKey(bounds)}`,
  };
}

function clearZoomHandoffs() {
  state.zoomHandoffs = [];
  state.expectedProgrammaticScroll = null;
}

function markProgrammaticScroll() {
  state.isProgrammaticScroll = true;
  window.clearTimeout(state.programmaticScrollTimer);
  state.programmaticScrollTimer = window.setTimeout(() => {
    state.isProgrammaticScroll = false;
  }, 250);
}

function rememberProgrammaticScrollPosition() {
  if (!viewerContainer) {
    return;
  }
  state.expectedProgrammaticScroll = {
    left: viewerContainer.scrollLeft,
    top: viewerContainer.scrollTop,
  };
}

function isExpectedProgrammaticScroll() {
  if (!viewerContainer || !state.expectedProgrammaticScroll) {
    return false;
  }
  return Math.abs(viewerContainer.scrollLeft - state.expectedProgrammaticScroll.left) <= 1
    && Math.abs(viewerContainer.scrollTop - state.expectedProgrammaticScroll.top) <= 1;
}

function constrainZoomCenterForBounds(center, bounds, scale) {
  if (!bounds || !viewerContainer || scale <= 0) {
    return center;
  }
  const visible = visibleWorldSize(scale);
  const next = { ...center };
  const size = boundsSize(bounds);
  if (size.width <= visible.width) {
    const minCenterX = bounds.maxX - visible.width / 2;
    const maxCenterX = bounds.minX + visible.width / 2;
    next.x = Math.min(Math.max(next.x, minCenterX), maxCenterX);
  }
  if (size.height <= visible.height) {
    const minCenterY = bounds.maxY - visible.height / 2;
    const maxCenterY = bounds.minY + visible.height / 2;
    next.y = Math.min(Math.max(next.y, minCenterY), maxCenterY);
  }
  return next;
}

function clampZoomPercent(value) {
  return Math.max(ZOOM_MIN_PERCENT, Math.min(ZOOM_MAX_PERCENT, Math.round(value)));
}

function closestZoomStep(value) {
  const clamped = clampZoomPercent(value);
  return ZOOM_STEP_LEVELS.reduce((best, candidate) => (
    Math.abs(candidate - clamped) < Math.abs(best - clamped) ? candidate : best
  ), ZOOM_STEP_LEVELS[0]);
}

function zoomStepAtOrBelow(value) {
  const clamped = clampZoomPercent(value);
  let best = ZOOM_STEP_LEVELS[0];
  for (const level of ZOOM_STEP_LEVELS) {
    if (level <= clamped + 0.5) {
      best = level;
    }
  }
  return best;
}

function syncZoomControl() {
  if (zoomInput) {
    zoomInput.value = String(zoomPercent);
  }
}

function nextZoomStep(direction) {
  if (direction > 0) {
    return ZOOM_STEP_LEVELS.find((level) => level > zoomPercent + 0.5) || ZOOM_MAX_PERCENT;
  }
  for (let index = ZOOM_STEP_LEVELS.length - 1; index >= 0; index -= 1) {
    if (ZOOM_STEP_LEVELS[index] < zoomPercent - 0.5) {
      return ZOOM_STEP_LEVELS[index];
    }
  }
  return ZOOM_MIN_PERCENT;
}

function scrollViewerToWorldPoint(point, center = true) {
  if (!viewerContainer) {
    return;
  }
  const viewBox = activeViewBox();
  const scale = viewportScale();
  const offsetX = center ? viewerContainer.clientWidth / 2 : 0;
  const offsetY = center ? viewerContainer.clientHeight / 2 : 0;
  markProgrammaticScroll();
  viewerContainer.scrollLeft = Math.max(0, (point.x - viewBox.x) * scale - offsetX);
  viewerContainer.scrollTop = Math.max(0, (point.y - viewBox.y) * scale - offsetY);
  rememberProgrammaticScrollPosition();
}

function scrollViewerToWorldPointAtClient(point, clientX, clientY) {
  if (!viewerContainer || !point) {
    return;
  }
  const rect = viewerContainer.getBoundingClientRect();
  const viewBox = activeViewBox();
  const scale = viewportScale();
  markProgrammaticScroll();
  viewerContainer.scrollLeft = Math.max(0, (point.x - viewBox.x) * scale - (clientX - rect.left));
  viewerContainer.scrollTop = Math.max(0, (point.y - viewBox.y) * scale - (clientY - rect.top));
  rememberProgrammaticScrollPosition();
}

function applyViewerViewport(options = {}) {
  if (!viewerSvg) {
    return;
  }
  const viewBox = activeViewBox();
  const pixelWidth = `${Math.max(1, viewBox.width * viewportScale())}px`;
  const pixelHeight = `${Math.max(1, viewBox.height * viewportScale())}px`;
  viewerSvg.setAttribute("viewBox", `${viewBox.x} ${viewBox.y} ${viewBox.width} ${viewBox.height}`);
  viewerSvg.style.width = pixelWidth;
  viewerSvg.style.height = pixelHeight;
  viewerSvg.style.setProperty("--chemcore-css-px-per-cm", String(state.displayMetrics.cssPxPerCm));
  viewerSvg.style.setProperty("--chemcore-device-pixel-ratio", String(state.displayMetrics.devicePixelRatio));
  viewerSvg.style.setProperty("--chemcore-device-dpi", String(state.displayMetrics.devicePxPerInch));
  if (textEditorLayer) {
    textEditorLayer.style.width = pixelWidth;
    textEditorLayer.style.height = pixelHeight;
  }

  const scrollDelta = options.scrollDelta;
  const centerWorld = options.centerWorld;
  if (!viewerContainer || (!scrollDelta && !centerWorld)) {
    if (activeTextEditor?.root) {
      renderActiveTextEditorFromModel(currentEditorSelectionOffsets());
    }
    positionActiveTextEditor();
    return;
  }
  requestAnimationFrame(() => {
    if (activeTextEditor?.root) {
      renderActiveTextEditorFromModel(currentEditorSelectionOffsets());
    }
    if (centerWorld) {
      scrollViewerToWorldPoint(centerWorld, true);
      positionActiveTextEditor();
      return;
    }
    if (scrollDelta) {
      markProgrammaticScroll();
      viewerContainer.scrollLeft += scrollDelta.x * viewportScale();
      viewerContainer.scrollTop += scrollDelta.y * viewportScale();
      rememberProgrammaticScrollPosition();
    }
    positionActiveTextEditor();
  });
}

function setRuntimeViewBox(viewBox, options = {}) {
  state.runtimeViewBox = {
    x: viewBox.x,
    y: viewBox.y,
    width: Math.max(1, viewBox.width),
    height: Math.max(1, viewBox.height),
  };
  applyViewerViewport(options);
}

function fitZoomPercentForViewBox(viewBox) {
  if (!viewerContainer) {
    return 100;
  }
  const width = Math.max(1, viewerContainer.clientWidth);
  const height = Math.max(1, viewerContainer.clientHeight);
  const scale = Math.min(width / Math.max(1, viewBox.width), height / Math.max(1, viewBox.height));
  return zoomStepAtOrBelow((scale / CSS_PX_PER_CM) * 100);
}

function editorCanvasViewBoxFromBounds(bounds) {
  const metrics = editorViewportMetrics();
  return paddedViewBoxFromBounds(
    bounds,
    metrics.bufferX,
    metrics.bufferY,
    metrics.minCanvasWidth,
    metrics.minCanvasHeight,
  );
}

function currentEditorRenderList() {
  return renderListFromEngine(state.editorEngine);
}

function currentRenderBounds(scope = "all") {
  const engine = isEditingRustDocument() ? state.editorEngine : state.documentEngine;
  return renderBoundsFromEngine(engine, scope);
}

function ensureEditorViewportCapacity(centerWorld = currentViewportCenterWorld()) {
  if (!isEditingRustDocument()) {
    return false;
  }
  const current = activeViewBox();
  const metrics = editorViewportMetrics();
  if (current.width >= metrics.minCanvasWidth && current.height >= metrics.minCanvasHeight) {
    return false;
  }
  const next = cloneViewBox(current);
  if (next.width < metrics.minCanvasWidth) {
    next.x = centerWorld.x - metrics.minCanvasWidth / 2;
    next.width = metrics.minCanvasWidth;
  }
  if (next.height < metrics.minCanvasHeight) {
    next.y = centerWorld.y - metrics.minCanvasHeight / 2;
    next.height = metrics.minCanvasHeight;
  }
  setRuntimeViewBox(next, { centerWorld });
  return true;
}

function maybeAutoExpandEditorViewport(_primitives) {
  if (!isEditingRustDocument()) {
    return false;
  }
  const bounds = currentRenderBounds("document");
  if (!bounds) {
    return false;
  }
  const current = activeViewBox();
  const metrics = editorViewportMetrics();
  const next = cloneViewBox(current);
  let shiftLeft = 0;
  let shiftTop = 0;
  let changed = false;

  if (bounds.minX < current.x + metrics.triggerX) {
    const targetX = bounds.minX - metrics.bufferX;
    shiftLeft = current.x - targetX;
    next.x = targetX;
    next.width += shiftLeft;
    changed = true;
  }
  if (bounds.minY < current.y + metrics.triggerY) {
    const targetY = bounds.minY - metrics.bufferY;
    shiftTop = current.y - targetY;
    next.y = targetY;
    next.height += shiftTop;
    changed = true;
  }
  if (bounds.maxX > current.x + current.width - metrics.triggerX) {
    next.width = Math.max(next.width, bounds.maxX + metrics.bufferX - next.x);
    changed = true;
  }
  if (bounds.maxY > current.y + current.height - metrics.triggerY) {
    next.height = Math.max(next.height, bounds.maxY + metrics.bufferY - next.y);
    changed = true;
  }

  next.width = Math.max(next.width, metrics.minCanvasWidth);
  next.height = Math.max(next.height, metrics.minCanvasHeight);

  if (!changed) {
    return false;
  }

  setRuntimeViewBox(next, {
    scrollDelta: {
      x: shiftLeft,
      y: shiftTop,
    },
  });
  return true;
}

async function syncCoreRenderListFromCurrentDocument() {
  state.coreRenderList = null;
  if (!state.currentDocument) {
    return;
  }
  if (state.currentPath) {
    if (!state.documentEngine) {
      await resetDocumentEngine();
    }
    await state.documentEngine.ready?.();
    await state.documentEngine.loadDocumentJson(JSON.stringify(state.currentDocument));
    state.coreRenderList = renderListFromEngine(state.documentEngine);
    return;
  }
  if (state.editorEngine) {
    state.coreRenderList = renderListFromEngine(state.editorEngine);
  }
}

function syncEditorRenderListFromEngine(options = {}) {
  if (!state.editorEngine) {
    return [];
  }
  const autoExpand = options.autoExpand ?? true;
  state.coreRenderList = renderListFromEngine(state.editorEngine);
  if (autoExpand) {
    maybeAutoExpandEditorViewport(state.coreRenderList || []);
  }
  return state.coreRenderList || [];
}

function syncEditorSelectionRenderListFromEngine() {
  return syncEditorRenderListFromEngine({ autoExpand: false });
}

function currentEditorOverlayRenderList() {
  const renderList = state.coreRenderList || currentEditorRenderList();
  return (renderList || []).filter((primitive) => !isDocumentPreviewPrimitive(primitive));
}

function corePrimitivesForObject(objectId) {
  return primitivesForObject(state.coreRenderList, objectId);
}

function activeEndpointEditorNodeId() {
  return activeTextEditor?.session?.target?.kind === "endpoint-label"
    ? activeTextEditor.session.target.nodeId || activeTextEditor.session.target.node_id
    : null;
}

function shouldHidePrimitiveForActiveEndpointEditor(primitive) {
  const nodeId = activeEndpointEditorNodeId();
  const role = primitive?.role;
  const primitiveNodeId = primitive?.nodeId || primitive?.node_id;
  if (nodeId && role === "selection-text-box") {
    return true;
  }
  if (nodeId && role === "hover-endpoint") {
    return primitiveNodeId === nodeId;
  }
  if (!nodeId || primitiveNodeId !== nodeId) {
    return false;
  }
  return role === "document-text"
    || role === "document-knockout"
    || role === "document-graphic"
    || role === "hover-label-glyph"
    || role === "hover-text-box";
}

function corePrimitiveRenderOptions() {
  return {
    labelDebugMode: LABEL_DEBUG_MODE,
    sharedGlyphProfiles,
    shouldHide: shouldHidePrimitiveForActiveEndpointEditor,
  };
}

async function syncDocumentFromEngine() {
  if (!state.editorEngine) {
    return;
  }
  const documentData = parseEngineJson(state.editorEngine.documentJson());
  if (documentData) {
    state.currentDocument = documentData;
    await syncCoreRenderListFromCurrentDocument();
    maybeAutoExpandEditorViewport(state.coreRenderList || []);
  }
  refreshCommandAvailability();
}

async function renderSelectionOnlyUpdate(point, syncCursor = syncSelectCursorForPoint) {
  if (point) {
    await syncCursor(point);
  }
  renderEditorOverlay(syncEditorSelectionRenderListFromEngine());
  refreshCommandAvailability();
}

async function selectClickTarget(point, additive = false) {
  await state.editorEngine.selectAtPoint(point.x, point.y, additive);
}

function currentEditorEngineState() {
  if (!state.editorEngine) {
    return null;
  }
  return parseEngineJson(state.editorEngine.stateJson());
}

async function resetEditorEngine() {
  await finishActiveTextEditor(false);
  await state.editorEngine?.free?.();
  state.editorEngine = engineHost.createEngineSession();
  await state.editorEngine.ready?.();
  state.runtimeViewBox = defaultEditorViewBox();
  state.lastEditFocusPoint = null;
  clearZoomHandoffs();
  state.currentFileName = null;
  await syncEngineToolState();
  await syncDocumentFromEngine();
}

async function resetDocumentEngine() {
  await state.documentEngine?.free?.();
  state.documentEngine = engineHost.createEngineSession();
  await state.documentEngine.ready?.();
}

function refreshCommandAvailability() {
  const undoButton = document.querySelector('[data-command="undo"]');
  const redoButton = document.querySelector('[data-command="redo"]');
  if (undoButton) {
    undoButton.disabled = !state.editorEngine?.canUndo?.();
  }
  if (redoButton) {
    redoButton.disabled = !state.editorEngine?.canRedo?.();
  }
  void updateCanvasContextMenuAvailability();
}

function editorSelectionHasItems(selection) {
  if (!selection) {
    return false;
  }
  return Boolean(selection.region)
    || ["nodes", "bonds", "labelNodes", "arrowObjects", "textObjects"].some((key) => (
      Array.isArray(selection[key]) && selection[key].length > 0
    ));
}

function currentEditorSelectionHasItems() {
  return editorSelectionHasItems(currentEditorEngineState()?.selection);
}

function sceneObjectHasSelectableContent(object, resources) {
  if (!object || object.visible === false) {
    return false;
  }
  if (object.type === "molecule") {
    const resource = resources?.[object.payload?.resourceRef];
    const fragment = resource?.data;
    return Boolean(fragment?.nodes?.length || fragment?.bonds?.length);
  }
  if (object.type === "group") {
    return true;
  }
  return ["text", "line", "bracket", "symbol", "shape"].includes(object.type);
}

function currentDocumentHasSelectableContent() {
  const documentData = state.currentDocument;
  if (!documentData?.objects?.length) {
    return false;
  }
  return documentData.objects.some((object) => sceneObjectHasSelectableContent(object, documentData.resources));
}

function collectSceneObjects(objects = [], out = new Map()) {
  for (const object of objects || []) {
    out.set(object.id, object);
    if (Array.isArray(object.children)) {
      collectSceneObjects(object.children, out);
    }
  }
  return out;
}

function currentSceneObjectMap() {
  return collectSceneObjects(state.currentDocument?.objects || []);
}

function currentEditableFragment() {
  const documentData = state.currentDocument;
  const molecule = documentData?.objects?.find((object) => object.type === "molecule" && object.payload?.resourceRef);
  return molecule ? documentData.resources?.[molecule.payload.resourceRef]?.data || null : null;
}

function currentSelectionInfo() {
  const selection = currentEditorEngineState()?.selection || {};
  const objectMap = currentSceneObjectMap();
  const textObjects = (selection.textObjects || []).map((id) => objectMap.get(id)).filter(Boolean);
  const graphicObjects = (selection.arrowObjects || []).map((id) => objectMap.get(id)).filter(Boolean);
  const fragment = currentEditableFragment();
  const nodeIds = selection.nodes || [];
  const bondIds = selection.bonds || [];
  const labelNodeIds = selection.labelNodes || [];
  return {
    selection,
    objectMap,
    textObjects,
    graphicObjects,
    sceneObjects: textObjects.concat(graphicObjects),
    fragment,
    nodes: nodeIds.map((id) => fragment?.nodes?.find((node) => node.id === id)).filter(Boolean),
    bonds: bondIds.map((id) => fragment?.bonds?.find((bond) => bond.id === id)).filter(Boolean),
    labelNodes: labelNodeIds.map((id) => fragment?.nodes?.find((node) => node.id === id)).filter(Boolean),
  };
}

function contextSelectionCount(info = currentSelectionInfo()) {
  return info.sceneObjects.length + info.nodes.length + info.bonds.length + info.labelNodes.length;
}

function contextHasSelection(info = currentSelectionInfo()) {
  return contextSelectionCount(info) > 0 || Boolean(info.selection?.region);
}

function uniformValue(values) {
  const normalized = values.filter((value) => value != null && value !== "");
  if (!normalized.length) {
    return null;
  }
  return normalized.every((value) => value === normalized[0]) ? normalized[0] : null;
}

async function currentClipboardHasPasteContent() {
  if (!state.editorEngine) {
    return false;
  }
  try {
    if (await Promise.resolve(state.editorEngine.hasClipboard?.())) {
      return true;
    }
  } catch (error) {
    console.warn("Failed to inspect engine clipboard", error);
  }
  if (!desktopFileHost?.available || !state.editorEngine.pasteClipboardJson) {
    return false;
  }
  try {
    const payload = await desktopFileHost.readClipboard();
    return Boolean(payload?.chemcoreFragmentJson);
  } catch (error) {
    console.warn("Failed to inspect native clipboard", error);
    return false;
  }
}

function createCanvasContextMenu() {
  const menu = document.createElement("div");
  menu.className = "canvas-context-menu";
  menu.hidden = true;
  menu.setAttribute("role", "menu");
  menu.setAttribute("aria-label", "Canvas menu");

  menu.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
  menu.addEventListener("click", (event) => {
    const item = event.target.closest("[data-canvas-context-command]");
    if (!item || item.disabled || item.dataset.hasSubmenu === "true") {
      return;
    }
    const command = item.dataset.canvasContextCommand;
    const value = item.dataset.canvasContextValue || "";
    void runCanvasContextMenuCommand(command, value);
  });

  return menu;
}

async function updateCanvasContextMenuAvailability() {
  if (!canvasContextMenu || canvasContextMenu.hidden) {
    return;
  }
  renderCanvasContextMenu(await buildCanvasContextMenuItems(activeContextMenuState?.hit || { kind: "canvas" }));
}

function hideCanvasContextMenu() {
  if (canvasContextMenu.hidden) {
    return;
  }
  canvasContextMenu.hidden = true;
}

async function finishTemporaryContextSelection() {
  if (!activeContextMenuState?.temporarySelection) {
    activeContextMenuState = null;
    return;
  }
  activeContextMenuState = null;
  if (await state.editorEngine?.clearSelection?.()) {
    await syncDocumentFromEngine();
    renderDocument();
  }
}

function closeCanvasContextMenu() {
  hideCanvasContextMenu();
  void finishTemporaryContextSelection();
}

function canvasContextMenuItem(item, depth = 0) {
  if (item.type === "separator") {
    const separator = document.createElement("div");
    separator.className = "canvas-context-menu-separator";
    separator.setAttribute("role", "separator");
    return separator;
  }

  const entry = document.createElement("div");
  entry.className = "canvas-context-menu-entry";
  if (item.submenu?.length) {
    entry.classList.add("has-submenu");
  }

  const button = document.createElement("button");
  button.type = "button";
  button.className = "canvas-context-menu-item";
  button.dataset.canvasContextCommand = item.command || "";
  button.dataset.canvasContextValue = item.value || "";
  button.dataset.hasSubmenu = item.submenu?.length ? "true" : "false";
  button.disabled = !!item.disabled;
  button.setAttribute("role", "menuitem");
  if (item.checked) {
    button.classList.add("is-checked");
    button.setAttribute("aria-checked", "true");
  }

  const check = document.createElement("span");
  check.className = "canvas-context-menu-check";
  check.textContent = item.checked ? "✓" : "";
  const label = document.createElement("span");
  label.className = "canvas-context-menu-label";
  label.textContent = item.label || "";
  const shortcut = document.createElement("span");
  shortcut.className = "canvas-context-menu-shortcut";
  shortcut.textContent = item.submenu?.length ? "›" : item.shortcut || "";
  button.append(check, label, shortcut);
  entry.appendChild(button);

  if (item.submenu?.length) {
    const submenu = document.createElement("div");
    submenu.className = "canvas-context-submenu";
    submenu.setAttribute("role", "menu");
    if (depth >= 1) {
      submenu.classList.add("is-nested");
    }
    item.submenu.forEach((child) => submenu.appendChild(canvasContextMenuItem(child, depth + 1)));
    entry.appendChild(submenu);
  }
  return entry;
}

function renderCanvasContextMenu(items) {
  canvasContextMenu.innerHTML = "";
  items.forEach((item) => canvasContextMenu.appendChild(canvasContextMenuItem(item)));
}

async function openCanvasContextMenu(event) {
  event.preventDefault();
  event.stopPropagation();
  if (!isEditingRustDocument()) {
    closeCanvasContextMenu();
    return;
  }
  const point = svgPointFromEvent(event);
  let hit = await contextHitTest(point);
  const temporarySelection = editorState.activeTool !== "select" && hit.kind !== "canvas" && !hit.selected;
  if (hit.kind !== "canvas" && !hit.selected) {
    await selectClickTarget(point, false);
    await renderSelectionOnlyUpdate(point);
    hit = await contextHitTest(point);
  }
  activeContextMenuState = {
    hit,
    point,
    temporarySelection,
    actionTaken: false,
  };
  renderCanvasContextMenu(await buildCanvasContextMenuItems(hit));
  canvasContextMenu.hidden = false;
  const margin = 6;
  const width = canvasContextMenu.offsetWidth;
  const height = canvasContextMenu.offsetHeight;
  const left = Math.max(margin, Math.min(event.clientX, window.innerWidth - width - margin));
  const top = Math.max(margin, Math.min(event.clientY, window.innerHeight - height - margin));
  canvasContextMenu.style.left = `${left}px`;
  canvasContextMenu.style.top = `${top}px`;
  canvasContextMenu.querySelector("button:not(:disabled):not([data-has-submenu='true'])")?.focus?.({ preventScroll: true });
}

async function contextHitTest(point) {
  if (!state.editorEngine?.contextHitTestJson) {
    return { kind: "canvas" };
  }
  try {
    return parseEngineJson(await state.editorEngine.contextHitTestJson(point.x, point.y), { kind: "canvas" }) || { kind: "canvas" };
  } catch (error) {
    console.warn("Failed to hit-test context menu target", error);
    return { kind: "canvas" };
  }
}

function selectedSceneObjects() {
  return currentSelectionInfo().sceneObjects;
}

function styleColorForObject(object) {
  const style = state.currentDocument?.styles?.[object?.styleRef];
  return cssColorToHex(
    object?.payload?.fill
    || object?.payload?.stroke
    || style?.fill
    || style?.stroke
    || object?.payload?.color
    || "#000000",
  );
}

function selectedUniformColor() {
  const info = currentSelectionInfo();
  const colors = [];
  for (const object of info.sceneObjects) {
    colors.push(styleColorForObject(object));
  }
  for (const bond of info.bonds) {
    colors.push(cssColorToHex(bond.stroke || "#000000"));
  }
  for (const node of info.labelNodes.concat(info.nodes)) {
    colors.push(cssColorToHex(node.label?.fill || "#000000"));
  }
  return uniformValue(colors);
}

function lineObjectStyle(object) {
  const style = state.currentDocument?.styles?.[object?.styleRef] || {};
  const arrowHead = object?.payload?.arrowHead || {};
  if (arrowHead.bold) {
    return "bold";
  }
  if (Array.isArray(style.dashArray) && style.dashArray.length) {
    return "dashed";
  }
  return "plain";
}

function selectedUniformLineStyle() {
  const lines = selectedSceneObjects().filter((object) => object.type === "line");
  return uniformValue(lines.map(lineObjectStyle));
}

function selectedUniformArrowEndpoint(endpoint) {
  const lines = selectedSceneObjects().filter((object) => object.type === "line");
  return uniformValue(lines.map((object) => object.payload?.arrowHead?.[endpoint] || "none"));
}

async function buildCanvasContextMenuItems(hit) {
  if (!state.editorEngine?.contextMenuJson) {
    return [];
  }
  const hasPaste = await currentClipboardHasPasteContent();
  return parseEngineJson(
    await state.editorEngine.contextMenuJson(JSON.stringify(hit || { kind: "canvas" }), hasPaste),
    [],
  ) || [];
}

async function runCanvasContextMenuCommand(command, value) {
  if (!command || command === "noop") {
    return;
  }
  if (activeContextMenuState) {
    activeContextMenuState.actionTaken = true;
  }
  hideCanvasContextMenu();
  let changed = false;
  if (["cut", "copy", "paste", "delete", "select-all"].includes(command)) {
    changed = await runEditorCommand(command);
  } else if (command === "order") {
    changed = !!(await state.editorEngine?.applySelectionOrderCommand?.(value));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "arrange") {
    changed = !!(await state.editorEngine?.applySelectionArrangeCommand?.(value));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "group") {
    changed = !!(await state.editorEngine?.groupSelection?.());
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "ungroup") {
    changed = !!(await state.editorEngine?.ungroupSelection?.());
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "color") {
    changed = await applySelectionColor(value);
  } else if (command === "color-other") {
    openColorDialog(selectedUniformColor() || editorState.selectionColor || "#000000", async (color) => {
      await applySelectionColor(color);
      await finishTemporaryContextSelection();
    }, { colorHost });
    return;
  } else if (command === "shape-style") {
    changed = !!(await state.editorEngine?.applyShapeStyleToSelection?.(value));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "bracket-kind") {
    changed = !!(await state.editorEngine?.applyBracketKindToSelection?.(value));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "line-style") {
    changed = !!(await state.editorEngine?.applyLineStyleToSelection?.(value));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "bond-style") {
    changed = !!(await state.editorEngine?.applyBondStyleToSelection?.(value));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "text-style") {
    const separatorIndex = value.indexOf(":");
    const styleCommand = separatorIndex >= 0 ? value.slice(0, separatorIndex) : value;
    const styleValue = separatorIndex >= 0 ? value.slice(separatorIndex + 1) : "";
    changed = !!(await state.editorEngine?.applyTextStyleToSelection?.(styleCommand, styleValue));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "text-line-spacing") {
    await numericDialogHost.choose("line-height");
    await finishTemporaryContextSelection();
    return;
  } else if (command === "chemical-check") {
    changed = !!(await state.editorEngine?.setChemicalCheckForSelection?.(value !== "off"));
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "expand-label") {
    changed = !!(await state.editorEngine?.expandLabelsInSelection?.());
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "center-page") {
    changed = !!(await state.editorEngine?.centerSelectionOnPage?.());
    if (changed) {
      await syncDocumentFromEngine();
      renderDocument();
    }
  } else if (command === "object-settings") {
    await objectSettingsHost.chooseObjectSettings();
    await finishTemporaryContextSelection();
    return;
  } else if (command === "scale-dialog") {
    await numericDialogHost.choose("scale");
    await finishTemporaryContextSelection();
    return;
  } else if (command === "rotate-dialog") {
    await numericDialogHost.choose("rotate");
    await finishTemporaryContextSelection();
    return;
  } else if (command === "edit-text") {
    const point = activeContextMenuState?.point;
    if (point) {
      await openTextEditorAt(point);
      changed = true;
    }
  } else if (command === "arrow-bold") {
    syncEditorArrowStateFromSelectedLine();
    editorState.arrowBold = selectedUniformLineStyle() !== "bold";
    changed = await applyArrowOptionsToSelection();
  } else if (command === "arrow-endpoint") {
    syncEditorArrowStateFromSelectedLine();
    const [endpoint, style] = value.split(":");
    const nextStyle = style || "none";
    if (endpoint === "head") {
      editorState.arrowHeadStyle = selectedUniformArrowEndpoint("head") === endpointStylePayloadName(nextStyle) ? "none" : nextStyle;
      editorState.arrowHead = editorState.arrowHeadStyle !== "none";
    } else {
      editorState.arrowTailStyle = selectedUniformArrowEndpoint("tail") === endpointStylePayloadName(nextStyle) ? "none" : nextStyle;
      editorState.arrowTail = editorState.arrowTailStyle !== "none";
    }
    changed = await applyArrowOptionsToSelection();
  }
  if (!changed) {
    renderEditorOverlay();
    refreshCommandAvailability();
  }
  await finishTemporaryContextSelection();
}

function endpointStylePayloadName(style) {
  if (style === "left") {
    return "half-left";
  }
  if (style === "right") {
    return "half-right";
  }
  return style;
}

function syncEditorArrowStateFromSelectedLine() {
  const line = selectedSceneObjects().find((object) => object.type === "line");
  if (!line) {
    return;
  }
  const arrowHead = line.payload?.arrowHead || {};
  const kind = arrowHead.kind || "solid";
  if (["solid", "curved", "curved-mirror", "hollow", "open"].includes(kind)) {
    editorState.arrowType = kind;
  }
  const curve = Math.abs(Number(arrowHead.curve || 0));
  if (curve >= 260) {
    editorState.arrowCurve = "270";
  } else if (curve >= 150) {
    editorState.arrowCurve = "180";
  } else if (curve >= 105) {
    editorState.arrowCurve = "120";
  } else if (curve >= 60) {
    editorState.arrowCurve = "90";
  }
  const head = arrowHead.head || "none";
  const tail = arrowHead.tail || "none";
  editorState.arrowHeadStyle = head === "half-left" ? "left" : head === "half-right" ? "right" : head;
  editorState.arrowTailStyle = tail === "half-left" ? "left" : tail === "half-right" ? "right" : tail;
  editorState.arrowHead = editorState.arrowHeadStyle !== "none";
  editorState.arrowTail = editorState.arrowTailStyle !== "none";
  editorState.arrowBold = !!arrowHead.bold;
  editorState.arrowNoGo = arrowHead.noGo || "none";
}

async function writeNativeClipboardFromSelection(fragmentJson = null, documentJson = undefined) {
  if (!desktopFileHost?.available || !state.editorEngine) {
    return false;
  }
  try {
    const resolvedFragmentJson = fragmentJson || await state.editorEngine.clipboardSelectionJson?.() || null;
    const resolvedDocumentJson = documentJson === undefined
      ? await state.editorEngine.clipboardDocumentJson?.() || null
      : documentJson;
    if (!resolvedFragmentJson && !resolvedDocumentJson) {
      return false;
    }
    const cdxml = await state.editorEngine.documentCdxml?.() || null;
    const svg = null;
    await desktopFileHost.writeClipboard({
      chemcoreFragmentJson: resolvedFragmentJson,
      chemcoreDocumentJson: resolvedDocumentJson,
      renderListJson: state.editorEngine.renderListJson?.() || null,
      cdxml,
      svg,
      text: cdxml,
    });
    return true;
  } catch (error) {
    console.warn("Failed to write native clipboard", error);
  }
  return false;
}

async function pasteFromNativeClipboard() {
  if (!desktopFileHost?.available || !state.editorEngine?.pasteClipboardJson) {
    return false;
  }
  try {
    const payload = await desktopFileHost.readClipboard();
    if (payload?.chemcoreFragmentJson) {
      return !!(await state.editorEngine.pasteClipboardJson(payload.chemcoreFragmentJson));
    }
  } catch (error) {
    console.warn("Failed to read native clipboard", error);
  }
  return false;
}

async function runEditorCommand(command) {
  if (!isEditingRustDocument()) {
    return false;
  }
  let changed = false;
  let shouldRenderDocument = false;
  if (command === "undo") {
    changed = await state.editorEngine.undo();
  } else if (command === "redo") {
    changed = await state.editorEngine.redo();
  } else if (command === "copy") {
    const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
    const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
    changed = !!(await state.editorEngine.copySelection?.());
    changed = await writeNativeClipboardFromSelection(fragmentJson, documentJson) || changed;
  } else if (command === "cut") {
    const fragmentJson = await state.editorEngine.clipboardSelectionJson?.() || null;
    const documentJson = await state.editorEngine.clipboardDocumentJson?.() || null;
    changed = !!(await state.editorEngine.cutSelection?.());
    if (changed) {
      await writeNativeClipboardFromSelection(fragmentJson, documentJson);
    }
  } else if (command === "paste") {
    changed = await pasteFromNativeClipboard();
    if (!changed) {
      changed = !!(await state.editorEngine.pasteClipboard?.());
    }
  } else if (command === "delete") {
    changed = await state.editorEngine.deleteSelection();
  } else if (command === "select-all") {
    await activateEditorTool("select");
    changed = !!(await state.editorEngine.selectAll?.());
    shouldRenderDocument = true;
  } else {
    return false;
  }
  if (changed || shouldRenderDocument) {
    await syncDocumentFromEngine();
    renderDocument();
  } else {
    renderEditorOverlay();
    refreshCommandAvailability();
  }
  return true;
}

async function activateEditorTool(nextTool) {
  if (!nextTool || editorState.activeTool === nextTool) {
    return false;
  }
  if (editorState.activeTool === "text" && nextTool !== "text") {
    await finishActiveTextEditor(true);
  }
  if (editorState.activeTool === "select" && nextTool !== "select") {
    activeSelectionGesture = null;
  }
  if (nextTool !== "bracket") {
    state.activeBracketDragStart = null;
  }
  editorState.activeTool = nextTool;
  document.querySelectorAll("[data-tool]").forEach((button) => {
    button.classList.toggle("is-active", button.dataset.tool === editorState.activeTool);
  });
  await syncEngineToolState();
  renderSecondaryToolbar();
  syncCanvasCursor();
  if (isEditingRustDocument()) {
    renderEditorOverlay(currentEditorRenderList());
  }
  return true;
}

function planZoomCenter(targetZoom) {
  if (state.zoomHandoffs.length && !isExpectedProgrammaticScroll()) {
    clearZoomHandoffs();
  }
  const previousZoom = zoomPercent;
  const currentCenter = currentViewportCenterWorld();
  const focus = zoomFocusBounds();
  const targetScale = viewportScaleForZoom(targetZoom);
  const direction = targetZoom > previousZoom ? 1 : targetZoom < previousZoom ? -1 : 0;
  if (!direction || !focus) {
    return { centerWorld: currentCenter, handoff: null };
  }

  if (direction > 0) {
    const currentVisible = visibleWorldRect(viewportScaleForZoom(previousZoom));
    const visibleFocus = intersectBounds(focus.bounds, currentVisible);
    const nextVisibleAtCurrentCenter = visibleWorldRectForCenter(currentCenter, targetScale);
    if (visibleFocus && !rectContainsBounds(nextVisibleAtCurrentCenter, visibleFocus)) {
      return {
        centerWorld: focus.center,
        handoff: {
          fromZoom: previousZoom,
          toZoom: targetZoom,
          restoreCenter: currentCenter,
          handoffCenter: focus.center,
          focusKey: focus.key,
        },
      };
    }
    return { centerWorld: currentCenter, handoff: null };
  }

  const handoff = state.zoomHandoffs[state.zoomHandoffs.length - 1];
  if (
    handoff
    && handoff.focusKey === focus.key
    && previousZoom <= handoff.toZoom + 0.5
    && targetZoom <= handoff.fromZoom + 0.5
  ) {
    state.zoomHandoffs.pop();
    return { centerWorld: handoff.restoreCenter, handoff: null };
  }

  const focusSize = boundsSize(focus.bounds);
  const visibleSize = visibleWorldSize(targetScale);
  if (focusSize.width <= visibleSize.width && focusSize.height <= visibleSize.height) {
    return { centerWorld: currentCenter, handoff: null };
  }
  return {
    centerWorld: constrainZoomCenterForBounds(currentCenter, focus.bounds, targetScale),
    handoff: null,
  };
}

function setZoomPercent(nextZoom, options = {}) {
  const previousZoom = zoomPercent;
  const targetZoom = closestZoomStep(nextZoom);
  const { centerWorld, handoff } = options.centerWorld
    ? { centerWorld: options.centerWorld, handoff: null }
    : planZoomCenter(targetZoom);
  zoomPercent = targetZoom;
  syncZoomControl();
  if (handoff) {
    state.zoomHandoffs.push(handoff);
  } else if (targetZoom > previousZoom) {
    const last = state.zoomHandoffs[state.zoomHandoffs.length - 1];
    if (last && last.toZoom < targetZoom) {
      last.toZoom = targetZoom;
    }
  }
  if (ensureEditorViewportCapacity(centerWorld)) {
    return;
  }
  applyViewerViewport({ centerWorld });
}

function handleViewerWheel(event) {
  if (!event.ctrlKey && !event.metaKey) {
    return;
  }
  event.preventDefault();
  if (!state.currentDocument || !viewerSvg) {
    return;
  }
  const direction = event.deltaY < 0 ? 1 : -1;
  setZoomPercent(nextZoomStep(direction));
}

function syncCanvasCursor() {
  if (!viewerSvg) {
    return;
  }
  if (activeSelectionGesture?.kind === "resize") {
    viewerSvg.style.cursor = activeSelectionGesture.cursor || "default";
    return;
  }
  if (activeSelectionGesture?.kind === "move" || activeSelectionGesture?.kind === "rotate") {
    viewerSvg.style.cursor = "grabbing";
    return;
  }
  viewerSvg.style.cursor = editorState.activeTool === "text"
    ? "text"
    : editorState.activeTool === "delete"
      ? DELETE_CURSOR
    : editorState.activeTool === "select"
      ? "default"
    : editorState.activeTool === "arrow"
      ? "crosshair"
      : "crosshair";
}

async function syncSelectCursorForPoint(point) {
  if (!viewerSvg || editorState.activeTool !== "select" || !isEditingRustDocument()) {
    syncCanvasCursor();
    return;
  }
  await syncArrowAwareCursorForPoint(point);
}

function cursorForShapeAction(action) {
  if (action === "circle-radius") {
    return "nwse-resize";
  }
  if (action === "ellipse-major-positive" || action === "ellipse-major-negative") {
    return "ew-resize";
  }
  if (action === "ellipse-minor-positive" || action === "ellipse-minor-negative") {
    return "ns-resize";
  }
  return {
    n: "ns-resize",
    s: "ns-resize",
    e: "ew-resize",
    w: "ew-resize",
    ne: "nesw-resize",
    sw: "nesw-resize",
    nw: "nwse-resize",
    se: "nwse-resize",
  }[action] || "";
}

async function syncArrowAwareCursorForPoint(point) {
  if (!viewerSvg || !isEditingRustDocument()) {
    syncCanvasCursor();
    return;
  }
  if (activeSelectionGesture?.kind === "move") {
    viewerSvg.style.cursor = "grabbing";
    return;
  }
  if (activeSelectionGesture?.kind === "rotate") {
    viewerSvg.style.cursor = "grabbing";
    return;
  }
  if (activeSelectionGesture?.kind === "resize") {
    viewerSvg.style.cursor = activeSelectionGesture.cursor || "default";
    return;
  }
  if (activeSelectionGesture?.kind === "arrow-endpoint") {
    viewerSvg.style.cursor = "move";
    return;
  }
  if (activeSelectionGesture?.kind === "arrow-curve") {
    viewerSvg.style.cursor = "nesw-resize";
    return;
  }
  if (activeSelectionGesture?.kind === "shape-resize") {
    viewerSvg.style.cursor = activeSelectionGesture.cursor || "nwse-resize";
    return;
  }
  if (editorState.activeTool === "select") {
    const resizeHandle = selectionResizeHandleHit(point);
    if (resizeHandle) {
      viewerSvg.style.cursor = resizeHandle.cursor;
      return;
    }
  }
  if (selectionRotateHandleHit(point)) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  const arrowAction = await state.editorEngine.hoverArrowAction?.(point.x, point.y) || "";
  if (arrowAction === "head" || arrowAction === "tail") {
    viewerSvg.style.cursor = "move";
    return;
  }
  if (arrowAction === "curve") {
    viewerSvg.style.cursor = "nesw-resize";
    return;
  }
  const overSelection = !!state.editorEngine.selectionContainsPoint?.(point.x, point.y);
  if (editorState.activeTool === "select" && overSelection) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  const shapeAction = await state.editorEngine.hoverShapeAction?.(point.x, point.y) || "";
  const shapeCursor = cursorForShapeAction(shapeAction);
  if (shapeCursor) {
    viewerSvg.style.cursor = shapeCursor;
    return;
  }
  if (editorState.activeTool === "arrow") {
    viewerSvg.style.cursor = "crosshair";
    return;
  }
  if (editorState.activeTool === "shape") {
    viewerSvg.style.cursor = "crosshair";
    return;
  }
  viewerSvg.style.cursor = overSelection ? "grab" : "default";
}

function renderSecondaryToolbar() {
  if (!secondaryToolbar) {
    return;
  }
  editorState.documentColors = currentDocumentColors();
  secondaryToolbar.innerHTML = renderSecondaryToolbarHtml(editorState);
  syncPrimaryToolButtons(editorState, document);
}

function currentDocumentColors() {
  if (typeof state.editorEngine?.documentColorsJson === "function") {
    const engineColorsJson = state.editorEngine.documentColorsJson();
    if (typeof engineColorsJson === "string") {
      const engineColors = parseEngineJson(engineColorsJson, null);
      if (Array.isArray(engineColors)) {
        return engineColors;
      }
    }
  }
  return [];
}

const textEditorController = createTextEditorController({
  getActiveEditor: () => activeTextEditor,
  setActiveEditor: (editor) => {
    activeTextEditor = editor;
  },
  textEditorLayer,
  editorState,
  textLength,
  runsPlainText,
  normalizeRuns: normalizeEditorSourceRuns,
  normalizeSelection: (plainText, selectionOffsets) => normalizeEditorSelectionOffsetsModel(plainText, selectionOffsets),
  splitRunsForSelection,
  styleAtOffset: styleAtEditorOffsetModel,
  cssColorToHex,
  editorRootBaseStyle,
  editorRootFontFamily,
  editorSourceRunsFromSession,
  previewTextEditLayoutFromKernel,
  defaultLineHeight: defaultTextEditorLineHeight,
  scriptScale: (script) => computeEditorScriptScale(sharedGlyphProfiles, script),
  scriptShiftEm: (script) => {
    if (script === "subscript") {
      return editorGlyphLayoutConfig().subscriptShiftDownEm;
    }
    if (script === "superscript") {
      return editorGlyphLayoutConfig().superscriptShiftUpEm;
    }
    return 0;
  },
  applyEditorRootFontFamily,
  syncTextToolbarStateFromSession,
  positionActiveTextEditor,
  syncTextEditorSize,
  updateCustomEditorChrome,
  defaultTextEditorLineHeight,
  editorOffsetFromPointerEvent,
  updateTextToolHoverFromPointerEvent,
  openHoveredTextEditTargetFromPointerEvent,
  buildEditorCaretLayout,
  editorLineIndexForOffset,
  measureEditorCaretRect,
  nearestOffsetOnLine,
});

function focusActiveTextEditor() {
  textEditorController.focusActiveTextEditor();
}

async function openTextEditorAt(point) {
  await finishActiveTextEditor(true);
  const sessionJson = await state.editorEngine?.beginTextEdit?.(point.x, point.y);
  const session = parseEngineJson(sessionJson, null);
  if (!session) {
    renderEditorOverlay(currentEditorRenderList());
    return;
  }
  renderEditorOverlay(currentEditorRenderList());
  openTextEditorSession(session);
  if (state.pendingTextSymbol) {
    const symbol = state.pendingTextSymbol;
    state.pendingTextSymbol = null;
    textEditorController.insertTextAtSelection(symbol);
    focusActiveTextEditor();
  }
}

function openTextEditorSession(session) {
  textEditorController.openTextEditorSession(engineSessionToEditorSession(session));
  renderDocument();
}

function textEditPrimitiveNodeId(primitive) {
  return primitive?.nodeId || primitive?.node_id || null;
}

function textEditPrimitiveObjectId(primitive) {
  return primitive?.objectId || primitive?.object_id || null;
}

function textEditHoverPrimitiveFromRenderList(renderList) {
  const hoverRoles = new Set(["hover-text-box", "hover-label-glyph", "hover-endpoint"]);
  return (renderList || []).find((primitive) => hoverRoles.has(primitive?.role)) || null;
}

function activeTextEditorTargetMatchesHoverPrimitive(primitive) {
  const target = activeTextEditor?.session?.target;
  if (!target || !primitive) {
    return false;
  }
  const role = primitive.role;
  if (role === "hover-text-box" || role === "hover-label-glyph") {
    const objectId = textEditPrimitiveObjectId(primitive);
    if (target.kind === "text-object" && objectId) {
      return objectId === (target.objectId || target.object_id || null);
    }
    const nodeId = textEditPrimitiveNodeId(primitive);
    if (target.kind === "endpoint-label" && nodeId) {
      return nodeId === (target.nodeId || target.node_id || null);
    }
    return false;
  }
  if (role === "hover-endpoint" && target.kind === "endpoint-label" && primitive.center) {
    const dx = Number(primitive.center.x) - Number(target.x);
    const dy = Number(primitive.center.y) - Number(target.y);
    return Math.hypot(dx, dy) <= 0.001;
  }
  return false;
}

async function updateTextToolHoverFromPointerEvent(event) {
  if (!routeEditorPointerEvents() || editorState.activeTool !== "text" || !state.editorEngine?.pointerMove) {
    return null;
  }
  const point = svgPointFromEvent(event);
  await state.editorEngine.pointerMove(point.x, point.y, event.altKey);
  const renderList = currentEditorRenderList();
  renderEditorOverlay(renderList);
  positionActiveTextEditor();
  return textEditHoverPrimitiveFromRenderList(renderList);
}

async function openHoveredTextEditTargetFromPointerEvent(event) {
  const hoverPrimitive = await updateTextToolHoverFromPointerEvent(event);
  if (!hoverPrimitive || activeTextEditorTargetMatchesHoverPrimitive(hoverPrimitive)) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  await openTextEditorAt(svgPointFromEvent(event));
  return true;
}

function editorSourceRunsFromSession(session, root) {
  return createEditorSourceRunsFromSession(session, root, {
    defaultFontFamily: editorState.textFontFamily,
    defaultFontSize: editorState.textFontSize,
    defaultTextColor: editorState.textColor,
    normalizeRuns: normalizeEditorSourceRuns,
    baseStyle: editorRootBaseStyle,
  });
}

function editorRootBaseStyle(root) {
  const baseFontSize = Number.parseFloat(root?.dataset?.baseFontSize || `${editorState.textFontSize}`)
    || editorState.textFontSize;
  return {
    fontFamily: editorRootFontFamily(root),
    fontSize: baseFontSize,
    fill: cssColorToHex(root.style.color || editorState.textColor),
    fontWeight: 400,
    fontStyle: "normal",
    underline: false,
    script: root.dataset.defaultChemical === "true" ? "chemical" : "normal",
  };
}

function syncTextToolbarStateFromSession(session) {
  editorState.textFontFamily = session.fontFamily || editorState.textFontFamily;
  const fontSize = Number(session.fontSize);
  if (Number.isFinite(fontSize) && fontSize > 0) {
    editorState.textFontSize = fontSize;
  }
  editorState.textColor = session.fill || editorState.textColor;
  editorState.textAlign = session.align || "left";
  editorState.textScript = session.defaultChemical ? "chemical" : "normal";
  editorState.textBold = false;
  editorState.textItalic = false;
  editorState.textUnderline = false;
  renderSecondaryToolbar();
}

function positionActiveTextEditor() {
  if (!activeTextEditor?.root) {
    return;
  }
  const { target } = activeTextEditor.session;
  const point = worldToLayerPoint({ x: target.x, y: target.y });
  if (!point) {
    return;
  }
  const root = activeTextEditor.root;
  const align = root.style.textAlign || "left";
  const anchorOffset = activeTextEditor.layout?.anchorOffset || { x: 0, y: 0 };
  const scale = editorDisplayScale();
  root.style.left = `${point.x}px`;
  root.style.top = `${point.y}px`;
  root.style.transform = `translate(${-anchorOffset.x * scale}px, ${-anchorOffset.y * scale}px) scale(${scale})`;
  root.dataset.anchor = align === "right"
    ? "end"
    : align === "center"
      ? "middle"
      : "start";
}

function syncEditorVisualMetrics() {
  if (!activeTextEditor?.root) {
    return;
  }
  const root = activeTextEditor.root;
  const baseFontSize = Number.parseFloat(root.dataset.baseFontSize || `${editorState.textFontSize}`)
    || editorState.textFontSize;
  const baseLineHeight = Number.parseFloat(root.dataset.baseLineHeight || `${defaultTextEditorLineHeight(baseFontSize)}`)
    || defaultTextEditorLineHeight(baseFontSize);
  root.style.fontSize = `${baseFontSize}px`;
  root.style.lineHeight = `${baseLineHeight}px`;
  root.style.minHeight = `${baseLineHeight}px`;
}

function syncTextEditorSize() {
  if (!activeTextEditor?.root) {
    return;
  }
  syncEditorVisualMetrics();
  const root = activeTextEditor.root;
  const display = activeTextEditor.display || root;
  const layout = activeTextEditor.layout;
  const width = Math.max(8, Math.ceil(Number(layout?.width || 0)));
  const height = Math.max(
    Number.parseFloat(root.style.minHeight || "15"),
    Math.ceil(Number(layout?.height || 0) || 0),
  );
  root.dataset.renderWidth = String(width);
  root.dataset.renderOffsetX = "0";
  root.dataset.renderOffsetY = "0";
  activeTextEditor.renderOffset = { x: 0, y: 0 };
  root.style.width = `${width}px`;
  root.style.height = `${height}px`;
  display.style.width = `${width}px`;
  display.style.height = `${height}px`;
  const svg = display.querySelector?.('svg[data-editor-text-svg="true"]');
  if (svg) {
    svg.setAttribute("width", String(width));
    svg.setAttribute("height", String(height));
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  }
  updateCustomEditorChrome();
  positionActiveTextEditor();
}

function defaultTextEditorLineHeight(fontSize) {
  const size = Number(fontSize || editorState.textFontSize) || editorState.textFontSize;
  return Math.max(size, size * 1.05);
}

function editorDisplayScale() {
  return Math.max(0.01, zoomScale());
}

function editorGlyphProfiles() {
  if (!sharedGlyphProfiles) {
    throw new Error("Shared glyph profiles have not loaded yet");
  }
  return sharedGlyphProfiles;
}

function editorGlyphLayoutConfig() {
  return editorGlyphProfiles().layout;
}

function buildEditorTextLayout() {
  return activeTextEditor?.layout || null;
}

function estimateTextRunsWidth(runs, fallbackFontSize = editorState.textFontSize) {
  return computeEstimateTextRunsWidth(
    sharedGlyphProfiles,
    runs,
    fallbackFontSize,
    editorState.textFontSize,
  );
}

function placeCaretAtEnd(element) {
  if (!activeTextEditor) {
    return;
  }
  const offset = textLength(activeTextEditor.plainText);
  setActiveEditorSelection({ anchor: offset, focus: offset }, false);
  renderActiveTextEditorFromModel();
}

function selectAllEditorText(element) {
  if (!activeTextEditor) {
    return;
  }
  setActiveEditorSelection({ anchor: 0, focus: textLength(activeTextEditor.plainText) }, false);
  renderActiveTextEditorFromModel();
}

function captureEditorCaretOffset(root) {
  const selectionOffsets = currentEditorSelectionOffsets();
  if (!selectionOffsets || !selectionOffsets.collapsed) {
    return null;
  }
  return selectionOffsets.anchor;
}

function restoreEditorCaretOffset(root, offset) {
  if (!Number.isFinite(offset)) {
    placeCaretAtEnd(root);
    return;
  }
  setActiveEditorSelection({ anchor: offset, focus: offset }, true);
}

function updateCustomEditorChrome() {
  if (!activeTextEditor?.root || !activeTextEditor.display || !activeTextEditor.caret || !activeTextEditor.input) {
    return;
  }
  const selection = currentEditorSelectionOffsets();
  const caret = activeTextEditor.caret;
  const selectionLayer = activeTextEditor.selectionLayer;
  const input = activeTextEditor.input;
  if (selectionLayer) {
    selectionLayer.replaceChildren();
  }
  if (!selection || !selection.collapsed) {
    caret.style.display = "none";
    renderEditorSelectionSegments(selection, selectionLayer);
    const focusRect = measureEditorCaretRect(selection?.focus ?? textLength(activeTextEditor.plainText));
    positionHiddenEditorInput(focusRect);
    return;
  }
  const caretRect = measureEditorCaretRect(selection.focus);
  if (!caretRect) {
    caret.style.display = "none";
    positionHiddenEditorInput(null);
    return;
  }
  caret.style.display = "block";
  caret.style.left = `${caretRect.x}px`;
  caret.style.top = `${caretRect.y}px`;
  caret.style.height = `${caretRect.height}px`;
  positionHiddenEditorInput(caretRect);
}

function renderEditorSelectionSegments(selection, selectionLayer) {
  if (!selection || selection.collapsed || !selectionLayer) {
    return;
  }
  const layout = buildEditorTextLayout();
  if (!layout) {
    return;
  }
  for (const segment of layout.selectionRects || []) {
    const node = document.createElement("div");
    node.className = "text-editor-selection-segment";
    node.style.left = `${segment.x}px`;
    node.style.top = `${segment.y}px`;
    node.style.width = `${Math.max(1, segment.width)}px`;
    node.style.height = `${Math.max(1, segment.height)}px`;
    selectionLayer.appendChild(node);
  }
}

function positionHiddenEditorInput(caretRect) {
  if (!activeTextEditor?.input) {
    return;
  }
  const input = activeTextEditor.input;
  if (!caretRect) {
    input.style.left = "0px";
    input.style.top = "0px";
    return;
  }
  input.style.left = `${caretRect.x}px`;
  input.style.top = `${caretRect.y}px`;
  input.style.height = `${Math.max(1, caretRect.height)}px`;
}

function measureEditorCaretRect(offset) {
  const layout = buildEditorTextLayout();
  if (!layout) {
    return null;
  }
  const caret = layout.caretPositions?.find((entry) => entry.offset === offset)
    || layout.caretPositions?.[Math.max(0, Math.min((layout.caretPositions?.length || 1) - 1, offset))];
  if (!caret) {
    return null;
  }
  return {
    x: caret.x,
    y: caret.y,
    width: 0,
    height: caret.height,
  };
}

function buildEditorCaretLayout() {
  const layout = buildEditorTextLayout();
  if (!layout) {
    return null;
  }
  return layout;
}

function editorLineIndexForOffset(offset) {
  const layout = buildEditorCaretLayout();
  if (!layout) {
    return -1;
  }
  for (let index = 0; index < layout.lines.length; index += 1) {
    const line = layout.lines[index];
    if (line.caretOffsets?.some((entry) => entry.offset === offset)) {
      return index;
    }
  }
  return layout.lines.length - 1;
}

function nearestOffsetOnLine(line, targetX) {
  if (!line?.caretOffsets?.length) {
    return 0;
  }
  return line.caretOffsets.reduce((best, entry) => {
    const bestDistance = Math.abs(best.x - targetX);
    const nextDistance = Math.abs(entry.x - targetX);
    if (nextDistance < bestDistance) {
      return entry;
    }
    return best;
  }).offset;
}

function editorOffsetFromPointerEvent(event) {
  const layout = buildEditorCaretLayout();
  if (!activeTextEditor?.display || !layout) {
    return 0;
  }
  const rect = activeTextEditor.display.getBoundingClientRect();
  const scale = editorDisplayScale();
  const localX = (event.clientX - rect.left) / scale;
  const localY = (event.clientY - rect.top) / scale;
  let line = layout.lines[0];
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const candidate of layout.lines) {
    const centerY = candidate.y + candidate.height * 0.5;
    const distance = Math.abs(centerY - localY);
    if (distance < bestDistance) {
      bestDistance = distance;
      line = candidate;
    }
  }
  if (!line) {
    return 0;
  }
  return nearestOffsetOnLine(line, localX);
}

function handleTextEditorPointerDown(event) {
  textEditorController.handleTextEditorPointerDown(event);
}

function handleTextEditorPointerMove(event) {
  textEditorController.handleTextEditorPointerMove(event);
}

function handleTextEditorPointerUp(event) {
  textEditorController.handleTextEditorPointerUp(event);
}

function handleTextEditorKeyDown(event) {
  textEditorController.handleTextEditorKeyDown(event);
}

async function finishActiveTextEditor(commit = true) {
  if (!activeTextEditor) {
    return false;
  }
  const { root, session, input } = activeTextEditor;
  input?.blur?.();
  const selection = window.getSelection?.();
  selection?.removeAllRanges?.();
  const nextSession = buildCommittedTextSession(session, root);
  textEditorLayer.replaceChildren();
  activeTextEditor = null;
  if (!commit) {
    renderDocument();
    return false;
  }
  const changed = await state.editorEngine?.applyTextEdit?.(JSON.stringify(editorSessionToEngineSession(nextSession)));
  await syncDocumentFromEngine();
  renderDocument();
  return Boolean(changed);
}

function buildCommittedTextSession(session, root) {
  const sourceRuns = normalizeEditorSourceRuns(
    activeTextEditor?.sourceRuns || [],
    editorRootBaseStyle(root),
  );
  const anchorOffset = activeTextEditor?.layout?.anchorOffset || { x: 0, y: 0 };
  const baseFontSize = Number.parseFloat(root.dataset.baseFontSize || `${editorState.textFontSize}`)
    || editorState.textFontSize;
  const baseLineHeight = Number.parseFloat(root.dataset.baseLineHeight || `${defaultTextEditorLineHeight(baseFontSize)}`)
    || defaultTextEditorLineHeight(baseFontSize);
  return {
    ...session,
    text: runsPlainText(sourceRuns),
    sourceRuns,
    fontFamily: editorRootFontFamily(root),
    fontSize: baseFontSize,
    fill: cssColorToHex(root.style.color || editorState.textColor),
    align: root.style.textAlign || editorState.textAlign,
    lineHeight: baseLineHeight,
    anchorOffset: session.target?.kind === "endpoint-label"
      ? [anchorOffset.x, anchorOffset.y]
      : undefined,
    defaultChemical: root.dataset.defaultChemical === "true",
  };
}

function normalizeEditorSourceRuns(runs, fallbackStyle) {
  return normalizeEditorSourceRunsModel(runs, fallbackStyle, cssColorToHex);
}

function cssColorToHex(color) {
  if (!color) {
    return "#000000";
  }
  if (/^#[0-9a-fA-F]{6}$/.test(color)) {
    return color.toLowerCase();
  }
  if (/^#[0-9a-fA-F]{3}$/.test(color)) {
    return `#${color[1]}${color[1]}${color[2]}${color[2]}${color[3]}${color[3]}`.toLowerCase();
  }
  const match = color.match(/\d+/g);
  if (!match || match.length < 3) {
    return color;
  }
  return `#${match.slice(0, 3).map((value) => Number(value).toString(16).padStart(2, "0")).join("")}`;
}

async function applySelectionColor(color) {
  const normalized = cssColorToHex(color);
  editorState.selectionColor = normalized;
  if (activeTextEditor) {
    applyTextInlineStyle({ color: normalized });
    return true;
  }
  if (!isEditingRustDocument() || !state.editorEngine?.applyColorToSelection) {
    return false;
  }
  const changed = !!(await state.editorEngine.applyColorToSelection(normalized));
  if (!changed) {
    return false;
  }
  await syncDocumentFromEngine();
  renderDocument();
  return true;
}

function applyTextAlignment(align) {
  if (!activeTextEditor?.root) {
    return;
  }
  activeTextEditor.root.style.textAlign = align === "justify" ? "justify" : align;
  syncTextEditorSize();
  positionActiveTextEditor();
}

function currentEditorSelectionOffsets() {
  return textEditorController.currentEditorSelectionOffsets();
}

function restoreEditorSelectionOffsets(selectionOffsets) {
  setActiveEditorSelection(selectionOffsets, false);
}

function normalizeEditorSelectionOffsets(selectionOffsets) {
  if (!activeTextEditor) {
    return null;
  }
  return normalizeEditorSelectionOffsetsModel(activeTextEditor.plainText, selectionOffsets);
}

function setActiveEditorSelection(selectionOffsets, syncDom = true) {
  return textEditorController.setActiveEditorSelection(selectionOffsets, syncDom);
}

function renderActiveTextEditorFromModel(selectionOffsets = null) {
  textEditorController.renderActiveTextEditorFromModel(selectionOffsets);
}

function syncPendingEditorStyleWithSelection() {
  textEditorController.syncPendingEditorStyleWithSelection();
}

function handleTextEditorBeforeInput(event, root) {
  textEditorController.handleTextEditorBeforeInput(event, root);
}

function applyTextFormatCommand(command) {
  textEditorController.applyTextFormatCommand(command);
}

function applyTextScript(script) {
  textEditorController.applyTextScript(script);
}

function applyTextInlineStyle(styles) {
  textEditorController.applyTextInlineStyle(styles);
}

function applyChemicalFormat() {
  textEditorController.applyChemicalFormat();
}

function insertTextAtSelection(text) {
  textEditorController.insertTextAtSelection(text);
}

function insertTextSymbol(character) {
  const symbol = String(character || "");
  if (!symbol) {
    return;
  }
  if (activeTextEditor) {
    textEditorController.insertTextAtSelection(symbol);
    focusActiveTextEditor();
    return;
  }
  state.pendingTextSymbol = symbol;
  editorState.activeTool = "text";
  void syncEngineToolState();
  renderSecondaryToolbar();
  syncCanvasCursor();
}

const documentFlow = createDocumentFlow({
  state,
  engineHost,
  desktopFileHost,
  openFileInput,
  viewerTitle,
  viewerStats,
  viewerSvg,
  docMeta,
  finishActiveTextEditor,
  clearZoomHandoffs,
  syncEngineToolState,
  syncDocumentFromEngine,
  syncCoreRenderListFromCurrentDocument,
  resetEditorEngine,
  pageViewBox,
  defaultEditorViewBox,
  renderDocument,
  fitView,
  waitForRuntimeReady: () => appRuntimeReady,
});

const {
  isAbortError,
  loadAndRender,
  loadJsonDocumentIntoEditor,
  openDocumentText,
  openDocumentFile,
  openDocumentPath,
  saveCurrentDocument,
  saveCurrentDocumentAs,
  saveCurrentDocumentCdxml,
  saveCurrentDocumentEmf,
  saveCurrentDocumentPdf,
  saveCurrentDocumentSvg,
  updateDocumentMeta,
} = documentFlow;

function browserTabUrlForPendingDocument(id) {
  const url = new URL(window.location.href);
  url.searchParams.set(BROWSER_PENDING_DOCUMENT_PARAM, id);
  return url.toString();
}

function openBrowserBlankDocumentTab() {
  if (typeof window === "undefined") {
    return false;
  }
  const url = new URL(window.location.href);
  url.searchParams.delete(BROWSER_PENDING_DOCUMENT_PARAM);
  return !!window.open(url.toString(), "_blank", "noopener,noreferrer");
}

async function openBrowserFileInNewTab(file) {
  if (!file) {
    return false;
  }
  const text = looksLikeCompressedChemcoreFile(file)
    ? await decompressChemcoreText(await file.arrayBuffer())
    : await file.text();
  const id = `doc-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const payload = {
    text,
    fileName: file.name || null,
    filePath: null,
    format: looksLikeCdxmlFile(file, text) ? "cdxml" : saveFormatFromFileName(file.name),
  };
  localStorage.setItem(`${BROWSER_PENDING_DOCUMENT_KEY_PREFIX}${id}`, JSON.stringify(payload));
  const opened = !!window.open(browserTabUrlForPendingDocument(id), "_blank", "noopener,noreferrer");
  if (!opened) {
    localStorage.removeItem(`${BROWSER_PENDING_DOCUMENT_KEY_PREFIX}${id}`);
  }
  return opened;
}

function takeBrowserPendingDocument() {
  if (isDesktopShell || typeof window === "undefined") {
    return null;
  }
  const id = new URL(window.location.href).searchParams.get(BROWSER_PENDING_DOCUMENT_PARAM);
  if (!id) {
    return null;
  }
  const key = `${BROWSER_PENDING_DOCUMENT_KEY_PREFIX}${id}`;
  const raw = localStorage.getItem(key);
  localStorage.removeItem(key);
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

async function documentSnapshotFromTab(tab) {
  if (!tab) {
    return null;
  }
  if (tab.id === activeDocumentTabId) {
    await finishActiveTextEditor(true);
    if (state.editorEngine) {
      await syncDocumentFromEngine();
    }
    saveActiveDocumentTabState();
  }
  const freshTab = documentTabs.find((entry) => entry.id === tab.id) || tab;
  if (!freshTab.currentDocument) {
    return null;
  }
  return {
    title: freshTab.title || documentTitleFromState(),
    fileName: freshTab.currentFileName || null,
    filePath: freshTab.currentFilePath || null,
    documentJson: JSON.stringify(freshTab.currentDocument),
    zoomPercent: Number(freshTab.zoomPercent || 100),
  };
}

async function detachDocumentTab(tabId, screenX = null, screenY = null) {
  if (!desktopFileHost?.detachDocumentWindow) {
    return false;
  }
  const tab = documentTabs.find((entry) => entry.id === tabId);
  const snapshot = await documentSnapshotFromTab(tab);
  if (!snapshot) {
    return false;
  }
  await desktopFileHost.detachDocumentWindow(snapshot, screenX, screenY);
  await closeDocumentTab(tabId);
  return true;
}

async function loadDetachedDocumentPayload(payload) {
  if (!payload?.documentJson) {
    return false;
  }
  const documentData = JSON.parse(payload.documentJson);
  await loadJsonDocumentIntoEditor(documentData, payload.fileName || null, payload.filePath || null);
  if (Number.isFinite(Number(payload.zoomPercent))) {
    setZoomPercent(Number(payload.zoomPercent));
  }
  saveActiveDocumentTabState();
  renderDocumentTabs();
  return true;
}

async function loadBrowserPendingDocumentPayload(payload) {
  if (!payload?.text) {
    return false;
  }
  await openDocumentText(payload.text, payload.fileName || null, payload.filePath || null, payload.format || null);
  saveActiveDocumentTabState();
  renderDocumentTabs();
  return true;
}

async function newDocumentTab() {
  if (!isDesktopShell && openBrowserBlankDocumentTab()) {
    return;
  }
  await appRuntimeReady;
  await finishActiveTextEditor(true);
  saveActiveDocumentTabState();
  const tab = createDocumentTab();
  documentTabs.push(tab);
  activeDocumentTabId = tab.id;
  await restoreDocumentTabState(tab);
  await resetEditorEngine();
  renderDocument();
  fitView();
  saveActiveDocumentTabState();
  renderDocumentTabs();
}

async function openDocumentPathInTab(path) {
  if (!path) {
    return;
  }
  await appRuntimeReady;
  await finishActiveTextEditor(true);
  saveActiveDocumentTabState();
  const previousTabId = activeDocumentTabId;
  const tab = createDocumentTab(fileNameFromPath(path) || "Loading...");
  documentTabs.push(tab);
  activeDocumentTabId = tab.id;
  await restoreDocumentTabState(tab);
  try {
    await openDocumentPath(path);
    saveActiveDocumentTabState();
    renderDocumentTabs();
  } catch (error) {
    await closeDocumentTab(tab.id);
    if (previousTabId && activeDocumentTabId !== previousTabId) {
      await activateDocumentTab(previousTabId);
    }
    throw error;
  }
}

async function openDocumentFileInTab(file) {
  if (!file) {
    return;
  }
  if (!isDesktopShell && await openBrowserFileInNewTab(file)) {
    return;
  }
  await appRuntimeReady;
  await finishActiveTextEditor(true);
  saveActiveDocumentTabState();
  const previousTabId = activeDocumentTabId;
  const tab = createDocumentTab(file.name || "Loading...");
  documentTabs.push(tab);
  activeDocumentTabId = tab.id;
  await restoreDocumentTabState(tab);
  try {
    await openDocumentFile(file);
    saveActiveDocumentTabState();
    renderDocumentTabs();
  } catch (error) {
    await closeDocumentTab(tab.id);
    if (previousTabId && activeDocumentTabId !== previousTabId) {
      await activateDocumentTab(previousTabId);
    }
    throw error;
  }
}

async function chooseAndOpenDocumentTab() {
  if (desktopFileHost?.available) {
    const path = await desktopFileHost.chooseOpenPath();
    if (path) {
      await openDocumentPathInTab(path);
    }
    return;
  }
  if (window.showOpenFilePicker) {
    const [handle] = await window.showOpenFilePicker({
      multiple: false,
      types: chemcoreOpenAcceptTypes(),
      excludeAcceptAllOption: false,
    });
    if (handle) {
      await openDocumentFileInTab(await handle.getFile());
    }
    return;
  }
  openFileInput.click();
}

async function confirmApplyDocumentStylePreset(preset) {
  const label = preset === "acs-document-1996" ? "ACS 1996" : "Default";
  const message = `Apply ${label} to this document? This will rescale the drawing and update existing bond, label, and graphic metrics.`;
  if (desktopFileHost?.confirmApplyStylePreset) {
    return desktopFileHost.confirmApplyStylePreset(label, message);
  }
  return window.confirm(message);
}

bindEditorControls({
  state,
  editorState,
  desktopFileHost,
  colorHost,
  openFileInput,
  zoomInput,
  secondaryToolbar,
  documentStyleButton,
  documentStyleMenu,
  confirmApplyDocumentStylePreset,
  getActiveTextEditor: () => activeTextEditor,
  clearActiveSelectionGesture: () => { activeSelectionGesture = null; },
  newDocumentTab,
  chooseAndOpenDocumentTab,
  openDocumentPathInTab,
  openDocumentFileInTab,
  getZoomPercent: () => zoomPercent,
  setTextFontSize: (size) => {
    const fontSize = normalizeToolbarFontSize(Math.max(5, Math.min(288, size)));
    editorState.textFontSize = cmToCssPx(fontSize);
  },
  isEditingRustDocument,
  syncEngineToolState,
  syncDocumentFromEngine,
  renderDocument,
  renderEditorOverlay,
  currentEditorRenderList,
  renderSecondaryToolbar,
  syncCanvasCursor,
  finishActiveTextEditor,
  openDocumentPath,
  saveCurrentDocument,
  saveCurrentDocumentAs,
  saveCurrentDocumentCdxml,
  saveCurrentDocumentEmf,
  saveCurrentDocumentPdf,
  saveCurrentDocumentSvg,
  isAbortError,
  runEditorCommand,
  setZoomPercent,
  nextZoomStep,
  fitView,
  resetEditorEngine,
  openDocumentFile,
  focusActiveTextEditor,
  applyTextAlignment,
  applyTextFormatCommand,
  applyTextScript,
  applyChemicalFormat,
  applyTextInlineStyle,
  applySelectionArrangeCommand,
  applyArrowOptionsToSelection,
  applySelectionColor,
  getDocumentColors: currentDocumentColors,
});

renderSecondaryToolbar();
syncCanvasCursor();

function svgPointFromEvent(event) {
  const screenMatrix = viewerSvg.getScreenCTM?.();
  if (screenMatrix) {
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(screenMatrix.inverse());
    return { x: point.x, y: point.y };
  }
  const rect = viewerSvg.getBoundingClientRect();
  const viewBox = viewerSvg.viewBox.baseVal;
  const activeBox = activeViewBox();
  const width = viewBox?.width || rect.width || activeBox.width;
  const height = viewBox?.height || rect.height || activeBox.height;
  return {
    x: (event.clientX - rect.left) * (width / Math.max(1, rect.width)) + (viewBox?.x || 0),
    y: (event.clientY - rect.top) * (height / Math.max(1, rect.height)) + (viewBox?.y || 0),
  };
}

function editorBondStrokeWidth() {
  const style = state.currentDocument?.styles?.style_molecule_default;
  return Number(style?.strokeWidth || style?.stroke_width || BOND_STROKE);
}

function routeEditorPointerEvents() {
  return isEditingRustDocument()
    && (editorState.activeTool === "bond"
      || editorState.activeTool === "delete"
      || editorState.activeTool === "arrow"
      || editorState.activeTool === "bracket"
      || editorState.activeTool === "symbol"
      || editorState.activeTool === "select"
      || editorState.activeTool === "text"
      || editorState.activeTool === "shape"
      || editorState.activeTool === "templates");
}

function isDocumentPreviewPrimitive(primitive) {
  return primitive?.role === "document-bond"
    || primitive?.role === "document-graphic"
    || primitive?.role === "document-knockout"
    || primitive?.role === "document-text";
}

function activeGestureUsesDocumentPreview() {
  if (activeDocumentPreviewObjectIds.size || activeDocumentPreviewLayer) {
    return false;
  }
  return ["move", "resize", "rotate", "arrow-endpoint", "arrow-curve", "shape-resize"]
    .includes(activeSelectionGesture?.kind);
}

function primitiveObjectId(primitive) {
  return primitive?.objectId || primitive?.object_id || null;
}

function primitiveNodeId(primitive) {
  return primitive?.nodeId || primitive?.node_id || null;
}

function primitiveBondId(primitive) {
  return primitive?.bondId || primitive?.bond_id || null;
}

function documentPrimitiveSelectedByState(primitive, selection) {
  if (!selection) {
    return false;
  }
  const objectId = primitiveObjectId(primitive);
  if (objectId && (
    selection.textObjects?.includes(objectId)
    || selection.arrowObjects?.includes(objectId)
  )) {
    return true;
  }
  const bondId = primitiveBondId(primitive);
  if (bondId && selection.bonds?.includes(bondId)) {
    return true;
  }
  const nodeId = primitiveNodeId(primitive);
  if (nodeId && (
    selection.nodes?.includes(nodeId)
    || selection.labelNodes?.includes(nodeId)
  )) {
    return true;
  }
  return false;
}

function selectedWholeDocumentObjectIds(renderList = currentEditorRenderList()) {
  const selection = currentEditorEngineState()?.selection;
  if (!selection || editorSelectionHasItems(selection) === false) {
    return [];
  }
  const objectSelection = new Map();
  for (const primitive of renderList || []) {
    if (!isDocumentPreviewPrimitive(primitive)) {
      continue;
    }
    const objectId = primitiveObjectId(primitive);
    if (!objectId || primitive.role === "document-knockout") {
      continue;
    }
    const entry = objectSelection.get(objectId) || { total: 0, selected: 0 };
    entry.total += 1;
    if (documentPrimitiveSelectedByState(primitive, selection)) {
      entry.selected += 1;
    }
    objectSelection.set(objectId, entry);
  }
  return [...objectSelection.entries()]
    .filter(([, entry]) => entry.total > 0 && entry.total === entry.selected)
    .map(([objectId]) => objectId);
}

function clearDocumentObjectPreviewTransform() {
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (activeDocumentPreviewLayer) {
    documentLayer?.removeAttribute("transform");
    activeDocumentPreviewLayer = false;
  }
  if (!activeDocumentPreviewObjectIds.size) {
    activeDocumentPreviewTransform = "";
    return;
  }
  for (const objectId of activeDocumentPreviewObjectIds) {
    const group = viewerSvg.querySelector(`[data-layer="document-content"] > [data-object-id="${CSS.escape(objectId)}"]`);
    group?.removeAttribute("transform");
    group?.classList.remove("is-preview-transforming");
  }
  activeDocumentPreviewObjectIds = new Set();
  activeDocumentPreviewTransform = "";
}

function selectionGestureTransform(gesture) {
  if (!gesture) {
    return "";
  }
  if (gesture.kind === "move") {
    const dx = (gesture.current?.x ?? gesture.start?.x ?? 0) - (gesture.start?.x ?? 0);
    const dy = (gesture.current?.y ?? gesture.start?.y ?? 0) - (gesture.start?.y ?? 0);
    return `translate(${dx} ${dy})`;
  }
  if (gesture.kind === "rotate" && gesture.center) {
    return `rotate(${gesture.angle || 0} ${gesture.center.x} ${gesture.center.y})`;
  }
  if (gesture.kind === "resize" && gesture.bounds && gesture.handle) {
    const pivot = selectionResizePivot(gesture.handle, gesture.bounds);
    const scale = gesture.scale || 1;
    return `translate(${pivot.x} ${pivot.y}) scale(${scale}) translate(${-pivot.x} ${-pivot.y})`;
  }
  return "";
}

function applyDocumentObjectPreviewTransform() {
  const transform = selectionGestureTransform(activeSelectionGesture);
  if (!transform) {
    clearDocumentObjectPreviewTransform();
    return false;
  }
  if (activeSelectionGesture.previewUsesLayer) {
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    documentLayer?.setAttribute("transform", transform);
    activeDocumentPreviewLayer = true;
    activeDocumentPreviewObjectIds = new Set();
    activeDocumentPreviewTransform = transform;
    return true;
  }
  const hasCachedObjectIds = Array.isArray(activeSelectionGesture.previewObjectIds);
  const objectIds = activeSelectionGesture.previewObjectIds || selectedWholeDocumentObjectIds();
  if (!objectIds.length) {
    clearDocumentObjectPreviewTransform();
    return false;
  }
  activeSelectionGesture.previewObjectIds = objectIds;
  const nextIds = new Set(objectIds);
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  const allGroups = hasCachedObjectIds
    ? []
    : [...viewerSvg.querySelectorAll('[data-layer="document-content"] > [data-object-id]')];
  const canTransformLayer = !hasCachedObjectIds
    && allGroups.length > 0
    && nextIds.size === allGroups.length
    && allGroups.every((group) => nextIds.has(group.dataset.objectId));
  if (canTransformLayer) {
    for (const objectId of activeDocumentPreviewObjectIds) {
      const group = viewerSvg.querySelector(`[data-layer="document-content"] > [data-object-id="${CSS.escape(objectId)}"]`);
      group?.removeAttribute("transform");
      group?.classList.remove("is-preview-transforming");
    }
    documentLayer?.setAttribute("transform", transform);
    activeDocumentPreviewLayer = true;
    activeDocumentPreviewObjectIds = new Set();
    activeDocumentPreviewTransform = transform;
    activeSelectionGesture.previewUsesLayer = true;
    return true;
  }
  if (activeDocumentPreviewLayer) {
    documentLayer?.removeAttribute("transform");
    activeDocumentPreviewLayer = false;
  }
  for (const objectId of activeDocumentPreviewObjectIds) {
    if (!nextIds.has(objectId)) {
      const group = viewerSvg.querySelector(`[data-layer="document-content"] > [data-object-id="${CSS.escape(objectId)}"]`);
      group?.removeAttribute("transform");
      group?.classList.remove("is-preview-transforming");
    }
  }
  for (const objectId of nextIds) {
    const group = viewerSvg.querySelector(`[data-layer="document-content"] > [data-object-id="${CSS.escape(objectId)}"]`);
    if (!group) {
      continue;
    }
    group.setAttribute("transform", transform);
    group.classList.add("is-preview-transforming");
  }
  activeDocumentPreviewObjectIds = nextIds;
  activeDocumentPreviewTransform = transform;
  return true;
}

function syncEditorOverlayPreviewTransform() {
  const overlay = viewerSvg.querySelector('[data-layer="editor-overlay"]');
  if (!overlay) {
    return false;
  }
  if (activeDocumentPreviewTransform) {
    overlay.setAttribute("transform", activeDocumentPreviewTransform);
  } else {
    overlay.removeAttribute("transform");
  }
  return true;
}

function screenPxToWorld(px) {
  return px / Math.max(1, viewportScale());
}

function selectionRotateHandleFromBounds(bounds) {
  if (!bounds) {
    return null;
  }
  return {
    x: (bounds.minX + bounds.maxX) * 0.5,
    y: bounds.minY - screenPxToWorld(SELECTION_ROTATE_HANDLE_OFFSET_PX),
    radius: screenPxToWorld(SELECTION_ROTATE_HANDLE_RADIUS_PX),
    hitRadius: screenPxToWorld(SELECTION_ROTATE_HANDLE_HIT_RADIUS_PX),
    bounds,
  };
}

function currentSelectionRotateHandle() {
  return selectionRotateHandleFromBounds(currentRenderBounds("selection"));
}

function selectionRotateHandleHit(point) {
  const handle = currentSelectionRotateHandle();
  return !!handle && pointDistance(point, handle) <= handle.hitRadius;
}

function selectionBoxPrimitives(renderList = currentEditorRenderList()) {
  const selectionRoles = new Set(["selection-box", "selection-bond", "selection-node", "selection-text-box"]);
  return (renderList || [])
    .filter((primitive) => primitive?.kind === "rect"
      && selectionRoles.has(primitive.role)
      && primitive.width > 0
      && primitive.height > 0);
}

function selectionResizeHandlesForBounds(bounds) {
  if (!bounds) {
    return [];
  }
  const size = screenPxToWorld(SELECTION_RESIZE_HANDLE_SIZE_PX);
  const hitRadius = screenPxToWorld(SELECTION_RESIZE_HANDLE_HIT_RADIUS_PX);
  const midX = (bounds.minX + bounds.maxX) * 0.5;
  const midY = (bounds.minY + bounds.maxY) * 0.5;
  return [
    { name: "nw", x: bounds.minX, y: bounds.minY, cursor: "nwse-resize" },
    { name: "n", x: midX, y: bounds.minY, cursor: "ns-resize" },
    { name: "ne", x: bounds.maxX, y: bounds.minY, cursor: "nesw-resize" },
    { name: "e", x: bounds.maxX, y: midY, cursor: "ew-resize" },
    { name: "se", x: bounds.maxX, y: bounds.maxY, cursor: "nwse-resize" },
    { name: "s", x: midX, y: bounds.maxY, cursor: "ns-resize" },
    { name: "sw", x: bounds.minX, y: bounds.maxY, cursor: "nesw-resize" },
    { name: "w", x: bounds.minX, y: midY, cursor: "ew-resize" },
  ].map((handle) => ({ ...handle, size, hitRadius, bounds }));
}

function selectionResizeHandles(renderList = currentEditorRenderList()) {
  const handles = selectionBoxPrimitives(renderList).flatMap((primitive) => selectionResizeHandlesForBounds({
    minX: primitive.x,
    minY: primitive.y,
    maxX: primitive.x + primitive.width,
    maxY: primitive.y + primitive.height,
  }));
  const globalBounds = currentRenderBounds("selection");
  if (globalBounds) {
    handles.push(...selectionResizeHandlesForBounds(globalBounds).map((handle) => ({
      ...handle,
      global: true,
    })));
  }
  return handles;
}

function selectionResizeHandleHit(point) {
  return selectionResizeHandles()
    .map((handle) => {
      const dx = Math.abs(point.x - handle.x);
      const dy = Math.abs(point.y - handle.y);
      const squareHit = dx <= handle.hitRadius && dy <= handle.hitRadius;
      const distance = pointDistance(point, handle);
      return { handle, distance, squareHit };
    })
    .filter((entry) => entry.squareHit || entry.distance <= entry.handle.hitRadius)
    .sort((a, b) => {
      const cornerPriority = Number(b.handle.name.length === 2) - Number(a.handle.name.length === 2);
      if (cornerPriority) {
        return cornerPriority;
      }
      const globalPriority = Number(b.handle.global) - Number(a.handle.global);
      if (globalPriority) {
        return globalPriority;
      }
      return a.distance - b.distance;
    })[0]?.handle || null;
}

function selectionResizePivot(handleName, bounds) {
  const centerX = (bounds.minX + bounds.maxX) * 0.5;
  const centerY = (bounds.minY + bounds.maxY) * 0.5;
  switch (handleName) {
    case "n": return { x: centerX, y: bounds.maxY };
    case "s": return { x: centerX, y: bounds.minY };
    case "e": return { x: bounds.minX, y: centerY };
    case "w": return { x: bounds.maxX, y: centerY };
    case "ne": return { x: bounds.minX, y: bounds.maxY };
    case "nw": return { x: bounds.maxX, y: bounds.maxY };
    case "se": return { x: bounds.minX, y: bounds.minY };
    case "sw": return { x: bounds.maxX, y: bounds.minY };
    default: return { x: centerX, y: centerY };
  }
}

function selectionResizeHandlePoint(handleName, bounds) {
  const centerX = (bounds.minX + bounds.maxX) * 0.5;
  const centerY = (bounds.minY + bounds.maxY) * 0.5;
  switch (handleName) {
    case "n": return { x: centerX, y: bounds.minY };
    case "s": return { x: centerX, y: bounds.maxY };
    case "e": return { x: bounds.maxX, y: centerY };
    case "w": return { x: bounds.minX, y: centerY };
    case "ne": return { x: bounds.maxX, y: bounds.minY };
    case "nw": return { x: bounds.minX, y: bounds.minY };
    case "se": return { x: bounds.maxX, y: bounds.maxY };
    case "sw": return { x: bounds.minX, y: bounds.maxY };
    default: return { x: centerX, y: centerY };
  }
}

function selectionResizeGestureScale(gesture, point) {
  const bounds = gesture?.bounds;
  const handle = gesture?.handle;
  if (!bounds || !handle) {
    return 1;
  }
  const width = Math.max(Number.EPSILON, bounds.maxX - bounds.minX);
  const height = Math.max(Number.EPSILON, bounds.maxY - bounds.minY);
  if (handle.length === 2) {
    const pivot = selectionResizePivot(handle, bounds);
    const original = selectionResizeHandlePoint(handle, bounds);
    const dx = original.x - pivot.x;
    const dy = original.y - pivot.y;
    const denominator = dx * dx + dy * dy;
    if (denominator <= Number.EPSILON) {
      return 1;
    }
    return Math.max(SELECTION_RESIZE_MIN_SCALE, ((point.x - pivot.x) * dx + (point.y - pivot.y) * dy) / denominator);
  }
  if (handle === "e") {
    return Math.max(SELECTION_RESIZE_MIN_SCALE, (point.x - bounds.minX) / width);
  }
  if (handle === "w") {
    return Math.max(SELECTION_RESIZE_MIN_SCALE, (bounds.maxX - point.x) / width);
  }
  if (handle === "s") {
    return Math.max(SELECTION_RESIZE_MIN_SCALE, (point.y - bounds.minY) / height);
  }
  if (handle === "n") {
    return Math.max(SELECTION_RESIZE_MIN_SCALE, (bounds.maxY - point.y) / height);
  }
  return 1;
}

function formatResizeScale(scale) {
  return `${(scale * 100).toFixed(1)}%`;
}

function signedAngleDelta(start, end) {
  let delta = ((end - start) % 360 + 360) % 360;
  if (delta > 180) {
    delta -= 360;
  }
  return delta;
}

function angleBetweenPoints(from, to) {
  const raw = Math.atan2(to.y - from.y, to.x - from.x) * 180 / Math.PI;
  return ((raw % 360) + 360) % 360;
}

function selectionRotateAngleForGesture(gesture, point, altKey) {
  if (!gesture?.center) {
    return 0;
  }
  const raw = signedAngleDelta(gesture.startAngle, angleBetweenPoints(gesture.center, point));
  return altKey ? raw : Math.round(raw / 15) * 15;
}

function formatRotationAngle(angle) {
  const rounded = Math.round(angle);
  return `${rounded}${String.fromCharCode(176)}`;
}

async function applySelectionArrangeCommand(command) {
  if (!isEditingRustDocument() || editorState.activeTool !== "select") {
    return false;
  }
  const changed = !!(await state.editorEngine.applySelectionArrangeCommand?.(command));
  if (!changed) {
    return false;
  }
  await syncDocumentFromEngine();
  renderDocument();
  return true;
}

async function applyArrowOptionsToSelection() {
  if (!isEditingRustDocument()) {
    return false;
  }
  const changed = state.editorEngine.applyArrowEndpointOptionsToSelection
    ? !!(await state.editorEngine.applyArrowEndpointOptionsToSelection(
      editorState.arrowType,
      editorState.arrowHeadSize,
      editorState.arrowCurve,
      editorState.arrowHeadStyle,
      editorState.arrowTailStyle,
      editorState.arrowNoGo,
      editorState.arrowBold,
    ))
    : !!(await state.editorEngine.applyArrowOptionsToSelection?.(
      editorState.arrowType,
      editorState.arrowHeadSize,
      editorState.arrowHead,
      editorState.arrowTail,
      editorState.arrowBold,
    ));
  if (changed) {
    await syncDocumentFromEngine();
    renderDocument();
  }
  return changed;
}

function bracketLabelAnchorPoint(start, end, kind = editorState.bracketKind) {
  const left = Math.min(start.x, end.x);
  const right = Math.max(start.x, end.x);
  const bottom = Math.max(start.y, end.y);
  const width = Math.abs(end.x - start.x);
  const height = Math.abs(end.y - start.y);
  let nominalRight = right;
  if (kind === "round") {
    const depth = Math.max(1, Math.min(height * (1 - Math.sqrt(3) * 0.5), width * 0.22));
    nominalRight = right - depth;
  } else if (kind === "curly") {
    const depth = Math.max(2, Math.min(height * 0.14423, width * 0.24));
    nominalRight = right - depth * 0.5;
  }
  return {
    x: nominalRight + 4.0,
    y: bottom - 8.0,
  };
}

async function handleEditorPointerMove(event) {
  const point = svgPointFromEvent(event);
  if ((editorState.activeTool === "select" || editorState.activeTool === "arrow" || editorState.activeTool === "shape") && activeSelectionGesture) {
    event.preventDefault();
    if (activeSelectionGesture.kind === "arrow-endpoint" || activeSelectionGesture.kind === "arrow-curve") {
      if (pointDistance(activeSelectionGesture.start, point) >= cssPxToCm(3)) {
        activeSelectionGesture.dragged = true;
      }
      activeSelectionGesture.current = point;
      await state.editorEngine.updateHoverArrowEdit?.(point.x, point.y, event.altKey);
      if (activeSelectionGesture.kind === "arrow-curve") {
        activeSelectionGesture.angle = state.editorEngine.activeArrowEditDegrees?.() || 0;
      }
      await syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(syncEditorRenderListFromEngine());
      return;
    }
    if (activeSelectionGesture.kind === "shape-resize") {
      if (pointDistance(activeSelectionGesture.start, point) >= cssPxToCm(3)) {
        activeSelectionGesture.dragged = true;
      }
      activeSelectionGesture.current = point;
      await state.editorEngine.updateHoverShapeEdit?.(point.x, point.y, event.altKey);
      await syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(syncEditorRenderListFromEngine());
      return;
    }
    if (activeSelectionGesture.kind === "rotate") {
      activeSelectionGesture.current = point;
      activeSelectionGesture.angle = selectionRotateAngleForGesture(activeSelectionGesture, point, event.altKey);
      if (applyDocumentObjectPreviewTransform()) {
        await syncSelectCursorForPoint(point);
        renderEditorOverlay(currentEditorOverlayRenderList());
        return;
      }
      await state.editorEngine.updateSelectionRotate(point.x, point.y, event.altKey);
      await syncSelectCursorForPoint(point);
      renderEditorOverlay(syncEditorRenderListFromEngine());
      return;
    }
    if (activeSelectionGesture.kind === "resize") {
      activeSelectionGesture.current = point;
      activeSelectionGesture.scale = selectionResizeGestureScale(activeSelectionGesture, point);
      if (applyDocumentObjectPreviewTransform()) {
        await syncSelectCursorForPoint(point);
        renderEditorOverlay(currentEditorOverlayRenderList());
        return;
      }
      await state.editorEngine.updateSelectionResize?.(point.x, point.y);
      await syncSelectCursorForPoint(point);
      renderEditorOverlay(syncEditorRenderListFromEngine());
      return;
    }
    if (activeSelectionGesture.kind === "move") {
      activeSelectionGesture.current = point;
      if (applyDocumentObjectPreviewTransform()) {
        await syncSelectCursorForPoint(point);
        if (!syncEditorOverlayPreviewTransform()) {
          renderEditorOverlay(currentEditorOverlayRenderList());
        }
        return;
      }
      await state.editorEngine.updateSelectionMove(point.x, point.y, event.altKey);
      await syncSelectCursorForPoint(point);
      renderEditorOverlay(syncEditorRenderListFromEngine());
      return;
    }
    if (pointDistance(activeSelectionGesture.start, point) >= cssPxToCm(3)) {
      activeSelectionGesture.dragged = true;
    }
    activeSelectionGesture.current = point;
    if (editorState.selectMode === "free") {
      const lastPoint = activeSelectionGesture.points[activeSelectionGesture.points.length - 1];
      if (!lastPoint || pointDistance(lastPoint, point) >= cssPxToCm(2)) {
        activeSelectionGesture.points.push(point);
      }
    }
    renderEditorOverlay(currentEditorRenderList());
    return;
  }
  if (!routeEditorPointerEvents()) {
    if (isEditingRustDocument()) {
      await state.editorEngine.clearInteraction();
      renderEditorOverlay();
    }
    return;
  }
  await state.editorEngine.pointerMove(point.x, point.y, event.altKey);
  if (editorState.activeTool === "select") {
    await syncSelectCursorForPoint(point);
  } else if (editorState.activeTool === "arrow" || editorState.activeTool === "shape") {
    await syncArrowAwareCursorForPoint(point);
  }
  const renderList = currentEditorRenderList();
  maybeAutoExpandEditorViewport(renderList);
  renderEditorOverlay(renderList);
  positionActiveTextEditor();
}

async function handleEditorPointerDown(event) {
  if (!routeEditorPointerEvents() || event.button !== 0) {
    return;
  }
  const point = svgPointFromEvent(event);
  state.lastEditFocusPoint = point;
  if (editorState.activeTool === "bracket") {
    state.activeBracketDragStart = point;
  }
  if (editorState.activeTool === "text") {
    event.preventDefault();
    await openTextEditorAt(point);
    return;
  }
  if (editorState.activeTool === "select") {
    event.preventDefault();
    viewerSvg.setPointerCapture?.(event.pointerId);
    await state.editorEngine.pointerMove(point.x, point.y, event.altKey);
    const resizeHandle = selectionResizeHandleHit(point);
    if (resizeHandle && await state.editorEngine.beginSelectionResize?.(resizeHandle.name, point.x, point.y)) {
      activeSelectionGesture = {
        kind: "resize",
        handle: resizeHandle.name,
        cursor: resizeHandle.cursor,
        bounds: currentRenderBounds("selection"),
        start: point,
        current: point,
        scale: 1,
      };
      await syncSelectCursorForPoint(point);
      syncEditorRenderListFromEngine();
      renderEditorOverlay(currentEditorOverlayRenderList());
      return;
    }
    const overSelection = !!state.editorEngine.selectionContainsPoint?.(point.x, point.y);
    const shapeEditAction = overSelection
      ? ""
      : await state.editorEngine.beginHoverShapeEdit?.(point.x, point.y) || "";
    if (shapeEditAction) {
      activeSelectionGesture = {
        kind: "shape-resize",
        action: shapeEditAction,
        cursor: cursorForShapeAction(shapeEditAction) || "nwse-resize",
        start: point,
        current: point,
        dragged: false,
        additive: !!event.shiftKey,
      };
      await syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(currentEditorRenderList());
      return;
    }
    const arrowEditAction = await state.editorEngine.beginHoverArrowEdit?.(point.x, point.y) || "";
    if (arrowEditAction) {
      activeSelectionGesture = {
        kind: arrowEditAction === "curve" ? "arrow-curve" : "arrow-endpoint",
        action: arrowEditAction,
        start: point,
        current: point,
        dragged: false,
        additive: !!event.shiftKey,
        angle: 0,
      };
      await syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(currentEditorRenderList());
      return;
    }
    const rotateHandle = currentSelectionRotateHandle();
    if (rotateHandle && pointDistance(point, rotateHandle) <= rotateHandle.hitRadius) {
      if (await state.editorEngine.beginSelectionRotate?.(point.x, point.y)) {
        activeSelectionGesture = {
          kind: "rotate",
          center: {
            x: (rotateHandle.bounds.minX + rotateHandle.bounds.maxX) * 0.5,
            y: (rotateHandle.bounds.minY + rotateHandle.bounds.maxY) * 0.5,
          },
          bounds: rotateHandle.bounds,
          start: point,
          current: point,
          startAngle: angleBetweenPoints(
            {
              x: (rotateHandle.bounds.minX + rotateHandle.bounds.maxX) * 0.5,
              y: (rotateHandle.bounds.minY + rotateHandle.bounds.maxY) * 0.5,
            },
            point,
          ),
          angle: 0,
        };
        await syncSelectCursorForPoint(point);
        syncEditorRenderListFromEngine();
        renderEditorOverlay(currentEditorOverlayRenderList());
        return;
      }
    }
    if (overSelection && await state.editorEngine.beginSelectionMove?.(point.x, point.y, !!event.shiftKey, event.altKey)) {
      activeSelectionGesture = {
        kind: "move",
        start: point,
        current: point,
        additive: !!event.shiftKey,
      };
      await syncSelectCursorForPoint(point);
      syncEditorRenderListFromEngine();
      renderEditorOverlay(currentEditorOverlayRenderList());
      return;
    }
    activeSelectionGesture = {
      kind: "select",
      start: point,
      current: point,
      points: [point],
      dragged: false,
      additive: !!event.shiftKey,
    };
    renderEditorOverlay(currentEditorRenderList());
    return;
  }
  event.preventDefault();
  viewerSvg.setPointerCapture?.(event.pointerId);
  if (editorState.activeTool === "arrow") {
    const arrowEditAction = await state.editorEngine.beginHoverArrowEdit?.(point.x, point.y) || "";
    if (arrowEditAction) {
      activeSelectionGesture = {
        kind: arrowEditAction === "curve" ? "arrow-curve" : "arrow-endpoint",
        action: arrowEditAction,
        start: point,
        current: point,
        dragged: false,
        angle: 0,
      };
      await syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(currentEditorRenderList());
      return;
    }
  }
  if (editorState.activeTool === "shape") {
    const shapeEditAction = await state.editorEngine.beginHoverShapeEdit?.(point.x, point.y) || "";
    if (shapeEditAction) {
      activeSelectionGesture = {
        kind: "shape-resize",
        action: shapeEditAction,
        cursor: cursorForShapeAction(shapeEditAction) || "nwse-resize",
        start: point,
        current: point,
        dragged: false,
      };
      await syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(currentEditorRenderList());
      return;
    }
  }
  await state.editorEngine.pointerDown(point.x, point.y, event.altKey);
  await syncDocumentFromEngine();
  renderEditorOverlay(currentEditorRenderList());
}

async function handleEditorPointerUp(event) {
  if (editorState.activeTool === "text") {
    return;
  }
  if (!routeEditorPointerEvents()) {
    return;
  }
  const point = svgPointFromEvent(event);
  state.lastEditFocusPoint = point;
  event.preventDefault();
  viewerSvg.releasePointerCapture?.(event.pointerId);
  if ((editorState.activeTool === "select" || editorState.activeTool === "arrow")
    && (activeSelectionGesture?.kind === "arrow-endpoint" || activeSelectionGesture?.kind === "arrow-curve")) {
    const gesture = activeSelectionGesture;
    activeSelectionGesture = null;
    const changed = !!(await state.editorEngine.finishHoverArrowEdit?.(point.x, point.y, event.altKey));
    if (changed) {
      await state.editorEngine.refreshRenderState?.();
      await syncDocumentFromEngine();
    } else if (!gesture.dragged && editorState.activeTool === "select") {
      await selectClickTarget(point, gesture.additive);
      clearDocumentObjectPreviewTransform();
      await renderSelectionOnlyUpdate(point, syncArrowAwareCursorForPoint);
      return;
    }
    if (changed) {
      await syncArrowAwareCursorForPoint(point);
      renderDocument();
    } else {
      clearDocumentObjectPreviewTransform();
      await renderSelectionOnlyUpdate(point, syncArrowAwareCursorForPoint);
    }
    return;
  }
  if ((editorState.activeTool === "select" || editorState.activeTool === "shape")
    && activeSelectionGesture?.kind === "shape-resize") {
    const gesture = activeSelectionGesture;
    activeSelectionGesture = null;
    const changed = !!(await state.editorEngine.finishHoverShapeEdit?.(point.x, point.y, event.altKey));
    if (changed) {
      await state.editorEngine.refreshRenderState?.();
      await syncDocumentFromEngine();
    } else if (!gesture.dragged && editorState.activeTool === "select") {
      await selectClickTarget(point, gesture.additive);
      clearDocumentObjectPreviewTransform();
      await renderSelectionOnlyUpdate(point, syncArrowAwareCursorForPoint);
      return;
    }
    if (changed) {
      await syncArrowAwareCursorForPoint(point);
      renderDocument();
    } else {
      clearDocumentObjectPreviewTransform();
      await renderSelectionOnlyUpdate(point, syncArrowAwareCursorForPoint);
    }
    return;
  }
  if (editorState.activeTool === "select") {
    const gesture = activeSelectionGesture;
    activeSelectionGesture = null;
    if (!gesture) {
      return;
    }
    if (gesture.kind === "rotate") {
      await state.editorEngine.finishSelectionRotate(point.x, point.y, event.altKey);
      await syncDocumentFromEngine();
      await syncSelectCursorForPoint(point);
      clearDocumentObjectPreviewTransform();
      renderDocument();
      return;
    }
    if (gesture.kind === "resize") {
      await state.editorEngine.finishSelectionResize?.(point.x, point.y);
      await syncDocumentFromEngine();
      await syncSelectCursorForPoint(point);
      clearDocumentObjectPreviewTransform();
      renderDocument();
      return;
    }
    if (gesture.kind === "move") {
      if (gesture.dragged) {
        await state.editorEngine.finishSelectionMove(point.x, point.y, event.altKey);
        await syncDocumentFromEngine();
        await syncSelectCursorForPoint(point);
        clearDocumentObjectPreviewTransform();
        renderDocument();
      } else {
        await selectClickTarget(point, gesture.additive);
        clearDocumentObjectPreviewTransform();
        await renderSelectionOnlyUpdate(point);
      }
      return;
    }
    if (!gesture.dragged) {
      await selectClickTarget(point, gesture.additive);
    } else if (editorState.selectMode === "box") {
      await state.editorEngine.selectInRect(
        gesture.start.x,
        gesture.start.y,
        point.x,
        point.y,
        gesture.additive,
      );
    } else {
      const polygonPoints = [...gesture.points, point].map((candidate) => [candidate.x, candidate.y]);
      await state.editorEngine.selectInPolygon(JSON.stringify(polygonPoints), gesture.additive);
    }
    await renderSelectionOnlyUpdate(point);
    return;
  }
  await state.editorEngine.pointerUp(point.x, point.y, event.altKey);
  await syncDocumentFromEngine();
  renderDocument();
  if (editorState.activeTool === "bracket") {
    const start = state.activeBracketDragStart;
    state.activeBracketDragStart = null;
    if (start && pointDistance(start, point) >= cssPxToCm(4)) {
      await openTextEditorAt(bracketLabelAnchorPoint(start, point));
    }
  }
}

async function handleEditorPointerLeave() {
  if (!isEditingRustDocument()) {
    return;
  }
  if (editorState.activeTool === "select" && activeSelectionGesture) {
    return;
  }
  if (editorState.activeTool !== "text") {
    await state.editorEngine.clearInteraction();
    renderEditorOverlay();
  }
}

async function handleEditorDoubleClick(event) {
  if (!routeEditorPointerEvents() || editorState.activeTool !== "select") {
    return;
  }
  const point = svgPointFromEvent(event);
  const changed = !!(await state.editorEngine.selectComponentAtPoint?.(point.x, point.y, event.shiftKey));
  if (!changed) {
    return;
  }
  event.preventDefault();
  activeSelectionGesture = null;
  await renderSelectionOnlyUpdate(point);
}

function renderEditorOverlay(renderList = null) {
  viewerSvg.querySelector('[data-layer="editor-overlay"]')?.remove();
  if (!isEditingRustDocument()) {
    return;
  }
  const primitives = renderList || currentEditorRenderList();
  const overlay = makeSvgNode("g", { "data-layer": "editor-overlay", "pointer-events": "none" });
  if (activeDocumentPreviewTransform) {
    overlay.setAttribute("transform", activeDocumentPreviewTransform);
  }
  const previewActive = activeGestureUsesDocumentPreview()
    || primitives.some((primitive) => primitive.role === "preview-end");
  if (previewActive) {
    const viewBox = activeViewBox();
    const pageBackground = normalizeDisplayColor(
      state.currentDocument?.document?.page?.background,
      CHEMDRAW_PAGE_BACKGROUND,
    );
    overlay.appendChild(makeSvgNode("rect", {
      x: viewBox.x,
      y: viewBox.y,
      width: viewBox.width,
      height: viewBox.height,
      fill: pageBackground,
      "data-role": "preview-document-mask",
    }));
  }
  for (const primitive of primitives) {
    if (shouldHidePrimitiveForActiveEndpointEditor(primitive)) {
      continue;
    }
    if (isDocumentPreviewPrimitive(primitive)) {
      if (previewActive) {
        renderCorePrimitive(overlay, primitive, corePrimitiveRenderOptions());
      }
      continue;
    }
    if (primitive.kind === "line" && primitive.from && primitive.to) {
      if (primitive.role !== "selection-bond") {
        continue;
      }
      overlay.appendChild(makeSvgNode("line", {
        x1: primitive.from.x,
        y1: primitive.from.y,
        x2: primitive.to.x,
        y2: primitive.to.y,
        class: "editor-selection-bond",
        "stroke-width": primitiveStrokeWidthValue(primitive, editorBondStrokeWidth()),
        "data-role": primitive.role,
      }));
    } else if (primitive.kind === "polygon" && Array.isArray(primitive.points)) {
      const className = primitive.role === "hover-bond-center" ? "editor-bond-center-rect" : "";
      if (!className) {
        continue;
      }
      overlay.appendChild(makeSvgNode("polygon", {
        points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
        class: className,
        "data-role": primitive.role,
      }));
    } else if (primitive.kind === "rect") {
      const classByRole = {
        "hover-text-box": "editor-text-box-focus",
        "hover-label-glyph": "editor-label-glyph-focus",
        "hover-arrow-handle": "editor-arrow-focus-handle",
        "selection-box": "editor-selection-box",
        "selection-bond": "editor-selection-bond-box",
        "selection-node": "editor-selection-node-box",
        "selection-text-box": "editor-selection-text-box",
      };
      const className = classByRole[primitive.role];
      if (!className) {
        continue;
      }
      overlay.appendChild(makeSvgNode("rect", {
        x: primitive.x,
        y: primitive.y,
        width: primitive.width,
        height: primitive.height,
        class: className,
        "data-role": primitive.role,
      }));
    } else if (primitive.kind === "circle" && primitive.center) {
      const classByRole = {
        "hover-endpoint": "editor-endpoint-halo",
        "hover-bond-center": "editor-bond-center-halo",
        "hover-arrow-center": "editor-arrow-center-halo",
        "hover-arrow-handle": "editor-arrow-focus-handle",
        "hover-shape-handle": "editor-arrow-focus-handle",
        "preview-end": "editor-preview-end",
        "selection-bond-dot": "editor-selection-bond-dot",
      };
      const className = classByRole[primitive.role];
      if (!className) {
        continue;
      }
      overlay.appendChild(makeSvgNode("circle", {
        cx: primitive.center.x,
        cy: primitive.center.y,
        r: primitive.radius,
        class: className,
        "data-role": primitive.role,
      }));
    }
  }
  if (editorState.activeTool === "select" && activeSelectionGesture?.kind === "resize") {
    const bounds = currentRenderBounds("selection") || activeSelectionGesture.bounds;
    if (bounds) {
      const labelOffset = screenPxToWorld(8);
      overlay.appendChild(makeSvgNode("text", {
        x: bounds.maxX + labelOffset,
        y: bounds.minY - labelOffset,
        class: "editor-selection-resize-label",
        "data-role": "selection-resize-scale",
      }));
      overlay.lastChild.textContent = formatResizeScale(activeSelectionGesture.scale || 1);
    }
  } else if (editorState.activeTool === "select" && activeSelectionGesture?.kind === "rotate") {
    const bounds = activeSelectionGesture.bounds;
    const labelOffset = screenPxToWorld(8);
    overlay.appendChild(makeSvgNode("text", {
      x: bounds.maxX + labelOffset,
      y: bounds.minY - labelOffset,
      class: "editor-selection-rotate-angle",
      "data-role": "selection-rotate-angle",
    }));
    overlay.lastChild.textContent = formatRotationAngle(activeSelectionGesture.angle || 0);
  } else if ((editorState.activeTool === "select" || editorState.activeTool === "arrow")
    && activeSelectionGesture?.kind === "arrow-curve") {
    const labelOffset = screenPxToWorld(8);
    const point = activeSelectionGesture.current || activeSelectionGesture.start;
    overlay.appendChild(makeSvgNode("text", {
      x: point.x + labelOffset,
      y: point.y - labelOffset,
      class: "editor-selection-rotate-angle",
      "data-role": "arrow-curve-angle",
    }));
    overlay.lastChild.textContent = formatRotationAngle(activeSelectionGesture.angle || 0);
  } else if (editorState.activeTool === "select" && !activeSelectionGesture) {
    for (const handle of selectionResizeHandles(primitives)) {
      overlay.appendChild(makeSvgNode("rect", {
        x: handle.x - handle.size * 0.5,
        y: handle.y - handle.size * 0.5,
        width: handle.size,
        height: handle.size,
        class: "editor-selection-resize-handle",
        "data-role": `selection-resize-${handle.name}`,
      }));
    }
    const handle = selectionRotateHandleFromBounds(currentRenderBounds("selection"));
    if (handle) {
      const topCenter = {
        x: (handle.bounds.minX + handle.bounds.maxX) * 0.5,
        y: handle.bounds.minY,
      };
      overlay.appendChild(makeSvgNode("line", {
        x1: topCenter.x,
        y1: topCenter.y,
        x2: handle.x,
        y2: handle.y + handle.radius,
        class: "editor-selection-rotate-stem",
        "data-role": "selection-rotate-stem",
      }));
      overlay.appendChild(makeSvgNode("circle", {
        cx: handle.x,
        cy: handle.y,
        r: handle.radius,
        class: "editor-selection-rotate-handle",
        "data-role": "selection-rotate-handle",
      }));
      overlay.appendChild(makeSvgNode("path", {
        d: `M ${handle.x - handle.radius * 0.55} ${handle.y} A ${handle.radius * 0.55} ${handle.radius * 0.55} 0 1 1 ${handle.x + handle.radius * 0.35} ${handle.y + handle.radius * 0.42}`,
        class: "editor-selection-rotate-glyph",
        "data-role": "selection-rotate-glyph",
      }));
    }
  }
  if (editorState.activeTool === "select" && activeSelectionGesture?.dragged) {
    if (editorState.selectMode === "box") {
      const start = activeSelectionGesture.start;
      const current = activeSelectionGesture.current;
      overlay.appendChild(makeSvgNode("rect", {
        x: Math.min(start.x, current.x),
        y: Math.min(start.y, current.y),
        width: Math.abs(current.x - start.x),
        height: Math.abs(current.y - start.y),
        class: "editor-selection-marquee",
        "data-role": "selection-marquee",
      }));
    } else {
      const points = activeSelectionGesture.points
        .concat([activeSelectionGesture.current])
        .map((candidate) => `${candidate.x},${candidate.y}`)
        .join(" ");
      overlay.appendChild(makeSvgNode("polyline", {
        points,
        class: "editor-selection-lasso",
        "data-role": "selection-lasso",
      }));
    }
  }
  if (overlay.childNodes.length) {
    viewerSvg.appendChild(overlay);
  }
}

viewerSvg?.addEventListener("pointermove", handleEditorPointerMove);
viewerSvg?.addEventListener("pointerdown", handleEditorPointerDown);
viewerSvg?.addEventListener("pointerup", handleEditorPointerUp);
viewerSvg?.addEventListener("dblclick", handleEditorDoubleClick);
viewerSvg?.addEventListener("pointercancel", async () => {
  activeSelectionGesture = null;
  clearDocumentObjectPreviewTransform();
  await state.editorEngine?.clearInteraction?.();
  syncCanvasCursor();
  renderEditorOverlay();
});
viewerSvg?.addEventListener("pointerleave", handleEditorPointerLeave);
viewerContainer?.addEventListener("wheel", handleViewerWheel, { passive: false });
viewerContainer?.addEventListener("contextmenu", openCanvasContextMenu);
viewerContainer?.addEventListener("scroll", () => {
  closeCanvasContextMenu();
  positionActiveTextEditor();
});
document.addEventListener("pointerdown", (event) => {
  if (canvasContextMenu.hidden || canvasContextMenu.contains(event.target)) {
    return;
  }
  closeCanvasContextMenu();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    closeCanvasContextMenu();
  }
});
window.addEventListener("blur", closeCanvasContextMenu);

window.addEventListener("resize", () => {
  if (!state.currentDocument) {
    return;
  }
  const centerWorld = currentViewportCenterWorld();
  if (isEditingRustDocument()) {
    if (!ensureEditorViewportCapacity(centerWorld)) {
      applyViewerViewport({ centerWorld });
    }
    return;
  }
  applyViewerViewport({ centerWorld });
});

function buildRenderList(documentData) {
  return [...documentData.objects].sort((a, b) => {
    if (a.zIndex !== b.zIndex) {
      return a.zIndex - b.zIndex;
    }
    return a.id.localeCompare(b.id);
  });
}

function renderDocument() {
  const documentData = state.currentDocument;
  if (!documentData) {
    return;
  }

  const page = documentData.document.page;
  const viewBox = activeViewBox();
  viewerSvg.innerHTML = "";
  activeDocumentPreviewObjectIds = new Set();
  activeDocumentPreviewLayer = false;
  activeDocumentPreviewTransform = "";
  applyViewerViewport();
  const pageBackground = normalizeDisplayColor(page.background, CHEMDRAW_PAGE_BACKGROUND);
  viewerSvg.style.setProperty("--chemcore-page-bg", pageBackground);
  viewerSvg.appendChild(makeSvgNode("rect", {
    x: viewBox.x,
    y: viewBox.y,
    width: viewBox.width,
    height: viewBox.height,
    fill: pageBackground,
    "data-layer": "page-background",
  }));
  const documentLayer = makeSvgNode("g", { "data-layer": "document-content" });
  viewerSvg.appendChild(documentLayer);

  const visibleObjects = buildRenderList(documentData);

  for (const object of visibleObjects) {
    if (!object.visible) {
      continue;
    }
    if (object.type === "molecule" && toggleMolecules && !toggleMolecules.checked) {
      continue;
    }
    if (object.type === "line" && toggleLines && !toggleLines.checked) {
      continue;
    }
    if (object.type === "text" && toggleTexts && !toggleTexts.checked) {
      continue;
    }
    if (LABEL_DEBUG_MODE && object.type !== "molecule") {
      continue;
    }

    const objectLayer = makeSvgNode("g", {
      "data-object-id": object.id,
      "data-object-type": object.type,
    });

    if (object.type === "molecule") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(objectLayer, primitive, corePrimitiveRenderOptions()));
      }
    } else if (object.type === "shape") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(objectLayer, primitive, corePrimitiveRenderOptions()));
      } else {
        renderShapeObject(objectLayer, object, documentData.styles);
      }
    } else if (object.type === "line") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(objectLayer, primitive, corePrimitiveRenderOptions()));
      } else {
        renderLineObject(objectLayer, object, state.currentDocument.styles);
      }
    } else if (object.type === "text") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(objectLayer, primitive, corePrimitiveRenderOptions()));
      } else {
        renderTextObject(objectLayer, object);
      }
    } else if (object.type === "bracket" || object.type === "symbol") {
      const corePrimitives = corePrimitivesForObject(object.id);
      corePrimitives.forEach((primitive) => renderCorePrimitive(objectLayer, primitive, corePrimitiveRenderOptions()));
    }
    if (objectLayer.childNodes.length) {
      documentLayer.appendChild(objectLayer);
    }
  }

  const counts = {};
  for (const object of documentData.objects) {
    counts[object.type] = (counts[object.type] || 0) + 1;
  }
  viewerStats.textContent = Object.entries(counts)
    .map(([type, count]) => `${type}: ${count}`)
    .join(" | ");
  renderEditorOverlay();
  positionActiveTextEditor();
}

function fitView() {
  if (!state.currentDocument) {
    return;
  }
  clearZoomHandoffs();
  let nextViewBox;
  let fitTargetBox = null;
  if (isEditingRustDocument()) {
    const bounds = currentRenderBounds("document");
    if (!bounds) {
      nextViewBox = defaultEditorViewBox();
      state.runtimeViewBox = nextViewBox;
      zoomPercent = 100;
      syncZoomControl();
      applyViewerViewport({ centerWorld: { x: 0, y: 0 } });
      return;
    }
    const metrics = editorViewportMetrics();
    nextViewBox = editorCanvasViewBoxFromBounds(bounds);
    fitTargetBox = paddedViewBoxFromBounds(bounds, metrics.fitPaddingX, metrics.fitPaddingY);
    zoomPercent = fitZoomPercentForViewBox(fitTargetBox);
  } else {
    nextViewBox = pageViewBox(state.currentDocument.document.page);
    zoomPercent = fitZoomPercentForViewBox(nextViewBox);
  }
  state.runtimeViewBox = nextViewBox;
  syncZoomControl();
  const target = fitTargetBox || nextViewBox;
  applyViewerViewport({ centerWorld: { x: target.x + target.width / 2, y: target.y + target.height / 2 } });
}

watchDisplayMetrics();

async function loadInitialDocumentTabs() {
  ensureDocumentTab();
  renderDocumentTabs();
  const detachedDocument = await desktopFileHost?.takeDetachedDocument?.();
  if (detachedDocument) {
    await loadDetachedDocumentPayload(detachedDocument);
    return;
  }
  const browserPendingDocument = takeBrowserPendingDocument();
  if (browserPendingDocument) {
    await loadBrowserPendingDocumentPayload(browserPendingDocument);
    return;
  }
  const pendingStartupPaths = await desktopFileHost?.takeStartupOpenPaths?.();
  const startupPaths = Array.isArray(pendingStartupPaths) ? pendingStartupPaths : [];
  const [firstPath, ...extraPaths] = startupPaths;
  if (firstPath) {
    await openDocumentPath(firstPath);
    saveActiveDocumentTabState();
    renderDocumentTabs();
    for (const path of extraPaths) {
      await openDocumentPathInTab(path);
    }
    return;
  }
  await loadAndRender();
  saveActiveDocumentTabState();
  renderDocumentTabs();
}

try {
  await appRuntimeReady;
  await loadInitialDocumentTabs();
} catch (error) {
  viewerTitle.textContent = "Runtime load failed";
  viewerStats.textContent = "";
  docMeta.textContent = String(error);
  viewerSvg.innerHTML = "";
}
