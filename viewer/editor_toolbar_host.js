import {
  ARROW_TOOL_ICON_TYPES,
  BOND_TOOL_ICON_TYPES,
  ORBITAL_TOOL_ICON_PHASES,
  ORBITAL_TOOL_ICON_STYLES,
  ORBITAL_TOOL_ICON_TEMPLATES,
  SHAPE_TOOL_ICON_KINDS,
  SHAPE_TOOL_ICON_STYLES,
  SHAPE_TOOL_STYLE_KINDS,
  SYMBOL_TOOL_ICON_TYPES,
  TEXT_FORMAT_ICON_TYPES,
  renderSecondaryToolbarHtml,
  syncPrimaryToolButtons,
} from "./toolbar.js";
import { createTextSymbolPalette } from "./text_symbol_palette.js";

export function createEditorToolbarHost(options) {
  const {
    state,
    editorState,
    secondaryToolbar,
    parseEngineJson,
  } = options;
  const insertTextSymbol = (...args) => options.insertTextSymbol(...args);
  const selectElementFromQuickPalette = (...args) => options.selectElementFromQuickPalette(...args);
  const handleQuickPaletteModeChange = (...args) => options.handleQuickPaletteModeChange(...args);
  let textSymbolPalette = null;

  function toolbarBondIconWidths() {
    const styles = getComputedStyle(document.documentElement);
    const thinPx = parseFloat(styles.getPropertyValue("--cc-icon-stroke-thin")) || 1.65;
    const thickPx = parseFloat(styles.getPropertyValue("--cc-icon-stroke-thick")) || 4.6;
    const iconPx = parseFloat(styles.getPropertyValue("--icon-svg-size")) || 30;
    const scale = 24 / Math.max(1, iconPx);
    return {
      thin: thinPx * scale,
      thick: thickPx * scale,
      key: `${thinPx}:${thickPx}:${iconPx}`,
    };
  }

  function refreshBondToolIcons() {
    const iconSvg = state.editorEngine?.bondToolIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const widths = toolbarBondIconWidths();
    const hasCompleteIconSet = BOND_TOOL_ICON_TYPES.every((type) => editorState.bondIconSvgs?.[type]);
    if (editorState.bondIconCacheKey === widths.key && hasCompleteIconSet) {
      return;
    }
    const icons = {};
    for (const type of BOND_TOOL_ICON_TYPES) {
      icons[type] = iconSvg.call(state.editorEngine, type, widths.thin, widths.thick);
    }
    editorState.bondIconSvgs = icons;
    editorState.bondIconCacheKey = widths.key;
  }

  function refreshChainToolIcon() {
    const iconSvg = state.editorEngine?.chainToolIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const widths = toolbarBondIconWidths();
    const cacheKey = `kernel-chain-v1:${widths.key}`;
    if (editorState.chainIconCacheKey === cacheKey && editorState.chainIconSvg) {
      return;
    }
    editorState.chainIconSvg = normalizeKernelChainIconSvg(
      iconSvg.call(state.editorEngine, widths.thin),
    );
    editorState.chainIconCacheKey = cacheKey;
  }

  function refreshArrowToolIcons() {
    const iconSvg = state.editorEngine?.arrowToolIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const hasCompleteIconSet = ARROW_TOOL_ICON_TYPES.every((type) => editorState.arrowIconSvgs?.[type]);
    if (editorState.arrowIconCacheKey === "kernel-arrow-v1" && hasCompleteIconSet) {
      return;
    }
    const icons = {};
    for (const type of ARROW_TOOL_ICON_TYPES) {
      icons[type] = normalizeKernelArrowIconSvg(iconSvg.call(state.editorEngine, type), type);
    }
    editorState.arrowIconSvgs = icons;
    editorState.arrowIconCacheKey = "kernel-arrow-v1";
  }

  function refreshTextFormatIcons() {
    const iconSvg = state.editorEngine?.textFormatIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const hasCompleteIconSet = TEXT_FORMAT_ICON_TYPES.every((type) => editorState.textIconSvgs?.[type]);
    if (editorState.textIconCacheKey === "kernel-text-v1" && hasCompleteIconSet) {
      return;
    }
    const icons = {};
    for (const type of TEXT_FORMAT_ICON_TYPES) {
      icons[type] = iconSvg.call(state.editorEngine, type);
    }
    editorState.textIconSvgs = icons;
    editorState.textIconCacheKey = "kernel-text-v1";
  }

  function refreshShapeToolIcons() {
    const iconSvg = state.editorEngine?.shapeToolIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const hasCompleteIconSet = SHAPE_TOOL_ICON_KINDS.every((kind) => (
      shapeToolIconStylesForKind(kind).every((style) => editorState.shapeIconSvgs?.[`${kind}:${style}`])
    ));
    if (editorState.shapeIconCacheKey === "kernel-shape-v2" && hasCompleteIconSet) {
      return;
    }
    const icons = {};
    for (const kind of SHAPE_TOOL_ICON_KINDS) {
      for (const style of shapeToolIconStylesForKind(kind)) {
        const key = `${kind}:${style}`;
        icons[key] = normalizeKernelShapeIconSvg(iconSvg.call(state.editorEngine, kind, style), key);
      }
    }
    editorState.shapeIconSvgs = icons;
    editorState.shapeIconCacheKey = "kernel-shape-v2";
  }

  function refreshSymbolToolIcons() {
    const iconSvg = state.editorEngine?.symbolToolIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const hasCompleteIconSet = SYMBOL_TOOL_ICON_TYPES.every((type) => editorState.symbolIconSvgs?.[type]);
    if (editorState.symbolIconCacheKey === "kernel-symbol-v1" && hasCompleteIconSet) {
      return;
    }
    const icons = {};
    for (const type of SYMBOL_TOOL_ICON_TYPES) {
      icons[type] = normalizeKernelSymbolIconSvg(iconSvg.call(state.editorEngine, type), type);
    }
    editorState.symbolIconSvgs = icons;
    editorState.symbolIconCacheKey = "kernel-symbol-v1";
  }

  function refreshOrbitalToolIcons() {
    const iconSvg = state.editorEngine?.orbitalToolIconSvg;
    if (typeof iconSvg !== "function") {
      return;
    }
    const hasCompleteIconSet = ORBITAL_TOOL_ICON_TEMPLATES.every((template) => (
      ORBITAL_TOOL_ICON_STYLES.every((style) => (
        ORBITAL_TOOL_ICON_PHASES.every((phase) => (
          editorState.orbitalIconSvgs?.[`${template}:${style}:${phase}`]
        ))
      ))
    ));
    if (editorState.orbitalIconCacheKey === "kernel-orbital-v1" && hasCompleteIconSet) {
      return;
    }
    const icons = {};
    for (const template of ORBITAL_TOOL_ICON_TEMPLATES) {
      for (const style of ORBITAL_TOOL_ICON_STYLES) {
        for (const phase of ORBITAL_TOOL_ICON_PHASES) {
          const key = `${template}:${style}:${phase}`;
          icons[key] = normalizeKernelOrbitalIconSvg(
            iconSvg.call(state.editorEngine, template, style, phase),
            key,
          );
        }
      }
    }
    editorState.orbitalIconSvgs = icons;
    editorState.orbitalIconCacheKey = "kernel-orbital-v1";
  }

  function shapeToolIconStylesForKind(kind) {
    return SHAPE_TOOL_STYLE_KINDS.includes(kind) ? SHAPE_TOOL_ICON_STYLES : ["solid"];
  }

  function normalizeKernelShapeIconSvg(svg, key) {
    if (!svg) {
      return "";
    }
    const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
    return addClassToSvg(svg, "cc-kernel-shape-icon")
      .replace(/\bid="([^"]+)"/g, `id="shape-icon-${safeKey}-$1"`)
      .replace(/url\(#([^)]+)\)/g, `url(#shape-icon-${safeKey}-$1)`);
  }

  function normalizeKernelArrowIconSvg(svg, key) {
    if (!svg) {
      return "";
    }
    const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
    return addClassToSvg(svg, "cc-kernel-arrow-icon")
      .replace(/\bid="([^"]+)"/g, `id="arrow-icon-${safeKey}-$1"`)
      .replace(/url\(#([^)]+)\)/g, `url(#arrow-icon-${safeKey}-$1)`);
  }

  function normalizeKernelSymbolIconSvg(svg, key) {
    if (!svg) {
      return "";
    }
    const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
    return addClassToSvg(svg, "cc-kernel-symbol-icon")
      .replace(/\bid="([^"]+)"/g, `id="symbol-icon-${safeKey}-$1"`)
      .replace(/url\(#([^)]+)\)/g, `url(#symbol-icon-${safeKey}-$1)`);
  }

  function normalizeKernelOrbitalIconSvg(svg, key) {
    if (!svg) {
      return "";
    }
    const safeKey = String(key).replace(/[^a-zA-Z0-9_-]/g, "-");
    return addClassToSvg(svg, "cc-kernel-orbital-icon")
      .replace(/\bid="([^"]+)"/g, `id="orbital-icon-${safeKey}-$1"`)
      .replace(/url\(#([^)]+)\)/g, `url(#orbital-icon-${safeKey}-$1)`);
  }

  function normalizeKernelChainIconSvg(svg) {
    if (!svg) {
      return "";
    }
    return addClassToSvg(svg, "cc-kernel-chain-icon");
  }

  function addClassToSvg(svg, className) {
    if (/\bclass="/.test(svg)) {
      return svg.replace(/\bclass="([^"]*)"/, `class="$1 ${className}"`);
    }
    return svg.replace("<svg ", `<svg class="${className}" `);
  }

  function renderSecondaryToolbar() {
    if (!secondaryToolbar) {
      return;
    }
    refreshBondToolIcons();
    refreshChainToolIcon();
    refreshArrowToolIcons();
    refreshTextFormatIcons();
    refreshShapeToolIcons();
    refreshSymbolToolIcons();
    refreshOrbitalToolIcons();
    editorState.documentColors = currentDocumentColors();
    editorState.colorPalette = currentToolbarColorPalette(editorState.documentColors);
    editorState.elementPalette = currentElementPalette();
    secondaryToolbar.innerHTML = renderSecondaryToolbarHtml(editorState);
    textSymbolPalette?.setElementPayload?.(editorState.elementPalette);
    syncPrimaryToolButtons(editorState, document);
  }

  function currentDocumentColors() {
    if (typeof state.editorEngine?.documentColorsJson === "function") {
      const engineColorsJson = state.editorEngine.documentColorsJson();
      if (typeof engineColorsJson === "string") {
        const engineColors = parseEngineJson(engineColorsJson, null);
        if (Array.isArray(engineColors)) {
          return engineColors;
        }
      }
    }
    return [];
  }

  function currentToolbarColorPalette(documentColors = []) {
    if (typeof state.editorEngine?.toolbarColorPaletteJson === "function") {
      const paletteJson = state.editorEngine.toolbarColorPaletteJson(JSON.stringify(documentColors));
      if (typeof paletteJson === "string") {
        return parseEngineJson(paletteJson, null);
      }
    }
    return null;
  }

  function currentElementPalette() {
    if (typeof state.editorEngine?.elementPaletteJson === "function") {
      const paletteJson = state.editorEngine.elementPaletteJson();
      if (typeof paletteJson === "string") {
        return elementPaletteWithCurrentSelection(parseEngineJson(paletteJson, null));
      }
    }
    return null;
  }

  function elementPaletteWithCurrentSelection(payload) {
    if (!payload || !editorState.elementSymbol) {
      return payload;
    }
    const elements = Array.isArray(payload.elements) ? payload.elements : [];
    const current = elements.find((element) => element?.symbol === editorState.elementSymbol);
    return current ? { ...payload, current } : payload;
  }

  function syncTextSymbolPaletteFromEngine() {
    if (typeof state.editorEngine?.textSymbolPaletteJson !== "function") {
      ensureTextSymbolPalette();
      return;
    }
    const payload = parseEngineJson(state.editorEngine.textSymbolPaletteJson(), null);
    if (!payload) {
      ensureTextSymbolPalette();
      return;
    }
    ensureTextSymbolPalette(payload);
  }

  function ensureTextSymbolPalette(payload = null) {
    const elementPayload = currentElementPalette();
    if (textSymbolPalette) {
      if (payload) {
        textSymbolPalette.setPayload(payload);
      }
      if (elementPayload) {
        textSymbolPalette.setElementPayload?.(elementPayload);
      }
      return;
    }
    textSymbolPalette = createTextSymbolPalette({
      mount: document.body,
      payload,
      elementPayload,
      onSelect: insertTextSymbol,
      onElementSelect: selectElementFromQuickPalette,
      onModeChange: handleQuickPaletteModeChange,
    });
  }


  return {
    renderSecondaryToolbar,
    currentDocumentColors,
    currentToolbarColorPalette,
    currentElementPalette,
    syncTextSymbolPaletteFromEngine,
    ensureTextSymbolPalette,
    syncPrimaryToolButtons: () => syncPrimaryToolButtons(editorState, document),
  };
}
