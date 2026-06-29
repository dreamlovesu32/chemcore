# ChemCore Entrypoints v1

Schema id: `chemcore.entrypoints.v1`.

Installed desktop builds ship a self-description file named
`chemcore-entrypoints.json`. It is intended for tools that discover ChemCore
without repository context.

## Stable Sections

- `schema`
- `product`
- `entrypoints`
- `packaging`
- `documentation`
- `formats`
- `agentWorkflow`

`entrypoints.cli` describes `chemcore-cli.exe`, installed path hints, and first
commands to run. `entrypoints.gui` describes the desktop executable and file
associations. `entrypoints.officeOleHelper` describes the Office/OLE helper.

## Discovery

Installed agents should run:

```powershell
chemcore-cli version --pretty
chemcore-cli guide --pretty
chemcore-cli doctor --pretty
chemcore-cli capabilities --pretty
```

When PATH is unavailable, callers can inspect installed path hints from
`chemcore-entrypoints.json` or Windows App Paths registration.
