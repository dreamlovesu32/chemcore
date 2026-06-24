export function createEditorPointerController(options) {
  let hoverMoveRequest = null;
  let hoverMoveFrame = 0;
  let hoverMoveRunning = false;
  let hoverMoveVersion = 0;
  let selectionHoverSuppressionActive = false;
  let documentPreviewFrame = 0;
  let postCommitHoverBlockPoint = null;

  async function executeDocumentCommand(command, apply, executeOptions = {}) {
    if (options.commandEngine?.executeEngineCommand) {
      return options.commandEngine.executeEngineCommand(command, apply, executeOptions);
    }
    const rawResult = await apply();
    if (rawResult) {
      await options.syncDocumentFromEngine();
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

  function scheduleHoverPointerMove(point, altKey) {
    hoverMoveRequest = { point, altKey };
    hoverMoveVersion += 1;
    if (!hoverMoveFrame && !hoverMoveRunning) {
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
    if (options.pointDistance(postCommitHoverBlockPoint, point) <= options.cssPxToPt(4)) {
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
    }
  }

  function clearEditorOverlayRoot() {
    const viewerSvg = options.viewerSvg?.();
    viewerSvg?.querySelector('[data-layer="editor-overlay"]')?.remove();
  }

  async function clearEngineHoverOverlay({ keepSelectionOverlay = false } = {}) {
    cancelScheduledHoverMove();
    options.clearTlcHoverState?.();
    await options.state().editorEngine?.clearInteraction?.();
    invalidateEngineReadCache();
    if (keepSelectionOverlay) {
      options.renderEditorOverlay?.(options.currentEditorRenderList?.() || []);
    } else {
      clearEditorOverlayRoot();
    }
  }

  function clearInteractionOverlayNow() {
    void clearEngineHoverOverlay();
  }

  function scheduleDocumentPreviewFrame() {
    if (documentPreviewFrame) {
      return;
    }
    documentPreviewFrame = requestAnimationFrame(() => {
      documentPreviewFrame = 0;
      const gesture = options.activeSelectionGesture();
      if (!gesture || !gesture.localDocumentPreviewActive) {
        return;
      }
      if (options.applyDocumentObjectPreviewTransform()) {
        clearEditorOverlayRoot();
      }
    });
  }

  function syncSelectionHoverSuppressionCursor(point, state) {
    const viewerSvg = options.viewerSvg?.();
    if (!viewerSvg) {
      return;
    }
    if (options.editorState().activeTool === "select") {
      const resizeHandle = options.selectionResizeHandleHit(point);
      if (resizeHandle) {
        viewerSvg.style.cursor = resizeHandle.cursor;
        return;
      }
      if (options.selectionRotateHandleHit(point)) {
        viewerSvg.style.cursor = "grab";
        return;
      }
    }
    if (state.overSelectionHit) {
      viewerSvg.style.cursor = "grab";
    } else {
      options.syncCanvasCursor?.();
    }
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
      const resizeHandle = options.selectionResizeHandleHit(point);
      if (resizeHandle) {
        const viewerSvg = options.viewerSvg?.();
        if (viewerSvg) {
          viewerSvg.style.cursor = resizeHandle.cursor;
        }
        return;
      }
      if (options.selectionRotateHandleHit(point)) {
        const viewerSvg = options.viewerSvg?.();
        if (viewerSvg) {
          viewerSvg.style.cursor = "grab";
        }
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
    await options.state().editorEngine.pointerMove(point.x, point.y, altKey);
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
    const renderList = options.currentEditorInteractionRenderList?.() || options.currentEditorRenderList();
    options.renderEditorOverlay(renderList);
    options.positionActiveTextEditor();
    if (editorState.elementPlacementActive) {
      options.syncCanvasCursor();
    } else if (editorState.activeTool === "select") {
      await options.syncSelectCursorForPoint(point);
    } else if (toolSupportsSelectionBoxMove(editorState.activeTool)) {
      await options.syncArrowAwareCursorForPoint(point);
    }
  }

  function toolSupportsSelectionBoxMove(tool) {
    return tool === "bond"
      || tool === "arrow"
      || tool === "bracket"
      || tool === "symbol"
      || tool === "element"
      || tool === "shape"
      || tool === "tlc-plate"
      || tool === "orbital"
      || tool === "templates"
      || tool === "chain";
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
    if (!await options.state().editorEngine.beginSelectionMove?.(point.x, point.y, !!event.shiftKey, event.altKey)) {
      return false;
    }
    invalidateEngineReadCache();
    options.setActiveSelectionGesture({
      kind: "move",
      start: point,
      current: point,
      dragged: false,
      additive: !!event.shiftKey,
    });
    await syncCursor(point);
    clearEditorOverlayRoot();
    return true;
  }

  function selectionHandleZoneContainsPoint(point) {
    const bounds = options.currentRenderBounds?.("selection");
    if (!bounds) {
      return true;
    }
    const edgePad = options.cssPxToPt(14);
    const rotatePad = options.cssPxToPt(18);
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
      y: bounds.minY - options.cssPxToPt(18),
    };
    return options.pointDistance(point, rotateHandle) <= rotatePad;
  }

  async function beginLargeSelectionMoveFastPath(point, event) {
    if (!options.selectionHasLargeOverlay?.()) {
      return false;
    }
    if (!options.selectionHitContainsPoint?.(point)) {
      return false;
    }
    if (selectionHandleZoneContainsPoint(point)) {
      return false;
    }
    return beginSelectionMoveGesture(point, event, options.syncSelectCursorForPoint);
  }

  function toolUsesEngineDragPreview(tool) {
    return tool === "bond"
      || tool === "arrow"
      || tool === "bracket"
      || tool === "symbol"
      || tool === "shape"
      || tool === "tlc-plate"
      || tool === "orbital"
      || tool === "templates"
      || tool === "chain";
  }

  function primaryButtonIsDown(event) {
    return (event.buttons & 1) === 1;
  }

  function invalidateEngineReadCache() {
    options.invalidateEditorEngineReadCache?.();
  }

  async function updateEngineDragPreview(point, event) {
    event.preventDefault();
    cancelScheduledHoverMove();
    leaveSelectionHoverSuppression(point);
    await options.state().editorEngine.pointerMove(point.x, point.y, event.altKey);
    invalidateEngineReadCache();
    options.renderEditorOverlay(
      options.currentEditorInteractionRenderList?.() || options.syncEditorRenderListFromEngine(),
    );
    options.positionActiveTextEditor();
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
    const point = options.svgPointFromEvent(event);
    const editorState = options.editorState();
    const gesture = options.activeSelectionGesture();
    if ((editorState.activeTool === "select" || toolSupportsSelectionBoxMove(editorState.activeTool)) && gesture) {
      event.preventDefault();
      if (gesture.kind === "tlc-spot-drag") {
        gesture.current = point;
        gesture.dragged = options.pointDistance(gesture.start, point) >= options.cssPxToPt(1.5);
        const hit = options.parseEngineJson(
          await options.state().editorEngine.updateTlcSpotDragJson?.(point.x, point.y),
          null,
        );
        if (hit) {
          gesture.hit = hit;
          await options.syncDocumentFromEngine();
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
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(3)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        await options.state().editorEngine.updateHoverArrowEdit?.(point.x, point.y, event.altKey);
        invalidateEngineReadCache();
        if (gesture.kind === "arrow-curve") {
          gesture.angle = options.state().editorEngine.activeArrowEditDegrees?.() || 0;
        }
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.syncEditorRenderListFromEngine());
        return;
      }
      if (gesture.kind === "shape-resize") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(3)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        await options.state().editorEngine.updateHoverShapeEdit?.(point.x, point.y, event.altKey);
        invalidateEngineReadCache();
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.syncEditorRenderListFromEngine());
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
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
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
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
        return;
      }
      if (gesture.kind === "move") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(3)) {
          gesture.dragged = true;
        }
        gesture.current = point;
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
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
        return;
      }
      if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(3)) {
        gesture.dragged = true;
      }
      gesture.current = point;
      if (editorState.selectMode === "free") {
        const lastPoint = gesture.points[gesture.points.length - 1];
        if (!lastPoint || options.pointDistance(lastPoint, point) >= options.cssPxToPt(2)) {
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
      await updateEngineDragPreview(point, event);
      return;
    }
    const selectionHoverSuppression = selectionHoverSuppressionState(point);
    if (selectionHoverSuppression) {
      event.preventDefault();
      enterSelectionHoverSuppression(point, selectionHoverSuppression);
      return;
    }
    leaveSelectionHoverSuppression(point);
    scheduleHoverPointerMove(point, event.altKey);
  }

  async function handleEditorPointerDown(event) {
    if (!options.routeEditorPointerEvents() || event.button !== 0) {
      return;
    }
    cancelScheduledHoverMove();
    cancelDocumentPreviewFrame();
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
      await options.syncDocumentFromEngine();
      options.renderEditorOverlay();
      return;
    }
    if (editorState.activeTool === "select") {
      event.preventDefault();
      options.viewerSvg().setPointerCapture?.(event.pointerId);
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
      if (await beginLargeSelectionMoveFastPath(point, event)) {
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
      const overSelection = !!options.state().editorEngine.selectionContainsPoint?.(point.x, point.y);
      const shapeEditAction = overSelection
        ? ""
        : await options.state().editorEngine.beginHoverShapeEdit?.(point.x, point.y) || "";
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
        options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.currentEditorRenderList());
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
        options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.currentEditorRenderList());
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
      if (await beginSelectionMoveGesture(point, event, options.syncSelectCursorForPoint)) {
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
    options.viewerSvg().setPointerCapture?.(event.pointerId);
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
        options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.currentEditorRenderList());
        return;
      }
    }
    if (editorState.activeTool === "arrow" && await beginSelectionBoxMove(point, event)) {
      return;
    }
    if (editorState.activeTool === "shape"
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
          options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.currentEditorRenderList());
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
        options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || options.currentEditorRenderList());
        return;
      }
    }
    if (await beginSelectionBoxMove(point, event)) {
      return;
    }
    if (editorState.activeTool === "bracket") {
      options.setActiveBracketDragStart(point);
    }
    const beforeRevision = engineRevision();
    await options.state().editorEngine.pointerDown(point.x, point.y, event.altKey);
    const pointerDownResult = readLastCommandResult();
    if (
      pointerDownResult?.changed
      && Number(pointerDownResult.beforeRevision ?? pointerDownResult.before_revision ?? -1) === beforeRevision
    ) {
      await options.syncDocumentFromEngine();
      options.renderDocumentChange?.(pointerDownResult) || options.renderDocument();
      return;
    }
    invalidateEngineReadCache();
    options.renderEditorOverlay(options.currentEditorInteractionRenderList?.() || []);
  }

  async function handleEditorPointerUp(event) {
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
    options.viewerSvg().releasePointerCapture?.(event.pointerId);
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
          if (changed) {
            await options.state().editorEngine.refreshRenderState?.();
          }
          return changed;
        },
      );
      const changed = !!result.changed;
      if (!changed && !gesture.dragged && options.editorState().activeTool === "select") {
        await options.selectClickTarget(point, gesture.additive);
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
        return;
      }
      if (changed) {
        await options.syncArrowAwareCursorForPoint(point);
        options.renderDocumentChange?.(result) || options.renderDocument();
      } else {
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
      }
      return;
    }
    if ((options.editorState().activeTool === "select"
      || options.editorState().activeTool === "shape"
      || options.editorState().activeTool === "tlc-plate"
      || options.editorState().activeTool === "orbital")
      && gesture?.kind === "shape-resize") {
      options.setActiveSelectionGesture(null);
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
          if (changed) {
            await options.state().editorEngine.refreshRenderState?.();
          }
          return changed;
        },
      );
      const changed = !!result.changed;
      if (!changed && !gesture.dragged && options.editorState().activeTool === "select") {
        await options.selectClickTarget(point, gesture.additive);
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
        return;
      }
      if (changed) {
        await options.syncArrowAwareCursorForPoint(point);
        options.renderDocumentChange?.(result) || options.renderDocument();
      } else {
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
      }
      return;
    }
    if (gesture?.kind === "move") {
      options.setActiveSelectionGesture(null);
      if (gesture.dragged) {
        const commitPoint = gesture.current || point;
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
        );
        suppressHoverUntilPointerLeavesPoint(commitPoint);
        options.clearDocumentObjectPreviewTransform();
        await clearEngineHoverOverlay();
        await options.syncArrowAwareCursorForPoint(commitPoint);
        await clearEngineHoverOverlay();
        options.renderDocumentChange?.(result) || options.renderDocument();
        clearEditorOverlayRoot();
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
        );
        await options.syncSelectCursorForPoint(point);
        options.clearDocumentObjectPreviewTransform();
        options.renderDocumentChange?.(result) || options.renderDocument();
        return;
      }
      if (gesture.kind === "resize") {
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
        );
        await options.syncSelectCursorForPoint(point);
        options.clearDocumentObjectPreviewTransform();
        options.renderDocumentChange?.(result) || options.renderDocument();
        return;
      }
      if (gesture.kind === "move") {
        if (gesture.dragged) {
          const commitPoint = gesture.current || point;
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
          );
          suppressHoverUntilPointerLeavesPoint(commitPoint);
          await clearEngineHoverOverlay();
          await options.syncSelectCursorForPoint(commitPoint);
          await clearEngineHoverOverlay();
          options.clearDocumentObjectPreviewTransform();
          options.renderDocumentChange?.(result) || options.renderDocument();
          clearEditorOverlayRoot();
        } else {
          await options.selectClickTarget(gesture.start || point, gesture.additive);
          options.clearDocumentObjectPreviewTransform();
          await options.renderSelectionOnlyUpdate(gesture.start || point);
        }
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
    clearInteractionOverlayNow();
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
    );
    const pendingGraphicObjectId = await Promise.resolve(
      options.state().editorEngine.pendingGraphicObjectId?.() || "",
    );
    options.renderDocumentChange?.(result) || options.renderDocument();
    invalidateEngineReadCache();
    clearInteractionOverlayNow();
    if (options.editorState().activeTool === "bracket") {
      const start = options.activeBracketDragStart();
      options.setActiveBracketDragStart(null);
      if (start && options.pointDistance(start, point) >= options.cssPxToPt(4)) {
        await options.openTextEditorAt(
          options.bracketLabelAnchorPoint(start, point),
          {
            ...(options.bracketLabelTextOptions?.() || {}),
            bracketObjectId: pendingGraphicObjectId,
          },
        );
      }
    }
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
    options.clearDocumentObjectPreviewTransform();
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
