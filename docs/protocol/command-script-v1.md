# ChemCore Command Script v1

Command scripts are JSON inputs for:

```powershell
chemcore-cli new commands.json --out output.cdxml --results results.json
chemcore-cli run input.cdxml commands.json --out edited.cdxml --results results.json
```

The input may be a single JSON object command or an array of command objects.
Use `-` instead of a file path to read the command JSON from stdin.

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
