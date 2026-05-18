from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PYTHON = Path(r"D:\anaconda3\python.exe")


def sanitize_policy(policy: str) -> str:
    return (
        policy.replace(",", "__")
        .replace("+", "p")
        .replace("-", "m")
        .replace(" ", "")
    )


def run_one(policy: str, out_root: str) -> dict:
    cmd = [
        str(PYTHON),
        str(ROOT / "scripts" / "evaluate-ppt-general-label-policy.py"),
        "--policy",
        policy,
        "--out-root",
        out_root,
    ]
    result = subprocess.run(
        cmd,
        cwd=str(ROOT),
        text=True,
        capture_output=True,
        env=os.environ.copy(),
    )
    payload = {
        "policy": policy,
        "outRoot": out_root,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }
    summary_path = ROOT / out_root / "summary.json"
    if result.returncode == 0 and summary_path.exists():
        payload["summary"] = json.loads(summary_path.read_text(encoding="utf-8"))
    return payload


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run PPT general-label policy evaluations in parallel."
    )
    parser.add_argument(
        "--policy",
        action="append",
        default=[],
        help="Policy string to evaluate. Can be repeated.",
    )
    parser.add_argument(
        "--policy-file",
        default=None,
        help="Optional text file with one policy per line.",
    )
    parser.add_argument(
        "--out-parent",
        default="tmp/ppt-policy-matrix-par",
        help="Parent output directory for per-policy runs.",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=min(16, os.cpu_count() or 1),
        help="Parallel worker count.",
    )
    parser.add_argument(
        "--skip-existing",
        action="store_true",
        help="Skip policies whose summary.json already exists.",
    )
    args = parser.parse_args()

    policies: list[str] = []
    policies.extend(args.policy)
    if args.policy_file:
        for line in Path(args.policy_file).read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            policies.append(line)
    if not policies:
        raise SystemExit("No policies provided.")

    seen = set()
    unique_policies: list[str] = []
    for policy in policies:
        if policy in seen:
            continue
        seen.add(policy)
        unique_policies.append(policy)

    jobs = max(1, args.jobs)
    out_parent = ROOT / args.out_parent
    out_parent.mkdir(parents=True, exist_ok=True)

    todo: list[tuple[str, str]] = []
    skipped: list[dict] = []
    for policy in unique_policies:
        out_root = f"{args.out_parent}/{sanitize_policy(policy)}"
        summary_path = ROOT / out_root / "summary.json"
        if args.skip_existing and summary_path.exists():
            skipped.append(
                {
                    "policy": policy,
                    "outRoot": out_root,
                    "summaryPath": str(summary_path),
                }
            )
            continue
        todo.append((policy, out_root))

    completed: list[dict] = []
    failures: list[dict] = []

    with ProcessPoolExecutor(max_workers=jobs) as executor:
        future_map = {
            executor.submit(run_one, policy, out_root): (policy, out_root)
            for policy, out_root in todo
        }
        for future in as_completed(future_map):
            policy, out_root = future_map[future]
            try:
                result = future.result()
            except Exception as exc:  # noqa: BLE001
                failures.append(
                    {
                        "policy": policy,
                        "outRoot": out_root,
                        "error": repr(exc),
                    }
                )
                continue
            if result.get("returncode") == 0 and "summary" in result:
                summary = result["summary"]
                completed.append(
                    {
                        "policy": policy,
                        "outRoot": out_root,
                        "avgBestIou": summary.get("avgBestIou"),
                        "avgDx": summary.get("avgDx"),
                        "avgDy": summary.get("avgDy"),
                        "count": summary.get("count"),
                    }
                )
            else:
                failures.append(result)

    completed.sort(
        key=lambda item: (
            item["avgBestIou"] is None,
            -(item["avgBestIou"] or -1.0),
            item["policy"],
        )
    )

    report = {
        "jobs": jobs,
        "countPolicies": len(unique_policies),
        "countRun": len(todo),
        "countCompleted": len(completed),
        "countFailures": len(failures),
        "countSkipped": len(skipped),
        "completed": completed,
        "failures": failures,
        "skipped": skipped,
    }
    report_path = out_parent / "matrix-run-report.json"
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(report_path)
    print(
        json.dumps(
            {
                "jobs": jobs,
                "completed": len(completed),
                "failures": len(failures),
                "skipped": len(skipped),
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    for item in completed:
        print(
            f"{item['avgBestIou']:.6f}\t{item['policy']}\t"
            f"dx={item['avgDx']:.3f}\tdy={item['avgDy']:.3f}\tcount={item['count']}"
        )


if __name__ == "__main__":
    main()
