# ChemDraw collapsed-fragment automatic layout rules

Status: active reverse-engineering specification. Rules marked **verified** are backed by
controlled ChemDraw open/save probes and public-corpus examples. Rules marked **open**
must not be encoded as guessed fallbacks.

## Scope

This document covers a CDXML import condition in which a displayed top-level
`fragment` contains one or more direct child nodes with `NodeType="Fragment"` but no
`p` coordinate. ChemDraw does not treat those nodes as independent page-grid points.
It completes and cleans each affected molecular component, then packs complete
components without overlap.

The rule applies by structure and topology. File names, corpus paths, object ids, and
reaction names are never inputs to the implementation.

## Primary sources

- The official CDX object model defines a `Fragment` as a collection of nodes and
  connectivity and a `Node` as the basic chemical-structure building block:
  <https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/AllCDXObjects.htm>.
- The official property table defines document `BondLength` as the default bond
  length and `ConnectionOrder` as the ordered attachment points of a fragment:
  <https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/TableOfProperties.htm>.
- CambridgeSoft's structure-diagram-generation patent describes molecule cleanup
  followed by component packing with the free-rectangle method. Each molecule is
  represented by its smallest enclosing rectangle plus a margin, boxes are processed
  in decreasing area, and the final conglomerate is centered:
  <https://patents.google.com/patent/US7912689B1/en>.

The patent establishes the algorithm family. Exact current-product constants and
tie-breaking are established by the probes in
`scripts/chemdraw-collapsed-fragment-layout-probe.mjs`.

## Verified pipeline

### 1. Detection

**Verified.** Automatic completion is triggered by a displayed wrapper node whose
outer `p` coordinate is absent. The nested fragment's coordinates do not make the
outer wrapper positioned.

All affected top-level fragments on the page participate as molecular components.
An affected component may be:

- a singleton collapsed wrapper;
- an otherwise positioned molecule with one or more missing wrapper positions; or
- a positioned nickname/wrapper pair whose missing endpoint must be completed.

Unrelated positioned molecules and graphics do not become entries merely because
they share the page.

### 2. Complete missing outer-node coordinates

**Verified.** Missing outer nodes are completed from parent-fragment topology before
component packing. A bonded missing node is placed relative to its positioned parent
neighbor using the document `BondLength` and the available valence sector. It is not
always horizontal.

Examples verified by controlled probes:

- a substituent on a regular ring is placed into the outward, largest angular sector;
- several missing substituents on one atom occupy distinct available sectors;
- bond endpoint order (`B` versus `E`) alone does not select left versus right;
- nested-fragment label width does not select the outer-node direction.

**Open.** Exact sector subdivision and tie-breaking for one existing neighbor plus
multiple missing neighbors still require a complete degree/stereochemistry matrix.

### 3. Clean each affected molecular component

**Verified.** Cleanup is component-level, never wrapper-node-level. For the common
public-corpus case with source bonds of 14.4 and document `BondLength="30"`, ChemDraw
normalizes ordinary source bonds by `30 / 14.4`. The missing wrapper coordinate is
first completed with the document bond length and then participates in the same
component transform; this explains observed 62.5-unit wrapper-center distances in
the uniformly scaled examples.

Rigid rings and many already-aesthetic fragments undergo a uniform similarity
transform. More complex acyclic portions may also change rotatable-bond angles, so a
similarity transform is not a complete cleanup algorithm.

If the existing component already uses the document bond length, the scale is one.

**Open.** Exact cleanup forces and stopping conditions for branched/fused/acyclic
components. The CambridgeSoft patent identifies preferred bond lengths and angles,
symmetry terms, and iterative 2-D dynamics, but current-product constants must be
measured.

### 4. Construct component boxes

**Verified for zero-height singleton and horizontal two-node components.** The
packing box is the node-coordinate bounding rectangle expanded by
`BondLength / 2` on all four sides:

- a singleton therefore occupies `BondLength × BondLength`;
- a horizontal two-node component with a one-bond node span occupies
  `2 × BondLength` by `BondLength`.

This exactly explains the measured 14.35/30-unit row spacing and 28.70/60-unit
column spacing. Label string width does not change singleton grid placement in the
controlled matrix.

**Open.** Whether atom-label visual bounds, unusually large fonts, annotations, or
bond decorations enlarge the packing box. These must be probed separately instead
of silently approximated.

### 4a. Wholly coordinate-free singleton collection

**Verified for every count from 1 through 64.** When every affected component is a
singleton with no parent anchor, ChemDraw's equal-box packing reduces to a closed
square-shell column rule. Coordinates below are expressed in document-bond-length
units:

1. counts one through three form one horizontal row;
2. for count `n >= 4`, the first-column height is
   `h = floor(sqrt(n - 1)) + 2`;
3. the first column contains `h` entries starting at `y = 0`;
4. subsequent full columns contain `h - 1` entries;
5. if division by `h - 1` leaves a remainder of one, that entry is absorbed into the
   preceding column, making that column height `h`;
6. a later column of height `k` starts at `y = (h - k + 1) / 2`, which produces the
   measured half-bond-length offsets.

This formula reproduces all 2,080 measured output points across counts 1–64. Changing
all nested-definition coordinates, translating them, reversing their x order, or
making them vertical leaves the outer-node result unchanged. The production rule is
therefore count-derived rather than a table of per-count coordinates.

### 5. Pack whole components

**Verified.** Components are sorted by decreasing packing-box area. Larger bonded
components are packed before singleton wrappers even when the singleton nodes occur
earlier in XML order. Equal-size components retain source order. Reversing the XML
component order reverses the equal-area packing order without changing the rule.

ChemDraw uses the CambridgeSoft free-rectangle method rather than a fixed count table:

1. maintain maximal free rectangles outside already placed boxes;
2. choose a free rectangle close to the current conglomerate center that can contain
   the next box;
3. place the next box flush with the free-rectangle corner nearest the center;
4. imprint the box, expand remaining rectangles until they touch placed boxes, discard
   rectangles too small for the smallest remaining box, and merge insignificant
   overlaps;
5. continue until all component boxes are placed.

The measured nine two-node plus six singleton case forms a three-column bonded block
followed by the smaller singleton block. This cannot be represented correctly by the
old per-node 1–8 lookup table.

**Open.** Exact free-rectangle distance metric, equal-distance tie order, merge
threshold used by the current ChemDraw release, and the mapping from the centered
conglomerate back into page coordinates.

### 6. Absolute placement

**Verified.** Source page count and page `BoundingBox` do not independently
shift a coordinate-free singleton grid. Unrelated positioned molecules also do not
shift it. A graphic can introduce a small deterministic translation in a cleaned
group, so graphics cannot be ignored in the final page-coordinate phase.

After stable decreasing-area sorting, the first component that has a source anchor
retains its completed/cleaned coordinates and the remaining components are packed
relative to it. Translating every source anchor by `(dx, dy)` translates every output
component by exactly `(dx, dy)`. For a wholly coordinate-free singleton set, the
first point is at the origin. Reversing source order makes the first equal-area
anchored component the preserved anchor.

**Open.** The anchor transformation when the first component itself undergoes a
non-unit cleanup scale, and the small deterministic translation introduced by some
graphics/page transforms.

## Importer invariants

The replacement implementation must satisfy all of the following:

1. Layout entries are top-level molecular components, not missing wrapper nodes.
2. Missing positions are completed from topology before cleanup.
3. Cleanup has one explicit rule branch per supported topology; unsupported topology
   is an error with evidence, never a hidden horizontal/grid fallback.
4. Packing uses computed component boxes and one general algorithm for every count.
5. The implementation contains no file names, source paths, reaction names, object-id
   lists, or corpus-specific constants.
6. Existing explicit coordinates are preserved unless the verified cleanup phase
   requires a documented transform.
7. Public visual-gate comparisons use a fixed gate version and protected-pass set.

## Regression matrix

The required automated matrix is:

- 1–64 singleton wrappers, including every square-shell boundary;
- label-width variants;
- page-count and page-bounds variants;
- unrelated molecule and graphic variants;
- one anchored component in every source slot;
- multiple equal-size anchored components;
- mixed component areas in multiple source orders;
- translated and scaled source coordinates;
- one and several substituents on rings;
- degree 1–4 parent atoms, including stereochemical bonds;
- all 19 public failing documents that contain missing outer wrapper coordinates;
- the complete 338-document protected visual gate.

Passing a synthetic matrix is necessary but not sufficient. A rule is promoted only
after the affected public documents improve without any protected-pass regression.
