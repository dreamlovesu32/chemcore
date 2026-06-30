# ChemCore Agent Examples

These examples are small, reproducible workflows for agents that use
`chemcore-cli` without source-code context.

Set `CHEMCORE_CLI` to a local executable when testing from the repository:

```powershell
$env:CHEMCORE_CLI = "$PWD\target\debug\chemcore-cli.exe"
```

Otherwise the scripts call `chemcore-cli` from PATH, which is how installed
desktop builds expose the CLI.

Run every checked-in workflow from the repository root:

```powershell
examples/agent/run-all.ps1
```

Or through npm:

```powershell
npm run examples:agent
```

## Examples

- `01-discover-targets`: list selectors from a public CDXML file.
- `02-context-crop`: inspect an arrow object's neighborhood and render a crop.
- `03-edit-reaction-scheme`: create a small editable document from JSON
  commands and capture it.
- `04-session-workflow`: run repeated target/detail/context/capture requests
  through one JSONL session.
- `05-office-copy`: generate an Office/OLE clipboard payload JSON without
  touching the clipboard.
- `06-reaction-poc`: run a compact external-review POC on a public reaction
  figure, producing context, before/after crops, editable output, and an audit
  report.

The checked-in JSON and PNG outputs are generated from the same scripts and act
as lightweight regression examples.
