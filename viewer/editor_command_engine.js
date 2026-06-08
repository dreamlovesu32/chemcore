export function createEditorCommandEngine(options = {}) {
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

  function engine() {
    return typeof options.engine === "function" ? options.engine() : options.engine || null;
  }

  function normalizeCommand(command) {
    if (typeof command === "string") {
      return { type: command };
    }
    return { ...(command || {}) };
  }

  function kernelCommandFrom(command) {
    const { payload, schemaVersion, apply, label, meta, ...rest } = command || {};
    return {
      ...(payload && typeof payload === "object" ? payload : {}),
      ...rest,
    };
  }

  function parseCommandResultJson(json) {
    if (!json || typeof json !== "string") {
      return null;
    }
    try {
      return JSON.parse(json);
    } catch {
      return null;
    }
  }

  function readEngineResult() {
    const activeEngine = engine();
    const resultJson = activeEngine?.lastCommandResultJson?.();
    return parseCommandResultJson(resultJson);
  }

  function readEngineRevision() {
    const activeEngine = engine();
    const nextRevision = activeEngine?.revision?.();
    return Number.isFinite(Number(nextRevision)) ? Number(nextRevision) : revision;
  }

  async function executeCommand(command, executeOptions = {}) {
    const normalized = normalizeCommand(command);
    const activeEngine = engine();
    const apply = executeOptions.apply || normalized.apply;
    let rawResult = null;
    let result = null;

    if (typeof apply === "function") {
      rawResult = await apply(normalized);
      if (executeOptions.sync !== false && rawResult !== false) {
        await options.syncDocumentFromEngine?.();
      }
      result = readEngineResult();
    } else if (activeEngine?.executeCommandJson) {
      const commandJson = JSON.stringify(kernelCommandFrom(normalized));
      const resultJson = await activeEngine.executeCommandJson(commandJson);
      result = parseCommandResultJson(resultJson);
      if (executeOptions.sync !== false && result?.changed) {
        await options.syncDocumentFromEngine?.();
      }
    } else {
      throw new Error(`Command '${normalized.type || "unknown"}' has no engine executor.`);
    }

    if (!result) {
      revision = readEngineRevision();
      result = {
        changed: Boolean(rawResult),
        revision,
        beforeRevision: revision,
        command: kernelCommandFrom(normalized),
      };
    }

    revision = Number.isFinite(Number(result.revision)) ? Number(result.revision) : readEngineRevision();

    if (!result.changed) {
      await executeOptions.onUnchanged?.();
      await emit("command-executed", {
        ...result,
        command: result.command || kernelCommandFrom(normalized),
        commandType: normalized.type,
        rawResult,
      });
      return {
        ...result,
        command: result.command || kernelCommandFrom(normalized),
        rawResult,
      };
    }

    const event = {
      ...result,
      commitId: `cmd_${String(nextCommandIndex++).padStart(6, "0")}`,
      command: result.command || kernelCommandFrom(normalized),
      commandType: normalized.type,
      source: executeOptions.source || normalized.meta?.source || "ui",
      label: executeOptions.label || normalized.label || normalized.type,
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
    return readEngineRevision();
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
