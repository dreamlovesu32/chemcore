export function createTableDialogHost({
  root = document.body,
  commandEngine,
  engine,
  onApply,
} = {}) {
  return {
    chooseInsert(spec) {
      if (spec?.kind !== "insert-table" || !Array.isArray(spec.fields)) {
        return Promise.resolve(null);
      }
      return new TableInsertDialog({ root, spec }).open();
    },
    chooseBorders(spec) {
      if (spec?.kind !== "table-borders") {
        return Promise.resolve(false);
      }
      return new TableBordersDialog({
        root,
        spec,
        apply: async (value) => {
          const command = {
            type: "set-table-borders",
            objectId: spec.objectId,
            row: spec.row,
            column: spec.column,
            ...value,
          };
          const target = engine?.();
          const result = await commandEngine.executeEngineCommand(
            command,
            () => target?.executeCommandJson?.(JSON.stringify(command)),
          );
          if (result.changed) {
            await onApply?.(result);
          }
          return !!result.changed;
        },
      }).open();
    },
  };
}

class TableInsertDialog {
  constructor({ root, spec }) {
    this.root = root;
    this.spec = spec;
  }

  open() {
    document.querySelector(".table-insert-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      const fields = Object.fromEntries(this.spec.fields.map((field) => [field.key, field]));
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog table-insert-dialog";
      this.backdrop.innerHTML = `
        <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
        <div class="numeric-dialog-backdrop" data-table-dialog-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.spec.title || "Insert Table")}">
          <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.spec.title || "Insert Table")}</div>
          ${integerField(fields.rows)}
          ${integerField(fields.columns)}
          <div class="numeric-dialog-error" data-table-dialog-error role="alert"></div>
          <div class="numeric-dialog-actions">
            <button type="button" data-table-dialog-close>Cancel</button>
            <button type="submit">OK</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.backdrop.addEventListener("click", (event) => {
        if (event.target.closest("[data-table-dialog-close]")) {
          this.close(null);
        }
      });
      this.backdrop.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
          this.close(null);
        }
      });
      this.backdrop.querySelector("form")?.addEventListener("submit", (event) => {
        event.preventDefault();
        this.submit(fields);
      });
      this.backdrop.querySelector('input[name="rows"]')?.focus?.({ preventScroll: true });
      this.backdrop.querySelector('input[name="rows"]')?.select?.();
    });
  }

  submit(fields) {
    const rows = integerValue(this.backdrop, "rows");
    const columns = integerValue(this.backdrop, "columns");
    const valid = (value, field) => Number.isInteger(value)
      && value >= Number(field.minimum)
      && value <= Number(field.maximum);
    if (!valid(rows, fields.rows) || !valid(columns, fields.columns)) {
      this.backdrop.querySelector("[data-table-dialog-error]").textContent =
        "Rows and columns must be whole numbers from 1 to 100.";
      return;
    }
    this.close({ rows, columns });
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

class TableBordersDialog {
  constructor({ root, spec, apply }) {
    this.root = root;
    this.spec = spec;
    this.apply = apply;
  }

  open() {
    document.querySelector(".table-borders-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      const sides = new Set(this.spec.sides || []);
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog table-borders-dialog";
      this.backdrop.innerHTML = `
        <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
        <div class="numeric-dialog-backdrop" data-table-borders-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.spec.title || "Table Borders")}">
          <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.spec.title || "Table Borders")}</div>
          <fieldset class="table-border-settings">
            <legend>Setting</legend>
            ${["none", "box", "all", "custom"].map((setting) => `
              <label><input type="radio" name="setting" value="${setting}"${this.spec.setting === setting ? " checked" : ""}>${capitalize(setting)}</label>
            `).join("")}
          </fieldset>
          <fieldset class="table-border-settings">
            <legend>Edges</legend>
            ${["top", "left", "bottom", "right"].map((side) => `
              <label><input type="checkbox" name="side" value="${side}"${sides.has(side) ? " checked" : ""}>${capitalize(side)}</label>
            `).join("")}
          </fieldset>
          <label class="numeric-dialog-field">
            <span>Style</span>
            <select name="lineStyle">
              <option value="solid"${this.spec.lineStyle === "solid" ? " selected" : ""}>Solid</option>
              <option value="dashed"${this.spec.lineStyle === "dashed" ? " selected" : ""}>Dashed</option>
            </select>
            <em></em>
          </label>
          <label class="numeric-dialog-field">
            <span>Color</span>
            <input name="color" type="color" value="${escapeHtml(this.spec.color || "#000000")}">
            <em></em>
          </label>
          <label class="numeric-dialog-field">
            <span>Width</span>
            <input name="width" type="number" min="0" step="0.05" value="${Number(this.spec.width) || 0.75}">
            <em>pt</em>
          </label>
          <div class="numeric-dialog-error" data-table-borders-error role="alert"></div>
          <div class="numeric-dialog-actions">
            <button type="button" data-table-borders-close>Cancel</button>
            <button type="submit">OK</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.bind();
    });
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-table-borders-close]")) {
        this.close(false);
      }
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close(false);
      }
    });
    this.backdrop.querySelectorAll('input[name="setting"]').forEach((input) => {
      input.addEventListener("change", () => {
        const setting = this.backdrop.querySelector('input[name="setting"]:checked')?.value;
        const checked = setting === "none" ? [] : ["top", "left", "bottom", "right"];
        if (setting !== "custom") {
          this.backdrop.querySelectorAll('input[name="side"]').forEach((side) => {
            side.checked = checked.includes(side.value);
          });
        }
      });
    });
    this.backdrop.querySelectorAll('input[name="side"]').forEach((input) => {
      input.addEventListener("change", () => {
        const custom = this.backdrop.querySelector('input[name="setting"][value="custom"]');
        if (custom) {
          custom.checked = true;
        }
      });
    });
    this.backdrop.querySelector("form")?.addEventListener("submit", (event) => {
      event.preventDefault();
      void this.submit();
    });
  }

  async submit() {
    const width = Number(this.backdrop.querySelector('[name="width"]')?.value);
    const color = String(this.backdrop.querySelector('[name="color"]')?.value || "");
    if (!Number.isFinite(width) || width < 0 || !/^#[0-9a-f]{6}$/i.test(color)) {
      this.backdrop.querySelector("[data-table-borders-error]").textContent =
        "Width must be zero or greater and color must be valid.";
      return;
    }
    const sides = Array.from(this.backdrop.querySelectorAll('input[name="side"]:checked'))
      .map((input) => input.value);
    const lineStyle = this.backdrop.querySelector('[name="lineStyle"]')?.value || "solid";
    const changed = await this.apply({ sides, lineStyle, width, color });
    this.close(changed);
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function integerField(field = {}) {
  return `
    <label class="numeric-dialog-field">
      <span>${escapeHtml(field.label || field.key)}</span>
      <input name="${escapeHtml(field.key)}" type="number" inputmode="numeric"
        min="${Number(field.minimum) || 1}" max="${Number(field.maximum) || 100}"
        step="1" value="${Number(field.value) || 2}">
      <em></em>
    </label>
  `;
}

function integerValue(root, name) {
  return Number.parseInt(String(root.querySelector(`[name="${name}"]`)?.value || ""), 10);
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function capitalize(value) {
  const text = String(value || "");
  return text ? `${text[0].toUpperCase()}${text.slice(1)}` : "";
}
