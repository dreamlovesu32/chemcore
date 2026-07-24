export function createChemicalPropertyDialogHost({ root = document.body, engine } = {}) {
  return {
    async choose() {
      const targetEngine = engine?.();
      if (!targetEngine?.chemicalPropertyDialogJson) {
        return null;
      }
      const payload = JSON.parse(await targetEngine.chemicalPropertyDialogJson());
      if (payload?.kind !== "chemical-property" || !Array.isArray(payload.fields)) {
        return null;
      }
      return new ChemicalPropertyDialog({ root, payload }).open();
    },
  };
}

class ChemicalPropertyDialog {
  constructor({ root, payload }) {
    this.root = root;
    this.payload = payload;
  }

  open() {
    document.querySelector(".chemical-property-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      const fields = Object.fromEntries(this.payload.fields.map((field) => [field.key, field]));
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog chemical-property-dialog";
      this.backdrop.innerHTML = `
        <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
        <div class="numeric-dialog-backdrop" data-chemical-property-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.payload.title)}">
          <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.payload.title)}</div>
          ${textField(fields.typeCode)}
          ${textField(fields.typeName)}
          ${textField(fields.value)}
          <label class="numeric-dialog-field chemical-property-checkbox">
            <input name="isActive" type="checkbox"${fields.isActive?.value ? " checked" : ""}>
            <span>${escapeHtml(fields.isActive?.label || "Active")}</span>
          </label>
          <p class="chemical-property-help">${escapeHtml(this.payload.typeHelp || "")}</p>
          <div class="numeric-dialog-error" data-chemical-property-error role="alert"></div>
          <div class="numeric-dialog-actions">
            ${this.payload.canDelete ? '<button type="button" data-chemical-property-delete>Delete</button>' : ""}
            <button type="button" data-chemical-property-close>Cancel</button>
            <button type="submit">Apply</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.bind();
      this.backdrop.querySelector('input[name="value"]')?.focus?.({ preventScroll: true });
    });
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-chemical-property-close]")) {
        this.close(null);
      } else if (event.target.closest("[data-chemical-property-delete]")) {
        this.close({ action: "delete" });
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
    const typeCodeText = this.value("typeCode").trim();
    if (typeCodeText && !/^\d+$/.test(typeCodeText)) {
      this.showError("Type code must be an unsigned integer or empty.");
      return;
    }
    const typeCode = typeCodeText ? Number(typeCodeText) : null;
    if (typeCode != null && (!Number.isSafeInteger(typeCode) || typeCode > 0xFFFFFFFF)) {
      this.showError("Type code must be between 0 and 4294967295.");
      return;
    }
    this.close({
      action: "apply",
      payload: {
        propertyId: this.payload.propertyId || null,
        typeCode,
        typeName: this.value("typeName").trim() || null,
        value: this.value("value"),
        isActive: Boolean(this.backdrop.querySelector('input[name="isActive"]')?.checked),
      },
    });
  }

  value(name) {
    return String(this.backdrop.querySelector(`[name="${name}"]`)?.value ?? "");
  }

  showError(message) {
    const host = this.backdrop.querySelector("[data-chemical-property-error]");
    if (host) host.textContent = message;
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function textField(field = {}) {
  return `
    <label class="numeric-dialog-field">
      <span>${escapeHtml(field.label || "Value")}</span>
      <input name="${escapeHtml(field.key || "value")}" type="text" inputmode="${escapeHtml(field.inputMode || "text")}" value="${escapeHtml(field.value ?? "")}">
      <em></em>
    </label>
  `;
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
