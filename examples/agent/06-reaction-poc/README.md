# 06 Reaction POC

This is the smallest checked-in POC for an external team evaluating ChemCore as
an agent-operable scientific document engine.

It uses the public repository fixture `figure1.cdxml`, discovers the first
reaction arrow region, captures nearby context, applies a deterministic JSON
edit script, and writes editable and visual outputs with audit metadata.

Run from the repository root:

```powershell
npm run poc:agent
```

Or run this directory directly:

```powershell
examples/agent/06-reaction-poc/run.ps1
```

## Workflow

1. Read the natural-language task in `request.md`.
2. Inspect the runtime protocol with `version.json` and `capabilities.json`.
3. Discover target selectors in `targets.json`.
4. Inspect the reaction arrow neighborhood in `context.json` and `context.png`.
5. Inspect the condition text in `condition-detail.json`.
6. Capture the original region as `before.png`.
7. Apply `commands.json` to generate `output.cdxml` and `results.json`.
8. Export `output.svg` and capture `after.png`.
9. Generate `office-payload.json` without touching the clipboard.

The edit intentionally stays conservative: it adds an agent review box and note
around an existing condition label. It demonstrates a human-reviewable document
operation, not autonomous chemistry.

## Outputs

- `version.json`: product and protocol ids.
- `capabilities.json`: machine-readable command and schema discovery.
- `targets.json`: selectors from the input CDXML.
- `context.json` / `context.png`: nearby objects around `object:obj_line_001`.
- `condition-detail.json`: raw detail for `object:obj_text_008`.
- `before.png`: crop before editing.
- `commands.json`: deterministic edit script.
- `results.json`: command audit report.
- `output.cdxml`: editable ChemDraw XML after the edit.
- `output.svg`: visual export of the edited document.
- `after.png`: crop after editing.
- `capture-before.json` / `capture-after.json`: capture manifests.
- `copy-result.json` / `office-payload.json`: Office/OLE payload manifest and data.
