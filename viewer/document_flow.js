import {
  CHEMCORE_COMPRESSED_EXTENSION,
  CHEMCORE_COMPRESSED_MIME,
  CHEMCORE_TEXT_EXTENSION,
  CHEMCORE_TEXT_MIME,
  baseNameWithoutDocumentExtension,
  chemcoreOpenAcceptTypes,
  compressChemcoreText,
  decompressChemcoreText,
  documentTitleForFileName,
  downloadBinaryFile,
  downloadBlobFile,
  downloadTextFile,
  looksLikeCompressedChemcoreFile,
  looksLikeCdxmlFile,
  saveFormatFromFileName,
} from "./file_io.js";
import { pdfPreviewBase64FromSvg } from "./export_preview.js";

export function createDocumentFlow(options) {
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

  async function loadJsonDocumentIntoEditor(documentData, fileName = null, filePath = null) {
    await waitForRuntimeReady();
    validateChemcoreJsonDocument(documentData);
    await options.finishActiveTextEditor(false);
    options.state.currentPath = null;
    options.state.currentFileName = fileName;
    options.state.currentFilePath = filePath;
    await options.state.editorEngine?.free?.();
    options.state.editorEngine = options.engineHost.createEngineSession();
    await options.state.editorEngine.ready?.();
    options.resetCommandEngineRevision?.();
    options.state.lastEditFocusPoint = null;
    options.clearZoomHandoffs();
    await options.state.editorEngine.loadDocumentJson(JSON.stringify(documentData));
    await options.syncEngineToolState();
    await options.syncDocumentFromEngine();
    options.renderSecondaryToolbar?.();
    options.state.runtimeViewBox = null;
    options.viewerTitle.textContent = options.state.currentDocument?.document?.title || fileName || "Untitled";
    updateDocumentMeta();
    options.fitView();
    options.renderDocument();
    options.markCurrentDocumentSaved?.();
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
      await saveCurrentDocumentToDesktopPath(options.state.currentFilePath);
      return;
    }
    await saveCurrentDocumentAs();
  }

  async function saveCurrentDocumentNative() {
    if (options.desktopFileHost?.available) {
      await saveCurrentDocumentAs();
      return;
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
      return;
    }
    const payload = await savePayloadForFormat("ccjz");
    downloadBinaryFile(payload.content, suggestedName, payload.mimeType);
  }

  async function currentDocumentCdxmlForSave() {
    await options.finishActiveTextEditor(true);
    if (!options.state.editorEngine) {
      throw new Error("CDXML export is unavailable.");
    }
    return options.state.editorEngine.documentCdxml();
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
        return;
      }
      await saveCurrentDocumentToDesktopPath(path, "cdxml");
      return;
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
      return;
    }
    const cdxml = await currentDocumentCdxmlForSave();
    downloadTextFile(cdxml, suggestedName, "chemical/x-cdxml");
  }

  async function saveCurrentDocumentSvg() {
    const suggestedName = svgFileNameForSave();
    if (options.desktopFileHost?.available) {
      const path = await options.desktopFileHost.chooseSavePath(suggestedName);
      if (!path) {
        return;
      }
      await saveCurrentDocumentToDesktopPath(path, "svg");
      return;
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
      return;
    }
    const svg = await currentDocumentSvgForSave();
    downloadTextFile(svg, suggestedName, "image/svg+xml");
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
        return;
      }
      const pdfBase64 = await currentDocumentPdfPreviewBase64ForSave();
      await options.desktopFileHost.writeBase64(path, pdfBase64);
      return;
    }
    const pdfBase64 = await currentDocumentPdfPreviewBase64ForSave();
    downloadBlobFile(
      new Blob([base64ToUint8(pdfBase64)], { type: "application/pdf" }),
      suggestedName,
    );
  }

  async function saveCurrentDocumentEmf() {
    if (!options.desktopFileHost?.available || !options.desktopFileHost.exportEmf) {
      throw new Error("EMF export is available only in the Windows desktop app.");
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
      return;
    }
    const boundsJson = options.state.editorEngine.renderBoundsJson("document")
      || options.state.editorEngine.renderBoundsJson("all");
    await options.desktopFileHost.exportEmf(
      path,
      options.state.editorEngine.renderListJson(),
      boundsJson,
    );
  }

  async function saveCurrentDocumentAs() {
    if (options.desktopFileHost?.available) {
      const path = await options.desktopFileHost.chooseSavePath(`${saveAsBaseName()}${CHEMCORE_COMPRESSED_EXTENSION}`);
      if (!path) {
        return;
      }
      await saveCurrentDocumentToDesktopPath(path);
      return;
    }
    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName: `${saveAsBaseName()}${CHEMCORE_COMPRESSED_EXTENSION}`,
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
          { description: "Scalable Vector Graphics", accept: { "image/svg+xml": [".svg"] } },
        ],
      });
      const format = saveFormatFromFileName(handle.name);
      const { content } = await savePayloadForFormat(format);
      const writable = await handle.createWritable();
      await writable.write(content);
      await writable.close();
      if (format !== "svg") {
        options.state.currentFileName = handle.name || options.state.currentFileName;
        options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
        options.markCurrentDocumentSaved?.();
      }
      return;
    }
    await saveCurrentDocumentNative();
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
    return currentDocumentJsonForSave();
  }

  async function saveCurrentDocumentToDesktopPath(path, forcedFormat = null) {
    const format = desktopFormatForPath(path, forcedFormat);
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
    if (format !== "svg") {
      options.state.currentFilePath = saved.path || path;
      options.state.currentFileName = saved.fileName || fileNameFromPath(path);
      options.viewerTitle.textContent = options.state.currentDocument?.document?.title || options.state.currentFileName || "Untitled";
      updateDocumentMeta();
      options.markCurrentDocumentSaved?.();
    } else {
      options.refreshCommandAvailability?.();
    }
  }

  async function openDocumentFile(file) {
    if (!file) {
      return;
    }
    const text = looksLikeCompressedChemcoreFile(file)
      ? await decompressChemcoreText(await file.arrayBuffer())
      : await file.text();
    await openDocumentText(text, file.name || null, null, looksLikeCdxmlFile(file, text) ? "cdxml" : saveFormatFromFileName(file.name));
  }

  async function openDocumentText(text, fileName = null, filePath = null, format = null) {
    const resolvedFormat = format || saveFormatFromFileName(fileName);
    if (resolvedFormat === "cdxml" || looksLikeCdxmlFile({ name: fileName || "" }, text)) {
      await loadCdxmlDocumentIntoEditor(text, fileName, filePath);
      return;
    }
    await loadJsonDocumentIntoEditor(JSON.parse(text), fileName, filePath);
  }

  async function openDocumentPath(path) {
    if (!options.desktopFileHost?.available || !path) {
      return;
    }
    const opened = await options.desktopFileHost.readPath(path);
    if (opened.format === "cdxml") {
      await loadCdxmlDocumentIntoEditor(opened.text, opened.fileName || fileNameFromPath(path), opened.path || path);
      return;
    }
    await loadJsonDocumentIntoEditor(JSON.parse(opened.text), opened.fileName || fileNameFromPath(path), opened.path || path);
  }

  async function loadCdxmlDocumentIntoEditor(cdxml, fileName = null, filePath = null) {
    await waitForRuntimeReady();
    await options.finishActiveTextEditor(false);
    options.state.currentPath = null;
    options.state.currentFileName = fileName;
    options.state.currentFilePath = filePath;
    await options.state.editorEngine?.free?.();
    options.state.editorEngine = options.engineHost.createEngineSession();
    await options.state.editorEngine.ready?.();
    options.resetCommandEngineRevision?.();
    options.state.lastEditFocusPoint = null;
    options.clearZoomHandoffs();
    await options.state.editorEngine.loadDocumentCdxml(cdxml);
    await options.syncEngineToolState();
    await options.syncDocumentFromEngine();
    options.state.runtimeViewBox = null;
    options.viewerTitle.textContent = options.state.currentDocument?.document?.title || fileName || "Imported CDXML";
    updateDocumentMeta();
    options.fitView();
    options.renderDocument();
    options.markCurrentDocumentSaved?.();
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
