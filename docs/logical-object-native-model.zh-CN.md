# ChemSema 逻辑对象原生模型

状态：NR-017 已完成

适用范围：CCJS 0.1、CDX、CDXML、浏览器与桌面编辑器

## 1. 目标

逻辑对象描述“对象之间有什么化学或文档语义”，不等同于可见图元。ChemSema
必须同时做到：

- 用来源无关的明确字段保存语义，不把标准对象长期留在 `interchange`；
- 对标准已有的关系完整导入和导出，不把纯逻辑对象强行画成新图形；
- 创建、修改、删除、排序、复制粘贴和撤销时保持引用完整；
- 几何可以唯一判定的关系使用 Link `auto`，不唯一时保持未绑定；
- 显式 Link/Unlink 覆盖 Auto，且不存在“找不到就沿用旧结果”的 fallback。

## 2. 原生对象族

CCJS 根对象的 `reactionSchemes` 保存反应 Scheme/Step，`logicalObjects` 保存其余对象：

| 对象族 | 原生结构 | 主要标准语义 |
| --- | --- | --- |
| `scheme/step` | `reactionSchemes[]` | 反应物、产物、箭头、加号、上下方对象和原子映射 |
| `altgroup` | `logicalObjects.alternativeGroups[]` | 命名替代基成员、连接原子、位置、框、颜色、层级、警告和替代链 |
| `bracketedgroup` | `logicalObjects.bracketedGroups[]` | 括号用途、重复方式、附件、穿越键和嵌套括号组 |
| `sequence` | `logicalObjects.sequences[]` | 本地序列标识符及显示文本 |
| `crossreference` | `logicalObjects.crossReferences[]` | 本地或外部序列引用 |
| `objecttag` | `logicalObjects.objectTags[]` | 带类型值、显示和定位规则的对象标签 |
| `annotation` | `logicalObjects.annotations[]` | 附着于对象的关键字/内容元数据 |
| `regnum` | `logicalObjects.registryNumbers[]` | 登记机构和登记号 |
| `represent` | `logicalObjects.representations[]` | 一个对象代表另一个对象的指定属性 |

`splitter` 属于文档布局，不属于本关系图。ChemSema 用
`document.layout.splitters[]` 原生保存每个 Splitter 的 `id`、`position` 和
`pageDefinition`；页面本身的枚举存为 `document.layout.pageDefinition`。ChemDraw 6
的 `SplitterPositions` 实际类型是对象 ID 数组，不是坐标数组，因此只在
`legacySplitterPositionIds[]` 中明确保真，不能参与几何计算。

## 3. 身份、引用和来源

- 每个 Scheme、Step 和逻辑对象都有全局唯一非空 `id`。
- 原生引用一律使用 CCJS entity id；不得保存数组下标。
- 导入时确实无法解析的标准 source id 放在明确的
  `unresolved...SourceId` 字段，不能猜成任意对象。
- 同一 source id 同时出现在容器汇总字段和精确子对象上时，精确子对象优先。
  例如括号组的 `graphicIds` 不能遮蔽单侧括号自己的 `graphicId`。
- `bindingOrigin` 只有 `authored/imported/inferred/none`，用于区分用户创建、标准
  导入和 Auto 推断；它不替代 Link policy。

## 4. Reaction 与 Link Auto

Reaction 是 typed relation，不写进通用 `links[]`：

1. 只有至少一个端点带箭头的直线箭头可成为反应轴；
2. curved/curved-mirror 机制箭头和 head/tail 都为 `none` 的线明确排除；
3. 每根箭头建立自己的局部轴，因此横向、纵向和斜向使用同一规则；
4. 分子按轴向投影分为 reactant/product；加号使用同一侧向规则；
5. 文本只有位于箭身投影范围且与轴保持最小间距时才成为 above/below 对象；
6. 候选必须唯一最佳；最优与次优距离差不超过 `0.1 × 默认键长` 时视为歧义，
   不建立关系；
7. `auto` 在提交操作后重算，生成 `bindingOrigin=inferred` 的 Step；
8. 显式 Link 生成 `linkPolicy=linked`、`bindingOrigin=authored` 的 Step；
9. Unlink 删除涉及所选对象的 typed Step，并把对象设为 `unlinked`；
10. 从 CDX/CDXML 导入的 Step 是显式标准关系，Auto 不覆盖其箭头。

删除 Step 或 Scheme 时，引用它的 StoichiometryGrid 不删除。网格保留现有值，
解除 source step，并进入 `orphaned`；计算值冻结为导入值。

## 5. 其他对象的规则

### Alternative Group

- 成员和连接原子必须存在且不重复。
- `position`、`boundingBox`、`textFrame`、`groupFrame`、`opacity`、`color`、
  `zIndex`、`visible`、`warning` 和 `ignoreWarnings` 都是显式可编辑字段。
- `supersededById` 可以引用仍存在的文档实体或逻辑对象；删除目标、复制子图和
  粘贴时必须分别清理或重映射，不能留下悬空引用。
- 删除成员后组仍有成员或连接点才保留；空组自动清理。
- 导出时成员移动到 `altgroup` 子树，连接原子写 `AltGroupID`。

### Bracketed Group

- `usage`、`polymerRepeatPattern`、`polymerFlipType` 使用完整枚举，不保存 face。
- 每个 attachment 明确引用一侧括号；每个 crossing bond 同时引用穿越键和内侧原子。
- `nestedGroupIds` 保存标准允许的递归 `bracketedgroup` 子层级；一个子组只能有
  一个父组，禁止自引用和环。导出必须重建嵌套树，不能压平成页面同级对象。
- attachment 缺括号、crossing 缺键或原子时删除失效子关系；组没有有效 attachment
  或没有被括对象时清理。

### Sequence 与 Cross Reference

- 本地 Sequence identifier 在文档内唯一。
- 没有 `container/document` 的 Cross Reference 必须引用本地 Sequence identifier。
- 删除 Sequence 会级联删除只引用该本地序列的 Cross Reference；外部引用不受影响。

### ObjectTag、Annotation、RegistryNumber、Representation

- owner/target 可以是 scene object、atom node 或 bond。
- 删除 owner 时删除附着元数据；Representation 的 owner 或 target 任一失效即删除。
- ObjectTag 的 `long/double` 值必须能按声明类型解析。
- `visible` 只控制标准标签的可见语义；没有显示文本对象时不凭空生成像素。
- `represent` 必须同时有有效 owner、target 和非空 attribute。

## 6. 编辑入口

选择对象或在空白画布右键，选择 `Logical Objects...`。该内核驱动对话框覆盖全部
对象族：

- 左侧按对象族和文档顺序列出实例；
- `New` 使用当前有序选择作为 owner、target 或成员的默认候选；
- `Apply` 走统一 `set-logical-object` 命令；
- `Delete` 和上下移动分别走 `delete-logical-object` 与
  `reorder-logical-object`；
- 所有操作使用同一命令历史、校验和桌面/WASM 接口。

Bracket、Reaction 等日常工具仍负责几何创建；逻辑对象面板是完整属性和高级关系
入口，不另建一套平行模型。

### ChemDraw 对应入口

ChemDraw 没有把 Reaction Scheme/Step 暴露成一个通用的 “Link” 面板。其常用入口是：

- 用 Reaction Arrow 工具画标准直线反应箭头，再把反应物、产物、加号和条件文字放在
  箭头两侧或上下；ChemDraw 由版面关系形成 `scheme/step`；
- 需要原子映射时使用 `Structure > Map Reaction`（不同版本或本地化界面的文字可能
  略有差异），映射结果写入 Step 的标准 atom-map 字段；
- CDX/CDXML 中的 `scheme/step` 是这种识别结果的正式载体，不等同于普通 group。

ChemSema 把第一项统一接入 Link `auto`：用户仍然按普通反应图绘制，提交移动、绘制或
粘贴操作后，由内核生成 typed Reaction Step。右键 `Logical Objects...` 是查看和精确
编辑完整字段的高级入口；显式 Link/Unlink 分别固定或禁止 Auto 关系。

## 7. 复制、删除和历史

- 只有关系所有端点都在剪贴板选择内时才携带该关系。
- 粘贴先分配所有新 entity id，再统一重映射逻辑 id、owner、target、成员、显示
  文本、crossing bond、Sequence identifier 和 Cross Reference。
- 删除对象后在同一提交内清理悬空引用。
- set/delete/reorder 都是单个可撤销命令；命令 delta 把逻辑 id 作为 object target，
  纯逻辑变化也必须增加 revision 并进入保存状态。

## 8. CDX/CDXML 与 CCJS 往返

- 原生模型是已建模字段的权威来源；导出前移除对应 interchange 逻辑节点，再从
  原生模型重建，防止双写。
- CDXML 与 CDX 共用同一逻辑对象写入路径。
- 原始数字 source id 可用时优先稳定复用；新对象分配不冲突的数字 id。
- Reaction atom map 按 Manual、Automatic、Imported 的明确来源分别写标准字段。
- ChemSema 私有 `linkPolicy/bindingOrigin` 不写入 CDX/CDXML；标准对象本身的关系
  完整写出。
- CCJS 保存全部原生字段和显式 unresolved source id。

## 9. 门禁

NR-017 关闭前必须通过：

- 全对象族 CDXML → CCJS → CDXML 和 CDX 往返；
- 创建、修改、删除、排序、undo/redo；
- 删除、复制粘贴和跨标签 id 重映射；
- Reaction Auto、显式 Link、Unlink、机制箭头排除和无箭头线排除；
- 浏览器真实 WASM 右键入口、对话框编辑、导出和删除；
- 字段总账生成与检查、Rust 全测试、WASM 重建、核心架构审查。
