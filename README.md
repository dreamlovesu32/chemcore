# chemcore

`chemcore` is a cross-platform chemistry document core.

The project goal is not "a web demo first, then a desktop rewrite later". The
goal is to keep the document model, editing behavior, hit testing, chemical
label logic, CDXML import/export, and render primitive generation in a shared
Rust core.

## Current Scope

The active implementation is centered on [`crates/chemcore-engine`](./crates/chemcore-engine):

- `document.rs`: `chemcore` v0.1 document model and JSON parsing
- `engine.rs` and `engine/*`: editing state, tools, command history, selection, deletion, clipboard, templates, text editing
- `render.rs` and `render_*`: backend-independent render primitives
- `cdxml.rs`: native Rust CDXML import and export
- `abbreviation.rs`, `label_rules.rs`, `symbols.rs`, `repeating_units.rs`: chemical label and symbol behavior
- `wasm.rs`: browser-facing engine bindings

The web editor under [`viewer`](./viewer) is the browser host. It owns toolbar
UI, file open/save, browser event handling, coordinate conversion, and SVG/DOM
drawing. It should consume engine state and render primitives rather than
redefining chemistry behavior.

## Design Documents

The current design baseline lives in:

- [docs/architecture.md](./docs/architecture.md)
- [docs/format-v0.1.md](./docs/format-v0.1.md)
- [docs/project-rules.zh-CN.md](./docs/project-rules.zh-CN.md)
- [docs/implicit-hydrogen-rules.zh-CN.md](./docs/implicit-hydrogen-rules.zh-CN.md)
- [docs/abbreviation-recognition-rules.zh-CN.md](./docs/abbreviation-recognition-rules.zh-CN.md)
- [docs/bond-rendering-rules.zh-CN.md](./docs/bond-rendering-rules.zh-CN.md)
- [docs/editor-command-history.md](./docs/editor-command-history.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)
- [examples/document-v0.1.ccjs](./examples/document-v0.1.ccjs)

## Workspace Layout

```text
chemcore/
  crates/chemcore-engine/    Rust document, editing, rendering, CDXML, WASM core
  viewer/                    Browser editor host and generated WASM package
  docs/                      Architecture, format, rendering, and behavior notes
  examples/                  Example ChemCore native documents
  scripts/                   Build, verification, and browser regression helpers
  shared/                    Shared JSON data consumed by Rust/viewer code
```

## Common Commands

```bash
cargo test
npm run build:engine-wasm
npm run dev:engine
npm run verify
node --check viewer/app.js
```

`npm run verify` runs Rust tests, rebuilds the browser engine WASM, checks the
viewer syntax, and verifies that generated `viewer/engine` files are in sync.
