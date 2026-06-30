# ChemCore CLI JSONL Session v1

Protocol id: `chemcore-cli-session-jsonl.v1`.

`chemcore-cli session [input]` starts a long-lived process over stdin/stdout.
The first stdout line is a ready event. After that, callers send one compact
JSON request per line and read one compact JSON response per line.

## Request Shape

```json
{"id":1,"op":"targets"}
```

Stable fields:

- `id`: optional caller value echoed in the response.
- `op`: operation name.

Stable operations:

- `open`
- `targets`
- `detail`
- `context`
- `capture`
- `execute`
- `save`
- `status`
- `close`
- `exit`

## Response Shape

Successful responses include:

```json
{"ok":true,"id":1,"op":"targets","result":{}}
```

Failed responses include:

```json
{"ok":false,"id":1,"op":"targets","error":{"kind":"operation_failed","message":"..."}}
```

## History Policy

The session keeps the current document in memory. It does not persist an undo
stack or full snapshot history. `execute` responses report before/after
revision and per-command results; callers should maintain durable history with
git, temporary files, or their own logs.

## File Outputs

`capture` and `save` may write files. File writes are verified before success is
reported. Prefer explicit output paths in automation.
