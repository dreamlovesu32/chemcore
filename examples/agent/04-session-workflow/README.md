# 04 Session Workflow

This example keeps one CDXML document loaded while an agent asks for targets,
details, context, and a multi-target crop.

```powershell
.\one-shot.ps1
```

Outputs:

- `session.jsonl`: request stream.
- `transcript.jsonl`: ready event and responses.
- `session-selection.png`: multi-target selection crop written by the session.
