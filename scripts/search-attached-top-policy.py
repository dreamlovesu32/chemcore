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
    d_m2: float
    d_m1: float
    d_p1: float
    d_p2: float


def load_rows(top_combined_json: Path, attached_phase_json: Path) -> list[Row]:
    top_rows = {
        row["nodeId"]: row
        for row in json.loads(top_combined_json.read_text(encoding="utf-8"))
    }
    phase_rows = json.loads(attached_phase_json.read_text(encoding="utf-8"))["rows"]
    rows: list[Row] = []
    for phase_row in phase_rows:
        node_id = phase_row["nodeId"]
        if node_id not in top_rows:
            continue
        if not phase_row.get("component") or not phase_row.get("text"):
            continue
        top_row = top_rows[node_id]
        rows.append(
            Row(
                node_id=node_id,
                text=phase_row.get("text", ""),
                fill=phase_row.get("fill"),
                phase=float(phase_row["topPagePhase"]),
                d_m2=float(top_row.get("globalDelta_top-2") or 0.0),
                d_m1=float(top_row.get("globalDelta_top-1") or 0.0),
                d_p1=float(top_row.get("globalDelta_top1") or 0.0),
                d_p2=float(top_row.get("globalDelta_top2") or 0.0),
            )
        )
    rows.sort(key=lambda row: (row.phase, row.node_id))
    return rows


def band_of(sorted_phases: list[float], start_idx: int, end_idx: int) -> tuple[float, float]:
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
    if action == "-2":
        return row.d_m2
    if action == "-1":
        return row.d_m1
    if action == "+1":
        return row.d_p1
    if action == "+2":
        return row.d_p2
    return 0.0


def evaluate_policy(
    rows: list[Row],
    bands: list[tuple[tuple[float, float], str]],
) -> dict:
    total = 0.0
    applied = []
    action_counts = {"0": 0, "-2": 0, "-1": 0, "+1": 0, "+2": 0}
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
    applied.sort(key=lambda item: (item["action"], item["phase"], item["nodeId"]))
    return {
        "totalDeltaIou": total,
        "avgDeltaIou": total / len(rows) if rows else 0.0,
        "actionCounts": action_counts,
        "selectedNegativeCount": sum(
            1 for row in applied if row["action"] != "0" and row["delta"] < 0.0
        ),
        "rows": applied,
    }


def search_band_policies(rows: list[Row], band_count: int, safe_only: bool) -> list[dict]:
    phases = sorted({row.phase for row in rows})
    phase_bands: list[tuple[tuple[float, float], list[Row]]] = []
    for i in range(len(phases)):
        for j in range(i, len(phases)):
            band = band_of(phases, i, j)
            members = [row for row in rows if in_band(row.phase, band)]
            if members:
                phase_bands.append((band, members))

    actions = ("-2", "-1", "+1", "+2")
    results: list[dict] = []

    def rec(
        start_idx: int,
        chosen: list[tuple[tuple[float, float], list[Row]]],
    ) -> None:
        if len(chosen) == band_count:
            def assign(idx: int, bands: list[tuple[tuple[float, float], str]]) -> None:
                if idx == len(chosen):
                    result = evaluate_policy(rows, bands)
                    results.append({"policy": bands, **result})
                    return
                band, members = chosen[idx]
                for action in actions:
                    if safe_only and any(action_delta(member, action) < 0.0 for member in members):
                        continue
                    assign(idx + 1, bands + [(band, action)])

            assign(0, [])
            return

        for next_idx in range(start_idx, len(phase_bands)):
            band, _ = phase_bands[next_idx]
            if chosen and band[0] < chosen[-1][0][1]:
                continue
            rec(next_idx + 1, chosen + [phase_bands[next_idx]])

    rec(0, [])
    results.sort(
        key=lambda item: (
            -item["totalDeltaIou"],
            item["selectedNegativeCount"],
            str(item["policy"]),
        )
    )
    limit = 10 if band_count == 1 else 20
    return results[:limit]


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Search topPagePhase band policies for attached-label packaged top-nudge "
            "experiments using precomputed atlas deltas."
        )
    )
    parser.add_argument("top_combined_json")
    parser.add_argument("attached_phase_json")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    rows = load_rows(Path(args.top_combined_json), Path(args.attached_phase_json))
    payload = {
        "sourceTopCombined": str(Path(args.top_combined_json).resolve()),
        "sourcePhase": str(Path(args.attached_phase_json).resolve()),
        "rowCount": len(rows),
        "rows": [
            {
                "nodeId": row.node_id,
                "text": row.text,
                "fill": row.fill,
                "phase": row.phase,
                "d_m2": row.d_m2,
                "d_m1": row.d_m1,
                "d_p1": row.d_p1,
                "d_p2": row.d_p2,
            }
            for row in rows
        ],
        "bestOneBandPolicies": search_band_policies(rows, band_count=1, safe_only=False),
        "bestOneBandSafePolicies": search_band_policies(rows, band_count=1, safe_only=True),
        "bestTwoBandPolicies": search_band_policies(rows, band_count=2, safe_only=False),
        "bestTwoBandSafePolicies": search_band_policies(rows, band_count=2, safe_only=True),
        "bestThreeBandPolicies": search_band_policies(rows, band_count=3, safe_only=False),
        "bestThreeBandSafePolicies": search_band_policies(rows, band_count=3, safe_only=True),
    }
    Path(args.output).write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
