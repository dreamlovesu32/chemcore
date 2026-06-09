class ChemcoreColorHost {
  constructor({ root = document.body, getPalette } = {}) {
    this.kind = "chemcore";
    this.root = root;
    this.getPalette = getPalette;
  }

  async chooseColor(initialColor, customColors = []) {
    const existing = document.querySelector(".color-dialog-backdrop");
    existing?.querySelector(".color-dialog-close")?.click();
    const palette = await this.palette(initialColor, customColors);
    return new Promise((resolve) => {
      const dialog = new ChemcoreColorDialog({
        root: this.root,
        palette,
        resolve,
      });
      dialog.open();
    });
  }

  async palette(initialColor, customColors) {
    const fallback = fallbackColorDialogPalette(initialColor, customColors);
    if (typeof this.getPalette !== "function") {
      return fallback;
    }
    try {
      return normalizeColorDialogPalette(await this.getPalette(initialColor, customColors), fallback);
    } catch (error) {
      console.warn("[chemcore] failed to load engine color palette", error);
      return fallback;
    }
  }
}

class ChemcoreColorDialog {
  constructor({ root, palette, resolve }) {
    this.root = root;
    this.resolve = resolve;
    this.palette = normalizeColorDialogPalette(palette);
    this.labels = this.palette.labels;
    this.selected = normalizeHexColor(this.palette.selected) || "#000000";
    this.hsv = rgbToHsv(hexToRgb(this.selected));
    this.customColors = this.palette.customColors;
    this.backdrop = null;
  }

  open() {
    this.backdrop = document.createElement("div");
    this.backdrop.className = "color-dialog-backdrop";
    this.backdrop.innerHTML = this.html();
    this.root.appendChild(this.backdrop);
    this.bind();
    this.sync(this.selected, this.hsv);
    this.backdrop.tabIndex = -1;
    this.backdrop.focus();
  }

  html() {
    return `
      <div class="color-dialog" role="dialog" aria-modal="true" aria-label="${escapeHtml(this.palette.title)}">
        <div class="color-dialog-titlebar">
          <span>${escapeHtml(this.palette.title)}</span>
          <button class="color-dialog-close" type="button" aria-label="${escapeHtml(this.labels.close)}">x</button>
        </div>
        <div class="color-dialog-body">
          <section class="color-dialog-palette">
            <p class="color-dialog-label">${escapeHtml(this.labels.basic)}</p>
            <div class="color-dialog-basic-grid">
              ${this.palette.basicColors.map((color) => colorChipHtml(color, this.selected)).join("")}
            </div>
            <div class="color-dialog-custom">
              <p class="color-dialog-label">${escapeHtml(this.labels.custom)}</p>
              <div class="color-dialog-custom-grid"></div>
            </div>
          </section>
          <section class="color-dialog-main">
            <div class="color-dialog-picker">
              <div class="color-dialog-spectrum" data-color-spectrum aria-label="Color spectrum" role="slider" tabindex="0">
                <span class="color-dialog-spectrum-cursor"></span>
              </div>
              <div class="color-dialog-value-slider" data-color-value-slider aria-label="Brightness" role="slider" tabindex="0">
                <span class="color-dialog-value-cursor"></span>
              </div>
            </div>
            <div class="color-dialog-bottom">
              <div class="color-dialog-preview-block">
                <div class="color-dialog-preview"></div>
                <span>${escapeHtml(this.labels.preview)}</span>
              </div>
              <div class="color-dialog-fields">
                ${this.palette.fields.map((field) => colorFieldHtml(field)).join("")}
              </div>
            </div>
            <div class="color-dialog-add-row">
              <button type="button" data-color-dialog-add-custom>${escapeHtml(this.labels.addCustom)}</button>
            </div>
            <div class="color-dialog-actions">
              <button type="button" data-color-dialog-ok>${escapeHtml(this.labels.ok)}</button>
              <button type="button" data-color-dialog-cancel>${escapeHtml(this.labels.cancel)}</button>
            </div>
          </section>
        </div>
      </div>
    `;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target === this.backdrop || event.target.closest("[data-color-dialog-cancel]") || event.target.closest(".color-dialog-close")) {
        this.close(null);
        return;
      }
      const chip = event.target.closest("[data-color-dialog-value]");
      if (chip) {
        this.sync(chip.dataset.colorDialogValue);
        return;
      }
      if (event.target.closest("[data-color-dialog-add-custom]")) {
        this.customColors = [
          this.selected,
          ...this.customColors.filter((color) => color !== this.selected),
        ].slice(0, 16);
        this.renderCustomColors();
        this.sync(this.selected, this.hsv);
        return;
      }
      if (event.target.closest("[data-color-dialog-ok]")) {
        this.close(this.selected);
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close(null);
      }
    });
    this.bindSpectrumDrag();
    this.bindFields();
  }

  bindSpectrumDrag() {
    const spectrum = this.backdrop.querySelector("[data-color-spectrum]");
    const valueSlider = this.backdrop.querySelector("[data-color-value-slider]");
    const bindDrag = (target, update) => {
      target.addEventListener("pointerdown", (event) => {
        event.preventDefault();
        target.setPointerCapture?.(event.pointerId);
        update(event);
        const move = (moveEvent) => update(moveEvent);
        const up = () => {
          window.removeEventListener("pointermove", move);
          window.removeEventListener("pointerup", up);
        };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", up, { once: true });
      });
    };
    bindDrag(spectrum, (event) => {
      const rect = spectrum.getBoundingClientRect();
      this.syncFromHsv({
        h: ((event.clientX - rect.left) / rect.width) * 359,
        s: (1 - ((event.clientY - rect.top) / rect.height)) * 100,
      });
    });
    bindDrag(valueSlider, (event) => {
      const rect = valueSlider.getBoundingClientRect();
      this.syncFromHsv({ v: (1 - ((event.clientY - rect.top) / rect.height)) * 100 });
    });
  }

  bindFields() {
    const hexInput = this.backdrop.querySelector('[data-color-field="hex"]');
    const rgbInputs = Array.from(this.backdrop.querySelectorAll("[data-rgb-field]"));
    const hsvInputs = Array.from(this.backdrop.querySelectorAll("[data-hsv-field]"));
    hexInput?.addEventListener("change", () => this.sync(hexInput.value));
    for (const input of rgbInputs) {
      input.addEventListener("change", () => {
        const values = Object.fromEntries(rgbInputs.map((field) => [
          field.dataset.rgbField,
          clampRgb(field.value),
        ]));
        this.sync(rgbToHex(values.r, values.g, values.b));
      });
    }
    for (const input of hsvInputs) {
      input.addEventListener("change", () => {
        const values = Object.fromEntries(hsvInputs.map((field) => [
          field.dataset.hsvField,
          Number.parseInt(String(field.value || 0), 10) || 0,
        ]));
        this.syncFromHsv({ h: values.h, s: values.s, v: values.v });
      });
    }
  }

  sync(color, nextHsv = null) {
    this.selected = normalizeHexColor(color) || this.selected;
    this.hsv = nextHsv ? {
      h: clampHue(nextHsv.h),
      s: clampPercent(nextHsv.s),
      v: clampPercent(nextHsv.v),
    } : rgbToHsv(hexToRgb(this.selected));
    this.backdrop.style.setProperty("--dialog-hue-position", `${(this.hsv.h / 359) * 100}%`);
    this.backdrop.style.setProperty("--dialog-saturation", `${this.hsv.s}%`);
    this.backdrop.style.setProperty("--dialog-value", `${this.hsv.v}%`);
    this.backdrop.querySelector(".color-dialog-preview")?.style.setProperty("--swatch", this.selected);
    const hexInput = this.backdrop.querySelector('[data-color-field="hex"]');
    if (hexInput) {
      hexInput.value = this.selected.toUpperCase();
    }
    const { r, g, b } = hexToRgb(this.selected);
    for (const input of this.backdrop.querySelectorAll("[data-rgb-field]")) {
      input.value = String({ r, g, b }[input.dataset.rgbField]);
    }
    for (const input of this.backdrop.querySelectorAll("[data-hsv-field]")) {
      input.value = String({
        h: Math.round(this.hsv.h),
        s: Math.round(this.hsv.s),
        v: Math.round(this.hsv.v),
      }[input.dataset.hsvField]);
    }
    this.backdrop.querySelectorAll(".color-dialog-chip").forEach((chip) => {
      chip.classList.toggle("is-selected", normalizeHexColor(chip.dataset.colorDialogValue) === this.selected);
    });
    this.renderCustomColors();
  }

  syncFromHsv(next) {
    this.hsv = {
      h: clampHue(next.h ?? this.hsv.h),
      s: clampPercent(next.s ?? this.hsv.s),
      v: clampPercent(next.v ?? this.hsv.v),
    };
    const { r, g, b } = hsvToRgb(this.hsv);
    this.sync(rgbToHex(r, g, b), this.hsv);
  }

  renderCustomColors() {
    const customGrid = this.backdrop.querySelector(".color-dialog-custom-grid");
    if (!customGrid) {
      return;
    }
    customGrid.innerHTML = this.customColors
      .map((color) => colorChipHtml(color, this.selected))
      .join("");
  }

  close(color) {
    this.backdrop?.remove();
    this.resolve(color);
  }
}

export function createColorHost(options = {}) {
  return new ChemcoreColorHost(options);
}

function colorChipHtml(color, selected) {
  return `<button class="color-dialog-chip${normalizeHexColor(color) === normalizeHexColor(selected) ? " is-selected" : ""}" type="button" data-color-dialog-value="${color}" style="--swatch:${color}" aria-label="${color}"></button>`;
}

function normalizeColorDialogPalette(payload, fallback = fallbackColorDialogPalette("#000000", [])) {
  const parsed = typeof payload === "string" ? safeJsonParse(payload, null) : payload;
  const labels = {
    basic: "Basic colors:",
    custom: "Custom colors:",
    preview: "Color | Solid",
    addCustom: "Add to custom colors",
    ok: "OK",
    cancel: "Cancel",
    close: "Close",
    ...(parsed?.labels || {}),
  };
  return {
    title: String(parsed?.title || fallback.title || "Color"),
    selected: normalizeHexColor(parsed?.selected) || normalizeHexColor(fallback.selected) || "#000000",
    labels,
    fields: normalizeColorFields(parsed?.fields || fallback.fields),
    basicColors: normalizeColorList(parsed?.basicColors || fallback.basicColors),
    customColors: normalizeColorList(parsed?.customColors || fallback.customColors).slice(0, 16),
  };
}

function fallbackColorDialogPalette(initialColor, customColors = []) {
  return {
    title: "Color",
    selected: normalizeHexColor(initialColor) || "#000000",
    labels: {},
    fields: [
      { kind: "hsv", key: "h", label: "Hue", min: 0, max: 359 },
      { kind: "rgb", key: "r", label: "Red", min: 0, max: 255 },
      { kind: "hsv", key: "s", label: "Saturation", min: 0, max: 100 },
      { kind: "rgb", key: "g", label: "Green", min: 0, max: 255 },
      { kind: "hsv", key: "v", label: "Brightness", min: 0, max: 100 },
      { kind: "rgb", key: "b", label: "Blue", min: 0, max: 255 },
      { kind: "hex", key: "hex", label: "Hex" },
    ],
    basicColors: ["#000000", "#ff0000", "#ffff00", "#00ff00", "#ffffff", "#00ffff", "#0000ff", "#ff00ff"],
    customColors,
  };
}

function normalizeColorFields(fields) {
  return (Array.isArray(fields) ? fields : [])
    .map((field) => ({
      kind: String(field?.kind || ""),
      key: String(field?.key || ""),
      label: String(field?.label || field?.key || ""),
      min: Number.isFinite(Number(field?.min)) ? Number(field.min) : null,
      max: Number.isFinite(Number(field?.max)) ? Number(field.max) : null,
    }))
    .filter((field) => field.key && field.label);
}

function normalizeColorList(colors) {
  const out = [];
  for (const color of colors || []) {
    const normalized = normalizeHexColor(color);
    if (normalized && !out.includes(normalized)) {
      out.push(normalized);
    }
  }
  return out;
}

function colorFieldHtml(field) {
  const attr = field.kind === "hsv"
    ? `data-hsv-field="${escapeHtml(field.key)}"`
    : field.kind === "rgb"
      ? `data-rgb-field="${escapeHtml(field.key)}"`
      : `data-color-field="${escapeHtml(field.key)}"`;
  const type = field.kind === "hex" ? "text" : "number";
  const min = field.min == null ? "" : ` min="${field.min}"`;
  const max = field.max == null ? "" : ` max="${field.max}"`;
  const extraClass = field.kind === "hex" ? " color-dialog-hex-field" : "";
  return `<label class="color-dialog-field${extraClass}"><span>${escapeHtml(field.label)}:</span><input ${attr} type="${type}"${min}${max}></label>`;
}

function normalizeHexColor(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(raw)) {
    return raw;
  }
  if (/^#[0-9a-f]{3}$/.test(raw)) {
    return `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`;
  }
  const match = raw.match(/^rgb\((\d+),\s*(\d+),\s*(\d+)\)$/);
  if (match) {
    return rgbToHex(match[1], match[2], match[3]);
  }
  return null;
}

function hexToRgb(color) {
  const hex = normalizeHexColor(color) || "#000000";
  return {
    r: Number.parseInt(hex.slice(1, 3), 16),
    g: Number.parseInt(hex.slice(3, 5), 16),
    b: Number.parseInt(hex.slice(5, 7), 16),
  };
}

function rgbToHex(r, g, b) {
  return `#${[r, g, b].map((value) => clampRgb(value).toString(16).padStart(2, "0")).join("")}`;
}

function clampRgb(value) {
  return Math.max(0, Math.min(255, Number.parseInt(String(value || 0), 10) || 0));
}

function rgbToHsv({ r, g, b }) {
  const red = clampRgb(r) / 255;
  const green = clampRgb(g) / 255;
  const blue = clampRgb(b) / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  let h = 0;
  if (delta !== 0) {
    if (max === red) {
      h = 60 * (((green - blue) / delta) % 6);
    } else if (max === green) {
      h = 60 * ((blue - red) / delta + 2);
    } else {
      h = 60 * ((red - green) / delta + 4);
    }
  }
  if (h < 0) {
    h += 360;
  }
  return {
    h,
    s: max === 0 ? 0 : (delta / max) * 100,
    v: max * 100,
  };
}

function hsvToRgb({ h, s, v }) {
  const hue = clampHue(h);
  const saturation = clampPercent(s) / 100;
  const value = clampPercent(v) / 100;
  const chroma = value * saturation;
  const x = chroma * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = value - chroma;
  let red = 0;
  let green = 0;
  let blue = 0;
  if (hue < 60) {
    [red, green, blue] = [chroma, x, 0];
  } else if (hue < 120) {
    [red, green, blue] = [x, chroma, 0];
  } else if (hue < 180) {
    [red, green, blue] = [0, chroma, x];
  } else if (hue < 240) {
    [red, green, blue] = [0, x, chroma];
  } else if (hue < 300) {
    [red, green, blue] = [x, 0, chroma];
  } else {
    [red, green, blue] = [chroma, 0, x];
  }
  return {
    r: Math.round((red + m) * 255),
    g: Math.round((green + m) * 255),
    b: Math.round((blue + m) * 255),
  };
}

function clampHue(value) {
  const hue = Number.parseFloat(String(value || 0));
  return ((Number.isFinite(hue) ? hue : 0) % 360 + 360) % 360;
}

function clampPercent(value) {
  const percent = Number.parseFloat(String(value || 0));
  return Math.max(0, Math.min(100, Number.isFinite(percent) ? percent : 0));
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
