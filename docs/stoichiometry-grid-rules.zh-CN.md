# 反应步骤与化学计量表规则

## 1. 对象边界

`ReactionSchemeData/ReactionStepData` 是反应语义，`StoichiometryGridData` 是引用反应参与物的计算与展示对象。二者都是 CCJS 原生类型，不借用普通 `TableData`，也不把反应关系重复写入通用 `links[]`。普通表格只与化学计量表共用网格绘制原语。

反应步骤明确保存反应物、产物、箭头、加号、箭头上下对象和原子映射。化学计量表明确保存来源步骤、绑定来源和状态、锚定模式、组件、行、数据项、显示状态、编辑状态、只读状态、计算状态和样式。

## 2. 创建入口

入口位于选择工具右键菜单 `Analyze Stoichiometry`：

- 已选择且只命中一个原生反应步骤时直接使用该步骤；
- 尚无步骤时，必须同时选择一根有效反应箭头和至少两个完整分子；
- 只有显式执行分析命令时，才按分子中心在箭头轴上的投影区分箭尾侧反应物和箭头侧产物；
- 两侧任一侧为空、箭头退化或候选不唯一时拒绝创建，不按距离猜测别的反应。

创建后表格位于反应下方、与反应包围盒水平居中，默认 `follow`；用户手动移动后改为 `fixed`。

## 3. Link 语义

化学计量表参与统一的 `auto/linked/unlinked` 用户协议，但关系真源是原生字段：

- `linked`：显式指向一个反应步骤；
- `auto`：仅当组件引用及角色与某一个反应步骤精确且唯一匹配时绑定；
- `unlinked`：冻结当前数值并停止跟随和重算。

解绑时 CCJS 内部保留组件的候选实体 ID，使用户以后可以重新选择 `auto`；导出 CDXML 时不写 `ComponentReferenceID`，因此不会把冻结表误报成标准文件中的有效引用。Alt+双击反应成员或化学计量表时，选择同一步骤的参与物、箭头、附属对象和已绑定表格；该投影不建立私有 `reaction-stoichiometry` Link，也不沿其他 Link 递归扩张。

## 4. 行、列与单元格

默认行是 Formula、Molecular Weight、Mass、Amount、Equivalents、Concentration、Volume、Density 和 Yield。右键单元格可编辑值、切换隐藏和只读、隐藏或删除行/组件，并修改组件角色。命令层还支持添加属性行、添加引用组件、刷新和解绑。

每个值同时保存：

- `canonical`：计算使用的规范值；
- `display`：用户看到的文本；
- `unit`：明确单位；
- `origin`：`authored/calculated/imported/empty`；
- `calculationState`：`current/stale/incomplete/inconsistent/unsupported/empty`。

用户输入永不被计算值覆盖。冲突输入保留原值并标为 `inconsistent`。

## 5. 确定性计算

当前规则使用明确单位注册表，不识别模糊自由文本单位：

- `mass = amount × molecularWeight`；
- `amount = concentration × volume`；
- `mass = density × volume`；
- 反应物中正的最小物质的量为限制试剂；
- `equivalents = amount / limitingAmount`；
- 产物 `yield = productAmount / limitingAmount × 100%`。

每组三元关系可由任意两个值确定第三个。计算最多执行三个固定传播轮次，不做数值拟合；全部值都存在但不满足关系时标记冲突。

## 6. 生命周期与复制

- 结构提交后，仍绑定的表刷新化学式、分子量和派生值；
- 反应参与物、箭头或步骤失效时，表转为 `orphaned/unlinked` 并冻结；
- 复制完整反应和表时，反应步骤、组件引用、原子映射和表绑定统一重映射；
- 只复制表或只复制不完整反应时，粘贴结果为静态解绑表；
- 删除、剪切、撤销、重做和跨标签页 CCJS 剪贴板均经过同一生命周期校验。

## 7. 格式边界

- CCJS 无损保存 ChemSema 的完整编辑、计算和 Link 状态；
- CDXML 使用官方 `scheme/step/stoichiometrygrid/sgcomponent/sgdatum` 字段，导入和导出均由原生模型负责；
- 经复核的官方 CDX 对象和属性表没有 StoichiometryGrid 标签。含该对象的文档保存为 CDX 时明确报错，要求使用 CCJS 或 CDXML；不得静默丢表、栅格化或塞入私有未知标签。

## 8. 门禁

回归测试至少覆盖原生 CDXML 导入、CCJS/CDXML 往返、CDX 明确拒绝、绘制原语、用户输入优先、冲突状态、解绑后重新自动绑定、完整反应跨标签复制及文档结构校验。
