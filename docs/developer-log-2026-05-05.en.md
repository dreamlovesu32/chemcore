# Chemcore Developer Log - 2026-05-05

Author: Jiajun Zhang

Time range: 2026-05-05 00:00 to 2026-05-05 23:59, Asia/Shanghai

Baseline commit: `56cdd4a feat: adopt ccjs and ccjz document extensions`

Workspace: `D:\chemcore`

### Summary

Today's main thread was moving the project from a WSL handoff into a Windows-native development environment and confirming that `chemcore-engine`, the WASM viewer, npm scripts, Git, VS Code, and browser save flow can work directly on Windows. The architecture direction remains unchanged: Rust `chemcore-engine` is authoritative for the document model, editing semantics, import/export, hit testing, and render primitives; `viewer/` is only the browser adapter.

The viewer save failure was also fixed today. The direct cause was that `viewer/document_flow.js` used `CHEMCORE_TEXT_EXTENSION` without importing it, so clicking Save as threw a `ReferenceError`. The save order was also hardened so the browser save picker opens first inside the user click gesture, and content generation/compression happens afterward. This avoids Chromium File System Access API failures caused by losing transient user activation during async compression.

The temporary root-level `HANDOFF-2026-05-05.md` has been folded into this log, including migration context, recent commits, handoff notes, known risks, and future direction. That temporary handoff file has been deleted and is no longer kept as a separate handoff document.

### Migration State Received Today

The migration target is `D:\chemcore`, corresponding to `/mnt/d/chemcore` in WSL. The original WSL source directory was `/home/jiajun/chemcore`. The migration was a full copy, including `.git`, `target/`, `node_modules/`, `tmp/`, generated `viewer/engine` artifacts, and the worktree state at the time.

Both worktrees were aligned at:

```text
56cdd4a feat: adopt ccjs and ccjz document extensions
```

The two development commits that needed to be carried forward into today's log were:

```text
0e4e0e1 feat: improve cdxml rendering fidelity
56cdd4a feat: adopt ccjs and ccjz document extensions
```

`0e4e0e1` was the largest development commit of the day, touching 49 files with about 4462 insertions and 542 deletions. It was not a narrow patch; it was a broad fidelity pass over ChemDraw/CDXML behavior, arrow semantics, structural labels, glyph profiles, text symbols, and regression coverage.

The CDXML color system was centralized:

- Added `crates/chemcore-engine/src/cdxml/colors.rs` to own CDXML `<colortable>` parsing, color id mapping, and export.
- Made ChemDraw color ids explicit: `color="0"` is foreground, default black; `bgcolor="1"` is background, default white; user color-table entries start at id `2`.
- Parsed RGB fractions in the `0..1` range and rounded them into `#rrggbb`, avoiding drift between floating-point CDXML colors and integer hex colors.
- Routed CDXML import for lines, shapes, text objects, text runs, fragment labels, page background, and object styles through one color resolver.
- Changed CDXML export to collect colors actually used by the document, write one stable `<colortable>`, and map object colors back to stable ids.
- Added color-focused tests for duplicate color slots, Default/ACS samples, non-white page backgrounds, and color preservation after re-import.

Arrow model and rendering semantics were made much richer:

- The internal document model now carries thicker `arrowHead` and `arrowGeometry` data instead of relying on a few frontend size buckets.
- `kind` records solid, hollow, and open hollow arrows; `head` and `tail` record full, left, right, and none endpoint styles.
- `length`, `centerLength`, and `width` are stored and aligned with ChemDraw `HeadSize`, `ArrowheadCenterSize`, and `ArrowheadWidth`.
- `curve` and ellipse-arc geometry are stored for curved arrows and curved double arrows.
- `noGo` records cross/hash no-go marks, and `bold` records bold arrow semantics.
- `crates/chemcore-engine/src/render_objects/arrows.rs` now renders solid arrows, hollow arrows, open arrows, half arrows, double-headed arrows, curved arrows, bold arrows, and no-go marks.
- CDXML import reads arrow size, type, endpoint, curve, geometry, and color parameters; CDXML export writes `ArrowheadType`, `ArrowheadHead`, `ArrowheadTail`, `ArrowheadCenterSize`, `ArrowheadWidth`, curve geometry, and color.
- Selection/editing paths were updated in `editing/arrows.rs`, `engine/arrows.rs`, `engine/select/arrows.rs`, and `editing/geometry.rs`, keeping hover, drag, selected-style updates, and scaled arrow geometry consistent.
- Tests now cover arrowhead size floors, arrow dimensions relative to line width, independent open/hollow templates, half-arrow visual sides on curves, and stable CDXML arrow fixtures after export/re-import.

Structural labels, abbreviation handling, and valence recognition also advanced:

- `abbreviation.rs`, `abbreviation/expansion.rs`, and `abbreviation/valence.rs` continued moving chemical abbreviations, open valence, terminal group, and bridge group rules into the Rust engine.
- Molecule fragment labels are no longer treated as ordinary text objects, avoiding a CDXML import path where structural labels and free text were mixed together.
- Structural labels can carry `lineRuns`, preserving multi-line and multi-run labels such as `H` above `N`.
- Source runs and normalized display runs were separated: source-file run data stays in import metadata, while editing/display uses chemically normalized runs.
- Text-edit internals under `engine/text_edit` were updated across geometry, labels, layout, and runs so endpoint labels, text objects, reopened edit sessions, and caret/selection geometry follow the Rust glyph kernel more closely.
- Tests were added or expanded for terminal abbreviations, two-connection bridge abbreviations, charged B/N/O exceptions, P/S implicit hydrogen rules, halogen alternating implicit hydrogen rules, right-side label anchors, and reopened text-edit bbox/anchor precision.

The glyph kernel, text symbols, and viewer text-rendering path received a full pass:

- Added `shared/text_symbols.json` as shared data for common text and chemical typography symbols.
- Added `viewer/text_symbol_palette.js`, allowing the viewer to show a text-symbol palette and insert selected symbols into the active text editor or arm the text tool for insertion.
- Added `scripts/generate-glyph-profiles.py` for generating/updating shared glyph profiles.
- Added `scripts/text-symbol-regression.mjs` for text-symbol regression checks.
- Updated `shared/glyph_profiles.json` so glyph advance, ink box, background box, polygon, and clipping data stay aligned with the Rust kernel.
- Updated `glyph_kernel.rs`, `viewer/text_metrics.js`, `viewer/primitive_dom_renderer.js`, `viewer/object_fallbacks.js`, and `viewer/styles.css` so the viewer consumes engine/shared-profile output rather than inventing text measurement locally.
- Added `docs/text-symbol-glyph-profile-rules.zh-CN.md` to document text-symbol and glyph-profile maintenance rules.
- Updated `docs/glyph-kernel.md`, preserving the rule that the Rust glyph kernel is authoritative for advance, ink box, background box, glyph polygon, and label clipping.

Rendering, format fields, and import boundaries were strengthened:

- `document.rs` added fields for arrows, shapes, text, and style payloads so JSON can preserve real CDXML semantics instead of only the subset the viewer currently draws.
- `render.rs`, `render_bonds.rs`, `render/bond_metrics.rs`, `render/style_payload.rs`, `render_objects/text.rs`, `render_primitives.rs`, and `render_svg.rs` continued consolidating render primitive output.
- Shape objects gained more ChemDraw style fidelity, including rectangle, round rectangle, ellipse, shadowed, shaded, and dashed geometry/style behavior.
- Small brackets, shapes, and text bboxes no longer get inflated by unreasonable fixed minimums after import.
- ACS Document 1996 and Default drawing parameters remain separate instead of treating ACS as a simple scale of Default.
- The JSON import boundary migrates legacy aligned text boxes, fills default arrow geometry, and normalizes text/shape payloads so older files do not open with missing fields.
- `docs/format-v0.1.md` and `docs/format-v0.1.zh-CN.md` were updated with these model fields, and `docs/project-rules.zh-CN.md` was updated to reinforce the engine/viewer ownership boundary.

Test assets and validation coverage were expanded significantly:

- `crates/chemcore-engine/tests/render_document.rs` gained many CDXML, SVG, arrow, shape, glyph, text, and import/export stability cases.
- `crates/chemcore-engine/tests/bond_tool.rs` covers editor-tool behavior for arrows, hover/drag, templates, shapes, select, text, symbols, and brackets.
- `crates/chemcore-engine/tests/text_tool.rs` covers endpoint labels, plain text objects, text runs, reopened editing, caret, selection, and unrecognized-abbreviation red boxes.
- Fixture stability checks now cover CDXML import -> export -> import render/SVG stability and preservation of object semantics in local `tmp/` fixtures.
- `viewer/engine/chemcore_engine_bg.wasm` was regenerated with the Rust engine changes so browser-visible behavior matches the core.

`56cdd4a` was the second development commit of the day, touching 11 files with about 176 insertions and 77 deletions. It moved Chemcore's own file entry points from generic `.json` to `.ccjs` / `.ccjz`, and updated viewer open/save flow accordingly.

Native format naming and responsibilities were clarified:

- `.ccjz` is the default product save format: gzip-compressed chemcore JSON, intended for everyday user saves and sharing.
- `.ccjs` is the readable debug format: plain-text chemcore JSON, intended for inspection, diffs, and reproductions.
- Chemcore native files are no longer exposed as plain `.json`, avoiding confusion with arbitrary JSON files and reducing the chance users treat the format as generic JSON data.
- The example file was renamed from `examples/document-v0.1.json` to `examples/document-v0.1.ccjs`, keeping examples aligned with the new format policy.

Viewer file flow was refactored around centralized format handling:

- `viewer/file_io.js` added `.ccjs` / `.ccjz` extension constants, MIME constants, filename-format detection, base-name handling, compression helpers, and decompression helpers.
- `.ccjz` uses browser `CompressionStream` / `DecompressionStream`; the content is still chemcore JSON, stored as gzip.
- The open accept list now includes `.ccjz`, `.ccjs`, `.cdxml`, and their MIME types so browser file pickers can filter correctly.
- `documentTitleForFileName` now defaults to `.ccjz`, deriving a safe filename from the document title or current file name.
- `viewer/document_flow.js` now decides whether gzip decompression is needed from filename/MIME, then decides whether the content is CDXML from content and file metadata.
- Save as supports `.ccjz`, `.ccjs`, `.cdxml`, and `.svg`; unknown save extensions default to `.ccjz`.
- `viewer/app.js` was adapted to the new example extension and file-flow API.

Docs and project rules were updated together:

- `README.md` and `README.zh-CN.md` now describe `.ccjs/.ccjz` as the native formats.
- `docs/format-v0.1.md` and `docs/format-v0.1.zh-CN.md` specify `.ccjz` as gzip JSON and `.ccjs` as debug JSON.
- `docs/project-rules.zh-CN.md` records the native-format rule: `.ccjz` is the default save format and `.ccjs` is the debug format.
- `docs/rust-engine-architecture.zh-CN.md` and `docs/viewer-rendering-report.zh-CN.md` were updated so old `.json` terminology no longer remains in developer rules.
- This format migration also prepares the future Tauri/desktop shell: the product can associate `.ccjz/.ccjs` instead of taking over ordinary `.json`.

### Windows-Native Environment Rebuild

The Windows toolchain was rebuilt and normalized today. The guiding rule was to install tools onto D drive where practical; if existing system tools were not new enough or not suitable, the Windows-native path was updated rather than forcing compatibility.

Main tools verified or configured:

```text
Git:                 D:\Git
Git version:         2.54.0.windows.1
Git Bash:            D:\Git\bin\bash.exe
Git Bash version:    GNU bash 5.3.9

Rust cargo home:     D:\Rust\cargo
Rust rustup home:    D:\Rust\rustup
Rust toolchain:      stable-x86_64-pc-windows-msvc
rustc:               1.95.0
cargo:               1.95.0
Rust targets:        x86_64-pc-windows-msvc, wasm32-unknown-unknown
wasm-pack:           0.14.0

Node installed path: D:\nodejs-24.15.0
Node installed ver.: 24.15.0
npm installed ver.:  11.12.1

Playwright browsers: D:\ms-playwright
VS Build Tools:      C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools
VS Build version:    17.14.36717.8 / 17.14.21
```

One operational detail remains important: the already-running Codex/PowerShell process may still resolve the old `D:\nodejs` and WSL `bash.exe`, because process PATH is fixed at launch time. The user PATH has been updated to put these entries first:

```text
D:\nodejs-24.15.0
D:\Rust\cargo\bin
D:\Git\cmd
D:\Git\bin
D:\Git\usr\bin
```

New PowerShell windows, VS Code terminals, or reloaded development sessions should pick up the newer Node, Rust, and Git Bash. npm `script-shell` is set to:

```text
D:\Git\bin\bash.exe
```

So scripts such as `npm run build:engine-wasm` and `npm run verify`, which call `bash scripts/*.sh`, should use Git Bash instead of WSL bash.

### Git, VS Code, And Windows Filesystem Handling

Git was configured as follows:

```powershell
git config --global core.autocrlf false
git config --global core.eol lf
git config core.filemode false
```

These settings avoid CRLF churn, avoid NTFS/WSL file-mode noise, and make `D:\chemcore` the main Windows-native development directory instead of editing the old WSL copy through `\\wsl$`.

VS Code's built-in Git support is enough for tracking status; no extension is required. `%APPDATA%\Code\User\settings.json` was updated with:

```json
{
  "git.path": "D:\\Git\\cmd\\git.exe",
  "git.enabled": true
}
```

GitLens is optional, not required. If VS Code still does not show Git status, verify that it opened local `D:\chemcore` rather than a WSL remote window, then reload VS Code.

The Windows copy contained WSL/Windows metadata filenames using a private-use colon glyph, such as `*Zone.Identifier` and `*mshield`. Those untracked files under `compare/` were removed, and `.gitignore` now ignores:

```text
*Zone.Identifier
*mshield
```

### Dependency Install And Validation Results

`npm install` was run. npm 11 wrote root package metadata into `package-lock.json`, so the lockfile gained a small metadata-only diff. The install showed cleanup warnings for old `.bin` temporary symlinks, but they did not block installation or validation.

Validation completed:

```text
cargo test
npm test
npm run build:engine-wasm
node --check viewer/app.js
node --check viewer/document_flow.js
```

`cargo test` passed with:

- library unit tests: 44 passed
- `bond_tool`: 141 passed
- `render_document`: 98 passed, 2 ignored
- `text_tool`: 36 passed
- doc-tests: 0

`npm test` passed and runs:

```text
cargo test && node --check viewer/app.js
```

`npm run build:engine-wasm` passed. After the Windows-native rebuild, `viewer/engine/chemcore_engine_bg.wasm` changed. Rebuilding twice produced the same SHA-256:

```text
BC3A87EF5A2310D622F6E58D44DB351C002348A2205D527FBDDF8A52036EA62C
```

So the difference is stable, not random. `npm run verify` passed its tests, WASM build, and JS syntax checks, then failed only at the generated-artifact sync check because `viewer/engine/chemcore_engine_bg.wasm` differs from the repository's previous generated artifact. That WASM diff should be reviewed as a Windows-native rebuild artifact before committing.

### Local Server And Viewer State

A local static file server was started from `D:\chemcore`:

```text
URL:      http://127.0.0.1:8766/viewer/
Process: 8632
Runtime: C:\Python314\python.exe
```

Port `8766` was used because `8765` already had a listener. `Invoke-WebRequest http://127.0.0.1:8766/viewer/` returned `200 OK`.

### Save Fix

The user reported that the viewer could not save. The investigated path was:

- The top save button in `viewer/index.html` uses `data-command="save"`.
- `viewer/editor_bindings.js` maps that command to `saveCurrentDocumentAs`.
- `viewer/document_flow.js` used `CHEMCORE_TEXT_EXTENSION` in the `.ccjs` accept list without importing it.

Direct failure:

```text
ReferenceError: CHEMCORE_TEXT_EXTENSION is not defined
```

Fix:

- Imported `CHEMCORE_TEXT_EXTENSION` in `viewer/document_flow.js`.
- Reordered `saveCurrentDocumentNative`, `saveCurrentDocumentCdxml`, and `saveCurrentDocumentSvg` so browsers with `showSaveFilePicker` open the save picker first, then generate/export/compress content.
- Updated the `app.js` query in `viewer/index.html` to `20260505-savefix` to reduce stale-browser-cache risk.

Verification:

```text
node --check viewer/document_flow.js
node --check viewer/app.js
```

Playwright was also used with a fake injected `showSaveFilePicker`; clicking the Save button successfully reached the write path:

```json
{
  "suggestedName": "Untitled.ccjz",
  "write": {
    "constructorName": "Uint8Array",
    "byteLength": 524
  },
  "closed": true
}
```

### Worktree Changes Today

Before this log was written, today's main modified files were:

```text
.gitignore
package-lock.json
viewer/document_flow.js
viewer/engine/chemcore_engine_bg.wasm
viewer/index.html
docs/developer-log-2026-05-05.zh-CN.md
docs/developer-log-2026-05-05.en.md
```

`HANDOFF-2026-05-05.md` has been absorbed into this log and deleted from the repository root.

Meaning of each change:

- `.gitignore`: ignore private-use colon metadata files created by Windows/WSL copying.
- `package-lock.json`: npm 11 root package metadata.
- `viewer/document_flow.js`: fix missing save constant import and open the save picker before content generation.
- `viewer/index.html`: update `app.js` cache-busting query.
- `viewer/engine/chemcore_engine_bg.wasm`: stable Windows-native WASM rebuild artifact.
- `docs/developer-log-2026-05-05.zh-CN.md`: the Chinese developer log.
- `docs/developer-log-2026-05-05.en.md`: the English developer log.

### Follow-Up Notes

Windows development should happen directly in `D:\chemcore`. The old WSL copy can remain as a reference backup, but dual-track uncommitted development should be avoided. Commit or explicitly back up before switching environments.

If Windows shows massive line-ending diffs, stop and confirm `core.autocrlf=false` and `core.eol=lf` before continuing. If massive file-mode diffs appear, confirm local repo `core.filemode=false`.

If `npm run verify` fails only because `viewer/engine` generated artifacts differ, first confirm that Rust tests, WASM build, and JS syntax checks passed, then decide whether to commit the generated artifact.

`.ccjz` depends on browser `CompressionStream` / `DecompressionStream`. Modern Chromium supports them; future Tauri/WebView2 packaging should verify support or provide native compression in the Tauri backend.

CDXML/SVG fidelity should continue using fixtures under `tmp/` and `compare/` for import, save, re-import, and SVG comparison. Important checks include object counts, object types, colors, text runs, structural labels, arrow parameters, relative positions, line widths, font sizes, and SVG primitives.

For a future desktop shell, Tauri remains the recommended path. Phase 1 should only provide shell capabilities: window, menu, shortcuts, open/save dialogs, file associations, recent files, drag-and-drop open, and installer packaging. Chemistry editing logic should stay out of the shell. Phase 2 can consider direct Rust engine calls from the Tauri backend, native compression, thumbnails, and OS integration.
