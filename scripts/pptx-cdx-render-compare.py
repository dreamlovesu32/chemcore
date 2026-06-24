from __future__ import annotations

import argparse
import io
import json
import re
import shlex
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path

import olefile

PREFERRED_STREAMS = ["CONTENTS", "OlePres000", "OlePres001", "Package"]


def natural_sort_key(value: str) -> list[object]:
    parts = re.split(r"(\d+)", value or "")
    out: list[object] = []
    for part in parts:
        out.append(int(part) if part.isdigit() else part.lower())
    return out


def safe_stem(value: str, *, max_len: int = 130) -> str:
    stem = re.sub(r'[<>:"/\\|?*\x00-\x1f]+', "_", value).strip(" ._")
    stem = re.sub(r"\s+", " ", stem)
    return (stem or "item")[:max_len]


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive.rstrip(":").lower()
    rest = resolved.as_posix()[3:] if len(resolved.as_posix()) > 2 else ""
    return f"/mnt/{drive}/{rest}"


def list_streams(ole: olefile.OleFileIO) -> list[str]:
    return ["/".join(parts) for parts in ole.listdir()]


def read_best_stream(ole: olefile.OleFileIO) -> tuple[str | None, bytes | None]:
    streams = list_streams(ole)
    for stream in PREFERRED_STREAMS:
        if stream in streams:
            with ole.openstream(stream) as fp:
                return stream, fp.read()
    candidates: list[tuple[str, bytes]] = []
    for stream in streams:
        try:
            with ole.openstream(stream) as fp:
                data = fp.read()
        except Exception:
            continue
        if len(data) >= 4 and data[:4] == b"VjCD":
            candidates.append((stream, data))
    if candidates:
        candidates.sort(key=lambda item: len(item[1]), reverse=True)
        return candidates[0]
    return None, None


def extract_cdx_from_ole_bytes(data: bytes) -> tuple[str | None, bytes | None]:
    try:
        ole = olefile.OleFileIO(io.BytesIO(data))
    except Exception:
        return None, None
    with ole:
        return read_best_stream(ole)


def discover_pptx(inputs: list[str]) -> list[Path]:
    paths: list[Path] = []
    for raw in inputs:
        path = Path(raw)
        if path.is_dir():
            paths.extend(path.rglob("*.pptx"))
            paths.extend(path.rglob("*.pptm"))
            paths.extend(path.rglob("*.ppt"))
        elif path.is_file() and path.suffix.lower() in {".pptx", ".pptm", ".ppt"}:
            paths.append(path)
    dedup: dict[str, Path] = {}
    for path in paths:
        dedup[str(path.resolve()).lower()] = path.resolve()
    return sorted(dedup.values(), key=lambda p: natural_sort_key(str(p)))


def extract_all_cdx(pptx_paths: list[Path], out_root: Path) -> list[dict[str, object]]:
    cdx_dir = out_root / "cdx"
    cdx_dir.mkdir(parents=True, exist_ok=True)
    records: list[dict[str, object]] = []
    for ppt_index, pptx in enumerate(pptx_paths, start=1):
        print(f"[PPTX] {pptx}")
        try:
            archive = zipfile.ZipFile(pptx)
        except Exception as exc:
            print(f"[PPTX_FAIL] {pptx} :: {exc}")
            continue
        with archive:
            embeddings = [
                name
                for name in archive.namelist()
                if name.lower().startswith("ppt/embeddings/") and name.lower().endswith(".bin")
            ]
            embeddings.sort(key=natural_sort_key)
            if not embeddings:
                print("[INFO] no ppt/embeddings/*.bin")
                continue
            for embedding in embeddings:
                data = archive.read(embedding)
                stream_name, cdx_bytes = extract_cdx_from_ole_bytes(data)
                if not cdx_bytes:
                    continue
                group = safe_stem(pptx.parent.name, max_len=32)
                ppt_stem = safe_stem(pptx.stem, max_len=72)
                ole_stem = safe_stem(Path(embedding).stem, max_len=32)
                stem = safe_stem(f"{len(records)+1:04d}_{group}__{ppt_stem}__{ole_stem}", max_len=150)
                cdx_path = cdx_dir / f"{stem}.cdx"
                cdx_path.write_bytes(cdx_bytes)
                record = {
                    "index": len(records) + 1,
                    "stem": stem,
                    "pptx": str(pptx),
                    "pptx_name": pptx.name,
                    "embedding": embedding,
                    "stream": stream_name,
                    "cdx": str(cdx_path),
                }
                records.append(record)
                print(f"[CDX] {embedding} / {stream_name} -> {cdx_path.name}")
    return records


def convert_cdx_to_cdxml_with_pycdxml(records: list[dict[str, object]], out_root: Path) -> None:
    cdxml_dir = out_root / "cdxml"
    cdxml_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    for record in records:
        cdxml_path = cdxml_dir / f"{record['stem']}.cdxml"
        record["cdxml"] = str(cdxml_path)
        manifest.append(
            {
                "cdx": windows_to_wsl(Path(str(record["cdx"]))),
                "cdxml": windows_to_wsl(cdxml_path),
            }
        )
    manifest_path = out_root / "cdx_to_cdxml_manifest.json"
    helper_path = out_root / "convert_cdx_to_cdxml.py"
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    helper_path.write_text(
        """
import json
import sys
from pathlib import Path
from pycdxml import cdxml_converter

items = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
ok = 0
fail = 0
for item in items:
    try:
        doc = cdxml_converter.read_cdx(item["cdx"], ignore_unknown_properties=True)
        Path(item["cdxml"]).parent.mkdir(parents=True, exist_ok=True)
        cdxml_converter.write_cdxml_file(doc, item["cdxml"])
        print(f"[CDXML] {item['cdx']} -> {item['cdxml']}")
        ok += 1
    except Exception as exc:
        print(f"[CDXML_FAIL] {item['cdx']} :: {exc!r}")
        fail += 1
print(f"CDXML_DONE OK={ok} FAIL={fail}")
sys.exit(1 if fail else 0)
""".lstrip(),
        encoding="utf-8",
    )
    command = (
        "cd /home/jiajun/chemrecords && python3 "
        f"{shlex.quote(windows_to_wsl(helper_path))} {shlex.quote(windows_to_wsl(manifest_path))}"
    )
    run(["wsl", "-e", "bash", "-lc", command], cwd=Path.cwd())


def run(args: list[str], *, cwd: Path) -> None:
    print(f"[RUN] {' '.join(args)}")
    result = subprocess.run(args, cwd=str(cwd), text=True)
    if result.returncode != 0:
        raise RuntimeError(f"command failed with exit code {result.returncode}: {' '.join(args)}")


def run_chemdraw_svg(records: list[dict[str, object]], out_root: Path, repo_root: Path, chunk_size: int) -> None:
    out_dir = out_root / "svg" / "chemdraw"
    out_dir.mkdir(parents=True, exist_ok=True)
    cdxml_paths = [str(record["cdxml"]) for record in records if Path(str(record.get("cdxml", ""))).exists()]
    for start in range(0, len(cdxml_paths), chunk_size):
        chunk = cdxml_paths[start : start + chunk_size]
        run(
            [
                "node",
                "scripts/chemdraw-oracle.mjs",
                "--out",
                str(out_dir),
                "--formats",
                "svg",
                *chunk,
            ],
            cwd=repo_root,
        )
    for record in records:
        record["chemdraw_svg"] = str(out_dir / f"{record['stem']}.chemdraw.svg")


def build_chemcore_example(repo_root: Path) -> Path:
    run(["cargo", "build", "-p", "chemcore-engine", "--example", "cdxml_to_svg"], cwd=repo_root)
    exe = repo_root / "target" / "debug" / "examples" / "cdxml_to_svg.exe"
    if not exe.exists():
        raise RuntimeError(f"missing built example: {exe}")
    return exe


def run_chemcore_svg(records: list[dict[str, object]], out_root: Path, repo_root: Path) -> None:
    out_dir = out_root / "svg" / "chemcore"
    out_dir.mkdir(parents=True, exist_ok=True)
    exe = build_chemcore_example(repo_root)
    for record in records:
        cdxml = Path(str(record.get("cdxml", "")))
        if not cdxml.exists():
            continue
        output = out_dir / f"{record['stem']}.chemcore.svg"
        run([str(exe), str(cdxml), str(output)], cwd=repo_root)
        record["chemcore_svg"] = str(output)


def render_compare_pngs(records: list[dict[str, object]], out_root: Path, repo_root: Path, scale: int) -> None:
    png_dir = out_root / "compare_png"
    png_dir.mkdir(parents=True, exist_ok=True)
    items = []
    for record in records:
        chemdraw_svg = Path(str(record.get("chemdraw_svg", "")))
        chemcore_svg = Path(str(record.get("chemcore_svg", "")))
        if not chemdraw_svg.exists() or not chemcore_svg.exists():
            continue
        output = png_dir / f"{record['stem']}.compare.png"
        record["compare_png"] = str(output)
        items.append(
            {
                "stem": record["stem"],
                "title": record["stem"],
                "sourcePpt": record.get("pptx_name", ""),
                "embedding": record.get("embedding", ""),
                "chemdrawSvg": str(chemdraw_svg),
                "chemcoreSvg": str(chemcore_svg),
                "output": str(output),
            }
        )
    manifest_path = out_root / "png_manifest.json"
    manifest_path.write_text(json.dumps({"items": items}, ensure_ascii=False, indent=2), encoding="utf-8")
    run(
        [
            "node",
            "scripts/render-svg-compare-pngs.mjs",
            "--manifest",
            str(manifest_path),
            "--scale",
            str(scale),
        ],
        cwd=repo_root,
    )


def write_manifest(records: list[dict[str, object]], pptx_paths: list[Path], out_root: Path) -> None:
    payload = {
        "created_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "pptx_count": len(pptx_paths),
        "record_count": len(records),
        "pptx": [str(path) for path in pptx_paths],
        "records": records,
    }
    (out_root / "manifest.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    lines = [
        "# ChemDraw / ChemCore PPTX CDX Render Comparison",
        "",
        f"- PPTX files: {len(pptx_paths)}",
        f"- Extracted CDX/CDXML: {len(records)}",
        f"- PNG folder: `{out_root / 'compare_png'}`",
        "",
        "| # | PPTX | Embedding | PNG |",
        "| ---: | --- | --- | --- |",
    ]
    for record in records:
        png = record.get("compare_png", "")
        lines.append(
            f"| {record['index']} | {record.get('pptx_name','')} | {record.get('embedding','')} | {Path(str(png)).name if png else ''} |"
        )
    (out_root / "README.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract PPTX ChemDraw CDX, render ChemDraw/ChemCore SVG, and make side-by-side PNGs.")
    parser.add_argument("inputs", nargs="+", help="PPTX files or folders containing PPTX files")
    parser.add_argument("--out", default=None, help="Output folder")
    parser.add_argument("--repo-root", default=str(Path.cwd()), help="ChemCore repository root")
    parser.add_argument("--chemdraw-chunk-size", type=int, default=20)
    parser.add_argument("--png-scale", type=int, default=3)
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    if args.out:
        out_root = Path(args.out).resolve()
    else:
        stamp = time.strftime("%Y%m%d-%H%M%S")
        out_root = Path.home() / "Desktop" / f"chemcore-chemdraw-pptx-compare-{stamp}"
    out_root.mkdir(parents=True, exist_ok=True)

    pptx_paths = discover_pptx(args.inputs)
    if not pptx_paths:
        raise RuntimeError("no PPTX/PPTM/PPT files found")
    print(f"[INFO] output: {out_root}")
    print(f"[INFO] pptx files: {len(pptx_paths)}")

    records = extract_all_cdx(pptx_paths, out_root)
    if not records:
        raise RuntimeError("no CDX objects extracted")
    convert_cdx_to_cdxml_with_pycdxml(records, out_root)
    run_chemdraw_svg(records, out_root, repo_root, args.chemdraw_chunk_size)
    run_chemcore_svg(records, out_root, repo_root)
    render_compare_pngs(records, out_root, repo_root, args.png_scale)
    write_manifest(records, pptx_paths, out_root)
    print(f"[DONE] output: {out_root}")
    print(f"[DONE] compare PNGs: {out_root / 'compare_png'}")


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[ERROR] {exc}", file=sys.stderr)
        sys.exit(1)
