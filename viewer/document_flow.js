import {
  documentTitleForFileName,
  downloadTextFile,
  looksLikeCdxmlFile,
  saveFormatFromFileName,
} from "./file_io.js";

export function createDocumentFlow(options) {
  function loadDocument(path) {
    return fetch(path, { cache: "no-store" }).then((response) => {
      if (!response.ok) {
        throw new Error(`Failed to load ${path}: ${response.status}`);
      }
      return response.json();
    });
  }

  function validateChemcoreJsonDocument(documentData) {
    if (!documentData || typeof documentData !== "object") {
      throw new Error("JSON root must be an object.");
    }
    if (!documentData.document || typeof documentData.document !== "object") {
      throw new Error("Missing document section.");
    }
    if (!Array.isArray(documentData.objects)) {
      throw new Error("Missing objects array.");
    }
    if (!documentData.resources || typeof documentData.resources !== "object") {
      throw new Error("Missing resources section.");
    }
  }

  function loadJsonDocumentIntoEditor(documentData, fileName = null) {
    validateChemcoreJsonDocument(documentData);
    options.finishActiveTextEditor(false);
    options.state.currentPath = null;
    options.state.currentFileName = fileName;
    options.state.editorEngine?.free?.();
    options.state.editorEngine = new options.WasmEngine();
    options.state.lastEditFocusPoint = null;
    options.clearZoomHandoffs();
    options.state.editorEngine.loadDocumentJson(JSON.stringify(documentData));
    options.syncDocumentStylePresetFromEngine();
    options.syncEngineToolState();
    options.syncDocumentFromEngine();
    options.state.runtimeViewBox = options.state.currentDocument?.document?.page
      ? options.pageViewBox(options.state.currentDocument.document.page)
      : options.defaultEditorViewBox();
    options.viewerTitle.textContent = options.state.currentDocument?.document?.title || fileName || "Untitled";
    updateDocumentMeta();
    options.renderDocument();
    options.fitView();
  }

  function currentDocumentJsonForSave() {
    options.finishActiveTextEditor(true);
    if (options.state.editorEngine && !options.state.currentPath) {
      options.syncDocumentFromEngine();
    }
    if (!options.state.currentDocument) {
      throw new Error("No document to save.");
    }
    return `${JSON.stringify(options.state.currentDocument, null, 2)}\n`;
  }

  function cdxmlFileNameForSave() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return baseName.replace(/\.[^.]+$/, "") + ".cdxml";
  }

  function svgFileNameForSave() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return baseName.replace(/\.[^.]+$/, "") + ".svg";
  }

  function saveAsBaseName() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return baseName.replace(/\.[^.]+$/, "") || "chemcore-document";
  }

  function savePayloadForFormat(format) {
    if (format === "svg") {
      return {
        content: currentDocumentSvgForSave(),
        mimeType: "image/svg+xml",
      };
    }
    if (format === "cdxml") {
      return {
        content: currentDocumentCdxmlForSave(),
        mimeType: "chemical/x-cdxml",
      };
    }
    return {
      content: currentDocumentJsonForSave(),
      mimeType: "application/json",
    };
  }

  async function saveCurrentDocumentJson() {
    const json = currentDocumentJsonForSave();
    const suggestedName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{ description: "chemcore JSON", accept: { "application/json": [".json"] } }],
      });
      const writable = await handle.createWritable();
      await writable.write(json);
      await writable.close();
      options.state.currentFileName = handle.name || suggestedName;
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      return;
    }
    downloadTextFile(json, suggestedName, "application/json");
  }

  function currentDocumentCdxmlForSave() {
    options.finishActiveTextEditor(true);
    if (!options.state.editorEngine) {
      throw new Error("CDXML export is unavailable.");
    }
    return options.state.editorEngine.documentCdxml();
  }

  function currentDocumentSvgForSave() {
    options.finishActiveTextEditor(true);
    if (!options.state.editorEngine?.documentSvg) {
      throw new Error("SVG export is unavailable.");
    }
    return options.state.editorEngine.documentSvg();
  }

  async function saveCurrentDocumentCdxml() {
    const cdxml = currentDocumentCdxmlForSave();
    const suggestedName = cdxmlFileNameForSave();
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{ description: "ChemDraw CDXML", accept: { "chemical/x-cdxml": [".cdxml"], "text/xml": [".cdxml"] } }],
      });
      const writable = await handle.createWritable();
      await writable.write(cdxml);
      await writable.close();
      options.state.currentFileName = handle.name || suggestedName;
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      return;
    }
    downloadTextFile(cdxml, suggestedName, "chemical/x-cdxml");
  }

  async function saveCurrentDocumentSvg() {
    const svg = currentDocumentSvgForSave();
    const suggestedName = svgFileNameForSave();
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{ description: "Scalable Vector Graphics", accept: { "image/svg+xml": [".svg"] } }],
      });
      const writable = await handle.createWritable();
      await writable.write(svg);
      await writable.close();
      return;
    }
    downloadTextFile(svg, suggestedName, "image/svg+xml");
  }

  async function saveCurrentDocumentAs() {
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName: `${saveAsBaseName()}.cdxml`,
        types: [
          { description: "ChemDraw CDXML", accept: { "chemical/x-cdxml": [".cdxml"], "text/xml": [".cdxml"] } },
          { description: "Scalable Vector Graphics", accept: { "image/svg+xml": [".svg"] } },
          { description: "chemcore JSON", accept: { "application/json": [".json"] } },
        ],
      });
      const format = saveFormatFromFileName(handle.name);
      const { content } = savePayloadForFormat(format);
      const writable = await handle.createWritable();
      await writable.write(content);
      await writable.close();
      if (format !== "svg") {
        options.state.currentFileName = handle.name || options.state.currentFileName;
        options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      }
      return;
    }
    await saveCurrentDocumentJson();
  }

  async function openDocumentFile(file) {
    if (!file) {
      return;
    }
    const text = await file.text();
    if (looksLikeCdxmlFile(file, text)) {
      options.finishActiveTextEditor(false);
      options.state.currentPath = null;
      options.state.currentFileName = file.name || null;
      options.state.editorEngine?.free?.();
      options.state.editorEngine = new options.WasmEngine();
      options.state.lastEditFocusPoint = null;
      options.clearZoomHandoffs();
      options.state.editorEngine.loadDocumentCdxml(text);
      options.syncDocumentStylePresetFromEngine();
      options.syncEngineToolState();
      options.syncDocumentFromEngine();
      options.state.runtimeViewBox = options.state.currentDocument?.document?.page
        ? options.pageViewBox(options.state.currentDocument.document.page)
        : options.defaultEditorViewBox();
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || file.name || "Imported CDXML";
      updateDocumentMeta();
      options.renderDocument();
      options.fitView();
      return;
    }
    loadJsonDocumentIntoEditor(JSON.parse(text), file.name || null);
  }

  function isAbortError(error) {
    return error?.name === "AbortError";
  }

  async function chooseAndOpenDocument() {
    if (window.showOpenFilePicker) {
      const [handle] = await window.showOpenFilePicker({
        multiple: false,
        types: [{
          description: "chemcore JSON or CDXML",
          accept: {
            "application/json": [".json"],
            "text/xml": [".cdxml"],
            "application/xml": [".cdxml"],
            "application/x-cdxml": [".cdxml"],
            "chemical/x-cdxml": [".cdxml"],
            "application/vnd.cambridgesoft.cdxml": [".cdxml"],
          },
        }],
        excludeAcceptAllOption: false,
      });
      if (!handle) {
        return;
      }
      await openDocumentFile(await handle.getFile());
      return;
    }
    options.openFileInput.click();
  }

  function currentDocumentMetaPayload() {
    if (!options.state.currentDocument) {
      return null;
    }
    return {
      sample: options.state.currentPath || options.state.currentFileName || "blank",
      page: options.state.currentDocument.document.page,
      meta: options.state.currentDocument.document.meta,
      display: options.state.displayMetrics,
    };
  }

  function updateDocumentMeta() {
    const payload = currentDocumentMetaPayload();
    if (!options.docMeta || !payload) {
      return;
    }
    options.docMeta.textContent = JSON.stringify(payload, null, 2);
  }

  async function loadAndRender() {
    options.finishActiveTextEditor(false);
    options.clearZoomHandoffs();
    options.viewerTitle.textContent = "Loading...";
    try {
      if (options.state.currentPath) {
        options.state.currentFileName = null;
        const documentData = await loadDocument(options.state.currentPath);
        options.state.currentDocument = documentData;
        options.state.runtimeViewBox = options.pageViewBox(documentData.document.page);
        options.syncCoreRenderListFromCurrentDocument();
      } else {
        options.state.coreRenderList = null;
        if (!options.state.editorEngine) {
          options.resetEditorEngine();
        } else {
          options.state.editorEngine.clearInteraction();
          options.syncEngineToolState();
          options.syncDocumentFromEngine();
        }
      }
      const documentData = options.state.currentDocument;
      options.state.currentDocument = documentData;
      options.viewerTitle.textContent = documentData.document.title || options.state.currentPath;
      updateDocumentMeta();
      options.renderDocument();
      options.fitView();
    } catch (error) {
      options.viewerTitle.textContent = "Load failed";
      options.viewerStats.textContent = "";
      options.docMeta.textContent = String(error);
      options.viewerSvg.innerHTML = "";
    }
  }

  return {
    chooseAndOpenDocument,
    isAbortError,
    loadAndRender,
    loadJsonDocumentIntoEditor,
    openDocumentFile,
    saveCurrentDocumentAs,
    saveCurrentDocumentCdxml,
    saveCurrentDocumentSvg,
    updateDocumentMeta,
  };
}
