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

function editorGlyphLayoutConfig(sharedGlyphProfiles) {
  if (!sharedGlyphProfiles) {
    throw new Error("Shared glyph profiles have not loaded yet");
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
      subscriptScale: Number(manifest?.layout?.subscriptScale || 0.78),
      superscriptScale: Number(manifest?.layout?.superscriptScale || 0.78),
      subscriptShiftDownEm: Number(manifest?.layout?.subscriptShiftDownEm || 0.30),
      superscriptShiftUpEm: Number(manifest?.layout?.superscriptShiftUpEm || 0.28),
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
  const layout = editorGlyphLayoutConfig(sharedGlyphProfiles);
  if (script === "subscript") {
    return layout.subscriptShiftDownEm * baseFontSize;
  }
  if (script === "superscript") {
    return -layout.superscriptShiftUpEm * baseFontSize;
  }
  return 0;
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
