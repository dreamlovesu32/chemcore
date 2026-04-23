# Chemcore Developer Log - 2026-04-24

Author: Jiajun Zhang

Time range: 2026-04-23 20:47 to 2026-04-24 00:28, Asia/Shanghai

## Summary

Today's work bootstrapped the repository into a usable cross-platform chemistry document project. The main outcome is not just a viewer or a web demo; it is the first coherent split between:

- a readable `chemcore.json` document format,
- a CDXML import compatibility layer,
- a deterministic glyph geometry kernel,
- a web SVG viewer/editor shell,
- and a Rust editing engine that can later be shared by Web, Windows, and iPad.

The most important rendering milestone was the glyph clipping system. We moved chemical label geometry away from browser-dependent measurement and into a native glyph kernel that produces per-character boxes and optical shapes. This makes label knockout, bond retreat, wedge contact behavior, and future native renderers much more predictable.

## Repository Bootstrap

The repository was initialized and committed with the current viewer, docs, examples, ignore rules, and generated runtime artifacts. The initial documentation established the direction of the project:

- `docs/architecture.md`
- `docs/architecture.zh-CN.md`
- `docs/format-v0.1.md`
- `docs/format-v0.1.zh-CN.md`
- `docs/glyph-kernel.md`
- `docs/viewer-rendering-report.zh-CN.md`
- `docs/rust-engine-architecture.zh-CN.md`

The project now has four active implementation areas:

- `src/chemcore`: Python CDXML import and document conversion.
- `cpp/chemcore_glyph_kernel`: native glyph layout and clipping geometry.
- `crates/chemcore-engine`: Rust document editing engine.
- `viewer`: current Web shell, SVG rendering, and WASM bindings.

## Readable Chemcore JSON

The CDXML conversion layer was normalized so the exported JSON is more readable and less tied to CDXML internals.

Important changes:

- Nodes now expose `element: "N"` and `atomicNumber: 7` rather than source-only numeric element strings.
- Label runs expose explicit fields such as `fontWeight`, `fontStyle`, `script`, `fontSize`, and `fill`.
- Bond rendering intent is explicit:
  - `stereo.kind`
  - `stereo.wideEnd`
  - `double.placement`
- Original CDXML flags such as `face`, `font`, and `color` are treated as import metadata or compatibility data, not the preferred model.

This matters because the project is intended to become a native editor, not only a CDXML viewer. The internal JSON needs to describe chemistry and rendering intent directly.

## CDXML Text Style Fixes

CDXML `face` bits combine multiple concepts:

- bold,
- italic,
- subscript,
- superscript.

A common CDXML issue was that script-only runs could lose neighboring bold or italic bits. For example, in bold `Cu(CH3CN)4PF6`, the normal glyphs could be bold while `3`, `4`, and `6` were emitted as subscript-only runs.

The importer now expands and normalizes molecule label runs so script-only runs inherit adjacent style bits when appropriate. The viewer also prefers readable run fields where available.

Related fixes:

- `CF3` bold rendering was traced to a CSS override on `.mol-atom-label`; removing the default `font-weight: 400` allowed per-glyph bold output to take effect.
- `CF3` subscript rendering was traced to stale generated example JSON; regenerating examples allowed the existing script layout to work.

## Glyph Kernel and Glyph Clipping

This was the most important low-level rendering work today.

### Problem

Chemical drawing is very sensitive to label geometry. A bond should not collide with `N`, `CF3`, `Me`, `Ts`, or `B(OH)2`, but it also should not retreat from empty space inside a coarse text bounding rectangle.

Browser text measurement is not a stable authority for this project because:

- Web, Windows, and iPad would produce different glyph metrics.
- SVG text and native text can differ in advance widths and ink bounds.
- Chemical labels need per-glyph geometry, not only a whole text box.
- Subscript and superscript affect both size and baseline.

### Current Design

The C++ glyph kernel owns deterministic glyph geometry:

- glyph advance widths,
- ink boxes,
- background boxes,
- script scaling and baseline shifts,
- glyph-level optical shapes,
- anchor placement for attached labels.

The current kernel uses built-in normalized profiles generated from a reference font. It does not yet rasterize actual font pixels with FreeType or HarfBuzz. That is intentional for now: the goal is deterministic geometry that is good enough for chemical label avoidance and portable to future native hosts.

### Font Metrics

The initial `Me` problem exposed the weakness of a generic uppercase width. `M` is wider than a normal capital letter, so `Me` looked too tight when `M` fell back to the default uppercase profile.

The profile table was expanded to cover common chemical label characters:

- `A-Z`
- `a-z`
- `0-9`
- common punctuation such as parentheses, brackets, signs, comma, slash, and bullet variants.

Wide and narrow glyphs now have explicit metrics. The viewer also prioritizes the same reference-family stack, currently including `TeX Gyre Heros`, so browser rendering and kernel geometry are closer.

### Optical Shapes

The kernel classifies glyph background geometry as:

- rectangle,
- ellipse,
- cut-corner rectangle.

Ellipse is used for rounded glyphs such as `C`, `G`, `O`, `Q`, `c`, `e`, `g`, `o`, `0`, `6`, `8`, and `9`.

Cut-corner rectangles are deliberately narrow in scope:

- `L`, `h`, `b`: cut top-right,
- `P`, `F`: cut bottom-right,
- `d`: cut top-left,
- `q`: cut bottom-left.

The viewer converts these cut-corner shapes into polygons using a `42%` corner cut relative to the smaller side of the glyph background box.

### Why the Cut Corners Matter

Without cut corners, characters such as `L`, `P`, `F`, `d`, and `q` reserve too much empty space. Bonds then retreat too far, which makes labels look detached from structures. The cut-corner shapes let bond retreat follow the visible optical mass more closely without over-generalizing to every diagonal or lowercase character.

### Validation

The glyph work is covered by:

- `cpp/chemcore_glyph_kernel/tests/glyph_kernel_smoke.cpp`
- `scripts/glyph_kernel_reference.py`
- `cpp/chemcore_glyph_kernel/tools/chemcore_glyph_svg_demo.cpp`
- generated preview assets under `docs/assets/viewer`
- viewer integration through `viewer/glyph_kernel_runtime.js`

The important conceptual validation is that the viewer no longer depends on browser `getBBox`, `getExtentOfChar`, or canvas scanning as the source of truth for chemical label geometry.

## Viewer Rendering

The viewer now handles a broad set of CDXML-derived visual behavior:

- molecule fragments,
- atom labels,
- group labels,
- bold/italic/subscript/superscript runs,
- label knockout and bond retreat,
- single, double, triple, dashed, wedge, and hashed wedge bonds,
- text objects, shapes, lines, arrows, and document layout.

### Wedge Geometry

Solid wedge rendering was refined heavily.

For unlabeled wide-end nodes, a solid wedge can deform when it touches normal bonds or offset double-bond main lines. This reproduces the ChemDraw-style filled contact area:

- single contact: wedge sides intersect the far side of the contacted bond line,
- double contact: wedge center line reaches the node and both sides extend to their respective contact lines,
- labeled wide ends are protected and do not use this contact deformation.

This prevents label-adjacent wedges such as `TsN` from being pulled into text.

### Double Bond Geometry

Double bond behavior was tuned to match ChemDraw more closely:

- side double bonds render as a main line plus an offset short line,
- the short line retreats proportionally to the main bond length,
- centered double bonds remain two equal-length parallel lines,
- adjacent side double bonds can join their inner short lines when the adjacent main bond lengths are comparable,
- unequal adjacent main bond lengths do not join, because their offset geometry scales differently and should not be forced to touch.

The latest rule is important: the "equal" condition refers to equal main bond lengths, not the `center` double-bond placement style.

## Web Editor Shell

The old viewer sidebar was removed and replaced with an editor-style UI:

- top toolbar with file, save, undo/redo, delete, clipboard, zoom, and fit controls,
- contextual second toolbar,
- left rail with primary modes: select, bond, text, shape, templates,
- full-canvas drawing area.

The second toolbar changes by mode:

- select: selection modes, alignment, distribution, flips,
- bond: single, double, triple, dashed, bold, wedge variants,
- text: font, size, color,
- shape: stroke, fill, style,
- templates: rings and benzene.

The UI is intentionally a shell. Editing behavior is being moved into Rust rather than kept in browser JavaScript.

## Rust Editing Engine

A new Rust workspace and core engine were added under `crates/chemcore-engine`.

The design decision was to treat Rust as the long-term cross-platform core. Web, Windows, and iPad should call the same engine for model mutation, hit testing, snapping, command behavior, and overlay geometry.

Implemented today:

- blank document creation,
- document model serialization,
- single bond tool,
- endpoint hover,
- fixed-length bond drawing,
- angle snapping,
- bond-center focus,
- single-bond click-to-double conversion,
- double-bond style cycling,
- selection of bonds and nodes,
- delete,
- undo/redo snapshot stack,
- WASM API for the web viewer.

The old JavaScript implementation of single-bond hit testing, snapping, and mutation was removed from the viewer path.

## Editor Interaction Details

Several interaction details were tuned:

- Empty click creates a horizontal bond.
- Empty drag creates a fixed-length bond with angle snapping.
- Endpoint click extends by the default 120-degree chemical angle.
- Endpoint drag previews a fixed-length snapped bond.
- Dragging to another endpoint locks the preview to that endpoint and reuses the existing node.
- Quick-click extension also reuses an existing endpoint if the computed default endpoint lands within endpoint hit range.
- Endpoint focus display radius is `4.5`, with a larger hit radius retained for usability.
- Bond-center focus renders as an `18 x 9` rectangle.
- Clicking a bond center in single-bond mode cycles:
  - side double,
  - centered double,
  - opposite side double.

These changes fixed duplicated carbon nodes during ring closure and made aromatic ring editing behave more like a chemical drawing tool.

## Verification

Commands used during the session include:

```bash
npm test
npm run build:engine-wasm
npm run build:glyph-wasm
node --check viewer/app.js
cargo test
python3 -m py_compile src/chemcore/convert/cdxml_to_document.py
```

Browser-level checks were also run with Playwright to verify:

- pointer coordinate mapping,
- endpoint hover,
- bond-center hover,
- single bond drawing,
- drag-to-endpoint snapping,
- click-to-endpoint snapping,
- double-bond style cycling,
- side-double short line retreat,
- adjacent side-double inner line joining.

## Commit Timeline

| Commit | Summary |
| --- | --- |
| `7eddb66` | Documented viewer rendering and initialized the repository. |
| `140b954` | Normalized readable `chemcore.json` fields. |
| `5c18034` | Built the first editor toolbar shell. |
| `5d958db` | Added contextual editor toolbars. |
| `a35b9e1` | Implemented basic JavaScript single-bond drawing. |
| `9901651` | Scaled editor drawing defaults. |
| `9767cbc` | Fixed visible bond size scaling. |
| `e38cc87` | Made endpoint focus visible. |
| `8076a35` | Started Rust engine migration. |
| `48cc3e5` | Added Rust selection and command history. |
| `d471a85` | Fixed SVG pointer coordinate mapping. |
| `be7e753` | Added bond-center double conversion. |
| `9df89c2` | Refined double-bond focus overlay. |
| `744b8d2` | Shrunk single-bond center focus. |
| `4c9daca` | Added double-bond style cycling from bond center. |
| `7ab6cc5` | Refined focus geometry and double rendering. |
| `f7a007f` | Tuned bond focus overlay sizes. |
| `a30f4b0` | Snapped dragged bonds to existing endpoints. |
| `d0cdff1` | Scaled side-double short-line inset. |
| `9b47a43` | Snapped click-created bonds to existing endpoints. |
| `5b530a7` | Joined adjacent side-double inner lines. |
| `c111fae` | Limited inner-line joins to comparable main bond lengths. |

## Remaining Risks and Next Steps

The project is now moving in the right direction, but the key risks are clear:

- The glyph kernel is deterministic geometry, not true font rasterization. It is good for current label clipping, but future native output may still require a deeper font pipeline.
- Editor logic should continue moving into Rust. Toolbar state and SVG rendering can remain in the web shell, but chemistry behavior should not drift back into JavaScript.
- The undo/redo stack is currently snapshot-based. It is acceptable for early development but should evolve toward explicit commands or transactions.
- More bond tools are still pending: triple, dashed, wedge, hashed wedge, text, shapes, and templates.
- Ring templates should be engine-native rather than hand-built in the viewer.
- The renderer should eventually expose a platform-neutral display list so Windows, Web, and iPad can share behavior and geometry.

The biggest architectural decision of the day is settled: `chemcore` should become one deterministic cross-platform chemistry core with thin platform shells, not separate implementations for Web and native apps.
