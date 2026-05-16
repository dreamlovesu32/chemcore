from __future__ import annotations

import argparse
import json
from pathlib import Path


def load_document(path: Path) -> dict:
    payload = json.loads(path.read_text(encoding="utf8"))
    if "chemcoreDocumentJson" in payload:
        return json.loads(payload["chemcoreDocumentJson"])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser(description="Dump text object layout features from a chemcore payload/doc JSON.")
    parser.add_argument("input_json")
    parser.add_argument("--output")
    args = parser.parse_args()

    doc = load_document(Path(args.input_json))
    rows: list[dict] = []
    for obj in doc.get("objects", []):
        if obj.get("type") != "text":
            continue
        payload = obj.get("payload", {})
        text = payload.get("text", "")
        runs = payload.get("runs", [])
        box = payload.get("box", [0, 0, 0, 0])
        scripts = sorted({run.get("script", "normal") for run in runs})
        normal_chars = sum(len(run.get("text", "")) for run in runs if run.get("script") == "normal")
        sub_chars = sum(len(run.get("text", "")) for run in runs if run.get("script") == "subscript")
        sup_chars = sum(len(run.get("text", "")) for run in runs if run.get("script") == "superscript")
        width = float(box[2]) - float(box[0])
        height = float(box[3]) - float(box[1])
        rows.append(
            {
                "id": obj.get("id"),
                "text": text,
                "align": payload.get("align"),
                "lines": text.count("\n") + 1,
                "runs": len(runs),
                "scripts": scripts,
                "normal_chars": normal_chars,
                "sub_chars": sub_chars,
                "sup_chars": sup_chars,
                "width": width,
                "height": height,
                "aspect": None if height == 0 else width / height,
                "baselineOffset": payload.get("baselineOffset"),
            }
        )

    text = json.dumps(rows, indent=2, ensure_ascii=False)
    if args.output:
        Path(args.output).write_text(text, encoding="utf8")
    else:
        print(text)


if __name__ == "__main__":
    main()
