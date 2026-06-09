export function createTextSymbolPalette({ mount, payload, elementPayload, onSelect, onElementSelect, onModeChange }) {
  if (!mount) {
    return null;
  }
  const root = document.createElement("div");
  root.className = "quick-palette";
  root.dataset.mode = "symbol";

  const panel = document.createElement("div");
  panel.className = "quick-palette-panel";
  panel.setAttribute("role", "menu");

  const header = document.createElement("div");
  header.className = "quick-palette-header";

  const title = document.createElement("div");
  title.className = "quick-palette-title";

  const pin = document.createElement("button");
  pin.type = "button";
  pin.className = "quick-palette-pin";
  pin.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4v1H6v-1l4-4z"/><path d="M12 15v5"/></svg>`;

  header.append(title, pin);
  panel.appendChild(header);

  const symbolContent = document.createElement("div");
  symbolContent.className = "quick-palette-content text-symbol-content";
  symbolContent.dataset.paletteMode = "symbol";

  const elementContent = document.createElement("div");
  elementContent.className = "quick-palette-content periodic-table-content";
  elementContent.dataset.paletteMode = "element";

  panel.append(symbolContent, elementContent);

  const controls = document.createElement("div");
  controls.className = "quick-palette-controls";

  const elementToggle = document.createElement("button");
  elementToggle.type = "button";
  elementToggle.className = "quick-palette-toggle quick-palette-toggle-element";
  elementToggle.dataset.quickPaletteMode = "element";
  elementToggle.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="12" y="16.5" text-anchor="middle" font-size="10pt" font-family="Arial, Helvetica, sans-serif">P</text></svg>`;
  const elementToggleText = elementToggle.querySelector("text");

  const symbolToggle = document.createElement("button");
  symbolToggle.type = "button";
  symbolToggle.className = "quick-palette-toggle quick-palette-toggle-symbol";
  symbolToggle.dataset.quickPaletteMode = "symbol";
  symbolToggle.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="12" y="16.5" text-anchor="middle" font-size="10pt" font-family="Arial, Helvetica, sans-serif">&#937;</text></svg>`;

  controls.append(elementToggle, symbolToggle);
  root.append(controls, panel);
  mount.appendChild(root);

  let symbolCatalog = normalizeTextSymbolPalette(payload);
  let elementCatalog = normalizeElementPalette(elementPayload);

  function setSymbolPayload(nextPayload) {
    symbolCatalog = normalizeTextSymbolPalette(nextPayload);
    symbolToggle.title = symbolCatalog.toggleLabel;
    symbolToggle.setAttribute("aria-label", symbolCatalog.toggleLabel);
    renderSymbolContent();
    syncState();
  }

  function setElementPayload(nextPayload) {
    elementCatalog = normalizeElementPalette(nextPayload, elementCatalog.current.symbol);
    elementToggle.title = elementCatalog.toggleLabel;
    elementToggle.setAttribute("aria-label", elementCatalog.toggleLabel);
    renderElementContent();
    syncState();
  }

  function renderSymbolContent() {
    symbolContent.innerHTML = "";
    for (const group of symbolCatalog.groups) {
      const section = document.createElement("section");
      section.className = "text-symbol-section";
      const label = document.createElement("div");
      label.className = "text-symbol-section-label";
      label.textContent = group.label;
      const grid = document.createElement("div");
      grid.className = "text-symbol-grid";
      for (const character of group.characters) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "text-symbol-cell";
        button.textContent = character;
        button.title = character;
        button.setAttribute("aria-label", character);
        button.addEventListener("click", (event) => {
          event.preventDefault();
          onSelect?.(character);
          closeAfterPick("symbol");
        });
        grid.appendChild(button);
      }
      section.append(label, grid);
      symbolContent.appendChild(section);
    }
  }

  function renderElementContent() {
    elementContent.innerHTML = "";
    const grid = document.createElement("div");
    grid.className = "periodic-table-grid";
    for (const element of elementCatalog.elements) {
      grid.appendChild(periodicElementButton(element));
    }
    elementContent.appendChild(grid);
  }

  function periodicElementButton(element) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = [
      "periodic-element-button",
      element.color ? "has-element-color" : "",
    ].filter(Boolean).join(" ");
    button.setAttribute("role", "menuitem");
    button.dataset.elementSymbol = element.symbol;
    button.dataset.elementAtomicNumber = String(element.atomicNumber);
    button.title = `${element.atomicNumber} ${element.name}`;
    button.setAttribute("aria-label", `${element.atomicNumber} ${element.name}`);
    button.style.gridColumn = String(element.column);
    button.style.gridRow = String(element.row >= 8 ? element.row + 1 : element.row);
    if (element.color?.background) {
      button.style.setProperty("--element-bg", element.color.background);
    }
    if (element.color?.foreground) {
      button.style.setProperty("--element-fg", element.color.foreground);
    }
    button.textContent = element.symbol;
    button.addEventListener("click", async (event) => {
      event.preventDefault();
      const changed = await onElementSelect?.(element.symbol, element.atomicNumber);
      elementCatalog = {
        ...elementCatalog,
        current: element,
      };
      renderElementContent();
      closeAfterPick("element");
      return changed;
    });
    return button;
  }

  function setOpen(open, mode = currentMode(), options = {}) {
    if (open) {
      root.dataset.mode = mode === "element" ? "element" : "symbol";
      root.classList.add("is-open");
    } else {
      root.classList.remove("is-open");
    }
    syncState();
    onModeChange?.({
      open: root.classList.contains("is-open"),
      mode: currentMode(),
      keepElementPlacement: !!options.keepElementPlacement,
    });
    if (open) {
      document.dispatchEvent(new CustomEvent("chemcore:quick-palette-open", { detail: { mode: currentMode() } }));
    }
  }

  function toggleMode(mode) {
    const nextMode = mode === "element" ? "element" : "symbol";
    if (root.classList.contains("is-open") && currentMode() === nextMode) {
      setOpen(false, nextMode);
      return;
    }
    setOpen(true, nextMode);
  }

  function closeAfterPick(mode) {
    if (!root.classList.contains("is-pinned")) {
      setOpen(false, mode, { keepElementPlacement: mode === "element" });
    }
  }

  function currentMode() {
    return root.dataset.mode === "element" ? "element" : "symbol";
  }

  function syncState() {
    const open = root.classList.contains("is-open");
    const mode = currentMode();
    const titleText = mode === "element" ? elementCatalog.title : symbolCatalog.title;
    if (elementToggleText) {
      elementToggleText.textContent = elementCatalog.current.symbol || "P";
    }
    title.textContent = titleText;
    panel.setAttribute("aria-label", titleText);
    pin.title = mode === "element" ? elementCatalog.pinLabel : symbolCatalog.pinLabel;
    pin.setAttribute("aria-label", pin.title);
    elementToggle.setAttribute("aria-expanded", open && mode === "element" ? "true" : "false");
    symbolToggle.setAttribute("aria-expanded", open && mode === "symbol" ? "true" : "false");
    elementToggle.classList.toggle("is-selected", open && mode === "element");
    symbolToggle.classList.toggle("is-selected", open && mode === "symbol");
    pin.classList.toggle("is-selected", root.classList.contains("is-pinned"));
  }

  symbolToggle.addEventListener("click", (event) => {
    event.preventDefault();
    toggleMode("symbol");
  });

  elementToggle.addEventListener("click", (event) => {
    event.preventDefault();
    toggleMode("element");
  });

  pin.addEventListener("click", (event) => {
    event.preventDefault();
    const pinned = !root.classList.contains("is-pinned");
    root.classList.toggle("is-pinned", pinned);
    if (pinned && !root.classList.contains("is-open")) {
      setOpen(true, currentMode());
      return;
    }
    syncState();
  });

  root.addEventListener("mousedown", (event) => {
    event.preventDefault();
  });

  document.addEventListener("chemcore:quick-palette-toggle", (event) => {
    toggleMode(event.detail?.mode);
  });
  document.addEventListener("chemcore:quick-palette-open-mode", (event) => {
    setOpen(true, event.detail?.mode);
  });
  document.addEventListener("pointerdown", (event) => {
    if (event.target.closest?.(".quick-palette")) {
      return;
    }
    if (!root.classList.contains("is-pinned") && root.classList.contains("is-open")) {
      setOpen(false);
    }
  });

  setSymbolPayload(payload);
  setElementPayload(elementPayload);
  setOpen(false);
  return {
    root,
    setOpen,
    toggleMode,
    setPayload: setSymbolPayload,
    setSymbolPayload,
    setElementPayload,
  };
}

function normalizeTextSymbolPalette(manifest) {
  const payload = typeof manifest === "string" ? safeJsonParse(manifest, null) : manifest;
  return {
    version: Number(payload?.version || 1),
    title: String(payload?.title || "Symbol"),
    toggleLabel: String(payload?.toggleLabel || "Text symbols"),
    pinLabel: String(payload?.pinLabel || "Pin"),
    groups: (payload?.groups || [])
      .map((group) => ({
        id: String(group?.id || ""),
        label: String(group?.label || group?.id || "Symbols"),
        characters: Array.from(String(group?.characters || "")),
      }))
      .filter((group) => group.id && group.characters.length),
  };
}

function normalizeElementPalette(palette, currentSymbol = "P") {
  const payload = typeof palette === "string" ? safeJsonParse(palette, null) : palette;
  const elements = (payload?.elements || [])
    .map((element) => ({
      symbol: String(element?.symbol || ""),
      atomicNumber: Number(element?.atomicNumber) || 0,
      name: String(element?.name || ""),
      column: Number(element?.column) || 1,
      row: Number(element?.row) || 1,
      color: element?.color || null,
    }))
    .filter((element) => element.symbol && element.atomicNumber);
  const fallback = { symbol: "P", atomicNumber: 15, name: "Phosphorus", column: 15, row: 3, color: null };
  const current = elements.find((element) => element.symbol === payload?.current?.symbol)
    || elements.find((element) => element.symbol === currentSymbol)
    || elements.find((element) => element.symbol === "P")
    || fallback;
  return {
    title: String(payload?.title || "Periodic Table"),
    toggleLabel: String(payload?.toggleLabel || "Element"),
    pinLabel: String(payload?.pinLabel || "Pin"),
    current,
    columns: Number(payload?.columns || 18),
    rows: Number(payload?.rows || 9) + 1,
    elements: elements.length ? elements : [fallback],
  };
}

function safeJsonParse(text, fallback) {
  try {
    return JSON.parse(text);
  } catch {
    return fallback;
  }
}
