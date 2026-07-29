# ChemSema Bond Rendering Rules

This document records the bond rendering and bond-contact rules currently used by the Rust rendering kernel. It fixes geometry definitions, implementation boundaries, and refactor baselines.

In this document, "should", "must", and "must not" are normative requirements. "May" and "allowed" describe implementation flexibility.

## Goals

- Bond rendering should be defined uniformly by the Rust kernel and should not rely on secondary viewer patches.
- Similar bond types should share geometry models as much as possible, reducing local heuristics split by bond type.
- Node contact should be determined first by contours and endpoint profiles, avoiding accidental shapes introduced by SVG `linecap` or foreground styles.
- Preview state and committed state should reuse the same geometry.

## Scope

- This document constrains only the fragment path.
- The `legacy molblock` path only needs compatible output; it does not need to satisfy each new geometry rule here.
- This document describes 2D geometry output. Foreground interaction state, hover decoration, and selection-box styling are constrained by interaction-layer documents.

## Terms

- Main bond: the principal-axis body that determines bond contact and node occupancy.
- Secondary line: the offset subsidiary line in one-sided double bonds and triple bonds.
- Body: the complete polygon before knockout splits it into segments.
- Endpoint profile: the contour chain actually consumed by one bond at one endpoint; it closes the final polygon.
- Keep as-is: keep the standard body outline of the bond and do not alter the cap contour for contact.
- Retreat: do not change outline topology; only shorten or move backward along the axis so the bond does not press into the contact object.
- Same side: relative to the node local normal, the secondary line or outer line is on the same side as the current reference line.
- Straight-through: two axes are approximately `180°` opposite and collinear.

## Overall Classification

Current bonds are divided into three classes:

1. Objects with a main bond.
2. Objects without a main bond but with a fixed body shape.
3. Legacy molblock path.

This document discusses only the first two classes in the fragment path.

### Objects With A Main Bond

- ordinary single bond
- bold solid bond
- ordinary dashed bond
- hash bond
- one-sided double bond
- centered double bond
- triple bond
- solid wedge bond
- hashed wedge bond

### Unified Geometry Vocabulary

- All solid bonds are treated as polygons.
- Ordinary single bonds and bold solid bonds are geometrically the same except for width.
- Solid wedge bonds are treated as trapezoids. The narrow end is a very short edge.
- Ordinary dashed bonds are "body polygon + white knockout segments".
- Hash bonds are "bold solid body rectangle + denser white knockout segments".
- Hashed wedge bonds are "solid wedge body trapezoid + white knockout segments".

## ChemDraw Drawing Template Parameters

Default and ACS Document 1996 use two independent sets of template parameters. When switching templates, existing bonds and future new bonds should both switch to the corresponding template parameters.

Key differences currently aligned with ChemDraw:

| Parameter | Default | ACS Document 1996 | Notes |
| --- | ---: | ---: | --- |
| Bond length | `30.0pt` | `14.4pt` | Existing structure geometry is scaled by this ratio when switching templates |
| Ordinary line width | `1.0pt` | `0.6pt` | Used for ordinary bonds, narrow wedge ends, and shape line width |
| Bold bond width | `4.0pt` | `2.0pt` | Template width for bold solid bonds |
| Solid/hollow wedge wide end | `6.0pt` | `3.0pt` | `1.5 * BoldWidth`; hollow wedges share the solid wedge outline |
| Label clip natural outset | source `MarginWidth` | source `MarginWidth` | Absolute pt value from the source template; it does not scale with label font size |
| Label clip circle radius | `2 * MarginWidth` | `2 * MarginWidth` | Absolute pt value derived from the source template margin |
| Hash spacing | `2.7pt` | `2.5pt` | Template spacing for the hash family |
| Bond spacing | `12%` | `18%` | Double-bond spacing percentage used with the actual bond length and the line-width floor |
| Margin width | `2.0pt` | `1.6pt` | White margin around upper bonds at non-endpoint bond-bond crossings |

Wedge wide-end width, label clip natural outset, label clip circle radius, and margin width are template parameters and should not be inferred backward from bond length. CDXML import derives the solid/hollow wedge wide end as `1.5 * BoldWidth` from the source template and uses the source label clipping numbers directly; do not infer an "ACS" label clipping mode from `MarginWidth`.

For CDXML imports, label clipping geometry is parameterized by the source `MarginWidth`: natural outset is exactly `MarginWidth`, and circular anchor radius is exactly `2 * MarginWidth`. These values are absolute points and must not be multiplied by the label font size or glyph height.

## Unified Decision Order

Each bond endpoint should compute final geometry in this order:

1. Determine the standard body shape of the bond.
2. If the node has a label, perform label clipping first.
3. Decide whether this endpoint enters node-contact rules.
4. If it enters contact, choose by bond type:
   - main-bond intersection
   - secondary-line intersection
   - retreat only
   - keep as-is
5. Apply dash/hash/knockout to the final body.
6. Preview state and committed state must use the same flow.

No implementation may treat viewer stroke, line join, or line cap as part of the geometry definition.

## Main-Bond Contact

"Main-bond contact" means the contact region is determined by the main-bond contour.

### Two-Bond Contact

This applies to objects "with a main bond": ordinary single bonds, bold solid bonds, ordinary dashed bonds, hash bonds, one-sided double-bond main lines, triple-bond main lines, solid wedge axes, hashed wedge axes, and similar cases.

Rules:

1. The two bonds share a center point at the node.
2. Each bond has two main-bond contour lines at that endpoint.
3. Compute the `inner-inner` intersection and the `outer-outer` intersection between the two bonds.
4. These two intersections form the contact-region boundary.
5. Each bond consumes its own endpoint profile and directly generates its own polygon, without extra mask patches.

Additional rules:

- Intersections may use extended lines; the intersection does not need to lie within the original segment length.
- ChemDraw's acute-angle miter limit is relative to usable bond length, not a fixed angle or a fixed stroke-width multiple. For every participating contour, the absolute intersection projection must satisfy `|projection| <= 0.235 * extent`. The conservative `0.235` threshold is measured from real ChemDraw output; the observed transition lies around `0.235–0.237`.
- If either contour exceeds the limit, that pair must not form a long spike. Each bond uses its own contour base at the node to form a bevel. Do not clamp the intersection to an arbitrary fixed distance and do not force both contour bases through a shared midpoint.
- For equal-width bonds meeting at angle `phi`, the theoretical axial miter length is `m = halfWidth / tan(phi / 2)` and remains subject to the relative-length limit above.

### Three-Or-More-Bond Contact

Rules:

1. Sort incident bonds around the node.
2. Process only adjacent pairs, not all combinations.
3. Intersect the inner contour lines of every adjacent pair to form a ring of points around the node.
4. Each bond consumes its own endpoint contour chain.
5. In a triple-bond node, a solid bond may naturally become a pentagon; with more bonds, the vertex count continues to grow.

Additional rules:

- Multi-bond nodes process only adjacent pairs, not all combinations.
- Intersections from non-adjacent pairs must not be used for center-node topology.
- If a bond class is defined as "keep as-is", that bond does not participate in center-node occupancy; other bonds retreat.

### Straight-Through Exception

- `180°` straight-through cases do not require intersections.
- Centered double bonds do not extend when the included angle is greater than `162°`.

## Single Bonds And Bold Solid Bonds

- Ordinary single bonds and bold solid bonds both render as quadrilaterals.
- They differ only by width parameters.
- In contact, both are main-bond contact objects.

## Dative Bonds

- CDX/CDXML `Order="dative"` uses an ordinary-width shaft and a solid concave-tail arrowhead; it does not use wedge-bond geometry.
- For clipped bond length `L` and ordinary line width `LW`, the head length is exactly `min(10 * LW, 2L / 3)`.
- The head base half-width is one quarter of the head length. The concave notch is seven eighths of the head length behind the tip.
- The shaft must end at the notch. It must not continue to the tip or enter ordinary bond contact at the acceptor end; the donor end still follows ordinary main-bond contact rules.
- This geometry depends only on `LineWidth` and visible bond length, not on `BoldWidth`, font settings, or nominal `BondLength`. It comes from silent ChemDraw SVG measurements across directions, `5–100pt` bond lengths, and four `LineWidth/BoldWidth` combinations.

## Ordinary Dashed Bonds

- The body of an ordinary dashed bond is still a standard rectangular main bond.
- White cut segments are placed at equal intervals along the main axis.
- Contact uses the body rectangle.
- Black segment length may vary according to dash rules; ordinary dashed bonds do not require strictly equal black segments like the hash family.
- When a dashed main line receives an endpoint contact profile, remove that
  profile's inward axial extent from the equal black/gap interval domain. The
  first and last black stripes absorb the contact miter; interior black and
  white intervals remain equal. Do not compute the dash rhythm on the original
  centerline and then paste a disconnected endpoint cap over it.
- `Display="Dash"` remains the losslessly preserved and exported source style.
  However, when and only when both endpoints of a single bond are
  `NodeType="Fragment"` nodes with a direct child `<fragment>`, ChemDraw draws
  the visible main line as solid. If either endpoint lacks that structure, the
  bond remains an ordinary dashed bond. `BS="N"` is the determined-symmetric
  CIP stereochemistry field, while `BeginAttach` and `EndAttach` are styled-text
  character attachment indices; none participates in this display exception.
  Silent ChemDraw A/B probes kept the solid result when changing the outer
  `Display`, internal connection style, attachment index, and completed
  connection mapping. Removing the direct child fragment from either endpoint
  consistently restored the same outer bond as 15 dashed segments.

## Hash Bonds

Hash bonds are a separate model: a bold solid body plus white cut segments.

- The body is always a bold solid rectangle.
- Black segment lengths must be equal.
- White segment lengths may vary within a range.
- The number of white segments changes only when the total length exceeds the range allowed by the current segment count.
- Black segment length has higher priority than equal spacing of white segments.

### Hash Bonds And Ordinary Main Bonds

- Keep the standard rectangle and do not actively deform it to meet other bonds.
- In a multi-bond node, other bonds retreat and the hash bond stays as-is.

### Hash Bonds And Centered Double Bonds

- Do not intersect and do not change the rectangular cap.
- Retreat only along the hash bond's own center line.
- The retreat reference is the correct-side outer line of the centered double bond.
- White cut segments follow the actual retreated rectangle, not the old length.

## Solid Wedge Bonds

- The body is a trapezoid.
- The wide-end width is fixed, and the narrow-end width is fixed as a very short top edge.
- In ordinary main-bond contact, the wide end may deform by contour intersection.
- If the wide end meets a label, solid wedge label clipping uses the centerline-clipped endpoint only. Do not add an extra wide-end margin retreat for solid wedges.
- When length changes, endpoint width definitions should be preserved first, then trapezoid side edges are updated.

## Hashed Wedge Bonds

- The body is a standard trapezoid.
- Black segments are arranged along the trapezoid main axis.
- Every black segment is exactly one `LineWidth` long along the main axis.
- The first and last black segments touch the two ends of the final trapezoid.
- `HashSpacing` is the minimum center-to-center pitch. ChemDraw chooses the
  greatest count that satisfies that minimum, then distributes the centers
  evenly between the endpoints:

  ```text
  count = max(1, 1 + floor((finalLength - LineWidth) / HashSpacing))
  pitch = (finalLength - LineWidth) / (count - 1)   when count > 1
  ```

- This rule is based on silent ChemDraw SVG measurements over Default, ACS,
  changed line widths, and changed hash spacings. Do not reuse the ordinary
  dashed-bond allocator or force a minimum of two stripes on short wedges.
- If either endpoint meets a label, label clipping is identical to solid wedge label clipping: use the centerline-clipped endpoint and do not add an extra wide-cap or hash-family retreat.

### EMF Bond Replay

ChemDraw replays ordinary bonds in dual EMF as pens, rather than preserving the
filled geometry used by the scene renderer. An ordinary bond uses `LineWidth`,
round start/end caps, a miter join with miter limit `2`, and a round dash cap.
The GDI compatibility record is a geometric pen with round end caps and a miter
join (`EXTCREATEPEN` style `73728`). A round line join is not equivalent.

A hashed wedge is also redrawn for EMF. Each black stripe becomes one
independent, perpendicular pen stroke with the same pen properties as an
ordinary bond; it is not emitted as a filled quadrilateral. For final wedge
length `L`, line width `LW`, bold width `BW`, and the stripe count above:

```text
s_i = LW / 2 + i * (L - LW) / (count - 1)
W(s) = LW + (1.5 * BW - LW) * s / L
```

`s_i` is the stripe-center position from the narrow endpoint and `W(s_i)` is
the transverse centerline length. This explains why the first and last EMF
centerlines are sampled half a `LineWidth` inward rather than using the literal
endpoint widths. If `count = 1`, the scene renderer still supplies the complete
short trapezoid, but direct EMF replay samples its only stroke at
`s = L - min(L, LW) / 2`, half a stripe inward from the wide end. Object
transform, rotation, and scale must be applied before recovering the bond axis
used for this conversion.

ChemDraw's Office/OLE presentation EMF is a separate output profile; it is not
the same byte stream or replay strategy as `SaveAs(.emf)`, even in the same
ChemDraw version. In the Office profile, a near-square narrow stripe (transverse
centerline no greater than `1.25 * LineWidth`) is replayed as a round pen along
the bond axis, with both pen width and centerline length equal to `LineWidth`.
This is the short axial mark that appears vertical on a near-vertical wedge in
PowerPoint. Wider stripes remain filled quadrilaterals. Standalone EMF export
must use the direct profile above; clipboard/OLE presentations and Office
preview media must use this Office profile.

The reusable local probe covers three style profiles, nine lengths (including
the one-stripe short-bond range), four directions, and both ordinary and
hashed-wedge bonds (216 ChemDraw EMFs):

```bash
node scripts/chemdraw-emf-bond-probe.mjs --verify-chemsema
```

The probe checks record type/count, pen flags and caps, normalized geometry,
and ChemDraw/ChemSema agreement without depending on the EMF frame size.

### Hashed Wedge Bonds And Ordinary Main Bonds

- Keep the standard trapezoid and do not actively intersect to reshape the wide end.
- In multi-bond nodes, prefer preserving its own body shape and let other bonds retreat.

### Hashed Wedge Bonds And Centered Double Bonds

- Do not intersect at the wide end.
- Retreat only along the hashed wedge's own axis.
- The wide-end width stays fixed, so the trapezoid naturally deforms after length changes.
- White cut segments must follow the actual retreated trapezoid.

## One-Sided Double Bonds

A one-sided double bond consists of one main bond and one secondary line. For CDXML-compatible spacing, `bondSpacing` is a percentage applied to the actual bond length, but ChemDraw also enforces a line-width floor. The inner gap is:

```text
max(actual bond length * bondSpacing / 100 - lineWidth, 1.5 * lineWidth)
```

The center distance is that inner gap plus half of each rendered line width. For two normal-weight lines, this simplifies to:

```text
max(actual bond length * bondSpacing / 100, 2.5 * lineWidth)
```

This is not a style-template branch: ACS, default, and custom CDXML documents use the same rule with their source `LineWidth`, `BondLength`, and `BondSpacing` values.

`bondSpacingAbsolute` / CDXML `BondSpacingAbs` is different: it is the exact
center-to-center distance in points, takes precedence over the percentage, and
does not receive the line-width floor. This remains true even when the authored
absolute distance is smaller than the line width.

### Automatic Side Selection

When CDXML or an editing operation gives only `Order="2"` and no explicit `DoublePosition`, the engine should decide automatically using ChemDraw-compatible rules:

1. If either end of the bond directly connects to another double bond, use centered double.
2. If neither end has other connected bonds, use centered double.
3. If one terminal end has substituent bonds on both left and right sides while the other end has no substituent bond, use centered double.
4. If the bond belongs to a ring, first choose the reference ring using ring-selection rules, then place the secondary line inside that reference ring:
   - a fully alternating six-membered ring has priority over shorter fused small rings
   - then compare complete alternation, alternation match count, closeness to six-membered rings, and path length in order
5. For non-ring structures, compute the signed projection sum of all connected substituent bonds along the left normal of the begin-to-end axis:
   - positive sum: place on the left side
   - negative sum: place on the right side
   - exact tie while adding a substituent: use the side of the newly added bond
   - exact tie without editing context: default to the right side

### Main Bond

- The main bond is an ordinary main-bond contact object.
- It contacts single bonds, bold solid bonds, triple bonds, wedges, and similar objects by the main-bond contact rules.

### Secondary Line

- The outer secondary line keeps its original shape by default and does not participate in ordinary contact.
- Only the inner secondary line participates in contact intersections.
- At a label-clipped endpoint, the main and secondary lines compute retreat on
  their own actual parallel axes with their own half-widths, then share the
  larger retreat. Do not derive both from an unshifted center axis.
- At terminal ends, it is the same length as the main bond.

## Triple Bonds

The center line lies on the bond axis. Each outer line is offset by the same
center-to-center distance used for two normal-weight multiple-bond lines:

```text
max(actual bond length * bondSpacing / 100, 2.5 * lineWidth)
```

The rule uses the bond's actual endpoint distance, not the document's nominal
`BondLength`, and is invariant under bond direction. If
`bondSpacingAbsolute` / CDXML `BondSpacingAbs` is present, each outer line is
offset by that exact absolute value and the line-width floor is not applied.
The reproducible ChemDraw SVG probe covers three lengths, three percentage
spacings, three line widths, horizontal/diagonal/vertical directions, and
absolute-spacing precedence:

```bash
node scripts/chemdraw-triple-bond-spacing-probe.mjs
```
- At non-terminal ends, it is shortened by "main-bond spacing * `sqrt(3) / 3`".
- When the angle between the main bond and the connected main bond is less than `90°`, the secondary line may retreat; however, if the other bond has a same-side secondary line, same-side secondary-line intersection still has priority.

### Acute-Angle Retreat

- If the main bond connects to a main bond and that side's angle is less than `90°`, the secondary line may retreat.
- If the other bond also has a same-side secondary line, same-side secondary lines still intersect directly.
- If the other secondary line is not on the same side, use retreat.
- If it meets a centered double bond, use the centered double's center line as the retreat reference.

## Centered Double Bonds

A centered double bond consists of two independent sub-lines.

Rules:

- Each sub-line evaluates its own endpoints independently.
- For label clipping, each sub-line computes endpoint retreat on its own
  offset axis using its own rendered half-width. At each endpoint, take the
  larger retreat required by the two sub-lines and apply that same retreat to
  both, so the final strokes remain equal in length.
- Do not compute centered-double label retreat on the unshifted center axis and
  then copy or translate that center-axis result to the two sub-lines.
- Each end and each side independently decides whether to extend to another bond's contour.
- Near-straight-through cases greater than `162°` do not extend.
- `180°` straight-through keeps the original shape.
- When contacting the hash family, the centered double bond is the reference object; it does not reverse-modify hash-family outlines.
- Applies to:
  - solid-solid centered double bonds
  - dashed-solid centered double bonds
  - dashed-dashed centered double bonds

## Triple Bonds

- The triple-bond main line is a main-bond contact object.
- The two outer lines independently evaluate their endpoint profiles.
- At terminal ends, outer lines remain full length.
- Similar to one-sided double-bond secondary lines, outer-line contact is determined by each line's own contour.

## Hash-Family Priority In Multi-Bond Nodes

If one node connects to multiple bonds and one of them is a `hash bond` or `hashed wedge`:

- `hash bond` / `hashed wedge` stays as-is.
- Other bonds retreat.
- A small white gap is intentionally left; complete seamless contact is not required.

## Decision Priority

If multiple rules hit the same endpoint, apply them in this order:

1. label clipping
2. hash-family keep-as-is rule
3. centered double near-straight-through no-extension rule
4. main-bond contact intersection
5. one-sided double secondary-line same-side intersection
6. one-sided double secondary-line acute-angle retreat
7. terminal equal-length / endpoint fixed-length rules
8. dash / hash knockout cuts

Later rules must not overturn geometry topology already determined by earlier rules; they may only continue tightening within remaining degrees of freedom.

## Labels And Clipping

- CDX/CDXML `BeginAttach` and `EndAttach` are authoritative when they identify
  an internal attachment atom/glyph in a structural node label. Import must
  decode them into `meta.endpointAttachments.<endpoint>` with semantic
  `target`, `characterIndex`, and `character` fields. Bond rendering resolves
  the endpoint from the active label glyph geometry instead of using the
  structural label's outer node position.
- If an endpoint attachment field is absent, out of range, or cannot be
  resolved against active glyph geometry, an existing bond endpoint stays on
  its structural node coordinate. That coordinate defines the main-bond axis;
  label clipping may retreat the visible line from it but must not move the
  axis to a nearby glyph.
- For a terminal label on a side double bond, the default attachment glyph is
  laid out on that structural-node/main-bond axis. The parallel secondary-line
  spacing must never shift the label by half the double-bond separation.
- An explicit fixed `LabelDisplay="Left"` or `"Right"` anchors the authored
  edge, not the nearest ordinary-baseline glyph. A leading/trailing subscript
  participates in that edge anchor. A superscript does not: search inward to
  the first/last non-superscript glyph. Automatic chemical layouts continue to
  exclude both generated subscripts and superscripts from attachment anchors.
- Export must write preserved endpoint attachments back to CDX/CDXML so an
  open-save-open cycle stabilizes. Applying an internal attachment must not
  move the opposite atom merely to make the bond axis look aligned.
- If a node has a visible label, perform label clipping first.
- Label clipping occurs before bond contact.
- Final bond geometry should satisfy both label clipping and node-contact rules.
- Endpoint label clipping is defined only by the glyph clip polygons generated
  from glyph geometry and source `MarginWidth`. Do not add a separate
  per-bond `labelClipMargin` or post-hoc retreat value.
- `MarginWidth` natural extension applies to every visible glyph edge,
  including internal stroke bays and stroke ends that remain inside the
  glyph's overall bounding box. Runtime mapping must not scale only the
  outside of the whole glyph bbox.
- For a same-row multi-glyph label, keep the outward half of the first and
  last glyph as real outline geometry. Rectangularize only their inward halves,
  and rectangularize each middle glyph using that glyph's own bounds.
- Bridge an internal gap only over the vertical overlap of the two adjacent
  glyphs. Never use a row-wide minimum or maximum Y: a low parenthesis or
  descender must not pull another glyph's clipping rectangle downward.
- Subscript and superscript glyphs do not join this same-row rectangular
  interior model.
- ChemDraw's narrow axial-contact sectors follow the writing axes. Bond rays
  within `10°` of the vertical axis use glyph-column ownership. Rays within
  `10°` of the horizontal axis use one shared left/right run envelope and that
  envelope replaces the general natural/feature outline kernel inside the
  sector. Diagonal contacts use the complete label outline.
  Horizontal glyph origins include the selected face's real kerning before
  the run envelope is built; `BeginAttach`/`EndAttach` choose the authored
  character used as the ray origin but do not change the envelope boundary.
  Top/bottom ownership is selected from the raw glyph intervals intersected by
  the bond center and body-edge rays; `MarginWidth` expands the selected
  glyph's clip geometry, but must not make a neighboring glyph steal the
  column. On the cap-facing side, the selected glyph's own top extent is
  authoritative. On the baseline-facing side, use the descent envelope of the
  complete run that shares the selected glyph's exact layout baseline. Shifted
  subscript and superscript runs remain separate. If a vertical ray lies in an
  internal kerning gap, assign the gap to the nearest glyph in the ray
  direction. The public
  `3060_iso_butane.cdxml` sample demonstrates the vertical branch: the bond
  under `CH3` stops at the `C` column (`y = 191.09pt` in ChemDraw), not at the
  lower edge of the remote subscript `3`. Public case `0137` demonstrates the
  horizontal branch repeatedly: fixed-display labels such as `(CH2CHO)m`
  continue to mask the bond through the complete run's X envelope. The
  same-baseline `T`/`Tyr` probe demonstrates the asymmetric vertical rule:
  their cap-facing endpoints coincide, while the baseline-facing endpoint of
  `Tyr` includes the `y` descender.
- For rendered bonds with thickness, the whole bond body must be clipped out of
  the glyph clip polygon. Ordinary, dashed, bold, and multi-line bonds should
  evaluate the center line plus both body boundary lines for the current visual
  half-width and use the largest retreat required by those glyph intersections.
- Parallel sub-lines of a double or triple bond share the retreat required by
  the complete label outline. The single-line vertical glyph-column branch
  must not be applied independently to those sub-lines; a shifted formula
  script can therefore determine their common endpoint.
- Label retreat is an unbounded ray operation, not a segment clamp. If the
  label exclusion boundary lies beyond the opposite atom because the authored
  bond is shorter than the label, keep the computed endpoint past that atom;
  the visible bond body is therefore reversed. Do not scale the two endpoint
  retreats back to the authored bond length. ChemDraw's bare-CDXML spacing
  probe remains linear through this crossover, and `NeedsClean` does not alter
  the result.
- The overrun rule above applies when only one endpoint exclusion boundary
  crosses the bond. If both endpoints have labels and their two exclusion
  distances meet or cross, ChemDraw collapses the bond body to zero width, so
  it contributes no visible ink. Single- and double-bond spacing probes both
  preserve this distinction.
- Apply endpoint label clipping exactly once. A body already produced from the
  unbounded endpoint rays must not be clipped again using its reversed endpoints.

## Bond-Bond Crossings And White Margins

Non-endpoint bond-bond crossings are intersections of two internal bond segments that do not share begin/end nodes. This rule covers bond-vs-bond only, not bond-vs-text, bond-vs-shape, or bond-vs-arrow.

Rules:

- CDX/CDXML `CrossingBonds` (CDX property `0x060E`, type `CDXObjectIDArray`) is authoritative crossing-pair semantics in document-global object-ID scope, including pairs whose bonds belong to different fragments. Import must preserve it and export must remap and write the new object IDs. If either bond in a pair has an explicit crossing list, no geometric crossing may be invented outside those lists; geometric fallback is allowed only when both bonds lack the property.
- Layering follows final paint order: compare CDXML `Z` first and document order within the same layer. The later-painted bond is the upper bond.
- When two ordinary, normal-width, solid single bonds cross, ChemDraw clips the lower bond-body polygon into two pieces at the cut interval instead of painting the complete bond and covering it with white. The engine must emit the same real segmented geometry so antialiased edges cannot leave black slivers.
- Composite, dashed, wavy, and wedge bonds use their own explicit local-background-knockout branches so the lower bond breaks only around the intersection. Never clone the whole upper bond with an enlarged background stroke; a whole-bond silhouette erases valid endpoint contacts and unrelated nearby geometry.
- White margin width uses the upper bond's template parameter: Default is `2.0pt`, ACS Document 1996 is `1.6pt`.
- Let `n = (-axis.y, axis.x)` be the upper bond's unit normal. Determine the upper bond's ChemDraw cut envelope `[c_min, c_max]` along `n` at the intersection, then expand that interval by the source margin. The knockout strip is exactly `[c_min - marginWidth, c_max + marginWidth]`; it is not required to be symmetric about the parent bond axis.
- A plain or bold filled line uses its visible body contour for `[c_min, c_max]`. A wedge uses its interpolated local contour. Composite line families instead use the envelope of their child centerlines: centered double and triple bonds use the outer child centers, and a side double uses the main and side-line centers. A wavy bond uses the extrema of its wave center path. Do not add the child stroke half-width again for these composite/path envelopes; the upper bond is repainted over the local knockout.
- Therefore a centered double with center distance `d` uses `[-d/2, d/2]`. A left side double uses `[0, d]`, and a right side double uses `[-d, 0]`, where left/right follow `n`. This asymmetric interval is required: symmetrizing a side double erases too much of the lower bond on the empty side.
- In the symmetric special case `[c_min, c_max] = [-h_over, h_over]`, with acute crossing angle `theta`, the lower bond's axial half-gap is:

  ```text
  gapHalf = (h_over + marginWidth) / abs(sin(theta))
  ```

- Both cut edges must be parallel to the upper bond. For ordinary solid single bonds, clip the lower polygon directly with these two half-planes. In local-knockout branches, the other two edges only confine the patch to the lower bond's complete visible contour and must not extend along the whole upper bond. A tiny antialiasing allowance may be added only to the knockout's lower-contour confinement edges; it must not widen the upper cut interval.
- Bonds sharing endpoints still use the node contact kernel, not this white-margin rule.
- For an interior centerline intersection, the intersection must lie inside both finite segments and the local strip rule above applies.
- ChemDraw also tests the upper bond's **finite margin envelope** against the lower bond silhouette. The envelope expands each lateral side by `marginWidth` and extends past both butt caps by `marginWidth` along the bond axis. Therefore a near-endpoint miss can still shorten or notch the lower bond even when the two finite centerlines do not intersect. Render the overlap of that finite envelope and the lower visible silhouette; do not extend an infinite white strip through the document.
- Shared endpoints remain node contacts, and nearly parallel or overlapping bonds do not enter this near-endpoint crossing rule.

## Default Circled Charge Symbols

- A default-style `CirclePlus` or `CircleMinus` bounding box is an anchor, not
  the circle centerline diameter. The diameter is the anchor height minus
  `0.30pt`; the source anchor remains unchanged for round-trip export.
- At editing scale 1, the internal plus/minus sign is `5.444pt` wide/high as
  applicable and uses a `0.8pt` stroke. These are symbol-template metrics, not
  values inferred from the rendered bitmap.

## Preview

- Dragging to draw a bond should not draw a fake overlay bond.
- Preview state calls the same `render_document` through a temporary document.
- Therefore preview and committed output must share the same contact geometry.

## Refactor Constraints

When splitting the `render` module later, keep these constraints:

- Do not change the geometry rules in this document.
- Prefer splitting high-level dispatch from low-level geometry; do not continue changing rules in the same refactor.
- After every split, pass:
  - `cargo test`
  - `npm run build:engine-wasm`

## Test Baseline

At least one layer of automated tests must cover these rule classes:

- main-bond contact side selection and straight-through decisions
- centered double `162°` threshold
- intersection fallback after miter-limit truncation
- hash-family rhythm allocation: equal black segments and floating white segments
- boundary pairing must not choose trivial assignments incorrectly
- hash family stays as-is in multi-bond nodes, while other bonds retreat

Suggested test layers:

- Low-level unit tests: directly test intersection, side selection, and rhythm-allocation helpers.
- Render-level tests: test output primitive counts, types, and key vertex relationships.
- Foreground validation: only confirms that the viewer has no stroke / antialiasing artifacts; it does not define geometry.

## Scope Boundaries

- SVG antialiasing details are handled by the concrete rendering backend.
- Pixel-level differences across browsers are handled by visual regression tolerance.
- Debug panels, debug SVGs, and development patches are development tools.

## Known Responsibility Boundaries Suitable For Splitting

- `render_document` and object-level dispatch
- high-level bond render dispatch
- main bond contact kernel
- bond geometry / intersection helpers
- render primitive definitions and push helpers
- legacy mol render compatibility layer
