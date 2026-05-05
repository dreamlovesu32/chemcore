# Glyph Kernel

## Purpose

`chemcore` needs host-independent text geometry for chemical labels.

The browser should not be the authority for:

- per-glyph label geometry used by bond clipping
- glyph advance estimates
- subscript / superscript scaling and baseline shifts
- background padding used for knockout and label-aware bond retreat

If hosts derive these details independently, web and desktop renderers will drift.

## Current Model

The active glyph geometry implementation lives in Rust:

- [crates/chemcore-engine/src/glyph_kernel.rs](../crates/chemcore-engine/src/glyph_kernel.rs)

The Rust engine consumes shared normalized glyph profiles:

- [shared/glyph_profiles.json](../shared/glyph_profiles.json)
- [shared/text_symbols.json](../shared/text_symbols.json) lists the text-symbol
  catalog used by the viewer palette and by the profile generation script

The kernel defines:

- normalized glyph advances
- normalized ink bounds
- scalable padding
- rect / ellipse / cut-corner background shapes
- normal / subscript / superscript layout
- conservative Unicode-category fallbacks for characters that are not explicitly
  listed in the shared profile manifest

The output is used by attached-label layout, label anchor geometry, label-aware bond clipping, and text edit preview geometry.

Text-symbol profiles should be regenerated instead of hand-filled one character
at a time:

```bash
python scripts/generate-glyph-profiles.py
```

## Web Status

The web viewer does not run a separate glyph runtime. It consumes Rust engine state and render primitives through WASM:

- [crates/chemcore-engine/src/wasm.rs](../crates/chemcore-engine/src/wasm.rs)
- [viewer/app.js](../viewer/app.js)

The old C++ glyph kernel and standalone glyph WASM path have been removed. Current validation should go through the Rust engine tests and viewer engine WASM build.
