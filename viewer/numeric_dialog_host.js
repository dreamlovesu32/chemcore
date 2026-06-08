export function createNumericDialogHost({ root = document.body, engine, commandEngine, onApply } = {}) {
  return {
    async choose(kind) {
      const targetEngine = engine?.();
      if (!targetEngine?.selectionNumericDialogJson || !targetEngine?.applySelectionNumericDialogJson) {
        return false;
      }
      const payload = JSON.parse(await targetEngine.selectionNumericDialogJson(kind));
      const dialog = new NumericDialog({
        root,
        payload,
        apply: async (value) => {
          const payloadJson = JSON.stringify({
            kind: payload.kind,
            value,
          });
          const result = commandEngine?.executeEngineCommand
            ? await commandEngine.executeEngineCommand(
              {
                type: "apply-selection-numeric",
                payload: {
                  kind: payload.kind,
                  value,
                },
              },
              () => targetEngine.applySelectionNumericDialogJson(payloadJson),
            )
            : { changed: !!(await targetEngine.applySelectionNumericDialogJson(payloadJson)) };
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

class NumericDialog {
  constructor({ root, payload, apply }) {
    this.root = root;
    this.payload = payload || {};
    this.apply = apply;
  }

  open() {
    document.querySelector(".numeric-dialog")?.remove();
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.backdrop = document.createElement("div");
      this.backdrop.className = "numeric-dialog";
      const field = this.payload.field || {};
      this.backdrop.innerHTML = `
        <div class="numeric-dialog-backdrop" data-numeric-dialog-close></div>
        <form class="numeric-dialog-panel" aria-label="${escapeHtml(this.payload.title || "Value")}">
          <div class="numeric-dialog-title">${escapeHtml(this.payload.title || "Value")}</div>
          <label class="numeric-dialog-field">
            <span>${escapeHtml(field.label || "Value")}</span>
            <input name="value" type="text" inputmode="decimal" value="${escapeHtml(formatNumber(field.value))}">
            <em>${escapeHtml(field.unit || "")}</em>
          </label>
          <div class="numeric-dialog-actions">
            <button type="button" data-numeric-dialog-close>Cancel</button>
            <button type="submit">Apply</button>
          </div>
        </form>
      `;
      this.root.appendChild(this.backdrop);
      this.bind();
      this.backdrop.querySelector("input")?.focus?.({ preventScroll: true });
      this.backdrop.querySelector("input")?.select?.();
    });
  }

  bind() {
    this.backdrop.addEventListener("click", (event) => {
      if (event.target.closest("[data-numeric-dialog-close]")) {
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
  }

  async submit() {
    const input = this.backdrop.querySelector("input");
    const value = Number(String(input?.value || "").trim());
    try {
      const changed = await this.apply(value);
      this.close(changed);
    } catch {
      input?.classList.add("is-invalid");
      input?.focus?.();
    }
  }

  close(result) {
    this.backdrop?.remove();
    this.resolve?.(result);
  }
}

function formatNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) ? String(Math.round(number * 1000) / 1000) : "";
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
