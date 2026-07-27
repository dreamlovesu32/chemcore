const CATALOG_URL = "./template-libraries/catalog.json";
const STORAGE_KEY = "chemsema.template-library-state.v1";

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

  function storedState() {
    try {
      const value = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
      return {
        recent: Array.isArray(value.recent) ? value.recent.slice(0, 24) : [],
        favorites: Array.isArray(value.favorites) ? value.favorites : [],
      };
    } catch {
      return { recent: [], favorites: [] };
    }
  }

  function writeStoredState(value) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      recent: value.recent.slice(0, 24),
      favorites: [...new Set(value.favorites)],
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
          if (catalog?.schema !== "chemsema.template-library-catalog.v1") {
            throw new Error("Unsupported template catalog schema.");
          }
          const previewIcon = state.editorEngine?.templatePreviewIconSvg;
          const libraries = (catalog.libraries || []).map((library) => ({
            ...library,
            iconSvg: typeof previewIcon === "function"
              ? previewIcon.call(state.editorEngine, library.iconCdxml)
              : "",
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
    showLoading(library.name, anchor);
    const request = fetch(library.path, { cache: "no-cache" })
      .then((response) => {
        if (!response.ok) throw new Error(`Template library request failed (${response.status}).`);
        return response.text();
      })
      .then((cdxml) => {
        const json = state.editorEngine?.templateLibraryPaletteJson?.(
          library.id,
          library.name,
          cdxml,
        );
        const palette = options.parseEngineJson?.(json, null) || JSON.parse(json);
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
    popup.innerHTML = `
      <div class="template-palette-header">
        <strong>${escapeHtml(library.name)}</strong>
        <span class="template-palette-count">${items.length}</span>
      </div>
      <label class="template-palette-search">
        <span class="visually-hidden">Search ${escapeHtml(library.name)}</span>
        <input type="search" placeholder="Search templates" autocomplete="off" spellcheck="false">
      </label>
      <div class="template-palette-grid" role="listbox" aria-label="${escapeHtml(library.name)} templates">
        ${items.map((item) => templateItemHtml(item, favoriteSet.has(item.id))).join("")}
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
    search?.focus({ preventScroll: true });
  }

  function templateItemHtml(item, favorite) {
    return `
      <div class="template-palette-item-wrap">
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
