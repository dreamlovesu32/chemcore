# Chemcore 项目全量总结报告

日期：2026-06-10

本报告用于给外部大模型或技术顾问继续分析 Chemcore 项目。内容基于当前仓库静态阅读、文件结构扫描、关键源码抽样和已有架构文档整理。本文没有重新运行完整构建或 `npm run verify`，因此它评价的是代码和架构现状，不等同于“当前工作区已通过所有测试”。

## 0. 一句话结论

Chemcore 已经不是普通网页画板原型，而是一个以 Rust 为核心的跨平台化学文档系统雏形：它试图统一承载化学结构编辑、文档对象模型、ChemDraw 格式兼容、后端无关渲染、浏览器/桌面宿主、Windows Office/OLE 嵌入和多格式导入导出。

当前阶段可概括为：

```text
后期工程原型 / alpha 到 pre-beta 之间
```

它已经有实质工程资产和较强方向感，但还没有达到可以放心替代 ChemDraw 的稳定产品级。下一阶段最重要的不是继续无限堆工具按钮，而是建立更强的格式回归、UI 端到端测试、复杂文件兼容矩阵和发行级产品边界。

## 1. 快照信息

当前仓库路径：

```text
D:\Projects\chemcore
```

当前 Git 状态在扫描开始时是干净的。

当前分支：

```text
emf-text-investigation
```

最近提交：

```text
2198e2b Add chain drawing tool
034743d Polish tool icons and interaction overlays
80540e9 Align equilibrium arrow rendering with ChemDraw
984de66 Match orbital default size to ChemDraw
8ce9ccf Refactor large coordination modules
86389dc Add SDF import and export
9186caa Add native CDX import and export
c545515 Update toolbar icon rendering and orbital interactions
725256d Add equilibrium arrow toolbar support
5791c4c Refine toolbar icon rendering
1dc079b Polish element palette and CDXML test handling
d91dc9f Add periodic element tools and selection summary bar
```

这些提交说明最近的主要推进方向是：链工具、图标体系、可逆箭头、轨道、SDF、CDX、元素周期表工具、底部化学统计栏。

## 2. 项目定位

Chemcore 的目标不是“用前端画一些 SVG 化学结构”。从 README 和架构文档看，它的长期定位是：

- 一个跨平台化学文档核心。
- 一个 ChemDraw 类编辑器的可控内核。
- 一个能服务浏览器、Windows 桌面、Office/OLE、批量转换和未来商业项目的基础设施。
- 一个尽量不依赖外部闭源组件的原生实现项目。

项目主线强调：

- 文档模型、编辑命令、命中测试、化学标签逻辑、CDXML/CDX/SDF 解析和 render primitives 尽量收在 Rust core。
- viewer、Tauri、Office 只是宿主和适配层，不应重新实现化学行为。
- WASM 不是前端临时 fallback，而是同一个 Rust engine 在浏览器/WebView 内的运行形态。
- 桌面端的系统能力由 native service 承担，但高频编辑热路径仍优先使用 WebView 内 WASM core。

## 3. 为什么这个项目有意义

化学结构编辑的难点远超过普通绘图软件。可靠的 ChemDraw 类工具需要同时处理：

- 原子、键、价态、隐式氢、形式电荷、自由基。
- 键型、双键偏置、楔形键、虚键、波浪键、hash/wedge、交叉白洞。
- 化学标签和缩写，例如 `Ph`、`Boc`、`CO2Et`、`N3`、`t-Bu`、元素标签、上下标。
- 文档对象，例如文本、箭头、括号、图形、TLC 板、轨道、环模板、链工具。
- ChemDraw 的 CDXML/CDX 导入导出。
- SDF 等结构交换格式。
- SVG、EMF、PDF 等展示/导出格式。
- Windows 剪贴板、Office OLE 嵌入、Word/PPT 预览。
- 浏览器端和桌面端一致行为。

如果这些规则散落在前端 UI 里，后期很快会不可维护。Chemcore 当前最重要的价值是坚持把核心语义收在 Rust engine，形成可测试、可复用、可移植的内核。

从长期商业角度看，Chemcore 即使本体免费，也可以成为后续化学软件产品、云服务、文档处理工具、企业定制和自动化链路的入口。

## 4. 仓库结构

当前顶层结构：

```text
chemcore/
  crates/chemcore-engine/             Rust 文档、编辑、渲染、格式核心
  crates/chemcore-desktop-service/    桌面端原生 session、文件、最近文件和服务层
  apps/chemcore-desktop/              Tauri 2 Windows 桌面应用
  apps/chemcore-office/               Windows Office/OLE 集成服务
  viewer/                             浏览器/WebView 共享 UI
  shared/                             Rust/viewer 共用 glyph 和符号数据
  docs/                               架构、格式、规则、调研和开发日志
  scripts/                            构建、验证、ChemDraw/Office 对照和回归工具
  examples/                           原生文档和格式转换示例
  compare/                            对比样本
  tmp/                                临时输出目录，当前未跟踪
```

Rust workspace 成员：

```text
crates/chemcore-engine
crates/chemcore-desktop-service
apps/chemcore-desktop/src-tauri
apps/chemcore-office
```

根 `package.json` 提供前端、WASM、桌面、Office 和验证脚本。

## 5. 代码规模概览

以下统计排除了 `target`、`node_modules`、`tmp`，并排除了部分生成产物、lockfile、图标、compare 和 example 文件。数字用于理解规模，不是严格审计结果。

按扩展名估算：

```text
.rs       113 文件，约 90,091 行
.json       9 文件，约 81,784 行，其中主要是 shared glyph 数据
.md        57 文件，约 19,783 行
.js        31 文件，约 13,284 行
.py        73 文件，约 11,456 行
.mjs       28 文件，约 4,070 行
.css        1 文件，约 2,146 行
.cdxml      9 文件，约 4,357 行
```

按目录估算：

```text
shared/   4 文件，约 81,385 行，主要是 glyph outlines/clip polygons 数据
crates/ 105 文件，约 79,951 行
docs/    54 文件，约 19,647 行
scripts/107 文件，约 16,475 行
viewer/  37 文件，约 15,904 行
apps/    24 文件，约 14,696 行
```

最大文件信号：

```text
shared/glyph_clip_polygons.json                              41,939 行
shared/glyph_outlines.json                                   39,205 行
crates/chemcore-engine/tests/render_document.rs               9,147 行
crates/chemcore-engine/tests/bond_tool.rs                     8,545 行
apps/chemcore-office/src/windows_office/emf_preview/renderer.rs 5,609 行
apps/chemcore-office/src/windows_office.rs                    4,592 行
viewer/app.js                                                 3,971 行
crates/chemcore-engine/src/engine.rs                          2,653 行
```

含义：

- 项目已经有真实工程体量。
- `shared` 里大文件是数据资产，不应按普通代码复杂度看。
- 测试文件巨大，说明 engine 行为已经有大量回归用例。
- 前端、Office 和 EMF 渲染仍有大文件维护风险。

## 6. 核心架构

当前架构可以理解为：

```text
Rust chemcore-engine
  文档模型
  编辑状态机
  命令历史
  命中测试
  化学语义
  CDXML/CDX/SDF
  render primitives
  SVG 输出
  WASM binding

viewer/
  工具栏
  DOM/SVG 渲染
  文件打开保存 UI
  鼠标键盘事件采集
  文本编辑 UI
  palette/dialog host
  EngineHost 抽象

Tauri desktop
  窗口
  原生菜单
  文件对话框
  最近文件
  原生剪贴板
  EMF 导出
  单实例和文件关联

Office/OLE
  COM local server
  OLE storage
  OLE clipboard object
  EMF presentation
  Word OOXML package writer
```

重要设计选择：

- `chemcore-engine` 是权威核心。
- 浏览器默认 `WasmEngineHost`。
- Windows 桌面默认 `DesktopHybridEngineHost`，热交互走 WebView 内 WASM core，系统能力走 Tauri/native。
- `TauriEngineHost` 仍保留为诊断和未来 native hot path 验证入口。
- 文档持久化和运行时交互状态分离。
- render 层输出 backend-neutral primitives，不把 DOM/SVG 作为唯一后端。

## 7. Rust engine 总览

路径：

```text
crates/chemcore-engine
```

主要模块：

```text
document.rs                 ChemcoreDocument、SceneObject、Resource、MoleculeFragment、Node、Bond
editing.rs                  Tool、BondVariant、ArrowVariant、ShapeKind、OrbitalTemplate 等编辑类型
engine.rs                   Engine 状态机入口、overlay、命令、工具调度
engine/*                    具体工具和命令子模块
render.rs                   render_document 入口和渲染模块组织
render_* / render/*         键、接触、边界、标签、primitive、style payload
render_objects/*            文本、图形、箭头等文档对象渲染
cdxml.rs / cdxml/*          CDXML 原生导入导出
cdx.rs                      CDX 二进制与 CDXML tree 桥接
sdf.rs                      SDF V2000 基础导入导出
symbols.rs                  电荷/自由基符号、氢覆盖、元素工具相关规则
label_rules.rs              标签解析、公式方向、文本规则
abbreviation/*              缩写识别和展开
glyph_kernel.rs             glyph profile/outlines/clip polygon 内核
wasm.rs                     wasm-bindgen API
```

Engine 当前承担：

- 文档加载和导出。
- pointer move/down/up。
- 选择、框选、套索、移动、旋转、缩放。
- 键工具、箭头工具、括号/符号工具、元素工具、文本工具、形状工具、TLC 板、轨道、模板、链工具。
- 右键菜单和 object settings。
- 剪贴板片段。
- undo/redo。
- command result、revision、history。
- selection chemistry summary。
- palette JSON。
- toolbar icon SVG 输出。

## 8. 文档模型

自有格式 v0.1 顶层结构：

```json
{
  "format": { "name": "chemcore", "version": "0.1", "unit": "pt" },
  "document": {},
  "styles": {},
  "objects": [],
  "resources": {}
}
```

核心原则：

- 单位使用 `pt`。
- 坐标原点左上，x 向右，y 向下。
- `objects` 是场景对象树。
- `resources` 存储可复用资源，例如 molecule fragment。
- `molecule` 是化学语义对象。
- `text`、`line`、`bracket`、`shape`、`group` 是文档对象。
- 分子 label 不是普通 text 对象，它属于 molecule fragment。

`ChemcoreDocument` 当前包含：

- `FormatInfo`
- `DocumentInfo`
- `styles: BTreeMap<String, Value>`
- `objects: Vec<SceneObject>`
- `resources: BTreeMap<String, Resource>`

`SceneObject` 通用字段：

- `id`
- `type`
- `name`
- `visible`
- `locked`
- `zIndex`
- `transform`
- `styleRef`
- `meta`
- `payload`
- `children`

`MoleculeFragment` 包含：

- `schema`
- `bbox`
- `nodes`
- `bonds`
- `meta`

`Node` 包含：

- 元素符号
- atomic number
- 位置
- charge
- `numHydrogens`
- label
- placeholder/external connection 标志
- meta

`Bond` 包含：

- begin/end 节点引用
- order
- double placement
- stereo
- stroke width/color
- bold/wedge/hash/spacing/margin 等绘图参数
- line styles/weights
- meta

评价：

- 文档模型已经能覆盖当前编辑器大量功能。
- 使用 `serde_json::Value` 存 style/meta/extra 有利于快速兼容格式，但长期 schema 严格性不足。
- object/resource 分离方向正确。
- 未来需要更明确的 migration、validation、schema 文档和容器设计。

## 9. 编辑命令与 Document Commit 合同

项目已有明确文档 `docs/document-commit-contract.zh-CN.md`。

核心定义：

- Document Commit 是实际修改文档、进入 undo/redo 历史、推进 revision 的一次变化。
- hover、focus、preview、工具切换、文本 caret movement、拖拽中的临时预览都不是 Document Commit。
- 拖动类操作应在 pointer up 或 finish 时只提交一次 revision。

`EditorCommand` 已覆盖：

- `add-bond`
- `add-arrow`
- `add-shape`
- `add-bracket`
- `add-symbol`
- `add-element`
- `add-orbital`
- `insert-template`
- `delete-selection`
- `cut-selection`
- `paste-clipboard`
- `apply-arrow-style`
- `apply-bond-style`
- `apply-text-style`
- `apply-shape-style`
- `apply-orbital-*`
- `apply-selection-arrange`
- `group-selection`
- `ungroup-selection`
- `move-selection`
- `rotate-selection`
- `resize-selection`
- `scale-selection`
- `apply-object-settings`
- `apply-document-style`
- `replace-hovered-endpoint-label`

评价：

- 命令边界是项目中很重要的工程资产。
- 目前命令仍有一些工具-specific 字段，长期可继续收敛为更稳定的 public API。
- history 使用 before/after snapshot，简单可靠，但大文档内存压力和增量 diff 未来需要评估。

## 10. 工具体系现状

内核 `Tool` 当前包括：

```text
select
bond
arrow
bracket
symbol
element
delete
text
shape
tlc-plate
orbital
templates
```

前端还引入了 `chain` 作为 UI active tool，但同步到 engine 时映射为 `templates + template=chain`。

### 10.1 选择工具

当前支持：

- 框选。
- 套索选择。
- selection move。
- rotate。
- resize/scale。
- arrange/order/group。
- 工具栏图标跟随框选/套索模式。

风险：

- 选择、临时选择、右键菜单、hover focus、对象内部 focus 之间交互复杂，需要 UI 端到端测试。

### 10.2 键工具

支持的 `BondVariant`：

```text
single
double
triple
dashed
dashed-double
bold
bold-dashed
wavy
wedge
hashed-wedge
hollow-wedge
```

已投入较多精修：

- 单键/双键/三键。
- 偏置双键、居中双键。
- 虚键和虚实双键。
- 楔形键和 hash wedge。
- 波浪键。
- 键端接触、交叉 whiteout、label clipping。
- ACS/CDXML 线宽参数和 ChemDraw 对齐。

风险：

- 视觉规则非常细，任何调整都可能引发导入文件或 EMF 预览回归。
- 需要持续扩大 fixture 和像素对比。

### 10.3 箭头工具

支持的 `ArrowVariant`：

```text
solid
curved
curved-mirror
hollow
open
equilibrium
unequal-equilibrium
```

支持：

- full head。
- left/right half head。
- head/tail。
- large/medium/small。
- curved arc 角度。
- no-go cross/hash。
- equilibrium 和 unequal equilibrium。

近期做过：

- 可逆箭头长度和箭头头大小逐渐放大规则对齐 ChemDraw。
- 不等长可逆箭头加入。
- 图标由内核生成后用于前端。

风险：

- 箭头特别容易出现“图标和实际绘制不一致”。
- 曲线半箭头视觉质量仍是高风险区域。
- EMF 输出要单独验证。

### 10.4 文本工具

支持：

- 文本对象。
- endpoint label 文本编辑。
- run 样式。
- bold/italic/underline。
- chemical/subscript/superscript。
- 文本对齐。
- 文本工具图标和格式图标内核输出，Times New Roman 风格。

风险：

- 浏览器文本、Rust glyph kernel、SVG、EMF、Word 预览之间保持一致很难。
- `glyph_profiles`、`glyph_outlines`、`glyph_clip_polygons` 是重要但复杂的数据资产。

### 10.5 括号与符号工具

`BracketKind` 同时覆盖括号和化学符号：

```text
round
square
curly
double-dagger
dagger
circle-plus
plus
radical-cation
lone-pair
circle-minus
minus
radical-anion
electron
```

支持：

- 括号对象。
- 电荷/自由基/孤对电子等符号。
- 符号弹窗和工具栏图标内核输出。

风险：

- `BracketKind` 同时表示 bracket 和 symbol，语义稍混合，但当前能工作。
- 如果未来符号扩展很多，建议拆成独立 enum。

### 10.6 元素周期表工具

当前支持：

- 右下角 P 按钮。
- 周期表 palette。
- 默认显示 P，不必显示当前选中元素。
- 选择元素后光标提示元素。
- 点击画布插入对应元素节点。
- 文本输入时点击元素可插入元素文本。
- 元素标签及隐式氢规则在内核里处理。

风险：

- UI 交互细节已经多次调整，必须有 Playwright 覆盖。
- 元素加氢规则和真实化学价态仍需扩大测试。

### 10.7 形状工具

`ShapeKind`：

```text
circle
ellipse
round-rect
rect
cross-table
tlc-plate
```

`ShapeStyle`：

```text
solid
dashed
shaded
filled
shadowed
```

支持：

- 4 类基础形状各 5 种样式，共 20 个样式图标。
- 田字形 shape。
- TLC 板。
- 形状工具图标内核生成。
- 当前形状图标采用更大尺寸内核绘制后缩小的策略，以改善圆角矩形等小图标失真。

风险：

- 图标缩放、线宽、前端覆盖规则容易造成“预览 SVG 和实际 UI 不一致”。

### 10.8 TLC 板

支持：

- TLC 板对象。
- spot/lane focus。
- spot 拖动。
- 拖动时显示 Rf 值。
- Rf 文本支持下标 f。
- Rf 当前显示格式趋向 `Rf = 0.1`。
- hover/drag 光标需要手形/抓手体验。

风险：

- TLC 是 shape 对象的特殊形式，交互语义容易和普通 shape resize/drag 混在一起。

### 10.9 轨道工具

`OrbitalTemplate`：

```text
s
p
dxy
oval
hybrid
dz2
lobe
```

`OrbitalStyle`：

```text
hollow
shaded
filled
```

支持：

- 轨道插入。
- 样式/phase。
- 轨道 focus 点。
- 部分轨道竖向默认方向。
- 工具栏和二级图标内核生成。
- CDXML 导入样式对齐推进中。

用户最近要求过：

- s/oval/lobe 支持 hollow/shaded/filled。
- 其他轨道删除不适用的 hollow 或 plus/minus。
- p、dxy、hybrid、dz2 默认方向和 ChemDraw 对齐。
- 轨道不能拖拽改大小，只能在选择工具里整体缩放。
- 轨道工具下可聚焦碳原子端点和标签字符中心，在锚点处绘制。

风险：

- 轨道属于 ChemDraw 特殊对象，导入、默认尺寸、旋转和 EMF 输出都需要持续对照。

### 10.10 模板与链工具

模板包括：

```text
ring-3
ring-4
ring-5
ring-6
ring-7
ring-8
chair-6-right
chair-6-left
benzene
chain
```

近期已加入链工具：

- 左侧多边形/模板工具下面有独立 chain 按钮。
- 前端 active tool 为 `chain`，同步到 engine 时使用 `templates` + `template=chain`。
- 空白拖拽生成 open zigzag chain。
- 端点拖拽复用 anchor node。
- click without drag 不插入 chain。
- preview 显示 terminal count label。
- 内核输出 chain tool SVG 图标，前端使用 `cc-kernel-chain-icon` 避免 CSS 覆盖。

链工具当前算法：

- 以拖拽起点为 anchor。
- 根据拖拽方向吸附到全局角度。
- 根据拖拽距离估算 bond count。
- 生成上下两个 zigzag phase。
- 选择末端更接近鼠标的 phase。

风险：

- 当前代码里的 `bond_count = (distance / side_length).round().max(1.0)`，是否完全等同 ChemDraw 仍需实机验证。
- ChemDraw chain tool 的旋转、起始键上下切换、count label 和吸附阈值非常细，应加入独立 fixture 或交互测试。

## 11. 化学语义能力

当前已有：

- 元素、atomic number、charge、numHydrogens。
- 隐式氢刷新。
- 元素标签识别。
- 选择统计 formula、formula weight、exact mass。
- ChemDraw 元素周期表颜色和加氢规则调研后引入的部分元素行为。
- 缩写识别和展开。
- 价态公式解析。
- 常见基团如 `Ph`、`Boc`、`FMOC`、`N3`、`t-Bu`、`CO2Et` 等在测试里有覆盖。

底部 selection chemistry summary：

- 无选中时不显示。
- 有选中时显示 formula、精度控件、Formula Weight、Exact Mass。
- formula 数字下标由前端渲染。
- 位数控件跟随 formula 后方，控制 Formula Weight 和 Exact Mass 的小数位。
- 氢统计来自内核公式逻辑，不是前端猜测。

风险：

- 这是化学编辑器最容易引发用户信任问题的部分。
- 当前公式和质量计算仍应标注支持范围，避免用户误认为已覆盖全部同位素、盐、配合物、query atom、reaction component 等。

## 12. 渲染系统

渲染层核心入口：

```rust
render_document(document: &ChemcoreDocument) -> Vec<RenderPrimitive>
```

`RenderPrimitive` 覆盖：

- line
- circle
- polygon
- rect
- ellipse
- polyline
- path
- filled path
- text

render role 用于区分：

- document ink
- knockout
- selection
- hover
- preview

当前渲染关注点：

- molecule bond geometry。
- endpoint contact kernel。
- label knockout 和 glyph polygon。
- dashed/wavy/wedge/hash。
- arrows。
- shapes。
- text。
- orbital/symbol。
- bounds。

SVG 输出：

- `document_to_svg`。
- viewer DOM/SVG renderer 消费 render list。

EMF 输出：

- Office crate 和 desktop Tauri 层都在推动 EMF preview/export。
- 有 Win32 GDI/GDI+ 相关 renderer。
- 有大量 EMF inspection/compare 脚本。

评价：

- 后端无关 primitives 方向正确。
- 视觉细节投入很深。
- EMF/Word 预览仍是高难度、易碎区域。
- renderer 文件仍较大，尤其 `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`。

## 13. 格式支持

### 13.1 Chemcore 自有格式

支持：

- `.ccjs`：可读 JSON。
- `.ccjz`：gzip JSON。

现状：

- v0.1 格式已文档化。
- 当前 `.ccjz` 仍是压缩 JSON，不是完整容器。
- 长期文档中建议演进为包含 manifest、document、preview、resources 的容器。

风险：

- 需要 schema validation。
- 需要 migration。
- 需要资源/预览容器策略。

### 13.2 CDXML

支持：

- Rust 原生 CDXML import/export。
- 解析 defaults、颜色、字体、fragment、bond、text、shape、line、bracket、orbital、TLC 等对象。
- CDXML export 写出 ChemDraw 可读结构。
- 保存 ChemDraw 相关 import meta。

重要代码：

```text
crates/chemcore-engine/src/cdxml.rs
crates/chemcore-engine/src/cdxml/export.rs
crates/chemcore-engine/src/cdxml/import_objects.rs
```

评价：

- CDXML 是当前最重要的外部兼容路径。
- 已有基础可用能力，但 ChemDraw CDXML 范围很大，不能认为完全兼容。

### 13.3 CDX

支持：

- `parse_cdx_document`
- `document_to_cdx`
- `cdx_to_cdxml`
- `cdxml_to_cdx`

实现策略：

- 读 CDX binary tree。
- 转成 CDXML-like tree/string。
- 再走现有 CDXML import。
- 导出时由 CDXML tree 编码回 CDX。

评价：

- 这是一个务实路线。
- 当前适合覆盖常见 CDX 文件，但不是完整 CDX 语义层。
- 需要扩大真实 ChemDraw CDX fixture。

### 13.4 SDF

支持：

- SDF/SD V2000 基础导入。
- 多 record 解析。
- data fields 读取/保留。
- SDF 导出 molecule records。
- V2000 上限 999 atoms/bonds。

产品策略：

- SDF 是分子结构交换格式，不保存完整文档样式、文本、箭头、形状、轨道、页面布局。
- 如果用户当前文件已经保存为 `.ccjz/.ccjs/.cdxml/.cdx` 且没有新修改，另存/导出 SDF/SVG/EMF 不需要额外提示破坏原保存状态。

评价：

- SDF 方向正确，但支持范围应明确标注。
- 需要 V3000、stereo、reaction/RXN、query data 等后续规划。

### 13.5 SVG/EMF/PDF

SVG：

- 当前可作为展示/导出格式。
- 不应承诺可完整还原编辑文档。

EMF：

- Windows 桌面/Office 预览重点。
- 可用于 Office fallback 和高质量预览。
- 仍需大量视觉回归。

PDF preview：

- 当前通过 WebView 将 SVG 预览栅格化封装成单页 PDF 的路径存在。
- 这更像预览导出，不是正式排版 PDF 引擎。

## 14. Browser viewer

路径：

```text
viewer/
```

主要模块：

```text
app.js                        主协调层，仍很大
app_dom.js                    DOM refs
engine_host.js                WasmEngineHost / DesktopHybridEngineHost / TauriEngineHost
toolbar.js                    toolbar HTML 和图标同步
editor_bindings.js            UI 事件绑定
editor_pointer_controller.js  pointer 路由
document_flow.js              打开/保存/导出流程
file_io.js                    文件格式识别、browser file APIs
scene_renderer.js             render primitive 到 SVG/DOM
editor_overlay.js             overlay
text_*                        文本编辑模型/渲染/控制器
color_host.js                 颜色 palette/dialog
object_settings_host.js       对象设置 host
numeric_dialog_host.js        数值对话框
```

viewer 当前能力：

- 浏览器端启动。
- 桌面 WebView 共用。
- 顶部工具栏、左侧工具栏、底部状态栏。
- 二级工具栏。
- 右下角元素/符号 palette。
- 文件打开/保存/另存为/导出。
- 文本编辑。
- selection summary。
- palette/dialog host。
- render list 绘制。
- 与 WASM/native host 同步。

风险：

- `viewer/app.js` 仍有约 3971 行，虽然已有拆分，但仍是前端复杂度集中点。
- UI 自动化测试相对不足。
- toolbar/icon/palette 状态与 engine tool state 的同步需要持续收敛。

## 15. EngineHost 与桌面混合运行时

`viewer/engine_host.js` 定义：

- `WasmEngineHost`
- `DesktopHybridEngineHost`
- `TauriEngineHost`

设计含义：

- 浏览器直接使用 WASM engine。
- 桌面默认使用 WebView 内 WASM engine 处理热编辑。
- 桌面 native service 处理文件、剪贴板、导出、Office 等系统能力。
- `TauriEngineHost` 只作为 native path 诊断和未来验证。

评价：

- 这是当前非常务实的选择。
- 它避免了每次 hover/pointer move 都跨 Tauri IPC。
- 风险是 WASM mirror 和 native session API 语义必须非常清楚，否则容易出现“调用名像改 native，实际改 mirror”的混乱。

## 16. Windows 桌面端

路径：

```text
apps/chemcore-desktop/src-tauri
crates/chemcore-desktop-service
```

Tauri 配置：

- productName: Chemcore
- Tauri 2.11
- WebView devUrl: `http://127.0.0.1:8767/viewer/`
- build 前重建 engine WASM。
- bundle 包含 `chemcore-office.exe`。
- 文件关联：
  - `.ccjz`
  - `.ccjs`
  - `.cdxml`
  - `.cdx`
  - `.sdf`
  - `.sd`

桌面能力：

- 窗口、标题栏、菜单。
- single instance。
- startup path。
- 拖拽打开文件。
- open/save/save as/export dialogs。
- 最近文件。
- native clipboard。
- EMF export。
- detachable document/window。
- native engine session commands。

当前拆分状态：

- `apps/chemcore-desktop/src-tauri/src/lib.rs` 已比之前拆出 `commands/`、`menus.rs`、`paths.rs`、`window_helpers.rs`、`desktop_emf.rs`。
- `commands/engine.rs` 仍较大。
- `crates/chemcore-desktop-service/src/lib.rs` 仍较大，但已拆出 `document_io.rs`、`file_format.rs`、`recent_files.rs`、`render_bounds.rs`、`tool_parsing.rs`。

评价：

- 桌面主线已经跑起来。
- 结构已开始拆分，但 desktop service facade 和 engine command bridge 仍需要继续瘦身。

## 17. Office/OLE 集成

路径：

```text
apps/chemcore-office
```

定位：

- 独立 COM local server。
- 不和桌面 app 生命周期绑死。
- 目标是 Office 中可嵌入、可预览、可双击编辑的 Chemcore object。

当前能力信号：

- user/machine scope 注册/反注册。
- `IClassFactory`。
- `IOleObject`。
- `IDataObject`。
- `IPersistStorage`。
- `IViewObject2`。
- `IRunnableObject`。
- OLE storage stream。
- OLE clipboard payload。
- `CF_ENHMETAFILE`。
- `ChemcoreDocument` stream。
- `ChemcorePreviewSvg`。
- `\x02OlePres001`。
- `\x03EPRINT`。
- Word OOXML package writer。
- EMF presentation renderer。
- self-test 脚本和 Word 自动化验证脚本。

固定身份：

```text
Display name:       Chemcore Document
ProgID:             Chemcore.Document
Versioned ProgID:   Chemcore.Document.1
CLSID:              {CB69F54F-F21E-44DE-84FB-89D98FECE056}
Local server:       chemcore-office.exe
```

评价：

- 这部分技术价值很高。
- 复杂度也最高。
- Office/OLE 是最容易出现环境差异、Word 版本差异、预览差异和调试成本爆炸的部分。
- 目前应保持边界清楚：Office 层只做嵌入、存储、预览、激活和回写，不承担化学语义。

## 18. 构建与验证命令

常用命令：

```bash
cargo test
npm run build:engine-wasm
npm run dev:engine
npm run verify
npm run desktop:dev
npm run desktop:build
npm run office:self-test
npm run office:register-dev
npm run office:unregister-dev
```

`npm run verify` 当前做：

```text
cargo test
node scripts/build-engine-wasm.mjs
node --check viewer/app.js
检查 viewer/engine 生成物是否与 Git 状态同步
```

`npm run test` 当前做：

```text
cargo test
node --check viewer/app.js
```

测试数量扫描：

- `rg "#[test]"` 扫描到约 504 个测试点。
- `bond_tool.rs` 有约 181 个测试。
- `render_document.rs` 有约 150 个测试。
- `text_tool.rs` 有约 41 个测试。
- Office EMF renderer、desktop service、label rules、abbreviation、render 等也有单元测试。

测试优势：

- Rust engine 侧测试很密。
- 键、渲染、文本、palette、special objects、desktop service 都有覆盖。

测试缺口：

- viewer 端到端自动化仍不够。
- 桌面端真实窗口测试不系统。
- Office/OLE 自动化仍很难，但必须继续做。
- CDXML/CDX/SDF fixture 覆盖需要更多真实文件。
- 视觉像素回归需要制度化。

## 19. 脚本和调研资产

`scripts/` 很丰富，尤其是：

- ChemDraw oracle。
- SVG pixel compare。
- EMF inspect/render。
- Word clipboard paste validation。
- Word OLE roundtrip validation。
- glyph profile/outlines/clip polygon generation。
- label anchor regression。
- text editor regression。
- viewer screenshot。
- Playwright browser helper。
- 多个 PNG/region/IoU/attribution 分析脚本。

这说明项目不是只靠肉眼调参，已经有一套实验和对照工具链。

风险：

- 脚本很多，入口和依赖环境需要文档化。
- 有些脚本依赖 Windows、Office、PowerShell、Python 环境、ChemDraw 实机，外部协作者复现成本较高。

## 20. 文档资产

重要文档：

```text
docs/architecture.zh-CN.md
docs/rust-engine-architecture.zh-CN.md
docs/format-v0.1.zh-CN.md
docs/document-commit-contract.zh-CN.md
docs/windows-desktop-office-architecture.zh-CN.md
docs/project-rules.zh-CN.md
docs/bond-rendering-rules.zh-CN.md
docs/implicit-hydrogen-rules.zh-CN.md
docs/abbreviation-recognition-rules.zh-CN.md
docs/right-click-context-menu-matrix.zh-CN.md
docs/large-module-refactor-plan.zh-CN.md
docs/project-evaluation-2026-06-10.zh-CN.md
```

评价：

- 文档意识非常好。
- 很多规则已经被显式写下来。
- 这对后续避免反复争论很有价值。

问题：

- 部分中文文件或 package description 曾出现编码乱码，需要统一检查 UTF-8。
- 文档数量多，需要一个“当前有效文档索引”，否则新参与者不知道读哪几份。

## 21. 当前完成度评估

粗略完成度，不是严格项目管理数值：

```text
Rust engine 基础架构            75%
文档模型 v0.1                  65%
编辑命令/undo/redo             70%
键/文本/箭头/形状/模板工具      65%
轨道/TLC/特殊对象              50%
元素周期表/化学统计             55%
渲染 primitives/SVG             70%
EMF/Office 预览                 45%
CDXML import/export             65%
CDX import/export               40%
SDF import/export               35%
Browser viewer                  65%
Windows desktop                 65%
Office/OLE 集成                 45%
测试体系                        60%
产品发行准备                    30%
```

如果按“是否能作为正式产品替代 ChemDraw”：

```text
还不能。
```

如果按“是否已经完成核心技术路线验证”：

```text
已经完成相当大一部分。
```

如果按“是否值得继续投入”：

```text
值得。
```

## 22. 主要优势

### 22.1 一核多壳方向正确

同一套 Rust core 支撑浏览器、桌面和 Office，是项目最重要的架构优势。

### 22.2 化学行为逐步从前端收回内核

这能避免前端、桌面和 Office 出现三套规则。

### 22.3 ChemDraw 兼容投入真实

项目已经在认真对齐 CDXML/CDX、ACS 线宽、箭头、轨道、glyph、EMF、Word 行为。这是专业工具的护城河。

### 22.4 测试和调研工具链有基础

大量 Rust 测试、视觉对比脚本、Office/Word 验证脚本说明项目具备持续收敛能力。

### 22.5 免费策略有传播价值

如果产品稳定，全免费能降低用户尝试门槛，尤其适合科研、学生和轻量用户。后续项目收费可以依托 Chemcore 获得信任和入口。

## 23. 主要风险

### 23.1 格式兼容没有自然终点

ChemDraw 生态太大。CDXML/CDX 的对象、样式、版本差异、Office 行为都很多，必须建立 fixture matrix。

### 23.2 Office/OLE 很容易拖慢项目

OLE 是高价值但高风险模块。它可能吞掉大量时间，且很多 bug 只能在特定 Office/Windows 环境复现。

### 23.3 前端协调层仍较大

`viewer/app.js` 仍约 3971 行，继续加 UI 功能会增加维护成本。

### 23.4 native/WASM session 同步语义复杂

DesktopHybrid 是正确路线，但必须明确：

- 谁是热编辑权威。
- 谁负责系统能力。
- 哪些 API 是 mirror。
- 哪些 API 是 native session。

### 23.5 图标体系曾反复不一致

最近多次围绕“内核生成 SVG、前端不要二次覆盖、线宽统一”调整。说明图标和真实渲染需要制度化：

- 所有 kernel icon class 要防止 CSS 覆盖。
- toolbar primary icon 要同步当前样式。
- 预览 SVG、前端 UI、实际绘制应共用参数。

### 23.6 自有格式还需要硬化

`.ccjz` 当前本质是 gzip JSON。长期作为产品格式，需要：

- 容器。
- manifest。
- preview。
- resource。
- migration。
- validation。

### 23.7 化学语义支持范围需要明确

用户看到 formula weight/exact mass 会默认相信结果。必须明确：

- 隐式氢规则范围。
- 同位素支持范围。
- 盐/配合物支持范围。
- query atom 支持范围。
- reaction 支持范围。

## 24. 建议的下一阶段路线

### 24.1 稳定产品主路径

优先保证：

- 新建文档。
- 画常见分子。
- 文本/箭头/括号/形状。
- 保存 `.ccjz/.ccjs`。
- 打开旧文件。
- 导入/导出 CDXML/CDX。
- SDF 作为结构导入导出。
- SVG/EMF 导出。
- 桌面端文件关联。
- 基础 Office 粘贴/预览。

这些比继续扩展小众工具更重要。

### 24.2 建立格式 fixture 回归库

建议分目录：

```text
tests/fixtures/chemdraw/cdxml/basic
tests/fixtures/chemdraw/cdxml/text
tests/fixtures/chemdraw/cdxml/bonds
tests/fixtures/chemdraw/cdxml/arrows
tests/fixtures/chemdraw/cdxml/orbitals
tests/fixtures/chemdraw/cdx/basic
tests/fixtures/sdf/v2000
tests/fixtures/sdf/v3000
```

每个 fixture 记录：

- 来源。
- ChemDraw 版本。
- 预期导入对象。
- 预期导出是否 roundtrip。
- 是否需要像素 oracle。

### 24.3 继续拆大文件

优先：

- `viewer/app.js`
- `crates/chemcore-desktop-service/src/lib.rs`
- `apps/chemcore-desktop/src-tauri/src/commands/engine.rs`
- `apps/chemcore-office/src/windows_office.rs`
- `apps/chemcore-office/src/windows_office/emf_preview/renderer.rs`

原则：

- 只拆职责，不改行为。
- 每次拆分可单独验证。
- 不在 refactor 里混 UI 调整。

### 24.4 加强 UI 自动化

至少覆盖：

- 浏览器启动。
- 桌面启动。
- 工具栏切换。
- 右下角 palette 展开/收回。
- 元素周期表/符号工具互斥。
- 链工具拖拽。
- 保存/另存为/导出提示。
- selection summary。
- 撤销/重做。

### 24.5 明确免费与开源边界

如果软件本体全免费、部分代码开源，建议考虑：

- 开源 viewer 壳、文档格式规范、部分工具 API、测试样例。
- 暂不全开源核心 ChemDraw 兼容、Office/OLE、EMF 精修、格式转换细节。
- 用 GitHub 托管公开 issue/docs/release，但可以采用 private monorepo + public mirror。
- 如果想获得社区贡献，需要开源足够可构建、可运行的部分，否则只开文档意义有限。

### 24.6 申请软著/商标/品牌

建议：

- 申请软件著作权可作为国内发布、合作、上架、维权的低成本证明。
- 更重要的是保留 Git commit 记录、设计文档、发布包 hash、官网发布时间。
- 如果要长期做品牌，商标可能比软著更关键。

## 25. 可给 ChatGPT 继续分析的问题

可以把本报告发给 ChatGPT 后，重点问这些问题：

1. 这个项目作为免费入口产品，最小可发布版本应该包含哪些功能？
2. 哪些模块适合开源，哪些不适合开源？
3. 当前一核多壳架构有没有明显长期风险？
4. DesktopHybridEngineHost 这个设计是否合理？如何降低 mirror/native session 同步风险？
5. CDXML/CDX/SDF 的测试矩阵应该怎么设计？
6. Office/OLE 是否值得作为第一版核心卖点，还是先作为实验功能？
7. `.ccjz` 长期应该采用 gzip JSON、zip package，还是自定义容器？
8. 如何设计对用户透明的 lossy export 提示策略？
9. 哪些功能最可能导致用户信任崩塌？
10. 如果只有 1 个月冲刺，应该优先修哪些稳定性问题？
11. 如果要申请软著和准备公开发布，缺哪些材料？
12. 如果部分开源，仓库拆分策略应该怎么做？

## 26. 建议给外部模型的提示词

可以直接复制下面这段：

```text
你是一位同时懂化学绘图软件、Rust/Tauri/Web 架构、文件格式兼容和桌面产品商业化的技术顾问。

下面是一份 Chemcore 项目全量总结报告。请你基于报告做二次分析：

1. 判断项目技术路线是否合理。
2. 判断当前完成度和最大风险。
3. 给出最小可发布版本范围。
4. 给出开源/闭源边界建议。
5. 给出接下来 1 个月、3 个月、6 个月的路线图。
6. 特别关注 ChemDraw 兼容、Office/OLE、CDXML/CDX/SDF、WASM/native hybrid、测试矩阵和产品传播策略。
7. 请指出报告中可能过度乐观或需要验证的假设。

请用中文回答，要求具体、可执行、分优先级。
```

## 27. 最终判断

Chemcore 是一个有真实技术含量和长期产品价值的项目。它的核心意义不只是做一个免费化学画图软件，而是建立一套可复用、可测试、可跨平台、可嵌入 Office 的化学文档内核。

当前项目已经具备：

- Rust 核心。
- 可编辑 viewer。
- Windows 桌面壳。
- Office/OLE 原型。
- CDXML/CDX/SDF 路径。
- SVG/EMF 输出。
- 大量测试和调研脚本。
- 清晰的架构文档。

但还需要继续完成：

- 产品级稳定性。
- 格式 fixture 回归。
- UI 自动化。
- Office/OLE 真实工作流验证。
- 自有格式容器化。
- 大模块继续拆分。
- 发布和开源边界设计。

综合评价：

```text
项目意义：高
技术路线：正确
工程完成度：中高
产品完成度：中
格式兼容成熟度：中
Office/OLE 成熟度：中低到中
发行准备度：偏低
长期潜力：高
```

最值得坚持的原则：

```text
化学语义和文档 mutation 继续留在 Rust engine。
前端、桌面和 Office 只做宿主和适配。
所有视觉/格式兼容问题都尽量变成可复跑 fixture，而不是靠一次肉眼调参。
```
