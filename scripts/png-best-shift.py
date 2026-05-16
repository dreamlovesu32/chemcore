from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image


def load_mask(image_path: Path, threshold: int) -> np.ndarray:
    image = Image.open(image_path).convert("RGBA")
    rgba = np.asarray(image)
    rgb = rgba[..., :3].astype(np.int16)
    alpha = rgba[..., 3].astype(np.int16)
    return (alpha > 0) & (rgb.sum(axis=2) < threshold)


def best_shift(ours: np.ndarray, reference: np.ndarray, limit: int) -> dict:
    height = min(ours.shape[0], reference.shape[0])
    width = min(ours.shape[1], reference.shape[1])
    ours = ours[:height, :width]
    reference = reference[:height, :width]

    best: tuple[float, int, int] | None = None
    for dy in range(-limit, limit + 1):
        for dx in range(-limit, limit + 1):
            ours_y0 = max(0, dy)
            ref_y0 = max(0, -dy)
            ours_x0 = max(0, dx)
            ref_x0 = max(0, -dx)
            h = height - abs(dy)
            w = width - abs(dx)
            if h <= 0 or w <= 0:
                continue
            ours_crop = ours[ours_y0 : ours_y0 + h, ours_x0 : ours_x0 + w]
            ref_crop = reference[ref_y0 : ref_y0 + h, ref_x0 : ref_x0 + w]
            intersection = int(np.count_nonzero(ours_crop & ref_crop))
            union = int(np.count_nonzero(ours_crop | ref_crop))
            iou = 1.0 if union == 0 else intersection / union
            if best is None or iou > best[0]:
                best = (iou, dx, dy)

    assert best is not None
    return {
        "best_iou": best[0],
        "dx": best[1],
        "dy": best[2],
        "shared_width": width,
        "shared_height": height,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Find best translational overlap between two PNG ink masks.")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("--threshold", type=int, default=740, help="RGB sum threshold for ink")
    parser.add_argument("--limit", type=int, default=20, help="Maximum absolute dx/dy to search")
    parser.add_argument("--output", help="Write JSON report to file")
    args = parser.parse_args()

    report = {
        "ours_image": args.ours_png,
        "reference_image": args.reference_png,
        "threshold": args.threshold,
        "limit": args.limit,
    }
    report.update(
        best_shift(
            load_mask(Path(args.ours_png), args.threshold),
            load_mask(Path(args.reference_png), args.threshold),
            args.limit,
        )
    )
    text = json.dumps(report, indent=2)
    if args.output:
        Path(args.output).write_text(text, encoding="utf8")
    else:
        print(text)


if __name__ == "__main__":
    main()
