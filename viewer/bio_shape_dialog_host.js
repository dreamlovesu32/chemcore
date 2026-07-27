export function createBioShapeDialogHost({ root = document.body } = {}) {
  return {
    choose(spec) {
      if (spec?.kind !== "bio-shape" || !spec?.data || !spec.objectId) {
        return Promise.resolve(null);
      }
      return new BioShapeDialog({ root, spec }).open();
    },
  };
}

class BioShapeDialog {
  constructor({ root, spec }) {
    this.root = root;
    this.spec = spec;
    this.data = structuredClone(spec.data);
  }

  open() {
    document.querySelector(".bio-shape-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog bio-shape-dialog";
      this.backdrop.innerHTML = this.markup();
      this.root.appendChild(this.backdrop);
      this.bind();
    });
  }

  markup() {
    const parameters = this.spec.parameterFields.map((field) => numberField(
      `parameter:${field.key}`,
      field.label,
      this.data.parameters?.[field.key] ?? 0,
      field.step,
      field.unit,
      field.minimum,
      field.maximum,
    )).join("");
    return `
      <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
      <div class="numeric-dialog-backdrop" data-bio-shape-close></div>
      <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.spec.title)}">
        <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.spec.title)}</div>
        <section class="bio-shape-dialog-grid">
          <label class="numeric-dialog-field"><span>Type</span><input value="${escapeHtml(this.data.kind)}" readonly><em></em></label>
          ${selectField("fillType", "Fill", this.data.fillType, ["unspecified", "none", "solid", "shaded"])}
          ${selectField("lineType", "Line", this.data.lineType, ["solid", "dashed", "bold", "wavy"])}
          <label class="numeric-dialog-field"><span>Color</span><input name="color" type="color" value="${escapeHtml(this.data.color)}"><em></em></label>
          ${numberField("lineWidth", "Line width", this.data.lineWidth, 0.05, "pt")}
          ${numberField("boldWidth", "Bold width", this.data.boldWidth, 0.05, "pt")}
          ${numberField("marginWidth", "Contour width", this.data.marginWidth, 0.05, "pt")}
          ${numberField("hashSpacing", "Dash spacing", this.data.hashSpacing, 0.1, "pt")}
          ${numberField("fadePercent", "Fade", this.data.fadePercent, 1, "%")}
          ${numberField("alpha", "Alpha", this.data.alpha ?? 1, 0.05)}
        </section>
        ${parameters ? `<section class="bio-shape-dialog-parameters"><div class="plasmid-map-dialog-section-title"><strong>Shape parameters</strong></div><div class="bio-shape-dialog-grid">${parameters}</div></section>` : ""}
        <div class="numeric-dialog-error" data-bio-shape-error role="alert"></div>
        <div class="numeric-dialog-actions"><button type="button" data-bio-shape-close>Cancel</button><button type="submit">OK</button></div>
      </form>`;
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-bio-shape-close]")) this.close(null);
    });
    this.backdrop.addEventListener("keydown", (event) => {
      if (event.key === "Escape") this.close(null);
    });
    this.backdrop.querySelector("form").addEventListener("submit", (event) => {
      event.preventDefault();
      const next = structuredClone(this.data);
      next.fillType = valueOf(this.backdrop, "fillType");
      next.lineType = valueOf(this.backdrop, "lineType");
      next.color = valueOf(this.backdrop, "color");
      for (const name of ["lineWidth", "boldWidth", "marginWidth", "hashSpacing", "fadePercent", "alpha"]) {
        next[name] = numberOf(this.backdrop, name);
      }
      next.parameters ||= {};
      for (const field of this.spec.parameterFields) {
        next.parameters[field.key] = numberOf(this.backdrop, `parameter:${field.key}`);
      }
      if (!Number.isFinite(next.alpha) || next.alpha < 0 || next.alpha > 1) {
        this.backdrop.querySelector("[data-bio-shape-error]").textContent = "Alpha must be between 0 and 1.";
        return;
      }
      this.close(next);
    });
  }

  close(value) {
    this.backdrop?.remove();
    this.resolve?.(value);
  }
}

function numberField(name, label, value, step, unit = "", minimum = null, maximum = null) {
  const range = `${minimum == null ? "" : ` min="${minimum}"`}${maximum == null ? "" : ` max="${maximum}"`}`;
  return `<label class="numeric-dialog-field"><span>${escapeHtml(label)}</span><input name="${escapeHtml(name)}" type="number" value="${Number(value)}" step="${step}"${range}><em>${unit}</em></label>`;
}
function selectField(name, label, value, values) {
  return `<label class="numeric-dialog-field"><span>${label}</span><select name="${name}">${values.map((entry) => `<option value="${entry}"${entry === value ? " selected" : ""}>${entry}</option>`).join("")}</select><em></em></label>`;
}
function valueOf(root, name) {
  return String(root.querySelector(`[name="${CSS.escape(name)}"]`)?.value ?? "");
}
function numberOf(root, name) {
  return Number(valueOf(root, name));
}
function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]);
}
