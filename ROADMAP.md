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
- Completed: stable structured issues for `validate structural|chemical|roundtrip`, explicit molecular validation, and CCJS/CCJZ/CDXML/CDX/SDF target-format semantic/visual round trips.
- Completed: editor visible-region scene-chunk loading, hydration that preserves edits and undo, copy-on-write reuse of unchanged entries/attachments, and browser Zip64 read/write with safe-integer rejection.
- Completed: smoke gates for first-chunk I/O, last-chunk-edit reuse ratio, attachment throughput, and cross-implementation Zip64/viewport behavior.
- Publication work: unify Rust/JavaScript/Python rejection fixtures into a fixed public corpus and archive full 100k/1m-entity plus 100 MB/1 GB attachment performance reports.

## Product Experience

- Improve the online demo so users can drag in CDXML files, export SVG/CDXML, and share reduced repro cases directly from the browser.
- Add compact onboarding examples while keeping the first screen a usable editor.
- Build clearer diagnostics for unsupported CDXML objects and partial imports.

## GUI Test Platform And Demo Reliability

- Build an independent `packages/gui-test`, versioned scenario/result/coverage protocols, and a test-build-only Test ABI inside the main repository; store large traces, soak logs, VMs, and installers in hash-bound artifact storage.
- Use WebdriverIO Tauri for the real desktop application, Playwright for browser and WebView2/visual validation, Windows UIA and real input for native window/file/clipboard/Office/touch/pen boundaries, and a production black-box gate for the final installer.
- Migrate the existing GUI, viewer-interaction, stability, toolbar, text, large-document, and Office scripts into one data-driven scenario model instead of extending separate thousand-line scripts; ultimately cover every user feature, object type, public property, and `0/1/2/many` homogeneous/heterogeneous multi-object combination through real clicks, drags, and drawing.
- Run real input only in isolated Hyper-V guests so the user's foreground remains untouched; cap all workers at 10 logical processors/20 GiB and parallelize across isolated desktops. Add a source-to-scenario impact graph and content-addressed evidence so only affected, expired, and non-cacheable work reruns while complex/large construction and soak continue.
- Add an explicit state model, seeded generation, automatic failure shrinking, fault profiles, and mutation qualification; retries cannot convert a flaky first failure into success.
- Establish `gui-pr`, `gui-nightly`, `release-qualification`, and `demo-qualification`; formal demos must satisfy the [long-term architecture](./docs/gui-test-platform-and-demo-reliability.md) with final-installer, clean-VM, repeated-run, and soak evidence.

## Community

- Use issues and discussions to collect real-world compatibility files that can be reduced into shareable fixtures.
- Tag compatibility reports by source application, object type, and output path.
- Keep documentation focused on stable behavior contracts.
