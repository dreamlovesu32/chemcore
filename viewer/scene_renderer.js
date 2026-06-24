import { makeSvgNode } from "./render_support.js";
import { renderCorePrimitive } from "./primitive_dom_renderer.js";

function buildRenderList(documentData) {
  return [...documentData.objects].sort((a, b) => {
    if (a.zIndex !== b.zIndex) {
      return a.zIndex - b.zIndex;
    }
    return a.id.localeCompare(b.id);
  });
}

function sortedSceneChildren(children = []) {
  return [...children].sort((a, b) => {
    if (a.zIndex !== b.zIndex) {
      return a.zIndex - b.zIndex;
    }
    return a.id.localeCompare(b.id);
  });
}

export function createSceneRenderer(options) {
  function shouldRenderSceneObject(object) {
    if (!object.visible) {
      return false;
    }
    if (object.type === "molecule" && options.toggleMolecules?.() === false) {
      return false;
    }
    if (object.type === "line" && options.toggleLines?.() === false) {
      return false;
    }
    if (object.type === "text" && options.toggleTexts?.() === false) {
      return false;
    }
    if (options.labelDebugMode && object.type !== "molecule" && object.type !== "group") {
      return false;
    }
    return true;
  }

  function renderObjectCorePrimitives(objectLayer, objectId) {
    const corePrimitives = options.corePrimitivesForObject(objectId);
    if (!corePrimitives.length) {
      return false;
    }
    objectLayer.setAttribute("data-renderer", "core");
    corePrimitives.forEach((primitive) => {
      const renderIndex = Number.isInteger(primitive.__renderIndex) ? primitive.__renderIndex : undefined;
      renderCorePrimitive(objectLayer, primitive, {
        ...options.corePrimitiveRenderOptions(),
        renderIndex,
      });
    });
    return true;
  }

  function collectRenderableObjects(documentData) {
    const objects = new Map();
    const visit = (object, ancestorsRenderable = true) => {
      if (!object) {
        return;
      }
      const renderable = ancestorsRenderable && shouldRenderSceneObject(object);
      objects.set(object.id, { object, renderable });
      for (const child of object.children || []) {
        visit(child, renderable);
      }
    };
    for (const object of documentData?.objects || []) {
      visit(object, true);
    }
    return objects;
  }

  function shouldRenderCorePrimitive(primitive, renderableObjects) {
    const objectId = primitive?.objectId || primitive?.object_id || null;
    if (!objectId) {
      return true;
    }
    const entry = renderableObjects.get(objectId);
    return entry ? entry.renderable : true;
  }

  function renderCorePrimitiveList(parentLayer, documentData) {
    const corePrimitives = options.coreRenderList?.() || [];
    if (!corePrimitives.length) {
      return false;
    }
    parentLayer.setAttribute("data-renderer", "core-global");
    const renderableObjects = collectRenderableObjects(documentData);
    for (let index = 0; index < corePrimitives.length; index += 1) {
      const primitive = corePrimitives[index];
      if (shouldRenderCorePrimitive(primitive, renderableObjects)) {
        renderCorePrimitive(parentLayer, primitive, {
          ...options.corePrimitiveRenderOptions(),
          renderIndex: index,
        });
      }
    }
    return true;
  }

  function renderSceneObject(parentLayer, object, documentData) {
    if (!shouldRenderSceneObject(object)) {
      return;
    }

    const objectLayer = makeSvgNode("g", {
      "data-object-id": object.id,
      "data-object-type": object.type,
    });

    if (object.type === "group") {
      for (const child of sortedSceneChildren(object.children || [])) {
        renderSceneObject(objectLayer, child, documentData);
      }
    } else if (object.type === "molecule") {
      renderObjectCorePrimitives(objectLayer, object.id);
    } else if (
      object.type === "shape"
      || object.type === "line"
      || object.type === "text"
      || object.type === "bracket"
      || object.type === "symbol"
    ) {
      renderObjectCorePrimitives(objectLayer, object.id);
    }

    if (objectLayer.childNodes.length) {
      parentLayer.appendChild(objectLayer);
    }
  }

  return {
    buildRenderList,
    renderCorePrimitiveList,
    renderSceneObject,
  };
}
