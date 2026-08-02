# Enhanced-stereo annotation layout

This document records the ChemDraw-compatible rules used for CDX/CDXML
enhanced-stereochemistry annotations. These are renderer rules, not corpus-case
exceptions.

## Field and object roles

- `ShowAtomEnhancedStereo`, `EnhancedStereoType`, and
  `EnhancedStereoGroupNum` carry atom semantics. Their official definitions
  and codecs are tracked as verified in
  `docs/cdx-cdxml-field-verification.zh-CN.md`.
- A visible `objecttag` named `enhancedstereo` supplies the displayed `abs`,
  `orN`, or `&N` annotation. When an eligible displayed atom has semantic
  fields but no visible tag, the renderer synthesizes the same annotation.
- A `graphic` with `SymbolType="Absolute"` is a fixed graphic symbol. It is
  not an atom annotation and never participates in automatic atom layout.
- Text imported from ChemDraw JS without an explicit `PositioningType` retains
  its authored absolute position. Desktop automatic object tags are reflowed
  from current topology when the document is opened or laid out.

## Automatic direction

For an atom whose enhanced-stereo annotation is automatic:

1. Collect the directions of all incident bonds and visible non-stereo object
   tags attached to that atom.
2. Sort those directions around the atom and form the intervening angular
   gaps.
3. Select a largest gap. Gaps whose widths differ by no more than 3 degrees
   are tied because CDXML coordinates quantize nominally equal bond angles.
4. If a tied gap contains the direction opposite an incident wedge bond,
   select that gap. Otherwise prefer the gap whose centre is closest to the
   positive x direction, then the upward direction, then numeric angle order.
5. Place the annotation along the selected gap centre. The centre-to-centre
   distance is the annotation half-extent in that direction plus the document
   `MarginWidth`; the annotation extent comes from the active font metrics,
   not a fixed character-count estimate.

This is a topology rule. A cached tag bounding box can be used as authored
geometry only for an explicitly absolute tag; it cannot override automatic
layout.

## Visibility and lifecycle

- Automatic annotations are generated only for atoms directly owned by a
  displayed fragment. Atoms inside a collapsed nickname or fragment definition
  must not leak annotations onto the page.
- Import, topology edits, label edits, and export use the same resolved
  placement. Opening a text editor may temporarily expose editable text, but
  completing the edit immediately restores automatic placement unless the
  object was explicitly converted to absolute positioning.
- The shared rule applies to CDX, CDXML, SVG, EMF, and the native scene model;
  exporters do not apply independent offsets.

## ChemDraw evidence

ChemDraw Save As probes in `tmp/enhanced-stereo-regression-saveas` and
`tmp/enhanced-stereo-beta-plain-saveas` were generated from the public cases
and controlled variants. In the beta-cypermethrin probe, ChemDraw wrote the
three automatic tag baselines at `(228.66, 386.30)`, `(261.52, 395.00)`, and
`(334.88, 381.47)` points. The shared topology implementation renders them at
`(228.66, 386.22)`, `(261.61, 394.94)`, and `(334.91, 381.47)` points,
respectively. The maximum observed coordinate difference is 0.09 points.

The public regression set also covers:

- three beta-cypermethrin variants with `abs`, `or`, and `and` groups;
- automatic labels adjacent to solid and hashed wedge bonds;
- three simultaneous `abs`, `or1`, and `&1` labels;
- hidden enhanced-stereo data inside a collapsed fragment definition.

Regression tests live in
`crates/chemsema-engine/tests/render_document/query_double_bonds.rs` and
`crates/chemsema-engine/tests/render_document.rs`. The public visual gate then
checks the same rule against retained ChemDraw SVG oracles without file-name,
object-id, or coordinate-specific branches.
