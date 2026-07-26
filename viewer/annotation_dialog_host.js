export function createAnnotationDialogHost({ root = document.body, engine } = {}) {
  return {
    async choose(annotation) {
      const targetEngine = engine?.();
      if (!targetEngine?.annotationDialogJson) {
        return null;
      }
      const payload = JSON.parse(await targetEngine.annotationDialogJson(annotation));
      if (payload?.kind !== "annotation-properties" || !payload.annotation) {
        return null;
      }
      return new AnnotationDialog({ root, payload }).open();
    },
  };
}

class AnnotationDialog {
  constructor({ root, payload }) {
    this.root = root;
    this.payload = payload;
  }

  open() {
    document.querySelector(".annotation-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog annotation-dialog";
      this.backdrop.innerHTML = `
        <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
        <div class="numeric-dialog-backdrop" data-annotation-dialog-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.payload.title || "Annotation Properties")}">
          <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.payload.title || "Annotation Properties")}</div>
          <div class="annotation-dialog-fields">
            ${(this.payload.fields || []).map(renderField).join("")}
          </div>
          <div class="numeric-dialog-actions">
            <button type="button" data-annotation-dialog-close>Cancel</button>
            <button type="submit">${this.payload.objectId ? "Apply" : "Create"}</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.bind();
      this.backdrop.querySelector("input:not([type=checkbox])")?.focus?.({ preventScroll: true });
      this.backdrop.querySelector("input:not([type=checkbox])")?.select?.();
    });
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-annotation-dialog-close]")) {
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
      this.submit();
    });
  }

  submit() {
    const properties = {};
    let valid = true;
    for (const field of this.payload.fields || []) {
      const input = this.backdrop.querySelector(`[name="${cssEscape(field.key)}"]`);
      if (!input) {
        valid = false;
        continue;
      }
      if (field.valueKind === "boolean") {
        properties[field.key] = !!input.checked;
        continue;
      }
      if (field.valueKind === "text" || field.valueKind === "choice") {
        properties[field.key] = String(input.value ?? "");
        continue;
      }
      const value = Number(String(input.value || "").trim());
      const fieldValid = Number.isFinite(value)
        && (field.minimum == null || value >= Number(field.minimum))
        && (field.maximum == null || value <= Number(field.maximum));
      input.classList.toggle("is-invalid", !fieldValid);
      valid &&= fieldValid;
      if (fieldValid) {
        properties[field.key] = value;
      }
    }
    if (properties.minimum != null
      && properties.maximum != null
      && properties.minimum > properties.maximum) {
      valid = false;
      this.backdrop.querySelector('[name="minimum"]')?.classList.add("is-invalid");
      this.backdrop.querySelector('[name="maximum"]')?.classList.add("is-invalid");
    }
    if (properties.autoValue === true) {
      delete properties.textOverride;
    }
    switch (properties.positioningType) {
      case "auto":
        delete properties.positionX;
        delete properties.positionY;
        delete properties.positioningOffsetX;
        delete properties.positioningOffsetY;
        delete properties.positioningAngle;
        break;
      case "absolute":
        delete properties.positioningOffsetX;
        delete properties.positioningOffsetY;
        delete properties.positioningAngle;
        break;
      case "offset":
        delete properties.positionX;
        delete properties.positionY;
        delete properties.positioningAngle;
        break;
      case "angle":
        delete properties.positioningOffsetX;
        delete properties.positioningOffsetY;
        break;
      default:
        valid = false;
        break;
    }
    if (!valid) {
      this.backdrop.querySelector(".is-invalid")?.focus?.();
      return;
    }
    this.close({
      annotation: this.payload.annotation,
      objectId: this.payload.objectId || null,
      properties,
    });
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function renderField(field) {
  if (field.valueKind === "boolean") {
    return `
      <label class="numeric-dialog-field annotation-dialog-checkbox">
        <input name="${escapeHtml(field.key)}" type="checkbox" ${field.value ? "checked" : ""}>
        <span>${escapeHtml(field.label || field.key)}</span>
      </label>
    `;
  }
  if (field.valueKind === "choice") {
    return `
      <label class="numeric-dialog-field">
        <span>${escapeHtml(field.label || field.key)}</span>
        <select name="${escapeHtml(field.key)}">
          ${(field.options || []).map((option) => `
            <option value="${escapeHtml(option.value)}" ${option.value === field.value ? "selected" : ""}>${escapeHtml(option.label || option.value)}</option>
          `).join("")}
        </select>
        <em></em>
      </label>
    `;
  }
  if (field.valueKind === "text") {
    return `
      <label class="numeric-dialog-field">
        <span>${escapeHtml(field.label || field.key)}</span>
        <input name="${escapeHtml(field.key)}" type="text" value="${escapeHtml(field.value || "")}">
        <em>${escapeHtml(field.unit || "")}</em>
      </label>
    `;
  }
  return `
    <label class="numeric-dialog-field">
      <span>${escapeHtml(field.label || field.key)}</span>
      <input name="${escapeHtml(field.key)}" type="text" inputmode="decimal" value="${escapeHtml(formatNumber(field.value))}">
      <em>${escapeHtml(field.unit || "")}</em>
    </label>
  `;
}

function formatNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) ? String(Math.round(number * 1000) / 1000) : "";
}

function cssEscape(value) {
  if (globalThis.CSS?.escape) {
    return globalThis.CSS.escape(String(value));
  }
  return String(value).replaceAll('"', '\\"');
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
