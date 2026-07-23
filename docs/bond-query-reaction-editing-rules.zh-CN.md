# 键查询、反应属性与显示标记规则

本文记录 `NR-004` 的来源无关模型、ChemDraw 21 静默实测结果和编辑约束。实现不得按文件名、对象 ID、方向样例或缓存 `objecttag` 写特例。

## 来源无关模型

- 原子反应语义放在 `node.atomProperties.reactionChange` 和 `reactionStereo`。
- 键语义统一放在 `bond.properties`：
  - `queryOrders`：`single | aromatic | double | triple` 数组；
  - `topology`：`unspecified | ring | chain | ring-or-chain`；
  - `reactionParticipation`：`unspecified | reaction-center | make-or-break | change-type | make-and-change | not-reaction-center | no-change | unmapped`；
  - `absoluteStereo`：`unspecified | none | e | z`；
  - `showQuery`、`showReaction`、`showStereo`：可缺省的对象级覆盖。缺省表示继承文档设置。
- CDXML 的 `face`、字体表编号、颜色表编号以及 `query`/`stereo` object tag 都不是原生语义。导入后由明确字段和文本样式表达。
- ChemDraw 写出的 object tag 是显示缓存。存在权威原生字段时不得再导入为独立可编辑文本，否则会重复绘制；只有没有对应原生字段的手工 object tag 才按普通附属文本保留。

## CDX/CDXML 映射

| 原生字段 | CDXML | CDX |
| --- | --- | --- |
| `reactionChange` | `RxnChange` | `0x0427`, `CDXBooleanImplied` |
| `reactionStereo` | `RxnStereo` | `0x0428`, `INT8`：0/1/2 |
| `queryOrders` | `Order` 的多个 `S/A/D/T` 值 | `0x0600` 位集合 |
| `topology` | `Topology` | `0x0606`, `INT8`：0..3 |
| `reactionParticipation` | `RxnParticipation` | `0x0607`, `INT8`：0..7 |
| `absoluteStereo` | `BS` | `0x0608`, `INT8`：U/N/E/Z |
| 三个显示覆盖 | `ShowBondQuery/ShowBondRxn/ShowBondStereo` | `0x060c/0x060d/0x060f`, boolean |

未指定的 `BS` 必须保持缺省，不能在普通键上主动写出 `BS="N"`。未知交换层值仍由通用 typed-interchange/raw 机制保留，但不得猜成原生枚举。

## ChemDraw 实测显示规则

可复现实验入口为：

```text
npm run probe:chemdraw-bond-query-reaction
```

探针通过 ChemDraw COM 静默生成 CDXML、CDX、SVG 和 EMF，不依赖鼠标操作。当前已覆盖各枚举、显示继承、字段组合、水平/垂直/斜向和端点反转。

### 文本内容与顺序

- 查询键级：`1 2 -> S/D`，`1 1.5 -> S/A`，`2 1.5 -> D/A`。ChemDraw 21 的界面会移除其他任意组合；内核仍按官方位集合无损读取和写出。
- 拓扑：`Ring -> Rng`，`Chain -> Chn`，`RingOrChain -> R/C`。
- `ReactionCenter`、`MakeOrBreak`、`ChangeType`、`MakeAndChange` 显示 `Rxn`；其余反应参与枚举保存但不显示。
- `E/Z` 显示为斜体 `(E)` / `(Z)`；`U/N` 不显示。
- 同一键的查询文本顺序固定为 `Topology + Rxn + Order`，例如 `RngRxnS/D`。
- 原子查询组合顺序固定为 `X/U/* -> S -> R -> C -> T -> L -> I`；其中 `C` 是反应变化，`T` 是反应构型。

### 显示继承

- `showQuery`、`showReaction`、`showStereo` 的对象值优先于文档值。
- ChemDraw 默认值：查询显示、反应显示、立体不显示。
- 多键级查询的 `S/D`、`S/A`、`D/A` 是键型本身唯一的可见表达，因此即使查询显示设为隐藏也必须显示；隐藏只影响拓扑文本。

### 方向和位置

- 标注字号为标签字号的 `0.75`，字体继承标签字体。
- 先把键轴规范化为与端点顺序无关的方向：优先令 `dx > 0`；垂直键令 `dy > 0`。法向量为 `(-dy, dx)`。
- 只有一组标注时放在负法向侧；E/Z 与查询同时存在时，E/Z 在负法向侧，查询在正法向侧。
- 不做 360 度搜索。位置采用按轴分离的确定函数：
  - `center.x = midpoint.x + side * normal.x * (textWidth / 2 + 0.29em)`
  - `center.y = midpoint.y + side * normal.y * (textHeight / 2 + verticalGap)`
  - 负法向 `verticalGap = 0.11em`，正法向 `verticalGap = 0.29em`
  - 实测文本高度为 `1.061333em`
- SVG、PNG、EMF 和 GUI 都只消费同一组 `RenderPrimitive::Text`，不得在某个导出器中另写偏移。

## 编辑行为

- 原子右键菜单的 `Atom Query` 提供反应变化和反应构型。
- 键右键菜单的 `Bond Query & Reaction` 提供键级查询、拓扑、反应参与、E/Z 和三个对象级显示覆盖。
- 所有修改走命令系统，是一个可撤销步骤，并立即更新 CCJS、画布和后续 CDX/CDXML 导出。
- 用户选择普通单键、双键或三键样式时，明确清除 `queryOrders`；拓扑、反应参与、E/Z 和显示覆盖是独立语义，不随普通线型切换丢失。
- 复制粘贴、跨标签页和跨 Web/桌面传递使用 CCJS 原生字段，不依赖 ChemDraw 缓存文本。

## 回归门槛

- CCJS、CDXML、CDX 三向往返必须保持所有枚举和对象级显示覆盖。
- 测试必须覆盖默认值、非默认值、继承、显式显示/隐藏、查询与 E/Z 组合、端点反转及至少水平/垂直/斜向。
- 渲染测试以文本内容、样式和相对键中点/法向的位置为准；视觉门禁再检查 SVG/EMF 的局部细节，不得用整张画布尺寸稀释误差。

