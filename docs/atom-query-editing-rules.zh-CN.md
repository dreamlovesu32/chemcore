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
6. 组合顺序固定为 `X/U/* → S → R → L → I`，其中 `I` 表示非 `Unspecified` 的同位素丰度。`SubstituentsExactly` 优先于 `SubstituentsUpTo`，二者又优先于 `FreeSites`；被压过的字段仍需往返保留。
7. 普通查询字符字号为原子标签字号的 `0.75`；`*` 使用 Symbol 字体，字号为普通查询字号再加 `0.8 pt`。查询位于主连接方向的反侧：右键在左、左键在右、上键在下、下键在上。
8. 节点级 `ShowAtomQuery` 覆盖文档默认值；`no` 仅隐藏短码，不删除查询字段。
9. `ElementList` 与 `GenericList` 的载体均为 `NodeType="ElementList"`；两者可以共存，显示顺序为元素符号在前、通用名在后，例如 `N, O, R, X`。排除列表以 `NOT ` 开头。
10. `AbnormalValence` 不产生短码；它关闭常规价态无效诊断，并使未明确给出的隐式氢不再由典型价态猜测。
11. ChemDraw 静默导入/导出会保留碳标签显示字段，但不会仅凭字段自动写入 `<t>` 缓存。ChemSema 在编辑器内按字段即时物化 `C/CHn` 标签，同时导出原字段；用户自写标签不被覆盖。
12. `ImplicitHydrogens` 限制的 `H` 是独立的右上角标记，不并入上述查询短码，也不替代标签自身的氢。

## 碳标签优先级

节点级 `showTerminalCarbonLabel` / `showNonTerminalCarbonLabel` 覆盖文档级默认值。端点碳按连接键数量 `0` 或 `1` 判定，非端点碳按连接键数量大于 `1` 判定；这里统计键数量，不统计键级。

显示文本中的氢数按实际键级总和、形式电荷和显式氢覆盖计算。字段关闭时只移除由该规则生成且未被用户编辑的标签，绝不删除用户 authored 标签。

## 编辑入口与往返

原子右键菜单的 “Atom Query” 提供列表、计数、环键、饱和性、翻译宽窄、异常价态和碳标签显示入口。所有修改使用统一的 `set-atom-property-for-selection` 可撤销命令，并进入 CCJS、CDXML 与 CDX 的同一往返链路。

公开最小夹具位于 `crates/chemsema-engine/tests/fixtures/cdxml/atom-query-properties.cdxml`。
