from __future__ import annotations

import argparse
import zipfile
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Replace word/media/image1.emf inside a docx package."
    )
    parser.add_argument("input_docx")
    parser.add_argument("replacement_emf")
    parser.add_argument("output_docx")
    args = parser.parse_args()

    input_docx = Path(args.input_docx)
    replacement_emf = Path(args.replacement_emf)
    output_docx = Path(args.output_docx)
    output_docx.parent.mkdir(parents=True, exist_ok=True)

    replacement_bytes = replacement_emf.read_bytes()

    with zipfile.ZipFile(input_docx, "r") as zin, zipfile.ZipFile(
        output_docx, "w", compression=zipfile.ZIP_STORED
    ) as zout:
        for info in zin.infolist():
            data = zin.read(info.filename)
            if info.filename == "word/media/image1.emf":
                data = replacement_bytes
            zout.writestr(info, data)

    print(f"patched {output_docx}")


if __name__ == "__main__":
    main()
