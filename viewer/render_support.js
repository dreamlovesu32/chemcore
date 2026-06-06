import { cssPxToPt } from "./units.js";

const CHEMDRAW_INK = "#000000";
const CHEMDRAW_COLOR_MAP = new Map([
  ["#d61f1f", "#ff0000"],
  ["#1b32d8", "#0000ff"],
]);

export function normalizeDisplayColor(color, fallback = CHEMDRAW_INK) {
  if (!color) {
    return fallback;
  }
  const value = String(color).trim().toLowerCase();
  return CHEMDRAW_COLOR_MAP.get(value) || color;
}

export function displayLabelFontFamily(fontFamily) {
  const value = String(fontFamily || "").trim();
  if (!value || /^(arial)$/i.test(value)) {
    return "Arial, \"Helvetica Neue\", Helvetica, sans-serif";
  }
  if (/^(helvetica)$/i.test(value)) {
    return "Helvetica, Arial, sans-serif";
  }
  if (/^(texgyreheros|tex gyre heros)$/i.test(value)) {
    return "\"TeX Gyre Heros\", Arial, Helvetica, sans-serif";
  }
  return `${value}, "TeX Gyre Heros", Arial, Helvetica, sans-serif`;
}

export function isSubscriptRun(run) {
  const script = String(run?.script || "").toLowerCase();
  return script === "subscript";
}

export function isSuperscriptRun(run) {
  const script = String(run?.script || "").toLowerCase();
  return script === "superscript";
}

export function fontWeightForRun(run) {
  if (run?.fontWeight !== undefined && run?.fontWeight !== null) {
    return Number(run.fontWeight);
  }
  return undefined;
}

export function fontStyleForRun(run) {
  if (run?.fontStyle) {
    return run.fontStyle;
  }
  return undefined;
}

export function makeSvgNode(name, attributes = {}) {
  const node = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [key, value] of Object.entries(attributes)) {
    if (value == null || value === undefined || value === "") {
      continue;
    }
    node.setAttribute(key, String(value));
  }
  return node;
}

export function ensureSvgDefs(svgRoot) {
  let defs = svgRoot.querySelector("defs");
  if (!defs) {
    defs = makeSvgNode("defs");
    svgRoot.appendChild(defs);
  }
  return defs;
}

export function wrapTextLines(text, maxWidth, fontSize) {
  const rawLines = String(text || "").split("\n");
  const maxChars = Math.max(8, Math.floor(maxWidth / Math.max(cssPxToPt(6), fontSize * 0.6)));
  const out = [];

  for (const rawLine of rawLines) {
    const line = rawLine.trim();
    if (!line) {
      continue;
    }
    if (line.length <= maxChars || !line.includes(" ")) {
      out.push(line);
      continue;
    }
    const words = line.split(/\s+/);
    let current = "";
    for (const word of words) {
      const next = current ? `${current} ${word}` : word;
      if (next.length > maxChars && current) {
        out.push(current);
        current = word;
      } else {
        current = next;
      }
    }
    if (current) {
      out.push(current);
    }
  }

  return out;
}
