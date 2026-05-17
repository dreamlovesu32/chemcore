from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image


def load_mask(path: Path, threshold: int) -> tuple[list[list[bool]], int, int]:
    image = Image.open(path).convert("RGBA")
    width, height = image.size
    pixels = image.load()
    mask = [[False] * width for _ in range(height)]
    for y in range(height):
        row = mask[y]
        for x in range(width):
            r, g, b, a = pixels[x, y]
            row[x] = a > 0 and (r + g + b) < threshold
    return mask, width, height


def project_box(
    box: list[float],
    visible: list[float],
    width: int,
    height: int,
    pad_px: int,
) -> list[int]:
    min_x, min_y, max_x, max_y = visible
    source_width = max_x - min_x
    source_height = max_y - min_y
    scale = min(width / source_width, height / source_height)
    offset_x = (width - source_width * scale) / 2.0
    offset_y = (height - source_height * scale) / 2.0
    x1, y1, x2, y2 = box
    px1 = int((x1 - min_x) * scale + offset_x - pad_px)
    py1 = int((y1 - min_y) * scale + offset_y - pad_px)
    px2 = int((x2 - min_x) * scale + offset_x + pad_px)
    py2 = int((y2 - min_y) * scale + offset_y + pad_px)
    return [
        max(0, px1),
        max(0, py1),
        min(width - 1, px2),
        min(height - 1, py2),
    ]


def iter_regions(geometry: dict) -> list[dict]:
    regions: list[dict] = []
    for obj in geometry.get("topLevelObjects", []):
        box = obj.get("worldBox")
        if box:
            regions.append(
                {
                    "id": obj["id"],
                    "kind": f"top:{obj['type']}",
                    "label": obj["id"],
                    "box": box,
                }
            )
    molecule = geometry.get("moleculeComponentReport") or geometry.get("moleculeComponents")
    if molecule:
        for component in molecule.get("components", []):
            box = component.get("worldBox")
            if box:
                regions.append(
                    {
                        "id": component.get("roleGuess") or component.get("slot"),
                        "kind": "component",
                        "label": component.get("roleGuess") or component.get("slot"),
                        "box": box,
                    }
                )
    return regions


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Attribute word replay residual pixels to geometry boxes."
    )
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("role_report_json")
    parser.add_argument("geometry_json")
    parser.add_argument("output_json")
    parser.add_argument("--dx", type=int, default=0)
    parser.add_argument("--dy", type=int, default=0)
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--pad-px", type=int, default=3)
    args = parser.parse_args()

    ours, width, height = load_mask(Path(args.ours_png), args.threshold)
    reference, ref_width, ref_height = load_mask(Path(args.reference_png), args.threshold)
    if (width, height) != (ref_width, ref_height):
        raise SystemExit("PNG sizes must match")

    role_report = json.loads(Path(args.role_report_json).read_text(encoding="utf-16"))
    geometry = json.loads(Path(args.geometry_json).read_text(encoding="utf-8"))
    visible = role_report["visibleBoundsNoKnockout"]

    residual_points: list[tuple[int, int]] = []
    dx = args.dx
    dy = args.dy
    for y in range(height):
        for x in range(width):
            xa = x + dx
            ya = y + dy
            ours_value = ours[ya][xa] if 0 <= xa < width and 0 <= ya < height else False
            ref_value = reference[y][x]
            if ours_value ^ ref_value:
                residual_points.append((x, y))

    region_rows = []
    for region in iter_regions(geometry):
        px_box = project_box(region["box"], visible, width, height, args.pad_px)
        x1, y1, x2, y2 = px_box
        count = 0
        for x, y in residual_points:
            if x1 <= x <= x2 and y1 <= y <= y2:
                count += 1
        area = (x2 - x1 + 1) * (y2 - y1 + 1)
        region_rows.append(
            {
                **region,
                "pixelBox": px_box,
                "residualCount": count,
                "residualDensity": count / area if area else 0.0,
            }
        )

    region_rows.sort(key=lambda row: row["residualCount"], reverse=True)
    output = {
        "ours_png": str(Path(args.ours_png).resolve()),
        "reference_png": str(Path(args.reference_png).resolve()),
        "dx": dx,
        "dy": dy,
        "threshold": args.threshold,
        "padPx": args.pad_px,
        "residualPixelCount": len(residual_points),
        "regions": region_rows,
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2), encoding="utf-8")
    print(json.dumps({"residualPixelCount": len(residual_points), "top": region_rows[:10]}, indent=2))


if __name__ == "__main__":
    main()
