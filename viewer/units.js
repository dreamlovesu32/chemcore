// Browser layout uses CSS pixels. OS/browser scaling affects device pixels via
// devicePixelRatio, while CSS geometry remains 96 CSS px per CSS inch.
export const CSS_PX_PER_INCH = 96.0;
export const PT_PER_INCH = 72.0;
export const CM_PER_INCH = 2.54;

export const CSS_PX_PER_PT = CSS_PX_PER_INCH / PT_PER_INCH;
export const PT_PER_CSS_PX = PT_PER_INCH / CSS_PX_PER_INCH;
export const PT_PER_CM = PT_PER_INCH / CM_PER_INCH;
export const CM_PER_PT = CM_PER_INCH / PT_PER_INCH;

export const devicePixelRatioValue = () =>
  Number(globalThis.window?.devicePixelRatio || globalThis.devicePixelRatio || 1) || 1;

export const devicePxPerInch = () => CSS_PX_PER_INCH * devicePixelRatioValue();
export const devicePxPerPt = () => CSS_PX_PER_PT * devicePixelRatioValue();

export const ptToCssPx = (pt) => pt * CSS_PX_PER_PT;
export const cssPxToPt = (px) => px * PT_PER_CSS_PX;

export const ptToDevicePx = (pt) => ptToCssPx(pt) * devicePixelRatioValue();
export const devicePxToPt = (px) => cssPxToPt(px / devicePixelRatioValue());

export const displayMetrics = () => {
  const devicePixelRatio = devicePixelRatioValue();
  return {
    cssPxPerInch: CSS_PX_PER_INCH,
    cssPxPerPt: CSS_PX_PER_PT,
    ptPerCssPx: PT_PER_CSS_PX,
    devicePixelRatio,
    displayScalePercent: devicePixelRatio * 100,
    devicePxPerInch: CSS_PX_PER_INCH * devicePixelRatio,
    devicePxPerPt: CSS_PX_PER_PT * devicePixelRatio,
  };
};

export const ptToPx = ptToCssPx;
export const pxToPt = cssPxToPt;

export const mapLengthArray = (values, convert) =>
  Array.isArray(values) ? values.map((value) => convert(Number(value || 0))) : values;
