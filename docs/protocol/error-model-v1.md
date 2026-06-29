# ChemCore CLI Error Model v1

Protocol id: `chemcore-cli-error.v1`.

Failed CLI commands print JSON:

```json
{"ok":false,"error":{"kind":"missing_argument","message":"..."}}
```

## Stable Error Fields

- `kind`
- `message`
- `hint`
- `fix`
- `usage`
- `examples`
- `suggestions`
- `command`
- `argument`

Not every error includes every field. Callers should branch first on `kind`,
then use `fix` and `suggestions` when present.

## Stable Kinds

- `unknown_command`
- `missing_argument`
- `invalid_argument`
- `command_error`

Missing argument errors include `fix.action="provide_required_argument"` and
machine-readable `missing` and `expected` fields when the CLI can infer them.

Unknown command errors include nearby command suggestions with command name,
summary, usage, and example.
