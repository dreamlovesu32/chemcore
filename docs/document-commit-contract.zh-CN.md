# Document Commit 契约

本文定义 Chemcore 编辑器中的“有效操作”。保存状态、撤销/重做、Office/OLE 回写，以及未来的协同或自动保存，都应当订阅同一个 Document Commit 契约，而不是分别挂在零散 UI 事件上。

相关底层命令记录见 [editor-command-history.md](editor-command-history.md)。本文描述的是更上层的行为边界。

## 核心定义

**Document Commit** 是一次已经完成、应当进入文档历史的文档内容变化。

一次操作必须同时满足以下条件，才算 Document Commit：

- 文档内容发生变化。
- 变化应进入 undo/redo 历史。
- undo 后能回到 commit 前的文档内容。
- redo 后能恢复 commit 后的文档内容。
- 操作完成时只产生一次明确的 commit 边界。

换句话说：**凡是会进入 undo stack 的文档变化，就是有效操作。**

## 操作记录格式

Document Commit 不只是“文档变了”这个布尔值。每次 commit 都应记录一个稳定的语义命令。

当前历史记录仍可以用 `before` / `after` 文档快照保证 undo/redo 完全准确，但 `command` 必须描述用户意图：

```json
{
  "command": {
    "type": "add-bond",
    "begin": { "nodeId": "node_1", "x": 120.0, "y": 80.0 },
    "end": { "x": 164.0, "y": 80.0 },
    "order": 1,
    "variant": "single"
  },
  "before": "<ChemcoreDocument>",
  "after": "<ChemcoreDocument>"
}
```

约定：

- `command.type` 使用稳定的 kebab-case 英文名称。
- 字段名使用 camelCase。
- 坐标使用文档世界坐标，不使用屏幕坐标。
- 颜色使用规范化 hex，如 `#000000`。
- 目标对象必须在 commit 时解析成稳定 id，不应只记录“当前选择”。
- `before` / `after` 是 undo/redo 的权威数据。
- `command` 是语义记录，用于调试、迁移、协同、外部回写和未来 patch 化。

## 命令命名原则

命令名应表达用户完成的动作，而不是底层实现函数名。

推荐规则：

- 新建对象用 `add-*` 或 `insert-*`。
- 删除用 `delete-*`。
- 粘贴、剪切用 `paste-*` / `cut-*`。
- 改几何用 `move-*`、`resize-*`、`rotate-*`、`scale-*`、`set-*-geometry`。
- 改样式用 `apply-*-style`。
- 改文档级样式用 `apply-document-style`。
- 改对象属性对话框结果用 `apply-object-settings`。
- 无法归类的新功能先补语义命令，不应新增 `legacy-mutation`。

不要用这些作为正式命令名：

- `pointer-up`
- `toolbar-click`
- `sync-document`
- `refresh-render`
- `mutation`
- `update-state`

这些是 UI 或实现事件，不是用户语义。

## 目标格式

批量操作必须记录 commit 时实际作用的目标，不能只记录“selection”。

推荐目标格式：

```json
{
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1", "bond_2"],
    "objects": ["obj_line_1", "obj_text_1"],
    "styles": ["style_bond_default"]
  }
}
```

如果命令只支持一类目标，可以直接使用专用字段：

```json
{
  "type": "apply-bond-style",
  "bondIds": ["bond_1", "bond_2"],
  "style": { "strokeWidth": 1.2 }
}
```

如果操作来自当前选择，命令记录的仍然是解析后的目标 id。选择本身属于交互状态，不是文档修改的稳定输入。

## 改动格式

对于 style、geometry、numeric settings 这类命令，除了目标 id，还应记录本次要应用的改动。

推荐通用结构：

```json
{
  "type": "apply-object-style",
  "objectIds": ["obj_line_1", "obj_shape_1"],
  "changes": {
    "stroke": "#ff0000",
    "strokeWidth": 1.5,
    "fill": "none"
  }
}
```

规则：

- `changes` 只记录用户要求设置的字段。
- 没有变化的字段不要写入 `changes`。
- undo/redo 仍以 `before` / `after` 为准。
- 如果未来要做 patch history，可以从 `before` / `after` 生成精确 patch，也可以把 `changes` 扩展为 `{ before, after }`。

patch 化后的候选格式：

```json
{
  "type": "apply-bond-style",
  "bondIds": ["bond_1"],
  "changes": {
    "style.strokeWidth": { "before": 1.0, "after": 1.5 },
    "style.color": { "before": "#000000", "after": "#ff0000" }
  }
}
```

v0 不要求所有命令都记录 before/after 字段级 patch，但要求命令名和目标/参数足够语义化。

## 推荐命令集合

下面是目标命令表。已有命令应逐步对齐；当前仍落在 `legacy-mutation` 的入口，应迁移到这些命令或补充新命令。

### 创建类

#### `add-bond`

新建化学键。

```json
{
  "type": "add-bond",
  "begin": { "nodeId": "node_1", "x": 120.0, "y": 80.0 },
  "end": { "x": 164.0, "y": 80.0 },
  "order": 1,
  "variant": "single"
}
```

`begin` / `end` 是 bond anchor。anchor 可以指向已有 node，也可以只给坐标，由 engine 在 commit 时创建或复用 node。

#### `add-arrow`

新建箭头或反应线。

```json
{
  "type": "add-arrow",
  "begin": { "x": 80.0, "y": 120.0 },
  "end": { "x": 180.0, "y": 120.0 },
  "variant": "equilibrium",
  "headSize": "small",
  "curve": "270",
  "headStyle": "full",
  "tailStyle": "none",
  "head": true,
  "tail": false,
  "bold": false,
  "noGo": "none"
}
```

#### `add-shape`

新建图形对象。

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

#### `add-text`

新建文本对象。

```json
{
  "type": "add-text",
  "objectId": "obj_text_1",
  "x": 120.0,
  "y": 80.0,
  "text": "Me",
  "style": {
    "fontFamily": "Arial",
    "fontSize": 12,
    "color": "#000000"
  }
}
```

#### `insert-template`

插入结构模板。

```json
{
  "type": "insert-template",
  "template": "benzene",
  "x": 120.0,
  "y": 80.0
}
```

### 几何类

#### `move-selection`

移动一批对象或结构片段。

```json
{
  "type": "move-selection",
  "targets": {
    "nodes": ["node_1", "node_2"],
    "objects": ["obj_line_1"]
  },
  "delta": { "x": 24.0, "y": -12.0 }
}
```

拖动过程中不产生多条命令；pointer up 后产生一次命令。实现中可以刷新同一条 history entry 的最终 `after`。

#### `resize-selection`

缩放或拖动选择框控制点。

```json
{
  "type": "resize-selection",
  "targets": {
    "objects": ["obj_shape_1", "obj_line_1"]
  },
  "handle": "south-east",
  "origin": { "x": 100.0, "y": 100.0 },
  "scale": { "x": 1.2, "y": 1.2 }
}
```

#### `rotate-selection`

旋转选择内容。

```json
{
  "type": "rotate-selection",
  "targets": {
    "nodes": ["node_1", "node_2"],
    "objects": ["obj_line_1"]
  },
  "center": { "x": 150.0, "y": 100.0 },
  "degrees": 30.0
}
```

#### `set-bond-geometry`

改变一根键的几何，如键长、端点位置或数值面板设置。

```json
{
  "type": "set-bond-geometry",
  "bondId": "bond_1",
  "begin": { "nodeId": "node_1", "x": 120.0, "y": 80.0 },
  "end": { "nodeId": "node_2", "x": 164.0, "y": 80.0 },
  "length": 44.0
}
```

如果实际操作是移动 node 导致多根键长度变化，应记录为 `move-selection` 或 `set-node-position`，而不是多条 `set-bond-geometry`。

#### `set-node-position`

改变一个或多个 node 坐标。

```json
{
  "type": "set-node-position",
  "nodes": [
    { "nodeId": "node_1", "x": 120.0, "y": 80.0 },
    { "nodeId": "node_2", "x": 164.0, "y": 80.0 }
  ]
}
```

### 样式类

样式类命令应优先记录“应用到哪些目标”和“设置了哪些字段”。

#### `apply-selection-color`

对当前选择应用统一颜色。commit 时应解析为实际目标。

```json
{
  "type": "apply-selection-color",
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1"],
    "objects": ["obj_text_1", "obj_line_1"]
  },
  "color": "#ff0000"
}
```

#### `apply-bond-style`

修改一批键的样式。

```json
{
  "type": "apply-bond-style",
  "bondIds": ["bond_1", "bond_2"],
  "changes": {
    "variant": "bold",
    "order": 1,
    "strokeWidth": 1.4,
    "color": "#000000"
  }
}
```

这类命令覆盖“改变一个或一批键宽”“改变键型”“改变颜色”等场景。

#### `apply-text-style`

修改文本对象或端点标签的文字样式。

```json
{
  "type": "apply-text-style",
  "targets": {
    "objects": ["obj_text_1"],
    "nodes": ["node_1"]
  },
  "changes": {
    "fontFamily": "Arial",
    "fontSize": 14,
    "bold": true,
    "italic": false,
    "color": "#0000ff"
  }
}
```

#### `apply-arrow-style`

修改一批箭头样式。

```json
{
  "type": "apply-arrow-style",
  "objectIds": ["obj_line_1"],
  "changes": {
    "variant": "equilibrium",
    "headSize": "small",
    "curve": "270",
    "headStyle": "full",
    "tailStyle": "none",
    "bold": false,
    "noGo": "none"
  }
}
```

#### `apply-shape-style`

修改图形对象样式。

```json
{
  "type": "apply-shape-style",
  "objectIds": ["obj_shape_1"],
  "changes": {
    "shapeStyle": "solid",
    "stroke": "#000000",
    "fill": "#ffffff",
    "strokeWidth": 1.0
  }
}
```

#### `apply-line-style`

修改普通线段、括号、图形线型等线样式。

```json
{
  "type": "apply-line-style",
  "objectIds": ["obj_line_1", "obj_bracket_1"],
  "changes": {
    "lineStyle": "dashed",
    "strokeWidth": 1.2,
    "color": "#000000"
  }
}
```

#### `apply-document-style`

应用文档级 style preset 或全局样式。

```json
{
  "type": "apply-document-style",
  "preset": "acs-document-1996",
  "scope": "document"
}
```

如果该命令会批量改变键长、键宽、字体、字号等，仍然应作为一条文档级命令提交，而不是拆成很多局部 style 命令。

### 对象设置类

#### `apply-object-settings`

应用对象设置面板里的组合修改。

```json
{
  "type": "apply-object-settings",
  "objectIds": ["obj_line_1", "obj_shape_1"],
  "settings": {
    "strokeWidth": 1.5,
    "color": "#ff0000",
    "locked": false
  }
}
```

如果设置面板同时修改 geometry 和 style，也仍是一条用户命令，因为用户点击一次确认只形成一次 commit。

### 结构编辑类

#### `replace-endpoint-label`

替换端点标签或缩写。

```json
{
  "type": "replace-endpoint-label",
  "nodeId": "node_1",
  "label": "Me"
}
```

#### `apply-text-edit`

提交文本编辑。

```json
{
  "type": "apply-text-edit",
  "target": {
    "type": "text-object",
    "objectId": "obj_text_1"
  }
}
```

文本具体内容以 `before` / `after` 文档为准。后续如果需要协同，可以扩展为文本 patch。

### 排列与层级类

#### `apply-selection-arrange`

对齐、分布、翻转。

```json
{
  "type": "apply-selection-arrange",
  "targets": {
    "objects": ["obj_1", "obj_2"]
  },
  "command": "align-left"
}
```

#### `apply-selection-order`

修改 z-order。

```json
{
  "type": "apply-selection-order",
  "objectIds": ["obj_1"],
  "command": "bring-forward"
}
```

#### `group-selection`

组合对象。

```json
{
  "type": "group-selection",
  "objectIds": ["obj_1", "obj_2"],
  "groupId": "obj_group_1"
}
```

#### `ungroup-selection`

取消组合。

```json
{
  "type": "ungroup-selection",
  "groupIds": ["obj_group_1"]
}
```

### 剪贴板与删除类

#### `delete-selection`

删除当前选择。命令应记录删除目标。

```json
{
  "type": "delete-selection",
  "targets": {
    "nodes": ["node_1"],
    "bonds": ["bond_1"],
    "objects": ["obj_line_1"]
  }
}
```

#### `cut-selection`

剪切当前选择。

```json
{
  "type": "cut-selection",
  "targets": {
    "objects": ["obj_text_1"]
  }
}
```

#### `paste-clipboard`

粘贴剪贴板内容。

```json
{
  "type": "paste-clipboard",
  "source": "chemcore-internal",
  "inserted": {
    "nodes": ["node_3"],
    "bonds": ["bond_2"],
    "objects": ["obj_text_2"]
  }
}
```

`inserted` 是 commit 后实际新增的对象 id，便于后续选择、调试和协同。

## 非有效操作

以下行为不属于 Document Commit，不应触发 dirty、保存按钮变更、Office 回写或自动保存：

- hover、高亮、focus halo。
- 选中、取消选中、框选、点选。
- pan、zoom、fit view。
- 切换工具、切换模板、切换 toolbar 默认选项。
- 打开菜单、关闭菜单、弹窗预览。
- 拖动过程中的中间预览状态。
- 文本编辑时尚未提交的光标移动和输入预览。
- 复制选择内容到剪贴板。

这些状态属于交互状态或视图状态，可以刷新界面，但不能修改文档提交历史。

## 有效操作

以下行为如果实际改变了文档内容，应产生 Document Commit：

- 新建键、原子、文本、图形、箭头、括号、符号、轨道等文档对象。
- 删除对象或选择内容。
- 剪切选择内容。
- 粘贴选择内容或外部文档片段。
- pointer up 后完成移动、缩放、旋转、翻转、对齐、分布等几何操作。
- pointer up 后完成箭头端点、图形控制点、括号范围等拖拽编辑。
- 点击或命令修改已有键型、键阶、立体样式、线型等。
- 对选中对象应用颜色、字体、字号、粗体、斜体、上下标等样式。
- 文本编辑 commit。
- 替换端点标签或缩写。
- 应用模板到文档。
- undo。
- redo。
- 打开、导入或替换整个文档内容。

如果命令执行完成后文档内容没有变化，不应产生 Document Commit。

## 拖动与预览

拖动中的每个 pointer move 不算有效操作。拖动应按两段处理：

```text
pointer down / move
  -> 更新交互预览或临时编辑状态

pointer up / cancel-to-commit
  -> 如果文档实际变化，产生一次 Document Commit
```

拖动中的预览可以实时显示在编辑器里，但不应写入 undo stack，也不应触发 Office/OLE 回写。用户松开鼠标后，如果最终状态和拖动前不同，才提交一次文档变化。

## 文本编辑

文本编辑应避免每个字符都产生独立 Document Commit。默认规则：

- 进入文本编辑：不是 commit。
- 光标移动、选择文本、输入预览：不是 commit。
- 确认文本编辑、失焦并接受修改、按 Enter/快捷键提交：如果文本内容变化，产生一次 commit。
- 取消文本编辑：不产生 commit。

后续如果需要更细粒度文本历史，可以在文本编辑器内部维护子历史，但对外仍应只在文档级提交点触发 Document Commit。

## Undo/Redo 关系

Undo 和 redo 本身也是 Document Commit，因为它们会改变当前文档内容，并且会影响 dirty 与外部宿主状态。

规则：

- undo 后如果当前文档等于保存基线，dirty 为 false。
- undo 后如果当前文档不等于保存基线，dirty 为 true。
- redo 同理。
- Office/OLE 文档执行 undo/redo 后，也应把回退或重做后的文档状态回写给宿主。

## Dirty 与保存基线

保存状态不应由“用户点过什么按钮”推断，而应由当前文档和保存基线比较得出。

建议概念：

```text
currentDocumentFingerprint = fingerprint(currentDocument)
savedDocumentFingerprint = fingerprint(lastSavedDocument)
dirty = currentDocumentFingerprint != savedDocumentFingerprint
```

Document Commit 后统一刷新 dirty 状态。保存成功后更新 `savedDocumentFingerprint`。

新建空文档在初始化完成后应设置保存基线，因此空白状态不是 dirty。第一次有效文档变化后才 dirty。

## Office/OLE 回写

Office/OLE 回写应订阅 Document Commit，而不是订阅 pointer move、render、sync 或 toolbar 状态变化。

规则：

- OLE 临时文档打开后，记录其 `currentFilePath`。
- 每次 Document Commit 完成后，如果当前文档是 OLE 临时文档，立即把当前文档写回临时 `.ccjs`。
- Office server 监听临时 `.ccjs` 变化，再更新 OLE storage 并通知 Word/PPT。
- 保存按钮仍可作为手动 flush，但不是 Office 回写的唯一入口。
- 关闭 tab 或关闭窗口仍应强制 flush，作为兜底。

这使 Office 行为接近 ChemDraw：有效编辑完成后宿主文档随之更新；拖动过程中的中间态不回写。

## Command Engine 设计

如果把 Chemcore 做成长期可维护、可测试、可二次开发的编辑器，Document Commit 不应该只是前端约定，而应该落到一个明确的 Command Engine 上。

推荐总体结构：

```text
UI / Office / CLI / tests / plugin / automation
  -> Command Engine
  -> Document Model
  -> History / Dirty / Events
  -> Renderer / Export / OLE writeback
```

也就是说，画布、菜单、快捷键、Office 激活、自动化测试和未来插件都只是命令来源。真正修改文档的唯一入口是 Command Engine。

### 目标

Command Engine 应解决这些问题：

- 所有文档修改都有稳定的语义名字。
- 所有文档修改都有统一的输入格式、校验规则和返回结果。
- undo/redo 不再依赖零散 UI 状态。
- dirty、保存按钮、自动保存、Office/OLE 回写都订阅同一个 commit 事件。
- headless 环境可以不打开前台界面直接操作文档。
- 二次开发者可以通过公开命令 API 扩展或测试编辑能力。
- 将来如果要做协同、宏命令、脚本、批处理或 command palette，不需要重写编辑器核心。

### 分层

推荐分成四层，不要混在 pointer controller 里：

```text
Command API
  负责接收外部命令、事务、undo/redo。

Command Registry
  负责注册每个命令的 schema、validate、apply、describe。

Document Mutator
  负责对 document model 做最小、可控的修改。

Commit Manager
  负责生成 history entry、revision、dirty、events、OLE writeback。
```

前端 controller 可以继续负责交互体验，例如吸附、preview、rubber band、hover、选择框，但它最终只能提交语义命令。

### 公开 API

第一版建议暴露这些入口：

```ts
type ExecuteOptions = {
  source?: "ui" | "shortcut" | "menu" | "office" | "cli" | "test" | "plugin";
  preview?: false;
  mergeKey?: string;
  selectionPolicy?: "preserve" | "select-created" | "clear" | "replace";
};

type CommandResult = {
  changed: boolean;
  commandId?: string;
  revision: number;
  beforeRevision: number;
  label?: string;
  created?: CommandTargets;
  updated?: CommandTargets;
  deleted?: CommandTargets;
  selection?: CommandTargets;
  diagnostics?: CommandDiagnostic[];
};

executeCommand(command: ChemcoreCommand, options?: ExecuteOptions): CommandResult;
executeTransaction(commands: ChemcoreCommand[], options?: ExecuteOptions): CommandResult;
undo(): CommandResult;
redo(): CommandResult;
canExecute(command: ChemcoreCommand): CanExecuteResult;
describeCommand(command: ChemcoreCommand): CommandDescription;
```

`executeCommand` 是唯一正式改文档入口。UI 不应该直接改 document model 后再补 dirty。

### 命令生命周期

每条命令应经过固定流程：

```text
receive command
  -> normalize
  -> validate schema
  -> resolve targets
  -> capture before
  -> apply mutation
  -> normalize document
  -> compare before/after
  -> if unchanged: return changed=false
  -> create history entry
  -> bump revision
  -> emit document-committed
  -> update dirty/save state
  -> trigger OLE writeback if needed
```

其中 `normalize` 用于补默认值、把别名字段转换成规范字段、规范颜色和单位。`resolve targets` 用于把 selection、hit result、临时 id 转成稳定 document id。

### 命令对象格式

推荐所有命令共享一个 envelope：

```json
{
  "type": "apply-bond-style",
  "id": "cmd_01JZ7W6QNTK6Z2PB3EQ5B0ZA7K",
  "schemaVersion": 1,
  "targets": {
    "bonds": ["bond_1", "bond_2"]
  },
  "payload": {
    "changes": {
      "strokeWidth": 1.4
    }
  },
  "meta": {
    "source": "ui",
    "createdAt": "2026-06-08T12:00:00.000Z"
  }
}
```

简写命令可以继续接受：

```json
{
  "type": "apply-bond-style",
  "bondIds": ["bond_1", "bond_2"],
  "changes": {
    "strokeWidth": 1.4
  }
}
```

但进入 engine 后应该 normalize 成统一 envelope，方便日志、测试、回放、迁移和插件集成。

### History Entry 格式

history 里不应该只存命令，也不应该只存快照。推荐同时存语义命令和可验证的状态变化：

```json
{
  "id": "hist_01JZ7W7E7VN3T1Q6J2A1PZQ6QE",
  "command": {
    "type": "apply-bond-style",
    "schemaVersion": 1,
    "targets": {
      "bonds": ["bond_1", "bond_2"]
    },
    "payload": {
      "changes": {
        "strokeWidth": 1.4
      }
    }
  },
  "label": "Change Bond Style",
  "beforeRevision": 41,
  "afterRevision": 42,
  "before": "<ChemcoreDocumentSnapshot>",
  "after": "<ChemcoreDocumentSnapshot>",
  "patch": null,
  "createdAt": "2026-06-08T12:00:00.000Z"
}
```

v0 可以用 `before` / `after` 快照作为权威 undo 数据。后续如果文档很大，再把 `patch` 做成权威数据，快照变成调试或 checkpoint。

### 历史保留与内存策略

命令历史是运行时编辑状态，不属于文档内容。保存 `.ccjs`、`.cdxml`、EMF 或 Office/OLE storage 时，不应把 undo stack、redo stack、command log、history entry 写入文件。

文件里只保存当前文档状态，以及必要的文档级 metadata。重新打开文件后，默认从一个空历史开始：

```text
open document
  -> currentDocument = file content
  -> savedDocument = file content
  -> undoStack = []
  -> redoStack = []
  -> dirty = false
```

保存成功后应更新保存基线：

```text
savedDocument = currentDocument
dirty = false
```

是否清空 undo stack 是产品策略，不应该和“写入文件成功”强绑定。

推荐默认策略：

- 保存后不把历史写进文件。
- 保存后更新 `savedDocument` 或 `savedFingerprint`。
- 保存后可以继续允许 undo 到保存前状态。
- undo 到保存前状态后，如果当前文档不同于保存基线，dirty 应重新变为 true。
- 关闭 tab、关闭窗口或重新打开文件后，历史自然丢弃。

这种行为更接近常见编辑软件：保存只是更新磁盘基线，不一定清空撤销历史。用户保存后发现刚才改错了，仍然可以 undo。

为了避免内存膨胀，Command Engine 应设置历史预算：

```text
maxUndoEntries = 100 或按配置
maxUndoMemoryBytes = 例如 64MB / 128MB
checkpointEvery = 20 commits
```

当超过预算时，可以从最旧历史开始裁剪。裁剪历史只影响能撤销多远，不影响当前文档和保存基线。

v0 如果先用完整 `before` / `after` 快照，应加硬性上限。更长期的形式是：

- 普通命令存字段级 patch 或 inverse patch。
- 每隔 N 次 commit 存一个 checkpoint 快照。
- 大型操作可以单独存 compact snapshot。
- redo stack 在新命令提交后清空。
- 关闭文档时释放全部 history。

如果产品希望“保存后释放历史”，也可以提供一种低内存策略：

```text
save success
  -> savedDocument = currentDocument
  -> undoStack = []
  -> redoStack = []
  -> dirty = false
```

但这会带来一个明确代价：用户保存后不能 undo 到保存前。除非内存压力很大，或者目标用户明确接受这种行为，否则不建议作为默认行为。

### 事务

有些用户动作内部会产生多步修改，但对用户只应该是一条历史：

- 插入模板：创建多个 node 和 bond。
- 应用 document style：批量修改键长、键宽、字体、箭头样式。
- 粘贴：插入多个对象并重映射 id。
- 对齐：同时移动多个对象。
- 对象设置面板确认：可能同时改 geometry、style、锁定状态。

这些应通过 transaction 表达：

```json
{
  "type": "transaction",
  "label": "Insert Template",
  "commands": [
    { "type": "add-node", "payload": { "x": 100, "y": 100, "label": "C" } },
    { "type": "add-node", "payload": { "x": 144, "y": 100, "label": "C" } },
    { "type": "add-bond", "payload": { "beginNodeId": "node_1", "endNodeId": "node_2" } }
  ]
}
```

事务规则：

- 对外只产生一次 Document Commit。
- undo 一次回到事务前。
- redo 一次恢复事务后。
- 如果任一步失败，整个事务回滚。
- 事务内部命令可以共享临时 id 映射。
- 事务完成后只触发一次 dirty 刷新和一次 OLE 回写。

### Preview 与 Commit

preview 不是命令，也不是 history。它是交互状态。

拖动画键时推荐流程：

```text
pointer down
  -> interaction.start("draw-bond")

pointer move
  -> interaction.updatePreview({ begin, end, snapped })

pointer up
  -> executeCommand({
       type: "add-bond",
       payload: {
         begin: { x, y, nodeId? },
         end: { x, y, nodeId? },
         order: 1,
         variant: "single"
       }
     })
```

拖动已有对象时：

```text
pointer down
  -> capture original positions

pointer move
  -> render preview positions

pointer up
  -> executeCommand({
       type: "move-selection",
       targets: resolvedTargets,
       payload: { delta }
     })
```

如果拖动结束时 delta 为零，或者吸附后最终 document 未变化，`executeCommand` 应返回 `changed=false`，不产生 history。

### 校验

每个命令都应有 schema 校验和语义校验。

schema 校验负责字段类型：

- `type` 必须存在。
- 坐标必须是有限数字。
- id 必须是字符串。
- enum 必须在允许范围内。
- `changes` 不能是空对象。

语义校验负责文档上下文：

- `bondIds` 指向的 bond 必须存在。
- 不允许删除不存在的对象。
- 不允许把 bond endpoint 指向非法 node。
- 不允许设置负数键宽。
- 不允许设置非法字体大小。
- 批量命令应跳过无变化对象，或整体返回 `changed=false`。

推荐错误格式：

```json
{
  "ok": false,
  "code": "bond.not_found",
  "message": "Bond does not exist.",
  "path": "targets.bonds[0]",
  "targetId": "bond_missing"
}
```

### 命令注册格式

每个命令应注册成一个独立定义：

```ts
type CommandDefinition<TCommand> = {
  type: string;
  version: number;
  label(command: TCommand): string;
  normalize(command: unknown, context: CommandContext): TCommand;
  validate(command: TCommand, context: CommandContext): CommandDiagnostic[];
  apply(command: TCommand, context: CommandContext): ApplyResult;
};
```

`label` 用于 undo 菜单、日志和调试。例如：

- `Add Bond`
- `Move Selection`
- `Change Bond Style`
- `Apply Document Style`
- `Insert Template`

不要把 label 当作唯一逻辑标识；逻辑标识必须是稳定的 `type`。

### 事件

Command Engine 应发出稳定事件，而不是让各模块各自猜状态：

```ts
on("command-executed", event)
on("document-committed", event)
on("history-changed", event)
on("dirty-changed", event)
on("selection-suggested", event)
```

其中 `document-committed` 是最重要事件：

```json
{
  "commitId": "hist_01JZ7W7E7VN3T1Q6J2A1PZQ6QE",
  "commandType": "add-bond",
  "revision": 42,
  "beforeRevision": 41,
  "changed": true,
  "source": "ui",
  "targets": {
    "bonds": ["bond_3"]
  }
}
```

保存按钮、dirty、tab 标题、Office/OLE 回写、自动保存和 telemetry 都应订阅这个事件。

### Headless 操作

Command Engine 应能在没有画布、没有 DOM、没有前台窗口的环境运行：

```text
chemcore document open input.cdxml
chemcore command add-bond --begin 120,80 --end 164,80 --order 1
chemcore command apply-bond-style --bonds bond_1,bond_2 --stroke-width 1.4
chemcore document export output.emf
```

测试里也应该可以这样写：

```js
const engine = createCommandEngine();
engine.loadDocument(blankDocument());

engine.executeCommand({
  type: "add-bond",
  payload: {
    begin: { x: 120, y: 80 },
    end: { x: 164, y: 80 },
    order: 1
  }
});

expect(engine.document.bonds).toHaveLength(1);
engine.undo();
expect(engine.document.bonds).toHaveLength(0);
```

这能避免用鼠标事件测试所有编辑能力。pointer controller 只需要测试“交互能生成正确命令”。

### 插件与二次开发

二次开发不应该直接改 document model。推荐开放：

- 注册新命令。
- 注册新命令的 schema 和 apply。
- 监听 commit 事件。
- 添加导入/导出管线。
- 添加 toolbar/menu 快捷入口，但入口最终仍调用 `executeCommand`。

插件命令命名建议带 namespace：

```json
{
  "type": "plugin.acme.add-reaction-label",
  "schemaVersion": 1,
  "payload": {
    "text": "hv",
    "x": 120,
    "y": 80
  }
}
```

内置命令不要使用 namespace，保持短而稳定，例如 `add-bond`。

### 与现有代码的迁移关系

迁移时不需要一次性重写所有 controller。推荐按风险分阶段：

1. 先实现 `executeCommand` 外壳和 history entry 格式。
2. 把 dirty、保存按钮、OLE 回写改成订阅 commit 事件。
3. 把最常用且最影响保存链路的命令接入：`add-bond`、`move-selection`、`delete-selection`、`apply-bond-style`、`undo`、`redo`。
4. 保留 `legacy-mutation` 作为临时桥接，但每次使用都打日志。
5. 逐步把 pointer controller、toolbar、快捷键、文本编辑、对象设置面板迁移到正式命令。
6. 当所有文档修改入口都有命令类型后，禁止新增 `legacy-mutation`。

临时桥接格式：

```json
{
  "type": "legacy-mutation",
  "label": "Legacy Mutation",
  "meta": {
    "entry": "viewer/editor_pointer_controller.js",
    "reason": "not migrated yet"
  }
}
```

它只能用于迁移，不应该成为新功能入口。

## 推荐前端入口

前端应收敛到一个统一提交入口，而不是每个 UI 操作各自刷新 dirty 或保存状态。

建议形态：

```js
async function commitDocumentChange(reason, action) {
  const before = currentDocumentFingerprint();
  await action();
  await syncDocumentFromEngine();
  const after = currentDocumentFingerprint();

  if (before === after) {
    refreshCommandAvailability();
    return false;
  }

  onDocumentCommitted({ reason, before, after });
  return true;
}
```

`onDocumentCommitted` 负责统一副作用：

- 刷新 undo/redo 可用状态。
- 刷新 dirty 和保存按钮。
- 更新 tab 标题或窗口标题。
- 如果是 OLE 临时文档，写回临时 `.ccjs`。
- 记录调试日志或遥测。

更理想的长期形态是由 engine 返回 `changed`、`revision` 和命令信息，而不是前端只用 fingerprint 判断。

## Engine 侧要求

Engine 应明确区分 interaction state 和 document mutation。

要求：

- 所有文档变化都通过命令或 commit 上下文执行。
- 每个 commit 只产生一个语义命令。
- 拖动命令可以在内部刷新最终 `after` 快照，但对外仍是一次 commit。
- 新功能不应依赖 `legacy-mutation`。
- Engine 最终应暴露文档 revision 或 commit id，便于前端判断是否发生有效变化。

## 迁移清单

后续实现应按以下顺序迁移：

1. 列出所有会修改文档的前端入口和 engine 命令入口。
2. 标注每个入口是否应产生 Document Commit。
3. 把 pointer up、菜单命令、快捷键命令、文本提交接入统一 commit 入口。
4. 把 dirty/save button 只挂到 commit 后刷新。
5. 把 OLE 临时文档回写挂到 commit 后刷新。
6. 为 undo/redo、拖动完成、样式应用、文本提交补测试。
7. 消除或收敛 `legacy-mutation`。

## 测试准则

每个有效操作至少应覆盖：

- 操作后文档内容发生预期变化。
- 操作后 undo 可用。
- undo 后文档回到操作前。
- redo 后文档回到操作后。
- dirty 状态和保存按钮符合保存基线。
- 如果文档是 OLE 临时文档，commit 后会触发一次写回。

每个非有效操作至少应覆盖：

- 文档 fingerprint 不变。
- undo/redo stack 不变。
- dirty 状态不变。
- 不触发 OLE 写回。

## 当前判断

在完成这份契约之前，不应继续把 Office 实时回写挂到新的临时事件上。下一步应先按本文梳理有效操作边界，把现有保存、dirty、undo/redo 可用状态逐步收敛到 Document Commit。
