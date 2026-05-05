#!/usr/bin/env python3
"""Merge generated text-symbol glyph profiles into shared/glyph_profiles.json."""

from __future__ import annotations

import json
from pathlib import Path

from fontTools.ttLib import TTFont
from PIL import ImageFont


ROOT = Path(__file__).resolve().parents[1]
CATALOG_PATH = ROOT / "shared" / "text_symbols.json"
PROFILE_PATH = ROOT / "shared" / "glyph_profiles.json"

FONT_CANDIDATES = [
    Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    Path("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf"),
    Path("/usr/share/fonts/truetype/arphic-gbsn00lp/gbsn00lp.ttf"),
    Path("/usr/share/fonts/opentype/ipaexfont-gothic/ipaexg.ttf"),
    Path("/usr/share/fonts/truetype/noto/NotoMono-Regular.ttf"),
]

FONT_SIZE = 1000
DEFAULT_PAD_EM = 0.09


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: dict) -> None:
    lines = ["{"]
    layout = value["layout"]
    lines.extend(
        [
            '  "layout": {',
            f'    "trackingEm": {json_number(layout["trackingEm"])},',
            f'    "subscriptScale": {json_number(layout["subscriptScale"])},',
            f'    "superscriptScale": {json_number(layout["superscriptScale"])},',
            f'    "subscriptShiftDownEm": {json_number(layout["subscriptShiftDownEm"])},',
            f'    "superscriptShiftUpEm": {json_number(layout["superscriptShiftUpEm"])}',
            "  },",
            '  "defaults": {',
        ]
    )
    defaults = value["defaults"]
    default_items = list(defaults.items())
    for index, (key, profile) in enumerate(default_items):
        suffix = "," if index + 1 < len(default_items) else ""
        lines.append(f'    "{key}": {profile_inline(profile)}{suffix}')
    lines.extend(["  },", '  "specials": {'])
    special_items = list(value["specials"].items())
    for index, (key, profile) in enumerate(special_items):
        suffix = "," if index + 1 < len(special_items) else ""
        lines.append(f'    {json.dumps(key, ensure_ascii=False)}: {profile_inline(profile)}{suffix}')
    lines.extend(["  }", "}"])
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def json_number(value) -> str:
    return json.dumps(value, ensure_ascii=False)


def profile_inline(profile: dict) -> str:
    return (
        "{ "
        f'"shape": {json.dumps(profile["shape"], ensure_ascii=False)}, '
        f'"advanceEm": {json_number(profile["advanceEm"])}, '
        f'"inkLeftEm": {json_number(profile["inkLeftEm"])}, '
        f'"inkTopEm": {json_number(profile["inkTopEm"])}, '
        f'"inkRightEm": {json_number(profile["inkRightEm"])}, '
        f'"inkBottomEm": {json_number(profile["inkBottomEm"])}, '
        f'"padXEm": {json_number(profile["padXEm"])}, '
        f'"padYEm": {json_number(profile["padYEm"])}, '
        f'"visible": {json.dumps(profile["visible"])}'
        " }"
    )


def catalog_characters(catalog: dict) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for group in catalog.get("groups", []):
        for character in str(group.get("characters", "")):
            if character not in seen:
                seen.add(character)
                out.append(character)
    return out


def font_cmap(path: Path) -> set[int]:
    font = TTFont(path, lazy=True)
    codepoints: set[int] = set()
    for table in font["cmap"].tables:
        codepoints.update(table.cmap.keys())
    return codepoints


def load_fonts() -> list[tuple[Path, set[int], ImageFont.FreeTypeFont]]:
    out = []
    for path in FONT_CANDIDATES:
        if not path.exists():
            continue
        out.append((path, font_cmap(path), ImageFont.truetype(str(path), FONT_SIZE)))
    if not out:
        raise RuntimeError("no usable font candidates found")
    return out


def font_for_character(fonts: list[tuple[Path, set[int], ImageFont.FreeTypeFont]], ch: str):
    codepoint = ord(ch)
    for _path, cmap, font in fonts:
        if codepoint in cmap:
            return font
    return fonts[0][2]


def clamp(value: float, low: float, high: float) -> float:
    return min(max(value, low), high)


def round_em(value: float) -> float:
    return round(value, 4)


def measured_profile(ch: str, font: ImageFont.FreeTypeFont) -> dict:
    if ch.isspace():
        advance = float(font.getlength(ch)) / FONT_SIZE
        return {
            "shape": "rect",
            "advanceEm": round_em(max(advance, 0.28)),
            "inkLeftEm": 0.0,
            "inkTopEm": 0.0,
            "inkRightEm": 0.0,
            "inkBottomEm": 0.0,
            "padXEm": 0.0,
            "padYEm": 0.0,
            "visible": False,
        }

    left, top, right, bottom = font.getbbox(ch, anchor="ls")
    advance = float(font.getlength(ch)) / FONT_SIZE
    width = max((right - left) / FONT_SIZE, 0.01)
    height = max((bottom - top) / FONT_SIZE, 0.01)

    return {
        "shape": generated_shape(ch, width, height),
        "advanceEm": round_em(max(advance, width)),
        "inkLeftEm": round_em(left / FONT_SIZE),
        "inkTopEm": round_em(top / FONT_SIZE),
        "inkRightEm": round_em(right / FONT_SIZE),
        "inkBottomEm": round_em(bottom / FONT_SIZE),
        "padXEm": DEFAULT_PAD_EM,
        "padYEm": DEFAULT_PAD_EM,
        "visible": True,
    }


def generated_shape(ch: str, width: float, height: float) -> str:
    if ch in "·•∙●○◦∘°" or (0.75 <= width / max(height, 0.01) <= 1.25 and ch in "ΟοΟΩΩ"):
        return "ellipse"
    if ch in "≤<‹«":
        return "rect-cut-top-right"
    if ch in "≥>›»":
        return "rect-cut-top-left"
    return "rect"


def fallback_profile(ch: str) -> dict:
    code = ord(ch)
    if ch.isspace():
        advance, top, right, bottom, visible = 0.28, 0.0, 0.0, 0.0, False
    elif is_cjk_or_fullwidth(code):
        advance, top, right, bottom, visible = 1.0, -0.86, 1.0, 0.14, True
    elif is_math_or_arrow(code):
        advance, top, right, bottom, visible = 0.84, -0.74, 0.84, 0.06, True
    elif code in (0x2030, 0x2031):
        advance, top, right, bottom, visible = 1.34, -0.74, 1.34, 0.06, True
    else:
        advance, top, right, bottom, visible = 0.62, -0.74, 0.62, 0.08, True
    return {
        "shape": "rect",
        "advanceEm": advance,
        "inkLeftEm": 0.0,
        "inkTopEm": top,
        "inkRightEm": right,
        "inkBottomEm": bottom,
        "padXEm": DEFAULT_PAD_EM if visible else 0.0,
        "padYEm": DEFAULT_PAD_EM if visible else 0.0,
        "visible": visible,
    }


def is_cjk_or_fullwidth(code: int) -> bool:
    return (
        0x1100 <= code <= 0x11FF
        or 0x2E80 <= code <= 0xA4CF
        or 0xAC00 <= code <= 0xD7AF
        or 0xF900 <= code <= 0xFAFF
        or 0xFE10 <= code <= 0xFE6F
        or 0xFF00 <= code <= 0xFFEF
        or 0x20000 <= code <= 0x2FA1F
    )


def is_math_or_arrow(code: int) -> bool:
    return 0x2190 <= code <= 0x21FF or 0x2200 <= code <= 0x22FF or 0x27F0 <= code <= 0x27FF


def main() -> None:
    catalog = load_json(CATALOG_PATH)
    manifest = load_json(PROFILE_PATH)
    specials = manifest.setdefault("specials", {})
    fonts = load_fonts()

    added = 0
    for ch in catalog_characters(catalog):
        if ch in specials:
            continue
        try:
            profile = measured_profile(ch, font_for_character(fonts, ch))
        except Exception:
            profile = fallback_profile(ch)
        specials[ch] = profile
        added += 1

    write_json(PROFILE_PATH, manifest)
    print(f"merged {added} generated glyph profiles into {PROFILE_PATH}")


if __name__ == "__main__":
    main()
