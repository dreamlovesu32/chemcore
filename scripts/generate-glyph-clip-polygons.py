#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage
from skimage import measure


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FONT = Path(r"C:\Windows\Fonts\arial.ttf")
DEFAULT_GLYPH_PROFILES = ROOT / "shared" / "glyph_profiles.json"
DEFAULT_OUTPUT = ROOT / "shared" / "glyph_clip_polygons.json"

FONT_SIZE = 240
PADDING = 160
NATURAL_OUTSET_RATIO = 0.25
GREEN_INSET_RATIO = 0.22
CIRCLE_RADIUS_RATIO = 0.50

ANCHOR_MAP = {
    "A": [("midpoint", "c0", 1, 2), ("point", "c0", 0), ("point", "c0", 3)],
    "B": [("point", "c0", 1), ("point", "c0", 0)],
    "C": [],
    "D": [("point", "c0", 1), ("point", "c0", 0)],
    "E": [("point", "c0", 1), ("point", "c0", 2), ("point", "c0", 0), ("point", "c0", 11)],
    "F": [("point", "c0", 1), ("point", "c0", 2), ("point", "c0", 0)],
    "G": [],
    "H": [("point", "c0", 1), ("point", "c0", 6), ("point", "c0", 0), ("point", "c0", 7)],
    "I": [("midpoint", "c0", 1, 2), ("midpoint", "c0", 0, 3)],
    "J": [("midpoint", "c0", 9, 10)],
    "K": [("point", "c0", 1), ("point", "c0", 5), ("point", "c0", 7), ("point", "c0", 0)],
    "L": [("point", "c0", 1), ("point", "c0", 0), ("point", "c0", 5)],
    "M": [("point", "c0", 1), ("point", "c0", 9), ("point", "c0", 0), ("point", "c0", 10)],
    "N": [("point", "c0", 1), ("point", "c0", 5), ("point", "c0", 0), ("point", "c0", 6)],
    "O": [],
    "P": [("point", "c0", 1), ("point", "c0", 0)],
    "Q": [("midpoint", "c0", 2, 3)],
    "R": [("point", "c0", 1), ("point", "c0", 0), ("point", "c0", 14)],
    "S": [],
    "T": [("midpoint", "c0", 2, 3), ("midpoint", "c0", 4, 5), ("midpoint", "c0", 0, 7)],
    "U": [("midpoint", "c0", 11, 12), ("midpoint", "c0", 0, 1)],
    "V": [("point", "c0", 1), ("point", "c0", 9), ("midpoint", "c0", 0, 10)],
    "W": [("point", "c0", 1), ("point", "c0", 16), ("point", "c0", 0), ("point", "c0", 17)],
    "X": [("point", "c0", 2), ("point", "c0", 10), ("point", "c0", 0), ("point", "c0", 12)],
    "Y": [("point", "c0", 2), ("point", "c0", 10), ("midpoint", "c0", 0, 12)],
    "Z": [("point", "c0", 6), ("point", "c0", 7), ("point", "c0", 0), ("point", "c0", 12)],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--font", default=str(DEFAULT_FONT))
    parser.add_argument("--glyph-profiles", default=str(DEFAULT_GLYPH_PROFILES))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    return parser.parse_args()


def disk_kernel(radius: int) -> np.ndarray:
    yy, xx = np.ogrid[-radius : radius + 1, -radius : radius + 1]
    return (xx * xx + yy * yy) <= radius * radius


def render_mask(font: ImageFont.FreeTypeFont, ch: str) -> tuple[np.ndarray, tuple[int, int, int, int], int, tuple[int, int]]:
    left, top, right, bottom = font.getbbox(ch, anchor="ls")
    width = max(PADDING * 2 + (right - left) + 4, 512)
    height = max(PADDING * 2 + (bottom - top) + 4, 512)
    ox = PADDING - left
    oy = PADDING - top
    mask_image = Image.new("L", (width, height), 0)
    ImageDraw.Draw(mask_image).text((ox, oy), ch, font=font, fill=255, anchor="ls")
    mask = np.array(mask_image) >= 128
    ys, xs = np.nonzero(mask)
    if ys.size == 0 or xs.size == 0:
        raise RuntimeError(f"No visible pixels rendered for {ch!r}")
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    glyph_height = bbox[3] - bbox[1]
    return mask, bbox, glyph_height, (ox, oy)


def load_contours(ttfont: TTFont, ch: str) -> list[list[tuple[int, int, bool]]]:
    glyph_name = ttfont.getBestCmap()[ord(ch)]
    glyph = ttfont["glyf"][glyph_name]
    coords, end_pts, flags = glyph.getCoordinates(ttfont["glyf"])
    contours: list[list[tuple[int, int, bool]]] = []
    start = 0
    for end in end_pts:
        contour = []
        for (x, y), flag in zip(coords[start : end + 1], flags[start : end + 1]):
            contour.append((int(x), int(y), bool(flag & 1)))
        contours.append(contour)
        start = end + 1
    return contours


def contour_points_for_char(ttfont: TTFont, ch: str, font: ImageFont.FreeTypeFont, origin: tuple[int, int]) -> dict[str, dict[int, tuple[float, float]]]:
    contours = load_contours(ttfont, ch)
    scale = FONT_SIZE / ttfont["head"].unitsPerEm
    ox, oy = origin
    contour_points: dict[str, dict[int, tuple[float, float]]] = {}
    for contour_index, contour in enumerate(contours):
        contour_key = f"c{contour_index}"
        contour_points[contour_key] = {}
        for point_index, (x, y, on_curve) in enumerate(contour):
            if not on_curve:
                continue
            contour_points[contour_key][point_index] = (ox + x * scale, oy - y * scale)
    return contour_points


def collect_anchor_positions(contour_points: dict[str, dict[int, tuple[float, float]]], ch: str) -> list[tuple[float, float]]:
    anchors: list[tuple[float, float]] = []
    for spec in ANCHOR_MAP.get(ch, []):
        mode, contour_key = spec[0], spec[1]
        points = contour_points.get(contour_key)
        if not points:
            continue
        if mode == "point":
            index = spec[2]
            if index in points:
                anchors.append(points[index])
            continue
        a, b = spec[2], spec[3]
        if a in points and b in points:
            p1 = points[a]
            p2 = points[b]
            anchors.append(((p1[0] + p2[0]) * 0.5, (p1[1] + p2[1]) * 0.5))
    return anchors


def inset_anchor(anchor: tuple[float, float], center: tuple[float, float], glyph_w: float, glyph_h: float) -> tuple[float, float]:
    offset = glyph_h * GREEN_INSET_RATIO
    center_band = glyph_w * 0.12
    ax, ay = anchor
    cx, cy = center
    if ax < cx - center_band:
        dx = offset
    elif ax > cx + center_band:
        dx = -offset
    else:
        dx = 0.0
    dy = offset if ay < cy else -offset
    return (ax + dx, ay + dy)


def contour_from_mask(mask: np.ndarray) -> list[list[float]]:
    contours = measure.find_contours(mask.astype(float), 0.5)
    if not contours:
        raise RuntimeError("No contour found")
    contour = max(contours, key=lambda c: c.shape[0])
    approx = measure.approximate_polygon(contour, tolerance=0.75)
    if len(approx) < 3:
        approx = contour
    return [[float(col), float(row)] for row, col in approx]


def clip_polygon_for_char(ttfont: TTFont, font: ImageFont.FreeTypeFont, ch: str) -> dict:
    mask, bbox, glyph_height, origin = render_mask(font, ch)
    glyph_w = bbox[2] - bbox[0]
    natural = ndimage.binary_dilation(mask, structure=disk_kernel(max(1, int(round(glyph_height * NATURAL_OUTSET_RATIO)))))
    merged = natural.copy()

    if ch.isascii() and ch.isupper():
        contour_points = contour_points_for_char(ttfont, ch, font, origin)
        center = ((bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5)
        anchors = collect_anchor_positions(contour_points, ch)
        inset = [inset_anchor(anchor, center, glyph_w, glyph_height) for anchor in anchors]
        radius = glyph_height * CIRCLE_RADIUS_RATIO
        yy, xx = np.ogrid[:mask.shape[0], :mask.shape[1]]
        for cx, cy in inset:
            merged |= (xx - cx) ** 2 + (yy - cy) ** 2 <= radius * radius

    contour = contour_from_mask(merged)
    height = max(1.0, bbox[3] - bbox[1])
    center_x = (bbox[0] + bbox[2]) * 0.5
    normalized = [[(x - center_x) / height, (y - bbox[1]) / height] for x, y in contour]
    return {
        "bboxPx": list(bbox),
        "glyphHeightPx": glyph_height,
        "points": [[round(x, 6), round(y, 6)] for x, y in normalized],
    }


def main() -> None:
    args = parse_args()
    glyph_profiles = json.loads(Path(args.glyph_profiles).read_text(encoding="utf-8"))
    chars = [ch for ch, profile in glyph_profiles["specials"].items() if profile.get("visible", True)]
    font = ImageFont.truetype(str(args.font), FONT_SIZE)
    ttfont = TTFont(str(args.font))

    glyphs = {}
    for ch in chars:
        try:
            glyphs[ch] = clip_polygon_for_char(ttfont, font, ch)
        except Exception as error:  # noqa: BLE001
            print(f"skip {ch!r}: {error}")

    payload = {
        "version": 2,
        "sourceFont": str(args.font),
        "fontSizePx": FONT_SIZE,
        "coordinateSystem": "heightCentered",
        "naturalOutsetRatio": NATURAL_OUTSET_RATIO,
        "greenInsetRatio": GREEN_INSET_RATIO,
        "circleRadiusRatio": CIRCLE_RADIUS_RATIO,
        "glyphs": glyphs,
    }
    Path(args.output).write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
