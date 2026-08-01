# CDXML/CDX 文本与原子标签字段审计

本文记录 ChemSema 对 CDXML/CDX 文本与原子标签字段的系统审计。审计依据 CambridgeSoft CDX SDK 的公开镜像、真实 ChemDraw 的受控对照输出，以及公开语料的连续三代往返结果。

字段当前是否已经达到完整复核标准，以 [CDX/CDXML 导入字段复核总账](./cdx-cdxml-field-verification.zh-CN.md) 为准；专项文档中的历史结论若发现新反例，应先在总账降级，再完成修复和新门禁。

## 审计结论

- 413 个公开文件中，407 个可导入案例完成连续三代保存与重开；404 个完全一致，1 个为预期安全清洗，2 个为预期无损归一化。
- 2 个文件按预期拒绝导入，4 个传输编码文件跳过。
- 在本审计覆盖的文本/标签语义和几何门禁内，未预期失败、语义漂移、非幂等和未分类计数漂移均为 **0**。
- 标签专项扫描覆盖 5,823 个原子标签、37 种相关字段组合；30 个受控 ChemDraw 对照案例全部符合预期。

这个“0”只表示下述字段族与当前公开语料的严格门禁已清零，不表示 CDXML/CDX 所有对象和所有私有扩展都已穷尽。

## 官方字段职责

| 字段 | 官方对象/类型 | 官方含义与默认 | ChemSema 处理原则 |
| --- | --- | --- | --- |
| `LabelDisplay` | Node / INT8 | 原子标签相对节点的显示位置；默认 `Auto` | 仅显式非 `Auto` 值固定标签锚点和保留作者文本，不由它合成新行 |
| `LabelAlignment` | Node / INT8 | 多行原子标签的对齐方式；默认 `Auto` | 只控制行对齐元数据，不改变字符顺序和自动化学布局方向 |
| `LabelJustification` | Node / INT8 | 原子标签文本的排版方式；默认 `Auto` | 只用于原子标签，不参与自由文本排版 |
| `Justification` | Text / INT8，已废弃 | 旧式自由文本对齐 | 仅作自由文本后备值，优先级低于 `CaptionJustification` |
| `CaptionJustification` | Text / INT8 | 自由文本对齐 | 自由文本的首选对齐字段 |
| `InterpretChemically` | Text / BOOL | 是否把自由文本按化学含义解释 | 保留字段语义，不把它混同为 Node 标签布局开关 |
| `LineHeight` | Text/Node / UINT16 | 旧式行高；`0` 为 variable，`1` 为 auto | 兼容读取；优先级低于对象专用的新字段 |
| `LabelLineHeight` | Node / INT16 | 原子标签行高 | 原子标签首选行高；再回退对象/根级旧字段 |
| `CaptionLineHeight` | Text / INT16 | 自由文本行高 | 自由文本首选行高；再回退对象/根级旧字段 |
| `WordWrapWidth` | Text/Node / INT16 | 自动换行宽度 | 原样保留并纳入往返门禁 |
| `LineStarts` | Text/Node / varies | 文本软换行边界或节点化学布局的派生边界 | 必须结合 `WordWrapWidth` 和对象角色解释，不能一律向源文字插入换行 |

字段优先级按对象类型分开处理：Node 标签使用标签字段；Text 自由文本使用 caption 字段，旧 `Justification`/`LineHeight` 仅作为兼容后备。根级行高默认也按相同的“专用字段优先、旧字段后备”规则继承。

## ChemDraw 对照结论

受控文件包含 30 个左右、左、上、下连接方向以及各种对齐/显示组合，交由真实 ChemDraw 打开并导出 SVG。观察结果如下：

1. `LabelAlignment`、`LabelJustification` 和旧 `Justification` 不会覆盖随连接方向决定的化学字符顺序与堆叠。
2. 显式 `LabelDisplay` 会保留作者写入的字符顺序和行结构；`Above`/`Below` 不会凭空把单行标签拆成两行。
3. 使用 `BeginAttach`/`EndAttach` 字符索引的标签必须保留作者顺序，否则附着点会指向错误字符。
4. 方括号包围的查询标签（例如 `[C,N,P]`）是一个整体，不能按普通末端氢规则拆分或翻转。

## 本轮清理的系统性错误

1. 将 `LabelAlignment`/`LabelJustification` 错当成标签字符流方向。
2. 显式 `LabelDisplay` 的 `Above`/`Below` 被错误实现为自动造行。
3. 导入后丢失作者已有换行与样式段结构。
4. 带 `BeginAttach`/`EndAttach` 的标签被重新排序。
5. 方括号查询标签被拆成 `C,N,P]` 与 `[` 两部分。
6. 自由文本错误读取 `LabelJustification`，没有执行 `CaptionJustification > Justification > 文档默认` 的优先级。
7. `LabelLineHeight`、`CaptionLineHeight`、旧 `LineHeight` 及根级默认的继承和自动行高不一致。
8. CDX 二进制层缺少行高/换行宽度字段和 `LabelAlignment=Best` 的完整编解码。
9. 单节点分子往返时被误降级成自由文本。
10. 旧公开语料门禁只比较源标签，没有比较最终显示文本、行结构和文字几何，因而会漏报上述问题。

## 自动门禁

公开语料门禁现在逐代比较：

- 原子标签源文本、最终显示文本、行结构和样式段；
- 对齐、布局方向、锚点、换行宽度和解析后的行高；
- 自由文本内容、样式段、对齐、换行和几何；
- 标签/文本位置、边界框和行高（允许 0.5 pt 的数值舍入容差）；
- 分子、无头箭头、括号语义，以及对象/资源/样式计数。

详细运行结果写入 `tmp/public-cdxml-roundtrip-label-audit/report.json`。基准摘要和运行说明见 `benchmarks/public-cdxml/README.zh-CN.md`。

## 尚未声称解决的范围

- 公开 SDK 对部分现代 ChemDraw 的视觉实现细节没有完整说明，字体字形度量仍需真实 ChemDraw 和肉眼对照。
- 0.5 pt 几何容差用于吸收 CDXML/CDX 数值量化，不允许文本、行结构或样式段发生差异。
- 本报告没有宣称审计了所有 CDX 属性、所有图形对象或厂商私有扩展；后续字段族应采用同样的“官方定义 + ChemDraw 对照 + 多代语料门禁”流程。

## 规范来源

- [Revvity 当前 CDXML DTD](https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd)
- [Object Tag 对象](https://chemapps.stolaf.edu/iupac/cdx/sdk/ObjectTag.htm)
- [Node 对象](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/Node.htm)
- [Text 对象](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/Text.htm)
- [LabelDisplay](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/Node_LabelDisplay.htm)
- [LabelAlignment](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LabelAlignment.htm)
- [LabelJustification](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LabelJustification.htm)
- [Justification（旧字段）](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/Justification.htm)
- [InterpretChemically](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/InterpretChemically.htm)
- [LineHeight](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LineHeight.htm)
- [LabelLineHeight](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LabelLineHeight.htm)
- [CaptionLineHeight](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/CaptionLineHeight.htm)
- [WordWrapWidth](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/WordWrapWidth.htm)
- [LineStarts](https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/properties/LineStarts.htm)

## `LineStarts` 导入规则补充（2026-08-01 复核）

- 官方 `INT16ListWithCounts` 的“计数前缀”只存在于二进制 CDX：CDX 先写一个 `UINT16` 数量，再写各位置；CDXML 属性只写位置列表本身。不能把 CDXML 的第一个数误当成数量。
- `LineStarts` 的位置按作者保存的 styled-text 原始 UTF-8 字节流计算；`′`、`°`、`α` 等多字节字符必须按编码长度推进。CR/LF 也占用位置；渲染时可以把 CRLF 规范化为一个换行，但计算源偏移时仍按原始字节流推进。
- 对带 `WordWrapWidth` 的普通文本，内部位置可表达没有写入 XML 字符串的软换行；此时导入器才把有效内部位置物化为行结构。跨多个 `<s>` run 时连续累计偏移，并在切分后继承该 run 的字体、字号、颜色、face 和上下标样式。列表最后一个值允许等于文本长度，作为结束哨兵，不产生空行。
- 没有 `WordWrapWidth` 时，`LineStarts` 本身不命令 ChemDraw 把无换行字符串拆行。静默探针中普通文本 `ABC + LineStarts="2 3"` 保存后字段被删除且 SVG 仍为单行；源文字中的真实 CR/LF 仍须原样保留。
- 节点化学标签的 `LineStarts` 是 ChemDraw 根据连接方向、`LabelAlignment`、隐式氢和电荷显示重新生成的布局边界，不是源公式的换行位置。同一个双键相连的 `NH+`，输入缺失该字段或写成 `2 4 6`，ChemDraw 22.2 均输出相同 SVG，并保存为 `LabelAlignment="Above" LineStarts="3 5"`。导入器必须保留源公式 `NH+`，再由化学标签布局生成 `H+` 在上、`N` 在锚点行的显示；不能先改写为 `NH\n+`。
- 节点标签开头的同位素质量数属于紧随其后的首个元素组，不是独立化学组。比如作者样式为上标的 `13` 与 `C` 必须共同组成锚点行 `13C`，后续 `H2` 才能按连接方向换到另一行；键的附着字符是该组第一个字母 `C`，不是质量数首位。字符串分组与保留字体/上下标的 styled-run 分组必须调用同一个规则源，避免导入语义正确但绘制再次拆组。
- `Isotope` 与作者标签是同一语义的两个层次，不是两个要叠加的可见对象。若节点的源标签已经以相同质量数和相同元素开头（如上标 `13` + `CH3`、`2` + `H`），或氢节点以 `D`/`T` 简写同一质量数，渲染器保留作者的字体与上下标呈现，不再由 `Isotope` 生成第二份质量数；只有标签未编码该质量数时才合成注记。该判断同时用于原子属性绘制和查询标签避让宽度。
- 公开语料复核中，49 个带 `LabelAlignment` 的节点 `LineStarts` 标签全部没有真实换行，而 87 个非节点/普通文本样本中有 79 个带真实换行；其余没有真实换行的普通文本均带 `WordWrapWidth`。这个分群与静默探针一致，因而采用字段上下文规则，而不是按字符串、节点 ID 或文件名分支。

## 旧式字体家族/字形拆分规则（2026-07-21）

- 旧式 CDXML fonttable 会把字形写进字体名，例如 `Arial Bold`、`Arial Bold Italic`、`Helvetica Bold`、`Helvetica Bold Oblique`；同一文本 run 的 `face` 通常还会重复保存 bold/italic 位。
- 导入 CSS/SVG 字体时，将末尾的 `Bold`、`Italic`、`Oblique`、`Bold Italic`、`Bold Oblique` 从家族名拆出，家族分别回到 `Arial` 或 `Helvetica`，字形合并进 `fontWeight`/`fontStyle`。
- 字体名字形和 `face` 按逻辑“或”合并：任何一处声明粗体或斜体都必须保留，不能因规范化家族名丢失样式。
- 该规则适用于节点标签、自由文本和文档默认文字样式，不按反应类型、化学名称或文件来源分支。

## 字体与 `face` 语义边界（2026-07-22）

- CDX/CDXML 字体来自文档自己的 `fonttable`，不是 ChemSema 维护的一组封闭枚举。导入时保留任意 `fontFamily` 字符串；编辑器只提供常用字体建议，不得因字体不在建议列表中替换或丢弃它。
- ChemDraw `face` 位定义为：`1` 粗体、`2` 斜体、`4` 下划线、`8` 轮廓、`16` 阴影、`32` 下标、`64` 上标；`32 | 64 = 96` 表示化学式自动排版。组合值逐位解释，不按完整整数建立案例表。
- `face` 只允许存在于 CDX/CDXML 读写边界。导入后必须拆成 `fontWeight`、`fontStyle`、`underline`、`outline`、`shadow`、`script`；ChemSema 自有 JSON 的 run、默认样式和元数据均不得保存源位掩码。导出时再从这些明确字段逐位组合。
- 字体家族与样式正交：字体名中旧式 `Bold`/`Italic` 后缀先按上一节规则归一化，随后与 `face` 的粗体、斜体位做逻辑“或”；轮廓、阴影和上下标位不受字体家族归一化影响。

## 节点立体标记与对象标签补充规则

本轮公共 CDXML/CDX 像素对照补充了以下导入与绘制规则。这些规则按字段语义执行，不依赖文件名、案例编号或分子结构特例。

1. `objecttag` 不是一律隐藏的元数据。`Name="stereo"`、`Name="enhancedstereo"`、`Name="number"`、`Name="query"` 和括号标签所包含的 `t/s` 文本都必须作为场景文字导入，并保留坐标、字体、字号、颜色和样式段；`Visible="no"` 的普通对象标签仍作为隐藏往返数据保留。
2. 括号标签采用字段优先级，而不是 `CreationProgram` 版本分支：同一 `graphic` 只有 `bracketusage` 时绘制它；新增 `parameterizedBracketLabel` 时，由后者提供 ChemDraw 实际生成的文字和位置，旧 `bracketusage` 仅保留为隐藏的往返数据。ChemDraw 22.2/23.1 文件是在旧结构上增加这个字段，没有改变旧字段在老文件中的含义。
3. `parameterizedBracketLabel` 是一个生成字段：即使它或内部 `t` 标记为 `Visible="no"`，只要它与 `bracketusage` 同时存在，ChemDraw 仍用它生成可见括号标签。这是此专用字段的语义例外；普通对象标签仍严格遵守 `Visible`。
4. 自动定位（`PositioningType` 缺省或 `auto`）时，文字左边缘位于承载它的右括号外侧 `0.1875 × 字号`，纵向基线继续使用 `t.p.y`。不能把 `t.p.x` 当成文字左边缘，因为它会随首字符及字体度量变化。
5. 显式定位（`PositioningType="offset"`、`absolute` 或 `angle`）时，使用文件记录的 `t.p`；这类字段会有意覆盖自动括号间距。ChemDraw 23.1 的同一文件同时包含自动和 `offset` 标签，证明这里应按定位字段分支，而不是按文件版本分支。
6. 节点具有 `EnhancedStereoType`/`EnhancedStereoGroupNum`、但文件没有保存可见 `enhancedstereo` 对象标签时，ChemDraw 会生成 `abs`、`orN` 或 `&N`。ChemSema 同样生成标签；有显式楔键时，标签放在楔键的反方向，以落入立体中心周围的空白扇区。
7. 四面体节点的 `HDot="yes"` 绘制实心圆点；`HDash="yes"` 绘制位于节点下方的两条短横线。尺寸来自文档 `BoldWidth`、`LineWidth`，不能按截图像素或单个样例写死。
8. `NodeType="MultiAttachment"` 且当前没有实际连接键时，绘制 ChemDraw 的三线星号占位标记；一旦该节点被键连接（例如金属到芳环多中心键），不再绘制星号。星号直径约为文档 `BondLength` 的 30%，线宽使用 `LineWidth`。
9. 分裂不连通分子组件时，带 `MultiAttachment`、`HDot` 或 `HDash` 语义的孤立碳节点仍属于可见内容，不能按“无键普通碳”过滤。
10. `bracketedgroup/bracketattachment@GraphicID` 是左右括号归组的权威关系。先按这组显式 ID 配对，只有没有被任何组引用的孤立括号才允许按几何位置回退配对；不能用中心或高度容差否决显式附件。
11. 同一组左右括号各自保留源 `BoundingBox` 的纵向起点和高度。组包围框只用于选择、移动和整体范围，不能把两侧强制拉到组包围框的统一高度。
12. 二进制 CDX 的 `SymbolType`（property `0x0A07`）是 `INT16` 枚举，不是可直接透传的普通整数。导入导出必须按官方值映射 `LonePair=0`、`Electron=1`、`RadicalCation=2`、`RadicalAnion=3`、`CirclePlus=4`、`CircleMinus=5`、`Dagger=6`、`DoubleDagger=7`、`Plus=8`、`Minus=9`、`Racemic=10`、`Absolute=11`、`Relative=12`。
13. `objecttag@PositioningType` 缺省值是官方定义的 `auto`，不能一概把内部 `t@p` 或 `t@BoundingBox` 当成最终显示位置。桌面 ChemDraw 生成的增强立体标签以所附节点为锚点，用缓存文本框相对节点的方向选择象限，再按字号归一化显示半径；只有 `offset`、`absolute` 等显式定位模式继续使用记录坐标。公共文件同时证明 ChemDraw JS 2.0 具有相反的生产者约定：它也省略 `PositioningType`，但 ChemDraw 打开时保留缓存位置。因此这里只在字段语义确实矛盾时按根级 `CreationProgram="ChemDraw JS …"` 兼容该生产者，而不按案例或文件名分支。
14. 键上的自动 `query` 标签以键中点为锚点，并用缓存文本框判断位于键的哪一侧。文字框必须完整落在该侧，水平/垂直留白由字号度量得到；这使不同方向的 `Rxn` 标签保持相同视觉间距，同时不改写显式 `PositioningType="offset"` 的标签。
15. 虚锲形键中每条黑段沿键轴的长度严格等于 `LineWidth`，`HashSpacing` 是相邻黑段中心距的下限。ChemDraw 取 `max(1, 1 + floor((最终键长 - LineWidth) / HashSpacing))` 条，再让首尾黑段贴住两端并均分中心距。该结论由 Default、ACS、不同 `LineWidth` 与不同 `HashSpacing` 的 0.5 pt 全域扫描和阈值附近 0.01 pt 精扫共同验证；不应复用普通虚线键的段数分配器，也不需要对 `Nickname`、外部连接点或短键增加分支。
16. CDX 二进制中的 `LineHeight`、`LabelLineHeight` 和 `CaptionLineHeight` 固定数值以 1/20 screen point 编码；解码到 CDXML/内核点值时除以 20，回写 CDX 时乘以 20。`0=variable`、`1=auto` 两个特殊值不参与缩放。真实 ChemDraw 将二进制 `160`/`165` 分别保存为 CDXML `8`/`8.25`，不能把原始整数直接当作点值。
17. 官方定义区分三种行距语义：`0/variable` 使用每一行实际最高字符分别决定相邻基线，`1/auto` 使用整个文本对象中的最高字符统一决定所有基线，其他正数是固定 screen-point 行距。Node 优先读取 `LabelLineHeight`，Text 优先读取 `CaptionLineHeight`，再回退旧 `LineHeight`；没有任何字段时，标签缺省为 variable，自由文本缺省为 auto。
18. ChemDraw SVG 保存连续的逻辑基线，比例为每 pt 对应 `2.6667` 个 SVG 用户单位。EMF 没有另一套行距规则，而是把同一组逻辑基线逐行独立量化成整数设备像素，因此相同逻辑行距在 EMF 记录中可交替相差 1 个设备像素；门禁应比较各基线量化后的坐标，不能要求每个相邻 EMF 差值完全相等。
19. Auto 行距必须查看对象内全部 styled runs，不能只取首个 run。10 pt 实测基线步进为 Arial regular `11.5` pt、Times New Roman regular `11.65` pt、Calibri regular `12.25` pt；Arial bold 为 `11.75` pt、italic 为 `11.45` pt、bold italic 为 `11.75` pt。8/18/12 pt 混合字号对象取 18 pt run，得到 `20.7` pt。上下标 face 还会提高对象的统一 auto 行距。
20. Variable 不是一个可被单一 `lineHeight` 完整表达的标量。连续行的基线步进为“上一行实际字形下深 + 下一行实际字形上升高度 × `7/6`”；其中额外的 `1/6 × 下一行上升高度` 是行间净空，而不是固定 `0.1 × 最大字号`。因此 `A→g`、`g→A`、`Q→A` 等方向可以不同，字体、字号、face、script 与字形 overshoot 也自然进入同一函数。Arial 10 pt 的 `A→g/g→Q` 为 `6.1834/10.5584 pt`，Times New Roman 为 `5.3667/10.0250 pt`，Calibri 为 `5.5333/9.2667 pt`；上下堆叠的 `H→N` 在 7/10/14/18 pt 下为 `5.8334/8.3417/11.6667/14.9917 pt`，且不随 `MarginWidth=0.8/1.6/2/3.2` 改变。CCJS 用 `lineHeightMode` 明确区分 `fixed/auto/variable`，用 `lineHeight` 保存可用的默认步进，并用 `lineAdvances[i]` 明确保存第 i 行到第 i+1 行的实际步进。单行标签也保留 mode 和正数 `lineHeight`，只是没有第二条基线时不会表现出来；不得从恰好相同的数值反猜模式。
21. 对本机 ChemDraw 安装目录中的 19 份 `.cds` 样式表进行静默规范化后，多数样式未写行距字段，应按上述官方缺省继承；`J. Het. Chem` 明确写出 `LabelLineHeight=6`、`CaptionLineHeight=7.10`。样式表省略字段不是固定 `1.15×字号` 的同义写法。
22. `LabelDisplay="Left"` / `"Right"` 的固定边缘按可连接字符解析：首尾下标参与锚定，上标不参与并向内寻找非上标字符。该规则由左右、居中、前导/尾随、单脚标/混合脚标和作者 `BoundingBox` 探针共同验证；自动化学布局仍按原规则排除自动生成的上下标。
23. 多连接自动化学标签使用 ChemDraw 的“最大开放扇区”规则：按实际连接角排序，取最大相邻角间隙，再以连接占据补弧的中线进入水平优先的 `135°/45°/135°/45°` 文字流扇区。`0.001°` 内的并列间隙有明确顺序：水平正轴开放间隙优先，其次取最靠近正上方的间隙，同距取顺时针侧；严格相反的两连接按无向轴 `22.5°/90°/157.5°` 分支，三等分连接按 `120°` 相位分支。ChemDraw 22.2 静默探针覆盖二、三、四连接完整 `30°` 网格共 1,079 例、边界精扫、多字号和多键长，分析器均为零误差。短键必须按源坐标量化后的实际角度判定，不能按请求角度或文件编号分支。单连接末端标签使用完整左右半平面，近似垂直时才进入碰撞分支。
24. CDXML `<s>` 的字体是按作者顺序推进的 styled-string 状态，不从根级 `LabelFont`/`CaptionFont` 或父 `<t font>` 起步。首个没有 `font` 的 `<s>` 使用 Arial；后续缺省 `font` 只继承前一个 `<s>` 的字体；显式但不在 `fonttable` 中的字体 ID 同样解析为 Arial，并成为后续缺省 run 的当前状态。ChemDraw 22.2 静默探针同时覆盖普通原子标签、Nickname、折叠 Fragment 标签、自由文本、首段缺省、显式未知 ID、先缺省后显式以及 `<t font>` 干扰项；保存回 CDXML 后均遵守这条顺序规则。内核仍在 CCJS 中保存明确 `fontFamily`，不得保留这个隐式状态或缺失 ID。

对应回归测试覆盖显式/隐藏对象标签、自动/显式对象标签定位、缺省增强立体标签、楔形虚线节距、`HDot`、`HDash`、未连接 `MultiAttachment`、显式括号附件、CDX 符号枚举，以及 fixed/auto/variable 行距、混合字号和逐行基线步进；公共图像门禁继续负责验证这些语义的最终像素位置和尺寸。行距字段在总账中暂时保持 `in-review`，直到 ChemSema SVG/EMF 与 ChemDraw 探针报告进入自动门禁后再升级为 `verified`。
