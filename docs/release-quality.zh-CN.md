# ChemSema 发布质量矩阵

这份矩阵记录主要公开能力当前的可信度。它是发布质量边界，不是营销承诺。

| 表面 | 状态 | 验证方式 |
| --- | --- | --- |
| CDXML 导入 | Beta | 公开 fixture、论文图、golden SVG snapshot、解析回归 |
| CDX 导入/导出 | Beta | round-trip 测试和二进制存储回归 |
| CCJS 0.2 / CCJZ v1 | Beta | Schema、迁移、稳定诊断、五格式往返、Rust/JS/Python 交叉读取、可见区加载、COW/Zip64、journal 与性能门禁；生态/corpus/full 报告边界见稳定化合同 |
| SVG 导出 | Usable | golden SVG snapshot 和像素比较脚本 |
| Office/OLE 复制与嵌入 | Beta | 剪贴板 payload、EMF preview、Word 粘贴/回读验证 |
| 浏览器编辑器 | Beta | viewer 交互 smoke test 和用户路径稳定性脚本 |
| 桌面端 | Beta | Tauri build、文件关联配置、hybrid latency 回归、安装验证 |
| GUI 测试平台 | 实施中/production 鼠标、键盘、同类/异类多对象、二层组合、不可变制品及全场景性能 trace sentinel 已通过 | 版本化 Schema/runner、真实 Playwright 路径、覆盖/影响/资源门禁、确定性 checkpoint 恢复、无人值守 Hyper-V 登录、逐项验证的专用用户 baseline、内容寻址候选部署、受守卫的 UIA/CDP 定位、有界持久输入与 session-0 公开观察 CDP 通道、单次调用 guest 动作事务、带 SHA 验证的真实点击/拖拽、白名单扫描码键盘输入、同类与异类对象选择/剪贴板/历史、分子与箭头的二层 group/ungroup 及嵌套剪贴板复制、层级感知的增量 DOM patch、图元计数和白名单不同身份 DOM oracle、不读取 production 调试全局的公开 DOM/window receipt、完整最终截图/DOM/公开状态/WebView 日志/性能 trace、guest→host PowerShell Direct 制品传输及 guest/host 两端 SHA-256 验证、Playwright 截图/DOM/CCJS/状态/console/trace bundle，以及不可变报告/制品对象和经过验证的 manifest 已运行；真实 GUI 保存—外部解析—重开化学文档 oracle、视频/崩溃 bundle、完整 capability 矩阵、更深及更多对象类型的组合/异类单元、复杂与大文档构建、模型/故障/变异测试和展示资格仍待完成 |
| CLI one-shot 命令 | Usable | Rust 测试、`npm run verify`、稳定性报告、输出写入验证 |
| CLI JSONL session | Experimental/usable | session 单测和大文件性能报告 |
| Agent 精确截图 | Usable beta | PNG/SVG capture 测试、公开 fixture crop、README 示例图 |
| Agent context/detail | Usable beta | selector/context/detail 测试和公开 fixture 示例 |
| 安装器 CLI PATH/App Paths | Beta | NSIS hook 和干净安装/卸载验证 |

## 安全基线

当前 beta 把这些区域作为硬化优先级：

| 区域 | 基线 |
| --- | --- |
| 文件导入 | 已有公开 fixture 和解析回归；恶意输入 corpus 继续扩展 |
| CCJZ 容器 | 已限制 entry 数、单 entry/总尺寸、路径、重复/大小写碰撞、哈希和声明绑定；公开拒绝类 conformance corpus 仍待统一 |
| XML/CDXML 解析 | 已有 parser 测试；深度和大小限制属于 beta 硬化项 |
| 栅格/矢量导出 | 已验证输出路径、字节数；渲染超时和超大输出限制属于 beta 硬化项 |
| CLI session | 已有确定性 JSONL 协议；请求超时和资源预算策略属于 beta 硬化项 |
| 文件写入 | 已验证输出存在和字节数；更严格的写入范围策略属于后续工作 |
| Office payload | 已有剪贴板/OLE schema 测试；畸形 payload 验证继续补强 |

## 发布门禁

公开 beta 发布前：

1. 运行 `npm ci`。
2. 运行 `cargo build -p chemsema-office -p chemsema-cli --release`。
3. 运行 `cargo test`。
4. 运行 `npm run verify`。
5. 用 `npm run desktop:build` 构建安装包。
6. 确认 GitHub CI 在 `main` 和 release tag 上通过。
7. 上传安装包并记录 SHA256。

以上是当前 beta 门禁，不等于完整 GUI 或展示资格。GUI 测试平台落地后，stable/正式展示还必须通过 `gui-pr`、`gui-nightly`、最终安装包 `release-qualification` 和 [Demo Qualification Gate](./gui-test-platform-and-demo-reliability.zh-CN.md#16-demo-qualification-gate)；资格 manifest 必须证明每个用户可见功能经过真实点击/输入/拖拽、每类对象经过实际绘制或创建、全部公开属性、`0/1/2/many` 多对象及复杂/大文档矩阵均有当前有效证据。闭包未变的证据可以复用，受影响、过期和不可缓存测试必须重跑。第一次失败后重跑成功仍记为 flaky failure，不能改写为通过。

## 当前对外边界

ChemSema 已经在 CDXML 保真、Office 工作流和 agent-oriented CLI 上形成可验证原型。
它仍是 beta，需要更多真实文件、真实工作流、安全硬化和干净安装验证，之后才能被描述为完整 ChemDraw 替代品。
