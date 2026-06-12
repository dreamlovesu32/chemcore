
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path


def l1(row: dict) -> int:
    vals = row.get("replayDeltaDims", [0, 0]) + row.get("replayDeltaTopLeft", [0, 0])
    return sum(abs(v) for v in vals)


def gap_right(row: dict) -> float:
    return -row["overhangToComponent"]["right"]


def bucket_gap(value: float) -> str:
    if value < 40:
        return "lt40"
    if value < 80:
        return "40to80"
    if value < 140:
        return "80to140"
    return "ge140"


def main() -> None:
    parser = argparse.ArgumentParser(description="Summarize attached-group replay geometry families.")
    parser.add_argument("input_json")
    parser.add_argument("output_json")
    args = parser.parse_args()

    rows = json.loads(Path(args.input_json).read_text(encoding="utf-8"))["rows"]
    rows = [r for r in rows if r.get("layout") == "attached-group"]

    for row in rows:
        row["replayL1"] = l1(row)
        row["gapRight"] = gap_right(row)
        row["gapRightBucket"] = bucket_gap(row["gapRight"])

    groups = defaultdict(list)
    for row in rows:
        key = (
            row.get("cdxmlLabelJustification"),
            row.get("componentHalfX"),
            row.get("primaryNeighborBucket"),
            row.get("gapRightBucket"),
        )
        groups[key].append(row)

    summaries = []
    for key, vals in groups.items():
        summaries.append(
            {
                "cdxmlLabelJustification": key[0],
                "componentHalfX": key[1],
                "primaryNeighborBucket": key[2],
                "gapRightBucket": key[3],
                "count": len(vals),
                "avgReplayWidth": sum(v["replayDeltaDims"][0] for v in vals) / len(vals),
                "avgReplayLeftShift": sum(-v["replayDeltaTopLeft"][0] for v in vals) / len(vals),
                "avgReplayL1": sum(v["replayL1"] for v in vals) / len(vals),
                "nodeIds": [v["nodeId"] for v in vals],
            }
        )
    summaries.sort(key=lambda item: (-item["avgReplayL1"], -item["count"]))

    output = {
        "rows": rows,
        "groups": summaries,
    }
    Path(args.output_json).write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
