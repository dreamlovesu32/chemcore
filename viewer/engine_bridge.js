export function parseEngineJson(json, defaultValue = null) {
  if (json === undefined || json === null || json === "") {
    return defaultValue;
  }
  try {
    return inflateChemSemaDocument(JSON.parse(json));
  } catch (error) {
    console.warn("Failed to parse chemsema engine JSON", error);
    return defaultValue;
  }
}

// The persisted CCJS v0.2 snapshot is normalized. The current editor renderer
// still consumes a nested scene view, so build that disposable view at the
// engine boundary without changing or writing the canonical document.
export function inflateChemSemaDocument(value) {
  if (
    !value
    || typeof value !== "object"
    || value.format?.name !== "chemsema"
    || value.format?.version !== "0.2"
    || !Array.isArray(value.entities?.scene)
    || !Array.isArray(value.hierarchy?.roots)
  ) {
    return value;
  }
  const entities = new Map(value.entities.scene.map((object) => [object?.id, object]));
  const children = value.hierarchy.children || {};
  const visiting = new Set();
  const build = (id) => {
    const source = entities.get(id);
    if (!source || visiting.has(id)) {
      return null;
    }
    visiting.add(id);
    const object = {
      ...source,
      children: (children[id] || []).map(build).filter(Boolean),
    };
    visiting.delete(id);
    return object;
  };
  return {
    ...value,
    objects: value.hierarchy.roots.map(build).filter(Boolean),
    links: Array.isArray(value.relations) ? value.relations : [],
  };
}

export function setChemSemaRuntimeRevision(document, revision) {
  if (!document || !Number.isFinite(Number(revision))) return;
  Object.defineProperty(document, "__runtimeRevision", {
    value: Number(revision),
    writable: true,
    configurable: true,
    enumerable: false,
  });
}

export function canonicalChemSemaDocumentForSave(document) {
  if (document?.format?.version !== "0.2" || !document.entities || !document.hierarchy) {
    return document;
  }
  const { objects: _objects, links: _links, ...canonical } = document;
  return canonical;
}

export function applyChemSemaDocumentPatch(document, patch) {
  if (!document || !patch || !Number.isFinite(Number(patch.revision))) {
    return false;
  }
  const currentRevision = Number(document.__runtimeRevision);
  if (Number.isFinite(currentRevision) && currentRevision !== Number(patch.beforeRevision)) {
    return false;
  }
  const objectMap = new Map();
  const collect = (objects) => {
    for (const object of objects || []) {
      if (!object?.id) continue;
      objectMap.set(object.id, object);
      collect(object.children);
    }
  };
  collect(document.objects);

  const deleted = new Set(patch.deletedEntityIds || []);
  for (const id of deleted) {
    objectMap.delete(id);
  }
  for (const record of patch.upsertEntities || []) {
    const entity = record?.entity;
    if (!entity?.id) continue;
    const previous = objectMap.get(entity.id);
    objectMap.set(entity.id, {
      ...previous,
      ...entity,
      children: previous?.children || [],
    });
  }
  for (const record of patch.upsertEntities || []) {
    if (!record?.entity?.id) continue;
    const entity = objectMap.get(record.entity.id);
    entity.children = (record.childIds || []).map((id) => objectMap.get(id)).filter(Boolean);
  }

  const removeMembership = (objects, ids) => (objects || [])
    .filter((object) => !ids.has(object?.id))
    .map((object) => {
      object.children = removeMembership(object.children, ids);
      return object;
    });
  const movedIds = new Set((patch.upsertEntities || []).map((record) => record?.entity?.id).filter(Boolean));
  let roots = removeMembership(document.objects, new Set([...deleted, ...movedIds]));
  for (const record of patch.upsertEntities || []) {
    const entity = objectMap.get(record?.entity?.id);
    if (!entity) continue;
    if (record.parentId) {
      const parent = objectMap.get(record.parentId);
      if (parent && !(parent.children || []).some((child) => child.id === entity.id)) {
        parent.children = [...(parent.children || []), entity];
      }
    } else if (!roots.some((root) => root.id === entity.id)) {
      roots.push(entity);
    }
  }
  if (Array.isArray(patch.hierarchyRoots)) {
    roots = patch.hierarchyRoots.map((id) => objectMap.get(id)).filter(Boolean);
  }
  document.objects = roots;

  const scene = [];
  const hierarchyChildren = {};
  const flatten = (object) => {
    if (!object?.id) return;
    const { children = [], ...entity } = object;
    scene.push(entity);
    if (children.length) {
      hierarchyChildren[object.id] = children.map((child) => child.id);
      children.forEach(flatten);
    }
  };
  roots.forEach(flatten);
  document.entities = { ...(document.entities || {}), scene };
  document.hierarchy = {
    roots: roots.map((root) => root.id),
    children: hierarchyChildren,
  };

  document.resources ||= {};
  Object.assign(document.resources, patch.upsertResources || {});
  document.styles ||= {};
  Object.assign(document.styles, patch.upsertStyles || {});
  for (const id of patch.deletedStyleIds || []) {
    delete document.styles[id];
  }

  const relationScope = new Set(patch.relationScopeEntityIds || []);
  document.links = (document.links || []).filter((relation) => !(
    relation.endpoints || []
  ).some((endpoint) => relationScope.has(endpoint.entityId)));
  document.links.push(...(patch.relations || []));
  document.relations = document.links;
  if (patch.logicalObjects !== undefined) document.logicalObjects = patch.logicalObjects;
  if (patch.reactionSchemes !== undefined) document.reactionSchemes = patch.reactionSchemes;
  if (patch.chemicalProperties !== undefined) document.chemicalProperties = patch.chemicalProperties;
  if (patch.orders !== undefined) document.orders = patch.orders;
  setChemSemaRuntimeRevision(document, patch.revision);
  return true;
}

export function renderListFromEngine(engine) {
  if (!engine?.renderListJson) {
    return [];
  }
  return parseEngineJson(engine.renderListJson(), []) || [];
}

export function interactionRenderListFromEngine(engine) {
  if (engine?.interactionRenderListJson) {
    return parseEngineJson(engine.interactionRenderListJson(), []) || [];
  }
  return renderListFromEngine(engine);
}

export function renderBoundsFromEngine(engine, scope = "all") {
  if (!engine?.renderBoundsJson) {
    return null;
  }
  return parseEngineJson(engine.renderBoundsJson(scope), null);
}

export function primitivesForObject(renderList, objectId) {
  return (renderList || []).filter((primitive) => (
    primitive?.objectId || primitive?.object_id || null
  ) === objectId);
}
