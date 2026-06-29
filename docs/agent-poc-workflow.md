# ChemCore Agent POC Workflow

This workflow is the recommended proof of concept for agent integrations:

```text
natural-language request
  -> agent selects ChemCore commands
  -> chemcore-cli executes deterministic JSON commands
  -> ChemCore returns selectors, visual crops, output files, and audit reports
  -> human reviews the editable result
```

The POC should focus on reaction-scheme editing rather than autonomous
chemistry. A useful demo edits a public CDXML reaction figure, inspects nearby
objects, applies a small set of JSON commands, exports CDXML/SVG/PNG or an
Office payload, and keeps the `results.json` audit report.

## Demonstration Steps

1. Run `chemcore-cli version --pretty` and `chemcore-cli capabilities --pretty`.
2. Discover object ids with `chemcore-cli targets figure1.cdxml --pretty`.
3. Inspect a local region with `chemcore-cli context ... --capture-out ...`.
4. Retrieve exact object JSON with `chemcore-cli detail ...`.
5. Generate or modify a document with `chemcore-cli new` or `chemcore-cli run`.
6. Export visual and editable outputs with `capture`, `export`, or `copy`.
7. Store the command script, output document, rendered crop, and audit report.

The `examples/agent` corpus provides runnable one-shot and JSONL session
versions of these steps.
