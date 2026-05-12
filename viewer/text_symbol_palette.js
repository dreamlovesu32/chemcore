export async function loadTextSymbolCatalog() {
  const candidates = [
    new URL("./shared/text_symbols.json", import.meta.url),
    new URL("./text_symbols.json", import.meta.url),
    new URL("../shared/text_symbols.json", import.meta.url),
    new URL("/shared/text_symbols.json", window.location.href),
  ];
  let lastStatus = "not attempted";
  for (const url of candidates) {
    const response = await fetch(url);
    if (response.ok) {
      return normalizeTextSymbolCatalog(await response.json());
    }
    lastStatus = `${response.status} ${url.href}`;
  }
  throw new Error(`Failed to load text symbol catalog: ${lastStatus}`);
}

export function createTextSymbolPalette({ mount, catalog, onSelect }) {
  if (!mount) {
    return null;
  }
  const root = document.createElement("div");
  root.className = "text-symbol-palette";

  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "text-symbol-toggle";
  toggle.title = "Text symbols";
  toggle.setAttribute("aria-label", "Text symbols");
  toggle.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><text x="12" y="16.5" text-anchor="middle" font-size="15" font-family="Arial, Helvetica, sans-serif">Ω</text></svg>`;

  const panel = document.createElement("div");
  panel.className = "text-symbol-panel";

  const header = document.createElement("div");
  header.className = "text-symbol-header";

  const title = document.createElement("div");
  title.className = "text-symbol-title";
  title.textContent = "Symbol";

  const pin = document.createElement("button");
  pin.type = "button";
  pin.className = "text-symbol-pin";
  pin.title = "Pin";
  pin.setAttribute("aria-label", "Pin text symbols");
  pin.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4v1H6v-1l4-4z"/><path d="M12 15v5"/></svg>`;

  header.append(title, pin);
  panel.appendChild(header);

  const content = document.createElement("div");
  content.className = "text-symbol-content";
  for (const group of catalog.groups) {
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
        if (!root.classList.contains("is-pinned")) {
          setOpen(false);
        }
      });
      grid.appendChild(button);
    }
    section.append(label, grid);
    content.appendChild(section);
  }
  panel.appendChild(content);

  root.append(toggle, panel);
  mount.appendChild(root);

  function setOpen(open) {
    root.classList.toggle("is-open", open);
    toggle.setAttribute("aria-expanded", open ? "true" : "false");
  }

  toggle.addEventListener("click", (event) => {
    event.preventDefault();
    setOpen(!root.classList.contains("is-open"));
  });
  pin.addEventListener("click", (event) => {
    event.preventDefault();
    const pinned = !root.classList.contains("is-pinned");
    root.classList.toggle("is-pinned", pinned);
    pin.classList.toggle("is-selected", pinned);
    setOpen(true);
  });
  root.addEventListener("mousedown", (event) => {
    event.preventDefault();
  });

  setOpen(false);
  return {
    root,
    setOpen,
  };
}

function normalizeTextSymbolCatalog(manifest) {
  return {
    version: Number(manifest?.version || 1),
    groups: (manifest?.groups || [])
      .map((group) => ({
        id: String(group?.id || ""),
        label: String(group?.label || group?.id || "Symbols"),
        characters: Array.from(String(group?.characters || "")),
      }))
      .filter((group) => group.id && group.characters.length),
  };
}
