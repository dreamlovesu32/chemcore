import initializeChemcoreEngine, { WasmEngine } from "./engine/chemcore_engine.js";
import {
  displayLabelFontFamily,
  ensureSvgDefs,
  fontStyleForRun,
  fontWeightForRun,
  isSubscriptRun,
  isSuperscriptRun,
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
  renderLineObject,
  renderShapeObject,
  renderTextObject,
} from "./object_fallbacks.js";
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
  currentDocument: null,
  editorEngine: null,
  documentEngine: null,
  coreRenderList: null,
  runtimeViewBox: null,
  lastEditFocusPoint: null,
  displayMetrics: displayMetrics(),
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

const DEFAULT_TEXT_FONT_SIZE = 0.2645833;
const BOND_STROKE = 0.035;
const CHEMDRAW_PAGE_BACKGROUND = "#ffffff";
const CHEMDRAW_INK = "#000000";
const DEFAULT_WORKSPACE_WIDTH = 31.75;
const DEFAULT_WORKSPACE_HEIGHT = 21.1666666667;
const EDITOR_VIEW_BUFFER_RATIO = 0.6;
const EDITOR_AUTO_EXPAND_TRIGGER_RATIO = 0.18;
const EDITOR_FIT_PADDING_RATIO = 0.08;
const ZOOM_MIN_PERCENT = 25;
const ZOOM_MAX_PERCENT = 400;
const ZOOM_STEP_LEVELS = [25, 33, 50, 67, 80, 100, 125, 150, 200, 250, 300, 400];
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
const textEditorLayer = document.createElement("div");
textEditorLayer.className = "text-editor-layer";
viewerContainer?.appendChild(textEditorLayer);

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
const TEXT_FONT_OPTIONS = [
  "Arial",
  "Helvetica",
  "TeX Gyre Heros",
  "Times New Roman",
  "Courier New",
];
const TEXT_FONT_SIZE_OPTIONS = [5, 6, 7, 8, 9, 10, 12, 14, 16, 18, 24];

function normalizeToolbarFontSize(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return 10;
  }
  const rounded = Math.round(numeric);
  if (Math.abs(numeric - rounded) < 0.05) {
    return rounded;
  }
  return Math.round(numeric * 10) / 10;
}

function formatToolbarFontSize(value) {
  const normalized = normalizeToolbarFontSize(value);
  return Number.isInteger(normalized) ? String(normalized) : normalized.toFixed(1);
}

const editorState = {
  activeTool: "bond",
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
  shapeStroke: "#000000",
  shapeFill: "none",
  shapeStyle: "rect",
  template: "benzene",
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
  state.editorEngine.setTool(editorState.activeTool, editorState.bondType);
}

function parseEngineJson(json, fallback = null) {
  try {
    return JSON.parse(json);
  } catch (error) {
    console.warn("Failed to parse chemcore engine JSON", error);
    return fallback;
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
  return {
    ...session,
    fontSize: session.fontSize == null ? session.fontSize : convert(Number(session.fontSize)),
    lineHeight: session.lineHeight == null ? session.lineHeight : convert(Number(session.lineHeight)),
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

function subtractPoints(a, b) {
  return { x: a.x - b.x, y: a.y - b.y };
}

function pointDistance(a, b) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function midpoint(a, b) {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

function pointLineDistance(point, lineStart, lineEnd) {
  const dx = lineEnd.x - lineStart.x;
  const dy = lineEnd.y - lineStart.y;
  const length = Math.hypot(dx, dy);
  if (length <= 1.0e-6) {
    return pointDistance(point, lineStart);
  }
  return Math.abs((point.x - lineStart.x) * dy - (point.y - lineStart.y) * dx) / length;
}

function lineQuadPoints(from, to, strokeWidth) {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy);
  if (length <= 1.0e-6) {
    return [from, to, to, from];
  }
  const halfWidth = Number(strokeWidth || 0) / 2;
  const nx = -dy / length;
  const ny = dx / length;
  return [
    { x: from.x + nx * halfWidth, y: from.y + ny * halfWidth },
    { x: to.x + nx * halfWidth, y: to.y + ny * halfWidth },
    { x: to.x - nx * halfWidth, y: to.y - ny * halfWidth },
    { x: from.x - nx * halfWidth, y: from.y - ny * halfWidth },
  ];
}

function primitiveStrokeWidthValue(primitive, fallback = 0) {
  const strokeWidth = primitive?.strokeWidth ?? primitive?.stroke_width;
  const numeric = Number(strokeWidth);
  return Number.isFinite(numeric) ? numeric : fallback;
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

function preferredZoomCenterWorld() {
  const selectionCenter = selectionZoomCenterWorld();
  if (selectionCenter) {
    return selectionCenter;
  }
  if (isEditingRustDocument() && state.lastEditFocusPoint) {
    return { x: state.lastEditFocusPoint.x, y: state.lastEditFocusPoint.y };
  }
  return currentViewportCenterWorld();
}

function clampZoomPercent(value) {
  return Math.max(ZOOM_MIN_PERCENT, Math.min(ZOOM_MAX_PERCENT, Math.round(value)));
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
  viewerContainer.scrollLeft = Math.max(0, (point.x - viewBox.x) * scale - offsetX);
  viewerContainer.scrollTop = Math.max(0, (point.y - viewBox.y) * scale - offsetY);
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
      viewerContainer.scrollLeft += scrollDelta.x * viewportScale();
      viewerContainer.scrollTop += scrollDelta.y * viewportScale();
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
  return Math.max(25, Math.min(400, Math.round((scale / CSS_PX_PER_CM) * 100)));
}

function extendBounds(bounds, minX, minY, maxX, maxY) {
  if (!Number.isFinite(minX) || !Number.isFinite(minY) || !Number.isFinite(maxX) || !Number.isFinite(maxY)) {
    return bounds;
  }
  if (!bounds) {
    return { minX, minY, maxX, maxY };
  }
  return {
    minX: Math.min(bounds.minX, minX),
    minY: Math.min(bounds.minY, minY),
    maxX: Math.max(bounds.maxX, maxX),
    maxY: Math.max(bounds.maxY, maxY),
  };
}

function boundsFromPrimitive(primitive) {
  const strokeWidth = primitiveStrokeWidthValue(primitive, 0);
  const halfStroke = strokeWidth / 2;
  if (primitive.kind === "line" && primitive.from && primitive.to) {
    return {
      minX: Math.min(primitive.from.x, primitive.to.x) - halfStroke,
      minY: Math.min(primitive.from.y, primitive.to.y) - halfStroke,
      maxX: Math.max(primitive.from.x, primitive.to.x) + halfStroke,
      maxY: Math.max(primitive.from.y, primitive.to.y) + halfStroke,
    };
  }
  if ((primitive.kind === "polygon" || primitive.kind === "polyline") && Array.isArray(primitive.points) && primitive.points.length) {
    const xs = primitive.points.map((point) => point.x);
    const ys = primitive.points.map((point) => point.y);
    return {
      minX: Math.min(...xs) - halfStroke,
      minY: Math.min(...ys) - halfStroke,
      maxX: Math.max(...xs) + halfStroke,
      maxY: Math.max(...ys) + halfStroke,
    };
  }
  if (primitive.kind === "rect") {
    return {
      minX: primitive.x - halfStroke,
      minY: primitive.y - halfStroke,
      maxX: primitive.x + primitive.width + halfStroke,
      maxY: primitive.y + primitive.height + halfStroke,
    };
  }
  if (primitive.kind === "text") {
    const fontSize = Number(primitive.fontSize || primitive.font_size || DEFAULT_TEXT_FONT_SIZE);
    const text = String(primitive.text || "");
    const runs = Array.isArray(primitive.runs) && primitive.runs.length
      ? primitive.runs
      : [{ text, fontSize, script: "normal" }];
    const width = Math.max(fontSize * 0.6, estimateTextRunsWidth(runs, fontSize));
    const anchor = primitive.textAnchor || primitive.text_anchor || "start";
    const x = Number(primitive.x || 0);
    const y = Number(primitive.y || 0);
    const minX = anchor === "middle" ? x - width / 2 : anchor === "end" ? x - width : x;
    return {
      minX,
      minY: y - fontSize * 0.86,
      maxX: minX + width,
      maxY: y + fontSize * 0.24,
    };
  }
  return null;
}

function boundsFromPrimitives(primitives) {
  let bounds = null;
  for (const primitive of primitives || []) {
    const primitiveBounds = boundsFromPrimitive(primitive);
    if (!primitiveBounds) {
      continue;
    }
    bounds = extendBounds(
      bounds,
      primitiveBounds.minX,
      primitiveBounds.minY,
      primitiveBounds.maxX,
      primitiveBounds.maxY,
    );
  }
  return bounds;
}

function paddedViewBoxFromBounds(bounds, paddingX, paddingY = paddingX, minWidth = 0, minHeight = 0) {
  const padded = {
    x: bounds.minX - paddingX,
    y: bounds.minY - paddingY,
    width: (bounds.maxX - bounds.minX) + paddingX * 2,
    height: (bounds.maxY - bounds.minY) + paddingY * 2,
  };
  if (padded.width < minWidth) {
    padded.x -= (minWidth - padded.width) / 2;
    padded.width = minWidth;
  }
  if (padded.height < minHeight) {
    padded.y -= (minHeight - padded.height) / 2;
    padded.height = minHeight;
  }
  return padded;
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
  if (!state.editorEngine) {
    return [];
  }
  return parseEngineJson(state.editorEngine.renderListJson(), []) || [];
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

function maybeAutoExpandEditorViewport(primitives) {
  if (!isEditingRustDocument()) {
    return false;
  }
  const bounds = boundsFromPrimitives(primitives);
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
    state.coreRenderList = parseEngineJson(state.documentEngine.renderListJson(), []) || [];
    return;
  }
  if (state.editorEngine) {
    state.coreRenderList = parseEngineJson(state.editorEngine.renderListJson(), []) || [];
  }
}

function corePrimitivesForObject(objectId) {
  return (state.coreRenderList || []).filter((primitive) => primitive.objectId === objectId);
}

function renderCorePrimitive(svgRoot, primitive) {
  if (primitive.kind === "line" && primitive.from && primitive.to) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      x1: primitive.from.x,
      y1: primitive.from.y,
      x2: primitive.to.x,
      y2: primitive.to.y,
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "data-bond-id": primitive.bondId || undefined,
    };
    if ((primitive.dashArray || primitive.dash_array)?.length) {
      attrs["stroke-dasharray"] = (primitive.dashArray || primitive.dash_array).join(" ");
    }
    if (primitive.role === "document-bond") {
      attrs.class = "mol-bond-stroked";
    }
    svgRoot.appendChild(makeSvgNode("line", attrs));
    return;
  }
  if (primitive.kind === "polyline" && Array.isArray(primitive.points)) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
      fill: "none",
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "stroke-dasharray": (primitive.dashArray || primitive.dash_array)?.join(" ") || undefined,
      "stroke-linecap": primitive.lineCap || primitive.line_cap || undefined,
      "stroke-linejoin": primitive.lineJoin || primitive.line_join || undefined,
      "data-bond-id": primitive.bondId || undefined,
    };
    if (primitive.role === "document-bond") {
      attrs.class = "mol-bond-stroked";
    }
    svgRoot.appendChild(makeSvgNode("polyline", attrs));
    return;
  }
  if (primitive.kind === "polygon" && Array.isArray(primitive.points)) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
      fill: primitive.fill || CHEMDRAW_INK,
      stroke: strokeWidth > 0 ? (primitive.stroke || primitive.fill || CHEMDRAW_INK) : "none",
      "stroke-width": strokeWidth,
      "data-bond-id": primitive.bondId || undefined,
    };
    if (primitive.role === "document-bond") {
      attrs.class = strokeWidth > 0 ? "mol-bond-stroked" : "mol-bond-filled";
    }
    svgRoot.appendChild(makeSvgNode("polygon", attrs));
    return;
  }
  if (primitive.kind === "rect") {
    if (primitive.role === "document-knockout" && !LABEL_DEBUG_MODE) {
      return;
    }
    const attrs = {
      x: primitive.x,
      y: primitive.y,
      width: primitive.width,
      height: primitive.height,
      fill: primitive.fill || "none",
      stroke: primitive.stroke || "none",
      "stroke-width": primitiveStrokeWidthValue(primitive, 1),
      rx: primitive.rx,
      ry: primitive.ry,
    };
    if (primitive.role === "document-knockout") {
      attrs.class = "label-knockout-shape";
    }
    const gradient = primitive.fillGradient || primitive.fill_gradient;
    if (gradient?.stops?.length) {
      const defs = ensureSvgDefs(svgRoot);
      const gradientId = `grad-core-${primitive.objectId || Math.random().toString(36).slice(2)}`;
      const linearGradient = makeSvgNode("linearGradient", {
        id: gradientId,
        x1: gradient.x1 || "0%",
        y1: gradient.y1 || "0%",
        x2: gradient.x2 || "0%",
        y2: gradient.y2 || "100%",
      });
      for (const stop of gradient.stops) {
        linearGradient.appendChild(makeSvgNode("stop", {
          offset: stop.offset,
          "stop-color": stop.color,
        }));
      }
      defs.appendChild(linearGradient);
      attrs.fill = `url(#${gradientId})`;
    }
    if ((primitive.dashArray || primitive.dash_array)?.length) {
      attrs["stroke-dasharray"] = (primitive.dashArray || primitive.dash_array).join(" ");
    }
    svgRoot.appendChild(makeSvgNode("rect", attrs));
    return;
  }
  if (primitive.kind === "text") {
    const textNode = makeSvgNode("text", {
      x: primitive.x,
      y: primitive.y,
      class: "chem-text",
      "font-size": primitive.fontSize || primitive.font_size || DEFAULT_TEXT_FONT_SIZE,
      "dominant-baseline": "alphabetic",
      "text-anchor": primitive.textAnchor || primitive.text_anchor || "start",
      fill: primitive.fill ? normalizeDisplayColor(primitive.fill) : undefined,
      "font-family": primitive.fontFamily
        ? displayLabelFontFamily(primitive.fontFamily)
        : primitive.font_family
          ? displayLabelFontFamily(primitive.font_family)
          : undefined,
    });
    if (Array.isArray(primitive.runs) && primitive.runs.length) {
      for (const run of primitive.runs) {
        const runFontSize = Number(run.fontSize || primitive.fontSize || DEFAULT_TEXT_FONT_SIZE);
        const isSub = isSubscriptRun(run);
        const isSuper = isSuperscriptRun(run);
        const isSubOrSuper = isSub || isSuper;
        const scriptScale = isSub ? editorScriptScale("subscript") : isSuper ? editorScriptScale("superscript") : 1;
        const tspan = makeSvgNode("tspan", {
          fill: run.fill ? normalizeDisplayColor(run.fill) : undefined,
          "font-size": isSubOrSuper ? Math.max(cssPxToCm(7), runFontSize * scriptScale) : runFontSize,
          "font-family": run.fontFamily ? displayLabelFontFamily(run.fontFamily) : undefined,
          "font-weight": fontWeightForRun(run),
          "font-style": fontStyleForRun(run),
          "text-decoration": run.underline ? "underline" : undefined,
          "baseline-shift": isSub
            ? `-${editorGlyphLayoutConfig().subscriptShiftDownEm}em`
            : isSuper
              ? `${editorGlyphLayoutConfig().superscriptShiftUpEm}em`
              : undefined,
          dx: isSuper ? "-0.02em" : undefined,
        });
        tspan.textContent = run.text || "";
        textNode.appendChild(tspan);
      }
    } else {
      textNode.textContent = primitive.text || "";
    }
    svgRoot.appendChild(textNode);
  }
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

function setZoomPercent(nextZoom) {
  const centerWorld = preferredZoomCenterWorld();
  zoomPercent = clampZoomPercent(nextZoom);
  if (zoomInput) {
    zoomInput.value = `${zoomPercent}%`;
  }
  if (ensureEditorViewportCapacity(centerWorld)) {
    return;
  }
  applyViewerViewport({ centerWorld });
}

document.querySelectorAll("[data-command]").forEach((button) => {
  button.addEventListener("click", () => {
    const command = button.dataset.command;
    if (runEditorCommand(command)) {
      return;
    }
    if (command === "zoom-in") {
      setZoomPercent(nextZoomStep(1));
    } else if (command === "zoom-out") {
      setZoomPercent(nextZoomStep(-1));
    } else if (command === "fit") {
      fitView();
    } else if (command === "new") {
      state.currentPath = null;
      resetEditorEngine();
      renderDocument();
      fitView();
    }
  });
});

zoomInput?.addEventListener("change", () => {
  const parsed = Number.parseInt(String(zoomInput.value || "").replace(/[^\d]/g, ""), 10);
  setZoomPercent(Number.isFinite(parsed) ? parsed : zoomPercent);
});

const HOVER_ENDPOINT_SHORTCUT_LABELS = {
  h: "H",
  n: "N",
  o: "O",
  s: "S",
  P: "P",
  p: "Ph",
  f: "F",
  l: "Cl",
  b: "Br",
  i: "I",
  m: "Me",
  S: "Si",
  N: "Na",
  B: "B",
  d: "D",
};

function hoverEndpointShortcutLabelForEvent(event) {
  if (!isEditingRustDocument() || editorState.activeTool !== "bond") {
    return null;
  }
  if (event.ctrlKey || event.metaKey || event.altKey) {
    return null;
  }
  if (event.key === "c") {
    return "C";
  }
  return HOVER_ENDPOINT_SHORTCUT_LABELS[event.key] || null;
}

function runHoverEndpointShortcut(event) {
  const label = hoverEndpointShortcutLabelForEvent(event);
  if (!label) {
    return false;
  }
  const changed = state.editorEngine?.replaceHoveredEndpointLabel?.(label);
  if (!changed) {
    return false;
  }
  syncDocumentFromEngine();
  renderDocument();
  return true;
}

document.addEventListener("keydown", (event) => {
  const target = event.target;
  if (activeTextEditor?.root?.contains?.(target)) {
    if (event.key === "Escape") {
      finishActiveTextEditor(false);
      event.preventDefault();
    }
    return;
  }
  if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement) {
    return;
  }
  const commandKey = event.ctrlKey || event.metaKey;
  let command = null;
  if (commandKey && event.key.toLowerCase() === "z" && !event.shiftKey) {
    command = "undo";
  } else if ((commandKey && event.key.toLowerCase() === "y") || (commandKey && event.shiftKey && event.key.toLowerCase() === "z")) {
    command = "redo";
  } else if (event.key === "Delete" || event.key === "Backspace") {
    command = "delete";
  }
  if (command && runEditorCommand(command)) {
    event.preventDefault();
    return;
  }
  if (runHoverEndpointShortcut(event)) {
    event.preventDefault();
  }
});

function toolbarButton(value, title, svg, selected = false) {
  return `
    <button class="secondary-button${selected ? " is-selected" : ""}" type="button" data-secondary-value="${value}" aria-label="${title}" title="${title}">
      ${svg}
    </button>
  `;
}

function colorButton(value, title, color, selected = false) {
  const noFillClass = color === "none" ? " no-fill" : "";
  const swatchStyle = color === "none" ? "" : ` style="--swatch:${color}"`;
  return `
    <button class="color-button${selected ? " is-selected" : ""}" type="button" data-secondary-value="${value}" aria-label="${title}" title="${title}">
      <span class="color-swatch${noFillClass}"${swatchStyle}></span>
    </button>
  `;
}

function secondaryDivider() {
  return `<span class="secondary-divider" aria-hidden="true"></span>`;
}

const BOND_TOOL_ICON_SPECS = {
  single: {
    title: "Single bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 19 7"/></svg>`,
  },
  double: {
    title: "Double bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 15 18 6"/><path d="M6 18 19 9"/></svg>`,
  },
  triple: {
    title: "Triple bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4.5 14 17.5 5"/><path d="M6 17 19 8"/><path d="M7.5 20 20.5 11"/></svg>`,
  },
  dashed: {
    title: "Dashed bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 7 15.5"/><path d="M9.5 13.8 11.5 12.4"/><path d="M14 10.6 16 9.2"/><path d="M18.5 7.5 19 7"/></svg>`,
  },
  "dashed-double": {
    title: "Dashed-solid double bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4.3 16 18.3 6" style="stroke-linecap:butt"/><path d="M5.7 18 19.7 8" style="stroke-dasharray:2.2 1.6;stroke-linecap:butt"/></svg>`,
  },
  bold: {
    title: "Bold bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon class="filled" points="4.1,15.7 18.1,5.7 19.9,8.3 5.9,18.3" style="stroke-linejoin:miter"/></svg>`,
  },
  "bold-dashed": {
    title: "Hash bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5.8 15.4 8.2 18.8" style="stroke-width:1.9"/><path d="M9.6 12.7 12 16.1" style="stroke-width:1.9"/><path d="M13.4 10 15.8 13.4" style="stroke-width:1.9"/><path d="M17.2 7.3 19.6 10.7" style="stroke-width:1.9"/></svg>`,
  },
  wedge: {
    title: "Solid wedge",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon class="filled" points="3.2,14.5 6.8,19.5 19,7" style="stroke-linejoin:miter"/></svg>`,
  },
  "hashed-wedge": {
    title: "Hash wedge",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon class="filled" points="3.5,14.9 3.8,15.3 5.7,13.3 4.5,13.9" style="stroke:none"/><polygon class="filled" points="4.1,15.7 4.4,16.2 8.6,11.9 7,12.7" style="stroke:none"/><polygon class="filled" points="4.7,16.6 5.1,17.2 11.7,10.4 9.8,11.3" style="stroke:none"/><polygon class="filled" points="5.5,17.7 6,18.4 15.5,8.6 13.3,9.7" style="stroke:none"/></svg>`,
  },
};

function bondToolIconSpec(type = editorState.bondType) {
  return BOND_TOOL_ICON_SPECS[type] || BOND_TOOL_ICON_SPECS.single;
}

function syncPrimaryBondToolButton() {
  const bondButton = document.querySelector('.tool-button[data-tool="bond"]');
  if (!bondButton) {
    return;
  }
  const spec = bondToolIconSpec();
  bondButton.innerHTML = spec.svg;
  bondButton.setAttribute("aria-label", spec.title);
  bondButton.setAttribute("title", spec.title);
}

function syncCanvasCursor() {
  if (!viewerSvg) {
    return;
  }
  viewerSvg.style.cursor = editorState.activeTool === "text"
    ? "text"
    : editorState.activeTool === "delete"
      ? DELETE_CURSOR
    : editorState.activeTool === "select"
      ? "default"
      : "crosshair";
}

function selectToolbarHtml() {
  const mode = editorState.selectMode;
  return [
    toolbarButton("select-free", "Free selection", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6c5-4 14 1 13 7-1 7-12 7-14 1"/></svg>`, mode === "free"),
    toolbarButton("select-box", "Box selection", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" stroke-dasharray="2 2"/></svg>`, mode === "box"),
    secondaryDivider(),
    toolbarButton("align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 5v14"/><path d="M9 7h9"/><path d="M9 12h6"/><path d="M9 17h11"/></svg>`),
    toolbarButton("align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 5v14"/><path d="M6 7h9"/><path d="M9 12h6"/><path d="M4 17h11"/></svg>`),
    toolbarButton("align-top", "Align top", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 9v9"/><path d="M12 9v6"/><path d="M17 9v11"/></svg>`),
    toolbarButton("align-bottom", "Align bottom", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 18h14"/><path d="M7 6v9"/><path d="M12 9v6"/><path d="M17 4v11"/></svg>`),
    toolbarButton("align-center", "Center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path d="M4 12h16"/><rect x="8" y="8" width="8" height="8"/></svg>`),
    toolbarButton("align-h-center", "Horizontal center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path d="M6 7h12"/><path d="M8 12h8"/><path d="M5 17h14"/></svg>`),
    secondaryDivider(),
    toolbarButton("distribute", "Distribute spacing", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="4" height="4"/><rect x="15" y="5" width="4" height="4"/><rect x="10" y="15" width="4" height="4"/><path d="M7 12h10"/></svg>`),
    toolbarButton("distribute-h", "Horizontal distribute", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="8" width="4" height="8"/><rect x="10" y="8" width="4" height="8"/><rect x="16" y="8" width="4" height="8"/><path d="M4 19h16"/></svg>`),
    secondaryDivider(),
    toolbarButton("flip-h", "Flip horizontal", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path class="filled" d="M5 7v10l5-5z"/><path d="M19 7v10l-5-5z"/></svg>`),
    toolbarButton("flip-v", "Flip vertical", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h16"/><path class="filled" d="M7 5h10l-5 5z"/><path d="M7 19h10l-5-5z"/></svg>`),
  ].join("");
}

function bondToolbarHtml() {
  const type = editorState.bondType;
  return [
    toolbarButton("bond-single", bondToolIconSpec("single").title, bondToolIconSpec("single").svg, type === "single"),
    toolbarButton("bond-double", bondToolIconSpec("double").title, bondToolIconSpec("double").svg, type === "double"),
    toolbarButton("bond-triple", bondToolIconSpec("triple").title, bondToolIconSpec("triple").svg, type === "triple"),
    toolbarButton("bond-dashed", bondToolIconSpec("dashed").title, bondToolIconSpec("dashed").svg, type === "dashed"),
    toolbarButton("bond-dashed-double", bondToolIconSpec("dashed-double").title, bondToolIconSpec("dashed-double").svg, type === "dashed-double"),
    toolbarButton("bond-bold", bondToolIconSpec("bold").title, bondToolIconSpec("bold").svg, type === "bold"),
    toolbarButton("bond-bold-dashed", bondToolIconSpec("bold-dashed").title, bondToolIconSpec("bold-dashed").svg, type === "bold-dashed"),
    toolbarButton("bond-wedge", bondToolIconSpec("wedge").title, bondToolIconSpec("wedge").svg, type === "wedge"),
    toolbarButton("bond-hashed-wedge", bondToolIconSpec("hashed-wedge").title, bondToolIconSpec("hashed-wedge").svg, type === "hashed-wedge"),
  ].join("");
}

function textToolbarHtml() {
  const align = editorState.textAlign;
  const fontOptions = TEXT_FONT_OPTIONS
    .map((fontFamily) => (
      `<option value="${fontFamily}"${editorState.textFontFamily === fontFamily ? " selected" : ""}>${fontFamily}</option>`
    ))
    .join("");
  const normalizedFontSize = normalizeToolbarFontSize(editorState.textFontSize);
  const knownFontSizes = new Set(TEXT_FONT_SIZE_OPTIONS);
  const fontSizeOptions = [
    ...TEXT_FONT_SIZE_OPTIONS,
    ...(knownFontSizes.has(normalizedFontSize) ? [] : [normalizedFontSize]),
  ]
    .sort((left, right) => left - right)
    .map((fontSize) => (
      `<option value="${fontSize}"${normalizedFontSize === fontSize ? " selected" : ""}>${formatToolbarFontSize(fontSize)}</option>`
    ))
    .join("");
  return `
    <select class="secondary-select" data-text-control="font" aria-label="Font family">
      ${fontOptions}
    </select>
    <select class="secondary-select" data-text-control="size" aria-label="Font size">
      ${fontSizeOptions}
    </select>
    ${secondaryDivider()}
    ${colorButton("text-black", "Text color", "#000000", editorState.textColor === "#000000")}
    ${secondaryDivider()}
    ${toolbarButton("text-align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h9"/><path d="M5 14h12"/><path d="M5 18h8"/></svg>`, align === "left")}
    ${toolbarButton("text-align-center", "Align center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 10h10"/><path d="M6 14h12"/><path d="M8 18h8"/></svg>`, align === "center")}
    ${toolbarButton("text-align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M10 10h9"/><path d="M7 14h12"/><path d="M11 18h8"/></svg>`, align === "right")}
    ${toolbarButton("text-align-justify", "Justify", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h14"/><path d="M5 14h14"/><path d="M5 18h14"/></svg>`, align === "justify")}
    ${secondaryDivider()}
    ${toolbarButton("text-bold", "Bold", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5h5.4a3.1 3.1 0 0 1 0 6.2H8z"/><path d="M8 11.2h6.2a3.4 3.4 0 0 1 0 6.8H8z"/></svg>`, editorState.textBold)}
    ${toolbarButton("text-italic", "Italic", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M14 5h-4"/><path d="M14 19h-4"/><path d="M13 5 11 19"/></svg>`, editorState.textItalic)}
    ${toolbarButton("text-underline", "Underline", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v7a4 4 0 0 0 8 0V5"/><path d="M6 19h12"/></svg>`, editorState.textUnderline)}
    ${secondaryDivider()}
    ${toolbarButton("text-chemical", "Chemical", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="3.6" y="15.4" fill="currentColor" font-size="10.8" font-family="Arial, Helvetica, sans-serif" font-weight="700">CH</text><text x="16.1" y="18.1" fill="currentColor" font-size="6.4" font-family="Arial, Helvetica, sans-serif" font-weight="700">2</text><text x="15.8" y="9.1" fill="currentColor" font-size="5.8" font-family="Arial, Helvetica, sans-serif" font-weight="700">+</text></svg>`, editorState.textScript === "chemical")}
    ${toolbarButton("text-subscript", "Subscript", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="4.2" y="14.8" fill="currentColor" font-size="12.2" font-family="Arial, Helvetica, sans-serif" font-style="italic" font-weight="700">X</text><text x="15.6" y="18.1" fill="currentColor" font-size="7" font-family="Arial, Helvetica, sans-serif" font-weight="700">2</text></svg>`, editorState.textScript === "subscript")}
    ${toolbarButton("text-superscript", "Superscript", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="4.2" y="14.8" fill="currentColor" font-size="12.2" font-family="Arial, Helvetica, sans-serif" font-style="italic" font-weight="700">X</text><text x="15.4" y="9.1" fill="currentColor" font-size="7" font-family="Arial, Helvetica, sans-serif" font-weight="700">2</text></svg>`, editorState.textScript === "superscript")}
  `;
}

function shapeToolbarHtml() {
  return `
    ${colorButton("stroke-black", "Black border", "#000000", editorState.shapeStroke === "#000000")}
    ${colorButton("stroke-red", "Red border", "#ff0000", editorState.shapeStroke === "#ff0000")}
    ${colorButton("stroke-blue", "Blue border", "#0000ff", editorState.shapeStroke === "#0000ff")}
    ${secondaryDivider()}
    ${colorButton("fill-none", "No fill", "none", editorState.shapeFill === "none")}
    ${colorButton("fill-white", "White fill", "#ffffff", editorState.shapeFill === "#ffffff")}
    ${colorButton("fill-black", "Black fill", "#000000", editorState.shapeFill === "#000000")}
    ${colorButton("fill-gray", "Gray fill", "#808892", editorState.shapeFill === "#808892")}
    ${secondaryDivider()}
    ${toolbarButton("shape-rect", "Rectangle", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14"/></svg>`, editorState.shapeStyle === "rect")}
    ${toolbarButton("shape-dashed", "Dashed outline", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" stroke-dasharray="2 2"/></svg>`, editorState.shapeStyle === "dashed")}
    ${toolbarButton("shape-filled", "Filled rectangle", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect class="filled" x="5" y="5" width="14" height="14"/></svg>`, editorState.shapeStyle === "filled")}
    ${toolbarButton("shape-ellipse", "Ellipse", `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="7"/></svg>`, editorState.shapeStyle === "ellipse")}
  `;
}

function ringSvg(sides, aromatic = false) {
  if (aromatic) {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m12 4 7 4v8l-7 4-7-4V8z"/><path d="M8.5 9.5v5"/><path d="M15.5 9.5v5"/></svg>`;
  }
  const pointsBySide = {
    3: "12,4 20,18 4,18",
    4: "6,6 18,6 18,18 6,18",
    5: "12,4 20,10 17,19 7,19 4,10",
    6: "12,4 19,8 19,16 12,20 5,16 5,8",
    7: "12,4 18,7 20,14 16,20 8,20 4,14 6,7",
    8: "9,4 15,4 20,9 20,15 15,20 9,20 4,15 4,9",
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon points="${pointsBySide[sides]}"/></svg>`;
}

function templatesToolbarHtml() {
  return [
    toolbarButton("ring-3", "3-membered ring", ringSvg(3), editorState.template === "ring-3"),
    toolbarButton("ring-4", "4-membered ring", ringSvg(4), editorState.template === "ring-4"),
    toolbarButton("ring-5", "5-membered ring", ringSvg(5), editorState.template === "ring-5"),
    toolbarButton("ring-6", "6-membered ring", ringSvg(6), editorState.template === "ring-6"),
    toolbarButton("ring-7", "7-membered ring", ringSvg(7), editorState.template === "ring-7"),
    toolbarButton("ring-8", "8-membered ring", ringSvg(8), editorState.template === "ring-8"),
    secondaryDivider(),
    toolbarButton("benzene", "Benzene ring", ringSvg(6, true), editorState.template === "benzene"),
  ].join("");
}

function renderSecondaryToolbar() {
  if (!secondaryToolbar) {
    return;
  }
  if (editorState.activeTool === "bond") {
    secondaryToolbar.innerHTML = bondToolbarHtml();
  } else if (editorState.activeTool === "delete") {
    secondaryToolbar.innerHTML = "";
  } else if (editorState.activeTool === "text") {
    secondaryToolbar.innerHTML = textToolbarHtml();
  } else if (editorState.activeTool === "shape") {
    secondaryToolbar.innerHTML = shapeToolbarHtml();
  } else if (editorState.activeTool === "templates") {
    secondaryToolbar.innerHTML = templatesToolbarHtml();
  } else {
    secondaryToolbar.innerHTML = selectToolbarHtml();
  }
  syncPrimaryBondToolButton();
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
  scriptScale: editorScriptScale,
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
}

function openTextEditorSession(session) {
  textEditorController.openTextEditorSession(engineSessionToEditorSession(session));
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

function editorScriptScale(script) {
  return computeEditorScriptScale(sharedGlyphProfiles, script);
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

function setActiveTool(toolButton) {
  const nextTool = toolButton?.dataset?.tool || editorState.activeTool;
  if (editorState.activeTool === "text" && nextTool !== "text") {
    finishActiveTextEditor(true);
  }
  if (editorState.activeTool === "select" && nextTool !== "select") {
    activeSelectionGesture = null;
  }
  editorState.activeTool = toolButton?.dataset?.tool || editorState.activeTool;
  document.querySelectorAll("[data-tool]").forEach((button) => {
    button.classList.toggle("is-active", button.dataset.tool === editorState.activeTool);
  });
  syncEngineToolState();
  renderSecondaryToolbar();
  syncCanvasCursor();
  if (isEditingRustDocument()) {
    renderEditorOverlay(currentEditorRenderList());
  }
}

document.querySelectorAll("[data-tool]").forEach((button) => {
  button.addEventListener("click", () => {
    setActiveTool(button);
  });
});

secondaryToolbar?.addEventListener("click", (event) => {
  const button = event.target.closest("[data-secondary-value]");
  if (!button) {
    return;
  }
  const value = button.dataset.secondaryValue;
  if (value?.startsWith("text-align-")) {
    editorState.textAlign = value.replace("text-align-", "");
    applyTextAlignment(editorState.textAlign);
  } else if (value === "text-bold") {
    editorState.textBold = !editorState.textBold;
    applyTextFormatCommand("bold");
  } else if (value === "text-italic") {
    editorState.textItalic = !editorState.textItalic;
    applyTextFormatCommand("italic");
  } else if (value === "text-underline") {
    editorState.textUnderline = !editorState.textUnderline;
    applyTextFormatCommand("underline");
  } else if (value === "text-chemical") {
    editorState.textScript = "chemical";
    applyChemicalFormat();
  } else if (value === "text-subscript") {
    editorState.textScript = "subscript";
    applyTextScript("subscript");
  } else if (value === "text-superscript") {
    editorState.textScript = "superscript";
    applyTextScript("superscript");
  } else if (value?.startsWith("text-")) {
    const colors = { "text-black": "#000000", "text-red": "#ff0000", "text-blue": "#0000ff", "text-green": "#0a8f3c" };
    editorState.textColor = colors[value] || editorState.textColor;
    applyTextInlineStyle({ color: editorState.textColor });
  } else if (value === "select-free" || value === "select-box") {
    editorState.selectMode = value.replace("select-", "");
  } else if (value?.startsWith("bond-")) {
    editorState.bondType = value.replace("bond-", "");
  } else if (value?.startsWith("shape-")) {
    editorState.shapeStyle = value.replace("shape-", "");
  } else if (value?.startsWith("ring-") || value === "benzene") {
    editorState.template = value;
  } else if (value?.startsWith("stroke-")) {
    const colors = { "stroke-black": "#000000", "stroke-red": "#ff0000", "stroke-blue": "#0000ff" };
    editorState.shapeStroke = colors[value] || editorState.shapeStroke;
  } else if (value?.startsWith("fill-")) {
    const fills = { "fill-none": "none", "fill-white": "#ffffff", "fill-black": "#000000", "fill-gray": "#808892" };
    editorState.shapeFill = fills[value] || editorState.shapeFill;
  }
  syncEngineToolState();
  renderSecondaryToolbar();
  focusActiveTextEditor();
});

secondaryToolbar?.addEventListener("change", (event) => {
  const target = event.target;
  if (!(target instanceof HTMLInputElement || target instanceof HTMLSelectElement)) {
    return;
  }
  const control = target.dataset.textControl;
  if (control === "font") {
    editorState.textFontFamily = target.value || editorState.textFontFamily;
    applyTextInlineStyle({ fontFamily: editorState.textFontFamily });
  } else if (control === "size") {
    const size = Number(target.value || editorState.textFontSize);
    if (Number.isFinite(size) && size > 0) {
      editorState.textFontSize = normalizeToolbarFontSize(Math.max(5, Math.min(288, size)));
      applyTextInlineStyle({ fontSize: `${editorState.textFontSize}px` });
    }
  }
  renderSecondaryToolbar();
  focusActiveTextEditor();
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
      || editorState.activeTool === "select"
      || editorState.activeTool === "text");
}

function isDocumentPreviewPrimitive(primitive) {
  return primitive?.role === "document-bond"
    || primitive?.role === "document-graphic"
    || primitive?.role === "document-knockout"
    || primitive?.role === "document-text";
}

function handleEditorPointerMove(event) {
  const point = svgPointFromEvent(event);
  if (editorState.activeTool === "select" && activeSelectionGesture) {
    event.preventDefault();
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
  if (editorState.activeTool === "text") {
    event.preventDefault();
    openTextEditorAt(point);
    return;
  }
  if (editorState.activeTool === "select") {
    event.preventDefault();
    viewerSvg.setPointerCapture?.(event.pointerId);
    state.editorEngine.pointerMove(point.x, point.y, event.altKey);
    activeSelectionGesture = {
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
  if (editorState.activeTool === "select") {
    const gesture = activeSelectionGesture;
    activeSelectionGesture = null;
    if (!gesture) {
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
    renderDocument();
    return;
  }
  state.editorEngine.pointerUp(point.x, point.y, event.altKey);
  syncDocumentFromEngine();
  renderDocument();
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
    if (isDocumentPreviewPrimitive(primitive)) {
      if (previewActive) {
        renderCorePrimitive(overlay, primitive);
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
viewerSvg?.addEventListener("pointercancel", () => {
  state.editorEngine?.clearInteraction?.();
  renderEditorOverlay();
});
viewerSvg?.addEventListener("pointerleave", handleEditorPointerLeave);
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
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive));
      }
    } else if (object.type === "shape") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive));
      } else {
        renderShapeObject(viewerSvg, object, documentData.styles);
      }
    } else if (object.type === "line") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive));
      } else {
        renderLineObject(viewerSvg, object, state.currentDocument.styles);
      }
    } else if (object.type === "text") {
      const corePrimitives = corePrimitivesForObject(object.id);
      if (corePrimitives.length) {
        corePrimitives.forEach((primitive) => renderCorePrimitive(viewerSvg, primitive));
      } else {
        renderTextObject(viewerSvg, object);
      }
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
  let nextViewBox;
  let fitTargetBox = null;
  if (isEditingRustDocument()) {
    const bounds = boundsFromPrimitives(state.coreRenderList || []);
    if (!bounds) {
      nextViewBox = defaultEditorViewBox();
      state.runtimeViewBox = nextViewBox;
      zoomPercent = 100;
      if (zoomInput) {
        zoomInput.value = `${zoomPercent}%`;
      }
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
  if (zoomInput) {
    zoomInput.value = `${zoomPercent}%`;
  }
  const target = fitTargetBox || nextViewBox;
  applyViewerViewport({ centerWorld: { x: target.x + target.width / 2, y: target.y + target.height / 2 } });
}

async function loadDocument(path) {
  const response = await fetch(path, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`Failed to load ${path}: ${response.status}`);
  }
  return response.json();
}

function currentDocumentMetaPayload() {
  if (!state.currentDocument) {
    return null;
  }
  return {
    sample: state.currentPath || "blank",
    page: state.currentDocument.document.page,
    meta: state.currentDocument.document.meta,
    display: state.displayMetrics,
  };
}

function updateDocumentMeta() {
  const payload = currentDocumentMetaPayload();
  if (!docMeta || !payload) {
    return;
  }
  docMeta.textContent = JSON.stringify(payload, null, 2);
}

async function loadAndRender() {
  finishActiveTextEditor(false);
  viewerTitle.textContent = "Loading...";
  try {
    if (state.currentPath) {
      const documentData = await loadDocument(state.currentPath);
      state.currentDocument = documentData;
      state.runtimeViewBox = pageViewBox(documentData.document.page);
      syncCoreRenderListFromCurrentDocument();
    } else {
      state.coreRenderList = null;
      if (!state.editorEngine) {
        resetEditorEngine();
      } else {
        state.editorEngine.clearInteraction();
        syncEngineToolState();
        syncDocumentFromEngine();
      }
    }
    const documentData = state.currentDocument;
    state.currentDocument = documentData;
    viewerTitle.textContent = documentData.document.title || state.currentPath;
    updateDocumentMeta();
    renderDocument();
    fitView();
  } catch (error) {
    viewerTitle.textContent = "Load failed";
    viewerStats.textContent = "";
    docMeta.textContent = String(error);
    viewerSvg.innerHTML = "";
  }
}

watchDisplayMetrics();

try {
  await Promise.all([initializeChemcoreEngine(), sharedGlyphProfilesReady]);
  await loadAndRender();
} catch (error) {
  viewerTitle.textContent = "Runtime load failed";
  viewerStats.textContent = "";
  docMeta.textContent = String(error);
  viewerSvg.innerHTML = "";
}
