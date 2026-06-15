# Chemcore 价键驱动标签识别规则

本文档记录 Chemcore 当前价键驱动标签识别规则。它不替代 `Boc`、`Fmoc`、`Ts`
这类命名取代基模板；它负责解释 `CN`、`CO2Cl`、`CH2COOCH2SO2NHCl`
这类可以由元素价键和线性书写顺序推出的 formula-like label。

## 行为目标

价键 parser 会把 `CN`、`CF3`、`CONH2`、`CO2Et` 等 formula-like
标签拆成通用规则处理：

- 读取元素和数量，例如 `CH3` 读作 `C + H + H + H`，
  `CO2Cl` 读作 `C + O + O + Cl`。
- 根据节点外部连接数先消耗 attachment 原子的价键。
- 从左到右读取后续原子；当前可连接原子尽量把价键用满，但不能超过允许价态。
- 由价键剩余量自动决定单键、双键或三键，例如 `CN` 推出 `C#N`，
  `CO2Cl` 推出 `C(=O)OCl`。
- 解析成功后生成 `functionalGroupExpansion.v1`，并把 `meta.labelRecognition`
  标记为 `status: recognized`、`source: valence-parser`。

## 连接数上下文

识别不能只看字符串，必须知道标签节点已有多少根外部键。

### 末端标签

末端 functional group 要求恰好 1 根外部键。解析时，外部键先连接到
第一个可连接原子，并消耗该原子 1 个价键单位。

示例：

```text
-CH3
```

解析过程：

```text
C 允许 4 价；左侧外部键消耗 1，还剩 3。
H、H、H 各消耗 1。
C 正好满价，识别为 methyl。
```

### 桥接标签

两键桥接标签要求恰好 2 根外部键。当前两键桥接由缩写识别入口处理，
而不是由末端价键 parser 直接生成任意桥接拓扑。如果无法同时满足左右
attachment 的价键，标签应标记为 invalid。

这条规则会保留现有桥接缩写行为，例如 `NH`、`CO`、`CO2/COO`、`SO2`
在两键节点上仍合法。

### 其他连接数

0 根外部键会进入 chemical text 校验：可由价键 tokenizer 读懂的文本会标记为
`groupKind: "chemical-text"`，但不生成 `functionalGroupExpansion.v1`。
3 根或更多外部键默认不进入末端 functional group 解析，除非有明确模板支持，
否则应标记为普通未知标签或 invalid。

## 元素价态表

价键 parser 里的“价态”先表示可连接的键单位数，不先区分正负性。也就是说，
碱金属一价、碱土金属二价，都是连接容量；只有少数例外价态需要在 expansion
里记录形式电荷。

### 常规价态

| 元素 | 允许价态 | 说明 |
| --- | --- | --- |
| `H` | 1 | 只作为终止原子。 |
| `Li/Na/K/Rb/Cs/Fr` | 1 | 碱金属统一按 1 价连接处理。 |
| `Be/Mg/Ca/Sr/Ba/Ra` | 2 | 碱土金属统一按 2 价连接处理。 |
| `B` | 3 | 常规硼按 3 价。4 价硼是带负电荷的例外，见下节。 |
| `C` | 4 | formula-like label 的核心骨架原子。 |
| `N` | 3, 5 | 普通氮优先按 3 价；5 价只在明确局部模式中启用。4 价氮是带正电荷的例外，见下节。 |
| `O` | 2 | 可作为羰基氧、醚氧、羟基氧或继续连接氧。3/4 价氧是带正电荷的例外，见下节。 |
| `Si` | 4 | 硅按 4 价连接处理。 |
| `P` | 3, 5 | 磷按 3/5 价处理。 |
| `As` | 3, 5 | 砷按 3/5 价处理。 |
| `S` | 2, 4, 6 | `SO2` 明确按 6 价硫处理；`SOO` 按 4 价硫处理。 |
| `F/Cl/Br/I` | 1, 3, 5, 7 | 在普通有机取代基里优先选择 1 价；高价只在满足上下文且明确需要时使用。 |

选择价态时使用“最小可满足价态”作为默认原则；但对 `S` 这类存在书写约定
的元素，需要先看局部模式。比如 `SO2` 直接选择 6 价硫，而不是先按 4 价
解释后失败。

### 形式电荷例外

以下例外可以参与拓扑解析，但必须在 `expansion.atoms[]` 或后续等价字段里
记录形式电荷，不能只当作普通中性价态。

| 元素 | 例外价态 | 形式电荷 | 限制 |
| --- | --- | --- | --- |
| `B` | 4 | `-1` | 只接受右侧至少 3 个氢参与满足价键的场景。 |
| `N` | 4 | `+1` | 当前只接受右侧补足原子全是氢的场景。 |
| `O` | 3 | `+1` | 当前只接受右侧补足原子全是氢的场景。 |
| `O` | 4 | `+2` | 当前只接受右侧补足原子全是氢的场景。 |

这里的“右侧”指标签书写中位于该原子之后、用于补足该原子价键的 atom
occurrence。末端标签左侧外部键已经消耗 1 个连接位，所以 `BH3` 可以识别为
四配位硼并记录 `formalCharge: -1`；但 `BCl3` 不接受，因为它不满足
“右侧至少三个氢”的限制。类似地，`NH3` 可以作为四价带正电氮的候选；
`NMe4` 不进入这个例外。

## Token 化规则

标签先转为 atom occurrence 流：

- 元素符号按标准大小写识别，例如 `Cl` 是一个原子，不是 `C + l`。
- 紧跟元素后的数字表示该元素重复次数。
- `H3` 展开成 3 个氢，`O2` 展开成 2 个氧。
- `CO2Cl` 展开成 `C, O, O, Cl`。
- 括号组按子 token 流解析，组后数字表示重复次数；空组、0 次重复和超过
  32 次重复都视为 invalid。
- `CH2COOCH2SO2NHCl` 展开成：

```text
C, H, H, C, O, O, C, H, H, S, O, O, N, H, Cl
```

当前不解析点号、显式电荷、同位素、芳香小写、环编号或任意 SMILES。
这些输入应交给专门模板或 invalid fallback。

## 核心解析原则

### 当前原子尽量满价

从左到右读取时，已经打开的当前骨架原子会优先吸收右侧原子，直到它的
价键被满足或下一个连接会超价。

示例 `-CO2Cl`：

```text
C 左侧外部键消耗 1，还剩 3。
第一个 O 可 2 价，C 与 O 尽量形成 C=O，C 还剩 1。
第二个 O 只能与 C 单键，C 满价，O 还剩 1。
Cl 选择 1 价，与第二个 O 单键。
```

结果：

```text
-C(=O)OCl
```

### 第一个可成多键的异原子优先

当碳右侧遇到 `O`、`S`、`N` 等可成多键原子时，优先让当前碳与这个
第一个异原子形成可行的最高键级；剩余价键再给后续原子。

这保证：

```text
-CN    -> -C#N
-COCl  -> -C(=O)Cl
-CSO-  -> -C(=S)O-
-COS-  -> -C(=O)S-
```

也就是说，`-CSO-` 不应先解释成 `-C-S(=O)-`；`C` 会先尽量和第一个
`S` 形成双键。`-COS-` 同理先得到碳氧双键。

### 后续原子接到最近可满足的 attachment

当当前骨架原子已满价，后续原子应接到最近一个仍有剩余价键、且书写上
可作为右侧 attachment 的原子。

例如 `-CO2Cl` 中，第二个 `O` 与 `C` 单键后还剩 1 个价键，所以后续
`Cl` 接到这个 `O`，而不是再尝试接到已经满价的 `C`。

### 特殊书写约定

`S` 的氧化态不能只靠贪心多键决定，需要承认常见书写约定：

```text
SO2  -> S(VI)，两个 S=O
SOO  -> S(IV)，一个 S=O，一个 S-O
```

因此：

```text
-SO2NHCl
```

解析为：

```text
S 左侧单键消耗 1。
SO2 约定选择 6 价硫，两个 O 各形成 S=O，消耗 4。
S 还剩 1，接 N。
N 选择 3 价，接 H 和 Cl 后满价。
```

结果：

```text
-S(=O)2NHCl
```

## 示例推导

### `-CH3`

```text
C: 外部键 1 + H + H + H = 4
```

结果：`-CH3`。

### `-CO2Cl`

```text
C: 外部键 1 + O(double) + O(single) = 4
O(single): C + Cl = 2
Cl: O = 1
```

结果：`-C(=O)OCl`。

### `-CH2COOCH2SO2NHCl`

Token 流：

```text
C H H C O O C H H S O O N H Cl
```

推导：

```text
C1: 外部键 1 + H + H + C2 = 4
C2: C1 + O1(double) + O2(single) = 4
O2: C2 + C3 = 2
C3: O2 + H + H + S = 4
S: C3 + O3(double) + O4(double) + N = 6
N: S + H + Cl = 3
Cl: N = 1
```

结果：

```text
-CH2-C(=O)-O-CH2-S(=O)2-NHCl
```

这个例子是价键解析器的核心回归用例：它同时覆盖碳满价、羰基、酯氧继续
连接、亚甲基、硫酰、氮桥和卤素终止。

## 与现有缩写表的关系

价键解析器是末端 formula-like 标签的主路径。旧的缩写/保护基定义不再
作为另一套组合 parser 抢优先级，而是作为价键 parser 的一价终止 token：
`Me`、`Et`、`Boc`、`Fmoc`、`Ts`、`TBDMS` 这类命名基团在价键满足时
等价于一个只需要 1 个连接位的终止原子。它们的内部展开仍使用原来人工确认的
模板。

当前优先级：

1. 简单元素标签和隐式氢规则，例如 `N`、`O`、`Cl`。
2. 价键驱动 formula-like parser，例如 `CN`、`CF3`、`CO2Cl`、
   `CH2COOCH2SO2NHCl`。
3. 单独输入的命名 functional group 模板，例如 `Boc`、`Fmoc`、`Ts`、
   `TBDMS`。它们不拆成普通元素串。
4. invalid fallback。

这意味着：

```text
Boc       -> 命名模板，单独作为一价取代基
CO2Boc    -> C + O + O + Boc，其中 Boc 消耗第二个 O 的 1 个连接位
CH2Boc    -> C + H + H + Boc，其中 Boc 消耗 C 的 1 个连接位
```

`CN`、`CF3`、`COCl`、`CONH2`、`CO2Et` 这类可以由价键规则推出的标签，
最终不应再依赖单独 hard-code。

## 元数据

价键 parser 成功时，`meta.labelRecognition` 仍使用现有结构，并额外保留
来源信息，方便调试和迁移：

```json
{
  "kind": "functional-group",
  "status": "recognized",
  "source": "valence-parser",
  "label": "CO2Cl",
  "canonicalLabel": "CO2Cl",
  "groupKind": "valence-fragment",
  "anchorAtom": "C",
  "formula": "-C(=O)OCl",
  "components": [
    { "label": "C", "kind": "atom" },
    { "label": "O", "kind": "atom", "bondOrderToParent": 2 },
    { "label": "O", "kind": "atom", "bondOrderToParent": 1 },
    { "label": "Cl", "kind": "atom", "bondOrderToParent": 1 }
  ],
  "expansion": {
    "schema": "chemcore.functionalGroupExpansion.v1",
    "connectionKind": "terminal",
    "complete": true,
    "attachments": [
      { "role": "external", "atomId": "c1" }
    ]
  }
}
```

`components` 主要用于测试、调试和导出校验，
尤其是检查 `CO2`、`SO2`、`CSO`、`COS` 这类容易产生歧义的局部决策。

## 回归用例

以下用例应由 Rust 单测覆盖：

```text
CH3                  -> -CH3
CN                   -> -C#N
CF3                  -> -CF3
COCl                 -> -C(=O)Cl
COBr                 -> -C(=O)Br
CONH2                -> -C(=O)NH2
CO2Cl                -> -C(=O)OCl
COOH                 -> -C(=O)OH
CO2Et                -> -C(=O)OCH2CH3
CH2COOCH2SO2NHCl     -> -CH2-C(=O)-O-CH2-S(=O)2-NHCl
CSO                  -> -C(=S)O-
COS                  -> -C(=O)S-
SO2NHCl              -> -S(=O)2NHCl
SOONHCl              -> -S(=O)ONHCl
Na                   -> -Na
MgCl                 -> -MgCl
SiH3                 -> -SiH3
PH2                  -> -PH2
AsH2                 -> -AsH2
BH3                  -> -BH3, B formalCharge -1
NH3                  -> -NH3, N formalCharge +1
OH2                  -> -OH2, O formalCharge +1
OH3                  -> -OH3, O formalCharge +2
CH2Boc               -> -CH2Boc, Boc as monovalent terminal token
```

同时保留这些模板优先用例：

```text
Boc
Fmoc
Ts
TBDMS
TBDPS
Ph
```

它们应继续走命名模板，不进入普通价键 parser。

以下用例应保持 invalid，避免例外价态被放得太宽：

```text
BCl3
NMe4
OCl3
OCl4
```
