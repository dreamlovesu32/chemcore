import fs from "node:fs";

import {
  estimatedEditorCharWidth,
  lookupEditorGlyphProfile,
  normalizeSharedGlyphProfiles,
} from "../viewer/text_metrics.js";

const profiles = normalizeSharedGlyphProfiles(JSON.parse(fs.readFileSync("shared/glyph_profiles.json", "utf8")));
const catalog = JSON.parse(fs.readFileSync("shared/text_symbols.json", "utf8"));

for (const group of catalog.groups || []) {
  for (const character of Array.from(group.characters || "")) {
    const profile = lookupEditorGlyphProfile(profiles, character);
    if (!profile?.visible || !(profile.advanceEm > 0) || !(profile.inkBottomEm > profile.inkTopEm)) {
      throw new Error(`invalid profile for ${character} in ${group.id}: ${JSON.stringify(profile)}`);
    }
  }
}

const unknownCjkWidth = estimatedEditorCharWidth(profiles, "龘", 10);
if (unknownCjkWidth < 9.5) {
  throw new Error(`unknown CJK fallback is too narrow: ${unknownCjkWidth}`);
}

const perMilleWidth = estimatedEditorCharWidth(profiles, "‰", 10);
if (perMilleWidth < 11) {
  throw new Error(`per-mille profile is too narrow: ${perMilleWidth}`);
}

console.log("text symbol regression checks passed");
