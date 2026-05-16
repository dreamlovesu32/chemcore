from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image


@dataclass
class Region:
    left: int
    top: int
    right: int
    bottom: int


@dataclass
class LineBox:
    top: int
    bottom: int
    left: int
    right: int

    @property
    def width(self) -> int:
        return self.right - self.left + 1

    @property
    def height(self) -> int:
        return self.bottom - self.top + 1

    @property
    def center_x(self) -> float:
        return (self.left + self.right) / 2.0

    @property
    def center_y(self) -> float:
        return (self.top + self.bottom) / 2.0


def parse_region(text: str) -> Region:
    left, top, right, bottom = [int(float(part.strip())) for part in text.split(",")]
    return Region(left, top, right, bottom)


def load_mask(image_path: Path, region: Region, threshold: int) -> np.ndarray:
    image = Image.open(image_path).convert("RGBA")
    crop = image.crop((region.left, region.top, region.right, region.bottom))
    rgba = np.asarray(crop)
    rgb = rgba[..., :3].astype(np.int16)
    alpha = rgba[..., 3].astype(np.int16)
    ink = (alpha > 0) & (rgb.sum(axis=2) < threshold)
    return ink


def collect_line_boxes(mask: np.ndarray, min_pixels: int, join_gap: int) -> list[LineBox]:
    row_counts = mask.sum(axis=1)
    active_rows = np.nonzero(row_counts >= min_pixels)[0]
    if active_rows.size == 0:
        return []

    groups: list[tuple[int, int]] = []
    start = int(active_rows[0])
    prev = int(active_rows[0])
    for row in active_rows[1:]:
        row = int(row)
        if row - prev <= join_gap:
            prev = row
            continue
        groups.append((start, prev))
        start = row
        prev = row
    groups.append((start, prev))

    boxes: list[LineBox] = []
    for top, bottom in groups:
        submask = mask[top : bottom + 1, :]
        ys, xs = np.nonzero(submask)
        if xs.size == 0:
            continue
        boxes.append(
            LineBox(
                top=top,
                bottom=bottom,
                left=int(xs.min()),
                right=int(xs.max()),
            )
        )
    return boxes


def build_report(ours: list[LineBox], reference: list[LineBox], region: Region) -> dict:
    rows = []
    count = max(len(ours), len(reference))
    for index in range(count):
        ours_box = ours[index] if index < len(ours) else None
        ref_box = reference[index] if index < len(reference) else None
        row = {
            "row": index,
            "ours": None,
            "reference": None,
            "diff": None,
        }
        if ours_box:
            row["ours"] = {
                "top": ours_box.top + region.top,
                "bottom": ours_box.bottom + region.top,
                "left": ours_box.left + region.left,
                "right": ours_box.right + region.left,
                "width": ours_box.width,
                "height": ours_box.height,
                "center_x": ours_box.center_x + region.left,
                "center_y": ours_box.center_y + region.top,
            }
        if ref_box:
            row["reference"] = {
                "top": ref_box.top + region.top,
                "bottom": ref_box.bottom + region.top,
                "left": ref_box.left + region.left,
                "right": ref_box.right + region.left,
                "width": ref_box.width,
                "height": ref_box.height,
                "center_x": ref_box.center_x + region.left,
                "center_y": ref_box.center_y + region.top,
            }
        if ours_box and ref_box:
            row["diff"] = {
                "top": (ours_box.top - ref_box.top),
                "bottom": (ours_box.bottom - ref_box.bottom),
                "left": (ours_box.left - ref_box.left),
                "right": (ours_box.right - ref_box.right),
                "width": (ours_box.width - ref_box.width),
                "height": (ours_box.height - ref_box.height),
                "center_x": (ours_box.center_x - ref_box.center_x),
                "center_y": (ours_box.center_y - ref_box.center_y),
            }
        rows.append(row)
    return {
        "region": vars(region),
        "ours_count": len(ours),
        "reference_count": len(reference),
        "rows": rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare rendered text line ink boxes.")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("--region", required=True, help="left,top,right,bottom in image pixels")
    parser.add_argument("--threshold", type=int, default=740, help="RGB sum threshold for ink")
    parser.add_argument("--min-pixels", type=int, default=8, help="minimum dark pixels per row")
    parser.add_argument("--join-gap", type=int, default=2, help="merge adjacent active row groups within this gap")
    parser.add_argument("--output", help="write JSON report to file")
    args = parser.parse_args()

    region = parse_region(args.region)
    ours_mask = load_mask(Path(args.ours_png), region, args.threshold)
    ref_mask = load_mask(Path(args.reference_png), region, args.threshold)
    ours_boxes = collect_line_boxes(ours_mask, args.min_pixels, args.join_gap)
    ref_boxes = collect_line_boxes(ref_mask, args.min_pixels, args.join_gap)
    report = build_report(ours_boxes, ref_boxes, region)
    text = json.dumps(report, indent=2)

    if args.output:
        Path(args.output).write_text(text, encoding="utf8")
    else:
        print(text)


if __name__ == "__main__":
    main()
