from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from statistics import mean


def derive_fields(row: dict) -> dict:
    text = row.get("text", "")
    line_count = text.count("\n") + 1
    char_count = len(text.replace("\n", ""))
    fill = (row.get("fill") or "").lower()
    is_black = fill == "#000000"
    derived = dict(row)
    derived.update(
        {
            "lineCount": line_count,
            "charCount": char_count,
            "isBlack": is_black,
            "charBucket": min(char_count, 9),
        }
    )
    return derived


def summarize_groups(rows: list[dict], key_fn, *, min_count: int = 3) -> list[dict]:
    groups: dict[object, list[dict]] = defaultdict(list)
    for row in rows:
        groups[key_fn(row)].append(row)

    out = []
    for key, items in groups.items():
        if len(items) < min_count:
            continue
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
                "examples": sorted({item["text"] for item in items})[:12],
                "sampleExamples": [f"{item['sampleStem']}:{item['text']}" for item in items[:6]],
            }
        )

    out.sort(key=lambda item: (item["avgIoU"], -item["count"], str(item["key"])))
    return out


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize generalized PPT label families from same-shell label-box comparisons."
    )
    parser.add_argument(
        "--input",
        default="tmp/ppt-generalization-label-boxes.json",
        help="Input JSON from summarize-ppt-label-boxes.py",
    )
    parser.add_argument(
        "--output",
        default="tmp/ppt-generalization-derived-label-families.json",
        help="Output JSON path.",
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    data = json.loads(input_path.read_text(encoding="utf-8"))
    rows = [derive_fields(row) for row in data["rows"]]

    def named_family(predicate_name: str, predicate):
        items = [row for row in rows if predicate(row)]
        if not items:
            return None
        return {
            "name": predicate_name,
            "count": len(items),
            "avgResidual": mean(item["residualCount"] for item in items),
            "sumResidual": sum(item["residualCount"] for item in items),
            "avgIoU": mean(item["iou"] for item in items),
            "avgDw": mean(item["deltaDims"][0] for item in items),
            "avgDh": mean(item["deltaDims"][1] for item in items),
            "avgDx": mean(item["deltaTopLeft"][0] for item in items),
            "avgDy": mean(item["deltaTopLeft"][1] for item in items),
            "examples": sorted({item["text"] for item in items})[:12],
            "sampleExamples": [f"{item['sampleStem']}:{item['text']}" for item in items[:8]],
        }

    explicit_families = [
        named_family(
            "attached_above_all",
            lambda r: r["layout"] == "attached-group-above",
        ),
        named_family(
            "attached_above_multiline_black",
            lambda r: r["layout"] == "attached-group-above" and r["lineCount"] >= 2 and r["isBlack"],
        ),
        named_family(
            "attached_multiline_black",
            lambda r: r["layout"] == "attached-group" and r["lineCount"] >= 2 and r["isBlack"],
        ),
        named_family(
            "attached_long_black",
            lambda r: r["layout"] == "attached-group"
            and r["lineCount"] == 1
            and r["charCount"] >= 4
            and r["isBlack"],
        ),
        named_family(
            "attached_compact_black_2char",
            lambda r: r["layout"] == "attached-group"
            and r["lineCount"] == 1
            and r["charCount"] == 2
            and r["isBlack"],
        ),
        named_family(
            "attached_compact_black_1char",
            lambda r: r["layout"] == "attached-group"
            and r["lineCount"] == 1
            and r["charCount"] == 1
            and r["isBlack"],
        ),
        named_family(
            "attached_nonblack",
            lambda r: r["layout"] == "attached-group" and not r["isBlack"],
        ),
    ]
    explicit_families = [family for family in explicit_families if family is not None]
    explicit_families.sort(key=lambda item: (item["avgIoU"], -item["count"], item["name"]))

    output = {
        "source": str(input_path),
        "countRows": len(rows),
        "layoutLineColorFamilies": summarize_groups(
            rows,
            lambda r: (
                r["layout"],
                r["lineCount"],
                "black" if r["isBlack"] else "nonblack",
            ),
        ),
        "layoutLineFamilies": summarize_groups(
            rows,
            lambda r: (r["layout"], r["lineCount"]),
        ),
        "layoutCharFamilies": summarize_groups(
            rows,
            lambda r: (r["layout"], r["charBucket"]),
        ),
        "textLayoutFamilies": summarize_groups(
            rows,
            lambda r: (r["text"], r["layout"]),
            min_count=2,
        ),
        "explicitFamilies": explicit_families,
    }

    output_path = Path(args.output)
    output_path.write_text(json.dumps(output, indent=2, ensure_ascii=False), encoding="utf-8")
    print(output_path)


if __name__ == "__main__":
    main()
