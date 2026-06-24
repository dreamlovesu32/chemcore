import {
  parseEngineJson,
  primitivesForObject,
  renderBoundsFromEngine,
  renderListFromEngine,
} from "./engine_bridge.js";
import { createAppDomRefs } from "./app_dom.js";
import { registerChemcoreDebug } from "./app_debug.js";
import { base64ToBytes, bytesToBase64 } from "./binary_helpers.js";
import { createColorHost } from "./color_host.js";
import { createObjectSettingsHost } from "./object_settings_host.js";
import { createNumericDialogHost } from "./numeric_dialog_host.js";
import { createDesktopFileHost, normalizeDesktopPath } from "./desktop_file_host.js";
import { createEngineHost } from "./engine_host.js";
import { bindEditorControls, openColorDialog } from "./editor_bindings.js?v=20260624-drop-hardened";
import { createDocumentFlow } from "./document_flow.js";
import {
  chemcoreOpenAcceptTypes,
  decompressChemcoreText,
  looksLikeCdxFile,
  looksLikeCdxmlFile,
  looksLikeCompressedChemcoreFile,
  looksLikeSdfFile,
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
  ARROW_TOOL_ICON_TYPES,
  BOND_TOOL_ICON_TYPES,
  ORBITAL_TOOL_ICON_PHASES,
  ORBITAL_TOOL_ICON_STYLES,
  ORBITAL_TOOL_ICON_TEMPLATES,
  SHAPE_TOOL_ICON_KINDS,
  SHAPE_TOOL_ICON_STYLES,
  SHAPE_TOOL_STYLE_KINDS,
  SYMBOL_TOOL_ICON_TYPES,
  TEXT_FORMAT_ICON_TYPES,
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
import { createSceneRenderer } from "./scene_renderer.js";
import { createEditorOverlayRenderer } from "./editor_overlay.js";
import { createEditorSelectionState } from "./editor_selection_state.js";
import { createEditorPointerController } from "./editor_pointer_controller.js";
import { createCanvasContextMenuHost } from "./editor_context_menu.js";
import { createEditorCommandController } from "./editor_command_controller.js";
import { createEditorCommandEngine } from "./editor_command_engine.js";
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
import { createTextSymbolPalette } from "./text_symbol_palette.js";
import {
  primitiveStrokeWidthValue,
  renderCorePrimitive,
} from "./primitive_dom_renderer.js";
import {
  ptToCssPx,
  cssPxToPt,
  displayMetrics,
  mapLengthArray,
  setDisplayScaleOverride,
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
const DEFAULT_WORKSPACE_WIDTH = 900;
const DEFAULT_WORKSPACE_HEIGHT = 600;
// The editor canvas is a growing world-space viewBox. These ratios define
// how much empty room to keep around content and when to expand the world.
const EDITOR_VIEW_BUFFER_RATIO = 0.6;
const EDITOR_AUTO_EXPAND_TRIGGER_RATIO = 0.18;
const EDITOR_FIT_PADDING_RATIO = 0.08;
const ZOOM_STEP_LEVELS = [12, 25, 50, 75, 100, 150, 200, 400, 600, 800];
const ZOOM_MIN_PERCENT = ZOOM_STEP_LEVELS[0];
const ZOOM_MAX_PERCENT = ZOOM_STEP_LEVELS[ZOOM_STEP_LEVELS.length - 1];
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
let textSymbolPalette = null;
let resolveAppInitialDocumentReady = null;
const appInitialDocumentReady = new Promise((resolve) => {
  resolveAppInitialDocumentReady = resolve;
});
registerChemcoreDebug({
  state,
  getEngineState: () => currentEditorEngineState(),
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

let zoomPercent = 100;
const documentTabs = [];
let activeDocumentTabId = null;
let activeTextEditor = null;
const UNSAVED_CLOSE_DECISION = {
  SAVE: "save",
  DISCARD: "discard",
  CANCEL: "cancel",
};
const REPEAT_UNIT_UNGROUP_WARNING_KEY = "chemcore:hide-repeat-unit-ungroup-warning";
const BROWSER_PENDING_DOCUMENT_KEY_PREFIX = "chemcore:pending-browser-document:";
const BROWSER_PENDING_DOCUMENT_PARAM = "chemcorePendingDocument";
let activeTitlebarTabDrag = null;
let detachingDocumentTabId = null;
let suppressNextDocumentTabClick = false;
let activeUnsavedChangesDialog = null;
let activeRepeatUnitUngroupDialog = null;
let windowCloseGuardInProgress = false;
let forceWindowClose = false;
let activeDocumentPreviewObjectIds = new Set();
let activeDocumentPreviewPrimitiveElements = new Set();
let activeDocumentPreviewHiddenElements = new Set();
let activeDocumentEditPreviewHiddenElements = new Set();
let activeDocumentPreviewLayer = false;
let activeDocumentPreviewBatchLayer = null;
let activeDocumentPreviewTransform = "";
let documentPrimitiveNodeElements = new Map();
let documentPrimitiveBondElements = new Map();
let documentMoleculeTopologyCache = null;
const DOCUMENT_PREVIEW_BATCH_ELEMENT_THRESHOLD = 96;
const editorEngineReadCache = {
  engine: null,
  revision: null,
  stateJson: null,
  parsedState: null,
  renderListJson: null,
  renderList: null,
  interactionRenderListJson: null,
  interactionRenderList: null,
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
  tab.zoomPercent = zoomPercent;
  tab.title = documentTitleFromState();
  syncWindowTitle();
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
  state.savedDocumentJson = currentDocumentSaveFingerprint();
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
  const revision = tab?.editorEngine?.revision?.();
  return Number.isFinite(Number(revision)) ? Number(revision) : null;
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
  const documentJson = tab?.currentDocument
    ? `${JSON.stringify(tab.currentDocument, null, 2)}\n`
    : tab?.editorEngine?.documentJson?.();
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
    scheduleDeferredDocumentSync();
    renderDocumentTabs();
    syncWindowTitle();
    refreshCommandAvailability();
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

function bindDesktopWindowChrome() {
  if (!isDesktopShell || !desktopFileHost?.available) {
    return;
  }
  void desktopFileHost.listenWindowCloseRequested?.(async (event) => {
    if (forceWindowClose) {
      return;
    }
    event?.preventDefault?.();
    await requestCloseWindow();
  });
  desktopTitlebar?.querySelectorAll("[data-window-command]").forEach((button) => {
    button.addEventListener("click", async () => {
      const command = button.dataset.windowCommand;
      if (command === "minimize") {
        await desktopFileHost.minimizeWindow?.();
      } else if (command === "maximize") {
        await desktopFileHost.toggleMaximizeWindow?.();
        await syncDesktopMaximizedState();
      } else if (command === "close") {
        await requestCloseWindow();
      }
    });
  });
  document.addEventListener("pointerdown", handleDesktopWindowDragPointerDown, true);
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

function desktopWindowDragRegionFromEvent(event) {
  if (!isDesktopShell || !desktopFileHost?.available || !event.target?.closest) {
    return null;
  }
  const region = event.target.closest("[data-desktop-window-drag-region]");
  if (!region) {
    return null;
  }
  const interactive = event.target.closest(
    "button, input, select, textarea, a, [role='button'], [contenteditable='true']",
  );
  return interactive && region.contains(interactive) ? null : region;
}

async function handleDesktopWindowDragPointerDown(event) {
  const region = desktopWindowDragRegionFromEvent(event);
  if (!region || event.button !== 0 || event.detail > 1) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  await desktopFileHost.startWindowDrag?.();
}

async function syncDesktopMaximizedState() {
  if (!isDesktopShell || !desktopFileHost?.isWindowMaximized) {
    return;
  }
  const maximized = await desktopFileHost.isWindowMaximized().catch(() => false);
  document.body.classList.toggle("is-window-maximized", !!maximized);
}

function makeUnsavedChangesButton(label, decision, className = "") {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.dataset.unsavedDecision = decision;
  if (className) {
    button.className = className;
  }
  return button;
}

function showUnsavedChangesDialog(title) {
  if (activeUnsavedChangesDialog) {
    return activeUnsavedChangesDialog;
  }
  activeUnsavedChangesDialog = new Promise((resolve) => {
    const previousFocus = document.activeElement;
    const root = document.createElement("div");
    root.className = "unsaved-changes-dialog";
    root.setAttribute("role", "alertdialog");
    root.setAttribute("aria-modal", "true");
    root.setAttribute("aria-labelledby", "unsaved-changes-title");
    root.setAttribute("aria-describedby", "unsaved-changes-message");

    const backdrop = document.createElement("div");
    backdrop.className = "unsaved-changes-backdrop";
    const windowDragStrip = document.createElement("div");
    windowDragStrip.className = "desktop-modal-window-drag-strip";
    windowDragStrip.dataset.desktopWindowDragRegion = "true";
    windowDragStrip.setAttribute("aria-hidden", "true");
    const panel = document.createElement("section");
    panel.className = "unsaved-changes-panel";
    const panelDragStrip = document.createElement("div");
    panelDragStrip.className = "desktop-dialog-panel-drag-strip";
    panelDragStrip.dataset.desktopWindowDragRegion = "true";
    panelDragStrip.setAttribute("aria-hidden", "true");

    const heading = document.createElement("h2");
    heading.id = "unsaved-changes-title";
    heading.className = "unsaved-changes-title";
    heading.textContent = "Save changes?";

    const message = document.createElement("p");
    message.id = "unsaved-changes-message";
    message.className = "unsaved-changes-message";
    message.textContent = `Do you want to save changes to "${title || "Untitled"}" before closing?`;

    const actions = document.createElement("div");
    actions.className = "unsaved-changes-actions";
    const saveButton = makeUnsavedChangesButton("Save", UNSAVED_CLOSE_DECISION.SAVE, "is-primary");
    const discardButton = makeUnsavedChangesButton("Don't Save", UNSAVED_CLOSE_DECISION.DISCARD);
    const cancelButton = makeUnsavedChangesButton("Cancel", UNSAVED_CLOSE_DECISION.CANCEL);
    actions.append(saveButton, discardButton, cancelButton);
    panel.append(panelDragStrip, heading, message, actions);
    root.append(backdrop, windowDragStrip, panel);

    const finish = (decision) => {
      root.remove();
      document.removeEventListener("keydown", onKeyDown, true);
      activeUnsavedChangesDialog = null;
      if (previousFocus && typeof previousFocus.focus === "function") {
        previousFocus.focus({ preventScroll: true });
      }
      resolve(decision);
    };
    const onKeyDown = (event) => {
      if (event.key === "Escape") {
        event.preventDefault();
        finish(UNSAVED_CLOSE_DECISION.CANCEL);
      }
    };
    actions.addEventListener("click", (event) => {
      const button = event.target.closest("[data-unsaved-decision]");
      if (!button) {
        return;
      }
      finish(button.dataset.unsavedDecision || UNSAVED_CLOSE_DECISION.CANCEL);
    });
    document.addEventListener("keydown", onKeyDown, true);
    document.body.append(root);
    saveButton.focus({ preventScroll: true });
  });
  return activeUnsavedChangesDialog;
}

function repeatUnitUngroupWarningHidden() {
  try {
    return localStorage.getItem(REPEAT_UNIT_UNGROUP_WARNING_KEY) === "true";
  } catch {
    return false;
  }
}

function setRepeatUnitUngroupWarningHidden(hidden) {
  try {
    if (hidden) {
      localStorage.setItem(REPEAT_UNIT_UNGROUP_WARNING_KEY, "true");
    } else {
      localStorage.removeItem(REPEAT_UNIT_UNGROUP_WARNING_KEY);
    }
  } catch {
    // Preferences are best-effort; the document command should not depend on storage.
  }
}

function showRepeatUnitUngroupDialog() {
  if (activeRepeatUnitUngroupDialog) {
    return activeRepeatUnitUngroupDialog;
  }
  activeRepeatUnitUngroupDialog = new Promise((resolve) => {
    const previousFocus = document.activeElement;
    const root = document.createElement("div");
    root.className = "repeat-unit-ungroup-dialog";
    root.setAttribute("role", "alertdialog");
    root.setAttribute("aria-modal", "true");
    root.setAttribute("aria-labelledby", "repeat-unit-ungroup-title");
    root.setAttribute("aria-describedby", "repeat-unit-ungroup-message");

    const backdrop = document.createElement("div");
    backdrop.className = "repeat-unit-ungroup-backdrop";
    const windowDragStrip = document.createElement("div");
    windowDragStrip.className = "desktop-modal-window-drag-strip";
    windowDragStrip.dataset.desktopWindowDragRegion = "true";
    windowDragStrip.setAttribute("aria-hidden", "true");
    const panel = document.createElement("section");
    panel.className = "repeat-unit-ungroup-panel";
    const panelDragStrip = document.createElement("div");
    panelDragStrip.className = "desktop-dialog-panel-drag-strip";
    panelDragStrip.dataset.desktopWindowDragRegion = "true";
    panelDragStrip.setAttribute("aria-hidden", "true");

    const heading = document.createElement("h2");
    heading.id = "repeat-unit-ungroup-title";
    heading.className = "repeat-unit-ungroup-title";
    heading.textContent = "Ungroup repeat unit?";

    const message = document.createElement("p");
    message.id = "repeat-unit-ungroup-message";
    message.className = "repeat-unit-ungroup-message";
    message.textContent = "Ungrouping will remove the repeat-count link from the number label. The bracket remains part of the molecule.";

    const footer = document.createElement("div");
    footer.className = "repeat-unit-ungroup-footer";
    const rememberLabel = document.createElement("label");
    rememberLabel.className = "repeat-unit-ungroup-remember";
    const rememberCheckbox = document.createElement("input");
    rememberCheckbox.type = "checkbox";
    rememberCheckbox.value = "1";
    rememberLabel.append(rememberCheckbox, document.createTextNode("Don't show again"));

    const actions = document.createElement("div");
    actions.className = "repeat-unit-ungroup-actions";
    const ungroupButton = document.createElement("button");
    ungroupButton.type = "button";
    ungroupButton.className = "is-primary";
    ungroupButton.dataset.repeatUnitUngroupDecision = "confirm";
    ungroupButton.textContent = "Ungroup";
    const cancelButton = document.createElement("button");
    cancelButton.type = "button";
    cancelButton.dataset.repeatUnitUngroupDecision = "cancel";
    cancelButton.textContent = "Cancel";
    actions.append(ungroupButton, cancelButton);
    footer.append(rememberLabel, actions);
    panel.append(panelDragStrip, heading, message, footer);
    root.append(backdrop, windowDragStrip, panel);

    const finish = (confirmed) => {
      if (confirmed && rememberCheckbox.checked) {
        setRepeatUnitUngroupWarningHidden(true);
      }
      root.remove();
      document.removeEventListener("keydown", onKeyDown, true);
      activeRepeatUnitUngroupDialog = null;
      if (previousFocus && typeof previousFocus.focus === "function") {
        previousFocus.focus({ preventScroll: true });
      }
      resolve(confirmed);
    };
    const onKeyDown = (event) => {
      if (event.key === "Escape") {
        event.preventDefault();
        finish(false);
      }
    };
    actions.addEventListener("click", (event) => {
      const button = event.target.closest("[data-repeat-unit-ungroup-decision]");
      if (!button) {
        return;
      }
      finish(button.dataset.repeatUnitUngroupDecision === "confirm");
    });
    document.addEventListener("keydown", onKeyDown, true);
    document.body.append(root);
    ungroupButton.focus({ preventScroll: true });
  });
  return activeRepeatUnitUngroupDialog;
}

async function confirmRepeatUnitUngroupIfNeeded() {
  if (repeatUnitUngroupWarningHidden()) {
    return true;
  }
  const hasRepeatUnitGroup = !!state.editorEngine?.selectionHasRepeatUnitGroups?.();
  if (!hasRepeatUnitGroup) {
    return true;
  }
  return showRepeatUnitUngroupDialog();
}

async function prepareDocumentTabForDirtyCheck(tab) {
  if (!tab || tab.id !== activeDocumentTabId) {
    return;
  }
  await finishActiveTextEditor(true);
  if (state.editorEngine) {
    await syncDocumentFromEngine();
  }
  saveActiveDocumentTabState();
  renderDocumentTabs();
  syncWindowTitle();
  refreshCommandAvailability();
}

async function saveDocumentTabBeforeClose(tab) {
  const target = documentTabs.find((entry) => entry.id === tab?.id);
  if (!target) {
    return true;
  }
  if (target.id !== activeDocumentTabId) {
    await activateDocumentTab(target.id);
  }
  try {
    const saved = await saveCurrentDocument();
    if (!saved) {
      return false;
    }
    saveActiveDocumentTabState();
    renderDocumentTabs();
    syncWindowTitle();
    return true;
  } catch (error) {
    if (isAbortError(error)) {
      return false;
    }
    console.error("Save before close failed", error);
    window.alert?.(`Save failed: ${error.message || error}`);
    return false;
  }
}

async function confirmUnsavedChangesForTab(tab) {
  await prepareDocumentTabForDirtyCheck(tab);
  const freshTab = documentTabs.find((entry) => entry.id === tab?.id) || tab;
  if (!documentTabIsDirty(freshTab)) {
    return true;
  }
  const decision = await showUnsavedChangesDialog(freshTab?.title || "Untitled");
  if (decision === UNSAVED_CLOSE_DECISION.CANCEL) {
    return false;
  }
  if (decision === UNSAVED_CLOSE_DECISION.DISCARD) {
    return true;
  }
  return saveDocumentTabBeforeClose(freshTab);
}

async function confirmCloseDocumentTab(tabId) {
  const tab = documentTabs.find((entry) => entry.id === tabId);
  if (!tab) {
    return true;
  }
  return confirmUnsavedChangesForTab(tab);
}

async function confirmCloseAllDocumentTabs() {
  const active = activeDocumentTab();
  const orderedTabs = [
    ...(active ? [active] : []),
    ...documentTabs.filter((tab) => tab.id !== active?.id),
  ];
  for (const tab of orderedTabs) {
    if (!documentTabs.some((entry) => entry.id === tab.id)) {
      continue;
    }
    if (!await confirmUnsavedChangesForTab(tab)) {
      return false;
    }
  }
  return true;
}

async function requestCloseWindow() {
  if (windowCloseGuardInProgress) {
    return false;
  }
  windowCloseGuardInProgress = true;
  try {
    if (!await confirmCloseAllDocumentTabs()) {
      return false;
    }
    await autoSaveAllOleEditDocumentTabs();
    forceWindowClose = true;
    try {
      if (desktopFileHost?.destroyWindow) {
        await desktopFileHost.destroyWindow();
      } else if (desktopFileHost?.closeWindow) {
        await desktopFileHost.closeWindow();
      } else {
        window.close();
      }
    } catch (error) {
      console.error("Window close failed", error);
      window.alert?.(`Close failed: ${error.message || error}`);
      return false;
    }
    return true;
  } finally {
    windowCloseGuardInProgress = false;
    if (!document.hidden) {
      forceWindowClose = false;
    }
  }
}

function bindBrowserBeforeUnloadGuard() {
  if (isDesktopShell) {
    return;
  }
  window.addEventListener("beforeunload", (event) => {
    if (!activeTextEditorIsDirty() && !documentTabs.some((tab) => documentTabIsDirty(tab))) {
      return;
    }
    event.preventDefault();
    event.returnValue = "";
  });
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
  const effectiveTool = editorState.elementPlacementActive
    ? "element"
    : editorState.activeTool === "chain"
      ? "templates"
      : editorState.activeTool;
  const effectiveTemplate = editorState.activeTool === "chain" ? "chain" : editorState.template;
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
  return state.displayMetrics.cssPxPerPt * (closestZoomStep(percent) / 100);
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
  return state.displayMetrics.cssPxPerPt * zoomScale();
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
    && (
      Math.abs(previous.devicePixelRatio - next.devicePixelRatio) > 0.001
      || Math.abs(previous.cssPxPerPt - next.cssPxPerPt) > 0.001
    )
    && viewerSvg
  ) {
    applyViewerViewport();
  }
  return next;
}

function applyDisplayScaleOverride(scale) {
  const centerWorld = currentViewportCenterWorld();
  setDisplayScaleOverride(scale);
  refreshDisplayMetrics();
  applyViewerViewport({
    centerWorld,
  });
  updateDocumentMeta();
  return state.displayMetrics;
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
  viewerSvg.style.setProperty("--chemcore-css-px-per-pt", String(state.displayMetrics.cssPxPerPt));
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
  return zoomStepAtOrBelow((scale / state.displayMetrics.cssPxPerPt) * 100);
}

function editorCanvasViewBoxFromBounds(bounds, scale = viewportScale()) {
  const metrics = editorViewportMetrics(scale);
  return paddedViewBoxFromBounds(
    bounds,
    metrics.bufferX,
    metrics.bufferY,
    metrics.minCanvasWidth,
    metrics.minCanvasHeight,
  );
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

function currentSelectionHandleZoneContainsPoint(point) {
  const bounds = currentRenderBounds("selection");
  if (!bounds) {
    return true;
  }
  const edgePad = screenPxToWorld(14);
  const rotatePad = screenPxToWorld(18);
  const insideExpandedBounds = point.x >= bounds.minX - edgePad
    && point.x <= bounds.maxX + edgePad
    && point.y >= bounds.minY - rotatePad
    && point.y <= bounds.maxY + edgePad;
  if (!insideExpandedBounds) {
    return false;
  }
  const nearEdge = Math.abs(point.x - bounds.minX) <= edgePad
    || Math.abs(point.x - bounds.maxX) <= edgePad
    || Math.abs(point.y - bounds.minY) <= edgePad
    || Math.abs(point.y - bounds.maxY) <= edgePad;
  if (nearEdge) {
    return true;
  }
  const rotateHandle = {
    x: (bounds.minX + bounds.maxX) * 0.5,
    y: bounds.minY - screenPxToWorld(18),
  };
  return pointDistance(point, rotateHandle) <= rotatePad;
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

  // Expanding left/top changes the world-space origin, so we record the delta
  // and compensate scroll afterward to avoid a visible jump.
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
  activeDocumentPreviewTransform: () => activeDocumentPreviewTransform,
  activeGestureUsesDocumentPreview,
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

function sceneObjectType(object) {
  return object?.type || object?.objectType || object?.object_type || "object";
}

function currentDocumentSceneObjectMap() {
  const objects = new Map();
  const visit = (object) => {
    if (!object?.id) {
      return;
    }
    objects.set(object.id, object);
    for (const child of object.children || []) {
      visit(child);
    }
  };
  for (const object of state.currentDocument?.objects || []) {
    visit(object);
  }
  return objects;
}

function currentDocumentObjectIdsInPaintOrder() {
  const order = [];
  const seen = new Set();
  const add = (objectId) => {
    if (objectId && !seen.has(objectId)) {
      seen.add(objectId);
      order.push(objectId);
    }
  };
  for (const primitive of state.coreRenderList || []) {
    add(primitiveObjectId(primitive));
  }
  for (const object of collectCurrentDocumentSceneObjects()) {
    add(object.id);
  }
  return order;
}

function targetIdsFromCommandResult(result, key) {
  const ids = new Set();
  for (const bucket of ["targets", "created", "updated", "deleted"]) {
    for (const id of result?.[bucket]?.[key] || []) {
      if (id) {
        ids.add(id);
      }
    }
  }
  return ids;
}

function objectStyleRef(object) {
  return object?.styleRef || object?.style_ref || "";
}

function addObjectIdsForStyleTargets(objectIds, styleIds, objectMap = currentDocumentSceneObjectMap()) {
  if (!styleIds.size) {
    return;
  }
  for (const [objectId, object] of objectMap) {
    if (styleIds.has(objectStyleRef(object))) {
      objectIds.add(objectId);
    }
  }
}

function addObjectIdsForPrimitiveTargets(objectIds, nodeIds, bondIds) {
  if (!nodeIds.size && !bondIds.size) {
    return;
  }
  const remainingNodeIds = new Set(nodeIds);
  const remainingBondIds = new Set(bondIds);
  const addFromPrimitive = (primitive) => {
    const objectId = primitiveObjectId(primitive);
    if (!objectId) {
      return;
    }
    const nodeId = primitiveNodeId(primitive);
    const bondId = primitiveBondId(primitive);
    if (nodeId && remainingNodeIds.has(nodeId)) {
      objectIds.add(objectId);
      remainingNodeIds.delete(nodeId);
    }
    if (bondId && remainingBondIds.has(bondId)) {
      objectIds.add(objectId);
      remainingBondIds.delete(bondId);
    }
  };
  for (const primitive of state.coreRenderList || []) {
    if (!remainingNodeIds.size && !remainingBondIds.size) {
      break;
    }
    addFromPrimitive(primitive);
  }
  const addFromExistingDom = (attribute, id, remaining) => {
    if (!remaining.has(id)) {
      return;
    }
    const element = documentPrimitiveElementsForId(attribute, id)[0];
    const objectElement = element?.closest?.("[data-object-id]");
    const objectId = objectElement?.dataset?.objectId || element?.dataset?.objectId || "";
    if (objectId) {
      objectIds.add(objectId);
      remaining.delete(id);
    }
  };
  for (const nodeId of [...remainingNodeIds]) {
    addFromExistingDom("data-node-id", nodeId, remainingNodeIds);
  }
  for (const bondId of [...remainingBondIds]) {
    addFromExistingDom("data-bond-id", bondId, remainingBondIds);
  }
  if (remainingNodeIds.size || remainingBondIds.size) {
    for (const object of collectCurrentDocumentSceneObjects()) {
      if (sceneObjectType(object) === "molecule") {
        objectIds.add(object.id);
      }
    }
  }
}

function expandObjectIdsWithDescendants(objectIds, objectMap = currentDocumentSceneObjectMap()) {
  const expanded = new Set(objectIds);
  const visit = (object) => {
    for (const child of object?.children || []) {
      if (child?.id) {
        expanded.add(child.id);
      }
      visit(child);
    }
  };
  for (const objectId of [...objectIds]) {
    visit(objectMap.get(objectId));
  }
  return expanded;
}

function objectIdsForCommandResultPatch(result) {
  if (!result?.changed) {
    return new Set();
  }
  const objectIds = targetIdsFromCommandResult(result, "objects");
  addObjectIdsForStyleTargets(
    objectIds,
    targetIdsFromCommandResult(result, "styles"),
  );
  addObjectIdsForPrimitiveTargets(
    objectIds,
    targetIdsFromCommandResult(result, "nodes"),
    targetIdsFromCommandResult(result, "bonds"),
  );
  return expandObjectIdsWithDescendants(objectIds);
}

function removeDocumentObjectDom(documentLayer, objectId) {
  const selector = `[data-object-id="${CSS.escape(objectId)}"]`;
  const nodes = [...documentLayer.querySelectorAll(selector)];
  const nodeSet = new Set(nodes);
  for (const node of nodes) {
    let parent = node.parentElement;
    let hasRemovedAncestor = false;
    while (parent && parent !== documentLayer) {
      if (nodeSet.has(parent)) {
        hasRemovedAncestor = true;
        break;
      }
      parent = parent.parentElement;
    }
    if (!hasRemovedAncestor) {
      node.remove();
    }
  }
}

function renderTargetObjectPrimitives(objectId) {
  return parseEngineJson(state.editorEngine?.renderTargetsJson?.(JSON.stringify({
    objects: [objectId],
  })) || "[]", []);
}

function renderDocumentObjectPatchNode(objectId, objectMap) {
  const object = objectMap.get(objectId);
  const primitives = renderTargetObjectPrimitives(objectId);
  if (!object && !primitives.length) {
    return null;
  }
  const group = makeSvgNode("g", {
    "data-object-id": objectId,
    "data-object-type": sceneObjectType(object),
    "data-renderer": primitives.length ? "core-patch" : "scene-patch",
  });
  if (primitives.length) {
    for (const primitive of primitives) {
      renderCorePrimitive(group, primitive, corePrimitiveRenderOptions());
    }
  } else if (object) {
    const wrapper = makeSvgNode("g", {});
    sceneRenderer.renderSceneObject(wrapper, object, state.currentDocument);
    group.append(...wrapper.childNodes);
  }
  return group.childNodes.length ? group : null;
}

function findDocumentPatchAnchor(documentLayer, objectId, patchedObjectIds, paintOrder) {
  const startIndex = paintOrder.indexOf(objectId);
  if (startIndex < 0) {
    return null;
  }
  const laterObjectIds = new Set(
    paintOrder.slice(startIndex + 1).filter((candidate) => !patchedObjectIds.has(candidate)),
  );
  for (const child of documentLayer.children) {
    const childObjectId = child.dataset?.objectId
      || child.querySelector?.("[data-object-id]")?.dataset?.objectId
      || "";
    if (laterObjectIds.has(childObjectId)) {
      return child;
    }
  }
  return null;
}

function renderDocumentChange(result = null) {
  if (!isEditingRustDocument() || !state.currentDocument || !result?.changed) {
    renderDocument();
    return true;
  }
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  const objectIds = objectIdsForCommandResultPatch(result);
  if (!documentLayer) {
    return true;
  }
  if (!objectIds.size) {
    return renderDocumentPrimitiveChange(result) || true;
  }
  clearDocumentObjectPreviewTransform();
  const objectMap = currentDocumentSceneObjectMap();
  const paintOrder = currentDocumentObjectIdsInPaintOrder();
  const orderedObjectIds = [...objectIds].sort((a, b) => {
    const ai = paintOrder.indexOf(a);
    const bi = paintOrder.indexOf(b);
    return (ai < 0 ? Number.MAX_SAFE_INTEGER : ai) - (bi < 0 ? Number.MAX_SAFE_INTEGER : bi);
  });
  for (const objectId of orderedObjectIds) {
    removeDocumentObjectDom(documentLayer, objectId);
  }
  for (const objectId of orderedObjectIds) {
    const node = renderDocumentObjectPatchNode(objectId, objectMap);
    if (!node) {
      continue;
    }
    const anchor = findDocumentPatchAnchor(documentLayer, objectId, objectIds, paintOrder);
    documentLayer.insertBefore(node, anchor);
  }
  rebuildDocumentPrimitiveIndex(documentLayer);
  syncViewerStats();
  renderEditorOverlay();
  positionActiveTextEditor();
  return true;
}

function commandTargetSet(values) {
  return [...(values || [])].filter(Boolean);
}

function primitiveDomIndexForAttribute(attribute) {
  if (attribute === "data-node-id") {
    return documentPrimitiveNodeElements;
  }
  if (attribute === "data-bond-id") {
    return documentPrimitiveBondElements;
  }
  return null;
}

function resetDocumentPrimitiveIndex() {
  documentPrimitiveNodeElements = new Map();
  documentPrimitiveBondElements = new Map();
}

function addDocumentPrimitiveIndexEntry(index, id, element) {
  if (!id || !index) {
    return;
  }
  let elements = index.get(id);
  if (!elements) {
    elements = new Set();
    index.set(id, elements);
  }
  elements.add(element);
}

function removeDocumentPrimitiveIndexEntry(index, id, element) {
  if (!id || !index) {
    return;
  }
  const elements = index.get(id);
  if (!elements) {
    return;
  }
  elements.delete(element);
  if (!elements.size) {
    index.delete(id);
  }
}

function indexDocumentPrimitiveElement(element) {
  addDocumentPrimitiveIndexEntry(documentPrimitiveNodeElements, element.getAttribute("data-node-id"), element);
  addDocumentPrimitiveIndexEntry(documentPrimitiveBondElements, element.getAttribute("data-bond-id"), element);
}

function unindexDocumentPrimitiveElement(element) {
  removeDocumentPrimitiveIndexEntry(documentPrimitiveNodeElements, element.getAttribute("data-node-id"), element);
  removeDocumentPrimitiveIndexEntry(documentPrimitiveBondElements, element.getAttribute("data-bond-id"), element);
}

function indexDocumentPrimitiveTree(root) {
  if (!root?.querySelectorAll) {
    return;
  }
  if (root.nodeType === Node.ELEMENT_NODE) {
    indexDocumentPrimitiveElement(root);
  }
  root
    .querySelectorAll("[data-node-id], [data-bond-id]")
    .forEach(indexDocumentPrimitiveElement);
}

function rebuildDocumentPrimitiveIndex(documentLayer) {
  resetDocumentPrimitiveIndex();
  indexDocumentPrimitiveTree(documentLayer);
}

function documentPrimitiveElementsForId(attribute, id) {
  const index = primitiveDomIndexForAttribute(attribute);
  return index ? [...(index.get(id) || [])].filter((element) => element.isConnected) : [];
}

function collectDocumentPrimitiveTargetElements(documentLayer, nodeIds, bondIds) {
  const elements = new Set();
  for (const nodeId of nodeIds) {
    documentPrimitiveElementsForId("data-node-id", nodeId).forEach((element) => elements.add(element));
  }
  for (const bondId of bondIds) {
    documentPrimitiveElementsForId("data-bond-id", bondId).forEach((element) => elements.add(element));
  }
  return elements;
}

function renderDocumentPrimitivePatch(primitives, nodeIds, bondIds) {
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (!documentLayer || !Array.isArray(primitives)) {
    return false;
  }
  const targetElements = [...collectDocumentPrimitiveTargetElements(documentLayer, nodeIds, bondIds)];
  const targetElementSet = new Set(targetElements);
  let anchor = null;
  let seenTarget = false;
  for (const child of [...documentLayer.children]) {
    if (targetElementSet.has(child)) {
      seenTarget = true;
      continue;
    }
    if (seenTarget) {
      anchor = child;
      break;
    }
  }
  for (const element of targetElements) {
    unindexDocumentPrimitiveElement(element);
    element.remove();
  }
  const fragment = document.createDocumentFragment();
  for (const primitive of primitives) {
    renderCorePrimitive(fragment, primitive, corePrimitiveRenderOptions());
  }
  if (!fragment.childNodes.length) {
    return false;
  }
  indexDocumentPrimitiveTree(fragment);
  documentLayer.insertBefore(fragment, anchor);
  syncViewerStats();
  positionActiveTextEditor();
  return true;
}

function renderDocumentPrimitiveChange(result = null) {
  if (!isEditingRustDocument() || !state.currentDocument || !result?.changed) {
    return false;
  }
  const nodeIds = targetIdsFromCommandResult(result, "nodes");
  const bondIds = targetIdsFromCommandResult(result, "bonds");
  if (!nodeIds.size && !bondIds.size) {
    return false;
  }
  if (targetIdsFromCommandResult(result, "styles").size > 0) {
    return false;
  }
  const primitives = parseEngineJson(state.editorEngine?.renderTargetsJson?.(JSON.stringify({
    nodes: commandTargetSet(nodeIds),
    bonds: commandTargetSet(bondIds),
  })) || "[]", []);
  clearDocumentObjectPreviewTransform();
  return renderDocumentPrimitivePatch(primitives, nodeIds, bondIds);
}

function structurePreviewTargetIds(selection = currentEditorEngineState()?.selection) {
  const nodeIds = activeStructurePreviewNodeIds(selection);
  const bondIds = selectedStructurePreviewBondIds(selection, nodeIds);
  const topology = currentDocumentMoleculeTopology();
  for (const nodeId of nodeIds) {
    for (const entry of topology.bondsByNode.get(nodeId) || []) {
      if (entry.bond?.id) {
        bondIds.add(entry.bond.id);
      }
    }
  }
  return { nodeIds, bondIds };
}

function selectionNeedsBackendMovePreview(selection = currentEditorEngineState()?.selection) {
  const targets = structurePreviewTargetIds(selection);
  return targets.nodeIds.size > 0 || targets.bondIds.size > 0;
}

function recordBackendMovePreviewTiming(sample) {
  const debug = window.__chemcoreDebug;
  if (!debug) {
    return;
  }
  const stats = debug.backendMovePreviewStats || { samples: [] };
  stats.samples.push(sample);
  if (stats.samples.length > 240) {
    stats.samples.splice(0, stats.samples.length - 240);
  }
  stats.last = sample;
  debug.backendMovePreviewStats = stats;
}

async function applyBackendSelectionMovePreview(point, altKey = false) {
  const gesture = activeSelectionGesture;
  const selection = gesture?.previewSelection || currentEditorEngineState()?.selection;
  if (gesture?.kind !== "move" || !point || !selectionNeedsBackendMovePreview(selection)) {
    return false;
  }
  gesture.previewSelection = selection;
  const targets = structurePreviewTargetIds(selection);
  const started = performance.now();
  const moveResult = state.editorEngine.updateSelectionMove?.(point.x, point.y, altKey);
  const changed = moveResult && typeof moveResult.then === "function" ? await moveResult : moveResult;
  const updatedAt = performance.now();
  if (!changed) {
    recordBackendMovePreviewTiming({
      updateMs: updatedAt - started,
      renderTargetsMs: 0,
      patchMs: 0,
      totalMs: updatedAt - started,
      primitiveCount: 0,
      nodeCount: targets.nodeIds.size,
      bondCount: targets.bondIds.size,
      patched: false,
      changed: false,
    });
    return true;
  }
  const primitives = parseEngineJson(state.editorEngine.renderTargetsJson?.(JSON.stringify({
    nodes: commandTargetSet(targets.nodeIds),
    bonds: commandTargetSet(targets.bondIds),
  })) || "[]", []);
  const renderedAt = performance.now();
  const patched = renderDocumentPrimitivePatch(primitives, targets.nodeIds, targets.bondIds);
  const patchedAt = performance.now();
  recordBackendMovePreviewTiming({
    updateMs: updatedAt - started,
    renderTargetsMs: renderedAt - updatedAt,
    patchMs: patchedAt - renderedAt,
    totalMs: patchedAt - started,
    primitiveCount: primitives.length,
    nodeCount: targets.nodeIds.size,
    bondCount: targets.bondIds.size,
    patched,
    changed: true,
  });
  return patched;
}

function syncViewerStats() {
  const counts = {};
  for (const object of state.currentDocument?.objects || []) {
    counts[object.type] = (counts[object.type] || 0) + 1;
  }
  viewerStats.textContent = Object.entries(counts)
    .map(([type, count]) => `${type}: ${count}`)
    .join(" | ");
}

function renderEditorOverlay(renderList = null) {
  const effectiveRenderList = activeGestureUsesObjectEditPreview() && !renderList
    ? currentEditorInteractionRenderList()
    : renderList;
  syncObjectEditPreviewHiddenElements(effectiveRenderList || []);
  editorOverlayRenderer.renderEditorOverlay(effectiveRenderList);
}
const currentSelectionRotateHandle = (...args) => editorOverlayRenderer.currentSelectionRotateHandle(...args);
const selectionResizeHandleHit = (...args) => editorOverlayRenderer.selectionResizeHandleHit(...args);
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
  renderSelectionOnlyUpdate,
  selectionResizeHandleHit,
  selectionRotateHandleHit,
  currentSelectionRotateHandle,
  selectionResizeGestureScale,
  selectionRotateAngleForGesture,
  currentRenderBounds,
  currentEditorRenderList,
  currentEditorInteractionRenderList,
  currentEditorOverlayRenderList,
  renderEditorOverlay,
  invalidateEditorEngineReadCache,
  selectionHasLargeOverlay: () => currentSelectionItemCount() >= 80,
  selectionBoundsContainsPoint: currentSelectionBoundsContainsPoint,
  selectionHitContainsPoint: currentSelectionHitContainsPoint,
  selectionNeedsBackendMovePreview,
  applyBackendSelectionMovePreview,
  applyDocumentObjectPreviewTransform,
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
  activeBracketDragStart: () => state.activeBracketDragStart,
  setActiveBracketDragStart: (value) => { state.activeBracketDragStart = value; },
});

async function syncDocumentFromEngine(options = {}) {
  if (!state.editorEngine) {
    return;
  }
  const syncRenderList = options.syncRenderList ?? true;
  if (!syncRenderList && typeof state.editorEngine.refreshSnapshot === "function") {
    await state.editorEngine.refreshSnapshot("documentState");
  }
  const documentData = parseEngineJson(state.editorEngine.documentJson());
  if (documentData) {
    state.currentDocument = documentData;
    documentMoleculeTopologyCache = null;
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
    if (activeSelectionGesture) {
      scheduleDeferredDocumentSync();
      return;
    }
    await syncDocumentFromEngine();
    saveActiveDocumentTabState();
    renderDocumentTabs();
    syncWindowTitle();
  }, 1200);
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
  if (!nextTool) {
    return false;
  }
  if (editorState.activeTool === nextTool && !editorState.elementPlacementActive) {
    return false;
  }
  editorState.elementPlacementActive = false;
  if (activeTextEditor && nextTool !== "element") {
    await closeActiveTextEditorForToolAction();
  }
  if (editorState.activeTool === "select" && nextTool !== "select") {
    activeSelectionGesture = null;
    await state.editorEngine?.clearSelection?.();
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
  viewerSvg.style.cursor = editorState.elementPlacementActive
    ? elementCursor(editorState.elementSymbol)
    : editorState.activeTool === "text"
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
    viewerSvg.style.cursor = "grabbing";
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
  if ((editorState.activeTool === "select" || editorState.activeTool === "tlc-plate") && activeTlcSpotHover) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  if (
    activeToolCanDragSelection()
    && currentSelectionHitContainsPoint(point)
    && !currentSelectionHandleZoneContainsPoint(point)
  ) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  if (editorState.activeTool === "select") {
    const resizeHandle = selectionResizeHandleHit(point);
    if (resizeHandle) {
      viewerSvg.style.cursor = resizeHandle.cursor;
      return;
    }
  }
  if (editorState.activeTool === "select" && selectionRotateHandleHit(point)) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  if (activeToolCanDragSelection() && currentSelectionHitContainsPoint(point)) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  const arrowAction = await state.editorEngine.hoverArrowAction?.(point.x, point.y) || "";
  if (arrowAction === "head" || arrowAction === "tail") {
    viewerSvg.style.cursor = "move";
    return;
  }
  if (arrowAction === "head-style" || arrowAction === "tail-style") {
    viewerSvg.style.cursor = "nwse-resize";
    return;
  }
  if (arrowAction === "curve") {
    viewerSvg.style.cursor = "nesw-resize";
    return;
  }
  const overSelection = currentSelectionHitContainsPoint(point);
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
    viewerSvg.style.cursor = "grab";
    return;
  }
  if (editorState.activeTool === "select"
    || editorState.activeTool === "shape"
    || editorState.activeTool === "tlc-plate"
    || editorState.activeTool === "orbital") {
    const shapeAction = await state.editorEngine.hoverShapeAction?.(point.x, point.y) || "";
    const shapeCursor = cursorForShapeAction(shapeAction);
    if (shapeCursor) {
      viewerSvg.style.cursor = shapeCursor;
      return;
    }
  }
  if (editorState.activeTool === "arrow") {
    viewerSvg.style.cursor = "crosshair";
    return;
  }
  if (editorState.activeTool === "shape" || editorState.activeTool === "tlc-plate" || editorState.activeTool === "orbital") {
    viewerSvg.style.cursor = "crosshair";
    return;
  }
  if (editorState.activeTool === "chain") {
    viewerSvg.style.cursor = "crosshair";
    return;
  }
  viewerSvg.style.cursor = overSelection ? "grab" : "default";
}

function toolbarBondIconWidths() {
  const styles = getComputedStyle(document.documentElement);
  const thinPx = parseFloat(styles.getPropertyValue("--cc-icon-stroke-thin")) || 1.65;
  const thickPx = parseFloat(styles.getPropertyValue("--cc-icon-stroke-thick")) || 4.6;
  const iconPx = parseFloat(styles.getPropertyValue("--icon-svg-size")) || 30;
  const scale = 24 / Math.max(1, iconPx);
  return {
    thin: thinPx * scale,
    thick: thickPx * scale,
    key: `${thinPx}:${thickPx}:${iconPx}`,
  };
}

function refreshBondToolIcons() {
  const iconSvg = state.editorEngine?.bondToolIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const widths = toolbarBondIconWidths();
  const hasCompleteIconSet = BOND_TOOL_ICON_TYPES.every((type) => editorState.bondIconSvgs?.[type]);
  if (editorState.bondIconCacheKey === widths.key && hasCompleteIconSet) {
    return;
  }
  const icons = {};
  for (const type of BOND_TOOL_ICON_TYPES) {
    icons[type] = iconSvg.call(state.editorEngine, type, widths.thin, widths.thick);
  }
  editorState.bondIconSvgs = icons;
  editorState.bondIconCacheKey = widths.key;
}

function refreshChainToolIcon() {
  const iconSvg = state.editorEngine?.chainToolIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const widths = toolbarBondIconWidths();
  const cacheKey = `kernel-chain-v1:${widths.key}`;
  if (editorState.chainIconCacheKey === cacheKey && editorState.chainIconSvg) {
    return;
  }
  editorState.chainIconSvg = normalizeKernelChainIconSvg(
    iconSvg.call(state.editorEngine, widths.thin),
  );
  editorState.chainIconCacheKey = cacheKey;
}

function refreshArrowToolIcons() {
  const iconSvg = state.editorEngine?.arrowToolIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const hasCompleteIconSet = ARROW_TOOL_ICON_TYPES.every((type) => editorState.arrowIconSvgs?.[type]);
  if (editorState.arrowIconCacheKey === "kernel-arrow-v1" && hasCompleteIconSet) {
    return;
  }
  const icons = {};
  for (const type of ARROW_TOOL_ICON_TYPES) {
    icons[type] = normalizeKernelArrowIconSvg(iconSvg.call(state.editorEngine, type), type);
  }
  editorState.arrowIconSvgs = icons;
  editorState.arrowIconCacheKey = "kernel-arrow-v1";
}

function refreshTextFormatIcons() {
  const iconSvg = state.editorEngine?.textFormatIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const hasCompleteIconSet = TEXT_FORMAT_ICON_TYPES.every((type) => editorState.textIconSvgs?.[type]);
  if (editorState.textIconCacheKey === "kernel-text-v1" && hasCompleteIconSet) {
    return;
  }
  const icons = {};
  for (const type of TEXT_FORMAT_ICON_TYPES) {
    icons[type] = iconSvg.call(state.editorEngine, type);
  }
  editorState.textIconSvgs = icons;
  editorState.textIconCacheKey = "kernel-text-v1";
}

function refreshShapeToolIcons() {
  const iconSvg = state.editorEngine?.shapeToolIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const hasCompleteIconSet = SHAPE_TOOL_ICON_KINDS.every((kind) => (
    shapeToolIconStylesForKind(kind).every((style) => editorState.shapeIconSvgs?.[`${kind}:${style}`])
  ));
  if (editorState.shapeIconCacheKey === "kernel-shape-v2" && hasCompleteIconSet) {
    return;
  }
  const icons = {};
  for (const kind of SHAPE_TOOL_ICON_KINDS) {
    for (const style of shapeToolIconStylesForKind(kind)) {
      const key = `${kind}:${style}`;
      icons[key] = normalizeKernelShapeIconSvg(iconSvg.call(state.editorEngine, kind, style), key);
    }
  }
  editorState.shapeIconSvgs = icons;
  editorState.shapeIconCacheKey = "kernel-shape-v2";
}

function refreshSymbolToolIcons() {
  const iconSvg = state.editorEngine?.symbolToolIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const hasCompleteIconSet = SYMBOL_TOOL_ICON_TYPES.every((type) => editorState.symbolIconSvgs?.[type]);
  if (editorState.symbolIconCacheKey === "kernel-symbol-v1" && hasCompleteIconSet) {
    return;
  }
  const icons = {};
  for (const type of SYMBOL_TOOL_ICON_TYPES) {
    icons[type] = normalizeKernelSymbolIconSvg(iconSvg.call(state.editorEngine, type), type);
  }
  editorState.symbolIconSvgs = icons;
  editorState.symbolIconCacheKey = "kernel-symbol-v1";
}

function refreshOrbitalToolIcons() {
  const iconSvg = state.editorEngine?.orbitalToolIconSvg;
  if (typeof iconSvg !== "function") {
    return;
  }
  const hasCompleteIconSet = ORBITAL_TOOL_ICON_TEMPLATES.every((template) => (
    ORBITAL_TOOL_ICON_STYLES.every((style) => (
      ORBITAL_TOOL_ICON_PHASES.every((phase) => (
        editorState.orbitalIconSvgs?.[`${template}:${style}:${phase}`]
      ))
    ))
  ));
  if (editorState.orbitalIconCacheKey === "kernel-orbital-v1" && hasCompleteIconSet) {
    return;
  }
  const icons = {};
  for (const template of ORBITAL_TOOL_ICON_TEMPLATES) {
    for (const style of ORBITAL_TOOL_ICON_STYLES) {
      for (const phase of ORBITAL_TOOL_ICON_PHASES) {
        const key = `${template}:${style}:${phase}`;
        icons[key] = normalizeKernelOrbitalIconSvg(
          iconSvg.call(state.editorEngine, template, style, phase),
          key,
        );
      }
    }
  }
  editorState.orbitalIconSvgs = icons;
  editorState.orbitalIconCacheKey = "kernel-orbital-v1";
}

function shapeToolIconStylesForKind(kind) {
  return SHAPE_TOOL_STYLE_KINDS.includes(kind) ? SHAPE_TOOL_ICON_STYLES : ["solid"];
}

function normalizeKernelShapeIconSvg(svg, key) {
  if (!svg) {
    return "";
  }
  const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
  return addClassToSvg(svg, "cc-kernel-shape-icon")
    .replace(/\bid="([^"]+)"/g, `id="shape-icon-${safeKey}-$1"`)
    .replace(/url\(#([^)]+)\)/g, `url(#shape-icon-${safeKey}-$1)`);
}

function normalizeKernelArrowIconSvg(svg, key) {
  if (!svg) {
    return "";
  }
  const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
  return addClassToSvg(svg, "cc-kernel-arrow-icon")
    .replace(/\bid="([^"]+)"/g, `id="arrow-icon-${safeKey}-$1"`)
    .replace(/url\(#([^)]+)\)/g, `url(#arrow-icon-${safeKey}-$1)`);
}

function normalizeKernelSymbolIconSvg(svg, key) {
  if (!svg) {
    return "";
  }
  const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
  return addClassToSvg(svg, "cc-kernel-symbol-icon")
    .replace(/\bid="([^"]+)"/g, `id="symbol-icon-${safeKey}-$1"`)
    .replace(/url\(#([^)]+)\)/g, `url(#symbol-icon-${safeKey}-$1)`);
}

function normalizeKernelOrbitalIconSvg(svg, key) {
  if (!svg) {
    return "";
  }
  const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
  return addClassToSvg(svg, "cc-kernel-orbital-icon")
    .replace(/\bid="([^"]+)"/g, `id="orbital-icon-${safeKey}-$1"`)
    .replace(/url\(#([^)]+)\)/g, `url(#orbital-icon-${safeKey}-$1)`);
}

function normalizeKernelChainIconSvg(svg) {
  if (!svg) {
    return "";
  }
  return addClassToSvg(svg, "cc-kernel-chain-icon");
}

function addClassToSvg(svg, className) {
  if (/\bclass="/.test(svg)) {
    return svg.replace(/\bclass="([^"]*)"/, `class="$1 ${className}"`);
  }
  return svg.replace("<svg ", `<svg class="${className}" `);
}

function renderSecondaryToolbar() {
  if (!secondaryToolbar) {
    return;
  }
  refreshBondToolIcons();
  refreshChainToolIcon();
  refreshArrowToolIcons();
  refreshTextFormatIcons();
  refreshShapeToolIcons();
  refreshSymbolToolIcons();
  refreshOrbitalToolIcons();
  editorState.documentColors = currentDocumentColors();
  editorState.colorPalette = currentToolbarColorPalette(editorState.documentColors);
  editorState.elementPalette = currentElementPalette();
  secondaryToolbar.innerHTML = renderSecondaryToolbarHtml(editorState);
  textSymbolPalette?.setElementPayload?.(editorState.elementPalette);
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

function currentToolbarColorPalette(documentColors = []) {
  if (typeof state.editorEngine?.toolbarColorPaletteJson === "function") {
    const paletteJson = state.editorEngine.toolbarColorPaletteJson(JSON.stringify(documentColors));
    if (typeof paletteJson === "string") {
      return parseEngineJson(paletteJson, null);
    }
  }
  return null;
}

function currentElementPalette() {
  if (typeof state.editorEngine?.elementPaletteJson === "function") {
    const paletteJson = state.editorEngine.elementPaletteJson();
    if (typeof paletteJson === "string") {
      return elementPaletteWithCurrentSelection(parseEngineJson(paletteJson, null));
    }
  }
  return null;
}

function elementPaletteWithCurrentSelection(payload) {
  if (!payload || !editorState.elementSymbol) {
    return payload;
  }
  const elements = Array.isArray(payload.elements) ? payload.elements : [];
  const current = elements.find((element) => element?.symbol === editorState.elementSymbol);
  return current ? { ...payload, current } : payload;
}

function syncTextSymbolPaletteFromEngine() {
  if (typeof state.editorEngine?.textSymbolPaletteJson !== "function") {
    ensureTextSymbolPalette();
    return;
  }
  const payload = parseEngineJson(state.editorEngine.textSymbolPaletteJson(), null);
  if (!payload) {
    ensureTextSymbolPalette();
    return;
  }
  ensureTextSymbolPalette(payload);
}

function ensureTextSymbolPalette(payload = null) {
  const elementPayload = currentElementPalette();
  if (textSymbolPalette) {
    if (payload) {
      textSymbolPalette.setPayload(payload);
    }
    if (elementPayload) {
      textSymbolPalette.setElementPayload?.(elementPayload);
    }
    return;
  }
  textSymbolPalette = createTextSymbolPalette({
    mount: viewerContainer,
    payload,
    elementPayload,
    onSelect: insertTextSymbol,
    onElementSelect: selectElementFromQuickPalette,
    onModeChange: handleQuickPaletteModeChange,
  });
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

async function openTextEditorAt(point, options = {}) {
  await finishActiveTextEditor(true);
  const sessionJson = await state.editorEngine?.beginTextEdit?.(point.x, point.y);
  const session = parseEngineJson(sessionJson, null);
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
  if (editorState.elementPlacementActive === nextActive) {
    return;
  }
  editorState.elementPlacementActive = nextActive;
  syncPrimaryToolButtons(editorState, document);
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
  if (looksLikeCdxFile(file)) {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const id = `doc-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const payload = {
      dataBase64: bytesToBase64(bytes),
      fileName: file.name || null,
      filePath: null,
      format: "cdx",
    };
    localStorage.setItem(`${BROWSER_PENDING_DOCUMENT_KEY_PREFIX}${id}`, JSON.stringify(payload));
    const opened = !!window.open(browserTabUrlForPendingDocument(id), "_blank", "noopener,noreferrer");
    if (!opened) {
      localStorage.removeItem(`${BROWSER_PENDING_DOCUMENT_KEY_PREFIX}${id}`);
    }
    return opened;
  }
  const text = looksLikeCompressedChemcoreFile(file)
    ? await decompressChemcoreText(await file.arrayBuffer())
    : await file.text();
  const id = `doc-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const payload = {
    text,
    fileName: file.name || null,
    filePath: null,
    format: looksLikeCdxmlFile(file, text)
      ? "cdxml"
      : looksLikeSdfFile(file, text)
        ? "sdf"
        : saveFormatFromFileName(file.name),
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
    savedDocumentJson: freshTab.savedDocumentJson || null,
    savedRevision: freshTab.savedRevision ?? null,
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
  await closeDocumentTab(tabId, { skipUnsavedPrompt: true });
  return true;
}

async function loadDetachedDocumentPayload(payload) {
  if (!payload?.documentJson) {
    return false;
  }
  const documentData = JSON.parse(payload.documentJson);
  await loadJsonDocumentIntoEditor(documentData, payload.fileName || null, payload.filePath || null);
  if (typeof payload.savedDocumentJson === "string") {
    state.savedDocumentJson = payload.savedDocumentJson;
    state.savedRevision = Number.isFinite(Number(payload.savedRevision))
      ? Number(payload.savedRevision)
      : currentDocumentRevision();
    refreshCommandAvailability();
  }
  if (Number.isFinite(Number(payload.zoomPercent))) {
    setZoomPercent(Number(payload.zoomPercent));
  }
  saveActiveDocumentTabState();
  renderDocumentTabs();
  return true;
}

async function loadBrowserPendingDocumentPayload(payload) {
  if (payload?.format === "cdx" && payload?.dataBase64) {
    await loadCdxDocumentIntoEditor(base64ToBytes(payload.dataBase64), payload.fileName || null, payload.filePath || null);
    saveActiveDocumentTabState();
    renderDocumentTabs();
    return true;
  }
  if (!payload?.text) {
    return false;
  }
  await openDocumentText(payload.text, payload.fileName || null, payload.filePath || null, payload.format || null);
  saveActiveDocumentTabState();
  renderDocumentTabs();
  return true;
}

async function newDocumentTab() {
  await appRuntimeReady;
  await finishActiveTextEditor(true);
  const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
  saveActiveDocumentTabState();
  if (!isDesktopShell && !reuseActiveTab && openBrowserBlankDocumentTab()) {
    return;
  }
  if (!reuseActiveTab) {
    const tab = createDocumentTab();
    documentTabs.push(tab);
    activeDocumentTabId = tab.id;
    await restoreDocumentTabState(tab);
  }
  await resetEditorEngine();
  renderDocument();
  fitView();
  saveActiveDocumentTabState();
  renderDocumentTabs();
}

async function openDocumentPathInTab(path) {
  const normalizedPath = normalizeDesktopPath(path);
  void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.begin", { path, normalizedPath });
  if (!normalizedPath) {
    void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.skipInvalid", { path });
    return;
  }
  await appRuntimeReady;
  await finishActiveTextEditor(true);
  saveActiveDocumentTabState();
  const existingTab = documentTabForFilePath(normalizedPath);
  if (existingTab) {
    void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.activateExisting", {
      normalizedPath,
      tabId: existingTab.id,
    });
    await activateDocumentTab(existingTab.id);
    return;
  }
  const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
  const previousTabId = activeDocumentTabId;
  let tab = activeDocumentTab();
  void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.plan", {
    normalizedPath,
    reuseActiveTab,
    previousTabId,
    activeTabId: tab?.id || null,
  });
  if (!reuseActiveTab) {
    tab = createDocumentTab(fileNameFromPath(normalizedPath) || "Loading...");
    documentTabs.push(tab);
    activeDocumentTabId = tab.id;
    void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.createdTab", {
      normalizedPath,
      tabId: tab.id,
    });
    await restoreDocumentTabState(tab);
  }
  try {
    await openDocumentPath(normalizedPath);
    saveActiveDocumentTabState();
    renderDocumentTabs();
    void desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.ok", {
      normalizedPath,
      tabId: activeDocumentTabId,
    });
  } catch (error) {
    await desktopFileHost?.traceEvent?.("app.openDocumentPathInTab.error", {
      normalizedPath,
      tabId: tab?.id || null,
      previousTabId,
      error,
    });
    if (!reuseActiveTab) {
      await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
    }
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
  const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
  saveActiveDocumentTabState();
  const previousTabId = activeDocumentTabId;
  let tab = activeDocumentTab();
  if (!reuseActiveTab) {
    tab = createDocumentTab(file.name || "Loading...");
    documentTabs.push(tab);
    activeDocumentTabId = tab.id;
    await restoreDocumentTabState(tab);
  }
  try {
    await openDocumentFile(file);
    saveActiveDocumentTabState();
    renderDocumentTabs();
  } catch (error) {
    if (!reuseActiveTab) {
      await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
    }
    if (previousTabId && activeDocumentTabId !== previousTabId) {
      await activateDocumentTab(previousTabId);
    }
    throw error;
  }
}

async function openDroppedDocumentFileInTab(file) {
  if (!file) {
    return;
  }
  await appRuntimeReady;
  await finishActiveTextEditor(true);
  const reuseActiveTab = activeDocumentTabIsBlankUntitled() && !currentDocumentIsDirty();
  saveActiveDocumentTabState();
  const previousTabId = activeDocumentTabId;
  let tab = activeDocumentTab();
  if (!reuseActiveTab) {
    tab = createDocumentTab(file.name || "Loading...");
    documentTabs.push(tab);
    activeDocumentTabId = tab.id;
    await restoreDocumentTabState(tab);
  }
  try {
    await openDocumentFile(file);
    saveActiveDocumentTabState();
    renderDocumentTabs();
  } catch (error) {
    if (!reuseActiveTab) {
      await closeDocumentTab(tab.id, { skipUnsavedPrompt: true });
    }
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
  getZoomPercent: () => zoomPercent,
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

function isDocumentPreviewPrimitive(primitive) {
  return primitive?.role === "document-bond"
    || primitive?.role === "document-graphic"
    || primitive?.role === "document-knockout"
    || primitive?.role === "document-text";
}

function activeGestureUsesDocumentPreview() {
  if (
    activeDocumentPreviewObjectIds.size
    || activeDocumentPreviewPrimitiveElements.size
    || activeDocumentPreviewHiddenElements.size
    || activeDocumentPreviewLayer
  ) {
    return false;
  }
  if (["move", "resize", "rotate"].includes(activeSelectionGesture?.kind) && !activeSelectionGesture?.dragged) {
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

function documentPrimitiveHasSelectionAnchor(primitive) {
  if (!isDocumentPreviewPrimitive(primitive)) {
    return false;
  }
  if (primitiveBondId(primitive) || primitiveNodeId(primitive)) {
    return true;
  }
  const objectId = primitiveObjectId(primitive);
  return Boolean(objectId && (
    primitive.role === "document-graphic"
    || primitive.role === "document-text"
  ));
}

function collectCurrentDocumentSceneObjects() {
  const objects = [];
  const visit = (object) => {
    if (!object) {
      return;
    }
    objects.push(object);
    for (const child of object.children || []) {
      visit(child);
    }
  };
  for (const object of state.currentDocument?.objects || []) {
    visit(object);
  }
  return objects;
}

function currentDocumentMoleculeFragments() {
  return currentDocumentMoleculeTopology().entries
    .map((entry) => entry.fragment);
}

function currentDocumentMoleculeEntries() {
  return collectCurrentDocumentSceneObjects()
    .filter((object) => object?.type === "molecule" || object?.object_type === "molecule")
    .map((object) => {
      const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
      const fragment = resourceRef ? state.currentDocument?.resources?.[resourceRef]?.data : null;
      return fragment ? { object, fragment } : null;
    })
    .filter(Boolean);
}

function currentDocumentMoleculeTopology() {
  if (documentMoleculeTopologyCache?.document === state.currentDocument) {
    return documentMoleculeTopologyCache;
  }
  const entries = currentDocumentMoleculeEntries();
  const bondsByNode = new Map();
  for (const entry of entries) {
    const nodeById = new Map((entry.fragment.nodes || []).map((node) => [node.id, node]));
    for (const bond of entry.fragment.bonds || []) {
      const begin = worldPointForFragmentNode(entry.object, nodeById.get(bond.begin));
      const end = worldPointForFragmentNode(entry.object, nodeById.get(bond.end));
      if (!begin || !end) {
        continue;
      }
      const bondEntry = {
        key: `${entry.object.id || ""}:${bond.id}`,
        bond,
        begin,
        end,
      };
      if (bond.begin) {
        if (!bondsByNode.has(bond.begin)) {
          bondsByNode.set(bond.begin, []);
        }
        bondsByNode.get(bond.begin).push(bondEntry);
      }
      if (bond.end && bond.end !== bond.begin) {
        if (!bondsByNode.has(bond.end)) {
          bondsByNode.set(bond.end, []);
        }
        bondsByNode.get(bond.end).push(bondEntry);
      }
    }
  }
  documentMoleculeTopologyCache = {
    document: state.currentDocument,
    entries,
    bondsByNode,
  };
  return documentMoleculeTopologyCache;
}

function selectedDocumentPreviewObjectIds(selection = currentEditorEngineState()?.selection) {
  if (!selection || editorSelectionHasItems(selection) === false) {
    return [];
  }
  return [
    ...(selection.textObjects || []),
    ...(selection.arrowObjects || []),
  ];
}

function selectedStructurePreviewNodeIds(selection) {
  const nodeIds = new Set([
    ...(selection?.nodes || []),
    ...(selection?.labelNodes || []),
  ]);
  const selectedBondIds = new Set(selection?.bonds || []);
  if (!selectedBondIds.size) {
    return nodeIds;
  }
  for (const fragment of currentDocumentMoleculeFragments()) {
    for (const bond of fragment.bonds || []) {
      if (!selectedBondIds.has(bond.id)) {
        continue;
      }
      if (bond.begin) {
        nodeIds.add(bond.begin);
      }
      if (bond.end) {
        nodeIds.add(bond.end);
      }
    }
  }
  return nodeIds;
}

function activeStructurePreviewNodeIds(selection) {
  if (activeSelectionGesture?.previewNodeIds) {
    return activeSelectionGesture.previewNodeIds;
  }
  const nodeIds = selectedStructurePreviewNodeIds(selection);
  if (activeSelectionGesture) {
    activeSelectionGesture.previewNodeIds = nodeIds;
  }
  return nodeIds;
}

function selectedStructurePreviewBondIds(selection, nodeIds = selectedStructurePreviewNodeIds(selection)) {
  const bondIds = new Set(selection?.bonds || []);
  if (!nodeIds?.size || nodeIds.size < 2) {
    return bondIds;
  }
  const seen = new Set();
  const topology = currentDocumentMoleculeTopology();
  for (const nodeId of nodeIds) {
    for (const entry of topology.bondsByNode.get(nodeId) || []) {
      if (seen.has(entry.key)) {
        continue;
      }
      seen.add(entry.key);
      if (nodeIds.has(entry.bond.begin) && nodeIds.has(entry.bond.end)) {
        bondIds.add(entry.bond.id);
      }
    }
  }
  return bondIds;
}

function partiallyMovingStructureBonds(selection, nodeIds = selectedStructurePreviewNodeIds(selection)) {
  const bonds = [];
  if (!nodeIds?.size) {
    return bonds;
  }
  const seen = new Set();
  const topology = currentDocumentMoleculeTopology();
  for (const nodeId of nodeIds) {
    for (const entry of topology.bondsByNode.get(nodeId) || []) {
      if (seen.has(entry.key)) {
        continue;
      }
      seen.add(entry.key);
      const beginMoves = nodeIds.has(entry.bond.begin);
      const endMoves = nodeIds.has(entry.bond.end);
      if (beginMoves !== endMoves) {
        bonds.push({
          bond: entry.bond,
          begin: entry.begin,
          end: entry.end,
          beginMoves,
          endMoves,
        });
      }
    }
  }
  return bonds;
}

function partialMovingStructureBondPreviewState(selection, nodeIds = selectedStructurePreviewNodeIds(selection)) {
  if (activeSelectionGesture?.previewPartialBondState) {
    return activeSelectionGesture.previewPartialBondState;
  }
  const partialBonds = partiallyMovingStructureBonds(selection, nodeIds);
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  const primitives = state.coreRenderList || currentEditorRenderList();
  const primitivesByBondId = new Map();
  const elementsByBondId = new Map();
  const partialBondIds = new Set(partialBonds.map((bondPreview) => bondPreview.bond.id));
  for (const bondPreview of partialBonds) {
    const bondId = bondPreview.bond.id;
    primitivesByBondId.set(bondId, []);
    if (documentLayer) {
      const escapedBondId = CSS.escape(bondId);
      elementsByBondId.set(
        bondId,
        [...documentLayer.querySelectorAll(`[data-bond-id="${escapedBondId}"]`)],
      );
    } else {
      elementsByBondId.set(bondId, []);
    }
  }
  for (const primitive of primitives) {
    const bondId = primitiveBondId(primitive);
    if (partialBondIds.has(bondId)) {
      primitivesByBondId.get(bondId)?.push(primitive);
    }
  }
  const previewState = {
    partialBonds,
    primitivesByBondId,
    elementsByBondId,
  };
  if (activeSelectionGesture) {
    activeSelectionGesture.previewPartialBondState = previewState;
  }
  return previewState;
}

function selectedDocumentPreviewPrimitiveElements(selection = currentEditorEngineState()?.selection) {
  if (!selection || editorSelectionHasItems(selection) === false) {
    return [];
  }
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (!documentLayer) {
    return [];
  }
  const elements = new Set();
  const addElementsByDataId = (attribute, id) => {
    if (!id) {
      return;
    }
    for (const element of documentLayer.querySelectorAll(`[${attribute}="${CSS.escape(id)}"]`)) {
      elements.add(element);
    }
  };
  const nodeIds = activeStructurePreviewNodeIds(selection);
  for (const nodeId of nodeIds) {
    addElementsByDataId("data-node-id", nodeId);
  }
  const selectedBondIds = selectedStructurePreviewBondIds(selection, nodeIds);
  for (const bondId of selectedBondIds) {
    addElementsByDataId("data-bond-id", bondId);
  }
  return [...elements];
}

function selectedPreviewAnchorCount(selection) {
  if (!selection) {
    return 0;
  }
  return (selection.textObjects?.length || 0)
    + (selection.arrowObjects?.length || 0)
    + (selection.nodes?.length || 0)
    + (selection.labelNodes?.length || 0)
    + (selection.bonds?.length || 0);
}

function selectionCoversRenderedDocument(
  selection = currentEditorEngineState()?.selection,
  renderList = state.coreRenderList || currentEditorRenderList(),
) {
  if (!selection || editorSelectionHasItems(selection) === false) {
    return false;
  }
  const selectedAnchorCount = selectedPreviewAnchorCount(selection);
  if (!selectedAnchorCount) {
    return false;
  }
  let selectableCount = 0;
  for (const primitive of renderList || []) {
    if (!documentPrimitiveHasSelectionAnchor(primitive)) {
      continue;
    }
    selectableCount += 1;
    if (selectableCount > selectedAnchorCount) {
      return false;
    }
    if (!documentPrimitiveSelectedByState(primitive, selection)) {
      return false;
    }
  }
  return selectableCount > 0;
}

function documentObjectElements(objectId) {
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (!documentLayer || !objectId) {
    return [];
  }
  const escapedId = CSS.escape(objectId);
  const groups = [
    ...documentLayer.querySelectorAll(`[data-object-id="${escapedId}"][data-object-type]`),
  ];
  if (groups.length) {
    return groups;
  }
  return [...documentLayer.querySelectorAll(`[data-object-id="${escapedId}"]`)];
}

function restoreDocumentPreviewElementTransform(element) {
  if (!element) {
    return;
  }
  const baseTransform = element.dataset.previewBaseTransform;
  if (baseTransform !== undefined) {
    if (baseTransform) {
      element.setAttribute("transform", baseTransform);
    } else {
      element.removeAttribute("transform");
    }
    delete element.dataset.previewBaseTransform;
  } else {
    element.removeAttribute("transform");
  }
  element.classList.remove("is-preview-transforming");
}

function applyDocumentPreviewElementTransform(element, transform) {
  if (!element) {
    return;
  }
  if (element.dataset.previewBaseTransform === undefined) {
    element.dataset.previewBaseTransform = element.getAttribute("transform") || "";
  }
  const baseTransform = element.dataset.previewBaseTransform;
  element.setAttribute("transform", baseTransform ? `${transform} ${baseTransform}` : transform);
  element.classList.add("is-preview-transforming");
}

function hideDocumentPreviewElement(element) {
  if (!element) {
    return;
  }
  if (element.dataset.previewBaseVisibility === undefined) {
    element.dataset.previewBaseVisibility = element.style.visibility || "";
  }
  element.style.visibility = "hidden";
  activeDocumentPreviewHiddenElements.add(element);
}

function restoreDocumentPreviewElementVisibility(element) {
  if (!element) {
    return;
  }
  if (element.dataset.previewBaseVisibility !== undefined) {
    element.style.visibility = element.dataset.previewBaseVisibility;
    delete element.dataset.previewBaseVisibility;
  } else {
    element.style.visibility = "";
  }
}

function clearDocumentPartialBondPreview() {
  viewerSvg.querySelectorAll('[data-layer="document-partial-bond-preview"]').forEach((layer) => layer.remove());
  for (const element of activeDocumentPreviewHiddenElements) {
    restoreDocumentPreviewElementVisibility(element);
  }
  activeDocumentPreviewHiddenElements = new Set();
  activeDocumentEditPreviewHiddenElements = new Set();
}

function activeGestureUsesObjectEditPreview() {
  return ["arrow-endpoint", "arrow-curve", "shape-resize"].includes(activeSelectionGesture?.kind);
}

function objectIdsFromDocumentPreviewPrimitives(renderList = []) {
  const ids = new Set();
  for (const primitive of renderList || []) {
    const objectId = primitiveObjectId(primitive);
    if (objectId && isDocumentPreviewPrimitive(primitive)) {
      ids.add(objectId);
    }
  }
  return ids;
}

function syncObjectEditPreviewHiddenElements(renderList = []) {
  const nextElements = new Set();
  if (activeGestureUsesObjectEditPreview()) {
    for (const objectId of objectIdsFromDocumentPreviewPrimitives(renderList)) {
      for (const element of documentObjectElements(objectId)) {
        hideDocumentPreviewElement(element);
        nextElements.add(element);
      }
    }
  }
  for (const element of activeDocumentEditPreviewHiddenElements) {
    if (!nextElements.has(element)) {
      restoreDocumentPreviewElementVisibility(element);
      activeDocumentPreviewHiddenElements.delete(element);
    }
  }
  activeDocumentEditPreviewHiddenElements = nextElements;
}

function commitDocumentPartialBondPreview() {
  const layer = viewerSvg.querySelector('[data-layer="document-partial-bond-preview"]');
  if (!layer) {
    return false;
  }
  const previewByBondId = new Map();
  for (const preview of [...layer.children]) {
    const bondId = preview.getAttribute("data-bond-id") || "";
    if (!bondId) {
      continue;
    }
    if (!previewByBondId.has(bondId)) {
      previewByBondId.set(bondId, []);
    }
    previewByBondId.get(bondId).push(preview.cloneNode(true));
  }
  const originalsByBondId = new Map();
  for (const element of activeDocumentPreviewHiddenElements) {
    const bondId = element.getAttribute?.("data-bond-id") || "";
    if (!bondId || !previewByBondId.has(bondId)) {
      continue;
    }
    if (!originalsByBondId.has(bondId)) {
      originalsByBondId.set(bondId, []);
    }
    originalsByBondId.get(bondId).push(element);
  }
  const committed = new Set();
  for (const [bondId, originals] of originalsByBondId) {
    const previews = previewByBondId.get(bondId) || [];
    const anchor = originals.find((element) => element.parentNode);
    if (!anchor?.parentNode || !previews.length) {
      continue;
    }
    for (const preview of previews) {
      anchor.parentNode.insertBefore(preview, anchor);
    }
    for (const original of originals) {
      committed.add(original);
      original.remove();
    }
  }
  layer.remove();
  if (!committed.size) {
    return false;
  }
  activeDocumentPreviewHiddenElements = new Set(
    [...activeDocumentPreviewHiddenElements].filter((element) => !committed.has(element)),
  );
  return true;
}

function removeDocumentPreviewBatchLayer() {
  activeDocumentPreviewBatchLayer?.remove();
  activeDocumentPreviewBatchLayer = null;
}

function clearDocumentObjectPreviewTransform() {
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  clearDocumentPartialBondPreview();
  removeDocumentPreviewBatchLayer();
  if (activeDocumentPreviewLayer) {
    documentLayer?.removeAttribute("transform");
    activeDocumentPreviewLayer = false;
  }
  if (!activeDocumentPreviewObjectIds.size && !activeDocumentPreviewPrimitiveElements.size) {
    activeDocumentPreviewTransform = "";
    return;
  }
  for (const objectId of activeDocumentPreviewObjectIds) {
    for (const element of documentObjectElements(objectId)) {
      restoreDocumentPreviewElementTransform(element);
    }
  }
  activeDocumentPreviewObjectIds = new Set();
  for (const element of activeDocumentPreviewPrimitiveElements) {
    restoreDocumentPreviewElementTransform(element);
  }
  activeDocumentPreviewPrimitiveElements = new Set();
  activeDocumentPreviewTransform = "";
}

function commitDocumentPreviewElementTransform(element, transform = activeDocumentPreviewTransform) {
  if (!element || !transform) {
    return;
  }
  const baseTransform = element.dataset.previewBaseTransform;
  const committedTransform = baseTransform !== undefined
    ? element.getAttribute("transform") || (baseTransform ? `${transform} ${baseTransform}` : transform)
    : transform
      ? `${transform}${element.getAttribute("transform") ? ` ${element.getAttribute("transform")}` : ""}`
      : element.getAttribute("transform") || "";
  if (committedTransform) {
    element.setAttribute("transform", committedTransform);
  } else {
    element.removeAttribute("transform");
  }
  delete element.dataset.previewBaseTransform;
  element.classList.remove("is-preview-transforming");
}

function commitDocumentObjectPreviewTransform() {
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  const transform = activeDocumentPreviewTransform;
  if (!transform) {
    clearDocumentObjectPreviewTransform();
    return false;
  }
  commitDocumentPartialBondPreview();
  removeDocumentPreviewBatchLayer();
  if (activeDocumentPreviewLayer) {
    for (const child of [...documentLayer?.children || []]) {
      commitDocumentPreviewElementTransform(child, transform);
    }
    documentLayer?.removeAttribute("transform");
    activeDocumentPreviewLayer = false;
  }
  for (const objectId of activeDocumentPreviewObjectIds) {
    for (const element of documentObjectElements(objectId)) {
      commitDocumentPreviewElementTransform(element, transform);
    }
  }
  for (const element of activeDocumentPreviewPrimitiveElements) {
    restoreDocumentPreviewElementVisibility(element);
    commitDocumentPreviewElementTransform(element, transform);
  }
  activeDocumentPreviewObjectIds = new Set();
  activeDocumentPreviewPrimitiveElements = new Set();
  activeDocumentPreviewTransform = "";
  return true;
}

function canCommitDocumentObjectPreviewTransform() {
  return !!activeDocumentPreviewTransform
    && (
      !!activeDocumentPreviewBatchLayer?.isConnected
      || activeDocumentPreviewLayer
      || activeDocumentPreviewObjectIds.size > 0
      || activeDocumentPreviewPrimitiveElements.size > 0
      || !!viewerSvg.querySelector('[data-layer="document-partial-bond-preview"]')
    );
}

function selectionGestureDelta(gesture) {
  if (gesture?.kind !== "move") {
    return null;
  }
  return {
    x: (gesture.current?.x ?? gesture.start?.x ?? 0) - (gesture.start?.x ?? 0),
    y: (gesture.current?.y ?? gesture.start?.y ?? 0) - (gesture.start?.y ?? 0),
  };
}

function addPointDelta(point, delta, weight = 1) {
  return {
    ...point,
    x: Number(point?.x || 0) + delta.x * weight,
    y: Number(point?.y || 0) + delta.y * weight,
  };
}

function partialBondPointMoveWeight(point, bondPreview) {
  const dx = bondPreview.end.x - bondPreview.begin.x;
  const dy = bondPreview.end.y - bondPreview.begin.y;
  const len2 = dx * dx + dy * dy;
  if (len2 <= 0.000001) {
    return 1;
  }
  const t = Math.max(0, Math.min(1, (
    ((Number(point?.x || 0) - bondPreview.begin.x) * dx)
    + ((Number(point?.y || 0) - bondPreview.begin.y) * dy)
  ) / len2));
  if (bondPreview.beginMoves) {
    return t <= 0.5 ? 1 : 0;
  }
  return t >= 0.5 ? 1 : 0;
}

function movePartialBondPreviewPoint(point, bondPreview, delta) {
  return addPointDelta(point, delta, partialBondPointMoveWeight(point, bondPreview));
}

function translatedPartialBondPrimitive(primitive, bondPreview, delta) {
  if (!primitive) {
    return null;
  }
  if (primitive.kind === "line" && primitive.from && primitive.to) {
    return {
      ...primitive,
      from: movePartialBondPreviewPoint(primitive.from, bondPreview, delta),
      to: movePartialBondPreviewPoint(primitive.to, bondPreview, delta),
    };
  }
  if ((primitive.kind === "polyline" || primitive.kind === "polygon") && Array.isArray(primitive.points)) {
    return {
      ...primitive,
      points: primitive.points.map((point) => movePartialBondPreviewPoint(point, bondPreview, delta)),
    };
  }
  if ((primitive.kind === "circle" || primitive.kind === "ellipse") && primitive.center) {
    return {
      ...primitive,
      center: movePartialBondPreviewPoint(primitive.center, bondPreview, delta),
    };
  }
  return null;
}

function ensureDocumentPartialBondPreviewLayer() {
  let layer = viewerSvg.querySelector('[data-layer="document-partial-bond-preview"]');
  if (layer) {
    layer.replaceChildren();
    return layer;
  }
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (!documentLayer) {
    return null;
  }
  layer = makeSvgNode("g", {
    "data-layer": "document-partial-bond-preview",
    "pointer-events": "none",
  });
  const editorOverlay = viewerSvg.querySelector('[data-layer="editor-overlay"]');
  if (editorOverlay) {
    viewerSvg.insertBefore(layer, editorOverlay);
  } else {
    viewerSvg.insertBefore(layer, documentLayer.nextSibling);
  }
  return layer;
}

function ensureDocumentPreviewBatchLayer() {
  if (activeDocumentPreviewBatchLayer?.isConnected) {
    return activeDocumentPreviewBatchLayer;
  }
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (!documentLayer) {
    return null;
  }
  const layer = makeSvgNode("g", {
    "data-layer": "document-batch-preview",
    "pointer-events": "none",
  });
  const partialPreviewLayer = viewerSvg.querySelector('[data-layer="document-partial-bond-preview"]');
  const editorOverlay = viewerSvg.querySelector('[data-layer="editor-overlay"]');
  if (partialPreviewLayer) {
    viewerSvg.insertBefore(layer, partialPreviewLayer);
  } else if (editorOverlay) {
    viewerSvg.insertBefore(layer, editorOverlay);
  } else {
    viewerSvg.insertBefore(layer, documentLayer.nextSibling);
  }
  activeDocumentPreviewBatchLayer = layer;
  return layer;
}

function applyDocumentPrimitiveBatchPreview(primitiveElements, transform) {
  if (!transform || primitiveElements.length < DOCUMENT_PREVIEW_BATCH_ELEMENT_THRESHOLD) {
    return false;
  }
  const layer = ensureDocumentPreviewBatchLayer();
  if (!layer) {
    return false;
  }
  if (layer.dataset.sourceCount !== String(primitiveElements.length)) {
    layer.replaceChildren();
    for (const element of primitiveElements) {
      hideDocumentPreviewElement(element);
      layer.appendChild(element.cloneNode(true));
    }
    layer.dataset.sourceCount = String(primitiveElements.length);
  }
  layer.setAttribute("transform", transform);
  activeDocumentPreviewObjectIds = new Set();
  activeDocumentPreviewPrimitiveElements = new Set(primitiveElements);
  activeDocumentPreviewTransform = transform;
  return true;
}

function renderFallbackPartialBondPreview(layer, bondPreview, delta, primitive = null) {
  const begin = bondPreview.beginMoves ? addPointDelta(bondPreview.begin, delta) : bondPreview.begin;
  const end = bondPreview.endMoves ? addPointDelta(bondPreview.end, delta) : bondPreview.end;
  layer.appendChild(makeSvgNode("line", {
    x1: begin.x,
    y1: begin.y,
    x2: end.x,
    y2: end.y,
    stroke: primitive?.stroke || "#000000",
    "stroke-width": primitiveStrokeWidthValue(primitive, editorBondStrokeWidth()),
    "stroke-linecap": "round",
    "data-role": "document-bond",
    "data-bond-id": bondPreview.bond.id,
  }));
}

function renderPartialMovingStructureBondPreview(previewState, delta) {
  if (!previewState?.partialBonds?.length || !delta) {
    return;
  }
  const previewLayer = ensureDocumentPartialBondPreviewLayer();
  if (!previewLayer) {
    return;
  }
  for (const bondPreview of previewState.partialBonds) {
    const bondId = bondPreview.bond.id;
    for (const element of previewState.elementsByBondId.get(bondId) || []) {
      hideDocumentPreviewElement(element);
    }
    const bondPrimitives = previewState.primitivesByBondId.get(bondId) || [];
    if (!bondPrimitives.length) {
      renderFallbackPartialBondPreview(previewLayer, bondPreview, delta);
      continue;
    }
    let rendered = false;
    for (const primitive of bondPrimitives) {
      const translated = translatedPartialBondPrimitive(primitive, bondPreview, delta);
      if (translated) {
        renderCorePrimitive(previewLayer, translated, corePrimitiveRenderOptions());
        rendered = true;
      }
    }
    if (!rendered) {
      renderFallbackPartialBondPreview(previewLayer, bondPreview, delta, bondPrimitives[0]);
    }
  }
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
  const selection = activeSelectionGesture.previewSelection
    || currentEditorEngineState()?.selection;
  activeSelectionGesture.previewSelection = selection;
  const nodeIds = activeStructurePreviewNodeIds(selection);
  const partialBondState = partialMovingStructureBondPreviewState(selection, nodeIds);
  const hasPartialBonds = partialBondState.partialBonds.length > 0;
  const partialBondDelta = hasPartialBonds ? selectionGestureDelta(activeSelectionGesture) : null;
  if (hasPartialBonds && !partialBondDelta) {
    clearDocumentObjectPreviewTransform();
    return false;
  }
  const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
  if (activeSelectionGesture.previewUsesLayer && !hasPartialBonds) {
    if (!documentLayer) {
      clearDocumentObjectPreviewTransform();
      return false;
    }
    documentLayer.setAttribute("transform", transform);
    activeDocumentPreviewLayer = true;
    activeDocumentPreviewObjectIds = new Set();
    activeDocumentPreviewPrimitiveElements = new Set();
    activeDocumentPreviewTransform = transform;
    return true;
  }
  if (!hasPartialBonds && selectionCoversRenderedDocument(selection)) {
    if (!documentLayer) {
      clearDocumentObjectPreviewTransform();
      return false;
    }
    for (const objectId of activeDocumentPreviewObjectIds) {
      for (const element of documentObjectElements(objectId)) {
        restoreDocumentPreviewElementTransform(element);
      }
    }
    for (const element of activeDocumentPreviewPrimitiveElements) {
      restoreDocumentPreviewElementTransform(element);
    }
    documentLayer.setAttribute("transform", transform);
    activeDocumentPreviewLayer = true;
    activeDocumentPreviewObjectIds = new Set();
    activeDocumentPreviewPrimitiveElements = new Set();
    activeDocumentPreviewTransform = transform;
    activeSelectionGesture.previewUsesLayer = true;
    return true;
  }
  const hasCachedObjectIds = Array.isArray(activeSelectionGesture.previewObjectIds);
  const hasCachedPrimitiveElements = Array.isArray(activeSelectionGesture.previewPrimitiveElements);
  const objectIds = activeSelectionGesture.previewObjectIds || selectedDocumentPreviewObjectIds(selection);
  const primitiveElements = activeSelectionGesture.previewPrimitiveElements || selectedDocumentPreviewPrimitiveElements(selection);
  if (!objectIds.length && !primitiveElements.length) {
    if (hasPartialBonds) {
      activeDocumentPreviewObjectIds = new Set();
      activeDocumentPreviewPrimitiveElements = new Set();
      activeDocumentPreviewTransform = transform;
      renderPartialMovingStructureBondPreview(partialBondState, partialBondDelta);
      return true;
    }
    clearDocumentObjectPreviewTransform();
    return false;
  }
  activeSelectionGesture.previewObjectIds = objectIds;
  activeSelectionGesture.previewPrimitiveElements = primitiveElements;
  if (
    !objectIds.length
    && applyDocumentPrimitiveBatchPreview(primitiveElements, transform)
  ) {
    if (activeDocumentPreviewLayer) {
      documentLayer?.removeAttribute("transform");
      activeDocumentPreviewLayer = false;
    }
    if (hasPartialBonds) {
      renderPartialMovingStructureBondPreview(partialBondState, partialBondDelta);
    } else {
      clearDocumentPartialBondPreview();
    }
    return true;
  }
  removeDocumentPreviewBatchLayer();
  const nextIds = new Set(objectIds);
  const nextPrimitiveElements = new Set(primitiveElements);
  const allGroups = hasCachedObjectIds || hasCachedPrimitiveElements
    ? []
    : [...viewerSvg.querySelectorAll('[data-layer="document-content"] [data-object-id][data-object-type]')];
  const canTransformLayer = !hasCachedObjectIds
    && !hasCachedPrimitiveElements
    && allGroups.length > 0
    && nextPrimitiveElements.size === 0
    && nextIds.size === allGroups.length
    && allGroups.every((group) => nextIds.has(group.dataset.objectId));
  if (canTransformLayer) {
    for (const objectId of activeDocumentPreviewObjectIds) {
      for (const element of documentObjectElements(objectId)) {
        restoreDocumentPreviewElementTransform(element);
      }
    }
    for (const element of activeDocumentPreviewPrimitiveElements) {
      restoreDocumentPreviewElementTransform(element);
    }
    if (!documentLayer) {
      clearDocumentObjectPreviewTransform();
      return false;
    }
    documentLayer.setAttribute("transform", transform);
    activeDocumentPreviewLayer = true;
    activeDocumentPreviewObjectIds = new Set();
    activeDocumentPreviewPrimitiveElements = new Set();
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
      for (const element of documentObjectElements(objectId)) {
        restoreDocumentPreviewElementTransform(element);
      }
    }
  }
  for (const element of activeDocumentPreviewPrimitiveElements) {
    if (!nextPrimitiveElements.has(element)) {
      restoreDocumentPreviewElementTransform(element);
    }
  }
  const nextObjectElements = new Map();
  for (const objectId of nextIds) {
    const elements = documentObjectElements(objectId);
    if (!elements.length) {
      clearDocumentObjectPreviewTransform();
      return false;
    }
    nextObjectElements.set(objectId, elements);
  }
  for (const elements of nextObjectElements.values()) {
    for (const element of elements) {
      applyDocumentPreviewElementTransform(element, transform);
    }
  }
  for (const element of nextPrimitiveElements) {
    applyDocumentPreviewElementTransform(element, transform);
  }
  activeDocumentPreviewObjectIds = nextIds;
  activeDocumentPreviewPrimitiveElements = nextPrimitiveElements;
  activeDocumentPreviewTransform = transform;
  if (hasPartialBonds) {
    renderPartialMovingStructureBondPreview(partialBondState, partialBondDelta);
  } else {
    clearDocumentPartialBondPreview();
  }
  return true;
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

viewerSvg?.addEventListener("pointermove", editorPointerController.handleEditorPointerMove);
viewerSvg?.addEventListener("pointerdown", editorPointerController.handleEditorPointerDown);
viewerSvg?.addEventListener("pointerup", editorPointerController.handleEditorPointerUp);
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
  activeDocumentPreviewObjectIds = new Set();
  activeDocumentPreviewPrimitiveElements = new Set();
  activeDocumentPreviewHiddenElements = new Set();
  activeDocumentEditPreviewHiddenElements = new Set();
  activeDocumentPreviewLayer = false;
  activeDocumentPreviewBatchLayer = null;
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
    let targetZoom = zoomPercent;
    let targetScale = viewportScaleForZoom(targetZoom);
    let metrics = editorViewportMetrics(targetScale);
    for (let index = 0; index < 3; index += 1) {
      const candidateFitBox = paddedViewBoxFromBounds(bounds, metrics.fitPaddingX, metrics.fitPaddingY);
      const nextZoom = fitZoomPercentForViewBox(candidateFitBox);
      if (nextZoom === targetZoom && index > 0) {
        fitTargetBox = candidateFitBox;
        break;
      }
      targetZoom = nextZoom;
      targetScale = viewportScaleForZoom(targetZoom);
      metrics = editorViewportMetrics(targetScale);
      fitTargetBox = paddedViewBoxFromBounds(bounds, metrics.fitPaddingX, metrics.fitPaddingY);
    }
    nextViewBox = editorCanvasViewBoxFromBounds(bounds, targetScale);
    zoomPercent = targetZoom;
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
  resolveAppInitialDocumentReady?.();
} catch (error) {
  resolveAppInitialDocumentReady?.();
  viewerTitle.textContent = "Runtime load failed";
  viewerStats.textContent = "";
  docMeta.textContent = String(error);
  viewerSvg.innerHTML = "";
}
