class TauriColorHost {
  constructor(desktopFileHost) {
    this.kind = "tauri";
    this.desktopFileHost = desktopFileHost;
  }

  async chooseColor(initialColor, customColors = []) {
    return this.desktopFileHost.chooseColor(initialColor, customColors);
  }
}

class WebColorHost {
  constructor(root = document.body) {
    this.kind = "web";
    this.root = root;
  }

  chooseColor(initialColor) {
    return new Promise((resolve) => {
      const input = document.createElement("input");
      input.type = "color";
      input.value = normalizeHexColor(initialColor) || "#000000";
      input.className = "visually-hidden";
      input.setAttribute("aria-label", "Choose color");
      this.root.appendChild(input);
      let settled = false;
      const finish = (color) => {
        if (settled) {
          return;
        }
        settled = true;
        input.remove();
        resolve(normalizeHexColor(color));
      };
      input.addEventListener("change", () => finish(input.value), { once: true });
      input.addEventListener("blur", () => setTimeout(() => finish(null), 0), { once: true });
      input.click();
    });
  }
}

export function createColorHost({ desktopFileHost } = {}) {
  if (desktopFileHost?.available && typeof desktopFileHost.chooseColor === "function") {
    return new TauriColorHost(desktopFileHost);
  }
  return new WebColorHost();
}

function normalizeHexColor(value) {
  const raw = String(value || "").trim().toLowerCase();
  if (/^#[0-9a-f]{6}$/.test(raw)) {
    return raw;
  }
  if (/^#[0-9a-f]{3}$/.test(raw)) {
    return `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`;
  }
  return null;
}
