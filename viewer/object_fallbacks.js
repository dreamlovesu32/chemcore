import {
  displayLabelFontFamily,
  ensureSvgDefs,
  fontStyleForRun,
  fontWeightForRun,
  isSubscriptRun,
  isSuperscriptRun,
  makeSvgNode,
  normalizeDisplayColor,
  wrapTextLines,
} from "./render_support.js";
import { cssPxToCm } from "./units.js";

const DEFAULT_TEXT_FONT_SIZE = 10;
const DEFAULT_TEXT_LINE_HEIGHT = cssPxToCm(10.5);
const DEFAULT_LINE_STROKE_WIDTH = cssPxToCm(1.6);
const DEFAULT_TEXT_WRAP_WIDTH = cssPxToCm(160);
const DEFAULT_SHAPE_STROKE_WIDTH = cssPxToCm(1);

export function renderLineObject(svgRoot, object, styles) {
  const points = object.payload.points || [];
  if (points.length < 2) {
    return;
  }

  const style = styles?.[object.styleRef] || {};
  const stroke = style.stroke || "#222222";
  const strokeWidth = style.strokeWidth || DEFAULT_LINE_STROKE_WIDTH;
  const lineCap = style.lineCap || "round";
  const lineJoin = style.lineJoin || "round";
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

export function renderTextObject(svgRoot, object) {
  const [tx, ty] = object.transform.translate;
  const fontSize = Number(object.payload.fontSize || DEFAULT_TEXT_FONT_SIZE);
  const lines = object.payload.preserveLines
    ? String(object.payload.text || "")
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
    : wrapTextLines(
        String(object.payload.text || ""),
        Number(object.payload.box?.[2] || DEFAULT_TEXT_WRAP_WIDTH),
        fontSize,
      );
  const align = object.payload.align || "left";
  const lineHeight = Number(object.payload.lineHeight || DEFAULT_TEXT_LINE_HEIGHT);
  const textAnchor = align === "center" ? "middle" : align === "right" ? "end" : "start";

  if (object.payload.preserveLines && object.payload.runs?.length) {
    const lineRuns = [[]];
    for (const run of object.payload.runs) {
      const segments = String(run.text || "").split("\n");
      for (let i = 0; i < segments.length; i += 1) {
        const segment = segments[i];
        if (segment) {
          lineRuns[lineRuns.length - 1].push({ ...run, text: segment });
        }
        if (i < segments.length - 1) {
          lineRuns.push([]);
        }
      }
    }

    lineRuns.forEach((runs, index) => {
      if (!runs.length) {
        return;
      }
      const lineY = ty + fontSize * 0.82 + index * lineHeight;
      const lineNode = makeSvgNode("text", {
        x: tx,
        y: lineY,
        class: "chem-text",
        "font-size": fontSize,
        "dominant-baseline": "alphabetic",
        "text-anchor": textAnchor,
      });
      for (const run of runs) {
        const runFontSize = Number(run.fontSize || fontSize);
        const isSub = isSubscriptRun(run);
        const isSuper = isSuperscriptRun(run);
        const isSubOrSuper = isSub || isSuper;
        const tspan = makeSvgNode("tspan", {
          fill: run.fill ? normalizeDisplayColor(run.fill) : undefined,
          "font-size": isSubOrSuper ? Math.max(cssPxToCm(7), runFontSize * 0.72) : runFontSize,
          "font-family": run.fontFamily ? displayLabelFontFamily(run.fontFamily) : undefined,
          "font-weight": fontWeightForRun(run),
          "font-style": fontStyleForRun(run),
          "text-decoration": run.underline ? "underline" : undefined,
          "baseline-shift": isSub ? "-28%" : isSuper ? "48%" : undefined,
          dx: isSuper ? "-0.02em" : undefined,
        });
        tspan.textContent = run.text;
        lineNode.appendChild(tspan);
      }
      svgRoot.appendChild(lineNode);
    });
  } else {
    const textNode = makeSvgNode("text", {
      x: tx,
      y: ty + fontSize * 0.82,
      class: "chem-text",
      "font-size": fontSize,
      "dominant-baseline": "alphabetic",
      "text-anchor": textAnchor,
    });
    lines.forEach((line, index) => {
      const tspan = makeSvgNode("tspan", {
        x: tx,
        dy: index === 0 ? 0 : lineHeight,
      });
      tspan.textContent = line;
      textNode.appendChild(tspan);
    });
    svgRoot.appendChild(textNode);
  }
}

export function renderShapeObject(svgRoot, object, styles) {
  const [tx, ty] = object.transform.translate;
  const style = styles?.[object.styleRef] || {};
  const [, , width, height] = object.payload.bbox || [0, 0, 0, 0];
  const gradient = style.fillGradient;
  const kind = object.payload.kind || "rect";
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
      "stroke-width": style.strokeWidth || DEFAULT_SHAPE_STROKE_WIDTH,
      transform: Math.abs(rotate) > 0.0001 ? `rotate(${rotate} ${center[0]} ${center[1]})` : undefined,
    };
    if (style.dashArray?.length) {
      attrs["stroke-dasharray"] = style.dashArray.join(" ");
    }
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
    "stroke-width": style.strokeWidth || DEFAULT_SHAPE_STROKE_WIDTH,
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
  if (style.dashArray?.length) {
    attrs["stroke-dasharray"] = style.dashArray.join(" ");
  }
  if (object.payload.kind === "roundRect") {
    attrs.rx = object.payload.cornerRadius || 0;
    attrs.ry = object.payload.cornerRadius || 0;
  }
  svgRoot.appendChild(makeSvgNode("rect", attrs));
}
