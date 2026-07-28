export function createDocumentLayoutHost(options) {
  const {
    root = document.body,
    state,
    paperButton,
    templateButton,
    engine,
    commandEngine,
    parseEngineJson,
    renderBoundsFromEngine,
    syncDocumentFromEngine,
    renderDocument,
    activateTemplateTool,
    setZoomPercent,
    transientNotificationHost,
  } = options;

  async function dialogSpec() {
    const value = await engine()?.documentLayoutDialogJson?.();
    return parseEngineJson(value, null);
  }

  async function executeLayout(layout) {
    const result = await commandEngine.executeEngineCommand({
      type: "set-document-layout",
      layout,
    });
    await syncDocumentFromEngine();
    renderDocument();
    setZoomPercent?.(layout.magnificationPercent, { exact: true });
    syncButtons();
    return result;
  }

  async function storeMagnificationPercent(value) {
    const spec = await dialogSpec();
    const numeric = Number(value);
    if (!spec?.data || !Number.isFinite(numeric) || near(spec.data.magnificationPercent, numeric)) {
      return false;
    }
    const layout = structuredClone(spec.data);
    layout.magnificationPercent = numeric;
    await commandEngine.executeEngineCommand({
      type: "set-document-layout",
      layout,
    });
    await syncDocumentFromEngine();
    return true;
  }

  async function initializeLayout() {
    await commandEngine.executeEngineCommand({
      type: "initialize-document-layout",
    });
    await syncDocumentFromEngine();
  }

  async function togglePaperView() {
    const next = !state.paperLayoutView;
    if (next) {
      await initializeLayout();
    }
    state.paperLayoutView = next;
    syncButtons();
    renderDocument();
  }

  async function openDialog() {
    try {
      const spec = await dialogSpec();
      if (!spec) {
        throw new Error("The layout engine did not provide a dialog.");
      }
      const next = await new DocumentLayoutDialog({ root, spec }).open();
      if (next) {
        await executeLayout(next);
      }
    } catch (error) {
      transientNotificationHost?.show?.(
        `Could not edit document layout: ${error instanceof Error ? error.message : String(error)}`,
        { error: true, duration: 4200 },
      );
    }
  }

  async function applyQuickChange(change) {
    const spec = await dialogSpec();
    if (!spec?.data) {
      return;
    }
    const layout = structuredClone(spec.data);
    if (change.paper) {
      layout.paper = {
        width: change.paper.width,
        height: change.paper.height,
      };
    }
    if (change.orientation === "portrait" && layout.paper.width > layout.paper.height) {
      [layout.paper.width, layout.paper.height] = [layout.paper.height, layout.paper.width];
    }
    if (change.orientation === "landscape" && layout.paper.width < layout.paper.height) {
      [layout.paper.width, layout.paper.height] = [layout.paper.height, layout.paper.width];
    }
    if (change.drawingSpace) {
      layout.drawingSpace = change.drawingSpace;
    }
    await executeLayout(layout);
  }

  function closeMenu() {
    document.querySelector(".document-layout-quick-menu")?.remove();
  }

  async function openQuickMenu(event) {
    event.preventDefault();
    event.stopPropagation();
    closeMenu();
    const spec = await dialogSpec();
    if (!spec) {
      return;
    }
    const menu = document.createElement("div");
    menu.className = "document-layout-quick-menu";
    menu.setAttribute("role", "menu");
    menu.style.left = `${Math.max(8, Math.min(event.clientX, window.innerWidth - 230))}px`;
    menu.style.bottom = `${Math.max(44, window.innerHeight - event.clientY)}px`;
    const add = (label, action, checked = false) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("role", "menuitem");
      button.innerHTML = `<span class="document-layout-menu-check">${checked ? "✓" : ""}</span><span>${escapeHtml(label)}</span>`;
      button.addEventListener("click", () => {
        closeMenu();
        Promise.resolve(action()).catch((error) => {
          transientNotificationHost?.show?.(
            `Could not change document layout: ${error instanceof Error ? error.message : String(error)}`,
            { error: true, duration: 4200 },
          );
        });
      });
      menu.appendChild(button);
    };
    const data = spec.data;
    for (const preset of spec.paperPresets || []) {
      const portraitWidth = Math.min(preset.width, preset.height);
      const portraitHeight = Math.max(preset.width, preset.height);
      const checked = near(data.paper.width, portraitWidth) && near(data.paper.height, portraitHeight);
      add(preset.label, () => applyQuickChange({
        paper: { width: portraitWidth, height: portraitHeight },
      }), checked);
    }
    menu.appendChild(divider());
    add("Portrait", () => applyQuickChange({ orientation: "portrait" }), data.paper.height >= data.paper.width);
    add("Landscape", () => applyQuickChange({ orientation: "landscape" }), data.paper.width > data.paper.height);
    menu.appendChild(divider());
    add("Pages", () => applyQuickChange({ drawingSpace: "pages" }), data.drawingSpace === "pages");
    add("Poster", () => applyQuickChange({ drawingSpace: "poster" }), data.drawingSpace === "poster");
    menu.appendChild(divider());
    add("Document Layout…", openDialog);
    root.appendChild(menu);
    const dismiss = (dismissEvent) => {
      if (!menu.contains(dismissEvent.target)) {
        closeMenu();
        document.removeEventListener("pointerdown", dismiss, true);
      }
    };
    queueMicrotask(() => document.addEventListener("pointerdown", dismiss, true));
  }

  function syncButtons() {
    if (paperButton) {
      paperButton.setAttribute("aria-pressed", state.paperLayoutView ? "true" : "false");
      paperButton.setAttribute(
        "aria-label",
        state.paperLayoutView ? "Switch to infinite canvas" : "Switch to paper layout",
      );
      paperButton.title = state.paperLayoutView
        ? "Infinite canvas (right-click for paper options)"
        : "Paper layout (right-click for paper options)";
    }
  }

  paperButton?.addEventListener("click", () => void togglePaperView());
  paperButton?.addEventListener("contextmenu", (event) => void openQuickMenu(event));
  templateButton?.addEventListener("click", () => {
    void activateTemplateTool?.();
  });
  templateButton?.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    void activateTemplateTool?.();
  });
  window.addEventListener("blur", closeMenu);
  syncButtons();

  function renderPaperBackground({ viewerSvg, viewBox, makeSvgNode, pageBackground }) {
    syncButtons();
    if (!state.paperLayoutView) {
      return false;
    }
    const documentData = state.currentDocument;
    const layout = documentData?.document?.layout;
    if (!layout?.paper) {
      return false;
    }
    const bounds = renderBoundsFromEngine(engine(), "document");
    const resolved = resolveLayout(layout, bounds);
    const layer = makeSvgNode("g", {
      "data-layer": "paper-layout",
      "pointer-events": "none",
    });
    const paperWidth = Number(layout.paper.width);
    const paperHeight = Number(layout.paper.height);
    const overlap = layout.drawingSpace === "poster" ? Number(layout.pageOverlap || 0) : 0;
    const stepX = paperWidth - overlap;
    const stepY = paperHeight - overlap;
    const margins = layout.margins || [36, 36, 36, 36];
    let pageNumber = 1;
    for (let row = 0; row < resolved.heightPages; row += 1) {
      for (let column = 0; column < resolved.widthPages; column += 1) {
        const x = resolved.origin[0] + column * stepX;
        const y = resolved.origin[1] + row * stepY;
        layer.appendChild(makeSvgNode("rect", {
          x,
          y,
          width: paperWidth,
          height: paperHeight,
          fill: pageBackground,
          stroke: "#aeb5bf",
          "stroke-width": 0.75,
          "data-page-number": pageNumber,
        }));
        layer.appendChild(makeSvgNode("rect", {
          x: x + Number(margins[3] || 0),
          y: y + Number(margins[0] || 0),
          width: Math.max(0, paperWidth - Number(margins[1] || 0) - Number(margins[3] || 0)),
          height: Math.max(0, paperHeight - Number(margins[0] || 0) - Number(margins[2] || 0)),
          fill: "none",
          stroke: "#d4d8de",
          "stroke-width": 0.45,
          "stroke-dasharray": "3 3",
        }));
        appendHeaderFooter(layer, makeSvgNode, {
          documentData,
          layout,
          x,
          y,
          width: paperWidth,
          height: paperHeight,
          pageNumber,
        });
        if (layout.printTrimMarks) {
          appendTrimMarks(layer, makeSvgNode, x, y, paperWidth, paperHeight);
        }
        pageNumber += 1;
      }
    }
    viewerSvg.appendChild(layer);
    return true;
  }

  return {
    openDialog,
    syncButtons,
    renderPaperBackground,
    resolveLayout: () => resolveLayout(
      state.currentDocument?.document?.layout,
      renderBoundsFromEngine(engine(), "document"),
    ),
    storeMagnificationPercent,
  };
}

class DocumentLayoutDialog {
  constructor({ root, spec }) {
    this.root = root;
    this.spec = spec;
    this.data = structuredClone(spec.data);
  }

  open() {
    document.querySelector(".document-layout-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog document-layout-dialog";
      this.backdrop.innerHTML = this.markup();
      this.root.appendChild(this.backdrop);
      this.bind();
    });
  }

  markup() {
    const data = this.data;
    const orientation = data.paper.width > data.paper.height ? "landscape" : "portrait";
    const preset = matchingPreset(this.spec.paperPresets, data.paper);
    return `
      <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
      <div class="numeric-dialog-backdrop" data-layout-close></div>
      <form class="numeric-dialog-panel document-layout-dialog-panel" aria-label="${escapeHtml(this.spec.title)}">
        <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.spec.title)}</div>
        <nav class="document-layout-tabs" aria-label="Document layout sections">
          ${tabButton("layout", "Layout", true)}
          ${tabButton("header-footer", "Header / Footer")}
          ${tabButton("view", "View")}
          ${tabButton("embedding", "Embedding")}
        </nav>
        <section class="document-layout-page is-active" data-layout-page="layout">
          <div class="document-layout-grid">
            ${selectField("paperPreset", "Paper", preset, [
              ...(this.spec.paperPresets || []).map((entry) => [entry.key, entry.label]),
              ["custom", "Custom"],
            ])}
            ${selectField("orientation", "Orientation", orientation, [["portrait", "Portrait"], ["landscape", "Landscape"]])}
            ${selectField("drawingSpace", "Drawing space", data.drawingSpace, [["pages", "Pages"], ["poster", "Poster"]])}
            ${checkboxField("autoPaginate", "Automatically add pages to cover all content", data.autoPaginate)}
            ${numberField("paperWidth", "Paper width", data.paper.width, "any", "pt", 1)}
            ${numberField("paperHeight", "Paper height", data.paper.height, "any", "pt", 1)}
            ${numberField("widthPages", "Minimum pages across", data.widthPages, 1, "", 1, 256)}
            ${numberField("heightPages", "Minimum pages down", data.heightPages, 1, "", 1, 256)}
            ${nullablePairFields("pageOrigin", "Original page origin", data.pageOrigin, null)}
            ${numberField("marginTop", "Top margin", data.margins[0], 0.1, "pt", 0)}
            ${numberField("marginRight", "Right margin", data.margins[1], 0.1, "pt", 0)}
            ${numberField("marginBottom", "Bottom margin", data.margins[2], 0.1, "pt", 0)}
            ${numberField("marginLeft", "Left margin", data.margins[3], 0.1, "pt", 0)}
            ${numberField("pageOverlap", "Poster overlap", data.pageOverlap, 0.1, "pt", 0)}
            ${checkboxField("printTrimMarks", "Print trim / registration marks", data.printTrimMarks)}
          </div>
          <p class="document-layout-resolved">Resolved: ${this.spec.resolved.widthPages} × ${this.spec.resolved.heightPages} pages, ${formatNumber(this.spec.resolved.totalWidth)} × ${formatNumber(this.spec.resolved.totalHeight)} pt</p>
        </section>
        <section class="document-layout-page" data-layout-page="header-footer">
          <div class="document-layout-grid">
            ${textField("header", "Header", data.header)}
            ${numberField("headerPosition", "Header position", data.headerPosition, 0.1, "pt", 0)}
            ${textField("footer", "Footer", data.footer)}
            ${numberField("footerPosition", "Footer position", data.footerPosition, 0.1, "pt", 0)}
          </div>
          <div class="document-layout-token-help">${(this.spec.headerFooterTokens || []).map((entry) => `<code>${escapeHtml(entry.token)}</code> ${escapeHtml(entry.label)}`).join(" · ")}</div>
        </section>
        <section class="document-layout-page" data-layout-page="view">
          <div class="document-layout-grid">
            ${numberField("magnificationPercent", "Saved magnification", data.magnificationPercent, 1, "%", 1, 999)}
            ${selectField("pageDefinition", "Page definition", data.pageDefinition || "undefined", [
              ["undefined", "Undefined"],
              ["center", "Center"],
              ["tl4", "TL4"],
              ["id-term", "ID term"],
              ["flush-left", "Flush left"],
              ["flush-right", "Flush right"],
              ["reaction1", "Reaction 1"],
              ["reaction2", "Reaction 2"],
              ["multicolumn-tl4", "Multicolumn TL4"],
              ["multicolumn-non-tl4", "Multicolumn non-TL4"],
              ["user-defined", "User defined"],
            ])}
            ${textField("legacySplitterPositionIds", "Legacy splitter object IDs", (data.legacySplitterPositionIds || []).join(" "))}
            ${textareaField("splitters", "Splitter objects", JSON.stringify(data.splitters || [], null, 2), 8)}
          </div>
          <p class="document-layout-hint">A splitter is an official logical page object with an optional point and page definition. Legacy SplitterPositions values are object IDs, not coordinates.</p>
        </section>
        <section class="document-layout-page" data-layout-page="embedding">
          <div class="document-layout-grid">
            ${nullablePairFields("fixInPlaceExtent", "In-place extent", data.fixInPlaceExtent)}
            ${nullablePairFields("fixInPlaceGap", "In-place gap", data.fixInPlaceGap)}
          </div>
          <p class="document-layout-hint">These values apply only while the document is edited as an embedded Office/OLE object.</p>
        </section>
        <div class="numeric-dialog-error" data-layout-error role="alert"></div>
        <div class="numeric-dialog-actions"><button type="button" data-layout-close>Cancel</button><button type="submit">OK</button></div>
      </form>`;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-layout-close]")) {
        this.close(null);
      }
      const tab = event.target.closest("[data-layout-tab]");
      if (tab) {
        const key = tab.dataset.layoutTab;
        this.backdrop.querySelectorAll("[data-layout-tab]").forEach((node) => node.classList.toggle("is-active", node === tab));
        this.backdrop.querySelectorAll("[data-layout-page]").forEach((node) => node.classList.toggle("is-active", node.dataset.layoutPage === key));
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") this.close(null);
    });
    const preset = this.backdrop.querySelector('[name="paperPreset"]');
    preset?.addEventListener("change", () => {
      const match = (this.spec.paperPresets || []).find((entry) => entry.key === preset.value);
      if (match) {
        this.backdrop.querySelector('[name="paperWidth"]').value = match.width;
        this.backdrop.querySelector('[name="paperHeight"]').value = match.height;
        this.applyOrientation();
      }
    });
    this.backdrop.querySelector('[name="orientation"]')?.addEventListener("change", () => this.applyOrientation());
    for (const name of ["paperWidth", "paperHeight"]) {
      this.backdrop.querySelector(`[name="${name}"]`)?.addEventListener("input", () => {
        preset.value = matchingPreset(this.spec.paperPresets, {
          width: numberOf(this.backdrop, "paperWidth"),
          height: numberOf(this.backdrop, "paperHeight"),
        });
      });
    }
    this.backdrop.querySelector("form").addEventListener("submit", (event) => {
      event.preventDefault();
      const next = structuredClone(this.data);
      next.paper = {
        width: numberOf(this.backdrop, "paperWidth"),
        height: numberOf(this.backdrop, "paperHeight"),
      };
      next.drawingSpace = valueOf(this.backdrop, "drawingSpace");
      next.autoPaginate = checked(this.backdrop, "autoPaginate");
      next.widthPages = numberOf(this.backdrop, "widthPages");
      next.heightPages = numberOf(this.backdrop, "heightPages");
      next.pageOrigin = nullablePair(this.backdrop, "pageOrigin");
      next.margins = ["marginTop", "marginRight", "marginBottom", "marginLeft"]
        .map((name) => numberOf(this.backdrop, name));
      next.pageOverlap = numberOf(this.backdrop, "pageOverlap");
      next.printTrimMarks = checked(this.backdrop, "printTrimMarks");
      next.header = valueOf(this.backdrop, "header");
      next.headerPosition = numberOf(this.backdrop, "headerPosition");
      next.footer = valueOf(this.backdrop, "footer");
      next.footerPosition = numberOf(this.backdrop, "footerPosition");
      next.magnificationPercent = numberOf(this.backdrop, "magnificationPercent");
      next.pageDefinition = valueOf(this.backdrop, "pageDefinition");
      next.legacySplitterPositionIds = stringList(valueOf(this.backdrop, "legacySplitterPositionIds"));
      try {
        next.splitters = JSON.parse(valueOf(this.backdrop, "splitters") || "[]");
      } catch {
        this.backdrop.querySelector("[data-layout-error]").textContent = "Splitter objects must contain valid structured data.";
        return;
      }
      next.fixInPlaceExtent = nullablePair(this.backdrop, "fixInPlaceExtent");
      next.fixInPlaceGap = nullablePair(this.backdrop, "fixInPlaceGap");
      const error = validateLayout(next);
      if (error) {
        this.backdrop.querySelector("[data-layout-error]").textContent = error;
        return;
      }
      this.close(next);
    });
  }

  applyOrientation() {
    const orientation = valueOf(this.backdrop, "orientation");
    const width = numberOf(this.backdrop, "paperWidth");
    const height = numberOf(this.backdrop, "paperHeight");
    if ((orientation === "portrait" && width > height) || (orientation === "landscape" && width < height)) {
      this.backdrop.querySelector('[name="paperWidth"]').value = height;
      this.backdrop.querySelector('[name="paperHeight"]').value = width;
    }
  }

  close(value) {
    this.backdrop?.remove();
    this.resolve?.(value);
  }
}

export function resolveLayout(layout, bounds) {
  if (!layout?.paper) {
    return null;
  }
  const paperWidth = Number(layout.paper.width);
  const paperHeight = Number(layout.paper.height);
  const overlap = layout.drawingSpace === "poster" ? Number(layout.pageOverlap || 0) : 0;
  const minColumns = Math.max(1, Number(layout.widthPages) || 1);
  const minRows = Math.max(1, Number(layout.heightPages) || 1);
  if (!bounds) {
    const anchorOrigin = layout.pageOrigin || [0, 0];
    return {
      origin: anchorOrigin,
      anchorOrigin,
      widthPages: minColumns,
      heightPages: minRows,
      prependedPages: [0, 0],
      totalWidth: pageSpan(paperWidth, overlap, minColumns),
      totalHeight: pageSpan(paperHeight, overlap, minRows),
    };
  }
  const contentWidth = Math.max(0, bounds.maxX - bounds.minX);
  const contentHeight = Math.max(0, bounds.maxY - bounds.minY);
  const centeredColumns = layout.autoPaginate
    ? Math.max(minColumns, pagesForSpan(contentWidth, paperWidth, overlap))
    : minColumns;
  const centeredRows = layout.autoPaginate
    ? Math.max(minRows, pagesForSpan(contentHeight, paperHeight, overlap))
    : minRows;
  const centeredOrigin = [
    bounds.minX - (pageSpan(paperWidth, overlap, centeredColumns) - contentWidth) / 2,
    bounds.minY - (pageSpan(paperHeight, overlap, centeredRows) - contentHeight) / 2,
  ];
  const anchorOrigin = layout.pageOrigin || centeredOrigin;
  const horizontal = resolvePaginationAxis(
    bounds.minX, bounds.maxX, anchorOrigin[0], minColumns,
    paperWidth, overlap, !!layout.autoPaginate,
  );
  const vertical = resolvePaginationAxis(
    bounds.minY, bounds.maxY, anchorOrigin[1], minRows,
    paperHeight, overlap, !!layout.autoPaginate,
  );
  return {
    origin: [horizontal.origin, vertical.origin],
    anchorOrigin,
    widthPages: horizontal.count,
    heightPages: vertical.count,
    prependedPages: [horizontal.prepend, vertical.prepend],
    totalWidth: pageSpan(paperWidth, overlap, horizontal.count),
    totalHeight: pageSpan(paperHeight, overlap, vertical.count),
  };
}

function pagesForSpan(span, paper, overlap) {
  if (span <= paper) return 1;
  return Math.max(1, Math.min(256, 1 + Math.ceil((span - paper) / Math.max(0.001, paper - overlap))));
}
function pageSpan(paper, overlap, count) {
  return paper * count - overlap * Math.max(0, count - 1);
}
function resolvePaginationAxis(contentMin, contentMax, anchorOrigin, minimumPages, paper, overlap, autoPaginate) {
  if (!autoPaginate) {
    return { origin: anchorOrigin, count: minimumPages, prepend: 0 };
  }
  const step = Math.max(0.001, paper - overlap);
  const prepend = contentMin < anchorOrigin - 1e-6
    ? Math.min(255, Math.ceil((anchorOrigin - contentMin) / step))
    : 0;
  const baseEnd = anchorOrigin + pageSpan(paper, overlap, minimumPages);
  const append = contentMax > baseEnd + 1e-6
    ? Math.min(255, Math.ceil((contentMax - baseEnd) / step))
    : 0;
  const count = Math.max(1, Math.min(256, minimumPages + prepend + append));
  const retainedPrepend = Math.min(prepend, count - minimumPages);
  return {
    origin: anchorOrigin - retainedPrepend * step,
    count,
    prepend: retainedPrepend,
  };
}
function appendHeaderFooter(layer, makeSvgNode, context) {
  const { documentData, layout, x, y, width, height, pageNumber } = context;
  const dynamic = {
    f: documentData?.document?.title || "Untitled",
    p: String(pageNumber),
    d: new Intl.DateTimeFormat(undefined, { dateStyle: "short" }).format(new Date()),
    t: new Intl.DateTimeFormat(undefined, { timeStyle: "short" }).format(new Date()),
  };
  for (const [text, baseline] of [
    [layout.header, y + Number(layout.headerPosition || 0)],
    [layout.footer, y + height - Number(layout.footerPosition || 0)],
  ]) {
    if (!text) continue;
    const sections = headerFooterSections(text, dynamic);
    for (const [anchor, value, position] of [
      ["start", sections.left, x + 6],
      ["middle", sections.center, x + width / 2],
      ["end", sections.right, x + width - 6],
    ]) {
      if (!value) continue;
      const node = makeSvgNode("text", {
        x: position,
        y: baseline,
        fill: "#4d535b",
        "font-family": "Arial, sans-serif",
        "font-size": 9,
        "text-anchor": anchor,
      });
      node.textContent = value;
      layer.appendChild(node);
    }
  }
}
function headerFooterSections(source, dynamic) {
  const sections = { left: "", center: "", right: "" };
  let target = "left";
  const chunks = String(source).split(/(&[lcr])/i);
  for (const chunk of chunks) {
    if (/^&[lcr]$/i.test(chunk)) {
      target = { l: "left", c: "center", r: "right" }[chunk[1].toLowerCase()];
    } else {
      sections[target] += chunk.replace(/&([fpdt])/gi, (_, token) => dynamic[token.toLowerCase()] || "");
    }
  }
  return sections;
}
function appendTrimMarks(layer, makeSvgNode, x, y, width, height) {
  const length = 9;
  const gap = 3;
  for (const [x1, y1, x2, y2] of [
    [x - gap - length, y, x - gap, y], [x, y - gap - length, x, y - gap],
    [x + width + gap, y, x + width + gap + length, y], [x + width, y - gap - length, x + width, y - gap],
    [x - gap - length, y + height, x - gap, y + height], [x, y + height + gap, x, y + height + gap + length],
    [x + width + gap, y + height, x + width + gap + length, y + height], [x + width, y + height + gap, x + width, y + height + gap + length],
  ]) {
    layer.appendChild(makeSvgNode("line", {
      x1, y1, x2, y2, stroke: "#60666e", "stroke-width": 0.55,
    }));
  }
}
function matchingPreset(presets = [], paper = {}) {
  const match = presets.find((entry) => (
    (near(entry.width, paper.width) && near(entry.height, paper.height))
    || (near(entry.width, paper.height) && near(entry.height, paper.width))
  ));
  return match?.key || "custom";
}
function validateLayout(layout) {
  const finitePositive = (value) => Number.isFinite(value) && value > 0;
  if (!finitePositive(layout.paper.width) || !finitePositive(layout.paper.height)) return "Paper dimensions must be positive.";
  if (!Number.isInteger(layout.widthPages) || !Number.isInteger(layout.heightPages) || layout.widthPages < 1 || layout.heightPages < 1) return "Page counts must be whole numbers of at least one.";
  if (layout.margins.some((value) => !Number.isFinite(value) || value < 0)) return "Margins must be non-negative.";
  if (layout.margins[1] + layout.margins[3] >= layout.paper.width || layout.margins[0] + layout.margins[2] >= layout.paper.height) return "Margins must leave a printable area.";
  if (!Number.isFinite(layout.pageOverlap) || layout.pageOverlap < 0 || layout.pageOverlap >= Math.min(layout.paper.width, layout.paper.height)) return "Poster overlap is outside the paper.";
  if (!Number.isFinite(layout.magnificationPercent) || layout.magnificationPercent < 1 || layout.magnificationPercent > 999) return "Magnification must be between 1% and 999%.";
  if (layout.pageOrigin?.some((value) => !Number.isFinite(value))) return "Both original page-origin coordinates are required.";
  for (const [label, pair] of [["In-place extent", layout.fixInPlaceExtent], ["In-place gap", layout.fixInPlaceGap]]) {
    if (pair?.some((value) => !Number.isFinite(value) || value < 0)) return `${label} requires two non-negative coordinates.`;
  }
  const definitions = new Set(["undefined", "center", "tl4", "id-term", "flush-left", "flush-right", "reaction1", "reaction2", "multicolumn-tl4", "multicolumn-non-tl4", "user-defined"]);
  if (!definitions.has(layout.pageDefinition)) return "Page definition is not supported.";
  if (!Array.isArray(layout.legacySplitterPositionIds) || layout.legacySplitterPositionIds.some((id) => typeof id !== "string" || !id.trim())) return "Legacy splitter object IDs must be non-empty strings.";
  if (!Array.isArray(layout.splitters)) return "Splitter objects must be a list.";
  const splitterIds = new Set();
  for (const splitter of layout.splitters) {
    if (!splitter || typeof splitter.id !== "string" || !splitter.id.trim() || splitterIds.has(splitter.id)) return "Splitter IDs must be non-empty and unique.";
    splitterIds.add(splitter.id);
    if (splitter.position != null && (!Array.isArray(splitter.position) || splitter.position.length !== 2 || splitter.position.some((value) => !Number.isFinite(value)))) return `Splitter '${splitter.id}' requires exactly two finite position coordinates.`;
    if (!definitions.has(splitter.pageDefinition || "undefined")) return `Splitter '${splitter.id}' has an unsupported page definition.`;
  }
  return "";
}
function tabButton(key, label, active = false) {
  return `<button type="button" data-layout-tab="${key}" class="${active ? "is-active" : ""}">${label}</button>`;
}
function numberField(name, label, value, step, unit = "", minimum = null, maximum = null) {
  const range = `${minimum == null ? "" : ` min="${minimum}"`}${maximum == null ? "" : ` max="${maximum}"`}`;
  const rendered = value === "" || value == null ? "" : Number(value);
  return `<label class="numeric-dialog-field"><span>${escapeHtml(label)}</span><input name="${name}" type="number" value="${rendered}" step="${step}"${range}><em>${unit}</em></label>`;
}
function textField(name, label, value) {
  return `<label class="numeric-dialog-field document-layout-wide-field"><span>${escapeHtml(label)}</span><input name="${name}" type="text" value="${escapeHtml(value)}"><em></em></label>`;
}
function textareaField(name, label, value, rows = 5) {
  return `<label class="numeric-dialog-field document-layout-wide-field"><span>${escapeHtml(label)}</span><textarea name="${name}" rows="${rows}">${escapeHtml(value)}</textarea><em></em></label>`;
}
function checkboxField(name, label, value) {
  return `<label class="document-layout-checkbox"><input name="${name}" type="checkbox"${value ? " checked" : ""}><span>${escapeHtml(label)}</span></label>`;
}
function selectField(name, label, value, entries) {
  return `<label class="numeric-dialog-field"><span>${escapeHtml(label)}</span><select name="${name}">${entries.map(([key, text]) => `<option value="${key}"${key === value ? " selected" : ""}>${escapeHtml(text)}</option>`).join("")}</select><em></em></label>`;
}
function nullablePairFields(name, label, value, minimum = 0) {
  return `${numberField(`${name}X`, `${label} X`, value?.[0] ?? "", 0.1, "pt", minimum)}${numberField(`${name}Y`, `${label} Y`, value?.[1] ?? "", 0.1, "pt", minimum)}`;
}
function nullablePair(root, name) {
  const x = valueOf(root, `${name}X`).trim();
  const y = valueOf(root, `${name}Y`).trim();
  if (!x && !y) return null;
  if (!x || !y) return [Number.NaN, Number.NaN];
  return [Number(x), Number(y)];
}
function stringList(value) {
  if (!String(value).trim()) return [];
  return String(value).split(/[\s,;]+/).filter(Boolean).map((entry) => entry.trim());
}
function valueOf(root, name) {
  return String(root.querySelector(`[name="${CSS.escape(name)}"]`)?.value ?? "");
}
function numberOf(root, name) {
  return Number(valueOf(root, name));
}
function checked(root, name) {
  return !!root.querySelector(`[name="${CSS.escape(name)}"]`)?.checked;
}
function divider() {
  const node = document.createElement("div");
  node.className = "document-layout-menu-divider";
  return node;
}
function near(a, b) {
  return Math.abs(Number(a) - Number(b)) < 0.05;
}
function formatNumber(value) {
  return Number(value).toFixed(2).replace(/\.?0+$/, "");
}
function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}
