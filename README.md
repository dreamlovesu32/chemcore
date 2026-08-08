# ChemSema

[中文版](./README.zh-CN.md) | **English**

[![CI](https://github.com/dreamlovesu32/chemsema/actions/workflows/ci.yml/badge.svg)](https://github.com/dreamlovesu32/chemsema/actions/workflows/ci.yml)
[![Demo](https://img.shields.io/badge/demo-GitHub%20Pages-2ea44f)](https://dreamlovesu32.github.io/chemsema/)
[![Windows installer](https://img.shields.io/badge/Windows-installer-0078d4)](https://github.com/dreamlovesu32/chemsema/releases/download/chemsema-v1.0.0-beta.1/ChemSema_1.0.0-beta.1_x64-setup.exe)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](./LICENSE)
[![Version](https://img.shields.io/badge/version-1.0.0--beta.1-orange)](https://github.com/dreamlovesu32/chemsema/releases/tag/chemsema-v1.0.0-beta.1)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.21443042.svg)](https://doi.org/10.5281/zenodo.21443042)

ChemSema was formerly published as **ChemCore**. The Git history remains intact,
while current code, packages, documentation, and public paths use the new name.
See the [project rename and compatibility notes](./docs/migration-to-chemsema.md).

ChemSema is an open-source platform for editable chemical documents. It is
built around the daily work of drawing structures, arranging reaction schemes,
importing and exporting ChemDraw files, and moving editable chemistry through
Word and PowerPoint without losing the ability to come back and edit it.

The same Rust engine powers the browser editor, Windows desktop app, Office/OLE
integration service, and headless CLI. That shared core owns document identity,
chemical editing commands, hit testing, label semantics, CDXML/CDX
import/export, render primitives, structured diffing, and editable export. The
goal is not only to display chemistry, but to preserve enough document state
that humans, scripts, and agents can all operate on the same objects.

For AI agents, ChemSema exposes that document model directly: one selector can
move through structured inspection, local visual rendering, scoped editing,
provenance, validation, and editable export without changing identity.

```text
CDXML / CCJS
      |
targets -> object:obj_mol_001
      |-- detail.json
      |-- context.json
      |-- capture.png
      |-- bundle/
      |     |-- editable-subset.ccjs
      |     |-- identity-map.json
      |     `-- provenance.json
      |-- transaction
      |-- diff.json
      `-- target.cdxml
```

For researchers, ChemSema is a visual editor for structures, schemes, figures,
and Office documents. For agents, it is an object-grounded operation layer:
agents can target a single object, read only the necessary data, inspect the
matching local pixels, perform a range-limited edit, and prove that unrelated
parts of the document were not changed.

Windows users can try the current beta with the [ChemSema 1.0.0-beta.1 x64 installer](https://github.com/dreamlovesu32/chemsema/releases/download/chemsema-v1.0.0-beta.1/ChemSema_1.0.0-beta.1_x64-setup.exe). The installer includes the desktop app and the Windows Office/OLE integration service; it is not code-signed yet, so Windows may show a SmartScreen warning during this beta stage. Maintainer: Jiajun ZHANG, [dreamlovesu@hotmail.com](mailto:dreamlovesu@hotmail.com). Feedback, issues, real CDXML files, and contributions are very welcome. The long-term goal is to make ChemSema a free research infrastructure platform that can grow into automation, batch processing, AI-assisted research interfaces, and more carefully designed scientific software.

![ChemSema editor interface](./docs/assets/readme/product-screenshot.png)

## Object-Grounded Agent CLI

ChemSema's CLI is a protocol surface over the same engine used by the editor.
It is designed for agents that need to work on real chemical figures without
reading the whole document, guessing from screenshots, or treating exported
images as the source of truth.

The central unit is a selector such as `object:obj_mol_004` or
`node:1176604361`. That selector is stable across:

- discovery with `targets`
- raw structural inspection with `detail`
- nearby layout inspection with `context`
- deterministic local visual rendering with `capture`
- object-grounded packaging with `bundle`
- scoped command execution with `run`
- before/after auditing with `diff`
- editable subset export with `export --target`

`bundle` is the handoff point for an agent work unit. It writes
`target.json`, `context.json`, `capture.png` or `capture.svg`, a target-only
editable subset, `identity-map.json`, `provenance.json`, and `manifest.json`.
The manifest separates editable scope from visual scope: nearby arrows, labels,
or molecules may appear in the image and context, but they are not editable
unless they are part of the declared target scope.

Transactions add a safety layer before mutation. An agent can state the
expected document hash or revision, the selectors it is allowed to edit,
whether creation or deletion is allowed, whether the run is a dry run, and
which postconditions must hold. Structured `diff` then compares documents by
ChemSema ids and field paths, so the result can be audited by an agent, a test,
or a human reviewer.

```bash
chemsema-cli targets figure1.cdxml --pretty
chemsema-cli bundle figure1.cdxml --target object:obj_mol_004 --out-dir tmp/mol-bundle --context-radius 55 --capture-format png --subset-format ccjs --pretty
chemsema-cli run figure1.cdxml transaction.json --out edited.ccjs --results report.json --pretty
chemsema-cli diff before.ccjs edited.ccjs --out diff.json --pretty
chemsema-cli export edited.ccjs target.cdxml --target object:obj_mol_004 --format cdxml
```

The checked-in [object-grounded edit example](./examples/agent/07-object-grounded-edit/)
runs this loop on the public [figure1.cdxml](./figure1.cdxml) fixture: it
selects one real molecule object, changes one labeled node, verifies that no
unexpected selectors changed, and exports both the modified full document and
the modified target molecule.

Precise capture is one visible part of that interface. It crops the same visual
region a GUI selection box would cover: the requested object or multi-selection
defines the frame, while everything visible inside that frame is rendered into
the PNG/SVG. Context queries use the same target model and return the
surrounding objects with ids, directions, distances, `inside`/`partial`
selection-box membership, group ancestry, and link metadata.

Examples generated from the public [figure1.cdxml](./figure1.cdxml) fixture:

| Exact object crop | Context crop around an arrow object | Multi-target selection crop |
| --- | --- | --- |
| ![Precise CLI crop of object obj_bracket_001](./docs/assets/readme/agent-cli/precise-bracket-object.png) | ![CLI context crop around arrow object obj_line_001](./docs/assets/readme/agent-cli/line-context.png) | ![CLI multi-target crop from a bracket object and nearby text target](./docs/assets/readme/agent-cli/multi-target-bracket-text.png) |

The context command that produced the middle image also returns structured ids
for the same region. For example, around `object:obj_line_001` it reports the
target arrow itself, the partly overlapping molecule
`object:obj_cdxml_merged_molecule`, the partly overlapping condition text
`object:obj_text_008`, and the nearby lower text `object:obj_text_025`.

## Project History

ChemSema development started on April 23, 2026. The early development history is
published on the [`history/pre-public-release`](https://github.com/dreamlovesu32/chemsema/tree/history/pre-public-release)
branch so readers can follow how the project grew before the public release.

## Published Figure Comparison

The comparison below uses CDXML source files from a published paper by the
maintainer:

Jiajun ZHANG, Pinhong Chen,* Guosheng Liu*, Copper-Catalyzed Site- and
Enantioselective C–H Cyanation of Trisubstituted Allenes,
[Chin. J. Chem. 2026, 44, 1729–1734](https://onlinelibrary.wiley.com/doi/full/10.1002/cjoc.70531).

These real publication figures exercise molecule rendering, text layout,
reaction arrows, brackets, colors, radical/single-electron dots, graphical
objects, and Office-oriented vector output. The left column is exported by
ChemDraw; the right column is exported by ChemSema after importing the same
CDXML files.

These benchmark CDXML files were authored by the maintainer and are included
for reproducible rendering comparison.

![ChemDraw and ChemSema CDXML rendering comparison](./docs/assets/readme/comparison/published-cdxml-comparison.svg)

The generated SVG and Office EMF vector files for both ChemDraw and ChemSema
are kept in [docs/assets/readme/comparison](./docs/assets/readme/comparison/),
and the README comparison image is regenerated from those refreshed assets.

The original CDXML files are tracked at the repository root:
[figure1.cdxml](./figure1.cdxml) and [figure2.cdxml](./figure2.cdxml).

## Agent Skills

ChemSema includes a dedicated agent skill suite in
[ChemSemaSkills](./ChemSemaSkills/). These skills package the project-specific
workflows for the CLI protocol, command scripts, drawing-agent planning,
Office/OLE debugging, and repository development. The suite can
be flattened into Codex skills or Claude Code skills; keep the installable
skills independent, and use this README as the public entry point for
discovering them.

## Current Status

Current version: `1.0.0-beta.1`.

The Windows x64 installer is available from the
[chemsema-v1.0.0-beta.1 release](https://github.com/dreamlovesu32/chemsema/releases/tag/chemsema-v1.0.0-beta.1).
It bundles the Tauri/WebView2 desktop app, file associations, and the
Office/OLE integration service. The first-stage HarmonyOS PC shell is available
from source for DevEco Studio experiments, but it is not part of the Windows
installer. The installer is still a beta build and is not code-signed yet. The
browser demo is published through GitHub Pages:
<https://dreamlovesu32.github.io/chemsema/>.

## Product Highlights

- **Built for real research drawing workflows**: ChemSema is designed around drawing structures, arranging reaction schemes, copying into Word or PowerPoint, and returning later for editing.
- **ChemDraw-compatible files and layout habits**: CDXML/CDX import and export are first-class paths, with source structure, text, arrows, brackets, symbols, colors, and object positions preserved as much as possible.
- **One shared core for browser, desktop, and Office integration**: Editing rules, hit testing, chemical labels, render primitives, and import/export logic live in the Rust engine to avoid behavior drift between surfaces.
- **Low-latency editing**: Hover, focus, selection, drag preview, rotation, and zoom use local WASM/Rust hot paths.
- **Modern desktop behavior**: The Tauri/WebView2 desktop shell supports file open/save, drag-to-open, recent files, tabs, unsaved-change prompts, shortcuts, and Windows file association.
- **Office paste and embedding are treated seriously**: Copy operations consider native ChemSema data, CDXML, SVG, EMF, RTF/OOXML, and OLE payloads so Office display and later editing remain reliable.
- **Agent-oriented headless CLI**: The CLI can inspect documents, convert formats, query object ids and relationships, create exact PNG/SVG crops, execute JSON editing commands with audit reports, and reuse large documents through cache/session workflows.

## Implemented Capabilities

- **CDXML/CDX import and export**: The Rust engine parses and writes ChemDraw-oriented formats, maps them into the ChemSema document model, and keeps enough drawing information for re-rendering and round trips.
- **Unified document and rendering model**: The document model, runtime scene, hit testing, selection state, and render primitives are defined in the engine while the front end focuses on events and display.
- **Scalable native documents**: CCJS 0.2 keeps flat entities with explicit hierarchy/relations; CCJZ adds hashed chunks, visible-region loading, unchanged-entry and attachment reuse, browser Zip64, and structured chemical/round-trip validation.
- **Complex bond geometry**: Single, double, triple, bold, dashed, solid wedge, hashed wedge, label clipping, bond joins, crossing gaps, and ChemDraw-style template parameters are implemented.
- **Arrows and graphical objects**: Reaction arrows, equilibrium arrows, hollow arrows, curved arrows, brackets, lines, shapes, and symbols are supported and continue to be aligned with ChemDraw interaction details.
- **Selection, drag, rotation, and arrangement**: Object-level and partial molecule selection, multi-select drag preview, rotation, flip, alignment, distribution, color application, and undoable command history are available.
- **Text and label editing**: Free text, endpoint element replacement, label editing, text selection, style synchronization, and the behavioral distinction between chemical labels and free text are handled.
- **Implicit hydrogen and element rules**: Main-group auto-hydrogen logic, valence counting, charge effects, and period-specific rules are centralized in the engine.
- **Abbreviation and group recognition**: Common abbreviations such as Me, Et, Ph, CN, NO2, Boc, Ts, TMS, TBDMS, and TBDPS are recognized and treated as groups during flipping, formula, and mass calculations.
- **Formula and mass statistics**: Selected structures can report formula weight, exact mass, and expanded abbreviation composition.
- **Desktop and Office foundations**: Browser, desktop, and Office/OLE boundaries are established, including Windows clipboard, EMF preview, and Word OOXML/OLE payload paths.

## Design Details

ChemSema focuses on the small details that decide whether a chemistry editor is
pleasant enough for real daily use. These rules live in the shared Rust engine
where possible, so the browser, desktop shell, SVG/EMF output, and Office paths
consume the same geometry.

### Text Clipping

Endpoint labels are split into styled runs and line runs, then the engine builds per-glyph clip
polygons from font size, baseline, subscript/superscript shifts, and character
advance data. When a bond is rendered, its endpoint ray intersects the glyph
polygon edges and picks the farthest exit point from the atom. Glyph polygons
already include the optical clipping allowance; renderers must not add a
separate label margin.

Common uppercase letters and symbols use tuned height-centered glyph polygons
generated from Arial, including cases such as `N`, `I`, and `+`. Unknown
uppercase letters fall back to conservative uppercase profiles, and full-width
or CJK characters use square profiles. If a document has no glyph polygons, the
engine falls back to the label bounding box.

### Label Grouping

Chemical labels are grouped by uppercase-led fragments and known abbreviations.
For example, `OTMS` is grouped as `O` + `TMS`, so a right-side label becomes
`TMSO`. Anchors are assigned to
the chemically meaningful terminal letter.

### Bond Joins

Shared endpoints are computed as real polygons. Each bond endpoint is converted
into an axis, normal, left/right contour lines, and half width. For two-bond
joins, the engine intersects inner-inner and outer-outer contours and stores
endpoint profiles for each bond. For three or more incident bonds, contours are
sorted by polar angle and only adjacent pairs are intersected, producing a ring
of profiles around the node. Sharp angles are limited by a miter cap.

### Crossing Gaps

Non-endpoint crossings use a separate knockout pass. Within a molecule
fragment, bonds are rendered in order and later bonds are treated as the upper
layer. Before drawing an upper bond, the engine checks it against already drawn
lower bonds, skips shared endpoints and near-parallel cases, and inserts a
white knockout polygon at true interior intersections. The gap length is
compensated by the crossing angle's sine, and the width uses the upper bond's
visual width plus its template `marginWidth`.

### Infinite Canvas

The editor uses a runtime `viewBox`.
The SVG `viewBox` is expressed in document-world coordinates, while CSS size is
computed from `pt -> css px -> zoom`; the scroll container is just a window into
that world. Empty documents start with a centered buffer around the visible
area. After each render, document bounds are compared with the current viewBox;
if content approaches an edge, the viewBox expands in that direction and
left/top expansions compensate scroll delta so the scene remains visually stable.

### Stable Interaction

Selection boxes, hover state, drag previews, rotation handles, curved-arrow
handles, text boxes, and graphical objects are kept stable in dense CDXML
documents. Selected content suppresses internal hover, drag previews follow
locally before committing to the engine, and each desktop tab preserves its
document, zoom, and runtime view state.

Architecturally, ChemSema keeps the rules that affect consistency inside the
engine wherever possible: hit testing, selection ranges, hover behavior,
drawing geometry, text clipping, bond joins, implicit hydrogen, abbreviation
recognition, CDXML parsing, and export rendering are shared by the browser,
desktop, and Office paths.

Complex CDXML compatibility, Office copy/paste, OLE embedding, and ChemDraw
format fidelity are still being actively refined. Reports with concrete files,
screenshots, or real Office workflows are especially useful.

## Welcome

If you use ChemDraw heavily, care about free research infrastructure, or are
interested in AI-assisted scientific software development, you are very welcome
to try ChemSema, open issues, join discussions, or contribute code.

The most useful feedback usually comes in two forms: concrete files with
screenshots that help align ChemDraw display and interaction, and real writing
workflows that expose copy/paste, Office editing, layout, or export problems.
ChemSema is built to become a tool that can enter daily research work.

Please contact the maintainer using the email address at the top of this README.

## Repository Layout

```text
chemsema/
  crates/chemsema-engine/          Rust document, editing, rendering, CDXML, and WASM core
  crates/chemsema-cli/             Headless file inspection, conversion, export, and command runner
  crates/chemsema-desktop-service/ Native desktop engine sessions and file helpers
  apps/chemsema-desktop/           Tauri Windows desktop application
  apps/chemsema-office/            Windows Office/OLE integration server
  viewer/                          Browser editor host and generated WASM package
  docs/                            Public rules, specs, architecture notes, and assets
  ChemSemaSkills/                  Codex/Claude skills for ChemSema agent and development workflows
  examples/                        Example ChemSema native documents
  fixtures/                        Public synthetic CDXML regression fixtures
  scripts/                         Build, verification, and regression helpers
  shared/                          Shared JSON data consumed by Rust and viewer code
```

## Prerequisites

- Rust stable with the MSVC toolchain on Windows
- Node.js and npm
- Python 3 for local static serving and some optional analysis scripts
- `wasm-pack` is installed automatically by `npm run build:engine-wasm` when needed
- Windows is required for the desktop shell and Office/OLE integration paths

## Quick Start

```bash
npm install
cargo test
npm run build:engine-wasm
```

Run the browser editor from the repository root:

```bash
python -m http.server 8765 --bind 127.0.0.1 --directory .
```

Then open:

```text
http://127.0.0.1:8765/viewer/
```

Run the Windows desktop shell:

```bash
npm run desktop:dev
```

Run the headless file CLI:

```bash
npm run cli -- inspect figure1.cdxml --pretty
npm run cli -- convert figure1.cdxml tmp/figure1.svg
npm run cli -- convert figure1.cdxml tmp/figure1.png --scale 6
npm run cli -- targets figure1.cdxml --pretty
npm run cli -- capture figure1.cdxml --target object:obj_bracket_001 --out tmp/bracket.png --width 1200 --expand 8 --pretty
npm run cli -- context figure1.cdxml --target object:obj_line_001 --radius 45 --expand-left 10 --expand-right 10 --expand-top 34 --expand-bottom 34 --capture-out tmp/line-context.png --out tmp/line-context.json --pretty
npm run cli -- new commands.json --out generated.cdxml --results results.json --pretty
npm run cli -- run input.cdxml commands.json --out edited.cdxml --results results.json --document-json after.ccjs --pretty
```

Build release binaries:

```bash
npm run desktop:build-fast
cargo build -p chemsema-office -p chemsema-cli --release
```

Register the Office/OLE integration for the current user:

```bash
npm run office:register-dev
```

Unregister it:

```bash
npm run office:unregister-dev
```

## Verification

The main verification command is:

```bash
npm run verify
```

It runs Rust tests, rebuilds the browser engine WASM, checks viewer JavaScript
syntax, and verifies that generated `viewer/engine` files are synchronized.

Useful focused commands:

```bash
npm test
cargo test -p chemsema-engine
cargo test -p chemsema-office
cargo test -p chemsema-engine public_cdxml_fixture_svg_golden_snapshots_match --test render_document
npm run build:engine-wasm
node --check viewer/app.js
```

Public synthetic CDXML fixtures live in [fixtures/cdxml](./fixtures/cdxml/),
with matching golden SVG snapshots in
[fixtures/expected/svg](./fixtures/expected/svg/). The comparison and snapshot
workflow is documented in [Rendering Comparison And Regression Assets](./docs/rendering-comparison.md).

The license-clear public round-trip corpus is documented in
[benchmarks/public-cdxml](./benchmarks/public-cdxml/README.md). It pins 413
CDXML/CDX files from five upstream projects without committing their source
files into this repository:

```bash
npm run benchmark:cdxml-public:fetch
cargo build -p chemsema-cli
npm run benchmark:cdxml-public
```

The current baseline has 404 exact three-generation round trips, with the other
nine files explicitly classified as safe sanitization, lossless normalization,
expected rejection, or transport-encoded input; unexpected semantic and count
drift are both zero.

Some scripts compare output against locally installed desktop applications or
Office. Those flows are optional and may require Windows-specific software,
local documents, or `CHEMSEMA_PYTHON` to point at a Python environment with the
needed analysis packages.

## Citation

If ChemSema contributes to published research, cite the archived software
release used for the work. The current version DOI is
[10.5281/zenodo.21443043](https://doi.org/10.5281/zenodo.21443043), and the
all-versions concept DOI is
[10.5281/zenodo.21443042](https://doi.org/10.5281/zenodo.21443042). Citation
metadata and the author ORCID are available in [CITING.md](./CITING.md). GitHub
also reads [CITATION.cff](./CITATION.cff) for its **Cite this repository** entry.

## Design Documents

- Abbreviation recognition rules: [English](./docs/abbreviation-recognition-rules.md) / [中文](./docs/abbreviation-recognition-rules.zh-CN.md)
- Architecture overview: [English](./docs/architecture.md) / [中文](./docs/architecture.zh-CN.md)
- Bond rendering rules: [English](./docs/bond-rendering-rules.md) / [中文](./docs/bond-rendering-rules.zh-CN.md)
- Charge and radical symbol rules: [English](./docs/charge-radical-symbol-rules.md) / [中文](./docs/charge-radical-symbol-rules.zh-CN.md)
- Agent POC workflow: [English](./docs/agent-poc-workflow.md) / [中文](./docs/agent-poc-workflow.zh-CN.md)
- ChemSema agent skills: [English](./ChemSemaSkills/README.md) / [中文](./ChemSemaSkills/README_ZH.md)
- ChemSema CLI command guide: [English](./docs/chemsema-cli-guide.md) / [中文](./docs/chemsema-cli-guide.zh-CN.md)
- CLI/GUI parity checklist: [docs/cli-gui-parity-checklist.md](./docs/cli-gui-parity-checklist.md)
- CLI protocol contracts: [docs/protocol](./docs/protocol/README.md)
- Document commit contract: [English](./docs/document-commit-contract.md) / [中文](./docs/document-commit-contract.zh-CN.md)
- Editor command history: [English](./docs/editor-command-history.md) / [中文](./docs/editor-command-history.zh-CN.md)
- Current CCJS format v0.2: [English](./docs/format-v0.2.md) / [中文](./docs/format-v0.2.zh-CN.md)
- CCJS architecture, comparison, and rationale: [中文](./docs/ccjs-architecture-and-format-rationale.zh-CN.md)
- CCJS v0.2 implementation status and stable-release gates: [中文](./docs/ccjs-v0.2-stability-architecture.zh-CN.md)
- CCJZ container, Document Patch, and Recovery Journal contracts: [docs/protocol](./docs/protocol/README.md)
- Legacy format v0.1: [English](./docs/format-v0.1.md) / [中文](./docs/format-v0.1.zh-CN.md)
- Glyph clipping rules: [English](./docs/glyph-clip-polygons.md) / [中文](./docs/glyph-clip-polygons.zh-CN.md)
- Glyph kernel: [English](./docs/glyph-kernel.md) / [中文](./docs/glyph-kernel.zh-CN.md)
- Implicit hydrogen rules: [English](./docs/implicit-hydrogen-rules.md) / [中文](./docs/implicit-hydrogen-rules.zh-CN.md)
- Project rules: [English](./docs/project-rules.md) / [中文](./docs/project-rules.zh-CN.md)
- Rendering comparison and regression assets: [English](./docs/rendering-comparison.md) / [中文](./docs/rendering-comparison.zh-CN.md)
- Rust engine architecture: [English](./docs/rust-engine-architecture.md) / [中文](./docs/rust-engine-architecture.zh-CN.md)
- Text symbols and glyph profiles: [English](./docs/text-symbol-glyph-profile-rules.md) / [中文](./docs/text-symbol-glyph-profile-rules.zh-CN.md)
- Valence-driven label recognition: [English](./docs/valence-label-recognition-rules.md) / [中文](./docs/valence-label-recognition-rules.zh-CN.md)
- Windows desktop and Office architecture: [English](./docs/windows-desktop-office-architecture.md) / [中文](./docs/windows-desktop-office-architecture.zh-CN.md)
- Release quality matrix: [English](./docs/release-quality.md) / [中文](./docs/release-quality.zh-CN.md)
- Release notes: [CHANGELOG.md](./CHANGELOG.md) / [中文](./CHANGELOG.zh-CN.md)
- Roadmap: [English](./ROADMAP.md) / [中文](./ROADMAP.zh-CN.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)

## License

ChemSema is licensed under the Apache License, Version 2.0. See
[LICENSE](./LICENSE) and [NOTICE](./NOTICE).
