#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from fontTools.pens.recordingPen import RecordingPen
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_GLYPH_PROFILES = ROOT / "shared" / "glyph_profiles.json"
DEFAULT_OUTPUT = ROOT / "shared" / "glyph_outlines.json"

FONT_FACES = {
    "Arial": {
        "regular": r"C:\Windows\Fonts\arial.ttf",
        "bold": r"C:\Windows\Fonts\arialbd.ttf",
        "italic": r"C:\Windows\Fonts\ariali.ttf",
        "boldItalic": r"C:\Windows\Fonts\arialbi.ttf",
    },
    "Arial Narrow": {
        "regular": r"C:\Windows\Fonts\ARIALN.TTF",
        "bold": r"C:\Windows\Fonts\ARIALNB.TTF",
        "italic": r"C:\Windows\Fonts\ARIALNI.TTF",
        "boldItalic": r"C:\Windows\Fonts\ARIALNBI.TTF",
    },
    "Arial Black": {"regular": r"C:\Windows\Fonts\ariblk.ttf"},
    "Times New Roman": {
        "regular": r"C:\Windows\Fonts\times.ttf",
        "bold": r"C:\Windows\Fonts\timesbd.ttf",
        "italic": r"C:\Windows\Fonts\timesi.ttf",
        "boldItalic": r"C:\Windows\Fonts\timesbi.ttf",
    },
    "Georgia": {
        "regular": r"C:\Windows\Fonts\georgia.ttf",
        "bold": r"C:\Windows\Fonts\georgiab.ttf",
        "italic": r"C:\Windows\Fonts\georgiai.ttf",
        "boldItalic": r"C:\Windows\Fonts\georgiaz.ttf",
    },
    "Cambria": {
        "regular": r"C:\Windows\Fonts\cambria.ttc",
        "bold": r"C:\Windows\Fonts\cambriab.ttf",
        "italic": r"C:\Windows\Fonts\cambriai.ttf",
        "boldItalic": r"C:\Windows\Fonts\cambriaz.ttf",
    },
    "Calibri": {
        "regular": r"C:\Windows\Fonts\calibri.ttf",
        "bold": r"C:\Windows\Fonts\calibrib.ttf",
        "italic": r"C:\Windows\Fonts\calibrii.ttf",
        "boldItalic": r"C:\Windows\Fonts\calibriz.ttf",
    },
    "Courier New": {
        "regular": r"C:\Windows\Fonts\cour.ttf",
        "bold": r"C:\Windows\Fonts\courbd.ttf",
        "italic": r"C:\Windows\Fonts\couri.ttf",
        "boldItalic": r"C:\Windows\Fonts\courbi.ttf",
    },
    "Consolas": {
        "regular": r"C:\Windows\Fonts\consola.ttf",
        "bold": r"C:\Windows\Fonts\consolab.ttf",
        "italic": r"C:\Windows\Fonts\consolai.ttf",
        "boldItalic": r"C:\Windows\Fonts\consolaz.ttf",
    },
    "Verdana": {
        "regular": r"C:\Windows\Fonts\verdana.ttf",
        "bold": r"C:\Windows\Fonts\verdanab.ttf",
        "italic": r"C:\Windows\Fonts\verdanai.ttf",
        "boldItalic": r"C:\Windows\Fonts\verdanaz.ttf",
    },
    "Tahoma": {
        "regular": r"C:\Windows\Fonts\tahoma.ttf",
        "bold": r"C:\Windows\Fonts\tahomabd.ttf",
    },
    "Trebuchet MS": {
        "regular": r"C:\Windows\Fonts\trebuc.ttf",
        "bold": r"C:\Windows\Fonts\trebucbd.ttf",
        "italic": r"C:\Windows\Fonts\trebucit.ttf",
        "boldItalic": r"C:\Windows\Fonts\trebucbi.ttf",
    },
    "Symbol": {"regular": r"C:\Windows\Fonts\symbol.ttf"},
    "Segoe UI Symbol": {"regular": r"C:\Windows\Fonts\seguisym.ttf"},
    "SimSun": {"regular": r"C:\Windows\Fonts\simsun.ttc"},
}

FONT_ALIASES = {
    "Helvetica": "Arial",
    "TeX Gyre Heros": "Arial",
    "Noto Sans SC": "SimSun",
    "Noto Serif SC": "SimSun",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--glyph-profiles", default=str(DEFAULT_GLYPH_PROFILES))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    return parser.parse_args()


def point_em(point: tuple[float, float], units_per_em: int) -> list[float]:
    return [round(point[0] / units_per_em, 7), round(-point[1] / units_per_em, 7)]


def command_payload(op: str, points, units_per_em: int) -> dict:
    return {
        "op": op,
        "points": [point_em(point, units_per_em) for point in points if point is not None],
    }


def glyph_outline(ttfont: TTFont, ch: str, cmap: dict[int, str]) -> dict:
    glyph_name = cmap.get(ord(ch))
    if not glyph_name:
        raise RuntimeError(f"missing glyph for {ch!r}")
    glyph_set = ttfont.getGlyphSet()
    glyph = glyph_set[glyph_name]
    units_per_em = ttfont["head"].unitsPerEm
    bounds = ttfont["glyf"][glyph_name]

    pen = RecordingPen()
    glyph.draw(pen)
    commands = []
    for op, points in pen.value:
        if op == "moveTo":
            commands.append(command_payload("M", points, units_per_em))
        elif op == "lineTo":
            commands.append(command_payload("L", points, units_per_em))
        elif op == "qCurveTo":
            commands.append(command_payload("Q", points, units_per_em))
        elif op == "curveTo":
            commands.append(command_payload("C", points, units_per_em))
        elif op in {"closePath", "endPath"}:
            commands.append({"op": "Z", "points": []})

    return {
        "advanceEm": round(glyph.width / units_per_em, 8),
        "boundsEm": [
            round(bounds.xMin / units_per_em, 8),
            round(-bounds.yMax / units_per_em, 8),
            round(bounds.xMax / units_per_em, 8),
            round(-bounds.yMin / units_per_em, 8),
        ],
        "commands": commands,
    }


def glyph_kerning(ttfont: TTFont, chars: list[str], cmap: dict[int, str]) -> dict:
    """Return horizontal legacy-kern adjustments in em for generated characters.

    ChemDraw and the Windows text stack apply the face's kerning pairs before
    positioning glyphs. The supported Windows faces expose those pairs through
    a horizontal format-0 `kern` table. Keep only pairs whose two Unicode
    characters are in this manifest; glyph substitution is intentionally not
    kerned across font faces.
    """

    if "kern" not in ttfont:
        return {}
    units_per_em = ttfont["head"].unitsPerEm
    glyph_to_chars: dict[str, list[str]] = {}
    for ch in chars:
        glyph_name = cmap.get(ord(ch))
        if glyph_name:
            glyph_to_chars.setdefault(glyph_name, []).append(ch)

    pair_values: dict[tuple[str, str], int] = {}
    for subtable in ttfont["kern"].kernTables:
        # Bit 0 identifies horizontal kerning. Ignore cross-stream/minimum
        # subtables because they do not alter the inline advance.
        coverage = getattr(subtable, "coverage", 0)
        if not coverage & 0x01 or not hasattr(subtable, "kernTable"):
            continue
        override = bool(coverage & 0x08)
        for (left_glyph, right_glyph), value in subtable.kernTable.items():
            if left_glyph not in glyph_to_chars or right_glyph not in glyph_to_chars:
                continue
            key = (left_glyph, right_glyph)
            pair_values[key] = value if override else pair_values.get(key, 0) + value

    kerning: dict[str, dict[str, float]] = {}
    for (left_glyph, right_glyph), value in pair_values.items():
        if not value:
            continue
        adjustment = round(value / units_per_em, 8)
        for left in glyph_to_chars[left_glyph]:
            for right in glyph_to_chars[right_glyph]:
                kerning.setdefault(left, {})[right] = adjustment
    return kerning


def main() -> None:
    args = parse_args()
    glyph_profiles = json.loads(Path(args.glyph_profiles).read_text(encoding="utf-8"))
    chars = [
        ch
        for ch, profile in glyph_profiles["specials"].items()
        if profile.get("visible", True)
    ]
    # Final explicit character substitution when supported families do not
    # contain a requested codepoint.
    chars.append("□")
    families = {}
    for family, faces in FONT_FACES.items():
        generated_faces = {}
        for face, source in faces.items():
            path = Path(source)
            if not path.exists():
                print(f"skip missing font face {family}/{face}: {path}")
                continue
            ttfont = TTFont(str(path), fontNumber=0)
            if ttfont.getBestCmap() is None:
                print(f"skip unsupported font face {family}/{face}: no Unicode cmap")
                continue
            cmap = ttfont.getBestCmap()
            glyphs = {}
            missing = 0
            for ch in chars:
                try:
                    glyphs[ch] = glyph_outline(ttfont, ch, cmap)
                except Exception:  # noqa: BLE001
                    missing += 1
            if missing:
                print(f"{family}/{face}: {missing} glyphs use runtime substitution")
            generated_faces[face] = {
                "sourceFont": path.name,
                "glyphs": glyphs,
                "kerning": glyph_kerning(ttfont, chars, cmap),
            }
        if generated_faces:
            families[family] = {"faces": generated_faces}

    payload = {
        "version": 3,
        "aliases": FONT_ALIASES,
        "families": families,
    }
    Path(args.output).write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")),
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
