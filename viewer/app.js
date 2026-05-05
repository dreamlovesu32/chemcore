import initializeChemcoreEngine, { WasmEngine } from "./engine/chemcore_engine.js";
import {
  parseEngineJson,
  primitivesForObject,
  renderBoundsFromEngine,
  renderListFromEngine,
} from "./engine_bridge.js";
import { bindEditorControls } from "./editor_bindings.js";
import { createDocumentFlow } from "./document_flow.js";
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
  normalizeToolbarFontSize,
  renderSecondaryToolbarHtml,
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
  "../tmp/examples/02-13/2017-2-13/oleObject1.chemcore.json",
  "../tmp/examples/02-13/2017-2-13/oleObject2.chemcore.json",
  "../tmp/examples/02-13/2017-2-13/oleObject3.chemcore.json",
  "../tmp/examples/02-13/2017-2-13/oleObject4.chemcore.json",
  "../tmp/examples/02-13/lm 2017-2-13  working report/oleObject1.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject1.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject2.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject3.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject4.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject5.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject6.chemcore.json",
];

const VIEW_MODE = document.body.dataset.viewMode || "document";
const LABEL_DEBUG_MODE = VIEW_MODE === "label-debug";

const state = {
  currentPath: LABEL_DEBUG_MODE ? SAMPLE_FILES[0] : null,
  currentFileName: null,
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
let sharedGlyphProfiles = null;
const sharedGlyphProfilesReady = loadSharedGlyphProfiles();

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
    insertEditorText(text) {
      if (!activeTextEditor) {
        return false;
      }
      for (const character of Array.from(String(text || ""))) {
        textEditorController.insertTextAtSelection(character);
      }
      return true;
    },
    syncDocument() {
      syncDocumentFromEngine();
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
const documentStylePresetInput = document.getElementById("document-style-preset");
const openFileInput = document.createElement("input");
openFileInput.type = "file";
openFileInput.accept = [
  ".json",
  ".cdxml",
  "application/json",
  "text/xml",
  "application/xml",
  "application/x-cdxml",
  "chemical/x-cdxml",
  "application/vnd.cambridgesoft.cdxml",
].join(",");
openFileInput.className = "visually-hidden";
document.body.appendChild(openFileInput);
const textEditorLayer = document.createElement("div");
textEditorLayer.className = "text-editor-layer";
viewerContainer?.appendChild(textEditorLayer);
let textSymbolPalette = null;
const textSymbolCatalogReady = loadTextSymbolCatalog().then((catalog) => {
  textSymbolPalette = createTextSymbolPalette({
    mount: viewerContainer,
    catalog,
    onSelect: insertTextSymbol,
  });
  return textSymbolPalette;
});

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

const editorState = {
  activeTool: "bond",
  documentStylePreset: "default",
  selectMode: "free",
  bondType: "single",
  textFontFamily: "Arial",
  textFontSize: normalizeToolbarFontSize(cmToCssPx(DEFAULT_TEXT_FONT_SIZE)),
  textColor: "#000000",
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
  bracketKind: "round",
  symbolKind: "circle-plus",
  template: "ring-6",
};
let activeTextEditor = null;
let activeSelectionGesture = null;

async function loadSharedGlyphProfiles() {
  const url = new URL("../shared/glyph_profiles.json", import.meta.url);
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to load shared glyph profiles: ${response.status}`);
  }
  sharedGlyphProfiles = normalizeSharedGlyphProfiles(await response.json());
  return sharedGlyphProfiles;
}

function isEditingRustDocument() {
  return !LABEL_DEBUG_MODE && !state.currentPath && state.editorEngine;
}

function syncEngineToolState() {
  if (!state.editorEngine) {
    return;
  }
  state.editorEngine.setDocumentStylePreset?.(editorState.documentStylePreset);
  state.editorEngine.setTool(editorState.activeTool, editorState.bondType);
  state.editorEngine.setTemplate?.(editorState.template);
  state.editorEngine.setShapeOptions?.(
    editorState.shapeKind,
    editorState.shapeStyle,
    editorState.shapeColor,
  );
  state.editorEngine.setBracketOptions?.(editorState.bracketKind);
  state.editorEngine.setSymbolOptions?.(editorState.symbolKind);
  if (state.editorEngine.setArrowEndpointOptions) {
    state.editorEngine.setArrowEndpointOptions(
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
  state.editorEngine.setArrowOptions?.(
    editorState.arrowType,
    editorState.arrowHeadSize,
    editorState.arrowHead,
    editorState.arrowTail,
    editorState.arrowBold,
  );
}

function syncDocumentStylePresetFromEngine() {
  if (!state.editorEngine?.documentStylePreset) {
    return;
  }
  const preset = state.editorEngine.documentStylePreset() || "default";
  editorState.documentStylePreset = preset;
  if (documentStylePresetInput) {
    documentStylePresetInput.value = preset;
  }
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
  const bounds = currentRenderBounds("all");
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

function syncCoreRenderListFromCurrentDocument() {
  state.coreRenderList = null;
  if (!state.currentDocument) {
    return;
  }
  if (state.currentPath) {
    if (!state.documentEngine) {
      resetDocumentEngine();
    }
    state.documentEngine.loadDocumentJson(JSON.stringify(state.currentDocument));
    state.coreRenderList = renderListFromEngine(state.documentEngine);
    return;
  }
  if (state.editorEngine) {
    state.coreRenderList = renderListFromEngine(state.editorEngine);
  }
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

function syncDocumentFromEngine() {
  if (!state.editorEngine) {
    return;
  }
  const documentData = parseEngineJson(state.editorEngine.documentJson());
  if (documentData) {
    state.currentDocument = documentData;
    syncCoreRenderListFromCurrentDocument();
    maybeAutoExpandEditorViewport(state.coreRenderList || []);
  }
  refreshCommandAvailability();
}

function currentEditorEngineState() {
  if (!state.editorEngine) {
    return null;
  }
  return parseEngineJson(state.editorEngine.stateJson());
}

function resetEditorEngine() {
  finishActiveTextEditor(false);
  state.editorEngine?.free?.();
  state.editorEngine = new WasmEngine();
  state.runtimeViewBox = defaultEditorViewBox();
  state.lastEditFocusPoint = null;
  clearZoomHandoffs();
  state.currentFileName = null;
  syncEngineToolState();
  syncDocumentFromEngine();
}

function resetDocumentEngine() {
  state.documentEngine?.free?.();
  state.documentEngine = new WasmEngine();
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
}

function runEditorCommand(command) {
  if (!isEditingRustDocument()) {
    return false;
  }
  let changed = false;
  if (command === "undo") {
    changed = state.editorEngine.undo();
  } else if (command === "redo") {
    changed = state.editorEngine.redo();
  } else if (command === "copy") {
    changed = !!state.editorEngine.copySelection?.();
  } else if (command === "cut") {
    changed = !!state.editorEngine.cutSelection?.();
  } else if (command === "paste") {
    changed = !!state.editorEngine.pasteClipboard?.();
  } else if (command === "delete") {
    changed = state.editorEngine.deleteSelection();
  } else {
    return false;
  }
  if (changed) {
    syncDocumentFromEngine();
    renderDocument();
  } else {
    renderEditorOverlay();
    refreshCommandAvailability();
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

function syncSelectCursorForPoint(point) {
  if (!viewerSvg || editorState.activeTool !== "select" || !isEditingRustDocument()) {
    syncCanvasCursor();
    return;
  }
  syncArrowAwareCursorForPoint(point);
}

function syncArrowAwareCursorForPoint(point) {
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
  if (activeSelectionGesture?.kind === "arrow-endpoint") {
    viewerSvg.style.cursor = "move";
    return;
  }
  if (activeSelectionGesture?.kind === "arrow-curve") {
    viewerSvg.style.cursor = "nesw-resize";
    return;
  }
  if (selectionRotateHandleHit(point)) {
    viewerSvg.style.cursor = "grab";
    return;
  }
  const arrowAction = state.editorEngine.hoverArrowAction?.(point.x, point.y) || "";
  if (arrowAction === "head" || arrowAction === "tail") {
    viewerSvg.style.cursor = "move";
    return;
  }
  if (arrowAction === "curve") {
    viewerSvg.style.cursor = "nesw-resize";
    return;
  }
  if (editorState.activeTool === "arrow") {
    viewerSvg.style.cursor = "crosshair";
    return;
  }
  const overSelection = !!state.editorEngine.selectionContainsPoint?.(point.x, point.y);
  viewerSvg.style.cursor = overSelection ? "grab" : "default";
}

function renderSecondaryToolbar() {
  if (!secondaryToolbar) {
    return;
  }
  secondaryToolbar.innerHTML = renderSecondaryToolbarHtml(editorState);
  syncPrimaryToolButtons(editorState, document);
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

function openTextEditorAt(point) {
  finishActiveTextEditor(true);
  const session = parseEngineJson(state.editorEngine?.beginTextEdit?.(point.x, point.y), null);
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

function updateTextToolHoverFromPointerEvent(event) {
  if (!routeEditorPointerEvents() || editorState.activeTool !== "text" || !state.editorEngine?.pointerMove) {
    return null;
  }
  const point = svgPointFromEvent(event);
  state.editorEngine.pointerMove(point.x, point.y, event.altKey);
  const renderList = currentEditorRenderList();
  renderEditorOverlay(renderList);
  positionActiveTextEditor();
  return textEditHoverPrimitiveFromRenderList(renderList);
}

function openHoveredTextEditTargetFromPointerEvent(event) {
  const hoverPrimitive = updateTextToolHoverFromPointerEvent(event);
  if (!hoverPrimitive || activeTextEditorTargetMatchesHoverPrimitive(hoverPrimitive)) {
    return false;
  }
  event.preventDefault();
  event.stopPropagation();
  openTextEditorAt(svgPointFromEvent(event));
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
  editorState.textFontSize = normalizeToolbarFontSize(session.fontSize || editorState.textFontSize);
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

function finishActiveTextEditor(commit = true) {
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
  const changed = state.editorEngine?.applyTextEdit?.(JSON.stringify(editorSessionToEngineSession(nextSession)));
  syncDocumentFromEngine();
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
  if (color.startsWith("#")) {
    return color;
  }
  const match = color.match(/\d+/g);
  if (!match || match.length < 3) {
    return color;
  }
  return `#${match.slice(0, 3).map((value) => Number(value).toString(16).padStart(2, "0")).join("")}`;
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
  syncEngineToolState();
  renderSecondaryToolbar();
  syncCanvasCursor();
}

const documentFlow = createDocumentFlow({
  state,
  WasmEngine,
  openFileInput,
  viewerTitle,
  viewerStats,
  viewerSvg,
  docMeta,
  finishActiveTextEditor,
  clearZoomHandoffs,
  syncDocumentStylePresetFromEngine,
  syncEngineToolState,
  syncDocumentFromEngine,
  syncCoreRenderListFromCurrentDocument,
  resetEditorEngine,
  pageViewBox,
  defaultEditorViewBox,
  renderDocument,
  fitView,
});

const {
  chooseAndOpenDocument,
  isAbortError,
  loadAndRender,
  loadJsonDocumentIntoEditor,
  openDocumentFile,
  saveCurrentDocumentAs,
  saveCurrentDocumentCdxml,
  saveCurrentDocumentSvg,
  updateDocumentMeta,
} = documentFlow;

bindEditorControls({
  state,
  editorState,
  openFileInput,
  zoomInput,
  secondaryToolbar,
  documentStylePresetInput,
  getActiveTextEditor: () => activeTextEditor,
  clearActiveSelectionGesture: () => { activeSelectionGesture = null; },
  getZoomPercent: () => zoomPercent,
  setTextFontSize: (size) => {
    editorState.textFontSize = normalizeToolbarFontSize(Math.max(5, Math.min(288, size)));
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
  chooseAndOpenDocument,
  saveCurrentDocumentAs,
  saveCurrentDocumentCdxml,
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

function applySelectionArrangeCommand(command) {
  if (!isEditingRustDocument() || editorState.activeTool !== "select") {
    return false;
  }
  const changed = !!state.editorEngine.applySelectionArrangeCommand?.(command);
  if (!changed) {
    return false;
  }
  syncDocumentFromEngine();
  renderDocument();
  return true;
}

function applyArrowOptionsToSelection() {
  if (!isEditingRustDocument()) {
    return false;
  }
  const changed = state.editorEngine.applyArrowEndpointOptionsToSelection
    ? !!state.editorEngine.applyArrowEndpointOptionsToSelection(
      editorState.arrowType,
      editorState.arrowHeadSize,
      editorState.arrowCurve,
      editorState.arrowHeadStyle,
      editorState.arrowTailStyle,
      editorState.arrowNoGo,
      editorState.arrowBold,
    )
    : !!state.editorEngine.applyArrowOptionsToSelection?.(
      editorState.arrowType,
      editorState.arrowHeadSize,
      editorState.arrowHead,
      editorState.arrowTail,
      editorState.arrowBold,
    );
  if (changed) {
    syncDocumentFromEngine();
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

function handleEditorPointerMove(event) {
  const point = svgPointFromEvent(event);
  if ((editorState.activeTool === "select" || editorState.activeTool === "arrow") && activeSelectionGesture) {
    event.preventDefault();
    if (activeSelectionGesture.kind === "arrow-endpoint" || activeSelectionGesture.kind === "arrow-curve") {
      if (pointDistance(activeSelectionGesture.start, point) >= cssPxToCm(3)) {
        activeSelectionGesture.dragged = true;
      }
      activeSelectionGesture.current = point;
      state.editorEngine.updateHoverArrowEdit?.(point.x, point.y, event.altKey);
      if (activeSelectionGesture.kind === "arrow-curve") {
        activeSelectionGesture.angle = state.editorEngine.activeArrowEditDegrees?.() || 0;
      }
      syncDocumentFromEngine();
      syncArrowAwareCursorForPoint(point);
      renderDocument();
      return;
    }
    if (activeSelectionGesture.kind === "rotate") {
      activeSelectionGesture.current = point;
      activeSelectionGesture.angle = selectionRotateAngleForGesture(activeSelectionGesture, point, event.altKey);
      state.editorEngine.updateSelectionRotate(point.x, point.y, event.altKey);
      syncDocumentFromEngine();
      syncSelectCursorForPoint(point);
      renderDocument();
      return;
    }
    if (activeSelectionGesture.kind === "move") {
      activeSelectionGesture.current = point;
      state.editorEngine.updateSelectionMove(point.x, point.y, event.altKey);
      syncDocumentFromEngine();
      syncSelectCursorForPoint(point);
      renderDocument();
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
      state.editorEngine.clearInteraction();
      renderEditorOverlay();
    }
    return;
  }
  state.editorEngine.pointerMove(point.x, point.y, event.altKey);
  if (editorState.activeTool === "select") {
    syncSelectCursorForPoint(point);
  } else if (editorState.activeTool === "arrow") {
    syncArrowAwareCursorForPoint(point);
  }
  const renderList = currentEditorRenderList();
  maybeAutoExpandEditorViewport(renderList);
  renderEditorOverlay(renderList);
  positionActiveTextEditor();
}

function handleEditorPointerDown(event) {
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
    openTextEditorAt(point);
    return;
  }
  if (editorState.activeTool === "select") {
    event.preventDefault();
    viewerSvg.setPointerCapture?.(event.pointerId);
    state.editorEngine.pointerMove(point.x, point.y, event.altKey);
    const arrowEditAction = state.editorEngine.beginHoverArrowEdit?.(point.x, point.y) || "";
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
      syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(currentEditorRenderList());
      return;
    }
    const rotateHandle = currentSelectionRotateHandle();
    if (rotateHandle && pointDistance(point, rotateHandle) <= rotateHandle.hitRadius) {
      if (state.editorEngine.beginSelectionRotate?.(point.x, point.y)) {
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
        syncSelectCursorForPoint(point);
        renderDocument();
        return;
      }
    }
    if (state.editorEngine.beginSelectionMove?.(point.x, point.y, !!event.shiftKey, event.altKey)) {
      activeSelectionGesture = {
        kind: "move",
        start: point,
        current: point,
        additive: !!event.shiftKey,
      };
      syncSelectCursorForPoint(point);
      renderDocument();
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
    const arrowEditAction = state.editorEngine.beginHoverArrowEdit?.(point.x, point.y) || "";
    if (arrowEditAction) {
      activeSelectionGesture = {
        kind: arrowEditAction === "curve" ? "arrow-curve" : "arrow-endpoint",
        action: arrowEditAction,
        start: point,
        current: point,
        dragged: false,
        angle: 0,
      };
      syncArrowAwareCursorForPoint(point);
      renderEditorOverlay(currentEditorRenderList());
      return;
    }
  }
  state.editorEngine.pointerDown(point.x, point.y, event.altKey);
  syncDocumentFromEngine();
  renderEditorOverlay(currentEditorRenderList());
}

function handleEditorPointerUp(event) {
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
    state.editorEngine.finishHoverArrowEdit?.(point.x, point.y, event.altKey);
    syncDocumentFromEngine();
    if (!gesture.dragged && editorState.activeTool === "select") {
      state.editorEngine.selectAtPoint(point.x, point.y, gesture.additive);
    }
    syncArrowAwareCursorForPoint(point);
    renderDocument();
    return;
  }
  if (editorState.activeTool === "select") {
    const gesture = activeSelectionGesture;
    activeSelectionGesture = null;
    if (!gesture) {
      return;
    }
    if (gesture.kind === "rotate") {
      state.editorEngine.finishSelectionRotate(point.x, point.y, event.altKey);
      syncDocumentFromEngine();
      syncSelectCursorForPoint(point);
      renderDocument();
      return;
    }
    if (gesture.kind === "move") {
      if (gesture.dragged) {
        state.editorEngine.finishSelectionMove(point.x, point.y, event.altKey);
        syncDocumentFromEngine();
      } else {
        state.editorEngine.selectAtPoint(point.x, point.y, gesture.additive);
      }
      syncSelectCursorForPoint(point);
      renderDocument();
      return;
    }
    if (!gesture.dragged) {
      state.editorEngine.selectAtPoint(point.x, point.y, gesture.additive);
    } else if (editorState.selectMode === "box") {
      state.editorEngine.selectInRect(
        gesture.start.x,
        gesture.start.y,
        point.x,
        point.y,
        gesture.additive,
      );
    } else {
      const polygonPoints = [...gesture.points, point].map((candidate) => [candidate.x, candidate.y]);
      state.editorEngine.selectInPolygon(JSON.stringify(polygonPoints), gesture.additive);
    }
    syncSelectCursorForPoint(point);
    renderDocument();
    return;
  }
  state.editorEngine.pointerUp(point.x, point.y, event.altKey);
  syncDocumentFromEngine();
  renderDocument();
  if (editorState.activeTool === "bracket") {
    const start = state.activeBracketDragStart;
    state.activeBracketDragStart = null;
    if (start && pointDistance(start, point) >= cssPxToCm(4)) {
      openTextEditorAt(bracketLabelAnchorPoint(start, point));
    }
  }
}

function handleEditorPointerLeave() {
  if (!isEditingRustDocument()) {
    return;
  }
  if (editorState.activeTool === "select" && activeSelectionGesture) {
    return;
  }
  if (editorState.activeTool !== "text") {
    state.editorEngine.clearInteraction();
    renderEditorOverlay();
  }
}

function handleEditorDoubleClick(event) {
  if (!routeEditorPointerEvents() || editorState.activeTool !== "select") {
    return;
  }
  const point = svgPointFromEvent(event);
  const changed = !!state.editorEngine.selectComponentAtPoint?.(point.x, point.y, event.shiftKey);
  if (!changed) {
    return;
  }
  event.preventDefault();
  activeSelectionGesture = null;
  syncDocumentFromEngine();
  syncSelectCursorForPoint(point);
  renderDocument();
}

function renderEditorOverlay(renderList = null) {
  viewerSvg.querySelector('[data-layer="editor-overlay"]')?.remove();
  if (!isEditingRustDocument()) {
    return;
  }
  const primitives = renderList || currentEditorRenderList();
  const overlay = makeSvgNode("g", { "data-layer": "editor-overlay", "pointer-events": "none" });
  const previewActive = primitives.some((primitive) => primitive.role === "preview-end");
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
  if (editorState.activeTool === "select" && activeSelectionGesture?.kind === "rotate") {
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
viewerSvg?.addEventListener("pointercancel", () => {
  activeSelectionGesture = null;
  state.editorEngine?.clearInteraction?.();
  syncCanvasCursor();
  renderEditorOverlay();
});
viewerSvg?.addEventListener("pointerleave", handleEditorPointerLeave);
viewerContainer?.addEventListener("wheel", handleViewerWheel, { passive: false });
viewerContainer?.addEventListener("scroll", () => {
  positionActiveTextEditor();
});

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

    if (object.type === "molecule") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive, corePrimitiveRenderOptions()));
      }
    } else if (object.type === "shape") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive, corePrimitiveRenderOptions()));
      } else {
        renderShapeObject(viewerSvg, object, documentData.styles);
      }
    } else if (object.type === "line") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive, corePrimitiveRenderOptions()));
      } else {
        renderLineObject(viewerSvg, object, state.currentDocument.styles);
      }
    } else if (object.type === "text") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive, corePrimitiveRenderOptions()));
      } else {
        renderTextObject(viewerSvg, object);
      }
    } else if (object.type === "bracket" || object.type === "symbol") {
      const corePrimitives = corePrimitivesForObject(object.id);
      corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive, corePrimitiveRenderOptions()));
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
    const bounds = currentRenderBounds("all");
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

try {
  await Promise.all([initializeChemcoreEngine(), sharedGlyphProfilesReady, textSymbolCatalogReady]);
  await loadAndRender();
} catch (error) {
  viewerTitle.textContent = "Runtime load failed";
  viewerStats.textContent = "";
  docMeta.textContent = String(error);
  viewerSvg.innerHTML = "";
}
