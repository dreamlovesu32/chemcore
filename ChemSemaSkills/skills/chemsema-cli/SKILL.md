---
name: chemsema-cli
description: Use ChemSema CLI for chemical document inspection, editing, capture, conversion, validation, canonicalization, migration, conformance, SMILES insertion, chemical identifiers, Office payload debugging, and JSONL sessions. Trigger this skill when working with ChemSema documents or formats such as CCJS, CCJZ, CDXML, CDX, SDF, SVG, PNG, EMF, chemical-graph-v2, SMILES, InChI, InChIKey, Office clipboard payloads, ChemSema command scripts, selectors, molecule targets, label-query, plan-bond, plan-template, capture, context, detail, targets, new, run, convert, export, validate, canonicalize, migrate, conformance, insert-smiles, chemistry, or session workflows.
---

# ChemSema CLI

## Core Workflow

Use the CLI as the machine contract for ChemSema. The public skill is
self-contained when it includes a platform runtime under `assets/bin`; do not
ask users to install Rust, Cargo, Node, or build ChemSema from source for normal
CLI work. Prefer runtime discovery over memory:

```powershell
chemsema-cli version --pretty
chemsema-cli doctor --pretty
chemsema-cli capabilities --pretty
chemsema-cli schema all --out chemsema-schema.json --pretty
chemsema-cli guide --kind detailed --out chemsema-guide.json --pretty
```

For large JSON, always pass `--out <path>` and read the file. `--pretty` only
changes whitespace.

Use this order for document work:

1. `about`, `examples`, `guide`, `schema`, and `capabilities` to discover the
   installed runtime.
2. `validate` for structural, chemical, or actual round-trip checks;
   `canonicalize`, `migrate`, and `conformance` for governed CCJS/CCJZ work.
3. `inspect` for whole-document summaries.
4. `targets` to discover stable selectors.
5. `context` to inspect neighborhoods and selection boxes.
6. `detail` to expand one object, molecule, node, or bond.
7. `capture` for deterministic SVG/PNG crops.
8. `bundle` when one selector needs detail, context, capture, editable subset,
   identity map, provenance, and manifest artifacts together.
9. `new`, `run`, or `insert-smiles` for edits; use
   `chemsema.command-transaction.v1` envelopes for scoped dry-runs or atomic
   guarded edits.
10. `chemistry` for `chemical-graph-v2`, canonical `smiles`, `inchi`, or
    `inchi-key` analysis of one complete molecule.
11. `diff` to compare before/after editable documents by structured IDs.
12. `convert` or `export` for whole-document output, or target-only editable
   subset output with `--target`/`--targets`.
13. `copy` for Windows Office/OLE clipboard payloads.
14. `session` for repeated work on one document.

## Read References As Needed

- For locating the executable and handling bundled vs installed builds, read
  `references/runtime-discovery.md`.
- For selectors, targets, detail, context, capture, bundle, and diff, read
  `references/document-inspection.md`.
- For `new`, `run`, `session execute`, command JSON, selection state, target
  editing, arrange/group/link/style commands, `plan-bond`, and `plan-template`,
  read `references/command-scripts.md`.
- For `validate`, `canonicalize`, `migrate`, `conformance`, `insert-smiles`,
  `chemistry`, `convert`, `export`, editable formats, chemical identifiers,
  and raster/vector output, read
  `references/formats-conversion.md`.
- For chemical text, visible text, reverse labels, anchors, and
  `defaultChemical:false`, read `references/label-query.md`.
- For long iterative operations, read `references/session-jsonl.md`.

## Helpers

Use bundled helpers instead of retyping discovery code:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\find_chemsema_cli.ps1 -Json
python scripts\run_chemsema_cli.py version --pretty
python scripts\session_jsonl.py input.cdxml requests.jsonl --out transcript.jsonl
python scripts\check_cli_skill_sync.py --suite-root ..\..\..\.. --json
```

## Guardrails

- Treat source checkout builds as development work. Use the
  `chemsema-development` skill for `cargo run`, package rebuilding, and local
  release validation.
- Do not parse console text when a JSON output path exists.
- Do not invent selectors; discover them from `targets` or `context`.
- Do not manually calculate label reversal, generated hydrogens, or chemical
  anchors when `label-query` can answer.
- Do not manually assemble temporary CCJS just to export part of a document;
  discover selectors with `targets` and use `convert`/`export --target` for
  editable subset output.
- Preserve original document semantics when the visible drawing intentionally
  disagrees with default chemical rewriting; use `defaultChemical:false` when
  appropriate.
