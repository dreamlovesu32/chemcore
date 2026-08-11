# CCJS 0.2 稳定化架构与发布门禁

状态：当前长期架构与稳定化合同。CCJS 0.2 是当前写出规范，但在本文“stable 发布条件”全部满足前仍不得宣称格式已经冻结或具备成熟生态。

本文不是第二份字段规范：[CCJS 0.2](./format-v0.2.zh-CN.md) 定义语义快照，[CCJZ Container v1](./protocol/ccjz-container-v1.md) 定义容器，[Document Patch v1](./protocol/document-patch-v1.md) 定义局部同步，[Recovery Journal v1](./protocol/journal-v1.md) 定义崩溃恢复。设计理由和格式比较见 [CCJS 架构、比较与必要性](./ccjs-architecture-and-format-rationale.zh-CN.md)。

当前实现状态：

| 能力 | 状态 | 准确边界 |
|---|---|---|
| CCJS 0.2 规范化快照与 v0.1 迁移 | 已实现 | 新写出只产生 v0.2 |
| CCJZ v1 确定性容器 | 已实现 | Rust/JS/Python 交叉读取；旧 gzip 只读 |
| scene 分块、资源寻址、opaque attachment range | 已实现 | 浏览器按可见区加载；未知 bounds 安全回退加载 |
| Document Patch 精确更新 | 已实现 | revision 缺口回退完整同步 |
| hash-chain recovery journal 与原子保存 | 已实现 | 崩溃恢复，不是协同编辑 |
| 大文档与大附件性能门禁 | 已实现 | smoke 纳入 `npm run verify`，full profile 可显式运行 |
| 稳定结构化诊断 | 已实现 | `chemsema.validation-report.v1` 固定 issue 字段 |
| 按目标格式的 chemical/visual roundtrip 等级 | 已实现 | CCJS/CCJZ/CDXML/CDX/SDF；SDF 显式 loss gate |
| 编辑器可见区懒加载和保存 entry copy-on-write | 已实现 | hydration 保留编辑/undo；保存保留附件并复用同哈希 entry |
| 浏览器 Zip64 读写 | 已实现 | 小文件保持经典 ZIP；超安全整数明确拒绝 |

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

容器读取接口支持只读 manifest、指定 scene chunk、指定 resource 和 attachment byte range，不强制先解压整个容器。chunk manifest 可记录保守 `bounds` 和 `entityIds`；浏览器 Blob 路径先加载 root，只读取与当前视口相交的 chunks，缺少可靠 bounds 时必须读取该 chunk。补载使用累计 partial snapshot hydration：既有对象保持编辑权威，新增对象/资源进入当前文档，relation、hierarchy 和 reading order 与已加载区域同步，并把新增磁盘对象合入既有历史快照，使 undo 不会删除后来补载的数据。保存前必须 materialize 全部 chunks。

桌面 CCJZ 保存使用 copy-on-write：path、descriptor 和内容哈希未变的 root/chunk/resource/attachment 直接从旧容器流式复制，变化 entry 重写；opaque attachment 即使调用方不再次提供 payload 也会保留。写出在同目录临时文件完成，重新打开验证后原子替换。浏览器 writer/reader 支持 Zip64 directory/offset/size；超过 `Number.MAX_SAFE_INTEGER` 的声明拒绝处理。

## 4. 增量同步

所有运行路径使用同一 revision 规则：浏览器 WASM、桌面进程内 WASM、Tauri 原生服务和 Office 文档服务。每个内容命令返回 `CommandResult` 和 `DocumentPatch`；补丁必须满足 `beforeRevision == localRevision`，否则请求完整 snapshot。

跨进程接口不得为单对象命令返回完整 document/render/state。原生服务返回 patch、目标 render primitives 和轻量状态；桌面 host 在本地应用同一补丁。完整 snapshot 只用于首次加载、显式恢复、revision 缺口或不支持旧后端。

## 5. Journal 与恢复

journal 是独立 JSONL，不写入 CCJS snapshot。桌面使用同目录 sidecar，Web 使用 IndexedDB。每条最小记录包含 schema、sequence、base snapshot SHA-256、Document Patch、上一记录 SHA-256 和本记录 SHA-256；patch 自身携带 `beforeRevision` 与 `revision`。协同编辑需要的作者、服务器时间和冲突元数据不冒充崩溃恢复字段。

提交流程为：追加并刷盘 journal → 应用内存事务 → 更新界面。正常保存生成新容器并验证后，删除已被新 snapshot 吸收的 journal；崩溃恢复从最后一个已验证 snapshot 开始重放。只允许忽略未换行的截断尾记录；哈希链错误、完整坏行或 revision 缺口必须停止并报错，不得猜测或静默跳过。

## 6. 合规层级

`chemsema-cli validate` 暴露三个等级，并统一返回 `chemsema.validation-report.v1`：

1. `structural`：执行容器/哈希、JSON、格式头、基本形状和引擎文档不变量；失败 issue 给出稳定 code、pointer/entry、条款、severity 和 information-loss；
2. `chemical`：在 structural 之上显式校验每个可编辑分子图，并报告对象定位和稳定化学错误 code；
3. `roundtrip`：在 chemical 之上按重复 `--target-format` 或逗号列表对 CCJS、CCJZ、CDXML、CDX、SDF 执行真实导出/导入；语义指纹精确比较，视觉 primitive 使用 2 pt 容差，SDF 的已知表达损失先明确拒绝。

已经提供 `migrate`、`canonicalize`、`schema ccjs-v0.2` 和 `conformance`。新增诊断 code 或条款可以向后扩展，但既有 code 含义和 issue 字段不得静默改变。

## 7. 独立实现

Rust 是产品权威实现，但不能是格式正确性的唯一证据。仓库包含无 Rust 绑定的浏览器 JavaScript 和 Python 参考读取器，二者完成：格式识别、旧 gzip 读取、新容器 manifest/hash 验证和 CCJS 0.2 装配；JavaScript 另提供规范化写出与 Blob range reader。

`npm run conformance:ccjz` 覆盖 Rust、JavaScript、Python 之间的确定性写出和跨实现装配，并包含 scene 分块、JSON resources、opaque attachment、浏览器 Zip64 和可见区只加载相交 chunk。Rust/JavaScript 单元测试另外覆盖旧 gzip、损坏哈希、危险或重复 entry、未声明 entry、Zip64 安全整数边界、局部 relation 删除和 journal 截断/损坏；v0.1 迁移由引擎测试覆盖。stable conformance corpus 仍需把这些分散的拒绝类夹具统一成可发布、跨实现复用的固定语料与报告。

## 8. 性能门禁

使用固定种子的 1 万、10 万和 100 万 scene entity 文档，以及 10 MB、100 MB、1 GB 资源场景。容器门禁报告写入、manifest 打开、首 chunk 读取、attachment range、吞吐，以及只修改末块时的 copy-on-write 时间、复用 entry/bytes 和复用比例；5000 原子桌面门禁另报告单对象编辑时延和完整 document JSON 调用次数。

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
