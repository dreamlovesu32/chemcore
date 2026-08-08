# CCJS 0.2 稳定化架构与发布门禁

状态：约束实现的候选规范。CCJS 0.2 在本文全部发布门禁通过前不得标记为 stable。

## 1. 版本边界

CCJS 0.2 是完整、来源无关的文档语义，不等同于某一种物理封装。以下协议独立版本化，但必须作为同一套 0.2 发行体系共同交付：

- `chemsema` / `0.2`：规范化文档快照；
- `chemsema.container.v1`：`.ccjz` 分块容器；
- `chemsema.document.patch.v1`：revision 有界的进程内和跨进程增量同步；
- `chemsema.journal.v1`：崩溃恢复、审计与压实日志；
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
interchange/<sha256>.json
previews/thumbnail.svg
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

容器读取接口必须支持只读 manifest、指定 scene chunk、指定 resource 和指定 preview，不得强制先解压整个容器。编辑器首次打开先读 root、层级索引和可见区域 chunks；其他 chunks 和资源按需加载。保存采用 copy-on-write，未改变且哈希相同的 entry 可直接复制。

## 4. 增量同步

所有运行路径使用同一 revision 规则：浏览器 WASM、桌面进程内 WASM、Tauri 原生服务和 Office 文档服务。每个内容命令返回 `CommandResult` 和 `DocumentPatch`；补丁必须满足 `beforeRevision == localRevision`，否则请求完整 snapshot。

跨进程接口不得为单对象命令返回完整 document/render/state。原生服务返回 patch、目标 render primitives 和轻量状态；桌面 host 在本地应用同一补丁。完整 snapshot 只用于首次加载、显式恢复、revision 缺口或不支持旧后端。

## 5. Journal 与恢复

journal 是独立 JSONL，不写入 CCJS snapshot。桌面使用同目录 sidecar，Web 使用 IndexedDB。每条最小记录包含 schema、sequence、base snapshot SHA-256、Document Patch、上一记录 SHA-256 和本记录 SHA-256；patch 自身携带 `beforeRevision` 与 `revision`。协同编辑需要的作者、服务器时间和冲突元数据不冒充崩溃恢复字段。

提交流程为：追加并刷盘 journal → 应用内存事务 → 更新界面。正常保存生成新容器并验证后，删除已被新 snapshot 吸收的 journal；崩溃恢复从最后一个已验证 snapshot 开始重放。只允许忽略未换行的截断尾记录；哈希链错误、完整坏行或 revision 缺口必须停止并报错，不得猜测或静默跳过。

## 6. 合规层级

`chemsema-cli validate` 必须提供三个独立等级：

1. `structural`：容器、哈希、JSON Schema、ID、层级、引用和 relation signature；
2. `chemical`：分子图、价态、芳香性、立体化学、反应角色、属性 basis 和光谱 assignment；
3. `roundtrip`：声明的目标格式导出再导入后满足规定的语义和视觉门禁。

同时提供 `migrate`、`canonicalize`、`schema ccjs-v0.2` 和 `conformance`。错误报告必须包含稳定 error code、JSON Pointer/entry、规范条款、严重级别和是否会造成信息损失。

## 7. 独立实现

Rust 是产品权威实现，但不能是格式正确性的唯一证据。仓库包含无 Rust 绑定的浏览器 JavaScript 和 Python 参考读取器，二者完成：格式识别、旧 gzip 读取、新容器 manifest/hash 验证和 CCJS 0.2 装配；JavaScript 另提供规范化写出与 Blob range reader。

`chemsema.conformance.v1` 覆盖确定性写出、跨实现装配、scene 分块、JSON resources、opaque attachment、旧 gzip、损坏哈希/CRC、危险或重复 entry、未声明 entry、截断 journal 和迁移。Rust、JavaScript、Python 对合法跨实现样例给出相同语义文档；拒绝类测试由各 reader 的安全门禁独立覆盖。

## 8. 性能门禁

使用固定种子的 1 万、10 万和 100 万 scene entity 文档，以及 10 MB、100 MB、1 GB 资源场景。容器门禁报告写入、manifest 打开、首 chunk 读取、attachment range 与吞吐；5000 原子桌面门禁另报告单对象编辑时延和完整 document JSON 调用次数。

稳定门禁：

- 单对象编辑不得传输完整 snapshot；
- patch 大小只与受影响闭包有关，不随总文档大小线性增长；
- manifest 和指定 entry 可以独立读取；
- opaque 大资源必须流式复制，不得整体物化到内存；
- 恶意压缩比、巨大声明尺寸、entry 数和层级深度受明确限额保护；
- 性能基线回退超过登记阈值时 CI 失败。

## 9. 发布条件

只有以下条件全部成立才能将 CCJS 0.2 标为 stable：

- Rust、TypeScript、Python conformance 全绿；
- 旧 v0.1、旧 gzip `.ccjz` 和当前 0.2 snapshot 均有确定迁移；
- 新 `.ccjz` 容器在 Web、CLI、桌面和 Office 路径一致；
- 所有增量路径通过 revision 缺口与大文件门禁；
- journal 恢复和损坏处理测试通过；
- `npm run verify`、workspace tests、WASM 和桌面回归全绿；
- 规范、Schema、CLI capabilities、用户指南和兼容政策同步；
- 已发布真实 corpus、性能报告和已知限制，不以未验证主张宣传优势。
