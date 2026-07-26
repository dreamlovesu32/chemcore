export const ENGINE_DRAG_PREVIEW_TOOLS = Object.freeze([
  "bond",
  "arrow",
  "bracket",
  "symbol",
  "shape",
  "table",
  "tlc-plate",
  "biodraw",
  "orbital",
  "templates",
  "chain",
]);

export const SELECTION_BOX_MOVE_TOOLS = Object.freeze([
  "bond",
  "arrow",
  "bracket",
  "symbol",
  "element",
  "shape",
  "table",
  "tlc-plate",
  "biodraw",
  "orbital",
  "templates",
  "chain",
]);

export function engineToolForUiTool(tool) {
  return tool === "chain" ? "templates" : tool || "";
}

export function engineToolForEditorState(editorState = {}) {
  return editorState.elementPlacementActive
    ? "element"
    : engineToolForUiTool(editorState.activeTool);
}

export function engineTemplateForEditorState(editorState = {}) {
  return editorState.activeTool === "chain" ? "chain" : editorState.template;
}

export function toolUsesEngineDragPreview(tool) {
  return ENGINE_DRAG_PREVIEW_TOOLS.includes(tool);
}

export function toolSupportsSelectionBoxMove(tool) {
  return SELECTION_BOX_MOVE_TOOLS.includes(tool);
}
