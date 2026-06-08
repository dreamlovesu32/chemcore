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
  return type === "solid" || type === "curved" || type === "curved-mirror";
}

const ICON_VIEWBOX = "0 0 24 24";
const ICON_BLACK = "#111318";
const ICON_BLUE = "#2f6fed";
export const CHEMDRAW_BASIC_COLORS = [
  { value: "#000000", title: "Black" },
  { value: "#ff0000", title: "Red" },
  { value: "#ffff00", title: "Yellow" },
  { value: "#00ff00", title: "Green" },
  { value: "#ffffff", title: "White" },
  { value: "#00ffff", title: "Cyan" },
  { value: "#0000ff", title: "Blue" },
  { value: "#ff00ff", title: "Magenta" },
];

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

const BOND_A = point(4.8, 16.6);
const BOND_B = point(19.2, 7.4);

function bondIconSvg(type = "single") {
  const axis = sub(BOND_B, BOND_A);
  const normalVector = normal(axis);
  const unitAxis = unit(axis);
  const offset = 1.42;
  const shiftedLine = (distance) => linePath(
    add(BOND_A, mul(normalVector, distance)),
    add(BOND_B, mul(normalVector, distance)),
    "cc-bond",
  );

  if (type === "double") {
    return iconSvg(`${shiftedLine(-offset)}${shiftedLine(offset)}`, "cc-bond-icon");
  }
  if (type === "triple") {
    return iconSvg(`${shiftedLine(-2.25)}${shiftedLine(0)}${shiftedLine(2.25)}`, "cc-bond-icon");
  }
  if (type === "dashed") {
    const pieces = [];
    for (let index = 0; index < 6; index += 1) {
      const startT = 0.02 + index * 0.165;
      const endT = startT + 0.082;
      pieces.push(linePath(
        add(BOND_A, mul(axis, startT)),
        add(BOND_A, mul(axis, endT)),
        "cc-bond",
      ));
    }
    return iconSvg(pieces.join(""), "cc-bond-icon");
  }
  if (type === "dashed-double") {
    const solid = shiftedLine(-offset);
    const dashed = [];
    const a = add(BOND_A, mul(normalVector, offset));
    const b = add(BOND_B, mul(normalVector, offset));
    const dashedAxis = sub(b, a);
    for (let index = 0; index < 5; index += 1) {
      const startT = index * 0.205;
      dashed.push(linePath(
        add(a, mul(dashedAxis, startT)),
        add(a, mul(dashedAxis, startT + 0.105)),
        "cc-bond",
      ));
    }
    return iconSvg(`${solid}${dashed.join("")}`, "cc-bond-icon");
  }
  if (type === "bold") {
    return iconSvg(polygon([
      add(BOND_A, mul(normalVector, -1.55)),
      add(BOND_B, mul(normalVector, -1.55)),
      add(BOND_B, mul(normalVector, 1.55)),
      add(BOND_A, mul(normalVector, 1.55)),
    ], "cc-bond-fill"), "cc-bond-icon");
  }
  if (type === "bold-dashed") {
    const pieces = [];
    for (let index = 0; index < 5; index += 1) {
      const t = 0.12 + index * 0.18;
      const center = add(BOND_A, mul(axis, t));
      const half = 1.35;
      pieces.push(linePath(
        add(center, mul(normalVector, -half)),
        add(center, mul(normalVector, half)),
        "cc-bond-hash",
      ));
    }
    return iconSvg(pieces.join(""), "cc-bond-icon");
  }
  if (type === "wavy") {
    const segments = [];
    const halfWaves = 6;
    const amplitude = 2.1;
    segments.push(`M${fmt(BOND_A.x)} ${fmt(BOND_A.y)}`);
    for (let index = 0; index < halfWaves; index += 1) {
      const t0 = index / halfWaves;
      const t1 = (index + 1) / halfWaves;
      const midT = (t0 + t1) * 0.5;
      const side = index % 2 === 0 ? -1 : 1;
      const control = add(add(BOND_A, mul(axis, midT)), mul(normalVector, amplitude * side));
      const endPoint = add(BOND_A, mul(axis, t1));
      segments.push(`Q${fmt(control.x)} ${fmt(control.y)} ${fmt(endPoint.x)} ${fmt(endPoint.y)}`);
    }
    return iconSvg(`<path class="cc-bond" d="${segments.join(" ")}"/>`, "cc-bond-icon");
  }
  if (type === "wedge") {
    return iconSvg(polygon([
      add(BOND_A, mul(normalVector, -2.45)),
      add(BOND_A, mul(normalVector, 2.45)),
      BOND_B,
    ], "cc-bond-fill"), "cc-bond-icon");
  }
  if (type === "hashed-wedge") {
    const pieces = [];
    for (let index = 0; index < 6; index += 1) {
      const t = 0.08 + index * 0.135;
      const center = add(BOND_A, mul(axis, t));
      const half = 2.25 - index * 0.32;
      pieces.push(linePath(
        add(center, mul(normalVector, -half)),
        add(center, mul(normalVector, half)),
        "cc-bond-hash",
      ));
    }
    return iconSvg(pieces.join(""), "cc-bond-icon");
  }
  if (type === "hollow-wedge") {
    const outer = [
      add(BOND_A, mul(normalVector, -2.45)),
      add(BOND_A, mul(normalVector, 2.45)),
      BOND_B,
    ];
    const center = point((outer[0].x + outer[1].x) * 0.5, (outer[0].y + outer[1].y) * 0.5);
    const inner = [
      center,
      add(outer[1], mul(unitAxis, -2.3)),
      add(outer[2], mul(unitAxis, -2.3)),
    ];
    return iconSvg(
      `<path class="cc-bond-fill" fill-rule="evenodd" d="M${fmt(outer[0].x)} ${fmt(outer[0].y)} L${fmt(outer[1].x)} ${fmt(outer[1].y)} L${fmt(outer[2].x)} ${fmt(outer[2].y)} Z M${fmt(inner[0].x)} ${fmt(inner[0].y)} L${fmt(inner[1].x)} ${fmt(inner[1].y)} L${fmt(inner[2].x)} ${fmt(inner[2].y)} Z"/>`,
      "cc-bond-icon",
    );
  }
  return iconSvg(linePath(BOND_A, BOND_B, "cc-bond"), "cc-bond-icon");
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

function shapeIconSvg(kind = "rect", style = "solid") {
  const fill = style === "filled" ? "cc-shape-fill" : style === "shaded" ? "cc-shape-soft-fill" : "cc-empty-fill";
  const dash = style === "dashed" ? ` stroke-dasharray="2.2 1.8"` : "";
  const shadow = style === "shadowed"
    ? `<path class="cc-shadow-fill" d="M8.2 8.2h10.3v9.5H8.2z"/><path class="cc-shadow-edge" d="M5.5 6.2 8.2 8.2M18.5 6.2v11.5M5.5 17.7h2.7"/>`
    : "";
  const mark = kind === "circle"
    ? `<circle class="${fill} cc-shape" cx="12" cy="12" r="6.2"${dash}/>`
    : kind === "ellipse"
      ? `<ellipse class="${fill} cc-shape" cx="12" cy="12" rx="7.2" ry="4.5"${dash}/>`
      : kind === "cross-table"
        ? `<rect class="${fill} cc-shape" x="5.5" y="6.2" width="13" height="11.5"${dash}/><path class="cc-shape" d="M12 6.2v11.5M5.5 11.95h13"${dash}/>`
        : `<rect class="${fill} cc-shape" x="5.5" y="6.2" width="13" height="11.5"${kind === "round-rect" ? ` rx="2.6"` : ""}${dash}/>`;
  return iconSvg(`${shadow}${mark}`, "cc-shape-icon");
}

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

function textFormatIconSvg(kind) {
  if (kind === "bold") {
    return iconSvg(
      `<path class="cc-fill" d="M7.05 4.9h6.08c3 0 4.74 1.34 4.74 3.53 0 1.4-.73 2.48-2.07 3.05 1.63.5 2.6 1.73 2.6 3.4 0 2.62-2.04 4.22-5.37 4.22H7.05zm3.16 5.75h2.57c1.32 0 2.08-.6 2.08-1.68 0-1.02-.74-1.58-2.07-1.58h-2.58zm0 5.88h2.92c1.44 0 2.21-.63 2.21-1.79 0-1.12-.82-1.72-2.3-1.72h-2.83z"/>`,
      "cc-text-format-icon",
    );
  }
  if (kind === "italic") {
    return iconSvg(
      `<text class="cc-italic-glyph" x="12" y="18.2" text-anchor="middle">I</text>`,
      "cc-text-format-icon",
    );
  }
  if (kind === "underline") {
    return iconSvg(
      `<path class="cc-text-stroke" d="M7.45 5.35v7.02c0 2.72 1.76 4.34 4.55 4.34s4.55-1.62 4.55-4.34V5.35"/><path class="cc-text-underline" d="M6.45 20.15h11.1"/>`,
      "cc-text-format-icon",
    );
  }
  if (kind === "chemical") {
    return iconSvg(
      `<text class="cc-chemical-main" x="3.8" y="16.2">CH2</text>`,
      "cc-text-format-icon cc-script-icon",
    );
  }
  if (kind === "subscript") {
    return iconSvg(
      `<text class="cc-script-main" x="4.45" y="14.7">X</text><text class="cc-script-sub" x="15.25" y="18.25">2</text>`,
      "cc-text-format-icon cc-script-icon",
    );
  }
  if (kind === "superscript") {
    return iconSvg(
      `<text class="cc-script-main" x="4.45" y="15.25">X</text><text class="cc-script-sup" x="15.18" y="9.15">2</text>`,
      "cc-text-format-icon cc-script-icon",
    );
  }
  return "";
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
    select: iconSvg(`<path class="cc-fill cc-select-fill" d="M5.2 4.5 18.8 12 12.2 14.1l-2.7 5.8z"/>`, "cc-tool-icon"),
    text: iconSvg(`<path class="cc-stroke" d="M7.5 19 12 5.1 16.5 19"/><path class="cc-stroke" d="M9 14.1h6"/>`, "cc-tool-icon"),
    arrow: straightArrowSvg(),
    shape: iconSvg(`<rect class="cc-shape cc-empty-fill" x="5.5" y="5.5" width="10.2" height="10.2"/><circle class="cc-shape cc-empty-fill" cx="16.8" cy="16.8" r="3.45"/>`, "cc-tool-icon"),
    "tlc-plate": iconSvg(`<rect class="cc-shape cc-empty-fill" x="5.3" y="4.8" width="13.4" height="14.2"/><path class="cc-shape" d="M5.3 8.5h13.4M5.3 15.6h13.4" stroke-dasharray="1.5 1.5"/><circle class="cc-shape-fill" cx="8.9" cy="12.9" r="1.2"/><circle class="cc-shape-fill" cx="12" cy="10.9" r="1.2"/><circle class="cc-shape-fill" cx="15.1" cy="14.1" r="1.2"/>`, "cc-tool-icon"),
    orbital: iconSvg(`<path class="cc-shape cc-empty-fill" d="M12 4c3.35 0 5.35 2.67 5.35 6.25 0 3.26-2 6.17-5.35 9.5-3.35-3.33-5.35-6.24-5.35-9.5C6.65 6.67 8.65 4 12 4Z"/><path class="cc-shape" d="M12 4c0 0 2 2.55 2 6.25S12 19.75 12 19.75"/>`, "cc-tool-icon"),
  };
  return icons[name] || "";
}

const PERIODIC_ELEMENTS = [
  ["H", 1, "Hydrogen", 1, 1],
  ["He", 2, "Helium", 18, 1],
  ["Li", 3, "Lithium", 1, 2],
  ["Be", 4, "Beryllium", 2, 2],
  ["B", 5, "Boron", 13, 2],
  ["C", 6, "Carbon", 14, 2],
  ["N", 7, "Nitrogen", 15, 2],
  ["O", 8, "Oxygen", 16, 2],
  ["F", 9, "Fluorine", 17, 2],
  ["Ne", 10, "Neon", 18, 2],
  ["Na", 11, "Sodium", 1, 3],
  ["Mg", 12, "Magnesium", 2, 3],
  ["Al", 13, "Aluminum", 13, 3],
  ["Si", 14, "Silicon", 14, 3],
  ["P", 15, "Phosphorus", 15, 3],
  ["S", 16, "Sulfur", 16, 3],
  ["Cl", 17, "Chlorine", 17, 3],
  ["Ar", 18, "Argon", 18, 3],
  ["K", 19, "Potassium", 1, 4],
  ["Ca", 20, "Calcium", 2, 4],
  ["Sc", 21, "Scandium", 3, 4],
  ["Ti", 22, "Titanium", 4, 4],
  ["V", 23, "Vanadium", 5, 4],
  ["Cr", 24, "Chromium", 6, 4],
  ["Mn", 25, "Manganese", 7, 4],
  ["Fe", 26, "Iron", 8, 4],
  ["Co", 27, "Cobalt", 9, 4],
  ["Ni", 28, "Nickel", 10, 4],
  ["Cu", 29, "Copper", 11, 4],
  ["Zn", 30, "Zinc", 12, 4],
  ["Ga", 31, "Gallium", 13, 4],
  ["Ge", 32, "Germanium", 14, 4],
  ["As", 33, "Arsenic", 15, 4],
  ["Se", 34, "Selenium", 16, 4],
  ["Br", 35, "Bromine", 17, 4],
  ["Kr", 36, "Krypton", 18, 4],
  ["Rb", 37, "Rubidium", 1, 5],
  ["Sr", 38, "Strontium", 2, 5],
  ["Y", 39, "Yttrium", 3, 5],
  ["Zr", 40, "Zirconium", 4, 5],
  ["Nb", 41, "Niobium", 5, 5],
  ["Mo", 42, "Molybdenum", 6, 5],
  ["Tc", 43, "Technetium", 7, 5],
  ["Ru", 44, "Ruthenium", 8, 5],
  ["Rh", 45, "Rhodium", 9, 5],
  ["Pd", 46, "Palladium", 10, 5],
  ["Ag", 47, "Silver", 11, 5],
  ["Cd", 48, "Cadmium", 12, 5],
  ["In", 49, "Indium", 13, 5],
  ["Sn", 50, "Tin", 14, 5],
  ["Sb", 51, "Antimony", 15, 5],
  ["Te", 52, "Tellurium", 16, 5],
  ["I", 53, "Iodine", 17, 5],
  ["Xe", 54, "Xenon", 18, 5],
  ["Cs", 55, "Cesium", 1, 6],
  ["Ba", 56, "Barium", 2, 6],
  ["La", 57, "Lanthanum", 3, 6],
  ["Hf", 72, "Hafnium", 4, 6],
  ["Ta", 73, "Tantalum", 5, 6],
  ["W", 74, "Tungsten", 6, 6],
  ["Re", 75, "Rhenium", 7, 6],
  ["Os", 76, "Osmium", 8, 6],
  ["Ir", 77, "Iridium", 9, 6],
  ["Pt", 78, "Platinum", 10, 6],
  ["Au", 79, "Gold", 11, 6],
  ["Hg", 80, "Mercury", 12, 6],
  ["Tl", 81, "Thallium", 13, 6],
  ["Pb", 82, "Lead", 14, 6],
  ["Bi", 83, "Bismuth", 15, 6],
  ["Po", 84, "Polonium", 16, 6],
  ["At", 85, "Astatine", 17, 6],
  ["Rn", 86, "Radon", 18, 6],
  ["Fr", 87, "Francium", 1, 7],
  ["Ra", 88, "Radium", 2, 7],
  ["Ac", 89, "Actinium", 3, 7],
  ["Rf", 104, "Rutherfordium", 4, 7],
  ["Db", 105, "Dubnium", 5, 7],
  ["Sg", 106, "Seaborgium", 6, 7],
  ["Bh", 107, "Bohrium", 7, 7],
  ["Hs", 108, "Hassium", 8, 7],
  ["Mt", 109, "Meitnerium", 9, 7],
  ["Ds", 110, "Darmstadtium", 10, 7],
  ["Rg", 111, "Roentgenium", 11, 7],
  ["Cn", 112, "Copernicium", 12, 7],
  ["Nh", 113, "Nihonium", 13, 7],
  ["Fl", 114, "Flerovium", 14, 7],
  ["Mc", 115, "Moscovium", 15, 7],
  ["Lv", 116, "Livermorium", 16, 7],
  ["Ts", 117, "Tennessine", 17, 7],
  ["Og", 118, "Oganesson", 18, 7],
  ["Ce", 58, "Cerium", 4, 8],
  ["Pr", 59, "Praseodymium", 5, 8],
  ["Nd", 60, "Neodymium", 6, 8],
  ["Pm", 61, "Promethium", 7, 8],
  ["Sm", 62, "Samarium", 8, 8],
  ["Eu", 63, "Europium", 9, 8],
  ["Gd", 64, "Gadolinium", 10, 8],
  ["Tb", 65, "Terbium", 11, 8],
  ["Dy", 66, "Dysprosium", 12, 8],
  ["Ho", 67, "Holmium", 13, 8],
  ["Er", 68, "Erbium", 14, 8],
  ["Tm", 69, "Thulium", 15, 8],
  ["Yb", 70, "Ytterbium", 16, 8],
  ["Lu", 71, "Lutetium", 17, 8],
  ["Th", 90, "Thorium", 4, 9],
  ["Pa", 91, "Protactinium", 5, 9],
  ["U", 92, "Uranium", 6, 9],
  ["Np", 93, "Neptunium", 7, 9],
  ["Pu", 94, "Plutonium", 8, 9],
  ["Am", 95, "Americium", 9, 9],
  ["Cm", 96, "Curium", 10, 9],
  ["Bk", 97, "Berkelium", 11, 9],
  ["Cf", 98, "Californium", 12, 9],
  ["Es", 99, "Einsteinium", 13, 9],
  ["Fm", 100, "Fermium", 14, 9],
  ["Md", 101, "Mendelevium", 15, 9],
  ["No", 102, "Nobelium", 16, 9],
  ["Lr", 103, "Lawrencium", 17, 9],
];

const ELEMENT_COLORS = {
  C: ["#000000", "#ffffff"],
  N: ["#0000d8", "#ffffff"],
  O: ["#ff0000", "#ffffff"],
  F: ["#62ee75", "#000000"],
  Na: ["#ff00d8", "#ffffff"],
  P: ["#ff72df", "#000000"],
  S: ["#ecff24", "#000000"],
  Cl: ["#00f32e", "#ffffff"],
  Fe: ["#0b6415", "#ffffff"],
  Ni: ["#6c6d6d", "#000000"],
  Cu: ["#d58428", "#ffffff"],
  Ag: ["#dcefff", "#000000"],
  Au: ["#fff438", "#000000"],
  Br: ["#8b4c42", "#ffffff"],
};

function elementIconSvg(symbol = "P") {
  const safe = String(symbol || "P").replace(/[^A-Za-z]/g, "").slice(0, 2) || "P";
  return iconSvg(`
    <path class="cc-element-tick" d="M9 4.5v3M15 4.5v3M9 16.5v3M15 16.5v3M4.5 9h3M4.5 15h3M16.5 9h3M16.5 15h3"/>
    <text class="cc-element-icon-text" x="12" y="15" text-anchor="middle">${safe}</text>
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
    ["select", commandIconSvg("select")],
    ["text", commandIconSvg("text")],
    ["arrow", straightArrowSvg()],
    ["bracket", generatedBracketIconSvg("round")],
    ["symbol", generatedBracketIconSvg("circle-plus")],
    ["element", elementIconSvg("P")],
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
    return elementToolbarHtml(editorState);
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
  syncPrimaryBondToolButton(editorState, root);
  syncPrimaryTemplateToolButton(editorState, root);
  syncPrimarySymbolToolButton(editorState, root);
  syncPrimaryElementToolButton(editorState, root);
  syncPrimaryOrbitalToolButton(editorState, root);
}

function toolbarButton(value, title, svg, selected = false) {
  return `
    <button class="secondary-button${selected ? " is-selected" : ""}" type="button" data-secondary-value="${value}" aria-label="${title}" title="${title}">
      ${svg}
    </button>
  `;
}

function colorPickerControl(prefix, currentColor, documentColors = []) {
  const color = normalizedHex(currentColor) || "#000000";
  const extras = uniqueColors(documentColors).filter((value) => (
    !CHEMDRAW_BASIC_COLORS.some((basic) => colorsEqual(basic.value, value))
  ));
  const swatches = [...CHEMDRAW_BASIC_COLORS, ...extras.map((value) => ({ value, title: value.toUpperCase() }))]
    .map((entry, index) => `
      <button class="color-panel-swatch${colorsEqual(color, entry.value) ? " is-selected" : ""}" type="button" data-color-swatch-value="${entry.value}" title="${entry.title}" aria-label="${entry.title}" style="--swatch:${entry.value}; --swatch-index:${index}"></button>
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
        <button class="color-panel-other" type="button" data-color-other>Other...</button>
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

function uniqueColors(colors) {
  const seen = new Set();
  const out = [];
  for (const color of colors || []) {
    const normalized = normalizedHex(color);
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function secondaryDivider() {
  return `<span class="secondary-divider" aria-hidden="true"></span>`;
}

const BOND_TOOL_ICON_SPECS = {
  single: {
    title: "Single bond",
    svg: bondIconSvg("single"),
  },
  double: {
    title: "Double bond",
    svg: bondIconSvg("double"),
  },
  triple: {
    title: "Triple bond",
    svg: bondIconSvg("triple"),
  },
  dashed: {
    title: "Dashed bond",
    svg: bondIconSvg("dashed"),
  },
  "dashed-double": {
    title: "Dashed-solid double bond",
    svg: bondIconSvg("dashed-double"),
  },
  bold: {
    title: "Bold bond",
    svg: bondIconSvg("bold"),
  },
  "bold-dashed": {
    title: "Hash bond",
    svg: bondIconSvg("bold-dashed"),
  },
  wavy: {
    title: "Wavy bond",
    svg: bondIconSvg("wavy"),
  },
  wedge: {
    title: "Solid wedge",
    svg: bondIconSvg("wedge"),
  },
  "hashed-wedge": {
    title: "Hash wedge",
    svg: bondIconSvg("hashed-wedge"),
  },
  "hollow-wedge": {
    title: "Hollow wedge",
    svg: bondIconSvg("hollow-wedge"),
  },
};

function bondToolIconSpec(type = "single") {
  return BOND_TOOL_ICON_SPECS[type] || BOND_TOOL_ICON_SPECS.single;
}

function syncPrimaryBondToolButton(editorState, root) {
  const bondButton = root.querySelector('.tool-button[data-tool="bond"]');
  if (!bondButton) {
    return;
  }
  const spec = bondToolIconSpec(editorState.bondType);
  bondButton.innerHTML = spec.svg;
  bondButton.setAttribute("aria-label", spec.title);
  bondButton.setAttribute("title", spec.title);
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
  symbolButton.innerHTML = bracketIconSvg(editorState.symbolKind);
}

function syncPrimaryElementToolButton(editorState, root) {
  const elementButton = root.querySelector('.tool-button[data-tool="element"]');
  if (!elementButton) {
    return;
  }
  const symbol = editorState.elementSymbol || "P";
  elementButton.innerHTML = elementIconSvg(symbol);
  elementButton.setAttribute("aria-label", `Element ${symbol}`);
  elementButton.setAttribute("title", `Element ${symbol}`);
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
  );
}

function selectToolbarHtml(editorState) {
  const mode = editorState.selectMode;
  return [
    toolbarButton("select-free", "Free selection", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6c5-4 14 1 13 7-1 7-12 7-14 1"/></svg>`, mode === "free"),
    toolbarButton("select-box", "Box selection", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" stroke-dasharray="2 2"/></svg>`, mode === "box"),
    secondaryDivider(),
    toolbarButton("align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 5v14"/><path d="M9 7h9"/><path d="M9 12h6"/><path d="M9 17h11"/></svg>`),
    toolbarButton("align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 5v14"/><path d="M6 7h9"/><path d="M9 12h6"/><path d="M4 17h11"/></svg>`),
    toolbarButton("align-top", "Align top", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 9v9"/><path d="M12 9v6"/><path d="M17 9v11"/></svg>`),
    toolbarButton("align-bottom", "Align bottom", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 18h14"/><path d="M7 6v9"/><path d="M12 9v6"/><path d="M17 4v11"/></svg>`),
    toolbarButton("align-h-center", "Horizontal center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path d="M6 7h12"/><path d="M8 12h8"/><path d="M5 17h14"/></svg>`),
    toolbarButton("align-v-center", "Vertical center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h16"/><path d="M7 6v12"/><path d="M12 8v8"/><path d="M17 5v14"/></svg>`),
    secondaryDivider(),
    toolbarButton("distribute-v", "Vertical distribute", distributeIconSvg("vertical")),
    toolbarButton("distribute-h", "Horizontal distribute", distributeIconSvg("horizontal")),
    secondaryDivider(),
    toolbarButton("flip-h", "Flip horizontal", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path class="filled" d="M5 7v10l5-5z"/><path d="M19 7v10l-5-5z"/></svg>`),
    toolbarButton("flip-v", "Flip vertical", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h16"/><path class="filled" d="M7 5h10l-5 5z"/><path d="M7 19h10l-5-5z"/></svg>`),
    secondaryDivider(),
    colorPickerControl("selection-color", editorState.selectionColor || editorState.textColor, editorState.documentColors),
  ].join("");
}

function bondToolbarHtml(editorState) {
  const type = editorState.bondType;
  return Object.entries(BOND_TOOL_ICON_SPECS)
    .map(([value, spec]) => toolbarButton(`bond-${value}`, spec.title, spec.svg, type === value))
    .join("");
}

function arrowIconSvg(type = "solid") {
  if (type === "curved" || type === "curved-mirror") {
    return curvedArrowSvg({ mirrored: type === "curved-mirror" });
  }
  if (type === "hollow") {
    return straightArrowSvg({ head: "hollow" });
  }
  if (type === "open") {
    return straightArrowSvg({ head: "open" });
  }
  return straightArrowSvg();
}

function isCurvedArrowType(type) {
  return type === "curved" || type === "curved-mirror";
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
  const mark = kind === "hash"
    ? `<path class="cc-arrow-fill" d="M10 7.5 12 8.2 8 17.5 6 16.8z"/><path class="cc-arrow-fill" d="M16 7.5 18 8.2 14 17.5 12 16.8z"/>`
    : `<path class="cc-arrow-fill" d="M7.1 6.2 17.8 16.9 16.4 18.3 5.7 7.6z"/><path class="cc-arrow-fill" d="M16.4 5.7 17.8 7.1 7.1 17.8 5.7 16.4z"/>`;
  return iconSvg(`<path class="cc-arrow" d="M4 12h12"/>${arrowHead(point(20.5, 12), point(1, 0), 0.9)}${mark}`, "cc-arrow-icon");
}

function distributeIconSvg(axis = "horizontal") {
  if (axis === "vertical") {
    return iconSvg(`
      <path class="cc-guide" d="M5.3 5.2v13.6M18.7 5.2v13.6"/>
      <rect class="cc-distribute-fill" x="7.2" y="5" width="9.6" height="3.2" rx="0.4"/>
      <rect class="cc-distribute-fill" x="7.2" y="10.4" width="9.6" height="3.2" rx="0.4"/>
      <rect class="cc-distribute-fill" x="7.2" y="15.8" width="9.6" height="3.2" rx="0.4"/>
    `, "cc-distribute-icon");
  }
  return iconSvg(`
    <path class="cc-guide" d="M5.2 5.3h13.6M5.2 18.7h13.6"/>
    <rect class="cc-distribute-fill" x="5" y="7.2" width="3.2" height="9.6" rx="0.4"/>
    <rect class="cc-distribute-fill" x="10.4" y="7.2" width="3.2" height="9.6" rx="0.4"/>
    <rect class="cc-distribute-fill" x="15.8" y="7.2" width="3.2" height="9.6" rx="0.4"/>
  `, "cc-distribute-icon");
}

function arrowToolbarHtml(editorState) {
  const type = editorState.arrowType;
  const lineSelected = editorState.arrowHeadStyle === "none" && editorState.arrowTailStyle === "none";
  const controls = [
    toolbarButton("arrow-type-solid", "Solid arrow", arrowIconSvg("solid"), type === "solid"),
    toolbarButton("arrow-type-curved", "Curved arrow", arrowIconSvg("curved"), type === "curved"),
    toolbarButton("arrow-type-curved-mirror", "Mirrored curved arrow", arrowIconSvg("curved-mirror"), type === "curved-mirror"),
    toolbarButton("arrow-type-hollow", "Hollow arrow", arrowIconSvg("hollow"), type === "hollow"),
    toolbarButton("arrow-type-open", "Open hollow arrow", arrowIconSvg("open"), type === "open"),
    secondaryDivider(),
  ];
  if (isCurvedArrowType(type)) {
    const mirrored = type === "curved-mirror";
    controls.push(
      toolbarButton("arrow-curve-270", "Curve 270 degrees", arrowCurveSvg("270", mirrored), editorState.arrowCurve === "270"),
      toolbarButton("arrow-curve-180", "Curve 180 degrees", arrowCurveSvg("180", mirrored), editorState.arrowCurve === "180"),
      toolbarButton("arrow-curve-120", "Curve 120 degrees", arrowCurveSvg("120", mirrored), editorState.arrowCurve === "120"),
      toolbarButton("arrow-curve-90", "Curve 90 degrees", arrowCurveSvg("90", mirrored), editorState.arrowCurve === "90"),
    );
    controls.push(secondaryDivider());
  }
  if (arrowTypeSupportsHeadSize(type)) {
    controls.push(
      toolbarButton("arrow-size-large", "Large arrow head", arrowSizeSvg("large"), editorState.arrowHeadSize === "large"),
      toolbarButton("arrow-size-medium", "Medium arrow head", arrowSizeSvg("medium"), editorState.arrowHeadSize === "medium"),
      toolbarButton("arrow-size-small", "Small arrow head", arrowSizeSvg("small"), editorState.arrowHeadSize === "small"),
      secondaryDivider(),
    );
  }
  controls.push(
    toolbarButton("arrow-line", "Line", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h16"/></svg>`, lineSelected),
    toolbarButton("arrow-head", "Head arrow", arrowEndpointSvg("head", "head"), editorState.arrowHeadStyle === "full"),
    toolbarButton("arrow-tail", "Tail arrow", arrowEndpointSvg("tail", "tail"), editorState.arrowTailStyle === "full"),
  );
  if (arrowTypeSupportsHeadSize(type)) {
    controls.push(
      toolbarButton("arrow-head-left", "Head left half arrow", arrowHalfEndpointSvg("head", "left"), editorState.arrowHeadStyle === "left"),
      toolbarButton("arrow-head-right", "Head right half arrow", arrowHalfEndpointSvg("head", "right"), editorState.arrowHeadStyle === "right"),
      toolbarButton("arrow-tail-left", "Tail left half arrow", arrowHalfEndpointSvg("tail", "left"), editorState.arrowTailStyle === "left"),
      toolbarButton("arrow-tail-right", "Tail right half arrow", arrowHalfEndpointSvg("tail", "right"), editorState.arrowTailStyle === "right"),
      secondaryDivider(),
      toolbarButton("arrow-nogo-cross", "Cross arrow", arrowNoGoSvg("cross"), editorState.arrowNoGo === "cross"),
      toolbarButton("arrow-nogo-hash", "Double slash arrow", arrowNoGoSvg("hash"), editorState.arrowNoGo === "hash"),
    );
  }
  controls.push(secondaryDivider());
  controls.push(toolbarButton("arrow-bold", "Bold arrow", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="12" y="17" text-anchor="middle" fill="currentColor" font-size="16" font-family="Arial, Helvetica, sans-serif" font-weight="700">B</text></svg>`, editorState.arrowBold));
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
    ${colorPickerControl("text-color", editorState.textColor, editorState.documentColors)}
    ${secondaryDivider()}
    ${toolbarButton("text-align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h9"/><path d="M5 14h12"/><path d="M5 18h8"/></svg>`, editorState.textAlign === "left")}
    ${toolbarButton("text-align-center", "Align center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 10h10"/><path d="M6 14h12"/><path d="M8 18h8"/></svg>`, editorState.textAlign === "center")}
    ${toolbarButton("text-align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M10 10h9"/><path d="M7 14h12"/><path d="M11 18h8"/></svg>`, editorState.textAlign === "right")}
    ${toolbarButton("text-align-justify", "Justify", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h14"/><path d="M5 14h14"/><path d="M5 18h14"/></svg>`, editorState.textAlign === "justify")}
    ${secondaryDivider()}
    ${toolbarButton("text-bold", "Bold", textFormatIconSvg("bold"), editorState.textBold)}
    ${toolbarButton("text-italic", "Italic", textFormatIconSvg("italic"), editorState.textItalic)}
    ${toolbarButton("text-underline", "Underline", textFormatIconSvg("underline"), editorState.textUnderline)}
    ${secondaryDivider()}
    ${toolbarButton("text-chemical", "Chemical", textFormatIconSvg("chemical"), editorState.textScript === "chemical")}
    ${toolbarButton("text-subscript", "Subscript", textFormatIconSvg("subscript"), editorState.textScript === "subscript")}
    ${toolbarButton("text-superscript", "Superscript", textFormatIconSvg("superscript"), editorState.textScript === "superscript")}
  `;
}

function shapeToolbarHtml(editorState) {
  return `
    ${toolbarButton("shape-kind-circle", "Circle", shapeIconSvg("circle"), editorState.shapeKind === "circle")}
    ${toolbarButton("shape-kind-ellipse", "Ellipse", shapeIconSvg("ellipse"), editorState.shapeKind === "ellipse")}
    ${toolbarButton("shape-kind-round-rect", "Rounded rectangle", shapeIconSvg("round-rect"), editorState.shapeKind === "round-rect")}
    ${toolbarButton("shape-kind-rect", "Rectangle", shapeIconSvg("rect"), editorState.shapeKind === "rect")}
    ${toolbarButton("shape-kind-cross-table", "Cross table", shapeIconSvg("cross-table"), editorState.shapeKind === "cross-table")}
    ${secondaryDivider()}
    ${toolbarButton("shape-style-solid", "Solid outline", shapeIconSvg("rect", "solid"), editorState.shapeStyle === "solid")}
    ${toolbarButton("shape-style-dashed", "Dashed outline", shapeIconSvg("rect", "dashed"), editorState.shapeStyle === "dashed")}
    ${toolbarButton("shape-style-shaded", "Shaded", shapeIconSvg("rect", "shaded"), editorState.shapeStyle === "shaded")}
    ${toolbarButton("shape-style-filled", "Filled", shapeIconSvg("rect", "filled"), editorState.shapeStyle === "filled")}
    ${toolbarButton("shape-style-shadowed", "Shadowed", shapeIconSvg("rect", "shadowed"), editorState.shapeStyle === "shadowed")}
    ${secondaryDivider()}
    ${colorPickerControl("shape-color", editorState.shapeColor, editorState.documentColors)}
  `;
}

function tlcPlateToolbarHtml(editorState) {
  return `
    ${colorPickerControl("shape-color", editorState.shapeColor, editorState.documentColors)}
  `;
}

function orbitalGlyphSvg(template = "s", style = "hollow", phase = "plus") {
  const filledClass = style === "filled" ? "cc-shape-fill" : style === "shaded" ? "cc-shape-soft-fill" : "cc-empty-fill";
  const secondaryFill = style === "hollow" ? "cc-empty-fill" : "cc-empty-fill";
  if (template === "s") {
    return iconSvg(`<circle class="${filledClass} cc-shape" cx="12" cy="12" r="6.1"/>`, "cc-shape-icon");
  }
  if (template === "oval") {
    return iconSvg(`<ellipse class="${filledClass} cc-shape" cx="12" cy="12" rx="7.1" ry="4.1"/>`, "cc-shape-icon");
  }
  if (template === "p") {
    const topClass = phase === "plus" ? filledClass : secondaryFill;
    const bottomClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${topClass} cc-shape" cx="12" cy="8.1" rx="3.1" ry="4.2"/><ellipse class="${bottomClass} cc-shape" cx="12" cy="15.9" rx="3.1" ry="4.2"/>`, "cc-shape-icon");
  }
  if (template === "dxy") {
    const primaryClass = phase === "plus" ? filledClass : secondaryFill;
    const secondaryClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${primaryClass} cc-shape" cx="8.4" cy="8.4" rx="2.2" ry="3.6" transform="rotate(-45 8.4 8.4)"/><ellipse class="${primaryClass} cc-shape" cx="15.6" cy="15.6" rx="2.2" ry="3.6" transform="rotate(-45 15.6 15.6)"/><ellipse class="${secondaryClass} cc-shape" cx="15.6" cy="8.4" rx="2.2" ry="3.6" transform="rotate(45 15.6 8.4)"/><ellipse class="${secondaryClass} cc-shape" cx="8.4" cy="15.6" rx="2.2" ry="3.6" transform="rotate(45 8.4 15.6)"/>`, "cc-shape-icon");
  }
  if (template === "hybrid") {
    const primaryClass = phase === "plus" ? filledClass : secondaryFill;
    const secondaryClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${primaryClass} cc-shape" cx="14.2" cy="12" rx="4.4" ry="2.8"/><ellipse class="${secondaryClass} cc-shape" cx="8.6" cy="12" rx="2.3" ry="1.6"/>`, "cc-shape-icon");
  }
  if (template === "dz2") {
    const primaryClass = phase === "plus" ? filledClass : secondaryFill;
    const secondaryClass = phase === "plus" ? secondaryFill : filledClass;
    return iconSvg(`<ellipse class="${primaryClass} cc-shape" cx="12" cy="7.2" rx="2.4" ry="3.5"/><ellipse class="${secondaryClass} cc-shape" cx="12" cy="16.8" rx="2.4" ry="3.5"/><ellipse class="cc-empty-fill cc-shape" cx="12" cy="12" rx="5.7" ry="1.8"/>`, "cc-shape-icon");
  }
  return iconSvg(`<path class="${filledClass} cc-shape" d="M9.1 18.2c4.1-1 6.6-4.4 6.1-8.3-.3-2.1-1.5-3.7-3.2-5.9-2.1 2.8-3.4 4.8-3.8 7.1-.4 2.9.9 5.5.9 7.1Z"/>`, "cc-shape-icon");
}

function orbitalToolbarHtml(editorState) {
  const template = editorState.orbitalTemplate || "s";
  const style = editorState.orbitalStyle || "hollow";
  const phase = editorState.orbitalPhase || "plus";
  return `
    ${toolbarButton("orbital-template-s", "s orbital", orbitalGlyphSvg("s", style, phase), template === "s")}
    ${toolbarButton("orbital-template-p", "p orbital", orbitalGlyphSvg("p", style, phase), template === "p")}
    ${toolbarButton("orbital-template-dxy", "dxy orbital", orbitalGlyphSvg("dxy", style, phase), template === "dxy")}
    ${toolbarButton("orbital-template-oval", "oval orbital", orbitalGlyphSvg("oval", style, phase), template === "oval")}
    ${toolbarButton("orbital-template-hybrid", "hybrid orbital", orbitalGlyphSvg("hybrid", style, phase), template === "hybrid")}
    ${toolbarButton("orbital-template-dz2", "dz2 orbital", orbitalGlyphSvg("dz2", style, phase), template === "dz2")}
    ${toolbarButton("orbital-template-lobe", "lobe orbital", orbitalGlyphSvg("lobe", style, phase), template === "lobe")}
    ${secondaryDivider()}
    ${toolbarButton("orbital-style-hollow", "Hollow", orbitalGlyphSvg(template, "hollow", phase), style === "hollow")}
    ${toolbarButton("orbital-style-filled", "Filled", orbitalGlyphSvg(template, "filled", phase), style === "filled")}
    ${toolbarButton("orbital-style-shaded", "Shaded", orbitalGlyphSvg(template, "shaded", phase), style === "shaded")}
    ${secondaryDivider()}
    ${toolbarButton("orbital-phase-plus", "Primary lobe position", orbitalGlyphSvg(template, style, "plus"), phase === "plus")}
    ${toolbarButton("orbital-phase-minus", "Alternate lobe position", orbitalGlyphSvg(template, style, "minus"), phase === "minus")}
    ${secondaryDivider()}
    ${colorPickerControl("orbital-color", editorState.orbitalColor || editorState.shapeColor, editorState.documentColors)}
  `;
}

function bracketIconSvg(kind = "round") {
  return generatedBracketIconSvg(kind);
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
    toolbarButton("symbol-kind-circle-plus", "Circle plus", bracketIconSvg("circle-plus"), editorState.symbolKind === "circle-plus"),
    toolbarButton("symbol-kind-plus", "Plus", bracketIconSvg("plus"), editorState.symbolKind === "plus"),
    toolbarButton("symbol-kind-radical-cation", "Radical cation", bracketIconSvg("radical-cation"), editorState.symbolKind === "radical-cation"),
    toolbarButton("symbol-kind-lone-pair", "Lone pair", bracketIconSvg("lone-pair"), editorState.symbolKind === "lone-pair"),
    toolbarButton("symbol-kind-circle-minus", "Circle minus", bracketIconSvg("circle-minus"), editorState.symbolKind === "circle-minus"),
    toolbarButton("symbol-kind-minus", "Minus", bracketIconSvg("minus"), editorState.symbolKind === "minus"),
    toolbarButton("symbol-kind-radical-anion", "Radical anion", bracketIconSvg("radical-anion"), editorState.symbolKind === "radical-anion"),
    toolbarButton("symbol-kind-electron", "Electron", bracketIconSvg("electron"), editorState.symbolKind === "electron"),
  ].join("");
}

function elementToolbarHtml(editorState) {
  const current = periodicElement(editorState.elementSymbol || "P") || periodicElement("P");
  const symbol = current?.[0] || "P";
  const atomicNumber = current?.[1] || 15;
  const name = current?.[2] || "Phosphorus";
  return `
    <div class="element-picker">
      <button class="secondary-button element-current-button" type="button" data-element-picker-toggle aria-label="Periodic table" title="Periodic table">
        <span class="periodic-element-symbol">${symbol}</span>
      </button>
      <div class="periodic-table-panel" role="menu" aria-label="Periodic table">
        <div class="periodic-table-title">${atomicNumber} ${name}</div>
        <div class="periodic-table-grid">
          ${PERIODIC_ELEMENTS.map((element) => periodicElementButton(element, symbol)).join("")}
        </div>
      </div>
    </div>
  `;
}

function periodicElement(symbol) {
  return PERIODIC_ELEMENTS.find((element) => element[0] === symbol) || null;
}

function periodicElementButton([symbol, atomicNumber, name, column, row], currentSymbol) {
  const colors = ELEMENT_COLORS[symbol];
  const style = [
    `grid-column:${column}`,
    `grid-row:${row}`,
    colors ? `--element-bg:${colors[0]}` : "",
    colors ? `--element-fg:${colors[1]}` : "",
  ].filter(Boolean).join(";");
  return `
    <button
      class="periodic-element-button${symbol === currentSymbol ? " is-selected" : ""}${colors ? " has-element-color" : ""}"
      type="button"
      role="menuitem"
      data-secondary-value="element-symbol-${symbol}-${atomicNumber}"
      title="${atomicNumber} ${name}"
      aria-label="${atomicNumber} ${name}"
      style="${style}">
      ${symbol}
    </button>
  `;
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
