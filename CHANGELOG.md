# Changelog

All notable public changes to ChemCore are tracked here.

## 1.0.0-beta.5

Agent-focused CLI expansion, installable entry points, and another round of
desktop/browser stabilization.

- Added formal CLI protocol/version reporting, machine-facing protocol
  contracts, an agent demo corpus, an agent POC workflow note,
  release-quality matrices, and a split `agent/` CLI module layout to keep the
  expanded agent surface maintainable.
- Packaged an installed `chemcore-cli` entry point alongside the desktop app, with installer PATH registration, `chemcore-entrypoints.json`, an installed agent guide, and `guide`/`doctor` discovery for machine callers.
- Clarified the CLI's two invocation modes: one-shot PowerShell commands for independent work, and JSONL `session` for repeated operations on the same loaded document.
- Expanded the CLI agent workflow with `targets`, `context`, `detail`, `capture`, and `copy`, covering stable selectors, nearby-object summaries, raw object/detail lookup, precise crops, and Office/OLE clipboard payload generation.
- Added deterministic high-resolution capture for objects, molecules, nodes, bonds, all content, explicit bounds, and multi-target selections. Multi-target crops use the minimum union bounds, matching the GUI selection box, and support absolute/relative per-side expansion, fixed pixel sizing, render metadata, and verified PNG/SVG writes.
- Added selection-box context reporting for precise crops, including objects, molecules, nodes, and bonds inside the crop box, `inside` versus `partial` containment, explicit target markers, and normal nearby summaries around the box.
- Added lightweight CLI audit reports for `new`/`run`, including document hash/revision transitions, created/updated/deleted selector summaries, failed-command details, optional `--inspect-after` snapshots, and `--continue-on-error`.
- Improved CLI resilience for agent use: verified document/JSON/screenshot/payload writes after saving, added machine-readable missing-argument fixes, and made command typos return nearby commands with purpose, usage, and examples.
- Added a long-lived JSONL `session` mode and an automatic CDXML/CDX import cache so repeated work on large documents can reuse loaded or cached state instead of reparsing every command.
- Optimized large-file CLI inspection and capture with target-scoped bounds, region rendering, and a `performance:cli-large` report covering CDXML conversion, target discovery, detail lookup, context screenshots, precise captures, session flows, and SVG export.
- Fixed centered/double-bond rendering near labels so parallel double-bond lines retreat and clip independently against endpoint labels, and updated the Office EMF preview stroke conversion for short clipped double-bond segments.
- Split the browser/editor host into focused document rendering, viewport, toolbar, tab, and window-lifecycle modules, reducing the maintenance burden of the large viewer surface.
- Expanded desktop and browser stability coverage for pointer workflows, hybrid latency, viewer operations, large-object editing, drag previews, text editing, generated fixtures, and repeatable stability reports.
- Tightened editor interaction behavior around selection, dragging, drawing, symbols, brackets, hover/focus lifecycle, grouped objects, mixed object workflows, and current-tool side panel activation.
- Updated README language links, Chinese README wording, rendered comparison assets, and the public CLI guides.

## 1.0.0-beta.4

Large-document interaction, CDXML fidelity, and agent-friendly CLI beta release.

- Added the `chemcore-cli` crate and direct engine commands for headless inspection, conversion, export, document editing, and structured JSON execution reports.
- Added `--document-json`, `--inspect-after`, and improved `.json`/`.ccjs` handling so scripts and agents can exchange ChemCore documents without driving the GUI.
- Improved CDXML import/export fidelity across labels, arrows, symbols, bold widths, radicals, grouped graphics, stacked/attached labels, cached fragments, and bracketed labels.
- Imported CDXML bracket pairs as bracket groups with independently draggable left/right sides while preserving repeat-count and bracket-label semantics.
- Tightened glyph clipping, label geometry, imported label anchors, and synthetic SVG snapshots.
- Reworked large-document interaction performance with more local rendering updates, cached drag-preview inputs, fragment-bounds filtering, reduced full-refresh paths, and safer deferred document synchronization.
- Rebuilt selection and drag previews so large structures, labels, arrows, shapes, brackets, and imported objects stay visually in sync during high-frequency editing.
- Fixed drawing and commit refresh artifacts, including bond preview persistence, bond creation patching, and rectangular/near-horizontal bond rendering quality.
- Unified Select-tool hover and cleaned up hover/focus/overlay lifecycle problems across drawing, object creation, selected-object drags, bracket/arrow edits, and multi-molecule operations.
- Clarified grouped-object selection semantics: ordinary child-object dragging stays independent, explicit group selection still collapses to a group box, and selected objects move together only when actually selected.
- Refined arrow, bracket, shape, and object handles, including bracket hit testing that ignores interior empty space, selected-object hover suppression, and consistent control styling.
- Added browser file drag-and-drop/current-viewer opening, shared display-scale handling, faster desktop/viewer development scripts, and expanded interaction/performance regression coverage.
- Added English and Chinese CLI command guides, public interaction-feedback rules, early project history notes, and README architecture updates.

## 1.0.0-beta.3

Installer hotfix beta release.

- Fixed the Windows NSIS installer Office/OLE registration hook so it finds `chemcore-office.exe` in the installed application directory instead of assuming the old `resources` subdirectory layout.
- Kept compatibility with both root-level and `resources`-level Office server layouts so older packaging experiments do not break registration.
- Hardened post-install registration: the installer now tries machine-wide COM/OLE registration first, then falls back to current-user registration if the machine step cannot run or returns a failure code.
- Hardened uninstall cleanup by attempting both machine-wide and current-user Office/OLE unregistration.
- Rebuilt and manually verified the Windows x64 installer after a clean-install trace cleanup.

## 1.0.0-beta.2

Second public beta release.

- Added bracket-to-count text links for repeating units, including Link/Unlink context-menu actions, `Ctrl+L` / `Ctrl+Shift+L` shortcuts, CDXML import pairing, and repeat-unit refresh after edits.
- Improved bracket text editing: empty labels created by bracket drawing are discarded on the next tool action, non-empty labels commit before switching tools, bracket labels remain editable with the text tool, and bracket-label placement/font defaults are aligned with ChemDraw bracket fixtures.
- Fixed repeat-unit chemistry summaries so linked numeric bracket counts contribute to formula and mass when the repeat unit is well-defined; unlinking detaches the count semantics without breaking bracket selection.
- Expanded selection behavior around grouped objects and brackets: double-clicking a molecule includes enclosing brackets and linked counts, grouped scene text remains editable, and switching away from Select clears the current selection state.
- Fixed stale hover/focus state after drawing, changing curved-arrow geometry, and moving between selected objects; selected labels, bonds, and atoms no longer keep internal hover highlights inside selection boxes.
- Added desktop/browser editing polish: the window top edge can start dragging even while modal prompts are open, browser context menu and common browser shortcuts are intercepted during editing, and context-menu glyph mojibake is removed.
- Fixed chemistry summaries for indeterminate generic labels: selected molecules containing `R`, `R'`, `R''`, or connected `Ar` no longer show formula or molecular-weight values that would imply a fully known composition.
- Treated connected `Ar` labels as generic aryl abbreviations instead of argon during structure-label editing, while keeping explicit element replacement available through the element workflow.
- Rebuilt the browser WASM engine and Windows desktop executable so the web and desktop surfaces use the same corrected engine behavior.
- Added regression coverage and public fixtures for bracket CDXML imports, repeat-unit links, grouped editing, selected-object hover suppression, generic-label chemistry summaries, and complete abbreviation expansion summaries.

## 1.0.0-beta.1

Initial public beta release.

- Published the shared Rust chemistry editor engine, browser viewer, Windows desktop shell, and Office/OLE integration foundations.
- Added CDXML/CDX import and export paths, SVG export, EMF preview generation, and Word-oriented clipboard/OLE payload support.
- Included public synthetic CDXML regression fixtures plus maintainer-authored published-figure benchmark files.
- Added GitHub Actions CI, GitHub Pages demo deployment, issue templates, roadmap, and rendering comparison documentation.
- Documented the current beta status: source builds are available now, and Windows installer packaging is still under test.
