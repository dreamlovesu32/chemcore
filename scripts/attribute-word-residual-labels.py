from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

from PIL import Image


def load_payload(path: Path) -> dict:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if "chemcoreDocumentJson" in payload:
        return json.loads(payload["chemcoreDocumentJson"])
    return payload


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


def bbox_from_points(points: list[list[float]]) -> list[float] | None:
    if not points:
        return None
    xs = [float(p[0]) for p in points]
    ys = [float(p[1]) for p in points]
    return [min(xs), min(ys), max(xs), max(ys)]


def transform_point(point: tuple[float, float], transform: dict) -> tuple[float, float]:
    x, y = point
    sx, sy = transform.get("scale", [1.0, 1.0])
    x *= float(sx)
    y *= float(sy)
    rotate_deg = float(transform.get("rotate", 0.0))
    if rotate_deg:
        theta = math.radians(rotate_deg)
        cos_t = math.cos(theta)
        sin_t = math.sin(theta)
        x, y = (x * cos_t - y * sin_t, x * sin_t + y * cos_t)
    tx, ty = transform.get("translate", [0.0, 0.0])
    return x + float(tx), y + float(ty)


def transform_box(box: list[float], transform: dict) -> list[float]:
    x1, y1, x2, y2 = box
    corners = [
        transform_point((x1, y1), transform),
        transform_point((x2, y1), transform),
        transform_point((x2, y2), transform),
        transform_point((x1, y2), transform),
    ]
    xs = [p[0] for p in corners]
    ys = [p[1] for p in corners]
    return [min(xs), min(ys), max(xs), max(ys)]


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


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Attribute same-shell residual pixels to payload node label boxes."
    )
    parser.add_argument("payload_json")
    parser.add_argument("role_report_json")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("output_json")
    parser.add_argument("--dx", type=int, default=0)
    parser.add_argument("--dy", type=int, default=0)
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--pad-px", type=int, default=3)
    args = parser.parse_args()

    document = load_payload(Path(args.payload_json))
    role_report = json.loads(Path(args.role_report_json).read_text(encoding="utf-16"))
    ours, width, height = load_mask(Path(args.ours_png), args.threshold)
    reference, ref_width, ref_height = load_mask(Path(args.reference_png), args.threshold)
    if (width, height) != (ref_width, ref_height):
        raise SystemExit("PNG sizes must match")

    resources_obj = document.get("resources", {})
    if isinstance(resources_obj, dict):
        resources = resources_obj
    else:
        resources = {resource["id"]: resource for resource in resources_obj}
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

    rows = []
    for obj in document.get("objects", []):
        if obj.get("type") != "molecule":
            continue
        resource = resources.get(obj["payload"]["resourceRef"])
        if not resource:
            continue
        transform = obj.get("transform", {})
        for node in resource["data"].get("nodes", []):
            label = node.get("label")
            if not label:
                continue
            glyph_points: list[list[float]] = []
            for polygon in label.get("glyphPolygons") or []:
                glyph_points.extend(polygon)
            local_box = bbox_from_points(glyph_points) or label.get("box")
            if not local_box:
                continue
            world_box = transform_box(local_box, transform)
            pixel_box = project_box(world_box, visible, width, height, args.pad_px)
            x1, y1, x2, y2 = pixel_box
            count = 0
            for x, y in residual_points:
                if x1 <= x <= x2 and y1 <= y <= y2:
                    count += 1
            area = (x2 - x1 + 1) * (y2 - y1 + 1)
            rows.append(
                {
                    "objectId": obj.get("id"),
                    "nodeId": node.get("id"),
                    "text": label.get("text", ""),
                    "fill": label.get("fill"),
                    "worldBox": world_box,
                    "pixelBox": pixel_box,
                    "residualCount": count,
                    "residualDensity": count / area if area else 0.0,
                }
            )

    rows.sort(key=lambda row: row["residualCount"], reverse=True)
    output = {
        "payload": str(Path(args.payload_json).resolve()),
        "ours_png": str(Path(args.ours_png).resolve()),
        "reference_png": str(Path(args.reference_png).resolve()),
        "dx": dx,
        "dy": dy,
        "padPx": args.pad_px,
        "residualPixelCount": len(residual_points),
        "labels": rows,
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2), encoding="utf-8")
    print(json.dumps({"residualPixelCount": len(residual_points), "topLabels": rows[:12]}, indent=2))


if __name__ == "__main__":
    main()
