from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize current vs no-knockout deltas by attached-label local geometry family."
    )
    parser.add_argument("current_compare_json")
    parser.add_argument("noknockout_compare_json")
    parser.add_argument("attached_geometry_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    current_rows = {
        row["nodeId"]: row
        for row in json.loads(Path(args.current_compare_json).read_text(encoding="utf-8"))["rows"]
    }
    no_knockout_rows = {
        row["nodeId"]: row
        for row in json.loads(Path(args.noknockout_compare_json).read_text(encoding="utf-8"))["rows"]
    }
    geometry_rows = {
        row["nodeId"]: row
        for row in json.loads(Path(args.attached_geometry_json).read_text(encoding="utf-8"))["rows"]
    }

    joined_rows = []
    grouped = defaultdict(
        lambda: {
            "count": 0,
            "sumResidual": 0,
            "sumReplayDw": 0,
            "sumReplayDh": 0,
            "sumReplayDx": 0,
            "sumReplayDy": 0,
            "sumKoDw": 0,
            "sumKoDh": 0,
            "sumKoDx": 0,
            "sumKoDy": 0,
            "sumAbsKo": 0,
            "rows": [],
        }
    )

    for node_id, current in current_rows.items():
        no = no_knockout_rows.get(node_id)
        geo = geometry_rows.get(node_id)
        if no is None or geo is None:
            continue

        replay_dw, replay_dh = no.get("deltaDims", [0, 0])
        replay_dx, replay_dy = no.get("deltaTopLeft", [0, 0])
        current_dw, current_dh = current.get("deltaDims", [0, 0])
        current_dx, current_dy = current.get("deltaTopLeft", [0, 0])
        ko_dw = current_dw - replay_dw
        ko_dh = current_dh - replay_dh
        ko_dx = current_dx - replay_dx
        ko_dy = current_dy - replay_dy

        row = {
            **geo,
            "currentResidualCount": current["residualCount"],
            "currentDeltaDims": current.get("deltaDims", [0, 0]),
            "currentDeltaTopLeft": current.get("deltaTopLeft", [0, 0]),
            "replayDeltaDims": [replay_dw, replay_dh],
            "replayDeltaTopLeft": [replay_dx, replay_dy],
            "knockoutDeltaDims": [ko_dw, ko_dh],
            "knockoutDeltaTopLeft": [ko_dx, ko_dy],
            "knockoutMagnitudeL1": abs(ko_dw) + abs(ko_dh) + abs(ko_dx) + abs(ko_dy),
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
        group["sumResidual"] += current["residualCount"]
        group["sumReplayDw"] += replay_dw
        group["sumReplayDh"] += replay_dh
        group["sumReplayDx"] += replay_dx
        group["sumReplayDy"] += replay_dy
        group["sumKoDw"] += ko_dw
        group["sumKoDh"] += ko_dh
        group["sumKoDx"] += ko_dx
        group["sumKoDy"] += ko_dy
        group["sumAbsKo"] += row["knockoutMagnitudeL1"]
        group["rows"].append(row)

    groups = []
    for key, group in grouped.items():
        count = group["count"]
        groups.append(
            {
                **json.loads(key),
                "count": count,
                "sumResidual": group["sumResidual"],
                "avgResidual": group["sumResidual"] / count,
                "avgReplayDw": group["sumReplayDw"] / count,
                "avgReplayDh": group["sumReplayDh"] / count,
                "avgReplayDx": group["sumReplayDx"] / count,
                "avgReplayDy": group["sumReplayDy"] / count,
                "avgKoDw": group["sumKoDw"] / count,
                "avgKoDh": group["sumKoDh"] / count,
                "avgKoDx": group["sumKoDx"] / count,
                "avgKoDy": group["sumKoDy"] / count,
                "avgKoL1": group["sumAbsKo"] / count,
                "rows": group["rows"],
            }
        )
    groups.sort(key=lambda item: item["avgKoL1"], reverse=True)

    output = {"rows": joined_rows, "groups": groups}
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
