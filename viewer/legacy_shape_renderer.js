import { ensureSvgDefs, makeSvgNode } from "./render_support.js";
import {
  dashArrayValue,
  DEFAULT_SHAPE_STROKE_WIDTH,
} from "./legacy_render_shared.js";

export function renderShapeObject(svgRoot, object, styles) {
  const [tx, ty] = object.transform.translate;
  const style = styles?.[object.styleRef] || {};
  const [, , width, height] = object.payload.bbox || [0, 0, 0, 0];
  const gradient = style.fillGradient;
  const kind = object.payload.kind || "rect";
  const shapeStrokeWidth = style.strokeWidth || DEFAULT_SHAPE_STROKE_WIDTH;
  const shapeDashArray = dashArrayValue(style.dashArray);
  if (kind === "circle" || kind === "ellipse") {
    const center = object.payload.center;
    const major = object.payload.majorAxisEnd;
    const minor = object.payload.minorAxisEnd;
    if (!center || !major || !minor) {
      return;
    }
    const rx = Math.hypot(major[0] - center[0], major[1] - center[1]);
    const ry = Math.hypot(minor[0] - center[0], minor[1] - center[1]);
    const rotate = Math.atan2(major[1] - center[1], major[0] - center[0]) * 180 / Math.PI;
    const attrs = {
      cx: center[0],
      cy: center[1],
      rx,
      ry,
      fill: style.fill || "none",
      stroke: style.stroke || "none",
      "stroke-width": shapeStrokeWidth,
      transform: Math.abs(rotate) > 0.0001 ? `rotate(${rotate} ${center[0]} ${center[1]})` : undefined,
      "stroke-dasharray": shapeDashArray,
    };
    svgRoot.appendChild(makeSvgNode("ellipse", attrs));
    return;
  }
  const attrs = {
    x: tx,
    y: ty,
    width,
    height,
    fill: style.fill || "none",
    stroke: style.stroke || "none",
    "stroke-width": shapeStrokeWidth,
    "stroke-dasharray": shapeDashArray,
  };
  if (gradient?.stops?.length) {
    const defs = ensureSvgDefs(svgRoot);
    const gradientId = `grad-${object.id}`;
    const linearGradient = makeSvgNode("linearGradient", {
      id: gradientId,
      x1: gradient.x1 || "0%",
      y1: gradient.y1 || "0%",
      x2: gradient.x2 || "0%",
      y2: gradient.y2 || "100%",
    });
    for (const stop of gradient.stops) {
      linearGradient.appendChild(makeSvgNode("stop", {
        offset: stop.offset,
        "stop-color": stop.color,
      }));
    }
    defs.appendChild(linearGradient);
    attrs.fill = `url(#${gradientId})`;
  }
  if (object.payload.kind === "roundRect") {
    attrs.rx = object.payload.cornerRadius || 0;
    attrs.ry = object.payload.cornerRadius || 0;
  }
  svgRoot.appendChild(makeSvgNode("rect", attrs));
  if (object.payload.kind === "crossTable") {
    renderLegacyCrossTable(svgRoot, tx, ty, width, height, style.stroke || "#000000", shapeStrokeWidth, shapeDashArray);
  } else if (object.payload.kind === "tlcPlate") {
    renderLegacyTlcPlate(svgRoot, object, tx, ty, width, height, style, shapeStrokeWidth, shapeDashArray);
  }
}

function renderLegacyCrossTable(svgRoot, tx, ty, width, height, stroke, strokeWidth, dashArray) {
  svgRoot.appendChild(makeSvgNode("path", {
    d: `M${tx + width * 0.5} ${ty} L${tx + width * 0.5} ${ty + height} M${tx} ${ty + height * 0.5} L${tx + width} ${ty + height * 0.5}`,
    fill: "none",
    stroke,
    "stroke-width": strokeWidth,
    "stroke-dasharray": dashArray,
  }));
}

function renderLegacyTlcPlate(svgRoot, object, tx, ty, width, height, style, strokeWidth, shapeDashArray) {
  const stroke = style.stroke || "#000000";
  const lineCap = style.lineCap || "butt";
  const lineJoin = style.lineJoin || "miter";
  const fallbackDashArray = style.dashArray?.length
    ? shapeDashArray
    : Number.isFinite(Number(object.payload.dashSpacing))
      ? String(Number(object.payload.dashSpacing))
      : undefined;
  const originFraction = Number(object.payload.originFraction ?? 0.1);
  const solventFraction = Number(object.payload.solventFrontFraction ?? 0.1);
  const originY = ty + height * (1 - originFraction);
  const solventY = ty + height * solventFraction;
  svgRoot.appendChild(makeSvgNode("path", {
    d: `M${tx} ${originY}L${tx + width} ${originY} M${tx} ${solventY}L${tx + width} ${solventY}`,
    fill: "none",
    stroke,
    "stroke-width": strokeWidth,
    "stroke-dasharray": fallbackDashArray,
    "stroke-linecap": lineCap,
    "stroke-linejoin": lineJoin,
  }));
  for (const lane of object.payload.lanes || []) {
    const offset = Number(lane.offset ?? 0.5);
    const laneX = tx + width * offset;
    svgRoot.appendChild(makeSvgNode("path", {
      d: `M${laneX} ${originY - 3}L${laneX} ${originY + 3}`,
      fill: "none",
      stroke,
      "stroke-width": strokeWidth,
    }));
    for (const spot of lane.spots || []) {
      const rf = Number(spot.rf ?? 0.15);
      const spotY = originY - (originY - solventY) * rf;
      const spotDiameter = Number(spot.width ?? spot.height ?? 0);
      const spotRadius = spotDiameter > 0
        ? Math.max(2, Math.min(10, spotDiameter * 0.5))
        : Math.max(2, Math.min(5, Math.min(width, height) * 0.015));
      svgRoot.appendChild(makeSvgNode("circle", {
        cx: laneX,
        cy: spotY,
        r: spotRadius,
        fill: stroke,
        stroke,
        "stroke-width": 0,
      }));
    }
  }
}
