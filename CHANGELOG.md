# Changelog

All notable public changes to ChemCore are tracked here.

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
