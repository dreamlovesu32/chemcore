from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare two png-region-iou reports and summarize per-region deltas."
    )
    parser.add_argument("base_report")
    parser.add_argument("target_report")
    parser.add_argument("--output")
    args = parser.parse_args()

    base = json.loads(Path(args.base_report).read_text(encoding="utf-8"))
    target = json.loads(Path(args.target_report).read_text(encoding="utf-8"))

    common_regions = sorted(set(base["regions"]).intersection(target["regions"]))
    deltas: list[dict[str, object]] = []
    for name in common_regions:
        base_stats = base["regions"][name]["global_shift"]
        target_stats = target["regions"][name]["global_shift"]
        deltas.append(
            {
                "region": name,
                "base_iou": base_stats["iou"],
                "target_iou": target_stats["iou"],
                "delta_iou": target_stats["iou"] - base_stats["iou"],
                "delta_intersection": target_stats["intersection"]
                - base_stats["intersection"],
                "delta_only_ours": target_stats["only_ours"] - base_stats["only_ours"],
                "delta_only_reference": target_stats["only_reference"]
                - base_stats["only_reference"],
            }
        )

    deltas.sort(key=lambda item: item["delta_iou"], reverse=True)
    result = {
        "base_report": args.base_report,
        "target_report": args.target_report,
        "base_global": base["global_best"],
        "target_global": target["global_best"],
        "global_delta_iou": target["global_best"]["iou"] - base["global_best"]["iou"],
        "regions": deltas,
    }

    text = json.dumps(result, ensure_ascii=False, indent=2)
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
    else:
        print(text)


if __name__ == "__main__":
    main()
