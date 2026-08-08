# Changelog

All notable public changes to ChemSema are tracked here.

## Unreleased

- Upgraded the native document system to CCJS 0.2 plus CCJZ Container v1: canonical flat scenes, independent hierarchy/relations, deterministic chunked ZIP storage, content-addressed resources, opaque attachments, read-only legacy-gzip migration, atomic saves, and a hash-chain recovery journal.
- Added precise Document Patch updates on desktop, cross-reading among Rust, browser JavaScript, and an independent Python reader, `validate`/`canonicalize`/`migrate`/`conformance` CLI tools, and container plus 5,000-atom interaction performance gates. Remaining stable diagnostics, target-format round trips, editor visible-region loading, entry copy-on-write, and browser Zip64 policy are explicitly tracked in the roadmap.
- Added a reproducible public CDXML/CDX round-trip corpus pinned to 413
  license-clear files from RDKit, Indigo, cdxml-toolkit, SAMPL6, and SAMPL9,
  together with machine-readable baseline reporting.
- Upgraded that corpus gate to three-generation semantic fingerprints and fixed
  all unexpected drift it exposed: element and isotope labels, singleton atoms,
  dangling bonds, represented charge symbols, element-list queries, headless
  arrows, bracket groups and anchors, and dashed secondary double-bond lines.
- Restored connection-aware layout for imported attached atom labels: horizontal
  CDXML text justification no longer suppresses `NH`/`NH2` reversal or vertical
  stacking, while explicit node-level `LabelDisplay` modes remain authoritative.
- Made CDXML file loading tolerate isolated Windows-1252 punctuation bytes in
  legacy files that incorrectly declare UTF-8, while keeping other document
  formats on the strict UTF-8 path.

## ChemSema 1.0.0-beta.1 — 2026-07-19

The first public beta under the ChemSema name.

- Renamed the repository, code surfaces, packages, applications, CLI, skills,
  documentation, generated bindings, and public URLs to the ChemSema identity.
- Restarted the public brand version at `1.0.0-beta.1`, using the unique Git tag
  `chemsema-v1.0.0-beta.1` so existing historical tags remain untouched.
- Added permanent compatibility handling and automated monitoring for earlier
  GitHub repository and Pages links.
- Included the latest editor, Office/OLE, agent CLI, CDXML/CDX, rendering, and
  chemical-label improvements from the previous beta line.
- Added a shared Rust chemical-semantics layer for SMILES import and canonical
  isomeric SMILES analysis, common valence/aromaticity checks, implicit
  hydrogens, tetrahedral and E/Z stereochemistry, molecular properties, and
  official IUPAC InChI/InChIKey generation across native and browser paths.
- Added repository citation and archival metadata, including the author ORCID,
  a GitHub citation entry, and Zenodo release metadata.
- Made local verification compare generated WASM artifacts before and after the
  build, so an intentionally dirty development worktree is not mistaken for a
  stale generated runtime.

## 1.0.0-beta.8

ChemDraw-aligned chemical-label interpretation and attachment geometry, stable
semantic CDX/CDXML round trips, short-bond rendering fidelity, and shared
Windows/Ubuntu CLI delivery across every ChemSema shell.

- Refined structural chemical-label tokenization and display direction. Formula
  groups such as `C10H21` keep their internal element/count structure and move
  as a unit when an attached label reverses; multi-group formulas reverse by
  chemical group, so a right-connected `C10H21O3` can display as `O3C10H21`
  while preserving source text and numeric subscripts. Known abbreviations such
  as `TFA`, oxidation-state labels such as `Cu(II)`, and metal-leading chemical
  text now have explicit interpretation rules instead of relying on plain-text
  fallback.
- Tightened label chemistry diagnostics and implicit-hydrogen behavior. The
  valence parser no longer invents hidden formal charges or expanded octets for
  second-period elements, distinguishes invalid valence from uninterpretable
  text, and preserves explicit per-node hydrogen-count overrides, including an
  authored `NumHydrogens="0"`, through CDXML round trips.
- Reworked CDX/CDXML rich-text import and export so justification, reversal,
  normal/subscript/superscript runs, font sizes, faces, and the standard color
  table survive open-save-open cycles predictably. Native CCJS/JSON now keeps
  readable semantic values such as font families, explicit text styles, hex
  colors, and named attachment targets; source-format font, face, color, and
  attachment indices are reconstructed only at the CDX/CDXML boundary.
- Added semantic support for ChemDraw `BeginAttach` and `EndAttach`. Explicit
  internal label attachments resolve against the current glyph geometry and
  are written back on export. When source attachment metadata is absent or
  invalid, the engine retains its structural-node/main-bond anchor instead of
  guessing the nearest glyph, keeping fallback behavior deterministic.
- Corrected attached-label geometry for coordination and formula labels.
  Same-row multi-glyph clipping now uses each glyph's own outline and vertical
  overlap, so low parentheses, descenders, and script text cannot pull an
  unrelated character's clipping region or bond anchor downward. This also
  restores ChemDraw-like vertical alignment for labels such as `N`, `Pd`, and
  `P(OPh)2` without per-molecule special cases.
- Corrected double-bond retreat near labels. Centered-double sub-lines compute
  clipping on their actual offset axes and share the larger endpoint retreat,
  preserving equal visible lengths; side doubles keep the main bond on the
  structural/attachment axis and no longer shift a terminal label by half the
  parallel-line spacing.
- Corrected automatic side placement for unfrozen double bonds when a newly
  added substituent produces an exact signed-side tie. Editing follows the side
  of the newest bond while context-free import remains deterministic, and the
  GUI reflects the new placement immediately without an ACS/default style
  refresh.
- Matched ChemDraw's adaptive distribution for very short dashed bonds,
  including the transition from three dashes to two and then one visible
  segment. OCR comparison rules now treat a one-segment dashed bond as visually
  indistinguishable from its corresponding single bond without changing the
  underlying bond semantics.
- Rebuilt and cross-checked the shared engine for the browser, Windows desktop,
  HarmonyOS ArkWeb shell, native CLI, and Codex skills. Web and Harmony packages
  now carry the same WASM artifact, while desktop and CLI release builds consume
  the same Rust engine behavior and the Harmony build can produce an unsigned
  HAP in clean environments without overwriting local signing configuration.
- Added first-class Ubuntu/WSL CLI validation and Linux skill packaging. The
  ChemSema CLI skill now bundles both `win-x64` and `linux-x64` runtimes with
  manifest hashes, WSL build/smoke/test commands verify the Linux binary, and a
  dedicated Ubuntu CI job runs the engine/CLI tests and skill contract checks.
  Headless PNG capture also binds generic font families to an installed Linux
  face when fontconfig does not provide aliases, so text remains visible in
  minimal Ubuntu environments without Microsoft fonts.
- Expanded regression coverage for chemical-label reversal, CDX/CDXML stable
  export, explicit and inferred attachments, per-glyph clipping, centered and
  side double bonds, and short dashed bonds. The updated engine was also
  reviewed against the first 1,000 ChemSema OCR registry molecules with
  before/after pixel comparisons, without replacing database records.

## 1.0.0-beta.7

Object-grounded agent workflows, transactional CLI editing, structured
document diffs, and clearer public positioning around editable chemical
documents.

- Added the Object-Grounded Agent Layer for `chemsema-cli`. The new `bundle`
  command packages a target selector into a complete work unit with
  `target.json`, `context.json`, deterministic `capture.png`/`capture.svg`, an
  editable target-only subset, `identity-map.json`, `provenance.json`, and a
  manifest that separates editable scope from visual scope.
- Added structured document diffing with `chemsema-cli diff`. Diffs compare
  ChemSema documents by object, resource, style, molecule node, molecule bond,
  and field path identity instead of raw JSON text, making before/after reports
  usable for agent audit, regression tests, and user review.
- Added transactional command-script support for agent edits. Transaction
  envelopes can declare expected document hash/revision, required selectors,
  editable scope, create/delete policy, dry-run behavior, atomic execution, and
  postconditions such as document validity, selector existence, and
  no-unexpected-changes.
- Extended JSONL session workflows with `bundle` and transaction execution so
  long-lived agent sessions can inspect, package, dry-run, edit, and validate a
  loaded document without reparsing or switching protocols.
- Added stable bundle provenance and selector identity tracking. Bundle
  provenance records privacy-preserving source metadata, source document hash,
  source bounds, visual bounds, subset translation, editable subset counts, and
  identity-map summaries without persisting local absolute paths.
- Added a real object-grounded workflow example at
  `examples/agent/07-object-grounded-edit`. The example runs on
  `figure1.cdxml`, bundles one molecule object, captures before/after views,
  dry-runs and executes a scoped node-label edit, generates a structured diff,
  exports both the full edited document and target-only editable subsets, and
  writes an `acceptance.json` report.
- Updated CLI protocol documentation, runtime schema/capabilities discovery,
  command-script and session guides, English/Chinese CLI guides, and ChemSema
  CLI skill references for the new bundle, diff, transaction, identity-map, and
  provenance contracts.
- Refocused the English and Chinese README around editable chemical documents
  and the shared object identity that connects visual editing, Office
  workflows, CLI inspection, scoped agent editing, validation, and editable
  export.
- Improved CDXML rendering fidelity for published reaction figures. Bond
  knockouts now preserve ChemDraw-style transparent margins when bonds cross,
  label retreat is driven by glyph clipping and the imported `MarginWidth`
  profile, and attached molecule labels now use a ChemDraw-calibrated line
  anchor for terminal letters, primes, superscripts, and subscripts.
- Refreshed the public README visuals, including the ChemSema editor interface
  screenshot and the ChemDraw/ChemSema comparison assets generated from
  `figure1.cdxml` and `figure2.cdxml`.

## 1.0.0-beta.6

HarmonyOS PC groundwork, stronger GUI parity coverage, expanded agent/CLI
workflows, and another round of editor fidelity fixes.

- Added the first-stage HarmonyOS PC shell. The new `apps/chemsema-harmony` project packages the existing web viewer and shared Rust/WASM engine into an ArkWeb rawfile app for desktop-class `2in1` devices, with DevEco-oriented build/signing templates, app icons, viewer sync/build wrappers, and smoke/bridge regressions for Open/Save/New, document tabs, clipboard handoff, window titles, and rawfile assets.
- Isolated the browser, Tauri desktop, and Harmony host shells. Harmony now uses a native-frame top bar with compact system-style document tabs and no custom desktop window controls, while the Tauri shell keeps its own custom titlebar and browser mode keeps browser-native behavior.
- Expanded end-to-end GUI regression coverage for the shared viewer surface, including file open/save/export, `Ctrl+S`, internal copy/paste/cut, toolbar icons, cursor styles, selection overlays, delete-tool behavior, zoom, document style presets, host-shell isolation, toolbar health, Harmony bridge behavior, and large-CDXML speed checks.
- Moved more interaction feedback and preview behavior into the shared Rust/WASM engine path. Bond creation previews, object-coordinate previews, render-target queries, hover/focus feedback, graphic hit radii, preview dependency tracking, zoom anchoring, and empty-document rendering now behave more consistently across browser, desktop, and Harmony.
- Fixed selection and deletion edge cases across the editor. Individually selected bonds render center selection dots without white strokes, complete molecule selections suppress internal bond dots, tiny endpoint box selections match click endpoint affordances, focused double bonds degrade to single bonds before deletion, triple-bond and endpoint deletion behavior remain covered, and very short wavy bonds render without invalid amplitude ranges.
- Expanded the CLI's image and capture workflow. `convert`/`export` can write PNG, `capture` can default to a temp PNG, quiet reports return machine-readable output paths, and multi-target `capture`/`context` supports selection-only crops, repeated or semicolon selectors, explicit crop bounds, expansion, fixed output sizing, PNG/SVG output, and render metadata.
- Added GUI-parity editing commands for agents and scripts. `select-targets`, `select-all`, `clear-selection`, selection-driven arrange/style/delete/group/link operations, `plan-bond`, `plan-template`, `insert-template`, explicit double-bond placement, and bond line-weight overrides let CLI/JSONL workflows use the same semantics as the GUI instead of simulating pointer gestures.
- Improved label, OCR, and command-driven chemistry workflows. `label-query` now supports source-text and reverse visible-text lookup with connection and hydrogen-anchor semantics; direct commands can edit node charge and hydrogen labels, preserve label source text, and keep measured endpoint boxes, glyph polygons, and label text positions for imported or OCR-derived labels.
- Improved CDXML/document import fidelity for disconnected structures and style defaults. Imported CDXML molecule fragments are preserved, disconnected chemistry is split into separate molecule objects by default, crossing disconnected fixtures remain covered, and ACS/document style defaults persist through import, commands, rendering, labels, bonds, SDF/CDXML paths, and export.
- Fixed Office full-document copy behavior and refreshed desktop/Office architecture documentation so full-document and target-specific clipboard/OLE payload paths stay clear while chemical logic remains in the shared engine.
- Added the public ChemSema Codex skill suite and agent examples. The release includes CLI, development, drawing-agent, and Office skills with English/Chinese guidance, install/sync helpers, runtime discovery, session helpers, repo hygiene/build references, plus a complete reaction-agent POC with request, commands, captures, context/detail/targets JSON, Office payload output, CDXML/SVG output, and a runner.
- Updated the English and Chinese README, CLI guides, command-script and JSONL session protocol notes, CLI-GUI parity checklist, editor command history, project rules, architecture docs, and public metadata. The shared browser/Harmony WASM engine artifact was rebuilt so all shipped shells use the updated engine behavior.

## 1.0.0-beta.5

Agent-focused CLI expansion, installable entry points, and another round of
desktop/browser stabilization.

- Added formal CLI protocol/version reporting, machine-facing protocol
  contracts, an agent demo corpus, an agent POC workflow note,
  release-quality matrices, and a split `agent/` CLI module layout to keep the
  expanded agent surface maintainable.
- Packaged an installed `chemsema-cli` entry point alongside the desktop app, with installer PATH registration, `chemsema-entrypoints.json`, an installed agent guide, and `guide`/`doctor` discovery for machine callers.
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

- Added the `chemsema-cli` crate and direct engine commands for headless inspection, conversion, export, document editing, and structured JSON execution reports.
- Added `--document-json`, `--inspect-after`, and improved `.json`/`.ccjs` handling so scripts and agents can exchange ChemSema documents without driving the GUI.
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

- Fixed the Windows NSIS installer Office/OLE registration hook so it finds `chemsema-office.exe` in the installed application directory instead of assuming the old `resources` subdirectory layout.
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
