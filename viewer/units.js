// Browser layout uses CSS pixels. We keep chemistry coordinates in points, then
// apply one app-level display scale when mapping points onto the interactive UI.
export const CSS_PX_PER_INCH = 96.0;
export const PT_PER_INCH = 72.0;
export const CM_PER_INCH = 2.54;
const DISPLAY_SCALE_STORAGE_KEY = "chemcore:display-scale";
const MIN_DISPLAY_SCALE = 0.25;
const MAX_DISPLAY_SCALE = 4.0;

export const CSS_PX_PER_PT = CSS_PX_PER_INCH / PT_PER_INCH;
export const PT_PER_CSS_PX = PT_PER_INCH / CSS_PX_PER_INCH;
export const PT_PER_CM = PT_PER_INCH / CM_PER_INCH;
export const CM_PER_PT = CM_PER_INCH / PT_PER_INCH;

export const devicePixelRatioValue = () =>
  Number(globalThis.window?.devicePixelRatio || globalThis.devicePixelRatio || 1) || 1;

function clampDisplayScale(value) {
  if (value == null || value === "") {
    return null;
  }
  const numeric = Number(value);
  return Number.isFinite(numeric)
    ? Math.max(MIN_DISPLAY_SCALE, Math.min(MAX_DISPLAY_SCALE, numeric))
    : null;
}

function displayScaleFromUrl() {
  try {
    const raw = globalThis.window
      ? new URL(globalThis.window.location.href).searchParams.get("displayScale")
      : null;
    return clampDisplayScale(raw);
  } catch {
    return null;
  }
}

function displayScaleFromStorage() {
  try {
    return clampDisplayScale(globalThis.localStorage?.getItem(DISPLAY_SCALE_STORAGE_KEY));
  } catch {
    return null;
  }
}

let displayScaleOverride = displayScaleFromUrl() ?? displayScaleFromStorage();

export function setDisplayScaleOverride(value) {
  displayScaleOverride = clampDisplayScale(value);
  try {
    if (displayScaleOverride == null) {
      globalThis.localStorage?.removeItem(DISPLAY_SCALE_STORAGE_KEY);
    } else {
      globalThis.localStorage?.setItem(DISPLAY_SCALE_STORAGE_KEY, String(displayScaleOverride));
    }
  } catch {
    // Display scaling must not depend on storage availability.
  }
  return displayScaleOverride;
}

export const isDesktopShell = () => Boolean(globalThis.__TAURI__?.core?.invoke);

export const defaultDisplayScale = () => {
  if (isDesktopShell()) {
    return 1.0;
  }
  return devicePixelRatioValue();
};

export const displayScaleValue = () => displayScaleOverride ?? defaultDisplayScale();
export const cssPxPerPtValue = () => CSS_PX_PER_PT * displayScaleValue();
export const ptPerCssPxValue = () => 1 / cssPxPerPtValue();

export const devicePxPerInch = () => CSS_PX_PER_INCH * devicePixelRatioValue();
export const devicePxPerPt = () => cssPxPerPtValue() * devicePixelRatioValue();

export const ptToCssPx = (pt) => pt * cssPxPerPtValue();
export const cssPxToPt = (px) => px * ptPerCssPxValue();

export const ptToDevicePx = (pt) => ptToCssPx(pt) * devicePixelRatioValue();
export const devicePxToPt = (px) => cssPxToPt(px / devicePixelRatioValue());

export const displayMetrics = () => {
  const devicePixelRatio = devicePixelRatioValue();
  const displayScale = displayScaleValue();
  const cssPxPerPt = cssPxPerPtValue();
  return {
    cssPxPerInch: CSS_PX_PER_INCH,
    baseCssPxPerPt: CSS_PX_PER_PT,
    cssPxPerPt,
    ptPerCssPx: 1 / cssPxPerPt,
    devicePixelRatio,
    displayScale,
    displayScalePercent: displayScale * 100,
    displayScaleSource: displayScaleOverride == null
      ? (isDesktopShell() ? "desktop-default" : "browser-device-pixel-ratio")
      : "override",
    devicePxPerInch: CSS_PX_PER_INCH * devicePixelRatio,
    devicePxPerPt: cssPxPerPt * devicePixelRatio,
  };
};

export const ptToPx = ptToCssPx;
export const pxToPt = cssPxToPt;

export const mapLengthArray = (values, convert) =>
  Array.isArray(values) ? values.map((value) => convert(Number(value || 0))) : values;
