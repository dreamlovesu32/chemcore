# Chemcore Agent Guide

This guide gives automation agents a source-code-independent map of Chemcore.
The CLI covers machine workflows. The desktop GUI covers interactive editing and
visual inspection.

## First Contact

Run these commands first:

```powershell
chemcore-cli guide --pretty
chemcore-cli guide --kind detailed --pretty
chemcore-cli doctor --pretty
chemcore-cli about --pretty
chemcore-cli capabilities --pretty
```

Installed builds add the CLI directory to PATH. Open a new terminal after
installing, then call `chemcore-cli` directly.

The CLI prints compact single-line JSON by default. `--pretty` formats JSON with
line breaks and indentation. Fields, values, output files, exit code, schema,
ordering, and command behavior stay the same.

For complete output, pass `--out <path>`. Large payloads and guide content are
available from the file written by `--out`.

This quick guide is installed as `chemcore-agent-guide.md`. The detailed English
CLI guide is installed as `chemcore-cli-guide.md`. To include guide Markdown in
JSON, use:

```powershell
chemcore-cli guide --kind agent --include-content --out chemcore-agent-guide.json --pretty
chemcore-cli guide --kind detailed --include-content --out chemcore-cli-guide.json --pretty
```

## Invocation Modes

There are two CLI invocation modes.

Use a PowerShell one-shot command when one operation can start a process, read
files, write files, print one JSON result, and exit.

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --pretty
```

Use a JSONL session for repeated work on one document. Start one long-lived
process, write one JSON request per stdin line, and read one JSON response per
stdout line. This keeps the document in memory and reuses the same loaded
document.

```powershell
chemcore-cli session input.cdxml
```

```jsonl
{"id":1,"op":"targets"}
{"id":2,"op":"capture","target":"molecule:0","out":"molecule.png","width":1800}
{"id":3,"op":"exit"}
```

The CDXML/CDX import cache belongs to one-shot mode. It stores normalized import
results on disk so repeated one-shot commands can reuse import work. JSONL
session is the mode for long iterative work on the same large file.

## Core Rule

Use a layered workflow:

1. Discover targets with `targets`.
2. Inspect the neighborhood with `context`.
3. Expand one id with `detail`.
4. Render an exact crop with `capture`.
5. Copy to Office with `copy` for editable Office clipboard payloads.

This keeps console output small and uses selectors instead of coordinate
guessing.

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

`bounds:` is for capture-style crops. `detail` accepts one `object:<id>`,
`molecule:<index>`, `node:<id>`, or `bond:<id>` selector.
`capture` and `context` accept multiple targets through repeated `--target`,
`--targets <selector;selector>`, `selection:<selector;selector>`, or a JSONL
session `target`/`targets` array. The crop box is the minimum bounds union,
matching the GUI selection box.
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
For multi-target context, `selectionBox.contents` lists items inside the target
box. `selectionBoxRelation` is `inside` or `partial`; `isTarget=true` marks the
explicitly selected targets.

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

`--summary-only` returns ids, bounds, and relationship metadata. `--include-resource`
expands the referenced resource as raw JSON when inspecting an object.

Aliases for `detail`: `details`, `describe`, `show`.

## Precise Screenshots

Use `capture` for deterministic exact crops. PNG is the raster format for visual
analysis.

```powershell
chemcore-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
```

Multiple targets use the same bounds logic as the GUI selection box:

```powershell
chemcore-cli capture input.cdxml --target object:obj_a --target object:obj_b --out selection.png --width 1800 --pretty
```

If `--out` is omitted, `capture` writes a PNG to the OS temp `chemcore-cli`
directory and returns the exact path with `output.defaulted=true` plus a
`default_output_path` warning. Capture manifests include `output.verified=true`
and `output.bytes` after the image file is verified on disk.

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

Execution reports include per-command success, document hash/revision changes,
created/updated/deleted targets, diagnostics, and invocation input/output paths.
Default execution reports contain change summaries. `--inspect-after
summary,objects,molecules` adds a structural snapshot after each command.
`--continue-on-error` keeps later commands running after a failure.

## Copy To Office

Use `copy` when the goal is to place an editable payload on the Windows
clipboard. Pasting is handled by Office.

```powershell
chemcore-cli copy input.cdxml --target molecule:0 --pretty
chemcore-cli copy input.cdxml --target object:obj_shape_001 --payload payload.json --no-copy --pretty
```

`--payload` is useful for debugging. `--no-copy` writes the payload JSON file.

## Output Policy

Deterministic output behavior:

- `new` and `run` are stateless command invocations. The CLI reports what each
  step changed; the caller can maintain history with git, temp files, or its own
  log.
- File-writing commands verify the written file before reporting success.
- `capture` with `--out` writes the requested image path. When `capture` omits
  `--out`, it writes a temp PNG path and reports
  `warnings[].kind=default_output_path`.
- `targets`, `context`, `detail`, and `inspect` accept `--out` for file-backed
  JSON output.
- `context` is the neighborhood discovery step before `detail`.
- `detail --summary-only` returns ids, bounds, and relationship metadata.
- `guide --include-content --out guide.json` writes guide Markdown content into
  JSON.
- stdout carries JSON status manifests. Images and payloads are written as
  files.

## Troubleshooting

Unknown command:

```powershell
chemcore-cli captur input.cdxml --target molecule:0 --out crop.png
```

The CLI returns JSON suggestions with nearby command names, purpose, usage, and
examples.

Missing argument:

`error.fix` is the primary repair object. Missing-argument errors include
`fix.action=provide_required_argument`, `fix.missing`, `fix.expected`, usage,
and an example command.

Ambiguous capture output:

```powershell
chemcore-cli capture input.cdxml --target molecule:0 --out crop
```

Use `.png`, `.svg`, or pass `--format png|svg`.

Target lookup failure:

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
```

Then copy a selector exactly from `targets.json`.

Large output:

Use `--out` and read the file for full document JSON, full guide content, or
large detail payloads.
