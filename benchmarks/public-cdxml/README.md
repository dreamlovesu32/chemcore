# Public CDXML/CDX Round-Trip Corpus

This benchmark uses public, license-clear ChemDraw CDXML/CDX files instead of
confidential research documents. Source files are downloaded into ignored
`tmp/` storage and are not vendored into the ChemSema repository.

The pinned manifest currently provides 413 files from five upstream projects:

| Source | License | CDXML | CDX | Main coverage |
| --- | --- | ---: | ---: | --- |
| RDKit | BSD-3-Clause | 94 | 126 | parser regressions, queries, templates, patent structures |
| Indigo | Apache-2.0 | 123 | 28 | molecules, reactions, rendering, malformed-input tests |
| cdxml-toolkit | MIT | 34 | 2 | complete linear, wrapped, and branched reaction schemes |
| SAMPL6 | MIT | 1 | 2 | published host/guest structures |
| SAMPL9 | MIT | 2 | 1 | published host/guest structures |

Two files are deliberate malformed-input tests. Four `.cdx` files contain
Base64 transport text rather than raw CDX bytes and are classified separately.
The remaining 407 files are positive round-trip cases. One deliberately broken
coordinate fixture is classified as safe sanitization, and two fixtures that
only discard unused shape styles are classified as lossless normalization.

## Reproduce

```bash
npm run benchmark:cdxml-public:fetch
cargo build -p chemsema-cli
npm run benchmark:cdxml-public
```

To build the ChemDraw-versus-ChemSema visual review gallery for every corpus
entry, run:

```bash
node scripts/render-public-cdxml-visual-review.mjs --all \
  --root tmp/public-corpus-pilot \
  --report tmp/public-cdxml-roundtrip-label-audit/report.json \
  --out tmp/public-cdxml-chemdraw-review-all
```

The gallery normalizes both panels into the ChemDraw reference coordinate
space. The ChemDraw SVG path matrix declares the conversion from
twentieth-of-a-point coordinates to reference pixels, so that uniform scale is
fixed. Because an absolute page origin is not portable document semantics, the
outer ink-bounds centers seed a translation-only overlap search; the gate
cannot fit scale, rotation, or non-uniform distortion. Translation is searched
on a fixed document-world lattice: the candidate SVG `viewBox` is only an
export crop, so changing its origin or extent adjusts display-space placement
without changing document-world registration. Tile and local-window lattices
are likewise anchored in ChemDraw reference coordinates rather than at a
viewport edge. Every current candidate is registered from its own pixels;
historical alignment never overrides current-image evidence.
The gate also uses the SVG's
possibly fractional declared `width` and `height`, rather than the browser's
independently rounded intrinsic dimensions, so the candidate is not silently
stretched. Only references without a vector scale use the explicitly marked
ink-overlap branch. Review
state, notes, the current item, display mode, opacity, and box-selection mode
are saved to browser local storage as they change. A box drawn on either panel
is stored in reference coordinates, appears on both panels, and immediately
marks the item as an issue. Box-selection mode remains active while navigating
and after reopening the gallery.

The gallery is a diagnostic aid, not the release gate. The automated visual
gate consumes its retained ChemDraw oracles and aligned ChemSema renders:

```bash
npm run benchmark:cdxml-public:visual-gate
# Inspect the current baseline without returning a failing exit status:
npm run benchmark:cdxml-public:visual-gate:report
```

When a new canonical gallery directory is required, retain the already reviewed
ChemDraw output instead of silently saving every source through ChemDraw again:

```bash
node scripts/render-public-cdxml-visual-review.mjs --all \
  --oracle-gallery tmp/public-cdxml-previous-clean \
  --out tmp/public-cdxml-current-clean
```

`--oracle-gallery` requires the exact same corpus manifest and source revisions,
and requires a retained reference for every eligible input. An incomplete or
incompatible oracle gallery fails explicitly; it never falls back to a mixed
old/new ChemDraw baseline.

The gate gives every comparable document one vote, regardless of canvas or
file size. Blank canvas pixels never enter the score. Its coarse stage uses
fixed-size local windows and connected missing/extra ink components. A second,
finer stage checks connected-object count, the dimensions of small symbols,
and repeated compact micro-defects such as disconnected dashed-bond endpoint
miters. For complex multi-object drawings, component counts and normalized X/Y
position distributions remain diagnostic signals, but cannot override an
empty local window, a fixed-span defect, or failed foreground coverage. All
thresholds are expressed in
ChemDraw reference coordinates or normalized structure coordinates, so a
small missing label, sign, or bond detail cannot be diluted by a large molecule,
reaction scheme, or page. Candidate SVGs also
receive an independent viewport self-consistency check: the renderer promises
an 8 pt ink margin, and the gate enforces a fixed 4 pt minimum on every edge.
Ink touching or clipped by the root viewport therefore fails with
`candidate-viewport-ink-margin` regardless of whole-image similarity or canvas
size. The JSON report includes canonical-coordinate boxes
and explicit reason codes for the strongest local defects. Cases without a
real ChemDraw oracle are reported separately and excluded from the pass-rate
denominator. Every gate run also writes
`passed.html` beside the full gallery so accepted cases can be inspected
without mixing in failures. Use `--reuse-report report.json` to rebuild that
page without rerunning pixel analysis.

### Incremental visual gate

Ordinary rendering fixes should not rerun all 413 files. The affected gate follows the OCR repository's contract: map changed code paths to visual rule families, select matching corpus features plus historical regression cases, and save the machine-generated contract to `tmp/public-cdxml-affected-gate-plan.json`. Do not replace that plan with a hand-written case list; use `--extra` for additional diagnostic cases.

Stamp an existing full report once so it can serve as the content-hashed baseline:

```bash
node scripts/public-cdxml-visual-gate.mjs \
  --gallery tmp/public-cdxml-chemdraw-review-all \
  --stamp-report tmp/public-cdxml-chemdraw-review-all/gate-report.json
```

Inspect and then run the affected plan:

```bash
npm run benchmark:cdxml-public:visual-gate:affected -- --dry-run
npm run benchmark:cdxml-public:visual-gate:affected
```

The planner updates only selected gallery items. The gate reuses classifications
only when the ChemDraw-oracle hash, ChemSema-SVG hash, and gate-definition
identity all match; `cache.reused` and `cache.analyzed` expose that split.
Every changed candidate is registered from its own current pixels. Regression
history is deliberately independent of that cache
identity, so upgrading the alignment or detail classifier cannot erase earlier
passes. Baseline mode permits historical failures to remain open, but it does
not permit them to become worse. Every pass-to-fail transition is recorded in
`delta.regressions`; `delta.continuousRegressions` independently compares cases
that remain red using global and fixed-window coverage, largest missing/extra
components, independent unmatched component counts, relative component
matches, component position distributions, and fine-detail defects. A new
defect reason, a protected metric disappearing, or a material metric loss
beyond the explicit raster tolerance is a regression. Improvement in another
metric cannot cancel it: registered-image statistics are not additive, so a
trade-off must be reviewed and explicitly promoted instead of being silently
accepted. Fixed reference-unit windows, component geometry, and absolute
defect bounds keep the decision from being diluted by canvas size. Both kinds
of regression are checked during pass-floor promotion and gate migration. Path-to-feature
declarations and historical regression cases live in
`benchmarks/public-cdxml/visual-impact-map.json`; unknown production changes and
gate-definition changes conservatively force a full run.

Strict original-338 runs also load
`benchmarks/public-cdxml/strict-pass-floor.json`. The tracked floor is bound to
one exact gate definition and stores all 338 paths: the cumulative union of
accepted passes plus the status, ChemDraw-oracle hash, defect reasons, and
non-regression metrics for every remaining failure. Strict mode compares
against this committed all-case floor directly; an optional report is only an
analysis cache and cannot replace regression history. It also rejects a
changed oracle, a missing protected metric, a stale/dirty/partial gallery, or
any loss hidden behind a gain elsewhere. After a clean strict run adds passes
or improves red cases with zero regressions, promote the new all-case floor:

```bash
npm run benchmark:cdxml-public:visual-gate:promote -- \
  --report tmp/public-cdxml-visual-gate-current-strict338.json
```

Promotion validates the exact 338-case cohort, clean and current repository/CLI
provenance, zero analysis errors, and zero immediate or cumulative regressions.
It unions new passes into the floor and ratchets every remaining red case to
its improved metrics; it never removes protected paths.

When the gate definition itself is corrected, the old verdict set is not
silently carried forward. Re-analyze both a frozen previous candidate gallery
and the current gallery with the new definition, then migrate only if the two
exact 338-case reports use identical ChemDraw oracles and show zero same-gate
pass-to-fail transitions:

```bash
npm run benchmark:cdxml-public:visual-gate:migrate-floor -- \
  --previous-report tmp/frozen-candidate-gate-report.json \
  --current-report tmp/current-candidate-gate-report.json
```

The migration records both report hashes and the same-gate improvement and
regression counts. This retires verdicts that existed only because of a proven
old gate bug without adding file exceptions or weakening the current rules.

Gate v20 counts a fine component only when the unmatched component has no
opposite-side ink within 0.5 ChemDraw reference units. Raw 8-connected component
counts remain in reports for diagnosis, but cannot by themselves reject a
drawing: font hinting can split one ChemDraw glyph into two raster components
without adding an object. Unlike whole-image dilation, spatial support does not
merge neighboring letters or bonds. Missing or displaced details still fail
through fixed-coordinate coverage, defect area/span, repeated micro-defects,
and spatially independent component counts. Gate v19 made the fixed-coordinate detail checks non-bypassable. Neither a
whole-page coverage score nor topology distribution can excuse an empty local
window or a large local defect, and coarse pixel agreement cannot discard a
fine connected-component mismatch. The detail pass uses zero dilation to
retain repeated sub-pixel join differences; its repeated-defect rule is
calibrated in reference-image units, not as a percentage of canvas size.
Missing and extra fragments may be classified as one displaced detail only
when they both fit within one fixed detail-window distance; look-alike edges on
opposite sides of a large drawing cannot be paired. SVG
scale is derived from the declared vector matrix. Translation is resolved for
each current candidate on a document-world lattice by a broad low-resolution
global-overlap search followed by two fixed-scale refinement passes. Tiles and
local windows use a fixed reference-coordinate lattice, so a root `viewBox`
crop cannot move registration or sampling. This prevents an asymmetric label box
from trapping registration in a nearby local optimum. Historical pass
protection is evaluated after current-image registration and never injects an
old candidate's translation into a changed render.
Canonical runs reject any gallery whose
repository, CLI, corpus, round-trip report, or per-item candidate provenance
does not match. `--allow-stale-gallery` is diagnostic only and never makes
stale artifacts canonical.

Use `--cohort original-338` to run the exact original review cohort recorded
in `benchmarks/public-cdxml/failure-ledger.json`. The gate fails before image
analysis if even one cohort path is absent from the gallery, and records the
cohort name, ledger path, expected count, and selected count in the report.

If a later full gate finds a regression that escaped the affected plan, first repair the impact map or feature extraction so that family is selected in the future, then fix the renderer. A one-off manual case addition is not a complete repair.

Set `CHEMSEMA_PUBLIC_CDXML_DIR` to choose another download directory. The
runner writes a detailed untracked report to
`tmp/public-cdxml-roundtrip/report.json`. By default, every positive case is
saved and reopened three times. Each generation is checked with molecule,
arrow-identity, bracket-geometry, atom-label, and free-text semantic
fingerprints as well as object, resource, style, and object-type counts. The
text gates compare source and displayed text, line structure, runs, alignment,
anchor, wrapping, line height, and label/text geometry. Semantic drift and
non-idempotence always fail the run; pass `--strict-counts` to also fail on a
classified count drift.

The current ChemSema 1.0.0-beta.1 source baseline has no unexpected failures,
semantic drift, non-idempotence, or unclassified count drift. Of 413 files, 404
are exact through all three generations, one is expected safe sanitization, two
are expected lossless normalization, two are expected import rejection, and
four transport-encoded files are skipped. The semantic gates cover atomic
identity and charge, molecule connectivity, headless-arrow identity, bracket
grouping and geometry, atom-label realization, and free-text layout; the count
gates independently catch object and resource growth.

The manifest pins every upstream commit and records its license URL. When the
corpus changes, update the manifest, rerun the benchmark, and commit a new
versioned summary rather than silently replacing an old baseline.

The license column records the license published for each upstream repository.
Because the downloader leaves every file in its original repository, this is
appropriate for a reproducible external benchmark. Before repackaging the
files as a standalone dataset, recheck per-file provenance and attribution,
especially for patent-derived RDKit fixtures.
