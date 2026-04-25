# Chemcore 开发者日志 - 2026-04-23

作者：张家骏

时间范围：2026-04-23 20:47 至 2026-04-24 00:28，Asia/Shanghai

## 总结

今天这轮工作把仓库从零整理成了一个可以继续长期推进的跨平台化学文档项目。今天做出来的不是单纯一个 viewer 或一个网页 demo，而是把几个核心边界初步立住了：

- 可读的 `chemcore.json` 文档格式；
- CDXML 导入兼容层；
- 确定性的 glyph 字形几何内核；
- Web SVG viewer/editor 外壳；
- 未来可被 Web、Windows、iPad 共用的 Rust 编辑引擎。

最关键的渲染成果是字形裁剪。化学标签不能只用整块文本 bbox 去避让，也不能让不同平台各自测字。今天把逐字形几何从浏览器测量里抽出来，放进了原生 glyph kernel。viewer 现在用每个字符的 background box 和 optical shape 做 bond retreat、label knockout、wedge 接触变形和碰撞判断。这个方向对未来 Windows 和 iPad 端非常重要，因为它能避免三端各自渲染、各自漂移。

## 仓库初始化

今天初始化了仓库，并把当前 viewer、文档、示例、ignore 规则和运行时产物纳入版本历史。现有文档已经覆盖项目方向：

- `docs/architecture.md`
- `docs/architecture.zh-CN.md`
- `docs/format-v0.1.md`
- `docs/format-v0.1.zh-CN.md`
- `docs/glyph-kernel.md`
- `docs/viewer-rendering-report.zh-CN.md`
- `docs/rust-engine-architecture.zh-CN.md`

当前代码分成几个主要区域：

- `src/chemcore`：Python CDXML 导入和文档转换；
- `cpp/chemcore_glyph_kernel`：原生 glyph layout 和裁剪几何；
- `crates/chemcore-engine`：Rust 编辑引擎；
- `viewer`：当前 Web 外壳、SVG 渲染和 WASM 绑定。

## 可读的 Chemcore JSON

CDXML 转换层做了格式归一化，让导出的 JSON 更接近我们自己的模型，而不是 CDXML 字段的直接泄漏。

主要变化：

- 节点使用 `element: "N"` 和 `atomicNumber: 7`，不再只暴露源格式里的数字元素值；
- label run 使用清晰字段，例如 `fontWeight`、`fontStyle`、`script`、`fontSize`、`fill`；
- 键的显示意图变成明确字段：
  - `stereo.kind`
  - `stereo.wideEnd`
  - `double.placement`
- CDXML 原始字段，例如 `face`、`font`、`color`，只作为导入兼容数据或源信息处理，不作为理想内部模型。

这件事很重要，因为项目目标不是只做 CDXML viewer，而是要做可编辑的跨平台化学绘图软件。内部 JSON 必须直观表达化学结构和渲染意图。

## CDXML 文本样式修复

CDXML 的 `face` 位同时混合了：

- bold；
- italic；
- subscript；
- superscript。

今天处理了一个典型问题：上下标 run 可能只有 `32/64`，从而丢掉相邻文本的 bold 或 italic 位。例如加粗的 `Cu(CH3CN)4PF6` 中，普通字符是加粗的，但 `3`、`4`、`6` 可能只带 subscript 位，前台渲染时就不会加粗。

现在导入器会展开并归一化 molecule label runs。对于只有上下标位的 run，会在同一组 runs 中继承相邻的 bold/italic 样式位。viewer 也会优先消费明确字段。

相关修复：

- `CF3` 的加粗问题最终定位到 CSS 覆盖：`.mol-atom-label { font-weight: 400; }` 把 SVG glyph 上的 bold 压掉了；
- `CF3` 的下标问题最终定位到旧生成 JSON，重新生成示例后前台已有 script layout 能正确渲染。

## Glyph Kernel 与字形裁剪

这是今天最重要的底层渲染工作。

### 问题

化学绘图对标签几何非常敏感。键线不能穿过 `N`、`CF3`、`Me`、`Ts`、`B(OH)2`，但也不能因为一个粗糙的大矩形 bbox 而从空白区域退得太远。

浏览器文本测量不适合作为长期权威来源：

- Web、Windows、iPad 的字体测量会不一致；
- SVG text 和原生文本的 advance、ink bounds 可能不同；
- 化学标签需要逐字形几何，而不是整段文本 bbox；
- 上下标会影响字号和基线。

### 当前设计

C++ glyph kernel 负责确定性的字形几何：

- glyph advance；
- ink box；
- background box；
- 上下标缩放和基线偏移；
- glyph-level optical shape；
- attached label 的 anchor 位置。

当前 kernel 使用从参考字体整理出的 normalized glyph profiles。它还不是 FreeType/HarfBuzz 级别的真实字体像素栅格化，这一点是明确的设计取舍：当前目标是先得到跨 host 稳定、足够贴近化学绘图需求的几何，而不是马上引入完整字体引擎和大体积 WASM。

### 字体 Metrics

`Me` 问题暴露了通用大写字母宽度的缺陷。`M` 明显比普通大写字母宽，之前如果走默认 uppercase profile，`e` 会贴得太近。

今天扩展了 profile 覆盖范围：

- `A-Z`
- `a-z`
- `0-9`
- 常见符号：括号、方括号、正负号、逗号、斜杠、点号等。

宽窄差异明显的字符现在都有显式 metrics。viewer 侧也优先使用同一参考字体族栈，包括 `TeX Gyre Heros`，尽量让浏览器实际绘制和 kernel 几何一致。

### Optical Shapes

glyph kernel 输出的 shape 目前分为：

- rectangle；
- ellipse；
- cut-corner rectangle。

圆形或类圆形字符使用 ellipse，例如 `C`、`G`、`O`、`Q`、`c`、`e`、`g`、`o`、`0`、`6`、`8`、`9`。

削角矩形范围刻意收窄，只覆盖确认需要的字符：

- `L`、`h`、`b`：右上角削；
- `P`、`F`：右下角削；
- `d`：左上角削；
- `q`：左下角削。

viewer 会把这些 cut-corner shape 转成多边形，削角尺寸是 glyph background box 较短边的 `42%`。

### 为什么削角重要

如果 `L`、`P`、`F`、`d`、`q` 这类字符全部按矩形处理，键线会被字符空白区域过度推开，视觉上像是标签和结构断开。削角后，键线 retreat 更接近字形的真实视觉占位，同时不会把规则扩大到 `r/t/k/A/W` 等暂时不该处理的字符。

### 验证

glyph 相关验证覆盖：

- `cpp/chemcore_glyph_kernel/tests/glyph_kernel_smoke.cpp`
- `scripts/glyph_kernel_reference.py`
- `cpp/chemcore_glyph_kernel/tools/chemcore_glyph_svg_demo.cpp`
- `docs/assets/viewer` 下的预览图；
- `npm run build:glyph-wasm` 的独立 wasm 构建。

关键结果是：viewer 不再把浏览器 `getBBox`、`getExtentOfChar` 或 canvas 扫描当成化学标签几何的权威来源。

## Viewer 渲染

viewer 现在已经覆盖了大量 CDXML 派生的视觉行为：

- molecule fragments；
- atom labels；
- group labels；
- bold、italic、subscript、superscript runs；
- label knockout 和 bond retreat；
- 单键、双键、三键、虚键、实楔形键、虚楔形键；
- 文本对象、形状、线条、箭头和页面布局。

### 楔形键几何

实楔形键今天做了比较细的 ChemDraw 兼容处理。

在宽端无标签时，solid wedge 会根据接触键变形：

- 单接触：楔形两侧边分别与接触键远侧边线求交；
- 双接触：楔形中线连到宽端节点，两侧边分别延长到两根接触键的远侧边线；
- 宽端有可见标签时，不触发这套规则，避免挤进文字。

这样可以避免 `TsN` 这类标签附近楔形键被相邻键错误拉伸。

### 双键几何

双键渲染也做了多轮调整：

- 单边双键画成一根主键加一根 offset 短线；
- 短线退让按主键长度比例计算；
- 居中双键保持两根等长平行线；
- 相邻单边双键在内侧相邻且主键长度近似相等时，会把内侧短线拉长到交点；
- 如果相邻主键长度明显不同，则不强行连接，因为 offset 间距和短线退让比例不同，几何上不应该硬贴。

这里的“等长”指相邻两根双键的主键长度接近，不是 `double.placement = center` 那种居中双键类型。

## Web Editor 外壳

旧 viewer 左侧控制面板已经被编辑器式 UI 替换：

- 顶部第一排：文件、新建、保存、撤销/重做、删除、剪切/复制/粘贴、缩放、全貌；
- 顶部第二排：根据当前工具动态变化；
- 左侧工具栏：选择、键、文字、形状、模板；
- 画布占满剩余屏幕。

第二栏会根据左侧模式变化：

- 选择：选择模式、对齐、分布、翻转；
- 键：单键、双键、三键、虚键、加粗、楔形键等；
- 文字：字体、字号、颜色；
- 形状：边框色、填充色、样式；
- 模板：三到八元环和苯环。

UI 目前只是外壳。真正的编辑行为正在迁到 Rust，而不是继续留在浏览器 JavaScript 里。

## Rust 编辑引擎

今天新增了 Rust workspace 和 `crates/chemcore-engine`。

核心决策是：未来 Web、Windows、iPad 都应该调用同一套 Rust core。文档 mutation、hit test、snapping、命令行为和 overlay 几何不能在各平台各写一遍。

今天 Rust engine 已实现：

- 空白文档创建；
- 文档模型序列化；
- 单键工具；
- 端点 hover；
- 固定键长绘制；
- 角度吸附；
- 键中心聚焦；
- 点击单键中心转双键；
- 双键样式循环；
- 选择端点或键；
- 删除；
- 基于文档快照的 undo/redo；
- Web WASM API。

旧的 JS 单键命中、吸附和 mutation 路径已从 viewer 编辑路径移除。

## 编辑交互细节

今天对单键和双键编辑手感做了多轮修正：

- 空白点击生成横向单键；
- 空白拖拽生成固定长度、角度吸附的单键；
- 端点点击按默认 120 度延伸；
- 端点拖拽显示固定长度预览；
- 拖到另一个端点聚焦范围时，预览直接锁到已有端点；
- 松手后复用已有 node，不再创建重合 C；
- 快速点击延伸时，如果默认 120 度终点附近已有端点，也复用已有 node；
- 端点显示半径是 `4.5`，命中半径保留更大，保证容易操作；
- 键中心聚焦矩形是 `18 x 9`；
- 单键模式点击键中心循环：
  - 单边双键；
  - 居中双键；
  - 另一侧单边双键。

这些修复解决了闭环时产生重合碳的问题，也让苯环和共轭双键的行为更接近化学绘图软件。

## 验证

今天常用验证命令包括：

```bash
npm test
npm run build:engine-wasm
npm run build:glyph-wasm
node --check viewer/app.js
cargo test
python3 -m py_compile src/chemcore/convert/cdxml_to_document.py
```

还用 Playwright 做了前台级别验证：

- pointer 坐标映射；
- endpoint hover；
- bond center hover；
- 单键绘制；
- 拖拽吸附已有端点；
- 快速点击吸附已有端点；
- 双键样式循环；
- 单边双键短线比例退让；
- 相邻单边双键内侧线连接条件。

## 提交时间线

| Commit | 内容 |
| --- | --- |
| `7eddb66` | 初始化仓库并记录 viewer 渲染效果。 |
| `140b954` | 归一化可读的 `chemcore.json` 字段。 |
| `5c18034` | 建立第一版 editor toolbar shell。 |
| `5d958db` | 加入上下文工具栏。 |
| `a35b9e1` | 实现早期 JS 单键绘制。 |
| `9901651` | 调整编辑器默认绘制尺寸。 |
| `9767cbc` | 修正屏幕可见键长和线宽。 |
| `e38cc87` | 增强端点聚焦显示。 |
| `8076a35` | 开始 Rust editor engine 迁移。 |
| `48cc3e5` | 增加 Rust 选择和历史栈。 |
| `d471a85` | 修复 SVG pointer 坐标映射。 |
| `be7e753` | 增加键中心转双键。 |
| `9df89c2` | 调整双键聚焦 overlay。 |
| `744b8d2` | 缩小单键中心聚焦。 |
| `4c9daca` | 单键模式下从键中心循环双键样式。 |
| `7ab6cc5` | 调整聚焦几何和双键渲染。 |
| `f7a007f` | 调整端点和键中心聚焦尺寸。 |
| `a30f4b0` | 拖拽绘制时吸附已有端点。 |
| `d0cdff1` | 单边双键短线按比例退让。 |
| `9b47a43` | 快速点击生成键时吸附已有端点。 |
| `5b530a7` | 相邻单边双键内侧线连接。 |
| `c111fae` | 把内侧线连接限制到主键长度近似相等的情况。 |

## 风险和下一步

当前方向已经比较清楚，但风险也明确：

- glyph kernel 现在是确定性几何，不是真实字体像素栅格化。当前足够支撑 label clipping，但未来原生端如果追求像素一致，可能还要继续增强字体管线。
- 编辑逻辑要继续进 Rust。toolbar 状态和 SVG 绘制可以留在 Web 外壳，但化学行为不能重新散回 JavaScript。
- undo/redo 现在是 snapshot 栈，早期可用，后续应该升级成明确 command 或 transaction。
- 还需要补齐更多工具：三键、虚键、楔形键、文字、形状、模板。
- 环模板应该成为 engine-native 操作，而不是 viewer 里临时拼。
- 渲染层未来最好输出平台无关 display list，让 Windows、Web、iPad 共用几何和行为。

今天最重要的架构结论是：`chemcore` 应该是一套确定性的跨平台化学核心，加多个薄平台外壳，而不是 Web 和原生端各做一套。
