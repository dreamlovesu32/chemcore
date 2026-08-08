# CCJS 0.2 稳定化架构与发布门禁

状态：当前长期架构与稳定化合同。CCJS 0.2 是当前写出规范，但在本文“stable 发布条件”全部满足前仍不得宣称格式已经冻结或具备成熟生态。

本文不是第二份字段规范：[CCJS 0.2](./format-v0.2.zh-CN.md) 定义语义快照，[CCJZ Container v1](./protocol/ccjz-container-v1.md) 定义容器，[Document Patch v1](./protocol/document-patch-v1.md) 定义局部同步，[Recovery Journal v1](./protocol/journal-v1.md) 定义崩溃恢复。设计理由和格式比较见 [CCJS 架构、比较与必要性](./ccjs-architecture-and-format-rationale.zh-CN.md)。

当前实现状态：

| 能力 | 状态 | 准确边界 |
|---|---|---|
| CCJS 0.2 规范化快照与 v0.1 迁移 | 已实现 | 新写出只产生 v0.2 |
| CCJZ v1 确定性容器 | 已实现 | Rust/JS/Python 交叉读取；旧 gzip 只读 |
| scene 分块、资源寻址、opaque attachment range | 已实现于容器层 | 编辑器打开仍装配完整快照 |
| Document Patch 精确更新 | 已实现 | revision 缺口回退完整同步 |
| hash-chain recovery journal 与原子保存 | 已实现 | 崩溃恢复，不是协同编辑 |
| 大文档与大附件性能门禁 | 已实现 | smoke 纳入 `npm run verify`，full profile 可显式运行 |
| 稳定结构化诊断 | 未完成 | 尚缺统一 error code、pointer、条款和 loss severity |
| 按目标格式的 chemical/visual roundtrip 等级 | 未完成 | 当前 roundtrip 只覆盖规范 CCJS 重装配 |
| 编辑器可见区懒加载和保存 entry copy-on-write | 未完成 | 底层 range/stream 能力已具备，应用层尚未接通 |
| 浏览器 Zip64 写出 | 未完成 | 当前浏览器 writer 使用经典 ZIP 上限 |

## 1. 版本边界

CCJS 0.2 是完整、来源无关的文档语义，不等同于某一种物理封装。以下协议独立版本化，但必须作为同一套 0.2 发行体系共同交付：

- `chemsema` / `0.2`：规范化文档快照；
- `chemsema.container.v1`：`.ccjz` 分块容器；
- `chemsema.document.patch.v1`：revision 有界的进程内和跨进程增量同步；
- `chemsema.journal.v1`：崩溃恢复与 checkpoint 压实日志；
- `chemsema.conformance.v1`：跨实现合规夹具与报告。

协议独立版本化不是推迟问题。任何消费者从容器装配得到的结果必须是通过 CCJS 0.2 Schema 和运行时语义验证的完整快照。

## 2. `.ccjs` 与 `.ccjz`

`.ccjs` 是单文件 UTF-8 JSON，适合小文档、代码审查、交换和调试。它仍需整份解析，不承诺随机访问。

`.ccjz` 是生产文档容器，不再是 gzip JSON。读取器必须继续识别旧 gzip 魔数 `1f 8b` 并迁移；新写出器只生成 ZIP 容器。新 MIME 为 `application/vnd.chemsema.document+zip`。

容器逻辑结构：

```text
mimetype
manifest.json
document/root.json
entities/scene-000000.jsonl
resources/<sha256>.<ext>
attachments/<sha256>.<ext>
```

`mimetype` 必须是第一个未压缩 entry，内容固定为 MIME。`manifest.json` 使用 `chemsema.container.v1`，记录所有 entry 的未压缩字节数、SHA-256、媒体类型、记录数、可选边界和引用依赖。entry 名必须是规范相对路径，禁止绝对路径、反斜杠、`.`、`..`、重复名和大小写碰撞。

`document/root.json` 保存除平铺 scene records 和可外置资源载荷以外的文档根。scene 采用 JSONL 分块；每行是一个完整平铺 entity。层级、关系和顺序仍在 root 中并保持唯一权威。装配器按 manifest 顺序读取 chunks、校验哈希和计数、补回资源，形成一个标准 CCJS 0.2 snapshot 后执行完整语义验证。

容器采用确定性写出：固定 entry 顺序、固定时间戳、UTF-8/LF、稳定 JSON 键序、相同输入产生相同字节。保存必须先写同目录临时文件、重新打开验证 manifest 与所有哈希，再原子替换目标。

## 3. 大型资源

图像、原始 FID、长光谱数组、多维矩阵和其他大载荷不得无限内联。manifest 资源条目采用内容寻址；文档中的资源描述保存媒体类型、编码、单位、轴、形状、字节数和 SHA-256。

- 小型结构化资源可保存为 JSON；
- 大型一维数值数组优先使用可分块二进制数组；
- NMR FID 和多维数据允许 HDF5/Zarr 资源；
- 外部引用必须同时带哈希和显式可移植性状态；默认稳定文档必须内嵌；
- 未知媒体类型可以保留和复制，但不得在未理解时改写。

容器读取接口已支持只读 manifest、指定 scene chunk、指定 resource 和 attachment byte range，不强制先解压整个容器。当前编辑器仍会装配完整快照后进入编辑；“首次只加载可见 chunks”是下一层应用优化，不是当前行为。保存当前流式生成新容器；未改变且哈希相同 entry 的 copy-on-write 复用仍是后续门禁。

## 4. 增量同步

所有运行路径使用同一 revision 规则：浏览器 WASM、桌面进程内 WASM、Tauri 原生服务和 Office 文档服务。每个内容命令返回 `CommandResult` 和 `DocumentPatch`；补丁必须满足 `beforeRevision == localRevision`，否则请求完整 snapshot。

跨进程接口不得为单对象命令返回完整 document/render/state。原生服务返回 patch、目标 render primitives 和轻量状态；桌面 host 在本地应用同一补丁。完整 snapshot 只用于首次加载、显式恢复、revision 缺口或不支持旧后端。

## 5. Journal 与恢复

journal 是独立 JSONL，不写入 CCJS snapshot。桌面使用同目录 sidecar，Web 使用 IndexedDB。每条最小记录包含 schema、sequence、base snapshot SHA-256、Document Patch、上一记录 SHA-256 和本记录 SHA-256；patch 自身携带 `beforeRevision` 与 `revision`。协同编辑需要的作者、服务器时间和冲突元数据不冒充崩溃恢复字段。

提交流程为：追加并刷盘 journal → 应用内存事务 → 更新界面。正常保存生成新容器并验证后，删除已被新 snapshot 吸收的 journal；崩溃恢复从最后一个已验证 snapshot 开始重放。只允许忽略未换行的截断尾记录；哈希链错误、完整坏行或 revision 缺口必须停止并报错，不得猜测或静默跳过。

## 6. 合规层级

`chemsema-cli validate` 暴露三个等级；当前能力与长期要求必须分开陈述：

1. `structural`：当前执行容器/哈希、格式头、基本形状和引擎文档不变量；长期还要把完整 JSON Schema 与所有错误位置写入结构化报告；
2. `chemical`：当前复用引擎装载与语义不变量；长期要显式覆盖分子图、价态、芳香性、立体化学、反应角色、属性 basis 和光谱 assignment；
3. `roundtrip`：当前验证规范 CCJS 的重新装载一致性；长期要按声明目标格式执行导出再导入，并绑定语义和视觉门禁。

已经提供 `migrate`、`canonicalize`、`schema ccjs-v0.2` 和 `conformance`。stable 前，失败报告还必须统一补齐稳定 error code、JSON Pointer/entry、规范条款、严重级别和是否会造成信息损失。

## 7. 独立实现

Rust 是产品权威实现，但不能是格式正确性的唯一证据。仓库包含无 Rust 绑定的浏览器 JavaScript 和 Python 参考读取器，二者完成：格式识别、旧 gzip 读取、新容器 manifest/hash 验证和 CCJS 0.2 装配；JavaScript 另提供规范化写出与 Blob range reader。

`npm run conformance:ccjz` 当前覆盖 Rust、JavaScript、Python 之间的确定性写出和跨实现装配，并包含 scene 分块、JSON resources 与 opaque attachment。Rust/JavaScript 单元测试另外覆盖旧 gzip、损坏哈希、危险或重复 entry、未声明 entry 和 journal 截断/损坏；v0.1 迁移由引擎测试覆盖。stable conformance corpus 仍需把这些分散的拒绝类夹具统一成可发布、跨实现复用的固定语料与报告。

## 8. 性能门禁

使用固定种子的 1 万、10 万和 100 万 scene entity 文档，以及 10 MB、100 MB、1 GB 资源场景。容器门禁报告写入、manifest 打开、首 chunk 读取、attachment range 与吞吐；5000 原子桌面门禁另报告单对象编辑时延和完整 document JSON 调用次数。

稳定门禁：

- 单对象编辑不得传输完整 snapshot；
- patch 大小只与受影响闭包有关，不随总文档大小线性增长；
- manifest 和指定 entry 可以独立读取；
- opaque 大资源必须流式复制，不得整体物化到内存；
- 巨大声明尺寸、entry 数、路径、重复/大小写碰撞和哈希绑定受明确限额或拒绝规则保护；
- smoke 性能阈值随 `npm run verify` 执行；full profile 在 stable 发布前显式运行并归档，不能以 smoke 代替。

## 9. 发布条件

只有以下条件全部成立才能将 CCJS 0.2 标为 stable。当前尚未完成的项目不得因 `npm run verify` 全绿而被省略：

- Rust、JavaScript、Python 的公开固定 conformance corpus 全绿；
- 旧 v0.1、旧 gzip `.ccjz` 和当前 0.2 snapshot 均有确定迁移；
- 新 `.ccjz` 容器在 Web、CLI、桌面和 Office 路径一致；
- 所有增量路径通过 revision 缺口与大文件门禁；编辑器可见区懒加载和未变 entry 复用有独立证据；
- journal 恢复和损坏处理测试通过；
- `npm run verify`、workspace tests、WASM 和桌面回归全绿；
- 规范、Schema、CLI capabilities、用户指南和兼容政策同步；
- `validate` 三等级和稳定结构化错误报告达到第 6 节合同；
- 浏览器对超出经典 ZIP 上限的输入明确拒绝或提供 Zip64 写出策略；
- 已发布真实 corpus、full performance 报告和已知限制，不以未验证主张宣传优势。
