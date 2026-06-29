# ChemCore CLI Command Guide

This guide is written for callers that use `chemcore-cli` directly. A caller should not need to read the ChemCore source code to know how to open files, create objects, edit objects, inspect results, or recover from command errors.

## 1. Start The CLI

Run from the repository root:

```powershell
npm run cli -- <command> [args...]
```

Equivalent Cargo command:

```powershell
cargo run -p chemcore-cli -- <command> [args...]
```

After building, the executable can be called directly:

```powershell
target\debug\chemcore-cli.exe <command> [args...]
```

Show help:

```powershell
npm run cli -- --help
```

If Windows PowerShell blocks `npm.ps1` by execution policy, use `npm.cmd`:

```powershell
npm.cmd run cli -- --help
```

Installed desktop builds also install `chemcore-cli.exe` next to the GUI and
ship this English guide as `chemcore-cli-guide.md`. The installer adds the CLI
directory to PATH. Open a new terminal after installing, then start with:

```powershell
chemcore-cli guide --pretty
chemcore-cli guide --kind detailed --pretty
chemcore-cli doctor --pretty
chemcore-cli capabilities --pretty
```

`--pretty` only changes JSON whitespace: compact JSON becomes line-broken and
indented. It does not change fields, values, output files, exit code, schema,
ordering, or command behavior. Without `--pretty`, JSON is compact single-line
JSON.

## 2. File Commands

Opening a file means passing the file path to `inspect`, `run`, `convert`, or `export`.

```text
chemcore-cli guide [--kind agent|detailed|all] [--include-content] [--pretty] [--out <path>]
chemcore-cli about [--pretty] [--out <path>]
chemcore-cli capabilities [--pretty] [--out <path>]
chemcore-cli doctor [--pretty] [--out <path>]
chemcore-cli examples [basic|capture-copy|all] [--pretty] [--out <path>]
chemcore-cli schema [commands|targets|capture|context|detail|guide|copy|json-output|command-script|all] [--pretty] [--out <path>]
chemcore-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]
chemcore-cli targets <input> [--out <path>] [--pretty]
chemcore-cli context <input> --target <selector> [--radius <pt>] [--out <context.json>] [--capture-out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--pretty]
chemcore-cli detail <input> --target <object:id|molecule:index|node:id|bond:id> [--summary-only] [--include-resource] [--out <detail.json>] [--pretty]
chemcore-cli capture <input> --target <selector> [--out <path.svg|path.png>] [--scale <n>|--width <px>|--height <px>] [--expand <pt>] [--expand-rel <fraction>] [--pretty]
chemcore-cli copy <input> [--target <selector>] [--payload <payload.json>] [--no-copy] [--pretty]
chemcore-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemcore-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
chemcore-cli convert <input> <output> [--format <format>]
chemcore-cli export <input> <output> [--format <format>]
```

Common calls:

```powershell
npm run cli -- inspect input.cdxml --include summary,objects,molecules --out inspect.json --pretty
npm run cli -- targets input.cdxml --out targets.json --pretty
npm run cli -- capture input.cdxml --target molecule:0 --out molecule.png --scale 6 --expand-rel 0.15 --pretty
npm run cli -- new commands.json --out output.cdxml --results results.json --pretty
npm run cli -- run input.cdxml commands.json --out output.cdxml --results results.json --document-json after.ccjs --pretty
npm run cli -- convert input.cdxml output.svg
npm run cli -- convert input.cdxml output.ccjs
```

File output policy:

- `capture` may omit `--out`; it then writes a PNG into the OS temp `chemcore-cli` directory, reports the exact path in `output.path`, and emits a `default_output_path` warning.
- `copy` may omit `--payload`; it then writes the clipboard payload JSON into the OS temp `chemcore-cli` directory, reports the exact path in `payload.path`, and emits a `default_payload_path` warning.
- `new`, `convert`, and `export` require explicit output paths because they create primary document files.
- Every file-writing command verifies after writing that the target exists, is a regular file, and has the expected or minimum byte size. A failed verification is a command failure.

Error output policy:

- Error JSON includes `error.kind`, `message`, `hint`, `fix`, `usage`, `examples`, and `suggestions`.
- Missing argument errors use `error.kind="missing_argument"` and include `error.fix.action="provide_required_argument"` plus machine-readable `missing` and `expected` fields.
- Agents should read `error.fix` first, then fall back to `usage` and `examples`.

`new` starts from a blank ChemCore internal document. It does not need an input format. The save format is inferred from `--out`:

```powershell
npm run cli -- new --out blank.ccjs --quiet
npm run cli -- new commands.json --out figure.cdxml
npm run cli -- new commands.json --out figure.svg
```

Use `--save-format` when the output path has no clear extension or when writing to stdout:

```powershell
npm run cli -- new commands.json --out output --save-format cdxml
npm run cli -- run input.cdxml commands.json --out - --save-format svg --quiet
```

`convert` and `export` use `--format` to override the output format:

```powershell
npm run cli -- convert input.cdxml output --format svg
```

Supported formats:

| Format | Read | Write | Use |
| --- | --- | --- | --- |
| `json` | yes | yes | ChemCore internal JSON. `.json` is treated as internal JSON |
| `ccjs` | yes | yes | ChemCore internal JSON, uncompressed |
| `ccjz` | yes | yes | gzip-compressed ChemCore JSON |
| `cdxml` | yes | yes | ChemDraw XML |
| `cdx` | yes | yes | ChemDraw binary |
| `sdf` | yes | yes | MDL SD file |
| `svg` | no | yes | vector export |

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

Coordinates use ChemCore document coordinates. `x` increases to the right, and `y` increases downward.

## 4. Execution Reports, Ids, And Internal JSON

Pass `--results` when using `new` or `run`. `results.json` is the primary machine-readable record for whether commands executed, whether they changed the document, which ids were created/updated/deleted, what failed, and which input/output files were involved. By default it is a lightweight audit report, not a stored history stack.

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
    "hashInput": "chemcore-document-json-v1",
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
| `document` | document hash/revision before and after the script. Use this to verify whether the document changed without storing a full snapshot |
| `io` | operation name plus input/script/output paths for this invocation |
| `final` | optional inspect snapshot after the script stops, present only when `--inspect-after` is used |
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
    "hashInput": "chemcore-document-json-v1",
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
| `changed` | whether it changed the document. A valid no-op is `false` |
| `commandType` | original `type` value |
| `document` | document hash/revision before and after this command |
| `changeSummary` | selector-form summary of created/updated/deleted ids, intended for agent history |
| `created` | created node, bond, scene object, and style ids |
| `updated` | updated node, bond, scene object, and style ids |
| `deleted` | deleted node, bond, scene object, and style ids |
| `engineResult` | raw ChemCore engine result |
| `after` | optional inspect snapshot after this command, present only when `--inspect-after` is used |

Decision table:

| Situation | Meaning |
| --- | --- |
| `ok=true, executed=true, changed=true` | command executed and changed the document |
| `ok=true, executed=true, changed=false` | command was valid but produced no change |
| `ok=false, executed=false` | command did not execute. Read `error.message` |
| top-level `ok=false` and `save.skipped=true` | script failed, so `--out` was not saved |

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
| `read-script` | command JSON could not be read or was not an object/array |
| `execute-command` | invalid field, invalid enum value, missing field, or context-only command |
| `inspect-after` | optional inspect after one command failed |
| `inspect-final` | optional final inspect failed |
| `write-document-json` | `--document-json` write failed |
| `save-output` | `--out` save failed |

If a script fails, earlier successful commands remain in the in-memory document and are visible in `document`, command entries, and `--document-json` if requested. The target `--out` file is not saved.

### 4.4 Optional After Snapshots

By default, command reports do not include `after` snapshots and the top-level report does not include `final`. This keeps reports small for large documents and long command scripts. The CLI reports what changed; the caller or agent should maintain history with git, temporary files, or its own log.

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

Editing existing objects requires ids. Get ids from `inspect`, `targets`, `results.commands[i].created`, or `results.commands[i].changeSummary`. Use `results.commands[i].after` only when `--inspect-after` was requested.

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

There are three common ways to read full ChemCore internal JSON.

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

`--document-json` is useful for debugging because it can be used together with `--out output.cdxml`. If the script fails partway through, it writes the in-memory ChemCore JSON at the failure point.

### 4.7 Agent Target, Context, Detail, Capture, And Copy Workflow

Use this workflow when an agent needs exact ids, exact crops, or nearby-object
context without guessing coordinates:

```powershell
chemcore-cli targets input.cdxml --out targets.json --pretty
chemcore-cli context input.cdxml --target object:obj_shape_001 --radius 80 --out context.json --capture-out context.png --scale 5 --pretty
chemcore-cli detail input.cdxml --target object:obj_shape_001 --out detail.json --pretty
chemcore-cli capture input.cdxml --target object:obj_shape_001 --out object.png --scale 6 --expand-rel 0.15 --pretty
chemcore-cli copy input.cdxml --target object:obj_shape_001 --pretty
```

Selectors accepted by target-aware commands:

```text
all
object:<scene-object-id>
molecule:<zero-based-molecule-index>
node:<node-id>
bond:<bond-id>
bounds:<minX>,<minY>,<maxX>,<maxY>
```

`bounds:` is accepted by capture-style crops. `detail` accepts one object,
molecule, node, or bond selector; it does not accept `all` or `bounds`.

`targets` returns stable selectors and bounds grouped under `objects`,
`molecules`, `nodes`, and `bonds`. Run it before `context`, `detail`,
`capture`, or `copy` when the caller does not already know the exact selector.

`context` returns nearby object summaries, molecule summaries, node summaries,
bond summaries, bounds, direction, distance, overlap flags, group ancestry,
child ids, and link metadata. It returns summaries by design. Use `detail` on a
returned selector when raw JSON is needed.

`detail` returns one selected entity. By default, it includes raw JSON for that
entity. Add `--summary-only` when ids, bounds, and relationship metadata are
sufficient. Add `--include-resource` for an object when the referenced resource
must be embedded in the response.

`capture` writes the rendered crop to `--out` and writes only a JSON manifest to
stdout. If `--out` is omitted, it writes a PNG to the OS temp `chemcore-cli`
directory and reports `output.defaulted=true` plus the exact `output.path`.
It also emits a `warnings[]` entry with `kind="default_output_path"`.
SVG output is vector. PNG output defaults to `--scale 4`; use `--scale`,
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

`copy` places an editable ChemCore Office/OLE payload on the Windows clipboard.
If `--payload` is omitted, the payload JSON is written to the OS temp
`chemcore-cli` directory and a `default_payload_path` warning is emitted. Use `--payload payload.json --no-copy` to write the
clipboard payload without touching the clipboard.

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

### 5.3 Add A Bond Between Existing Atoms

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

### 5.4 Insert A Template

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
```

Create chains with multiple `add-bond` commands.

### 5.5 Edit Bond Style

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

### 5.6 Replace Atom Label

```json
{
  "type": "replace-node-label",
  "node_id": "node_1",
  "label": "OH"
}
```

### 5.7 Set Atom Label Runs

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
  "defaultChemical": true
}
```

### 5.8 Edit Atom Label Style

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

### 11.1 Move Targets

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

### 11.2 Rotate Targets

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

### 11.3 Delete Targets

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

### 12.1 Z Order

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

### 12.2 Group

```json
{
  "type": "group-selection",
  "object_ids": ["arrow_1", "text_1"]
}
```

### 12.3 Ungroup

```json
{
  "type": "ungroup-selection",
  "object_ids": ["group_1"]
}
```

### 12.4 Link And Unlink

```json
{
  "type": "link-selection",
  "object_ids": ["bracket_1", "text_1"]
}
```

```json
{
  "type": "unlink-selection",
  "object_ids": ["bracket_1", "text_1"]
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
