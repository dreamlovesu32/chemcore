# Natural-Language Request

Open the public reaction figure `figure1.cdxml`.

Find the first reaction arrow and its nearby condition label. Add a visible
review annotation that marks the condition area for human verification, without
changing the underlying chemistry. Return editable output, before/after crops,
and an audit report that records exactly which deterministic ChemCore commands
were executed.

Expected agent behavior:

- discover selectors with `chemcore-cli targets`;
- use `context` and `detail` before editing;
- execute only deterministic JSON commands;
- export editable CDXML and visual SVG/PNG outputs;
- keep the final result human-reviewable.
