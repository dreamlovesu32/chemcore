export function createPlasmidMapDialogHost({ root = document.body } = {}) {
  return {
    choose(spec) {
      if (spec?.kind !== "plasmid-map" || !spec?.data || !spec.objectId) {
        return Promise.resolve(null);
      }
      return new PlasmidMapDialog({ root, spec }).open();
    },
  };
}

class PlasmidMapDialog {
  constructor({ root, spec }) {
    this.root = root;
    this.spec = spec;
    this.data = structuredClone(spec.data);
  }

  open() {
    document.querySelector(".plasmid-map-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog plasmid-map-dialog";
      this.render();
      this.root.appendChild(this.backdrop);
      this.bind();
      this.backdrop.querySelector('[name="numberBasePairs"]')?.focus?.({ preventScroll: true });
      this.backdrop.querySelector('[name="numberBasePairs"]')?.select?.();
    });
  }

  render() {
    this.backdrop.innerHTML = `
      <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
      <div class="numeric-dialog-backdrop" data-plasmid-close></div>
      <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.spec.title || "Plasmid Map")}">
        <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.spec.title || "Plasmid Map")}</div>
        <section class="plasmid-map-dialog-section plasmid-map-dialog-general">
          ${numberField("numberBasePairs", "Base pairs", this.data.numberBasePairs, 1, 1)}
          ${numberField("radius", "Ring radius", this.data.radius, 0.01, 0.25, "pt")}
          ${numberField("lineWidth", "Line width", this.data.lineWidth, 0, 0.05, "pt")}
          ${numberField("boldWidth", "Bold width", this.data.boldWidth, 0, 0.05, "pt")}
          ${numberField("marginWidth", "Margin width", this.data.marginWidth, 0, 0.05, "pt")}
          ${numberField("labelSize", "Label size", this.data.labelSize, 0.01, 0.25, "pt")}
          ${numberField("labelFont", "Font ID", this.data.labelFont, 0, 1)}
          ${numberField("labelFace", "Face flags", this.data.labelFace, 0, 1)}
          <label class="numeric-dialog-field">
            <span>Color</span><input name="color" type="color" value="${colorValue(this.data.color)}"><em></em>
          </label>
          <label class="plasmid-map-dialog-checkbox">
            <input name="showBasePairs" type="checkbox"${this.data.showBasePairs ? " checked" : ""}>
            Show base-pair count
          </label>
        </section>
        ${this.collectionSection("Regions", "region", this.data.regions || [])}
        ${this.collectionSection("Markers", "marker", this.data.markers || [])}
        <div class="numeric-dialog-error" data-plasmid-error role="alert"></div>
        <div class="numeric-dialog-actions">
          <button type="button" data-plasmid-close>Cancel</button>
          <button type="submit">OK</button>
        </div>
      </form>
    `;
  }

  collectionSection(title, kind, rows) {
    return `
      <section class="plasmid-map-dialog-section" data-plasmid-section="${kind}">
        <div class="plasmid-map-dialog-section-title">
          <strong>${title}</strong>
          <button type="button" data-plasmid-add="${kind}">Add ${kind}</button>
        </div>
        <div class="plasmid-map-dialog-table">
          ${rows.map((row) => kind === "region" ? regionRow(row) : markerRow(row)).join("")}
        </div>
      </section>
    `;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-plasmid-close]")) {
        this.close(null);
        return;
      }
      const remove = event.target.closest("[data-plasmid-remove]");
      if (remove) {
        remove.closest("[data-plasmid-row]")?.remove();
        return;
      }
      const add = event.target.closest("[data-plasmid-add]")?.dataset.plasmidAdd;
      if (add === "region") {
        const id = nextId(this.backdrop, "region");
        this.backdrop.querySelector('[data-plasmid-section="region"] .plasmid-map-dialog-table')
          ?.insertAdjacentHTML("beforeend", regionRow({
            id,
            start: 1,
            end: Number(this.backdrop.querySelector('[name="numberBasePairs"]')?.value) || 1,
            offset: 0,
            arrowAtStart: false,
            arrowAtEnd: false,
            filled: false,
            shaded: false,
            faded: false,
            width: 6,
            color: this.backdrop.querySelector('[name="color"]')?.value || "#000000",
            alpha: 1,
          }));
      } else if (add === "marker") {
        const id = nextId(this.backdrop, "marker");
        this.backdrop.querySelector('[data-plasmid-section="marker"] .plasmid-map-dialog-table')
          ?.insertAdjacentHTML("beforeend", markerRow({
            id,
            position: 1,
            label: "",
            offset: 48,
            labelAngle: null,
            color: this.backdrop.querySelector('[name="color"]')?.value || "#000000",
          }));
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close(null);
      }
    });
    this.backdrop.querySelector("form")?.addEventListener("submit", (event) => {
      event.preventDefault();
      this.submit();
    });
  }

  submit() {
    const value = this.read();
    const error = validate(value);
    if (error) {
      this.backdrop.querySelector("[data-plasmid-error]").textContent = error;
      return;
    }
    this.close(value);
  }

  read() {
    const numberBasePairs = numericValue(this.backdrop, "numberBasePairs");
    return {
      numberBasePairs,
      radius: numericValue(this.backdrop, "radius"),
      showBasePairs: !!this.backdrop.querySelector('[name="showBasePairs"]')?.checked,
      lineWidth: numericValue(this.backdrop, "lineWidth"),
      boldWidth: numericValue(this.backdrop, "boldWidth"),
      marginWidth: numericValue(this.backdrop, "marginWidth"),
      labelFont: numericValue(this.backdrop, "labelFont"),
      labelSize: numericValue(this.backdrop, "labelSize"),
      labelFace: numericValue(this.backdrop, "labelFace"),
      color: String(this.backdrop.querySelector('[name="color"]')?.value || ""),
      regions: Array.from(this.backdrop.querySelectorAll('[data-plasmid-row="region"]'))
        .map((row) => {
          const first = numericValue(row, "start");
          const second = numericValue(row, "end");
          return {
            id: textValue(row, "id"),
            start: Math.min(first, second),
            end: Math.max(first, second),
            offset: numericValue(row, "offset"),
            arrowAtStart: !!row.querySelector('[name="arrowAtStart"]')?.checked,
            arrowAtEnd: !!row.querySelector('[name="arrowAtEnd"]')?.checked,
            filled: row.querySelector('[name="fill"]')?.value === "filled",
            shaded: row.querySelector('[name="fill"]')?.value === "shaded",
            faded: row.querySelector('[name="fill"]')?.value === "faded",
            width: numericValue(row, "width"),
            color: textValue(row, "color"),
            alpha: numericValue(row, "alpha"),
          };
        }),
      markers: Array.from(this.backdrop.querySelectorAll('[data-plasmid-row="marker"]'))
        .map((row) => ({
          id: textValue(row, "id"),
          position: numericValue(row, "position"),
          label: textValue(row, "label"),
          offset: numericValue(row, "offset"),
          labelAngle: optionalNumericValue(row, "labelAngle"),
          color: textValue(row, "color"),
        })),
    };
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function regionRow(region) {
  const fill = region.filled ? "filled" : region.shaded ? "shaded" : region.faded ? "faded" : "none";
  return `
    <div class="plasmid-map-dialog-row plasmid-map-region-row" data-plasmid-row="region">
      ${textInput("id", "ID", region.id)}
      ${numberInput("start", "Start", region.start, 1)}
      ${numberInput("end", "End", region.end, 1)}
      ${numberInput("offset", "Offset", region.offset, 0.25)}
      ${numberInput("width", "Width", region.width, 0.25)}
      <label><span>Fill</span><select name="fill">
        ${["none", "filled", "shaded", "faded"].map((value) =>
          `<option value="${value}"${fill === value ? " selected" : ""}>${capitalize(value)}</option>`).join("")}
      </select></label>
      ${numberInput("alpha", "Alpha", region.alpha, 0.05)}
      <label><span>Color</span><input name="color" type="color" value="${colorValue(region.color)}"></label>
      <label class="plasmid-map-row-checkbox"><input name="arrowAtStart" type="checkbox"${region.arrowAtStart ? " checked" : ""}>Start arrow</label>
      <label class="plasmid-map-row-checkbox"><input name="arrowAtEnd" type="checkbox"${region.arrowAtEnd ? " checked" : ""}>End arrow</label>
      <button type="button" data-plasmid-remove aria-label="Remove region">Remove</button>
    </div>
  `;
}

function markerRow(marker) {
  return `
    <div class="plasmid-map-dialog-row plasmid-map-marker-row" data-plasmid-row="marker">
      ${textInput("id", "ID", marker.id)}
      ${numberInput("position", "Position", marker.position, 1)}
      ${textInput("label", "Label", marker.label)}
      ${numberInput("offset", "Offset", marker.offset, 0.25)}
      ${numberInput("labelAngle", "Label angle", marker.labelAngle ?? "", 0.25)}
      <label><span>Color</span><input name="color" type="color" value="${colorValue(marker.color)}"></label>
      <button type="button" data-plasmid-remove aria-label="Remove marker">Remove</button>
    </div>
  `;
}

function numberField(name, label, value, minimum, step, unit = "") {
  const inputStep = Number.isInteger(step) ? step : "any";
  return `
    <label class="numeric-dialog-field">
      <span>${label}</span><input name="${name}" type="number" min="${minimum}" step="${inputStep}" value="${escapeHtml(value)}"><em>${unit}</em>
    </label>
  `;
}

function textInput(name, label, value) {
  return `<label><span>${label}</span><input name="${name}" type="text" value="${escapeHtml(value)}"></label>`;
}

function numberInput(name, label, value, step) {
  const inputStep = Number.isInteger(step) ? step : "any";
  return `<label><span>${label}</span><input name="${name}" type="number" step="${inputStep}" value="${escapeHtml(value)}"></label>`;
}

function validate(data) {
  if (!Number.isInteger(data.numberBasePairs) || data.numberBasePairs < 1) {
    return "Base pairs must be a whole number greater than zero.";
  }
  if (!Number.isInteger(data.labelFont) || data.labelFont < 0
      || !Number.isInteger(data.labelFace) || data.labelFace < 0) {
    return "Font ID and face flags must be whole numbers zero or greater.";
  }
  if (![data.radius, data.lineWidth, data.boldWidth, data.marginWidth, data.labelSize]
    .every(Number.isFinite)
      || data.radius <= 0 || data.lineWidth < 0 || data.boldWidth < 0
      || data.marginWidth < 0 || data.labelSize <= 0) {
    return "Radius and label size must be positive; line widths cannot be negative.";
  }
  const ids = new Set();
  for (const region of data.regions) {
    if (!region.id || ids.has(region.id)) return "Region and marker IDs must be non-empty and unique.";
    ids.add(region.id);
    if (![region.start, region.end].every((value) =>
      Number.isInteger(value) && value >= 1 && value <= data.numberBasePairs)) {
      return `Region ${region.id} must stay within 1–${data.numberBasePairs}.`;
    }
    if (![region.offset, region.width, region.alpha].every(Number.isFinite)
        || region.width <= 0 || region.alpha < 0 || region.alpha > 1) {
      return `Region ${region.id} has invalid geometry or opacity.`;
    }
  }
  for (const marker of data.markers) {
    if (!marker.id || ids.has(marker.id)) return "Region and marker IDs must be non-empty and unique.";
    ids.add(marker.id);
    if (!Number.isInteger(marker.position)
        || marker.position < 1 || marker.position > data.numberBasePairs) {
      return `Marker ${marker.id} must stay within 1–${data.numberBasePairs}.`;
    }
    if (!Number.isFinite(marker.offset)
        || (marker.labelAngle !== null && !Number.isFinite(marker.labelAngle))) {
      return `Marker ${marker.id} has invalid label geometry.`;
    }
  }
  return "";
}

function nextId(root, prefix) {
  const used = new Set(Array.from(root.querySelectorAll('[data-plasmid-row] [name="id"]'))
    .map((input) => input.value));
  let index = 1;
  while (used.has(`${prefix}_${index}`)) index += 1;
  return `${prefix}_${index}`;
}

function numericValue(root, name) {
  return Number(root.querySelector(`[name="${name}"]`)?.value);
}

function optionalNumericValue(root, name) {
  const raw = String(root.querySelector(`[name="${name}"]`)?.value ?? "").trim();
  return raw === "" ? null : Number(raw);
}

function textValue(root, name) {
  return String(root.querySelector(`[name="${name}"]`)?.value ?? "").trim();
}

function colorValue(value) {
  return /^#[0-9a-f]{6}$/i.test(String(value || "")) ? String(value) : "#000000";
}

function capitalize(value) {
  return `${value[0].toUpperCase()}${value.slice(1)}`;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
