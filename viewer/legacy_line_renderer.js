import { makeSvgNode } from "./render_support.js";
import {
  DEFAULT_LINE_STROKE_WIDTH,
  strokeStyleAttrs,
} from "./legacy_render_shared.js";

export function renderLineObject(svgRoot, object, styles) {
  const points = object.payload.points || [];
  if (points.length < 2) {
    return;
  }

  const style = styles?.[object.styleRef] || {};
  const {
    stroke,
    strokeWidth,
    lineCap,
    lineJoin,
    dashArray,
  } = strokeStyleAttrs(style, DEFAULT_LINE_STROKE_WIDTH);
  const arrowHead = object.payload.arrowHead || null;

  const pathValue = points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point[0]} ${point[1]}`)
    .join(" ");

  const path = makeSvgNode("path", {
    d: pathValue,
    fill: "none",
    stroke,
    "stroke-width": strokeWidth,
    "stroke-linecap": lineCap,
    "stroke-linejoin": lineJoin,
    "stroke-dasharray": dashArray,
  });

  if (object.payload.head === "end") {
    const from = points[points.length - 2];
    const to = points[points.length - 1];
    if (arrowHead?.length > 0) {
      const shaftEnd = arrowShaftEnd(from, to, arrowHead, strokeWidth);
      path.setAttribute("d", points
        .slice(0, -2)
        .map((point, index) => `${index === 0 ? "M" : "L"} ${point[0]} ${point[1]}`)
        .concat(`M ${from[0]} ${from[1]} L ${shaftEnd[0]} ${shaftEnd[1]}`)
        .join(" "));
    }
    svgRoot.appendChild(path);
    renderArrowHead(svgRoot, from, to, arrowHead, stroke, strokeWidth);
    return;
  }

  svgRoot.appendChild(path);
}

function arrowShaftEnd(from, to, arrowHead, strokeWidth) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const scale = Number(strokeWidth) > 0 ? Number(strokeWidth) : DEFAULT_LINE_STROKE_WIDTH;
  const headLength = (arrowHead?.length || 10) * scale;
  const notchLength = Math.min(Math.max(0, (arrowHead?.centerLength || (arrowHead?.length || 10) * 0.875) * scale), headLength);
  const centerLength = Math.max(0, Math.min(notchLength, length * 0.8));
  return [to[0] - ux * centerLength, to[1] - uy * centerLength];
}

function renderArrowHead(svgRoot, from, to, arrowHead, stroke, strokeWidth) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const nx = -uy;
  const ny = ux;
  const scale = Number(strokeWidth) > 0 ? Number(strokeWidth) : DEFAULT_LINE_STROKE_WIDTH;
  const sourceLength = arrowHead?.length || 10;
  const sourceWidth = arrowHead?.width || sourceLength * 0.25;
  const headLength = sourceLength * scale;
  const headHalfWidth = Math.max(0, sourceWidth * scale) + 0.05;
  const notchLength = Math.min(Math.max(0, (arrowHead?.centerLength || sourceLength * 0.875) * scale), headLength);

  const p1 = [to[0], to[1]];
  const p2 = [to[0] - ux * headLength + nx * headHalfWidth, to[1] - uy * headLength + ny * headHalfWidth];
  const p3 = [to[0] - ux * headLength - nx * headHalfWidth, to[1] - uy * headLength - ny * headHalfWidth];
  const notch = [to[0] - ux * notchLength, to[1] - uy * notchLength];
  const useNotch = String(arrowHead?.head || "").toLowerCase() === "full" && notchLength < headLength - 0.2;
  const points = useNotch
    ? `${p1[0]},${p1[1]} ${p2[0]},${p2[1]} ${notch[0]},${notch[1]} ${p3[0]},${p3[1]}`
    : `${p1[0]},${p1[1]} ${p2[0]},${p2[1]} ${p3[0]},${p3[1]}`;

  svgRoot.appendChild(
    makeSvgNode("polygon", {
      points,
      fill: stroke || "#222222",
    }),
  );
}
