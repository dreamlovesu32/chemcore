from __future__ import annotations

import argparse
import json
import statistics as st
import struct
import zipfile
from pathlib import Path
from typing import Any


def read_docx_image1_frame(docx_path: Path) -> dict[str, list[int]]:
    with zipfile.ZipFile(docx_path, "r") as zf:
        data = zf.read("word/media/image1.emf")
    if len(data) < 40:
        raise ValueError(f"{docx_path} image1.emf too short")
    bounds = list(struct.unpack_from("<4i", data, 8))
    frame = list(struct.unpack_from("<4i", data, 24))
    return {
        "bounds": bounds,
        "frame": frame,
        "size": [frame[2] - frame[0], frame[3] - frame[1]],
    }


def summarize_compare_dir(compare_dir: Path) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    summary_path = compare_dir / "summary.json"
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    rows: list[dict[str, Any]] = []
    for item in summary.get("results", []):
        stem = item["stem"]
        ours_docx = compare_dir / f"{stem}.chemcore.docx"
        ref_docx = compare_dir / f"{stem}.chemdraw-shell.docx"
        ours = read_docx_image1_frame(ours_docx)
        ref = read_docx_image1_frame(ref_docx)
        rows.append(
            {
                "sample": compare_dir.parent.name,
                "stem": stem,
                "bestIou": item["bestShift"]["best_iou"],
                "dx": item["bestShift"]["dx"],
                "dy": item["bestShift"]["dy"],
                "oursFrame": ours["frame"],
                "refFrame": ref["frame"],
                "oursSize": ours["size"],
                "refSize": ref["size"],
                "frameDelta": [ref["frame"][i] - ours["frame"][i] for i in range(4)],
                "sizeDelta": [ref["size"][0] - ours["size"][0], ref["size"][1] - ours["size"][1]],
                "widthRatio": ref["size"][0] / ours["size"][0] if ours["size"][0] else None,
                "heightRatio": ref["size"][1] / ours["size"][1] if ours["size"][1] else None,
            }
        )
    failures = [
        {
            "sample": compare_dir.parent.name,
            "index": failure["index"],
            "reason": failure["reason"],
        }
        for failure in summary.get("failures", [])
    ]
    return rows, failures


def aggregate(rows: list[dict[str, Any]], failures: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {"count": 0, "rows": [], "failures": failures}
    return {
        "count": len(rows),
        "avgBestIou": st.mean(row["bestIou"] for row in rows),
        "avgDx": st.mean(row["dx"] for row in rows),
        "avgDy": st.mean(row["dy"] for row in rows),
        "avgFrameDelta": [st.mean(row["frameDelta"][i] for row in rows) for i in range(4)],
        "avgSizeDelta": [st.mean(row["sizeDelta"][i] for row in rows) for i in range(2)],
        "avgWidthRatio": st.mean(row["widthRatio"] for row in rows if row["widthRatio"] is not None),
        "avgHeightRatio": st.mean(row["heightRatio"] for row in rows if row["heightRatio"] is not None),
        "rows": rows,
        "failures": failures,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize same-shell PPT ChemDraw compare outputs and EMF frame deltas."
    )
    parser.add_argument("compare_dirs", nargs="+", help="Directories containing same-shell summary.json outputs")
    parser.add_argument("--output", required=True, help="Path to write the aggregate JSON report")
    args = parser.parse_args()

    all_rows: list[dict[str, Any]] = []
    all_failures: list[dict[str, Any]] = []
    for compare_dir_arg in args.compare_dirs:
        compare_dir = Path(compare_dir_arg)
        rows, failures = summarize_compare_dir(compare_dir)
        all_rows.extend(rows)
        all_failures.extend(failures)

    report = aggregate(all_rows, all_failures)
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(output_path)
    print(
        json.dumps(
            {
                "count": report["count"],
                "avgBestIou": report.get("avgBestIou"),
                "avgDx": report.get("avgDx"),
                "avgDy": report.get("avgDy"),
                "avgFrameDelta": report.get("avgFrameDelta"),
                "avgSizeDelta": report.get("avgSizeDelta"),
                "avgWidthRatio": report.get("avgWidthRatio"),
                "avgHeightRatio": report.get("avgHeightRatio"),
                "failureCount": len(report.get("failures", [])),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
