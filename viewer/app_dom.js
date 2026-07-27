import { chemsemaOpenAcceptString } from "./file_io.js";

export function createAppDomRefs(rootDocument = document) {
  const sampleSelect = rootDocument.getElementById("sample-select");
  const reloadButton = rootDocument.getElementById("reload-button");
  const fitButton = rootDocument.getElementById("fit-button");
  const toggleMolecules = rootDocument.getElementById("toggle-molecules");
  const toggleLines = rootDocument.getElementById("toggle-lines");
  const toggleTexts = rootDocument.getElementById("toggle-texts");
  const docMeta = rootDocument.getElementById("doc-meta");
  const viewerTitle = rootDocument.getElementById("viewer-title");
  const viewerStats = rootDocument.getElementById("viewer-stats");
  const viewerSvg = rootDocument.getElementById("viewer-svg");
  const viewerContainer = rootDocument.getElementById("viewer-container");
  const secondaryToolbar = rootDocument.getElementById("secondary-toolbar");
  const selectionChemistrySummary = rootDocument.getElementById("selection-chemistry-summary");
  const paperLayoutModeButton = rootDocument.getElementById("paper-layout-mode-button");
  const desktopTitlebar = rootDocument.getElementById("desktop-titlebar");
  const documentTabsRoot = rootDocument.getElementById("document-tabs");
  const documentStyleButton = rootDocument.getElementById("document-style-button");
  const documentStyleMenu = rootDocument.getElementById("document-style-menu");
  const zoomInput = rootDocument.getElementById("zoom-input");

  const openFileInput = rootDocument.createElement("input");
  openFileInput.type = "file";
  openFileInput.accept = chemsemaOpenAcceptString();
  openFileInput.className = "visually-hidden";
  rootDocument.body.appendChild(openFileInput);

  const imageFileInput = rootDocument.createElement("input");
  imageFileInput.type = "file";
  imageFileInput.accept = ".png,.jpg,.jpeg,.gif,.bmp,image/png,image/jpeg,image/gif,image/bmp";
  imageFileInput.multiple = true;
  imageFileInput.className = "visually-hidden";
  rootDocument.body.appendChild(imageFileInput);

  const textEditorLayer = rootDocument.createElement("div");
  textEditorLayer.className = "text-editor-layer";
  viewerContainer?.appendChild(textEditorLayer);

  return {
    sampleSelect,
    reloadButton,
    fitButton,
    toggleMolecules,
    toggleLines,
    toggleTexts,
    docMeta,
    viewerTitle,
    viewerStats,
    viewerSvg,
    viewerContainer,
    secondaryToolbar,
    selectionChemistrySummary,
    paperLayoutModeButton,
    desktopTitlebar,
    documentTabsRoot,
    documentStyleButton,
    documentStyleMenu,
    zoomInput,
    openFileInput,
    imageFileInput,
    textEditorLayer,
  };
}
