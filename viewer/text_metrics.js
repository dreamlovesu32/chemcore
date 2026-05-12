function normalizeSharedGlyphProfile(profile) {
  return {
    shape: String(profile?.shape || "rect"),
    advanceEm: Number(profile?.advanceEm || 0),
    inkLeftEm: Number(profile?.inkLeftEm || 0),
    inkTopEm: Number(profile?.inkTopEm || 0),
    inkRightEm: Number(profile?.inkRightEm || 0),
    inkBottomEm: Number(profile?.inkBottomEm || 0),
    padXEm: Number(profile?.padXEm || 0),
    padYEm: Number(profile?.padYEm || 0),
    visible: profile?.visible !== false,
  };
}

function isAsciiGlyphCode(character, lowerBound, upperBound) {
  if (!character || Array.from(character).length !== 1) {
    return false;
  }
  const code = character.codePointAt(0);
  return Number.isFinite(code) && code >= lowerBound && code <= upperBound;
}

const DEFAULT_GLYPH_LAYOUT = {
  trackingEm: 0,
  subscriptScale: 0.75,
  superscriptScale: 0.75,
  subscriptShiftDownEm: 0.22,
  superscriptShiftUpEm: 0.392,
};
const CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM = 0.215;

function editorGlyphLayoutConfig(sharedGlyphProfiles) {
  if (!sharedGlyphProfiles) {
    return DEFAULT_GLYPH_LAYOUT;
  }
  return sharedGlyphProfiles.layout;
}

function defaultUpperGlyphProfile(sharedGlyphProfiles) {
  return sharedGlyphProfiles.defaults.upper;
}

function defaultLowerGlyphProfile(sharedGlyphProfiles) {
  return sharedGlyphProfiles.defaults.lower;
}

function defaultDigitGlyphProfile(sharedGlyphProfiles) {
  return sharedGlyphProfiles.defaults.digit;
}

function defaultPunctuationGlyphProfile(sharedGlyphProfiles) {
  return sharedGlyphProfiles.defaults.punctuation;
}

function fallbackRectGlyphProfile(advanceEm, inkTopEm, inkRightEm, inkBottomEm) {
  return {
    shape: "rect",
    advanceEm,
    inkLeftEm: 0,
    inkTopEm,
    inkRightEm,
    inkBottomEm,
    padXEm: 0.09,
    padYEm: 0.09,
    visible: true,
  };
}

function invisibleWhitespaceGlyphProfile() {
  return {
    shape: "rect",
    advanceEm: 0.28,
    inkLeftEm: 0,
    inkTopEm: 0,
    inkRightEm: 0,
    inkBottomEm: 0,
    padXEm: 0,
    padYEm: 0,
    visible: false,
  };
}

function isSingleCodePoint(character) {
  return !!character && Array.from(character).length === 1;
}

function characterCodePoint(character) {
  return isSingleCodePoint(character) ? character.codePointAt(0) : NaN;
}

function isCjkOrFullwidth(character) {
  const code = characterCodePoint(character);
  return Number.isFinite(code) && (
    (code >= 0x1100 && code <= 0x11ff)
    || (code >= 0x2e80 && code <= 0xa4cf)
    || (code >= 0xac00 && code <= 0xd7af)
    || (code >= 0xf900 && code <= 0xfaff)
    || (code >= 0xfe10 && code <= 0xfe6f)
    || (code >= 0xff00 && code <= 0xffef)
    || (code >= 0x20000 && code <= 0x2fa1f)
  );
}

function isMathOrArrowSymbol(character) {
  const code = characterCodePoint(character);
  return Number.isFinite(code) && (
    (code >= 0x2190 && code <= 0x21ff)
    || (code >= 0x2200 && code <= 0x22ff)
    || (code >= 0x27f0 && code <= 0x27ff)
  );
}

function isGreekOrExtendedLetter(character) {
  return isSingleCodePoint(character) && /^\p{L}$/u.test(character);
}

function isUppercaseLetter(character) {
  return isSingleCodePoint(character) && character.toLocaleUpperCase() === character && character.toLocaleLowerCase() !== character;
}

function inferredGlyphProfile(character) {
  if (!character) {
    return null;
  }
  if (/\s/u.test(character)) {
    return invisibleWhitespaceGlyphProfile();
  }
  if (isCjkOrFullwidth(character)) {
    return fallbackRectGlyphProfile(1.0, -0.86, 1.0, 0.14);
  }
  if (isMathOrArrowSymbol(character)) {
    return fallbackRectGlyphProfile(0.84, -0.74, 0.84, 0.06);
  }
  if (character === "‰" || character === "‱") {
    return fallbackRectGlyphProfile(1.34, -0.74, 1.34, 0.06);
  }
  if (isGreekOrExtendedLetter(character)) {
    return isUppercaseLetter(character)
      ? fallbackRectGlyphProfile(0.72, -0.74, 0.72, 0.04)
      : fallbackRectGlyphProfile(0.62, -0.62, 0.62, 0.08);
  }
  return fallbackRectGlyphProfile(0.62, -0.74, 0.62, 0.08);
}

export function textCodePoints(text) {
  return Array.from(String(text || ""));
}

export function textLength(text) {
  return textCodePoints(text).length;
}

export function sliceTextByOffset(text, start = 0, end = undefined) {
  return textCodePoints(text).slice(start, end).join("");
}

export function normalizeSharedGlyphProfiles(manifest) {
  const specials = Object.create(null);
  for (const [key, value] of Object.entries(manifest?.specials || {})) {
    if (Array.from(key).length !== 1) {
      throw new Error(`Glyph profile key must be exactly one character: ${JSON.stringify(key)}`);
    }
    specials[key] = normalizeSharedGlyphProfile(value);
  }
  return {
    layout: {
      trackingEm: Number(manifest?.layout?.trackingEm || 0),
      subscriptScale: Number(manifest?.layout?.subscriptScale || 0.75),
      superscriptScale: Number(manifest?.layout?.superscriptScale || 0.75),
      subscriptShiftDownEm: Number(manifest?.layout?.subscriptShiftDownEm || 0.22),
      superscriptShiftUpEm: Number(manifest?.layout?.superscriptShiftUpEm || 0.392),
    },
    defaults: {
      upper: normalizeSharedGlyphProfile(manifest?.defaults?.upper),
      lower: normalizeSharedGlyphProfile(manifest?.defaults?.lower),
      digit: normalizeSharedGlyphProfile(manifest?.defaults?.digit),
      punctuation: normalizeSharedGlyphProfile(manifest?.defaults?.punctuation),
    },
    specials,
  };
}

export function lookupEditorGlyphProfile(sharedGlyphProfiles, character) {
  if (!sharedGlyphProfiles) {
    throw new Error("Shared glyph profiles have not loaded yet");
  }
  if (character && Object.hasOwn(sharedGlyphProfiles.specials, character)) {
    return sharedGlyphProfiles.specials[character];
  }
  if (isAsciiGlyphCode(character, 65, 90)) {
    return defaultUpperGlyphProfile(sharedGlyphProfiles);
  }
  if (isAsciiGlyphCode(character, 97, 122)) {
    return defaultLowerGlyphProfile(sharedGlyphProfiles);
  }
  if (isAsciiGlyphCode(character, 48, 57)) {
    return defaultDigitGlyphProfile(sharedGlyphProfiles);
  }
  if (!isAsciiGlyphCode(character, 0x21, 0x7e)) {
    return inferredGlyphProfile(character);
  }
  return defaultPunctuationGlyphProfile(sharedGlyphProfiles);
}

export function editorScriptScale(sharedGlyphProfiles, script) {
  const layout = editorGlyphLayoutConfig(sharedGlyphProfiles);
  if (script === "subscript") {
    return layout.subscriptScale;
  }
  if (script === "superscript") {
    return layout.superscriptScale;
  }
  return 1;
}

export function editorScriptBaselineShift(sharedGlyphProfiles, baseFontSize, script) {
  return editorScriptBaselineShiftEm(sharedGlyphProfiles, script) * baseFontSize;
}

export function editorScriptBaselineShiftEm(sharedGlyphProfiles, script, fontWeight = 400) {
  const layout = editorGlyphLayoutConfig(sharedGlyphProfiles);
  if (script === "subscript") {
    return Number(fontWeight) >= 600
      ? CHEMDRAW_BOLD_SUBSCRIPT_SHIFT_DOWN_EM
      : layout.subscriptShiftDownEm;
  }
  if (script === "superscript") {
    return -layout.superscriptShiftUpEm;
  }
  return 0;
}

export function editorSvgScriptBaselineShift(sharedGlyphProfiles, runFontSize, script, fontWeight = 400) {
  return -editorScriptBaselineShiftEm(sharedGlyphProfiles, script, fontWeight) * runFontSize;
}

export function editorChargeSignBaselineAdjustment(sharedGlyphProfiles, profile, baseFontSize, script) {
  if (script !== "subscript" && script !== "superscript") {
    return 0;
  }
  const digit = defaultDigitGlyphProfile(sharedGlyphProfiles);
  const digitCenter = (digit.inkTopEm + digit.inkBottomEm) * 0.5;
  const signCenter = (profile.inkTopEm + profile.inkBottomEm) * 0.5;
  return (digitCenter - signCenter) * baseFontSize * editorScriptScale(sharedGlyphProfiles, script);
}

export function estimatedEditorCharWidth(sharedGlyphProfiles, character, fontSize) {
  if (!character) {
    return sharedGlyphProfiles ? fontSize * defaultUpperGlyphProfile(sharedGlyphProfiles).advanceEm : fontSize * 0.72;
  }
  if (!sharedGlyphProfiles) {
    if (/\s/.test(character)) {
      return fontSize * 0.34;
    }
    if (/[.,;:!?()[\]/+-]/.test(character)) {
      return fontSize * 0.42;
    }
    return fontSize * 0.62;
  }
  return lookupEditorGlyphProfile(sharedGlyphProfiles, character).advanceEm * fontSize;
}

export function estimateTextRunsWidth(sharedGlyphProfiles, runs, fallbackFontSize, defaultFontSize) {
  let width = 0;
  let lineWidth = 0;
  for (const run of runs || []) {
    const baseFontSize = Number(run.fontSize || fallbackFontSize || defaultFontSize);
    const runFontSize = Math.max(7, baseFontSize * editorScriptScale(sharedGlyphProfiles, run.script));
    for (const ch of String(run.text || "")) {
      if (ch === "\n") {
        width = Math.max(width, lineWidth);
        lineWidth = 0;
        continue;
      }
      lineWidth += estimatedEditorCharWidth(sharedGlyphProfiles, ch, runFontSize);
    }
  }
  return Math.max(width, lineWidth);
}
