import { cssPxToCm } from "./units.js";

export const DEFAULT_TEXT_FONT_SIZE = 10;
export const DEFAULT_TEXT_LINE_HEIGHT = cssPxToCm(10.5);
export const DEFAULT_LINE_STROKE_WIDTH = cssPxToCm(1.6);
export const DEFAULT_TEXT_WRAP_WIDTH = cssPxToCm(160);
export const DEFAULT_SHAPE_STROKE_WIDTH = cssPxToCm(1);

export function dashArrayValue(dashArray) {
  return dashArray?.length ? dashArray.join(" ") : undefined;
}

export function strokeStyleAttrs(style, fallbackStrokeWidth = DEFAULT_LINE_STROKE_WIDTH) {
  return {
    stroke: style.stroke || "#222222",
    strokeWidth: style.strokeWidth || fallbackStrokeWidth,
    lineCap: style.lineCap || "round",
    lineJoin: style.lineJoin || "round",
    dashArray: dashArrayValue(style.dashArray),
  };
}
