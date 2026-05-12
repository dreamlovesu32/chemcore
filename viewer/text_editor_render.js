import { editorSvgScriptBaselineShift, textLength } from "./text_metrics.js";
import {
  displayLabelFontFamily,
  fontStyleForRun,
  fontWeightForRun,
  isSubscriptRun,
  isSuperscriptRun,
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";

export function editorSourceRunsFromSession(session, root, options) {
  const {
    defaultFontFamily,
    defaultFontSize,
    defaultTextColor,
    normalizeRuns,
    baseStyle,
  } = options;
  const fallbackStyle = baseStyle(root);
  const rawRuns = Array.isArray(session.sourceRuns) && session.sourceRuns.length
    ? session.sourceRuns.map((run) => ({ ...run }))
    : session.text
      ? [{
        text: String(session.text || ""),
        fontFamily: session.fontFamily || fallbackStyle.fontFamily || defaultFontFamily,
        fontSize: Number(session.fontSize || root.dataset.baseFontSize || defaultFontSize),
        fill: session.fill || root.style.color || defaultTextColor,
        fontWeight: 400,
        fontStyle: "normal",
        underline: false,
        script: session.defaultChemical ? "chemical" : "normal",
      }]
      : [];
  return normalizeRuns(rawRuns, fallbackStyle);
}

function normalizedSelectionRange(selectionOffsets) {
  if (!selectionOffsets) {
    return null;
  }
  const start = Number(selectionOffsets.start ?? Math.min(selectionOffsets.anchor, selectionOffsets.focus));
  const end = Number(selectionOffsets.end ?? Math.max(selectionOffsets.anchor, selectionOffsets.focus));
  if (!Number.isFinite(start) || !Number.isFinite(end) || start === end) {
    return null;
  }
  return {
    start: Math.min(start, end),
    end: Math.max(start, end),
  };
}

export function fillTextEditorContent(root, session, selectionOffsets, options) {
  const { scriptScale } = options;
  root.innerHTML = "";
  const layout = session;
  const lines = Array.isArray(layout?.lines) && layout.lines.length
    ? layout.lines
    : [{
      index: 0,
      x: 0,
      y: 0,
      baselineY: Number.parseFloat(root.style.fontSize || root.dataset.baseFontSize || "10") * 0.82,
      height: Number.parseFloat(root.style.lineHeight || root.dataset.baseLineHeight || "10.5"),
      textAnchor: "start",
      runs: [],
    }];
  const selectionRange = normalizedSelectionRange(layout?.selection || selectionOffsets);
  let textOffset = 0;
  const svgWidth = Math.max(8, Number(layout?.width || 0));
  const svgHeight = Math.max(1, Number(layout?.height || 0));
  const svg = makeSvgNode("svg", {
    class: "text-editor-svg",
    width: svgWidth,
    height: svgHeight,
    viewBox: `0 0 ${svgWidth} ${svgHeight}`,
    "data-editor-text-svg": "true",
  });
  const content = makeSvgNode("g", { "data-editor-text-content": "true" });

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] || {};
    const lineRuns = Array.isArray(line.runs) ? line.runs : [];
    const baseFontSize = lineRuns.find((run) => Number.isFinite(Number(run.fontSize)))
      ? Number(lineRuns.find((run) => Number.isFinite(Number(run.fontSize))).fontSize)
      : Number.parseFloat(root.dataset.baseFontSize || `${layout?.lineHeight || 10}`) || 10;
    const textNode = makeSvgNode("text", {
      x: Number(line.x || 0),
      y: Number(line.baselineY || 0),
      "data-editor-text-line": String(lineIndex),
      class: "chem-text",
      "font-size": baseFontSize,
      "dominant-baseline": "alphabetic",
      "text-anchor": line.textAnchor || "start",
    });
    for (const run of lineRuns) {
      const runText = String(run.text || "");
      const runStart = textOffset;
      const runEnd = runStart + textLength(runText);
      const isSelected = selectionRange && runStart < selectionRange.end && runEnd > selectionRange.start;
      const runFontSize = Number(run.fontSize || baseFontSize);
      const isSub = isSubscriptRun(run);
      const isSuper = isSuperscriptRun(run);
      const isSubOrSuper = isSub || isSuper;
      const scale = isSub ? scriptScale("subscript") : isSuper ? scriptScale("superscript") : 1;
      const fontWeight = fontWeightForRun(run);
      const tspan = makeSvgNode("tspan", {
        class: isSelected ? "text-editor-run is-selected" : "text-editor-run",
        "data-script": run.script || undefined,
        fill: run.fill ? normalizeDisplayColor(run.fill) : undefined,
        "font-size": isSubOrSuper ? Math.max(7, runFontSize * scale) : runFontSize,
        "font-family": run.fontFamily ? displayLabelFontFamily(run.fontFamily) : undefined,
        "font-weight": fontWeight,
        "font-style": fontStyleForRun(run),
        "text-decoration": run.underline ? "underline" : undefined,
        "baseline-shift": isSubOrSuper
          ? editorSvgScriptBaselineShift(null, runFontSize, run.script, fontWeight)
          : undefined,
        dx: isSuper ? "-0.02em" : undefined,
      });
      tspan.textContent = runText;
      textNode.appendChild(tspan);
      textOffset = runEnd;
    }
    if (lineIndex < lines.length - 1) {
      textOffset += 1;
    }
    if (!lineRuns.length) {
      textNode.textContent = "\u00A0";
      textNode.setAttribute("fill", "transparent");
    }
    content.appendChild(textNode);
  }

  if (!textLength(layout?.text || "")) {
    const placeholder = makeSvgNode("text", {
      x: 0,
      y: Number(lines[0]?.baselineY || 0),
      "font-size": Number.parseFloat(root.dataset.baseFontSize || "10") || 10,
      "dominant-baseline": "alphabetic",
      fill: "transparent",
    });
    placeholder.textContent = "\u00A0";
    content.appendChild(placeholder);
  }

  svg.appendChild(content);
  root.appendChild(svg);
}
