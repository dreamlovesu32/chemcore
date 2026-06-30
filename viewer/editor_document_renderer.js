import {
  primitiveStrokeWidthValue,
  renderCorePrimitive,
} from "./primitive_dom_renderer.js";

const DOCUMENT_PREVIEW_BATCH_ELEMENT_THRESHOLD = 96;

export function createEditorDocumentRenderer(options) {
  const {
    state,
    viewerSvg,
    viewerStats,
    sceneRenderer,
    makeSvgNode,
    parseEngineJson,
  } = options;
  let activeDocumentPreviewObjectIds = new Set();
  let activeDocumentPreviewPrimitiveElements = new Set();
  let activeDocumentPreviewHiddenElements = new Set();
  let activeDocumentEditPreviewHiddenElements = new Set();
  let activeDocumentDiagnosticPreviewHiddenElements = new Set();
  let activeDocumentPreviewLayer = false;
  let activeDocumentPreviewBatchLayer = null;
  let activeDocumentPreviewTransform = "";
  let documentPrimitiveNodeElements = new Map();
  let documentPrimitiveBondElements = new Map();
  let documentMoleculeTopologyCache = null;

  function resetDocumentRenderState() {
    activeDocumentPreviewObjectIds = new Set();
    activeDocumentPreviewPrimitiveElements = new Set();
    activeDocumentPreviewHiddenElements = new Set();
    activeDocumentEditPreviewHiddenElements = new Set();
    activeDocumentDiagnosticPreviewHiddenElements = new Set();
    activeDocumentPreviewLayer = false;
    activeDocumentPreviewBatchLayer = null;
    activeDocumentPreviewTransform = "";
    documentPrimitiveNodeElements = new Map();
    documentPrimitiveBondElements = new Map();
    documentMoleculeTopologyCache = null;
  }

  const activeSelectionGesture = () => options.activeSelectionGesture?.() || null;
  const currentEditorEngineState = (...args) => options.currentEditorEngineState(...args);
  const currentEditorRenderList = (...args) => options.currentEditorRenderList(...args);
  const editorSelectionHasItems = (...args) => options.editorSelectionHasItems(...args);
  const worldPointForFragmentNode = (...args) => options.worldPointForFragmentNode(...args);
  const isEditingRustDocument = (...args) => options.isEditingRustDocument(...args);
  const renderDocument = (...args) => options.renderDocument(...args);
  const renderEditorOverlay = (...args) => options.renderEditorOverlay(...args);
  const positionActiveTextEditor = (...args) => options.positionActiveTextEditor(...args);
  const syncDocumentFromEngine = (...args) => options.syncDocumentFromEngine(...args);
  const corePrimitiveRenderOptions = (...args) => options.corePrimitiveRenderOptions(...args);
  const editorBondStrokeWidth = (...args) => options.editorBondStrokeWidth(...args);
  const selectionResizePivot = (...args) => options.selectionResizePivot(...args);
  const freshestPreviewSelection = (...args) => options.freshestPreviewSelection(...args);

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
  
  function currentDocumentSceneObjectParentMap() {
    const parents = new Map();
    const visit = (object, parent = null) => {
      if (!object?.id) {
        return;
      }
      if (parent?.id) {
        parents.set(object.id, parent.id);
      }
      for (const child of object.children || []) {
        visit(child, object);
      }
    };
    for (const object of state.currentDocument?.objects || []) {
      visit(object, null);
    }
    return parents;
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
  
  function expandObjectIdsWithRenderableAncestors(objectIds, objectMap = currentDocumentSceneObjectMap()) {
    const expanded = new Set(objectIds);
    const parentMap = currentDocumentSceneObjectParentMap();
    for (const objectId of [...objectIds]) {
      let currentId = objectId;
      while (parentMap.has(currentId)) {
        const parentId = parentMap.get(currentId);
        if (!parentId || expanded.has(parentId)) {
          break;
        }
        expanded.add(parentId);
        currentId = parentId;
      }
    }
    return expandObjectIdsWithDescendants(expanded, objectMap);
  }
  
  function topmostObjectIds(objectIds) {
    const ids = new Set(objectIds);
    const parentMap = currentDocumentSceneObjectParentMap();
    return [...ids].filter((objectId) => {
      let currentId = objectId;
      while (parentMap.has(currentId)) {
        currentId = parentMap.get(currentId);
        if (ids.has(currentId)) {
          return false;
        }
      }
      return true;
    });
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
    return expandObjectIdsWithRenderableAncestors(objectIds);
  }
  
  function removeDocumentObjectDom(documentLayer, objectId) {
    const selector = `[data-object-id="${CSS.escape(objectId)}"]`;
    const nodes = [...documentLayer.querySelectorAll(selector)];
    const nodeSet = new Set(nodes);
    let removed = 0;
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
        removed += 1;
      }
    }
    return removed;
  }
  
  function removeDocumentObjectDomTree(documentLayer, objectId, objectMap = currentDocumentSceneObjectMap()) {
    const removed = removeDocumentObjectDom(documentLayer, objectId);
    if (removed > 0) {
      return removed;
    }
    let descendantRemoved = 0;
    const visit = (object) => {
      for (const child of object?.children || []) {
        descendantRemoved += removeDocumentObjectDomTree(documentLayer, child.id, objectMap);
      }
    };
    visit(objectMap.get(objectId));
    return descendantRemoved;
  }
  
  function highestPatchObjectIds(objectIds) {
    const ids = new Set(objectIds);
    const covered = new Set();
    const visit = (object, hasPatchedAncestor = false) => {
      if (!object?.id) {
        return;
      }
      if (hasPatchedAncestor && ids.has(object.id)) {
        covered.add(object.id);
      }
      const nextHasPatchedAncestor = hasPatchedAncestor || ids.has(object.id);
      for (const child of object.children || []) {
        visit(child, nextHasPatchedAncestor);
      }
    };
    for (const object of state.currentDocument?.objects || []) {
      visit(object, false);
    }
    return objectIds.filter((objectId) => !covered.has(objectId));
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
    if (result && result.changed === false) {
      return true;
    }
    if (!isEditingRustDocument() || !state.currentDocument || !result?.changed) {
      renderDocument();
      return true;
    }
    if (result.deferDocumentSync) {
      const patchPrimitiveTargetsFirst = targetIdsFromCommandResult(result, "nodes").size > 0
        || targetIdsFromCommandResult(result, "bonds").size > 0;
      const patched = patchPrimitiveTargetsFirst
        ? (renderDocumentPrimitiveChange(result) || renderDocumentObjectPrimitiveChange(result))
        : (renderDocumentObjectPrimitiveChange(result) || renderDocumentPrimitiveChange(result));
      if (patched) {
        renderEditorOverlay();
        return true;
      }
    }
    if (renderDocumentObjectPrimitiveChange(result)) {
      renderEditorOverlay();
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
  
  function renderDocumentObjectPrimitiveChange(result = null) {
    const objectIds = expandObjectIdsWithRenderableAncestors(targetIdsFromCommandResult(result, "objects"));
    const debugSample = {
      commandType: result?.commandType || result?.command?.type || null,
      objectIds: [...objectIds],
      entries: [],
      patched: false,
    };
    if (!objectIds.size) {
      if (window.__chemcoreDebug) {
        window.__chemcoreDebug.objectPrimitivePatchStats = debugSample;
      }
      return false;
    }
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    if (!documentLayer) {
      if (window.__chemcoreDebug) {
        window.__chemcoreDebug.objectPrimitivePatchStats = {
          ...debugSample,
          missingDocumentLayer: true,
        };
      }
      return false;
    }
    const objectMap = currentDocumentSceneObjectMap();
    const deletedObjectIds = new Set(result?.deleted?.objects || []);
    let patched = false;
    for (const objectId of deletedObjectIds) {
      const removed = removeDocumentObjectDomTree(documentLayer, objectId, objectMap);
      if (removed > 0) {
        debugSample.entries.push({ objectId, primitiveCount: 0, appended: false, removed });
        patched = true;
      }
    }
    for (const objectId of topmostObjectIds(objectIds)) {
      if (deletedObjectIds.has(objectId)) {
        continue;
      }
      const primitives = renderTargetObjectPrimitives(objectId);
      const entry = { objectId, primitiveCount: primitives.length, appended: false, removed: 0 };
      debugSample.entries.push(entry);
      if (!primitives.length) {
        continue;
      }
      removeDocumentObjectDom(documentLayer, objectId);
      const group = makeSvgNode("g", {
        "data-object-id": objectId,
        "data-renderer": "core-patch",
      });
      for (const primitive of primitives) {
        renderCorePrimitive(group, primitive, corePrimitiveRenderOptions());
      }
      documentLayer.appendChild(group);
      indexDocumentPrimitiveTree(group);
      entry.appended = true;
      entry.childCount = group.childNodes.length;
      entry.domCount = documentLayer.querySelectorAll(`[data-object-id="${cssEscape(objectId)}"]`).length;
      patched = true;
    }
    debugSample.patched = patched;
    if (window.__chemcoreDebug) {
      window.__chemcoreDebug.objectPrimitivePatchStats = debugSample;
    }
    if (!patched) {
      return false;
    }
    rebuildDocumentPrimitiveIndex(documentLayer);
    syncViewerStats();
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
    const patchNodeIds = new Set(nodeIds || []);
    const patchBondIds = new Set(bondIds || []);
    for (const primitive of primitives) {
      const nodeId = primitiveNodeId(primitive);
      const bondId = primitiveBondId(primitive);
      if (nodeId) {
        patchNodeIds.add(nodeId);
      }
      if (bondId) {
        patchBondIds.add(bondId);
      }
    }
    const targetElements = [...collectDocumentPrimitiveTargetElements(documentLayer, patchNodeIds, patchBondIds)];
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
    const removedCount = targetElements.length;
    const fragment = document.createDocumentFragment();
    for (const primitive of primitives) {
      renderCorePrimitive(fragment, primitive, corePrimitiveRenderOptions());
    }
    if (!fragment.childNodes.length) {
      if (removedCount > 0) {
        syncViewerStats();
        positionActiveTextEditor();
        return true;
      }
      return false;
    }
    indexDocumentPrimitiveTree(fragment);
    documentLayer.insertBefore(fragment, anchor);
    syncViewerStats();
    positionActiveTextEditor();
    return true;
  }
  
  function renderDocumentObjectIdPatch(objectIds = new Set()) {
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    if (!documentLayer || !objectIds.size) {
      return false;
    }
    const objectMap = currentDocumentSceneObjectMap();
    const patchIds = highestPatchObjectIds([...objectIds])
      .filter((objectId) => sceneObjectType(objectMap.get(objectId)) !== "molecule");
    if (!patchIds.length) {
      return false;
    }
    const patchSet = new Set(patchIds);
    const paintOrder = currentDocumentObjectIdsInPaintOrder();
    const orderedObjectIds = patchIds.sort((a, b) => {
      const ai = paintOrder.indexOf(a);
      const bi = paintOrder.indexOf(b);
      return (ai < 0 ? Number.MAX_SAFE_INTEGER : ai) - (bi < 0 ? Number.MAX_SAFE_INTEGER : bi);
    });
    for (const objectId of orderedObjectIds) {
      removeDocumentObjectDomTree(documentLayer, objectId, objectMap);
    }
    for (const objectId of orderedObjectIds) {
      const node = renderDocumentObjectPatchNode(objectId, objectMap);
      if (!node) {
        continue;
      }
      const anchor = findDocumentPatchAnchor(documentLayer, objectId, patchSet, paintOrder);
      documentLayer.insertBefore(node, anchor);
    }
    rebuildDocumentPrimitiveIndex(documentLayer);
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
    if (targetIdsFromCommandResult(result, "styles").size > 0) {
      return false;
    }
    const objectIds = targetIdsFromCommandResult(result, "objects");
    clearDocumentObjectPreviewTransform();
    let patched = false;
    if (nodeIds.size || bondIds.size) {
      const primitives = parseEngineJson(state.editorEngine?.renderTargetsJson?.(JSON.stringify({
        nodes: commandTargetSet(nodeIds),
        bonds: commandTargetSet(bondIds),
      })) || "[]", []);
      patched = renderDocumentPrimitivePatch(primitives, nodeIds, bondIds) || patched;
    }
    patched = renderDocumentObjectIdPatch(objectIds) || patched;
    return patched;
  }
  
  function cssEscape(value) {
    return window.CSS?.escape
      ? window.CSS.escape(String(value))
      : String(value).replace(/["\\]/g, "\\$&");
  }
  
  async function ensureDocumentObjectDomForCommandResult(result = null) {
    if (!isEditingRustDocument() || !result?.changed) {
      return false;
    }
    const objectIds = targetIdsFromCommandResult(result, "objects");
    if (!objectIds.size) {
      return false;
    }
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    if (!documentLayer) {
      return false;
    }
    const existingCount = [...objectIds].reduce(
      (count, objectId) => count + documentLayer.querySelectorAll(`[data-object-id="${cssEscape(objectId)}"]`).length,
      0,
    );
    if (existingCount > 0) {
      return true;
    }
    await syncDocumentFromEngine({ syncRenderList: false, refreshSnapshot: false });
    const patched = renderDocumentObjectIdPatch(objectIds);
    if (patched) {
      renderEditorOverlay();
    }
    return patched;
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
  
  function applyDocumentObjectOnlyPreviewTransform(selection, transform) {
    if (!transform) {
      return false;
    }
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    if (!documentLayer) {
      return false;
    }
    const objectIds = selectedDocumentPreviewObjectIds(selection);
    if (!objectIds.length) {
      return false;
    }
    removeDocumentPreviewBatchLayer();
    if (activeDocumentPreviewLayer) {
      documentLayer.removeAttribute("transform");
      activeDocumentPreviewLayer = false;
    }
    for (const element of activeDocumentPreviewPrimitiveElements) {
      restoreDocumentPreviewElementTransform(element);
    }
    activeDocumentPreviewPrimitiveElements = new Set();
  
    const nextIds = new Set(objectIds);
    for (const objectId of activeDocumentPreviewObjectIds) {
      if (!nextIds.has(objectId)) {
        for (const element of documentObjectElements(objectId)) {
          restoreDocumentPreviewElementTransform(element);
        }
      }
    }
    const nextObjectElements = new Map();
    for (const objectId of nextIds) {
      const elements = documentObjectElements(objectId);
      if (!elements.length) {
        continue;
      }
      nextObjectElements.set(objectId, elements);
    }
    if (!nextObjectElements.size) {
      return false;
    }
    const appliedElements = new Set();
    for (const elements of nextObjectElements.values()) {
      for (const element of elements) {
        if (appliedElements.has(element)) {
          continue;
        }
        appliedElements.add(element);
        applyDocumentPreviewElementTransform(element, transform);
      }
    }
    activeDocumentPreviewObjectIds = new Set(nextObjectElements.keys());
    activeDocumentPreviewTransform = transform;
    return true;
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
    const gesture = activeSelectionGesture();
    let selection = freshestPreviewSelection(gesture?.previewSelection);
    if (gesture?.kind !== "move" || !point || !selectionNeedsBackendMovePreview(selection)) {
      return false;
    }
    hideDocumentDiagnosticsForPreview();
    gesture.previewSelection = selection;
    const targets = structurePreviewTargetIds(selection);
    const started = performance.now();
    const moveResult = state.editorEngine.updateSelectionMove?.(point.x, point.y, altKey);
    const changed = moveResult && typeof moveResult.then === "function" ? await moveResult : moveResult;
    const updatedAt = performance.now();
    selection = freshestPreviewSelection(selection);
    gesture.previewSelection = selection;
    const objectPreviewIds = selectedDocumentPreviewObjectIds(selection);
    const previewTransform = selectionGestureTransform(gesture);
    const objectPreviewed = applyDocumentObjectOnlyPreviewTransform(
      selection,
      previewTransform,
    );
    if (!changed) {
      recordBackendMovePreviewTiming({
        updateMs: updatedAt - started,
        renderTargetsMs: 0,
        patchMs: performance.now() - updatedAt,
        totalMs: performance.now() - started,
        primitiveCount: 0,
        nodeCount: targets.nodeIds.size,
        bondCount: targets.bondIds.size,
        objectPreviewed,
        objectPreviewIds,
        selection: previewSelectionDebugSummary(selection),
        previewTransform,
        patched: objectPreviewed,
        changed: false,
      });
      return objectPreviewed;
    }
    const primitives = parseEngineJson(state.editorEngine.renderTargetsJson?.(JSON.stringify({
      nodes: commandTargetSet(targets.nodeIds),
      bonds: commandTargetSet(targets.bondIds),
    })) || "[]", []);
    const renderedAt = performance.now();
    const patched = renderDocumentPrimitivePatch(primitives, targets.nodeIds, targets.bondIds);
    hideDocumentDiagnosticsForPreview();
    const patchedAt = performance.now();
    recordBackendMovePreviewTiming({
      updateMs: updatedAt - started,
      renderTargetsMs: renderedAt - updatedAt,
      patchMs: patchedAt - renderedAt,
      totalMs: patchedAt - started,
      primitiveCount: primitives.length,
      nodeCount: targets.nodeIds.size,
      bondCount: targets.bondIds.size,
      objectPreviewed,
      objectPreviewIds,
      selection: previewSelectionDebugSummary(selection),
      previewTransform,
      patched: patched || objectPreviewed,
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
    if (["move", "resize", "rotate"].includes(activeSelectionGesture()?.kind) && !activeSelectionGesture()?.dragged) {
      return false;
    }
    return ["move", "resize", "rotate", "arrow-endpoint", "arrow-curve", "shape-resize"]
      .includes(activeSelectionGesture()?.kind);
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
  
  function collectCurrentDocumentSceneObjects(documentData = state.currentDocument) {
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
    for (const object of documentData?.objects || []) {
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
      ...(selection.textObjects || selection.text_objects || []),
      ...(selection.arrowObjects || selection.arrow_objects || []),
    ];
  }
  
  function previewSelectionDebugSummary(selection) {
    return {
      nodes: [...(selection?.nodes || [])],
      bonds: [...(selection?.bonds || [])],
      labelNodes: [...(selection?.labelNodes || selection?.label_nodes || [])],
      textObjects: [...(selection?.textObjects || selection?.text_objects || [])],
      arrowObjects: [...(selection?.arrowObjects || selection?.arrow_objects || [])],
    };
  }
  
  function selectedStructurePreviewNodeIds(selection) {
    const nodeIds = new Set([
      ...(selection?.nodes || []),
      ...(selection?.labelNodes || selection?.label_nodes || []),
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
    if (activeSelectionGesture()?.previewNodeIds) {
      return activeSelectionGesture().previewNodeIds;
    }
    const nodeIds = selectedStructurePreviewNodeIds(selection);
    if (activeSelectionGesture()) {
      activeSelectionGesture().previewNodeIds = nodeIds;
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
    if (activeSelectionGesture()?.previewPartialBondState) {
      return activeSelectionGesture().previewPartialBondState;
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
    if (activeSelectionGesture()) {
      activeSelectionGesture().previewPartialBondState = previewState;
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
    const directElements = [...documentLayer.querySelectorAll(`[data-object-id="${escapedId}"]`)];
    if (directElements.length) {
      return directElements;
    }
    const object = currentDocumentSceneObjectMap().get(objectId);
    if (!object?.children?.length) {
      return [];
    }
    const descendantElements = new Set();
    const visit = (child) => {
      if (child?.id) {
        for (const element of documentObjectElements(child.id)) {
          descendantElements.add(element);
        }
      }
      for (const grandchild of child?.children || []) {
        visit(grandchild);
      }
    };
    for (const child of object.children) {
      visit(child);
    }
    return [...descendantElements];
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
  
  function hideDocumentDiagnosticsForPreview() {
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    if (!documentLayer) {
      return;
    }
    for (const element of documentLayer.querySelectorAll(".document-diagnostic-marker")) {
      if (element.dataset.previewDiagnosticBaseVisibility === undefined) {
        element.dataset.previewDiagnosticBaseVisibility = element.style.visibility || "";
      }
      element.style.visibility = "hidden";
      activeDocumentDiagnosticPreviewHiddenElements.add(element);
    }
  }
  
  function restoreDocumentDiagnosticsForPreview() {
    for (const element of activeDocumentDiagnosticPreviewHiddenElements) {
      if (element.dataset.previewDiagnosticBaseVisibility !== undefined) {
        element.style.visibility = element.dataset.previewDiagnosticBaseVisibility;
        delete element.dataset.previewDiagnosticBaseVisibility;
      } else {
        element.style.visibility = "";
      }
    }
    activeDocumentDiagnosticPreviewHiddenElements = new Set();
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
    return ["arrow-endpoint", "arrow-curve", "shape-resize"].includes(activeSelectionGesture()?.kind);
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
    restoreDocumentDiagnosticsForPreview();
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
    restoreDocumentDiagnosticsForPreview();
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
    const transform = selectionGestureTransform(activeSelectionGesture());
    if (!transform) {
      clearDocumentObjectPreviewTransform();
      return false;
    }
    hideDocumentDiagnosticsForPreview();
    const selection = freshestPreviewSelection(activeSelectionGesture().previewSelection);
    activeSelectionGesture().previewSelection = selection;
    const nodeIds = activeStructurePreviewNodeIds(selection);
    const selectedObjectIds = selectedDocumentPreviewObjectIds(selection);
    const partialBondState = partialMovingStructureBondPreviewState(selection, nodeIds);
    const hasPartialBonds = partialBondState.partialBonds.length > 0;
    const partialBondDelta = hasPartialBonds ? selectionGestureDelta(activeSelectionGesture) : null;
    if (hasPartialBonds && !partialBondDelta) {
      clearDocumentObjectPreviewTransform();
      return false;
    }
    const documentLayer = viewerSvg.querySelector('[data-layer="document-content"]');
    if (activeSelectionGesture().previewUsesLayer && !hasPartialBonds && !selectedObjectIds.length) {
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
    if (!hasPartialBonds && !selectedObjectIds.length && selectionCoversRenderedDocument(selection)) {
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
      activeSelectionGesture().previewUsesLayer = true;
      return true;
    }
    const hasCachedObjectIds = Array.isArray(activeSelectionGesture().previewObjectIds);
    const hasCachedPrimitiveElements = Array.isArray(activeSelectionGesture().previewPrimitiveElements);
    const freshObjectIds = selectedObjectIds;
    const freshPrimitiveElements = selectedDocumentPreviewPrimitiveElements(selection);
    const objectIds = activeSelectionGesture().previewObjectIds
      ? [...new Set([...activeSelectionGesture().previewObjectIds, ...freshObjectIds])]
      : freshObjectIds;
    const primitiveElements = activeSelectionGesture().previewPrimitiveElements
      ? [...new Set([...activeSelectionGesture().previewPrimitiveElements, ...freshPrimitiveElements])]
      : freshPrimitiveElements;
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
    activeSelectionGesture().previewObjectIds = objectIds;
    activeSelectionGesture().previewPrimitiveElements = primitiveElements;
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
      activeSelectionGesture().previewUsesLayer = true;
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
    const appliedElements = new Set();
    for (const elements of nextObjectElements.values()) {
      for (const element of elements) {
        if (appliedElements.has(element)) {
          continue;
        }
        appliedElements.add(element);
        applyDocumentPreviewElementTransform(element, transform);
      }
    }
    for (const element of nextPrimitiveElements) {
      if (appliedElements.has(element)) {
        continue;
      }
      appliedElements.add(element);
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
  
  

  return {
    activeDocumentPreviewTransform: () => activeDocumentPreviewTransform,
    resetDocumentRenderState,
    sceneObjectType,
    currentDocumentSceneObjectMap,
    currentDocumentSceneObjectParentMap,
    currentDocumentObjectIdsInPaintOrder,
    targetIdsFromCommandResult,
    renderDocumentChange,
    renderDocumentPrimitiveChange,
    rebuildDocumentPrimitiveIndex,
    ensureDocumentObjectDomForCommandResult,
    selectionNeedsBackendMovePreview,
    applyBackendSelectionMovePreview,
    syncViewerStats,
    isDocumentPreviewPrimitive,
    activeGestureUsesDocumentPreview,
    activeGestureUsesObjectEditPreview,
    primitiveObjectId,
    primitiveNodeId,
    primitiveBondId,
    collectCurrentDocumentSceneObjects,
    currentDocumentMoleculeTopology,
    syncObjectEditPreviewHiddenElements,
    clearDocumentObjectPreviewTransform,
    commitDocumentObjectPreviewTransform,
    canCommitDocumentObjectPreviewTransform,
    applyDocumentObjectPreviewTransform,
    hideDocumentDiagnosticsForPreview,
  };
}
