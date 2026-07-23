# 外部连接点解析、编辑与绘制规则

本文记录 `ExternalConnectionPoint` 的来源无关模型和 ChemDraw 实测规则。
可重复探针为 `scripts/chemdraw-external-connection-probe.mjs`，本地证据输出到
忽略目录 `tmp/chemdraw-external-connection-probe/`。

## 数据规则

- 节点存在 `externalConnection` 即表示外部连接点；不存在即为普通节点。
- `type` 是明确枚举，不允许用布尔字段、`meta` 或源格式字符串承担运行时语义。
- `number` 对应 CDXML `ExternalConnectionNum`，用于连接关系。ChemDraw 对
  `Unspecified`/`Diamond` 会按片段内外部连接点顺序显示白色序号；即使显式
  `ExternalConnectionNum="2"` 或 `"12"`，单连接点夹具仍显示 `1`，因此
  绘制序号不得读取 `number`。
- 旧 CCJS 的 `isExternalConnectionPoint: true` 只在读取边界迁移为
  `externalConnection: { type: "unspecified" }`，规范序列化只写新对象。
- 非法的 `ExternalConnectionType` 枚举或非 `u16` 的 `ExternalConnectionNum`
  在导入边界直接报错，不降级成 `Unspecified` 或静默丢失。

## CDX/CDXML

- CDX `0x0440` 是 `INT8`。ChemDraw 25.0.0.330 实测值为：
  `0 Unspecified`、`1 Diamond`、`2 Star`、`3 PolymerBead`、`4 Wavy`、
  `5 Residue`、`6 Peptide`、`7 DNA`、`8 RNA`、`9 Terminus`、
  `10 Sulfide`、`11 Nucleotide`、`12 UnlinkedBranch`。
- CDXML 缺失 `ExternalConnectionType` 或显式 `Unspecified` 都按
  `Unspecified` 读入；导出时 `Unspecified` 省略该属性，其他值明确写出。
- `ExternalConnectionNum` 是可选正整数；缺失和显式值必须在往返中保持区别。

## 尺寸和退让

设文档标签字号为 `S` pt，线宽为 `W` pt：

- `Unspecified`、`Diamond`、`Star`、`Nucleotide`、`UnlinkedBranch`
  的菱形边界半径为 `R = 0.375S + W`。
- `PolymerBead` 半径为 `2R = 0.75S + 2W`。
- `Wavy` 先计算原始跨度 `L = 1.5S + 4W`；可见总长为 `round(L)`，
  三次曲线段数为 `ceil(2L)`。振幅固定为 `0.5 pt`，线宽为 `W`。
- `Residue`、`Peptide`、`DNA`、`RNA`、`Terminus`、`Sulfide`
  使用横半径 `R`、纵半径 `2R/3` 的灰色扁菱形，填充 `#b3b3b3`，描边宽 `W`。
- 键与菱形的交点用菱形解析边界计算；圆珠使用圆边界。键端退让到边界，
  不按轴向固定偏移。`Wavy` 是覆盖在连接点上的横截标记，键明确延伸到中心。

聚合物珠由 32 层同心偏移圆和一条外轮廓构成。层中心沿左上方向移动，
半径线性收缩，亮度按正弦函数增加；所有输出面共用同一组绘制原语。

## 方向、编辑和无效组合

- `Wavy` 的长轴垂直于相连键；多键导入按片段键顺序使用第一条非零长度连接键。
- 未连接的菱形、星形和聚合物珠可稳定导出。未连接 `Wavy` 的 ChemDraw EMF
  导出会导致 COM RPC 服务异常，因此编辑器不创建该组合；导入时仍保存语义，
  并以固定水平法向绘制，避免丢数据或使用隐式回退。
- 右键把普通节点转换成外部连接点时，节点成为原子序数 0 的非元素节点，
  清除不再适用的元素标签、电荷、氢和原子属性。移除外部连接语义时，
  原子序数 0 的节点明确恢复为骨架碳。
- 类型和编号修改都进入命令历史；复制粘贴、跨标签页和 Web/桌面互通依赖
  CCJS 原生对象，不依赖 CDXML 导入元数据。
