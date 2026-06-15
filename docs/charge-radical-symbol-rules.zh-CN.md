# Chemcore 电荷与自由基符号归属规则

本文档定义 8 个电荷/电子符号的化学语义。这里的符号不是普通装饰图形；当它们靠近分子端点或 attached label 时，应归属到对应原子，并参与价态、隐式氢、红框合法性和 repeating unit expansion。

## 符号集合

当前 bracket/symbol 工具里的 8 个符号按化学语义分为：

| UI kind | 语义 | 对原子的影响 |
| --- | --- | --- |
| `circle-plus` | 正电荷 | `formalCharge += 1` |
| `plus` | 正电荷 | `formalCharge += 1` |
| `circle-minus` | 负电荷 | `formalCharge -= 1` |
| `minus` | 负电荷 | `formalCharge -= 1` |
| `radical-cation` | 自由基阳离子 | `formalCharge += 1`，`radicalCount += 1` |
| `radical-anion` | 自由基阴离子 | `formalCharge -= 1`，`radicalCount += 1` |
| `electron` | 单电子 | `radicalCount += 1` |
| `lone-pair` | 孤对电子显示符号 | 当前不参与价态修正 |

带圈正负和普通正负在化学语义上等价。带圈只是显示风格，不应导致不同的节点电荷结果。

`lone-pair` 的两个点只作为分子图形的一部分保留，不进入隐式氢和价态合法性计算。Lewis 结构扩展可以在后续格式版本中把它升级为显式孤对电子计数。

## 存储模型

这些符号仍可以作为 `SceneObject { type: "symbol" }` 存在，以保留可选择、可拖动、可独立显示的行为。但只要它们被归属到原子，就必须同时写入分子语义层。

symbol 对象保存归属信息：

```json
{
  "kind": "plus",
  "chemicalRole": "charge",
  "chargeDelta": 1,
  "radicalDelta": 0,
  "attachedFragmentObjectId": "obj_mol_1",
  "attachedAtomId": "n1",
  "attachmentSource": "endpoint",
  "attachmentDistance": 5.8
}
```

同时在节点语义上保存汇总后的状态：

```json
{
  "id": "n1",
  "element": "N",
  "charge": 1,
  "numHydrogens": 3,
  "meta": {
    "attachedElectronSymbols": [
      {
        "symbolObjectId": "obj_symbol_12",
        "kind": "circle-plus",
        "chargeDelta": 1,
        "radicalDelta": 0
      }
    ]
  }
}
```

节点上的 `charge`、后续的 `radicalCount` 或等价字段是化学计算权威；symbol 对象上的归属信息用于编辑、选择、拖动和 round-trip。不能只把符号当成独立图形，否则 expansion、导出、红框和隐式氢都会丢语义。

## 归属规则

符号归属只发生在“靠近某个原子”的范围内。候选目标包括：

- 裸端点，即没有 attached label 的 atom endpoint。
- attached label 的 heavy atom 锚点，例如 `N`、`O`、`Cl`、`NH2` 的重原子。
- 由合法 abbreviation 或 formula-like label 展开的 attachment atom，例如 `CF3` 的 `C`、`CO2Et` 的入口 `C`。

不应归属到自动生成的氢字符。用户把符号拖到 `NH2` 的 `H` 字符旁边时，实际目标仍是该 label 的重原子 `N`。

候选选择：

1. 先找距离符号中心最近的 endpoint/label anchor。
2. 距离必须小于电荷归属半径。该半径应和 symbol click/drag 的视觉间距同量级，并由测试固定。
3. 如果多个候选都在范围内，优先当前 hover/focus 的 atom；否则选距离最近者。
4. 选中模式下拖动符号时，鼠标移动过程中可以实时更新候选归属和预览红框；鼠标松开时提交到文档。

只有选中模式下才能拖动已有符号。Symbol 工具负责创建新符号；Select 工具负责移动已有符号并重新归属。

## 与标签合法性的关系

现有 label 红框来自 `meta.labelRecognition.status == "invalid"` 或简单元素标签与节点状态不匹配。电荷符号归属后，合法性必须使用“节点 + 归属符号”的综合状态重新计算。

### 非法标签少一个连接点

如果一个 formula-like label 因为少了一个连接点而 invalid，正负符号或带圈正负拖到标签旁边后，应按原子电荷重新解释该 label。只要带电状态能让价态满足，红框应消失。

例子：

```text
非法中性标签 + 正电荷符号 -> 允许正价态后 recognized
非法中性标签 + 负电荷符号 -> 允许负价态后 recognized
```

实现上不要把红框简单隐藏；必须让 `labelRecognition` 重新进入 recognized 状态，并把 charge/radical 写进 expansion。

### N/O 等元素旁边的正电荷

对支持隐式氢的元素，正电荷通常提高可连接价态，因此可能增加显示氢，并让原本超价的节点合法。

典型例子：

```text
R-O      + plus -> R-OH2+
R-N      + plus -> R-NH3+
R2-O     + plus -> R2-OH+
R3-N     + plus -> R3-NH+
R4-N     + plus -> R4-N+
```

其中 `R4-N` 在中性规则下应标红；拖入正电荷后成为四价铵型氮，红框消失。

### 负电荷减少氢

负电荷通常降低需要补足的氢数。用户给末端 `NH2` 旁边画负电荷或拖入负电荷，应该得到 `NH-`：

```text
R-NH2 + minus -> R-NH-
```

如果没有可减少的氢，负电荷不应强行通过。例如三键连接的 `N` 已经没有隐式氢可减：

```text
R3-N + minus -> invalid
```

但自由基阴离子不同，它同时带 `-1` 和一个未成对电子；在某些三配位氮场景下可以合法：

```text
R3-N + radical-anion -> allowed when radical valence model permits it
```

自由基阴离子应作为独立合法性分支处理，不和普通负电荷共用“必须能少一个 H”的规则。

## 隐式氢计算

现有文档里的旧公式：

```text
numHydrogens = typical_valence - radical_count - connection_count - abs(charge)
```

不适合直接表达用户期望的“正电荷加 H、负电荷减 H”。规则应按元素、形式电荷和自由基状态选择目标价态，再由目标价态减去已有连接数：

```text
target_valence = valence_model(element, formal_charge, radical_count, connection_count)
numHydrogens = target_valence - connection_count
```

如果是普通负电荷，还需要检查是否存在可减少的氢；不能让已经无氢的三配位 N 通过。自由基阴离子可走 radical 分支。

规则覆盖当前已经支持隐式氢的元素：

| 元素 | 中性基线 | 正电荷规则 | 负电荷规则 | 自由基说明 |
| --- | --- | --- | --- | --- |
| `C` | 骨架碳按 4 价隐式补氢，但不显示 H | 普通正电荷表示少 1 个隐式 H 后形成碳正离子；如果已经 4 根显式键则 invalid | 普通负电荷表示少 1 个隐式 H 后形成碳负离子；如果已经 4 根显式键则 invalid | 单电子自由基也表示少 1 个隐式 H；4 根显式键时 invalid。`radical-cation`/`radical-anion` 作为特殊带电自由基态可合法 |
| `B` | 3 价 | 少见，默认不自动加 H | 4 价硼酸盐/硼负离子可支持 | 按明确模板补充 |
| `N` | 3/5 价 | 4 价铵型，正电荷通常可多 1 个 H | 比中性少 1 个 H；无 H 可减时 invalid | radical-anion 可允许三配位 N |
| `P` | 3/5 价 | 4/6 价鏻型可作为扩展规则 | 比中性少 1 个 H，需保守 | 可只影响 expansion |
| `O` | 2 价 | 3 价氧鎓，正电荷通常可多 1 个 H | 比中性少 1 个 H；无 H 可减时 invalid | radical-anion 可用于氧自由基阴离子 |
| `S` | 2/4/6 价 | 3/5 价硫鎓型可支持 | 比中性少 1 个 H，需保守 | 高价硫要结合键级判断 |
| `F/Cl/Br/I` | 1 价及高价阶梯 | 高价正卤素默认不自动推断 | 卤负离子通常不挂在有机端点上，保守 invalid | 自由基卤素可保留符号但不自动加氢 |
| `Si` | 4 价 | 硅正离子保守处理 | 硅负离子保守处理 | 按明确需求扩展 |

这里的“保守”意思是：符号仍归属并进入 expansion，但如果没有明确价态规则，不应为了消红框而凭空制造一个化学不可靠的状态。

## Expansion 要求

重复单元 expansion 和 functional group expansion 都必须包含符号带来的电荷/自由基语义。

### Repeating unit expansion

当括号内原子带有归属符号时，展开后的每个重复 atom 都要复制该语义：

```json
{
  "id": "n1_r1",
  "element": "N",
  "atomicNumber": 7,
  "charge": 1,
  "radicalCount": 0,
  "numHydrogens": 3,
  "electronSymbols": [
    {
      "sourceSymbolObjectId": "obj_symbol_12",
      "kind": "circle-plus",
      "chargeDelta": 1,
      "radicalDelta": 0
    }
  ],
  "sourceAtomId": "n1",
  "repeatIndex": 1
}
```

如果符号本身位于括号内但未归属到任何内部原子，应把 repeating unit 判为不完整或不生成 expansion，避免丢失语义。

### Functional group expansion

标签旁的电荷符号如果归属到 functional group 的 attachment atom，也要写入 `functionalGroupExpansion.v1` 的 atom：

```json
{
  "id": "n1",
  "element": "N",
  "formalCharge": 1,
  "radicalCount": 0,
  "numHydrogens": 3,
  "sourceSymbolObjectIds": ["obj_symbol_12"]
}
```

识别结果的 `canonicalLabel` 可以继续是用户输入的原始标签，例如 `N` 或 `NH2`；但 `formula` 和 expansion 必须反映带电后的真实语义，例如 `-NH3+`。

## 编辑行为

### 创建

Symbol 工具创建 8 个符号时：

- 如果创建位置在 atom/label 归属半径内，立即绑定到该原子。
- 如果不在范围内，仍创建为未归属的分子符号；它参与选择和显示，但不影响价态。
- 创建后刷新该 atom 的 `charge`、`radicalCount`、`numHydrogens`、label 文本和红框状态。

### 移动

Select 工具拖动已有符号时：

- drag start 记录原归属。
- drag move 计算当前候选原子，可显示 hover/preview。
- mouse up 提交新归属；如果离开所有候选范围，符号变为未归属，并撤销对原 atom 的 charge/radical 影响。
- 整个拖动是一个 undoable command。

### 删除

删除归属符号时：

- 从原 atom 的 `attachedElectronSymbols` 中移除。
- 重算 `charge`、`radicalCount`、`numHydrogens` 和 labelRecognition。
- 如果删除正电荷导致原本四价 N 重新非法，应恢复红框。

## 化学规则总结

规则坚持以下原则：

1. 正负号是原子属性，不是文字装饰。
2. 带圈和不带圈的正负号化学等价。
3. 普通正电荷可让 N/O/S/P 等元素进入更高价态，并可能增加隐式 H。
4. 普通负电荷通常来自去质子化，应减少一个隐式 H；如果没有 H 可减，不应强行合法化。
5. 自由基阴离子/阳离子不是普通负/正电荷的简单别名；它们还包含一个未成对电子，合法性要单独建模。
6. 两点孤对符号只保留显示和分子归属，不参与价态。
7. 所有这些语义必须进入 expansion，否则复制、括号展开、导出和后续化学分析都会丢信息。
8. Viewer 不应单独推断这些规则；归属、合法性和隐式氢刷新必须由 Rust engine 统一完成。

裸碳端点的特殊规则：

- 裸端点仍是碳原子，隐式氢补到四价，但氢不显示。
- 普通正电荷、普通负电荷、单电子自由基都要求该碳还有至少 1 个隐式氢可减少。
- 如果碳已经有 4 根显式键，再拖入普通正/负或单电子自由基，应保持符号归属但把该原子标为 invalid，用和聚焦点同尺寸的红圈提示。
- `radical-cation` 和 `radical-anion` 是特殊带电自由基态，四连接碳上按独立自由基分支处理，不走“必须有 H 可减少”的普通自由基规则。

## 回归用例

这些规则应有自动化测试覆盖：

```text
circle-plus 和 plus 归属同一 N 后得到相同 charge/numHydrogens。
circle-minus 和 minus 归属同一 N 后得到相同 charge/numHydrogens。
末端 N + plus -> NH3+，红框消失。
末端 O + plus -> OH2+，红框消失。
四连接 N + plus -> N+，红框消失。
末端 NH2 + minus -> NH-。
三连接 N + minus -> invalid。
三连接 N + radical-anion -> recognized when radical branch is enabled。
无归属符号不改变任何 atom charge。
裸碳端点 + plus/minus/electron 时少 1 个隐式 H。
四连接裸碳 + plus/minus/electron -> invalid 红圈。
四连接裸碳 + radical-cation/radical-anion -> allowed。
拖动符号从 atom A 到 atom B，A 和 B 的 charge/numHydrogens 都正确刷新。
删除归属符号后恢复原 atom 的红框/隐式氢状态。
repeating unit expansion 复制内部 atom 的 charge/radical/electronSymbols。
functional group expansion 写入 attachment atom 的 formalCharge/radicalCount。
```
