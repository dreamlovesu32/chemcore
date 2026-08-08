# CCJS 文档格式 v0.2

状态：当前规范。JSON Schema：[`schemas/ccjs-v0.2.schema.json`](../schemas/ccjs-v0.2.schema.json)。

## 1. 定位

CCJS 是 ChemSema 的来源无关、可编辑化学文档快照。它同时表达页面对象、分子图、样式、资源、层级、跨对象关系、反应和其他化学语义。它不是 CDXML 的 JSON 转写，也不替代 SMILES、InChI、MOL/SDF、CML 或 HDF5；这些格式继续通过导入、导出、资源或标识符字段协作。

v0.2 的目标是：以稳定 ID 平铺场景实体；在文件中显式保存树索引；把归属、语义关系、阅读顺序和空间索引分开；严格治理格式名、版本、单位、引用和迁移；支持引擎局部补丁；保留未被原生模型覆盖的 CDX/CDXML 信息。

## 2. 顶层结构

```json
{
  "format": {
    "name": "chemsema",
    "version": "0.2",
    "unit": "pt",
    "profile": "snapshot"
  },
  "document": {},
  "style": {},
  "styles": {},
  "entities": { "scene": [] },
  "hierarchy": { "roots": [], "children": {} },
  "relations": [],
  "orders": { "reading": [] },
  "reactionSchemes": [],
  "chemicalProperties": [],
  "resources": {},
  "logicalObjects": {},
  "interchange": {}
}
```

JSON 对象成员顺序没有语义。实现不得把字段在文本中的先后或对象键顺序解释成绘制、阅读或所有权顺序。

## 3. 格式头与版本门禁

写出端必须生成固定的 `chemsema`、`0.2`、`pt`、`snapshot`。读取端接受并验证 v0.2；接受 v0.1 后先迁移再按当前规则验证；拒绝错误格式名、未知版本、非 pt 单位和未知 profile。v0.2 拒绝未知顶层字段。读取旧文件不等于继续写旧格式，成功保存只产生 v0.2。

## 4. 场景实体

`entities.scene` 是平铺数组。每个成员必须有非空且在 scene 命名空间内唯一的 `id`。数组位置不是引用方式，也不是层级或绘制语义；引用必须使用 ID。

```json
{
  "id": "obj_text_1",
  "type": "text",
  "name": "condition",
  "visible": true,
  "locked": false,
  "zIndex": 20,
  "transform": {
    "translate": [120, 80],
    "rotate": 0,
    "scale": [1, 1]
  },
  "styleRef": "style_text_default",
  "linkPolicy": "auto",
  "meta": {},
  "payload": {}
}
```

v0.2 scene entity 禁止携带 `children`。新增顶层对象通常可以追加到数组末尾，但文件仍是完整 JSON 快照；数组尾部不是日志协议，数组顺序也不具语义。

`zIndex` 是绘制层级的唯一权威，同层按稳定 ID 决定次序。v0.2 不增加另一份 `paintOrder`，避免两份权威顺序互相矛盾。

## 5. 层级索引

```json
{
  "hierarchy": {
    "roots": ["group_1", "obj_free_text"],
    "children": {
      "group_1": ["obj_molecule_1", "obj_condition_1"]
    }
  }
}
```

不变量：

1. 每个 scene ID 必须恰好出现一次：位于 roots，或位于一个父项的 children；
2. roots 和每个 children 数组内部不得重复；
3. 父、子 ID 必须存在；
4. 只有 `type = group` 的实体可以成为父节点；
5. 层级必须无环，所有实体必须从 roots 可达；
6. hierarchy 是归属的唯一权威，relation 不得重复表达 scene 容器关系。

`group` 仍是可编辑 scene entity，承担共同变换、显隐、锁定、选择和包围盒。atom 属于 molecule、bond 端点、reaction step 成员和 spectrum 数据点不属于通用 group，继续由专业模型表达。

## 6. 关系模型

`relations` 保存跨实体语义，不保存场景容器关系：

```json
{
  "id": "link_12",
  "kind": "analysis-caption",
  "endpoints": [
    { "entityId": "obj_molecule_1", "role": "source" },
    { "entityId": "obj_text_1", "role": "caption" }
  ],
  "data": {}
}
```

当前注册类型为 `bracket-repeat-label`、`analysis-caption`、`atom-symbol`、`chemical-property-display` 和 `annotation-basis`。验证器检查唯一关系 ID、端点存在、端点不重复、角色签名和对象类型；未知 kind 被拒绝。

scene entity 的 `linkPolicy`：

- `auto`：空间索引寻找候选，类型化 resolver 决定是否形成专业关系；
- `linked`：用户确认关系，普通移动不会自动解除；
- `unlinked`：禁止为该对象自动求解关系。

空间近邻只是候选条件，不是关系本身。自动反应识别先查询派生空间索引，再按箭头轴、方向、距离、对象类型和歧义规则判断角色。

## 7. 阅读顺序与空间索引

`orders.reading` 是可选、持久化的阅读顺序。空数组表示没有确认顺序，消费者可从几何派生。每个 ID 必须存在且不得重复；它不决定绘制层级。

R-tree、均匀网格、BVH、反向依赖表和渲染缓存属于运行时派生索引，不写入 `.ccjs`。当前内核按 document revision 构建 96 pt 网格索引，精确检查候选包围盒，并在 revision 变化后重建。缓存丢失不影响文档含义。

## 8. 资源与专业数据

`resources` 通过资源 ID 保存可复用或体量较大的载荷。scene entity 的 `resourceRef` 必须指向存在的资源。molecule scene entity 负责页面定位，`molecule_fragment2d` resource 负责 atom、bond、stereo、标签和结构语义。reactionSchemes、chemicalProperties 和 logicalObjects 分别保存反应、属性和高级逻辑语义。

大规模 FID、长光谱数组、图像或多维实验数据不应无限制内联到普通 JSON。v0.2 的 `.ccjs` 定义完整语义快照；`.ccjz` 使用 [`chemsema.container.v1`](protocol/ccjz-container-v1.md) 确定性 ZIP 容器，将 scene 拆成可独立读取的 JSONL chunk，并按 SHA-256 内容寻址外置资源。旧 gzip `.ccjz` 继续只读兼容，新写出器不再生成 gzip。HDF5、Zarr 或专业二进制数组可以作为带明确媒体类型、尺寸和哈希的资源载荷，而不是替代 CCJS 文档语义。

## 9. Interchange 无损层

`interchange` 保存尚未提升为来源无关字段的 CDX/CDXML 对象、属性、顺序和原始字节。已有 native 字段时 native 字段是编辑权威；exporter 用 native 值重新编码已建模内容，再用 interchange 补回未建模信息。未识别信息不得塞入不参与导出的普通 meta 后静默丢失。

## 10. 精确更新协议

文件快照和前端更新是两个协议。每个 Document Commit 推进一次 revision，并可返回局部 `DocumentPatch`：

```json
{
  "beforeRevision": 7,
  "revision": 8,
  "upsertEntities": [],
  "deletedEntityIds": [],
  "hierarchyRoots": [],
  "upsertResources": {},
  "relationScopeEntityIds": [],
  "relations": [],
  "upsertStyles": {},
  "deletedStyleIds": [],
  "logicalObjects": {},
  "reactionSchemes": [],
  "chemicalProperties": [],
  "orders": { "reading": [] }
}
```

补丁只携带受影响实体及依赖资源；高级语义区和顺序只在相关目标可能变化时作为可选替换快照出现。前端仅在 `beforeRevision` 与本地 revision 相等时应用，乱序或缺口立即回退完整同步。前端更新本地 ID 映射和必要层级，再通过 renderTargets 获取目标 primitives；旧后端缺少补丁接口时也回退完整同步。详见 [`protocol/document-patch-v1.md`](protocol/document-patch-v1.md)。

## 11. 子文档与“截取”

任意字节截断不是合法 JSON，也不能保证引用闭包。安全截取是依赖闭包子文档：包含选中实体、必要 group 上下文、styles、resources、relations、chemical properties、reaction steps 和 logical objects，输出另一份通过 v0.2 验证的自包含 snapshot。当前 clipboard document 和 CLI bundle/subset 路径使用这一原则。

日志式追加应使用独立 JSONL journal，并在保存时压实为 snapshot，不能把半截 JSON 当作恢复格式。

## 12. 合规要求

合规写出器必须写规范格式头、平铺 scene、生成完整无环单归属 hierarchy、不输出孤立引用、只使用注册 relation kind，并通过 JSON Schema 和运行时语义验证。Schema 无法表达跨数组唯一 ID、无环和类型化端点等全部约束，因此运行时验证不可省略。

## 13. v0.1 迁移

v0.1 的嵌套 objects 以先序遍历展平；顶层对象成为 roots；原 group.children 生成 hierarchy children；links 改名为 relations；缺失 orders 生成空 reading。迁移后统一执行 v0.2 验证，保存只产生 v0.2。
