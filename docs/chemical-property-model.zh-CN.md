# ChemicalProperty 原生模型

状态：NR-007 已完成

格式范围：CCJS、CDXML、CDX、WASM、桌面服务与 GUI

## 1. 对象边界

ChemicalProperty 是一个逻辑对象，不是另一种文本。它保存：

- 稳定的内部 `id` 和仅用于来源追踪的 `sourceId`；
- `propertyType.code/name`；
- 有序 `basisEntityIds` 和尚未解析的来源 ID；
- 可选的 `displayObjectId`；
- `isActive`；
- `valueOrigin`：`imported`、`authored` 或 `calculated`；
- `calculationState`：`static`、`current`、`stale` 或 `unsupported`；
- 可选的 `lastCalculatedValue`。

显示值由普通文本对象承载，因而字体、字号、颜色、位置、选择、移动和导出完全复用
现有文本链路。逻辑对象与文本通过 `chemical-property-display` Link 关联。

分析栏 Paste 生成的 `analysis-caption` 不是 ChemicalProperty：前者是 ChemSema 的
实时分析文本，导出到 CDX/CDXML 时仍是普通文本；后者对应标准
`chemicalproperty` 对象。

## 2. 类型规则

官方类型按下列规则解释，不猜测公式、分子量或精确质量等未分配的枚举：

| 输入 | 内核值 | 行为 |
| --- | --- | --- |
| 属性缺失 | `code=null, name=null` | undefined |
| `0` / `Unspecified` | `code=0, name=Unspecified` | 明确未指定 |
| `1` / `ChemicalName` | `code=1, name=ChemicalName` | 可调用命名提供器 |
| 数值 `> 0x8000` | 保留数值和可选名称 | 自定义 CDX 类型 |
| 其他名称 | `code=null, name=原名` | CDXML 自定义类型 |

`2..0x8000` 以及 `0x8000` 本身不被当作自定义类型。已知码和名称冲突时拒绝修改，
不选一个值兜底。

仅名称的自定义类型可写 CDXML，但 CDX 的 `UINT32` 字段无法承载名称；此时 CDX
导出明确失败。数值自定义类型可在 CDX 中无损往返。

## 3. 导入与导出

导入按源文档顺序读取所有 `chemicalproperty`：

1. 类型字段按上节规范化；
2. `BasisObjects` 按原顺序映射到 molecule、node 或 bond；
3. 无法映射的来源 ID 保存在 `unresolvedBasisIds`，不伪造对象；
4. `ChemicalPropertyDisplayID` 若指向文本，则复用该文本，绝不改写文件提供值；
5. 缺失 `ChemicalPropertyIsActive` 等价于 false；
6. 活动的 `ChemicalName` 标为 `stale`，其他活动类型标为 `unsupported`。

导出以当前文档对象为准重新分配和引用 CDX/CDXML ID。已删除的属性不会由来源交换树
复活；仍存在的未知属性和子对象继续由交换层无损合并。
作为显示对象使用的空文本仍保留身份、位置和来源 ID；它不能被普通“空文本清理”
删除，否则标准显示引用会失效。

## 4. 编辑与布局

选择完整且唯一的单分子时，右键菜单显示 “Chemical Property...”；选择一个属性的
显示文本时显示同一入口。对话框可编辑类型码、类型名、显示值和自动更新状态，也可
删除已有属性。

新建显示文本默认水平居中于分子包围盒，下方间距 9 pt，Arial 10 pt、12 pt 行距。
创建后文本仍是普通可移动文本；ChemicalProperty 不强制自动跟随几何移动，因为标准
文件记录的是显示对象的具体位置。移动分子不会改变其计算状态。

手工编辑活动显示文本代表用户接管该值：`isActive=false`、
`valueOrigin=authored`、`calculationState=static`，并由内核发出一次明确提示。

## 5. 重新计算

当前可重新计算的标准类型只有 `ChemicalName`。内核不内置名称猜测，而是输出
`NomenclatureRequestV1`，其中结构使用规范化 Chemical Graph V2。提供器返回值后，
同一可撤销命令更新文本并写入：

```text
valueOrigin = calculated
calculationState = current
lastCalculatedValue = 返回值
```

每个已提交操作前后比较规范结构指纹。坐标、布局、选择和样式变化不会失效；元素、
电荷、键级、连接关系等结构变化会把活动名称标成 `stale`。活动但没有已实现提供器的
类型明确为 `unsupported`，不会沿用名称提供器。

## 6. 删除、Link 与剪贴板

- 删除显示文本：保留属性，清空 `displayObjectId`。
- 删除部分 basis：保留仍存在的有序端点。
- 删除最后一个 basis：删除属性；原显示文本变为普通文本。
- 删除属性：默认连同显示文本删除，不删除 basis。
- 对显示文本执行 Unlink：解除显示关联、关闭自动更新，文本保留。
- Alt+双击任一端点：选择该属性 Link 连通分量；普通双击仍遵循 group 规则。
- 只有所有 basis 和 display 都进入复制片段时才复制属性；粘贴会分别重映射属性、
  scene object、node、bond 和关系 ID，绝不修改已有关系。

## 7. 门禁

回归测试覆盖：

- 空值、缺失值与自定义类型边界；
- CDXML/CDX 导入导出和实时引用重写；
- 文件值不被改写；
- 结构变化与纯几何变化的失效差异；
- 命名请求和结果回填；
- 新建、编辑、删除、Unlink、undo/redo；
- display/basis 生命周期；
- 复制粘贴 ID 重映射；
- 右键菜单和内核对话框 schema。
