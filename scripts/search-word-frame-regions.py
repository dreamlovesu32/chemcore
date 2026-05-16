from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PYTHON = Path(r"D:\anaconda3\python.exe")
POWERSHELL = "powershell"


def parse_frame(text: str) -> tuple[int, int, int, int]:
    parts = [int(part.strip()) for part in text.split(",")]
    if len(parts) != 4:
        raise ValueError(f"invalid frame {text!r}")
    return tuple(parts)  # type: ignore[return-value]


def parse_range(text: str) -> list[int]:
    text = text.strip()
    if "," in text:
        return [int(part.strip()) for part in text.split(",") if part.strip()]
    if ":" in text:
        parts = [int(part.strip()) for part in text.split(":")]
        if len(parts) == 2:
            start, stop = parts
            step = 1
        elif len(parts) == 3:
            start, stop, step = parts
        else:
            raise ValueError(f"invalid range {text!r}")
        if step == 0:
            raise ValueError("step must be non-zero")
        if stop < start and step > 0:
            step = -step
        stop_inclusive = stop + (1 if step > 0 else -1)
        return list(range(start, stop_inclusive, step))
    return [int(text)]


def run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd) if cwd else None,
        check=True,
        capture_output=True,
        text=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Search same-shell Word frame candidates for selected regions.")
    parser.add_argument("--template-docx", required=True)
    parser.add_argument("--reference-png", required=True)
    parser.add_argument("--base-frame", required=True, help="left,top,right,bottom in HIMETRIC")
    parser.add_argument("--delta-left", default="0")
    parser.add_argument("--delta-top", default="0")
    parser.add_argument("--delta-right", default="0")
    parser.add_argument("--delta-bottom", default="0")
    parser.add_argument("--region", action="append", default=[], help="name=l,t,r,b")
    parser.add_argument("--word-shape-index", type=int, default=1)
    parser.add_argument("--limit", type=int, default=12)
    parser.add_argument("--threshold", type=int, default=740)
    parser.add_argument("--output", required=True)
    parser.add_argument("--keep-artifacts", action="store_true")
    args = parser.parse_args()

    template_docx = (ROOT / args.template_docx).resolve()
    reference_png = (ROOT / args.reference_png).resolve()
    output_path = (ROOT / args.output).resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    base_frame = parse_frame(args.base_frame)
    left_values = parse_range(args.delta_left)
    top_values = parse_range(args.delta_top)
    right_values = parse_range(args.delta_right)
    bottom_values = parse_range(args.delta_bottom)

    region_args: list[str] = []
    region_names: list[str] = []
    for item in args.region:
        name, raw = item.split("=", 1)
        region_names.append(name)
        region_args.extend(["--region", f"{name}={raw}"])

    results: list[dict[str, object]] = []

    with tempfile.TemporaryDirectory(prefix="chemcore-frame-search-") as td:
        temp_dir = Path(td)
        for dl in left_values:
            for dt in top_values:
                for dr in right_values:
                    for db in bottom_values:
                        frame = (
                            base_frame[0] + dl,
                            base_frame[1] + dt,
                            base_frame[2] + dr,
                            base_frame[3] + db,
                        )
                        tag = f"l{dl:+d}_t{dt:+d}_r{dr:+d}_b{db:+d}".replace("+", "p").replace("-", "m")
                        docx_path = temp_dir / f"{tag}.docx"
                        png_path = temp_dir / f"{tag}.png"
                        json_path = temp_dir / f"{tag}.json"

                        run(
                            [
                                str(PYTHON),
                                str(ROOT / "scripts" / "patch-docx-image1-frame.py"),
                                str(template_docx),
                                str(docx_path),
                                "--frame",
                                ",".join(str(x) for x in frame),
                            ]
                        )
                        run(
                            [
                                POWERSHELL,
                                "-NoProfile",
                                "-ExecutionPolicy",
                                "Bypass",
                                "-File",
                                str(ROOT / "scripts" / "word-copy-inline-shape.ps1"),
                                "-InputDocx",
                                str(docx_path),
                                "-OutputPng",
                                str(png_path),
                                "-ShapeIndex",
                                str(args.word_shape_index),
                            ]
                        )
                        run(
                            [
                                str(PYTHON),
                                str(ROOT / "scripts" / "png-region-iou.py"),
                                str(png_path),
                                str(reference_png),
                                "--threshold",
                                str(args.threshold),
                                "--limit",
                                str(args.limit),
                                *region_args,
                                "--output",
                                str(json_path),
                            ]
                        )
                        report = json.loads(json_path.read_text(encoding="utf8"))
                        if region_names:
                            score = sum(
                                float(report["regions"][name]["global_shift"]["iou"]) for name in region_names
                            )
                        else:
                            score = float(report["global_best"]["iou"])
                        results.append(
                            {
                                "frame": list(frame),
                                "delta": [dl, dt, dr, db],
                                "global_best": report["global_best"],
                                "score": score,
                                "regions": {
                                    name: report["regions"][name]["global_shift"] for name in region_names
                                },
                                "artifacts": {
                                    "docx": str(docx_path),
                                    "png": str(png_path),
                                    "report": str(json_path),
                                },
                            }
                        )

        results.sort(
            key=lambda item: (
                float(item["score"]),
                float(item["global_best"]["iou"]),  # type: ignore[index]
                -sum(abs(x) for x in item["delta"]),  # type: ignore[arg-type]
            ),
            reverse=True,
        )
        output = {
            "template_docx": str(template_docx),
            "reference_png": str(reference_png),
            "base_frame": list(base_frame),
            "delta_ranges": {
                "left": left_values,
                "top": top_values,
                "right": right_values,
                "bottom": bottom_values,
            },
            "regions": region_names,
            "results": results,
        }
        output_path.write_text(json.dumps(output, indent=2), encoding="utf8")

        if args.keep_artifacts:
            keep_dir = output_path.with_suffix("")
            keep_dir.mkdir(parents=True, exist_ok=True)
            for item in results:
                tag = "_".join(
                    [
                        f"l{item['delta'][0]}",
                        f"t{item['delta'][1]}",
                        f"r{item['delta'][2]}",
                        f"b{item['delta'][3]}",
                    ]
                )
                for key, suffix in [("docx", ".docx"), ("png", ".png"), ("report", ".json")]:
                    src = Path(item["artifacts"][key])
                    dst = keep_dir / f"{tag}{suffix}"
                    dst.write_bytes(src.read_bytes())


if __name__ == "__main__":
    main()
