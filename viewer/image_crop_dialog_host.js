export function createImageCropDialogHost({ root = document.body } = {}) {
  return {
    choose(spec) {
      if (spec?.kind !== "image-crop" || !spec.objectId) {
        return Promise.resolve(null);
      }
      return new ImageCropDialog({ root, spec }).open();
    },
  };
}

class ImageCropDialog {
  constructor({ root, spec }) {
    this.root = root;
    this.spec = spec;
  }

  open() {
    document.querySelector(".image-crop-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog image-crop-dialog";
      const crop = this.spec.crop;
      this.backdrop.innerHTML = `
        <div class="desktop-modal-window-drag-strip" data-desktop-window-drag-region aria-hidden="true"></div>
        <div class="numeric-dialog-backdrop" data-image-crop-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.spec.title)}">
          <div class="numeric-dialog-title" data-desktop-window-drag-region>${escapeHtml(this.spec.title)}</div>
          <p class="numeric-dialog-hint">Source: ${this.spec.sourceWidth} × ${this.spec.sourceHeight} px</p>
          ${field("x", "Left", crop.x, 0, this.spec.sourceWidth)}
          ${field("y", "Top", crop.y, 0, this.spec.sourceHeight)}
          ${field("width", "Width", crop.width, 1, this.spec.sourceWidth)}
          ${field("height", "Height", crop.height, 1, this.spec.sourceHeight)}
          <div class="numeric-dialog-error" data-image-crop-error role="alert"></div>
          <div class="numeric-dialog-actions">
            <button type="button" data-image-crop-full>Full Image</button>
            <button type="button" data-image-crop-close>Cancel</button>
            <button type="submit">Apply</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.bind();
      this.backdrop.querySelector('[name="x"]')?.focus?.({ preventScroll: true });
      this.backdrop.querySelector('[name="x"]')?.select?.();
    });
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-image-crop-close]")) {
        this.close(null);
      } else if (event.target.closest("[data-image-crop-full]")) {
        this.close({ objectId: this.spec.objectId, crop: null });
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
    const crop = Object.fromEntries(
      ["x", "y", "width", "height"].map((key) => [
        key,
        Number(this.backdrop.querySelector(`[name="${key}"]`)?.value),
      ]),
    );
    const valid = Object.values(crop).every((value) => Number.isInteger(value))
      && crop.x >= 0
      && crop.y >= 0
      && crop.width > 0
      && crop.height > 0
      && crop.x + crop.width <= Number(this.spec.sourceWidth)
      && crop.y + crop.height <= Number(this.spec.sourceHeight);
    if (!valid) {
      this.backdrop.querySelector("[data-image-crop-error]").textContent =
        "The crop rectangle must stay inside the source image.";
      return;
    }
    this.close({ objectId: this.spec.objectId, crop });
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function field(name, label, value, minimum, maximum) {
  return `
    <label class="numeric-dialog-field">
      <span>${label}</span>
      <input name="${name}" type="number" value="${Number(value)}" min="${minimum}" max="${maximum}" step="1">
      <em>px</em>
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
