from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
from collections import defaultdict
from pathlib import Path
from statistics import mean


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module from {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def ensure_role_report(repo_root: Path, payload: Path, role_report: Path) -> None:
    if role_report.exists():
        return
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "chemcore-engine",
        "--example",
        "render_role_report",
        "--",
        str(payload),
    ]
    result = subprocess.run(
        cmd,
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=True,
    )
    role_report.write_text(result.stdout, encoding="utf-8")


def summarize_rows(rows: list[dict]) -> dict:
    def family_table(input_rows: list[dict], key_fn) -> list[dict]:
        groups = defaultdict(list)
        for row in input_rows:
            groups[key_fn(row)].append(row)
        out = []
        for key, items in groups.items():
            out.append(
                {
                    "key": key,
                    "count": len(items),
                    "avgResidual": mean(item["residualCount"] for item in items),
                    "sumResidual": sum(item["residualCount"] for item in items),
                    "avgIoU": mean(item["iou"] for item in items),
                    "avgDw": mean(item["deltaDims"][0] for item in items),
                    "avgDh": mean(item["deltaDims"][1] for item in items),
                    "avgDx": mean(item["deltaTopLeft"][0] for item in items),
                    "avgDy": mean(item["deltaTopLeft"][1] for item in items),
                    "examples": [f"{item['sampleStem']}:{item['text']}" for item in items[:5]],
                }
            )
        out.sort(key=lambda item: (-item["sumResidual"], item["key"]))
        return out

    sample_summary = []
    sample_groups = defaultdict(list)
    for row in rows:
        sample_groups[row["sampleStem"]].append(row)
    for key, items in sorted(sample_groups.items()):
        sample_summary.append(
            {
                "sampleStem": key,
                "count": len(items),
                "avgResidual": mean(item["residualCount"] for item in items),
                "sumResidual": sum(item["residualCount"] for item in items),
                "avgIoU": mean(item["iou"] for item in items),
            }
        )

    return {
        "byTextFill": family_table(rows, lambda r: f"{r.get('text','')}|{r.get('fill')}"),
        "byLayout": family_table(rows, lambda r: r.get("layout", "")),
        "byTextLayout": family_table(rows, lambda r: f"{r.get('text','')}|{r.get('layout','')}"),
        "byAnchor": family_table(rows, lambda r: r.get("anchor", "")),
        "byAttachment": family_table(rows, lambda r: r.get("attachment", "")),
        "sampleSummary": sample_summary,
        "topResidualRows": sorted(rows, key=lambda r: r["residualCount"], reverse=True)[:30],
        "worstIouRows": sorted(rows, key=lambda r: r["iou"])[:30],
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Batch-compare same-shell PPT molecule label boxes against ChemDraw references."
    )
    parser.add_argument(
        "--pattern",
        default="tmp/ppt-sample-*/same-shell-compare/*.payload.json",
        help="Glob for payload JSON files.",
    )
    parser.add_argument(
        "--output",
        default="tmp/ppt-generalization-label-boxes.json",
        help="Output JSON path.",
    )
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--pad-px", type=int, default=0)
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    attr_mod = load_module(repo_root / "scripts" / "attribute-word-residual-labels.py", "attr_word_residual_labels")
    cmp_mod = load_module(repo_root / "scripts" / "compare-full-label-boxes.py", "compare_full_label_boxes")
    iou_mod = load_module(repo_root / "scripts" / "compare-full-label-iou.py", "compare_full_label_iou")

    payload_paths = sorted(repo_root.glob(args.pattern))
    rows: list[dict] = []
    objects = []
    skipped = []

    for payload_path in payload_paths:
        base_no_payload = payload_path.name.removesuffix(".payload.json")
        compare_dir = payload_path.parent
        role_report = compare_dir / f"{base_no_payload}.role-report.json"
        ours_png = compare_dir / f"{base_no_payload}.chemcore.wordcopy.png"
        ref_png = compare_dir / f"{base_no_payload}.chemdraw.wordcopy.png"
        bestshift = compare_dir / f"{base_no_payload}.bestshift.json"

        if not ours_png.exists() or not ref_png.exists() or not bestshift.exists():
            skipped.append({"payload": str(payload_path), "reason": "missing compare asset"})
            continue

        ensure_role_report(repo_root, payload_path, role_report)
        best_data = json.loads(bestshift.read_text(encoding="utf-8"))
        dx = int(best_data["dx"])
        dy = int(best_data["dy"])

        document = attr_mod.load_payload(payload_path)
        role_data = attr_mod.load_json_any_encoding(role_report)
        ours_mask, width, height = attr_mod.load_mask(ours_png, args.threshold)
        ref_mask, ref_width, ref_height = attr_mod.load_mask(ref_png, args.threshold)
        ours_rgba = cmp_mod.load_rgba(ours_png)
        ref_rgba = cmp_mod.load_rgba(ref_png)
        if (width, height) != (ref_width, ref_height):
            skipped.append({"payload": str(payload_path), "reason": "wordcopy size mismatch"})
            continue

        resources_obj = document.get("resources", {})
        resources = resources_obj if isinstance(resources_obj, dict) else {r["id"]: r for r in resources_obj}
        visible = role_data["visibleBoundsNoKnockout"]

        residual_points: list[tuple[int, int]] = []
        for y in range(height):
            for x in range(width):
                xa = x + dx
                ya = y + dy
                ours_value = ours_mask[ya][xa] if 0 <= xa < width and 0 <= ya < height else False
                ref_value = ref_mask[y][x]
                if ours_value ^ ref_value:
                    residual_points.append((x, y))

        sample_name = compare_dir.parent.parent.name
        sample_stem = f"{sample_name}/{base_no_payload}"
        label_count = 0

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
                local_box = attr_mod.bbox_from_points(glyph_points) or label.get("box")
                if not local_box:
                    continue
                world_box = attr_mod.transform_box(local_box, transform)
                pixel_box = attr_mod.project_box(world_box, visible, width, height, args.pad_px)
                x1, y1, x2, y2 = pixel_box
                residual_count = 0
                for x, y in residual_points:
                    if x1 <= x <= x2 and y1 <= y <= y2:
                        residual_count += 1

                bbox_row = {
                    "sample": sample_name,
                    "sampleStem": sample_stem,
                    "objectId": obj.get("id"),
                    "nodeId": node.get("id"),
                    "text": label.get("text", ""),
                    "fill": label.get("fill"),
                    "layout": label.get("layout"),
                    "attachment": label.get("attachment"),
                    "anchor": label.get("anchor"),
                    "align": label.get("align"),
                    "fontFamily": label.get("fontFamily"),
                    "fontSize": label.get("fontSize"),
                    "pixelBox": pixel_box,
                    "residualCount": residual_count,
                }

                box = [x1 - args.pad_px, y1 - args.pad_px, x2 + args.pad_px, y2 + args.pad_px]
                obox = [box[0] + dx, box[1] + dy, box[2] + dx, box[3] + dy]
                ours_bbox = cmp_mod.mask_bbox(ours_rgba, obox)
                ref_bbox = cmp_mod.mask_bbox(ref_rgba, box)
                if ours_bbox is None or ref_bbox is None:
                    continue
                ours_dims = cmp_mod.dims(ours_bbox)
                ref_dims = cmp_mod.dims(ref_bbox)
                iou_stats = iou_mod.mask_box_counts(
                    ours_rgba,
                    ref_rgba,
                    box,
                    dx,
                    dy,
                    args.threshold,
                )
                if iou_stats is None:
                    continue

                bbox_row.update(
                    {
                        "oursLocalBbox": ours_bbox,
                        "refLocalBbox": ref_bbox,
                        "oursDims": ours_dims,
                        "refDims": ref_dims,
                        "deltaDims": [ours_dims[0] - ref_dims[0], ours_dims[1] - ref_dims[1]],
                        "deltaTopLeft": [ours_bbox[0] - ref_bbox[0], ours_bbox[1] - ref_bbox[1]],
                        **iou_stats,
                    }
                )
                rows.append(bbox_row)
                label_count += 1

        objects.append(
            {
                "sampleStem": sample_stem,
                "payload": str(payload_path),
                "roleReport": str(role_report),
                "dx": dx,
                "dy": dy,
                "labelCount": label_count,
            }
        )

    summary = summarize_rows(rows)
    output = {
        "pattern": args.pattern,
        "countObjects": len(objects),
        "countRows": len(rows),
        "objects": objects,
        "skipped": skipped,
        "rows": rows,
        **summary,
    }
    output_path = repo_root / args.output
    output_path.write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    try:
        print(json.dumps(output, indent=2, ensure_ascii=False))
    except UnicodeEncodeError:
        print(json.dumps(output, indent=2, ensure_ascii=True))


if __name__ == "__main__":
    main()
