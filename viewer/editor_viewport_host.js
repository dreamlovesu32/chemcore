import {
  boundsCenter,
  boundsSize,
  boundsToKey,
  intersectBounds,
  paddedViewBoxFromBounds,
  rectContainsBounds,
} from "./geometry.js";
import { displayMetrics, setDisplayScaleOverride } from "./units.js";

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

export function createEditorViewportHost(options) {
  const {
    state,
    viewerSvg,
    viewerContainer,
    zoomInput,
    textEditorLayer,
  } = options;
  const isEditingRustDocument = (...args) => options.isEditingRustDocument(...args);
  const currentRenderBounds = (...args) => options.currentRenderBounds(...args);
  const renderActiveTextEditorFromModel = (...args) => options.renderActiveTextEditorFromModel(...args);
  const currentEditorSelectionOffsets = (...args) => options.currentEditorSelectionOffsets(...args);
  const positionActiveTextEditor = (...args) => options.positionActiveTextEditor(...args);
  const updateDocumentMeta = (...args) => options.updateDocumentMeta(...args);
  const getActiveTextEditor = () => options.getActiveTextEditor?.() || null;
  let zoomPercent = 100;

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

  function clientPointToWorld(clientX, clientY) {
    if (!viewerContainer) {
      return currentViewportCenterWorld();
    }
    const rect = viewerContainer.getBoundingClientRect();
    const viewBox = activeViewBox();
    const scale = viewportScale();
    if (scale <= 0) {
      return currentViewportCenterWorld();
    }
    return {
      x: viewBox.x + (viewerContainer.scrollLeft + clientX - rect.left) / scale,
      y: viewBox.y + (viewerContainer.scrollTop + clientY - rect.top) / scale,
    };
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
    const anchorWorld = options.anchorWorld;
    const anchorClient = options.anchorClient;
    if (!viewerContainer || (!scrollDelta && !centerWorld && !anchorWorld)) {
      if (getActiveTextEditor()?.root) {
        renderActiveTextEditorFromModel(currentEditorSelectionOffsets());
      }
      positionActiveTextEditor();
      return;
    }
    requestAnimationFrame(() => {
      if (getActiveTextEditor()?.root) {
        renderActiveTextEditorFromModel(currentEditorSelectionOffsets());
      }
      if (anchorWorld && anchorClient) {
        scrollViewerToWorldPointAtClient(anchorWorld, anchorClient.x, anchorClient.y);
        positionActiveTextEditor();
        return;
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

  function ensureEditorViewportCapacity(
    centerWorld = currentViewportCenterWorld(),
    viewportOptions = null,
  ) {
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
    setRuntimeViewBox(next, viewportOptions || { centerWorld });
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
    const anchorWorld = options.anchorWorld || null;
    const anchorClient = options.anchorClient || null;
    const { centerWorld, handoff } = options.centerWorld
      ? { centerWorld: options.centerWorld, handoff: null }
      : anchorWorld && anchorClient
        ? { centerWorld: currentViewportCenterWorld(), handoff: null }
        : planZoomCenter(targetZoom);
    zoomPercent = targetZoom;
    syncZoomControl();
    if (anchorWorld && anchorClient) {
      clearZoomHandoffs();
    }
    if (handoff) {
      state.zoomHandoffs.push(handoff);
    } else if (targetZoom > previousZoom) {
      const last = state.zoomHandoffs[state.zoomHandoffs.length - 1];
      if (last && last.toZoom < targetZoom) {
        last.toZoom = targetZoom;
      }
    }
    const viewportOptions = anchorWorld && anchorClient
      ? { anchorWorld, anchorClient }
      : { centerWorld };
    if (ensureEditorViewportCapacity(centerWorld, viewportOptions)) {
      return;
    }
    applyViewerViewport(viewportOptions);
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
    const selectionBounds = isEditingRustDocument() ? currentRenderBounds("selection") : null;
    if (selectionBounds) {
      setZoomPercent(nextZoomStep(direction));
      return;
    }
    setZoomPercent(nextZoomStep(direction), {
      anchorWorld: clientPointToWorld(event.clientX, event.clientY),
      anchorClient: {
        x: event.clientX,
        y: event.clientY,
      },
    });
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

  function getZoomPercent() {
    return zoomPercent;
  }

  function setStoredZoomPercent(value = 100) {
    const numeric = Number(value);
    zoomPercent = Number.isFinite(numeric) && numeric > 0 ? numeric : 100;
  }

  return {
    cloneViewBox,
    pageViewBox,
    visibleWorldSize,
    viewportScaleForZoom,
    visibleWorldRect,
    visibleWorldRectForCenter,
    editorViewportMetrics,
    defaultEditorViewBox,
    activeViewBox,
    viewportScale,
    zoomScale,
    refreshDisplayMetrics,
    applyDisplayScaleOverride,
    watchDisplayMetrics,
    currentViewportCenterWorld,
    worldToScreenPoint,
    worldToLayerPoint,
    documentContentBoundsForZoom,
    zoomFocusBounds,
    clearZoomHandoffs,
    markProgrammaticScroll,
    rememberProgrammaticScrollPosition,
    isExpectedProgrammaticScroll,
    constrainZoomCenterForBounds,
    clampZoomPercent,
    closestZoomStep,
    zoomStepAtOrBelow,
    syncZoomControl,
    nextZoomStep,
    scrollViewerToWorldPoint,
    scrollViewerToWorldPointAtClient,
    clientPointToWorld,
    applyViewerViewport,
    setRuntimeViewBox,
    fitZoomPercentForViewBox,
    editorCanvasViewBoxFromBounds,
    ensureEditorViewportCapacity,
    maybeAutoExpandEditorViewport,
    planZoomCenter,
    setZoomPercent,
    handleViewerWheel,
    fitView,
    getZoomPercent,
    setStoredZoomPercent,
  };
}
