# 快捷键实现计划

## 目标

把 ChemDraw 风格的键盘交互分成明确的三层实现，避免把工具切换、hover 编辑和文档事实修改混在一起：

1. 工具快捷键：只切换当前工具或当前工具的绘图变体。
2. Hover hotkeys：鼠标悬停在原子、标签、键或对象上时，按单键直接修改被悬停目标。
3. 文档命令快捷键：对当前选择执行 group、ungroup、join、层级调整等命令。

快捷键不得影响文本编辑、输入框、下拉框、颜色/数值/对象设置弹窗内的键盘输入。

## 当前已经接入

第一批低风险快捷键已经接入前端键盘路由：

| 快捷键 | 行为 |
| --- | --- |
| `Space` | 切换到选择工具 |
| `x` | 切换到单键工具 |
| `e` | 切换到箭头工具 |
| `t` | 切换到文本工具 |
| `g` | 切换到 TLC plate 工具 |
| `1` / `2` / `3` | 切换到单键 / 双键 / 三键绘图变体 |
| `d` | 切换到虚线键绘图变体 |
| `b` | 切换到粗键绘图变体 |
| `w` | 切换到波浪键绘图变体 |
| `y` | 切换到实楔形键绘图变体 |
| `h` | 切换到 hashed wedge 绘图变体 |
| `Ctrl+G` | Group selected objects |
| `Shift+Ctrl+G` | Ungroup selected objects |
| `F2` | Bring to front |
| `F3` | Send to back |

原子/端点 hover label hotkey 优先于工具快捷键。例如鼠标悬停在端点上按 `b` 应优先改成 `Br`，而不是切到粗键工具。

## 本轮继续实现范围

### Hover bond hotkeys

当鼠标悬停在已有键上，按键应直接修改该键，而不是只切换工具。第一批覆盖：

| 快捷键 | 目标行为 |
| --- | --- |
| `1` | Change hovered bond to single |
| `2` | Change hovered bond to double |
| `3` | Change hovered bond to triple |
| `d` | Change hovered bond to dashed |
| `b` | Change hovered bond to bold |
| `w` | Change hovered bond to wavy |
| `y` | Change hovered bond to wedge |
| `h` | Change hovered bond to hashed wedge |

实现原则：

- 后端负责命中当前 hover bond 和修改文档事实。
- 前端只负责按键路由和命令历史包装。
- 如果当前没有 hovered bond，则回退为工具/绘图变体快捷键。
- 如果当前 hover 是 endpoint/label，则原子 hotkey 优先，不触发 bond hotkey。
- 修改后刷新文档和 overlay，并记录 undo/redo。

### `Ctrl+J` Join selected objects

ChemDraw 的 Join 不是 Group。Group 只是 scene object 组合；Join 会把选择中的结构按重叠原子/键合并成一个结构事实。

本轮不允许把 `Ctrl+J` 映射成 group。实现顺序：

1. 先在后端增加显式 `join_selection` 命令入口。
2. 如果当前选择不满足可 join 条件，返回 `false`，不改变文档。
3. 前端 `Ctrl+J` 只调用后端 `joinSelection`。
4. 为可 join 和不可 join 两类情况补测试。

第一版 join 的保守规则：

- 只处理分子 fragment 之间的 join。
- 只在存在几何重叠或非常接近的端点时合并节点。
- 不处理 text/arrow/shape/group 对象。
- 不猜测用户想连接的键；没有明确重叠关系时不修改文档。

如果当前代码里没有足够稳定的 fragment 合并 helper，应先落命令框架和不可 join 返回，再补真正合并逻辑；不能用 group 冒充 join。

## 前端路由顺序

键盘事件处理应按以下顺序：

1. 文件级快捷键：`Ctrl+N/O/S` 等。
2. 正在编辑文本时，只处理文本编辑自己的快捷键和 `Escape`。
3. 输入框、select、textarea、弹窗内不接管绘图快捷键。
4. 文档命令快捷键：undo/redo/copy/cut/paste/select-all/group/ungroup/join/order。
5. Hover endpoint/label hotkeys。
6. Hover bond hotkeys。
7. 工具/绘图变体快捷键。

## 后端职责

后端应提供文档事实修改 API：

- `apply_hovered_bond_hotkey(key) -> bool`
- `join_selection() -> bool`

这些 API 必须进入 command history，支持 undo/redo。前端不直接改 `currentDocument`，也不基于 DOM 推断 chemistry。

## 验证计划

Rust 测试：

- 悬停单键后按 `2` 变双键。
- 悬停双键后按 `1` 变单键。
- 悬停键后按 `d/b/w/y/h` 分别变对应样式。
- 没有 hovered bond 时 hotkey API 返回 `false`。
- `Ctrl+J` 对不可 join 的选择返回 `false` 并保持文档不变。
- 如果实现 fragment join，补一个两 fragment 重叠端点合并的 undo/redo 测试。

前端验证：

- 文本编辑器内输入 `b/d/h/1/2/3` 不触发绘图快捷键。
- hover endpoint 时 `b` 仍优先变 `Br`。
- hover bond 时 `2` 修改已有键，不只是切换工具。
- 非 hover bond 时 `2` 只切换双键工具。
- `Ctrl+J` 调用 `joinSelection`，不会调用 `groupSelection`。

