# ChemSema Command Script v1

Command scripts are JSON inputs for:

```powershell
chemsema-cli new commands.json --out output.cdxml --results results.json
chemsema-cli run input.cdxml commands.json --out edited.cdxml --results results.json
```

The input may be a single JSON object command, an array of command objects, or
the optional transaction envelope defined in
[Command Transaction v1](./command-transaction-v1.md). Use `-` instead of a
file path to read the command JSON from stdin.

## Stable Report Fields

`new` and `run` result reports include stable audit fields:

- `ok`
- `commandCount`
- `executedCount`
- `failedCount`
- `failedIndex`
- `failedIndices`
- `continueOnError`
- `document`
- `commands`

Per-command entries include:

- `index`
- `ok`
- `executed`
- `commandType`
- `document`
- `changeSummary`
- `error`

`document` includes hash/revision transition metadata. `changeSummary` includes
created, updated, deleted, and touched selector summaries when the engine
reports target deltas.

## Snapshot Policy

Reports are lightweight by default. Use `--inspect-after <include>` to request
per-command and final snapshots. Use `--inspect-after none` or
`--no-inspect-after` to force no snapshots.

## Selection State

Command scripts support GUI-style selection state:

- `select-targets` sets the current selection from explicit `targets`.
- `select-all` selects visible text/graphic objects and editable molecule
  nodes, bonds, label nodes, and molecule objects.
- `clear-selection` clears the current selection.

These commands do not change the document revision. Later commands in the same
`new`/`run` script can use the current selection. The same command JSON is used
by JSONL session `execute`.

Selection-driven commands include `apply-selection-arrange`, `scale-selection`,
`center-selection-on-page`, `apply-selection-color`, `apply-selection-order`,
`group-selection`, `ungroup-selection`, `link-selection`, `set-link-policy`,
`unlink-selection`, `paste-analysis-caption`, `apply-chemical-property`,
style commands, `apply-object-settings-to-selection`, `delete-selection`, and
`cut-selection`.

`apply-chemical-property` creates a property when `propertyId` is omitted and
the current selection is one complete molecule; with `propertyId`, it edits
that logical property. `delete-chemical-property` and
`apply-chemical-property-result` address a property by id.

Target-driven commands that do not depend on current selection include
`move-targets`, `rotate-targets`, `scale-targets`, and `delete-targets`.
