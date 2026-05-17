from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def load_rows(path: str) -> dict[str, dict]:
    return {
        row["nodeId"]: row
        for row in json.loads(Path(path).read_text(encoding="utf-8"))["rows"]
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize attached-label sensitivity to alternate structure-label replay paths."
    )
    parser.add_argument("current_compare_json")
    parser.add_argument("structext_compare_json")
    parser.add_argument("nodezero_compare_json")
    parser.add_argument("attached_geometry_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    current_rows = load_rows(args.current_compare_json)
    structext_rows = load_rows(args.structext_compare_json)
    nodezero_rows = load_rows(args.nodezero_compare_json)
    geometry_rows = {
        row["nodeId"]: row
        for row in json.loads(Path(args.attached_geometry_json).read_text(encoding="utf-8"))["rows"]
    }

    joined_rows = []
    grouped = defaultdict(
        lambda: {
            "count": 0,
            "sumStructDw": 0,
            "sumStructDh": 0,
            "sumStructDx": 0,
            "sumStructDy": 0,
            "sumNodezeroDw": 0,
            "sumNodezeroDh": 0,
            "sumNodezeroDx": 0,
            "sumNodezeroDy": 0,
            "sumAbsStruct": 0,
            "sumAbsNodezero": 0,
            "rows": [],
        }
    )

    for node_id, current in current_rows.items():
        structext = structext_rows.get(node_id)
        nodezero = nodezero_rows.get(node_id)
        geo = geometry_rows.get(node_id)
        if structext is None or nodezero is None or geo is None:
            continue

        cur_dw, cur_dh = current.get("deltaDims", [0, 0])
        cur_dx, cur_dy = current.get("deltaTopLeft", [0, 0])

        se_dw, se_dh = structext.get("deltaDims", [0, 0])
        se_dx, se_dy = structext.get("deltaTopLeft", [0, 0])

        nz_dw, nz_dh = nodezero.get("deltaDims", [0, 0])
        nz_dx, nz_dy = nodezero.get("deltaTopLeft", [0, 0])

        struct_delta = [se_dw - cur_dw, se_dh - cur_dh, se_dx - cur_dx, se_dy - cur_dy]
        nodezero_delta = [nz_dw - cur_dw, nz_dh - cur_dh, nz_dx - cur_dx, nz_dy - cur_dy]

        row = {
            **geo,
            "currentDeltaDims": [cur_dw, cur_dh],
            "currentDeltaTopLeft": [cur_dx, cur_dy],
            "structextDeltaDims": [se_dw, se_dh],
            "structextDeltaTopLeft": [se_dx, se_dy],
            "nodezeroDeltaDims": [nz_dw, nz_dh],
            "nodezeroDeltaTopLeft": [nz_dx, nz_dy],
            "structextMinusCurrent": struct_delta,
            "nodezeroMinusCurrent": nodezero_delta,
            "structextChangeL1": sum(abs(v) for v in struct_delta),
            "nodezeroChangeL1": sum(abs(v) for v in nodezero_delta),
        }
        joined_rows.append(row)

        key = json.dumps(
            {
                "component": row["component"],
                "text": row["text"],
                "fill": row["fill"],
                "nodeType": row.get("nodeType"),
                "cdxmlLabelJustification": row.get("cdxmlLabelJustification"),
                "componentQuadrant": row.get("componentQuadrant"),
                "primaryNeighborBucket": row.get("primaryNeighborBucket"),
            },
            ensure_ascii=False,
            sort_keys=True,
        )
        group = grouped[key]
        group["count"] += 1
        group["sumStructDw"] += struct_delta[0]
        group["sumStructDh"] += struct_delta[1]
        group["sumStructDx"] += struct_delta[2]
        group["sumStructDy"] += struct_delta[3]
        group["sumNodezeroDw"] += nodezero_delta[0]
        group["sumNodezeroDh"] += nodezero_delta[1]
        group["sumNodezeroDx"] += nodezero_delta[2]
        group["sumNodezeroDy"] += nodezero_delta[3]
        group["sumAbsStruct"] += row["structextChangeL1"]
        group["sumAbsNodezero"] += row["nodezeroChangeL1"]
        group["rows"].append(row)

    groups = []
    for key, group in grouped.items():
        base = json.loads(key)
        count = group["count"]
        groups.append(
            {
                **base,
                "count": count,
                "avgStructDw": group["sumStructDw"] / count,
                "avgStructDh": group["sumStructDh"] / count,
                "avgStructDx": group["sumStructDx"] / count,
                "avgStructDy": group["sumStructDy"] / count,
                "avgNodezeroDw": group["sumNodezeroDw"] / count,
                "avgNodezeroDh": group["sumNodezeroDh"] / count,
                "avgNodezeroDx": group["sumNodezeroDx"] / count,
                "avgNodezeroDy": group["sumNodezeroDy"] / count,
                "avgStructL1": group["sumAbsStruct"] / count,
                "avgNodezeroL1": group["sumAbsNodezero"] / count,
                "rows": group["rows"],
            }
        )
    groups.sort(key=lambda item: item["avgStructL1"], reverse=True)

    output = {"rows": joined_rows, "groups": groups}
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
