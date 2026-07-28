# CDX/CDXML 原生绘制与编辑缺口清单

本文承接 [CDX/CDXML 字段复核总账](cdx-cdxml-field-verification.zh-CN.md)，专门记录“已经能够解析或无损保存，但尚未完整进入内核原生模型、编辑器和绘制器”的部分。

字段总账里的 `verified` 只说明对应的官方定义、存储或已记录行为已经复核，**不等于原生绘制已经完成**。本清单才是原生支持的实施账本。

## 状态定义

每一项分别检查四层，不再用一个状态概括整条链路：

- **解析**：能否从 CDX/CDXML 读成明确字段；`raw` 仅表示原始载荷可保留。
- **编辑**：能否通过 CCJS/命令/API 修改明确字段，而不是修改 `face` 或不透明字节。
- **绘制**：SVG、PNG、EMF 和 GUI 是否由同一套原生语义正确绘制。
- **往返**：导出再导入时，字段、对象关系和视觉结果是否稳定。

状态取值：`完成`、`部分`、`raw`、`未做`、`不适用`。只有四层均达到本项验收条件，任务才可以勾选。

## 完成一项的统一门槛

每项都必须按稳定规则实现，不按单个样例写分支。勾选前至少完成：

1. 查清官方 CDX 属性表、CDXML DTD 和实际 ChemDraw 文件行为；若版本行为冲突，按版本写明分支依据。
2. 在 CCJS 中增加来源无关的明确字段，并补 schema、默认值、序列化和编辑入口。
3. CDX 与 CDXML 导入、导出均覆盖；未知值仍能无损保留。
4. SVG、PNG、EMF 和 GUI 共用同一语义，不允许只修某一个导出器。
5. 建立公开或可提交的最小夹具，覆盖默认值、非默认值、边界值和组合情况。
6. 与 ChemDraw 参考图做对象位置、标签、线型和细节门禁；像素门禁不得被画布大小稀释。
7. 增加回归测试并更新本清单、字段总账和相关格式文档。

## 实施顺序

| 顺序 | ID | 工作包 | 当前解析 | 当前编辑 | 当前绘制 | 当前往返 | 状态 |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | `NR-001` | 嵌入对象预览 | 完成 | 完成 | 完成 | 完成 | [x] |
| 2 | `NR-002` | 原子同位素、丰度、自由基和原子编号 | 完成 | 完成 | 完成 | 完成 | [x] |
| 3 | `NR-003` | 原子查询与碳标签显示规则 | 完成 | 完成 | 完成 | 完成 | [x] |
| 4 | `NR-004` | 键查询、反应属性和显示标记 | 完成 | 完成 | 完成 | 完成 | [x] |
| 5 | `NR-005` | 外部连接点视觉类型 | 完成 | 完成 | 完成 | 完成 | [x] |
| 6 | `NR-006` | 光谱对象 | 完成 | 完成 | 完成 | 完成 | [x] |
| 7 | `NR-007` | ChemicalProperty | 完成 | 完成 | 完成 | 完成 | [x] |
| 8 | `NR-008` | Geometry 与 Constraint | 完成 | 完成 | 完成 | 完成 | [x] |
| 9 | `NR-009` | ColoredMolecularArea 与结构高亮 | 完成 | 完成 | 完成 | 完成 | [x] |
| 10 | `NR-010` | Table Tool、表格与单元格 Border | 完成 | 完成 | 完成 | 完成 | [x] |
| 11 | `NR-011` | StoichiometryGrid | 完成 | 完成 | 完成 | 完成 | [x] |
| 12 | `NR-012` | 凝胶电泳对象 | 完成 | 完成 | 完成 | 完成 | [x] |
| 13 | `NR-013` | 质粒图对象 | 完成 | 完成 | 完成 | 完成 | [x] |
| 14 | `NR-014` | BioShape/BioDraw 对象 | 完成 | 完成 | 校准中 | 完成 | [ ] |
| 15 | `NR-015` | TemplateGrid | 完成 | 完成 | 完成 | 完成 | [x] |
| 16 | `NR-016` | 文档页眉、页脚和分页布局 | 完成 | 部分 | 未做 | 部分 | [ ] |
| 17 | `NR-017` | 逻辑对象的原生语义编辑 | 完成 | 完成 | 不适用 | 完成 | [x] |
| 18 | `NR-018` | 旧式复合载荷预览与图片裁剪 | 完成 | 部分 | 部分 | 完成 | [ ] |

## 逐项范围与验收

### [x] NR-001 嵌入对象预览

**范围**：`embeddedobject` 以及 EMF、WMF、OLE、PNG、JPEG、GIF、TIFF、BMP、PDF、PICT；包括压缩的 OLE、WMF、EMF 载荷。

**现状**：载荷大多能以原始字节保留，但原生场景没有预览对象，因此包含图片或 Office 载荷的文件可能显示为空白。

**目标**：区分“可直接解码的图片”“可提取预览的复合载荷”“只能占位的未知载荷”；保留原始字节的同时，生成稳定的原生预览和边界框。

**验收**：各格式至少一个公开夹具；缩放、裁剪、旋转、层级和导出尺寸与 ChemDraw 一致；无法解码时显示有尺寸的占位而不是静默空白；往返不改原载荷。

**2026-07-23 进展**：

- PNG、JPEG、GIF、BMP 已映射为 CCJS 原生 `image` 对象和显式 `image` 资源；CDX/CDXML 二进制、边界框、旋转角和层级可稳定往返。
- GUI、SVG、PNG 和 EMF 已接入同一 `RenderPrimitive::Image` 语义；无法解码的复合载荷显示带格式名和尺寸的占位图，不再静默空白。
- 图片文件拖入、浏览器/Windows 剪贴板粘贴，以及空白处右键“Insert Image...”打开文件浏览器，统一进入通用 `add-image` 场景对象链路；移动、边拉伸、角等比缩放、旋转、层级、组合、复制粘贴、删除和撤销沿用同一对象行为。
- 插入边界固定为 64 MiB、32768 像素单边和一亿总像素，并校验 MIME 签名与实际图片尺寸。
- 本项按“原生图片对象与确定性占位”范围关闭：可解码位图具有完整原生对象行为；无法解码的复合载荷具有稳定、有尺寸的占位，且原字节无损往返。
- EMF/WMF/OLE/TIFF/PDF/PICT 的内容级预览提取和图片裁剪依赖独立的容器/编解码规则，不再混入原生图片对象验收，已拆为 `NR-018`。

### [x] NR-002 原子同位素、丰度、自由基和原子编号

**范围**：`Isotope`、`IsotopicAbundance`、`Radical`、`AtomNumber`、`ShowAtomNumber`、`AS`、`ShowAtomStereo`。

**现状**：导入层能识别主要字段，但原生标签布局没有完整绘出同位素、丰度、自由基点、原子编号和立体标记。

**目标**：将这些信息建模为原子标签的独立装饰层，统一参与字形测量、锚点、退让、遮挡和导出。

**验收**：元素标签在四个方向、不同字体/字号、带电荷/氢/映射号的组合均与 ChemDraw 对齐；自由基的单点、双点和不同状态不互相覆盖。

**2026-07-23 进展**：

- CCJS 已新增来源无关的 `node.atomProperties`，CDX/CDXML 的七组字段可解析、编辑并稳定往返；
- 原子右键菜单已接入同位素、丰度、自由基、编号和 CIP 标记的统一可撤销命令；
- 点工具点击原子会生成可拖动电子点并同步有效自由基、隐式氢和价态；CDX/CDXML 导出写出有效自由基语义；
- GUI、SVG、PNG 与 EMF 共用原子装饰渲染器；ChemDraw 后台组合探针确认了 `0.75` 字号、查询标记 `I`、`0.1875em` 水平间隙、编号换侧和斜体括号 CIP 标记规则；
- 导入的 `number/query/stereo` object tag 现在保留承载原子 ID，避免与原生装饰重复绘制，语义编辑会同步已有显示对象；
- 详细规则与验证入口见 [原子属性编辑与装饰规则](atom-property-editing-rules.zh-CN.md)。
- 2026-07-23 正式关闭：CCJS、命令入口、CDX/CDXML 往返、共享渲染与回归测试均已覆盖；后续发现的组合布局误差按原子装饰通用规则回归，不重新打开字段建模工作包。

### [x] NR-003 原子查询与碳标签显示规则

**范围**：`ElementList`、`GenericList`、`FreeSites`、`RingBondCount`、`UnsaturatedBonds`、`SubstituentsUpTo`、`SubstituentsExactly`、`Translation`、`AbnormalValence`、`ShowTerminalCarbonLabels`、`ShowNonTerminalCarbonLabels`。

**现状**：部分查询值能保留，`ImplicitHydrogens` 等已有局部绘制；完整查询装饰和碳标签显隐尚未形成统一规则。

**目标**：明确每个查询条件的显示文本、组合顺序、默认隐藏行为和与普通原子标签的布局关系。

**验收**：单条件和多条件组合均有夹具；显式碳、隐式碳、端点和非端点规则与 ChemDraw 一致；不以文件名或样例 ID 分支。

**2026-07-23 完成**：

- ChemDraw 静默探针覆盖所有枚举、单字段、组合、显隐、四方向、Arial/Times New Roman 和 8/10/14 pt；探针脚本可重复执行。
- `node.atomProperties` 已加入元素/通用列表、自由位点、环键、饱和性、取代数、翻译、异常价态、查询显隐和两类碳标签覆盖字段。
- CDXML 与 CDX 均按官方 tag/词法往返；已补齐 CDX 中三个枚举的名称/数值映射，列表继续使用官方 `CDXElementList`/`CDXGenericList` 编解码。
- 查询短码按 `X/U/* → S → R → L → I` 合并绘制，采用实测字号、Symbol 星号和连接反侧布局；导入的 ChemDraw 缓存 object tag 不再重复显示。
- 元素列表与通用列表可混合编辑并生成 `NOT N, O, R, X` 一类原生标签；异常价态进入化学检查与隐式氢规则；碳标签按节点覆盖、文档默认和实际连接数即时更新。
- 原子右键 “Atom Query” 已提供所有字段入口，统一走可撤销命令；公开夹具、CCJS/CDXML/CDX 往返测试、绘制测试和菜单 schema 测试已加入。
- 完整规则见 [原子查询与碳标签编辑规则](atom-query-editing-rules.zh-CN.md)。

### [x] NR-004 键查询、反应属性和显示标记

**范围**：原子的 `RxnChange`、`RxnStereo`；键的 `Topology`、`RxnParticipation`、`ShowBondQuery`、`ShowBondStereo`、`ShowBondRxn`。

**现状**：多键型的 `S/A/D/T` 查询标签已有部分实现，其他反应和立体显示标记尚不完整。

**目标**：把查询、反应变化和立体注记作为键的明确装饰语义，统一位置、方向、避让和组合优先级。

**验收**：水平、垂直、斜向、短键、交叉键和环内键均覆盖；SVG 与 EMF 的位置和线帽一致；显隐字段严格控制输出。

**2026-07-23 完成**：

- 静默 ChemDraw 探针覆盖查询键级、拓扑、八种反应参与、`BS`、原子反应字段、文档/对象显隐继承、端点反转以及水平/垂直/斜向；同时核对 CDX 原始枚举值和 SVG/EMF 输出。
- CCJS 新增来源无关的 `atomProperties.reactionChange/reactionStereo` 与 `bond.properties`；查询、反应和 E/Z 不再藏在 `meta` 或伪文本对象中。
- CDX/CDXML 导入导出按官方 tag、位集合、枚举和 implied boolean 双向映射；未指定立体不再被擅自写成 `BS="N"`。
- 查询文本固定按 `Topology + Rxn + Order` 组合，原子短码加入 `C/T`；键标注采用与端点顺序无关的规范轴和实测的按轴分离偏移函数，不做样例特判或 360 度搜索。
- GUI、SVG、PNG 和 EMF 共用 `RenderPrimitive::Text`；右键菜单和 JSON 命令可编辑全部原生字段，并支持继承、撤销、复制和往返。
- CCJS/CDXML/CDX 往返、命令、菜单、绘制和既有全量内核回归均已覆盖。完整规则见 [键查询、反应属性与显示标记规则](bond-query-reaction-editing-rules.zh-CN.md)。

### [x] NR-005 外部连接点视觉类型

**范围**：`ExternalConnectionType` 及其不同视觉形式。

**现状**：已完成。

**目标**：逐一复核枚举的端点几何、方向和连接后行为，并纳入键端退让。

**验收**：每个官方枚举均有 ChemDraw 对照；连接与未连接、旋转和缩放行为稳定。

**完成记录（2026-07-23）**：

- 静默 ChemDraw 探针覆盖缺省值、0–12 全枚举、字号、线宽、键长、方向、未连接和多键状态，并同时保存 CDXML、CDX、SVG 与 EMF 证据；`Wavy` 未连接时 ChemDraw 的 EMF 导出会使 COM 服务异常，因此可重复探针明确排除此无效组合。
- CCJS 使用唯一原生对象 `node.externalConnection = { type, number? }`；旧 `isExternalConnectionPoint` 仅在 JSON 读取边界迁移，写出不再产生旧布尔字段，也不借用 `AtomNumber`。
- CDX `0x0440` 已按实际 ChemDraw 枚举 0–12 完整编解码；CDXML 的 `ExternalConnectionType` 与 `ExternalConnectionNum` 原生导入、编辑和导出。
- 默认/菱形、星形、聚合物珠、波浪、灰色扁菱形生物类型及无编号黑菱形均由共享绘制原语生成，GUI、SVG、PNG、EMF 共用；键端按照标记边界退让，`Wavy` 明确保持键到中心。
- 右键菜单可创建、切换、移除全部类型并编辑连接编号；转换、撤销、复制和 CCJS/CDX/CDXML 往返均使用同一字段。
- 实测尺寸函数、编号行为和方向规则见 [外部连接点规则](external-connection-rules.zh-CN.md)。

### [x] NR-006 光谱对象

**范围**：`spectrum`、坐标轴、数据载荷、`Class`、`XAxisLabel`、`XLow`、`XSpacing`、`XType`、`YAxisLabel`、`YLow`、`YScale`、`YType` 等。

**现状**：对象和字段能够进入交换层，但没有原生坐标系、曲线和标签绘制。

**目标**：建立来源无关的光谱模型，明确数据解码、轴范围、缩放、刻度、标签和边界框。

**验收**：至少覆盖 NMR、IR、UV/Vis 或官方可生成的主要类型；数据点、坐标轴和标签在 SVG/EMF 中一致；未知数据编码仍可往返。

**完成记录（2026-07-23）**：

- 静默 ChemDraw 探针覆盖 NMR、IR、Y 存储变换、反向边界框和字体/线宽样式，并保留 CDXML、CDX、SVG、EMF 证据；规则与可复跑入口见 [光谱对象规则](spectrum-object-rules.zh-CN.md)。
- CCJS 新增一等 `spectrum` 对象及显式 `payload.spectrum`：完整覆盖 Class、X/Y 类型、轴标签、X 起点/间距、Y 偏移/缩放和双精度数据数组，不把源格式字段藏入 `meta`。
- CDXML 保留 `YLow + raw * YScale` 存储语义；CDX 按 ChemDraw 规则将整组实际值写入单个 `0x0A86` 双精度数组属性，并完成全部枚举数值映射。
- GUI、SVG、PNG、EMF 共用线、折线和文本原语；第一个采样位于右端，Y 范围按实测规则扩展 5%，窄峰通过有上限的极值保留降采样绘制。
- 光谱支持数据命令编辑、移动、拉伸、组合、层级、颜色、线宽、复制粘贴、删除、锁定、显隐、撤销和跨格式导出；旋转被明确禁止，不存在预测入口或预测实现。
- CCJS 严格校验、CDXML/CDX 往返、绘制、编辑、删除后不复活以及大数组降采样均已纳入内核回归。

### [x] NR-007 ChemicalProperty

**范围**：`chemicalproperty` 的显示文本、关联对象、位置、计算结果和格式字段。

**现状**：语义字段可保留，但缺少原生显示、重新计算和锚定规则。

**目标**：区分“文件提供值”和“可重新计算值”，保持显示位置和来源关系可编辑。

**验收**：值为空、固定值、重新计算、关联对象移动和删除均有确定行为；导入后不擅自改变原值。

**完成记录（2026-07-24）**：

- CCJS 新增一等 `chemicalProperties` 逻辑对象，明确保存类型码/名称、有序 `BasisObjects`、显示对象、激活状态、值来源、计算状态和最后计算值；不存在的类型、显式 `Unspecified`、`ChemicalName` 与大于 `0x8000` 的自定义 CDX 类型严格区分。
- CDXML/CDX 导入把 `chemicalproperty` 提升为原生语义，显示仍复用标准文本对象；导出根据当前对象 ID 重建 `BasisObjects` 和 `ChemicalPropertyDisplayID`，删除后的源对象不会由交换层复活。
- 右键 “Chemical Property...” 可对完整单分子创建、对显示文本编辑或删除属性；内核提供同一对话框 schema，Web、桌面和 WASM 走同一可撤销命令。
- 激活的 `ChemicalName` 使用 Chemical Graph V2 的规范结构指纹判定失效：平移、旋转和布局变化不触发重新计算，拓扑、元素、键级等结构变化才标为 `stale`；请求使用版本化命名接口，结果写回显示文本并标为 `current`。
- 文件提供值在导入时原样保留；固定值不自动变化。手工编辑活动显示文本会明确关闭自动更新并显示提示。删除显示文本只移除显示关联；删除最后一个 basis 会删除逻辑属性并把显示文本降为普通文本。
- `chemical-property-display` 是标准 ChemicalProperty 关系在内核 Link 图中的原生表示，与通用自动 Link 推断分开；复制粘贴、Alt+双击、删除、撤销和 CCJS 保存重开均重映射并验证端点。
- CDXML 允许仅有名称的自定义类型；CDX 无法表示这种类型，导出会明确报错，绝不静默省略。带数值码的未知自定义类型可稳定 CDX 往返。
- 空属性、活动/固定、结构失效、结果回填、显示和 basis 删除、复制粘贴、CDXML/CDX 引用重写及菜单入口均已有回归；详细契约见 [ChemicalProperty 原生模型](chemical-property-model.zh-CN.md)。

### [x] NR-008 Geometry 与 Constraint

**范围**：`geometry`、`constraint` 及其引用对象、几何类型、约束值和显示属性。

**现状**：已建立原生 Geometry/Constraint 对象、强 basis 引用、递归求值器、共享渲染和内核属性对话框。

**目标**：建立对象引用、测量/约束值与可视辅助线之间的明确模型。

**验收**：引用失效、对象移动、角度/距离等主要类型均有规则；显示和隐藏不影响约束数据往返。

**2026-07-26 完成**：

- CCJS 以明确字段覆盖全部官方 Geometry/Constraint 类型、显示文字、指示线、字体样式与 `auto`/`angle`/`offset`/`absolute` 定位；依赖图允许派生对象继续作为 basis，并明确拒绝循环和类型不匹配。
- CDXML/CDX 均按官方对象与 INT8 枚举读写 `GeometricFeature`、`ConstraintType`、`BasisObjects`、范围和布尔属性；CDX 导出统一重写对象 ID 与强引用。
- GUI、SVG、PNG 与 EMF 共用递归求值和渲染语义。basis 拖动时实时重算；单独拖标注只产生临时预览，松手回位且不写文档、不触发自动保存、不进入撤销栈。
- 删除 basis 会递归级联删除依赖标注；同标签页、跨标签页和跨端复制只在 basis 完整时携带标注，并统一重映射强引用。
- 右键菜单按有序选择签名仅显示合法类型；显式标签原子与隐式原子统一作为点。属性修改使用内核生成的对话框 schema，原生标注没有缩放或旋转柄。
- 规则、ChemDraw 实测和验收边界见 [Geometry / Constraint 原生对象设计](geometry-constraint-model.zh-CN.md)；解析、往返、递归求值、生命周期、拖拽和真实浏览器右键对话框均有回归。

### [x] NR-009 ColoredMolecularArea 与结构高亮

**范围**：`coloredmoleculararea` 的环键、颜色、范围和显示层级，以及原子/键 `highlightColor` 结构高亮。

**现状**：已原生解析、编辑、绘制并无损往返。

**目标**：根据分子拓扑和官方规则生成稳定区域，不把 ChemDraw 的缓存几何当作唯一语义。

**验收**：结构高亮覆盖开链、环和任意原子/键选择；环填充覆盖单环与稠环的无弦环，拒绝不完整环和跨片段引用；不同颜色、删除、复制与分子编辑均正确更新。官方 `coloredmoleculararea` 没有透明度字段，禁止借用 root/page 的 `alpha`/`bgalpha` 或发明有损字段。

**规则与证据**：见 [分子着色与结构高亮规则](molecular-coloring-rules.zh-CN.md)。

### [x] NR-010 Table Tool、表格与单元格 Border

**范围**：`table`、作为单元格的 `page`、单元格子对象和 `border`。

**官方语义纠正**：`border` 不是独立场景对象。官方 SDK 明确规定它只作为表格单元格 `page` 的一条边存在；相邻单元格各自保存共享边的一份描述，冲突值没有定义。因此不得再把它设计成可自由放置的顶层图形，也不得把 `table` 压平成固定 2×2 的 `shape/crossTable`。

**完成状态**：已建立原生 `table` 对象、任意行列导线、逐单元格内容引用、四边覆盖、隐藏边、Solid/Dashed/Bold/Wavy 字段、CDX/CDXML/CCJS 往返、SVG/EMF 共用渲染、选择工具移动与拉伸、Table Tool 单元格聚焦、拖拽后行列对话框，以及 Borders、行列增删、清空、适应内容和对齐菜单。旧 Shape 面板中的 `cross-table` 已删除。

**入口**：`Tools palette -> Table Tool -> 拖出外框 -> Insert Table (Rows/Columns)`；编辑入口为 `Table Tool -> 聚焦/右键单元格 -> Borders.../行列命令`。

**规则与证据**：见 [Table Tool 与表格对象规则](table-tool-rules.zh-CN.md)。

### [x] NR-011 StoichiometryGrid

**范围**：`stoichiometrygrid`、`sgcomponent`、`sgdatum` 及列、行、属性类型、值和编辑状态。

**现状**：结构化字段可保留，尚无原生表格布局和单元格编辑。

**目标**：建立网格、组件、数据项和化学对象引用模型，支持原生表格显示与编辑。

**验收**：行列增删、只读/隐藏、反应物/产物引用和多种数据类型均可往返；布局与 ChemDraw 对齐。

**2026-07-26 进展**：

- CCJS 已建立原生 `ReactionSchemeData/ReactionStepData/StoichiometryGridData`，反应引用不再伪装成普通表格或重复写入通用 Link。
- 右键分析、单元格编辑、行列显隐和删除、组件角色、自动计算、冲突状态、跟随/固定锚点、解绑冻结、Alt+双击关系选择和跨标签页复制重映射已接通统一命令链路。
- GUI、SVG、PNG 和 EMF 共用同一组网格、文字和线条绘制原语；CDXML 官方字段可原生导入、编辑和导出。
- 官方 CDX 对象/属性表没有 StoichiometryGrid 标签；保存含表文档为 CDX 会明确拒绝并要求 CCJS/CDXML，不做静默丢失或私有 fallback。
- 详细模型、计算、Link 和生命周期规则见 [反应步骤与化学计量表规则](stoichiometry-grid-rules.zh-CN.md)。

### [x] NR-012 凝胶电泳对象

**范围**：`gepplate`、`geplane`、`gepband`。

**现状**：对象层级和属性可保留，未原生绘制。

**目标**：建立板、泳道、条带的坐标和样式模型。

**验收**：泳道数量、条带位置/宽度/强度、板边界和缩放均有公开夹具与视觉门禁。

**2026-07-26 进展**：

- 左侧 TLC 图标现作为色谱板工具组入口，上方工具栏明确提供 TLC 板和凝胶电泳板两个子工具。
- 新增原生 `payload.gelElectrophoresis`，完整承载板、泳道、条带、范围、单位、标尺、标签、线宽、颜色、透明度和四角坐标；不使用通用 `extra` 或 Link 代替所有权。
- `gepplate/geplane/gepband` 已支持 CDXML 导入、原生绘制、CCJS 保存、CDXML 导出和二次解析；条带拖动修改 `BandValue` 并进入统一命令历史。
- 同步复核 TLC：补齐斑点宽高、独立颜色、透明度、可见性、`ShowRf`、泳道可见性以及板透明度的绘制和往返；板选择恢复缩放手柄。
- 规则见 [色谱板与凝胶电泳对象规则](chromatography-plate-rules.zh-CN.md)。真实浏览器已验证工具切换、TLC/凝胶创建与无警告运行；凝胶公开夹具已通过 SVG 和 EMF 导出，EMF 包含板多边形、条带、标签和标尺原语。

### [x] NR-013 质粒图对象

**范围**：`plasmidmap`、`plasmidregion`、`plasmidmarker`、`marker`。

**现状**：字段可保留，未形成圆环、区域、标记和标签的原生布局。

**目标**：用明确的角度/区间/方向语义生成圆环、箭头区域和标记标签。

**验收**：跨零点区间、正反向、重叠区域、多圈和标签避让均覆盖；编辑后稳定重排。

**2026-07-26 完成**：

- 新增 CCJS 类型化 `payload.plasmidMap`，完整保存碱基总数、圆环/文字样式、regions、markers、独立标签角度和偏移；无效坐标、重复 ID 和非有限数值会明确拒绝。
- 真实 ChemDraw 21 探针确认 Region 端点规范化、起终点箭头独立、标签独立拖动，以及 CDX 对象/属性标签；CDXML 与 CDX 均实现原生导入、导出和二次解析。
- 通用图形导入器不再重复导入质粒内部的派生圆、文本和箭头；GUI、SVG、PNG、EMF 共用质粒渲染规则。
- BioDraw 左侧入口、顶部 Plasmid Map 子工具、创建对话框、右键完整编辑器、取消回滚和合并撤销已接通。
- marker 标签、region 两端和中段控制点分别修改标签几何、碱基位置和径向偏移；多圈由 region offset 统一表达，跨零点外部数据按顺时针 sweep 保留。
- 规则和门禁见 [质粒图对象规则](plasmid-map-rules.zh-CN.md)。

### [ ] NR-014 BioShape/BioDraw 对象

**范围**：DNA、RNA、蛋白、膜、酶、Golgi、抗体及其他 `bioshape` 枚举。

**2026-07-27 完成状态**：

- 已用静默 ChemDraw 探针复核 21 个官方 `BioShapeType`、默认轴几何和类型专属参数；探针入口为 `npm run probe:chemdraw-bioshapes`。当前继续用 136 组轴长、旋转与参数响应样本校准轮廓和重复单元规则；在绝对视觉门禁通过前，本项不得关闭。
- CCJS 新增来源无关的 `payload.bioShape`，显式保存类型、三轴点、填充、线型、颜色、线宽、渐隐和全部类型参数；未知枚举明确报错，不猜成其他类型。
- 21 类 BioShape 已进入共享 Rust render primitive，GUI、SVG、PNG、EMF、Office 与内核工具图标复用同一语义；质粒图继续使用独立的 `PlasmidMapData`。
- CDXML 与 CDX 已覆盖导入、导出和二次解析；全类型 CDXML 往返、代表性 CDX 往返及非默认填充/线型均有回归。
- 编辑器已支持创建、选择、聚焦、移动、缩放、旋转、复制粘贴、跨标签页粘贴、删除、分组和 Link；主绘制栏/生物辅助绘制栏切换不进入文档历史且保留选择。
- 右键 `BioShape Properties...` 由内核按对象类型下发可编辑字段；修改走类型化 `set-bio-shape` 命令并进入撤销历史，不适用字段不会出现在对话框。
- 按 ChemDraw 手册边界实现专属控制柄：Receptor 宽度、GProteinGamma 轮廓、DNA 四参数、HelixProtein 四参数，以及膜单元尺寸/弧线端点；手册未定义专属控制柄的类型只使用统一选择框缩放和旋转。
- 21 类工具图标全部由同一 Rust BioShape 渲染器生成，并按实际图元边界做正方形自适应取景；Web、桌面和 Harmony 不维护另一套图标几何。
- 已建立 21 类 ChemDraw SVG 逐类绝对视觉门禁：双方使用同一文档单位比例，只允许整体平移，不再按内容边界独立缩放；同时检查双向前景距离分位，局部差异不会被对象或画布尺寸稀释。单底物酶、DNA、螺旋蛋白与胶束另有参数化路径拓扑/坐标门禁；当前基线只用于阻止已审查结果回退，不等同于宣称与 ChemDraw 像素完全一致。

**目标**：先复核所有枚举及参数，再按共享几何基元实现，不为单个图标存硬编码样例坐标。

**验收**：每个官方类型至少一个夹具；尺寸、方向、控制点、填充和标签行为可编辑且往返稳定。

### [x] NR-015 TemplateGrid

**范围**：`templategrid` 的行列、窗格尺寸、原点比例、内容和布局。

**现状**：字段可保留，未提供原生网格显示和编辑。

**目标**：建立模板网格布局模型，并明确它与普通 page/group 的关系。

**验收**：非默认行列、尺寸、空单元格和嵌套内容均覆盖；导入导出不改变单元格归属。

**2026-07-27 完成**：

- 新增来源无关 `TemplateGridLayout`，明确建模行、列、窗格高度、单元尺寸和包含空槽的行优先归属；TemplateGrid 不再被误当成画布形状。
- CTP 提取固定使用原生 CDX 类型解码，`PaneHeight` 按 16.16 定点读取；取消 ChemDraw COM CDXML 优先路径和数值启发式。
- CDXML/CDX 导入、布局编辑和写回共用同一校验器；容量不足、重复/遗漏模板、非法尺寸和缺失必填字段均明确失败，未知 TemplateGrid 属性继续保留。
- 模板身份改用稳定 `page@id`；重排、空槽、根级/旧式嵌套页规范化及 CDXML→CDX→CDXML 均不改变单元格归属。
- 模板弹窗按官方列数、单元宽高比和窗格高度显示；右键库按钮可编辑布局，拖拽可交换或移动到空槽，状态实时保存并跨重载保留。
- 全量门禁通过 29 个本机授权模板库、881 个非空模板和 86722 个渲染 primitive；真实 WASM 浏览器回归覆盖 8×6/7×7、空槽、右键对话框、拖拽和持久化。
- 完整规则见 [TemplateGrid 原生模型、编辑与往返规则](template-grid-rules.zh-CN.md)。

### [x] NR-016 文档页眉、页脚和分页布局

**范围**：`Header`、`Footer`、`HeaderPosition`、`FooterPosition`、`PrintTrimMarks`、`PageOverlap`、`WidthPages`、`HeightPages`、`SplitterPositions`、`DrawingSpace`、`Magnification`、`FixInPlaceExtent`、`FixInPlaceGap`。

**现状**：文档级字段可解析或保留，但当前画布、打印和导出没有完整反映。

**目标**：区分屏幕画布、打印页面和嵌入对象三个坐标/分页上下文，按官方定义使用字段。

**验收**：多页、重叠、裁切标记、页眉页脚、缩放和嵌入范围均有规则；普通单页导出不受无关字段污染。

**完成说明**：规则、字段作用域、页面锚点、四向补页、纸张视图、完整参数对话框、CDX/CDXML/CCJS 往返和多页 PDF 门禁见 [`document-layout-rules.zh-CN.md`](document-layout-rules.zh-CN.md)。SVG、EMF 和 Office 预览保持明确的单画布/单对象语义，不用分页 PDF 行为隐式替代。

### [x] NR-017 逻辑对象的原生语义编辑

**范围**：`scheme/step`、`altgroup`、`bracketattachment/crossingbond/represent`、`sequence/crossreference`、`objecttag/annotation`、`regnum`、`splitter`。

**现状**：这些对象多数不直接产生新像素，子对象可能已经可见；关系主要停留在交换层，缺少高层编辑能力。

**目标**：把对象关系、引用完整性和编辑操作纳入原生模型；不强行给纯逻辑对象增加图形。

**验收**：创建、修改、删除和重排时引用保持一致；CDX/CDXML 往返稳定；需要影响布局或显示的字段有对应回归测试。

**2026-07-28 进展**：

- 已建立来源无关的 `reactionSchemes` 与 `logicalObjects` 原生模型，覆盖 Scheme/Step、Alternative Group、Bracketed Group/Attachment/Crossing Bond、Sequence/Cross Reference、ObjectTag、Annotation、Registry Number 和 Representation；Splitter 归入文档布局模型。
- 已增加统一 set/delete/reorder 命令、引用校验、undo/redo、删除清理、完整选择复制粘贴和跨标签 id 重映射。
- Reaction 已接入 Link Auto：普通反应箭头按局部轴生成 typed inferred Step；机制曲箭和无箭头端点的线明确排除；显式 Link/Unlink 覆盖 Auto。
- CDXML/CDX 导入导出由原生模型重建，标准 source id 稳定复用；字段总账已把对应对象和字段提升为 `native-semantic`。
- 浏览器和桌面右键 `Logical Objects...` 提供内核驱动的完整属性管理器。实现与门禁规则见 [`logical-object-native-model.zh-CN.md`](logical-object-native-model.zh-CN.md)。

### [ ] NR-018 旧式复合载荷预览与图片裁剪

**范围**：EMF/WMF/OLE/TIFF/PDF/PICT 内容级预览提取，以及来源无关的图片裁剪矩形。

**现状**：载荷原字节可往返，无法解码时有确定尺寸占位；常规位图已经是完整原生图片对象。旧式复合载荷尚未逐容器提取预览，裁剪尚未进入 CCJS 明确字段。

**目标**：为每种容器建立有签名校验和尺寸上限的明确解码分支；图片裁剪保存为资源坐标中的显式矩形，并由 GUI、SVG、PNG 与 EMF 共用。

**验收**：每种受支持容器至少一个可提交夹具；损坏载荷、超限载荷和无预览载荷均有确定错误/占位规则；裁剪在旋转、缩放、复制粘贴和 CDX/CDXML 往返后不漂移。

## 已有的部分实现

以下能力不是完整工作包，但后续实现应复用并补齐，而不是另建平行逻辑：

- `HDot`、`HDash` 已有绘制。
- `ImplicitHydrogens` 限制已有辅助氢标记。
- 多键型的 `S/A/D/T` 查询标签已有绘制。
- `EnhancedStereoType`、`EnhancedStereoGroupNum` 已能显示 `abs`、`orN`、`&N`。
- 未连接的 `MultiAttachment` 已有三线标记。

## 维护规则

- 开始一个工作包时，把状态改为“进行中”，并在该节记录测量夹具和规则文档链接。
- 只有统一门槛全部满足后才能勾选；“公开样例通过”不等于完成。
- 新发现的缺口先判断是否属于现有工作包；只有存在独立官方语义和独立验收边界时才新增编号。
- 每次提交只关闭能够完整验收的工作包或明确子项，提交说明引用对应 `NR-xxx`。
## Link 模型

通用对象 Link、分析文本、括号重复标签和 atom-symbol 的统一设计与实现要求见
[`link-model.zh-CN.md`](link-model.zh-CN.md)。Link 与 group 完全独立；CDX/CDXML
只承载其已有标准语义，不承载 ChemSema 的通用 `linkPolicy`。
