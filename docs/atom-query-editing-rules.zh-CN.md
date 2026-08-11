# 原子查询与碳标签编辑规则

本文记录 `NR-003` 的来源无关模型、ChemDraw 实测行为和编辑约束。实现不得根据文件名、对象 ID 或单个夹具分支。

## CCJS 原生字段

以下字段位于 `node.atomProperties`，均为明确语义，不使用 CDXML `face`、原始字节或 `meta.import` 作为编辑入口：

- `elementList: number[]` 与 `elementListExcluded: boolean`
- `genericList: string[]` 与 `genericListExcluded: boolean`
- `freeSites: number | null`
- `showAtomQuery: boolean | null`
- `ringBondCount: unspecified | no-ring-bonds | as-drawn | simple-ring | fusion | spiro-or-higher`
- `unsaturatedBonds: unspecified | must-be-absent | must-be-present`
- `substituentsUpTo: number | null`
- `substituentsExactly: number | null`
- `translation: equal | broad | narrow | any`
- `abnormalValence: boolean`
- `showTerminalCarbonLabel: boolean | null`
- `showNonTerminalCarbonLabel: boolean | null`

`null` 表示继承文档默认值或未设置限制；它与显式 `false` 不等价。

## ChemDraw 后台探针结论

可复现探针入口为 `npm run probe:chemdraw-atom-queries`，探针使用 ChemDraw COM 静默打开、保存 CDXML 与 SVG，不依赖屏幕自动化。

1. `FreeSites=0/1/2` 分别显示 `*0`、`*`、`*2`。
2. 任一非 `Unspecified` 的 `RingBondCount` 显示 `R`。
3. `MustBeAbsent` 与 `MustBePresent` 都显示 `S`；具体语义保留在字段中，短码本身不区分。
4. `SubstituentsUpTo=n` 显示 `Un`；`SubstituentsExactly=n` 显示 `Xn`。
5. `Translation=Equal` 不显示；`Broad/Narrow/Any` 都显示 `L`。
6. 组合顺序固定为 `X/U/* → H → S → R → L → I`，其中 `H` 表示 `ImplicitHydrogens` 限制，`I` 表示非 `Unspecified` 的同位素丰度。`SubstituentsExactly` 优先于 `SubstituentsUpTo`，二者又优先于 `FreeSites`；被压过的字段仍需往返保留。
7. 普通查询字符字号为原子标签字号的 `0.75`；`*` 使用 Symbol 字体，字号为普通查询字号再加 `0.8 pt`。位置不是上下左右四档，而是沿连接占用方向的反向连续变化：标注中心相对标签字形盒的间距按各轴投影计算，边缘净距固定为 `MarginWidth + LineWidth / 2`。
8. 节点级 `ShowAtomQuery` 覆盖文档默认值；`no` 仅隐藏短码，不删除查询字段。
9. `ElementList` 与 `GenericList` 的载体均为 `NodeType="ElementList"`；两者可以共存，显示顺序为元素符号在前、通用名在后，例如 `N, O, R, X`。排除列表以 `NOT ` 开头。
10. `ElementList` 与 `GenericList` 的普通基线文本按所选字体的字符推进格进行整体布局和挂接，不以首字符的可见墨迹包围盒代替字符推进格。静默 ChemDraw 探针在 Arial 8/10/14 pt、14.4/30 pt 键长以及 `[C]`、`[C,N,P]` 上得到同一规则：上方双连接布局把首个 `[` 的推进格中心放在原子横坐标上；Arial 10 pt 的推进宽度为 2.778 pt，因此文本基线起点为 `x(atom)-1.389 pt`。带脚本的查询文本继续走逐字形化学布局，不能套用普通基线文本的整体推进规则。
10. `AbnormalValence` 不产生短码；它关闭常规价态无效诊断，并使未明确给出的隐式氢不再由典型价态猜测。
11. ChemDraw 静默导入/导出会保留碳标签显示字段，但不会仅凭字段自动写入 `<t>` 缓存。ChemSema 在编辑器内按字段即时物化 `C/CHn` 标签，同时导出原字段；用户自写标签不被覆盖。
12. `ImplicitHydrogens` 限制的 `H` 并入同一个查询标注对象；它不替代标签自身的氢。`FreeSites` 同时占用相应价态，因此会减少标签中自动推导的隐式氢数；例如单键相连的氮在 `FreeSites=0/1/2` 时分别显示 `NH2/NH/N`。

## 碳标签优先级

除显式的 `ShowTerminalCarbonLabels` / `ShowNonTerminalCarbonLabels` 外，ChemDraw 还会为不能由普通隐式碳顶点完整表达的碳自动物化标签。静默探针覆盖孤立碳、端点单/双键、两个单键、单双键、两个双键以及三/四个单键，并得到统一规则：

1. `NumHydrogens` 缺失时，普通中性碳仍按键线顶点显示；`NumHydrogens` 明确存在时，仅当“键级总和 + 显式氢数”不等于 4 才物化 `C`、`CH` 或 `CHn`。
2. 直接写在原子节点上的电荷、同位素或自由基要求显示碳元素符号，因此无论 `NumHydrogens` 是否存在都物化标签。若电荷或自由基由独立 `graphic/represent` 对象显示，该图形拥有显示权威，不能据此物化骨架碳；同一原子因同位素或显式价态异常仍需物化时，标签也不得重复写入已由图形代表的电荷或自由基。缺失的氢数按 `4 - 共价键级总和 - |charge| - radicalElectronCount` 推导并截断到非负值；显式氢数始终优先。非金属到金属的配位以及供体端 dative 键不计入这个共价键级总和。
3. `AbnormalValence="yes"` 只关闭常规价态诊断，不单独强制显示碳；当显式氢数与键价仍正好补足 4 时标签保持隐藏。
4. 同位素使用左上标 run，电荷保留在化学 formula run，自由基使用右上标 run；它们共同参与标签方向翻转和键端退让。自动物化标签属于显示缓存，未被用户编辑时不写回源 CDXML 的 `<t>`，原有语义字段无损往返。

复现实验入口为 `npm run probe:chemdraw-carbon-valence-labels`，完整矩阵写入 `tmp/chemdraw-carbon-valence-label-probe/summary.json`。

节点级 `showTerminalCarbonLabel` / `showNonTerminalCarbonLabel` 覆盖文档级默认值。端点碳按连接键数量 `0` 或 `1` 判定，非端点碳按连接键数量大于 `1` 判定；这里统计键数量，不统计键级。

显示文本中的氢数按实际键级总和、形式电荷和显式氢覆盖计算。字段关闭时只移除由该规则生成且未被用户编辑的标签，绝不删除用户 authored 标签。

## 编辑入口与往返

原子右键菜单的 “Atom Query” 提供列表、计数、环键、饱和性、翻译宽窄、异常价态和碳标签显示入口。所有修改使用统一的 `set-atom-property-for-selection` 可撤销命令，并进入 CCJS、CDXML 与 CDX 的同一往返链路。

公开最小夹具位于 `crates/chemsema-engine/tests/fixtures/cdxml/atom-query-properties.cdxml`。
