from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> None:
    ap = argparse.ArgumentParser(
        description="Summarize attached-label local IoU sensitivity across frame shift variants."
    )
    ap.add_argument("geometry_json")
    ap.add_argument("base_iou_json")
    ap.add_argument("variant_iou_json", nargs="+")
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    geometry_rows = load_json(Path(args.geometry_json))["rows"]
    base_rows = {row["nodeId"]: row for row in load_json(Path(args.base_iou_json))["rows"]}
    variant_maps = {
        Path(path).stem: {row["nodeId"]: row for row in load_json(Path(path))["rows"]}
        for path in args.variant_iou_json
    }

    rows = []
    for geo in geometry_rows:
        node_id = geo["nodeId"]
        base = base_rows.get(node_id)
        if base is None:
            continue
        row = {
            "nodeId": node_id,
            "text": geo.get("text", ""),
            "fill": geo.get("fill"),
            "layout": geo.get("layout"),
            "anchor": geo.get("anchor"),
            "attachment": geo.get("attachment"),
            "component": geo.get("component"),
            "componentQuadrant": geo.get("componentQuadrant"),
            "cdxmlLabelJustification": geo.get("cdxmlLabelJustification"),
            "primaryNeighborBucket": geo.get("primaryNeighborBucket"),
            "worldBox": geo.get("worldBox"),
            "labelCenterWorld": geo.get("labelCenterWorld"),
            "baseIou": base["iou"],
            "baseResidual": base["residualCount"],
        }
        if geo.get("worldBox"):
            row["boxLeftPhase"] = geo["worldBox"][0] % 1.0
            row["boxTopPhase"] = geo["worldBox"][1] % 1.0
        if geo.get("labelCenterWorld"):
            row["centerXPhase"] = geo["labelCenterWorld"][0] % 1.0
            row["centerYPhase"] = geo["labelCenterWorld"][1] % 1.0
        for name, variant in variant_maps.items():
            other = variant.get(node_id)
            if other is None:
                continue
            row[f"{name}Iou"] = other["iou"]
            row[f"{name}Residual"] = other["residualCount"]
            row[f"{name}DeltaIou"] = other["iou"] - base["iou"]
        rows.append(row)

    def summarize(key_fn, key_repr_fn=None):
        groups = defaultdict(
            lambda: {
                "count": 0,
                "sumBaseIou": 0.0,
                "sumBaseResidual": 0,
                "sumDelta": defaultdict(float),
                "samples": [],
            }
        )
        for row in rows:
            key = key_fn(row)
            g = groups[key]
            g["count"] += 1
            g["sumBaseIou"] += row["baseIou"]
            g["sumBaseResidual"] += row["baseResidual"]
            for name in variant_maps:
                delta = row.get(f"{name}DeltaIou")
                if delta is not None:
                    g["sumDelta"][name] += delta
            if len(g["samples"]) < 8:
                g["samples"].append(
                    {
                        "nodeId": row["nodeId"],
                        "text": row["text"],
                        "fill": row["fill"],
                        "baseIou": row["baseIou"],
                        **{
                            f"{name}DeltaIou": row.get(f"{name}DeltaIou")
                            for name in variant_maps
                        },
                    }
                )
        summary = []
        for key, g in groups.items():
            item = {
                "key": key_repr_fn(key) if key_repr_fn else key,
                "count": g["count"],
                "avgBaseIou": g["sumBaseIou"] / g["count"],
                "avgBaseResidual": g["sumBaseResidual"] / g["count"],
                "samples": g["samples"],
            }
            for name in variant_maps:
                item[f"avg{name}DeltaIou"] = g["sumDelta"][name] / g["count"]
            summary.append(item)
        summary.sort(key=lambda item: item["count"], reverse=True)
        return summary

    payload = {
        "geometryJson": str(Path(args.geometry_json).resolve()),
        "baseIouJson": str(Path(args.base_iou_json).resolve()),
        "variantIouJsons": [str(Path(path).resolve()) for path in args.variant_iou_json],
        "variantNames": list(variant_maps.keys()),
        "rows": rows,
        "summaryByTextFill": summarize(lambda row: f"{row['text']}|{row['fill']}"),
        "summaryByPhaseBucket": summarize(
            lambda row: (
                round(row.get("centerYPhase", 0.0), 3),
                round(row.get("boxTopPhase", 0.0), 3),
            ),
            lambda key: {"centerYPhase": key[0], "boxTopPhase": key[1]},
        ),
        "summaryByAttachedFamily": summarize(
            lambda row: (
                row["text"],
                row["fill"],
                row.get("cdxmlLabelJustification"),
                row.get("componentQuadrant"),
                row.get("primaryNeighborBucket"),
            ),
            lambda key: {
                "text": key[0],
                "fill": key[1],
                "justify": key[2],
                "quadrant": key[3],
                "neighbor": key[4],
            },
        ),
    }
    Path(args.output).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
