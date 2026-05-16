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


def mask_bbox(mask: np.ndarray) -> dict | None:
    ys, xs = np.nonzero(mask)
    if xs.size == 0:
        return None
    left = int(xs.min())
    right = int(xs.max())
    top = int(ys.min())
    bottom = int(ys.max())
    return {
        "left": left,
        "top": top,
        "right": right,
        "bottom": bottom,
        "width": right - left + 1,
        "height": bottom - top + 1,
        "center_x": (left + right) / 2.0,
        "center_y": (top + bottom) / 2.0,
    }


def write_overlay(ours_mask: np.ndarray, ref_mask: np.ndarray, output_path: Path) -> None:
    if ours_mask.shape != ref_mask.shape:
        raise ValueError("overlay requires same-sized masks")
    image = np.full((*ours_mask.shape, 3), 255, dtype=np.uint8)
    overlap = ours_mask & ref_mask
    only_ours = ours_mask & ~ref_mask
    only_ref = ref_mask & ~ours_mask
    image[overlap] = [0, 0, 0]
    image[only_ours] = [255, 0, 0]
    image[only_ref] = [0, 0, 255]
    Image.fromarray(image, mode="RGB").save(output_path)


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare whole-image visible ink bounding boxes.")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("--threshold", type=int, default=740, help="RGB sum threshold for ink")
    parser.add_argument("--output", help="Write JSON report to file")
    parser.add_argument("--overlay", help="Write red/blue/black overlay PNG")
    args = parser.parse_args()

    ours_path = Path(args.ours_png)
    ref_path = Path(args.reference_png)
    ours_mask = load_mask(ours_path, args.threshold)
    ref_mask = load_mask(ref_path, args.threshold)

    report = {
        "ours_image": str(ours_path),
        "reference_image": str(ref_path),
        "threshold": args.threshold,
        "ours_bbox": mask_bbox(ours_mask),
        "reference_bbox": mask_bbox(ref_mask),
    }

    if report["ours_bbox"] and report["reference_bbox"]:
        ours = report["ours_bbox"]
        ref = report["reference_bbox"]
        report["bbox_diff"] = {
            "left": ours["left"] - ref["left"],
            "top": ours["top"] - ref["top"],
            "right": ours["right"] - ref["right"],
            "bottom": ours["bottom"] - ref["bottom"],
            "width": ours["width"] - ref["width"],
            "height": ours["height"] - ref["height"],
            "center_x": ours["center_x"] - ref["center_x"],
            "center_y": ours["center_y"] - ref["center_y"],
        }

    text = json.dumps(report, indent=2)
    if args.output:
        Path(args.output).write_text(text, encoding="utf8")
    else:
        print(text)

    if args.overlay:
        overlay_path = Path(args.overlay)
        overlay_path.parent.mkdir(parents=True, exist_ok=True)
        if ours_mask.shape == ref_mask.shape:
            write_overlay(ours_mask, ref_mask, overlay_path)


if __name__ == "__main__":
    main()
