# 03 Edit Reaction Scheme

This example creates a tiny editable ChemCore document from JSON commands,
writes an audit report, then captures the full result.

```powershell
.\one-shot.ps1
```

Outputs:

- `output.ccjs`: generated editable ChemCore JSON document.
- `expected-results.json`: command audit report.
- `crop.png`: full-document capture.
- `capture.json`: capture manifest.
