export function createEditorSelectionState(options) {
  function editorSelectionHasItems(selection) {
    if (!selection) {
      return false;
    }
    return Boolean(selection.region)
      || ["nodes", "bonds", "labelNodes", "arrowObjects", "textObjects"].some((key) => (
        Array.isArray(selection[key]) && selection[key].length > 0
      ));
  }

  function currentEditorSelectionHasItems() {
    return editorSelectionHasItems(options.currentEditorEngineState()?.selection);
  }

  function sceneObjectHasSelectableContent(object, resources) {
    if (!object || object.visible === false) {
      return false;
    }
    if (object.type === "molecule") {
      const resource = resources?.[object.payload?.resourceRef];
      const fragment = resource?.data;
      return Boolean(fragment?.nodes?.length || fragment?.bonds?.length);
    }
    if (object.type === "group") {
      return true;
    }
    return ["text", "line", "bracket", "symbol", "shape"].includes(object.type);
  }

  function currentDocumentHasSelectableContent() {
    const documentData = options.state().currentDocument;
    if (!documentData?.objects?.length) {
      return false;
    }
    return documentData.objects.some((object) => sceneObjectHasSelectableContent(object, documentData.resources));
  }

  function activeDocumentTabIsBlankUntitled() {
    const tab = options.activeDocumentTab();
    if (!tab) {
      return false;
    }
    const title = options.documentTitleFromState();
    const state = options.state();
    const hasPath = Boolean(state.currentPath || state.currentFileName || state.currentFilePath);
    return title === "Untitled" && !hasPath && !currentDocumentHasSelectableContent();
  }

  function collectSceneObjects(objects = [], out = new Map()) {
    for (const object of objects || []) {
      out.set(object.id, object);
      if (Array.isArray(object.children)) {
        collectSceneObjects(object.children, out);
      }
    }
    return out;
  }

  function currentSceneObjectMap() {
    return collectSceneObjects(options.state().currentDocument?.objects || []);
  }

  function currentEditableFragment() {
    const documentData = options.state().currentDocument;
    const molecule = documentData?.objects?.find((object) => object.type === "molecule" && object.payload?.resourceRef);
    return molecule ? documentData.resources?.[molecule.payload.resourceRef]?.data || null : null;
  }

  function currentSelectionInfo() {
    const selection = options.currentEditorEngineState()?.selection || {};
    const objectMap = currentSceneObjectMap();
    const textObjects = (selection.textObjects || []).map((id) => objectMap.get(id)).filter(Boolean);
    const graphicObjects = (selection.arrowObjects || []).map((id) => objectMap.get(id)).filter(Boolean);
    const fragment = currentEditableFragment();
    const nodeIds = selection.nodes || [];
    const bondIds = selection.bonds || [];
    const labelNodeIds = selection.labelNodes || [];
    return {
      selection,
      objectMap,
      textObjects,
      graphicObjects,
      sceneObjects: textObjects.concat(graphicObjects),
      fragment,
      nodes: nodeIds.map((id) => fragment?.nodes?.find((node) => node.id === id)).filter(Boolean),
      bonds: bondIds.map((id) => fragment?.bonds?.find((bond) => bond.id === id)).filter(Boolean),
      labelNodes: labelNodeIds.map((id) => fragment?.nodes?.find((node) => node.id === id)).filter(Boolean),
    };
  }

  function clearTlcHoverState() {
    options.setActiveTlcSpotHover(null);
    options.setActiveTlcLaneHover(null);
  }

  async function updateTlcSpotHover(point) {
    const state = options.state();
    const editorState = options.editorState();
    if (!state.editorEngine || (editorState.activeTool !== "select" && editorState.activeTool !== "tlc-plate")) {
      options.setActiveTlcSpotHover(null);
      options.setActiveTlcLaneHover(null);
      return null;
    }
    const gesture = options.activeSelectionGesture();
    if (gesture?.kind === "tlc-spot-drag") {
      options.setActiveTlcSpotHover(gesture.hit || null);
      options.setActiveTlcLaneHover(null);
      return gesture.hit || null;
    }
    const spotHit = options.parseEngineJson(
      await state.editorEngine.tlcSpotHitTestJson?.(point.x, point.y),
      null,
    );
    options.setActiveTlcSpotHover(spotHit);
    const laneHit = spotHit
      ? null
      : options.parseEngineJson(
        await state.editorEngine.tlcLaneGuideHitTestJson?.(point.x, point.y),
        null,
      );
    options.setActiveTlcLaneHover(laneHit);
    return spotHit;
  }

  function contextSelectionCount(info = currentSelectionInfo()) {
    return info.sceneObjects.length + info.nodes.length + info.bonds.length + info.labelNodes.length;
  }

  function contextHasSelection(info = currentSelectionInfo()) {
    return contextSelectionCount(info) > 0 || Boolean(info.selection?.region);
  }

  function selectedSceneObjects() {
    return currentSelectionInfo().sceneObjects;
  }

  return {
    editorSelectionHasItems,
    currentEditorSelectionHasItems,
    currentDocumentHasSelectableContent,
    activeDocumentTabIsBlankUntitled,
    currentSceneObjectMap,
    currentEditableFragment,
    currentSelectionInfo,
    clearTlcHoverState,
    updateTlcSpotHover,
    contextSelectionCount,
    contextHasSelection,
    selectedSceneObjects,
  };
}
