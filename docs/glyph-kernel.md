# Glyph kernel

ChemSema derives label layout and bond-retreat geometry in the Rust engine from real font outlines stored in [shared/glyph_outlines.json](../shared/glyph_outlines.json). The manifest contains multiple font families and regular, bold, italic, and bold-italic faces where available. Each face also carries its measured horizontal kerning pairs, so visible text, character anchors, and retreat geometry use the same glyph positions.

For a label margin `m` and glyph font size `s`, the retreat kernel uses:

```text
q = min(m, 0.25 * s)
natural = real outline dilated by a Euclidean disk of radius m
feature = circles of radius 1.5q at hull vertices moved 0.5q toward the glyph center
axial = contact sectors within 10 degrees of the four cardinal directions
clip = natural union feature union axial
```

Bond bodies, including their finite width, are clipped against `clip`. This is a direct rule, not a character lookup table or a 360-degree fit.

## Attachment anchoring

Text anchoring and bond retreat use different metric domains. Bond retreat
always uses the real outline kernel above. Whole-text nickname/query labels use
the selected face's GDI advance cells for horizontal anchoring. Scripted text
keeps its script-scaled advance cell only for an explicit left/right free
endpoint (`NodeType=Unspecified`, one connection, and invalid chemical
interpretation); recognized chemical fragments keep outline-based chemical
group anchoring. ChemDraw also treats an explicitly zero-hydrogen divalent
boron label as one GDI advance cell. These are field- and topology-derived
branches, not label-string exceptions.

Multiple bonds likewise keep the algebraically retreated bond axis when two
endpoint labels overlap. Each parallel rail is then clipped independently at
its own lateral offset. A single bond still disappears when both label retreats
consume its visible centerline.

## Data ownership

- `glyphPolygons` contains per-character real-outline hulls for editing, hit testing, and character anchors.
- `glyph_clip_polygons` is derived runtime-only retreat geometry. It is never CCJS authority and is not serialized.
- The former `shared/glyph_clip_polygons.json` character table and its generator have been removed; there is no legacy renderer fallback.
- Missing characters use an explicit font substitution chain, ending at the real `□` glyph outline. The substituted outline supplies both metrics and retreat geometry; there is no synthesized rectangular retreat fallback.
- Kerning comes from the selected font face's horizontal OpenType/TrueType `kern` data. It applies only between adjacent glyphs with the same resolved face, size, and script baseline; it never crosses a font-substitution, size, style, or script boundary.
- Kerning is not injected into the coarse, source-neutral text-width estimate used to pad a document viewBox. That estimate preserves the authored page frame; exact kerning belongs to glyph placement and must not translate or resize an otherwise unchanged drawing.

## Rebuild timing

The two geometry layers are rebuilt atomically on document load, text-edit confirmation, and font/style or MarginWidth changes. Typing inside an open text editor does not mutate document geometry. Label dragging translates both layers on every pointer move, so bond retreat stays live before pointer-up.

## Generation and verification

Run `python scripts/generate-glyph-outlines.py`. The `.mjs` entry point delegates to the same generator so there is only one schema implementation. Manifest version 3 stores outlines, advances, and face kerning together. The build script gzip-compresses the manifest before embedding it and the kernel expands it once on first use. Verify with the Rust engine test suite and the viewer WASM build.
