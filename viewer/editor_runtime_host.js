export function createEditorRuntimeHost(scope) {
  const { viewerSvg, DOMPoint, activeViewBox, state, BOND_STROKE, isEditingRustDocument, editorState, getCanvasPointerShieldActive, viewportScale, commandEngine, renderDocumentChange, BRACKET_LABEL_OFFSET_X, BRACKET_LABEL_OFFSET_Y, sceneRenderer, resetDocumentRenderState, applyViewerViewport, normalizeDisplayColor, CHEMDRAW_PAGE_BACKGROUND, makeSvgNode, rebuildDocumentPrimitiveIndex, syncViewerStats, renderEditorOverlay, positionActiveTextEditor, ensureDocumentTab, renderDocumentTabs, desktopFileHost, loadDetachedDocumentPayload, takeBrowserPendingDocument, loadBrowserPendingDocumentPayload, openDocumentPath, saveActiveDocumentTabState, openDocumentPathInTab, loadAndRender } = scope;

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
        || editorState.activeTool === "table"
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
      || editorState.activeTool === "table"
      || editorState.activeTool === "tlc-plate"
      || editorState.activeTool === "orbital"
      || editorState.activeTool === "templates"
      || editorState.activeTool === "chain";
  }

  function syncViewerSvgPointerEventMode() {
    viewerSvg?.classList.toggle(
      "is-pointer-capture-disabled",
      getCanvasPointerShieldActive() || activeToolUsesContainerPointerEvents(),
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

  function renderDocument() {
    const documentData = state.currentDocument;
    if (!documentData) {
      return;
    }
    if (window.__chemsemaDebug?.renderStats) {
      window.__chemsemaDebug.renderStats.documentRenderCount += 1;
    }
  
    const page = documentData.document.page;
    const viewBox = activeViewBox();
    viewerSvg.innerHTML = "";
    resetDocumentRenderState();
    applyViewerViewport();
    const pageBackground = normalizeDisplayColor(page.background, CHEMDRAW_PAGE_BACKGROUND);
    viewerSvg.style.setProperty("--chemsema-page-bg", pageBackground);
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

  return { svgPointFromEvent, editorBondStrokeWidth, routeEditorPointerEvents, activeToolUsesContainerPointerEvents, syncViewerSvgPointerEventMode, screenPxToWorld, applySelectionArrangeCommand, applyArrowOptionsToSelection, bracketLabelAnchorPoint, handleViewerContainerPointerEvent, renderDocument, loadInitialDocumentTabs };
}
