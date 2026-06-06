# 渲染覆盖层与单位审计报告

日期：2026-06-06

## 结论摘要

这次排查后，我把问题分成两类：

1. 前端“负责把内核 primitive 画成 SVG DOM”是正常的渲染后端职责，不算私自渲染。入口主要是 `viewer/primitive_dom_renderer.js`。
2. 目前仍然存在若干“前端自己决定几何/覆盖层形状”的路径。它们不是文档主体化学结构，但会影响交互视觉、命中区域、预览表现，确实和“所有东西都由内核决定”这个目标不完全一致。

单位方面，当前最核心的问题不是数值本身，而是命名和边界混乱：`WorldCm`、`*_CM`、`px_to_cm` 这些名字看起来是 cm，但 `crates/chemcore-engine/src/units.rs` 实际已经把内部世界单位当成 pt 使用。也就是说，很多值是 pt 语义，名字却还叫 cm。少量地方仍明确从 CSS px 或真实 cm 派生到内部单位，这些需要逐步收敛。

## 本轮刚修过的选择框状态

当前选择框主路径已经变成：

- 内核产出选中框、选中 bond 点、选中 node box、选中文本框、resize handles。
- 前端 overlay 只筛选可见状态并调用 `renderCorePrimitive(...)`。
- 原子选中框已回退到 `ENDPOINT_FOCUS_RADIUS * 2.0`。
- resize 小方块由内核 `SelectionResizeHandle` primitive 输出，边长 `2.0`，按当前内部单位语义就是 2pt。

相关位置：

- `crates/chemcore-engine/src/engine/select.rs`
- `crates/chemcore-engine/src/engine/select/geometry.rs`
- `crates/chemcore-engine/src/render_primitives.rs`
- `viewer/editor_overlay.js`

## 前端仍在自己决定几何的内容

### A. 选择交互 overlay

文件：`viewer/editor_overlay.js`

以下内容仍由前端直接 `makeSvgNode(...)` 生成，而不是内核 primitive：

| 内容 | 位置/角色 | 现状 | 风险 |
| --- | --- | --- | --- |
| 旋转把手位置、半径、stem、旋转 glyph | `selectionRotateHandleFromBounds`、`selection-rotate-*` | 前端用 `currentRenderBounds("selection")` 和 `screenPxToWorld(5/18/10)` 生成 | 视觉尺寸和命中半径不由内核统一，和 resize handle 已经不一致 |
| 单选特殊行为 | `currentSelectionOverlayBehavior` | 前端判断 line/orbital/tlc/crossTable 是否显示 resize/rotate/center cross | 选择 UI 规则散落在前端，内核并不知道某些对象为什么没有把手 |
| 中心十字 | `selectionCenterCrossFromBounds`、`selection-center-cross` | 前端用 bounds 中心和 `screenPxToWorld(5)` 画两条线 | orbital/TLC 等特殊选择视觉没有内核 primitive |
| resize/rotate/arrow curve 数值浮标 | `selection-resize-scale`、`selection-rotate-angle`、`arrow-curve-angle` | 前端生成 text，偏移 `screenPxToWorld(8)` | 属于交互 UI，但如果要完全内核化，需要内核输出 transient annotation primitive |
| 框选矩形和套索线 | `selection-marquee`、`selection-lasso` | 前端根据 pointer gesture 直接画 rect/polyline | 这是典型交互 chrome，可以保留在前端，也可以下沉到内核 gesture preview |
| TLC spot hover/drag guide 和 Rf label | `tlc-spot-hover-guide`、`tlc-spot-rf-*` | 前端根据 hit-test 返回点和 rf 自己画 guide/label | TLC 交互视觉规则不完全在内核 render list 中 |
| preview document mask | `preview-document-mask` | 前端在 preview active 时画一个 page 背景 rect | 主要是显示管理，不是化学几何；但仍是前端生成的 overlay primitive |

建议：如果目标是“交互层也全部内核化”，可以新增内核接口 `interaction_overlay_render_list()`，输入当前 gesture state，输出 rotate handle、center cross、marquee/lasso、TLC guide、数字浮标等 transient primitives。前端只保留事件采集和 SVG backend。

### B. 文本编辑 DOM

文件：

- `viewer/text_editor_controller.js`
- `viewer/text_editor_render.js`
- `viewer/app.js`

当前文本编辑不是直接编辑内核 SVG primitive，而是创建一个 DOM 编辑器：

- `div.text-editor`
- `svg.text-editor-svg`
- `textarea.text-editor-input`
- `div.text-editor-caret`
- `div.text-editor-selection-segment`

文本 layout 有内核参与，但编辑态的显示、caret、selection highlight 是前端 DOM/CSS 生成的。典型位置：

- `text_editor_controller.js` 创建编辑器根节点和隐藏 textarea。
- `text_editor_render.js` 用前端 SVG text/tspan 重绘编辑态文本。
- `app.js` 的 `renderEditorSelectionSegments` 用 div 画文本选区。

风险：

- 编辑态和提交后的内核文本 primitive 可能有基线、上下标、选区、字体 fallback 的细微差异。
- caret/selection 这些视觉不可能出现在内核 render list 中，调试时容易出现“两套文本渲染”。

建议：

- 保留隐藏 textarea 作为输入法/键盘桥。
- 让内核输出 text edit overlay primitives：编辑文本、caret rect、selection rects、placeholder。
- 前端只负责把 textarea 放在合适的位置并把输入事件同步回内核。

### C. 文档对象 legacy fallback

文件：

- `viewer/scene_renderer.js`
- `viewer/object_fallbacks.js`

`scene_renderer` 当前优先用内核 primitive：

- `renderObjectCorePrimitives(...)`

但如果 shape/line/text 对象找不到对应内核 primitive，会走前端 fallback：

- shape -> `renderShapeObject`
- line -> `renderLineObject`
- text -> `renderTextObject`

代码里已经有 `warnUnexpectedLegacyFallback(...)`，说明作者也意识到在 core pipeline 存在时 fallback 是异常路径。

风险等级：

- Molecule、bracket、symbol 当前基本只走 core，没有明显 fallback。
- shape/line/text 有 fallback。只要某类对象内核漏了 primitive，前端仍会把它画出来，可能掩盖内核缺失。

建议：

- 开发模式下把 `warnUnexpectedLegacyFallback` 升级成明显 UI warning 或测试失败。
- 增加覆盖测试：导入/创建所有 scene object kind，断言每个可见对象都有 core primitive。
- 最终删除 `object_fallbacks.js` 的文档对象渲染职责，或只允许它服务旧 demo/迁移模式。

### D. 文档页面背景和 preview transform

文件：`viewer/app.js`

前端还直接创建：

- `page-background` rect
- `document-content` group
- active gesture 的 DOM transform preview

这类不一定违反“内核渲染”，因为它更像 viewport/page shell 和交互性能优化。但如果要求严格一致：

- 页面背景 rect 可以由内核作为 page primitive 输出。
- move/rotate/resize preview transform 可以由内核输出 preview primitives，而不是前端对现有 DOM group 套 transform。

### E. 工具栏和普通 UI

文件：

- `viewer/toolbar.js`
- `viewer/color_host.js`
- `viewer/editor_context_menu.js`
- `viewer/text_symbol_palette.js`

这些是应用 UI，不是化学文档/画布内容。它们用 HTML/SVG 自己画图标、菜单、颜色面板是合理的，不建议下沉到内核。

## 已经比较干净的路径

### 主文档渲染

内核通过 `render_list` 输出 `RenderPrimitive`，前端 `renderCorePrimitive` 转 SVG。

关键文件：

- `crates/chemcore-engine/src/render_primitives.rs`
- `crates/chemcore-engine/src/render.rs`
- `crates/chemcore-engine/src/render_objects/*`
- `viewer/primitive_dom_renderer.js`
- `viewer/scene_renderer.js`

只要 `scene_renderer` 不走 fallback，几何和样式基本由内核决定。

### 当前选择框/resize handles

本轮改完后，选择矩形、选中 bond、选中 atom box、选中文本框、resize 小方块都已经是内核 primitive。

仍留在前端的是：

- rotate handle
- center cross
- gesture labels
- marquee/lasso
- TLC hover/drag guide

## 单位体系现状

### 关键事实：`WorldCm` 实际存 pt

文件：`crates/chemcore-engine/src/units.rs`

当前定义：

- `CssPx::to_world_cm()` 返回 `px * 72 / 96`。
- `WorldCm::to_css_px()` 返回 `world * 96 / 72`。
- `DEFAULT_BOND_LENGTH_CM = 30.0`
- `DEFAULT_BOND_STROKE_CM = 1.0`
- `DEFAULT_TEXT_FONT_SIZE_CM = 10.0`

这说明当前 internal world unit 的数值语义是 pt，而不是 cm。比如：

- 1 internal unit -> 1pt -> 1.333 CSS px。
- 30 internal units -> 30pt，约等于 ChemDraw 默认 bond length。
- 10 internal units -> 10pt 字号。

但是类型名/函数名仍然叫：

- `WorldCm`
- `world_cm(...)`
- `px_to_cm(...)`
- `cm_to_px(...)`
- `*_CM`

这是最大认知风险。它会让人以为 30 是 30cm，或者以为 `px_to_cm(8)` 得到的是物理 cm；事实上它得到的是 6pt。

### 明确是 pt 语义的核心字段

这些字段虽然名字带 `CM`，但当前应按 pt 理解：

| 字段/常量 | 当前值 | 实际语义 |
| --- | ---: | --- |
| `DEFAULT_PAGE_WIDTH_CM` | 900 | 900pt 页面宽 |
| `DEFAULT_PAGE_HEIGHT_CM` | 600 | 600pt 页面高 |
| `DEFAULT_BOND_LENGTH_CM` | 30 | 30pt 键长 |
| `DEFAULT_BOND_STROKE_CM` | 1 | 1pt 键线宽 |
| `DEFAULT_TEXT_FONT_SIZE_CM` | 10 | 10pt |
| `DEFAULT_MOLECULE_LABEL_FONT_SIZE_CM` | 10 | 10pt |
| `DEFAULT_TEXT_LINE_HEIGHT_CM` | 12 | 12pt |
| `DEFAULT_TEXT_BLOCK_LINE_HEIGHT_CM` | 11.25 | 11.25pt |
| `BOLD_BOND_WIDTH_CM` | 4 | 4pt |
| `SOLID_WEDGE_WIDTH_CM` | 6 | 6pt |
| `LABEL_GEOMETRY_CLIP_MARGIN_CM` | 1.2 | 1.2pt |
| `SELECTION_BOX_STROKE_WIDTH` | 1.0 | 1pt |
| `SELECTION_RESIZE_HANDLE_SIZE` | 2.0 | 2pt |

### 仍然从 CSS px 派生的内核值

这些位置不是纯 pt 常量，而是以 CSS px 为源，再换算成内部单位。换算后仍落在内部 pt 数值上，但语义来源是屏幕像素。

| 位置 | 值 | 换算后 | 备注 |
| --- | ---: | ---: | --- |
| `ENDPOINT_HIT_RADIUS_CM = css_px(9)` | 9px | 6.75pt | 命中半径，屏幕体验导向 |
| `BOND_HIT_RADIUS_CM = css_px(6)` | 6px | 4.5pt | 命中半径 |
| `DRAG_START_THRESHOLD_CM = css_px(4)` | 4px | 3pt | 拖拽启动阈值 |
| hover stroke widths `px_to_cm(1.1/1.2/1.4)` | 1.1-1.4px | 0.825-1.05pt | hover 视觉 |
| `PREVIEW_END_RADIUS = px_to_cm(5)` | 5px | 3.75pt | preview endpoint |
| text edit min width/padding `px_to_cm(8)` | 8px | 6pt | 编辑态最小尺寸 |
| graphics fallback stroke `px_to_cm(1)` | 1px | 0.75pt | 默认 graphic stroke fallback |
| legacy renderer label estimates | 多处 px | 多处 | legacy 路径，不应长期依赖 |

判断：

- 命中半径、拖拽阈值用 screen/CSS px 语义可以理解，因为它们是交互体验，不是文档尺寸。
- hover/preview 的可见 stroke 如果目标是 ChemDraw/print-like 精度，应改成 pt 常量。
- legacy/fallback renderer 里的 px 估算应随 fallback 删除而消失。

### 明确仍从真实 cm 派生的值

这些代码用 `PT_PER_CM` 把真实 cm 转成内部单位：

| 位置 | 源值 | 换算后 | 备注 |
| --- | ---: | ---: | --- |
| `ENDPOINT_FOCUS_RADIUS_CM = world_cm(0.1 * PT_PER_CM)` | 0.1cm | 2.835pt | 原子 focus 半径 |
| `BOND_CENTER_FOCUS_LENGTH_CM = world_cm(0.8 * PT_PER_CM)` | 0.8cm | 22.677pt | bond center focus 长度 |
| `BOND_CENTER_FOCUS_WIDTH_CM = world_cm(0.2 * PT_PER_CM)` | 0.2cm | 5.669pt | bond center focus 宽度 |
| `CLIPBOARD_PASTE_OFFSET_CM = 0.35 * PT_PER_CM` | 0.35cm | 9.921pt | 粘贴偏移 |

这几处是真正“以 cm 为设计源”的值。它们不一定错，但和“内核统一 pt”目标不一致，至少应该改名或改为直接 pt 常量：

- `ENDPOINT_FOCUS_RADIUS_PT = 2.835`，或重定为更明确的 `3.0pt`。
- `BOND_CENTER_FOCUS_LENGTH_PT = 22.677`，或重定为 `22.5pt`/`23pt`。
- `CLIPBOARD_PASTE_OFFSET_PT = 10.0`。

### UI 对象设置仍支持 cm

文件：`crates/chemcore-engine/src/engine/presets.rs`

对象设置 dialog 会同时输出：

- `values.pt = internal value`
- `values.cm = internal value / PT_PER_CM`

输入时如果单位不是 `pt`，会 `value * PT_PER_CM` 转回内部单位。

这说明 UI 层还保留“cm 作为显示/输入单位”的能力。这个不是内核几何单位混乱本身，但会让命名更容易误导。

建议：保留显示单位可以，但内部类型应改名为 `WorldPt` / `WorldUnit`。UI 转换函数应叫 `pt_to_cm_display`、`cm_display_to_pt`，不要叫 `world_cm`。

### 旧 JSON 兼容仍按 cm 迁移

文件：`crates/chemcore-engine/src/document.rs`

`parse_document_json` 检测旧 JSON 后会用 `PT_PER_CM` 整体缩放，再写入当前文档模型。这是合理的兼容层，但应在报告/代码注释中明确：

- 旧文件：cm 语义。
- 当前模型：pt 语义，只是类型名尚未迁移。

## 建议的整改顺序

### 1. 先把单位命名改干净

推荐改名：

- `WorldCm` -> `WorldPt`
- `world_cm` -> `world_pt`
- `px_to_cm` -> `css_px_to_pt`
- `cm_to_px` -> `pt_to_css_px`
- `*_CM` -> `*_PT`
- `*_world_cm()` -> `*_world_pt()`

兼容策略：

- 第一阶段保留 type alias，例如 `pub type WorldCm = WorldPt`，但新增新名字。
- 第二阶段迁移内部调用点。
- 第三阶段只在 legacy JSON / UI display unit 边界保留 `cm` 字样。

### 2. 把前端 overlay 下沉到内核

建议新增一组 transient overlay primitive：

- `SelectionRotateHandle`
- `SelectionRotateStem`
- `SelectionRotateGlyph`
- `SelectionCenterCross`
- `SelectionMarquee`
- `SelectionLasso`
- `GestureMeasurementLabel`
- `TlcSpotGuide`
- `TlcRfLabel`
- `PreviewDocumentMask`

前端输入当前 gesture state 后调用内核：

```text
engine.interaction_overlay_render_list(gesture_state, viewport_state)
```

这样前端仍能负责事件和实际 SVG DOM，但不再决定几何。

### 3. 删除或钉死 legacy object fallback

短期：

- CI 增加测试，断言 core render list 覆盖所有可见 object。
- 开发模式下 fallback 直接报错或显示红色诊断。

长期：

- 删除 `object_fallbacks.js` 的文档对象绘制职责。
- 保留它最多只用于旧 fixture 对比或迁移工具，不参与正常 editor/viewer。

### 4. 区分“文档视觉”和“交互命中”

建议规则：

- 文档视觉尺寸：全部 pt。
- 导入导出转换：边界处理 cm/CDXML/px，不污染内部命名。
- 命中半径、拖拽阈值：可以保留 CSS px 语义，但命名要直说，例如 `ENDPOINT_HIT_RADIUS_SCREEN_PX`，再在运行时/边界换算成 world pt。
- hover/selection 可见 stroke：如果它是画布视觉，应由内核用 pt 输出。

## 高风险清单

1. `WorldCm` 名字和真实行为冲突。它现在是最大误导源。
2. `viewer/scene_renderer.js` 的 fallback 会掩盖内核 render list 缺失。
3. `viewer/editor_overlay.js` 仍有多个交互 UI 几何由前端决定。
4. 文本编辑态仍是单独 DOM/SVG 渲染，和内核文本 primitive 存在双实现。
5. `px_to_cm` / `css_px(...).to_world_cm()` 分布在内核多处，有些是合理交互命中，有些是历史视觉常量。

## 本次排查用到的主要命令

```bash
rg -n 'makeSvgNode|appendChild|screenPxToWorld|selection-|hover-|preview-' viewer -g'*.js' -g'*.css'
rg -n 'px_to_cm|css_px|WorldCm|PT_PER_CM|DEFAULT_.*_CM|_CM' crates/chemcore-engine/src -g'*.rs'
rg -n 'renderCorePrimitive|object_fallbacks|legacy fallback' viewer -g'*.js'
```

## 建议下一步

我建议下一步先做一个小而硬的改造：把 rotate handle、center cross、gesture labels 从 `viewer/editor_overlay.js` 下沉到内核。这样选择交互层就只剩 marquee/lasso/TLC/text edit 这些更“工具态”的 UI，再继续拆会更清楚。
