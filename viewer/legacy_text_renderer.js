import {
  displayLabelFontFamily,
  fontStyleForRun,
  fontWeightForRun,
  isSubscriptRun,
  isSuperscriptRun,
  makeSvgNode,
  normalizeDisplayColor,
  wrapTextLines,
} from "./render_support.js";
import { editorScriptScale, editorSvgScriptBaselineShift } from "./text_metrics.js";
import {
  DEFAULT_TEXT_FONT_SIZE,
  DEFAULT_TEXT_LINE_HEIGHT,
  DEFAULT_TEXT_WRAP_WIDTH,
} from "./legacy_render_shared.js";
import { cssPxToCm } from "./units.js";

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
        const fontWeight = fontWeightForRun(run);
        const tspan = makeSvgNode("tspan", {
          fill: run.fill ? normalizeDisplayColor(run.fill) : undefined,
          "font-size": isSubOrSuper
            ? Math.max(cssPxToCm(7), runFontSize * editorScriptScale(null, run.script))
            : runFontSize,
          "font-family": run.fontFamily ? displayLabelFontFamily(run.fontFamily) : undefined,
          "font-weight": fontWeight,
          "font-style": fontStyleForRun(run),
          "text-decoration": run.underline ? "underline" : undefined,
          "baseline-shift": isSubOrSuper
            ? editorSvgScriptBaselineShift(null, runFontSize, run.script, fontWeight)
            : undefined,
          dx: isSuper ? "-0.02em" : undefined,
        });
        tspan.textContent = run.text;
        lineNode.appendChild(tspan);
      }
      svgRoot.appendChild(lineNode);
    });
    return;
  }

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
