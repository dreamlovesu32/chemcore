# Formats And Conversion

Use `validate`, `canonicalize`, `migrate`, and `conformance` for governed
document-format work. Use `convert` for editable document conversion and
rendered exports. Use `export` as the export-oriented alias when the intent is
image/vector output. Use `insert-smiles` and `chemistry` for molecular input
and identifiers rather than treating identifiers as editable documents.

## Commands

```powershell
chemsema-cli convert input.cdxml output.ccjs
chemsema-cli convert input.ccjs output.cdxml
chemsema-cli convert input.cdxml output.svg
chemsema-cli convert input.cdxml output.png --scale 6
chemsema-cli export input.cdxml output.png --width 1800
chemsema-cli convert input.ccjs molecule-1.cdxml --target molecule:1
chemsema-cli export input.ccjs selected.ccjs --targets "object:obj_a;object:obj_b"
```

Govern and validate CCJS/CCJZ documents without overwriting the source:

```powershell
chemsema-cli validate document.ccjz --level roundtrip --target-format ccjs,ccjz,cdxml,cdx --pretty
chemsema-cli canonicalize input.ccjs --out canonical.ccjz --pretty
chemsema-cli migrate legacy.ccjs --out migrated.ccjz --pretty
chemsema-cli conformance --pretty
```

Create a structure from SMILES and request a molecular representation:

```powershell
chemsema-cli insert-smiles "CC(=O)O" --out acetic-acid.ccjs --x 120 --y 100 --pretty
chemsema-cli chemistry acetic-acid.ccjs --format chemical-graph-v2 --pretty
chemsema-cli chemistry acetic-acid.ccjs --format smiles --pretty
chemsema-cli chemistry acetic-acid.ccjs --format inchi --pretty
chemsema-cli chemistry acetic-acid.ccjs --format inchi-key --pretty
```

Use `--format <format>` when the output extension is ambiguous:

```powershell
chemsema-cli convert input.cdxml output --format svg
chemsema-cli export input.cdxml output --format png --width 1800
```

## Runtime Formats

Read the current format contract from:

```powershell
chemsema-cli capabilities --out capabilities.json --pretty
```

As of protocol v1, editable inputs include `ccjs`, `ccjz`, `cdxml`, `cdx`, and
`sdf`. Document outputs include `json`, `ccjs`, `ccjz`, `cdxml`, `cdx`, `sdf`,
`svg`, `png`, and Windows-only `emf`. Capture output includes `svg` and `png`.
Chemical analysis outputs include `chemical-graph-v2`, `smiles`, `inchi`, and
`inchi-key`. These analysis outputs describe a complete molecule; they are not
all editable drawing formats.

## Guardrails

- Use `capture` when the target is a visual bounds crop.
- Use `convert` or `export` when the target is the whole input document or an
  editable target subset.
- For editable subset export, use `--target <selector>` for one object,
  molecule, node, or bond. Use repeated `--target` or `--targets
  "object:a;object:b"` for multi-object/multi-molecule selection. Discover
  selectors with `targets` first.
- For PNG, specify `--scale`, `--width`, or `--height` when deterministic pixel
  dimensions matter.
- For structural comparisons, prefer `ccjs`/JSON over rendered SVG/PNG.
- Use `chemistry --format chemical-graph-v2|smiles|inchi|inchi-key` only after
  selecting a complete molecule; the command rejects ambiguous or lossy
  molecular targets instead of silently inventing an identifier.
- `canonicalize` and `migrate` require a distinct `--out` path and refuse
  in-place overwrite. Use `validate --level roundtrip` when conversion fidelity
  must be proven rather than assumed.
