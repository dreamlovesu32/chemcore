# Chemcore Developer Log - 2026-05-07

Author: Jiajun Zhang

Time range: 2026-05-07 00:00 to the early hours of 2026-05-08, Asia/Shanghai

Note: By today's working rhythm, the commits made shortly after midnight on 2026-05-08 are still treated as part of the 2026-05-07 development day.

Baseline commit: `9496e9f Improve Office OLE preview fidelity`

Workspace: `D:\chemcore`

### Summary

Today's work had three layers.

The first layer continued moving editing semantics into the Rust engine. Grouping, ungrouping, ordering, context menus, object settings, numeric rotate/scale panels, bond-crossing margins, and selected-object property edits moved away from temporary viewer logic and into the engine plus desktop service boundary. Object Settings ended the day as a dynamic dialog instead of a static ChemDraw-style panel: bonds expose bond length, line width, double-bond spacing, margin width, and hash spacing; graphics and arrows reuse line width; multi-selection shows the union of supported fields; mixed values come back as empty fields that can be explicitly overwritten by the user.

The second layer was desktop performance and long-term architecture. User testing showed that the desktop app lagged on focus/click in larger documents while the browser path stayed smooth. Today's conclusion was written into the architecture docs: the desktop default editor runtime is `DesktopHybridEngineHost`. Hot editing paths such as hover, focus, hit testing, selection, and dragging run synchronously through the WASM build of the same Rust core inside the WebView. Tauri native service owns filesystem, clipboard, export, Office/OLE, windows, and background previews. `TauriEngineHost` remains as a diagnostic and future incremental native path, but it is not the default hot interaction path.

The third layer was Office/OLE integration. Starting from an empty `apps/chemcore-office`, the project now has an independent COM local server, current-user registration commands, `IClassFactory`, baseline `IOleObject`, `IDataObject`, `IPersistStorage`, `IViewObject2`, and `IRunnableObject` interfaces, OLE clipboard publishing, double-click activation through an OLE verb, compound storage payload streams, OLE presentation streams, `CF_ENHMETAFILE`, a Word OOXML package writer, and WMF/EMF preview rendering paths. The preview still does not match ChemDraw quality, but this is no longer a plain image-copy path: Office can identify the Chemcore OLE class, Word can paste an editable Chemcore object, and the object stores Chemcore document payload internally.

### Scope And Code Surface

The commit range reviewed for this log is:

```text
719aa73^..9496e9f
```

Across the range, roughly 70 files changed with about 19443 insertions and 706 deletions. Compared with May 6, today was not just a set of editor features. It advanced three long-term tracks at once: Rust-engine ownership of editing semantics, the formal desktop hybrid runtime decision, and the first real end-to-end Office/OLE editable-object path.

The work was concentrated in four code areas:

```text
crates/chemcore-engine
  Grouping, ordering, context menus, Object Settings, bond-crossing margin,
  selection semantics, and clipboard document extraction.

crates/chemcore-desktop-service
  Desktop engine session APIs, tiered snapshots, object settings, menu,
  selection, clipboard, grouping, and order command bridges.

apps/chemcore-desktop/src-tauri
  Tauri command boundary, desktop clipboard, Office OLE clipboard handoff,
  and native service capability exposure.

apps/chemcore-office
  Independent COM local server, registration, OLE interfaces, clipboard data object,
  compound storage, presentation streams, IViewObject2 drawing, Word OOXML package writer,
  and WMF/EMF previews.

viewer/
  UI/host responsibilities only: dynamic Object Settings forms, context menu display,
  desktop hybrid host cache, selection interaction, cache busting, and Web/Desktop parity.
```

The key structural decision was that Office, Desktop, and Viewer layers must not reimplement chemistry behavior outside the engine. Object Settings, context menus, rotation/scale, selection, grouping, and bond-crossing whiteout may look like UI features, but they affect document semantics, export, Office payloads, and cross-shell consistency. They belong in engine APIs. In the other direction, Office OLE, Windows clipboard, registry registration, compound storage, and Word OOXML are system integration features. They should live in desktop/office adapters that call engine document/render APIs, not inside the chemistry engine.

### Grouping And Object Ordering

The day started by adding native grouping and ordering. The Rust engine gained `engine/groups.rs`, moving group/ungroup and bring/send ordering operations into engine commands instead of letting the viewer reorder scene objects directly. `group_selection()` wraps selected sibling objects in a `SceneObject` with `object_type = "group"`, preserving children, z-index, and insertion position. `ungroup_selection()` restores a group's children into the original sibling level. Ordering commands rank objects by z-index for bring forward, send backward, bring to front, and send to back.

Selection, deletion, dragging, rendering, and CDXML import/export were updated with the same object-tree model. `engine/select.rs` now includes `line`, `bracket`, `symbol`, `shape`, and `group` objects in the same hit testing and region selection path. `document.rs`, `render.rs`, `render/bounds.rs`, and CDXML code now understand nested scene objects. The desktop service and Tauri commands expose group/order APIs, and the viewer calls them through `EngineHost`.

This matters because Object Settings, context menus, clipboard content, and Office payloads now have an object-tree model instead of assuming one flat scene-object list.

There is an important boundary here. The molecule fragment is still Chemcore's primary editable chemistry object, but reaction arrows, graphic shapes, brackets, symbols, text, and groups are scene objects. Previously many paths implicitly assumed that the only meaningful editable content was the molecule fragment. Once object trees appear, that becomes fragile: deletion, selection, dragging, ordering, copy, and export can all diverge. Today's grouping/order work was not just for two toolbar commands; it brought the scene-object lifecycle into the engine so a full reaction scheme pasted into Office will not lose arrows, labels, or reagent text around the molecules.

Tests were written around that boundary: grouping leaves selection on the new group; ungrouping restores children to the same sibling level; z-index ordering keeps stable ranks; nested objects remain reachable by render bounds, hit testing, and region selection; CDXML import/export does not lose graphics because they are nested.

### Bond Crossings And Margin Width

Bond-crossing margin width was the first explicit drawing-rule addition today. Margin width applies only when two bonds cross without sharing an endpoint: before the overpassing bond is drawn, Chemcore creates a white knockout so the lower bond breaks at the crossing and a white margin appears around the upper bond. The default value is `2.0`; ACS Document 1996 uses `1.6`. The current internal value is still a world-centimeter numeric field, and the Object Settings UI displays it in centimeters by default.

The main code changes were:

- `document.rs`: `Bond` gained `margin_width`.
- `editing.rs` and `render_constants.rs`: `EditorOptions` gained default and ACS margin width values.
- `cdxml.rs` and `cdxml/export.rs`: import/export of `MarginWidth` and document defaults.
- `render_objects.rs`: new `render_bond_crossing_knockouts()`, computing the whiteout polygon from crossing angle, upper bond visual width, lower bond visual width, and margin width.
- `render/bond_metrics.rs`: shared `margin_width_for_bond()` and legacy template fallback.
- `docs/bond-rendering-rules.zh-CN.md`, `docs/format-v0.1.md`, and `docs/format-v0.1.zh-CN.md`: format and rendering rules for margin width.

Tests cover white margin knockout generation for later crossing bonds, shared endpoints staying in the normal contact kernel, Default/ACS defaults, CDXML import/export preservation, and margin width in Object Settings.

This rule deliberately does not reuse the endpoint contact kernel. If two bonds share an endpoint, that is an ordinary chemical connection and must be handled by `render_contact`, with joins, contact patches, wedge/hash retreat, and label clipping. If two bonds merely cross geometrically, then margin whiteout applies. This avoids drawing ordinary branch/ring nodes as "overpassing" bonds and avoids punching white holes around normal chemical contacts.

The render code is also not a fixed rectangle erase. `render_bond_crossing_knockouts()` first verifies that the two segments intersect internally, filters near-parallel cases, then projects the lower bond's visual width through the crossing angle. Double bonds, bold/wedge/hash bonds, and different visual widths all affect the whiteout length and width. That makes margin width a real drawing parameter instead of a temporary SVG stroke-white trick.

### Engine-Owned Context Menus And Object Settings

Context menus first landed as a menu matrix and viewer UI. They were then moved into the engine after user feedback that these behaviors must be kernel-level to keep Web and Desktop consistent. The new `engine/context_menu.rs` generates menu items, enabled states, checked states, and command payloads from Rust. Desktop service, Tauri commands, and WASM expose `context_menu_json`; the viewer only renders the menu and forwards commands.

Object Settings also became an engine-owned dialog payload. `viewer/object_settings_host.js` and `viewer/numeric_dialog_host.js` are UI hosts only. Field definitions, values, units, mixed status, and application logic come from `engine/presets.rs`. The frontend no longer hardcodes when to show bond length, line width, bold width, double spacing, margin width, or hash spacing.

The next fix addressed the most important semantic bug: changing Object Settings should edit selected objects, not global defaults. `apply_object_settings_to_selection()` walks selected bonds and graphics and only writes fields that belong to those objects. Bond length moves the selected bond endpoints; line width writes bond or graphic stroke; bold width only applies to wedge/bold-style bonds; bond spacing only applies to multiple bonds; hash spacing only applies to hash/hashed-wedge bonds.

Mixed-selection behavior was completed in the same arc. `object_settings_fields()` takes the union of supported fields across the current selection, then `object_setting_field_value()` decides whether each value is consistent. Consistent values return a number; inconsistent values return `mixed: true`, displayed as a blank field by the UI. When the user enters a new value, the engine applies that value only to objects that support the field. This dynamic model fits Chemcore better than a static ChemDraw-like dialog.

The final Object Settings field set today is:

```text
Bond Length
Line Width
Bold Width
Double Spacing
Margin Width
Hash Spacing
```

`Double Spacing` keeps its percentage semantics because it represents double-bond spacing as a percentage of bond length. The other fields default to `cm`; `pt` remains available as a unit option. The UI no longer relies on browser number-input step validation, because that produced confusing "nearest valid value" popups for otherwise reasonable inputs. Engine-side validation is intentionally simple: values must parse and must be positive.

This discussion also clarified the ownership of context popups, rotation panels, and scale panels. Any dialog that mutates document geometry or object properties should work like color: the engine exposes a renderable payload and an apply payload. The viewer can lay out fields, collect input, and manage focus, but it cannot own which objects can be edited, how changes are applied, whether undo is recorded, or whether label geometry is refreshed.

### Desktop Performance And Hybrid Runtime

User testing showed that the desktop app lagged during focus/click on larger content, while the browser app stayed responsive. The first response was to optimize the desktop IPC path: `crates/chemcore-desktop-service` gained `DesktopEngineSnapshotMode` and `snapshot_json()`, separating document, selection, interaction, and state refreshes. `viewer/engine_host.js` added local cache, snapshot application, export-dirty tracking, and a serialized mutation queue for `TauriEngineSession`. The viewer stopped refreshing full document JSON, render list, bounds, CDXML, and SVG after every selection or hover operation.

Those changes reduced the cost of the native IPC path, but the deeper analysis still held: high-frequency pointer move, hover, focus, and drag should not depend on Tauri IPC plus JSON snapshots. Today's architecture update therefore wrote the long-term rule into `docs/windows-desktop-office-architecture.zh-CN.md`, `docs/architecture.zh-CN.md`, `docs/rust-engine-architecture.zh-CN.md`, and `docs/project-rules.zh-CN.md`.

The rule is:

```text
DesktopHybridEngineHost is the default desktop editor runtime.
The WASM core owns hot interaction paths.
The native desktop service owns files, clipboard, export, Office/OLE, windows, and background work.
TauriEngineHost remains only as ?engine=tauri-native diagnostics and a future experiment path.
```

`a05844e` and `d5f4227` were cache-busting commits to make sure the desktop WebView loaded the new `viewer/app.js` and `viewer/engine_host.js` during testing.

This is a subtle but important choice. WASM does not mean "a second frontend chemistry engine." It is the same Rust `chemcore-engine` compiled as the editor runtime. The long-term meaning of desktop hybrid is: hot editor loops synchronously call the same core inside the WebView; filesystem, system clipboard, Office/OLE, registration, windows, and export run through native service. That is still one core with two shells, not separate Web and Desktop behavior.

`TauriEngineHost` was not deleted because it remains useful for diagnostics and may later serve background tasks or a redesigned incremental native path. But today made the condition explicit: a native hot path must first prove event coalescing/cancellation, incremental diffing, no full JSON snapshots, and large-document latency no worse than the hybrid path. Without those constraints, moving hover and focus into IPC would make the professional editor feel sluggish.

### Selection Semantics

Several selection semantics were fixed today. The first was an important molecule-selection issue. When the user draws a bond, types `Ph` at the endpoint, and switches back to select, the endpoint label should be part of the same molecule component instead of selecting only the bond. Selection rendering and text-edit logic now include endpoint label bounds in molecule selection, with coverage in `text_tool.rs`.

Office debugging later exposed two selection regressions. `aace7e1` tightened selection boxes around content bounds and avoided unnecessary rerenders when clicking selected objects. `a4a7b71` fixed click-selection refresh and component focus: region selection still selects components, but single-click selection should not make every other object lose focusability, and it should not cause a visual page jump.

The final selection fix restored the intended product behavior for primitive selection: single-click selects one bond, node, label, shape, or other primitive; double-click or explicit component selection selects the whole molecule. This does not conflict with endpoint labels participating in molecule components; it only prevents single-click hits from being upgraded into whole-molecule selection.

This reflects a product rule: Chemcore needs both precise editing and component-level editing. Single-click is the precise path for editing one bond, one label, one point, or one shape. Region selection and double-click can be component-level operations for moving a whole molecule or reaction block. A previous component-selection fix accidentally promoted single-click to whole-molecule selection, which removed the user's local-editing entry point. Today separated those layers again.

### Office/OLE Server Skeleton

The second half of the day moved into Office/OLE. The repo gained `apps/chemcore-office`, included in the Cargo workspace. This crate builds `chemcore-office.exe`, an independent COM local server rather than a temporary feature inside the desktop app process. `package.json` added:

```text
npm run office:register-dev
npm run office:unregister-dev
npm run office:print-registration
npm run office:self-test
```

Chemcore's fixed OLE identity is:

```text
Display name:       Chemcore Document
ProgID:             Chemcore.Document
Versioned ProgID:   Chemcore.Document.1
CLSID:              {CB69F54F-F21E-44DE-84FB-89D98FECE056}
Local server:       chemcore-office.exe
```

Development registration writes to `HKCU\Software\Classes`, so it usually does not require administrator privileges. Machine-scope commands exist, but they should be run by the installer or an elevated PowerShell.

The skeleton then gained the minimal OLE object interface implementation: `IClassFactory`, COM reference counting, `ChemcoreOleObject`, interface-part pointers, and vtables for `IOleObject`, `IDataObject`, `IPersistStorage`, `IViewObject2`, and `IRunnableObject`. `office:self-test` verifies class factory creation, interface queries, CLSID, IDataObject formats, and basic storage behavior without requiring Office.

The riskiest part here is implementing COM vtables directly in Rust. The code gives each interface an `InterfacePart<T>` that stores a vtable pointer and owner pointer. `QueryInterface` returns the appropriate interface part for a requested IID and routes all reference counting through `chemcore_object_add_ref()` / `chemcore_object_release()`. This is more involved than a few extern functions, but it lets the same `ChemcoreOleObject` behave as `IDataObject`, `IOleObject`, `IPersistStorage`, `IViewObject2`, and `IRunnableObject`, which is the baseline shape Office expects from an embedded OLE object.

Office Add-ins were deliberately not chosen as the main path. Add-ins can later provide Ribbon buttons, template libraries, or batch insertion, but they do not replace OLE objects. Word/PPT double-click editing, embedded object storage, and static presentation all come from the OLE compound document model. Web sketchers such as Ketcher may inform chemistry editing and format handling, but they do not give ChemDraw/ChemSketch-style Windows Office object activation.

### OLE Storage, Clipboard, And Activation

Chemcore OLE objects began persisting compound storage payloads today. `IPersistStorage::Save` calls `OleSave` and `WriteClassStg`, writing:

```text
ChemcoreManifest
ChemcoreDocument
ChemcorePreviewSvg
\x02OlePres001
\x03EPRINT
```

`ChemcoreDocument` stores Chemcore document JSON. The manifest records ProgID, CLSID, payload stream, preview stream, and presentation stream names. Future Office-object restore and edit-back should continue from these streams.

Desktop copy was connected to the OLE clipboard. In `apps/chemcore-desktop/src-tauri/src/lib.rs`, native clipboard write still publishes Chemcore fragment, document JSON, CDXML, SVG, and Unicode text, but it also calls sibling `chemcore-office.exe --copy-clipboard-payload <payload.json>`. The OLE clipboard object supports `Embedded Object`, `Embed Source`, `Object Descriptor`, Chemcore JSON, CDXML, SVG, Unicode text, and `CF_ENHMETAFILE`.

Double-click activation was also connected. `IOleObject::DoVerb` writes the payload to a temp file and launches `chemcore-desktop.exe` with that file. A Chemcore object embedded in Word can therefore launch the desktop app even if the desktop app was not already running.

Clipboard format enumeration was adjusted so Office prefers the Chemcore OLE object instead of treating SVG/WMF fallback as a plain picture. Word OLE paste and activation were then tightened further, and `crates/chemcore-engine/examples/cdxml_to_clipboard_payload.rs` was added to convert `tmp/*.cdxml` files into JSON payloads that can be fed directly into the OLE clipboard path for testing.

Format order is sensitive. Office decides what to paste from `IDataObject::EnumFormatEtc`, `QueryGetData`, `GetData`, and the available `STGMEDIUM` values. If it accepts ordinary SVG/WMF first, the user gets an uneditable picture. If it accepts `Embedded Object` or `Embed Source`, the user gets a double-clickable Chemcore object. The repeated adjustments today were about making "editable object" the preferred Office result while preserving SVG/CDXML/text as interoperability fallbacks.

### Word Preview, Object Extent, And External Display Media

Most of the late-night work chased Word display fidelity. The first step added OLE metafile previews through `CF_ENHMETAFILE`. Word OLE preview rendering was then fixed by filling in `IViewObject2::Draw`, extents, object descriptors, and metafile mediums, so Word no longer saw only a blank object.

Object extents were fixed next. Word object frames were much larger than the molecule because the extent came from page/canvas size instead of content bounds. The code now uses `visible_payload_bounds()`, backed by `parse_document_json()`, `render_document()`, and `render_primitives_bounds()`, to compute the smallest visible primitive bounds and convert that into HIMETRIC extent. If the content is narrower than Word's default A4 text area, it keeps the real centimeter size; otherwise it scales down.

Content loss caused by sending only a simplified selection fragment to Office paste was fixed afterward. OLE paste needs a complete Chemcore document payload, otherwise Word preview can miss same-page objects or resources. This was tightened further by adding `clipboard_selection_json()` and `document_from_selection()` in the engine, producing a Chemcore document containing the selected content with resources and bounds preserved.

An SVG-rendered bitmap fallback using `resvg` was tried in the middle. That helped with blank previews, but the long-term Word/OLE structure still depends on metafile and storage presentation. The implementation therefore returned to vector previews by mapping engine render primitives directly to GDI lines, polygons, and text instead of asking Office to infer the preview from SVG fallback.

OLE storage presentation streams were filled in as well: `\x02OlePres001` stores an OLE presentation stream, and `\x03EPRINT` stores enhanced print EMF bits. `office:self-test` reads these streams back and verifies that EPRINT contains an EMF payload.

The first direct Word OOXML path was also added: `chemcore-office.exe --write-word-docx-payload <payload.json> <output.docx>` creates a docx package containing `word/embeddings/oleObject1.bin` and `word/media/image1.emf`. This follows the ChemDraw-like direction where the external preview is first-class Office media instead of something Word reverse-engineers during paste.

The final Office work improved WMF/EMF preview fidelity. `windows_office.rs` now includes `PreviewTransform`, `draw_payload_vector_preview()`, `draw_preview_primitive()`, `draw_preview_line()`, `draw_preview_polygon()`, `draw_preview_text()`, `preview_text_lines()`, `create_preview_font()`, baseline/scale handling for scripts, and ANSI text fallback. The final version tightened text, thick lines, polygon centerlines, colors, maximum preview canvas size, and transforms to reduce obvious failures such as huge frames, tiny drawings, missing text, or exaggerated line widths in Word.

The final preview is still not ChemDraw-level. The native vector renderer currently covers the basic subset of engine render primitives. Complex paths, font metrics, advanced fills, transparency/clipping, and parts of text layout still need work. The important progress today is that the editable OLE object, storage payload, presentation streams, and OOXML EMF package writer are now real long-term foundations.

This also clarified why SVG alone is not enough. SVG remains valuable for Web and modern-document fallback, but Windows Office OLE object previews, print caches, and compatibility paths still revolve around metafiles and presentation streams. ChemDraw/ChemSketch-style applications get double-click editing and static display in Word/PPT through OLE objects plus presentations, not through a browser-style SVG embed. Chemcore should therefore provide native objects, CDXML/SVG fallback, EMF/WMF presentation, and OOXML media together.

The Word manual tests exposed this exact layer cake. Early paste attempts were blank. Later attempts displayed content but with an oversized frame. Then some objects were missing, drawings were too small, line widths were too heavy, text disappeared, or clicking an object caused a visual refresh and selection regression. These were not one bug. They came from multiple incomplete OLE surfaces: payload extraction, extent calculation, presentation streams, IDataObject format priority, IViewObject2 drawing, Word's own preview cache path, viewer cache busting, and selection behavior. Today's code connected those surfaces one by one, but real Word/PPT documents still need black-box verification.

### Documentation Updates

The following docs were updated:

- `docs/project-rules.zh-CN.md`: Office/OLE and hybrid desktop runtime rules.
- `docs/windows-desktop-office-architecture.zh-CN.md`: `DesktopHybridEngineHost` as the default desktop runtime, `TauriEngineHost` as diagnostics/future experiment, Chemcore OLE ProgID/CLSID, registration commands, storage streams, and the Word OOXML EMF package writer.
- `docs/architecture.zh-CN.md` and `docs/rust-engine-architecture.zh-CN.md`: long-term desktop hybrid runtime notes.
- `docs/right-click-context-menu-matrix.zh-CN.md`: context menu matrix and behavior.
- `docs/bond-rendering-rules.zh-CN.md`, `docs/format-v0.1.md`, and `docs/format-v0.1.zh-CN.md`: margin width and bond-crossing whiteout.

### Tests And Verification

Today's commits added or extended Rust tests around:

- scene object group/ungroup/order behavior.
- margin width defaults, ACS values, CDXML import/export, and render knockouts.
- Object Settings dynamic fields, selected-object application, and mixed selection.
- endpoint labels participating in molecule selection.
- click selection, component selection, and selection bounds.
- OLE self-tests, storage streams, and Word OOXML package writing.

The Office path was also manually tested with Word: import `tmp/氰化.cdxml` and later reaction-condition examples, select all, copy, paste into Word, then check whether the object double-clicks open, whether paste is blank, whether objects are missing, whether the frame is much larger than content, whether text is missing, and whether line widths are distorted. The current code can paste a Chemcore OLE object and double-click launch the desktop app; display fidelity still needs to chase ChemDraw.

Before this log was written, the worktree was clean and HEAD was `9496e9f`. Writing this log adds:

```text
docs/developer-log-2026-05-07.zh-CN.md
docs/developer-log-2026-05-07.en.md
```

### Follow-Up Notes

Office/OLE is today's largest new surface and the riskiest one. The goal cannot stop at "Word can paste an object." The path should continue toward ChemDraw-like structure: compound storage payloads, external EMF/OOXML previews, double-click activation, edit-back into Office storage, PowerPoint/Excel validation, static-display fallback when Chemcore is not installed, and future compatibility with ChemDraw OLE/CDX payloads.

Preview rendering should not live forever as one large `windows_office.rs` file. Today's GDI renderer was placed in the Office crate to validate OLE structure quickly. It should later move into a testable render/export crate or native preview module so SVG, EMF, WMF, PDF, and Office previews share more primitive mapping and test coverage.

Object Settings is now back in the engine. Future properties should follow the dynamic-field rule: show only fields supported by selected objects, use the union for multi-selection, show mixed values as blank, and apply changes only to objects that own the field. Do not move a static ChemDraw-style dialog back into frontend logic.

The desktop performance direction is settled for now: do not move hot editing paths to Tauri IPC by default. Any future native path must first prove that hover, focus, and drag latency on large documents is at least as good as the hybrid path, and it must not depend on refreshing the whole world with full JSON snapshots.
