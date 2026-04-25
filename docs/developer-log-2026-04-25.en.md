# Chemcore Developer Log - 2026-04-25

Author: Jiajun Zhang

Time range: 2026-04-25 00:47 to 2026-04-25 23:59, Asia/Shanghai

## Summary

Today’s work shifted from bond-contact geometry into editor behavior itself. The main result was not another pile of isolated fixes, but a much more coherent rule set for hover atom replacement, generated label geometry, 15-degree snapping, automatic double-bond placement, freeze semantics, and the meaning of `Left` and `Right` across editing and rendering.

The core problem was consistency. A double bond could be stored as one placement while being drawn on the opposite side. A newly created double bond could freeze too early and stop reacting to later substituents. A newly generated atom label could render text correctly but still break retreat, knockout, or label anchoring because its geometry was incomplete. By the end of the session those behaviors were aligned into one predictable system.

## Ketcher Shortcut Research

The first step was to verify how Ketcher handles keyboard replacement while hovering an endpoint in bond-drawing mode. The target interaction was explicit:

- hover an atom endpoint,
- do not click,
- type a key,
- and let the atom change into an element or abbreviation directly.

That research produced a compact Chemcore rule document rather than a copy of Ketcher internals. The current baseline rules are:

- `p -> Ph`
- `P -> P`
- `m -> Me`
- `c -> clear back to default carbon`

They are recorded in:

- `docs/ketcher-hover-atom-hotkeys.zh-CN.md`

## Hover Endpoint Replacement

After fixing the intended interaction, Chemcore gained direct keyboard replacement for hovered endpoints in bond mode.

This was wired through the actual engine path rather than through a frontend-only overlay:

- the viewer intercepts keyboard input while an endpoint is hovered,
- wasm exposes a replacement API,
- the Rust engine updates the hovered node,
- and element replacement, abbreviation replacement, and reset-to-carbon each use explicit state updates.

The important detail is that “clear to carbon” is not just deleting visible text. It removes the label state and returns the node to ordinary default carbon semantics, so later serialization and editing behavior stay clean.

## Generated Label Geometry

Once hover replacement worked, a second issue appeared immediately: the new label text rendered, but retreat, knockout, and centered-double label handling degraded.

The cause was straightforward: the generated labels had text content but not full geometry. Existing rendering logic depends on geometry fields such as:

- `label.position`
- `label.box`
- `label.glyphPolygons`

Today the engine was updated so generated labels receive geometry up front, and hover-replaced labels also refresh their geometry afterwards. That allowed the existing retreat and clipping logic to keep working instead of forcing a separate “quick label” render path.

## Label Font Size

Generated abbreviation labels also looked too small next to ordinary structure labels. Their default font size was raised in two steps and settled at `15`.

This is not only a cosmetic tweak:

- font size changes the label box,
- the label box changes retreat and knockout behavior,
- so the font default and the geometry defaults have to stay synchronized.

## 15-Degree Snapping

Angle snapping was expanded to a full `15°` lattice.

This changed the editor snapping rules themselves rather than adding a frontend helper:

- global snap angles now cover `0°..345°` in `15°` increments,
- relative bond angles also use `15°` increments.

That makes horizontal, vertical, `30°`, `45°`, `60°`, and `75°` structures all snap naturally, and it keeps click-extension and drag-extension on the same angular grid.

## Double Bonds and Labels

Another large part of the session was about making double bonds and labels behave by clear rules.

Two visible bugs were involved:

- label retreat was shrinking the apparent spacing between double-bond lines,
- and some centered doubles looked non-parallel when one endpoint carried a label.

These were not the same bug.

### Label Retreat Must Not Change Real Bond Length

The user requirement was explicit: except for wedge-family behavior, label retreat should not redefine the real bond length. It only shortens the visible segment near the label.

The engine and render path were adjusted so:

- double and triple offset scaling use real node-to-node length,
- side insets also use the true bond length,
- label clipping affects only visible retreat, not bond-spacing calculations.

That removed the incorrect “compressed” look of labeled side doubles.

### Parallel Centered Doubles

The non-parallel centered-double bug had a different cause. The center line had already been retreated against the label once, but the two child lines were still generating some endpoint profiles and offsets from a mismatched basis. That let the two visible lines attach to slightly different parallel references.

The fix was to unify the offset basis used for:

- centered-double child-line rendering,
- endpoint profile generation,
- and label-end clipping behavior.

After that, the two centered-double lines remained genuinely parallel again.

### Terminal Double-Bond Label Anchoring

A separate rule was also clarified:

- at a terminal side or centered double bond with no further substituent, the label should sit between the two visible lines,
- once that endpoint gains another substituent, anchoring should fall back to the ordinary main-bond logic.

That behavior was folded into the generated-label geometry refresh path.

## Automatic Double-Bond Placement and Freezing

The trickiest editor-side work today was the automatic placement and freezing model for double bonds.

### Initial Default Style

When a bond becomes a double bond for the first time in an ambiguous multi-connection case, the default should be `center double`. That rule was applied both to:

- converting an existing bond by clicking it with the double-bond tool,
- directly drawing a new double bond,
- and creating a dashed double bond for the first time.

### Automatic Repositioning

For unfrozen double bonds, placement now follows substituent distribution:

- when a third bond is added, the double bond moves to the more substituted side,
- when a fourth bond creates a tie, it moves to the side of the most recently drawn bond,
- for a mono-substituted double bond, adding a cis substituent on the other end moves the double to the inner side.

The implementation was kept as one side-counting framework rather than many overlapping special cases.

### Freeze Semantics

The user clarified the intended freeze model:

- a newly born double bond is not frozen,
- that includes bonds converted from single, triple, wedge, or dashed styles,
- and also includes directly drawn double or dashed-double bonds,
- a double bond becomes frozen only after the user clicks an already-existing double bond to manually change its style.

That required adding an explicit `frozen` field to `DoubleBond` and tightening all creation and cycling paths so “first creation” and “manual restyling” are no longer confused with each other.

## Final Left/Right Alignment

The last major correction of the day was about what “inner side” really means for slanted double bonds.

The editor logic had one meaning for `DoubleBondPlacement::Left` and `Right`, but the renderer and generated label anchoring were still drawing those placements on the opposite side. That is why some horizontal cases looked acceptable while a slanted bond immediately exposed the mismatch.

The final fix was to align all three layers:

- placement calculation in the editor,
- side selection in the renderer,
- label offset direction for generated labels on side doubles.

After that, a stored placement and a drawn placement finally referred to the same visible side.

## Verification

The main closing verification steps were:

```bash
cargo test -p chemcore-engine
npm run build:engine-wasm
```

The regression coverage now includes:

- hover endpoint replacement,
- 15-degree angle snapping,
- labeled side-double and centered-double geometry,
- unfrozen first-created double and dashed-double bonds,
- third-bond and fourth-bond automatic side movement,
- cis mono-substituted double-bond movement to the inner side,
- and freeze behavior after manual style changes.

## Final Result

By the end of the day the editor behavior had settled into a much more coherent shape:

- hovered endpoints can be changed directly from the keyboard,
- generated labels carry full geometry,
- new labels match surrounding chemistry labels better,
- bond drawing snaps on a complete `15°` grid,
- label retreat no longer distorts double-bond spacing,
- centered doubles remain parallel near labels,
- newly created double bonds stay unfrozen,
- manually restyled double bonds freeze,
- unfrozen doubles reposition automatically as substituents are added,
- and `Left`/`Right` now mean the same thing in state and on screen.
