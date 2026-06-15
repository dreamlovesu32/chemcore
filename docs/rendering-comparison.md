# Rendering Comparison And Regression Assets

ChemCore treats rendering fidelity as an engineering target. The public test
assets are split into two groups:

- synthetic fixtures in `fixtures/cdxml/`, used for reproducible CI-friendly
  regression tests;
- maintainer-authored published figure benchmarks at the repository root,
  `figure1.cdxml` and `figure2.cdxml`, used for high-signal visual comparison.

The benchmark CDXML files were authored by the maintainer and are included for
reproducible rendering comparison. Synthetic fixtures should remain the default
choice for new automated tests because they are small, license-clear, and easy
to reduce when a regression occurs.

## Golden SVG Snapshots

Golden SVG snapshots live in `fixtures/expected/svg/`. The Rust test suite reads
each `fixtures/cdxml/*.cdxml`, imports it through the engine, exports SVG, and
compares the result with the matching expected SVG file.

Run the snapshot test with:

```bash
cargo test -p chemcore-engine public_cdxml_fixture_svg_golden_snapshots_match --test render_document
```

When an intentional rendering change updates a fixture, regenerate the affected
SVG with:

```bash
cargo run -p chemcore-engine --example cdxml_to_svg -- fixtures/cdxml/synthetic-reaction.cdxml fixtures/expected/svg/synthetic-reaction.svg
```

Review the diff before committing. SVG snapshots are text fixtures, so changes
should show the exact primitives, coordinates, colors, and text output affected
by the engine change.

## ChemDraw Oracle Comparison

The ChemDraw oracle scripts are optional local tools. They require Windows plus
a local ChemDraw installation that exposes the ChemDraw COM application object.

Generate ChemDraw SVG/EMF for one or more CDXML files:

```bash
npm run emf:chemdraw-oracle -- --out tmp/chemdraw-oracle figure1.cdxml figure2.cdxml
```

Generate ChemDraw and ChemCore SVG/EMF, EMF inspection reports, and raster EMF
previews:

```bash
npm run emf:compare-oracle -- --out tmp/emf-oracle figure1.cdxml figure2.cdxml
```

The README comparison assets in `docs/assets/readme/comparison/` are regenerated
from those outputs and from ChemCore's `cdxml_to_svg` and Office EMF writers.
The default GitHub Actions CI uses open-source, dependency-free regression
checks; ChemDraw and Office oracle checks remain local Windows workflows.

## Adding Public Fixtures

Use synthetic chemistry, reduced layouts, or maintainer-authored files with
clear redistribution rights. Public fixtures should be shareable, minimized,
and free of unpublished reactions, customer/user documents, ignored-folder
screenshots, or files with unclear rights.

Good fixture names describe the behavior under test, for example
`label-clipping-basic.cdxml`, `equilibrium-arrow-geometry.cdxml`, or
`orbital-stacking.cdxml`. Pair each fixture with an expected SVG snapshot before
opening a pull request.
