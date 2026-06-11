export function parseEngineJson(json, fallback = null) {
  try {
    return JSON.parse(json);
  } catch (error) {
    console.warn("Failed to parse chemcore engine JSON", error);
    return fallback;
  }
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
