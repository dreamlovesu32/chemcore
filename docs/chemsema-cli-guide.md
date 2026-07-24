# ChemSema CLI Command Guide

This guide describes direct `chemsema-cli` usage: opening files, creating
objects, editing objects, inspecting results, and recovering from command
errors.

## 1. Start The CLI

Run from the repository root:

```powershell
npm run cli -- <command> [args...]
```

Equivalent Cargo command:

```powershell
cargo run -p chemsema-cli -- <command> [args...]
```

After building, the executable can be called directly:

```powershell
target\debug\chemsema-cli.exe <command> [args...]
```

Show help:

```powershell
npm run cli -- --help
```

If Windows PowerShell blocks `npm.ps1` by execution policy, use `npm.cmd`:

```powershell
npm.cmd run cli -- --help
```

Installed desktop builds also install `chemsema-cli.exe` next to the GUI and
ship this English guide as `chemsema-cli-guide.md`. The installer adds the CLI
directory to PATH. Open a new terminal after installing, then start with:

```powershell
chemsema-cli guide --pretty
chemsema-cli guide --kind detailed --pretty
chemsema-cli version --pretty
chemsema-cli doctor --pretty
chemsema-cli capabilities --pretty
```

`--pretty` formats JSON with line breaks and indentation. Fields, values,
output files, exit code, schema, ordering, and command behavior stay the same.
Default JSON is compact single-line JSON.

## Invocation Modes

ChemSema CLI has two invocation modes.

Use a PowerShell one-shot command when each operation can start a process, read
its input files, write its output files, print one JSON result, and exit. This is
the simplest mode for independent inspection, conversion, export, copy, precise
capture, or a single `new`/`run` edit batch. One-shot commands are stateless:
edits are written through explicit output paths such as `--out`, `--results`,
or `--document-json`.

```powershell
chemsema-cli targets input.cdxml --out targets.json --pretty
chemsema-cli capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --pretty
chemsema-cli run input.cdxml commands.json --out edited.cdxml --results results.json --pretty
```

Use a JSONL session when many operations target the same document. Start one
long-lived process with `chemsema-cli session [input]`, then write one JSON
request per stdin line and read one JSON response per stdout line. A session
keeps the document in memory, so repeated `targets`, `detail`, `context`,
`capture`, `execute`, and `save` operations reuse the same loaded document.

```powershell
chemsema-cli session input.cdxml
```

```jsonl
{"id":1,"op":"targets"}
{"id":2,"op":"capture","target":"molecule:0","out":"molecule.png","width":1800}
{"id":3,"op":"save","out":"edited.cdxml"}
{"id":4,"op":"exit"}
```

The automatic CDXML/CDX import cache belongs to one-shot mode. It stores the
normalized imported document on disk so repeated one-shot commands can reuse
import work. JSONL session is the mode for long iterative work on one large
file.

## 2. File Commands

Opening a file means passing the file path to `inspect`, `run`, `convert`, or `export`.

```text
chemsema-cli --version
chemsema-cli version [--pretty] [--out <path>]
chemsema-cli guide [--kind agent|detailed|all] [--include-content] [--pretty] [--out <path>]
chemsema-cli about [--pretty] [--out <path>]
chemsema-cli capabilities [--pretty] [--out <path>]
chemsema-cli doctor [--pretty] [--out <path>]
chemsema-cli examples [basic|capture-copy|all] [--pretty] [--out <path>]
chemsema-cli schema [protocol|commands|targets|capture|context|bundle|detail|diff|guide|copy|json-output|command-script|command-transaction|all] [--pretty] [--out <path>]
chemsema-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]
chemsema-cli targets <input> [--out <path>] [--pretty]
chemsema-cli context <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] [--radius <pt>] [--out <context.json>] [--capture-out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--pretty]
chemsema-cli bundle <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] --out-dir <directory> [--context-radius <pt>] [--capture-format png|svg] [--capture-width <px>] [--capture-height <px>] [--capture-scale <n>] [--subset-format ccjs|ccjz|cdxml|cdx|sdf] [--pretty]
chemsema-cli detail <input> --target <object:id|molecule:index|node:id|bond:id> [--summary-only] [--include-resource] [--out <detail.json>] [--pretty]
chemsema-cli capture <input> --target <selector> [--target <selector> ...] [--targets <selector;selector>] [--selection-only] [--crop-bounds <minX,minY,maxX,maxY>] [--out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--expand <pt>] [--expand-rel <fraction>] [--pretty]
chemsema-cli copy <input> [--target <selector>] [--payload <payload.json>] [--no-copy] [--pretty]
chemsema-cli session [input]
chemsema-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemsema-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemsema-cli convert <input> <output> [--format <format>] [--scale <n>|--width <px>|--height <px>]
chemsema-cli export <input> <output> [--format <format>] [--scale <n>|--width <px>|--height <px>]
chemsema-cli diff <before> <after> --out <diff.json> [--pretty]
chemsema-cli label-query --text <source-label> [--connection-angle <deg> ...] [--connection-count <n>] [--display-mode connection-auto|right-auto|left-auto|preserve-right|preserve-left|preserve-center] [--no-default-chemical] [--pretty]
chemsema-cli label-query --visible-text <visible-label> [--connection-angle <deg> ...] [--connection-count <n>] [--display-mode connection-auto|right-auto|left-auto|preserve-right|preserve-left|preserve-center] [--pretty]
```

Common calls:

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty
npm run cli -- targets input.cdxml --out targets.json --pretty
npm run cli -- capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
npm run cli -- capture input.cdxml --target object:obj_text_1 --selection-only --crop-bounds 0,0,800,600 --out text-layer.png --width 4800 --height 3600 --pretty
npm run cli -- new commands.json --out output.cdxml --results results.json --pretty
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
npm run cli -- convert input.cdxml output.svg
npm run cli -- convert input.cdxml output.png --scale 6
npm run cli -- convert input.cdxml output.emf
npm run cli -- convert input.cdxml output.ccjs
npm run cli -- bundle input.cdxml --target molecule:0 --out-dir molecule-0-bundle --context-radius 40 --capture-format png --subset-format ccjs --pretty
npm run cli -- diff before.ccjs after.ccjs --out diff.json --pretty
```

Label query calls:

```powershell
npm run cli -- label-query --text CF3 --connection-angle 0 --pretty
npm run cli -- label-query --visible-text H2N --connection-angle 0 --pretty
npm run cli -- label-query --visible-text CF3 --connection-angle 0 --pretty
npm run cli -- label-query --visible-text "(II)Cu" --display-mode right-auto --connection-angle 93 --connection-angle 17 --connection-angle -30 --pretty
```

`label-query --text` is the forward contract: ChemSema receives source text,
connection geometry, and `defaultChemical`, then reports whether the label is
accepted, its displayed text, source runs, generated-hydrogen anchor semantics,
and functional-group recognition metadata. `label-query --visible-text` is the
reverse contract for imported or externally measured drawings: ChemSema generates candidate source labels
from visible text using the same label-group reversal rules, runs the normal
label engine for each candidate, and recommends the candidate whose display
matches the visible text. If no chemical source candidate both validates and
renders back to the visible text, the reverse report recommends the
`defaultChemical:false` plain-text candidate so the caller can preserve the
source drawing instead of forcing chemical rewriting.

Both forward and reverse reports include `commandFields`, a copy-ready subset
for `set-node-label-runs`. It contains the source/edit `text`, `sourceText`,
source `runs`, `defaultChemical`, and, when needed, `displayMode`. OCR callers
should prefer this object over reconstructing command fields from display and
recognition fields by hand.

`--display-mode` separates endpoint-command layout from imported ChemDraw
alignment semantics. The default `connection-auto` follows the normal endpoint
label flow for the supplied connection angles. `right-auto` models CDXML/CDX
`LabelAlignment=Right` without `LabelDisplay`: ChemSema may reverse chemical
groups while keeping the original first group as the right/end anchor. The
`preserve-right`, `preserve-left`, and `preserve-center` modes model forced
visible display. If an auto-reversed source and a forced visible source have the
same visible text and the same anchor policy, pixels cannot recover the hidden
source choice; callers should keep that ambiguity explicit rather than invent a
distinction.

File output policy:

- `capture` may omit `--out`; it then writes a PNG into the OS temp `chemsema-cli` directory, reports the exact path in `output.path`, and emits a `default_output_path` warning.
- `copy` may omit `--payload`; it then writes the clipboard payload JSON into the OS temp `chemsema-cli` directory, reports the exact path in `payload.path`, and emits a `default_payload_path` warning.
- `new`, `convert`, and `export` require explicit output paths because they create primary document files.
- Every file-writing command verifies after writing that the target exists, is a regular file, and has the expected or minimum byte size. A failed verification is a command failure.

Object-grounded bundle and diff:

- `bundle` writes a directory containing `manifest.json`, `target.json`, `context.json`, `editable-subset.<format>`, `capture.png` or `capture.svg`, `identity-map.json`, and `provenance.json`.
- `manifest.json` separates `editableScope` from `visualScope`. Visual context can contain nearby non-target objects; only editable scope is authorized for modification.
- `context.json` keeps `selectionBoxRelation` and `isTarget`, so callers can tell selected objects from merely visible neighbors.
- `diff` compares two editable documents by object/resource/style/node/bond identity and reports created, updated, deleted, and field-level changes.

Transactional edits:

- `new`, `run`, and JSONL session `execute` accept `chemsema.command-transaction.v1` envelopes in addition to plain command objects and command arrays.
- Transactions check optional document hash/revision preconditions, run commands on a cloned engine state, validate the structured diff against `scope.editableTargets`, and only apply the clone when checks pass.
- `dryRun: true` reports execution, diff, allowed selectors, and unexpected changes without mutating the open document or writing document outputs.

Import cache policy:

- CDXML/CDX input uses an automatic normalized-document import cache to speed repeated CLI invocations. The cache key includes the source content, format, CLI version, and executable stamp; changed files or rebuilt binaries create new cache entries.
- Use `CHEMSEMA_CLI_DISABLE_CACHE=1` to disable import caching. Use `CHEMSEMA_CLI_CACHE_DIR=<path>` to place the cache in a specific directory. `chemsema-cli doctor --pretty` reports the effective cache settings.

Error output policy:

- Error JSON includes `error.kind`, `message`, `hint`, `fix`, `usage`, `examples`, and `suggestions`.
- Missing argument errors use `error.kind="missing_argument"` and include `error.fix.action="provide_required_argument"` plus machine-readable `missing` and `expected` fields.
- `error.fix` is the primary repair object. `usage` and `examples` provide command context.

Protocol contract:

- `chemsema-cli --version` prints a single text line for shell checks.
- `chemsema-cli version --pretty` returns product and protocol versions as JSON.
- `chemsema-cli schema protocol --pretty` returns the runtime protocol ids.
- Machine-facing contracts are documented in [docs/protocol](./protocol/README.md).

`new` starts from a blank ChemSema internal document. The command takes a command
script and an output path. The save format is inferred from `--out`:

```powershell
npm run cli -- new --out blank.ccjs --quiet
npm run cli -- new commands.json --out figure.cdxml
npm run cli -- new commands.json --out figure.svg
```

Use `--save-format` when the output path has an ambiguous extension or when writing to stdout:

```powershell
npm run cli -- new commands.json --out output --save-format cdxml
npm run cli -- run input.cdxml commands.json --out - --save-format svg --quiet
```

`convert` and `export` use `--format` to override the output format:

```powershell
npm run cli -- convert input.cdxml output --format svg
npm run cli -- convert input.cdxml output --format png --width 1800
npm run cli -- convert input.cdxml output --format emf
```

Supported formats:

| Format | Read | Write | Use |
| --- | --- | --- | --- |
| `json` | yes | yes | ChemSema internal JSON. `.json` is treated as internal JSON |
| `ccjs` | yes | yes | ChemSema internal JSON, uncompressed |
| `ccjz` | yes | yes | gzip-compressed ChemSema JSON |
| `cdxml` | yes | yes | ChemDraw XML |
| `cdx` | yes | yes | ChemDraw binary |
| `sdf` | yes | yes | MDL SD file |
| `svg` | - | yes | vector export |
| `png` | - | yes | raster export. Defaults to `--scale 10`; use `--scale`, `--width`, or `--height` |
| `emf` | - | Windows only | direct Enhanced Metafile vector export; uses the same renderer as Office/OLE previews |

## 3. Command Script Format

`commands.json` can be one JSON object or a JSON array.

Single command:

```json
{
  "type": "insert-template",
  "template": "benzene",
  "x": 300.0,
  "y": 260.0
}
```

Multiple commands:

```json
[
  {
    "type": "insert-template",
    "template": "benzene",
    "x": 300.0,
    "y": 260.0
  },
  {
    "type": "add-arrow",
    "begin": { "x": 370.0, "y": 260.0 },
    "end": { "x": 520.0, "y": 260.0 },
    "variant": "solid",
    "headSize": "small",
    "curve": "arc270",
    "headStyle": "full",
    "tailStyle": "none",
    "head": true,
    "tail": false,
    "bold": false,
    "noGo": "none"
  }
]
```

Common field shapes:

| Name | JSON shape | Meaning |
| --- | --- | --- |
| point | `{ "x": 100.0, "y": 120.0 }` | page coordinates |
| anchor | `{ "x": 100.0, "y": 120.0, "nodeId": "n1" }` | `nodeId` or `objectId` is optional |
| target set | `{ "nodes": [], "bonds": [], "objects": [], "labelNodes": [] }` | used for move, rotate, delete |
| text run | `{ "text": "H", "script": "normal" }` | `script` can be `normal`, `subscript`, `superscript`, `chemical` |

Coordinates use ChemSema document coordinates. `x` increases to the right, and `y` increases downward.

## 4. Execution Reports, Ids, And Internal JSON

Pass `--results` when using `new` or `run`. `results.json` is the primary machine-readable record for whether commands executed, whether they changed the document, which ids were created/updated/deleted, what failed, and which input/output files were involved. By default it is a lightweight audit report.

```powershell
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
```

### 4.1 Top-Level Report

`results.json` is an object:

```json
{
  "ok": true,
  "commandCount": 1,
  "executedCount": 1,
  "failedIndex": null,
  "commands": [],
  "document": {
    "hashAlgorithm": "sha256",
    "hashInput": "chemsema-document-json-v1",
    "beforeHash": "64 hex chars",
    "afterHash": "64 hex chars",
    "hashChanged": true,
    "beforeRevision": 0,
    "afterRevision": 1
  },
  "io": {
    "operation": "run",
    "input": { "path": "input.cdxml" },
    "script": "commands.json",
    "output": { "path": "output.cdxml", "format": "cdxml" }
  },
  "documentJson": {
    "ok": true,
    "path": "after.ccjs",
    "format": "json"
  },
  "save": {
    "ok": true,
    "path": "output.cdxml",
    "format": "cdxml"
  }
}
```

| Field | Meaning |
| --- | --- |
| `ok` | whether the whole script succeeded. Save failure also sets it to `false` |
| `commandCount` | number of commands in the script |
| `executedCount` | number of commands that reached the engine and returned an engine result |
| `failedIndex` | 0-based index of the failed command, or `null` |
| `commands` | per-command reports |
| `document` | document hash/revision before and after the script, useful for change detection while keeping reports small |
| `io` | operation name plus input/script/output paths for this invocation |
| `final` | optional inspect snapshot after the script stops, present when `--inspect-after` is used |
| `documentJson` | result of `--document-json` |
| `save` | result of `--out` |
| `error` | top-level failure reason |

When the CLI fails, the process exits non-zero and prints an error to stderr. If `--results` was provided, the CLI still tries to write the structured report.

### 4.2 Per-Command Report

`commands[i]` has this shape:

```json
{
  "index": 0,
  "ok": true,
  "executed": true,
  "changed": true,
  "commandType": "add-bond",
  "command": {},
  "revision": 1,
  "beforeRevision": 0,
  "document": {
    "hashAlgorithm": "sha256",
    "hashInput": "chemsema-document-json-v1",
    "beforeHash": "64 hex chars",
    "afterHash": "64 hex chars",
    "hashChanged": true,
    "beforeRevision": 0,
    "afterRevision": 1
  },
  "changeSummary": {
    "createdCount": 3,
    "updatedCount": 1,
    "deletedCount": 0,
    "createdSelectors": {
      "objects": [],
      "nodes": ["node:n_1", "node:n_2"],
      "bonds": ["bond:b_3"],
      "styles": []
    },
    "updatedSelectors": { "objects": ["object:obj_editor_molecule"], "nodes": [], "bonds": [], "styles": [] },
    "deletedSelectors": { "objects": [], "nodes": [], "bonds": [], "styles": [] },
    "touchedSelectors": ["node:n_1", "node:n_2", "bond:b_3", "object:obj_editor_molecule"]
  },
  "targets": {},
  "created": {
    "nodes": ["n_1", "n_2"],
    "bonds": ["b_3"]
  },
  "updated": {
    "objects": ["obj_editor_molecule"]
  },
  "deleted": {},
  "diagnostics": {},
  "engineResult": {}
}
```

| Field | Meaning |
| --- | --- |
| `ok` | whether this command succeeded |
| `executed` | whether it reached the engine and returned `engineResult` |
| `changed` | whether it changed the document. A valid unchanged result is `false` |
| `commandType` | original `type` value |
| `document` | document hash/revision before and after this command |
| `changeSummary` | selector-form summary of created/updated/deleted ids, intended for agent history |
| `created` | created node, bond, scene object, and style ids |
| `updated` | updated node, bond, scene object, and style ids |
| `deleted` | deleted node, bond, scene object, and style ids |
| `engineResult` | raw ChemSema engine result |
| `after` | optional inspect snapshot after this command, present when `--inspect-after` is used |

Decision table:

| Situation | Meaning |
| --- | --- |
| `ok=true, executed=true, changed=true` | command executed and changed the document |
| `ok=true, executed=true, changed=false` | command was valid and left the document unchanged |
| `ok=false, executed=false` | command execution was rejected or skipped. Read `error.message` |
| top-level `ok=false` and `save.skipped=true` | script failed and `--out` save was skipped |

### 4.3 Failed Command Report

Example:

```json
{
  "index": 1,
  "ok": false,
  "executed": false,
  "changed": false,
  "commandType": "add-bond",
  "command": {
    "type": "add-bond",
    "variant": "wrong-style"
  },
  "error": {
    "stage": "execute-command",
    "message": "unknown variant `wrong-style`, expected one of `single`, `double`, `triple`, `dashed`, `dashed-double`, `bold`, `bold-dashed`, `wavy`, `wedge`, `hashed-wedge`, `hollow-wedge`"
  }
}
```

Common `error.stage` values:

| stage | Meaning |
| --- | --- |
| `read-script` | command JSON read/parsing rejected the script shape |
| `execute-command` | invalid field, invalid enum value, missing field, or command requiring interaction context |
| `inspect-after` | optional inspect after one command failed |
| `inspect-final` | optional final inspect failed |
| `write-document-json` | `--document-json` write failed |
| `save-output` | `--out` save failed |

If a script fails, earlier successful commands remain in the in-memory document and are visible in `document`, command entries, and `--document-json` if requested. The target `--out` save reports `save.skipped=true`.

### 4.4 Optional After Snapshots

Default command reports include change summaries. `--inspect-after` adds per-command `after` snapshots and a top-level `final` snapshot. The CLI reports what changed; the caller or agent can maintain history with git, temporary files, or its own log.

Pass `--inspect-after` when a command-by-command structural snapshot is useful:

```text
summary,objects,molecules
```

With `--inspect-after summary,objects,molecules`, molecule edits can be read from:

```text
commands[i].after.molecules
```

It contains the current molecule fragments, nodes, bonds, elements, coordinates, and labels:

```json
{
  "molecules": [
    {
      "objectId": "obj_editor_molecule",
      "resourceRef": "mol_editor",
      "nodeCount": 2,
      "bondCount": 1,
      "nodes": [
        {
          "id": "n_1",
          "element": "C",
          "atomicNumber": 6,
          "position": [100.0, 120.0],
          "label": null
        }
      ],
      "bonds": [
        {
          "id": "b_3",
          "begin": "n_1",
          "end": "n_2",
          "order": 1,
          "stereo": null
        }
      ]
    }
  ]
}
```

Control snapshot contents explicitly:

```powershell
npm run cli -- run input.cdxml commands.json --results results.json --inspect-after summary,molecules
npm run cli -- run input.cdxml commands.json --results results.json --inspect-after summary,objects,molecules,styles
npm run cli -- run input.cdxml commands.json --results results.json --inspect-after none
```

`--no-inspect-after` is equivalent to `--inspect-after none`.

### 4.5 Getting Object Ids

Editing existing objects requires ids. Get ids from `inspect`, `targets`, `results.commands[i].created`, or `results.commands[i].changeSummary`. When `--inspect-after` is requested, `results.commands[i].after` also contains ids from the post-command snapshot.

Write `--results` when creating objects:

```powershell
npm run cli -- new commands.json --out output.cdxml --results results.json --pretty
```

For commands that create entities, new ids are recorded at:

```text
commands[i].created.nodes
commands[i].created.bonds
commands[i].created.objects
```

Read ids from an existing file:

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty
```

`inspect.json` sections:

| section | Contents |
| --- | --- |
| `summary` | counts, page, revision, render bounds |
| `objects` | scene object ids, types, bbox, styleRef |
| `molecules` | molecule fragments, node ids, bond ids, elements, coordinates, labels |
| `resources` | fragment/text/json resource summaries |
| `styles` | style summaries |

### 4.6 Reading Internal JSON

There are three common ways to read full ChemSema internal JSON.

Convert an existing file:

```powershell
npm run cli -- convert input.cdxml input.ccjs
```

Write internal JSON while editing:

```powershell
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
```

Save the edit result as internal JSON:

```powershell
npm run cli -- run input.cdxml commands.json --out after.ccjs --results results.json --pretty
```

`--document-json` is useful for debugging because it can be used together with `--out output.cdxml`. If the script fails partway through, it writes the in-memory ChemSema JSON at the failure point.

### 4.7 Agent Target, Context, Detail, Capture, And Copy Workflow

Use this selector-based workflow when an agent needs exact ids, exact crops, or
nearby-object context:

```powershell
chemsema-cli targets input.cdxml --out targets.json --pretty
chemsema-cli context input.cdxml --target object:obj_shape_001 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
chemsema-cli detail input.cdxml --target object:obj_shape_001 --out detail.json --pretty
chemsema-cli capture input.cdxml --target object:obj_shape_001 --out object.png --scale 6 --expand-rel 0.15 --pretty
chemsema-cli copy input.cdxml --target object:obj_shape_001 --pretty
```

Selectors accepted by target-aware commands:

```text
all
object:<scene-object-id>
molecule:<zero-based-molecule-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
selection:<selector;selector>
```

`bounds:` is accepted by capture-style crops. `detail` accepts one
`object:<id>`, `molecule:<index>`, `node:<id>`, or `bond:<id>` selector.
`capture` and `context` accept multiple targets through repeated `--target`,
`--targets <selector;selector>`, `selection:<selector;selector>`, or a JSONL
session `target`/`targets` array. The target box is the minimum bounds union,
matching the GUI selection box.

`targets` returns stable selectors and bounds grouped under `objects`,
`molecules`, `nodes`, and `bonds`. It is the discovery step before `context`,
`detail`, `capture`, or `copy`.

`context` returns nearby object summaries, molecule summaries, node summaries,
bond summaries, bounds, direction, distance, overlap flags, selection-box
relation, group ancestry, child ids, and link metadata. `selectionBox.contents`
lists items inside the target box; each item has `selectionBoxRelation="inside"`
or `"partial"` and `isTarget=true` only for explicitly selected targets. Use
`detail` on a returned selector when raw JSON is needed.

`detail` returns one selected entity. By default, it includes raw JSON for that
entity. Add `--summary-only` when ids, bounds, and relationship metadata are
sufficient. Add `--include-resource` for an object when the referenced resource
is part of the requested response.

`capture` writes the rendered crop to `--out` and writes a JSON manifest to
stdout. For multiple targets, the crop is the target box minimum union and the
image shows everything visible in that box plus requested expansion. If `--out`
is omitted, it writes a PNG to the OS temp `chemsema-cli`
directory and reports `output.defaulted=true` plus the exact `output.path`.
It also emits a `warnings[]` entry with `kind="default_output_path"`.
SVG output is vector. PNG output defaults to `--scale 10`; use `--scale`,
`--width`, or `--height` when the caller needs a sharper or bounded
raster image. The manifest includes `output.verified=true` and `output.bytes`
after the file is verified on disk. Use absolute expansion (`--expand`, `--expand-left`,
`--expand-right`, `--expand-top`, `--expand-bottom`) and proportional expansion
(`--expand-rel`, `--expand-rel-left`, `--expand-rel-right`, `--expand-rel-top`,
`--expand-rel-bottom`) to include surrounding context.

Capture manifests also include `render.mode`, `render.primitiveCount`, and
`render.targets`. These fields report how the crop was rendered and how many
nearby node, bond, and object targets were included in the crop. `context`
includes the same render fields under `capture.render` when `--capture-out` is
used.

`copy` places an editable ChemSema Office/OLE payload on the Windows clipboard.
If `--payload` is omitted, the payload JSON is written to the OS temp
`chemsema-cli` directory and a `default_payload_path` warning is emitted.
`--payload payload.json --no-copy` writes the clipboard payload JSON file.

`session` starts a long-lived JSON Lines process for agents. The first stdout
line is a compact `ready` event. Then send one compact JSON request per line and
read one compact JSON response per line. A session keeps one document open in
memory, so repeated `targets`, `detail`, `context`, `capture`, `execute`, and
`save` operations reuse the same loaded document. `execute` responses report
before/after revision and per-command results, which gives the caller enough
data for git, file-based, or log-based history.

```json
{"id":1,"op":"open","input":"input.cdxml"}
{"id":2,"op":"targets"}
{"id":3,"op":"capture","target":"molecule:0","out":"molecule.png","width":1800}
{"id":4,"op":"execute","command":{"type":"add-text","position":{"x":40,"y":40},"text":"note"}}
{"id":5,"op":"save","out":"edited.ccjs"}
{"id":6,"op":"exit"}
```

## 5. Molecule Objects

### 5.1 Create One Atom

```json
{
  "type": "add-element",
  "symbol": "O",
  "atomic_number": 8,
  "center": { "x": 120.0, "y": 120.0 }
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | string | element symbol, for example `C`, `N`, `O`, `Cl` |
| `atomic_number` | number | atomic number |
| `center` | anchor | placement coordinate |

### 5.2 Create A Bond And Auto-Create Carbon Endpoints

```json
{
  "type": "add-bond",
  "begin": { "x": 100.0, "y": 120.0 },
  "end": { "x": 140.0, "y": 120.0 },
  "order": 1,
  "variant": "single"
}
```

`variant` values:

```text
single
double
triple
dashed
dashed-double
bold
bold-dashed
wavy
wedge
hashed-wedge
hollow-wedge
```

### 5.3 Plan A Bond Without Editing

Use `plan-bond` when an external agent wants the same landing point that the
ChemSema engine would use for a new bond, without changing the document.

```json
{
  "type": "plan-bond",
  "begin": { "nodeId": "n_1", "x": 100.0, "y": 120.0 },
  "angle": 0.0,
  "bondLength": 14.4,
  "order": 1,
  "variant": "single"
}
```

Inputs:

| Field | Type | Meaning |
| --- | --- | --- |
| `begin` | anchor | start node or coordinate |
| `cursor` | point, optional | pointer position to resolve through the engine's bond-angle snapping |
| `angle` | number, optional | explicit angle in degrees; bypasses cursor snapping |
| `bondLength` | number, optional | world pt bond length; defaults to current object settings |
| `order` | number | bond order, defaults to `1` |
| `variant` | string | bond variant, defaults to `single` |

Output is an unchanged command result whose `output.command` is an executable
`add-bond` command. `output.keypadSlots` exposes numeric-keypad convenience
directions, including key `5` for the engine default angle. This command is for
drawing agents and GUI-like automation that want ChemSema's own placement
rules instead of duplicating GUI geometry.

### 5.4 Add A Bond Between Existing Atoms

Use node ids from `inspect` or `results`:

```json
{
  "type": "add-bond",
  "begin": { "nodeId": "node_a", "x": 100.0, "y": 120.0 },
  "end": { "nodeId": "node_b", "x": 140.0, "y": 120.0 },
  "order": 2,
  "variant": "double"
}
```

When `nodeId` is present, the node is the target. `x/y` are still required.

### 5.5 Plan Or Insert A Template

```json
{
  "type": "insert-template",
  "template": "benzene",
  "x": 300.0,
  "y": 260.0
}
```

`template` values:

```text
ring-3
ring-4
ring-5
ring-6
ring-7
ring-8
benzene
chair-6-right
chair-6-left
chain
```

`plan-template` returns the same vertex/edge plan without editing:

```json
{
  "type": "plan-template",
  "template": "benzene",
  "x": 300.0,
  "y": 260.0,
  "angle": 270.0,
  "bondLength": 14.4
}
```

Optional placement fields on both `plan-template` and `insert-template`:

| Field | Type | Meaning |
| --- | --- | --- |
| `anchor` | anchor | attach the first template vertex to an existing endpoint |
| `bondId` | string | fuse the template to an existing bond |
| `cursor` | point | use GUI drag placement from the anchor or center |
| `angle` | number | explicit template axis angle in degrees |
| `bondLength` | number | template side length in world pt |
| `side` | number | for fused rings, positive or negative side of the anchor bond |

`plan-template` output includes `vertices`, `edges`, and an `insertCommand` that
can be used as the corresponding edit command. Create chains with the `chain`
template or with multiple `add-bond` commands.

### 5.6 Edit Bond Style

```json
{
  "type": "apply-bond-style",
  "bondIds": ["bond_1"],
  "style": "double-center"
}
```

`style` values:

```text
single-plain
single-dashed
single-hashed
single-hashed-wedged
single-bold
single-bold-wedged
single-hollow-wedged
single-wavy
double-left
double-right
double-center
double-bold
double-dashed
double-double-dashed
triple-plain
```

Short aliases:

```text
single
dashed
hashed
hashed-wedge
bold
wedge
hollow-wedge
wavy
double
triple
```

### 5.7 Replace Atom Label

```json
{
  "type": "replace-node-label",
  "node_id": "node_1",
  "label": "OH"
}
```

### 5.8 Set Atom Label Runs

```json
{
  "type": "set-node-label-runs",
  "nodeId": "node_1",
  "runs": [
    { "text": "SO", "fontSize": 10.0, "script": "normal" },
    { "text": "3", "fontSize": 10.0, "script": "subscript" },
    { "text": "H", "fontSize": 10.0, "script": "normal" }
  ],
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000",
  "box": [72.0, 92.0, 96.0, 104.0],
  "anchorOffset": [28.0, 8.0],
  "textPosition": [71.2, 104.0],
  "glyphPolygons": [
    [[72.0, 92.0], [84.0, 92.0], [84.0, 104.0], [72.0, 104.0]],
    [[86.0, 92.0], [96.0, 92.0], [96.0, 104.0], [86.0, 104.0]]
  ],
  "preserveMeasuredBox": true,
  "preserveImplicitHydrogenLabel": false,
  "defaultChemical": true,
  "displayMode": "preserve-left"
}
```

`box`, `anchorOffset`, `textPosition`, `glyphPolygons`, and
`preserveMeasuredBox` are optional. Use them when a caller has measured
endpoint-label geometry from any source. The box is world-space
`[x1, y1, x2, y2]`; `anchorOffset` is the endpoint anchor relative to the box
origin; and `textPosition` is the renderer-facing text baseline position.
`glyphPolygons` are optional authoritative world-space glyph outlines; omit them
when the caller only has visible ink/component boxes, so ChemSema can rebuild
renderer glyph outlines from the font and text position for knockout and bond
retreat. Source-neutral measured geometry must not be encoded as
`meta.import.cdxml`; that namespace is reserved for data actually imported from
CDXML. When `preserveMeasuredBox` is true, ChemSema stores the source-neutral
measurement under `label.meta.measuredGeometry`, keeps the measured box and text
position, and still applies node-label recognition and editable label semantics.
When `preserveImplicitHydrogenLabel` is true on an endpoint label such as
`NH2`, ChemSema treats the element-hydrogen source text as user-authored and
keeps it even if the current valence-derived implicit hydrogen count would
change the visible hydrogen count.

`displayMode` is optional and uses readable values:
`connection-auto`, `left-auto`, `right-auto`, `preserve-left`,
`preserve-right`, or `preserve-center`. Use it only to express ChemSema label
layout semantics. For example, OCR may emit `displayMode:"preserve-left"` for a
visible `CN` attached at the C/left glyph when the source drawing must not be
auto-reversed to `NC`. Do not use measured boxes to force this layout; let the
kernel recompute glyph outlines, anchor, clipping, and bond retreat from the
source runs plus `displayMode`.
normally refresh the label to another source, such as `NH`.

### 5.9 Interpret Atom Label Chemically

```json
{
  "type": "select-targets",
  "targets": { "nodes": ["node_1"] }
}
```

```json
{
  "type": "set-interpret-chemically-for-selection",
  "enabled": false
}
```

Use this for ChemDraw-style `Interpret Chemically`. When disabled, ChemSema
preserves the label as plain text by setting `defaultChemical:false`, converting
label source runs to normal text, and clearing chemical recognition diagnostics.
It is not an implicit-hydrogen display switch.

### 5.10 Override Implicit Hydrogen Count

```json
{
  "type": "select-targets",
  "targets": { "nodes": ["node_1"] }
}
```

```json
{
  "type": "set-implicit-hydrogen-count-for-selection",
  "count": 0
}
```

Use `count:0` to hide generated hydrogens while keeping the atom chemically
interpreted. Use `count:null` to return selected nodes to the automatic
valence-derived implicit-hydrogen calculation.

### 5.11 Set Atom Charge

```json
{
  "type": "set-node-charge",
  "nodeId": "node_1",
  "charge": 1
}
```

Use this when the caller has explicit or inferred formal-charge semantics for an
atom, including imported or externally measured drawings where a visible label
such as `NH2` becomes positively charged because the recovered bond order gives
nitrogen valence 4.
ChemSema refreshes implicit hydrogens, label recognition, and attached label
geometry after the charge update.

### 5.12 Edit Atom Label Style

```json
{
  "type": "apply-text-style",
  "textObjectIds": [],
  "labelNodeIds": ["node_1"],
  "nodeIds": [],
  "command": "font-size",
  "value": "12"
}
```

`command` values:

```text
font-family
font-size
align
line-height
bold
italic
underline
outline
shadow
superscript
subscript
formula
```

`align` values are `left`, `center`, `right`, `justify`. Boolean commands accept values such as `true`, `false`, `on`, `off`, `1`, `0`.

## 6. Arrow Objects

### 6.1 Create Arrow

```json
{
  "type": "add-arrow",
  "begin": { "x": 370.0, "y": 260.0 },
  "end": { "x": 520.0, "y": 260.0 },
  "variant": "solid",
  "headSize": "small",
  "curve": "arc270",
  "headStyle": "full",
  "tailStyle": "none",
  "head": true,
  "tail": false,
  "bold": false,
  "noGo": "none"
}
```

| Field | Values |
| --- | --- |
| `variant` | `solid`, `curved`, `curved-mirror`, `hollow`, `open`, `equilibrium`, `unequal-equilibrium` |
| `headSize` | `large`, `medium`, `small` |
| `curve` | `arc270`, `arc180`, `arc120`, `arc90` |
| `headStyle` | `none`, `full`, `left`, `right` |
| `tailStyle` | `none`, `full`, `left`, `right` |
| `noGo` | `none`, `cross`, `hash` |

### 6.2 Set Arrow Geometry

```json
{
  "type": "set-arrow-geometry",
  "objectId": "arrow_1",
  "begin": { "x": 360.0, "y": 260.0 },
  "end": { "x": 540.0, "y": 260.0 },
  "curve": 0.0,
  "headStyle": "full",
  "tailStyle": "none"
}
```

`curve` is a numeric degree value. Use `0.0` for a straight arrow.

### 6.3 Edit Arrow Style

```json
{
  "type": "apply-arrow-style",
  "objectIds": ["arrow_1"],
  "variant": "equilibrium",
  "headSize": "small",
  "curve": "arc270",
  "headStyle": "full",
  "tailStyle": "full",
  "head": true,
  "tail": true,
  "bold": false,
  "noGo": "none"
}
```

### 6.4 Edit Arrow Line Style

```json
{
  "type": "apply-line-style",
  "objectIds": ["arrow_1"],
  "style": "dashed"
}
```

`style` values: `plain`, `dashed`, `bold`.

## 7. Text Objects

### 7.1 Create Plain Text

```json
{
  "type": "add-text",
  "position": { "x": 120.0, "y": 80.0 },
  "text": "Yield 85%",
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000",
  "align": "left",
  "lineHeight": 12.0,
  "box": [0.0, 0.0, 160.0, 14.0]
}
```

### 7.2 Create Styled Text Runs

```json
{
  "type": "add-text",
  "position": { "x": 120.0, "y": 110.0 },
  "runs": [
    { "text": "H", "fontSize": 10.0, "script": "normal" },
    { "text": "2", "fontSize": 10.0, "script": "subscript" },
    { "text": "O", "fontSize": 10.0, "script": "normal" }
  ],
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000"
}
```

Run fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `text` | string | text fragment |
| `fontFamily` | string | optional |
| `fontSize` | number | optional |
| `fill` | string | optional color |
| `fontWeight` | number | optional, for example `400` or `700` |
| `fontStyle` | string | optional, for example `normal` or `italic` |
| `underline` | boolean | optional |
| `outline` | boolean | optional |
| `shadow` | boolean | optional |
| `script` | string | `normal`, `subscript`, `superscript`, `chemical` |

### 7.3 Replace Text Object Content

```json
{
  "type": "set-text-runs",
  "objectId": "text_1",
  "runs": [
    { "text": "Fe", "script": "normal", "fontSize": 10.0 },
    { "text": "3+", "script": "superscript", "fontSize": 10.0 }
  ],
  "fontFamily": "Arial",
  "fontSize": 10.0,
  "fill": "#000000"
}
```

Or use plain `text`:

```json
{
  "type": "set-text-runs",
  "objectId": "text_1",
  "text": "Updated note",
  "fontSize": 11.0
}
```

### 7.4 Edit Text Style

```json
{
  "type": "apply-text-style",
  "textObjectIds": ["text_1"],
  "labelNodeIds": [],
  "nodeIds": [],
  "command": "bold",
  "value": "true"
}
```

## 8. Shape Objects

### 8.1 Create Shape

```json
{
  "type": "add-shape",
  "kind": "rect",
  "style": "solid",
  "color": "#000000",
  "begin": { "x": 80.0, "y": 80.0 },
  "end": { "x": 180.0, "y": 140.0 }
}
```

`kind` values:

```text
circle
ellipse
round-rect
rect
cross-table
tlc-plate
```

`style` values:

```text
solid
dashed
shaded
filled
shadowed
```

### 8.2 Set Shape Geometry

Applies to `circle`, `ellipse`, `rect`, and `round-rect`.

```json
{
  "type": "set-shape-geometry",
  "objectId": "shape_1",
  "begin": { "x": 90.0, "y": 90.0 },
  "end": { "x": 210.0, "y": 150.0 }
}
```

For `circle` and `ellipse`, `begin` is the center and `end` is the major-axis endpoint. For `rect` and `round-rect`, they are opposite bounding-box corners.

### 8.3 Edit Shape Style

```json
{
  "type": "apply-shape-style",
  "objectIds": ["shape_1"],
  "style": "shadowed"
}
```

`style` values: `plain`, `dashed`, `filled`, `shaded`, `faded`, `shadowed`.

### 8.4 Edit Native Spectrum Data

Updates every semantic field of an existing `spectrum` object. The data array is
replaced atomically and the command is undoable.

```json
{
  "type": "set-spectrum-data",
  "objectId": "obj_spectrum_1",
  "spectrum": {
    "class": "nmr",
    "xLow": 0,
    "xSpacing": 0.5,
    "xType": "parts-per-million",
    "xAxisLabel": "ppm",
    "yLow": 0,
    "yScale": 1,
    "yType": "arbitrary-units",
    "yAxisLabel": "intensity",
    "dataPoints": [0, 0.2, 1, 0.3, 0]
  }
}
```

NMR prediction uses a separate deterministic provider. The engine exposes a
versioned prediction request for exactly one fully selected molecule and turns
a successful `chemsema.nmr-prediction-response.v2` response into a new native
result document. The GUI exposes these adapters through the `NMR Prediction`
context submenu; prediction data is never substituted when the provider or a
verified rule is unavailable. Browser and desktop builds ship the same
independent `viewer/nmr-engine` WASM provider and initialize it on the first
prediction; the document engine does not link the prediction crate.

## 9. Brackets And Symbols

### 9.1 Create Bracket

```json
{
  "type": "add-bracket",
  "kind": "square",
  "begin": { "x": 100.0, "y": 100.0 },
  "end": { "x": 180.0, "y": 160.0 }
}
```

### 9.2 Create Symbol

```json
{
  "type": "add-symbol",
  "kind": "circle-plus",
  "center": { "x": 220.0, "y": 120.0 }
}
```

`kind` values:

```text
round
square
curly
double-dagger
dagger
circle-plus
plus
radical-cation
lone-pair
circle-minus
minus
radical-anion
electron
```

### 9.3 Edit Bracket Kind

```json
{
  "type": "apply-bracket-kind",
  "objectIds": ["bracket_1"],
  "kind": "curly"
}
```

`apply-bracket-kind` accepts `round`, `square`, and `curly`.

## 10. Orbital Objects

### 10.1 Create Orbital

```json
{
  "type": "add-orbital",
  "template": "p",
  "style": "hollow",
  "phase": "plus",
  "color": "#000000",
  "center": { "x": 300.0, "y": 120.0 },
  "end": { "x": 340.0, "y": 120.0 }
}
```

| Field | Values |
| --- | --- |
| `template` | `s`, `p`, `dxy`, `oval`, `hybrid`, `dz2`, `lobe` |
| `style` | `hollow`, `shaded`, `filled` |
| `phase` | `plus`, `minus` |

### 10.2 Edit Orbital Template

```json
{
  "type": "apply-orbital-template",
  "objectIds": ["orbital_1"],
  "template": "dxy"
}
```

### 10.3 Edit Orbital Style

```json
{
  "type": "apply-orbital-style",
  "objectIds": ["orbital_1"],
  "style": "filled"
}
```

### 10.4 Edit Orbital Phase

```json
{
  "type": "apply-orbital-phase",
  "objectIds": ["orbital_1"],
  "phase": "minus"
}
```

## 11. General Target Editing

Command scripts run in both one-shot mode (`new`/`run`) and JSONL session mode
(`session` + `execute`). Selection commands change the current in-memory
selection for later commands in the same script or session, but they do not
change the document revision by themselves.

### 11.1 Selection State

Select explicit targets:

```json
{
  "type": "select-targets",
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1"],
    "objects": ["text_1", "arrow_1"],
    "labelNodes": []
  }
}
```

Use one target for single-select and multiple targets for multi-select. Select
everything visible/editable:

```json
{ "type": "select-all" }
```

Clear the current selection:

```json
{ "type": "clear-selection" }
```

After `select-targets` or `select-all`, GUI-style selection commands can omit
their id arrays and operate on the current selection. This applies to
`apply-selection-arrange`, `scale-selection`, `center-selection-on-page`,
`apply-selection-color`, `apply-selection-order`, `group-selection`,
`ungroup-selection`, `link-selection`, `unlink-selection`, `apply-text-style`,
`apply-bond-style`, `apply-line-style`, shape/bracket/orbital style commands,
`apply-object-settings-to-selection`, `delete-selection`, and
`cut-selection`.

### 11.2 Move Targets

```json
{
  "type": "move-targets",
  "targets": {
    "nodes": ["node_1"],
    "bonds": [],
    "objects": ["text_1", "arrow_1"],
    "labelNodes": []
  },
  "delta": { "dx": 10.0, "dy": -5.0 }
}
```

### 11.3 Rotate Targets

```json
{
  "type": "rotate-targets",
  "targets": {
    "nodes": ["node_1", "node_2"],
    "bonds": ["bond_1"],
    "objects": ["arrow_1"],
    "labelNodes": []
  },
  "center": { "x": 200.0, "y": 200.0 },
  "degrees": 30.0
}
```

### 11.4 Scale Or Stretch Targets

```json
{
  "type": "scale-targets",
  "targets": {
    "nodes": ["node_1", "node_2"],
    "bonds": ["bond_1"],
    "objects": ["text_1"]
  },
  "scaleX": 1.25,
  "scaleY": 0.8,
  "pivot": { "x": 200.0, "y": 200.0 }
}
```

Omit `pivot` to scale around the selected target bounds center. Use equal
`scaleX`/`scaleY` for uniform scaling and unequal values for scripted stretch.

### 11.5 Delete Targets

```json
{
  "type": "delete-targets",
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1"],
    "objects": ["text_1"],
    "labelNodes": []
  }
}
```

Target fields:

| Field | Target |
| --- | --- |
| `nodes` | molecule nodes |
| `bonds` | molecule bonds |
| `objects` | scene objects such as text, arrow, shape, bracket, symbol, orbital |
| `labelNodes` | atom label nodes |

## 12. Arrange, Group, And Z Order

### 12.1 Arrange Current Selection

```json
[
  {
    "type": "select-targets",
    "targets": { "objects": ["text_1", "text_2", "text_3"] }
  },
  {
    "type": "apply-selection-arrange",
    "command": "align-left"
  }
]
```

`command` values:

```text
align-left
align-right
align-top
align-bottom
align-h-center
align-v-center
distribute-h
distribute-v
flip-h
flip-v
```

### 12.2 Z Order

```json
{
  "type": "apply-selection-order",
  "objectIds": ["arrow_1", "text_1"],
  "command": "bring-front"
}
```

`command` values:

```text
bring-front
send-back
bring-forward
send-backward
front
back
forward
backward
```

When `objectIds` is omitted, the command uses the current selection.

### 12.3 Group

```json
{
  "type": "group-selection",
  "object_ids": ["arrow_1", "text_1"]
}
```

When `object_ids` is omitted, the command uses the current selection.

### 12.4 Ungroup

```json
{
  "type": "ungroup-selection",
  "object_ids": ["group_1"]
}
```

When `object_ids` is omitted, the command uses the current selection.

### 12.5 Link Policy And Typed Links

```json
{
  "type": "link-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

```json
{
  "type": "set-link-policy",
  "objectIds": ["bracket_1", "text_1"],
  "policy": "auto"
}
```

```json
{
  "type": "unlink-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

When object ids are omitted, the command uses the current selection. `policy`
is `auto`, `linked`, or `unlinked`. Explicit Link is accepted only when the
selection matches exactly one registered typed-link signature; two molecules
are deliberately not linkable. See `docs/link-model.zh-CN.md` for the complete
data and interaction contract.

To paste the current complete single-molecule analysis as a linked caption:

```json
{
  "type": "paste-analysis-caption",
  "digits": 2
}
```

## 13. Document Style And Object Settings

### 13.1 Apply Document Style Preset

```json
{
  "type": "apply-document-style",
  "preset": "acs-document-1996"
}
```

`preset` values:

```text
default
acs-document-1996
```

ChemSema JSON persists the active file defaults as `style.preset` and
`style.defaults`. `chemsema-cli run` restores those defaults from the input file,
so later edit commands use the file's bond length, line width, wedge width,
hash spacing, margins, and font defaults unless the command explicitly provides
different values.

### 13.2 Set Default Object Settings

```json
{
  "type": "apply-object-settings",
  "settings": {
    "bondLength": 14.4,
    "lineWidth": 0.6,
    "boldWidth": 2.0,
    "bondSpacing": 18.0,
    "marginWidth": 1.6,
    "hashSpacing": 2.5
  }
}
```

### 13.3 Apply Settings To Specific Objects

```json
{
  "type": "apply-object-settings-to-selection",
  "bond_ids": ["bond_1"],
  "object_ids": ["arrow_1", "shape_1"],
  "settings": {
    "bondLength": 14.4,
    "lineWidth": 0.6,
    "boldWidth": 2.0,
    "bondSpacing": 18.0,
    "marginWidth": 1.6,
    "hashSpacing": 2.5
  }
}
```

All `settings` fields are optional.

## 14. Document Read/Write Commands Inside Scripts

The CLI subcommands cover most file IO. Use these JSON commands when a script needs structured document output.

Inspect current document:

```json
{
  "type": "inspect-document",
  "include": ["summary", "objects", "molecules"]
}
```

Export current document:

```json
{
  "type": "export-document",
  "format": "svg"
}
```

Convert content inside a script:

```json
{
  "type": "convert-document",
  "from": "cdxml",
  "to": "json",
  "content": "<CDXML>...</CDXML>"
}
```

`format`, `from`, and `to` values:

```text
json
ccjs
cdxml
cdx
sdf
svg
```

## 15. Generate Benzene And An Arrow From Blank

`commands.json`:

```json
[
  {
    "type": "insert-template",
    "template": "benzene",
    "x": 300.0,
    "y": 260.0
  },
  {
    "type": "add-arrow",
    "begin": { "x": 370.0, "y": 260.0 },
    "end": { "x": 520.0, "y": 260.0 },
    "variant": "solid",
    "headSize": "small",
    "curve": "arc270",
    "headStyle": "full",
    "tailStyle": "none",
    "head": true,
    "tail": false,
    "bold": false,
    "noGo": "none"
  }
]
```

Save as desktop CDXML:

```powershell
npm run cli -- new commands.json --out "$env:USERPROFILE\Desktop\benzene-arrow.cdxml" --results results.json --pretty
```

Inspect:

```powershell
npm run cli -- inspect "$env:USERPROFILE\Desktop\benzene-arrow.cdxml" --include summary,objects,molecules --pretty
```

## 16. Standard Workflow For Editing Existing Files

First, discover available ids and exact selectors:

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out before.json --pretty
npm run cli -- targets input.cdxml --out targets.json --pretty
```

When the edit depends on surrounding objects, inspect the neighborhood and then
expand one selector:

```powershell
npm run cli -- context input.cdxml --target object:arrow_1 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
npm run cli -- detail input.cdxml --target object:arrow_1 --out detail.json --pretty
```

Then write an edit script:

```json
[
  {
    "type": "apply-document-style",
    "preset": "acs-document-1996"
  },
  {
    "type": "apply-bond-style",
    "bondIds": ["bond_1"],
    "style": "double-center"
  },
  {
    "type": "set-arrow-geometry",
    "objectId": "arrow_1",
    "begin": { "x": 360.0, "y": 260.0 },
    "end": { "x": 540.0, "y": 260.0 },
    "curve": 0.0,
    "headStyle": "full",
    "tailStyle": "none"
  },
  {
    "type": "set-text-runs",
    "objectId": "text_1",
    "text": "Updated condition",
    "fontSize": 10.0
  }
]
```

Run and save:

```powershell
npm run cli -- run input.cdxml edit.json --out output.cdxml --results edit-results.json --document-json after.ccjs --pretty
```

Inspect again:

```powershell
npm run cli -- inspect output.cdxml --include summary,objects,molecules --out after.json --pretty
```

## 17. Import SMILES And Copy Chemical Identifiers

Create a document from SMILES with the shared Rust chemistry kernel:

```powershell
npm run cli -- insert-smiles "CC(=O)O" --out acetic-acid.ccjs --x 120 --y 100 --pretty
```

`--input existing.ccjs` inserts into an existing document. The final rendered
selection box is anchored at `--x`, `--y`; dot-separated components remain one
import record.

Generate an identifier for the first complete molecule, or restrict the input
with repeated `--node <id>` options:

```powershell
npm run cli -- chemistry acetic-acid.ccjs --format smiles --pretty
npm run cli -- chemistry acetic-acid.ccjs --format inchi --pretty
npm run cli -- chemistry acetic-acid.ccjs --format inchi-key --pretty
```

SMILES output is isomeric and uses input-order-independent canonical atom
ranking. Tetrahedral parity and normalized E/Z slash directions are rewritten
for the canonical traversal, so these cases also report `"canonical": true`.
Ordinary tetrahedral centers include a `cip` R/S descriptor when all four
ligands are distinguishable by the supported comparison. InChI and
InChIKey use the official IUPAC InChI 1.07.5
implementation locally: native builds statically link the vendored C library,
while the GUI loads the official WebAssembly build. No Python runtime or remote
service is involved.

The current InChI bridge preserves normalized E/Z through deterministic stereo
coordinate projection and tetrahedral chirality through a configuration-aware
V2000 wedge. It rejects dative bonds instead of silently converting them
to single bonds; metal-disconnection and reconnected-layer policy remains a
separate chemistry-kernel task.

The same operations are available in command scripts:

```json
[
  { "type": "insert-smiles", "smiles": "CCO", "x": 120.0, "y": 100.0 },
  { "type": "chemical-analysis", "format": "smiles", "targets": {} }
]
```
