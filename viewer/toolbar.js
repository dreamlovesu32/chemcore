import { cssPxToPt } from "./units.js";

export const TEXT_FONT_OPTIONS = [
  "Arial",
  "Helvetica",
  "TeX Gyre Heros",
  "Times New Roman",
  "Courier New",
];

export const TEXT_FONT_SIZE_OPTIONS = [5, 6, 7, 8, 9, 10, 12, 14, 16, 18, 24];

export function normalizeToolbarFontSize(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) {
    return 10;
  }
  const rounded = Math.round(numeric);
  if (Math.abs(numeric - rounded) < 0.05) {
    return rounded;
  }
  return Math.round(numeric * 10) / 10;
}

export function formatToolbarFontSize(value) {
  const normalized = normalizeToolbarFontSize(value);
  return Number.isInteger(normalized) ? String(normalized) : normalized.toFixed(1);
}

export function arrowTypeSupportsHeadSize(type) {
  return type === "solid" || type === "curved" || type === "curved-mirror" || isEquilibriumArrowType(type);
}

export const ARROW_TOOL_ICON_TYPES = [
  "equilibrium-small",
  "equilibrium-medium",
  "equilibrium-large",
  "unequal-equilibrium-small",
  "unequal-equilibrium-medium",
  "unequal-equilibrium-large",
];

export const SHAPE_TOOL_ICON_KINDS = ["circle", "ellipse", "round-rect", "rect", "cross-table"];
export const SHAPE_TOOL_STYLE_KINDS = ["circle", "ellipse", "round-rect", "rect"];
export const SHAPE_TOOL_ICON_STYLES = ["solid", "dashed", "shaded", "filled", "shadowed"];
export const ORBITAL_TOOL_ICON_TEMPLATES = ["s", "p", "dxy", "oval", "hybrid", "dz2", "lobe"];
export const ORBITAL_TOOL_ICON_STYLES = ["hollow", "filled", "shaded"];
export const ORBITAL_TOOL_ICON_PHASES = ["plus", "minus"];
export const SYMBOL_TOOL_ICON_TYPES = [
  "circle-plus",
  "plus",
  "radical-cation",
  "lone-pair",
  "circle-minus",
  "minus",
  "radical-anion",
  "electron",
];

const ORBITAL_TEMPLATE_BUTTONS = [
  { template: "s", title: "s orbital", style: "shaded", phase: "plus" },
  { template: "oval", title: "oval orbital", style: "shaded", phase: "plus" },
  { template: "lobe", title: "lobe orbital", style: "shaded", phase: "plus" },
  { template: "p", title: "p orbital", style: "shaded", phase: "plus" },
  { template: "dxy", title: "dxy orbital", style: "shaded", phase: "plus" },
  { template: "hybrid", title: "hybrid orbital", style: "shaded", phase: "minus" },
  { template: "dz2", title: "dz2 orbital", style: "shaded", phase: "minus" },
];

const ORBITAL_STYLE_BUTTONS_BY_TEMPLATE = {
  s: [
    { title: "Hollow", style: "hollow", phase: "plus" },
    { title: "Filled", style: "filled", phase: "plus" },
    { title: "Shaded", style: "shaded", phase: "plus" },
  ],
  oval: [
    { title: "Hollow", style: "hollow", phase: "plus" },
    { title: "Filled", style: "filled", phase: "plus" },
    { title: "Shaded", style: "shaded", phase: "plus" },
  ],
  lobe: [
    { title: "Hollow", style: "hollow", phase: "plus" },
    { title: "Filled", style: "filled", phase: "plus" },
    { title: "Shaded", style: "shaded", phase: "plus" },
  ],
  p: [
    { title: "Filled", style: "filled", phase: "plus" },
    { title: "Shaded", style: "shaded", phase: "plus" },
  ],
  dxy: [
    { title: "Filled", style: "filled", phase: "plus" },
    { title: "Shaded", style: "shaded", phase: "plus" },
  ],
  hybrid: [
    { title: "Filled plus", style: "filled", phase: "plus" },
    { title: "Filled minus", style: "filled", phase: "minus" },
    { title: "Shaded plus", style: "shaded", phase: "plus" },
    { title: "Shaded minus", style: "shaded", phase: "minus" },
  ],
  dz2: [
    { title: "Filled plus", style: "filled", phase: "plus" },
    { title: "Filled minus", style: "filled", phase: "minus" },
    { title: "Shaded plus", style: "shaded", phase: "plus" },
    { title: "Shaded minus", style: "shaded", phase: "minus" },
  ],
};

const ICON_VIEWBOX = "0 0 24 24";
const ICON_BLACK = "#111318";
const ICON_BLUE = "#2f6fed";

function iconSvg(content, className = "") {
  const classAttr = className ? ` class="chemcore-icon ${className}"` : ` class="chemcore-icon"`;
  return `<svg${classAttr} viewBox="${ICON_VIEWBOX}" aria-hidden="true">${content}</svg>`;
}

function linePath(from, to, className = "cc-stroke", extra = "") {
  return `<path class="${className}" d="M${fmt(from.x)} ${fmt(from.y)} L${fmt(to.x)} ${fmt(to.y)}"${extra}/>`;
}

function polyline(points, className = "cc-stroke", extra = "") {
  const d = points
    .map((point, index) => `${index ? "L" : "M"}${fmt(point.x)} ${fmt(point.y)}`)
    .join(" ");
  return `<path class="${className}" d="${d}"${extra}/>`;
}

function polygon(points, className = "cc-fill", extra = "") {
  return `<polygon class="${className}" points="${points.map((point) => `${fmt(point.x)},${fmt(point.y)}`).join(" ")}"${extra}/>`;
}

function fmt(value) {
  return Number(value).toFixed(2).replace(/\.?0+$/, "");
}

function point(x, y) {
  return { x, y };
}

function add(left, right) {
  return point(left.x + right.x, left.y + right.y);
}

function sub(left, right) {
  return point(left.x - right.x, left.y - right.y);
}

function mul(vector, scalar) {
  return point(vector.x * scalar, vector.y * scalar);
}

function unit(vector) {
  const length = Math.hypot(vector.x, vector.y) || 1;
  return point(vector.x / length, vector.y / length);
}

function normal(vector) {
  const normalized = unit(vector);
  return point(-normalized.y, normalized.x);
}

function arrowHead(tip, direction, size = 1, className = "cc-arrow-fill") {
  const axis = unit(direction);
  const side = normal(axis);
  const base = add(tip, mul(axis, -4.7 * size));
  return polygon([
    tip,
    add(base, mul(side, 3.1 * size)),
    add(base, mul(side, -3.1 * size)),
  ], className);
}

function openArrowHead(tip, direction, size = 1) {
  const axis = unit(direction);
  const side = normal(axis);
  const base = add(tip, mul(axis, -4.9 * size));
  return `${linePath(tip, add(base, mul(side, 3.1 * size)), "cc-arrow")}${linePath(tip, add(base, mul(side, -3.1 * size)), "cc-arrow")}`;
}

function straightArrowSvg({ head = "solid", tail = false, bold = false } = {}) {
  const start = point(4, 12);
  const end = point(19.5, 12);
  const strokeClass = bold ? "cc-arrow cc-arrow-bold" : "cc-arrow";
  let body = linePath(start, head === "none" ? point(20, 12) : point(15.6, 12), strokeClass);
  if (tail) {
    body += arrowHead(start, point(-1, 0), 0.92);
  }
  if (head === "solid") {
    body += arrowHead(end, point(1, 0), 0.92);
  } else if (head === "open") {
    body += openArrowHead(end, point(1, 0), 0.92);
  } else if (head === "hollow") {
    body = `<path class="${strokeClass}" d="M4 12h10.4v3.25L20 12l-5.6-3.25V12z"/>`;
  }
  return iconSvg(body, "cc-arrow-icon");
}

function curvedArrowSvg({ mirrored = false, curve = "270" } = {}) {
  const paths = {
    "270": "M18.4 6.2C12.2 3.8 5.4 7.9 5.2 14.2c-.1 4.5 3.3 7.1 7.5 6.1",
    "180": "M18.4 7.2C13.1 4.6 6.6 8.3 6.5 14.1c-.1 3.4 2.8 5.5 6.1 4.8",
    "120": "M18.4 8.4C14.2 6.1 8.5 8.1 7.2 13.2",
    "90": "M18.4 9.6C15.3 7.8 11.1 8.8 8.7 12",
  };
  const transform = mirrored ? ` transform="translate(0 24) scale(1 -1)"` : "";
  return iconSvg(`<g${transform}><path class="cc-arrow" d="${paths[curve] || paths["270"]}"/>${arrowHead(point(19.9, 7.5), point(1, -0.25), 0.78)}</g>`, "cc-arrow-icon");
}

function shapeIconSvg(kind = "rect", style = "solid", editorState = null) {
  const normalizedKind = SHAPE_TOOL_ICON_KINDS.includes(kind) ? kind : "rect";
  const normalizedStyle = SHAPE_TOOL_STYLE_KINDS.includes(normalizedKind) && SHAPE_TOOL_ICON_STYLES.includes(style)
    ? style
    : "solid";
  const cached = editorState?.shapeIconSvgs?.[shapeIconKey(normalizedKind, normalizedStyle)];
  if (cached) {
    return cached;
  }
  const fill = normalizedStyle === "filled" ? "cc-shape-fill" : normalizedStyle === "shaded" ? "cc-shape-soft-fill" : "cc-empty-fill";
  const dash = normalizedStyle === "dashed" ? ` stroke-dasharray="2.2 1.8"` : "";
  const shadow = normalizedStyle === "shadowed"
    ? `<path class="cc-shadow-fill" d="M8.2 8.2h10.3v9.5H8.2z"/><path class="cc-shadow-edge" d="M5.5 6.2 8.2 8.2M18.5 6.2v11.5M5.5 17.7h2.7"/>`
    : "";
  const mark = normalizedKind === "circle"
    ? `<circle class="${fill} cc-shape" cx="12" cy="12" r="6.2"${dash}/>`
    : normalizedKind === "ellipse"
      ? `<ellipse class="${fill} cc-shape" cx="12" cy="12" rx="7.2" ry="4.5"${dash}/>`
      : normalizedKind === "cross-table"
        ? `<rect class="${fill} cc-shape" x="5.5" y="6.2" width="13" height="11.5"${dash}/><path class="cc-shape" d="M12 6.2v11.5M5.5 11.95h13"${dash}/>`
        : `<rect class="${fill} cc-shape" x="5.5" y="6.2" width="13" height="11.5"${normalizedKind === "round-rect" ? ` rx="2.6"` : ""}${dash}/>`;
  return iconSvg(`${shadow}${mark}`, "cc-shape-icon");
}

function shapeIconKey(kind, style) {
  return `${kind}:${style}`;
}

const SHAPE_KIND_TITLES = {
  circle: "Circle",
  ellipse: "Ellipse",
  "round-rect": "Rounded rectangle",
  rect: "Rectangle",
  "cross-table": "Cross table",
};

const SHAPE_STYLE_TITLES = {
  solid: "Solid outline",
  dashed: "Dashed outline",
  shaded: "Shaded",
  filled: "Filled",
  shadowed: "Shadowed",
};

function generatedRingSvg(sides, aromatic = false) {
  const pointsBySide = {
    3: [point(12, 4.5), point(20, 18.5), point(4, 18.5)],
    4: [point(6, 6), point(18, 6), point(18, 18), point(6, 18)],
    5: [point(12, 4.2), point(20, 10.1), point(16.9, 19.3), point(7.1, 19.3), point(4, 10.1)],
    6: [point(12, 4.2), point(19, 8.2), point(19, 15.8), point(12, 19.8), point(5, 15.8), point(5, 8.2)],
    7: [point(12, 4.1), point(18.2, 7), point(20.2, 13.7), point(16.4, 19.6), point(7.6, 19.6), point(3.8, 13.7), point(5.8, 7)],
    8: [point(9, 4), point(15, 4), point(20, 9), point(20, 15), point(15, 20), point(9, 20), point(4, 15), point(4, 9)],
  };
  const ring = polygon(pointsBySide[sides] || pointsBySide[6], "cc-ring");
  const aromaticMark = aromatic ? `<circle class="cc-ring" cx="12" cy="12" r="4.35"/>` : "";
  return iconSvg(`${ring}${aromaticMark}`, "cc-ring-icon");
}

function generatedBracketIconSvg(kind = "round") {
  if (kind === "square") {
    return iconSvg(`<path class="cc-stroke" d="M9 5.2H6.4v13.6H9"/><path class="cc-stroke" d="M15 5.2h2.6v13.6H15"/>`, "cc-bracket-icon");
  }
  if (kind === "curly") {
    return iconSvg(`<path class="cc-stroke" d="M10.1 4.8c-2.3.2-2.4 2.2-2.3 3.8v1.1c0 1.4-.9 2.2-2 2.3 1.1.1 2 .9 2 2.3v1.1c-.1 1.6 0 3.6 2.3 3.8"/><path class="cc-stroke" d="M13.9 4.8c2.3.2 2.4 2.2 2.3 3.8v1.1c0 1.4.9 2.2 2 2.3-1.1.1-2 .9-2 2.3v1.1c.1 1.6 0 3.6-2.3 3.8"/>`, "cc-bracket-icon");
  }
  if (kind === "circle-plus" || kind === "circle-minus") {
    const plus = kind === "circle-plus" ? `<path class="cc-stroke" d="M12 8.1v7.8"/>` : "";
    return iconSvg(`<circle class="cc-stroke" cx="12" cy="12" r="6.15"/><path class="cc-stroke" d="M8.1 12h7.8"/>${plus}`, "cc-symbol-icon");
  }
  if (kind === "plus") {
    return iconSvg(`<path class="cc-stroke" d="M12 6.5v11"/><path class="cc-stroke" d="M6.5 12h11"/>`, "cc-symbol-icon");
  }
  if (kind === "minus") {
    return iconSvg(`<path class="cc-stroke" d="M6.5 12h11"/>`, "cc-symbol-icon");
  }
  if (kind === "radical-cation" || kind === "radical-anion") {
    const plus = kind === "radical-cation" ? `<path class="cc-stroke" d="M16 8.3v7.4"/>` : "";
    return iconSvg(`<circle class="cc-dot" cx="7.3" cy="12" r="1.7"/><path class="cc-stroke" d="M12.4 12h7.2"/>${plus}`, "cc-symbol-icon");
  }
  if (kind === "lone-pair") {
    return iconSvg(`<circle class="cc-dot" cx="9" cy="12" r="1.75"/><circle class="cc-dot" cx="15" cy="12" r="1.75"/>`, "cc-symbol-icon");
  }
  if (kind === "electron") {
    return iconSvg(`<circle class="cc-dot" cx="12" cy="12" r="2.1"/>`, "cc-symbol-icon");
  }
  return iconSvg(`<path class="cc-stroke" d="M10 5c-3 3-3 11 0 14"/><path class="cc-stroke" d="M14 5c3 3 3 11 0 14"/>`, "cc-bracket-icon");
}

function textFormatIconSvg(kind, editorState = null) {
  return editorState?.textIconSvgs?.[kind] || "";
}

function selectModeIconSpec(mode = "box") {
  if (mode === "free") {
    return {
      title: "Free selection",
      svg: iconSvg(
        `<path class="cc-stroke" d="M7 8.1c2.2-3 8.2-3.1 10.4-.1 2.8 3.8-.4 8.9-6.1 8.8-5.2-.1-8.1-4-6.1-7.2.6-.9 1.4-1.5 2.6-1.9" stroke-dasharray="2.05 2.05"/>`,
        "cc-tool-icon cc-select-mode-icon",
      ),
    };
  }
  return {
    title: "Box selection",
    svg: iconSvg(
      `<rect class="cc-stroke" x="5.2" y="5.2" width="13.6" height="13.6" rx="1.5" stroke-dasharray="2.1 2.1"/>`,
      "cc-tool-icon cc-select-mode-icon",
    ),
  };
}

function arrangeIconSvg(kind) {
  const icons = {
    "align-left": `<path class="cc-guide" d="M6 5v14"/><path class="cc-stroke-strong" d="M9 7h9"/><path class="cc-stroke-strong" d="M9 12h6"/><path class="cc-stroke-strong" d="M9 17h11"/>`,
    "align-right": `<path class="cc-guide" d="M18 5v14"/><path class="cc-stroke-strong" d="M6 7h9"/><path class="cc-stroke-strong" d="M9 12h6"/><path class="cc-stroke-strong" d="M4 17h11"/>`,
    "align-top": `<path class="cc-guide" d="M5 6h14"/><path class="cc-stroke-strong" d="M7 9v9"/><path class="cc-stroke-strong" d="M12 9v6"/><path class="cc-stroke-strong" d="M17 9v11"/>`,
    "align-bottom": `<path class="cc-guide" d="M5 18h14"/><path class="cc-stroke-strong" d="M7 6v9"/><path class="cc-stroke-strong" d="M12 9v6"/><path class="cc-stroke-strong" d="M17 4v11"/>`,
    "align-h-center": `<path class="cc-guide" d="M12 4v16"/><path class="cc-stroke-strong" d="M6 7h12"/><path class="cc-stroke-strong" d="M8 12h8"/><path class="cc-stroke-strong" d="M5 17h14"/>`,
    "align-v-center": `<path class="cc-guide" d="M4 12h16"/><path class="cc-stroke-strong" d="M7 6v12"/><path class="cc-stroke-strong" d="M12 8v8"/><path class="cc-stroke-strong" d="M17 5v14"/>`,
    "flip-h": `<path class="cc-guide" d="M12 4v16"/><path class="cc-fill-outline" d="M5 7v10l5-5z"/><path class="cc-stroke" d="M19 7v10l-5-5z"/>`,
    "flip-v": `<path class="cc-guide" d="M4 12h16"/><path class="cc-fill-outline" d="M7 5h10l-5 5z"/><path class="cc-stroke" d="M7 19h10l-5-5z"/>`,
  };
  return iconSvg(icons[kind] || "", "cc-arrange-icon");
}

function commandIconSvg(name) {
  const icons = {
    new: iconSvg(`<path class="cc-stroke" d="M6.3 3.8h8.4L18.7 8v12.2H6.3z"/><path class="cc-stroke" d="M14.7 3.8V8h4"/><path class="cc-stroke" d="M12.5 11v6"/><path class="cc-stroke" d="M9.5 14h6"/>`, "cc-command-icon"),
    open: iconSvg(`<path class="cc-stroke" d="M3.7 8h6l2 2h8.6v8.7H3.7z"/><path class="cc-stroke" d="M3.7 8V5.2h5.1l2 2h6.5V10"/><path class="cc-stroke" d="M8.2 14.2h7.2"/><path class="cc-stroke" d="m12.8 11.4 2.9 2.8-2.9 2.8"/>`, "cc-command-icon"),
    save: iconSvg(`<path class="cc-stroke" d="M5.1 4.2h11.3l2.5 2.5v13.1H5.1z"/><path class="cc-stroke" d="M8.2 4.2v6h7.2v-6"/><path class="cc-stroke" d="M8.2 15.5h7.6v4.3H8.2z"/>`, "cc-command-icon"),
    "save-as": iconSvg(`<path class="cc-stroke" d="M5.1 4.2h11.3l2.5 2.5v13.1H5.1z"/><path class="cc-stroke" d="M8.2 4.2v6h7.2v-6"/><path class="cc-stroke" d="M8.2 15.5h5.8"/><path class="cc-stroke" d="m13.9 19.5 4.2-4.2 1.6 1.6-4.2 4.2h-1.6z"/><path class="cc-stroke" d="m17.5 15.9 1.6 1.6"/>`, "cc-command-icon"),
    undo: iconSvg(`<path class="cc-stroke" d="M9.1 7.1 4.7 11.5 9.1 16"/><path class="cc-stroke" d="M5 11.5h9.3c3.5 0 5.5 2.2 5.5 5.1 0 2.7-2.1 4.9-5.2 4.9"/>`, "cc-command-icon"),
    redo: iconSvg(`<path class="cc-stroke" d="m14.9 7.1 4.4 4.4-4.4 4.5"/><path class="cc-stroke" d="M19 11.5H9.7c-3.5 0-5.5 2.2-5.5 5.1 0 2.7 2.1 4.9 5.2 4.9"/>`, "cc-command-icon"),
    delete: iconSvg(`<path class="cc-delete-stroke" d="M5.5 7.6h13"/><path class="cc-delete-stroke" d="M9.05 7.6V4.85h5.9V7.6"/><path class="cc-delete-stroke" d="M7.45 7.6 8.2 19.15h7.6l.75-11.55"/><path class="cc-delete-soft" d="M10.45 10.95v5.05"/><path class="cc-delete-soft" d="M13.55 10.95v5.05"/>`, "cc-command-icon"),
    cut: iconSvg(`<circle class="cc-stroke" cx="6.5" cy="17.3" r="2.05"/><circle class="cc-stroke" cx="17.5" cy="17.3" r="2.05"/><path class="cc-stroke" d="M8.1 15.9 18 5.3"/><path class="cc-stroke" d="m6.1 5.3 9.8 10.6"/>`, "cc-command-icon"),
    copy: iconSvg(`<rect class="cc-stroke" x="8.2" y="7.2" width="9.7" height="11.6"/><rect class="cc-stroke" x="5.2" y="4.2" width="9.7" height="11.6"/>`, "cc-command-icon"),
    paste: iconSvg(`<path class="cc-stroke" d="M8.2 5.2h7.6v3H8.2z"/><path class="cc-stroke" d="M6.2 7.2h11.6v12.6H6.2z"/><path class="cc-stroke" d="M9.1 12.3h5.8"/><path class="cc-stroke" d="M9.1 16h5"/>`, "cc-command-icon"),
    "zoom-in": iconSvg(`<circle class="cc-stroke" cx="10.3" cy="10.3" r="6"/><path class="cc-stroke" d="m14.8 14.8 5.2 5.2"/><path class="cc-stroke" d="M10.3 7.2v6.2"/><path class="cc-stroke" d="M7.2 10.3h6.2"/>`, "cc-command-icon"),
    "zoom-out": iconSvg(`<circle class="cc-stroke" cx="10.3" cy="10.3" r="6"/><path class="cc-stroke" d="m14.8 14.8 5.2 5.2"/><path class="cc-stroke" d="M7.2 10.3h6.2"/>`, "cc-command-icon"),
    fit: iconSvg(`<path class="cc-stroke" d="M4.5 9V4.5H9"/><path class="cc-stroke" d="M19.5 9V4.5H15"/><path class="cc-stroke" d="M4.5 15v4.5H9"/><path class="cc-stroke" d="M19.5 15v4.5H15"/><rect class="cc-stroke" x="8.1" y="8.1" width="7.8" height="7.8"/>`, "cc-command-icon"),
    select: selectModeIconSpec("box").svg,
    text: iconSvg(`<path class="cc-stroke" d="M7.5 19 12 5.1 16.5 19"/><path class="cc-stroke" d="M9 14.1h6"/>`, "cc-tool-icon"),
    arrow: straightArrowSvg(),
    shape: iconSvg(`<rect class="cc-shape cc-empty-fill" x="5.5" y="5.5" width="10.2" height="10.2"/><circle class="cc-shape cc-empty-fill" cx="16.8" cy="16.8" r="3.45"/>`, "cc-tool-icon"),
    "tlc-plate": iconSvg(`<rect class="cc-shape cc-empty-fill" x="5.3" y="4.8" width="13.4" height="14.2"/><path class="cc-shape" d="M5.3 8.5h13.4M5.3 15.6h13.4" stroke-dasharray="1.5 1.5"/><circle class="cc-shape-fill" cx="8.9" cy="12.9" r="1.2"/><circle class="cc-shape-fill" cx="12" cy="10.9" r="1.2"/><circle class="cc-shape-fill" cx="15.1" cy="14.1" r="1.2"/>`, "cc-tool-icon"),
    orbital: iconSvg(`<path class="cc-shape cc-empty-fill" d="M12 4c3.35 0 5.35 2.67 5.35 6.25 0 3.26-2 6.17-5.35 9.5-3.35-3.33-5.35-6.24-5.35-9.5C6.65 6.67 8.65 4 12 4Z"/><path class="cc-shape" d="M12 4c0 0 2 2.55 2 6.25S12 19.75 12 19.75"/>`, "cc-tool-icon"),
  };
  return icons[name] || "";
}

function elementIconSvg() {
  return iconSvg(`
    <text class="cc-element-icon-text" x="12" y="16.5" text-anchor="middle">P</text>
  `, "cc-tool-icon cc-element-icon");
}

export function syncPrimaryChromeIcons(root = document) {
  for (const button of root.querySelectorAll(".icon-button[data-command]")) {
    const svg = commandIconSvg(button.dataset.command);
    if (svg) {
      button.innerHTML = svg;
    }
  }
  const deleteToolButton = root.querySelector('.icon-button[data-tool="delete"]');
  const deleteSvg = commandIconSvg("delete");
  if (deleteToolButton && deleteSvg) {
    deleteToolButton.innerHTML = deleteSvg;
  }
  for (const [tool, svg] of [
    ["select", selectModeIconSpec("box").svg],
    ["text", commandIconSvg("text")],
    ["arrow", arrowIconSvg("solid")],
    ["bracket", generatedBracketIconSvg("round")],
    ["element", elementIconSvg()],
    ["shape", commandIconSvg("shape")],
    ["tlc-plate", commandIconSvg("tlc-plate")],
    ["orbital", commandIconSvg("orbital")],
    ["templates", generatedRingSvg(6)],
  ]) {
    const button = root.querySelector(`.tool-button[data-tool="${tool}"]`);
    if (button && svg) {
      button.innerHTML = svg;
    }
  }
}

export function renderSecondaryToolbarHtml(editorState) {
  if (editorState.activeTool === "bond") {
    return bondToolbarHtml(editorState);
  }
  if (editorState.activeTool === "delete") {
    return "";
  }
  if (editorState.activeTool === "text") {
    return textToolbarHtml(editorState);
  }
  if (editorState.activeTool === "arrow") {
    return arrowToolbarHtml(editorState);
  }
  if (editorState.activeTool === "bracket") {
    return bracketToolbarHtml(editorState);
  }
  if (editorState.activeTool === "symbol") {
    return symbolToolbarHtml(editorState);
  }
  if (editorState.activeTool === "element") {
    return "";
  }
  if (editorState.activeTool === "shape") {
    return shapeToolbarHtml(editorState);
  }
  if (editorState.activeTool === "tlc-plate") {
    return tlcPlateToolbarHtml(editorState);
  }
  if (editorState.activeTool === "orbital") {
    return orbitalToolbarHtml(editorState);
  }
  if (editorState.activeTool === "templates") {
    return templatesToolbarHtml(editorState);
  }
  return selectToolbarHtml(editorState);
}

export function syncPrimaryToolButtons(editorState, root = document) {
  const activeTool = editorState.elementPlacementActive ? null : editorState.activeTool;
  root.querySelectorAll("[data-tool]").forEach((button) => {
    button.classList.toggle("is-active", button.dataset.tool === activeTool);
  });
  syncPrimarySelectToolButton(editorState, root);
  syncPrimaryBondToolButton(editorState, root);
  syncPrimaryArrowToolButton(editorState, root);
  syncPrimaryTemplateToolButton(editorState, root);
  syncPrimarySymbolToolButton(editorState, root);
  syncPrimaryElementToolButton(editorState, root);
  syncPrimaryShapeToolButton(editorState, root);
  syncPrimaryOrbitalToolButton(editorState, root);
}

function toolbarButton(value, title, svg, selected = false) {
  return `
    <button class="secondary-button${selected ? " is-selected" : ""}" type="button" data-secondary-value="${value}" aria-label="${title}" title="${title}">
      ${svg}
    </button>
  `;
}

function colorPickerControl(prefix, currentColor, palette = null) {
  const color = normalizedHex(currentColor) || "#000000";
  const colorPalette = normalizeToolbarColorPalette(palette);
  const swatches = colorPalette.colors
    .map((entry, index) => `
      <button class="color-panel-swatch${colorsEqual(color, entry.value) ? " is-selected" : ""}" type="button" data-color-swatch-value="${entry.value}" title="${escapeHtml(entry.title)}" aria-label="${escapeHtml(entry.title)}" style="--swatch:${entry.value}; --swatch-index:${index}"></button>
    `)
    .join("");
  return `
    <div class="color-picker" data-color-prefix="${prefix}">
      <button class="color-picker-button" type="button" data-secondary-value="${prefix}-apply" aria-label="Apply color" title="Apply color">
        <span class="color-picker-swatch" style="--swatch:${color}"></span>
        <span class="color-picker-arrow" data-color-picker-arrow aria-hidden="true"></span>
      </button>
      <div class="color-picker-panel" role="menu">
        <div class="color-panel-grid">
          ${swatches}
        </div>
        <button class="color-panel-other" type="button" data-color-other>${escapeHtml(colorPalette.otherLabel)}</button>
      </div>
    </div>
  `;
}

function colorsEqual(left, right) {
  return String(left || "").toLowerCase() === String(right || "").toLowerCase();
}

function normalizedHex(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(raw)) {
    return raw;
  }
  if (/^#[0-9a-f]{3}$/.test(raw)) {
    return `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`;
  }
  return null;
}

function normalizeToolbarColorPalette(palette) {
  const payload = typeof palette === "string" ? safeJsonParse(palette, null) : palette;
  const colors = (payload?.colors || [])
    .map((entry) => ({
      value: normalizedHex(entry?.value),
      title: String(entry?.title || entry?.value || ""),
    }))
    .filter((entry) => entry.value);
  return {
    colors,
    otherLabel: String(payload?.otherLabel || "Other..."),
  };
}

function secondaryDivider() {
  return `<span class="secondary-divider" aria-hidden="true"></span>`;
}

export const BOND_TOOL_ICON_TYPES = [
  "single",
  "double",
  "triple",
  "dashed",
  "dashed-double",
  "bold",
  "bold-dashed",
  "wedge",
  "hashed-wedge",
  "hollow-wedge",
  "wavy",
];

export const TEXT_FORMAT_ICON_TYPES = [
  "bold",
  "italic",
  "underline",
  "chemical",
  "subscript",
  "superscript",
];

const BOND_TOOL_ICON_SPECS = {
  single: {
    title: "Single bond",
  },
  double: {
    title: "Double bond",
  },
  triple: {
    title: "Triple bond",
  },
  dashed: {
    title: "Dashed bond",
  },
  "dashed-double": {
    title: "Dashed-solid double bond",
  },
  bold: {
    title: "Bold bond",
  },
  "bold-dashed": {
    title: "Hash bond",
  },
  wedge: {
    title: "Solid wedge",
  },
  "hashed-wedge": {
    title: "Hash wedge",
  },
  "hollow-wedge": {
    title: "Hollow wedge",
  },
  wavy: {
    title: "Wavy bond",
  },
};

function bondToolIconSpec(type = "single", editorState = null) {
  const normalizedType = BOND_TOOL_ICON_SPECS[type] ? type : "single";
  const spec = BOND_TOOL_ICON_SPECS[normalizedType] || BOND_TOOL_ICON_SPECS.single;
  return {
    ...spec,
    svg: editorState?.bondIconSvgs?.[normalizedType] || "",
  };
}

function syncPrimaryBondToolButton(editorState, root) {
  const bondButton = root.querySelector('.tool-button[data-tool="bond"]');
  if (!bondButton) {
    return;
  }
  const spec = bondToolIconSpec(editorState.bondType, editorState);
  if (spec.svg) {
    bondButton.innerHTML = spec.svg;
  }
  bondButton.setAttribute("aria-label", spec.title);
  bondButton.setAttribute("title", spec.title);
}

function syncPrimaryArrowToolButton(editorState, root) {
  const arrowButton = root.querySelector('.tool-button[data-tool="arrow"]');
  if (!arrowButton) {
    return;
  }
  arrowButton.innerHTML = currentArrowIconSvg(editorState);
  arrowButton.setAttribute("aria-label", currentArrowTitle(editorState));
  arrowButton.setAttribute("title", currentArrowTitle(editorState));
}

function syncPrimarySelectToolButton(editorState, root) {
  const selectButton = root.querySelector('.tool-button[data-tool="select"]');
  if (!selectButton) {
    return;
  }
  const spec = selectModeIconSpec(editorState.selectMode || "box");
  selectButton.innerHTML = spec.svg;
  selectButton.setAttribute("aria-label", spec.title);
  selectButton.setAttribute("title", spec.title);
}

function syncPrimaryTemplateToolButton(editorState, root) {
  const templateButton = root.querySelector('.tool-button[data-tool="templates"]');
  if (!templateButton) {
    return;
  }
  const spec = templateIconSpec(editorState.template);
  templateButton.innerHTML = spec.svg;
  templateButton.setAttribute("aria-label", spec.title);
  templateButton.setAttribute("title", spec.title);
}

function syncPrimarySymbolToolButton(editorState, root) {
  const symbolButton = root.querySelector('.tool-button[data-tool="symbol"]');
  if (!symbolButton) {
    return;
  }
  const kind = SYMBOL_TOOL_ICON_TYPES.includes(editorState.symbolKind) ? editorState.symbolKind : "circle-plus";
  symbolButton.innerHTML = symbolIconSvg(kind, editorState);
}

function syncPrimaryElementToolButton(editorState, root) {
  const elementButton = root.querySelector('.tool-button[data-tool="element"]');
  if (!elementButton) {
    return;
  }
  elementButton.innerHTML = elementIconSvg();
  elementButton.setAttribute("aria-label", "Element");
  elementButton.setAttribute("title", "Element");
}

function syncPrimaryShapeToolButton(editorState, root) {
  const shapeButton = root.querySelector('.tool-button[data-tool="shape"]');
  if (!shapeButton) {
    return;
  }
  const kind = SHAPE_TOOL_ICON_KINDS.includes(editorState.shapeKind) ? editorState.shapeKind : "circle";
  const style = SHAPE_TOOL_STYLE_KINDS.includes(kind) && SHAPE_TOOL_ICON_STYLES.includes(editorState.shapeStyle)
    ? editorState.shapeStyle
    : "solid";
  const title = SHAPE_TOOL_STYLE_KINDS.includes(kind)
    ? `${SHAPE_KIND_TITLES[kind]} ${SHAPE_STYLE_TITLES[style]}`
    : SHAPE_KIND_TITLES[kind];
  shapeButton.innerHTML = shapeIconSvg(kind, style, editorState);
  shapeButton.setAttribute("aria-label", title);
  shapeButton.setAttribute("title", title);
}

function syncPrimaryOrbitalToolButton(editorState, root) {
  const orbitalButton = root.querySelector('.tool-button[data-tool="orbital"]');
  if (!orbitalButton) {
    return;
  }
  orbitalButton.innerHTML = orbitalGlyphSvg(
    editorState.orbitalTemplate || "s",
    editorState.orbitalStyle || "hollow",
    editorState.orbitalPhase || "plus",
    editorState,
  );
}

function selectToolbarHtml(editorState) {
  const mode = editorState.selectMode;
  const free = selectModeIconSpec("free");
  const box = selectModeIconSpec("box");
  return [
    toolbarButton("select-free", free.title, free.svg, mode === "free"),
    toolbarButton("select-box", box.title, box.svg, mode === "box"),
    secondaryDivider(),
    toolbarButton("align-left", "Align left", arrangeIconSvg("align-left")),
    toolbarButton("align-right", "Align right", arrangeIconSvg("align-right")),
    toolbarButton("align-top", "Align top", arrangeIconSvg("align-top")),
    toolbarButton("align-bottom", "Align bottom", arrangeIconSvg("align-bottom")),
    toolbarButton("align-h-center", "Horizontal center", arrangeIconSvg("align-h-center")),
    toolbarButton("align-v-center", "Vertical center", arrangeIconSvg("align-v-center")),
    secondaryDivider(),
    toolbarButton("distribute-v", "Vertical distribute", distributeIconSvg("vertical")),
    toolbarButton("distribute-h", "Horizontal distribute", distributeIconSvg("horizontal")),
    secondaryDivider(),
    toolbarButton("flip-h", "Flip horizontal", arrangeIconSvg("flip-h")),
    toolbarButton("flip-v", "Flip vertical", arrangeIconSvg("flip-v")),
    secondaryDivider(),
    colorPickerControl("selection-color", editorState.selectionColor || editorState.textColor, editorState.colorPalette),
  ].join("");
}

function bondToolbarHtml(editorState) {
  const type = editorState.bondType;
  return BOND_TOOL_ICON_TYPES
    .map((value) => {
      const spec = bondToolIconSpec(value, editorState);
      return toolbarButton(`bond-${value}`, spec.title, spec.svg, type === value);
    })
    .join("");
}

function arrowIconSvg(type = "solid", editorState = null) {
  const cached = editorState?.arrowIconSvgs?.[type];
  if (cached) {
    return cached;
  }
  const icon = KERNEL_ARROW_ICONS[type] || KERNEL_ARROW_ICONS.solid;
  return kernelArrowIconSvg(icon.viewBox, icon.body);
}

function kernelArrowIconSvg(viewBox, body) {
  return `<svg class="chemcore-icon cc-arrow-icon cc-kernel-arrow-icon" viewBox="${viewBox}" aria-hidden="true">${body}</svg>`;
}

function currentArrowIconSvg(editorState) {
  const type = editorState?.arrowType || "solid";
  if (isCurvedArrowType(type)) {
    return currentCurvedArrowIconSvg(editorState, type);
  }
  if (isOpenArrowType(type)) {
    return currentOpenArrowIconSvg(editorState, type);
  }
  if (isEquilibriumArrowType(type)) {
    return currentEquilibriumArrowIconSvg(editorState);
  }
  if (type !== "solid") {
    return arrowIconSvg(type);
  }
  if (isNoGoArrowState(editorState, "cross")) {
    return arrowIconSvg("nogo-cross");
  }
  if (isNoGoArrowState(editorState, "hash")) {
    return arrowIconSvg("nogo-hash");
  }
  const size = normalizedArrowIconSize(editorState.arrowHeadSize);
  const headStyle = normalizedArrowHeadStyle(editorState.arrowHeadStyle);
  if (headStyle === "left") {
    return arrowIconSvg(`size-${size}-head-left`);
  }
  if (headStyle === "right") {
    return arrowIconSvg(`size-${size}-head-right`);
  }
  return arrowIconSvg(`size-${size}`);
}

function currentCurvedArrowIconSvg(editorState, type = "curved") {
  const prefix = type === "curved-mirror" ? "curve-mirror" : "curve";
  const curve = normalizedArrowCurve(editorState?.arrowCurve);
  const headStyle = normalizedArrowHeadStyle(editorState?.arrowHeadStyle);
  if (headStyle === "left" || headStyle === "right") {
    return arrowIconSvg(`${prefix}-${curve}-head-${headStyle}`);
  }
  return arrowIconSvg(`${prefix}-${curve}`);
}

function currentOpenArrowIconSvg(editorState, type = "hollow") {
  return arrowIconSvg(`${type}-${normalizedOpenArrowIconSize(editorState?.arrowHeadSize)}`);
}

function currentEquilibriumArrowIconSvg(editorState) {
  const type = editorState?.arrowType === "unequal-equilibrium" ? "unequal-equilibrium" : "equilibrium";
  return arrowIconSvg(`${type}-${normalizedArrowIconSize(editorState?.arrowHeadSize)}`, editorState);
}

function currentArrowTitle(editorState) {
  const type = editorState?.arrowType || "solid";
  if (isOpenArrowType(type)) {
    const size = normalizedOpenArrowIconSize(editorState?.arrowHeadSize);
    const label = type === "open" ? "open hollow arrow" : "hollow arrow";
    return `${size === "large" ? "Large" : "Small"} ${label}`;
  }
  if (isEquilibriumArrowType(type)) {
    const label = type === "unequal-equilibrium" ? "unequal equilibrium arrow" : "equilibrium arrow";
    return `${ARROW_SIZE_TITLES[editorState.arrowHeadSize] || "Small"} ${label}`;
  }
  if (type !== "solid") {
    return ARROW_TYPE_TITLES[type] || "Arrow";
  }
  if (isNoGoArrowState(editorState, "cross")) {
    return "Cross arrow";
  }
  if (isNoGoArrowState(editorState, "hash")) {
    return "Double slash arrow";
  }
  if (editorState.arrowHeadStyle === "left") {
    return "Head left half arrow";
  }
  if (editorState.arrowHeadStyle === "right") {
    return "Head right half arrow";
  }
  return ARROW_SIZE_TITLES[editorState.arrowHeadSize] || "Solid arrow";
}

function normalizedArrowIconSize(size) {
  return size === "large" || size === "medium" || size === "small" ? size : "small";
}

function normalizedArrowCurve(curve) {
  return curve === "270" || curve === "180" || curve === "120" || curve === "90" ? curve : "270";
}

function normalizedArrowHeadStyle(style) {
  return style === "left" || style === "right" ? style : "full";
}

function isNoGoArrowState(editorState, kind) {
  return (editorState?.arrowType || "solid") === "solid" && editorState?.arrowNoGo === kind;
}

function normalizedOpenArrowIconSize(size) {
  return size === "large" ? "large" : "small";
}

const ARROW_TYPE_TITLES = {
  solid: "Solid arrow",
  curved: "Curved arrow",
  "curved-mirror": "Mirrored curved arrow",
  hollow: "Hollow arrow",
  open: "Open hollow arrow",
  equilibrium: "Equilibrium arrow",
  "unequal-equilibrium": "Unequal equilibrium arrow",
};

const ARROW_SIZE_TITLES = {
  large: "Large arrow head",
  medium: "Medium arrow head",
  small: "Small arrow head",
};

const KERNEL_ARROW_ICONS = {
  solid: {
    viewBox: "32 9.45 36 21.1",
    body: `
      <polyline points="40,20 51.25,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20 C 60,20 50,22.55 50,22.55 C 50,22.55 51.25,21.115625 51.25,20 C 51.25,18.884375 50,17.45 50,17.45 C 50,17.45 60,20 60,20" fill="currentColor" stroke="none"/>
    `,
  },
  "nogo-cross": {
    viewBox: "32 7 36 26",
    body: `
      <polyline points="40,20 51.25,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <line x1="39" y1="15" x2="49" y2="25" stroke="currentColor" stroke-width="1"/>
      <line x1="39" y1="25" x2="49" y2="15" stroke="currentColor" stroke-width="1"/>
      <path d="M 60,20 C 60,20 50,22.55 50,22.55 C 50,22.55 51.25,21.115625 51.25,20 C 51.25,18.884375 50,17.45 50,17.45 C 50,17.45 60,20 60,20" fill="currentColor" stroke="none"/>
    `,
  },
  "nogo-hash": {
    viewBox: "32 7 36 26",
    body: `
      <polyline points="40,20 51.25,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <line x1="39" y1="25" x2="44" y2="15" stroke="currentColor" stroke-width="1"/>
      <line x1="44" y1="25" x2="49" y2="15" stroke="currentColor" stroke-width="1"/>
      <path d="M 60,20 C 60,20 50,22.55 50,22.55 C 50,22.55 51.25,21.115625 51.25,20 C 51.25,18.884375 50,17.45 50,17.45 C 50,17.45 60,20 60,20" fill="currentColor" stroke="none"/>
    `,
  },
  "equilibrium-small": {
    viewBox: "40 16 20 8",
    body: `
      <polyline points="40,18.5 52.916666666666664,18.5" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19 L 50,16 C 51.9875,17.68 51.25,19 51.25,19 Z" fill="currentColor" stroke="none"/>
      <polyline points="60,21.5 47.083333333333336,21.5" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 40,21 L 50,24 C 48.0125,22.32 48.75,21 48.75,21 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "equilibrium-medium": {
    viewBox: "40 14.75 20 10.5",
    body: `
      <polyline points="40,18.5 51,18.5" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19 L 45,14.75 C 47.973299999999995,17.48 46.87,19 46.87,19 Z" fill="currentColor" stroke="none"/>
      <polyline points="60,21.5 49,21.5" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 40,21 L 55,25.25 C 52.026700000000005,22.52 53.13,21 53.13,21 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "equilibrium-large": {
    viewBox: "37.5 12.870000000000001 25 14.259999999999998",
    body: `
      <polyline points="40,18.5 51,18.5" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19 L 37.5,12.870000000000001 C 41.9679,17.1792 40.31,19 40.31,19 Z" fill="currentColor" stroke="none"/>
      <polyline points="60,21.5 49,21.5" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 40,21 L 62.5,27.13 C 58.0321,22.8208 59.69,21 59.69,21 Z" fill="currentColor" stroke="none"/>
    `,
  },
  curved: {
    viewBox: "31.99589 3.843403 36.00411 24.156597",
    body: `
      <path d="M 39.99588959803465 19.997628298065987 C 42.66825955045352 15.366137912591348, 48.170293994946036 13.1744872955324, 53.29522790667578 14.700041868052745" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 50.54376469477005,15.866948602772691 53.09685800890133,14.623278819695944 53.67762503986155,11.84340298510375" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curved-mirror": {
    viewBox: "31.998055 12 36.001945 24.151439",
    body: `
      <path d="M 39.99805512887619 20.00112219063844 C 42.67160319867403 24.626634414992118, 48.17282685527247 26.814501086159773, 53.29651300391359 25.289991463899685" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 53.670976422641104,28.151439170921613 53.09247623745835,25.37109071510829 50.54039783440656,24.125339606468764" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-270": {
    viewBox: "27.867844 -12.139959 46.968737 40.139959",
    body: `
      <path d="M 40.00151011402222 19.998489885977783 C 35.26879238258676 15.26577215454233, 34.502560824665906 7.863130032569336, 38.16558081626419 2.26138755437218 C 41.82860080786247 -3.3403549238249752, 48.91736189243155 -5.606560446777749, 55.15060841716541 -3.168554701751626 C 61.383854941899266 -0.7305489567255026, 65.05394455699877 5.743758991117373, 63.94432746998286 12.344212321843182" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 62.29983173205632,9.939519196170254 63.9971808595139,12.216360415825015 66.8365816611183,12.269304611429783" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-180": {
    viewBox: "32 1.869793 36 26.130207",
    body: `
      <path d="M 40 20 C 40 16.11799066414049, 42.246695583812155 12.58675056984978, 45.76303622535644 10.941957276963295 C 49.27937686690072 9.29716398407681, 53.42990601096564 9.83605680655179, 56.40962759447164 12.324280222663955" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 53.41947358797239,12.050209302088067 56.25908916593267,12.090000876639891 58.029873077016575,9.869792701660256" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-120": {
    viewBox: "31.99589 3.843403 36.00411 24.156597",
    body: `
      <path d="M 39.99588959803465 19.997628298065987 C 42.66825955045352 15.366137912591348, 48.170293994946036 13.1744872955324, 53.29522790667578 14.700041868052745" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 50.54376469477005,15.866948602772691 53.09685800890133,14.623278819695944 53.67762503986155,11.84340298510375" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-90": {
    viewBox: "32.00151 5.187232 35.99849 22.814278",
    body: `
      <path d="M 40.00151011402222 20.001510114022217 C 43.23549540723127 16.767524820813165, 47.83395622558416 15.297443211444504, 52.344212321843195 16.05567253001714" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 49.932491886364346,17.73112794942502 52.204100820969785,16.02678266509419 52.248309990138026,13.187232427933123" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-mirror-270": {
    viewBox: "27.867844 12 46.968737 40.139959",
    body: `
      <path d="M 40.00151011402222 20.001510114022217 C 35.26879238258677 24.734227845457667, 34.502560824665906 32.13686996743067, 38.16558081626419 37.73861244562782 C 41.82860080786247 43.340354923824975, 48.91736189243155 45.60656044677775, 55.15060841716541 43.168554701751624 C 61.383854941899266 40.7305489567255, 65.05394455699877 34.256241008882625, 63.94432746998286 27.65578767815682" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 66.83658166111832,27.730695388570208 63.99718085951391,27.783639584174978 62.29983173205633,30.060480803829744" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-mirror-180": {
    viewBox: "32 12 36 26.130207",
    body: `
      <path d="M 40 20 C 40 23.882009335859514, 42.24669558381217 27.413249430150223, 45.76303622535645 29.058042723036706 C 49.27937686690073 30.70283601592319, 53.42990601096566 30.163943193448205, 56.40962759447166 27.67571977733603" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 58.02987307701658,30.130207298339748 56.25908916593268,27.909999123360112 53.4194735879724,27.949790697911936" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-mirror-120": {
    viewBox: "31.998055 12 36.001945 24.151439",
    body: `
      <path d="M 39.99805512887619 20.00112219063844 C 42.67160319867403 24.626634414992118, 48.17282685527247 26.814501086159773, 53.29651300391359 25.289991463899685" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 53.670976422641104,28.151439170921613 53.09247623745835,25.37109071510829 50.54039783440656,24.125339606468764" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-mirror-90": {
    viewBox: "32.00151 11.99849 35.99849 22.814278",
    body: `
      <path d="M 40.00151011402222 19.998489885977783 C 43.23549540723127 23.232475179186835, 47.83395622558416 24.702556788555494, 52.344212321843195 23.94432746998286" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <polygon points="60,20 52.248309990138026,26.81276757206688 52.204100820969785,23.97321733490581 49.932491886364346,22.26887205057498" fill="currentColor" stroke="none" stroke-width="0"/>
    `,
  },
  "curve-270-head-left": {
    viewBox: "27.869337 -12.139719 46.922767 40.138208",
    body: `
      <path d="M 40.00151011402222 19.998489885977783 C 35.12081528654369 15.117795058499254, 34.47713878384104 7.426530811954529, 38.478699351595374 1.802486269052599 C 42.480259919349706 -3.821558273849331, 49.957547499102176 -5.734680225003585, 56.168476673586184 -2.723580295161545 C 62.37940584807019 0.28751963468049446, 65.50874877831018 7.342806870170325, 63.57188051617641 13.967828027350901" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.55522059519,19.771589665170634 L 66.7921037206373,12.246463577946846 C 64.38971383952898,13.247002987039926 63.5524014547039,11.987950080995649 63.5524014547039,11.987950080995649 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-270-head-right": {
    viewBox: "27.869937 -12.139346 44.267706 40.39084",
    body: `
      <path d="M 40.00151011402222 19.998489885977783 C 35.11338872567548 15.110368497631043, 34.476118874038455 7.404585543416881, 38.49482626204815 1.779867564354996 C 42.513533650057845 -3.8448504147068894, 50.00993920756348 -5.739344415399859, 56.218421960681106 -2.6992451869754444 C 62.426904713798734 0.34085404144897113, 65.52714668983475 7.424193895732915, 63.548259564808355 14.047747863266547" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.43214695359709,20.251493559553225 L 62.295309138463246,12.079674504233608 C 62.584569942148654,14.087665586746493 63.95305678734225,14.201436209193922 63.95305678734225,14.201436209193922 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-180-head-left": {
    viewBox: "32 1.891169 35.548 26.322597",
    body: `
      <path d="M 40 20 C 40 15.814767507612288, 42.606289242751934 12.072250492511003, 46.53174261975337 10.620704144533835 C 50.45719599675481 9.169157796556668, 54.8713416197531 10.31567235999323, 57.59423814887296 13.49403758555257" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.548000050093705,20.213766333375276 L 57.98467308202594,9.891169334997782 C 57.315674425507524,12.406124016016221 55.80708921602638,12.303767210015167 55.80708921602638,12.303767210015167 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-180-head-right": {
    viewBox: "32 2.010044 36.468404 25.989956",
    body: `
      <path d="M 40 20 C 40 15.799504396853274, 42.62504964505324 12.046744655543517, 46.570854999634214 10.60633380588461 C 50.51666035421519 9.165922956225703, 54.942045172477684 10.344931236638189, 57.648192913293265 13.55755130706882" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.468404102961316,19.825078313725818 L 55.32763660776779,13.205221097715661 C 57.06426590958222,14.253943495721707 58.01950049512274,13.267420872267378 58.01950049512274,13.267420872267378 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-120-head-left": {
    viewBox: "31.99589 3.88285 35.696869 24.511616",
    body: `
      <path d="M 39.99588959803465 19.997628298065987 C 42.98348613063975 14.819818016254727, 49.42827348471593 12.777268141465182, 54.852915374358105 15.288983237809473" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.69275878969691,20.39446525663421 L 53.64690091883124,11.882849510767171 C 54.18256984733385,14.429536584012896 52.78961679859824,15.017744076330153 52.78961679859824,15.017744076330153 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-120-head-right": {
    viewBox: "31.99589 6.23116 36.341639 21.766469",
    body: `
      <path d="M 39.99588959803465 19.997628298065987 C 42.9994725567528 14.792111905140253, 49.493105806470226 12.759832061672437, 54.92841284433516 15.324271359268796" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.33752901562428,19.631117683248835 L 52.74776686948425,16.075065017016232 C 54.771385756126136,16.218823848488494 55.17317658110797,14.905711464508961 55.17317658110797,14.905711464508961 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-90-head-left": {
    viewBox: "32.00151 5.23178 35.771449 23.2137",
    body: `
      <path d="M 40.00151011402222 20.001510114022217 C 43.65458848847809 16.348431739566344, 49.00916489124329 14.978421886929139, 53.96782802735092 16.4281194838236" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.772959009433954,20.445479953087442 L 52.225605891081415,13.231780423241867 C 53.233530976302085,15.631081003115701 51.97705983040374,16.47226261818163 51.97705983040374,16.47226261818163 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-90-head-right": {
    viewBox: "32.00151 7.860066 36.250246 20.141444",
    body: `
      <path d="M 40.00151011402222 20.001510114022217 C 43.67571483228217 16.327305395762263, 49.069088107152666 14.964290383519543, 54.04774786326656 16.451740435191645" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.251756431714206,19.568006135355454 L 52.08107243883043,17.699872551150918 C 54.08923913631989,17.41183346509783 54.20384232669055,16.04341609135659 54.20384232669055,16.04341609135659 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-270-head-left": {
    viewBox: "27.869937 11.748506 44.267706 40.39084",
    body: `
      <path d="M 40.00151011402222 20.001510114022217 C 35.11338872567548 24.889631502368957, 34.476118874038455 32.59541445658312, 38.49482626204815 38.220132435645006 C 42.513533650057845 43.84485041470689, 50.00993920756348 45.73934441539986, 56.218421960681106 42.699245186975446 C 62.426904713798734 39.65914595855103, 65.52714668983475 32.57580610426709, 63.548259564808355 25.952252136733453" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.43214695359709,19.748506440446775 L 62.295309138463246,27.920325495766388 C 62.584569942148654,25.912334413253504 63.953056787342256,25.798563790806075 63.953056787342256,25.798563790806075 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-270-head-right": {
    viewBox: "27.869337 12.00151 46.922767 40.138208",
    body: `
      <path d="M 40.00151011402222 20.001510114022217 C 35.12081528654369 24.882204941500746, 34.47713878384104 32.57346918804547, 38.478699351595374 38.1975137309474 C 42.480259919349706 43.82155827384933, 49.957547499102176 45.734680225003586, 56.168476673586184 42.72358029516155 C 62.37940584807019 39.71248036531951, 65.50874877831018 32.65719312982967, 63.57188051617641 26.0321719726491" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.55522059519,20.228410334829366 L 66.79210372063731,27.753536422053145 C 64.389713839529,26.75299701296007 63.55240145470391,28.012049919004344 63.55240145470391,28.012049919004344 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-180-head-left": {
    viewBox: "32 12 36.468404 25.989956",
    body: `
      <path d="M 40 20 C 40 24.200495603146727, 42.625049645053245 27.95325534445649, 46.57085499963422 29.393666194115394 C 50.5166603542152 30.834077043774297, 54.942045172477705 29.6550687633618, 57.64819291329328 26.442448692931162" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.468404102961316,20.174921686274182 L 55.3276366077678,26.79477890228434 C 57.064265909582225,25.746056504278297 58.01950049512275,26.73257912773262 58.01950049512275,26.73257912773262 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-180-head-right": {
    viewBox: "32 11.786234 35.548 26.322597",
    body: `
      <path d="M 40 20 C 40 24.185232492387716, 42.606289242751934 27.927749507489004, 46.53174261975338 29.379295855466168 C 50.45719599675483 30.830842203443332, 54.87134161975311 29.684327640006764, 57.59423814887297 26.505962414447417" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.548000050093705,19.786233666624724 L 57.98467308202595,30.10883066500222 C 57.31567442550753,27.593875983983782 55.807089216026384,27.696232789984837 55.807089216026384,27.696232789984837 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-120-head-left": {
    viewBox: "31.998055 12.001122 36.339154 21.757628",
    body: `
      <path d="M 39.99805512887619 20.00112219063844 C 43.003053220778575 25.200076929950452, 49.49580743786399 27.228513151746533, 54.92980240340954 24.66602956487209" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.337209134459584,20.369174754875 L 52.74436738416164,23.918647131853355 C 54.7681101235023,23.77664246655688 55.168762566209566,25.090102637309194 55.168762566209566,25.090102637309194 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-120-head-right": {
    viewBox: "31.998055 11.605284 35.695025 24.506683",
    body: `
      <path d="M 39.99805512887619 20.00112219063844 C 42.98705468583819 25.172397824504593, 49.43096469710251 27.211105718360507, 54.85429793157225 24.701302581418616" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.69308053056524,19.605284356426193 L 53.640284475697634,28.11196760656423 C 54.17802974160273,25.565718153153068 52.78555676802359,24.976375071534484 52.78555676802359,24.976375071534484 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-90-head-left": {
    viewBox: "32.00151 11.99849 36.250246 20.141444",
    body: `
      <path d="M 40.00151011402222 19.998489885977783 C 43.67571483228217 23.672694604237737, 49.069088107152666 25.035709616480457, 54.04774786326656 23.548259564808355" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 60.251756431714206,20.431993864644546 L 52.08107243883043,22.300127448849082 C 54.08923913631989,22.58816653490217 54.20384232669055,23.95658390864341 54.20384232669055,23.95658390864341 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "curve-mirror-90-head-right": {
    viewBox: "32.00151 11.55452 35.771449 23.2137",
    body: `
      <path d="M 40.00151011402222 19.998489885977783 C 43.65458848847809 23.651568260433656, 49.00916489124329 25.02157811307086, 53.96782802735092 23.5718805161764" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="round"/>
      <path d="M 59.772959009433954,19.554520046912558 L 52.225605891081415,26.768219576758135 C 53.233530976302085,24.368918996884297 51.97705983040374,23.52773738181837 51.97705983040374,23.52773738181837 Z" fill="currentColor" stroke="none"/>
    `,
  },
  hollow: {
    viewBox: "32 6 36 28",
    body: `
      <polygon points="40,14 51,14 51,8 60,20 51,32 51,26 40,26" fill="none" stroke="currentColor" stroke-width="1"/>
    `,
  },
  "hollow-large": {
    viewBox: "32 6 36 28",
    body: `
      <polygon points="40,14 51,14 51,8 60,20 51,32 51,26 40,26" fill="none" stroke="currentColor" stroke-width="1"/>
    `,
  },
  "hollow-small": {
    viewBox: "32 12 36 16",
    body: `
      <polygon points="40,17 54,17 54,14 60,20 54,26 54,23 40,23" fill="none" stroke="currentColor" stroke-width="1"/>
    `,
  },
  open: {
    viewBox: "32 6 36 28",
    body: `
      <polyline points="40,26 55.5,26" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <polyline points="40,14 55.5,14" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <polyline points="55.5,26 51,32 60,20 51,8 55.5,14" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
    `,
  },
  "open-large": {
    viewBox: "32 6 36 28",
    body: `
      <polyline points="40,26 55.5,26" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <polyline points="40,14 55.5,14" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <polyline points="55.5,26 51,32 60,20 51,8 55.5,14" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
    `,
  },
  "open-small": {
    viewBox: "32 12 36 16",
    body: `
      <polyline points="40,23 57,23" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <polyline points="40,17 57,17" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <polyline points="57,23 54,26 60,20 54,14 57,17" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
    `,
  },
  "size-large": {
    viewBox: "29.5 6.32 38.5 27.36",
    body: `
      <polyline points="40,20 51,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20 C 60,20 37.5,25.68 37.5,25.68 C 37.5,25.68 40.31,22.485 40.31,20 C 40.31,17.515 37.5,14.32 37.5,14.32 C 37.5,14.32 60,20 60,20" fill="currentColor" stroke="none"/>
    `,
  },
  "size-medium": {
    viewBox: "32 8.2 36 23.6",
    body: `
      <polyline points="40,20 51,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20 C 60,20 45,23.8 45,23.8 C 45,23.8 46.87,21.6625 46.87,20 C 46.87,18.3375 45,16.2 45,16.2 C 45,16.2 60,20 60,20" fill="currentColor" stroke="none"/>
    `,
  },
  "size-small": {
    viewBox: "32 9.45 36 21.1",
    body: `
      <polyline points="40,20 51.25,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20 C 60,20 50,22.55 50,22.55 C 50,22.55 51.25,21.115625 51.25,20 C 51.25,18.884375 50,17.45 50,17.45 C 50,17.45 60,20 60,20" fill="currentColor" stroke="none"/>
    `,
  },
  "size-large-head-left": {
    viewBox: "29.5 6.37 38.5 22.13",
    body: `
      <polyline points="40,20 44.063333333333,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20.5 L 37.5,14.37 C 41.9679,18.6792 40.31,20.5 40.31,20.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "size-large-head-right": {
    viewBox: "29.5 11.5 38.5 22.13",
    body: `
      <polyline points="40,20 44.063333333333,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19.5 L 37.5,25.63 C 41.9679,21.3208 40.31,19.5 40.31,19.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "size-medium-head-left": {
    viewBox: "32 8.25 36 20.25",
    body: `
      <polyline points="40,20 49.37,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20.5 L 45,16.25 C 47.9733,18.98 46.87,20.5 46.87,20.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "size-medium-head-right": {
    viewBox: "32 11.5 36 20.25",
    body: `
      <polyline points="40,20 49.37,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19.5 L 45,23.75 C 47.9733,21.02 46.87,19.5 46.87,19.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "size-small-head-left": {
    viewBox: "32 9.5 36 19",
    body: `
      <polyline points="40,20 52.916666666667,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20.5 L 50,17.5 C 51.9875,19.18 51.25,20.5 51.25,20.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "size-small-head-right": {
    viewBox: "32 11.5 36 19",
    body: `
      <polyline points="40,20 52.916666666667,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19.5 L 50,22.5 C 51.9875,20.82 51.25,19.5 51.25,19.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "head-left": {
    viewBox: "32 9.5 36 19",
    body: `
      <polyline points="40,20 52.916666666666664,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,20.5 L 50,17.5 C 51.9875,19.18 51.25,20.5 51.25,20.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
  "head-right": {
    viewBox: "32 11.5 36 19",
    body: `
      <polyline points="40,20 52.916666666666664,20" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="butt" stroke-linejoin="miter"/>
      <path d="M 60,19.5 L 50,22.5 C 51.9875,20.82 51.25,19.5 51.25,19.5 Z" fill="currentColor" stroke="none"/>
    `,
  },
};

function isCurvedArrowType(type) {
  return type === "curved" || type === "curved-mirror";
}

function isOpenArrowType(type) {
  return type === "hollow" || type === "open";
}

function isEquilibriumArrowType(type) {
  return type === "equilibrium" || type === "unequal-equilibrium";
}

function arrowCurveSvg(curve, mirrored = false) {
  return curvedArrowSvg({ curve, mirrored });
}

function arrowSizeSvg(size) {
  const scale = size === "large" ? 1 : size === "small" ? 0.62 : 0.78;
  const tip = 20;
  const base = tip - 7 * scale;
  const half = 4.8 * scale;
  return iconSvg(`<path class="cc-arrow" d="M4 12h${Math.max(8, base - 4)}"/>${polygon([point(tip, 12), point(base, 12 - half), point(base, 12 + half)], "cc-arrow-fill")}`, "cc-arrow-icon");
}

function arrowEndpointSvg(label, side) {
  const isHead = side === "head";
  const head = isHead ? arrowHead(point(20.2, 10.8), point(1, 0), 0.82) : arrowHead(point(3.8, 10.8), point(-1, 0), 0.82);
  const body = isHead ? `<path class="cc-arrow" d="M4.4 10.8h11.4"/>` : `<path class="cc-arrow" d="M8.2 10.8h11.4"/>`;
  return iconSvg(`${body}${head}<text class="cc-icon-label" x="12" y="21.1" text-anchor="middle">${label}</text>`, "cc-arrow-icon cc-arrow-endpoint-icon");
}

function arrowHalfEndpointSvg(side, half) {
  const isHead = side === "head";
  const tipX = isHead ? 21 : 3;
  const baseX = isHead ? 15 : 9;
  const shaftStart = isHead ? 5 : 9;
  const shaftEnd = isHead ? 15 : 19;
  const head = half === "left"
    ? `<path class="cc-arrow-fill" d="M${tipX} 12 ${baseX} 12 ${baseX} 7.2z"/>`
    : `<path class="cc-arrow-fill" d="M${tipX} 12 ${baseX} 16.8 ${baseX} 12z"/>`;
  const topLabel = half === "left" ? "left" : "right";
  const bottomLabel = isHead ? "head" : "tail";
  return iconSvg(`<text class="cc-icon-label cc-icon-label-small" x="12" y="5.15" text-anchor="middle">${topLabel}</text><path class="cc-arrow" d="M${shaftStart} 12h${shaftEnd - shaftStart}"/>${head}<text class="cc-icon-label cc-icon-label-small" x="12" y="22.15" text-anchor="middle">${bottomLabel}</text>`, "cc-arrow-icon cc-arrow-endpoint-icon");
}

function arrowNoGoSvg(kind) {
  const headLength = 4.7 * 0.9;
  const center = point((4 + 20.5) * 0.5, 12);
  const lineClass = "cc-arrow cc-arrow-butt";
  let mark;
  if (kind === "hash") {
    const axis = unit(point(1, -2));
    const halfLength = headLength * Math.sqrt(5) * 0.25;
    const offset = headLength * 0.25;
    mark = [
      add(center, point(-offset, 0)),
      add(center, point(offset, 0)),
    ].map((markCenter) => linePath(
      add(markCenter, mul(axis, -halfLength)),
      add(markCenter, mul(axis, halfLength)),
      lineClass,
    )).join("");
  } else {
    const halfLength = headLength * Math.SQRT2 * 0.5;
    mark = [
      unit(point(1, 1)),
      unit(point(1, -1)),
    ].map((axis) => linePath(
      add(center, mul(axis, -halfLength)),
      add(center, mul(axis, halfLength)),
      lineClass,
    )).join("");
  }
  return iconSvg(`<path class="cc-arrow" d="M4 12h12"/>${arrowHead(point(20.5, 12), point(1, 0), 0.9)}${mark}`, "cc-arrow-icon");
}

function distributeIconSvg(axis = "horizontal") {
  if (axis === "vertical") {
    return iconSvg(`
      <path class="cc-guide" d="M5.3 5.2v13.6M18.7 5.2v13.6"/>
      <path class="cc-stroke-strong" d="M8 6.6h8"/>
      <path class="cc-stroke-strong" d="M8 12h8"/>
      <path class="cc-stroke-strong" d="M8 17.4h8"/>
    `, "cc-distribute-icon");
  }
  return iconSvg(`
    <path class="cc-guide" d="M5.2 5.3h13.6M5.2 18.7h13.6"/>
    <path class="cc-stroke-strong" d="M6.6 8v8"/>
    <path class="cc-stroke-strong" d="M12 8v8"/>
    <path class="cc-stroke-strong" d="M17.4 8v8"/>
  `, "cc-distribute-icon");
}

function arrowToolbarHtml(editorState) {
  const type = editorState.arrowType;
  const isCrossNoGo = isNoGoArrowState(editorState, "cross");
  const isHashNoGo = isNoGoArrowState(editorState, "hash");
  const hasNoGo = isCrossNoGo || isHashNoGo;
  const solidSelected = type === "solid" && !hasNoGo;
  const solidTypeIcon = solidSelected ? currentArrowIconSvg(editorState) : arrowIconSvg("solid");
  const curvedTypeIcon = currentCurvedArrowIconSvg(editorState, "curved");
  const mirroredCurvedTypeIcon = currentCurvedArrowIconSvg(editorState, "curved-mirror");
  const hollowTypeIcon = type === "hollow" ? currentOpenArrowIconSvg(editorState, "hollow") : arrowIconSvg("hollow-large");
  const openTypeIcon = type === "open" ? currentOpenArrowIconSvg(editorState, "open") : arrowIconSvg("open-large");
  const equilibriumTypeIcon = type === "equilibrium"
    ? currentEquilibriumArrowIconSvg(editorState)
    : arrowIconSvg("equilibrium-small", editorState);
  const unequalEquilibriumTypeIcon = type === "unequal-equilibrium"
    ? currentEquilibriumArrowIconSvg(editorState)
    : arrowIconSvg("unequal-equilibrium-small", editorState);
  const controls = [
    toolbarButton("arrow-type-solid", "Solid arrow", solidTypeIcon, solidSelected),
    toolbarButton("arrow-type-curved", "Curved arrow", curvedTypeIcon, type === "curved"),
    toolbarButton("arrow-type-curved-mirror", "Mirrored curved arrow", mirroredCurvedTypeIcon, type === "curved-mirror"),
    toolbarButton("arrow-type-hollow", "Hollow arrow", hollowTypeIcon, type === "hollow"),
    toolbarButton("arrow-type-open", "Open hollow arrow", openTypeIcon, type === "open"),
    toolbarButton("arrow-type-nogo-cross", "Cross arrow", arrowIconSvg("nogo-cross"), isCrossNoGo),
    toolbarButton("arrow-type-nogo-hash", "Double slash arrow", arrowIconSvg("nogo-hash"), isHashNoGo),
    toolbarButton("arrow-type-equilibrium", "Equilibrium arrow", equilibriumTypeIcon, type === "equilibrium"),
    toolbarButton("arrow-type-unequal-equilibrium", "Unequal equilibrium arrow", unequalEquilibriumTypeIcon, type === "unequal-equilibrium"),
  ];
  if (type === "solid" && !hasNoGo) {
    controls.push(
      secondaryDivider(),
      toolbarButton("arrow-size-large", "Large arrow head", arrowIconSvg("size-large")),
      toolbarButton("arrow-size-medium", "Medium arrow head", arrowIconSvg("size-medium")),
      toolbarButton("arrow-size-small", "Small arrow head", arrowIconSvg("size-small")),
      secondaryDivider(),
      toolbarButton("arrow-head-full", "Full arrow head", arrowIconSvg("solid")),
      toolbarButton("arrow-head-left", "Head left half arrow", arrowIconSvg("head-left")),
      toolbarButton("arrow-head-right", "Head right half arrow", arrowIconSvg("head-right")),
    );
  } else if (isOpenArrowType(type)) {
    controls.push(
      secondaryDivider(),
      toolbarButton("arrow-size-large", "Large arrow style", arrowIconSvg(`${type}-large`)),
      toolbarButton("arrow-size-small", "Small arrow style", arrowIconSvg(`${type}-small`)),
    );
  } else if (isEquilibriumArrowType(type)) {
    const label = type === "unequal-equilibrium" ? "unequal equilibrium arrow" : "equilibrium arrow";
    controls.push(
      secondaryDivider(),
      toolbarButton("arrow-size-small", `Small ${label}`, arrowIconSvg(`${type}-small`, editorState)),
      toolbarButton("arrow-size-medium", `Medium ${label}`, arrowIconSvg(`${type}-medium`, editorState)),
      toolbarButton("arrow-size-large", `Large ${label}`, arrowIconSvg(`${type}-large`, editorState)),
    );
  } else if (isCurvedArrowType(type)) {
    const prefix = type === "curved-mirror" ? "curve-mirror" : "curve";
    controls.push(
      secondaryDivider(),
      toolbarButton("arrow-curve-270", "Curve 270 degrees", arrowIconSvg(`${prefix}-270`)),
      toolbarButton("arrow-curve-180", "Curve 180 degrees", arrowIconSvg(`${prefix}-180`)),
      toolbarButton("arrow-curve-120", "Curve 120 degrees", arrowIconSvg(`${prefix}-120`)),
      toolbarButton("arrow-curve-90", "Curve 90 degrees", arrowIconSvg(`${prefix}-90`)),
      secondaryDivider(),
      toolbarButton("arrow-head-full", "Full arrow head", arrowIconSvg("solid")),
      toolbarButton("arrow-head-left", "Head left half arrow", arrowIconSvg("head-left")),
      toolbarButton("arrow-head-right", "Head right half arrow", arrowIconSvg("head-right")),
    );
  }
  return controls.join("");
}

function textToolbarHtml(editorState) {
  const fontOptions = TEXT_FONT_OPTIONS
    .map((fontFamily) => `<option value="${fontFamily}"${editorState.textFontFamily === fontFamily ? " selected" : ""}>${fontFamily}</option>`)
    .join("");
  const normalizedFontSize = normalizeToolbarFontSize(cssPxToPt(editorState.textFontSize));
  const knownFontSizes = new Set(TEXT_FONT_SIZE_OPTIONS);
  const fontSizeOptions = [
    ...TEXT_FONT_SIZE_OPTIONS,
    ...(knownFontSizes.has(normalizedFontSize) ? [] : [normalizedFontSize]),
  ]
    .sort((left, right) => left - right)
    .map((fontSize) => `<option value="${fontSize}"${normalizedFontSize === fontSize ? " selected" : ""}>${formatToolbarFontSize(fontSize)}</option>`)
    .join("");
  return `
    <select class="secondary-select" data-text-control="font" aria-label="Font family">${fontOptions}</select>
    <select class="secondary-select" data-text-control="size" aria-label="Font size">${fontSizeOptions}</select>
    ${secondaryDivider()}
    ${colorPickerControl("text-color", editorState.textColor, editorState.colorPalette)}
    ${secondaryDivider()}
    ${toolbarButton("text-align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h9"/><path d="M5 14h12"/><path d="M5 18h8"/></svg>`, editorState.textAlign === "left")}
    ${toolbarButton("text-align-center", "Align center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 10h10"/><path d="M6 14h12"/><path d="M8 18h8"/></svg>`, editorState.textAlign === "center")}
    ${toolbarButton("text-align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M10 10h9"/><path d="M7 14h12"/><path d="M11 18h8"/></svg>`, editorState.textAlign === "right")}
    ${toolbarButton("text-align-justify", "Justify", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h14"/><path d="M5 14h14"/><path d="M5 18h14"/></svg>`, editorState.textAlign === "justify")}
    ${secondaryDivider()}
    ${toolbarButton("text-bold", "Bold", textFormatIconSvg("bold", editorState), editorState.textBold)}
    ${toolbarButton("text-italic", "Italic", textFormatIconSvg("italic", editorState), editorState.textItalic)}
    ${toolbarButton("text-underline", "Underline", textFormatIconSvg("underline", editorState), editorState.textUnderline)}
    ${secondaryDivider()}
    ${toolbarButton("text-chemical", "Chemical", textFormatIconSvg("chemical", editorState), editorState.textScript === "chemical")}
    ${toolbarButton("text-subscript", "Subscript", textFormatIconSvg("subscript", editorState), editorState.textScript === "subscript")}
    ${toolbarButton("text-superscript", "Superscript", textFormatIconSvg("superscript", editorState), editorState.textScript === "superscript")}
  `;
}

function shapeToolbarHtml(editorState) {
  const activeKind = SHAPE_TOOL_ICON_KINDS.includes(editorState.shapeKind) ? editorState.shapeKind : "circle";
  const kindButtons = SHAPE_TOOL_ICON_KINDS
    .map((kind) => toolbarButton(
      `shape-kind-${kind}`,
      SHAPE_KIND_TITLES[kind],
      shapeIconSvg(kind, shapeStyleForKind(editorState, kind), editorState),
      editorState.shapeKind === kind,
    ))
    .join("");
  const styleButtons = SHAPE_TOOL_STYLE_KINDS.includes(activeKind)
    ? SHAPE_TOOL_ICON_STYLES
      .map((style) => toolbarButton(
        `shape-style-${activeKind}-${style}`,
        `${SHAPE_KIND_TITLES[activeKind]} ${SHAPE_STYLE_TITLES[style]}`,
        shapeIconSvg(activeKind, style, editorState),
        editorState.shapeStyle === style,
      ))
      .join("")
    : "";
  return `
    ${kindButtons}
    ${styleButtons ? `${secondaryDivider()}${styleButtons}` : ""}
    ${secondaryDivider()}
    ${colorPickerControl("shape-color", editorState.shapeColor, editorState.colorPalette)}
  `;
}

function shapeStyleForKind(editorState, kind) {
  if (!SHAPE_TOOL_STYLE_KINDS.includes(kind)) {
    return "solid";
  }
  const saved = editorState?.shapeStyleByKind?.[kind];
  if (SHAPE_TOOL_ICON_STYLES.includes(saved)) {
    return saved;
  }
  if (editorState?.shapeKind === kind && SHAPE_TOOL_ICON_STYLES.includes(editorState?.shapeStyle)) {
    return editorState.shapeStyle;
  }
  return "solid";
}

function tlcPlateToolbarHtml(editorState) {
  return `
    ${colorPickerControl("shape-color", editorState.shapeColor, editorState.colorPalette)}
  `;
}

function orbitalIconKey(template, style, phase) {
  return `${template}:${style}:${phase}`;
}

function orbitalComboValue(template, style, phase) {
  return `orbital-combo-${template}-${style}-${phase}`;
}

function orbitalGlyphSvg(template = "s", style = "hollow", phase = "plus", editorState = null) {
  const normalizedTemplate = ORBITAL_TOOL_ICON_TEMPLATES.includes(template) ? template : "s";
  const normalizedStyle = ORBITAL_TOOL_ICON_STYLES.includes(style) ? style : "hollow";
  const normalizedPhase = ORBITAL_TOOL_ICON_PHASES.includes(phase) ? phase : "plus";
  const cached = editorState?.orbitalIconSvgs?.[orbitalIconKey(normalizedTemplate, normalizedStyle, normalizedPhase)];
  if (cached) {
    return cached;
  }
  const filledClass = style === "filled" ? "cc-shape-fill" : style === "shaded" ? "cc-shape-soft-fill" : "cc-empty-fill";
  const secondaryFill = style === "hollow" ? "cc-empty-fill" : "cc-empty-fill";
  if (normalizedTemplate === "s") {
    return iconSvg(`<circle class="${filledClass} cc-shape" cx="12" cy="12" r="6.1"/>`, "cc-shape-icon");
  }
  if (normalizedTemplate === "oval") {
    return iconSvg(`<ellipse class="${filledClass} cc-shape" cx="12" cy="12" rx="7.1" ry="4.1"/>`, "cc-shape-icon");
  }
  if (normalizedTemplate === "p") {
    const topClass = phase === "plus" ? filledClass : secondaryFill;
    const bottomClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${topClass} cc-shape" cx="12" cy="8.1" rx="3.1" ry="4.2"/><ellipse class="${bottomClass} cc-shape" cx="12" cy="15.9" rx="3.1" ry="4.2"/>`, "cc-shape-icon");
  }
  if (normalizedTemplate === "dxy") {
    const primaryClass = phase === "plus" ? filledClass : secondaryFill;
    const secondaryClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${primaryClass} cc-shape" cx="8.4" cy="8.4" rx="2.2" ry="3.6" transform="rotate(-45 8.4 8.4)"/><ellipse class="${primaryClass} cc-shape" cx="15.6" cy="15.6" rx="2.2" ry="3.6" transform="rotate(-45 15.6 15.6)"/><ellipse class="${secondaryClass} cc-shape" cx="15.6" cy="8.4" rx="2.2" ry="3.6" transform="rotate(45 15.6 8.4)"/><ellipse class="${secondaryClass} cc-shape" cx="8.4" cy="15.6" rx="2.2" ry="3.6" transform="rotate(45 8.4 15.6)"/>`, "cc-shape-icon");
  }
  if (normalizedTemplate === "hybrid") {
    const primaryClass = phase === "plus" ? filledClass : secondaryFill;
    const secondaryClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${primaryClass} cc-shape" cx="14.2" cy="12" rx="4.4" ry="2.8"/><ellipse class="${secondaryClass} cc-shape" cx="8.6" cy="12" rx="2.3" ry="1.6"/>`, "cc-shape-icon");
  }
  if (normalizedTemplate === "dz2") {
    const primaryClass = phase === "plus" ? filledClass : secondaryFill;
    const secondaryClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${primaryClass} cc-shape" cx="12" cy="7.2" rx="2.4" ry="3.5"/><ellipse class="${secondaryClass} cc-shape" cx="12" cy="16.8" rx="2.4" ry="3.5"/><ellipse class="cc-empty-fill cc-shape" cx="12" cy="12" rx="5.7" ry="1.8"/>`, "cc-shape-icon");
  }
  return iconSvg(`<path class="${filledClass} cc-shape" d="M9.1 18.2c4.1-1 6.6-4.4 6.1-8.3-.3-2.1-1.5-3.7-3.2-5.9-2.1 2.8-3.4 4.8-3.8 7.1-.4 2.9.9 5.5.9 7.1Z"/>`, "cc-shape-icon");
}

function orbitalToolbarHtml(editorState) {
  const template = ORBITAL_STYLE_BUTTONS_BY_TEMPLATE[editorState.orbitalTemplate]
    ? editorState.orbitalTemplate
    : "s";
  const style = editorState.orbitalStyle || "hollow";
  const phase = editorState.orbitalPhase || "plus";
  const templateButtons = ORBITAL_TEMPLATE_BUTTONS.map((spec) => toolbarButton(
    orbitalComboValue(spec.template, spec.style, spec.phase),
    spec.title,
    orbitalGlyphSvg(spec.template, spec.style, spec.phase, editorState),
    template === spec.template,
  ));
  const styleButtons = (ORBITAL_STYLE_BUTTONS_BY_TEMPLATE[template] || ORBITAL_STYLE_BUTTONS_BY_TEMPLATE.s)
    .map((spec) => toolbarButton(
      orbitalComboValue(template, spec.style, spec.phase),
      spec.title,
      orbitalGlyphSvg(template, spec.style, spec.phase, editorState),
      style === spec.style && phase === spec.phase,
    ));
  return [
    ...templateButtons,
    secondaryDivider(),
    ...styleButtons,
  ].join("");
}

function bracketIconSvg(kind = "round") {
  return generatedBracketIconSvg(kind);
}

function symbolIconSvg(kind = "circle-plus", editorState = null) {
  const normalizedKind = SYMBOL_TOOL_ICON_TYPES.includes(kind) ? kind : "circle-plus";
  return editorState?.symbolIconSvgs?.[normalizedKind] || generatedBracketIconSvg(normalizedKind);
}

function bracketToolbarHtml(editorState) {
  return [
    toolbarButton("bracket-kind-round", "Parentheses", bracketIconSvg("round"), editorState.bracketKind === "round"),
    toolbarButton("bracket-kind-square", "Square brackets", bracketIconSvg("square"), editorState.bracketKind === "square"),
    toolbarButton("bracket-kind-curly", "Braces", bracketIconSvg("curly"), editorState.bracketKind === "curly"),
  ].join("");
}

function symbolToolbarHtml(editorState) {
  return [
    toolbarButton("symbol-kind-circle-plus", "Circle plus", symbolIconSvg("circle-plus", editorState), editorState.symbolKind === "circle-plus"),
    toolbarButton("symbol-kind-plus", "Plus", symbolIconSvg("plus", editorState), editorState.symbolKind === "plus"),
    toolbarButton("symbol-kind-radical-cation", "Radical cation", symbolIconSvg("radical-cation", editorState), editorState.symbolKind === "radical-cation"),
    toolbarButton("symbol-kind-lone-pair", "Lone pair", symbolIconSvg("lone-pair", editorState), editorState.symbolKind === "lone-pair"),
    toolbarButton("symbol-kind-circle-minus", "Circle minus", symbolIconSvg("circle-minus", editorState), editorState.symbolKind === "circle-minus"),
    toolbarButton("symbol-kind-minus", "Minus", symbolIconSvg("minus", editorState), editorState.symbolKind === "minus"),
    toolbarButton("symbol-kind-radical-anion", "Radical anion", symbolIconSvg("radical-anion", editorState), editorState.symbolKind === "radical-anion"),
    toolbarButton("symbol-kind-electron", "Electron", symbolIconSvg("electron", editorState), editorState.symbolKind === "electron"),
  ].join("");
}

function safeJsonParse(text, fallback) {
  try {
    return JSON.parse(text);
  } catch {
    return fallback;
  }
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function ringSvg(sides, aromatic = false) {
  return generatedRingSvg(sides, aromatic);
}

function templateIconSpec(template = "ring-6") {
  if (template === "benzene") {
    return { title: "Benzene ring", svg: ringSvg(6, true) };
  }
  const match = /^ring-(\d+)$/.exec(template || "");
  const sides = Number(match?.[1] || 6);
  return { title: `${sides}-membered ring`, svg: ringSvg(sides) };
}

function templatesToolbarHtml(editorState) {
  return [
    toolbarButton("ring-3", "3-membered ring", ringSvg(3), editorState.template === "ring-3"),
    toolbarButton("ring-4", "4-membered ring", ringSvg(4), editorState.template === "ring-4"),
    toolbarButton("ring-5", "5-membered ring", ringSvg(5), editorState.template === "ring-5"),
    toolbarButton("ring-6", "6-membered ring", ringSvg(6), editorState.template === "ring-6"),
    toolbarButton("ring-7", "7-membered ring", ringSvg(7), editorState.template === "ring-7"),
    toolbarButton("ring-8", "8-membered ring", ringSvg(8), editorState.template === "ring-8"),
    toolbarButton("benzene", "Benzene ring", ringSvg(6, true), editorState.template === "benzene"),
  ].join("");
}
