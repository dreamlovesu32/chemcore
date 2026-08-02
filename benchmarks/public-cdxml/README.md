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

Generate the baseline with the current gate definition. Old classifications
cannot be made trustworthy by replacing their hashes or provenance:

```bash
node scripts/public-cdxml-visual-gate.mjs \
  --gallery tmp/public-cdxml-chemdraw-review-all \
  --out tmp/public-cdxml-chemdraw-review-all/gate-report.json
```

Inspect and then run the affected plan:

```bash
npm run benchmark:cdxml-public:visual-gate:affected -- --dry-run
npm run benchmark:cdxml-public:visual-gate:affected
```

The planner updates only selected gallery items. The gate reuses classifications
only when the ChemDraw-oracle hash, ChemSema-SVG hash, and gate-definition
identity all match; `cache.reused` and `cache.analyzed` expose that split.
Every changed candidate is registered from its own current pixels for its
ordinary pass/fail verdict. Regression history is deliberately independent of
that cache identity, so upgrading the alignment or detail classifier cannot
erase earlier passes. Baseline mode permits historical failures to remain
open, but it does not permit them to become worse. Every pass-to-fail
transition is recorded in `delta.regressions`;
`delta.continuousRegressions` independently compares cases that remain red.
For this second comparison, gate v23 restores the historical
document-coordinate registration and compensates for any changed SVG `viewBox`
crop before rebuilding the exact coarse and zero-tolerance detail mismatch
masks. This prevents an unrelated crop or newly corrected bounding box from
making unchanged ink look displaced.

A current missing or extra pixel is stable when a historical pixel of the same
kind lies within 0.75 ChemDraw reference units. Each unsupported connected
defect is then compared against the pooled missing-plus-extra mismatch mass in
a fixed 12-reference-unit neighbourhood; it is accepted only when that local
mass strictly decreases. Unsupported pixels accumulate across the whole
drawing and remain subject to fixed absolute area limits, so improving a
distant label cannot pay for a new bond or glyph defect. A protected metric
disappearing or a material loss without authoritative spatial masks is still a
regression. A genuine local raster trade-off must be reviewed and explicitly
promoted instead of being silently accepted. Fixed reference-unit windows,
component geometry, and absolute defect bounds keep the decision independent
of canvas size. Both kinds of regression are checked during pass-floor
migration. Path-to-feature
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
pass-to-fail transitions. If the corrected definition proves that an old pass
was a false positive, every retired path must appear in a committed, sorted,
reasoned review manifest:

```bash
node scripts/public-cdxml-visual-gate.mjs \
  --gallery tmp/frozen-candidate-gallery \
  --gate-definition-upgrade --report-only --allow-stale-gallery --jobs 8 \
  --out tmp/frozen-candidate-gate-report.json
```

`--gate-definition-upgrade` is the only bootstrap path past an incompatible
committed floor. It forces the exact original-338 cohort, produces a diagnostic
report only, rejects baselines, caches, and partial filters, and is rejected as
soon as the committed floor already matches the current definition.

```bash
npm run benchmark:cdxml-public:visual-gate:migrate-floor -- \
  --previous-report tmp/frozen-candidate-gate-report.json \
  --current-report tmp/current-candidate-gate-report.json \
  --reviewed-retirements benchmarks/public-cdxml/gate-definition-retirements-v22.json \
  --reviewed-renderer-migration benchmarks/public-cdxml/renderer-migrations/hash-bond-v2.json
```

The migration binds the frozen report to the old floor's exact repository
identity and records both report hashes, same-gate changes, and the retirement
manifest hash. Unlisted old passes and any further floor shrink remain blocked.
The manifest audits a gate-definition correction; it is never consulted by the
renderer or by ordinary pass classification.

A verified renderer-rule correction can move residual mismatch pixels inside
documents that remain red. The ordinary gate must still report that movement.
Accepting it into a new floor requires a separate committed renderer-migration
review that binds the exact previous/current repository identities, the common
rule and probe evidence, and both candidate hashes for every affected red case.
Its sorted case set must exactly equal the continuous regressions: it cannot
hide an unrelated seventh regression, change a pass to a failure, or weaken
future comparisons. Renderer-migration reviews are migration audit records,
not runtime path exceptions.

Gate v23 runs both raster resolutions for every comparable case, including
images that already fail badly at coarse resolution. It assigns every analyzed
core pixel exactly once to a fixed ChemDraw-coordinate cell and compresses the
exact directional missing/extra occupancy masks into the protected floor. A
current mismatch pixel must be supported by a historical same-kind pixel inside
the fixed absolute raster tolerance. Unsupported components additionally need
a strict reduction of pooled local mismatch mass, and all remaining unsupported
pixels accumulate across cells. The raw mask records carry a SHA-256 integrity
hash; zlib bytes are storage only and are not treated as canonical across
runtime versions. Pass classification has one path: the fixed-window coarse
limits and the zero-tolerance detail rules must both pass. Retired whole-page
coverage and topology-equivalence branches cannot make the same defect easier
to accept merely because the canvas is larger.

The gate counts a fine component only when the unmatched component has no
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
crop cannot move registration or sampling. This prevents an asymmetric label
box from trapping registration in a nearby local optimum. Historical pass
protection never changes the current candidate's ordinary registration. The
separate continuous-regression analysis reuses only the old
document-coordinate map, adjusted by the exact old/new `viewBox` origin delta,
so it compares the same physical locations without biasing the current
pass/fail result.
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
