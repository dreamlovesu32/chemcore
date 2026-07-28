export function createLogicalObjectsDialogHost({
  root = document.body,
  engine,
  commandEngine,
  onApply,
  notify,
} = {}) {
  return {
    async open() {
      const targetEngine = engine?.();
      if (!targetEngine?.logicalObjectsDialogJson || !commandEngine?.executeEngineCommand) {
        return false;
      }
      const dialog = new LogicalObjectsDialog({
        root,
        engine,
        commandEngine,
        onApply,
        notify,
      });
      return dialog.open();
    },
  };
}

class LogicalObjectsDialog {
  constructor({ root, engine, commandEngine, onApply, notify }) {
    this.root = root;
    this.engine = engine;
    this.commandEngine = commandEngine;
    this.onApply = onApply;
    this.notify = notify;
    this.familyKind = null;
    this.itemId = null;
    this.draft = null;
  }

  async open() {
    document.querySelector(".logical-objects-dialog")?.remove();
    this.backdrop = document.createElement("div");
    this.backdrop.className = "numeric-dialog logical-objects-dialog";
    this.root.appendChild(this.backdrop);
    await this.refresh();
    return true;
  }

  async refresh({ keepDraft = false } = {}) {
    const raw = await this.engine()?.logicalObjectsDialogJson?.();
    this.payload = JSON.parse(raw || "null");
    if (this.payload?.kind !== "logical-objects") {
      this.close();
      return;
    }
    const families = this.payload.families || [];
    if (!families.some((family) => family.kind === this.familyKind)) {
      this.familyKind = families[0]?.kind || null;
    }
    const family = this.currentFamily();
    if (!keepDraft) {
      const item = family?.items?.find((candidate) => candidate.id === this.itemId);
      if (item) {
        this.draft = structuredClone(item);
      } else {
        this.itemId = family?.items?.[0]?.id || null;
        this.draft = this.itemId
          ? structuredClone(family.items.find((candidate) => candidate.id === this.itemId))
          : null;
      }
    }
    this.render();
  }

  currentFamily() {
    return (this.payload?.families || []).find((family) => family.kind === this.familyKind);
  }

  render() {
    const family = this.currentFamily();
    this.backdrop.innerHTML = `
      <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
      <div class="numeric-dialog-backdrop" data-logical-close></div>
      <section class="logical-objects-panel" role="dialog" aria-modal="true" aria-label="${escapeHtml(this.payload.title || "Logical Objects")}">
        <header class="logical-objects-titlebar" data-desktop-window-drag-region>
          <strong>${escapeHtml(this.payload.title || "Logical Objects")}</strong>
          <button type="button" data-logical-close aria-label="Close">×</button>
        </header>
        <nav class="logical-family-tabs" aria-label="Logical object type">
          ${(this.payload.families || []).map((candidate) => `
            <button type="button" data-logical-family="${escapeHtml(candidate.kind)}"
              class="${candidate.kind === this.familyKind ? "is-active" : ""}">
              ${escapeHtml(candidate.label)} <span>${candidate.items?.length || 0}</span>
            </button>
          `).join("")}
        </nav>
        <div class="logical-objects-body">
          <aside class="logical-object-list">
            <div class="logical-object-list-actions">
              <button type="button" data-logical-new>New</button>
              <button type="button" data-logical-up ${this.itemId ? "" : "disabled"}>↑</button>
              <button type="button" data-logical-down ${this.itemId ? "" : "disabled"}>↓</button>
            </div>
            <div class="logical-object-list-items">
              ${(family?.items || []).map((item, index) => `
                <button type="button" data-logical-item="${escapeHtml(item.id)}"
                  class="${item.id === this.itemId ? "is-active" : ""}">
                  <span>${escapeHtml(itemSummary(family, item))}</span>
                  <small>${index + 1}</small>
                </button>
              `).join("") || `<p>No ${escapeHtml((family?.label || "objects").toLowerCase())}.</p>`}
            </div>
          </aside>
          <form class="logical-object-editor">
            ${this.draft
              ? (family?.fields || []).map((field) => renderField(field, this.draft)).join("")
              : `<div class="logical-object-empty">Choose an item or create a new one.</div>`}
            <div class="logical-object-actions">
              <button type="button" data-logical-delete ${this.itemId ? "" : "disabled"}>Delete</button>
              <span></span>
              <button type="button" data-logical-close>Close</button>
              <button type="submit" ${this.draft ? "" : "disabled"}>Apply</button>
            </div>
          </form>
        </div>
      </section>
    `;
    this.bind();
  }

  bind() {
    this.backdrop.querySelectorAll("[data-logical-close]").forEach((button) => {
      button.addEventListener("click", () => this.close());
    });
    this.backdrop.querySelector(".numeric-dialog-backdrop")?.addEventListener("click", () => this.close());
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        this.close();
      }
    });
    this.backdrop.querySelectorAll("[data-logical-family]").forEach((button) => {
      button.addEventListener("click", () => {
        this.familyKind = button.dataset.logicalFamily;
        this.itemId = null;
        this.draft = null;
        this.refresh();
      });
    });
    this.backdrop.querySelectorAll("[data-logical-item]").forEach((button) => {
      button.addEventListener("click", () => {
        this.itemId = button.dataset.logicalItem;
        const family = this.currentFamily();
        this.draft = structuredClone(
          family.items.find((item) => item.id === this.itemId),
        );
        this.render();
      });
    });
    this.backdrop.querySelector("[data-logical-new]")?.addEventListener("click", () => {
      this.itemId = null;
      this.draft = structuredClone(this.currentFamily()?.defaultValue || { id: "" });
      this.render();
      this.backdrop.querySelector("input:not([readonly]), textarea, select")?.focus?.();
    });
    this.backdrop.querySelector("[data-logical-delete]")?.addEventListener("click", () => {
      void this.deleteCurrent();
    });
    this.backdrop.querySelector("[data-logical-up]")?.addEventListener("click", () => {
      void this.moveCurrent(-1);
    });
    this.backdrop.querySelector("[data-logical-down]")?.addEventListener("click", () => {
      void this.moveCurrent(1);
    });
    this.backdrop.querySelector("form")?.addEventListener("submit", (event) => {
      event.preventDefault();
      void this.applyDraft();
    });
  }

  async applyDraft() {
    const priorIds = new Set((this.currentFamily()?.items || []).map((item) => item.id));
    let value;
    try {
      value = readDraft(this.backdrop, this.currentFamily(), this.draft);
    } catch (error) {
      this.showError(error);
      return;
    }
    const command = {
      type: "set-logical-object",
      kind: this.familyKind,
      value,
    };
    try {
      const result = await this.commandEngine.executeEngineCommand(
        command,
        () => this.engine()?.executeCommandJson?.(JSON.stringify(command)),
      );
      if (result.changed) {
        await this.onApply?.(result);
      }
      this.itemId = value.id || null;
      await this.refresh();
      if (!value.id) {
        this.itemId = (this.currentFamily()?.items || [])
          .find((item) => !priorIds.has(item.id))?.id || null;
        await this.refresh();
      }
    } catch (error) {
      this.showError(error);
    }
  }

  async deleteCurrent() {
    if (!this.itemId) {
      return;
    }
    const command = {
      type: "delete-logical-object",
      kind: this.familyKind,
      id: this.itemId,
    };
    try {
      const result = await this.commandEngine.executeEngineCommand(
        command,
        () => this.engine()?.executeCommandJson?.(JSON.stringify(command)),
      );
      if (result.changed) {
        await this.onApply?.(result);
      }
      this.itemId = null;
      this.draft = null;
      await this.refresh();
    } catch (error) {
      this.showError(error);
    }
  }

  async moveCurrent(delta) {
    const family = this.currentFamily();
    const current = family?.items?.findIndex((item) => item.id === this.itemId) ?? -1;
    const target = Math.max(0, Math.min((family?.items?.length || 1) - 1, current + delta));
    if (current < 0 || current === target) {
      return;
    }
    const command = {
      type: "reorder-logical-object",
      kind: this.familyKind,
      id: this.itemId,
      index: target,
    };
    try {
      const result = await this.commandEngine.executeEngineCommand(
        command,
        () => this.engine()?.executeCommandJson?.(JSON.stringify(command)),
      );
      if (result.changed) {
        await this.onApply?.(result);
      }
      await this.refresh();
    } catch (error) {
      this.showError(error);
    }
  }

  showError(error) {
    const message = String(error?.message || error || "Logical object update failed")
      .replace(/^Error:\s*/i, "");
    this.notify?.(message);
    const box = this.backdrop.querySelector(".logical-object-error")
      || document.createElement("div");
    box.className = "logical-object-error";
    box.textContent = message;
    this.backdrop.querySelector(".logical-object-editor")?.prepend(box);
  }

  close() {
    this.backdrop?.remove();
  }
}

function renderField(field, draft) {
  const value = draft?.[field.key];
  const common = `name="${escapeHtml(field.key)}" data-value-kind="${escapeHtml(field.valueKind)}"`;
  if (field.valueKind === "boolean") {
    return `<label class="logical-field logical-field-checkbox">
      <input ${common} type="checkbox" ${value ? "checked" : ""}>
      <span>${escapeHtml(field.label)}</span>
    </label>`;
  }
  if (field.valueKind === "choice") {
    return `<label class="logical-field">
      <span>${escapeHtml(field.label)}</span>
      <select ${common}>
        ${(field.options || []).map((option) => `<option value="${escapeHtml(option.value)}" ${option.value === value ? "selected" : ""}>${escapeHtml(option.label)}</option>`).join("")}
      </select>
    </label>`;
  }
  if (field.valueKind === "json" || field.valueKind.endsWith("-list")) {
    const text = field.valueKind === "json"
      ? JSON.stringify(value ?? [], null, 2)
      : Array.isArray(value) ? value.join("\n") : "";
    return `<label class="logical-field">
      <span>${escapeHtml(field.label)}</span>
      <textarea ${common} rows="${field.valueKind === "json" ? 7 : 3}">${escapeHtml(text)}</textarea>
    </label>`;
  }
  const readOnly = field.readOnlyWhenPresent && value ? "readonly" : "";
  return `<label class="logical-field">
    <span>${escapeHtml(field.label)}</span>
    <input ${common} type="text" value="${escapeHtml(formatFieldValue(value))}" ${readOnly}
      placeholder="${escapeHtml(field.placeholder || "")}">
  </label>`;
}

function readDraft(root, family, previous) {
  const value = structuredClone(previous || family?.defaultValue || { id: "" });
  for (const field of family?.fields || []) {
    const input = root.querySelector(`[name="${cssEscape(field.key)}"]`);
    if (!input) {
      continue;
    }
    const raw = String(input.value ?? "").trim();
    switch (field.valueKind) {
      case "boolean":
        value[field.key] = !!input.checked;
        break;
      case "optional-text":
      case "optional-entity":
        assignOptional(value, field.key, raw || null);
        break;
      case "optional-number":
        assignOptional(value, field.key, raw ? finiteNumber(raw, field.label) : null);
        break;
      case "optional-integer":
        assignOptional(value, field.key, raw ? finiteInteger(raw, field.label) : null);
        break;
      case "optional-number-list-2":
        assignOptional(value, field.key, raw ? numberList(raw, 2, field.label) : null);
        break;
      case "optional-number-list-4":
        assignOptional(value, field.key, raw ? numberList(raw, 4, field.label) : null);
        break;
      case "entity-list":
      case "text-list":
        value[field.key] = stringList(raw);
        break;
      case "json":
        try {
          value[field.key] = JSON.parse(raw || "[]");
        } catch {
          throw new Error(`${field.label} must contain valid structured data.`);
        }
        break;
      default:
        value[field.key] = raw;
        break;
    }
  }
  return value;
}

function assignOptional(target, key, value) {
  if (value == null || value === "") {
    delete target[key];
  } else {
    target[key] = value;
  }
}

function stringList(raw) {
  return raw
    .split(/[\n,;]+/)
    .map((value) => value.trim())
    .filter(Boolean);
}

function finiteNumber(raw, label) {
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${label} must be a finite number.`);
  }
  return value;
}

function finiteInteger(raw, label) {
  const value = Number(raw);
  if (!Number.isInteger(value) || value < -32768 || value > 32767) {
    throw new Error(`${label} must be an integer from -32768 to 32767.`);
  }
  return value;
}

function numberList(raw, length, label) {
  const values = raw.split(/[\s,;]+/).filter(Boolean).map(Number);
  if (values.length !== length || values.some((value) => !Number.isFinite(value))) {
    throw new Error(`${label} requires exactly ${length} finite numbers.`);
  }
  return values;
}

function itemSummary(family, item) {
  const keys = {
    "reaction-scheme": ["id"],
    "reaction-step": ["id", "schemeId"],
    "alternative-group": ["warning", "id"],
    "bracketed-group": ["sruLabel", "usage", "id"],
    sequence: ["identifier", "id"],
    "cross-reference": ["identifier", "sequenceIdentifier", "id"],
    "object-tag": ["displayName", "name", "id"],
    annotation: ["keyword", "content", "id"],
    "registry-number": ["number", "authority", "id"],
    representation: ["attribute", "id"],
  }[family?.kind] || ["id"];
  return keys.map((key) => item?.[key]).find((value) => String(value || "").trim())
    || "Untitled";
}

function formatFieldValue(value) {
  if (Array.isArray(value)) {
    return value.join(", ");
  }
  return value == null ? "" : String(value);
}

function cssEscape(value) {
  return globalThis.CSS?.escape
    ? globalThis.CSS.escape(String(value))
    : String(value).replaceAll('"', '\\"');
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
