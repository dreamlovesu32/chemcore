# Long-Term Architecture For Windows Desktop And Office Integration

This document records the long-term plan for ChemSema Windows desktop and Office integration. The plan builds toward the final product shape from the first stage: one Rust chemical kernel, one professional Windows desktop application, one real Office/OLE integration layer, and a still-shareable Web adaptation layer.

## Target Experience

The final Windows version of ChemSema should reach system integration similar to ChemDraw:

- Double-clicking `.ccjz`, `.ccjs`, or `.cdxml` opens ChemSema directly.
- Word, PowerPoint, and Excel can insert ChemSema objects.
- Office documents show high-quality previews.
- Double-clicking a ChemSema object inside Office opens ChemSema for editing.
- After editing, object data and preview inside Office update together.
- Copying from ChemSema to Office provides an editable ChemSema native object as well as CDXML, SVG, PNG, and other fallbacks.
- Web, desktop, and Office objects use the same Rust engine and do not fork business logic.

## Overall Architecture

Long-term ChemSema should be:

```text
one Rust chemical kernel
+ one professional Windows desktop UI shell
+ one Windows/OLE/Office integration layer
+ one Web adaptation layer
```

The repository should gradually form these modules:

```text
crates/chemsema-engine
  The only chemical kernel: document model, editing commands, CDXML, SVG, render primitives, hit testing.

crates/chemsema-document
  Native document container: .ccjz/.ccjs, version migration, manifest, previews, resources, validation.

crates/chemsema-render
  Reusable render outputs: SVG, PNG, EMF/WMF/PDF export or preview targets.

crates/chemsema-desktop-service
  Desktop document service: open, save, recent files, file locks, auto-recovery, batch export.

apps/chemsema-desktop
  Tauri Windows desktop app: windows, menus, shortcuts, file dialogs, UI, WebView.

apps/chemsema-office
  Windows Office/OLE integration: COM/OLE server, object embedding, preview, activation, paste.

viewer/
  Shared editing UI layer for Web UI and desktop UI.
```

Important principles:

- Rust `chemsema-engine` remains authoritative for editing, import/export, hit testing, render primitives, and document mutation.
- The desktop app does not duplicate chemical logic.
- The Office integration layer does not directly parse or modify ChemSema JSON.
- The Web viewer and desktop viewer should not fork into two behavior sets.
- System capabilities are exposed through services/adapters; the UI layer must not bypass the engine arbitrarily.

All entry points should eventually go through the same service API set:

```text
open_document()
apply_command()
render_document()
save_document()
export_document()
generate_preview()
migrate_document()
```

## Desktop Technical Route

The desktop app uses Tauri 2 + WebView2. Official Tauri Windows prerequisites include Microsoft C++ Build Tools, Microsoft Edge WebView2, Rust, and Node.js. Microsoft WebView2 Runtime is the underlying Web platform for WebView2 apps; release packaging should choose an Evergreen or Fixed Version runtime distribution strategy.

In this project, Tauri is the long-term system adapter:

- Windows windowing.
- Native menu bar.
- Shortcuts.
- File open/save/save-as dialogs.
- Recent files.
- File drag-and-drop open.
- System clipboard.
- File associations.
- Single instance and external-file wake-up.
- Calling the Rust desktop service.
- Hosting the professional ChemSema editing UI.

WebView is only the display and interaction container; it does not mean the product should look like a browser. The window should not show an address bar, browser menu, or temporary web-page layout. The desktop UI should follow professional drawing software conventions: top menu and toolbar, left toolbox, center canvas, right properties panel, and bottom status bar.

## EngineHost Abstraction

To let Web and Desktop evolve together, the UI layer and kernel need a host abstraction:

```text
EngineHost
  WasmEngineHost
    Browser Web version: calls chemsema-engine through wasm-bindgen.

  DesktopHybridEngineHost
    Windows desktop default path: synchronously calls the same chemsema-engine WASM core inside WebView,
    while using Rust native desktop-service system capabilities through Tauri commands.

  TauriEngineHost
    Explicit native diagnostic/future path: calls the Rust native desktop service through Tauri commands.
```

"Hybrid" means the same Rust `chemsema-engine` is compiled both as the WASM editor runtime and as the native desktop-service runtime. Browser and desktop hot editing paths share the same engine behavior, while desktop system capabilities are handled by the native service.

The desktop app should default long-term to `DesktopHybridEngineHost`:

- High-frequency editing paths such as pointer move, hover, focus, hit testing, selection, drag preview, rotate/scale/move, and object settings must synchronously call the WASM core inside the WebView process.
- System capabilities such as file open/save, recent files, system clipboard, multi-format export, Office/OLE, windows, menus, and background preview generation must go through the Tauri native service.
- The UI layer must not reimplement chemical rules because it uses WASM; WASM is only a runtime form of the same Rust core.
- The native service may hold the same engine session, but every mouse move, hover, or focus event must not become Tauri IPC plus a JSON snapshot.

`TauriEngineHost` and `?engine=tauri-native` are retained for diagnostics, regression tests, and future incremental native editor paths. It can be reconsidered for hot editing only after all of the following are true:

- pointer move / hover / focus have coalescing, cancellation, and priority strategies.
- editing feedback does not require transmitting full document/render/state JSON snapshots for every interaction.
- render primitives and selection/focus overlays support incremental diffs or shared-memory style updates.
- focus, drag, and lasso latency on large files is no worse than the desktop default hybrid path.

Long-term direction:

```text
same Rust core
  -> wasm editor runtime: browser + desktop default hot interaction path
  -> native desktop service runtime: files, clipboard, export, Office/OLE, background tasks

two shells
  -> browser shell
  -> Tauri Windows desktop shell
```

As of 2026-05-06, code has begun to enforce this boundary:

```text
viewer/engine_host.js
  Frontend EngineHost entry. Web uses WasmEngineHost; Tauri defaults to DesktopHybridEngineHost.
  tauri-native is enabled only through explicit ?engine=tauri-native for diagnostics and future native path validation.

crates/chemsema-desktop-service
  Native desktop document/engine service. It directly holds chemsema-engine::Engine sessions.

apps/chemsema-desktop/src-tauri
  Tauri command boundary. desktop_engine_* commands are already exposed for future TauriEngineHost use.
```

At the current stage, Web defaults to `WasmEngineHost` and desktop uses `DesktopHybridEngineHost`. This keeps the editor's synchronous call model stable while allowing Tauri native commands to serve low-frequency system capabilities. When extending the native service, the UI should use desktop file/export/clipboard hosts for low-frequency system features, while high-frequency editing keeps using the same editor-facing engine API.

Long-term rule starting 2026-05-07: `DesktopHybridEngineHost` is the official desktop editing runtime. `TauriEngineHost` / native path remains a diagnostic and performance-validation path; it only enters hot-editing evaluation after satisfying the performance conditions above on large-file high-frequency interactions.

Later work on the same day continued thickening non-Office native desktop capabilities:

```text
crates/chemsema-desktop-service
  Owns native file read/write: CCJZ Container v1, legacy gzip read, .ccjs, .cdxml, and .svg.
  Has persisted recent files for the desktop menu.

apps/chemsema-desktop/src-tauri
  Has added native File/Edit/View menus, shortcuts, file open/save/save-as dialogs, drag open,
  startup-argument open, recent-file menu, and .ccjz/.ccjs/.cdxml file association config.
  Has integrated the Tauri single-instance plugin: a second launch forwards openable file arguments
  to the existing window and wakes the main window.
  Has integrated Windows native clipboard commands: copy/cut writes ChemSema selection fragment,
  whole-document JSON, CDXML, SVG, and Unicode text fallback; paste prefers ChemSema selection fragment
  and inserts it into the current canvas.
  Supports PDF preview export: currently the WebView rasterizes the SVG preview and wraps it in a single-page PDF.
  Supports basic EMF preview export: the Tauri backend maps document render primitives to Win32 GDI
  Enhanced Metafile. This path is suitable for preview/Office fallback; path, font, and advanced fill
  fidelity should continue to improve.

viewer/desktop_file_host.js
  Desktop file host inside WebView. Desktop prefers Tauri native file commands;
  browser continues to use File System Access API or download fallback.
```

Current desktop chemical editing interactions still mainly run synchronously through WebView + WASM engine. This part should stay stable so that editor-facing API sync/async behavior remains stable while file-system capability becomes native. Basic EMF preview has landed; later work should move more SVG/path/text details into a testable native vector renderer.

Recommended sequence:

1. Keep `DesktopHybridEngineHost` as the desktop default editing runtime, ensuring browser and desktop hot interactions remain consistent and smooth.
2. Continue thickening `chemsema-desktop-service`: file container, system clipboard, export, recent files, Office/OLE, background preview generation.
3. Keep all object settings, context menus, rotation/scaling, and other editing semantics inside engine APIs; UI only displays dynamic forms and collects input.
4. Keep `TauriEngineHost` as a native diagnostic path, validating incremental protocols, IPC coalescing, and snapshot diffs with real large files.
5. If the native path later proves hot-interaction performance no worse than the hybrid path, discuss it as an optional implementation; otherwise it serves only low-frequency/native system capabilities.

This sequence avoids changing UI and engine heavily at the same time, and prevents the later Office layer from bypassing the desktop service. More importantly, it keeps mouse focus, hover, and dragging, which users feel most directly, on the low-latency path.

## Office Integration Strategy

The core of ChemDraw-level Office experience is Windows OLE/COM embedded objects.

Office integration has three layers:

### 1. File Associations

`.ccjz`, `.ccjs`, and `.cdxml` are registered to ChemSema. When users double-click these files in the filesystem, Outlook attachments, Office recent files, or downloads, Windows opens them with ChemSema.

The Tauri bundle can configure file associations; the Windows layer should use a clear extension + ProgID scheme.

### 2. Custom Protocol

Register:

```text
chemsema://open?file=...
chemsema://open?id=...
chemsema://edit-object?id=...
```

This wakes ChemSema from external systems, web pages, Office Add-ins, or document links, and serves as a launch and navigation mechanism.

### 3. OLE/COM Embedded Object

The long-term target is a ChemSema OLE Object:

```text
ChemSema OLE Object
  - stores .ccjz or equivalent native object payload internally
  - exposes high-quality preview to Office
  - supports double-click activation for editing
  - supports copy/paste as an editable object
  - supports saving and restoring object state from Office documents
```

Implementation recommendations:

- Business core remains Rust.
- Prefer the Rust `windows` crate for the COM/OLE boundary.
- If OLE interface implementation cost is too high, a very thin C++/Win32 shim is allowed.
- A C++ shim may only handle COM/OLE registration, interface forwarding, and Windows lifecycle. It must not implement chemical logic.

An Office Add-in may later enhance Ribbon buttons, template library, batch insertion, selected-object editing, import/export entry points, and similar workflows. However, the Add-in should not replace OLE objects because an Add-in alone cannot provide ChemDraw-style double-click object editing.

## ChemSema OLE Registration

ChemSema's Office object is designed from the start as a long-term OLE class, not as a temporary image paste path.

Fixed object identity:

```text
Display name:       ChemSema Document
ProgID:             ChemSema.Document
Versioned ProgID:   ChemSema.Document.1
CLSID:              {CB69F54F-F21E-44DE-84FB-89D98FECE056}
Local server:       chemsema-office.exe
```

During development, prefer current-user registration:

```powershell
npm run office:register-dev
npm run office:unregister-dev
npm run office:print-registration
npm run office:self-test
```

`office:register-dev` writes to `HKCU\Software\Classes`, usually needs no administrator rights, and only affects the current Windows user. After the production installer stabilizes, it should write to `HKLM\Software\Classes` through:

```powershell
target\debug\chemsema-office.exe --register-machine
target\debug\chemsema-office.exe --unregister-machine
```

`--register-machine` needs administrator privileges and should be run elevated by the production installer. During development, if machine-scope testing is needed, run the corresponding command in an administrator PowerShell.

`apps/chemsema-office` has established the long-term boundary:

- `chemsema-office.exe` is an independent COM local server and is not tied to the lifecycle of `chemsema-desktop.exe`.
- User/machine scope registration and unregistration are supported.
- Basic OLE keys such as `Insertable`, `LocalServer32`, `ProgID`, `VersionIndependentProgID`, `Verb`, and `DefaultIcon` are registered.
- An `IClassFactory` local-server skeleton exists and can be launched by COM and register the class object.
- `IClassFactory::CreateInstance` can return a ChemSema object and supports querying `IOleObject`, `IDataObject`, `IPersistStorage`, `IViewObject2`, and `IRunnableObject`.
- `IPersistStorage::InitNew/Save` has started writing ChemSema OLE compound storage. Current fixed stream names are:

```text
ChemSemaManifest    OLE object manifest, records class/progId and payload stream name.
ChemSemaDocument    ChemSema document JSON, generated by chemsema-engine.
ChemSemaPreviewSvg  Current-stage SVG preview placeholder, to be replaced by real render output later.
\x02OlePres001       EMF presentation stream, used for internal preview in OLE storage.
\x03EPRINT           Enhanced print stream, contents are EMF bits.
```

- `npm run office:self-test` validates COM object creation, interface querying, CLSID return, and OLE storage stream write/read without an Office environment.
- Desktop copy continues to write ordinary Windows clipboard formats, while also calling `chemsema-office.exe --copy-clipboard-payload` to put the same ChemSema document/svg/cdxml payload into the OLE clipboard. That OLE clipboard object supports `Embed Source`, `Object Descriptor`, ChemSema custom JSON, CDXML, SVG, Unicode text, and `CF_ENHMETAFILE`, enabling Office paste as an editable object. The default OLE clipboard enumeration excludes `CF_METAFILEPICT`, avoiding Word's preference for WMF preview generation.
- `chemsema-office.exe --write-word-docx-payload <payload.json> <output.docx>` has been added. This is the first "direct-write Word structure" path: it directly generates an OOXML package containing `word/embeddings/oleObject1.bin` and `word/media/image1.emf`, used to verify and settle the ChemDraw-style external EMF preview structure. Later clipboard/active Word insertion should reuse this package writer for stable previews.

Remaining embedded object interfaces to complete:

```text
IOleObject      Basic extent and DoVerb desktop wake-up are implemented; next is write-back to Office storage after editing.
IDataObject     OLE clipboard already supports Embed Source/Object Descriptor/custom text formats/CF_ENHMETAFILE.
IPersistStorage Writes ChemSema payload stream, SVG preview stream, EMF presentation, and EPRINT; next is Load read-back and edit write-back.
IViewObject2    Has basic native vector preview renderer path; next is more path, font, and advanced fill fidelity.
IRunnableObject Skeleton exists; next is running state and desktop wake-up.
```

Stage 1 only requires Windows/Office to recognize the ChemSema OLE class. Stage 2 lets inserted Office objects show previews. Stage 3 implements double-click activation and edit write-back.

## Native Document Container

Current `.ccjz` is the implemented deterministic `chemsema.container.v1` ZIP package. It supports hashed scene chunks, content-addressed resources, opaque binary attachments, and seek/range reads; legacy gzip remains read-only.

The current normative entries are:

```text
mimetype
manifest.json
document/root.json
entities/scene-000000.jsonl
resources/<sha256>.json
attachments/<sha256>.<ext>
```

Previews, application-version metadata, and migration records are not part of the current `chemsema.container.v1` manifest. Adding them requires a governed schema/compatible field and cross-reader fixtures; callers must not insert undeclared ZIP entries.

All callers continue to use stable container APIs:

```text
load_ccjz()
save_ccjz()
extract_preview()
update_preview()
migrate()
```

This keeps Web, Desktop, Office, CLI, and independent readers on the same governed container boundary.

## Clipboard Formats

ChemDraw-level experience requires serious clipboard support. Copying ChemSema objects should write multiple formats at once:

```text
ChemSema native object
CDXML
SVG
PNG
Plain text / SMILES / InChI (optional later)
```

Paste should read formats by priority:

```text
ChemSema native > CDXML > SVG/PNG > text chemistry
```

This gives reasonable fallbacks between ChemSema, Office, ChemDraw, browsers, and chat tools.

## Preview And Export Formats

Object previews in Office cannot rely only on SVG. Long-term formats are:

- SVG: Web and modern Office.
- PNG: general fallback.
- EMF: high-quality embedded preview for Windows Office.
- PDF: print and publication export.

These outputs should be generated uniformly by engine/render service. The Office layer must not draw chemical structures by itself.

## Development Stages

### Stage 0: Environment And Dependencies

- Get the Windows native toolchain working.
- Remove Bash/WSL dependencies from active runtime entry points.
- Install Tauri project-level dependencies: `@tauri-apps/cli`, `@tauri-apps/api`.
- Confirm WebView2 Runtime, MSVC Build Tools, Rust, and Node.js are available.

### Stage 1: Final Directory Structure

- Create `apps/chemsema-desktop`.
- Create `apps/chemsema-office`.
- Create `crates/chemsema-document`.
- Create `crates/chemsema-desktop-service`.
- Establish boundaries and empty implementations first; do not rush to migrate large amounts of logic.

### Stage 2: Document Service

- Wrap existing `chemsema-engine`. Started: `crates/chemsema-desktop-service` now holds native engine sessions.
- Define open, save, export, preview, migration, and command-execution APIs. Started: currently exposes document JSON, state JSON, render list, bounds, SVG, and CDXML.
- Model Web and Desktop through the same API semantics.

### Stage 3: Tauri Desktop Shell

- Establish the Tauri app. Done: `apps/chemsema-desktop/src-tauri`.
- Load the existing viewer UI. Done: `npm run desktop:dev` starts a Windows desktop window.
- Add menus, shortcuts, file dialogs, recent files, drag open, and single instance. Completed up to single-window native menus, shortcuts, file dialogs, recent files, drag open, startup-argument open, and single-instance wake-up.
- Configure `.ccjz/.ccjs/.cdxml` file associations. Written into Tauri bundle config; needs verification at the Windows system layer after installer installation.

### Stage 4: Desktop Hybrid Runtime And Native Service

- Desktop default editing runtime uses `DesktopHybridEngineHost`: hot interactions run synchronously through the WASM core inside WebView.
- Tauri backend directly calls the Rust engine. Started: Tauri holds `DesktopDocumentService` and exposes `desktop_engine_*` commands.
- Local filesystem, atomic save, recovery sidecars, container validation, and path permissions belong to the Tauri/Rust service. Desktop open/save/save-as uses native file commands and CCJZ Container v1 is handled by the Rust service.
- Viewer is responsible only for UI, event collection, coordinate conversion, and rendering; editing semantics remain decided by the Rust core.
- `TauriEngineHost` remains a `?engine=tauri-native` diagnostic path and is not the desktop default hot interaction path.

### Stage 5: Document Container And Preview

- Done: containerized `.ccjz`, format/version checks, v0.1 and legacy-gzip migration, content-addressed resources, opaque attachments, atomic saves, and recovery journals.
- Done: cross-reading among Rust, browser JavaScript, and the independent Python reader, plus container performance gates.
- Pending: add preview/thumbnail as manifest-governed optional entries rather than private ZIP additions.
- Done: browser visible-region CCJZ chunk loading with edit/undo-preserving hydration, native atomic saves that reuse unchanged entries and preserve opaque attachments, and browser Zip64 with safe-integer rejection.

### Stage 6: Windows Clipboard

- Support native + SVG + PNG first.
- Add CDXML next.
- Later add EMF and text chemistry.

### Stage 7: Office OLE Prototype

- Register ChemSema OLE object.
- Insert objects into Office.
- Display preview.
- Double-click opens ChemSema desktop.

### Stage 8: Complete Office Lifecycle

- Save and restore ChemSema object payload in Office documents.
- Update Office preview after editing.
- Support copy/paste editable objects.
- Support embedded `.ccjz` data inside objects.

### Stage 9: Office Add-in Enhancements

- Ribbon buttons.
- Insert ChemSema Object.
- Edit Selected ChemSema Object.
- Export/Convert.
- Template library.

### Stage 10: Installation, Signing, Updates

- MSI/NSIS.
- File associations.
- COM/OLE registration.
- WebView2 runtime distribution strategy.
- Code signing.
- Auto update.

## Disallowed Routes

- No temporary Electron version.
- No desktop-only chemical editing logic.
- No Office plugin directly parsing or modifying ChemSema JSON.
- No SVG-only paste with editable object postponed indefinitely.
- Do not regress `.ccjz` to a single gzip JSON or bypass the manifest with undeclared entries.
- Do not turn the Tauri backend into a second business layer.

## GUI Testing And Demo Reliability

Real desktop E2E, WebView2 cross-validation, Windows UIA/real input, Office and OS boundaries, final-installer black-box execution, and demo qualification follow the [GUI Test Platform and Demo Reliability Architecture](./gui-test-platform-and-demo-reliability.md). Browser dev-server tests alone do not prove desktop correctness, and the test-build Test ABI must never enter production installers.

## Current Environment Status

As of 2026-05-06:

```text
Tauri CLI:          2.11.0
@tauri-apps/api:    2.11.0
WebView2 Runtime:   147.0.3912.98
WebView2 location:  C:\Program Files (x86)\Microsoft\EdgeWebView\Application
MSVC Build Tools:   Visual Studio Build Tools 2022 17.14.21
Rust:               1.95.0, x86_64-pc-windows-msvc
Node.js:            project-supported Node.js runtime in PATH
```

Still incomplete:

- Windows system-level file association verification after installer installation.
- Code signing and auto update.
- Office/OLE/COM integration.
- Continued fidelity improvements for EMF/native vector renderer paths, fonts, and advanced fills.

## References

- Tauri prerequisites: https://v2.tauri.app/start/prerequisites/
- Tauri project creation and CLI installation: https://v2.tauri.app/start/create-project/
- Tauri CLI reference: https://v2.tauri.app/reference/cli/
- Tauri configuration and file associations: https://v2.tauri.app/fr/develop/configuration-files/
- Microsoft WebView2 Runtime distribution: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- Windows file type registration: https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application
- Office Add-ins overview: https://learn.microsoft.com/en-us/office/dev/add-ins/overview/office-add-ins
