# ChemSema Link 模型

状态：已批准实现

格式范围：CCJS `0.1`

术语：本文的 `Link` 是 ChemSema 的编辑语义，不等于 group，也不等于
CDX/CDXML 中名称含有 “Link” 的节点类型。

## 1. 目标

Link 表示“对象仍可独立选择和编辑，但一个对象的语义或自动布局依赖另一个
对象”。它解决以下统一问题：

- 分析文本与单个分子；
- 括号与右下角重复次数文本；
- 电荷、自由基、电子或孤对符号与原子；
- 将来可明确增加的对象关系。

Link 不是通用约束系统，也不是任意对象之间的用户连线。只有内核注册过、
能够唯一判定关系类型和端点角色的组合才允许显式 Link。

## 2. 与 group 的严格区别

| 行为 | group | Link |
| --- | --- | --- |
| 所有权 | group 包含 child | 文档级有类型关系图 |
| 普通双击 | 选中整个 group | 不扩展 Link |
| Alt+双击 | 不改变 group 规则 | 选中命中对象及同一 Link 连通分量 |
| 移动 | group 子对象一起移动 | 只有关系类型声明的自动布局会跟随 |
| 复制 | 复制 group 会复制 children | 仅在所有端点同时复制时复制关系 |
| 删除 | 按 group 所有权处理 | 删除端点会删除关系，不级联删除其他端点 |
| CDXML | 使用 CDXML group | 通用 Link 策略不导出 |

任何代码不得通过 group 的祖先关系推断 Link，也不得通过 Link 扩大普通双击
选择。

## 3. CCJS 数据契约

每个 `SceneObject` 都有：

```json
{
  "linkPolicy": "auto"
}
```

取值只有：

- `auto`：默认值。只在一个已注册关系规则能找到唯一候选时自动建立或更新；
- `linked`：用户显式建立关系。关系不会因为几何距离增加而解除；
- `unlinked`：用户显式禁止自动关系。现有关系立即解除。

文档根对象有 `links`：

```json
{
  "links": [
    {
      "id": "link_12",
      "kind": "analysis-caption",
      "endpoints": [
        { "entityId": "obj_molecule_1", "role": "source" },
        { "entityId": "obj_text_4", "role": "caption" }
      ],
      "data": {}
    }
  ]
}
```

`entityId` 可以指 scene object 或分子 resource 内的 node。文档内所有 entity id
必须唯一；Link 端点必须存在；同一关系中端点不得重复。

已注册的 `kind`：

- `analysis-caption`：一个完整单分子对象 `source` + 一个文本对象 `caption`；
- `bracket-repeat-label`：一个括号对象 `bracket` + 一个纯数字文本对象 `label`；
- `atom-symbol`：一个原子 node `atom` + 一个支持化学语义的 symbol 对象 `symbol`。
- `chemical-property-display`：一个或多个有序 `basis` 端点 + 一个标准文本
  `display`；关系由原生 ChemicalProperty 逻辑对象声明，不参与通用 Link 自动推断。

Reaction Scheme/Step 也由同一个 Link 菜单控制，但它是标准 typed relation，存放在
`reactionSchemes`，不复制进通用 `links[]`。这是为了保留 reactant、product、
arrow、plus、above/below 和 atom mapping 的完整角色。

关系类型、端点数量、端点角色和对象类型均由内核验证。未知关系不得静默降级。

## 4. 菜单与选择

选择工具右键菜单为：

```text
Link >
  Auto
  Link
  Unlink
```

- 只要当前选择包含可设 Link 策略的对象，就显示此菜单。
- `Link` 只有在当前选择恰好匹配一个已注册关系签名时可点击。
- 两个分子、两个普通文本、混合了多余对象等不能唯一确定语义的选择，
  `Link` 必须为灰色。
- `Auto` 把所选对象策略设为 `auto`，清除其显式关系，然后立即执行一次确定性
  自动判定。
- `Unlink` 把所选对象策略设为 `unlinked`，删除涉及它们的关系；对象本身不删。
- 显式 `Link` 把端点对象策略设为 `linked` 并建立已判定类型的关系。
- 普通双击优先选择命中对象所属的整个 group；未处于 group 内的分子仍选择完整
  连通分量。
- Alt+双击选中命中 entity、其所属 scene object（如适用）及 Link 图中的整个
  连通分量，但不按 group 展开 Link 外的对象。
- 选择工具聚焦一个有 Link 的对象时，用青色虚线框显示关联对象，并用细虚线
  连接中心；这是聚焦提示，不进入 selection。

## 5. 自动判定

自动判定只在提交操作后运行，不在鼠标每一帧做最近邻搜索。触发点包括导入、
粘贴、拖拽提交、文本编辑提交、插入和删除。

共同规则：

1. `unlinked` 永不参与自动判定；
2. `linked` 只服从已有显式关系；
3. `auto` 只接受类型、内容和空间门限都满足的候选；
4. 必须恰好有一个最佳候选；平分或多个候选均不建立关系；
5. 自动关系可以在下一次提交时因规则不再满足而解除；
6. 不允许用“找不到就沿用旧元数据”之类 fallback。

当前自动规则：

- 括号重复标签：文本是大于等于 2 的纯整数，并落在括号右下角的规范候选区；
- 原子符号：符号中心在唯一原子的命中半径内，化学作用由符号的明确
  `chemicalRole/chargeDelta/radicalDelta` 决定；
- 分析文本不由空间自动生成；它只能由分析栏 Paste 创建，创建后是显式关系。
- 反应：带箭头端点的非机制直线箭头建立局部轴；分子、加号和上下方文本只有在
  唯一最佳候选成立时组成 inferred Reaction Step。无箭头线和 curved/
  curved-mirror 机制箭头不参与。歧义门限和完整规则见
  [`logical-object-native-model.zh-CN.md`](logical-object-native-model.zh-CN.md)。

## 6. 分析文本

分析栏的 Paste 只在选中完整且唯一的单分子时可用。点击后创建一个普通可编辑
文本对象，默认内容为：

```text
Formula: C8H10N4O2
Formula Weight: 194.19
Exact Mass: 194.0804
```

具体数字使用分析栏当前精度。创建后：

- 文本与分子建立 `analysis-caption`；
- 文本水平中心与分子包围盒中心一致，位于分子下方；
- 在文本未被手动移动时，分子包围盒中心改变会带动文本重新居中；
- 用户手动移动文本后，`anchorMode` 从 `follow` 变为 `fixed`，内容仍实时更新；
- 每次已经提交的结构操作完成后更新 Formula、Formula Weight 和 Exact Mass；
- 编辑字段名或其他说明文字不解除关系；
- 修改或删除任一生成值时，先保留用户编辑，再自动改为 `unlinked`，冻结全文，
  并显示由内核提供的单按钮提示：
  `This change to the text has caused auto-updating to be disabled.`；
- 删除分子只删除关系，保留并冻结文本；删除文本不影响分子；
- 右键 Unlink 与上述冻结行为相同，但不弹提示。

文本 payload 保存字段模板、最近一次生成值、精度及 `anchorMode`；关系本身只保存在
文档根 `links`，不得再把另一端 id 复制进 `meta`。

## 7. 括号与符号适配

- 括号重复次数以 `bracket-repeat-label` 为唯一关系来源。重复单元计算不得再读取
  `linkedTextObjectId`、`linkedBracketObjectId` 或 `linkKind`。
- atom-symbol 以 `atom-symbol` 为唯一附着关系来源。`attachedAtomId` 可以作为
  渲染/导出时重建的派生值，但不得作为关系 fallback。
- symbol 为 `unlinked` 时，靠近原子也不改变原子电荷或自由基。

## 8. 导入导出

- CCJS 完整保存 `linkPolicy`、`links` 和分析文本 payload。
- ChemSema 剪贴板仅在所有端点都被复制时携带并重映射关系；跨标签页、Web 与
  桌面使用同一 CCJS 片段协议。
- 通用 Link 策略和分析自动更新语义不导出到 CDX/CDXML，也不弹丢失警告。
- 括号重复语义、原子电荷/自由基等已有标准表达继续按标准字段导出。
- CDX/CDXML 导入仍按各标准对象本身恢复括号、电荷和自由基等化学语义，但不据此
  恢复 ChemSema Link 策略；不得依赖 ChemSema 私有属性。
- 分析文本导出为普通标准文本。按产品决定，ChemSema 的自动更新关系不映射为
  CDX/CDXML `chemicalproperty`，再次导入时也是普通文本。
- 原生 ChemicalProperty 与分析文本是两个不同功能。只有
  `chemicalProperties` 中的 `chemical-property-display` 会写成标准
  CDX/CDXML `chemicalproperty`；它的通用 `linkPolicy` 仍不写入标准文件。
- Reaction 的 ChemSema `linkPolicy/bindingOrigin` 不导出，但 Scheme/Step 及其标准
  角色和 atom mapping 按 CDX/CDXML 标准对象导出。

## 9. 生命周期与历史

- Link、Auto、Unlink、分析 Paste、因文本编辑导致的自动 Unlink 都是单个可撤销
  命令。
- undo/redo 必须同时恢复对象策略、关系、文本和派生化学状态。
- 加载时先迁移旧括号 link 元数据，再验证；迁移完成后删除旧字段。
- 删除、剪切、粘贴、group/ungroup、跨标签复制、保存重开均须保持有效关系；
  孤立端点在提交时被清理。

## 10. 不变量与门禁

必须有自动测试覆盖：

- 每个 scene object 序列化后都有合法 `linkPolicy`；
- 关系端点存在且签名合法；
- 两个分子不能 Link；
- 普通双击 group 与 Alt+双击 Link 的选择结果不同且稳定；
- Auto/Link/Unlink 菜单 enabled/checked 状态；
- 分析 Paste、结构更新、锚点跟随、手动移动、字段名编辑、数值编辑和提示；
- 括号计数与 atom-symbol 在 linked/auto/unlinked 三种策略下的语义；
- 删除、复制粘贴、undo/redo、CCJS 保存重开；
- CDX/CDXML 不写 ChemSema 私有 Link 字段，标准语义可往返。
