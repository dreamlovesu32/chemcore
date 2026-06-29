# 05 Office Copy

This example generates an Office/OLE clipboard payload JSON for one object
without touching the Windows clipboard.

```powershell
.\one-shot.ps1
```

Outputs:

- `payload.json`: serialized clipboard payload.
- `copy-result.json`: copy command manifest.

Remove `--no-copy` in the script when the installed Office helper should place
the payload on the Windows clipboard.
