import { sliceTextByOffset, textLength } from "./text_metrics.js";

function appendEditorTextSegment(root, run, text, offset, selectionOffsets, renderRunNode) {
  const selection = selectionOffsets || null;
  const segmentLength = textLength(text);
  if (!selection || selection.start >= offset + segmentLength || selection.end <= offset) {
    root.appendChild(renderRunNode(run, text));
    return;
  }
  const start = Math.max(0, selection.start - offset);
  const end = Math.min(segmentLength, selection.end - offset);
  if (start > 0) {
    root.appendChild(renderRunNode(run, sliceTextByOffset(text, 0, start)));
  }
  if (end > start) {
    root.appendChild(renderRunNode(run, sliceTextByOffset(text, start, end), true));
  }
  if (end < segmentLength) {
    root.appendChild(renderRunNode(run, sliceTextByOffset(text, end)));
  }
}

export function editorSourceRunsFromSession(session, root, options) {
  const {
    defaultFontFamily,
    defaultFontSize,
    defaultTextColor,
    normalizeRuns,
    baseStyle,
  } = options;
  const rawRuns = Array.isArray(session.sourceRuns) && session.sourceRuns.length
    ? session.sourceRuns.map((run) => ({ ...run }))
    : session.text
      ? [{
        text: String(session.text || ""),
        fontFamily: session.fontFamily || root.style.fontFamily || defaultFontFamily,
        fontSize: Number(session.fontSize || root.style.fontSize || defaultFontSize),
        fill: session.fill || root.style.color || defaultTextColor,
        fontWeight: 400,
        fontStyle: "normal",
        underline: false,
        script: session.defaultChemical ? "chemical" : "normal",
      }]
      : [];
  return normalizeRuns(rawRuns, baseStyle(root));
}

export function previewTextRunsFromKernel(sourceRuns, root, options) {
  const {
    engine,
    parseJson,
    baseStyle,
    normalizeRuns,
    runsPlainText,
    defaultTextAlign,
    defaultLineHeight,
    target,
  } = options;
  if (!engine?.previewTextRuns || !root) {
    return null;
  }
  const fallbackStyle = baseStyle(root);
  const preview = parseJson(engine.previewTextRuns(JSON.stringify({
    target: target || {
      kind: "text-object",
      objectId: null,
      x: 0,
      y: 0,
    },
    text: runsPlainText(sourceRuns || []),
    sourceRuns: sourceRuns || [],
    fontFamily: fallbackStyle.fontFamily,
    fontSize: fallbackStyle.fontSize,
    fill: fallbackStyle.fill,
    align: root.style.textAlign || defaultTextAlign,
    lineHeight: Number.parseFloat(root.style.lineHeight || `${defaultLineHeight(fallbackStyle.fontSize)}`),
    defaultChemical: root.dataset.defaultChemical === "true",
  })), null);
  if (!preview) {
    return null;
  }
  return {
    sourceRuns: normalizeRuns(preview.sourceRuns || sourceRuns || [], fallbackStyle),
    displayRuns: normalizeRuns(preview.displayRuns || [], fallbackStyle),
  };
}

export function displayRunsForEditor(sourceRuns, root, options) {
  const preview = previewTextRunsFromKernel(sourceRuns, root, options);
  if (preview?.displayRuns) {
    return preview.displayRuns;
  }
  return options.normalizeRuns(sourceRuns || [], options.baseStyle(root));
}

export function fillTextEditorContent(root, session, selectionOffsets, options) {
  const {
    resolveDisplayRuns,
    renderRunNode,
  } = options;
  root.innerHTML = "";
  const runs = resolveDisplayRuns(session);
  let hasContent = false;
  let offset = 0;
  for (const run of runs) {
    const parts = String(run.text || "").split("\n");
    for (let index = 0; index < parts.length; index += 1) {
      if (parts[index]) {
        appendEditorTextSegment(root, run, parts[index], offset, selectionOffsets, renderRunNode);
        hasContent = true;
        offset += textLength(parts[index]);
      }
      if (index < parts.length - 1) {
        root.appendChild(document.createElement("br"));
        offset += 1;
      }
    }
  }
  if (!hasContent && !root.childNodes.length) {
    root.appendChild(document.createElement("br"));
  }
}
