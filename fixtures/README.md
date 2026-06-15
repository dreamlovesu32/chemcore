# Public Fixtures

This directory contains public, license-clear regression fixtures.

- `cdxml/` stores synthetic CDXML source files.
- `expected/svg/` stores ChemCore SVG golden snapshots generated from those CDXML files.

The fixtures are intentionally small. They should cover one or two behaviors at a
time, such as label clipping, arrow geometry, object stacking, or text layout.
Committed fixtures should be shareable and suitable for open-source regression
coverage.

Regenerate one SVG snapshot with:

```bash
cargo run -p chemcore-engine --example cdxml_to_svg -- fixtures/cdxml/synthetic-reaction.cdxml fixtures/expected/svg/synthetic-reaction.svg
```
