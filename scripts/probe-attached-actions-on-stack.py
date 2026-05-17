from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run(cmd: list[str], env: dict[str, str] | None = None) -> None:
    subprocess.run(cmd, check=True, cwd=ROOT, env=env)


def extract_image1(docx_path: Path, output_emf: Path) -> None:
    with zipfile.ZipFile(docx_path, "r") as zf:
        output_emf.write_bytes(zf.read("word/media/image1.emf"))


def parse_specs(values: list[str]) -> list[tuple[str, str]]:
    out: list[tuple[str, str]] = []
    for value in values:
        amount, filt = value.split("|", 1)
        out.append((amount, filt))
    return out


def amount_matches(a: str, b: str) -> bool:
    try:
        return abs(float(a) - float(b)) < 1e-9
    except ValueError:
        return a == b


def join_filter(existing: str, node_id: str) -> str:
    parts = [part.strip() for part in existing.split(",") if part.strip()]
    if node_id not in parts:
        parts.append(node_id)
    return ",".join(parts)


def add_env_specs(env: dict[str, str], specs: list[tuple[str, str]], value_keys: list[str], filter_keys: list[str]) -> None:
    for idx, (amount, filt) in enumerate(specs):
        if idx >= len(value_keys):
            raise ValueError("too many baseline specs for available env slots")
        env[value_keys[idx]] = amount
        env[filter_keys[idx]] = filt


def label_iou_map(path: Path) -> dict[str, dict]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    return {row["nodeId"]: row for row in payload["rows"]}


def main() -> None:
    ap = argparse.ArgumentParser(description="Probe attached-label single-node actions on top of a stacked replay baseline.")
    ap.add_argument("--payload-json", required=True)
    ap.add_argument("--template-docx", required=True)
    ap.add_argument("--reference-png", required=True)
    ap.add_argument("--label-json", required=True)
    ap.add_argument("--frame", required=True, help="left,top,right,bottom")
    ap.add_argument("--output-dir", required=True)
    ap.add_argument("--nodes", required=True, help="comma-separated node ids")
    ap.add_argument("--actions", required=True, help="comma-separated actions, e.g. x:+1,top:-1,font:0.97")
    ap.add_argument("--phase-policy", default="")
    ap.add_argument("--baseline-x", action="append", default=[], help="repeatable amount|filter")
    ap.add_argument("--baseline-top", action="append", default=[], help="repeatable amount|filter")
    ap.add_argument("--baseline-font", action="append", default=[], help="repeatable scale|filter")
    args = ap.parse_args()

    payload_json = (ROOT / args.payload_json).resolve()
    template_docx = (ROOT / args.template_docx).resolve()
    reference_png = (ROOT / args.reference_png).resolve()
    label_json = (ROOT / args.label_json).resolve()
    out_dir = (ROOT / args.output_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    nodes = [part.strip() for part in args.nodes.split(",") if part.strip()]
    actions = [part.strip() for part in args.actions.split(",") if part.strip()]
    baseline_x = parse_specs(args.baseline_x)
    baseline_top = parse_specs(args.baseline_top)
    baseline_font = parse_specs(args.baseline_font)

    common_env = os.environ.copy()
    if args.phase_policy:
        common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_PHASE_POLICY_EXPERIMENT"] = args.phase_policy
    add_env_specs(
        common_env,
        baseline_x,
        [
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT_2",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT_3",
        ],
        [
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_2",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_3",
        ],
    )
    add_env_specs(
        common_env,
        baseline_top,
        [
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_EXPERIMENT",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_EXPERIMENT_2",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_EXPERIMENT_3",
        ],
        [
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT_2",
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT_3",
        ],
    )
    add_env_specs(
        common_env,
        baseline_font,
        [
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_EXPERIMENT",
        ],
        [
            "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_NODE_FILTER_EXPERIMENT",
        ],
    )

    baseline_dir = out_dir / "baseline"
    baseline_dir.mkdir(parents=True, exist_ok=True)
    baseline_docx = baseline_dir / "fg3.docx"
    baseline_png = baseline_dir / "wordcopy.fg3.png"
    baseline_best = baseline_dir / "bestshift.fg3.json"
    baseline_label = baseline_dir / "label-iou.fg3.json"
    if not baseline_best.exists() or not baseline_label.exists():
        with tempfile.TemporaryDirectory(prefix="probe_baseline_", dir=str(out_dir)) as td:
            td_path = Path(td)
            raw_docx = td_path / "baseline.raw.docx"
            raw_emf = td_path / "baseline.raw.image1.emf"
            run(
                [
                    "cargo",
                    "run",
                    "-q",
                    "-p",
                    "chemcore-office",
                    "--",
                    "--write-word-docx-payload",
                    str(payload_json),
                    str(raw_docx),
                ],
                env=common_env,
            )
            extract_image1(raw_docx, raw_emf)
            shell_docx = td_path / "baseline.shell.docx"
            run(
                [
                    "D:\\anaconda3\\python.exe",
                    str(ROOT / "scripts" / "patch-docx-image1.py"),
                    str(template_docx),
                    str(raw_emf),
                    str(shell_docx),
                ]
            )
            run(
                [
                    "D:\\anaconda3\\python.exe",
                    str(ROOT / "scripts" / "patch-docx-image1-frame.py"),
                    str(shell_docx),
                    "--frame",
                    args.frame,
                    str(baseline_docx),
                ]
            )
        run(
            [
                "powershell",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(ROOT / "scripts" / "word-copy-inline-shape.ps1"),
                "-InputDocx",
                str(baseline_docx),
                "-ShapeIndex",
                "1",
                "-OutputPng",
                str(baseline_png),
            ]
        )
        run(
            [
                "D:\\anaconda3\\python.exe",
                str(ROOT / "scripts" / "png-best-shift.py"),
                str(baseline_png),
                str(reference_png),
                "--limit",
                "20",
                "--output",
                str(baseline_best),
            ]
        )
        best = json.loads(baseline_best.read_text(encoding="utf-8"))
        run(
            [
                "D:\\anaconda3\\python.exe",
                str(ROOT / "scripts" / "compare-full-label-iou.py"),
                str(label_json),
                str(baseline_png),
                str(reference_png),
                str(baseline_label),
                "--dx",
                str(best["dx"]),
                "--dy",
                str(best["dy"]),
                "--pad",
                "3",
            ]
        )

    baseline_best_json = json.loads(baseline_best.read_text(encoding="utf-8"))
    baseline_label_map = label_iou_map(baseline_label)
    results = {
        "baseline": {
            "bestshift": baseline_best_json,
            "labelIoU": {
                node: baseline_label_map.get(node, {}).get("iou")
                for node in nodes
            },
        },
        "variants": [],
    }

    for node_id in nodes:
        for action in actions:
            kind, value = action.split(":", 1)
            env = common_env.copy()
            label = f"{node_id}.{kind}.{value}".replace("+", "p").replace("-", "m")
            if kind == "x":
                matched = False
                for idx, (amount, filt) in enumerate(baseline_x):
                    if amount_matches(amount, value):
                        env[
                            [
                                "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT",
                                "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_2",
                                "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_3",
                            ][idx]
                        ] = join_filter(filt, node_id)
                        matched = True
                        break
                if not matched:
                    env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT_3"] = value
                    env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_3"] = node_id
            elif kind == "top":
                matched = False
                for idx, (amount, filt) in enumerate(baseline_top):
                    if amount_matches(amount, value):
                        env[
                            [
                                "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT",
                                "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT_2",
                                "CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TOP_NUDGE_NODE_FILTER_EXPERIMENT_3",
                            ][idx]
                        ] = join_filter(filt, node_id)
                        matched = True
                        break
                if not matched:
                    raise ValueError(
                        f"cannot add top:{value} on top of a baseline that already uses all top slots"
                    )
            elif kind == "y":
                env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_Y_NUDGE_EXPERIMENT_2"] = value
                env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NODE_FILTER_EXPERIMENT_2"] = node_id
            elif kind == "font":
                if baseline_font and amount_matches(baseline_font[0][0], value):
                    env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_EXPERIMENT"] = value
                    env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_NODE_FILTER_EXPERIMENT"] = join_filter(
                        baseline_font[0][1], node_id
                    )
                elif not baseline_font:
                    env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_EXPERIMENT"] = value
                    env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_NODE_FILTER_EXPERIMENT"] = node_id
                else:
                    raise ValueError(
                        f"cannot add font:{value} on top of an incompatible baseline font family"
                    )
            else:
                raise ValueError(f"unsupported action kind: {kind}")

            variant_dir = out_dir / label
            variant_dir.mkdir(parents=True, exist_ok=True)
            variant_docx = variant_dir / "fg3.docx"
            variant_png = variant_dir / "wordcopy.fg3.png"
            variant_best = variant_dir / "bestshift.fg3.json"
            variant_label = variant_dir / "label-iou.fg3.json"

            with tempfile.TemporaryDirectory(prefix="probe_variant_", dir=str(out_dir)) as td:
                td_path = Path(td)
                raw_docx = td_path / "variant.raw.docx"
                raw_emf = td_path / "variant.raw.image1.emf"
                run(
                    [
                        "cargo",
                        "run",
                        "-q",
                        "-p",
                        "chemcore-office",
                        "--",
                        "--write-word-docx-payload",
                        str(payload_json),
                        str(raw_docx),
                    ],
                    env=env,
                )
                extract_image1(raw_docx, raw_emf)
                shell_docx = td_path / "variant.shell.docx"
                run(
                    [
                        "D:\\anaconda3\\python.exe",
                        str(ROOT / "scripts" / "patch-docx-image1.py"),
                        str(template_docx),
                        str(raw_emf),
                        str(shell_docx),
                    ]
                )
                run(
                    [
                        "D:\\anaconda3\\python.exe",
                        str(ROOT / "scripts" / "patch-docx-image1-frame.py"),
                        str(shell_docx),
                        "--frame",
                        args.frame,
                        str(variant_docx),
                    ]
                )
            run(
                [
                    "powershell",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(ROOT / "scripts" / "word-copy-inline-shape.ps1"),
                    "-InputDocx",
                    str(variant_docx),
                    "-ShapeIndex",
                    "1",
                    "-OutputPng",
                    str(variant_png),
                ]
            )
            run(
                [
                    "D:\\anaconda3\\python.exe",
                    str(ROOT / "scripts" / "png-best-shift.py"),
                    str(variant_png),
                    str(reference_png),
                    "--limit",
                    "20",
                    "--output",
                    str(variant_best),
                ]
            )
            best = json.loads(variant_best.read_text(encoding="utf-8"))
            run(
                [
                    "D:\\anaconda3\\python.exe",
                    str(ROOT / "scripts" / "compare-full-label-iou.py"),
                    str(label_json),
                    str(variant_png),
                    str(reference_png),
                    str(variant_label),
                    "--dx",
                    str(best["dx"]),
                    "--dy",
                    str(best["dy"]),
                    "--pad",
                    "3",
                ]
            )
            label_map = label_iou_map(variant_label)
            baseline_iou = baseline_label_map.get(node_id, {}).get("iou")
            variant_iou = label_map.get(node_id, {}).get("iou")
            results["variants"].append(
                {
                    "nodeId": node_id,
                    "action": action,
                    "globalIoU": best["best_iou"],
                    "globalDelta": best["best_iou"] - baseline_best_json["best_iou"],
                    "dx": best["dx"],
                    "dy": best["dy"],
                    "labelIoU": variant_iou,
                    "labelDelta": (variant_iou - baseline_iou) if baseline_iou is not None and variant_iou is not None else None,
                    "dir": str(variant_dir.relative_to(ROOT)),
                }
            )

    out_path = out_dir / "summary.json"
    out_path.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
