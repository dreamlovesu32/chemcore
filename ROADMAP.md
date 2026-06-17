# Roadmap

ChemCore is in public beta. The near-term roadmap focuses on making the editor easier to try, easier to validate, and safer to evolve with outside contributors.

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

## Product Experience

- Improve the online demo so users can drag in CDXML files, export SVG/CDXML, and share reduced repro cases directly from the browser.
- Add compact onboarding examples while keeping the first screen a usable editor.
- Build clearer diagnostics for unsupported CDXML objects and partial imports.

## Community

- Use issues and discussions to collect real-world compatibility files that can be reduced into shareable fixtures.
- Tag compatibility reports by source application, object type, and output path.
- Keep documentation focused on stable behavior contracts.
