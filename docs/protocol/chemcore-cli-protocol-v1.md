# ChemCore CLI Protocol v1

Status: beta contract for ChemCore `1.0.0-beta.5` and later beta builds.

Protocol id: `chemcore-cli-protocol.v1`.

This document defines the stable machine-facing surface of `chemcore-cli`.
Human-facing wording in README files and guides may change freely; callers
should depend on this document, `chemcore-cli version`, `chemcore-cli schema`,
and `chemcore-cli capabilities`.

## Versioning

The product version is reported by:

```powershell
chemcore-cli --version
chemcore-cli version --pretty
```

The protocol version is independent from the product version. Beta releases may
add fields, commands, examples, and enum values without changing the protocol
id. Removing a stable field, changing a stable field type, or changing selector
semantics requires a new protocol id such as `chemcore-cli-protocol.v2`.

## Stable Commands

The following command names are stable in v1:

- `version`
- `capabilities`
- `schema`
- `doctor`
- `about`
- `examples`
- `guide`
- `inspect`
- `targets`
- `context`
- `detail`
- `capture`
- `copy`
- `session`
- `new`
- `run`
- `convert`
- `export`

Aliases such as `details`, `describe`, and `show` are convenience aliases. They
are useful, but automation should prefer canonical names.

## Stable Output Rules

- Successful JSON commands include `ok: true`.
- Failed commands print JSON with `ok: false` and an `error` object.
- `--pretty` only changes whitespace and indentation. It does not change fields,
  values, output files, exit code, schema, ordering, or command behavior.
- Commands that write files verify the file after writing before reporting
  success.
- Large payloads should be requested with `--out <path>` or command-specific
  file outputs instead of relying on console buffers.

## Related Contracts

- [Selectors](./selector-v1.md)
- [JSONL Session](./session-jsonl-v1.md)
- [Command Scripts](./command-script-v1.md)
- [Capture Manifest](./capture-manifest-v1.md)
- [Error Model](./error-model-v1.md)
- [Entrypoints](./entrypoints-v1.md)
