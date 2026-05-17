from __future__ import annotations

import argparse
import json
from collections import deque
from pathlib import Path

from PIL import Image, ImageDraw


def load_mask(path: Path, threshold: int) -> tuple[Image.Image, list[list[bool]], int, int]:
    image = Image.open(path).convert("RGBA")
    width, height = image.size
    pixels = image.load()
    mask = [[False] * width for _ in range(height)]
    for y in range(height):
        row = mask[y]
        for x in range(width):
            r, g, b, a = pixels[x, y]
            row[x] = a > 0 and (r + g + b) < threshold
    return image, mask, width, height


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Find connected-component hotspots in the XOR of two rendered PNG masks."
    )
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("output_dir")
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--dx", type=int, default=0)
    parser.add_argument("--dy", type=int, default=0)
    parser.add_argument("--topn", type=int, default=20)
    args = parser.parse_args()

    ours_image, ours, width, height = load_mask(Path(args.ours_png), args.threshold)
    _, reference, ref_width, ref_height = load_mask(Path(args.reference_png), args.threshold)
    if (width, height) != (ref_width, ref_height):
        raise SystemExit("input PNGs must have the same dimensions")

    dx = args.dx
    dy = args.dy
    xor = [[False] * width for _ in range(height)]
    seen = [[False] * width for _ in range(height)]
    overlay = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    overlay_pixels = overlay.load()

    for y in range(height):
        ya = y + dy
        for x in range(width):
            xa = x + dx
            ours_value = ours[ya][xa] if 0 <= ya < height and 0 <= xa < width else False
            reference_value = reference[y][x]
            if ours_value and reference_value:
                overlay_pixels[x, y] = (0, 0, 0, 255)
            elif ours_value:
                overlay_pixels[x, y] = (220, 0, 0, 255)
            elif reference_value:
                overlay_pixels[x, y] = (0, 70, 220, 255)
            else:
                overlay_pixels[x, y] = (255, 255, 255, 255)
            xor[y][x] = ours_value ^ reference_value

    components: list[dict[str, float | int | list[int]]] = []
    for y in range(height):
        for x in range(width):
            if not xor[y][x] or seen[y][x]:
                continue
            queue = deque([(x, y)])
            seen[y][x] = True
            area = 0
            min_x = max_x = x
            min_y = max_y = y
            while queue:
                cx, cy = queue.popleft()
                area += 1
                min_x = min(min_x, cx)
                min_y = min(min_y, cy)
                max_x = max(max_x, cx)
                max_y = max(max_y, cy)
                for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                    if (
                        0 <= nx < width
                        and 0 <= ny < height
                        and xor[ny][nx]
                        and not seen[ny][nx]
                    ):
                        seen[ny][nx] = True
                        queue.append((nx, ny))
            components.append(
                {
                    "area": area,
                    "bbox": [min_x, min_y, max_x, max_y],
                    "width": max_x - min_x + 1,
                    "height": max_y - min_y + 1,
                    "cx": (min_x + max_x) / 2.0,
                    "cy": (min_y + max_y) / 2.0,
                }
            )

    components.sort(key=lambda component: int(component["area"]), reverse=True)
    top_components = components[: args.topn]

    draw = ImageDraw.Draw(overlay)
    for index, component in enumerate(top_components, start=1):
        x1, y1, x2, y2 = component["bbox"]  # type: ignore[index]
        draw.rectangle([x1, y1, x2, y2], outline=(0, 180, 0, 255), width=1)
        draw.text((x1, max(0, y1 - 10)), str(index), fill=(0, 140, 0, 255))

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    overlay.save(output_dir / "overlay-hotspots-topn.png")
    (output_dir / "hotspots-topn.json").write_text(
        json.dumps(top_components, indent=2), encoding="utf-8"
    )
    summary = {
        "ours_png": str(Path(args.ours_png).resolve()),
        "reference_png": str(Path(args.reference_png).resolve()),
        "threshold": args.threshold,
        "dx": dx,
        "dy": dy,
        "topn": args.topn,
        "componentCount": len(components),
    }
    (output_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
