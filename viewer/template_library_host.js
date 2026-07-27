const CATALOG_URL = "./template-libraries/catalog.json";
const STORAGE_KEY = "chemsema.template-library-state.v2";

export function createTemplateLibraryHost(options) {
  const {
    root = document,
    state,
    editorState,
    secondaryToolbar,
    viewerContainer,
  } = options;
  const popup = createPalettePopup(root);
  let catalogPromise = null;
  let activeLibraryPromise = null;
  let placementPointerId = null;
  const libraryCdxml = new Map();

  function storedState() {
    try {
      const value = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
      return {
        recent: Array.isArray(value.recent) ? value.recent.slice(0, 24) : [],
        favorites: Array.isArray(value.favorites) ? value.favorites : [],
        layouts: value.layouts && typeof value.layouts === "object" ? value.layouts : {},
      };
    } catch {
      return { recent: [], favorites: [], layouts: {} };
    }
  }

  function writeStoredState(value) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      recent: value.recent.slice(0, 24),
      favorites: [...new Set(value.favorites)],
      layouts: value.layouts || {},
    }));
  }

  async function ensureCatalog() {
    if (Array.isArray(editorState.templateLibraries) && editorState.templateLibraries.length) {
      return editorState.templateLibraries;
    }
    if (!catalogPromise) {
      catalogPromise = fetch(CATALOG_URL, { cache: "no-cache" })
        .then((response) => {
          if (!response.ok) throw new Error(`Template catalog request failed (${response.status}).`);
          return response.json();
        })
        .then((catalog) => {
          if (catalog?.schema !== "chemsema.template-library-catalog.v2") {
            throw new Error("Unsupported template catalog schema.");
          }
          const previewIcon = state.editorEngine?.templatePreviewIconSvg;
          if (typeof previewIcon !== "function") {
            throw new Error("The ChemSema kernel does not support template previews.");
          }
          const libraries = (catalog.libraries || []).map((library) => ({
            ...library,
            iconSvg: previewIcon.call(state.editorEngine, library.iconCdxml),
          }));
          editorState.templateLibraryCatalog = catalog;
          editorState.templateLibraries = libraries;
          return libraries;
        })
        .catch((error) => {
          catalogPromise = null;
          throw error;
        });
    }
    return catalogPromise;
  }

  async function enterTemplateMode() {
    if (editorState.toolRailMode === "templates") return;
    editorState.toolRailLastDrawingMode = editorState.toolRailMode === "biology" ? "biology" : "main";
    await options.activateEditorTool?.("select");
    editorState.toolRailMode = "templates";
    editorState.templateLibrariesLoading = true;
    options.syncEditorPrimaryToolButtons?.();
    options.renderSecondaryToolbar?.();
    try {
      await ensureCatalog();
    } catch (error) {
      options.notify?.(`Template libraries could not be loaded: ${error.message || error}`);
    } finally {
      editorState.templateLibrariesLoading = false;
      options.syncEditorPrimaryToolButtons?.();
      options.renderSecondaryToolbar?.();
    }
  }

  async function leaveTemplateMode() {
    closePopup();
    clearActiveTemplate();
    editorState.toolRailMode = editorState.toolRailLastDrawingMode === "biology"
      ? "biology"
      : "main";
    await options.activateEditorTool?.("select");
    options.syncEditorPrimaryToolButtons?.();
    options.renderSecondaryToolbar?.();
    options.syncCanvasCursor?.();
  }

  async function toggleTemplateMode() {
    if (editorState.toolRailMode === "templates") {
      await leaveTemplateMode();
    } else {
      await enterTemplateMode();
    }
  }

  async function openLibrary(libraryId, anchor) {
    const libraries = await ensureCatalog();
    const library = libraries.find((entry) => entry.id === libraryId);
    if (!library) return;
    if (editorState.activeTemplateLibraryId === libraryId && !popup.hidden) {
      closePopup();
      return;
    }
    editorState.activeTemplateLibraryId = libraryId;
    options.renderSecondaryToolbar?.();
    anchor = libraryAnchor(libraryId) || anchor;
    showLoading(library.name, anchor);
    const request = fetch(library.path, { cache: "no-cache" })
      .then((response) => {
        if (!response.ok) throw new Error(`Template library request failed (${response.status}).`);
        return response.text();
      })
      .then((sourceCdxml) => {
        const storage = storedState();
        const storedLayout = storage.layouts[library.id];
        const cdxml = storedLayout
          ? state.editorEngine.applyTemplateLibraryLayoutJson(sourceCdxml, JSON.stringify(storedLayout))
          : sourceCdxml;
        libraryCdxml.set(library.id, { sourceCdxml, cdxml });
        const json = state.editorEngine.templateLibraryPaletteJson(
          library.id,
          library.name,
          cdxml,
        );
        const palette = options.parseEngineJson(json, null);
        if (palette?.type !== "template-library-palette") {
          throw new Error("The kernel returned an invalid template palette.");
        }
        editorState.templateLibraryPalettes ||= {};
        editorState.templateLibraryPalettes[library.id] = palette;
        return palette;
      });
    activeLibraryPromise = request;
    try {
      const palette = await request;
      if (activeLibraryPromise !== request || editorState.activeTemplateLibraryId !== libraryId) {
        return;
      }
      renderPalette(library, palette, anchor);
    } catch (error) {
      if (activeLibraryPromise === request) {
        showError(library.name, error, anchor);
      }
    }
  }

  function libraryAnchor(libraryId) {
    return secondaryToolbar?.querySelector(
      `[data-template-library-id="${CSS.escape(libraryId)}"]`,
    );
  }

  function showLoading(name, anchor) {
    popup.innerHTML = `
      <div class="template-palette-header">
        <strong>${escapeHtml(name)}</strong>
        <span class="template-palette-count">Loading…</span>
      </div>
      <div class="template-palette-loading" role="status">Rendering templates with the ChemSema kernel…</div>
    `;
    showPopupAt(anchor);
  }

  function showError(name, error, anchor) {
    popup.innerHTML = `
      <div class="template-palette-header"><strong>${escapeHtml(name)}</strong></div>
      <div class="template-palette-error" role="alert">${escapeHtml(error?.message || String(error))}</div>
    `;
    showPopupAt(anchor);
  }

  function renderPalette(library, palette, anchor) {
    const storage = storedState();
    const favoriteSet = new Set(storage.favorites);
    const items = palette.templates || [];
    const layout = palette.library?.layout;
    if (!layout || !Array.isArray(layout.cells)) {
      throw new Error("The kernel omitted the template-grid layout.");
    }
    const itemById = new Map(items.map((item) => [item.id, item]));
    popup.innerHTML = `
      <div class="template-palette-header">
        <strong>${escapeHtml(library.name)}</strong>
        <button class="template-palette-layout-button" type="button" data-template-layout
          title="Edit template-grid layout">Layout…</button>
        <button class="template-palette-layout-button" type="button" data-template-export
          title="Download the edited library as CDXML">Export</button>
        <button class="template-palette-layout-button" type="button" data-template-reset
          title="Restore the source library layout">Reset</button>
        <span class="template-palette-count">${items.length}</span>
      </div>
      <label class="template-palette-search">
        <span class="visually-hidden">Search ${escapeHtml(library.name)}</span>
        <input type="search" placeholder="Search templates" autocomplete="off" spellcheck="false">
      </label>
      <div class="template-palette-grid" role="listbox" aria-label="${escapeHtml(library.name)} templates"
        style="--template-columns:${layout.columns};--template-cell-ratio:${layout.extent[0]} / ${layout.extent[1]};--template-pane-height:${templatePaneHeightPx(layout)}px">
        ${layout.cells.map((templateId, cellIndex) => {
          const item = templateId ? itemById.get(templateId) : null;
          return item
            ? templateItemHtml(item, favoriteSet.has(item.id), cellIndex)
            : `<div class="template-palette-empty-cell" data-template-cell="${cellIndex}" aria-hidden="true"></div>`;
        }).join("")}
      </div>
    `;
    showPopupAt(anchor);
    const search = popup.querySelector("input[type=search]");
    search?.addEventListener("input", () => {
      const query = search.value.trim().toLocaleLowerCase();
      popup.querySelectorAll("[data-template-id]").forEach((button) => {
        const label = button.dataset.templateLabel?.toLocaleLowerCase() || "";
        button.hidden = Boolean(query && !label.includes(query));
      });
    });
    bindGridReordering(library, palette, anchor);
    search?.focus({ preventScroll: true });
  }

  function templateItemHtml(item, favorite, cellIndex) {
    return `
      <div class="template-palette-item-wrap" data-template-cell="${cellIndex}" draggable="true">
        <button class="template-palette-item${editorState.activeDocumentTemplate?.id === item.id ? " is-selected" : ""}"
          type="button" role="option" data-template-id="${escapeHtml(item.id)}"
          data-template-label="${escapeHtml(item.label)}" aria-label="${escapeHtml(item.label)}"
          title="${escapeHtml(item.label)}">
          ${item.iconSvg || ""}
        </button>
        <button class="template-favorite-button${favorite ? " is-favorite" : ""}" type="button"
          data-template-favorite="${escapeHtml(item.id)}"
          aria-label="${favorite ? "Remove from favorites" : "Add to favorites"}"
          title="${favorite ? "Remove from favorites" : "Add to favorites"}">★</button>
      </div>
    `;
  }

  async function openLayoutDialog(libraryId, anchor) {
    const libraries = await ensureCatalog();
    const library = libraries.find((entry) => entry.id === libraryId);
    if (!library) return;
    if (!libraryCdxml.has(libraryId)) {
      if (activeLibraryPromise && editorState.activeTemplateLibraryId === libraryId) {
        await activeLibraryPromise;
      } else {
        await openLibrary(libraryId, anchor);
      }
    }
    const current = libraryCdxml.get(libraryId);
    if (!current) return;
    const spec = options.parseEngineJson(
      state.editorEngine.templateLibraryLayoutDialogJson(current.sourceCdxml),
      null,
    );
    if (!spec) {
      throw new Error("The kernel returned an invalid template-grid dialog.");
    }
    const savedLayout = storedState().layouts[library.id];
    if (savedLayout) spec.data = structuredClone(savedLayout);
    const next = await new TemplateGridDialog({ root, spec }).open();
    if (!next) return;
    const cdxml = state.editorEngine.applyTemplateLibraryLayoutJson(
      current.sourceCdxml,
      JSON.stringify(next),
    );
    const palette = JSON.parse(state.editorEngine.templateLibraryPaletteJson(
      library.id,
      library.name,
      cdxml,
    ));
    libraryCdxml.set(library.id, { ...current, cdxml });
    editorState.templateLibraryPalettes[library.id] = palette;
    const storage = storedState();
    storage.layouts[library.id] = next;
    writeStoredState(storage);
    renderPalette(library, palette, anchor);
  }

  function bindGridReordering(library, palette, anchor) {
    const grid = popup.querySelector(".template-palette-grid");
    if (!grid) return;
    let draggedCell = null;
    grid.addEventListener("dragstart", (event) => {
      const cell = event.target.closest("[data-template-cell]");
      if (!cell || !cell.querySelector("[data-template-id]")) {
        event.preventDefault();
        return;
      }
      draggedCell = Number(cell.dataset.templateCell);
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", String(draggedCell));
      grid.classList.add("is-reordering");
    });
    grid.addEventListener("dragover", (event) => {
      if (draggedCell == null || !event.target.closest("[data-template-cell]")) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
    });
    grid.addEventListener("drop", (event) => {
      const target = event.target.closest("[data-template-cell]");
      if (draggedCell == null || !target) return;
      event.preventDefault();
      const targetCell = Number(target.dataset.templateCell);
      if (targetCell !== draggedCell) {
        void reorderCells(library, palette, draggedCell, targetCell, anchor);
      }
      draggedCell = null;
      grid.classList.remove("is-reordering");
    });
    grid.addEventListener("dragend", () => {
      draggedCell = null;
      grid.classList.remove("is-reordering");
    });
  }

  async function reorderCells(library, palette, from, to, anchor) {
    const current = libraryCdxml.get(library.id);
    if (!current) return;
    const storage = storedState();
    const layout = storage.layouts[library.id]
      ? structuredClone(storage.layouts[library.id])
      : JSON.parse(state.editorEngine.templateLibraryLayoutJson(current.sourceCdxml));
    [layout.cells[from], layout.cells[to]] = [layout.cells[to], layout.cells[from]];
    const cdxml = state.editorEngine.applyTemplateLibraryLayoutJson(
      current.sourceCdxml,
      JSON.stringify(layout),
    );
    const nextPalette = JSON.parse(state.editorEngine.templateLibraryPaletteJson(
      library.id,
      library.name,
      cdxml,
    ));
    libraryCdxml.set(library.id, { ...current, cdxml });
    editorState.templateLibraryPalettes[library.id] = nextPalette;
    storage.layouts[library.id] = layout;
    writeStoredState(storage);
    renderPalette(library, nextPalette, anchor);
  }

  function exportLibrary(libraryId) {
    const library = editorState.templateLibraries?.find((entry) => entry.id === libraryId);
    const current = libraryCdxml.get(libraryId);
    if (!library || !current) return;
    const blob = new Blob([current.cdxml], { type: "chemical/x-cdxml;charset=utf-8" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `${library.name}.cdxml`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 0);
  }

  function resetLibraryLayout(libraryId, anchor) {
    const library = editorState.templateLibraries?.find((entry) => entry.id === libraryId);
    const current = libraryCdxml.get(libraryId);
    if (!library || !current) return;
    const palette = JSON.parse(state.editorEngine.templateLibraryPaletteJson(
      library.id,
      library.name,
      current.sourceCdxml,
    ));
    libraryCdxml.set(library.id, { ...current, cdxml: current.sourceCdxml });
    editorState.templateLibraryPalettes[library.id] = palette;
    const storage = storedState();
    delete storage.layouts[library.id];
    writeStoredState(storage);
    renderPalette(library, palette, anchor);
  }

  async function selectTemplate(templateId) {
    const palette = editorState.templateLibraryPalettes?.[editorState.activeTemplateLibraryId];
    const item = palette?.templates?.find((entry) => entry.id === templateId);
    if (!item) return;
    editorState.activeDocumentTemplate = {
      id: item.id,
      label: item.label,
      documentJson: item.documentJson,
      iconSvg: item.iconSvg,
    };
    const storage = storedState();
    storage.recent = [item.id, ...storage.recent.filter((id) => id !== item.id)];
    writeStoredState(storage);
    closePopup();
    await options.activateEditorTool?.("templates");
    options.syncEditorPrimaryToolButtons?.();
    options.renderSecondaryToolbar?.();
    options.syncCanvasCursor?.();
  }

  function toggleFavorite(templateId) {
    const storage = storedState();
    const current = new Set(storage.favorites);
    if (current.has(templateId)) current.delete(templateId);
    else current.add(templateId);
    storage.favorites = [...current];
    writeStoredState(storage);
    const button = popup.querySelector(`[data-template-favorite="${CSS.escape(templateId)}"]`);
    if (button) {
      const favorite = current.has(templateId);
      button.classList.toggle("is-favorite", favorite);
      button.setAttribute("aria-label", favorite ? "Remove from favorites" : "Add to favorites");
      button.title = favorite ? "Remove from favorites" : "Add to favorites";
    }
  }

  function clearActiveTemplate() {
    editorState.activeDocumentTemplate = null;
    placementPointerId = null;
  }

  function closePopup() {
    popup.hidden = true;
    popup.classList.remove("is-open");
    activeLibraryPromise = null;
  }

  function showPopupAt(anchor) {
    popup.hidden = false;
    popup.classList.add("is-open");
    const rect = anchor?.getBoundingClientRect?.();
    const left = rect ? rect.left : 72;
    const top = rect ? rect.bottom + 5 : 110;
    popup.style.left = `${Math.max(8, Math.min(left, window.innerWidth - popup.offsetWidth - 8))}px`;
    popup.style.top = `${Math.max(8, Math.min(top, window.innerHeight - popup.offsetHeight - 8))}px`;
  }

  function bind() {
    root.querySelector("[data-template-rail-toggle]")?.addEventListener("click", () => {
      void toggleTemplateMode();
    });
    secondaryToolbar?.addEventListener("click", (event) => {
      const libraryButton = event.target.closest("[data-template-library-id]");
      if (libraryButton) {
        event.preventDefault();
        void openLibrary(libraryButton.dataset.templateLibraryId, libraryButton);
      }
    });
    popup.addEventListener("click", (event) => {
      const layoutButton = event.target.closest("[data-template-layout]");
      if (layoutButton) {
        event.preventDefault();
        const anchor = secondaryToolbar?.querySelector(
          `[data-template-library-id="${CSS.escape(editorState.activeTemplateLibraryId)}"]`,
        );
        void openLayoutDialog(editorState.activeTemplateLibraryId, anchor);
        return;
      }
      if (event.target.closest("[data-template-export]")) {
        event.preventDefault();
        exportLibrary(editorState.activeTemplateLibraryId);
        return;
      }
      if (event.target.closest("[data-template-reset]")) {
        event.preventDefault();
        resetLibraryLayout(
          editorState.activeTemplateLibraryId,
          libraryAnchor(editorState.activeTemplateLibraryId),
        );
        return;
      }
      const favorite = event.target.closest("[data-template-favorite]");
      if (favorite) {
        event.preventDefault();
        event.stopPropagation();
        toggleFavorite(favorite.dataset.templateFavorite);
        return;
      }
      const item = event.target.closest("[data-template-id]");
      if (item) {
        event.preventDefault();
        void selectTemplate(item.dataset.templateId);
      }
    });
    secondaryToolbar?.addEventListener("contextmenu", (event) => {
      const libraryButton = event.target.closest("[data-template-library-id]");
      if (!libraryButton) return;
      event.preventDefault();
      void openLayoutDialog(libraryButton.dataset.templateLibraryId, libraryButton);
    });
    root.addEventListener("pointerdown", (event) => {
      if (popup.hidden || popup.contains(event.target) || event.target.closest("[data-template-library-id]")) return;
      closePopup();
    });
    root.addEventListener("keydown", (event) => {
      if (event.key !== "Escape") return;
      if (!popup.hidden) {
        closePopup();
        return;
      }
      if (editorState.activeDocumentTemplate) {
        clearActiveTemplate();
        void options.activateEditorTool?.("select");
      }
    });
    viewerContainer?.addEventListener("pointerdown", handlePlacementPointerDown, true);
    viewerContainer?.addEventListener("pointerup", handlePlacementPointerUp, true);
    viewerContainer?.addEventListener("pointercancel", handlePlacementPointerCancel, true);
  }

  function placementIsActive() {
    return editorState.toolRailMode === "templates"
      && editorState.activeTool === "templates"
      && Boolean(editorState.activeDocumentTemplate?.documentJson);
  }

  function handlePlacementPointerDown(event) {
    if (!placementIsActive() || event.button !== 0) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    placementPointerId = event.pointerId;
    viewerContainer?.setPointerCapture?.(event.pointerId);
  }

  async function handlePlacementPointerUp(event) {
    if (!placementIsActive() || placementPointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    placementPointerId = null;
    viewerContainer?.releasePointerCapture?.(event.pointerId);
    const point = options.svgPointFromEvent?.(event);
    if (!point) return;
    const template = editorState.activeDocumentTemplate;
    const changed = await state.editorEngine?.insertDocumentTemplateJsonAt?.(
      template.id,
      template.documentJson,
      point.x,
      point.y,
    );
    if (changed === false) return;
    await options.syncDocumentFromEngine?.();
    options.renderDocument?.();
    options.renderSecondaryToolbar?.();
  }

  function handlePlacementPointerCancel(event) {
    if (placementPointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    placementPointerId = null;
  }

  bind();
  return {
    enterTemplateMode,
    leaveTemplateMode,
    toggleTemplateMode,
    closePopup,
  };
}

class TemplateGridDialog {
  constructor({ root, spec }) {
    this.root = root;
    this.spec = spec;
    this.data = structuredClone(spec.data);
  }

  open() {
    document.querySelector(".template-grid-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog template-grid-dialog";
      this.backdrop.innerHTML = `
        <div class="numeric-dialog-backdrop" data-template-grid-close></div>
        <form class="numeric-dialog-panel template-grid-dialog-panel" aria-label="${escapeHtml(this.spec.title)}">
          <div class="numeric-dialog-title">${escapeHtml(this.spec.title)}</div>
          <div class="template-grid-dialog-fields">
            ${this.spec.fields.map((field) => this.fieldHtml(field)).join("")}
          </div>
          <p class="template-grid-dialog-hint">Templates are assigned in row-major order. Empty cells are preserved explicitly.</p>
          <div class="numeric-dialog-error" data-template-grid-error role="alert"></div>
          <div class="numeric-dialog-actions">
            <button type="button" data-template-grid-close>Cancel</button>
            <button type="submit">OK</button>
          </div>
        </form>`;
      this.root.body.appendChild(this.backdrop);
      this.bind();
    });
  }

  fieldHtml(field) {
    const value = field.key.startsWith("extent.")
      ? this.data.extent[Number(field.key.split(".")[1])]
      : this.data[field.key];
    return `<label><span>${escapeHtml(field.label)}</span><input name="${escapeHtml(field.key)}"
      type="number" value="${value}" ${field.kind === "integer" ? 'step="1"' : 'step="any"'}
      ${field.minimum != null ? `min="${field.minimum}"` : ""}
      ${field.maximum != null ? `max="${field.maximum}"` : ""} required>
      ${field.unit ? `<small>${escapeHtml(field.unit)}</small>` : ""}</label>`;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-template-grid-close]")) this.close(null);
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") this.close(null);
    });
    this.backdrop.querySelector("form").addEventListener("submit", (event) => {
      event.preventDefault();
      const rows = Number(this.value("rows"));
      const columns = Number(this.value("columns"));
      const capacity = rows * columns;
      if (this.data.cells.slice(capacity).some((cell) => cell != null)) {
        this.error("The smaller grid would remove a template. Rearrange templates first.");
        return;
      }
      const cells = this.data.cells.slice(0, capacity);
      cells.length = capacity;
      for (let index = 0; index < capacity; index += 1) {
        if (cells[index] === undefined) cells[index] = null;
      }
      const next = {
        rows,
        columns,
        paneHeight: Number(this.value("paneHeight")),
        extent: [Number(this.value("extent.0")), Number(this.value("extent.1"))],
        cells,
      };
      if (!Number.isInteger(rows) || rows <= 0 || !Number.isInteger(columns) || columns <= 0) {
        this.error("Rows and columns must be positive integers.");
        return;
      }
      if (capacity < this.spec.templateCount) {
        this.error(`The grid needs at least ${this.spec.templateCount} cells.`);
        return;
      }
      if (![next.paneHeight, ...next.extent].every((value) => Number.isFinite(value) && value > 0)) {
        this.error("Pane height and cell extent must be positive numbers.");
        return;
      }
      this.close(next);
    });
  }

  value(name) {
    return this.backdrop.querySelector(`[name="${CSS.escape(name)}"]`)?.value;
  }

  error(message) {
    this.backdrop.querySelector("[data-template-grid-error]").textContent = message;
  }

  close(value) {
    this.backdrop?.remove();
    this.resolve?.(value);
  }
}

function createPalettePopup(root) {
  const popup = root.createElement("section");
  popup.className = "template-palette";
  popup.setAttribute("aria-label", "Template library");
  popup.hidden = true;
  root.body.appendChild(popup);
  return popup;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function templatePaneHeightPx(layout) {
  const referenceCellWidth = 72;
  const horizontalScale = referenceCellWidth / Number(layout.extent[0]);
  const visibleRows = Number(layout.paneHeight) / Number(layout.extent[1]);
  const rowGaps = Math.max(0, Math.ceil(visibleRows) - 1) * 7;
  return Math.max(120, Math.min(820, Number(layout.paneHeight) * horizontalScale + rowGaps));
}
