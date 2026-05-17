from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
import zipfile
from pathlib import Path

import numpy as np
from PIL import Image


ROOT = Path(__file__).resolve().parents[1]


def run(cmd: list[str], env: dict[str, str] | None = None) -> None:
    subprocess.run(cmd, check=True, cwd=ROOT, env=env)


def load_mask(path: Path, threshold: int = 740) -> np.ndarray:
    image = Image.open(path).convert("RGBA")
    rgba = np.asarray(image)
    rgb = rgba[..., :3].astype(np.int16)
    alpha = rgba[..., 3].astype(np.int16)
    return (alpha > 0) & (rgb.sum(axis=2) < threshold)


def best_shift(ours: np.ndarray, reference: np.ndarray, limit: int = 4) -> dict[str, int | float]:
    best_key = (-1.0, -1, -10**9)
    best_data: dict[str, int | float] | None = None
    height = min(ours.shape[0], reference.shape[0])
    width = min(ours.shape[1], reference.shape[1])
    ours = ours[:height, :width]
    reference = reference[:height, :width]
    for dy in range(-limit, limit + 1):
        for dx in range(-limit, limit + 1):
            ours_y0 = max(0, dy)
            ref_y0 = max(0, -dy)
            ours_x0 = max(0, dx)
            ref_x0 = max(0, -dx)
            h = height - abs(dy)
            w = width - abs(dx)
            if h <= 0 or w <= 0:
                continue
            ours_crop = ours[ours_y0 : ours_y0 + h, ours_x0 : ours_x0 + w]
            ref_crop = reference[ref_y0 : ref_y0 + h, ref_x0 : ref_x0 + w]
            inter = int(np.count_nonzero(ours_crop & ref_crop))
            ours_only = int(np.count_nonzero(ours_crop & ~ref_crop))
            ref_only = int(np.count_nonzero(ref_crop & ~ours_crop))
            union = inter + ours_only + ref_only
            iou = 1.0 if union == 0 else inter / union
            key = (iou, inter, -(abs(dx) + abs(dy)))
            if key > best_key:
                best_key = key
                best_data = {
                    "dx": dx,
                    "dy": dy,
                    "iou": iou,
                    "intersection": inter,
                    "only_ours": ours_only,
                    "only_reference": ref_only,
                }
    assert best_data is not None
    return best_data


def mask_box_counts(
    ours: np.ndarray,
    reference: np.ndarray,
    box: list[int],
    dx: int,
    dy: int,
) -> dict[str, int | float]:
    x0, y0, x1, y1 = box
    x0 = max(0, x0)
    y0 = max(0, y0)
    x1 = min(reference.shape[1] - 1, x1)
    y1 = min(reference.shape[0] - 1, y1)
    inter = ours_only = ref_only = 0
    for ry in range(y0, y1 + 1):
        oy = ry + dy
        if oy < 0 or oy >= ours.shape[0]:
            continue
        for rx in range(x0, x1 + 1):
            ox = rx + dx
            if ox < 0 or ox >= ours.shape[1]:
                continue
            rv = bool(reference[ry, rx])
            ov = bool(ours[oy, ox])
            if ov and rv:
                inter += 1
            elif ov:
                ours_only += 1
            elif rv:
                ref_only += 1
    union = inter + ours_only + ref_only
    return {
        "intersection": inter,
        "oursOnly": ours_only,
        "refOnly": ref_only,
        "union": union,
        "iou": 1.0 if union == 0 else inter / union,
    }


def extract_image1(docx_path: Path, output_emf: Path) -> None:
    with zipfile.ZipFile(docx_path, "r") as zf:
        output_emf.write_bytes(zf.read("word/media/image1.emf"))


def label_iou_rows(
    label_json: Path,
    ours_png: Path,
    ref_png: Path,
    dx: int,
    dy: int,
    pad: int = 0,
) -> dict[str, dict[str, int | float | str | list[int] | None]]:
    labels = json.loads(label_json.read_text(encoding="utf-8"))["labels"]
    ours = load_mask(ours_png)
    ref = load_mask(ref_png)
    rows: dict[str, dict[str, int | float | str | list[int] | None]] = {}
    for row in labels:
        x0, y0, x1, y1 = row["pixelBox"]
        stats = mask_box_counts(
            ours=ours,
            reference=ref,
            box=[x0 - pad, y0 - pad, x1 + pad, y1 + pad],
            dx=dx,
            dy=dy,
        )
        rows[row["nodeId"]] = {
            "nodeId": row["nodeId"],
            "text": row.get("text", ""),
            "fill": row.get("fill"),
            "layout": row.get("layout"),
            "pixelBox": row["pixelBox"],
            **stats,
        }
    return rows


def main() -> None:
    ap = argparse.ArgumentParser(description="Run same-shell attached-label text-hint atlas on top of a stacked replay baseline.")
    ap.add_argument("--payload-json", required=True)
    ap.add_argument("--phase-json", required=True)
    ap.add_argument("--template-docx", required=True)
    ap.add_argument("--reference-png", required=True)
    ap.add_argument("--label-json", required=True)
    ap.add_argument("--frame", required=True, help="left,top,right,bottom")
    ap.add_argument("--phase-policy", default="phase3band")
    ap.add_argument("--baseline-x-filter", required=True)
    ap.add_argument("--baseline-x-nudge", type=float, default=1.0)
    ap.add_argument("--baseline-x2-filter", default="")
    ap.add_argument("--baseline-x2-nudge", type=float, default=0.0)
    ap.add_argument("--font-scale", type=float, default=0.97)
    ap.add_argument("--font-scale-filter", default="")
    ap.add_argument("--text-hint", type=int, required=True)
    ap.add_argument("--output-dir", required=True)
    ap.add_argument("--nodes", default="", help="optional comma-separated subset")
    args = ap.parse_args()

    payload_json = (ROOT / args.payload_json).resolve()
    phase_json = (ROOT / args.phase_json).resolve()
    template_docx = (ROOT / args.template_docx).resolve()
    reference_png = (ROOT / args.reference_png).resolve()
    label_json = (ROOT / args.label_json).resolve()
    out_dir = (ROOT / args.output_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    frame = args.frame
    phase_rows = json.loads(phase_json.read_text(encoding="utf-8"))["rows"]
    nodes = [row["nodeId"] for row in phase_rows if row.get("component") and row.get("text")]
    if args.nodes.strip():
        allowed = {part.strip() for part in args.nodes.split(",") if part.strip()}
        nodes = [node for node in nodes if node in allowed]

    baseline_png = out_dir / "baseline.wordcopy.png"
    baseline_docx = out_dir / "baseline.fg3.docx"
    baseline_emf = out_dir / "baseline.raw.image1.emf"
    baseline_best_path = out_dir / "baseline.bestshift.json"
    baseline_label_path = out_dir / "baseline.label-iou.json"

    common_env = os.environ.copy()
    common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_PHASE_POLICY_EXPERIMENT"] = args.phase_policy
    common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT"] = str(args.baseline_x_nudge)
    common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT"] = args.baseline_x_filter
    if args.baseline_x2_filter.strip():
        common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_EXPERIMENT_2"] = str(args.baseline_x2_nudge)
        common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_NUDGE_NODE_FILTER_EXPERIMENT_2"] = args.baseline_x2_filter
    if args.font_scale_filter.strip():
        common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_EXPERIMENT"] = str(args.font_scale)
        common_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_FONT_SCALE_NODE_FILTER_EXPERIMENT"] = args.font_scale_filter

    baseline_ready = baseline_png.exists() and baseline_best_path.exists() and baseline_label_path.exists()
    if not baseline_ready:
        with tempfile.TemporaryDirectory(prefix="attached_hint_baseline_", dir=str(out_dir)) as td:
            td_path = Path(td)
            raw_docx = td_path / "baseline.raw.docx"
            run(
                [
                    "cargo", "run", "-q", "-p", "chemcore-office", "--",
                    "--write-word-docx-payload", str(payload_json), str(raw_docx),
                ],
                env=common_env,
            )
            extract_image1(raw_docx, baseline_emf)
        run(["D:\\anaconda3\\python.exe", str(ROOT / "scripts" / "patch-docx-image1.py"), str(template_docx), str(baseline_emf), str(out_dir / "baseline.shell.docx")])
        run(["D:\\anaconda3\\python.exe", str(ROOT / "scripts" / "patch-docx-image1-frame.py"), str(out_dir / "baseline.shell.docx"), str(baseline_docx), "--frame", frame])
        run(["powershell", "-ExecutionPolicy", "Bypass", "-File", str(ROOT / "scripts" / "word-copy-inline-shape.ps1"), "-InputDocx", str(baseline_docx), "-OutputPng", str(baseline_png)])
        baseline_best = best_shift(load_mask(baseline_png), load_mask(reference_png))
        baseline_best_path.write_text(json.dumps(baseline_best, indent=2), encoding="utf-8")
        baseline_rows = label_iou_rows(label_json, baseline_png, reference_png, int(baseline_best["dx"]), int(baseline_best["dy"]))
        baseline_label_path.write_text(json.dumps({"rows": list(baseline_rows.values())}, indent=2), encoding="utf-8")
    else:
        baseline_best = json.loads(baseline_best_path.read_text(encoding="utf-8"))
        baseline_rows = {row["nodeId"]: row for row in json.loads(baseline_label_path.read_text(encoding="utf-8"))["rows"]}

    summary: list[dict[str, object]] = []
    for node_id in nodes:
        variant_env = dict(common_env)
        variant_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TEXT_HINT_EXPERIMENT"] = str(args.text_hint)
        variant_env["CHEMCORE_EMF_ATTACHED_LABEL_REPLAY_TEXT_HINT_NODE_FILTER_EXPERIMENT"] = node_id

        raw_docx = out_dir / f"{node_id}.raw.docx"
        raw_emf = out_dir / f"{node_id}.raw.image1.emf"
        shell_docx = out_dir / f"{node_id}.shell.docx"
        fg3_docx = out_dir / f"{node_id}.fg3.docx"
        wordcopy_png = out_dir / f"{node_id}.fg3.wordcopy.png"
        best_path = out_dir / f"{node_id}.bestshift.json"
        label_path = out_dir / f"{node_id}.label-iou.json"

        run(
            [
                "cargo", "run", "-q", "-p", "chemcore-office", "--",
                "--write-word-docx-payload", str(payload_json), str(raw_docx),
            ],
            env=variant_env,
        )
        extract_image1(raw_docx, raw_emf)
        run(["D:\\anaconda3\\python.exe", str(ROOT / "scripts" / "patch-docx-image1.py"), str(template_docx), str(raw_emf), str(shell_docx)])
        run(["D:\\anaconda3\\python.exe", str(ROOT / "scripts" / "patch-docx-image1-frame.py"), str(shell_docx), str(fg3_docx), "--frame", frame])
        run(["powershell", "-ExecutionPolicy", "Bypass", "-File", str(ROOT / "scripts" / "word-copy-inline-shape.ps1"), "-InputDocx", str(fg3_docx), "-OutputPng", str(wordcopy_png)])

        ours_mask = load_mask(wordcopy_png)
        ref_mask = load_mask(reference_png)
        best = best_shift(ours_mask, ref_mask)
        best_path.write_text(json.dumps(best, indent=2), encoding="utf-8")
        rows = label_iou_rows(label_json, wordcopy_png, reference_png, int(best["dx"]), int(best["dy"]))
        label_path.write_text(json.dumps({"rows": list(rows.values())}, indent=2), encoding="utf-8")

        base_label = baseline_rows[node_id]
        row = rows[node_id]
        summary.append({
            "nodeId": node_id,
            "globalIou": best["iou"],
            "globalDelta": best["iou"] - baseline_best["iou"],
            "labelIou": row["iou"],
            "labelDelta": row["iou"] - base_label["iou"],
            "text": row["text"],
            "fill": row["fill"],
        })

    summary.sort(key=lambda r: (-r["globalDelta"], -r["labelDelta"], r["nodeId"]))
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary[:25], indent=2))


if __name__ == "__main__":
    main()
