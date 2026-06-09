export function createEditorPointerController(options) {
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
    if (tool === "templates") {
      return "insert-template";
    }
    if (tool === "delete") {
      return "delete-selection";
    }
    return "pointer-document-edit";
  }

  async function handleEditorPointerMove(event) {
    const point = options.svgPointFromEvent(event);
    const editorState = options.editorState();
    const gesture = options.activeSelectionGesture();
    if ((editorState.activeTool === "select"
      || editorState.activeTool === "arrow"
      || editorState.activeTool === "shape"
      || editorState.activeTool === "tlc-plate"
      || editorState.activeTool === "orbital") && gesture) {
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
        options.renderDocument();
        return;
      }
      if (gesture.kind === "arrow-endpoint" || gesture.kind === "arrow-curve") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(3)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        await options.state().editorEngine.updateHoverArrowEdit?.(point.x, point.y, event.altKey);
        if (gesture.kind === "arrow-curve") {
          gesture.angle = options.state().editorEngine.activeArrowEditDegrees?.() || 0;
        }
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
        return;
      }
      if (gesture.kind === "shape-resize") {
        if (options.pointDistance(gesture.start, point) >= options.cssPxToPt(3)) {
          gesture.dragged = true;
        }
        gesture.current = point;
        await options.state().editorEngine.updateHoverShapeEdit?.(point.x, point.y, event.altKey);
        await options.syncArrowAwareCursorForPoint(point);
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
        return;
      }
      if (gesture.kind === "rotate") {
        gesture.current = point;
        gesture.angle = options.selectionRotateAngleForGesture(gesture, point, event.altKey);
        if (options.applyDocumentObjectPreviewTransform()) {
          await options.syncSelectCursorForPoint(point);
          options.renderEditorOverlay(options.currentEditorOverlayRenderList());
          return;
        }
        await options.state().editorEngine.updateSelectionRotate(point.x, point.y, event.altKey);
        await options.syncSelectCursorForPoint(point);
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
        return;
      }
      if (gesture.kind === "resize") {
        gesture.current = point;
        gesture.scale = options.selectionResizeGestureScale(gesture, point);
        if (options.applyDocumentObjectPreviewTransform()) {
          await options.syncSelectCursorForPoint(point);
          options.renderEditorOverlay(options.currentEditorOverlayRenderList());
          return;
        }
        await options.state().editorEngine.updateSelectionResize?.(point.x, point.y);
        await options.syncSelectCursorForPoint(point);
        options.renderEditorOverlay(options.syncEditorRenderListFromEngine());
        return;
      }
      if (gesture.kind === "move") {
        gesture.current = point;
        if (options.applyDocumentObjectPreviewTransform()) {
          await options.syncSelectCursorForPoint(point);
          if (!options.syncEditorOverlayPreviewTransform()) {
            options.renderEditorOverlay(options.currentEditorOverlayRenderList());
          }
          return;
        }
        await options.state().editorEngine.updateSelectionMove(point.x, point.y, event.altKey);
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
      options.renderEditorOverlay(options.currentEditorRenderList());
      return;
    }
    if (!options.routeEditorPointerEvents()) {
      if (options.isEditingRustDocument()) {
        await options.state().editorEngine.clearInteraction();
        options.renderEditorOverlay();
      }
      return;
    }
    await options.state().editorEngine.pointerMove(point.x, point.y, event.altKey);
    if (!editorState.elementPlacementActive && (editorState.activeTool === "select" || editorState.activeTool === "tlc-plate") && !options.activeSelectionGesture()) {
      await options.updateTlcSpotHover(point);
    } else if (options.activeSelectionGesture()?.kind !== "tlc-spot-drag") {
      options.clearTlcHoverState();
    }
    if (editorState.elementPlacementActive) {
      options.syncCanvasCursor();
    } else if (editorState.activeTool === "select") {
      await options.syncSelectCursorForPoint(point);
    } else if (editorState.activeTool === "arrow"
      || editorState.activeTool === "shape"
      || editorState.activeTool === "tlc-plate"
      || editorState.activeTool === "orbital") {
      await options.syncArrowAwareCursorForPoint(point);
    }
    const renderList = options.currentEditorRenderList();
    options.maybeAutoExpandEditorViewport(renderList);
    options.renderEditorOverlay(renderList);
    options.positionActiveTextEditor();
  }

  async function handleEditorPointerDown(event) {
    if (!options.routeEditorPointerEvents() || event.button !== 0) {
      return;
    }
    const point = options.svgPointFromEvent(event);
    options.setLastEditFocusPoint(point);
    const editorState = options.editorState();
    if (editorState.elementPlacementActive) {
      event.preventDefault();
      options.viewerSvg().setPointerCapture?.(event.pointerId);
      await options.state().editorEngine.pointerDown(point.x, point.y, event.altKey);
      await options.syncDocumentFromEngine();
      options.renderEditorOverlay(options.currentEditorRenderList());
      return;
    }
    if (editorState.activeTool === "bracket") {
      options.setActiveBracketDragStart(point);
    }
    if (editorState.activeTool === "text") {
      event.preventDefault();
      await options.openTextEditorAt(point);
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
          cursor: "ns-resize",
          hit: tlcSpotHit,
        });
        options.setActiveTlcSpotHover(tlcSpotHit);
        options.setActiveTlcLaneHover(null);
        await options.selectClickTarget(point, !!event.shiftKey);
        await options.renderSelectionOnlyUpdate(point);
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
        options.syncEditorRenderListFromEngine();
        options.renderEditorOverlay(options.currentEditorOverlayRenderList());
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
        options.renderEditorOverlay(options.currentEditorRenderList());
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
        options.renderEditorOverlay(options.currentEditorRenderList());
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
          options.syncEditorRenderListFromEngine();
          options.renderEditorOverlay(options.currentEditorOverlayRenderList());
          return;
        }
      }
      if (overSelection && await options.state().editorEngine.beginSelectionMove?.(point.x, point.y, !!event.shiftKey, event.altKey)) {
        options.setActiveSelectionGesture({
          kind: "move",
          start: point,
          current: point,
          additive: !!event.shiftKey,
        });
        await options.syncSelectCursorForPoint(point);
        options.syncEditorRenderListFromEngine();
        options.renderEditorOverlay(options.currentEditorOverlayRenderList());
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
      options.renderEditorOverlay(options.currentEditorRenderList());
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
        options.renderEditorOverlay(options.currentEditorRenderList());
        return;
      }
    }
    if (editorState.activeTool === "shape" || editorState.activeTool === "tlc-plate") {
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
            cursor: "ns-resize",
            hit: tlcSpotHit,
          });
          options.setActiveTlcSpotHover(tlcSpotHit);
          options.setActiveTlcLaneHover(null);
          await options.syncArrowAwareCursorForPoint(point);
          options.renderEditorOverlay(options.currentEditorRenderList());
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
        options.renderEditorOverlay(options.currentEditorRenderList());
        return;
      }
    }
    await options.state().editorEngine.pointerDown(point.x, point.y, event.altKey);
    await options.syncDocumentFromEngine();
    options.renderEditorOverlay(options.currentEditorRenderList());
  }

  async function handleEditorPointerUp(event) {
    if (options.editorState().activeTool === "text") {
      return;
    }
    if (!options.routeEditorPointerEvents()) {
      return;
    }
    const point = options.svgPointFromEvent(event);
    options.setLastEditFocusPoint(point);
    event.preventDefault();
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
      options.renderDocument();
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
        options.renderDocument();
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
        options.renderDocument();
      } else {
        options.clearDocumentObjectPreviewTransform();
        await options.renderSelectionOnlyUpdate(point, options.syncArrowAwareCursorForPoint);
      }
      return;
    }
    if (options.editorState().activeTool === "select") {
      options.setActiveSelectionGesture(null);
      if (!gesture) {
        return;
      }
      if (gesture.kind === "rotate") {
        await executeDocumentCommand(
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
        options.renderDocument();
        return;
      }
      if (gesture.kind === "resize") {
        await executeDocumentCommand(
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
        options.renderDocument();
        return;
      }
      if (gesture.kind === "move") {
        if (gesture.dragged) {
          await executeDocumentCommand(
            {
              type: "move-selection",
              payload: {
                start: gesture.start,
                end: point,
                altKey: event.altKey,
              },
            },
            () => options.state().editorEngine.finishSelectionMove(point.x, point.y, event.altKey),
          );
          await options.syncSelectCursorForPoint(point);
          options.clearDocumentObjectPreviewTransform();
          options.renderDocument();
        } else {
          await options.selectClickTarget(point, gesture.additive);
          options.clearDocumentObjectPreviewTransform();
          await options.renderSelectionOnlyUpdate(point);
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
    await executeDocumentCommand(
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
    options.renderDocument();
    if (options.editorState().activeTool === "bracket") {
      const start = options.activeBracketDragStart();
      options.setActiveBracketDragStart(null);
      if (start && options.pointDistance(start, point) >= options.cssPxToPt(4)) {
        await options.openTextEditorAt(options.bracketLabelAnchorPoint(start, point));
      }
    }
  }

  async function handleEditorPointerLeave() {
    if (!options.isEditingRustDocument()) {
      return;
    }
    if (options.editorState().activeTool === "select" && options.activeSelectionGesture()) {
      return;
    }
    options.clearTlcHoverState();
    if (options.editorState().activeTool !== "text") {
      await options.state().editorEngine.clearInteraction();
      options.renderEditorOverlay();
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
    options.setActiveSelectionGesture(null);
    options.clearDocumentObjectPreviewTransform();
    await options.state().editorEngine?.clearInteraction?.();
    options.syncCanvasCursor();
    options.renderEditorOverlay();
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
