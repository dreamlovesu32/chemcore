export const SELECTION_EDGE_HANDLE_PAD_SCREEN_PX = 14;
export const SELECTION_ROTATE_HANDLE_PAD_SCREEN_PX = 18;
export const SELECTION_POST_COMMIT_HOVER_BLOCK_SCREEN_PX = 4;
export const TLC_SPOT_DRAG_THRESHOLD_SCREEN_PX = 1.5;
export const SELECTION_DRAG_THRESHOLD_SCREEN_PX = 3;
export const SELECTION_FREEHAND_POINT_SPACING_SCREEN_PX = 2;
export const BRACKET_LABEL_OPEN_DRAG_THRESHOLD_SCREEN_PX = 4;
export const DOCUMENT_BOUNDS_HIT_PAD_SCREEN_PX = 8;

export function selectionHandleZoneContainsPoint({
  point,
  bounds,
  pointDistance,
  toWorld,
  selectedContentHitContainsPoint = null,
}) {
  if (!bounds) {
    return true;
  }
  if (selectedContentHitContainsPoint?.(point)) {
    return false;
  }
  const edgePad = toWorld(SELECTION_EDGE_HANDLE_PAD_SCREEN_PX);
  const rotatePad = toWorld(SELECTION_ROTATE_HANDLE_PAD_SCREEN_PX);
  const width = Math.max(0, Number(bounds.maxX || 0) - Number(bounds.minX || 0));
  const height = Math.max(0, Number(bounds.maxY || 0) - Number(bounds.minY || 0));
  const strictlyInsideBounds = point.x > bounds.minX
    && point.x < bounds.maxX
    && point.y > bounds.minY
    && point.y < bounds.maxY;
  if (strictlyInsideBounds && (width <= edgePad * 4 || height <= edgePad * 4)) {
    return false;
  }
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
    y: bounds.minY - toWorld(SELECTION_ROTATE_HANDLE_PAD_SCREEN_PX),
  };
  return pointDistance(point, rotateHandle) <= rotatePad;
}
