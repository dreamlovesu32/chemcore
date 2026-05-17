from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Row:
    node_id: str
    text: str
    fill: str | None
    phase: float
    dy1: float
    dy3: float


def load_rows(path: Path) -> list[Row]:
    data = json.loads(path.read_text(encoding="utf-8"))
    rows: list[Row] = []
    for row in data["rows"]:
        if not row.get("component") or not row.get("text"):
            continue
        rows.append(
            Row(
                node_id=row["nodeId"],
                text=row["text"],
                fill=row.get("fill"),
                phase=float(row["topPagePhase"]),
                dy1=float(row.get("frame_dy1DeltaIou") or 0.0),
                dy3=float(row.get("frame_dy3DeltaIou") or 0.0),
            )
        )
    return rows


def band_of(sorted_phases: list[float], start_idx: int, end_idx: int) -> tuple[float, float]:
    """Return [lo, hi) band covering phases[start_idx:end_idx+1]."""
    lo_src = sorted_phases[start_idx]
    hi_src = sorted_phases[end_idx]
    prev_phase = sorted_phases[start_idx - 1] if start_idx > 0 else None
    next_phase = sorted_phases[end_idx + 1] if end_idx + 1 < len(sorted_phases) else None
    lo = 0.0 if prev_phase is None else (prev_phase + lo_src) / 2.0
    hi = 1.0 if next_phase is None else (hi_src + next_phase) / 2.0
    return (lo, hi)


def in_band(phase: float, band: tuple[float, float]) -> bool:
    lo, hi = band
    return lo <= phase < hi


def action_delta(row: Row, action: str) -> float:
    if action == "1":
        return row.dy1
    if action == "3":
        return row.dy3
    return 0.0


def evaluate_policy(
    rows: list[Row],
    bands: list[tuple[tuple[float, float], str]],
) -> dict:
    total = 0.0
    applied = []
    action_counts = {"0": 0, "1": 0, "3": 0}
    for row in rows:
        action = "0"
        for band, candidate_action in bands:
            if in_band(row.phase, band):
                action = candidate_action
                break
        action_counts[action] += 1
        delta = action_delta(row, action)
        total += delta
        applied.append(
            {
                "nodeId": row.node_id,
                "text": row.text,
                "fill": row.fill,
                "phase": row.phase,
                "action": action,
                "delta": delta,
            }
        )
    applied.sort(key=lambda r: (r["action"], r["phase"], r["nodeId"]))
    return {
        "totalDeltaIou": total,
        "avgDeltaIou": total / len(rows) if rows else 0.0,
        "actionCounts": action_counts,
        "selectedNegativeCount": sum(
            1 for row in applied if row["action"] != "0" and row["delta"] < 0.0
        ),
        "rows": applied,
    }


def search_one_band(rows: list[Row]) -> list[dict]:
    phases = sorted({row.phase for row in rows})
    best: list[dict] = []
    for i in range(len(phases)):
        for j in range(i, len(phases)):
            band = band_of(phases, i, j)
            for action in ("1", "3"):
                result = evaluate_policy(rows, [(band, action)])
                best.append(
                    {
                        "policy": [{"band": band, "action": action}],
                        **result,
                    }
                )
    best.sort(key=lambda r: (-r["totalDeltaIou"], str(r["policy"])))
    return best[:10]


def search_one_band_safe(rows: list[Row]) -> list[dict]:
    phases = sorted({row.phase for row in rows})
    best: list[dict] = []
    for i in range(len(phases)):
        for j in range(i, len(phases)):
            band = band_of(phases, i, j)
            band_rows = [row for row in rows if in_band(row.phase, band)]
            if not band_rows:
                continue
            for action in ("1", "3"):
                if any(action_delta(row, action) < 0.0 for row in band_rows):
                    continue
                result = evaluate_policy(rows, [(band, action)])
                best.append(
                    {
                        "policy": [{"band": band, "action": action}],
                        **result,
                    }
                )
    best.sort(key=lambda r: (-r["totalDeltaIou"], -r["actionCounts"]["1"] - r["actionCounts"]["3"], str(r["policy"])))
    return best[:10]


def search_two_band(rows: list[Row]) -> list[dict]:
    phases = sorted({row.phase for row in rows})
    best: list[dict] = []
    n = len(phases)
    for i1 in range(n):
        for j1 in range(i1, n):
            band1 = band_of(phases, i1, j1)
            for i2 in range(j1 + 1, n):
                for j2 in range(i2, n):
                    band2 = band_of(phases, i2, j2)
                    for action1 in ("1", "3"):
                        for action2 in ("1", "3"):
                            result = evaluate_policy(rows, [(band1, action1), (band2, action2)])
                            best.append(
                                {
                                    "policy": [
                                        {"band": band1, "action": action1},
                                        {"band": band2, "action": action2},
                                    ],
                                    **result,
                                }
                            )
    best.sort(key=lambda r: (-r["totalDeltaIou"], str(r["policy"])))
    return best[:20]


def search_two_band_safe(rows: list[Row]) -> list[dict]:
    phases = sorted({row.phase for row in rows})
    best: list[dict] = []
    n = len(phases)
    for i1 in range(n):
        for j1 in range(i1, n):
            band1 = band_of(phases, i1, j1)
            rows1 = [row for row in rows if in_band(row.phase, band1)]
            if not rows1:
                continue
            for i2 in range(j1 + 1, n):
                for j2 in range(i2, n):
                    band2 = band_of(phases, i2, j2)
                    rows2 = [row for row in rows if in_band(row.phase, band2)]
                    if not rows2:
                        continue
                    for action1 in ("1", "3"):
                        if any(action_delta(row, action1) < 0.0 for row in rows1):
                            continue
                        for action2 in ("1", "3"):
                            if any(action_delta(row, action2) < 0.0 for row in rows2):
                                continue
                            result = evaluate_policy(rows, [(band1, action1), (band2, action2)])
                            best.append(
                                {
                                    "policy": [
                                        {"band": band1, "action": action1},
                                        {"band": band2, "action": action2},
                                    ],
                                    **result,
                                }
                            )
    best.sort(key=lambda r: (-r["totalDeltaIou"], -r["actionCounts"]["1"] - r["actionCounts"]["3"], str(r["policy"])))
    return best[:20]


def main() -> None:
    ap = argparse.ArgumentParser(
        description=(
            "Search simple topPagePhase band policies for attached-label dy nudges "
            "using existing same-shell sensitivity deltas."
        )
    )
    ap.add_argument("attached_page_phase_json")
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    rows = load_rows(Path(args.attached_page_phase_json))
    rows.sort(key=lambda r: (r.phase, r.node_id))

    payload = {
        "source": str(Path(args.attached_page_phase_json).resolve()),
        "rowCount": len(rows),
        "rows": [
            {
                "nodeId": row.node_id,
                "text": row.text,
                "fill": row.fill,
                "phase": row.phase,
                "dy1": row.dy1,
                "dy3": row.dy3,
            }
            for row in rows
        ],
        "bestOneBandPolicies": search_one_band(rows),
        "bestOneBandSafePolicies": search_one_band_safe(rows),
        "bestTwoBandPolicies": search_two_band(rows),
        "bestTwoBandSafePolicies": search_two_band_safe(rows),
    }
    Path(args.output).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
