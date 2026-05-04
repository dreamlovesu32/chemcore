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
  if (editorState.activeTool === "shape") {
    return shapeToolbarHtml(editorState);
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
}

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

const BOND_TOOL_ICON_SPECS = {
  single: {
    title: "Single bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 19 7"/></svg>`,
  },
  double: {
    title: "Double bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 15 18 6"/><path d="M6 18 19 9"/></svg>`,
  },
  triple: {
    title: "Triple bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4.5 14 17.5 5"/><path d="M6 17 19 8"/><path d="M7.5 20 20.5 11"/></svg>`,
  },
  dashed: {
    title: "Dashed bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 17 7 15.5"/><path d="M9.5 13.8 11.5 12.4"/><path d="M14 10.6 16 9.2"/><path d="M18.5 7.5 19 7"/></svg>`,
  },
  "dashed-double": {
    title: "Dashed-solid double bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4.3 16 18.3 6" style="stroke-linecap:butt"/><path d="M5.7 18 19.7 8" style="stroke-dasharray:2.2 1.6;stroke-linecap:butt"/></svg>`,
  },
  bold: {
    title: "Bold bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon class="filled" points="4.1,15.7 18.1,5.7 19.9,8.3 5.9,18.3" style="stroke-linejoin:miter"/></svg>`,
  },
  "bold-dashed": {
    title: "Hash bond",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5.8 15.4 8.2 18.8" style="stroke-width:1.9"/><path d="M9.6 12.7 12 16.1" style="stroke-width:1.9"/><path d="M13.4 10 15.8 13.4" style="stroke-width:1.9"/><path d="M17.2 7.3 19.6 10.7" style="stroke-width:1.9"/></svg>`,
  },
  wedge: {
    title: "Solid wedge",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon class="filled" points="3.2,14.5 6.8,19.5 19,7" style="stroke-linejoin:miter"/></svg>`,
  },
  "hashed-wedge": {
    title: "Hash wedge",
    svg: `<svg viewBox="0 0 24 24" aria-hidden="true"><polygon class="filled" points="3.5,14.9 3.8,15.3 5.7,13.3 4.5,13.9" style="stroke:none"/><polygon class="filled" points="4.1,15.7 4.4,16.2 8.6,11.9 7,12.7" style="stroke:none"/><polygon class="filled" points="4.7,16.6 5.1,17.2 11.7,10.4 9.8,11.3" style="stroke:none"/><polygon class="filled" points="5.5,17.7 6,18.4 15.5,8.6 13.3,9.7" style="stroke:none"/></svg>`,
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
    toolbarButton("distribute-v", "Vertical distribute", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="7" y="4" width="10" height="3"/><rect x="7" y="10.5" width="10" height="3"/><rect x="7" y="17" width="10" height="3"/><path d="M5 7v3.5"/><path d="M5 13.5V17"/><path d="M19 7v3.5"/><path d="M19 13.5V17"/></svg>`),
    toolbarButton("distribute-h", "Horizontal distribute", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="7" width="3" height="10"/><rect x="10.5" y="7" width="3" height="10"/><rect x="17" y="7" width="3" height="10"/><path d="M7 5h3.5"/><path d="M13.5 5H17"/><path d="M7 19h3.5"/><path d="M13.5 19H17"/></svg>`),
    secondaryDivider(),
    toolbarButton("flip-h", "Flip horizontal", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16"/><path class="filled" d="M5 7v10l5-5z"/><path d="M19 7v10l-5-5z"/></svg>`),
    toolbarButton("flip-v", "Flip vertical", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h16"/><path class="filled" d="M7 5h10l-5 5z"/><path d="M7 19h10l-5-5z"/></svg>`),
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
    const transform = type === "curved-mirror" ? ` transform="translate(0 24) scale(1 -1)"` : "";
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><g${transform}><path d="M18.8 7.2C12.8 4.8 6 8.9 5.9 15.4"/><path class="filled" d="M20.5 9.6 17.2 6l4.9-.7z"/></g></svg>`;
  }
  if (type === "hollow") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 14h10v3l6-5-6-5v3H4z"/></svg>`;
  }
  if (type === "open") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9h12"/><path d="M4 15h12"/><path d="m15 6 5 6-5 6"/></svg>`;
  }
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h12"/><path class="filled" d="M15 7 21 12l-6 5z"/></svg>`;
}

function isCurvedArrowType(type) {
  return type === "curved" || type === "curved-mirror";
}

function arrowCurveSvg(curve, mirrored = false) {
  const paths = {
    "270": "M18.8 6.2C11.9 3.6 4.5 8.3 4.5 15.4c0 4 3.4 6.4 7.3 5.3",
    "180": "M18.8 7.1C13.1 4.3 6.2 8.6 6.2 14.6c0 3.4 2.9 5.3 6.1 4.5",
    "120": "M18.8 8.4C14.5 5.9 8.4 8.2 7.2 13.2",
    "90": "M18.8 9.6C15.2 7.5 10.8 8.9 8.4 12.1",
  };
  const transform = mirrored ? ` transform="translate(0 24) scale(1 -1)"` : "";
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><g${transform}><path d="${paths[curve] || paths["270"]}"/><path class="filled" d="M20.4 8.8 17.1 5.8l4.7-1z"/></g></svg>`;
}

function arrowSizeSvg(size) {
  const scale = size === "large" ? 1 : size === "small" ? 0.62 : 0.78;
  const tip = 20;
  const base = tip - 7 * scale;
  const half = 4.8 * scale;
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h${Math.max(8, base - 4)}"/><path class="filled" d="M${base} ${12 - half} ${tip} 12 ${base} ${12 + half}z"/></svg>`;
}

function arrowEndpointSvg(label, side) {
  const head = side === "head"
    ? `<path class="filled" d="M15 7 21 12l-6 5z"/>`
    : `<path class="filled" d="M9 7 3 12l6 5z"/>`;
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14"/>${head}<text x="12" y="22" text-anchor="middle" fill="currentColor" font-size="5.5" font-family="Arial, Helvetica, sans-serif">${label}</text></svg>`;
}

function arrowHalfEndpointSvg(side, half) {
  const isHead = side === "head";
  const tipX = isHead ? 21 : 3;
  const baseX = isHead ? 15 : 9;
  const shaftStart = isHead ? 5 : 9;
  const shaftEnd = isHead ? 15 : 19;
  const topLabel = half === "left" ? "left" : "right";
  const bottomLabel = isHead ? "head" : "tail";
  const head = half === "left"
    ? `<path class="filled" d="M${tipX} 12 ${baseX} 12 ${baseX} 7z"/>`
    : `<path class="filled" d="M${tipX} 12 ${baseX} 17 ${baseX} 12z"/>`;
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="12" y="5" text-anchor="middle" fill="currentColor" font-size="4.8" font-family="Arial, Helvetica, sans-serif">${topLabel}</text><path d="M${shaftStart} 12h${shaftEnd - shaftStart}"/>${head}<text x="12" y="22" text-anchor="middle" fill="currentColor" font-size="4.8" font-family="Arial, Helvetica, sans-serif">${bottomLabel}</text></svg>`;
}

function arrowNoGoSvg(kind) {
  const mark = kind === "hash"
    ? `<path class="filled" d="M10 7.5 12 8.2 8 17.5 6 16.8z"/><path class="filled" d="M16 7.5 18 8.2 14 17.5 12 16.8z"/>`
    : `<path class="filled" d="M7.1 6.2 17.8 16.9 16.4 18.3 5.7 7.6z"/><path class="filled" d="M16.4 5.7 17.8 7.1 7.1 17.8 5.7 16.4z"/>`;
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h12"/><path class="filled" d="M15 7 21 12l-6 5z"/>${mark}</svg>`;
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
  const normalizedFontSize = normalizeToolbarFontSize(editorState.textFontSize);
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
    ${colorButton("text-black", "Text color", "#000000", editorState.textColor === "#000000")}
    ${secondaryDivider()}
    ${toolbarButton("text-align-left", "Align left", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h9"/><path d="M5 14h12"/><path d="M5 18h8"/></svg>`, editorState.textAlign === "left")}
    ${toolbarButton("text-align-center", "Align center", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M7 10h10"/><path d="M6 14h12"/><path d="M8 18h8"/></svg>`, editorState.textAlign === "center")}
    ${toolbarButton("text-align-right", "Align right", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M10 10h9"/><path d="M7 14h12"/><path d="M11 18h8"/></svg>`, editorState.textAlign === "right")}
    ${toolbarButton("text-align-justify", "Justify", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6h14"/><path d="M5 10h14"/><path d="M5 14h14"/><path d="M5 18h14"/></svg>`, editorState.textAlign === "justify")}
    ${secondaryDivider()}
    ${toolbarButton("text-bold", "Bold", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5h5.4a3.1 3.1 0 0 1 0 6.2H8z"/><path d="M8 11.2h6.2a3.4 3.4 0 0 1 0 6.8H8z"/></svg>`, editorState.textBold)}
    ${toolbarButton("text-italic", "Italic", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M14 5h-4"/><path d="M14 19h-4"/><path d="M13 5 11 19"/></svg>`, editorState.textItalic)}
    ${toolbarButton("text-underline", "Underline", `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v7a4 4 0 0 0 8 0V5"/><path d="M6 19h12"/></svg>`, editorState.textUnderline)}
    ${secondaryDivider()}
    ${toolbarButton("text-chemical", "Chemical", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="3.6" y="15.4" fill="currentColor" font-size="10.8" font-family="Arial, Helvetica, sans-serif" font-weight="700">CH</text><text x="16.1" y="18.1" fill="currentColor" font-size="6.4" font-family="Arial, Helvetica, sans-serif" font-weight="700">2</text><text x="15.8" y="9.1" fill="currentColor" font-size="5.8" font-family="Arial, Helvetica, sans-serif" font-weight="700">+</text></svg>`, editorState.textScript === "chemical")}
    ${toolbarButton("text-subscript", "Subscript", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="4.2" y="14.8" fill="currentColor" font-size="12.2" font-family="Arial, Helvetica, sans-serif" font-style="italic" font-weight="700">X</text><text x="15.6" y="18.1" fill="currentColor" font-size="7" font-family="Arial, Helvetica, sans-serif" font-weight="700">2</text></svg>`, editorState.textScript === "subscript")}
    ${toolbarButton("text-superscript", "Superscript", `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="4.2" y="14.8" fill="currentColor" font-size="12.2" font-family="Arial, Helvetica, sans-serif" font-style="italic" font-weight="700">X</text><text x="15.4" y="9.1" fill="currentColor" font-size="7" font-family="Arial, Helvetica, sans-serif" font-weight="700">2</text></svg>`, editorState.textScript === "superscript")}
  `;
}

function shapeToolbarHtml(editorState) {
  return `
    ${toolbarButton("shape-kind-circle", "Circle", `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="6.8"/></svg>`, editorState.shapeKind === "circle")}
    ${toolbarButton("shape-kind-ellipse", "Ellipse", `<svg viewBox="0 0 24 24" aria-hidden="true"><ellipse cx="12" cy="12" rx="7" ry="4.2"/></svg>`, editorState.shapeKind === "ellipse")}
    ${toolbarButton("shape-kind-round-rect", "Rounded rectangle", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="6" width="14" height="12" rx="3"/></svg>`, editorState.shapeKind === "round-rect")}
    ${toolbarButton("shape-kind-rect", "Rectangle", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="6" width="14" height="12"/></svg>`, editorState.shapeKind === "rect")}
    ${secondaryDivider()}
    ${toolbarButton("shape-style-solid", "Solid outline", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="6" width="14" height="12"/></svg>`, editorState.shapeStyle === "solid")}
    ${toolbarButton("shape-style-dashed", "Dashed outline", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="6" width="14" height="12" stroke-dasharray="2 2"/></svg>`, editorState.shapeStyle === "dashed")}
    ${toolbarButton("shape-style-shaded", "Shaded", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect class="filled" x="5" y="6" width="14" height="12"/><rect class="soft-fill" x="6.5" y="7.2" width="9.2" height="7.8"/><rect x="5" y="6" width="14" height="12"/></svg>`, editorState.shapeStyle === "shaded")}
    ${toolbarButton("shape-style-filled", "Filled", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect class="filled" x="5" y="6" width="14" height="12"/></svg>`, editorState.shapeStyle === "filled")}
    ${toolbarButton("shape-style-shadowed", "Shadowed", `<svg viewBox="0 0 24 24" aria-hidden="true"><rect class="soft-fill" x="7" y="8" width="12" height="10"/><rect x="5" y="6" width="12" height="10"/></svg>`, editorState.shapeStyle === "shadowed")}
    ${secondaryDivider()}
    ${colorButton("shape-color-black", "Black", "#000000", editorState.shapeColor === "#000000")}
    ${colorButton("shape-color-red", "Red", "#ff0000", editorState.shapeColor === "#ff0000")}
    ${colorButton("shape-color-blue", "Blue", "#0000ff", editorState.shapeColor === "#0000ff")}
    ${colorButton("shape-color-green", "Green", "#008000", editorState.shapeColor === "#008000")}
  `;
}

function bracketIconSvg(kind = "round") {
  if (kind === "square") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5H6v14h3"/><path d="M15 5h3v14h-3"/></svg>`;
  }
  if (kind === "curly") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M10 5c-2 0-2 2-2 3.5V10c0 1.2-.9 2-2 2 1.1 0 2 .8 2 2v1.5C8 17 8 19 10 19"/><path d="M14 5c2 0 2 2 2 3.5V10c0 1.2.9 2 2 2-1.1 0-2 .8-2 2v1.5c0 1.5 0 3.5-2 3.5"/></svg>`;
  }
  if (kind === "circle-plus" || kind === "circle-minus") {
    const mark = kind === "circle-plus" ? `<path d="M12 8v8"/><path d="M8 12h8"/>` : `<path d="M8 12h8"/>`;
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="6.2"/>${mark}</svg>`;
  }
  if (kind === "plus") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 7v10"/><path d="M7 12h10"/></svg>`;
  }
  if (kind === "minus") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 12h10"/></svg>`;
  }
  if (kind === "radical-cation" || kind === "radical-anion") {
    const mark = kind === "radical-cation" ? `<path d="M15.5 8v8"/><path d="M11.5 12h8"/>` : `<path d="M11.5 12h8"/>`;
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><circle class="filled" cx="7.5" cy="12" r="1.8"/>${mark}</svg>`;
  }
  if (kind === "lone-pair") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><circle class="filled" cx="9" cy="12" r="1.8"/><circle class="filled" cx="15" cy="12" r="1.8"/></svg>`;
  }
  if (kind === "electron") {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><circle class="filled" cx="12" cy="12" r="2.2"/></svg>`;
  }
  return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M10 5c-3 3-3 11 0 14"/><path d="M14 5c3 3 3 11 0 14"/></svg>`;
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

function ringSvg(sides, aromatic = false) {
  if (aromatic) {
    return `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m12 4 7 4v8l-7 4-7-4V8z"/><circle cx="12" cy="12" r="4.6"/></svg>`;
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
