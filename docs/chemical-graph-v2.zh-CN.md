# ChemicalGraphV2 语义契约

## 定位

`chemsema-nomenclature/chemical-graph/2` 是 ChemSema 用于确定分子实体、分子片段和离散
整数配比的、与绘图表现无关的化学语义图。系统命名、核磁预测、图身份比较以及外部格式
适配器都应以它作为统一输入边界。

它不是绘图文档、查询语言、反应模型、聚合物模型或通用物质配方。坐标、字体、颜色、说明
文字、选择状态和缓存几何不得进入该图。机器规范为
[`schemas/chemical-graph-v2.schema.json`](../schemas/chemical-graph-v2.schema.json)，Rust
强类型模型是生成依据，测试会保证仓库内 Schema 与模型严格同步。
规范中的合法和非法样例位于
[`fixtures/chemical-graph-v2`](../fixtures/chemical-graph-v2)。

## 固定语义

新导出的图必须声明：

- `profile`：`molecular-entity`、`molecular-fragment` 或 `discrete-composition`；
- `aromaticityModel`：目前为 `explicit-aromatic-bonds`；
- `hydrogenModel`：目前为 `resolved-counts`；
- `valenceModel`：目前为 `chem-sema2026`；
- `normalization`：目前为 `chemsema-chemical-graph-normalization/1`。

最初的 V2 线上契约没有 `semantics` 对象，因此读取器会把缺失值严格解释为上述固定值。
该对象不是每个文档自行选择规则的开关；只要出现，就必须与受支持的 V2 契约一致。

芳香键形式与交替 Kekulé 形式不会被偷偷视为相同。导入适配器必须先使用明确支持的芳香性
模型完成规范化，再生成 V2。已经解析的隐式氢数参与化学身份。

`molecular-entity` 只允许一个连通、计数为一的组分。`molecular-fragment` 同样只允许一个
计数为一的连通组分，并且必须至少有一个结构化 `freeValences` 项；每项记录自由价所在原子
及缺失键级（`single`、`double` 或 `triple`）。重复项有意义：同一原子上的两个单自由价
不会被折叠成一个双自由价。`discrete-composition` 可包含多个
连通组分及正整数计数。分数占位、非化学计量固体、Markush/查询结构、聚合物和反应是明确
的不支持边界，不得近似后静默通过。

## 校验、规范化和身份

`validate()` 会拒绝未知 schema、未知 JSON 字段、缺失引用、重复 id、重复二中心键、错误
配位方向、不合法立体信息、断裂组分、错误多中心角色以及空白或重复 assumption。

`normalized()` 只负责稳定排序，不重编号原子，因此不能当作分子的规范标识符。

`is_isomorphic_to()` 才是精确身份操作。它比较原子属性、已解析氢数、键型和配位方向、
组分及计数、立体元素和增强立体组，以及多中心相互作用。来源 id、数组顺序、组分/相互
作用 id 和审计 assumption 不参与化学身份。自由价所在原子、键级和重复次数参与身份。

因此，V2 不承诺“一个分子只有一段 JSON”，也不把 SMILES 字符串当作身份键。只有当导入
适配器按照声明的芳香性、氢、价态、电荷和立体语义完成解析后，两条不同 SMILES 才可能
得到同构的 V2 图。需要数据库唯一键时，应使用图同构或另行版本化的规范身份算法；不能
直接散列 `normalized()` JSON，因为它有意保留来源原子 id。

## 分子片段存储示例

`propan-2-yl` 正常存储三个碳组成的骨架，只额外声明中心碳 `c2` 上有一个单自由价：

```json
{
  "semantics": {"profile": "molecular-fragment"},
  "atoms": [
    {"id":"c1","atomicNumber":6,"implicitHydrogens":3},
    {"id":"c2","atomicNumber":6,"implicitHydrogens":1},
    {"id":"c3","atomicNumber":6,"implicitHydrogens":3}
  ],
  "bonds": [
    {"id":"b1","atoms":["c1","c2"],"kind":"single"},
    {"id":"b2","atoms":["c2","c3"],"kind":"single"}
  ],
  "freeValences": [{"atom":"c2","order":"single"}]
}
```

上面为便于阅读省略了固定字段。完整规范样例见
[`fixtures/chemical-graph-v2/valid/propan-2-yl.json`](../fixtures/chemical-graph-v2/valid/propan-2-yl.json)。

## 立体与多中心相互作用

四面体和双键立体使用语义引用，不读取二维几何猜测。扩展立体描述符按类别强类型存储，
不再接受任意字符串；配位几何必须带正的排列序号，富勒烯和环系描述符必须带合法 locant。

普通二中心配位键在 Rust 中使用强类型 donor/acceptor 方向；为保持 V2 兼容，JSON 线上格式
仍为 `donorId->acceptorId`，解析时会严格校验两个端点而不是接受任意字符串。不能化约成
二中心键的关系使用：

- `coordination`：恰好一个供体中心、一个或多个受体中心；
- `delocalized-bond`：至少三个原子组成 shared 中心。

V2 不携带相互作用电子数。电子数是可变分子事实；若在既有 V2 schema 下增加，旧读取器会
静默忽略影响身份的信息。未来只有使用新 schema 并提供显式适配器后才能增加。eta、
kappa、mu 等命名符号由命名规则推导，不作为绘图字符串存储。

## 产品入口

- Rust：`Engine::chemical_graph_v2_json()`；
- WebAssembly：`chemicalGraphV2Json()`；
- CLI：`chemsema-cli chemistry input.ccjs --format chemical-graph-v2 --pretty`；
- 系统命名请求：`Engine::nomenclature_request_json()` /
  `nomenclatureRequestJson()`，其机器契约为
  [`schemas/nomenclature-request-v1.schema.json`](../schemas/nomenclature-request-v1.schema.json)；
- NMR：`nmr_prediction_request_json()` 直接嵌入同一张已校验图，不再另写一套转换。

系统命名和 NMR provider 遇到不认识的 schema、规范化契约或身份字段必须拒绝，不得忽略
后继续计算。
NMR 请求面向完整分子实体；`molecular-fragment` 主要用于系统命名和结构编辑边界。

## 外部格式和损失规则

| 外部表示 | 主要用途 | 必须明确的边界 |
| --- | --- | --- |
| V3000 Mol/SDF | 广义结构交换 | 查询原子、S-group、聚合物、可变连接及不支持的增强立体必须拒绝或报告 |
| CommonChem/rdkitjson | 工具包 JSON 桥接 | 必须声明工具包芳香性和立体模型的映射 |
| SMILES/CXSMILES | 紧凑传输 | 记录解析器、芳香性和价态模型；无法表达的文档语义不得静默丢弃 |
| InChI/InChIKey | 身份和检索 | 不得当作可编辑语义图的无损往返格式 |
| CML | 科学数据交换 | 只接受声明过的 convention/profile；身份扩展不得被静默忽略 |
| CDX/CDXML | ChemDraw 文档保真 | 绘图和文档字段留在 CCJS；V2 只接收已经解析的化学事实 |

所有适配器只能返回“通过校验的完整图”或带损失清单的明确 unsupported/partial 结果，禁止
静默丢字段。
Rust API `ChemicalGraphV2::assess_mapping_to()` 会针对当前实现的 ChemicalGraph、
CDX/CDXML、SMILES 和 SDF V2000 边界生成带版本号的
`chemsema.chemical-graph-mapping-report.v1` 损失清单。
从 CDXML 导入的 CCJS 文档可以在源格式往返时保留已有的 MultiAttachment
代理几何；但仅有 ChemicalGraph 时不能假定这些表现层几何存在。因此，在适配器
尚未明确构造这套文档编码之前，映射报告会拒绝把配位相互作用声明为无损
CDX/CDXML 映射。
当前 CDX/CDXML、SMILES 和 SDF V2000 映射报告也会拒绝把片段声明为无损，直至对应
适配器具有经过验证的结构化自由价编码。
