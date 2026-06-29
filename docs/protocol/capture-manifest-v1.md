# ChemCore Capture Manifest v1

Protocol id: `chemcore-cli-capture-manifest.v1`.

`chemcore-cli capture` and session `capture` return a JSON manifest describing
the visual output file.

## Stable Fields

- `ok`
- `input`
- `target`
- `warnings`
- `output`
- `bounds`
- `viewBox`
- `expansion`
- `render`

`output` includes:

- `path`
- `format`
- `defaulted`
- `verified`
- `bytes`
- `pixelSize`

`render` includes:

- `mode`
- `primitiveCount`
- `targets`

## Output Rules

PNG/SVG image data is written to `--out` or to the default temp path. Stdout is
reserved for the JSON manifest. If `--out` is omitted, capture writes a PNG into
the OS temp `chemcore-cli` directory and emits a `default_output_path` warning.

PNG resolution defaults to `--scale 4`. Use `--scale`, `--width`, or `--height`
for agent inspection crops that need higher resolution or bounded output size.
