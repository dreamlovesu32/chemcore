from __future__ import annotations

import argparse
import re
import zipfile
from pathlib import Path


OBJECT_RE = re.compile(
    r'(<w:object\b[^>]*\bw:dxaOrig=")(\d+)(" [^>]*\bw:dyaOrig=")(\d+)(">)'
    r'(.*?)'
    r'(<v:shape\b[^>]*\bstyle="width:)([0-9.]+)(pt;height:)([0-9.]+)(pt"[^>]*>)',
    re.DOTALL,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Patch the first Word OLE object's natural size and display size inside a .docx package."
    )
    parser.add_argument("input_docx")
    parser.add_argument("output_docx")
    parser.add_argument("--width-pt", type=float, help="display width in points")
    parser.add_argument("--height-pt", type=float, help="display height in points")
    parser.add_argument("--dxa-orig", type=int, help="natural width in twips")
    parser.add_argument("--dya-orig", type=int, help="natural height in twips")
    parser.add_argument(
        "--display-scale",
        type=float,
        help="multiply existing display width/height by this factor",
    )
    parser.add_argument(
        "--natural-scale",
        type=float,
        help="multiply existing dxaOrig/dyaOrig by this factor",
    )
    return parser.parse_args()


def patch_document_xml(
    xml: str,
    *,
    width_pt: float | None,
    height_pt: float | None,
    dxa_orig: int | None,
    dya_orig: int | None,
    display_scale: float | None,
    natural_scale: float | None,
) -> str:
    match = OBJECT_RE.search(xml)
    if not match:
        raise RuntimeError("could not find first w:object + v:shape pair in word/document.xml")

    old_dxa = int(match.group(2))
    old_dya = int(match.group(4))
    old_width_pt = float(match.group(8))
    old_height_pt = float(match.group(10))

    new_dxa = dxa_orig if dxa_orig is not None else old_dxa
    new_dya = dya_orig if dya_orig is not None else old_dya
    new_width_pt = width_pt if width_pt is not None else old_width_pt
    new_height_pt = height_pt if height_pt is not None else old_height_pt

    if natural_scale is not None:
        new_dxa = int(round(old_dxa * natural_scale))
        new_dya = int(round(old_dya * natural_scale))

    if display_scale is not None:
        new_width_pt = old_width_pt * display_scale
        new_height_pt = old_height_pt * display_scale

    replacement = (
        f"{match.group(1)}{new_dxa}{match.group(3)}{new_dya}{match.group(5)}"
        f"{match.group(6)}"
        f"{match.group(7)}{new_width_pt:.3f}{match.group(9)}{new_height_pt:.3f}{match.group(11)}"
    )
    return xml[: match.start()] + replacement + xml[match.end() :]


def main() -> None:
    args = parse_args()
    input_docx = Path(args.input_docx)
    output_docx = Path(args.output_docx)
    output_docx.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(input_docx, "r") as zin, zipfile.ZipFile(
        output_docx, "w", compression=zipfile.ZIP_STORED
    ) as zout:
        for info in zin.infolist():
            data = zin.read(info.filename)
            if info.filename == "word/document.xml":
                xml = data.decode("utf-8")
                xml = patch_document_xml(
                    xml,
                    width_pt=args.width_pt,
                    height_pt=args.height_pt,
                    dxa_orig=args.dxa_orig,
                    dya_orig=args.dya_orig,
                    display_scale=args.display_scale,
                    natural_scale=args.natural_scale,
                )
                data = xml.encode("utf-8")
            zout.writestr(info, data)

    print(f"patched {output_docx}")


if __name__ == "__main__":
    main()
