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


def load_role_report(path: Path) -> dict:
    raw = path.read_bytes()
    if raw.startswith(b"\xff\xfe") or raw.startswith(b"\xfe\xff"):
        return json.loads(path.read_text(encoding="utf-16"))
    return json.loads(path.read_text(encoding="utf-8"))


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


def point_in_box(point: tuple[int, int], box: list[int]) -> bool:
    x, y = point
    x1, y1, x2, y2 = box
    return x1 <= x <= x2 and y1 <= y <= y2


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Partition full-doc residual pixels into molecule labels vs non-label molecule residual."
    )
    parser.add_argument("payload_json")
    parser.add_argument("role_report_json")
    parser.add_argument("geometry_json")
    parser.add_argument("ours_png")
    parser.add_argument("reference_png")
    parser.add_argument("output_json")
    parser.add_argument("--dx", type=int, default=0)
    parser.add_argument("--dy", type=int, default=0)
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--pad-px", type=int, default=3)
    args = parser.parse_args()

    document = load_payload(Path(args.payload_json))
    role_report = load_role_report(Path(args.role_report_json))
    geometry = json.loads(Path(args.geometry_json).read_text(encoding="utf-8"))

    ours, width, height = load_mask(Path(args.ours_png), args.threshold)
    reference, ref_width, ref_height = load_mask(Path(args.reference_png), args.threshold)
    if (width, height) != (ref_width, ref_height):
        raise SystemExit("PNG sizes must match")

    visible = role_report["visibleBoundsNoKnockout"]
    dx = args.dx
    dy = args.dy
    residual_points: list[tuple[int, int]] = []
    for y in range(height):
        for x in range(width):
            xa = x + dx
            ya = y + dy
            ours_value = ours[ya][xa] if 0 <= xa < width and 0 <= ya < height else False
            ref_value = reference[y][x]
            if ours_value ^ ref_value:
                residual_points.append((x, y))

    resources_obj = document.get("resources", {})
    resources = resources_obj if isinstance(resources_obj, dict) else {r["id"]: r for r in resources_obj}

    label_rows: list[dict] = []
    label_boxes: list[list[int]] = []
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
            label_boxes.append(pixel_box)
            label_rows.append(
                {
                    "objectId": obj.get("id"),
                    "nodeId": node.get("id"),
                    "text": label.get("text", ""),
                    "fill": label.get("fill"),
                    "worldBox": world_box,
                    "pixelBox": pixel_box,
                }
            )

    component_rows: list[dict] = []
    molecule_components = geometry.get("moleculeComponents", {}).get("components", [])
    for component in molecule_components:
        pixel_box = project_box(component["worldBox"], visible, width, height, args.pad_px)
        component_rows.append(
            {
                "name": component.get("roleGuess") or component.get("slot") or "component",
                "worldBox": component["worldBox"],
                "pixelBox": pixel_box,
            }
        )

    label_union_count = 0
    component_union_count = 0
    component_non_label_count = 0
    component_breakdown = []
    for component in component_rows:
        comp_box = component["pixelBox"]
        points_in_comp = [pt for pt in residual_points if point_in_box(pt, comp_box)]
        label_in_comp = 0
        non_label_in_comp = 0
        for pt in points_in_comp:
            if any(point_in_box(pt, box) for box in label_boxes):
                label_in_comp += 1
            else:
                non_label_in_comp += 1
        component_breakdown.append(
            {
                "name": component["name"],
                "pixelBox": comp_box,
                "residualCount": len(points_in_comp),
                "labelResidualCount": label_in_comp,
                "nonLabelResidualCount": non_label_in_comp,
            }
        )
        component_union_count += len(points_in_comp)
        component_non_label_count += non_label_in_comp

    for pt in residual_points:
        if any(point_in_box(pt, box) for box in label_boxes):
            label_union_count += 1

    component_breakdown.sort(key=lambda row: row["residualCount"], reverse=True)
    output = {
        "payload": str(Path(args.payload_json).resolve()),
        "roleReport": str(Path(args.role_report_json).resolve()),
        "geometry": str(Path(args.geometry_json).resolve()),
        "oursPng": str(Path(args.ours_png).resolve()),
        "referencePng": str(Path(args.reference_png).resolve()),
        "dx": dx,
        "dy": dy,
        "padPx": args.pad_px,
        "residualPixelCount": len(residual_points),
        "labelUnionResidualCount": label_union_count,
        "componentUnionResidualCount": component_union_count,
        "componentNonLabelResidualCount": component_non_label_count,
        "componentBreakdown": component_breakdown,
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2), encoding="utf-8")
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
