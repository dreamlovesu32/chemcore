import {
  parseEngineJson,
  primitivesForObject,
  renderBoundsFromEngine,
  renderListFromEngine,
} from "./engine_bridge.js";
import { createAppDomRefs } from "./app_dom.js";
import { registerChemcoreDebug } from "./app_debug.js";
import { createColorHost } from "./color_host.js";
import { createObjectSettingsHost } from "./object_settings_host.js";
import { createNumericDialogHost } from "./numeric_dialog_host.js";
import { createDesktopFileHost, normalizeDesktopPath } from "./desktop_file_host.js";
import { createEngineHost } from "./engine_host.js?v=20260629-local-text-commit";
import { bindEditorControls, openColorDialog } from "./editor_bindings.js?v=20260627-browser-drop-tabs";
import { createDocumentFlow } from "./document_flow.js";
import { createBrowserDocumentTabs } from "./browser_document_tabs.js";
import { createAppWindowLifecycleHost } from "./app_window_lifecycle.js";
import { pointDistance } from "./geometry.js";
import {
  normalizeToolbarFontSize,
  syncPrimaryChromeIcons,
} from "./toolbar.js";
import {
  displayLabelFontFamily,
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";
import { createSceneRenderer } from "./scene_renderer.js";
import { createEditorOverlayRenderer } from "./editor_overlay.js?v=20260627-hover-scale";
import { createEditorSelectionState } from "./editor_selection_state.js";
import { createEditorDocumentRenderer } from "./editor_document_renderer.js";
import { createEditorToolbarHost } from "./editor_toolbar_host.js";
import { createEditorViewportHost } from "./editor_viewport_host.js";
import { createEditorPointerController } from "./editor_pointer_controller.js?v=20260629-deep-stability";
import { createCanvasContextMenuHost } from "./editor_context_menu.js";
import { createEditorCommandController } from "./editor_command_controller.js";
import { createEditorCommandEngine } from "./editor_command_engine.js?v=20260626-interaction-feedback";
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
  primitiveStrokeWidthValue,
  renderCorePrimitive,
} from "./primitive_dom_renderer.js";
import {
  engineTemplateForEditorState,
  engineToolForEditorState,
} from "./editor_tool_model.js";
import {
  DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX,
  selectionHandleZoneContainsPoint,
} from "./editor_selection_hit_model.js";
import {
  ptToCssPx,
  cssPxToPt,
  displayMetrics,
  mapLengthArray,
} from "./units.js";

const SAMPLE_FILES = [];

const VIEW_MODE = document.body.dataset.viewMode || "document";
const LABEL_DEBUG_MODE = VIEW_MODE === "label-debug";

const state = {
  currentPath: LABEL_DEBUG_MODE ? SAMPLE_FILES[0] : null,
  currentFileName: null,
  currentFilePath: null,
  savedDocumentJson: null,
  savedRevision: null,
  oleSyncedDocumentJson: null,
  oleSyncedRevision: null,
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
const colorHost = createColorHost({
  getPalette: (initialColor, customColors = []) => (
    state.editorEngine?.colorDialogPaletteJson?.(
      initialColor,
      JSON.stringify(customColors),
    )
  ),
});
const commandEngine = createEditorCommandEngine({
  engine: () => state.editorEngine,
  syncDocumentFromEngine,
  onDocumentCommitted: handleDocumentCommandCommitted,
});
const objectSettingsHost = createObjectSettingsHost({
  root: document.body,
  engine: () => state.editorEngine,
  commandEngine,
  onApply: async (result) => {
    renderDocumentChange(result);
  },
});
const numericDialogHost = createNumericDialogHost({
  root: document.body,
  engine: () => state.editorEngine,
  commandEngine,
  onApply: async (result) => {
    renderDocumentChange(result);
  },
});
const isDesktopShell = !!desktopFileHost?.available;
let sharedGlyphProfiles = null;
const sharedGlyphProfilesReady = loadSharedGlyphProfiles();

document.body.classList.toggle("desktop-shell", isDesktopShell);
document.body.classList.toggle("browser-shell", !isDesktopShell);

const DEFAULT_TEXT_FONT_SIZE = 10;
const BRACKET_LABEL_FONT_SIZE = 7.5;
const BRACKET_LABEL_LINE_HEIGHT = BRACKET_LABEL_FONT_SIZE * 1.2;
const BRACKET_LABEL_OFFSET_X = 3.12;
const BRACKET_LABEL_BASELINE_OFFSET_Y = 2.4;
const BRACKET_LABEL_OFFSET_Y = BRACKET_LABEL_BASELINE_OFFSET_Y - BRACKET_LABEL_FONT_SIZE * 0.82;
const BOND_STROKE = 1.0;
const CHEMDRAW_PAGE_BACKGROUND = "#ffffff";
const DELETE_CURSOR_SVG = encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
    <rect x="4" y="4" width="8" height="8" fill="#ffffff" stroke="#000000" stroke-width="1"/>
  </svg>`,
);
const DELETE_CURSOR = `url("data:image/svg+xml,${DELETE_CURSOR_SVG}") 8 8, crosshair`;

function elementCursor(symbol) {
  const safeSymbol = String(symbol || "P").replace(/[^A-Za-z]/g, "").slice(0, 2) || "P";
  const svg = encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
      <rect x="11.5" y="3" width="1" height="4" fill="#000000"/>
      <rect x="11.5" y="17" width="1" height="4" fill="#000000"/>
      <rect x="3" y="11.5" width="4" height="1" fill="#000000"/>
      <rect x="17" y="11.5" width="4" height="1" fill="#000000"/>
      <text x="12" y="15" text-anchor="middle" font-family="Arial, Helvetica, sans-serif" font-size="6pt" font-weight="400" fill="#000000">${safeSymbol}</text>
    </svg>`,
  );
  return `url("data:image/svg+xml,${svg}") 12 12, crosshair`;
}

const {
  sampleSelect,
  reloadButton,
  fitButton,
  toggleMolecules,
  toggleLines,
  toggleTexts,
  docMeta,
  viewerTitle,
  viewerStats,
  viewerSvg,
  viewerContainer,
  secondaryToolbar,
  selectionChemistrySummary,
  desktopTitlebar,
  documentTabsRoot,
  documentStyleButton,
  documentStyleMenu,
  zoomInput,
  openFileInput,
  textEditorLayer,
} = createAppDomRefs();
let canvasContextMenuHost = null;
let canvasContextMenu = null;
const canvasPointerShield = document.createElement("div");
canvasPointerShield.className = "canvas-pointer-shield";
document.body.appendChild(canvasPointerShield);
const canvasDragPreviewSvg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
canvasDragPreviewSvg.classList.add("canvas-drag-preview-svg");
canvasPointerShield.appendChild(canvasDragPreviewSvg);
let activeToolActivationPromise = Promise.resolve();
let canvasPointerShieldActive = false;
let resolveAppInitialDocumentReady = null;
const appInitialDocumentReady = new Promise((resolve) => {
  resolveAppInitialDocumentReady = resolve;
});
let lastEditorPointerActivityAt = 0;
registerChemcoreDebug({
  state,
  getEditorState: () => editorState,
  getEngineState: () => currentEditorEngineState(),
  getDocument: () => currentEditorDocumentData(),
  getActiveTextEditor: () => activeTextEditor,
  getActiveSelectionGesture: () => activeSelectionGesture,
  getDisplayMetrics: () => state.displayMetrics,
  engineHost,
  desktopFileHost,
  commandEngine,
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
  async loadDocumentForTest(documentData) {
    await appInitialDocumentReady;
    await loadJsonDocumentIntoEditor(documentData, "test-large-drag.ccjs", null);
    fitView();
    return state.currentDocument;
  },
  resetEditorEngine,
  renderDocumentChange,
  renderStats: {
    captureRenderListStacks: false,
    documentRenderCount: 0,
    renderListJsonCount: 0,
    lastRenderListJsonStack: "",
  },
  getRenderListJson() {
    return state.editorEngine?.renderListJson?.() || "[]";
  },
  setDisplayScale(scale = null) {
    return applyDisplayScaleOverride(scale);
  },
  worldToClient(x, y) {
    const matrix = viewerSvg?.getScreenCTM?.();
    if (!matrix) {
      return null;
    }
    const point = new DOMPoint(x, y).matrixTransform(matrix);
    return { x: point.x, y: point.y };
  },
});
const appRuntimeReady = Promise.all([
  engineHost.initialize(),
  sharedGlyphProfilesReady,
]);

syncPrimaryChromeIcons();

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

const documentTabs = [];
let activeDocumentTabId = null;
let activeTextEditor = null;
let activeTitlebarTabDrag = null;
let detachingDocumentTabId = null;
let suppressNextDocumentTabClick = false;
const editorEngineReadCache = {
  engine: null,
  revision: null,
  stateJson: null,
  parsedState: null,
  renderListJson: null,
  renderList: null,
  interactionRenderListJson: null,
  interactionRenderList: null,
  documentJson: null,
  parsedDocument: undefined,
  boundsJsonByScope: new Map(),
  boundsByScope: new Map(),
};

function invalidateEditorEngineReadCache() {
  editorEngineReadCache.engine = null;
  editorEngineReadCache.revision = null;
  editorEngineReadCache.stateJson = null;
  editorEngineReadCache.parsedState = null;
  editorEngineReadCache.renderListJson = null;
  editorEngineReadCache.renderList = null;
  editorEngineReadCache.interactionRenderListJson = null;
  editorEngineReadCache.interactionRenderList = null;
  editorEngineReadCache.documentJson = null;
  editorEngineReadCache.parsedDocument = undefined;
  editorEngineReadCache.boundsJsonByScope = new Map();
  editorEngineReadCache.boundsByScope = new Map();
}

const syncWindowTitle = () => {
  updateActiveDocumentTabTitle();
  const title = documentTitleFromState();
  const displayTitle = documentTitleWithDirtyMarker(title, currentDocumentIsDirty());
  document.title = `${displayTitle} - Chemcore`;
  desktopFileHost?.setWindowTitle?.(displayTitle).catch?.(() => {});
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
  secondaryToolbarTool: "bond",
  selectMode: "box",
  bondType: "single",
  bondIconSvgs: {},
  bondIconCacheKey: "",
  textIconSvgs: {},
  textIconCacheKey: "",
  textFontFamily: "Arial",
  textFontSize: ptToCssPx(DEFAULT_TEXT_FONT_SIZE),
  textColor: "#000000",
  selectionColor: "#000000",
  textAlign: "left",
  textBold: false,
  textItalic: false,
  textUnderline: false,
  textScript: "normal",
  arrowType: "solid",
  arrowIconSvgs: {},
  arrowIconCacheKey: "",
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
  shapeStyleByKind: {},
  shapeIconSvgs: {},
  shapeIconCacheKey: "",
  shapeColor: "#000000",
  orbitalTemplate: "s",
  orbitalStyle: "shaded",
  orbitalPhase: "plus",
  orbitalColor: "#000000",
  orbitalIconSvgs: {},
  orbitalIconCacheKey: "",
  chainIconSvg: "",
  chainIconCacheKey: "",
  documentColors: [],
  colorPalette: null,
  bracketKind: "round",
  symbolKind: "circle-plus",
  symbolIconSvgs: {},
  symbolIconCacheKey: "",
  elementSymbol: "P",
  elementAtomicNumber: 15,
  elementPlacementActive: false,
  elementPalette: null,
  massDigits: 2,
  template: "ring-6",
};
let activeSelectionGesture = null;
let activeTlcSpotHover = null;
let activeTlcLaneHover = null;
let deferredDocumentSyncHandle = 0;

const TAB_STATE_KEYS = [
  "currentPath",
  "currentFileName",
  "currentFilePath",
  "savedDocumentJson",
  "savedRevision",
  "oleSyncedDocumentJson",
  "oleSyncedRevision",
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
    savedDocumentJson: null,
    savedRevision: null,
    oleSyncedDocumentJson: null,
    oleSyncedRevision: null,
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
  tab.zoomPercent = getZoomPercent();
  tab.title = documentTitleFromState();
  syncWindowTitle();
}

async function restoreDocumentTabState(tab) {
  for (const key of TAB_STATE_KEYS) {
    state[key] = tab[key];
  }
  setStoredZoomPercent(Number(tab.zoomPercent || 100));
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

function documentTitleWithDirtyMarker(title, dirty) {
  return dirty ? `${title || "Untitled"} *` : title || "Untitled";
}

function currentDocumentSaveFingerprint() {
  return state.currentDocument ? JSON.stringify(state.currentDocument) : null;
}

function currentDocumentRevision() {
  const revision = state.editorEngine?.revision?.();
  return Number.isFinite(Number(revision)) ? Number(revision) : null;
}

function markCurrentDocumentSaved() {
  state.savedDocumentJson = null;
  state.savedRevision = currentDocumentRevision();
  const tab = activeDocumentTab();
  if (tab) {
    tab.savedDocumentJson = state.savedDocumentJson;
    tab.savedRevision = state.savedRevision;
  }
  refreshCommandAvailability();
  renderDocumentTabs();
  syncWindowTitle();
}

function activeTextEditorIsDirty() {
  return Boolean(activeTextEditor?.hasUserEdited);
}

function activeTextEditorIsNewTextObject() {
  const target = activeTextEditor?.session?.target;
  if (target?.kind !== "text-object") {
    return false;
  }
  return !(target.objectId || target.object_id);
}

function activeTextEditorHasVisibleText() {
  const text = String(activeTextEditor?.plainText || "");
  return text.trim().length > 0;
}

async function closeActiveTextEditorForToolAction() {
  if (!activeTextEditor) {
    return false;
  }
  const shouldCommit = !activeTextEditorIsNewTextObject() || activeTextEditorHasVisibleText();
  return finishActiveTextEditor(shouldCommit);
}

function currentDocumentIsDirty() {
  if (activeTextEditorIsDirty()) {
    return true;
  }
  const revision = currentDocumentRevision();
  if (revision != null && state.savedRevision != null) {
    return revision !== state.savedRevision;
  }
  const fingerprint = currentDocumentSaveFingerprint();
  return !!fingerprint && state.savedDocumentJson != null && fingerprint !== state.savedDocumentJson;
}

function canSaveCurrentDocument() {
  return currentDocumentIsDirty();
}

function isOleEditFilePath(path) {
  const fileName = fileNameFromPath(path).toLowerCase();
  return fileName.startsWith("chemcore-ole-edit-") && fileName.endsWith(".ccjs");
}

function markCurrentDocumentOfficeSynced() {
  state.oleSyncedDocumentJson = currentDocumentSaveFingerprint();
  state.oleSyncedRevision = currentDocumentRevision();
  const tab = activeDocumentTab();
  if (tab) {
    tab.oleSyncedDocumentJson = state.oleSyncedDocumentJson;
    tab.oleSyncedRevision = state.oleSyncedRevision;
  }
}

function tabDocumentFingerprint(tab) {
  return tab?.currentDocument ? JSON.stringify(tab.currentDocument) : null;
}

function tabDocumentRevision(tab) {
  try {
    const revision = tab?.editorEngine?.revision?.();
    return Number.isFinite(Number(revision)) ? Number(revision) : null;
  } catch {
    return null;
  }
}

function documentTabIsDirty(tab) {
  if (!tab) {
    return false;
  }
  if (tab.id === activeDocumentTabId && activeTextEditorIsDirty()) {
    return true;
  }
  const revision = tabDocumentRevision(tab);
  if (revision != null && tab.savedRevision != null) {
    return revision !== tab.savedRevision;
  }
  const fingerprint = tabDocumentFingerprint(tab);
  return !!fingerprint && tab.savedDocumentJson != null && fingerprint !== tab.savedDocumentJson;
}

async function buildOleEditPayloadForTab(tab) {
  const rawDocumentJson = await tab?.editorEngine?.documentJson?.();
  const documentJson = rawDocumentJson
    ? `${String(rawDocumentJson).trimEnd()}\n`
    : tab?.currentDocument
      ? `${JSON.stringify(tab.currentDocument, null, 2)}\n`
      : null;
  if (!documentJson || !String(documentJson).trim()) {
    return null;
  }
  let cdxml = null;
  try {
    cdxml = await tab.editorEngine?.documentCdxml?.() || null;
  } catch (error) {
    console.warn("Failed to build OLE edit CDXML payload", error);
  }
  return {
    chemcoreFragmentJson: null,
    chemcoreDocumentJson: documentJson,
    renderListJson: tab.editorEngine?.renderListJson?.() || null,
    cdxml,
    svg: null,
    text: cdxml,
  };
}

async function syncOleEditDocumentTabToOffice(tab, options = {}) {
  if (
    !(desktopFileHost?.writeOleEditPayload || desktopFileHost?.writeTransientPath)
    || !tab?.currentFilePath
    || !isOleEditFilePath(tab.currentFilePath)
  ) {
    return false;
  }
  if (!tab.currentDocument) {
    return false;
  }
  const fingerprint = tabDocumentFingerprint(tab);
  const revision = tabDocumentRevision(tab);
  if (
    !options.force
    && tab.oleSyncedDocumentJson === fingerprint
    && (revision == null || tab.oleSyncedRevision === revision)
  ) {
    return false;
  }
  const payload = await buildOleEditPayloadForTab(tab);
  if (!payload) {
    return false;
  }
  if (desktopFileHost.writeOleEditPayload) {
    await desktopFileHost.writeOleEditPayload(tab.currentFilePath, payload);
  } else {
    await desktopFileHost.writeTransientPath(tab.currentFilePath, payload.chemcoreDocumentJson);
  }
  tab.oleSyncedDocumentJson = fingerprint;
  tab.oleSyncedRevision = revision;
  if (options.markSaved) {
    tab.savedDocumentJson = fingerprint;
    tab.savedRevision = revision;
  }
  if (tab.id === activeDocumentTabId) {
    state.oleSyncedDocumentJson = tab.oleSyncedDocumentJson;
    state.oleSyncedRevision = tab.oleSyncedRevision;
    if (options.markSaved) {
      state.savedDocumentJson = tab.savedDocumentJson;
      state.savedRevision = tab.savedRevision;
    }
  }
  refreshCommandAvailability();
  return true;
}

async function autoSaveAllOleEditDocumentTabs() {
  await finishActiveTextEditor(true);
  if (state.editorEngine) {
    await syncDocumentFromEngine();
  }
  saveActiveDocumentTabState();
  for (const tab of documentTabs) {
    await syncOleEditDocumentTabToOffice(tab, { force: true });
  }
}

async function handleDocumentCommandCommitted(event) {
  if (event.deferDocumentSync) {
    renderDocumentTabs();
    syncWindowTitle();
    refreshCommandAvailability();
    scheduleDeferredDocumentSync();
    console.debug?.("[chemcore] document command committed", {
      type: event.commandType,
      revision: event.revision,
      source: event.source,
      deferredSync: true,
    });
    return;
  }
  saveActiveDocumentTabState();
  const tab = activeDocumentTab();
  if (tab) {
    await syncOleEditDocumentTabToOffice(tab);
  }
  renderDocumentTabs();
  syncWindowTitle();
  refreshCommandAvailability();
  console.debug?.("[chemcore] document command committed", {
    type: event.commandType,
    revision: event.revision,
    source: event.source,
  });
}

function fileNameFromPath(path) {
  return String(path || "").split(/[\\/]/).filter(Boolean).pop() || "";
}

function normalizedFilePathKey(path) {
  const value = String(path || "").trim();
  if (!value) {
    return "";
  }
  return value.replace(/\\/g, "/").replace(/\/+/g, "/").toLowerCase();
}

function documentTabForFilePath(path) {
  const key = normalizedFilePathKey(path);
  if (!key) {
    return null;
  }
  return documentTabs.find((tab) => normalizedFilePathKey(tab.currentFilePath) === key) || null;
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
    const baseTitle = tab.title || "Untitled";
    const title = escapeHtml(documentTitleWithDirtyMarker(baseTitle, documentTabIsDirty(tab)));
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

async function closeDocumentTab(tabId, options = {}) {
  const activeTabIdBeforeClose = activeDocumentTabId;
  let index = documentTabs.findIndex((tab) => tab.id === tabId);
  if (index < 0) {
    return false;
  }
  if (!options.skipUnsavedPrompt && !await confirmCloseDocumentTab(tabId)) {
    return false;
  }
  index = documentTabs.findIndex((tab) => tab.id === tabId);
  if (index < 0) {
    return true;
  }
  const closing = documentTabs[index];
  const wasActive = closing.id === activeDocumentTabId;
  if (wasActive) {
    await finishActiveTextEditor(true);
    saveActiveDocumentTabState();
  }
  await syncOleEditDocumentTabToOffice(closing, { force: true });
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
    return true;
  }
  if (wasActive) {
    const previousActiveTab = activeTabIdBeforeClose !== tabId
      ? documentTabs.find((tab) => tab.id === activeTabIdBeforeClose)
      : null;
    const nextTab = previousActiveTab || documentTabs[Math.max(0, Math.min(index, documentTabs.length - 1))];
    activeDocumentTabId = nextTab.id;
    await restoreDocumentTabState(nextTab);
  } else {
    renderDocumentTabs();
  }
  return true;
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

const appWindowLifecycleHost = createAppWindowLifecycleHost({
  state,
  documentTabs,
  desktopFileHost,
  desktopTitlebar,
  isDesktopShell: () => isDesktopShell,
  getActiveDocumentTabId: () => activeDocumentTabId,
  setActiveDocumentTabId: (value) => { activeDocumentTabId = value; },
  activeDocumentTab,
  activeTextEditorIsDirty,
  documentTabIsDirty,
  finishActiveTextEditor,
  syncDocumentFromEngine,
  saveActiveDocumentTabState,
  renderDocumentTabs,
  syncWindowTitle,
  refreshCommandAvailability,
  activateDocumentTab,
  saveCurrentDocument: (...args) => saveCurrentDocument(...args),
  isAbortError: (...args) => isAbortError(...args),
  autoSaveAllOleEditDocumentTabs,
});

function bindDesktopWindowChrome(...args) { return appWindowLifecycleHost.bindDesktopWindowChrome(...args); }
function syncDesktopMaximizedState(...args) { return appWindowLifecycleHost.syncDesktopMaximizedState(...args); }
function confirmRepeatUnitUngroupIfNeeded(...args) { return appWindowLifecycleHost.confirmRepeatUnitUngroupIfNeeded(...args); }
function prepareDocumentTabForDirtyCheck(...args) { return appWindowLifecycleHost.prepareDocumentTabForDirtyCheck(...args); }
function saveDocumentTabBeforeClose(...args) { return appWindowLifecycleHost.saveDocumentTabBeforeClose(...args); }
function confirmUnsavedChangesForTab(...args) { return appWindowLifecycleHost.confirmUnsavedChangesForTab(...args); }
function confirmCloseDocumentTab(...args) { return appWindowLifecycleHost.confirmCloseDocumentTab(...args); }
function confirmCloseAllDocumentTabs(...args) { return appWindowLifecycleHost.confirmCloseAllDocumentTabs(...args); }
function requestCloseWindow(...args) { return appWindowLifecycleHost.requestCloseWindow(...args); }
function bindBrowserBeforeUnloadGuard(...args) { return appWindowLifecycleHost.bindBrowserBeforeUnloadGuard(...args); }

bindDesktopWindowChrome();
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
  const effectiveTool = engineToolForEditorState(editorState);
  const effectiveTemplate = engineTemplateForEditorState(editorState);
  await state.editorEngine.setTool(effectiveTool, editorState.bondType);
  await state.editorEngine.setTemplate?.(effectiveTemplate);
  const shapeKind = editorState.activeTool === "tlc-plate" ? "tlc-plate" : editorState.shapeKind;
  await state.editorEngine.setShapeOptions?.(
    shapeKind,
    editorState.shapeStyle,
    editorState.shapeColor,
  );
  await state.editorEngine.setOrbitalOptions?.(
    editorState.orbitalTemplate,
    editorState.orbitalStyle,
    editorState.orbitalPhase,
    editorState.orbitalColor,
  );
  await state.editorEngine.setBracketOptions?.(editorState.bracketKind);
  await state.editorEngine.setSymbolOptions?.(editorState.symbolKind);
  await state.editorEngine.setElementOptions?.(editorState.elementSymbol, editorState.elementAtomicNumber);
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
  } else {
    await state.editorEngine.setArrowOptions?.(
      editorState.arrowType,
      editorState.arrowHeadSize,
      editorState.arrowHead,
      editorState.arrowTail,
      editorState.arrowBold,
    );
  }
  invalidateEditorEngineReadCache();
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
  return mapTextSessionLengths(session, ptToCssPx);
}

function editorSessionToEngineSession(session) {
  return mapTextSessionLengths(session, cssPxToPt);
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
  return preview ? mapTextEditLayoutLengths(preview, ptToCssPx) : null;
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

const editorViewportHost = createEditorViewportHost({
  state,
  viewerSvg,
  viewerContainer,
  zoomInput,
  textEditorLayer,
  isEditingRustDocument,
  currentRenderBounds,
  renderActiveTextEditorFromModel,
  currentEditorSelectionOffsets,
  positionActiveTextEditor,
  updateDocumentMeta: (...args) => updateDocumentMeta(...args),
  getActiveTextEditor: () => activeTextEditor,
});

function cloneViewBox(...args) { return editorViewportHost.cloneViewBox(...args); }
function pageViewBox(...args) { return editorViewportHost.pageViewBox(...args); }
function visibleWorldSize(...args) { return editorViewportHost.visibleWorldSize(...args); }
function viewportScaleForZoom(...args) { return editorViewportHost.viewportScaleForZoom(...args); }
function visibleWorldRect(...args) { return editorViewportHost.visibleWorldRect(...args); }
function visibleWorldRectForCenter(...args) { return editorViewportHost.visibleWorldRectForCenter(...args); }
function editorViewportMetrics(...args) { return editorViewportHost.editorViewportMetrics(...args); }
function defaultEditorViewBox(...args) { return editorViewportHost.defaultEditorViewBox(...args); }
function activeViewBox(...args) { return editorViewportHost.activeViewBox(...args); }
function viewportScale(...args) { return editorViewportHost.viewportScale(...args); }
function zoomScale(...args) { return editorViewportHost.zoomScale(...args); }
function refreshDisplayMetrics(...args) { return editorViewportHost.refreshDisplayMetrics(...args); }
function applyDisplayScaleOverride(...args) { return editorViewportHost.applyDisplayScaleOverride(...args); }
function watchDisplayMetrics(...args) { return editorViewportHost.watchDisplayMetrics(...args); }
function currentViewportCenterWorld(...args) { return editorViewportHost.currentViewportCenterWorld(...args); }
function worldToScreenPoint(...args) { return editorViewportHost.worldToScreenPoint(...args); }
function worldToLayerPoint(...args) { return editorViewportHost.worldToLayerPoint(...args); }
function documentContentBoundsForZoom(...args) { return editorViewportHost.documentContentBoundsForZoom(...args); }
function zoomFocusBounds(...args) { return editorViewportHost.zoomFocusBounds(...args); }
function clearZoomHandoffs(...args) { return editorViewportHost.clearZoomHandoffs(...args); }
function markProgrammaticScroll(...args) { return editorViewportHost.markProgrammaticScroll(...args); }
function rememberProgrammaticScrollPosition(...args) { return editorViewportHost.rememberProgrammaticScrollPosition(...args); }
function isExpectedProgrammaticScroll(...args) { return editorViewportHost.isExpectedProgrammaticScroll(...args); }
function constrainZoomCenterForBounds(...args) { return editorViewportHost.constrainZoomCenterForBounds(...args); }
function clampZoomPercent(...args) { return editorViewportHost.clampZoomPercent(...args); }
function closestZoomStep(...args) { return editorViewportHost.closestZoomStep(...args); }
function zoomStepAtOrBelow(...args) { return editorViewportHost.zoomStepAtOrBelow(...args); }
function syncZoomControl(...args) { return editorViewportHost.syncZoomControl(...args); }
function nextZoomStep(...args) { return editorViewportHost.nextZoomStep(...args); }
function scrollViewerToWorldPoint(...args) { return editorViewportHost.scrollViewerToWorldPoint(...args); }
function scrollViewerToWorldPointAtClient(...args) { return editorViewportHost.scrollViewerToWorldPointAtClient(...args); }
function applyViewerViewport(...args) { return editorViewportHost.applyViewerViewport(...args); }
function setRuntimeViewBox(...args) { return editorViewportHost.setRuntimeViewBox(...args); }
function fitZoomPercentForViewBox(...args) { return editorViewportHost.fitZoomPercentForViewBox(...args); }
function editorCanvasViewBoxFromBounds(...args) { return editorViewportHost.editorCanvasViewBoxFromBounds(...args); }
function ensureEditorViewportCapacity(...args) { return editorViewportHost.ensureEditorViewportCapacity(...args); }
function maybeAutoExpandEditorViewport(...args) { return editorViewportHost.maybeAutoExpandEditorViewport(...args); }
function planZoomCenter(...args) { return editorViewportHost.planZoomCenter(...args); }
function setZoomPercent(...args) { return editorViewportHost.setZoomPercent(...args); }
function handleViewerWheel(...args) { return editorViewportHost.handleViewerWheel(...args); }
function fitView(...args) { return editorViewportHost.fitView(...args); }
function getZoomPercent(...args) { return editorViewportHost.getZoomPercent(...args); }
function setStoredZoomPercent(...args) { return editorViewportHost.setStoredZoomPercent(...args); }
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

function editorEngineRevision(engine) {
  if (!engine?.revision) {
    return 0;
  }
  return Number(engine.revision()) || 0;
}

function currentEditorEngineReadCache() {
  const engine = state.editorEngine;
  if (!engine) {
    return null;
  }
  const revision = editorEngineRevision(engine);
  const canTrustRevision = typeof engine.revision === "function";
  const stateJson = canTrustRevision ? null : engine.stateJson?.() || "";
  if (
    editorEngineReadCache.engine !== engine
    || editorEngineReadCache.revision !== revision
    || (!canTrustRevision && editorEngineReadCache.stateJson !== stateJson)
  ) {
    editorEngineReadCache.engine = engine;
    editorEngineReadCache.revision = revision;
    editorEngineReadCache.stateJson = stateJson;
    editorEngineReadCache.parsedState = undefined;
    editorEngineReadCache.renderListJson = null;
    editorEngineReadCache.renderList = null;
    editorEngineReadCache.interactionRenderListJson = null;
    editorEngineReadCache.interactionRenderList = null;
    editorEngineReadCache.documentJson = null;
    editorEngineReadCache.parsedDocument = undefined;
    editorEngineReadCache.boundsJsonByScope = new Map();
    editorEngineReadCache.boundsByScope = new Map();
  }
  return editorEngineReadCache;
}

function currentEditorEngineState() {
  const cache = currentEditorEngineReadCache();
  if (!cache) {
    return null;
  }
  if (cache.stateJson === null) {
    cache.stateJson = state.editorEngine.stateJson?.() || "";
  }
  if (cache.parsedState === undefined) {
    cache.parsedState = parseEngineJson(cache.stateJson, null);
  }
  return cache.parsedState;
}

function currentEditorDocumentData() {
  if (!isEditingRustDocument()) {
    return state.currentDocument;
  }
  const cache = currentEditorEngineReadCache();
  if (!cache) {
    return state.currentDocument;
  }
  if (cache.documentJson === null) {
    cache.documentJson = state.editorEngine.documentJson?.() || "";
  }
  if (cache.parsedDocument === undefined) {
    cache.parsedDocument = parseEngineJson(cache.documentJson, null);
  }
  return cache.parsedDocument || state.currentDocument;
}

function currentEditorRenderList() {
  const cache = currentEditorEngineReadCache();
  if (!cache) {
    return [];
  }
  if (!cache.renderList) {
    if (window.__chemcoreDebug?.renderStats) {
      window.__chemcoreDebug.renderStats.renderListJsonCount += 1;
      if (window.__chemcoreDebug.renderStats.captureRenderListStacks) {
        window.__chemcoreDebug.renderStats.lastRenderListJsonStack = new Error().stack || "";
      }
    }
    cache.renderListJson = state.editorEngine.renderListJson?.() || "[]";
    cache.renderList = parseEngineJson(cache.renderListJson, []) || [];
  }
  return cache.renderList;
}

function currentEditorInteractionRenderList() {
  const cache = currentEditorEngineReadCache();
  if (!cache) {
    return [];
  }
  if (!cache.interactionRenderList) {
    cache.interactionRenderListJson = state.editorEngine.interactionRenderListJson?.() || "[]";
    cache.interactionRenderList = parseEngineJson(cache.interactionRenderListJson, []) || [];
  }
  return cache.interactionRenderList;
}

function currentEditorRenderBounds(scope = "all") {
  const cache = currentEditorEngineReadCache();
  if (!cache) {
    return null;
  }
  if (!cache.boundsByScope.has(scope)) {
    const json = scope === "selection" && state.editorEngine.selectionBoundsJson
      ? state.editorEngine.selectionBoundsJson()
      : state.editorEngine.renderBoundsJson?.(scope);
    cache.boundsJsonByScope.set(scope, json || "null");
    cache.boundsByScope.set(scope, parseEngineJson(json || "null", null));
  }
  return cache.boundsByScope.get(scope);
}

function currentRenderBounds(scope = "all") {
  if (isEditingRustDocument()) {
    return currentEditorRenderBounds(scope);
  }
  return renderBoundsFromEngine(state.documentEngine, scope);
}

function pointInAxisBounds(point, bounds, padding = 0) {
  return Boolean(point && bounds
    && point.x >= bounds.minX - padding
    && point.x <= bounds.maxX + padding
    && point.y >= bounds.minY - padding
    && point.y <= bounds.maxY + padding);
}

function currentSelectionBoundsContainsPoint(point, padding = 0) {
  if (!isEditingRustDocument()) {
    return false;
  }
  const selection = activeSelectionGesture?.previewSelection
    || currentEditorEngineState()?.selection;
  if (!editorSelectionHasItems(selection)) {
    return false;
  }
  return pointInAxisBounds(point, currentRenderBounds("selection"), padding);
}

function currentSelectionHitContainsPoint(point) {
  if (!isEditingRustDocument() || !point) {
    return false;
  }
  const selection = activeSelectionGesture?.previewSelection
    || currentEditorEngineState()?.selection;
  if (!editorSelectionHasItems(selection)) {
    return false;
  }
  return !!state.editorEngine?.selectionContainsPoint?.(point.x, point.y);
}

function currentSelectedContentHitContainsPoint(point) {
  if (!isEditingRustDocument() || !point) {
    return false;
  }
  const selection = activeSelectionGesture?.previewSelection
    || currentEditorEngineState()?.selection;
  if (!editorSelectionHasItems(selection)) {
    return false;
  }
  const hit = parseEngineJson(state.editorEngine?.contextHitTestJson?.(point.x, point.y) || "null", null);
  return hit?.selected === true;
}

function currentSelectionHandleZoneContainsPoint(point) {
  return selectionHandleZoneContainsPoint({
    point,
    bounds: currentRenderBounds("selection"),
    pointDistance,
    toWorld: screenPxToWorld,
    selectedContentHitContainsPoint: currentSelectedContentHitContainsPoint,
  });
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
    state.coreRenderList = currentEditorRenderList();
  }
}

function syncEditorRenderListFromEngine(options = {}) {
  if (!state.editorEngine) {
    return [];
  }
  const autoExpand = options.autoExpand ?? true;
  state.coreRenderList = currentEditorRenderList();
  if (autoExpand) {
    maybeAutoExpandEditorViewport(state.coreRenderList || []);
  }
  return state.coreRenderList || [];
}

function syncEditorSelectionRenderListFromEngine() {
  return currentEditorInteractionRenderList();
}

function currentEditorOverlayRenderList() {
  const renderList = currentEditorInteractionRenderList();
  return (renderList || []).filter((primitive) => !isDocumentPreviewPrimitive(primitive));
}

function currentSelectionItemCount(selection = currentEditorEngineState()?.selection) {
  if (!selection) {
    return 0;
  }
  return (selection.nodes?.length || 0)
    + (selection.bonds?.length || 0)
    + (selection.labelNodes?.length || selection.label_nodes?.length || 0)
    + (selection.textObjects?.length || selection.text_objects?.length || 0)
    + (selection.arrowObjects?.length || selection.arrow_objects?.length || 0);
}

function freshestPreviewSelection(cachedSelection = null) {
  const currentSelection = parseEngineJson(state.editorEngine?.stateJson?.() || "", null)?.selection || null;
  if (!cachedSelection) {
    return currentSelection;
  }
  if (!currentSelection) {
    return cachedSelection;
  }
  const cachedObjectIds = new Set([
    ...(cachedSelection.textObjects || cachedSelection.text_objects || []),
    ...(cachedSelection.arrowObjects || cachedSelection.arrow_objects || []),
  ]);
  const currentObjectIds = [
    ...(currentSelection.textObjects || currentSelection.text_objects || []),
    ...(currentSelection.arrowObjects || currentSelection.arrow_objects || []),
  ];
  if (currentObjectIds.some((objectId) => !cachedObjectIds.has(objectId))) {
    return currentSelection;
  }
  return currentSelectionItemCount(currentSelection) > currentSelectionItemCount(cachedSelection)
    ? currentSelection
    : cachedSelection;
}

function corePrimitivesForObject(objectId) {
  return primitivesForObject(state.coreRenderList, objectId);
}

const sceneRenderer = createSceneRenderer({
  labelDebugMode: LABEL_DEBUG_MODE,
  toggleMolecules: () => !(toggleMolecules && !toggleMolecules.checked),
  toggleLines: () => !(toggleLines && !toggleLines.checked),
  toggleTexts: () => !(toggleTexts && !toggleTexts.checked),
  hasCoreRenderList: () => Boolean(state.coreRenderList?.length),
  coreRenderList: () => state.coreRenderList || [],
  corePrimitivesForObject,
  corePrimitiveRenderOptions,
});

function activeEndpointEditorNodeId() {
  return activeTextEditor?.session?.target?.kind === "endpoint-label"
    ? activeTextEditor.session.target.nodeId || activeTextEditor.session.target.node_id
    : null;
}

function primitiveMatchesActiveTextEditorTarget(primitive) {
  const target = activeTextEditor?.session?.target;
  if (!target || !primitive) {
    return false;
  }
  const role = primitive.role;
  const primitiveObjectId = primitive.objectId || primitive.object_id || null;
  const primitiveNodeId = primitive.nodeId || primitive.node_id || null;
  if (target.kind === "text-object" && primitiveObjectId) {
    const targetObjectId = target.objectId || target.object_id || null;
    return primitiveObjectId === targetObjectId
      && (role === "hover-text-box" || role === "selection-text-box");
  }
  if (target.kind === "endpoint-label" && primitiveNodeId) {
    const targetNodeId = target.nodeId || target.node_id || null;
    return primitiveNodeId === targetNodeId
      && (role === "hover-text-box" || role === "hover-label-glyph" || role === "selection-text-box");
  }
  return false;
}

function currentDocumentBoundsContainsPoint(point, paddingPx = 0) {
  if (!isEditingRustDocument() || !point) {
    return true;
  }
  const bounds = currentRenderBounds("document") || currentRenderBounds("all");
  if (!bounds) {
    return true;
  }
  return pointInAxisBounds(point, bounds, screenPxToWorld(paddingPx));
}

function rotatePointAround(point, center, degrees) {
  const radians = degrees * Math.PI / 180;
  const cos = Math.cos(radians);
  const sin = Math.sin(radians);
  const dx = point.x - center.x;
  const dy = point.y - center.y;
  return {
    x: center.x + dx * cos - dy * sin,
    y: center.y + dx * sin + dy * cos,
  };
}

function pointToSegmentDistance(point, start, end) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const lengthSq = dx * dx + dy * dy;
  if (lengthSq <= 1e-9) {
    return pointDistance(point, start);
  }
  const t = Math.max(0, Math.min(1, (((point.x - start.x) * dx) + ((point.y - start.y) * dy)) / lengthSq));
  return pointDistance(point, {
    x: start.x + dx * t,
    y: start.y + dy * t,
  });
}

function bracketPairLip(width, height) {
  return Math.max(0, Math.min(height * 0.07248, width * 0.22));
}

function bracketPairDepth(width, height, kind) {
  if (kind === "curly") {
    return Math.max(0, Math.min(height * 0.14423, width * 0.24));
  }
  return Math.max(0, Math.min(height * (1 - Math.sqrt(3) * 0.5), width * 0.22));
}

function bracketStrokeHitPadding(object) {
  const strokeWidth = Number(object?.payload?.strokeWidth ?? object?.payload?.extra?.strokeWidth ?? 1);
  return Number.isFinite(strokeWidth) && strokeWidth > 0 ? strokeWidth * 0.5 : 0.5;
}

function bracketSideHandleX(kind, side, width) {
  if (kind === "round") {
    return side === "right" ? 0 : width;
  }
  return side === "right" ? width : 0;
}

function squareBracketSideLocalHit(point, x, y, width, height, side, pad) {
  const right = x + width;
  const bottom = y + height;
  if (side === "right") {
    return pointToSegmentDistance(point, { x: right, y }, { x: right, y: bottom }) <= pad
      || pointToSegmentDistance(point, { x, y }, { x: right, y }) <= pad
      || pointToSegmentDistance(point, { x, y: bottom }, { x: right, y: bottom }) <= pad;
  }
  return pointToSegmentDistance(point, { x, y }, { x, y: bottom }) <= pad
    || pointToSegmentDistance(point, { x, y }, { x: right, y }) <= pad
    || pointToSegmentDistance(point, { x, y: bottom }, { x: right, y: bottom }) <= pad;
}

function roundBracketSidePolyline(x, y, width, height, side) {
  const chordHalf = height * 0.5;
  const base = Math.sqrt(Math.max(0, height * height - chordHalf * chordHalf));
  const sampleCount = 24;
  const points = [];
  for (let index = 0; index <= sampleCount; index += 1) {
    const t = index / sampleCount;
    const dy = (t - 0.5) * height;
    const sagitta = Math.max(0, Math.sqrt(Math.max(0, height * height - dy * dy)) - base);
    const clampedSagitta = Math.min(width, sagitta);
    points.push({
      x: side === "right" ? x + clampedSagitta : x + width - clampedSagitta,
      y: y + height * t,
    });
  }
  return points;
}

function cubicPoint(p0, p1, p2, p3, t) {
  const mt = 1 - t;
  const mt2 = mt * mt;
  const t2 = t * t;
  return {
    x: p0.x * mt2 * mt + p1.x * 3 * mt2 * t + p2.x * 3 * mt * t2 + p3.x * t2 * t,
    y: p0.y * mt2 * mt + p1.y * 3 * mt2 * t + p2.y * 3 * mt * t2 + p3.y * t2 * t,
  };
}

function appendCubicSamples(points, p0, p1, p2, p3) {
  const sampleCount = 8;
  const start = points.length ? 1 : 0;
  for (let index = start; index <= sampleCount; index += 1) {
    points.push(cubicPoint(p0, p1, p2, p3, index / sampleCount));
  }
}

function curlyBracketSidePolyline(x, y, width, height, side) {
  const right = x + width;
  const bottom = y + height;
  const halfDepth = width * 0.5;
  const middle = y + height * 0.5;
  const cLarge = height * 0.039805;
  const cSmall = height * 0.032308;
  const topInner = y + halfDepth;
  const bottomInner = bottom - halfDepth;
  const points = [];
  if (side === "right") {
    const re = x;
    const rm = x + halfDepth;
    appendCubicSamples(points, { x: re, y: bottom }, { x: re + cLarge, y: bottom }, { x: rm, y: bottom - cSmall }, { x: rm, y: bottomInner });
    appendCubicSamples(points, { x: rm, y: bottomInner }, { x: rm, y: bottomInner }, { x: rm, y: middle + halfDepth }, { x: rm, y: middle + halfDepth });
    appendCubicSamples(points, { x: rm, y: middle + halfDepth }, { x: rm, y: middle + halfDepth - cLarge }, { x: rm + cSmall, y: middle }, { x: right, y: middle });
    appendCubicSamples(points, { x: right, y: middle }, { x: rm + cSmall, y: middle }, { x: rm, y: middle - halfDepth + cLarge }, { x: rm, y: middle - halfDepth });
    appendCubicSamples(points, { x: rm, y: middle - halfDepth }, { x: rm, y: middle - halfDepth }, { x: rm, y: y + cSmall }, { x: re + cLarge, y });
    appendCubicSamples(points, { x: re + cLarge, y }, { x: re, y }, { x: re, y }, { x: re, y });
    return points;
  }
  const le = right;
  const lm = x + halfDepth;
  appendCubicSamples(points, { x: le, y }, { x: le - cLarge, y }, { x: lm, y: y + cSmall }, { x: lm, y: topInner });
  appendCubicSamples(points, { x: lm, y: topInner }, { x: lm, y: topInner }, { x: lm, y: middle - halfDepth }, { x: lm, y: middle - halfDepth });
  appendCubicSamples(points, { x: lm, y: middle - halfDepth }, { x: lm, y: middle - halfDepth + cLarge }, { x: lm - cSmall, y: middle }, { x, y: middle });
  appendCubicSamples(points, { x, y: middle }, { x: lm - cSmall, y: middle }, { x: lm, y: middle + halfDepth - cLarge }, { x: lm, y: middle + halfDepth });
  appendCubicSamples(points, { x: lm, y: middle + halfDepth }, { x: lm, y: middle + halfDepth }, { x: lm, y: bottom - cSmall }, { x: le - cLarge, y: bottom });
  appendCubicSamples(points, { x: le - cLarge, y: bottom }, { x: le, y: bottom }, { x: le, y: bottom }, { x: le, y: bottom });
  return points;
}

function pointToPolylineDistance(point, points) {
  let distance = Infinity;
  for (let index = 1; index < points.length; index += 1) {
    distance = Math.min(distance, pointToSegmentDistance(point, points[index - 1], points[index]));
  }
  return distance;
}

function bracketSideLocalHit(point, x, y, width, height, kind, side, pad) {
  if (width <= 0 || height <= 0) {
    return false;
  }
  if (kind === "square") {
    return squareBracketSideLocalHit(point, x, y, width, height, side, pad);
  }
  const points = kind === "curly"
    ? curlyBracketSidePolyline(x, y, width, height, side)
    : roundBracketSidePolyline(x, y, width, height, side);
  return pointToPolylineDistance(point, points) <= pad;
}

function bracketPairLocalHit(point, x, y, width, height, kind, pad) {
  const right = x + width;
  const bottom = y + height;
  if (kind === "square") {
    const lip = bracketPairLip(width, height);
    return pointToSegmentDistance(point, { x, y }, { x, y: bottom }) <= pad
      || pointToSegmentDistance(point, { x: right, y }, { x: right, y: bottom }) <= pad
      || pointToSegmentDistance(point, { x, y }, { x: x + lip, y }) <= pad
      || pointToSegmentDistance(point, { x, y: bottom }, { x: x + lip, y: bottom }) <= pad
      || pointToSegmentDistance(point, { x: right - lip, y }, { x: right, y }) <= pad
      || pointToSegmentDistance(point, { x: right - lip, y: bottom }, { x: right, y: bottom }) <= pad;
  }
  const depth = bracketPairDepth(width, height, kind);
  const leftX = kind === "round" ? x - depth : x;
  const rightX = kind === "round" ? right : right - depth;
  return bracketSideLocalHit(point, leftX, y, depth, height, kind, "left", pad)
    || bracketSideLocalHit(point, rightX, y, depth, height, kind, "right", pad);
}

function shouldHidePrimitiveForActiveEndpointEditor(primitive) {
  if (primitiveMatchesActiveTextEditorTarget(primitive)) {
    return true;
  }
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
    || role === "document-diagnostic"
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

const editorOverlayRenderer = createEditorOverlayRenderer({
  currentRenderBounds,
  currentEditorRenderList: currentEditorInteractionRenderList,
  screenPxToWorld,
  pointDistance,
  viewerSvg: () => viewerSvg,
  isEditingRustDocument,
  activeDocumentPreviewTransform: () => editorDocumentRenderer.activeDocumentPreviewTransform(),
  activeGestureUsesDocumentPreview,
  activeGestureUsesObjectEditPreview,
  activeViewBox,
  currentPageBackground: () => state.currentDocument?.document?.page?.background,
  defaultPageBackground: () => CHEMDRAW_PAGE_BACKGROUND,
  shouldHidePrimitiveForActiveEndpointEditor,
  isDocumentPreviewPrimitive,
  corePrimitiveRenderOptions,
  primitiveStrokeWidthValue,
  editorBondStrokeWidth,
  editorState: () => editorState,
  activeSelectionGesture: () => activeSelectionGesture,
  activeTlcLaneHover: () => activeTlcLaneHover,
});

const editorSelectionState = createEditorSelectionState({
  state: () => state,
  editorState: () => editorState,
  currentEditorEngineState,
  activeDocumentTab,
  documentTitleFromState,
  parseEngineJson,
  activeSelectionGesture: () => activeSelectionGesture,
  setActiveTlcSpotHover: (value) => { activeTlcSpotHover = value; },
  setActiveTlcLaneHover: (value) => { activeTlcLaneHover = value; },
});

const editorSelectionHasItems = (...args) => editorSelectionState.editorSelectionHasItems(...args);
const currentEditorSelectionHasItems = (...args) => editorSelectionState.currentEditorSelectionHasItems(...args);
const currentDocumentHasSelectableContent = (...args) => editorSelectionState.currentDocumentHasSelectableContent(...args);
const activeDocumentTabIsBlankUntitled = (...args) => editorSelectionState.activeDocumentTabIsBlankUntitled(...args);
const currentSceneObjectMap = (...args) => editorSelectionState.currentSceneObjectMap(...args);
const currentEditableFragment = (...args) => editorSelectionState.currentEditableFragment(...args);
const currentSelectionInfo = (...args) => editorSelectionState.currentSelectionInfo(...args);
const clearTlcHoverState = (...args) => editorSelectionState.clearTlcHoverState(...args);
const updateTlcSpotHover = (...args) => editorSelectionState.updateTlcSpotHover(...args);
const contextSelectionCount = (...args) => editorSelectionState.contextSelectionCount(...args);
const contextHasSelection = (...args) => editorSelectionState.contextHasSelection(...args);
const selectedSceneObjects = (...args) => editorSelectionState.selectedSceneObjects(...args);

function currentDocumentHasTlcPlate() {
  return (state.currentDocument?.objects || []).some((object) => {
    const payload = object?.payload || {};
    const extra = payload.extra || payload;
    return (object?.type || object?.objectType || object?.object_type) === "shape"
      && (payload.kind === "tlcPlate" || extra.kind === "tlcPlate");
  });
}

const editorDocumentRenderer = createEditorDocumentRenderer({
  state,
  viewerSvg,
  viewerStats,
  sceneRenderer,
  makeSvgNode,
  parseEngineJson,
  activeSelectionGesture: () => activeSelectionGesture,
  currentEditorEngineState,
  currentEditorRenderList,
  editorSelectionHasItems,
  worldPointForFragmentNode,
  isEditingRustDocument,
  renderDocument,
  renderEditorOverlay,
  positionActiveTextEditor,
  syncDocumentFromEngine,
  corePrimitiveRenderOptions,
  editorBondStrokeWidth,
  selectionResizePivot: (...args) => selectionResizePivot(...args),
  freshestPreviewSelection,
});

function sceneObjectType(...args) { return editorDocumentRenderer.sceneObjectType(...args); }
function currentDocumentSceneObjectMap(...args) { return editorDocumentRenderer.currentDocumentSceneObjectMap(...args); }
function currentDocumentSceneObjectParentMap(...args) { return editorDocumentRenderer.currentDocumentSceneObjectParentMap(...args); }
function currentDocumentObjectIdsInPaintOrder(...args) { return editorDocumentRenderer.currentDocumentObjectIdsInPaintOrder(...args); }
function targetIdsFromCommandResult(...args) { return editorDocumentRenderer.targetIdsFromCommandResult(...args); }
function renderDocumentChange(...args) { return editorDocumentRenderer.renderDocumentChange(...args); }
function renderDocumentPrimitiveChange(...args) { return editorDocumentRenderer.renderDocumentPrimitiveChange(...args); }
function rebuildDocumentPrimitiveIndex(...args) { return editorDocumentRenderer.rebuildDocumentPrimitiveIndex(...args); }
function ensureDocumentObjectDomForCommandResult(...args) { return editorDocumentRenderer.ensureDocumentObjectDomForCommandResult(...args); }
function selectionNeedsBackendMovePreview(...args) { return editorDocumentRenderer.selectionNeedsBackendMovePreview(...args); }
function applyBackendSelectionMovePreview(...args) { return editorDocumentRenderer.applyBackendSelectionMovePreview(...args); }
function syncViewerStats(...args) { return editorDocumentRenderer.syncViewerStats(...args); }
function isDocumentPreviewPrimitive(...args) { return editorDocumentRenderer.isDocumentPreviewPrimitive(...args); }
function activeGestureUsesDocumentPreview(...args) { return editorDocumentRenderer.activeGestureUsesDocumentPreview(...args); }
function activeGestureUsesObjectEditPreview(...args) { return editorDocumentRenderer.activeGestureUsesObjectEditPreview(...args); }
function primitiveObjectId(...args) { return editorDocumentRenderer.primitiveObjectId(...args); }
function primitiveNodeId(...args) { return editorDocumentRenderer.primitiveNodeId(...args); }
function primitiveBondId(...args) { return editorDocumentRenderer.primitiveBondId(...args); }
function collectCurrentDocumentSceneObjects(...args) { return editorDocumentRenderer.collectCurrentDocumentSceneObjects(...args); }
function currentDocumentMoleculeTopology(...args) { return editorDocumentRenderer.currentDocumentMoleculeTopology(...args); }
function syncObjectEditPreviewHiddenElements(...args) { return editorDocumentRenderer.syncObjectEditPreviewHiddenElements(...args); }
function clearDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.clearDocumentObjectPreviewTransform(...args); }
function commitDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.commitDocumentObjectPreviewTransform(...args); }
function canCommitDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.canCommitDocumentObjectPreviewTransform(...args); }
function applyDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.applyDocumentObjectPreviewTransform(...args); }
function hideDocumentDiagnosticsForPreview(...args) { return editorDocumentRenderer.hideDocumentDiagnosticsForPreview(...args); }
function resetDocumentRenderState(...args) { return editorDocumentRenderer.resetDocumentRenderState(...args); }
function renderEditorOverlay(renderList = null) {
  const objectEditPreviewActive = activeGestureUsesObjectEditPreview();
  const effectiveRenderList = objectEditPreviewActive && !renderList
    ? currentEditorInteractionRenderList()
    : renderList;
  syncObjectEditPreviewHiddenElements(objectEditPreviewActive ? effectiveRenderList || [] : []);
  editorOverlayRenderer.renderEditorOverlay(effectiveRenderList);
}

function syncCanvasDragPreviewViewport() {
  canvasDragPreviewSvg.style.left = "0";
  canvasDragPreviewSvg.style.top = "0";
  canvasDragPreviewSvg.style.width = "100vw";
  canvasDragPreviewSvg.style.height = "100vh";
  canvasDragPreviewSvg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
}

function clearCanvasDragPreview() {
  const hadPreview = canvasDragPreviewSvg.childElementCount > 0
    || canvasDragPreviewSvg.hasAttribute("viewBox");
  canvasDragPreviewSvg.replaceChildren();
  canvasDragPreviewSvg.removeAttribute("viewBox");
  return hadPreview;
}

function screenPointFromSvgMatrix(point, matrix) {
  return {
    x: Number(point?.x || 0) * matrix.a + Number(point?.y || 0) * matrix.c + matrix.e,
    y: Number(point?.x || 0) * matrix.b + Number(point?.y || 0) * matrix.d + matrix.f,
  };
}

function canvasScreenFeedbackPrimitiveNode(primitive, matrix) {
  if (!matrix || primitive?.kind !== "circle" || !primitive.center) {
    return null;
  }
  if (primitive.role !== "preview-end" && primitive.role !== "hover-endpoint") {
    return null;
  }
  const center = screenPointFromSvgMatrix(primitive.center, matrix);
  const scale = Math.hypot(matrix.a || 0, matrix.b || 0);
  const radius = Number(primitive.radius || 0) * scale;
  return makeSvgNode("circle", {
    cx: center.x,
    cy: center.y,
    r: radius,
    class: "editor-endpoint-halo",
    "data-role": primitive.role,
  });
}

function renderCanvasDragPreview(renderList = []) {
  canvasDragPreviewSvg.replaceChildren();
  if (!renderList?.length) {
    return;
  }
  syncCanvasDragPreviewViewport();
  const matrix = viewerSvg.getScreenCTM?.();
  const target = matrix
    ? makeSvgNode("g", {
        transform: `matrix(${matrix.a} ${matrix.b} ${matrix.c} ${matrix.d} ${matrix.e} ${matrix.f})`,
      })
    : canvasDragPreviewSvg;
  const screenFeedbackNodes = [];
  for (const primitive of renderList) {
    const feedbackNode = canvasScreenFeedbackPrimitiveNode(primitive, matrix);
    if (feedbackNode) {
      screenFeedbackNodes.push(feedbackNode);
      continue;
    }
    renderCorePrimitive(target, primitive, corePrimitiveRenderOptions());
  }
  if (target !== canvasDragPreviewSvg) {
    canvasDragPreviewSvg.appendChild(target);
  }
  for (const node of screenFeedbackNodes) {
    canvasDragPreviewSvg.appendChild(node);
  }
}
const currentSelectionRotateHandle = (...args) => editorOverlayRenderer.currentSelectionRotateHandle(...args);
const selectionResizeHandleHit = (...args) => editorOverlayRenderer.selectionResizeHandleHit(...args);
const selectionResizePivot = (...args) => editorOverlayRenderer.selectionResizePivot(...args);
const selectionResizeGestureScale = (...args) => editorOverlayRenderer.selectionResizeGestureScale(...args);
const selectionRotateAngleForGesture = (...args) => editorOverlayRenderer.selectionRotateAngleForGesture(...args);
const selectionRotateHandleHit = (...args) => editorOverlayRenderer.selectionRotateHandleHit(...args);

const editorCommandController = createEditorCommandController({
  state: () => state,
  desktopFileHost,
  isEditingRustDocument,
  syncDocumentFromEngine,
  renderDocument,
  renderDocumentChange,
  renderEditorOverlay,
  refreshCommandAvailability,
  activateEditorTool,
  commandEngine,
});
const writeNativeClipboardFromSelection = (...args) => editorCommandController.writeNativeClipboardFromSelection(...args);
const pasteFromNativeClipboard = (...args) => editorCommandController.pasteFromNativeClipboard(...args);
const runEditorCommand = (...args) => editorCommandController.runEditorCommand(...args);

canvasContextMenuHost = createCanvasContextMenuHost({
  state: () => state,
  editorState: () => editorState,
  desktopFileHost,
  colorHost,
  objectSettingsHost,
  numericDialogHost,
  commandEngine,
  openColorDialog,
  isEditingRustDocument,
  parseEngineJson,
  svgPointFromEvent,
  selectClickTarget,
  renderSelectionOnlyUpdate,
  syncDocumentFromEngine,
  renderDocument,
  renderDocumentChange,
  runEditorCommand,
  applySelectionColor,
  applyArrowOptionsToSelection,
  currentSelectionInfo,
  selectedSceneObjects,
  cssColorToHex,
  openTextEditorAt,
  renderEditorOverlay,
  refreshCommandAvailability,
  confirmRepeatUnitUngroup: confirmRepeatUnitUngroupIfNeeded,
});
canvasContextMenu = canvasContextMenuHost.canvasContextMenu;
document.body.appendChild(canvasContextMenu);
const updateCanvasContextMenuAvailability = (...args) => canvasContextMenuHost.updateCanvasContextMenuAvailability(...args);
const closeCanvasContextMenu = (...args) => canvasContextMenuHost.closeCanvasContextMenu(...args);
const openCanvasContextMenu = (...args) => canvasContextMenuHost.openCanvasContextMenu(...args);

const editorPointerController = createEditorPointerController({
  state: () => state,
  editorState: () => editorState,
  viewerSvg: () => viewerSvg,
  svgPointFromEvent,
  parseEngineJson,
  pointDistance,
  cssPxToPt,
  screenPxToWorld,
  routeEditorPointerEvents,
  isEditingRustDocument,
  openTextEditorAt,
  closeActiveTextEditorForToolAction,
  syncSelectCursorForPoint,
  syncArrowAwareCursorForPoint,
  syncDocumentFromEngine,
  renderDocument,
  renderDocumentChange,
  renderDocumentPrimitiveChange,
  ensureDocumentObjectDomForCommandResult,
  renderSelectionOnlyUpdate,
  selectionResizeHandleHit,
  selectionRotateHandleHit,
  currentSelectionRotateHandle,
  selectionResizeGestureScale,
  selectionRotateAngleForGesture,
  currentRenderBounds,
  currentEditorRenderList,
  currentEditorEngineState,
  currentEditorInteractionRenderList,
  currentEditorOverlayRenderList,
  renderEditorOverlay,
  invalidateEditorEngineReadCache,
  selectionBoundsContainsPoint: currentSelectionBoundsContainsPoint,
  selectionHitContainsPoint: currentSelectionHitContainsPoint,
  selectedContentHitContainsPoint: currentSelectedContentHitContainsPoint,
  documentBoundsContainsPoint: currentDocumentBoundsContainsPoint,
  selectionNeedsBackendMovePreview,
  applyBackendSelectionMovePreview,
  applyDocumentObjectPreviewTransform,
  hideDocumentDiagnosticsForPreview,
  clearDocumentObjectPreviewTransform,
  commitDocumentObjectPreviewTransform,
  canCommitDocumentObjectPreviewTransform,
  scheduleDeferredDocumentSync,
  syncEditorRenderListFromEngine,
  updateTlcSpotHover,
  clearTlcHoverState,
  documentHasTlcPlate: currentDocumentHasTlcPlate,
  maybeAutoExpandEditorViewport,
  positionActiveTextEditor,
  selectClickTarget,
  cursorForShapeAction,
  syncCanvasCursor,
  setCanvasCursorStyle,
  hoverPointerMoveDelayMs: (tool) => (
    tool === "select"
    && (viewerSvg?.querySelector('[data-layer="document-content"]')?.childElementCount || 0) > 1000
      ? 60
      : 0
  ),
  awaitPendingToolActivation: () => activeToolActivationPromise,
  renderDragCapturePreview: renderCanvasDragPreview,
  clearDragCapturePreview: clearCanvasDragPreview,
  setCanvasPointerShieldActive: (active, options = {}) => {
    const enabled = Boolean(active);
    canvasPointerShieldActive = enabled;
    canvasPointerShield.classList.toggle("is-active", enabled);
    syncViewerSvgPointerEventMode();
    if (!enabled && options.clearPreview !== false) {
      return clearCanvasDragPreview();
    }
    return false;
  },
  commandEngine,
  bracketLabelAnchorPoint,
  bracketLabelTextOptions: () => ({
    fontSize: BRACKET_LABEL_FONT_SIZE,
    lineHeight: BRACKET_LABEL_LINE_HEIGHT,
  }),
  angleBetweenPoints: (from, to) => {
    const raw = Math.atan2(to.y - from.y, to.x - from.x) * 180 / Math.PI;
    return ((raw % 360) + 360) % 360;
  },
  activeSelectionGesture: () => activeSelectionGesture,
  setActiveSelectionGesture: (value) => { activeSelectionGesture = value; },
  setActiveTlcSpotHover: (value) => { activeTlcSpotHover = value; },
  setActiveTlcLaneHover: (value) => { activeTlcLaneHover = value; },
  setLastEditFocusPoint: (value) => { state.lastEditFocusPoint = value; },
  noteEditorPointerActivity: () => { lastEditorPointerActivityAt = performance.now(); },
  activeBracketDragStart: () => state.activeBracketDragStart,
  setActiveBracketDragStart: (value) => { state.activeBracketDragStart = value; },
});

async function syncDocumentFromEngine(options = {}) {
  if (!state.editorEngine) {
    return;
  }
  const syncRenderList = options.syncRenderList ?? true;
  const refreshSnapshot = options.refreshSnapshot ?? true;
  if (!syncRenderList && refreshSnapshot && typeof state.editorEngine.refreshSnapshot === "function") {
    await state.editorEngine.refreshSnapshot("documentState");
  }
  const documentData = parseEngineJson(state.editorEngine.documentJson());
  if (documentData) {
    state.currentDocument = documentData;
    resetDocumentRenderState();
    currentDocumentMoleculeTopology();
    if (syncRenderList) {
      await syncCoreRenderListFromCurrentDocument();
      maybeAutoExpandEditorViewport(state.coreRenderList || []);
    }
  }
  syncSelectionChemistrySummary();
  refreshCommandAvailability();
}

function scheduleDeferredDocumentSync() {
  if (deferredDocumentSyncHandle) {
    return;
  }
  deferredDocumentSyncHandle = window.setTimeout(async () => {
    deferredDocumentSyncHandle = 0;
    const recentlyActive = performance.now() - lastEditorPointerActivityAt < 2500;
    if (activeSelectionGesture || activeTextEditor || recentlyActive) {
      scheduleDeferredDocumentSync();
      return;
    }
    await syncDocumentFromEngine();
    saveActiveDocumentTabState();
    renderDocumentTabs();
    syncWindowTitle();
  }, 3000);
}

async function renderSelectionOnlyUpdate(point, syncCursor = syncSelectCursorForPoint, options = {}) {
  const renderList = options.useInteractionList === false
    ? currentEditorRenderList()
    : syncEditorSelectionRenderListFromEngine();
  renderEditorOverlay(renderList);
  if (point && typeof syncCursor === "function") {
    await syncCursor(point);
  }
  if (options.deferEngineReads) {
    return;
  }
  syncSelectionChemistrySummary();
  refreshCommandAvailability();
}

async function selectClickTarget(point, additive = false) {
  if (!additive && !currentDocumentBoundsContainsPoint(point, DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX)) {
    await state.editorEngine.clearSelection?.();
    invalidateEditorEngineReadCache();
    return;
  }
  await state.editorEngine.selectAtPoint(point.x, point.y, additive);
}

function formatSelectionSummaryMass(value, digits) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric.toFixed(digits) : "";
}

function clampMassDigits(value) {
  const numeric = Math.trunc(Number(value));
  if (!Number.isFinite(numeric)) {
    return 2;
  }
  return Math.max(0, Math.min(8, numeric));
}

function setMassDigits(value) {
  const next = clampMassDigits(value);
  if (editorState.massDigits === next) {
    return;
  }
  editorState.massDigits = next;
  syncSelectionChemistrySummary();
}

function massPrecisionIcon(direction) {
  const path = direction === "up"
    ? "M6 9 12 3l6 6"
    : "M6 5l6 6 6-6";
  return `<svg viewBox="0 0 24 14" aria-hidden="true"><path d="${path}"/></svg>`;
}

function makeMassPrecisionControl() {
  const control = document.createElement("span");
  control.className = "selection-mass-precision";
  const value = document.createElement("span");
  value.className = "selection-mass-precision-value";
  value.textContent = String(clampMassDigits(editorState.massDigits));
  const buttons = document.createElement("span");
  buttons.className = "selection-mass-precision-buttons";
  const increase = document.createElement("button");
  increase.className = "selection-mass-precision-button";
  increase.type = "button";
  increase.title = "Increase mass decimals";
  increase.setAttribute("aria-label", "Increase mass decimals");
  increase.innerHTML = massPrecisionIcon("up");
  increase.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    setMassDigits(editorState.massDigits + 1);
  });
  const decrease = document.createElement("button");
  decrease.className = "selection-mass-precision-button";
  decrease.type = "button";
  decrease.title = "Decrease mass decimals";
  decrease.setAttribute("aria-label", "Decrease mass decimals");
  decrease.innerHTML = massPrecisionIcon("down");
  decrease.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    setMassDigits(editorState.massDigits - 1);
  });
  buttons.append(increase, decrease);
  control.append(value, buttons);
  return control;
}

function makeSelectionSummaryItem(label, value, accessory = null) {
  const item = document.createElement("span");
  item.className = "selection-chemistry-summary-item";
  const labelNode = document.createElement("span");
  labelNode.className = "selection-chemistry-summary-label";
  labelNode.textContent = label;
  const valueNode = document.createElement("span");
  valueNode.className = "selection-chemistry-summary-value";
  valueNode.textContent = value;
  item.append(labelNode, valueNode);
  if (accessory) {
    item.append(accessory);
  }
  return item;
}

function appendFormulaText(container, formula) {
  const parts = String(formula || "").match(/[A-Z][a-z]?|\d+/g) || [];
  for (const part of parts) {
    const node = document.createElement(/^\d+$/.test(part) ? "sub" : "span");
    node.textContent = part;
    container.append(node);
  }
}

function syncSelectionChemistrySummary() {
  if (!selectionChemistrySummary) {
    return;
  }
  selectionChemistrySummary.replaceChildren();
  const summary = isEditingRustDocument()
    ? parseEngineJson(state.editorEngine?.selectionChemistrySummaryJson?.(), null)
    : null;
  if (!summary?.formula) {
    return;
  }
  const formula = document.createElement("span");
  formula.className = "selection-chemistry-summary-item selection-chemistry-summary-formula";
  appendFormulaText(formula, summary.formula);
  editorState.massDigits = clampMassDigits(editorState.massDigits);
  const formulaWeight = formatSelectionSummaryMass(summary.formulaWeight, editorState.massDigits);
  const exactMass = formatSelectionSummaryMass(summary.exactMass, editorState.massDigits);
  selectionChemistrySummary.append(formula);
  selectionChemistrySummary.append(makeMassPrecisionControl());
  if (formulaWeight) {
    selectionChemistrySummary.append(makeSelectionSummaryItem("Formula Weight", formulaWeight));
  }
  if (exactMass) {
    selectionChemistrySummary.append(makeSelectionSummaryItem("Exact Mass", exactMass));
  }
}

async function resetEditorEngine() {
  await finishActiveTextEditor(false);
  await state.editorEngine?.free?.();
  state.editorEngine = engineHost.createEngineSession();
  await state.editorEngine.ready?.();
  syncTextSymbolPaletteFromEngine();
  commandEngine.resetRevision();
  state.runtimeViewBox = defaultEditorViewBox();
  state.lastEditFocusPoint = null;
  clearZoomHandoffs();
  state.currentFileName = null;
  state.currentFilePath = null;
  state.savedDocumentJson = null;
  state.savedRevision = null;
  await syncEngineToolState();
  await syncDocumentFromEngine();
  renderSecondaryToolbar();
  markCurrentDocumentSaved();
}

async function resetDocumentEngine() {
  await state.documentEngine?.free?.();
  state.documentEngine = engineHost.createEngineSession();
  await state.documentEngine.ready?.();
}

function refreshCommandAvailability() {
  const undoButton = document.querySelector('[data-command="undo"]');
  const redoButton = document.querySelector('[data-command="redo"]');
  const saveButtons = document.querySelectorAll('[data-command="save"]');
  if (undoButton) {
    undoButton.disabled = !state.editorEngine?.canUndo?.();
  }
  if (redoButton) {
    redoButton.disabled = !state.editorEngine?.canRedo?.();
  }
  for (const saveButton of saveButtons) {
    saveButton.disabled = !canSaveCurrentDocument();
  }
  void updateCanvasContextMenuAvailability();
}

function uniformValue(values) {
  const normalized = values.filter((value) => value != null && value !== "");
  if (!normalized.length) {
    return null;
  }
  return normalized.every((value) => value === normalized[0]) ? normalized[0] : null;
}

async function activateEditorTool(nextTool) {
  const activation = activateEditorToolNow(nextTool);
  activeToolActivationPromise = activation.catch(() => {});
  return activation;
}

async function activateEditorToolNow(nextTool) {
  if (!nextTool) {
    return false;
  }
  canvasPointerShieldActive = false;
  canvasPointerShield.classList.remove("is-active");
  syncViewerSvgPointerEventMode();
  clearCanvasDragPreview();
  if (editorState.activeTool === nextTool && !editorState.elementPlacementActive) {
    return false;
  }
  const previousTool = editorState.activeTool;
  activeSelectionGesture = null;
  editorState.elementPlacementActive = false;
  editorState.activeTool = nextTool;
  if (nextTool !== "delete") {
    editorState.secondaryToolbarTool = nextTool;
  }
  syncViewerSvgPointerEventMode();
  if (activeTextEditor && nextTool !== "element") {
    await closeActiveTextEditorForToolAction();
  }
  if (previousTool === "select" && nextTool !== "select") {
    activeSelectionGesture = null;
    await state.editorEngine?.clearSelection?.();
    invalidateEditorEngineReadCache();
  }
  if (nextTool !== "bracket") {
    state.activeBracketDragStart = null;
  }
  await state.editorEngine?.clearInteraction?.();
  document.querySelectorAll("[data-tool]").forEach((button) => {
    button.classList.toggle("is-active", button.dataset.tool === editorState.activeTool);
  });
  await syncEngineToolState();
  syncViewerSvgPointerEventMode();
  invalidateEditorEngineReadCache();
  renderSecondaryToolbar();
  syncCanvasCursor();
  if (isEditingRustDocument()) {
    renderEditorOverlay(currentEditorRenderList());
  }
  return true;
}

function setCanvasCursorStyle(cursor) {
  const value = cursor || "default";
  if (viewerSvg) {
    viewerSvg.style.cursor = value;
  }
  if (viewerContainer) {
    viewerContainer.style.cursor = value;
  }
  canvasPointerShield.style.cursor = value;
}

function syncCanvasCursor() {
  if (!viewerSvg) {
    return;
  }
  if (activeSelectionGesture?.kind === "resize") {
    setCanvasCursorStyle(activeSelectionGesture.cursor || "default");
    return;
  }
  if (activeSelectionGesture?.kind === "move") {
    setCanvasCursorStyle(activeSelectionGesture.cursor || "grabbing");
    return;
  }
  if (activeSelectionGesture?.kind === "rotate") {
    setCanvasCursorStyle("grabbing");
    return;
  }
  setCanvasCursorStyle(editorState.elementPlacementActive
    ? elementCursor(editorState.elementSymbol)
    : editorState.activeTool === "text"
    ? "text"
    : editorState.activeTool === "delete"
      ? DELETE_CURSOR
    : editorState.activeTool === "select"
      ? "default"
    : editorState.activeTool === "arrow"
      ? "crosshair"
      : "crosshair");
}

async function syncSelectCursorForPoint(point) {
  if (editorState.elementPlacementActive) {
    syncCanvasCursor();
    return;
  }
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

function activeToolCanDragSelection() {
  return editorState.activeTool === "select";
}

async function syncArrowAwareCursorForPoint(point) {
  if (!viewerSvg || !isEditingRustDocument()) {
    syncCanvasCursor();
    return;
  }
  if (editorState.elementPlacementActive) {
    syncCanvasCursor();
    return;
  }
  if (activeSelectionGesture?.kind === "tlc-spot-drag") {
    setCanvasCursorStyle("grabbing");
    return;
  }
  if (activeSelectionGesture?.kind === "move") {
    setCanvasCursorStyle(activeSelectionGesture.cursor || "grabbing");
    return;
  }
  if (activeSelectionGesture?.kind === "rotate") {
    setCanvasCursorStyle("grabbing");
    return;
  }
  if (activeSelectionGesture?.kind === "resize") {
    setCanvasCursorStyle(activeSelectionGesture.cursor || "default");
    return;
  }
  if (activeSelectionGesture?.kind === "arrow-endpoint") {
    setCanvasCursorStyle("move");
    return;
  }
  if (activeSelectionGesture?.kind === "arrow-curve") {
    setCanvasCursorStyle("nesw-resize");
    return;
  }
  if (activeSelectionGesture?.kind === "shape-resize") {
    setCanvasCursorStyle(activeSelectionGesture.cursor || "nwse-resize");
    return;
  }
  if ((editorState.activeTool === "select" || editorState.activeTool === "tlc-plate") && activeTlcSpotHover) {
    setCanvasCursorStyle("grab");
    return;
  }
  const overSelectionBox = currentSelectionHitContainsPoint(point)
    || currentSelectionBoundsContainsPoint(point);
  if (
    activeToolCanDragSelection()
    && overSelectionBox
    && !currentSelectionHandleZoneContainsPoint(point)
  ) {
    setCanvasCursorStyle("grab");
    return;
  }
  if (editorState.activeTool === "select") {
    const resizeHandle = selectionResizeHandleHit(point);
    if (resizeHandle) {
      setCanvasCursorStyle(resizeHandle.cursor);
      return;
    }
  }
  if (editorState.activeTool === "select" && selectionRotateHandleHit(point)) {
    setCanvasCursorStyle("grab");
    return;
  }
  if (activeToolCanDragSelection() && overSelectionBox) {
    setCanvasCursorStyle("grab");
    return;
  }
  const overSelection = overSelectionBox;
  if ((editorState.activeTool === "select"
    || editorState.activeTool === "bond"
    || editorState.activeTool === "arrow"
    || editorState.activeTool === "bracket"
    || editorState.activeTool === "symbol"
    || editorState.activeTool === "element"
    || editorState.activeTool === "text"
    || editorState.activeTool === "shape"
    || editorState.activeTool === "tlc-plate"
    || editorState.activeTool === "orbital"
    || editorState.activeTool === "templates"
    || editorState.activeTool === "chain")
    && overSelection) {
    setCanvasCursorStyle("grab");
    return;
  }
  if (editorState.activeTool === "bracket"
    || editorState.activeTool === "shape"
    || editorState.activeTool === "tlc-plate"
    || editorState.activeTool === "orbital") {
    const shapeAction = await state.editorEngine.hoverShapeAction?.(point.x, point.y) || "";
    const shapeCursor = cursorForShapeAction(shapeAction);
    if (shapeCursor) {
      setCanvasCursorStyle(shapeCursor);
      return;
    }
  }
  if (editorState.activeTool === "arrow") {
    const arrowAction = await state.editorEngine.hoverArrowAction?.(point.x, point.y) || "";
    if (arrowAction === "head" || arrowAction === "tail") {
      setCanvasCursorStyle("move");
      return;
    }
    if (arrowAction === "head-style" || arrowAction === "tail-style") {
      setCanvasCursorStyle("nwse-resize");
      return;
    }
    if (arrowAction === "curve") {
      setCanvasCursorStyle("nesw-resize");
      return;
    }
  }
  if (editorState.activeTool === "arrow") {
    setCanvasCursorStyle("crosshair");
    return;
  }
  if (editorState.activeTool === "shape" || editorState.activeTool === "tlc-plate" || editorState.activeTool === "orbital") {
    setCanvasCursorStyle("crosshair");
    return;
  }
  if (editorState.activeTool === "chain") {
    setCanvasCursorStyle("crosshair");
    return;
  }
  setCanvasCursorStyle(overSelection ? "grab" : "default");
}

const editorToolbarHost = createEditorToolbarHost({
  state,
  editorState,
  secondaryToolbar,
  parseEngineJson,
  insertTextSymbol,
  selectElementFromQuickPalette,
  handleQuickPaletteModeChange,
});

function renderSecondaryToolbar(...args) { return editorToolbarHost.renderSecondaryToolbar(...args); }
function currentDocumentColors(...args) { return editorToolbarHost.currentDocumentColors(...args); }
function currentToolbarColorPalette(...args) { return editorToolbarHost.currentToolbarColorPalette(...args); }
function currentElementPalette(...args) { return editorToolbarHost.currentElementPalette(...args); }
function syncTextSymbolPaletteFromEngine(...args) { return editorToolbarHost.syncTextSymbolPaletteFromEngine(...args); }
function ensureTextSymbolPalette(...args) { return editorToolbarHost.ensureTextSymbolPalette(...args); }
function syncEditorPrimaryToolButtons(...args) { return editorToolbarHost.syncPrimaryToolButtons(...args); }
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

async function openTextEditorAt(point, options = {}) {
  await finishActiveTextEditor(true);
  const session = options.bracketObjectId
    ? {
        target: {
          kind: "text-object",
          objectId: null,
          x: point.x,
          y: point.y,
        },
        text: "",
        sourceRuns: [],
        fontFamily: "Arial",
        fontSize: Number(options.fontSize) > 0 ? Number(options.fontSize) : DEFAULT_TEXT_FONT_SIZE,
        fill: "#000000",
        align: "left",
        lineHeight: Number(options.lineHeight) > 0
          ? Number(options.lineHeight)
          : DEFAULT_TEXT_FONT_SIZE * 1.2,
        box: [
          0,
          0,
          cssPxToPt(8),
          Number(options.lineHeight) > 0 ? Number(options.lineHeight) : DEFAULT_TEXT_FONT_SIZE * 1.2,
        ],
        preserveLines: true,
        defaultChemical: false,
      }
    : parseEngineJson(await state.editorEngine?.beginTextEdit?.(point.x, point.y), null);
  if (!session) {
    renderEditorOverlay(currentEditorInteractionRenderList());
    return;
  }
  const nextSession = { ...session };
  const fontSize = Number(options.fontSize);
  if (Number.isFinite(fontSize) && fontSize > 0) {
    nextSession.fontSize = fontSize;
  }
  const lineHeight = Number(options.lineHeight);
  if (Number.isFinite(lineHeight) && lineHeight > 0) {
    nextSession.lineHeight = lineHeight;
  }
  renderEditorOverlay(currentEditorInteractionRenderList());
  openTextEditorSession(nextSession);
  if (options.bracketObjectId && activeTextEditor) {
    activeTextEditor.bracketLabelObjectId = String(options.bracketObjectId);
  }
  if (state.pendingTextSymbol) {
    const symbol = state.pendingTextSymbol;
    state.pendingTextSymbol = null;
    textEditorController.insertTextAtSelection(symbol);
    focusActiveTextEditor();
  }
}

function openTextEditorSession(session) {
  textEditorController.openTextEditorSession(engineSessionToEditorSession(session));
  const targetResult = commandResultForTextEditorTarget(activeTextEditor?.session?.target);
  if (targetResult) {
    renderDocumentChange(targetResult);
  } else {
    renderEditorOverlay(currentEditorInteractionRenderList());
  }
}

function commandResultForTextEditorTarget(target) {
  if (!target) {
    return null;
  }
  if (target.kind === "text-object") {
    const objectId = target.objectId || target.object_id || null;
    return objectId ? { changed: true, targets: { objects: [objectId] } } : null;
  }
  if (target.kind === "endpoint-label") {
    const nodeId = target.nodeId || target.node_id || null;
    return nodeId ? { changed: true, targets: { nodes: [nodeId] } } : null;
  }
  return null;
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
  const activeTextTargetResult = commandResultForTextEditorTarget(session?.target);
  input?.blur?.();
  const selection = window.getSelection?.();
  selection?.removeAllRanges?.();
  const nextSession = buildCommittedTextSession(session, root);
  const bracketLabelObjectId = activeTextEditor?.bracketLabelObjectId || null;
  textEditorLayer.replaceChildren();
  activeTextEditor = null;
  if (!commit) {
    if (activeTextTargetResult) {
      renderDocumentChange(activeTextTargetResult);
    } else {
      renderEditorOverlay(currentEditorRenderList());
    }
    return false;
  }
  const engineSessionJson = JSON.stringify(editorSessionToEngineSession(nextSession));
  const result = await commandEngine.executeEngineCommand(
    {
      type: bracketLabelObjectId ? "apply-bracket-label-text" : "apply-text-edit",
      payload: {
        target: nextSession.target || null,
        bracketObjectId: bracketLabelObjectId,
      },
    },
    () => bracketLabelObjectId
      ? state.editorEngine?.applyBracketLabelText?.(bracketLabelObjectId, engineSessionJson)
      : state.editorEngine?.applyTextEdit?.(engineSessionJson),
  );
  renderDocumentChange(result);
  return Boolean(result.changed);
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
  const result = await commandEngine.executeEngineCommand(
    {
      type: "apply-selection-color",
      payload: { color: normalized },
    },
    () => state.editorEngine.applyColorToSelection(normalized),
  );
  const changed = !!result.changed;
  if (!changed) {
    return false;
  }
  renderDocumentChange(result);
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
  editorState.secondaryToolbarTool = "text";
  void syncEngineToolState();
  renderSecondaryToolbar();
  syncCanvasCursor();
}

function insertElementSymbol(symbol) {
  const text = String(symbol || "");
  if (!text) {
    return;
  }
  if (activeTextEditor) {
    textEditorController.insertTextAtSelection(text);
    focusActiveTextEditor();
  }
}

function setElementPlacementActive(active) {
  const nextActive = Boolean(active) && !activeTextEditor;
  canvasPointerShieldActive = false;
  canvasPointerShield.classList.remove("is-active");
  clearCanvasDragPreview();
  syncViewerSvgPointerEventMode();
  if (editorState.elementPlacementActive === nextActive) {
    return;
  }
  editorState.elementPlacementActive = nextActive;
  syncEditorPrimaryToolButtons();
  syncCanvasCursor();
}

function handleQuickPaletteModeChange({ open, mode, keepElementPlacement = false } = {}) {
  setElementPlacementActive((open && mode === "element") || keepElementPlacement);
  void syncEngineToolState();
}

async function selectElementFromQuickPalette(symbol, atomicNumber = null) {
  const normalizedSymbol = String(symbol || "");
  if (!normalizedSymbol) {
    return false;
  }
  const nextAtomicNumber = Number(atomicNumber) || editorState.elementAtomicNumber || 15;
  const changed = editorState.elementSymbol !== normalizedSymbol
    || editorState.elementAtomicNumber !== nextAtomicNumber;
  editorState.elementSymbol = normalizedSymbol;
  editorState.elementAtomicNumber = nextAtomicNumber;
  if (activeTextEditor) {
    insertElementSymbol(normalizedSymbol);
  } else {
    setElementPlacementActive(true);
  }
  await syncEngineToolState();
  renderSecondaryToolbar();
  syncCanvasCursor();
  focusActiveTextEditor();
  return changed;
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
  renderSecondaryToolbar,
  resetEditorEngine,
  pageViewBox,
  defaultEditorViewBox,
  renderDocument,
  fitView,
  markCurrentDocumentSaved,
  currentDocumentIsDirty,
  markCurrentDocumentOfficeSynced,
  traceEvent: (event, detail = null) => desktopFileHost?.traceEvent?.(event, detail),
  resetCommandEngineRevision: () => commandEngine.resetRevision(),
  refreshCommandAvailability,
  waitForRuntimeReady: () => appRuntimeReady,
});

const {
  isAbortError,
  loadAndRender,
  loadCdxDocumentIntoEditor,
  loadSdfDocumentIntoEditor,
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

const browserDocumentTabs = createBrowserDocumentTabs({
  state,
  documentTabs,
  desktopFileHost,
  openFileInput,
  isDesktopShell: () => isDesktopShell,
  appRuntimeReady: () => appRuntimeReady,
  getActiveDocumentTabId: () => activeDocumentTabId,
  setActiveDocumentTabId: (value) => { activeDocumentTabId = value; },
  finishActiveTextEditor,
  syncDocumentFromEngine,
  saveActiveDocumentTabState,
  documentTitleFromState,
  closeDocumentTab,
  loadJsonDocumentIntoEditor,
  currentDocumentRevision,
  refreshCommandAvailability,
  setZoomPercent,
  renderDocumentTabs,
  loadCdxDocumentIntoEditor,
  openDocumentText,
  activeDocumentTabIsBlankUntitled,
  currentDocumentIsDirty,
  createDocumentTab,
  restoreDocumentTabState,
  resetEditorEngine,
  renderDocument,
  fitView,
  documentTabForFilePath,
  activateDocumentTab,
  activeDocumentTab,
  fileNameFromPath,
  openDocumentPath,
  openDocumentFile,
});

function detachDocumentTab(...args) { return browserDocumentTabs.detachDocumentTab(...args); }
function loadDetachedDocumentPayload(...args) { return browserDocumentTabs.loadDetachedDocumentPayload(...args); }
function takeBrowserPendingDocument(...args) { return browserDocumentTabs.takeBrowserPendingDocument(...args); }
function loadBrowserPendingDocumentPayload(...args) { return browserDocumentTabs.loadBrowserPendingDocumentPayload(...args); }
function newDocumentTab(...args) { return browserDocumentTabs.newDocumentTab(...args); }
function openDocumentPathInTab(...args) { return browserDocumentTabs.openDocumentPathInTab(...args); }
function openDocumentFileInTab(...args) { return browserDocumentTabs.openDocumentFileInTab(...args); }
function openDroppedDocumentFileInTab(...args) { return browserDocumentTabs.openDroppedDocumentFileInTab(...args); }
function openDroppedDocumentFilesInTabs(...args) { return browserDocumentTabs.openDroppedDocumentFilesInTabs(...args); }
function chooseAndOpenDocumentTab(...args) { return browserDocumentTabs.chooseAndOpenDocumentTab(...args); }
function confirmApplyDocumentStylePreset(...args) { return browserDocumentTabs.confirmApplyDocumentStylePreset(...args); }

ensureTextSymbolPalette();

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
  openDroppedDocumentFileInTab,
  openDroppedDocumentFilesInTabs,
  getZoomPercent,
  setTextFontSize: (size) => {
    const fontSize = normalizeToolbarFontSize(Math.max(5, Math.min(288, size)));
    editorState.textFontSize = ptToCssPx(fontSize);
  },
  isEditingRustDocument,
  syncEngineToolState,
  syncDocumentFromEngine,
  renderDocument,
  renderDocumentChange,
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
  activateEditorTool,
  runEditorCommand,
  commandEngine,
  setZoomPercent,
  nextZoomStep,
  fitView,
  resetEditorEngine,
  openDocumentFile,
  viewerContainer,
  focusActiveTextEditor,
  applyTextAlignment,
  applyTextFormatCommand,
  applyTextScript,
  applyChemicalFormat,
  insertElementSymbol,
  applyTextInlineStyle,
  applySelectionArrangeCommand,
  applyArrowOptionsToSelection,
  applySelectionColor,
  getDocumentColors: currentDocumentColors,
});

renderSecondaryToolbar();
syncCanvasCursor();
syncViewerSvgPointerEventMode();
bindBrowserBeforeUnloadGuard();

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
    && (editorState.elementPlacementActive
      || editorState.activeTool === "bond"
      || editorState.activeTool === "delete"
      || editorState.activeTool === "arrow"
      || editorState.activeTool === "bracket"
      || editorState.activeTool === "symbol"
      || editorState.activeTool === "element"
      || editorState.activeTool === "select"
      || editorState.activeTool === "text"
      || editorState.activeTool === "shape"
      || editorState.activeTool === "tlc-plate"
      || editorState.activeTool === "orbital"
      || editorState.activeTool === "templates"
      || editorState.activeTool === "chain");
}

function activeToolUsesContainerPointerEvents() {
  return editorState.activeTool === "bond"
    || editorState.activeTool === "arrow"
    || editorState.activeTool === "bracket"
    || editorState.activeTool === "symbol"
    || editorState.activeTool === "element"
    || editorState.activeTool === "select"
    || editorState.activeTool === "shape"
    || editorState.activeTool === "tlc-plate"
    || editorState.activeTool === "orbital"
    || editorState.activeTool === "templates"
    || editorState.activeTool === "chain";
}

function syncViewerSvgPointerEventMode() {
  viewerSvg?.classList.toggle(
    "is-pointer-capture-disabled",
    canvasPointerShieldActive || activeToolUsesContainerPointerEvents(),
  );
}

function screenPxToWorld(px) {
  return px / Math.max(1, viewportScale());
}

async function applySelectionArrangeCommand(command) {
  if (!isEditingRustDocument() || editorState.activeTool !== "select") {
    return false;
  }
  const result = await commandEngine.executeEngineCommand(
    {
      type: "apply-selection-arrange",
      payload: { command },
    },
    () => state.editorEngine.applySelectionArrangeCommand?.(command),
  );
  const changed = !!result.changed;
  if (!changed) {
    return false;
  }
  renderDocumentChange(result);
  return true;
}

async function applyArrowOptionsToSelection() {
  if (!isEditingRustDocument()) {
    return false;
  }
  const result = await commandEngine.executeEngineCommand(
    {
      type: "apply-arrow-style",
      payload: {
        changes: {
          variant: editorState.arrowType,
          headSize: editorState.arrowHeadSize,
          curve: editorState.arrowCurve,
          headStyle: editorState.arrowHeadStyle,
          tailStyle: editorState.arrowTailStyle,
          noGo: editorState.arrowNoGo,
          bold: editorState.arrowBold,
        },
      },
    },
    () => state.editorEngine.applyArrowEndpointOptionsToSelection
      ? state.editorEngine.applyArrowEndpointOptionsToSelection(
        editorState.arrowType,
        editorState.arrowHeadSize,
        editorState.arrowCurve,
        editorState.arrowHeadStyle,
        editorState.arrowTailStyle,
        editorState.arrowNoGo,
        editorState.arrowBold,
      )
      : state.editorEngine.applyArrowOptionsToSelection?.(
        editorState.arrowType,
        editorState.arrowHeadSize,
        editorState.arrowHead,
        editorState.arrowTail,
        editorState.arrowBold,
      ),
  );
  const changed = !!result.changed;
  if (changed) {
    renderDocumentChange(result);
  }
  return changed;
}

function bracketLabelAnchorPoint(start, end) {
  const right = Math.max(start.x, end.x);
  const bottom = Math.max(start.y, end.y);
  return {
    x: right + BRACKET_LABEL_OFFSET_X,
    y: bottom + BRACKET_LABEL_OFFSET_Y,
  };
}

function handleViewerContainerPointerEvent(handler) {
  return (event) => {
    if (event.target === viewerSvg || viewerSvg?.contains?.(event.target)) {
      return;
    }
    void handler(event);
  };
}

viewerSvg?.addEventListener("pointermove", editorPointerController.handleEditorPointerMove);
viewerSvg?.addEventListener("pointerdown", editorPointerController.handleEditorPointerDown);
viewerSvg?.addEventListener("pointerup", editorPointerController.handleEditorPointerUp);
viewerContainer?.addEventListener("pointermove", handleViewerContainerPointerEvent(editorPointerController.handleEditorPointerMove));
viewerContainer?.addEventListener("pointerdown", handleViewerContainerPointerEvent(editorPointerController.handleEditorPointerDown));
viewerContainer?.addEventListener("pointerup", handleViewerContainerPointerEvent(editorPointerController.handleEditorPointerUp));
viewerContainer?.addEventListener("pointercancel", handleViewerContainerPointerEvent(editorPointerController.handleEditorPointerCancel));
canvasPointerShield.addEventListener("pointermove", editorPointerController.handleEditorPointerMove);
canvasPointerShield.addEventListener("pointerup", editorPointerController.handleEditorPointerUp);
canvasPointerShield.addEventListener("pointercancel", editorPointerController.handleEditorPointerCancel);
viewerSvg?.addEventListener("dblclick", editorPointerController.handleEditorDoubleClick);
viewerSvg?.addEventListener("pointercancel", async () => {
  await editorPointerController.handleEditorPointerCancel();
});
window.addEventListener("pointerup", () => {
  queueMicrotask(() => {
    if (!activeSelectionGesture) {
      clearDocumentObjectPreviewTransform();
    }
  });
});
viewerSvg?.addEventListener("pointerleave", editorPointerController.handleEditorPointerLeave);
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

function renderDocument() {
  const documentData = state.currentDocument;
  if (!documentData) {
    return;
  }
  if (window.__chemcoreDebug?.renderStats) {
    window.__chemcoreDebug.renderStats.documentRenderCount += 1;
  }

  const page = documentData.document.page;
  const viewBox = activeViewBox();
  viewerSvg.innerHTML = "";
  resetDocumentRenderState();
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
  const documentLayer = makeSvgNode("g", {
    "data-layer": "document-content",
    "pointer-events": "none",
  });
  viewerSvg.appendChild(documentLayer);

  if (!sceneRenderer.renderCorePrimitiveList(documentLayer, documentData)) {
    const visibleObjects = sceneRenderer.buildRenderList(documentData);

    for (const object of visibleObjects) {
      sceneRenderer.renderSceneObject(documentLayer, object, documentData);
    }
  }
  rebuildDocumentPrimitiveIndex(documentLayer);

  syncViewerStats();
  renderEditorOverlay();
  positionActiveTextEditor();
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
  const browserPendingDocument = await takeBrowserPendingDocument();
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
  resolveAppInitialDocumentReady?.();
} catch (error) {
  resolveAppInitialDocumentReady?.();
  viewerTitle.textContent = "Runtime load failed";
  viewerStats.textContent = "";
  docMeta.textContent = String(error);
  viewerSvg.innerHTML = "";
}
