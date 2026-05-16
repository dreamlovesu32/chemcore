from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path


def parse_row(value: str) -> tuple[str, Path, Path]:
    parts = value.split("::")
    if len(parts) != 3:
        raise argparse.ArgumentTypeError(
            "row must be NAME::preview-bounds.json::chemdraw.emf"
        )
    name, bounds_path, emf_path = parts
    return name, Path(bounds_path), Path(emf_path)


def load_frame(path: Path) -> tuple[int, int, int, int]:
    data = path.read_bytes()
    return struct.unpack_from("<4i", data, 24)


def fit_linear(xs: list[float], ys: list[float]) -> dict[str, float]:
    if len(xs) != len(ys) or not xs:
        raise ValueError("xs/ys must be non-empty and aligned")
    if len(xs) == 1:
        return {"m": 0.0, "b": ys[0], "r2": 1.0}
    n = float(len(xs))
    x_mean = sum(xs) / n
    y_mean = sum(ys) / n
    sxx = sum((x - x_mean) ** 2 for x in xs)
    if sxx == 0:
        return {"m": 0.0, "b": y_mean, "r2": 0.0}
    sxy = sum((x - x_mean) * (y - y_mean) for x, y in zip(xs, ys))
    m = sxy / sxx
    b = y_mean - m * x_mean
    preds = [m * x + b for x in xs]
    ss_res = sum((y - p) ** 2 for y, p in zip(ys, preds))
    ss_tot = sum((y - y_mean) ** 2 for y in ys)
    r2 = 1.0 if ss_tot == 0 else 1.0 - ss_res / ss_tot
    return {"m": m, "b": b, "r2": r2}


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fit centered-family frame pads relative to visible bounds."
    )
    parser.add_argument(
        "--row",
        action="append",
        required=True,
        type=parse_row,
        help="NAME::preview-bounds.json::chemdraw.emf",
    )
    parser.add_argument("--output")
    args = parser.parse_args()

    rows = []
    for name, bounds_path, emf_path in args.row:
        bounds = json.loads(bounds_path.read_text(encoding="utf-8"))
        svg = bounds["svgViewBoxBoundsSvgPx"]
        visible = bounds["visibleBoundsSvgPx"]
        current = bounds["frameBoundsHimetric"]
        chem = load_frame(emf_path)
        factor = current["width"] / (svg[2] - svg[0])
        visible_frame = [round(v * factor) for v in visible]
        pads_hm = [chem[i] - visible_frame[i] for i in range(4)]
        pads_px = [pad / factor for pad in pads_hm]
        rows.append(
            {
                "name": name,
                "visibleWidthPx": visible[2] - visible[0],
                "visibleHeightPx": visible[3] - visible[1],
                "scaleHimetricPerSvgPx": factor,
                "chemFrameHimetric": list(chem),
                "visibleFrameHimetric": visible_frame,
                "padsHimetric": pads_hm,
                "padsPx": pads_px,
                "horizontalTotalPx": pads_px[0] + pads_px[2],
                "horizontalNetWidthPx": pads_px[2] - pads_px[0],
                "verticalTotalPx": pads_px[1] + pads_px[3],
            }
        )

    widths = [row["visibleWidthPx"] for row in rows]
    heights = [row["visibleHeightPx"] for row in rows]
    lefts = [row["padsPx"][0] for row in rows]
    rights = [row["padsPx"][2] for row in rows]
    tops = [row["padsPx"][1] for row in rows]
    bottoms = [row["padsPx"][3] for row in rows]

    result = {
        "rows": rows,
        "fits": {
            "left_vs_width": fit_linear(widths, lefts),
            "right_vs_width": fit_linear(widths, rights),
            "top_vs_height": fit_linear(heights, tops),
            "bottom_vs_height": fit_linear(heights, bottoms),
        },
    }

    text = json.dumps(result, indent=2, ensure_ascii=False)
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
    else:
        print(text)


if __name__ == "__main__":
    main()
