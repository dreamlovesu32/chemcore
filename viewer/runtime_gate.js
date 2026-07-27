const RUNTIME_STATES = new Set(["loading", "ready", "failed"]);

function setRuntimeState(body, state) {
  if (!RUNTIME_STATES.has(state)) {
    throw new Error(`unsupported runtime state '${state}'`);
  }
  body.dataset.runtimeState = state;
  body.setAttribute("aria-busy", state === "loading" ? "true" : "false");
}

export class RuntimeGate {
  constructor({
    body,
    titlebar,
    editorShell,
    gate,
    title,
    message,
    details,
    reloadButton,
    reload = () => globalThis.location.reload(),
  }) {
    if (!body || !editorShell || !gate || !title || !message || !details || !reloadButton) {
      throw new Error("runtime gate markup is incomplete");
    }
    this.body = body;
    this.titlebar = titlebar;
    this.editorShell = editorShell;
    this.gate = gate;
    this.title = title;
    this.message = message;
    this.details = details;
    this.reloadButton = reloadButton;
    this.reload = reload;
    this.reloadButton.addEventListener("click", () => this.reload());
    this.loading();
  }

  loading() {
    setRuntimeState(this.body, "loading");
    this.title.textContent = "Starting ChemSema";
    this.message.textContent = "Verifying the chemistry kernel before opening the editor…";
    this.details.textContent = "";
    this.details.hidden = true;
    this.reloadButton.hidden = true;
    this.gate.hidden = false;
    this.editorShell.hidden = true;
    if (this.titlebar) {
      this.titlebar.hidden = true;
    }
  }

  ready() {
    setRuntimeState(this.body, "ready");
    this.gate.hidden = true;
    this.editorShell.hidden = false;
    if (this.titlebar) {
      this.titlebar.hidden = false;
    }
  }

  failed(error) {
    const reason = String(error?.message || error || "unknown kernel initialization error");
    setRuntimeState(this.body, "failed");
    this.title.textContent = "ChemSema cannot start";
    this.message.textContent =
      "The chemistry kernel is unavailable. The editor has been disabled instead of opening an unsupported page.";
    this.details.textContent = reason;
    this.details.hidden = false;
    this.reloadButton.hidden = false;
    this.gate.hidden = false;
    this.editorShell.hidden = true;
    if (this.titlebar) {
      this.titlebar.hidden = true;
    }
  }
}

export function createRuntimeGate(documentRef = document, options = {}) {
  return new RuntimeGate({
    body: documentRef.body,
    titlebar: documentRef.querySelector("#desktop-titlebar"),
    editorShell: documentRef.querySelector(".editor-shell"),
    gate: documentRef.querySelector("#runtime-gate"),
    title: documentRef.querySelector("#runtime-gate-title"),
    message: documentRef.querySelector("#runtime-gate-message"),
    details: documentRef.querySelector("#runtime-gate-details"),
    reloadButton: documentRef.querySelector("#runtime-gate-reload"),
    ...options,
  });
}
