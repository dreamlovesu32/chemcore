from __future__ import annotations

import argparse
import struct
import zipfile
from pathlib import Path


def parse_frame(value: str) -> tuple[int, int, int, int]:
    parts = [part.strip() for part in value.split(",")]
    if len(parts) != 4:
        raise argparse.ArgumentTypeError("frame must be left,top,right,bottom")
    try:
        left, top, right, bottom = (int(part) for part in parts)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("frame values must be integers") from exc
    return left, top, right, bottom


def patch_emf_frame(data: bytes, frame: tuple[int, int, int, int]) -> bytes:
    patched = bytearray(data)
    struct.pack_into("<4i", patched, 24, *frame)
    return bytes(patched)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Patch word/media/image1.emf header frame inside a docx package."
    )
    parser.add_argument("input_docx")
    parser.add_argument("output_docx")
    parser.add_argument(
        "--frame",
        required=True,
        type=parse_frame,
        help="left,top,right,bottom frame values in HIMETRIC units",
    )
    args = parser.parse_args()

    input_docx = Path(args.input_docx)
    output_docx = Path(args.output_docx)
    output_docx.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(input_docx, "r") as zin, zipfile.ZipFile(
        output_docx, "w", compression=zipfile.ZIP_STORED
    ) as zout:
        for info in zin.infolist():
            data = zin.read(info.filename)
            if info.filename == "word/media/image1.emf":
                data = patch_emf_frame(data, args.frame)
            zout.writestr(info, data)

    print(f"patched {output_docx}")


if __name__ == "__main__":
    main()
