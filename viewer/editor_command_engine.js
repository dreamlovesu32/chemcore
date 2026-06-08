export function createEditorCommandEngine(options) {
  const listeners = new Map();
  let revision = 0;
  let nextCommandIndex = 1;

  function on(eventName, listener) {
    if (!listeners.has(eventName)) {
      listeners.set(eventName, new Set());
    }
    listeners.get(eventName).add(listener);
    return () => listeners.get(eventName)?.delete(listener);
  }

  async function emit(eventName, event) {
    for (const listener of listeners.get(eventName) || []) {
      await listener(event);
    }
  }

  function normalizeCommand(command) {
    if (typeof command === "string") {
      return {
        type: command,
        schemaVersion: 1,
        payload: {},
      };
    }
    return {
      schemaVersion: 1,
      payload: {},
      ...(command || {}),
    };
  }

  async function executeCommand(command, executeOptions = {}) {
    const normalized = normalizeCommand(command);
    const apply = executeOptions.apply || normalized.apply;
    if (typeof apply !== "function") {
      throw new Error(`Command '${normalized.type || "unknown"}' has no apply handler.`);
    }

    const beforeRevision = revision;
    const beforeFingerprint = options.currentDocumentFingerprint?.() || null;
    const rawResult = await apply(normalized);
    const rawChanged = rawResult !== false;
    const shouldSync = executeOptions.sync !== false
      && (rawChanged || executeOptions.assumeChanged || executeOptions.compareDocument);

    if (shouldSync) {
      await options.syncDocumentFromEngine?.();
    }

    const afterFingerprint = options.currentDocumentFingerprint?.() || null;
    const changed = Boolean(
      rawChanged
      && (executeOptions.assumeChanged || beforeFingerprint !== afterFingerprint),
    );

    if (!changed) {
      await executeOptions.onUnchanged?.();
      await emit("command-executed", {
        command: normalized,
        changed: false,
        revision,
        beforeRevision,
      });
      return {
        changed: false,
        revision,
        beforeRevision,
        command: normalized,
        rawResult,
      };
    }

    revision += 1;
    const event = {
      commitId: `cmd_${String(nextCommandIndex++).padStart(6, "0")}`,
      command: normalized,
      commandType: normalized.type,
      changed: true,
      revision,
      beforeRevision,
      source: executeOptions.source || normalized.meta?.source || "ui",
      label: executeOptions.label || normalized.label || normalized.type,
      beforeFingerprint,
      afterFingerprint,
      rawResult,
    };
    await emit("command-executed", event);
    await emit("document-committed", event);
    await options.onDocumentCommitted?.(event);
    return event;
  }

  async function executeEngineCommand(command, apply, executeOptions = {}) {
    return executeCommand(command, {
      ...executeOptions,
      apply,
    });
  }

  function currentRevision() {
    return revision;
  }

  function resetRevision(nextRevision = 0) {
    revision = Math.max(0, Number(nextRevision) || 0);
    nextCommandIndex = 1;
  }

  return {
    on,
    executeCommand,
    executeEngineCommand,
    currentRevision,
    resetRevision,
  };
}
