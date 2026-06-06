# 渲染覆盖层与单位审计报告

日期：2026-06-06

## 结论摘要

本轮已经把项目里最容易误导的两类问题处理掉：

1. 文档对象渲染不再有前端 fallback。`shape`、`line`、`text`、`bracket`、`symbol`、`molecule` 等对象现在只能通过内核 render primitive 进入画布；如果内核漏出 primitive，前端不会再悄悄补画。
2. 内核世界单位已经统一改名为 pt 语义。`WorldCm`、`world_cm`、`px_to_cm`、`*_CM` 这类误导性命名已迁移为 `WorldPt`、`world_pt`、`css_px_to_pt`、`*_PT`。开发阶段 JSON 也不再做旧 cm 文件兼容迁移。

前端仍然负责把内核 primitive 转成 SVG DOM，这属于渲染后端职责，不算“前端自己决定文档几何”。前端仍有少量动态工具 chrome 自己生成，比如框选矩形、套索线、拖拽过程数值浮标和 TLC guide。这些不是文档对象，但如果目标是“交互覆盖层也全部由内核决定”，后续还应继续下沉。

## 本轮已完成

### 1. 文档对象 fallback 已删除

删除文件：

- `viewer/object_fallbacks.js`
- `viewer/legacy_line_renderer.js`
- `viewer/legacy_shape_renderer.js`
- `viewer/legacy_text_renderer.js`
- `viewer/legacy_render_shared.js`

`viewer/scene_renderer.js` 现在只渲染内核 primitive，不再在 `shape`、`line`、`text` 缺 primitive 时走前端补画。这样内核 render list 的缺失会直接暴露出来，不会被前端 fallback 掩盖。

注意：内核里仍有 `legacy_mol` / `render_legacy.rs`，这是 molecule 对象内部的旧 molblock 渲染路径，不是前端 fallback。它仍属于内核输出 primitive 的路径。

### 2. 旧 JSON cm 兼容已去掉

位置：`crates/chemcore-engine/src/document.rs`

当前行为：

- `parse_document_json` 会检查 `format.unit`。
- 缺少 `format.unit` 时补为 `pt`。
- 如果显式写了非 `pt`，直接报错。
- 不再根据旧结构判断 cm 文件，也不再整体乘 `PT_PER_CM` 做迁移。

这符合当前“开发阶段没有旧东西”的策略。以后如果真要支持历史文件，建议另写显式 migration 命令或导入器，不放在正常 parse 路径里。

### 3. 内核单位命名已改为 pt

核心文件：`crates/chemcore-engine/src/units.rs`

当前主要类型/函数：

- `WorldPt`
- `world_pt(...)`
- `CssPx::to_world_pt()`
- `WorldPt::to_css_px()`
- `pt_to_css_px(...)`
- `css_px_to_pt(...)`
- `pt_to_px(...)`
- `px_to_pt(...)`

默认文档视觉常量也已改为 pt 命名：

| 常量 | 值 | 语义 |
| --- | ---: | --- |
| `DEFAULT_PAGE_WIDTH_PT` | 900 | 900pt 页面宽 |
| `DEFAULT_PAGE_HEIGHT_PT` | 600 | 600pt 页面高 |
| `DEFAULT_BOND_LENGTH_PT` | 30 | 30pt 键长 |
| `DEFAULT_BOND_STROKE_PT` | 1 | 1pt 键线宽 |
| `DEFAULT_TEXT_FONT_SIZE_PT` | 10 | 10pt 字号 |
| `DEFAULT_TEXT_LINE_HEIGHT_PT` | 12 | 12pt 行高 |
| `DEFAULT_TEXT_BLOCK_LINE_HEIGHT_PT` | 11.25 | 11.25pt 文本块行高 |
| `BOLD_BOND_WIDTH_PT` | 4 | 4pt |
| `SOLID_WEDGE_WIDTH_PT` | 6 | 6pt |
| `SELECTION_BOX_STROKE_WIDTH` | 1 | 1pt |
| `SELECTION_RESIZE_HANDLE_SIZE` | 2 | 2pt |

### 4. 选择框静态 chrome 已由内核输出

相关文件：

- `crates/chemcore-engine/src/engine/select.rs`
- `crates/chemcore-engine/src/engine/select/geometry.rs`
- `crates/chemcore-engine/src/render_primitives.rs`
- `viewer/editor_overlay.js`

当前选择状态由内核输出：

- selection bounds
- atom / endpoint focus box
- bond center focus
- text focus box
- resize handles
- rotate handle
- rotate stem
- rotate glyph
- orbital / TLC 等特殊选择的 center cross

前端 `editor_overlay.js` 现在只从 render list 读取这些 selection primitive，再调用 `renderCorePrimitive(...)` 转成 SVG。前端不再根据 selection info 自己判断 line/orbital/tlc/crossTable 的选择框行为。

当前尺寸：

- 选择框线宽：`1pt`
- resize 小方块边长：`2pt`
- rotate handle 半径：`3.75pt`
- rotate handle 与 bounds 顶部距离：`13.5pt`
- center cross 半长：`3.75pt`

## 仍留在前端的内容

这些内容仍由前端自己用 DOM/SVG 生成。它们不是文档对象 fallback，但属于交互覆盖层或编辑器 shell。

| 内容 | 位置/role | 当前状态 | 建议 |
| --- | --- | --- | --- |
| resize 数值浮标 | `selection-resize-scale` | 前端根据 active gesture 画 text | 下沉为内核 transient annotation primitive |
| rotate 数值浮标 | `selection-rotate-angle` | 前端根据 active gesture 画 text | 下沉为内核 transient annotation primitive |
| arrow curve 数值浮标 | `arrow-curve-angle` | 前端根据 arrow gesture 画 text | 下沉为内核 transient annotation primitive |
| 框选矩形 | `selection-marquee` | 前端根据 pointer gesture 画 rect | 可下沉为内核 gesture overlay |
| 套索线 | `selection-lasso` | 前端根据 pointer gesture 画 polyline | 可下沉为内核 gesture overlay |
| TLC hover/drag guide | `tlc-spot-hover-guide`、`tlc-spot-rf-*` | 前端根据 hit-test 和 rf 画 guide/label | 建议下沉到 TLC 工具内核 overlay |
| preview document mask | `preview-document-mask` | 前端画 page 背景 mask | 可保留为 viewport shell，或由内核 page primitive 输出 |
| 文本编辑 DOM | `text_editor_*` | 前端创建 textarea/caret/selection DOM | 输入法桥可以留前端，caret/selection 可由内核输出 |

建议下一阶段新增一个明确的“交互 overlay render list”接口，让前端把 gesture state 传给内核，由内核输出这些 transient primitive。前端仍负责事件采集和 SVG DOM backend，但不再决定几何规则。

## UI 设置单位

位置：

- `crates/chemcore-engine/src/engine/presets.rs`
- `viewer/object_settings_host.js`
- `viewer/units.js`

对象设置 UI 继续支持 `cm` 和 `pt`：

- 默认显示单位仍是 `cm`。
- `values.pt` 是内核 pt 值。
- `values.cm` 是 `pt / PT_PER_CM` 的显示值。
- 用户输入 `cm` 时，提交到内核前转成 `pt`。
- 用户输入 `pt` 时，直接作为内核值提交。

这是边界层换算，不再意味着内核世界单位是 cm。`PT_PER_CM` / `CM_PER_PT` 只应出现在导入导出、显示单位、用户输入单位这些边界场景。

## 当前仍允许的非 pt 来源

### 1. UI cm 显示/输入换算

这是有意保留：

- `crates/chemcore-engine/src/engine/presets.rs`
- `viewer/units.js`

### 2. 屏幕交互阈值

命中半径、拖拽启动阈值等仍可以从 CSS px 派生到 pt，因为它们是屏幕交互体验，不是文档尺寸。命名上已改为 `css_px_to_pt` / `to_world_pt`，避免误读为真实 cm。

典型例子：

- endpoint hit radius
- bond hit radius
- drag start threshold
- hover 命中范围

如果某个可见 stroke 属于文档视觉，而不是屏幕命中体验，应优先用固定 pt 常量。

## 风险清单

1. 动态交互 chrome 仍在前端生成，下一步应把 gesture labels、marquee/lasso、TLC guide 继续下沉。
2. 文本编辑态仍有 DOM/SVG 实现，可能和提交后的内核 text primitive 存在细微差异。
3. 内核 molecule 的 `legacy_mol` 路径仍存在，但它已经是内核 primitive 路径，不属于前端 fallback。
4. 旧 JSON 兼容已移除后，非 `pt` 文件会直接失败。这是开发阶段期望行为，但对测试 fixture 和外部样例要同步清理。

## 快速核查命令

```bash
rg -n 'WorldCm|world_cm|to_world_cm|cm_to_px|px_to_cm|DEFAULT_.*_CM|_CM\b' crates/chemcore-engine/src crates/chemcore-desktop-service/src viewer
rg -n 'object_fallbacks|legacy_line_renderer|legacy_shape_renderer|legacy_text_renderer|legacy_render_shared|warnUnexpectedLegacyFallback' viewer
rg -n 'selection-resize-scale|selection-rotate-angle|arrow-curve-angle|selection-marquee|selection-lasso|tlc-spot-rf|tlc-spot-guide' viewer/editor_overlay.js
```

当前预期：

- 第一条只应剩 UI cm/pt 换算相关结果。
- 第二条不应再命中文档对象前端 fallback。
- 第三条会列出仍待下沉的动态交互 chrome。
