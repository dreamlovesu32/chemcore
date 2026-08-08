# Roadmap

ChemSema is in public beta. The near-term roadmap focuses on making the editor easier to try, easier to validate, and safer to evolve with outside contributors.

## v1.0.0-beta Series

- Publish repeatable browser and desktop build instructions.
- Keep CI green for Rust tests, WASM generation, and browser JavaScript syntax checks.
- Expand synthetic CDXML fixtures and SVG golden snapshots around labels, arrows, brackets, orbitals, reactions, and Office export edge cases.
- Keep the published-figure comparison as a high-signal fidelity benchmark while moving routine tests to synthetic assets.
- Keep unsigned Windows installers in the beta channel until clean install, upgrade, uninstall, and Office/OLE registration are repeatedly validated.
- Release a signed Windows installer after desktop packaging, file association, update behavior, and Office copy/paste validation are stable enough.

## Fidelity And Compatibility

- Add more ChemDraw oracle comparison reports for public synthetic fixtures.
- Add optional pixel-diff and EMF-record diff workflows for local Windows machines with ChemDraw and Office available.
- Continue hardening CDXML/CDX round trips, text layout, arrow geometry, bond joins, and object stacking.

## CCJS 0.2 Stabilization

- Keep CCJS 0.2, CCJZ Container v1, Document Patch v1, and Recovery Journal v1 independently versioned; forbid undeclared ZIP entries and any second hierarchy authority.
- Complete stable structured diagnostics for `validate structural|chemical|roundtrip`: error code, JSON Pointer/entry, specification clause, severity, and information-loss classification.
- Unify target-format semantic/visual round trips and Rust/JavaScript/Python rejection fixtures into a publishable conformance corpus.
- Connect visible-region scene-chunk loading and copy-on-write reuse of unchanged entries in the editor; until then, do not market the low-level range reader as end-to-end lazy loading.
- Define the classic-ZIP limit and Zip64 policy for the browser writer, and publish full performance reports for 10k/100k/1m entities and 10 MB/100 MB/1 GB attachments.

## Product Experience

- Improve the online demo so users can drag in CDXML files, export SVG/CDXML, and share reduced repro cases directly from the browser.
- Add compact onboarding examples while keeping the first screen a usable editor.
- Build clearer diagnostics for unsupported CDXML objects and partial imports.

## Community

- Use issues and discussions to collect real-world compatibility files that can be reduced into shareable fixtures.
- Tag compatibility reports by source application, object type, and output path.
- Keep documentation focused on stable behavior contracts.
