# Document Commit Contract

This document defines "effective operations" and the kernel command system in the ChemSema editor. Save button state, undo/redo, Office/OLE write-back, autosave, tests, and secondary development must all use the same Document Commit result.

## Core Definition

A Document Commit is a completed content change that should enter document history.

An operation is a Document Commit only when all of the following are true:

- Document content actually changed.
- The change enters undo/redo history.
- Undo can return the document to the state before the change.
- Redo can restore the document to the state after the change.
- Externally, exactly one clear revision advance is produced.

Hover, highlight, selection, lasso selection, menu opening, zooming, panning, tool switching, caret movement during text editing, and temporary drag previews are transient interaction state.

## Kernel Entry Points

The formal document mutation entry points are in the Rust/WASM engine:

```rust
Engine::execute_command_json(command_json: &str) -> Result<String, String>
Engine::execute_command(command: EditorCommand) -> Result<CommandResult, String>
```

WASM exposes:

```ts
engine.executeCommandJson(commandJson): string
engine.revision(): number
engine.lastCommandResultJson(): string
engine.historyJson(): string
```

Tauri exposes:

```ts
desktop_engine_execute_command_json(sessionId, commandJson): string
```

Command JSON directly uses the serde shape of `EditorCommand`:

```json
{
  "type": "add-bond",
  "begin": { "x": 120.0, "y": 80.0 },
  "end": { "x": 168.0, "y": 80.0 },
  "order": 1,
  "variant": "single"
}
```

## Return Result

Every command returns `CommandResult`:

```json
{
  "changed": true,
  "revision": 1,
  "beforeRevision": 0,
  "command": {
    "type": "add-bond",
    "begin": { "x": 120.0, "y": 80.0 },
    "end": { "x": 168.0, "y": 80.0 },
    "order": 1,
    "variant": "single"
  },
  "targets": {
    "nodes": ["n_1", "n_2"],
    "bonds": ["b_3"]
  },
  "created": {
    "nodes": ["n_1", "n_2"],
    "bonds": ["b_3"]
  },
  "canUndo": true,
  "canRedo": false,
  "undoDepth": 1,
  "redoDepth": 0
}
```

`created`, `updated`, and `deleted` record only stable object ids; full object contents are not embedded in the result. Full undo/redo data remains the responsibility of `before` / `after` document snapshots in runtime history.

## History Entry

Runtime history entry shape:

```json
{
  "command": { "type": "add-bond", "...": "..." },
  "before": "<ChemSemaDocument>",
  "after": "<ChemSemaDocument>"
}
```

History exists only at runtime. It is not written to `.ccjs`, `.ccjz`, `.cdxml`, EMF, or Office/OLE storage. Reopening a file starts with empty history.

This does not conflict with [Recovery Journal v1](./protocol/journal-v1.md): the journal stores replayable Document Patches since the last verified snapshot solely for crash recovery. It has no undo/redo cursor and is not user-visible edit history. A newly saved and verified snapshot removes the absorbed journal while the runtime undo stack remains available by default.

## Revision And Save State

Every Document Commit advances `revision` once. After a successful save, record:

```text
savedRevision = engine.revision()
savedDocument = currentDocument
dirty = engine.revision() != savedRevision
```

If undo returns to the save point, `revision` keeps advancing, but the save button should become disabled when the document equals the saved baseline. The current frontend prefers kernel revision and keeps a document fingerprint as fallback when no kernel revision exists.

Saving does not clear the undo stack by default. Saving updates the disk/host baseline; it does not remove the user's ability to keep undoing. A future memory-pressure strategy may add "clear history after save" as a product policy, but it is not the default.

## Drag Boundary

Pointer move during dragging is not a Document Commit. The principle is:

```text
pointer down / move
  -> update interaction state or temporary document preview
  -> may maintain one pending undo snapshot
  -> does not advance revision

pointer up / finish
  -> if the final document differs from the pre-drag document, produce one Document Commit
  -> advance revision only once
```

In the current implementation, selection move/rotate/resize, arrow handles, shape handles, TLC spots, and similar live updates use a transient command context. They may capture the before snapshot for undo, but only finish commits the revision.

## Command Naming

Command names use kebab-case and express user semantics.

Valid naming examples:

- `add-bond`
- `add-arrow`
- `add-shape`
- `add-bracket`
- `add-symbol`
- `add-orbital`
- `insert-template`
- `delete-selection`
- `cut-selection`
- `paste-clipboard`
- `apply-bond-style`
- `apply-text-style`
- `apply-document-style`
- `apply-object-settings`
- `apply-object-settings-to-selection`
- `group-selection`
- `ungroup-selection`
- `undo`
- `redo`

New fallback-style command names such as `mutation`, `pointer-up`, `toolbar-click`, and `legacy-mutation` are not allowed.

## Current Command Shapes

### `add-bond`

```json
{
  "type": "add-bond",
  "begin": { "nodeId": "n_1", "x": 120.0, "y": 80.0 },
  "end": { "x": 168.0, "y": 80.0 },
  "order": 1,
  "variant": "single"
}
```

`begin` / `end` are document world coordinates. `nodeId` is optional; when absent, the engine creates or reuses a node.

### `add-arrow`

```json
{
  "type": "add-arrow",
  "begin": { "x": 80.0, "y": 120.0 },
  "end": { "x": 180.0, "y": 120.0 },
  "variant": "solid",
  "headSize": "small",
  "curve": "arc270",
  "headStyle": "full",
  "tailStyle": "none",
  "head": true,
  "tail": false,
  "bold": false,
  "noGo": "none"
}
```

### `add-shape`

```json
{
  "type": "add-shape",
  "kind": "circle",
  "style": "solid",
  "color": "#000000",
  "begin": { "x": 80.0, "y": 80.0 },
  "end": { "x": 140.0, "y": 120.0 }
}
```

### `add-bracket`

```json
{
  "type": "add-bracket",
  "kind": "square",
  "begin": { "x": 80.0, "y": 80.0 },
  "end": { "x": 160.0, "y": 140.0 }
}
```

### `add-symbol`

```json
{
  "type": "add-symbol",
  "kind": "circle-plus",
  "center": { "x": 120.0, "y": 80.0 }
}
```

### `add-orbital`

```json
{
  "type": "add-orbital",
  "template": "p",
  "style": "hollow",
  "phase": "plus",
  "color": "#000000",
  "center": { "x": 120.0, "y": 80.0 },
  "end": { "x": 120.0, "y": 128.0 }
}
```

`end` must be recorded because orbital direction/angle comes from drag direction.

### `apply-bond-style`

```json
{
  "type": "apply-bond-style",
  "bondIds": ["b_1", "b_2"],
  "style": "bold"
}
```

### `apply-text-style`

```json
{
  "type": "apply-text-style",
  "textObjectIds": ["obj_text_1"],
  "labelNodeIds": ["n_1"],
  "nodeIds": [],
  "command": "font-size",
  "value": "14"
}
```

Text style commands include `font-family`, `font-size`, `align`, `line-height`,
`bold`, `italic`, `underline`, `outline`, `shadow`, `superscript`, `subscript`,
and `formula`. Font-family values are free-form strings.

### `apply-object-settings-to-selection`

```json
{
  "type": "apply-object-settings-to-selection",
  "bondIds": ["b_1"],
  "objectIds": ["obj_line_1"],
  "settings": {
    "bondLength": 48.0,
    "lineWidth": 1.2,
    "boldWidth": 4.0,
    "bondSpacing": 18.0,
    "marginWidth": 2.0,
    "hashSpacing": 3.0
  }
}
```

All fields are optional patches. Fields without settings should not be written.

### `apply-document-style`

```json
{
  "type": "apply-document-style",
  "preset": "acs-document-1996"
}
```

This is a document-level command, even though internally it may batch-modify bond length, bond width, fonts, and shape strokes.

ChemSema JSON documents persist the active defaults near the top of the file as
`style.preset` and `style.defaults`. CLI `new` and `run` commands load those
defaults from the document; later edit commands use them whenever a command does
not explicitly provide style parameters. `apply-document-style` and object
settings commands must keep this document-level style ledger in sync.

## Direct Execution And Interaction Context

Self-contained commands can execute headlessly through `execute_command_json`, such as `add-bond`, `add-shape`, `apply-bond-style`, `undo`, and `redo`.

Commands that depend on current interaction state cannot execute directly outside context, such as:

- `move-selection`
- `rotate-selection`
- `resize-selection`
- `edit-arrow-geometry`
- `edit-shape-geometry`
- `apply-text-edit`

These commands still enter history to record user-completed operations. When external callers invoke `execute_command_json` directly, the engine returns a clear error.

## Office/OLE Write-Back

Office/OLE write-back subscribes to Document Commit.

Rules:

- After an OLE temporary `.ccjs` is opened, record `currentFilePath`.
- After every Document Commit, if the current document is an OLE temporary document, immediately write back the temporary `.ccjs`.
- The Office server watches temporary file changes, then updates OLE storage and notifies Word/PPT.
- The manual save button may act as a flush entry point; autosave and Office/OLE write-back may also trigger flush.
- Before closing a tab or closing the window, the app should still force-finish current text editing and flush all OLE temporary documents.

## Test Guidelines

Every effective operation should cover at least:

- document content changes after the command
- `CommandResult.changed == true`
- `revision` advances exactly once
- `created` / `updated` / `deleted` targets match expectations
- undo is available
- undo returns the document to the pre-operation state
- redo returns the document to the post-operation state
- save button state matches the saved revision

Every non-effective operation should cover at least:

- document content does not change
- revision does not change
- undo/redo stack does not change
- save button does not change
- OLE write-back is not triggered

The current kernel tests already cover `execute_command_json(add-bond)`, `undo`, `redo`, and rejection of direct execution for interaction-context commands.
