# Chemcore 缩写识别规则

本文档定义 Chemcore 当前内核对结构端点标签、functional group 缩写和
formula-like 标签的识别行为。普通自由文本不属于这里的结构标签规则。

识别入口必须带连接数上下文。同一个字符串在不同连接数下可能有不同语义，
也可能从合法变为非法。

## 连接数分流

| 外部连接数 | 行为 |
| ---: | --- |
| `0` | 只做 chemical text 校验。可由价键 tokenizer 读懂的化学文本会标记为 `groupKind: "chemical-text"`，不生成 `expansion`。 |
| `1` | 末端取代基。先走价键驱动 parser，再走命名 functional group 模板。 |
| `2` | 桥接标签。只接受开放桥接片段，或 `N` 加一价末端取代基。 |
| `>=3` | 当前不识别为 functional group。 |

识别成功时，节点和节点 label 都写入 `meta.labelRecognition`。识别失败时，
化学结构标签保留原始文本，并写入 invalid metadata，渲染层用红色诊断框提示。

## 元数据结构

识别成功的 metadata 使用同一层结构：

```json
{
  "kind": "functional-group",
  "status": "recognized",
  "label": "CO2Et",
  "canonicalLabel": "CO2Et",
  "groupKind": "valence-fragment",
  "source": "valence-parser",
  "formula": "-C(=O)OEt",
  "anchorAtom": "C",
  "components": [],
  "expansion": {}
}
```

字段规则：

- `label` 保留用户输入。
- `canonicalLabel` 保存归一化标签，例如 `COOH -> CO2H`、`OCH3 -> OMe`、
  `Tos -> Ts`、`FMOC -> Fmoc`、`t-Bu -> tBu`。
- `groupKind` 当前取值为 `terminal-fragment`、`valence-fragment`、
  `bridge-fragment` 或 `chemical-text`。
- `source: "valence-parser"` 只在 `valence-fragment` 上写入。
- `chemical-text` 不生成 `expansion`。
- `expansion.schema` 固定为 `chemcore.functionalGroupExpansion.v1`。
- `expansion.connectionKind` 当前为 `terminal` 或 `bridge`。
- `expansion.atoms[].id` 是 expansion 内部局部 id，不污染主分子图节点 id。
- `expansion.atoms[]` 可携带 `numHydrogens`、`label` 和 `formalCharge`。
- `expansion.attachments` 用 `external` 表示末端外部连接点，用 `left` /
  `right` 表示桥接连接点。
- `complete: false` 表示标签已合法识别，但 expansion 中有不完整或占位拓扑。

invalid metadata 使用：

```json
{
  "kind": "functional-label",
  "status": "invalid",
  "label": "NotAGroup"
}
```

## 命名末端模板

以下命名模板在 1 根外部键时可作为末端取代基识别。外部键连接到
`anchorAtom`。其中部分模板有完整拓扑展开；尚未完整展开的复杂模板仍保留
合法识别 metadata，并在 expansion 中标记 `complete: false`。

| canonical | aliases | 名称 | formula / structure | anchorAtom |
| --- | --- | --- | --- | --- |
| `R` | `R'`, `R''` | generic substituent | `R` | `R` |
| `Ar` | - | generic aromatic substituent | `Ar` | `Ar` |
| `Me` | `CH3` | methyl | `-CH3` | `C` |
| `Et` | `C2H5` | ethyl | `-CH2CH3` | `C` |
| `Pr` | - | propyl | `-CH2CH2CH3` | `C` |
| `nPr` | `n-Pr` | n-propyl | `-CH2CH2CH3` | `C` |
| `iPr` | `i-Pr` | isopropyl | `-CH(CH3)2` | `C` |
| `Bu` | - | butyl | `-CH2CH2CH2CH3` | `C` |
| `nBu` | `n-Bu` | n-butyl | `-CH2CH2CH2CH3` | `C` |
| `iBu` | `i-Bu` | isobutyl | `-CH2CH(CH3)2` | `C` |
| `sBu` | `s-Bu` | sec-butyl | `-CH(CH3)CH2CH3` | `C` |
| `tBu` | `t-Bu` | tert-butyl | `-C(CH3)3` | `C` |
| `Ph` | - | phenyl | `-C6H5` | `C` |
| `PhCOOH` | - | benzoic acid substituent | `PhCOOH` | `C` |
| `Bn` | - | benzyl | `-CH2Ph` | `C` |
| `Bz` | - | benzoyl | `-C(=O)Ph` | `C` |
| `Ac` | - | acetyl | `-C(=O)CH3` | `C` |
| `Piv` | - | pivaloyl | `-C(=O)tBu` | `C` |
| `CHO` | - | formyl | `-C(=O)H` | `C` |
| `CN` | - | cyano | `-C#N` | `C` |
| `NCO` | - | isocyanato | `-N=C=O` | `N` |
| `NCS` | - | isothiocyanato | `-N=C=S` | `N` |
| `SCN` | - | thiocyanato | `-S-C#N` | `S` |
| `NO2` | - | nitro | `-N(=O)O` | `N` |
| `N3` | - | azido | `-N3` | `N` |
| `H` | - | hydrogen terminator | `-H` | `H` |
| `F` | - | fluoro | `-F` | `F` |
| `Cl` | - | chloro | `-Cl` | `Cl` |
| `Br` | - | bromo | `-Br` | `Br` |
| `I` | - | iodo | `-I` | `I` |
| `OH` | - | hydroxy | `-OH` | `O` |
| `NH2` | - | amino | `-NH2` | `N` |
| `Ts` | `Tos` | tosyl | `-S(=O)2-p-Tol` | `S` |
| `Bs` | - | brosyl | `-S(=O)2-p-BrPh` | `S` |
| `Ms` | - | mesyl | `-S(=O)2CH3` | `S` |
| `Tf` | - | triflyl | `-S(=O)2CF3` | `S` |
| `SO3H` | - | sulfonic acid | `-S(=O)2OH` | `S` |
| `SO2H` | - | sulfinic acid style label | `-S(=O)OH` | `S` |
| `SO3` | - | sulfonate fragment | `-S(=O)3-` | `S` |
| `SO4` | - | sulfate fragment | `SO4` | `S` |
| `SO4H` | - | sulfate monoacid | `SO4H` | `O` |
| `PO2` | - | phosphoryl fragment | `PO2` | `P` |
| `PO3` | - | phosphate fragment | `PO3` | `P` |
| `PO3H2` | - | phosphonic acid | `-P(=O)(OH)2` | `P` |
| `PO4` | - | phosphate | `PO4` | `P` |
| `PO4H2` | - | phosphate acid form | `PO4H2` | `O` |
| `Boc` | - | tert-butyloxycarbonyl | `-C(=O)O-tBu` | `C` |
| `Cbz` | - | benzyloxycarbonyl | `-C(=O)OCH2Ph` | `C` |
| `Fmoc` | `FMOC` | fluorenylmethoxycarbonyl | `-C(=O)OCH2-fluorenyl` | `C` |
| `TMS` | - | trimethylsilyl | `-Si(CH3)3` | `Si` |
| `TBDMS` | - | tert-butyldimethylsilyl | `-Si(CH3)2tBu` | `Si` |
| `TBDPS` | - | tert-butyldiphenylsilyl | `-Si(Ph)2tBu` | `Si` |
| `CCl3` | - | trichloromethyl | `-CCl3` | `C` |
| `CF3` | - | trifluoromethyl | `-CF3` | `C` |
| `CPh3` | - | trityl | `-CPh3` | `C` |
| `Cp` | - | cyclopentadienyl | `Cp` | `C` |
| `Cy` | - | cyclohexyl | `-C6H11` | `C` |
| `Mes` | - | mesityl | `2,4,6-trimethylphenyl` | `C` |
| `NHPh` | - | anilino | `-NHPh` | `N` |
| `Indole` | - | indolyl / indole template | `Indole` | `C` |
| `ster` | - | generic steric label | `ster` | `C` |

## 价键驱动 formula-like 标签

1 根外部键时，内核先尝试价键驱动 parser。parser 将标签 token 化为元素、
数量、括号组和一价命名模板，再从左到右分配键级，生成
`groupKind: "valence-fragment"`。

典型结果：

```text
CH3                  -> -CH3
CN                   -> -C#N
CF3                  -> -CF3
COCl                 -> -C(=O)Cl
COBr                 -> -C(=O)Br
CONH2                -> -C(=O)NH2
COOH                 -> canonical CO2H, formula -C(=O)OH
CO2Et                -> -C(=O)OEt
CO2Boc               -> -C(=O)OBoc
COOSO2Me             -> -C(=O)OS(=O)2Me
CH2COOCH2SO2NHCl     -> -CH2C(=O)OCH2S(=O)2NHCl
B(OH)2               -> boronic-acid style terminal fragment
```

命名模板也能作为价键 parser 的一价终止 token 使用。例如 `CH2Boc` 中，
`Boc` 消耗前一原子的一个连接位，内部拓扑仍使用 `Boc` 模板展开。

当前 tokenizer 支持：

- 标准大小写元素符号，例如 `Cl`、`Si`、`Na`。
- 元素后的数字重复，例如 `H3`、`O2`。
- 括号组及组后的重复数，重复数必须为 `1..=32`。
- 一价命名模板作为终止 token。

当前价键例外：

- 碱金属按 1 价，碱土金属按 2 价。
- 过渡金属和若干金属标签作为 unconstrained valence 处理，主要用于
  chemical text 校验。
- `B` 在 `BH3` 这类右侧氢补足场景可记录 `formalCharge: -1`。
- `N` 在 `NH3` 这类右侧氢补足场景可记录 `formalCharge: +1`。
- `O` 在 `OH2` / `OH3` 这类右侧氢补足场景可记录 `formalCharge: +1` /
  `+2`。
- `S` 根据局部书写约定优先识别 `SO2` 为两个 `S=O`，再考虑其他可行价态。

以下模式当前不放宽：

```text
BCl3
NMe4
OCl3
OCl4
```

## 桥接标签

2 根外部键时，以下开放片段可单独作为桥接标签：

| label | aliases | formula | left / right attachment |
| --- | --- | --- | --- |
| `CO2` | `COO` | `-C(=O)O-` | `C` / `O` |
| `OCO` | - | `-O-C(=O)-` | `O` / `C` |
| `SO2` | - | `-S(=O)2-` | `S` / `S` |
| `SO` | - | `-S(=O)-` | `S` / `S` |
| `CH2` | - | `-CH2-` | `C` / `C` |
| `NH` | - | `-NH-` | `N` / `N` |
| `CO` | - | `-C(=O)-` | `C` / `C` |
| `O` | - | `-O-` | `O` / `O` |

此外，`N` 加一价末端取代基可作为取代氮桥：

```text
NMe  -> -N(Me)-
NTs  -> -N(Ts)-
NTos -> canonical NTs
NCl  -> -N(Cl)-
```

两键上下文不接受普通末端模板，例如 `Boc`、`CN`、`NO2`、`CO2Et`。

## 标签显示与反转

结构标签的显示不是简单逐字符反转。内核会先把显示文本切成化学上有意义的
组，再按连接方向决定组顺序。

分组规则：

- 含小写字母的命名缩写作为一组，例如 `Ph`、`Boc`、`iPr`、`tBu`。
- `R`、`TMS`、`TBDMS`、`TBDPS` 作为一整个字母组。
- 分组后的数字后缀保留在对应组内。
- `TMS` 的连接点是 `Si`，只允许一个外部连接点。

因此右侧连接时：

```text
OTMS -> TMSO
```

而不是逐字符变成 `SMTO`。

`iPr`、`nBu`、`tBu` 这类以小写字母开头且后面包含大写字母的末端模板，
使用 whole-label layout：选择和锚点把整个标签视作一个不可拆的结构标签。

## 与元素隐式氢的关系

缩写识别发生在简单元素隐式氢之前。命中 functional group 后，不再按元素串
逐字符加氢。

示例：

- `NO2` 是 nitro group，不是 `N` 加两个普通 `O` 字符。
- `CN` 是 cyano group，不是自由文本。
- `TMS` 是一价 trimethylsilyl group，连接点是 `Si`。
- `CO2Et`、`COOSO2Me`、`CH2CH2CH3` 由价键 parser 解释。

普通元素标签和自动加氢规则见 `docs/implicit-hydrogen-rules.zh-CN.md`。
更完整的价键 parser 规则见 `docs/valence-label-recognition-rules.zh-CN.md`。
