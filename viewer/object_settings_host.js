export function createObjectSettingsHost({ root = document.body, engine, commandEngine, onApply } = {}) {
  return {
    async chooseObjectSettings() {
      const targetEngine = engine?.();
      if (!targetEngine?.objectSettingsDialogJson || !targetEngine?.applyObjectSettingsDialogJson) {
        return false;
      }
      const payload = JSON.parse(await targetEngine.objectSettingsDialogJson());
      const dialog = new ObjectSettingsDialog({
        root,
        payload,
        apply: async (settings) => {
          const settingsJson = JSON.stringify(settings);
          const result = commandEngine?.executeEngineCommand
            ? await commandEngine.executeEngineCommand(
              {
                type: "apply-object-settings",
                payload: { settings },
              },
              () => targetEngine.applyObjectSettingsDialogJson(settingsJson),
            )
            : { changed: !!(await targetEngine.applyObjectSettingsDialogJson(settingsJson)) };
          const changed = !!result.changed;
          if (changed) {
            await onApply?.();
          }
          return changed;
        },
      });
      return dialog.open();
    },
  };
}

class ObjectSettingsDialog {
  constructor({ root, payload, apply }) {
    this.root = root;
    this.payload = normalizePayload(payload);
    this.apply = apply;
    this.unit = this.payload.unit || "cm";
    this.backdrop = null;
  }

  open() {
    document.querySelector(".object-settings-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "object-settings-dialog";
      this.backdrop.innerHTML = this.html();
      this.root.appendChild(this.backdrop);
      this.bind();
      this.populate();
      this.backdrop.querySelector("[name='bondLength']")?.focus?.({ preventScroll: true });
      this.backdrop.querySelector("[name='bondLength']")?.select?.();
    });
  }

  html() {
    return `
      <div class="object-settings-backdrop" data-object-settings-close="cancel"></div>
      <form class="object-settings-panel" aria-label="Object settings">
        <div class="object-settings-titlebar">
          <div class="object-settings-title">Object Settings</div>
          <label class="object-settings-unit">
            <span>Unit</span>
            <select name="unit">
              ${this.payload.units.map((unit) => `<option value="${escapeHtml(unit)}">${escapeHtml(unit)}</option>`).join("")}
            </select>
          </label>
        </div>
        <div class="object-settings-grid">
          ${this.payload.fields.map((field) => this.fieldHtml(field)).join("")}
        </div>
        <div class="object-settings-actions">
          <button type="button" data-object-settings-close="cancel">Cancel</button>
          <button type="submit">Apply</button>
        </div>
      </form>
    `;
  }

  fieldHtml(field) {
    return `
      <label>
        <span>${escapeHtml(field.label)}</span>
        <input name="${escapeHtml(field.key)}" type="text" inputmode="decimal">
        <em data-object-setting-unit-for="${escapeHtml(field.key)}">${escapeHtml(unitForField(field, this.unit))}</em>
      </label>
    `;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-object-settings-close]")) {
        this.close(false);
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close(false);
      }
    });
    this.backdrop.querySelector("form").addEventListener("submit", (event) => {
      event.preventDefault();
      void this.submit();
    });
    this.backdrop.querySelector("[name='unit']").addEventListener("change", (event) => {
      this.unit = event.target.value === "pt" ? "pt" : "cm";
      this.populate();
    });
  }

  populate() {
    this.backdrop.querySelector("[name='unit']").value = this.unit;
    for (const field of this.payload.fields) {
      const input = this.backdrop.querySelector(`[name='${cssEscape(field.key)}']`);
      const unitLabel = this.backdrop.querySelector(`[data-object-setting-unit-for='${cssEscape(field.key)}']`);
      if (input) {
        input.value = formatNumber(displayValueForField(field, this.unit));
        input.classList.remove("is-invalid");
      }
      if (unitLabel) {
        unitLabel.textContent = unitForField(field, this.unit);
      }
    }
  }

  values() {
    const values = {};
    for (const field of this.payload.fields) {
      const input = this.backdrop.querySelector(`[name='${cssEscape(field.key)}']`);
      const rawValue = String(input?.value ?? "").trim();
      input.classList.remove("is-invalid");
      if (!rawValue) {
        continue;
      }
      const value = Number(rawValue);
      values[field.key] = value;
    }
    return values;
  }

  async submit() {
    const values = this.values();
    if (!values) {
      return;
    }
    try {
      const changed = await this.apply({ unit: this.unit, values });
      this.close(changed);
    } catch {
      const first = this.backdrop.querySelector("input");
      first?.classList.add("is-invalid");
      first?.focus?.();
    }
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function normalizePayload(payload) {
  return {
    unit: payload?.unit === "pt" ? "pt" : "cm",
    units: Array.isArray(payload?.units) && payload.units.length ? payload.units : ["cm", "pt"],
    fields: Array.isArray(payload?.fields) ? payload.fields : [],
  };
}

function unitForField(field, unit) {
  return field.unit === "%" ? "%" : unit;
}

function displayValueForField(field, unit) {
  if (field.mixed) {
    return null;
  }
  if (field.unit === "%") {
    return valueOrNull(field.value ?? field.values?.cm);
  }
  return valueOrNull(field.values?.[unit] ?? field.value);
}

function formatNumber(value) {
  return Number.isFinite(value) ? String(Math.round(value * 1000) / 1000) : "";
}

function valueOrNull(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function cssEscape(value) {
  return globalThis.CSS?.escape ? globalThis.CSS.escape(String(value)) : String(value).replace(/'/g, "\\'");
}
