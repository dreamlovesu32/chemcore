# Chemcore Developer Log - 2026-04-28

Author: Jiajun Zhang

Time range: 2026-04-28 00:00 to 2026-04-28 23:59, Asia/Shanghai

## Summary

Today was not a single bug-fix pass. It was a broad cleanup of the editor’s unit system, geometry boundaries, edit-mode behavior, selection logic, delete logic, and rendering ownership. The real work followed five main tracks:

- internal geometry continued to converge on `cm`, while a set of legacy `px` constants, render offsets, and defaults were moved toward shared constant sources and typed-unit boundaries;
- default styles and interaction geometry were reset toward ChemDraw-style values, including bond length, line width, text size, label size, focus boxes, focus circles, bold/hashed bond width, and wedge width;
- text editing moved further away from browser DOM geometry and back into engine-owned layout, fixing font scaling, frame drift, click-position mismatch, black-box mismatch, and edit-mode stretch/shift problems;
- the oversized text-edit, delete, and select logic inside `engine.rs` started to split into separate modules, while the viewer continued to shrink back toward “input events + world-coordinate conversion + primitive rendering”;
- selection and deletion were fully wired through, and bond selection boxes were changed to use the full rendered outline of a bond instead of only the center line.

The most important outcome is not that one visual issue now “looks right.” It is that geometry truth kept moving out of browser measurements and frontend compensation and back into shared engine and render logic.

## Unit System Continued to Converge on cm

The clearest decision at the start of the day was to keep pushing internal geometry toward `cm` and away from a hybrid model where the engine thinks in cm and the frontend keeps guessing in px.

### Why cm Remains the Core Unit

The reasoning is concrete:

- ChemDraw users already think in terms of physical-looking sizes;
- future multi-platform synchronization becomes much harder if browser and desktop editing do not share one internal geometry model;
- bond length, line width, font size, double-bond spacing, and focus geometry are fundamentally closer to physical dimensions than to screen pixels.

The goal is not to delete every px reference immediately. The goal is to keep px at the viewer boundary and in the few places where CSS-device-pixel interaction is genuinely the correct layer.

### Typed Units and Explicit Boundary Conversion

Today also pushed further toward a safer model where `CssPx` and `WorldCm` are treated as meaningfully different quantities.

The long-term goal is not to rely on engineers remembering that a specific `f64` “should already be cm.” The goal is to make unit conversion explicit at boundary functions so px leaks into kernel geometry are exposed earlier.

That direction is already visible in the code:

- `render_constants.rs` contains a growing set of `WorldCm` typed constants;
- some old high-frequency `px_to_cm(...)` calls in rendering now pull from typed constant sources instead;
- viewer/engine boundaries are moving toward explicit conversion points rather than passing bare floating-point values everywhere.

## Default Styles and Geometry Parameters Were Reset

This round also continued the ChemDraw-style parameter reset that the user explicitly confirmed.

### Default Values

The currently accepted baseline values are:

- default bond length: `1.058 cm`
- default bond stroke width: `0.035 cm`
- default text font size: `0.2645833 cm`, equivalent to `10 pt`
- default atom-label font size: `0.2645833 cm`, equivalent to `10 pt`
- default font family: `Arial`

These are not just toolbar display values. They are being pushed toward document defaults, text-edit session defaults, and generated-label defaults together.

### Double-Bond and Multi-Bond Geometry

Several multi-bond rules were tightened today:

- double-bond spacing still uses `bond_length * 0.12` as the base;
- but that value is now interpreted as the inner gap between the two visible lines, not simply centerline spacing;
- so the true center-to-center distance must include the widths of the two lines;
- when the main line becomes bold, the spacing must be recomputed accordingly instead of remaining unchanged;
- triple bonds continue to use the same spacing baseline;
- bold and hashed bonds now share the same default width of `0.141 cm`;
- the wide end of a wedge bond is `1.5x` the bold-bond width;
- the narrow end of a wedge returns to the ordinary bond stroke width of `0.035 cm`.

The underlying goal is consistent: multi-line and wide-line bonds should scale from one coherent set of geometry rules instead of carrying unrelated ratios.

### Focus and Hover Geometry

The interaction geometry was also explicitly reset:

- focus-box width: `0.2 cm`
- focus-box length: `0.8 cm`
- endpoint focus-circle radius: `0.1 cm`

This was not only a numeric tweak. It was another step away from “close enough px sizes” and back toward the same cm-driven geometry system used elsewhere.

## DPI, Browser Zoom, and Viewer Boundary Logic

The user also raised a necessary concern: DPI must not be hard-coded. The browser viewer should respect the user’s real environment, such as `150%` scaling with an effective `144 dpi`-like display context.

### Decision

The response was not to hard-code `144 dpi` as a new fixed value. The boundary was clarified instead:

- engine geometry still remains entirely in `cm`;
- the viewer is responsible for reading the real browser/device scaling context;
- `cm -> CSS px` conversion should depend on the current environment instead of assuming a fixed `96 dpi`.

That separation matters because:

- document geometry must not be tied to one machine’s DPI;
- browser display cannot ignore the user’s real scaling environment;
- so the document and engine stay stable while the display layer adapts.

The regression path also kept a `DEVICE_SCALE_FACTOR=1.5` replay to simulate the 150% scaling case the user explicitly called out.

## Shared Constant Sources and Render-Side Constant Cleanup

The user made a strong and correct suggestion: constants should be centralized, and future maintenance should avoid hunting down near-duplicate values across multiple files.

Today this translated into two directions:

- more high-frequency render-side `px_to_cm(...)` constants are being pushed toward `render_constants.rs` and typed constant sources;
- remaining high-risk constants in `render.rs` and `render_bonds.rs` were explicitly identified for continued cleanup, especially wedge dimensions, hashed-bond spacing, arrows, label clip margins, and bond offsets.

It was not realistic to eliminate every old constant in one pass, but the direction is now clear: geometry constants need one source of truth rather than two or three “almost identical” values spread across the engine and viewer.

## Text Editing Continued to Move Into the Engine

Most of the day still went into text editing, because unit migration exposed a long chain of browser-versus-engine mismatches.

### Symptoms That Were Actually Traced

The problems investigated today included:

- text stretching slightly when entering edit mode;
- text shifting downward slightly when entering edit mode;
- toolbar font-size displays drifting into values like `9.8` or `9.999`;
- the black edit frame failing to cover the visible text;
- fixing the frame and then reintroducing visible text scaling;
- mismatch between click position, visible I-beam caret, and actual text-box origin;
- cases where typing did not display text correctly at all or the whole edit layer drifted visibly.

The user’s architectural objection was correct: if the frontend is only the display layer, this class of bug should not be solved by endlessly patching viewer coordinates.

### Moving From DOM Geometry Back to Kernel Layout

The eventual direction was not to keep repairing `getBBox()`, `renderOffset`, or DOM `measuredSize` wrappers. It was to continue pushing edit-state geometry into the engine:

- line layout, anchor position, bbox, caret, and selection rectangles increasingly come from kernel-side layout;
- the viewer `textarea` is reduced to keyboard and IME input;
- visible text is rendered through SVG `text/tspan`;
- the black frame, caret, and selection rectangles follow kernel layout output rather than DOM measurement feedback.

This is what makes “what you see while editing should still be what you see after commit” a realistic engineering target instead of a fragile browser-dependent approximation.

### Edit Anchors and Text-Box Origins

Today also clarified the semantics of where an edit box should start.

The user’s requested behavior was explicit:  
the top-left corner of the I-beam caret is a more reasonable origin than a point near the middle of a glyph.

That pushed edit-mode anchoring further toward this model:

- the click position determines the logical start;
- the edit-box origin is much closer to the top-left of the I-beam;
- the visible editing layer and committed text position remain aligned;
- reopening an edit session prefers a stable anchor instead of drifting based on browser measurement.

### `measuredSize` Continued to Leave the Protocol Boundary

Another structural change was the continued removal of `measuredSize`-style DOM geometry from the engine boundary.

The viewer should not be allowed to send a browser-measured text block back to the engine as geometry truth. As long as that feedback path exists, the cm-based geometry model remains contaminated by browser-private layout behavior.

## Text Toolbar and Default Font Behavior

The text toolbar was also brought closer to the intended default behavior:

- font size should no longer sit in obviously wrong values like `9.999`;
- font size selection was turned into a dropdown with explicit options;
- the default size is `10`;
- the default font is explicitly `Arial`;
- these defaults should already be true before the user types, not only after the first edit event.

This looks like UI polish on the surface, but it is actually another piece of keeping default edit sessions consistent with document-level defaults.

## Bond Tool, Label Focus, and Direction Rules

Several smaller but important bond-tool rules were also tightened today.

### Character-Level Label Focus in Bond Mode

In text mode, plain text boxes still focus as whole objects.  
In bond mode, however:

- plain text boxes do not participate in focus;
- attached labels focus at the character level;
- every character can serve as a drag-out anchor for a new bond;
- vertical drags may keep the current character center;
- leftward drags snap to the leftmost character;
- rightward drags usually snap to the first uppercase letter when traversing from right to left;
- but if the user explicitly starts from the rightmost character and drags rightward, that exact rightmost character remains the anchor.

These rules were moved into engine-side logic and tests rather than remaining viewer-only hover guesses.

### Hover Should Not Jump to the New Endpoint After Drawing

The user also pointed out a bond-drawing interaction bug: after drawing a bond, hover jumped to the new endpoint instead of remaining at the actual pointer location.

That was corrected by no longer forcing `hover_endpoint` to the newly created endpoint at draw completion. Instead, bond-mode hit testing is recomputed from the real pointer location.

That restores the correct rule: where the mouse is is where the focus logic should be.

### Default Double-Bond Direction

The default double-bond rule was also clarified more precisely:

- only when one side already has two or more substituents and the opposite side has none should the first result be an equal-length double bond;
- in other cases the default should remain a side double;
- the orientation decision belongs to the existing structural logic rather than to a display-only layer.

## Delete Mode Was Fully Wired Through

Deletion finally became a complete interaction path today.

### Delete Tool

The delete entry point was unified as a mode rather than becoming a second parallel icon path.  
In delete mode:

- the cursor enters deletion state;
- clicking a single bond removes the entire bond;
- clicking a double bond first degrades it to a single bond;
- clicking a triple bond first degrades it to a side double;
- clicking an endpoint removes all bonds connected to that endpoint;
- clicking a label removes the label itself while preserving the endpoint;
- clicking a plain text object removes that text object.

### Delete Key Outside Delete Mode

Pressing `Delete` in other modes now follows this model:

- if a bond is focused, the whole bond is removed regardless of order;
- if an endpoint, label, or text box is focused, it follows that target’s own delete semantics.

## Select Mode Was Fully Wired Through

Selection today went beyond “box selection exists.” The object model, display rules, and molecule-internal aggregation logic were all tightened.

### Selection Methods

The current selection methods are:

- point selection,
- rectangle selection,
- lasso selection,
- additive multi-selection with `Shift`.

### Molecule-Internal Display Rules

The final user-confirmed rule set is:

- if exactly one object is selected inside a molecule, show that object’s own box plus a center blue dot;
- if multiple objects are selected inside the same molecule, stop drawing all the small boxes and instead draw one minimal outer box;
- the internal bonds, atoms, and labels keep only center blue dots;
- different molecules compute their own minimal boxes independently.

The per-object small-box definitions were also made explicit:

- atom: endpoint square;
- label: label text box;
- bond: an axis-aligned rectangle that tightly covers the entire visible bond outline.

### Bond Selection Boxes Now Use Real Rendered Outlines

The final fix of the day came from the user’s direct complaint that “covering the whole bond” must include double and triple bonds, not only the main line.

The solution was not to add even more spacing heuristics to `select.rs`. It was to switch selection geometry to real render output:

- `render.rs` now exposes `fragment_bond_visual_bounds(...)`;
- it reuses existing bond rendering to generate the actual `DocumentBond` primitives for one bond;
- it computes a true axis-aligned bounding box from line / polygon / polyline output;
- `select.rs` simply consumes that result.

That means:

- double-bond selection boxes cover both lines;
- triple-bond selection boxes cover all three lines;
- bold and wedge bonds are naturally enclosed as well;
- future rendering-constant changes automatically flow into selection geometry.

## `engine.rs` Split and Continued Viewer Reduction

Another important maintenance improvement today was the first real split of `engine.rs`.

### Rust-Side Split

The extracted modules were:

- `crates/chemcore-engine/src/engine/text_edit.rs`
- `crates/chemcore-engine/src/engine/delete.rs`
- `crates/chemcore-engine/src/engine/select.rs`

The practical gain is immediate:

- text editing, deletion, and selection no longer compete inside one several-thousand-line file;
- future debugging can target one module directly instead of scanning the whole engine file.

### Viewer Keeps Returning to a Display Role

The viewer-side direction remained consistent:

- handle keyboard and IME input;
- convert screen coordinates into world coordinates;
- consume wasm-exposed state and primitives;
- render the SVG / HTML surface.

Old browser-geometry scaffolding continued to be removed, especially the layers that tried to redefine text-box dimensions, label anchors, or bond geometry outside the engine.

## Verification

The main verification steps for today included:

```bash
cargo test -p chemcore-engine --test bond_tool --test text_tool
npm run build:engine-wasm
node --check viewer/app.js
node --check viewer/text_editor_render.js
```

The main coverage areas were:

- text-edit sessions, reopen behavior, and stable anchors;
- label and attached-label editing geometry;
- delete tool behavior and `Delete`-key behavior;
- character-level label focus and bond dragging;
- selection-box regressions for single, double, triple, and bold bonds;
- and consistency between regenerated wasm output and viewer bindings.

High-scale replay coverage was also kept in place to simulate user environments such as 150% display scaling.

## Risks and Next Steps

Even after this round, several follow-ups remain worth doing:

- continue moving the remaining high-frequency `px_to_cm(...)` constants into typed constant sources, especially for wedges, hashed bonds, arrows, and label clip margins;
- text editing is already much less dependent on DOM geometry, but caret, selection, and IME boundaries can still move further toward engine-driven results;
- selection boxes now use rendered bounds, and the same “rendering is truth” rule should be reused for more complex future selectable objects;
- unit boundaries between viewer and engine can still become more explicit so that `CssPx -> WorldCm` conversion happens only at a small number of known entry points.

The simplest summary of today is this:  
instead of adding more frontend coordinate patches, the work kept pulling units, geometry, text editing, and selection/deletion behavior back into a more stable shared engine-and-render model.
