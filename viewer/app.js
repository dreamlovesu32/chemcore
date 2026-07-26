import {
  parseEngineJson,
  primitivesForObject,
  renderBoundsFromEngine,
  renderListFromEngine,
} from "./engine_bridge.js";
import { createAppDomRefs } from "./app_dom.js";
import { registerChemSemaDebug } from "./app_debug.js";
import { createColorHost } from "./color_host.js";
import { createCanvasPreviewHost } from "./canvas_preview_host.js";
import { createObjectSettingsHost } from "./object_settings_host.js";
import { createNumericDialogHost } from "./numeric_dialog_host.js";
import { createAtomPropertyDialogHost } from "./atom_property_dialog_host.js";
import { createChemicalPropertyDialogHost } from "./chemical_property_dialog_host.js";
import { createAnnotationDialogHost } from "./annotation_dialog_host.js";
import { createSmilesDialogHost } from "./smiles_dialog_host.js";
import { createTableDialogHost } from "./table_dialog_host.js";
import { createTransientNotificationHost } from "./transient_notification_host.js";
import { createUiActionRunner } from "./ui_action_runner.js";
import { createInchiHost } from "./inchi_host.js";
import { createNmrPredictionHost } from "./nmr_prediction_host.js";
import { createBundledNmrProvider } from "./nmr_prediction_provider.js";
import { createImageImportHost } from "./image_import_host.js";
import { createDesktopFileHost, normalizeDesktopPath } from "./desktop_file_host.js";
import { createEngineHost } from "./engine_host.js?v=20260726-molecular-coloring-2";
import { bindEditorControls, openColorDialog } from "./editor_bindings.js?v=20260627-browser-drop-tabs";
import { createDocumentFlow } from "./document_flow.js";
import { createBrowserDocumentTabs } from "./browser_document_tabs.js";
import { createDocumentTabInteractions } from "./document_tab_interactions.js";
import { createDocumentTabStateHost } from "./document_tab_state_host.js";
import { createAppWindowLifecycleHost } from "./app_window_lifecycle.js";
import { pointDistance } from "./geometry.js";
import { createBracketHitGeometry } from "./bracket_hit_geometry.js";
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
import { createEditorOverlayRenderer } from "./editor_overlay.js?v=20260723-image-focus";
import { createEditorSelectionState } from "./editor_selection_state.js";
import { createEditorDocumentRenderer } from "./editor_document_renderer.js";
import { createEditorReadHost } from "./editor_read_host.js";
import { createEditorRuntimeHost } from "./editor_runtime_host.js";
import { createEditorStateRuntimeHost } from "./editor_state_runtime_host.js";
import { createEditorToolbarHost } from "./editor_toolbar_host.js";
import { createEditorViewportHost } from "./editor_viewport_host.js";
import { createEditorPointerController } from "./editor_pointer_controller.js?v=20260629-deep-stability";
import { createCanvasContextMenuHost } from "./editor_context_menu.js?v=20260726-molecular-coloring";
import { createEditorCommandController } from "./editor_command_controller.js";
import { createEditorCommandEngine } from "./editor_command_engine.js?v=20260626-interaction-feedback";
import {
  editorScriptScale as computeEditorScriptScale,
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
import { createTextEditorLayoutHost } from "./text_editor_layout_host.js";
import { createTextEditCommitHost } from "./text_edit_commit_host.js";
import { createTextEditActionHost } from "./text_edit_action_host.js";
import {
  primitiveStrokeWidthValue,
  renderCorePrimitive,
} from "./primitive_dom_renderer.js?v=20260726-molecular-coloring";
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

const {
  rotatePointAround,
  pointToSegmentDistance,
  bracketPairLip,
  bracketPairDepth,
  bracketStrokeHitPadding,
  bracketSideHandleX,
  squareBracketSideLocalHit,
  roundBracketSidePolyline,
  cubicPoint,
  appendCubicSamples,
  curlyBracketSidePolyline,
  pointToPolylineDistance,
  bracketSideLocalHit,
  bracketPairLocalHit,
} = createBracketHitGeometry({ pointDistance });

const SAMPLE_FILES = [];

const VIEW_MODE = document.body.dataset.viewMode || "document";
const LABEL_DEBUG_MODE = VIEW_MODE === "label-debug";

const state = {
  currentPath: LABEL_DEBUG_MODE ? SAMPLE_FILES[0] : null,
  currentFileName: null,
  currentFilePath: null,
  savedDocumentJson: null,
  savedRevision: null,
  unsavedDocument: false,
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
const atomPropertyDialogHost = createAtomPropertyDialogHost({
  root: document.body,
  engine: () => state.editorEngine,
});
const chemicalPropertyDialogHost = createChemicalPropertyDialogHost({
  root: document.body,
  engine: () => state.editorEngine,
});
const annotationDialogHost = createAnnotationDialogHost({
  root: document.body,
  engine: () => state.editorEngine,
});
const smilesDialogHost = createSmilesDialogHost({
  root: document.body,
  commandEngine,
  onApply: async (result) => {
    renderDocumentChange(result);
  },
});
const tableDialogHost = createTableDialogHost({
  root: document.body,
  engine: () => state.editorEngine,
  commandEngine,
  onApply: async (result) => {
    renderDocumentChange(result);
  },
});
const transientNotificationHost = createTransientNotificationHost({
  root: document.body,
});
const uiActions = createUiActionRunner({
  isAbortError: (error) => isAbortError(error),
  notify: (message) => transientNotificationHost.show(message, {
    error: true,
    duration: 3600,
  }),
  trace: (scope, detail) => desktopFileHost?.traceEvent?.(scope, detail),
});
const inchiHost = createInchiHost();
const bundledNmrProvider = createBundledNmrProvider();
const nmrPredictionHost = createNmrPredictionHost({
  engine: () => state.editorEngine,
  provider: () => bundledNmrProvider,
  openDocumentTab: (documentData, title) => openGeneratedDocumentTab(documentData, title),
});
const isDesktopShell = !!desktopFileHost?.usesCustomWindowChrome;
const isNativeFrameShell = !!desktopFileHost?.available && !isDesktopShell;
let sharedGlyphProfiles = null;
const sharedGlyphProfilesReady = loadSharedGlyphProfiles();

document.body.classList.toggle("desktop-shell", isDesktopShell);
document.body.classList.toggle("native-frame-shell", isNativeFrameShell);
document.body.classList.toggle("browser-shell", !desktopFileHost?.available);

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
  imageFileInput,
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
registerChemSemaDebug({
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
  clientPointToWorld(x, y) {
    return clientPointToWorld(x, y);
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
  sampleSelect.addEventListener("change", uiActions.listener("sample.change", async (event) => {
    state.currentPath = event.target.value;
    await loadAndRender();
  }));
}

reloadButton?.addEventListener("click", uiActions.listener("document.reload", async () => {
  await loadAndRender();
}));

fitButton?.addEventListener("click", () => {
  fitView();
});

toggleMolecules?.addEventListener("change", () => renderDocument());
toggleLines?.addEventListener("change", () => renderDocument());
toggleTexts?.addEventListener("change", () => renderDocument());

const documentTabs = [];
let activeDocumentTabId = null;
let activeTextEditor = null;
let pendingImageInsertWorldPoint = null;
let documentTabInteractions = null;
let documentTabStateHost = null;
let textEditorLayoutHost = null;
let textEditCommitHost = null;
let editorRuntimeHost = null;
let textEditActionHost = null;
let textEditorController = null;
let canvasPreviewHost = null;
let editorStateRuntimeHost = null;
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
  document.title = `${displayTitle} - ChemSema`;
  desktopFileHost?.setWindowTitle?.(displayTitle).catch?.(() => {});
};

if (viewerTitle) {
  new MutationObserver(syncWindowTitle).observe(viewerTitle, {
    childList: true,
    characterData: true,
    subtree: true,
  });
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
  textOutline: false,
  textShadow: false,
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
  chromatographyKind: "tlc-plate",
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
  "unsavedDocument",
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

function escapeHtml(value) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

documentTabStateHost = createDocumentTabStateHost({
  state,
  documentTabs,
  documentTabsRoot,
  tabStateKeys: TAB_STATE_KEYS,
  getActiveDocumentTabId: () => activeDocumentTabId,
  setActiveDocumentTabId: (value) => { activeDocumentTabId = value; },
  getActiveTextEditor: () => activeTextEditor,
  setActiveTextEditor: (value) => { activeTextEditor = value; },
  setActiveSelectionGesture: (value) => { activeSelectionGesture = value; },
  getDocumentTabInteractions: () => documentTabInteractions,
  textEditorLayer,
  getZoomPercent: (...args) => getZoomPercent(...args),
  setStoredZoomPercent: (...args) => setStoredZoomPercent(...args),
  syncWindowTitle: (...args) => syncWindowTitle(...args),
  syncEngineToolState: (...args) => syncEngineToolState(...args),
  syncZoomControl: (...args) => syncZoomControl(...args),
  renderSecondaryToolbar: (...args) => renderSecondaryToolbar(...args),
  renderDocument: (...args) => renderDocument(...args),
  refreshCommandAvailability: (...args) => refreshCommandAvailability(...args),
  finishActiveTextEditor: (...args) => finishActiveTextEditor(...args),
  desktopFileHost,
  syncDocumentFromEngine: (...args) => syncDocumentFromEngine(...args),
  scheduleDeferredDocumentSync: (...args) => scheduleDeferredDocumentSync(...args),
  confirmCloseDocumentTab: (...args) => confirmCloseDocumentTab(...args),
  resetEditorEngine: (...args) => resetEditorEngine(...args),
  fitView: (...args) => fitView(...args),
  escapeHtml,
});

function activeDocumentTab(...args) { return documentTabStateHost.activeDocumentTab(...args); }
function createDocumentTab(...args) { return documentTabStateHost.createDocumentTab(...args); }
function ensureDocumentTab(...args) { return documentTabStateHost.ensureDocumentTab(...args); }
function saveActiveDocumentTabState(...args) { return documentTabStateHost.saveActiveDocumentTabState(...args); }
function restoreDocumentTabState(...args) { return documentTabStateHost.restoreDocumentTabState(...args); }
function documentTitleFromState(...args) { return documentTabStateHost.documentTitleFromState(...args); }
function documentTitleWithDirtyMarker(...args) { return documentTabStateHost.documentTitleWithDirtyMarker(...args); }
function currentDocumentSaveFingerprint(...args) { return documentTabStateHost.currentDocumentSaveFingerprint(...args); }
function currentDocumentRevision(...args) { return documentTabStateHost.currentDocumentRevision(...args); }
function markCurrentDocumentSaved(...args) { return documentTabStateHost.markCurrentDocumentSaved(...args); }
function activeTextEditorIsDirty(...args) { return documentTabStateHost.activeTextEditorIsDirty(...args); }
function closeActiveTextEditorForToolAction(...args) { return documentTabStateHost.closeActiveTextEditorForToolAction(...args); }
function currentDocumentIsDirty(...args) { return documentTabStateHost.currentDocumentIsDirty(...args); }
function canSaveCurrentDocument(...args) { return documentTabStateHost.canSaveCurrentDocument(...args); }
function isOleEditFilePath(...args) { return documentTabStateHost.isOleEditFilePath(...args); }
function markCurrentDocumentOfficeSynced(...args) { return documentTabStateHost.markCurrentDocumentOfficeSynced(...args); }
function documentTabIsDirty(...args) { return documentTabStateHost.documentTabIsDirty(...args); }
function syncOleEditDocumentTabToOffice(...args) { return documentTabStateHost.syncOleEditDocumentTabToOffice(...args); }
function autoSaveAllOleEditDocumentTabs(...args) { return documentTabStateHost.autoSaveAllOleEditDocumentTabs(...args); }
function handleDocumentCommandCommitted(...args) { return documentTabStateHost.handleDocumentCommandCommitted(...args); }
function fileNameFromPath(...args) { return documentTabStateHost.fileNameFromPath(...args); }
function documentTabForFilePath(...args) { return documentTabStateHost.documentTabForFilePath(...args); }
function updateActiveDocumentTabTitle(...args) { return documentTabStateHost.updateActiveDocumentTabTitle(...args); }
function renderDocumentTabs(...args) { return documentTabStateHost.renderDocumentTabs(...args); }
function activateDocumentTab(...args) { return documentTabStateHost.activateDocumentTab(...args); }
function closeDocumentTab(...args) { return documentTabStateHost.closeDocumentTab(...args); }

documentTabInteractions = createDocumentTabInteractions({
  root: documentTabsRoot,
  titlebar: desktopTitlebar,
  detachEnabled: () => isDesktopShell,
  uiActions,
  closeDocumentTab,
  activateDocumentTab,
  detachDocumentTab: (...args) => detachDocumentTab(...args),
});
documentTabInteractions.bind();
syncWindowTitle();

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
  uiActions,
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
  const shapeKind = editorState.activeTool === "tlc-plate"
    ? editorState.chromatographyKind
    : editorState.shapeKind;
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

const editorReadHost = createEditorReadHost({
  state,
  editorEngineReadCache,
  parseEngineJson,
  isEditingRustDocument,
  renderBoundsFromEngine,
  renderListFromEngine,
  resetDocumentEngine: (...args) => resetDocumentEngine(...args),
  maybeAutoExpandEditorViewport: (...args) => maybeAutoExpandEditorViewport(...args),
  isDocumentPreviewPrimitive: (...args) => isDocumentPreviewPrimitive(...args),
  primitivesForObject,
});
const {
  currentEditableFragmentData,
  worldPointForFragmentPosition,
  worldPointForFragmentNode,
  selectionZoomCenterWorld,
  editorEngineRevision,
  currentEditorEngineReadCache,
  currentEditorEngineState,
  currentEditorDocumentData,
  currentEditorRenderList,
  currentEditorInteractionRenderList,
  currentEditorRenderBounds,
  currentRenderBounds,
  syncCoreRenderListFromCurrentDocument,
  syncEditorRenderListFromEngine,
  syncEditorSelectionRenderListFromEngine,
  currentEditorOverlayRenderList,
  currentSelectionItemCount,
  freshestPreviewSelection,
  corePrimitivesForObject,
} = editorReadHost;

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
function clientPointToWorld(...args) { return editorViewportHost.clientPointToWorld(...args); }
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
function clearDocumentBondCreationPreview(...args) { return editorDocumentRenderer.clearDocumentBondCreationPreview(...args); }
function commitDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.commitDocumentObjectPreviewTransform(...args); }
function canCommitDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.canCommitDocumentObjectPreviewTransform(...args); }
function applyDocumentObjectPreviewTransform(...args) { return editorDocumentRenderer.applyDocumentObjectPreviewTransform(...args); }
function applyDocumentBondCreationPreview(...args) { return editorDocumentRenderer.applyDocumentBondCreationPreview(...args); }
function hideDocumentDiagnosticsForPreview(...args) { return editorDocumentRenderer.hideDocumentDiagnosticsForPreview(...args); }
function resetDocumentRenderState(...args) { return editorDocumentRenderer.resetDocumentRenderState(...args); }

canvasPreviewHost = createCanvasPreviewHost({
  activeGestureUsesObjectEditPreview,
  currentEditorInteractionRenderList,
  syncObjectEditPreviewHiddenElements,
  editorOverlayRenderer,
  canvasDragPreviewSvg,
  window,
  makeSvgNode,
  viewerSvg,
  renderCorePrimitive,
  corePrimitiveRenderOptions,
});
function renderEditorOverlay(...args) { return canvasPreviewHost.renderEditorOverlay(...args); }
function syncCanvasDragPreviewViewport(...args) { return canvasPreviewHost.syncCanvasDragPreviewViewport(...args); }
function clearCanvasDragPreview(...args) { return canvasPreviewHost.clearCanvasDragPreview(...args); }
function screenPointFromSvgMatrix(...args) { return canvasPreviewHost.screenPointFromSvgMatrix(...args); }
function canvasScreenFeedbackPrimitiveNode(...args) { return canvasPreviewHost.canvasScreenFeedbackPrimitiveNode(...args); }
function renderCanvasDragPreview(...args) { return canvasPreviewHost.renderCanvasDragPreview(...args); }

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
  insertClipboardImage: (image) => insertImagePayload(image, null, 0, "clipboard"),
});
const writeNativeClipboardFromSelection = (...args) => editorCommandController.writeNativeClipboardFromSelection(...args);
const pasteFromNativeClipboard = (...args) => editorCommandController.pasteFromNativeClipboard(...args);
const runEditorCommand = (...args) => editorCommandController.runEditorCommand(...args);

canvasContextMenuHost = createCanvasContextMenuHost({
  state: () => state,
  editorState: () => editorState,
  desktopFileHost,
  hasPortableClipboard: () => editorCommandController.hasPortableClipboard(),
  colorHost,
  objectSettingsHost,
  numericDialogHost,
  atomPropertyDialogHost,
  chemicalPropertyDialogHost,
  annotationDialogHost,
  smilesDialogHost,
  tableDialogHost,
  transientNotificationHost,
  inchiHost,
  nmrPredictionHost,
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
  openImageFilePickerAt,
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
  applyDocumentBondCreationPreview,
  hideDocumentDiagnosticsForPreview,
  clearDocumentObjectPreviewTransform,
  clearDocumentBondCreationPreview,
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
  tableDialogHost,
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
    if (syncRenderList) {
      resetDocumentRenderState();
      currentDocumentMoleculeTopology();
      await syncCoreRenderListFromCurrentDocument();
      maybeAutoExpandEditorViewport(state.coreRenderList || []);
    } else {
      currentDocumentMoleculeTopology();
      const documentLayer = viewerSvg?.querySelector('[data-layer="document-content"]');
      if (documentLayer) {
        rebuildDocumentPrimitiveIndex(documentLayer);
      }
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

editorStateRuntimeHost = createEditorStateRuntimeHost({
  state,
  editorState,
  document,
  selectionChemistrySummary,
  isEditingRustDocument,
  parseEngineJson,
  renderDocumentPrimitiveChange,
  currentEditorRenderList,
  currentEditorInteractionRenderList,
  syncEditorSelectionRenderListFromEngine,
  currentDocumentBoundsContainsPoint,
  documentBoundsHitPadScreenPx: DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX,
  invalidateEditorEngineReadCache,
  renderEditorOverlay,
  syncSelectCursorForPoint: (...args) => syncSelectCursorForPoint(...args),
  finishActiveTextEditor,
  engineHost,
  syncTextSymbolPaletteFromEngine,
  commandEngine,
  transientNotificationHost,
  defaultEditorViewBox,
  clearZoomHandoffs,
  syncEngineToolState,
  syncDocumentFromEngine,
  renderSecondaryToolbar,
  markCurrentDocumentSaved,
  canSaveCurrentDocument,
  updateCanvasContextMenuAvailability,
});
function renderSelectionOnlyUpdate(...args) { return editorStateRuntimeHost.renderSelectionOnlyUpdate(...args); }
function selectClickTarget(...args) { return editorStateRuntimeHost.selectClickTarget(...args); }
function formatSelectionSummaryMass(...args) { return editorStateRuntimeHost.formatSelectionSummaryMass(...args); }
function clampMassDigits(...args) { return editorStateRuntimeHost.clampMassDigits(...args); }
function setMassDigits(...args) { return editorStateRuntimeHost.setMassDigits(...args); }
function massPrecisionIcon(...args) { return editorStateRuntimeHost.massPrecisionIcon(...args); }
function makeMassPrecisionControl(...args) { return editorStateRuntimeHost.makeMassPrecisionControl(...args); }
function makeSelectionSummaryItem(...args) { return editorStateRuntimeHost.makeSelectionSummaryItem(...args); }
function appendFormulaText(...args) { return editorStateRuntimeHost.appendFormulaText(...args); }
function syncSelectionChemistrySummary(...args) { return editorStateRuntimeHost.syncSelectionChemistrySummary(...args); }
function resetEditorEngine(...args) { return editorStateRuntimeHost.resetEditorEngine(...args); }
function resetDocumentEngine(...args) { return editorStateRuntimeHost.resetDocumentEngine(...args); }
function refreshCommandAvailability(...args) { return editorStateRuntimeHost.refreshCommandAvailability(...args); }
function uniformValue(...args) { return editorStateRuntimeHost.uniformValue(...args); }

async function activateEditorTool(nextTool) {
  const activation = activateEditorToolNow(nextTool);
  activeToolActivationPromise = activation;
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
    || editorState.activeTool === "table"
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
  if (editorState.activeTool === "shape" || editorState.activeTool === "table" || editorState.activeTool === "tlc-plate" || editorState.activeTool === "orbital") {
    setCanvasCursorStyle("crosshair");
    return;
  }
  if (editorState.activeTool === "chain") {
    setCanvasCursorStyle("crosshair");
    return;
  }
  setCanvasCursorStyle(overSelection ? "grab" : "default");
}

textEditActionHost = createTextEditActionHost({
  getActiveTextEditor: () => activeTextEditor,
  editorState,
  syncTextEditorSize: (...args) => syncTextEditorSize(...args),
  positionActiveTextEditor: (...args) => positionActiveTextEditor(...args),
  getTextEditorController: () => textEditorController,
  normalizeEditorSelectionOffsetsModel,
  state,
  focusActiveTextEditor: (...args) => focusActiveTextEditor(...args),
  syncEngineToolState: (...args) => syncEngineToolState(...args),
  renderSecondaryToolbar: (...args) => renderSecondaryToolbar(...args),
  syncCanvasCursor: (...args) => syncCanvasCursor(...args),
  setCanvasPointerShieldActive: (value) => { canvasPointerShieldActive = value; },
  canvasPointerShield,
  clearCanvasDragPreview,
  syncViewerSvgPointerEventMode: (...args) => syncViewerSvgPointerEventMode(...args),
  syncEditorPrimaryToolButtons: (...args) => syncEditorPrimaryToolButtons(...args),
});
function applyTextAlignment(...args) { return textEditActionHost.applyTextAlignment(...args); }
function currentEditorSelectionOffsets(...args) { return textEditActionHost.currentEditorSelectionOffsets(...args); }
function restoreEditorSelectionOffsets(...args) { return textEditActionHost.restoreEditorSelectionOffsets(...args); }
function normalizeEditorSelectionOffsets(...args) { return textEditActionHost.normalizeEditorSelectionOffsets(...args); }
function setActiveEditorSelection(...args) { return textEditActionHost.setActiveEditorSelection(...args); }
function renderActiveTextEditorFromModel(...args) { return textEditActionHost.renderActiveTextEditorFromModel(...args); }
function syncPendingEditorStyleWithSelection(...args) { return textEditActionHost.syncPendingEditorStyleWithSelection(...args); }
function handleTextEditorBeforeInput(...args) { return textEditActionHost.handleTextEditorBeforeInput(...args); }
function applyTextFormatCommand(...args) { return textEditActionHost.applyTextFormatCommand(...args); }
function applyTextScript(...args) { return textEditActionHost.applyTextScript(...args); }
function applyTextInlineStyle(...args) { return textEditActionHost.applyTextInlineStyle(...args); }
function applyChemicalFormat(...args) { return textEditActionHost.applyChemicalFormat(...args); }
function insertTextAtSelection(...args) { return textEditActionHost.insertTextAtSelection(...args); }
function insertTextSymbol(...args) { return textEditActionHost.insertTextSymbol(...args); }
function insertElementSymbol(...args) { return textEditActionHost.insertElementSymbol(...args); }
function setElementPlacementActive(...args) { return textEditActionHost.setElementPlacementActive(...args); }
function handleQuickPaletteModeChange(...args) { return textEditActionHost.handleQuickPaletteModeChange(...args); }
function selectElementFromQuickPalette(...args) { return textEditActionHost.selectElementFromQuickPalette(...args); }

const editorToolbarHost = createEditorToolbarHost({
  state,
  editorState,
  secondaryToolbar,
  parseEngineJson,
  insertTextSymbol,
  selectElementFromQuickPalette,
  handleQuickPaletteModeChange,
  uiActions,
});

function renderSecondaryToolbar(...args) { return editorToolbarHost.renderSecondaryToolbar(...args); }
function currentDocumentColors(...args) { return editorToolbarHost.currentDocumentColors(...args); }
function currentToolbarColorPalette(...args) { return editorToolbarHost.currentToolbarColorPalette(...args); }
function currentElementPalette(...args) { return editorToolbarHost.currentElementPalette(...args); }
function syncTextSymbolPaletteFromEngine(...args) { return editorToolbarHost.syncTextSymbolPaletteFromEngine(...args); }
function ensureTextSymbolPalette(...args) { return editorToolbarHost.ensureTextSymbolPalette(...args); }
function syncEditorPrimaryToolButtons(...args) { return editorToolbarHost.syncPrimaryToolButtons(...args); }

textEditorLayoutHost = createTextEditorLayoutHost({
  getActiveTextEditor: () => activeTextEditor,
  editorState,
  routeEditorPointerEvents: (...args) => routeEditorPointerEvents(...args),
  state,
  svgPointFromEvent: (...args) => svgPointFromEvent(...args),
  currentEditorRenderList,
  renderEditorOverlay,
  openTextEditorAt: (...args) => openTextEditorAt(...args),
  createEditorSourceRunsFromSession,
  normalizeEditorSourceRuns: (...args) => normalizeEditorSourceRuns(...args),
  editorRootFontFamily,
  cssColorToHex: (...args) => cssColorToHex(...args),
  renderSecondaryToolbar,
  worldToLayerPoint,
  getSharedGlyphProfiles: () => sharedGlyphProfiles,
  zoomScale,
  textLength,
  setActiveEditorSelection: (...args) => setActiveEditorSelection(...args),
  renderActiveTextEditorFromModel: (...args) => renderActiveTextEditorFromModel(...args),
  currentEditorSelectionOffsets: (...args) => currentEditorSelectionOffsets(...args),
  document,
});

function commandResultForTextEditorTarget(...args) { return textEditorLayoutHost.commandResultForTextEditorTarget(...args); }
function textEditPrimitiveNodeId(...args) { return textEditorLayoutHost.textEditPrimitiveNodeId(...args); }
function textEditPrimitiveObjectId(...args) { return textEditorLayoutHost.textEditPrimitiveObjectId(...args); }
function textEditHoverPrimitiveFromRenderList(...args) { return textEditorLayoutHost.textEditHoverPrimitiveFromRenderList(...args); }
function activeTextEditorTargetMatchesHoverPrimitive(...args) { return textEditorLayoutHost.activeTextEditorTargetMatchesHoverPrimitive(...args); }
function updateTextToolHoverFromPointerEvent(...args) { return textEditorLayoutHost.updateTextToolHoverFromPointerEvent(...args); }
function openHoveredTextEditTargetFromPointerEvent(...args) { return textEditorLayoutHost.openHoveredTextEditTargetFromPointerEvent(...args); }
function editorSourceRunsFromSession(...args) { return textEditorLayoutHost.editorSourceRunsFromSession(...args); }
function editorRootBaseStyle(...args) { return textEditorLayoutHost.editorRootBaseStyle(...args); }
function syncTextToolbarStateFromSession(...args) { return textEditorLayoutHost.syncTextToolbarStateFromSession(...args); }
function positionActiveTextEditor(...args) { return textEditorLayoutHost.positionActiveTextEditor(...args); }
function syncEditorVisualMetrics(...args) { return textEditorLayoutHost.syncEditorVisualMetrics(...args); }
function syncTextEditorSize(...args) { return textEditorLayoutHost.syncTextEditorSize(...args); }
function defaultTextEditorLineHeight(...args) { return textEditorLayoutHost.defaultTextEditorLineHeight(...args); }
function editorDisplayScale(...args) { return textEditorLayoutHost.editorDisplayScale(...args); }
function editorGlyphProfiles(...args) { return textEditorLayoutHost.editorGlyphProfiles(...args); }
function editorGlyphLayoutConfig(...args) { return textEditorLayoutHost.editorGlyphLayoutConfig(...args); }
function buildEditorTextLayout(...args) { return textEditorLayoutHost.buildEditorTextLayout(...args); }
function placeCaretAtEnd(...args) { return textEditorLayoutHost.placeCaretAtEnd(...args); }
function selectAllEditorText(...args) { return textEditorLayoutHost.selectAllEditorText(...args); }
function captureEditorCaretOffset(...args) { return textEditorLayoutHost.captureEditorCaretOffset(...args); }
function restoreEditorCaretOffset(...args) { return textEditorLayoutHost.restoreEditorCaretOffset(...args); }
function updateCustomEditorChrome(...args) { return textEditorLayoutHost.updateCustomEditorChrome(...args); }
function renderEditorSelectionSegments(...args) { return textEditorLayoutHost.renderEditorSelectionSegments(...args); }
function positionHiddenEditorInput(...args) { return textEditorLayoutHost.positionHiddenEditorInput(...args); }
function measureEditorCaretRect(...args) { return textEditorLayoutHost.measureEditorCaretRect(...args); }
function buildEditorCaretLayout(...args) { return textEditorLayoutHost.buildEditorCaretLayout(...args); }
function editorLineIndexForOffset(...args) { return textEditorLayoutHost.editorLineIndexForOffset(...args); }
function nearestOffsetOnLine(...args) { return textEditorLayoutHost.nearestOffsetOnLine(...args); }
function editorOffsetFromPointerEvent(...args) { return textEditorLayoutHost.editorOffsetFromPointerEvent(...args); }

textEditorController = createTextEditorController({
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
  uiActions,
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

textEditCommitHost = createTextEditCommitHost({
  getActiveTextEditor: () => activeTextEditor,
  setActiveTextEditor: (value) => { activeTextEditor = value; },
  textEditorLayer,
  commandResultForTextEditorTarget,
  window,
  renderDocumentChange,
  renderEditorOverlay,
  currentEditorRenderList,
  editorSessionToEngineSession,
  commandEngine,
  state,
  editorRootBaseStyle,
  editorState,
  defaultTextEditorLineHeight,
  runsPlainText,
  editorRootFontFamily,
  normalizeEditorSourceRunsModel,
  applyTextInlineStyle: (...args) => applyTextInlineStyle(...args),
  isEditingRustDocument,
  showKernelNoticeDialog: (...args) => appWindowLifecycleHost.showKernelNoticeDialog(...args),
});
function finishActiveTextEditor(...args) { return textEditCommitHost.finishActiveTextEditor(...args); }
function buildCommittedTextSession(...args) { return textEditCommitHost.buildCommittedTextSession(...args); }
function normalizeEditorSourceRuns(...args) { return textEditCommitHost.normalizeEditorSourceRuns(...args); }
function cssColorToHex(...args) { return textEditCommitHost.cssColorToHex(...args); }
function applySelectionColor(...args) { return textEditCommitHost.applySelectionColor(...args); }

const documentFlow = createDocumentFlow({
  state,
  engineHost,
  desktopFileHost,
  openFileInput,
  imageFileInput,
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
function openGeneratedDocumentTab(...args) { return browserDocumentTabs.openGeneratedDocumentTab(...args); }
function openDocumentPathInTab(...args) { return browserDocumentTabs.openDocumentPathInTab(...args); }
function openDocumentFileInTab(...args) { return browserDocumentTabs.openDocumentFileInTab(...args); }
function openDroppedDocumentFileInTab(...args) { return browserDocumentTabs.openDroppedDocumentFileInTab(...args); }
function openDroppedDocumentFilesInTabs(...args) { return browserDocumentTabs.openDroppedDocumentFilesInTabs(...args); }
function chooseAndOpenDocumentTab(...args) { return browserDocumentTabs.chooseAndOpenDocumentTab(...args); }
function confirmApplyDocumentStylePreset(...args) { return browserDocumentTabs.confirmApplyDocumentStylePreset(...args); }

const imageImportHost = createImageImportHost({
  svgPointFromEvent: (...args) => svgPointFromEvent(...args),
  activeViewBox: (...args) => activeViewBox(...args),
  getPendingImageInsertWorldPoint: () => pendingImageInsertWorldPoint,
  setPendingImageInsertWorldPoint: (value) => { pendingImageInsertWorldPoint = value; },
  imageFileInput,
  isEditingRustDocument,
  commandEngine,
  activateEditorTool,
  renderDocumentChange,
  desktopFileHost,
});
function openImageFilePickerAt(...args) { return imageImportHost.openImageFilePickerAt(...args); }
function consumeImageInsertWorldPoint(...args) { return imageImportHost.consumeImageInsertWorldPoint(...args); }
function insertDroppedImageFiles(...args) { return imageImportHost.insertDroppedImageFiles(...args); }
function insertDroppedImagePaths(...args) { return imageImportHost.insertDroppedImagePaths(...args); }

editorRuntimeHost = createEditorRuntimeHost({
  viewerSvg,
  DOMPoint,
  activeViewBox,
  state,
  BOND_STROKE,
  isEditingRustDocument,
  editorState,
  getCanvasPointerShieldActive: () => canvasPointerShieldActive,
  viewportScale,
  commandEngine,
  renderDocumentChange,
  BRACKET_LABEL_OFFSET_X,
  BRACKET_LABEL_OFFSET_Y,
  sceneRenderer,
  resetDocumentRenderState,
  applyViewerViewport,
  normalizeDisplayColor,
  CHEMDRAW_PAGE_BACKGROUND,
  makeSvgNode,
  rebuildDocumentPrimitiveIndex,
  syncViewerStats,
  renderEditorOverlay,
  positionActiveTextEditor,
  ensureDocumentTab,
  renderDocumentTabs,
  desktopFileHost,
  loadDetachedDocumentPayload,
  takeBrowserPendingDocument,
  loadBrowserPendingDocumentPayload,
  openDocumentPath,
  saveActiveDocumentTabState,
  openDocumentPathInTab,
  loadAndRender,
});
function svgPointFromEvent(...args) { return editorRuntimeHost.svgPointFromEvent(...args); }
function editorBondStrokeWidth(...args) { return editorRuntimeHost.editorBondStrokeWidth(...args); }
function routeEditorPointerEvents(...args) { return editorRuntimeHost.routeEditorPointerEvents(...args); }
function activeToolUsesContainerPointerEvents(...args) { return editorRuntimeHost.activeToolUsesContainerPointerEvents(...args); }
function syncViewerSvgPointerEventMode(...args) { return editorRuntimeHost.syncViewerSvgPointerEventMode(...args); }
function screenPxToWorld(...args) { return editorRuntimeHost.screenPxToWorld(...args); }
function applySelectionArrangeCommand(...args) { return editorRuntimeHost.applySelectionArrangeCommand(...args); }
function applyArrowOptionsToSelection(...args) { return editorRuntimeHost.applyArrowOptionsToSelection(...args); }
function bracketLabelAnchorPoint(...args) { return editorRuntimeHost.bracketLabelAnchorPoint(...args); }
function handleViewerContainerPointerEvent(...args) { return editorRuntimeHost.handleViewerContainerPointerEvent(...args); }
function renderDocument(...args) { return editorRuntimeHost.renderDocument(...args); }
function loadInitialDocumentTabs(...args) { return editorRuntimeHost.loadInitialDocumentTabs(...args); }

ensureTextSymbolPalette();

bindEditorControls({
  state,
  editorState,
  desktopFileHost,
  colorHost,
  openFileInput,
  imageFileInput,
  openImageFilePicker: () => openImageFilePickerAt(null),
  consumeImageInsertWorldPoint,
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
  insertDroppedImageFiles,
  insertDroppedImagePaths,
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
  uiActions,
});

renderSecondaryToolbar();
syncCanvasCursor();
syncViewerSvgPointerEventMode();
bindBrowserBeforeUnloadGuard();

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
viewerSvg?.addEventListener("pointercancel", uiActions.listener("editor.pointer-cancel", async () => {
  await editorPointerController.handleEditorPointerCancel();
}));
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

watchDisplayMetrics();

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
