import {
  CHEMCORE_COMPRESSED_EXTENSION,
  CHEMCORE_COMPRESSED_MIME,
  CHEMCORE_TEXT_EXTENSION,
  CHEMCORE_TEXT_MIME,
  CHEMDRAW_CDX_MIME,
  MDL_SDF_MIME,
  baseNameWithoutDocumentExtension,
  chemcoreOpenAcceptTypes,
  compressChemcoreText,
  decompressChemcoreText,
  documentTitleForFileName,
  downloadBinaryFile,
  downloadBlobFile,
  downloadTextFile,
  looksLikeCompressedChemcoreFile,
  looksLikeCdxFile,
  looksLikeCdxmlFile,
  looksLikeSdfFile,
  saveFormatFromFileName,
} from "./file_io.js";
import { pdfPreviewBase64FromSvg } from "./export_preview.js";

export function createDocumentFlow(options) {
  function traceEvent(event, detail = null) {
    void options.traceEvent?.(event, detail);
  }

  async function waitForRuntimeReady() {
    await options.waitForRuntimeReady?.();
  }

  async function loadDocument(path) {
    const response = await fetch(path, { cache: "no-store" });
    if (!response.ok) {
      throw new Error(`Failed to load ${path}: ${response.status}`);
    }
    const compressed = path.toLowerCase().endsWith(CHEMCORE_COMPRESSED_EXTENSION);
    const text = compressed
      ? await decompressChemcoreText(await response.arrayBuffer())
      : await response.text();
    return JSON.parse(text);
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

  async function createLoadedEditorEngine(label, load, detail = null) {
    traceEvent("documentFlow.createLoadedEngine.begin", { label, detail });
    const engine = options.engineHost.createEngineSession();
    try {
      await engine.ready?.();
      traceEvent("documentFlow.createLoadedEngine.ready", { label });
      await load(engine);
      traceEvent("documentFlow.createLoadedEngine.loaded", { label });
      return engine;
    } catch (error) {
      traceEvent("documentFlow.createLoadedEngine.error", { label, error });
      await engine.free?.();
      throw error;
    }
  }

  async function replaceEditorDocumentEngine(engine, fileName, filePath, titleFallback) {
    traceEvent("documentFlow.replaceEngine.begin", { fileName, filePath, titleFallback });
    const previousEngine = options.state.editorEngine;
    options.state.currentPath = null;
    options.state.currentFileName = fileName;
    options.state.currentFilePath = filePath;
    options.state.editorEngine = engine;
    options.resetCommandEngineRevision?.();
    options.state.lastEditFocusPoint = null;
    options.clearZoomHandoffs();
    try {
      await options.syncEngineToolState();
      await options.syncDocumentFromEngine();
      options.renderSecondaryToolbar?.();
      options.state.runtimeViewBox = null;
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || fileName || titleFallback;
      updateDocumentMeta();
      options.fitView();
      options.renderDocument();
      options.markCurrentDocumentSaved?.();
      traceEvent("documentFlow.replaceEngine.rendered", {
        title: options.viewerTitle.textContent,
        hasPreviousEngine: !!previousEngine,
      });
    } finally {
      traceEvent("documentFlow.replaceEngine.freePrevious.begin", { hasPreviousEngine: !!previousEngine });
      await previousEngine?.free?.();
      traceEvent("documentFlow.replaceEngine.freePrevious.done", { hasPreviousEngine: !!previousEngine });
    }
  }

  async function loadJsonDocumentIntoEditor(documentData, fileName = null, filePath = null) {
    await waitForRuntimeReady();
    validateChemcoreJsonDocument(documentData);
    await options.finishActiveTextEditor(false);
    const json = JSON.stringify(documentData);
    const engine = await createLoadedEditorEngine(
      "json",
      (nextEngine) => nextEngine.loadDocumentJson(json),
      { jsonLength: json.length, fileName, filePath },
    );
    await replaceEditorDocumentEngine(engine, fileName, filePath, "Untitled");
  }

  async function currentDocumentJsonForSave() {
    await options.finishActiveTextEditor(true);
    if (options.state.editorEngine && !options.state.currentPath) {
      await options.syncDocumentFromEngine();
    }
    if (!options.state.currentDocument) {
      throw new Error("No document to save.");
    }
    return `${JSON.stringify(options.state.currentDocument, null, 2)}\n`;
  }

  function cdxmlFileNameForSave() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return `${baseNameWithoutDocumentExtension(baseName)}.cdxml`;
  }

  function svgFileNameForSave() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return `${baseNameWithoutDocumentExtension(baseName)}.svg`;
  }

  function pdfFileNameForSave() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return `${baseNameWithoutDocumentExtension(baseName)}.pdf`;
  }

  function emfFileNameForSave() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return `${baseNameWithoutDocumentExtension(baseName)}.emf`;
  }

  function saveAsBaseName() {
    const baseName = options.state.currentFileName || documentTitleForFileName(options.state.currentDocument);
    return baseNameWithoutDocumentExtension(baseName) || "chemcore-document";
  }

  function preferredSaveFormat() {
    const format = saveFormatFromFileName(options.state.currentFileName || options.state.currentFilePath || "");
    return format || "ccjz";
  }

  function extensionForSaveFormat(format) {
    switch (format) {
      case "ccjs": return CHEMCORE_TEXT_EXTENSION;
      case "cdxml": return ".cdxml";
      case "cdx": return ".cdx";
      case "sdf": return ".sdf";
      case "svg": return ".svg";
      default: return CHEMCORE_COMPRESSED_EXTENSION;
    }
  }

  function suggestedSaveAsName() {
    const format = preferredSaveFormat();
    return `${saveAsBaseName()}${extensionForSaveFormat(format)}`;
  }

  function currentHasCleanCompleteDocumentSave() {
    const sourceName = options.state.currentFileName || options.state.currentFilePath || "";
    if (!sourceName) {
      return false;
    }
    const format = saveFormatFromFileName(sourceName);
    return ["ccjz", "ccjs", "cdxml", "cdx"].includes(format) && !options.currentDocumentIsDirty?.();
  }

  function lossyExportWarningMessage(format) {
    if (format === "sdf") {
      return "SDF is a molecular structure exchange format. Exporting to SDF will save molecule structures and mappable data fields only; text, arrows, shapes, orbitals, colors, line widths, fonts, page layout, and other drawing styles will not be written. To preserve the full editable document appearance, save as ChemCore, CDXML, or CDX.";
    }
    if (format === "svg" || format === "emf") {
      return `${format.toUpperCase()} is a presentation/export format. It does not preserve the full editable ChemCore document. To preserve editable molecules, objects, and styling, save as ChemCore, CDXML, or CDX.`;
    }
    return "";
  }

  async function confirmLossyExportIfNeeded(format) {
    if (!["sdf", "svg", "emf"].includes(format) || currentHasCleanCompleteDocumentSave()) {
      return true;
    }
    const message = lossyExportWarningMessage(format);
    if (!message) {
      return true;
    }
    return window.confirm(message);
  }

  async function savePayloadForFormat(format) {
    if (format === "svg") {
      return {
        content: await currentDocumentSvgForSave(),
        mimeType: "image/svg+xml",
      };
    }
    if (format === "cdxml") {
      return {
        content: await currentDocumentCdxmlForSave(),
        mimeType: "chemical/x-cdxml",
      };
    }
    if (format === "cdx") {
      return {
        content: await currentDocumentCdxForSave(),
        mimeType: CHEMDRAW_CDX_MIME,
      };
    }
    if (format === "sdf") {
      return {
        content: await currentDocumentSdfForSave(),
        mimeType: MDL_SDF_MIME,
      };
    }
    const json = await currentDocumentJsonForSave();
    if (format === "ccjs") {
      return {
        content: json,
        mimeType: CHEMCORE_TEXT_MIME,
      };
    }
    return {
      content: await compressChemcoreText(json),
      mimeType: CHEMCORE_COMPRESSED_MIME,
    };
  }

  async function saveCurrentDocument() {
    if (options.desktopFileHost?.available && options.state.currentFilePath) {
      return saveCurrentDocumentToDesktopPath(options.state.currentFilePath);
    }
    return saveCurrentDocumentAs();
  }

  async function saveCurrentDocumentNative() {
    if (options.desktopFileHost?.available) {
      return saveCurrentDocumentAs();
    }
    const suggestedName = `${saveAsBaseName()}${CHEMCORE_COMPRESSED_EXTENSION}`;
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{
          description: "ChemCore CCJZ",
          accept: { [CHEMCORE_COMPRESSED_MIME]: [CHEMCORE_COMPRESSED_EXTENSION] },
        }],
      });
      const payload = await savePayloadForFormat("ccjz");
      const writable = await handle.createWritable();
      await writable.write(payload.content);
      await writable.close();
      options.state.currentFileName = handle.name || suggestedName;
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      options.markCurrentDocumentSaved?.();
      return true;
    }
    const payload = await savePayloadForFormat("ccjz");
    downloadBinaryFile(payload.content, suggestedName, payload.mimeType);
    return true;
  }

  async function currentDocumentCdxmlForSave() {
    await options.finishActiveTextEditor(true);
    if (!options.state.editorEngine) {
      throw new Error("CDXML export is unavailable.");
    }
    return options.state.editorEngine.documentCdxml();
  }

  async function currentDocumentCdxForSave() {
    await options.finishActiveTextEditor(true);
    if (!options.state.editorEngine?.documentCdx) {
      throw new Error("CDX export is unavailable.");
    }
    return options.state.editorEngine.documentCdx();
  }

  async function currentDocumentSdfForSave() {
    await options.finishActiveTextEditor(true);
    if (!options.state.editorEngine?.documentSdf) {
      throw new Error("SDF export is unavailable.");
    }
    return options.state.editorEngine.documentSdf();
  }

  async function currentDocumentSvgForSave() {
    await options.finishActiveTextEditor(true);
    if (!options.state.editorEngine?.documentSvg) {
      throw new Error("SVG export is unavailable.");
    }
    return options.state.editorEngine.documentSvg();
  }

  async function currentOleEditPayloadForSave() {
    const chemcoreDocumentJson = await currentDocumentJsonForSave();
    let cdxml = null;
    try {
      cdxml = await currentDocumentCdxmlForSave();
    } catch (error) {
      console.warn("Failed to build OLE edit CDXML payload", error);
    }
    return {
      chemcoreFragmentJson: null,
      chemcoreDocumentJson,
      renderListJson: options.state.editorEngine?.renderListJson?.() || null,
      cdxml,
      svg: null,
      text: cdxml,
    };
  }

  async function saveCurrentDocumentCdxml() {
    const suggestedName = cdxmlFileNameForSave();
    if (options.desktopFileHost?.available) {
      const path = await options.desktopFileHost.chooseSavePath(suggestedName);
      if (!path) {
        return false;
      }
      return saveCurrentDocumentToDesktopPath(path, "cdxml");
    }
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{ description: "ChemDraw CDXML", accept: { "chemical/x-cdxml": [".cdxml"], "text/xml": [".cdxml"] } }],
      });
      const cdxml = await currentDocumentCdxmlForSave();
      const writable = await handle.createWritable();
      await writable.write(cdxml);
      await writable.close();
      options.state.currentFileName = handle.name || suggestedName;
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      return true;
    }
    const cdxml = await currentDocumentCdxmlForSave();
    downloadTextFile(cdxml, suggestedName, "chemical/x-cdxml");
    return true;
  }

  async function saveCurrentDocumentSvg() {
    if (!await confirmLossyExportIfNeeded("svg")) {
      return false;
    }
    const suggestedName = svgFileNameForSave();
    if (options.desktopFileHost?.available) {
      const path = await options.desktopFileHost.chooseSavePath(suggestedName);
      if (!path) {
        return false;
      }
      return saveCurrentDocumentToDesktopPath(path, "svg");
    }
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName,
        types: [{ description: "Scalable Vector Graphics", accept: { "image/svg+xml": [".svg"] } }],
      });
      const svg = await currentDocumentSvgForSave();
      const writable = await handle.createWritable();
      await writable.write(svg);
      await writable.close();
      return true;
    }
    const svg = await currentDocumentSvgForSave();
    downloadTextFile(svg, suggestedName, "image/svg+xml");
    return true;
  }

  async function currentDocumentPdfPreviewBase64ForSave() {
    return pdfPreviewBase64FromSvg(await currentDocumentSvgForSave());
  }

  async function saveCurrentDocumentPdf() {
    const suggestedName = pdfFileNameForSave();
    if (options.desktopFileHost?.available) {
      const path = await (
        options.desktopFileHost.chooseExportSavePath?.(suggestedName, "pdf")
        || options.desktopFileHost.chooseSavePath(suggestedName)
      );
      if (!path) {
        return false;
      }
      const pdfBase64 = await currentDocumentPdfPreviewBase64ForSave();
      await options.desktopFileHost.writeBase64(path, pdfBase64);
      return true;
    }
    const pdfBase64 = await currentDocumentPdfPreviewBase64ForSave();
    downloadBlobFile(
      new Blob([base64ToUint8(pdfBase64)], { type: "application/pdf" }),
      suggestedName,
    );
    return true;
  }

  async function saveCurrentDocumentEmf() {
    if (!options.desktopFileHost?.available || !options.desktopFileHost.exportEmf) {
      throw new Error("EMF export is available only in the Windows desktop app.");
    }
    if (!await confirmLossyExportIfNeeded("emf")) {
      return false;
    }
    await options.finishActiveTextEditor(true);
    if (!options.state.editorEngine?.renderListJson || !options.state.editorEngine?.renderBoundsJson) {
      throw new Error("EMF export is unavailable.");
    }
    const suggestedName = emfFileNameForSave();
    const path = await (
      options.desktopFileHost.chooseExportSavePath?.(suggestedName, "emf")
      || options.desktopFileHost.chooseSavePath(suggestedName)
    );
    if (!path) {
      return false;
    }
    const boundsJson = options.state.editorEngine.renderBoundsJson("document")
      || options.state.editorEngine.renderBoundsJson("all");
    await options.desktopFileHost.exportEmf(
      path,
      options.state.editorEngine.renderListJson(),
      boundsJson,
    );
    return true;
  }

  async function saveCurrentDocumentAs() {
    if (options.desktopFileHost?.available) {
      const path = await options.desktopFileHost.chooseSavePath(suggestedSaveAsName());
      if (!path) {
        return false;
      }
      return saveCurrentDocumentToDesktopPath(path);
    }
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName: suggestedSaveAsName(),
        types: [
          {
            description: "ChemCore CCJZ",
            accept: { [CHEMCORE_COMPRESSED_MIME]: [CHEMCORE_COMPRESSED_EXTENSION] },
          },
          {
            description: "ChemCore CCJS",
            accept: { [CHEMCORE_TEXT_MIME]: [CHEMCORE_TEXT_EXTENSION] },
          },
          { description: "ChemDraw CDXML", accept: { "chemical/x-cdxml": [".cdxml"], "text/xml": [".cdxml"] } },
          { description: "ChemDraw CDX", accept: { [CHEMDRAW_CDX_MIME]: [".cdx"], "application/x-cdx": [".cdx"] } },
          { description: "MDL SDfile", accept: { [MDL_SDF_MIME]: [".sdf", ".sd"], "chemical/x-mdl-sdfile": [".sdf", ".sd"] } },
          { description: "Scalable Vector Graphics", accept: { "image/svg+xml": [".svg"] } },
        ],
      });
      const format = saveFormatFromFileName(handle.name);
      if (!await confirmLossyExportIfNeeded(format)) {
        return false;
      }
      const { content } = await savePayloadForFormat(format);
      const writable = await handle.createWritable();
      await writable.write(content);
      await writable.close();
      if (format !== "svg") {
        options.state.currentFileName = handle.name || options.state.currentFileName;
        options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
        options.markCurrentDocumentSaved?.();
      }
      return true;
    }
    return saveCurrentDocumentNative();
  }

  function desktopFormatForPath(path, fallbackFormat = null) {
    return fallbackFormat || saveFormatFromFileName(path);
  }

  function isOleEditPath(path) {
    const fileName = fileNameFromPath(path).toLowerCase();
    return fileName.startsWith("chemcore-ole-edit-") && fileName.endsWith(".ccjs");
  }

  async function desktopContentForFormat(format) {
    if (format === "svg") {
      return currentDocumentSvgForSave();
    }
    if (format === "cdxml") {
      return currentDocumentCdxmlForSave();
    }
    if (format === "cdx") {
      return currentDocumentCdxmlForSave();
    }
    if (format === "sdf") {
      return currentDocumentSdfForSave();
    }
    return currentDocumentJsonForSave();
  }

  async function saveCurrentDocumentToDesktopPath(path, forcedFormat = null) {
    const format = desktopFormatForPath(path, forcedFormat);
    if (!await confirmLossyExportIfNeeded(format)) {
      return false;
    }
    let saved;
    if (isOleEditPath(path) && options.desktopFileHost.writeOleEditPayload) {
      saved = await options.desktopFileHost.writeOleEditPayload(path, await currentOleEditPayloadForSave());
      options.markCurrentDocumentOfficeSynced?.();
    } else {
      const content = await desktopContentForFormat(format);
      saved = isOleEditPath(path) && options.desktopFileHost.writeTransientPath
        ? await options.desktopFileHost.writeTransientPath(path, content)
        : await options.desktopFileHost.writePath(path, content, format);
    }
    if (format !== "svg" && format !== "emf") {
      options.state.currentFilePath = saved.path || path;
      options.state.currentFileName = saved.fileName || fileNameFromPath(path);
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      updateDocumentMeta();
      options.markCurrentDocumentSaved?.();
    } else {
      options.refreshCommandAvailability?.();
    }
    return true;
  }

  async function openDocumentFile(file) {
    if (!file) {
      return;
    }
    if (looksLikeCdxFile(file)) {
      await loadCdxDocumentIntoEditor(new Uint8Array(await file.arrayBuffer()), file.name || null, null);
      return;
    }
    const text = looksLikeCompressedChemcoreFile(file)
      ? await decompressChemcoreText(await file.arrayBuffer())
      : await file.text();
    if (looksLikeSdfFile(file, text)) {
      await loadSdfDocumentIntoEditor(text, file.name || null, null);
      return;
    }
    await openDocumentText(text, file.name || null, null, looksLikeCdxmlFile(file, text) ? "cdxml" : saveFormatFromFileName(file.name));
  }

  async function openDocumentText(text, fileName = null, filePath = null, format = null) {
    const resolvedFormat = format || saveFormatFromFileName(fileName);
    if (resolvedFormat === "cdxml" || looksLikeCdxmlFile({ name: fileName || "" }, text)) {
      await loadCdxmlDocumentIntoEditor(text, fileName, filePath);
      return;
    }
    if (resolvedFormat === "sdf" || looksLikeSdfFile({ name: fileName || "" }, text)) {
      await loadSdfDocumentIntoEditor(text, fileName, filePath);
      return;
    }
    await loadJsonDocumentIntoEditor(JSON.parse(text), fileName, filePath);
  }

  async function loadCdxDocumentIntoEditor(cdx, fileName = null, filePath = null) {
    await waitForRuntimeReady();
    await options.finishActiveTextEditor(false);
    const engine = await createLoadedEditorEngine(
      "cdx",
      (nextEngine) => nextEngine.loadDocumentCdx(cdx),
      { byteLength: cdx?.byteLength ?? cdx?.length ?? null, fileName, filePath },
    );
    await replaceEditorDocumentEngine(engine, fileName, filePath, "Imported CDX");
  }

  async function loadSdfDocumentIntoEditor(sdf, fileName = null, filePath = null) {
    if (typeof sdf !== "string") {
      throw new Error("SDF document text is unavailable.");
    }
    await waitForRuntimeReady();
    await options.finishActiveTextEditor(false);
    const engine = await createLoadedEditorEngine(
      "sdf",
      (nextEngine) => nextEngine.loadDocumentSdf(sdf),
      { textLength: sdf.length, fileName, filePath },
    );
    await replaceEditorDocumentEngine(engine, fileName, filePath, "Imported SDF");
  }

  async function openDocumentPath(path) {
    if (!options.desktopFileHost?.available || !path) {
      return;
    }
    const opened = await options.desktopFileHost.readPath(path);
    if (opened.format === "cdxml" || opened.format === "cdx") {
      await loadCdxmlDocumentIntoEditor(opened.text, opened.fileName || fileNameFromPath(path), opened.path || path);
      return;
    }
    if (opened.format === "sdf") {
      await loadSdfDocumentIntoEditor(opened.text, opened.fileName || fileNameFromPath(path), opened.path || path);
      return;
    }
    await loadJsonDocumentIntoEditor(JSON.parse(opened.text), opened.fileName || fileNameFromPath(path), opened.path || path);
  }

  async function loadCdxmlDocumentIntoEditor(cdxml, fileName = null, filePath = null) {
    if (typeof cdxml !== "string") {
      throw new Error("CDXML document text is unavailable.");
    }
    await waitForRuntimeReady();
    await options.finishActiveTextEditor(false);
    const engine = await createLoadedEditorEngine(
      "cdxml",
      (nextEngine) => nextEngine.loadDocumentCdxml(cdxml),
      { textLength: cdxml.length, fileName, filePath },
    );
    await replaceEditorDocumentEngine(engine, fileName, filePath, "Imported CDXML");
  }

  function isAbortError(error) {
    return error?.name === "AbortError";
  }

  async function chooseAndOpenDocument() {
    if (options.desktopFileHost?.available) {
      const path = await options.desktopFileHost.chooseOpenPath();
      if (path) {
        await openDocumentPath(path);
      }
      return;
    }
    if (window.showOpenFilePicker) {
      const [handle] = await window.showOpenFilePicker({
        multiple: false,
        types: chemcoreOpenAcceptTypes(),
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

  function fileNameFromPath(path) {
    return String(path || "").split(/[\\/]/).filter(Boolean).pop() || "Untitled";
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
    await options.finishActiveTextEditor(false);
    options.clearZoomHandoffs();
    options.viewerTitle.textContent = "Loading...";
    try {
      if (options.state.currentPath) {
        options.state.currentFileName = null;
        options.state.currentFilePath = null;
        const documentData = await loadDocument(options.state.currentPath);
        options.state.currentDocument = documentData;
        options.state.runtimeViewBox = options.pageViewBox(documentData.document.page);
        await options.syncCoreRenderListFromCurrentDocument();
      } else {
        options.state.coreRenderList = null;
        if (!options.state.editorEngine) {
          await options.resetEditorEngine();
        } else {
          await options.state.editorEngine.clearInteraction();
          await options.syncEngineToolState();
          options.syncDocumentFromEngine();
        }
      }
      options.renderSecondaryToolbar?.();
      const documentData = options.state.currentDocument;
      options.state.currentDocument = documentData;
      options.viewerTitle.textContent = documentData.document.title || options.state.currentPath;
      updateDocumentMeta();
      options.renderDocument();
      options.fitView();
      options.markCurrentDocumentSaved?.();
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
    loadCdxDocumentIntoEditor,
    loadSdfDocumentIntoEditor,
    loadJsonDocumentIntoEditor,
    openDocumentText,
    openDocumentFile,
    openDocumentPath,
    saveCurrentDocument,
    saveCurrentDocumentAs,
    saveCurrentDocumentCdxml,
    saveCurrentDocumentEmf,
    saveCurrentDocumentPdf,
    saveCurrentDocumentSvg,
    updateDocumentMeta,
  };
}

function base64ToUint8(value) {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}
