import {
  toolSupportsSelectionBoxMove,
  toolUsesEngineDragPreview,
} from "./editor_tool_model.js";
import {
  BRACKET_LABEL_OPEN_DRAG_THRESHOLD_SCREEN_PX,
  DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX,
  SELECTION_DRAG_THRESHOLD_SCREEN_PX,
  SELECTION_FREEHAND_POINT_SPACING_SCREEN_PX,
  SELECTION_POST_COMMIT_HOVER_BLOCK_SCREEN_PX,
  TLC_SPOT_DRAG_THRESHOLD_SCREEN_PX,
  selectionHandleZoneContainsPoint as selectionHandleZoneContainsPointModel,
} from "./editor_selection_hit_model.js";

export function createEditorPointerController(options) {
  let hoverMoveRequest = null;
  let hoverMoveFrame = 0;
  let hoverMoveTimer = 0;
  let hoverMoveRunning = false;
  let hoverMoveVersion = 0;
  let selectionHoverSuppressionActive = false;
  let documentPreviewFrame = 0;
  let documentPreviewRunning = false;
  let engineDragPreviewFrame = 0;
  let engineDragPreviewRunning = false;
  let engineDragPreviewRequest = null;
  let engineDragPreviewVersion = 0;
  let engineCreationDrag = null;
  let postCommitHoverBlockPoint = null;

  async function executeDocumentCommand(command, apply, executeOptions = {}) {
    if (options.commandEngine?.executeEngineCommand) {
      return options.commandEngine.executeEngineCommand(command, apply, executeOptions);
    }
    const applyResult = apply();
    const rawResult = applyResult && typeof applyResult.then === "function" ? await applyResult : applyResult;
    if (rawResult && !(executeOptions.sync === false && executeOptions.deferDocumentSync)) {
      await options.syncDocumentFromEngine({
        syncRenderList: executeOptions.syncRenderList !== false,
      });
    }
    return {
      changed: !!rawResult,
      rawResult,
    };
  }

  function pointerCommitCommandType() {
    const tool = options.editorState().activeTool;
    if (tool === "bond") {
      return "add-bond";
    }
    if (tool === "arrow") {
      return "add-arrow";
    }
    if (tool === "shape" || tool === "tlc-plate" || tool === "orbital") {
      return "add-shape";
    }
    if (tool === "bracket") {
      return "add-bracket";
    }
    if (tool === "symbol") {
      return "add-symbol";
    }
    if (tool === "element") {
      return "add-element";
    }
    if (tool === "templates" || tool === "chain") {
      return "insert-template";
    }
    if (tool === "delete") {
      return "delete-selection";
    }
    return "pointer-document-edit";
  }

  function cancelScheduledHoverMove() {
    hoverMoveRequest = null;
    hoverMoveVersion += 1;
    if (hoverMoveTimer) {
      clearTimeout(hoverMoveTimer);
      hoverMoveTimer = 0;
    }
    if (hoverMoveFrame) {
      cancelAnimationFrame(hoverMoveFrame);
      hoverMoveFrame = 0;
    }
  }

  function cancelDocumentPreviewFrame() {
    if (documentPreviewFrame) {
      cancelAnimationFrame(documentPreviewFrame);
      documentPreviewFrame = 0;
    }
  }

  function cancelEngineDragPreviewFrame() {
    engineDragPreviewVersion += 1;
    engineDragPreviewRequest = null;
    if (engineDragPreviewFrame) {
      cancelAnimationFrame(engineDragPreviewFrame);
      engineDragPreviewFrame = 0;
    }
  }

  function scheduleHoverPointerMove(point, altKey, delayMs = 0) {
    hoverMoveRequest = { point, altKey };
    hoverMoveVersion += 1;
    if (hoverMoveTimer || hoverMoveFrame || hoverMoveRunning) {
      return;
    }
    if (delayMs > 0) {
      hoverMoveTimer = setTimeout(() => {
        hoverMoveTimer = 0;
        if (!hoverMoveFrame && !hoverMoveRunning && hoverMoveRequest) {
          hoverMoveFrame = requestAnimationFrame(drainScheduledHoverPointerMove);
        }
      }, delayMs);
      return;
    }
    if (!hoverMoveFrame) {
      hoverMoveFrame = requestAnimationFrame(drainScheduledHoverPointerMove);
    }
  }

  function hoverMoveStale(version) {
    return version !== hoverMoveVersion || !!options.activeSelectionGesture();
  }

  function suppressHoverUntilPointerLeavesPoint(point) {
    postCommitHoverBlockPoint = point || null;
    cancelScheduledHoverMove();
  }

  function hoverBlockedAtPoint(point) {
    if (!postCommitHoverBlockPoint || !point) {
      return false;
    }
    if (options.pointDistance(postCommitHoverBlockPoint, point) <= options.cssPxToPt(SELECTION_POST_COMMIT_HOVER_BLOCK_SCREEN_PX)) {
      return true;
    }
    postCommitHoverBlockPoint = null;
    return false;
  }

  function toolCanHoverSuppressSelection(tool) {
    return tool === "select" || toolSupportsSelectionBoxMove(tool);
  }

  function selectionHoverSuppressionState(point) {
    const editorState = options.editorState();
    if (editorState.elementPlacementActive
      || !toolCanHoverSuppressSelection(editorState.activeTool)) {
      return null;
    }
    const selectionBounds = options.currentRenderBounds?.("selection");
    const overSelectionBounds = !!options.selectionBoundsContainsPoint?.(point);
    const overSelectionHit = !!options.selectionHitContainsPoint?.(point);
    const inHandleZone = !!selectionBounds
      && editorState.activeTool === "select"
      && selectionHandleZoneContainsPoint(point);
    if (!overSelectionBounds && !overSelectionHit && !inHandleZone) {
      return null;
    }
    return { overSelectionBounds, overSelectionHit, inHandleZone };
  }

  function clearVisibleHoverOverlay() {
    const viewerSvg = options.viewerSvg?.();
    const overlay = viewerSvg?.querySelector('[data-layer="editor-overlay"]');
    if (overlay?.querySelector('[data-role^="hover-"], [data-role^="preview-"]')) {
      overlay
        .querySelectorAll('[data-role^="hover-"], [data-role^="preview-"]')
        .forEach((node) => node.remove());
      if (!overlay.childNodes.length) {
        overlay.remove();
      }
      return true;
    }
    return false;
  }

  function clearEditorOverlayRoot() {
    const viewerSvg = options.viewerSvg?.();
    const overlay = viewerSvg?.querySelector('[data-layer="editor-overlay"]');
    if (!overlay) {
      return false;
    }
    overlay.remove();
    return true;
  }

  async function clearEngineHoverOverlay({ keepSelectionOverlay = false } = {}) {
    cancelScheduledHoverMove();
    options.clearTlcHoverState?.();
    await options.state().editorEngine?.clearInteraction?.();
    invalidateEngineReadCache();
    if (keepSelectionOverlay) {
      await options.renderSelectionOnlyUpdate?.(null, null);
    } else {
      clearEditorOverlayRoot();
    }
  }

  function clearInteractionOverlayNow() {
    cancelScheduledHoverMove();
    const clearedOverlay = clearEditorOverlayRoot();
    const clearedDragPreview = !!options.clearDragCapturePreview?.();
    return clearedOverlay || clearedDragPreview;
  }

  function clearInteractionOverlayBeforeCommit() {
    const cleared = clearInteractionOverlayNow();
    if (cleared && window.__chemcoreDebug) {
      window.__chemcoreDebug.creationPreviewClearedBeforeCommitAt = performance.now();
    }
    return cleared;
  }

  function waitForNextFrame() {
    if (typeof requestAnimationFrame !== "function") {
      return Promise.resolve();
    }
    return new Promise((resolve) => requestAnimationFrame(resolve));
  }

  async function waitForClearedInteractionPaint() {
    await waitForNextFrame();
    await waitForNextFrame();
  }

  async function clearPostCommitInteraction(point = null) {
    suppressHoverUntilPointerLeavesPoint(point);
    cancelEngineDragPreviewFrame();
    await options.state().editorEngine?.clearInteraction?.();
    invalidateEngineReadCache();
    clearEditorOverlayRoot();
    options.clearDragCapturePreview?.();
    options.clearTlcHoverState?.();
    options.syncCanvasCursor?.();
  }

  function recordCreationCommitTiming(sample) {
    const debug = window.__chemcoreDebug;
    if (!debug) {
      return;
    }
    const stats = debug.creationCommitStats || { samples: [] };
    stats.samples.push(sample);
    if (stats.samples.length > 120) {
      stats.samples.splice(0, stats.samples.length - 120);
    }
    stats.last = sample;
    debug.creationCommitStats = stats;
  }

  function recordInteractionCommitTiming(sample) {
    const debug = window.__chemcoreDebug;
    if (!debug) {
      return;
    }
    const stats = debug.interactionCommitStats || { samples: [] };
    stats.samples.push(sample);
    if (stats.samples.length > 120) {
      stats.samples.splice(0, stats.samples.length - 120);
    }
    stats.last = sample;
    debug.interactionCommitStats = stats;
  }

  function scheduleDocumentPreviewFrame() {
    if (documentPreviewFrame || documentPreviewRunning) {
      return;
    }
    documentPreviewFrame = requestAnimationFrame(async () => {
      documentPreviewFrame = 0;
      documentPreviewRunning = true;
      const gesture = options.activeSelectionGesture();
      if (!gesture || (!gesture.localDocumentPreviewActive && !gesture.backendDocumentPreviewActive)) {
        documentPreviewRunning = false;
        return;
      }
      try {
        if (gesture.backendDocumentPreviewActive) {
          if (window.__chemcoreDebug) {
            const stats = window.__chemcoreDebug.backendPreviewSchedulerStats || { runs: 0, backendRuns: 0, errors: [] };
            stats.runs += 1;
            stats.backendRuns += 1;
            window.__chemcoreDebug.backendPreviewSchedulerStats = stats;
          }
          await options.applyBackendSelectionMovePreview?.(gesture.current, gesture.altKey);
          clearEditorOverlayRoot();
        } else if (options.applyDocumentObjectPreviewTransform()) {
          if (window.__chemcoreDebug) {
            const stats = window.__chemcoreDebug.backendPreviewSchedulerStats || { runs: 0, backendRuns: 0, errors: [] };
            stats.runs += 1;
            window.__chemcoreDebug.backendPreviewSchedulerStats = stats;
          }
          clearEditorOverlayRoot();
        }
      } catch (error) {
        if (window.__chemcoreDebug) {
          const stats = window.__chemcoreDebug.backendPreviewSchedulerStats || { runs: 0, backendRuns: 0, errors: [] };
          stats.errors.push(String(error?.stack || error?.message || error));
          window.__chemcoreDebug.backendPreviewSchedulerStats = stats;
        }
        throw error;
      } finally {
        documentPreviewRunning = false;
        if (options.activeSelectionGesture() === gesture && gesture.previewDirty) {
          gesture.previewDirty = false;
          scheduleDocumentPreviewFrame();
        }
      }
    });
  }

  function setSelectionHoverSuppressionCursor(cursor) {
    const viewerSvg = options.viewerSvg?.();
    if (options.setCanvasCursorStyle) {
      options.setCanvasCursorStyle(cursor);
    } else if (viewerSvg) {
      viewerSvg.style.cursor = cursor;
    }
  }

  function syncSelectionHoverSuppressionCursor(point, state) {
    const viewerSvg = options.viewerSvg?.();
    if (!viewerSvg) {
      return;
    }
    if (options.editorState().activeTool === "select") {
      const resizeHandle = state.inHandleZone ? options.selectionResizeHandleHit(point) : null;
      if (resizeHandle) {
        setSelectionHoverSuppressionCursor(resizeHandle.cursor);
        return;
      }
      if (state.inHandleZone && options.selectionRotateHandleHit(point)) {
        setSelectionHoverSuppressionCursor("grab");
        return;
      }
    }
    if (state.overSelectionHit || state.overSelectionBounds) {
      setSelectionHoverSuppressionCursor("grab");
      return;
    }
    if (state.inHandleZone) {
      setSelectionHoverSuppressionCursor("grab");
      return;
    }
    options.syncCanvasCursor?.();
  }

  async function syncDeferredDocumentModelAfterCommit() {
    invalidateEngineReadCache();
    await options.syncDocumentFromEngine?.({
      syncRenderList: false,
      refreshSnapshot: false,
    });
    invalidateEngineReadCache();
  }

  function enterSelectionHoverSuppression(point, state) {
    selectionHoverSuppressionActive = true;
    cancelScheduledHoverMove();
    options.clearTlcHoverState();
    syncSelectionHoverSuppressionCursor(point, state);
    clearVisibleHoverOverlay();
    options.positionActiveTextEditor();
  }

  function leaveSelectionHoverSuppression(point = null) {
    if (!selectionHoverSuppressionActive) {
      return;
    }
    selectionHoverSuppressionActive = false;
    if (point && options.editorState().activeTool === "select") {
      const overSelectionBounds = !!options.selectionBoundsContainsPoint?.(point);
      const overSelectionHit = !!options.selectionHitContainsPoint?.(point);
      const inHandleZone = selectionHandleZoneContainsPoint(point);
      if ((overSelectionBounds || overSelectionHit) && !inHandleZone) {
        setSelectionHoverSuppressionCursor("grab");
        return;
      }
      const resizeHandle = inHandleZone ? options.selectionResizeHandleHit(point) : null;
      if (resizeHandle) {
        setSelectionHoverSuppressionCursor(resizeHandle.cursor);
        return;
      }
      if (options.selectionRotateHandleHit(point)) {
        setSelectionHoverSuppressionCursor("grab");
        return;
      }
      if (overSelectionBounds || overSelectionHit || inHandleZone) {
        setSelectionHoverSuppressionCursor("grab");
        return;
      }
    }
    options.syncCanvasCursor?.();
  }

  function renderSelectionHoverFastPath(point) {
    const state = selectionHoverSuppressionState(point);
    if (!state) {
      return false;
    }
    enterSelectionHoverSuppression(point, state);
    return true;
  }

  async function drainScheduledHoverPointerMove() {
    hoverMoveFrame = 0;
    if (hoverMoveRunning) {
      return;
    }
    hoverMoveRunning = true;
    try {
      while (hoverMoveRequest) {
        const request = hoverMoveRequest;
        const version = hoverMoveVersion;
        hoverMoveRequest = null;
        await processHoverPointerMove(request.point, request.altKey, version);
      }
    } finally {
      hoverMoveRunning = false;
      if (hoverMoveRequest && !hoverMoveFrame) {
        hoverMoveFrame = requestAnimationFrame(drainScheduledHoverPointerMove);
      }
    }
  }

  async function processHoverPointerMove(point, altKey, version) {
    if (hoverBlockedAtPoint(point)) {
      clearEditorOverlayRoot();
      return;
    }
    if (!options.routeEditorPointerEvents()) {
      if (options.isEditingRustDocument()) {
        await options.state().editorEngine.clearInteraction();
        if (hoverMoveStale(version)) {
          return;
        }
        options.renderEditorOverlay();
      }
      return;
    }
    if (renderSelectionHoverFastPath(point)) {
      return;
    }
    if (
      options.editorState().activeTool === "select"
      && !options.documentBoundsContainsPoint?.(point, DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX)
    ) {
      await options.state().editorEngine.clearInteraction?.();
      invalidateEngineReadCache();
      if (hoverMoveStale(version)) {
        return;
      }
      options.renderEditorOverlay(options.currentEditorOverlayRenderList());
      options.syncCanvasCursor?.();
      return;
    }
    await options.state().editorEngine.pointerMove(point.x, point.y, altKey);
    invalidateEngineReadCache();
    if (hoverMoveStale(version)) {
      return;
    }
    const editorState = options.editorState();
    const shouldUpdateTlcHover = editorState.activeTool === "tlc-plate"
      || (editorState.activeTool === "select" && options.documentHasTlcPlate?.());
    if (!editorState.elementPlacementActive && shouldUpdateTlcHover) {
      await options.updateTlcSpotHover(point);
    } else if (options.activeSelectionGesture()?.kind !== "tlc-spot-drag") {
      options.clearTlcHoverState();
    }
    if (hoverMoveStale(version)) {
      return;
    }
    options.renderEditorOverlay(currentInteractionRenderList());
    options.positionActiveTextEditor();
    if (editorState.elementPlacementActive) {
      options.syncCanvasCursor();
    } else if (editorState.activeTool === "select") {
      await options.syncSelectCursorForPoint(point);
    } else if (toolSupportsSelectionBoxMove(editorState.activeTool)) {
      await options.syncArrowAwareCursorForPoint(point);
    }
  }

  async function beginSelectionBoxMove(point, event) {
    if (!toolSupportsSelectionBoxMove(options.editorState().activeTool)) {
      return false;
    }
    const overSelection = !!options.state().editorEngine.selectionContainsPoint?.(point.x, point.y);
    if (!overSelection) {
      return false;
    }
    return beginSelectionMoveGesture(point, event, options.syncArrowAwareCursorForPoint);
  }

  async function beginSelectionMoveGesture(point, event, syncCursor = options.syncSelectCursorForPoint) {
    const startedFromExistingSelection = !!options.state().editorEngine.selectionContainsPoint?.(point.x, point.y);
    const beginResult = options.state().editorEngine.beginSelectionMove?.(
      point.x,
      point.y,
      !!event.shiftKey,
      event.altKey,
    );
    const began = beginResult && typeof beginResult.then === "function" ? await beginResult : beginResult;
    if (!began) {
      return false;
    }
    invalidateEngineReadCache();
    const previewSelection = options.parseEngineJson(
      options.state().editorEngine.stateJson?.() || "",
      null,
    )?.selection || options.currentEditorEngineState?.()?.selection || null;
    options.setActiveSelectionGesture({
      kind: "move",
      start: point,
      current: point,
      dragged: false,
      additive: !!event.shiftKey,
      previewSelection,
      cursor: startedFromExistingSelection ? "grabbing" : "default",
    });
    await syncCursor(point);
    clearEditorOverlayRoot();
    return true;
  }

  function selectionHandleZoneContainsPoint(point) {
    return selectionHandleZoneContainsPointModel({
      point,
      bounds: options.currentRenderBounds?.("selection"),
      pointDistance: options.pointDistance,
      toWorld: (screenPx) => options.screenPxToWorld?.(screenPx) ?? options.cssPxToPt(screenPx),
      selectedContentHitContainsPoint: options.selectedContentHitContainsPoint,
    });
  }

  function primaryButtonIsDown(event) {
    return (event.buttons & 1) === 1;
  }

  function invalidateEngineReadCache() {
    options.invalidateEditorEngineReadCache?.();
  }

  function currentInteractionRenderList() {
    return options.currentEditorInteractionRenderList?.() || [];
  }

  function editorCommandAnchor(point) {
    return { x: point.x, y: point.y };
  }

  function creationCommandForDrag(tool, start, end) {
    const editorState = options.editorState();
    if (tool === "arrow") {
      return {
        type: "add-arrow",
        begin: editorCommandAnchor(start),
        end: editorCommandAnchor(end),
        variant: editorState.arrowType,
        headSize: editorState.arrowHeadSize,
        curve: editorState.arrowCurve,
        headStyle: editorState.arrowHeadStyle,
        tailStyle: editorState.arrowTailStyle,
        head: editorState.arrowHeadStyle !== "none",
        tail: editorState.arrowTailStyle !== "none",
        bold: !!editorState.arrowBold,
        noGo: editorState.arrowNoGo || "none",
      };
    }
    if (tool === "shape" || tool === "tlc-plate") {
      return {
        type: "add-shape",
        kind: tool === "tlc-plate" ? "tlc-plate" : editorState.shapeKind,
        style: editorState.shapeStyle,
        color: editorState.shapeColor,
        begin: editorCommandAnchor(start),
        end: editorCommandAnchor(end),
      };
    }
    if (tool === "bracket") {
      return {
        type: "add-bracket",
        kind: editorState.bracketKind,
        begin: editorCommandAnchor(start),
        end: editorCommandAnchor(end),
      };
    }
    if (tool === "symbol") {
      return {
        type: "add-symbol",
        kind: editorState.symbolKind,
        center: editorCommandAnchor(end),
      };
    }
    return null;
  }

  async function executeCreationCommand(command) {
    if (!command) {
      return null;
    }
    return executeDocumentCommand(
      command,
      () => options.state().editorEngine.executeCommandJson?.(JSON.stringify(command)),
      { sync: false, deferDocumentSync: true },
    );
  }

  async function refreshHoverOverlayAtPoint(point, event) {
    await options.state().editorEngine.pointerMove(point.x, point.y, event?.altKey || false);
    invalidateEngineReadCache();
    options.renderEditorOverlay(currentInteractionRenderList());
    await options.syncArrowAwareCursorForPoint(point);
  }

  async function updateEngineDragPreview(point, event, version = engineDragPreviewVersion) {
    cancelScheduledHoverMove();
    leaveSelectionHoverSuppression(point);
    await options.state().editorEngine.pointerMove(point.x, point.y, event.altKey);
    invalidateEngineReadCache();
    if (version !== engineDragPreviewVersion) {
      return;
    }
    let renderList = currentInteractionRenderList();
    if (!renderList.length && engineCreationDrag?.start) {
      await options.state().editorEngine.clearInteraction?.();
      await options.state().editorEngine.pointerDown(
        engineCreationDrag.start.x,
        engineCreationDrag.start.y,
        event.altKey,
      );
      await options.state().editorEngine.pointerMove(point.x, point.y, event.altKey);
      invalidateEngineReadCache();
      if (version !== engineDragPreviewVersion) {
        return;
      }
      renderList = currentInteractionRenderList();
    }
    if (engineCreationDrag?.start && options.renderDragCapturePreview) {
      options.renderDragCapturePreview(renderList);
    } else {
      options.renderEditorOverlay(renderList);
    }
    options.positionActiveTextEditor();
  }

  async function drainEngineDragPreviewFrame() {
    engineDragPreviewFrame = 0;
    if (engineDragPreviewRunning) {
      return;
    }
    engineDragPreviewRunning = true;
    try {
      const request = engineDragPreviewRequest;
      engineDragPreviewRequest = null;
      if (request) {
        await updateEngineDragPreview(request.point, request, request.version);
      }
    } finally {
      engineDragPreviewRunning = false;
      if (engineDragPreviewRequest && !engineDragPreviewFrame) {
        engineDragPreviewFrame = requestAnimationFrame(drainEngineDragPreviewFrame);
      }
    }
  }

  function scheduleEngineDragPreview(point, event) {
    event.preventDefault();
    engineDragPreviewRequest = { point, altKey: event.altKey, version: engineDragPreviewVersion };
    if (!engineDragPreviewFrame && !engineDragPreviewRunning) {
      engineDragPreviewFrame = requestAnimationFrame(drainEngineDragPreviewFrame);
    }
  }

  function readLastCommandResult() {
    return options.parseEngineJson(
      options.state().editorEngine.lastCommandResultJson?.(),
      null,
    );
  }

  function engineRevision() {
    return Number(options.state().editorEngine.revision?.() || 0);
  }

  async function handleEditorPointerMove(event) {
    options.noteEditorPointerActivity?.();
    const point = options.svgPointFromEvent(event);
    const editorState = options.editorState();
    const gesture = options.activeSelectionGesture();
    if ((editorState.activeTool === "select" || toolSupportsSelectionBoxMove(editorState.activeTool)) && gesture) {
      event.preventDefault();
      if (gesture.kind === "tlc-spot-drag") {
        gesture.current = point;
        gesture.dragged = options.pointDistance(gesture.start, point) >= options.cssPxToPt(TLC_SPOT_DRAG_THRESHOLD_SCREEN_PX);
        const hit = options.parseEngineJson(
          await options.state().editorEngine.updateTlcSpotDragJson?.(point.x, point.y),
          null,
        );
        if (hit) {
          gesture.hit = hit;
          await options.syncDocumentFromEngine({ syncRenderList: false, refreshSnapshot: false });
        }
        await options.syncSelectCursorForPoint(point);
        if (hit?.objectId) {
          options.renderDocumentChange?.({
            changed: true,
            targets: { objects: [hit.objectId] },
          }) || options.renderDocument();
        } else {
          options.renderEditorOverlay(options.currentEditorOverlayRenderList());
        }
        return;
      }
      if (gesture.kind === "arrow-endpoint" || gesture.kind === "arrow-curve") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(SELECTION_DRAG_THRESHOLD_SCREEN_PX)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        await options.state().editorEngine.updateHoverArrowEdit?.(point.x, point.y, event.altKey);
        invalidateEngineReadCache();
        if (gesture.kind === "arrow-curve") {
          gesture.angle = options.state().editorEngine.activeArrowEditDegrees?.() || 0;
        }
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      if (gesture.kind === "shape-resize") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(SELECTION_DRAG_THRESHOLD_SCREEN_PX)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        await options.state().editorEngine.updateHoverShapeEdit?.(point.x, point.y, event.altKey);
        invalidateEngineReadCache();
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      if (gesture.kind === "rotate") {
        gesture.current = point;
        gesture.angle = options.selectionRotateAngleForGesture(gesture, point, event.altKey);
        if (options.applyDocumentObjectPreviewTransform()) {
          await options.syncSelectCursorForPoint(point);
          clearEditorOverlayRoot();
          return;
        }
        await options.state().editorEngine.updateSelectionRotate(point.x, point.y, event.altKey);
        invalidateEngineReadCache();
        await options.syncSelectCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      if (gesture.kind === "resize") {
        gesture.current = point;
        gesture.scale = options.selectionResizeGestureScale(gesture, point);
        if (options.applyDocumentObjectPreviewTransform()) {
          await options.syncSelectCursorForPoint(point);
          clearEditorOverlayRoot();
          return;
        }
        await options.state().editorEngine.updateSelectionResize?.(point.x, point.y);
        invalidateEngineReadCache();
        await options.syncSelectCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      if (gesture.kind === "move") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(SELECTION_DRAG_THRESHOLD_SCREEN_PX)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        gesture.altKey = event.altKey;
        const previewSelection = gesture.previewSelection || options.currentEditorEngineState?.()?.selection;
        if (options.selectionNeedsBackendMovePreview?.(previewSelection)) {
          gesture.previewSelection = previewSelection;
          gesture.backendDocumentPreviewActive = true;
          gesture.previewDirty = true;
          options.hideDocumentDiagnosticsForPreview?.();
          scheduleDocumentPreviewFrame();
          clearEditorOverlayRoot();
          return;
        }
        if (gesture.localDocumentPreviewActive) {
          scheduleDocumentPreviewFrame();
          return;
        }
        if (options.applyDocumentObjectPreviewTransform()) {
          gesture.localDocumentPreviewActive = true;
          clearEditorOverlayRoot();
          return;
        }
        await options.state().editorEngine.updateSelectionMove(point.x, point.y, event.altKey);
        invalidateEngineReadCache();
        await options.syncSelectCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(SELECTION_DRAG_THRESHOLD_SCREEN_PX)) {
        gesture.dragged = true;
      }
      gesture.current = point;
      if (editorState.selectMode === "free") {
        const lastPoint = gesture.points[gesture.points.length - 1];
        if (!lastPoint || options.pointDistance(lastPoint, point) >= options.cssPxToPt(SELECTION_FREEHAND_POINT_SPACING_SCREEN_PX)) {
          gesture.points.push(point);
        }
      }
      options.renderEditorOverlay(options.currentEditorOverlayRenderList());
      return;
    }
    if (
      primaryButtonIsDown(event)
      && toolUsesEngineDragPreview(editorState.activeTool)
      && options.routeEditorPointerEvents()
    ) {
      event.preventDefault();
      if (engineCreationDrag?.start) {
        await updateEngineDragPreview(point, event);
      } else {
        scheduleEngineDragPreview(point, event);
      }
      return;
    }
    if (
      primaryButtonIsDown(event)
      && editorState.activeTool === "select"
      && options.routeEditorPointerEvents()
      && await beginSelectionMoveGesture(point, event, options.syncSelectCursorForPoint)
    ) {
      event.preventDefault();
      return;
    }
    const selectionHoverSuppression = selectionHoverSuppressionState(point);
    if (selectionHoverSuppression) {
      event.preventDefault();
      enterSelectionHoverSuppression(point, selectionHoverSuppression);
      return;
    }
    leaveSelectionHoverSuppression(point);
    if (
      editorState.activeTool === "select"
      && !options.documentBoundsContainsPoint?.(point, DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX)
    ) {
      cancelScheduledHoverMove();
      await options.state().editorEngine.clearInteraction?.();
      invalidateEngineReadCache();
      options.renderEditorOverlay(options.currentEditorOverlayRenderList());
      options.syncCanvasCursor?.();
      return;
    }
    scheduleHoverPointerMove(
      point,
      event.altKey,
      options.hoverPointerMoveDelayMs?.(editorState.activeTool) || 0,
    );
  }

  async function handleEditorPointerDown(event) {
    options.noteEditorPointerActivity?.();
    await options.awaitPendingToolActivation?.();
    if (!options.routeEditorPointerEvents() || event.button !== 0) {
      return;
    }
    cancelScheduledHoverMove();
    cancelDocumentPreviewFrame();
    cancelEngineDragPreviewFrame();
    engineCreationDrag = null;
    postCommitHoverBlockPoint = null;
    const point = options.svgPointFromEvent(event);
    options.setLastEditFocusPoint(point);
    const editorState = options.editorState();
    if (editorState.activeTool !== "text" && !editorState.elementPlacementActive) {
      await options.closeActiveTextEditorForToolAction?.();
    }
    if (editorState.elementPlacementActive) {
      event.preventDefault();
      options.viewerSvg().setPointerCapture?.(event.pointerId);
      await options.state().editorEngine.pointerDown(point.x, point.y, event.altKey);
      await options.syncDocumentFromEngine({ syncRenderList: false, refreshSnapshot: false });
      options.renderEditorOverlay();
      return;
    }
    if (editorState.activeTool === "select") {
      event.preventDefault();
      options.viewerSvg().setPointerCapture?.(event.pointerId);
      if (!options.documentBoundsContainsPoint?.(point, DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX)) {
        options.setActiveSelectionGesture({
          kind: "select",
          start: point,
          current: point,
          points: [point],
          dragged: false,
          additive: !!event.shiftKey,
        });
        options.renderEditorOverlay(options.currentEditorOverlayRenderList());
        return;
      }
      await options.state().editorEngine.pointerMove(point.x, point.y, event.altKey);
      const tlcSpotHit = options.parseEngineJson(
        await options.state().editorEngine.beginTlcSpotDragJson?.(point.x, point.y),
        null,
      );
      if (tlcSpotHit) {
        options.setActiveSelectionGesture({
          kind: "tlc-spot-drag",
          start: point,
          current: point,
          dragged: false,
          cursor: "grabbing",
          hit: tlcSpotHit,
        });
        options.setActiveTlcSpotHover(tlcSpotHit);
        options.setActiveTlcLaneHover(null);
        await options.selectClickTarget(point, !!event.shiftKey);
        await options.renderSelectionOnlyUpdate(point);
        return;
      }
      const hitAtPoint = options.parseEngineJson(
        await options.state().editorEngine.contextHitTestJson?.(point.x, point.y),
        null,
      );
      if (hitAtPoint?.objectType === "bracket") {
        const bracketEditAction = await options.state().editorEngine.beginHoverShapeEdit?.(point.x, point.y) || "";
        if (bracketEditAction) {
          options.setActiveSelectionGesture({
            kind: "shape-resize",
            action: bracketEditAction,
            cursor: options.cursorForShapeAction(bracketEditAction) || "nwse-resize",
            start: point,
            current: point,
            dragged: false,
            additive: !!event.shiftKey,
          });
          await options.syncArrowAwareCursorForPoint(point);
          options.renderEditorOverlay(currentInteractionRenderList());
          return;
        }
      }
      const overSelectionInterior = (
        !!options.selectionBoundsContainsPoint?.(point)
        || !!options.selectionHitContainsPoint?.(point)
      ) && !selectionHandleZoneContainsPoint(point);
      if (!event.shiftKey && overSelectionInterior && await beginSelectionMoveGesture(point, event, options.syncSelectCursorForPoint)) {
        return;
      }
      const resizeHandle = options.selectionResizeHandleHit(point);
      if (resizeHandle && await options.state().editorEngine.beginSelectionResize?.(resizeHandle.name, point.x, point.y)) {
        options.setActiveSelectionGesture({
          kind: "resize",
          handle: resizeHandle.name,
          cursor: resizeHandle.cursor,
          bounds: options.currentRenderBounds("selection"),
          start: point,
          current: point,
          scale: 1,
        });
        await options.syncSelectCursorForPoint(point);
        clearEditorOverlayRoot();
        return;
      }
      const shapeEditAction = await options.state().editorEngine.beginHoverShapeEdit?.(point.x, point.y) || "";
      if (shapeEditAction) {
        options.setActiveSelectionGesture({
          kind: "shape-resize",
          action: shapeEditAction,
          cursor: options.cursorForShapeAction(shapeEditAction) || "nwse-resize",
          start: point,
          current: point,
          dragged: false,
          additive: !!event.shiftKey,
        });
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      const arrowEditAction = await options.state().editorEngine.beginHoverArrowEdit?.(point.x, point.y) || "";
      if (arrowEditAction) {
        options.setActiveSelectionGesture({
          kind: arrowEditAction === "curve" ? "arrow-curve" : "arrow-endpoint",
          action: arrowEditAction,
          start: point,
          current: point,
          dragged: false,
          additive: !!event.shiftKey,
          angle: 0,
        });
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
      const rotateHandle = options.selectionRotateHandleHit(point);
      if (rotateHandle) {
        if (await options.state().editorEngine.beginSelectionRotate?.(point.x, point.y)) {
          options.setActiveSelectionGesture({
            kind: "rotate",
            center: {
              x: (rotateHandle.bounds.minX + rotateHandle.bounds.maxX) * 0.5,
              y: (rotateHandle.bounds.minY + rotateHandle.bounds.maxY) * 0.5,
            },
            bounds: rotateHandle.bounds,
            start: point,
            current: point,
            startAngle: options.angleBetweenPoints(
              {
                x: (rotateHandle.bounds.minX + rotateHandle.bounds.maxX) * 0.5,
                y: (rotateHandle.bounds.minY + rotateHandle.bounds.maxY) * 0.5,
              },
              point,
            ),
            angle: 0,
          });
          await options.syncSelectCursorForPoint(point);
          clearEditorOverlayRoot();
          return;
        }
      }
      if (!event.shiftKey && await beginSelectionMoveGesture(point, event, options.syncSelectCursorForPoint)) {
        return;
      }
      options.setActiveSelectionGesture({
        kind: "select",
        start: point,
        current: point,
        points: [point],
        dragged: false,
        additive: !!event.shiftKey,
      });
      options.renderEditorOverlay(options.currentEditorOverlayRenderList());
      return;
    }
    if (editorState.activeTool === "text") {
      event.preventDefault();
      if (await beginSelectionBoxMove(point, event)) {
        options.viewerSvg().setPointerCapture?.(event.pointerId);
        return;
      }
      await options.openTextEditorAt(point);
      return;
    }
    event.preventDefault();
    if (toolUsesEngineDragPreview(editorState.activeTool)) {
      options.setCanvasPointerShieldActive?.(true);
    } else {
      options.viewerSvg().setPointerCapture?.(event.pointerId);
    }
    if (editorState.activeTool === "arrow") {
      const arrowEditAction = await options.state().editorEngine.beginHoverArrowEdit?.(point.x, point.y) || "";
      if (arrowEditAction) {
        options.setActiveSelectionGesture({
          kind: arrowEditAction === "curve" ? "arrow-curve" : "arrow-endpoint",
          action: arrowEditAction,
          start: point,
          current: point,
          dragged: false,
          angle: 0,
        });
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
    }
    if (editorState.activeTool === "shape"
      || editorState.activeTool === "bracket"
      || editorState.activeTool === "tlc-plate"
      || editorState.activeTool === "orbital") {
      if (editorState.activeTool === "tlc-plate") {
        const tlcSpotHit = options.parseEngineJson(
          await options.state().editorEngine.beginTlcSpotDragJson?.(point.x, point.y),
          null,
        );
        if (tlcSpotHit) {
          options.setActiveSelectionGesture({
            kind: "tlc-spot-drag",
            start: point,
            current: point,
            dragged: false,
            cursor: "grabbing",
            hit: tlcSpotHit,
          });
          options.setActiveTlcSpotHover(tlcSpotHit);
          options.setActiveTlcLaneHover(null);
          await options.syncArrowAwareCursorForPoint(point);
          options.renderEditorOverlay(currentInteractionRenderList());
          return;
        }
      }
      const shapeEditAction = await options.state().editorEngine.beginHoverShapeEdit?.(point.x, point.y) || "";
      if (shapeEditAction) {
        options.setActiveSelectionGesture({
          kind: "shape-resize",
          action: shapeEditAction,
          cursor: options.cursorForShapeAction(shapeEditAction) || "nwse-resize",
          start: point,
          current: point,
          dragged: false,
        });
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(currentInteractionRenderList());
        return;
      }
    }
    if (editorState.activeTool === "bracket") {
      options.setActiveBracketDragStart(point);
    }
    const beforeRevision = engineRevision();
    await options.state().editorEngine.pointerDown(point.x, point.y, event.altKey);
    const pointerDownResult = readLastCommandResult();
    if (toolUsesEngineDragPreview(editorState.activeTool)) {
      engineCreationDrag = {
        tool: editorState.activeTool,
        start: point,
      };
    }
    if (
      pointerDownResult?.changed
      && Number(pointerDownResult.beforeRevision ?? pointerDownResult.before_revision ?? -1) === beforeRevision
    ) {
      await options.syncDocumentFromEngine({ syncRenderList: false, refreshSnapshot: false });
      options.renderDocumentChange?.(pointerDownResult) || options.renderDocument();
      return;
    }
    invalidateEngineReadCache();
    options.renderEditorOverlay(currentInteractionRenderList());
  }

  async function handleEditorPointerUp(event) {
    options.noteEditorPointerActivity?.();
    if (options.editorState().activeTool === "text" && !options.activeSelectionGesture()) {
      return;
    }
    if (!options.routeEditorPointerEvents()) {
      return;
    }
    const point = options.svgPointFromEvent(event);
    options.setLastEditFocusPoint(point);
    event.preventDefault();
    cancelScheduledHoverMove();
    cancelDocumentPreviewFrame();
    cancelEngineDragPreviewFrame();
    if (engineDragPreviewRunning) {
      await drainEngineDragPreviewFrame();
    }
    options.viewerSvg().releasePointerCapture?.(event.pointerId);
    const keepCreationPreviewUntilCommit = !!engineCreationDrag?.start;
    const clearedDragCaptureOnPointerUp = !!options.setCanvasPointerShieldActive?.(
      false,
      { clearPreview: !keepCreationPreviewUntilCommit },
    );
    const gesture = options.activeSelectionGesture();
    if (gesture?.kind === "tlc-spot-drag") {
      const result = await executeDocumentCommand(
        {
          type: "set-tlc-spot-position",
          payload: { x: point.x, y: point.y },
        },
        () => options.state().editorEngine.finishTlcSpotDragJson?.(point.x, point.y),
      );
      const hit = options.parseEngineJson(result.rawResult, null);
      options.setActiveSelectionGesture(null);
      if (hit) {
        options.setActiveTlcSpotHover(hit);
        options.setActiveTlcLaneHover(null);
      } else {
        options.clearTlcHoverState();
      }
      if (options.editorState().activeTool === "select") {
        await options.syncSelectCursorForPoint(point);
      } else {
        await options.syncArrowAwareCursorForPoint(point);
      }
      options.renderDocumentChange?.(result) || options.renderDocument();
      return;
    }
    if ((options.editorState().activeTool === "select" || options.editorState().activeTool === "arrow")
      && (gesture?.kind === "arrow-endpoint" || gesture?.kind === "arrow-curve")) {
      options.setActiveSelectionGesture(null);
      const commitStarted = performance.now();
      const result = await executeDocumentCommand(
        {
          type: "set-arrow-geometry",
          payload: {
            action: gesture.kind === "arrow-curve" ? "curve" : "endpoint",
            x: point.x,
            y: point.y,
            altKey: event.altKey,
          },
        },
        async () => {
          const changed = !!(await options.state().editorEngine.finishHoverArrowEdit?.(point.x, point.y, event.altKey));
          return changed;
        },
        { syncRenderList: false },
      );
      const executedAt = performance.now();
      const changed = !!result.changed;
      if (!changed && !gesture.dragged && options.editorState().activeTool === "select") {
        await options.selectClickTarget(point, gesture.additive);
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
        recordInteractionCommitTiming({
          kind: gesture.kind,
          commandType: result.commandType || "set-arrow-geometry",
          changed,
          executeMs: executedAt - commitStarted,
          totalMs: performance.now() - commitStarted,
          targets: result.targets || null,
        });
        return;
      }
      if (changed) {
        options.renderDocumentChange?.(result) || options.renderDocument();
        await refreshHoverOverlayAtPoint(point, event);
      } else {
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
      }
      recordInteractionCommitTiming({
        kind: gesture.kind,
        commandType: result.commandType || "set-arrow-geometry",
        changed,
        executeMs: executedAt - commitStarted,
        totalMs: performance.now() - commitStarted,
        targets: result.targets || null,
      });
      return;
    }
    if ((options.editorState().activeTool === "select"
      || options.editorState().activeTool === "bracket"
      || options.editorState().activeTool === "shape"
      || options.editorState().activeTool === "tlc-plate"
      || options.editorState().activeTool === "orbital")
      && gesture?.kind === "shape-resize") {
      options.setActiveSelectionGesture(null);
      const commitStarted = performance.now();
      const result = await executeDocumentCommand(
        {
          type: "set-shape-geometry",
          payload: {
            action: gesture.action || "resize",
            x: point.x,
            y: point.y,
            altKey: event.altKey,
          },
        },
        async () => {
          const changed = !!(await options.state().editorEngine.finishHoverShapeEdit?.(point.x, point.y, event.altKey));
          return changed;
        },
        { sync: false, deferDocumentSync: true },
      );
      const executedAt = performance.now();
      const changed = !!result.changed;
      if (!changed && !gesture.dragged && options.editorState().activeTool === "select") {
        await options.selectClickTarget(point, gesture.additive);
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
        recordInteractionCommitTiming({
          kind: gesture.kind,
          commandType: result.commandType || "set-shape-geometry",
          changed,
          executeMs: executedAt - commitStarted,
          totalMs: performance.now() - commitStarted,
          targets: result.targets || null,
        });
        return;
      }
      if (changed) {
        options.renderDocumentChange?.(result) || options.renderDocument();
        await refreshHoverOverlayAtPoint(point, event);
      } else {
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
      }
      recordInteractionCommitTiming({
        kind: gesture.kind,
        commandType: result.commandType || "set-shape-geometry",
        changed,
        executeMs: executedAt - commitStarted,
        totalMs: performance.now() - commitStarted,
        targets: result.targets || null,
      });
      return;
    }
    if (gesture?.kind === "move") {
      options.setActiveSelectionGesture(null);
      if (gesture.dragged) {
        const commitPoint = gesture.current || point;
        const commitPreviewDom = !!gesture.localDocumentPreviewActive
          && !!options.canCommitDocumentObjectPreviewTransform?.()
          && typeof options.commitDocumentObjectPreviewTransform === "function";
        const commitBackendPreview = !!gesture.backendDocumentPreviewActive
          && typeof options.renderDocumentPrimitiveChange === "function";
        const commitStarted = performance.now();
        const result = await executeDocumentCommand(
          {
            type: "move-selection",
            payload: {
              start: gesture.start,
              end: commitPoint,
              altKey: event.altKey,
            },
          },
          () => options.state().editorEngine.finishSelectionMove(commitPoint.x, commitPoint.y, event.altKey),
          (commitPreviewDom || commitBackendPreview) ? { sync: false, deferDocumentSync: true } : {},
        );
        const executedAt = performance.now();
        suppressHoverUntilPointerLeavesPoint(commitPoint);
        if (commitBackendPreview && result.changed) {
          options.renderDocumentPrimitiveChange(result);
          options.clearDocumentObjectPreviewTransform();
        } else if (commitPreviewDom && result.changed) {
          options.commitDocumentObjectPreviewTransform();
          options.clearDocumentObjectPreviewTransform();
        } else {
          options.clearDocumentObjectPreviewTransform();
        }
        if ((commitPreviewDom || commitBackendPreview) && result.changed) {
          clearEditorOverlayRoot();
        } else {
          await clearEngineHoverOverlay();
        }
        options.syncCanvasCursor?.();
        if ((commitPreviewDom || commitBackendPreview) && result.changed) {
          await syncDeferredDocumentModelAfterCommit();
          await options.renderSelectionOnlyUpdate(commitPoint, null, {
            deferEngineReads: true,
            useInteractionList: false,
          });
        } else {
          options.renderDocumentChange?.(result) || options.renderDocument();
        }
        clearEditorOverlayRoot();
        recordInteractionCommitTiming({
          kind: gesture.kind,
          commandType: result.commandType || "move-selection",
          changed: !!result.changed,
          previewDom: commitPreviewDom,
          backendPreview: commitBackendPreview,
          executeMs: executedAt - commitStarted,
          totalMs: performance.now() - commitStarted,
          targets: result.targets || null,
        });
      } else if (options.editorState().activeTool === "select") {
        await options.selectClickTarget(gesture.start || point, gesture.additive);
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(gesture.start || point);
      } else {
        options.clearDocumentObjectPreviewTransform();
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(options.currentEditorOverlayRenderList());
      }
      return;
    }
    if (options.editorState().activeTool === "select") {
      options.setActiveSelectionGesture(null);
      if (!gesture) {
        return;
      }
      if (gesture.kind === "rotate") {
        const commitStarted = performance.now();
        const result = await executeDocumentCommand(
          {
            type: "rotate-selection",
            payload: {
              x: point.x,
              y: point.y,
              altKey: event.altKey,
            },
          },
          () => options.state().editorEngine.finishSelectionRotate(point.x, point.y, event.altKey),
          { sync: false, deferDocumentSync: true },
        );
        const executedAt = performance.now();
        await options.syncSelectCursorForPoint(point);
        options.clearDocumentObjectPreviewTransform();
        options.renderDocumentChange?.(result) || options.renderDocument();
        recordInteractionCommitTiming({
          kind: gesture.kind,
          commandType: result.commandType || "rotate-selection",
          changed: !!result.changed,
          executeMs: executedAt - commitStarted,
          totalMs: performance.now() - commitStarted,
          targets: result.targets || null,
        });
        return;
      }
      if (gesture.kind === "resize") {
        const commitStarted = performance.now();
        const result = await executeDocumentCommand(
          {
            type: "resize-selection",
            payload: {
              handle: gesture.handle || null,
              x: point.x,
              y: point.y,
            },
          },
          () => options.state().editorEngine.finishSelectionResize?.(point.x, point.y),
          { sync: false, deferDocumentSync: true },
        );
        const executedAt = performance.now();
        await options.syncSelectCursorForPoint(point);
        options.clearDocumentObjectPreviewTransform();
        options.renderDocumentChange?.(result) || options.renderDocument();
        recordInteractionCommitTiming({
          kind: gesture.kind,
          commandType: result.commandType || "resize-selection",
          changed: !!result.changed,
          executeMs: executedAt - commitStarted,
          totalMs: performance.now() - commitStarted,
          targets: result.targets || null,
        });
        return;
      }
      if (!gesture.dragged) {
        await options.selectClickTarget(point, gesture.additive);
      } else if (options.editorState().selectMode === "box") {
        await options.state().editorEngine.selectInRect(
          gesture.start.x,
          gesture.start.y,
          point.x,
          point.y,
          gesture.additive,
        );
      } else {
        const polygonPoints = [...gesture.points, point].map((candidate) => [candidate.x, candidate.y]);
        await options.state().editorEngine.selectInPolygon(JSON.stringify(polygonPoints), gesture.additive);
      }
      await options.renderSelectionOnlyUpdate(point);
      return;
    }
    const clearedInteractionBeforeCommit = keepCreationPreviewUntilCommit
      ? false
      : (clearInteractionOverlayBeforeCommit() || clearedDragCaptureOnPointerUp);
    if (clearedInteractionBeforeCommit) {
      await waitForClearedInteractionPaint();
    }
    const commitStarted = performance.now();
    const result = await executeDocumentCommand(
      {
        type: pointerCommitCommandType(),
        payload: {
          x: point.x,
          y: point.y,
          altKey: event.altKey,
        },
      },
      () => options.state().editorEngine.pointerUp(point.x, point.y, event.altKey),
      { sync: false, deferDocumentSync: true },
    );
    const executedAt = performance.now();
    let commitResult = result;
    if (!commitResult?.changed && engineCreationDrag?.start) {
      commitResult = await executeCreationCommand(
        creationCommandForDrag(engineCreationDrag.tool, engineCreationDrag.start, point),
      ) || commitResult;
    }
    const fallbackAt = performance.now();
    const pendingGraphicObjectId = await Promise.resolve(
      options.state().editorEngine.pendingGraphicObjectId?.() || "",
    ) || commitResult?.targets?.objects?.[0] || commitResult?.created?.objects?.[0] || "";
    options.renderDocumentChange?.(commitResult) || options.renderDocument();
    if (keepCreationPreviewUntilCommit) {
      options.clearDragCapturePreview?.();
    }
    const renderedAt = performance.now();
    await options.ensureDocumentObjectDomForCommandResult?.(commitResult);
    const ensuredAt = performance.now();
    invalidateEngineReadCache();
    await clearPostCommitInteraction(point);
    const clearedAt = performance.now();
    recordCreationCommitTiming({
      tool: options.editorState().activeTool,
      changed: !!commitResult?.changed,
      commandType: commitResult?.commandType || pointerCommitCommandType(),
      executeMs: executedAt - commitStarted,
      fallbackMs: fallbackAt - executedAt,
      renderMs: renderedAt - fallbackAt,
      ensureDomMs: ensuredAt - renderedAt,
      clearMs: clearedAt - ensuredAt,
      totalMs: clearedAt - commitStarted,
      commitStartedAt: commitStarted,
      executedAt,
      fallbackAt,
      renderedAt,
      ensuredAt,
      clearedAt,
      targets: commitResult?.targets || null,
      created: commitResult?.created || null,
    });
    if (options.editorState().activeTool === "bracket") {
      const start = options.activeBracketDragStart();
      options.setActiveBracketDragStart(null);
      if (commitResult?.changed && start && options.pointDistance(start, point) >= options.cssPxToPt(BRACKET_LABEL_OPEN_DRAG_THRESHOLD_SCREEN_PX)) {
        await options.openTextEditorAt(
          options.bracketLabelAnchorPoint(start, point),
          {
            ...(options.bracketLabelTextOptions?.() || {}),
            bracketObjectId: pendingGraphicObjectId,
          },
        );
      }
    }
    engineCreationDrag = null;
  }

  async function handleEditorPointerLeave() {
    cancelScheduledHoverMove();
    cancelDocumentPreviewFrame();
    postCommitHoverBlockPoint = null;
    leaveSelectionHoverSuppression();
    if (!options.isEditingRustDocument()) {
      return;
    }
    if ((options.editorState().activeTool === "select"
      || toolSupportsSelectionBoxMove(options.editorState().activeTool))
      && options.activeSelectionGesture()) {
      return;
    }
    options.clearTlcHoverState();
    if (options.editorState().activeTool !== "text") {
      await clearEngineHoverOverlay({ keepSelectionOverlay: true });
    }
  }

  async function handleEditorDoubleClick(event) {
    if (!options.routeEditorPointerEvents() || options.editorState().activeTool !== "select") {
      return;
    }
    const point = options.svgPointFromEvent(event);
    const changed = !!(await options.state().editorEngine.selectComponentAtPoint?.(point.x, point.y, event.shiftKey));
    if (!changed) {
      return;
    }
    event.preventDefault();
    options.setActiveSelectionGesture(null);
    await options.renderSelectionOnlyUpdate(point);
  }

  async function handleEditorPointerCancel() {
    cancelScheduledHoverMove();
    cancelDocumentPreviewFrame();
    postCommitHoverBlockPoint = null;
    options.setActiveSelectionGesture(null);
    engineCreationDrag = null;
    options.clearDocumentObjectPreviewTransform();
    options.clearDragCapturePreview?.();
    options.setCanvasPointerShieldActive?.(false);
    await clearEngineHoverOverlay();
    options.syncCanvasCursor();
  }

  return {
    handleEditorPointerMove,
    handleEditorPointerDown,
    handleEditorPointerUp,
    handleEditorPointerLeave,
    handleEditorDoubleClick,
    handleEditorPointerCancel,
  };
}
