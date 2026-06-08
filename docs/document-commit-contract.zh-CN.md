# Document Commit 合同

本文定义 Chemcore 编辑器里的“有效操作”和内核命令系统。保存按钮、撤销/重做、Office/OLE 回写、自动保存、测试和二次开发都必须以同一个 Document Commit 结果为准。

## 核心定义

Document Commit 是一次已经完成、应该进入文档历史的内容变化。

一次操作只有同时满足下面条件，才算 Document Commit：

- 文档内容实际发生变化。
- 变化进入 undo/redo 历史。
- undo 能回到变化前文档。
- redo 能恢复变化后文档。
- 对外只产生一次明确的 revision 推进。

hover、高亮、选择、框选、菜单打开、缩放、平移、工具切换、文本编辑中的光标移动、拖动中的临时预览都不是 Document Commit。

## 内核入口

正式文档修改入口在 Rust/WASM engine 内核：

```rust
Engine::execute_command_json(command_json: &str) -> Result<String, String>
Engine::execute_command(command: EditorCommand) -> Result<CommandResult, String>
```

WASM 暴露：

```ts
engine.executeCommandJson(commandJson): string
engine.revision(): number
engine.lastCommandResultJson(): string
engine.historyJson(): string
```

Tauri 暴露：

```ts
desktop_engine_execute_command_json(sessionId, commandJson): string
```

命令 JSON 直接使用 `EditorCommand` 的 serde 格式，不再使用旧的前端 envelope：

```json
{
  "type": "add-bond",
  "begin": { "x": 120.0, "y": 80.0 },
  "end": { "x": 168.0, "y": 80.0 },
  "order": 1,
  "variant": "single"
}
```

## 返回结果

每次命令返回 `CommandResult`：

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

`created`、`updated`、`deleted` 只记录稳定对象 id，不把完整对象内容塞进结果。完整 undo/redo 数据仍由运行时 history 里的 `before` / `after` 文档快照负责。

## History Entry

运行时历史项格式：

```json
{
  "command": { "type": "add-bond", "...": "..." },
  "before": "<ChemcoreDocument>",
  "after": "<ChemcoreDocument>"
}
```

历史只存在于运行时，不写入 `.ccjs`、`.ccjz`、`.cdxml`、EMF 或 Office/OLE storage。重新打开文件后从空 history 开始。

## Revision 与保存状态

每次 Document Commit 推进一次 `revision`。保存成功后记录：

```text
savedRevision = engine.revision()
savedDocument = currentDocument
dirty = engine.revision() != savedRevision
```

如果 undo 回到保存点，`revision` 会继续推进，但文档等于保存基线时保存按钮应变灰。当前前端优先使用内核 revision，并保留文档 fingerprint 作为没有内核 revision 时的兜底。

保存后默认不清空 undo stack。保存只是更新磁盘/宿主基线，不剥夺用户继续 undo 的能力。将来如果内存压力需要，可以增加“保存后清空历史”的产品策略，但不作为默认行为。

## 拖动边界

拖动过程中的 pointer move 不算 Document Commit。原则是：

```text
pointer down / move
  -> 更新交互状态或临时文档预览
  -> 可以维护同一个待完成 undo snapshot
  -> 不推进 revision

pointer up / finish
  -> 如果最终文档不同于拖动前，产生一次 Document Commit
  -> 只推进一次 revision
```

当前实现中，selection move/rotate/resize、arrow handle、shape handle、TLC spot 等 live update 使用 transient command context。它们可以为撤销捕获 before 快照，但最终只有 finish 才提交 revision。

## 命令命名

命令名使用 kebab-case，表达用户语义，而不是 UI 事件或函数名。

有效命名示例：

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

不允许新增兜底式命令名，例如 `mutation`、`pointer-up`、`toolbar-click`、`legacy-mutation`。

## 当前命令格式

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

`begin` / `end` 是文档世界坐标。`nodeId` 可选；没有 `nodeId` 时由 engine 创建或复用节点。

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

`end` 必须记录，因为 orbital 的方向/角度来自拖拽方向。

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

字段都是可选 patch。没有设置的字段不应写入。

### `apply-document-style`

```json
{
  "type": "apply-document-style",
  "preset": "acs-document-1996"
}
```

这是一条文档级命令，即便它内部会批量修改键长、键宽、字体和图形 stroke。

## 直接执行与交互上下文

自包含命令可以通过 `execute_command_json` headless 执行，例如 `add-bond`、`add-shape`、`apply-bond-style`、`undo`、`redo`。

依赖当前交互状态的命令不能脱离上下文直接执行，例如：

- `move-selection`
- `rotate-selection`
- `resize-selection`
- `edit-arrow-geometry`
- `edit-shape-geometry`
- `apply-text-edit`

这些命令仍会进入 history，用于记录用户完成的操作；但如果外部直接调用 `execute_command_json`，engine 会返回明确错误，而不是假装执行。

## Office/OLE 回写

Office/OLE 回写订阅 Document Commit，而不是订阅 pointer move 或 render。

规则：

- OLE 临时 `.ccjs` 打开后记录 `currentFilePath`。
- 每次 Document Commit 后，如果当前文档是 OLE 临时文档，立即写回临时 `.ccjs`。
- Office server 监听临时文件变化，再更新 OLE storage 并通知 Word/PPT。
- 手动保存按钮可以作为 flush 入口，但不是唯一入口。
- 关闭 tab 或关闭窗口前仍应强制 finish 当前文本编辑并 flush 所有 OLE 临时文档。

## 测试准则

每个有效操作至少应覆盖：

- 命令后文档内容变化。
- `CommandResult.changed == true`。
- `revision` 只推进一次。
- `created` / `updated` / `deleted` 目标符合预期。
- undo 可用。
- undo 后文档回到操作前。
- redo 后文档回到操作后。
- 保存按钮状态符合保存 revision。

每个非有效操作至少应覆盖：

- 文档内容不变。
- revision 不变。
- undo/redo stack 不变。
- 保存按钮不变。
- 不触发 OLE 回写。

当前已有内核测试覆盖 `execute_command_json(add-bond)`、`undo`、`redo` 和交互上下文命令拒绝直接执行。
