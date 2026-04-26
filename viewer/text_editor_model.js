import { sliceTextByOffset, textLength } from "./text_metrics.js";

export function runsPlainText(runs) {
  return (runs || []).map((run) => run.text || "").join("");
}

export function mergeSerializedRuns(runs) {
  const merged = [];
  for (const run of runs) {
    if (!run.text) {
      continue;
    }
    const previous = merged[merged.length - 1];
    if (
      previous
      && previous.fontFamily === run.fontFamily
      && previous.fontSize === run.fontSize
      && previous.fill === run.fill
      && previous.fontWeight === run.fontWeight
      && previous.fontStyle === run.fontStyle
      && previous.underline === run.underline
      && previous.script === run.script
    ) {
      previous.text += run.text;
    } else {
      merged.push(run);
    }
  }
  return merged;
}

export function normalizeEditorSourceRuns(runs, fallbackStyle, normalizeColor) {
  return mergeSerializedRuns((runs || [])
    .filter((run) => typeof run?.text === "string" && textLength(run.text))
    .map((run) => ({
      text: run.text,
      fontFamily: run.fontFamily || fallbackStyle.fontFamily,
      fontSize: Number(run.fontSize || fallbackStyle.fontSize),
      fill: normalizeColor(run.fill || fallbackStyle.fill),
      fontWeight: Number(run.fontWeight || fallbackStyle.fontWeight || 400),
      fontStyle: String(run.fontStyle || fallbackStyle.fontStyle || "normal"),
      underline: Boolean(run.underline ?? fallbackStyle.underline),
      script: String(run.script || fallbackStyle.script || "normal"),
    })));
}

export function normalizeEditorSelectionOffsets(plainText, selectionOffsets) {
  if (!selectionOffsets) {
    return null;
  }
  const plainTextLength = textLength(plainText || "");
  const anchor = Math.max(0, Math.min(plainTextLength, Number(selectionOffsets.anchor ?? 0)));
  const focus = Math.max(0, Math.min(plainTextLength, Number(selectionOffsets.focus ?? anchor)));
  const start = Math.min(anchor, focus);
  const end = Math.max(anchor, focus);
  return {
    anchor,
    focus,
    start,
    end,
    collapsed: anchor === focus,
  };
}

export function splitRunsForSelection(runs, start, end) {
  const before = [];
  const selected = [];
  const after = [];
  let cursor = 0;
  for (const originalRun of runs) {
    const run = { ...originalRun };
    const text = run.text || "";
    const runStart = cursor;
    const runLength = textLength(text);
    const runEnd = cursor + runLength;
    cursor = runEnd;

    if (runEnd <= start) {
      before.push(run);
      continue;
    }
    if (runStart >= end) {
      after.push(run);
      continue;
    }
    const localStart = Math.max(0, start - runStart);
    const localEnd = Math.min(runLength, end - runStart);
    if (localStart > 0) {
      before.push({ ...run, text: sliceTextByOffset(text, 0, localStart) });
    }
    if (localEnd > localStart) {
      selected.push({ ...run, text: sliceTextByOffset(text, localStart, localEnd) });
    }
    if (localEnd < runLength) {
      after.push({ ...run, text: sliceTextByOffset(text, localEnd) });
    }
  }
  return { before, selected, after };
}

export function styleAtEditorOffset(offset, runs, baseStyle, normalizeColor) {
  let cursor = 0;
  for (const run of runs || []) {
    const length = textLength(run.text || "");
    if (offset <= cursor + length) {
      return {
        fontFamily: run.fontFamily || baseStyle.fontFamily,
        fontSize: Number(run.fontSize || baseStyle.fontSize),
        fill: normalizeColor(run.fill || baseStyle.fill),
        fontWeight: Number(run.fontWeight || baseStyle.fontWeight || 400),
        fontStyle: String(run.fontStyle || baseStyle.fontStyle || "normal"),
        underline: Boolean(run.underline ?? baseStyle.underline),
        script: String(run.script || baseStyle.script || "normal"),
      };
    }
    cursor += length;
  }
  return baseStyle;
}
