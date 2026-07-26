import {
  displayLabelFontFamily,
  ensureSvgDefs,
  fontStyleForRun,
  fontWeightForRun,
  isSubscriptRun,
  isSuperscriptRun,
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";
import { editorScriptScale, editorSvgScriptBaselineShift } from "./text_metrics.js";
import { cssPxToPt } from "./units.js";

const DEFAULT_TEXT_FONT_SIZE = 10;
const BOND_STROKE = 1.0;
const CHEMDRAW_INK = "#000000";

let renderClipPathId = 0;

export function primitiveStrokeWidthValue(primitive, defaultValue = 0) {
  const strokeWidth = primitive?.strokeWidth ?? primitive?.stroke_width;
  const numeric = Number(strokeWidth);
  return Number.isFinite(numeric) ? numeric : defaultValue;
}

function primitiveIdentityAttrs(primitive, options = {}) {
  return {
    "data-object-id": primitive.objectId || primitive.object_id || undefined,
    "data-node-id": primitive.nodeId || primitive.node_id || undefined,
    "data-bond-id": primitive.bondId || primitive.bond_id || undefined,
    "data-render-index": Number.isInteger(options.renderIndex) ? String(options.renderIndex) : undefined,
  };
}

function primitiveInteractionAttrs(primitive) {
  return primitive?.role === "document-molecular-color"
    ? { "pointer-events": "none" }
    : {};
}

function distanceBetweenPoints(a, b) {
  const dx = Number(a?.x || 0) - Number(b?.x || 0);
  const dy = Number(a?.y || 0) - Number(b?.y || 0);
  return Math.hypot(dx, dy);
}

function midpoint(a, b) {
  return {
    x: (Number(a?.x || 0) + Number(b?.x || 0)) * 0.5,
    y: (Number(a?.y || 0) + Number(b?.y || 0)) * 0.5,
  };
}

function dot(ax, ay, bx, by) {
  return ax * bx + ay * by;
}

function rectangularBondLineFromPolygon(primitive) {
  const points = primitive?.points || [];
  if ((primitive.role || primitive.role_name) !== "document-bond" || points.length !== 4) {
    return null;
  }
  const [p0, p1, p2, p3] = points;
  const start = midpoint(p0, p3);
  const end = midpoint(p1, p2);
  const axisX = end.x - start.x;
  const axisY = end.y - start.y;
  const length = Math.hypot(axisX, axisY);
  const widthStart = distanceBetweenPoints(p0, p3);
  const widthEnd = distanceBetweenPoints(p1, p2);
  const width = (widthStart + widthEnd) * 0.5;
  if (!Number.isFinite(length) || !Number.isFinite(width) || length <= width * 1.25 || width <= 0) {
    return null;
  }
  const tolerance = Math.max(0.01, width * 0.03);
  if (Math.abs(widthStart - widthEnd) > tolerance) {
    return null;
  }
  const normalStartX = Number(p0.x || 0) - Number(p3.x || 0);
  const normalStartY = Number(p0.y || 0) - Number(p3.y || 0);
  const normalEndX = Number(p1.x || 0) - Number(p2.x || 0);
  const normalEndY = Number(p1.y || 0) - Number(p2.y || 0);
  if (Math.hypot(normalStartX - normalEndX, normalStartY - normalEndY) > tolerance) {
    return null;
  }
  const perpendicularTolerance = Math.max(0.02, length * width * 0.01);
  if (Math.abs(dot(axisX, axisY, normalStartX, normalStartY)) > perpendicularTolerance) {
    return null;
  }
  return { start, end, width };
}

export function renderCorePrimitive(svgRoot, primitive, options = {}) {
  if (options.shouldHide?.(primitive)) {
    return;
  }
  if (primitive.kind === "line" && primitive.from && primitive.to) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      x1: primitive.from.x,
      y1: primitive.from.y,
      x2: primitive.to.x,
      y2: primitive.to.y,
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
    };
    if ((primitive.dashArray || primitive.dash_array)?.length) {
      attrs["stroke-dasharray"] = (primitive.dashArray || primitive.dash_array).join(" ");
    }
    if (primitive.role === "document-bond") {
      attrs.class = "mol-bond-stroked";
    }
    svgRoot.appendChild(makeSvgNode("line", attrs));
    return;
  }
  if (primitive.kind === "polyline" && Array.isArray(primitive.points)) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
      fill: "none",
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "stroke-dasharray": (primitive.dashArray || primitive.dash_array)?.join(" ") || undefined,
      "stroke-linecap": primitive.lineCap || primitive.line_cap || undefined,
      "stroke-linejoin": primitive.lineJoin || primitive.line_join || undefined,
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
    };
    if (primitive.role === "document-bond") {
      attrs.class = "mol-bond-stroked";
    }
    svgRoot.appendChild(makeSvgNode("polyline", attrs));
    return;
  }
  if (primitive.kind === "path" && primitive.d) {
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      d: primitive.d,
      fill: "none",
      stroke: primitive.stroke || CHEMDRAW_INK,
      "stroke-width": strokeWidth,
      "stroke-dasharray": (primitive.dashArray || primitive.dash_array)?.join(" ") || undefined,
      "stroke-linecap": primitive.lineCap || primitive.line_cap || undefined,
      "stroke-linejoin": primitive.lineJoin || primitive.line_join || undefined,
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
    };
    if (primitive.role === "document-bond") {
      attrs.class = "mol-bond-stroked";
    }
    const transform = primitiveRotateTransform(primitive);
    if (transform) {
      attrs.transform = transform;
    }
    svgRoot.appendChild(makeSvgNode("path", attrs));
    return;
  }
  if (primitive.kind === "filled-path" && primitive.d) {
    const attrs = {
      d: primitive.d,
      fill: primitive.fill || CHEMDRAW_INK,
      "fill-rule": primitive.fillRule || primitive.fill_rule || undefined,
      stroke: "none",
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
    };
    const clipPathD = primitive.clipPathD || primitive.clip_path_d;
    if (clipPathD) {
      const defs = ensureSvgDefs(svgRoot);
      const clipId = `clip-core-${primitive.objectId || "shape"}-${renderClipPathId++}`;
      const clipPath = makeSvgNode("clipPath", { id: clipId });
      clipPath.appendChild(makeSvgNode("path", {
        d: clipPathD,
        "clip-rule": primitive.clipRule || primitive.clip_rule || "nonzero",
      }));
      defs.appendChild(clipPath);
      attrs["clip-path"] = `url(#${clipId})`;
    }
    const transform = primitiveRotateTransform(primitive);
    if (transform) {
      attrs.transform = transform;
    }
    svgRoot.appendChild(makeSvgNode("path", attrs));
    return;
  }
  if (primitive.kind === "polygon" && Array.isArray(primitive.points)) {
    if (
      primitive.role === "document-knockout"
      && (primitive.nodeId || primitive.node_id)
      && !options.labelDebugMode
    ) {
      return;
    }
    const rectangularBondLine = rectangularBondLineFromPolygon(primitive);
    if (rectangularBondLine) {
      const attrs = {
        x1: rectangularBondLine.start.x,
        y1: rectangularBondLine.start.y,
        x2: rectangularBondLine.end.x,
        y2: rectangularBondLine.end.y,
        stroke: primitive.stroke || primitive.fill || CHEMDRAW_INK,
        "stroke-width": rectangularBondLine.width,
        "stroke-linecap": "butt",
        "stroke-linejoin": "miter",
        "shape-rendering": "geometricPrecision",
        "data-role": primitive.role || undefined,
        ...primitiveIdentityAttrs(primitive, options),
        class: "mol-bond-stroked",
      };
      svgRoot.appendChild(makeSvgNode("line", attrs));
      return;
    }
    const strokeWidth = primitiveStrokeWidthValue(primitive, BOND_STROKE);
    const attrs = {
      points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
      fill: primitive.fill || CHEMDRAW_INK,
      stroke: strokeWidth > 0 ? (primitive.stroke || primitive.fill || CHEMDRAW_INK) : "none",
      "stroke-width": strokeWidth,
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
    };
    if (primitive.role === "document-bond") {
      attrs.class = strokeWidth > 0 ? "mol-bond-stroked" : "mol-bond-filled";
    } else if (primitive.role === "document-knockout") {
      attrs.class = "label-knockout-shape";
    }
    svgRoot.appendChild(makeSvgNode("polygon", attrs));
    return;
  }
  if (primitive.kind === "image") {
    const href = String(primitive.href || "");
    if (!/^data:image\/(?:png|jpeg|gif|bmp);base64,/i.test(href)) {
      return;
    }
    const attrs = {
      x: primitive.x,
      y: primitive.y,
      width: primitive.width,
      height: primitive.height,
      href,
      opacity: Number.isFinite(Number(primitive.opacity)) ? primitive.opacity : 1,
      preserveAspectRatio: primitive.preserveAspectRatio || primitive.preserve_aspect_ratio
        ? "xMidYMid meet"
        : "none",
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
    };
    const transform = primitiveRotateTransform(primitive, {
      x: Number(primitive.x || 0) + Number(primitive.width || 0) * 0.5,
      y: Number(primitive.y || 0) + Number(primitive.height || 0) * 0.5,
    });
    if (transform) {
      attrs.transform = transform;
    }
    svgRoot.appendChild(makeSvgNode("image", attrs));
    return;
  }
  if (primitive.kind === "rect") {
    if (primitive.role === "document-knockout" && !options.labelDebugMode) {
      return;
    }
    const attrs = {
      x: primitive.x,
      y: primitive.y,
      width: primitive.width,
      height: primitive.height,
      fill: primitive.fill || "none",
      stroke: primitive.stroke || "none",
      "stroke-width": primitiveStrokeWidthValue(primitive, 1),
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
      rx: primitive.rx,
      ry: primitive.ry,
    };
    if (primitive.role === "document-knockout") {
      attrs.class = "label-knockout-shape";
    } else if (
      primitive.role === "document-diagnostic"
      || (primitive.role === "document-graphic" && primitive.stroke === "#d32f2f")
    ) {
      attrs.class = "document-diagnostic-marker";
    }
    applyGradientFill(svgRoot, attrs, primitive.fillGradient || primitive.fill_gradient, primitive.objectId, "0%", "0%", "0%", "100%");
    if ((primitive.dashArray || primitive.dash_array)?.length) {
      attrs["stroke-dasharray"] = (primitive.dashArray || primitive.dash_array).join(" ");
    }
    svgRoot.appendChild(makeSvgNode("rect", attrs));
    return;
  }
  if (primitive.kind === "ellipse") {
    const attrs = {
      cx: primitive.center?.x,
      cy: primitive.center?.y,
      rx: primitive.rx,
      ry: primitive.ry,
      fill: primitive.fill || "none",
      stroke: primitive.stroke || "none",
      "stroke-width": primitiveStrokeWidthValue(primitive, 1),
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
    };
    const rotate = Number(primitive.rotate || 0);
    if (Math.abs(rotate) > 0.0001) {
      attrs.transform = `rotate(${rotate} ${primitive.center.x} ${primitive.center.y})`;
    }
    applyGradientFill(svgRoot, attrs, primitive.fillGradient || primitive.fill_gradient, primitive.objectId, "0%", "0%", "100%", "100%");
    if ((primitive.dashArray || primitive.dash_array)?.length) {
      attrs["stroke-dasharray"] = (primitive.dashArray || primitive.dash_array).join(" ");
    }
    svgRoot.appendChild(makeSvgNode("ellipse", attrs));
    return;
  }
  if (primitive.kind === "circle" && primitive.center) {
    const attrs = {
      cx: primitive.center.x,
      cy: primitive.center.y,
      r: primitive.radius,
      fill: primitive.fill || "none",
      stroke: primitive.stroke || "none",
      "stroke-width": primitiveStrokeWidthValue(primitive, 1),
      "data-role": primitive.role || undefined,
      ...primitiveIdentityAttrs(primitive, options),
      ...primitiveInteractionAttrs(primitive),
    };
    if (primitive.role === "document-diagnostic") {
      attrs.class = "document-diagnostic-marker";
    }
    svgRoot.appendChild(makeSvgNode("circle", attrs));
    return;
  }
  if (primitive.kind === "text") {
    renderTextPrimitive(svgRoot, primitive, options);
  }
}

function applyGradientFill(svgRoot, attrs, gradient, objectId, defaultX1, defaultY1, defaultX2, defaultY2) {
  if (!gradient?.stops?.length) {
    return;
  }
  const defs = ensureSvgDefs(svgRoot);
  const gradientId = `grad-core-${objectId || Math.random().toString(36).slice(2)}`;
  const linearGradient = makeSvgNode("linearGradient", {
    id: gradientId,
    x1: gradient.x1 || defaultX1,
    y1: gradient.y1 || defaultY1,
    x2: gradient.x2 || defaultX2,
    y2: gradient.y2 || defaultY2,
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

function renderTextPrimitive(svgRoot, primitive, options) {
  const textNode = makeSvgNode("text", {
    x: primitive.x,
    y: primitive.y,
    class: "chem-text",
    "font-size": primitive.fontSize || primitive.font_size || DEFAULT_TEXT_FONT_SIZE,
    "dominant-baseline": primitive.dominantBaseline || primitive.dominant_baseline || "alphabetic",
    "alignment-baseline": primitive.dominantBaseline || primitive.dominant_baseline || undefined,
    "text-anchor": primitive.textAnchor || primitive.text_anchor || "start",
    "data-role": primitive.role || undefined,
    ...primitiveIdentityAttrs(primitive, options),
    fill: primitive.fill ? normalizeDisplayColor(primitive.fill) : undefined,
    "font-family": primitive.fontFamily
      ? displayLabelFontFamily(primitive.fontFamily)
      : primitive.font_family
        ? displayLabelFontFamily(primitive.font_family)
        : undefined,
  });
  const transform = primitiveRotateTransform(primitive, { x: primitive.x, y: primitive.y });
  if (transform) {
    textNode.setAttribute("transform", transform);
  }
  if (Array.isArray(primitive.runs) && primitive.runs.length) {
    for (const run of primitive.runs) {
      const runFontSize = Number(run.fontSize || primitive.fontSize || DEFAULT_TEXT_FONT_SIZE);
      const isSub = isSubscriptRun(run);
      const isSuper = isSuperscriptRun(run);
      const isSubOrSuper = isSub || isSuper;
      const scriptScale = isSub
        ? editorScriptScale(options.sharedGlyphProfiles, "subscript")
        : isSuper
          ? editorScriptScale(options.sharedGlyphProfiles, "superscript")
          : 1;
      const fontWeight = fontWeightForRun(run);
      const effectColor = run.fill ? normalizeDisplayColor(run.fill) : undefined;
      const tspan = makeSvgNode("tspan", {
        fill: run.outline ? "none" : effectColor,
        "font-size": isSubOrSuper ? Math.max(cssPxToPt(7), runFontSize * scriptScale) : runFontSize,
        "font-family": run.fontFamily ? displayLabelFontFamily(run.fontFamily) : undefined,
        "font-weight": fontWeight,
        "font-style": fontStyleForRun(run),
        "text-decoration": run.underline ? "underline" : undefined,
        stroke: run.outline ? (effectColor || "#000000") : undefined,
        "stroke-width": run.outline ? Math.max(0.35, runFontSize * 0.045) : undefined,
        "paint-order": run.outline ? "stroke" : undefined,
        style: run.shadow ? `filter:drop-shadow(0.08em 0.08em 0 ${effectColor || "#000000"})` : undefined,
        "baseline-shift": isSubOrSuper
          ? editorSvgScriptBaselineShift(options.sharedGlyphProfiles, runFontSize, run.script, fontWeight)
          : undefined,
        dx: isSuper ? "-0.02em" : undefined,
      });
      tspan.textContent = run.text || "";
      textNode.appendChild(tspan);
    }
  } else {
    textNode.textContent = primitive.text || "";
  }
  svgRoot.appendChild(textNode);
}

function primitiveRotateTransform(primitive, defaultCenter = null) {
  const rotate = Number(primitive.rotate || 0);
  if (Math.abs(rotate) <= 0.0001) {
    return "";
  }
  const center = primitive.rotateCenter || primitive.rotate_center || defaultCenter;
  if (!center) {
    return "";
  }
  return `rotate(${rotate} ${center.x} ${center.y})`;
}
