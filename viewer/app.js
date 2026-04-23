import {
  initializeGlyphKernel,
  LABEL_ALIGN,
} from "./glyph_kernel_runtime.js";
import initializeChemcoreEngine, { WasmEngine } from "./engine/chemcore_engine.js";

const SAMPLE_FILES = [
  "../tmp/examples/02-13/2017-2-13/oleObject1.chemcore.json",
  "../tmp/examples/02-13/2017-2-13/oleObject2.chemcore.json",
  "../tmp/examples/02-13/2017-2-13/oleObject3.chemcore.json",
  "../tmp/examples/02-13/2017-2-13/oleObject4.chemcore.json",
  "../tmp/examples/02-13/lm 2017-2-13  working report/oleObject1.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject1.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject2.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject3.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject4.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject5.chemcore.json",
  "../tmp/examples/02-13/工作汇报-jc-2017-2-13/oleObject6.chemcore.json",
];

const VIEW_MODE = document.body.dataset.viewMode || "document";
const LABEL_DEBUG_MODE = VIEW_MODE === "label-debug";

const state = {
  currentPath: LABEL_DEBUG_MODE ? SAMPLE_FILES[0] : null,
  currentDocument: null,
  glyphKernel: null,
  editorEngine: null,
};

if (typeof window !== "undefined") {
  window.__chemcoreDebug = {
    state,
    get document() {
      return state.currentDocument;
    },
    get engineState() {
      return currentEditorEngineState();
    },
  };
}

const DEFAULT_TEXT_FONT_SIZE = 12;
const LABEL_FONT_SIZE = 11;
const BOND_STROKE = 0.85;
const MULTI_BOND_OFFSET = 2.15;
const DOUBLE_BOND_OFFSET = 2.85;
const TRIPLE_BOND_OFFSET = 2.9;
const DOUBLE_BOND_SIDE_INSET = 1.4;
const DOUBLE_BOND_SIDE_INSET_RATIO = 0.14;
const HASH_WEDGE_SPACING = 3.2;
const HASH_WEDGE_START_OFFSET = 1.95;
const HASH_WEDGE_END_INSET = 0.18;
const SOLID_WEDGE_END_INSET = 0.55;
const CHEMDRAW_PAGE_BACKGROUND = "#ffffff";
const CHEMDRAW_INK = "#000000";
const CHEMDRAW_COLOR_MAP = new Map([
  ["#d61f1f", "#ff0000"],
  ["#1b32d8", "#0000ff"],
]);

function normalizeDisplayColor(color, fallback = CHEMDRAW_INK) {
  if (!color) {
    return fallback;
  }
  const value = String(color).trim().toLowerCase();
  return CHEMDRAW_COLOR_MAP.get(value) || color;
}

function displayLabelFontFamily(fontFamily) {
  const value = String(fontFamily || "").trim();
  if (!value || /^(arial|helvetica|texgyreheros|tex gyre heros)$/i.test(value)) {
    return "\"TeX Gyre Heros\", Arial, Helvetica, sans-serif";
  }
  return `${value}, "TeX Gyre Heros", Arial, Helvetica, sans-serif`;
}

function isSubscriptFace(face) {
  const value = Number(face) || 0;
  return (value & 32) !== 0 && (value & 64) === 0;
}

function isSuperscriptFace(face) {
  const value = Number(face) || 0;
  return (value & 64) !== 0 && (value & 32) === 0;
}

function scriptKindForRun(run) {
  const script = String(run?.script || "").toLowerCase();
  if (script === "subscript") {
    return 1;
  }
  if (script === "superscript") {
    return 2;
  }
  return state.glyphKernel.scriptKindForFace(run?.face);
}

function isSubscriptRun(run) {
  const script = String(run?.script || "").toLowerCase();
  return script === "subscript" || (!script && isSubscriptFace(run?.face));
}

function isSuperscriptRun(run) {
  const script = String(run?.script || "").toLowerCase();
  return script === "superscript" || (!script && isSuperscriptFace(run?.face));
}

function fontWeightForRun(run, fallbackFace = 0) {
  if (run?.fontWeight !== undefined && run?.fontWeight !== null) {
    return Number(run.fontWeight);
  }
  const face = Number(run?.face ?? fallbackFace ?? 0);
  return (face & 1) ? 700 : undefined;
}

function fontStyleForRun(run, fallbackFace = 0) {
  if (run?.fontStyle) {
    return run.fontStyle;
  }
  const face = Number(run?.face ?? fallbackFace ?? 0);
  return (face & 2) ? "italic" : undefined;
}

function boxCenter(box) {
  return {
    x: (box.x1 + box.x2) / 2,
    y: (box.y1 + box.y2) / 2,
  };
}

function isCutCornerShape(shape) {
  return shape?.kind === "rect-cut-top-right"
    || shape?.kind === "rect-cut-bottom-right"
    || shape?.kind === "rect-cut-top-left"
    || shape?.kind === "rect-cut-bottom-left";
}

function cutCornerSize(shape) {
  return Math.max(0, Math.min(shape.x2 - shape.x1, shape.y2 - shape.y1) * 0.42);
}

function polygonPointsForShape(shape) {
  if (!shape) {
    return [];
  }
  const cut = cutCornerSize(shape);
  if (shape.kind === "rect-cut-top-right") {
    return [
      { x: shape.x1, y: shape.y1 },
      { x: shape.x2 - cut, y: shape.y1 },
      { x: shape.x2, y: shape.y1 + cut },
      { x: shape.x2, y: shape.y2 },
      { x: shape.x1, y: shape.y2 },
    ];
  }
  if (shape.kind === "rect-cut-bottom-right") {
    return [
      { x: shape.x1, y: shape.y1 },
      { x: shape.x2, y: shape.y1 },
      { x: shape.x2, y: shape.y2 - cut },
      { x: shape.x2 - cut, y: shape.y2 },
      { x: shape.x1, y: shape.y2 },
    ];
  }
  if (shape.kind === "rect-cut-top-left") {
    return [
      { x: shape.x1 + cut, y: shape.y1 },
      { x: shape.x2, y: shape.y1 },
      { x: shape.x2, y: shape.y2 },
      { x: shape.x1, y: shape.y2 },
      { x: shape.x1, y: shape.y1 + cut },
    ];
  }
  if (shape.kind === "rect-cut-bottom-left") {
    return [
      { x: shape.x1, y: shape.y1 },
      { x: shape.x2, y: shape.y1 },
      { x: shape.x2, y: shape.y2 },
      { x: shape.x1 + cut, y: shape.y2 },
      { x: shape.x1, y: shape.y2 - cut },
    ];
  }
  return [
    { x: shape.x1, y: shape.y1 },
    { x: shape.x2, y: shape.y1 },
    { x: shape.x2, y: shape.y2 },
    { x: shape.x1, y: shape.y2 },
  ];
}

function svgPathForShape(shape) {
  const points = polygonPointsForShape(shape);
  if (!points.length) {
    return "";
  }
  return points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
    .join(" ") + " Z";
}

function pointInPolygon(point, points) {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i, i += 1) {
    const pi = points[i];
    const pj = points[j];
    const intersects = ((pi.y > point.y) !== (pj.y > point.y))
      && point.x < ((pj.x - pi.x) * (point.y - pi.y)) / ((pj.y - pi.y) || 1e-9) + pi.x;
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}

function offsetPointAlongRay(point, direction, distance) {
  const length = Math.hypot(direction.x, direction.y) || 1;
  return {
    x: point.x + (direction.x / length) * distance,
    y: point.y + (direction.y / length) * distance,
  };
}

function pointInOpticalShape(point, shape, margin = 0) {
  if (!shape) {
    return false;
  }
  if (shape.kind === "ellipse") {
    const rx = shape.rx + margin;
    const ry = shape.ry + margin;
    if (rx <= 0 || ry <= 0) {
      return false;
    }
    const dx = point.x - shape.cx;
    const dy = point.y - shape.cy;
    return (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) <= 1;
  }
  if (isCutCornerShape(shape)) {
    return pointInPolygon(point, polygonPointsForShape(expandOpticalShape(shape, margin)));
  }
  return (
    point.x >= shape.x1 - margin &&
    point.x <= shape.x2 + margin &&
    point.y >= shape.y1 - margin &&
    point.y <= shape.y2 + margin
  );
}

function shiftRayOpticalShape(point, direction, shape) {
  if (!shape) {
    return 0;
  }
  if (shape.kind === "ellipse") {
    const length = Math.hypot(direction.x, direction.y) || 1;
    const d = { x: direction.x / length, y: direction.y / length };
    const dx = point.x - shape.cx;
    const dy = point.y - shape.cy;
    const rx = Math.max(0.1, shape.rx);
    const ry = Math.max(0.1, shape.ry);
    const a = (d.x * d.x) / (rx * rx) + (d.y * d.y) / (ry * ry);
    const b = 2 * ((dx * d.x) / (rx * rx) + (dy * d.y) / (ry * ry));
    const c = (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) - 1;
    const disc = b * b - 4 * a * c;
    if (disc < 0 || Math.abs(a) < 1e-9) {
      return 0;
    }
    const root = Math.sqrt(disc);
    return Math.max((-b - root) / (2 * a), (-b + root) / (2 * a), 0);
  }

  const corners = polygonPointsForShape(shape);
  const length = Math.hypot(direction.x, direction.y) || 1;
  const d = { x: direction.x / length, y: direction.y / length };
  const rel = corners.map((corner) => ({
    x: corner.x - point.x,
    y: corner.y - point.y,
  }));
  const rc = rel.map((value) => value.x * d.y - value.y * d.x);
  const rd = rel.map((value) => value.x * d.x + value.y * d.y);

  let positive = -1;
  let negative = -1;
  for (let index = 0; index < corners.length; index += 1) {
    if (rc[index] > 0) {
      if (positive < 0 || rd[positive] < rd[index]) positive = index;
    } else if (negative < 0 || rd[negative] < rd[index]) {
      negative = index;
    }
  }
  if (positive < 0 || negative < 0) {
    return 0;
  }
  const first = rd[positive] > rd[negative] ? negative : positive;
  const second = rd[positive] > rd[negative] ? positive : negative;
  return (
    rd[first] +
    (Math.abs(rc[first]) * (rd[second] - rd[first])) /
      (Math.abs(rc[first]) + Math.abs(rc[second]))
  );
}

function shiftRayOpticalShapes(point, direction, shapes) {
  let shift = 0;
  for (const shape of shapes || []) {
    shift = Math.max(shift, shiftRayOpticalShape(point, direction, shape));
  }
  return shift;
}

function pointInShapes(point, shapes, margin) {
  return (shapes || []).some((shape) => pointInOpticalShape(point, shape, margin));
}

function expandOpticalShape(shape, margin) {
  if (!shape || margin <= 0) {
    return shape;
  }
  if (shape.kind === "ellipse") {
    return {
      ...shape,
      rx: shape.rx + margin,
      ry: shape.ry + margin,
    };
  }
  return {
    ...shape,
    x1: shape.x1 - margin,
    y1: shape.y1 - margin,
    x2: shape.x2 + margin,
    y2: shape.y2 + margin,
  };
}

function lineIntersection(point, direction, otherPoint, otherDirection) {
  return lineIntersectionWithParameters(point, direction, otherPoint, otherDirection)?.point || null;
}

function lineIntersectionWithParameters(point, direction, otherPoint, otherDirection) {
  const cross = direction.x * otherDirection.y - direction.y * otherDirection.x;
  if (Math.abs(cross) < 1e-6) {
    return null;
  }
  const dx = otherPoint.x - point.x;
  const dy = otherPoint.y - point.y;
  const t = (dx * otherDirection.y - dy * otherDirection.x) / cross;
  const u = (dx * direction.y - dy * direction.x) / cross;
  return {
    point: {
      x: point.x + direction.x * t,
      y: point.y + direction.y * t,
    },
    t,
    u,
  };
}

function farSideContactLinePoint(contactPoint, contactDirection, interiorPoint) {
  const normal = {
    x: -contactDirection.y,
    y: contactDirection.x,
  };
  const toInterior = {
    x: interiorPoint.x - contactPoint.x,
    y: interiorPoint.y - contactPoint.y,
  };
  const interiorSide = Math.sign(toInterior.x * normal.x + toInterior.y * normal.y) || 1;
  const offset = BOND_STROKE * 0.55;
  return {
    x: contactPoint.x - normal.x * interiorSide * offset,
    y: contactPoint.y - normal.y * interiorSide * offset,
  };
}

function computeSolidWedgeGeometry(start, end, targetShapes = null, options = {}) {
  const tuning = {
    endInset: 0.55,
    minWidth: 1.05,
    maxWidth: 1.8,
    widthScale: 0.155,
    probeMargin: 0.04,
    retreatStep: 0.12,
    ...options,
  };
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const normalX = -dy / length;
  const normalY = dx / length;
  const width = Math.min(tuning.maxWidth, Math.max(tuning.minWidth, length * tuning.widthScale));

  let capInset = Math.min(tuning.endInset, length * 0.22);
  if (targetShapes?.length) {
    const maxCapInset = Math.max(capInset, length - 1.1);
    while (capInset + 1e-6 < maxCapInset) {
      const capCenter = {
        x: end.x - ux * capInset,
        y: end.y - uy * capInset,
      };
      const probes = [
        capCenter,
        { x: capCenter.x + normalX * width, y: capCenter.y + normalY * width },
        { x: capCenter.x - normalX * width, y: capCenter.y - normalY * width },
      ];
      if (!probes.some((point) => pointInShapes(point, targetShapes, tuning.probeMargin))) {
        break;
      }
      const next = Math.min(maxCapInset, capInset + tuning.retreatStep);
      if (Math.abs(next - capInset) < 1e-6) {
        break;
      }
      capInset = next;
    }
  }

  const capCenter = {
    x: end.x - ux * capInset,
    y: end.y - uy * capInset,
  };
  const capPlus = {
    x: capCenter.x + normalX * width,
    y: capCenter.y + normalY * width,
  };
  const capMinus = {
    x: capCenter.x - normalX * width,
    y: capCenter.y - normalY * width,
  };

  const wideContactDirections = Array.isArray(options.wideContactDirections)
    ? options.wideContactDirections
    : (options.wideContactDirection ? [options.wideContactDirection] : []);
  const wideContactEntries = wideContactDirections
    .map((direction) => {
      const contactLength = Math.hypot(direction.x, direction.y);
      if (contactLength <= 1e-6) {
        return null;
      }
      const unit = {
        x: direction.x / contactLength,
        y: direction.y / contactLength,
      };
      const sideValue = normalX * unit.x + normalY * unit.y;
      const side = Math.sign(sideValue);
      if (side === 0) {
        return null;
      }
      return { direction: unit, side, sideValue };
    })
    .filter(Boolean);
  const wideContactSides = wideContactEntries.map((entry) => entry.side);
  const hasPlusContact = wideContactSides.includes(1);
  const hasMinusContact = wideContactSides.includes(-1);

  if (hasPlusContact && hasMinusContact) {
    const plusContact = wideContactEntries
      .filter((entry) => entry.side === 1)
      .sort((a, b) => Math.abs(b.sideValue) - Math.abs(a.sideValue))[0];
    const minusContact = wideContactEntries
      .filter((entry) => entry.side === -1)
      .sort((a, b) => Math.abs(b.sideValue) - Math.abs(a.sideValue))[0];
    const plusIntersection = lineIntersection(
      start,
      { x: capPlus.x - start.x, y: capPlus.y - start.y },
      farSideContactLinePoint(end, plusContact.direction, start),
      plusContact.direction,
    ) || capPlus;
    const minusIntersection = lineIntersection(
      start,
      { x: capMinus.x - start.x, y: capMinus.y - start.y },
      farSideContactLinePoint(end, minusContact.direction, start),
      minusContact.direction,
    ) || capMinus;
    return {
      width,
      capInset,
      wideContact: true,
      wideContactCount: wideContactEntries.length,
      points: [
        { x: start.x, y: start.y },
        plusIntersection,
        { x: end.x, y: end.y },
        minusIntersection,
      ],
    };
  }

  if (hasPlusContact !== hasMinusContact) {
    const plusFacesContact = hasPlusContact;
    const contact = wideContactEntries
      .filter((entry) => entry.side === (plusFacesContact ? 1 : -1))
      .sort((a, b) => Math.abs(b.sideValue) - Math.abs(a.sideValue))[0];
    const plusIntersection = lineIntersection(
      start,
      { x: capPlus.x - start.x, y: capPlus.y - start.y },
      farSideContactLinePoint(end, contact.direction, start),
      contact.direction,
    ) || capPlus;
    const minusIntersection = lineIntersection(
      start,
      { x: capMinus.x - start.x, y: capMinus.y - start.y },
      farSideContactLinePoint(end, contact.direction, start),
      contact.direction,
    ) || capMinus;
    return {
      width,
      capInset,
      wideContact: true,
      wideContactCount: wideContactSides.length,
      points: [
        { x: start.x, y: start.y },
        plusIntersection,
        minusIntersection,
      ],
    };
  }

  return {
    width,
    capInset,
    wideContact: hasPlusContact || hasMinusContact,
    wideContactCount: wideContactSides.length,
    points: [
      { x: start.x, y: start.y },
      capPlus,
      capMinus,
    ],
  };
}

function computeHashedWedgeGeometry(start, end, targetShapes = null, options = {}) {
  const tuning = {
    spacing: 3.2,
    startOffset: 1.95,
    endInset: 0.18,
    firstHalfWidth: 0.42,
    bodyStartHalfWidth: 0.16,
    bodyGrowHalfWidth: 1.72,
    firstStrokeWidth: 0.82,
    bodyStrokeWidth: 0.72,
    probeMargin: 0.08,
    retreatStep: 0.12,
    retreatFromProgress: 0.42,
    minUsableLength: 0.35,
    ...options,
  };
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  const nx = -dy / length;
  const ny = dx / length;
  const startGap = Math.min(tuning.startOffset, length * 0.3);
  let endGap = Math.min(tuning.endInset, length * 0.08);

  if (targetShapes?.length) {
    const maxEndGap = Math.max(endGap, length - startGap - tuning.minUsableLength);
    while (endGap + 1e-6 < maxEndGap) {
      const usable = Math.max(0.01, length - startGap - endGap);
      const steps = Math.max(2, Math.round(usable / tuning.spacing) + 1);
      const spacing = steps > 1 ? usable / (steps - 1) : usable;
      let intrudes = false;
      for (let index = 0; index < steps; index += 1) {
        const dist = startGap + spacing * index;
        if (dist > length - endGap + 1e-6) {
          break;
        }
        const progress = steps > 1 ? index / (steps - 1) : 1;
        if (progress < tuning.retreatFromProgress) {
          continue;
        }
        const halfWidth = index === 0
          ? tuning.firstHalfWidth
          : tuning.bodyStartHalfWidth + progress * tuning.bodyGrowHalfWidth;
        const cx = start.x + dx * (dist / length);
        const cy = start.y + dy * (dist / length);
        const vertical = index === 0;
        const segment = {
          x1: vertical ? cx : cx - nx * halfWidth,
          y1: vertical ? cy - halfWidth : cy - ny * halfWidth,
          x2: vertical ? cx : cx + nx * halfWidth,
          y2: vertical ? cy + halfWidth : cy + ny * halfWidth,
          strokeWidth: vertical ? tuning.firstStrokeWidth : tuning.bodyStrokeWidth,
        };
        if (hashedSegmentIntrudes(segment, targetShapes, tuning.probeMargin)) {
          intrudes = true;
          break;
        }
      }
      if (!intrudes) {
        break;
      }
      const next = Math.min(maxEndGap, endGap + tuning.retreatStep);
      if (Math.abs(next - endGap) < 1e-6) {
        break;
      }
      endGap = next;
    }
  }

  const usable = Math.max(0.01, length - startGap - endGap);
  const steps = Math.max(2, Math.round(usable / tuning.spacing) + 1);
  const spacing = steps > 1 ? usable / (steps - 1) : usable;
  const segments = [];
  for (let index = 0; index < steps; index += 1) {
    const dist = startGap + spacing * index;
    if (dist > length - endGap + 1e-6) {
      break;
    }
    const progress = steps > 1 ? index / (steps - 1) : 1;
    const halfWidth = index === 0
      ? tuning.firstHalfWidth
      : tuning.bodyStartHalfWidth + progress * tuning.bodyGrowHalfWidth;
    const cx = start.x + dx * (dist / length);
    const cy = start.y + dy * (dist / length);
    const vertical = index === 0;
    segments.push({
      x1: vertical ? cx : cx - nx * halfWidth,
      y1: vertical ? cy - halfWidth : cy - ny * halfWidth,
      x2: vertical ? cx : cx + nx * halfWidth,
      y2: vertical ? cy + halfWidth : cy + ny * halfWidth,
      strokeWidth: vertical ? tuning.firstStrokeWidth : tuning.bodyStrokeWidth,
    });
  }
  const visibleSegments = targetShapes?.length
    ? segments.filter((segment) => !hashedSegmentIntrudes(segment, targetShapes, tuning.probeMargin))
    : segments;
  return { startGap, endGap, segments: visibleSegments };
}

function hashedSegmentIntrudes(segment, targetShapes, probeMargin) {
  const margin = probeMargin + (segment.strokeWidth || 0) * 0.5;
  for (const t of [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1]) {
    const point = {
      x: segment.x1 + (segment.x2 - segment.x1) * t,
      y: segment.y1 + (segment.y2 - segment.y1) * t,
    };
    if (pointInShapes(point, targetShapes, margin)) {
      return true;
    }
  }
  return false;
}

const sampleSelect = document.getElementById("sample-select");
const reloadButton = document.getElementById("reload-button");
const fitButton = document.getElementById("fit-button");
const toggleMolecules = document.getElementById("toggle-molecules");
const toggleLines = document.getElementById("toggle-lines");
const toggleTexts = document.getElementById("toggle-texts");
const docMeta = document.getElementById("doc-meta");
const viewerTitle = document.getElementById("viewer-title");
const viewerStats = document.getElementById("viewer-stats");
const viewerSvg = document.getElementById("viewer-svg");
const secondaryToolbar = document.getElementById("secondary-toolbar");

if (sampleSelect) {
  for (const samplePath of SAMPLE_FILES) {
    const option = document.createElement("option");
    option.value = samplePath;
    option.textContent = samplePath.replace("../tmp/examples/", "");
    sampleSelect.appendChild(option);
  }

  sampleSelect.value = state.currentPath;
  sampleSelect.addEventListener("change", async (event) => {
    state.currentPath = event.target.value;
    await loadAndRender();
  });
}

reloadButton?.addEventListener("click", async () => {
  await loadAndRender();
});

fitButton?.addEventListener("click", () => {
  fitView();
});

toggleMolecules?.addEventListener("change", () => renderDocument());
toggleLines?.addEventListener("change", () => renderDocument());
toggleTexts?.addEventListener("change", () => renderDocument());

const zoomInput = document.getElementById("zoom-input");
let zoomPercent = 100;
const editorState = {
  activeTool: "select",
  selectMode: "free",
  bondType: "single",
  textColor: "#000000",
  shapeStroke: "#000000",
  shapeFill: "none",
  shapeStyle: "rect",
  template: "benzene",
};

function isEditingRustDocument() {
  return !LABEL_DEBUG_MODE && !state.currentPath && state.editorEngine;
}

function syncEngineToolState() {
  if (!state.editorEngine) {
    return;
  }
  state.editorEngine.setTool(editorState.activeTool, editorState.bondType);
}

function parseEngineJson(json, fallback = null) {
  try {
    return JSON.parse(json);
  } catch (error) {
    console.warn("Failed to parse chemcore engine JSON", error);
    return fallback;
  }
}

function syncDocumentFromEngine() {
  if (!state.editorEngine) {
    return;
  }
  const documentData = parseEngineJson(state.editorEngine.documentJson());
  if (documentData) {
    state.currentDocument = documentData;
  }
  refreshCommandAvailability();
}

function currentEditorEngineState() {
  if (!state.editorEngine) {
    return null;
  }
  return parseEngineJson(state.editorEngine.stateJson());
}

function resetEditorEngine() {
  state.editorEngine?.free?.();
  state.editorEngine = new WasmEngine();
  syncEngineToolState();
  syncDocumentFromEngine();
}

function refreshCommandAvailability() {
  const undoButton = document.querySelector('[data-command="undo"]');
  const redoButton = document.querySelector('[data-command="redo"]');
  if (undoButton) {
    undoButton.disabled = !state.editorEngine?.canUndo?.();
  }
  if (redoButton) {
    redoButton.disabled = !state.editorEngine?.canRedo?.();
  }
}

function runEditorCommand(command) {
  if (!isEditingRustDocument()) {
    return false;
  }
  let changed = false;
  if (command === "undo") {
    changed = state.editorEngine.undo();
  } else if (command === "redo") {
    changed = state.editorEngine.redo();
  } else if (command === "delete") {
    changed = state.editorEngine.deleteSelection();
  } else {
    return false;
  }
  if (changed) {
    syncDocumentFromEngine();
    renderDocument();
  } else {
    renderEditorOverlay();
    refreshCommandAvailability();
  }
  return true;
}

function setZoomPercent(nextZoom) {
  zoomPercent = Math.max(25, Math.min(400, Math.round(nextZoom)));
  if (zoomInput) {
    zoomInput.value = `${zoomPercent}%`;
  }
}

document.querySelectorAll("[data-command]").forEach((button) => {
  button.addEventListener("click", () => {
    const command = button.dataset.command;
    if (runEditorCommand(command)) {
      return;
    }
    if (command === "zoom-in") {
      setZoomPercent(zoomPercent + 10);
    } else if (command === "zoom-out") {
      setZoomPercent(zoomPercent - 10);
    } else if (command === "fit") {
      setZoomPercent(100);
    } else if (command === "new") {
      state.currentPath = null;
      resetEditorEngine();
      renderDocument();
      fitView();
    }
  });
});

zoomInput?.addEventListener("change", () => {
  const parsed = Number.parseInt(String(zoomInput.value || "").replace(/[^\d]/g, ""), 10);
  setZoomPercent(Number.isFinite(parsed) ? parsed : zoomPercent);
});

document.addEventListener("keydown", (event) => {
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement) {
    return;
  }
  const commandKey = event.ctrlKey || event.metaKey;
  let command = null;
  if (commandKey && event.key.toLowerCase() === "z" && !event.shiftKey) {
    command = "undo";
  } else if ((commandKey && event.key.toLowerCase() === "y") || (commandKey && event.shiftKey && event.key.toLowerCase() === "z")) {
    command = "redo";
  } else if (event.key === "Delete" || event.key === "Backspace") {
    command = "delete";
  }
  if (command && runEditorCommand(command)) {
    event.preventDefault();
  }
});

function toolbarButton(value, title, svg, selected = false) {
  return `
    <button class="secondary-button${selected ? " is-selected" : ""}" type="button" data-secondary-value="${value}" aria-label="${title}" title="${title}">
      ${svg}
    </button>
  `;
}

function colorButton(value, title, color, selected = false) {
  const noFillClass = color === "none" ? " no-fill" : "";
  const swatchStyle = color === "none" ? "" : ` style="--swatch:${color}"`;
  return `
    <button class="color-button${selected ? " is-selected" : ""}" type="button" data-secondary-value="${value}" aria-label="${title}" title="${title}">
      <span class="color-swatch${noFillClass}"${swatchStyle}></span>
    </button>
  `;
}

function secondaryDivider() {
  return `<span class="secondary-divider" aria-hidden="true"></span>`;
}

function selectToolbarHtml() {
  const mode = editorState.selectMode;
  return [
    toolbarButton("select-free", "Free selection", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6c5-4 14 1 13 7-1 7-12 7-14 1"/></svg>`, mode === "free"),
    toolbarButton("select-box", "Box selection", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" stroke-dasharray="2 2"/></svg>`, mode === "box"),
    secondaryDivider(),
    toolbarButton("align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 5v14"/><path d="M9 7h9"/><path d="M9 12h6"/><path d="M9 17h11"/></svg>`),
    toolbarButton("align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 5v14"/><path d="M6 7h9"/><path d="M9 12h6"/><path d="M4 17h11"/></svg>`),
    toolbarButton("align-top", "Align top", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 9v9"/><path d="M12 9v6"/><path d="M17 9v11"/></svg>`),
    toolbarButton("align-bottom", "Align bottom", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 18h14"/><path d="M7 6v9"/><path d="M12 9v6"/><path d="M17 4v11"/></svg>`),
    toolbarButton("align-center", "Center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path d="M4 12h16"/><rect x="8" y="8" width="8" height="8"/></svg>`),
    toolbarButton("align-h-center", "Horizontal center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path d="M6 7h12"/><path d="M8 12h8"/><path d="M5 17h14"/></svg>`),
    secondaryDivider(),
    toolbarButton("distribute", "Distribute spacing", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="4" height="4"/><rect x="15" y="5" width="4" height="4"/><rect x="10" y="15" width="4" height="4"/><path d="M7 12h10"/></svg>`),
    toolbarButton("distribute-h", "Horizontal distribute", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="8" width="4" height="8"/><rect x="10" y="8" width="4" height="8"/><rect x="16" y="8" width="4" height="8"/><path d="M4 19h16"/></svg>`),
    secondaryDivider(),
    toolbarButton("flip-h", "Flip horizontal", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path class="filled" d="M5 7v10l5-5z"/><path d="M19 7v10l-5-5z"/></svg>`),
    toolbarButton("flip-v", "Flip vertical", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h16"/><path class="filled" d="M7 5h10l-5 5z"/><path d="M7 19h10l-5-5z"/></svg>`),
  ].join("");
}

function bondToolbarHtml() {
  const type = editorState.bondType;
  return [
    toolbarButton("bond-single", "Single bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 19 7"/></svg>`, type === "single"),
    toolbarButton("bond-double", "Double bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 15 18 6"/><path d="M6 18 19 9"/></svg>`, type === "double"),
    toolbarButton("bond-triple", "Triple bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4.5 14 17.5 5"/><path d="M6 17 19 8"/><path d="M7.5 20 20.5 11"/></svg>`, type === "triple"),
    toolbarButton("bond-dashed", "Dashed bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 7 15.5"/><path d="M9.5 13.8 11.5 12.4"/><path d="M14 10.6 16 9.2"/><path d="M18.5 7.5 19 7"/></svg>`, type === "dashed"),
    toolbarButton("bond-dashed-double", "Dashed-solid double bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 14 18 5"/><path d="M6 18 8 16.6"/><path d="M10.5 14.8 12.5 13.4"/><path d="M15 11.6 17 10.2"/></svg>`, type === "dashed-double"),
    toolbarButton("bond-bold", "Bold bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 19 7" style="stroke-width:3.4"/></svg>`, type === "bold"),
    toolbarButton("bond-bold-dashed", "Bold dashed bond", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 8 15" style="stroke-width:3.2"/><path d="M10.5 13.2 13.5 11.2" style="stroke-width:3.2"/><path d="M16 9.5 19 7.5" style="stroke-width:3.2"/></svg>`, type === "bold-dashed"),
    toolbarButton("bond-wedge", "Solid wedge", `<svg viewBox="0 0 24 24" aria-hidden="true"><path class="filled" d="M6 17 19 6 14 19z"/></svg>`, type === "wedge"),
    toolbarButton("bond-hashed-wedge", "Hashed wedge", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 17 19 6"/><path d="M9 16.4 7.6 14.9"/><path d="M12 15.5 9.4 12.7"/><path d="M15 14.6 11.2 10.6"/><path d="M18 13.7 13 8.4"/></svg>`, type === "hashed-wedge"),
  ].join("");
}

function textToolbarHtml() {
  return `
    <select class="secondary-select" data-text-control="font" aria-label="Font family">
      <option>Arial</option>
      <option>Helvetica</option>
      <option>Times New Roman</option>
      <option>Courier New</option>
      <option>TeX Gyre Heros</option>
    </select>
    <input class="secondary-input" data-text-control="size" aria-label="Font size" value="12" />
    ${secondaryDivider()}
    ${colorButton("text-black", "Black text", "#000000", editorState.textColor === "#000000")}
    ${colorButton("text-red", "Red text", "#ff0000", editorState.textColor === "#ff0000")}
    ${colorButton("text-blue", "Blue text", "#0000ff", editorState.textColor === "#0000ff")}
    ${colorButton("text-green", "Green text", "#0a8f3c", editorState.textColor === "#0a8f3c")}
  `;
}

function shapeToolbarHtml() {
  return `
    ${colorButton("stroke-black", "Black border", "#000000", editorState.shapeStroke === "#000000")}
    ${colorButton("stroke-red", "Red border", "#ff0000", editorState.shapeStroke === "#ff0000")}
    ${colorButton("stroke-blue", "Blue border", "#0000ff", editorState.shapeStroke === "#0000ff")}
    ${secondaryDivider()}
    ${colorButton("fill-none", "No fill", "none", editorState.shapeFill === "none")}
    ${colorButton("fill-white", "White fill", "#ffffff", editorState.shapeFill === "#ffffff")}
    ${colorButton("fill-black", "Black fill", "#000000", editorState.shapeFill === "#000000")}
    ${colorButton("fill-gray", "Gray fill", "#808892", editorState.shapeFill === "#808892")}
    ${secondaryDivider()}
    ${toolbarButton("shape-rect", "Rectangle", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14"/></svg>`, editorState.shapeStyle === "rect")}
    ${toolbarButton("shape-dashed", "Dashed outline", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" stroke-dasharray="2 2"/></svg>`, editorState.shapeStyle === "dashed")}
    ${toolbarButton("shape-filled", "Filled rectangle", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect class="filled" x="5" y="5" width="14" height="14"/></svg>`, editorState.shapeStyle === "filled")}
    ${toolbarButton("shape-ellipse", "Ellipse", `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="7"/></svg>`, editorState.shapeStyle === "ellipse")}
  `;
}

function ringSvg(sides, aromatic = false) {
  if (aromatic) {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m12 4 7 4v8l-7 4-7-4V8z"/><path d="M8.5 9.5v5"/><path d="M15.5 9.5v5"/></svg>`;
  }
  const pointsBySide = {
    3: "12,4 20,18 4,18",
    4: "6,6 18,6 18,18 6,18",
    5: "12,4 20,10 17,19 7,19 4,10",
    6: "12,4 19,8 19,16 12,20 5,16 5,8",
    7: "12,4 18,7 20,14 16,20 8,20 4,14 6,7",
    8: "9,4 15,4 20,9 20,15 15,20 9,20 4,15 4,9",
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon points="${pointsBySide[sides]}"/></svg>`;
}

function templatesToolbarHtml() {
  return [
    toolbarButton("ring-3", "3-membered ring", ringSvg(3), editorState.template === "ring-3"),
    toolbarButton("ring-4", "4-membered ring", ringSvg(4), editorState.template === "ring-4"),
    toolbarButton("ring-5", "5-membered ring", ringSvg(5), editorState.template === "ring-5"),
    toolbarButton("ring-6", "6-membered ring", ringSvg(6), editorState.template === "ring-6"),
    toolbarButton("ring-7", "7-membered ring", ringSvg(7), editorState.template === "ring-7"),
    toolbarButton("ring-8", "8-membered ring", ringSvg(8), editorState.template === "ring-8"),
    secondaryDivider(),
    toolbarButton("benzene", "Benzene ring", ringSvg(6, true), editorState.template === "benzene"),
  ].join("");
}

function renderSecondaryToolbar() {
  if (!secondaryToolbar) {
    return;
  }
  if (editorState.activeTool === "bond") {
    secondaryToolbar.innerHTML = bondToolbarHtml();
  } else if (editorState.activeTool === "text") {
    secondaryToolbar.innerHTML = textToolbarHtml();
  } else if (editorState.activeTool === "shape") {
    secondaryToolbar.innerHTML = shapeToolbarHtml();
  } else if (editorState.activeTool === "templates") {
    secondaryToolbar.innerHTML = templatesToolbarHtml();
  } else {
    secondaryToolbar.innerHTML = selectToolbarHtml();
  }
}

function setActiveTool(toolButton) {
  editorState.activeTool = toolButton?.dataset?.tool || editorState.activeTool;
  document.querySelectorAll(".tool-button").forEach((button) => {
    button.classList.toggle("is-active", button === toolButton);
  });
  syncEngineToolState();
  renderSecondaryToolbar();
}

document.querySelectorAll(".tool-button").forEach((button) => {
  button.addEventListener("click", () => {
    setActiveTool(button);
  });
});

secondaryToolbar?.addEventListener("click", (event) => {
  const button = event.target.closest("[data-secondary-value]");
  if (!button) {
    return;
  }
  const value = button.dataset.secondaryValue;
  if (value === "select-free" || value === "select-box") {
    editorState.selectMode = value.replace("select-", "");
  } else if (value?.startsWith("bond-")) {
    editorState.bondType = value.replace("bond-", "");
  } else if (value?.startsWith("shape-")) {
    editorState.shapeStyle = value.replace("shape-", "");
  } else if (value?.startsWith("ring-") || value === "benzene") {
    editorState.template = value;
  } else if (value?.startsWith("text-")) {
    const colors = { "text-black": "#000000", "text-red": "#ff0000", "text-blue": "#0000ff", "text-green": "#0a8f3c" };
    editorState.textColor = colors[value] || editorState.textColor;
  } else if (value?.startsWith("stroke-")) {
    const colors = { "stroke-black": "#000000", "stroke-red": "#ff0000", "stroke-blue": "#0000ff" };
    editorState.shapeStroke = colors[value] || editorState.shapeStroke;
  } else if (value?.startsWith("fill-")) {
    const fills = { "fill-none": "none", "fill-white": "#ffffff", "fill-black": "#000000", "fill-gray": "#808892" };
    editorState.shapeFill = fills[value] || editorState.shapeFill;
  }
  syncEngineToolState();
  renderSecondaryToolbar();
});

renderSecondaryToolbar();

function svgPointFromEvent(event) {
  const screenMatrix = viewerSvg.getScreenCTM?.();
  if (screenMatrix) {
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(screenMatrix.inverse());
    return { x: point.x, y: point.y };
  }
  const rect = viewerSvg.getBoundingClientRect();
  const viewBox = viewerSvg.viewBox.baseVal;
  const page = state.currentDocument?.document?.page || { width: 1200, height: 800 };
  const width = viewBox?.width || rect.width || page.width;
  const height = viewBox?.height || rect.height || page.height;
  return {
    x: (event.clientX - rect.left) * (width / Math.max(1, rect.width)) + (viewBox?.x || 0),
    y: (event.clientY - rect.top) * (height / Math.max(1, rect.height)) + (viewBox?.y || 0),
  };
}

function editorBondStrokeWidth() {
  const style = state.currentDocument?.styles?.style_molecule_default;
  return Number(style?.strokeWidth || style?.stroke_width || BOND_STROKE);
}

function routeEditorPointerEvents() {
  return isEditingRustDocument() && (editorState.activeTool === "bond" || editorState.activeTool === "select");
}

function handleEditorPointerMove(event) {
  if (!routeEditorPointerEvents()) {
    if (isEditingRustDocument()) {
      state.editorEngine.clearInteraction();
      renderEditorOverlay();
    }
    return;
  }
  const point = svgPointFromEvent(event);
  state.editorEngine.pointerMove(point.x, point.y);
  renderEditorOverlay();
}

function handleEditorPointerDown(event) {
  if (!routeEditorPointerEvents() || event.button !== 0) {
    return;
  }
  const point = svgPointFromEvent(event);
  event.preventDefault();
  viewerSvg.setPointerCapture?.(event.pointerId);
  state.editorEngine.pointerDown(point.x, point.y);
  syncDocumentFromEngine();
  renderEditorOverlay();
}

function handleEditorPointerUp(event) {
  if (!routeEditorPointerEvents()) {
    return;
  }
  const point = svgPointFromEvent(event);
  event.preventDefault();
  viewerSvg.releasePointerCapture?.(event.pointerId);
  state.editorEngine.pointerUp(point.x, point.y);
  syncDocumentFromEngine();
  renderDocument();
}

function handleEditorPointerLeave() {
  if (!isEditingRustDocument()) {
    return;
  }
  state.editorEngine.clearInteraction();
  renderEditorOverlay();
}

function renderEditorOverlay() {
  viewerSvg.querySelector('[data-layer="editor-overlay"]')?.remove();
  if (!isEditingRustDocument()) {
    return;
  }
  const renderList = parseEngineJson(state.editorEngine.renderListJson(), []) || [];
  const overlay = makeSvgNode("g", { "data-layer": "editor-overlay", "pointer-events": "none" });
  for (const primitive of renderList) {
    if (primitive.role === "document-bond") {
      continue;
    }
    if (primitive.kind === "line" && primitive.from && primitive.to) {
      const className = primitive.role === "selection-bond" ? "editor-selection-bond" : "editor-bond-preview";
      overlay.appendChild(makeSvgNode("line", {
        x1: primitive.from.x,
        y1: primitive.from.y,
        x2: primitive.to.x,
        y2: primitive.to.y,
        class: className,
        "stroke-width": primitive.strokeWidth || primitive.stroke_width || editorBondStrokeWidth(),
        "data-role": primitive.role,
      }));
    } else if (primitive.kind === "polygon" && Array.isArray(primitive.points)) {
      const className = primitive.role === "hover-bond-center" ? "editor-bond-center-rect" : "";
      if (!className) {
        continue;
      }
      overlay.appendChild(makeSvgNode("polygon", {
        points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
        class: className,
        "data-role": primitive.role,
      }));
    } else if (primitive.kind === "circle" && primitive.center) {
      const classByRole = {
        "hover-endpoint": "editor-endpoint-halo",
        "hover-bond-center": "editor-bond-center-halo",
        "preview-end": "editor-preview-end",
        "selection-node": "editor-selection-node",
      };
      const className = classByRole[primitive.role];
      if (!className) {
        continue;
      }
      overlay.appendChild(makeSvgNode("circle", {
        cx: primitive.center.x,
        cy: primitive.center.y,
        r: primitive.radius,
        class: className,
        "data-role": primitive.role,
      }));
    }
  }
  if (overlay.childNodes.length) {
    viewerSvg.appendChild(overlay);
  }
}

viewerSvg?.addEventListener("pointermove", handleEditorPointerMove);
viewerSvg?.addEventListener("pointerdown", handleEditorPointerDown);
viewerSvg?.addEventListener("pointerup", handleEditorPointerUp);
viewerSvg?.addEventListener("pointercancel", () => {
  state.editorEngine?.clearInteraction?.();
  renderEditorOverlay();
});
viewerSvg?.addEventListener("pointerleave", handleEditorPointerLeave);

function parseMolblock(molblock) {
  const lines = molblock.replace(/\r/g, "").split("\n");
  if (lines.length < 4) {
    return null;
  }

  const countsLine = lines[3] || "";
  const atomCount = Number.parseInt(countsLine.slice(0, 3).trim(), 10);
  const bondCount = Number.parseInt(countsLine.slice(3, 6).trim(), 10);
  if (!Number.isFinite(atomCount) || !Number.isFinite(bondCount)) {
    return null;
  }

  const atoms = [];
  const bonds = [];
  const charges = new Map();
  const sgroups = new Map();

  for (let i = 0; i < atomCount; i += 1) {
    const line = lines[4 + i] || "";
    const x = Number.parseFloat(line.slice(0, 10).trim());
    const y = Number.parseFloat(line.slice(10, 20).trim());
    const symbol = line.slice(31, 34).trim() || "C";
    atoms.push({ x, y, symbol, charge: 0 });
  }

  for (let i = 0; i < bondCount; i += 1) {
    const line = lines[4 + atomCount + i] || "";
    const begin = Number.parseInt(line.slice(0, 3).trim(), 10) - 1;
    const end = Number.parseInt(line.slice(3, 6).trim(), 10) - 1;
    const order = Number.parseInt(line.slice(6, 9).trim(), 10) || 1;
    const stereo = Number.parseInt(line.slice(9, 12).trim(), 10) || 0;
    bonds.push({ begin, end, order, stereo });
  }

  for (let i = 4 + atomCount + bondCount; i < lines.length; i += 1) {
    const line = lines[i] || "";
    if (line.startsWith("M  CHG")) {
      const parts = line.trim().split(/\s+/);
      const pairCount = Number.parseInt(parts[2], 10) || 0;
      for (let j = 0; j < pairCount; j += 1) {
        const atomIndex = Number.parseInt(parts[3 + j * 2], 10) - 1;
        const charge = Number.parseInt(parts[4 + j * 2], 10) || 0;
        charges.set(atomIndex, charge);
      }
    } else if (line.startsWith("M  STY")) {
      const parts = line.trim().split(/\s+/);
      const pairCount = Number.parseInt(parts[2], 10) || 0;
      for (let j = 0; j < pairCount; j += 1) {
        const sgId = parts[3 + j * 2];
        const sgType = parts[4 + j * 2];
        if (!sgroups.has(sgId)) {
          sgroups.set(sgId, { id: sgId, type: sgType, atoms: [], label: "", bonds: [], vectors: new Map() });
        } else {
          sgroups.get(sgId).type = sgType;
        }
      }
    } else if (line.startsWith("M  SAL")) {
      const parts = line.trim().split(/\s+/);
      const sgId = parts[2];
      const count = Number.parseInt(parts[3], 10) || 0;
      if (!sgroups.has(sgId)) {
        sgroups.set(sgId, { id: sgId, type: "", atoms: [], label: "", bonds: [], vectors: new Map() });
      }
      const sg = sgroups.get(sgId);
      for (let j = 0; j < count; j += 1) {
        sg.atoms.push((Number.parseInt(parts[4 + j], 10) || 1) - 1);
      }
    } else if (line.startsWith("M  SBL")) {
      const parts = line.trim().split(/\s+/);
      const sgId = parts[2];
      const count = Number.parseInt(parts[3], 10) || 0;
      if (!sgroups.has(sgId)) {
        sgroups.set(sgId, { id: sgId, type: "", atoms: [], label: "", bonds: [], vectors: new Map() });
      }
      const sg = sgroups.get(sgId);
      for (let j = 0; j < count; j += 1) {
        sg.bonds.push((Number.parseInt(parts[4 + j], 10) || 1) - 1);
      }
    } else if (line.startsWith("M  SMT")) {
      const parts = line.trim().split(/\s+/);
      const sgId = parts[2];
      const label = parts.slice(3).join(" ").replace(/\\s\^/g, "").replace(/\\n/g, "");
      if (!sgroups.has(sgId)) {
        sgroups.set(sgId, { id: sgId, type: "", atoms: [], label: "", bonds: [], vectors: new Map() });
      }
      sgroups.get(sgId).label = label;
    } else if (line.startsWith("M  SBV")) {
      const parts = line.trim().split(/\s+/);
      const sgId = parts[2];
      const bondIndex = (Number.parseInt(parts[3], 10) || 1) - 1;
      const vx = Number.parseFloat(parts[4]) || 0;
      const vy = Number.parseFloat(parts[5]) || 0;
      if (!sgroups.has(sgId)) {
        sgroups.set(sgId, { id: sgId, type: "", atoms: [], label: "", bonds: [], vectors: new Map() });
      }
      sgroups.get(sgId).vectors.set(bondIndex, { x: vx, y: vy });
    }
  }

  for (const [atomIndex, charge] of charges.entries()) {
    if (atoms[atomIndex]) {
      atoms[atomIndex].charge = charge;
    }
  }

  const xs = atoms.map((atom) => atom.x);
  const ys = atoms.map((atom) => atom.y);
  return {
    atoms,
    bonds,
    sgroups: [...sgroups.values()],
    minX: Math.min(...xs),
    maxX: Math.max(...xs),
    minY: Math.min(...ys),
    maxY: Math.max(...ys),
  };
}

function atomDegree(parsedMol, atomIndex) {
  let degree = 0;
  for (const bond of parsedMol.bonds) {
    if (bond.begin === atomIndex || bond.end === atomIndex) {
      degree += 1;
    }
  }
  return degree;
}

function atomNeedsLabel(parsedMol, atomIndex) {
  const atom = parsedMol.atoms[atomIndex];
  if (atom.symbol !== "C") {
    return true;
  }
  return atomDegree(parsedMol, atomIndex) === 0;
}

function bondNeighbors(parsedMol, atomIndex, excludedAtomIndex) {
  const neighbors = [];
  for (const bond of parsedMol.bonds) {
    if (bond.begin === atomIndex && bond.end !== excludedAtomIndex) {
      neighbors.push(parsedMol.atoms[bond.end]);
    } else if (bond.end === atomIndex && bond.begin !== excludedAtomIndex) {
      neighbors.push(parsedMol.atoms[bond.begin]);
    }
  }
  return neighbors;
}

function mapAtomPoint(parsedMol, atom, bbox) {
  const width = parsedMol.maxX - parsedMol.minX || 1;
  const height = parsedMol.maxY - parsedMol.minY || 1;
  const scale = Math.min(bbox.width / width, bbox.height / height);
  const offsetX = (bbox.width - width * scale) / 2;
  const offsetY = (bbox.height - height * scale) / 2;
  const x = (atom.x - parsedMol.minX) * scale + offsetX;
  const y = (parsedMol.maxY - atom.y) * scale + offsetY;
  return { x, y };
}

function makeSvgNode(name, attributes = {}) {
  const node = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [key, value] of Object.entries(attributes)) {
    if (value == null || value === undefined || value === "") {
      continue;
    }
    node.setAttribute(key, String(value));
  }
  return node;
}

function ensureSvgDefs(svgRoot) {
  let defs = svgRoot.querySelector("defs");
  if (!defs) {
    defs = makeSvgNode("defs");
    svgRoot.appendChild(defs);
  }
  return defs;
}

function lineEndpointsWithLabelPadding(start, end, startPad, endPad) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  return {
    start: {
      x: start.x + (dx / length) * startPad,
      y: start.y + (dy / length) * startPad,
    },
    end: {
      x: end.x - (dx / length) * endPad,
      y: end.y - (dy / length) * endPad,
    },
  };
}

function chooseDoubleBondSide(parsedMol, bond) {
  const a = parsedMol.atoms[bond.begin];
  const b = parsedMol.atoms[bond.end];
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = Math.hypot(dx, dy) || 1;
  const normalX = -dy / length;
  const normalY = dx / length;

  let score = 0;
  for (const neighbor of bondNeighbors(parsedMol, bond.begin, bond.end)) {
    score += (neighbor.x - a.x) * normalX + (neighbor.y - a.y) * normalY;
  }
  for (const neighbor of bondNeighbors(parsedMol, bond.end, bond.begin)) {
    score += (neighbor.x - b.x) * normalX + (neighbor.y - b.y) * normalY;
  }
  return score >= 0 ? -1 : 1;
}

function renderHashedWedge(group, start, end, color, options = {}) {
  const geometry = computeHashedWedgeGeometry(start, end, options.targetBoxes || null, {
    spacing: HASH_WEDGE_SPACING,
    startOffset: HASH_WEDGE_START_OFFSET,
    endInset: HASH_WEDGE_END_INSET,
  });
  for (const segment of geometry.segments) {
    group.appendChild(
      makeSvgNode("line", {
        x1: segment.x1,
        y1: segment.y1,
        x2: segment.x2,
        y2: segment.y2,
        stroke: color,
        "stroke-width": segment.strokeWidth,
        "stroke-linecap": "butt",
      }),
    );
  }
}

function renderSolidWedge(group, start, end, color, options = {}) {
  const geometry = computeSolidWedgeGeometry(start, end, options.targetBoxes || null, {
    endInset: options.targetHasLabel ? SOLID_WEDGE_END_INSET : 0,
    wideContactDirection: options.wideContactDirection || null,
    wideContactDirections: options.wideContactDirections || null,
  });
  group.appendChild(
    makeSvgNode("polygon", {
      points: geometry.points.map((point) => `${point.x},${point.y}`).join(" "),
      "data-role": "solid-wedge",
      "data-wide-contact": geometry.wideContact ? "true" : null,
      "data-wide-contact-count": geometry.wideContactCount || null,
      fill: color,
    }),
  );
}

function retreatBondSegmentEndpoints(start, end, options = {}) {
  let nextStart = start;
  let nextEnd = end;
  const gap = options.shapeGap ?? (BOND_STROKE * 0.5 + 0.045);
  if (options.startBoxes?.length) {
    nextStart = retreatPointFromShapes(nextStart, nextEnd, options.startBoxes, gap);
  }
  if (options.endBoxes?.length) {
    nextEnd = retreatPointFromShapes(nextEnd, nextStart, options.endBoxes, gap);
  }
  return { start: nextStart, end: nextEnd };
}

function renderBondLines(group, start, end, offsets, color, options = {}) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  const normalX = -dy / length;
  const normalY = dx / length;

  for (const offset of offsets) {
    const segment = retreatBondSegmentEndpoints({
      x: start.x + normalX * offset,
      y: start.y + normalY * offset,
    }, {
      x: end.x + normalX * offset,
      y: end.y + normalY * offset,
    }, options);
    renderBondSegment(group, {
      ...segment,
      color,
      strokeWidth: options.strokeWidth,
    });
  }
}

function renderBondSegment(group, { start, end, color, strokeWidth = BOND_STROKE }) {
  group.appendChild(
    makeSvgNode("line", {
      x1: start.x,
      y1: start.y,
      x2: end.x,
      y2: end.y,
      class: "mol-bond",
      stroke: color,
      "stroke-width": strokeWidth,
    }),
  );
}

function isMolBondSolidWedge(bond) {
  return Number(bond?.stereo || 0) === 1;
}

function isMolBondSideDouble(bond, parsedMol) {
  if (Number(bond?.order || 1) !== 2) {
    return false;
  }
  const degreeA = atomDegree(parsedMol, bond.begin);
  const degreeB = atomDegree(parsedMol, bond.end);
  return degreeA > 1 && degreeB > 1;
}

function isMolBondWideContactCandidate(bond, parsedMol) {
  if (Number(bond?.order || 1) === 1 && Number(bond?.stereo || 0) === 0) {
    return true;
  }
  return isMolBondSideDouble(bond, parsedMol);
}

function hasVisibleMolAtomLabel(parsedMol, atomIndex, hiddenAtoms = null) {
  if (hiddenAtoms?.has(atomIndex)) {
    return false;
  }
  const atom = parsedMol?.atoms?.[atomIndex];
  return Boolean(atom && atom.symbol && atom.symbol !== "C");
}

function solidWedgeWideContactDirectionsForMol(bond, wideAtomIndex, parsedMol, atomPoints, hiddenAtoms = null) {
  if (hasVisibleMolAtomLabel(parsedMol, wideAtomIndex, hiddenAtoms)) {
    return [];
  }
  const widePoint = atomPoints[wideAtomIndex];
  if (!widePoint) {
    return [];
  }
  const directions = [];
  for (const otherBond of parsedMol?.bonds || []) {
    if (otherBond === bond || isMolBondSolidWedge(otherBond) || !isMolBondWideContactCandidate(otherBond, parsedMol)) {
      continue;
    }
    if (otherBond.begin !== wideAtomIndex && otherBond.end !== wideAtomIndex) {
      continue;
    }
    const otherAtomIndex = otherBond.begin === wideAtomIndex ? otherBond.end : otherBond.begin;
    if (hiddenAtoms?.has(otherAtomIndex)) {
      continue;
    }
    const otherPoint = atomPoints[otherAtomIndex];
    if (!otherPoint) {
      continue;
    }
    const dx = otherPoint.x - widePoint.x;
    const dy = otherPoint.y - widePoint.y;
    if (Math.hypot(dx, dy) > 1e-6) {
      directions.push({ x: dx, y: dy });
    }
  }
  return directions;
}

function renderRetreatedBondSegment(group, { start, end, color, startBoxes = null, endBoxes = null, shapeGap = null, strokeWidth = BOND_STROKE }) {
  const segment = retreatBondSegmentEndpoints(start, end, { startBoxes, endBoxes, shapeGap });
  renderBondSegment(group, {
    ...segment,
    color,
    strokeWidth,
  });
}

function insetBondSegment(start, end, insetStart = 0, insetEnd = 0) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const clampedStart = Math.max(0, Math.min(insetStart, length * 0.45));
  const clampedEnd = Math.max(0, Math.min(insetEnd, length * 0.45));
  return {
    start: {
      x: start.x + ux * clampedStart,
      y: start.y + uy * clampedStart,
    },
    end: {
      x: end.x - ux * clampedEnd,
      y: end.y - uy * clampedEnd,
    },
  };
}

function renderDoubleBond(group, start, end, doublePosition, color, options = {}) {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const length = Math.hypot(dx, dy) || 1;
  const normalX = -dy / length;
  const normalY = dx / length;
  const strokeWidth = Number(options.strokeWidth || BOND_STROKE);
  const scale = strokeWidth / BOND_STROKE;
  const doubleOffset = DOUBLE_BOND_OFFSET * scale;
  const sideInset = Math.max(DOUBLE_BOND_SIDE_INSET * scale, length * DOUBLE_BOND_SIDE_INSET_RATIO);
  const terminalStart = Boolean(options.terminalStart);
  const terminalEnd = Boolean(options.terminalEnd);
  const alignStart = terminalStart || Boolean(options.alignStart);
  const alignEnd = terminalEnd || Boolean(options.alignEnd);

  const sideMode = String(doublePosition || "").toLowerCase();
  if (sideMode === "left" || sideMode === "right") {
    const side = sideMode === "left" ? 1 : -1;
    renderRetreatedBondSegment(group, {
      start,
      end,
      color,
      startBoxes: options.startBoxes,
      endBoxes: options.endBoxes,
      shapeGap: options.shapeGap,
      strokeWidth,
    });
    const offsetStart = {
      x: start.x + normalX * doubleOffset * side,
      y: start.y + normalY * doubleOffset * side,
    };
    const offsetEnd = {
      x: end.x + normalX * doubleOffset * side,
      y: end.y + normalY * doubleOffset * side,
    };
    const joinedStart = options.shortStartPoint || null;
    const joinedEnd = options.shortEndPoint || null;
    const shortSegment = insetBondSegment(
      joinedStart || offsetStart,
      joinedEnd || offsetEnd,
      (joinedStart || alignStart) ? 0 : sideInset,
      (joinedEnd || alignEnd) ? 0 : sideInset,
    );
    renderRetreatedBondSegment(group, {
      ...shortSegment,
      color,
      startBoxes: options.startBoxes,
      endBoxes: options.endBoxes,
      shapeGap: options.shapeGap,
      strokeWidth,
    });
    return;
  }

  renderBondLines(group, start, end, [-doubleOffset / 2, doubleOffset / 2], color, {
    ...options,
    strokeWidth,
  });
}

function renderBond(group, parsedMol, bond, atomPoints, hiddenAtoms, labelMetrics) {
  if (hiddenAtoms.has(bond.begin) || hiddenAtoms.has(bond.end)) {
    return;
  }

  const startPoint = atomPoints[bond.begin];
  const endPoint = atomPoints[bond.end];
  const startPad = labelMetrics.get(bond.begin)?.pad || 0;
  const endPad = labelMetrics.get(bond.end)?.pad || 0;
  let start = startPoint;
  let end = endPoint;
  const color = CHEMDRAW_INK;

  if (bond.stereo === 1) {
    ({ start, end } = lineEndpointsWithLabelPadding(startPoint, endPoint, startPad, endPad));
    renderSolidWedge(group, start, end, color, {
      targetHasLabel: endPad > 0,
      targetBoxes: null,
      wideContactDirections: solidWedgeWideContactDirectionsForMol(bond, bond.end, parsedMol, atomPoints, hiddenAtoms),
    });
    return;
  }
  if (bond.stereo === 6) {
    ({ start, end } = lineEndpointsWithLabelPadding(startPoint, endPoint, startPad, endPad));
    renderHashedWedge(group, start, end, color, {
      targetHasLabel: endPad > 0,
    });
    return;
  }

  if (bond.order === 2) {
    const degreeA = atomDegree(parsedMol, bond.begin);
    const degreeB = atomDegree(parsedMol, bond.end);
    if (degreeA === 1 || degreeB === 1) {
      renderDoubleBond(group, start, end, null, color);
    } else {
      const side = chooseDoubleBondSide(parsedMol, bond);
      renderDoubleBond(group, start, end, side < 0 ? "Left" : "Right", color, {
        terminalStart: degreeA === 1,
        terminalEnd: degreeB === 1,
        alignStart: startPad > 0,
        alignEnd: endPad > 0,
      });
    }
    return;
  }

  if (bond.order >= 3) {
    renderBondLines(group, start, end, [-TRIPLE_BOND_OFFSET, 0, TRIPLE_BOND_OFFSET], color);
    return;
  }

  renderBondLines(group, start, end, [0], color);
}

function localPointFromAbsolute(point, originX, originY) {
  return {
    x: Number(point[0]) - originX,
    y: Number(point[1]) - originY,
  };
}

function localBoxFromAbsolute(box, originX, originY) {
  if (!box || box.length < 4) {
    return null;
  }
  return {
    x1: Number(box[0]) - originX,
    y1: Number(box[1]) - originY,
    x2: Number(box[2]) - originX,
    y2: Number(box[3]) - originY,
  };
}

function centeredLabelBox(point, radiusX = 3.6, radiusY = 3.6) {
  return {
    x1: point.x - radiusX,
    y1: point.y - radiusY,
    x2: point.x + radiusX,
    y2: point.y + radiusY,
  };
}

function pointInBox(point, box, margin = 0) {
  return pointInOpticalShape(point, box, margin);
}

function clipPointOutOfBox(start, end, box, margin = 1.5) {
  const expanded = {
    x1: box.x1 - margin,
    y1: box.y1 - margin,
    x2: box.x2 + margin,
    y2: box.y2 + margin,
  };
  if (!pointInBox(start, expanded)) {
    return start;
  }

  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const candidates = [];

  if (Math.abs(dx) > 1e-6) {
    const t1 = (expanded.x1 - start.x) / dx;
    const y1 = start.y + dy * t1;
    if (t1 >= 0 && t1 <= 1 && y1 >= expanded.y1 && y1 <= expanded.y2) {
      candidates.push({ t: t1, x: expanded.x1, y: y1 });
    }
    const t2 = (expanded.x2 - start.x) / dx;
    const y2 = start.y + dy * t2;
    if (t2 >= 0 && t2 <= 1 && y2 >= expanded.y1 && y2 <= expanded.y2) {
      candidates.push({ t: t2, x: expanded.x2, y: y2 });
    }
  }

  if (Math.abs(dy) > 1e-6) {
    const t3 = (expanded.y1 - start.y) / dy;
    const x3 = start.x + dx * t3;
    if (t3 >= 0 && t3 <= 1 && x3 >= expanded.x1 && x3 <= expanded.x2) {
      candidates.push({ t: t3, x: x3, y: expanded.y1 });
    }
    const t4 = (expanded.y2 - start.y) / dy;
    const x4 = start.x + dx * t4;
    if (t4 >= 0 && t4 <= 1 && x4 >= expanded.x1 && x4 <= expanded.x2) {
      candidates.push({ t: t4, x: x4, y: expanded.y2 });
    }
  }

  if (!candidates.length) {
    return start;
  }
  candidates.sort((a, b) => a.t - b.t);
  return { x: candidates[0].x, y: candidates[0].y };
}

function lineEndpointsForFragmentBond(start, end, startLabelBox, endLabelBox) {
  let startPoint = { ...start };
  let endPoint = { ...end };
  if (startLabelBox) {
    startPoint = clipPointOutOfBox(startPoint, endPoint, startLabelBox, startLabelBox.margin ?? 1.8);
  }
  if (endLabelBox) {
    endPoint = clipPointOutOfBox(endPoint, startPoint, endLabelBox, endLabelBox.margin ?? 1.8);
  }
  return { start: startPoint, end: endPoint };
}

function retreatPointFromShapes(point, target, shapes, gap = 0.05) {
  if (!shapes?.length) {
    return point;
  }
  const expandedShapes = shapes
    .map((shape) => expandOpticalShape(shape, gap))
    .filter((shape) => pointInOpticalShape(point, shape));
  if (!expandedShapes.length) {
    return point;
  }
  const direction = {
    x: target.x - point.x,
    y: target.y - point.y,
  };
  const shift = shiftRayBoxes(point, direction, expandedShapes);
  if (shift <= 0) {
    return point;
  }
  return offsetPointAlongRay(point, direction, shift + 0.01);
}

function labelBox(label, originX, originY) {
  const box = label?.box || label?.bbox || null;
  return box ? localBoxFromAbsolute(box, originX, originY) : null;
}

function labelSourceText(label) {
  return label?.sourceText || label?.inputText || label?.text || "";
}

function labelLayoutMode(label) {
  return label?.layout || label?.layoutMode || "default";
}

function labelAttachment(label) {
  return label?.attachment || label?.attachmentLayout || label?.labelAlignment || "";
}

function labelConnectionAnchor(label) {
  return label?.anchor || label?.connectionAnchor || "start";
}

function attachedGroupCollisionBox(node, originX, originY) {
  const label = node?.label;
  const box = labelBox(label, originX, originY);
  if (!box) {
    return null;
  }
  const width = box.x2 - box.x1;
  const localX = Number(node?.position?.[0] || 0) - originX;
  const localY = Number(node?.position?.[1] || 0) - originY;
  const distLeft = Math.abs(localX - box.x1);
  const distRight = Math.abs(box.x2 - localX);
  const anchorWidth = Math.min(2.2, Math.max(1.5, width * 0.12));
  const anchorOnRight = distRight < distLeft;

  if (anchorOnRight) {
    return {
      x1: Math.min(localX, box.x2 - anchorWidth),
      y1: box.y1,
      x2: box.x2,
      y2: box.y2,
      margin: 0.72,
    };
  }

  return {
    x1: box.x1,
    y1: box.y1,
    x2: Math.max(localX, box.x1 + anchorWidth),
    y2: box.y2,
    margin: 0.72,
  };
}

function getFragmentLabelLines(label) {
  if (Array.isArray(label?.lines) && label.lines.length) {
    return label.lines;
  }
  if (typeof label?.text === "string" && label.text.includes("\n")) {
    return label.text.split(/\n+/).filter(Boolean);
  }
  return label?.text ? [label.text] : [];
}

function isCenteredAtomicLabel(label) {
  return /^[A-Z][a-z]?$/.test(String(label?.text || ""));
}

function getChemicalLabelMode(label) {
  const layout = labelLayoutMode(label);
  if (layout && layout !== "default") {
    return layout;
  }
  if (isCenteredAtomicLabel(label)) {
    return "centered-atom";
  }
  return layout || "default";
}

function getFragmentLabelMode(node, label) {
  return getChemicalLabelMode(label);
}

function centeredAttachedGroupCollisionBox(node, originX, originY) {
  const box = labelBox(node?.label, originX, originY);
  if (box) {
    return {
      ...box,
      margin: 0.4,
    };
  }
  const point = localPointFromAbsolute(node.position, originX, originY);
  const halfWidth = 1.15;
  const halfHeight = 1.35;
  return {
    x1: point.x - halfWidth,
    y1: point.y - halfHeight,
    x2: point.x + halfWidth,
    y2: point.y + halfHeight,
    margin: 0.4,
  };
}

function fragmentNodeDegree(bonds, nodeId) {
  let degree = 0;
  for (const bond of bonds || []) {
    if (bond.begin === nodeId || bond.end === nodeId) {
      degree += 1;
    }
  }
  return degree;
}

function isFragmentBondStereo(bond) {
  const display = bond?.display || "";
  return Boolean(bond?.stereo?.kind)
    || Boolean(bond?.stereo?.wideEnd)
    || Boolean(bond?.stereoStyle)
    || Boolean(bond?.stereoEnd)
    || display === "WedgeBegin"
    || display === "WedgeEnd"
    || display === "WedgedHashBegin"
    || display === "WedgedHashEnd";
}

function isFragmentBondSideDouble(bond) {
  if (Number(bond?.order || 1) !== 2) {
    return false;
  }
  return Boolean(fragmentSideDoublePlacement(bond));
}

function fragmentSideDoublePlacement(bond) {
  if (Number(bond?.order || 1) !== 2) {
    return null;
  }
  const sideMode = String(bond.double?.placement || bond.doubleStyle || bond.doublePosition || "").toLowerCase();
  return sideMode === "left" || sideMode === "right" ? sideMode : null;
}

function doublePlacementSide(placement) {
  return String(placement || "").toLowerCase() === "left" ? 1 : -1;
}

function fragmentNodePoint(node, originX, originY) {
  return localPointFromAbsolute(node.position, originX, originY);
}

function sideDoubleOffsetLineForFragmentEndpoint(bond, sharedNodeId, nodeMap, originX, originY, doubleOffset) {
  const placement = fragmentSideDoublePlacement(bond);
  if (!placement) {
    return null;
  }
  const beginNode = nodeMap.get(bond.begin);
  const endNode = nodeMap.get(bond.end);
  if (!beginNode || !endNode || (sharedNodeId !== bond.begin && sharedNodeId !== bond.end)) {
    return null;
  }
  const begin = fragmentNodePoint(beginNode, originX, originY);
  const end = fragmentNodePoint(endNode, originX, originY);
  const dx = end.x - begin.x;
  const dy = end.y - begin.y;
  const length = Math.hypot(dx, dy);
  if (length <= 1e-6) {
    return null;
  }
  const normalX = -dy / length;
  const normalY = dx / length;
  const side = doublePlacementSide(placement);
  const shared = sharedNodeId === bond.begin ? begin : end;
  const other = sharedNodeId === bond.begin ? end : begin;
  return {
    point: {
      x: shared.x + normalX * doubleOffset * side,
      y: shared.y + normalY * doubleOffset * side,
    },
    direction: {
      x: (other.x - shared.x) / length,
      y: (other.y - shared.y) / length,
    },
    shared,
    length,
  };
}

function pointToLineDistance(point, linePoint, direction) {
  return Math.abs((point.x - linePoint.x) * direction.y - (point.y - linePoint.y) * direction.x);
}

function sideDoubleJoinPointForFragmentEndpoint(bond, sharedNodeId, bonds, nodeMap, originX, originY, doubleOffset) {
  const sharedNode = nodeMap.get(sharedNodeId);
  if (!sharedNode || hasVisibleFragmentNodeLabel(sharedNode)) {
    return null;
  }
  const currentLine = sideDoubleOffsetLineForFragmentEndpoint(bond, sharedNodeId, nodeMap, originX, originY, doubleOffset);
  if (!currentLine) {
    return null;
  }
  for (const otherBond of bonds || []) {
    if (otherBond === bond || !isFragmentBondSideDouble(otherBond)) {
      continue;
    }
    if (otherBond.begin !== sharedNodeId && otherBond.end !== sharedNodeId) {
      continue;
    }
    const otherLine = sideDoubleOffsetLineForFragmentEndpoint(otherBond, sharedNodeId, nodeMap, originX, originY, doubleOffset);
    if (!otherLine) {
      continue;
    }
    const cross = currentLine.direction.x * otherLine.direction.y - currentLine.direction.y * otherLine.direction.x;
    if (Math.abs(cross) < 1e-6) {
      if (pointToLineDistance(otherLine.point, currentLine.point, currentLine.direction) <= doubleOffset * 0.08) {
        return {
          x: (currentLine.point.x + otherLine.point.x) / 2,
          y: (currentLine.point.y + otherLine.point.y) / 2,
        };
      }
      continue;
    }
    const intersection = lineIntersectionWithParameters(
      currentLine.point,
      currentLine.direction,
      otherLine.point,
      otherLine.direction,
    );
    if (!intersection || intersection.t < -0.2 || intersection.u < -0.2) {
      continue;
    }
    const sharedDistance = Math.hypot(intersection.point.x - currentLine.shared.x, intersection.point.y - currentLine.shared.y);
    const maxJoinDistance = Math.min(currentLine.length, otherLine.length) * 0.45;
    if (sharedDistance <= Math.max(doubleOffset * 4, maxJoinDistance)) {
      return intersection.point;
    }
  }
  return null;
}

function isFragmentBondWideContactCandidate(bond) {
  if (isFragmentBondStereo(bond)) {
    return false;
  }
  if (Number(bond?.order || 1) === 1) {
    return true;
  }
  return isFragmentBondSideDouble(bond);
}

function hasVisibleFragmentNodeLabel(node) {
  const label = node?.label;
  if (!label) {
    return false;
  }
  const text = String(label.text || labelSourceText(label) || "").trim();
  return Boolean(text);
}

function solidWedgeWideContactDirections(bond, wideNodeId, bonds, nodeMap, originX, originY) {
  const wideNode = nodeMap.get(wideNodeId);
  if (!wideNode) {
    return [];
  }
  if (hasVisibleFragmentNodeLabel(wideNode)) {
    return [];
  }
  const widePoint = localPointFromAbsolute(wideNode.position, originX, originY);
  const directions = [];

  for (const otherBond of bonds || []) {
    if (otherBond.id === bond.id || !isFragmentBondWideContactCandidate(otherBond)) {
      continue;
    }
    if (otherBond.begin !== wideNodeId && otherBond.end !== wideNodeId) {
      continue;
    }
    const otherNodeId = otherBond.begin === wideNodeId ? otherBond.end : otherBond.begin;
    const otherNode = nodeMap.get(otherNodeId);
    if (!otherNode) {
      continue;
    }
    const otherPoint = localPointFromAbsolute(otherNode.position, originX, originY);
    const dx = otherPoint.x - widePoint.x;
    const dy = otherPoint.y - widePoint.y;
    if (Math.hypot(dx, dy) > 1e-6) {
      directions.push({ x: dx, y: dy });
    }
  }
  return directions;
}

function connectionPointForNode(node, mode, shape, originX, originY) {
  const nodePoint = localPointFromAbsolute(node.position, originX, originY);
  return shape?.anchor || nodePoint;
}

function collisionBoxesForNode(node, mode, shape, isStereoBond, originX = 0, originY = 0) {
  if (!shape?.charBoxes?.length) {
    return null;
  }
  if (isStereoBond) {
    return shape.charBoxes.map((entry) => entry.shape).filter(Boolean);
  }
  const anchorIndex = shape.anchor?.glyphIndex;
  if (Number.isInteger(anchorIndex)) {
    const entry = shape.charBoxes.find((item) => item.glyphIndex === anchorIndex);
    return entry?.shape ? [entry.shape] : null;
  }
  return shape.charBoxes.map((entry) => entry.shape).filter(Boolean);
}

function stereoTargetBoxesForNode(node, mode, shape) {
  if (!shape?.charBoxes?.length) {
    return null;
  }
  return shape.charBoxes.map((entry) => entry.shape).filter(Boolean);
}

function allLabelShapes(textGeometry) {
  const shapes = [];
  for (const entry of textGeometry?.values?.() || []) {
    for (const charEntry of entry.charBoxes || []) {
      if (charEntry.shape) {
        shapes.push(charEntry.shape);
      }
    }
  }
  return shapes;
}

function uniqueShapes(...shapeLists) {
  const seen = new Set();
  const shapes = [];
  for (const shape of shapeLists.flat().filter(Boolean)) {
    const key = shape.kind === "ellipse"
      ? `e:${shape.cx}:${shape.cy}:${shape.rx}:${shape.ry}`
      : `${shape.kind}:${shape.x1}:${shape.y1}:${shape.x2}:${shape.y2}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    shapes.push(shape);
  }
  return shapes;
}

function appendOpticalContactShape(group, shape, attrs = {}) {
  if (!shape) {
    return;
  }
  if (shape.kind === "ellipse") {
    group.appendChild(
      makeSvgNode("ellipse", {
        cx: shape.cx,
        cy: shape.cy,
        rx: shape.rx,
        ry: shape.ry,
        ...attrs,
      }),
    );
    return;
  }
  if (isCutCornerShape(shape)) {
    group.appendChild(
      makeSvgNode("path", {
        d: svgPathForShape(shape),
        ...attrs,
      }),
    );
    return;
  }
  group.appendChild(
    makeSvgNode("rect", {
      x: shape.x1,
      y: shape.y1,
      width: shape.x2 - shape.x1,
      height: shape.y2 - shape.y1,
      ...attrs,
    }),
  );
}

function renderFragmentLabelKnockouts(knockoutLayer, textGeometry) {
  for (const entry of textGeometry.values()) {
    for (const charEntry of entry.charBoxes || []) {
      appendOpticalContactShape(knockoutLayer, charEntry.shape, {
        class: "label-knockout-shape",
        fill: CHEMDRAW_PAGE_BACKGROUND,
      });
    }
  }
}

function clipPointOutOfBoxes(start, end, boxes, margin = 0) {
  const candidates = [];
  for (const box of boxes) {
    const expanded = {
      x1: box.x1 - margin,
      y1: box.y1 - margin,
      x2: box.x2 + margin,
      y2: box.y2 + margin,
    };
    if (!pointInBox(start, expanded)) {
      continue;
    }

    const dx = end.x - start.x;
    const dy = end.y - start.y;
    if (Math.abs(dx) > 1e-6) {
      const t1 = (expanded.x1 - start.x) / dx;
      const y1 = start.y + dy * t1;
      if (t1 >= 0 && t1 <= 1 && y1 >= expanded.y1 && y1 <= expanded.y2) {
        candidates.push({ t: t1, x: expanded.x1, y: y1 });
      }
      const t2 = (expanded.x2 - start.x) / dx;
      const y2 = start.y + dy * t2;
      if (t2 >= 0 && t2 <= 1 && y2 >= expanded.y1 && y2 <= expanded.y2) {
        candidates.push({ t: t2, x: expanded.x2, y: y2 });
      }
    }

    if (Math.abs(dy) > 1e-6) {
      const t3 = (expanded.y1 - start.y) / dy;
      const x3 = start.x + dx * t3;
      if (t3 >= 0 && t3 <= 1 && x3 >= expanded.x1 && x3 <= expanded.x2) {
        candidates.push({ t: t3, x: x3, y: expanded.y1 });
      }
      const t4 = (expanded.y2 - start.y) / dy;
      const x4 = start.x + dx * t4;
      if (t4 >= 0 && t4 <= 1 && x4 >= expanded.x1 && x4 <= expanded.x2) {
        candidates.push({ t: t4, x: x4, y: expanded.y2 });
      }
    }
  }

  if (!candidates.length) {
    return start;
  }
  candidates.sort((a, b) => a.t - b.t);
  return { x: candidates[0].x, y: candidates[0].y };
}

function shiftRayBox(point, direction, box) {
  return shiftRayOpticalShape(point, direction, box);
}

function shiftRayBoxes(point, direction, boxes) {
  return shiftRayOpticalShapes(point, direction, boxes);
}

function glyphRunsForLabel(label, runsOverride = null, textOverride = null) {
  const sourceRuns = runsOverride || label.runs;
  const glyphs = [];
  if (sourceRuns?.length) {
    for (const run of sourceRuns) {
      for (const char of Array.from(String(run.text || ""))) {
        if (char === "\n" || char === "\r") {
          continue;
        }
        glyphs.push({
          char,
          codepoint: char.codePointAt(0),
          scriptKind: scriptKindForRun(run),
          fill: normalizeDisplayColor(run.fill || label.fill),
          fontFamily: run.fontFamily || label.fontFamily || "Arial",
          fontWeight: fontWeightForRun(run),
          fontStyle: fontStyleForRun(run),
        });
      }
    }
    return glyphs;
  }

  for (const char of Array.from(String(textOverride ?? label.text ?? ""))) {
    if (char === "\n" || char === "\r") {
      continue;
    }
    glyphs.push({
      char,
      codepoint: char.codePointAt(0),
      scriptKind: /^\d$/.test(char) ? 1 : 0,
      fill: normalizeDisplayColor(label.fill),
      fontFamily: label.fontFamily || "Arial",
      fontWeight: fontWeightForRun(label),
      fontStyle: fontStyleForRun(label),
    });
  }
  return glyphs;
}

function firstVisibleGlyphIndex(glyphs) {
  return glyphs.findIndex((glyph) => /\S/.test(glyph.char || ""));
}

function lastVisibleGlyphIndex(glyphs) {
  for (let index = glyphs.length - 1; index >= 0; index -= 1) {
    if (/\S/.test(glyphs[index].char || "")) {
      return index;
    }
  }
  return -1;
}

function anchorGlyphIndexForLabel(label, glyphs) {
  const explicit = Number(label?.anchorGlyphIndex ?? label?.anchor_glyph_index);
  if (Number.isInteger(explicit) && explicit >= 0 && explicit < glyphs.length) {
    return explicit;
  }
  if (labelConnectionAnchor(label) === "end") {
    const last = lastVisibleGlyphIndex(glyphs);
    return last >= 0 ? last : null;
  }
  const first = firstVisibleGlyphIndex(glyphs);
  return first >= 0 ? first : null;
}

function alignForLabel(label, mode) {
  const layout = String(labelAttachment(label)).toLowerCase();
  if (layout === "above" || mode === "hetero-h-above" || mode === "attached-group-above") {
    return LABEL_ALIGN.above;
  }
  if (layout === "below") {
    return LABEL_ALIGN.below;
  }
  if (layout === "right" || labelConnectionAnchor(label) === "end") {
    return LABEL_ALIGN.left;
  }
  return LABEL_ALIGN.right;
}

function geometryEntryFromLayout(node, mode, layout, glyphs, lineIndex = null) {
  const charBoxes = layout.placements
    .map((placement, glyphIndex) => ({
      glyphIndex,
      char: glyphs[glyphIndex]?.char || placement.char,
      box: placement.backgroundBox,
      measuredGlyphBox: placement.inkBox,
      shape: placement.shape,
      placement,
    }))
    .filter((entry) => entry.placement.visible);
  const boxes = charBoxes.map((entry) => entry.box);
  return {
    nodeId: node.id,
    mode,
    anchor: layout.anchor,
    boxes,
    measuredGlyphBoxes: charBoxes.map((entry) => entry.measuredGlyphBox),
    charBoxes,
    textEntries: [
      {
        lineIndex,
        boxes,
        measuredGlyphBoxes: charBoxes.map((entry) => entry.measuredGlyphBox),
        charBoxes,
      },
    ],
  };
}

function mergeGeometryEntry(target, source) {
  if (!target) {
    return source;
  }
  target.boxes.push(...source.boxes);
  target.measuredGlyphBoxes.push(...source.measuredGlyphBoxes);
  target.charBoxes.push(...source.charBoxes);
  target.textEntries.push(...source.textEntries);
  if (!target.anchor && source.anchor) {
    target.anchor = source.anchor;
  }
  return target;
}

function renderGlyphLayout(group, label, layout, glyphs) {
  layout.placements.forEach((placement, glyphIndex) => {
    if (!placement.visible) {
      return;
    }
    const glyph = glyphs[glyphIndex] || {};
    const text = makeSvgNode("text", {
      x: placement.originX,
      y: placement.baselineY,
      class: "mol-atom-label",
      "font-size": placement.fontSize,
      fill: normalizeDisplayColor(glyph.fill || label.fill),
      "font-family": displayLabelFontFamily(glyph.fontFamily || label.fontFamily),
      "font-weight": glyph.fontWeight,
      "font-style": glyph.fontStyle,
      "dominant-baseline": "alphabetic",
    });
    text.textContent = glyph.char || placement.char;
    group.appendChild(text);
  });
}

function layoutAndRenderGlyphs(group, node, label, mode, glyphs, fontSize, anchorPoint, align, anchorGlyphIndex, lineIndex = null) {
  const layout = state.glyphKernel.layoutAtAnchor({
    glyphs,
    fontSize,
    anchorPoint,
    anchorGlyphIndex,
    align,
  });
  renderGlyphLayout(group, label, layout, glyphs);
  return geometryEntryFromLayout(node, mode, layout, glyphs, lineIndex);
}

function renderFragmentText(group, node, nodeMap, bonds, originX, originY) {
  const label = node?.label;
  if (!label?.text || !label.position) {
    return null;
  }
  const box = labelBox(label, originX, originY);
  const point = localPointFromAbsolute(label.position, originX, originY);
  const lines = getFragmentLabelLines(label);
  const lineCount = Math.max(1, lines.length);
  const fontSize = Math.max(
    9.5,
    Number(label.fontSize || label.size || 0),
    ...((label.runs || []).map((run) => Number(run.fontSize || run.size) || 0)),
  );
  const nodePoint = localPointFromAbsolute(node.position, originX, originY);
  const mode = getFragmentLabelMode(node, label);

  if (mode === "attached-group-above" && lineCount >= 2) {
    const anchorLineIndex = lineCount - 1;
    const anchorRuns = Array.isArray(label.lineRuns?.[anchorLineIndex]) ? label.lineRuns[anchorLineIndex] : null;
    const anchorGlyphs = glyphRunsForLabel(label, anchorRuns, lines[anchorLineIndex]);
    const stackedGlyphs = [...anchorGlyphs];
    for (let index = 0; index < anchorLineIndex; index += 1) {
      const lineRuns = Array.isArray(label.lineRuns?.[index]) ? label.lineRuns[index] : null;
      stackedGlyphs.push(...glyphRunsForLabel(label, lineRuns, lines[index]));
    }
    return layoutAndRenderGlyphs(
      group,
      node,
      label,
      mode,
      stackedGlyphs,
      fontSize,
      nodePoint,
      LABEL_ALIGN.above,
      firstVisibleGlyphIndex(anchorGlyphs),
    );
  }

  if (lineCount === 1 || mode === "hetero-h-above" || mode === "hetero-h-right") {
    const text = mode === "hetero-h-above" ? labelSourceText(label) : label.text;
    const glyphs = glyphRunsForLabel(label, null, text);
    const anchorGlyphIndex = anchorGlyphIndexForLabel(label, glyphs);
    const align = alignForLabel(label, mode);
    return layoutAndRenderGlyphs(
      group,
      node,
      label,
      mode,
      glyphs,
      fontSize,
      nodePoint,
      align,
      anchorGlyphIndex,
    );
  }

  const lineHeight = box ? (box.y2 - box.y1) / lineCount : fontSize * 1.05;
  const boxTop = box ? box.y1 : point.y - lineHeight * 0.8;
  let geometry = null;
  for (let index = 0; index < lineCount; index += 1) {
    const lineRuns = Array.isArray(label.lineRuns?.[index]) ? label.lineRuns[index] : null;
    const glyphs = glyphRunsForLabel(label, lineRuns, lines[index]);
    const anchorPoint = {
      x: point.x,
      y: boxTop + lineHeight * (index + 0.5),
    };
    const entry = layoutAndRenderGlyphs(
      group,
      node,
      label,
      mode,
      glyphs,
      fontSize,
      anchorPoint,
      alignForLabel(label, mode),
      anchorGlyphIndexForLabel(label, glyphs),
      index,
    );
    geometry = mergeGeometryEntry(geometry, entry);
  }
  return geometry;
}

function renderFragmentBond(group, bond, nodeMap, bonds, originX, originY, textGeometry = null) {
  const beginNode = nodeMap.get(bond.begin);
  const endNode = nodeMap.get(bond.end);
  if (!beginNode || !endNode) {
    return;
  }

  const beginMode = getFragmentLabelMode(beginNode, beginNode.label);
  const endMode = getFragmentLabelMode(endNode, endNode.label);
  const beginShape = textGeometry?.get(beginNode.id) || null;
  const endShape = textGeometry?.get(endNode.id) || null;
  const beginPoint = connectionPointForNode(beginNode, beginMode, beginShape, originX, originY);
  const endPoint = connectionPointForNode(endNode, endMode, endShape, originX, originY);
  const stereoStyle = bond.stereo?.kind || bond.stereoStyle || null;
  const stereoEnd = bond.stereo?.wideEnd || bond.stereoEnd || null;
  const legacyDisplay = bond.display || bond.meta?.import?.cdxml?.display || "";
  const isStereoBond = Boolean(stereoStyle) || legacyDisplay === "WedgeBegin" || legacyDisplay === "WedgedHashBegin" || legacyDisplay === "WedgeEnd" || legacyDisplay === "WedgedHashEnd";
  const stereoTargetsBegin = (stereoStyle === "solid-wedge" && stereoEnd === "begin")
    || (stereoStyle === "hashed-wedge" && stereoEnd === "begin")
    || legacyDisplay === "WedgeEnd"
    || legacyDisplay === "WedgedHashEnd";
  const stereoTargetsEnd = (stereoStyle === "solid-wedge" && stereoEnd === "end")
    || (stereoStyle === "hashed-wedge" && stereoEnd === "end")
    || legacyDisplay === "WedgeBegin"
    || legacyDisplay === "WedgedHashBegin";
  const shapeGap = isStereoBond
    ? 0.06
    : (beginMode === "attached-group" || beginMode === "attached-group-above" || beginMode === "attached-group-center" ||
       endMode === "attached-group" || endMode === "attached-group-above" || endMode === "attached-group-center")
      ? BOND_STROKE * 0.5 + 0.12
      : BOND_STROKE * 0.5 + 0.045;
  const beginCollisionBoxes = collisionBoxesForNode(beginNode, beginMode, beginShape, isStereoBond, originX, originY);
  const endCollisionBoxes = collisionBoxesForNode(endNode, endMode, endShape, isStereoBond, originX, originY);
  const beginBox = beginCollisionBoxes?.length
    ? null
    : (beginMode === "default" || beginMode === "centered-atom")
      ? labelBox(beginNode.label, originX, originY)
      : beginMode === "attached-group"
        ? (beginShape ? null : attachedGroupCollisionBox(beginNode, originX, originY))
        : beginMode === "attached-group-above"
          ? labelBox(beginNode.label, originX, originY)
        : beginMode === "attached-group-center"
          ? (beginShape ? null : centeredAttachedGroupCollisionBox(beginNode, originX, originY))
        : centeredLabelBox(beginPoint, 3.2, 3.2);
  const endBox = endCollisionBoxes?.length
    ? null
    : (endMode === "default" || endMode === "centered-atom")
      ? labelBox(endNode.label, originX, originY)
      : endMode === "attached-group"
        ? (endShape ? null : attachedGroupCollisionBox(endNode, originX, originY))
        : endMode === "attached-group-above"
          ? labelBox(endNode.label, originX, originY)
        : endMode === "attached-group-center"
          ? (endShape ? null : centeredAttachedGroupCollisionBox(endNode, originX, originY))
        : centeredLabelBox(endPoint, 3.2, 3.2);
  const beginHasLabel = Boolean(beginCollisionBoxes?.length || beginBox);
  const endHasLabel = Boolean(endCollisionBoxes?.length || endBox);
  const beginStereoTargetBoxes = stereoTargetBoxesForNode(beginNode, beginMode, beginShape);
  const endStereoTargetBoxes = stereoTargetBoxesForNode(endNode, endMode, endShape);
  const labelShapes = allLabelShapes(textGeometry);
  const beginWedgeRetreatShapes = uniqueShapes(beginStereoTargetBoxes || beginCollisionBoxes || [], labelShapes);
  const endWedgeRetreatShapes = uniqueShapes(endStereoTargetBoxes || endCollisionBoxes || [], labelShapes);
  const beginWideContactDirections = solidWedgeWideContactDirections(bond, bond.begin, bonds, nodeMap, originX, originY);
  const endWideContactDirections = solidWedgeWideContactDirections(bond, bond.end, bonds, nodeMap, originX, originY);
  let start = beginPoint;
  let end = endPoint;
  if (beginCollisionBoxes?.length && (!isStereoBond || !stereoTargetsBegin)) {
    start = retreatPointFromShapes(beginPoint, endPoint, beginCollisionBoxes, shapeGap);
  }
  if (endCollisionBoxes?.length && (!isStereoBond || !stereoTargetsEnd)) {
    end = retreatPointFromShapes(endPoint, beginPoint, endCollisionBoxes, shapeGap);
  }
  ({ start, end } = lineEndpointsForFragmentBond(start, end, beginBox, endBox));
  const display = legacyDisplay;
  const color = CHEMDRAW_INK;
  const strokeWidth = Number(bond.strokeWidth || BOND_STROKE);

  if (stereoStyle === "solid-wedge" && stereoEnd === "end") {
    renderSolidWedge(group, start, end, color, {
      targetMode: endMode,
      targetHasLabel: endHasLabel,
      targetBoxes: endWedgeRetreatShapes,
      wideContactDirections: endWideContactDirections,
    });
    return;
  }
  if (stereoStyle === "hashed-wedge" && stereoEnd === "end") {
    renderHashedWedge(group, start, end, color, {
      targetHasLabel: endHasLabel,
      targetBoxes: endWedgeRetreatShapes,
    });
    return;
  }
  if (stereoStyle === "solid-wedge" && stereoEnd === "begin") {
    renderSolidWedge(group, end, start, color, {
      targetMode: beginMode,
      targetHasLabel: beginHasLabel,
      targetBoxes: beginWedgeRetreatShapes,
      wideContactDirections: beginWideContactDirections,
    });
    return;
  }
  if (stereoStyle === "hashed-wedge" && stereoEnd === "begin") {
    renderHashedWedge(group, end, start, color, {
      targetHasLabel: beginHasLabel,
      targetBoxes: beginWedgeRetreatShapes,
    });
    return;
  }

  if (display === "WedgeBegin") {
    renderSolidWedge(group, start, end, color, {
      targetMode: endMode,
      targetHasLabel: endHasLabel,
      targetBoxes: endWedgeRetreatShapes,
      wideContactDirections: endWideContactDirections,
    });
    return;
  }
  if (display === "WedgedHashBegin") {
    renderHashedWedge(group, start, end, color, {
      targetHasLabel: endHasLabel,
      targetBoxes: endWedgeRetreatShapes,
    });
    return;
  }
  if (display === "WedgeEnd") {
    renderSolidWedge(group, end, start, color, {
      targetMode: beginMode,
      targetHasLabel: beginHasLabel,
      targetBoxes: beginWedgeRetreatShapes,
      wideContactDirections: beginWideContactDirections,
    });
    return;
  }
  if (display === "WedgedHashEnd") {
    renderHashedWedge(group, end, start, color, {
      targetHasLabel: beginHasLabel,
      targetBoxes: beginWedgeRetreatShapes,
    });
    return;
  }

  if (bond.order === 2) {
    const doublePlacement = bond.double?.placement || bond.doubleStyle || bond.doublePosition || null;
    const doubleScale = strokeWidth / BOND_STROKE;
    const doubleOffset = DOUBLE_BOND_OFFSET * doubleScale;
    renderDoubleBond(group, start, end, bond.double?.placement || bond.doubleStyle || bond.doublePosition || null, color, {
      terminalStart: fragmentNodeDegree(bonds, bond.begin) === 1,
      terminalEnd: fragmentNodeDegree(bonds, bond.end) === 1,
      alignStart: beginHasLabel,
      alignEnd: endHasLabel,
      startBoxes: beginCollisionBoxes,
      endBoxes: endCollisionBoxes,
      shapeGap,
      strokeWidth,
      shortStartPoint: doublePlacement ? sideDoubleJoinPointForFragmentEndpoint(bond, bond.begin, bonds, nodeMap, originX, originY, doubleOffset) : null,
      shortEndPoint: doublePlacement ? sideDoubleJoinPointForFragmentEndpoint(bond, bond.end, bonds, nodeMap, originX, originY, doubleOffset) : null,
    });
    return;
  }
  if (bond.order >= 3) {
    renderBondLines(group, start, end, [-TRIPLE_BOND_OFFSET, 0, TRIPLE_BOND_OFFSET], color, {
      startBoxes: beginCollisionBoxes,
      endBoxes: endCollisionBoxes,
      shapeGap,
      strokeWidth,
    });
    return;
  }
  renderBondLines(group, start, end, [0], color, {
    startBoxes: beginCollisionBoxes,
    endBoxes: endCollisionBoxes,
    shapeGap,
    strokeWidth,
  });
}

function renderCdxmlFragmentObject(svgRoot, object, resource) {
  const fragment = resource?.data;
  if (!fragment?.nodes?.length) {
    return;
  }

  const [tx, ty] = object.transform.translate;
  const group = makeSvgNode("g", {
    transform: `translate(${tx} ${ty})`,
    "data-object-id": object.id,
  });
  const bondLayer = makeSvgNode("g", { "data-layer": "bonds" });
  const knockoutLayer = LABEL_DEBUG_MODE ? makeSvgNode("g", { "data-layer": "label-knockout" }) : null;
  const textLayer = makeSvgNode("g", { "data-layer": "labels" });
  group.appendChild(bondLayer);
  if (knockoutLayer) {
    group.appendChild(knockoutLayer);
  }
  group.appendChild(textLayer);
  svgRoot.appendChild(group);

  const originX = Number(fragment.bbox?.[0] || 0);
  const originY = Number(fragment.bbox?.[1] || 0);
  const nodeMap = new Map(fragment.nodes.map((node) => [node.id, node]));

  const textGeometry = new Map();
  for (const node of fragment.nodes) {
    const geometry = renderFragmentText(textLayer, node, nodeMap, fragment.bonds || [], originX, originY);
    if (geometry) {
      textGeometry.set(node.id, geometry);
    }
  }
  if (knockoutLayer) {
    renderFragmentLabelKnockouts(knockoutLayer, textGeometry);
  }
  if (!LABEL_DEBUG_MODE) {
    for (const bond of fragment.bonds || []) {
      renderFragmentBond(bondLayer, bond, nodeMap, fragment.bonds || [], originX, originY, textGeometry);
    }
  }
}

function formatAtomLabel(atom) {
  let label = atom.symbol;
  if (atom.charge) {
    if (atom.charge === 1) {
      label += "+";
    } else if (atom.charge > 1) {
      label += `${atom.charge}+`;
    } else if (atom.charge === -1) {
      label += "−";
    } else {
      label += `${Math.abs(atom.charge)}−`;
    }
  }
  return label;
}

function getLabelMetrics(parsedMol, atomIndex) {
  const atom = parsedMol.atoms[atomIndex];
  if (!atomNeedsLabel(parsedMol, atomIndex)) {
    return { visible: false, pad: 0 };
  }
  const label = formatAtomLabel(atom);
  const width = Math.max(12, label.length * 6.2 + 8);
  const height = 15;
  return { visible: true, pad: width / 2 - 2, label, width, height };
}

function getAtomLabelOffset(parsedMol, atomIndex) {
  const atom = parsedMol.atoms[atomIndex];
  const neighbors = bondNeighbors(parsedMol, atomIndex, -1);
  if (!neighbors.length) {
    return { x: 0, y: 0 };
  }
  let vx = 0;
  let vy = 0;
  for (const neighbor of neighbors) {
    vx += neighbor.x - atom.x;
    vy += neighbor.y - atom.y;
  }
  const length = Math.hypot(vx, vy) || 1;
  return { x: (-vx / length) * 4.5, y: (vy / length) * 4.5 };
}

function buildCollapsedGroups(parsedMol, atomPoints) {
  const collapsed = [];
  const hiddenAtoms = new Set();
  const hiddenBonds = new Set();

  for (const sgroup of parsedMol.sgroups || []) {
    if (sgroup.type !== "SUP" || !sgroup.label || !sgroup.atoms.length) {
      continue;
    }
    const groupAtoms = new Set(sgroup.atoms);
    const connections = [];
    for (let bondIndex = 0; bondIndex < parsedMol.bonds.length; bondIndex += 1) {
      const bond = parsedMol.bonds[bondIndex];
      const beginInside = groupAtoms.has(bond.begin);
      const endInside = groupAtoms.has(bond.end);
      if (beginInside && endInside) {
        hiddenBonds.add(bondIndex);
      } else if (beginInside || endInside) {
        const outsideAtom = beginInside ? bond.end : bond.begin;
        connections.push({ bondIndex, outsideAtom });
        hiddenBonds.add(bondIndex);
      }
    }
    for (const atomIndex of groupAtoms) {
      hiddenAtoms.add(atomIndex);
    }

    const points = sgroup.atoms.map((index) => atomPoints[index]).filter(Boolean);
    if (!points.length) {
      continue;
    }

    let anchorPoint = null;
    let direction = null;

    for (const bondIndex of sgroup.bonds || []) {
      const bond = parsedMol.bonds[bondIndex];
      if (!bond) {
        continue;
      }
      const insideAtomIndex = groupAtoms.has(bond.begin) ? bond.begin : groupAtoms.has(bond.end) ? bond.end : null;
      if (insideAtomIndex == null) {
        continue;
      }
      anchorPoint = atomPoints[insideAtomIndex];
      const vector = sgroup.vectors?.get(bondIndex);
      if (vector) {
        direction = { x: vector.x, y: -vector.y };
      }
      break;
    }

    if (!anchorPoint) {
      anchorPoint = {
        x: points.reduce((sum, point) => sum + point.x, 0) / points.length,
        y: points.reduce((sum, point) => sum + point.y, 0) / points.length,
      };
    }
    if (!direction) {
      if (connections.length) {
        const outsidePoint = atomPoints[connections[0].outsideAtom];
        direction = {
          x: anchorPoint.x - outsidePoint.x,
          y: anchorPoint.y - outsidePoint.y,
        };
      } else {
        direction = { x: 0, y: -1 };
      }
    }
    const dlen = Math.hypot(direction.x, direction.y) || 1;
    const unit = { x: direction.x / dlen, y: direction.y / dlen };
    const labelWidth = Math.max(20, sgroup.label.length * 7.1 + 8);
    const centroid = {
      x: anchorPoint.x + unit.x * (labelWidth * 0.45 + 7),
      y: anchorPoint.y + unit.y * 10,
    };

    collapsed.push({
      label: sgroup.label,
      centroid,
      labelWidth,
      direction: unit,
      connections,
    });
  }

  return { collapsed, hiddenAtoms, hiddenBonds };
}

function renderMoleculeObject(svgRoot, object, resources) {
  const resource = resources[object.payload.resourceRef];
  if (!resource || !resource.data) {
    return;
  }
  if (resource.type === "molecule_fragment2d") {
    renderCdxmlFragmentObject(svgRoot, object, resource);
    return;
  }

  const parsedMol = parseMolblock(resource.data);
  if (!parsedMol) {
    return;
  }

  const [tx, ty] = object.transform.translate;
  const [, , bboxWidth, bboxHeight] = object.payload.bbox;
  const group = makeSvgNode("g", {
    transform: `translate(${tx} ${ty})`,
    "data-object-id": object.id,
  });
  const atomPoints = parsedMol.atoms.map((atom) =>
    mapAtomPoint(parsedMol, atom, {
      width: bboxWidth,
      height: bboxHeight,
    }),
  );
  const { collapsed, hiddenAtoms, hiddenBonds } = buildCollapsedGroups(parsedMol, atomPoints);
  const labelMetrics = new Map();
  parsedMol.atoms.forEach((_, index) => {
    if (!hiddenAtoms.has(index)) {
      labelMetrics.set(index, getLabelMetrics(parsedMol, index));
    }
  });

  if (!LABEL_DEBUG_MODE) {
    parsedMol.bonds.forEach((bond, bondIndex) => {
      if (hiddenBonds.has(bondIndex)) {
        return;
      }
      renderBond(group, parsedMol, bond, atomPoints, hiddenAtoms, labelMetrics);
    });
  }

  for (const collapsedGroup of collapsed) {
    if (!LABEL_DEBUG_MODE) {
      for (const connection of collapsedGroup.connections) {
        const from = atomPoints[connection.outsideAtom];
        const to = collapsedGroup.centroid;
        renderBondLines(group, from, to, [0], CHEMDRAW_INK);
      }
    }
    const text = makeSvgNode("text", {
      x: collapsedGroup.centroid.x,
      y: collapsedGroup.centroid.y,
      class: "mol-atom-label mol-group-label",
      "font-size": LABEL_FONT_SIZE,
      fill: CHEMDRAW_INK,
      "font-family": displayLabelFontFamily("Arial"),
    });
    text.textContent = collapsedGroup.label;
    group.appendChild(text);
  }

  parsedMol.atoms.forEach((atom, index) => {
    if (hiddenAtoms.has(index)) {
      return;
    }
    const metrics = labelMetrics.get(index);
    if (!metrics?.visible) {
      return;
    }
    const point = atomPoints[index];
    const offset = getAtomLabelOffset(parsedMol, index);
    const text = makeSvgNode("text", {
      x: point.x + offset.x,
      y: point.y + offset.y,
      class: "mol-atom-label",
      "font-size": LABEL_FONT_SIZE,
      fill: CHEMDRAW_INK,
      "font-family": displayLabelFontFamily("Arial"),
    });
    text.textContent = metrics.label;
    group.appendChild(text);
  });

  svgRoot.appendChild(group);
}

function renderLineObject(svgRoot, object, styles) {
  const points = object.payload.points || [];
  if (points.length < 2) {
    return;
  }

  const style = styles?.[object.styleRef] || {};
  const stroke = style.stroke || "#222222";
  const strokeWidth = style.strokeWidth || 1.6;
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
      const shaftEnd = arrowShaftEnd(from, to, arrowHead);
      path.setAttribute("d", points
        .slice(0, -2)
        .map((point, index) => `${index === 0 ? "M" : "L"} ${point[0]} ${point[1]}`)
        .concat(`M ${from[0]} ${from[1]} L ${shaftEnd[0]} ${shaftEnd[1]}`)
        .join(" "));
    }
    svgRoot.appendChild(path);
    renderArrowHead(svgRoot, from, to, arrowHead, stroke);
    return;
  }

  svgRoot.appendChild(path);
}

function arrowShaftEnd(from, to, arrowHead) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const headLength = Math.max(5.4, (arrowHead?.length || 8) * 0.6);
  const notchLength = Math.max(3.2, headLength * 0.66);
  const centerLength = Math.max(0, Math.min(notchLength, length * 0.8));
  return [to[0] - ux * centerLength, to[1] - uy * centerLength];
}

function renderArrowHead(svgRoot, from, to, arrowHead, stroke) {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const length = Math.hypot(dx, dy) || 1;
  const ux = dx / length;
  const uy = dy / length;
  const nx = -uy;
  const ny = ux;
  const sourceLength = arrowHead?.length || 8;
  const sourceWidth = arrowHead?.width || sourceLength * 0.55;
  const headLength = Math.max(5.4, sourceLength * 0.6);
  const headWidth = Math.max(4.8, sourceWidth * 1.16);
  const notchLength = Math.max(3.2, Math.min(headLength * 0.66, headLength - 0.8));

  const p1 = [to[0], to[1]];
  const p2 = [to[0] - ux * headLength + nx * (headWidth / 2), to[1] - uy * headLength + ny * (headWidth / 2)];
  const p3 = [to[0] - ux * headLength - nx * (headWidth / 2), to[1] - uy * headLength - ny * (headWidth / 2)];
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

function renderTextObject(svgRoot, object) {
  const [tx, ty] = object.transform.translate;
  const fontSize = Number(object.payload.fontSize || DEFAULT_TEXT_FONT_SIZE);
  const lines = object.payload.preserveLines
    ? String(object.payload.text || "")
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
    : wrapTextLines(
        String(object.payload.text || ""),
        Number(object.payload.box?.[2] || 160),
        fontSize,
      );
  const align = object.payload.align || "left";
  const lineHeight = Number(object.payload.lineHeight || 15);
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
          "font-size": isSubOrSuper ? Math.max(7, runFontSize * 0.72) : runFontSize,
          "font-family": run.fontFamily || undefined,
          "font-weight": fontWeightForRun(run),
          "font-style": fontStyleForRun(run),
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

function renderShapeObject(svgRoot, object, styles) {
  const [tx, ty] = object.transform.translate;
  const style = styles?.[object.styleRef] || {};
  const [ , , width, height ] = object.payload.bbox || [0, 0, 0, 0];
  const gradient = style.fillGradient;
  const attrs = {
    x: tx,
    y: ty,
    width,
    height,
    fill: style.fill || "none",
    stroke: style.stroke || "none",
    "stroke-width": style.strokeWidth || 1,
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

function wrapTextLines(text, maxWidth, fontSize) {
  const rawLines = String(text || "").split("\n");
  const maxChars = Math.max(8, Math.floor(maxWidth / Math.max(6, fontSize * 0.6)));
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

function buildRenderList(documentData) {
  return [...documentData.objects].sort((a, b) => {
    if (a.zIndex !== b.zIndex) {
      return a.zIndex - b.zIndex;
    }
    return a.id.localeCompare(b.id);
  });
}

function renderDocument() {
  const documentData = state.currentDocument;
  if (!documentData) {
    return;
  }

  const page = documentData.document.page;
  viewerSvg.innerHTML = "";
  viewerSvg.setAttribute("viewBox", `0 0 ${page.width} ${page.height}`);
  const pageBackground = normalizeDisplayColor(page.background, CHEMDRAW_PAGE_BACKGROUND);
  viewerSvg.style.setProperty("--chemcore-page-bg", pageBackground);
  viewerSvg.appendChild(makeSvgNode("rect", {
    x: 0,
    y: 0,
    width: page.width,
    height: page.height,
    fill: pageBackground,
    "data-layer": "page-background",
  }));

  const visibleObjects = buildRenderList(documentData);

  for (const object of visibleObjects) {
    if (!object.visible) {
      continue;
    }
    if (object.type === "molecule" && toggleMolecules && !toggleMolecules.checked) {
      continue;
    }
    if (object.type === "line" && toggleLines && !toggleLines.checked) {
      continue;
    }
    if (object.type === "text" && toggleTexts && !toggleTexts.checked) {
      continue;
    }
    if (LABEL_DEBUG_MODE && object.type !== "molecule") {
      continue;
    }

    if (object.type === "molecule") {
      renderMoleculeObject(viewerSvg, object, documentData.resources);
    } else if (object.type === "shape") {
      renderShapeObject(viewerSvg, object, documentData.styles);
    } else if (object.type === "line") {
      renderLineObject(viewerSvg, object, state.currentDocument.styles);
    } else if (object.type === "text") {
      renderTextObject(viewerSvg, object);
    }
  }

  const counts = {};
  for (const object of documentData.objects) {
    counts[object.type] = (counts[object.type] || 0) + 1;
  }
  viewerStats.textContent = Object.entries(counts)
    .map(([type, count]) => `${type}: ${count}`)
    .join(" | ");
  renderEditorOverlay();
}

function fitView() {
  const page = state.currentDocument?.document?.page;
  if (!page) {
    return;
  }
  viewerSvg.setAttribute("viewBox", `0 0 ${page.width} ${page.height}`);
}

async function loadDocument(path) {
  const response = await fetch(path, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`Failed to load ${path}: ${response.status}`);
  }
  return response.json();
}

async function loadAndRender() {
  viewerTitle.textContent = "Loading...";
  try {
    if (state.currentPath) {
      const documentData = await loadDocument(state.currentPath);
      state.currentDocument = documentData;
    } else {
      if (!state.editorEngine) {
        resetEditorEngine();
      } else {
        state.editorEngine.clearInteraction();
        syncEngineToolState();
        syncDocumentFromEngine();
      }
    }
    const documentData = state.currentDocument;
    state.currentDocument = documentData;
    viewerTitle.textContent = documentData.document.title || state.currentPath;
    docMeta.textContent = JSON.stringify(
      {
        sample: state.currentPath || "blank",
        page: documentData.document.page,
        meta: documentData.document.meta,
      },
      null,
      2,
    );
    renderDocument();
    fitView();
  } catch (error) {
    viewerTitle.textContent = "Load failed";
    viewerStats.textContent = "";
    docMeta.textContent = String(error);
    viewerSvg.innerHTML = "";
  }
}

try {
  const [glyphKernel] = await Promise.all([
    initializeGlyphKernel(),
    initializeChemcoreEngine(),
  ]);
  state.glyphKernel = glyphKernel;
  await loadAndRender();
} catch (error) {
  viewerTitle.textContent = "Runtime load failed";
  viewerStats.textContent = "";
  docMeta.textContent = String(error);
  viewerSvg.innerHTML = "";
}
