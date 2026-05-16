from __future__ import annotations

import argparse
import os
import subprocess
import tempfile
from pathlib import Path
from xml.etree import ElementTree


EDGE_CANDIDATES = [
    Path(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"),
    Path(r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"),
]


def find_edge() -> Path:
    for candidate in EDGE_CANDIDATES:
        if candidate.exists():
            return candidate
    raise FileNotFoundError("Microsoft Edge was not found in the standard install locations.")


def parse_svg_size(svg_path: Path) -> tuple[int, int]:
    root = ElementTree.parse(svg_path).getroot()
    view_box = root.attrib.get("viewBox")
    if view_box:
        parts = [float(part) for part in view_box.replace(",", " ").split()]
        if len(parts) == 4 and parts[2] > 0 and parts[3] > 0:
            return int(round(parts[2])), int(round(parts[3]))

    width = root.attrib.get("width")
    height = root.attrib.get("height")
    if width and height:
        try:
            return int(round(float(width))), int(round(float(height)))
        except ValueError:
            pass
    raise ValueError(f"Could not determine SVG viewport size for {svg_path}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Render one SVG file to PNG using headless Edge.")
    parser.add_argument("input_svg")
    parser.add_argument("output_png")
    parser.add_argument("--scale", type=float, default=2.0, help="Output scale factor")
    args = parser.parse_args()

    input_path = Path(args.input_svg).resolve()
    output_path = Path(args.output_png).resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    width, height = parse_svg_size(input_path)
    scale = max(args.scale, 0.01)
    viewport_width = max(1, int(round(width * scale)))
    viewport_height = max(1, int(round(height * scale)))

    html = f"""<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    html, body {{
      margin: 0;
      padding: 0;
      width: {viewport_width}px;
      height: {viewport_height}px;
      overflow: hidden;
      background: white;
    }}
    img {{
      display: block;
      width: {viewport_width}px;
      height: {viewport_height}px;
    }}
  </style>
</head>
<body>
  <img src="{input_path.as_uri()}">
</body>
</html>"""

    with tempfile.TemporaryDirectory(prefix="chemcore-svg-render-") as temp_dir:
        html_path = Path(temp_dir) / "render.html"
        html_path.write_text(html, encoding="utf8")
        edge = find_edge()
        env = os.environ.copy()
        env.setdefault("CHROME_HEADLESS", "1")
        result = subprocess.run(
            [
                str(edge),
                "--headless",
                "--disable-gpu",
                f"--window-size={viewport_width},{viewport_height}",
                f"--screenshot={output_path}",
                html_path.as_uri(),
            ],
            env=env,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"Edge headless SVG render failed ({result.returncode}):\n"
                f"{result.stdout}\n{result.stderr}"
            )


if __name__ == "__main__":
    main()
