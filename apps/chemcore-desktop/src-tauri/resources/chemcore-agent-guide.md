# Chemcore Agent Guide

This guide is for automation agents using Chemcore without source-code context.
Prefer the CLI for machine workflows. Use the desktop GUI for interactive editing
and visual inspection.

## First Contact

Run these commands first:

```powershell
chemcore-cli guide --pretty
chemcore-cli doctor --pretty
chemcore-cli about --pretty
chemcore-cli capabilities --pretty
```

The CLI prints JSON by default. Use `--out <path>` for large payloads. Use
`--pretty` when a human or language model will read the output.

To include this full guide in JSON, use:

```powershell
chemcore-cli guide --include-content --out chemcore-agent-guide.json --pretty
```

## Core Rule

Use a layered workflow:

1. Discover targets with `targets`.
2. Inspect the neighborhood with `context`.
3. Expand one id with `detail`.
4. Render an exact crop with `capture`.
5. Copy to Office with `copy` only when the clipboard is the goal.

This keeps console output small and avoids guessing coordinates.

## Selectors

Most target-aware commands accept:

```text
all
object:<scene-object-id>
molecule:<zero-based-molecule-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
```

`bounds:` is for capture-style crops. `detail` does not accept `all` or `bounds`.
Use `inspect` for whole-document summaries.

## Discover Targets

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
```

The output groups stable selectors under `objects`, `molecules`, `nodes`, and
`bonds`. Use these selectors in `context`, `detail`, `capture`, and `copy`.

## Nearby Context

Use `context` to ask what is around a target. It returns ids, bounds, directions,
distances, and relationship metadata. It can also screenshot the same query box.

```powershell
chemcore-cli context input.cdxml --target object:obj_shape_001 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
```

Directional expansion is supported:

```powershell
chemcore-cli context input.cdxml --target molecule:1 --expand-left 40 --expand-right 120 --expand-rel-y 0.25 --out context.json --capture-out context.png --scale 6 --pretty
```

Use `--limit <n>` to cap each returned list.

## Object Details

After `targets` or `context` returns an id, use `detail` to expand one selector.

```powershell
chemcore-cli detail input.cdxml --target object:obj_shape_001 --out detail.json --pretty
chemcore-cli detail input.cdxml --target molecule:0 --out molecule-detail.json --pretty
chemcore-cli detail input.cdxml --target node:n_1 --out node-detail.json --pretty
chemcore-cli detail input.cdxml --target bond:b_1 --out bond-detail.json --pretty
```

Default behavior:

- `object:<id>` returns summary plus `raw.object`.
- `molecule:<index>` returns summary plus `raw.object` and `raw.fragment`.
- `node:<id>` returns summary plus `raw.node`.
- `bond:<id>` returns summary plus `raw.bond`.

Use `--summary-only` when you only need ids, bounds, and relationship metadata.
Use `--include-resource` when inspecting an object and you need the referenced
resource expanded as raw JSON.

Aliases for `detail`: `details`, `describe`, `show`.

## Precise Screenshots

Use `capture` for deterministic exact crops. PNG is recommended for visual
analysis.

```powershell
chemcore-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
```

Use fixed pixel dimensions when the model needs a predictable image budget:

```powershell
chemcore-cli capture input.cdxml --target object:obj_shape_001 --out object.png --width 1800 --expand 12 --pretty
```

Expansion options:

```text
--expand <pt>
--expand-x <pt>
--expand-y <pt>
--expand-left <pt>
--expand-right <pt>
--expand-top <pt>
--expand-bottom <pt>
--expand-rel <fraction>
--expand-rel-x <fraction>
--expand-rel-y <fraction>
--expand-rel-left <fraction>
--expand-rel-right <fraction>
--expand-rel-top <fraction>
--expand-rel-bottom <fraction>
```

PNG defaults to `--scale 4`. Use higher scale for close inspection.

## Editing Documents

Create a new document from a JSON command script:

```powershell
$script = '[{"type":"add-bond","begin":{"x":100,"y":120},"end":{"x":145,"y":120},"order":1,"variant":"single"}]'
$script | chemcore-cli new - --out example.ccjs --results example-results.json --pretty
```

Run commands against an existing document:

```powershell
chemcore-cli run input.cdxml commands.json --out edited.cdxml --results run-results.json --pretty
```

Execution reports include per-command success, created/updated/deleted targets,
diagnostics, and final inspection snapshots. Use `--continue-on-error` for batch
experiments where one failure should not stop later commands.

## Copy To Office

Use `copy` when the goal is to place an editable payload on the Windows
clipboard. Pasting is handled by Office.

```powershell
chemcore-cli copy input.cdxml --target molecule:0 --pretty
chemcore-cli copy input.cdxml --target object:obj_shape_001 --payload payload.json --no-copy --pretty
```

`--payload` is useful for debugging. `--no-copy` writes the payload without
touching the clipboard.

## Output Discipline

Recommended policy for agents:

- Always use `--out` for `targets`, `context`, `detail`, and `inspect` when the
  document may be large.
- Use `context` before `detail` when exploring unknown documents.
- Use `detail --summary-only` unless raw object JSON is needed.
- Use `guide --include-content --out guide.json` instead of printing the whole
  guide to the console.
- Treat stdout as a JSON status channel, not an image or payload channel.

## Troubleshooting

Unknown command:

```powershell
chemcore-cli captur input.cdxml --target molecule:0 --out crop.png
```

The CLI returns JSON suggestions with nearby command names, purpose, usage, and
examples.

Ambiguous capture output:

```powershell
chemcore-cli capture input.cdxml --target molecule:0 --out crop
```

Use `.png`, `.svg`, or pass `--format png|svg`.

Target not found:

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
```

Then copy a selector exactly from `targets.json`.

Large output:

Use `--out` and read the file. Do not rely on a console buffer for full document
JSON, full guide content, or large detail payloads.
